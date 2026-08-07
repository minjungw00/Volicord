//! Canonical semantic identity for generated agent guidance.

use serde::Serialize;

use crate::canonical::canonical_json_sha256;
use crate::ids::RequestHash;

/// Closed semantic facts that every managed agent-guidance rendering preserves.
pub const MANAGED_GUIDANCE_FACTS: &[&str] = &[
    "method_failure_and_response_projection_failure_are_distinct",
    "applied_effect_prohibits_mutation_retry",
    "current_status_read_required_after_response_projection_failure",
    "exact_operation_result_retrieved_when_ref_exists",
    "new_commit_staging_effect_and_replay_are_distinct",
    "applied_effect_never_reports_unchanged_state",
    "post_effect_recovery_does_not_establish_successful_completion",
    "refreshed_status_preserves_current_workflow_authority",
];

/// Closed semantics whose change invalidates mutation-finalization identity.
pub const MUTATION_FINALIZATION_CONTRACT_FACTS: &[&str] = &[
    "pre_effect_internal_contract_failure_is_an_ordinary_error_without_effect",
    "committed_post_effect_failure_preserves_commit_effect_and_state_change",
    "staging_post_effect_failure_preserves_applied_effect_without_commit_or_state_change",
    "replayed_committed_result_preserves_effect_without_a_new_commit",
    "normal_no_effect_result_and_typed_rejection_remain_distinct",
    "workflow_contract_diagnostics_are_constructed_with_observed_effect_facts",
];

/// Closed semantics whose change invalidates post-effect response recovery.
pub const POST_EFFECT_RESPONSE_CONTRACT_FACTS: &[&str] = &[
    "method_failure_and_response_projection_failure_are_distinct",
    "post_effect_recovery_is_non_retryable",
    "post_effect_recovery_requires_current_status_read",
    "operation_result_ref_routes_exact_result_retrieval_when_present",
    "post_effect_recovery_withholds_successful_completion_claim",
    "current_workflow_authority_governs_after_refresh",
];

/// Closed semantics whose change invalidates the Core workflow contract.
pub const WORKFLOW_CONTRACT_FACTS: &[&str] = &[
    "core_owns_workflow_machine_and_transition_catalog",
    "catalog_contains_every_current_executable_transition",
    "catalog_contains_every_current_method_owned_semantic_variant",
    "catalog_contains_zero_or_one_required_transition",
    "required_transition_is_a_catalog_member",
    "catalog_contains_no_non_executable_task_bound_transition",
    "every_nonterminal_state_has_user_wait_agent_transition_or_terminal_path",
    "core_admission_consumes_exact_transition_descriptor",
    "every_transition_owns_a_method_and_state_specific_submission_contract",
    "no_commit_planning_accepts_only_closed_result_and_staging_branches",
    "no_commit_planning_rejection_is_a_typed_failure_not_a_plan",
    "no_commit_planning_stops_before_every_durable_transient_and_repository_effect",
    "transition_submission_contract_rejects_duplicated_authority_semantic_mismatches",
    "advisor_scope_submission_fixes_empty_paths_and_canonical_observe_only_effects",
    "retry_contract_projects_only_typed_core_recovery",
    "current_submitted_canonicality_match_and_compatibility_remain_distinct",
];

/// Closed semantics whose change invalidates transition-submission identity.
pub const SUBMISSION_CONTRACT_FACTS: &[&str] = &[
    "every_transition_owns_one_method_state_and_semantic_variant_specific_submission_contract",
    "submission_contracts_separate_fixed_required_and_optional_values",
    "fixed_values_are_owned_by_current_core_authority_coordinates",
    "required_and_optional_agent_inputs_are_task_mode_specific",
    "submission_contract_witnesses_are_bounded_and_typed",
    "advisor_scope_submission_fixes_empty_paths_and_canonical_observe_only_effects",
    "record_run_submission_fixes_current_task_change_unit_and_baseline_and_authors_kind",
    "record_run_validation_witness_kind_and_baseline_match_current_authority_coordinates",
    "checkpoint_and_change_unit_submission_variants_match_current_authority_operations",
    "checkpoint_stale_authority_witness_cardinality_matches_current_stale_authority",
    "method_examples_never_supply_submission_contract_values",
];

