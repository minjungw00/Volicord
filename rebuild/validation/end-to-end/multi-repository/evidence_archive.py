#!/usr/bin/env python3
"""Build a bounded, sanitized review archive from one validation gate run."""

from __future__ import annotations

import datetime as dt
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
from typing import Any


ARCHIVE_ROOT = "validation-evidence"
ARCHIVE_MAX_BYTES = 512 * 1024
PAYLOAD_MODE = 0o644
DIRECTORY_MODE = 0o755
TRACKED_EVIDENCE_PATHS = (
    "rebuild/scripts/validate",
    "rebuild/scripts/verify-validation-archive",
    "rebuild/validation/end-to-end/multi-repository/evidence_archive.py",
    "rebuild/validation/end-to-end/multi-repository/gate.py",
    "rebuild/validation/end-to-end/multi-repository/harness.py",
    "rebuild/scripts/check-fixture-manifest",
)
V11_TARGETS = {"volicord", "small-python", "polyglot-medium"}
V11_EXECUTION_ROOTS = {"repository", "clone"}
OPAQUE_ARGUMENT = re.compile(
    r"(?:[0-9a-fA-F]{32,}|[0-9a-fA-F]{8}-(?:[0-9a-fA-F]{4}-){3}[0-9a-fA-F]{12})"
)
SAFE_ARGUMENT = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.+-]*")


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="microseconds")


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def path_within(path: Path, parent: Path) -> Path | None:
    try:
        return path.resolve(strict=False).relative_to(parent.resolve(strict=False))
    except ValueError:
        return None


def official_v11_path(path: Path, gate_directory: Path) -> dict[str, str] | None:
    relative = path_within(path, gate_directory / "official-v11" / "work")
    if relative is None or len(relative.parts) < 2:
        return None
    target, execution_root, *remainder = relative.parts
    if target not in V11_TARGETS or execution_root not in V11_EXECUTION_ROOTS:
        return None
    return {
        "kind": "official_v11_target",
        "target": target,
        "execution_root": execution_root,
        "path": Path(*remainder).as_posix() if remainder else ".",
    }


def normalized_working_directory(
    value: object, repository_root: Path, gate_directory: Path
) -> dict[str, str]:
    if not isinstance(value, str):
        raise ValueError("execution record has invalid working directory")
    path = Path(value)
    if path == repository_root:
        return {"kind": "repository_root", "path": "."}
    projected = official_v11_path(path, gate_directory)
    if projected is not None:
        return projected
    raise ValueError("review archive rejected an unrecognized external working directory")


def projected_path_argument(
    value: str, repository_root: Path, gate_directory: Path
) -> tuple[str, str] | None:
    path = Path(value)
    if not path.is_absolute():
        return None
    official = official_v11_path(path, gate_directory)
    if official is not None:
        suffix = "" if official["path"] == "." else f"/{official['path']}"
        return (
            f"<official-v11:{official['target']}:{official['execution_root']}>{suffix}",
            "official_v11_path",
        )
    gate_relative = path_within(path, gate_directory)
    if gate_relative is not None:
        suffix = gate_relative.as_posix()
        return ("<gate-artifact>" + (f"/{suffix}" if suffix != "." else ""), "gate_path")
    repository_relative = path_within(path, repository_root)
    if repository_relative is not None:
        suffix = repository_relative.as_posix()
        return ("." if suffix == "." else f"./{suffix}", "repository_path")
    return ("<redacted:absolute-path>", "external_absolute_path")


def sanitized_argv(
    argv: list[str], repository_root: Path, gate_directory: Path
) -> dict[str, Any]:
    projected: list[str] = []
    sanitizations: list[dict[str, Any]] = []
    redact_next = False
    codex_exec_seen = False
    for index, argument in enumerate(argv):
        replacement = argument
        reason: str | None = None
        path_projection = projected_path_argument(argument, repository_root, gate_directory)
        if index == 0:
            if path_projection is not None:
                replacement = Path(argument).name
                reason = "executable_path"
        elif redact_next:
            replacement = "<redacted:argument-payload>"
            reason = "flag_payload"
            redact_next = False
        elif path_projection is not None:
            replacement, reason = path_projection
        elif argument in {"-c", "--config", "-m", "--message"}:
            redact_next = True
        elif argument == "exec" and any(Path(value).name == "codex" for value in argv[:1]):
            codex_exec_seen = True
        elif codex_exec_seen and index == len(argv) - 1:
            replacement = "<redacted:private-prompt>"
            reason = "private_prompt"
        elif OPAQUE_ARGUMENT.fullmatch(argument):
            replacement = "<redacted:opaque-identity>"
            reason = "opaque_identity"
        elif not argument.startswith("-") and not SAFE_ARGUMENT.fullmatch(argument):
            replacement = "<redacted:argument-payload>"
            reason = "unclassified_payload"
        projected.append(replacement)
        if reason is not None:
            sanitizations.append({"argument_index": index, "reason": reason})
    return {
        "argv": projected,
        "argv_completeness": "sanitized_portable_projection",
        "raw_argv_retained_locally": True,
        "sanitization_applied": bool(sanitizations),
        "sanitized_argument_count": len(sanitizations),
        "sanitizations": sanitizations,
    }


