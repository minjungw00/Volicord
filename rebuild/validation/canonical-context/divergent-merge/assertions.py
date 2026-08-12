#!/usr/bin/env python3
"""V04 orchestration for production Rust merge and recovery assertions."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shlex
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[4]
MANIFEST = ROOT / "rebuild/validation/shared/fixture-manifest.json"
FIXTURE_ROOT = ROOT / "rebuild/validation/canonical-context/divergent-merge/fixtures/v04-scenarios"
SCENARIO = FIXTURE_ROOT / "scenario.json"
MANIFEST_PATH = "rebuild/validation/canonical-context/divergent-merge/fixtures/v04-scenarios"

SEMANTIC_TESTS = {
    "trustworthy_common_base": "verified_base_auto_merges_independent_additions_and_replays",
    "independent_additions": "verified_base_auto_merges_independent_additions_and_replays",
    "compatible_non_semantic_correction": "one_sided_correction_is_bounded_automatic_but_question_state_is_user_owned",
    "same_record_revision_conflict": "competing_context_corrections_require_user_resolution",
    "question_state_conflict": "one_sided_correction_is_bounded_automatic_but_question_state_is_user_owned",
    "semantic_decision_conflict": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
    "context_item_delete_modify_choose_forgotten": "delete_modify_binding_and_unavailable_base_are_never_automatic",
    "context_item_delete_modify_choose_modified": "delete_modify_binding_and_unavailable_base_are_never_automatic",
    "source_local_modify_incoming_forget": "incoming_forgetting_sanitizes_every_supported_local_record_closure",
    "question_local_modify_incoming_forget": "incoming_forgetting_sanitizes_every_supported_local_record_closure",
    "decision_local_modify_incoming_forget": "incoming_forgetting_sanitizes_every_supported_local_record_closure",
    "context_item_local_modify_incoming_forget": "incoming_forgetting_sanitizes_every_supported_local_record_closure",
    "checkpoint_incoming_forget": "incoming_forgetting_sanitizes_every_supported_local_record_closure",
    "decision_delete_modify_choose_forgotten": "decision_delete_modify_selects_one_complete_closure_and_rolls_back_atomically",
    "decision_delete_modify_choose_modified": "decision_delete_modify_selects_one_complete_closure_and_rolls_back_atomically",
    "source_forgetting_modify": "source_question_and_checkpoint_forgetting_propagate_through_merge",
    "question_forgetting_modify": "source_question_and_checkpoint_forgetting_propagate_through_merge",
    "checkpoint_forgetting_modify": "source_question_and_checkpoint_forgetting_propagate_through_merge",
    "supersede_decision_source_forgetting": "supersession_source_payload_is_purged_by_source_only_forgetting",
    "question_only_forgetting_surviving_decision": "question_only_forgetting_purges_owned_decision_presentation",
    "supersede_supersede": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
    "source_binding_conflict": "delete_modify_binding_and_unavailable_base_are_never_automatic",
    "common_base_unavailable": "delete_modify_binding_and_unavailable_base_are_never_automatic",
    "source_repository_unavailable": "delete_modify_binding_and_unavailable_base_are_never_automatic",
}

RESOLUTION_TESTS = {
    "choose_local": "decision_delete_modify_selects_one_complete_closure_and_rolls_back_atomically",
    "choose_incoming": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
    "explicit_merged_result": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
    "context_branch_result": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
}

REQUIREMENT_TESTS = {
    "exact_current_host_user_turn_source": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
    "exact_conflict_set_identity": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
    "exact_conflict_revision": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
}

POST_MERGE_TESTS = {
    "merge_provenance": "merge_and_branch_basis_remain_ordered_and_portable_without_sources",
    "post_merge_deterministic_export_import": "merge_and_branch_basis_remain_ordered_and_portable_without_sources",
    "post_merge_deterministic_canonical_read": "merge_and_branch_basis_remain_ordered_and_portable_without_sources",
    "deletion_propagation": "delete_modify_binding_and_unavailable_base_are_never_automatic",
    "no_forgotten_content_resurrection": "every_content_bearing_kind_forgets_to_one_minimal_portable_tombstone",
    "bundle_format_interaction": "corruption_and_newer_version_fail_before_any_mutation",
    "operation_replay": "verified_base_auto_merges_independent_additions_and_replays",
    "changed_replay_rejection": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
    "operation_dependency_cleanup": "supersession_source_payload_is_purged_by_source_only_forgetting",
    "copied_question_content_absence": "question_only_forgetting_purges_owned_decision_presentation",
    "managed_bundle_refresh": "merge_selected_forgetting_requires_and_recovers_managed_sanitation",
    "database_wal_temp_residue_absence": "merge_selected_forgetting_requires_and_recovers_managed_sanitation",
    "sanitation_failure_not_success": "merge_selected_forgetting_requires_and_recovers_managed_sanitation",
    "sanitation_recovery_replay": "merge_selected_forgetting_requires_and_recovers_managed_sanitation",
}

RECOVERY_TESTS = {
    "transaction_termination_before_commit": "hard_termination_preserves_only_committed_operations",
    "termination_after_commit_before_response": "hard_termination_preserves_only_committed_operations",
    "bundle_publication_interruption": "process_faults_preserve_published_bundle_and_prior_import_state",
    "import_interruption_before_mutation": "process_faults_preserve_published_bundle_and_prior_import_state",
    "merge_interruption": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
    "merge_forgetting_interruption_before_commit": "merge_selected_forgetting_requires_and_recovers_managed_sanitation",
    "merge_forgetting_interruption_after_commit": "merge_selected_forgetting_requires_and_recovers_managed_sanitation",
    "forgetting_failure": "new_forgetting_kinds_roll_back_their_complete_closure_and_retry_safely",
    "committed_state_reopen": "explicit_path_survives_process_reopen_without_cwd_or_runtime_home_discovery",
    "operation_duplicate_prevention": "operation_replay_preserves_prior_result_and_detects_mismatch_and_stale_basis",
    "managed_sensitive_residue_absence": "every_content_bearing_kind_forgets_to_one_minimal_portable_tombstone",
    "managed_merge_residue_absence": "incoming_forgetting_sanitizes_every_supported_local_record_closure",
    "bundle_local_absolute_path_absence": "repeated_export_is_identical_and_excludes_local_and_noncanonical_classes",
    "bundle_candidate_absence": "repeated_export_is_identical_and_excludes_local_and_noncanonical_classes",
    "bundle_derived_state_absence": "repeated_export_is_identical_and_excludes_local_and_noncanonical_classes",
}

PORTABLE_INVARIANT_TESTS = {
    "valid_original_answered_decision": "direct_decision_admission_rejects_the_same_representative_semantics",
    "valid_delegation": "records_explicit_delegation_as_a_decision",
    "valid_correction_lineage": "valid_direct_decision_lineages_pass_every_portable_forgetting_boundary",
    "valid_supersession_lineage": "valid_direct_decision_lineages_pass_every_portable_forgetting_boundary",
    "valid_source_forgotten_decision": "valid_direct_decision_lineages_pass_every_portable_forgetting_boundary",
    "valid_question_forgotten_decision": "valid_direct_decision_lineages_pass_every_portable_forgetting_boundary",
    "file_source_decision_authority": "portable_rejects_file_source_as_decision_authority",
    "agent_authored_host_turn_decision_authority": "portable_rejects_agent_authored_host_turn_as_decision_authority",
    "nonexistent_question_revision": "portable_rejects_nonexistent_question_revision",
    "alternative_outside_question_revision": "portable_rejects_alternative_outside_question_revision",
    "question_terminal_outcome_mismatch": "portable_rejects_question_outcome_and_response_link_mismatch",
    "question_response_link_absent": "portable_rejects_question_outcome_and_response_link_mismatch",
    "question_response_link_mismatch": "portable_rejects_mismatched_response_source_and_orphan_decision",
    "orphan_decision_without_response_or_supersession": "portable_rejects_mismatched_response_source_and_orphan_decision",
    "branching_or_cyclic_supersession": "portable_rejects_branching_and_cyclic_decision_supersession",
    "decision_current_snapshot_mismatch": "portable_rejects_current_snapshot_mismatch_and_semantic_correction",
    "decision_revision_semantic_mutation": "portable_rejects_current_snapshot_mismatch_and_semantic_correction",
    "decision_revision_provenance_mutation": "portable_rejects_decision_revision_provenance_mutation",
    "direct_write_portable_parity": "direct_decision_admission_rejects_the_same_representative_semantics",
    "non_user_authority_import_rejection": "portable_rejects_file_source_as_decision_authority",
    "non_user_authority_explicit_merged_rejection": "explicit_merged_rejects_non_user_decision_with_active_or_forgotten_local_source",
    "local_source_forgotten_before_explicit_rejection": "explicit_merged_rejects_non_user_decision_with_active_or_forgotten_local_source",
    "generated_merge_target_validation": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
    "non_user_internal_state_not_exported": "export_rejects_internal_non_user_decision_authority_without_republication",
    "decision_rejection_no_partial_mutation": "portable_rejects_mismatched_response_source_and_orphan_decision",
    "valid_sanitized_forgotten_question_import": "direct_question_forgetting_bundle_imports_with_sanitized_dependents",
    "invalid_import_active_decision_presentation": "import_rejects_recomputed_forgotten_question_presentation_before_mutation",
    "invalid_import_decision_revision_only_presentation": "import_rejects_revision_only_presentation_and_missing_review_due",
    "invalid_import_missing_review_due": "import_rejects_revision_only_presentation_and_missing_review_due",
    "invalid_explicit_merged_bundle": "explicit_merged_rejects_forgotten_question_presentation_before_mutation",
    "invalid_explicit_merged_local_question_already_forgotten": "explicit_merged_cannot_repopulate_an_already_forgotten_local_question",
    "rejection_preserves_canonical_and_operational_state": "import_rejects_recomputed_forgotten_question_presentation_before_mutation",
    "recomputed_checksum_and_history_basis": "import_rejects_recomputed_forgotten_question_presentation_before_mutation",
    "invalid_active_question_empty_presentation": "active_question_decision_with_empty_presentation_is_rejected",
    "valid_forgotten_question_without_active_decision": "forgotten_question_without_active_decision_does_not_require_review_due",
    "invalid_internal_state_not_exported": "export_rejects_internal_forgotten_question_state_missing_review_due",
    "valid_direct_forget_export_import_merge_and_read_unchanged": "direct_question_forgetting_bundle_imports_with_sanitized_dependents",
    "source_typed_payload_semantic_mutations": "source_context_and_checkpoint_semantic_mutations_reject_every_portable_write_boundary",
    "context_role_provenance_revision_mutations": "source_context_and_checkpoint_semantic_mutations_reject_every_portable_write_boundary",
    "checkpoint_state_source_semantic_mutations": "source_context_and_checkpoint_semantic_mutations_reject_every_portable_write_boundary",
}

INVARIANT_PARITY_TESTS = {
    "catalog_semantic_owner_and_test_anchors": "invariant_catalog_rows_have_semantic_owners_and_executable_anchors",
    "catalog_central_admission_paths": "every_full_state_admission_path_names_the_central_boundary",
    "command_full_validation_export_import_equivalence": "command_generated_state_round_trips_with_semantic_and_deterministic_equivalence",
    "witness_removal_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "unrelated_decision_tombstone_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "wrong_witness_question_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "wrong_witness_revision_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "wrong_witness_outcome_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "wrong_root_decision_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "duplicate_root_witness_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "missing_active_root_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "forgotten_root_without_matching_tombstone_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "unrelated_response_source_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "non_user_response_authority_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "supersession_branch_mutation": "portable_rejects_branching_and_cyclic_decision_supersession",
    "supersession_cycle_mutation": "portable_rejects_branching_and_cyclic_decision_supersession",
    "terminal_without_role_history_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "role_history_on_open_question_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "role_history_on_non_decision_question_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "tombstone_plus_content_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "decision_authority_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "question_revision_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "decision_alternative_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "decision_correction_semantic_mutation": "portable_rejects_current_snapshot_mismatch_and_semantic_correction",
    "operation_dependency_removal_after_forgetting": "supersession_source_payload_is_purged_by_source_only_forgetting",
    "local_binding_in_portable_bytes_mutation": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "mutation_import_no_partial_state": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "mutation_explicit_merge_no_partial_state": "canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary",
    "generated_target_and_replacement_validation": "semantic_decision_conflict_requires_exact_user_resolution_and_supports_branch",
    "isolated_export_inconsistent_state_refusal": "export_rejects_internal_missing_question_history_without_republication",
    "affected_semantics_import_no_partial_state": "source_context_and_checkpoint_semantic_mutations_reject_every_portable_write_boundary",
    "affected_semantics_explicit_merge_no_partial_state": "source_context_and_checkpoint_semantic_mutations_reject_every_portable_write_boundary",
    "affected_semantics_direct_and_export_rejection": "affected_semantic_corruption_blocks_direct_commit_and_production_export",
}

METAMORPHIC_TESTS = {
    "direct_command_export_import": "command_generated_state_round_trips_with_semantic_and_deterministic_equivalence",
    "decision_forget_export_import": "valid_direct_decision_lineages_pass_every_portable_forgetting_boundary",
    "source_forget_export_import": "valid_direct_decision_lineages_pass_every_portable_forgetting_boundary",
    "question_forget_export_import": "valid_direct_decision_lineages_pass_every_portable_forgetting_boundary",
    "local_forgetting_incoming_content": "incoming_forgetting_sanitizes_every_supported_local_record_closure",
    "local_content_incoming_forgetting": "incoming_forgetting_sanitizes_every_supported_local_record_closure",
    "merge_export_import_deterministic_read": "merge_and_branch_basis_remain_ordered_and_portable_without_sources",
    "replay_after_forgetting": "supersession_source_payload_is_purged_by_source_only_forgetting",
    "restart_after_direct_and_portable_operations": "explicit_path_survives_process_reopen_without_cwd_or_runtime_home_discovery",
    "restart_after_forgetting": "valid_direct_decision_lineages_pass_every_portable_forgetting_boundary",
    "restart_after_merge_and_recovery": "merge_selected_forgetting_requires_and_recovers_managed_sanitation",
}

TEST_TARGETS = (
    "canonical_read",
    "context_checkpoint",
    "divergent_merge",
    "forgetting_matrix",
    "inquiry",
    "invariant_catalog",
    "kernel",
    "lifecycle",
    "portable_bundle",
    "portable_invariants",
    "portable_process",
    "process_reopen",
    "transaction_process",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def fixture_hash(directory: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(item for item in directory.rglob("*") if item.is_file()):
        relative = path.relative_to(directory).as_posix().encode("utf-8")
        content = path.read_bytes()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def run(arguments: list[str], *, capture: bool = False) -> subprocess.CompletedProcess[str]:
    print(f"$ {shlex.join(arguments)}", flush=True)
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    if result.returncode != 0:
        if capture:
            print(result.stdout, end="", file=sys.stdout)
            print(result.stderr, end="", file=sys.stderr)
        raise RuntimeError(f"command failed with exit {result.returncode}: {shlex.join(arguments)}")
    return result


def require_scenarios(actual: object, expected: dict[str, str], field: str) -> None:
    require(isinstance(actual, list), f"{field} must be a list")
    require(set(actual) == set(expected), f"{field} does not match maintained V04 coverage")


def main() -> int:
    scenario = json.loads(SCENARIO.read_text(encoding="utf-8"))
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    require(scenario.get("validation_id") == "V04", "fixture validation_id must remain V04")
    require(scenario.get("external_semantic_authority") is False, "the harness must not own merge semantics")
    require_scenarios(scenario.get("semantic_scenarios"), SEMANTIC_TESTS, "semantic_scenarios")
    require_scenarios(scenario.get("resolution_modes"), RESOLUTION_TESTS, "resolution_modes")
    require_scenarios(
        scenario.get("resolution_requirements"), REQUIREMENT_TESTS, "resolution_requirements"
    )
    require_scenarios(scenario.get("post_merge_assertions"), POST_MERGE_TESTS, "post_merge_assertions")
    require_scenarios(scenario.get("recovery_scenarios"), RECOVERY_TESTS, "recovery_scenarios")
    require_scenarios(
        scenario.get("portable_invariant_scenarios"),
        PORTABLE_INVARIANT_TESTS,
        "portable_invariant_scenarios",
    )
    require_scenarios(
        scenario.get("invariant_parity_scenarios"),
        INVARIANT_PARITY_TESTS,
        "invariant_parity_scenarios",
    )
    require_scenarios(
        scenario.get("metamorphic_scenarios"),
        METAMORPHIC_TESTS,
        "metamorphic_scenarios",
    )

    entry = next(
        (item for item in manifest.get("fixtures", []) if item.get("id") == "v04-divergent-merge"),
        None,
    )
    require(entry is not None, "V04 fixture-manifest entry is missing")
    require(entry.get("validation_id") == "V04", "manifest validation_id must remain V04")
    require(entry.get("path") == MANIFEST_PATH, "manifest points to a different V04 fixture")
    actual_hash = fixture_hash(FIXTURE_ROOT)
    require(entry.get("content_sha256") == actual_hash, "V04 fixture hash does not match manifest")

    cargo = [
        "cargo",
        "test",
        "--manifest-path",
        "rebuild/Cargo.toml",
        "-p",
        "volicord-context",
    ]
    targets = [argument for target in TEST_TARGETS for argument in ("--test", target)]
    catalog = run([*cargo, *targets, "--", "--list"], capture=True).stdout
    mapped_tests = {
        *SEMANTIC_TESTS.values(),
        *RESOLUTION_TESTS.values(),
        *REQUIREMENT_TESTS.values(),
        *POST_MERGE_TESTS.values(),
        *RECOVERY_TESTS.values(),
        *PORTABLE_INVARIANT_TESTS.values(),
        *INVARIANT_PARITY_TESTS.values(),
        *METAMORPHIC_TESTS.values(),
    }
    missing = sorted(name for name in mapped_tests if f"{name}: test" not in catalog)
    require(not missing, f"mapped production Rust tests are missing: {missing}")
    production_tests = sum(line.endswith(": test") for line in catalog.splitlines())

    run([*cargo, *targets, "--", "--test-threads=1"])
    summary = {
        "fixture_sha256": actual_hash,
        "mapped_assertions": len(
            SEMANTIC_TESTS
            | RESOLUTION_TESTS
            | REQUIREMENT_TESTS
            | POST_MERGE_TESTS
            | RECOVERY_TESTS
            | PORTABLE_INVARIANT_TESTS
            | INVARIANT_PARITY_TESTS
            | METAMORPHIC_TESTS
        ),
        "production_test_targets": list(TEST_TARGETS),
        "production_tests": production_tests,
        "status": "passed",
        "validation_id": "V04",
    }
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
