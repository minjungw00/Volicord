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
SENSITIVE_OPERANDS = (
    "SECRETWORD",
    "internal_name",
    "foo-bar",
    "abc123",
    "x",
    "-looks-like-a-flag",
    "multi word private content",
    "550e8400-e29b-41d4-a716-446655440000",
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "/home/private-user/source/repository.rs",
)
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
            "argv_policy": builder.argv_policy(),
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


def codex_argv(repository: Path, prompt: str) -> list[str]:
    return [
        "/opt/codex/bin/codex",
        "--dangerously-bypass-hook-trust",
        "--ask-for-approval",
        "never",
        "--config",
        'mcp_servers.volicord.tools.project_health.approval_mode="approve"',
        "exec",
        "--ephemeral",
        "--json",
        "--sandbox",
        "read-only",
        "--skip-git-repo-check",
        "-C",
        str(repository),
        prompt,
    ]


def assert_semantic_policy(root: Path, gate: Path) -> None:
    for payload in SENSITIVE_OPERANDS:
        command = [
            "volicord",
            "--json",
            "--project",
            "project-identity",
            "advanced",
            "records",
            "source",
            "--host",
            "codex",
            "--session",
            "v11",
            "--text",
            payload,
        ]
        projected = builder.sanitized_argv(command, ROOT, gate)
        assert projected["argv"][:3] == ["volicord", "--json", "--project"]
        assert projected["argv"][-1] == "<redacted:sensitive-operand>"
        assert projected["non_structural_argument_roles"][-1] == [
            12,
            "redacted",
            "sensitive_operand",
        ]

        privacy = builder.sanitized_argv(
            [
                "volicord",
                "--json",
                "--project",
                "project-identity",
                "privacy",
                "enable",
                "provider",
                "model",
                "--source",
                "decision-identity",
                "--scope",
                payload,
            ],
            ROOT,
            gate,
        )
        assert privacy["argv"][-1] == "<redacted:sensitive-operand>"

    current_codex = builder.sanitized_argv(codex_argv(root, PROMPT_SENTINEL), ROOT, gate)
    assert current_codex["argv"][1:5] == [
        "--dangerously-bypass-hook-trust",
        "--ask-for-approval",
        "never",
        "--config",
    ]
    assert current_codex["argv"][5] == "<redacted:config-payload>"
    assert current_codex["argv"][-1] == "<redacted:private-prompt>"

    for command, sensitive_index in (
        (["python3", "-c", "SECRETWORD"], 2),
        (["sh", "-c", "internal_name"], 2),
        (["git", "-c", "user.name=foo-bar", "commit", "--quiet", "-m", "abc123"], 2),
        (["git", "-c", "user.name=foo-bar", "commit", "--quiet", "-m", "abc123"], 6),
    ):
        projected = builder.sanitized_argv(command, ROOT, gate)
        assert projected["argv"][sensitive_index].startswith("<redacted:")

    unknown = builder.sanitized_argv(
        ["unrecognized-command", "--flag", "SECRETWORD"], ROOT, gate
    )
    assert all(argument.startswith("<redacted:") for argument in unknown["argv"])

    exact_final = builder.sanitized_argv(
        [
            "cargo",
            "test",
            "--manifest-path",
            "rebuild/Cargo.toml",
            "--workspace",
            "--all-targets",
            "--all-features",
        ],
        ROOT,
        gate,
    )
    assert exact_final["argv"] == [
        "cargo",
        "test",
        "--manifest-path",
        "./rebuild/Cargo.toml",
        "--workspace",
        "--all-targets",
        "--all-features",
    ]
    assert exact_final["non_structural_argument_roles"] == [
        [3, "projected", "repository_path"]
    ]

    warning_clean_clippy = builder.sanitized_argv(
        [
            "cargo", "clippy", "--manifest-path", "rebuild/Cargo.toml",
            "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings",
        ],
        ROOT,
        gate,
    )
    assert warning_clean_clippy["argv"][-3:] == ["--", "-D", "warnings"]

    provider_live = builder.sanitized_argv(
        [
            str(ROOT / "rebuild/validation/privacy/background-provider-qualification/harness.py"),
            "--live",
            "--authorize-source-transmission",
            "openai-codex-background-semantic-bounded-rust-v1",
            "--model",
            "private-exact-model",
            "--evidence-output",
            str(gate / "provider-live-qualification/evidence.json"),
        ],
        ROOT,
        gate,
    )
    assert provider_live["argv"][3] == "openai-codex-background-semantic-bounded-rust-v1"
    assert provider_live["argv"][5] == "<redacted:provider-model>"
    assert provider_live["argv"][7] == "<gate-artifact>/provider-live-qualification/evidence.json"

    current_commands = (
        (["volicord", "--json", "--repository", str(root), "codex", "enable"], ("codex", "enable")),
        (["volicord", "--json", "viewer", "export", "--output", str(root / "understanding.html"), "--level", "project", "--language", "en"], ("viewer", "export")),
        (["volicord", "--json", "document", "export", "handoff-resume", "--format", "html", "--output", str(root / "handoff.html"), "--language", "en"], ("document", "export")),
    )
    for command, subcommands in current_commands:
        projected = builder.sanitized_argv(command, ROOT, gate)
        assert projected["argv"][0:2] == ["volicord", "--json"]
        assert all(value in projected["argv"] for value in subcommands)
        assert not any(role[2] == "unknown_operand" for role in projected["non_structural_argument_roles"])

    family_shapes = (
        ([str(ROOT / "rebuild/scripts/validate"), "self-test"], ["validate", "self-test"]),
        (
            [
                str(ROOT / "rebuild/validation/end-to-end/multi-repository/harness.py"),
                "run",
                "--validated-head",
                "a" * 40,
                "--final-artifact",
                str(gate / "final" / "summary.json"),
                "--output-dir",
                str(gate / "official-v11"),
            ],
            ["harness.py", "run", "--validated-head"],
        ),
        (
            [
                str(ROOT / "rebuild/scripts/check-fixture-manifest"),
                "rebuild/validation/shared/fixture-manifest.json",
            ],
            ["check-fixture-manifest", "./rebuild/validation/shared/fixture-manifest.json"],
        ),
        (
            [
                str(ROOT / "rebuild/install.sh"),
                "--prefix",
                str(gate / "official-v11/work/small-python/prefix"),
                "--runtime-dir",
                str(gate / "official-v11/work/small-python/runtime"),
            ],
            ["install.sh", "--prefix"],
        ),
        (
            [
                str(VERIFIER),
                str(gate / "validation-evidence.tar.gz"),
                "--expected-candidate",
                "b" * 40,
            ],
            ["verify-validation-archive", "<gate-artifact>/validation-evidence.tar.gz"],
        ),
        (
            ["git", "clone", "--quiet", "--no-hardlinks", str(ROOT), str(root)],
            ["git", "clone", "--quiet", "--no-hardlinks", "."],
        ),
        (
            [
                "cargo",
                "test",
                "--manifest-path",
                "rebuild/Cargo.toml",
                "-p",
                "volicord-operations",
                "--test",
                "v11_fixture_control",
                "--all-features",
                "--",
                "--exact",
                "seed_and_inspect_v11_forgetting_control",
                "--nocapture",
            ],
            ["cargo", "test", "--manifest-path", "./rebuild/Cargo.toml"],
        ),
    )
    for command, expected_prefix in family_shapes:
        projected = builder.sanitized_argv(command, ROOT, gate)
        assert projected["argv"][: len(expected_prefix)] == expected_prefix


