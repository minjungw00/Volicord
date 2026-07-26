use std::{
    error::Error,
    sync::{Arc, Barrier},
    thread,
};

use volicord_types::ids::SequenceDurableIdGenerator;
use volicord_types::integration_verification::IntegrationVerificationWorkflowState;

use super::support::*;
use crate::integration_verification::{
    acknowledge_guard_integration_probe, begin_guard_integration_verification_with_generator,
    BeginGuardIntegrationVerificationInput,
};
use crate::mutation::TestRuntimeHomeAdmission;

#[test]
fn concurrent_same_coordinate_begins_converge_on_one_attempt() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-concurrent-begin")?;
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for generated in ["first", "second"] {
        let barrier = Arc::clone(&barrier);
        let runtime_home = fixture.runtime_home.path().to_path_buf();
        let caller = fixture.caller();
        let project_session_id = fixture.project_session_id.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            let mutation = TestRuntimeHomeAdmission::shared(&runtime_home)?;
            let context = mutation.context()?;
            begin_guard_integration_verification_with_generator(
                &context,
                BeginGuardIntegrationVerificationInput {
                    caller,
                    project_id: PROJECT_ID.to_owned(),
                    project_session_id,
                    observed_at: BEGIN_AT.to_owned(),
                },
                &SequenceDurableIdGenerator::new([generated]),
            )
        }));
    }
    let first = handles
        .remove(0)
        .join()
        .map_err(|_| "first begin thread panicked")??;
    let second = handles
        .remove(0)
        .join()
        .map_err(|_| "second begin thread panicked")??;
    assert_eq!(first.verification_id, second.verification_id);
    Ok(())
}

#[test]
fn concurrent_first_probe_calls_converge_on_one_timestamp() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-concurrent-probe")?;
    let run = fixture.begin()?;
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for observed_at in [ACK_AT, "2026-07-23T00:00:04.100Z"] {
        let barrier = Arc::clone(&barrier);
        let runtime_home = fixture.runtime_home.path().to_path_buf();
        let verification_id = run.verification_id.clone();
        let caller = fixture.caller();
        handles.push(thread::spawn(move || {
            barrier.wait();
            let mutation = TestRuntimeHomeAdmission::shared(&runtime_home)?;
            let context = mutation.context()?;
            acknowledge_guard_integration_probe(&context, &verification_id, &caller, observed_at)
        }));
    }
    let first = handles
        .remove(0)
        .join()
        .map_err(|_| "first probe thread panicked")??;
    let second = handles
        .remove(0)
        .join()
        .map_err(|_| "second probe thread panicked")??;
    assert_eq!(first.workflow, second.workflow);
    let IntegrationVerificationWorkflowState::AwaitingObservation {
        acknowledged_at, ..
    } = first.workflow
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
