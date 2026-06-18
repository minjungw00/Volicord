use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use harness_store::{
    bootstrap::{
        initialize_runtime_home, register_project, register_surface, ProjectRegistration,
        SurfaceRegistration, ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::{CoreProjectStore, StorageEffectCounts},
    sqlite::open_project_state_database,
};
use harness_test_support::TempRuntimeHome;
use harness_types::{
    ActorKind, ChangeUnitUpdate, IdempotencyKey, InitialScope, RequestId, ScopeUpdate,
    SequenceDurableIdGenerator, SurfaceId, VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};
use serde_json::{json, Map, Value};

use super::*;

const PROJECT_ID: &str = "project_methods";
const SURFACE_ID: &str = "surface_methods";
const SURFACE_INSTANCE_ID: &str = "surface_instance_methods";

struct MethodHarness {
    _runtime_home: TempRuntimeHome,
    runtime_home_path: PathBuf,
    service: CoreService,
}

impl MethodHarness {
    fn new() -> Result<Self, Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("core-methods")?;
        let repo_root = runtime_home.path().join("repo");
        fs::create_dir_all(&repo_root)?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_methods", "{}")?;
        register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: PROJECT_ID.to_owned(),
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        register_surface(
            runtime_home.path(),
            SurfaceRegistration {
                project_id: PROJECT_ID.to_owned(),
                surface_id: SURFACE_ID.to_owned(),
                surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
                surface_kind: "local_test".to_owned(),
                display_name: Some("Method Test Surface".to_owned()),
                capability_profile_json: json!({
                    "access_class": "write_authorization",
                    "supported_access_classes": ["write_authorization"]
                })
                .to_string(),
                local_access_json: json!({
                    "access_class": "core_mutation",
                    "authorized_access_classes": [
                        "read_status",
                        "core_mutation",
                        "write_authorization",
                        "run_recording",
                        "artifact_registration",
                        "artifact_read"
                    ],
                    "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
                })
                .to_string(),
                metadata_json: "{}".to_owned(),
            },
        )?;

        let runtime_home_path = runtime_home.path().to_path_buf();
        let service = CoreService::new(&runtime_home_path);
        Ok(Self {
            _runtime_home: runtime_home,
            runtime_home_path,
            service,
        })
    }

    fn counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
        let store = CoreProjectStore::open(&self.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
        Ok(store.effect_counts()?)
    }

    fn conn(&self) -> Result<rusqlite::Connection, Box<dyn Error>> {
        Ok(open_project_state_database(
            self.runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite"),
        )?)
    }
}

