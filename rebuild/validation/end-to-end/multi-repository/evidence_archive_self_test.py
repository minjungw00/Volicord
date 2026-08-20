#!/usr/bin/env python3
"""Positive and negative checks for sanitized validation evidence archives."""

from __future__ import annotations

import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
from typing import Any, Callable


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
BUILDER = HERE / "evidence_archive.py"
VERIFIER = ROOT / "rebuild/scripts/verify-validation-archive"
HEAD = "a" * 40
PROMPT_SENTINEL = "PROMPT_SENTINEL_MUST_NOT_SURVIVE_7f91"
sys.dont_write_bytecode = True


def load_builder() -> Any:
    specification = importlib.util.spec_from_file_location(
        "volicord_evidence_archive_self_test_target", BUILDER
    )
    if specification is None or specification.loader is None:
        raise RuntimeError("could not load evidence archive builder")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


builder = load_builder()


def payloads() -> dict[str, object]:
    return {
        "admission.json": {
            "kind": "sanitized_validation_admission",
            "candidate_head": HEAD,
            "status": "blocked",
        },
        "capsule.json": {
            "kind": "validation_handoff_capsule",
            "validated_candidate_head": HEAD,
            "final_summary_sha256": None,
            "evidence_archive": {"status": "pending"},
            "phase_8_ready": False,
        },
        "gate-result.json": {
            "kind": "validation_gate_result",
            "candidate_head": HEAD,
            "status": "blocked",
            "evidence_archive_status": "pending",
        },
        "processes.json": {
            "kind": "sanitized_gate_processes",
            "processes": [],
        },
        "tracked-files.json": {
            "kind": "candidate_tracked_file_modes",
            "candidate_head": HEAD,
            "files": [
                {
                    "path": "rebuild/scripts/validate",
                    "tracked": True,
                    "git_mode": "100755",
                    "executable": True,
                    "git_object_id": "b" * 40,
                    "sha256": "c" * 64,
                }
            ],
        },
    }


def run_verifier(path: Path, expected: str = HEAD) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(VERIFIER), str(path), "--expected-candidate", expected],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )


def execution(argv: list[str], cwd: Path) -> dict[str, object]:
    return {
        "argv": argv,
        "working_directory": str(cwd),
        "started_at": "2026-08-21T00:00:00.000000+00:00",
        "ended_at": "2026-08-21T00:00:00.001000+00:00",
        "duration_ms": 1.0,
        "outcome": "succeeded",
        "exit_code": 0,
        "wrapper_exit_code": 0,
        "termination": None,
        "spawn_error": None,
    }


def integration_archive(root: Path) -> tuple[Path, str]:
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, capture_output=True, check=True
    ).stdout.strip()
    gate = root / "synthetic-gate"
    repository_result = gate / "admission-checks" / "runner" / "result.json"
    target = gate / "official-v11" / "work" / "small-python" / "repository"
    target_result = gate / "official-v11" / "operations" / "result.json"
    target.mkdir(parents=True)
    repository_result.parent.mkdir(parents=True)
    target_result.parent.mkdir(parents=True)
    repository_result.write_text(
        json.dumps(execution(["cargo", "test", "--workspace"], ROOT)), encoding="utf-8"
    )
    target_result.write_text(
        json.dumps(
            execution(
                [
                    "/opt/codex/bin/codex",
                    "exec",
                    "--ephemeral",
                    "-C",
                    str(target),
                    PROMPT_SENTINEL,
                ],
                target,
            )
        ),
        encoding="utf-8",
    )
    final = {
        "working_directory": str(ROOT),
        "started_at": "2026-08-21T00:00:00.000000+00:00",
        "ended_at": "2026-08-21T00:00:00.001000+00:00",
        "duration_ms": 1.0,
        "command_count": 1,
        "failure_count": 0,
        "outcome": "succeeded",
        "commands": [execution(["cargo", "test", "--workspace"], ROOT)],
    }
    final_hash = "1" * 64
    admission = {
        "candidate_head": head,
        "status": "eligible",
        "eligible": True,
        "checks": [],
    }
    capsule = {
        "kind": "validation_handoff_capsule",
        "validated_candidate_head": head,
        "final_summary_sha256": final_hash,
        "phase_8_ready": False,
        "blocking_classification": "evidence_archive_pending",
        "evidence_archive": {"status": "pending"},
    }
    gate_result = {
        "kind": "validation_gate_result",
        "candidate_head": head,
        "status": "blocked",
        "blocking_classification": "evidence_archive_pending",
        "evidence_archive_status": "pending",
    }
    identity = builder.create_review_archive(
        repository_root=ROOT,
        gate_directory=gate,
        candidate_head=head,
        admission=admission,
        capsule=capsule,
        gate_result=gate_result,
        final_summary=final,
    )
    archive = Path(identity["path"])
    verified = run_verifier(archive, head)
    assert verified.returncode == 0, verified.stderr
    with tarfile.open(archive, "r:gz") as retained:
        bodies = b"".join(
            retained.extractfile(member).read()
            for member in retained.getmembers()
            if member.isfile()
        )
        processes = json.loads(
            retained.extractfile("validation-evidence/processes.json").read()
        )
    assert PROMPT_SENTINEL.encode() not in bodies
    assert str(ROOT).encode() not in bodies
    assert str(root).encode() not in bodies
    target_process = next(
        process
        for process in processes["processes"]
        if process["working_directory"]["kind"] == "official_v11_target"
    )
    assert target_process["working_directory"] == {
        "kind": "official_v11_target",
        "target": "small-python",
        "execution_root": "repository",
        "path": ".",
    }
    assert target_process["argv"][-1] == "<redacted:private-prompt>"
    assert target_process["argv_completeness"] == "sanitized_portable_projection"
    assert target_process["raw_argv_retained_locally"] is True

    unknown = gate / "unknown" / "result.json"
    unknown.parent.mkdir()
    unknown.write_text(
        json.dumps(execution(["tool"], root / "arbitrary-external-cwd")), encoding="utf-8"
    )
    try:
        builder.create_review_archive(
            repository_root=ROOT,
            gate_directory=gate,
            candidate_head=head,
            admission=admission,
            capsule=capsule,
            gate_result=gate_result,
            final_summary=final,
        )
    except ValueError as error:
        assert "unrecognized external working directory" in str(error)
    else:
        raise AssertionError("unknown external working directory was accepted")
    return archive, head