def sanitized_execution(
    value: dict[str, Any], repository_root: Path, gate_directory: Path
) -> dict[str, Any]:
    argv = value.get("argv")
    if not isinstance(argv, list) or not all(isinstance(item, str) for item in argv):
        raise ValueError("execution record has invalid argv")
    spawn_error = value.get("spawn_error")
    return {
        **sanitized_argv(argv, repository_root, gate_directory),
        "working_directory": normalized_working_directory(
            value.get("working_directory"), repository_root, gate_directory
        ),
        "started_at": value.get("started_at"),
        "ended_at": value.get("ended_at"),
        "duration_ms": value.get("duration_ms"),
        "outcome": value.get("outcome"),
        "exit_code": value.get("exit_code"),
        "wrapper_exit_code": value.get("wrapper_exit_code"),
        "termination": value.get("termination"),
        "spawn": {
            "status": "failed" if spawn_error is not None else "started",
            "error_type": (
                str(spawn_error).split(":", 1)[0]
                if spawn_error is not None
                else None
            ),
        },
    }


def sanitized_final_summary(
    summary: dict[str, Any], repository_root: Path, gate_directory: Path
) -> dict[str, Any]:
    commands = summary.get("commands")
    if not isinstance(commands, list):
        raise ValueError("final summary commands are unavailable")
    return {
        "kind": "sanitized_exact_final_summary",
        "working_directory": normalized_working_directory(
            summary.get("working_directory"), repository_root, gate_directory
        ),
        "started_at": summary.get("started_at"),
        "ended_at": summary.get("ended_at"),
        "duration_ms": summary.get("duration_ms"),
        "command_count": summary.get("command_count"),
        "failure_count": summary.get("failure_count"),
        "outcome": summary.get("outcome"),
        "commands": [
            sanitized_execution(command, repository_root, gate_directory)
            for command in commands
        ],
    }


def sanitized_admission(value: dict[str, Any]) -> dict[str, Any]:
    return {
        "kind": "sanitized_validation_admission",
        "status": value.get("status"),
        "eligible": value.get("eligible"),
        "blocking_classification": value.get("blocking_classification"),
        "candidate_head": value.get("candidate_head"),
        "checks": [
            {"name": check.get("name"), "status": check.get("status")}
            for check in value.get("checks", [])
            if isinstance(check, dict)
        ],
        "required_fixture_identities": value.get("required_fixture_identities", []),
        "execution_environment": value.get("execution_environment", {}),
        "dependency_snapshot": value.get("dependency_snapshot", {}),
        "gate_configuration": value.get("gate_configuration", {}),
        "external_transmission": value.get("external_transmission", {}),
        "final_command_count": value.get("final_command_count"),
        "official_v11_command_count": value.get("official_v11_command_count"),
    }


def collected_processes(gate_directory: Path, repository_root: Path) -> dict[str, Any]:
    processes: list[dict[str, Any]] = []
    for path in sorted(gate_directory.rglob("result.json")):
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        if not isinstance(value, dict) or "argv" not in value or "working_directory" not in value:
            continue
        processes.append(
            {
                "artifact": path.relative_to(gate_directory).as_posix(),
                **sanitized_execution(value, repository_root, gate_directory),
            }
        )
    return {"kind": "sanitized_gate_processes", "processes": processes}