#[test]
fn reused_request_id_does_not_collide_for_core_generated_records() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let request_id = "req_reused_for_generated_ids";

    let first_intake = harness.service.intake(
        intake_request(
            request_id,
            "idem_reused_intake_1",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let first_task_id = response_record_id(&first_intake.response_value, "task_ref");
    let first_event_id = response_event_id(&first_intake.response_value);

    let second_intake = harness.service.intake(
        intake_request(
            request_id,
            "idem_reused_intake_2",
            false,
            Some(1),
            RequestedMode::Work,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let second_task_id = response_record_id(&second_intake.response_value, "task_ref");
    let second_event_id = response_event_id(&second_intake.response_value);
    assert_ne!(first_task_id, second_task_id);
    assert_ne!(first_event_id, second_event_id);

    let first_scope = harness.service.update_scope(
        update_scope_request(
            request_id,
            "idem_reused_scope_1",
            false,
            Some(2),
            &second_task_id,
            ChangeUnitOperation::CreateCurrent,
            "First reused request scope.",
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let first_change_unit_id = response_record_id(&first_scope.response_value, "change_unit_ref");
    let first_scope_event_id = response_event_id(&first_scope.response_value);

    let second_scope = harness.service.update_scope(
        update_scope_request(
            request_id,
            "idem_reused_scope_2",
            false,
            Some(3),
            &second_task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Second reused request scope.",
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let second_change_unit_id = response_record_id(&second_scope.response_value, "change_unit_ref");
    let second_scope_event_id = response_event_id(&second_scope.response_value);
    assert_ne!(first_change_unit_id, second_change_unit_id);
    assert_ne!(first_scope_event_id, second_scope_event_id);

    let first_write = harness.service.prepare_write(
        prepare_write_request(
            request_id,
            "idem_reused_write_1",
            Some(4),
            Some(&second_task_id),
            Some(&second_change_unit_id),
        ),
        invocation(AccessClass::WriteAuthorization),
    )?;
    let first_write_id = response_record_id(&first_write.response_value, "write_authorization_ref");
    let first_write_event_id = response_event_id(&first_write.response_value);

    let second_write = harness.service.prepare_write(
        prepare_write_request(
            request_id,
            "idem_reused_write_2",
            Some(5),
            Some(&second_task_id),
            Some(&second_change_unit_id),
        ),
        invocation(AccessClass::WriteAuthorization),
    )?;
    let second_write_id =
        response_record_id(&second_write.response_value, "write_authorization_ref");
    let second_write_event_id = response_event_id(&second_write.response_value);
    assert_ne!(first_write_id, second_write_id);
    assert_ne!(first_write_event_id, second_write_event_id);

    let first_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            request_id,
            "idem_reused_judgment_1",
            false,
            Some(6),
            &second_task_id,
            Some(&second_change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let first_judgment_id = response_record_id(&first_judgment.response_value, "user_judgment_ref");
    let first_judgment_event_id = response_event_id(&first_judgment.response_value);

    let second_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            request_id,
            "idem_reused_judgment_2",
            false,
            Some(7),
            &second_task_id,
            Some(&second_change_unit_id),
            JudgmentKind::TechnicalDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let second_judgment_id =
        response_record_id(&second_judgment.response_value, "user_judgment_ref");
    let second_judgment_event_id = response_event_id(&second_judgment.response_value);
    assert_ne!(first_judgment_id, second_judgment_id);
    assert_ne!(first_judgment_event_id, second_judgment_event_id);

    Ok(())
}

#[test]
fn reused_request_id_stage_artifact_returns_distinct_handles() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_reused_request")?;

    let first = harness.service.stage_artifact(
        stage_artifact_request("req_stage_reused", None, false, None, &task_id),
        invocation(AccessClass::ArtifactRegistration),
    )?;
    let second = harness.service.stage_artifact(
        stage_artifact_request("req_stage_reused", None, false, None, &task_id),
        invocation(AccessClass::ArtifactRegistration),
    )?;

    let first_handle = first.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("first handle should be present");
    let second_handle = second.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("second handle should be present");
    assert_ne!(first_handle, second_handle);
    assert_eq!(harness.counts()?.artifact_staging, 2);
    Ok(())
}

#[test]
fn idempotent_replay_returns_original_generated_ids() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let request = intake_request(
        "req_replay_generated_ids",
        "idem_replay_generated_ids",
        false,
        Some(0),
        RequestedMode::Work,
    );

    let first = harness
        .service
        .intake(request.clone(), invocation(AccessClass::CoreMutation))?;
    let second = harness
        .service
        .intake(request, invocation(AccessClass::CoreMutation))?;

    assert!(second.replayed);
    assert_eq!(
        response_record_id(&first.response_value, "task_ref"),
        response_record_id(&second.response_value, "task_ref")
    );
    assert_eq!(
        response_event_id(&first.response_value),
        response_event_id(&second.response_value)
    );
    assert_eq!(harness.counts()?.tasks, 1);
    assert_eq!(harness.counts()?.task_events, 1);
    Ok(())
}

#[test]
fn deterministic_generated_id_collision_retries_bounded_candidates() -> Result<(), Box<dyn Error>> {
    let mut harness = MethodHarness::new()?;
    insert_superseding_task(&harness, "task_collision")?;
    harness.service = CoreService::with_id_generator(
        &harness.runtime_home_path,
        SequenceDurableIdGenerator::new(["collision", "fresh", "event"]),
    );

    let response = harness.service.intake(
        intake_request(
            "req_collision_retry",
            "idem_collision_retry",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(
        response_record_id(&response.response_value, "task_ref"),
        "task_fresh"
    );
    assert_eq!(response_event_id(&response.response_value), "evt_event");
    assert_eq!(harness.counts()?.tasks, 2);
    Ok(())
}

fn response_record_id(response_value: &Value, field: &str) -> String {
    response_value[field]["record_id"]
        .as_str()
        .expect("record_id should be present")
        .to_owned()
}

fn response_event_id(response_value: &Value) -> String {
    response_value["base"]["events"][0]["event_id"]
        .as_str()
        .expect("event_id should be present")
        .to_owned()
}

#[test]
fn status_is_read_only_including_dry_run() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status", None, false, None, None),
            include: status_include(),
        },
        invocation(AccessClass::ReadStatus),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(response.response_value["base"]["dry_run"], false);
    assert_eq!(response.response_value["base"]["events"], json!([]));
    assert_eq!(harness.counts()?, before);

    let dry_run = harness.service.status(
        StatusRequest {
            envelope: envelope(
                "req_status_dry",
                Some("idem_status_dry"),
                true,
                Some(0),
                None,
            ),
            include: status_include(),
        },
        invocation(AccessClass::ReadStatus),
    )?;

    assert_eq!(dry_run.response_value["base"]["response_kind"], "result");
    assert_eq!(dry_run.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(dry_run.response_value["base"]["dry_run"], true);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn invalid_stored_method_owned_json_routes_to_structured_unavailability(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_method_json")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_method_json_judgment",
            "idem_bad_method_json_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    harness.conn()?.execute(
        "UPDATE user_judgments
                SET options_json = '{not-json'
              WHERE project_id = ?1
                AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id],
    )?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_bad_method_json_record",
            "idem_bad_method_json_record",
            Some(3),
            &task_id,
            &judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_store_rejection(&response, "MCP_UNAVAILABLE", "corrupt_stored_json");
    assert_eq!(
        response.response_value["errors"][0]["details"]["field"],
        "user_judgments.options_json"
    );
    assert_public_response_has_no_internal_leak(&response, &harness.runtime_home_path);
    Ok(())
}

#[test]
fn authority_owner_json_decode_paths_do_not_reintroduce_fail_open_patterns() {
    let sources = [
        ("methods/mod.rs", include_str!("mod.rs")),
        ("methods/close_task.rs", include_str!("close_task.rs")),
        ("methods/prepare_write.rs", include_str!("prepare_write.rs")),
        ("methods/record_run.rs", include_str!("record_run.rs")),
        ("methods/update_scope.rs", include_str!("update_scope.rs")),
        ("methods/status.rs", include_str!("status.rs")),
    ];
    let forbidden = [
        "parse_json_object(&task.completion_policy_json)",
        "parse_json_object(&context.task.close_summary_json)",
        "parse_json_object(&record.close_basis_json)",
        "parse_json_object(&record.lifecycle_json)",
        "parse_json_object(&change_unit.write_basis_json)",
        "serde_json::from_str::<Vec<String>>(&change_unit.bounded_paths_json).unwrap_or_default()",
    ];

    for (path, source) in sources {
        for pattern in forbidden {
            assert!(
                !source.contains(pattern),
                "{path} reintroduced fail-open owner-state JSON decoding: {pattern}"
            );
        }
    }
}

#[test]
fn public_methods_use_same_verified_surface_context() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "verified_context")?;

    let status = harness.service.status(
        StatusRequest {
            envelope: envelope("req_verified_status", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(AccessClass::ReadStatus),
    )?;
    assert_verified_surface(&status, AccessClass::ReadStatus);

    let intake = harness.service.intake(
        intake_request(
            "req_verified_intake",
            "idem_verified_intake",
            true,
            Some(2),
            RequestedMode::Work,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    assert_verified_surface(&intake, AccessClass::CoreMutation);

    let update_scope = harness.service.update_scope(
        update_scope_request(
            "req_verified_scope",
            "idem_verified_scope",
            true,
            Some(2),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Initial current scope.",
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    assert_verified_surface(&update_scope, AccessClass::CoreMutation);

    let mut prepare_write = prepare_write_request(
        "req_verified_prepare",
        "idem_verified_prepare",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    prepare_write.envelope.dry_run = true;
    let prepare_write = harness
        .service
        .prepare_write(prepare_write, invocation(AccessClass::WriteAuthorization))?;
    assert_verified_surface(&prepare_write, AccessClass::WriteAuthorization);

    let stage_artifact = harness.service.stage_artifact(
        stage_artifact_request(
            "req_verified_stage",
            Some("idem_verified_stage"),
            true,
            Some(2),
            &task_id,
        ),
        invocation(AccessClass::ArtifactRegistration),
    )?;
    assert_verified_surface(&stage_artifact, AccessClass::ArtifactRegistration);

    let record_run = harness.service.record_run(
        record_run_request(
            "req_verified_run",
            "idem_verified_run",
            true,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(AccessClass::RunRecording),
    )?;
    assert_verified_surface(&record_run, AccessClass::RunRecording);

    let request_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_verified_judgment_preview",
            "idem_verified_judgment_preview",
            true,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    assert_verified_surface(&request_judgment, AccessClass::CoreMutation);

    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_verified_judgment_pending",
            "idem_verified_judgment_pending",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let mut record_judgment = record_judgment_request(
        "req_verified_record_judgment",
        "idem_verified_record_judgment",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );
    record_judgment.envelope.dry_run = true;
    let record_judgment = harness
        .service
        .record_user_judgment(record_judgment, invocation(AccessClass::CoreMutation))?;
    assert_verified_surface(&record_judgment, AccessClass::CoreMutation);

    let close_check = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_verified_close",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(AccessClass::ReadStatus),
    )?;
    assert_verified_surface(&close_check, AccessClass::ReadStatus);

    Ok(())
}

#[test]
fn intake_commits_once_and_replays_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let request = intake_request(
        "req_intake",
        "idem_intake",
        false,
        Some(0),
        RequestedMode::Auto,
    );

    let first = harness
        .service
        .intake(request.clone(), invocation(AccessClass::CoreMutation))?;
    let after_first = harness.counts()?;

    assert_eq!(first.response_value["base"]["response_kind"], "result");
    assert_eq!(
        first.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(first.response_value["base"]["state_version"], 1);
    assert_eq!(first.response_value["state"]["mode"], "work");
    assert_eq!(after_first.state_version, before.state_version + 1);
    assert_eq!(after_first.tasks, before.tasks + 1);
    assert_eq!(after_first.task_events, before.task_events + 1);
    assert_eq!(after_first.tool_invocations, before.tool_invocations + 1);

    let second = harness
        .service
        .intake(request, invocation(AccessClass::CoreMutation))?;
    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn intake_dry_run_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let response = harness.service.intake(
        intake_request(
            "req_intake_dry",
            "idem_intake_dry",
            true,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn update_scope_commits_once_and_creates_one_current_change_unit() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_scope_task",
            "idem_scope_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let before = harness.counts()?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_create",
            "idem_scope_create",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Create current export scope.",
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 2);
    assert!(response.response_value["change_unit_ref"].is_object());
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.change_units, before.change_units + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(active_current_change_units(&harness, &task_id)?, 1);
    Ok(())
}

#[test]
fn update_scope_replaces_current_and_marks_write_authorization_stale() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_replace_task",
            "idem_replace_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let create = harness.service.update_scope(
        update_scope_request(
            "req_replace_create",
            "idem_replace_create",
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Initial current scope.",
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let change_unit_id = create.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .expect("change unit ref should be present")
        .to_owned();
    insert_active_write_authorization(&harness, &task_id, &change_unit_id)?;
    let before = harness.counts()?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_replace_current",
            "idem_replace_current",
            false,
            Some(2),
            &task_id,
            ChangeUnitOperation::ReplaceCurrent,
            "Replacement current scope.",
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["stale_write_authorization_refs"]
            .as_array()
            .expect("stale refs should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.change_units, before.change_units + 1);
    assert_eq!(active_current_change_units(&harness, &task_id)?, 1);
    assert_eq!(write_authorization_status(&harness, "wa_replace")?, "stale");
    Ok(())
}

#[test]
fn update_scope_dry_run_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_dry_task",
            "idem_dry_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let before = harness.counts()?;

    let response = harness.service.update_scope(
        update_scope_request(
            "req_scope_dry",
            "idem_scope_dry",
            true,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Dry-run scope.",
        ),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn scope_decision_ref_alone_does_not_change_current_scope() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_decision_task",
            "idem_decision_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let decision_ref = StateRecordRef {
        record_kind: StateRecordKind::UserJudgment,
        record_id: RecordId::new("uj_scope_decision"),
        project_id: ProjectId::new(PROJECT_ID),
        task_id: Some(TaskId::new(&task_id)),
        state_version: Some(1),
    };

    let response = harness.service.update_scope(
        UpdateScopeRequest {
            envelope: envelope(
                "req_decision_only",
                Some("idem_decision_only"),
                false,
                Some(1),
                Some(&task_id),
            ),
            task_id: TaskId::new(&task_id),
            goal_summary: None,
            scope_update: None,
            scope_boundary: None,
            non_goals: None,
            acceptance_criteria: None,
            autonomy_boundary: None,
            baseline_ref: None,
            change_unit: ChangeUnitUpdate {
                operation: ChangeUnitOperation::KeepCurrent,
                fields: Map::new(),
            },
            related_scope_decision_refs: vec![decision_ref],
        },
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(
        response.response_value["state"]["scope_summary"],
        "Initial test scope."
    );
    assert_eq!(
        response.response_value["linked_scope_decision_refs"]
            .as_array()
            .expect("linked refs should be an array")
            .len(),
        1
    );
    Ok(())
}

#[test]
fn prepare_write_allowed_creates_one_authorization_with_post_commit_basis(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_allowed")?;
    let sensitive_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_prepare_allowed_sensitive",
            "idem_prepare_allowed_sensitive",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let sensitive_judgment_id =
        response_record_id(&sensitive_judgment.response_value, "user_judgment_ref");
    harness.service.record_user_judgment(
        record_judgment_request(
            "req_prepare_allowed_record",
            "idem_prepare_allowed_record",
            Some(3),
            &task_id,
            &sensitive_judgment_id,
            JudgmentKind::SensitiveApproval,
            answer_payload(JudgmentKind::SensitiveApproval),
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_allowed",
        "idem_prepare_allowed",
        Some(4),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "allowed");
    assert_eq!(response.response_value["authorization_effect"], "created");
    assert_eq!(response.response_value["base"]["state_version"], 5);
    assert_eq!(
        response.response_value["write_authorization"]["basis_state_version"],
        5
    );
    assert_eq!(
        response.response_value["write_authorization"]["authorized_attempt_scope"]
            ["intended_paths"],
        json!(["src/export.rs"])
    );
    assert_eq!(
        response.response_value["active_user_judgment_refs"]
            .as_array()
            .expect("active judgment refs should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.write_authorizations, before.write_authorizations + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    let write_authorization_id =
        response_record_id(&response.response_value, "write_authorization_ref");
    assert_eq!(
        write_authorization_basis(&harness, &write_authorization_id)?,
        5
    );
    Ok(())
}

#[test]
fn prepare_write_blocked_path_creates_no_authorization() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_path")?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_path",
        "idem_prepare_path",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["src/other.rs".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "path_out_of_scope");
    assert!(response.response_value["write_authorization"].is_null());
    assert!(response.response_value["write_authorization_ref"].is_null());
    assert_eq!(response.response_value["authorization_effect"], "none");
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.write_authorizations, before.write_authorizations);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    assert_eq!(after.artifact_staging, before.artifact_staging);
    assert_eq!(after.artifacts, before.artifacts);
    assert_eq!(after.artifact_links, before.artifact_links);
    assert_eq!(after.evidence_summaries, before.evidence_summaries);
    assert_eq!(after.blockers, before.blockers);
    assert_eq!(after.runs, before.runs);
    let event_payload = assert_latest_prepare_write_event(
        &harness,
        &response.response_value,
        "blocked",
        "path_out_of_scope",
    )?;
    assert_eq!(event_payload["task_id"], task_id);
    assert_eq!(event_payload["change_unit_id"], change_unit_id);
    let reason = event_payload["write_decision_reasons"][0].clone();
    assert_eq!(reason["category"], "scope");
    assert_eq!(reason["code"], "path_out_of_scope");
    assert!(reason["message"]
        .as_str()
        .expect("reason message should be present")
        .contains("outside the current Change Unit path scope"));
    assert!(!reason["related_refs"]
        .as_array()
        .expect("related_refs should be an array")
        .is_empty());
    Ok(())
}

#[test]
fn prepare_write_missing_change_unit_returns_decision_reason() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_prepare_no_cu_task",
            "idem_prepare_no_cu_task",
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_no_cu",
        "idem_prepare_no_cu",
        Some(1),
        Some(&task_id),
        None,
    );
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "no_current_change_unit");
    assert_eq!(after.write_authorizations, before.write_authorizations);
    Ok(())
}

#[test]
fn prepare_write_unresolved_user_judgment_requires_decision() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_judgment")?;
    harness.service.request_user_judgment(
        user_judgment_request(
            "req_prepare_judgment_pending",
            "idem_prepare_judgment_pending",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_judgment",
        "idem_prepare_judgment",
        Some(3),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "decision_required");
    assert_prepare_reason(&response.response_value, "user_judgment_unresolved");
    assert_eq!(after.write_authorizations, before.write_authorizations);
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    let event_payload = assert_latest_prepare_write_event(
        &harness,
        &response.response_value,
        "decision_required",
        "user_judgment_unresolved",
    )?;
    let reason = event_payload["write_decision_reasons"][0].clone();
    assert_eq!(reason["category"], "user_judgment");
    assert_eq!(reason["code"], "user_judgment_unresolved");
    assert!(reason["message"]
        .as_str()
        .expect("reason message should be present")
        .contains("user-owned judgment"));
    assert!(!reason["related_refs"]
        .as_array()
        .expect("related_refs should be an array")
        .is_empty());
    Ok(())
}

#[test]
fn prepare_write_missing_sensitive_approval_requires_approval() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_sensitive")?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_sensitive",
        "idem_prepare_sensitive",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.sensitive_categories = vec!["network".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "approval_required");
    assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
    assert_eq!(after.write_authorizations, before.write_authorizations);
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    let event_payload = assert_latest_prepare_write_event(
        &harness,
        &response.response_value,
        "approval_required",
        "sensitive_approval_missing",
    )?;
    let reason = event_payload["write_decision_reasons"][0].clone();
    assert_eq!(reason["category"], "sensitive_approval");
    assert_eq!(reason["code"], "sensitive_approval_missing");
    assert!(reason["message"]
        .as_str()
        .expect("reason message should be present")
        .contains("sensitive-action approval"));
    assert!(reason["related_refs"]
        .as_array()
        .expect("related_refs should be an array")
        .is_empty());
    Ok(())
}

#[test]
fn prepare_write_baseline_mismatch_blocks_authorization() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_baseline")?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_baseline",
        "idem_prepare_baseline",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.baseline_ref = BaselineRef::new("baseline_other");
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "baseline_mismatch");
    assert_eq!(after.write_authorizations, before.write_authorizations);
    Ok(())
}

#[test]
fn prepare_write_surface_access_mismatch_is_access_rejection() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_surface")?;
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_surface_access",
        "idem_prepare_surface_access",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::CoreMutation))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "LOCAL_ACCESS_MISMATCH"
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(after.write_authorizations, before.write_authorizations);
    Ok(())
}

#[test]
fn prepare_write_unregistered_grant_fails_before_method_decision() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_grant_fail")?;
    set_surface_local_access(
        &harness,
        json!({
            "authorized_access_classes": ["core_mutation"],
            "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
        }),
    )?;
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_grant_fail",
        "idem_prepare_grant_fail",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "LOCAL_ACCESS_MISMATCH"
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn prepare_write_surface_capability_insufficient_is_method_decision() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_cap")?;
    set_surface_capability(&harness, "{}")?;
    let before = harness.counts()?;

    let request = prepare_write_request(
        "req_prepare_capability",
        "idem_prepare_capability",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "surface_capability_insufficient");
    assert_eq!(after.write_authorizations, before.write_authorizations);
    Ok(())
}

#[test]
fn prepare_write_product_write_flag_mismatch_blocks_authorization() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_flag")?;
    let before = harness.counts()?;

    let mut request = prepare_write_request(
        "req_prepare_flag",
        "idem_prepare_flag",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.product_file_write_intended = false;
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["decision"], "blocked");
    assert_prepare_reason(&response.response_value, "product_write_flag_mismatch");
    assert_eq!(after.write_authorizations, before.write_authorizations);
    Ok(())
}

#[test]
fn prepare_write_dry_run_has_no_authorization_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_dry")?;
    let before = harness.counts()?;
    let before_decision_events = write_decision_event_count(&harness)?;

    let mut request = prepare_write_request(
        "req_prepare_dry",
        "idem_prepare_dry",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.envelope.dry_run = true;
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(
        response.response_value["dry_run_summary"]["planned_effects"][0]["action"],
        "would_create"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );

    let mut blocked_preview = prepare_write_request(
        "req_prepare_dry_blocked",
        "idem_prepare_dry_blocked",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    blocked_preview.envelope.dry_run = true;
    blocked_preview.intended_paths = vec!["src/other.rs".to_owned()];
    let blocked_preview = harness
        .service
        .prepare_write(blocked_preview, invocation(AccessClass::WriteAuthorization))?;
    assert_eq!(
        blocked_preview.response_value["base"]["response_kind"],
        "dry_run"
    );
    assert_eq!(
        blocked_preview.response_value["dry_run_summary"]["would_blockers"][0]["code"],
        "path_out_of_scope"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );
    Ok(())
}

#[test]
fn prepare_write_rejects_escaping_product_path_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_escape")?;
    let before = harness.counts()?;
    let before_decision_events = write_decision_event_count(&harness)?;

    let mut request = prepare_write_request(
        "req_prepare_escape",
        "idem_prepare_escape",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["../outside.rs".to_owned()];
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );
    Ok(())
}

#[test]
fn prepare_write_stale_state_rejects_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_stale")?;
    let before = harness.counts()?;
    let before_decision_events = write_decision_event_count(&harness)?;

    let request = prepare_write_request(
        "req_prepare_stale",
        "idem_prepare_stale",
        Some(1),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert!(!response.response_json.contains("write_decision_reasons"));
    assert!(!response
        .response_json
        .contains("STATE_VERSION_CONFLICT\",\"category"));
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        write_decision_event_count(&harness)?,
        before_decision_events
    );
    Ok(())
}

#[test]
fn prepare_write_idempotency_replays_without_second_authorization() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_replay")?;
    let request = prepare_write_request(
        "req_prepare_replay",
        "idem_prepare_replay",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );

    let first = harness
        .service
        .prepare_write(request.clone(), invocation(AccessClass::WriteAuthorization))?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

    assert_eq!(first.response_value["decision"], "allowed");
    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    assert_eq!(write_authorization_count(&harness)?, 1);
    Ok(())
}

#[test]
fn prepare_write_non_allow_replay_returns_original_response_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_non_allow_replay")?;
    let mut request = prepare_write_request(
        "req_prepare_non_allow_replay",
        "idem_prepare_non_allow_replay",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.intended_paths = vec!["src/other.rs".to_owned()];
    let before = harness.counts()?;

    let first = harness
        .service
        .prepare_write(request.clone(), invocation(AccessClass::WriteAuthorization))?;
    let after_first = harness.counts()?;
    let same_context = harness
        .service
        .prepare_write(request.clone(), invocation(AccessClass::WriteAuthorization))?;
    let context_mismatch = harness
        .service
        .prepare_write(request, invocation(AccessClass::CoreMutation))?;

    assert_eq!(first.response_value["decision"], "blocked");
    assert_prepare_reason(&first.response_value, "path_out_of_scope");
    assert_eq!(after_first.state_version, before.state_version + 1);
    assert_eq!(after_first.task_events, before.task_events + 1);
    assert_eq!(after_first.tool_invocations, before.tool_invocations + 1);
    assert_eq!(
        after_first.write_authorizations,
        before.write_authorizations
    );
    assert_latest_prepare_write_event(
        &harness,
        &first.response_value,
        "blocked",
        "path_out_of_scope",
    )?;
    assert!(same_context.replayed);
    assert_eq!(same_context.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    assert!(!context_mismatch.replayed);
    assert_eq!(
        context_mismatch.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(
        context_mismatch.response_value["errors"][0]["code"],
        "LOCAL_ACCESS_MISMATCH"
    );
    assert!(!context_mismatch.response_json.contains("path_out_of_scope"));
    assert!(context_mismatch
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn prepare_write_replay_requires_current_verified_grant() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "prepare_replay_verify")?;
    let request = prepare_write_request(
        "req_prepare_replay_verify",
        "idem_prepare_replay_verify",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    let first = harness
        .service
        .prepare_write(request.clone(), invocation(AccessClass::WriteAuthorization))?;
    let after_first = harness.counts()?;
    set_surface_local_access(
        &harness,
        json!({
            "authorized_access_classes": ["core_mutation"],
            "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
        }),
    )?;

    let second = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

    assert_eq!(first.response_value["decision"], "allowed");
    assert!(!second.replayed);
    assert_eq!(second.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        second.response_value["errors"][0]["code"],
        "LOCAL_ACCESS_MISMATCH"
    );
    assert_ne!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn stage_artifact_creates_transient_handle_without_core_commit() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_valid")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_valid",
        Some("idem_stage_valid"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "trace.log".to_owned();
    request.content_type = "text/plain; charset=utf-8".to_owned();
    request.safe_bytes_or_notice = "Local trace sample captured for debugging.".to_owned();
    let response = harness
        .service
        .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;
    let after = harness.counts()?;
    let handle_id = response.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("handle id should be present")
        .to_owned();
    let row = staged_artifact_row(&harness, &handle_id)?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["base"]["effect_kind"],
        "staging_created"
    );
    assert_eq!(response.response_value["base"]["state_version"], 2);
    assert_eq!(response.response_value["base"]["events"], json!([]));
    assert_eq!(
        response.response_value["staged_artifact_handle"]["consumed"],
        false
    );
    assert_eq!(response.response_value.get("artifact_ref"), None);
    assert_eq!(after.state_version, before.state_version);
    assert_eq!(after.artifact_staging, before.artifact_staging + 1);
    assert_eq!(after.artifacts, before.artifacts);
    assert_eq!(after.task_events, before.task_events);
    assert_eq!(after.tool_invocations, before.tool_invocations);
    assert_eq!(row.status, "staged");
    assert_eq!(row.redaction_state, "none");
    assert_eq!(row.created_by_surface_id, SURFACE_ID);
    assert_eq!(row.created_by_surface_instance_id, SURFACE_INSTANCE_ID);
    assert!(row.tmp_path.ends_with(".txt"));
    assert!(harness
        .runtime_home_path
        .join("projects")
        .join(PROJECT_ID)
        .join(&row.tmp_path)
        .exists());
    assert!(
        (23.99..=24.01).contains(&row.ttl_hours),
        "expected 24h TTL, got {}",
        row.ttl_hours
    );
    Ok(())
}

#[test]
fn stage_artifact_rejects_checksum_mismatch_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_sha")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_sha",
        Some("idem_stage_sha"),
        false,
        Some(2),
        &task_id,
    );
    request.safe_bytes_or_notice = "checksum mismatch sample".to_owned();
    request.expected_sha256 = Some("sha256:0000".to_owned());
    let response = harness
        .service
        .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_size_mismatch_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_size")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_size",
        Some("idem_stage_size"),
        false,
        Some(2),
        &task_id,
    );
    request.safe_bytes_or_notice = "size mismatch sample".to_owned();
    request.expected_size_bytes = Some(999);
    let response = harness
        .service
        .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_oversized_input_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_big")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_big",
        Some("idem_stage_big"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "huge.log".to_owned();
    request.safe_bytes_or_notice = "x".repeat(MAX_STAGED_BODY_BYTES + 1);
    let response = harness
        .service
        .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_unsafe_secret_input_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_secret")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_secret",
        Some("idem_stage_secret"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "secrets.log".to_owned();
    request.safe_bytes_or_notice = "password=hunter2".to_owned();
    let response = harness
        .service
        .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_rejects_unsupported_redaction_state() -> Result<(), Box<dyn Error>> {
    let mut value = serde_json::to_value(stage_artifact_request(
        "req_stage_bad_redaction",
        Some("idem_stage_bad_redaction"),
        false,
        Some(2),
        "task_redaction",
    ))?;
    value["redaction_state"] = json!("unsupported");

    let error = serde_json::from_value::<StageArtifactRequest>(value)
        .expect_err("unsupported redaction_state should not deserialize");
    assert!(error.to_string().contains("unknown variant"));
    Ok(())
}

#[test]
fn stage_artifact_dry_run_creates_no_handle_or_storage() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_dry")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_dry",
        Some("idem_stage_dry"),
        true,
        Some(2),
        &task_id,
    );
    request.display_name = "trace.md".to_owned();
    request.content_type = "text/markdown".to_owned();
    request.redaction_state = RedactionState::Redacted;
    request.safe_bytes_or_notice = "Redacted diagnostic excerpt.".to_owned();
    let response = harness
        .service
        .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert!(response
        .response_value
        .get("staged_artifact_handle")
        .is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_dry_run_still_checks_stale_state() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_dry_stale")?;
    let before = harness.counts()?;

    let request = stage_artifact_request(
        "req_stage_dry_stale",
        Some("idem_stage_dry_stale"),
        true,
        Some(1),
        &task_id,
    );
    let response = harness
        .service
        .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_invalid_input_does_not_bypass_access_preflight() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_access_first")?;
    let before = harness.counts()?;

    let mut request = stage_artifact_request(
        "req_stage_access_first",
        Some("idem_stage_access_first"),
        true,
        Some(2),
        &task_id,
    );
    request.safe_bytes_or_notice = String::new();
    let response = harness
        .service
        .stage_artifact(request, invocation(AccessClass::ReadStatus))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "LOCAL_ACCESS_MISMATCH"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn stage_artifact_uses_verified_surface_provenance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_stage_artifact_capability(&harness)?;
    let (task_id, _) = create_task_with_change_unit(&harness, "stage_provenance")?;

    let mut request = stage_artifact_request(
        "req_stage_provenance",
        Some("idem_stage_provenance"),
        false,
        Some(2),
        &task_id,
    );
    request.display_name = "binary.bin".to_owned();
    request.content_type = "application/octet-stream".to_owned();
    request.redaction_state = RedactionState::Blocked;
    request.safe_bytes_or_notice = "Binary output omitted; see local run context.".to_owned();

    let response = harness
        .service
        .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

    assert_eq!(
        response.response_value["staged_artifact_handle"]["created_by_surface_id"],
        SURFACE_ID
    );
    assert_eq!(
        response.response_value["staged_artifact_handle"]["created_by_surface_instance_id"],
        SURFACE_INSTANCE_ID
    );
    assert_eq!(
        response.response_value["staged_artifact_handle"]["redaction_state"],
        "blocked"
    );
    let handle_id = response.response_value["staged_artifact_handle"]["handle_id"]
        .as_str()
        .expect("handle id should be present");
    let row = staged_artifact_row(&harness, handle_id)?;
    assert_eq!(row.created_by_surface_id, SURFACE_ID);
    assert_eq!(row.created_by_surface_instance_id, SURFACE_INSTANCE_ID);
    Ok(())
}

#[test]
fn stage_artifact_rejects_caller_submitted_provenance_fields() -> Result<(), Box<dyn Error>> {
    let mut value = serde_json::to_value(stage_artifact_request(
        "req_stage_forged_provenance",
        Some("idem_stage_forged_provenance"),
        false,
        Some(2),
        "task_forged_provenance",
    ))?;
    value["created_by_surface_id"] = json!("forged_surface");
    value["created_by_surface_instance_id"] = json!("forged_instance");

    let error = serde_json::from_value::<StageArtifactRequest>(value)
        .expect_err("caller-submitted provenance fields should be rejected");

    assert!(error.to_string().contains("created_by_surface_id"));
    Ok(())
}

#[test]
fn record_run_without_product_write_commits_run_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_no_write")?;
    let before = harness.counts()?;

    let response = harness.service.record_run(
        record_run_request(
            "req_run_no_write",
            "idem_run_no_write",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(AccessClass::RunRecording),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["run_summary"]["observed_changes"]["product_file_write_observed"],
        false
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.write_authorizations, before.write_authorizations);
    assert_eq!(after.artifacts, before.artifacts);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn record_run_product_write_consumes_valid_authorization_once() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_write")?;
    let write_authorization_id =
        prepare_write_authorization(&harness, &task_id, &change_unit_id, 2, "run_write")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_write",
        "idem_run_write",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    request.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
    let response = harness
        .service
        .record_run(request, invocation(AccessClass::RunRecording))?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["state_version"], 4);
    assert_eq!(
        write_authorization_status(&harness, &write_authorization_id)?,
        "consumed"
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.write_authorizations, before.write_authorizations);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn record_run_missing_authorization_rejects_product_write_without_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_missing_auth")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_missing_auth",
        "idem_run_missing_auth",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    let response = harness
        .service
        .record_run(request, invocation(AccessClass::RunRecording))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_AUTHORIZATION_REQUIRED"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_stale_authorization_basis_rejects_before_consumption() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stale_auth")?;
    let write_authorization_id =
        prepare_write_authorization(&harness, &task_id, &change_unit_id, 2, "run_stale_auth")?;
    harness.service.update_scope(
        update_scope_request(
            "req_run_stale_auth_touch",
            "idem_run_stale_auth_touch",
            false,
            Some(3),
            &task_id,
            ChangeUnitOperation::KeepCurrent,
            "Initial current scope.",
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stale_auth",
        "idem_run_stale_auth",
        false,
        Some(4),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    request.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
    let response = harness
        .service
        .record_run(request, invocation(AccessClass::RunRecording))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert_eq!(
        write_authorization_status(&harness, &write_authorization_id)?,
        "active"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_consumed_authorization_reuse_rejects_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_reuse_auth")?;
    let write_authorization_id =
        prepare_write_authorization(&harness, &task_id, &change_unit_id, 2, "run_reuse_auth")?;

    let mut first = record_run_request(
        "req_run_reuse_first",
        "idem_run_reuse_first",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    first.observed_changes.product_file_write_observed = true;
    first.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    first.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
    harness
        .service
        .record_run(first, invocation(AccessClass::RunRecording))?;
    let before = harness.counts()?;

    let mut second = record_run_request(
        "req_run_reuse_second",
        "idem_run_reuse_second",
        false,
        Some(4),
        &task_id,
        &change_unit_id,
    );
    second.observed_changes.product_file_write_observed = true;
    second.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
    second.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
    let response = harness
        .service
        .record_run(second, invocation(AccessClass::RunRecording))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_AUTHORIZATION_INVALID"
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["authorization_reason"],
        "consumed"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_path_mismatch_rejects_without_consuming_authorization() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_path_auth")?;
    let write_authorization_id =
        prepare_write_authorization(&harness, &task_id, &change_unit_id, 2, "run_path_auth")?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_path_auth",
        "idem_run_path_auth",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    request.observed_changes.product_file_write_observed = true;
    request.observed_changes.changed_paths = vec!["tests/export.rs".to_owned()];
    request.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
    let response = harness
        .service
        .record_run(request, invocation(AccessClass::RunRecording))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "WRITE_AUTHORIZATION_INVALID"
    );
    assert_eq!(
        write_authorization_status(&harness, &write_authorization_id)?,
        "active"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_promotes_staged_artifact_and_updates_evidence() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_artifact")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_artifact", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_artifact",
        "idem_run_artifact",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_report",
        handle,
        Some("validation_report"),
        Some("Search-result count validation passed."),
    )];
    request.evidence_updates = vec![supported_evidence_update(
        "Search-result count validation passed.",
    )];
    let response = harness
        .service
        .record_run(request, invocation(AccessClass::RunRecording))?;
    let after = harness.counts()?;
    let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
        .as_str()
        .expect("artifact id should be present")
        .to_owned();

    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["evidence_summary"]["status"],
        "sufficient"
    );
    assert_eq!(
        response.response_value["evidence_summary"]["coverage_items"][0]["supporting_refs"][0]
            ["record_kind"],
        "run"
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.runs, before.runs + 1);
    assert_eq!(after.artifacts, before.artifacts + 1);
    assert_eq!(after.artifact_links, before.artifact_links + 2);
    assert_eq!(after.evidence_summaries, before.evidence_summaries + 1);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "consumed");
    assert!(artifact_owner_link_exists(&harness, &artifact_id, "run")?);
    assert!(artifact_owner_link_exists(
        &harness,
        &artifact_id,
        "evidence_summary"
    )?);
    Ok(())
}

#[test]
fn record_run_staged_artifact_surface_mismatch_rejects_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_surface")?;
    let mut handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_surface", 2)?;
    handle.created_by_surface_id = SurfaceId::new("forged_surface");
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_surface",
        "idem_run_stage_surface",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_surface",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(AccessClass::RunRecording))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_surface_mismatch"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_expired_staged_artifact_rejects_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_expired")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_expired", 2)?;
    expire_staged_artifact(&harness, handle.handle_id.as_str())?;
    let before = harness.counts()?;

    let mut request = record_run_request(
        "req_run_stage_expired",
        "idem_run_stage_expired",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_expired",
        handle,
        None,
        None,
    )];
    let response = harness
        .service
        .record_run(request, invocation(AccessClass::RunRecording))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_expired"
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn record_run_checksum_mismatch_rejects_and_rolls_back_all_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_sha")?;
    let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_sha", 2)?;
    let handle_id = handle.handle_id.as_str().to_owned();
    let before = harness.counts()?;

    let mut input = artifact_input_for_handle("artifact_input_sha", handle, None, None);
    input.expected_sha256 = Some("sha256:0000".to_owned());
    let mut request = record_run_request(
        "req_run_stage_sha",
        "idem_run_stage_sha",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.artifact_inputs = vec![input];
    let response = harness
        .service
        .record_run(request, invocation(AccessClass::RunRecording))?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
        "staged_handle_checksum_mismatch"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
    Ok(())
}

