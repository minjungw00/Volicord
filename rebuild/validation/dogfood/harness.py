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
import stat
import subprocess
import sys
import tempfile
import threading
import time
from typing import Any, Callable

from codex_events import (
    CanonicalBundle,
    CodexCapture,
    EvidenceError,
    ToolCall,
    command_is_clean_git_status,
    command_is_repository_inspection,
    decode_established_fact_statements,
    decode_question_alternatives,
    decode_string_blob,
    load_canonical_bundle,
    load_codex_capture,
    parse_custom_call,
    parse_mcp_wrapper,
    recalled_checkpoint,
    recalled_decision_ids,
    relevant_context_ids,
)


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
DEFINITION = HERE / "evaluation.json"
CURRENT_MCP_FIXTURE = HERE / "fixtures/current-codex-mcp-completion.jsonl"
V11_HARNESS = ROOT / "rebuild/validation/end-to-end/multi-repository/harness.py"
DECISION_REGISTER = ROOT / "rebuild/docs/design/open-decisions.md"
ALLOWED_STATUS = {
    "passed", "failed", "partial", "unsupported", "skipped", "environment_blocked"
}
CLASSES = ("volicord", "small-python", "polyglot-medium")
RESOURCE_OPERATIONS = (
    "repository_analysis",
    "document_projection",
    "derived_analysis_repair",
)
RESOURCE_STORAGE_METRICS = (
    "runtime_home_bytes",
    "derived_state_bytes",
    "document_output_bytes",
)
REAL_SESSION_CHECKS = (
    "naturalistic_prompt_integrity",
    "plain_task_goal_linkage",
    "clean_bounded_baseline",
    "researched_material_question",
    "meaningful_ordinary_changes",
    "source_grounded_checkpoint",
    "explicit_user_decision_source",
    "distinct_work_and_resume_invocations",
    "fresh_resume_without_prior_context",
    "repository_bound_project_resolution",
    "recall_precedes_inspection_and_continuation",
    "recall_matches_checkpoint_decision_and_context",
    "meaningful_recalled_continuation",
)
MAX_USER_TASK_BYTES = 8192
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


def linux_process_tree_procfs_unavailability() -> str | None:
    """Return why the required Linux process-tree procfs interface cannot be used."""

    system = platform.system()
    if system != "Linux":
        return f"unsupported_operating_system:{system or 'unknown'}"
    process_id = os.getpid()
    try:
        page_size = int(os.sysconf("SC_PAGE_SIZE"))
    except (OSError, TypeError, ValueError) as error:
        return f"linux_page_size_unavailable:{type(error).__name__}"
    if page_size <= 0:
        return "linux_page_size_invalid"
    try:
        statm_fields = (Path("/proc") / str(process_id) / "statm").read_text(
            encoding="ascii"
        ).split()
    except OSError as error:
        return f"linux_procfs_statm_unavailable:{type(error).__name__}"
    if len(statm_fields) < 2:
        return "linux_procfs_statm_malformed"
    try:
        int(statm_fields[1])
    except ValueError:
        return "linux_procfs_statm_malformed"
    try:
        children = (
            Path("/proc") / str(process_id) / "task" / str(process_id) / "children"
        ).read_text(encoding="ascii")
    except OSError as error:
        return f"linux_procfs_children_unavailable:{type(error).__name__}"
    try:
        [int(value) for value in children.split()]
    except ValueError:
        return "linux_procfs_children_malformed"
    return None


class LinuxProcessTreePeakRss:
    """Observe the harness and its descendants without changing child execution."""

    def __init__(self, sampling_interval_ms: int) -> None:
        self.sampling_interval_ms = sampling_interval_ms
        self.peak_bytes = 0
        self.sample_count = 0
        self.observed_process_count = 0
        self.error: str | None = None
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None
        self._page_size = int(os.sysconf("SC_PAGE_SIZE"))

    def _process_tree_rss(self) -> tuple[int, int]:
        root_pid = os.getpid()
        pending = [root_pid]
        observed: set[int] = set()
        total = 0
        while pending:
            process_id = pending.pop()
            if process_id in observed:
                continue
            observed.add(process_id)
            process_root = Path("/proc") / str(process_id)
            try:
                fields = (process_root / "statm").read_text(encoding="ascii").split()
                total += int(fields[1]) * self._page_size
                children = (
                    process_root / "task" / str(process_id) / "children"
                ).read_text(encoding="ascii")
            except FileNotFoundError:
                if process_id == root_pid:
                    raise
                continue
            pending.extend(int(value) for value in children.split())
        return total, len(observed)

    def _sample(self) -> None:
        try:
            rss_bytes, process_count = self._process_tree_rss()
        except (OSError, ValueError, IndexError) as error:
            self.error = f"linux_procfs_process_tree_sampling_failed:{type(error).__name__}"
            self._stop.set()
            return
        self.peak_bytes = max(self.peak_bytes, rss_bytes)
        self.observed_process_count = max(self.observed_process_count, process_count)
        self.sample_count += 1

    def _observe(self) -> None:
        while not self._stop.wait(self.sampling_interval_ms / 1000):
            self._sample()

    def start(self) -> None:
        unavailability = linux_process_tree_procfs_unavailability()
        if unavailability is not None:
            self.error = unavailability
            return
        self._sample()
        if self.error is None:
            self._thread = threading.Thread(target=self._observe, daemon=True)
            self._thread.start()

    def stop(self) -> dict[str, Any]:
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout=2)
        if self.error is None:
            self._sample()
        status = (
            "passed"
            if self.error is None and self.sample_count > 0 and self.peak_bytes > 0
            else "environment_blocked"
        )
        return {
            "status": status,
            "peak_memory_bytes": self.peak_bytes if status == "passed" else None,
            "mechanism": "linux_procfs_process_tree_rss_sampling",
            "sampling_interval_ms": self.sampling_interval_ms,
            "sample_count": self.sample_count,
            "maximum_observed_process_count": self.observed_process_count,
            "measurement_error": self.error,
            "scope": "dogfood_harness_and_descendant_processes",
        }


def bounded_process_result(value: dict[str, Any]) -> dict[str, Any]:
    return {
        "outcome": value.get("outcome"),
        "exit_code": value.get("exit_code"),
        "termination": value.get("termination"),
        "spawn_failed": value.get("spawn_error") is not None,
        "duration_ms": value.get("duration_ms"),
    }


def repeated_resource_conclusion(rounds: list[dict[str, Any]]) -> dict[str, Any]:
    if len(rounds) < 3:
        return {
            "status": "unsupported",
            "conclusion": "insufficient_repeated_observations",
            "unexplained_cumulative_growth_observed": None,
            "metric_deltas_bytes": {},
        }
    if any(
        set(round_value.get("operations", {})) != set(RESOURCE_OPERATIONS)
        for round_value in rounds
    ):
        return {
            "status": "unsupported",
            "conclusion": "repeated_operation_evidence_incomplete",
            "unexplained_cumulative_growth_observed": None,
            "metric_deltas_bytes": {},
        }
    if any(
        operation.get("exit_code") != 0 or operation.get("termination") is not None
        for round_value in rounds
        for operation in round_value.get("operations", {}).values()
    ):
        return {
            "status": "failed",
            "conclusion": "repeated_operation_failed",
            "unexplained_cumulative_growth_observed": None,
            "metric_deltas_bytes": {},
        }
    if any(
        not isinstance(round_value.get(name), int)
        for round_value in rounds
        for name in RESOURCE_STORAGE_METRICS
    ):
        return {
            "status": "unsupported",
            "conclusion": "resource_measurement_unavailable",
            "unexplained_cumulative_growth_observed": None,
            "metric_deltas_bytes": {},
        }
    deltas = {
        name: [
            rounds[index][name] - rounds[index - 1][name]
            for index in range(1, len(rounds))
        ]
        for name in RESOURCE_STORAGE_METRICS
    }
    post_warmup = {name: values[1:] for name, values in deltas.items()}
    cumulative = [
        name for name, values in post_warmup.items()
        if values and all(value > 0 for value in values)
    ]
    stable = all(all(value == 0 for value in values) for values in post_warmup.values())
    return {
        "status": "failed" if cumulative else "passed",
        "conclusion": (
            "unexplained_cumulative_growth_observed"
            if cumulative else
            "stable_after_warmup"
            if stable else
            "bounded_variation_without_cumulative_growth"
        ),
        "unexplained_cumulative_growth_observed": bool(cumulative),
        "cumulative_growth_metrics": cumulative,
        "metric_deltas_bytes": deltas,
    }


def repeated_resource_rehearsal(
    kind: str,
    cycle_root: Path,
    recorder: Any,
    base_env: dict[str, str],
    project_id: str | None,
    repetition_count: int,
) -> dict[str, Any]:
    target_root = cycle_root / "work" / kind
    cli = target_root / "prefix/bin/volicord"
    if not project_id or not cli.is_file():
        return {
            "status": "environment_blocked",
            "conclusion": "product_prerequisite_unavailable",
            "unexplained_cumulative_growth_observed": None,
            "repetition_count": 0,
            "rounds": [],
        }
    home = target_root / "home"
    runtime = target_root / "runtime"
    environment = base_env | {
        "HOME": str(home),
        "XDG_DATA_HOME": str(home / ".local/share"),
        "CODEX_HOME": str(home / ".codex"),
        "VOLICORD_RUNTIME_DIR": str(runtime),
        "VOLICORD_HOME": str(target_root / "legacy-runtime"),
        "PATH": f"{target_root / 'prefix/bin'}:{base_env.get('PATH', '')}",
    }
    repeated_document = target_root / "repeated-resource/project-architecture-guide.html"
    repeated_document.parent.mkdir(parents=True, exist_ok=True)
    rounds: list[dict[str, Any]] = []

    def destination_present() -> bool:
        return repeated_document.exists() or repeated_document.is_symlink()

    def failed_rehearsal(conclusion: str) -> dict[str, Any]:
        return {
            "status": "failed",
            "conclusion": conclusion,
            "unexplained_cumulative_growth_observed": None,
            "repetition_count": repetition_count,
            "operations_per_round": list(RESOURCE_OPERATIONS),
            "fixed_input_and_destination": True,
            "universal_product_ceiling_applied": False,
            "rounds": rounds,
        }

    if destination_present():
        return failed_rehearsal("rehearsal_destination_preexisting")

    for repetition in range(1, repetition_count + 1):
        if destination_present():
            return failed_rehearsal("rehearsal_destination_ownership_ambiguous")
        analysis = recorder.run(
            f"resource-{repetition}-analyze",
            [str(cli), "analyze", project_id],
            environment,
        )
        document = recorder.run(
            f"resource-{repetition}-document",
            [
                str(cli), "documents", "export", project_id,
                "project-architecture-guide", "html", str(repeated_document), "en",
            ],
            environment,
        )
        document_output_bytes: int | None = None
        document_identity: tuple[int, int] | None = None
        ownership_failure: str | None = None
        document_succeeded = (
            document.get("exit_code") == 0
            and document.get("termination") is None
            and document.get("spawn_error") is None
        )
        if document_succeeded:
            try:
                output_stat = repeated_document.lstat()
            except OSError:
                ownership_failure = "successful_document_output_unavailable"
            else:
                if not stat.S_ISREG(output_stat.st_mode):
                    ownership_failure = "successful_document_output_not_regular"
                else:
                    document_output_bytes = output_stat.st_size
                    document_identity = (output_stat.st_dev, output_stat.st_ino)
        elif destination_present():
            ownership_failure = "failed_document_export_created_unowned_destination"
        repair = recorder.run(
            f"resource-{repetition}-repair",
            [str(cli), "repair", project_id, "derived-analysis"],
            environment,
        )
        rounds.append({
            "round": repetition,
            "operations": {
                "repository_analysis": bounded_process_result(analysis),
                "document_projection": bounded_process_result(document),
                "derived_analysis_repair": bounded_process_result(repair),
            },
            "runtime_home_bytes": directory_bytes(runtime),
            "derived_state_bytes": directory_bytes(runtime / "analysis"),
            "document_output_bytes": document_output_bytes,
        })
        if document_identity is not None:
            try:
                cleanup_stat = repeated_document.lstat()
            except OSError:
                ownership_failure = "rehearsal_owned_output_cleanup_unavailable"
            else:
                if (cleanup_stat.st_dev, cleanup_stat.st_ino) != document_identity:
                    ownership_failure = "rehearsal_owned_output_replaced_before_cleanup"
                else:
                    try:
                        repeated_document.unlink()
                    except OSError:
                        ownership_failure = "rehearsal_owned_output_cleanup_failed"
                    else:
                        if destination_present():
                            ownership_failure = "rehearsal_destination_reappeared_after_cleanup"
        if ownership_failure is not None:
            return failed_rehearsal(ownership_failure)
    conclusion = repeated_resource_conclusion(rounds)
    return {
        **conclusion,
        "repetition_count": repetition_count,
        "operations_per_round": list(RESOURCE_OPERATIONS),
        "fixed_input_and_destination": True,
        "universal_product_ceiling_applied": False,
        "rounds": rounds,
    }


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
    resources = value.get("resource_qualification", {})
    if (
        resources.get("supported_operating_system") != "Linux"
        or resources.get("peak_memory_mechanism")
        != "linux_procfs_process_tree_rss_sampling"
        or not isinstance(resources.get("peak_memory_sampling_interval_ms"), int)
        or resources.get("peak_memory_sampling_interval_ms", 0) <= 0
        or resources.get("repeated_resource_repetition_count", 0) < 3
        or tuple(resources.get("repeated_operations", [])) != RESOURCE_OPERATIONS
        or tuple(resources.get("measured_storage_classes", []))
        != RESOURCE_STORAGE_METRICS
        or resources.get("universal_product_ceiling_applied") is not False
        or resources.get("raw_evidence_retention") != "ignored_local_state_only"
    ):
        raise ValueError("the Phase 8 bounded resource qualification contract changed")
    accessibility = value.get("accessibility_machine_contract", {})
    if (
        tuple(accessibility.get("visible_form_control_names", []))
        != (
            "associated_label",
            "enclosing_label",
            "aria_label",
            "aria_labelledby",
        )
        or "meaningful_visible_text" not in accessibility.get("button_names", [])
        or "unlabeled_visible_control"
        not in accessibility.get("deterministic_failures", [])
        or accessibility.get("manual_observation_may_override_deterministic_failure")
        is not False
    ):
        raise ValueError("the Phase 8 accessibility machine contract changed")
    if (
        evidence.get("required_capture_format")
        != "codex_mcp_completion_rollout_jsonl"
        or tuple(evidence.get("bounded_call_forms", []))
        != (
            "event_msg.mcp_tool_call_end with volicord invocation and structured result",
            "tools.exec_command(literal_object)",
            "tools.apply_patch(literal_string)",
        )
        or tuple(evidence.get("work_session_contract", []))
        != (
            "the first captured user turn matches the descriptor plain work_user_task exactly or after removing at most one Codex transport terminal LF or CRLF",
            "after Project initialization source canonical goal Context from the exact descriptor work_user_task",
            "establish the repository baseline through repository_analyze before ordinary work",
            "submit, source-ground research, mark ready, and promote the material Question Candidate through candidate_manage before inquiry_frontier",
            "independently research and present a material Question without receiving its alternatives or recommendation in the user task",
            "obtain and record the exact current-host user Decision",
            "perform real repository work after the baseline",
            "commands used only for incidental inspection need not become Checkpoint verification facts",
            "every command referenced by checkpoint_record passed or failed verification has a numeric exit_code from the same captured command result, through either complete-result forwarding or exact same-result output/status forwarding; output-only forwarding is outcome-unknown",
            "record a grounded Checkpoint using the Goal Context identity, applicable current-host Decisions, truthful verification evidence, limits, and next meaningful state or step",
        )
        or tuple(evidence.get("resume_session_contract", []))
        != (
            "the first captured user turn matches the descriptor plain fresh_resume_user_task exactly or after removing at most one Codex transport terminal LF or CRLF, and does not disclose Recall",
            "a fresh resume session resolves the repository-bound existing Project through project_resolve before Recall without initializing a replacement Project",
            "a fresh resume session invokes Recall after project_resolve and before repository inspection or continued work",
            "the resume session produces meaningful observed repository changes relevant to the recalled Checkpoint current state or next step",
            "the resume session preserves separate same-command numeric-exit validation after that change",
        )
        or evidence.get("codex_user_turn_transport_identity")
        != {
            "captured_text_allowance": (
                "exact text or removal of at most one terminal LF or one terminal CRLF"
            ),
            "descriptor_task_mutated": False,
            "raw_capture_mutated": False,
            "evidence_sha256_mutated": False,
            "other_whitespace_normalized": False,
        }
        or evidence.get("command_forwarding_contract")
        != {
            "complete_result_evidence": "one statically bound exec_command result forwards its complete structured result",
            "correlated_split_evidence": "one statically bound exec_command result forwards its output and an exact numeric exit_code projection from that same result",
            "incidental_inspection": "may remain an observed command without becoming a Checkpoint verification fact",
            "checkpoint_verification": "passed or failed facts require a numeric exit_code correlated to the same captured command observation",
            "output_only_outcome": "unknown",
            "uncorrelated_or_synthesized_status_outcome": "unknown",
        }
        or len(evidence.get("bounded_parser_limitations", [])) != 3
    ):
        raise ValueError("the current Codex rollout evidence contract changed")
    if evidence.get("mcp_completion_contract") != {
        "authoritative_event": "event_msg.mcp_tool_call_end",
        "server": "volicord",
        "success": "result.Ok.isError is false with object structuredContent",
        "failure": "result.Err, result.Ok.isError true, malformed completion, or correlated wrapper mismatch cannot qualify",
        "deduplication": "one completion call_id yields one semantic operation; wrapper output is not a second semantic source",
    }:
        raise ValueError("the current MCP completion evidence contract changed")
    descriptor_contract = evidence.get("cycle_descriptor_contract", {})
    if (
        descriptor_contract.get("work_user_task_field") != "work_user_task"
        or descriptor_contract.get("fresh_resume_user_task_field") != "fresh_resume_user_task"
        or descriptor_contract.get("hidden_decision_oracle_field") != "decision_oracle"
        or tuple(descriptor_contract.get("identity_fields", []))
        != ("repository_class", "cycle", "repository_revision")
        or descriptor_contract.get("evidence_reference_field") != "evidence"
    ):
        raise ValueError("the Phase 8 cycle descriptor contract changed")
    hidden_oracle = evidence.get("hidden_decision_oracle", {})
    if (
        "work_task_materiality_basis" not in hidden_oracle.get("required_fields", [])
        or hidden_oracle.get("materiality_basis_normalization")
        != "casefold_and_collapse_whitespace"
        or hidden_oracle.get("materiality_basis_location")
        != "normalized basis must occur in work_user_task; fresh_resume_user_task alone is insufficient"
        or hidden_oracle.get("materiality_basis_hidden_content_exclusion")
        != "must not disclose alternatives, recommendation, or expected choice"
        or evidence.get("full_replacement_session_count") != 12
        or evidence.get("required_codex_sessions_per_cycle") != 2
        or evidence.get("work_blocker_qualification")
        != {
            "subcommand": "qualify-work-blocker",
            "result_kind": "phase8_dogfood_blocker_result",
            "failure_only": True,
            "campaign_complete": False,
            "replacement_pass_candidate": False,
            "phase_9_ready": False,
            "later_evidence_status": "not_run",
        }
    ):
        raise ValueError("the Phase 8 materiality or work-blocker contract changed")
    goal = evidence.get("plain_task_goal", {})
    if (
        goal.get("maximum_utf8_bytes") != MAX_USER_TASK_BYTES
        or tuple(goal.get("required_linkage", []))
        != (
            "descriptor_plain_work_user_task",
            "first_work_session_user_task_turn_transport_identity_match",
            "evaluated_repository_revision",
            "context_record_exact_user_turn_source",
            "canonical_goal_identity_and_statement",
            "checkpoint_goal_context_identity",
            "fresh_session_recall_same_goal_identity_and_materially_consistent_statement",
        )
    ):
        raise ValueError("the Phase 8 plain-task Goal evidence contract changed")
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
    calls = capture.successful_calls(operation)
    return calls[0] if len(calls) == 1 else None


def normalized_prompt_text(value: str) -> str:
    return " ".join(value.casefold().split())


def codex_user_turn_transport_identity_matches(
    captured_user_turn: Any,
    descriptor_task: Any,
) -> bool:
    if not isinstance(captured_user_turn, str) or not isinstance(descriptor_task, str):
        return False
    if captured_user_turn == descriptor_task:
        return True
    if captured_user_turn.endswith("\r\n"):
        return captured_user_turn[:-2] == descriptor_task
    if captured_user_turn.endswith("\n"):
        return captured_user_turn[:-1] == descriptor_task
    return False


def plain_user_task_error(value: Any, field: str) -> str | None:
    if not nonempty_string(value) or value != value.strip():
        return f"{field} must be a non-empty exact plain user task"
    encoded = value.encode("utf-8")
    if len(encoded) > MAX_USER_TASK_BYTES or any(ord(character) < 32 and character not in "\n\t" for character in value):
        return f"{field} exceeds the bounded plain-task contract"
    return None


