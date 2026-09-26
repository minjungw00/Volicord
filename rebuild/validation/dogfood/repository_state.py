"""Read-only Git attestation for the observed naturalistic journey end state.

This supplements Source-bound analysis/checkpoint evidence; it creates no
Product snapshot or canonical authority. Ignored content is outside its scope.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import subprocess


class StateError(ValueError):
    pass


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True, ensure_ascii=True, separators=(",", ":")) + "\n").encode()


def git(repository, *arguments):
    result = subprocess.run(
        ["git", "--no-optional-locks", "-c", "core.fsmonitor=false", "-c",
         "core.quotePath=true", *arguments], cwd=repository,
        env={**os.environ, "GIT_OPTIONAL_LOCKS": "0", "LC_ALL": "C"},
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode:
        raise StateError("cannot attest Git repository: " + result.stderr.decode(errors="replace"))
    return result.stdout


def path_text(raw):
    # Surrogate escaping preserves non-UTF8 Git filenames in deterministic JSON.
    value = os.fsdecode(raw)
    path = PurePosixPath(value)
    if not value or path.is_absolute() or any(p in {"", ".", "..", ".git"} for p in value.split("/")):
        raise StateError("unexpected repository path escape")
    return value


def file_state(repository, name, *, missing_allowed=False):
    path = repository / name
    # Never traverse a directory symlink, even one currently pointing inside.
    parent = repository
    for component in PurePosixPath(name).parts[:-1]:
        parent /= component
        if parent.is_symlink():
            raise StateError("unexpected repository path escape")
    try:
        mode = path.lstat().st_mode
    except FileNotFoundError:
        if missing_allowed:
            return {"path": name, "state": "deleted"}
        raise StateError("missing attested repository file")
    if stat.S_ISLNK(mode):
        if not path.resolve().is_relative_to(repository):
            raise StateError("unexpected repository symlink escape")
        content = os.fsencode(os.readlink(path))
        kind = "symlink"
    elif stat.S_ISREG(mode):
        content = path.read_bytes()
        kind = "executable" if mode & 0o111 else "file"
    else:
        raise StateError("unsupported repository entry (including submodule or special file)")
    return {"path": name, "state": kind, "bytes": len(content), "sha256": digest(content)}


def observe(repository: Path):
    if repository.is_symlink() or any(parent.is_symlink() for parent in repository.parents):
        raise StateError("unexpected repository root path escape")
    repository = repository.resolve(strict=True)
    top = Path(os.fsdecode(git(repository, "rev-parse", "--show-toplevel")).strip()).resolve(strict=True)
    if top != repository:
        raise StateError("wrong repository identity: journey root is not the Git worktree root")
    head = git(repository, "rev-parse", "HEAD").decode().strip()
    status_raw = git(repository, "-c", "status.renames=false", "status", "--porcelain=v1", "-z", "--untracked-files=all")
    status = []
    for record in status_raw.split(b"\0"):
        if not record:
            continue
        if len(record) < 4 or record[2:3] != b" ":
            raise StateError("unverifiable Git status")
        status.append({"path": path_text(record[3:]), "index": chr(record[0]), "worktree": chr(record[1])})
    status.sort(key=lambda item: item["path"])
    index = []
    for record in git(repository, "ls-files", "--stage", "-z").split(b"\0"):
        if record:
            metadata, name = record.split(b"\t", 1)
            mode, object_id, stage = metadata.decode().split()
            if stage != "0" or mode == "160000":
                raise StateError("unverifiable unmerged index or submodule")
            index.append({"path": path_text(name), "mode": mode, "object_id": object_id})
    index.sort(key=lambda item: item["path"])
    tracked_names = {item["path"] for item in index}
    tracked_names.update(path_text(name) for name in git(repository, "ls-tree", "-r", "--name-only", "-z", "HEAD").split(b"\0") if name)
    deleted = {item["path"] for item in status if "D" in (item["index"], item["worktree"])}
    tracked = [file_state(repository, name, missing_allowed=name in deleted) for name in sorted(tracked_names)]
    untracked = [file_state(repository, path_text(name)) for name in sorted(git(repository, "ls-files", "--others", "--exclude-standard", "-z").split(b"\0")) if name]
    options = ("--binary", "--full-index", "--no-ext-diff", "--no-textconv", "--no-renames", "--src-prefix=a/", "--dst-prefix=b/")
    patches = {"staged": git(repository, "diff", "--cached", *options, "HEAD", "--"),
               "unstaged": git(repository, "diff", *options, "--")}
    value = {"kind": "dogfood_journey_repository_state", "schema_version": 1,
             "final_head": head, "status": status, "index": index,
             "tracked": tracked, "untracked": untracked,
             "diffs": {key: {"bytes": len(data), "sha256": digest(data)} for key, data in patches.items()},
             "workspace_clean": not status,
             "boundary": "HEAD_index_tracked_and_nonignored_untracked; ignored_content_excluded"}
    value["fingerprint"] = digest(b"dogfood-journey-repository-state\0" + encoded(value))
    # Detect ordinary concurrent mutation; this is observation, not filesystem locking.
    if git(repository, "rev-parse", "HEAD").decode().strip() != head or git(repository, "-c", "status.renames=false", "status", "--porcelain=v1", "-z", "--untracked-files=all") != status_raw:
        raise StateError("repository changed during attestation")
    return value, patches


def verify(repository, expected):
    observed, _ = observe(repository)
    if observed != expected:
        raise StateError("attested repository state changed before publication")


def verify_retained(value, patches):
    """Verify portable immutable evidence without requiring the original workspace."""
    if not isinstance(value, dict) or value.get("kind") != "dogfood_journey_repository_state" or value.get("schema_version") != 1:
        raise StateError("unverifiable repository-state attestation")
    body = {key: item for key, item in value.items() if key != "fingerprint"}
    if value.get("fingerprint") != digest(b"dogfood-journey-repository-state\0" + encoded(body)):
        raise StateError("tampered repository-state fingerprint")
    for key in ("staged", "unstaged"):
        data = patches[key]
        if value.get("diffs", {}).get(key) != {"bytes": len(data), "sha256": digest(data)}:
            raise StateError("tampered repository-state diff")
    for item in value.get("tracked", []) + value.get("untracked", []):
        path_text(os.fsencode(item["path"]))
