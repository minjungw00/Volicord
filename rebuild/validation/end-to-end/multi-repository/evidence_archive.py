#!/usr/bin/env python3
"""Build a bounded, sanitized review archive from one validation gate run."""

from __future__ import annotations

import datetime as dt
import hashlib
import io
import json
import os
from pathlib import Path
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
KNOWN_EXECUTABLES = {
    "bash",
    "cargo",
    "check-fixture-manifest",
    "codex",
    "git",
    "harness.py",
    "install.sh",
    "python",
    "python3",
    "sh",
    "validate",
    "verify-validation-archive",
    "volicord",
    "zsh",
}
CARGO_SHAPES = (
    (
        ("metadata", "--manifest-path", None, "--no-deps", "--format-version", "1"),
        {2},
    ),
    (("fmt", "--manifest-path", None, "--all", "--", "--check"), {2}),
    (
        (
            "clippy",
            "--manifest-path",
            None,
            "--workspace",
            "--all-targets",
            "--all-features",
        ),
        {2},
    ),
    (
        (
            "test",
            "--manifest-path",
            None,
            "--workspace",
            "--all-targets",
            "--all-features",
        ),
        {2},
    ),
    (
        (
            "test",
            "--manifest-path",
            None,
            "-p",
            "volicord-operations",
            "--test",
            "v11_fixture_control",
            "--all-features",
            "--",
            "--exact",
            "seed_and_inspect_v11_forgetting_control",
            "--nocapture",
        ),
        {2},
    ),
)
VOLICORD_SHAPES = (
    (("codex", "enable"), 3, {}, {2}),
    (("project", "init"), 5, {3: "--repository"}, {4}),
    (("project", "bind"), 4, {}, {3}),
    (("analyze",), 2, {}, set()),
    (("canonical", "user-source"), 6, {}, set()),
    (("privacy", "enable"), 7, {}, set()),
    (("checkpoint", "record"), 8, {}, set()),
    (("recall",), 2, {}, set()),
    (("portable", "export"), 4, {}, {3}),
    (("portable", "import"), 3, {}, {2}),
    (("canonical", "supersede-decision"), 7, {}, set()),
    (("portable", "compare"), 5, {3: "--base"}, {2, 4}),
    (("portable", "resolve"), 9, {7: "--base"}, {2, 8}),
    (("canonical", "correct-decision"), 7, {}, set()),
    (("canonical", "forget"), 6, {}, set()),
    (("canonical", "inspect"), 3, {}, set()),
    (("documents", "export"), 7, {}, {5}),
    (("privacy", "status"), 3, {}, set()),
    (("health",), 2, {}, set()),
    (("repair",), 3, {}, set()),
    (("candidates",), 2, {}, set()),
)


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
    value: str,
    repository_root: Path,
    gate_directory: Path,
    working_directory: Path,
) -> tuple[str, str, str]:
    path = Path(value)
    if not path.is_absolute():
        if ".." in path.parts:
            return ("<redacted:path>", "redacted", "escaping_relative_path")
        resolved = working_directory / path
        official = official_v11_path(resolved, gate_directory)
        if official is not None:
            suffix = "" if official["path"] == "." else f"/{official['path']}"
            return (
                f"<official-v11:{official['target']}:{official['execution_root']}>{suffix}",
                "projected",
                "official_v11_path",
            )
        repository_relative = path_within(resolved, repository_root)
        if repository_relative is not None:
            suffix = repository_relative.as_posix()
            return (
                "." if suffix == "." else f"./{suffix}",
                "projected",
                "repository_path",
            )
        return ("<redacted:path>", "redacted", "unrecognized_relative_path")
    official = official_v11_path(path, gate_directory)
    if official is not None:
        suffix = "" if official["path"] == "." else f"/{official['path']}"
        return (
            f"<official-v11:{official['target']}:{official['execution_root']}>{suffix}",
            "projected",
            "official_v11_path",
        )
    gate_relative = path_within(path, gate_directory)
    if gate_relative is not None:
        suffix = gate_relative.as_posix()
        return (
            "<gate-artifact>" + (f"/{suffix}" if suffix != "." else ""),
            "projected",
            "gate_path",
        )
    repository_relative = path_within(path, repository_root)
    if repository_relative is not None:
        suffix = repository_relative.as_posix()
        local_parts = Path(suffix).parts
        if local_parts[:3] == ("rebuild", ".local", "validation"):
            remainder = Path(*local_parts[3:]).as_posix() if len(local_parts) > 3 else "."
            return (
                "<local-validation-artifact>"
                + (f"/{remainder}" if remainder != "." else ""),
                "projected",
                "local_validation_path",
            )
        return (
            "." if suffix == "." else f"./{suffix}",
            "projected",
            "repository_path",
        )
    return ("<redacted:absolute-path>", "redacted", "external_absolute_path")


