use std::error::Error;

use rusqlite::params;
use serde_json::{json, Value};
use volicord_types::ids::{IdempotencyKey, ProjectId, RequestHash};
use volicord_types::values::{
    ActorSource, JudgmentResolutionOutcome, MethodName, UserActionBasisStatus,
    UserActionChannelKind, UserActionKind, UserActionOptionAction, UserActionRequiredFor,
    UserActionStatus, UserActionVerificationBasis, UtcTimestamp,
};

use super::{
    UserActionBasisStatusMark, UserActionMutation, UserActionRequestInsert,
    UserActionResolutionInsert,
};
use crate::core_pipeline::test_support::{
    local_user_replay_context as user_replay_context, pending_event_for_task, replay_context,
    response_json, task_insert, StoreFixture as StoreHarness, ACTOR_SOURCE, CONNECTION_ID,
    PROJECT_ID,
};
use crate::core_pipeline::{
    commit_input, CoreStorageMutation, MutationCommitOutcome, TaskMutation, VerifiedReplayContext,
};
use crate::sqlite::open_project_state_database_for_test;
use crate::StoreError;

#[test]
fn user_action_request_and_basis_store_apis_round_trip() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_basis_round_trip";
    let request_id = "action_basis_round_trip";
    let now = UtcTimestamp::parse("2026-01-01T00:10:00Z")?;

    let first_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RequestUserAction,
        Some(&IdempotencyKey::new("idem_store_basis_initial")),
        &RequestHash::new("sha256:basis-initial"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("basis_initial", task_id)],
    );
    let first = store.commit_with(
        first_input,
        |mutation, facts| {
            for storage_mutation in [
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(request_id, task_id, None),
                )),
            ] {
                storage_mutation.apply(mutation, facts).map(|_| ())?;
            }
            Ok(())
        },
        response_json,
    )?;
    assert!(matches!(first, MutationCommitOutcome::Committed { .. }));

    let current = store
        .user_action_record(request_id, &now)?
        .expect("user-action request should be readable");
    assert_eq!(current.status, UserActionStatus::Pending);
    assert_eq!(current.request.user_action_request_id, request_id);
    assert_eq!(current.request.task_id, task_id);
    assert_eq!(current.request.action_kind, UserActionKind::ProductDecision);
    assert_eq!(current.request.basis_status, UserActionBasisStatus::Current);
    assert_eq!(
        current.request.required_for,
        vec![UserActionRequiredFor::Informational]
    );
    assert_eq!(
        current
            .request
            .requested_by_actor_source
            .to_canonical_string(),
        ACTOR_SOURCE
    );
    assert!(current.resolution.is_none());
    let basis = &current.request.basis;
    assert_eq!(basis.compatibility_status(), UserActionBasisStatus::Current);
    assert_eq!(basis.coordinates().task_id.as_str(), task_id);

    let stale_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_basis_stale")),
        &RequestHash::new("sha256:basis-stale"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(1),
        vec![pending_event_for_task("basis_stale", task_id)],
    );
    let stale = store.commit_with(
        stale_input,
        |mutation, facts| {
            CoreStorageMutation::UserAction(UserActionMutation::MarkBasesStatus(
                UserActionBasisStatusMark {
                    user_action_request_ids: vec![request_id.to_owned()],
                    basis_status: UserActionBasisStatus::Stale,
                },
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(stale, MutationCommitOutcome::Committed { .. }));
    let stale = store
        .user_action_record(request_id, &now)?
        .expect("stale request should remain readable");
    assert_eq!(stale.status, UserActionStatus::Stale);
    assert_eq!(stale.request.basis_status, UserActionBasisStatus::Stale);
    let stale_basis = &stale.request.basis;
    assert_eq!(
        stale_basis.compatibility_status(),
        UserActionBasisStatus::Stale
    );

    let superseded_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::UpdateScope,
        Some(&IdempotencyKey::new("idem_store_basis_superseded")),
        &RequestHash::new("sha256:basis-superseded"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(2),
        vec![pending_event_for_task("basis_superseded", task_id)],
    );
    let superseded = store.commit_with(
        superseded_input,
        |mutation, facts| {
            CoreStorageMutation::UserAction(UserActionMutation::MarkBasesStatus(
                UserActionBasisStatusMark {
                    user_action_request_ids: vec![request_id.to_owned()],
                    basis_status: UserActionBasisStatus::Superseded,
                },
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(
        superseded,
        MutationCommitOutcome::Committed { .. }
    ));
    assert_eq!(
        store
            .user_action_record(request_id, &now)?
            .expect("superseded request should remain readable")
            .status,
        UserActionStatus::Superseded
    );
    Ok(())
}

#[test]
fn user_action_request_store_rejects_empty_duplicate_and_mismatched_owner_facts(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_invalid_user_action_owner_facts";

    for (marker, mut action) in [
        (
            "empty_required_for",
            user_action_request_insert("action_empty_required_for", task_id, None),
        ),
        (
            "duplicate_required_for",
            user_action_request_insert("action_duplicate_required_for", task_id, None),
        ),
        (
            "mismatched_sensitive_scope",
            user_action_request_insert("action_mismatched_sensitive_scope", task_id, None),
        ),
        (
            "incompatible_required_for",
            user_action_request_insert("action_incompatible_required_for", task_id, None),
        ),
    ] {
        match marker {
            "empty_required_for" => {
                let mut request = serde_json::from_str::<Value>(&action.request_json)?;
                request["required_for"] = json!([]);
                action.request_json = request.to_string();
                action.required_for_json = "[]".to_owned();
            }
            "duplicate_required_for" => {
                let mut request = serde_json::from_str::<Value>(&action.request_json)?;
                request["required_for"] = json!(["informational", "informational"]);
                action.request_json = request.to_string();
                action.required_for_json = r#"["informational","informational"]"#.to_owned();
            }
            "mismatched_sensitive_scope" => {
                let mut basis = serde_json::from_str::<Value>(&action.basis_json)?;
                basis["sensitive_action_scope"] = json!({
                    "action_kind": "write_files",
                    "description": "Bounded write.",
                    "intended_paths": ["src/lib.rs"],
                    "sensitive_categories": ["product_file_write"],
                    "command_or_tool_summary": null,
                    "network_or_host_summary": null,
                    "secret_or_credential_summary": null,
                    "capability_claim": "Local file write only.",
                    "expires_at": null
                });
                action.basis_json = basis.to_string();
            }
            "incompatible_required_for" => {
                let mut request = serde_json::from_str::<Value>(&action.request_json)?;
                request["required_for"] = json!(["close_cancel"]);
                action.request_json = request.to_string();
                action.required_for_json = r#"["close_cancel"]"#.to_owned();
            }
            _ => unreachable!("test table contains only declared invalid cases"),
        }
        let error = store
            .commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RequestUserAction,
                    Some(&IdempotencyKey::new(format!("idem_store_{marker}"))),
                    &RequestHash::new(format!("sha256:{marker}")),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(0),
                    vec![pending_event_for_task(marker, task_id)],
                ),
                |mutation, facts| {
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                        .apply(mutation, facts)
                        .map(|_| ())?;
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                        .apply(mutation, facts)
                        .map(|_| ())
                },
                response_json,
            )
            .expect_err("invalid user-action owner facts must fail closed");
        assert!(matches!(&error, StoreError::InvalidInput { .. }));
        if marker == "incompatible_required_for" {
            assert!(matches!(
                error,
                StoreError::InvalidInput { detail }
                    if detail == "user_action_requests.request_json required_for contains an operation incompatible with its action kind"
            ));
        }
        assert_eq!(store.effect_counts()?.tasks, 0);
    }
    Ok(())
}

#[test]
fn user_action_request_timestamp_order_is_strict_at_insert_boundaries() -> Result<(), Box<dyn Error>>
{
    for (suffix, expires_at, should_commit) in [
        ("before", "2025-12-31T23:59:59.999Z", false),
        ("equal", "2026-01-01T00:00:00Z", false),
        ("after", "2026-01-01T00:00:00.001Z", true),
    ] {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = format!("task_request_timestamp_{suffix}");
        let request_id = format!("action_request_timestamp_{suffix}");
        let mut action = user_action_request_insert(&request_id, &task_id, None);
        set_user_action_request_expiry(&mut action, expires_at);
        let outcome = store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new(format!(
                    "idem_request_timestamp_{suffix}"
                ))),
                &RequestHash::new(format!("sha256:request-timestamp-{suffix}")),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task(
                    &format!("{suffix}_request"),
                    &task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(&task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                    .apply(mutation, facts)
                    .map(|_| ())
            },
            response_json,
        );

        if should_commit {
            assert!(matches!(outcome?, MutationCommitOutcome::Committed { .. }));
            assert_eq!(
                    store
                        .user_action_record(
                            &request_id,
                            &UtcTimestamp::parse("2026-01-01T00:00:00Z")?,
                        )?
                        .expect("strictly later expiry should remain readable")
                        .status,
                    UserActionStatus::Pending
                );
        } else {
            let error = outcome.expect_err("non-later expiry must reject atomically");
            assert!(matches!(
                error,
                StoreError::InvalidInput { detail }
                    if detail == "user_action_requests.expires_at must be later than user_action_requests.requested_at"
            ));
            assert_eq!(store.effect_counts()?.tasks, 0);
        }
    }
    Ok(())
}

#[test]
fn evidence_observation_request_insert_rejects_extended_ttl_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_evidence_action_extended_ttl";
    let request_id = "action_evidence_action_extended_ttl";
    let mut action = evidence_user_action_request_insert(request_id, task_id, 1);
    set_user_action_request_expiry(&mut action, "2026-01-01T00:16:00Z");
    let before_state = store.project_state()?;
    let before_effects = store.effect_counts()?;

    let error = store
        .commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_evidence_action_extended_ttl")),
                &RequestHash::new("sha256:evidence-action-extended-ttl"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task(
                    "evidence_action_extended_ttl",
                    task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                    .apply(mutation, facts)
                    .map(|_| ())
            },
            response_json,
        )
        .expect_err("a 16-minute evidence-observation request TTL must reject atomically");

    assert!(matches!(
        error,
        StoreError::InvalidInput { detail }
            if detail == "evidence-observation user_action_requests.expires_at must be exactly 15 minutes after user_action_requests.requested_at"
    ));
    assert_eq!(store.project_state()?, before_state);
    assert_eq!(store.effect_counts()?, before_effects);
    Ok(())
}

#[test]
fn user_action_store_derives_expiry_resolution_and_stale_status() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_user_action_status";

    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_action_expiring")),
            &RequestHash::new("sha256:action-expiring"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("action_expiring", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                user_action_request_insert(
                    "action_expiring",
                    task_id,
                    Some("2026-01-01T00:15:00Z"),
                ),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    let before_expiry = UtcTimestamp::parse("2026-01-01T00:14:59Z")?;
    let at_expiry = UtcTimestamp::parse("2026-01-01T00:15:00Z")?;
    assert_eq!(
        store
            .user_action_record("action_expiring", &before_expiry)?
            .expect("expiring action should be readable")
            .status,
        UserActionStatus::Pending
    );
    assert_eq!(
        store
            .user_action_record("action_expiring", &at_expiry)?
            .expect("expired action should remain readable")
            .status,
        UserActionStatus::Expired
    );

    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_action_current")),
            &RequestHash::new("sha256:action-current"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(1),
            vec![pending_event_for_task("action_current", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                user_action_request_insert("action_current", task_id, None),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_action_resolve")),
            &RequestHash::new("sha256:action-resolve"),
            Some(VerifiedReplayContext {
                actor_source: "local_user".to_owned(),
                operation_category: "user_only".to_owned(),
                verification_basis: Some("store_test_user_channel".to_owned()),
                git_workspace_context_json: None,
            }),
            Some(2),
            vec![pending_event_for_task("action_resolve", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                user_action_resolution_insert("resolution_current", "action_current"),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    assert_eq!(
        store
            .user_action_record("action_current", &at_expiry)?
            .expect("resolved action should be readable")
            .status,
        UserActionStatus::Resolved
    );

    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_action_stale")),
            &RequestHash::new("sha256:action-stale"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(3),
            vec![pending_event_for_task("action_stale", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::UserAction(UserActionMutation::MarkBasesStatus(
                UserActionBasisStatusMark {
                    user_action_request_ids: vec!["action_current".to_owned()],
                    basis_status: UserActionBasisStatus::Stale,
                },
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    let stale = store
        .user_action_record("action_current", &at_expiry)?
        .expect("stale action should be readable");
    assert_eq!(stale.status, UserActionStatus::Stale);
    assert_eq!(
        serde_json::to_value(&stale.request.basis)?["coordinates"]["compatibility_status"],
        "stale"
    );
    Ok(())
}

#[test]
fn user_action_resolution_round_trips_choice_and_channel_provenance() -> Result<(), Box<dyn Error>>
{
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_deferred_action";
    let request_id = "action_deferred_pair";
    let resolution_id = "resolution_deferred_pair";
    let mut deferred_request = user_action_request_insert(request_id, task_id, None);
    let mut deferred_request_json = serde_json::from_str::<Value>(&deferred_request.request_json)?;
    deferred_request_json["body"]["options"]
        .as_array_mut()
        .expect("choice options should be an array")
        .push(json!({
            "option_id": "defer",
            "label": "Defer",
            "description": "Defer this bounded decision.",
            "consequence": "The request remains resolved as deferred.",
            "machine_action": "defer",
            "resolution_outcome": "deferred",
            "is_default": false
        }));
    deferred_request.request_json = deferred_request_json.to_string();

    let insert_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RequestUserAction,
        Some(&IdempotencyKey::new("idem_store_defer_insert")),
        &RequestHash::new("sha256:defer-insert"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("defer_insert", task_id)],
    );
    let inserted = store.commit_with(
        insert_input,
        |mutation, facts| {
            for storage_mutation in [
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    deferred_request,
                )),
            ] {
                storage_mutation.apply(mutation, facts).map(|_| ())?;
            }
            Ok(())
        },
        response_json,
    )?;
    assert!(matches!(inserted, MutationCommitOutcome::Committed { .. }));

    let mut resolution = user_action_resolution_insert(resolution_id, request_id);
    resolution.channel_submission_id = "submission_deferred_pair".to_owned();
    resolution.resolution_json = choice_resolution_json(
        "defer",
        UserActionOptionAction::Defer,
        JudgmentResolutionOutcome::Deferred,
    );
    resolution.resolved_assurance_level = "verified_local_user_channel".to_owned();
    let resolve_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::ResolveUserAction,
        Some(&IdempotencyKey::new("idem_store_defer_resolve")),
        &RequestHash::new("sha256:defer-resolve"),
        Some(user_replay_context()),
        Some(1),
        vec![pending_event_for_task("defer_resolve", task_id)],
    );
    let resolved = store.commit_with(
        resolve_input,
        |mutation, facts| {
            CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(resolution))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    assert!(matches!(resolved, MutationCommitOutcome::Committed { .. }));

    let record = store
        .user_action_resolution_record(resolution_id)?
        .expect("resolved user action should be readable");
    assert_eq!(record.user_action_request_id, request_id);
    assert_eq!(record.channel_kind, UserActionChannelKind::Cli);
    assert_eq!(record.channel_submission_id, "submission_deferred_pair");
    assert_eq!(record.resolved_by_actor_source, ActorSource::LocalUser);
    assert_eq!(
        serde_json::to_value(&record.resolution)?["machine_action"],
        "defer"
    );
    assert_eq!(
        store
            .user_action_resolution_for_channel_submission(
                UserActionChannelKind::Cli,
                "submission_deferred_pair",
            )?
            .expect("channel submission lookup should return the immutable resolution"),
        record
    );
    assert_eq!(
        store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:11:00Z")?,)?
            .expect("resolved request should remain readable")
            .status,
        UserActionStatus::Resolved
    );
    let before_tamper = store.effect_counts()?;
    store
        .conn
        .execute_batch("PRAGMA ignore_check_constraints = ON")?;
    store.conn.execute(
        "UPDATE user_action_resolutions
                SET channel_submission_id = ?3
              WHERE project_id = ?1
                AND user_action_resolution_id = ?2",
        params![PROJECT_ID, resolution_id, "x".repeat(257)],
    )?;
    store
        .conn
        .execute_batch("PRAGMA ignore_check_constraints = OFF")?;
    assert!(matches!(
        store.user_action_resolution_record(resolution_id),
        Err(StoreError::CorruptOwnerStateValue { .. })
    ));
    assert_eq!(store.effect_counts()?, before_tamper);
    Ok(())
}

#[test]
fn user_action_resolution_timestamp_order_enforces_half_open_boundaries(
) -> Result<(), Box<dyn Error>> {
    for (suffix, resolved_at, expected_error) in [
            (
                "before_request",
                "2025-12-31T23:59:59.999Z",
                Some(
                    "user_action_resolutions.resolved_at must be at or after user_action_requests.requested_at",
                ),
            ),
            ("at_request", "2026-01-01T00:00:00Z", None),
            ("before_expiry", "2026-01-01T00:00:09.999Z", None),
            (
                "at_expiry",
                "2026-01-01T00:00:10Z",
                Some(
                    "user_action_resolutions.resolved_at must be before user_action_requests.expires_at",
                ),
            ),
            (
                "after_expiry",
                "2026-01-01T00:00:10.001Z",
                Some(
                    "user_action_resolutions.resolved_at must be before user_action_requests.expires_at",
                ),
            ),
        ] {
            let harness = StoreHarness::new()?;
            let mut store = harness.store()?;
            let task_id = format!("task_resolution_timestamp_{suffix}");
            let request_id = format!("action_resolution_timestamp_{suffix}");
            let resolution_id = format!("resolution_timestamp_{suffix}");
            let mut action = user_action_request_insert(&request_id, &task_id, None);
            set_user_action_request_expiry(&mut action, "2026-01-01T00:00:10Z");
            store.commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RequestUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_resolution_timestamp_request_{suffix}"
                    ))),
                    &RequestHash::new(format!(
                        "sha256:resolution-timestamp-request-{suffix}"
                    )),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(0),
                    vec![pending_event_for_task(
                        &format!("{suffix}_request"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::Task(TaskMutation::insert(task_insert(&task_id)))
                        .apply(mutation, facts).map(|_| ())?;
                    CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                        .apply(mutation, facts).map(|_| ())
                },
                response_json,
            )?;

            let mut resolution = user_action_resolution_insert(&resolution_id, &request_id);
            resolution.resolved_at = resolved_at.to_owned();
            let outcome = store.commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::ResolveUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_resolution_timestamp_resolve_{suffix}"
                    ))),
                    &RequestHash::new(format!(
                        "sha256:resolution-timestamp-resolve-{suffix}"
                    )),
                    Some(user_replay_context()),
                    Some(1),
                    vec![pending_event_for_task(
                        &format!("{suffix}_resolve"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(resolution))
                        .apply(mutation, facts).map(|_| ())
                },
                response_json,
            );

            if let Some(expected_error) = expected_error {
                let error = outcome.expect_err("out-of-window resolution must reject atomically");
                assert!(matches!(
                    error,
                    StoreError::InvalidInput { detail } if detail == expected_error
                ));
                assert_eq!(store.effect_counts()?.user_action_resolutions, 0);
                assert_eq!(store.project_state()?.state_version, 1);
            } else {
                assert!(matches!(outcome?, MutationCommitOutcome::Committed { .. }));
                assert_eq!(
                    store
                        .user_action_resolution_record(&resolution_id)?
                        .expect("in-window resolution should remain readable")
                        .resolved_at,
                    UtcTimestamp::parse(resolved_at)?
                );
            }
        }
    Ok(())
}

#[test]
fn evidence_observation_resolution_preserves_exact_candidate_after_projection_advances(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_observation_resolution_reread";
    let request_id = "action_observation_resolution_reread";
    let resolution_id = "resolution_observation_reread";

    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_observation_request")),
            &RequestHash::new("sha256:observation-request"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("observation_request", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                evidence_user_action_request_insert(request_id, task_id, 3),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    let before_mismatch = store.effect_counts()?;
    let mismatch = store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_observation_resolution")),
            &RequestHash::new("sha256:observation-resolution"),
            Some(user_replay_context()),
            Some(1),
            vec![pending_event_for_task("observation_resolution", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                evidence_user_action_resolution_insert(resolution_id, request_id, task_id, 4),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    );
    assert!(matches!(mismatch, Err(StoreError::InvalidInput { .. })));
    assert_eq!(store.effect_counts()?, before_mismatch);
    assert!(store
        .user_action_resolution_record(resolution_id)?
        .is_none());

    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_observation_resolution")),
            &RequestHash::new("sha256:observation-resolution"),
            Some(user_replay_context()),
            Some(1),
            vec![pending_event_for_task("observation_resolution", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                evidence_user_action_resolution_insert(resolution_id, request_id, task_id, 3),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    let resolved = store
        .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
        .expect("resolved evidence-observation action should remain readable");
    assert_eq!(resolved.status, UserActionStatus::Resolved);
    let resolution = store
        .user_action_resolution_record(resolution_id)?
        .expect("the immutable resolution should be readable by id");
    assert_eq!(
        serde_json::to_value(&resolution.resolution)?["observation"]["output_artifact_refs"][0]
            ["created_by_run_ref"]["produced_at_state_version"],
        3
    );

    let mut tampered: Value = serde_json::to_value(&resolution.resolution)?;
    tampered["observation"]["output_artifact_refs"][0]["sha256"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    store.conn.execute(
        "UPDATE user_action_resolutions
                SET resolution_json = ?2
              WHERE project_id = ?1
                AND user_action_resolution_id = ?3",
        params![PROJECT_ID, tampered.to_string(), resolution_id],
    )?;
    assert!(matches!(
        store.user_action_resolution_record(resolution_id),
        Err(StoreError::CorruptOwnerStateValue { .. })
    ));
    Ok(())
}

#[test]
fn user_action_resolution_is_one_to_one_and_channel_submission_is_unique(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_resolution_uniqueness";
    let first_request_id = "action_resolution_unique_first";
    let second_request_id = "action_resolution_unique_second";

    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_resolution_unique_insert")),
            &RequestHash::new("sha256:resolution-unique-insert"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("resolution_unique_insert", task_id)],
        ),
        |mutation, facts| {
            for storage_mutation in [
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(first_request_id, task_id, None),
                )),
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(second_request_id, task_id, None),
                )),
            ] {
                storage_mutation.apply(mutation, facts).map(|_| ())?;
            }
            Ok(())
        },
        response_json,
    )?;

    let mut first_resolution =
        user_action_resolution_insert("resolution_unique_first", first_request_id);
    first_resolution.channel_submission_id = "submission_unique".to_owned();
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_resolution_unique_first")),
            &RequestHash::new("sha256:resolution-unique-first"),
            Some(user_replay_context()),
            Some(1),
            vec![pending_event_for_task("resolution_unique_first", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(first_resolution))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    let before_conflicts = store.effect_counts()?;

    let second_for_same_request = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::ResolveUserAction,
        Some(&IdempotencyKey::new("idem_store_resolution_same_request")),
        &RequestHash::new("sha256:resolution-same-request"),
        Some(user_replay_context()),
        Some(2),
        vec![pending_event_for_task("resolution_same_request", task_id)],
    );
    let error = store
        .commit_with(
            second_for_same_request,
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                    user_action_resolution_insert(
                        "resolution_unique_duplicate_request",
                        first_request_id,
                    ),
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )
        .expect_err("one request must not accept a second immutable resolution");
    assert!(matches!(error, StoreError::Sqlite(_)));
    assert_eq!(store.effect_counts()?, before_conflicts);

    let mut reused_submission =
        user_action_resolution_insert("resolution_unique_submission", second_request_id);
    reused_submission.channel_submission_id = "submission_unique".to_owned();
    let error = store
        .commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new(
                    "idem_store_resolution_same_submission",
                )),
                &RequestHash::new("sha256:resolution-same-submission"),
                Some(user_replay_context()),
                Some(2),
                vec![pending_event_for_task(
                    "resolution_same_submission",
                    task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                    reused_submission,
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )
        .expect_err("one channel submission must not resolve two requests");
    assert!(matches!(error, StoreError::Sqlite(_)));
    assert_eq!(store.effect_counts()?, before_conflicts);
    assert_eq!(
        store
            .user_action_resolution_for_channel_submission(
                UserActionChannelKind::Cli,
                "submission_unique",
            )?
            .expect("the first resolution must remain canonical")
            .user_action_request_id,
        first_request_id
    );
    Ok(())
}

#[test]
fn user_action_resolution_rejects_request_action_kind_mismatch() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_resolution_kind_mismatch";
    let request_id = "action_resolution_kind_mismatch";

    let insert_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RequestUserAction,
        Some(&IdempotencyKey::new("idem_store_missing_action_insert")),
        &RequestHash::new("sha256:missing-action-insert"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("missing_action_insert", task_id)],
    );
    let inserted = store.commit_with(
        insert_input,
        |mutation, facts| {
            for storage_mutation in [
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(request_id, task_id, None),
                )),
            ] {
                storage_mutation.apply(mutation, facts).map(|_| ())?;
            }
            Ok(())
        },
        response_json,
    )?;
    assert!(matches!(inserted, MutationCommitOutcome::Committed { .. }));
    let before = store.effect_counts()?;

    let resolve_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::ResolveUserAction,
        Some(&IdempotencyKey::new("idem_store_missing_action_resolve")),
        &RequestHash::new("sha256:missing-action-resolve"),
        Some(user_replay_context()),
        Some(1),
        vec![pending_event_for_task("missing_action_resolve", task_id)],
    );
    let mut resolution = user_action_resolution_insert("resolution_kind_mismatch", request_id);
    resolution.action_kind = UserActionKind::TechnicalDecision;

    let error = store
        .commit_with(
            resolve_input,
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(resolution))
                    .apply(mutation, facts)
                    .map(|_| ())
            },
            response_json,
        )
        .expect_err("resolution action kind must match its request");
    assert!(matches!(error, StoreError::InvalidInput { .. }));
    assert_eq!(store.effect_counts()?, before);
    let record = store
        .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
        .expect("pending user action should remain readable");
    assert_eq!(record.status, UserActionStatus::Pending);
    assert!(record.resolution.is_none());
    Ok(())
}