/// Closed semantics whose change invalidates MCP action-form identity.
pub const ACTION_FORM_CONTRACT_FACTS: &[&str] = &[
    "mcp_projects_each_descriptor_bound_action_form",
    "form_ref_binds_project_task_action_state_coordinates_submission_contract_workflow_action_form_semantic_schema_and_scalar_contracts",
    "every_agent_transition_has_exactly_one_executable_form",
    "fixed_and_agent_authored_inputs_derive_from_the_exact_submission_contract",
    "canonical_minimal_request_uses_only_bounded_submission_contract_witness_values",
    "method_examples_never_supply_live_action_form_policy_or_witness_values",
    "canonical_minimal_request_passes_schema_and_binding_validation",
    "complete_submission_witness_requires_an_accepted_exact_method_no_commit_plan",
    "one_rejected_agent_transition_witness_suppresses_the_entire_action_form_catalog",
    "published_action_form_keys_equal_all_current_agent_transition_keys",
    "task_bound_method_is_admitted_before_core",
    "allowed_task_bound_mutation_requires_exact_method_and_variant_form_ref",
    "action_form_never_authorizes_different_method_or_semantic_variant",
    "invalid_current_method_variant_rejects_before_core",
    "fixed_authority_arguments_must_be_present_and_deeply_equal",
    "authority_mirror_fields_are_not_public_mutation_inputs",
    "read_only_status_grants_no_mutation_authority",
    "invalid_arguments_load_only_independently_valid_authority_context",
    "missing_core_recovery_form_is_internal_contract_inconsistency",
    "pre_core_rejection_reports_no_commit_and_no_state_change",
    "method_planning_rejection_reports_typed_method_error_without_retry",
    "authority_basis_mismatch_reports_typed_expected_and_received_values",
];

/// Closed semantics whose change invalidates MCP semantic-schema identity.
pub const MCP_SEMANTIC_SCHEMA_FACTS: &[&str] = &[
    "rust_types_own_mcp_field_semantics",
    "type_owned_scalar_contracts_are_preserved",
    "required_nullable_semantics_are_generic",
    "tagged_unions_declare_explicit_discriminators",
    "validation_is_local_to_the_selected_union_branch",
    "runtime_discovery_is_bounded",
    "runtime_discovery_preserves_load_bearing_semantic_annotations",
    "canonical_examples_validate_and_decode_through_one_descriptor",
    "workflow_contract_failures_report_typed_stage_method_error_commit_and_side_effect_facts",
];

#[derive(Serialize)]
struct SemanticDigestBasis<'a> {
    domain: &'static str,
    facts: &'a [&'a str],
}

/// Returns the canonical semantic digest bound into project integration identity.
pub fn managed_guidance_semantic_digest() -> RequestHash {
    canonical_json_sha256(&SemanticDigestBasis {
        domain: "volicord.managed-agent-guidance",
        facts: MANAGED_GUIDANCE_FACTS,
    })
    .expect("static managed guidance semantics always serialize")
}

/// Returns the canonical semantic digest for mutation finalization.
pub fn mutation_finalization_contract_semantic_digest() -> RequestHash {
    canonical_json_sha256(&SemanticDigestBasis {
        domain: "volicord.mutation-finalization-contract",
        facts: MUTATION_FINALIZATION_CONTRACT_FACTS,
    })
    .expect("static mutation-finalization semantics always serialize")
}

/// Returns the canonical semantic digest for post-effect response recovery.
pub fn post_effect_response_contract_semantic_digest() -> RequestHash {
    canonical_json_sha256(&SemanticDigestBasis {
        domain: "volicord.post-effect-response-contract",
        facts: POST_EFFECT_RESPONSE_CONTRACT_FACTS,
    })
    .expect("static post-effect response semantics always serialize")
}

/// Returns the canonical semantic digest for the current Core workflow contract.
pub fn workflow_contract_semantic_digest() -> RequestHash {
    canonical_json_sha256(&SemanticDigestBasis {
        domain: "volicord.workflow-contract",
        facts: WORKFLOW_CONTRACT_FACTS,
    })
    .expect("static workflow contract semantics always serialize")
}

/// Returns the canonical semantic digest for current transition submissions.
pub fn submission_contract_semantic_digest() -> RequestHash {
    canonical_json_sha256(&SemanticDigestBasis {
        domain: "volicord.submission-contract",
        facts: SUBMISSION_CONTRACT_FACTS,
    })
    .expect("static submission contract semantics always serialize")
}

/// Returns the canonical semantic digest for current MCP action-form behavior.
pub fn action_form_contract_semantic_digest() -> RequestHash {
    canonical_json_sha256(&SemanticDigestBasis {
        domain: "volicord.action-form-contract",
        facts: ACTION_FORM_CONTRACT_FACTS,
    })
    .expect("static action-form contract semantics always serialize")
}