#[test]
fn record_run_dry_run_and_idempotency_replay_have_no_extra_effects() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_replay")?;
    let before_dry = harness.counts()?;
    let dry_run = harness.service.record_run(
        record_run_request(
            "req_run_dry",
            "idem_run_dry",
            true,
            Some(2),
            &task_id,
            &change_unit_id,
        ),
        invocation(AccessClass::RunRecording),
    )?;
    assert_eq!(dry_run.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(harness.counts()?, before_dry);

    let request = record_run_request(
        "req_run_replay",
        "idem_run_replay",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    let first = harness
        .service
        .record_run(request.clone(), invocation(AccessClass::RunRecording))?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .record_run(request, invocation(AccessClass::RunRecording))?;

    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn request_user_judgment_creates_pending_record() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "pending")?;
    let before = harness.counts()?;

    let response = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_pending",
            "idem_judgment_pending",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let after = harness.counts()?;
    let judgment_id = response_record_id(&response.response_value, "user_judgment_ref");

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 3);
    assert_eq!(
        response.response_value["user_judgment"]["status"],
        "pending"
    );
    assert_eq!(
        response.response_value["user_judgment"]["judgment_kind"],
        "product_decision"
    );
    assert_eq!(
        response.response_value["state"]["pending_user_judgment_refs"]
            .as_array()
            .expect("pending refs should be an array")
            .len(),
        1
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_judgments, before.user_judgments + 1);
    assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
    Ok(())
}

