#!/usr/bin/env python3
"""Non-fail-fast V11 rehearsal through installed CLI and MCP boundaries."""

from __future__ import annotations

import argparse
import ast
import copy
from contextlib import contextmanager
import hashlib
import html
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import selectors
import shlex
import shutil
import signal
import sqlite3
import stat
import subprocess
import sys
import tempfile
import time
from typing import Any, Iterator
from types import SimpleNamespace


ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
_restart_spec = importlib.util.spec_from_file_location("v11_restart_recall", HERE / "restart_recall.py")
assert _restart_spec is not None and _restart_spec.loader is not None
restart_recall = importlib.util.module_from_spec(_restart_spec)
_restart_spec.loader.exec_module(restart_recall)
_multi_work_spec = importlib.util.spec_from_file_location("v11_multi_work", HERE / "multi_work.py")
assert _multi_work_spec is not None and _multi_work_spec.loader is not None
multi_work = importlib.util.module_from_spec(_multi_work_spec)
_multi_work_spec.loader.exec_module(multi_work)
_contract_spec = importlib.util.spec_from_file_location("v11_result_contract", HERE / "result_contract.py")
assert _contract_spec is not None and _contract_spec.loader is not None
result_contract = importlib.util.module_from_spec(_contract_spec)
_contract_spec.loader.exec_module(result_contract)
_multi_work_test_spec = importlib.util.spec_from_file_location(
    "v11_multi_work_self_test", HERE / "multi_work_self_test.py"
)
assert _multi_work_test_spec is not None and _multi_work_test_spec.loader is not None
multi_work_self_test = importlib.util.module_from_spec(_multi_work_test_spec)
_multi_work_test_spec.loader.exec_module(multi_work_self_test)
_metadata_spec = importlib.util.spec_from_file_location("v11_analysis_metadata", HERE / "analysis_metadata.py")
assert _metadata_spec is not None and _metadata_spec.loader is not None
analysis_metadata = importlib.util.module_from_spec(_metadata_spec)
_metadata_spec.loader.exec_module(analysis_metadata)
_performance_spec = importlib.util.spec_from_file_location("v11_performance", HERE / "performance.py")
assert _performance_spec is not None and _performance_spec.loader is not None
performance_module = importlib.util.module_from_spec(_performance_spec)
_performance_spec.loader.exec_module(performance_module)
_final_spec = importlib.util.spec_from_file_location("v11_final_evidence", HERE / "final_evidence.py")
assert _final_spec is not None and _final_spec.loader is not None
final_evidence = importlib.util.module_from_spec(_final_spec)
_final_spec.loader.exec_module(final_evidence)
PERFORMANCE = performance_module.Collector()
INSTALLER = ROOT / "rebuild/install.sh"
SMALL_FIXTURE = ROOT / "rebuild/validation/repository-intelligence/polyglot-structural/fixtures/python"
POLYGLOT_FIXTURE = HERE / "fixtures/polyglot-medium"
DECISION_REGISTER = ROOT / "rebuild/docs/design/open-decisions.md"
DECISION_REGISTER_PATH = "rebuild/docs/design/open-decisions.md"
DOCUMENT_KINDS = (
    "project-architecture-guide",
    "decision-report",
    "implementation-plan",
    "handoff-resume",
)
REQUIRED_STEPS = result_contract.COMMON_STEPS

ALLOWED_STATUS = {
    "passed",
    "partial",
    "unsupported",
    "failed",
    "environment_blocked",
    "skipped",
}
PROVIDER_SOURCE_PATHS = {
    "volicord": "rebuild/Cargo.toml",
    "small-python": "src/greeter/__init__.py",
    "polyglot-medium": "system.json",
}
OFFICIAL_REVISIT_ASSESSMENT = "reported_by_official_v11"
FAILED_REVISIT_ASSESSMENT = "official_v11_assessment_failed"
ENGINEERING_EFFECT_CATEGORIES = (
    "public_api_shape_or_semantics",
    "compatibility",
    "failure_or_error_semantics",
    "persistence_or_lifetime",
    "privacy_or_disclosure",
    "security",
    "user_visible_behavior_or_default",
    "performance_or_resource_behavior",
    "concurrency_or_operability",
    "maintenance_or_support",
    "implementation_internal",
)


def coupled_artifact_review(paths: list[str]) -> dict[str, Any]:
    categories = (
        "implementation",
        "focused_tests",
        "public_or_internal_documentation",
        "changelog_or_release_notes",
        "schema_snapshot_or_generated_artifact",
        "other_repository_owned_artifact",
    )
    return {
        "assessments": [
            {
                "category": category,
                "disposition": (
                    {"state": "included", "repository_paths": paths}
                    if category == "implementation"
                    else {"state": "no_coupled_artifact"}
                ),
                "basis_summary": "V11 repository inspection accounts for this artifact category.",
            }
            for category in categories
        ],
        "materiality_closure": {"state": "no_new_material_outcome", "commitments": [{"commitment_id": "private-fixture", "description": "Private fixture change preserves the entire current reviewed material outcome graph", "repository_paths": paths, "temporal_effect": {"state": "no_temporal_change", "outcome_id": "fixture-temporal_and_lifetime", "result_id": "unchanged", "rationale": "This fixture preserves temporal results without choosing timestamp or lifetime behavior."}, "outcome_binding": {"state": "private_equivalent", "equivalence_rationale": "The fixture introduces no new material result; all server-bound current dimensions and interactions remain unchanged"}}], "rationale": (
            "The bounded V11 artifact introduces no new material outcome beyond the current dimensions."
        )},
    }


def alternative_accounting(
    choice_id: str,
    alternative_ids: list[str],
    source_id: str,
    *,
    resolution_decision_id: str | None = None,
) -> list[dict[str, Any]]:
    accounts: list[dict[str, Any]] = []
    for index, alternative_id in enumerate(alternative_ids):
        account: dict[str, Any] = {
            "choice_id": choice_id,
            "alternative_id": alternative_id,
            "status": "unresolved",
            "rationale": "This discovered alternative remains credible on the current authority basis.",
            "source_ids": [source_id],
        }
        if resolution_decision_id:
            account.update(
                {
                    "status": (
                        "selected"
                        if index == 0
                        else "eliminated_by_applicable_decision"
                    ),
                    "rationale": (
                        "The current-host Decision selects this alternative."
                        if index == 0
                        else "The current-host Decision excludes this alternative."
                    ),
                }
            )
            if index != 0:
                account["decision_id"] = resolution_decision_id
        accounts.append(account)
    return accounts


def outside_interactions(choices, source_ids):
    """Isolated qualification choices do not alter these interactions."""
    return [{"axis": axis, "outcomes": [{
        "outcome_id": "fixture-" + axis,
        "scenario": "The bounded fixture leaves existing " + axis + " behavior unchanged",
        "credible_outcomes": [{"result_id": "unchanged", "description": "Existing interaction result is preserved"}],
        "affected_choice_ids": [], "source_basis": source_ids,
        "conclusion": {"result_id": "unchanged", "state": "no_independent_fork", "basis": "outside_affected_scope",
            "rationale": "The maintained fixture Source limits this isolated authority test; these interaction results are unchanged"},
    }]} for axis in ("reference_basis", "composition_and_precedence", "multi_item_effects", "failure_and_recovery", "temporal_and_lifetime")]


def material_boundary_review(
    choices: list[dict[str, Any]], source_ids: list[str]
) -> list[dict[str, Any]]:
    return [
        {
            "effect_category": category,
            "reviewed_outcomes": [f"Fixture observable behavior within {category}"],
            "conclusion": (
                {
                    "state": "represented_by_choices",
                    "choice_ids": [
                        choice["choice_id"]
                        for choice in choices
                        if category in choice["effect_categories"]
                    ],
                }
                if any(category in choice["effect_categories"] for choice in choices)
                else {
                    "state": "no_independent_fork",
                    "basis": "outside_affected_scope",
                    "rationale": f"The V11 source basis exposes no independent {category} fork.",
                }
            ),
            "source_ids": source_ids,
        }
        for category in ENGINEERING_EFFECT_CATEGORIES
    ]
DECISION_HEADING = re.compile(r"^## \d+\. (Q[0-9]+(?:-[A-Z])?) —", re.MULTILINE)
DECISION_ID = re.compile(r"Q[0-9]+(?:-[A-Z])?")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree_hash(directory: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(
        item
        for item in directory.rglob("*")
        if item.is_file() and ".git" not in item.relative_to(directory).parts
    ):
        relative = path.relative_to(directory).as_posix().encode()
        content = path.read_bytes()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def decision_revisit_assessment(
    source: str,
    *,
    source_sha256: str,
    source_path: str = DECISION_REGISTER_PATH,
) -> dict[str, Any]:
    """Read the maintained Decision register's explicit current trigger state."""
    headings = list(DECISION_HEADING.finditer(source))
    if not headings:
        raise ValueError("the accepted Decision register has no Question decisions")
    decision_ids = [match.group(1) for match in headings]
    if len(decision_ids) != len(set(decision_ids)):
        raise ValueError("the accepted Decision register repeats a Question decision ID")

    accepted_ids: list[str] = []
    for index, match in enumerate(headings):
        block_end = headings[index + 1].start() if index + 1 < len(headings) else len(source)
        block = source[match.end():block_end]
        statuses = re.findall(r"^- 상태: `([^`]+)`$", block, re.MULTILINE)
        if len(statuses) != 1:
            raise ValueError(f"Decision {match.group(1)} has no unambiguous maintained status")
        if statuses[0] == "accepted":
            accepted_ids.append(match.group(1))
            if block.count("- 재검토 조건:") != 1:
                raise ValueError(
                    f"accepted Decision {match.group(1)} has no unambiguous revisit-trigger section"
                )

    unresolved = re.findall(r"^- 미해결 필수 제품 질문: (.+)$", source, re.MULTILINE)
    if len(unresolved) != 1:
        raise ValueError("the Decision register has no unambiguous unresolved-question state")
    active: list[str]
    if unresolved[0].strip() == "없음":
        active = []
    else:
        active = DECISION_ID.findall(unresolved[0])
        normalized = re.sub(r"[` ,·]", "", unresolved[0])
        if not active or normalized != "".join(active):
            raise ValueError("the Decision register unresolved-question state is not a bounded ID list")
        if len(active) != len(set(active)) or any(value not in accepted_ids for value in active):
            raise ValueError("the Decision register names an unknown or duplicate active trigger")

    return {
        "decision_revisit_trigger_assessment": OFFICIAL_REVISIT_ASSESSMENT,
        "active_decision_revisit_triggers": active,
        "decision_revisit_trigger_source": {
            "kind": "accepted_decision_register",
            "path": source_path,
            "content_sha256": source_sha256,
            "assessed_decision_ids": accepted_ids,
            "assessed_decision_count": len(accepted_ids),
        },
    }


def read_decision_revisit_assessment(path: Path = DECISION_REGISTER) -> dict[str, Any]:
    source = path.read_text(encoding="utf-8")
    return decision_revisit_assessment(
        source,
        source_sha256=hashlib.sha256(source.encode("utf-8")).hexdigest(),
        source_path=path.relative_to(ROOT).as_posix(),
    )


def failed_decision_revisit_assessment(path: Path = DECISION_REGISTER) -> dict[str, Any]:
    try:
        source_hash = sha256(path)
    except OSError:
        source_hash = None
    return {
        "decision_revisit_trigger_assessment": FAILED_REVISIT_ASSESSMENT,
        "active_decision_revisit_triggers": None,
        "decision_revisit_trigger_source": {
            "kind": "accepted_decision_register",
            "path": DECISION_REGISTER_PATH,
            "content_sha256": source_hash,
            "assessed_decision_ids": [],
            "assessed_decision_count": 0,
        },
    }


def make_v11_result(
    *,
    validated_production_head: str,
    final_gate_artifact: str,
    duration_ms: float,
    repositories: list[dict[str, Any]],
    revisit_assessment: dict[str, Any],
    performance: dict[str, Any] | None = None,
    qualification: dict[str, Any] | None = None,
) -> dict[str, Any]:
    statuses = [
        value["status"]
        for repository in repositories
        for value in repository.get("steps", {}).values()
    ]
    phase_8_ready = v11_readiness(repositories, revisit_assessment, performance) and qualification_passed(
        qualification, validated_production_head, final_gate_artifact)
    result = {
        "schema_version": result_contract.SCHEMA_VERSION,
        "technical_contract": result_contract.TECHNICAL_CONTRACT,
        "workload_identity": result_contract.WORKLOAD_IDENTITY,
        "required_by_target": result_contract.required_counts(),
        "validation_id": "V11",
        "validated_production_head": validated_production_head,
        "final_gate_artifact": final_gate_artifact,
        "duration_ms": duration_ms,
        "repositories": repositories,
        "counts": {status: statuses.count(status) for status in sorted(ALLOWED_STATUS)},
        "status": "passed" if phase_8_ready else "failed",
        **revisit_assessment,
        "phase_8_ready": phase_8_ready,
    }
    if qualification is not None:
        result["qualification"] = qualification
    if performance is not None:
        result["performance"] = performance
    validate_result(result)
    return result


def process_group_active(pid: int) -> bool:
    """Ignore reparented zombies; they cannot execute or keep streams open."""
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            fields = (entry / "stat").read_text().rsplit(")", 1)[1].split()
        except (FileNotFoundError, ProcessLookupError):
            continue
        if int(fields[2]) == pid and fields[0] not in {"Z", "X"}:
            return True
    return False


def cleanup_process_group(process, grace_seconds=1.0, kill_seconds=1.0):
    """Bound Linux V11 cleanup, including children whose leader already exited."""
    sent = []
    errors = []
    for number, seconds in ((signal.SIGTERM, grace_seconds), (signal.SIGKILL, kill_seconds)):
        try:
            if not process_group_active(process.pid):
                break
            os.killpg(process.pid, number)
            sent.append(int(number))
        except ProcessLookupError:
            break
        except (OSError, ValueError, IndexError) as error:
            errors.append(type(error).__name__)
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            process.poll()
            try:
                if not process_group_active(process.pid):
                    break
            except (OSError, ValueError, IndexError) as error:
                errors.append(type(error).__name__)
                break
            time.sleep(min(0.01, max(0, deadline - time.monotonic())))
    try:
        process.wait(timeout=kill_seconds)
        complete = not process_group_active(process.pid)
    except (subprocess.TimeoutExpired, OSError, ValueError, IndexError) as error:
        errors.append(type(error).__name__)
        complete = False
    return {"complete": complete and not errors, "signals_sent": sent,
            "error_kinds": errors[:8]}


class Recorder:
    def __init__(self, root: Path):
        self.root = root
        self.sequence = 0

    def run(
        self, label: str, argv: list[str], env: dict[str, str], *,
        cwd: Path = ROOT, timeout: float = 300, cleanup_grace_seconds: float = 1.0,
    ) -> dict[str, Any]:
        self.sequence += 1
        directory = self.root / "operations" / f"{self.sequence:03d}-{label}"
        directory.mkdir(parents=True)
        started_at = time.time_ns() // 1_000
        started = time.monotonic_ns()
        metadata = {
            "schema_version": 1, "argv": argv, "command": shlex.join(argv),
            "working_directory": str(cwd), "started_at_unix_micros": started_at,
        }
        write_json(directory / "command.json", metadata)
        termination = spawn_error = exit_code = stop_cause = cleanup = interrupted = None
        try:
            # File sinks preserve complete streams without pipe backpressure or an
            # unbounded communicate() waiting for an inherited descendant pipe.
            with (directory / "stdout.log").open("wb") as stdout, (directory / "stderr.log").open("wb") as stderr:
                process = subprocess.Popen(
                    argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                    stdout=stdout, stderr=stderr, start_new_session=True,
                )
                try:
                    process.wait(timeout=timeout)
                except subprocess.TimeoutExpired:
                    stop_cause = {"kind": "timeout", "timeout_seconds": timeout}
                except BaseException as error:
                    stop_cause = {"kind": "interruption", "exception_kind": type(error).__name__}
                    interrupted = error
                finally:
                    cleanup = cleanup_process_group(process, cleanup_grace_seconds, cleanup_grace_seconds)
            if process.returncode is not None and process.returncode >= 0:
                exit_code = process.returncode
            elif process.returncode is not None:
                termination = {"kind": "signal", "number": -process.returncode}
        except OSError as error:
            spawn_error = f"{type(error).__name__}: {error}"
            (directory / "stderr.log").write_text(spawn_error + "\n")
        result = {
            **metadata, "ended_at_unix_micros": time.time_ns() // 1_000,
            "duration_ms": round((time.monotonic_ns() - started) / 1_000_000, 3),
            "exit_code": exit_code, "termination": termination,
            "stop_cause": stop_cause, "cleanup": cleanup, "spawn_error": spawn_error,
            "stdout": str(directory / "stdout.log"), "stderr": str(directory / "stderr.log"),
            "outcome": (
                "spawn_failed" if spawn_error else "terminated" if termination or stop_cause else
                "failed" if not cleanup or not cleanup["complete"] else
                "succeeded" if exit_code == 0 else "failed"
            ),
        }
        write_json(directory / "result.json", result)
        if interrupted is not None:
            raise interrupted
        PERFORMANCE.snapshots(env)
        return result


def decoded(result: dict[str, Any]) -> str:
    return Path(result["stdout"]).read_text(encoding="utf-8")


def stderr_text(result: dict[str, Any]) -> str:
    return Path(result["stderr"]).read_text(encoding="utf-8")


def contains_hangul(value: str) -> bool:
    return any("\uac00" <= character <= "\ud7a3" for character in value)


def viewer_project_understanding_evidence(
    snapshot: Path,
    understanding: dict[str, Any] | None,
) -> dict[str, Any]:
    """Inspect readable, grounded Viewer content without retaining the HTML body."""

    if not snapshot.is_file():
        return {
            "status": "failed",
            "checks": {"snapshot_available": False},
            "entity_count": 0,
            "explanation_count": 0,
            "diagram_count": 0,
            "grounded_relation_count": 0,
        }
    content = snapshot.read_text(encoding="utf-8")
    node_ids = set(re.findall(
        r'<g class="diagram-node" data-entity-id="([^"]+)"',
        content,
    ))
    relations = re.findall(
        r'<g class="diagram-edge" data-relation-id="([^"]+)" '
        r'data-relation-class="([^"]+)" data-source-entity="([^"]+)" '
        r'data-target-entity="([^"]+)"',
        content,
    )
    explanations = re.findall(
        r'<article class="deterministic-derived explanation-item"[^>]*>'
        r'<p>([^<]+)</p>',
        content,
    )
    repository_entities = (
        understanding.get("architecture", {}).get("components", [])
        if isinstance(understanding, dict)
        else []
    )
    named_entities = [
        entity.get("display_name") or entity.get("identity")
        for entity in repository_entities
        if isinstance(entity, dict)
        and isinstance(entity.get("display_name") or entity.get("identity"), str)
    ]
    grounded_relations = [
        relation
        for relation in relations
        if relation[0]
        and relation[2] in node_ids
        and relation[3] in node_ids
    ]
    reduced_architecture_message = (
        "No repository component is grounded in the current Goal, Checkpoint, or active "
        "Decision; generic topology was not substituted."
    )
    reduced_gap_with_basis = re.search(
        r'<article class="deterministic-derived explanation-item"[^>]*'
        r'data-explanation-kind="gap"[^>]*><p>[^<]*'
        r'no execution or data-flow path is inferred\.</p>'
        r'<details class="explanation-evidence">.*?'
        r'<dt>Evidence class</dt><dd>capability gap</dd>.*?</details></article>',
        content,
        flags=re.DOTALL,
    )
    explicit_reduced_architecture = (
        not node_ids
        and not relations
        and content.count(f'<p class="empty-state">{reduced_architecture_message}</p>') >= 2
        and reduced_gap_with_basis is not None
    )
    grounded_architecture = bool(node_ids) and bool(grounded_relations)
    relation_explanation_basis = (
        'class="explanation-evidence"' in content
        and 'data-relation-id="' in content
    )
    reduced_explanation_basis = (
        explicit_reduced_architecture
        and reduced_gap_with_basis is not None
    )
    checks = {
        "snapshot_available": True,
        "project_understanding_heading": (
            "Project Understanding" in content
            and "How the architecture and code connect" in content
        ),
        "readable_repository_entity_or_truthful_reduction": (
            explicit_reduced_architecture
            or any(
                len(name.strip()) >= 2 and html.escape(name, quote=True) in content
                for name in named_entities
            )
        ),
        "readable_grounded_explanation": any(
            len(explanation.strip()) >= 24 for explanation in explanations
        ),
        "fact_interpretation_distinction": all(
            marker in content
            for marker in (
                'data-statement-role="verified-fact"',
                'data-statement-role="deterministic-derived"',
                'data-statement-role="generated-interpretation"',
            )
        ),
        "grounded_architecture_result": (
            'data-diagram="architecture-topology"' in content
            and (grounded_architecture or explicit_reduced_architecture)
        ),
        "grounded_flow_diagram": 'data-diagram="flow-topology"' in content,
        "inspectable_explanation_basis": relation_explanation_basis
        or reduced_explanation_basis,
    }
    return {
        "status": "passed" if all(checks.values()) else "failed",
        "checks": checks,
        "entity_count": len(node_ids),
        "explanation_count": len(explanations),
        "diagram_count": content.count('<figure class="grounded-diagram"'),
        "grounded_relation_count": len(grounded_relations),
    }


def cli_json(
    recorder: Recorder,
    label: str,
    cli: Path,
    env: dict[str, str],
    *args: str,
    runtime: Path | None = None,
    cwd: Path = ROOT,
) -> tuple[dict[str, Any] | None, dict[str, Any]]:
    argv = [str(cli), "--json"]
    if runtime is not None:
        argv += ["--runtime", str(runtime)]
    result = recorder.run(label, argv + list(args), env, cwd=cwd)
    if result["exit_code"] != 0:
        return None, result
    try:
        value = json.loads(decoded(result))
    except json.JSONDecodeError:
        return None, result
    return value, result


def qualify_candidate_dependency_failures(
    recorder: Recorder,
    cli: Path,
    mcp_binary: Path,
    env: dict[str, str],
    runtime: Path,
    project_id: str,
) -> tuple[bool, list[dict[str, Any]]]:
    path = runtime / "candidates.sqlite3"
    connection = sqlite3.connect(path)
    connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    connection.close()
    baseline = path.read_bytes()
    sidecars = [Path(f"{path}-wal"), Path(f"{path}-shm"), Path(f"{path}-journal")]

    def restore() -> None:
        if path.is_dir():
            path.rmdir()
        elif path.exists():
            path.unlink()
        path.write_bytes(baseline)
        path.chmod(0o600)
        for sidecar in sidecars:
            if sidecar.is_file():
                sidecar.unlink()

    observations: list[dict[str, Any]] = []
    try:
        for fault, expected in (
            ("unsupported", "unsupported"),
            ("corrupt", "corrupt"),
            ("unavailable", "unavailable"),
        ):
            restore()
            if fault == "unsupported":
                with sqlite3.connect(path) as connection:
                    connection.execute(
                        "UPDATE metadata SET value = '999' WHERE key = 'schema_version'"
                    )
            elif fault == "corrupt":
                with sqlite3.connect(path) as connection:
                    connection.execute("DROP TABLE candidates")
            else:
                path.unlink()
                path.mkdir()
            cli_result, cli_operation = cli_json(
                recorder, f"candidate-{fault}-cli", cli, env,
                "--project", project_id, "advanced", "candidates"
            )
            canonical, canonical_operation = cli_json(
                recorder, f"candidate-{fault}-canonical", cli, env,
                "--project", project_id, "advanced", "records", "list",
            )
            host = None
            cleanup: dict[str, Any] = {}
            try:
                host = Mcp(mcp_binary, env)
                host.initialize()
                mcp_result, mcp_ok = host.tool(
                    "candidate_inspect", {"project_id": project_id}
                )
            except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
                mcp_result, mcp_ok = {"error": str(error)}, False
            finally:
                if host is not None:
                    cleanup = host.close()
            issue_preserved = any(
                issue.get("scope") == "candidate_inspection"
                and expected in issue.get("kind", "")
                for issue in (mcp_result or {}).get("issues", [])
            )
            passed = all((
                cli_result is not None and cli_result.get("health") == "degraded",
                cli_result is not None and cli_result.get("candidates") == [],
                canonical is not None and bool(canonical.get("records")),
                mcp_ok,
                mcp_result is not None and mcp_result.get("health") == expected,
                mcp_result is not None and mcp_result.get("candidates") == [],
                issue_preserved,
                cleanup.get("exit_code") == 0,
            ))
            observations.append({
                "fault": fault,
                "status": "passed" if passed else "failed",
                "cli": cli_result,
                "cli_operation": cli_operation,
                "canonical_usable": canonical is not None and bool(canonical.get("records")),
                "canonical_operation": canonical_operation,
                "mcp": mcp_result,
                "cleanup": cleanup,
            })
    finally:
        restore()
    return all(item["status"] == "passed" for item in observations), observations


class Mcp:
    active = set()

    def __init__(self, binary: Path, env: dict[str, str], *, rpc_timeout_seconds=None,
                 cleanup_grace_seconds=1.0):
        self.rpc_timeout_seconds = (performance_module.maintained_limits()["max_mcp_call_ms"] / 1000
                                    if rpc_timeout_seconds is None else rpc_timeout_seconds)
        if not 0 < self.rpc_timeout_seconds < float("inf"):
            raise ValueError("MCP deadline must be finite and positive")
        self.cleanup_grace_seconds = cleanup_grace_seconds
        self.stderr_sink = tempfile.TemporaryFile()
        self.stdout_sink = tempfile.TemporaryFile()
        try:
            self.process = subprocess.Popen(
                [str(binary)], cwd=ROOT, env=env, bufsize=0,
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr_sink,
                start_new_session=True,
            )
        except BaseException:
            self.stderr_sink.close()
            self.stdout_sink.close()
            raise
        os.set_blocking(self.process.stdin.fileno(), False)
        os.set_blocking(self.process.stdout.fileno(), False)
        self.performance_process = PERFORMANCE.register_process(self.process.pid)
        self.request_id = 0
        self.env = env
        self.buffer = bytearray()
        self.closed = None
        self.stop_cause = None
        self.active.add(self)

    def rpc(self, method: str, params: dict[str, Any]) -> dict[str, Any]:
        if self.closed is not None:
            raise RuntimeError("MCP connection is closed")
        operation = params.get("name", method) if method == "tools/call" else method
        try:
            with PERFORMANCE.measurement(self.performance_process, operation, self.env):
                return self._rpc(method, params)
        except BaseException as error:
            self.stop_cause = {"kind": "timeout" if isinstance(error, TimeoutError) else
                               "rpc_failure" if isinstance(error, Exception) else "interruption",
                               "exception_kind": type(error).__name__}
            self.close()
            raise

    @staticmethod
    def _unique_object(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError("duplicate JSON member")
            value[key] = item
        return value

    def _rpc(self, method: str, params: dict[str, Any]) -> dict[str, Any]:
        self.request_id += 1
        deadline = time.monotonic() + self.rpc_timeout_seconds
        message = {"jsonrpc": "2.0", "id": self.request_id, "method": method, "params": params}
        pending = memoryview((json.dumps(message, separators=(",", ":")) + "\n").encode())
        with selectors.DefaultSelector() as selector:
            selector.register(self.process.stdin, selectors.EVENT_WRITE)
            selector.register(self.process.stdout, selectors.EVENT_READ)
            while True:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError("MCP RPC deadline expired")
                if not pending and b"\n" in self.buffer:
                    line, _, remainder = self.buffer.partition(b"\n")
                    self.buffer = bytearray(remainder)
                    break
                for key, _events in selector.select(remaining):
                    if key.fileobj is self.process.stdin:
                        count = os.write(key.fd, pending[:65536])
                        pending = pending[count:]
                        if not pending:
                            selector.unregister(self.process.stdin)
                    else:
                        chunk = os.read(key.fd, 65536)
                        if not chunk:
                            raise RuntimeError("MCP ended before a complete response frame")
                        self.stdout_sink.write(chunk)
                        self.buffer.extend(chunk)
        try:
            response = json.loads(line, object_pairs_hook=self._unique_object,
                                  parse_constant=lambda _value: (_ for _ in ()).throw(ValueError()))
        except (ValueError, UnicodeError):
            raise RuntimeError("MCP response is malformed JSON") from None
        if (not isinstance(response, dict) or response.get("jsonrpc") != "2.0"
                or type(response.get("id")) is not int or response["id"] != self.request_id
                or ("result" in response) == ("error" in response)):
            raise RuntimeError("MCP response identity or result/error framing is invalid")
        if "error" in response:
            error = response["error"]
            if (not isinstance(error, dict) or type(error.get("code")) is not int
                    or not isinstance(error.get("message"), str)):
                raise RuntimeError("MCP RPC error framing is invalid")
            raise RuntimeError("MCP returned an explicit RPC error")
        if time.monotonic() >= deadline:
            raise TimeoutError("MCP RPC deadline expired during response validation")
        if not isinstance(response["result"], dict):
            raise RuntimeError("MCP result shape is invalid")
        return response

    def initialize(self) -> list[dict[str, Any]]:
        self.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}})
        catalog = self.rpc("tools/list", {})
        tools = catalog["result"].get("tools")
        if not isinstance(tools, list) or any(not isinstance(tool, dict) for tool in tools):
            raise RuntimeError("MCP tool catalog shape is invalid")
        return tools

    def tool(self, name: str, arguments: dict[str, Any]) -> tuple[dict[str, Any] | None, bool]:
        response = self.rpc("tools/call", {"name": name, "arguments": arguments})
        result = response["result"]
        content = result.get("structuredContent")
        return (content if isinstance(content, dict) else None,
                isinstance(content, dict) and result.get("isError") is False)

    def close(self) -> dict[str, Any]:
        if self.closed is not None:
            return self.closed
        self.process.stdin.close()
        try:
            self.process.wait(timeout=self.cleanup_grace_seconds)
        except subprocess.TimeoutExpired:
            pass
        cleanup = cleanup_process_group(self.process, self.cleanup_grace_seconds, self.cleanup_grace_seconds)
        # Consume only immediately available stdout, never wait for descendant EOF.
        drain_deadline = time.monotonic() + self.cleanup_grace_seconds
        while time.monotonic() < drain_deadline:
            try:
                chunk = os.read(self.process.stdout.fileno(), 65536)
            except BlockingIOError:
                break
            if not chunk:
                break
            self.stdout_sink.write(chunk)
        self.process.stdout.close()
        self.stdout_sink.seek(0)
        self.stderr_sink.seek(0)
        code = self.process.returncode
        self.closed = {"exit_code": code, "termination": (
            {"kind": "signal", "number": -code} if code is not None and code < 0 else None),
            "stop_cause": self.stop_cause, "cleanup": cleanup,
            "stdout": self.stdout_sink.read(os.fstat(self.stdout_sink.fileno()).st_size).decode("utf-8", errors="replace"),
            "stderr": self.stderr_sink.read(os.fstat(self.stderr_sink.fileno()).st_size).decode("utf-8", errors="replace")}
        self.stdout_sink.close()
        self.stderr_sink.close()
        self.active.discard(self)
        return self.closed


