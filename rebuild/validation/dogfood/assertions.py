#!/usr/bin/env python3
"""Self-test the Phase 8 dogfood evaluation support boundary."""

from __future__ import annotations

import ast
import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
HARNESS = HERE / "harness.py"
CODEX_EVENTS = HERE / "codex_events.py"
DEFINITION = HERE / "evaluation.json"
CURRENT_MCP_FIXTURE = HERE / "fixtures/current-codex-mcp-completion.jsonl"


def main() -> int:
    source = HARNESS.read_text(encoding="utf-8")
    event_source = CODEX_EVENTS.read_text(encoding="utf-8")
    definition = DEFINITION.read_text(encoding="utf-8")
    definition_value = json.loads(definition)
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
        "mcp_tool_call_end",
        "context_record",
        "baseline_analysis_snapshot_id",
        "checkpoint_verifications",
        "current_host_user_turn",
        "work_user_task",
        "fresh_resume_user_task",
        "decision_oracle",
        "work_task_materiality_basis",
        "project_resolve",
        "repository_bound_project_resolution",
        "qualify-work-blocker",
        "phase8_dogfood_blocker_result",
        "naturalistic_prompt_integrity",
        "task_goal_basis",
        "question_relevance_review",
        "continuation_basis",
        "check-descriptors",
    ):
        if marker not in source and marker not in event_source:
            raise AssertionError(f"Phase 8 content normalizer is missing {marker}")
    if "verified_capture" in source or "first_repository_inspection_sequence" in source:
        raise AssertionError("the declaration-only real-session path remains active")
    if 'payload_type == "function_call"' in event_source or "eval(" in event_source:
        raise AssertionError("the obsolete decoder or JavaScript evaluation path remains active")
    if 'cli_version ==' in source or 'cli_version ==' in event_source:
        raise AssertionError("Phase 8 may not dispatch capture parsing by numeric Codex version")
    for obsolete in (
        "PHASE8_OBJECTIVE_PREFIX",
        "MAX_PHASE8_OBJECTIVE_BYTES",
        "phase8_objective_from_turns",
        "Phase8Objective",
        "normalized_resume_change_scope",
        "next_step_reserves_change",
    ):
        if obsolete in source or obsolete in event_source:
            raise AssertionError(f"obsolete scripted Phase 8 mechanism remains active: {obsolete}")
    if "--authorize-codex-transmission" in source:
        raise AssertionError("the superseded project-health-only Phase 8 assertion remains")
    if '"codex_transmission"' in definition or "project-health-six-real-repository-cycles" in definition:
        raise AssertionError("the superseded Phase 8 transmission contract remains")
    if "verify_repository_normalized_codex_rollout_and_canonical_bundle" not in definition:
        raise AssertionError("Phase 8 definition does not select content-normalized evidence")
    real_session = definition_value["real_session_evidence"]
    if (
        real_session.get("required_codex_sessions_per_cycle") != 2
        or real_session.get("full_replacement_session_count") != 12
        or definition_value.get("candidate_cycle_count") != 2
        or len(definition_value.get("repository_classes", {})) != 3
        or len(definition_value["repository_classes"])
        * definition_value["candidate_cycle_count"]
        * real_session["required_codex_sessions_per_cycle"]
        != 12
    ):
        raise AssertionError("Phase 8 no longer requires twelve distinct real Codex sessions")
    resources = definition_value.get("resource_qualification", {})
    if (
        resources.get("supported_operating_system") != "Linux"
        or resources.get("peak_memory_mechanism")
        != "linux_procfs_process_tree_rss_sampling"
        or resources.get("repeated_resource_repetition_count", 0) < 3
        or resources.get("universal_product_ceiling_applied") is not False
    ):
        raise AssertionError("Phase 8 resource qualification definition is incomplete")
    accessibility = definition_value.get("accessibility_machine_contract", {})
    if (
        "meaningful_visible_text" not in accessibility.get("button_names", [])
        or "aria_labelledby" not in accessibility.get("visible_form_control_names", [])
        or accessibility.get("manual_observation_may_override_deterministic_failure")
        is not False
    ):
        raise AssertionError("Phase 8 accessible-name qualification definition is incomplete")
    transport_identity = definition_value.get("real_session_evidence", {}).get(
        "codex_user_turn_transport_identity", {}
    )
    if transport_identity != {
        "captured_text_allowance": (
            "exact text or removal of at most one terminal LF or one terminal CRLF"
        ),
        "descriptor_task_mutated": False,
        "raw_capture_mutated": False,
        "evidence_sha256_mutated": False,
        "other_whitespace_normalized": False,
    }:
        raise AssertionError("Phase 8 Codex user-turn transport identity contract changed")
    for marker in (
        "control_has_accessible_name",
        "aria-labelledby",
        "aria-label",
        "hidden_control_count",
        "unlabeled_control_count",
        "LinuxProcessTreePeakRss",
        "linux_process_tree_procfs_unavailability",
        "repeated_resource_rehearsal",
        "rehearsal_destination_preexisting",
        "failed_document_export_created_unowned_destination",
        "unexplained_cumulative_growth_observed",
        "universal_product_ceiling_applied",
        "codex_user_turn_transport_identity_matches",
    ):
        if marker not in source:
            raise AssertionError(f"Phase 8 qualification support is missing {marker}")
    for forwarding_requirement in (
        "commands used only for incidental inspection",
        "full-result text(r) forwarding",
        '"full_result_wrapper": "text(r)"',
        '"output_only_wrapper": "text(r.output)"',
        '"output_only_outcome": "unknown"',
        "the first captured user turn matches the descriptor plain work_user_task exactly or after removing at most one Codex transport terminal LF or CRLF",
        "does not disclose Recall",
        "resolves the repository-bound existing Project through project_resolve before Recall",
        "a fresh resume session invokes Recall after project_resolve",
        "event_msg.mcp_tool_call_end",
        "meaningful observed repository changes relevant to the recalled Checkpoint",
        "the resume session preserves separate full-result numeric-exit validation",
    ):
        if forwarding_requirement not in definition:
            raise AssertionError(
                f"Phase 8 definition is missing forwarding requirement {forwarding_requirement}"
            )
    if "MCP_WRAPPER" not in event_source or "normalize_mcp_completion" not in event_source:
        raise AssertionError("Phase 8 MCP completion normalization boundary is missing")
    if "parsed.tool_name.startswith" in event_source:
        raise AssertionError("custom wrapper output remains an MCP semantic source")
    for linkage in (
        "descriptor_plain_work_user_task",
        "first_work_session_user_task_turn_transport_identity_match",
        "evaluated_repository_revision",
        "context_record_exact_user_turn_source",
        "canonical_goal_identity_and_statement",
        "checkpoint_goal_context_identity",
        "fresh_session_recall_same_goal_identity_and_materially_consistent_statement",
    ):
        if linkage not in definition:
            raise AssertionError(f"Phase 8 definition is missing plain-task Goal linkage {linkage}")
    for oracle_field in (
        "work_task_materiality_basis",
        "user_owned_dimension",
        "established_repository_facts",
        "why_repository_inspection_cannot_decide",
        "viable_alternatives",
        "recommendation",
        "material_consequence",
    ):
        if oracle_field not in definition:
            raise AssertionError(f"Phase 8 definition is missing hidden oracle field {oracle_field}")
    blocker_contract = real_session.get("work_blocker_qualification", {})
    if blocker_contract != {
        "subcommand": "qualify-work-blocker",
        "result_kind": "phase8_dogfood_blocker_result",
        "failure_only": True,
        "campaign_complete": False,
        "replacement_pass_candidate": False,
        "phase_9_ready": False,
        "later_evidence_status": "not_run",
    }:
        raise AssertionError("Phase 8 failure-only work-blocker contract is incomplete")
    if "rehearse_target(kind, cycle_root, recorder, base_env, None)" not in source:
        raise AssertionError("Phase 8 deterministic V11 coverage may not launch Codex")
    fixture_source = CURRENT_MCP_FIXTURE.read_text(encoding="utf-8")
    for marker in ("text(JSON.stringify(x))", '"type":"mcp_tool_call_end"', '"server":"volicord"'):
        if marker not in fixture_source:
            raise AssertionError(f"current Codex MCP completion fixture is missing {marker}")
    result = subprocess.run(
        [sys.executable, "-B", str(HARNESS), "self-test"],
        cwd=ROOT,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"Phase 8 harness self-test failed with exit {result.returncode}")
    print("phase 8 dogfood assertions passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