#[test]
fn user_action_resolution_read_fails_closed_on_tampered_choice_authority(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_tampered_choice_authority";
    let request_id = "action_tampered_choice_authority";
    let resolution_id = "resolution_tampered_choice_authority";
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_tampered_choice_insert")),
            &RequestHash::new("sha256:tampered-choice-insert"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("tampered_choice_insert", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                user_action_request_insert(request_id, task_id, None),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_tampered_choice_resolve")),
            &RequestHash::new("sha256:tampered-choice-resolve"),
            Some(user_replay_context()),
            Some(1),
            vec![pending_event_for_task("tampered_choice_resolve", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                user_action_resolution_insert(resolution_id, request_id),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    for tampered_resolution in [
        choice_resolution_json(
            "not_a_request_option",
            UserActionOptionAction::Accept,
            JudgmentResolutionOutcome::Accepted,
        ),
        choice_resolution_json(
            "accept",
            UserActionOptionAction::Reject,
            JudgmentResolutionOutcome::Rejected,
        ),
    ] {
        store.conn.execute(
            "UPDATE user_action_resolutions
                    SET resolution_json = ?2
                  WHERE project_id = ?1
                    AND user_action_resolution_id = ?3",
            params![PROJECT_ID, tampered_resolution, resolution_id],
        )?;
        assert!(matches!(
            store.user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?),
            Err(StoreError::CorruptOwnerStateValue { .. })
        ));
        assert!(matches!(
            store.user_action_resolution_record(resolution_id),
            Err(StoreError::CorruptOwnerStateValue { .. })
        ));
    }
    Ok(())
}

#[test]
fn user_action_resolution_requires_local_user_and_assurance() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_resolution_provenance";
    let request_id = "action_resolution_provenance";

    let insert_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RequestUserAction,
        Some(&IdempotencyKey::new("idem_store_blocked_resolution_insert")),
        &RequestHash::new("sha256:blocked-resolution-insert"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("blocked_resolution_insert", task_id)],
    );
    let inserted = store.commit_with(
        insert_input,
        |mutation, facts| {
            for storage_mutation in [
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(request_id, task_id, None),
                )),
            ] {
                storage_mutation.apply(mutation, facts).map(|_| ())?;
            }
            Ok(())
        },
        response_json,
    )?;
    assert!(matches!(inserted, MutationCommitOutcome::Committed { .. }));
    let before = store.effect_counts()?;

    let mut invalid_resolutions = Vec::new();
    let mut wrong_actor = user_action_resolution_insert("resolution_wrong_actor", request_id);
    wrong_actor.resolved_by_actor_source = ACTOR_SOURCE.to_owned();
    invalid_resolutions.push(("wrong_actor", wrong_actor));
    let mut missing_assurance =
        user_action_resolution_insert("resolution_missing_assurance", request_id);
    missing_assurance.resolved_assurance_level.clear();
    invalid_resolutions.push(("missing_assurance", missing_assurance));

    for (marker, resolution) in invalid_resolutions {
        let error = store
            .commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::ResolveUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_store_resolution_{marker}"
                    ))),
                    &RequestHash::new(format!("sha256:resolution-{marker}")),
                    Some(user_replay_context()),
                    Some(1),
                    vec![pending_event_for_task(marker, task_id)],
                ),
                |mutation, facts| {
                    CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                        resolution,
                    ))
                    .apply(mutation, facts)
                    .map(|_| ())
                },
                response_json,
            )
            .expect_err("invalid user actor or provenance must reject");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert_eq!(store.effect_counts()?, before);
    }
    let record = store
        .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
        .expect("pending request should remain readable");
    assert_eq!(record.status, UserActionStatus::Pending);
    assert!(record.resolution.is_none());
    Ok(())
}

