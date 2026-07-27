use super::canonical_choice;
use crate::{
    error::{UserActionInvariantError, UserActionServiceError},
    projection::user_action_resolution_facts,
};
use volicord_types::{
    ids::{ProjectId, TaskId, UserActionRequestId, UserActionResolutionId},
    schema::{
        RequiredNullable, UserActionEvidenceObservation, UserActionRequest, UserActionResolution,
        UserActionResolutionBody,
    },
    values::{
        ActorSource, EvidenceRelevanceStatus, UserActionChannelKind, UserActionKind,
        UserActionStatus, UserActionVerificationBasis,
    },
};

#[test]
fn projection_reports_valid_typed_fact_mismatch_as_a_service_invariant() {
    let constructed = canonical_choice();
    let request_id = UserActionRequestId::new("action-test");
    let request = UserActionRequest {
        user_action_request_id: request_id.clone(),
        project_id: ProjectId::new("project-test"),
        task_id: TaskId::new("task-test"),
        change_unit_id: constructed.coordinate_change_unit_id.into(),
        action_kind: UserActionKind::ProductDecision,
        status: UserActionStatus::Resolved,
        body: constructed.body,
        basis: constructed.basis,
        required_for: constructed.required_for,
        user_action_resolution_ref: RequiredNullable::null(),
        expires_at: constructed.expires_at,
        created_at: constructed.created_at,
    };
    let resolution = UserActionResolution {
        user_action_resolution_id: UserActionResolutionId::new("resolution-test"),
        user_action_request_id: request_id,
        project_id: ProjectId::new("project-test"),
        task_id: TaskId::new("task-test"),
        action_kind: UserActionKind::EvidenceObservation,
        body: UserActionResolutionBody::EvidenceObservation {
            observation: UserActionEvidenceObservation {
                target: volicord_types::schema::EvidenceTarget::SupplementalClaim {
                    evidence_claim_id: volicord_types::ids::EvidenceClaimId::new("claim-test"),
                    statement: "A valid typed observation.".to_owned(),
                },
                relevance_status: EvidenceRelevanceStatus::Supported,
                output_artifact_refs: Vec::new(),
                summary: "Observed.".to_owned(),
            },
        },
        resolved_by_actor_source: ActorSource::LocalUser,
        resolved_verification_basis: UserActionVerificationBasis::CliDirectUserChannel,
        resolved_assurance_level: "local_verified".to_owned(),
        channel_kind: UserActionChannelKind::Cli,
        channel_submission_id: "submission-test".to_owned(),
        resolved_at: volicord_types::values::UtcTimestamp::parse("2026-07-27T00:01:00Z")
            .expect("test timestamp must parse"),
    };

    let error = user_action_resolution_facts(&request, &resolution)
        .expect_err("valid typed facts from different semantic families must not project");

    assert!(matches!(
        error,
        UserActionServiceError::Invariant(UserActionInvariantError::ActionFactsMismatch)
    ));
}
