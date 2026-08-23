"""Admission and same-session final-to-V11 validation orchestration."""

from __future__ import annotations

import copy
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import socket
import subprocess
import sys
import tempfile
from typing import Any, Callable, Sequence


ROOT = Path(__file__).resolve().parents[4]
REBUILD_ROOT = ROOT / "rebuild"
HERE = Path(__file__).resolve().parent
HARNESS = HERE / "harness.py"
ARCHITECTURE_CHECKER = REBUILD_ROOT / "scripts/check-architecture-contracts"
REALISTIC_QUALIFICATION = REBUILD_ROOT / "validation/repository-intelligence/realistic-qualification/assertions.py"
DOGFOOD_HARNESS = REBUILD_ROOT / "validation/dogfood/harness.py"
DOGFOOD_CAMPAIGN_SELF_TEST = REBUILD_ROOT / "validation/dogfood/campaign_self_test.py"
PROVIDER_QUALIFICATION = REBUILD_ROOT / "validation/privacy/background-provider-qualification/harness.py"
FIXTURE_MANIFEST = REBUILD_ROOT / "validation/shared/fixture-manifest.json"
FIXTURE_CHECKER = REBUILD_ROOT / "scripts/check-fixture-manifest"
RESOURCE_ESTIMATE = HERE / "resource-estimate.json"
REQUIRED_FIXTURE_IDS = (
    "v01-python",
    "v11-polyglot-medium",
    "repository-intelligence-realistic-v1",
    "v12-current-codex-mcp-completion",
    "v07-background-provider-bounded-rust",
)
FINAL_COMMAND_LABELS = ("cargo_metadata", "cargo_fmt", "cargo_clippy", "cargo_test")
AUTHORIZATION_ASSERTION = "v11-openai-codex-project-health-three-targets"
PROVIDER_AUTHORIZATION_ASSERTION = "openai-codex-background-semantic-bounded-rust-v1"
VERSION_OUTPUT_LIMIT_BYTES = 4096
TOOL_VERSION_COMMANDS = {
    "python": ("python3", "--version"),
    "git": ("git", "--version"),
    "cargo": ("cargo", "--version"),
    "rustc": ("rustc", "--version"),
    "codex": ("codex", "--version"),
}
DEPENDENCY_INPUTS = {
    "cargo_lock": REBUILD_ROOT / "Cargo.lock",
    "workspace_manifest": REBUILD_ROOT / "Cargo.toml",
    "fixture_manifest": FIXTURE_MANIFEST,
}
EXTERNAL_TRANSMISSION = {
    "required": True,
    "destination": "OpenAI Codex service used by the installed Codex CLI",
    "purpose": "three authenticated turns that select each repository-scoped Volicord project_health MCP tool",
    "scope": ["volicord", "small-python", "polyglot-medium"],
    "source_scope": "bounded V11 prompt, Project identity, and project_health tool result; no intended repository source body",
    "authorization_assertion": AUTHORIZATION_ASSERTION,
}
PROVIDER_EXTERNAL_TRANSMISSION = {
    "required": True,
    "destination": "OpenAI Codex service used by the installed Codex CLI",
    "purpose": "qualify the production background semantic provider against one bounded maintained Rust source",
    "scope": [
        "rebuild/validation/privacy/background-provider-qualification/fixtures/bounded-rust/src/lib.rs"
    ],
    "source_scope": "the maintained bounded-rust fixture's src/lib.rs source body, at most 4096 bytes",
    "authorization_assertion": PROVIDER_AUTHORIZATION_ASSERTION,
}

Check = dict[str, Any]
CommandRunner = Callable[[Path, Sequence[str]], dict[str, Any]]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def bounded_version_probe(argv: Sequence[str]) -> dict[str, Any]:
    result: dict[str, Any] = {"argv": list(argv), "status": "error", "version": None}
    try:
        completed = subprocess.run(
            list(argv),
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            text=True,
            capture_output=True,
            check=False,
            timeout=10,
        )
    except FileNotFoundError:
        return {**result, "status": "unavailable", "error": "executable_not_found"}
    except subprocess.TimeoutExpired:
        return {**result, "error": "probe_timeout"}
    except OSError as error:
        return {**result, "error": type(error).__name__}

    output = completed.stdout.strip() or completed.stderr.strip()
    if completed.returncode != 0:
        return {**result, "exit_code": completed.returncode, "error": "nonzero_exit"}
    if not output:
        return {**result, "exit_code": completed.returncode, "error": "empty_output"}
    if len(output.encode("utf-8")) > VERSION_OUTPUT_LIMIT_BYTES:
        return {**result, "exit_code": completed.returncode, "error": "output_exceeded_bound"}
    return {
        "argv": list(argv),
        "status": "available",
        "version": output,
        "exit_code": completed.returncode,
    }


def execution_environment() -> dict[str, Any]:
    return {
        "platform": {
            "operating_system": platform.system(),
            "release": platform.release(),
            "platform_version": platform.version(),
            "machine": platform.machine(),
            "architecture": platform.architecture()[0],
        },
        "python_runtime": {
            "implementation": platform.python_implementation(),
            "version": platform.python_version(),
            "executable_basename": Path(sys.executable).name,
        },
        "tools": {
            name: bounded_version_probe(argv)
            for name, argv in TOOL_VERSION_COMMANDS.items()
        },
    }


