use crate::user_action::persistence::{
    map_user_action_request_persistence, materialize_user_action_resolution_mutation,
    UserActionRequestPersistenceInput,
};
use volicord_store::core_pipeline::{
    CoreStorageMutation, UserActionMutation, UserActionResolutionRecord,
};
use volicord_types::values::{
    UserActionChannelKind, UserActionKind, UserActionStatus, UserActionVerificationBasis,
};

#[test]
fn request_mapping_produces_matching_effective_record_and_store_input() {
    let (effective, mutation) =
        map_user_action_request_persistence(UserActionRequestPersistenceInput {
            project_id: "project-test".to_owned(),
            user_action_request_id: "action-test".to_owned(),
            task_id: "task-test".to_owned(),
            change_unit_id: Some("change-test".to_owned()),
            action_kind: UserActionKind::ProductDecision,
            request_json: "{\"request\":\"typed\"}".to_owned(),
            basis_json: "{\"basis\":\"typed\"}".to_owned(),
            required_for_json: "[\"close_complete\"]".to_owned(),
            requested_by_actor_source: "agent_connection:test".to_owned(),
            source_method: "volicord.request_user_action".to_owned(),
            source_idempotency_key: "idem-test".to_owned(),
            requested_at: "2026-07-27T00:00:00Z".to_owned(),
            expires_at: None,
            metadata_json: "{}".to_owned(),
        });

    let CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(insert)) = mutation
    else {
        panic!("request mapping must produce a typed request insert")
    };
    assert_eq!(effective.status, UserActionStatus::Pending);
    assert!(effective.resolution.is_none());
    assert_eq!(
        effective.request.user_action_request_id,
        insert.user_action_request_id
    );
    assert_eq!(effective.request.request_json, insert.request_json);
    assert_eq!(effective.request.basis_json, insert.basis_json);
    assert_eq!(
        effective.request.source_idempotency_key,
        insert.source_idempotency_key
    );
    assert_eq!(insert.source_idempotency_key, "idem-test");
}

#[test]
fn resolution_mapping_preserves_the_immutable_store_input() {
    let mutation = materialize_user_action_resolution_mutation(UserActionResolutionRecord {
        project_id: "project-test".to_owned(),
        user_action_resolution_id: "resolution-test".to_owned(),
        user_action_request_id: "action-test".to_owned(),
        action_kind: UserActionKind::ProductDecision,
        channel_kind: UserActionChannelKind::Cli,
        channel_submission_id: "submission-test".to_owned(),
        resolution_json: "{\"resolution_type\":\"choice\"}".to_owned(),
        resolved_by_actor_source: "local_user".to_owned(),
        resolved_verification_basis: UserActionVerificationBasis::CliDirectUserChannel,
        resolved_assurance_level: "local_verified".to_owned(),
        resolved_at: "2026-07-27T00:01:00Z".to_owned(),
    });

    let CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(insert)) = mutation
    else {
        panic!("resolution mapping must produce a typed resolution insert")
    };
    assert_eq!(insert.user_action_resolution_id, "resolution-test");
    assert_eq!(insert.user_action_request_id, "action-test");
    assert_eq!(insert.channel_submission_id, "submission-test");
    assert_eq!(
        insert.resolved_verification_basis,
        UserActionVerificationBasis::CliDirectUserChannel
    );
}