def step(status: str, summary: str, **evidence: Any) -> dict[str, Any]:
    if status not in ALLOWED_STATUS:
        raise ValueError(f"invalid V11 status: {status}")
    return {"status": status, "summary": summary, "evidence": evidence}


def qualify_materiality_scenarios(binary: Path, env: dict[str, str], root: Path) -> dict[str, Any]:
    spec = importlib.util.spec_from_file_location("v11_materiality_scenarios", HERE / "materiality_scenarios.py")
    if spec is None or spec.loader is None:
        raise RuntimeError("maintained Materiality scenarios unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    api = SimpleNamespace(Mcp=Mcp, material_boundary_review=material_boundary_review, outside_interactions=outside_interactions,
                          alternative_accounting=alternative_accounting,
                          coupled_artifact_review=coupled_artifact_review)
    return module.qualify(api, binary, env, root)


def parser_degradation_status(result: dict[str, Any] | None) -> str:
    degraded_scopes = (
        [
            *result.get("partial_scopes", []),
            *result.get("failed_scopes", []),
        ]
        if result
        else []
    )
    return (
        "passed"
        if result
        and result.get("state") == "partial"
        and any(scope.startswith("Structural:") for scope in degraded_scopes)
        else "failed"
    )


def recall_meaning(value: dict[str, Any] | None) -> dict[str, Any] | None:
    if value is None:
        return None
    return {key: item for key, item in value.items()
            if key not in {"used_sources", "source_details", "snapshots"}}


def recovery_recall_checks(
    before: dict[str, Any] | None,
    after: dict[str, Any] | None,
    prior_analysis_id: str | None,
    repaired: dict[str, Any] | None,
    capability_evidence: tuple[dict[str, Any], dict[str, Any]] | None = None,
) -> dict[str, bool]:
    """Qualify the controlled repair's canonical meaning and refreshed read basis."""
    checks = dict.fromkeys((
        "canonical_recall_meaning_unchanged", "source_catalogs_consistent",
        "retained_source_provenance_unchanged", "repository_source_refreshed",
        "analysis_basis_refreshed", "capability_meaning_preserved",
    ), False)
    required = {
        "project_id", "goals", "goal_basis", "decisions", "checkpoint",
        "open_questions", "risks_assumptions_and_limits", "declared_assumptions",
        "next_step", "used_sources", "source_details", "snapshots",
    }
    if not all(isinstance(value, dict) and required <= value.keys() for value in (before, after)):
        return checks
    checks["canonical_recall_meaning_unchanged"] = recall_meaning(before) == recall_meaning(after)

    def source_catalog(value: dict[str, Any]) -> dict[str, Any] | None:
        sources = value["source_details"]
        if not isinstance(sources, list) or not all(
            isinstance(source, dict)
            and isinstance(source.get("identity"), str)
            and re.fullmatch(r"[0-9a-f]{32}", source["identity"])
            and isinstance(source.get("actor"), dict)
            and source["actor"].get("kind") in {"User", "Agent", "Repository", "Command", "Provider", "Generator", "Importer"}
            and (source.get("snapshot_basis") is None or isinstance(source["snapshot_basis"], str))
            and source.get("availability") in {"available", "unavailable", "stale", "unknown"}
            and source.get("freshness") in {"current", "unavailable", "stale", "unknown"}
            for source in sources
        ):
            return None
        identities = [source["identity"] for source in sources]
        if identities != value["used_sources"] or len(set(identities)) != len(identities):
            return None
        return {source["identity"]: source for source in sources}

    old_sources, new_sources = source_catalog(before), source_catalog(after)
    if old_sources is not None and new_sources is not None:
        checks["source_catalogs_consistent"] = True
        def non_repository(sources: dict[str, Any]) -> dict[str, Any]:
            return {identity: source for identity, source in sources.items()
                    if source["actor"]["kind"] != "Repository"}
        checks["retained_source_provenance_unchanged"] = (
            non_repository(old_sources) == non_repository(new_sources)
            and all(old_sources[key] == new_sources[key] for key in old_sources.keys() & new_sources.keys())
        )
        added = [source for key, source in new_sources.items() if key not in old_sources]
        old_basis = {source.get("snapshot_basis") for source in old_sources.values()}
        checks["repository_source_refreshed"] = bool(added) and all(
            source["actor"]["kind"] == "Repository"
            and source["availability"] == "available" and source["freshness"] == "current"
            and isinstance(source.get("snapshot_basis"), str) and source["snapshot_basis"]
            and source["snapshot_basis"] not in old_basis
            for source in added
        )

    def current_snapshot(snapshot: Any) -> bool:
        if not isinstance(snapshot, dict):
            return False
        repository_id = snapshot.get("repository_snapshot")
        capabilities = snapshot.get("capabilities")
        return bool(
            isinstance(repository_id, str) and re.fullmatch(r"[0-9a-f]{64}", repository_id)
            and isinstance(snapshot.get("analysis_snapshot"), str)
            and re.fullmatch(r"[0-9a-f]{64}", snapshot["analysis_snapshot"])
            and isinstance(snapshot.get("freshness"), dict)
            and snapshot["freshness"].get("state") == "current"
            and snapshot["freshness"].get("compared_repository_snapshot") in (None, repository_id)
            and snapshot["freshness"].get("repository_snapshot") == repository_id
            and isinstance(capabilities, list) and capabilities
            and all(
                isinstance(capability, dict)
                and capability.get("repository_snapshot") == repository_id
                and isinstance(capability.get("freshness"), dict)
                and capability["freshness"].get("state") == "current"
                and capability["freshness"].get("compared_repository_snapshot") in (None, repository_id)
                and capability["freshness"].get("repository_snapshot") == repository_id
                and isinstance(capability.get("coverage"), dict)
                and {"included", "excluded", "unsupported", "unavailable", "failed", "stale",
                     "covered_file_count", "covered_entity_count", "covered_relation_count"} <= capability["coverage"].keys()
                for capability in capabilities
            )
        )

    old_snapshots, new_snapshots = before["snapshots"], after["snapshots"]
    if not isinstance(old_snapshots, list) or not isinstance(new_snapshots, list):
        return checks
    old = [snapshot for snapshot in old_snapshots if isinstance(snapshot, dict)
           and snapshot.get("analysis_snapshot") == prior_analysis_id]
    if len(old) != 1 or len(new_snapshots) != 1:
        return checks
    old_snapshot, new_snapshot = old[0], new_snapshots[0]
    if capability_evidence is not None:
        expanded = []
        for snapshot, evidence in zip((old_snapshot, new_snapshot), capability_evidence):
            if not all(snapshot.get(key) == evidence.get(key) for key in (
                "analysis_snapshot", "repository_snapshot",
            )) or not bounded_projection_matches(evidence.get("capabilities"), snapshot.get("capabilities")):
                return checks
            expanded.append({**snapshot, "capabilities": evidence["capabilities"]})
        old_snapshot, new_snapshot = expanded
    if not all(map(current_snapshot, (old_snapshot, new_snapshot))):
        return checks
    checks["analysis_basis_refreshed"] = bool(
        repaired and new_snapshot["analysis_snapshot"] == repaired.get("analysis_snapshot")
        and new_snapshot["analysis_snapshot"] != prior_analysis_id
        and new_snapshot["repository_snapshot"] != old_snapshot["repository_snapshot"]
    )
    def capability_meaning(snapshot: dict[str, Any]) -> list[dict[str, Any]]:
        return [{key: value for key, value in capability.items()
                 if key not in {"repository_snapshot", "freshness", "observed_at_unix_micros"}}
                for capability in snapshot["capabilities"]]
    checks["capability_meaning_preserved"] = capability_meaning(old_snapshot) == capability_meaning(new_snapshot)
    return checks


def bounded_projection_matches(full: Any, projected: Any) -> bool:
    """Validate visible content and exact omissions against same-identity local evidence."""
    if isinstance(projected, dict) and "transport_omission" in projected:
        omission = projected["transport_omission"]
        if not isinstance(omission, dict) or omission.get("reason") != "serialized_byte_budget":
            return False
        if "exact_json_bytes" in omission:
            return len(projected) == 1 and omission["exact_json_bytes"] == len(
                json.dumps(full, ensure_ascii=False, separators=(",", ":")).encode()
            )
        visible = {key: value for key, value in projected.items() if key != "transport_omission"}
        return bool(
            isinstance(full, dict) and visible.keys() <= full.keys()
            and omission.get("omitted_field_count") == len(full.keys() - visible.keys())
            and omission["omitted_field_count"] > 0
            and all(bounded_projection_matches(full[key], value) for key, value in visible.items())
        )
    if isinstance(full, dict) and isinstance(projected, dict):
        return full.keys() == projected.keys() and all(
            bounded_projection_matches(full[key], value) for key, value in projected.items()
        )
    if isinstance(full, list) and isinstance(projected, list):
        visible = projected
        if projected and isinstance(projected[-1], dict):
            omission = projected[-1].get("transport_omission", {})
            if isinstance(omission, dict) and "omitted_count" in omission:
                visible = projected[:-1]
                if (len(projected[-1]) != 1 or omission.get("reason") != "serialized_byte_budget"
                        or omission["omitted_count"] != len(full) - len(visible)
                        or omission["omitted_count"] <= 0):
                    return False
                return all(bounded_projection_matches(a, b) for a, b in zip(full, visible))
        return len(full) == len(visible) and all(
            bounded_projection_matches(a, b) for a, b in zip(full, visible)
        )
    return type(full) is type(projected) and full == projected


def read_analysis_capabilities(path: Path, analysis_id: str, project_id: str) -> dict[str, Any]:
    """Read verified, compressed metadata without loading the analysis graph."""
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        manifest = None
    if (
        isinstance(manifest, dict)
        and manifest.get("storage_format") == "volicord.normalized_analysis.v2"
    ):
        metadata = analysis_metadata.read_metadata(path, manifest.get("metadata_blob", ""))
        if (
            manifest.get("identity") != metadata.get("identity")
            or manifest.get("project") != metadata.get("project")
            or metadata.get("identity") != analysis_id
            or metadata.get("project") != {"identity": project_id}
        ):
            raise ValueError("Analysis Snapshot capability evidence identity mismatch")
        if not isinstance(metadata.get("repository_snapshot"), str) or not isinstance(
            metadata.get("capabilities"), list
        ):
            raise ValueError("Analysis Snapshot manifest capability metadata is incomplete")
        return {
            "analysis_snapshot": metadata["identity"],
            "repository_snapshot": metadata["repository_snapshot"],
            "capabilities": metadata["capabilities"],
        }

    decoder = json.JSONDecoder()
    with path.open(encoding="utf-8") as stream:
        buffer = ""

        def token(expected: str | None = None) -> Any:
            nonlocal buffer
            while True:
                buffer = buffer.lstrip()
                if buffer:
                    if expected is not None:
                        if not buffer.startswith(expected):
                            raise ValueError("invalid Analysis Snapshot metadata delimiter")
                        buffer = buffer[len(expected):]
                        return None
                    try:
                        value, end = decoder.raw_decode(buffer)
                    except json.JSONDecodeError:
                        pass
                    else:
                        # A number at a chunk boundary may still be incomplete.
                        if end < len(buffer):
                            buffer = buffer[end:]
                            return value
                chunk = stream.read(65536)
                if not chunk:
                    raise ValueError("incomplete Analysis Snapshot capability evidence")
                buffer += chunk

        token("{")
        fields: dict[str, Any] = {}
        required = {"identity", "repository_snapshot", "project", "capabilities"}
        while True:
            key = token()
            if not isinstance(key, str):
                raise ValueError("invalid Analysis Snapshot metadata key")
            token(":")
            value = token()
            if key in required:
                if key in fields:
                    raise ValueError("duplicate Analysis Snapshot metadata")
                fields[key] = value
            if required <= fields.keys():
                break
            token(",")
    if fields["identity"] != analysis_id or fields["project"] != {"identity": project_id}:
        raise ValueError("Analysis Snapshot capability evidence identity mismatch")
    return {
        "analysis_snapshot": fields["identity"],
        "repository_snapshot": fields["repository_snapshot"],
        "capabilities": fields["capabilities"],
    }


def completed_learning_recalled(items: Any) -> bool:
    return isinstance(items, list) and any(
        isinstance(item, dict) and isinstance(item.get("state"), dict)
        and item["state"].get("state") == "completed"
        and item.get("canonical_decision") is False
        and isinstance(item.get("candidate_id"), str)
        and re.fullmatch(r"[0-9a-f]{32}", item["candidate_id"])
        for item in items
    )


def canonical_record(
    inspection: dict[str, Any] | None,
    kind: str,
    *,
    lifecycle_state: str | None = None,
) -> dict[str, Any] | None:
    records = inspection.get("records", []) if inspection else []
    return next(
        (
            record
            for record in records
            if record.get("kind") == kind
            and (lifecycle_state is None or record.get("lifecycle_state") == lifecycle_state)
        ),
        None,
    )


def candidate_repository_source_basis(
    analysis: dict[str, Any] | None,
) -> list[str]:
    source_id = analysis.get("repository_source_id") if analysis else None
    return [source_id] if isinstance(source_id, str) and source_id else []


def unsupported_cli(*operations: dict[str, Any]) -> bool:
    attempted = [operation for operation in operations if operation.get("exit_code") is not None]
    return bool(attempted) and any(
        operation.get("exit_code") == 2
        or "unsupported" in stderr_text(operation).lower()
        for operation in attempted
    )


class AuthenticationCleanupError(RuntimeError):
    """The bounded authenticated Codex staging directory could not be removed."""


@contextmanager
def staged_codex_authentication(
    source_auth: Path,
    registered_codex_home: Path,
    retained_root: Path,
    *,
    staging_parent: Path | None = None,
    credential_material: dict | None = None,
) -> Iterator[Path]:
    """Yield a Codex home containing only the registered config and staged auth."""
    if staging_parent is not None:
        staging_parent.mkdir(parents=True, exist_ok=True)
    temporary = tempfile.TemporaryDirectory(
        prefix="volicord-v11-codex-auth-",
        dir=staging_parent,
    )
    staging_directory = Path(temporary.name)
    staged_auth = staging_directory / "codex-home/auth.json"
    try:
        retained = retained_root.resolve()
        staged = staging_directory.resolve()
        if staged == retained or retained in staged.parents:
            raise RuntimeError("Codex authentication staging resolved inside retained V11 artifacts")

        codex_home = staging_directory / "codex-home"
        codex_home.mkdir(mode=0o700)
        registered_config = registered_codex_home / "config.toml"
        if not registered_config.is_file():
            raise FileNotFoundError("the isolated V11 Codex registration is unavailable")
        shutil.copyfile(registered_config, codex_home / "config.toml")
        (codex_home / "config.toml").chmod(0o600)
        shutil.copyfile(source_auth, codex_home / "auth.json")
        (codex_home / "auth.json").chmod(0o600)
        if credential_material is not None:
            capture_credential_material(staged_auth, credential_material)
        yield codex_home
    finally:
        # Codex can refresh staged tokens. Keep both versions in memory until the
        # raw retained-artifact audit, and always remove staging afterwards.
        if credential_material is not None and staged_auth.exists():
            capture_credential_material(staged_auth, credential_material)
        try:
            temporary.cleanup()
        except OSError as error:
            raise AuthenticationCleanupError(
                f"temporary Codex authentication cleanup failed: {type(error).__name__}"
            ) from error
        if staging_directory.exists():
            raise AuthenticationCleanupError("temporary Codex authentication directory remains")


def git_revision(recorder: Recorder, repository: Path, env: dict[str, str]) -> str:
    result = recorder.run("git-revision", ["git", "rev-parse", "HEAD"], env, cwd=repository)
    return decoded(result).strip() if result["exit_code"] == 0 else "unavailable"


def prepare_repository(kind: str, destination: Path, recorder: Recorder, env: dict[str, str]) -> dict[str, str]:
    if kind == "volicord":
        result = recorder.run(
            "clone-volicord", ["git", "clone", "--quiet", "--no-hardlinks", str(ROOT), str(destination)], env
        )
        if result["exit_code"] != 0:
            raise RuntimeError(stderr_text(result))
    else:
        source = SMALL_FIXTURE if kind == "small-python" else POLYGLOT_FIXTURE
        shutil.copytree(source, destination)
        for argv in (
            ["git", "init", "--quiet"],
            ["git", "add", "."],
            ["git", "-c", "user.name=V11", "-c", "user.email=v11@example.invalid", "commit", "--quiet", "-m", "fixture"],
        ):
            result = recorder.run("fixture-git", argv, env, cwd=destination)
            if result["exit_code"] != 0:
                raise RuntimeError(stderr_text(result))
    return {"revision": git_revision(recorder, destination, env), "content_sha256": tree_hash(destination)}