def hashed_identity(path: Path) -> dict[str, Any]:
    relative = path.relative_to(ROOT).as_posix()
    try:
        return {"path": relative, "status": "available", "sha256": sha256(path)}
    except FileNotFoundError:
        return {"path": relative, "status": "unavailable", "sha256": None}
    except OSError as error:
        return {
            "path": relative,
            "status": "error",
            "sha256": None,
            "error": type(error).__name__,
        }


def dependency_snapshot(candidate_head: str | None) -> dict[str, Any]:
    return {
        "candidate_head": candidate_head,
        **{
            name: hashed_identity(path)
            for name, path in DEPENDENCY_INPUTS.items()
        },
    }


def gate_configuration(
    *,
    authorization_assertion: str | None,
    provider_authorization_assertion: str | None,
    provider_model: str | None,
    external_network: str,
) -> dict[str, Any]:
    argv = [
        "rebuild/scripts/validate",
        "gate",
        "--external-network",
        external_network,
    ]
    argv_status = "complete"
    if authorization_assertion == AUTHORIZATION_ASSERTION:
        argv.extend(("--authorize-external-transmission", AUTHORIZATION_ASSERTION))
    elif authorization_assertion is not None:
        argv_status = "unrecognized_authorization_assertion_not_retained"
    if provider_authorization_assertion == PROVIDER_AUTHORIZATION_ASSERTION:
        argv.extend(("--authorize-provider-source-transmission", PROVIDER_AUTHORIZATION_ASSERTION))
    elif provider_authorization_assertion is not None:
        argv_status = "unrecognized_authorization_assertion_not_retained"
    if provider_model:
        argv.extend(("--provider-model", provider_model))
    return {
        "argv": argv,
        "argv_status": argv_status,
        "technical_external_network_assertion": external_network,
        "authorization_assertion_id": (
            AUTHORIZATION_ASSERTION
            if authorization_assertion == AUTHORIZATION_ASSERTION
            else None
        ),
        "provider_authorization_assertion_id": (
            PROVIDER_AUTHORIZATION_ASSERTION
            if provider_authorization_assertion == PROVIDER_AUTHORIZATION_ASSERTION
            else None
        ),
        "provider_model": provider_model,
        "external_transmission": EXTERNAL_TRANSMISSION,
        "provider_external_transmission": PROVIDER_EXTERNAL_TRANSMISSION,
    }


def directory_bytes(directory: Path) -> int:
    total = 0
    if not directory.exists():
        return total
    for root, directories, files in os.walk(directory):
        directories[:] = [name for name in directories if name not in {".git", ".local"}]
        for name in files:
            try:
                total += (Path(root) / name).stat().st_size
            except OSError:
                continue
    return total


def check(name: str, status: str, summary: str, **details: Any) -> Check:
    return {"name": name, "status": status, "summary": summary, "details": details}


def command_check(name: str, result: dict[str, Any], summary: str) -> Check:
    passed = result.get("wrapper_exit_code") == 0 or result.get("exit_code") == 0
    return check(
        name,
        "passed" if passed else "failed",
        summary if passed else f"{summary} failed",
        outcome=result.get("outcome"),
        exit_code=result.get("wrapper_exit_code", result.get("exit_code")),
    )


def git_output(*arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *arguments], cwd=ROOT, text=True, capture_output=True, check=False
    )


def repository_check() -> tuple[Check, str | None]:
    head_result = git_output("rev-parse", "HEAD")
    status_result = git_output("status", "--porcelain=v1", "--untracked-files=all")
    head = head_result.stdout.strip() if head_result.returncode == 0 else None
    dirty_entries = status_result.stdout.splitlines() if status_result.returncode == 0 else None
    dirty_count = len(dirty_entries) if dirty_entries is not None else None
    passed = head is not None and dirty_count == 0
    return (
        check(
            "candidate_identity_and_clean_worktree",
            "passed" if passed else "environment_blocked",
            "candidate HEAD is recorded and the worktree is clean"
            if passed
            else "candidate identity is unavailable or the worktree is not clean",
            candidate_head=head,
            dirty_entry_count=dirty_count,
            dirty_entries=dirty_entries,
        ),
        head,
    )


def pre_final_repository_check(candidate_head: str) -> Check:
    current, observed_head = repository_check()
    details = current["details"]
    head_matches = None if observed_head is None else observed_head == candidate_head
    worktree_clean = details.get("dirty_entry_count") == 0
    passed = head_matches is True and worktree_clean
    return check(
        "pre_final_candidate_identity_and_clean_worktree",
        "passed" if passed else "environment_blocked",
        "candidate HEAD is unchanged and the worktree is clean immediately before final"
        if passed
        else "candidate HEAD changed or the worktree became dirty before final",
        expected_candidate_head=candidate_head,
        observed_candidate_head=observed_head,
        head_unchanged=head_matches,
        dirty_entry_count=details.get("dirty_entry_count"),
        dirty_entries=details.get("dirty_entries"),
    )


