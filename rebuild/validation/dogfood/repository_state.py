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


def git(repository, *arguments, allowed_returncodes=(0,)):
    result = subprocess.run(
        ["git", "--no-optional-locks", "-c", "core.fsmonitor=false", "-c",
         "core.quotePath=true", *arguments], cwd=repository,
        env={**os.environ, "GIT_OPTIONAL_LOCKS": "0", "LC_ALL": "C"},
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode not in allowed_returncodes:
        raise StateError("cannot attest Git repository: " + result.stderr.decode(errors="replace"))
    return result.stdout


def path_text(raw):
    # Surrogate escaping preserves non-UTF8 Git filenames in deterministic JSON.
    value = os.fsdecode(raw)
    path = PurePosixPath(value)
    if not value or path.is_absolute() or any(p in {"", ".", "..", ".git"} for p in value.split("/")):
        raise StateError("unexpected repository path escape")
    return value


def ignored_path(repository, name):
    # Check one path's eligibility without enumerating or reading ignored content.
    return bool(git(repository, "check-ignore", "--no-index", "--", name,
        allowed_returncodes=(0, 1)))


def check_replacement_directory(repository, name, replacement_leaves):
    # Git may omit special files from its untracked listing. Check metadata,
    # including empty containers, without turning excluded bytes into evidence.
    for child in sorted((repository / name).iterdir()):
        child_name = path_text(os.fsencode(child.relative_to(repository).as_posix()))
        mode = child.lstat().st_mode
        if not (stat.S_ISDIR(mode) or stat.S_ISREG(mode) or stat.S_ISLNK(mode)):
            raise StateError("unsupported repository entry (including submodule or special file)")
        if stat.S_ISLNK(mode) and not child.resolve().is_relative_to(repository):
            raise StateError("unexpected repository symlink escape")
        if ignored_path(repository, child_name):
            continue
        if stat.S_ISDIR(mode):
            check_replacement_directory(repository, child_name, replacement_leaves)
        elif child_name not in replacement_leaves:
            raise StateError("missing attested repository replacement file")


def file_state(repository, name, *, missing_allowed=False, replacement_leaves=(), git_deleted=False):
    path = repository / name
    # Never traverse a directory symlink, even one currently pointing inside.
    parent = repository
    for component in PurePosixPath(name).parts[:-1]:
        parent /= component
        try:
            parent_mode = parent.lstat().st_mode
        except FileNotFoundError:
            break  # The leaf's missing-path check below still applies.
        if stat.S_ISLNK(parent_mode):
            raise StateError("unexpected repository path escape")
        if not stat.S_ISDIR(parent_mode):
            # A regular replacement may be attested or intentionally ignored.
            # Git deletion proof is independent of that replacement's eligibility.
            # Never traverse symlinks or treat a special object as deletion.
            parent_name = parent.relative_to(repository).as_posix()
            if missing_allowed and stat.S_ISREG(parent_mode) and (
                parent_name in replacement_leaves or
                (git_deleted and ignored_path(repository, parent_name))
            ):
                return {"path": name, "state": "deleted"}
            raise StateError("unsupported repository entry (including submodule or special file)")
    try:
        mode = path.lstat().st_mode
    except FileNotFoundError:
        if missing_allowed:
            return {"path": name, "state": "deleted"}
        raise StateError("missing attested repository file")
    if stat.S_ISLNK(mode):
        if not path.resolve().is_relative_to(repository):
            raise StateError("unexpected repository symlink escape")
        # Validate the replacement's safety before excluding its link text.
        if missing_allowed and git_deleted and ignored_path(repository, name):
            return {"path": name, "state": "deleted"}
        content = os.fsencode(os.readlink(path))
        kind = "symlink"
    elif stat.S_ISREG(mode):
        if missing_allowed and git_deleted and ignored_path(repository, name):
            return {"path": name, "state": "deleted"}
        content = path.read_bytes()
        kind = "executable" if mode & 0o111 else "file"
    elif stat.S_ISDIR(mode) and missing_allowed and git_deleted:
        # Git proves this historical/index leaf was deleted. Current nonignored
        # descendants have already passed file checks; empty directories and
        # ignored descendants create no attested content objects.
        check_replacement_directory(repository, name, replacement_leaves)
        return {"path": name, "state": "deleted"}
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
    index_names = {item["path"] for item in index}
    head_names = set()
    for record in git(repository, "ls-tree", "-r", "-z", "HEAD").split(b"\0"):
        if record:
            metadata, name = record.split(b"\t", 1)
            if metadata.split()[0] == b"160000":
                raise StateError("unverifiable HEAD submodule")
            head_names.add(path_text(name))
    deleted_head = {item["path"] for item in status if item["index"] == "D"} & (head_names - index_names)
    deleted_index = {item["path"] for item in status if item["worktree"] == "D"} & index_names
    deleted = deleted_head | deleted_index
    untracked_names = [path_text(name) for name in sorted(git(repository, "ls-files", "--others", "--exclude-standard", "-z").split(b"\0")) if name]
    # Attest current leaves first. HEAD-only staged deletions and index leaves
    # deleted in the worktree are historical paths, not current filesystem leaves.
    current = {name: file_state(repository, name) for name in sorted((index_names - deleted_index) | set(untracked_names))}
    tracked = [current[name] if name in current else file_state(repository, name,
        missing_allowed=name in deleted, replacement_leaves=current, git_deleted=name in deleted)
        for name in sorted(head_names | index_names)]
    untracked = [current[name] for name in untracked_names]
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