#[test]
fn record_user_judgment_resolves_pending_record() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "resolve")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_resolve",
            "idem_judgment_resolve",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_resolve",
            "idem_record_resolve",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 4);
    assert_eq!(
        response.response_value["user_judgment"]["status"],
        "resolved"
    );
    assert_eq!(
        response.response_value["user_judgment"]["resolution"]["resolved_by_actor_kind"],
        "user"
    );
    assert_eq!(
        response.response_value["state"]["pending_user_judgment_refs"]
            .as_array()
            .expect("pending refs should be an array")
            .len(),
        0
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_judgments, before.user_judgments);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "resolved"
    );
    assert!(
        resolution_json(&harness, &pending_judgment_id)?["answer"]["product_decision"].is_object()
    );
    Ok(())
}

#[test]
fn incompatible_judgment_kind_is_rejected_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "kind")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_kind",
            "idem_judgment_kind",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_wrong_kind",
            "idem_record_wrong_kind",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::TechnicalDecision,
            answer_payload(JudgmentKind::TechnicalDecision),
        ),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "DECISION_UNRESOLVED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn final_acceptance_does_not_substitute_for_residual_risk_acceptance() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_risk",
            "idem_judgment_risk",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ResidualRiskAcceptance,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_final_for_risk",
            "idem_record_final_for_risk",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ResidualRiskAcceptance,
            answer_payload(JudgmentKind::FinalAcceptance),
        ),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "VALIDATION_FAILED"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn sensitive_action_scope_does_not_create_write_authorization() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "sensitive")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_sensitive",
            "idem_judgment_sensitive",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_sensitive",
            "idem_record_sensitive",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::SensitiveApproval,
            answer_payload(JudgmentKind::SensitiveApproval),
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(after.write_authorizations, before.write_authorizations);
    assert_eq!(
        response.response_value["state"]["write_authority_summary"],
        Value::Null
    );
    Ok(())
}