def fixture_identities() -> tuple[list[dict[str, str]], Check]:
    try:
        manifest = json.loads(FIXTURE_MANIFEST.read_text(encoding="utf-8"))
        by_id = {value["id"]: value for value in manifest["fixtures"]}
        identities = [
            {
                "id": identifier,
                "validation_id": by_id[identifier]["validation_id"],
                "content_sha256": by_id[identifier]["content_sha256"],
            }
            for identifier in REQUIRED_FIXTURE_IDS
        ]
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        return [], check(
            "required_fixture_identities",
            "failed",
            "required fixture identities could not be read",
            error_type=type(error).__name__,
        )
    return identities, check(
        "required_fixture_identities",
        "passed",
        "required fixture identities are present in the maintained manifest",
        fixtures=identities,
    )


def executable_check() -> Check:
    tools = {name: shutil.which(name) is not None for name in ("cargo", "git", "python3", "codex")}
    files = {
        "installer": (REBUILD_ROOT / "install.sh").is_file()
        and os.access(REBUILD_ROOT / "install.sh", os.X_OK),
        "v11_harness": HARNESS.is_file() and os.access(HARNESS, os.X_OK),
    }
    passed = all(tools.values()) and all(files.values())
    return check(
        "required_local_executables",
        "passed" if passed else "environment_blocked",
        "all required local executables are available"
        if passed
        else "one or more required local executables are unavailable",
        executables=tools | files,
    )


def filesystem_check(artifact_root: Path) -> Check:
    try:
        artifact_root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="admission-filesystem-", dir=artifact_root) as directory:
            probe = Path(directory) / "runtime-home-probe"
            probe.mkdir()
            (probe / "write-probe").write_text("bounded\n", encoding="utf-8")
        passed = True
        error_type = None
    except OSError as error:
        passed = False
        error_type = type(error).__name__
    return check(
        "filesystem_and_runtime_home",
        "passed" if passed else "environment_blocked",
        "validation artifacts and disposable runtime homes can be created and removed"
        if passed
        else "validation artifacts or disposable runtime homes are unavailable",
        error_type=error_type,
    )


def resource_check(artifact_root: Path) -> Check:
    try:
        estimate = json.loads(RESOURCE_ESTIMATE.read_text(encoding="utf-8"))
        tracked = git_output("ls-files", "-z")
        if tracked.returncode != 0:
            raise OSError("git ls-files failed")
        tracked_bytes = sum(
            (ROOT / path).stat().st_size
            for path in tracked.stdout.split("\0")
            if path and (ROOT / path).is_file()
        )
        fixture_bytes = sum(directory_bytes(path) for path in (
            REBUILD_ROOT / "validation/repository-intelligence/polyglot-structural/fixtures/python",
            HERE / "fixtures/polyglot-medium",
        ))
        target_bytes = directory_bytes(REBUILD_ROOT / "target")
        base = (
            tracked_bytes * int(estimate["candidate_copy_count"])
            + fixture_bytes * int(estimate["fixture_copy_count"])
            + max(target_bytes, int(estimate["minimum_build_and_evidence_bytes"]))
        )
        required = base * (100 + int(estimate["free_space_headroom_percent"])) // 100
        free = shutil.disk_usage(artifact_root).free
        passed = free >= required
        return check(
            "bounded_local_resource_estimate",
            "passed" if passed else "environment_blocked",
            "free space satisfies the maintained V11 estimate"
            if passed
            else "free space is below the maintained V11 estimate",
            available_bytes=free,
            required_bytes=required,
            estimate_basis=estimate["basis"],
            tracked_candidate_bytes=tracked_bytes,
            fixture_bytes=fixture_bytes,
            current_build_tree_bytes=target_bytes,
        )
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        return check(
            "bounded_local_resource_estimate",
            "environment_blocked",
            "the maintained V11 resource estimate could not be evaluated",
            error_type=type(error).__name__,
        )


def loopback_check() -> Check:
    listener: socket.socket | None = None
    client: socket.socket | None = None
    accepted: socket.socket | None = None
    try:
        listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        listener.bind(("127.0.0.1", 0))
        listener.listen(1)
        client = socket.create_connection(listener.getsockname(), timeout=2)
        accepted, _ = listener.accept()
        client.sendall(b"v11-loopback")
        passed = accepted.recv(32) == b"v11-loopback"
        error_type = None
    except OSError as error:
        passed = False
        error_type = type(error).__name__
    finally:
        for value in (accepted, client, listener):
            if value is not None:
                value.close()
    return check(
        "local_loopback",
        "passed" if passed else "environment_blocked",
        "local loopback bind and connection succeeded"
        if passed
        else "local loopback is unavailable; qualified execution requires environment escalation",
        requires_escalation=not passed,
        error_type=error_type,
    )


def authentication_check() -> Check:
    auth = Path(os.environ.get("CODEX_HOME", str(Path.home() / ".codex"))) / "auth.json"
    try:
        available = auth.is_file() and os.access(auth, os.R_OK) and auth.stat().st_size > 0
    except OSError:
        available = False
    return check(
        "codex_authentication_material",
        "passed" if available else "environment_blocked",
        "Codex authentication material is available and readable"
        if available
        else "Codex authentication material is unavailable",
        available=available,
    )


