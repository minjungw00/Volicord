//! Canonical semantic identity for generated agent guidance.

use serde::Serialize;

use crate::canonical::canonical_json_sha256;
use crate::ids::RequestHash;

/// Closed semantic facts that every managed agent-guidance rendering preserves.
pub const MANAGED_GUIDANCE_FACTS: &[&str] = &[
    "follow_tagged_required_action",
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

#[derive(Serialize)]
struct ManagedGuidanceSemanticBasis<'a> {
    domain: &'static str,
    facts: &'a [&'a str],
}

/// Returns the canonical semantic digest bound into project integration identity.
pub fn managed_guidance_semantic_digest() -> RequestHash {
    canonical_json_sha256(&ManagedGuidanceSemanticBasis {
        domain: "volicord.managed-agent-guidance",
        facts: MANAGED_GUIDANCE_FACTS,
    })
    .expect("static managed guidance semantics always serialize")
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
        let changed = canonical_json_sha256(&ManagedGuidanceSemanticBasis {
            domain: "volicord.managed-agent-guidance",
            facts: &MANAGED_GUIDANCE_FACTS[..MANAGED_GUIDANCE_FACTS.len() - 1],
        })
        .expect("test semantics serialize");
        assert_ne!(digest, changed);
    }
}
