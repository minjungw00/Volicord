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
sys.path.insert(0, str(ROOT / "rebuild/validation/shared"))
from architecture_owners import ACTIVE_ARCHITECTURE_OWNER_PATHS  # noqa: E402

DEFINITION = HERE / "evaluation.json"
CURRENT_MCP_FIXTURE = HERE / "fixtures/current-codex-mcp-completion.jsonl"
CURRENT_EXECUTION_FIXTURE = HERE / "fixtures/current-codex-execution-evidence.jsonl"
V11_HARNESS = ROOT / "rebuild/validation/end-to-end/multi-repository/harness.py"
STRICT_FAKE_CLI = ROOT / "rebuild/validation/shared/strict_fake_volicord.py"
DECISION_REGISTER = ROOT / "rebuild/docs/design/open-decisions.md"
ALLOWED_STATUS = {
    "passed", "failed", "partial", "unsupported", "skipped", "environment_blocked"
}
CLASSES = ("volicord", "small-python", "polyglot-medium")
BEHAVIOR_CLASSES = (
    "explicit_user_owned_decision",
    "hidden_user_owned_decision",
    "research_or_no_question",
    "delegated_implementation_choice",
    "exploratory_uncertainty",
    "learning_deliberation",
    "learning_routine_control",
)
CYCLE_COUNT_BY_REPOSITORY = {
    "volicord": 3,
    "small-python": 3,
    "polyglot-medium": 2,
}
QUALIFICATION_CYCLE_COUNT = sum(CYCLE_COUNT_BY_REPOSITORY.values())
QUALIFICATION_SESSION_COUNT = QUALIFICATION_CYCLE_COUNT * 2
_PRIVATE_QUALIFICATION_BEHAVIOR_COUNTS = Counter({
    "explicit_user_owned_decision": 1,
    "hidden_user_owned_decision": 2,
    "research_or_no_question": 1,
    "delegated_implementation_choice": 1,
    "exploratory_uncertainty": 1,
    "learning_deliberation": 1,
    "learning_routine_control": 1,
})
USER_OWNED_BEHAVIOR_CLASSES = {
    "explicit_user_owned_decision",
    "hidden_user_owned_decision",
}


def cycle_numbers(repository_class: str) -> range:
    return range(1, CYCLE_COUNT_BY_REPOSITORY[repository_class] + 1)
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
RESOURCE_HEALTH_METRICS = (
    "process_count",
    "open_file_descriptor_count",
    "runtime_file_count",
    "stale_temporary_file_count",
    "operation_latency_ms",
)
REAL_RESOURCE_OBSERVATION_MODE = (
    "linux_procfs_filesystem_storage_and_operation_latency"
)
SELF_TEST_RESOURCE_OBSERVATION_MODE = "injected_self_test_observer"
REAL_SESSION_CHECKS = (
    "repository_scoped_activation",
    "naturalistic_prompt_integrity",
    "plain_task_goal_linkage",
    "grounded_pre_work_repository_baseline",
    "engineering_choice_discovery",
    "pre_write_materiality_work_authority",
    "learning_participation",
    "learning_deliberation_order",
    "learning_not_canonical_decision",
    "learning_interruption_precision",
    "behavior_classification",
    "appropriate_inquiry_outcome",
    "hidden_material_discovery_order",
    "no_silent_user_owned_choice",
    "meaningful_ordinary_changes",
    "source_grounded_checkpoint",
    "decision_provenance_when_required",
    "distinct_work_and_resume_invocations",
    "fresh_resume_without_prior_context",
    "repository_bound_project_resolution",
    "recall_precedes_inspection_and_continuation",
    "resume_pre_work_repository_baseline",
    "resume_materiality_work_authority",
    "recall_matches_checkpoint_decision_and_context",
    "learning_recall_continuity",
    "resolved_material_question_not_reasked",
    "meaningful_recalled_continuation",
    "canonical_bundle_and_provenance",
    "generated_document_outputs",
    "static_viewer_snapshot",
    "bounded_runtime_and_activation_evidence",
)
MAX_USER_TASK_BYTES = 8192
MAX_REVIEW_TEXT_BYTES = 8192
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
BEHAVIOR_REVIEW_PROVENANCE_SCOPES = {
    "volicord_active_owner",
    "target_repository",
}


def is_user_owned_behavior(value: Any) -> bool:
    return value in USER_OWNED_BEHAVIOR_CLASSES


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="microseconds")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def safe_relative_evidence_path(value: Any) -> str | None:
    if not nonempty_string(value) or len(value.encode("utf-8")) > 512 or "\\" in value:
        return None
    path = Path(value)
    if path.is_absolute() or value != path.as_posix() or any(part in {"", ".", ".."} for part in path.parts):
        return None
    return value


