use std::error::Error;

use volicord_types::integration_verification::{
    GuardVerificationRepairReason, GuardVerificationRetryPolicy,
    IntegrationVerificationWorkflowState,
};

use super::support::*;
use crate::{
    integration_verification::{
        acknowledge_guard_integration_probe, GuardIntegrationVerificationCaller,
    },
    StoreError,
};

#[test]
fn first_acknowledgement_and_active_replay_preserve_timestamp() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-probe-replay")?;
    let run = fixture.begin()?;
    let first = fixture.acknowledge(&run.verification_id, ACK_AT)?;
    let replay = fixture.acknowledge(&run.verification_id, "2026-07-23T00:00:04.100Z")?;
    assert_eq!(first.workflow, replay.workflow);
    assert!(matches!(
        first.workflow,
        IntegrationVerificationWorkflowState::AwaitingObservation { .. }
    ));
    Ok(())
}

#[test]
fn complete_and_repair_replay_keep_terminal_meaning() -> Result<(), Box<dyn Error>> {
    let passed_fixture = VerificationFixture::new("guard-integration-probe-passed")?;
    let passed = passed_fixture.begin()?;
    passed_fixture.complete(&passed.verification_id)?;
    assert!(matches!(
        passed_fixture
            .acknowledge(&passed.verification_id, "2026-07-23T00:00:05Z")?
            .workflow,
        IntegrationVerificationWorkflowState::Complete { .. }
    ));

    let failed_fixture = VerificationFixture::new("guard-integration-probe-repair")?;
    let failed = failed_fixture.begin()?;
    failed_fixture.acknowledge(&failed.verification_id, ACK_AT)?;
    failed_fixture.force_repair(
        &failed.verification_id,
        GuardVerificationRepairReason::PolicyChanged,
        GuardVerificationRetryPolicy::RepairRequired,
    )?;
    assert!(matches!(
        failed_fixture
            .acknowledge(&failed.verification_id, "2026-07-23T00:00:05Z")?
            .workflow,
        IntegrationVerificationWorkflowState::RepairRequired {
            reason: GuardVerificationRepairReason::PolicyChanged,
            ..
        }
    ));
    Ok(())
}

#[test]
fn terminal_run_without_acknowledgement_cannot_acquire_one() -> Result<(), Box<dyn Error>> {
    let failed_fixture = VerificationFixture::new("guard-integration-probe-late-repair")?;
    let failed = failed_fixture.begin()?;
    failed_fixture.force_repair(
        &failed.verification_id,
        GuardVerificationRepairReason::HookEventNotObserved,
        GuardVerificationRetryPolicy::HostReloadRequired,
    )?;
    let error = failed_fixture
        .acknowledge(&failed.verification_id, ACK_AT)
        .expect_err("terminal repair run cannot be acknowledged");
    assert!(matches!(error, StoreError::Conflict { .. }));
    assert!(failed_fixture
        .record(&failed.verification_id)?
        .probe_acknowledged_at
        .is_none());
    Ok(())
}

#[test]
fn wrong_caller_cannot_observe_or_change_acknowledgement() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-probe-wrong-caller")?;
    let run = fixture.begin()?;
    let acknowledged = fixture.acknowledge(&run.verification_id, ACK_AT)?;
    for caller in [
        GuardIntegrationVerificationCaller {
            runtime_session_id: "runtime_other".to_owned(),
            ..fixture.caller()
        },
        GuardIntegrationVerificationCaller {
            host_session_id: "host_session_other".to_owned(),
            ..fixture.caller()
        },
        GuardIntegrationVerificationCaller {
            host_turn_id: "host_turn_other".to_owned(),
            ..fixture.caller()
        },
    ] {
        acknowledge_guard_integration_probe(
            &fixture.context()?,
            &run.verification_id,
            &caller,
            "2026-07-23T00:00:05Z",
        )
        .expect_err("different caller coordinate must fail");
    }
    let IntegrationVerificationWorkflowState::AwaitingObservation {
        acknowledged_at, ..
    } = acknowledged.workflow
    else {
        panic!("probe must be acknowledged");
    };
    assert_eq!(
        fixture
            .record(&run.verification_id)?
            .probe_acknowledged_at
            .as_deref(),
        Some(acknowledged_at.to_canonical_string().as_str())
    );
    Ok(())
}