def authenticated_codex(
    recorder: Recorder,
    codex: str | None,
    env: dict[str, str],
    repository: Path,
    project_id: str,
    retained_root: Path,
    *,
    model: str,
    staging_parent: Path | None = None,
    authentication_source: Path | None = None,
) -> dict[str, Any]:
    if not model.strip():
        return step("environment_blocked", "an explicit Codex probe model is required")
    auth = authentication_source or (
        Path(os.environ.get("CODEX_HOME", str(Path.home() / ".codex"))) / "auth.json"
    )
    if codex is None:
        return step("environment_blocked", "Codex CLI is unavailable")
    if not auth.is_file():
        return step("environment_blocked", "Codex authentication is unavailable")
    prompt = (
        "This is a bounded MCP connectivity probe, not repository work. Call only this "
        "repository's Volicord MCP server project_health tool for Project "
        f"{project_id}. Do not call project_resolve, Recall, context_record, "
        "repository_analyze, another tool, or the shell. Report the returned connection "
        "and capability state."
    )
    credential_material = {"needles": set(), "errors": 0}
    try:
        with staged_codex_authentication(
            auth,
            Path(env["CODEX_HOME"]),
            retained_root,
            staging_parent=staging_parent,
            credential_material=credential_material,
        ) as codex_home:
            result = recorder.run(
                "authenticated-codex",
                [
                    codex, "--dangerously-bypass-hook-trust", "--ask-for-approval", "never", "--config",
                    'mcp_servers.volicord.tools.project_health.approval_mode="approve"',
                    "exec", "--model", model, "--ephemeral", "--json", "--sandbox", "read-only",
                    "--skip-git-repo-check", "-C", str(repository), prompt,
                ],
                env | {"CODEX_HOME": str(codex_home)},
                timeout=180,
            )
    except AuthenticationCleanupError as error:
        return step("failed", "temporary Codex authentication cleanup failed", error=str(error))
    except OSError as error:
        return step(
            "environment_blocked",
            "authenticated Codex material could not be staged",
            error=f"{type(error).__name__}: {error}",
        )
    retention = credential_retention_audit(retained_root, known_material=credential_material)
    write_json(Path(result["stdout"]).with_name("credential-audit.json"), retention)
    if retention["status"] != "passed":
        return step("failed", "authenticated credential retention audit failed",
                    credential_audit=retention, operation=result)
    if (result["exit_code"] != 0 or result.get("termination") is not None
        or result.get("spawn_error") is not None or result.get("outcome") != "succeeded"):
        return step("environment_blocked", "authenticated Codex turn did not complete", operation=result)
    try:
        proof = validate_codex_probe(decoded(result), project_id)
    except ValueError as error:
        return step("failed", str(error), operation=result)
    return step("passed", "authenticated Codex completed the bounded project_health probe",
                operation=result, probe=proof, credential_audit=retention)


def validate_codex_probe(stdout: str, project_id: str) -> dict[str, Any]:
    """Validate codex exec --json items, rather than dogfood rollout envelopes.

    CLI items use snake-case structured_content and may omit isError; the
    rollout helper handles a different ItemCompleted/CallToolResult shape.
    Both result representations must agree when supplied.
    """
    started = None
    completed = None
    turn_started = False
    turn_completed = False
    for line in stdout.splitlines():
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError("Codex probe contains malformed JSON events") from error
        if not isinstance(event, dict) or turn_completed:
            raise ValueError("Codex probe contains invalid or post-turn events")
        kind = event.get("type")
        if kind == "thread.started":
            if turn_started or not isinstance(event.get("thread_id"), str):
                raise ValueError("Codex probe has invalid thread identity")
        elif kind == "turn.started":
            if turn_started:
                raise ValueError("Codex probe contains additional turns")
            turn_started = True
        elif kind == "turn.completed":
            if not turn_started or completed is None:
                raise ValueError("Codex probe turn lacks successful tool completion")
            turn_completed = True
        elif kind in {"item.started", "item.completed"}:
            item = event.get("item")
            if not isinstance(item, dict):
                raise ValueError("Codex probe has malformed item evidence")
            item_kind = item.get("type")
            if item_kind in {"agent_message", "reasoning", "error"}:
                continue  # Non-action items never count as tool evidence.
            if item_kind != "mcp_tool_call" or not turn_started:
                raise ValueError("Codex probe contains forbidden additional activity")
            if (item.get("server") != "volicord" or item.get("tool") != "project_health"
                or item.get("arguments") != {"project_id": project_id}
                or not isinstance(item.get("id"), str) or not item["id"]):
                raise ValueError("Codex probe tool or Project identity is invalid")
            if kind == "item.started":
                if started is not None or completed is not None or item.get("status") != "in_progress":
                    raise ValueError("Codex probe must contain exactly one tool call")
                started = item
                continue
            if (started is None or completed is not None or item["id"] != started["id"]
                or item.get("status") != "completed" or item.get("error") is not None):
                raise ValueError("Codex probe tool did not complete successfully")
            result = item.get("result")
            if not isinstance(result, dict) or (
                "isError" in result and result["isError"] is not False
            ):
                raise ValueError("Codex probe returned an invalid or error result")
            content = result.get("content")
            if not isinstance(content, list) or len(content) != 1 or not isinstance(content[0], dict):
                raise ValueError("Codex probe lacks a typed health result")
            if content[0].get("type") != "text" or not isinstance(content[0].get("text"), str):
                raise ValueError("Codex probe health content is invalid")
            try:
                health = json.loads(content[0]["text"])
            except json.JSONDecodeError as error:
                raise ValueError("Codex probe health result is not structured JSON") from error
            if "structured_content" in result and result["structured_content"] != health:
                raise ValueError("Codex probe health result representations disagree")
            if (not isinstance(health, dict) or health.get("connection") != "connected"
                or health.get("capability_state") != "healthy"
                or health.get("canonical_available") is not True
                or health.get("repository_available") is not True
                or health.get("issues") != []
                or not isinstance(health.get("runtime_root"), str) or not health["runtime_root"]):
                raise ValueError("Codex probe health is disconnected or unavailable")
            completed = {"item_id": item["id"], "server": "volicord", "tool": "project_health",
                         "project_id": project_id, "status": "completed", "health": health}
        else:
            raise ValueError("Codex probe contains failed or unsupported events")
    if not turn_completed or completed is None:
        raise ValueError("Codex probe lacks a completed successful project_health turn")
    return completed


def assert_codex_probe_completion() -> None:
    events = [json.loads(line) for line in (HERE / "fixtures/authenticated-project-health.jsonl")
              .read_text(encoding="utf-8").splitlines()]
    def encode(value):
        return "\n".join(json.dumps(event) for event in value)
    validate_codex_probe(encode(events), "synthetic-project")
    cases = {"started-only": events[:3], "text-only": [
        {"type": "item.completed", "item": {"type": "agent_message",
         "text": "mcp_tool_call volicord project_health"}}]}
    for label, field, value in (
        ("failed", "status", "failed"),
        ("error", "error", {"message": "failed"}),
        ("wrong-server", "server", "other"),
        ("wrong-tool", "tool", "recall"),
    ):
        mutated = copy.deepcopy(events)
        mutated[3]["item"][field] = value
        cases[label] = mutated
    mutated = copy.deepcopy(events)
    for event in mutated[2:4]:
        event["item"]["arguments"]["project_id"] = "another-project"
    cases["wrong-project"] = mutated
    for label, field, value in (
        ("disconnected", "connection", "disconnected"),
        ("unhealthy", "capability_state", "unavailable"),
        ("canonical-unavailable", "canonical_available", False),
        ("repository-unavailable", "repository_available", False),
    ):
        mutated = copy.deepcopy(events)
        result = mutated[3]["item"]["result"]
        result["structured_content"][field] = value
        result["content"][0]["text"] = json.dumps(result["structured_content"])
        cases[label] = mutated
    mutated = copy.deepcopy(events)
    mutated[3]["item"]["result"]["content"][0]["text"] = '{}'
    cases["conflicting-result"] = mutated
    for kind in ("command_execution", "file_change", "web_search", "unknown_action"):
        cases[kind] = events[:-1] + [{"type": "item.started", "item": {
            "id": "forbidden", "type": kind}}] + events[-1:]
    extra = copy.deepcopy(events[3])
    extra["item"].update(id="extra", tool="recall")
    cases["extra-tool"] = events[:-1] + [extra] + events[-1:]
    cases["duplicate-completion"] = events[:-1] + [events[3]] + events[-1:]
    cases["unfinished-turn"] = events[:-1]
    cases["failed-turn"] = events[:-1] + [{"type": "turn.failed", "error": {"message": "failed"}}]
    accepted = []
    for label, mutated in cases.items():
        try:
            validate_codex_probe(encode(mutated), "synthetic-project")
        except ValueError:
            pass
        else:
            accepted.append(label)
    if accepted:
        raise AssertionError(f"invalid bounded Codex probes accepted: {accepted}")