#[test]
fn recorded_scope_decision_does_not_change_scope_or_current_change_unit(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_judgment")?;
    let original_scope = current_change_unit_scope(&harness, &task_id)?;
    let original_current = current_change_unit_id(&harness, &task_id)?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_scope",
            "idem_judgment_scope",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ScopeDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_scope",
            "idem_record_scope",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ScopeDecision,
            answer_payload(JudgmentKind::ScopeDecision),
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let after = harness.counts()?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["state"]["scope_summary"],
        "Initial current scope."
    );
    assert_eq!(
        current_change_unit_scope(&harness, &task_id)?,
        original_scope
    );
    assert_eq!(
        current_change_unit_id(&harness, &task_id)?,
        original_current
    );
    assert_eq!(after.change_units, before.change_units);
    Ok(())
}

#[test]
fn judgment_dry_runs_have_no_storage_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "dry_judgment")?;
    let before_request = harness.counts()?;

    let request_preview = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_dry",
            "idem_judgment_dry",
            true,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(
        request_preview.response_value["base"]["response_kind"],
        "dry_run"
    );
    assert_eq!(harness.counts()?, before_request);

    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_dry_record",
            "idem_judgment_dry_record",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before_record = harness.counts()?;

    let mut record_preview_request = record_judgment_request(
        "req_record_dry",
        "idem_record_dry",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );
    record_preview_request.envelope.dry_run = true;
    let record_preview = harness.service.record_user_judgment(
        record_preview_request,
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(
        record_preview.response_value["base"]["response_kind"],
        "dry_run"
    );
    assert_eq!(harness.counts()?, before_record);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn stale_state_rejects_record_user_judgment_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "stale_judgment")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_stale",
            "idem_judgment_stale",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let before = harness.counts()?;

    let response = harness.service.record_user_judgment(
        record_judgment_request(
            "req_record_stale",
            "idem_record_stale",
            Some(2),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        ),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(
        user_judgment_status(&harness, &pending_judgment_id)?,
        "pending"
    );
    Ok(())
}

#[test]
fn record_user_judgment_idempotency_replays_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "replay_judgment")?;
    let pending_judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_judgment_replay",
            "idem_judgment_replay",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::ProductDecision,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let pending_judgment_id =
        response_record_id(&pending_judgment.response_value, "user_judgment_ref");
    let request = record_judgment_request(
        "req_record_replay",
        "idem_record_replay",
        Some(3),
        &task_id,
        &pending_judgment_id,
        JudgmentKind::ProductDecision,
        answer_payload(JudgmentKind::ProductDecision),
    );

    let first = harness
        .service
        .record_user_judgment(request.clone(), invocation(AccessClass::CoreMutation))?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .record_user_judgment(request, invocation(AccessClass::CoreMutation))?;

    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn close_task_check_is_read_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_check")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(AccessClass::ReadStatus),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(response.response_value["base"]["events"], json!([]));
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_check_dry_run_is_read_only() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_check_dry")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_check_dry",
            idempotency_key: Some("idem_close_check_dry"),
            dry_run: true,
            expected_state_version: Some(1),
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(AccessClass::ReadStatus),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(response.response_value["base"]["dry_run"], true);
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_completion_policy_rejects_close_check_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "bad_policy_check")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "completion_policy_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_policy_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(AccessClass::ReadStatus),
    )?;

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "completion_policy_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_completion_policy_rejects_close_complete_without_effect() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "bad_policy_complete")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "completion_policy_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_policy_complete",
            idempotency_key: Some("idem_bad_policy_complete"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "completion_policy_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn schema_invalid_close_summary_rejects_instead_of_hiding_residual_risk(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "bad_close_summary")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_summary_json",
        Some(r#"{"residual_risks":"known-but-wrong-shape"}"#),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_close_summary",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(AccessClass::ReadStatus),
    )?;

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "close_summary_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_close_basis_stops_close_readiness_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_close_basis")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "close_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_close_basis",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(AccessClass::ReadStatus),
    )?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "close_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_lifecycle_state_does_not_default_close_phase() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_lifecycle")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "lifecycle_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_lifecycle",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(AccessClass::ReadStatus),
    )?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "lifecycle_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_write_basis_rejects_prepare_write_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_write_basis")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "write_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_bad_write_basis",
            "idem_bad_write_basis",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(AccessClass::WriteAuthorization),
    )?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "write_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_bounded_paths_rejects_prepare_write_without_empty_scope() -> Result<(), Box<dyn Error>>
{
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_paths")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "bounded_paths_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.prepare_write(
        prepare_write_request(
            "req_bad_paths",
            "idem_bad_paths",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ),
        invocation(AccessClass::WriteAuthorization),
    )?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "bounded_paths_json",
        &harness.runtime_home_path,
    );
    assert!(response
        .response_value
        .get("write_decision_reasons")
        .is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn prepare_write_dry_run_with_corrupt_owner_state_is_rejected_no_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "dry_bad_owner")?;
    set_change_unit_owner_json(
        &harness,
        &change_unit_id,
        "write_basis_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;
    let mut request = prepare_write_request(
        "req_dry_bad_owner",
        "idem_dry_bad_owner",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );
    request.envelope.dry_run = true;

    let response = harness
        .service
        .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

    assert_owner_state_rejection(
        &response,
        "change_units",
        &change_unit_id,
        "write_basis_json",
        &harness.runtime_home_path,
    );
    assert_eq!(response.response_value["base"]["dry_run"], true);
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn status_read_only_rejects_corrupt_owner_state_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "status_bad_owner")?;
    set_task_owner_json(
        &harness,
        &task_id,
        "close_summary_json",
        Some(corrupt_owner_json()),
    )?;
    let before = harness.counts()?;

    let response = harness.service.status(
        StatusRequest {
            envelope: envelope("req_status_bad_owner", None, false, None, Some(&task_id)),
            include: status_include(),
        },
        invocation(AccessClass::ReadStatus),
    )?;

    assert_owner_state_rejection(
        &response,
        "tasks",
        &task_id,
        "close_summary_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn optional_resolution_null_remains_absent_not_corrupt() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "null_resolution")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_null_resolution_judgment",
            "idem_null_resolution_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(&harness, &judgment_id, None)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_null_resolution_close",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(AccessClass::ReadStatus),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn malformed_optional_resolution_json_rejects_close_readiness() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "bad_resolution")?;
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            "req_bad_resolution_judgment",
            "idem_bad_resolution_judgment",
            false,
            Some(2),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let judgment_id = response_record_id(&judgment.response_value, "user_judgment_ref");
    set_user_judgment_resolution_json(&harness, &judgment_id, Some(corrupt_owner_json()))?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_bad_resolution_close",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }),
        invocation(AccessClass::ReadStatus),
    )?;

    assert_owner_state_rejection(
        &response,
        "user_judgments",
        &judgment_id,
        "resolution_json",
        &harness.runtime_home_path,
    );
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_complete_blocks_missing_final_acceptance() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_no_final")?;
    let state_version =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "no_final", true)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_no_final",
            idempotency_key: Some("idem_close_no_final"),
            dry_run: false,
            expected_state_version: Some(state_version),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_complete_blocks_unsupported_evidence_claim() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_bad_evidence")?;
    let after_evidence = record_close_evidence(
        &harness,
        &task_id,
        &change_unit_id,
        2,
        "bad_evidence",
        false,
    )?;
    let after_final =
        record_final_acceptance(&harness, &task_id, &change_unit_id, after_evidence, "bad")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_bad_evidence",
            idempotency_key: Some("idem_close_bad_evidence"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(response.response_value["close_state"], "blocked");
    assert_close_blocker(&response.response_value, "evidence_claim_unsupported");
    assert_no_close_blocker(&response.response_value, "STATE_VERSION_CONFLICT");
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_complete_success() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_success")?;
    let after_evidence =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "success", true)?;
    let after_final =
        record_final_acceptance(&harness, &task_id, &change_unit_id, after_evidence, "ok")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_success",
            idempotency_key: Some("idem_close_success"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(AccessClass::CoreMutation),
    )?;
    let after = harness.counts()?;
    let fields = task_terminal_fields(&harness, &task_id)?;

    assert_eq!(response.response_value["close_state"], "closed");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(
        response.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(
        response.response_value["base"]["state_version"],
        after_final + 1
    );
    assert_eq!(fields.lifecycle_phase, "completed");
    assert_eq!(fields.result.as_deref(), Some("completed"));
    assert_eq!(
        fields.close_summary["close_reason"],
        "completed_self_checked"
    );
    assert!(fields.closed_at.is_some());
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn close_task_cancel_success_despite_missing_completion_evidence() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_cancel")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_cancel",
            idempotency_key: Some("idem_close_cancel"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Cancel,
            close_reason: Some(CloseReason::Cancelled),
            superseding_task_id: None,
        }),
        invocation(AccessClass::CoreMutation),
    )?;
    let after = harness.counts()?;
    let fields = task_terminal_fields(&harness, &task_id)?;

    assert_eq!(response.response_value["close_state"], "cancelled");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(fields.lifecycle_phase, "cancelled");
    assert_eq!(fields.result.as_deref(), Some("cancelled"));
    assert_eq!(fields.close_summary["close_reason"], "cancelled");
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn close_task_supersede_success() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_supersede")?;
    let superseding_task_id = "task_close_superseding";
    insert_superseding_task(&harness, superseding_task_id)?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_supersede",
            idempotency_key: Some("idem_close_supersede"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Supersede,
            close_reason: Some(CloseReason::Superseded),
            superseding_task_id: Some(superseding_task_id),
        }),
        invocation(AccessClass::CoreMutation),
    )?;
    let after = harness.counts()?;
    let fields = task_terminal_fields(&harness, &task_id)?;

    assert_eq!(response.response_value["close_state"], "superseded");
    assert_eq!(response.response_value["blockers"], json!([]));
    assert_eq!(fields.lifecycle_phase, "superseded");
    assert_eq!(fields.result.as_deref(), Some("superseded"));
    assert_eq!(
        active_task_id(&harness)?.as_deref(),
        Some(superseding_task_id)
    );
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.task_events, before.task_events + 1);
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn close_task_stale_state_rejected_without_blocker() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_stale")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_stale",
            idempotency_key: Some("idem_close_stale"),
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "STATE_VERSION_CONFLICT"
    );
    assert!(response.response_value.get("blockers").is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_blocker_code_routing_uses_method_local_codes() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, _) = create_task_with_change_unit(&harness, "close_codes")?;
    let before = harness.counts()?;

    let response = harness.service.close_task(
        close_task_request(CloseTaskFixture {
            request_id: "req_close_codes",
            idempotency_key: Some("idem_close_codes"),
            dry_run: false,
            expected_state_version: Some(2),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }),
        invocation(AccessClass::CoreMutation),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_close_blocker(&response.response_value, "missing_final_acceptance");
    assert_no_close_blocker(&response.response_value, "STATE_VERSION_CONFLICT");
    assert!(response.response_value.get("errors").is_none());
    assert_eq!(harness.counts()?, before);
    Ok(())
}

#[test]
fn close_task_idempotency_replays_terminal_transition() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_replay")?;
    let after_evidence =
        record_close_evidence(&harness, &task_id, &change_unit_id, 2, "replay", true)?;
    let after_final = record_final_acceptance(
        &harness,
        &task_id,
        &change_unit_id,
        after_evidence,
        "replay",
    )?;
    let request = close_task_request(CloseTaskFixture {
        request_id: "req_close_replay",
        idempotency_key: Some("idem_close_replay"),
        dry_run: false,
        expected_state_version: Some(after_final),
        task_id: &task_id,
        intent: CloseIntent::Complete,
        close_reason: Some(CloseReason::CompletedSelfChecked),
        superseding_task_id: None,
    });

    let first = harness
        .service
        .close_task(request.clone(), invocation(AccessClass::CoreMutation))?;
    let after_first = harness.counts()?;
    let second = harness
        .service
        .close_task(request, invocation(AccessClass::CoreMutation))?;

    assert_eq!(first.response_value["close_state"], "closed");
    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