/// Returns the canonical semantic digest for current MCP schema behavior.
pub fn mcp_semantic_schema_digest() -> RequestHash {
    canonical_json_sha256(&SemanticDigestBasis {
        domain: "volicord.mcp-semantic-schema",
        facts: MCP_SEMANTIC_SCHEMA_FACTS,
    })
    .expect("static MCP semantic schema semantics always serialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_digest_is_canonical_and_fact_bound() {
        let digest = managed_guidance_semantic_digest();
        assert!(crate::canonical::is_canonical_sha256_digest(
            digest.as_str()
        ));
        let changed = canonical_json_sha256(&SemanticDigestBasis {
            domain: "volicord.managed-agent-guidance",
            facts: &MANAGED_GUIDANCE_FACTS[..MANAGED_GUIDANCE_FACTS.len() - 1],
        })
        .expect("test semantics serialize");
        assert_ne!(digest, changed);
    }

    #[test]
    fn workflow_contract_semantic_digest_is_canonical_and_fact_bound() {
        let digest = workflow_contract_semantic_digest();
        assert!(crate::canonical::is_canonical_sha256_digest(
            digest.as_str()
        ));
        let changed = canonical_json_sha256(&SemanticDigestBasis {
            domain: "volicord.workflow-contract",
            facts: &WORKFLOW_CONTRACT_FACTS[..WORKFLOW_CONTRACT_FACTS.len() - 1],
        })
        .expect("test workflow semantics serialize");
        assert_ne!(digest, changed);
    }

    #[test]
    fn mutation_finalization_contract_semantic_digest_is_canonical_and_fact_bound() {
        let digest = mutation_finalization_contract_semantic_digest();
        assert!(crate::canonical::is_canonical_sha256_digest(
            digest.as_str()
        ));
        let changed = canonical_json_sha256(&SemanticDigestBasis {
            domain: "volicord.mutation-finalization-contract",
            facts: &MUTATION_FINALIZATION_CONTRACT_FACTS
                [..MUTATION_FINALIZATION_CONTRACT_FACTS.len() - 1],
        })
        .expect("test mutation-finalization semantics serialize");
        assert_ne!(digest, changed);
    }

    #[test]
    fn post_effect_response_contract_semantic_digest_is_canonical_and_fact_bound() {
        let digest = post_effect_response_contract_semantic_digest();
        assert!(crate::canonical::is_canonical_sha256_digest(
            digest.as_str()
        ));
        let changed = canonical_json_sha256(&SemanticDigestBasis {
            domain: "volicord.post-effect-response-contract",
            facts: &POST_EFFECT_RESPONSE_CONTRACT_FACTS
                [..POST_EFFECT_RESPONSE_CONTRACT_FACTS.len() - 1],
        })
        .expect("test post-effect response semantics serialize");
        assert_ne!(digest, changed);
    }

    #[test]
    fn submission_contract_semantic_digest_is_canonical_and_fact_bound() {
        let digest = submission_contract_semantic_digest();
        assert!(crate::canonical::is_canonical_sha256_digest(
            digest.as_str()
        ));
        let changed = canonical_json_sha256(&SemanticDigestBasis {
            domain: "volicord.submission-contract",
            facts: &SUBMISSION_CONTRACT_FACTS[..SUBMISSION_CONTRACT_FACTS.len() - 1],
        })
        .expect("test submission contract semantics serialize");
        assert_ne!(digest, changed);
    }

    #[test]
    fn action_form_contract_semantic_digest_is_canonical_and_fact_bound() {
        let digest = action_form_contract_semantic_digest();
        assert!(crate::canonical::is_canonical_sha256_digest(
            digest.as_str()
        ));
        let changed = canonical_json_sha256(&SemanticDigestBasis {
            domain: "volicord.action-form-contract",
            facts: &ACTION_FORM_CONTRACT_FACTS[..ACTION_FORM_CONTRACT_FACTS.len() - 1],
        })
        .expect("test action-form semantics serialize");
        assert_ne!(digest, changed);
    }

    #[test]
    fn mcp_semantic_schema_digest_is_canonical_and_fact_bound() {
        let digest = mcp_semantic_schema_digest();
        assert!(crate::canonical::is_canonical_sha256_digest(
            digest.as_str()
        ));
        let changed = canonical_json_sha256(&SemanticDigestBasis {
            domain: "volicord.mcp-semantic-schema",
            facts: &MCP_SEMANTIC_SCHEMA_FACTS[..MCP_SEMANTIC_SCHEMA_FACTS.len() - 1],
        })
        .expect("test semantics serialize");
        assert_ne!(digest, changed);
    }
}