def rehearse_target(
    target_kind: str,
    run_root: Path,
    recorder: Recorder,
    base_env: dict[str, str],
    codex: str | None,
    model: str,
) -> dict[str, Any]:
    target_root = run_root / "work" / target_kind
    home = target_root / "home"
    prefix = target_root / "prefix"
    runtime = target_root / "runtime"
    repository = target_root / "repository"
    clone = target_root / "clone"
    clone_runtime = target_root / "clone-runtime"
    codex_home = home / ".codex"
    legacy = target_root / "legacy-runtime"
    for path in (home, codex_home, legacy):
        path.mkdir(parents=True, exist_ok=True)
    legacy_sentinel = legacy / "DO-NOT-READ"
    legacy_sentinel.write_text("legacy sentinel\n", encoding="utf-8")
    legacy_before = (legacy_sentinel.stat().st_mtime_ns, sha256(legacy_sentinel))
    env = base_env | {
        "HOME": str(home),
        "XDG_DATA_HOME": str(home / ".local/share"),
        "CODEX_HOME": str(codex_home),
        "VOLICORD_RUNTIME_DIR": str(runtime),
        "VOLICORD_HOME": str(legacy),
        "PATH": f"{prefix / 'bin'}:{base_env.get('PATH', '')}",
    }
    identity = prepare_repository(target_kind, repository, recorder, env)
    steps: dict[str, dict[str, Any]] = {}

    install = recorder.run(
        "install", [str(INSTALLER), "--prefix", str(prefix), "--runtime-dir", str(runtime)], env
    )
    cli = prefix / "bin/volicord"
    mcp_binary = prefix / "bin/volicord-mcp"
    installation_only = install["exit_code"] == 0 and not (codex_home / "config.toml").exists()
    activation = recorder.run(
        "codex-enable",
        [str(cli), "--repository", str(repository), "codex", "enable"],
        env,
        cwd=repository,
    ) if installation_only else {"exit_code": None}
    codex_home.joinpath("config.toml").write_text(
        f'[projects.{json.dumps(str(repository.resolve()))}]\ntrust_level = "trusted"\n',
        encoding="utf-8",
    )
    installed = installation_only and activation["exit_code"] == 0 and all(
        path.is_file() and path.stat().st_mode & stat.S_IXUSR
        for path in (cli, prefix / "bin/volicord-viewer", mcp_binary)
    )
    steps["clean_install"] = step(
        "passed" if installed else "failed",
        "isolated replacement install completed" if installed else "isolated install failed",
        operation=install,
        repository_activation=activation,
        installation_created_global_registration=not installation_only,
    )
    if not installed:
        for name in result_contract.required_steps_for_target(target_kind)[1:]:
            steps[name] = step("skipped", "prerequisite clean installation failed")
        return {"class": target_kind, "identity": identity, "steps": steps}

    initialized, init_op = cli_json(
        recorder, "project-init", cli, env, "init", f"V11 {target_kind}", cwd=repository
    )
    project_id = initialized.get("project_id") if initialized else None
    steps["project_binding"] = step(
        "passed" if project_id and initialized.get("binding", {}).get("path") == str(repository.resolve()) else "failed",
        "Project initialized with an explicit clone binding" if project_id else "Project initialization failed",
        operation=init_op, project_id=project_id,
    )
    project_prerequisite_status = "passed" if project_id else "skipped"
    if not project_id:
        for name in result_contract.required_steps_for_target(target_kind)[3:]:
            steps[name] = step(project_prerequisite_status, "Project initialization failed")
        steps["codex_mcp_connection"] = step(
            project_prerequisite_status,
            "Project initialization failed",
        )
        return {"class": target_kind, "identity": identity, "steps": steps}

    mcp_evidence: dict[str, Any] = {}
    try:
        host = Mcp(mcp_binary, env)
        catalog = host.initialize()
        health, health_ok = host.tool("project_health", {"project_id": project_id})
        mcp_evidence = {
            "tool_names": sorted(tool["name"] for tool in catalog),
            "health": health,
            "cleanup": host.close(),
        }
        direct_ok = health_ok and health and health.get("connection") == "connected"
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        direct_ok = False
        mcp_evidence = {"error": str(error)}
    codex_result = authenticated_codex(
        recorder, codex, env, repository, project_id, target_root, model=model
    )
    combined_status = "passed" if direct_ok and codex_result["status"] == "passed" else (
        "environment_blocked" if direct_ok and codex_result["status"] == "environment_blocked" else "failed"
    )
    steps["codex_mcp_connection"] = step(
        combined_status, "installed MCP connection and authenticated Codex probe evaluated",
        direct=mcp_evidence, authenticated=codex_result,
    )

    status_before, status_before_op = cli_json(
        recorder, "project-understanding-before-analysis", cli, env, "status", cwd=repository
    )
    analysis, analysis_op = cli_json(
        recorder, "analyze", cli, env, "analyze", cwd=repository
    )
    analysis_ok = analysis is not None and analysis.get("state") in {"succeeded", "partial"}
    steps["repository_analysis"] = step(
        "passed" if analysis_ok else "failed", "inventory/capability analysis returned structured coverage",
        operation=analysis_op, result=analysis,
    )
    try:
        host = Mcp(mcp_binary, env)
        host.initialize()
        understanding, understanding_ok = host.tool("repository_understanding", {"project_id": project_id})
        understanding_cleanup = host.close()
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        understanding, understanding_ok, understanding_cleanup = {"error": str(error)}, False, {}
    try:
        language_host = Mcp(mcp_binary, env)
        language_host.initialize()
        language_plan, language_plan_ok = language_host.tool(
            "document_preview",
            {
                "project_id": project_id,
                "kind": "handoff-resume",
                "format": "markdown",
                "language": "ko-KR",
                "locale": "en",
            },
        )
        plan = (language_plan or {}).get("plan", {})
        realization = {
            "plan_fingerprint": plan.get("plan_fingerprint"),
            "requested_language": "ko-KR",
            "all_generated_prose_realized": True,
            "title": "프로젝트 인수인계와 작업 재개",
            "sections": [
                {
                    "key": section.get("key"),
                    "title": f"한국어 설명: {section.get('source_title', '')}",
                    "claims": [
                        {
                            "identity": claim.get("identity"),
                            "text": f"한국어 설명: {claim.get('source_text', '')}",
                        }
                        for claim in section.get("claims", [])
                    ],
                }
                for section in plan.get("sections", [])
            ],
            "generator": {
                "generator": "v11-current-host",
                "agent": "codex",
                "model": "deterministic-v11-realizer",
            },
        }
        realized_document, realized_ok = language_host.tool(
            "document_preview",
            {
                "project_id": project_id,
                "kind": "handoff-resume",
                "format": "markdown",
                "language": "ko-KR",
                "locale": "en",
                "realization": realization,
            },
        )
        language_cleanup = language_host.close()
        language_ok = bool(
            language_plan_ok
            and language_plan
            and language_plan.get("outcome") == "realization_required"
            and realized_ok
            and realized_document
            and realized_document.get("outcome") == "realized"
            and realized_document.get("requested_language") == "ko-KR"
            and contains_hangul(realized_document.get("content", ""))
        )
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        language_plan = realized_document = {"error": str(error)}
        language_cleanup = {}
        language_ok = False
    viewer_snapshot = target_root / "project-understanding.html"
    viewer_result, viewer_operation = cli_json(
        recorder,
        "project-understanding-viewer-snapshot",
        cli,
        env,
        "viewer",
        "export",
        "--output",
        str(viewer_snapshot),
        "--level",
        "working",
        "--language",
        "en",
        cwd=repository,
    )
    viewer_understanding = viewer_project_understanding_evidence(
        viewer_snapshot,
        understanding if isinstance(understanding, dict) else None,
    )
    status_ok = bool(
        status_before
        and status_before.get("operation") == "project_status"
        and isinstance(status_before.get("architecture"), dict)
        and isinstance(status_before.get("evidence"), dict)
    )
    understanding_architecture = (
        understanding.get("architecture") if isinstance(understanding, dict) else None
    )
    understanding_shape_ok = bool(
        isinstance(understanding_architecture, dict)
        and isinstance(understanding_architecture.get("components"), list)
        and isinstance(understanding_architecture.get("relationships"), list)
        and isinstance(understanding_architecture.get("gaps"), list)
    )
    steps["source_grounded_understanding"] = step(
        "passed"
        if understanding_ok
        and understanding
        and understanding_shape_ok
        and status_ok
        and viewer_result
        and viewer_snapshot.is_file()
        and viewer_understanding["status"] == "passed"
        and language_ok
        else "failed",
        "task-oriented CLI status, MCP understanding, and the Viewer snapshot exposed grounded Project Understanding",
        cli_status=status_before,
        cli_status_operation=status_before_op,
        mcp_result=understanding,
        cleanup=understanding_cleanup,
        viewer_result=viewer_result,
        viewer_operation=viewer_operation,
        viewer_understanding=viewer_understanding,
        requested_language_plan=language_plan,
        requested_language_realization=realized_document,
        requested_language_cleanup=language_cleanup,
    )

    try:
        materiality_semantics = qualify_materiality_scenarios(mcp_binary, env, target_root / "materiality-scenarios")
    except (OSError, RuntimeError, ValueError) as error:
        materiality_semantics = {"status": "failed", "error": str(error)}

    candidate_tools = {
        "candidate_inspect", "candidate_manage", "inquiry_frontier", "decision_record",
        "canonical_inspect", "context_record", "repository_analyze",
        "engineering_choice_discovery", "materiality_review", "learning_deliberation",
        "checkpoint_record",
    }
    candidate_evidence: dict[str, Any] = {}
    inquiry_evidence: dict[str, Any] = {}
    candidate_status = "failed"
    inquiry_status = "failed"
    goal = candidate_analysis = decision_record = None
    decision_id = None
    decision_revision = None
    decision_source_id = None
    ordinary = repository / "v11-ordinary-work.txt"
    ordinary_evidence: dict[str, Any] = {}
    ordinary_status = "failed"
    checkpoint_value: dict[str, Any] | None = None
    checkpoint_evidence: dict[str, Any] = {}
    checkpoint_status = "failed"
    checkpoint_next_step = f"Resume the {target_kind} V11 journey in a new session"
    checkpoint_target = "next Codex session"
    try:
        host = Mcp(mcp_binary, env)
        catalog = host.initialize()
        catalog_names = {tool["name"] for tool in catalog}
        missing_candidate_tools = sorted(candidate_tools - catalog_names)
        if missing_candidate_tools:
            candidate_status = inquiry_status = "unsupported"
            candidate_evidence = {"missing_public_tools": missing_candidate_tools}
            inquiry_evidence = {"missing_public_tools": missing_candidate_tools}
        else:
            canonical_before, canonical_before_ok = host.tool("canonical_inspect", {"project_id": project_id})
            purpose, purpose_ok = host.tool("context_record", {
                "project_id": project_id,
                "user_turn": "Keep the V11 Project understandable across its Work history",
                "role": "project_purpose",
                "statement": "Keep the V11 Project understandable across its Work history",
            }) if target_kind == "volicord" else (None, True)
            learning_active = target_kind == "small-python"
            technical_delegated = target_kind == "polyglot-medium"
            base_goal_statement = (
                f"Rehearse {target_kind} through the resolved self-guiding work-authority path"
            )
            learning_statement = "Teach me through the meaningful technical fork"
            goal_statement = (
                f"{base_goal_statement}; I delegate the internal technical state representation to you"
                if technical_delegated
                else base_goal_statement
            )
            goal_turn = (
                f"{goal_statement}; {learning_statement}"
                if learning_active
                else goal_statement
            )
            goal, goal_ok = host.tool("context_record", {
                "project_id": project_id,
                "user_turn": goal_turn,
                "role": "goal",
                "statement": goal_statement,
            })
            learning_context, learning_context_ok = (
                host.tool("context_record", {
                    "project_id": project_id,
                    "user_turn": goal_turn,
                    "role": "learning",
                    "statement": learning_statement,
                })
                if learning_active
                else (None, True)
            )
            behavioral_context_ids = (
                [learning_context.get("context_item_id")]
                if learning_context_ok and learning_context
                else []
            )
            candidate_analysis, candidate_analysis_ok = host.tool(
                "repository_analyze", {"project_id": project_id, "excluded_paths": []}
            )
            source_ids = candidate_repository_source_basis(candidate_analysis)
            source_id = source_ids[0] if source_ids else None
            materiality_dimension_id = "project-context-boundary"
            technical_dimension_id = "technical-state-representation"
            discovery_choices = [
                    {
                        "choice_id": materiality_dimension_id,
                        "summary": "Choose how this Project preserves its durable context boundary",
                        "affected_scope": ["project-context"],
                        "alternatives": [
                            {"material_decomposition": {"state": "materially_atomic", "rationale": "The maintained fixture Source bounds this alternative to its stated outcome; no subordinate product policy remains.", "residual_fork_closure": {"interaction_comparisons": [], "fixed_outcome": "The bounded fixture alternative stated consequence", "credible_implementations": ["Direct implementation preserving the consequence", "Private helper preserving the same consequence"], "remaining_material_outcomes": [], "source_basis": [source_id]}}, "alternative_id": "local", "summary": "Keep canonical context local", "technical_consequences": ["Canonical context remains locally controlled"]},
                            {"material_decomposition": {"state": "materially_atomic", "rationale": "The maintained fixture Source bounds this alternative to its stated outcome; no subordinate product policy remains.", "residual_fork_closure": {"interaction_comparisons": [], "fixed_outcome": "The bounded fixture alternative stated consequence", "credible_implementations": ["Direct implementation preserving the consequence", "Private helper preserving the same consequence"], "remaining_material_outcomes": [], "source_basis": [source_id]}}, "alternative_id": "remote", "summary": "Use provider-backed canonical context", "technical_consequences": ["Canonical behavior would depend on a separately authorized provider boundary"]},
                        ],
                        "technical_consequences": ["The outcome changes durable local versus provider-backed behavior"],
                        "source_ids": [source_id] if source_id else [],
                        "effect_categories": ["persistence_or_lifetime", "privacy_or_disclosure"],
                        "relationship": {"state": "independent"},
                        "evidence_state": "sufficient",
                    },
                    {
                        "choice_id": technical_dimension_id,
                        "summary": "Represent bounded state as ordered records or a keyed index",
                        "affected_scope": ["internal-state", "v11-ordinary-work.txt"],
                        "alternatives": [
                            {"material_decomposition": {"state": "materially_atomic", "rationale": "The maintained fixture Source bounds this alternative to its stated outcome; no subordinate product policy remains.", "residual_fork_closure": {"interaction_comparisons": [], "fixed_outcome": "The bounded fixture alternative stated consequence", "credible_implementations": ["Direct implementation preserving the consequence", "Private helper preserving the same consequence"], "remaining_material_outcomes": [], "source_basis": [source_id]}}, "alternative_id": "ordered-records", "summary": "Use ordered records", "technical_consequences": ["Simple deterministic iteration with bounded lookup"]},
                            {"material_decomposition": {"state": "materially_atomic", "rationale": "The maintained fixture Source bounds this alternative to its stated outcome; no subordinate product policy remains.", "residual_fork_closure": {"interaction_comparisons": [], "fixed_outcome": "The bounded fixture alternative stated consequence", "credible_implementations": ["Direct implementation preserving the consequence", "Private helper preserving the same consequence"], "remaining_material_outcomes": [], "source_basis": [source_id]}}, "alternative_id": "keyed-index", "summary": "Use a keyed index", "technical_consequences": ["Direct lookup with additional ordering and synchronization obligations"]},
                        ],
                        "technical_consequences": ["The representation changes invariant placement and maintenance cost"],
                        "source_ids": [source_id] if source_id else [],
                        "effect_categories": ["maintenance_or_support", "implementation_internal"],
                        "relationship": {"state": "independent"},
                        "evidence_state": "sufficient",
                    },
                ]
            decomposition_rejections = {}
            for shape in ("control-status-contract", "compatible-existing-result"):
                coarse_choices = json.loads(json.dumps(discovery_choices))
                coarse_choices[0]["alternatives"][0]["material_decomposition"] = {
                    "state": "decomposed", "choice_ids": [f"{shape}-public-representation"]
                }
                rejected, accepted = host.tool("engineering_choice_discovery", {
                    "project_id": project_id,
                    "goal_context_id": (goal or {}).get("context_item_id"),
                    "baseline_analysis_snapshot_id": (candidate_analysis or {}).get("analysis_snapshot_id"),
                    "source_operation": "bounded decomposition closure probe",
                    "summary": "A broad alternative leaves a subordinate public representation open",
                    "choices": coarse_choices,
                    "interaction_review": outside_interactions(coarse_choices, [source_id] if source_id else []), "material_boundary_review": material_boundary_review(coarse_choices, [source_id] if source_id else []),
                })
                decomposition_rejections[shape] = not accepted and "missing choice" in str((rejected or {}).get("error", ""))
            discovery, discovery_ok = host.tool("engineering_choice_discovery", {
                "project_id": project_id,
                "goal_context_id": (goal or {}).get("context_item_id"),
                "baseline_analysis_snapshot_id": (candidate_analysis or {}).get(
                    "analysis_snapshot_id"
                ),
                "source_operation": "V11 installed MCP engineering-choice discovery",
                "summary": "Discover durable context authority and an independent technical representation fork",
                "choices": discovery_choices,
                "interaction_review": outside_interactions(
                    discovery_choices, [source_id] if source_id else []
                ), "material_boundary_review": material_boundary_review(
                    discovery_choices, [source_id] if source_id else []
                ),
            })
            discovery_id = discovery.get("discovery_candidate_id") if discovery_ok and discovery else None
            learning_participation = (
                {
                    "state": "active",
                    "user_turn_source_id": (learning_context or {}).get("source_id"),
                    "verbatim_statement": learning_statement,
                }
                if learning_active
                else {"state": "inactive"}
            )
            technical_learning_value = (
                {
                    "state": "deliberation_worthy",
                    "rationale": "Invariant placement and representation costs are transferable across repositories.",
                    "consequence_significance": ["The representation determines where consistency invariants live"],
                    "transferable_principles": ["Choose representations that make invariants explicit"],
                    "non_obvious_trade_offs": ["Faster direct lookup adds ordering and synchronization obligations"],
                    "interruption_counterfactual": "Without discussion now, implementation would fix a meaningful invariant-placement strategy before the learner can compare it.",
                    "participation_scope_alignment": "The technical representation is the meaningful fork requested by the complete V11 Goal.",
                }
                if learning_active
                else {
                    "state": "routine",
                    "rationale": "Normal mode keeps the bounded internal representation agent-owned and non-interrupting.",
                }
            )
            review, review_ok = host.tool("materiality_review", {
                "action": "record",
                "project_id": project_id,
                "engineering_choice_discovery_candidate_id": discovery_id,
                "rationale": "The durable Project context boundary requires explicit user authority.",
                "behavioral_context_basis": {
                    "context_item_ids": behavioral_context_ids,
                    "completeness_rationale": (
                        "The separate Learning Context is the only behaviorally consequential non-Goal statement used by this review."
                        if learning_active
                        else "No behaviorally consequential non-Goal Context is used by this review."
                    ),
                },
                "learning_participation": learning_participation,
                "judgments": [
                    {
                        "choice_id": materiality_dimension_id,
                        "disposition": "unresolved_user_owned_outcome",
                        "materially_varying_outcomes": [
                            "whether canonical Project context remains local or depends on a provider-backed boundary"
                        ],
                        "contains_user_owned_outcome": True,
                        "user_owned_outcomes": [
                            "the durable local-versus-provider Project context policy"
                        ],
                        "ownership_rationale": "The alternatives change durable product behavior and provider dependence reserved to the user.",
                        "ownership_source_ids": [source_id],
                        "alternative_accounting": alternative_accounting(
                            materiality_dimension_id,
                            ["local", "remote"],
                            source_id,
                        ),
                        "basis_summary": "Repository evidence establishes the boundary but cannot choose it",
                        "authority_counterfactual": "The Goal can be satisfied with either Project or clone-local context; no repository fact, contract, Decision, or exact delegation selects the user-owned boundary.",
                        "learning_authority": {"state": "assessed", "independent_user_authority": True, "rationale": "Without the learning request, the fixture public policy remains user-owned and its private representation remains implementation discretion.", "source_ids": [source_id]},
                        "learning_value": {"state": "routine", "rationale": "User authority uses Inquiry even when learning is active."},
                    },
                    {
                        "choice_id": technical_dimension_id,
                        "disposition": (
                            "delegated_implementation_choice"
                            if technical_delegated
                            else "agent_owned_implementation_choice"
                        ),
                        "materially_varying_outcomes": [
                            "the private bounded-state representation and invariant placement"
                        ],
                        "contains_user_owned_outcome": False,
                        "user_owned_outcomes": [],
                        "ownership_rationale": "Both alternatives preserve the selected product behavior and vary only private implementation mechanics.",
                        "discretion_counterfactuals": [{
                "choice_id": technical_dimension_id, "alternative_id": alternative,
                "externally_observable": False,
                "observation_rationale": "The fixture preserves caller behavior; the alternatives vary private organization only.",
                "source_id": source_id,
                "source_supported_boundary": "The current fixture source fixes caller results and leaves internal record organization unconstrained.",
            } for alternative in ["ordered-records", "keyed-index"]],
                        "bounded_implementation_discretion_rationale": "Ordered records and a keyed index remain within the bounded internal representation scope.",
                        "ownership_source_ids": [source_id],
                        "alternative_accounting": alternative_accounting(
                            technical_dimension_id,
                            ["ordered-records", "keyed-index"],
                            source_id,
                        ),
                        "basis_summary": "The representation is explicitly delegated in this Goal and does not change the user-owned context boundary."
                        if technical_delegated
                        else "The representation is agent-owned and does not change the user-owned context boundary.",
                        "authority_counterfactual": "The Goal explicitly delegates the exact internal technical state representation, independently from the user-owned context boundary."
                        if technical_delegated
                        else "The alternatives change only the internal state representation and no material user-owned product outcome.",
                        **(
                            {
                                "delegation_statement": "I delegate the internal technical state representation to you",
                                "delegated_scope": [
                                    "internal-state",
                                    "v11-ordinary-work.txt",
                                ],
                            }
                            if technical_delegated
                            else {}
                        ),
                        "learning_authority": {"state": "assessed", "independent_user_authority": False, "rationale": "Without the learning request, the fixture public policy remains user-owned and its private representation remains implementation discretion.", "source_ids": [source_id]},
                        "learning_value": technical_learning_value,
                    },
                ],
            }) if discovery_id else (None, False)
            review_candidate_id = (
                review.get("review_candidate_id") if review_ok and review else None
            )
            review_readiness = review
            review_readiness_ok = review_ok
            if review_candidate_id:
                review_readiness, review_readiness_ok = host.tool("materiality_review", {
                    "action": "inspect",
                    "project_id": project_id,
                    "review_candidate_id": review_candidate_id,
                    "goal_context_id": (goal or {}).get("context_item_id"),
                    "baseline_analysis_snapshot_id": (candidate_analysis or {}).get(
                        "analysis_snapshot_id"
                    ),
                    "paths": ["v11-ordinary-work.txt"],
                    "components": [],
                    "work_contexts": ["internal-state"] if technical_delegated else [],
                    "met_revisit_triggers": [],
                    "coupled_artifact_review": coupled_artifact_review(["v11-ordinary-work.txt"]),
                })
            candidate_research_analysis, candidate_research_analysis_ok = host.tool(
                "repository_analyze", {"project_id": project_id, "excluded_paths": []}
            )
            research_source_ids = candidate_repository_source_basis(
                candidate_research_analysis
            )
            research_source_id = (
                research_source_ids[0] if research_source_ids else None
            )
            submitted, submitted_ok = host.tool("candidate_manage", {
                "action": "submit_question_from_materiality",
                "project_id": project_id,
                "review_candidate_id": review_candidate_id,
                "dimension_id": materiality_dimension_id,
                "research_state": "research_required",
                "research_state_basis": "repository structure must be inspected before asking for a user judgment",
                "retention_basis": "retain through the explicit V11 inquiry disposition",
                "bounded_summary": "Choose how this Project should preserve its local context boundary",
                "prompt": "Which context boundary should this Project use?",
                "why_now": "the integrated journey needs one material current-host Decision",
                "established_facts": ["The Project has a current local Analysis Snapshot"],
                "assumptions": ["The Project remains local-first"],
                "uncertainty": ["External augmentation may be evaluated separately"],
                "alternatives": [
                    {"key": "local", "label": "Local", "consequence": "Keep canonical context local"},
                    {"key": "remote", "label": "Remote", "consequence": "Require a separate provider boundary"},
                ],
                "recommendation_key": "local",
                "recommendation_rationale": "the accepted Project boundary is local-first",
                "trade_offs": ["Remote augmentation remains a separately authorized capability"],
                "known_limits": ["The configured external provider is intentionally unavailable"],
                "what_unlocks": ["the integrated Checkpoint and portability journey"],
                "duplicate_basis": "canonical inspection found no matching Question",
                "presentation_order": 1,
            }) if review_candidate_id else (None, False)
            candidate_id = submitted.get("candidate_id") if submitted_ok and submitted else None
            after_submission, after_submission_ok = host.tool("candidate_inspect", {"project_id": project_id})
            frontier_before, frontier_before_ok = host.tool("inquiry_frontier", {"project_id": project_id})
            insufficient, insufficient_ok = host.tool("candidate_manage", {
                "action": "attach_repository_research",
                "project_id": project_id,
                "candidate_id": candidate_id,
                "capability": "structural",
                "coverage": "current repository declarations",
                "freshness": "current",
                "source_ids": [research_source_id] if research_source_id else [],
                "evidence_assessment": "insufficient",
                "limits": ["cross-component consequences still require review"],
            }) if candidate_id else (None, False)
            premature_ready, premature_ready_ok = host.tool("candidate_manage", {
                "action": "mark_research_ready", "project_id": project_id, "candidate_id": candidate_id,
            }) if candidate_id else (None, False)
            sufficient, sufficient_ok = host.tool("candidate_manage", {
                "action": "attach_repository_research",
                "project_id": project_id,
                "candidate_id": candidate_id,
                "capability": "structural",
                "coverage": "current repository structure and explicit Project boundary",
                "freshness": "current",
                "source_ids": [research_source_id] if research_source_id else [],
                "evidence_assessment": "sufficient",
                "limits": ["runtime-only external behavior remains excluded"],
            }) if candidate_id else (None, False)
            ready, ready_ok = host.tool("candidate_manage", {
                "action": "mark_research_ready", "project_id": project_id, "candidate_id": candidate_id,
            }) if candidate_id else (None, False)
            ready_inspection, ready_inspection_ok = host.tool("candidate_inspect", {"project_id": project_id})
            promoted, promoted_ok = host.tool("candidate_manage", {
                "action": "promote_question", "project_id": project_id, "candidate_id": candidate_id,
            }) if candidate_id else (None, False)
            question_id = promoted.get("question_id") if promoted_ok and promoted else None
            promoted_inspection, promoted_inspection_ok = host.tool(
                "candidate_inspect", {"project_id": project_id}
            )
            promoted_candidate = next(
                (
                    item
                    for item in (promoted_inspection or {}).get("candidates", [])
                    if item.get("identity") == candidate_id
                ),
                None,
            )
            frontier, frontier_ok = host.tool("inquiry_frontier", {"project_id": project_id})
            questions = frontier.get("questions", []) if frontier_ok and frontier else []
            displayed = next((question for question in questions if question.get("identity") == question_id), None)
            decision, decision_ok = host.tool("decision_record", {
                "project_id": project_id,
                "question_id": question_id,
                "question_revision": displayed.get("revision") if displayed else 0,
                "presentation_receipt_id": displayed.get("presentation_receipt_id") if displayed else "",
                "alternative_key": "local",
                "work_scope": "work_item" if target_kind == "volicord" else "project_wide",
                **({"work_item_id": goal.get("context_item_id")} if target_kind == "volicord" else {}),
                "user_turn": "Choose the local Project context boundary",
                "user_rationale": "Keep canonical Project context local and authorize providers separately",
            }) if displayed else (None, False)
            canonical_after, canonical_after_ok = host.tool("canonical_inspect", {"project_id": project_id})
            decision_record = canonical_record(canonical_after, "decision", lifecycle_state="active")
            decision_id = decision_record.get("identity") if decision_record else None
            decision_revision = decision_record.get("revision") if decision_record else None
            decision_source_id = decision.get("user_response_source_id") if decision_ok and decision else None
            resolved_review, resolved_review_ok = host.tool("materiality_review", {
                "action": "revise",
                "project_id": project_id,
                "review_candidate_id": review_candidate_id,
                "rationale": "The explicit current-host Decision resolves the Project context boundary.",
                "learning_participation": learning_participation,
                "judgments": [
                    {
                        "choice_id": materiality_dimension_id,
                        "disposition": "unresolved_user_owned_outcome",
                        "resolution_decision_id": decision_id,
                        "materially_varying_outcomes": [
                            "whether canonical Project context remains local or depends on a provider-backed boundary"
                        ],
                        "contains_user_owned_outcome": True,
                        "user_owned_outcomes": [
                            "the durable local-versus-provider Project context policy"
                        ],
                        "ownership_rationale": "The alternatives change durable product behavior and provider dependence reserved to the user.",
                        "ownership_source_ids": [source_id],
                        "alternative_accounting": alternative_accounting(
                            materiality_dimension_id,
                            ["local", "remote"],
                            source_id,
                            resolution_decision_id=decision_id,
                        ),
                        "basis_summary": "The explicit current-host Decision supplies current authority",
                        "authority_counterfactual": "The current-host Decision now selects the exact context boundary that the broad Goal left unresolved.",
                        "learning_authority": {"state": "assessed", "independent_user_authority": True, "rationale": "Without the learning request, the fixture public policy remains user-owned and its private representation remains implementation discretion.", "source_ids": [source_id]},
                        "learning_value": {"state": "routine", "rationale": "The user-owned outcome remains on the canonical Inquiry path."},
                    },
                    {
                        "choice_id": technical_dimension_id,
                        "disposition": (
                            "delegated_implementation_choice"
                            if technical_delegated
                            else "agent_owned_implementation_choice"
                        ),
                        "materially_varying_outcomes": [
                            "the private bounded-state representation and invariant placement"
                        ],
                        "contains_user_owned_outcome": False,
                        "user_owned_outcomes": [],
                        "ownership_rationale": "Both alternatives preserve the selected product behavior and vary only private implementation mechanics.",
                        "discretion_counterfactuals": [{
                "choice_id": technical_dimension_id, "alternative_id": alternative,
                "externally_observable": False,
                "observation_rationale": "The fixture preserves caller behavior; the alternatives vary private organization only.",
                "source_id": source_id,
                "source_supported_boundary": "The current fixture source fixes caller results and leaves internal record organization unconstrained.",
            } for alternative in ["ordered-records", "keyed-index"]],
                        "bounded_implementation_discretion_rationale": "Ordered records and a keyed index remain within the bounded internal representation scope.",
                        "ownership_source_ids": [source_id],
                        "alternative_accounting": alternative_accounting(
                            technical_dimension_id,
                            ["ordered-records", "keyed-index"],
                            source_id,
                        ),
                        "basis_summary": "The representation remains explicitly delegated independently from the canonical Decision."
                        if technical_delegated
                        else "The representation remains agent-owned independently from the canonical Decision.",
                        "authority_counterfactual": "The Goal explicitly delegates the exact internal technical state representation, independently from the canonical Decision."
                        if technical_delegated
                        else "The alternatives still change only the internal representation and no material user-owned product outcome.",
                        **(
                            {
                                "delegation_statement": "I delegate the internal technical state representation to you",
                                "delegated_scope": [
                                    "internal-state",
                                    "v11-ordinary-work.txt",
                                ],
                            }
                            if technical_delegated
                            else {}
                        ),
                        "learning_authority": {"state": "assessed", "independent_user_authority": False, "rationale": "Without the learning request, the fixture public policy remains user-owned and its private representation remains implementation discretion.", "source_ids": [source_id]},
                        "learning_value": technical_learning_value,
                    },
                ],
            }) if review_candidate_id and decision_id else (None, False)
            resolved_review_readiness = resolved_review
            resolved_review_readiness_ok = resolved_review_ok
            if resolved_review_ok and resolved_review:
                resolved_review_readiness, resolved_review_readiness_ok = host.tool(
                    "materiality_review",
                    {
                        "action": "inspect",
                        "project_id": project_id,
                        "review_candidate_id": review_candidate_id,
                        "goal_context_id": (goal or {}).get("context_item_id"),
                        "baseline_analysis_snapshot_id": (candidate_analysis or {}).get(
                            "analysis_snapshot_id"
                        ),
                        "paths": ["v11-ordinary-work.txt"],
                        "components": [],
                        "work_contexts": ["internal-state"] if technical_delegated else [],
                        "met_revisit_triggers": [],
                        "coupled_artifact_review": coupled_artifact_review(["v11-ordinary-work.txt"]),
                    },
                )
            learning_evidence: dict[str, Any] = {"active": learning_active}
            work_ready_review = resolved_review_readiness
            work_ready_review_ok = resolved_review_readiness_ok
            if learning_active and resolved_review_ok and resolved_review:
                begun, begun_ok = host.tool("learning_deliberation", {
                    "action": "begin",
                    "project_id": project_id,
                    "review_candidate_id": review_candidate_id,
                    "dimension_id": technical_dimension_id,
                    "source_operation": "V11 installed MCP learning deliberation",
                    "problem": "Which state representation makes the invariant easiest to preserve?",
                    "established_facts": ["The state set is bounded and deterministic ordering is required"],
                })
                deliberation_id = begun.get("deliberation_candidate_id") if begun_ok and begun else None
                responded, responded_ok = host.tool("learning_deliberation", {
                    "action": "respond_select",
                    "project_id": project_id,
                    "deliberation_candidate_id": deliberation_id,
                    "user_turn": "Choose ordered records because deterministic invariant inspection matters more than direct lookup.",
                    "user_rationale": "Deterministic invariant inspection matters more than direct lookup.",
                    "selections": [{"choice_id": technical_dimension_id, "alternative_id": "ordered-records"}],
                }) if deliberation_id else (None, False)
                feedback, feedback_ok = host.tool("learning_deliberation", {
                    "action": "feedback",
                    "project_id": project_id,
                    "deliberation_candidate_id": deliberation_id,
                    "feedback": "Ordered records keep deterministic inspection explicit; bounded lookup avoids making the linear scan operationally significant.",
                    "recommendation_selections": [{"choice_id": technical_dimension_id, "alternative_id": "ordered-records"}],
                    "recommendation_rationale": "The bounded size makes direct indexing unnecessary while deterministic order supports auditing.",
                }) if responded_ok else (None, False)
                completed, completed_ok = host.tool("learning_deliberation", {
                    "action": "complete",
                    "project_id": project_id,
                    "deliberation_candidate_id": deliberation_id,
                }) if feedback_ok else (None, False)
                work_ready_review = completed
                work_ready_review_ok = completed_ok
                learning_evidence = {
                    "active": True,
                    "begin": begun,
                    "response": responded,
                    "feedback": feedback,
                    "completion": completed,
                    "ordered": bool(
                        begun_ok
                        and begun
                        and begun.get("state", {}).get("state") == "awaiting_initial_response"
                        and not begun.get("rounds")
                        and responded_ok
                        and responded
                        and responded.get("state", {}).get("state") == "awaiting_agent_feedback"
                        and feedback_ok
                        and feedback
                        and feedback.get("state", {}).get("state") == "feedback_provided"
                        and completed_ok
                        and completed
                        and completed.get("state", {}).get("state") == "completed"
                        and completed.get("canonical_decision") is False
                    ),
                }
            canonical_after_learning, canonical_after_learning_ok = host.tool(
                "canonical_inspect", {"project_id": project_id}
            )
            decision_records_after_learning = [
                record
                for record in (canonical_after_learning or {}).get("records", [])
                if record.get("kind") == "decision"
            ]
            learning_evidence["canonical_decision_count"] = len(
                decision_records_after_learning
            )
            guarded_before = sha256(runtime / "guarded.sqlite3")
            if (
                work_ready_review_ok
                and work_ready_review
                and work_ready_review.get("workflow", {}).get("stage") == "ready_for_work"
                and work_ready_review.get("workflow", {}).get("blocks_ordinary_work") is False
            ):
                ordinary.write_text(
                    "ordinary repository work requires no Guarded confirmation\n",
                    encoding="utf-8",
                )
            guarded_after = sha256(runtime / "guarded.sqlite3")
            ordinary_status = (
                "passed"
                if ordinary.is_file() and guarded_before == guarded_after
                else "failed"
            )
            ordinary_evidence = {
                "changed_path": str(ordinary.relative_to(repository)),
                "guarded_store_unchanged": guarded_before == guarded_after,
                "resolved_materiality_review": resolved_review,
                "resolved_materiality_review_readiness": resolved_review_readiness,
                "learning_deliberation": learning_evidence,
            }
            preservation_rejected, preservation_accepted = host.tool("checkpoint_record", {
                "project_id": project_id,
                "goal_context_id": (goal or {}).get("context_item_id"),
                "baseline_analysis_snapshot_id": (candidate_analysis or {}).get("analysis_snapshot_id"),
                "kind": "completion", "work_state": "completed",
                "applied_decision_ids": [decision_id] if decision_id else [],
                "work_contexts": ["internal-state"] if technical_delegated else [],
                "verification": [{"state": "not_run"}],
                "verification_basis": {
                    "state": "behavior_preserving",
                    "preservation_rationale": "Completion must exercise the repository-relevant contract",
                    "surfaces": [{
                        "surface_id": "ordinary-work-contract",
                        "inspected_paths": [str(ordinary.relative_to(repository))],
                        "preserved_contract": "The existing ordinary-work file contract is preserved",
                        "verification_indices": [0],
                        "coverage_rationale": "This deliberate negative probe has no executed coverage",
                    }],
                },
                "next_step": checkpoint_next_step,
            }) if ordinary_status == "passed" else (None, False)
            preservation_rejection_ok = not preservation_accepted and "adequate focused verification" in str((preservation_rejected or {}).get("error", ""))
            checkpoint_value, checkpoint_ok = host.tool("checkpoint_record", {"verification_basis": {"state": "ordinary_change"},
                "project_id": project_id,
                "goal_context_id": (goal or {}).get("context_item_id"),
                "baseline_analysis_snapshot_id": (candidate_analysis or {}).get(
                    "analysis_snapshot_id"
                ),
                "kind": "handoff",
                "work_state": "paused",
                "applied_decision_ids": [decision_id] if decision_id else [],
                "work_contexts": ["internal-state"] if technical_delegated else [],
                "verification": [{"state": "not_run"}],
                "next_step": checkpoint_next_step,
                "known_limits": [
                    "The configured external provider is intentionally unavailable"
                ],
                "handoff_to": checkpoint_target,
            }) if ordinary_status == "passed" else (None, False)
            checkpoint_status = (
                "passed"
                if checkpoint_ok
                and preservation_rejection_ok
                and checkpoint_value
                and checkpoint_value.get("workflow", {}).get("disposition")
                == "checkpoint_recorded"
                and checkpoint_value.get("changed_paths")
                == [str(ordinary.relative_to(repository))]
                else "failed"
            )
            checkpoint_evidence = {
                "goal": goal,
                "baseline": candidate_analysis,
                "materiality_review": review,
                "materiality_review_readiness": review_readiness,
                "resolved_materiality_review": resolved_review,
                "resolved_materiality_review_readiness": resolved_review_readiness,
                "decision_source_id": decision_source_id,
                "handoff_target": checkpoint_target,
                "checkpoint": checkpoint_value,
                "mcp_call_succeeded": checkpoint_ok,
                "untested_preservation_rejected": preservation_rejection_ok,
            }
            submitted_candidate = next(
                (item for item in (after_submission or {}).get("candidates", []) if item.get("identity") == candidate_id),
                None,
            )
            ready_candidate = next(
                (item for item in (ready_inspection or {}).get("candidates", []) if item.get("identity") == candidate_id),
                None,
            )
            candidate_ok = all([
                canonical_before_ok,
                purpose_ok,
                goal_ok,
                goal,
                not learning_active
                or (
                    learning_context_ok
                    and learning_context
                    and learning_context.get("context_item_id")
                    in behavioral_context_ids
                ),
                (goal or {}).get("workflow", {}).get("stage")
                == "repository_baseline",
                candidate_analysis_ok,
                source_id,
                all(decomposition_rejections.values()),
                discovery_ok,
                discovery_id,
                discovery
                and discovery.get("workflow", {}).get("stage")
                == "materiality_review",
                review_ok,
                review,
                review_readiness_ok,
                review_readiness,
                (review_readiness or {}).get("workflow", {}).get("stage")
                == "question_candidate",
                (review_readiness or {}).get("workflow", {}).get("required_next_action")
                == {
                    "tool": "candidate_manage",
                    "action": "submit_question_from_materiality",
                },
                candidate_research_analysis_ok,
                research_source_id,
                submitted_ok,
                submitted and submitted.get("research_state") == "research_required",
                submitted and submitted.get("review_candidate_id") == review_candidate_id,
                submitted and submitted.get("dimension_id") == materiality_dimension_id,
                after_submission_ok, submitted_candidate and submitted_candidate.get("research_state") == "research_required",
                frontier_before_ok, not (frontier_before or {}).get("questions"),
                insufficient_ok,
                insufficient and insufficient.get("research_state") == "research_required",
                not premature_ready_ok, sufficient_ok,
                sufficient and sufficient.get("research_state") == "research_required",
                ready_ok, ready and ready.get("research_state") == "ready_to_ask",
                ready_inspection_ok, ready_candidate and ready_candidate.get("research_state") == "ready_to_ask",
                promoted_ok, question_id,
                promoted_inspection_ok,
                promoted_candidate and promoted_candidate.get("disposition", {}).get("state") == "promoted",
                promoted_candidate and promoted_candidate.get("promotion_target") == question_id,
            ])
            inquiry_ok = all([
                frontier_ok, displayed, decision_ok, decision and decision.get("all_succeeded") is True,
                canonical_after_ok, decision_id, decision_revision, decision_source_id,
                resolved_review_ok,
                resolved_review,
                resolved_review_readiness_ok,
                resolved_review_readiness,
                (resolved_review_readiness or {}).get("workflow", {}).get("stage")
                == ("learning_deliberation" if learning_active else "ready_for_work"),
                (not learning_active or learning_evidence.get("ordered") is True),
                work_ready_review_ok,
                work_ready_review,
                (work_ready_review or {}).get("workflow", {}).get("stage")
                == "ready_for_work",
                canonical_after_learning_ok,
                len(decision_records_after_learning) == 1,
            ])
            candidate_status = "passed" if candidate_ok else "failed"
            inquiry_status = "passed" if inquiry_ok and materiality_semantics["status"] == "passed" else "failed"
            candidate_evidence = {
                "repository_analysis": candidate_analysis,
                "repository_source_id": source_id,
                "engineering_choice_discovery": discovery,
                "material_decomposition_rejections": decomposition_rejections,
                "candidate_research_analysis": candidate_research_analysis,
                "candidate_research_source_id": research_source_id,
                "goal": goal,
                "project_purpose": purpose,
                "learning_context": learning_context,
                "materiality_review": review,
                "materiality_review_readiness": review_readiness,
                "submission": submitted,
                "inspection_after_submission": submitted_candidate,
                "frontier_after_submission": frontier_before,
                "insufficient_research": insufficient,
                "premature_ready_transition": premature_ready,
                "sufficient_research": sufficient,
                "ready_transition": ready,
                "ready_inspection": ready_candidate,
                "promotion": promoted,
                "promoted_disposition": promoted_candidate,
            }
            inquiry_evidence = {
                "materiality_semantics": materiality_semantics,
                "frontier": frontier,
                "displayed_question": displayed,
                "decision": decision,
                "canonical_decision": decision_record,
                "resolved_materiality_review": resolved_review,
                "resolved_materiality_review_readiness": resolved_review_readiness,
                "learning_deliberation": learning_evidence,
                "canonical_after_learning": canonical_after_learning,
            }
        candidate_cleanup = host.close()
        candidate_evidence["cleanup"] = candidate_cleanup
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        candidate_evidence = {"error": str(error), **candidate_evidence}
        inquiry_evidence = {"error": str(error), **inquiry_evidence}
    steps["candidate_boundary"] = step(
        candidate_status,
        "research-required Candidate stayed off the frontier until source-grounded research, readiness, inspection, and explicit promotion completed",
        **candidate_evidence,
    )
    steps["inquiry_decision"] = step(
        inquiry_status,
        "the promoted frontier Question received one explicit current-host response and produced an inspectable Decision",
        **inquiry_evidence,
    )

    steps["ordinary_work"] = step(
        ordinary_status,
        "ordinary repository work completed without Guarded ceremony",
        **ordinary_evidence,
    )

    provider_source_path = PROVIDER_SOURCE_PATHS[target_kind]
    provider_opt_in_source, provider_source_op = cli_json(
        recorder, "provider-opt-in-source", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "cli", "--session", "cli", "--text",
        "Enable the bounded V11 background semantic provider scope",
    )
    provider_opt_in, provider_opt_in_op = (None, {"exit_code": None})
    if provider_opt_in_source:
        provider_opt_in, provider_opt_in_op = cli_json(
            recorder, "provider-opt-in", cli, env, "--project", project_id,
            "privacy", "enable", "v11-unavailable-provider", "v11-model",
            "--source", provider_opt_in_source["identity"], "--scope", provider_source_path,
        )
    guarded_status = "failed"
    provider_status = "failed"
    guarded_evidence: dict[str, Any] = {
        "opt_in_source": provider_opt_in_source,
        "opt_in_source_operation": provider_source_op,
        "opt_in": provider_opt_in,
        "opt_in_operation": provider_opt_in_op,
    }
    provider_evidence: dict[str, Any] = {}
    required_provider_tools = {"background_semantic_operation", "guarded_interaction", "canonical_inspect", "repository_analyze"}
    try:
        provider_host = Mcp(mcp_binary, env)
        provider_catalog = provider_host.initialize()
        provider_tool_names = {tool["name"] for tool in provider_catalog}
        missing_provider_tools = sorted(required_provider_tools - provider_tool_names)
        if missing_provider_tools:
            guarded_status = provider_status = "unsupported"
            guarded_evidence["missing_public_tools"] = missing_provider_tools
            provider_evidence["missing_public_tools"] = missing_provider_tools
        elif provider_opt_in:
            expiration = time.time_ns() // 1_000 + 600_000_000
            prepare_arguments = {
                "action": "prepare",
                "project_id": project_id,
                "provider": "v11-unavailable-provider",
                "model": "v11-model",
                "purpose": "background semantic analysis",
                "requested_capability": "semantic",
                "source_paths": [provider_source_path],
                "expiration_unix_micros": expiration,
            }
            denied_preparation, denied_preparation_ok = provider_host.tool(
                "background_semantic_operation", prepare_arguments
            )
            denied_request = (denied_preparation or {}).get("guarded_request", {})
            denied, denied_ok = provider_host.tool("guarded_interaction", {
                "confirmation_request_id": denied_request.get("confirmation_request_id"),
                "request_revision": denied_request.get("request_revision"),
                "effect_fingerprint": denied_request.get("effect_fingerprint"),
                "decision": "deny",
                "user_turn": "Deny this exact V11 provider transmission",
            }) if denied_request else (None, False)
            denied_dispatch, denied_dispatch_ok = provider_host.tool("background_semantic_operation", {
                "action": "dispatch",
                "confirmation_request_id": denied_request.get("confirmation_request_id"),
                "request_revision": denied_request.get("request_revision"),
                "effect_fingerprint": denied_request.get("effect_fingerprint"),
            }) if denied_request else (None, False)

            prepared, prepared_ok = provider_host.tool("background_semantic_operation", prepare_arguments)
            request = (prepared or {}).get("guarded_request", {})
            provider_request = (prepared or {}).get("provider_request", {})
            inspected_request, inspected_request_ok = provider_host.tool("guarded_interaction", {
                "confirmation_request_id": request.get("confirmation_request_id"),
            }) if request else (None, False)
            mismatched, mismatched_ok = provider_host.tool("background_semantic_operation", {
                "action": "dispatch",
                "confirmation_request_id": request.get("confirmation_request_id"),
                "request_revision": request.get("request_revision"),
                "effect_fingerprint": "sha256:" + "0" * 64,
            }) if request else (None, False)
            missing, missing_ok = provider_host.tool("background_semantic_operation", {
                "action": "dispatch",
                "confirmation_request_id": request.get("confirmation_request_id"),
                "request_revision": request.get("request_revision"),
                "effect_fingerprint": request.get("effect_fingerprint"),
            }) if request else (None, False)
            confirmed, confirmed_ok = provider_host.tool("guarded_interaction", {
                "confirmation_request_id": request.get("confirmation_request_id"),
                "request_revision": request.get("request_revision"),
                "effect_fingerprint": request.get("effect_fingerprint"),
                "decision": "confirm",
                "user_turn": "Confirm this exact filtered V11 provider transmission",
            }) if request else (None, False)
            dispatched, dispatched_ok = provider_host.tool("background_semantic_operation", {
                "action": "dispatch",
                "confirmation_request_id": request.get("confirmation_request_id"),
                "request_revision": request.get("request_revision"),
                "effect_fingerprint": request.get("effect_fingerprint"),
            }) if request else (None, False)
            durable, durable_ok = provider_host.tool("background_semantic_operation", {
                "action": "inspect",
                "project_id": project_id,
                "operation_id": (dispatched or {}).get("operation_id"),
                "provider_request_id": provider_request.get("provider_request_id"),
            }) if dispatched_ok and dispatched else (None, False)
            reused, reused_ok = provider_host.tool("background_semantic_operation", {
                "action": "dispatch",
                "confirmation_request_id": request.get("confirmation_request_id"),
                "request_revision": request.get("request_revision"),
                "effect_fingerprint": request.get("effect_fingerprint"),
            }) if request else (None, False)
            local_canonical, local_canonical_ok = provider_host.tool("canonical_inspect", {"project_id": project_id})
            local_structural, local_structural_ok = provider_host.tool(
                "repository_analyze", {"project_id": project_id, "excluded_paths": []}
            )
            provider_request_after = (durable or {}).get("provider_request", {})
            manifest = provider_request_after.get("manifest", [])
            guarded_ok = all([
                denied_preparation_ok,
                denied_preparation and denied_preparation.get("state") == "awaiting_exact_confirmation",
                denied_ok, denied and denied.get("decision") == "denied",
                not denied_dispatch_ok,
                prepared_ok, prepared and prepared.get("state") == "awaiting_exact_confirmation",
                prepared and prepared.get("dispatch_occurred") is False,
                inspected_request_ok,
                inspected_request and inspected_request.get("effect_fingerprint") == request.get("effect_fingerprint"),
                mismatched_ok,
                mismatched and mismatched.get("guarded_outcome", {}).get("rejection") == "mismatched",
                mismatched and mismatched.get("provider_request", {}).get("outcome") == "prepared",
                missing_ok,
                missing and missing.get("guarded_outcome", {}).get("rejection") == "missing",
                missing and missing.get("provider_request", {}).get("outcome") == "prepared",
                confirmed_ok, confirmed and confirmed.get("decision") == "confirmed",
                dispatched_ok,
                dispatched and dispatched.get("guarded_outcome", {}).get("confirmation_consumed") is True,
                durable_ok, durable == dispatched,
                not reused_ok,
                reused and "live provider preparation is unavailable" in reused.get("error", ""),
            ])
            provider_ok = all([
                durable_ok,
                provider_request_after.get("outcome") == "provider_unavailable",
                manifest,
                all(entry.get("transmission_outcome") == "not_transmitted" for entry in manifest),
                local_canonical_ok,
                local_structural_ok,
                local_structural and local_structural.get("state") in {"succeeded", "partial"},
            ])
            guarded_status = "passed" if guarded_ok else "failed"
            provider_status = "passed" if provider_ok else "failed"
            guarded_evidence.update({
                "denied_preparation": denied_preparation,
                "denial": denied,
                "dispatch_after_denial": denied_dispatch,
                "preparation": prepared,
                "exact_inspection": inspected_request,
                "mismatched_dispatch": mismatched,
                "missing_confirmation_dispatch": missing,
                "confirmation": confirmed,
                "dispatch_attempt": dispatched,
                "durable_inspection": durable,
                "reuse_attempt": reused,
            })
            provider_evidence = {
                "durable_inspection": durable,
                "local_canonical": local_canonical,
                "local_structural": local_structural,
            }
        elif unsupported_cli(provider_opt_in_op):
            guarded_status = provider_status = "unsupported"
        provider_cleanup = provider_host.close()
        guarded_evidence["cleanup"] = provider_cleanup
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        guarded_evidence["error"] = str(error)
        provider_evidence["error"] = str(error)
    steps["guarded_boundary"] = step(
        guarded_status,
        "the public provider operation enforced exact inspection, denial cleanup, pre-confirmation rejection, confirmation consumption, durable outcome inspection, and terminal reuse rejection",
        **guarded_evidence,
    )

    steps["checkpoint"] = step(
        checkpoint_status,
        "the exact Goal, pre-work Analysis Snapshot, resolved Materiality Review, and explicit Decision grounded a Handoff Checkpoint",
        **checkpoint_evidence,
    )
    pre_restart_resolution = pre_restart_canonical = None
    expected = None
    restart_errors: list[str] = []
    try:
        pre_host = Mcp(mcp_binary, env)
        pre_host.initialize()
        pre_restart_resolution, resolved_ok = pre_host.tool(
            "project_resolve", {"repository": str(repository.resolve())}
        )
        pre_restart_canonical, canonical_ok = pre_host.tool(
            "canonical_inspect", {"project_id": project_id}
        )
        pre_cleanup = pre_host.close()
        if not resolved_ok or not canonical_ok or not pre_restart_resolution:
            raise ValueError("pre-restart Project/canonical inspection failed")
        expected = restart_recall.expected_state(
            project_id, pre_restart_resolution.get("binding", {}),
            (initialized or {}).get("binding", {}), goal_statement if goal else "",
            goal or {}, decision_record or {}, decision_source_id,
            checkpoint_value or {}, candidate_analysis or {},
            provider_evidence.get("local_structural") or {}, pre_restart_canonical,
            checkpoint_next_step,
            decision_work_scope=(
                {"kind": "work_item", "work_item_id": goal.get("context_item_id")}
                if target_kind == "volicord" else {"kind": "project_wide"}
            ),
        )
        restart_errors.extend(restart_recall.binding_errors(expected, pre_restart_resolution))
    except (OSError, RuntimeError, ValueError, TypeError, KeyError) as error:
        pre_cleanup = {}
        restart_errors.append(f"pre-restart evidence: {error}")

    recall_before, recall_op = cli_json(
        recorder, "recall", cli, env, "recall", cwd=repository
    )
    resolved_after = recall_after = continued = canonical_after_read = canonical_after_continue = None
    recall_after_ok = continue_ok = read_inspection_ok = continue_inspection_ok = False
    try:
        restarted = Mcp(mcp_binary, env)
        restarted.initialize()
        resolved_after, resolved_after_ok = restarted.tool(
            "project_resolve", {"repository": str(repository.resolve())}
        )
        recall_after, recall_after_ok = restarted.tool("recall", {"project_id": project_id})
        canonical_after_read, read_inspection_ok = restarted.tool(
            "canonical_inspect", {"project_id": project_id}
        )
        if expected and recall_after_ok and not restart_recall.recall_errors(expected, recall_after):
            continued, continue_ok = restarted.tool("context_record", {
                "project_id": project_id, "role": "goal", "work_transition": "continue",
                "goal_context_id": expected["goal_id"],
            })
            canonical_after_continue, continue_inspection_ok = restarted.tool(
                "canonical_inspect", {"project_id": project_id}
            )
        restart_cleanup = restarted.close()
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        restart_errors.append(f"fresh MCP process: {error}")
        restart_cleanup = {}
    if expected:
        restart_errors.extend(restart_recall.verify_restart(
            expected, recall_before, recall_after, resolved_after,
            canonical_after_read, continued, canonical_after_continue,
        ))
    recalled_learning = (recall_after or {}).get("learning_context", [])
    learning_recall_ok = (
        completed_learning_recalled(recalled_learning)
        if target_kind == "small-python"
        else not recalled_learning
    )
    restart_ok = bool(
        checkpoint_status == "passed" and expected and recall_before
        and recall_op.get("exit_code") == 0 and resolved_after_ok and recall_after_ok
        and continue_ok and read_inspection_ok and continue_inspection_ok
        and recall_after.get("learning_context_health", {}).get("state") == "available"
        and learning_recall_ok and not restart_errors
    )
    steps["restart_recall"] = step(
        "passed" if restart_ok else "failed",
        "CLI and fresh MCP Recall matched authored Project, Work, Checkpoint, Decision, and Source state; continue preserved canonical identity",
        expected_state=expected, errors=restart_errors,
        pre_restart_resolution=pre_restart_resolution, pre_restart_canonical=pre_restart_canonical,
        pre_restart_cleanup=pre_cleanup, cli_recall=recall_before, cli_operation=recall_op,
        restarted_resolution=resolved_after, restarted_recall=recall_after,
        canonical_after_read=canonical_after_read, continuation=continued,
        canonical_after_continue=canonical_after_continue, cleanup=restart_cleanup,
    )

    multi_work_evidence = None
    if target_kind == "volicord":
        multi_work_evidence = multi_work.rehearse(
            sys.modules[__name__], project_id=project_id, repository=repository,
            cli=cli, mcp_binary=mcp_binary, env=env, recorder=recorder,
            expected_a=expected or {}, canonical_after_a=canonical_after_continue or {},
            purpose_id=(candidate_evidence.get("project_purpose") or {}).get("context_item_id"),
        )

    base_bundle = target_root / "base.volicord.json"
    exported, export_op = cli_json(
        recorder, "bundle-export", cli, env, "context", "export", "--output", str(base_bundle), cwd=repository
    )
    clone_result = recorder.run("clone-target", ["git", "clone", "--quiet", "--no-hardlinks", str(repository), str(clone)], env)
    imported, import_op = cli_json(
        recorder, "bundle-import", cli, env, "context", "import", "--input", str(base_bundle), runtime=clone_runtime
    )
    bound, bind_op = (None, {"exit_code": None})
    if imported:
        bound, bind_op = cli_json(
            recorder, "clone-bind", cli, env, "--project", project_id,
            "--repository", str(clone), "bind", runtime=clone_runtime
        )
    portability_ok = bool(exported and clone_result["exit_code"] == 0 and imported and bound)
    portability_status = "passed" if portability_ok else (
        "unsupported" if unsupported_cli(export_op, import_op, bind_op) else "failed"
    )
    steps["portable_clone"] = step(
        portability_status, "portable bundle imported and explicitly rebound in another clone",
        export=exported, export_operation=export_op, clone_operation=clone_result,
        import_result=imported, import_operation=import_op, binding=bound, bind_operation=bind_op,
    )

    if target_kind == "volicord" and multi_work_evidence is not None:
        portable_status, portable_status_op = cli_json(
            recorder, "multi-work-portable-status", cli, env, "status",
            runtime=clone_runtime, cwd=clone,
        ) if portability_ok else (None, {"exit_code": None})
        multi_work_evidence["portable_status"] = portable_status
        multi_work_evidence["portable_status_operation"] = portable_status_op
        multi_work_evidence["checks"] = multi_work.verify_rehearsal(
            multi_work_evidence, expected or {}, portable_status)
        multi_work_evidence["status"] = (
            "passed" if all(multi_work_evidence["checks"].values()) else "failed"
        )

        proof = None
        if multi_work_evidence["status"] == "passed":
            try:
                proof = result_contract.make_lifecycle_proof(
                    multi_work_evidence, steps["restart_recall"]["evidence"]
                )
            except (KeyError, TypeError, ValueError) as error:
                multi_work_evidence["errors"].append(f"bounded proof: {error}")
                multi_work_evidence["status"] = "failed"
        steps["multi_work_continuity"] = step(
            multi_work_evidence["status"],
            "installed A to exact Resume A to B/C Work continuity, scoped rejection, and portable readback",
            checks=multi_work_evidence["checks"], proof=proof,
            errors=multi_work_evidence["errors"],
        )

    local_decision = incoming_decision = comparison = resolution = None
    source_a = source_b = None
    conflict_operations: list[dict[str, Any]] = []
    if portability_ok and decision_id:
        source_a, source_a_op = cli_json(
            recorder, "diverge-a-source", cli, env, "--project", project_id,
            "advanced", "records", "source", "--host", "codex", "--session", "clone-a",
            "--text", "Choose the remote branch in clone A",
        )
        source_b, source_b_op = cli_json(
            recorder, "diverge-b-source", cli, env, "--project", project_id,
            "advanced", "records", "source", "--host", "codex", "--session", "clone-b",
            "--text", "Retain the local branch in clone B", runtime=clone_runtime,
        )
        conflict_operations.extend([source_a_op, source_b_op])
        if source_a:
            local_decision, local_decision_op = cli_json(
                recorder, "diverge-a-decision", cli, env, "--project", project_id,
                "advanced", "records", "supersede-decision", decision_id,
                "--source", source_a["identity"], "--alternative", "remote",
                "--rationale", "Clone A chooses remote augmentation",
            )
            conflict_operations.append(local_decision_op)
        if source_b:
            incoming_decision, incoming_decision_op = cli_json(
                recorder, "diverge-b-decision", cli, env, "--project", project_id,
                "advanced", "records", "supersede-decision", decision_id,
                "--source", source_b["identity"], "--alternative", "local",
                "--rationale", "Clone B retains local context",
                runtime=clone_runtime,
            )
            conflict_operations.append(incoming_decision_op)
        bundle_a = target_root / "a.volicord.json"
        bundle_b = target_root / "b.volicord.json"
        bundle_a_value, bundle_a_op = cli_json(
            recorder, "diverge-a-export", cli, env, "--project", project_id,
            "context", "export", "--output", str(bundle_a)
        )
        bundle_b_value, bundle_b_op = cli_json(
            recorder, "diverge-b-export", cli, env, "--project", project_id,
            "context", "export", "--output", str(bundle_b),
            runtime=clone_runtime,
        )
        conflict_operations.extend([bundle_a_op, bundle_b_op])
        if bundle_a_value and bundle_b_value:
            comparison, comparison_op = cli_json(
                recorder, "divergent-compare", cli, env, "context", "compare",
                "--input", str(bundle_b), "--base", str(base_bundle),
            )
            conflict_operations.append(comparison_op)
            if comparison and source_a:
                resolution, resolution_op = cli_json(
                    recorder, "divergent-resolution", cli, env, "context", "resolve",
                    "--input", str(bundle_b), "--conflict-set", comparison["conflict_set_identity"],
                    "--revision", str(comparison["conflict_revision"]),
                    "--source", source_a["identity"], "--mode", "context-branch",
                    "--base", str(base_bundle),
                )
                conflict_operations.append(resolution_op)
    conflicts = comparison.get("conflicts", []) if comparison else []
    conflict_ok = all([
        source_a, source_b, local_decision, incoming_decision, comparison,
        comparison and comparison.get("requires_user_resolution") is True,
        any(conflict.get("class") in {"same_record_revision", "semantic_decision_conflict"} for conflict in conflicts),
        resolution,
        resolution and resolution.get("status") in {"resolved", "branched"},
        resolution and resolution.get("resolution_source_id") == source_a.get("identity") if source_a else False,
    ])
    conflict_status = "passed" if conflict_ok else (
        "unsupported" if unsupported_cli(*conflict_operations) else "failed"
    )
    steps["divergent_conflict"] = step(
        conflict_status,
        "both clones superseded the same integrated Decision, exposed the canonical conflict set, and explicitly created a context branch",
        clone_a_source=source_a, clone_b_source=source_b,
        clone_a_decision=local_decision, clone_b_decision=incoming_decision,
        comparison=comparison, resolution=resolution,
    )

    correction_authorization, correction_source_op = cli_json(
        recorder, "correction-authorization", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "codex", "--session", "v11-correction",
        "--text", "Correct the integrated Decision rationale",
    )
    corrected = None
    correction_op = {"exit_code": None}
    if correction_authorization and local_decision:
        corrected, correction_op = cli_json(
            recorder, "correct-decision", cli, env, "--project", project_id,
            "advanced", "records", "correct-decision", local_decision["identity"],
            "--revision", str(local_decision["revision"]),
            "--source", correction_authorization["identity"],
            "--text", "Remote augmentation Clone A chooses",
        )
    supersession_authorization, supersession_source_op = cli_json(
        recorder, "supersession-authorization", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "codex", "--session", "v11-supersession",
        "--text", "Return the integrated Decision to the local boundary",
    )
    superseded = None
    supersession_op = {"exit_code": None}
    if supersession_authorization and local_decision and corrected:
        superseded, supersession_op = cli_json(
            recorder, "supersede-corrected-decision", cli, env, "--project", project_id,
            "advanced", "records", "supersede-decision", local_decision["identity"],
            "--source", supersession_authorization["identity"], "--alternative", "local",
            "--rationale", "Keep canonical context local after evaluating the explicit provider boundary",
        )
    deletion_authorization, deletion_source_op = cli_json(
        recorder, "deletion-authorization", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "codex", "--session", "v11",
        "--text", "Authorize deletion of the disposable V11 Source",
    )
    disposable_source, disposable_source_op = cli_json(
        recorder, "disposable-source", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "codex", "--session", "v11",
        "--text", "Disposable Source created by the integrated V11 journey",
    )
    cleanup_control_source, cleanup_control_source_op = cli_json(
        recorder, "cleanup-control-source", cli, env, "--project", project_id,
        "advanced", "records", "source", "--host", "codex", "--session", "v11",
        "--text", "Unrelated Source that must survive the V11 forgetting journey",
    )
    cleanup_control_state = target_root / "forgetting-control-state.json"
    cleanup_control_result = target_root / "forgetting-control-result.json"
    fixture_control_command = [
        "cargo", "test", "--manifest-path", "rebuild/Cargo.toml", "-p", "volicord-operations",
        "--test", "v11_fixture_control", "--all-features", "--", "--exact",
        "seed_and_inspect_v11_forgetting_control", "--nocapture",
    ]
    if disposable_source and cleanup_control_source:
        fixture_env = env | {
            "VOLICORD_V11_RUNTIME": str(runtime),
            "VOLICORD_V11_PROJECT": project_id,
            "VOLICORD_V11_RELATED_SOURCE": disposable_source["identity"],
            "VOLICORD_V11_UNRELATED_SOURCE": cleanup_control_source["identity"],
            "VOLICORD_V11_FIXTURE_ACTION": "seed",
            "VOLICORD_V11_CONTROL_OUTPUT": str(cleanup_control_state),
        }
        cleanup_seed_op = recorder.run(
            "seed-forgetting-controls", fixture_control_command, fixture_env
        )
    else:
        fixture_env = env
        cleanup_seed_op = {"exit_code": None}
    deletion = None
    if deletion_authorization and disposable_source and cleanup_seed_op.get("exit_code") == 0:
        deletion, deletion_op = cli_json(
            recorder, "forget-source", cli, env, "--project", project_id,
            "advanced", "records", "forget", "source", disposable_source["identity"],
            "--source", deletion_authorization["identity"],
        )
    else:
        deletion_op = {"exit_code": None}
    canonical_after_mutations, canonical_after_mutations_op = cli_json(
        recorder, "canonical-after-mutations", cli, env,
        "--project", project_id, "advanced", "records", "list"
    )
    if deletion and cleanup_control_state.is_file():
        cleanup_inspect_op = recorder.run(
            "inspect-forgetting-controls",
            fixture_control_command,
            fixture_env | {
                "VOLICORD_V11_FIXTURE_ACTION": "inspect",
                "VOLICORD_V11_CONTROL_STATE": str(cleanup_control_state),
                "VOLICORD_V11_CONTROL_OUTPUT": str(cleanup_control_result),
            },
        )
        cleanup_control = (
            json.loads(cleanup_control_result.read_text(encoding="utf-8"))
            if cleanup_inspect_op["exit_code"] == 0 and cleanup_control_result.is_file()
            else None
        )
    else:
        cleanup_inspect_op = {"exit_code": None}
        cleanup_control = None
    records_after_mutations = canonical_after_mutations.get("records", []) if canonical_after_mutations else []
    mutation_operations = [
        correction_source_op, correction_op, supersession_source_op, supersession_op,
        deletion_source_op, disposable_source_op, cleanup_control_source_op, cleanup_seed_op,
        deletion_op, canonical_after_mutations_op, cleanup_inspect_op,
    ]
    mutations_ok = all([
        corrected,
        corrected and corrected.get("identity") == local_decision.get("identity") if local_decision else False,
        corrected and corrected.get("revision") == 2,
        superseded,
        superseded and superseded.get("identity") != local_decision.get("identity") if local_decision else False,
        deletion,
        deletion and deletion.get("state") == "completed",
        deletion and deletion.get("candidate_cleanup_completed") is True,
        deletion and deletion.get("managed_derived_cleanup_completed") is True,
        deletion and deletion.get("residue_verified") is True,
        canonical_after_mutations,
        disposable_source and all(record.get("identity") != disposable_source.get("identity") for record in records_after_mutations),
        cleanup_control_source and any(record.get("identity") == cleanup_control_source.get("identity") for record in records_after_mutations),
        cleanup_control and cleanup_control.get("related_candidate_absent") is True,
        cleanup_control and cleanup_control.get("related_derived_absent") is True,
        cleanup_control and cleanup_control.get("unrelated_candidate_present") is True,
        cleanup_control and cleanup_control.get("unrelated_derived_present") is True,
        canonical_record(canonical_after_mutations, "decision", lifecycle_state="active") is not None,
    ])
    mutation_status = "passed" if mutations_ok else (
        "unsupported" if unsupported_cli(*mutation_operations) else "failed"
    )
    steps["correction_supersession_deletion"] = step(
        mutation_status,
        "the integrated Decision was corrected and superseded, and public forgetting removed linked Candidate/managed Derived controls while preserving unrelated controls after restart",
        correction_authorization=correction_authorization, correction=corrected,
        supersession_authorization=supersession_authorization, supersession=superseded,
        deletion_authorization=deletion_authorization, disposable_source=disposable_source,
        unrelated_control_source=cleanup_control_source, cleanup_seed_operation=cleanup_seed_op,
        deletion=deletion, canonical_after=canonical_after_mutations,
        cleanup_restart_inspection=cleanup_control, cleanup_inspection_operation=cleanup_inspect_op,
    )

    canonical_before_docs = target_root / "before-documents.json"
    cli_json(
        recorder, "docs-before-bundle", cli, env, "context", "export", "--output", str(canonical_before_docs), cwd=repository
    )
    document_results = []
    for kind in DOCUMENT_KINDS:
        for format_name, suffix in (("markdown", "md"), ("html", "html")):
            destination = target_root / "documents" / f"{kind}.{suffix}"
            destination.parent.mkdir(parents=True, exist_ok=True)
            value, operation = cli_json(
                recorder, f"document-{kind}-{format_name}", cli, env,
                "document", "export", kind, "--format", format_name,
                "--output", str(destination), "--language", "en", cwd=repository,
            )
            document_results.append({"kind": kind, "format": format_name, "result": value, "operation": operation})
    canonical_after_docs = target_root / "after-documents.json"
    cli_json(
        recorder, "docs-after-bundle", cli, env, "context", "export", "--output", str(canonical_after_docs), cwd=repository
    )
    published_documents = all(
        item["result"]
        and item["result"].get("outcome") != "unavailable"
        and Path(item["result"]["destination"]).is_file()
        and Path(item["result"]["destination"]).stat().st_size > 0
        for item in document_results
    )
    docs_ok = (
        published_documents
        and language_ok
        and canonical_before_docs.read_bytes() == canonical_after_docs.read_bytes()
    )
    steps["document_outputs"] = step(
        "passed" if docs_ok else "failed", "all four Markdown and self-contained HTML outputs were published, and the current host realized the requested Korean body without canonical mutation",
        documents=document_results,
        published_documents=published_documents,
        requested_language_body_realized=language_ok,
        canonical_unchanged=canonical_before_docs.read_bytes() == canonical_after_docs.read_bytes(),
    )

    privacy, privacy_op = cli_json(
        recorder, "privacy-status", cli, env, "privacy", "status", cwd=repository
    )
    provider_evidence.update({"privacy": privacy, "privacy_operation": privacy_op})
    steps["provider_failure"] = step(
        provider_status,
        "the configured production adapter reported provider_unavailable without transmission while canonical inspection and local structural analysis remained usable",
        **provider_evidence,
    )

    malformed = repository / ("src/v11_broken.rs" if target_kind == "volicord" else "v11_broken.py")
    malformed.parent.mkdir(parents=True, exist_ok=True)
    malformed.write_text("fn broken( {\n" if malformed.suffix == ".rs" else "def broken(:\n", encoding="utf-8")
    parser_result, parser_op = cli_json(
        recorder, "parser-degradation", cli, env, "analyze", cwd=repository
    )
    parser_status = parser_degradation_status(parser_result)
    steps["parser_failure"] = step(
        parser_status, "malformed language area was analyzed and required scoped failure/partial reporting",
        result=parser_result, operation=parser_op,
    )

    stored_at = Path(parser_result["stored_at"]) if parser_result and parser_result.get("stored_at") else None
    recall_pre_recovery, _ = cli_json(
        recorder, "recovery-recall-before", cli, env, "recall", cwd=repository
    )
    prior_capabilities: dict[str, Any] = {}
    repaired_capabilities: dict[str, Any] = {}
    if stored_at and stored_at.is_file():
        try:
            prior_capabilities = read_analysis_capabilities(
                stored_at, parser_result["analysis_snapshot"], project_id
            )
        except (OSError, ValueError) as error:
            prior_capabilities = {"error": str(error)}
        stored_at.write_bytes(b"{ controlled V11 derived corruption")
        degraded_health, health_op = cli_json(
            recorder, "corrupt-health", cli, env, "doctor", "check", cwd=repository
        )
        repaired, repair_op = cli_json(
            recorder, "derived-repair", cli, env, "doctor", "repair", cwd=repository
        )
        recall_post_recovery, _ = cli_json(
            recorder, "recovery-recall-after", cli, env, "recall", cwd=repository
        )
        try:
            repaired_capabilities = read_analysis_capabilities(
                Path(repaired["stored_at"]), repaired["analysis_snapshot"], project_id
            ) if repaired and repaired.get("stored_at") else {}
        except (OSError, ValueError) as error:
            repaired_capabilities = {"error": str(error)}
        recovery_checks = recovery_recall_checks(
            recall_pre_recovery, recall_post_recovery, parser_result.get("analysis_snapshot"), repaired,
            (prior_capabilities, repaired_capabilities),
        )
        recovery_ok = (
            degraded_health and degraded_health.get("state") == "degraded" and repaired and
            repaired.get("state") in {"succeeded", "partial"} and all(recovery_checks.values())
        )
    else:
        degraded_health = repaired = recall_post_recovery = None
        health_op = repair_op = {"exit_code": None}
        recovery_checks = recovery_recall_checks(None, None, None, None)
        recovery_ok = False
    steps["derived_index_recovery"] = step(
        "passed" if recovery_ok else "failed", "controlled derived corruption was rebuilt while preserving canonical Recall and refreshing analysis provenance",
        degraded_health=degraded_health, health_operation=health_op, repair=repaired,
        repair_operation=repair_op,
        checks=recovery_checks,
        capability_evidence_errors=[value["error"] for value in (
            prior_capabilities, repaired_capabilities,
        ) if "error" in value],
        canonical_recall_meaning_unchanged=recovery_checks["canonical_recall_meaning_unchanged"],
        repository_source_refreshed=recovery_checks["repository_source_refreshed"],
    )

    try:
        candidate_failure_ok, candidate_failure_evidence = qualify_candidate_dependency_failures(
            recorder, cli, mcp_binary, env, runtime, project_id
        )
    except (OSError, RuntimeError, ValueError, sqlite3.Error, json.JSONDecodeError) as error:
        candidate_failure_ok = False
        candidate_failure_evidence = [{"status": "failed", "error": str(error)}]
    steps["candidate_boundary"]["evidence"]["dependency_failure_qualification"] = (
        candidate_failure_evidence
    )
    if not candidate_failure_ok:
        steps["candidate_boundary"]["status"] = "failed"
        steps["candidate_boundary"]["summary"] = (
            "Candidate lifecycle passed, but dependency failure honesty qualification failed"
        )

    legacy_after = (legacy_sentinel.stat().st_mtime_ns, sha256(legacy_sentinel))
    return {
        "class": target_kind,
        "identity": identity,
        "project_id": project_id,
        "legacy_runtime_untouched": legacy_before == legacy_after,
        **({"multi_work_rehearsal": multi_work_evidence} if target_kind == "volicord" else {}),
        "steps": steps,
    }


