use std::error::Error;

use volicord_types::ids::SequenceDurableIdGenerator;
use volicord_types::integration_revision::McpRuntimeSessionSource;
use volicord_types::integration_verification::{
    GuardVerificationRepairReason, GuardVerificationRetryPolicy,
};

use super::support::*;
use crate::{
    integration_verification::{
        begin_guard_integration_verification_with_generator,
        BeginGuardIntegrationVerificationInput, GuardIntegrationVerificationCaller,
    },
    operational_sessions::{start_mcp_runtime_session_for_test, McpRuntimeSessionStart},
    StoreError,
};

#[test]
fn first_begin_and_same_coordinate_resume_use_one_id() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-begin-resume")?;
    let first = fixture.begin()?;
    let resumed = fixture.begin_at(BEGIN_AT, [])?;
    assert_eq!(first, resumed);
    assert_eq!(first.status, "awaiting_probe");
    assert!(first.verification_id.starts_with("guard_verification_"));
    Ok(())
}

#[test]
fn immutable_coordinate_columns_reject_update() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-coordinate-immutable")?;
    let run = fixture.begin()?;
    let conn = crate::sqlite::open_registry_database_for_test(crate::sqlite::registry_db_path(
        fixture.runtime_home.path(),
    ))?;
    let error = conn
        .execute(
            "UPDATE guard_integration_verification_runs
                SET policy_digest = ?2
              WHERE verification_id = ?1",
            rusqlite::params![run.verification_id, STALE_HASH],
        )
        .expect_err("coordinate update must be rejected");
    assert!(error
        .to_string()
        .contains("guard integration verification coordinate is immutable"));
    Ok(())
}

#[test]
fn terminal_same_turn_begin_never_creates_a_new_id() -> Result<(), Box<dyn Error>> {
    let complete_fixture = VerificationFixture::new("guard-integration-begin-complete")?;
    let complete = complete_fixture.begin()?;
    complete_fixture.complete(&complete.verification_id)?;
    let resumed = complete_fixture.begin_at("2026-07-23T00:10:06Z", [])?;
    assert_eq!(resumed.verification_id, complete.verification_id);
    assert_eq!(resumed.status, "complete");

    let repair_fixture = VerificationFixture::new("guard-integration-begin-repair")?;
    let repair = repair_fixture.begin()?;
    repair_fixture.force_repair(
        &repair.verification_id,
        GuardVerificationRepairReason::HookEventNotObserved,
        GuardVerificationRetryPolicy::NewTurnRequired,
    )?;
    let resumed = repair_fixture.begin_at("2026-07-23T00:10:06Z", [])?;
    assert_eq!(resumed.verification_id, repair.verification_id);
    assert_eq!(resumed.status, "repair_required");
    Ok(())
}

#[test]
fn prompt_event_cannot_be_shared_across_attempts() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-prompt-owner")?;
    fixture.begin()?;
    let conn = crate::sqlite::open_registry_database_for_test(crate::sqlite::registry_db_path(
        fixture.runtime_home.path(),
    ))?;
    let error = conn
        .execute(
            "INSERT INTO guard_integration_verification_runs
             SELECT 'guard_verification_other', connection_internal_id,
                    project_internal_id, project_id, runtime_session_id,
                    host_session_id, 'host_turn_other', integration_revision,
                    guard_installation_id, host_contract_profile,
                    hook_definition_digest, policy_digest, expected_probe_tool,
                    expected_host_callable_name, observation_policy_kind,
                    observation_deadline_at, allowed_status_reads,
                    status_read_count, created_at, cleanup_after, status,
                    probe_acknowledged_at, completed_at, matched_prompt_event_id,
                    matched_pre_tool_event_id, matched_post_tool_event_id,
                    repair_reason, retry_policy, terminal_finding_code,
                    terminal_finding_summary
               FROM guard_integration_verification_runs",
            [],
        )
        .expect_err("one prompt event cannot own two attempts");
    assert!(error.to_string().contains(
        "guard_integration_verification_runs.project_internal_id, \
         guard_integration_verification_runs.matched_prompt_event_id"
    ));
    Ok(())
}

#[test]
fn new_turn_creates_a_new_coordinate_and_attempt() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-begin-new-turn")?;
    let first = fixture.begin()?;
    let second = fixture.begin_new_turn(
        "host_turn_second",
        "guard_event_prompt_second",
        "2026-07-23T00:01:00Z",
        ["two"],
    )?;
    assert_ne!(first.verification_id, second.verification_id);
    assert_eq!(second.host_turn_id, "host_turn_second");
    assert_eq!(
        second.matched_prompt_event_id.as_deref(),
        Some("guard_event_prompt_second")
    );
    assert_eq!(
        fixture.record(&first.verification_id)?.status,
        "repair_required"
    );
    Ok(())
}

#[test]
fn retry_policy_blocks_an_ineligible_new_coordinate() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-retry-policy")?;
    let first = fixture.begin()?;
    fixture.force_repair(
        &first.verification_id,
        GuardVerificationRepairReason::CallableIdentityMismatch,
        GuardVerificationRetryPolicy::NoAutomaticRetry,
    )?;
    let error = fixture
        .begin_new_turn(
            "host_turn_second",
            "guard_event_prompt_second",
            "2026-07-23T00:01:00Z",
            ["two"],
        )
        .expect_err("NoAutomaticRetry must reject a new attempt");
    assert!(matches!(error, StoreError::Conflict { .. }));
    assert_eq!(
        fixture.record(&first.verification_id)?.status,
        "repair_required"
    );
    Ok(())
}

#[test]
fn manual_and_preflight_runtime_sources_cannot_begin() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-begin-runtime-source")?;
    for (index, source) in [
        McpRuntimeSessionSource::ManualCli,
        McpRuntimeSessionSource::CliPreflight,
    ]
    .into_iter()
    .enumerate()
    {
        let runtime = start_mcp_runtime_session_for_test(
            &fixture.context()?,
            McpRuntimeSessionStart {
                connection_internal_id: CONNECTION_ID.to_owned(),
                session_source: source,
                observed_host_executable_version: None,
                process_id: 100 + index as u32,
                process_started_at: "2026-07-23T00:00:10Z".to_owned(),
            },
        )?;
        let error = begin_guard_integration_verification_with_generator(
            &fixture.context()?,
            BeginGuardIntegrationVerificationInput {
                caller: GuardIntegrationVerificationCaller {
                    runtime_session_id: runtime.runtime_session_id,
                    ..fixture.caller()
                },
                project_id: PROJECT_ID.to_owned(),
                project_session_id: fixture.project_session_id.clone(),
                observed_at: "2026-07-23T00:00:11Z".to_owned(),
            },
            &SequenceDurableIdGenerator::new(["rejected"]),
        )
        .expect_err("non-managed runtime source must fail");
        assert!(matches!(error, StoreError::Conflict { .. }));
    }
    Ok(())
}