fn envelope(
    request_id: &str,
    idempotency_key: Option<&str>,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: Option<&str>,
) -> ToolEnvelope {
    ToolEnvelope {
        project_id: ProjectId::new(PROJECT_ID),
        task_id: task_id.map(TaskId::new),
        actor_kind: ActorKind::Agent,
        surface_id: SurfaceId::new(SURFACE_ID),
        request_id: RequestId::new(request_id),
        idempotency_key: idempotency_key.map(IdempotencyKey::new),
        expected_state_version,
        dry_run,
        locale: None,
    }
}

fn invocation(access_class: AccessClass) -> InvocationContext {
    InvocationContext {
        surface_instance_id: Some(SurfaceInstanceId::new(SURFACE_INSTANCE_ID)),
        requested_access_class: access_class,
        invocation_binding_basis: VERIFICATION_BASIS_TEST_FIXTURE_BINDING.to_owned(),
    }
}

fn assert_verified_surface(response: &PipelineResponse, access_class: AccessClass) {
    let verified = response
        .verified_surface
        .as_ref()
        .expect("method response should carry verified surface context");
    assert_eq!(verified.project_id.as_str(), PROJECT_ID);
    assert_eq!(verified.surface_id.as_str(), SURFACE_ID);
    assert_eq!(verified.surface_instance_id.as_str(), SURFACE_INSTANCE_ID);
    assert_eq!(verified.access_class, access_class);
    assert!(verified
        .verification_basis
        .contains(VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION));
    assert!(verified
        .verification_basis
        .contains(VERIFICATION_BASIS_TEST_FIXTURE_BINDING));
}

fn assert_store_rejection(
    response: &PipelineResponse,
    expected_code: &str,
    expected_category: &str,
) {
    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(response.response_value["errors"][0]["code"], expected_code);
    assert_eq!(
        response.response_value["errors"][0]["details"]["store_failure_category"],
        expected_category
    );
}

fn assert_owner_state_rejection(
    response: &PipelineResponse,
    table: &str,
    record_ref: &str,
    logical_column: &str,
    runtime_home_path: &Path,
) {
    assert_store_rejection(response, "MCP_UNAVAILABLE", "corrupt_stored_json");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    let details = &response.response_value["errors"][0]["details"];
    assert_eq!(details["owner_state_error"]["table"], table);
    assert_eq!(details["owner_state_error"]["record_ref"], record_ref);
    assert_eq!(
        details["owner_state_error"]["logical_column"],
        logical_column
    );
    assert_eq!(
        details["owner_state_error"]["corruption_category"],
        "corrupt_stored_json"
    );
    assert!(!response.response_json.contains(corrupt_owner_json()));
    assert!(!response
        .response_json
        .contains("/home/minjungw00/Projects/Harness_Project/secret"));
    assert_public_response_has_no_internal_leak(response, runtime_home_path);
}

fn assert_public_response_has_no_internal_leak(
    response: &PipelineResponse,
    runtime_home_path: &Path,
) {
    let body = &response.response_json;
    let runtime_home = runtime_home_path.to_string_lossy();
    assert!(!body.contains(runtime_home.as_ref()));
    for fragment in [
        "SELECT ",
        "INSERT INTO",
        "UPDATE ",
        "DELETE ",
        "constraint failed",
        "state.sqlite",
    ] {
        assert!(
            !body.contains(fragment),
            "public response leaked internal fragment {fragment}: {body}"
        );
    }
}

fn corrupt_owner_json() -> &'static str {
    "{not-json /home/minjungw00/Projects/Harness_Project/secret"
}

fn status_include() -> StatusInclude {
    StatusInclude {
        task: true,
        pending_user_judgments: true,
        write_authority: true,
        evidence: true,
        close: true,
        guarantees: true,
    }
}

fn intake_request(
    request_id: &str,
    idempotency_key: &str,
    dry_run: bool,
    expected_state_version: Option<u64>,
    requested_mode: RequestedMode,
) -> harness_types::IntakeRequest {
    harness_types::IntakeRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            dry_run,
            expected_state_version,
            None,
        ),
        plain_language_request: "Create a test export flow.".to_owned(),
        requested_mode,
        resume_policy: ResumePolicy::CreateNew,
        initial_scope: InitialScope {
            boundary: "Initial test scope.".to_owned(),
            non_goals: vec!["Changing unrelated flows.".to_owned()],
            acceptance_criteria: vec!["The test export flow is represented.".to_owned()],
        },
        initial_context_refs: Vec::new(),
    }
}

fn update_scope_request(
    request_id: &str,
    idempotency_key: &str,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: &str,
    operation: ChangeUnitOperation,
    scope_summary: &str,
) -> UpdateScopeRequest {
    let mut fields = Map::new();
    fields.insert(
        "scope_summary".to_owned(),
        Value::String(scope_summary.to_owned()),
    );
    fields.insert(
        "affected_paths".to_owned(),
        json!(["src/export.rs", "tests/export.rs"]),
    );
    UpdateScopeRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            dry_run,
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        goal_summary: Some(scope_summary.to_owned()),
        scope_update: Some(ScopeUpdate {
            include: vec![scope_summary.to_owned()],
            exclude: vec!["Unrelated behavior.".to_owned()],
        }),
        scope_boundary: Some(scope_summary.to_owned()),
        non_goals: Some(vec!["Unrelated behavior.".to_owned()]),
        acceptance_criteria: Some(vec!["The scoped behavior is represented.".to_owned()]),
        autonomy_boundary: Some("Stay inside the scoped test behavior.".to_owned()),
        baseline_ref: Some(BaselineRef::new("baseline_test")),
        change_unit: ChangeUnitUpdate { operation, fields },
        related_scope_decision_refs: Vec::new(),
    }
}

fn prepare_write_request(
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: Option<u64>,
    task_id: Option<&str>,
    change_unit_id: Option<&str>,
) -> PrepareWriteRequest {
    PrepareWriteRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            false,
            expected_state_version,
            task_id,
        ),
        task_id: task_id.map(TaskId::new),
        change_unit_id: change_unit_id.map(ChangeUnitId::new),
        intended_operation: "local_sensitive_step".to_owned(),
        intended_paths: vec!["src/export.rs".to_owned()],
        product_file_write_intended: true,
        sensitive_categories: Vec::new(),
        baseline_ref: BaselineRef::new("baseline_test"),
    }
}

fn stage_artifact_request(
    request_id: &str,
    idempotency_key: Option<&str>,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: &str,
) -> StageArtifactRequest {
    StageArtifactRequest {
        envelope: envelope(
            request_id,
            idempotency_key,
            dry_run,
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        display_name: "trace.log".to_owned(),
        content_type: "text/plain".to_owned(),
        redaction_state: RedactionState::None,
        safe_bytes_or_notice: "staging sample".to_owned(),
        expected_sha256: None,
        expected_size_bytes: None,
        relation_hint: Some("diagnostic_log".to_owned()),
    }
}

fn record_run_request(
    request_id: &str,
    idempotency_key: &str,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: &str,
    change_unit_id: &str,
) -> RecordRunRequest {
    RecordRunRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            dry_run,
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        change_unit_id: ChangeUnitId::new(change_unit_id),
        kind: harness_types::RunKind::Implementation,
        run_id: None,
        baseline_ref: BaselineRef::new("baseline_test"),
        write_authorization_id: None,
        summary: "Recorded implementation run.".to_owned(),
        observed_changes: ObservedChanges {
            changed_paths: Vec::new(),
            product_file_write_observed: false,
            sensitive_categories: Vec::new(),
            baseline_ref: Some(BaselineRef::new("baseline_test")),
        },
        artifact_inputs: Vec::new(),
        evidence_updates: Vec::new(),
    }
}

struct CloseTaskFixture<'a> {
    request_id: &'a str,
    idempotency_key: Option<&'a str>,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: &'a str,
    intent: CloseIntent,
    close_reason: Option<CloseReason>,
    superseding_task_id: Option<&'a str>,
}

fn close_task_request(input: CloseTaskFixture<'_>) -> CloseTaskRequest {
    CloseTaskRequest {
        envelope: envelope(
            input.request_id,
            input.idempotency_key,
            input.dry_run,
            input.expected_state_version,
            Some(input.task_id),
        ),
        task_id: TaskId::new(input.task_id),
        intent: input.intent,
        close_reason: input.close_reason,
        superseding_task_id: input.superseding_task_id.map(TaskId::new),
        user_note: Some("Focused close-task test.".to_owned()),
    }
}