def integration_archive(root: Path) -> tuple[Path, str]:
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, capture_output=True, check=True
    ).stdout.strip()
    gate = root / "synthetic-gate"
    repository_result = gate / "admission-checks" / "runner" / "result.json"
    target = gate / "official-v11" / "work" / "small-python" / "repository"
    operations = gate / "official-v11" / "operations"
    target.mkdir(parents=True)
    repository_result.parent.mkdir(parents=True)
    operations.mkdir(parents=True)
    assert_semantic_policy(root, gate)
    repository_result.write_text(
        json.dumps(
            execution(
                [
                    "cargo",
                    "test",
                    "--manifest-path",
                    "rebuild/Cargo.toml",
                    "--workspace",
                    "--all-targets",
                    "--all-features",
                ],
                ROOT,
            )
        ),
        encoding="utf-8",
    )
    codex_result = operations / "001-authenticated-codex" / "result.json"
    codex_result.parent.mkdir()
    codex_result.write_text(
        json.dumps(
            execution(codex_argv(target, PROMPT_SENTINEL), target)
        ),
        encoding="utf-8",
    )
    for index, payload in enumerate(SENSITIVE_OPERANDS, start=2):
        (operations / f"{index:03d}-user-source" / "result.json").parent.mkdir()
        (operations / f"{index:03d}-user-source" / "result.json").write_text(
            json.dumps(
                execution(
                    [
                        "volicord",
                        "--json",
                        "--project",
                        "project-identity",
                        "advanced",
                        "records",
                        "source",
                        "--host",
                        "codex",
                        "--session",
                        "v11",
                        "--text",
                        payload,
                    ],
                    target,
                )
            ),
            encoding="utf-8",
        )
    config_result = operations / "020-git-config" / "result.json"
    config_result.parent.mkdir()
    config_result.write_text(
        json.dumps(
            execution(
                [
                    "git",
                    "-c",
                    "user.name=SECRETWORD",
                    "-c",
                    "user.email=internal_name",
                    "commit",
                    "--quiet",
                    "-m",
                    "foo-bar",
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
        "commands": [
            execution(
                [
                    "cargo",
                    "test",
                    "--manifest-path",
                    "rebuild/Cargo.toml",
                    "--workspace",
                    "--all-targets",
                    "--all-features",
                ],
                ROOT,
            )
        ],
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
    for payload in SENSITIVE_OPERANDS:
        if len(payload) > 1:
            assert payload.encode() not in bodies
    assert b'"x"' not in bodies
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
    assert processes["argv_policy"] == builder.argv_policy()
    assert target_process["non_structural_argument_roles"][-1] == [
        14,
        "redacted",
        "private_prompt",
    ]
    assert PROMPT_SENTINEL in codex_result.read_text(encoding="utf-8")
    for index, payload in enumerate(SENSITIVE_OPERANDS, start=2):
        assert payload in (
            operations / f"{index:03d}-user-source" / "result.json"
        ).read_text(encoding="utf-8")

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


def production_scale_archive(root: Path) -> dict[str, int]:
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, capture_output=True, check=True
    ).stdout.strip()
    gate = root / "production-scale-gate"
    target = gate / "official-v11" / "work" / "polyglot-medium" / "repository"
    target.mkdir(parents=True)
    commands: list[list[str]] = []
    for index in range(174):
        commands.append(
            [
                "volicord",
                "--json",
                "--project",
                "project-identity",
                "advanced",
                "records",
                "source",
                "--host",
                "codex",
                "--session",
                "v11",
                "--text",
                SENSITIVE_OPERANDS[index % len(SENSITIVE_OPERANDS)],
            ]
        )
    commands.extend(
        [
            codex_argv(target, PROMPT_SENTINEL),
            ["python3", "-c", "repository body"],
            [
                "git",
                "-c",
                "user.name=SECRETWORD",
                "-c",
                "user.email=internal_name",
                "commit",
                "--quiet",
                "-m",
                "foo-bar",
            ],
            [
                "unrecognized-command",
                "--flag",
                "SECRETWORD",
                "internal_name",
                "foo-bar",
                "abc123",
                "x",
            ],
        ]
    )
    assert len(commands) == 178
    for index, command in enumerate(commands):
        result = gate / "official-v11" / "operations" / f"{index:03d}" / "result.json"
        result.parent.mkdir(parents=True)
        result.write_text(json.dumps(execution(command, target)), encoding="utf-8")

    admission = {
        "candidate_head": head,
        "status": "eligible",
        "eligible": True,
        "checks": [],
    }
    capsule = {
        "kind": "validation_handoff_capsule",
        "validated_candidate_head": head,
        "final_summary_sha256": None,
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
        final_summary=None,
    )
    archive = Path(identity["path"])
    verified = run_verifier(archive, head)
    assert verified.returncode == 0, verified.stderr
    with tarfile.open(archive, "r:gz") as retained:
        file_members = [member for member in retained.getmembers() if member.isfile()]
        assert all(member.size <= builder.MEMBER_MAX_BYTES for member in file_members)
        bodies = b"".join(retained.extractfile(member).read() for member in file_members)
        processes = json.loads(
            retained.extractfile("validation-evidence/processes.json").read()
        )
    process_count = len(processes["processes"])
    argv_entries = sum(len(process["argv"]) for process in processes["processes"])
    role_records = sum(
        len(process["non_structural_argument_roles"])
        for process in processes["processes"]
    )
    assert process_count >= 178
    assert argv_entries >= 1_116
    assert role_records >= 659
    assert PROMPT_SENTINEL.encode() not in bodies
    for payload in SENSITIVE_OPERANDS:
        if len(payload) > 1:
            assert payload.encode() not in bodies
    assert b'"x"' not in bodies
    assert b"repository body" not in bodies
    assert str(ROOT).encode() not in bodies
    assert str(root).encode() not in bodies
    return {
        "process_records": process_count,
        "argv_entries": argv_entries,
        "non_structural_argument_role_records": role_records,
        "processes_member_bytes": len(builder.json_bytes(processes)),
    }


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
        scale = production_scale_archive(root)
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

        oversized_payloads = payloads()
        oversized_payloads["processes.json"]["oversized_test_value"] = (
            "x" * builder.MEMBER_MAX_BYTES
        )
        oversized_archive = root / "oversized.tar.gz"
        try:
            builder.write_archive(
                oversized_archive,
                candidate_head=HEAD,
                payloads=oversized_payloads,
                source_final_summary_sha256=None,
            )
        except ValueError as error:
            assert "archive member exceeds" in str(error)
        else:
            raise AssertionError("builder accepted an over-bound payload member")
        assert not oversized_archive.exists()

        non_current_payloads = payloads()
        non_current_execution = builder.sanitized_execution(
            execution(["cargo", "metadata"], ROOT), ROOT, root
        )
        non_current_execution.pop("non_structural_argument_roles")
        non_current_execution.update(
            {
                "argv_completeness": "sanitized_portable_projection",
                "argv_projection_policy": "explicit_semantic_argument_roles",
                "raw_argv_retained_locally": True,
                "sanitization_applied": False,
                "sanitized_argument_count": 0,
                "sanitizations": [],
                "argument_classifications": ["structural", "structural"],
            }
        )
        non_current_payloads["processes.json"]["processes"] = [non_current_execution]
        non_current_archive = root / "non-current-process-representation.tar.gz"
        builder.write_archive(
            non_current_archive,
            candidate_head=HEAD,
            payloads=non_current_payloads,
            source_final_summary_sha256=None,
        )
        assert run_verifier(non_current_archive).returncode == 1

        prohibited_payloads = payloads()
        prohibited_payloads["processes.json"] = {
            "kind": "sanitized_gate_processes",
            "argv_policy": builder.argv_policy(),
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
        leaked_prompt = builder.sanitized_execution(
            execution(codex_argv(ROOT, PROMPT_SENTINEL), ROOT), ROOT, root
        )
        leaked_prompt["argv"][-1] = PROMPT_SENTINEL
        leaked_prompt["non_structural_argument_roles"] = [
            role
            for role in leaked_prompt["non_structural_argument_roles"]
            if role[0] != len(leaked_prompt["argv"]) - 1
        ]
        prompt_payloads["processes.json"] = {
            "kind": "sanitized_gate_processes",
            "argv_policy": builder.argv_policy(),
            "processes": [leaked_prompt],
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
        "production_scale": scale,
        "scenarios": [
            "positive",
            "integration_shaped_gate_and_official_v11",
            "logical_official_v11_working_directory",
            "unknown_external_working_directory",
            "private_prompt_sanitization",
            "semantic_role_payload_regressions",
            "flag_shaped_content_operand",
            "shell_python_git_config_payloads",
            "exact_final_command_shape",
            "official_v11_harness_shape",
            "volicord_product_path_shapes",
            "verifier_execution_shape",
            "owned_path_projection",
            "unknown_command_conservative_redaction",
            "raw_local_argv_preservation",
            "absolute_path_rejection",
            "repository_source_body_rejection",
            "tampered_content",
            "missing_file",
            "wrong_posix_mode",
            "candidate_mismatch",
            "credential_like_prohibited_content",
            "production_scale_builder_and_verifier",
            "builder_member_bound_rejection",
            "non_current_process_representation_rejection",
        ],
        "real_final_invocations": 0,
        "official_v11_invocations": 0,
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
