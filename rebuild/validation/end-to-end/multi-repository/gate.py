"""Admission and same-session final-to-V11 validation orchestration."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import socket
import subprocess
import tempfile
from typing import Any, Callable, Sequence


ROOT = Path(__file__).resolve().parents[4]
REBUILD_ROOT = ROOT / "rebuild"
HERE = Path(__file__).resolve().parent
HARNESS = HERE / "harness.py"
FIXTURE_MANIFEST = REBUILD_ROOT / "validation/shared/fixture-manifest.json"
FIXTURE_CHECKER = REBUILD_ROOT / "scripts/check-fixture-manifest"
RESOURCE_ESTIMATE = HERE / "resource-estimate.json"
REQUIRED_FIXTURE_IDS = ("v01-python", "v11-polyglot-medium")
FINAL_COMMAND_LABELS = ("cargo_metadata", "cargo_fmt", "cargo_clippy", "cargo_test")
AUTHORIZATION_ASSERTION = "v11-openai-codex-project-health-three-targets"
EXTERNAL_TRANSMISSION = {
    "required": True,
    "destination": "OpenAI Codex service used by the installed Codex CLI",
    "purpose": "three authenticated turns that select the installed Volicord project_health MCP tool",
    "scope": ["volicord", "small-python", "polyglot-medium"],
    "source_scope": "bounded V11 prompt, Project identity, and project_health tool result; no intended repository source body",
    "authorization_assertion": AUTHORIZATION_ASSERTION,
}

Check = dict[str, Any]
CommandRunner = Callable[[Path, Sequence[str]], dict[str, Any]]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


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
    external_network: str,
    artifact_root: Path,
    command_runner: CommandRunner,
    runner_path: Path,
    overrides: dict[str, Check] | None = None,
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
        "external_transmission": EXTERNAL_TRANSMISSION,
        "final_command_count": 0,
        "official_v11_command_count": 0,
    }


def final_summary_view(summary: dict[str, Any]) -> dict[str, Any]:
    commands = summary.get("commands") if isinstance(summary.get("commands"), list) else []
    return {
        "status": summary.get("outcome", "invalid"),
        "command_count": summary.get("command_count", 0),
        "failure_count": summary.get("failure_count"),
        "commands": [
            {
                "name": FINAL_COMMAND_LABELS[index] if index < len(FINAL_COMMAND_LABELS) else f"command_{index + 1}",
                "outcome": value.get("outcome"),
                "exit_code": value.get("exit_code"),
                "termination": value.get("termination"),
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


def bounded_revisit_triggers(result: dict[str, Any] | None) -> list[str]:
    if not result:
        return []
    values = result.get("active_decision_revisit_triggers", [])
    if not isinstance(values, list):
        return []
    return [
        value
        for value in values
        if isinstance(value, str)
        and len(value) <= 120
        and value.startswith("Q")
        and value.partition(":")[0][1:].replace("-", "").isalnum()
    ][:13]


def make_capsule(
    *,
    admission: dict[str, Any],
    candidate_head: str | None,
    blocking_classification: str | None,
    final_summary: dict[str, Any] | None = None,
    final_summary_hash: str | None = None,
    v11_result: dict[str, Any] | None = None,
    v11_result_hash: str | None = None,
    credential_audit: dict[str, Any] | None = None,
    pre_final_check: Check | None = None,
) -> dict[str, Any]:
    final_view = final_summary_view(final_summary or {})
    counts = v11_result.get("counts", {}) if v11_result else {}
    return {
        "kind": "validation_handoff_capsule",
        "validated_candidate_head": candidate_head,
        "admission_status": admission.get("status"),
        "blocking_classification": blocking_classification,
        "pre_final_candidate_check": pre_final_check,
        "final_aggregate": final_view,
        "final_summary_sha256": final_summary_hash,
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
        "active_decision_revisit_triggers": bounded_revisit_triggers(v11_result),
        "decision_revisit_trigger_assessment": (
            "reported_by_official_v11" if v11_result and "active_decision_revisit_triggers" in v11_result
            else "independent_documentation_review_required"
        ),
        "phase_8_ready": bool(
            v11_result
            and v11_result.get("phase_8_ready")
            and credential_audit
            and credential_audit.get("status") == "passed"
            and blocking_classification is None
        ),
    }


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
    preflight_owner: Callable[[str, Path], tuple[dict[str, Any] | None, dict[str, Any]]],
    v11_owner: Callable[[str, Path, Path], tuple[dict[str, Any] | None, dict[str, Any]]],
    audit_owner: Callable[[Path], tuple[dict[str, Any] | None, dict[str, Any]]],
    pre_final_check_owner: Callable[[str], Check] = pre_final_repository_check,
) -> tuple[dict[str, Any], dict[str, int]]:
    counts = {"final": 0, "preflight": 0, "official_v11": 0, "credential_audit": 0}
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
            pre_final_check=pre_final,
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
        v11_result=v11_result,
        v11_result_hash=v11_hash,
        credential_audit=audit,
        pre_final_check=pre_final,
    )
    return capsule, counts