#[test]
fn user_action_resolution_rejects_unknown_fields_and_invalid_outcomes() -> Result<(), Box<dyn Error>>
{
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_invalid_resolution_json";
    let request_id = "action_invalid_resolution_json";

    let insert_input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RequestUserAction,
        Some(&IdempotencyKey::new("idem_store_unknown_rationale_insert")),
        &RequestHash::new("sha256:unknown-rationale-insert"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("unknown_rationale_insert", task_id)],
    );
    let inserted = store.commit_with(
        insert_input,
        |mutation, facts| {
            for storage_mutation in [
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(request_id, task_id, None),
                )),
            ] {
                storage_mutation.apply(mutation, facts).map(|_| ())?;
            }
            Ok(())
        },
        response_json,
    )?;
    assert!(matches!(inserted, MutationCommitOutcome::Committed { .. }));
    let before = store.effect_counts()?;

    let mut unknown_field = user_action_resolution_insert("resolution_unknown_field", request_id);
    let mut unknown_value: Value = serde_json::from_str(&unknown_field.resolution_json)?;
    unknown_value["unknown_resolution_field"] = json!(true);
    unknown_field.resolution_json = unknown_value.to_string();
    let mut invalid_outcome =
        user_action_resolution_insert("resolution_invalid_outcome", request_id);
    let mut invalid_outcome_value: Value = serde_json::from_str(&invalid_outcome.resolution_json)?;
    invalid_outcome_value["resolution_outcome"] = json!("blocked");
    invalid_outcome.resolution_json = invalid_outcome_value.to_string();

    for (marker, resolution) in [
        ("unknown_field", unknown_field),
        ("invalid_outcome", invalid_outcome),
    ] {
        let error = store
            .commit_with(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::ResolveUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_store_resolution_{marker}"
                    ))),
                    &RequestHash::new(format!("sha256:resolution-{marker}")),
                    Some(user_replay_context()),
                    Some(1),
                    vec![pending_event_for_task(marker, task_id)],
                ),
                |mutation, facts| {
                    CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                        resolution,
                    ))
                    .apply(mutation, facts)
                    .map(|_| ())
                },
                response_json,
            )
            .expect_err("unsupported closed resolution shapes must reject");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert_eq!(store.effect_counts()?, before);
    }
    let record = store
        .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
        .expect("pending request should remain readable");
    assert_eq!(record.status, UserActionStatus::Pending);
    assert!(record.resolution.is_none());
    Ok(())
}

