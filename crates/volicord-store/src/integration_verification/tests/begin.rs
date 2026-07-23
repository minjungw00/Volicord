use std::error::Error;

use volicord_types::{McpRuntimeSessionSource, SequenceDurableIdGenerator};

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
    assert_eq!(first.status, "active");
    assert!(first.verification_id.starts_with("guard_verification_"));
    Ok(())
}

#[test]
fn current_coordinate_conflict_rejects_resume() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-begin-conflict")?;
    let run = fixture.begin()?;
    fixture.set_policy_hash(&run.verification_id, STALE_HASH)?;
    let error = fixture
        .begin_at("2026-07-23T00:00:04Z", [])
        .expect_err("stored current facts must match");
    assert!(matches!(error, StoreError::Conflict { .. }));
    Ok(())
}

#[test]
fn expired_run_is_replaced_and_passed_run_is_resumed() -> Result<(), Box<dyn Error>> {
    let expired_fixture = VerificationFixture::new("guard-integration-begin-expired")?;
    let expired = expired_fixture.begin()?;
    let replacement = expired_fixture.begin_at("2026-07-23T00:05:03Z", ["two"])?;
    assert_ne!(expired.verification_id, replacement.verification_id);
    assert_eq!(replacement.status, "active");
    assert_eq!(
        expired_fixture.record(&expired.verification_id)?.status,
        "expired"
    );

    let passed_fixture = VerificationFixture::new("guard-integration-begin-passed")?;
    let passed = passed_fixture.begin()?;
    passed_fixture.complete(&passed.verification_id)?;
    let resumed = passed_fixture.begin_at("2026-07-23T00:00:06Z", [])?;
    assert_eq!(resumed.verification_id, passed.verification_id);
    assert_eq!(resumed.status, "passed");
    Ok(())
}

#[test]
fn failed_new_id_generation_rolls_back_expiry() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-begin-atomic")?;
    let run = fixture.begin()?;
    begin_guard_integration_verification_with_generator(
        fixture.runtime_home.path(),
        BeginGuardIntegrationVerificationInput {
            caller: fixture.caller(),
            project_id: PROJECT_ID.to_owned(),
            project_session_id: fixture.project_session_id.clone(),
            observed_at: "2026-07-23T00:05:03Z".to_owned(),
        },
        &SequenceDurableIdGenerator::new(Vec::<String>::new()),
    )
    .expect_err("missing generated ID must fail");
    assert_eq!(fixture.record(&run.verification_id)?.status, "active");
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
            fixture.runtime_home.path(),
            McpRuntimeSessionStart {
                connection_internal_id: CONNECTION_ID.to_owned(),
                session_source: source,
                observed_host_executable_version: None,
                process_id: 100 + index as u32,
                process_started_at: "2026-07-23T00:00:10Z".to_owned(),
            },
        )?;
        let error = begin_guard_integration_verification_with_generator(
            fixture.runtime_home.path(),
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
