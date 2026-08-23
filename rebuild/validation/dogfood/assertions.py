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
        "evaluation_basis",
        "behavior_class",
        "behavior_review",
        "fact_authority_agreement",
        "counterfactual_review",
        "fully_satisfies_without_user_owned_outcome",
        "unavoidable_user_owned_outcome",
        "research_or_no_question",
        "delegated_implementation_choice",
        "exploratory_uncertainty",
        "campaign_preparation_independent_reviewer",
        "terminal_checkpoint_call",
        "verified_state_continuation",
        "repository_scoped_activation_observed",
        "operator_environment_invalid",
        "deterministic_human_review_samples",
        "human_review_template",
        "validate_human_review_artifact",
        "combine_human_review",
        "project_resolve",
        "repository_bound_project_resolution",
        "qualify-work-blocker",
        "phase8_dogfood_blocker_result",
        "naturalistic_prompt_integrity",
        "task_goal_basis",
        "inquiry_behavior_basis",
        "continuation_basis",
        "check-descriptors",
        "batch_campaign_contract",
        "automated_qualification",
        "replacement_qualification",
    ):
        if marker not in source and marker not in event_source:
            raise AssertionError(f"Phase 8 content normalizer is missing {marker}")
    if "verified_capture" in source or "first_repository_inspection_sequence" in source:
        raise AssertionError("the declaration-only real-session path remains active")
    for obsolete in (
        "def observation_template(",
        "def validate_observation_object(",
        "permitted_accessibility_observations",
        '"observation_schema"',
    ):
        if obsolete in source or obsolete in definition:
            raise AssertionError(f"obsolete per-cycle review compatibility remains: {obsolete}")
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
        or real_session.get("full_replacement_session_count") != 24
        or definition_value.get("candidate_cycle_count") != 4
        or tuple(definition_value.get("behavior_classes", [])) != (
            "user_owned_decision",
            "research_or_no_question",
            "delegated_implementation_choice",
            "exploratory_uncertainty",
        )
        or len(definition_value.get("repository_classes", {})) != 3
        or len(definition_value["repository_classes"])
        * definition_value["candidate_cycle_count"]
        * real_session["required_codex_sessions_per_cycle"]
        != 24
    ):
        raise AssertionError("Phase 8 no longer requires twenty-four distinct real Codex sessions")
    small_rules = definition_value["repository_classes"]["small-python"]
    polyglot_rules = definition_value["repository_classes"]["polyglot-medium"]
    if (
        small_rules.get("minimum_files", 0) < 8
        or small_rules.get("production_source_files_required", 0) < 3
        or small_rules.get("test_files_required", 0) < 2
        or small_rules.get("configuration_required") is not True
        or small_rules.get("trivial_arithmetic_or_example_disallowed") is not True
        or polyglot_rules.get("component_boundary_required") is not True
        or polyglot_rules.get("cross_language_config_api_or_process_work_required") is not True
    ):
        raise AssertionError("Phase 8 repository suitability contracts are incomplete")
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
        "numeric exit_code from the same captured command result",
        '"complete_result_evidence"',
        '"correlated_split_evidence"',
        '"output_only_outcome": "unknown"',
        '"uncorrelated_or_synthesized_status_outcome": "unknown"',
        "the first captured user turn matches the descriptor plain work_user_task exactly or after removing at most one Codex transport terminal LF or CRLF",
        "does not disclose Recall",
        "resolves the repository-bound existing Project through project_resolve before Recall",
        "a fresh resume session invokes Recall after project_resolve",
        "event_msg.mcp_tool_call_end",
        "permit one or more successful work Checkpoints",
        "latest terminal Checkpoint candidate after the last meaningful repository change",
        "change continuation produces a relevant repository change",
        "verified-state continuation requires a recalled completed Checkpoint",
    ):
        if forwarding_requirement not in definition:
            raise AssertionError(
                f"Phase 8 definition is missing forwarding requirement {forwarding_requirement}"
            )
    if "MCP_WRAPPER" not in event_source or "normalize_mcp_completion" not in event_source:
        raise AssertionError("Phase 8 MCP completion normalization boundary is missing")
    if "correlated_split" not in event_source or "custom_correlated_command_result" not in event_source:
        raise AssertionError("Phase 8 correlated command-result normalization boundary is missing")
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
    for basis_field in (
        "repository_facts",
        "accepted_contract_constraints",
        "delegated_boundaries",
        "possible_material_concerns",
        "consequences",
        "facts_not_for_user",
        "current_relevance",
    ):
        if basis_field not in definition:
            raise AssertionError(f"Phase 8 definition is missing evaluation-basis field {basis_field}")
    behavior_review = real_session.get("behavior_review", {})
    agreement = behavior_review.get("fact_authority_agreement", {})
    counterfactual = behavior_review.get("user_owned_counterfactual_review", {})
    if (
        behavior_review.get("required_independent_review_fields")
        != [
            "status",
            "reviewer_role",
            "basis",
            "fact_authority_agreement",
            "counterfactual_review",
        ]
        or agreement.get("sealing_blocked_status") != "unresolved_conflict"
        or set(agreement.get("accepted_statuses", []))
        != {"agreed", "resolved_from_evidence"}
        or counterfactual.get("applicability") != "required_for_user_owned_decision"
        or counterfactual.get("accepted_conclusion") != "unavoidable_user_owned_outcome"
        or counterfactual.get("rejecting_task_satisfaction")
        != "fully_satisfies_without_user_owned_outcome"
        or counterfactual.get("question_wording_prescribed") is not False
        or counterfactual.get("alternatives_prescribed") is not False
        or counterfactual.get("user_selection_prescribed") is not False
        or behavior_review.get("non_user_decision_counterfactual_applicability")
        != "not_required_for_behavior_class"
    ):
        raise AssertionError("Phase 8 independent counterfactual-review contract is incomplete")
    blocker_contract = real_session.get("work_blocker_qualification", {})
    if blocker_contract != {
        "subcommand": "qualify-work-blocker",
        "result_kind": "phase8_dogfood_blocker_result",
        "failure_only": True,
        "campaign_complete": False,
        "replacement_pass_candidate": False,
        "phase_9_ready": False,
        "later_evidence_status": "not_run",
        "missing_activation_outcome": "operator_environment_invalid",
    }:
        raise AssertionError("Phase 8 failure-only work-blocker contract is incomplete")
    batch_contract = real_session.get("batch_campaign_contract", {})
    if (
        batch_contract.get("operation") != "collect-batch"
        or batch_contract.get("required_raw_rollout_count") != 24
        or batch_contract.get("global_mapping_precedes_campaign_mutation") is not True
        or batch_contract.get("terminal_work_failure_repaired_by_resume") is not False
        or "read_only_static_viewer_snapshot"
        not in batch_contract.get("automatic_cycle_evidence", [])
    ):
        raise AssertionError("Phase 8 batch campaign contract is incomplete")
    human_review = definition_value.get("human_review_contract", {})
    if (
        human_review.get("artifact_kind") != "phase8_dogfood_human_review"
        or human_review.get("machine_accessibility_may_be_overridden") is not False
        or human_review.get("sampling_algorithm")
        != "every_automated_passed_interaction_cycle"
        or human_review.get("every_cycle_review_surfaces")
        != [
            "interaction",
            "generated_documents",
            "viewer_snapshot",
            "repository_intelligence",
            "cli_usability",
        ]
        or set(human_review.get("live_viewer_locales", [])) != {"en", "ko"}
    ):
        raise AssertionError("Phase 8 campaign-level human-review contract is incomplete")
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