def evaluate_admission(
    *,
    authorization_assertion: str | None,
    provider_authorization_assertion: str | None = None,
    provider_model: str | None = None,
    external_network: str,
    artifact_root: Path,
    command_runner: CommandRunner,
    runner_path: Path,
    overrides: dict[str, Check] | None = None,
    environment_evidence: dict[str, Any] | None = None,
    dependency_evidence: dict[str, Any] | None = None,
) -> dict[str, Any]:
    overrides = overrides or {}
    checks: list[Check] = []

    repository = overrides.get("candidate_identity_and_clean_worktree")
    candidate_head = None
    if repository is None:
        repository, candidate_head = repository_check()
    checks.append(repository)
    if checks[-1]["details"].get("candidate_head"):
        candidate_head = checks[-1]["details"]["candidate_head"]

    runner = overrides.get("validation_runner_self_check")
    if runner is None:
        runner_result = command_runner(artifact_root / "runner-self-check", (str(runner_path), "self-test"))
        runner = command_check("validation_runner_self_check", runner_result, "validation runner self-check completed")
    checks.append(runner)

    v11 = overrides.get("v11_harness_self_check")
    if v11 is None:
        v11_result = command_runner(artifact_root / "v11-self-check", (str(HARNESS), "self-check"))
        v11 = command_check("v11_harness_self_check", v11_result, "V11 harness self-check completed")
    checks.append(v11)

    maintained_self_checks = (
        ("architecture_contracts", (str(ARCHITECTURE_CHECKER),)),
        ("architecture_contracts_self_test", (str(ARCHITECTURE_CHECKER), "--self-test")),
        ("repository_intelligence_realistic_qualification", (sys.executable, str(REALISTIC_QUALIFICATION))),
        ("dogfood_harness_self_test", (sys.executable, str(DOGFOOD_HARNESS), "self-test")),
        ("dogfood_campaign_self_test", (sys.executable, str(DOGFOOD_CAMPAIGN_SELF_TEST))),
        ("provider_qualification_self_test", (sys.executable, str(PROVIDER_QUALIFICATION), "--self-test")),
    )
    for name, argv in maintained_self_checks:
        support = overrides.get(name)
        if support is None:
            support_result = command_runner(artifact_root / name.replace("_", "-"), argv)
            support = command_check(name, support_result, f"{name.replace('_', ' ')} completed")
        checks.append(support)

    identity_check = overrides.get("required_fixture_identities")
    identities: list[dict[str, str]] = []
    if identity_check is None:
        identities, identity_check = fixture_identities()
    checks.append(identity_check)
    if checks[-1]["details"].get("fixtures"):
        identities = checks[-1]["details"]["fixtures"]

    integrity = overrides.get("fixture_manifest_integrity")
    if integrity is None:
        fixture_result = command_runner(
            artifact_root / "fixture-integrity",
            (str(FIXTURE_CHECKER), str(FIXTURE_MANIFEST)),
        )
        integrity = command_check("fixture_manifest_integrity", fixture_result, "fixture manifest integrity check completed")
    checks.append(integrity)

    for name, factory in (
        ("required_local_executables", executable_check),
        ("filesystem_and_runtime_home", lambda: filesystem_check(artifact_root)),
        ("bounded_local_resource_estimate", lambda: resource_check(artifact_root)),
        ("local_loopback", loopback_check),
        ("codex_authentication_material", authentication_check),
    ):
        checks.append(overrides[name] if name in overrides else factory())

    network = check(
        "external_network_capability",
        "passed" if external_network == "available" else "environment_blocked",
        "the current invocation asserts technical external-network availability"
        if external_network == "available"
        else "technical external-network availability is absent or requires escalation",
        assertion=external_network,
        requires_escalation=external_network == "requires-escalation",
    )
    checks.append(overrides.get(network["name"], network))

    transmission = check(
        "authenticated_v11_external_transmission",
        "passed",
        "the maintained authenticated V11 journey requires external transmission",
        transmission=EXTERNAL_TRANSMISSION,
    )
    checks.append(overrides.get(transmission["name"], transmission))
    provider_transmission = check(
        "production_provider_external_transmission",
        "passed",
        "the maintained production-provider qualification requires one bounded source transmission",
        transmission=PROVIDER_EXTERNAL_TRANSMISSION,
    )
    checks.append(overrides.get(provider_transmission["name"], provider_transmission))

    authorized = authorization_assertion == AUTHORIZATION_ASSERTION
    authorization = check(
        "operator_external_transmission_authorization",
        "passed" if authorized else "authorization_blocked",
        "the current invocation carries the exact bounded V11 authorization assertion"
        if authorized
        else "the current invocation lacks the exact bounded V11 authorization assertion",
        authorized=authorized,
        required_assertion=AUTHORIZATION_ASSERTION,
    )
    checks.append(overrides.get(authorization["name"], authorization))

    provider_authorized = provider_authorization_assertion == PROVIDER_AUTHORIZATION_ASSERTION
    provider_authorization = check(
        "operator_provider_source_transmission_authorization",
        "passed" if provider_authorized else "authorization_blocked",
        "the current invocation carries the exact bounded provider-source authorization assertion"
        if provider_authorized
        else "the current invocation lacks the exact bounded provider-source authorization assertion",
        authorized=provider_authorized,
        required_assertion=PROVIDER_AUTHORIZATION_ASSERTION,
        distinct_from_v11=True,
    )
    checks.append(overrides.get(provider_authorization["name"], provider_authorization))
    model_available = isinstance(provider_model, str) and bool(provider_model.strip())
    provider_model_check = check(
        "provider_qualification_model",
        "passed" if model_available else "environment_blocked",
        "the exact provider qualification model is configured"
        if model_available
        else "the provider qualification requires an exact model name",
        model=provider_model,
    )
    checks.append(overrides.get(provider_model_check["name"], provider_model_check))

    statuses = {value["status"] for value in checks}
    eligible = statuses == {"passed"}
    classification = None
    if not eligible:
        classification = (
            "authorization_blocked"
            if "authorization_blocked" in statuses
            else "environment_blocked"
            if "environment_blocked" in statuses
            else "validation_failed"
        )
    return {
        "kind": "validation_admission_result",
        "status": "eligible" if eligible else "blocked",
        "eligible": eligible,
        "blocking_classification": classification,
        "candidate_head": candidate_head,
        "checks": checks,
        "required_fixture_identities": identities,
        "execution_environment": environment_evidence or execution_environment(),
        "dependency_snapshot": dependency_evidence or dependency_snapshot(candidate_head),
        "gate_configuration": gate_configuration(
            authorization_assertion=authorization_assertion,
            provider_authorization_assertion=provider_authorization_assertion,
            provider_model=provider_model,
            external_network=external_network,
        ),
        "external_transmission": EXTERNAL_TRANSMISSION,
        "provider_external_transmission": PROVIDER_EXTERNAL_TRANSMISSION,
        "final_command_count": 0,
        "official_v11_command_count": 0,
        "provider_live_qualification_command_count": 0,
    }