def authenticated_target_passed(repository: dict[str, Any]) -> bool:
    """Replay the bounded probe evidence for this repository's exact Project."""
    try:
        project_id = repository["project_id"]
        if not isinstance(project_id, str) or not project_id:
            return False
        if repository["steps"]["project_binding"]["evidence"]["project_id"] != project_id:
            return False
        authenticated = repository["steps"]["codex_mcp_connection"]["evidence"]["authenticated"]
        operation = authenticated["evidence"]["operation"]
        if (authenticated["status"] != "passed" or type(operation.get("exit_code")) is not int
            or operation["exit_code"] != 0 or operation.get("outcome") != "succeeded"
            or operation.get("termination") is not None or operation.get("spawn_error") is not None):
            return False
        proof = validate_codex_probe(decoded(operation), project_id)
        return proof == authenticated["evidence"]["probe"]
    except (KeyError, TypeError, ValueError, OSError):
        return False


def v11_readiness(
    repositories: list[dict[str, Any]], assessment: dict[str, Any], performance: dict[str, Any] | None
) -> bool:
    return bool(
        len(repositories) == 3
        and [repository.get("class") for repository in repositories]
        == ["volicord", "small-python", "polyglot-medium"]
        and all(set(repository.get("steps", {})) == set(
            result_contract.required_steps_for_target(repository["class"]))
                and all(value.get("status") == "passed" for value in repository["steps"].values())
                and authenticated_target_passed(repository) for repository in repositories)
        and assessment.get("decision_revisit_trigger_assessment") == OFFICIAL_REVISIT_ASSESSMENT
        and assessment.get("active_decision_revisit_triggers") == []
        and performance_module.accepted(performance)
    )


