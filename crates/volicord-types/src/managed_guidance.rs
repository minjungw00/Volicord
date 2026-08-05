//! Canonical semantic identity for generated agent guidance.

use serde::Serialize;

use crate::canonical::canonical_json_sha256;
use crate::ids::RequestHash;

/// Closed semantic facts that every managed agent-guidance rendering preserves.
pub const MANAGED_GUIDANCE_FACTS: &[&str] = &[
    "follow_tagged_required_action",
    "tagged_workflow_action_catalog_is_mutation_admission_authority",
    "call_only_currently_allowed_task_bound_method",
    "use_exact_method_and_semantic_variant_action_form",
    "copy_and_preserve_fixed_authority_arguments_exactly",
    "action_form_never_authorizes_different_method_or_semantic_variant",
    "use_type_owned_mcp_schema_semantics",
    "read_only_status_grants_no_mutation_authority",
    "do_not_speculate_different_shaping_or_implementation_method",
    "surface_pre_core_admission_rejection_exactly",
    "use_mcp_tool_schema_and_retry_contract",
    "ordinary_cli_help_is_not_mcp_request_schema",
    "binary_strings_are_not_tool_schema",
    "preserve_json_null_boolean_and_number_primitive_types",
    "do_not_infer_another_union_branch",
    "argument_error_does_not_imply_state_corruption",
    "product_repository_edit_requires_successful_authority_mutation",
    "failed_checkpoint_and_user_action_creation_surface_exactly",
    "use_store_derived_current_effective_shaping_authority_graph",
    "superseded_shaping_history_is_immutable_and_non_actionable",
    "preserve_current_checkpoint_and_user_action_authority",
    "checkpoint_replacement_forbidden_while_decision_live",
    "compatible_applied_decisions_require_explicit_checkpoint_carry_forward",
    "stale_shaping_authority_grants_no_permission",
    "stale_accepted_resolution_is_never_reused",
    "stale_authority_requires_exact_retirement_or_reauthorization",
    "reauthorization_creates_fresh_user_action_identity",
    "reauthorization_lineage_is_immutable",
    "resolution_does_not_apply_shaping_decision",
    "inspect_exact_shaping_resolution_outcome",
    "only_accepted_current_shaping_authority_is_applicable",
    "follow_shaping_decision_application_owner",
    "rejected_deferred_or_expired_decision_requires_shaping_recovery",
    "terminal_or_expired_user_action_is_never_resolved_again",
    "successor_user_action_required_when_revised_plan_still_needs_decision",
    "non_authorizing_decision_does_not_enable_product_repository_mutation",
    "scope_decision_not_invented_for_product_or_technical_only",
    "change_unit_creation_does_not_advance_phase",
    "explicit_advance_required_for_work_implementation",
    "implementation_authority_invalidating_update_rejected_before_mutation",
    "implementation_never_silently_returns_to_shaping",
    "advisor_results_finalize_through_finalize_advice",
    "advisor_change_units_are_non_write",
    "user_decisions_require_user_action_requests",
    "chat_reply_is_not_resolution",
    "advance_task_forbidden_while_user_action_pending",
    "prepare_write_forbidden_before_implementation",
    "rejection_must_not_be_presented_as_success",
    "all_rejection_and_recovery_facts_must_surface",
    "non_authorizing_decision_must_surface_no_authority",
    "presentation_must_surface_required_facts",
    "close_blockers_do_not_replace_workflow_progression",
    "close_readiness_only_during_close_review",
];

/// Closed semantics whose change invalidates state-bound workflow action contracts.
pub const WORKFLOW_ACTION_CONTRACT_FACTS: &[&str] = &[
    "core_owns_workflow_action_catalog",
    "catalog_contains_one_group_for_every_allowed_task_bound_method",
    "catalog_contains_every_current_method_owned_semantic_variant",
    "required_method_variants_are_alternative_actions",
    "catalog_contains_no_disallowed_task_bound_method",
    "mcp_projects_each_descriptor_bound_action_form",
    "form_ref_binds_project_task_method_variant_state_coordinates_and_schema",
    "task_bound_method_is_admitted_before_core",
    "allowed_task_bound_mutation_requires_exact_method_and_variant_form_ref",
    "action_form_never_authorizes_different_method_or_semantic_variant",
    "invalid_current_method_variant_rejects_before_core",
    "fixed_authority_arguments_must_be_present_and_deeply_equal",
    "authority_mirror_fields_are_not_public_mutation_inputs",
    "read_only_status_grants_no_mutation_authority",
    "invalid_arguments_load_only_independently_valid_authority_context",
    "retry_contract_supplies_only_authority_owned_coordinates",
    "pre_core_rejection_reports_no_commit_and_no_state_change",
    "authority_basis_mismatch_reports_typed_expected_and_received_values",
];

/// Closed semantics whose change invalidates MCP semantic-schema identity.
pub const MCP_SEMANTIC_SCHEMA_FACTS: &[&str] = &[
    "rust_types_own_mcp_field_semantics",
    "required_nullable_semantics_are_generic",
    "tagged_unions_declare_explicit_discriminators",
    "validation_is_local_to_the_selected_union_branch",
    "runtime_discovery_is_bounded",
    "runtime_discovery_preserves_load_bearing_semantic_annotations",
    "canonical_examples_validate_and_decode_through_one_descriptor",
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

/// Returns the canonical semantic digest for current workflow action contracts.
pub fn workflow_action_contract_semantic_digest() -> RequestHash {
    canonical_json_sha256(&SemanticDigestBasis {
        domain: "volicord.workflow-action-contract",
        facts: WORKFLOW_ACTION_CONTRACT_FACTS,
    })
    .expect("static workflow action contract semantics always serialize")
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
    fn action_contract_semantic_digest_is_canonical_and_fact_bound() {
        let digest = workflow_action_contract_semantic_digest();
        assert!(crate::canonical::is_canonical_sha256_digest(
            digest.as_str()
        ));
        let changed = canonical_json_sha256(&SemanticDigestBasis {
            domain: "volicord.workflow-action-contract",
            facts: &WORKFLOW_ACTION_CONTRACT_FACTS[..WORKFLOW_ACTION_CONTRACT_FACTS.len() - 1],
        })
        .expect("test semantics serialize");
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