def final_summary_view(summary: dict[str, Any]) -> dict[str, Any]:
    commands = summary.get("commands") if isinstance(summary.get("commands"), list) else []
    if not summary:
        return {
            "status": "not_run",
            "command_count": 0,
            "failure_count": 0,
            "commands": [],
        }
    return {
        "status": summary.get("outcome", "invalid"),
        "command_count": summary.get("command_count", 0),
        "failure_count": summary.get("failure_count"),
        "commands": [
            {
                "name": FINAL_COMMAND_LABELS[index] if index < len(FINAL_COMMAND_LABELS) else f"command_{index + 1}",
                "argv": value.get("argv") if isinstance(value.get("argv"), list) else None,
                "outcome": value.get("outcome"),
                "exit_code": value.get("exit_code"),
                "termination": value.get("termination"),
                "spawn_error": value.get("spawn_error") is not None,
                "duration_ms": value.get("duration_ms"),
            }
            for index, value in enumerate(commands)
        ],
    }


def exact_final_passed(summary: dict[str, Any], final_commands: Sequence[Sequence[str]]) -> bool:
    commands = summary.get("commands")
    return (
        summary.get("outcome") == "succeeded"
        and summary.get("failure_count") == 0
        and summary.get("command_count") == len(final_commands)
        and isinstance(commands, list)
        and len(commands) == len(final_commands)
        and all(value.get("argv") == list(expected) for value, expected in zip(commands, final_commands))
    )


def provider_qualification_passed(
    evidence: dict[str, Any] | None,
    candidate_head: str,
    expected_model: str | None,
) -> bool:
    if not isinstance(evidence, dict):
        return False
    authorization = evidence.get("authorization")
    provider = evidence.get("provider")
    fixture = evidence.get("fixture")
    success = evidence.get("success")
    degradation = evidence.get("degradation")
    retained = evidence.get("retained_evidence")
    return bool(
        evidence.get("schema_version") == 1
        and evidence.get("qualification_id") == "background-provider-openai-codex-v1"
        and evidence.get("production_head") == candidate_head
        and isinstance(authorization, dict)
        and authorization.get("assertion_id") == PROVIDER_AUTHORIZATION_ASSERTION
        and authorization.get("distinct_from_v11") is True
        and isinstance(provider, dict)
        and provider.get("identity") == "openai-codex"
        and isinstance(expected_model, str)
        and bool(expected_model.strip())
        and provider.get("model") == expected_model
        and isinstance(fixture, dict)
        and fixture.get("id") == "background-provider-bounded-rust-v1"
        and fixture.get("source_locator") == "src/lib.rs"
        and fixture.get("source_count") == 1
        and isinstance(fixture.get("transmitted_bytes"), int)
        and 0 < fixture["transmitted_bytes"] <= 4096
        and isinstance(success, dict)
        and success.get("guarded_outcome") == "dispatched_and_completed"
        and success.get("provider_request_outcome") == "completed"
        and success.get("transmission_outcome") == "transmitted"
        and isinstance(success.get("semantic_annotation_count"), int)
        and success["semantic_annotation_count"] > 0
        and success.get("annotation_provenance_complete") is True
        and isinstance(degradation, dict)
        and degradation.get("guarded_confirmation_consumed") is True
        and degradation.get("provider_request_outcome") == "provider_unavailable"
        and degradation.get("transmission_outcome") == "not_transmitted"
        and degradation.get("local_canonical_continuity") is True
        and isinstance(retained, dict)
        and set(retained) == {"source_body", "provider_response_body", "credential"}
        and all(value is False for value in retained.values())
    )