#[test]
fn malformed_stored_user_action_basis_json_is_store_data_error() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_malformed_basis";
    let request_id = "action_malformed_basis";

    let input = commit_input(
        &ProjectId::new(PROJECT_ID),
        MethodName::RequestUserAction,
        Some(&IdempotencyKey::new("idem_store_basis_malformed")),
        &RequestHash::new("sha256:basis-malformed"),
        Some(replay_context(CONNECTION_ID, "agent_workflow")),
        Some(0),
        vec![pending_event_for_task("basis_malformed", task_id)],
    );
    store.commit_with(
        input,
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                user_action_request_insert(request_id, task_id, None),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    let conn = open_project_state_database_for_test(harness.state_database_path())?;
    conn.execute(
        "UPDATE user_action_requests
                SET basis_json = 'not-json'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
        params![PROJECT_ID, request_id],
    )?;
    drop(conn);

    let store = harness.store()?;
    let error = store
        .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)
        .expect_err("malformed persisted basis JSON should be corruption");
    assert!(matches!(
        error,
        StoreError::CorruptOwnerStateValue {
            table: "user_action_requests",
            logical_column: "basis_json",
            ..
        }
    ));
    Ok(())
}

#[test]
fn stored_user_action_request_errors_preserve_request_and_required_for_columns(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_request_owner_columns";
    let malformed_request_id = "action_malformed_request_column";
    let mismatched_required_for_id = "action_mismatched_required_for_column";

    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_request_owner_columns")),
            &RequestHash::new("sha256:request-owner-columns"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("request_owner_columns", task_id)],
        ),
        |mutation, facts| {
            for storage_mutation in [
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id))),
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(malformed_request_id, task_id, None),
                )),
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                    user_action_request_insert(mismatched_required_for_id, task_id, None),
                )),
            ] {
                storage_mutation.apply(mutation, facts).map(|_| ())?;
            }
            Ok(())
        },
        response_json,
    )?;
    store.conn.execute(
        "UPDATE user_action_requests
                SET request_json = 'not-json'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
        params![PROJECT_ID, malformed_request_id],
    )?;
    store.conn.execute(
        "UPDATE user_action_requests
                SET required_for_json = '[\"close_complete\"]'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
        params![PROJECT_ID, mismatched_required_for_id],
    )?;

    for (request_id, expected_column) in [
        (malformed_request_id, "request_json"),
        (mismatched_required_for_id, "required_for_json"),
    ] {
        let error = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)
            .expect_err("invalid owner JSON should fail closed on its canonical column");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column,
                ..
            } if logical_column == expected_column
        ));
    }
    Ok(())
}

