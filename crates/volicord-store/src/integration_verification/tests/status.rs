use std::error::Error;

use volicord_types::{
    GuardIntegrationVerificationPhaseStatus, GuardIntegrationVerificationPhases,
    IntegrationVerificationRestartReason, IntegrationVerificationWorkflowState,
};

use super::support::*;
use crate::integration_verification::{
    get_guard_integration_verification, status::begin_result_from_record,
};

#[test]
fn active_status_distinguishes_probe_and_hook_waiting() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-status-active")?;
    let run = fixture.begin()?;
    let awaiting_probe = get_guard_integration_verification(
        fixture.runtime_home.path(),
        &run.verification_id,
        &fixture.caller(),
        BEGIN_AT,
    )?;
    assert!(matches!(
        awaiting_probe.workflow,
        IntegrationVerificationWorkflowState::AwaitingProbe { .. }
    ));
    assert_eq!(
        begin_result_from_record(fixture.runtime_home.path(), &run, BEGIN_AT)?.workflow,
        awaiting_probe.workflow
    );
    let probe = fixture.acknowledge(&run.verification_id, ACK_AT)?;
    let awaiting_hooks = get_guard_integration_verification(
        fixture.runtime_home.path(),
        &run.verification_id,
        &fixture.caller(),
        ACK_AT,
    )?;
    assert!(matches!(
        awaiting_hooks.workflow,
        IntegrationVerificationWorkflowState::AwaitingHookCompletion { .. }
    ));
    assert_eq!(probe.workflow, awaiting_hooks.workflow);
    assert_eq!(
        begin_result_from_record(
            fixture.runtime_home.path(),
            &fixture.record(&run.verification_id)?,
            ACK_AT,
        )?
        .workflow,
        awaiting_hooks.workflow
    );
    Ok(())
}

#[test]
fn passed_failed_and_expired_status_project_exact_workflow() -> Result<(), Box<dyn Error>> {
    let passed_fixture = VerificationFixture::new("guard-integration-status-passed")?;
    let passed = passed_fixture.begin()?;
    let completed = passed_fixture.complete(&passed.verification_id)?;
    let result = get_guard_integration_verification(
        passed_fixture.runtime_home.path(),
        &passed.verification_id,
        &passed_fixture.caller(),
        "2026-07-23T00:00:05Z",
    )?;
    assert!(matches!(
        result.workflow,
        IntegrationVerificationWorkflowState::Complete { .. }
    ));
    assert_eq!(
        result.guard_phases,
        GuardIntegrationVerificationPhases {
            prompt_capture: GuardIntegrationVerificationPhaseStatus::Matched,
            pre_tool: GuardIntegrationVerificationPhaseStatus::Matched,
            post_tool: GuardIntegrationVerificationPhaseStatus::Matched,
        }
    );
    assert_eq!(
        result
            .matched_prompt_event_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("guard_event_prompt")
    );
    assert_eq!(
        result
            .matched_pre_tool_event_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("guard_event_pre")
    );
    assert_eq!(
        result
            .matched_post_tool_event_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("guard_event_post")
    );
    let begin = begin_result_from_record(
        passed_fixture.runtime_home.path(),
        &completed,
        "2026-07-23T00:00:05Z",
    )?;
    assert_eq!(begin.workflow, result.workflow);

    passed_fixture.set_policy_hash(&passed.verification_id, STALE_HASH)?;
    let failed = get_guard_integration_verification(
        passed_fixture.runtime_home.path(),
        &passed.verification_id,
        &passed_fixture.caller(),
        "2026-07-23T00:00:06Z",
    )?;
    assert!(matches!(
        failed.workflow,
        IntegrationVerificationWorkflowState::RestartRequired {
            reason: IntegrationVerificationRestartReason::Failed,
            ..
        }
    ));

    let owner_fixture = VerificationFixture::new("guard-integration-status-owner-facts")?;
    let owner_run = owner_fixture.begin()?;
    for owner_fact in ["policy", "hook_digest", "revision"] {
        match owner_fact {
            "policy" => owner_fixture.set_policy_hash(&owner_run.verification_id, STALE_HASH)?,
            "hook_digest" => {
                owner_fixture.set_hook_contract_digest(&owner_run.verification_id, STALE_HASH)?
            }
            "revision" => {
                owner_fixture.set_integration_revision(&owner_run.verification_id, STALE_HASH)?
            }
            _ => unreachable!(),
        }
        let failed = get_guard_integration_verification(
            owner_fixture.runtime_home.path(),
            &owner_run.verification_id,
            &owner_fixture.caller(),
            "2026-07-23T00:00:04Z",
        )?;
        assert!(
            matches!(
                failed.workflow,
                IntegrationVerificationWorkflowState::RestartRequired {
                    reason: IntegrationVerificationRestartReason::Failed,
                    ..
                }
            ),
            "{owner_fact} drift must fail",
        );
        match owner_fact {
            "policy" => owner_fixture.set_policy_hash(&owner_run.verification_id, POLICY_HASH)?,
            "hook_digest" => owner_fixture.set_hook_contract_digest(
                &owner_run.verification_id,
                &owner_run.hook_contract_digest,
            )?,
            "revision" => owner_fixture.set_integration_revision(
                &owner_run.verification_id,
                &owner_run.integration_revision,
            )?,
            _ => unreachable!(),
        }
    }

    let expired_fixture = VerificationFixture::new("guard-integration-status-expired")?;
    let expired = expired_fixture.begin()?;
    let expired_result = get_guard_integration_verification(
        expired_fixture.runtime_home.path(),
        &expired.verification_id,
        &expired_fixture.caller(),
        "2026-07-23T00:05:03Z",
    )?;
    assert!(matches!(
        expired_result.workflow,
        IntegrationVerificationWorkflowState::RestartRequired {
            reason: IntegrationVerificationRestartReason::Expired,
            ..
        }
    ));
    Ok(())
}