def git_blob_bytes(repository: Path, revision: str, relative_path: str) -> bytes | None:
    completed = subprocess.run(
        ["git", "-C", str(repository), "show", f"{revision}:{relative_path}"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return completed.stdout if completed.returncode == 0 else None


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


def directory_file_count(path: Path) -> int:
    total = 0
    if not path.exists():
        return total
    for _root, directories, files in os.walk(path):
        directories[:] = [name for name in directories if name != ".git"]
        total += len(files)
    return total


def stale_temporary_file_count(path: Path) -> int:
    suffixes = {".tmp", ".temp", ".partial", ".part"}
    return sum(
        item.suffix.casefold() in suffixes
        for item in path.rglob("*")
        if item.is_file()
    ) if path.exists() else 0


def current_process_resource_counts() -> tuple[int, int]:
    process_root = Path("/proc") / str(os.getpid())
    children = (process_root / "task" / str(os.getpid()) / "children").read_text(
        encoding="ascii"
    ).split()
    return 1 + len(children), len(list((process_root / "fd").iterdir()))


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
        resident_pages = int(statm_fields[1])
    except ValueError:
        return "linux_procfs_statm_malformed"
    if resident_pages <= 0:
        return "linux_procfs_resident_pages_unavailable"
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


def repeated_resource_diagnostic(
    kind: str,
    *,
    round_number: int | None = None,
    operation: str | None = None,
    metric: str | None = None,
    observed: Any = None,
    deltas: list[int] | None = None,
) -> dict[str, Any]:
    diagnostic: dict[str, Any] = {"kind": kind}
    if round_number is not None:
        diagnostic["round"] = round_number
    if operation is not None:
        diagnostic["operation"] = operation
    if metric is not None:
        diagnostic["metric"] = metric
    if observed is not None:
        diagnostic["observed"] = observed
    if deltas is not None:
        diagnostic["deltas"] = deltas
    return diagnostic


def repeated_resource_conclusion(rounds: list[dict[str, Any]]) -> dict[str, Any]:
    if len(rounds) < 3:
        return {
            "status": "unsupported",
            "conclusion": "insufficient_repeated_observations",
            "unexplained_cumulative_growth_observed": None,
            "metric_deltas": {},
            "diagnostic": repeated_resource_diagnostic(
                "insufficient_repeated_observations",
                observed={"round_count": len(rounds), "required_minimum": 3},
            ),
        }
    for index, round_value in enumerate(rounds, start=1):
        operations = round_value.get("operations", {})
        if set(operations) != set(RESOURCE_OPERATIONS):
            return {
                "status": "unsupported",
                "conclusion": "repeated_operation_evidence_incomplete",
                "unexplained_cumulative_growth_observed": None,
                "metric_deltas": {},
                "diagnostic": repeated_resource_diagnostic(
                    "repeated_operation_evidence_incomplete",
                    round_number=round_value.get("round", index),
                    observed={
                        "missing": sorted(set(RESOURCE_OPERATIONS) - set(operations)),
                        "unexpected": sorted(set(operations) - set(RESOURCE_OPERATIONS)),
                    },
                ),
            }
        for operation_name, operation in operations.items():
            if operation.get("exit_code") != 0 or operation.get("termination") is not None:
                return {
                    "status": "failed",
                    "conclusion": "repeated_operation_failed",
                    "unexplained_cumulative_growth_observed": None,
                    "metric_deltas": {},
                    "diagnostic": repeated_resource_diagnostic(
                        "repeated_operation_failed",
                        round_number=round_value.get("round", index),
                        operation=operation_name,
                        observed={
                            "exit_code": operation.get("exit_code"),
                            "termination": operation.get("termination"),
                            "spawn_failed": operation.get("spawn_failed", False),
                        },
                    ),
                }
        for name in (*RESOURCE_STORAGE_METRICS, *RESOURCE_HEALTH_METRICS):
            if not isinstance(round_value.get(name), int):
                return {
                    "status": "unsupported",
                    "conclusion": "resource_measurement_unavailable",
                    "unexplained_cumulative_growth_observed": None,
                    "metric_deltas": {},
                    "diagnostic": repeated_resource_diagnostic(
                        "resource_measurement_unavailable",
                        round_number=round_value.get("round", index),
                        metric=name,
                        observed={"value_type": type(round_value.get(name)).__name__},
                    ),
                }
    deltas = {
        name: [
            rounds[index][name] - rounds[index - 1][name]
            for index in range(1, len(rounds))
        ]
        for name in (*RESOURCE_STORAGE_METRICS, *RESOURCE_HEALTH_METRICS)
    }
    post_warmup = {name: values[1:] for name, values in deltas.items()}
    cumulative = [
        name for name, values in post_warmup.items()
        if values and all(value > 0 for value in values)
    ]
    stale_temporary_files = any(
        round_value["stale_temporary_file_count"] > 0 for round_value in rounds
    )
    leaked_processes = any(round_value["process_count"] > 1 for round_value in rounds)
    failures = cumulative or stale_temporary_files or leaked_processes
    stable = all(all(value == 0 for value in values) for values in post_warmup.values())
    diagnostic = None
    if stale_temporary_files:
        failed_index, failed_round = next(
            (index, value)
            for index, value in enumerate(rounds, start=1)
            if value["stale_temporary_file_count"] > 0
        )
        diagnostic = repeated_resource_diagnostic(
            "stale_temporary_files_observed",
            round_number=failed_round.get("round", failed_index),
            metric="stale_temporary_file_count",
            observed=failed_round["stale_temporary_file_count"],
        )
    elif leaked_processes:
        failed_index, failed_round = next(
            (index, value)
            for index, value in enumerate(rounds, start=1)
            if value["process_count"] > 1
        )
        diagnostic = repeated_resource_diagnostic(
            "descendant_process_leak_observed",
            round_number=failed_round.get("round", failed_index),
            metric="process_count",
            observed=failed_round["process_count"],
        )
    elif cumulative:
        diagnostic = repeated_resource_diagnostic(
            "unexplained_cumulative_growth_or_latency_degradation_observed",
            metric=cumulative[0],
            deltas=deltas[cumulative[0]],
            observed={"all_cumulative_metrics": cumulative},
        )
    return {
        "status": "failed" if failures else "passed",
        "conclusion": (
            "stale_temporary_files_observed"
            if stale_temporary_files else
            "descendant_process_leak_observed"
            if leaked_processes else
            "unexplained_cumulative_growth_or_latency_degradation_observed"
            if cumulative else
            "stable_after_warmup"
            if stable else
            "bounded_variation_without_cumulative_growth"
        ),
        "unexplained_cumulative_growth_observed": bool(cumulative),
        "stale_temporary_files_observed": stale_temporary_files,
        "descendant_process_leak_observed": leaked_processes,
        "cumulative_growth_metrics": cumulative,
        "metric_deltas": deltas,
        "diagnostic": diagnostic,
    }


def observe_repeated_resource_round(
    runtime: Path,
    document_output_bytes: int | None,
    operations: tuple[dict[str, Any], ...],
) -> dict[str, Any]:
    """Collect the real Phase 8 Linux process, filesystem, storage, and latency evidence."""

    process_count, open_file_descriptor_count = current_process_resource_counts()
    return {
        "runtime_home_bytes": directory_bytes(runtime),
        "derived_state_bytes": directory_bytes(runtime / "analysis"),
        "document_output_bytes": document_output_bytes,
        "process_count": process_count,
        "open_file_descriptor_count": open_file_descriptor_count,
        "runtime_file_count": directory_file_count(runtime),
        "stale_temporary_file_count": stale_temporary_file_count(runtime),
        "operation_latency_ms": int(sum(
            value.get("duration_ms", 0)
            for value in operations
            if isinstance(value.get("duration_ms"), (int, float))
        )),
    }


def repeated_resource_rehearsal(
    kind: str,
    cycle_root: Path,
    recorder: Any,
    base_env: dict[str, str],
    project_id: str | None,
    repetition_count: int,
    *,
    resource_observer: Callable[
        [Path, int | None, tuple[dict[str, Any], ...]], dict[str, Any]
    ] | None = None,
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
    repository = target_root / "repository"
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

    observation_mode = (
        REAL_RESOURCE_OBSERVATION_MODE
        if resource_observer is None
        else SELF_TEST_RESOURCE_OBSERVATION_MODE
    )
    observer = resource_observer or observe_repeated_resource_round

    def failed_rehearsal(
        conclusion: str,
        *,
        round_number: int | None = None,
        operation: str | None = None,
        metric: str | None = None,
        observed: Any = None,
    ) -> dict[str, Any]:
        return {
            "status": "failed",
            "conclusion": conclusion,
            "unexplained_cumulative_growth_observed": None,
            "repetition_count": repetition_count,
            "operations_per_round": list(RESOURCE_OPERATIONS),
            "fixed_input_and_destination": True,
            "universal_product_ceiling_applied": False,
            "observation_mode": observation_mode,
            "diagnostic": repeated_resource_diagnostic(
                conclusion,
                round_number=round_number,
                operation=operation,
                metric=metric,
                observed=observed,
            ),
            "rounds": rounds,
        }

    if destination_present():
        return failed_rehearsal(
            "rehearsal_destination_preexisting",
            round_number=0,
            operation="document_projection",
            observed={"cleanup_condition": "destination_present_before_rehearsal"},
        )

    for repetition in range(1, repetition_count + 1):
        if destination_present():
            return failed_rehearsal(
                "rehearsal_destination_ownership_ambiguous",
                round_number=repetition,
                operation="document_projection",
                observed={"cleanup_condition": "destination_present_before_round"},
            )
        analysis = recorder.run(
            f"resource-{repetition}-analyze",
            [str(cli), "--json", "--repository", str(repository), "analyze"],
            environment,
        )
        document = recorder.run(
            f"resource-{repetition}-document",
            [
                str(cli), "--json", "--repository", str(repository),
                "document", "export", "project-architecture-guide",
                "--format", "html", "--output", str(repeated_document),
                "--language", "en",
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
            [str(cli), "--repository", str(repository), "doctor", "repair"],
            environment,
        )
        try:
            observation = observer(
                runtime,
                document_output_bytes,
                (analysis, document, repair),
            )
        except OSError as error:
            return failed_rehearsal(
                "linux_process_or_file_descriptor_observation_unavailable",
                round_number=repetition,
                metric="process_count_or_open_file_descriptor_count",
                observed={"error_class": type(error).__name__},
            )
        rounds.append({
            "round": repetition,
            "operations": {
                "repository_analysis": bounded_process_result(analysis),
                "document_projection": bounded_process_result(document),
                "derived_analysis_repair": bounded_process_result(repair),
            },
            **observation,
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
            return failed_rehearsal(
                ownership_failure,
                round_number=repetition,
                operation="document_projection",
                observed={"cleanup_condition": ownership_failure},
            )
    conclusion = repeated_resource_conclusion(rounds)
    return {
        **conclusion,
        "repetition_count": repetition_count,
        "operations_per_round": list(RESOURCE_OPERATIONS),
        "fixed_input_and_destination": True,
        "universal_product_ceiling_applied": False,
        "observation_mode": observation_mode,
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
    if value.get("cycles_by_repository") != CYCLE_COUNT_BY_REPOSITORY:
        raise ValueError("Phase 8 requires the maintained non-uniform repository cycle allocation")
    if value.get("qualification_cycle_count") != QUALIFICATION_CYCLE_COUNT:
        raise ValueError("Phase 8 requires exactly eight qualification cycles")
    profile_contract = value.get("qualification_profile_contract", {})
    if profile_contract != {
        "visibility": "evaluator_steward_private_until_all_provisionals_recorded",
        "reveal_requires_provisional_count": QUALIFICATION_CYCLE_COUNT,
        "validation_phase": "post_reveal_before_sealing",
        "reviewer_safe_profile_disclosure": False,
    }:
        raise ValueError("the Phase 8 private qualification-profile boundary changed")
    if tuple(value.get("behavior_classes", [])) != BEHAVIOR_CLASSES:
        raise ValueError("the Phase 8 behavior-class matrix changed")
    if tuple(value.get("repository_classes", {})) != CLASSES:
        raise ValueError("the Phase 8 repository class order changed")
    small_rules = value["repository_classes"]["small-python"]
    if (
        small_rules.get("minimum_files", 0) < 8
        or small_rules.get("maximum_files", 0) > 250
        or small_rules.get("official_structural_language_count") != 1
        or small_rules.get("application_structure_required") is not True
        or small_rules.get("production_source_files_required", 0) < 3
        or small_rules.get("test_files_required", 0) < 2
        or small_rules.get("configuration_required") is not True
        or small_rules.get("behavioral_boundary_required") is not True
        or small_rules.get("trivial_arithmetic_or_example_disallowed") is not True
        or small_rules.get("multi_file_or_user_visible_work_required") is not True
    ):
        raise ValueError("the realistic small-Python repository contract changed")
    polyglot_rules = value["repository_classes"]["polyglot-medium"]
    if (
        polyglot_rules.get("minimum_files", 0) < 100
        or polyglot_rules.get("minimum_official_structural_languages", 0) < 3
        or polyglot_rules.get("documentation_required") is not True
        or polyglot_rules.get("component_boundary_required") is not True
        or polyglot_rules.get("cross_language_config_api_or_process_work_required") is not True
    ):
        raise ValueError("the realistic polyglot work-boundary contract changed")
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
        or tuple(resources.get("repeated_operation_health_checks", []))
        != RESOURCE_HEALTH_METRICS
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
    if tuple(value.get("automated_accessibility_checks", [])) != (
        "keyboard_reachability",
        "visible_focus",
        "headings_and_labels",
        "narrow_and_zoomed_presentation",
        "korean_english_fixed_ui",
        "document_html_language",
    ):
        raise ValueError("the Phase 8 automated accessibility qualification changed")
    human_review = value.get("human_review_contract", {})
    if (
        human_review.get("artifact_kind") != "phase8_dogfood_human_review"
        or human_review.get("states") != ["not_provided", "passed", "failed"]
        or human_review.get("replacement_states")
        != ["pending_human_review", "passed", "failed"]
        or tuple(human_review.get("interaction_repository_classes", [])) != CLASSES
        or tuple(human_review.get("document_repository_classes", [])) != CLASSES
        or human_review.get("live_viewer_locales") != ["en", "ko"]
        or human_review.get("machine_accessibility_may_be_overridden") is not False
        or human_review.get("sampling_algorithm")
        != "every_automated_passed_interaction_cycle"
        or tuple(human_review.get("every_cycle_review_surfaces", []))
        != (
            "interaction",
            "generated_documents",
            "viewer_snapshot",
            "repository_intelligence",
            "cli_usability",
        )
        or tuple(human_review.get("interaction_behavior_criteria", []))
        != (
            "explicit_material_handling_quality",
            "hidden_material_discovery_quality",
            "unnecessary_interruption",
            "learning_fork_value",
            "learning_alternatives_and_tradeoffs",
            "pre_response_recommendation_anchoring",
            "post_response_feedback_quality",
            "learning_implementation_fidelity",
            "routine_detail_omission",
            "proportional_learning_cost",
        )
    ):
        raise ValueError("the Phase 8 campaign-level human review contract changed")
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
            "repository-scoped SessionStart activation is observed before product inquiry behavior is judged",
            "the first captured user turn matches the descriptor plain work_user_task after comparison-only CRLF-to-LF normalization and removal of terminal CR/LF characters",
            "after Project initialization source canonical goal Context from the exact descriptor work_user_task",
            "establish the repository baseline through repository_analyze before ordinary work",
            "record Engineering Choice Discovery with current Goal, baseline, source-grounded alternatives, effect categories, and independent or coupled relationships before Materiality Review",
            "record a typed Materiality Review bound to the exact Goal and pre-work Analysis Snapshot before the first affected ordinary write, and follow its production-derived workflow directive",
            "select inquiry behavior appropriate to the sealed behavior class and current evidence without prescribed Question choreography",
            "for explicit or hidden user-owned decision classes, source-ground and promote a genuinely material Question and record an explicit current-host user Decision",
            "for explicit or hidden user-owned decision classes, reuse the unresolved review dimension in the Question Candidate, obtain the explicit Decision, and revise the same review to ready-for-work before the affected write",
            "for hidden_user_owned_decision, observe meaningful repository investigation before the material Question path and before the first ordinary repository write that commits the affected outcome",
            "for research, delegated, or exploratory classes, correct non-interruption may pass without a Candidate, Question, or Decision",
            "for learning_deliberation, prove explicit participation, agent-owned authority, deliberation-worthy value, current-host response before feedback, terminal learning state, ready-for-work, and no manufactured Decision",
            "for learning_routine_control, prove explicit participation and routine value without a Learning Deliberation, Candidate, Question, or Decision",
            "perform real repository work after the baseline",
            "commands used only for incidental inspection need not become Checkpoint verification facts",
            "every command referenced by checkpoint_record passed or failed verification has a numeric exit_code from the same captured command result, through either complete-result forwarding or exact same-result output/status forwarding; output-only forwarding is outcome-unknown",
            "permit one or more successful work Checkpoints while preserving pause and handoff history",
            "select the latest terminal Checkpoint candidate after the last meaningful repository change without falling back past a malformed final candidate",
            "require the selected terminal Checkpoint to use the Goal Context identity, baseline, applicable Decision or evidence-backed no-Decision behavior basis, truthful numeric-exit verification, limits, and next meaningful state or step",
        )
        or tuple(evidence.get("resume_session_contract", []))
        != (
            "repository-scoped SessionStart activation is observed before continuation behavior is judged",
            "the first captured user turn matches the descriptor plain fresh_resume_user_task after comparison-only CRLF-to-LF normalization and removal of terminal CR/LF characters, and does not disclose Recall",
            "a fresh resume session resolves the repository-bound existing Project through project_resolve before Recall without initializing a replacement Project",
            "a fresh resume session invokes Recall after project_resolve and before repository inspection or continued work",
            "Recall preserves completed learning context as learning participation rather than a canonical Decision",
            "after Recall a fresh resume session establishes and retains a repository_analyze baseline before the first ordinary repository write",
            "after the fresh resume baseline, recompute Materiality Review/work authority before continued ordinary work rather than treating the recalled Checkpoint as current frontier authority",
            "change continuation produces a relevant repository change after the retained pre-write baseline plus separate numeric-exit validation after that change",
            "verified-state continuation requires a recalled completed Checkpoint, repository inspection, post-inspection numeric-exit verification, and no behavior contradicting the completed state",
            "paused or in-progress recalled work with an unfinished next step cannot use verified-state continuation",
            "Recall without repository inspection and post-inspection numeric-exit verification cannot qualify",
        )
        or evidence.get("codex_user_turn_transport_identity")
        != {
            "captured_text_allowance": (
                "CRLF-to-LF normalization and removal of only terminal CR/LF characters"
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
        or descriptor_contract.get("behavior_class_field") != "behavior_class"
        or descriptor_contract.get("evaluation_basis_field") != "evaluation_basis"
        or descriptor_contract.get("behavior_review_field") != "behavior_review"
        or tuple(descriptor_contract.get("identity_fields", []))
        != ("repository_class", "cycle", "repository_revision")
        or descriptor_contract.get("evidence_reference_field") != "evidence"
    ):
        raise ValueError("the Phase 8 cycle descriptor contract changed")
    evaluation_basis = evidence.get("bounded_evaluation_basis", {})
    if (
        tuple(evaluation_basis.get("required_fields", []))
        != (
            "behavior_class",
            "repository_facts",
            "accepted_contract_constraints",
            "delegated_boundaries",
            "possible_material_concerns",
            "consequences",
            "facts_not_for_user",
            "current_relevance",
        )
        or evaluation_basis.get("possible_material_concerns_are_exhaustive") is not False
        or evaluation_basis.get("unique_question_wording_required") is not False
        or evaluation_basis.get("unique_alternatives_required") is not False
        or evaluation_basis.get("unique_recommendation_required") is not False
        or evaluation_basis.get("prescribed_user_selection_required") is not False
        or evidence.get("full_replacement_session_count") != QUALIFICATION_SESSION_COUNT
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
            "missing_activation_outcome": "operator_environment_invalid",
        }
    ):
        raise ValueError("the Phase 8 evaluation-basis or work-blocker contract changed")
    behavior_review = evidence.get("behavior_review", {})
    agreement_contract = behavior_review.get("fact_authority_agreement", {})
    comparison_contract = behavior_review.get("classification_comparison", {})
    blind_first = behavior_review.get("blind_first_review", {})
    counterfactual_contract = behavior_review.get(
        "material_user_owned_counterfactual_review", {}
    )
    if (
        behavior_review.get("kind") != "phase8_behavior_review"
        or behavior_review.get("accepted_classifications") != list(BEHAVIOR_CLASSES)
        or behavior_review.get("required_independent_review_status") != "accepted"
        or behavior_review.get("required_independent_review_fields")
        != [
            "status",
            "reviewer_role",
            "basis",
            "review_preparation",
            "provisional_review",
            "classification_comparison",
            "fact_authority_agreement",
            "counterfactual_review",
        ]
        or blind_first.get("preparation_artifact_kind")
        != "phase8_blind_review_preparation"
        or blind_first.get("provisional_artifact_kind")
        != "phase8_provisional_behavior_review"
        or blind_first.get("preparation_fields")
        != [
            "review_slot_id",
            "candidate_head",
            "repository_revision",
            "reviewer_repository_path",
            "work_user_task",
            "fresh_resume_user_task",
            "work_scope",
            "owner_document_locations",
        ]
        or blind_first.get("provisional_fields")
        != [
            "review_slot_id",
            "status",
            "reviewer_role",
            "preparation_sha256",
            "classification",
            "materiality_conclusion",
            "material_outcome_unavoidable",
            "operator_prompt_does_not_disclose_material_outcome",
            "basis",
            "provenance_reference_indices",
        ]
        or blind_first.get("evaluator_material_visible_before_provisional_fix") is not False
        or blind_first.get("reviewer_order") != "opaque_review_slot_id"
        or blind_first.get("logical_identity_visible_before_provisional_fix") is not False
        or blind_first.get("recording_operation") != "record-provisional-review"
        or blind_first.get("recording_identity")
        != "candidate_and_opaque_review_slot"
        or blind_first.get("recording_transition")
        != "review_prepared_to_provisional_recorded"
        or blind_first.get("recording_success_exit_code") != 0
        or blind_first.get("recording_reads_evaluator_descriptor") is not False
        or blind_first.get("recording_compares_evaluator_classification_or_materiality")
        is not False
        or blind_first.get("recording_failure_atomic") is not True
        or blind_first.get("sealed_provisional_immutable_and_inventory_bound") is not True
        or blind_first.get("all_provisionals_required_before_any_reveal") is not True
        or blind_first.get("qualification_profile_reveal_operation")
        != "reveal-qualification-profile"
        or blind_first.get("evaluator_reveal_operation") != "seal-cycle"
        or blind_first.get("required_provisional_count_before_reveal")
        != QUALIFICATION_CYCLE_COUNT
        or blind_first.get("sealing_accepts_provisional_payload") is not False
        or agreement_contract.get("accepted_statuses")
        != ["agreed", "resolved_from_evidence"]
        or agreement_contract.get("sealing_blocked_status") != "unresolved_conflict"
        or comparison_contract.get("accepted_statuses")
        != ["agreed", "resolved_from_evidence"]
        or comparison_contract.get("sealing_blocked_status") != "unresolved_conflict"
        or comparison_contract.get("required_fields")
        != [
            "status",
            "provisional_classification",
            "evaluator_classification",
            "disagreements",
            "resolution_basis",
            "provenance_reference_indices",
        ]
        or comparison_contract.get("mechanical_disagreement_fields")
        != [
            "classification",
            "materiality_conclusion",
            "material_outcome_unavoidable",
            "operator_prompt_disclosure",
        ]
        or comparison_contract.get("provisional_artifact_rewritten") is not False
        or counterfactual_contract.get("applicability")
        != "required_for_material_user_owned_decision"
        or counterfactual_contract.get("accepted_conclusion")
        != "unavoidable_user_owned_outcome"
        or counterfactual_contract.get("rejecting_task_satisfaction")
        != "fully_satisfies_without_user_owned_outcome"
        or counterfactual_contract.get("question_wording_prescribed") is not False
        or counterfactual_contract.get("alternatives_prescribed") is not False
        or counterfactual_contract.get("user_selection_prescribed") is not False
        or behavior_review.get("non_user_decision_counterfactual_applicability")
        != "not_required_for_behavior_class"
        or behavior_review.get("purpose")
        != "bounded independent evidence and no-question counterfactual review without prescribing one Question expression or user selection"
        or behavior_review.get("visibility") != "evaluator_input_only"
    ):
        raise ValueError("the Phase 8 behavior-review contract changed")
    if evidence.get("opaque_slot_contract") != {
        "identity_generation": "campaign_time_cryptographic_random_128_bit_token",
        "stable_for_prepared_campaign": True,
        "unique_within_campaign": True,
        "derived_from_repository_or_cycle": False,
        "physical_workspace_layout": "slots/<review_slot_id>/repository",
        "reviewer_workspace_layout": "reviewer/workspaces/<review_slot_id>/repository",
        "reviewer_artifact_naming": "review_slot_id_only",
        "operator_ordering": "opaque_review_slot_id_with_optional_repository_grouping",
        "private_mapping_visibility": "evaluator_steward_private_until_all_provisional_reviews_are_fixed",
        "private_mapping_integrity": "campaign_bound_sha256_and_evidence_inventory",
        "numeric_compatibility_branch": False,
    }:
        raise ValueError("the Phase 8 opaque-slot contract changed")
    batch = evidence.get("batch_campaign_contract", {})
    if (
        batch.get("operation") != "collect-batch"
        or batch.get("required_raw_rollout_count") != QUALIFICATION_SESSION_COUNT
        or batch.get("global_mapping_precedes_campaign_mutation") is not True
        or batch.get("raw_rollout_bytes_preserved") is not True
        or batch.get("terminal_work_failure_repaired_by_resume") is not False
        or batch.get("missing_activation_classification")
        != "operator_environment_invalid"
        or "read_only_static_viewer_snapshot"
        not in batch.get("automatic_cycle_evidence", [])
    ):
        raise ValueError("the Phase 8 batch campaign contract changed")
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
        python_files = [item for item in files if item.suffix.casefold() == ".py"]
        test_files = [
            item
            for item in python_files
            if item.name.startswith("test_") or "tests" in item.relative_to(path).parts
        ]
        production_files = [item for item in python_files if item not in test_files]
        configuration_files = [
            item
            for item in files
            if item.name
            in {
                "pyproject.toml",
                "setup.cfg",
                "setup.py",
                "tox.ini",
                "requirements.txt",
                "requirements-dev.txt",
            }
        ]
        if languages != ["Python"]:
            blockers.append("small repository is not a single official Python application")
        if len(files) < rules["minimum_files"]:
            blockers.append("small Python application does not meet the realistic file floor")
        if len(files) > rules["maximum_files"]:
            blockers.append("small repository exceeds the bounded file ceiling")
        if len(production_files) < rules["production_source_files_required"]:
            blockers.append("small Python application lacks meaningful multi-file production structure")
        if len(test_files) < rules["test_files_required"]:
            blockers.append("small Python application lacks multiple focused test files")
        if not configuration_files:
            blockers.append("small Python application lacks maintained configuration")
        if not any(
            len(item.relative_to(path).parts) > 1 for item in production_files
        ):
            blockers.append("small Python application lacks a package or application boundary")
    if kind == "polyglot-medium":
        if len(files) < rules["minimum_files"]:
            blockers.append("polyglot repository does not meet the medium file floor")
        if len(languages) < rules["minimum_official_structural_languages"]:
            blockers.append("polyglot repository has fewer than three official structural languages")
        if documents == 0:
            blockers.append("polyglot repository has no documentation")
        manifests = [
            item
            for item in files
            if item.name
            in {
                "Cargo.toml",
                "package.json",
                "pyproject.toml",
                "pom.xml",
                "build.gradle",
                "CMakeLists.txt",
            }
        ]
        if len(manifests) < 2:
            blockers.append("polyglot repository lacks a genuine multi-component build boundary")
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
    value["_evidence_directory"] = str(manifest_directory.resolve())
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


def verified_json_evidence(
    reference: Any,
    evidence_directory: Path | None,
) -> tuple[Path | None, dict[str, Any] | None]:
    path = verified_evidence_path(reference, evidence_directory)
    if path is None:
        return None, None
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return path, None
    return path, value if isinstance(value, dict) else None


def campaign_support_evidence(
    evidence: dict[str, Any],
    evidence_directory: Path | None,
    bundle: CanonicalBundle | None,
    *,
    project_id: str | None,
    candidate_revision: str | None,
    kind: str,
    cycle: int,
) -> tuple[dict[str, bool], dict[str, Any]]:
    _runtime_path, runtime = verified_json_evidence(
        evidence.get("runtime_summary"), evidence_directory
    )
    _activation_path, activation = verified_json_evidence(
        evidence.get("activation_summary"), evidence_directory
    )
    _documents_path, documents = verified_json_evidence(
        evidence.get("generated_documents"), evidence_directory
    )
    _snapshot_path, snapshot = verified_json_evidence(
        evidence.get("viewer_snapshot"), evidence_directory
    )
    document_files_valid = False
    if (
        isinstance(documents, dict)
        and documents.get("kind") == "phase8_generated_document_evidence_summary"
        and documents.get("status") == "passed"
    ):
        document_values = documents.get("documents")
        document_files_valid = (
            isinstance(document_values, dict)
            and len(document_values) == 4
            and all(isinstance(item, dict) for item in document_values.values())
            and all(
                item.get("status") == "passed"
                and all(
                    format_value.get("status") == "passed"
                    and verified_evidence_path(
                        {
                            "file": format_value.get("relative_evidence_path"),
                            "sha256": format_value.get("sha256"),
                        },
                        evidence_directory,
                    )
                    is not None
                    for format_value in item.get("formats", {}).values()
                )
                and len(item.get("formats", {})) == 2
                for item in document_values.values()
            )
        )
    snapshot_file = (
        verified_evidence_path(
            {
                "file": snapshot.get("relative_evidence_path"),
                "sha256": snapshot.get("sha256"),
            },
            evidence_directory,
        )
        if isinstance(snapshot, dict)
        else None
    )
    checks = {
        "canonical_bundle_and_provenance": (
            bundle is not None
            and nonempty_string(project_id)
            and bundle.project_id == project_id
        ),
        "generated_document_outputs": document_files_valid,
        "static_viewer_snapshot": (
            isinstance(snapshot, dict)
            and snapshot.get("kind") == "phase8_viewer_snapshot_evidence_summary"
            and snapshot.get("status") == "passed"
            and snapshot.get("project_id") == project_id
            and snapshot.get("candidate_head") == candidate_revision
            and snapshot.get("repository_class") == kind
            and snapshot.get("cycle") == cycle
            and snapshot_file is not None
        ),
        "bounded_runtime_and_activation_evidence": (
            isinstance(runtime, dict)
            and runtime.get("kind") == "phase8_bounded_runtime_summary"
            and runtime.get("content_included") is False
            and isinstance(activation, dict)
            and activation.get("kind") == "phase8_dogfood_activation_summary"
            and activation.get("repository_class") == kind
            and activation.get("cycle") == cycle
            and activation.get("work_session_start_activation_observed") is True
            and activation.get("resume_session_start_activation_observed") is True
        ),
    }
    return checks, {
        "canonical_bundle_sha256": bundle.source_sha256 if bundle is not None else None,
        "generated_document_summary_status": (
            documents.get("status") if isinstance(documents, dict) else "unavailable"
        ),
        "static_viewer_snapshot_status": (
            snapshot.get("status") if isinstance(snapshot, dict) else "unavailable"
        ),
        "runtime_summary_content_included": (
            runtime.get("content_included") if isinstance(runtime, dict) else None
        ),
        "activation_summary_complete": checks[
            "bounded_runtime_and_activation_evidence"
        ],
    }


def unique_call(capture: CodexCapture | None, operation: str) -> ToolCall | None:
    if capture is None:
        return None
    calls = capture.successful_calls(operation)
    return calls[0] if len(calls) == 1 else None


def checkpoint_baseline_calls(
    capture: CodexCapture | None,
    checkpoint: ToolCall | None,
) -> list[ToolCall]:
    """Select successful analyses by the exact identity retained by a Checkpoint."""
    if capture is None or checkpoint is None:
        return []
    project_id = checkpoint.arguments.get("project_id")
    baseline_id = checkpoint.arguments.get("baseline_analysis_snapshot_id")
    if not nonempty_string(project_id) or not nonempty_string(baseline_id):
        return []
    return [
        call
        for call in capture.successful_calls("repository_analyze")
        if call.arguments.get("project_id") == project_id
        and call.result.get("project_id") == project_id
        and call.result.get("analysis_snapshot_id") == baseline_id
        and call.completion_sequence < checkpoint.sequence
    ]


def selected_checkpoint_baseline_call(
    capture: CodexCapture | None,
    checkpoint: ToolCall | None,
) -> ToolCall | None:
    calls = checkpoint_baseline_calls(capture, checkpoint)
    return min(calls, key=lambda call: call.completion_sequence) if calls else None


def checkpoint_baseline_is_pre_work(
    capture: CodexCapture | None,
    checkpoint: ToolCall | None,
    *,
    project_id: str,
    boundary_completion_sequence: int,
    first_write_sequence: int | None,
) -> bool:
    if checkpoint is None or checkpoint.outcome != "succeeded":
        return False
    baseline_id = checkpoint.arguments.get("baseline_analysis_snapshot_id")
    if checkpoint.result.get("baseline_analysis_snapshot_id") != baseline_id:
        return False
    matching = checkpoint_baseline_calls(capture, checkpoint)
    return any(
        call.arguments.get("project_id") == project_id
        and boundary_completion_sequence < call.sequence
        and (
            first_write_sequence is None
            or call.completion_sequence < first_write_sequence
        )
        for call in matching
    )


def normalized_prompt_text(value: str) -> str:
    return " ".join(value.casefold().split())


def codex_user_turn_transport_identity_matches(
    captured_user_turn: Any,
    descriptor_task: Any,
) -> bool:
    if not isinstance(captured_user_turn, str) or not isinstance(descriptor_task, str):
        return False
    return canonical_frozen_task_transport_text(
        captured_user_turn
    ) == canonical_frozen_task_transport_text(descriptor_task)


def canonical_frozen_task_transport_text(value: str) -> str:
    """Canonicalize only transport-equivalent line endings for task identity."""
    return value.replace("\r\n", "\n").rstrip("\r\n")


def plain_user_task_error(value: Any, field: str) -> str | None:
    if not nonempty_string(value) or value != value.strip():
        return f"{field} must be a non-empty exact plain user task"
    encoded = value.encode("utf-8")
    if len(encoded) > MAX_USER_TASK_BYTES or any(ord(character) < 32 and character not in "\n\t" for character in value):
        return f"{field} exceeds the bounded plain-task contract"
    return None


def bounded_text_list_errors(
    value: Any,
    field: str,
    *,
    minimum: int = 0,
    owner: str = "evaluation_basis",
) -> list[str]:
    if (
        not isinstance(value, list)
        or len(value) < minimum
        or len(value) > 32
        or len(value) != len(set(value))
        or not all(
            nonempty_string(item) and len(item.encode("utf-8")) <= MAX_REVIEW_TEXT_BYTES
            for item in value
        )
    ):
        return [f"{owner}.{field} must contain bounded unique text entries"]
    return []


def evaluation_basis_errors(value: Any, behavior_class: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["evaluation_basis must be bounded evaluator material"]
    errors: list[str] = []
    required = {
        "behavior_class",
        "repository_facts",
        "accepted_contract_constraints",
        "delegated_boundaries",
        "possible_material_concerns",
        "consequences",
        "facts_not_for_user",
        "current_relevance",
    }
    if set(value) != required:
        errors.append("evaluation_basis must contain the current bounded fields only")
    if value.get("behavior_class") != behavior_class or behavior_class not in BEHAVIOR_CLASSES:
        errors.append("evaluation_basis.behavior_class must match the cycle behavior class")
    for field, minimum in (
        ("repository_facts", 1),
        ("accepted_contract_constraints", 0),
        ("delegated_boundaries", 0),
        ("possible_material_concerns", 0),
        ("consequences", 1),
        ("facts_not_for_user", 1),
    ):
        errors.extend(bounded_text_list_errors(value.get(field), field, minimum=minimum))
    relevance = value.get("current_relevance")
    if not nonempty_string(relevance) or len(relevance.encode("utf-8")) > MAX_REVIEW_TEXT_BYTES:
        errors.append("evaluation_basis.current_relevance must be bounded non-empty text")
    if is_user_owned_behavior(behavior_class) and not value.get("possible_material_concerns"):
        errors.append(
            "material user-owned behavior requires at least one non-exhaustive material concern"
        )
    if behavior_class == "delegated_implementation_choice" and not value.get("delegated_boundaries"):
        errors.append("delegated_implementation_choice requires an explicit delegated boundary")
    if behavior_class == "research_or_no_question" and not value.get("repository_facts"):
        errors.append("research_or_no_question requires repository facts")
    return errors


def behavior_review_errors(
    value: Any,
    behavior_class: Any,
    *,
    candidate_revision: str | None = None,
    target_revision: str | None = None,
    candidate_root: Path = ROOT,
    target_repository: Path | None = None,
    verify_provenance: bool = False,
) -> list[str]:
    if not isinstance(value, dict) or value.get("kind") != "phase8_behavior_review":
        return ["behavior_review must be a Phase 8 behavior review"]
    errors: list[str] = []
    if value.get("classification") != behavior_class or behavior_class not in BEHAVIOR_CLASSES:
        errors.append("behavior_review.classification must match the maintained behavior class")
    for field in (
        "outcome_rationale",
        "user_ownership_assessment",
        "silent_choice_risk_assessment",
    ):
        field_value = value.get(field)
        if (
            not nonempty_string(field_value)
            or len(field_value.encode("utf-8")) > MAX_REVIEW_TEXT_BYTES
        ):
            errors.append(f"behavior_review.{field} must be bounded non-empty text")
    expected_unresolved = is_user_owned_behavior(behavior_class)
    if value.get("unresolved_material_user_outcome") is not expected_unresolved:
        errors.append("behavior_review unresolved material-user outcome does not match the class")
    references = value.get("provenance_references")
    if not isinstance(references, list) or not references or len(references) > 32:
        errors.append("behavior_review.provenance_references must contain bounded typed references")
        references = []
    reference_keys: list[tuple[Any, ...]] = []
    for index, reference in enumerate(references):
        prefix = f"behavior_review.provenance_references[{index}]"
        if not isinstance(reference, dict) or set(reference) != {
            "scope", "path", "sha256", "repository_revision"
        }:
            errors.append(f"{prefix} must contain one current typed reference")
            continue
        scope = reference.get("scope")
        path = safe_relative_evidence_path(reference.get("path"))
        content_hash = reference.get("sha256")
        reference_revision = reference.get("repository_revision")
        if scope not in BEHAVIOR_REVIEW_PROVENANCE_SCOPES:
            errors.append(f"{prefix}.scope is unsupported")
        if path is None:
            errors.append(f"{prefix}.path must be a safe relative path")
        if not valid_capture_sha256(content_hash):
            errors.append(f"{prefix}.sha256 must be a SHA-256 identity")
        if not isinstance(reference_revision, str) or re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", reference_revision) is None:
            errors.append(f"{prefix}.repository_revision must be a full Git identity")
        reference_keys.append((scope, path, content_hash, reference_revision))
        if scope == "volicord_active_owner" and path not in ACTIVE_ARCHITECTURE_OWNER_PATHS:
            errors.append(f"{prefix}.path is not a current active architecture owner")
        if not verify_provenance or path is None or not valid_capture_sha256(content_hash):
            continue
        if scope == "volicord_active_owner":
            if reference_revision != candidate_revision:
                errors.append(f"{prefix} is bound to the wrong candidate revision")
                continue
            content = git_blob_bytes(candidate_root, reference_revision, path)
        elif scope == "target_repository":
            if target_repository is None:
                errors.append(f"{prefix} cannot be verified without the pinned target repository")
                continue
            if reference_revision != target_revision:
                errors.append(f"{prefix} is bound to the wrong pinned target revision")
                continue
            content = git_blob_bytes(target_repository, reference_revision, path)
        else:
            continue
        if content is None:
            errors.append(f"{prefix}.path does not exist at the bound revision")
        elif hashlib.sha256(content).hexdigest() != content_hash:
            errors.append(f"{prefix}.sha256 is stale for the bound revision")
    if len(reference_keys) != len(set(reference_keys)):
        errors.append("behavior_review.provenance_references must be unique")
    independent = value.get("independent_review")
    required_independent_fields = {
        "status",
        "reviewer_role",
        "basis",
        "review_preparation",
        "provisional_review",
        "classification_comparison",
        "fact_authority_agreement",
        "counterfactual_review",
    }
    if not isinstance(independent, dict) or set(independent) != required_independent_fields:
        errors.append("behavior_review requires the current independent review fields")
        return errors
    if (
        independent.get("status") != "accepted"
        or independent.get("reviewer_role") != "campaign_preparation_independent_reviewer"
        or not nonempty_string(independent.get("basis"))
        or len(independent.get("basis", "").encode("utf-8")) > MAX_REVIEW_TEXT_BYTES
    ):
        errors.append("behavior_review requires an accepted independent review")
    errors.extend(
        blind_first_review_errors(
            independent.get("review_preparation"),
            independent.get("provisional_review"),
            len(references),
        )
    )
    errors.extend(
        classification_comparison_errors(
            independent.get("classification_comparison"),
            independent.get("provisional_review"),
            behavior_class,
            len(references),
        )
    )
    errors.extend(
        fact_authority_agreement_errors(
            independent.get("fact_authority_agreement"),
            len(references),
        )
    )
    errors.extend(
        counterfactual_review_errors(
            independent.get("counterfactual_review"),
            behavior_class,
        )
    )
    return errors


def blind_first_review_errors(
    preparation: Any,
    provisional: Any,
    reference_count: int,
) -> list[str]:
    errors: list[str] = []
    if not isinstance(preparation, dict) or set(preparation) != {
        "kind",
        "review_slot_id",
        "sha256",
    }:
        return ["independent review requires one blind review preparation reference"]
    if preparation.get("kind") != "phase8_blind_review_preparation_reference" or not valid_capture_sha256(
        preparation.get("sha256")
    ) or re.fullmatch(r"[0-9a-f]{32}", str(preparation.get("review_slot_id", ""))) is None:
        errors.append("blind review preparation reference is malformed")
    required = {
        "kind",
        "review_slot_id",
        "status",
        "reviewer_role",
        "preparation_sha256",
        "classification",
        "materiality_conclusion",
        "material_outcome_unavoidable",
        "operator_prompt_does_not_disclose_material_outcome",
        "basis",
        "provenance_reference_indices",
    }
    if not isinstance(provisional, dict) or set(provisional) != required:
        errors.append("independent review requires the fixed provisional review fields")
        return errors
    if (
        provisional.get("kind") != "phase8_provisional_behavior_review"
        or provisional.get("status") != "recorded"
        or provisional.get("reviewer_role")
        != "campaign_preparation_independent_reviewer"
        or provisional.get("review_slot_id") != preparation.get("review_slot_id")
        or provisional.get("preparation_sha256") != preparation.get("sha256")
    ):
        errors.append("provisional review is not fixed to the blind preparation")
    classification = provisional.get("classification")
    if classification not in BEHAVIOR_CLASSES:
        errors.append("provisional review classification is unsupported")
    basis = provisional.get("basis")
    if not nonempty_string(basis) or len(basis.encode("utf-8")) > MAX_REVIEW_TEXT_BYTES:
        errors.append("provisional review requires bounded source-grounded reasoning")
    indices = provisional.get("provenance_reference_indices")
    if (
        not isinstance(indices, list)
        or not indices
        or len(indices) != len(set(indices))
        or any(not isinstance(index, int) or isinstance(index, bool) for index in indices)
        or any(index < 0 or index >= reference_count for index in indices)
    ):
        errors.append("provisional review must cite reviewer-visible provenance locations")
    if is_user_owned_behavior(classification):
        if provisional.get("materiality_conclusion") != "user_owned_material_outcome":
            errors.append("user-owned provisional review must identify a material user-owned outcome")
        if provisional.get("material_outcome_unavoidable") is not True:
            errors.append("user-owned provisional review must find the material outcome unavoidable")
        expected_non_disclosure = classification == "hidden_user_owned_decision"
        if (
            provisional.get("operator_prompt_does_not_disclose_material_outcome")
            is not expected_non_disclosure
        ):
            errors.append("provisional prompt-disclosure conclusion is inconsistent with its classification")
    elif classification in BEHAVIOR_CLASSES:
        if provisional.get("materiality_conclusion") != "no_user_owned_material_outcome":
            errors.append("non-user-owned provisional review must reject a material user-owned outcome")
        if provisional.get("material_outcome_unavoidable") is not False:
            errors.append("non-user-owned provisional review must reject an unavoidable user-owned outcome")
        if provisional.get("operator_prompt_does_not_disclose_material_outcome") is not None:
            errors.append("non-user-owned provisional review does not classify hidden prompt disclosure")
    return errors


def classification_comparison_errors(
    value: Any,
    provisional: Any,
    behavior_class: Any,
    reference_count: int,
) -> list[str]:
    required = {
        "status",
        "provisional_classification",
        "evaluator_classification",
        "disagreements",
        "resolution_basis",
        "provenance_reference_indices",
    }
    if not isinstance(value, dict) or set(value) != required:
        return ["independent review requires the current classification comparison fields"]
    errors: list[str] = []
    status = value.get("status")
    if status not in {"agreed", "resolved_from_evidence", "unresolved_conflict"}:
        errors.append("classification comparison status is unsupported")
    provisional_classification = (
        provisional.get("classification") if isinstance(provisional, dict) else None
    )
    if value.get("provisional_classification") != provisional_classification:
        errors.append("classification comparison must retain the immutable provisional classification")
    if value.get("evaluator_classification") != behavior_class:
        errors.append("classification comparison must identify the evaluator classification")

    expected_materiality = (
        "user_owned_material_outcome"
        if is_user_owned_behavior(behavior_class)
        else "no_user_owned_material_outcome"
    )
    expected_unavoidable = is_user_owned_behavior(behavior_class)
    expected_disclosure = (
        True
        if behavior_class == "hidden_user_owned_decision"
        else False
        if behavior_class == "explicit_user_owned_decision"
        else None
    )
    expected_disagreements: list[str] = []
    if provisional_classification != behavior_class:
        expected_disagreements.append("classification")
    if not isinstance(provisional, dict) or provisional.get("materiality_conclusion") != expected_materiality:
        expected_disagreements.append("materiality_conclusion")
    if not isinstance(provisional, dict) or provisional.get("material_outcome_unavoidable") is not expected_unavoidable:
        expected_disagreements.append("material_outcome_unavoidable")
    if (
        not isinstance(provisional, dict)
        or provisional.get("operator_prompt_does_not_disclose_material_outcome")
        is not expected_disclosure
    ):
        expected_disagreements.append("operator_prompt_disclosure")
    disagreements = value.get("disagreements")
    if disagreements != expected_disagreements:
        errors.append("classification comparison must enumerate the exact evaluator-relative disagreements")

    basis = value.get("resolution_basis")
    if not nonempty_string(basis) or len(basis.encode("utf-8")) > MAX_REVIEW_TEXT_BYTES:
        errors.append("classification comparison requires a bounded resolution basis")
    indices = value.get("provenance_reference_indices")
    if (
        not isinstance(indices, list)
        or not indices
        or len(indices) != len(set(indices))
        or any(not isinstance(index, int) or isinstance(index, bool) for index in indices)
        or any(index < 0 or index >= reference_count for index in indices)
    ):
        errors.append("classification comparison must cite inspectable provenance references")
    if expected_disagreements:
        if status == "agreed":
            errors.append("classification or materiality disagreement cannot be marked agreed")
        elif status == "unresolved_conflict":
            errors.append("unresolved classification or materiality disagreement blocks sealing")
    elif status != "agreed":
        errors.append("matching classification and materiality conclusions must be marked agreed")
    return errors


def fact_authority_agreement_errors(value: Any, reference_count: int) -> list[str]:
    required = {
        "status",
        "evaluator_conclusions",
        "reviewer_conclusions",
        "conflicts",
        "resolution_basis",
        "provenance_reference_indices",
    }
    if not isinstance(value, dict) or set(value) != required:
        return ["independent review requires the current fact/authority agreement fields"]
    errors: list[str] = []
    status = value.get("status")
    if status not in {"agreed", "resolved_from_evidence", "unresolved_conflict"}:
        errors.append("fact/authority agreement status is unsupported")
    for field in ("evaluator_conclusions", "reviewer_conclusions"):
        errors.extend(
            bounded_text_list_errors(
                value.get(field),
                field,
                minimum=1,
                owner="fact_authority_agreement",
            )
        )
    errors.extend(
        bounded_text_list_errors(
            value.get("conflicts"),
            "conflicts",
            owner="fact_authority_agreement",
        )
    )
    basis = value.get("resolution_basis")
    if not nonempty_string(basis) or len(basis.encode("utf-8")) > MAX_REVIEW_TEXT_BYTES:
        errors.append("fact/authority agreement requires a bounded resolution basis")
    indices = value.get("provenance_reference_indices")
    if (
        not isinstance(indices, list)
        or not indices
        or len(indices) != len(set(indices))
        or any(not isinstance(index, int) or isinstance(index, bool) for index in indices)
        or any(index < 0 or index >= reference_count for index in indices)
    ):
        errors.append("fact/authority agreement must cite inspectable provenance references")
    conflicts = value.get("conflicts") if isinstance(value.get("conflicts"), list) else []
    if status == "agreed" and conflicts:
        errors.append("agreed fact/authority conclusions cannot retain conflicts")
    if status == "resolved_from_evidence" and not conflicts:
        errors.append("resolved fact/authority disagreement must identify the resolved conflicts")
    if status == "unresolved_conflict":
        errors.append("unresolved evaluator/reviewer fact or authority disagreement blocks sealing")
    return errors


def counterfactual_review_errors(value: Any, behavior_class: Any) -> list[str]:
    required = {
        "applicability",
        "specific_unresolved_outcome",
        "frozen_task_necessity",
        "repository_research_cannot_settle",
        "repository_facts_settle_outcome",
        "accepted_decision_or_contract_cannot_settle",
        "accepted_decision_or_contract_settles_outcome",
        "not_delegated_basis",
        "outcome_within_delegated_authority",
        "materially_different_consequences",
        "no_question_approaches",
        "material_outcome_unavoidable",
        "operator_prompt_does_not_disclose_material_outcome",
        "conclusion",
    }
    if not isinstance(value, dict) or set(value) != required:
        return ["independent review requires the current no-question counterfactual fields"]
    errors: list[str] = []
    if not is_user_owned_behavior(behavior_class):
        if value != {
            "applicability": "not_required_for_behavior_class",
            "specific_unresolved_outcome": None,
            "frozen_task_necessity": None,
            "repository_research_cannot_settle": None,
            "repository_facts_settle_outcome": None,
            "accepted_decision_or_contract_cannot_settle": None,
            "accepted_decision_or_contract_settles_outcome": None,
            "not_delegated_basis": None,
            "outcome_within_delegated_authority": None,
            "materially_different_consequences": [],
            "no_question_approaches": [],
            "material_outcome_unavoidable": False,
            "operator_prompt_does_not_disclose_material_outcome": None,
            "conclusion": "not_applicable",
        }:
            errors.append(
                "non-user-decision behavior classes use a non-applicable counterfactual without Decision ceremony"
            )
        return errors

    if value.get("applicability") != "required_for_material_user_owned_decision":
        errors.append("material user-owned behavior requires an independent no-question counterfactual")
    for field in (
        "specific_unresolved_outcome",
        "frozen_task_necessity",
        "repository_research_cannot_settle",
        "accepted_decision_or_contract_cannot_settle",
        "not_delegated_basis",
    ):
        field_value = value.get(field)
        if (
            not nonempty_string(field_value)
            or len(field_value.encode("utf-8")) > MAX_REVIEW_TEXT_BYTES
        ):
            errors.append(f"counterfactual_review.{field} must be bounded non-empty text")
    for field in (
        "repository_facts_settle_outcome",
        "accepted_decision_or_contract_settles_outcome",
        "outcome_within_delegated_authority",
    ):
        if value.get(field) is not False:
            errors.append(f"counterfactual_review.{field} must be false for user-owned sealing")
    errors.extend(
        bounded_text_list_errors(
            value.get("materially_different_consequences"),
            "materially_different_consequences",
            minimum=2,
            owner="counterfactual_review",
        )
    )
    approaches = value.get("no_question_approaches")
    if not isinstance(approaches, list) or not approaches or len(approaches) > 16:
        errors.append("material user-owned behavior requires bounded no-question approaches")
        approaches = []
    for index, approach in enumerate(approaches):
        prefix = f"counterfactual_review.no_question_approaches[{index}]"
        if not isinstance(approach, dict) or set(approach) != {
            "approach",
            "task_satisfaction",
            "assessment",
        }:
            errors.append(f"{prefix} must contain the current counterfactual fields")
            continue
        for field in ("approach", "assessment"):
            field_value = approach.get(field)
            if (
                not nonempty_string(field_value)
                or len(field_value.encode("utf-8")) > MAX_REVIEW_TEXT_BYTES
            ):
                errors.append(f"{prefix}.{field} must be bounded non-empty text")
        satisfaction = approach.get("task_satisfaction")
        if satisfaction not in {
            "fails_frozen_task",
            "implicitly_chooses_same_user_owned_outcome",
            "fully_satisfies_without_user_owned_outcome",
        }:
            errors.append(f"{prefix}.task_satisfaction is unsupported")
        if satisfaction == "fully_satisfies_without_user_owned_outcome":
            errors.append(
                "a defensible no-question path fully satisfies the frozen task and prevents material user-owned sealing"
            )
    if value.get("material_outcome_unavoidable") is not True:
        errors.append("accepted material user-owned review must explicitly find the outcome unavoidable")
    expected_non_disclosure = behavior_class == "hidden_user_owned_decision"
    if value.get("operator_prompt_does_not_disclose_material_outcome") is not expected_non_disclosure:
        errors.append("counterfactual prompt-disclosure conclusion does not match the behavior class")
    if value.get("conclusion") != "unavoidable_user_owned_outcome":
        errors.append("accepted material user-owned behavior requires an unavoidable outcome conclusion")
    return errors


def naturalistic_prompt_errors(work_task: Any, resume_task: Any, evaluation_basis: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(work_task, str) or not isinstance(resume_task, str) or not isinstance(evaluation_basis, dict):
        return ["naturalistic prompt integrity requires both plain tasks and a bounded evaluation basis"]
    prompts = (("work_user_task", work_task), ("fresh_resume_user_task", resume_task))
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
        *evaluation_basis.get("accepted_contract_constraints", []),
        *evaluation_basis.get("delegated_boundaries", []),
        *evaluation_basis.get("possible_material_concerns", []),
        *evaluation_basis.get("consequences", []),
        *evaluation_basis.get("facts_not_for_user", []),
        evaluation_basis.get("current_relevance"),
    ]
    for hidden in hidden_values:
        if not nonempty_string(hidden):
            continue
        hidden_text = normalized_prompt_text(hidden)
        for field, prompt in prompts:
            if hidden_text in normalized_prompt_text(prompt):
                errors.append(f"{field} discloses exact evaluator-only behavior material")

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


def hidden_prompt_static_disclosure_errors(work_task: Any, resume_task: Any) -> list[str]:
    """Conservative supplement to the required independent semantic review."""

    if not isinstance(work_task, str) or not isinstance(resume_task, str):
        return ["hidden-decision prompt review requires both frozen tasks"]
    errors: list[str] = []
    patterns = (
        r"\b(?:unsettled|undecided|not\s+(?:yet\s+)?(?:settled|decided))\b",
        r"\buser\s+must\s+choose\b",
        r"\bchoose\s+before\s+(?:implementation|implementing|work)\b",
        r"\bresolve\s+(?:the\s+)?(?:product\s+)?decision\b",
        r"\bhidden\s+material(?:ity)?\b",
        r"\b(?:volicord|inquiry|question\s+candidate|checkpoint|recall|behavior\s+class)\b",
    )
    for field, prompt in (("work_user_task", work_task), ("fresh_resume_user_task", resume_task)):
        lowered = normalized_prompt_text(prompt)
        if any(re.search(pattern, lowered) for pattern in patterns):
            errors.append(
                f"{field} statically telegraphs the hidden material-decision evaluation"
            )
    return errors


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
        if (
            looks_like_synthetic_marker(path)
            or Path(path).suffix.lower() in {".txt", ".marker"}
            or generated_repository_path(path)
        ):
            continue
        candidate = Path(path)
        terms = {path.casefold(), candidate.name.casefold(), candidate.stem.casefold()}
        terms.update(part.casefold() for part in candidate.parts if len(part) >= 4)
        if any(term not in generic and term in lowered for term in terms):
            result.append(path)
    return sorted(set(result))


def generated_repository_path(path: str) -> bool:
    return any(
        part in {
            "build",
            "dist",
            "target",
            ".cache",
            ".venv",
            ".ruff_cache",
            "__pycache__",
            ".pytest_cache",
            ".mypy_cache",
        }
        for part in Path(path).parts
    )


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


def work_scope_errors(
    value: Any,
    repository_class: Any,
    repository_revision: Any,
    target_repository: Path | None,
    verify_provenance: bool,
) -> list[str]:
    if not isinstance(value, dict) or set(value) != {
        "affected_paths",
        "user_visible_behavior",
        "boundary_kind",
    }:
        return ["work_scope must contain affected paths, visibility, and boundary kind"]
    errors: list[str] = []
    paths = value.get("affected_paths")
    if (
        not isinstance(paths, list)
        or not paths
        or len(paths) > 16
        or len(paths) != len(set(paths))
        or any(safe_relative_evidence_path(path) is None for path in paths)
    ):
        errors.append("work_scope.affected_paths must contain bounded safe unique paths")
        paths = []
    if not isinstance(value.get("user_visible_behavior"), bool):
        errors.append("work_scope.user_visible_behavior must be boolean")
    if value.get("boundary_kind") not in {
        "component",
        "language",
        "configuration",
        "api",
        "process",
    }:
        errors.append("work_scope.boundary_kind is unsupported")
    if repository_class == "small-python" and len(paths) < 2 and value.get("user_visible_behavior") is not True:
        errors.append("small-python work must be multi-file or user-visible")
    if repository_class == "polyglot-medium" and len(paths) < 2:
        errors.append("polyglot-medium work must cross at least two bounded paths")
    if verify_provenance and target_repository is not None and isinstance(repository_revision, str):
        # A naturalistic task may legitimately create a new affected path. The
        # captured change and repository revision prove the work boundary; the
        # descriptor must not turn the pre-work tree into a prescribed file list.
        if repository_class == "polyglot-medium" and paths:
            suffix_languages = {
                OFFICIAL_SUFFIXES[Path(path).suffix.casefold()]
                for path in paths
                if Path(path).suffix.casefold() in OFFICIAL_SUFFIXES
            }
            top_level_areas = {
                Path(path).parts[0]
                for path in paths
                if Path(path).parts[0].casefold()
                not in {"src", "test", "tests", "docs", "config"}
            }
            if len(suffix_languages) < 2 and len(top_level_areas) < 2:
                errors.append("polyglot-medium work scope does not cross a language or component boundary")
    return errors


def cycle_descriptor_errors(
    value: Any,
    *,
    candidate_revision: str | None = None,
    candidate_root: Path = ROOT,
    target_repository: Path | None = None,
    verify_provenance: bool = False,
) -> list[str]:
    if not isinstance(value, dict) or value.get("kind") != "phase8_cycle_descriptor":
        return ["descriptor kind must be phase8_cycle_descriptor"]
    errors: list[str] = []
    for obsolete in ("objective", "resume_change_scope", "work_session_contract", "resume_session_contract"):
        if obsolete in value:
            errors.append(f"descriptor does not support obsolete field {obsolete}")
    if value.get("repository_class") not in CLASSES:
        errors.append("repository_class must identify a Phase 8 repository class")
    behavior_class = value.get("behavior_class")
    if behavior_class not in BEHAVIOR_CLASSES:
        errors.append("behavior_class must identify one maintained behavior class")
    repository_class = value.get("repository_class")
    if repository_class in CLASSES and value.get("cycle") not in cycle_numbers(repository_class):
        errors.append("cycle must identify a private assignment for its repository class")
    revision = value.get("repository_revision")
    if not isinstance(revision, str) or re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", revision) is None:
        errors.append("repository_revision must be a full Git object identity")
    for field in ("work_user_task", "fresh_resume_user_task"):
        error = plain_user_task_error(value.get(field), field)
        if error:
            errors.append(error)
    errors.extend(
        work_scope_errors(
            value.get("work_scope"),
            value.get("repository_class"),
            revision,
            target_repository,
            verify_provenance,
        )
    )
    basis = value.get("evaluation_basis")
    errors.extend(evaluation_basis_errors(basis, behavior_class))
    errors.extend(behavior_review_errors(
        value.get("behavior_review"),
        behavior_class,
        candidate_revision=candidate_revision or git_head(ROOT),
        target_revision=revision if isinstance(revision, str) else None,
        candidate_root=candidate_root,
        target_repository=target_repository,
        verify_provenance=verify_provenance,
    ))
    if not evaluation_basis_errors(basis, behavior_class):
        errors.extend(
            naturalistic_prompt_errors(
                value.get("work_user_task"), value.get("fresh_resume_user_task"), basis
            )
        )
        if behavior_class == "hidden_user_owned_decision":
            errors.extend(
                hidden_prompt_static_disclosure_errors(
                    value.get("work_user_task"), value.get("fresh_resume_user_task")
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
    "materiality_review_operation",
    "behavior_class_evidence",
    "source_grounded_checkpoint_operation",
)
USER_DECISION_BLOCKER_CHECKS = (
    "material_question_candidate_lifecycle",
    "explicit_current_host_user_decision_operation",
)
SETUP_ACTIVATION_CHECK = "repository_scoped_session_start_activation"


def build_work_blocker_result(
    candidate_head: str,
    descriptor: dict[str, Any],
    descriptor_sha256: str,
    capture: CodexCapture,
    *,
    target_repository: Path | None = None,
) -> dict[str, Any]:
    descriptor_errors = cycle_descriptor_errors(
        descriptor,
        candidate_revision=candidate_head,
        target_repository=target_repository,
        verify_provenance=True,
    )
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

    activation_observed = capture.repository_scoped_activation_observed
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
    checkpoint_call = terminal_checkpoint_call(capture)
    baseline_call = selected_checkpoint_baseline_call(capture, checkpoint_call)
    meaningful_changes = meaningful_work_path_observations(capture)
    first_work_change = min(
        (observation.sequence for observation in meaningful_changes),
        default=None,
    )
    baseline_analysis_id = (
        baseline_call.result.get("analysis_snapshot_id")
        if baseline_call is not None
        else None
    )
    baseline_grounding_observed = (
        baseline_call is not None
        and first_work_change is not None
        and checkpoint_call is not None
        and nonempty_string(baseline_analysis_id)
        and baseline_call.arguments.get("project_id")
        == baseline_call.result.get("project_id")
        and checkpoint_call.arguments.get("project_id")
        == baseline_call.result.get("project_id")
        and any(
            entry.result.get("project_id") == baseline_call.result.get("project_id")
            and entry.completion_sequence < baseline_call.sequence
            for entry in project_entries
        )
        and any(
            goal.arguments.get("project_id") == baseline_call.result.get("project_id")
            and goal.result.get("project_id") == baseline_call.result.get("project_id")
            and goal.completion_sequence < baseline_call.sequence
            for goal in goal_calls
        )
        and baseline_call.completion_sequence < first_work_change
        and checkpoint_call.arguments.get("baseline_analysis_snapshot_id")
        == baseline_analysis_id
    )
    candidate_actions = {
        call.arguments.get("action")
        for call in capture.successful_calls("candidate_manage")
        if call.arguments.get("action") == call.result.get("action")
    }
    required_candidate_actions = {
        "submit_question_from_materiality",
        "attach_repository_research",
        "mark_research_ready",
        "promote_question",
    }
    material_question_lifecycle = (
        required_candidate_actions <= candidate_actions
        and bool(capture.successful_calls("inquiry_frontier"))
    )
    materiality_review_operation = any(
        call.arguments.get("action") == "record"
        and call.result.get("action") == "record"
        and nonempty_string(call.result.get("review_candidate_id"))
        and isinstance(call.result.get("workflow"), dict)
        and baseline_call is not None
        and baseline_call.completion_sequence < call.sequence
        and (first_work_change is None or call.completion_sequence < first_work_change)
        for call in capture.successful_calls("materiality_review")
    )
    observed = {
        "project_session_entry": bool(project_entries),
        "goal_context_operation": bool(goal_calls),
        "repository_baseline_operation": baseline_grounding_observed,
        "materiality_review_operation": materiality_review_operation,
        "behavior_class_evidence": descriptor.get("behavior_class") in BEHAVIOR_CLASSES,
        "material_question_candidate_lifecycle": material_question_lifecycle,
        "explicit_current_host_user_decision_operation": bool(
            capture.successful_calls("decision_record")
        ),
        "source_grounded_checkpoint_operation": bool(
            capture.successful_calls("checkpoint_record")
        ),
    }
    required_checks = (
        (*WORK_BLOCKER_CHECKS, *USER_DECISION_BLOCKER_CHECKS)
        if is_user_owned_behavior(descriptor.get("behavior_class"))
        else WORK_BLOCKER_CHECKS
    )
    failed_checks = (
        [SETUP_ACTIVATION_CHECK]
        if not activation_observed
        else [name for name in required_checks if not observed[name]]
    )
    if not failed_checks:
        raise ValueError(
            "completed work capture has no machine-observable terminal work blocker; use normal full qualification"
        )
    result = {
        "kind": "phase8_dogfood_blocker_result",
        "status": "failed",
        "classification": (
            "operator_environment_setup_failure"
            if not activation_observed
            else "product_work_session_blocker"
        ),
        "outcome": (
            "operator_environment_invalid"
            if not activation_observed
            else "campaign_stop"
        ),
        "candidate_head": candidate_head,
        "repository_class": descriptor["repository_class"],
        "cycle": descriptor["cycle"],
        "behavior_class": descriptor["behavior_class"],
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
        "classification",
        "outcome",
        "candidate_head",
        "repository_class",
        "cycle",
        "behavior_class",
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
    classification = result.get("classification")
    outcome = result.get("outcome")
    if (classification, outcome) not in {
        ("operator_environment_setup_failure", "operator_environment_invalid"),
        ("product_work_session_blocker", "campaign_stop"),
    }:
        raise ValueError("work-blocker result has an invalid failure classification")
    if (
        not re.fullmatch(r"[0-9a-f]{40}", result.get("candidate_head", ""))
        or result.get("repository_class") not in CLASSES
        or result.get("behavior_class") not in BEHAVIOR_CLASSES
        or result.get("cycle") not in cycle_numbers(result.get("repository_class"))
        or not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", result.get("repository_revision", ""))
        or not valid_capture_sha256(result.get("descriptor_sha256"))
        or not valid_capture_sha256(result.get("work_capture_sha256"))
    ):
        raise ValueError("work-blocker result identity is incomplete")
    if (
        not isinstance(failed_checks, list)
        or not failed_checks
        or any(check not in (*WORK_BLOCKER_CHECKS, *USER_DECISION_BLOCKER_CHECKS, SETUP_ACTIVATION_CHECK) for check in failed_checks)
        or (
            classification == "operator_environment_setup_failure"
            and failed_checks != [SETUP_ACTIVATION_CHECK]
        )
        or (
            classification == "product_work_session_blocker"
            and failed_checks
            != [
                name
                for name in (*WORK_BLOCKER_CHECKS, *USER_DECISION_BLOCKER_CHECKS)
                if name in failed_checks
            ]
        )
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
        target_repository=Path(args.repository).resolve(),
    )
    write_json(output_path, result)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 1


def decision_facts(
    work: CodexCapture | None,
    bundle: CanonicalBundle | None,
) -> tuple[
    bool,
    str | None,
    str | None,
    int | None,
    str | None,
    dict[str, dict[str, Any]],
]:
    if work is None or bundle is None:
        return False, None, None, None, None, {}
    calls = work.successful_calls("decision_record")
    if not calls:
        return False, None, None, None, None, {}
    evidence: dict[str, dict[str, Any]] = {}
    valid = True
    for call in calls:
        turn = work.turn_for_call(call)
        question_id = call.arguments.get("question_id")
        revision = call.arguments.get("question_revision")
        user_text = call.arguments.get("user_turn")
        source_id = call.result.get("user_response_source_id")
        source = (
            bundle.one("sources", id=source_id, project_id=bundle.project_id)
            if nonempty_string(source_id)
            else None
        )
        response = (
            bundle.one(
                "question_response_sources",
                project_id=bundle.project_id,
                question_id=question_id,
                question_revision=revision,
                source_id=source_id,
            )
            if nonempty_string(question_id) and isinstance(revision, int)
            else None
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
        decision_id = decisions[0].get("id") if len(decisions) == 1 else None
        witness = (
            bundle.one(
                "question_decision_history_witnesses",
                project_id=bundle.project_id,
                question_id=question_id,
                question_revision=revision,
                root_decision_id=decision_id,
                response_source_id=source_id,
                response_authority="current_host_user_turn",
            )
            if nonempty_string(decision_id)
            else None
        )
        question_revision_row = (
            bundle.one(
                "question_revisions",
                project_id=bundle.project_id,
                question_id=question_id,
                revision=revision,
            )
            if nonempty_string(question_id) and isinstance(revision, int)
            else None
        )
        material_scope = (
            decode_string_blob(question_revision_row.get("material_scope"))
            if question_revision_row is not None
            else None
        )
        call_valid = (
            turn is not None
            and nonempty_string(question_id)
            and isinstance(revision, int)
            and revision >= 1
            and nonempty_string(user_text)
            and turn.text == user_text
            and nonempty_string(source_id)
            and call.result.get("all_succeeded") is True
            and call.arguments.get("project_id") == bundle.project_id
            and source is not None
            and response is not None
            and witness is not None
            and nonempty_string(decision_id)
            and source.get("source_kind") == "current_host_user_turn"
            and source.get("locator") == turn.text
            and source.get("detail_one") == "codex"
            and source.get("detail_two") == work.session_id
            and source.get("actor_kind") == "user"
            and isinstance(material_scope, list)
        )
        valid &= bool(call_valid)
        if call_valid and nonempty_string(decision_id):
            if decision_id in evidence:
                valid = False
            evidence[str(decision_id)] = {
                "question_id": str(question_id),
                "question_revision": revision,
                "source_id": str(source_id),
                "material_scope": material_scope,
                "completion_sequence": call.completion_sequence,
            }
    ordered = sorted(
        evidence.items(), key=lambda item: item[1]["completion_sequence"]
    )
    if not valid or not ordered:
        return False, None, None, None, None, evidence
    primary_id, primary = ordered[0]
    return (
        True,
        primary_id,
        primary["question_id"],
        primary["question_revision"],
        primary["source_id"],
        evidence,
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


def expected_materiality_disposition(behavior_class: Any) -> str | None:
    if is_user_owned_behavior(behavior_class):
        return "unresolved_user_owned_outcome"
    return {
        "research_or_no_question": "repository_or_environment_fact",
        "delegated_implementation_choice": "delegated_implementation_choice",
        "exploratory_uncertainty": "exploratory_uncertainty",
        "learning_deliberation": "agent_owned_implementation_choice",
        "learning_routine_control": "agent_owned_implementation_choice",
    }.get(behavior_class)


MATERIALITY_DISPOSITIONS = {
    "repository_or_environment_fact",
    "settled_authority",
    "delegated_implementation_choice",
    "exploratory_uncertainty",
    "unresolved_user_owned_outcome",
    "agent_owned_implementation_choice",
}

DELEGATION_CONTRADICTORY_KINDS = {
    "accepted_contract",
    "applicable_decision",
    "agent_recommendation",
    "library_or_convention",
    "implementation_preference",
}


def scope_covers(declared_scope: list[str], affected_scope: list[str]) -> bool:
    return all(
        any(
            declared == affected
            or affected.startswith(f"{declared}/")
            for declared in declared_scope
        )
        for affected in affected_scope
    )


def current_goal_delegation_evidence_valid(
    dimension: dict[str, Any],
    *,
    goal_context_id: str | None,
    goal_source_id: str | None,
    goal_statement: str | None,
    frozen_task: str | None,
    repository_source_id: str | None,
) -> bool:
    basis = dimension["basis"]
    evidence = basis.get("explicit_delegation")
    evidence_scope = evidence.get("affected_scope") if isinstance(evidence, dict) else None
    affected_scope = dimension.get("affected_scope")
    statement = evidence.get("verbatim_statement") if isinstance(evidence, dict) else None
    source_ids = set(basis["source_ids"])
    additional_source_ids = (
        source_ids - {goal_source_id} if nonempty_string(goal_source_id) else source_ids
    )
    independent_research_valid = not additional_source_ids or (
        nonempty_string(repository_source_id)
        and additional_source_ids == {repository_source_id}
        and "research_evidence" in basis["kinds"]
        and bool(basis["research_basis"])
    )
    return (
        isinstance(evidence, dict)
        and set(evidence)
        == {
            "goal_context_id",
            "user_turn_source_id",
            "verbatim_statement",
            "affected_scope",
        }
        and nonempty_string(goal_context_id)
        and evidence.get("goal_context_id") == goal_context_id
        and nonempty_string(goal_source_id)
        and evidence.get("user_turn_source_id") == goal_source_id
        and goal_source_id in basis["source_ids"]
        and independent_research_valid
        and nonempty_string(statement)
        and nonempty_string(goal_statement)
        and statement in goal_statement
        and nonempty_string(frozen_task)
        and statement in frozen_task
        and isinstance(evidence_scope, list)
        and bool(evidence_scope)
        and all(nonempty_string(item) for item in evidence_scope)
        and isinstance(affected_scope, list)
        and bool(affected_scope)
        and scope_covers(evidence_scope, affected_scope)
        and not basis["decision_ids"]
        and not basis["contract_basis"]
        and not (set(basis["kinds"]) & DELEGATION_CONTRADICTORY_KINDS)
    )


def indexed_materiality_dimensions(value: Any) -> dict[str, dict[str, Any]] | None:
    if not isinstance(value, list) or not value:
        return None
    indexed: dict[str, dict[str, Any]] = {}
    for dimension in value:
        dimension_id = dimension.get("dimension_id") if isinstance(dimension, dict) else None
        basis = dimension.get("basis") if isinstance(dimension, dict) else None
        source_ids = basis.get("source_ids") if isinstance(basis, dict) else None
        explicit_delegation = (
            basis.get("explicit_delegation") if isinstance(basis, dict) else None
        )
        discovered_choice_ids = (
            dimension.get("discovered_choice_ids") if isinstance(dimension, dict) else None
        )
        learning_value = dimension.get("learning_value") if isinstance(dimension, dict) else None
        if (
            not isinstance(dimension, dict)
            or not nonempty_string(dimension_id)
            or dimension_id in indexed
            or not nonempty_string(dimension.get("summary"))
            or not isinstance(dimension.get("affected_scope"), list)
            or not dimension["affected_scope"]
            or not all(nonempty_string(item) for item in dimension["affected_scope"])
            or not isinstance(dimension.get("material_consequences"), list)
            or not dimension["material_consequences"]
            or not all(nonempty_string(item) for item in dimension["material_consequences"])
            or not isinstance(dimension.get("observable_signals"), list)
            or not dimension["observable_signals"]
            or dimension.get("disposition") not in MATERIALITY_DISPOSITIONS
            or not isinstance(discovered_choice_ids, list)
            or not discovered_choice_ids
            or not all(nonempty_string(choice_id) for choice_id in discovered_choice_ids)
            or len(set(discovered_choice_ids)) != len(discovered_choice_ids)
            or not isinstance(learning_value, dict)
            or learning_value.get("state") not in {"routine", "deliberation_worthy"}
            or not nonempty_string(learning_value.get("rationale"))
            or (
                learning_value.get("state") == "deliberation_worthy"
                and not all(
                    isinstance(learning_value.get(field), list)
                    and learning_value[field]
                    and all(nonempty_string(item) for item in learning_value[field])
                    for field in (
                        "consequence_significance",
                        "transferable_principles",
                        "non_obvious_trade_offs",
                    )
                )
            )
            or not isinstance(basis, dict)
            or not isinstance(basis.get("kinds"), list)
            or not basis["kinds"]
            or len(set(basis["kinds"])) != len(basis["kinds"])
            or not nonempty_string(basis.get("summary"))
            or not isinstance(source_ids, list)
            or not source_ids
            or not all(nonempty_string(source_id) for source_id in source_ids)
            or len(set(source_ids)) != len(source_ids)
            or not isinstance(basis.get("contract_basis"), list)
            or not isinstance(basis.get("decision_ids"), list)
            or not isinstance(basis.get("research_basis"), list)
            or (
                explicit_delegation is not None
                and (
                    not isinstance(explicit_delegation, dict)
                    or not nonempty_string(
                        explicit_delegation.get("goal_context_id")
                    )
                    or not nonempty_string(
                        explicit_delegation.get("user_turn_source_id")
                    )
                    or not nonempty_string(
                        explicit_delegation.get("verbatim_statement")
                    )
                    or not isinstance(
                        explicit_delegation.get("affected_scope"), list
                    )
                    or not explicit_delegation["affected_scope"]
                    or not all(
                        nonempty_string(item)
                        for item in explicit_delegation["affected_scope"]
                    )
                )
            )
        ):
            return None
        indexed[str(dimension_id)] = dimension
    return indexed


def indexed_engineering_choices(value: Any) -> dict[str, dict[str, Any]] | None:
    if not isinstance(value, list) or not value:
        return None
    indexed: dict[str, dict[str, Any]] = {}
    for choice in value:
        choice_id = choice.get("choice_id") if isinstance(choice, dict) else None
        relationship = choice.get("relationship") if isinstance(choice, dict) else None
        alternatives = choice.get("alternatives") if isinstance(choice, dict) else None
        if (
            not isinstance(choice, dict)
            or not nonempty_string(choice_id)
            or choice_id in indexed
            or not nonempty_string(choice.get("summary"))
            or not isinstance(choice.get("affected_scope"), list)
            or not choice["affected_scope"]
            or not all(nonempty_string(item) for item in choice["affected_scope"])
            or not isinstance(alternatives, list)
            or len(alternatives) < 2
            or any(
                not isinstance(alternative, dict)
                or not nonempty_string(alternative.get("alternative_id"))
                or not nonempty_string(alternative.get("summary"))
                or not isinstance(alternative.get("technical_consequences"), list)
                or not alternative["technical_consequences"]
                or not all(nonempty_string(item) for item in alternative["technical_consequences"])
                for alternative in alternatives
            )
            or len({alternative["alternative_id"] for alternative in alternatives}) != len(alternatives)
            or not isinstance(choice.get("technical_consequences"), list)
            or not choice["technical_consequences"]
            or not isinstance(choice.get("source_ids"), list)
            or not choice["source_ids"]
            or not isinstance(choice.get("effect_categories"), list)
            or not choice["effect_categories"]
            or not isinstance(relationship, dict)
            or relationship.get("state") not in {"independent", "coupled"}
            or choice.get("evidence_state") != "sufficient"
        ):
            return None
        indexed[str(choice_id)] = choice
    for choice in indexed.values():
        relationship = choice["relationship"]
        if relationship["state"] == "coupled" and (
            not isinstance(relationship.get("choice_ids"), list)
            or not relationship["choice_ids"]
            or not set(relationship["choice_ids"]) <= set(indexed)
            or not nonempty_string(relationship.get("rationale"))
        ):
            return None
    return indexed


def engineering_choice_discovery_facts(
    work: CodexCapture,
    bundle: CanonicalBundle,
    goal_context_id: str,
    baseline_call: ToolCall,
    review_call: ToolCall,
) -> tuple[bool, dict[str, Any]]:
    baseline_id = baseline_call.result.get("analysis_snapshot_id")
    calls = [
        call
        for call in work.successful_calls("engineering_choice_discovery")
        if call.arguments.get("project_id") == bundle.project_id
        and call.arguments.get("goal_context_id") == goal_context_id
        and call.arguments.get("baseline_analysis_snapshot_id") == baseline_id
    ]
    discovery = calls[0] if len(calls) == 1 else None
    choices = indexed_engineering_choices(discovery.arguments.get("choices")) if discovery else None
    dimensions = indexed_materiality_dimensions(review_call.arguments.get("dimensions"))
    discovered_id = discovery.result.get("discovery_candidate_id") if discovery else None
    referenced_ids = {
        choice_id
        for dimension in (dimensions or {}).values()
        for choice_id in dimension["discovered_choice_ids"]
    }
    valid = bool(
        discovery
        and choices
        and dimensions
        and baseline_call.completion_sequence < discovery.sequence
        and discovery.completion_sequence < review_call.sequence
        and nonempty_string(discovered_id)
        and review_call.arguments.get("engineering_choice_discovery_candidate_id")
        == discovered_id
        and discovery.result.get("goal_context_id") == goal_context_id
        and discovery.result.get("baseline_analysis_snapshot_id") == baseline_id
        and set(choices) == referenced_ids
        and nonempty_string(baseline_call.result.get("repository_source_id"))
        and all(
            choice.get("source_ids")
            == [baseline_call.result.get("repository_source_id")]
            for choice in choices.values()
        )
        and all(
            set(dimension["discovered_choice_ids"]) <= set(choices)
            for dimension in dimensions.values()
        )
    )
    return valid, {
        "matching_discovery_count": len(calls),
        "discovery_candidate_id": discovered_id,
        "choice_ids": sorted(choices or {}),
        "materiality_referenced_choice_ids": sorted(referenced_ids),
        "effect_categories": sorted({
            effect
            for choice in (choices or {}).values()
            for effect in choice.get("effect_categories", [])
        }),
        "relationship_states": sorted({
            choice.get("relationship", {}).get("state")
            for choice in (choices or {}).values()
        }),
    }


def materiality_dimension_authority_valid(
    dimension: dict[str, Any],
    *,
    goal_context_id: str | None,
    goal_source_id: str | None,
    goal_statement: str | None,
    frozen_task: str | None,
    repository_source_id: str | None,
    decision_evidence: dict[str, dict[str, Any]],
    require_current_goal_delegation: bool,
) -> bool:
    disposition = dimension.get("disposition")
    basis = dimension["basis"]
    kinds = set(basis["kinds"])
    source_ids = set(basis["source_ids"])
    decision_ids = basis["decision_ids"]
    if disposition != "delegated_implementation_choice" and basis.get(
        "explicit_delegation"
    ) is not None:
        return False
    if disposition == "repository_or_environment_fact":
        return (
            "repository_or_environment_fact" in kinds
            and nonempty_string(repository_source_id)
            and repository_source_id in source_ids
        )
    if disposition == "settled_authority":
        accepted = "accepted_contract" in kinds and bool(basis["contract_basis"])
        decided = "applicable_decision" in kinds and bool(decision_ids)
        return accepted or decided
    if disposition == "delegated_implementation_choice":
        if "explicit_delegation" not in kinds:
            return False
        current_goal = (
            current_goal_delegation_evidence_valid(
                dimension,
                goal_context_id=goal_context_id,
                goal_source_id=goal_source_id,
                goal_statement=goal_statement,
                frozen_task=frozen_task,
                repository_source_id=repository_source_id,
            )
        )
        decision_path = (
            basis.get("explicit_delegation") is None
            and not basis["contract_basis"]
            and not (
                kinds
                & (DELEGATION_CONTRADICTORY_KINDS - {"applicable_decision"})
            )
            and bool(decision_ids)
            and all(
                decision_id in decision_evidence
                and f"work-authority:{dimension['dimension_id']}"
                in decision_evidence[decision_id]["material_scope"]
                for decision_id in decision_ids
            )
        )
        return current_goal if require_current_goal_delegation else current_goal or decision_path
    if disposition == "exploratory_uncertainty":
        exploratory = dimension.get("exploratory_disposition")
        return (
            exploratory
            in {"resolved_by_research", "deferred_with_revisit"}
            and bool(basis["research_basis"])
            and (
                "research_evidence" in kinds
                if exploratory == "resolved_by_research"
                else "defer_or_revisit_basis" in kinds
            )
        )
    if disposition == "agent_owned_implementation_choice":
        return (
            "implementation_preference" in kinds
            and nonempty_string(repository_source_id)
            and repository_source_id in source_ids
            and not decision_ids
        )
    return (
        disposition == "unresolved_user_owned_outcome"
        and nonempty_string(repository_source_id)
        and repository_source_id in source_ids
    )


def resolved_user_owned_dimensions_valid(
    dimensions: dict[str, dict[str, Any]],
    dimension_ids: set[str],
    decision_evidence: dict[str, dict[str, Any]],
    *,
    revision_sequence: int | None = None,
) -> bool:
    return (
        bool(dimension_ids)
        and dimension_ids <= set(dimensions)
        and all(
            nonempty_string(dimensions[dimension_id].get("resolution_decision_id"))
            and dimensions[dimension_id]["resolution_decision_id"]
            in dimensions[dimension_id]["basis"]["decision_ids"]
            and dimensions[dimension_id]["resolution_decision_id"] in decision_evidence
            and f"work-authority:{dimension_id}"
            in decision_evidence[dimensions[dimension_id]["resolution_decision_id"]][
                "material_scope"
            ]
            and (
                revision_sequence is None
                or decision_evidence[dimensions[dimension_id]["resolution_decision_id"]][
                    "completion_sequence"
                ]
                < revision_sequence
            )
            for dimension_id in dimension_ids
        )
    )


def materiality_review_facts(
    work: CodexCapture | None,
    bundle: CanonicalBundle | None,
    behavior_class: Any,
    goal_context_id: str | None,
    goal_statement: str | None,
    frozen_task: str | None,
    baseline_call: ToolCall | None,
    first_write_sequence: int | None,
    goal_source_id: str | None,
    decision_evidence: dict[str, dict[str, Any]],
    *,
    resumed: bool = False,
) -> tuple[bool, str | None, str | None, dict[str, Any]]:
    expected = expected_materiality_disposition(behavior_class)
    if (
        work is None
        or bundle is None
        or expected is None
        or not nonempty_string(goal_context_id)
        or baseline_call is None
        or first_write_sequence is None
    ):
        return False, None, None, {}
    baseline_id = baseline_call.result.get("analysis_snapshot_id")
    records = [
        call
        for call in work.successful_calls("materiality_review")
        if call.arguments.get("action") == "record"
        and call.arguments.get("project_id") == bundle.project_id
        and call.arguments.get("goal_context_id") == goal_context_id
        and call.arguments.get("baseline_analysis_snapshot_id") == baseline_id
    ]
    if len(records) != 1:
        return False, None, None, {"matching_record_count": len(records)}
    record = records[0]
    review_id = record.result.get("review_candidate_id")
    dimensions = indexed_materiality_dimensions(record.arguments.get("dimensions"))
    discovery_ok, discovery_basis = engineering_choice_discovery_facts(
        work, bundle, goal_context_id, baseline_call, record
    )
    learning_participation = record.arguments.get("learning_participation")
    learning_active = behavior_class in {
        "learning_deliberation",
        "learning_routine_control",
    }
    participation_ok = (
        isinstance(learning_participation, dict)
        and (
            learning_participation == {"state": "inactive"}
            if not learning_active
            else learning_participation.get("state") == "active"
            and learning_participation.get("user_turn_source_id") == goal_source_id
            and nonempty_string(learning_participation.get("verbatim_statement"))
            and learning_participation["verbatim_statement"] in (goal_statement or "")
            and learning_participation["verbatim_statement"] in (frozen_task or "")
        )
    )
    repository_source_id = baseline_call.result.get("repository_source_id")
    workflow = record.result.get("workflow")
    dimension_ids = set(dimensions) if dimensions is not None else set()
    relevant_ids = (
        [
            dimension_id
            for dimension_id, dimension in dimensions.items()
            if dimension.get("disposition") == expected
        ]
        if dimensions is not None
        else []
    )
    user_owned_ids = (
        {
            dimension_id
            for dimension_id, dimension in dimensions.items()
            if dimension.get("disposition") == "unresolved_user_owned_outcome"
        }
        if dimensions is not None
        else set()
    )
    primary_dimension_id = relevant_ids[0] if relevant_ids else None
    dimension_authority = bool(dimensions) and all(
        materiality_dimension_authority_valid(
            dimension,
            goal_context_id=goal_context_id,
            goal_source_id=goal_source_id,
            goal_statement=goal_statement,
            frozen_task=frozen_task,
            repository_source_id=(
                str(repository_source_id)
                if nonempty_string(repository_source_id)
                else None
            ),
            decision_evidence=decision_evidence,
            require_current_goal_delegation=(
                behavior_class == "delegated_implementation_choice"
                and dimension.get("disposition")
                == "delegated_implementation_choice"
            ),
        )
        for dimension in dimensions.values()
    )
    common = (
        baseline_call.completion_sequence < record.sequence
        and record.completion_sequence < first_write_sequence
        and nonempty_string(review_id)
        and record.result.get("goal_context_id") == goal_context_id
        and record.result.get("baseline_analysis_snapshot_id") == baseline_id
        and record.result.get("review_revision") == 1
        and nonempty_string(record.result.get("review_analysis_snapshot_id"))
        and dimensions is not None
        and bool(relevant_ids)
        and discovery_ok
        and participation_ok
        and all(
            dimension["learning_value"]["state"]
            == (
                "deliberation_worthy"
                if behavior_class == "learning_deliberation"
                and dimension.get("disposition") == expected
                else "routine"
            )
            for dimension in dimensions.values()
        )
        and dimension_authority
        and isinstance(workflow, dict)
        and workflow.get("satisfied_basis_identities") is not None
        and workflow.get("unresolved_requirements") is not None
    )
    if resumed:
        resolved = (
            resolved_user_owned_dimensions_valid(
                dimensions or {}, user_owned_ids, decision_evidence
            )
            if user_owned_ids
            else True
        )
        valid = (
            common
            and resolved
            and (is_user_owned_behavior(behavior_class) or not user_owned_ids)
            and workflow.get("stage") == "ready_for_work"
            and workflow.get("disposition") == "ready_for_work"
            and workflow.get("blocks_ordinary_work") is False
        )
    elif is_user_owned_behavior(behavior_class):
        revisions = [
            call
            for call in work.successful_calls("materiality_review")
            if call.arguments.get("action") == "revise"
            and call.arguments.get("project_id") == bundle.project_id
            and call.arguments.get("review_candidate_id") == review_id
        ]
        revisions.sort(key=lambda call: call.sequence)
        revision_chain = [
            (revision, indexed_materiality_dimensions(revision.arguments.get("dimensions")))
            for revision in revisions
        ]
        final_revision = revision_chain[-1][0] if revision_chain else None
        final_dimensions = revision_chain[-1][1] if revision_chain else None
        revised_workflow = (
            final_revision.result.get("workflow") if final_revision is not None else None
        )
        chain_preserves_dimensions = bool(revision_chain) and all(
            revised is not None and set(revised) == dimension_ids
            for _, revised in revision_chain
        )
        chain_preserves_review_identity = bool(revision_chain) and all(
            revision.result.get("review_candidate_id") == review_id
            and revision.result.get("goal_context_id") == goal_context_id
            and revision.result.get("baseline_analysis_snapshot_id") == baseline_id
            and revision.result.get("review_revision") == expected_revision
            and nonempty_string(
                revision.result.get("review_analysis_snapshot_id")
            )
            for expected_revision, (revision, _) in enumerate(
                revision_chain, start=2
            )
        )
        resolved = bool(final_dimensions) and final_revision is not None and (
            resolved_user_owned_dimensions_valid(
                final_dimensions,
                user_owned_ids,
                decision_evidence,
                revision_sequence=final_revision.sequence,
            )
        )
        unresolved_workflow_ids = {
            requirement.get("dimension_id")
            for requirement in workflow.get("unresolved_requirements", [])
            if isinstance(requirement, dict)
        }
        valid = (
            common
            and bool(user_owned_ids)
            and all(
                dimensions[dimension_id].get("resolution_decision_id") is None
                for dimension_id in user_owned_ids
            )
            and workflow.get("stage") == "question_candidate"
            and workflow.get("blocks_ordinary_work") is True
            and workflow.get("required_next_action")
            == {"tool": "candidate_manage", "action": "submit_question_from_materiality"}
            and unresolved_workflow_ids == user_owned_ids
            and chain_preserves_dimensions
            and chain_preserves_review_identity
            and resolved
            and final_revision is not None
            and final_revision.completion_sequence < first_write_sequence
            and isinstance(revised_workflow, dict)
            and revised_workflow.get("stage") == "ready_for_work"
            and revised_workflow.get("blocks_ordinary_work") is False
        )
    else:
        learning_deliberation_expected = behavior_class == "learning_deliberation"
        valid = (
            common
            and not user_owned_ids
            and workflow.get("stage")
            == ("learning_deliberation" if learning_deliberation_expected else "ready_for_work")
            and workflow.get("blocks_ordinary_work") is learning_deliberation_expected
            and not work.calls("candidate_manage")
            and not work.calls("inquiry_frontier")
            and not work.calls("decision_record")
        )
    return bool(valid), str(review_id) if nonempty_string(review_id) else None, str(primary_dimension_id) if nonempty_string(primary_dimension_id) else None, {
        "record_sequence": record.sequence,
        "review_candidate_id": review_id,
        "dimension_ids": sorted(dimension_ids),
        "relevant_dimension_ids": relevant_ids,
        "user_owned_dimension_ids": sorted(user_owned_ids),
        "dimension_correlation": "dimension_id",
        "disposition": expected,
        "explicit_delegation": (
            dimensions[primary_dimension_id]["basis"].get("explicit_delegation")
            if dimensions is not None and primary_dimension_id is not None
            else None
        ),
        "pre_write": record.completion_sequence < first_write_sequence,
        "resumed": resumed,
        "learning_participation": learning_participation,
        "engineering_choice_discovery": discovery_basis,
    }


def learning_deliberation_facts(
    work: CodexCapture | None,
    bundle: CanonicalBundle | None,
    behavior_class: Any,
    review_candidate_id: str | None,
    dimension_id: str | None,
    first_write_sequence: int | None,
    materiality_basis: dict[str, Any],
) -> tuple[bool, bool, bool, dict[str, Any]]:
    if work is None or bundle is None or first_write_sequence is None:
        return False, False, False, {}
    calls = work.successful_calls("learning_deliberation")
    participation = materiality_basis.get("learning_participation", {})
    participation_active = participation.get("state") == "active"
    expected_active = behavior_class in {
        "learning_deliberation",
        "learning_routine_control",
    }
    participation_ok = participation_active == expected_active
    if behavior_class != "learning_deliberation":
        precision_ok = not calls
        return participation_ok, precision_ok, precision_ok, {
            "participation_state": participation.get("state"),
            "learning_call_count": len(calls),
            "expected_deliberation": False,
        }
    actions = [call.arguments.get("action") for call in calls]
    by_action = {
        action: [call for call in calls if call.arguments.get("action") == action]
        for action in ("begin", "respond_select", "feedback", "complete")
    }
    singular = all(len(by_action[action]) == 1 for action in by_action)
    if not singular:
        return participation_ok, False, False, {
            "participation_state": participation.get("state"),
            "actions": actions,
            "expected_deliberation": True,
        }
    begin, response, feedback, complete = (
        by_action["begin"][0],
        by_action["respond_select"][0],
        by_action["feedback"][0],
        by_action["complete"][0],
    )
    deliberation_id = begin.result.get("deliberation_candidate_id")
    response_text = response.arguments.get("user_turn")
    matching_user_turns = [
        turn
        for turn in work.user_turns
        if turn.text == response_text and turn.sequence < response.sequence
    ]
    all_non_decision = all(
        call.result.get("interaction_kind") == "learning_participation"
        and call.result.get("canonical_decision") is False
        for call in calls
    )
    no_pre_response_recommendation = (
        not any("recommendation" in key for key in begin.arguments)
        and not any("recommendation" in key for key in begin.result)
        and not begin.result.get("rounds")
    )
    ordered = (
        begin.sequence < response.sequence < feedback.sequence < complete.sequence
        and complete.completion_sequence < first_write_sequence
        and begin.arguments.get("review_candidate_id") == review_candidate_id
        and begin.arguments.get("dimension_id") == dimension_id
        and nonempty_string(deliberation_id)
        and all(
            call.arguments.get("deliberation_candidate_id") == deliberation_id
            and call.result.get("deliberation_candidate_id") == deliberation_id
            for call in (response, feedback, complete)
        )
        and begin.result.get("state", {}).get("state") == "awaiting_initial_response"
        and response.result.get("state", {}).get("state") == "awaiting_agent_feedback"
        and feedback.result.get("state", {}).get("state") == "feedback_provided"
        and complete.result.get("state", {}).get("state") == "completed"
        and bool(matching_user_turns)
        and nonempty_string(response.arguments.get("user_rationale"))
        and isinstance(response.arguments.get("selections"), list)
        and bool(response.arguments["selections"])
        and no_pre_response_recommendation
        and nonempty_string(feedback.arguments.get("feedback"))
        and complete.result.get("workflow", {}).get("stage") == "ready_for_work"
        and complete.result.get("workflow", {}).get("blocks_ordinary_work") is False
    )
    non_decision_ok = (
        all_non_decision
        and not work.calls("decision_record")
        and not [row for row in bundle.rows("decisions") if row.get("project_id") == bundle.project_id]
    )
    return participation_ok, ordered, non_decision_ok, {
        "participation_state": participation.get("state"),
        "expected_deliberation": True,
        "deliberation_candidate_id": deliberation_id,
        "actions": actions,
        "current_host_response_count": len(matching_user_turns),
        "pre_response_recommendation_absent": no_pre_response_recommendation,
        "terminal_state": complete.result.get("state", {}).get("state"),
        "ready_for_work_after_terminal": complete.result.get("workflow", {}).get("stage")
        == "ready_for_work",
        "canonical_decision_count": len([
            row for row in bundle.rows("decisions") if row.get("project_id") == bundle.project_id
        ]),
    }


def learning_recall_facts(
    resume: CodexCapture | None,
    behavior_class: Any,
) -> tuple[bool, dict[str, Any]]:
    if resume is None:
        return False, {}
    recalls = resume.successful_calls("recall")
    if len(recalls) != 1:
        return False, {"matching_recall_count": len(recalls)}
    recall = recalls[0]
    context = recall.result.get("learning_context")
    health = recall.result.get("learning_context_health", {})
    if behavior_class == "learning_deliberation":
        matching = [
            item
            for item in context or []
            if item.get("learning_deliberation", {}).get("state", {}).get("state")
            == "completed"
            and item.get("learning_deliberation", {}).get("canonical_decision") is False
            and item.get("learning_deliberation", {}).get("interaction_kind")
            == "learning_participation"
        ]
        valid = isinstance(context, list) and len(matching) == 1
    else:
        matching = []
        valid = context == []
    valid = valid and health.get("state") == "available"
    return bool(valid), {
        "learning_context_count": len(context) if isinstance(context, list) else None,
        "matching_completed_learning_count": len(matching),
        "health": health,
        "canonical_decision": False if matching else None,
    }


def question_review_facts(
    work: CodexCapture | None,
    bundle: CanonicalBundle | None,
    question_id: str | None,
    question_revision: int | None,
    evaluation_basis: Any,
    baseline_call: ToolCall | None,
    review_candidate_id: str | None,
    dimension_id: str | None,
) -> tuple[bool, dict[str, Any]]:
    if (
        work is None
        or bundle is None
        or not nonempty_string(question_id)
        or not isinstance(question_revision, int)
        or not isinstance(evaluation_basis, dict)
        or baseline_call is None
    ):
        return False, {}
    decision_calls = [
        call
        for call in work.successful_calls("decision_record")
        if call.arguments.get("question_id") == question_id
        and call.arguments.get("question_revision") == question_revision
    ]
    decision_call = decision_calls[0] if len(decision_calls) == 1 else None
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
    submit_calls = [
        call
        for call in candidate_calls
        if call.arguments.get("action") == "submit_question_from_materiality"
        and call.result.get("action") == "submit_question_from_materiality"
        and call.arguments.get("review_candidate_id") == review_candidate_id
        and call.arguments.get("dimension_id") == dimension_id
    ]
    submit_call = submit_calls[0] if len(submit_calls) == 1 else None
    candidate_id = (
        submit_call.result.get("candidate_id") if submit_call is not None else None
    )
    candidate_lifecycle_calls: list[ToolCall | None] = [submit_call]
    for action in (
        "attach_repository_research",
        "mark_research_ready",
        "promote_question",
    ):
        calls = [
            call
            for call in candidate_calls
            if call.arguments.get("action") == action
            and call.result.get("action") == action
            and call.arguments.get("candidate_id") == candidate_id
        ]
        candidate_lifecycle_calls.append(calls[0] if len(calls) == 1 else None)
    submit_call, research_call, ready_call, promote_call = candidate_lifecycle_calls
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
        and submit_call.arguments.get("review_candidate_id") == review_candidate_id
        and submit_call.result.get("review_candidate_id") == review_candidate_id
        and submit_call.arguments.get("dimension_id") == dimension_id
        and submit_call.result.get("dimension_id") == dimension_id
        and submit_call.arguments.get("research_state") == "research_required"
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
    alternatives_have_real_consequences = bool(alternatives) and all(
        nonempty_string(alternative.get("label"))
        and nonempty_string(alternative.get("consequence"))
        for alternative in alternatives
    )
    basis = {
        "canonical_question_present": revision is not None,
        "canonical_materiality": revision.get("materiality") if revision else None,
        "repository_facts_observed_count": len(established_facts or []),
        "observed_alternative_count": len(alternatives or []),
        "observed_question_basis_bytes": len(observed_question_text.encode("utf-8")),
        "alternatives_have_real_consequences": alternatives_have_real_consequences,
        "candidate_lifecycle_observed": candidate_lifecycle_ok,
        "ask_user_invariants": {
            "material_consequence": alternatives_have_real_consequences,
            "user_ownership": True,
            "not_repository_or_environment_fact": True,
            "not_settled_by_accepted_decision": True,
            "not_delegated": True,
            "current_relevance": nonempty_string(revision.get("why_it_matters_now")) if revision else False,
            "source_grounding": bool(established_facts) and candidate_lifecycle_ok,
            "real_consequence_between_alternatives": alternatives_have_real_consequences,
        },
        "exact_preferred_expression_required": False,
    }
    valid = (
        revision is not None
        and revision.get("materiality") == "material"
        and nonempty_string(revision.get("prompt_basis"))
        and nonempty_string(revision.get("why_it_matters_now"))
        and established_facts is not None
        and alternatives is not None
        and f"work-authority:{dimension_id}" in (material_scope or [])
        and len(alternatives) >= 2
        and alternatives_have_real_consequences
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
    used_command_occurrences: set[tuple[int, int]] = set()
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
        if set(claim) - {
            "state",
            "command_label",
            "command_invocation",
            "exit_code",
            "termination",
            "outcome",
        }:
            return False
        label = claim.get("command_label")
        invocation = claim.get("command_invocation")
        exit_code = claim.get("exit_code")
        termination = claim.get("termination")
        outcome = claim.get("outcome")
        if (
            not nonempty_string(label)
            or not nonempty_string(invocation)
            or not nonempty_string(outcome)
        ):
            return False
        invocation_fingerprint = "sha256:" + hashlib.sha256(
            invocation.encode("utf-8")
        ).hexdigest()
        commands = [
            command
            for command in work.commands
            if command.completion_sequence < call.sequence
            and (command.sequence, command.group_index) not in used_command_occurrences
            and command_invocation_fingerprint(command) == invocation_fingerprint
            and command.exit_code == exit_code
            and command.termination == termination
        ]
        if not commands:
            return False
        command = min(commands, key=lambda value: (value.sequence, value.group_index))
        used_command_occurrences.add((command.sequence, command.group_index))
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
            or source.get("detail_one") != invocation_fingerprint
            or source.get("detail_two") is not None
            or source.get("exit_code") != exit_code
            or source.get("termination") != termination
            or source.get("actor_kind") != "command"
            or source.get("observer_kind") != "agent"
        ):
            return False
        executed_ids.append(str(source_id))
    return returned_ids == executed_ids


def command_invocation_fingerprint(command: Any) -> str | None:
    parsed = command.parsed_command
    invocation = parsed.get("cmd") if isinstance(parsed, dict) else None
    if not isinstance(invocation, str) or not invocation:
        return None
    encoded = invocation.encode("utf-8")
    if len(encoded) > 16_384:
        return None
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def checkpoint_pre_existing_dirty_paths(call: ToolCall | None) -> list[str] | None:
    if call is None or call.outcome != "succeeded":
        return None
    paths = call.result.get("pre_existing_dirty_paths")
    if not isinstance(paths, list) or len(paths) > 256:
        return None
    if not all(isinstance(path, str) for path in paths):
        return None
    if paths != sorted(set(paths)):
        return None
    for path in paths:
        candidate = Path(path)
        if (
            not path
            or candidate.is_absolute()
            or ".." in candidate.parts
            or path != candidate.as_posix()
            or any(part in {".git", ".local"} for part in candidate.parts)
        ):
            return None
    return paths


def meaningful_work_path_observations(work: CodexCapture | None) -> list[Any]:
    if work is None:
        return []
    return [
        observation
        for observation in work.path_observations
        if any(
            not looks_like_synthetic_marker(path)
            and Path(path).suffix.lower() not in {".txt", ".marker"}
            and not generated_repository_path(path)
            for path in observation.paths
        )
    ]


def terminal_checkpoint_call(work: CodexCapture | None) -> ToolCall | None:
    """Select the latest observed work Checkpoint candidate without fallback."""
    if work is None:
        return None
    calls = work.calls("checkpoint_record")
    return max(calls, key=lambda call: call.sequence) if calls else None


def checkpoint_facts(
    work: CodexCapture | None,
    bundle: CanonicalBundle | None,
    decision_ids: list[str],
    goal_context_id: str | None,
    goal_source_id: str | None,
    baseline_analysis_id: str | None,
    goal_statement: str | None,
) -> tuple[bool, bool, bool, str | None, list[str], str | None]:
    call = terminal_checkpoint_call(work)
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
    decision_links = all(
        bundle.one(
            "checkpoint_decisions",
            project_id=bundle.project_id,
            checkpoint_id=checkpoint_id,
            decision_id=decision_id,
        )
        is not None
        for decision_id in decision_ids
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
    meaningful_changes = meaningful_work_path_observations(work)
    terminal_after_last_meaningful_change = (
        bool(meaningful_changes)
        and max(item.sequence for item in meaningful_changes) < call.sequence
    )
    valid = (
        call.outcome == "succeeded"
        and terminal_after_last_meaningful_change
        and bounded_paths is not None
        and set(bounded_paths) == set(observed_paths)
        and set(bounded_paths) == source_paths
        and call.result.get("changed_paths") == bounded_paths
        and supported is not None
        and decision_links
        and call.arguments.get("project_id") == bundle.project_id
        and call.arguments.get("baseline_analysis_snapshot_id") == baseline_analysis_id
        and call.result.get("baseline_analysis_snapshot_id") == baseline_analysis_id
        and goal_linked
        and isinstance(applied, list)
        and set(decision_ids) <= set(applied)
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
    candidate_revision: str | None = None,
    target_repository: Path | None = None,
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
    descriptor_errors = cycle_descriptor_errors(
        raw,
        candidate_revision=candidate_revision or git_head(ROOT),
        target_repository=target_repository,
        verify_provenance=True,
    )
    evidence = raw.get("evidence") if isinstance(raw.get("evidence"), dict) else {}
    captures = evidence.get("captures") if isinstance(evidence.get("captures"), dict) else {}
    work_reference = captures.get("work")
    resume_reference = captures.get("resume")
    bundle_reference = evidence.get("canonical_bundle")
    work_user_task = raw.get("work_user_task")
    resume_user_task = raw.get("fresh_resume_user_task")
    behavior_class = raw.get("behavior_class")
    evaluation_basis = raw.get("evaluation_basis")
    behavior_review = raw.get("behavior_review")
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

    support_checks, support_basis = campaign_support_evidence(
        evidence,
        evidence_directory,
        bundle,
        project_id=bundle.project_id if bundle is not None else None,
        candidate_revision=candidate_revision or git_head(ROOT),
        kind=kind,
        cycle=cycle,
    )
    support_references_present = {
        name: isinstance(evidence.get(reference), dict)
        for name, reference in {
            "canonical_bundle_and_provenance": "canonical_bundle",
            "generated_document_outputs": "generated_documents",
            "static_viewer_snapshot": "viewer_snapshot",
            "bounded_runtime_and_activation_evidence": "runtime_summary",
        }.items()
    }
    support_references_present["bounded_runtime_and_activation_evidence"] &= isinstance(
        evidence.get("activation_summary"), dict
    )

    (
        decision_ok,
        decision_id,
        question_id,
        question_revision,
        user_source_id,
        decision_evidence,
    ) = decision_facts(work_capture, bundle)
    goal_ok, goal_context_id, goal_source_id, goal_statement = goal_facts(
        work_capture, bundle, work_user_task
    )
    checkpoint_call = terminal_checkpoint_call(work_capture)
    baseline_call = selected_checkpoint_baseline_call(work_capture, checkpoint_call)
    baseline_analysis_id = (
        baseline_call.result.get("analysis_snapshot_id") if baseline_call is not None else None
    )
    first_work_change = min(
        (item.sequence for item in meaningful_work_path_observations(work_capture)),
        default=None,
    ) if work_capture else None
    (
        materiality_ok,
        review_candidate_id,
        materiality_dimension_id,
        materiality_basis,
    ) = materiality_review_facts(
        work_capture,
        bundle,
        behavior_class,
        goal_context_id,
        goal_statement,
        work_user_task,
        baseline_call,
        first_work_change,
        goal_source_id,
        decision_evidence,
    )
    (
        learning_participation_ok,
        learning_order_ok,
        learning_non_decision_ok,
        learning_basis,
    ) = learning_deliberation_facts(
        work_capture,
        bundle,
        behavior_class,
        review_candidate_id,
        materiality_dimension_id,
        first_work_change,
        materiality_basis,
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
        list(decision_evidence),
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
        evaluation_basis,
        baseline_call,
        review_candidate_id,
        materiality_dimension_id,
    )
    frontier_interrupted = bool(
        work_capture
        and any(
            call.result.get("questions")
            for call in work_capture.successful_calls("inquiry_frontier")
        )
    )
    decision_attempted = bool(
        work_capture and work_capture.calls("decision_record")
    )
    non_question_outcome_ok = (
        not frontier_interrupted
        and not decision_attempted
        and bool(work_capture)
        and not work_capture.calls("candidate_manage")
    )
    behavior_classification_ok = (
        behavior_class in BEHAVIOR_CLASSES
        and isinstance(evaluation_basis, dict)
        and evaluation_basis.get("behavior_class") == behavior_class
        and isinstance(behavior_review, dict)
        and behavior_review.get("classification") == behavior_class
    )
    appropriate_inquiry_outcome = (
        question_ok and decision_ok
        if is_user_owned_behavior(behavior_class)
        else non_question_outcome_ok
    )
    no_silent_user_owned_choice = (
        decision_ok
        if is_user_owned_behavior(behavior_class)
        else behavior_review.get("unresolved_material_user_outcome") is False
        if isinstance(behavior_review, dict)
        else False
    )
    decision_calls_for_order = (
        work_capture.successful_calls("decision_record") if work_capture is not None else []
    )
    hidden_material_discovery_order_ok = (
        behavior_class != "hidden_user_owned_decision"
        or (
            question_ok
            and decision_calls_for_order
            and len(
                materiality_basis.get("engineering_choice_discovery", {}).get(
                    "choice_ids", []
                )
            )
            >= 2
            and materiality_basis.get("engineering_choice_discovery", {}).get(
                "effect_categories"
            )
            and "coupled"
            in materiality_basis.get("engineering_choice_discovery", {}).get(
                "relationship_states", []
            )
            and first_work_change is not None
            and all(
                call.completion_sequence < first_work_change
                for call in decision_calls_for_order
            )
        )
    )
    ordinary_ok = (
        checkpoint_call is not None
        and bool(changed_paths)
        and not all(looks_like_synthetic_marker(path) for path in changed_paths)
        and not all(Path(path).suffix.lower() in {".txt", ".marker"} for path in changed_paths)
        and all(
            item.sequence < checkpoint_call.sequence
            for item in meaningful_work_path_observations(work_capture)
        )
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
        and not naturalistic_prompt_errors(work_user_task, resume_user_task, evaluation_basis)
        and task_turns_ok
    )
    initialize_call = unique_call(work_capture, "project_initialize")
    goal_call = unique_call(work_capture, "context_record")
    pre_existing_dirty_paths = checkpoint_pre_existing_dirty_paths(checkpoint_call)
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
        and initialize_call.completion_sequence < goal_call.sequence
        and checkpoint_baseline_is_pre_work(
            work_capture,
            checkpoint_call,
            project_id=bundle.project_id,
            boundary_completion_sequence=goal_call.completion_sequence,
            first_write_sequence=first_work_change,
        )
        and pre_existing_dirty_paths is not None
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
    activation_ok = (
        work_capture is not None
        and resume_capture is not None
        and work_capture.repository_scoped_activation_observed
        and resume_capture.repository_scoped_activation_observed
    )
    resolve_call = unique_call(resume_capture, "project_resolve")
    recall_call = unique_call(resume_capture, "recall")
    learning_recall_ok, learning_recall_basis = learning_recall_facts(
        resume_capture, behavior_class
    )
    resume_checkpoint_calls = (
        resume_capture.successful_calls("checkpoint_record")
        if resume_capture is not None
        else []
    )
    resume_terminal_checkpoint = (
        max(resume_checkpoint_calls, key=lambda call: call.sequence)
        if resume_checkpoint_calls
        else None
    )
    resume_baseline_call = selected_checkpoint_baseline_call(
        resume_capture, resume_terminal_checkpoint
    )
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
        and recall_call.completion_sequence < first_inspection
    )
    first_resume_write = min(
        (
            observation.sequence
            for observation in meaningful_work_path_observations(resume_capture)
        ),
        default=None,
    ) if resume_capture is not None else None
    resume_baseline_analysis_id = (
        resume_baseline_call.result.get("analysis_snapshot_id")
        if resume_baseline_call is not None
        else None
    )
    resume_baseline_ok = (
        resume_capture is not None
        and bundle is not None
        and recall_call is not None
        and resume_baseline_call is not None
        and bool(resume_checkpoint_calls)
        and nonempty_string(resume_baseline_analysis_id)
        and resume_baseline_call.arguments.get("project_id") == bundle.project_id
        and resume_baseline_call.result.get("project_id") == bundle.project_id
        and all(
            checkpoint_baseline_is_pre_work(
                resume_capture,
                call,
                project_id=bundle.project_id,
                boundary_completion_sequence=recall_call.completion_sequence,
                first_write_sequence=first_resume_write,
            )
            for call in resume_checkpoint_calls
        )
        and all(
            checkpoint_pre_existing_dirty_paths(call) is not None
            for call in resume_checkpoint_calls
        )
    )
    (
        resume_materiality_ok,
        _resume_review_candidate_id,
        _resume_materiality_dimension_id,
        resume_materiality_basis,
    ) = materiality_review_facts(
        resume_capture,
        bundle,
        behavior_class,
        goal_context_id,
        goal_statement,
        work_user_task,
        resume_baseline_call,
        first_resume_write,
        goal_source_id,
        decision_evidence,
        resumed=True,
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
    change_continuation_ok = (
        recall_match_ok
        and fresh_ok
        and ordering_ok
        and resume_baseline_ok
        and bool(relevant_resume_paths)
        and resume_validation_ok
    )
    checkpoint_work_state = (
        recalled_checkpoint_row.get("work_state")
        if recalled_checkpoint_row is not None
        else None
    )
    recalled_work_state = (
        recall_checkpoint.get("work_state")
        if isinstance(recall_checkpoint, dict)
        else None
    )
    contradictory_resume_behavior = False
    if resume_capture is not None and first_inspection is not None:
        contradictory_resume_behavior = any(
            command.sequence > first_inspection
            and isinstance(command.exit_code, int)
            and command.exit_code != 0
            for command in resume_capture.commands
        ) or any(
            call.sequence > first_inspection
            and call.arguments.get("work_state") not in {None, "completed"}
            for call in resume_capture.successful_calls("checkpoint_record")
        )
    verified_state_continuation_ok = (
        recall_match_ok
        and fresh_ok
        and ordering_ok
        and resume_baseline_ok
        and checkpoint_work_state == "completed"
        and recalled_work_state == "completed"
        and not continuation_paths
        and meaningful_resume_validation(resume_capture, first_inspection)
        and not contradictory_resume_behavior
    )
    continuation_mode = (
        "change_continuation"
        if change_continuation_ok
        else "verified_state_continuation"
        if verified_state_continuation_ok
        else None
    )
    continuation_ok = continuation_mode is not None
    resolved_material_question_not_reasked = (
        not is_user_owned_behavior(behavior_class)
        or (
            resume_capture is not None
            and not resume_capture.calls("decision_record")
            and not any(
                call.result.get("questions")
                for call in resume_capture.successful_calls("inquiry_frontier")
            )
        )
    )

    checks = {
        "repository_scoped_activation": evidence_check(references_present, activation_ok),
        "naturalistic_prompt_integrity": evidence_check(references_present, prompt_integrity_ok),
        "plain_task_goal_linkage": evidence_check(references_present, task_goal_ok),
        "grounded_pre_work_repository_baseline": evidence_check(
            references_present, baseline_ok
        ),
        "engineering_choice_discovery": evidence_check(
            references_present,
            bool(
                materiality_basis.get("engineering_choice_discovery", {}).get(
                    "discovery_candidate_id"
                )
            ),
        ),
        "pre_write_materiality_work_authority": evidence_check(
            references_present, materiality_ok
        ),
        "learning_participation": evidence_check(
            references_present, learning_participation_ok
        ),
        "learning_deliberation_order": evidence_check(
            references_present, learning_order_ok
        ),
        "learning_not_canonical_decision": evidence_check(
            references_present, learning_non_decision_ok
        ),
        "learning_interruption_precision": evidence_check(
            references_present,
            learning_order_ok
            if behavior_class == "learning_deliberation"
            else learning_non_decision_ok,
        ),
        "behavior_classification": evidence_check(references_present, behavior_classification_ok),
        "appropriate_inquiry_outcome": evidence_check(references_present, appropriate_inquiry_outcome),
        "hidden_material_discovery_order": evidence_check(
            references_present, hidden_material_discovery_order_ok
        ),
        "no_silent_user_owned_choice": evidence_check(references_present, no_silent_user_owned_choice),
        "meaningful_ordinary_changes": evidence_check(references_present, ordinary_ok),
        "source_grounded_checkpoint": evidence_check(references_present, checkpoint_ok),
        "decision_provenance_when_required": evidence_check(
            references_present,
            decision_ok if is_user_owned_behavior(behavior_class) else not decision_attempted,
        ),
        "distinct_work_and_resume_invocations": evidence_check(references_present, invocations_ok),
        "fresh_resume_without_prior_context": evidence_check(references_present, fresh_ok),
        "repository_bound_project_resolution": evidence_check(references_present, resolution_ok),
        "recall_precedes_inspection_and_continuation": evidence_check(references_present, ordering_ok),
        "resume_pre_work_repository_baseline": evidence_check(references_present, resume_baseline_ok),
        "resume_materiality_work_authority": evidence_check(
            references_present, resume_materiality_ok
        ),
        "recall_matches_checkpoint_decision_and_context": evidence_check(references_present, recall_match_ok),
        "learning_recall_continuity": evidence_check(
            references_present, learning_recall_ok
        ),
        "resolved_material_question_not_reasked": evidence_check(
            references_present, resolved_material_question_not_reasked
        ),
        "meaningful_recalled_continuation": evidence_check(references_present, continuation_ok),
        **{
            name: evidence_check(support_references_present[name], passed)
            for name, passed in support_checks.items()
        },
    }
    return {
        "evidence_class": "actual_repository_real_session",
        "status": status_from_steps(checks),
        "checks": checks,
        "changed_paths": changed_paths or [],
        "pre_existing_dirty_paths": pre_existing_dirty_paths or [],
        "continuation_paths": continuation_paths,
        "relevant_resume_paths": relevant_resume_paths,
        "continuation_basis": {
            "fresh_resume_session": fresh_ok,
            "repository_bound_project_resolution": resolution_ok,
            "recall_before_inspection_and_continuation": ordering_ok,
            "pre_work_repository_baseline": resume_baseline_ok,
            "pre_work_analysis_snapshot_id": resume_baseline_analysis_id,
            "materiality_work_authority": resume_materiality_basis,
            "checkpoint_supplied_next_meaningful_step": nonempty_string(next_step),
            "observed_change_relevant_to_checkpoint_next_step": bool(relevant_resume_paths),
            "resume_numeric_exit_validation": resume_validation_ok,
            "recalled_checkpoint_work_state": checkpoint_work_state,
            "continuation_mode": continuation_mode,
            "change_continuation_qualified": change_continuation_ok,
            "verified_state_continuation_qualified": verified_state_continuation_ok,
            "final_behavior_contradicts_completed_state": contradictory_resume_behavior,
        },
        "learning_basis": {
            **learning_basis,
            "recall": learning_recall_basis,
        },
        "activation_basis": {
            "work_session_start_observed": (
                work_capture.repository_scoped_activation_observed
                if work_capture is not None
                else False
            ),
            "resume_session_start_observed": (
                resume_capture.repository_scoped_activation_observed
                if resume_capture is not None
                else False
            ),
        },
        "checkpoint_id": checkpoint_id,
        "goal_context_id": goal_context_id,
        "decision_id": decision_id,
        "decision_ids": list(decision_evidence),
        "behavior_class": behavior_class,
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
        "inquiry_behavior_basis": {
            "frontier_interrupted_user": frontier_interrupted,
            "decision_attempted": decision_attempted,
            "non_question_outcome_qualified": non_question_outcome_ok,
            "ask_user_question_basis": question_review_basis,
            "materiality_review_basis": materiality_basis,
            "behavior_review_sha256": hashlib.sha256(
                json.dumps(behavior_review, sort_keys=True, separators=(",", ":")).encode("utf-8")
            ).hexdigest()
            if isinstance(behavior_review, dict)
            else None,
        },
        "campaign_support_evidence": support_basis,
        "evidence_origin": "repository_normalized_codex_rollout_and_canonical_bundle",
    }


def quality_observations(step_statuses: dict[str, str]) -> dict[str, dict[str, str]]:
    routes = {
        "context_recovery_accuracy": ("restart_recall",),
        "decision_repetition": ("inquiry_decision", "restart_recall"),
        "question_relevance": ("candidate_boundary", "inquiry_decision"),
        "question_necessity": ("candidate_boundary", "inquiry_decision"),
        "unnecessary_interruption": ("inquiry_decision", "ordinary_work"),
        "user_ownership": ("inquiry_decision",),
        "decision_comprehension": ("inquiry_decision",),
        "source_grounding": ("source_grounded_understanding", "checkpoint", "document_outputs"),
        "fact_interpretation_comprehension": ("source_grounded_understanding", "document_outputs"),
        "repository_analysis_usefulness": ("repository_analysis", "source_grounded_understanding"),
        "structural_navigation_usefulness": ("repository_analysis",),
        "semantic_value": ("repository_analysis", "source_grounded_understanding"),
        "capability_honesty": ("repository_analysis", "parser_failure", "provider_failure"),
        "coverage": ("repository_analysis", "source_grounded_understanding"),
        "polyglot_comprehension": ("repository_analysis", "source_grounded_understanding"),
        "cli_usability": ("project_binding", "repository_analysis", "restart_recall", "document_outputs"),
        "viewer_understanding": ("source_grounded_understanding", "document_outputs"),
        "memory_correctability": ("correction_supersession_deletion",),
        "interruption_cost": ("ordinary_work", "guarded_boundary"),
        "document_fidelity": ("document_outputs",),
        "document_usefulness": ("document_outputs",),
        "document_remaining_work_accuracy": ("checkpoint", "document_outputs"),
        "requested_language_body_content": ("document_outputs",),
        "portability": ("portable_clone", "divergent_conflict"),
        "recovery": ("provider_failure", "parser_failure", "derived_index_recovery"),
    }
    result: dict[str, dict[str, str]] = {}
    for name, steps in routes.items():
        routed = {step: step_statuses.get(step, "skipped") for step in steps}
        status = status_from_steps(routed)
        if name in {
            "question_relevance",
            "question_necessity",
            "unnecessary_interruption",
            "user_ownership",
            "decision_comprehension",
            "fact_interpretation_comprehension",
            "repository_analysis_usefulness",
            "structural_navigation_usefulness",
            "semantic_value",
            "polyglot_comprehension",
            "cli_usability",
            "viewer_understanding",
            "document_fidelity",
            "document_usefulness",
            "document_remaining_work_accuracy",
            "requested_language_body_content",
            "interruption_cost",
        } and status == "passed":
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
        "narrow_and_zoomed_presentation": "passed" if parser.viewport else "failed",
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
    candidate_revision: str,
    target_repository: Path,
    real_session_raw: dict[str, Any] | None,
    peak_memory: dict[str, Any],
    repeated_resources: dict[str, Any],
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
    quality = quality_observations(step_statuses)
    actual = real_session_evidence(
        real_session_raw,
        kind=kind,
        cycle=cycle,
        repository_revision=repository_revision,
        candidate_revision=candidate_revision,
        target_repository=target_repository,
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
        "machine_quality_evidence": quality,
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


def aggregate_machine_accessibility(
    repositories: list[dict[str, Any]],
    definition: dict[str, Any],
) -> dict[str, Any]:
    required = set(definition["automated_accessibility_checks"])
    checks: dict[str, str] = {}
    for repository in repositories:
        for cycle in repository.get("cycles", []):
            for name, status in cycle.get("accessibility", {}).get("checks", {}).items():
                if name not in required:
                    continue
                current = checks.get(name)
                if current == "failed" or status == "failed":
                    checks[name] = "failed"
                elif current == "partial" or status == "partial":
                    checks[name] = "partial"
                else:
                    checks[name] = status
    for name in required:
        checks.setdefault(name, "failed")
    return {
        "status": status_from_steps(checks),
        "checks": checks,
        "required_checks": list(definition["automated_accessibility_checks"]),
        "limits": [
            "Machine checks do not claim human keyboard, focus, color, or zoom usability.",
            "No standards certification or human accessibility review was performed by the automated run.",
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
    if "status" in result or "blockers" in result:
        raise ValueError("dogfood result may not conflate automated and replacement status")
    automated = result.get("automated_qualification")
    human = result.get("human_review")
    replacement = result.get("replacement_qualification")
    if (
        not isinstance(automated, dict)
        or automated.get("status") not in ALLOWED_STATUS
        or not isinstance(automated.get("passed"), bool)
        or not isinstance(automated.get("blockers"), list)
        or automated["passed"]
        != (automated["status"] == "passed" and not automated["blockers"])
        or result.get("automated_campaign_complete") is not True
    ):
        raise ValueError("dogfood automated qualification is incomplete or inconsistent")
    if (
        not isinstance(human, dict)
        or human.get("state") not in {"not_provided", "passed", "failed"}
        or human.get("required_samples")
        != deterministic_human_review_samples(result.get("repositories", []))
        or (
            human.get("state") == "not_provided"
            and human.get("artifact_sha256") is not None
        )
        or (
            human.get("state") != "not_provided"
            and not valid_capture_sha256(human.get("artifact_sha256"))
        )
    ):
        raise ValueError("dogfood human-review state is incomplete or inconsistent")
    expected_replacement = (
        "failed"
        if not automated["passed"]
        else "pending_human_review"
        if human["state"] == "not_provided"
        else "passed"
        if human["state"] == "passed"
        else "failed"
    )
    if (
        not isinstance(replacement, dict)
        or replacement.get("status") != expected_replacement
        or not nonempty_string(replacement.get("basis"))
        or result.get("replacement_pass_candidate")
        != (expected_replacement == "passed")
        or result.get("phase_9_ready") != (expected_replacement == "passed")
    ):
        raise ValueError("dogfood replacement qualification is incomplete or inconsistent")
    repositories = result.get("repositories", [])
    if [item.get("class") for item in repositories] != list(CLASSES):
        raise ValueError("dogfood result does not contain the three ordered repository classes")
    real_invocations: list[str] = []
    observed_behaviors: Counter[str] = Counter()
    hidden_repositories: set[str] = set()
    for repository in repositories:
        if len(repository.get("cycles", [])) != CYCLE_COUNT_BY_REPOSITORY[repository["class"]]:
            raise ValueError("dogfood result does not contain the maintained repository cycle allocation")
        for cycle in repository["cycles"]:
            behavior_class = cycle.get("real_session_dogfood", {}).get("behavior_class")
            if behavior_class not in BEHAVIOR_CLASSES:
                raise ValueError("dogfood cycle contains an unknown behavior class")
            observed_behaviors[behavior_class] += 1
            if behavior_class == "hidden_user_owned_decision":
                hidden_repositories.add(repository["class"])
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
                or repeated.get("observation_mode") != REAL_RESOURCE_OBSERVATION_MODE
                or tuple(repeated.get("operations_per_round", []))
                != RESOURCE_OPERATIONS
                or repeated.get("repetition_count")
                != definition["resource_qualification"][
                    "repeated_resource_repetition_count"
                ]
                or len(repeated.get("rounds", []))
                != repeated.get("repetition_count")
                or repeated.get("stale_temporary_files_observed") is not False
                or repeated.get("descendant_process_leak_observed") is not False
                or any(
                    not all(isinstance(round_value.get(name), int) for name in RESOURCE_HEALTH_METRICS)
                    for round_value in repeated.get("rounds", [])
                )
            ):
                raise ValueError("passed repeated-resource evidence is not bounded and measured")
            quality = cycle.get("machine_quality_evidence", {})
            if (
                not isinstance(quality, dict)
                or set(quality) != set(definition["quality_observations"])
                or not all(isinstance(observation, dict) for observation in quality.values())
                or any(
                observation.get("status") not in ALLOWED_STATUS
                for observation in quality.values()
                )
            ):
                raise ValueError("dogfood cycle machine quality evidence is incomplete")
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
        or aggregate_resources.get("observation_count") != QUALIFICATION_CYCLE_COUNT
        or aggregate_resources.get("universal_product_ceiling_applied") is not False
        or (
            aggregate_resources.get("status") == "passed"
            and aggregate_resources.get("measured_peak_count")
            != aggregate_resources.get("observation_count")
        )
    ):
        raise ValueError("dogfood aggregate resource qualification is incomplete")
    if automated["passed"]:
        if result.get("decision_revisit", {}).get("observed_active_triggers"):
            raise ValueError("replacement pass cannot have an active Decision revisit trigger")
        if result.get("candidate_worktree") != {"clean_before": True, "clean_after": True}:
            raise ValueError("replacement pass requires a clean candidate throughout dogfood")
        if result.get("fixture_regression", {}).get("status") != "passed":
            raise ValueError("replacement pass requires current structural/fallback regression")
        if result.get("machine_accessibility", {}).get("status") != "passed":
            raise ValueError("automated pass requires passed machine accessibility")
        if result.get("resource_qualification", {}).get("status") != "passed":
            raise ValueError("replacement pass requires passed resource qualification")
        accessibility_checks = result.get("machine_accessibility", {}).get("checks", {})
        if set(accessibility_checks) != set(definition["automated_accessibility_checks"]) or any(
            status != "passed" for status in accessibility_checks.values()
        ):
            raise ValueError("automated pass requires every machine accessibility check to pass")
        for repository in repositories:
            if repository.get("status") != "passed" or not repository.get("independent_fresh_runtime_cycles"):
                raise ValueError("replacement pass requires two independent passed behavior cycles")
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
                if cycle.get("resource_qualification", {}).get("status") != "passed":
                    raise ValueError("automated pass contains unqualified resource evidence")
        if (
            any(not nonempty_string(identity) for identity in real_invocations)
            or len(real_invocations)
            != definition["real_session_evidence"]["full_replacement_session_count"]
            or len(set(real_invocations)) != len(real_invocations)
        ):
            raise ValueError("automated pass requires sixteen globally distinct Codex sessions")
    if observed_behaviors != _PRIVATE_QUALIFICATION_BEHAVIOR_COUNTS:
        raise ValueError("dogfood result does not contain the required eight-cycle behavior multiset")
    if len(hidden_repositories) != 2:
        raise ValueError("hidden qualification cycles must span two repository classes")
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


def deterministic_human_review_samples(
    repositories: list[dict[str, Any]],
) -> dict[str, Any]:
    interaction: list[dict[str, Any]] = []
    for repository in repositories:
        kind = repository.get("class")
        candidates = sorted([
            cycle
            for cycle in repository.get("cycles", [])
            if cycle.get("status") == "passed"
            and cycle.get("real_session_dogfood", {}).get("status") == "passed"
        ], key=lambda item: item.get("cycle", 999))
        for cycle in candidates:
            interaction.append({
                "repository_class": kind,
                "cycle": cycle["cycle"],
                "behavior_class": cycle.get("real_session_dogfood", {}).get("behavior_class"),
                "project_id": cycle.get("project_identity"),
            })
    live_viewer = next(
        (sample for sample in interaction if sample["repository_class"] == "volicord"),
        None,
    )
    return {
        "algorithm": "every_automated_passed_interaction_cycle",
        "interaction": interaction,
        "documents": interaction,
        "viewer_snapshots": interaction,
        "repository_intelligence": interaction,
        "cli": interaction,
        "live_viewer": {
            "sample": live_viewer,
            "locales": ["en", "ko"],
        },
    }


COMMON_INTERACTION_REVIEW_CRITERIA = (
    "question_necessity_and_relevance",
    "user_ownership",
    "source_grounding",
    "decision_comprehension_when_applicable",
    "repeat_behavior",
    "correct_no_question_behavior",
)


def interaction_review_criteria(
    behavior_class: Any,
    definition: dict[str, Any],
) -> tuple[str, ...]:
    contract = definition["human_review_contract"]
    criterion_contracts = contract["interaction_behavior_criterion_contracts"]
    applicable = tuple(
        criterion
        for criterion in contract["interaction_behavior_criteria"]
        if behavior_class in criterion_contracts[criterion]["applies_to"]
    )
    return (*COMMON_INTERACTION_REVIEW_CRITERIA, *applicable)


def human_review_observation_template(review_prompt: str | None = None) -> dict[str, str]:
    return {
        "status": "not_provided",
        "basis": (
            f"Not yet reviewed. Apply this maintained contract: {review_prompt}"
            if review_prompt is not None
            else "Not yet reviewed; replace with a bounded human observation."
        ),
    }


def human_review_template(automated_result: dict[str, Any], result_sha256: str) -> dict[str, Any]:
    if not isinstance(automated_result, dict):
        raise ValueError("automated Dogfood result must be a JSON object")
    validate_result(automated_result, load_definition())
    if automated_result["automated_qualification"]["passed"] is not True:
        raise ValueError("human review is only meaningful after automated qualification passes")
    samples = automated_result["human_review"]["required_samples"]
    definition = load_definition()
    behavior_contracts = definition["human_review_contract"][
        "interaction_behavior_criterion_contracts"
    ]
    return {
        "kind": "phase8_dogfood_human_review",
        "candidate_head": automated_result["candidate_head"],
        "automated_result_sha256": result_sha256,
        "sampling": samples,
        "interaction_reviews": [
            {
                "sample": sample,
                **{
                    criterion: human_review_observation_template(
                        behavior_contracts[criterion]["review_prompt"]
                        if criterion in behavior_contracts
                        else None
                    )
                    for criterion in interaction_review_criteria(
                        sample.get("behavior_class"), definition
                    )
                },
            }
            for sample in samples["interaction"]
        ],
        "document_reviews": [
            {
                "sample": sample,
                **{
                    criterion: human_review_observation_template()
                    for criterion in (
                        "fidelity",
                        "usefulness",
                        "source_grounding",
                        "remaining_work_accuracy",
                        "requested_language_body_content",
                    )
                },
            }
            for sample in samples["documents"]
        ],
        "viewer_snapshot_reviews": [
            {
                "sample": sample,
                **{
                    criterion: human_review_observation_template()
                    for criterion in (
                        "completed_current_remaining_work",
                        "next_step",
                        "decision_rationale",
                        "architecture_components_flow",
                        "code_behavior",
                        "fact_versus_interpretation",
                        "diagram_usefulness",
                    )
                },
            }
            for sample in samples["viewer_snapshots"]
        ],
        "repository_intelligence_reviews": [
            {
                "sample": sample,
                **{
                    criterion: human_review_observation_template()
                    for criterion in (
                        "structural_navigation_usefulness",
                        "semantic_value_over_structural_only",
                        "capability_honesty",
                        "polyglot_comprehension_when_applicable",
                    )
                },
            }
            for sample in samples["repository_intelligence"]
        ],
        "cli_usability_reviews": [
            {
                "sample": sample,
                **{
                    task: human_review_observation_template()
                    for task in (
                        "discover_with_cli_help",
                        "status_without_project_id",
                        "analyze_without_project_id",
                        "recall_without_project_id",
                        "documents_without_project_id",
                        "export_without_project_id",
                        "doctor_without_project_id",
                    )
                },
            }
            for sample in samples["cli"]
        ],
        "live_viewer_accessibility": {
            "sample": samples["live_viewer"]["sample"],
            "locales": {
                locale: {
                    criterion: human_review_observation_template()
                    for criterion in (
                        "keyboard_reachability",
                        "visible_focus",
                        "not_color_only",
                        "narrow_and_zoomed_presentation",
                    )
                }
                for locale in samples["live_viewer"]["locales"]
            },
        },
    }


def human_review_observations(artifact: dict[str, Any]) -> list[dict[str, Any]]:
    values: list[dict[str, Any]] = []
    for review in artifact.get("interaction_reviews", []):
        if isinstance(review, dict):
            values.extend(value for name, value in review.items() if name != "sample")
    for field in (
        "document_reviews",
        "viewer_snapshot_reviews",
        "repository_intelligence_reviews",
    ):
        for review in artifact.get(field, []):
            if isinstance(review, dict):
                values.extend(value for name, value in review.items() if name != "sample")
    for review in artifact.get("cli_usability_reviews", []):
        if isinstance(review, dict):
            values.extend(value for name, value in review.items() if name != "sample")
    live = artifact.get("live_viewer_accessibility", {})
    locales = live.get("locales", {}) if isinstance(live, dict) else {}
    if isinstance(locales, dict):
        for review in locales.values():
            if isinstance(review, dict):
                values.extend(review.values())
    return values


def validate_human_review_artifact(
    artifact: dict[str, Any],
    automated_result: dict[str, Any],
    automated_result_sha256: str,
) -> str:
    if not isinstance(artifact, dict):
        raise ValueError("human review artifact must be a JSON object")
    expected_samples = automated_result["human_review"]["required_samples"]
    expected_artifact_fields = {
        "kind",
        "candidate_head",
        "automated_result_sha256",
        "sampling",
        "interaction_reviews",
        "document_reviews",
        "viewer_snapshot_reviews",
        "repository_intelligence_reviews",
        "cli_usability_reviews",
        "live_viewer_accessibility",
    }
    interaction_reviews = artifact.get("interaction_reviews")
    document_reviews = artifact.get("document_reviews")
    snapshot_reviews = artifact.get("viewer_snapshot_reviews")
    intelligence_reviews = artifact.get("repository_intelligence_reviews")
    cli_reviews = artifact.get("cli_usability_reviews")
    live_review = artifact.get("live_viewer_accessibility")
    locales = live_review.get("locales") if isinstance(live_review, dict) else None
    if (
        set(artifact) != expected_artifact_fields
        or artifact.get("kind") != "phase8_dogfood_human_review"
        or artifact.get("candidate_head") != automated_result.get("candidate_head")
        or artifact.get("automated_result_sha256") != automated_result_sha256
        or artifact.get("sampling") != expected_samples
        or not isinstance(interaction_reviews, list)
        or len(interaction_reviews) != len(expected_samples["interaction"])
        or not all(isinstance(item, dict) for item in interaction_reviews)
        or not isinstance(document_reviews, list)
        or not isinstance(snapshot_reviews, list)
        or not isinstance(intelligence_reviews, list)
        or not isinstance(cli_reviews, list)
        or not all(
            isinstance(item, dict)
            for collection in (document_reviews, snapshot_reviews, intelligence_reviews, cli_reviews)
            for item in collection
        )
        or not isinstance(live_review, dict)
        or not isinstance(locales, dict)
        or set(locales) != {"en", "ko"}
        or not all(isinstance(item, dict) for item in locales.values())
    ):
        raise ValueError("human review artifact identity or deterministic sampling is invalid")
    if [item.get("sample") for item in interaction_reviews] != expected_samples[
        "interaction"
    ]:
        raise ValueError("human interaction review samples are not deterministic")
    definition = load_definition()
    if any(
        set(item)
        != {
            "sample",
            *interaction_review_criteria(item["sample"].get("behavior_class"), definition),
        }
        for item in interaction_reviews
    ):
        raise ValueError("human interaction review criteria are incomplete")
    document_criteria = {
        "fidelity",
        "usefulness",
        "source_grounding",
        "remaining_work_accuracy",
        "requested_language_body_content",
    }
    viewer_criteria = {
        "completed_current_remaining_work",
        "next_step",
        "decision_rationale",
        "architecture_components_flow",
        "code_behavior",
        "fact_versus_interpretation",
        "diagram_usefulness",
    }
    intelligence_criteria = {
        "structural_navigation_usefulness",
        "semantic_value_over_structural_only",
        "capability_honesty",
        "polyglot_comprehension_when_applicable",
    }
    for collection, samples, criteria, label in (
        (document_reviews, expected_samples["documents"], document_criteria, "document"),
        (snapshot_reviews, expected_samples["viewer_snapshots"], viewer_criteria, "Viewer"),
        (intelligence_reviews, expected_samples["repository_intelligence"], intelligence_criteria, "Repository Intelligence"),
    ):
        if [item.get("sample") for item in collection] != samples or any(
            set(item) != {"sample", *criteria} for item in collection
        ):
            raise ValueError(f"human {label} review samples or criteria are incomplete")
    cli_criteria = {
        "discover_with_cli_help",
        "status_without_project_id",
        "analyze_without_project_id",
        "recall_without_project_id",
        "documents_without_project_id",
        "export_without_project_id",
        "doctor_without_project_id",
    }
    if (
        [item.get("sample") for item in cli_reviews] != expected_samples["cli"]
        or any(set(item) != {"sample", *cli_criteria} for item in cli_reviews)
    ):
        raise ValueError("human CLI review does not cover representative help-discovered tasks")
    if (
        set(live_review) != {"sample", "locales"}
        or live_review.get("sample") != expected_samples["live_viewer"]["sample"]
        or any(
            set(review) != set(load_definition()["human_review_contract"]["live_viewer_criteria"])
            for review in locales.values()
        )
    ):
        raise ValueError("human live Viewer sample is not deterministic")
    observations = human_review_observations(artifact)
    expected_count = (
        sum(
            len(interaction_review_criteria(sample.get("behavior_class"), definition))
            for sample in expected_samples["interaction"]
        )
        + len(expected_samples["documents"]) * len(document_criteria)
        + len(expected_samples["viewer_snapshots"]) * len(viewer_criteria)
        + len(expected_samples["repository_intelligence"]) * len(intelligence_criteria)
        + len(expected_samples["cli"]) * len(cli_criteria)
        + 2 * 4
    )
    if len(observations) != expected_count:
        raise ValueError("human review artifact does not contain every required criterion")
    statuses: list[str] = []
    for index, observation in enumerate(observations):
        if not isinstance(observation, dict) or set(observation) != {"status", "basis"}:
            raise ValueError("human review observations require status and basis")
        status = observation.get("status")
        basis = observation.get("basis")
        if status not in {"not_provided", "passed", "failed"}:
            raise ValueError("human review observation status is invalid")
        if not nonempty_string(basis) or len(basis.encode("utf-8")) > MAX_REVIEW_TEXT_BYTES:
            raise ValueError(f"human review observation {index} has no bounded basis")
        statuses.append(str(status))
    if set(statuses) == {"not_provided"}:
        return "not_provided"
    return "passed" if set(statuses) == {"passed"} else "failed"


def combine_human_review(
    automated_result: dict[str, Any],
    artifact: dict[str, Any],
    automated_result_sha256: str,
) -> dict[str, Any]:
    if not isinstance(automated_result, dict):
        raise ValueError("automated Dogfood result must be a JSON object")
    definition = load_definition()
    validate_result(automated_result, definition)
    if automated_result.get("human_review", {}).get("state") != "not_provided":
        raise ValueError("qualification requires the immutable automated Dogfood result")
    review_state = validate_human_review_artifact(
        artifact,
        automated_result,
        automated_result_sha256,
    )
    result = json.loads(json.dumps(automated_result))
    result["human_review"] = {
        "state": review_state,
        "artifact_sha256": hashlib.sha256(
            json.dumps(artifact, sort_keys=True, separators=(",", ":")).encode("utf-8")
        ).hexdigest(),
        "required_samples": artifact["sampling"],
    }
    automated_passed = result["automated_qualification"]["passed"] is True
    if not automated_passed:
        replacement_status = "failed"
        basis = "automated qualification did not pass and cannot be overridden by human review"
    elif review_state == "not_provided":
        replacement_status = "pending_human_review"
        basis = "automated qualification passed; campaign-level human review was not provided"
    elif review_state == "passed":
        replacement_status = "passed"
        basis = "automated qualification and the maintained campaign-level human review passed"
    else:
        replacement_status = "failed"
        basis = "one or more maintained qualitative human-review criteria did not pass"
    result["replacement_qualification"] = {
        "status": replacement_status,
        "basis": basis,
    }
    result["replacement_pass_candidate"] = replacement_status == "passed"
    result["phase_9_ready"] = replacement_status == "passed"
    validate_result(result, definition)
    return result


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
                for cycle_number in cycle_numbers(kind):
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
                        candidate_head,
                        source_by_class[kind],
                        load_real_session_cycle(
                            specs[kind].get("real_session_evidence", {}).get(str(cycle_number)),
                            repository_manifest.parent,
                        ),
                        peak_memory,
                        repeated_resources,
                    ))
            else:
                for cycle_number in cycle_numbers(kind):
                    skipped = {name: "environment_blocked" for name in definition["required_product_steps"]}
                    actual = real_session_evidence(
                        load_real_session_cycle(
                            specs[kind].get("real_session_evidence", {}).get(str(cycle_number)),
                            repository_manifest.parent,
                        ),
                        kind=kind,
                        cycle=cycle_number,
                        repository_revision=identity["revision"],
                        candidate_revision=candidate_head,
                        target_repository=source_by_class[kind],
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
                        "machine_quality_evidence": quality_observations(skipped),
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
    accessibility = aggregate_machine_accessibility(repository_results, definition)
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
        blockers.append("machine accessibility qualification did not pass")
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
    if regression["status"] != "passed":
        blockers.append("maintained structural/fallback regression did not pass")
    automated_status = aggregate_status(
        repository_results,
        regression,
        accessibility,
        resource_qualification,
        blockers,
    )
    automated_passed = automated_status == "passed" and not blockers
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
        "automated_campaign_complete": True,
        "automated_qualification": {
            "status": automated_status,
            "passed": automated_passed,
            "blockers": blockers,
        },
        "human_review": {
            "state": "not_provided",
            "artifact_sha256": None,
            "required_samples": deterministic_human_review_samples(repository_results),
        },
        "replacement_qualification": {
            "status": "pending_human_review" if automated_passed else "failed",
            "basis": (
                "automated qualification passed; campaign-level human review was not provided"
                if automated_passed
                else "automated qualification did not pass"
            ),
        },
        "replacement_pass_candidate": False,
        "phase_9_ready": False,
        "repositories": repository_results,
        "fixture_regression": regression,
        "machine_accessibility": accessibility,
        "resource_qualification": resource_qualification,
        "privacy_and_transmission": {
            "evidence_mode": definition["real_session_evidence"]["mode"],
            "harness_performed_or_authorized_codex_transmission": False,
            "verified_external_codex_session_count_expected": QUALIFICATION_SESSION_COUNT,
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
            "No subjective human usability or live manual accessibility conclusion is included in automated qualification.",
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
        "automated_qualification": automated_status,
        "automated_passed": automated_passed,
        "human_review": "not_provided",
        "replacement_qualification": result["replacement_qualification"]["status"],
        "candidate_head": candidate_head,
        "blockers": blockers,
        "result": "dogfood-result.json",
    }, indent=2, sort_keys=True))
    return 0 if automated_passed else 1


def fixture_work_user_task(kind: str, behavior_class: str) -> str:
    tasks = {
        "explicit_user_owned_decision": "The operator-facing error-detail policy is unsettled. Improve it and add focused coverage for the selected observable behavior.",
        "hidden_user_owned_decision": "Improve invalid-configuration diagnostics and add focused coverage for the observable behavior.",
        "research_or_no_question": "Correct repository configuration discovery and update the affected multi-file tests.",
        "delegated_implementation_choice": "Refactor the adapter while preserving behavior; choose the internal helper naming and module structure.",
        "exploratory_uncertainty": "Investigate the intermittent analysis latency and leave a tested prototype or a clear revisit basis.",
        "learning_deliberation": "I want to learn through one meaningful agent-owned technical fork before implementation. Improve the adapter state model and add focused tests.",
        "learning_routine_control": "I want to learn while you improve the adapter, but keep routine naming and local formatting choices non-interrupting. Add focused tests.",
    }
    repository_reference = (
        "In this repository"
        if behavior_class == "hidden_user_owned_decision"
        else f"In the {kind} repository"
    )
    return f"{repository_reference}, {tasks[behavior_class]} Keep the change bounded and do not add dependencies."


def fixture_question_content() -> dict[str, Any]:
    return {
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


def fixture_evaluation_basis(behavior_class: str) -> dict[str, Any]:
    return {
        "behavior_class": behavior_class,
        "repository_facts": [
            "The adapter already separates diagnostic detail from its stable result envelope."
        ],
        "accepted_contract_constraints": [
            "The active contract requires truthful failure reporting without prescribing wording."
        ],
        "delegated_boundaries": (
            ["The current task explicitly delegates internal helper naming and module structure to the agent."]
            if behavior_class == "delegated_implementation_choice"
            else []
        ),
        "possible_material_concerns": (
            ["The externally visible diagnostic policy may materially affect operators."]
            if is_user_owned_behavior(behavior_class)
            else []
        ),
        "consequences": ["The outcome changes diagnostic usefulness or maintenance cost."],
        "facts_not_for_user": ["Existing adapter and test behavior must be inspected locally."],
        "current_relevance": "The selected class tests proportional inquiry behavior for this bounded task.",
    }


def fixture_behavior_review(behavior_class: str) -> dict[str, Any]:
    if is_user_owned_behavior(behavior_class):
        counterfactual_review = {
            "applicability": "required_for_material_user_owned_decision",
            "specific_unresolved_outcome": (
                "The stable operator-facing error-detail policy remains unresolved."
            ),
            "frozen_task_necessity": (
                "Improving the named public error policy necessarily selects what stable detail operators receive."
            ),
            "repository_research_cannot_settle": (
                "Repository inspection establishes the existing envelope but cannot choose the user-valued stability and troubleshooting trade-off."
            ),
            "repository_facts_settle_outcome": False,
            "accepted_decision_or_contract_cannot_settle": (
                "The active truthful-failure contract does not prescribe the public detail boundary."
            ),
            "accepted_decision_or_contract_settles_outcome": False,
            "not_delegated_basis": (
                "Delegation covers implementation structure, not the externally observable error policy."
            ),
            "outcome_within_delegated_authority": False,
            "materially_different_consequences": [
                "Concise stable errors reduce automation churn but require separate diagnostic inspection.",
                "Detailed public errors improve immediate troubleshooting but enlarge the compatibility surface.",
            ],
            "no_question_approaches": [
                {
                    "approach": "Keep the existing public error policy and only refactor internals.",
                    "task_satisfaction": "fails_frozen_task",
                    "assessment": "It preserves behavior and therefore does not improve the requested public error policy.",
                },
                {
                    "approach": "Choose a conventional concise error and add private diagnostics.",
                    "task_satisfaction": "implicitly_chooses_same_user_owned_outcome",
                    "assessment": "It completes the task only by selecting the same unresolved public detail boundary.",
                },
            ],
            "material_outcome_unavoidable": True,
            "operator_prompt_does_not_disclose_material_outcome": (
                behavior_class == "hidden_user_owned_decision"
            ),
            "conclusion": "unavoidable_user_owned_outcome",
        }
    else:
        counterfactual_review = {
            "applicability": "not_required_for_behavior_class",
            "specific_unresolved_outcome": None,
            "frozen_task_necessity": None,
            "repository_research_cannot_settle": None,
            "repository_facts_settle_outcome": None,
            "accepted_decision_or_contract_cannot_settle": None,
            "accepted_decision_or_contract_settles_outcome": None,
            "not_delegated_basis": None,
            "outcome_within_delegated_authority": None,
            "materially_different_consequences": [],
            "no_question_approaches": [],
            "material_outcome_unavoidable": False,
            "operator_prompt_does_not_disclose_material_outcome": None,
            "conclusion": "not_applicable",
        }
    return {
        "kind": "phase8_behavior_review",
        "classification": behavior_class,
        "provenance_references": [
            {
                "scope": "volicord_active_owner",
                "path": "rebuild/docs/design/inquiry-and-decision.md",
                "sha256": sha256(ROOT / "rebuild/docs/design/inquiry-and-decision.md"),
                "repository_revision": git_head(ROOT),
            },
        ],
        "outcome_rationale": "Independent evidence supports the selected proportional behavior class.",
        "user_ownership_assessment": "Ownership was checked against facts, contracts, and delegated boundaries.",
        "silent_choice_risk_assessment": "The review records whether proceeding would silently choose a user-owned outcome.",
        "unresolved_material_user_outcome": is_user_owned_behavior(behavior_class),
        "independent_review": {
            "status": "accepted",
            "reviewer_role": "campaign_preparation_independent_reviewer",
            "basis": "Independent control-session review accepted the bounded behavior classification.",
            "review_preparation": {
                "kind": "phase8_blind_review_preparation_reference",
                "review_slot_id": "11" * 16,
                "sha256": "aa" * 32,
            },
            "provisional_review": {
                "kind": "phase8_provisional_behavior_review",
                "review_slot_id": "11" * 16,
                "status": "recorded",
                "reviewer_role": "campaign_preparation_independent_reviewer",
                "preparation_sha256": "aa" * 32,
                "classification": behavior_class,
                "materiality_conclusion": (
                    "user_owned_material_outcome"
                    if is_user_owned_behavior(behavior_class)
                    else "no_user_owned_material_outcome"
                ),
                "material_outcome_unavoidable": is_user_owned_behavior(behavior_class),
                "operator_prompt_does_not_disclose_material_outcome": (
                    True
                    if behavior_class == "hidden_user_owned_decision"
                    else False
                    if behavior_class == "explicit_user_owned_decision"
                    else None
                ),
                "basis": "Repository and owner inspection produced this conclusion before evaluator material was revealed.",
                "provenance_reference_indices": [0],
            },
            "classification_comparison": {
                "status": "agreed",
                "provisional_classification": behavior_class,
                "evaluator_classification": behavior_class,
                "disagreements": [],
                "resolution_basis": (
                    "The cited evidence supports the matching provisional and evaluator conclusions."
                ),
                "provenance_reference_indices": [0],
            },
            "fact_authority_agreement": {
                "status": "agreed",
                "evaluator_conclusions": [
                    "The repository and active owner evidence support the selected behavior class."
                ],
                "reviewer_conclusions": [
                    "Independent inspection reached the same repository-fact and authority conclusion."
                ],
                "conflicts": [],
                "resolution_basis": (
                    "The cited active-owner evidence resolves the relevant authority boundary."
                ),
                "provenance_reference_indices": [0],
            },
            "counterfactual_review": counterfactual_review,
        },
    }


def real_session_fixture(
    kind: str,
    cycle: int,
    revision: str,
    evidence_directory: Path,
    repository_path: Path | None = None,
    *,
    behavior_class: str = "explicit_user_owned_decision",
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
    resume_baseline_analysis = "12" * 32
    resume_baseline_repository = "13" * 32
    resume_repository_source = "14" * 16
    resume_current_analysis = "15" * 32
    resume_current_repository = "16" * 32
    resume_checkpoint = "17" * 16
    review_candidate = "18" * 16
    resume_review_candidate = "19" * 16
    review_analysis = "1a" * 32
    resume_review_analysis = "1b" * 32
    discovery_candidate = "1c" * 16
    resume_discovery_candidate = "1d" * 16
    learning_candidate = "1e" * 16
    learning_response_source = "1f" * 16
    materiality_dimension_id = "operator-error-boundary"
    secondary_materiality_dimension_id = "repository-shape-boundary"
    repository_cwd = str(repository_path.resolve()) if repository_path else "/phase8/repository"
    work_session = f"{kind}-work-session-{cycle}"
    resume_session = f"{kind}-resume-session-{cycle}"
    work_user_task = fixture_work_user_task(kind, behavior_class)
    question_content = fixture_question_content()
    evaluation_basis = fixture_evaluation_basis(behavior_class)
    applied_decisions = [decision] if is_user_owned_behavior(behavior_class) else []
    decision_turn_text = "Keep the normal output concise; diagnostics can carry the actionable cause."
    resume_user_task = "Continue the validation-adapter improvement from the current project state."
    question_prompt = "Which error-detail boundary should the validation adapter expose to operators?"
    next_step = "Update src/resume.rs to carry the chosen concise diagnostic boundary and verify it"
    work_paths = (
        ["backend/src/existing.rs", "frontend/src/existing.ts"]
        if kind == "polyglot-medium"
        else ["src/existing.rs", "tests/existing.rs"]
    )
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
                "cwd": repository_cwd,
                "originator": "codex_vscode",
                "cli_version": "0.148.0-alpha.9",
                "source": "vscode",
                "thread_source": "user",
                "model_provider": "openai",
                "git": {"commit_hash": revision, "branch": "phase8"},
            },
        )

    def activation_message() -> dict[str, Any]:
        return event(
            "response_item",
            {
                "type": "message",
                "role": "developer",
                "content": [
                    {
                        "type": "input_text",
                        "text": (
                            "Volicord is active for this explicitly authorized repository. "
                            "Start project-scoped repository work with project_resolve, then follow "
                            "every returned workflow.required_next_action until blocks_ordinary_work is false."
                        ),
                    }
                ],
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
            {"cmd": command, "workdir": repository_cwd, "yield_time_ms": 30000},
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
    discovery_call = f"{kind}-discovery-call-{cycle}"
    materiality_call = f"{kind}-materiality-call-{cycle}"
    candidate_submit_call = f"{kind}-candidate-submit-call-{cycle}"
    candidate_research_call = f"{kind}-candidate-research-call-{cycle}"
    candidate_ready_call = f"{kind}-candidate-ready-call-{cycle}"
    candidate_promote_call = f"{kind}-candidate-promote-call-{cycle}"
    inquiry_call = f"{kind}-inquiry-call-{cycle}"
    decision_call = f"{kind}-decision-call-{cycle}"
    materiality_revision_call = f"{kind}-materiality-revision-call-{cycle}"
    patch_call = f"{kind}-patch-call-{cycle}"
    current_analysis_call = f"{kind}-current-analysis-call-{cycle}"
    verification_call = f"{kind}-verification-call-{cycle}"
    checkpoint_call = f"{kind}-checkpoint-call-{cycle}"
    learning_begin_call = f"{kind}-learning-begin-call-{cycle}"
    learning_response_call = f"{kind}-learning-response-call-{cycle}"
    learning_feedback_call = f"{kind}-learning-feedback-call-{cycle}"
    learning_complete_call = f"{kind}-learning-complete-call-{cycle}"
    materiality_disposition = expected_materiality_disposition(behavior_class)
    materiality_basis_kind = {
        "explicit_user_owned_decision": "agent_recommendation",
        "hidden_user_owned_decision": "agent_recommendation",
        "research_or_no_question": "repository_or_environment_fact",
        "delegated_implementation_choice": "explicit_delegation",
        "exploratory_uncertainty": "research_evidence",
        "learning_deliberation": "implementation_preference",
        "learning_routine_control": "implementation_preference",
    }[behavior_class]
    delegation_statement = "choose the internal helper naming and module structure"
    delegation_scope = "internal helper naming and module structure"
    learning_active = behavior_class in {
        "learning_deliberation",
        "learning_routine_control",
    }

    def engineering_choices(source_id: str) -> list[dict[str, Any]]:
        return [
            {
                "choice_id": materiality_dimension_id,
                "summary": "Choose the adapter state representation",
                "affected_scope": ["adapter state representation"],
                "alternatives": [
                    {
                        "alternative_id": "ordered-records",
                        "summary": "Use ordered records",
                        "technical_consequences": ["Deterministic inspection with bounded linear lookup"],
                    },
                    {
                        "alternative_id": "keyed-index",
                        "summary": "Use a keyed index",
                        "technical_consequences": ["Direct lookup with ordering synchronization obligations"],
                    },
                ],
                "technical_consequences": ["The representation changes invariant placement and maintenance cost"],
                "source_ids": [source_id],
                "effect_categories": ["maintenance_or_support", "implementation_internal"],
                "relationship": {"state": "independent"},
                "evidence_state": "sufficient",
            },
            {
                "choice_id": secondary_materiality_dimension_id,
                "summary": "Choose the coupled repository-shape boundary",
                "affected_scope": ["repository file shape"],
                "alternatives": [
                    {"alternative_id": "bounded", "summary": "Keep the bounded file shape", "technical_consequences": ["Limits the touched surface"]},
                    {"alternative_id": "expanded", "summary": "Expand the file shape", "technical_consequences": ["Broadens the touched surface"]},
                ],
                "technical_consequences": ["The shape affects the scope of implementation changes"],
                "source_ids": [source_id],
                "effect_categories": ["maintenance_or_support"],
                "relationship": (
                    {
                        "state": "coupled",
                        "choice_ids": [materiality_dimension_id],
                        "rationale": "The observable diagnostic boundary and stable shape share one user-owned outcome.",
                    }
                    if is_user_owned_behavior(behavior_class)
                    else {"state": "independent"}
                ),
                "evidence_state": "sufficient",
            },
        ]

    def materiality_dimension(
        *, resolved: bool = False, source_id: str | None = None
    ) -> dict[str, Any]:
        authority_source = (
            goal_source
            if behavior_class == "delegated_implementation_choice"
            else source_id or repository_source
        )
        dimension = {
            "dimension_id": materiality_dimension_id,
            "discovered_choice_ids": [materiality_dimension_id],
            "summary": (
                "Select the delegated internal helper and module structure"
                if behavior_class == "delegated_implementation_choice"
                else "Classify the operator-facing error-detail outcome"
            ),
            "affected_scope": [
                delegation_scope
                if behavior_class == "delegated_implementation_choice"
                else question_content["user_owned_dimension"]
            ],
            "material_consequences": [
                "The choice changes internal maintainability without changing public behavior."
                if behavior_class == "delegated_implementation_choice"
                else question_content["material_consequence"]
            ],
            "observable_signals": ["observable_failure_policy"],
            "disposition": materiality_disposition,
            "learning_value": (
                {
                    "state": "deliberation_worthy",
                    "rationale": "The state representation exposes transferable invariant-placement trade-offs.",
                    "consequence_significance": ["The representation determines where consistency invariants live."],
                    "transferable_principles": ["Choose representations that make invariants explicit."],
                    "non_obvious_trade_offs": ["Direct lookup adds synchronization and ordering obligations."],
                }
                if behavior_class == "learning_deliberation"
                else {
                    "state": "routine",
                    "rationale": "This bounded detail is routine and should not interrupt implementation.",
                }
            ),
            "basis": {
                "kinds": ["applicable_decision" if resolved else materiality_basis_kind],
                "summary": "Bounded repository and owner-authority evidence",
                "source_ids": [authority_source],
                "contract_basis": (
                    evaluation_basis.get("accepted_contract_constraints", [])
                    if behavior_class == "research_or_no_question"
                    else []
                ),
                "decision_ids": [decision] if resolved else [],
                "research_basis": (
                    evaluation_basis.get("repository_facts", [])
                    if behavior_class == "exploratory_uncertainty"
                    else []
                ),
            },
        }
        if behavior_class == "delegated_implementation_choice" and not resolved:
            dimension["basis"]["explicit_delegation"] = {
                "goal_context_id": context,
                "user_turn_source_id": goal_source,
                "verbatim_statement": delegation_statement,
                "affected_scope": [delegation_scope],
            }
        if behavior_class == "exploratory_uncertainty":
            dimension["exploratory_disposition"] = "resolved_by_research"
        if resolved:
            dimension["resolution_decision_id"] = decision
        return dimension

    def secondary_materiality_dimension(
        *, resolved: bool = False, source_id: str = repository_source
    ) -> dict[str, Any]:
        if is_user_owned_behavior(behavior_class):
            return {
                "dimension_id": secondary_materiality_dimension_id,
                "discovered_choice_ids": [secondary_materiality_dimension_id],
                "summary": "Classify the coupled diagnostic stability outcome",
                "affected_scope": ["operator diagnostic stability"],
                "material_consequences": [
                    "The same public choice controls stable diagnostic disclosure."
                ],
                "observable_signals": ["observable_failure_policy"],
                "disposition": "unresolved_user_owned_outcome",
                "learning_value": {"state": "routine", "rationale": "User-owned authority stays on the Inquiry path."},
                "resolution_decision_id": decision if resolved else None,
                "basis": {
                    "kinds": [
                        "applicable_decision" if resolved else "agent_recommendation"
                    ],
                    "summary": "A coupled independently material diagnostic consequence",
                    "source_ids": [source_id],
                    "contract_basis": [],
                    "decision_ids": [decision] if resolved else [],
                    "research_basis": [],
                },
            }
        return {
            "dimension_id": secondary_materiality_dimension_id,
            "discovered_choice_ids": [secondary_materiality_dimension_id],
            "summary": "Record the repository-established bounded file shape",
            "affected_scope": ["repository file shape"],
            "material_consequences": ["The implementation stays within inspected files."],
            "observable_signals": ["other_material_outcome"],
            "disposition": "repository_or_environment_fact",
            "learning_value": {"state": "routine", "rationale": "A repository-established fact needs no learning interruption."},
            "basis": {
                "kinds": ["repository_or_environment_fact"],
                "summary": "The retained Analysis Snapshot establishes this fact.",
                "source_ids": [source_id],
                "contract_basis": [],
                "decision_ids": [],
                "research_basis": [],
            },
        }

    def materiality_dimensions(
        *, resolved: bool = False, source_id: str = repository_source
    ) -> list[dict[str, Any]]:
        primary = materiality_dimension(resolved=resolved, source_id=source_id)
        secondary = secondary_materiality_dimension(
            resolved=resolved, source_id=source_id
        )
        return [secondary, primary] if resolved else [primary, secondary]

    def ready_workflow(review_id: str, baseline_id: str) -> dict[str, Any]:
        return {
            "stage": "ready_for_work",
            "disposition": "ready_for_work",
            "required_next_action": {"tool": "checkpoint_record", "action": None},
            "blocks_ordinary_work": False,
            "reason": "all material outcome dimensions have resolved work authority",
            "satisfied_basis_identities": [
                {"kind": "project", "identity": project},
                {"kind": "goal_context", "identity": context},
                {"kind": "baseline_analysis_snapshot", "identity": baseline_id},
                {"kind": "materiality_review_candidate", "identity": review_id},
            ],
            "unresolved_requirements": [],
        }

    def learning_workflow(review_id: str, baseline_id: str) -> dict[str, Any]:
        return {
            **ready_workflow(review_id, baseline_id),
            "stage": "learning_deliberation",
            "disposition": "learning_required",
            "required_next_action": {"tool": "learning_deliberation", "action": "begin"},
            "blocks_ordinary_work": True,
            "reason": "explicit learning participation and a deliberation-worthy agent-owned choice require pre-work deliberation",
            "unresolved_requirements": [{"dimension_id": materiality_dimension_id, "reason": "learning deliberation is pending", "basis_identities": []}],
        }
    patch_text = (
        "*** Begin Patch\n"
        + "".join(
            f"*** Update File: {repository_cwd}/{path}\n@@\n-old\n+new\n"
            for path in work_paths
        )
        + "*** End Patch"
    )
    work_events = [
        session_meta(work_session),
        activation_message(),
        task(work_turn),
        user(work_turn, f"{kind}-user-turn-{cycle}", work_user_task),
        mcp_call(
            work_turn,
            initialize_call,
            "project_initialize",
            {"display_name": "Phase 8 fixture", "repository": repository_cwd},
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
                "repository_source_id": repository_source,
            },
        ),
        mcp_call(
            work_turn,
            discovery_call,
            "engineering_choice_discovery",
            {
                "project_id": project,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": baseline_analysis,
                "source_operation": "naturalistic engineering-choice discovery",
                "summary": "Discover independently meaningful technical choices before authority review.",
                "choices": engineering_choices(repository_source),
            },
        ),
        custom_output(
            work_turn,
            discovery_call,
            {
                "action": "record",
                "discovery_candidate_id": discovery_candidate,
                "revision": 1,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": baseline_analysis,
                "choices": engineering_choices(repository_source),
                "canonical_mutation": False,
                "workflow": {"stage": "materiality_review", "blocks_ordinary_work": True},
            },
        ),
        mcp_call(
            work_turn,
            materiality_call,
            "materiality_review",
            {
                "action": "record",
                "project_id": project,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": baseline_analysis,
                "engineering_choice_discovery_candidate_id": discovery_candidate,
                "source_operation": "naturalistic pre-work Materiality Review",
                "rationale": "Classify independently material outcomes before affected work.",
                "learning_participation": (
                    {
                        "state": "active",
                        "user_turn_source_id": goal_source,
                        "verbatim_statement": "I want to learn",
                    }
                    if learning_active
                    else {"state": "inactive"}
                ),
                "dimensions": materiality_dimensions(),
            },
        ),
        custom_output(
            work_turn,
            materiality_call,
            {
                "action": "record",
                "review_candidate_id": review_candidate,
                "review_revision": 1,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": baseline_analysis,
                "review_analysis_snapshot_id": review_analysis,
                "canonical_mutation": False,
                "workflow": (
                    {
                        "stage": "question_candidate",
                        "disposition": "question_required",
                        "required_next_action": {
                            "tool": "candidate_manage",
                            "action": "submit_question_from_materiality",
                        },
                        "blocks_ordinary_work": True,
                        "reason": "explicit user authority is required",
                        "satisfied_basis_identities": [
                            {"kind": "project", "identity": project},
                            {"kind": "goal_context", "identity": context},
                            {"kind": "baseline_analysis_snapshot", "identity": baseline_analysis},
                            {"kind": "materiality_review_candidate", "identity": review_candidate},
                        ],
                        "unresolved_requirements": [
                            {
                                "dimension_id": materiality_dimension_id,
                                "reason": "explicit user authority is required",
                                "basis_identities": [],
                            },
                            {
                                "dimension_id": secondary_materiality_dimension_id,
                                "reason": "explicit user authority is required",
                                "basis_identities": [],
                            },
                        ],
                    }
                    if is_user_owned_behavior(behavior_class)
                    else learning_workflow(review_candidate, baseline_analysis)
                    if behavior_class == "learning_deliberation"
                    else ready_workflow(review_candidate, baseline_analysis)
                ),
            },
        ),
        mcp_call(
            work_turn,
            candidate_submit_call,
            "candidate_manage",
            {
                "action": "submit_question_from_materiality",
                "project_id": project,
                "review_candidate_id": review_candidate,
                "dimension_id": materiality_dimension_id,
                "research_state": "research_required",
                "research_state_basis": question_content["why_repository_inspection_cannot_decide"],
                "retention_basis": "current work session",
                "bounded_summary": "Choose the operator-facing error detail boundary",
                "prompt": question_prompt,
                "why_now": question_content["material_consequence"],
                "established_facts": question_content["established_repository_facts"],
                "assumptions": [],
                "uncertainty": [question_content["why_repository_inspection_cannot_decide"]],
                "alternatives": [
                    {"key": "concise", "label": question_content["viable_alternatives"][0], "consequence": "Stable public output"},
                    {"key": "detailed", "label": question_content["viable_alternatives"][1], "consequence": "More immediate detail"},
                ],
                "recommendation_key": "concise",
                "recommendation_rationale": question_content["recommendation"],
                "trade_offs": [question_content["material_consequence"]],
                "known_limits": [],
                "what_unlocks": ["ordinary implementation work"],
                "duplicate_basis": "canonical inspection found no matching Question",
                "presentation_order": 1,
            },
        ),
        custom_output(
            work_turn,
            candidate_submit_call,
            {
                "action": "submit_question_from_materiality",
                "state": "stored",
                "review_candidate_id": review_candidate,
                "dimension_id": materiality_dimension_id,
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
        mcp_call(
            decision_turn,
            materiality_revision_call,
            "materiality_review",
            {
                "action": "revise",
                "project_id": project,
                "review_candidate_id": review_candidate,
                "rationale": "The explicit current-host Decision resolves the user-owned outcome.",
                "dimensions": materiality_dimensions(resolved=True),
            },
        ),
        custom_output(
            decision_turn,
            materiality_revision_call,
            {
                "action": "revise",
                "review_candidate_id": review_candidate,
                "review_revision": 2,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": baseline_analysis,
                "review_analysis_snapshot_id": current_analysis,
                "canonical_mutation": False,
                "workflow": ready_workflow(review_candidate, baseline_analysis),
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
                    f"{repository_cwd}/{path}": {
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
        mcp_call(
            decision_turn,
            current_analysis_call,
            "repository_analyze",
            {"project_id": project},
        ),
        custom_output(
            decision_turn,
            current_analysis_call,
            {
                "project_id": project,
                "analysis_snapshot_id": current_analysis,
                "repository_snapshot_id": current_repository,
            },
        ),
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
                "applied_decision_ids": applied_decisions,
                "verification": [
                    {
                        "state": "passed",
                        "command_label": "focused verification",
                        "command_invocation": verification_command,
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
                "pre_existing_dirty_paths": [],
                "changed_paths": work_paths,
                "applied_decision_ids": applied_decisions,
                "verification_source_ids": [verification_source],
            },
        ),
        task_complete(decision_turn),
    ]
    if behavior_class == "learning_deliberation":
        learning_response_text = (
            "Choose ordered records because deterministic invariant inspection matters more "
            "than direct lookup."
        )
        patch_index = next(
            index
            for index, value in enumerate(work_events)
            if value.get("payload", {}).get("call_id") == patch_call
        )
        work_events[patch_index:patch_index] = [
            user(
                decision_turn,
                f"{kind}-learning-user-turn-{cycle}",
                learning_response_text,
            ),
            mcp_call(
                decision_turn,
                learning_begin_call,
                "learning_deliberation",
                {
                    "action": "begin",
                    "project_id": project,
                    "review_candidate_id": review_candidate,
                    "dimension_id": materiality_dimension_id,
                    "source_operation": "naturalistic learning deliberation",
                    "problem": "Which state representation makes the invariant easiest to preserve?",
                    "established_facts": ["The state set is bounded and deterministic ordering is required."],
                },
            ),
            custom_output(
                decision_turn,
                learning_begin_call,
                {
                    "action": "begin",
                    "interaction_kind": "learning_participation",
                    "canonical_decision": False,
                    "deliberation_candidate_id": learning_candidate,
                    "materiality_review_candidate_id": review_candidate,
                    "dimension_id": materiality_dimension_id,
                    "choices": engineering_choices(repository_source)[:1],
                    "rounds": [],
                    "state": {"state": "awaiting_initial_response"},
                    "workflow": {"stage": "learning_deliberation", "blocks_ordinary_work": True},
                },
            ),
            mcp_call(
                decision_turn,
                learning_response_call,
                "learning_deliberation",
                {
                    "action": "respond_select",
                    "project_id": project,
                    "deliberation_candidate_id": learning_candidate,
                    "user_turn": learning_response_text,
                    "user_rationale": "Deterministic invariant inspection matters more than direct lookup.",
                    "selections": [{"choice_id": materiality_dimension_id, "alternative_id": "ordered-records"}],
                },
            ),
            custom_output(
                decision_turn,
                learning_response_call,
                {
                    "action": "respond_select",
                    "interaction_kind": "learning_participation",
                    "canonical_decision": False,
                    "deliberation_candidate_id": learning_candidate,
                    "rounds": [{"initial_response_source_id": learning_response_source, "response": {"kind": "selected", "selections": [{"choice_id": materiality_dimension_id, "alternative_id": "ordered-records"}]}, "user_rationale": "Deterministic invariant inspection matters more than direct lookup."}],
                    "state": {"state": "awaiting_agent_feedback"},
                    "workflow": {"stage": "learning_deliberation", "blocks_ordinary_work": True},
                },
            ),
            mcp_call(
                decision_turn,
                learning_feedback_call,
                "learning_deliberation",
                {
                    "action": "feedback",
                    "project_id": project,
                    "deliberation_candidate_id": learning_candidate,
                    "feedback": "Ordered records keep deterministic inspection explicit; bounded lookup keeps the linear scan insignificant.",
                    "recommendation_selections": [{"choice_id": materiality_dimension_id, "alternative_id": "ordered-records"}],
                    "recommendation_rationale": "The bounded size makes direct indexing unnecessary while deterministic order supports auditing.",
                },
            ),
            custom_output(
                decision_turn,
                learning_feedback_call,
                {
                    "action": "feedback",
                    "interaction_kind": "learning_participation",
                    "canonical_decision": False,
                    "deliberation_candidate_id": learning_candidate,
                    "state": {"state": "feedback_provided"},
                    "workflow": {"stage": "learning_deliberation", "blocks_ordinary_work": True},
                },
            ),
            mcp_call(
                decision_turn,
                learning_complete_call,
                "learning_deliberation",
                {"action": "complete", "project_id": project, "deliberation_candidate_id": learning_candidate},
            ),
            custom_output(
                decision_turn,
                learning_complete_call,
                {
                    "action": "complete",
                    "interaction_kind": "learning_participation",
                    "canonical_decision": False,
                    "deliberation_candidate_id": learning_candidate,
                    "state": {"state": "completed"},
                    "workflow": ready_workflow(review_candidate, baseline_analysis),
                },
            ),
        ]

    resume_turn = f"{kind}-resume-turn-{cycle}"
    resolve_call = f"{kind}-resolve-call-{cycle}"
    recall_call = f"{kind}-recall-call-{cycle}"
    inspect_call = f"{kind}-inspect-call-{cycle}"
    resume_baseline_call = f"{kind}-resume-baseline-call-{cycle}"
    resume_discovery_call = f"{kind}-resume-discovery-call-{cycle}"
    resume_materiality_call = f"{kind}-resume-materiality-call-{cycle}"
    resume_patch_call = f"{kind}-resume-patch-call-{cycle}"
    resume_current_analysis_call = f"{kind}-resume-current-analysis-call-{cycle}"
    resume_verification_call = f"{kind}-resume-verification-call-{cycle}"
    resume_checkpoint_call = f"{kind}-resume-checkpoint-call-{cycle}"
    resume_verification_command = "python3 -m unittest tests.test_resume"
    resume_patch_text = (
        "*** Begin Patch\n"
        f"*** Update File: {repository_cwd}/src/resume.rs\n"
        "@@\n+continued\n*** End Patch"
    )
    resume_events = [
        session_meta(resume_session),
        activation_message(),
        task(resume_turn),
        user(resume_turn, f"{kind}-resume-user-turn-{cycle}", resume_user_task),
        mcp_call(
            resume_turn,
            resolve_call,
            "project_resolve",
            {"repository": repository_cwd},
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
                    "canonical_repository_path": repository_cwd,
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
                "decisions": (
                    [{"identity": decision, "revision": 1, "state": "active", "choice": "concise", "rationale": None}]
                    if is_user_owned_behavior(behavior_class)
                    else []
                ),
                "open_questions": [],
                "learning_context": (
                    [
                        {
                            "identity": learning_candidate,
                            "kind": "learning_deliberation",
                            "learning_deliberation": {
                                "interaction_kind": "learning_participation",
                                "canonical_decision": False,
                                "deliberation_candidate_id": learning_candidate,
                                "materiality_review_candidate_id": review_candidate,
                                "dimension_id": materiality_dimension_id,
                                "choices": engineering_choices(repository_source)[:1],
                                "rounds": [{
                                    "initial_response_source_id": learning_response_source,
                                    "response": {"kind": "selected", "selections": [{"choice_id": materiality_dimension_id, "alternative_id": "ordered-records"}]},
                                    "user_rationale": "Deterministic invariant inspection matters more than direct lookup.",
                                    "agent_feedback": "Ordered records keep deterministic inspection explicit.",
                                }],
                                "state": {"state": "completed"},
                            },
                        }
                    ]
                    if behavior_class == "learning_deliberation"
                    else []
                ),
                "learning_context_health": {"state": "available"},
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
                    "applied_decisions": applied_decisions,
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
        mcp_call(
            resume_turn,
            resume_baseline_call,
            "repository_analyze",
            {"project_id": project},
            fallback="??",
        ),
        custom_output(
            resume_turn,
            resume_baseline_call,
            {
                "project_id": project,
                "analysis_snapshot_id": resume_baseline_analysis,
                "repository_snapshot_id": resume_baseline_repository,
                "repository_source_id": resume_repository_source,
            },
        ),
        mcp_call(
            resume_turn,
            resume_discovery_call,
            "engineering_choice_discovery",
            {
                "project_id": project,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": resume_baseline_analysis,
                "source_operation": "fresh-session engineering-choice rediscovery",
                "summary": "Re-establish current technical choices after Recall.",
                "choices": engineering_choices(resume_repository_source),
            },
            fallback="??",
        ),
        custom_output(
            resume_turn,
            resume_discovery_call,
            {
                "action": "record",
                "discovery_candidate_id": resume_discovery_candidate,
                "revision": 1,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": resume_baseline_analysis,
                "choices": engineering_choices(resume_repository_source),
                "canonical_mutation": False,
                "workflow": {"stage": "materiality_review", "blocks_ordinary_work": True},
            },
        ),
        mcp_call(
            resume_turn,
            resume_materiality_call,
            "materiality_review",
            {
                "action": "record",
                "project_id": project,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": resume_baseline_analysis,
                "engineering_choice_discovery_candidate_id": resume_discovery_candidate,
                "source_operation": "fresh-session Materiality Review recomputation",
                "rationale": "Recompute current work authority after Recall and a fresh baseline.",
                "learning_participation": (
                    {
                        "state": "active",
                        "user_turn_source_id": goal_source,
                        "verbatim_statement": "I want to learn",
                    }
                    if learning_active
                    else {"state": "inactive"}
                ),
                "dimensions": materiality_dimensions(
                    resolved=is_user_owned_behavior(behavior_class),
                    source_id=resume_repository_source,
                ),
            },
            fallback="??",
        ),
        custom_output(
            resume_turn,
            resume_materiality_call,
            {
                "action": "record",
                "review_candidate_id": resume_review_candidate,
                "review_revision": 1,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": resume_baseline_analysis,
                "review_analysis_snapshot_id": resume_review_analysis,
                "canonical_mutation": False,
                "workflow": ready_workflow(
                    resume_review_candidate, resume_baseline_analysis
                ),
            },
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
                    f"{repository_cwd}/src/resume.rs": {
                        "type": "update",
                        "unified_diff": "@@ -0,0 +1 @@\n+continued\n",
                        "move_path": None,
                    }
                },
                "status": "completed",
            },
        ),
        custom_output(resume_turn, resume_patch_call, {}),
        mcp_call(
            resume_turn,
            resume_current_analysis_call,
            "repository_analyze",
            {"project_id": project},
            fallback="??",
        ),
        custom_output(
            resume_turn,
            resume_current_analysis_call,
            {
                "project_id": project,
                "analysis_snapshot_id": resume_current_analysis,
                "repository_snapshot_id": resume_current_repository,
                "repository_source_id": resume_repository_source,
            },
        ),
        command_call(resume_turn, resume_verification_call, resume_verification_command),
        custom_output(
            resume_turn,
            resume_verification_call,
            {"output": "Ran resumed tests\nOK\n", "exit_code": 0},
        ),
        mcp_call(
            resume_turn,
            resume_checkpoint_call,
            "checkpoint_record",
            {
                "project_id": project,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": resume_baseline_analysis,
                "kind": "handoff",
                "work_state": "completed",
                "state_change": "Continued the recalled implementation",
                "applied_decision_ids": applied_decisions,
                "verification": [
                    {
                        "state": "passed",
                        "command_label": "resume verification",
                        "command_invocation": resume_verification_command,
                        "exit_code": 0,
                        "termination": "exited",
                        "outcome": "resumed tests passed",
                    }
                ],
                "next_step": "Review the resumed implementation",
                "known_limits": [],
                "handoff_to": "user",
            },
            fallback="??",
        ),
        custom_output(
            resume_turn,
            resume_checkpoint_call,
            {
                "checkpoint_id": resume_checkpoint,
                "revision": 1,
                "goal_context_id": context,
                "baseline_analysis_snapshot_id": resume_baseline_analysis,
                "current_analysis_snapshot_id": resume_current_analysis,
                "baseline_repository_snapshot_id": resume_baseline_repository,
                "current_repository_snapshot_id": resume_current_repository,
                "pre_existing_dirty_paths": [],
                "changed_paths": ["src/resume.rs"],
                "applied_decision_ids": applied_decisions,
                "verification_source_ids": [verification_source],
            },
        ),
        task_complete(resume_turn),
    ]
    if not is_user_owned_behavior(behavior_class):
        question_call_ids = {
            candidate_submit_call,
            candidate_research_call,
            candidate_ready_call,
            candidate_promote_call,
            inquiry_call,
            decision_call,
            materiality_revision_call,
        }
        work_events = [
            value
            for value in work_events
            if value.get("payload", {}).get("call_id") not in question_call_ids
            and not (
                value.get("payload", {}).get("type") == "user_message"
                and "decision-user-turn"
                in str(value.get("payload", {}).get("client_id", ""))
            )
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
        [blob(verification_source), blob(project), integer(1), text("command_execution"), text("focused verification"), null(), text("sha256:" + hashlib.sha256(verification_command.encode("utf-8")).hexdigest()), null(), integer(0), text("exited"), text("command"), text("current-host-reported-command"), text("agent"), text("codex"), text("available"), integer(5)],
        [blob(repository_source), blob(project), integer(1), text("repository_snapshot"), text(revision), text(revision), null(), null(), null(), null(), text("repository"), text("local-repository-observer"), text("agent"), text("codex"), text("available"), integer(6)],
        *(
            [[blob(learning_response_source), blob(project), integer(1), text("current_host_user_turn"), text("Choose ordered records because deterministic invariant inspection matters more than direct lookup."), null(), text("codex"), text(work_session), null(), null(), text("user"), text("fixture-user"), null(), null(), text("available"), integer(7)]]
            if behavior_class == "learning_deliberation"
            else []
        ),
    ]
    tables = [
        table("sources", source_columns, sources),
        table("questions", ["id", "project_id", "revision", "terminal_outcome", "created_at", "updated_at"], [[blob(question), blob(project), integer(1), text("answered"), integer(1), integer(1)]] if is_user_owned_behavior(behavior_class) else []),
        table("question_revisions", ["question_id", "revision", "project_id", "prompt_basis", "source_basis", "dependencies", "alternatives", "recommendation_key", "recommendation_rationale", "recommendation_sources", "trade_offs", "uncertainty", "material_scope", "materiality", "presentation_order", "why_it_matters_now", "established_facts", "assumptions", "known_limits", "answer_unlocks", "allowed_dispositions", "research_state", "recorded_at"], [[blob(question), integer(1), blob(project), text(question_prompt), blob(encoded_source_ids([goal_source]).hex()), blob(encoded_strings([])), blob(encoded_alternatives(question_content["viable_alternatives"])), text("concise"), text(question_content["recommendation"]), blob(encoded_source_ids([goal_source]).hex()), blob(encoded_strings([question_content["material_consequence"]])), blob(encoded_strings([])), blob(encoded_strings([question_content["user_owned_dimension"], f"work-authority:{materiality_dimension_id}", f"work-authority:{secondary_materiality_dimension_id}"])), text("material"), integer(1), text(question_content["material_consequence"]), blob(encoded_established_facts(question_content["established_repository_facts"])), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings(["ordinary implementation work"])), blob(encoded_strings(["deferred"])), text("researched"), integer(1)]] if is_user_owned_behavior(behavior_class) else []),
        table("question_response_sources", ["project_id", "question_id", "question_revision", "source_id", "recorded_at"], [[blob(project), blob(question), integer(1), blob(user_source), integer(1)]] if is_user_owned_behavior(behavior_class) else []),
        table("question_decision_history_witnesses", ["project_id", "question_id", "question_revision", "root_decision_id", "terminal_outcome", "response_source_id", "response_authority", "creation_kind", "created_at"], [[blob(project), blob(question), integer(1), blob(decision), text("answered"), blob(user_source), text("current_host_user_turn"), text("alternative"), integer(1)]] if is_user_owned_behavior(behavior_class) else []),
        table("decisions", ["id", "project_id", "revision", "question_id", "question_revision", "user_turn_source_id", "user_authority", "choice_kind", "choice_value", "user_rationale", "displayed_alternatives", "recommendation_key", "recommendation_rationale", "recommendation_sources", "applicability_paths", "applicability_components", "applicability_work_contexts", "assumptions", "revisit_triggers", "recorded_at"], [[blob(decision), blob(project), integer(1), blob(question), integer(1), blob(user_source), text("current_host_user_turn"), text("alternative"), text("concise"), null(), blob(encoded_alternatives(question_content["viable_alternatives"])), text("concise"), text(question_content["recommendation"]), blob(encoded_source_ids([goal_source]).hex()), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), integer(1)]] if is_user_owned_behavior(behavior_class) else []),
        table("context_items", ["id", "project_id", "revision", "role", "statement", "provenance_role", "author_kind", "author_identity", "applicability_paths", "applicability_components", "applicability_work_contexts", "recorded_at"], [[blob(context), blob(project), integer(1), text("goal"), text(work_user_task), text("user_statement"), text("user"), text("fixture-user"), blob(encoded_strings([])), blob(encoded_strings([])), blob(encoded_strings([])), integer(1)]]),
        table("context_item_sources", ["project_id", "context_item_id", "source_id", "position"], [[blob(project), blob(context), blob(goal_source), integer(0)]]),
        table("checkpoints", ["id", "project_id", "revision", "checkpoint_kind", "goal", "work_state", "state_change", "changed_paths", "user_review", "user_review_source_id", "user_acceptance", "user_acceptance_source_id", "known_limits", "non_goals", "next_step", "handoff_to", "recorded_at"], [[blob(checkpoint), blob(project), integer(1), text("handoff"), text(work_user_task), text("paused"), text("Updated the bounded implementation and test"), blob(encoded_strings(work_paths)), text("not_requested"), null(), text("not_requested"), null(), blob(encoded_strings([])), blob(encoded_strings([])), text(next_step), text("next Codex session"), integer(1)]]),
        table("checkpoint_source_relations", ["project_id", "checkpoint_id", "relation_kind", "source_id", "position"], [[blob(project), blob(checkpoint), text("supported_by"), blob(goal_source), integer(0)], [blob(project), blob(checkpoint), text("changed_basis"), blob(changed_source_one), integer(0)], [blob(project), blob(checkpoint), text("changed_basis"), blob(changed_source_two), integer(1)]]),
        table(
            "checkpoint_decisions",
            ["project_id", "checkpoint_id", "decision_id", "position"],
            [[blob(project), blob(checkpoint), blob(decision), integer(0)]]
            if is_user_owned_behavior(behavior_class)
            else [],
        ),
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
        "format_version": 7,
        "kind": "volicord-context-bundle",
        "payload": payload,
    }
    write_json(bundle_path, bundle)
    support_prefix = f"{kind}-{cycle}"
    generated_documents: dict[str, Any] = {}
    for document_kind in (
        "project-architecture-guide",
        "decision-report",
        "implementation-plan",
        "handoff-resume",
    ):
        formats: dict[str, Any] = {}
        for format_name, suffix in (("markdown", "md"), ("html", "html")):
            path = evidence_directory / f"{support_prefix}-{document_kind}.{suffix}"
            path.write_text(
                f"{document_kind} {format_name} fixture evidence\n",
                encoding="utf-8",
            )
            formats[format_name] = {
                "status": "passed",
                "relative_evidence_path": path.name,
                "bytes": path.stat().st_size,
                "sha256": sha256(path),
            }
        generated_documents[document_kind] = {
            "status": "passed",
            "formats": formats,
        }
    documents_summary_path = evidence_directory / f"{support_prefix}-documents-summary.json"
    write_json(documents_summary_path, {
        "kind": "phase8_generated_document_evidence_summary",
        "schema_version": 1,
        "language": "en",
        "status": "passed",
        "required_document_kinds": list(generated_documents),
        "documents": generated_documents,
    })
    snapshot_html_path = evidence_directory / f"{support_prefix}-viewer-snapshot.html"
    snapshot_html_path.write_text(
        '<!doctype html><html lang="en"><body data-viewer-mode="snapshot">fixture</body></html>\n',
        encoding="utf-8",
    )
    snapshot_summary_path = evidence_directory / f"{support_prefix}-viewer-snapshot-summary.json"
    write_json(snapshot_summary_path, {
        "kind": "phase8_viewer_snapshot_evidence_summary",
        "schema_version": 1,
        "status": "passed",
        "project_id": project,
        "candidate_head": git_head(ROOT),
        "repository_class": kind,
        "cycle": cycle,
        "locale": "en",
        "requested_language": "en",
        "relative_evidence_path": snapshot_html_path.name,
        "bytes": snapshot_html_path.stat().st_size,
        "sha256": sha256(snapshot_html_path),
    })
    runtime_summary_path = evidence_directory / f"{support_prefix}-runtime-summary.json"
    write_json(runtime_summary_path, {
        "kind": "phase8_bounded_runtime_summary",
        "runtime_home_bytes": 1024,
        "derived_analysis_bytes": 128,
        "managed_file_inventory": [],
        "repository_config_present": True,
        "repository_ownership_manifest_present": True,
        "work_session_start_activation_observed": True,
        "resume_session_start_activation_observed": True,
        "content_included": False,
    })
    activation_summary_path = evidence_directory / f"{support_prefix}-activation-summary.json"
    write_json(activation_summary_path, {
        "kind": "phase8_dogfood_activation_summary",
        "repository_class": kind,
        "cycle": cycle,
        "repository_config_present": True,
        "repository_ownership_manifest_present": True,
        "work_session_start_activation_observed": True,
        "resume_session_start_activation_observed": True,
    })
    return {
        "kind": "phase8_cycle_descriptor",
        "producer": "volicord_phase8_codex_event_normalizer",
        "_evidence_file_sha256": "0" * 64,
        "_evidence_directory": str(evidence_directory),
        "repository_class": kind,
        "cycle": cycle,
        "behavior_class": behavior_class,
        "repository_revision": revision,
        "work_user_task": work_user_task,
        "fresh_resume_user_task": resume_user_task,
        "work_scope": {
            "affected_paths": work_paths,
            "user_visible_behavior": is_user_owned_behavior(behavior_class),
            "boundary_kind": "component",
        },
        "evaluation_basis": evaluation_basis,
        "behavior_review": fixture_behavior_review(behavior_class),
        "evidence": {
            "captures": {
                "work": {"file": work_capture.name, "sha256": sha256(work_capture)},
                "resume": {"file": resume_capture.name, "sha256": sha256(resume_capture)},
            },
            "canonical_bundle": {"file": bundle_path.name, "sha256": sha256(bundle_path)},
            "runtime_summary": {"file": runtime_summary_path.name, "sha256": sha256(runtime_summary_path)},
            "activation_summary": {"file": activation_summary_path.name, "sha256": sha256(activation_summary_path)},
            "generated_documents": {"file": documents_summary_path.name, "sha256": sha256(documents_summary_path)},
            "viewer_snapshot": {"file": snapshot_summary_path.name, "sha256": sha256(snapshot_summary_path)},
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
        ("two terminal newlines", descriptor_task + "\n\n", True),
        ("mixed terminal CR/LF", descriptor_task + "\r\n\r\n\r", True),
        ("trailing space", descriptor_task + " ", False),
        ("space before terminal LF", descriptor_task + " \n", False),
        ("leading whitespace", " " + descriptor_task, False),
        ("interior difference", descriptor_task.replace("exact", "altered"), False),
        ("extra instruction", descriptor_task + "\nextra instruction", False),
        ("terminal tab", descriptor_task + "\t", False),
        ("terminal Unicode whitespace", descriptor_task + "\u00a0", False),
    )
    for label, captured, expected in transport_identity_cases:
        if codex_user_turn_transport_identity_matches(captured, descriptor_task) is not expected:
            raise AssertionError(f"Codex user-turn transport identity mishandled {label}")
    if not codex_user_turn_transport_identity_matches("line one\r\nline two\r\n", "line one\nline two"):
        raise AssertionError("Codex user-turn transport identity did not normalize internal CRLF")
    if codex_user_turn_transport_identity_matches(descriptor_task, None):
        raise AssertionError("non-text descriptor qualified as Codex user-turn identity")
    revision = "0" * 40
    candidate_revision = git_head(ROOT)
    if candidate_revision is None:
        raise AssertionError("dogfood self-test could not resolve the current candidate")
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
            "process_count": 1,
            "open_file_descriptor_count": 8,
            "runtime_file_count": 6,
            "stale_temporary_file_count": 0,
            "operation_latency_ms": latency,
        }
        for runtime, derived, document, latency in (
            (100, 20, 10, 20),
            (120, 24, 10, 22),
            (120, 24, 10, 21),
            (120, 24, 10, 22),
        )
    ]
    stable_resources = repeated_resource_conclusion(stable_rounds)
    if (
        stable_resources["status"] != "passed"
        or stable_resources["unexplained_cumulative_growth_observed"] is not False
        or stable_resources["diagnostic"] is not None
    ):
        raise AssertionError("stable repeated resources did not qualify")
    growing_rounds = json.loads(json.dumps(stable_rounds))
    for index, round_value in enumerate(growing_rounds):
        round_value["runtime_home_bytes"] = 100 + (index * 10)
    growing_resources = repeated_resource_conclusion(growing_rounds)
    if (
        growing_resources["status"] != "failed"
        or growing_resources["unexplained_cumulative_growth_observed"] is not True
        or growing_resources["diagnostic"].get("metric") != "runtime_home_bytes"
    ):
        raise AssertionError("unexplained cumulative resource growth qualified")
    for metric in (
        "open_file_descriptor_count",
        "runtime_file_count",
        "operation_latency_ms",
    ):
        degraded_rounds = json.loads(json.dumps(stable_rounds))
        for index, round_value in enumerate(degraded_rounds):
            round_value[metric] = 10 + index
        conclusion = repeated_resource_conclusion(degraded_rounds)
        if (
            conclusion["status"] != "failed"
            or metric not in conclusion["cumulative_growth_metrics"]
            or conclusion["diagnostic"].get("metric") != metric
        ):
            raise AssertionError(f"cumulative {metric} degradation qualified")
    leaked_process_rounds = json.loads(json.dumps(stable_rounds))
    leaked_process_rounds[-1]["process_count"] = 2
    leaked_processes = repeated_resource_conclusion(leaked_process_rounds)
    if (
        leaked_processes["status"] != "failed"
        or leaked_processes["diagnostic"].get("round") != 4
        or leaked_processes["diagnostic"].get("metric") != "process_count"
    ):
        raise AssertionError("descendant process leakage qualified")
    stale_temporary_rounds = json.loads(json.dumps(stable_rounds))
    stale_temporary_rounds[-1]["stale_temporary_file_count"] = 1
    stale_temporary = repeated_resource_conclusion(stale_temporary_rounds)
    if (
        stale_temporary["status"] != "failed"
        or stale_temporary["diagnostic"].get("round") != 4
        or stale_temporary["diagnostic"].get("metric")
        != "stale_temporary_file_count"
    ):
        raise AssertionError("stale temporary file accumulation qualified")
    insufficient = repeated_resource_conclusion(stable_rounds[:2])
    if (
        insufficient["status"] != "unsupported"
        or insufficient["diagnostic"].get("observed", {}).get("round_count") != 2
    ):
        raise AssertionError("unobserved repeated resources were treated as measured")
    incomplete_rounds = json.loads(json.dumps(stable_rounds))
    incomplete_rounds[0]["operations"].pop("document_projection")
    incomplete = repeated_resource_conclusion(incomplete_rounds)
    if (
        incomplete["status"] != "unsupported"
        or incomplete["diagnostic"].get("round") != 1
        or incomplete["diagnostic"].get("observed", {}).get("missing")
        != ["document_projection"]
    ):
        raise AssertionError("incomplete repeated-operation evidence qualified")
    failed_rounds = json.loads(json.dumps(stable_rounds))
    failed_rounds[1]["operations"]["repository_analysis"]["exit_code"] = 7
    failed_operation = repeated_resource_conclusion(failed_rounds)
    if (
        failed_operation["status"] != "failed"
        or failed_operation["diagnostic"].get("round") != 2
        or failed_operation["diagnostic"].get("operation") != "repository_analysis"
        or failed_operation["diagnostic"].get("observed", {}).get("exit_code") != 7
    ):
        raise AssertionError("failed repeated resource operation qualified")

    def stable_self_test_resource_observer(
        _runtime: Path,
        document_output_bytes: int | None,
        _operations: tuple[dict[str, Any], ...],
    ) -> dict[str, Any]:
        return {
            "runtime_home_bytes": 0,
            "derived_state_bytes": 0,
            "document_output_bytes": document_output_bytes,
            "process_count": 1,
            "open_file_descriptor_count": 8,
            "runtime_file_count": 0,
            "stale_temporary_file_count": 0,
            "operation_latency_ms": 20,
        }

    def install_no_replace_resource_fake(cycle_root: Path, kind: str) -> Path:
        fake = cycle_root / "work" / kind / "prefix/bin/volicord"
        fake.parent.mkdir(parents=True, exist_ok=True)
        fake.symlink_to(STRICT_FAKE_CLI)
        (cycle_root / "work" / kind / "repository").mkdir()
        return fake

    rehearsal_root = evidence_directory / "rehearsal-pass"
    install_no_replace_resource_fake(rehearsal_root, "volicord")
    strict_repository = rehearsal_root / "work/volicord/repository"
    for obsolete in (
        ["repair", "11" * 16, "derived-analysis"],
        ["reindex", "11" * 16],
        ["documents", "export", "11" * 16, "project-architecture-guide", "html", "old.html", "en"],
        ["doctor", "--repository", str(strict_repository), "repair"],
        ["unknown"],
    ):
        rejected = subprocess.run(
            [str(rehearsal_root / "work/volicord/prefix/bin/volicord"), *obsolete],
            text=True,
            capture_output=True,
            check=False,
        )
        if rejected.returncode != 2:
            raise AssertionError(f"strict repeated-resource fake accepted obsolete argv: {obsolete}")
    rehearsal = repeated_resource_rehearsal(
        "volicord",
        rehearsal_root,
        v11.Recorder(evidence_directory / "rehearsal-pass-processes"),
        os.environ.copy(),
        "11" * 16,
        definition["resource_qualification"]["repeated_resource_repetition_count"],
        resource_observer=stable_self_test_resource_observer,
    )
    rehearsal_destination = (
        rehearsal_root
        / "work/volicord/repeated-resource/project-architecture-guide.html"
    )
    if (
        rehearsal["status"] != "passed"
        or rehearsal.get("observation_mode") != SELF_TEST_RESOURCE_OBSERVATION_MODE
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
        resource_observer=stable_self_test_resource_observer,
    )
    if (
        preexisting["status"] != "failed"
        or preexisting["conclusion"] != "rehearsal_destination_preexisting"
        or preexisting["diagnostic"].get("round") != 0
        or preexisting["diagnostic"].get("operation") != "document_projection"
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
        resource_observer=stable_self_test_resource_observer,
    )
    failed_export_destination = (
        failed_export_root
        / "work/polyglot-medium/repeated-resource/project-architecture-guide.html"
    )
    if (
        failed_export["status"] != "failed"
        or failed_export["conclusion"]
        != "failed_document_export_created_unowned_destination"
        or failed_export["diagnostic"].get("round") != 1
        or failed_export["diagnostic"].get("operation") != "document_projection"
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
        failed_checks = sorted(
            key for key, value in external_result["checks"].items() if value != "passed"
        )
        raise AssertionError(
            "external sanitized process evidence did not qualify: "
            + ", ".join(failed_checks)
        )
    hidden_fixture = real_session_fixture(
        "volicord",
        2,
        revision,
        evidence_directory,
        behavior_class="hidden_user_owned_decision",
    )
    hidden_result = real_session_evidence(
        hidden_fixture,
        kind="volicord",
        cycle=2,
        repository_revision=revision,
    )
    if (
        hidden_result["status"] != "passed"
        or hidden_result["checks"]["hidden_material_discovery_order"] != "passed"
        or hidden_result["checks"]["decision_provenance_when_required"] != "passed"
    ):
        raise AssertionError("hidden material-decision discovery did not qualify")
    uninterrupted_fixture = real_session_fixture(
        "small-python",
        1,
        revision,
        evidence_directory,
        behavior_class="research_or_no_question",
    )
    uninterrupted_capture = load_codex_capture(
        evidence_directory
        / uninterrupted_fixture["evidence"]["captures"]["work"]["file"]
    )
    uninterrupted_result = real_session_evidence(
        uninterrupted_fixture,
        kind="small-python",
        cycle=1,
        repository_revision=revision,
    )
    if (
        len(uninterrupted_capture.user_turns) != 1
        or uninterrupted_result["status"] != "passed"
        or uninterrupted_result["checks"]["appropriate_inquiry_outcome"] != "passed"
    ):
        raise AssertionError("small-Python no-question cycle manufactured a user interruption")
    if (
        "recall" in external_fixture["fresh_resume_user_task"].casefold()
        or external_result["checks"]["recall_precedes_inspection_and_continuation"]
        != "passed"
    ):
        raise AssertionError("automatic Recall was not observed from a plain resume request")
    ask_basis = external_result["inquiry_behavior_basis"]["ask_user_question_basis"]
    if (
        ask_basis.get("exact_preferred_expression_required") is not False
        or ask_basis.get("ask_user_invariants", {}).get("material_consequence") is not True
        or external_result["checks"]["decision_provenance_when_required"] != "passed"
    ):
        raise AssertionError("ASK_USER invariants or current-host provenance were not qualified")
    representative_capture = load_codex_capture(CURRENT_MCP_FIXTURE)
    representative_calls = representative_capture.calls("context_record")
    if (
        len(representative_calls) != 1
        or representative_calls[0].outcome != "succeeded"
        or representative_calls[0].result.get("context_item_id") != "08" * 16
        or b"text(JSON.stringify(x))" not in CURRENT_MCP_FIXTURE.read_bytes()
    ):
        raise AssertionError("current JSON.stringify wrapper fixture did not normalize from MCP completion")
    execution_capture = load_codex_capture(CURRENT_EXECUTION_FIXTURE)
    if len(execution_capture.commands) != 5:
        raise AssertionError("current execution fixture did not produce one observation per command")
    expected_execution = [
        ("python3 -m unittest tests.test_template", 0, "template output\n", False, 0),
        ("python3 -m unittest tests.test_json_status", 2, "json status output\n", False, 0),
        ("python3 -m unittest tests.test_json_result", 0, "json result output\n", False, 0),
        ("rg --files src && git status --short", 0, "src/lib.rs\n", False, 0),
        ("python3 -m unittest tests.test_empty", 3, "", True, 1),
    ]
    observed_execution = [
        (
            command.parsed_command.get("cmd"),
            command.exit_code,
            command.output,
            command.output_was_empty,
            command.group_index,
        )
        for command in execution_capture.commands
        if isinstance(command.parsed_command, dict)
    ]
    if observed_execution != expected_execution or any(
        command.termination != "exited" for command in execution_capture.commands
    ):
        raise AssertionError("current execution fixture lost command order, outcome, or output state")
    if [item.paths for item in execution_capture.path_observations] != [
        ("src/lib.rs", "tests/lib.rs")
    ]:
        raise AssertionError("authoritative bounded patch completion paths were not normalized")
    if not command_is_repository_inspection(
        {"cmd": "rg --files src && git status --short"}
    ):
        raise AssertionError("bounded compound repository inspection was not classified")
    if command_is_repository_inspection(
        {"cmd": "rg --files src && python3 scripts/rewrite.py"}
    ):
        raise AssertionError("compound command with a non-inspection action was misclassified")
    compacted_fixture_root = evidence_directory / "compacted-fresh-thread"
    compacted_fixture_root.mkdir()
    compacted_fixture = real_session_fixture(
        "volicord", 1, revision, compacted_fixture_root
    )
    compacted_path = (
        evidence_directory
        / "compacted-fresh-thread"
        / compacted_fixture["evidence"]["captures"]["work"]["file"]
    )
    compacted_events = [
        json.loads(line) for line in compacted_path.read_text(encoding="utf-8").splitlines()
    ]
    insertion = next(
        index
        for index, value in enumerate(compacted_events)
        if value.get("payload", {}).get("call_id") == "volicord-status-call-1"
        and value.get("payload", {}).get("type") == "custom_tool_call_output"
    ) + 1
    compacted_events.insert(
        insertion,
        {
            "timestamp": "2026-08-15T00:00:00Z",
            "type": "event_msg",
            "payload": {"type": "context_compacted"},
        },
    )
    compacted_path.write_text(
        "".join(json.dumps(value, separators=(",", ":")) + "\n" for value in compacted_events),
        encoding="utf-8",
    )
    compacted_capture = load_codex_capture(compacted_path)
    if (
        not compacted_capture.fresh_user_thread
        or len(compacted_capture.compacted_sequences) != 1
        or not compacted_capture.task_sequences
        or not compacted_capture.completed_task_sequences
        or not compacted_capture.commands
        or not compacted_capture.path_observations
        or not (
            compacted_capture.task_sequences[0]
            < compacted_capture.compacted_sequences[0]
            < compacted_capture.completed_task_sequences[-1]
        )
    ):
        raise AssertionError("automatic mid-session compaction invalidated or disappeared from a fresh thread")
    for label, meta_update in (
        ("forked", {"forked_from_id": "parent-session"}),
        ("non-user", {"thread_source": "subagent"}),
    ):
        negative_events = json.loads(json.dumps(compacted_events))
        negative_events[0]["payload"].update(meta_update)
        negative_path = evidence_directory / f"{label}-compacted-thread.jsonl"
        negative_path.write_text(
            "".join(json.dumps(value, separators=(",", ":")) + "\n" for value in negative_events),
            encoding="utf-8",
        )
        negative_capture = load_codex_capture(negative_path)
        if negative_capture.fresh_user_thread or len(negative_capture.compacted_sequences) != 1:
            raise AssertionError(f"{label} compacted thread qualified as fresh")
    valid_descriptor = {
        "kind": "phase8_cycle_descriptor",
        "repository_class": "volicord",
        "cycle": 1,
        "behavior_class": "explicit_user_owned_decision",
        "repository_revision": revision,
        "work_user_task": fixture_work_user_task("volicord", "explicit_user_owned_decision"),
        "fresh_resume_user_task": (
            "Continue the validation-adapter improvement from the current project state."
        ),
        "work_scope": {
            "affected_paths": ["src/existing.rs", "tests/existing.rs"],
            "user_visible_behavior": True,
            "boundary_kind": "component",
        },
        "evaluation_basis": fixture_evaluation_basis("explicit_user_owned_decision"),
        "behavior_review": fixture_behavior_review("explicit_user_owned_decision"),
    }
    if cycle_descriptor_errors(valid_descriptor):
        raise AssertionError("valid naturalistic plain-task descriptor was rejected")
    cycle_two_same_behavior = json.loads(json.dumps(valid_descriptor))
    cycle_two_same_behavior["cycle"] = 2
    if cycle_descriptor_errors(cycle_two_same_behavior):
        raise AssertionError("logical cycle numbering still determines behavior class")
    hidden_descriptor = {
        **json.loads(json.dumps(valid_descriptor)),
        "cycle": 2,
        "behavior_class": "hidden_user_owned_decision",
        "work_user_task": fixture_work_user_task("volicord", "hidden_user_owned_decision"),
        "evaluation_basis": fixture_evaluation_basis("hidden_user_owned_decision"),
        "behavior_review": fixture_behavior_review("hidden_user_owned_decision"),
    }
    if cycle_descriptor_errors(hidden_descriptor):
        raise AssertionError("valid hidden user-owned descriptor was rejected")
    disclosed_hidden = json.loads(json.dumps(hidden_descriptor))
    disclosed_hidden["work_user_task"] += " This policy is unsettled and the user must choose."
    if not any(
        "statically telegraphs" in error
        for error in cycle_descriptor_errors(disclosed_hidden)
    ):
        raise AssertionError("hidden descriptor with a disclosed unresolved policy qualified")
    for non_user_class in BEHAVIOR_CLASSES[2:]:
        review_errors = behavior_review_errors(
            fixture_behavior_review(non_user_class),
            non_user_class,
        )
        if review_errors:
            raise AssertionError(
                f"{non_user_class} gained mandatory user-decision ceremony: {review_errors}"
            )
    bypassable = json.loads(json.dumps(hidden_descriptor))
    bypassable_counterfactual = bypassable["behavior_review"]["independent_review"][
        "counterfactual_review"
    ]
    bypassable_counterfactual["no_question_approaches"][0].update(
        {
            "task_satisfaction": "fully_satisfies_without_user_owned_outcome",
            "assessment": (
                "A narrower diagnostics-only change fully satisfies the frozen task without selecting the claimed public policy."
            ),
        }
    )
    bypassable_errors = cycle_descriptor_errors(bypassable)
    if not any("defensible no-question path" in error for error in bypassable_errors):
        raise AssertionError("bypassable hidden user-owned descriptor qualified")
    delegated_hidden = json.loads(json.dumps(hidden_descriptor))
    delegated_hidden["behavior_review"]["independent_review"]["counterfactual_review"][
        "outcome_within_delegated_authority"
    ] = True
    if not any(
        "outcome_within_delegated_authority" in error
        for error in cycle_descriptor_errors(delegated_hidden)
    ):
        raise AssertionError("delegated outcome qualified as a hidden user-owned decision")
    fact_settled_hidden = json.loads(json.dumps(hidden_descriptor))
    fact_settled_hidden["behavior_review"]["independent_review"]["counterfactual_review"][
        "repository_facts_settle_outcome"
    ] = True
    if not any(
        "repository_facts_settle_outcome" in error
        for error in cycle_descriptor_errors(fact_settled_hidden)
    ):
        raise AssertionError("repository-settled outcome qualified as a hidden user-owned decision")
    disagreement = json.loads(json.dumps(valid_descriptor))
    agreement = disagreement["behavior_review"]["independent_review"][
        "fact_authority_agreement"
    ]
    agreement.update(
        {
            "status": "unresolved_conflict",
            "conflicts": [
                "Evaluator and reviewer disagree whether the active contract delegates the public outcome."
            ],
            "resolution_basis": "No inspectable owner evidence has resolved the disagreement yet.",
        }
    )
    disagreement_errors = cycle_descriptor_errors(disagreement)
    if not any("disagreement blocks sealing" in error for error in disagreement_errors):
        raise AssertionError("unresolved evaluator/reviewer disagreement qualified")
    classification_mismatch = json.loads(json.dumps(hidden_descriptor))
    mismatch_independent = classification_mismatch["behavior_review"][
        "independent_review"
    ]
    mismatch_provisional = mismatch_independent["provisional_review"]
    mismatch_provisional.update(
        {
            "classification": "research_or_no_question",
            "materiality_conclusion": "no_user_owned_material_outcome",
            "material_outcome_unavoidable": False,
            "operator_prompt_does_not_disclose_material_outcome": None,
        }
    )
    if blind_first_review_errors(
        mismatch_independent["review_preparation"], mismatch_provisional, 1
    ):
        raise AssertionError("well-formed evaluator-wrong provisional review failed blind validation")
    mismatch_comparison = mismatch_independent["classification_comparison"]
    mismatch_comparison.update(
        {
            "provisional_classification": "research_or_no_question",
            "evaluator_classification": "hidden_user_owned_decision",
            "disagreements": [
                "classification",
                "materiality_conclusion",
                "material_outcome_unavoidable",
                "operator_prompt_disclosure",
            ],
            "resolution_basis": (
                "The cited active-owner evidence establishes the hidden material outcome after reveal."
            ),
        }
    )
    mismatch_comparison["status"] = "agreed"
    false_agreement_errors = cycle_descriptor_errors(classification_mismatch)
    if not any("cannot be marked agreed" in error for error in false_agreement_errors):
        raise AssertionError("classification mismatch masqueraded as agreement")
    mismatch_comparison["status"] = "unresolved_conflict"
    unresolved_classification_errors = cycle_descriptor_errors(classification_mismatch)
    if not any(
        "disagreement blocks sealing" in error
        for error in unresolved_classification_errors
    ):
        raise AssertionError("unresolved classification/materiality mismatch qualified")
    mismatch_comparison["status"] = "resolved_from_evidence"
    resolved_classification_errors = cycle_descriptor_errors(classification_mismatch)
    if resolved_classification_errors:
        raise AssertionError(
            "evidence-resolved classification/materiality mismatch was rejected: "
            f"{resolved_classification_errors}"
        )
    for label, mutation in (
        ("missing work task", lambda value: value.pop("work_user_task")),
        ("missing evaluation basis", lambda value: value.pop("evaluation_basis")),
        ("missing behavior review", lambda value: value.pop("behavior_review")),
        (
            "scripted resume",
            lambda value: value.update({"fresh_resume_user_task": "Invoke Recall before continuing."}),
        ),
        ("obsolete reserved scope", lambda value: value.update({"resume_change_scope": ["src/resume.rs"]})),
        (
            "repository fact class mismatch",
            lambda value: value["behavior_review"].update(
                {"classification": "research_or_no_question"}
            ),
        ),
        (
            "cycle outside private assignment range",
            lambda value: value.update({
                "cycle": CYCLE_COUNT_BY_REPOSITORY[value["repository_class"]] + 1
            }),
        ),
        (
            "unaccepted independent review",
            lambda value: value["behavior_review"]["independent_review"].update(
                {"status": "pending"}
            ),
        ),
        (
            "obsolete independent review form",
            lambda value: value["behavior_review"].update(
                {
                    "independent_review": {
                        "status": "accepted",
                        "reviewer_role": "campaign_preparation_independent_reviewer",
                        "basis": "The old form cannot seal.",
                    }
                }
            ),
        ),
    ):
        invalid_descriptor = json.loads(json.dumps(valid_descriptor))
        mutation(invalid_descriptor)
        errors = cycle_descriptor_errors(invalid_descriptor)
        expected_error = {
            "missing work task": "work_user_task",
            "missing evaluation basis": "evaluation_basis",
            "missing behavior review": "behavior_review",
            "scripted resume": "Recall",
            "obsolete reserved scope": "obsolete field",
            "repository fact class mismatch": "classification",
            "cycle outside private assignment range": "private assignment for its repository class",
            "unaccepted independent review": "accepted independent review",
            "obsolete independent review form": "current independent review fields",
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
    json_result_parsed = parse_custom_call(
        f"const r=await tools.exec_command({command_arguments});\n"
        "text(JSON.stringify(r));\n"
    )
    template_exit_parsed = parse_custom_call(
        f"const r=await tools.exec_command({command_arguments});\n"
        "text(r.output); text(`exit=${r.exit_code}`);\n"
    )
    promise_prefix_parsed = parse_custom_call(
        "const rs=await Promise.all(["
        f"tools.exec_command({command_arguments}),"
        f"tools.exec_command({command_arguments})]);"
        "rs.forEach((r,i)=>text(`OUT${i+1} exit=${r.exit_code}\\n${r.output}`));"
    )
    promise_loop_parsed = parse_custom_call(
        "const results=await Promise.all(["
        f"tools.exec_command({command_arguments}),"
        f"tools.exec_command({command_arguments})]);"
        "for(let i=0;i<results.length;i++){"
        "text(`CMD${i+1}\\n${results[i].output}\\nEXIT ${results[i].exit_code}`);}"
    )
    if (
        json_result_parsed is None
        or json_result_parsed.output_mode != "result"
        or template_exit_parsed is None
        or template_exit_parsed.output_mode != "template_exit"
        or promise_prefix_parsed is None
        or promise_prefix_parsed.output_mode != "indexed_prefix_one"
        or promise_loop_parsed is None
        or promise_loop_parsed.output_mode != "indexed_suffix_one"
    ):
        raise AssertionError("bounded current command forwarding variants were not recognized")
    dynamic_promise = (
        "const calls=[" + command_arguments + "];"
        "const rs=await Promise.all(calls.map(x=>tools.exec_command(x)));"
        "rs.forEach((r,i)=>text(`R${i}\\n${r.output}\\nexit=${r.exit_code}`));"
    )
    if parse_custom_call(dynamic_promise) is not None:
        raise AssertionError("dynamically generated Promise.all commands were accepted")
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
            candidate_revision,
            external_fixture,
            descriptor_identity,
            positive_work_capture,
        )
    except ValueError as error:
        if "no machine-observable terminal work blocker" not in str(error):
            raise
    else:
        raise AssertionError("positive work session converted into an early-stop failure")

    non_question_fixture = real_session_fixture(
        "small-python",
        1,
        revision,
        evidence_directory,
        behavior_class="research_or_no_question",
    )
    non_question_capture_path = (
        evidence_directory
        / non_question_fixture["evidence"]["captures"]["work"]["file"]
    )
    non_question_capture = load_codex_capture(non_question_capture_path)
    non_question_descriptor_identity = hashlib.sha256(
        json.dumps(non_question_fixture, sort_keys=True).encode("utf-8")
    ).hexdigest()
    try:
        build_work_blocker_result(
            candidate_revision,
            non_question_fixture,
            non_question_descriptor_identity,
            non_question_capture,
        )
    except ValueError as error:
        if "no machine-observable terminal work blocker" not in str(error):
            raise
    else:
        raise AssertionError("correct non-question work was treated as a blocker")
    non_question_zero_path = evidence_directory / "zero-small-python-non-question-work.jsonl"
    non_question_zero_events = [
        json.loads(line)
        for line in non_question_capture_path.read_text(encoding="utf-8").splitlines()
    ]
    non_question_zero_events = [
        value
        for value in non_question_zero_events
        if value.get("payload", {}).get("type") != "mcp_tool_call_end"
    ]
    non_question_zero_path.write_text(
        "".join(
            json.dumps(value, separators=(",", ":")) + "\n"
            for value in non_question_zero_events
        ),
        encoding="utf-8",
    )
    non_question_blocker = build_work_blocker_result(
        candidate_revision,
        non_question_fixture,
        non_question_descriptor_identity,
        load_codex_capture(non_question_zero_path),
    )
    if any(
        check in non_question_blocker["failed_checks"]
        for check in USER_DECISION_BLOCKER_CHECKS
    ):
        raise AssertionError("non-question blocker manufactured Question/Decision requirements")

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
        candidate_revision,
        external_fixture,
        descriptor_identity,
        zero_workflow_capture,
    )
    if (
        blocker_result["kind"] != "phase8_dogfood_blocker_result"
        or blocker_result["failed_checks"]
        != [
            name
            for name in (*WORK_BLOCKER_CHECKS, *USER_DECISION_BLOCKER_CHECKS)
            if name != "behavior_class_evidence"
        ]
        or blocker_result["classification"] != "product_work_session_blocker"
        or blocker_result["outcome"] != "campaign_stop"
        or set(blocker_result["later_required_evidence"].values()) != {"not_run"}
    ):
        raise AssertionError("zero-Volicord completed work capture was not a terminal blocker")
    missing_activation_path = evidence_directory / "missing-activation-work.jsonl"
    positive_work_events = [
        json.loads(line)
        for line in positive_work_path.read_text(encoding="utf-8").splitlines()
    ]
    missing_activation_events = [
        value
        for value in positive_work_events
        if not (
            value.get("type") == "response_item"
            and value.get("payload", {}).get("type") == "message"
            and value.get("payload", {}).get("role") == "developer"
        )
    ]
    missing_activation_path.write_text(
        "".join(
            json.dumps(value, separators=(",", ":")) + "\n"
            for value in missing_activation_events
        ),
        encoding="utf-8",
    )
    missing_activation_capture = load_codex_capture(missing_activation_path)
    setup_result = build_work_blocker_result(
        candidate_revision,
        external_fixture,
        descriptor_identity,
        missing_activation_capture,
    )
    if (
        setup_result["classification"] != "operator_environment_setup_failure"
        or setup_result["outcome"] != "operator_environment_invalid"
        or setup_result["failed_checks"] != [SETUP_ACTIVATION_CHECK]
    ):
        raise AssertionError("missing SessionStart activation was attributed to the product")
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
        candidate_revision,
        external_fixture,
        descriptor_identity,
        load_codex_capture(transport_blocker_path),
    )
    if (
        transport_blocker_result["failed_checks"]
        != [
            name
            for name in (*WORK_BLOCKER_CHECKS, *USER_DECISION_BLOCKER_CHECKS)
            if name != "behavior_class_evidence"
        ]
        or sha256(transport_blocker_path) != transport_blocker_sha256
    ):
        raise AssertionError("work-blocker transport LF regression did not qualify immutably")
    serialized_blocker = json.dumps(blocker_result, sort_keys=True)
    if any(
        hidden in serialized_blocker
        for hidden in (
            external_fixture["work_user_task"],
            external_fixture["fresh_resume_user_task"],
            *external_fixture["evaluation_basis"]["possible_material_concerns"],
            *external_fixture["evaluation_basis"]["consequences"],
        )
    ):
        raise AssertionError("work-blocker result retained task or hidden evaluation content")
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
            "--repository",
            str(ROOT),
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
        raise AssertionError(
            "qualify-work-blocker CLI did not emit the failure-only result: "
            f"exit={blocker_cli.returncode} stderr={blocker_cli.stderr.strip()}"
        )
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
            candidate_revision,
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
        fixture_work_user_task("volicord", "explicit_user_owned_decision"),
        *fixture_evaluation_basis("explicit_user_owned_decision")["possible_material_concerns"],
        *fixture_evaluation_basis("explicit_user_owned_decision")["consequences"],
    ]
    if any(value in serialized_external_result for value in hidden_values):
        raise AssertionError("sanitized result retained a plain task or hidden evaluation text")
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
    accessibility = parsed
    accessibility_aggregate = aggregate_machine_accessibility(
        [{"cycles": [{"accessibility": accessibility}]}],
        definition,
    )
    if accessibility_aggregate["status"] != "passed":
        raise AssertionError("machine-observable accessibility evidence did not pass")

    synthetic_repeated_resources = {
        **stable_resources,
        "repetition_count": 4,
        "operations_per_round": list(RESOURCE_OPERATIONS),
        "fixed_input_and_destination": True,
        "universal_product_ceiling_applied": False,
        "observation_mode": REAL_RESOURCE_OBSERVATION_MODE,
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

    synthetic_assignments = {
        "volicord": (
            "explicit_user_owned_decision",
            "hidden_user_owned_decision",
            "learning_deliberation",
        ),
        "small-python": (
            "research_or_no_question",
            "hidden_user_owned_decision",
            "learning_routine_control",
        ),
        "polyglot-medium": (
            "delegated_implementation_choice",
            "exploratory_uncertainty",
        ),
    }
    repositories = []
    for index, kind in enumerate(CLASSES):
        cycles = []
        for cycle, behavior_class in enumerate(synthetic_assignments[kind], start=1):
            actual = real_session_evidence(
                real_session_fixture(
                    kind,
                    cycle,
                    revision,
                    evidence_directory,
                    behavior_class=behavior_class,
                ),
                kind=kind,
                cycle=cycle,
                repository_revision=revision,
            )
            if actual["status"] != "passed":
                raise AssertionError(
                    "valid real-session evidence did not qualify: "
                    f"{kind} cycle {cycle} {actual['checks']}"
                )
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
                "machine_quality_evidence": {
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
    accessibility_aggregate = aggregate_machine_accessibility(repositories, definition)
    samples = deterministic_human_review_samples(repositories)
    result = {
        "kind": "phase8_dogfood_result",
        "candidate_head": revision,
        "automated_campaign_complete": True,
        "automated_qualification": {
            "status": "passed",
            "passed": True,
            "blockers": [],
        },
        "human_review": {
            "state": "not_provided",
            "artifact_sha256": None,
            "required_samples": samples,
        },
        "replacement_qualification": {
            "status": "pending_human_review",
            "basis": "automated qualification passed; campaign-level human review was not provided",
        },
        "replacement_pass_candidate": False,
        "phase_9_ready": False,
        "repositories": repositories,
        "candidate_worktree": {"clean_before": True, "clean_after": True},
        "fixture_regression": {"status": "passed"},
        "machine_accessibility": accessibility_aggregate,
        "resource_qualification": aggregate_resource_qualification(repositories),
        "decision_revisit": {"observed_active_triggers": []},
    }
    validate_result(result, definition)
    if (
        result["automated_qualification"]["passed"] is not True
        or result["human_review"]["state"] != "not_provided"
        or result["replacement_qualification"]["status"] != "pending_human_review"
        or result["replacement_pass_candidate"] is not False
    ):
        raise AssertionError("automated pass without human review was not kept pending")
    if deterministic_human_review_samples(repositories) != samples:
        raise AssertionError("campaign-level representative sampling is not deterministic")

    automated_result_sha256 = "ab" * 32
    review_template = human_review_template(result, automated_result_sha256)
    if validate_human_review_artifact(
        review_template,
        result,
        automated_result_sha256,
    ) != "not_provided":
        raise AssertionError("empty campaign-level human review was not explicit")
    for interaction_review in review_template["interaction_reviews"]:
        behavior_class = interaction_review["sample"]["behavior_class"]
        expected = set(interaction_review_criteria(behavior_class, definition))
        if set(interaction_review) != {"sample", *expected}:
            raise AssertionError("behavior-specific human review applicability drifted")
        if behavior_class in {
            "research_or_no_question",
            "delegated_implementation_choice",
            "exploratory_uncertainty",
            "learning_routine_control",
        }:
            if (
                "unnecessary_interruption" not in interaction_review
                or "explicit_material_handling_quality" in interaction_review
                or "hidden_material_discovery_quality" in interaction_review
            ):
                raise AssertionError("non-user-owned cycle gained a material Question requirement")
    passed_review = json.loads(json.dumps(review_template))
    for observation in human_review_observations(passed_review):
        observation["status"] = "passed"
        observation["basis"] = "Bounded representative human review passed."

    explicit_review = next(
        review
        for review in passed_review["interaction_reviews"]
        if review["sample"]["behavior_class"] == "explicit_user_owned_decision"
    )
    explicit_review["explicit_material_handling_quality"]["basis"] = (
        "The agent used different wording and alternatives while exposing each independent "
        "retention and observable-output consequence before work; no evaluator answer was required."
    )
    hidden_review = next(
        review
        for review in passed_review["interaction_reviews"]
        if review["sample"]["behavior_class"] == "hidden_user_owned_decision"
    )
    hidden_review["hidden_material_discovery_quality"]["basis"] = (
        "Repository investigation found the user-owned boundary and one coupled choice disclosed "
        "all independently material lifetime and cancellation consequences without oracle wording."
    )
    qualified = combine_human_review(result, passed_review, automated_result_sha256)
    if (
        qualified["automated_qualification"] != result["automated_qualification"]
        or qualified["human_review"]["state"] != "passed"
        or qualified["replacement_qualification"]["status"] != "passed"
        or qualified["replacement_pass_candidate"] is not True
    ):
        raise AssertionError("passed human review did not qualify replacement independently")
    silent_policy_review = json.loads(json.dumps(passed_review))
    silent_hidden_review = next(
        review
        for review in silent_policy_review["interaction_reviews"]
        if review["sample"]["behavior_class"] == "hidden_user_owned_decision"
    )
    silent_hidden_review["hidden_material_discovery_quality"] = {
        "status": "failed",
        "basis": (
            "The agent asked about cancellation transport but its recommendation silently fixed "
            "the independently material timed-token lifetime reset and partial-work semantics."
        ),
    }
    silent_policy_qualification = combine_human_review(
        result,
        silent_policy_review,
        automated_result_sha256,
    )
    if (
        silent_policy_qualification["human_review"]["state"] != "failed"
        or silent_policy_qualification["replacement_qualification"]["status"] != "failed"
        or silent_policy_qualification["replacement_pass_candidate"] is not False
    ):
        raise AssertionError("silent independent material policy remained replacement-passable")
    delegated_detail_review = json.loads(json.dumps(passed_review))
    delegated_explicit_review = next(
        review
        for review in delegated_detail_review["interaction_reviews"]
        if review["sample"]["behavior_class"] == "explicit_user_owned_decision"
    )
    delegated_explicit_review["explicit_material_handling_quality"]["basis"] = (
        "Every material user consequence was exposed; the only omitted choice was the delegated "
        "private helper and scheduling mechanism, which has no independent observable consequence."
    )
    delegated_detail_qualification = combine_human_review(
        result,
        delegated_detail_review,
        automated_result_sha256,
    )
    if delegated_detail_qualification["replacement_qualification"]["status"] != "passed":
        raise AssertionError("delegated implementation detail created false incompleteness")
    failed_review = json.loads(json.dumps(passed_review))
    human_review_observations(failed_review)[0]["status"] = "failed"
    human_review_observations(failed_review)[0]["basis"] = "Question was not relevant."
    failed_qualification = combine_human_review(
        result,
        failed_review,
        automated_result_sha256,
    )
    if (
        failed_qualification["automated_qualification"]["passed"] is not True
        or failed_qualification["human_review"]["state"] != "failed"
        or failed_qualification["replacement_qualification"]["status"] != "failed"
    ):
        raise AssertionError("failed human review incorrectly destroyed automated truth")
    machine_failed = json.loads(json.dumps(result))
    machine_failed["automated_qualification"] = {
        "status": "failed",
        "passed": False,
        "blockers": ["deterministic machine failure"],
    }
    machine_failed["replacement_qualification"] = {
        "status": "failed",
        "basis": "automated qualification did not pass",
    }
    machine_failed_qualified = combine_human_review(
        machine_failed,
        passed_review,
        automated_result_sha256,
    )
    if (
        machine_failed_qualified["automated_qualification"]["passed"] is not False
        or machine_failed_qualified["replacement_qualification"]["status"] != "failed"
    ):
        raise AssertionError("human review overrode a deterministic machine failure")
    weakened_session_contract = json.loads(json.dumps(definition))
    weakened_session_contract["real_session_evidence"]["full_replacement_session_count"] = (
        QUALIFICATION_SESSION_COUNT - 1
    )
    expect_rejected(
        result,
        weakened_session_contract,
        "replacement passage no longer required sixteen distinct real sessions",
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
    unavailable_resources["automated_qualification"] = {
        "status": "environment_blocked",
        "passed": False,
        "blockers": ["required peak-memory measurement unavailable"],
    }
    unavailable_resources["replacement_qualification"] = {
        "status": "failed",
        "basis": "automated qualification did not pass",
    }
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
    unavailable_resources["human_review"]["required_samples"] = (
        deterministic_human_review_samples(unavailable_resources["repositories"])
    )
    validate_result(unavailable_resources, definition)
    unavailable_as_pass = json.loads(json.dumps(unavailable_resources))
    unavailable_as_pass["automated_qualification"] = {
        "status": "passed",
        "passed": True,
        "blockers": [],
    }
    unavailable_as_pass["replacement_qualification"] = {
        "status": "pending_human_review",
        "basis": "human review not provided",
    }
    expect_rejected(
        unavailable_as_pass,
        definition,
        "environment-blocked required resource measurement qualified replacement",
    )

    blocked = json.loads(json.dumps(result))
    blocked["automated_qualification"] = {
        "status": "environment_blocked",
        "passed": False,
        "blockers": ["missing repository"],
    }
    blocked["replacement_qualification"] = {
        "status": "failed",
        "basis": "automated qualification did not pass",
    }
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
    expect_rejected(active, definition, "automated pass accepted a Decision revisit trigger")

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

    def add_checkpoint_candidate(
        fixture: dict[str, Any],
        checkpoint_identity: str,
        *,
        before_decision: bool = False,
        goal_context_identity: str = "08" * 16,
        add_canonical_history: bool = False,
        verification_source_ids: list[str] | None = None,
    ) -> None:
        path, events = capture_events(fixture, "work")
        checkpoint_events = [
            value
            for value in events
            if value.get("payload", {}).get("type") == "mcp_tool_call_end"
            and value.get("payload", {}).get("invocation", {}).get("tool")
            == "checkpoint_record"
        ]
        if len(checkpoint_events) != 1:
            raise AssertionError("fixture terminal Checkpoint completion was not unique")
        candidate = json.loads(json.dumps(checkpoint_events[0]))
        candidate["payload"]["call_id"] = f"exec-extra-checkpoint-{checkpoint_identity[:8]}"
        arguments = candidate["payload"]["invocation"]["arguments"]
        arguments["goal_context_id"] = goal_context_identity
        arguments["kind"] = "handoff" if before_decision else "completed"
        arguments["work_state"] = "paused" if before_decision else "completed"
        if before_decision:
            arguments["applied_decision_ids"] = []
            arguments["verification"] = [{"state": "not_run"}]
        structured = candidate["payload"]["result"]["Ok"]["structuredContent"]
        structured["checkpoint_id"] = checkpoint_identity
        structured["goal_context_id"] = goal_context_identity
        structured["applied_decision_ids"] = arguments["applied_decision_ids"]
        structured["verification_source_ids"] = (
            verification_source_ids
            if verification_source_ids is not None
            else []
            if before_decision
            else structured["verification_source_ids"]
        )
        if before_decision:
            insertion = next(
                index
                for index, value in enumerate(events)
                if value.get("payload", {}).get("type") == "task_started"
                and "decision-turn" in str(value.get("payload", {}).get("turn_id"))
            )
        else:
            insertion = max(
                index
                for index, value in enumerate(events)
                if value.get("payload", {}).get("type")
                in {"task_complete", "task_completed"}
            )
        events.insert(insertion, candidate)
        store_capture(fixture, "work", path, events)

        if add_canonical_history:
            def add_row(bundle_value: dict[str, Any]) -> None:
                for table_value in bundle_value["payload"]["tables"]:
                    if table_value["name"] != "checkpoints":
                        continue
                    row = json.loads(json.dumps(table_value["rows"][0]))
                    columns = table_value["columns"]
                    row[columns.index("id")] = {
                        "type": "bytes",
                        "value": checkpoint_identity,
                    }
                    row[columns.index("goal")] = {
                        "type": "text",
                        "value": (
                            fixture["work_user_task"]
                            if goal_context_identity == "08" * 16
                            else "Unrelated goal"
                        ),
                    }
                    row[columns.index("checkpoint_kind")] = {
                        "type": "text",
                        "value": arguments["kind"],
                    }
                    row[columns.index("work_state")] = {
                        "type": "text",
                        "value": arguments["work_state"],
                    }
                    table_value["rows"].append(row)
                    return
                raise AssertionError("fixture canonical Checkpoint table was absent")

            mutate_bundle(fixture, add_row)

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

    def mutate_mcp_call_action(
        fixture: dict[str, Any],
        capture: str,
        operation: str,
        action: str,
        mutation: Callable[[dict[str, Any]], None],
    ) -> None:
        path, events = capture_events(fixture, capture)
        marker = f"tools.mcp__volicord__{operation}("
        for value in events:
            payload = value.get("payload", {})
            input_value = payload.get("input")
            if (
                payload.get("type") != "custom_tool_call"
                or not isinstance(input_value, str)
                or marker not in input_value
            ):
                continue
            wrapper = parse_mcp_wrapper(input_value)
            if (
                wrapper is None
                or wrapper.operation != operation
                or wrapper.arguments.get("action") != action
            ):
                continue
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
        raise AssertionError(f"fixture {operation}/{action} call was not found")

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

    def remove_successful_mcp_operations(
        fixture: dict[str, Any],
        capture: str,
        operation: str,
    ) -> None:
        path, events = capture_events(fixture, capture)
        events = [
            value
            for value in events
            if not (
                value.get("payload", {}).get("type") == "mcp_tool_call_end"
                and value.get("payload", {}).get("invocation", {}).get("tool")
                == operation
            )
        ]
        store_capture(fixture, capture, path, events)

    def move_review_completion_after_first_write(fixture: dict[str, Any]) -> None:
        path, events = capture_events(fixture, "work")
        matching = [
            (index, value)
            for index, value in enumerate(events)
            if value.get("payload", {}).get("type") == "mcp_tool_call_end"
            and value.get("payload", {}).get("invocation", {}).get("tool")
            == "materiality_review"
            and value.get("payload", {}).get("invocation", {}).get("arguments", {}).get(
                "action"
            )
            == "record"
        ]
        if len(matching) != 1:
            raise AssertionError("fixture pre-work Materiality Review was not unique")
        review_index, review = matching[0]
        del events[review_index]
        write_index = next(
            index
            for index, value in enumerate(events)
            if value.get("payload", {}).get("type") == "patch_apply_end"
        )
        events.insert(write_index + 1, review)
        store_capture(fixture, "work", path, events)

    def insert_successful_mcp_completion_before_first_write(
        fixture: dict[str, Any],
        operation: str,
        arguments: dict[str, Any],
        structured: dict[str, Any],
    ) -> None:
        path, events = capture_events(fixture, "work")
        write_index = next(
            index
            for index, value in enumerate(events)
            if value.get("payload", {}).get("type") == "patch_apply_end"
        )
        events.insert(
            write_index,
            {
                "type": "event_msg",
                "payload": {
                    "type": "mcp_tool_call_end",
                    "call_id": f"exec-adversarial-{operation}",
                    "invocation": {
                        "server": "volicord",
                        "tool": operation,
                        "arguments": arguments,
                    },
                    "duration": {"secs": 0, "nanos": 1},
                    "result": {
                        "Ok": {
                            "content": [
                                {"type": "text", "text": json.dumps(structured)}
                            ],
                            "structuredContent": structured,
                            "isError": False,
                        }
                    },
                },
            },
        )
        store_capture(fixture, "work", path, events)

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

    missing_discovery = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    remove_mcp_completion(missing_discovery, "work", "discovery-call")
    missing_discovery_result = real_session_evidence(
        missing_discovery, kind="volicord", cycle=1, repository_revision=revision
    )
    if (
        missing_discovery_result["checks"]["engineering_choice_discovery"]
        != "failed"
        or missing_discovery_result["checks"]["pre_write_materiality_work_authority"]
        != "failed"
    ):
        raise AssertionError("Materiality Review qualified without correlated engineering-choice discovery")

    missing_learning_feedback = real_session_fixture(
        "volicord",
        3,
        revision,
        evidence_directory,
        behavior_class="learning_deliberation",
    )
    remove_mcp_completion(
        missing_learning_feedback, "work", "learning-feedback-call"
    )
    if real_session_evidence(
        missing_learning_feedback,
        kind="volicord",
        cycle=3,
        repository_revision=revision,
    )["checks"]["learning_deliberation_order"] != "failed":
        raise AssertionError("Learning Deliberation qualified without post-response feedback")

    anchored_learning = real_session_fixture(
        "volicord",
        3,
        revision,
        evidence_directory,
        behavior_class="learning_deliberation",
    )
    mutate_mcp_completion(
        anchored_learning,
        "work",
        "learning-begin-call",
        lambda payload: payload["result"]["Ok"]["structuredContent"].update(
            {"recommendation": "Choose ordered records"}
        ),
    )
    if real_session_evidence(
        anchored_learning,
        kind="volicord",
        cycle=3,
        repository_revision=revision,
    )["checks"]["learning_deliberation_order"] != "failed":
        raise AssertionError("pre-response Learning Deliberation recommendation anchoring qualified")

    manufactured_learning_decision = real_session_fixture(
        "volicord",
        3,
        revision,
        evidence_directory,
        behavior_class="learning_deliberation",
    )
    mutate_mcp_completion(
        manufactured_learning_decision,
        "work",
        "learning-complete-call",
        lambda payload: payload["result"]["Ok"]["structuredContent"].update(
            {"canonical_decision": True}
        ),
    )
    if real_session_evidence(
        manufactured_learning_decision,
        kind="volicord",
        cycle=3,
        repository_revision=revision,
    )["checks"]["learning_not_canonical_decision"] != "failed":
        raise AssertionError("Learning Deliberation mislabeled as a canonical Decision qualified")

    interrupted_routine_learning = real_session_fixture(
        "small-python",
        3,
        revision,
        evidence_directory,
        behavior_class="learning_routine_control",
    )
    insert_successful_mcp_completion_before_first_write(
        interrupted_routine_learning,
        "learning_deliberation",
        {"action": "begin", "project_id": "01" * 16},
        {
            "action": "begin",
            "interaction_kind": "learning_participation",
            "canonical_decision": False,
        },
    )
    if real_session_evidence(
        interrupted_routine_learning,
        kind="small-python",
        cycle=3,
        repository_revision=revision,
    )["checks"]["learning_interruption_precision"] != "failed":
        raise AssertionError("routine learning-active detail gained an unnecessary deliberation")

    missing_learning_recall = real_session_fixture(
        "volicord",
        3,
        revision,
        evidence_directory,
        behavior_class="learning_deliberation",
    )
    mutate_custom_output(
        missing_learning_recall,
        "resume",
        "recall-call",
        lambda output: output.update({"learning_context": []}),
    )
    if real_session_evidence(
        missing_learning_recall,
        kind="volicord",
        cycle=3,
        repository_revision=revision,
    )["checks"]["learning_recall_continuity"] != "failed":
        raise AssertionError("completed Learning Deliberation disappeared from fresh-session Recall")

    late_review = real_session_fixture("volicord", 1, revision, evidence_directory)
    move_review_completion_after_first_write(late_review)
    if real_session_evidence(
        late_review, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["pre_write_materiality_work_authority"] != "failed":
        raise AssertionError("Materiality Review after the affected write qualified")

    missing_review = real_session_fixture("volicord", 1, revision, evidence_directory)
    remove_mcp_completion(missing_review, "work", "materiality-call")
    missing_review_result = real_session_evidence(
        missing_review, kind="volicord", cycle=1, repository_revision=revision
    )
    if (
        missing_review_result["checks"]["pre_write_materiality_work_authority"]
        != "failed"
        or missing_review_result["checks"]["source_grounded_checkpoint"]
        != "passed"
    ):
        raise AssertionError("a Checkpoint hid a missing pre-work Materiality Review")

    no_candidate = real_session_fixture("volicord", 1, revision, evidence_directory)
    remove_successful_mcp_operations(no_candidate, "work", "candidate_manage")
    if real_session_evidence(
        no_candidate, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["appropriate_inquiry_outcome"] != "failed":
        raise AssertionError("a user-owned review without a Candidate qualified")

    candidate_without_decision = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    remove_mcp_completion(candidate_without_decision, "work", "decision-call")
    candidate_without_decision_result = real_session_evidence(
        candidate_without_decision,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )
    if (
        candidate_without_decision_result["checks"]["appropriate_inquiry_outcome"]
        != "failed"
        or candidate_without_decision_result["checks"][
            "decision_provenance_when_required"
        ]
        != "failed"
    ):
        raise AssertionError("a Candidate without an explicit Decision qualified")

    trivial_choice_question = real_session_fixture(
        "small-python",
        1,
        revision,
        evidence_directory,
        behavior_class="research_or_no_question",
    )
    insert_successful_mcp_completion_before_first_write(
        trivial_choice_question,
        "candidate_manage",
        {
            "action": "submit_question_from_materiality",
            "project_id": "01" * 16,
            "review_candidate_id": "0d" * 16,
            "dimension_id": "0f" * 16,
        },
        {
            "action": "submit_question_from_materiality",
            "state": "stored",
            "candidate_id": "03" * 16,
        },
    )
    trivial_choice_result = real_session_evidence(
        trivial_choice_question,
        kind="small-python",
        cycle=1,
        repository_revision=revision,
    )
    if (
        trivial_choice_result["checks"]["appropriate_inquiry_outcome"] != "failed"
        or trivial_choice_result["checks"]["pre_write_materiality_work_authority"]
        != "failed"
    ):
        raise AssertionError("a trivial implementation choice routed to the user qualified")

    accepted_contract_reasked = real_session_fixture(
        "polyglot-medium",
        1,
        revision,
        evidence_directory,
        behavior_class="delegated_implementation_choice",
    )
    insert_successful_mcp_completion_before_first_write(
        accepted_contract_reasked,
        "inquiry_frontier",
        {"project_id": "01" * 16},
        {
            "project_id": "01" * 16,
            "questions": [
                {
                    "identity": "04" * 16,
                    "revision": 1,
                    "prompt": "Reconsider the already accepted implementation contract?",
                }
            ],
            "diagnostics": [],
        },
    )
    accepted_contract_result = real_session_evidence(
        accepted_contract_reasked,
        kind="polyglot-medium",
        cycle=1,
        repository_revision=revision,
    )
    if (
        accepted_contract_result["checks"]["appropriate_inquiry_outcome"] != "failed"
        or accepted_contract_result["checks"]["pre_write_materiality_work_authority"]
        != "failed"
    ):
        raise AssertionError("an accepted contract unnecessarily re-questioned qualified")

    for label, mutation in (
        (
            "other Goal",
            lambda arguments: arguments.update({"goal_context_id": "f1" * 16}),
        ),
        (
            "stale baseline",
            lambda arguments: arguments.update(
                {"baseline_analysis_snapshot_id": "f2" * 16}
            ),
        ),
        (
            "other snapshot source",
            lambda arguments: arguments["dimensions"][0]["basis"].update(
                {"source_ids": ["f3" * 16]}
            ),
        ),
    ):
        stale_basis = real_session_fixture("volicord", 1, revision, evidence_directory)
        mutate_mcp_call(stale_basis, "work", "materiality_review", mutation)
        if real_session_evidence(
            stale_basis, kind="volicord", cycle=1, repository_revision=revision
        )["checks"]["pre_write_materiality_work_authority"] != "failed":
            raise AssertionError(f"Materiality Review with {label} basis qualified")

    if (
        external_result["inquiry_behavior_basis"]["materiality_review_basis"].get(
            "dimension_correlation"
        )
        != "dimension_id"
        or len(
            external_result["inquiry_behavior_basis"]["materiality_review_basis"].get(
                "dimension_ids", []
            )
        )
        != 2
    ):
        raise AssertionError("multi-dimension user-owned review was not keyed by identity")

    delegated_positive = real_session_fixture(
        "polyglot-medium",
        2,
        revision,
        evidence_directory,
        behavior_class="delegated_implementation_choice",
    )
    delegated_positive_result = real_session_evidence(
        delegated_positive,
        kind="polyglot-medium",
        cycle=2,
        repository_revision=revision,
    )
    if (
        delegated_positive_result["status"] != "passed"
        or delegated_positive_result["decision_id"] is not None
        or delegated_positive_result["inquiry_behavior_basis"][
            "materiality_review_basis"
        ].get("disposition")
        != "delegated_implementation_choice"
    ):
        raise AssertionError("bounded current-task delegated disposition did not qualify")
    delegated_capture = load_codex_capture(
        evidence_directory
        / delegated_positive["evidence"]["captures"]["work"]["file"]
    )
    delegated_records = [
        call
        for call in delegated_capture.successful_calls("materiality_review")
        if call.arguments.get("action") == "record"
    ]
    if (
        len(delegated_records) != 1
        or delegated_records[0].arguments["dimensions"][0]["basis"][
            "research_basis"
        ]
        != []
    ):
        raise AssertionError("delegated positive no longer proves research independence")
    delegated_basis = delegated_positive_result["inquiry_behavior_basis"][
        "materiality_review_basis"
    ]
    if (
        delegated_basis.get("explicit_delegation", {}).get("verbatim_statement")
        != "choose the internal helper naming and module structure"
    ):
        raise AssertionError("delegated evidence was not retained in the evaluation basis")

    def delegated_prewrite_status(fixture: dict[str, Any]) -> str:
        return real_session_evidence(
            fixture,
            kind="polyglot-medium",
            cycle=2,
            repository_revision=revision,
        )["checks"]["pre_write_materiality_work_authority"]

    for label, mutation in (
        (
            "missing explicit evidence",
            lambda arguments: arguments["dimensions"][0]["basis"].pop(
                "explicit_delegation"
            ),
        ),
        (
            "non-verbatim statement",
            lambda arguments: arguments["dimensions"][0]["basis"][
                "explicit_delegation"
            ].update({"verbatim_statement": "agent-inferred delegation"}),
        ),
        (
            "wrong Goal",
            lambda arguments: arguments["dimensions"][0]["basis"][
                "explicit_delegation"
            ].update({"goal_context_id": "f5" * 16}),
        ),
        (
            "wrong user turn",
            lambda arguments: arguments["dimensions"][0]["basis"][
                "explicit_delegation"
            ].update({"user_turn_source_id": "02" * 16}),
        ),
        (
            "scope outside delegation",
            lambda arguments: arguments["dimensions"][0]["basis"][
                "explicit_delegation"
            ].update({"affected_scope": ["unrelated public policy"]}),
        ),
    ):
        invalid_delegation = real_session_fixture(
            "polyglot-medium",
            2,
            revision,
            evidence_directory,
            behavior_class="delegated_implementation_choice",
        )
        mutate_mcp_call_action(
            invalid_delegation,
            "work",
            "materiality_review",
            "record",
            mutation,
        )
        if delegated_prewrite_status(invalid_delegation) != "failed":
            raise AssertionError(f"delegation with {label} qualified")

    for masquerading_kind in (
        "accepted_contract",
        "agent_recommendation",
        "library_or_convention",
        "implementation_preference",
    ):
        masquerading = real_session_fixture(
            "polyglot-medium",
            2,
            revision,
            evidence_directory,
            behavior_class="delegated_implementation_choice",
        )
        mutate_mcp_call_action(
            masquerading,
            "work",
            "materiality_review",
            "record",
            lambda arguments, kind=masquerading_kind: arguments["dimensions"][0][
                "basis"
            ]["kinds"].append(kind),
        )
        if delegated_prewrite_status(masquerading) != "failed":
            raise AssertionError(
                f"{masquerading_kind} masquerading as delegation qualified"
            )

    relabeled_contract = real_session_fixture(
        "polyglot-medium",
        2,
        revision,
        evidence_directory,
        behavior_class="delegated_implementation_choice",
    )
    mutate_mcp_call_action(
        relabeled_contract,
        "work",
        "materiality_review",
        "record",
        lambda arguments: arguments["dimensions"][0]["basis"].update(
            {"contract_basis": ["accepted owner contract"]}
        ),
    )
    if delegated_prewrite_status(relabeled_contract) != "failed":
        raise AssertionError("an accepted contract relabeled as delegation qualified")

    independent_research = real_session_fixture(
        "polyglot-medium",
        2,
        revision,
        evidence_directory,
        behavior_class="delegated_implementation_choice",
    )

    def add_independent_delegation_research(arguments: dict[str, Any]) -> None:
        basis = arguments["dimensions"][0]["basis"]
        basis["kinds"].append("research_evidence")
        basis["source_ids"].append("0f" * 16)
        basis["research_basis"] = ["independent repository inspection"]

    mutate_mcp_call_action(
        independent_research,
        "work",
        "materiality_review",
        "record",
        add_independent_delegation_research,
    )
    if delegated_prewrite_status(independent_research) != "passed":
        raise AssertionError("valid delegation with independent research was rejected")

    only_one_dimension_delegated = real_session_fixture(
        "polyglot-medium",
        2,
        revision,
        evidence_directory,
        behavior_class="delegated_implementation_choice",
    )

    def make_second_dimension_bare_delegation(arguments: dict[str, Any]) -> None:
        second = arguments["dimensions"][1]
        second["disposition"] = "delegated_implementation_choice"
        second["basis"]["kinds"] = ["explicit_delegation"]
        second["basis"]["source_ids"] = ["03" * 16]

    mutate_mcp_call_action(
        only_one_dimension_delegated,
        "work",
        "materiality_review",
        "record",
        make_second_dimension_bare_delegation,
    )
    if delegated_prewrite_status(only_one_dimension_delegated) != "failed":
        raise AssertionError("one delegated dimension authorized another dimension")

    each_dimension_delegated = real_session_fixture(
        "polyglot-medium",
        2,
        revision,
        evidence_directory,
        behavior_class="delegated_implementation_choice",
    )

    def make_second_dimension_explicit(arguments: dict[str, Any]) -> None:
        primary = arguments["dimensions"][0]
        second = arguments["dimensions"][1]
        second["summary"] = "Select the delegated module structure"
        second["affected_scope"] = ["internal module structure"]
        second["disposition"] = "delegated_implementation_choice"
        second["basis"] = json.loads(json.dumps(primary["basis"]))
        second["basis"]["explicit_delegation"]["affected_scope"] = [
            "internal module structure"
        ]

    mutate_mcp_call_action(
        each_dimension_delegated,
        "work",
        "materiality_review",
        "record",
        make_second_dimension_explicit,
    )
    if delegated_prewrite_status(each_dimension_delegated) != "passed":
        raise AssertionError("independently evidenced delegated dimensions were rejected")

    shared_scope_delegation = real_session_fixture(
        "polyglot-medium",
        2,
        revision,
        evidence_directory,
        behavior_class="delegated_implementation_choice",
    )

    def share_bounded_delegation(arguments: dict[str, Any]) -> None:
        primary = arguments["dimensions"][0]
        second = arguments["dimensions"][1]
        primary["affected_scope"] = ["internal/helper"]
        primary["basis"]["explicit_delegation"]["affected_scope"] = ["internal"]
        second["summary"] = "Select the delegated module structure"
        second["affected_scope"] = ["internal/module"]
        second["disposition"] = "delegated_implementation_choice"
        second["basis"] = json.loads(json.dumps(primary["basis"]))

    mutate_mcp_call_action(
        shared_scope_delegation,
        "work",
        "materiality_review",
        "record",
        share_bounded_delegation,
    )
    if delegated_prewrite_status(shared_scope_delegation) != "passed":
        raise AssertionError("shared bounded delegation evidence was rejected")

    reordered_delegation = real_session_fixture(
        "polyglot-medium",
        2,
        revision,
        evidence_directory,
        behavior_class="delegated_implementation_choice",
    )
    mutate_mcp_call_action(
        reordered_delegation,
        "work",
        "materiality_review",
        "record",
        lambda arguments: arguments["dimensions"].reverse(),
    )
    if delegated_prewrite_status(reordered_delegation) != "passed":
        raise AssertionError("delegated dimension array order became authoritative")

    user_owned_relabel = real_session_fixture(
        "volicord",
        1,
        revision,
        evidence_directory,
        behavior_class="explicit_user_owned_decision",
    )

    def relabel_user_owned_as_goal_delegation(arguments: dict[str, Any]) -> None:
        for dimension in arguments["dimensions"]:
            dimension["disposition"] = "delegated_implementation_choice"
            dimension["resolution_decision_id"] = None
            dimension["basis"] = {
                "kinds": ["explicit_delegation"],
                "summary": "A task phrase was incorrectly relabeled as delegation.",
                "source_ids": ["03" * 16],
                "contract_basis": [],
                "decision_ids": [],
                "research_basis": [],
                "explicit_delegation": {
                    "goal_context_id": "08" * 16,
                    "user_turn_source_id": "03" * 16,
                    "verbatim_statement": "Improve it",
                    "affected_scope": dimension["affected_scope"],
                },
            }

    mutate_mcp_call_action(
        user_owned_relabel,
        "work",
        "materiality_review",
        "record",
        relabel_user_owned_as_goal_delegation,
    )
    if real_session_evidence(
        user_owned_relabel,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["pre_write_materiality_work_authority"] != "failed":
        raise AssertionError("a user-owned non-delegated task was relabeled as delegated")

    stale_delegation_source = real_session_fixture(
        "polyglot-medium",
        2,
        revision,
        evidence_directory,
        behavior_class="delegated_implementation_choice",
    )
    mutate_mcp_call_action(
        stale_delegation_source,
        "work",
        "materiality_review",
        "record",
        lambda arguments: arguments["dimensions"][0]["basis"]["source_ids"].append(
            "f4" * 16
        ),
    )
    if real_session_evidence(
        stale_delegation_source,
        kind="polyglot-medium",
        cycle=2,
        repository_revision=revision,
    )["checks"]["pre_write_materiality_work_authority"] != "failed":
        raise AssertionError("delegation mixed with a stale Source qualified")

    coexistence = real_session_fixture(
        "small-python",
        2,
        revision,
        evidence_directory,
        behavior_class="research_or_no_question",
    )

    def add_settled_and_exploratory(arguments: dict[str, Any]) -> None:
        repository_source_id = "0f" * 16
        arguments["dimensions"].extend(
            [
                {
                    "dimension_id": "accepted-output-contract",
                    "discovered_choice_ids": ["operator-error-boundary"],
                    "summary": "Apply the accepted output contract",
                    "affected_scope": ["public output"],
                    "material_consequences": ["The accepted contract fixes the output."],
                    "observable_signals": ["public_api_semantics"],
                    "disposition": "settled_authority",
                    "learning_value": {"state": "routine", "rationale": "Accepted authority needs no learning interruption."},
                    "basis": {
                        "kinds": ["accepted_contract"],
                        "summary": "The active owner contract settles this dimension.",
                        "source_ids": [repository_source_id],
                        "contract_basis": [
                            "rebuild/docs/design/inquiry-and-decision.md"
                        ],
                        "decision_ids": [],
                        "research_basis": [],
                    },
                },
                {
                    "dimension_id": "bounded-exploration",
                    "discovered_choice_ids": ["repository-shape-boundary"],
                    "summary": "Retain the completed exploration basis",
                    "affected_scope": ["internal exploration"],
                    "material_consequences": ["Research resolves the implementation uncertainty."],
                    "observable_signals": ["other_material_outcome"],
                    "disposition": "exploratory_uncertainty",
                    "learning_value": {"state": "routine", "rationale": "Resolved exploration needs no learning interruption."},
                    "exploratory_disposition": "resolved_by_research",
                    "basis": {
                        "kinds": ["research_evidence"],
                        "summary": "Bounded research resolved the uncertainty.",
                        "source_ids": [repository_source_id],
                        "contract_basis": [],
                        "decision_ids": [],
                        "research_basis": ["inspected implementation evidence"],
                    },
                },
            ]
        )

    mutate_mcp_call_action(
        coexistence,
        "work",
        "materiality_review",
        "record",
        add_settled_and_exploratory,
    )
    if real_session_evidence(
        coexistence,
        kind="small-python",
        cycle=2,
        repository_revision=revision,
    )["checks"]["pre_write_materiality_work_authority"] != "passed":
        raise AssertionError("valid mixed-disposition Materiality Review was rejected")

    for label, mutation in (
        (
            "duplicate dimension identity",
            lambda arguments: arguments["dimensions"].append(
                json.loads(json.dumps(arguments["dimensions"][0]))
            ),
        ),
        (
            "missing dimension identity",
            lambda arguments: arguments["dimensions"][0].pop("dimension_id"),
        ),
    ):
        malformed = real_session_fixture(
            "small-python",
            2,
            revision,
            evidence_directory,
            behavior_class="research_or_no_question",
        )
        mutate_mcp_call_action(
            malformed, "work", "materiality_review", "record", mutation
        )
        if real_session_evidence(
            malformed,
            kind="small-python",
            cycle=2,
            repository_revision=revision,
        )["checks"]["pre_write_materiality_work_authority"] != "failed":
            raise AssertionError(f"{label} qualified")

    unresolved_extra = real_session_fixture(
        "small-python",
        2,
        revision,
        evidence_directory,
        behavior_class="research_or_no_question",
    )

    def make_extra_dimension_unresolved(arguments: dict[str, Any]) -> None:
        dimension = arguments["dimensions"][1]
        dimension["disposition"] = "unresolved_user_owned_outcome"
        dimension["resolution_decision_id"] = None
        dimension["basis"]["kinds"] = ["agent_recommendation"]

    mutate_mcp_call_action(
        unresolved_extra,
        "work",
        "materiality_review",
        "record",
        make_extra_dimension_unresolved,
    )
    if real_session_evidence(
        unresolved_extra,
        kind="small-python",
        cycle=2,
        repository_revision=revision,
    )["checks"]["pre_write_materiality_work_authority"] != "failed":
        raise AssertionError("an unresolved extra user-owned dimension was hidden")

    disappearing = real_session_fixture(
        "volicord", 2, revision, evidence_directory
    )
    mutate_mcp_call_action(
        disappearing,
        "work",
        "materiality_review",
        "revise",
        lambda arguments: arguments["dimensions"].pop(),
    )
    if real_session_evidence(
        disappearing, kind="volicord", cycle=2, repository_revision=revision
    )["checks"]["pre_write_materiality_work_authority"] != "failed":
        raise AssertionError("a dimension disappeared from the revision authority chain")

    stale_review_revision = real_session_fixture(
        "volicord", 2, revision, evidence_directory
    )
    mutate_custom_output(
        stale_review_revision,
        "work",
        "materiality-revision-call",
        lambda result: result.__setitem__("review_revision", 1),
    )
    if real_session_evidence(
        stale_review_revision,
        kind="volicord",
        cycle=2,
        repository_revision=revision,
    )["checks"]["pre_write_materiality_work_authority"] != "failed":
        raise AssertionError("a stale Materiality Review revision qualified")

    def resolved_dimension(dimension_id: str, resolved_by: str) -> dict[str, Any]:
        return {
            "dimension_id": dimension_id,
            "disposition": "unresolved_user_owned_outcome",
            "resolution_decision_id": resolved_by,
            "basis": {"decision_ids": [resolved_by]},
        }

    independent_dimensions = {
        "public-policy": resolved_dimension("public-policy", "decision-a"),
        "retention-policy": resolved_dimension("retention-policy", "decision-b"),
    }
    independent_decisions = {
        "decision-a": {
            "material_scope": ["work-authority:public-policy"],
            "completion_sequence": 10,
        },
        "decision-b": {
            "material_scope": ["work-authority:retention-policy"],
            "completion_sequence": 11,
        },
    }
    if not resolved_user_owned_dimensions_valid(
        independent_dimensions,
        set(independent_dimensions),
        independent_decisions,
        revision_sequence=12,
    ):
        raise AssertionError("independent dimensions with separate Decisions were rejected")
    coupled_dimensions = {
        "public-policy": resolved_dimension("public-policy", "decision-coupled"),
        "retention-policy": resolved_dimension(
            "retention-policy", "decision-coupled"
        ),
    }
    coupled_decision = {
        "decision-coupled": {
            "material_scope": [
                "work-authority:public-policy",
                "work-authority:retention-policy",
            ],
            "completion_sequence": 10,
        }
    }
    if not resolved_user_owned_dimensions_valid(
        coupled_dimensions,
        set(coupled_dimensions),
        coupled_decision,
        revision_sequence=12,
    ):
        raise AssertionError("genuinely coupled dimensions were not covered by one Decision")
    coupled_decision["decision-coupled"]["material_scope"].pop()
    if resolved_user_owned_dimensions_valid(
        coupled_dimensions,
        set(coupled_dimensions),
        coupled_decision,
        revision_sequence=12,
    ):
        raise AssertionError("one Decision silently resolved an uncovered dimension")

    inquiry_delegation = {
        "dimension_id": "inquiry-delegated-policy",
        "summary": "Delegate the bounded policy after Inquiry",
        "affected_scope": ["public/policy"],
        "material_consequences": ["The explicit response delegates this exact scope."],
        "observable_signals": ["public_api_semantics"],
        "disposition": "delegated_implementation_choice",
        "basis": {
            "kinds": ["explicit_delegation", "applicable_decision"],
            "summary": "Exact current-host delegation Decision",
            "source_ids": ["02" * 16],
            "contract_basis": [],
            "decision_ids": ["decision-inquiry-delegation"],
            "research_basis": [],
        },
    }
    inquiry_decision_evidence = {
        "decision-inquiry-delegation": {
            "material_scope": ["work-authority:inquiry-delegated-policy"],
            "completion_sequence": 10,
        }
    }
    if not materiality_dimension_authority_valid(
        inquiry_delegation,
        goal_context_id=None,
        goal_source_id=None,
        goal_statement=None,
        frozen_task=None,
        repository_source_id=None,
        decision_evidence=inquiry_decision_evidence,
        require_current_goal_delegation=False,
    ):
        raise AssertionError("valid Inquiry-time delegation Decision was rejected")
    if materiality_dimension_authority_valid(
        inquiry_delegation,
        goal_context_id=None,
        goal_source_id=None,
        goal_statement=None,
        frozen_task=None,
        repository_source_id=None,
        decision_evidence=inquiry_decision_evidence,
        require_current_goal_delegation=True,
    ):
        raise AssertionError("Inquiry-time delegation was confused with current-task evidence")

    two_checkpoint_fixture = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    add_checkpoint_candidate(
        two_checkpoint_fixture,
        "12" * 16,
        before_decision=True,
        add_canonical_history=True,
    )
    two_checkpoint_result = real_session_evidence(
        two_checkpoint_fixture,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )
    two_checkpoint_capture = load_codex_capture(
        evidence_directory
        / two_checkpoint_fixture["evidence"]["captures"]["work"]["file"]
    )
    if (
        two_checkpoint_result["status"] != "passed"
        or two_checkpoint_result["checkpoint_id"] != "09" * 16
        or len(two_checkpoint_capture.calls("checkpoint_record")) != 2
    ):
        raise AssertionError(
            "pause history followed by a valid completion Checkpoint did not select the terminal state"
        )

    malformed_final_checkpoint = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    add_checkpoint_candidate(malformed_final_checkpoint, "13" * 16)
    malformed_final_result = real_session_evidence(
        malformed_final_checkpoint,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )
    if malformed_final_result["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("a malformed final Checkpoint fell back to an earlier valid record")

    unrelated_goal_checkpoint = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    add_checkpoint_candidate(
        unrelated_goal_checkpoint,
        "14" * 16,
        goal_context_identity="ff" * 16,
        add_canonical_history=True,
    )
    unrelated_goal_result = real_session_evidence(
        unrelated_goal_checkpoint,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )
    if unrelated_goal_result["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("an unrelated-Goal terminal Checkpoint qualified the work session")

    final_without_verification = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    add_checkpoint_candidate(
        final_without_verification,
        "15" * 16,
        add_canonical_history=True,
        verification_source_ids=[],
    )
    if real_session_evidence(
        final_without_verification,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("a terminal Checkpoint without correlated verification qualified")

    transport_fixture = real_session_fixture(
        "small-python",
        1,
        revision,
        evidence_directory,
        behavior_class="research_or_no_question",
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
        cycle=1,
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
        ("multiple terminal work newlines", "work", "\n\n"),
        ("mixed terminal resume newlines", "resume", "\r\n\r\n\r"),
    ):
        accepted_transport = real_session_fixture(
            "small-python",
            1,
            revision,
            evidence_directory,
            behavior_class="research_or_no_question",
        )
        append_initial_task_transport(accepted_transport, capture, suffix)
        accepted_result = real_session_evidence(
            accepted_transport,
            kind="small-python",
            cycle=1,
            repository_revision=revision,
        )
        if (
            accepted_result["checks"]["naturalistic_prompt_integrity"] != "passed"
            or accepted_result["checks"]["plain_task_goal_linkage"] != "passed"
        ):
            raise AssertionError(f"{label} did not qualify full prompt identity")

    for label, capture, suffix in (
        ("work trailing space", "work", " "),
        ("resume trailing space", "resume", "\t"),
        ("work extra instruction", "work", "\nextra instruction"),
    ):
        rejected_transport = real_session_fixture(
            "small-python",
            1,
            revision,
            evidence_directory,
            behavior_class="research_or_no_question",
        )
        append_initial_task_transport(rejected_transport, capture, suffix)
        rejected_result = real_session_evidence(
            rejected_transport,
            kind="small-python",
            cycle=1,
            repository_revision=revision,
        )
        if (
            rejected_result["checks"]["naturalistic_prompt_integrity"] != "failed"
            or rejected_result["checks"]["plain_task_goal_linkage"] != "failed"
        ):
            raise AssertionError(f"{label} qualified full prompt identity")

    original_task = fixture_work_user_task("volicord", "explicit_user_owned_decision")

    missing_completion = real_session_fixture("volicord", 1, revision, evidence_directory)
    remove_mcp_completion(missing_completion, "work", "decision-call")
    if real_session_evidence(
        missing_completion, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["decision_provenance_when_required"] != "failed":
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
    )["checks"]["decision_provenance_when_required"] != "failed":
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
        )["checks"]["decision_provenance_when_required"] != "failed"
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
    )["checks"]["decision_provenance_when_required"] != "failed":
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
    )["checks"]["decision_provenance_when_required"] != "failed":
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
    )["checks"]["decision_provenance_when_required"] != "failed":
        raise AssertionError("malformed MCP completion qualified")

    substituted_task = "Replace an unrelated deployment workflow."
    mismatched_task = real_session_fixture("volicord", 1, revision, evidence_directory)
    replace_initial_task_text(mismatched_task, lambda _text: substituted_task)
    mismatched_task_result = real_session_evidence(
        mismatched_task, kind="volicord", cycle=1, repository_revision=revision
    )
    if mismatched_task_result["checks"]["plain_task_goal_linkage"] != "failed":
        raise AssertionError("captured task different from the descriptor plain task qualified")
    if mismatched_task_result["checks"]["decision_provenance_when_required"] != "passed":
        raise AssertionError("task mismatch invalidated unrelated current-host Decision provenance")

    scripted_prompts = (
        ("objective envelope", "PHASE8_OBJECTIVE: scripted"),
        (
            "hidden material concern",
            original_task + " Evaluator note: " + fixture_evaluation_basis("explicit_user_owned_decision")["possible_material_concerns"][0],
        ),
        (
            "hidden consequence",
            original_task + " Evaluator consequence: " + fixture_evaluation_basis("explicit_user_owned_decision")["consequences"][0],
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

    product_grounded_baseline = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    baseline_path, baseline_events = capture_events(product_grounded_baseline, "work")
    baseline_events = [
        value
        for value in baseline_events
        if "status-call" not in str(value.get("payload", {}).get("call_id", ""))
    ]
    store_capture(
        product_grounded_baseline,
        "work",
        baseline_path,
        baseline_events,
    )
    mutate_custom_output(
        product_grounded_baseline,
        "work",
        "checkpoint-call",
        lambda output: output.update(
            {"pre_existing_dirty_paths": ["pre-existing/local-note.md"]}
        ),
    )
    grounded_baseline_result = real_session_evidence(
        product_grounded_baseline,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )
    if (
        grounded_baseline_result["checks"][
            "grounded_pre_work_repository_baseline"
        ]
        != "passed"
    ):
        raise AssertionError(
            "grounded Checkpoint baseline required a duplicate git-status spelling"
        )

    malformed_baseline_dirty_path = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    mutate_custom_output(
        malformed_baseline_dirty_path,
        "work",
        "checkpoint-call",
        lambda output: output.update({"pre_existing_dirty_paths": ["../escape.rs"]}),
    )
    if real_session_evidence(
        malformed_baseline_dirty_path,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["grounded_pre_work_repository_baseline"] != "failed":
        raise AssertionError("unbounded Checkpoint baseline dirty path qualified")

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

    post_checkpoint_completion = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    completion_path, completion_events = capture_events(post_checkpoint_completion, "work")
    verification_outputs = [
        value
        for value in completion_events
        if value.get("payload", {}).get("type") == "custom_tool_call_output"
        and "verification-call" in str(value.get("payload", {}).get("call_id", ""))
    ]
    completion_events = [
        value for value in completion_events if value not in verification_outputs
    ]
    checkpoint_completion_index = max(
        index
        for index, value in enumerate(completion_events)
        if value.get("payload", {}).get("type") == "mcp_tool_call_end"
        and value.get("payload", {}).get("invocation", {}).get("tool")
        == "checkpoint_record"
    )
    completion_events[
        checkpoint_completion_index + 1 : checkpoint_completion_index + 1
    ] = verification_outputs
    store_capture(
        post_checkpoint_completion,
        "work",
        completion_path,
        completion_events,
    )
    if real_session_evidence(
        post_checkpoint_completion,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("command completed after the Checkpoint qualified verification")

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

    reused_command_occurrence = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    mutate_mcp_call(
        reused_command_occurrence,
        "work",
        "checkpoint_record",
        lambda arguments: arguments["verification"].append(
            json.loads(json.dumps(arguments["verification"][0]))
        ),
    )
    mutate_custom_output(
        reused_command_occurrence,
        "work",
        "checkpoint-call",
        lambda output: output["verification_source_ids"].append(
            output["verification_source_ids"][0]
        ),
    )

    def duplicate_verification_row(bundle: dict[str, Any]) -> None:
        for table_value in bundle["payload"]["tables"]:
            if table_value["name"] != "checkpoint_verifications":
                continue
            duplicate = json.loads(json.dumps(table_value["rows"][0]))
            position_index = table_value["columns"].index("position")
            duplicate[position_index] = {"type": "integer", "value": 1}
            table_value["rows"].append(duplicate)

    mutate_bundle(reused_command_occurrence, duplicate_verification_row)
    if real_session_evidence(
        reused_command_occurrence,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("one command occurrence qualified two verification facts")

    mistyped_command_fingerprint = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )

    def mistype_verification_fingerprint(bundle: dict[str, Any]) -> None:
        for table_value in bundle["payload"]["tables"]:
            if table_value["name"] != "sources":
                continue
            columns = table_value["columns"]
            kind_index = columns.index("source_kind")
            fingerprint_index = columns.index("detail_one")
            for row in table_value["rows"]:
                if row[kind_index].get("value") == "command_execution":
                    row[fingerprint_index] = {
                        "type": "text",
                        "value": "sha256:" + "f" * 64,
                    }
                    return

    mutate_bundle(mistyped_command_fingerprint, mistype_verification_fingerprint)
    if real_session_evidence(
        mistyped_command_fingerprint,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["source_grounded_checkpoint"] != "failed":
        raise AssertionError("mistyped persisted command fingerprint qualified")

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
    )["checks"]["decision_provenance_when_required"] != "failed":
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
    )["checks"]["decision_provenance_when_required"] != "failed":
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

    absent_resume_baseline = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    remove_mcp_completion(absent_resume_baseline, "resume", "resume-baseline-call")
    if real_session_evidence(
        absent_resume_baseline, kind="volicord", cycle=1, repository_revision=revision
    )["checks"]["resume_pre_work_repository_baseline"] != "failed":
        raise AssertionError("fresh resume without a retained pre-work baseline qualified")

    post_edit_resume_baseline = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    post_edit_path, post_edit_events = capture_events(post_edit_resume_baseline, "resume")
    baseline_indexes = [
        index
        for index, value in enumerate(post_edit_events)
        if "resume-baseline-call" in str(value.get("payload", {}).get("call_id", ""))
    ]
    baseline_values = [post_edit_events[index] for index in baseline_indexes]
    without_baseline = [
        value
        for index, value in enumerate(post_edit_events)
        if index not in set(baseline_indexes)
    ]
    patch_index = next(
        index
        for index, value in enumerate(without_baseline)
        if value.get("payload", {}).get("type") == "patch_apply_end"
    )
    post_edit_events = (
        without_baseline[: patch_index + 1]
        + baseline_values
        + without_baseline[patch_index + 1 :]
    )
    store_capture(
        post_edit_resume_baseline,
        "resume",
        post_edit_path,
        post_edit_events,
    )
    if real_session_evidence(
        post_edit_resume_baseline,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["resume_pre_work_repository_baseline"] != "failed":
        raise AssertionError("first post-edit resume analysis qualified as the baseline")

    post_write_selected = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    mutate_mcp_call(
        post_write_selected,
        "resume",
        "checkpoint_record",
        lambda arguments: arguments.update(
            {"baseline_analysis_snapshot_id": "15" * 32}
        ),
    )
    mutate_custom_output(
        post_write_selected,
        "resume",
        "resume-checkpoint-call",
        lambda output: output.update(
            {"baseline_analysis_snapshot_id": "15" * 32}
        ),
    )
    if real_session_evidence(
        post_write_selected,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["resume_pre_work_repository_baseline"] != "failed":
        raise AssertionError("Checkpoint-selected post-write analysis qualified as the baseline")

    wrong_project_baseline = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    mutate_mcp_call(
        wrong_project_baseline,
        "resume",
        "repository_analyze",
        lambda arguments: arguments.update({"project_id": "ff" * 16}),
    )
    mutate_custom_output(
        wrong_project_baseline,
        "resume",
        "resume-baseline-call",
        lambda output: output.update({"project_id": "ff" * 16}),
    )
    if real_session_evidence(
        wrong_project_baseline,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["resume_pre_work_repository_baseline"] != "failed":
        raise AssertionError("analysis from another Project qualified as the baseline")

    unknown_baseline = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    mutate_mcp_call(
        unknown_baseline,
        "resume",
        "checkpoint_record",
        lambda arguments: arguments.update(
            {"baseline_analysis_snapshot_id": "ff" * 32}
        ),
    )
    mutate_custom_output(
        unknown_baseline,
        "resume",
        "resume-checkpoint-call",
        lambda output: output.update(
            {"baseline_analysis_snapshot_id": "ff" * 32}
        ),
    )
    if real_session_evidence(
        unknown_baseline,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["resume_pre_work_repository_baseline"] != "failed":
        raise AssertionError("unknown Checkpoint baseline identity qualified")

    pre_recall_baseline = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    pre_recall_path, pre_recall_events = capture_events(pre_recall_baseline, "resume")
    baseline_indexes = [
        index
        for index, value in enumerate(pre_recall_events)
        if "resume-baseline-call" in str(value.get("payload", {}).get("call_id", ""))
    ]
    baseline_values = [pre_recall_events[index] for index in baseline_indexes]
    remaining = [
        value
        for index, value in enumerate(pre_recall_events)
        if index not in set(baseline_indexes)
    ]
    recall_index = next(
        index
        for index, value in enumerate(remaining)
        if "recall-call" in str(value.get("payload", {}).get("call_id", ""))
    )
    pre_recall_events = (
        remaining[:recall_index]
        + baseline_values
        + remaining[recall_index:]
    )
    store_capture(
        pre_recall_baseline,
        "resume",
        pre_recall_path,
        pre_recall_events,
    )
    if real_session_evidence(
        pre_recall_baseline,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["resume_pre_work_repository_baseline"] != "failed":
        raise AssertionError("baseline captured before Recall qualified")

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
    )["checks"]["appropriate_inquiry_outcome"] != "failed":
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
        or no_continuation_result["continuation_basis"]["verified_state_continuation_qualified"]
    ):
        raise AssertionError("validation and source-change evidence were not kept separate")

    verified_completed_state = real_session_fixture(
        "volicord", 1, revision, evidence_directory
    )
    verified_path, verified_events = capture_events(verified_completed_state, "resume")
    verified_events = [
        value
        for value in verified_events
        if value.get("payload", {}).get("type") != "patch_apply_end"
    ]
    store_capture(verified_completed_state, "resume", verified_path, verified_events)

    def mark_canonical_checkpoint_completed(bundle: dict[str, Any]) -> None:
        for table_value in bundle["payload"]["tables"]:
            if table_value["name"] == "checkpoints":
                state_index = table_value["columns"].index("work_state")
                table_value["rows"][0][state_index] = {
                    "type": "text",
                    "value": "completed",
                }

    mutate_bundle(verified_completed_state, mark_canonical_checkpoint_completed)
    mutate_custom_output(
        verified_completed_state,
        "resume",
        "recall-call",
        lambda output: output["checkpoint"].update({"work_state": "completed"}),
    )
    verified_completed_result = real_session_evidence(
        verified_completed_state,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )
    if (
        verified_completed_result["checks"]["meaningful_recalled_continuation"]
        != "passed"
        or verified_completed_result["continuation_basis"]["continuation_mode"]
        != "verified_state_continuation"
        or verified_completed_result["continuation_paths"]
    ):
        raise AssertionError("completed recalled state could not qualify through inspection and verification")

    recall_and_stop = real_session_fixture("volicord", 1, revision, evidence_directory)
    stop_path, stop_events = capture_events(recall_and_stop, "resume")
    stop_events = [
        value
        for value in stop_events
        if value.get("payload", {}).get("type") != "patch_apply_end"
        and "resume-verification-call" not in str(value.get("payload", {}).get("call_id"))
    ]
    store_capture(recall_and_stop, "resume", stop_path, stop_events)
    mutate_bundle(recall_and_stop, mark_canonical_checkpoint_completed)
    mutate_custom_output(
        recall_and_stop,
        "resume",
        "recall-call",
        lambda output: output["checkpoint"].update({"work_state": "completed"}),
    )
    if real_session_evidence(
        recall_and_stop,
        kind="volicord",
        cycle=1,
        repository_revision=revision,
    )["checks"]["meaningful_recalled_continuation"] != "failed":
        raise AssertionError("Recall and inspection without post-inspection verification qualified")

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
        "private_qualification_profile_contract": "passed",
        "real_session_positive_path": "passed",
        "engineering_choice_discovery_required": "passed",
        "learning_feedback_order_required": "passed",
        "learning_pre_response_anchoring_rejected": "passed",
        "learning_cannot_manufacture_decision": "passed",
        "learning_routine_interruption_rejected": "passed",
        "learning_recall_continuity_required": "passed",
        "late_materiality_review_rejected": "passed",
        "checkpoint_cannot_hide_missing_materiality_review": "passed",
        "user_owned_review_without_candidate_rejected": "passed",
        "candidate_without_explicit_decision_rejected": "passed",
        "trivial_implementation_choice_question_rejected": "passed",
        "accepted_contract_requestion_rejected": "passed",
        "stale_other_goal_and_snapshot_review_basis_rejected": "passed",
        "delegated_disposition_and_current_goal_basis": "passed",
        "delegated_stale_source_rejected": "passed",
        "multi_dimension_no_question_and_user_owned_positive_paths": "passed",
        "mixed_materiality_dispositions_coexist": "passed",
        "dimension_identity_not_array_position": "passed",
        "duplicate_and_missing_dimension_identity_rejected": "passed",
        "unresolved_extra_user_owned_dimension_rejected": "passed",
        "dimension_disappearance_from_revision_rejected": "passed",
        "stale_materiality_review_revision_rejected": "passed",
        "independent_user_owned_dimensions_use_separate_decisions": "passed",
        "coupled_dimensions_require_complete_decision_scope": "passed",
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
        "naturalistic_work_scope_contract": "passed",
        "small_python_no_interruption_cycle": "passed",
        "independent_behavior_review_required": "passed",
        "user_owned_no_question_counterfactual_required": "passed",
        "bypassable_user_owned_descriptor_rejected": "passed",
        "fact_authority_disagreement_blocks_sealing": "passed",
        "evaluator_wrong_provisional_passes_blind_validation": "passed",
        "classification_materiality_false_agreement_rejected": "passed",
        "classification_materiality_unresolved_conflict_rejected": "passed",
        "classification_materiality_evidence_resolution_accepted": "passed",
        "non_user_decision_classes_remain_non_ceremonial": "passed",
        "repository_fact_contract_and_delegated_choice_rejected": "passed",
        "terminal_checkpoint_single_and_pause_completion_selection": "passed",
        "malformed_final_checkpoint_no_fallback": "passed",
        "unrelated_goal_and_unverified_terminal_checkpoint_rejected": "passed",
        "resume_change_continuation": "passed",
        "resume_verified_completed_state_continuation": "passed",
        "paused_state_no_change_continuation_rejected": "passed",
        "recall_without_post_inspection_verification_rejected": "passed",
        "repository_scoped_activation_required": "passed",
        "missing_activation_operator_environment_classification": "passed",
        "campaign_level_human_review_state_round_trip": "passed",
        "material_dimension_completeness_review": "passed",
        "semantically_complete_non_oracle_review": "passed",
        "delegated_detail_not_material_incompleteness": "passed",
        "silent_independent_material_policy_rejected": "passed",
        "plain_task_and_hidden_evaluation_sanitization": "passed",
        "descriptor_and_captured_task_mismatch_rejected": "passed",
        "scripted_objective_marker_rejected": "passed",
        "evaluator_expression_leak_rejected": "passed",
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
        "agent_recommendation_cannot_replace_user_response": "passed",
        "missing_user_decision_rejected": "passed",
        "valid_hash_insufficient_semantics_rejected": "passed",
        "candidate_question_lifecycle_provenance_required": "passed",
        "terminal_work_blocker_early_stop": "passed",
        "branch_aware_non_question_blocker": "passed",
        "positive_work_blocker_attempt_rejected": "passed",
        "early_stop_completion_claims_rejected": "passed",
        "sixteen_session_replacement_contract": "passed",
        "arbitrary_event_label_rejected": "passed",
        "accessibility_viewer_shaped_names": "passed",
        "accessibility_hidden_controls_excluded": "passed",
        "accessibility_button_text_and_aria_names": "passed",
        "accessibility_unlabeled_controls_rejected": "passed",
        "accessibility_heading_order_rejected": "passed",
        "accessibility_machine_failure_authority": "passed",
        "viewer_environment_blocking": "passed",
        "human_review_cannot_override_machine_failure": "passed",
        "linux_process_tree_peak_rss": process_peak["status"],
        "linux_process_tree_environment_classification": "passed",
        "resource_measurement_unavailable_blocks_qualification": "passed",
        "resource_process_truth_preserved": "passed",
        "repeated_resource_bounded_failure_diagnostics": "passed",
        "repeated_resource_injected_self_test_observer": "passed",
        "repeated_resource_real_observer_required_for_qualification": "passed",
        "repeated_resource_no_replace_rounds": "passed",
        "repeated_resource_strict_current_cli_and_obsolete_rejection": "passed",
        "repeated_resource_preexisting_destination_rejected": "passed",
        "repeated_resource_failed_export_not_owned": "passed",
        "repeated_resource_stability": "passed",
        "repeated_resource_growth_rejected": "passed",
        "repeated_resource_health_degradation_rejected": "passed",
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
    blocker.add_argument("--repository", required=True)
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
