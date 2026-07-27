use crate::materialization::{
    materialize_user_action_resolution, UserActionResolutionMaterializationInput,
};
use volicord_store::core_pipeline::{CoreStorageMutation, UserActionMutation};
use volicord_types::ids::{
    ProjectId, UserActionOptionId, UserActionRequestId, UserActionResolutionId,
};
use volicord_types::schema::{RequiredNullable, UserActionResolutionBody};
use volicord_types::values::{
    ActorSource, JudgmentResolutionOutcome, UserActionChannelKind, UserActionKind,
    UserActionOptionAction, UserActionVerificationBasis, UtcTimestamp,
};

#[test]
fn resolution_materialization_serializes_only_at_the_store_boundary() {
    let resolution = UserActionResolutionBody::Choice {
        selected_option_id: UserActionOptionId::new("accept"),
        machine_action: UserActionOptionAction::Accept,
        resolution_outcome: JudgmentResolutionOutcome::Accepted,
        note: RequiredNullable::null(),
        accepted_risk_ids: Vec::new(),
    };
    let materialized =
        materialize_user_action_resolution(UserActionResolutionMaterializationInput {
            project_id: &ProjectId::new("project-test"),
            user_action_resolution_id: UserActionResolutionId::new("resolution-test"),
            user_action_request_id: &UserActionRequestId::new("action-test"),
            action_kind: UserActionKind::ProductDecision,
            channel_kind: UserActionChannelKind::Cli,
            channel_submission_id: "submission-test",
            resolution: resolution.clone(),
            actor_source: ActorSource::LocalUser,
            verification_basis: UserActionVerificationBasis::CliDirectUserChannel,
            assurance_level: "local_verified".to_owned(),
            resolved_at: &UtcTimestamp::parse("2026-07-27T00:01:00Z")
                .expect("test timestamp must parse"),
        })
        .expect("typed resolution must materialize");

    assert_eq!(materialized.record.resolution, resolution);
    assert_eq!(
        materialized.record.resolved_by_actor_source,
        ActorSource::LocalUser
    );
    let CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(insert)) =
        materialized.mutation
    else {
        panic!("resolution materialization must produce a typed Store mutation")
    };
    assert_eq!(insert.user_action_resolution_id, "resolution-test");
    let stored: UserActionResolutionBody =
        serde_json::from_str(&insert.resolution_json).expect("stored resolution must be canonical");
    assert_eq!(stored, materialized.record.resolution);
}