def authenticated_outcomes(result: dict[str, Any] | None) -> list[dict[str, Any]]:
    if not result:
        return []
    outcomes = []
    for repository in result.get("repositories", []):
        target = repository.get("class")
        if target not in EXTERNAL_TRANSMISSION["scope"]:
            continue
        authenticated = (
            repository.get("steps", {})
            .get("codex_mcp_connection", {})
            .get("evidence", {})
            .get("authenticated", {})
        )
        outcomes.append({
            "target": target,
            "status": authenticated.get("status", "unavailable"),
            "classification": authenticated.get("status", "unavailable"),
        })
    return outcomes


def credential_audit_view(value: dict[str, Any] | None) -> dict[str, Any]:
    value = value or {}
    return {
        "status": value.get("status", "not_run"),
        "auth_named_file_count": value.get("auth_named_file_count", 0),
        "credential_content_match_count": value.get("credential_content_match_count", 0),
        "scan_error_count": value.get("scan_error_count", 0),
    }


def revisit_evidence_view(
    result: dict[str, Any] | None,
) -> tuple[str, list[str] | None, dict[str, Any] | None, bool]:
    if not result:
        return "not_run", None, None, False
    assessment = result.get("decision_revisit_trigger_assessment")
    triggers = result.get("active_decision_revisit_triggers")
    raw_source = result.get("decision_revisit_trigger_source")
    if not isinstance(raw_source, dict):
        return assessment if isinstance(assessment, str) else "invalid", None, None, False
    source = {
        "kind": raw_source.get("kind"),
        "path": raw_source.get("path"),
        "content_sha256": raw_source.get("content_sha256"),
        "assessed_decision_ids": raw_source.get("assessed_decision_ids"),
        "assessed_decision_count": raw_source.get("assessed_decision_count"),
    }
    assessed_ids = source["assessed_decision_ids"]
    source_valid = (
        source["kind"] == "accepted_decision_register"
        and source["path"] == "rebuild/docs/design/open-decisions.md"
        and isinstance(source["content_sha256"], str)
        and re.fullmatch(r"[0-9a-f]{64}", source["content_sha256"]) is not None
        and isinstance(assessed_ids, list)
        and bool(assessed_ids)
        and all(isinstance(value, str) and re.fullmatch(r"Q[0-9]+(?:-[A-Z])?", value) for value in assessed_ids)
        and len(assessed_ids) == len(set(assessed_ids))
        and source["assessed_decision_count"] == len(assessed_ids)
    )
    triggers_valid = (
        isinstance(triggers, list)
        and all(isinstance(value, str) and value in assessed_ids for value in triggers)
        and len(triggers) == len(set(triggers))
    ) if isinstance(assessed_ids, list) else False
    completed = (
        assessment == "reported_by_official_v11" and source_valid and triggers_valid
    )
    if completed:
        return assessment, list(triggers), source, True
    if assessment == "official_v11_assessment_failed" and triggers is None:
        return assessment, None, source, False
    return assessment if isinstance(assessment, str) else "invalid", None, source, False


def make_capsule(
    *,
    admission: dict[str, Any],
    candidate_head: str | None,
    blocking_classification: str | None,
    final_summary: dict[str, Any] | None = None,
    final_summary_hash: str | None = None,
    provider_qualification: dict[str, Any] | None = None,
    provider_qualification_hash: str | None = None,
    provider_qualification_status: str = "not_run",
    v11_result: dict[str, Any] | None = None,
    v11_result_hash: str | None = None,
    credential_audit: dict[str, Any] | None = None,
    pre_final_check: Check | None = None,
    final_artifact_produced: bool = False,
    preflight_consumed_final_artifact: bool = False,
    official_v11_consumed_final_artifact: bool = False,
) -> dict[str, Any]:
    final_view = final_summary_view(final_summary or {})
    counts = v11_result.get("counts", {}) if v11_result else {}
    revisit_assessment, revisit_triggers, revisit_source, revisit_completed = (
        revisit_evidence_view(v11_result)
    )
    return {
        "kind": "validation_handoff_capsule",
        "validated_candidate_head": candidate_head,
        "admission_status": admission.get("status"),
        "blocking_classification": blocking_classification,
        "admission_checks": [
            {
                "name": value.get("name"),
                "status": value.get("status"),
            }
            for value in admission.get("checks", [])
            if isinstance(value, dict)
        ],
        "pre_final_candidate_check": pre_final_check,
        "execution_environment": admission.get("execution_environment", {}),
        "dependency_snapshot": admission.get("dependency_snapshot", {}),
        "gate_configuration": {
            **admission.get("gate_configuration", {}),
            "same_session_artifact_flow": {
                "final_artifact_produced_by_gate": final_artifact_produced,
                "v11_preflight_consumed_same_gate_final_artifact": preflight_consumed_final_artifact,
                "official_v11_consumed_same_gate_final_artifact": official_v11_consumed_final_artifact,
            },
        },
        "final_aggregate": final_view,
        "final_summary_sha256": final_summary_hash,
        "live_provider_qualification": {
            "status": provider_qualification_status,
            "evidence_sha256": provider_qualification_hash,
            "evidence": provider_qualification,
        },
        "official_v11": {
            "status": v11_result.get("status", "not_run") if v11_result else "not_run",
            "result_sha256": v11_result_hash,
            "required_step_count": sum(counts.values()) if counts else 0,
            "status_counts": counts,
            "phase_8_ready": bool(v11_result and v11_result.get("phase_8_ready")),
        },
        "required_identities": {
            "candidate_head": candidate_head,
            "fixtures": admission.get("required_fixture_identities", []),
        },
        "credential_retention_audit": credential_audit_view(credential_audit),
        "authenticated_codex_outcomes": authenticated_outcomes(v11_result),
        "active_decision_revisit_triggers": revisit_triggers,
        "decision_revisit_trigger_assessment": revisit_assessment,
        "decision_revisit_trigger_source": revisit_source,
        "evidence_archive": {
            "status": "not_run",
            "prerequisites_passed": False,
            "candidate_head": candidate_head,
            "filename": None,
            "sha256": None,
            "size_bytes": None,
            "member_count": None,
            "verification_status": "not_run",
        },
        "phase_8_ready": bool(
            v11_result
            and v11_result.get("phase_8_ready")
            and revisit_completed
            and revisit_triggers == []
            and credential_audit
            and credential_audit.get("status") == "passed"
            and provider_qualification is not None
            and blocking_classification is None
        ),
    }