def tracked_file_evidence(repository_root: Path, candidate_head: str) -> dict[str, Any]:
    files = []
    for relative in TRACKED_EVIDENCE_PATHS:
        completed = subprocess.run(
            ["git", "ls-tree", candidate_head, "--", relative],
            cwd=repository_root,
            text=True,
            capture_output=True,
            check=False,
        )
        fields = completed.stdout.rstrip("\n").split(None, 3)
        if completed.returncode != 0 or len(fields) != 4:
            raise ValueError(f"candidate does not track required archive evidence file: {relative}")
        mode, object_type, object_id, listed_path = fields
        if object_type != "blob" or listed_path != relative or mode not in {"100644", "100755"}:
            raise ValueError(f"candidate has invalid tracked file mode or identity: {relative}")
        blob = subprocess.run(
            ["git", "cat-file", "blob", object_id],
            cwd=repository_root,
            capture_output=True,
            check=False,
        )
        if blob.returncode != 0:
            raise ValueError(f"candidate blob is unavailable: {relative}")
        files.append(
            {
                "path": relative,
                "tracked": True,
                "git_mode": mode,
                "executable": mode == "100755",
                "git_object_id": object_id,
                "sha256": hashlib.sha256(blob.stdout).hexdigest(),
            }
        )
    return {
        "kind": "candidate_tracked_file_modes",
        "candidate_head": candidate_head,
        "files": files,
    }


def write_archive(
    archive_path: Path,
    *,
    candidate_head: str,
    payloads: dict[str, object],
    source_final_summary_sha256: str | None,
) -> dict[str, Any]:
    encoded = {name: json_bytes(value) for name, value in payloads.items()}
    mode_entries = [
        {"path": ARCHIVE_ROOT, "type": "directory", "mode": "0755"},
        {
            "path": f"{ARCHIVE_ROOT}/archive-manifest.json",
            "type": "file",
            "mode": "0644",
        },
        *[
            {
                "path": f"{ARCHIVE_ROOT}/{name}",
                "type": "file",
                "mode": "0644",
            }
            for name in sorted((*encoded.keys(), "file-modes.json"))
        ],
    ]
    mode_bytes = json_bytes({"kind": "archive_file_modes", "entries": mode_entries})
    encoded["file-modes.json"] = mode_bytes
    manifest = {
        "kind": "validation_evidence_archive_manifest",
        "candidate_head": candidate_head,
        "created_at": utc_now(),
        "source_final_summary_sha256": source_final_summary_sha256,
        "capsule_sha256": sha256_bytes(encoded["capsule.json"]),
        "files": {
            name: {
                "sha256": sha256_bytes(content),
                "size_bytes": len(content),
                "mode": "0644",
            }
            for name, content in sorted(encoded.items())
        },
    }
    manifest_bytes = json_bytes(manifest)

    archive_path.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive_path, "w:gz", format=tarfile.PAX_FORMAT) as archive:
        directory = tarfile.TarInfo(ARCHIVE_ROOT)
        directory.type = tarfile.DIRTYPE
        directory.mode = DIRECTORY_MODE
        directory.uid = directory.gid = 0
        directory.uname = directory.gname = ""
        directory.mtime = 0
        archive.addfile(directory)
        for name, content in (
            ("archive-manifest.json", manifest_bytes),
            *sorted(encoded.items()),
        ):
            member = tarfile.TarInfo(f"{ARCHIVE_ROOT}/{name}")
            member.size = len(content)
            member.mode = PAYLOAD_MODE
            member.uid = member.gid = 0
            member.uname = member.gname = ""
            member.mtime = 0
            archive.addfile(member, io.BytesIO(content))
    os.chmod(archive_path, 0o600)
    size = archive_path.stat().st_size
    if size > ARCHIVE_MAX_BYTES:
        archive_path.unlink()
        raise ValueError("sanitized evidence archive exceeded its size bound")
    return {
        "kind": "validation_evidence_archive_identity",
        "candidate_head": candidate_head,
        "path": str(archive_path),
        "sha256": hashlib.sha256(archive_path.read_bytes()).hexdigest(),
        "size_bytes": size,
        "member_count": 2 + len(encoded),
    }


def create_review_archive(
    *,
    repository_root: Path,
    gate_directory: Path,
    candidate_head: str,
    admission: dict[str, Any],
    capsule: dict[str, Any],
    gate_result: dict[str, Any],
    final_summary: dict[str, Any] | None,
) -> dict[str, Any]:
    payloads: dict[str, object] = {
        "admission.json": sanitized_admission(admission),
        "capsule.json": capsule,
        "gate-result.json": gate_result,
        "processes.json": collected_processes(gate_directory, repository_root),
        "tracked-files.json": tracked_file_evidence(repository_root, candidate_head),
    }
    if final_summary is not None:
        payloads["final-summary.json"] = sanitized_final_summary(
            final_summary, repository_root, gate_directory
        )
    archive_path = gate_directory / f"validation-evidence-{candidate_head[:12]}.tar.gz"
    return write_archive(
        archive_path,
        candidate_head=candidate_head,
        payloads=payloads,
        source_final_summary_sha256=capsule.get("final_summary_sha256"),
    )