def qualification_passed(qualification: Any, candidate_head: str, final_artifact: str) -> bool:
    if not isinstance(qualification, dict):
        return False
    return bool(
        qualification.get("status") == "passed"
        and isinstance(qualification.get("gate_invocation"), str) and qualification["gate_invocation"]
        and re.fullmatch(r"[0-9a-f]{64}", str(qualification.get("final_sha256")))
        and qualification.get("final_artifact") == final_artifact
        and all(isinstance(qualification.get(boundary), dict)
                and qualification[boundary].get("status") == "passed"
                and qualification[boundary].get("head") == candidate_head
                and qualification[boundary].get("worktree_clean") is True
                and qualification[boundary].get("same_gate_final_valid") is True
                for boundary in ("start", "end"))
    )


def validate_result(result: dict[str, Any]) -> None:
    if (result.get("schema_version") != result_contract.SCHEMA_VERSION
        or result.get("technical_contract") != result_contract.TECHNICAL_CONTRACT
        or result.get("workload_identity") != result_contract.WORKLOAD_IDENTITY
        or result.get("required_by_target") != result_contract.required_counts()
        or result.get("validation_id") != "V11"):
        raise AssertionError("result must use the maintained V11 schema")
    if not re.fullmatch(r"[0-9a-f]{40}", str(result.get("validated_production_head"))):
        raise AssertionError("V11 result has invalid exact candidate identity")
    repositories = result.get("repositories")
    if not isinstance(repositories, list) or len(repositories) != 3:
        raise AssertionError("V11 result must contain three repositories")
    if [repository.get("class") for repository in repositories] != [
        "volicord", "small-python", "polyglot-medium"
    ]:
        raise AssertionError("V11 result has the wrong repository target contract")
    statuses: list[str] = []
    for repository in repositories:
        if set(repository.get("steps", {})) != set(
            result_contract.required_steps_for_target(repository["class"])
        ):
            raise AssertionError(f"incomplete V11 steps for {repository.get('class')}")
        for value in repository["steps"].values():
            if value.get("status") not in ALLOWED_STATUS:
                raise AssertionError("invalid per-step status")
            statuses.append(value["status"])
    volicord = repositories[0]
    lifecycle = volicord["steps"]["multi_work_continuity"]
    if lifecycle["status"] == "passed":
        proof = lifecycle.get("evidence", {}).get("proof")
        errors = result_contract.lifecycle_errors(proof)
        if errors:
            raise AssertionError(f"V11 lifecycle proof is inconsistent: {errors}")
        if proof["project_id"] != volicord.get("project_id"):
            raise AssertionError("V11 lifecycle proof Project differs from target")
        raw = volicord.get("multi_work_rehearsal")
        if isinstance(raw, dict):
            checks = multi_work.verify_rehearsal(
                raw, volicord["steps"]["restart_recall"].get("evidence", {}).get("expected_state", {}),
                raw.get("portable_status"))
            if (not all(checks.values()) or lifecycle["evidence"].get("checks") != checks
                    or result_contract.make_lifecycle_proof(
                        raw, volicord["steps"]["restart_recall"]["evidence"]) != proof):
                raise AssertionError("V11 lifecycle proof disagrees with raw public-operation evidence")
    elif lifecycle.get("evidence", {}).get("proof") is not None:
        raise AssertionError("non-passing V11 lifecycle contains a passing proof")
    expected_counts = {status: statuses.count(status) for status in sorted(ALLOWED_STATUS)}
    if (result.get("counts") != expected_counts
        or any(type(value) is not int for value in result.get("counts", {}).values())):
        raise AssertionError("V11 result status counts do not match repository steps")
    assessment = result.get("decision_revisit_trigger_assessment")
    triggers = result.get("active_decision_revisit_triggers")
    source = result.get("decision_revisit_trigger_source")
    if assessment not in {OFFICIAL_REVISIT_ASSESSMENT, FAILED_REVISIT_ASSESSMENT}:
        raise AssertionError("V11 result has an invalid revisit-trigger assessment state")
    if assessment == OFFICIAL_REVISIT_ASSESSMENT:
        if not isinstance(triggers, list) or not all(
            isinstance(value, str) and DECISION_ID.fullmatch(value) for value in triggers
        ):
            raise AssertionError("official V11 revisit triggers are not a bounded ID list")
        if len(triggers) != len(set(triggers)):
            raise AssertionError("official V11 revisit triggers are duplicated")
    elif triggers is not None:
        raise AssertionError("unassessable V11 revisit evidence must not become an empty list")
    if not isinstance(source, dict):
        raise AssertionError("V11 result is missing Decision-register source identity")
    if source.get("kind") != "accepted_decision_register" or source.get("path") != DECISION_REGISTER_PATH:
        raise AssertionError("V11 result has the wrong Decision-register source identity")
    source_hash = source.get("content_sha256")
    if source_hash is not None and not re.fullmatch(r"[0-9a-f]{64}", source_hash):
        raise AssertionError("V11 result has an invalid Decision-register digest")
    assessed_ids = source.get("assessed_decision_ids")
    if not isinstance(assessed_ids, list) or source.get("assessed_decision_count") != len(assessed_ids):
        raise AssertionError("V11 result has incomplete assessed Decision identities")
    if not all(isinstance(value, str) and DECISION_ID.fullmatch(value) for value in assessed_ids):
        raise AssertionError("V11 result has invalid assessed Decision identities")
    if len(assessed_ids) != len(set(assessed_ids)):
        raise AssertionError("V11 result repeats an assessed Decision identity")
    if assessment == OFFICIAL_REVISIT_ASSESSMENT and (
        not assessed_ids
        or source_hash is None
        or any(value not in assessed_ids for value in triggers)
    ):
        raise AssertionError("official V11 revisit evidence is not grounded in its Decision source")
    if result.get("phase_8_ready") is True and (
        result.get("status") != "passed"
        or assessment != OFFICIAL_REVISIT_ASSESSMENT
        or triggers != []
    ):
        raise AssertionError("V11 phase readiness requires a completed no-trigger assessment")
    if triggers and result.get("phase_8_ready") is not False:
        raise AssertionError("an active Decision revisit trigger cannot be Phase 8 ready")
    if (result.get("status") == "passed") != (result.get("phase_8_ready") is True):
        raise AssertionError("V11 result status and Phase 8 readiness disagree")
    if result.get("status") not in {"passed", "failed"}:
        raise AssertionError("V11 result has an invalid aggregate status")
    performance = result.get("performance")
    if performance is not None:
        qualified = performance_module.qualify(performance.get("observed", {}), performance_module.maintained_limits())
        if qualified != performance or (result.get("phase_8_ready") is True and qualified["status"] != "passed"):
            raise AssertionError("V11 performance evidence or readiness is inconsistent")
    expected_ready = v11_readiness(repositories, result, performance) and qualification_passed(
        result.get("qualification"), result.get("validated_production_head"), result.get("final_gate_artifact"))
    if (result.get("phase_8_ready") is not expected_ready
        or result.get("status") != ("passed" if expected_ready else "failed")):
        raise AssertionError("V11 aggregate verdict disagrees with required leaf evidence")