#[test]
fn stored_user_action_request_fails_closed_on_incompatible_required_for(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_incompatible_required_for_reread";
    let request_id = "action_incompatible_required_for_reread";

    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new(
                "idem_store_incompatible_required_for_reread",
            )),
            &RequestHash::new("sha256:incompatible-required-for-reread"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task(
                "incompatible_required_for_reread",
                task_id,
            )],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                user_action_request_insert(request_id, task_id, None),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    let stored_request_json: String = store.conn.query_row(
        "SELECT request_json
               FROM user_action_requests
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
        params![PROJECT_ID, request_id],
        |row| row.get(0),
    )?;
    let mut request_json = serde_json::from_str::<Value>(&stored_request_json)?;
    request_json["required_for"] = json!(["close_cancel"]);
    store.conn.execute(
        "UPDATE user_action_requests
                SET request_json = ?3,
                    required_for_json = '[\"close_cancel\"]'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
        params![PROJECT_ID, request_id, request_json.to_string()],
    )?;

    let error = store
        .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)
        .expect_err("incompatible persisted required_for must fail closed");
    assert!(matches!(
        error,
        StoreError::CorruptOwnerStateValue {
            table: "user_action_requests",
            logical_column: "request_json",
            ..
        }
    ));
    Ok(())
}

