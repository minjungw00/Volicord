#!/usr/bin/env python3
"""Reproducible Linux behavior probes for V10 platform primitive qualification."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time


def run(arguments: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(arguments, cwd=cwd, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)


def git(repository: Path, *arguments: str) -> bytes:
    return run(["git", "-C", str(repository), *arguments]).stdout


def hash_fields(*fields: bytes) -> str:
    digest = hashlib.sha256()
    for field in fields:
        digest.update(len(field).to_bytes(8, "big"))
        digest.update(field)
    return "sha256:" + digest.hexdigest()


def process_is_running(pid: int) -> bool:
    status = Path(f"/proc/{pid}/status")
    try:
        text = status.read_text(encoding="utf-8")
    except FileNotFoundError:
        return False
    state = next((line for line in text.splitlines() if line.startswith("State:")), "")
    return "Z (zombie)" not in state


def await_not_running(pid: int, timeout: float) -> bool:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if not process_is_running(pid):
            return True
        time.sleep(0.01)
    return not process_is_running(pid)


def await_descendant_readiness(read_fd: int, child: subprocess.Popen[bytes], timeout: float) -> int:
    deadline = time.monotonic() + timeout
    payload = bytearray()
    while b"\n" not in payload:
        if child.poll() is not None:
            raise AssertionError(
                f"descendant fixture exited before readiness with status {child.returncode}"
            )
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise AssertionError("descendant fixture did not report readiness")
        readable, _, _ = select.select([read_fd], [], [], remaining)
        if not readable:
            raise AssertionError("descendant fixture readiness timed out")
        chunk = os.read(read_fd, 128)
        if not chunk:
            raise AssertionError("descendant fixture closed readiness before one complete record")
        payload.extend(chunk)
        if len(payload) > 128:
            raise AssertionError("descendant readiness record exceeded its bound")
    if payload.count(b"\n") != 1 or not payload.startswith(b"descendant="):
        raise AssertionError("descendant fixture emitted malformed readiness")
    try:
        return int(payload.removeprefix(b"descendant=").strip().decode("ascii"))
    except ValueError as error:
        raise AssertionError("descendant fixture emitted a non-numeric identity") from error


def process_probe() -> dict[str, object]:
    started = time.monotonic_ns()
    completed = subprocess.run(
        [sys.executable, __file__, "--fixture", "streams"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    duration_ms = round((time.monotonic_ns() - started) / 1_000_000, 3)
    if completed.stdout != b"stdout-complete\x00\xff\n":
        raise AssertionError("stdout was not preserved byte-for-byte")
    if completed.stderr != b"stderr-complete\x00\xfe\n":
        raise AssertionError("stderr was not preserved byte-for-byte")
    if completed.returncode != 23:
        raise AssertionError("numeric exit status was not preserved")

    readiness_read, readiness_write = os.pipe()
    try:
        child = subprocess.Popen(
            [sys.executable, __file__, "--fixture", "descendant", str(readiness_write)],
            start_new_session=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            pass_fds=(readiness_write,),
        )
    except BaseException:
        os.close(readiness_read)
        os.close(readiness_write)
        raise
    os.close(readiness_write)
    timeout_triggered = False
    termination_requested = False
    descendant_pid: int | None = None
    captured_stdout = b""
    captured_stderr = b""
    try:
        descendant_pid = await_descendant_readiness(readiness_read, child, 2.0)
        try:
            child.communicate(timeout=0.05)
        except subprocess.TimeoutExpired:
            timeout_triggered = True
        if not timeout_triggered:
            raise AssertionError("ready hanging process did not reach the timeout path")
    finally:
        os.close(readiness_read)
        if child.poll() is None:
            os.killpg(child.pid, signal.SIGKILL)
            termination_requested = True
        captured_stdout, captured_stderr = child.communicate(timeout=2)
    if descendant_pid is None:
        raise AssertionError("hanging process did not complete readiness")
    if captured_stderr:
        raise AssertionError("descendant fixture emitted unexpected stderr")
    if captured_stdout != f"descendant={descendant_pid}\n".encode("ascii"):
        raise AssertionError("complete descendant stdout was not preserved")
    descendant_stopped = await_not_running(descendant_pid, 2.0)
    if not descendant_stopped:
        raise AssertionError("process-group termination left a running descendant")

    return {
        "complete_stdout_sha256": hashlib.sha256(completed.stdout).hexdigest(),
        "complete_stderr_sha256": hashlib.sha256(completed.stderr).hexdigest(),
        "exit_code": completed.returncode,
        "duration_ms": duration_ms,
        "timeout_triggered": timeout_triggered,
        "termination_requested": termination_requested,
        "readiness_protocol": "dedicated inherited pipe observed before timeout",
        "direct_child_returncode": child.returncode,
        "direct_child_signal": -child.returncode if child.returncode is not None and child.returncode < 0 else None,
        "descendant_termination_observed": descendant_stopped,
        "cancellation_contract": "request, timeout trigger, termination request, direct-child result, and descendant observation remain separate facts"
    }


def repository_probe(root: Path) -> dict[str, object]:
    primary = root / "primary"
    linked = root / "linked"
    clone = root / "clone"
    primary.mkdir()
    git(primary, "init", "-q")
    git(primary, "config", "user.name", "V10 Fixture")
    git(primary, "config", "user.email", "v10@example.invalid")
    (primary / "tracked.txt").write_bytes(b"base\n")
    git(primary, "add", "tracked.txt")
    git(primary, "commit", "-qm", "fixture")
    git(primary, "worktree", "add", "-q", "-b", "linked", str(linked))
    run(["git", "clone", "-q", str(primary), str(clone)])

    def observation(repository: Path) -> dict[str, str]:
        common = Path(git(repository, "rev-parse", "--path-format=absolute", "--git-common-dir").decode().strip())
        git_dir = Path(git(repository, "rev-parse", "--path-format=absolute", "--git-dir").decode().strip())
        head = git(repository, "rev-parse", "HEAD").decode().strip()
        status = git(repository, "status", "--porcelain=v2", "-z", "--untracked-files=all", "--no-renames")
        return {
            "clone_identity": hash_fields(str(common.resolve()).encode()),
            "worktree_identity": hash_fields(str(git_dir.resolve()).encode()),
            "head": head,
            "dirty_identity": hash_fields(status),
            "dirty": str(bool(status)).lower(),
        }

    primary_clean = observation(primary)
    linked_clean = observation(linked)
    clone_clean = observation(clone)
    (linked / "tracked.txt").write_bytes(b"dirty linked worktree\n")
    linked_dirty = observation(linked)

    if primary_clean["clone_identity"] != linked_clean["clone_identity"]:
        raise AssertionError("linked worktrees did not share a local clone identity")
    if primary_clean["worktree_identity"] == linked_clean["worktree_identity"]:
        raise AssertionError("linked worktrees did not retain distinct worktree identities")
    if primary_clean["clone_identity"] == clone_clean["clone_identity"]:
        raise AssertionError("a separate clone was inferred to be the same local clone")
    if primary_clean["head"] != linked_clean["head"] or primary_clean["head"] != clone_clean["head"]:
        raise AssertionError("fixture repositories did not share the source revision")
    if linked_clean["dirty_identity"] == linked_dirty["dirty_identity"] or linked_dirty["dirty"] != "true":
        raise AssertionError("dirty worktree change was not observed")

    return {
        "primary": primary_clean,
        "linked": linked_clean,
        "clone": clone_clean,
        "linked_dirty": linked_dirty,
        "identity_contract": "local clone/worktree observation is not portable Project identity"
    }


def filesystem_probe(root: Path) -> dict[str, object]:
    repository = root / "paths"
    outside = root / "outside"
    repository.mkdir()
    outside.mkdir()
    nested = repository / "a" / "b"
    nested.mkdir(parents=True)
    target = outside / "secret"
    target.write_bytes(b"outside\n")
    os.symlink(outside, repository / "escape")
    normalized = (nested / ".." / "." / "file").resolve(strict=False)
    escaped = (repository / "escape" / "secret").resolve(strict=True)
    if normalized != repository / "a" / "file":
        raise AssertionError("lexical path normalization was not deterministic")
    if escaped.is_relative_to(repository.resolve()):
        raise AssertionError("symlink escape was not observable")

    regular = repository / "regular"
    regular.write_bytes(b"same bytes")
    symlink = repository / "link"
    os.symlink("regular", symlink)
    regular_fingerprint = hash_fields(b"regular", regular.read_bytes(), b"mode:644")
    symlink_fingerprint = hash_fields(b"symlink", os.readlink(symlink).encode())
    if regular_fingerprint == symlink_fingerprint:
        raise AssertionError("file and symlink fingerprints were conflated")

    first = repository / "first.staging"
    second = repository / "second.staging"
    destination = repository / "published"
    first.write_bytes(b"first")
    second.write_bytes(b"second")
    os.link(first, destination)
    first.unlink()
    destination_exists = False
    try:
        os.link(second, destination)
    except FileExistsError:
        destination_exists = True
    if not destination_exists or destination.read_bytes() != b"first" or second.read_bytes() != b"second":
        raise AssertionError("no-replace publication semantics were not preserved")

    return {
        "normalized_path": str(normalized.relative_to(repository)),
        "symlink_escape_rejected": True,
        "regular_fingerprint": regular_fingerprint,
        "symlink_fingerprint": symlink_fingerprint,
        "publication": "one complete destination published; existing destination and losing source preserved",
        "publication_probe_limit": "link/unlink proves no-replace namespace semantics; Linux renameat2 plus parent fsync remains the production candidate"
    }


def storage_probe(root: Path) -> dict[str, object]:
    database = root / "canonical-probe.sqlite"
    connection = sqlite3.connect(database)
    connection.execute("PRAGMA journal_mode=WAL")
    connection.execute("PRAGMA synchronous=FULL")
    connection.execute("CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
    connection.execute("CREATE TABLE records (id TEXT PRIMARY KEY, value TEXT NOT NULL)")
    connection.executemany(
        "INSERT INTO metadata(key, value) VALUES (?, ?)",
        [("schema_kind", "v10-probe"), ("schema_version", "1")],
    )
    connection.commit()
    connection.close()

    crashed = subprocess.run(
        [sys.executable, __file__, "--fixture", "uncommitted", str(database)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if crashed.returncode != 29:
        raise AssertionError("transaction crash fixture did not terminate at the fault point")
    reopened = sqlite3.connect(database)
    records = reopened.execute("SELECT id, value FROM records ORDER BY id").fetchall()
    metadata = dict(reopened.execute("SELECT key, value FROM metadata"))
    integrity = reopened.execute("PRAGMA integrity_check").fetchone()[0]
    reopened.close()
    if records or metadata != {"schema_kind": "v10-probe", "schema_version": "1"} or integrity != "ok":
        raise AssertionError("crash recovery did not preserve the committed transaction boundary")
    return {
        "uncommitted_rows_after_crash": len(records),
        "schema_kind": metadata["schema_kind"],
        "schema_version": metadata["schema_version"],
        "integrity_check": integrity,
        "classification": "reference only; canonical storage remains owned by volicord-context"
    }


def full_probe() -> dict[str, object]:
    if sys.platform != "linux":
        raise RuntimeError("V10 qualification requires Linux")
    if shutil.which("git") is None:
        raise RuntimeError("V10 qualification requires git")
    with tempfile.TemporaryDirectory(prefix="volicord-v10-") as temporary:
        root = Path(temporary)
        return {
            "schema_version": 1,
            "validation_id": "V10",
            "platform": sys.platform,
            "git_version": run(["git", "--version"]).stdout.decode().strip(),
            "process": process_probe(),
            "filesystem": filesystem_probe(root),
            "repository": repository_probe(root),
            "storage": storage_probe(root),
        }


def fixture(name: str, arguments: list[str]) -> int:
    if name == "streams":
        os.write(sys.stdout.fileno(), b"stdout-complete\x00\xff\n")
        os.write(sys.stderr.fileno(), b"stderr-complete\x00\xfe\n")
        return 23
    if name == "descendant":
        descendant = subprocess.Popen([sys.executable, __file__, "--fixture", "park"])
        readiness = int(arguments[0])
        os.write(readiness, f"descendant={descendant.pid}\n".encode("ascii"))
        os.close(readiness)
        print(f"descendant={descendant.pid}", flush=True)
        while True:
            time.sleep(60)
    if name == "park":
        while True:
            time.sleep(60)
    if name == "uncommitted":
        connection = sqlite3.connect(arguments[0])
        connection.execute("BEGIN IMMEDIATE")
        connection.execute("INSERT INTO records(id, value) VALUES ('uncommitted', 'must disappear')")
        os._exit(29)
    raise ValueError(f"unknown fixture: {name}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture")
    parser.add_argument("fixture_arguments", nargs="*")
    arguments = parser.parse_args()
    if arguments.fixture:
        return fixture(arguments.fixture, arguments.fixture_arguments)
    print(json.dumps(full_probe(), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