fn record_close_evidence(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
    supported: bool,
) -> Result<u64, Box<dyn Error>> {
    enable_record_run_capabilities(harness)?;
    let request_id = format!("req_close_evidence_{suffix}");
    let idempotency_key = format!("idem_close_evidence_{suffix}");
    let mut request = record_run_request(
        &request_id,
        &idempotency_key,
        false,
        Some(expected_state_version),
        task_id,
        change_unit_id,
    );
    request.evidence_updates = vec![if supported {
        supported_evidence_update("Close claim supported.")
    } else {
        unsupported_evidence_update("Close claim supported.")
    }];
    let response = harness
        .service
        .record_run(request, invocation(AccessClass::RunRecording))?;
    Ok(response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present"))
}

fn record_final_acceptance(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<u64, Box<dyn Error>> {
    let request_id = format!("req_close_final_{suffix}");
    let idempotency_key = format!("idem_close_final_{suffix}");
    let judgment = harness.service.request_user_judgment(
        user_judgment_request(
            &request_id,
            &idempotency_key,
            false,
            Some(expected_state_version),
            task_id,
            Some(change_unit_id),
            JudgmentKind::FinalAcceptance,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let judgment_id = judgment.response_value["user_judgment_ref"]["record_id"]
        .as_str()
        .expect("user judgment ref should be present")
        .to_owned();
    let record_request_id = format!("req_close_final_record_{suffix}");
    let record_idempotency_key = format!("idem_close_final_record_{suffix}");
    let response = harness.service.record_user_judgment(
        record_judgment_request(
            &record_request_id,
            &record_idempotency_key,
            Some(expected_state_version + 1),
            task_id,
            &judgment_id,
            JudgmentKind::FinalAcceptance,
            answer_payload(JudgmentKind::FinalAcceptance),
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    Ok(response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state_version should be present"))
}

fn prepare_write_authorization(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
    expected_state_version: u64,
    suffix: &str,
) -> Result<String, Box<dyn Error>> {
    let request_id = format!("req_prepare_{suffix}");
    let idempotency_key = format!("idem_prepare_{suffix}");
    let response = harness.service.prepare_write(
        prepare_write_request(
            &request_id,
            &idempotency_key,
            Some(expected_state_version),
            Some(task_id),
            Some(change_unit_id),
        ),
        invocation(AccessClass::WriteAuthorization),
    )?;
    assert_eq!(response.response_value["decision"], "allowed");
    Ok(
        response.response_value["write_authorization_ref"]["record_id"]
            .as_str()
            .expect("write authorization ref should be present")
            .to_owned(),
    )
}

fn stage_artifact_for_record_run(
    harness: &MethodHarness,
    task_id: &str,
    suffix: &str,
    expected_state_version: u64,
) -> Result<StagedArtifactHandle, Box<dyn Error>> {
    let request_id = format!("req_stage_{suffix}");
    let idempotency_key = format!("idem_stage_{suffix}");
    let mut request = stage_artifact_request(
        &request_id,
        Some(&idempotency_key),
        false,
        Some(expected_state_version),
        task_id,
    );
    request.display_name = format!("{suffix}.json");
    request.content_type = "application/json".to_owned();
    request.safe_bytes_or_notice = format!("{{\"fixture\":\"{suffix}\"}}");
    let response = harness
        .service
        .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;
    Ok(serde_json::from_value(
        response.response_value["staged_artifact_handle"].clone(),
    )?)
}

fn artifact_input_for_handle(
    artifact_input_id: &str,
    handle: StagedArtifactHandle,
    relation_hint: Option<&str>,
    claim: Option<&str>,
) -> ArtifactInput {
    ArtifactInput {
        artifact_input_id: harness_types::ArtifactInputId::new(artifact_input_id),
        source_kind: ArtifactInputSourceKind::StagedArtifact,
        staged_artifact_handle: Some(handle.clone()),
        existing_artifact_ref: None,
        relation_hint: relation_hint.map(str::to_owned),
        claim: claim.map(str::to_owned),
        expected_sha256: Some(handle.sha256),
        expected_size_bytes: Some(handle.size_bytes),
        redaction_state: Some(handle.redaction_state),
    }
}

fn supported_evidence_update(claim: &str) -> EvidenceCoverageItem {
    EvidenceCoverageItem {
        claim: claim.to_owned(),
        required_for_close: true,
        coverage_state: EvidenceCoverageState::Supported,
        supporting_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }
}

fn unsupported_evidence_update(claim: &str) -> EvidenceCoverageItem {
    EvidenceCoverageItem {
        claim: claim.to_owned(),
        required_for_close: true,
        coverage_state: EvidenceCoverageState::Unsupported,
        supporting_refs: Vec::new(),
        supporting_artifact_refs: Vec::new(),
        gap_refs: Vec::new(),
    }
}

fn enable_record_run_capabilities(harness: &MethodHarness) -> Result<(), Box<dyn Error>> {
    set_surface_capability(
        harness,
        &json!({
            "access_class": "run_recording",
            "supported_access_classes": [
                "write_authorization",
                "artifact_registration",
                "run_recording"
            ],
            "manual_artifact_attachment_supported": true
        })
        .to_string(),
    )
}

fn assert_close_blocker(response_value: &Value, code: &str) {
    let codes = close_blocker_codes(response_value);
    assert!(
        codes.iter().any(|candidate| candidate == code),
        "expected close blocker code {code}, got {codes:?}"
    );
}

fn assert_no_close_blocker(response_value: &Value, code: &str) {
    let codes = close_blocker_codes(response_value);
    assert!(
        codes.iter().all(|candidate| candidate != code),
        "did not expect close blocker code {code}, got {codes:?}"
    );
}

fn close_blocker_codes(response_value: &Value) -> Vec<String> {
    response_value["blockers"]
        .as_array()
        .expect("blockers should be an array")
        .iter()
        .filter_map(|blocker| blocker["code"].as_str().map(str::to_owned))
        .collect()
}

fn assert_prepare_reason(response_value: &Value, code: &str) {
    let reasons = response_value["write_decision_reasons"]
        .as_array()
        .expect("write_decision_reasons should be an array");
    assert!(
        reasons.iter().any(|reason| reason["code"] == code),
        "expected prepare_write reason code {code}, got {reasons:?}"
    );
}

fn create_task_with_change_unit(
    harness: &MethodHarness,
    prefix: &str,
) -> Result<(String, String), Box<dyn Error>> {
    let intake_request_id = format!("req_{prefix}_task");
    let intake_idempotency_key = format!("idem_{prefix}_task");
    let intake = harness.service.intake(
        intake_request(
            &intake_request_id,
            &intake_idempotency_key,
            false,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();

    let scope_request_id = format!("req_{prefix}_scope");
    let scope_idempotency_key = format!("idem_{prefix}_scope");
    let scope = harness.service.update_scope(
        update_scope_request(
            &scope_request_id,
            &scope_idempotency_key,
            false,
            Some(1),
            &task_id,
            ChangeUnitOperation::CreateCurrent,
            "Initial current scope.",
        ),
        invocation(AccessClass::CoreMutation),
    )?;
    let change_unit_id = scope.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .expect("change unit ref should be present")
        .to_owned();
    Ok((task_id, change_unit_id))
}

#[derive(Debug, PartialEq)]
struct TaskTerminalFields {
    lifecycle_phase: String,
    result: Option<String>,
    close_summary: Value,
    closed_at: Option<String>,
}

fn task_terminal_fields(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<TaskTerminalFields, Box<dyn Error>> {
    let conn = harness.conn()?;
    let (lifecycle_phase, result, close_summary_text, closed_at): (
        String,
        Option<String>,
        String,
        Option<String>,
    ) = conn.query_row(
        "SELECT lifecycle_phase, result, close_summary_json, closed_at
               FROM tasks
              WHERE project_id = ?1
                AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    Ok(TaskTerminalFields {
        lifecycle_phase,
        result,
        close_summary: serde_json::from_str(&close_summary_text)?,
        closed_at,
    })
}

fn insert_superseding_task(harness: &MethodHarness, task_id: &str) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.execute(
        "INSERT INTO tasks (
                project_id,
                task_id,
                created_by_surface_id,
                created_by_surface_instance_id,
                mode,
                lifecycle_phase,
                result,
                title,
                summary,
                shaping_summary_json,
                bounded_context_json,
                autonomy_boundary_json,
                close_summary_json,
                completion_policy_json,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                'work',
                'ready',
                'none',
                'Superseding task',
                'Superseding task',
                '{\"goal_summary\":\"Superseding task\"}',
                '{}',
                '{}',
                '{\"close_reason\":\"none\"}',
                '{}',
                't0',
                't0'
            )",
        rusqlite::params![PROJECT_ID, task_id, SURFACE_ID, SURFACE_INSTANCE_ID],
    )?;
    Ok(())
}

fn active_task_id(harness: &MethodHarness) -> Result<Option<String>, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT active_task_id
               FROM project_state
              WHERE project_id = ?1",
        rusqlite::params![PROJECT_ID],
        |row| row.get(0),
    )?)
}

#[derive(Debug, PartialEq)]
struct StagedArtifactRow {
    created_by_surface_id: String,
    created_by_surface_instance_id: String,
    status: String,
    redaction_state: String,
    tmp_path: String,
    ttl_hours: f64,
}

fn enable_stage_artifact_capability(harness: &MethodHarness) -> Result<(), Box<dyn Error>> {
    set_surface_capability(
        harness,
        &json!({
            "access_class": "artifact_registration",
            "supported_access_classes": ["artifact_registration"],
            "manual_artifact_attachment_supported": true
        })
        .to_string(),
    )
}

fn staged_artifact_row(
    harness: &MethodHarness,
    handle_id: &str,
) -> Result<StagedArtifactRow, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT
                created_by_surface_id,
                created_by_surface_instance_id,
                status,
                redaction_state,
                tmp_path,
                (julianday(expires_at) - julianday(created_at)) * 24.0
             FROM artifact_staging
             WHERE project_id = ?1
               AND handle_id = ?2",
        rusqlite::params![PROJECT_ID, handle_id],
        |row| {
            Ok(StagedArtifactRow {
                created_by_surface_id: row.get(0)?,
                created_by_surface_instance_id: row.get(1)?,
                status: row.get(2)?,
                redaction_state: row.get(3)?,
                tmp_path: row.get(4)?,
                ttl_hours: row.get(5)?,
            })
        },
    )?)
}

fn user_judgment_request(
    request_id: &str,
    idempotency_key: &str,
    dry_run: bool,
    expected_state_version: Option<u64>,
    task_id: &str,
    change_unit_id: Option<&str>,
    judgment_kind: JudgmentKind,
) -> harness_types::RequestUserJudgmentRequest {
    harness_types::RequestUserJudgmentRequest {
        envelope: envelope(
            request_id,
            Some(idempotency_key),
            dry_run,
            expected_state_version,
            Some(task_id),
        ),
        task_id: TaskId::new(task_id),
        change_unit_id: change_unit_id.map(ChangeUnitId::new),
        judgment_kind,
        presentation: harness_types::JudgmentPresentation::Short,
        question: "Choose the focused test judgment outcome.".to_owned(),
        options: vec![
            UserJudgmentOption {
                option_id: harness_types::UserJudgmentOptionId::new("accept"),
                label: "Accept".to_owned(),
                description: "Record the focused user-owned judgment.".to_owned(),
                consequence: "Only this judgment record is resolved.".to_owned(),
                is_default: true,
            },
            UserJudgmentOption {
                option_id: harness_types::UserJudgmentOptionId::new("decline"),
                label: "Decline".to_owned(),
                description: "Record that the focused judgment was not accepted.".to_owned(),
                consequence: "The Task remains unresolved for this question.".to_owned(),
                is_default: false,
            },
        ],
        context: UserJudgmentContext {
            summary: "A focused test judgment needs a user-owned answer.".to_owned(),
            related_refs: Vec::new(),
            artifact_refs: Vec::new(),
            visible_risks: Vec::new(),
            constraints: vec!["The answer covers only the requested judgment kind.".to_owned()],
        },
        affected_refs: vec![StateRecordRef {
            record_kind: StateRecordKind::Task,
            record_id: RecordId::new(task_id),
            project_id: ProjectId::new(PROJECT_ID),
            task_id: Some(TaskId::new(task_id)),
            state_version: expected_state_version,
        }],
        required_for: harness_types::JudgmentRequiredFor::Close,
        expires_at: None,
    }
}

fn record_judgment_request(
    request_id: &str,
    idempotency_key: &str,
    expected_state_version: Option<u64>,
    task_id: &str,
    user_judgment_id: &str,
    judgment_kind: JudgmentKind,
    answer: RecordUserJudgmentPayload,
) -> RecordUserJudgmentRequest {
    let mut request_envelope = envelope(
        request_id,
        Some(idempotency_key),
        false,
        expected_state_version,
        Some(task_id),
    );
    request_envelope.actor_kind = ActorKind::User;
    RecordUserJudgmentRequest {
        envelope: request_envelope,
        user_judgment_id: harness_types::UserJudgmentId::new(user_judgment_id),
        judgment_kind,
        selected_option_id: harness_types::UserJudgmentOptionId::new("accept"),
        answer,
        note: Some("Recorded by the focused judgment test.".to_owned()),
        accepted_risks: Vec::new(),
    }
}

fn answer_payload(judgment_kind: JudgmentKind) -> RecordUserJudgmentPayload {
    let mut payload = RecordUserJudgmentPayload {
        product_decision: None,
        technical_decision: None,
        scope_decision: None,
        sensitive_action_scope: None,
        final_acceptance: None,
        residual_risk_acceptance: None,
        cancellation: None,
    };
    match judgment_kind {
        JudgmentKind::ProductDecision => {
            payload.product_decision = Some(json_object(json!({
                "judgment": {
                    "decision": "accepted",
                    "rationale": "The product direction is accepted for this focused test."
                }
            })));
        }
        JudgmentKind::TechnicalDecision => {
            payload.technical_decision = Some(json_object(json!({
                "judgment": {
                    "decision": "accepted",
                    "rationale": "The technical direction is accepted for this focused test."
                }
            })));
        }
        JudgmentKind::ScopeDecision => {
            payload.scope_decision = Some(json_object(json!({
                "requested_scope_summary": "Expanded scope that must not apply silently.",
                "decision": "accepted"
            })));
        }
        JudgmentKind::SensitiveApproval => {
            payload.sensitive_action_scope = Some(harness_types::SensitiveActionScope {
                action_kind: "local_sensitive_step".to_owned(),
                description: "Allow the named sensitive step only.".to_owned(),
                intended_paths: vec!["src/export.rs".to_owned()],
                sensitive_categories: vec!["network".to_owned()],
                command_or_tool_summary: Some("Run a local diagnostic command.".to_owned()),
                network_or_host_summary: Some("No remote host is authorized here.".to_owned()),
                secret_or_credential_summary: None,
                capability_claim: "This is not Write Authorization.".to_owned(),
                expires_at: None,
            });
        }
        JudgmentKind::FinalAcceptance => {
            payload.final_acceptance = Some(json_object(json!({
                "judgment": {
                    "decision": "accepted",
                    "basis": "The visible close basis is acceptable."
                }
            })));
        }
        JudgmentKind::ResidualRiskAcceptance => {
            payload.residual_risk_acceptance = Some(json_object(json!({
                "risk_id": "risk_visible_001",
                "decision": "accepted"
            })));
        }
        JudgmentKind::Cancellation => {
            payload.cancellation = Some(json_object(json!({
                "decision": "cancel",
                "reason": "The user chose to stop the Task."
            })));
        }
    }
    payload
}

fn json_object(value: Value) -> JsonObject {
    match value {
        Value::Object(object) => object,
        _ => panic!("test helper expected a JSON object"),
    }
}

fn insert_active_write_authorization(
    harness: &MethodHarness,
    task_id: &str,
    change_unit_id: &str,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.execute(
        "INSERT INTO write_authorizations (
                project_id,
                write_authorization_id,
                task_id,
                change_unit_id,
                basis_state_version,
                status,
                attempt_scope_json,
                created_by_surface_id,
                created_by_surface_instance_id,
                expires_at,
                created_at
            )
            VALUES (
                ?1,
                'wa_replace',
                ?2,
                ?3,
                2,
                'active',
                '{\"intended_paths\":[\"src/export.rs\"]}',
                ?4,
                ?5,
                '2999-01-01T00:00:00Z',
                't0'
            )",
        rusqlite::params![
            PROJECT_ID,
            task_id,
            change_unit_id,
            SURFACE_ID,
            SURFACE_INSTANCE_ID
        ],
    )?;
    Ok(())
}

fn set_surface_capability(
    harness: &MethodHarness,
    capability_profile_json: &str,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.execute(
        "UPDATE surfaces
                SET capability_profile_json = ?3
              WHERE project_id = ?1
                AND surface_id = ?2",
        rusqlite::params![PROJECT_ID, SURFACE_ID, capability_profile_json],
    )?;
    Ok(())
}

fn set_surface_local_access(
    harness: &MethodHarness,
    local_access: Value,
) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.execute(
        "UPDATE surfaces
                SET local_access_json = ?3
              WHERE project_id = ?1
                AND surface_id = ?2",
        rusqlite::params![PROJECT_ID, SURFACE_ID, local_access.to_string()],
    )?;
    Ok(())
}

fn write_authorization_count(harness: &MethodHarness) -> Result<u64, Box<dyn Error>> {
    let conn = harness.conn()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
               FROM write_authorizations
              WHERE project_id = ?1",
        rusqlite::params![PROJECT_ID],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(count)?)
}