def stage_evidence_archive(capsule: dict[str, Any]) -> dict[str, Any]:
    staged = copy.deepcopy(capsule)
    prerequisites_passed = staged.get("phase_8_ready") is True
    staged["evidence_archive"] = {
        "status": "pending",
        "prerequisites_passed": prerequisites_passed,
        "candidate_head": staged.get("validated_candidate_head"),
        "filename": None,
        "sha256": None,
        "size_bytes": None,
        "member_count": None,
        "verification_status": "not_run",
    }
    if prerequisites_passed:
        staged["blocking_classification"] = "evidence_archive_pending"
    staged["phase_8_ready"] = False
    return staged


def complete_evidence_archive(
    capsule: dict[str, Any],
    archive_identity: dict[str, Any],
    verification: dict[str, Any],
) -> dict[str, Any]:
    completed = copy.deepcopy(capsule)
    evidence = completed.get("evidence_archive", {})
    candidate_head = completed.get("validated_candidate_head")
    if (
        evidence.get("status") != "pending"
        or archive_identity.get("candidate_head") != candidate_head
        or verification.get("candidate_head") != candidate_head
        or verification.get("status") != "passed"
        or archive_identity.get("sha256") != verification.get("archive_sha256")
    ):
        raise ValueError("evidence archive completion identities are inconsistent")
    prerequisites_passed = evidence.get("prerequisites_passed") is True
    completed["evidence_archive"] = {
        "status": "verified",
        "prerequisites_passed": prerequisites_passed,
        "candidate_head": candidate_head,
        "filename": Path(str(archive_identity["path"])).name,
        "sha256": archive_identity.get("sha256"),
        "size_bytes": archive_identity.get("size_bytes"),
        "member_count": archive_identity.get("member_count"),
        "verification_status": "passed",
    }
    if prerequisites_passed:
        completed["blocking_classification"] = None
        completed["phase_8_ready"] = True
    return completed


def fail_evidence_archive(capsule: dict[str, Any], stage: str) -> dict[str, Any]:
    if stage not in {"creation", "verification"}:
        raise ValueError("unknown evidence archive failure stage")
    failed = copy.deepcopy(capsule)
    evidence = failed.get("evidence_archive", {})
    prerequisites_passed = evidence.get("prerequisites_passed") is True
    failed["evidence_archive"] = {
        **evidence,
        "status": f"{stage}_failed",
        "verification_status": "failed" if stage == "verification" else "not_run",
    }
    if prerequisites_passed:
        failed["blocking_classification"] = f"evidence_archive_{stage}_failed"
    failed["phase_8_ready"] = False
    return failed


def run_json_command(
    command_runner: CommandRunner,
    directory: Path,
    argv: Sequence[str],
) -> tuple[dict[str, Any] | None, dict[str, Any]]:
    execution = command_runner(directory, argv)
    value = None
    stdout_path = execution.get("stdout")
    if stdout_path:
        try:
            value = json.loads(Path(str(stdout_path)).read_text(encoding="utf-8"))
        except (OSError, TypeError, json.JSONDecodeError):
            value = None
    return value, execution