#[test]
fn stored_user_action_request_fails_closed_on_invalid_timestamp_order() -> Result<(), Box<dyn Error>>
{
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_request_timestamp_reread";
    let request_id = "action_request_timestamp_reread";
    let mut action = user_action_request_insert(request_id, task_id, None);
    set_user_action_request_expiry(&mut action, "2026-01-01T00:00:10Z");
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_request_timestamp_reread")),
            &RequestHash::new("sha256:request-timestamp-reread"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("request_timestamp_reread", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;

    let stored_request_json: String = store.conn.query_row(
        "SELECT request_json
               FROM user_action_requests
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
        params![PROJECT_ID, request_id],
        |row| row.get(0),
    )?;
    let mut request_json = serde_json::from_str::<Value>(&stored_request_json)?;
    request_json["expires_at"] = json!("2026-01-01T00:00:00Z");
    store.conn.execute(
        "UPDATE user_action_requests
                SET request_json = ?3,
                    expires_at = '2026-01-01T00:00:00Z'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
        params![PROJECT_ID, request_id, request_json.to_string()],
    )?;

    let error = store
        .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:00:00Z")?)
        .expect_err("invalid stored request timestamp order must fail closed");
    assert!(matches!(
        error,
        StoreError::CorruptOwnerStateValue {
            table: "user_action_requests",
            logical_column: "expires_at",
            ..
        }
    ));
    Ok(())
}

#[test]
fn stored_user_action_resolution_fails_closed_on_invalid_timestamp_order(
) -> Result<(), Box<dyn Error>> {
    for (suffix, corrupted_resolved_at) in [
        ("before_request", "2025-12-31T23:59:59.999Z"),
        ("at_expiry", "2026-01-01T00:00:10Z"),
    ] {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = format!("task_resolution_timestamp_reread_{suffix}");
        let request_id = format!("action_resolution_timestamp_reread_{suffix}");
        let resolution_id = format!("resolution_timestamp_reread_{suffix}");
        let mut action = user_action_request_insert(&request_id, &task_id, None);
        set_user_action_request_expiry(&mut action, "2026-01-01T00:00:10Z");
        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new(format!(
                    "idem_resolution_timestamp_reread_request_{suffix}"
                ))),
                &RequestHash::new(format!(
                    "sha256:resolution-timestamp-reread-request-{suffix}"
                )),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task(
                    &format!("{suffix}_request"),
                    &task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::Task(TaskMutation::insert(task_insert(&task_id)))
                    .apply(mutation, facts)
                    .map(|_| ())?;
                CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(action))
                    .apply(mutation, facts)
                    .map(|_| ())
            },
            response_json,
        )?;
        let mut resolution = user_action_resolution_insert(&resolution_id, &request_id);
        resolution.resolved_at = "2026-01-01T00:00:05Z".to_owned();
        store.commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new(format!(
                    "idem_resolution_timestamp_reread_resolve_{suffix}"
                ))),
                &RequestHash::new(format!(
                    "sha256:resolution-timestamp-reread-resolve-{suffix}"
                )),
                Some(user_replay_context()),
                Some(1),
                vec![pending_event_for_task(
                    &format!("{suffix}_resolve"),
                    &task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(resolution))
                    .apply(mutation, facts)
                    .map(|_| ())
            },
            response_json,
        )?;

        store.conn.execute(
            "UPDATE user_action_resolutions
                    SET resolved_at = ?3
                  WHERE project_id = ?1
                    AND user_action_resolution_id = ?2",
            params![PROJECT_ID, resolution_id, corrupted_resolved_at],
        )?;
        let error = store
            .user_action_resolution_record(&resolution_id)
            .expect_err("invalid stored resolution timestamp order must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_resolutions",
                logical_column: "resolved_at",
                ..
            }
        ));
    }
    Ok(())
}

