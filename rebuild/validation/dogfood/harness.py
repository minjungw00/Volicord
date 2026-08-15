#!/usr/bin/env python3
"""Run and sanitize the repository-owned Phase 8 repeated dogfood evaluation."""

from __future__ import annotations

import argparse
from collections import Counter
import datetime as dt
import hashlib
from html.parser import HTMLParser
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import socket
import subprocess
import sys
import tempfile
import time
from typing import Any, Callable

from codex_events import (
    CanonicalBundle,
    CodexCapture,
    EvidenceError,
    ToolCall,
    command_is_repository_inspection,
    decode_string_blob,
    load_canonical_bundle,
    load_codex_capture,
    recalled_checkpoint,
    recalled_decision_ids,
    relevant_context_ids,
)


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
DEFINITION = HERE / "evaluation.json"
V11_HARNESS = ROOT / "rebuild/validation/end-to-end/multi-repository/harness.py"
DECISION_REGISTER = ROOT / "rebuild/docs/design/open-decisions.md"
ALLOWED_STATUS = {
    "passed", "failed", "partial", "unsupported", "skipped", "environment_blocked"
}
CLASSES = ("volicord", "small-python", "polyglot-medium")
REAL_SESSION_CHECKS = (
    "repository_specific_objective",
    "clean_bounded_baseline",
    "meaningful_ordinary_changes",
    "source_grounded_checkpoint",
    "explicit_user_decision_source",
    "distinct_work_and_resume_invocations",
    "fresh_resume_without_prior_context",
    "recall_precedes_inspection_and_continuation",
    "recall_matches_checkpoint_decision_and_context",
    "meaningful_recalled_continuation",
)
OFFICIAL_SUFFIXES = {
    ".java": "Java",
    ".py": "Python",
    ".js": "JavaScript",
    ".jsx": "JavaScript",
    ".ts": "TypeScript",
    ".tsx": "TypeScript",
    ".c": "C",
    ".h": "C",
    ".cc": "C++",
    ".cpp": "C++",
    ".cxx": "C++",
    ".hpp": "C++",
    ".rs": "Rust",
}
DOCUMENT_SUFFIXES = {".md", ".markdown", ".rst", ".adoc"}
IGNORED_PARTS = {".git", ".local", "target", "node_modules", "vendor", "dist", "build"}
SECRET_MARKERS = (
    "bearer ", "api_key", "api-key", "access_token", "refresh_token",
    "private prompt", "auth.json", "credential_content",
)


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="microseconds")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def command_output(cwd: Path, *argv: str) -> str | None:
    result = subprocess.run(argv, cwd=cwd, text=True, capture_output=True, check=False)
    return result.stdout.strip() if result.returncode == 0 else None


def git_head(path: Path) -> str | None:
    return command_output(path, "git", "rev-parse", "HEAD")


def git_clean(path: Path) -> bool:
    value = command_output(path, "git", "status", "--porcelain=v1", "--untracked-files=all")
    return value == ""


def directory_bytes(path: Path) -> int:
    total = 0
    if not path.exists():
        return 0
    for root, directories, files in os.walk(path):
        directories[:] = [name for name in directories if name not in {".git"}]
        for name in files:
            try:
                total += (Path(root) / name).stat().st_size
            except OSError:
                continue
    return total


def load_v11() -> Any:
    spec = importlib.util.spec_from_file_location("volicord_phase8_v11", V11_HARNESS)
    if spec is None or spec.loader is None:
        raise RuntimeError("the maintained V11 harness could not be loaded")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_definition() -> dict[str, Any]:
    value = json.loads(DEFINITION.read_text(encoding="utf-8"))
    if value.get("kind") != "phase8_dogfood_evaluation_definition":
        raise ValueError("unexpected Phase 8 evaluation definition kind")
    if set(value.get("status_values", [])) != ALLOWED_STATUS:
        raise ValueError("the Phase 8 status vocabulary is incomplete")
    if value.get("candidate_cycle_count") != 2:
        raise ValueError("Phase 8 requires exactly two independent cycles per repository")
    if tuple(value.get("repository_classes", {})) != CLASSES:
        raise ValueError("the Phase 8 repository class order changed")
    v11 = load_v11()
    if tuple(value.get("required_product_steps", [])) != tuple(v11.REQUIRED_STEPS):
        raise ValueError("Phase 8 no longer routes its product journey through maintained V11 steps")
    evidence = value.get("real_session_evidence", {})
    if evidence.get("mode") != "verify_repository_normalized_codex_rollout_and_canonical_bundle":
        raise ValueError("Phase 8 must normalize Codex rollout and canonical product evidence")
    if evidence.get("harness_performs_or_authorizes_transmission") is not False:
        raise ValueError("the Phase 8 verifier may not claim Codex transmission authority")
    return value


def repository_files(path: Path) -> list[Path]:
    return [
        item
        for item in path.rglob("*")
        if item.is_file() and not any(part in IGNORED_PARTS for part in item.relative_to(path).parts)
    ]


def repository_identity(kind: str, spec: dict[str, Any], definition: dict[str, Any]) -> dict[str, Any]:
    path = Path(spec["path"]).resolve()
    head = git_head(path)
    origin = command_output(path, "git", "remote", "get-url", "origin")
    files = repository_files(path) if path.is_dir() else []
    languages = sorted({OFFICIAL_SUFFIXES[item.suffix.lower()] for item in files if item.suffix.lower() in OFFICIAL_SUFFIXES})
    documents = sum(item.suffix.lower() in DOCUMENT_SUFFIXES for item in files)
    license_path = path / spec.get("license_file", "") if spec.get("license_file") else None
    license_identity = {
        "spdx": spec.get("license_spdx"),
        "file": license_path.relative_to(path).as_posix() if license_path and license_path.is_file() else None,
        "sha256": sha256(license_path) if license_path and license_path.is_file() else None,
    }
    status = "passed"
    blockers: list[str] = []
    if not path.is_dir() or head is None:
        blockers.append("source repository or revision is unavailable")
    if head != spec.get("revision"):
        blockers.append("configured revision does not match the source repository HEAD")
    if origin != spec.get("origin"):
        blockers.append("configured origin does not match the source repository")
    if not git_clean(path):
        blockers.append("source repository is not clean")
    if license_identity["file"] is None and kind != "volicord":
        blockers.append("external repository license file is unavailable")
    fixture_root = (ROOT / "rebuild/validation").resolve()
    if path == fixture_root or fixture_root in path.parents:
        blockers.append("a maintained validation fixture cannot substitute for an actual repository")
    rules = definition["repository_classes"][kind]
    if kind == "volicord" and head != git_head(ROOT):
        blockers.append("Volicord repository revision is not the candidate HEAD")
    if kind == "small-python":
        if languages != ["Python"]:
            blockers.append("small repository is not a single official Python application")
        if len(files) > rules["maximum_files"]:
            blockers.append("small repository exceeds the bounded file ceiling")
    if kind == "polyglot-medium":
        if len(files) < rules["minimum_files"]:
            blockers.append("polyglot repository does not meet the medium file floor")
        if len(languages) < rules["minimum_official_structural_languages"]:
            blockers.append("polyglot repository has fewer than three official structural languages")
        if documents == 0:
            blockers.append("polyglot repository has no documentation")
    if blockers:
        status = "environment_blocked"
    return {
        "class": kind,
        "status": status,
        "origin": origin,
        "revision": head,
        "license": license_identity,
        "file_count": len(files),
        "documentation_file_count": documents,
        "official_structural_languages": languages,
        "blockers": blockers,
    }