def assert_required_steps_are_evidence_driven(
    source: str | None = None,
    required_steps: set[str] | None = None,
) -> None:
    tree = ast.parse(source if source is not None else Path(__file__).read_text(encoding="utf-8"))
    required = set(REQUIRED_STEPS + result_contract.VOLICORD_EXTRA) if required_steps is None else required_steps
    assigned: set[str] = set()
    for node in ast.walk(tree):
        if not isinstance(node, ast.Assign) or not isinstance(node.value, ast.Call):
            continue
        call = node.value
        if not isinstance(call.func, ast.Name) or call.func.id != "step" or not call.args:
            continue
        for target in node.targets:
            if not (
                isinstance(target, ast.Subscript)
                and isinstance(target.value, ast.Name)
                and target.value.id == "steps"
                and isinstance(target.slice, ast.Constant)
                and isinstance(target.slice.value, str)
            ):
                continue
            name = target.slice.value
            if name not in required:
                continue
            assigned.add(name)
            if isinstance(call.args[0], ast.Constant):
                raise AssertionError(
                    f"required production-backed step {name} has a permanently hard-coded status"
                )
    missing = required - assigned
    if missing:
        raise AssertionError(f"required steps have no evidence-driven assignment: {sorted(missing)}")


def assert_required_step_policy_regressions() -> None:
    required = {"clean_install"}
    hard_coded_skip = 'steps["clean_install"] = step("skipped", "permanent")\n'
    try:
        assert_required_steps_are_evidence_driven(hard_coded_skip, required)
    except AssertionError as error:
        if "permanently hard-coded status" not in str(error):
            raise
    else:
        raise AssertionError("required step with hard-coded skipped status was accepted")

    evidence_conditional = (
        'steps["clean_install"] = step("passed" if observed else "failed", "observed")\n'
    )
    assert_required_steps_are_evidence_driven(evidence_conditional, required)

    dynamic_runtime_classification = (
        'runtime_status = "skipped" if prerequisite_failed else "environment_blocked"\n'
        'steps["clean_install"] = step(runtime_status, "observed runtime classification")\n'
    )
    assert_required_steps_are_evidence_driven(dynamic_runtime_classification, required)


def assert_authenticated_codex_lifecycle() -> None:
    synthetic_material = b'{"synthetic":"v11-auth-lifecycle"}\n'
    with tempfile.TemporaryDirectory(prefix="volicord-v11-auth-self-check-") as directory:
        root = Path(directory)
        retained = root / "retained"
        registered_codex_home = retained / "work" / "synthetic" / "home" / ".codex"
        registered_codex_home.mkdir(parents=True)
        (registered_codex_home / "config.toml").write_text(
            '[projects."/synthetic/repository"]\ntrust_level = "trusted"\n',
            encoding="utf-8",
        )
        source_auth = root / "source-auth.json"
        source_auth.write_bytes(synthetic_material)
        repository = root / "repository"
        repository.mkdir()
        visibility_marker = root / "child-saw-auth"
        staging_parent = root / "ephemeral-auth"
        fake_codex = root / "synthetic-codex"
        fake_codex.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, pathlib, sys\n"
            "auth = pathlib.Path(os.environ['CODEX_HOME']) / 'auth.json'\n"
            "expected = b'{\\\"synthetic\\\":\\\"v11-auth-lifecycle\\\"}\\n'\n"
            "if not auth.is_file() or auth.read_bytes() != expected:\n"
            "    raise SystemExit(41)\n"
            "if '--model' not in sys.argv or sys.argv[sys.argv.index('--model') + 1] != 'synthetic-selected-model':\n"
            "    raise SystemExit(44)\n"
            "prompt = sys.argv[-1]\n"
            "if 'bounded MCP connectivity probe, not repository work' not in prompt:\n"
            "    raise SystemExit(42)\n"
            "if 'Do not call project_resolve' not in prompt or 'project_health' not in prompt:\n"
            "    raise SystemExit(43)\n"
            "pathlib.Path(os.environ['V11_AUTH_VISIBILITY_MARKER']).write_text('visible\\n')\n"
            "if os.environ.get('V11_SYNTHETIC_CODEX_FAILURE') == '1':\n"
            "    raise SystemExit(19)\n"
            f"print(pathlib.Path({str((HERE / 'fixtures/authenticated-project-health.jsonl').resolve())!r}).read_text())\n",
            encoding="utf-8",
        )
        fake_codex.chmod(0o700)
        recorder = Recorder(retained)
        env = os.environ.copy() | {
            "CODEX_HOME": str(registered_codex_home),
            "V11_AUTH_VISIBILITY_MARKER": str(visibility_marker),
        }

        missing_model = authenticated_codex(
            recorder, str(fake_codex), env, repository, "synthetic-project", retained,
            model=" ", staging_parent=staging_parent, authentication_source=source_auth,
        )
        if missing_model["status"] != "environment_blocked" or visibility_marker.exists():
            raise AssertionError("missing model dispatched a Codex probe")

        succeeded = authenticated_codex(
            recorder,
            str(fake_codex),
            env,
            repository,
            "synthetic-project",
            retained,
            model="synthetic-selected-model",
            staging_parent=staging_parent,
            authentication_source=source_auth,
        )
        if succeeded["status"] != "passed" or not visibility_marker.is_file():
            raise AssertionError("synthetic child could not use staged Codex authentication")
        if any(staging_parent.iterdir()):
            raise AssertionError("Codex authentication staging remains after successful execution")

        failed = authenticated_codex(
            recorder,
            str(fake_codex),
            env | {"V11_SYNTHETIC_CODEX_FAILURE": "1"},
            repository,
            "synthetic-project",
            retained,
            model="synthetic-selected-model",
            staging_parent=staging_parent,
            authentication_source=source_auth,
        )
        if failed["status"] != "environment_blocked":
            raise AssertionError("synthetic child failure was not handled")
        if any(staging_parent.iterdir()):
            raise AssertionError("Codex authentication staging remains after child failure")

        caught = False
        try:
            with staged_codex_authentication(
                source_auth,
                registered_codex_home,
                retained,
                staging_parent=staging_parent,
            ) as codex_home:
                if (codex_home / "auth.json").read_bytes() != synthetic_material:
                    raise AssertionError("staged authentication was unavailable during execution")
                raise RuntimeError("synthetic handled exception")
        except RuntimeError as error:
            if str(error) != "synthetic handled exception":
                raise
            caught = True
        if not caught or any(staging_parent.iterdir()):
            raise AssertionError("Codex authentication staging remains after handled exception")
        if list(retained.rglob("auth.json")):
            raise AssertionError("retained V11 artifacts contain synthetic authentication")
        for path in retained.rglob("*"):
            if path.is_file() and synthetic_material.rstrip() in path.read_bytes():
                raise AssertionError("retained V11 evidence contains synthetic authentication content")
        if source_auth.read_bytes() != synthetic_material:
            raise AssertionError("source Codex authentication was modified")


def assert_recovery_recall_contract() -> None:
    from copy import deepcopy

    user_id, old_source_id, new_source_id = "1" * 32, "2" * 32, "3" * 32
    old_analysis, new_analysis = "a" * 64, "c" * 64

    def source(identity: str, kind: str, basis: str | None) -> dict[str, Any]:
        return {
            "identity": identity, "actor": {"kind": kind, "identity": "fixture-actor"},
            "observer": {"kind": "Agent", "identity": "fixture-host"},
            "availability": "available", "freshness": "current", "snapshot_basis": basis,
        }

    def snapshot(analysis: str, repository: str, observed: int) -> dict[str, Any]:
        freshness = {"state": "current", "repository_snapshot": repository,
                     "compared_repository_snapshot": None, "reason": None}
        return {
            "analysis_snapshot": analysis, "repository_snapshot": repository,
            "freshness": deepcopy(freshness),
            "capabilities": [{
                "capability": "structural", "language": "Python", "state": "partial",
                "repository_snapshot": repository, "freshness": freshness,
                "observed_at_unix_micros": observed,
                "area": {"kind": "repository", "path": "."},
                "coverage": {
                    "included": [{"kind": "file", "path": "app.py"}],
                    "failed": [{"kind": "file", "path": "broken.py"}],
                    "excluded": [], "unsupported": [], "unavailable": [], "stale": [],
                    "covered_file_count": 1, "covered_entity_count": 1,
                    "covered_relation_count": 0,
                },
                "diagnostics": ["parser failure: broken.py"],
                "uncertainty": {"level": "partial", "reasons": ["broken.py"]},
            }],
        }

    before = {
        "project_id": "4" * 32, "goals": ["Keep service behavior understandable"],
        "goal_basis": [{"source_ids": [user_id], "role": "goal"}],
        "decisions": [{"choice": "bounded retry", "user_rationale": "avoid duplicate writes"}],
        "checkpoint": {"verification": "tests passed", "user_review": "pending",
                       "user_acceptance": "pending"},
        "open_questions": [{"revision": 1, "question": "Which retry limit?"}],
        "risks_assumptions_and_limits": ["remote availability is unknown"],
        "declared_assumptions": ["requests have stable identities"],
        "next_step": "review retry behavior",
        "used_sources": [user_id, old_source_id],
        "source_details": [source(user_id, "User", None),
                           source(old_source_id, "Repository", "local-observation:before")],
        "snapshots": [snapshot(old_analysis, "b" * 64, 1)],
    }
    after = deepcopy(before)
    after["used_sources"].append(new_source_id)
    after["source_details"].append(source(new_source_id, "Repository", "local-observation:after"))
    after["snapshots"] = [snapshot(new_analysis, "d" * 64, 2)]
    repaired = {"analysis_snapshot": new_analysis}
    if not all(recovery_recall_checks(before, after, old_analysis, repaired).values()):
        raise AssertionError("valid repair observation failed the Recall contract")

    # Compact Recall is a projection; full same-identity local capability evidence
    # must still reject loss hidden in an omitted field or suffix.
    compact_before, compact_after = deepcopy(before), deepcopy(after)
    for compact in (compact_before, compact_after):
        capability = compact["snapshots"][0]["capabilities"][0]
        del capability["coverage"]
        capability["transport_omission"] = {
            "reason": "serialized_byte_budget", "omitted_field_count": 1,
        }
    evidence = (before["snapshots"][0], after["snapshots"][0])
    if not all(recovery_recall_checks(
        compact_before, compact_after, old_analysis, repaired, evidence
    ).values()):
        raise AssertionError("bounded Recall failed with complete bound capability evidence")
    if all(recovery_recall_checks(compact_before, compact_after, old_analysis, repaired).values()):
        raise AssertionError("omitted capability detail qualified without expansion evidence")
    for key, replacement in (("analysis_snapshot", old_analysis), ("capabilities", [])):
        wrong = deepcopy(evidence[1])
        wrong[key] = replacement
        if all(recovery_recall_checks(
            compact_before, compact_after, old_analysis, repaired, (evidence[0], wrong)
        ).values()):
            raise AssertionError("unbound/incomplete capability expansion qualified")
    changed_coverage = deepcopy(evidence[1])
    changed_coverage["capabilities"][0]["coverage"]["failed"] = []
    if all(recovery_recall_checks(
        compact_before, compact_after, old_analysis, repaired, (evidence[0], changed_coverage)
    ).values()):
        raise AssertionError("omitted failed coverage loss qualified after expansion")
    suffix = [{"transport_omission": {"reason": "serialized_byte_budget", "omitted_count": 1}}]
    if not bounded_projection_matches(evidence[0]["capabilities"], suffix):
        raise AssertionError("exact stable suffix omission rejected")
    suffix[0]["transport_omission"]["omitted_count"] = 2
    if bounded_projection_matches(evidence[0]["capabilities"], suffix):
        raise AssertionError("incorrect suffix omission count accepted")

    with tempfile.TemporaryDirectory(prefix="v11-capability-evidence-") as directory:
        path = Path(directory) / "analysis.json"
        metadata = {
            "identity": old_analysis, "repository_snapshot": "b" * 64,
            "project": {"identity": before["project_id"]},
            "inventory": {"fixture_padding": "x" * 70000},
            "capabilities": evidence[0]["capabilities"],
        }
        # The capability reader deliberately stops before the graph body.
        path.write_text(json.dumps(metadata)[:-1] + ',"structural_facts":[', encoding="utf-8")
        read = read_analysis_capabilities(path, old_analysis, before["project_id"])
        if read["capabilities"] != evidence[0]["capabilities"]:
            raise AssertionError("streamed capability metadata changed meaning")
        normalized = {
            "format_kind": "volicord.analysis_snapshot",
            "format_version": 1,
            "storage_format": "volicord.normalized_analysis.v2",
            "identity": old_analysis,
            "project": {"identity": before["project_id"]},
            "metadata_blob": None,
        }
        packed = analysis_metadata.RAW_MAGIC + json.dumps(metadata).encode()
        metadata_hash = hashlib.sha256(packed).hexdigest()
        (path.parent / "blobs").mkdir(exist_ok=True)
        (path.parent / "blobs" / f"{metadata_hash}.metadata").write_bytes(packed)
        normalized["metadata_blob"] = metadata_hash
        path.write_text(json.dumps(normalized), encoding="utf-8")
        read = read_analysis_capabilities(path, old_analysis, before["project_id"])
        if read["capabilities"] != evidence[0]["capabilities"]:
            raise AssertionError("normalized manifest capability metadata changed meaning")
        mismatched_manifest = deepcopy(normalized)
        mismatched_manifest["identity"] = new_analysis
        path.write_text(json.dumps(mismatched_manifest), encoding="utf-8")
        try:
            read_analysis_capabilities(path, old_analysis, before["project_id"])
        except ValueError:
            pass
        else:
            raise AssertionError("unbound normalized manifest capability metadata accepted")
        path.write_text(json.dumps(normalized), encoding="utf-8")
        for expected_analysis, project in ((new_analysis, before["project_id"]), (old_analysis, "5" * 32)):
            try:
                read_analysis_capabilities(path, expected_analysis, project)
            except ValueError:
                pass
            else:
                raise AssertionError("mismatched local capability identity accepted")
        path.write_text('{"identity":', encoding="utf-8")
        try:
            read_analysis_capabilities(path, old_analysis, before["project_id"])
        except ValueError:
            pass
        else:
            raise AssertionError("incomplete capability metadata accepted")

    learning = [{"candidate_id": "6" * 32, "state": {"state": "completed"}, "canonical_decision": False}]
    if not completed_learning_recalled(learning):
        raise AssertionError("compact completed learning Recall rejected")
    for invalid in ([], [{"learning_deliberation": learning[0]}],
                    [{**learning[0], "canonical_decision": True}],
                    [{**learning[0], "state": {"state": "pending"}}]):
        if completed_learning_recalled(invalid):
            raise AssertionError("missing, pending, or canonical learning authority accepted")

    # A refreshed observation may not conceal any change to retained meaning or provenance.
    mutations = [
        (("goals", 0), "silently changed goal"),
        (("goal_basis", 0, "source_ids", 0), old_source_id),
        (("decisions", 0, "user_rationale"), "inferred rationale"),
        (("decisions", 0, "choice"), "unbounded retry"),
        (("checkpoint", "verification"), "not run"),
        (("checkpoint", "user_review"), "approved"),
        (("checkpoint", "user_acceptance"), "accepted"),
        (("open_questions", 0, "revision"), 2),
        (("risks_assumptions_and_limits", 0), "no risks"),
        (("declared_assumptions", 0), "identities are irrelevant"),
        (("next_step",), "deploy immediately"),
        (("used_sources",), [user_id, old_source_id]),
        (("source_details", 0, "availability"), "unavailable"),
        (("source_details", 0, "actor", "identity"), "another-user"),
        (("source_details", 0, "snapshot_basis"), []),
        (("source_details", 2, "identity"), old_source_id),
        (("source_details", 2, "actor", "kind"), "User"),
        (("source_details", 2, "freshness"), "unknown"),
        (("source_details", 2, "snapshot_basis"), "local-observation:before"),
        (("snapshots",), []),
        (("snapshots", 0, "analysis_snapshot"), old_analysis),
        (("snapshots", 0, "freshness"), None),
        (("snapshots", 0, "freshness", "state"), "stale"),
        (("snapshots", 0, "freshness", "compared_repository_snapshot"), "e" * 64),
        (("snapshots", 0, "capabilities", 0, "freshness"), None),
        (("snapshots", 0, "capabilities", 0, "freshness", "repository_snapshot"), "b" * 64),
        (("snapshots", 0, "capabilities", 0, "coverage"), {}),
        (("snapshots", 0, "capabilities", 0, "coverage", "covered_file_count"), 0),
        (("snapshots", 0, "capabilities", 0, "coverage", "failed"), []),
        (("snapshots", 0, "capabilities", 0, "state"), "available"),
        (("snapshots", 0, "capabilities", 0, "diagnostics"), []),
        (("snapshots", 0, "capabilities"), []),
    ]
    for path, value in mutations:
        changed = deepcopy(after)
        parent = changed
        for key in path[:-1]:
            parent = parent[key]
        parent[path[-1]] = value
        if all(recovery_recall_checks(before, changed, old_analysis, repaired).values()):
            raise AssertionError(f"repair concealed a Recall regression at {path}")
    missing_user = deepcopy(after)
    missing_user["used_sources"].remove(user_id)
    missing_user["source_details"].pop(0)
    incomplete = deepcopy(after)
    del incomplete["goal_basis"]
    for previous, current, repair in (
        (None, None, None), (before, before, repaired), (before, incomplete, repaired),
        (before, missing_user, repaired), (before, after, {"analysis_snapshot": "e" * 64}),
    ):
        if all(recovery_recall_checks(previous, current, old_analysis, repair).values()):
            raise AssertionError("incomplete or mismatched repair evidence qualified")


def assert_candidate_repository_source_contract() -> None:
    source_id = "0f" * 16
    analysis = {
        "repository_source_id": source_id,
        "analysis_snapshot_id": "1a" * 32,
        "repository_snapshot_id": "2b" * 32,
    }
    human_facing_inspection = {
        "records": [{
            "kind": "source",
            "identity": source_id,
            "summary": "Repository snapshot at a readable revision",
        }]
    }
    summary = human_facing_inspection["records"][0]["summary"]
    if "RepositorySnapshot" in summary:
        raise AssertionError("synthetic display summary retained the old machine token")
    if candidate_repository_source_basis(analysis) != [source_id]:
        raise AssertionError("Candidate submission did not use structured repository Source identity")
    if candidate_repository_source_basis(None):
        raise AssertionError("missing analysis fabricated a repository Source identity")


CREDENTIAL_KEYS = {
    "token", "accesstoken", "refreshtoken", "idtoken", "apikey", "openaiapikey",
    "secret", "clientsecret", "password", "authorization", "bearertoken",
}


def credential_needles(material: bytes) -> set[tuple[str, bytes]]:
    """Known auth values and JSON string spellings stay in memory, never evidence."""
    try:
        value = json.loads(material)
    except (ValueError, UnicodeError):
        raise ValueError("authentication material is not valid JSON") from None
    if not isinstance(value, dict):
        raise ValueError("authentication material is not a JSON object")
    needles = {("whole_file", item) for item in (material, material.rstrip()) if item}

    def walk(item, credential_bearing=False):
        if isinstance(item, dict):
            for key, child in item.items():
                normalized = re.sub(r"[^a-z0-9]", "", key.lower())
                walk(child, credential_bearing or normalized in CREDENTIAL_KEYS
                     or normalized.endswith(("token", "secret", "password", "apikey")))
        elif isinstance(item, list):
            for child in item:
                walk(child, credential_bearing)
        elif credential_bearing and isinstance(item, str) and item:
            needles.add(("value", item.encode("utf-8")))
            for ascii_only in (False, True):
                needles.add(("value", json.dumps(item, ensure_ascii=ascii_only)[1:-1].encode("utf-8")))

    walk(value)
    return needles


def capture_credential_material(path: Path, material: dict) -> None:
    try:
        material["needles"].update(credential_needles(path.read_bytes()))
    except (OSError, ValueError, RecursionError):
        material["errors"] += 1


def credential_retention_audit(
    artifact_directory: Path, authentication_source: Path | None = None, *,
    known_material: dict | None = None,
) -> dict[str, Any]:
    material = known_material
    if material is None:
        material = {"needles": set(), "errors": 0}
        auth = authentication_source or (
            Path(os.environ.get("CODEX_HOME", str(Path.home() / ".codex"))) / "auth.json"
        )
        capture_credential_material(auth, material)
    named_files = content_matches = whole_matches = value_matches = 0
    scan_errors = material["errors"]
    needles = material["needles"]
    overlap = max((len(needle) for _category, needle in needles), default=1) - 1
    if not artifact_directory.is_dir() or artifact_directory.is_symlink():
        scan_errors += 1
    else:
        try:
            for path in artifact_directory.rglob("*"):
                # Never follow retained links into unrelated user files.
                if path.is_symlink():
                    scan_errors += 1
                    continue
                if not path.is_file():
                    continue
                if path.name == "auth.json":
                    named_files += 1
                matches = set()
                try:
                    with path.open("rb") as stream:
                        tail = b""
                        while chunk := stream.read(65536):
                            content = tail + chunk
                            for category, needle in needles:
                                if needle in content:
                                    matches.add(category)
                            tail = content[-overlap:] if overlap else b""
                except OSError:
                    scan_errors += 1
                    continue
                if matches:
                    content_matches += 1
                whole_matches += "whole_file" in matches
                value_matches += "value" in matches
        except OSError:
            scan_errors += 1
    passed = named_files == 0 and content_matches == 0 and scan_errors == 0
    return {
        "kind": "v11_credential_retention_audit",
        "status": "passed" if passed else "failed",
        "auth_named_file_count": named_files,
        "credential_content_match_count": content_matches,
        "whole_auth_file_match_count": whole_matches,
        "credential_value_match_count": value_matches,
        "scan_error_count": scan_errors,
    }


def assert_credential_retention_audit() -> None:
    import credential_audit_self_test
    credential_audit_self_test.self_check(sys.modules[__name__])


def assert_current_materiality_review_contract(source: str) -> None:
    required_by_action = {
        "record": {
            "action",
            "project_id",
            "engineering_choice_discovery_candidate_id",
            "rationale",
            "behavioral_context_basis",
            "learning_participation",
            "judgments",
        },
        "revise": {
            "action",
            "project_id",
            "review_candidate_id",
            "rationale",
            "learning_participation",
            "judgments",
        },
    }
    obsolete_judgment_fields = {
        "dimension_id",
        "discovered_choice_ids",
        "summary",
        "affected_scope",
        "material_consequences",
        "observable_signals",
        "basis",
    }
    found: set[str] = set()
    for node in ast.walk(ast.parse(source)):
        if not (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and node.func.attr == "tool"
            and len(node.args) >= 2
            and isinstance(node.args[0], ast.Constant)
            and node.args[0].value == "materiality_review"
            and isinstance(node.args[1], ast.Dict)
        ):
            continue
        payload = {
            key.value: value
            for key, value in zip(node.args[1].keys, node.args[1].values)
            if isinstance(key, ast.Constant) and isinstance(key.value, str)
        }
        action_node = payload.get("action")
        if not isinstance(action_node, ast.Constant) or action_node.value not in required_by_action:
            continue
        action = action_node.value
        found.add(action)
        payload_fields = set(payload)
        if payload_fields != required_by_action[action]:
            raise AssertionError(
                f"V11 {action} materiality payload does not match the current public contract: "
                f"{sorted(payload_fields)}"
            )
        judgments = payload["judgments"]
        if not isinstance(judgments, ast.List) or not judgments.elts:
            raise AssertionError(f"V11 {action} materiality payload lost caller-owned judgments")
        for judgment in judgments.elts:
            if not isinstance(judgment, ast.Dict):
                raise AssertionError(f"V11 {action} materiality judgment is not inspectable")
            judgment_fields = {
                key.value
                for key in judgment.keys
                if isinstance(key, ast.Constant) and isinstance(key.value, str)
            }
            if not {
                "choice_id",
                "disposition",
                "basis_summary",
                "authority_counterfactual",
                "materially_varying_outcomes",
                "contains_user_owned_outcome",
                "user_owned_outcomes",
                "ownership_rationale",
                "ownership_source_ids",
                "alternative_accounting",
                "learning_value",
            } <= judgment_fields:
                raise AssertionError(f"V11 {action} materiality judgment lost required authority fields")
            retained = sorted(judgment_fields & obsolete_judgment_fields)
            if retained:
                raise AssertionError(
                    f"V11 {action} materiality judgment retained discovery-owned fields: {retained}"
                )
    if found != set(required_by_action):
        raise AssertionError(f"V11 materiality journey is incomplete: {sorted(found)}")