#[test]
fn effective_user_action_rejects_resolution_from_future_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_resolution_future_reread";
    let request_id = "action_resolution_future_reread";
    let resolution_id = "resolution_future_reread";
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_resolution_future_request")),
            &RequestHash::new("sha256:resolution-future-request"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("resolution_future_request", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                user_action_request_insert(request_id, task_id, None),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_resolution_future_resolve")),
            &RequestHash::new("sha256:resolution-future-resolve"),
            Some(user_replay_context()),
            Some(1),
            vec![pending_event_for_task("resolution_future_resolve", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::UserAction(UserActionMutation::InsertResolution(
                user_action_resolution_insert(resolution_id, request_id),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;
    store.conn.execute(
        "UPDATE user_action_resolutions
                SET resolved_at = '2999-07-13T00:00:00Z'
              WHERE project_id = ?1 AND user_action_resolution_id = ?2",
        params![PROJECT_ID, resolution_id],
    )?;
    let before = (store.effect_counts()?, store.project_state()?);
    let now = UtcTimestamp::parse(&store.current_timestamp()?)?;

    let error = store
        .user_action_record(request_id, &now)
        .expect_err("a future stored resolution cannot be current authority");
    assert!(matches!(
        error,
        StoreError::CorruptOwnerStateValue {
            table: "user_action_resolutions",
            logical_column: "resolved_at",
            ..
        }
    ));
    assert_eq!((store.effect_counts()?, store.project_state()?), before);
    Ok(())
}

#[test]
fn effective_user_action_read_enforces_requested_at_lower_bound() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_requested_at_lower_bound";
    let request_id = "action_requested_at_lower_bound";

    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
            Some(&IdempotencyKey::new("idem_store_requested_at_lower_bound")),
            &RequestHash::new("sha256:requested-at-lower-bound"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("requested_at_lower_bound", task_id)],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())?;
            CoreStorageMutation::UserAction(UserActionMutation::InsertRequest(
                user_action_request_insert(request_id, task_id, None),
            ))
            .apply(mutation, facts)
            .map(|_| ())
        },
        response_json,
    )?;

    let error = store
        .user_action_record(
            request_id,
            &UtcTimestamp::parse("2025-12-31T23:59:59.999Z")?,
        )
        .expect_err("time before requested_at must fail closed");
    assert!(matches!(
        error,
        StoreError::CorruptOwnerStateValue {
            table: "user_action_requests",
            logical_column: "requested_at",
            ..
        }
    ));

    assert_eq!(
        store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:00:00Z")?,)?
            .expect("requested_at boundary is inclusive")
            .status,
        UserActionStatus::Pending
    );
    Ok(())
}

fn user_action_request_insert(
    request_id: &str,
    task_id: &str,
    expires_at: Option<&str>,
) -> UserActionRequestInsert {
    let request_json = json!({
        "body": {
            "action_type": "choice",
            "judgment_kind": "product_decision",
            "presentation": "short",
            "question": "Choose the current product direction.",
            "options": [{
                "option_id": "accept",
                "label": "Accept",
                "description": "Accept the current direction.",
                "consequence": "The work may continue.",
                "machine_action": "accept",
                "resolution_outcome": "accepted",
                "is_default": true
            }],
            "context": {
                "summary": "A bounded choice is required.",
                "related_refs": [],
                "artifact_refs": [],
                "visible_risks": [],
                "constraints": []
            },
            "affected_refs": [],
            "sensitive_action_scope": null
        },
        "required_for": ["informational"],
        "expires_at": expires_at
    })
    .to_string();
    let basis_json = json!({
        "action_type": "choice",
        "coordinates": {
            "task_id": task_id,
            "change_unit_id": null,
            "scope_revision": 0,
            "baseline_ref": null,
            "created_at_state_version": 0,
            "compatibility_status": "current"
        },
        "close_basis_revision": null,
        "result_refs": [],
        "residual_risk_ids": [],
        "sensitive_action_scope": null
    })
    .to_string();
    UserActionRequestInsert {
        user_action_request_id: request_id.to_owned(),
        task_id: task_id.to_owned(),
        change_unit_id: None,
        action_kind: UserActionKind::ProductDecision,
        request_json,
        basis_json,
        basis_status: UserActionBasisStatus::Current,
        required_for_json: r#"["informational"]"#.to_owned(),
        requested_by_actor_source: ACTOR_SOURCE.to_owned(),
        source_method: MethodName::RequestUserAction.as_str().to_owned(),
        source_idempotency_key: format!("idem_{request_id}"),
        requested_at: "2026-01-01T00:00:00Z".to_owned(),
        expires_at: expires_at.map(str::to_owned),
        metadata_json: "{}".to_owned(),
    }
}

fn set_user_action_request_expiry(input: &mut UserActionRequestInsert, expires_at: &str) {
    let mut request_json = serde_json::from_str::<Value>(&input.request_json)
        .expect("test user-action request JSON should decode");
    request_json["expires_at"] = json!(expires_at);
    input.request_json = request_json.to_string();
    input.expires_at = Some(expires_at.to_owned());
}

fn evidence_user_action_request_insert(
    request_id: &str,
    task_id: &str,
    produced_at_state_version: u64,
) -> UserActionRequestInsert {
    let target = json!({
        "target_kind": "acceptance_criterion",
        "acceptance_criterion_id": "criterion_observation_reread"
    });
    let artifact = user_action_artifact_ref_json(task_id, produced_at_state_version);
    UserActionRequestInsert {
        user_action_request_id: request_id.to_owned(),
        task_id: task_id.to_owned(),
        change_unit_id: None,
        action_kind: UserActionKind::EvidenceObservation,
        request_json: json!({
            "body": {
                "action_type": "evidence_observation",
                "question": "Does this artifact support the criterion?",
                "context_summary": "Review the exact stored artifact bytes.",
                "target_candidates": [target.clone()],
                "artifact_candidates": [artifact.clone()]
            },
            "required_for": ["record_run"],
            "expires_at": "2026-01-01T00:15:00Z"
        })
        .to_string(),
        basis_json: json!({
            "action_type": "evidence_observation",
            "coordinates": {
                "task_id": task_id,
                "change_unit_id": null,
                "scope_revision": 0,
                "baseline_ref": null,
                "created_at_state_version": 0,
                "compatibility_status": "current"
            },
            "target_candidates": [target],
            "artifact_candidates": [artifact]
        })
        .to_string(),
        basis_status: UserActionBasisStatus::Current,
        required_for_json: r#"["record_run"]"#.to_owned(),
        requested_by_actor_source: ACTOR_SOURCE.to_owned(),
        source_method: MethodName::RequestUserAction.as_str().to_owned(),
        source_idempotency_key: format!("idem_{request_id}"),
        requested_at: "2026-01-01T00:00:00Z".to_owned(),
        expires_at: Some("2026-01-01T00:15:00Z".to_owned()),
        metadata_json: "{}".to_owned(),
    }
}

fn evidence_user_action_resolution_insert(
    resolution_id: &str,
    request_id: &str,
    task_id: &str,
    produced_at_state_version: u64,
) -> UserActionResolutionInsert {
    UserActionResolutionInsert {
        user_action_resolution_id: resolution_id.to_owned(),
        user_action_request_id: request_id.to_owned(),
        action_kind: UserActionKind::EvidenceObservation,
        channel_kind: UserActionChannelKind::Cli,
        channel_submission_id: format!("submission_{resolution_id}"),
        resolution_json: json!({
            "resolution_type": "evidence_observation",
            "observation": {
                "target": {
                    "target_kind": "acceptance_criterion",
                    "acceptance_criterion_id": "criterion_observation_reread"
                },
                "relevance_status": "supported",
                "output_artifact_refs": [user_action_artifact_ref_json(
                    task_id,
                    produced_at_state_version
                )],
                "summary": "The exact artifact bytes support the criterion."
            }
        })
        .to_string(),
        resolved_by_actor_source: "local_user".to_owned(),
        resolved_verification_basis: UserActionVerificationBasis::CliDirectUserChannel,
        resolved_assurance_level: "local_user_channel".to_owned(),
        resolved_at: "2026-01-01T00:10:00Z".to_owned(),
    }
}

fn user_action_artifact_ref_json(task_id: &str, produced_at_state_version: u64) -> Value {
    json!({
        "artifact_id": "artifact_observation_reread",
        "project_id": PROJECT_ID,
        "task_id": task_id,
        "display_name": "observation.json",
        "content_type": "application/json",
        "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "size_bytes": 64,
        "integrity_status": "verified",
        "redaction_state": "none",
        "availability": "available",
        "created_by_run_ref": {
            "record_kind": "run",
            "record_id": "run_observation_reread",
            "project_id": PROJECT_ID,
            "task_id": task_id,
            "produced_at_state_version": produced_at_state_version
        },
        "created_by_actor_source": ACTOR_SOURCE,
        "storage_ref": "artifact-storage://observation-reread"
    })
}

fn user_action_resolution_insert(
    resolution_id: &str,
    request_id: &str,
) -> UserActionResolutionInsert {
    UserActionResolutionInsert {
        user_action_resolution_id: resolution_id.to_owned(),
        user_action_request_id: request_id.to_owned(),
        action_kind: UserActionKind::ProductDecision,
        channel_kind: UserActionChannelKind::Cli,
        channel_submission_id: format!("submission_{resolution_id}"),
        resolution_json: json!({
            "resolution_type": "choice",
            "selected_option_id": "accept",
            "machine_action": "accept",
            "resolution_outcome": "accepted",
            "note": null,
            "accepted_risk_ids": []
        })
        .to_string(),
        resolved_by_actor_source: "local_user".to_owned(),
        resolved_verification_basis: UserActionVerificationBasis::CliDirectUserChannel,
        resolved_assurance_level: "local_user_channel".to_owned(),
        resolved_at: "2026-01-01T00:10:00Z".to_owned(),
    }
}

fn choice_resolution_json(
    selected_option_id: &str,
    machine_action: UserActionOptionAction,
    resolution_outcome: JudgmentResolutionOutcome,
) -> String {
    json!({
        "resolution_type": "choice",
        "selected_option_id": selected_option_id,
        "machine_action": machine_action,
        "resolution_outcome": resolution_outcome,
        "note": null,
        "accepted_risk_ids": []
    })
    .to_string()
}