def semantic_argument_roles(argv: list[str]) -> list[dict[str, str]]:
    roles = [
        {"classification": "redacted", "role": "unknown_operand"}
        for _argument in argv
    ]
    if not argv:
        return roles
    executable = Path(argv[0]).name
    if executable not in KNOWN_EXECUTABLES:
        roles[0] = {"classification": "redacted", "role": "unknown_executable"}
        return roles

    def structural(index: int, role: str = "structural_token") -> None:
        if 0 <= index < len(roles):
            roles[index] = {"classification": "structural", "role": role}

    def path(index: int) -> None:
        if 0 <= index < len(roles):
            roles[index] = {"classification": "path", "role": "owned_path"}

    def redact(index: int, role: str) -> None:
        if 0 <= index < len(roles):
            roles[index] = {"classification": "redacted", "role": role}

    structural(0, "executable")

    if executable in {"python", "python3", "bash", "sh", "zsh"}:
        if len(argv) >= 2 and argv[1] in {"-c", "-m"}:
            structural(1, "payload_flag")
            if len(argv) >= 3:
                redact(2, "inline_program" if argv[1] == "-c" else "module_operand")
            for index in range(3, len(argv)):
                redact(index, "program_argument")
        return roles

    if executable == "codex":
        fixed = {
            1: "--dangerously-bypass-hook-trust",
            2: "--ask-for-approval",
            3: "never",
            4: "--config",
            6: "exec",
            7: "--ephemeral",
            8: "--json",
            9: "--sandbox",
            10: "read-only",
            11: "--skip-git-repo-check",
            12: "-C",
        }
        if len(argv) == 15 and all(argv[index] == value for index, value in fixed.items()):
            for index in fixed:
                structural(index, "subcommand" if argv[index] == "exec" else "structural_token")
            redact(5, "config_payload")
            path(13)
            redact(14, "private_prompt")
        return roles

    if executable == "cargo":
        arguments = tuple(argv[1:])
        for shape, path_indexes in CARGO_SHAPES:
            if len(arguments) != len(shape) or any(
                expected is not None and arguments[index] != expected
                for index, expected in enumerate(shape)
            ):
                continue
            for relative, expected in enumerate(shape):
                absolute = relative + 1
                if relative in path_indexes:
                    path(absolute)
                elif expected is not None:
                    structural(
                        absolute,
                        "subcommand" if relative == 0 else (
                            "flag" if expected.startswith("-") else "closed_structural_value"
                        ),
                    )
            break
        return roles

    if executable == "git":
        index = 1
        while index + 1 < len(argv) and argv[index] == "-c":
            structural(index, "config_flag")
            redact(index + 1, "config_payload")
            index += 2
        if index >= len(argv) or argv[index] not in {"add", "clone", "commit", "init", "rev-parse"}:
            return roles
        subcommand = argv[index]
        structural(index, "subcommand")
        if subcommand == "clone" and argv[index + 1 : index + 3] == ["--quiet", "--no-hardlinks"] and len(argv) == index + 5:
            structural(index + 1, "flag")
            structural(index + 2, "flag")
            path(index + 3)
            path(index + 4)
        elif subcommand == "rev-parse" and argv[index + 1 :] == ["HEAD"]:
            structural(index + 1, "closed_structural_value")
        elif subcommand == "init" and argv[index + 1 :] in ([], ["--quiet"]):
            if len(argv) == index + 2:
                structural(index + 1, "flag")
        elif subcommand == "add" and len(argv) == index + 2:
            path(index + 1)
        elif subcommand == "commit" and argv[index + 1 : index + 3] == ["--quiet", "-m"] and len(argv) == index + 4:
            structural(index + 1, "flag")
            structural(index + 2, "flag")
            redact(index + 3, "message_payload")
        return roles

    if executable == "volicord":
        offset = 1
        if len(argv) >= 3 and argv[1] == "--runtime":
            structural(1, "flag")
            path(2)
            offset = 3
        arguments = argv[offset:]
        for prefix, expected_length, fixed, path_indexes in VOLICORD_SHAPES:
            if len(arguments) != expected_length or tuple(arguments[: len(prefix)]) != prefix:
                continue
            for relative in range(len(prefix)):
                structural(offset + relative, "subcommand" if relative else "command")
            fixed_match = all(
                arguments[relative] == expected for relative, expected in fixed.items()
            )
            if fixed_match:
                for relative in fixed:
                    structural(offset + relative, "flag")
                for relative in path_indexes:
                    path(offset + relative)
            for relative in range(len(prefix), len(arguments)):
                absolute = offset + relative
                if roles[absolute]["classification"] == "redacted":
                    redact(absolute, "sensitive_operand")
            return roles
        return roles

    if executable == "harness.py":
        shapes = {
            "self-check": (2, {}, set()),
            "credential-audit": (4, {2: "--artifact-dir"}, {3}),
            "preflight": (
                6,
                {2: "--validated-head", 4: "--final-artifact"},
                {5},
            ),
            "run": (
                8,
                {
                    2: "--validated-head",
                    4: "--final-artifact",
                    6: "--output-dir",
                },
                {5, 7},
            ),
        }
        subcommand = argv[1] if len(argv) >= 2 else None
        shape = shapes.get(subcommand)
        if shape is not None and len(argv) == shape[0] and all(
            argv[index] == value for index, value in shape[1].items()
        ):
            structural(1, "subcommand")
            for index, argument in shape[1].items():
                structural(index, "flag")
                if argument == "--validated-head":
                    redact(index + 1, "candidate_identity")
            for index in shape[2]:
                path(index)
        return roles

    if executable == "verify-validation-archive":
        if len(argv) == 4 and argv[2] == "--expected-candidate":
            path(1)
            structural(2, "flag")
            redact(3, "candidate_identity")
        return roles

    if executable == "check-fixture-manifest":
        if len(argv) == 2:
            path(1)
        return roles

    if executable == "install.sh":
        if len(argv) == 5 and argv[1] == "--prefix" and argv[3] == "--runtime-dir":
            structural(1, "flag")
            path(2)
            structural(3, "flag")
            path(4)
        return roles

    if executable == "validate":
        if len(argv) == 2 and argv[1] in {
            "evidence-archive-self-test",
            "gate-entrypoint-self-test",
            "gate-self-test",
            "self-test",
        }:
            structural(1, "subcommand")
        return roles

    return roles