def load_repository_specs(path: Path, candidate_head: str, definition: dict[str, Any]) -> tuple[dict[str, dict[str, Any]], list[dict[str, Any]]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    repositories = value.get("repositories")
    if not isinstance(repositories, list):
        raise ValueError("repository manifest requires a repositories list")
    by_class = {item.get("class"): item for item in repositories if isinstance(item, dict)}
    if tuple(item.get("class") for item in repositories) != CLASSES or set(by_class) != set(CLASSES):
        raise ValueError("repository manifest must contain the three ordered Phase 8 classes")
    by_class["volicord"]["revision"] = candidate_head
    identities = [repository_identity(kind, by_class[kind], definition) for kind in CLASSES]
    return by_class, identities


def load_real_session_cycle(reference: Any, manifest_directory: Path) -> dict[str, Any] | None:
    if not nonempty_string(reference):
        return None
    relative = Path(reference)
    if relative.is_absolute() or ".." in relative.parts:
        raise ValueError("real-session evidence references must be relative to the repository manifest")
    evidence_path = (manifest_directory / relative).resolve()
    try:
        evidence_path.relative_to(manifest_directory.resolve())
    except ValueError as error:
        raise ValueError("real-session evidence escaped the manifest directory") from error
    value = json.loads(evidence_path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("real-session evidence must be a JSON object")
    value["_evidence_file_sha256"] = sha256(evidence_path)
    value["_evidence_directory"] = str(evidence_path.parent)
    return value


def operation_duration(step_value: dict[str, Any], *keys: str) -> float | None:
    evidence = step_value.get("evidence", {})
    for key in keys:
        operation = evidence.get(key)
        if isinstance(operation, dict) and isinstance(operation.get("duration_ms"), (int, float)):
            return float(operation["duration_ms"])
    return None


def document_measurements(step_value: dict[str, Any]) -> tuple[list[float], list[int]]:
    durations: list[float] = []
    sizes: list[int] = []
    for item in step_value.get("evidence", {}).get("documents", []):
        operation = item.get("operation", {})
        result = item.get("result", {})
        if isinstance(operation.get("duration_ms"), (int, float)):
            durations.append(float(operation["duration_ms"]))
        if isinstance(result.get("bytes"), int):
            sizes.append(result["bytes"])
    return durations, sizes


def status_from_steps(step_statuses: dict[str, str]) -> str:
    values = list(step_statuses.values())
    for status in ("failed", "environment_blocked", "partial", "unsupported", "skipped"):
        if status in values:
            return status
    return "passed" if values else "failed"


def deterministic_v11_statuses(steps: dict[str, Any]) -> dict[str, str]:
    statuses = {name: value.get("status", "failed") for name, value in steps.items()}
    connection = steps.get("codex_mcp_connection", {}).get("evidence", {})
    direct = connection.get("direct", {})
    health = direct.get("health") if isinstance(direct, dict) else None
    if isinstance(health, dict) and health.get("connection") == "connected":
        statuses["codex_mcp_connection"] = "passed"
    return statuses


def nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def looks_like_synthetic_marker(path: str) -> bool:
    lowered = Path(path).name.lower()
    return (
        lowered == "v11-ordinary-work.txt"
        or "synthetic-marker" in lowered
        or lowered.startswith("marker.")
        or lowered.endswith(".marker")
    )


def evidence_check(present: bool, valid: bool) -> str:
    if not present:
        return "partial"
    return "passed" if valid else "failed"


def valid_capture_sha256(value: Any) -> bool:
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def verified_evidence_path(
    reference: Any,
    evidence_directory: Path | None,
) -> Path | None:
    if not isinstance(reference, dict) or evidence_directory is None:
        return None
    relative_value = reference.get("file")
    expected_hash = reference.get("sha256")
    if not nonempty_string(relative_value) or not valid_capture_sha256(expected_hash):
        return None
    relative = Path(relative_value)
    if relative.is_absolute() or ".." in relative.parts:
        return None
    path = (evidence_directory / relative).resolve()
    try:
        path.relative_to(evidence_directory.resolve())
    except ValueError:
        return None
    if not path.is_file() or sha256(path) != expected_hash:
        return None
    return path


def unique_call(capture: CodexCapture | None, operation: str) -> ToolCall | None:
    if capture is None:
        return None
    calls = capture.calls(operation)
    return calls[0] if len(calls) == 1 else None


def decision_facts(
    work: CodexCapture | None,
    bundle: CanonicalBundle | None,
) -> tuple[bool, str | None, str | None, int | None, str | None]:
    call = unique_call(work, "decision_record")
    if call is None or work is None or bundle is None:
        return False, None, None, None, None
    turn = work.turn_for_call(call)
    question_id = call.arguments.get("question_id")
    revision = call.arguments.get("question_revision")
    user_text = call.arguments.get("user_turn")
    source_id = call.result.get("user_response_source_id")
    all_succeeded = call.result.get("all_succeeded") is True
    if (
        turn is None
        or not nonempty_string(question_id)
        or not isinstance(revision, int)
        or revision < 1
        or not nonempty_string(user_text)
        or turn.text != user_text
        or not nonempty_string(source_id)
        or not all_succeeded
        or call.arguments.get("project_id") != bundle.project_id
    ):
        return False, None, None, None, None

    source = bundle.one("sources", id=source_id, project_id=bundle.project_id)
    response = bundle.one(
        "question_response_sources",
        project_id=bundle.project_id,
        question_id=question_id,
        question_revision=revision,
        source_id=source_id,
    )
    decisions = [
        row
        for row in bundle.rows("decisions")
        if row.get("project_id") == bundle.project_id
        and row.get("question_id") == question_id
        and row.get("question_revision") == revision
        and row.get("user_turn_source_id") == source_id
        and row.get("user_authority") == "current_host_user_turn"
    ]
    if source is None or response is None or len(decisions) != 1:
        return False, None, None, None, None
    decision_id = decisions[0].get("id")
    witness = bundle.one(
        "question_decision_history_witnesses",
        project_id=bundle.project_id,
        question_id=question_id,
        question_revision=revision,
        root_decision_id=decision_id,
        response_source_id=source_id,
        response_authority="current_host_user_turn",
    )
    valid_source = (
        source.get("source_kind") == "current_host_user_turn"
        and source.get("locator") == turn.text
        and source.get("detail_one") == "codex"
        and source.get("detail_two") == work.session_id
        and source.get("actor_kind") == "user"
    )
    return (
        valid_source and witness is not None and nonempty_string(decision_id),
        str(decision_id) if nonempty_string(decision_id) else None,
        str(question_id),
        revision,
        str(source_id),
    )


def checkpoint_facts(
    work: CodexCapture | None,
    bundle: CanonicalBundle | None,
    decision_id: str | None,
) -> tuple[bool, str | None, list[str], str | None]:
    call = unique_call(work, "checkpoint_record")
    if call is None or work is None or bundle is None:
        return False, None, [], None
    checkpoint_id = call.result.get("checkpoint_id")
    source_id = call.result.get("user_response_source_id")
    checkpoint = bundle.one("checkpoints", id=checkpoint_id, project_id=bundle.project_id)
    turn = work.turn_for_call(call)
    source = bundle.one("sources", id=source_id, project_id=bundle.project_id)
    if checkpoint is None or source is None or turn is None or not nonempty_string(source_id):
        return False, None, [], None
    changed_paths = decode_string_blob(checkpoint.get("changed_paths"))
    observed_paths = work.paths_before(call.sequence)
    bounded_paths = (
        changed_paths
        if changed_paths
        and all(
            not Path(path).is_absolute()
            and ".." not in Path(path).parts
            and path == Path(path).as_posix()
            for path in changed_paths
        )
        else None
    )
    changed_sources = {
        row.get("source_id")
        for row in bundle.rows("checkpoint_source_relations")
        if row.get("project_id") == bundle.project_id
        and row.get("checkpoint_id") == checkpoint_id
        and row.get("relation_kind") == "changed_basis"
    }
    source_paths = {
        row.get("locator")
        for row in bundle.rows("sources")
        if row.get("id") in changed_sources and row.get("source_kind") == "file"
    }
    supported = bundle.one(
        "checkpoint_source_relations",
        project_id=bundle.project_id,
        checkpoint_id=checkpoint_id,
        relation_kind="supported_by",
        source_id=source_id,
    )
    decision_link = bundle.one(
        "checkpoint_decisions",
        project_id=bundle.project_id,
        checkpoint_id=checkpoint_id,
        decision_id=decision_id,
    )
    next_step = checkpoint.get("next_step")
    valid = (
        bounded_paths is not None
        and set(bounded_paths) == set(observed_paths)
        and set(bounded_paths) == source_paths
        and supported is not None
        and decision_link is not None
        and source.get("source_kind") == "current_host_user_turn"
        and source.get("locator") == turn.text == call.arguments.get("user_turn")
        and source.get("detail_one") == "codex"
        and source.get("detail_two") == work.session_id
        and source.get("actor_kind") == "user"
        and call.arguments.get("project_id") == bundle.project_id
        and call.arguments.get("next_step") == next_step
        and nonempty_string(next_step)
    )
    return valid, str(checkpoint_id) if nonempty_string(checkpoint_id) else None, observed_paths, str(next_step) if nonempty_string(next_step) else None


def real_session_evidence(
    raw: Any,
    *,
    kind: str,
    cycle: int,
    repository_revision: str | None,
) -> dict[str, Any]:
    if not isinstance(raw, dict):
        return {
            "evidence_class": "actual_repository_real_session",
            "status": "partial",
            "checks": {name: "partial" for name in REAL_SESSION_CHECKS},
            "basis": "externally supplied sanitized real-session evidence was absent",
        }

    evidence_directory_value = raw.get("_evidence_directory")
    evidence_directory = (
        Path(evidence_directory_value) if nonempty_string(evidence_directory_value) else None
    )
    captures = raw.get("captures") if isinstance(raw.get("captures"), dict) else {}
    work_reference = captures.get("work")
    resume_reference = captures.get("resume")
    bundle_reference = raw.get("canonical_bundle")
    work_path = verified_evidence_path(work_reference, evidence_directory)
    resume_path = verified_evidence_path(resume_reference, evidence_directory)
    bundle_path = verified_evidence_path(bundle_reference, evidence_directory)
    references_present = all(isinstance(value, dict) for value in (work_reference, resume_reference, bundle_reference))

    work_capture: CodexCapture | None = None
    resume_capture: CodexCapture | None = None
    bundle: CanonicalBundle | None = None
    try:
        work_capture = load_codex_capture(work_path) if work_path else None
        resume_capture = load_codex_capture(resume_path) if resume_path else None
        bundle = load_canonical_bundle(bundle_path) if bundle_path else None
    except (OSError, EvidenceError):
        work_capture = None
        resume_capture = None
        bundle = None

    decision_ok, decision_id, question_id, question_revision, user_source_id = decision_facts(
        work_capture, bundle
    )
    checkpoint_ok, checkpoint_id, changed_paths, next_step = checkpoint_facts(
        work_capture, bundle, decision_id
    )
    checkpoint_call = unique_call(work_capture, "checkpoint_record")
    first_work_change = min(
        (item.sequence for item in work_capture.path_observations),
        default=None,
    ) if work_capture else None
    ordinary_ok = (
        checkpoint_call is not None
        and bool(changed_paths)
        and not all(looks_like_synthetic_marker(path) for path in changed_paths)
        and not all(Path(path).suffix.lower() in {".txt", ".marker"} for path in changed_paths)
        and all(item.sequence < checkpoint_call.sequence for item in work_capture.path_observations)
    ) if work_capture else False
    metadata_ok = (
        raw.get("kind") == "phase8_real_session_cycle_evidence"
        and raw.get("producer") == "volicord_phase8_codex_event_normalizer"
        and valid_capture_sha256(raw.get("_evidence_file_sha256"))
        and raw.get("repository_class") == kind
        and raw.get("cycle") == cycle
        and raw.get("repository_revision") == repository_revision
        and work_capture is not None
        and work_capture.git_revision == repository_revision
        and ordinary_ok
    )
    baseline_ok = (
        work_capture is not None
        and first_work_change is not None
        and work_capture.git_revision == repository_revision
        and work_capture.clean_git_status_before(first_work_change)
    )

    invocations_ok = (
        work_capture is not None
        and resume_capture is not None
        and work_capture.session_id != resume_capture.session_id
        and work_capture.source in {"cli", "vscode"}
        and resume_capture.source in {"cli", "vscode"}
    )
    recall_call = unique_call(resume_capture, "recall")
    first_inspection = (
        resume_capture.first_inspection_after(recall_call.completion_sequence)
        if resume_capture is not None and recall_call is not None
        else None
    )
    continuation_paths = (
        resume_capture.paths_after(first_inspection)
        if resume_capture is not None and first_inspection is not None
        else []
    )
    prior_inspections = []
    if resume_capture is not None and recall_call is not None:
        prior_inspections = [
            sequence
            for sequence in [
                *(call.sequence for call in resume_capture.calls("repository_understanding")),
                *(
                    command.sequence
                    for command in resume_capture.commands
                    if command_is_repository_inspection(command.parsed_command)
                ),
            ]
            if sequence < recall_call.sequence
        ]
    ordering_ok = (
        recall_call is not None
        and first_inspection is not None
        and not prior_inspections
        and bool(continuation_paths)
        and recall_call.completion_sequence < first_inspection
    )
    turns_before_recall = (
        [turn for turn in resume_capture.user_turns if turn.sequence < recall_call.sequence]
        if resume_capture is not None and recall_call is not None
        else []
    )
    tasks_before_recall = (
        [sequence for sequence in resume_capture.task_sequences if sequence < recall_call.sequence]
        if resume_capture is not None and recall_call is not None
        else []
    )
    fresh_ok = (
        resume_capture is not None
        and resume_capture.fresh_user_thread
        and len(turns_before_recall) == 1
        and len(tasks_before_recall) == 1
    )

    recalled_checkpoint_row = recalled_checkpoint(bundle, recall_call.result) if bundle and recall_call else None
    recalled_decisions = recalled_decision_ids(recall_call.result) if recall_call else None
    recalled_context = relevant_context_ids(bundle, recall_call.result) if bundle and recall_call else None
    checkpoint_decisions = {
        row.get("decision_id")
        for row in bundle.rows("checkpoint_decisions")
        if bundle is not None and row.get("checkpoint_id") == checkpoint_id
    } if bundle else set()
    recall_match_ok = (
        recall_call is not None
        and recall_call.result.get("read_only") is True
        and recalled_checkpoint_row is not None
        and recalled_checkpoint_row.get("id") == checkpoint_id
        and recalled_decisions is not None
        and checkpoint_decisions
        and checkpoint_decisions <= set(recalled_decisions)
        and recalled_context is not None
        and bool(recalled_context)
    )
    continuation_ok = (
        recall_match_ok
        and nonempty_string(next_step)
        and bool(continuation_paths)
        and any(path in next_step for path in continuation_paths)
        and not all(looks_like_synthetic_marker(path) for path in continuation_paths)
    )

    checks = {
        "repository_specific_objective": evidence_check(references_present, metadata_ok),
        "clean_bounded_baseline": evidence_check(references_present, baseline_ok),
        "meaningful_ordinary_changes": evidence_check(references_present, ordinary_ok),
        "source_grounded_checkpoint": evidence_check(references_present, checkpoint_ok),
        "explicit_user_decision_source": evidence_check(references_present, decision_ok),
        "distinct_work_and_resume_invocations": evidence_check(references_present, invocations_ok),
        "fresh_resume_without_prior_context": evidence_check(references_present, fresh_ok),
        "recall_precedes_inspection_and_continuation": evidence_check(references_present, ordering_ok),
        "recall_matches_checkpoint_decision_and_context": evidence_check(references_present, recall_match_ok),
        "meaningful_recalled_continuation": evidence_check(references_present, continuation_ok),
    }
    return {
        "evidence_class": "actual_repository_real_session",
        "status": status_from_steps(checks),
        "checks": checks,
        "changed_paths": changed_paths or [],
        "continuation_paths": continuation_paths,
        "checkpoint_id": checkpoint_id,
        "decision_id": decision_id,
        "question_id": question_id,
        "question_revision": question_revision,
        "user_response_source_id": user_source_id,
        "recalled_context_ids": recalled_context or [],
        "work_session_id": work_capture.session_id if work_capture else None,
        "resume_session_id": resume_capture.session_id if resume_capture else None,
        "capture_sha256": {
            "work": work_capture.source_sha256 if work_capture else None,
            "resume": resume_capture.source_sha256 if resume_capture else None,
            "canonical_bundle": bundle.source_sha256 if bundle else None,
        },
        "evidence_origin": "repository_normalized_codex_rollout_and_canonical_bundle",
    }


def quality_observations(step_statuses: dict[str, str]) -> dict[str, dict[str, str]]:
    routes = {
        "context_recovery_accuracy": ("restart_recall",),
        "decision_repetition": ("inquiry_decision", "restart_recall"),
        "question_relevance": ("candidate_boundary", "inquiry_decision"),
        "decision_comprehension": ("inquiry_decision",),
        "source_grounding": ("source_grounded_understanding", "checkpoint", "document_outputs"),
        "capability_honesty": ("repository_analysis", "parser_failure", "provider_failure"),
        "coverage": ("repository_analysis", "source_grounded_understanding"),
        "memory_correctability": ("correction_supersession_deletion",),
        "interruption_cost": ("ordinary_work", "guarded_boundary"),
        "document_fidelity_and_usefulness": ("document_outputs",),
        "portability": ("portable_clone", "divergent_conflict"),
        "recovery": ("provider_failure", "parser_failure", "derived_index_recovery"),
    }
    result: dict[str, dict[str, str]] = {}
    for name, steps in routes.items():
        routed = {step: step_statuses.get(step, "skipped") for step in steps}
        status = status_from_steps(routed)
        if name in {"question_relevance", "decision_comprehension", "document_fidelity_and_usefulness", "interruption_cost"} and status == "passed":
            status = "partial"
        result[name] = {
            "status": status,
            "basis": ",".join(f"{key}:{value}" for key, value in routed.items()),
        }
    return result


class AccessibilityParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.html_lang: str | None = None
        self.heading_levels: list[int] = []
        self.controls = 0
        self.labels = 0
        self.links = 0
        self.viewport = False
        self.styles: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attributes = dict(attrs)
        if tag == "html":
            self.html_lang = attributes.get("lang")
        if tag in {"h1", "h2", "h3", "h4", "h5", "h6"}:
            self.heading_levels.append(int(tag[1]))
        if tag in {"input", "select", "textarea", "button"}:
            self.controls += 1
        if tag == "label":
            self.labels += 1
        if tag == "a" and attributes.get("href"):
            self.links += 1
        if tag == "meta" and attributes.get("name") == "viewport":
            self.viewport = True

    def handle_data(self, data: str) -> None:
        if ":root{" in data or "@media" in data or ":focus" in data:
            self.styles.append(data)


def parse_accessibility_html(content: str, *, expected_language: str | None) -> dict[str, Any]:
    parser = AccessibilityParser()
    parser.feed(content)
    style = "\n".join(parser.styles)
    heading_order = all(next_level <= current + 1 for current, next_level in zip(parser.heading_levels, parser.heading_levels[1:]))
    checks = {
        "keyboard_reachability": "passed" if parser.links + parser.controls > 0 else "partial",
        "visible_focus": "passed" if re.search(r":focus(?:-visible)?", style) else "partial",
        "not_color_only": "partial",
        "headings_and_labels": "passed" if parser.heading_levels and heading_order and parser.labels >= parser.controls else "partial",
        "narrow_and_zoomed_presentation": "partial" if parser.viewport else "failed",
        "document_html_language": "passed" if expected_language is None or parser.html_lang == expected_language else "failed",
    }
    return {
        "checks": checks,
        "html_language": parser.html_lang,
        "heading_count": len(parser.heading_levels),
        "control_count": parser.controls,
        "label_count": parser.labels,
        "viewport": parser.viewport,
    }


def qualify_accessibility(
    machine_result: dict[str, Any],
    observations: dict[str, Any] | None,
    permitted: set[str],
) -> dict[str, Any]:
    machine_checks = dict(machine_result.get("checks", {}))
    checks = dict(machine_checks)
    qualified: dict[str, dict[str, str]] = {}
    for name, observation in (observations or {}).items():
        if name not in permitted or not isinstance(observation, dict):
            raise ValueError(f"unsupported accessibility observation: {name}")
        status = observation.get("status")
        basis = observation.get("basis")
        if status not in ALLOWED_STATUS or not nonempty_string(basis):
            raise ValueError(f"accessibility observation needs a status and bounded basis: {name}")
        if name not in machine_checks:
            qualified[name] = {
                "machine_status": "absent",
                "observation_status": status,
                "effective_status": machine_result.get("status", "failed"),
                "basis": basis,
            }
            continue
        machine_status = machine_checks[name]
        effective = machine_status
        if status == "failed":
            effective = "failed"
        elif status == "passed" and machine_status == "partial":
            effective = "passed"
        checks[name] = effective
        qualified[name] = {
            "machine_status": machine_status,
            "observation_status": status,
            "effective_status": effective,
            "basis": basis,
        }
    barrier = machine_result.get("status")
    effective_inputs = dict(checks)
    if barrier in {"failed", "environment_blocked", "unsupported", "skipped"}:
        effective_inputs["machine_availability"] = barrier
    return {
        **machine_result,
        "status": status_from_steps(effective_inputs),
        "checks": checks,
        "machine_checks": machine_checks,
        "observations": qualified,
    }


def qualify_quality_observation(
    machine: dict[str, str],
    observation: dict[str, Any],
    name: str,
) -> dict[str, str]:
    if observation.get("status") not in ALLOWED_STATUS:
        raise ValueError(f"manual Phase 8 observation has an invalid status: {name}")
    basis = observation.get("basis")
    if not nonempty_string(basis):
        raise ValueError(f"manual Phase 8 observation needs a bounded basis: {name}")
    machine_status = machine.get("status", "failed")
    observed_status = observation["status"]
    effective = machine_status
    if observed_status == "failed":
        effective = "failed"
    elif observed_status == "passed" and machine_status in {"passed", "partial"}:
        effective = "passed"
    return {
        "status": effective,
        "basis": f"machine={machine_status}; observation={observed_status}; {basis}",
    }


def exchange_http(address: str, target: str) -> str:
    host, port = address.rsplit(":", 1)
    with socket.create_connection((host, int(port)), timeout=5) as connection:
        connection.settimeout(5)
        request = f"GET {target} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        connection.sendall(request.encode("ascii"))
        chunks: list[bytes] = []
        while True:
            chunk = connection.recv(65536)
            if not chunk:
                break
            chunks.append(chunk)
    response = b"".join(chunks).decode("utf-8", errors="replace")
    return response.split("\r\n\r\n", 1)[1] if "\r\n\r\n" in response else response


def viewer_start_failure_status(message: str) -> str:
    lowered = message.lower()
    if "operation not permitted" in lowered or "permission denied" in lowered:
        return "environment_blocked"
    return "failed"


def live_viewer_accessibility(target_root: Path, project_id: str) -> dict[str, Any]:
    results: dict[str, Any] = {}
    for locale in ("en", "ko"):
        argv = [
            str(target_root / "prefix/bin/volicord-viewer"),
            "--runtime", str(target_root / "runtime"),
            "--project", project_id,
            "--bind", "127.0.0.1:0",
            "--locale", locale,
            "--level", "deep",
            "--language", locale,
        ]
        process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        try:
            assert process.stderr is not None
            startup = process.stderr.readline().strip()
            marker = "Volicord local viewer: http://"
            if not startup.startswith(marker):
                results[locale] = {
                    "status": viewer_start_failure_status(startup),
                    "reason": "viewer_start_failed",
                }
                continue
            address = startup[len(marker):].rstrip("/")
            content = exchange_http(address, "/?level=deep")
            results[locale] = {"status": "passed", **parse_accessibility_html(content, expected_language=locale)}
        except (AssertionError, OSError, ValueError) as error:
            results[locale] = {"status": "failed", "reason": type(error).__name__}
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
    checks: dict[str, str] = {}
    for locale_result in results.values():
        for name, status in locale_result.get("checks", {}).items():
            if checks.get(name) == "failed" or status == "failed":
                checks[name] = "failed"
            elif checks.get(name) == "partial" or status == "partial":
                checks[name] = "partial"
            else:
                checks[name] = status
    locale_status = status_from_steps({locale: value.get("status", "failed") for locale, value in results.items()})
    checks["korean_english_fixed_ui"] = locale_status
    return {"status": status_from_steps(checks), "checks": checks, "locales": results}


def sanitized_cycle(
    kind: str,
    cycle: int,
    raw: dict[str, Any],
    cycle_root: Path,
    duration_ms: float,
    repository_revision: str | None,
    real_session_raw: dict[str, Any] | None,
    manual_observations: dict[str, Any] | None = None,
    accessibility_observations: dict[str, Any] | None = None,
) -> dict[str, Any]:
    steps = raw.get("steps", {})
    step_statuses = deterministic_v11_statuses(steps)
    document_durations, document_sizes = document_measurements(steps.get("document_outputs", {}))
    target_root = cycle_root / "work" / kind
    bundle = target_root / "base.volicord.json"
    analysis_store = target_root / "runtime/analysis"
    metrics = {
        "cycle_duration_ms": round(duration_ms, 3),
        "inventory_analysis_duration_ms": operation_duration(steps.get("repository_analysis", {}), "operation"),
        "recall_duration_ms": operation_duration(steps.get("restart_recall", {}), "cli_operation"),
        "document_generation_duration_ms": document_durations,
        "repair_reindex_duration_ms": operation_duration(steps.get("derived_index_recovery", {}), "repair_operation"),
        "document_output_bytes": document_sizes,
        "portable_bundle_bytes": bundle.stat().st_size if bundle.is_file() else None,
        "runtime_home_bytes": directory_bytes(target_root / "runtime"),
        "derived_state_bytes": directory_bytes(analysis_store),
        "peak_memory_bytes": None,
        "peak_memory_status": "unsupported",
    }
    accessibility = (
        live_viewer_accessibility(target_root, raw["project_id"])
        if raw.get("project_id") and (target_root / "prefix/bin/volicord-viewer").is_file()
        else {"status": "skipped", "checks": {}, "reason": "viewer prerequisite failed"}
    )
    generated_html = sorted((target_root / "documents").glob("*.html"))
    generated_language = [
        parse_accessibility_html(path.read_text(encoding="utf-8"), expected_language="en")
        for path in generated_html
    ]
    accessibility.setdefault("checks", {})["document_html_language"] = (
        "passed"
        if generated_language
        and all(item["checks"]["document_html_language"] == "passed" for item in generated_language)
        else "failed"
    )
    accessibility["status"] = status_from_steps(accessibility["checks"])
    accessibility = qualify_accessibility(
        accessibility,
        accessibility_observations,
        set(load_definition()["permitted_accessibility_observations"]),
    )
    quality = quality_observations(step_statuses)
    subjective = {
        "question_relevance",
        "decision_comprehension",
        "interruption_cost",
        "document_fidelity_and_usefulness",
    }
    for name, observation in (manual_observations or {}).items():
        if name not in subjective or not isinstance(observation, dict):
            raise ValueError(f"unsupported manual Phase 8 observation: {name}")
        quality[name] = qualify_quality_observation(quality[name], observation, name)
    actual = real_session_evidence(
        real_session_raw,
        kind=kind,
        cycle=cycle,
        repository_revision=repository_revision,
    )
    deterministic_status = status_from_steps(step_statuses)
    return {
        "cycle": cycle,
        "status": status_from_steps(
            {
                "deterministic_v11": deterministic_status,
                "real_session_dogfood": actual["status"],
            }
        ),
        "project_identity": raw.get("project_id"),
        "repository_revision": raw.get("identity", {}).get("revision"),
        "legacy_runtime_untouched": raw.get("legacy_runtime_untouched"),
        "step_statuses": step_statuses,
        "step_status_counts": dict(sorted(Counter(step_statuses.values()).items())),
        "deterministic_v11": {
            "evidence_class": "deterministic_product_path_regression",
            "status": deterministic_status,
            "scripted_inquiry_and_decision": True,
            "synthetic_ordinary_work": True,
            "qualifies_as_real_session_dogfood": False,
        },
        "real_session_dogfood": actual,
        "quality_observations": quality,
        "measurements": metrics,
        "accessibility": accessibility,
    }


def aggregate_accessibility(repositories: list[dict[str, Any]]) -> dict[str, Any]:
    checks: dict[str, str] = {}
    for repository in repositories:
        for cycle in repository.get("cycles", []):
            for name, status in cycle.get("accessibility", {}).get("checks", {}).items():
                current = checks.get(name)
                if current == "failed" or status == "failed":
                    checks[name] = "failed"
                elif current == "partial" or status == "partial":
                    checks[name] = "partial"
                else:
                    checks[name] = status
    return {
        "status": status_from_steps(checks),
        "checks": checks,
        "limits": [
            "No standards certification or human-subject accessibility qualification was performed.",
            "Narrow and zoom behavior combines live HTML/viewport structure with a bounded operator observation; no browser layout engine or standards certification was used.",
        ],
    }


def detect_revisit_triggers(repositories: list[dict[str, Any]], accessibility: dict[str, Any]) -> list[dict[str, str]]:
    triggers: list[dict[str, str]] = []
    failed_checks = sorted(name for name, status in accessibility.get("checks", {}).items() if status == "failed")
    if failed_checks:
        triggers.append({"decision_id": "Q5", "basis": "generated/viewer HTML accessibility blocker: " + ", ".join(failed_checks)})
    for repository in repositories:
        for cycle in repository.get("cycles", []):
            if cycle.get("step_statuses", {}).get("inquiry_decision") == "failed":
                triggers.append({"decision_id": "Q1", "basis": f"{repository['class']} cycle {cycle['cycle']} inquiry/Decision failure"})
            if cycle.get("step_statuses", {}).get("repository_analysis") == "failed":
                triggers.append({"decision_id": "Q2", "basis": f"{repository['class']} cycle {cycle['cycle']} repository analysis failure"})
    unique = {(item["decision_id"], item["basis"]): item for item in triggers}
    return [unique[key] for key in sorted(unique)]


def sanitize_check(value: Any) -> None:
    encoded = json.dumps(value, sort_keys=True).lower()
    if any(marker in encoded for marker in SECRET_MARKERS):
        raise ValueError("sanitized result contains a prohibited secret/private marker")
    if re.search(r"/(?:home|tmp)/[^\s\"']+", encoded):
        raise ValueError("sanitized result contains an absolute local path")


def validate_result(result: dict[str, Any], definition: dict[str, Any]) -> None:
    if result.get("kind") != "phase8_dogfood_result":
        raise ValueError("unexpected dogfood result kind")
    if not re.fullmatch(r"[0-9a-f]{40}", result.get("candidate_head", "")):
        raise ValueError("dogfood result has no exact candidate HEAD")
    repositories = result.get("repositories", [])
    if [item.get("class") for item in repositories] != list(CLASSES):
        raise ValueError("dogfood result does not contain the three ordered repository classes")
    real_invocations: list[str] = []
    for repository in repositories:
        if len(repository.get("cycles", [])) != definition["candidate_cycle_count"]:
            raise ValueError("dogfood result does not contain two cycles per repository")
        for cycle in repository["cycles"]:
            if set(cycle.get("step_statuses", {})) != set(definition["required_product_steps"]):
                raise ValueError("dogfood cycle silently dropped a maintained product step")
            statuses = set(cycle["step_statuses"].values())
            if not statuses <= ALLOWED_STATUS:
                raise ValueError("dogfood cycle contains an unknown status")
            deterministic = cycle.get("deterministic_v11", {})
            if (
                deterministic.get("evidence_class") != "deterministic_product_path_regression"
                or deterministic.get("qualifies_as_real_session_dogfood") is not False
                or deterministic.get("synthetic_ordinary_work") is not True
            ):
                raise ValueError("V11 regression was not kept separate from real-session dogfood")
            actual = cycle.get("real_session_dogfood", {})
            if actual.get("evidence_class") != "actual_repository_real_session":
                raise ValueError("dogfood cycle lacks the real-session evidence class")
            if set(actual.get("checks", {})) != set(REAL_SESSION_CHECKS):
                raise ValueError("real-session dogfood evidence checks are incomplete")
            if not set(actual["checks"].values()) <= ALLOWED_STATUS:
                raise ValueError("real-session dogfood evidence contains an unknown status")
            real_invocations.extend(
                [
                    actual.get("work_session_id"),
                    actual.get("resume_session_id"),
                ]
            )
    if result.get("replacement_pass_candidate") is True:
        if result.get("status") != "passed" or result.get("blockers"):
            raise ValueError("replacement pass cannot have a non-pass status or blocker")
        if result.get("decision_revisit", {}).get("observed_active_triggers"):
            raise ValueError("replacement pass cannot have an active Decision revisit trigger")
        if result.get("candidate_worktree") != {"clean_before": True, "clean_after": True}:
            raise ValueError("replacement pass requires a clean candidate throughout dogfood")
        if result.get("fixture_regression", {}).get("status") != "passed":
            raise ValueError("replacement pass requires current structural/fallback regression")
        if result.get("accessibility", {}).get("status") != "passed":
            raise ValueError("replacement pass requires passed accessibility evaluation")
        accessibility_checks = result.get("accessibility", {}).get("checks", {})
        if set(accessibility_checks) != set(definition["accessibility_checks"]) or any(
            status != "passed" for status in accessibility_checks.values()
        ):
            raise ValueError("replacement pass requires every accessibility check to pass")
        for repository in repositories:
            if repository.get("status") != "passed" or not repository.get("independent_fresh_runtime_cycles"):
                raise ValueError("replacement pass requires two independent passed repository cycles")
            for cycle in repository["cycles"]:
                if cycle.get("status") != "passed":
                    raise ValueError("replacement pass contains a non-pass dogfood cycle")
                if cycle.get("deterministic_v11", {}).get("status") != "passed":
                    raise ValueError("replacement pass requires passed deterministic V11 regression")
                actual = cycle.get("real_session_dogfood", {})
                if actual.get("status") != "passed" or any(
                    status != "passed" for status in actual.get("checks", {}).values()
                ):
                    raise ValueError("replacement pass requires complete real-session dogfood evidence")
                if any(
                    observation.get("status") != "passed"
                    for observation in cycle.get("quality_observations", {}).values()
                ):
                    raise ValueError("replacement pass contains an unqualified quality observation")
        if (
            any(not nonempty_string(identity) for identity in real_invocations)
            or len(set(real_invocations)) != len(real_invocations)
        ):
            raise ValueError("replacement pass requires globally distinct Codex invocations")
    sanitize_check(result)


def aggregate_status(repositories: list[dict[str, Any]], regression: dict[str, Any], accessibility: dict[str, Any], blockers: list[str]) -> str:
    statuses = [regression.get("status", "failed"), accessibility.get("status", "failed")]
    statuses.extend(repository.get("status", "failed") for repository in repositories)
    if "failed" in statuses:
        return "failed"
    if "environment_blocked" in statuses:
        return "environment_blocked"
    for status in ("partial", "unsupported", "skipped"):
        if status in statuses:
            return status
    return "environment_blocked" if blockers else "passed"


def run_fixture_regression(v11: Any, raw_root: Path, base_env: dict[str, str], definition: dict[str, Any]) -> dict[str, Any]:
    recorder = v11.Recorder(raw_root / "fixture-regression")
    command = definition["fixture_regression_command"]
    result = recorder.run("official-structural-and-fallback", command, base_env, cwd=ROOT, timeout=900)
    return {
        "status": "passed" if result.get("exit_code") == 0 else "failed",
        "command": command,
        "duration_ms": result.get("duration_ms"),
        "covers_official_structural_fixtures": True,
        "covers_out_of_set_fallback_fixture": True,
        "fixtures_are_not_real_repository_substitutes": True,
    }


def run_evaluation(args: argparse.Namespace) -> int:
    definition = load_definition()
    candidate_head = git_head(ROOT)
    if candidate_head is None or candidate_head != args.candidate_head:
        raise RuntimeError("candidate HEAD does not match --candidate-head")
    clean_before = git_clean(ROOT)
    repository_manifest = Path(args.repositories).resolve()
    specs, identities = load_repository_specs(repository_manifest, candidate_head, definition)
    output = Path(args.output_dir).resolve()
    if output.exists() and any(output.iterdir()):
        raise RuntimeError("Phase 8 output directory must be absent or empty")
    output.mkdir(parents=True, exist_ok=True)
    raw_root = output / "raw"
    raw_root.mkdir()
    v11 = load_v11()
    base_env = os.environ.copy()
    base_env.setdefault("CARGO_HOME", str(Path.home() / ".cargo"))
    base_env.setdefault("RUSTUP_HOME", str(Path.home() / ".rustup"))
    original_prepare: Callable[..., Any] = v11.prepare_repository
    source_by_class = {kind: Path(specs[kind]["path"]).resolve() for kind in CLASSES}

    def prepare_actual(kind: str, destination: Path, recorder: Any, env: dict[str, str]) -> dict[str, str]:
        source = source_by_class[kind]
        result = recorder.run(
            "clone-actual-repository",
            ["git", "clone", "--quiet", "--no-hardlinks", str(source), str(destination)],
            env,
            timeout=300,
        )
        if result.get("exit_code") != 0:
            raise RuntimeError("actual repository disposable clone failed")
        return {
            "revision": v11.git_revision(recorder, destination, env),
            "content_sha256": v11.tree_hash(destination),
        }

    v11.prepare_repository = prepare_actual
    for kind in CLASSES:
        v11.PROVIDER_SOURCE_PATHS[kind] = specs[kind]["provider_source_path"]
    started_at = utc_now()
    started = time.monotonic_ns()
    repository_results: list[dict[str, Any]] = []
    try:
        for identity in identities:
            kind = identity["class"]
            cycles: list[dict[str, Any]] = []
            if identity["status"] == "passed":
                for cycle_number in range(1, definition["candidate_cycle_count"] + 1):
                    cycle_root = raw_root / f"{kind}-cycle-{cycle_number}"
                    cycle_root.mkdir()
                    recorder = v11.Recorder(cycle_root)
                    cycle_started = time.monotonic_ns()
                    try:
                        raw = v11.rehearse_target(kind, cycle_root, recorder, base_env, None)
                    except (AssertionError, OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
                        raw = {
                            "class": kind,
                            "identity": {"revision": identity["revision"]},
                            "steps": {
                                name: v11.step("failed" if name == "clean_install" else "skipped", type(error).__name__)
                                for name in definition["required_product_steps"]
                            },
                        }
                    write_json(cycle_root / "raw-cycle-result.json", raw)
                    cycles.append(sanitized_cycle(
                        kind,
                        cycle_number,
                        raw,
                        cycle_root,
                        (time.monotonic_ns() - cycle_started) / 1_000_000,
                        identity["revision"],
                        load_real_session_cycle(
                            specs[kind].get("real_session_evidence", {}).get(str(cycle_number)),
                            repository_manifest.parent,
                        ),
                        specs[kind].get("manual_observations", {}).get(str(cycle_number)),
                        specs[kind].get("accessibility_observations", {}).get(str(cycle_number)),
                    ))
            else:
                for cycle_number in range(1, definition["candidate_cycle_count"] + 1):
                    skipped = {name: "environment_blocked" for name in definition["required_product_steps"]}
                    actual = real_session_evidence(
                        load_real_session_cycle(
                            specs[kind].get("real_session_evidence", {}).get(str(cycle_number)),
                            repository_manifest.parent,
                        ),
                        kind=kind,
                        cycle=cycle_number,
                        repository_revision=identity["revision"],
                    )
                    cycles.append({
                        "cycle": cycle_number,
                        "status": "environment_blocked",
                        "project_identity": None,
                        "repository_revision": identity["revision"],
                        "legacy_runtime_untouched": None,
                        "step_statuses": skipped,
                        "step_status_counts": {"environment_blocked": len(skipped)},
                        "deterministic_v11": {
                            "evidence_class": "deterministic_product_path_regression",
                            "status": "environment_blocked",
                            "scripted_inquiry_and_decision": True,
                            "synthetic_ordinary_work": True,
                            "qualifies_as_real_session_dogfood": False,
                        },
                        "real_session_dogfood": actual,
                        "quality_observations": quality_observations(skipped),
                        "measurements": {
                            name: None for name in definition["measurements"]
                        },
                        "accessibility": {"status": "skipped", "checks": {}},
                    })
            cycle_status = status_from_steps({str(item["cycle"]): item["status"] for item in cycles})
            independent = len({item.get("project_identity") for item in cycles if item.get("project_identity")}) == len(cycles)
            if not independent and cycle_status == "passed":
                cycle_status = "failed"
            repository_results.append({
                **identity,
                "status": cycle_status if identity["status"] == "passed" else identity["status"],
                "independent_fresh_runtime_cycles": independent,
                "cycles": cycles,
            })
    finally:
        v11.prepare_repository = original_prepare
    regression = run_fixture_regression(v11, raw_root, base_env, definition)
    accessibility = aggregate_accessibility(repository_results)
    try:
        maintained_decisions = v11.read_decision_revisit_assessment(DECISION_REGISTER)
    except (OSError, ValueError):
        maintained_decisions = v11.failed_decision_revisit_assessment(DECISION_REGISTER)
    observed_triggers = detect_revisit_triggers(repository_results, accessibility)
    clean_after = git_clean(ROOT)
    blockers: list[str] = []
    blockers.extend(
        f"{identity['class']}: {blocker}" for identity in identities for blocker in identity["blockers"]
    )
    if not clean_before or not clean_after:
        blockers.append("candidate worktree was not clean for the complete dogfood run")
    if maintained_decisions.get("decision_revisit_trigger_assessment") != v11.OFFICIAL_REVISIT_ASSESSMENT:
        blockers.append("the accepted Decision register could not be assessed")
    if maintained_decisions.get("active_decision_revisit_triggers"):
        blockers.append("the accepted Decision register reports an active revisit trigger")
    if observed_triggers:
        blockers.append("dogfood evidence activates one or more accepted Decision revisit triggers")
    if accessibility.get("status") != "passed":
        blockers.append("accessibility evaluation has a blocker or unqualified criterion")
    for repository in repository_results:
        if repository["status"] != "passed":
            blockers.append(f"{repository['class']} repeated journey did not pass")
        for cycle in repository["cycles"]:
            if cycle.get("real_session_dogfood", {}).get("status") != "passed":
                blockers.append(
                    f"{repository['class']} cycle {cycle['cycle']} lacks qualifying real-session dogfood evidence"
                )
            incomplete_quality = sorted(
                name
                for name, observation in cycle["quality_observations"].items()
                if observation.get("status") != "passed"
            )
            if incomplete_quality:
                blockers.append(
                    f"{repository['class']} cycle {cycle['cycle']} has unqualified quality observations: "
                    + ", ".join(incomplete_quality)
                )
    if regression["status"] != "passed":
        blockers.append("maintained structural/fallback regression did not pass")
    status = aggregate_status(repository_results, regression, accessibility, blockers)
    replacement_pass_candidate = status == "passed" and not blockers
    result = {
        "kind": "phase8_dogfood_result",
        "candidate_head": candidate_head,
        "definition_sha256": sha256(DEFINITION),
        "started_at": started_at,
        "ended_at": utc_now(),
        "duration_ms": round((time.monotonic_ns() - started) / 1_000_000, 3),
        "environment": {
            "operating_system": platform.system(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "candidate_worktree": {"clean_before": clean_before, "clean_after": clean_after},
        "status": status,
        "replacement_pass_candidate": replacement_pass_candidate,
        "blockers": blockers,
        "repositories": repository_results,
        "fixture_regression": regression,
        "accessibility": accessibility,
        "privacy_and_transmission": {
            "evidence_mode": definition["real_session_evidence"]["mode"],
            "harness_performed_or_authorized_codex_transmission": False,
            "verified_external_codex_session_count_expected": 12,
            "task_relevant_repository_content_may_have_been_transmitted_by_external_evidence_producers": True,
            "commercial_semantic_provider_success_claimed": False,
            "raw_source_in_sanitized_result": False,
            "credentials_in_sanitized_result": False,
        },
        "decision_revisit": {
            "maintained_assessment": maintained_decisions.get("decision_revisit_trigger_assessment"),
            "maintained_active_triggers": maintained_decisions.get("active_decision_revisit_triggers"),
            "observed_active_triggers": observed_triggers,
        },
        "known_limits": [
            "Question relevance, decision comprehension, interruption cost, and document usefulness include agent-observed rather than human-subject evidence.",
            "Peak memory is unsupported because the reused maintained child runner does not expose a reliable per-operation peak measurement.",
            "A successful unavailable-provider recovery path is not commercial semantic-provider qualification.",
        ],
        "raw_evidence_retention": "ignored_phase8_raw_state_only",
        "sanitization": {
            "source_bodies": "excluded",
            "private_prompts": "excluded",
            "command_logs": "excluded",
            "credentials_and_provider_payloads": "excluded",
            "local_absolute_paths": "excluded",
        },
    }
    validate_result(result, definition)
    write_json(output / "dogfood-result.json", result)
    print(json.dumps({
        "status": status,
        "replacement_pass_candidate": replacement_pass_candidate,
        "candidate_head": candidate_head,
        "blockers": blockers,
        "result": "dogfood-result.json",
    }, indent=2, sort_keys=True))
    return 0 if replacement_pass_candidate else 1


def real_session_fixture(
    kind: str,
    cycle: int,
    revision: str,
    evidence_directory: Path,
) -> dict[str, Any]:
    project = "01" * 16
    user_source = "02" * 16
    checkpoint_source = "03" * 16
    changed_source_one = "04" * 16
    changed_source_two = "05" * 16
    question = "06" * 16
    decision = "07" * 16
    context = "08" * 16
    checkpoint = "09" * 16
    work_session = f"{kind}-work-session-{cycle}"
    resume_session = f"{kind}-resume-session-{cycle}"
    decision_turn_text = "<redacted-current-host-decision>"
    checkpoint_turn_text = "<redacted-checkpoint-request>"
    next_step = "Update src/resume.rs and verify the resumed work"
    work_paths = ["src/existing.rs", "tests/existing.rs"]
    work_capture = evidence_directory / f"{kind}-{cycle}-work-events.jsonl"
    resume_capture = evidence_directory / f"{kind}-{cycle}-resume-events.jsonl"
    bundle_path = evidence_directory / f"{kind}-{cycle}-context.bundle.json"

    def event(event_type: str, payload: dict[str, Any]) -> dict[str, Any]:
        return {"timestamp": "2026-08-15T00:00:00Z", "type": event_type, "payload": payload}

    def session_meta(session: str) -> dict[str, Any]:
        return event(
            "session_meta",
            {
                "id": session,
                "session_id": session,
                "timestamp": "2026-08-15T00:00:00Z",
                "cwd": "/phase8/repository",
                "originator": "codex_cli_rs",
                "cli_version": "0.145.0",
                "source": "cli",
                "thread_source": "user",
                "model_provider": "openai",
                "history_mode": "legacy",
                "git": {"commit_hash": revision, "branch": "phase8"},
            },
        )

    def task(turn_id: str) -> dict[str, Any]:
        return event("event_msg", {"type": "task_started", "turn_id": turn_id, "started_at": 1})

    def user(turn_id: str, user_turn_id: str, text: str) -> dict[str, Any]:
        return event(
            "event_msg",
            {
                "type": "user_message",
                "client_id": user_turn_id,
                "message": text,
                "images": [],
                "local_images": [],
                "audio": [],
                "local_audio": [],
                "text_elements": [],
            },
        )

    def tool_call(turn_id: str, call_id: str, name: str, arguments: dict[str, Any]) -> dict[str, Any]:
        return event(
            "response_item",
            {
                "type": "function_call",
                "id": f"item-{call_id}",
                "call_id": call_id,
                "namespace": "mcp__volicord",
                "name": name,
                "arguments": json.dumps(arguments, separators=(",", ":")),
                "internal_chat_message_metadata_passthrough": {"turn_id": turn_id},
            },
        )

    def tool_output(turn_id: str, call_id: str, structured: dict[str, Any]) -> dict[str, Any]:
        return event(
            "response_item",
            {
                "type": "function_call_output",
                "id": f"output-{call_id}",
                "call_id": call_id,
                "output": json.dumps(
                    {
                        "content": [{"type": "text", "text": "<redacted-structured-result>"}],
                        "structuredContent": structured,
                        "isError": False,
                    },
                    separators=(",", ":"),
                ),
                "internal_chat_message_metadata_passthrough": {"turn_id": turn_id},
            },
        )

    work_turn = f"{kind}-work-turn-{cycle}"
    checkpoint_turn = f"{kind}-checkpoint-turn-{cycle}"
    decision_call = f"{kind}-decision-call-{cycle}"
    checkpoint_call = f"{kind}-checkpoint-call-{cycle}"
    work_events = [
        session_meta(work_session),
        task(work_turn),
        user(work_turn, f"{kind}-user-turn-{cycle}", decision_turn_text),
        event(
            "event_msg",
            {
                "type": "exec_command_end",
                "turn_id": work_turn,
                "call_id": f"{kind}-git-status-{cycle}",
                "process_id": f"{kind}-work-process-{cycle}",
                "parsed_cmd": [{"cmd": "git status --porcelain=v1 --untracked-files=all"}],
                "interaction_input": None,
                "aggregated_output": "",
                "exit_code": 0,
                "warning": None,
                "context": {},
            },
        ),
        tool_call(
            work_turn,
            decision_call,
            "decision_record",
            {
                "project_id": project,
                "question_id": question,
                "question_revision": 1,
                "alternative_key": "apply",
                "user_turn": decision_turn_text,
            },
        ),
        tool_output(
            work_turn,
            decision_call,
            {
                "project_id": project,
                "user_response_source_id": user_source,
                "all_succeeded": True,
                "outcomes": [{"question_id": question, "revision": 1, "outcome": "recorded"}],
            },
        ),
        event(
            "event_msg",
            {
                "type": "patch_apply_end",
                "call_id": f"{kind}-work-patch-{cycle}",
                "turn_id": work_turn,
                "stdout": "",
                "stderr": "",
                "success": True,
                "changes": [f"/phase8/repository/{path}" for path in work_paths],
                "status": "completed",
            },
        ),
        task(checkpoint_turn),
        user(checkpoint_turn, f"{kind}-checkpoint-user-turn-{cycle}", checkpoint_turn_text),
        tool_call(
            checkpoint_turn,
            checkpoint_call,
            "checkpoint_record",
            {
                "project_id": project,
                "user_turn": checkpoint_turn_text,
                "goal": "Improve the repository implementation",
                "next_step": next_step,
                "known_limits": [],
            },
        ),
        tool_output(
            checkpoint_turn,
            checkpoint_call,
            {"checkpoint_id": checkpoint, "revision": 1, "user_response_source_id": checkpoint_source},
        ),
    ]

    resume_turn = f"{kind}-resume-turn-{cycle}"
    recall_call = f"{kind}-recall-call-{cycle}"
    inspect_call = f"{kind}-inspect-call-{cycle}"
    resume_events = [
        session_meta(resume_session),
        task(resume_turn),
        user(resume_turn, f"{kind}-resume-user-turn-{cycle}", "<redacted-resume-request>"),
        tool_call(resume_turn, recall_call, "recall", {"project_id": project}),
        tool_output(
            resume_turn,
            recall_call,
            {
                "project_id": project,
                "project_name": "Phase 8 fixture",
                "goals": ["Improve the repository implementation"],
                "decisions": [{"identity": decision, "revision": 1, "state": "active", "choice": "apply", "rationale": None}],
                "open_questions": [],
                "known_limits": [],
                "next_step": next_step,
                "omitted_count": 0,
                "read_only": True,
            },
        ),
        tool_call(resume_turn, inspect_call, "repository_understanding", {"project_id": project}),
        tool_output(
            resume_turn,
            inspect_call,
            {"health": "available", "overview": {}, "repository_map": {}, "decision_context_code": [], "issues": [], "read_only": True},
        ),
        event(
            "event_msg",
            {
                "type": "patch_apply_end",
                "call_id": f"{kind}-resume-patch-{cycle}",
                "turn_id": resume_turn,
                "stdout": "",
                "stderr": "",
                "success": True,
                "changes": ["/phase8/repository/src/resume.rs"],
                "status": "completed",
            },
        ),
    ]
    work_capture.write_text("".join(json.dumps(value, separators=(",", ":")) + "\n" for value in work_events), encoding="utf-8")
    resume_capture.write_text("".join(json.dumps(value, separators=(",", ":")) + "\n" for value in resume_events), encoding="utf-8")

    def null() -> dict[str, str]:
        return {"type": "null"}

    def integer(value: int) -> dict[str, Any]:
        return {"type": "integer", "value": value}

    def text(value: str | None) -> dict[str, str]:
        return null() if value is None else {"type": "text", "value": value}

    def blob(value: str) -> dict[str, str]:
        return {"type": "bytes", "value": value}

    def encoded_strings(values: list[str]) -> str:
        raw = len(values).to_bytes(8, "big")
        for value in values:
            encoded = value.encode("utf-8")
            raw += len(encoded).to_bytes(8, "big") + encoded
        return raw.hex()

    def table(name: str, columns: list[str], rows: list[list[dict[str, Any]]]) -> dict[str, Any]:
        return {"columns": columns, "name": name, "rows": rows}

    source_columns = [
        "id", "project_id", "revision", "source_kind", "locator", "snapshot_basis",
        "detail_one", "detail_two", "exit_code", "termination", "actor_kind",
        "actor_identity", "observer_kind", "observer_identity", "availability", "recorded_at",
    ]
    sources = [
        [blob(user_source), blob(project), integer(1), text("current_host_user_turn"), text(decision_turn_text), null(), text("codex"), text(work_session), null(), null(), text("user"), text("fixture-user"), null(), null(), text("available"), integer(1)],
        [blob(checkpoint_source), blob(project), integer(1), text("current_host_user_turn"), text(checkpoint_turn_text), null(), text("codex"), text(work_session), null(), null(), text("user"), text("fixture-user"), null(), null(), text("available"), integer(2)],
        [blob(changed_source_one), blob(project), integer(1), text("file"), text(work_paths[0]), text(revision), null(), null(), null(), null(), text("repository"), text("codex-observer"), null(), null(), text("available"), integer(3)],
        [blob(changed_source_two), blob(project), integer(1), text("file"), text(work_paths[1]), text(revision), null(), null(), null(), null(), text("repository"), text("codex-observer"), null(), null(), text("available"), integer(4)],
    ]
    tables = [
        table("sources", source_columns, sources),
        table("questions", ["id", "project_id", "revision", "terminal_outcome", "created_at", "updated_at"], [[blob(question), blob(project), integer(1), text("answered"), integer(1), integer(1)]]),
        table("question_response_sources", ["project_id", "question_id", "question_revision", "source_id", "recorded_at"], [[blob(project), blob(question), integer(1), blob(user_source), integer(1)]]),
        table("question_decision_history_witnesses", ["project_id", "question_id", "question_revision", "root_decision_id", "terminal_outcome", "response_source_id", "response_authority", "creation_kind", "created_at"], [[blob(project), blob(question), integer(1), blob(decision), text("answered"), blob(user_source), text("current_host_user_turn"), text("alternative"), integer(1)]]),
        table("decisions", ["id", "project_id", "revision", "question_id", "question_revision", "user_turn_source_id", "user_authority", "choice_kind", "choice_value", "user_rationale", "displayed_alternatives", "recommendation_key", "recommendation_rationale", "recommendation_sources", "applicability_paths", "applicability_components", "applicability_work_contexts", "assumptions", "revisit_triggers", "recorded_at"], [[blob(decision), blob(project), integer(1), blob(question), integer(1), blob(user_source), text("current_host_user_turn"), text("alternative"), text("apply"), null(), blob(encoded_strings([])), text("apply"), text("fixture recommendation"), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), integer(1)]]),
        table("context_items", ["id", "project_id", "revision", "role", "statement", "provenance_role", "author_kind", "author_identity", "applicability_paths", "applicability_components", "applicability_work_contexts", "recorded_at"], [[blob(context), blob(project), integer(1), text("goal"), text("Improve the repository implementation"), text("user_statement"), text("user"), text("fixture-user"), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), integer(1)]]),
        table("context_item_sources", ["project_id", "context_item_id", "source_id", "position"], [[blob(project), blob(context), blob(user_source), integer(0)]]),
        table("checkpoints", ["id", "project_id", "revision", "checkpoint_kind", "goal", "work_state", "state_change", "changed_paths", "user_review", "user_review_source_id", "user_acceptance", "user_acceptance_source_id", "known_limits", "non_goals", "next_step", "handoff_to", "recorded_at"], [[blob(checkpoint), blob(project), integer(1), text("handoff"), text("Improve the repository implementation"), text("paused"), text("ordinary repository work"), blob(encoded_strings(work_paths)), text("not_requested"), null(), text("not_requested"), null(), blob(encoded_strings([])), blob(encoded_strings([])), text(next_step), text("next Codex session"), integer(1)]]),
        table("checkpoint_source_relations", ["project_id", "checkpoint_id", "relation_kind", "source_id", "position"], [[blob(project), blob(checkpoint), text("supported_by"), blob(checkpoint_source), integer(0)], [blob(project), blob(checkpoint), text("changed_basis"), blob(changed_source_one), integer(0)], [blob(project), blob(checkpoint), text("changed_basis"), blob(changed_source_two), integer(1)]]),
        table("checkpoint_decisions", ["project_id", "checkpoint_id", "decision_id", "position"], [[blob(project), blob(checkpoint), blob(decision), integer(0)]]),
    ]
    state = {"project_id": project, "tables": tables}
    history_basis = hashlib.sha256(json.dumps(state, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
    payload = {
        "lineage": {"common_base_basis": history_basis, "history_basis": history_basis},
        "project_id": project,
        "tables": tables,
    }
    bundle = {
        "checksum": hashlib.sha256(json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest(),
        "format_version": 6,
        "kind": "volicord-context-bundle",
        "payload": payload,
    }
    write_json(bundle_path, bundle)
    return {
        "kind": "phase8_real_session_cycle_evidence",
        "producer": "volicord_phase8_codex_event_normalizer",
        "_evidence_file_sha256": "0" * 64,
        "_evidence_directory": str(evidence_directory),
        "repository_class": kind,
        "cycle": cycle,
        "repository_revision": revision,
        "captures": {
            "work": {"file": work_capture.name, "sha256": sha256(work_capture)},
            "resume": {"file": resume_capture.name, "sha256": sha256(resume_capture)},
        },
        "canonical_bundle": {"file": bundle_path.name, "sha256": sha256(bundle_path)},
    }


def expect_rejected(result: dict[str, Any], definition: dict[str, Any], message: str) -> None:
    try:
        validate_result(result, definition)
    except ValueError:
        return
    raise AssertionError(message)


def self_test() -> int:
    definition = load_definition()
    revision = "0" * 40
    temporary = tempfile.TemporaryDirectory(prefix="volicord-phase8-self-test-")
    evidence_directory = Path(temporary.name)
    external_fixture = real_session_fixture("volicord", 1, revision, evidence_directory)
    external_fixture.pop("_evidence_file_sha256")
    external_fixture.pop("_evidence_directory")
    external_fixture_path = evidence_directory / "cycle-evidence.json"
    write_json(external_fixture_path, external_fixture)
    loaded_fixture = load_real_session_cycle(
        external_fixture_path.name,
        evidence_directory,
    )
    if real_session_evidence(
        loaded_fixture,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["status"] != "passed":
        raise AssertionError("external sanitized process evidence did not qualify")
    fake_steps = {name: "passed" for name in definition["required_product_steps"]}
    valid_html = (
        "<!doctype html><html lang=\"en\"><head>"
        "<meta name=\"viewport\" content=\"width=device-width\">"
        "<style>:focus{outline:2px solid}</style></head>"
        "<body><h1>x</h1><a href=\"/\">x</a><label>y<input></label></body></html>"
    )
    parsed = parse_accessibility_html(valid_html, expected_language="en")
    parsed["checks"]["korean_english_fixed_ui"] = "passed"
    parsed["status"] = status_from_steps(parsed["checks"])
    accessibility = qualify_accessibility(
        parsed,
        {
            "not_color_only": {
                "status": "passed",
                "basis": "Operator verified that every status includes a textual label.",
            },
            "narrow_and_zoomed_presentation": {
                "status": "passed",
                "basis": "Operator verified the live page at narrow width and browser zoom.",
            },
        },
        set(definition["permitted_accessibility_observations"]),
    )
    if accessibility["status"] != "passed":
        raise AssertionError("real parser and permitted observations did not reach accessibility pass")
    accessibility_aggregate = aggregate_accessibility(
        [{"cycles": [{"accessibility": accessibility}]}]
    )
    if accessibility_aggregate["status"] != "passed":
        raise AssertionError("qualified parser evidence did not reach aggregate accessibility pass")

    repositories = []
    for index, kind in enumerate(CLASSES):
        cycles = []
        for cycle in (1, 2):
            actual = real_session_evidence(
                real_session_fixture(kind, cycle, revision, evidence_directory),
                kind=kind,
                cycle=cycle,
                repository_revision=revision,
            )
            if actual["status"] != "passed":
                raise AssertionError("valid real-session evidence did not qualify")
            cycles.append({
                "cycle": cycle,
                "status": "passed",
                "project_identity": f"{index * 10 + cycle:032x}",
                "repository_revision": revision,
                "legacy_runtime_untouched": True,
                "step_statuses": fake_steps,
                "step_status_counts": {"passed": len(fake_steps)},
                "deterministic_v11": {
                    "evidence_class": "deterministic_product_path_regression",
                    "status": "passed",
                    "scripted_inquiry_and_decision": True,
                    "synthetic_ordinary_work": True,
                    "qualifies_as_real_session_dogfood": False,
                },
                "real_session_dogfood": actual,
                "quality_observations": {
                    name: {"status": "passed", "basis": "bounded observation"}
                    for name in definition["quality_observations"]
                },
                "measurements": {},
                "accessibility": accessibility,
            })
        repositories.append({
            "class": kind,
            "status": "passed",
            "independent_fresh_runtime_cycles": True,
            "cycles": cycles,
        })
    result = {
        "kind": "phase8_dogfood_result",
        "candidate_head": revision,
        "status": "passed",
        "replacement_pass_candidate": True,
        "blockers": [],
        "repositories": repositories,
        "candidate_worktree": {"clean_before": True, "clean_after": True},
        "fixture_regression": {"status": "passed"},
        "accessibility": accessibility_aggregate,
        "decision_revisit": {"observed_active_triggers": []},
    }
    validate_result(result, definition)

    blocked = json.loads(json.dumps(result))
    blocked["status"] = "environment_blocked"
    blocked["replacement_pass_candidate"] = False
    blocked["blockers"] = ["missing repository"]
    validate_result(blocked, definition)
    leaked = json.loads(json.dumps(blocked))
    leaked["private_prompt"] = "private prompt body"
    expect_rejected(leaked, definition, "sanitizer accepted private prompt content")
    active = json.loads(json.dumps(result))
    active["decision_revisit"]["observed_active_triggers"] = [{"decision_id": "Q5"}]
    expect_rejected(active, definition, "replacement pass accepted a Decision revisit trigger")

    v11_only = json.loads(json.dumps(result))
    v11_only["repositories"][0]["cycles"][0]["real_session_dogfood"] = real_session_evidence(
        None, kind="volicord", cycle=1, repository_revision=revision
    )
    expect_rejected(v11_only, definition, "V11-only evidence qualified as real dogfood")

    def capture_events(fixture: dict[str, Any], name: str) -> tuple[Path, list[dict[str, Any]]]:
        reference = fixture["captures"][name]
        path = evidence_directory / reference["file"]
        return path, [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]

    def store_capture(
        fixture: dict[str, Any], name: str, path: Path, events: list[dict[str, Any]]
    ) -> None:
        path.write_text(
            "".join(json.dumps(value, separators=(",", ":")) + "\n" for value in events),
            encoding="utf-8",
        )
        fixture["captures"][name]["sha256"] = sha256(path)

    def mutate_bundle(fixture: dict[str, Any], mutation: Callable[[dict[str, Any]], None]) -> None:
        reference = fixture["canonical_bundle"]
        path = evidence_directory / reference["file"]
        value = json.loads(path.read_text(encoding="utf-8"))
        mutation(value)
        semantic_state = {
            "project_id": value["payload"]["project_id"],
            "tables": value["payload"]["tables"],
        }
        value["payload"]["lineage"]["history_basis"] = hashlib.sha256(
            json.dumps(semantic_state, ensure_ascii=False, separators=(",", ":")).encode()
        ).hexdigest()
        value["checksum"] = hashlib.sha256(
            json.dumps(value["payload"], ensure_ascii=False, separators=(",", ":")).encode()
        ).hexdigest()
        write_json(path, value)
        reference["sha256"] = sha256(path)

    marker_only = real_session_fixture("volicord", 1, revision, evidence_directory)
    marker_only["ordinary_work"] = {
        "status": "completed",
        "changed_paths": ["src/claimed.rs"],
    }
    marker_path, marker_events = capture_events(marker_only, "work")
    for value in marker_events:
        payload = value.get("payload", {})
        if payload.get("type") == "patch_apply_end":
            payload["changes"] = ["/phase8/repository/v11-ordinary-work.txt"]
    store_capture(marker_only, "work", marker_path, marker_events)
    if real_session_evidence(
        marker_only, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["meaningful_ordinary_changes"] != "failed":
        raise AssertionError("capture containing only synthetic marker work qualified")

    fabricated_decision = real_session_fixture("volicord", 1, revision, evidence_directory)
    fabricated_decision["user_decision"] = {"status": "passed", "actor": "user"}

    def remove_user_authority(bundle: dict[str, Any]) -> None:
        for table in bundle["payload"]["tables"]:
            if table["name"] != "sources":
                continue
            actor_index = table["columns"].index("actor_kind")
            kind_index = table["columns"].index("source_kind")
            for row in table["rows"]:
                if row[kind_index].get("value") == "current_host_user_turn":
                    row[actor_index] = {"type": "text", "value": "agent"}
                    return

    mutate_bundle(fabricated_decision, remove_user_authority)
    if real_session_evidence(
        fabricated_decision, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["explicit_user_decision_source"] != "failed":
        raise AssertionError("manifest Decision claim hid missing product user provenance")

    missing_decision = real_session_fixture("volicord", 1, revision, evidence_directory)
    missing_path, missing_events = capture_events(missing_decision, "work")
    missing_events = [
        value
        for value in missing_events
        if not (
            value.get("payload", {}).get("type") == "user_message"
            and value.get("payload", {}).get("message") == "<redacted-current-host-decision>"
        )
    ]
    store_capture(missing_decision, "work", missing_path, missing_events)
    if real_session_evidence(
        missing_decision, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["explicit_user_decision_source"] != "failed":
        raise AssertionError("capture without the matching current-host user turn qualified")

    same_session = real_session_fixture("volicord", 1, revision, evidence_directory)
    same_session["work_invocation"] = {"session_id": "claimed-work"}
    same_session["resume_invocation"] = {"session_id": "claimed-resume"}
    same_path, same_events = capture_events(same_session, "resume")
    same_events[0]["payload"]["id"] = "volicord-work-session-1"
    same_events[0]["payload"]["session_id"] = "volicord-work-session-1"
    store_capture(same_session, "resume", same_path, same_events)
    if real_session_evidence(
        same_session, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["distinct_work_and_resume_invocations"] != "failed":
        raise AssertionError("manifest identities hid same-session capture content")

    no_recall_first = real_session_fixture("volicord", 1, revision, evidence_directory)
    no_recall_first["resume_invocation"] = {"recall": {"sequence": 1}}
    order_path, order_events = capture_events(no_recall_first, "resume")
    recall_indexes = [
        index
        for index, value in enumerate(order_events)
        if value.get("payload", {}).get("name") == "recall"
        or (
            value.get("payload", {}).get("type") == "function_call_output"
            and "recall-call" in str(value.get("payload", {}).get("call_id"))
        )
    ]
    inspection_indexes = [
        index
        for index, value in enumerate(order_events)
        if value.get("payload", {}).get("name") == "repository_understanding"
        or (
            value.get("payload", {}).get("type") == "function_call_output"
            and "inspect-call" in str(value.get("payload", {}).get("call_id"))
        )
    ]
    recall_values = [order_events[index] for index in recall_indexes]
    inspection_values = [order_events[index] for index in inspection_indexes]
    remaining = [
        value
        for index, value in enumerate(order_events)
        if index not in set(recall_indexes + inspection_indexes)
    ]
    insert_at = 3
    order_events = remaining[:insert_at] + inspection_values + recall_values + remaining[insert_at:]
    store_capture(no_recall_first, "resume", order_path, order_events)
    if real_session_evidence(
        no_recall_first, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["recall_precedes_inspection_and_continuation"] != "failed":
        raise AssertionError("manifest order claim hid inspection before Recall")

    mismatched_recall = real_session_fixture("volicord", 1, revision, evidence_directory)
    mismatched_recall["resume_invocation"] = {
        "recall": {"checkpoint_id": "claimed", "decision_ids": ["claimed"], "context_ids": ["claimed"]}
    }
    mismatch_path, mismatch_events = capture_events(mismatched_recall, "resume")
    for value in mismatch_events:
        payload = value.get("payload", {})
        if payload.get("type") != "function_call_output" or "recall-call" not in str(payload.get("call_id")):
            continue
        output = json.loads(payload["output"])
        output["structuredContent"]["decisions"][0]["identity"] = "ff" * 16
        output["structuredContent"]["goals"] = ["Different recalled goal"]
        output["structuredContent"]["next_step"] = "Different recalled next step"
        payload["output"] = json.dumps(output, separators=(",", ":"))
    store_capture(mismatched_recall, "resume", mismatch_path, mismatch_events)
    if real_session_evidence(
        mismatched_recall, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["recall_matches_checkpoint_decision_and_context"] != "failed":
        raise AssertionError("manifest Recall IDs hid mismatched Recall result content")

    no_continuation = real_session_fixture("volicord", 1, revision, evidence_directory)
    no_continuation["resume_invocation"] = {"continuation": {"status": "passed"}}
    continuation_path, continuation_events = capture_events(no_continuation, "resume")
    continuation_events = [
        value
        for value in continuation_events
        if value.get("payload", {}).get("type") != "patch_apply_end"
    ]
    store_capture(no_continuation, "resume", continuation_path, continuation_events)
    if real_session_evidence(
        no_continuation, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["meaningful_recalled_continuation"] != "failed":
        raise AssertionError("manifest continuation claim hid absent post-Recall work")

    insufficient = real_session_fixture("volicord", 1, revision, evidence_directory)
    insufficient_path = evidence_directory / insufficient["captures"]["work"]["file"]
    insufficient_path.write_text('{"event":"work"}\n', encoding="utf-8")
    insufficient["captures"]["work"]["sha256"] = sha256(insufficient_path)
    insufficient["ordinary_work"] = {"status": "passed", "changed_paths": ["src/claimed.rs"]}
    insufficient_result = real_session_evidence(
        insufficient, kind="volicord", cycle=1, repository_revision=revision
    )
    if insufficient_result["status"] == "passed":
        raise AssertionError("validly hashed arbitrary event label qualified as Codex evidence")

    missing_language = parse_accessibility_html(
        "<!doctype html><html><head><meta name=\"viewport\" content=\"width=device-width\">"
        "</head><body><h1>x</h1></body></html>",
        expected_language="en",
    )
    missing_language["status"] = status_from_steps(missing_language["checks"])
    missing_qualified = qualify_accessibility(
        missing_language,
        {
            "not_color_only": {"status": "passed", "basis": "bounded observation"},
            "narrow_and_zoomed_presentation": {
                "status": "passed",
                "basis": "bounded observation",
            },
        },
        set(definition["permitted_accessibility_observations"]),
    )
    if missing_qualified["checks"]["document_html_language"] != "failed":
        raise AssertionError("observations hid missing HTML language")
    wrong_language = parse_accessibility_html(valid_html, expected_language="ko")
    wrong_language["status"] = status_from_steps(wrong_language["checks"])
    wrong_qualified = qualify_accessibility(
        wrong_language,
        {
            "not_color_only": {"status": "passed", "basis": "bounded observation"},
            "narrow_and_zoomed_presentation": {
                "status": "passed",
                "basis": "bounded observation",
            },
        },
        set(definition["permitted_accessibility_observations"]),
    )
    if wrong_qualified["checks"]["document_html_language"] != "failed":
        raise AssertionError("observations hid wrong HTML language")

    unavailable_viewer = {
        "status": "environment_blocked",
        "checks": {},
        "reason": "viewer_start_failed",
    }
    unavailable_qualified = qualify_accessibility(
        unavailable_viewer,
        {
            "not_color_only": {"status": "passed", "basis": "bounded observation"},
            "narrow_and_zoomed_presentation": {
                "status": "passed",
                "basis": "bounded observation",
            },
        },
        set(definition["permitted_accessibility_observations"]),
    )
    if unavailable_qualified["status"] != "environment_blocked":
        raise AssertionError("observations hid viewer environment failure")

    failed_quality = qualify_quality_observation(
        {"status": "failed", "basis": "machine failure"},
        {"status": "passed", "basis": "operator impression"},
        "question_relevance",
    )
    if failed_quality["status"] != "failed":
        raise AssertionError("manual quality evidence hid a machine failure")

    if viewer_start_failure_status(
        "cannot bind: Operation not permitted (os error 1)"
    ) != "environment_blocked":
        raise AssertionError("sandbox viewer bind denial was not preserved as environment-blocked")
    if viewer_start_failure_status("viewer rejected the project") != "failed":
        raise AssertionError("product viewer startup failure was not preserved as failed")
    if load_real_session_cycle(
        real_session_fixture("volicord", 1, revision, evidence_directory),
        evidence_directory,
    ) is not None:
        raise AssertionError("inline manifest literal was accepted as real-session evidence")
    print(json.dumps({
        "status": "passed",
        "definition_sha256": sha256(DEFINITION),
        "required_product_steps": len(definition["required_product_steps"]),
        "repository_classes": list(CLASSES),
        "two_cycle_contract": "passed",
        "real_session_positive_path": "passed",
        "v11_only_rejected": "passed",
        "synthetic_marker_work_rejected": "passed",
        "same_session_rejected": "passed",
        "recall_order_rejected": "passed",
        "mismatched_recall_state_rejected": "passed",
        "missing_continuation_rejected": "passed",
        "user_decision_provenance_rejected": "passed",
        "missing_user_decision_rejected": "passed",
        "valid_hash_insufficient_semantics_rejected": "passed",
        "arbitrary_event_label_rejected": "passed",
        "accessibility_real_parser_success": "passed",
        "accessibility_machine_failure_authority": "passed",
        "viewer_environment_blocking": "passed",
        "manual_override_boundary": "passed",
        "sanitization_regressions": "passed",
        "decision_revisit_blocking": "passed",
        "v11_reuse_route": str(V11_HARNESS.relative_to(ROOT)),
    }, indent=2, sort_keys=True))
    temporary.cleanup()
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("self-test")
    run = subparsers.add_parser("run")
    run.add_argument("--candidate-head", required=True)
    run.add_argument("--repositories", required=True)
    run.add_argument("--output-dir", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "self-test":
        return self_test()
    return run_evaluation(args)


if __name__ == "__main__":
    raise SystemExit(main())