def rewrite_archive(
    source: Path,
    destination: Path,
    transform: Callable[[tarfile.TarInfo, bytes | None], tuple[tarfile.TarInfo, bytes | None] | None],
) -> None:
    members: list[tuple[tarfile.TarInfo, bytes | None]] = []
    with tarfile.open(source, "r:*") as archive:
        for original in archive.getmembers():
            member = tarfile.TarInfo(original.name)
            member.type = original.type
            member.mode = original.mode
            member.uid = original.uid
            member.gid = original.gid
            member.mtime = original.mtime
            body = archive.extractfile(original).read() if original.isfile() else None
            changed = transform(member, body)
            if changed is not None:
                members.append(changed)
    with tarfile.open(destination, "w:gz", format=tarfile.PAX_FORMAT) as archive:
        for member, body in members:
            member.size = len(body) if body is not None else 0
            archive.addfile(member, io.BytesIO(body) if body is not None else None)


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="volicord-evidence-archive-self-test-") as directory:
        root = Path(directory)
        integration_archive(root)
        archive = root / "positive.tar.gz"
        builder.write_archive(
            archive,
            candidate_head=HEAD,
            payloads=payloads(),
            source_final_summary_sha256=None,
        )
        positive = run_verifier(archive)
        assert positive.returncode == 0, positive.stderr
        assert json.loads(positive.stdout)["candidate_head"] == HEAD

        tampered = root / "tampered.tar.gz"
        rewrite_archive(
            archive,
            tampered,
            lambda member, body: (
                member,
                body + b" " if member.name.endswith("/capsule.json") and body is not None else body,
            ),
        )
        assert run_verifier(tampered).returncode == 1

        missing = root / "missing.tar.gz"
        rewrite_archive(
            archive,
            missing,
            lambda member, body: None
            if member.name.endswith("/processes.json")
            else (member, body),
        )
        assert run_verifier(missing).returncode == 1

        wrong_mode = root / "wrong-mode.tar.gz"

        def change_mode(
            member: tarfile.TarInfo, body: bytes | None
        ) -> tuple[tarfile.TarInfo, bytes | None]:
            if member.name.endswith("/tracked-files.json"):
                member.mode = 0o600
            return member, body

        rewrite_archive(archive, wrong_mode, change_mode)
        assert run_verifier(wrong_mode).returncode == 1
        assert run_verifier(archive, "d" * 40).returncode == 1

        prohibited_payloads = payloads()
        prohibited_payloads["processes.json"] = {
            "kind": "sanitized_gate_processes",
            "processes": [],
            "api_key": "sk-prohibited-credential-value",
        }
        prohibited = root / "prohibited.tar.gz"
        builder.write_archive(
            prohibited,
            candidate_head=HEAD,
            payloads=prohibited_payloads,
            source_final_summary_sha256=None,
        )
        credential_failure = run_verifier(prohibited)
        assert credential_failure.returncode == 1
        assert "prohibited" in credential_failure.stderr

        prompt_payloads = payloads()
        prompt_payloads["processes.json"] = {
            "kind": "sanitized_gate_processes",
            "processes": [
                {
                    **execution(["codex", "exec", PROMPT_SENTINEL], ROOT),
                    "argv_completeness": "sanitized_portable_projection",
                    "raw_argv_retained_locally": True,
                    "sanitization_applied": False,
                    "sanitized_argument_count": 0,
                    "sanitizations": [],
                }
            ],
        }
        prompt_archive = root / "prompt-retained.tar.gz"
        builder.write_archive(
            prompt_archive,
            candidate_head=HEAD,
            payloads=prompt_payloads,
            source_final_summary_sha256=None,
        )
        assert run_verifier(prompt_archive).returncode == 1

        absolute_payloads = payloads()
        absolute_payloads["admission.json"]["leaked_path"] = (
            "/home/private-user/repository"
        )
        absolute_archive = root / "absolute-path-retained.tar.gz"
        builder.write_archive(
            absolute_archive,
            candidate_head=HEAD,
            payloads=absolute_payloads,
            source_final_summary_sha256=None,
        )
        assert run_verifier(absolute_archive).returncode == 1

        source_payloads = payloads()
        source_payloads["processes.json"]["source_body"] = "repository body"
        source_archive = root / "source-body-retained.tar.gz"
        builder.write_archive(
            source_archive,
            candidate_head=HEAD,
            payloads=source_payloads,
            source_final_summary_sha256=None,
        )
        assert run_verifier(source_archive).returncode == 1

    print(json.dumps({
        "kind": "validation_evidence_archive_self_test",
        "status": "passed",
        "scenarios": [
            "positive",
            "integration_shaped_gate_and_official_v11",
            "logical_official_v11_working_directory",
            "unknown_external_working_directory",
            "private_prompt_sanitization",
            "absolute_path_rejection",
            "repository_source_body_rejection",
            "tampered_content",
            "missing_file",
            "wrong_posix_mode",
            "candidate_mismatch",
            "credential_like_prohibited_content",
        ],
        "real_final_invocations": 0,
        "official_v11_invocations": 0,
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
