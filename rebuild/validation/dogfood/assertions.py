#!/usr/bin/env python3
"""Self-test the Phase 8 dogfood evaluation support boundary."""

from __future__ import annotations

import ast
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
HARNESS = HERE / "harness.py"
CODEX_EVENTS = HERE / "codex_events.py"
DEFINITION = HERE / "evaluation.json"


def main() -> int:
    source = HARNESS.read_text(encoding="utf-8")
    event_source = CODEX_EVENTS.read_text(encoding="utf-8")
    definition = DEFINITION.read_text(encoding="utf-8")
    tree = ast.parse(source)
    imports = {
        alias.name
        for node in ast.walk(tree)
        if isinstance(node, ast.Import)
        for alias in node.names
    }
    if "requests" in imports or "psutil" in imports:
        raise AssertionError("Phase 8 harness unexpectedly added a process/network framework dependency")
    if "rebuild/scripts/validate final" in source:
        raise AssertionError("Phase 8 harness may not invoke direct final validation")
    if "rebuild/scripts/validate gate" in source:
        raise AssertionError("Phase 8 harness may not own the final gate")
    if "rehearse_target" not in source or "V11_HARNESS" not in source:
        raise AssertionError("Phase 8 harness no longer reuses the maintained product journey")
    if "real_session_evidence" not in source or "REAL_SESSION_CHECKS" not in source:
        raise AssertionError("Phase 8 harness no longer requires real-session evidence")
    for marker in (
        "load_codex_capture",
        "load_canonical_bundle",
        "custom_tool_call",
        "custom_tool_call_output",
        "patch_apply_end",
        "mcp__volicord",
        "context_record",
        "baseline_analysis_snapshot_id",
        "checkpoint_verifications",
        "current_host_user_turn",
        "PHASE8_OBJECTIVE_PREFIX",
        "objective_basis",
    ):
        if marker not in source and marker not in event_source:
            raise AssertionError(f"Phase 8 content normalizer is missing {marker}")
    if "verified_capture" in source or "first_repository_inspection_sequence" in source:
        raise AssertionError("the declaration-only real-session path remains active")
    if 'payload_type == "function_call"' in event_source or "eval(" in event_source:
        raise AssertionError("the obsolete decoder or JavaScript evaluation path remains active")
    if 'cli_version ==' in source or 'cli_version ==' in event_source:
        raise AssertionError("Phase 8 may not dispatch capture parsing by numeric Codex version")
    if "--authorize-codex-transmission" in source:
        raise AssertionError("the superseded project-health-only Phase 8 assertion remains")
    if '"codex_transmission"' in definition or "project-health-six-real-repository-cycles" in definition:
        raise AssertionError("the superseded Phase 8 transmission contract remains")
    if "verify_repository_normalized_codex_rollout_and_canonical_bundle" not in definition:
        raise AssertionError("Phase 8 definition does not select content-normalized evidence")
    for forwarding_requirement in (
        "commands used only for incidental inspection",
        "full-result text(r) forwarding",
        '"full_result_wrapper": "text(r)"',
        '"output_only_wrapper": "text(r.output)"',
        '"output_only_outcome": "unknown"',
        "a fresh resume session invokes Recall before repository inspection",
    ):
        if forwarding_requirement not in definition:
            raise AssertionError(
                f"Phase 8 definition is missing forwarding requirement {forwarding_requirement}"
            )
    if "(?:\\|\\||\\?\\?)" not in event_source:
        raise AssertionError("Phase 8 MCP parser does not share the static fallback grammar")
    if '"repository_specific_objective": evidence_check(references_present, metadata_ok)' in source:
        raise AssertionError("repository-specific objective still aliases outer metadata")
    for linkage in (
        "first_work_session_user_task_turn",
        "work_session_git_revision",
        "evaluated_repository_revision",
        "context_record_inner_objective",
        "context_record_exact_user_turn_source",
        "checkpoint_goal_context_identity",
        "canonical_checkpoint_goal_statement",
        "fresh_session_recall_goal_identity_and_statement",
    ):
        if linkage not in definition:
            raise AssertionError(f"Phase 8 definition is missing objective linkage {linkage}")
    if "rehearse_target(kind, cycle_root, recorder, base_env, None)" not in source:
        raise AssertionError("Phase 8 deterministic V11 coverage may not launch Codex")
    result = subprocess.run(
        [sys.executable, str(HARNESS), "self-test"],
        cwd=ROOT,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"Phase 8 harness self-test failed with exit {result.returncode}")
    print("phase 8 dogfood assertions passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