def orchestrate(
    *,
    gate_directory: Path,
    admission: dict[str, Any],
    final_commands: Sequence[Sequence[str]],
    final_owner: Callable[[], tuple[dict[str, Any], Path]],
    provider_owner: Callable[[str], tuple[dict[str, Any] | None, dict[str, Any], Path]],
    preflight_owner: Callable[[str, Path], tuple[dict[str, Any] | None, dict[str, Any]]],
    v11_owner: Callable[[str, Path, Path], tuple[dict[str, Any] | None, dict[str, Any]]],
    audit_owner: Callable[[Path], tuple[dict[str, Any] | None, dict[str, Any]]],
    pre_final_check_owner: Callable[[str], Check] = pre_final_repository_check,
) -> tuple[dict[str, Any], dict[str, int]]:
    counts = {"final": 0, "provider_live_qualification": 0, "preflight": 0, "official_v11": 0, "credential_audit": 0}
    candidate_head = admission.get("candidate_head")
    if not admission.get("eligible"):
        capsule = make_capsule(
            admission=admission,
            candidate_head=candidate_head,
            blocking_classification=admission.get("blocking_classification"),
        )
        return capsule, counts

    if not isinstance(candidate_head, str) or not candidate_head:
        return make_capsule(
            admission=admission,
            candidate_head=None,
            blocking_classification="candidate_state_unavailable",
        ), counts

    pre_final = pre_final_check_owner(candidate_head)
    if pre_final.get("status") != "passed":
        details = pre_final.get("details", {})
        blocking = (
            "candidate_changed"
            if details.get("head_unchanged") is False
            else "candidate_worktree_dirty"
            if details.get("dirty_entry_count")
            else "candidate_state_unavailable"
        )
        return make_capsule(
            admission=admission,
            candidate_head=candidate_head,
            blocking_classification=blocking,
            pre_final_check=pre_final,
        ), counts

    counts["final"] += 1
    final_summary, final_path = final_owner()
    final_hash = sha256(final_path)
    if not exact_final_passed(final_summary, final_commands):
        return make_capsule(
            admission=admission,
            candidate_head=candidate_head,
            blocking_classification="final_failed",
            final_summary=final_summary,
            final_summary_hash=final_hash,
            pre_final_check=pre_final,
            final_artifact_produced=True,
        ), counts

    counts["provider_live_qualification"] += 1
    provider_qualification, provider_execution, provider_path = provider_owner(candidate_head)
    provider_hash = sha256(provider_path) if provider_path.is_file() else None
    expected_provider_model = admission.get("gate_configuration", {}).get("provider_model")
    if (
        provider_execution.get("exit_code", provider_execution.get("wrapper_exit_code")) != 0
        or not provider_qualification_passed(
            provider_qualification, candidate_head, expected_provider_model
        )
    ):
        return make_capsule(
            admission=admission,
            candidate_head=candidate_head,
            blocking_classification="provider_live_qualification_failed",
            final_summary=final_summary,
            final_summary_hash=final_hash,
            provider_qualification_hash=provider_hash,
            provider_qualification_status="failed",
            pre_final_check=pre_final,
            final_artifact_produced=True,
        ), counts

    counts["preflight"] += 1
    preflight, preflight_execution = preflight_owner(candidate_head, final_path)
    if preflight_execution.get("exit_code", preflight_execution.get("wrapper_exit_code")) != 0 or not preflight or preflight.get("status") != "passed":
        return make_capsule(
            admission=admission,
            candidate_head=candidate_head,
            blocking_classification="v11_preflight_failed",
            final_summary=final_summary,
            final_summary_hash=final_hash,
            provider_qualification=provider_qualification,
            provider_qualification_hash=provider_hash,
            provider_qualification_status="passed",
            pre_final_check=pre_final,
            final_artifact_produced=True,
            preflight_consumed_final_artifact=True,
        ), counts

    output_directory = gate_directory / "official-v11"
    counts["official_v11"] += 1
    v11_result, v11_execution = v11_owner(candidate_head, final_path, output_directory)
    counts["credential_audit"] += 1
    audit, audit_execution = audit_owner(output_directory)
    audit = audit or {
        "status": "failed",
        "auth_named_file_count": 0,
        "credential_content_match_count": 0,
        "scan_error_count": 1,
    }
    v11_path = output_directory / "result.json"
    v11_hash = sha256(v11_path) if v11_path.is_file() else None
    v11_passed = (
        v11_execution.get("exit_code", v11_execution.get("wrapper_exit_code")) == 0
        and v11_result is not None
        and v11_result.get("status") == "passed"
        and v11_result.get("phase_8_ready") is True
        and revisit_evidence_view(v11_result)[3]
        and revisit_evidence_view(v11_result)[1] == []
    )
    audit_passed = (
        audit_execution.get("exit_code", audit_execution.get("wrapper_exit_code")) == 0
        and audit.get("status") == "passed"
    )
    blocking = None if v11_passed and audit_passed else (
        "credential_retention_audit_failed" if not audit_passed else "v11_failed"
    )
    capsule = make_capsule(
        admission=admission,
        candidate_head=candidate_head,
        blocking_classification=blocking,
        final_summary=final_summary,
        final_summary_hash=final_hash,
        provider_qualification=provider_qualification,
        provider_qualification_hash=provider_hash,
        provider_qualification_status="passed",
        v11_result=v11_result,
        v11_result_hash=v11_hash,
        credential_audit=audit,
        pre_final_check=pre_final,
        final_artifact_produced=True,
        preflight_consumed_final_artifact=True,
        official_v11_consumed_final_artifact=True,
    )
    return capsule, counts
