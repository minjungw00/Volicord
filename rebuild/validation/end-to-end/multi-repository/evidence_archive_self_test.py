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
        },
        "gate-result.json": {
            "kind": "validation_gate_result",
            "candidate_head": HEAD,
            "status": "blocked",
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

    print(json.dumps({
        "kind": "validation_evidence_archive_self_test",
        "status": "passed",
        "scenarios": [
            "positive",
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
