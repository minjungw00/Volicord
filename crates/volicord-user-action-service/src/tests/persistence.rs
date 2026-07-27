use crate::persistence::{
    map_user_action_request_persistence, materialize_user_action_resolution_mutation,
    UserActionRequestPersistenceInput,
};
use volicord_store::core_pipeline::{
    CoreStorageMutation, UserActionMutation, UserActionResolutionRecord,
};
use volicord_types::ids::UserActionOptionId;
use volicord_types::schema::{
    PersistedUserActionRequest, RequiredNullable, UserActionResolutionBody,
};
use volicord_types::values::{
    ActorSource, JudgmentResolutionOutcome, MethodName, UserActionChannelKind, UserActionKind,
    UserActionOptionAction, UserActionStatus, UserActionVerificationBasis, UtcTimestamp,
};

#[test]
fn request_mapping_produces_matching_effective_record_and_store_input() {
    let constructed = super::canonical_choice();
    let request = PersistedUserActionRequest {
        body: constructed.body,
        required_for: constructed.required_for.clone(),
        expires_at: constructed.expires_at,
    };
    let (effective, mutation) =
        map_user_action_request_persistence(UserActionRequestPersistenceInput {
            project_id: "project-test".to_owned(),
            user_action_request_id: "action-test".to_owned(),
            task_id: "task-test".to_owned(),
            change_unit_id: Some("change-test".to_owned()),
            action_kind: UserActionKind::ProductDecision,
            request,
            basis: constructed.basis,
            required_for: constructed.required_for,
            requested_by_actor_source: ActorSource::AgentConnection(
                volicord_types::ids::AgentConnectionId::new("test"),
            ),
            source_method: MethodName::RequestUserAction,
            source_idempotency_key: "idem-test".to_owned(),
            requested_at: UtcTimestamp::parse("2026-07-27T00:00:00Z")
                .expect("test timestamp must parse"),
            expires_at: None,
            metadata: serde_json::json!({}),
        })
        .expect("canonical request must map");

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
    assert_eq!(
        serde_json::to_string(&effective.request.request).expect("request must serialize"),
        insert.request_json
    );
    assert_eq!(
        serde_json::to_string(&effective.request.basis).expect("basis must serialize"),
        insert.basis_json
    );
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
        resolution: UserActionResolutionBody::Choice {
            selected_option_id: UserActionOptionId::new("accept"),
            machine_action: UserActionOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            note: RequiredNullable::null(),
            accepted_risk_ids: Vec::new(),
        },
        resolved_by_actor_source: ActorSource::LocalUser,
        resolved_verification_basis: UserActionVerificationBasis::CliDirectUserChannel,
        resolved_assurance_level: "local_verified".to_owned(),
        resolved_at: UtcTimestamp::parse("2026-07-27T00:01:00Z")
            .expect("test timestamp must parse"),
    })
    .expect("canonical resolution must map");

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