def assert_qualification_publication() -> None:
    from unittest.mock import patch

    with tempfile.TemporaryDirectory(prefix="volicord-v11-publication-self-check-") as directory:
        root = Path(directory)
        probe = root / "probe.jsonl"
        probe.write_text((HERE / "fixtures/authenticated-project-health.jsonl").read_text())
        performance = performance_module.qualify({
            **{key: 1 for key in performance_module.METRICS}, "mcp_call_count": 1,
            "mcp_sample_count": 1, "sampling_error_count": 0,
            "analysis_snapshot_count": 1, "analysis_graph_item_count": 1,
        }, performance_module.maintained_limits())
        for mutation in (None, "head", "worktree", "final"):
            candidate = "1" * 40
            state = {"status": "passed", "head": candidate, "worktree_clean": True}
            binding = {"gate_invocation": "synthetic-current-gate", "final_sha256": "2" * 64,
                       "final_artifact": str(root / "final.json")}
            mutated = False
            def read_final(*_args):
                if mutation == "final" and mutated:
                    raise ValueError("Final bytes changed during qualification")
                return binding
            def target(name, *_args):
                nonlocal mutated
                mutated = True
                if mutation == "head":
                    state.update(status="failed", head="3" * 40)
                elif mutation == "worktree":
                    state.update(status="failed", worktree_clean=False)
                steps = {key: step("passed", "synthetic publication fixture")
                         for key in result_contract.required_steps_for_target(name)}
                if name == "volicord":
                    raw, restart, proof = multi_work_self_test.contract_fixture(
                        result_contract, "synthetic-project")
                    synthetic_lifecycle_status = "passed"
                    steps["multi_work_continuity"] = step(
                        synthetic_lifecycle_status, "synthetic publication fixture",
                        proof=proof,
                        checks=multi_work.verify_rehearsal(
                            raw, restart["expected_state"], raw["portable_status"]),
                    )
                    steps["restart_recall"]["evidence"] = restart
                steps["project_binding"]["evidence"] = {"project_id": "synthetic-project"}
                steps["codex_mcp_connection"]["evidence"] = {"authenticated": step("passed", "fixture",
                    operation={"exit_code": 0, "outcome": "succeeded", "spawn_error": None,
                               "termination": None, "stdout": str(probe)},
                    probe=validate_codex_probe(probe.read_text(), "synthetic-project"))}
                return {
                    "class": name, "project_id": "synthetic-project", "steps": steps,
                    **({"multi_work_rehearsal": raw} if name == "volicord" else {}),
                }
            args = SimpleNamespace(model="synthetic-model", validated_head=candidate,
                final_artifact=binding["final_artifact"], output_dir=str(root / str(mutation)))
            collector = SimpleNamespace(enabled=False, report=lambda _duration: performance,
                                        diagnostics=lambda: {})
            with patch.dict(run.__globals__, {"PERFORMANCE": collector}), \
                 patch.dict(qualification_preflight.__globals__, {"candidate_state": lambda _head: dict(state)}), \
                 patch.object(final_evidence, "read_gate_final", side_effect=read_final), \
                 patch.dict(run.__globals__, {"rehearse_target": target}):
                from contextlib import redirect_stdout
                import io
                with redirect_stdout(io.StringIO()):
                    exit_code = run(args)
            result = json.loads((Path(args.output_dir) / "result.json").read_text())
            assert result["counts"]["passed"] == result_contract.required_total()
            assert (exit_code == 0) is (mutation is None)
            assert result["phase_8_ready"] is (mutation is None)
            assert result["qualification"]["status"] == ("passed" if mutation is None else "failed")
            validate_result(result)


def self_check() -> int:
    analysis_metadata.self_check()
    performance_module.self_check()
    restart_test_spec = importlib.util.spec_from_file_location(
        "v11_restart_recall_self_test", HERE / "restart_recall_self_test.py"
    )
    assert restart_test_spec is not None and restart_test_spec.loader is not None
    restart_test = importlib.util.module_from_spec(restart_test_spec)
    restart_test_spec.loader.exec_module(restart_test)
    restart_test.self_check(restart_recall)
    multi_work_self_test.self_check(multi_work)
    multi_work_self_test.contract_self_check(result_contract)
    if platform.system() != "Linux":
        raise AssertionError("V11 is qualified only on Linux")
    if not SMALL_FIXTURE.is_dir() or not POLYGLOT_FIXTURE.is_dir():
        raise AssertionError("required repository fixtures are missing")
    if len(list(POLYGLOT_FIXTURE.rglob("*"))) < 16:
        raise AssertionError("polyglot fixture is no longer medium-sized")
    suffixes = {path.suffix for path in POLYGLOT_FIXTURE.rglob("*") if path.is_file()}
    if not {".java", ".py", ".ts", ".md"} <= suffixes:
        raise AssertionError("polyglot fixture lost three languages or documentation")
    boundary_fixture = material_boundary_review(
        [{
            "choice_id": "api-boundary",
            "effect_categories": [
                "public_api_shape_or_semantics",
                "failure_or_error_semantics",
            ],
        }],
        ["current-source"],
    )
    if (
        len(boundary_fixture) != len(ENGINEERING_EFFECT_CATEGORIES)
        or len({review["effect_category"] for review in boundary_fixture})
        != len(ENGINEERING_EFFECT_CATEGORIES)
        or next(
            review
            for review in boundary_fixture
            if review["effect_category"] == "public_api_shape_or_semantics"
        )["conclusion"]
        != {"state": "represented_by_choices", "choice_ids": ["api-boundary"]}
        or next(
            review
            for review in boundary_fixture
            if review["effect_category"] == "security"
        )["conclusion"].get("state")
        != "no_independent_fork"
    ):
        raise AssertionError("V11 material-boundary review fixture is incomplete")
    unresolved_accounts = alternative_accounting(
        "context-boundary", ["local", "remote"], "current-source"
    )
    resolved_accounts = alternative_accounting(
        "context-boundary",
        ["local", "remote"],
        "current-source",
        resolution_decision_id="current-decision",
    )
    if (
        [account["status"] for account in unresolved_accounts]
        != ["unresolved", "unresolved"]
        or [account["status"] for account in resolved_accounts]
        != ["selected", "eliminated_by_applicable_decision"]
        or resolved_accounts[1].get("decision_id") != "current-decision"
    ):
        raise AssertionError("V11 alternative accounting fixture is incomplete")
    assert_required_steps_are_evidence_driven()
    assert_required_step_policy_regressions()
    if parser_degradation_status(
        {"state": "partial", "partial_scopes": ["Structural:Some(Rust):."]}
    ) != "passed":
        raise AssertionError("V11 rejected a truthful partial structural degradation")
    if parser_degradation_status(
        {"state": "partial", "failed_scopes": ["Structural:Some(Python):."]}
    ) != "passed":
        raise AssertionError("V11 rejected a truthful failed structural degradation")
    if parser_degradation_status(
        {"state": "partial", "partial_scopes": ["Ecosystem:Some(Rust):."]}
    ) != "failed":
        raise AssertionError("V11 accepted degradation without a structural scope")
    source = Path(__file__).read_text(encoding="utf-8")
    assert_current_materiality_review_contract(source)
    obsolete_pairs = {
        ("project", "init"),
        ("canonical", "user-source"),
        ("portable", "export"),
        ("documents", "export"),
        ("checkpoint", "record"),
        ("advanced", "checkpoint"),
    }
    for node in ast.walk(ast.parse(source)):
        if not (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Name)
            and node.func.id == "cli_json"
        ):
            continue
        literal_arguments = [
            argument.value
            for argument in node.args
            if isinstance(argument, ast.Constant) and isinstance(argument.value, str)
        ]
        for pair in zip(literal_arguments, literal_arguments[1:]):
            if pair in obsolete_pairs:
                raise AssertionError(f"V11 retained an obsolete CLI vector: {pair}")
    for current in (
        'argv = [str(cli), "--json"]',
        '"viewer",',
        '"document_preview",',
        '"ko-KR",',
        '"requested_language": "ko-KR",',
        '"all_generated_prose_realized": True,',
        'contains_hangul(realized_document.get("content", ""))',
        'viewer_understanding = viewer_project_understanding_evidence(',
        'viewer_understanding["status"] == "passed"',
        'provider_request_after.get("outcome") == "provider_unavailable"',
        '"local_canonical": local_canonical',
        '"action": "submit_question_from_materiality"',
        '"presentation_receipt_id": displayed.get("presentation_receipt_id")',
        '"work_scope": "project_wide"',
        'discovery, discovery_ok = host.tool("engineering_choice_discovery"',
        '"material_boundary_review": material_boundary_review(',
        '"role": "learning"',
        '"behavioral_context_basis": {',
        '"interruption_counterfactual":',
        '"participation_scope_alignment":',
        '"delegated_implementation_choice"',
        '"affected_scope": ["internal-state", "v11-ordinary-work.txt"]',
        '"v11-ordinary-work.txt",\n                                ],',
        'review, review_ok = host.tool("materiality_review"',
        '"review_candidate_id": review_candidate_id,',
        '"paths": ["v11-ordinary-work.txt"],',
        '"components": [],',
        '"work_contexts": ["internal-state"] if technical_delegated else []',
        'candidate_research_analysis, candidate_research_analysis_ok = host.tool(',
        'resolved_review, resolved_review_ok = host.tool("materiality_review"',
        'begun, begun_ok = host.tool("learning_deliberation"',
        'recall_after.get("learning_context_health", {}).get("state") == "available"',
        'checkpoint_value, checkpoint_ok = host.tool("checkpoint_record"',
        'parser_status = parser_degradation_status(parser_result)',
    ):
        if current not in source:
            raise AssertionError(f"V11 lost a current public-journey contract: {current}")
    materiality_source = (HERE / "materiality_scenarios.py").read_text(encoding="utf-8")
    if '"work_scope": "project_wide"' not in materiality_source:
        raise AssertionError("V11 materiality scenarios lost explicit Decision work scope")
    if source.count('"delegated_scope": ["internal-state"],') != 1:
        raise AssertionError("V11 retained a delegation narrower than the reviewed work scope")
    with tempfile.TemporaryDirectory(prefix="volicord-v11-viewer-contract-") as directory:
        viewer_contract = Path(directory) / "project-understanding.html"
        viewer_contract.write_text(
            '<!doctype html><html lang="en"><body><h1>Project Understanding</h1>'
            '<h2>How the architecture and code connect</h2>'
            '<span data-statement-role="verified-fact">Verified fact</span>'
            '<span data-statement-role="deterministic-derived">Deterministic explanation</span>'
            '<span data-statement-role="generated-interpretation">Generated interpretation</span>'
            '<p>Service</p><div class="grounded-explanations">'
            '<article class="deterministic-derived explanation-item" '
            'data-explanation-kind="component-role"><p>Service owns a grounded and readable '
            'repository component responsibility.</p><details class="explanation-evidence">'
            '<summary>Inspect evidence basis</summary></details></article></div>'
            '<figure class="grounded-diagram" data-diagram="architecture-topology">'
            '<g class="diagram-node" data-entity-id="service"></g>'
            '<g class="diagram-node" data-entity-id="client"></g>'
            '<g class="diagram-edge" data-relation-id="calls" '
            'data-relation-class="structural" data-source-entity="client" '
            'data-target-entity="service"></g></figure>'
            '<figure class="grounded-diagram" data-diagram="flow-topology"></figure>'
            '</body></html>',
            encoding="utf-8",
        )
        viewer_contract_result = viewer_project_understanding_evidence(
            viewer_contract,
            {"architecture": {"components": [{"display_name": "Service"}]}},
        )
        if viewer_contract_result["status"] != "passed":
            raise AssertionError("grounded Viewer Project Understanding did not qualify")
        viewer_contract.write_text(
            viewer_contract.read_text(encoding="utf-8").replace(
                'class="diagram-edge"',
                'class="ungrounded-edge"',
            ),
            encoding="utf-8",
        )
        if viewer_project_understanding_evidence(
            viewer_contract,
            {"architecture": {"components": [{"display_name": "Service"}]}},
        )["status"] != "failed":
            raise AssertionError("ungrounded Viewer diagram qualified")
        reduced_contract = Path(directory) / "project-understanding-reduced.html"
        reduced_message = (
            "No repository component is grounded in the current Goal, Checkpoint, or active "
            "Decision; generic topology was not substituted."
        )
        reduced_contract.write_text(
            '<!doctype html><html lang="en"><body><h1>Project Understanding</h1>'
            '<h2>How the architecture and code connect</h2>'
            '<span data-statement-role="verified-fact">Verified fact</span>'
            '<span data-statement-role="deterministic-derived">Deterministic explanation</span>'
            '<span data-statement-role="generated-interpretation">Generated interpretation</span>'
            '<p>Service</p><div class="grounded-explanations">'
            '<article class="deterministic-derived explanation-item" '
            'data-explanation-kind="gap"><p>No resolved flow is available; no execution or '
            'data-flow path is inferred.</p><details class="explanation-evidence">'
            '<summary>Inspect evidence basis</summary><dl>'
            '<div><dt>Evidence class</dt><dd>capability gap</dd></div>'
            '</dl></details></article></div>'
            '<figure class="grounded-diagram" data-diagram="architecture-topology">'
            '<p class="empty-state">'
            + reduced_message
            + '</p></figure><figure class="grounded-diagram" data-diagram="flow-topology">'
            '<p class="empty-state">'
            + reduced_message
            + "</p></figure></body></html>",
            encoding="utf-8",
        )
        if viewer_project_understanding_evidence(
            reduced_contract,
            {"architecture": {"components": [], "relationships": [], "gaps": [{}]}},
        )["status"] != "passed":
            raise AssertionError("truthful reduced Viewer architecture did not qualify")
        reduced_contract.write_text(
            reduced_contract.read_text(encoding="utf-8").replace(
                "<dd>capability gap</dd>",
                "<dd>uninspected absence</dd>",
            ),
            encoding="utf-8",
        )
        if viewer_project_understanding_evidence(
            reduced_contract,
            {"architecture": {"components": [], "relationships": [], "gaps": [{}]}},
        )["status"] != "failed":
            raise AssertionError("uninspectable reduced Viewer architecture qualified")
    assert_recovery_recall_contract()
    assert_candidate_repository_source_contract()
    assert_qualification_publication()
    assert_codex_probe_completion()
    assert_authenticated_codex_lifecycle()
    assert_credential_retention_audit()
    import execution_controls_self_test
    execution_controls_self_test.self_check(sys.modules[__name__])
    assessment = read_decision_revisit_assessment()
    if assessment["active_decision_revisit_triggers"]:
        raise AssertionError("the maintained Decision register has an active revisit trigger")
    active_source = DECISION_REGISTER.read_text(encoding="utf-8").replace(
        "- 미해결 필수 제품 질문: 없음",
        "- 미해결 필수 제품 질문: Q3",
        1,
    )
    active_assessment = decision_revisit_assessment(
        active_source,
        source_sha256=hashlib.sha256(active_source.encode("utf-8")).hexdigest(),
    )
    if active_assessment["active_decision_revisit_triggers"] != ["Q3"]:
        raise AssertionError("active Decision revisit trigger was not assessed")
    malformed_source = DECISION_REGISTER.read_text(encoding="utf-8").replace(
        "- 미해결 필수 제품 질문: 없음",
        "- 미해결 필수 제품 질문: unresolved prose",
        1,
    )
    try:
        decision_revisit_assessment(
            malformed_source,
            source_sha256=hashlib.sha256(malformed_source.encode("utf-8")).hexdigest(),
        )
    except ValueError:
        pass
    else:
        raise AssertionError("unassessable Decision revisit evidence was accepted")
    fake_repositories = [
        {"class": name, "steps": {
            key: step("skipped", "self-check")
            for key in result_contract.required_steps_for_target(name)}}
        for name in result_contract.TARGETS
    ]
    make_v11_result(
        validated_production_head="0" * 40,
        final_gate_artifact="/synthetic/final.json",
        duration_ms=0.0,
        repositories=fake_repositories,
        revisit_assessment=assessment,
    )
    active_result = make_v11_result(
        validated_production_head="0" * 40,
        final_gate_artifact="/synthetic/final.json",
        duration_ms=0.0,
        repositories=[
            {
                "class": name,
                **({"project_id": "p" * 32} if name == "volicord" else {}),
                "steps": {
                    key: step("passed", "self-check", **(
                        {"proof": multi_work_self_test.contract_fixture(result_contract)[2]}
                        if key == "multi_work_continuity" else {}
                    ))
                    for key in result_contract.required_steps_for_target(name)
                },
            }
            for name in result_contract.TARGETS
        ],
        revisit_assessment=active_assessment,
    )
    if active_result["phase_8_ready"] is not False or active_result["status"] != "failed":
        raise AssertionError("active Decision revisit trigger did not block Phase 8")
    limits = performance_module.maintained_limits()
    observed = {**limits, "mcp_sample_count": 1, "mcp_call_count": 1,
                "sampling_error_count": 0, "analysis_snapshot_count": 1,
                "analysis_graph_item_count": 1}
    for metric in performance_module.METRICS:
        report = performance_module.qualify({**observed, metric: limits[metric] + 1}, limits)
        failed = make_v11_result(
            validated_production_head="0" * 40,
            final_gate_artifact="/synthetic/final.json", duration_ms=1.0,
            repositories=active_result["repositories"], revisit_assessment=assessment,
            performance=report,
        )
        assert failed["counts"]["passed"] == result_contract.required_total()
        assert failed["status"] == "failed" and failed["phase_8_ready"] is False
        report["status"] = "passed"
        try:
            validate_result(failed)
        except AssertionError:
            pass
        else:
            raise AssertionError("falsified performance verdict was accepted")
    print(json.dumps({
        "status": "passed",
        "required_steps_by_target": result_contract.required_counts(),
        "evidence_driven_steps": len(REQUIRED_STEPS + result_contract.VOLICORD_EXTRA),
        "required_step_policy_regressions": "passed",
        "candidate_structured_repository_source_regression": "passed",
        "self_guiding_work_authority_checkpoint_path": "passed",
        "viewer_project_understanding_contract": "passed",
        "execution_controls": "passed",
        "authentication_lifecycle": "passed",
        "credential_retention_audit": "passed",
        "decision_revisit_trigger_assessment": "passed",
        "active_decision_revisit_trigger_regression": "passed",
        "unassessable_decision_revisit_regression": "passed",
        "polyglot_hash": tree_hash(POLYGLOT_FIXTURE),
    }, indent=2))
    return 0


def candidate_state(candidate_head: str) -> dict[str, Any]:
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                          text=True, capture_output=True, check=False)
    status = subprocess.run(["git", "status", "--porcelain=v1", "--untracked-files=all"],
                            cwd=ROOT, text=True, capture_output=True, check=False)
    observed = head.stdout.strip() if head.returncode == 0 else None
    clean = status.returncode == 0 and not status.stdout
    passed = head.returncode == 0 and observed == candidate_head and clean
    return {"status": "passed" if passed else "failed", "head": observed,
            "validated_head": candidate_head, "head_matches": observed == candidate_head,
            "worktree_clean": clean}


def qualification_preflight(args: argparse.Namespace) -> tuple[dict[str, Any], dict[str, Any] | None]:
    state = candidate_state(args.validated_head)
    binding = None
    error = None
    try:
        binding = final_evidence.read_gate_final(ROOT, args.validated_head, Path(args.final_artifact))
    except (OSError, ValueError, KeyError, TypeError, AttributeError) as failure:
        error = str(failure)
    passed = platform.system() == "Linux" and state["status"] == "passed" and binding is not None
    return {**state, "status": "passed" if passed else "failed",
            "final_artifact": str(Path(args.final_artifact).resolve()),
            "same_gate_final_valid": binding is not None, "error": error}, binding


def preflight(args: argparse.Namespace) -> int:
    result, _binding = qualification_preflight(args)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["status"] == "passed" else 1


def run(args: argparse.Namespace) -> int:
    if not args.model.strip():
        raise ValueError("an explicit Codex probe model is required")
    start_state, binding = qualification_preflight(args)
    if start_state["status"] != "passed":
        print(json.dumps(start_state, indent=2, sort_keys=True))
        return 1
    assert binding is not None
    output = Path(args.output_dir).resolve()
    if output.exists():
        raise RuntimeError("V11 output directory already exists")
    output.mkdir(parents=True)
    recorder = Recorder(output)
    base_env = os.environ.copy()
    base_env.setdefault("CARGO_HOME", str(Path.home() / ".cargo"))
    base_env.setdefault("RUSTUP_HOME", str(Path.home() / ".rustup"))
    started = time.monotonic_ns()
    PERFORMANCE.enabled = True
    repositories = []
    for target in ("volicord", "small-python", "polyglot-medium"):
        try:
            repositories.append(rehearse_target(target, output, recorder, base_env, shutil.which("codex"), args.model))
        except (AssertionError, OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
            repositories.append({
                "class": target,
                "identity": {},
                "steps": {name: step("failed" if name == "clean_install" else "skipped", str(error)) for name in result_contract.required_steps_for_target(target)},
            })
    try:
        revisit_assessment = read_decision_revisit_assessment()
    except (OSError, ValueError):
        revisit_assessment = failed_decision_revisit_assessment()
    duration_ms = round((time.monotonic_ns() - started) / 1_000_000, 3)
    performance = PERFORMANCE.report(duration_ms)
    write_json(output / "performance-calls.json", PERFORMANCE.diagnostics())
    end_state, end_binding = qualification_preflight(args)
    qualification = {
        "status": "passed" if end_state["status"] == "passed" and end_binding == binding else "failed",
        "gate_invocation": binding["gate_invocation"],
        "final_artifact": binding["final_artifact"], "final_sha256": binding["final_sha256"],
        "start": start_state, "end": end_state,
    }
    result = make_v11_result(
        validated_production_head=args.validated_head,
        final_gate_artifact=str(Path(args.final_artifact).resolve()),
        duration_ms=duration_ms,
        repositories=repositories,
        revisit_assessment=revisit_assessment,
        performance=performance,
        qualification=qualification,
    )
    # Recheck immediately at publication, after result validation and probe replay.
    publication_state, publication_binding = qualification_preflight(args)
    if publication_state["status"] != "passed" or publication_binding != binding:
        result["qualification"]["status"] = "failed"
        result["qualification"]["end"] = publication_state
        result["status"] = "failed"
        result["phase_8_ready"] = False
    write_json(output / "result.json", result)
    print(json.dumps({
        "status": result["status"], "phase_8_ready": result["phase_8_ready"],
        "result": str(output / "result.json"), "counts": result["counts"],
        "decision_revisit_trigger_assessment": result["decision_revisit_trigger_assessment"],
        "active_decision_revisit_triggers": result["active_decision_revisit_triggers"],
    }, indent=2, sort_keys=True))
    return 0 if result["status"] == "passed" else 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-check")
    for name in ("preflight", "run"):
        child = subparsers.add_parser(name)
        child.add_argument("--validated-head", required=True)
        child.add_argument("--final-artifact", required=True)
        if name == "run":
            child.add_argument("--output-dir", required=True)
            child.add_argument("--model", required=True)
    audit = subparsers.add_parser("credential-audit")
    audit.add_argument("--artifact-dir", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "self-check":
        return self_check()
    if args.command == "preflight":
        return preflight(args)
    if args.command == "credential-audit":
        result = credential_retention_audit(Path(args.artifact_dir).resolve())
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0 if result["status"] == "passed" else 1
    return run(args)


def interrupted_by_signal(number, _frame):
    raise SystemExit(128 + number)


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupted_by_signal)
    try:
        raise SystemExit(main())
    finally:
        for host in list(Mcp.active):
            host.close()
