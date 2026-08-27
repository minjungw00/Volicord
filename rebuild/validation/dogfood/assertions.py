#!/usr/bin/env python3
"""Self-test the Phase 8 dogfood evaluation support boundary."""

from __future__ import annotations

import ast
import json
import re
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
HARNESS = HERE / "harness.py"
CAMPAIGN = HERE / "campaign.py"
CODEX_EVENTS = HERE / "codex_events.py"
DEFINITION = HERE / "evaluation.json"
CURRENT_MCP_FIXTURE = HERE / "fixtures/current-codex-mcp-completion.jsonl"
HOST_MCP = ROOT / "rebuild/crates/volicord-host/src/mcp.rs"
OPERATIONS = ROOT / "rebuild/crates/volicord-operations/src/operations.rs"
REVIEWER_SAFE_CONTRACTS = (
    ROOT / "rebuild/docs/design/validation-plan.md",
    ROOT / "rebuild/docs/design/cutover-plan.md",
    ROOT / "rebuild/validation/README.md",
    ROOT / "rebuild/validation/phase-8-summary.md",
    ROOT / "rebuild/validation/dogfood/report.md",
)


def main() -> int:
    source = HARNESS.read_text(encoding="utf-8")
    campaign_source = CAMPAIGN.read_text(encoding="utf-8")
    event_source = CODEX_EVENTS.read_text(encoding="utf-8")
    host_source = HOST_MCP.read_text(encoding="utf-8")
    compact_host_source = re.sub(r"\s+", "", host_source)
    operations_source = OPERATIONS.read_text(encoding="utf-8")
    definition = DEFINITION.read_text(encoding="utf-8")
    definition_value = json.loads(definition)
    if "qualification_behavior_multiset" in definition_value:
        raise AssertionError("reviewer-safe evaluation definition exposes the behavior histogram")
    profile_contract = definition_value.get("qualification_profile_contract")
    if not isinstance(profile_contract, dict) or any(
        "behavior" in key or "hidden" in key or "repository" in key
        for key in profile_contract
    ):
        raise AssertionError("reviewer-safe evaluation definition exposes profile composition")
    old_profile_patterns = (
        r"one\s+explicit.{0,80}two\s+hidden",
        r"exactly\s+one\s+explicit",
        r"exactly\s+two\s+hidden",
        r"hidden\s+(?:assignments|cycles).{0,80}(?:span|across)\s+two\s+repository",
        r"hidden_user_owned_decision.{0,80}정확히\s*두\s*번",
        r"hidden\s*두\s*sample",
        r"나머지\s*behavior는\s*각각\s*한\s*번",
    )
    for path in REVIEWER_SAFE_CONTRACTS:
        text = path.read_text(encoding="utf-8")
        if "qualification_behavior_multiset" in text or any(
            re.search(pattern, text, flags=re.IGNORECASE | re.DOTALL)
            for pattern in old_profile_patterns
        ):
            raise AssertionError(
                f"reviewer-safe maintained contract exposes the realized behavior profile: {path}"
            )
    tree = ast.parse(source)
    for node in ast.walk(tree):
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Name)
            and node.func.id == "unique_call"
            and len(node.args) >= 2
            and isinstance(node.args[1], ast.Constant)
            and node.args[1].value == "repository_analyze"
        ):
            raise AssertionError(
                "Dogfood qualification still assumes one successful repository analysis"
            )
    if "exactly one pre-work repository analysis baseline" in campaign_source:
        raise AssertionError("campaign resume inspection still requires one analysis call")
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
        "materiality_review",
        "pre_write_materiality_work_authority",
        "submit_question_from_materiality",
        "resume_materiality_work_authority",
        "checkpoint_verifications",
        "current_host_user_turn",
        "work_user_task",
        "fresh_resume_user_task",
        "evaluation_basis",
        "behavior_class",
        "behavior_review",
        "blind_first_review_errors",
        "phase8_blind_review_preparation",
        "phase8_provisional_behavior_review",
        "classification_comparison",
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
    if "evaluate_work_authority(" in host_source:
        raise AssertionError("the MCP host fabricates work-authority policy")
    for marker in (
        "evaluate_work_authority(",
        "workflow_from_authority(",
        "workflow_for_review_candidate(",
        "workflow_for_question_candidate(",
        "workflow_for_work_basis(",
    ):
        if marker not in operations_source:
            raise AssertionError(
                f"production Operations no longer derives workflow through {marker}"
            )
    for delegation in (
        ".operations.record_materiality_review(",
        ".operations.workflow_for_review_candidate(",
        ".operations.workflow_for_question_candidate(",
        ".operations.workflow_for_work_basis(",
    ):
        if delegation not in compact_host_source:
            raise AssertionError(
                f"the MCP host no longer delegates workflow derivation through {delegation}"
            )
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
        or real_session.get("full_replacement_session_count") != 12
        or definition_value.get("cycles_per_repository") != 2
        or definition_value.get("qualification_cycle_count") != 6
        or definition_value.get("qualification_profile_contract")
        != {
            "visibility": "evaluator_steward_private_until_all_provisionals_recorded",
            "reveal_requires_provisional_count": 6,
            "validation_phase": "post_reveal_before_sealing",
            "reviewer_safe_profile_disclosure": False,
        }
        or tuple(definition_value.get("behavior_classes", [])) != (
            "explicit_user_owned_decision",
            "hidden_user_owned_decision",
            "research_or_no_question",
            "delegated_implementation_choice",
            "exploratory_uncertainty",
        )
        or len(definition_value.get("repository_classes", {})) != 3
        or len(definition_value["repository_classes"])
        * definition_value["cycles_per_repository"]
        * real_session["required_codex_sessions_per_cycle"]
        != 12
    ):
        raise AssertionError("Phase 8 no longer requires twelve distinct real Codex sessions")
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
            "CRLF-to-LF normalization and removal of only terminal CR/LF characters"
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
        "the first captured user turn matches the descriptor plain work_user_task after comparison-only CRLF-to-LF normalization and removal of terminal CR/LF characters",
        "does not disclose Recall",
        "resolves the repository-bound existing Project through project_resolve before Recall",
        "a fresh resume session invokes Recall after project_resolve",
        "record a typed Materiality Review bound to the exact Goal and pre-work Analysis Snapshot before the first affected ordinary write",
        "reuse the unresolved review dimension in the Question Candidate",
        "recompute Materiality Review/work authority before continued ordinary work",
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
    comparison = behavior_review.get("classification_comparison", {})
    blind_first = behavior_review.get("blind_first_review", {})
    counterfactual = behavior_review.get("material_user_owned_counterfactual_review", {})
    if (
        behavior_review.get("required_independent_review_fields")
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
        or blind_first.get("evaluator_material_visible_before_provisional_fix") is not False
        or blind_first.get("logical_identity_visible_before_provisional_fix") is not False
        or blind_first.get("reviewer_order") != "opaque_review_slot_id"
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
        or blind_first.get("required_provisional_count_before_reveal") != 6
        or blind_first.get("sealing_accepts_provisional_payload") is not False
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
        or agreement.get("sealing_blocked_status") != "unresolved_conflict"
        or set(agreement.get("accepted_statuses", []))
        != {"agreed", "resolved_from_evidence"}
        or comparison.get("sealing_blocked_status") != "unresolved_conflict"
        or set(comparison.get("accepted_statuses", []))
        != {"agreed", "resolved_from_evidence"}
        or comparison.get("mechanical_disagreement_fields")
        != [
            "classification",
            "materiality_conclusion",
            "material_outcome_unavoidable",
            "operator_prompt_disclosure",
        ]
        or comparison.get("provisional_artifact_rewritten") is not False
        or counterfactual.get("applicability")
        != "required_for_material_user_owned_decision"
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
    opaque_slots = real_session.get("opaque_slot_contract", {})
    if (
        opaque_slots.get("identity_generation")
        != "campaign_time_cryptographic_random_128_bit_token"
        or opaque_slots.get("derived_from_repository_or_cycle") is not False
        or opaque_slots.get("physical_workspace_layout")
        != "slots/<review_slot_id>/repository"
        or opaque_slots.get("reviewer_workspace_layout")
        != "reviewer/workspaces/<review_slot_id>/repository"
        or opaque_slots.get("private_mapping_integrity")
        != "campaign_bound_sha256_and_evidence_inventory"
        or opaque_slots.get("numeric_compatibility_branch") is not False
    ):
        raise AssertionError("Phase 8 opaque review-slot contract is incomplete")
    for marker in (
        "secrets.token_hex(16)",
        "phase8_dogfood_opaque_slot_mapping",
        "opaque_review_slot_id",
        "reviewer/workspaces",
        "record-provisional-review",
        "provisional_recorded",
        "reveal-qualification-profile",
        "all six provisional reviews",
        "qualification_profile_state",
    ):
        if marker not in campaign_source:
            raise AssertionError(f"Dogfood campaign helper is missing opaque-slot boundary {marker}")
    for stale_public_identity in (
        'root / "cycles"',
        'f"{cycle_key(kind, cycle)}.json"',
        'f"## {kind} — cycle {cycle}',
        'f"# Generated document review: {kind} cycle {cycle}',
    ):
        if stale_public_identity in campaign_source:
            raise AssertionError(
                f"Dogfood reviewer/operator path retains fixed-cycle identity: {stale_public_identity}"
            )
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
        or batch_contract.get("required_raw_rollout_count") != 12
        or batch_contract.get("global_mapping_precedes_campaign_mutation") is not True
        or batch_contract.get("terminal_work_failure_repaired_by_resume") is not False
        or "read_only_static_viewer_snapshot"
        not in batch_contract.get("automatic_cycle_evidence", [])
    ):
        raise AssertionError("Phase 8 batch campaign contract is incomplete")
    human_review = definition_value.get("human_review_contract", {})
    behavior_criteria = human_review.get("interaction_behavior_criterion_contracts", {})
    material_grounding = human_review.get("material_completeness_grounding", {})
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
        or set(human_review.get("interaction_behavior_criteria", []))
        != {
            "explicit_material_handling_quality",
            "hidden_material_discovery_quality",
            "unnecessary_interruption",
        }
        or set(behavior_criteria) != set(human_review.get("interaction_behavior_criteria", []))
        or behavior_criteria.get("explicit_material_handling_quality", {}).get("applies_to")
        != ["explicit_user_owned_decision"]
        or behavior_criteria.get("hidden_material_discovery_quality", {}).get("applies_to")
        != ["hidden_user_owned_decision"]
        or set(behavior_criteria.get("unnecessary_interruption", {}).get("applies_to", []))
        != {
            "research_or_no_question",
            "delegated_implementation_choice",
            "exploratory_uncertainty",
        }
        or any(
            contract.get("exact_evaluator_wording_required") is not False
            or contract.get("exact_evaluator_alternatives_required") is not False
            or contract.get("exact_expected_user_answer_required") is not False
            or contract.get("semantically_equivalent_decomposition_allowed") is not True
            or not isinstance(contract.get("review_prompt"), str)
            or "independently material" not in contract.get("review_prompt", "")
            for criterion, contract in behavior_criteria.items()
            if criterion != "unnecessary_interruption"
        )
        or material_grounding.get("available_only_after_naturalistic_execution") is not True
        or material_grounding.get("operator_task_or_work_resume_session_visibility") is not False
        or material_grounding.get("possible_material_concerns_are_exhaustive") is not False
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