def decision_oracle_errors(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["decision_oracle must be hidden evaluator material"]
    errors: list[str] = []
    for field in (
        "work_task_materiality_basis",
        "user_owned_dimension",
        "why_repository_inspection_cannot_decide",
        "recommendation",
        "material_consequence",
    ):
        if not nonempty_string(value.get(field)):
            errors.append(f"decision_oracle.{field} must be non-empty")
    for field, minimum in (("established_repository_facts", 1), ("viable_alternatives", 2)):
        items = value.get(field)
        if (
            not isinstance(items, list)
            or len(items) < minimum
            or len(items) > 32
            or not all(nonempty_string(item) for item in items)
            or len(items) != len(set(items))
        ):
            errors.append(f"decision_oracle.{field} must contain unique bounded text entries")
    return errors


def naturalistic_prompt_errors(work_task: Any, resume_task: Any, oracle: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(work_task, str) or not isinstance(resume_task, str) or not isinstance(oracle, dict):
        return ["naturalistic prompt integrity requires both plain tasks and a hidden oracle"]
    prompts = (("work_user_task", work_task), ("fresh_resume_user_task", resume_task))
    materiality_basis = oracle.get("work_task_materiality_basis")
    if nonempty_string(materiality_basis):
        normalized_basis = normalized_prompt_text(materiality_basis)
        if normalized_basis not in normalized_prompt_text(work_task):
            if normalized_basis in normalized_prompt_text(resume_task):
                errors.append(
                    "decision_oracle.work_task_materiality_basis appears only in fresh_resume_user_task"
                )
            else:
                errors.append(
                    "decision_oracle.work_task_materiality_basis is absent from work_user_task"
                )
        if len(materiality_basis.encode("utf-8")) > MAX_USER_TASK_BYTES:
            errors.append("decision_oracle.work_task_materiality_basis exceeds its bound")
        disclosed = [
            *oracle.get("viable_alternatives", []),
            oracle.get("recommendation"),
            oracle.get("expected_choice"),
        ]
        if any(
            nonempty_string(hidden)
            and normalized_prompt_text(hidden) in normalized_basis
            for hidden in disclosed
        ):
            errors.append(
                "decision_oracle.work_task_materiality_basis discloses an alternative or recommendation"
            )
    operation_names = (
        "project_resolve",
        "project_initialize",
        "repository_analyze",
        "context_record",
        "candidate_manage",
        "inquiry_frontier",
        "decision_record",
        "checkpoint_record",
    )
    for field, prompt in prompts:
        lowered = normalized_prompt_text(prompt)
        if "phase8_objective" in lowered or "resume_change_scope" in lowered:
            errors.append(f"{field} exposes a Phase 8 harness marker")
        for operation in operation_names:
            if re.search(rf"(?<![a-z0-9_]){re.escape(operation)}(?![a-z0-9_])", lowered):
                errors.append(f"{field} prescribes the {operation} domain operation")
        if re.search(r"\bask (?:the )?user\b|\bquestion (?:to ask|for the user)\b", lowered):
            errors.append(f"{field} prescribes the Question to ask")
        if re.search(r"\b(recall first|invoke recall|call recall|use recall|perform recall|run recall)\b", lowered):
            errors.append(f"{field} prescribes Recall")
    if re.search(r"\brecall\b", normalized_prompt_text(resume_task)):
        errors.append("fresh_resume_user_task discloses the automatic Recall expectation")

    hidden_values = [
        *oracle.get("viable_alternatives", []),
        oracle.get("recommendation"),
        oracle.get("expected_choice"),
    ]
    for hidden in hidden_values:
        if not nonempty_string(hidden):
            continue
        hidden_text = normalized_prompt_text(hidden)
        for field, prompt in prompts:
            if hidden_text in normalized_prompt_text(prompt):
                errors.append(f"{field} discloses an exact hidden alternative or recommendation")

    path_pattern = r"(?:^|\s)(?:[A-Za-z0-9_.-]+/)+[A-Za-z0-9_.-]+"
    for field, prompt in prompts:
        lowered = normalized_prompt_text(prompt)
        reserves_future_work = (
            re.search(r"\b(leave|reserve|keep)\b", lowered)
            or re.search(r"\bdo not (?:change|modify|complete|finish)\b", lowered)
        )
        if reserves_future_work and re.search(r"\b(next|later|resume|continuation)\s+(?:work\s+)?session\b", lowered) and re.search(path_pattern, prompt):
            errors.append(f"{field} reserves a named path for a later session")
    return sorted(set(errors))


def evidence_reference_shape(value: Any) -> bool:
    return (
        isinstance(value, dict)
        and nonempty_string(value.get("file"))
        and valid_capture_sha256(value.get("sha256"))
    )


def relevant_continuation_paths(paths: list[str], next_step: Any) -> list[str]:
    if not nonempty_string(next_step):
        return []
    lowered = normalized_prompt_text(next_step)
    generic = {"src", "source", "test", "tests", "lib", "crates", "file", "code"}
    result: list[str] = []
    for path in paths:
        if looks_like_synthetic_marker(path) or Path(path).suffix.lower() in {".txt", ".marker"}:
            continue
        candidate = Path(path)
        terms = {path.casefold(), candidate.name.casefold(), candidate.stem.casefold()}
        terms.update(part.casefold() for part in candidate.parts if len(part) >= 4)
        if any(term not in generic and term in lowered for term in terms):
            result.append(path)
    return sorted(set(result))


def meaningful_resume_validation(capture: CodexCapture | None, after_sequence: int | None) -> bool:
    if capture is None or after_sequence is None:
        return False
    return any(
        command.sequence > after_sequence
        and command.termination == "exited"
        and command.exit_code == 0
        and not command_is_clean_git_status(command.parsed_command)
        and not command_is_repository_inspection(command.parsed_command)
        for command in capture.commands
    )


def cycle_descriptor_errors(value: Any) -> list[str]:
    if not isinstance(value, dict) or value.get("kind") != "phase8_cycle_descriptor":
        return ["descriptor kind must be phase8_cycle_descriptor"]
    errors: list[str] = []
    for obsolete in ("objective", "resume_change_scope", "work_session_contract", "resume_session_contract"):
        if obsolete in value:
            errors.append(f"descriptor does not support obsolete field {obsolete}")
    if value.get("repository_class") not in CLASSES:
        errors.append("repository_class must identify a Phase 8 repository class")
    if not isinstance(value.get("cycle"), int) or value.get("cycle") not in {1, 2}:
        errors.append("cycle must identify one of the two independent repetitions")
    revision = value.get("repository_revision")
    if not isinstance(revision, str) or re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", revision) is None:
        errors.append("repository_revision must be a full Git object identity")
    for field in ("work_user_task", "fresh_resume_user_task"):
        error = plain_user_task_error(value.get(field), field)
        if error:
            errors.append(error)
    oracle = value.get("decision_oracle")
    errors.extend(decision_oracle_errors(oracle))
    if not decision_oracle_errors(oracle):
        errors.extend(
            naturalistic_prompt_errors(
                value.get("work_user_task"), value.get("fresh_resume_user_task"), oracle
            )
        )
    evidence = value.get("evidence")
    if evidence is not None:
        captures = evidence.get("captures") if isinstance(evidence, dict) else None
        if (
            not isinstance(captures, dict)
            or not evidence_reference_shape(captures.get("work"))
            or not evidence_reference_shape(captures.get("resume"))
            or not evidence_reference_shape(evidence.get("canonical_bundle"))
        ):
            errors.append("evidence must contain bounded work, resume, and canonical bundle references")
    return errors


def check_descriptors(paths: list[str]) -> int:
    if not paths:
        raise ValueError("at least one Phase 8 descriptor path is required")
    failures: dict[str, list[str]] = {}
    for raw_path in paths:
        path = Path(raw_path)
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            failures[str(path)] = [f"descriptor could not be read: {error}"]
            continue
        errors = cycle_descriptor_errors(value)
        if errors:
            failures[str(path)] = errors
    print(json.dumps({
        "status": "failed" if failures else "passed",
        "descriptor_count": len(paths),
        "failures": failures,
    }, indent=2, sort_keys=True))
    return 1 if failures else 0


WORK_BLOCKER_CHECKS = (
    "project_session_entry",
    "goal_context_operation",
    "repository_baseline_operation",
    "material_question_candidate_lifecycle",
    "explicit_current_host_user_decision_operation",
    "source_grounded_checkpoint_operation",
)


def build_work_blocker_result(
    candidate_head: str,
    descriptor: dict[str, Any],
    descriptor_sha256: str,
    capture: CodexCapture,
) -> dict[str, Any]:
    descriptor_errors = cycle_descriptor_errors(descriptor)
    if descriptor_errors:
        raise ValueError("qualify-work-blocker requires one valid cycle descriptor")
    if not re.fullmatch(r"[0-9a-f]{40}", candidate_head):
        raise ValueError("qualify-work-blocker requires an exact candidate HEAD")
    if (
        capture.git_revision != descriptor.get("repository_revision")
        or capture.source != "vscode"
        or capture.originator != "codex_vscode"
        or not capture.fresh_user_thread
        or not capture.user_turns
        or not codex_user_turn_transport_identity_matches(
            capture.user_turns[0].text,
            descriptor.get("work_user_task"),
        )
    ):
        raise ValueError("work capture does not match the descriptor and fresh VS Code Codex contract")
    if (
        not capture.task_sequences
        or len(capture.completed_task_sequences) < len(capture.task_sequences)
        or max(capture.completed_task_sequences) <= max(capture.task_sequences)
    ):
        raise ValueError("work capture is not machine-observably completed")

    project_entries = [
        call
        for call in (
            *capture.successful_calls("project_initialize"),
            *capture.successful_calls("project_resolve"),
        )
        if nonempty_string(call.result.get("project_id"))
        and (
            call.operation == "project_initialize"
            or call.result.get("status") == "found"
        )
    ]
    goal_calls = [
        call
        for call in capture.successful_calls("context_record")
        if call.arguments.get("role") == "goal"
        and call.arguments.get("user_turn") == descriptor.get("work_user_task")
    ]
    baseline_calls = capture.successful_calls("repository_analyze")
    candidate_actions = {
        call.arguments.get("action")
        for call in capture.successful_calls("candidate_manage")
        if call.arguments.get("action") == call.result.get("action")
    }
    required_candidate_actions = {
        "submit_question",
        "attach_repository_research",
        "mark_research_ready",
        "promote_question",
    }
    material_question_lifecycle = (
        required_candidate_actions <= candidate_actions
        and bool(capture.successful_calls("inquiry_frontier"))
    )
    observed = {
        "project_session_entry": bool(project_entries),
        "goal_context_operation": bool(goal_calls),
        "repository_baseline_operation": bool(baseline_calls),
        "material_question_candidate_lifecycle": material_question_lifecycle,
        "explicit_current_host_user_decision_operation": bool(
            capture.successful_calls("decision_record")
        ),
        "source_grounded_checkpoint_operation": bool(
            capture.successful_calls("checkpoint_record")
        ),
    }
    failed_checks = [name for name in WORK_BLOCKER_CHECKS if not observed[name]]
    if not failed_checks:
        raise ValueError(
            "completed work capture has no machine-observable terminal work blocker; use normal full qualification"
        )
    result = {
        "kind": "phase8_dogfood_blocker_result",
        "status": "failed",
        "candidate_head": candidate_head,
        "repository_class": descriptor["repository_class"],
        "cycle": descriptor["cycle"],
        "repository_revision": descriptor["repository_revision"],
        "descriptor_sha256": descriptor_sha256,
        "work_capture_sha256": capture.source_sha256,
        "failed_checks": failed_checks,
        "failed_check_count": len(failed_checks),
        "campaign_complete": False,
        "replacement_pass_candidate": False,
        "phase_9_ready": False,
        "later_required_evidence": {
            "fresh_resume_session": "not_run",
            "remaining_repository_cycles": "not_run",
            "automatic_checks": "not_run",
            "manual_observations": "not_run",
            "resource_qualification": "not_run",
            "accessibility_qualification": "not_run",
        },
        "evidence_origin": "completed_repository_normalized_codex_work_rollout",
    }
    validate_blocker_result(result)
    return result


def validate_blocker_result(result: dict[str, Any]) -> None:
    expected_keys = {
        "kind",
        "status",
        "candidate_head",
        "repository_class",
        "cycle",
        "repository_revision",
        "descriptor_sha256",
        "work_capture_sha256",
        "failed_checks",
        "failed_check_count",
        "campaign_complete",
        "replacement_pass_candidate",
        "phase_9_ready",
        "later_required_evidence",
        "evidence_origin",
    }
    failed_checks = result.get("failed_checks")
    later = result.get("later_required_evidence")
    if set(result) != expected_keys or result.get("kind") != "phase8_dogfood_blocker_result":
        raise ValueError("unexpected Phase 8 work-blocker result shape")
    if (
        result.get("status") != "failed"
        or result.get("campaign_complete") is not False
        or result.get("replacement_pass_candidate") is not False
        or result.get("phase_9_ready") is not False
    ):
        raise ValueError("work-blocker result cannot claim campaign completion or passage")
    if (
        not re.fullmatch(r"[0-9a-f]{40}", result.get("candidate_head", ""))
        or result.get("repository_class") not in CLASSES
        or result.get("cycle") not in {1, 2}
        or not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", result.get("repository_revision", ""))
        or not valid_capture_sha256(result.get("descriptor_sha256"))
        or not valid_capture_sha256(result.get("work_capture_sha256"))
    ):
        raise ValueError("work-blocker result identity is incomplete")
    if (
        not isinstance(failed_checks, list)
        or not failed_checks
        or any(check not in WORK_BLOCKER_CHECKS for check in failed_checks)
        or failed_checks != [name for name in WORK_BLOCKER_CHECKS if name in failed_checks]
        or result.get("failed_check_count") != len(failed_checks)
    ):
        raise ValueError("work-blocker result has invalid failed checks")
    if (
        not isinstance(later, dict)
        or set(later)
        != {
            "fresh_resume_session",
            "remaining_repository_cycles",
            "automatic_checks",
            "manual_observations",
            "resource_qualification",
            "accessibility_qualification",
        }
        or set(later.values()) != {"not_run"}
    ):
        raise ValueError("work-blocker result does not preserve later evidence as not_run")
    sanitize_check(result)


def qualify_work_blocker(args: argparse.Namespace) -> int:
    candidate_head = git_head(ROOT)
    if candidate_head is None or candidate_head != args.candidate_head:
        raise RuntimeError("candidate HEAD does not match --candidate-head")
    descriptor_path = Path(args.descriptor)
    capture_path = Path(args.work_capture)
    output_path = Path(args.output)
    if output_path.exists():
        raise RuntimeError("work-blocker output path must not already exist")
    try:
        descriptor_bytes = descriptor_path.read_bytes()
        descriptor = json.loads(descriptor_bytes)
        capture = load_codex_capture(capture_path)
    except (OSError, json.JSONDecodeError, EvidenceError) as error:
        raise ValueError("work-blocker input evidence is invalid") from error
    result = build_work_blocker_result(
        candidate_head,
        descriptor,
        hashlib.sha256(descriptor_bytes).hexdigest(),
        capture,
    )
    write_json(output_path, result)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 1


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


def goal_facts(
    work: CodexCapture | None,
    bundle: CanonicalBundle | None,
    descriptor_task: Any,
) -> tuple[bool, str | None, str | None, str | None]:
    call = unique_call(work, "context_record")
    if call is None or work is None or bundle is None or not work.user_turns:
        return False, None, None, None
    first_turn = work.user_turns[0]
    turn = work.turn_for_call(call)
    context_id = call.result.get("context_item_id")
    source_id = call.result.get("source_id")
    item = bundle.one("context_items", id=context_id, project_id=bundle.project_id)
    source = bundle.one("sources", id=source_id, project_id=bundle.project_id)
    relation = bundle.one(
        "context_item_sources",
        project_id=bundle.project_id,
        context_item_id=context_id,
        source_id=source_id,
        position=0,
    )
    statement = call.arguments.get("statement")
    valid = (
        nonempty_string(descriptor_task)
        and statement == descriptor_task
        and turn == first_turn
        and call.arguments.get("project_id") == bundle.project_id
        and call.arguments.get("user_turn") == descriptor_task
        and call.arguments.get("role") == "goal"
        and call.result.get("project_id") == bundle.project_id
        and call.result.get("role") == "goal"
        and nonempty_string(context_id)
        and nonempty_string(source_id)
        and item is not None
        and item.get("role") == "goal"
        and item.get("statement") == statement
        and item.get("provenance_role") == "user_statement"
        and item.get("author_kind") == "user"
        and source is not None
        and source.get("source_kind") == "current_host_user_turn"
        and source.get("locator") == descriptor_task
        and source.get("detail_one") == "codex"
        and source.get("detail_two") == work.session_id
        and source.get("actor_kind") == "user"
        and relation is not None
    )
    return (
        bool(valid),
        str(context_id) if nonempty_string(context_id) else None,
        str(source_id) if nonempty_string(source_id) else None,
        str(statement) if nonempty_string(statement) else None,
    )


def question_review_facts(
    work: CodexCapture | None,
    bundle: CanonicalBundle | None,
    question_id: str | None,
    question_revision: int | None,
    decision_oracle: Any,
    baseline_call: ToolCall | None,
) -> tuple[bool, dict[str, Any]]:
    if (
        work is None
        or bundle is None
        or not nonempty_string(question_id)
        or not isinstance(question_revision, int)
        or not isinstance(decision_oracle, dict)
        or baseline_call is None
    ):
        return False, {}
    decision_call = unique_call(work, "decision_record")
    revision = bundle.one(
        "question_revisions",
        project_id=bundle.project_id,
        question_id=question_id,
        revision=question_revision,
    )
    alternatives = (
        decode_question_alternatives(revision.get("alternatives"))
        if revision is not None
        else None
    )
    established_facts = (
        decode_established_fact_statements(revision.get("established_facts"))
        if revision is not None
        else None
    )
    frontier_calls = [
        call
        for call in work.successful_calls("inquiry_frontier")
        if any(
            isinstance(question, dict)
            and question.get("identity") == question_id
            and question.get("revision") == question_revision
            for question in call.result.get("questions", [])
        )
    ]
    frontier_call = frontier_calls[0] if len(frontier_calls) == 1 else None
    candidate_calls = work.successful_calls("candidate_manage")
    candidate_actions = {
        action: [
            call
            for call in candidate_calls
            if call.arguments.get("action") == action
            and call.result.get("action") == action
        ]
        for action in (
            "submit_question",
            "attach_repository_research",
            "mark_research_ready",
            "promote_question",
        )
    }
    candidate_lifecycle_calls = [
        calls[0] if len(calls) == 1 else None
        for calls in candidate_actions.values()
    ]
    submit_call, research_call, ready_call, promote_call = candidate_lifecycle_calls
    candidate_id = (
        submit_call.result.get("candidate_id") if submit_call is not None else None
    )
    research_source_ids = (
        research_call.arguments.get("source_ids") if research_call is not None else None
    )
    research_sources_are_repository_grounded = (
        isinstance(research_source_ids, list)
        and bool(research_source_ids)
        and all(
            bundle.one("sources", id=source_id, project_id=bundle.project_id) is not None
            and bundle.one("sources", id=source_id, project_id=bundle.project_id).get(
                "source_kind"
            )
            == "repository_snapshot"
            for source_id in research_source_ids
        )
    )
    candidate_lifecycle_ok = (
        all(call is not None for call in candidate_lifecycle_calls)
        and nonempty_string(candidate_id)
        and all(
            call.arguments.get("project_id") == bundle.project_id
            for call in candidate_lifecycle_calls
            if call is not None
        )
        and all(
            call.arguments.get("candidate_id") == candidate_id
            and call.result.get("candidate_id") == candidate_id
            for call in (research_call, ready_call, promote_call)
            if call is not None
        )
        and submit_call.arguments.get("research_state") == "research_required"
        and submit_call.arguments.get("source_operation") == "repository_analyze"
        and submit_call.result.get("state") == "stored"
        and submit_call.result.get("canonical_mutation") is False
        and research_call.arguments.get("evidence_assessment") == "sufficient"
        and research_sources_are_repository_grounded
        and research_call.result.get("canonical_mutation") is False
        and research_call.result.get("promoted") is False
        and ready_call.result.get("research_state") == "ready_to_ask"
        and ready_call.result.get("canonical_mutation") is False
        and promote_call.result.get("question_id") == question_id
        and baseline_call.completion_sequence < submit_call.sequence
        < research_call.sequence
        < ready_call.sequence
        < promote_call.sequence
        and promote_call.completion_sequence
        < (frontier_call.sequence if frontier_call is not None else -1)
    ) if all(call is not None for call in candidate_lifecycle_calls) else False
    material_scope = decode_string_blob(revision.get("material_scope")) if revision else None
    prompt_fields = [
        revision.get("prompt_basis") if revision else None,
        revision.get("why_it_matters_now") if revision else None,
        *(material_scope or []),
    ]
    observed_question_text = "\n".join(
        value for value in prompt_fields if nonempty_string(value)
    )
    oracle_facts = decision_oracle.get("established_repository_facts", [])
    oracle_alternatives = decision_oracle.get("viable_alternatives", [])
    observed_alternative_texts = {
        normalized_prompt_text(value)
        for alternative in alternatives or []
        for value in (
            alternative.get("label", ""),
            alternative.get("consequence", ""),
            f"{alternative.get('label', '')}: {alternative.get('consequence', '')}",
        )
        if nonempty_string(value)
    }
    facts_matched = sum(
        normalized_prompt_text(value)
        in {normalized_prompt_text(item) for item in established_facts or []}
        for value in oracle_facts
    )
    alternatives_matched = sum(
        normalized_prompt_text(value) in observed_alternative_texts
        for value in oracle_alternatives
    )
    dimension = decision_oracle.get("user_owned_dimension")
    recommendation = decision_oracle.get("recommendation")
    basis = {
        "canonical_question_present": revision is not None,
        "canonical_materiality": revision.get("materiality") if revision else None,
        "repository_facts_observed_count": len(established_facts or []),
        "oracle_repository_fact_count": len(oracle_facts) if isinstance(oracle_facts, list) else 0,
        "exact_repository_fact_matches": facts_matched,
        "observed_alternative_count": len(alternatives or []),
        "oracle_alternative_count": len(oracle_alternatives) if isinstance(oracle_alternatives, list) else 0,
        "exact_alternative_matches": alternatives_matched,
        "user_owned_dimension_present_in_observed_question": (
            nonempty_string(dimension)
            and normalized_prompt_text(dimension)
            in normalized_prompt_text(observed_question_text)
        ),
        "recommendation_matches_hidden_oracle": (
            nonempty_string(recommendation)
            and revision is not None
            and normalized_prompt_text(recommendation)
            == normalized_prompt_text(str(revision.get("recommendation_rationale", "")))
        ),
        "candidate_lifecycle_observed": candidate_lifecycle_ok,
        "automatic_relevance_conclusion": None,
        "manual_review_required": True,
    }
    valid = (
        revision is not None
        and revision.get("materiality") == "material"
        and nonempty_string(revision.get("prompt_basis"))
        and nonempty_string(revision.get("why_it_matters_now"))
        and established_facts is not None
        and alternatives is not None
        and len(alternatives) >= 2
        and nonempty_string(revision.get("recommendation_rationale"))
        and candidate_lifecycle_ok
        and frontier_call is not None
        and decision_call is not None
        and baseline_call.completion_sequence < frontier_call.sequence
        and frontier_call.completion_sequence < decision_call.sequence
    )
    return valid, basis


def checkpoint_verification_facts(
    work: CodexCapture,
    bundle: CanonicalBundle,
    call: ToolCall,
    checkpoint_id: str,
) -> bool:
    declared = call.arguments.get("verification")
    returned_ids = call.result.get("verification_source_ids")
    if not isinstance(declared, list) or not declared or not isinstance(returned_ids, list):
        return False
    rows = sorted(
        (
            row
            for row in bundle.rows("checkpoint_verifications")
            if row.get("project_id") == bundle.project_id
            and row.get("checkpoint_id") == checkpoint_id
        ),
        key=lambda row: row.get("position") if isinstance(row.get("position"), int) else -1,
    )
    if len(rows) != len(declared):
        return False
    executed_ids: list[str] = []
    for position, (claim, row) in enumerate(zip(declared, rows, strict=True)):
        if not isinstance(claim, dict) or row.get("position") != position:
            return False
        state = claim.get("state")
        if row.get("verification_state") != state or row.get("outcome") != claim.get("outcome"):
            return False
        if state == "not_run":
            if set(claim) != {"state"} or row.get("source_id") is not None:
                return False
            continue
        if state not in {"partial", "passed", "failed"}:
            return False
        label = claim.get("command_label")
        exit_code = claim.get("exit_code")
        termination = claim.get("termination")
        outcome = claim.get("outcome")
        if not nonempty_string(label) or not nonempty_string(outcome):
            return False
        commands = [
            command
            for command in work.commands
            if command.sequence < call.sequence
            and isinstance(command.parsed_command, dict)
            and command.parsed_command.get("cmd") == label
            and command.exit_code == exit_code
            and command.termination == termination
        ]
        if len(commands) != 1:
            return False
        if state == "passed" and not (termination == "exited" and exit_code == 0):
            return False
        if state == "failed" and termination == "exited" and exit_code == 0:
            return False
        source_id = row.get("source_id")
        source = bundle.one("sources", id=source_id, project_id=bundle.project_id)
        if (
            not nonempty_string(source_id)
            or source is None
            or source.get("source_kind") != "command_execution"
            or source.get("locator") != label
            or source.get("exit_code") != exit_code
            or source.get("termination") != termination
            or source.get("actor_kind") != "command"
            or source.get("observer_kind") != "agent"
        ):
            return False
        executed_ids.append(str(source_id))
    return returned_ids == executed_ids


def checkpoint_facts(
    work: CodexCapture | None,
    bundle: CanonicalBundle | None,
    decision_id: str | None,
    goal_context_id: str | None,
    goal_source_id: str | None,
    baseline_analysis_id: str | None,
    goal_statement: str | None,
) -> tuple[bool, bool, bool, str | None, list[str], str | None]:
    call = unique_call(work, "checkpoint_record")
    if call is None or work is None or bundle is None:
        return False, False, False, None, [], None
    checkpoint_id = call.result.get("checkpoint_id")
    checkpoint = bundle.one("checkpoints", id=checkpoint_id, project_id=bundle.project_id)
    if checkpoint is None or not nonempty_string(checkpoint_id):
        return False, False, False, None, [], None
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
        source_id=goal_source_id,
    )
    decision_link = bundle.one(
        "checkpoint_decisions",
        project_id=bundle.project_id,
        checkpoint_id=checkpoint_id,
        decision_id=decision_id,
    )
    next_step = checkpoint.get("next_step")
    goal_linked = (
        nonempty_string(goal_statement)
        and call.arguments.get("goal_context_id") == goal_context_id
        and call.result.get("goal_context_id") == goal_context_id
        and checkpoint.get("goal") == goal_statement
    )
    applied = call.arguments.get("applied_decision_ids")
    verification_ok = checkpoint_verification_facts(
        work, bundle, call, str(checkpoint_id)
    )
    valid = (
        bounded_paths is not None
        and set(bounded_paths) == set(observed_paths)
        and set(bounded_paths) == source_paths
        and call.result.get("changed_paths") == bounded_paths
        and supported is not None
        and decision_link is not None
        and call.arguments.get("project_id") == bundle.project_id
        and call.arguments.get("baseline_analysis_snapshot_id") == baseline_analysis_id
        and call.result.get("baseline_analysis_snapshot_id") == baseline_analysis_id
        and goal_linked
        and isinstance(applied, list)
        and decision_id in applied
        and call.result.get("applied_decision_ids") == applied
        and verification_ok
        and call.arguments.get("next_step") == next_step
        and nonempty_string(next_step)
    )
    return (
        valid,
        goal_linked,
        verification_ok,
        str(checkpoint_id) if nonempty_string(checkpoint_id) else None,
        observed_paths,
        str(next_step) if nonempty_string(next_step) else None,
    )


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
    descriptor_errors = cycle_descriptor_errors(raw)
    evidence = raw.get("evidence") if isinstance(raw.get("evidence"), dict) else {}
    captures = evidence.get("captures") if isinstance(evidence.get("captures"), dict) else {}
    work_reference = captures.get("work")
    resume_reference = captures.get("resume")
    bundle_reference = evidence.get("canonical_bundle")
    work_user_task = raw.get("work_user_task")
    resume_user_task = raw.get("fresh_resume_user_task")
    decision_oracle = raw.get("decision_oracle")
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
    goal_ok, goal_context_id, goal_source_id, goal_statement = goal_facts(
        work_capture, bundle, work_user_task
    )
    baseline_call = unique_call(work_capture, "repository_analyze")
    baseline_analysis_id = (
        baseline_call.result.get("analysis_snapshot_id") if baseline_call is not None else None
    )
    (
        checkpoint_ok,
        checkpoint_goal_ok,
        checkpoint_verification_ok,
        checkpoint_id,
        changed_paths,
        next_step,
    ) = checkpoint_facts(
        work_capture,
        bundle,
        decision_id,
        goal_context_id,
        goal_source_id,
        str(baseline_analysis_id) if nonempty_string(baseline_analysis_id) else None,
        goal_statement,
    )
    question_ok, question_review_basis = question_review_facts(
        work_capture,
        bundle,
        question_id,
        question_revision,
        decision_oracle,
        baseline_call,
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
    cycle_metadata_ok = (
        not descriptor_errors
        and raw.get("kind") == "phase8_cycle_descriptor"
        and raw.get("producer") == "volicord_phase8_codex_event_normalizer"
        and valid_capture_sha256(raw.get("_evidence_file_sha256"))
        and raw.get("repository_class") == kind
        and raw.get("cycle") == cycle
        and raw.get("repository_revision") == repository_revision
        and work_capture is not None
        and work_capture.git_revision == repository_revision
    )
    task_turns_ok = (
        work_capture is not None
        and resume_capture is not None
        and nonempty_string(work_user_task)
        and nonempty_string(resume_user_task)
        and bool(work_capture.user_turns)
        and bool(resume_capture.user_turns)
        and codex_user_turn_transport_identity_matches(
            work_capture.user_turns[0].text,
            work_user_task,
        )
        and codex_user_turn_transport_identity_matches(
            resume_capture.user_turns[0].text,
            resume_user_task,
        )
    )
    prompt_integrity_ok = (
        not descriptor_errors
        and not naturalistic_prompt_errors(work_user_task, resume_user_task, decision_oracle)
        and task_turns_ok
    )
    initialize_call = unique_call(work_capture, "project_initialize")
    goal_call = unique_call(work_capture, "context_record")
    clean_statuses = [
        command
        for command in work_capture.commands
        if command_is_clean_git_status(command.parsed_command)
        and command.exit_code == 0
        and command.output_was_empty
    ] if work_capture is not None else []
    baseline_ok = (
        bundle is not None
        and work_capture is not None
        and first_work_change is not None
        and work_capture.git_revision == repository_revision
        and initialize_call is not None
        and initialize_call.result.get("project_id") == bundle.project_id
        and goal_call is not None
        and baseline_call is not None
        and baseline_call.arguments.get("project_id") == bundle.project_id
        and baseline_call.result.get("project_id") == bundle.project_id
        and nonempty_string(baseline_analysis_id)
        and initialize_call.sequence < goal_call.sequence < baseline_call.sequence < first_work_change
        and any(command.sequence < baseline_call.sequence for command in clean_statuses)
    )

    invocations_ok = (
        work_capture is not None
        and resume_capture is not None
        and work_capture.session_id != resume_capture.session_id
        and work_capture.source == "vscode"
        and resume_capture.source == "vscode"
        and work_capture.originator == "codex_vscode"
        and resume_capture.originator == "codex_vscode"
        and nonempty_string(work_capture.cli_version)
        and nonempty_string(resume_capture.cli_version)
    )
    resolve_call = unique_call(resume_capture, "project_resolve")
    recall_call = unique_call(resume_capture, "recall")
    resolved_binding = (
        resolve_call.result.get("binding")
        if resolve_call is not None and isinstance(resolve_call.result.get("binding"), dict)
        else None
    )
    resolution_ok = (
        resume_capture is not None
        and bundle is not None
        and resolve_call is not None
        and recall_call is not None
        and resolve_call.arguments.get("repository") == str(resume_capture.cwd)
        and resolve_call.result.get("status") == "found"
        and resolve_call.result.get("project_id") == bundle.project_id
        and isinstance(resolved_binding, dict)
        and nonempty_string(resolved_binding.get("binding_id"))
        and isinstance(resolved_binding.get("revision"), int)
        and resolved_binding.get("revision") >= 1
        and resolved_binding.get("canonical_repository_path") == str(resume_capture.cwd)
        and resolved_binding.get("availability") == "available"
        and resolve_call.completion_sequence < recall_call.sequence
        and not resume_capture.successful_calls("project_initialize")
    )
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
    last_continuation_sequence = max(
        (
            item.sequence
            for item in resume_capture.path_observations
            if item.sequence > (first_inspection if first_inspection is not None else -1)
        ),
        default=None,
    ) if resume_capture is not None else None
    resume_validation_ok = meaningful_resume_validation(
        resume_capture,
        last_continuation_sequence
        if last_continuation_sequence is not None
        else first_inspection,
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
        resolution_ok
        and recall_call is not None
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
    recall_checkpoint = recall_call.result.get("checkpoint") if recall_call is not None else None
    recalled_verification = recall_checkpoint.get("verification") if isinstance(recall_checkpoint, dict) else None
    recalled_checkpoint_decisions = (
        recall_checkpoint.get("applied_decisions")
        if isinstance(recall_checkpoint, dict)
        and isinstance(recall_checkpoint.get("applied_decisions"), list)
        and all(nonempty_string(value) for value in recall_checkpoint["applied_decisions"])
        else None
    )
    canonical_verification = sorted(
        (
            row
            for row in bundle.rows("checkpoint_verifications")
            if row.get("checkpoint_id") == checkpoint_id
        ),
        key=lambda row: row.get("position") if isinstance(row.get("position"), int) else -1,
    ) if bundle is not None else []
    expected_recalled_verification = [
        {
            "state": row.get("verification_state"),
            "source_id": row.get("source_id"),
            "outcome": row.get("outcome"),
        }
        for row in canonical_verification
    ]
    recall_match_ok = (
        bundle is not None
        and recall_call is not None
        and recall_call.arguments.get("project_id") == bundle.project_id
        and recall_call.result.get("project_id") == bundle.project_id
        and recall_call.result.get("read_only") is True
        and recalled_checkpoint_row is not None
        and recalled_checkpoint_row.get("id") == checkpoint_id
        and isinstance(recall_checkpoint, dict)
        and recall_checkpoint.get("identity") == checkpoint_id
        and recall_checkpoint.get("goal") == goal_statement
        and recall_checkpoint.get("changed_paths") == changed_paths
        and recall_checkpoint.get("next_step") == next_step
        and recalled_verification == expected_recalled_verification
        and recalled_checkpoint_decisions is not None
        and set(recalled_checkpoint_decisions) == checkpoint_decisions
        and recalled_decisions is not None
        and checkpoint_decisions
        and checkpoint_decisions <= set(recalled_decisions)
        and recalled_context is not None
        and goal_context_id in recalled_context
    )
    recall_goals = recall_call.result.get("goals") if recall_call is not None else None
    recalled_goal_ok = (
        nonempty_string(goal_statement)
        and isinstance(recall_goals, list)
        and all(nonempty_string(goal) for goal in recall_goals)
        and goal_statement in recall_goals
        and recalled_context is not None
        and goal_context_id in recalled_context
    )
    task_revision_ok = (
        work_capture is not None
        and raw.get("repository_revision") == work_capture.git_revision
        and raw.get("repository_revision") == repository_revision
    )
    task_goal_ok = (
        cycle_metadata_ok
        and task_turns_ok
        and task_revision_ok
        and goal_ok
        and checkpoint_ok
        and checkpoint_goal_ok
        and invocations_ok
        and fresh_ok
        and recall_match_ok
        and recalled_goal_ok
    )
    relevant_resume_paths = relevant_continuation_paths(continuation_paths, next_step)
    continuation_ok = (
        recall_match_ok
        and fresh_ok
        and ordering_ok
        and bool(relevant_resume_paths)
        and resume_validation_ok
    )

    checks = {
        "naturalistic_prompt_integrity": evidence_check(references_present, prompt_integrity_ok),
        "plain_task_goal_linkage": evidence_check(references_present, task_goal_ok),
        "clean_bounded_baseline": evidence_check(references_present, baseline_ok),
        "researched_material_question": evidence_check(references_present, question_ok),
        "meaningful_ordinary_changes": evidence_check(references_present, ordinary_ok),
        "source_grounded_checkpoint": evidence_check(references_present, checkpoint_ok),
        "explicit_user_decision_source": evidence_check(references_present, decision_ok),
        "distinct_work_and_resume_invocations": evidence_check(references_present, invocations_ok),
        "fresh_resume_without_prior_context": evidence_check(references_present, fresh_ok),
        "repository_bound_project_resolution": evidence_check(references_present, resolution_ok),
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
        "relevant_resume_paths": relevant_resume_paths,
        "continuation_basis": {
            "fresh_resume_session": fresh_ok,
            "repository_bound_project_resolution": resolution_ok,
            "recall_before_inspection_and_continuation": ordering_ok,
            "checkpoint_supplied_next_meaningful_step": nonempty_string(next_step),
            "observed_change_relevant_to_checkpoint_next_step": bool(relevant_resume_paths),
            "resume_numeric_exit_validation": resume_validation_ok,
        },
        "checkpoint_id": checkpoint_id,
        "goal_context_id": goal_context_id,
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
        "task_goal_basis": {
            "work_user_task_sha256": (
                hashlib.sha256(work_user_task.encode("utf-8")).hexdigest()
                if nonempty_string(work_user_task)
                else None
            ),
            "work_user_task_utf8_length": (
                len(work_user_task.encode("utf-8")) if nonempty_string(work_user_task) else None
            ),
            "fresh_resume_user_task_sha256": (
                hashlib.sha256(resume_user_task.encode("utf-8")).hexdigest()
                if nonempty_string(resume_user_task)
                else None
            ),
            "first_turns_match_descriptor_transport_identity": task_turns_ok,
            "repository_revision_matches": task_revision_ok,
            "checkpoint_call_and_canonical_goal_match": checkpoint_goal_ok,
            "goal_context_matches_descriptor_task": goal_ok,
            "checkpoint_verification_matches_observed_command": checkpoint_verification_ok,
            "fresh_session_recall_goal_identity_and_statement_match": recalled_goal_ok,
        },
        "question_relevance_review": question_review_basis,
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


VOID_HTML_ELEMENTS = {
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link",
    "meta", "param", "source", "track", "wbr",
}


def normalized_text(parts: list[str]) -> str:
    return " ".join(" ".join(parts).split())


class AccessibilityParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.html_lang: str | None = None
        self.heading_levels: list[int] = []
        self.controls: list[dict[str, Any]] = []
        self.labels: list[dict[str, Any]] = []
        self.element_text: dict[str, list[str]] = {}
        self.stack: list[dict[str, Any]] = []
        self.links = 0
        self.viewport = False
        self.styles: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attributes = {name: value for name, value in attrs}
        parent_hidden = bool(self.stack and self.stack[-1]["hidden"])
        inline_style = (attributes.get("style") or "").replace(" ", "").lower()
        hidden = parent_hidden or "hidden" in attributes or attributes.get("aria-hidden", "").lower() == "true"
        hidden = hidden or "display:none" in inline_style or "visibility:hidden" in inline_style
        if tag == "input" and (attributes.get("type") or "text").lower() == "hidden":
            hidden = True
        node: dict[str, Any] = {
            "tag": tag,
            "hidden": hidden,
            "element_id": attributes.get("id"),
            "label_index": None,
            "control_index": None,
        }
        if tag == "html":
            self.html_lang = attributes.get("lang")
        if tag in {"h1", "h2", "h3", "h4", "h5", "h6"} and not hidden:
            self.heading_levels.append(int(tag[1]))
        if tag in {"input", "select", "textarea", "button"}:
            enclosing_labels = [
                int(item["label_index"])
                for item in self.stack
                if item["label_index"] is not None
            ]
            node["control_index"] = len(self.controls)
            self.controls.append({
                "tag": tag,
                "attributes": attributes,
                "hidden": hidden,
                "enclosing_labels": enclosing_labels,
                "text": [],
            })
        if tag == "label":
            node["label_index"] = len(self.labels)
            self.labels.append({"for": attributes.get("for"), "text": []})
        if tag == "a" and attributes.get("href"):
            self.links += 1
        if tag == "meta" and attributes.get("name") == "viewport":
            self.viewport = True
        if node["element_id"]:
            self.element_text.setdefault(str(node["element_id"]), [])
        if tag not in VOID_HTML_ELEMENTS:
            self.stack.append(node)

    def handle_endtag(self, tag: str) -> None:
        for index in range(len(self.stack) - 1, -1, -1):
            if self.stack[index]["tag"] == tag:
                del self.stack[index:]
                return

    def handle_data(self, data: str) -> None:
        if ":root{" in data or "@media" in data or ":focus" in data:
            self.styles.append(data)
        if not data.strip() or (self.stack and self.stack[-1]["hidden"]):
            return
        for node in self.stack:
            if node["element_id"]:
                self.element_text[str(node["element_id"])].append(data)
            if node["label_index"] is not None:
                self.labels[int(node["label_index"])]["text"].append(data)
            control_index = node["control_index"]
            if control_index is not None and self.controls[int(control_index)]["tag"] == "button":
                self.controls[int(control_index)]["text"].append(data)

    def control_has_accessible_name(self, control: dict[str, Any]) -> bool:
        attributes = control["attributes"]
        labelled_by = (attributes.get("aria-labelledby") or "").split()
        if labelled_by and normalized_text([
            normalized_text(self.element_text.get(element_id, []))
            for element_id in labelled_by
        ]):
            return True
        if (attributes.get("aria-label") or "").strip():
            return True
        control_id = attributes.get("id")
        label_indexes = set(control["enclosing_labels"])
        if control_id:
            label_indexes.update(
                index for index, label in enumerate(self.labels) if label["for"] == control_id
            )
        if any(normalized_text(self.labels[index]["text"]) for index in label_indexes):
            return True
        return control["tag"] == "button" and bool(normalized_text(control["text"]))

    def control_summary(self) -> dict[str, int]:
        visible = [control for control in self.controls if not control["hidden"]]
        named = [control for control in visible if self.control_has_accessible_name(control)]
        return {
            "visible_control_count": len(visible),
            "hidden_control_count": len(self.controls) - len(visible),
            "named_control_count": len(named),
            "unlabeled_control_count": len(visible) - len(named),
        }


def parse_accessibility_html(content: str, *, expected_language: str | None) -> dict[str, Any]:
    parser = AccessibilityParser()
    parser.feed(content)
    parser.close()
    style = "\n".join(parser.styles)
    heading_order = all(next_level <= current + 1 for current, next_level in zip(parser.heading_levels, parser.heading_levels[1:]))
    controls = parser.control_summary()
    headings_and_labels = bool(
        parser.heading_levels
        and heading_order
        and controls["unlabeled_control_count"] == 0
    )
    actual_language = parser.html_lang.strip().lower() if parser.html_lang else None
    required_language = expected_language.strip().lower() if expected_language else None
    checks = {
        "keyboard_reachability": "passed" if parser.links + controls["visible_control_count"] > 0 else "partial",
        "visible_focus": "passed" if re.search(r":focus(?:-visible)?", style) else "partial",
        "not_color_only": "partial",
        "headings_and_labels": "passed" if headings_and_labels else "failed",
        "narrow_and_zoomed_presentation": "partial" if parser.viewport else "failed",
        "document_html_language": "passed" if required_language is None or actual_language == required_language else "failed",
    }
    return {
        "checks": checks,
        "html_language": parser.html_lang,
        "heading_count": len(parser.heading_levels),
        "heading_order_valid": heading_order,
        **controls,
        "label_count": len(parser.labels),
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
    peak_memory: dict[str, Any],
    repeated_resources: dict[str, Any],
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
        "peak_memory_bytes": peak_memory.get("peak_memory_bytes"),
        "peak_memory_status": peak_memory.get("status", "environment_blocked"),
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
    resource_qualification = {
        "status": status_from_steps({
            "peak_memory": peak_memory.get("status", "environment_blocked"),
            "repeated_resources": repeated_resources.get("status", "unsupported"),
        }),
        "peak_memory": peak_memory,
        "repeated_resources": repeated_resources,
    }
    return {
        "cycle": cycle,
        "status": status_from_steps(
            {
                "deterministic_v11": deterministic_status,
                "real_session_dogfood": actual["status"],
                "resource_qualification": resource_qualification["status"],
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
        "resource_qualification": resource_qualification,
        "accessibility": accessibility,
    }


def aggregate_resource_qualification(repositories: list[dict[str, Any]]) -> dict[str, Any]:
    observations = [
        cycle.get("resource_qualification", {})
        for repository in repositories
        for cycle in repository.get("cycles", [])
    ]
    statuses = {
        f"observation_{index}": observation.get("status", "unsupported")
        for index, observation in enumerate(observations)
    }
    peaks = [
        observation.get("peak_memory", {}).get("peak_memory_bytes")
        for observation in observations
        if isinstance(observation.get("peak_memory", {}).get("peak_memory_bytes"), int)
    ]
    conclusions = Counter(
        observation.get("repeated_resources", {}).get("conclusion", "unobserved")
        for observation in observations
    )
    return {
        "status": status_from_steps(statuses),
        "observation_count": len(observations),
        "measured_peak_count": len(peaks),
        "maximum_observed_peak_memory_bytes": max(peaks) if peaks else None,
        "repeated_resource_conclusions": dict(sorted(conclusions.items())),
        "universal_product_ceiling_applied": False,
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

    prohibited_fields = {
        "command_log", "raw_command_log", "source_body", "stdout", "stderr",
        "credential", "credentials", "private_prompt",
    }

    def inspect(item: Any) -> None:
        if isinstance(item, dict):
            if prohibited_fields & set(item):
                raise ValueError("sanitized result contains a prohibited raw evidence field")
            for child in item.values():
                inspect(child)
        elif isinstance(item, list):
            for child in item:
                inspect(child)

    inspect(value)


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
            if set(cycle.get("measurements", {})) != set(definition["measurements"]) | {
                "cycle_duration_ms",
                "peak_memory_status",
            }:
                raise ValueError("dogfood cycle measurements are incomplete")
            resources = cycle.get("resource_qualification", {})
            if resources.get("status") not in ALLOWED_STATUS:
                raise ValueError("dogfood cycle has an invalid resource qualification status")
            peak = resources.get("peak_memory", {})
            repeated = resources.get("repeated_resources", {})
            if peak.get("status") not in ALLOWED_STATUS or repeated.get("status") not in ALLOWED_STATUS:
                raise ValueError("dogfood cycle has an invalid resource evidence status")
            if resources.get("status") != status_from_steps({
                "peak_memory": peak["status"],
                "repeated_resources": repeated["status"],
            }):
                raise ValueError("dogfood cycle resource status does not match its evidence")
            if peak.get("status") == "passed" and not (
                isinstance(peak.get("peak_memory_bytes"), int)
                and peak["peak_memory_bytes"] > 0
            ):
                raise ValueError("passed peak-memory evidence has no measured peak")
            if repeated.get("status") == "passed" and (
                repeated.get("unexplained_cumulative_growth_observed") is not False
                or repeated.get("universal_product_ceiling_applied") is not False
                or repeated.get("fixed_input_and_destination") is not True
                or tuple(repeated.get("operations_per_round", []))
                != RESOURCE_OPERATIONS
                or repeated.get("repetition_count")
                != definition["resource_qualification"][
                    "repeated_resource_repetition_count"
                ]
                or len(repeated.get("rounds", []))
                != repeated.get("repetition_count")
            ):
                raise ValueError("passed repeated-resource evidence is not bounded and measured")
            real_invocations.extend(
                [
                    actual.get("work_session_id"),
                    actual.get("resume_session_id"),
                ]
            )
    aggregate_resources = result.get("resource_qualification", {})
    if (
        aggregate_resources != aggregate_resource_qualification(repositories)
        or aggregate_resources.get("status") not in ALLOWED_STATUS
        or aggregate_resources.get("observation_count") != len(CLASSES) * definition["candidate_cycle_count"]
        or aggregate_resources.get("universal_product_ceiling_applied") is not False
        or (
            aggregate_resources.get("status") == "passed"
            and aggregate_resources.get("measured_peak_count")
            != aggregate_resources.get("observation_count")
        )
    ):
        raise ValueError("dogfood aggregate resource qualification is incomplete")
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
        if result.get("resource_qualification", {}).get("status") != "passed":
            raise ValueError("replacement pass requires passed resource qualification")
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
                if cycle.get("resource_qualification", {}).get("status") != "passed":
                    raise ValueError("replacement pass contains unqualified resource evidence")
        if (
            any(not nonempty_string(identity) for identity in real_invocations)
            or len(real_invocations)
            != definition["real_session_evidence"]["full_replacement_session_count"]
            or len(set(real_invocations)) != len(real_invocations)
        ):
            raise ValueError("replacement pass requires twelve globally distinct Codex invocations")
    sanitize_check(result)


def aggregate_status(
    repositories: list[dict[str, Any]],
    regression: dict[str, Any],
    accessibility: dict[str, Any],
    resources: dict[str, Any],
    blockers: list[str],
) -> str:
    statuses = [
        regression.get("status", "failed"),
        accessibility.get("status", "failed"),
        resources.get("status", "failed"),
    ]
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
                    peak_observer = LinuxProcessTreePeakRss(
                        definition["resource_qualification"]["peak_memory_sampling_interval_ms"]
                    )
                    peak_observer.start()
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
                    try:
                        repeated_resources = repeated_resource_rehearsal(
                            kind,
                            cycle_root,
                            recorder,
                            base_env,
                            raw.get("project_id"),
                            definition["resource_qualification"][
                                "repeated_resource_repetition_count"
                            ],
                        )
                    except (OSError, RuntimeError, ValueError) as error:
                        repeated_resources = {
                            "status": "failed",
                            "conclusion": "resource_rehearsal_error",
                            "unexplained_cumulative_growth_observed": None,
                            "error_class": type(error).__name__,
                            "repetition_count": 0,
                            "rounds": [],
                        }
                    peak_memory = peak_observer.stop()
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
                        peak_memory,
                        repeated_resources,
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
                            **{name: None for name in definition["measurements"]},
                            "cycle_duration_ms": None,
                            "peak_memory_status": "environment_blocked",
                        },
                        "resource_qualification": {
                            "status": "environment_blocked",
                            "peak_memory": {
                                "status": "environment_blocked",
                                "peak_memory_bytes": None,
                                "mechanism": definition["resource_qualification"][
                                    "peak_memory_mechanism"
                                ],
                            },
                            "repeated_resources": {
                                "status": "environment_blocked",
                                "conclusion": "repository_prerequisite_unavailable",
                                "unexplained_cumulative_growth_observed": None,
                                "repetition_count": 0,
                                "rounds": [],
                            },
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
    resource_qualification = aggregate_resource_qualification(repository_results)
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
    if resource_qualification.get("status") != "passed":
        blockers.append("peak-memory or repeated-resource qualification did not pass")
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
    status = aggregate_status(
        repository_results,
        regression,
        accessibility,
        resource_qualification,
        blockers,
    )
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
        "resource_qualification": resource_qualification,
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
            "Peak RSS is an observed Linux process-tree sample for each dogfood cycle, not a universal product memory ceiling.",
            "Repeated-resource qualification is a bounded fixed-input rehearsal and does not claim indefinite-duration stability.",
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


def fixture_work_user_task(kind: str, cycle: int) -> str:
    return (
        f"Improve the operator-facing error reporting in the {kind} validation adapter and add focused tests "
        f"for cycle {cycle}. Keep the change local and do not add dependencies."
    )


def fixture_decision_oracle() -> dict[str, Any]:
    return {
        "work_task_materiality_basis": "operator-facing error reporting",
        "user_owned_dimension": "operator-facing detail versus stable concise output",
        "established_repository_facts": [
            "The adapter already distinguishes internal diagnostics from its public error string."
        ],
        "why_repository_inspection_cannot_decide": (
            "Repository structure cannot determine which error detail operators want in normal output."
        ),
        "viable_alternatives": [
            "Keep public errors concise and expose details only in diagnostics",
            "Include the actionable cause in every public error",
        ],
        "recommendation": "Keep public errors concise and expose details only in diagnostics",
        "material_consequence": "The choice changes output stability and troubleshooting usefulness.",
    }


def real_session_fixture(
    kind: str,
    cycle: int,
    revision: str,
    evidence_directory: Path,
) -> dict[str, Any]:
    project = "01" * 16
    user_source = "02" * 16
    goal_source = "03" * 16
    changed_source_one = "04" * 16
    changed_source_two = "05" * 16
    question = "06" * 16
    decision = "07" * 16
    context = "08" * 16
    checkpoint = "09" * 16
    verification_source = "0a" * 16
    baseline_analysis = "0b" * 32
    current_analysis = "0c" * 32
    baseline_repository = "0d" * 32
    current_repository = "0e" * 32
    repository_source = "0f" * 16
    candidate = "10" * 16
    binding = "11" * 16
    work_session = f"{kind}-work-session-{cycle}"
    resume_session = f"{kind}-resume-session-{cycle}"
    work_user_task = fixture_work_user_task(kind, cycle)
    decision_oracle = fixture_decision_oracle()
    decision_turn_text = "Keep the normal output concise; diagnostics can carry the actionable cause."
    resume_user_task = "Continue the validation-adapter improvement from the current project state."
    question_prompt = "Which error-detail boundary should the validation adapter expose to operators?"
    next_step = "Update src/resume.rs to carry the chosen concise diagnostic boundary and verify it"
    work_paths = ["src/existing.rs", "tests/existing.rs"]
    verification_command = "python3 -m unittest tests.test_existing"
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
                "originator": "codex_vscode",
                "cli_version": "0.148.0-alpha.9",
                "source": "vscode",
                "thread_source": "user",
                "model_provider": "openai",
                "git": {"commit_hash": revision, "branch": "phase8"},
            },
        )

    def task(turn_id: str) -> dict[str, Any]:
        return event("event_msg", {"type": "task_started", "turn_id": turn_id, "started_at": 1})

    def task_complete(turn_id: str) -> dict[str, Any]:
        return event("event_msg", {"type": "task_complete", "turn_id": turn_id})

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

    def custom_call(turn_id: str, call_id: str, input_value: str) -> dict[str, Any]:
        return event(
            "response_item",
            {
                "type": "custom_tool_call",
                "id": f"ctc-{call_id}",
                "call_id": call_id,
                "name": "exec",
                "status": "completed",
                "input": input_value,
                "internal_chat_message_metadata_passthrough": {"turn_id": turn_id},
            },
        )

    def custom_output(turn_id: str, call_id: str, structured: dict[str, Any]) -> dict[str, Any]:
        return event(
            "response_item",
            {
                "type": "custom_tool_call_output",
                "id": f"ctco-{call_id}",
                "call_id": call_id,
                "output": [
                    {"type": "input_text", "text": "Script completed\nWall time 0.1 seconds\nOutput:\n"},
                    {"type": "input_text", "text": json.dumps(structured, separators=(",", ":"))},
                ],
                "internal_chat_message_metadata_passthrough": {"turn_id": turn_id},
            },
        )

    def mcp_call(
        turn_id: str,
        call_id: str,
        operation: str,
        arguments: dict[str, Any],
        *,
        fallback: str = "||",
        json_wrapper: bool = False,
    ) -> dict[str, Any]:
        if fallback not in {"||", "??"}:
            raise AssertionError("fixture MCP fallback is not supported")
        encoded = json.dumps(arguments, separators=(",", ":"))
        forwarding = (
            "text(JSON.stringify(r));\n"
            if json_wrapper
            else f'for(const c of (r.content{fallback}[])) if(c.type==="text") text(c.text);\n'
        )
        return custom_call(
            turn_id,
            call_id,
            f"const r=await tools.mcp__volicord__{operation}({encoded});\n{forwarding}",
        )

    def mcp_completion(
        call_id: str,
        operation: str,
        arguments: dict[str, Any],
        structured: dict[str, Any],
        *,
        server: str = "volicord",
        is_error: bool = False,
    ) -> dict[str, Any]:
        return event(
            "event_msg",
            {
                "type": "mcp_tool_call_end",
                "call_id": f"exec-{call_id}",
                "invocation": {
                    "server": server,
                    "tool": operation,
                    "arguments": arguments,
                },
                "duration": {"secs": 0, "nanos": 1},
                "result": {
                    "Ok": {
                        "content": [{"type": "text", "text": json.dumps(structured)}],
                        "structuredContent": structured,
                        "isError": is_error,
                    }
                },
            },
        )

    def command_call(turn_id: str, call_id: str, command: str) -> dict[str, Any]:
        arguments = json.dumps(
            {"cmd": command, "workdir": "/phase8/repository", "yield_time_ms": 30000},
            separators=(",", ":"),
        )
        return custom_call(
            turn_id,
            call_id,
            f"const r=await tools.exec_command({arguments});\ntext(r);\n",
        )

    def with_mcp_completions(events: list[dict[str, Any]]) -> list[dict[str, Any]]:
        outputs: dict[str, dict[str, Any]] = {}
        for value in events:
            payload = value.get("payload", {})
            if payload.get("type") != "custom_tool_call_output":
                continue
            output = payload.get("output")
            if isinstance(output, list) and len(output) == 2:
                try:
                    structured = json.loads(output[1].get("text", ""))
                except json.JSONDecodeError:
                    continue
                if isinstance(structured, dict):
                    outputs[str(payload.get("call_id"))] = structured
        expanded: list[dict[str, Any]] = []
        for value in events:
            expanded.append(value)
            payload = value.get("payload", {})
            wrapper = (
                parse_mcp_wrapper(payload.get("input"))
                if payload.get("type") == "custom_tool_call"
                else None
            )
            call_id = str(payload.get("call_id"))
            if wrapper is not None and call_id in outputs:
                expanded.append(
                    mcp_completion(
                        call_id,
                        wrapper.operation,
                        wrapper.arguments,
                        outputs[call_id],
                    )
                )
        return expanded

    work_turn = f"{kind}-work-turn-{cycle}"
    decision_turn = f"{kind}-decision-turn-{cycle}"
    initialize_call = f"{kind}-initialize-call-{cycle}"
    goal_call = f"{kind}-goal-call-{cycle}"
    status_call = f"{kind}-status-call-{cycle}"
    baseline_call = f"{kind}-baseline-call-{cycle}"
    candidate_submit_call = f"{kind}-candidate-submit-call-{cycle}"
    candidate_research_call = f"{kind}-candidate-research-call-{cycle}"
    candidate_ready_call = f"{kind}-candidate-ready-call-{cycle}"
    candidate_promote_call = f"{kind}-candidate-promote-call-{cycle}"
    inquiry_call = f"{kind}-inquiry-call-{cycle}"
    decision_call = f"{kind}-decision-call-{cycle}"
    patch_call = f"{kind}-patch-call-{cycle}"
    verification_call = f"{kind}-verification-call-{cycle}"
    checkpoint_call = f"{kind}-checkpoint-call-{cycle}"
    patch_text = "*** Begin Patch\n*** Update File: /phase8/repository/src/existing.rs\n@@\n-old\n+new\n*** Update File: /phase8/repository/tests/existing.rs\n@@\n-old\n+new\n*** End Patch"
    work_events = [
        session_meta(work_session),
        task(work_turn),
        user(work_turn, f"{kind}-user-turn-{cycle}", work_user_task),
        mcp_call(
            work_turn,
            initialize_call,
            "project_initialize",
            {"display_name": "Phase 8 fixture", "repository": "/phase8/repository"},
        ),
        custom_output(work_turn, initialize_call, {"project_id": project}),
        mcp_call(
            work_turn,
            goal_call,
            "context_record",
            {
                "project_id": project,
                "user_turn": work_user_task,
                "role": "goal",
                "statement": work_user_task,
            },
            json_wrapper=True,
        ),
        custom_output(
            work_turn,
            goal_call,
            {
                "project_id": project,
                "source_id": goal_source,
                "context_item_id": context,
                "revision": 1,
                "role": "goal",
            },
        ),
        command_call(work_turn, status_call, "git status --short"),
        custom_output(work_turn, status_call, {"output": "", "exit_code": 0}),
        mcp_call(work_turn, baseline_call, "repository_analyze", {"project_id": project}),
        custom_output(
            work_turn,
            baseline_call,
            {
                "project_id": project,
                "analysis_snapshot_id": baseline_analysis,
                "repository_snapshot_id": baseline_repository,
            },
        ),
        mcp_call(
            work_turn,
            candidate_submit_call,
            "candidate_manage",
            {
                "action": "submit_question",
                "project_id": project,
                "source_ids": [repository_source],
                "source_operation": "repository_analyze",
                "repository_snapshot": baseline_repository,
                "research_state": "research_required",
                "research_state_basis": decision_oracle["why_repository_inspection_cannot_decide"],
                "retention_basis": "current work session",
                "bounded_summary": "Choose the operator-facing error detail boundary",
                "prompt": question_prompt,
                "why_now": decision_oracle["material_consequence"],
                "affected_scope": [decision_oracle["user_owned_dimension"]],
                "established_facts": decision_oracle["established_repository_facts"],
                "assumptions": [],
                "uncertainty": [decision_oracle["why_repository_inspection_cannot_decide"]],
                "alternatives": [
                    {"key": "concise", "label": decision_oracle["viable_alternatives"][0], "consequence": "Stable public output"},
                    {"key": "detailed", "label": decision_oracle["viable_alternatives"][1], "consequence": "More immediate detail"},
                ],
                "recommendation_key": "concise",
                "recommendation_rationale": decision_oracle["recommendation"],
                "trade_offs": [decision_oracle["material_consequence"]],
                "known_limits": [],
                "what_unlocks": ["ordinary implementation work"],
                "materiality_rationale": decision_oracle["material_consequence"],
                "duplicate_basis": "canonical inspection found no matching Question",
                "presentation_order": 1,
            },
        ),
        custom_output(
            work_turn,
            candidate_submit_call,
            {
                "action": "submit_question",
                "state": "stored",
                "candidate_id": candidate,
                "candidate_revision": 1,
                "research_state": "research_required",
                "canonical_mutation": False,
            },
        ),
        mcp_call(
            work_turn,
            candidate_research_call,
            "candidate_manage",
            {
                "action": "attach_repository_research",
                "project_id": project,
                "candidate_id": candidate,
                "capability": "structural",
                "coverage": "current adapter and tests",
                "freshness": "current",
                "source_ids": [repository_source],
                "evidence_assessment": "sufficient",
                "limits": [],
            },
        ),
        custom_output(
            work_turn,
            candidate_research_call,
            {
                "action": "attach_repository_research",
                "candidate_id": candidate,
                "candidate_revision": 2,
                "research_state": "research_required",
                "repository_research": [{"source_ids": [repository_source]}],
                "canonical_mutation": False,
                "promoted": False,
            },
        ),
        mcp_call(
            work_turn,
            candidate_ready_call,
            "candidate_manage",
            {"action": "mark_research_ready", "project_id": project, "candidate_id": candidate},
        ),
        custom_output(
            work_turn,
            candidate_ready_call,
            {
                "action": "mark_research_ready",
                "candidate_id": candidate,
                "candidate_revision": 3,
                "research_state": "ready_to_ask",
                "canonical_mutation": False,
                "promoted": False,
            },
        ),
        mcp_call(
            work_turn,
            candidate_promote_call,
            "candidate_manage",
            {"action": "promote_question", "project_id": project, "candidate_id": candidate},
        ),
        custom_output(
            work_turn,
            candidate_promote_call,
            {
                "action": "promote_question",
                "candidate_id": candidate,
                "question_id": question,
                "canonical_replayed": False,
                "candidate_reconciled": True,
            },
        ),
        mcp_call(
            work_turn,
            inquiry_call,
            "inquiry_frontier",
            {"project_id": project},
        ),
        custom_output(
            work_turn,
            inquiry_call,
            {
                "project_id": project,
                "questions": [
                    {
                        "identity": question,
                        "revision": 1,
                        "prompt": question_prompt,
                    }
                ],
                "diagnostics": [],
            },
        ),
        task_complete(work_turn),
        task(decision_turn),
        user(decision_turn, f"{kind}-decision-user-turn-{cycle}", decision_turn_text),
        mcp_call(
            decision_turn,
            decision_call,
            "decision_record",
            {
                "project_id": project,
                "question_id": question,
                "question_revision": 1,
                "alternative_key": "concise",
                "user_turn": decision_turn_text,
            },
        ),
        custom_output(
            decision_turn,
            decision_call,
            {
                "project_id": project,
                "user_response_source_id": user_source,
                "all_succeeded": True,
                "outcomes": [{"question_id": question, "revision": 1, "outcome": "recorded"}],
            },
        ),
        custom_call(
            decision_turn,
            patch_call,
            f"const patch={json.dumps(patch_text)};\ntext(await tools.apply_patch(patch));\n",
        ),
        event(
            "event_msg",
            {
                "type": "patch_apply_end",
                "call_id": f"{kind}-work-patch-{cycle}",
                "turn_id": decision_turn,
                "stdout": "",
                "stderr": "",
                "success": True,
                "changes": {
                    f"/phase8/repository/{path}": {
                        "type": "update",
                        "unified_diff": "@@ -1 +1 @@\n-old\n+new\n",
                        "move_path": None,
                    }
                    for path in work_paths
                },
                "status": "completed",
            },
        ),
        custom_output(decision_turn, patch_call, {}),
        command_call(decision_turn, verification_call, verification_command),
        custom_output(
            decision_turn,
            verification_call,
            {"output": "Ran focused tests\nOK\n", "exit_code": 0},
        ),
        mcp_call(
            decision_turn,
            checkpoint_call,
            "checkpoint_record",
            {
                "project_id": project,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": baseline_analysis,
                "kind": "handoff",
                "work_state": "paused",
                "state_change": "Updated the bounded implementation and test",
                "applied_decision_ids": [decision],
                "verification": [
                    {
                        "state": "passed",
                        "command_label": verification_command,
                        "exit_code": 0,
                        "termination": "exited",
                        "outcome": "focused tests passed",
                    }
                ],
                "next_step": next_step,
                "known_limits": [],
                "handoff_to": "next Codex session",
            },
        ),
        custom_output(
            decision_turn,
            checkpoint_call,
            {
                "checkpoint_id": checkpoint,
                "revision": 1,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": baseline_analysis,
                "current_analysis_snapshot_id": current_analysis,
                "baseline_repository_snapshot_id": baseline_repository,
                "current_repository_snapshot_id": current_repository,
                "changed_paths": work_paths,
                "applied_decision_ids": [decision],
                "verification_source_ids": [verification_source],
            },
        ),
        task_complete(decision_turn),
    ]

    resume_turn = f"{kind}-resume-turn-{cycle}"
    resolve_call = f"{kind}-resolve-call-{cycle}"
    recall_call = f"{kind}-recall-call-{cycle}"
    inspect_call = f"{kind}-inspect-call-{cycle}"
    resume_patch_call = f"{kind}-resume-patch-call-{cycle}"
    resume_verification_call = f"{kind}-resume-verification-call-{cycle}"
    resume_verification_command = "python3 -m unittest tests.test_resume"
    resume_patch_text = "*** Begin Patch\n*** Update File: /phase8/repository/src/resume.rs\n@@\n+continued\n*** End Patch"
    resume_events = [
        session_meta(resume_session),
        task(resume_turn),
        user(resume_turn, f"{kind}-resume-user-turn-{cycle}", resume_user_task),
        mcp_call(
            resume_turn,
            resolve_call,
            "project_resolve",
            {"repository": "/phase8/repository"},
            fallback="??",
        ),
        custom_output(
            resume_turn,
            resolve_call,
            {
                "status": "found",
                "project_id": project,
                "display_name": "Phase 8 fixture",
                "project_revision": 1,
                "binding": {
                    "binding_id": binding,
                    "revision": 1,
                    "canonical_repository_path": "/phase8/repository",
                    "availability": "available",
                    "clone_identity": None,
                    "worktree_identity": None,
                },
            },
        ),
        mcp_call(
            resume_turn,
            recall_call,
            "recall",
            {"project_id": project},
            fallback="??",
        ),
        custom_output(
            resume_turn,
            recall_call,
            {
                "project_id": project,
                "project_name": "Phase 8 fixture",
                "goals": [work_user_task],
                "decisions": [{"identity": decision, "revision": 1, "state": "active", "choice": "concise", "rationale": None}],
                "open_questions": [],
                "known_limits": [],
                "next_step": next_step,
                "checkpoint": {
                    "identity": checkpoint,
                    "revision": 1,
                    "kind": "handoff",
                    "goal": work_user_task,
                    "work_state": "paused",
                    "state_change": "Updated the bounded implementation and test",
                    "source_basis": [goal_source],
                    "changed_source_basis": [changed_source_one, changed_source_two],
                    "changed_paths": work_paths,
                    "applied_decisions": [decision],
                    "verification": [
                        {
                            "state": "passed",
                            "source_id": verification_source,
                            "outcome": "focused tests passed",
                        }
                    ],
                    "known_limits": [],
                    "non_goals": [],
                    "open_questions": [],
                    "next_step": next_step,
                    "handoff_to": "next Codex session",
                },
                "omitted_count": 0,
                "read_only": True,
            },
        ),
        mcp_call(
            resume_turn,
            inspect_call,
            "repository_understanding",
            {"project_id": project},
            fallback="??",
        ),
        custom_output(
            resume_turn,
            inspect_call,
            {"health": "available", "overview": {}, "repository_map": {}, "decision_context_code": [], "issues": [], "read_only": True},
        ),
        custom_call(
            resume_turn,
            resume_patch_call,
            f"const patch={json.dumps(resume_patch_text)};\ntext(await tools.apply_patch(patch));\n",
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
                "changes": {
                    "/phase8/repository/src/resume.rs": {
                        "type": "update",
                        "unified_diff": "@@ -0,0 +1 @@\n+continued\n",
                        "move_path": None,
                    }
                },
                "status": "completed",
            },
        ),
        custom_output(resume_turn, resume_patch_call, {}),
        command_call(resume_turn, resume_verification_call, resume_verification_command),
        custom_output(
            resume_turn,
            resume_verification_call,
            {"output": "Ran resumed tests\nOK\n", "exit_code": 0},
        ),
        task_complete(resume_turn),
    ]
    work_events = with_mcp_completions(work_events)
    resume_events = with_mcp_completions(resume_events)
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

    def framed(value: bytes) -> bytes:
        return len(value).to_bytes(8, "big") + value

    def encoded_source_ids(values: list[str]) -> bytes:
        return len(values).to_bytes(8, "big") + b"".join(
            framed(bytes.fromhex(value)) for value in values
        )

    def encoded_alternatives(values: list[str]) -> str:
        raw = len(values).to_bytes(8, "big")
        for index, value in enumerate(values):
            raw += framed(("concise" if index == 0 else "detailed").encode())
            raw += framed(value.encode())
            raw += framed(f"Consequence for {value}".encode())
        return raw.hex()

    def encoded_established_facts(values: list[str]) -> str:
        raw = len(values).to_bytes(8, "big")
        for value in values:
            raw += framed(value.encode())
            raw += framed(encoded_source_ids([goal_source]))
            raw += b"\x01" + framed(b"structural")
            raw += framed(b"current")
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
        [blob(goal_source), blob(project), integer(1), text("current_host_user_turn"), text(work_user_task), null(), text("codex"), text(work_session), null(), null(), text("user"), text("fixture-user"), null(), null(), text("available"), integer(2)],
        [blob(changed_source_one), blob(project), integer(1), text("file"), text(work_paths[0]), text(revision), null(), null(), null(), null(), text("repository"), text("codex-observer"), null(), null(), text("available"), integer(3)],
        [blob(changed_source_two), blob(project), integer(1), text("file"), text(work_paths[1]), text(revision), null(), null(), null(), null(), text("repository"), text("codex-observer"), null(), null(), text("available"), integer(4)],
        [blob(verification_source), blob(project), integer(1), text("command_execution"), text(verification_command), null(), null(), null(), integer(0), text("exited"), text("command"), text("current-host-reported-command"), text("agent"), text("codex"), text("available"), integer(5)],
        [blob(repository_source), blob(project), integer(1), text("repository_snapshot"), text(revision), text(revision), null(), null(), null(), null(), text("repository"), text("local-repository-observer"), text("agent"), text("codex"), text("available"), integer(6)],
    ]
    tables = [
        table("sources", source_columns, sources),
        table("questions", ["id", "project_id", "revision", "terminal_outcome", "created_at", "updated_at"], [[blob(question), blob(project), integer(1), text("answered"), integer(1), integer(1)]]),
        table("question_revisions", ["question_id", "revision", "project_id", "prompt_basis", "source_basis", "dependencies", "alternatives", "recommendation_key", "recommendation_rationale", "recommendation_sources", "trade_offs", "uncertainty", "material_scope", "materiality", "presentation_order", "why_it_matters_now", "established_facts", "assumptions", "known_limits", "answer_unlocks", "allowed_dispositions", "research_state", "recorded_at"], [[blob(question), integer(1), blob(project), text(question_prompt), blob(encoded_source_ids([goal_source]).hex()), blob(encoded_strings([])), blob(encoded_alternatives(decision_oracle["viable_alternatives"])), text("concise"), text(decision_oracle["recommendation"]), blob(encoded_source_ids([goal_source]).hex()), blob(encoded_strings([decision_oracle["material_consequence"]])), blob(encoded_strings([])), blob(encoded_strings([decision_oracle["user_owned_dimension"]])), text("material"), integer(1), text(decision_oracle["material_consequence"]), blob(encoded_established_facts(decision_oracle["established_repository_facts"])), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings(["ordinary implementation work"])), blob(encoded_strings(["deferred"])), text("researched"), integer(1)]]),
        table("question_response_sources", ["project_id", "question_id", "question_revision", "source_id", "recorded_at"], [[blob(project), blob(question), integer(1), blob(user_source), integer(1)]]),
        table("question_decision_history_witnesses", ["project_id", "question_id", "question_revision", "root_decision_id", "terminal_outcome", "response_source_id", "response_authority", "creation_kind", "created_at"], [[blob(project), blob(question), integer(1), blob(decision), text("answered"), blob(user_source), text("current_host_user_turn"), text("alternative"), integer(1)]]),
        table("decisions", ["id", "project_id", "revision", "question_id", "question_revision", "user_turn_source_id", "user_authority", "choice_kind", "choice_value", "user_rationale", "displayed_alternatives", "recommendation_key", "recommendation_rationale", "recommendation_sources", "applicability_paths", "applicability_components", "applicability_work_contexts", "assumptions", "revisit_triggers", "recorded_at"], [[blob(decision), blob(project), integer(1), blob(question), integer(1), blob(user_source), text("current_host_user_turn"), text("alternative"), text("concise"), null(), blob(encoded_alternatives(decision_oracle["viable_alternatives"])), text("concise"), text(decision_oracle["recommendation"]), blob(encoded_source_ids([goal_source]).hex()), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), integer(1)]]),
        table("context_items", ["id", "project_id", "revision", "role", "statement", "provenance_role", "author_kind", "author_identity", "applicability_paths", "applicability_components", "applicability_work_contexts", "recorded_at"], [[blob(context), blob(project), integer(1), text("goal"), text(work_user_task), text("user_statement"), text("user"), text("fixture-user"), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), integer(1)]]),
        table("context_item_sources", ["project_id", "context_item_id", "source_id", "position"], [[blob(project), blob(context), blob(goal_source), integer(0)]]),
        table("checkpoints", ["id", "project_id", "revision", "checkpoint_kind", "goal", "work_state", "state_change", "changed_paths", "user_review", "user_review_source_id", "user_acceptance", "user_acceptance_source_id", "known_limits", "non_goals", "next_step", "handoff_to", "recorded_at"], [[blob(checkpoint), blob(project), integer(1), text("handoff"), text(work_user_task), text("paused"), text("Updated the bounded implementation and test"), blob(encoded_strings(work_paths)), text("not_requested"), null(), text("not_requested"), null(), blob(encoded_strings([])), blob(encoded_strings([])), text(next_step), text("next Codex session"), integer(1)]]),
        table("checkpoint_source_relations", ["project_id", "checkpoint_id", "relation_kind", "source_id", "position"], [[blob(project), blob(checkpoint), text("supported_by"), blob(goal_source), integer(0)], [blob(project), blob(checkpoint), text("changed_basis"), blob(changed_source_one), integer(0)], [blob(project), blob(checkpoint), text("changed_basis"), blob(changed_source_two), integer(1)]]),
        table("checkpoint_decisions", ["project_id", "checkpoint_id", "decision_id", "position"], [[blob(project), blob(checkpoint), blob(decision), integer(0)]]),
        table("checkpoint_verifications", ["project_id", "checkpoint_id", "position", "verification_state", "source_id", "outcome"], [[blob(project), blob(checkpoint), integer(0), text("passed"), blob(verification_source), text("focused tests passed")]]),
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
        "kind": "phase8_cycle_descriptor",
        "producer": "volicord_phase8_codex_event_normalizer",
        "_evidence_file_sha256": "0" * 64,
        "_evidence_directory": str(evidence_directory),
        "repository_class": kind,
        "cycle": cycle,
        "repository_revision": revision,
        "work_user_task": work_user_task,
        "fresh_resume_user_task": resume_user_task,
        "decision_oracle": decision_oracle,
        "evidence": {
            "captures": {
                "work": {"file": work_capture.name, "sha256": sha256(work_capture)},
                "resume": {"file": resume_capture.name, "sha256": sha256(resume_capture)},
            },
            "canonical_bundle": {"file": bundle_path.name, "sha256": sha256(bundle_path)},
        },
    }


def expect_rejected(result: dict[str, Any], definition: dict[str, Any], message: str) -> None:
    try:
        validate_result(result, definition)
    except ValueError:
        return
    raise AssertionError(message)


def self_test() -> int:
    definition = load_definition()
    v11 = load_v11()
    descriptor_task = "Preserve exact prompt identity."
    transport_identity_cases = (
        ("exact text", descriptor_task, True),
        ("one terminal LF", descriptor_task + "\n", True),
        ("one terminal CRLF", descriptor_task + "\r\n", True),
        ("two terminal newlines", descriptor_task + "\n\n", False),
        ("trailing space", descriptor_task + " ", False),
        ("space before terminal LF", descriptor_task + " \n", False),
        ("leading whitespace", " " + descriptor_task, False),
        ("interior difference", descriptor_task.replace("exact", "altered"), False),
        ("terminal tab", descriptor_task + "\t", False),
        ("terminal Unicode whitespace", descriptor_task + "\u00a0", False),
    )
    for label, captured, expected in transport_identity_cases:
        if codex_user_turn_transport_identity_matches(captured, descriptor_task) is not expected:
            raise AssertionError(f"Codex user-turn transport identity mishandled {label}")
    if codex_user_turn_transport_identity_matches(descriptor_task, None):
        raise AssertionError("non-text descriptor qualified as Codex user-turn identity")
    revision = "0" * 40
    temporary = tempfile.TemporaryDirectory(prefix="volicord-phase8-self-test-")
    evidence_directory = Path(temporary.name)
    process_recorder = v11.Recorder(evidence_directory / "resource-processes")
    procfs_unavailability = linux_process_tree_procfs_unavailability()
    observer = LinuxProcessTreePeakRss(
        definition["resource_qualification"]["peak_memory_sampling_interval_ms"]
    )
    observer.start()
    successful_process = process_recorder.run(
        "successful-capture",
        [
            sys.executable,
            "-c",
            "import sys,time; x=bytearray(16*1024*1024); "
            "sys.stdout.buffer.write(b'complete stdout\\n'); "
            "sys.stderr.buffer.write(b'complete stderr\\n'); time.sleep(.12)",
        ],
        os.environ.copy(),
        cwd=ROOT,
    )
    failed_process = process_recorder.run(
        "failed-capture",
        [sys.executable, "-c", "raise SystemExit(23)"],
        os.environ.copy(),
        cwd=ROOT,
    )
    terminated_process = process_recorder.run(
        "terminated-capture",
        [
            sys.executable,
            "-c",
            "import os,signal; os.kill(os.getpid(), signal.SIGTERM)",
        ],
        os.environ.copy(),
        cwd=ROOT,
    )
    after_failure_process = process_recorder.run(
        "after-failure",
        [sys.executable, "-c", "print('continued')"],
        os.environ.copy(),
        cwd=ROOT,
    )
    process_peak = observer.stop()
    if procfs_unavailability is None:
        if (
            process_peak["status"] != "passed"
            or not isinstance(process_peak["peak_memory_bytes"], int)
            or process_peak["peak_memory_bytes"] <= 0
        ):
            raise AssertionError("capable Linux procfs did not produce measured peak RSS")
    elif (
        process_peak["status"] != "environment_blocked"
        or process_peak["measurement_error"] != procfs_unavailability
    ):
        raise AssertionError("unavailable process-tree procfs was not truthfully classified")
    if (
        Path(successful_process["stdout"]).read_bytes() != b"complete stdout\n"
        or Path(successful_process["stderr"]).read_bytes() != b"complete stderr\n"
        or successful_process["exit_code"] != 0
        or failed_process["exit_code"] != 23
        or terminated_process["termination"] != {"kind": "signal", "number": 15}
        or after_failure_process["exit_code"] != 0
    ):
        raise AssertionError("resource observation changed process output, exit, or non-fail-fast truth")
    stable_rounds = [
        {
            "operations": {
                name: {"exit_code": 0, "termination": None}
                for name in RESOURCE_OPERATIONS
            },
            "runtime_home_bytes": runtime,
            "derived_state_bytes": derived,
            "document_output_bytes": document,
        }
        for runtime, derived, document in (
            (100, 20, 10),
            (120, 24, 10),
            (120, 24, 10),
            (120, 24, 10),
        )
    ]
    stable_resources = repeated_resource_conclusion(stable_rounds)
    if (
        stable_resources["status"] != "passed"
        or stable_resources["unexplained_cumulative_growth_observed"] is not False
    ):
        raise AssertionError("stable repeated resources did not qualify")
    growing_rounds = json.loads(json.dumps(stable_rounds))
    for index, round_value in enumerate(growing_rounds):
        round_value["runtime_home_bytes"] = 100 + (index * 10)
    growing_resources = repeated_resource_conclusion(growing_rounds)
    if (
        growing_resources["status"] != "failed"
        or growing_resources["unexplained_cumulative_growth_observed"] is not True
    ):
        raise AssertionError("unexplained cumulative resource growth qualified")
    if repeated_resource_conclusion(stable_rounds[:2])["status"] != "unsupported":
        raise AssertionError("unobserved repeated resources were treated as measured")
    incomplete_rounds = json.loads(json.dumps(stable_rounds))
    incomplete_rounds[0]["operations"].pop("document_projection")
    if repeated_resource_conclusion(incomplete_rounds)["status"] != "unsupported":
        raise AssertionError("incomplete repeated-operation evidence qualified")
    failed_rounds = json.loads(json.dumps(stable_rounds))
    failed_rounds[1]["operations"]["repository_analysis"]["exit_code"] = 7
    if repeated_resource_conclusion(failed_rounds)["status"] != "failed":
        raise AssertionError("failed repeated resource operation qualified")

    def install_no_replace_resource_fake(cycle_root: Path, kind: str) -> Path:
        fake = cycle_root / "work" / kind / "prefix/bin/volicord"
        fake.parent.mkdir(parents=True, exist_ok=True)
        fake.write_text(
            f"#!{sys.executable}\n"
            "import os\n"
            "from pathlib import Path\n"
            "import sys\n"
            "if sys.argv[1:3] == ['documents', 'export']:\n"
            "    destination = Path(sys.argv[-2])\n"
            "    try:\n"
            "        with destination.open('xb') as output:\n"
            "            output.write(b'fixed no-replace document\\n')\n"
            "    except FileExistsError:\n"
            "        raise SystemExit(17)\n"
            "    if os.environ.get('PHASE8_FAKE_DOCUMENT_FAIL_AFTER_CREATE') == '1':\n"
            "        raise SystemExit(19)\n"
            "raise SystemExit(0)\n",
            encoding="utf-8",
        )
        fake.chmod(0o755)
        return fake

    rehearsal_root = evidence_directory / "rehearsal-pass"
    install_no_replace_resource_fake(rehearsal_root, "volicord")
    rehearsal = repeated_resource_rehearsal(
        "volicord",
        rehearsal_root,
        v11.Recorder(evidence_directory / "rehearsal-pass-processes"),
        os.environ.copy(),
        "11" * 16,
        definition["resource_qualification"]["repeated_resource_repetition_count"],
    )
    rehearsal_destination = (
        rehearsal_root
        / "work/volicord/repeated-resource/project-architecture-guide.html"
    )
    if (
        rehearsal["status"] != "passed"
        or len(rehearsal["rounds"]) != 4
        or any(
            round_value["operations"]["document_projection"]["exit_code"] != 0
            or not isinstance(round_value["document_output_bytes"], int)
            or round_value["document_output_bytes"] <= 0
            for round_value in rehearsal["rounds"]
        )
        or rehearsal_destination.exists()
        or rehearsal_destination.is_symlink()
    ):
        raise AssertionError("actual repeated no-replace rehearsal did not pass every round")

    preexisting_root = evidence_directory / "rehearsal-preexisting"
    install_no_replace_resource_fake(preexisting_root, "small-python")
    preexisting_destination = (
        preexisting_root
        / "work/small-python/repeated-resource/project-architecture-guide.html"
    )
    preexisting_destination.parent.mkdir(parents=True, exist_ok=True)
    preexisting_destination.write_bytes(b"pre-existing destination\n")
    preexisting_recorder = v11.Recorder(evidence_directory / "rehearsal-preexisting-processes")
    preexisting = repeated_resource_rehearsal(
        "small-python",
        preexisting_root,
        preexisting_recorder,
        os.environ.copy(),
        "22" * 16,
        definition["resource_qualification"]["repeated_resource_repetition_count"],
    )
    if (
        preexisting["status"] != "failed"
        or preexisting["conclusion"] != "rehearsal_destination_preexisting"
        or preexisting_recorder.sequence != 0
        or preexisting_destination.read_bytes() != b"pre-existing destination\n"
    ):
        raise AssertionError("pre-existing rehearsal destination was not preserved and rejected")

    failed_export_root = evidence_directory / "rehearsal-failed-export"
    install_no_replace_resource_fake(failed_export_root, "polyglot-medium")
    failed_export_environment = os.environ.copy()
    failed_export_environment["PHASE8_FAKE_DOCUMENT_FAIL_AFTER_CREATE"] = "1"
    failed_export = repeated_resource_rehearsal(
        "polyglot-medium",
        failed_export_root,
        v11.Recorder(evidence_directory / "rehearsal-failed-export-processes"),
        failed_export_environment,
        "33" * 16,
        definition["resource_qualification"]["repeated_resource_repetition_count"],
    )
    failed_export_destination = (
        failed_export_root
        / "work/polyglot-medium/repeated-resource/project-architecture-guide.html"
    )
    if (
        failed_export["status"] != "failed"
        or failed_export["conclusion"]
        != "failed_document_export_created_unowned_destination"
        or not failed_export_destination.is_file()
    ):
        raise AssertionError("failed export incorrectly acquired rehearsal cleanup ownership")

    external_fixture = real_session_fixture("volicord", 1, revision, evidence_directory)
    external_fixture.pop("_evidence_file_sha256")
    external_fixture.pop("_evidence_directory")
    external_fixture_path = evidence_directory / "cycle-evidence.json"
    write_json(external_fixture_path, external_fixture)
    loaded_fixture = load_real_session_cycle(
        external_fixture_path.name,
        evidence_directory,
    )
    external_result = real_session_evidence(
        loaded_fixture,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )
    if external_result["status"] != "passed":
        raise AssertionError("external sanitized process evidence did not qualify")
    if (
        "recall" in external_fixture["fresh_resume_user_task"].casefold()
        or external_result["checks"]["recall_precedes_inspection_and_continuation"]
        != "passed"
    ):
        raise AssertionError("automatic Recall was not observed from a plain resume request")
    if (
        external_result["question_relevance_review"].get("automatic_relevance_conclusion")
        is not None
        or external_result["question_relevance_review"].get("manual_review_required")
        is not True
        or external_result["question_relevance_review"].get("exact_repository_fact_matches")
        != 1
        or external_result["question_relevance_review"].get("exact_alternative_matches")
        != 2
    ):
        raise AssertionError("hidden oracle review was absent or automatically declared Question quality")
    representative_capture = load_codex_capture(CURRENT_MCP_FIXTURE)
    representative_calls = representative_capture.calls("context_record")
    if (
        len(representative_calls) != 1
        or representative_calls[0].outcome != "succeeded"
        or representative_calls[0].result.get("context_item_id") != "08" * 16
        or b"text(JSON.stringify(x))" not in CURRENT_MCP_FIXTURE.read_bytes()
    ):
        raise AssertionError("current JSON.stringify wrapper fixture did not normalize from MCP completion")
    valid_descriptor = {
        "kind": "phase8_cycle_descriptor",
        "repository_class": "volicord",
        "cycle": 1,
        "repository_revision": revision,
        "work_user_task": fixture_work_user_task("volicord", 1),
        "fresh_resume_user_task": (
            "Continue the validation-adapter improvement from the current project state."
        ),
        "decision_oracle": fixture_decision_oracle(),
    }
    if cycle_descriptor_errors(valid_descriptor):
        raise AssertionError("valid naturalistic plain-task descriptor was rejected")
    for label, mutation in (
        ("missing work task", lambda value: value.pop("work_user_task")),
        ("missing oracle", lambda value: value.pop("decision_oracle")),
        (
            "missing work materiality basis",
            lambda value: value["decision_oracle"].pop("work_task_materiality_basis"),
        ),
        (
            "materiality basis absent from work task",
            lambda value: value["decision_oracle"].update(
                {"work_task_materiality_basis": "a requirement absent from both tasks"}
            ),
        ),
        (
            "materiality basis only in resume task",
            lambda value: (
                value["decision_oracle"].update(
                    {"work_task_materiality_basis": "resume-only materiality requirement"}
                ),
                value.update(
                    {
                        "fresh_resume_user_task": value["fresh_resume_user_task"]
                        + " Preserve the resume-only materiality requirement."
                    }
                ),
            ),
        ),
        (
            "scripted resume",
            lambda value: value.update({"fresh_resume_user_task": "Invoke Recall before continuing."}),
        ),
        ("obsolete reserved scope", lambda value: value.update({"resume_change_scope": ["src/resume.rs"]})),
    ):
        invalid_descriptor = json.loads(json.dumps(valid_descriptor))
        mutation(invalid_descriptor)
        errors = cycle_descriptor_errors(invalid_descriptor)
        expected_error = {
            "missing work task": "work_user_task",
            "missing oracle": "decision_oracle",
            "missing work materiality basis": "work_task_materiality_basis",
            "materiality basis absent from work task": "absent from work_user_task",
            "materiality basis only in resume task": "appears only in fresh_resume_user_task",
            "scripted resume": "Recall",
            "obsolete reserved scope": "obsolete field",
        }[label]
        if not any(expected_error in error for error in errors):
            raise AssertionError(f"{label} descriptor qualified")
    for fallback in ("||", "??"):
        parsed = parse_mcp_wrapper(
            "const r=await tools.mcp__volicord__recall({\"project_id\":\"01\"});\n"
            f'for(const c of (r.content{fallback}[])) if(c.type==="text") text(c.text);\n'
        )
        if parsed is None:
            raise AssertionError(f"static {fallback} MCP wrapper did not correlate")
    multiple_mcp_calls = (
        "const r=await tools.mcp__volicord__recall({\"project_id\":\"01\"});\n"
        'text(JSON.stringify(r));\nawait tools.mcp__volicord__project_health({});\n'
    )
    if parse_mcp_wrapper(multiple_mcp_calls) is not None:
        raise AssertionError("multiple MCP invocations escaped bounded wrapper correlation")
    command_arguments = (
        '{"cmd":"python3 -m unittest tests.test_existing",'
        '"workdir":"/phase8/repository","yield_time_ms":30000}'
    )
    complete_result_wrapper = (
        f"const r=await tools.exec_command({command_arguments});\ntext(r);\n"
    )
    correlated_split_wrapper = (
        f"const r=await tools.exec_command({command_arguments});\n"
        "text(r.output); text(JSON.stringify({exit_code:r.exit_code}));\n"
    )
    complete_parsed = parse_custom_call(complete_result_wrapper)
    correlated_parsed = parse_custom_call(correlated_split_wrapper)
    if complete_parsed is None or complete_parsed.output_mode != "result":
        raise AssertionError("complete command result forwarding is no longer supported")
    if correlated_parsed is None or correlated_parsed.output_mode != "correlated_split":
        raise AssertionError("same-result correlated command forwarding was not recognized")
    unsupported_split_wrappers = {
        "literal exit status": (
            f"const r=await tools.exec_command({command_arguments});\n"
            'text(r.output); text("{\\"exit_code\\":0}");\n'
        ),
        "detached exit status": (
            f"const r=await tools.exec_command({command_arguments});\n"
            "const status=r.exit_code; text(r.output); "
            "text(JSON.stringify({exit_code:status}));\n"
        ),
        "multiple command results": (
            f"const r=await tools.exec_command({command_arguments});\n"
            f"const other=await tools.exec_command({command_arguments});\n"
            "text(r.output); text(JSON.stringify({exit_code:r.exit_code}));\n"
        ),
        "unsupported template forwarding": (
            f"const r=await tools.exec_command({command_arguments});\n"
            "text(r.output); text(`exit=${r.exit_code}`);\n"
        ),
    }
    for label, wrapper in unsupported_split_wrappers.items():
        if parse_custom_call(wrapper) is not None:
            raise AssertionError(f"{label} escaped command result correlation")
    positive_work_capture = load_codex_capture(
        evidence_directory / external_fixture["evidence"]["captures"]["work"]["file"]
    )
    positive_resume_path = (
        evidence_directory / external_fixture["evidence"]["captures"]["resume"]["file"]
    )
    positive_resume_capture = load_codex_capture(positive_resume_path)
    recall_calls = positive_resume_capture.successful_calls("recall")
    inspection_calls = positive_resume_capture.successful_calls("repository_understanding")
    if (
        len(recall_calls) != 1
        or len(inspection_calls) != 1
        or recall_calls[0].sequence >= inspection_calls[0].sequence
        or b"r.content??[]" not in positive_resume_path.read_bytes()
    ):
        raise AssertionError("actual-style nullish MCP forwarding did not preserve Recall ordering")
    expected_work_operations = {
        "project_initialize", "context_record", "repository_analyze", "candidate_manage",
        "inquiry_frontier", "decision_record",
        "checkpoint_record",
    }
    observed_work_operations = {
        call.operation for call in positive_work_capture.tool_calls if call.outcome == "succeeded"
    }
    if not expected_work_operations <= observed_work_operations:
        raise AssertionError("current MCP completions did not yield the expected work operations")
    if "project_resolve" not in {
        call.operation for call in positive_resume_capture.tool_calls if call.outcome == "succeeded"
    }:
        raise AssertionError("real resume capture omitted repository-bound Project resolution")

    descriptor_identity = hashlib.sha256(
        json.dumps(external_fixture, sort_keys=True).encode("utf-8")
    ).hexdigest()
    try:
        build_work_blocker_result(
            revision,
            external_fixture,
            descriptor_identity,
            positive_work_capture,
        )
    except ValueError as error:
        if "no machine-observable terminal work blocker" not in str(error):
            raise
    else:
        raise AssertionError("positive work session converted into an early-stop failure")

    zero_workflow_path = evidence_directory / "zero-volicord-completed-work.jsonl"
    positive_work_path = (
        evidence_directory / external_fixture["evidence"]["captures"]["work"]["file"]
    )
    zero_workflow_events = [
        json.loads(line) for line in positive_work_path.read_text(encoding="utf-8").splitlines()
    ]
    zero_workflow_events = [
        value
        for value in zero_workflow_events
        if value.get("payload", {}).get("type") != "mcp_tool_call_end"
    ]
    zero_workflow_path.write_text(
        "".join(
            json.dumps(value, separators=(",", ":")) + "\n"
            for value in zero_workflow_events
        ),
        encoding="utf-8",
    )
    zero_workflow_capture = load_codex_capture(zero_workflow_path)
    blocker_result = build_work_blocker_result(
        revision,
        external_fixture,
        descriptor_identity,
        zero_workflow_capture,
    )
    if (
        blocker_result["kind"] != "phase8_dogfood_blocker_result"
        or blocker_result["failed_checks"] != list(WORK_BLOCKER_CHECKS)
        or set(blocker_result["later_required_evidence"].values()) != {"not_run"}
    ):
        raise AssertionError("zero-Volicord completed work capture was not a terminal blocker")
    transport_blocker_events = json.loads(json.dumps(zero_workflow_events))
    for value in transport_blocker_events:
        payload = value.get("payload", {})
        if value.get("type") == "event_msg" and payload.get("type") == "user_message":
            payload["message"] = str(payload.get("message", "")) + "\n"
            break
    else:
        raise AssertionError("work-blocker fixture has no initial user turn")
    transport_blocker_path = evidence_directory / "transport-lf-zero-volicord-work.jsonl"
    transport_blocker_path.write_text(
        "".join(
            json.dumps(value, separators=(",", ":")) + "\n"
            for value in transport_blocker_events
        ),
        encoding="utf-8",
    )
    transport_blocker_sha256 = sha256(transport_blocker_path)
    transport_blocker_result = build_work_blocker_result(
        revision,
        external_fixture,
        descriptor_identity,
        load_codex_capture(transport_blocker_path),
    )
    if (
        transport_blocker_result["failed_checks"] != list(WORK_BLOCKER_CHECKS)
        or sha256(transport_blocker_path) != transport_blocker_sha256
    ):
        raise AssertionError("work-blocker transport LF regression did not qualify immutably")
    serialized_blocker = json.dumps(blocker_result, sort_keys=True)
    if any(
        hidden in serialized_blocker
        for hidden in (
            external_fixture["work_user_task"],
            external_fixture["fresh_resume_user_task"],
            external_fixture["decision_oracle"]["work_task_materiality_basis"],
            *external_fixture["decision_oracle"]["viable_alternatives"],
            external_fixture["decision_oracle"]["recommendation"],
        )
    ):
        raise AssertionError("work-blocker result retained task or hidden oracle content")
    blocker_descriptor_path = evidence_directory / "work-blocker-descriptor.json"
    blocker_output_path = evidence_directory / "work-blocker-result.json"
    write_json(blocker_descriptor_path, external_fixture)
    current_candidate = git_head(ROOT)
    if current_candidate is None:
        raise AssertionError("work-blocker CLI self-test could not resolve current candidate")
    blocker_cli = subprocess.run(
        [
            sys.executable,
            "-B",
            str(Path(__file__).resolve()),
            "qualify-work-blocker",
            "--candidate-head",
            current_candidate,
            "--descriptor",
            str(blocker_descriptor_path),
            "--work-capture",
            str(zero_workflow_path),
            "--output",
            str(blocker_output_path),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if (
        blocker_cli.returncode != 1
        or not blocker_output_path.is_file()
        or json.loads(blocker_output_path.read_text(encoding="utf-8")).get("kind")
        != "phase8_dogfood_blocker_result"
    ):
        raise AssertionError("qualify-work-blocker CLI did not emit the failure-only result")
    for forbidden_true in (
        "campaign_complete",
        "replacement_pass_candidate",
        "phase_9_ready",
    ):
        invalid_blocker = json.loads(json.dumps(blocker_result))
        invalid_blocker[forbidden_true] = True
        try:
            validate_blocker_result(invalid_blocker)
        except ValueError:
            pass
        else:
            raise AssertionError(f"work-blocker result allowed {forbidden_true}=true")

    incomplete_work_path = evidence_directory / "incomplete-zero-volicord-work.jsonl"
    incomplete_work_path.write_text(
        "".join(
            json.dumps(value, separators=(",", ":")) + "\n"
            for value in zero_workflow_events
            if value.get("payload", {}).get("type")
            not in {"task_complete", "task_completed"}
        ),
        encoding="utf-8",
    )
    try:
        build_work_blocker_result(
            revision,
            external_fixture,
            descriptor_identity,
            load_codex_capture(incomplete_work_path),
        )
    except ValueError as error:
        if "not machine-observably completed" not in str(error):
            raise
    else:
        raise AssertionError("incomplete work capture produced an early-stop result")
    if len(positive_work_capture.calls("context_record")) != 1:
        raise AssertionError("wrapper and completion duplicated one semantic MCP operation")
    decision_sequence = unique_call(positive_work_capture, "decision_record").sequence
    checkpoint_sequence = unique_call(positive_work_capture, "checkpoint_record").sequence
    work_patch_sequence = positive_work_capture.path_observations[0].sequence
    resume_patch_sequence = positive_resume_capture.path_observations[0].sequence
    resume_validation_sequence = next(
        command.sequence
        for command in positive_resume_capture.commands
        if isinstance(command.parsed_command, dict)
        and command.parsed_command.get("cmd") == "python3 -m unittest tests.test_resume"
    )
    if not (
        decision_sequence < work_patch_sequence < checkpoint_sequence
        and recall_calls[0].sequence < inspection_calls[0].sequence
        < resume_patch_sequence < resume_validation_sequence
    ):
        raise AssertionError("content-derived Decision/patch/Checkpoint/Recall ordering changed")
    full_result_commands = [
        command
        for command in positive_work_capture.commands
        if isinstance(command.parsed_command, dict)
        and command.parsed_command.get("cmd")
        == "python3 -m unittest tests.test_existing"
    ]
    if len(full_result_commands) != 1 or full_result_commands[0].exit_code != 0:
        raise AssertionError("complete-result command forwarding did not expose exit code zero")
    serialized_external_result = json.dumps(external_result, sort_keys=True)
    hidden_values = [
        fixture_work_user_task("volicord", 1),
        fixture_decision_oracle()["work_task_materiality_basis"],
        *fixture_decision_oracle()["viable_alternatives"],
        fixture_decision_oracle()["recommendation"],
    ]
    if any(value in serialized_external_result for value in hidden_values):
        raise AssertionError("sanitized result retained a plain task or hidden oracle text")
    fake_steps = {name: "passed" for name in definition["required_product_steps"]}
    valid_html = (
        "<!doctype html><html lang=\"en\"><head>"
        "<meta name=\"viewport\" content=\"width=device-width\">"
        "<style>:focus{outline:2px solid}</style></head>"
        "<body><h1>Project</h1><a href=\"/\">Overview</a>"
        "<form><input type=\"hidden\" name=\"csrf\">"
        "<label>Kind <select name=\"kind\"><option>Guide</option></select></label>"
        "<label for=\"destination\">Destination</label><input id=\"destination\">"
        "<textarea aria-label=\"Current user turn\"></textarea>"
        "<span id=\"alternative-name\">Alternative</span>"
        "<input aria-labelledby=\"alternative-name\">"
        "<button type=\"submit\">Export document</button></form></body></html>"
    )
    parsed = parse_accessibility_html(valid_html, expected_language="en")
    if (
        parsed["checks"]["headings_and_labels"] != "passed"
        or parsed["visible_control_count"] != 5
        or parsed["hidden_control_count"] != 1
        or parsed["named_control_count"] != 5
    ):
        raise AssertionError("viewer-shaped label, ARIA, hidden-input, and button names did not qualify")
    unlabeled = parse_accessibility_html(
        "<!doctype html><html lang=\"en\"><body><h1>Project</h1>"
        "<input name=\"unlabeled\"><button></button></body></html>",
        expected_language="en",
    )
    if (
        unlabeled["checks"]["headings_and_labels"] != "failed"
        or unlabeled["unlabeled_control_count"] != 2
    ):
        raise AssertionError("unlabeled visible controls qualified")
    malformed_heading = parse_accessibility_html(
        "<!doctype html><html lang=\"en\"><body><h1>Project</h1><h3>Skipped</h3></body></html>",
        expected_language="en",
    )
    if malformed_heading["checks"]["headings_and_labels"] != "failed":
        raise AssertionError("malformed heading hierarchy qualified")
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

    synthetic_repeated_resources = {
        **stable_resources,
        "repetition_count": 4,
        "operations_per_round": list(RESOURCE_OPERATIONS),
        "fixed_input_and_destination": True,
        "universal_product_ceiling_applied": False,
        "rounds": stable_rounds,
    }
    qualifying_peak = {
        "status": "passed",
        "peak_memory_bytes": 1024,
        "mechanism": "linux_procfs_process_tree_rss_sampling",
        "sampling_interval_ms": definition["resource_qualification"][
            "peak_memory_sampling_interval_ms"
        ],
        "sample_count": 1,
        "maximum_observed_process_count": 1,
        "measurement_error": None,
        "scope": "dogfood_harness_and_descendant_processes",
    }
    synthetic_resource_qualification = {
        "status": "passed",
        "peak_memory": qualifying_peak,
        "repeated_resources": synthetic_repeated_resources,
    }
    synthetic_measurements = {
        **{name: None for name in definition["measurements"]},
        "cycle_duration_ms": 1.0,
        "peak_memory_bytes": qualifying_peak["peak_memory_bytes"],
        "peak_memory_status": "passed",
    }

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
                "measurements": synthetic_measurements,
                "resource_qualification": synthetic_resource_qualification,
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
        "resource_qualification": aggregate_resource_qualification(repositories),
        "decision_revisit": {"observed_active_triggers": []},
    }
    validate_result(result, definition)
    weakened_session_contract = json.loads(json.dumps(definition))
    weakened_session_contract["real_session_evidence"]["full_replacement_session_count"] = 11
    expect_rejected(
        result,
        weakened_session_contract,
        "replacement passage no longer required twelve distinct real sessions",
    )

    unavailable_peak = {
        **qualifying_peak,
        "status": "environment_blocked",
        "peak_memory_bytes": None,
        "sample_count": 0,
        "maximum_observed_process_count": 0,
        "measurement_error": "linux_procfs_children_unavailable:PermissionError",
    }
    unavailable_resources = json.loads(json.dumps(result))
    unavailable_resources["status"] = "environment_blocked"
    unavailable_resources["replacement_pass_candidate"] = False
    unavailable_resources["blockers"] = ["required peak-memory measurement unavailable"]
    for repository in unavailable_resources["repositories"]:
        repository["status"] = "environment_blocked"
        for cycle in repository["cycles"]:
            cycle["status"] = "environment_blocked"
            cycle["measurements"]["peak_memory_bytes"] = None
            cycle["measurements"]["peak_memory_status"] = "environment_blocked"
            cycle["resource_qualification"] = {
                "status": "environment_blocked",
                "peak_memory": unavailable_peak,
                "repeated_resources": synthetic_repeated_resources,
            }
    unavailable_resources["resource_qualification"] = aggregate_resource_qualification(
        unavailable_resources["repositories"]
    )
    validate_result(unavailable_resources, definition)
    unavailable_as_pass = json.loads(json.dumps(unavailable_resources))
    unavailable_as_pass["status"] = "passed"
    unavailable_as_pass["replacement_pass_candidate"] = True
    unavailable_as_pass["blockers"] = []
    expect_rejected(
        unavailable_as_pass,
        definition,
        "environment-blocked required resource measurement qualified replacement",
    )

    blocked = json.loads(json.dumps(result))
    blocked["status"] = "environment_blocked"
    blocked["replacement_pass_candidate"] = False
    blocked["blockers"] = ["missing repository"]
    validate_result(blocked, definition)
    leaked = json.loads(json.dumps(blocked))
    leaked["private_prompt"] = "private prompt body"
    expect_rejected(leaked, definition, "sanitizer accepted private prompt content")
    raw_log_leak = json.loads(json.dumps(blocked))
    raw_log_leak["resource_qualification"]["command_log"] = "raw.log"
    expect_rejected(raw_log_leak, definition, "sanitizer accepted a raw resource command log")
    local_path_leak = json.loads(json.dumps(blocked))
    local_path_leak["resource_qualification"]["artifact"] = "/tmp/private/resource.json"
    expect_rejected(local_path_leak, definition, "sanitizer accepted a local resource path")
    active = json.loads(json.dumps(result))
    active["decision_revisit"]["observed_active_triggers"] = [{"decision_id": "Q5"}]
    expect_rejected(active, definition, "replacement pass accepted a Decision revisit trigger")

    v11_only = json.loads(json.dumps(result))
    v11_only["repositories"][0]["cycles"][0]["real_session_dogfood"] = real_session_evidence(
        None, kind="volicord", cycle=1, repository_revision=revision
    )
    expect_rejected(v11_only, definition, "V11-only evidence qualified as real dogfood")

    def capture_events(fixture: dict[str, Any], name: str) -> tuple[Path, list[dict[str, Any]]]:
        reference = fixture["evidence"]["captures"][name]
        path = evidence_directory / reference["file"]
        return path, [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]

    def store_capture(
        fixture: dict[str, Any], name: str, path: Path, events: list[dict[str, Any]]
    ) -> None:
        path.write_text(
            "".join(json.dumps(value, separators=(",", ":")) + "\n" for value in events),
            encoding="utf-8",
        )
        fixture["evidence"]["captures"][name]["sha256"] = sha256(path)

    def append_initial_task_transport(
        fixture: dict[str, Any],
        capture: str,
        suffix: str,
    ) -> None:
        path, events = capture_events(fixture, capture)
        for value in events:
            payload = value.get("payload", {})
            if value.get("type") == "event_msg" and payload.get("type") == "user_message":
                payload["message"] = str(payload.get("message", "")) + suffix
                store_capture(fixture, capture, path, events)
                return
        raise AssertionError("fixture has no initial user task turn")

    def mutate_bundle(fixture: dict[str, Any], mutation: Callable[[dict[str, Any]], None]) -> None:
        reference = fixture["evidence"]["canonical_bundle"]
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

    def mutate_mcp_call(
        fixture: dict[str, Any],
        capture: str,
        operation: str,
        mutation: Callable[[dict[str, Any]], None],
    ) -> None:
        path, events = capture_events(fixture, capture)
        marker = f"tools.mcp__volicord__{operation}("
        for value in events:
            payload = value.get("payload", {})
            input_value = payload.get("input")
            if payload.get("type") != "custom_tool_call" or not isinstance(input_value, str) or marker not in input_value:
                continue
            wrapper = parse_mcp_wrapper(input_value)
            if wrapper is None or wrapper.operation != operation:
                raise AssertionError("fixture MCP call does not use the current bounded wrapper")
            arguments = json.loads(json.dumps(wrapper.arguments))
            mutation(arguments)
            old_encoded = json.dumps(wrapper.arguments, separators=(",", ":"))
            encoded = json.dumps(arguments, separators=(",", ":"))
            payload["input"] = input_value.replace(
                f"tools.mcp__volicord__{operation}({old_encoded})",
                f"tools.mcp__volicord__{operation}({encoded})",
                1,
            )
            call_id = str(payload.get("call_id"))
            for completion in events:
                completion_payload = completion.get("payload", {})
                if (
                    completion_payload.get("type") == "mcp_tool_call_end"
                    and completion_payload.get("call_id") == f"exec-{call_id}"
                ):
                    completion_payload["invocation"]["arguments"] = arguments
            store_capture(fixture, capture, path, events)
            return
        raise AssertionError(f"fixture {operation} call was not found")

    def mutate_custom_output(
        fixture: dict[str, Any],
        capture: str,
        call_marker: str,
        mutation: Callable[[dict[str, Any]], None],
    ) -> None:
        path, events = capture_events(fixture, capture)
        for value in events:
            payload = value.get("payload", {})
            if payload.get("type") != "custom_tool_call_output" or call_marker not in str(payload.get("call_id")):
                continue
            output = payload.get("output")
            if not isinstance(output, list) or len(output) != 2:
                raise AssertionError("fixture custom output does not use the current shape")
            structured = json.loads(output[1]["text"])
            mutation(structured)
            output[1]["text"] = json.dumps(structured, separators=(",", ":"))
            call_id = str(payload.get("call_id"))
            for completion in events:
                completion_payload = completion.get("payload", {})
                if (
                    completion_payload.get("type") == "mcp_tool_call_end"
                    and completion_payload.get("call_id") == f"exec-{call_id}"
                ):
                    completion_payload["result"]["Ok"]["structuredContent"] = json.loads(
                        json.dumps(structured)
                    )
            store_capture(fixture, capture, path, events)
            return
        raise AssertionError("fixture custom call result was not found")

    def replace_custom_call_input(
        fixture: dict[str, Any],
        capture: str,
        call_marker: str,
        replacement: str,
    ) -> None:
        path, events = capture_events(fixture, capture)
        for value in events:
            payload = value.get("payload", {})
            if payload.get("type") == "custom_tool_call" and call_marker in str(payload.get("call_id")):
                payload["input"] = replacement
                store_capture(fixture, capture, path, events)
                return
        raise AssertionError("fixture custom call was not found")

    def replace_command_observation(
        fixture: dict[str, Any],
        capture: str,
        call_marker: str,
        wrapper: str,
        output_text: str,
        status_text: str,
        *,
        remove_mcp_completion: bool = False,
    ) -> None:
        path, events = capture_events(fixture, capture)
        matched_call_ids: set[str] = set()
        matched_output_ids: set[str] = set()
        for value in events:
            payload = value.get("payload", {})
            if payload.get("type") == "custom_tool_call" and call_marker in str(payload.get("call_id")):
                payload["input"] = wrapper
                matched_call_ids.add(str(payload.get("call_id")))
        for value in events:
            payload = value.get("payload", {})
            if (
                payload.get("type") == "custom_tool_call_output"
                and str(payload.get("call_id")) in matched_call_ids
            ):
                payload["output"] = [
                    {
                        "type": "input_text",
                        "text": "Script completed\nWall time 0.1 seconds\nOutput:\n",
                    },
                    {"type": "input_text", "text": output_text},
                    {"type": "input_text", "text": status_text},
                ]
                matched_output_ids.add(str(payload.get("call_id")))
        if len(matched_call_ids) != 1:
            raise AssertionError("fixture command call was not uniquely found")
        if matched_output_ids != matched_call_ids:
            raise AssertionError("fixture command output was not uniquely found")
        if remove_mcp_completion:
            events = [
                value
                for value in events
                if not (
                    value.get("payload", {}).get("type") == "mcp_tool_call_end"
                    and value.get("payload", {}).get("call_id")
                    in {f"exec-{call_id}" for call_id in matched_call_ids}
                )
            ]
        store_capture(fixture, capture, path, events)

    def remove_custom_output(
        fixture: dict[str, Any], capture: str, call_marker: str
    ) -> None:
        path, events = capture_events(fixture, capture)
        events = [
            value
            for value in events
            if not (
                value.get("payload", {}).get("type") == "custom_tool_call_output"
                and call_marker in str(value.get("payload", {}).get("call_id"))
            )
        ]
        store_capture(fixture, capture, path, events)

    def mutate_mcp_completion(
        fixture: dict[str, Any],
        capture: str,
        call_marker: str,
        mutation: Callable[[dict[str, Any]], None],
    ) -> None:
        path, events = capture_events(fixture, capture)
        for value in events:
            payload = value.get("payload", {})
            if (
                payload.get("type") == "mcp_tool_call_end"
                and call_marker in str(payload.get("call_id"))
            ):
                mutation(payload)
                store_capture(fixture, capture, path, events)
                return
        raise AssertionError("fixture MCP completion was not found")

    def remove_mcp_completion(
        fixture: dict[str, Any], capture: str, call_marker: str
    ) -> None:
        path, events = capture_events(fixture, capture)
        events = [
            value
            for value in events
            if not (
                value.get("payload", {}).get("type") == "mcp_tool_call_end"
                and call_marker in str(value.get("payload", {}).get("call_id"))
            )
        ]
        store_capture(fixture, capture, path, events)

    def replace_initial_task_text(
        fixture: dict[str, Any], replacement: Callable[[str], str]
    ) -> None:
        path, events = capture_events(fixture, "work")
        user_events = [
            value
            for value in events
            if value.get("type") == "event_msg"
            and value.get("payload", {}).get("type") == "user_message"
        ]
        if not user_events:
            raise AssertionError("fixture has no initial user task turn")
        old_text = user_events[0]["payload"]["message"]
        new_text = replacement(old_text)
        user_events[0]["payload"]["message"] = new_text
        store_capture(fixture, "work", path, events)
        for operation in ("decision_record", "context_record"):
            mutate_mcp_call(
                fixture,
                "work",
                operation,
                lambda arguments, old=old_text, new=new_text: arguments.update(
                    {"user_turn": new}
                ) if arguments.get("user_turn") == old else None,
            )

        def replace_source_locator(bundle: dict[str, Any]) -> None:
            for table_value in bundle["payload"]["tables"]:
                if table_value["name"] != "sources":
                    continue
                kind_index = table_value["columns"].index("source_kind")
                locator_index = table_value["columns"].index("locator")
                for row in table_value["rows"]:
                    if (
                        row[kind_index].get("value") == "current_host_user_turn"
                        and row[locator_index].get("value") == old_text
                    ):
                        row[locator_index] = {"type": "text", "value": new_text}

        mutate_bundle(fixture, replace_source_locator)

    def replace_checkpoint_call_goal(fixture: dict[str, Any], _goal: str) -> None:
        mutate_mcp_call(
            fixture,
            "work",
            "checkpoint_record",
            lambda arguments: arguments.update({"goal_context_id": "ff" * 16}),
        )

    def replace_checkpoint_next_step(fixture: dict[str, Any], next_step: str) -> None:
        mutate_mcp_call(
            fixture,
            "work",
            "checkpoint_record",
            lambda arguments: arguments.update({"next_step": next_step}),
        )

        def replace_bundle_next_step(bundle: dict[str, Any]) -> None:
            for table_value in bundle["payload"]["tables"]:
                if table_value["name"] == "checkpoints":
                    index = table_value["columns"].index("next_step")
                    table_value["rows"][0][index] = {"type": "text", "value": next_step}

        mutate_bundle(fixture, replace_bundle_next_step)
        mutate_custom_output(
            fixture,
            "resume",
            "recall-call",
            lambda output: (
                output.update({"next_step": next_step}),
                output["checkpoint"].update({"next_step": next_step}),
            ),
        )

    def replace_recall_goals(fixture: dict[str, Any], goals: list[str]) -> None:
        mutate_custom_output(
            fixture,
            "resume",
            "recall-call",
            lambda output: output.update({"goals": goals}),
        )

    def replace_canonical_goal(
        bundle: dict[str, Any], goal: str, *, include_context: bool
    ) -> None:
        for table_value in bundle["payload"]["tables"]:
            if table_value["name"] == "checkpoints":
                goal_index = table_value["columns"].index("goal")
                table_value["rows"][0][goal_index] = {"type": "text", "value": goal}
            if include_context and table_value["name"] == "context_items":
                statement_index = table_value["columns"].index("statement")
                table_value["rows"][0][statement_index] = {"type": "text", "value": goal}

    def replace_work_task(fixture: dict[str, Any], task_text: str) -> None:
        replace_initial_task_text(fixture, lambda _text: task_text)
        fixture["work_user_task"] = task_text
        mutate_bundle(
            fixture,
            lambda bundle: replace_canonical_goal(bundle, task_text, include_context=True),
        )
        replace_recall_goals(fixture, [task_text])
        mutate_custom_output(
            fixture,
            "resume",
            "recall-call",
            lambda output: output["checkpoint"].update({"goal": task_text}),
        )

    def replace_resume_task(fixture: dict[str, Any], task_text: str) -> None:
        path, events = capture_events(fixture, "resume")
        user_events = [
            value
            for value in events
            if value.get("type") == "event_msg"
            and value.get("payload", {}).get("type") == "user_message"
        ]
        if len(user_events) != 1:
            raise AssertionError("fixture resume capture does not have one user task")
        user_events[0]["payload"]["message"] = task_text
        store_capture(fixture, "resume", path, events)
        fixture["fresh_resume_user_task"] = task_text

    transport_fixture = real_session_fixture(
        "small-python", 2, revision, evidence_directory
    )
    append_initial_task_transport(transport_fixture, "work", "\n")
    append_initial_task_transport(transport_fixture, "resume", "\r\n")
    transport_descriptor_tasks = (
        transport_fixture["work_user_task"],
        transport_fixture["fresh_resume_user_task"],
    )
    transport_references = json.loads(json.dumps(transport_fixture["evidence"]))
    transport_paths = [
        evidence_directory / transport_fixture["evidence"]["captures"][name]["file"]
        for name in ("work", "resume")
    ]
    transport_hashes = [sha256(path) for path in transport_paths]
    transport_result = real_session_evidence(
        transport_fixture,
        kind="small-python",
        cycle=2,
        repository_revision=revision,
    )
    if (
        transport_result["status"] != "passed"
        or transport_result["checks"]["naturalistic_prompt_integrity"] != "passed"
        or transport_result["checks"]["plain_task_goal_linkage"] != "passed"
    ):
        raise AssertionError("single Codex transport line endings did not qualify full work/resume identity")
    if (
        transport_descriptor_tasks
        != (
            transport_fixture["work_user_task"],
            transport_fixture["fresh_resume_user_task"],
        )
        or transport_references != transport_fixture["evidence"]
        or transport_hashes != [sha256(path) for path in transport_paths]
    ):
        raise AssertionError("Codex transport identity qualification mutated source evidence")

    for label, capture, suffix in (
        ("two terminal work newlines", "work", "\n\n"),
        ("work trailing space", "work", " "),
        ("two terminal resume newlines", "resume", "\r\n\r\n"),
        ("resume trailing space", "resume", "\t"),
    ):
        rejected_transport = real_session_fixture(
            "small-python", 2, revision, evidence_directory
        )
        append_initial_task_transport(rejected_transport, capture, suffix)
        rejected_result = real_session_evidence(
            rejected_transport,
            kind="small-python",
            cycle=2,
            repository_revision=revision,
        )
        if (
            rejected_result["checks"]["naturalistic_prompt_integrity"] != "failed"
            or rejected_result["checks"]["plain_task_goal_linkage"] != "failed"
        ):
            raise AssertionError(f"{label} qualified full prompt identity")

    original_task = fixture_work_user_task("volicord", 1)

    missing_completion = real_session_fixture("volicord", 1, revision, evidence_directory)
    remove_mcp_completion(missing_completion, "work", "decision-call")
    if real_session_evidence(
        missing_completion, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["explicit_user_decision_source"] != "failed":
        raise AssertionError("MCP wrapper without authoritative completion qualified")

    wrapper_mismatch = real_session_fixture("volicord", 1, revision, evidence_directory)
    mismatch_path, mismatch_events = capture_events(wrapper_mismatch, "work")
    for value in mismatch_events:
        payload = value.get("payload", {})
        input_value = payload.get("input")
        if payload.get("type") == "custom_tool_call" and "decision-call" in str(payload.get("call_id")):
            payload["input"] = str(input_value).replace(
                "mcp__volicord__decision_record", "mcp__volicord__recall", 1
            )
            break
    store_capture(wrapper_mismatch, "work", mismatch_path, mismatch_events)
    if real_session_evidence(
        wrapper_mismatch, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["explicit_user_decision_source"] != "failed":
        raise AssertionError("wrapper/completion operation disagreement qualified")

    completion_error = real_session_fixture("volicord", 1, revision, evidence_directory)
    mutate_mcp_completion(
        completion_error,
        "work",
        "decision-call",
        lambda payload: payload.update({
            "result": {
                "Ok": {
                    "content": [{"type": "text", "text": "sanitized MCP failure"}],
                    "structuredContent": {"error": "sanitized MCP failure"},
                    "isError": True,
                }
            }
        }),
    )
    error_capture = load_codex_capture(
        evidence_directory / completion_error["evidence"]["captures"]["work"]["file"]
    )
    failed_decisions = error_capture.calls("decision_record")
    if (
        len(failed_decisions) != 1
        or failed_decisions[0].outcome != "failed"
        or real_session_evidence(
            completion_error, kind="volicord", cycle=1, repository_revision=revision
        )["checks"]["explicit_user_decision_source"] != "failed"
    ):
        raise AssertionError("MCP completion error did not remain a failed operation")

    transport_error = real_session_fixture("volicord", 1, revision, evidence_directory)
    mutate_mcp_completion(
        transport_error,
        "work",
        "decision-call",
        lambda payload: payload.update({"result": {"Err": "sanitized transport failure"}}),
    )
    if real_session_evidence(
        transport_error, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["explicit_user_decision_source"] != "failed":
        raise AssertionError("MCP transport error qualified")

    unrelated_server = real_session_fixture("volicord", 1, revision, evidence_directory)
    mutate_mcp_completion(
        unrelated_server,
        "work",
        "decision-call",
        lambda payload: payload["invocation"].update({"server": "unrelated"}),
    )
    if real_session_evidence(
        unrelated_server, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["explicit_user_decision_source"] != "failed":
        raise AssertionError("unrelated MCP server completion qualified as Volicord")

    malformed_completion = real_session_fixture("volicord", 1, revision, evidence_directory)
    mutate_mcp_completion(
        malformed_completion,
        "work",
        "decision-call",
        lambda payload: payload.update({"result": {"Ok": {"isError": False}}}),
    )
    if real_session_evidence(
        malformed_completion, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["explicit_user_decision_source"] != "failed":
        raise AssertionError("malformed MCP completion qualified")

    substituted_task = "Replace an unrelated deployment workflow."
    mismatched_task = real_session_fixture("volicord", 1, revision, evidence_directory)
    replace_initial_task_text(mismatched_task, lambda _text: substituted_task)
    mismatched_task_result = real_session_evidence(
        mismatched_task, kind="volicord", cycle=1, repository_revision=revision
    )
    if mismatched_task_result["checks"]["plain_task_goal_linkage"] != "failed":
        raise AssertionError("captured task different from the descriptor plain task qualified")
    if mismatched_task_result["checks"]["explicit_user_decision_source"] != "passed":
        raise AssertionError("task mismatch invalidated unrelated current-host Decision provenance")

    scripted_prompts = (
        ("objective envelope", "PHASE8_OBJECTIVE: scripted"),
        (
            "hidden alternative",
            original_task + " Choose: " + fixture_decision_oracle()["viable_alternatives"][0],
        ),
        (
            "hidden recommendation",
            original_task + " Follow this recommendation: " + fixture_decision_oracle()["recommendation"],
        ),
        ("prescribed question", original_task + " Ask the user whether concise output is preferred."),
        (
            "operation order",
            original_task + " Run project_initialize, repository_analyze, context_record, then checkpoint_record.",
        ),
        (
            "reserved path",
            original_task + " Leave src/resume.rs unchanged for the next session.",
        ),
    )
    for label, prompt in scripted_prompts:
        scripted = real_session_fixture("volicord", 1, revision, evidence_directory)
        replace_work_task(scripted, prompt)
        scripted_result = real_session_evidence(
            scripted, kind="volicord", cycle=1, repository_revision=revision
        )
        if scripted_result["checks"]["naturalistic_prompt_integrity"] != "failed":
            raise AssertionError(f"{label} choreography qualified as naturalistic")

    explicit_recall = real_session_fixture("volicord", 1, revision, evidence_directory)
    replace_resume_task(explicit_recall, "Recall first, then continue the adapter improvement.")
    if real_session_evidence(
        explicit_recall, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["naturalistic_prompt_integrity"] != "failed":
        raise AssertionError("resume prompt explicitly instructing Recall qualified")

    wrong_checkpoint_call = real_session_fixture("volicord", 1, revision, evidence_directory)
    replace_checkpoint_call_goal(wrong_checkpoint_call, substituted_task)
    if real_session_evidence(
        wrong_checkpoint_call, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["plain_task_goal_linkage"] != "failed":
        raise AssertionError("Checkpoint call with a different goal qualified")

    wrong_canonical_checkpoint = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    mutate_bundle(
        wrong_canonical_checkpoint,
        lambda bundle: replace_canonical_goal(bundle, substituted_task, include_context=False),
    )
    if real_session_evidence(
        wrong_canonical_checkpoint, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["plain_task_goal_linkage"] != "failed":
        raise AssertionError("canonical Checkpoint with a different goal qualified")

    wrong_recall_goal = real_session_fixture("volicord", 1, revision, evidence_directory)
    replace_recall_goals(wrong_recall_goal, [substituted_task])
    if real_session_evidence(
        wrong_recall_goal, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["plain_task_goal_linkage"] != "failed":
        raise AssertionError("Recall result with a different goal qualified")

    wrong_goal_turn = real_session_fixture("volicord", 1, revision, evidence_directory)
    mutate_mcp_call(
        wrong_goal_turn,
        "work",
        "context_record",
        lambda arguments: arguments.update({"user_turn": "A different unobserved user turn"}),
    )
    if real_session_evidence(
        wrong_goal_turn, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["plain_task_goal_linkage"] != "failed":
        raise AssertionError("Goal linked to a different user turn qualified")

    non_user_goal = real_session_fixture("volicord", 1, revision, evidence_directory)

    def remove_goal_user_authority(bundle: dict[str, Any]) -> None:
        for table in bundle["payload"]["tables"]:
            if table["name"] != "sources":
                continue
            id_index = table["columns"].index("id")
            actor_index = table["columns"].index("actor_kind")
            for row in table["rows"]:
                if row[id_index].get("value") == "03" * 16:
                    row[actor_index] = {"type": "text", "value": "agent"}

    mutate_bundle(non_user_goal, remove_goal_user_authority)
    if real_session_evidence(
        non_user_goal, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["plain_task_goal_linkage"] != "failed":
        raise AssertionError("agent-provenance Goal Source qualified")

    marker_only = real_session_fixture("volicord", 1, revision, evidence_directory)
    marker_only["ordinary_work"] = {
        "status": "completed",
        "changed_paths": ["src/claimed.rs"],
    }
    marker_path, marker_events = capture_events(marker_only, "work")
    for value in marker_events:
        payload = value.get("payload", {})
        if payload.get("type") == "patch_apply_end":
            payload["changes"] = {
                "/phase8/repository/v11-ordinary-work.txt": {
                    "type": "update",
                    "unified_diff": "@@ -0,0 +1 @@\n+marker\n",
                    "move_path": None,
                }
            }
    store_capture(marker_only, "work", marker_path, marker_events)
    if real_session_evidence(
        marker_only, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["meaningful_ordinary_changes"] != "failed":
        raise AssertionError("capture containing only synthetic marker work qualified")

    missing_patch = real_session_fixture("volicord", 1, revision, evidence_directory)
    missing_patch["ordinary_work"] = {
        "status": "passed",
        "changed_paths": ["src/existing.rs", "tests/existing.rs"],
    }
    missing_patch_path, missing_patch_events = capture_events(missing_patch, "work")
    missing_patch_events = [
        value
        for value in missing_patch_events
        if value.get("payload", {}).get("type") != "patch_apply_end"
    ]
    store_capture(missing_patch, "work", missing_patch_path, missing_patch_events)
    if real_session_evidence(
        missing_patch, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("manifest patch claim hid absent rollout change evidence")

    mismatched_checkpoint_paths = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    mutate_custom_output(
        mismatched_checkpoint_paths,
        "work",
        "checkpoint-call",
        lambda output: output.update({"changed_paths": ["src/claimed.rs"]}),
    )
    if real_session_evidence(
        mismatched_checkpoint_paths,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("Checkpoint changed paths diverging from rollout work qualified")

    unobserved_checkpoint_decision = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    mutate_mcp_call(
        unobserved_checkpoint_decision,
        "work",
        "checkpoint_record",
        lambda arguments: arguments.update({"applied_decision_ids": ["ff" * 16]}),
    )
    mutate_custom_output(
        unobserved_checkpoint_decision,
        "work",
        "checkpoint-call",
        lambda output: output.update({"applied_decision_ids": ["ff" * 16]}),
    )
    if real_session_evidence(
        unobserved_checkpoint_decision,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("Checkpoint Decision absent from the current-host response qualified")

    def mark_verification_failed(fixture: dict[str, Any]) -> None:
        verification_source_id = "0a" * 16
        mutate_custom_output(
            fixture,
            "work",
            "verification-call",
            lambda output: output.update({"exit_code": 1}),
        )
        mutate_mcp_call(
            fixture,
            "work",
            "checkpoint_record",
            lambda arguments: arguments["verification"][0].update(
                {
                    "state": "failed",
                    "exit_code": 1,
                    "outcome": "focused tests failed",
                }
            ),
        )

        def mutate_verification_rows(bundle: dict[str, Any]) -> None:
            for table_value in bundle["payload"]["tables"]:
                columns = table_value["columns"]
                if table_value["name"] == "sources":
                    id_index = columns.index("id")
                    exit_index = columns.index("exit_code")
                    for row in table_value["rows"]:
                        if row[id_index].get("value") == verification_source_id:
                            row[exit_index] = {"type": "integer", "value": 1}
                elif table_value["name"] == "checkpoint_verifications":
                    state_index = columns.index("verification_state")
                    outcome_index = columns.index("outcome")
                    table_value["rows"][0][state_index] = {
                        "type": "text",
                        "value": "failed",
                    }
                    table_value["rows"][0][outcome_index] = {
                        "type": "text",
                        "value": "focused tests failed",
                    }

        mutate_bundle(fixture, mutate_verification_rows)

    matching_failed_verification = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    mark_verification_failed(matching_failed_verification)
    if real_session_evidence(
        matching_failed_verification,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["source_grounded_checkpoint"] != "passed":
        raise AssertionError("complete-result non-zero exit did not qualify matching failed verification")

    correlated_passed_verification = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    replace_command_observation(
        correlated_passed_verification,
        "work",
        "verification-call",
        correlated_split_wrapper,
        "Ran focused tests\nOK\n",
        '{"exit_code":0}',
    )
    if real_session_evidence(
        correlated_passed_verification,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["source_grounded_checkpoint"] != "passed":
        raise AssertionError("same-result split forwarding did not qualify matching passed verification")
    correlated_passed_capture = load_codex_capture(
        evidence_directory
        / correlated_passed_verification["evidence"]["captures"]["work"]["file"]
    )
    correlated_passed_commands = [
        command
        for command in correlated_passed_capture.commands
        if isinstance(command.parsed_command, dict)
        and command.parsed_command.get("cmd")
        == "python3 -m unittest tests.test_existing"
    ]
    if (
        len(correlated_passed_commands) != 1
        or correlated_passed_commands[0].exit_code != 0
        or correlated_passed_commands[0].termination != "exited"
        or correlated_passed_commands[0].output != "Ran focused tests\nOK\n"
    ):
        raise AssertionError("same-result split forwarding lost its correlated command observation")

    correlated_failed_verification = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    mark_verification_failed(correlated_failed_verification)
    replace_command_observation(
        correlated_failed_verification,
        "work",
        "verification-call",
        correlated_split_wrapper,
        "Ran focused tests\nFAILED\n",
        '{"exit_code":1}',
    )
    if real_session_evidence(
        correlated_failed_verification,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["source_grounded_checkpoint"] != "passed":
        raise AssertionError("same-result split forwarding did not qualify matching failed verification")

    for label, wrapper in unsupported_split_wrappers.items():
        uncorrelated = real_session_fixture("volicord", 1, revision, evidence_directory)
        replace_command_observation(
            uncorrelated,
            "work",
            "verification-call",
            wrapper,
            "Ran focused tests\nOK\n",
            '{"exit_code":0}',
        )
        if real_session_evidence(
            uncorrelated, kind="volicord", cycle=1, repository_revision=revision
        )["checks"]["source_grounded_checkpoint"] != "failed":
            raise AssertionError(f"{label} qualified passed verification")

    for label, status_text in {
        "non-numeric correlated status": '{"exit_code":"0"}',
        "correlated status with an extra field": '{"exit_code":0,"session_id":null}',
        "correlated status literal": "exit_code=0",
    }.items():
        malformed_correlated = real_session_fixture(
            "volicord", 1, revision, evidence_directory
        )
        replace_command_observation(
            malformed_correlated,
            "work",
            "verification-call",
            correlated_split_wrapper,
            "Ran focused tests\nOK\n",
            status_text,
        )
        if real_session_evidence(
            malformed_correlated,
            kind="volicord",
            cycle=1,
            repository_revision=revision,
        )["checks"]["source_grounded_checkpoint"] != "failed":
            raise AssertionError(f"{label} qualified passed verification")

    correlated_inspection = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    inspection_arguments = (
        '{"cmd":"sed -n 1,120p src/resume.rs",'
        '"workdir":"/phase8/repository","yield_time_ms":10000}'
    )
    inspection_wrapper = (
        f"const result=await tools.exec_command({inspection_arguments});\n"
        "text(result.output); text(JSON.stringify({exit_code:result.exit_code}));\n"
    )
    replace_command_observation(
        correlated_inspection,
        "resume",
        "inspect-call",
        inspection_wrapper,
        "current resume source\n",
        '{"exit_code":0}',
        remove_mcp_completion=True,
    )
    correlated_inspection_result = real_session_evidence(
        correlated_inspection,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )
    if (
        correlated_inspection_result["checks"][
            "recall_precedes_inspection_and_continuation"
        ]
        != "passed"
    ):
        raise AssertionError("same-result split command could not establish post-Recall inspection")

    failed_verification = real_session_fixture("volicord", 1, revision, evidence_directory)
    mutate_custom_output(
        failed_verification,
        "work",
        "verification-call",
        lambda output: output.update({"exit_code": 1}),
    )
    if real_session_evidence(
        failed_verification, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("passed Checkpoint verification hid a failed rollout command")

    absent_verification = real_session_fixture("volicord", 1, revision, evidence_directory)
    absent_verification["verification"] = {"status": "passed"}
    remove_custom_output(absent_verification, "work", "verification-call")
    if real_session_evidence(
        absent_verification, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("shell claim without a completed rollout execution qualified")

    stdout_only_verification = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    replace_custom_call_input(
        stdout_only_verification,
        "work",
        "verification-call",
        'const r=await tools.exec_command({"cmd":"python3 -m unittest tests.test_existing",'
        '"workdir":"/phase8/repository","yield_time_ms":30000});\ntext(r.output);\n',
    )
    if real_session_evidence(
        stdout_only_verification,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("printed stdout without command outcome qualified passed verification")
    stdout_only_capture = load_codex_capture(
        evidence_directory
        / stdout_only_verification["evidence"]["captures"]["work"]["file"]
    )
    stdout_only_commands = [
        command
        for command in stdout_only_capture.commands
        if isinstance(command.parsed_command, dict)
        and command.parsed_command.get("cmd")
        == "python3 -m unittest tests.test_existing"
    ]
    if (
        len(stdout_only_commands) != 1
        or stdout_only_commands[0].exit_code is not None
        or stdout_only_commands[0].termination is not None
    ):
        raise AssertionError("output-only command was not preserved as outcome-unknown")

    stdout_only_failed_verification = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    mark_verification_failed(stdout_only_failed_verification)
    replace_custom_call_input(
        stdout_only_failed_verification,
        "work",
        "verification-call",
        'const r=await tools.exec_command({"cmd":"python3 -m unittest tests.test_existing",'
        '"workdir":"/phase8/repository","yield_time_ms":30000});\ntext(r.output);\n',
    )
    if real_session_evidence(
        stdout_only_failed_verification,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("printed stdout without command outcome qualified failed verification")

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
            and "decision-user-turn" in str(value.get("payload", {}).get("client_id", ""))
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
        if "tools.mcp__volicord__recall(" in str(value.get("payload", {}).get("input", ""))
        or (
            value.get("payload", {}).get("type") == "custom_tool_call_output"
            and "recall-call" in str(value.get("payload", {}).get("call_id"))
        )
        or (
            value.get("payload", {}).get("type") == "mcp_tool_call_end"
            and value.get("payload", {}).get("invocation", {}).get("tool") == "recall"
        )
    ]
    inspection_indexes = [
        index
        for index, value in enumerate(order_events)
        if "tools.mcp__volicord__repository_understanding(" in str(value.get("payload", {}).get("input", ""))
        or (
            value.get("payload", {}).get("type") == "custom_tool_call_output"
            and "inspect-call" in str(value.get("payload", {}).get("call_id"))
        )
        or (
            value.get("payload", {}).get("type") == "mcp_tool_call_end"
            and value.get("payload", {}).get("invocation", {}).get("tool")
            == "repository_understanding"
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

    absent_recall = real_session_fixture("volicord", 1, revision, evidence_directory)
    remove_mcp_completion(absent_recall, "resume", "recall-call")
    if real_session_evidence(
        absent_recall, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["recall_precedes_inspection_and_continuation"] != "failed":
        raise AssertionError("fresh session without observed automatic Recall qualified")

    absent_resolution = real_session_fixture("volicord", 1, revision, evidence_directory)
    remove_mcp_completion(absent_resolution, "resume", "resolve-call")
    if real_session_evidence(
        absent_resolution, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["repository_bound_project_resolution"] != "failed":
        raise AssertionError("fresh resume without repository-bound Project resolution qualified")

    replacement_project = real_session_fixture("volicord", 1, revision, evidence_directory)
    replacement_path, replacement_events = capture_events(replacement_project, "resume")
    replacement_completion = {
        "timestamp": "2026-08-15T00:00:00Z",
        "type": "event_msg",
        "payload": {
            "type": "mcp_tool_call_end",
            "call_id": "exec-replacement-project-initialize",
            "invocation": {
                "server": "volicord",
                "tool": "project_initialize",
                "arguments": {
                    "display_name": "Replacement Project",
                    "repository": "/phase8/repository",
                },
            },
            "duration": {"secs": 0, "nanos": 1},
            "result": {
                "Ok": {
                    "content": [],
                    "structuredContent": {"project_id": "ff" * 16},
                    "isError": False,
                }
            },
        },
    }
    replacement_events.insert(-1, replacement_completion)
    store_capture(replacement_project, "resume", replacement_path, replacement_events)
    replacement_capture = load_codex_capture(replacement_path)
    if (
        len(replacement_capture.successful_calls("project_initialize")) != 1
        or real_session_evidence(
            replacement_project, kind="volicord", cycle=1, repository_revision=revision
        )["checks"]["repository_bound_project_resolution"]
        != "failed"
    ):
        raise AssertionError("fresh resume replacement Project initialization qualified")

    missing_candidate_promotion = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    remove_mcp_completion(missing_candidate_promotion, "work", "candidate-promote-call")
    if real_session_evidence(
        missing_candidate_promotion,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["researched_material_question"] != "failed":
        raise AssertionError("Question without observed Candidate promotion qualified")

    mismatched_recall = real_session_fixture("volicord", 1, revision, evidence_directory)
    mismatched_recall["resume_invocation"] = {
        "recall": {"checkpoint_id": "claimed", "decision_ids": ["claimed"], "context_ids": ["claimed"]}
    }
    def mismatch_recall_output(output: dict[str, Any]) -> None:
        output["decisions"][0]["identity"] = "ff" * 16
        output["goals"] = ["Different recalled goal"]
        output["next_step"] = "Different recalled next step"
        output["checkpoint"]["identity"] = "ee" * 16

    mutate_custom_output(
        mismatched_recall,
        "resume",
        "recall-call",
        mismatch_recall_output,
    )
    if real_session_evidence(
        mismatched_recall, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["recall_matches_checkpoint_decision_and_context"] != "failed":
        raise AssertionError("manifest Recall IDs hid mismatched Recall result content")

    for label, mutation in (
        (
            "Checkpoint",
            lambda output: output["checkpoint"].update({"identity": "ee" * 16}),
        ),
        (
            "Decision",
            lambda output: output["decisions"][0].update({"identity": "ff" * 16}),
        ),
    ):
        mismatch = real_session_fixture("volicord", 1, revision, evidence_directory)
        mutate_custom_output(mismatch, "resume", "recall-call", mutation)
        if real_session_evidence(
            mismatch, kind="volicord", cycle=1, repository_revision=revision
        )["checks"]["recall_matches_checkpoint_decision_and_context"] != "failed":
            raise AssertionError(f"Recall {label} mismatch qualified")

    continuation_before_recall = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    before_path, before_events = capture_events(continuation_before_recall, "resume")
    continuation_indexes = [
        index
        for index, value in enumerate(before_events)
        if "resume-patch-call" in str(value.get("payload", {}).get("call_id"))
        or value.get("payload", {}).get("type") == "patch_apply_end"
    ]
    continuation_values = [before_events[index] for index in continuation_indexes]
    before_events = [
        value for index, value in enumerate(before_events) if index not in continuation_indexes
    ]
    before_events = before_events[:3] + continuation_values + before_events[3:]
    store_capture(continuation_before_recall, "resume", before_path, before_events)
    before_result = real_session_evidence(
        continuation_before_recall,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )
    if before_result["checks"]["meaningful_recalled_continuation"] != "failed":
        raise AssertionError("continuation before Recall qualified as resumed work")

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
    no_continuation_result = real_session_evidence(
        no_continuation, kind="volicord", cycle=1, repository_revision=revision
    )
    if (
        not no_continuation_result["continuation_basis"]["resume_numeric_exit_validation"]
        or no_continuation_result["continuation_basis"]["observed_change_relevant_to_checkpoint_next_step"]
    ):
        raise AssertionError("validation and source-change evidence were not kept separate")

    irrelevant_continuation = real_session_fixture("volicord", 1, revision, evidence_directory)
    replace_checkpoint_next_step(
        irrelevant_continuation, "Update tests/reserved.rs for a different component"
    )
    if real_session_evidence(
        irrelevant_continuation, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["meaningful_recalled_continuation"] != "failed":
        raise AssertionError("resume change unrelated to the recalled Checkpoint qualified")

    generated_only = real_session_fixture("volicord", 1, revision, evidence_directory)
    generated_path, generated_events = capture_events(generated_only, "resume")
    for value in generated_events:
        payload = value.get("payload", {})
        if payload.get("type") == "patch_apply_end":
            payload["changes"] = {
                "/phase8/repository/build/generated.rs": {
                    "type": "update",
                    "unified_diff": "@@ -0,0 +1 @@\n+generated\n",
                    "move_path": None,
                }
            }
    store_capture(generated_only, "resume", generated_path, generated_events)
    if real_session_evidence(
        generated_only, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["meaningful_recalled_continuation"] != "failed":
        raise AssertionError("generated/build-only resume change qualified")

    no_resume_validation = real_session_fixture("volicord", 1, revision, evidence_directory)
    validation_path, validation_events = capture_events(no_resume_validation, "resume")
    validation_events = [
        value
        for value in validation_events
        if "resume-verification-call" not in str(value.get("payload", {}).get("call_id"))
    ]
    store_capture(no_resume_validation, "resume", validation_path, validation_events)
    if real_session_evidence(
        no_resume_validation, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["meaningful_recalled_continuation"] != "failed":
        raise AssertionError("resume change without separate numeric-exit validation qualified")

    insufficient = real_session_fixture("volicord", 1, revision, evidence_directory)
    insufficient_path = (
        evidence_directory / insufficient["evidence"]["captures"]["work"]["file"]
    )
    insufficient_path.write_text('{"event":"work"}\n', encoding="utf-8")
    insufficient["evidence"]["captures"]["work"]["sha256"] = sha256(insufficient_path)
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
    missing_viewport = parse_accessibility_html(
        "<!doctype html><html lang=\"en\"><body><h1>Project</h1></body></html>",
        expected_language="en",
    )
    missing_viewport["status"] = status_from_steps(missing_viewport["checks"])
    viewport_qualified = qualify_accessibility(
        missing_viewport,
        {
            "narrow_and_zoomed_presentation": {
                "status": "passed",
                "basis": "bounded observation",
            },
        },
        set(definition["permitted_accessibility_observations"]),
    )
    if viewport_qualified["checks"]["narrow_and_zoomed_presentation"] != "failed":
        raise AssertionError("manual observation hid deterministic viewport failure")

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
        "current_mcp_completion_envelope": "passed",
        "json_stringify_wrapper_completion_authority": "passed",
        "actual_style_recall_completion_before_inspection": "passed",
        "bounded_wrapper_correlation_without_execution": "passed",
        "missing_mcp_completion_rejected": "passed",
        "wrapper_completion_mismatch_rejected": "passed",
        "mcp_completion_error_rejected": "passed",
        "unrelated_and_malformed_mcp_completion_rejected": "passed",
        "wrapper_completion_deduplicated": "passed",
        "naturalistic_plain_task_descriptor_contract": "passed",
        "work_task_materiality_basis_required_in_work_task": "passed",
        "plain_task_and_hidden_oracle_sanitization": "passed",
        "descriptor_and_captured_task_mismatch_rejected": "passed",
        "scripted_objective_marker_rejected": "passed",
        "hidden_alternatives_and_recommendation_rejected": "passed",
        "prescribed_question_and_operation_order_rejected": "passed",
        "explicit_resume_recall_instruction_rejected": "passed",
        "reserved_later_session_path_rejected": "passed",
        "plain_task_checkpoint_call_mismatch_rejected": "passed",
        "plain_task_canonical_checkpoint_mismatch_rejected": "passed",
        "plain_task_recall_mismatch_rejected": "passed",
        "wrong_and_non_user_goal_source_rejected": "passed",
        "v11_only_rejected": "passed",
        "synthetic_marker_work_rejected": "passed",
        "missing_and_mismatched_patch_evidence_rejected": "passed",
        "unobserved_checkpoint_decision_rejected": "passed",
        "failed_absent_and_stdout_only_verification_rejected": "passed",
        "complete_result_passed_and_failed_verification": "passed",
        "correlated_split_passed_and_failed_verification": "passed",
        "uncorrelated_and_synthesized_split_status_rejected": "passed",
        "output_only_command_outcome_unknown": "passed",
        "same_session_rejected": "passed",
        "resume_repository_resolution_required": "passed",
        "resume_replacement_project_rejected": "passed",
        "resume_without_recall_word_qualified": "passed",
        "absent_recall_rejected": "passed",
        "recall_order_rejected": "passed",
        "correlated_split_repository_inspection_ordering": "passed",
        "mismatched_recall_state_rejected": "passed",
        "continuation_before_recall_rejected": "passed",
        "missing_continuation_rejected": "passed",
        "irrelevant_and_generated_resume_change_rejected": "passed",
        "resume_change_and_validation_separated": "passed",
        "checkpoint_owned_continuation_relevance": "passed",
        "user_decision_provenance_rejected": "passed",
        "missing_user_decision_rejected": "passed",
        "valid_hash_insufficient_semantics_rejected": "passed",
        "candidate_question_lifecycle_provenance_required": "passed",
        "terminal_work_blocker_early_stop": "passed",
        "positive_work_blocker_attempt_rejected": "passed",
        "early_stop_completion_claims_rejected": "passed",
        "twelve_session_replacement_contract": "passed",
        "arbitrary_event_label_rejected": "passed",
        "accessibility_viewer_shaped_names": "passed",
        "accessibility_hidden_controls_excluded": "passed",
        "accessibility_button_text_and_aria_names": "passed",
        "accessibility_unlabeled_controls_rejected": "passed",
        "accessibility_heading_order_rejected": "passed",
        "accessibility_machine_failure_authority": "passed",
        "viewer_environment_blocking": "passed",
        "manual_override_boundary": "passed",
        "linux_process_tree_peak_rss": process_peak["status"],
        "linux_process_tree_environment_classification": "passed",
        "resource_measurement_unavailable_blocks_qualification": "passed",
        "resource_process_truth_preserved": "passed",
        "repeated_resource_no_replace_rounds": "passed",
        "repeated_resource_preexisting_destination_rejected": "passed",
        "repeated_resource_failed_export_not_owned": "passed",
        "repeated_resource_stability": "passed",
        "repeated_resource_growth_rejected": "passed",
        "repeated_resource_operation_failure_rejected": "passed",
        "repeated_resource_incomplete_evidence_unsupported": "passed",
        "unobserved_resource_state_preserved": "passed",
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
    descriptors = subparsers.add_parser("check-descriptors")
    descriptors.add_argument("descriptors", nargs="+")
    blocker = subparsers.add_parser("qualify-work-blocker")
    blocker.add_argument("--candidate-head", required=True)
    blocker.add_argument("--descriptor", required=True)
    blocker.add_argument("--work-capture", required=True)
    blocker.add_argument("--output", required=True)
    run = subparsers.add_parser("run")
    run.add_argument("--candidate-head", required=True)
    run.add_argument("--repositories", required=True)
    run.add_argument("--output-dir", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "self-test":
        return self_test()
    if args.command == "check-descriptors":
        return check_descriptors(args.descriptors)
    if args.command == "qualify-work-blocker":
        return qualify_work_blocker(args)
    return run_evaluation(args)


if __name__ == "__main__":
    raise SystemExit(main())
