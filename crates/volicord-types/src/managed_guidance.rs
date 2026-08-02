//! Canonical semantic identity for generated agent guidance.

use serde::Serialize;

use crate::canonical::canonical_json_sha256;
use crate::ids::RequestHash;

/// Closed semantic facts that every managed agent-guidance rendering preserves.
pub const MANAGED_GUIDANCE_FACTS: &[&str] = &[
    "follow_tagged_required_action",
    "record_shaping_before_scope_implementation",
    "change_unit_creation_does_not_advance_phase",
    "user_decisions_require_user_action_requests",
    "chat_reply_is_not_resolution",
    "apply_decisions_through_current_resolution_refs",
    "advance_task_forbidden_while_user_action_pending",
    "prepare_write_forbidden_before_implementation",
    "rejection_must_not_be_presented_as_success",
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