def sanitized_argv(
    argv: list[str],
    repository_root: Path,
    gate_directory: Path,
    working_directory: Path | None = None,
) -> dict[str, Any]:
    if not argv:
        raise ValueError("execution record has empty argv")
    roles = semantic_argument_roles(argv)
    projected: list[str] = []
    sanitizations: list[dict[str, Any]] = []
    argument_classifications: list[str] = []
    for index, (argument, decision) in enumerate(zip(argv, roles)):
        classification = decision["classification"]
        role = decision["role"]
        replacement = argument
        if index == 0 and classification == "structural":
            replacement = Path(argument).name
            if replacement != argument:
                classification = "projected"
                role = "executable_path"
        elif classification == "path":
            replacement, classification, role = projected_path_argument(
                argument,
                repository_root,
                gate_directory,
                working_directory or repository_root,
            )
        elif classification == "redacted":
            replacement = f"<redacted:{role.replace('_', '-')}>"
        projected.append(replacement)
        argument_classifications.append(classification)
        if classification != "structural":
            sanitizations.append({"argument_index": index, "reason": role})
    return {
        "argv": projected,
        "argv_completeness": "sanitized_portable_projection",
        "argv_projection_policy": "explicit_semantic_argument_roles",
        "raw_argv_retained_locally": True,
        "sanitization_applied": bool(sanitizations),
        "sanitized_argument_count": len(sanitizations),
        "sanitizations": sanitizations,
        "argument_classifications": argument_classifications,
    }


def sanitized_execution(
    value: dict[str, Any], repository_root: Path, gate_directory: Path
) -> dict[str, Any]:
    argv = value.get("argv")
    if not isinstance(argv, list) or not all(isinstance(item, str) for item in argv):
        raise ValueError("execution record has invalid argv")
    spawn_error = value.get("spawn_error")
    return {
        **sanitized_argv(
            argv,
            repository_root,
            gate_directory,
            Path(str(value.get("working_directory"))),
        ),
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