fn write_decision_event_count(harness: &MethodHarness) -> Result<u64, Box<dyn Error>> {
    let conn = harness.conn()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
               FROM task_events
              WHERE project_id = ?1
                AND event_kind = 'write_decision_recorded'",
        rusqlite::params![PROJECT_ID],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(count)?)
}

fn latest_task_event(harness: &MethodHarness) -> Result<(String, Value, u64), Box<dyn Error>> {
    let conn = harness.conn()?;
    let (event_kind, event_payload_text, state_version): (String, String, i64) = conn.query_row(
        "SELECT event_kind, event_payload_json, state_version
                   FROM task_events
                  WHERE project_id = ?1
                  ORDER BY event_seq DESC
                  LIMIT 1",
        rusqlite::params![PROJECT_ID],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    Ok((
        event_kind,
        serde_json::from_str(&event_payload_text)?,
        u64::try_from(state_version)?,
    ))
}

fn assert_latest_prepare_write_event(
    harness: &MethodHarness,
    response_value: &Value,
    expected_decision: &str,
    expected_reason_code: &str,
) -> Result<Value, Box<dyn Error>> {
    let (event_kind, payload, event_state_version) = latest_task_event(harness)?;
    assert_eq!(event_kind, "write_decision_recorded");
    assert_eq!(event_state_version, response_value["base"]["state_version"]);
    assert_eq!(payload["decision"], expected_decision);
    assert!(payload["write_authorization_id"].is_null());
    assert!(payload.get("reason_codes").is_none());
    assert!(payload.get("intended_paths").is_none());
    assert!(payload.get("intended_operation").is_none());
    assert!(payload.get("sensitive_categories").is_none());
    assert!(payload.get("baseline_ref").is_none());
    assert_eq!(
        payload["write_decision_reasons"],
        response_value["write_decision_reasons"]
    );
    assert_prepare_reason(&payload, expected_reason_code);
    Ok(payload)
}

fn write_authorization_basis(
    harness: &MethodHarness,
    write_authorization_id: &str,
) -> Result<u64, Box<dyn Error>> {
    let conn = harness.conn()?;
    let basis: i64 = conn.query_row(
        "SELECT basis_state_version
               FROM write_authorizations
              WHERE project_id = ?1
                AND write_authorization_id = ?2",
        rusqlite::params![PROJECT_ID, write_authorization_id],
        |row| row.get(0),
    )?;
    Ok(u64::try_from(basis)?)
}

fn user_judgment_status(
    harness: &MethodHarness,
    user_judgment_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT status
               FROM user_judgments
              WHERE project_id = ?1
                AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, user_judgment_id],
        |row| row.get(0),
    )?)
}

fn resolution_json(
    harness: &MethodHarness,
    user_judgment_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT resolution_json
               FROM user_judgments
              WHERE project_id = ?1
                AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, user_judgment_id],
        |row| row.get(0),
    )?;
    Ok(serde_json::from_str(&text)?)
}

fn current_change_unit_id(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT current_change_unit_id
               FROM tasks
              WHERE project_id = ?1
                AND task_id = ?2",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?)
}

fn current_change_unit_scope(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    let text: String = conn.query_row(
        "SELECT scope_summary_json
               FROM change_units
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'
                AND is_current = 1",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?;
    let value: Value = serde_json::from_str(&text)?;
    Ok(value["scope_summary"]
        .as_str()
        .expect("scope_summary should be a string")
        .to_owned())
}

fn set_task_owner_json(
    harness: &MethodHarness,
    task_id: &str,
    logical_column: &str,
    value: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let sql = match logical_column {
        "shaping_summary_json" => {
            "UPDATE tasks
                SET shaping_summary_json = ?3
              WHERE project_id = ?1
                AND task_id = ?2"
        }
        "autonomy_boundary_json" => {
            "UPDATE tasks
                SET autonomy_boundary_json = ?3
              WHERE project_id = ?1
                AND task_id = ?2"
        }
        "close_summary_json" => {
            "UPDATE tasks
                SET close_summary_json = ?3
              WHERE project_id = ?1
                AND task_id = ?2"
        }
        "completion_policy_json" => {
            "UPDATE tasks
                SET completion_policy_json = ?3
              WHERE project_id = ?1
                AND task_id = ?2"
        }
        _ => panic!("unsupported task owner JSON column {logical_column}"),
    };
    harness
        .conn()?
        .execute(sql, rusqlite::params![PROJECT_ID, task_id, value])?;
    Ok(())
}

fn set_change_unit_owner_json(
    harness: &MethodHarness,
    change_unit_id: &str,
    logical_column: &str,
    value: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let sql = match logical_column {
        "scope_summary_json" => {
            "UPDATE change_units
                SET scope_summary_json = ?3
              WHERE project_id = ?1
                AND change_unit_id = ?2"
        }
        "bounded_paths_json" => {
            "UPDATE change_units
                SET bounded_paths_json = ?3
              WHERE project_id = ?1
                AND change_unit_id = ?2"
        }
        "write_basis_json" => {
            "UPDATE change_units
                SET write_basis_json = ?3
              WHERE project_id = ?1
                AND change_unit_id = ?2"
        }
        "close_basis_json" => {
            "UPDATE change_units
                SET close_basis_json = ?3
              WHERE project_id = ?1
                AND change_unit_id = ?2"
        }
        "lifecycle_json" => {
            "UPDATE change_units
                SET lifecycle_json = ?3
              WHERE project_id = ?1
                AND change_unit_id = ?2"
        }
        _ => panic!("unsupported change-unit owner JSON column {logical_column}"),
    };
    harness
        .conn()?
        .execute(sql, rusqlite::params![PROJECT_ID, change_unit_id, value])?;
    Ok(())
}

fn set_user_judgment_resolution_json(
    harness: &MethodHarness,
    judgment_id: &str,
    value: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    harness.conn()?.execute(
        "UPDATE user_judgments
            SET status = 'resolved',
                resolution_json = ?3,
                resolved_at = 't1'
          WHERE project_id = ?1
            AND judgment_id = ?2",
        rusqlite::params![PROJECT_ID, judgment_id, value],
    )?;
    Ok(())
}

fn active_current_change_units(
    harness: &MethodHarness,
    task_id: &str,
) -> Result<i64, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT COUNT(*)
               FROM change_units
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'
                AND is_current = 1",
        rusqlite::params![PROJECT_ID, task_id],
        |row| row.get(0),
    )?)
}

fn write_authorization_status(
    harness: &MethodHarness,
    write_authorization_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT status
               FROM write_authorizations
              WHERE project_id = ?1
                AND write_authorization_id = ?2",
        rusqlite::params![PROJECT_ID, write_authorization_id],
        |row| row.get(0),
    )?)
}

fn artifact_staging_status(
    harness: &MethodHarness,
    handle_id: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = harness.conn()?;
    Ok(conn.query_row(
        "SELECT status
               FROM artifact_staging
              WHERE project_id = ?1
                AND handle_id = ?2",
        rusqlite::params![PROJECT_ID, handle_id],
        |row| row.get(0),
    )?)
}

fn expire_staged_artifact(harness: &MethodHarness, handle_id: &str) -> Result<(), Box<dyn Error>> {
    let conn = harness.conn()?;
    conn.execute(
        "UPDATE artifact_staging
                SET expires_at = '2000-01-01T00:00:00.000Z'
              WHERE project_id = ?1
                AND handle_id = ?2",
        rusqlite::params![PROJECT_ID, handle_id],
    )?;
    Ok(())
}

fn artifact_owner_link_exists(
    harness: &MethodHarness,
    artifact_id: &str,
    owner_record_kind: &str,
) -> Result<bool, Box<dyn Error>> {
    let conn = harness.conn()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
               FROM artifact_links
              WHERE project_id = ?1
                AND artifact_id = ?2
                AND owner_record_kind = ?3",
        rusqlite::params![PROJECT_ID, artifact_id, owner_record_kind],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}
