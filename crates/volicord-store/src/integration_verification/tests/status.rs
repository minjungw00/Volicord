use std::error::Error;

use volicord_types::integration_revision::IntegrationRevision;
use volicord_types::integration_verification::{
    GuardIntegrationVerificationPhaseStatus, GuardIntegrationVerificationPhases,
    GuardVerificationRepairReason, GuardVerificationRetryPolicy,
    IntegrationVerificationWorkflowState,
};

use super::support::*;
use crate::integration_verification::{
    current_guard_integration_verification_workflow, get_guard_integration_verification,
    latest_completed_guard_integration_verification_for_connection,
    latest_completed_guard_integration_verification_for_membership,
    latest_guard_integration_verification_for_connection,
    latest_guard_integration_verification_for_membership, status::begin_result_from_record,
};
use crate::{sqlite::registry_db_path, StoreError};

#[test]
fn nonterminal_projection_distinguishes_probe_and_observation() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-status-active")?;
    let run = fixture.begin()?;
    let awaiting_probe = get_guard_integration_verification(
        &fixture.context()?,
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
    assert!(matches!(
        probe.workflow,
        IntegrationVerificationWorkflowState::AwaitingObservation {
            remaining_status_reads: 1,
            ..
        }
    ));
    assert_eq!(
        begin_result_from_record(
            fixture.runtime_home.path(),
            &fixture.record(&run.verification_id)?,
            ACK_AT,
        )?
        .workflow,
        probe.workflow
    );
    Ok(())
}

#[test]
fn complete_projects_exact_workflow_and_remains_terminal() -> Result<(), Box<dyn Error>> {
    let passed_fixture = VerificationFixture::new("guard-integration-status-passed")?;
    let passed = passed_fixture.begin()?;
    let completed = passed_fixture.complete(&passed.verification_id)?;
    let result = get_guard_integration_verification(
        &passed_fixture.context()?,
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
    let replay = get_guard_integration_verification(
        &passed_fixture.context()?,
        &passed.verification_id,
        &passed_fixture.caller(),
        "2026-07-23T00:00:06Z",
    )?;
    assert!(matches!(
        replay.workflow,
        IntegrationVerificationWorkflowState::Complete { .. }
    ));
    assert_eq!(
        passed_fixture.record(&passed.verification_id)?.status,
        "complete"
    );
    Ok(())
}

#[test]
fn newest_attempt_and_newest_completed_proof_are_selected_independently(
) -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-status-proof-history")?;
    let completed = fixture.begin()?;
    fixture.complete(&completed.verification_id)?;
    let newer = fixture.begin_new_turn(
        "host_turn_newer",
        "guard_event_prompt_newer",
        "2026-07-23T00:01:00Z",
        ["newer"],
    )?;
    fixture.force_repair(
        &newer.verification_id,
        GuardVerificationRepairReason::CallableIdentityMismatch,
        GuardVerificationRetryPolicy::NewTurnRequired,
    )?;

    let latest = latest_guard_integration_verification_for_connection(
        fixture.runtime_home.path(),
        CONNECTION_ID,
        &IntegrationRevision::parse(fixture.integration_revision.clone())?,
    )?
    .expect("latest attempt");
    let proof = latest_completed_guard_integration_verification_for_connection(
        fixture.runtime_home.path(),
        CONNECTION_ID,
        &IntegrationRevision::parse(fixture.integration_revision.clone())?,
    )?
    .expect("latest completed proof");

    assert_eq!(latest.verification_id, newer.verification_id);
    assert_eq!(latest.status, "repair_required");
    assert_eq!(proof.verification_id, completed.verification_id);
    assert_eq!(proof.status, "complete");
    Ok(())
}

#[test]
fn membership_queries_never_substitute_another_projects_guard_run() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-status-membership")?;
    let run = fixture.begin()?;
    let completed = fixture.complete(&run.verification_id)?;
    let revision = IntegrationRevision::parse(fixture.integration_revision.clone())?;

    let latest = latest_guard_integration_verification_for_membership(
        fixture.runtime_home.path(),
        CONNECTION_ID,
        &completed.project_internal_id,
        &revision,
    )?
    .expect("selected membership attempt");
    let proof = latest_completed_guard_integration_verification_for_membership(
        fixture.runtime_home.path(),
        CONNECTION_ID,
        &completed.project_internal_id,
        &revision,
    )?
    .expect("selected membership proof");
    assert_eq!(latest.verification_id, completed.verification_id);
    assert_eq!(proof.verification_id, completed.verification_id);

    assert!(latest_guard_integration_verification_for_membership(
        fixture.runtime_home.path(),
        CONNECTION_ID,
        "project_internal_unrelated",
        &revision,
    )?
    .is_none());
    assert!(
        latest_completed_guard_integration_verification_for_membership(
            fixture.runtime_home.path(),
            CONNECTION_ID,
            "project_internal_unrelated",
            &revision,
        )?
        .is_none()
    );
    Ok(())
}

#[test]
fn completed_run_missing_its_completion_timestamp_is_corrupt_data() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-status-missing-completion")?;
    let run = fixture.begin()?;
    let conn = crate::sqlite::open_registry_database_for_test(registry_db_path(
        fixture.runtime_home.path(),
    ))?;
    conn.execute_batch("PRAGMA ignore_check_constraints = ON")?;
    conn.execute(
        "UPDATE guard_integration_verification_runs
            SET status = 'complete', probe_acknowledged_at = ?2,
                completed_at = NULL, matched_pre_tool_event_id = 'guard_event_pre',
                matched_post_tool_event_id = 'guard_event_post'
          WHERE verification_id = ?1",
        rusqlite::params![run.verification_id, ACK_AT],
    )?;
    drop(conn);

    let corrupt = fixture.record(&run.verification_id)?;
    assert!(matches!(
        current_guard_integration_verification_workflow(&corrupt),
        Err(StoreError::CorruptStoredValue {
            field: "guard_integration_verification_runs.completed_at",
            ..
        })
    ));
    Ok(())
}

#[test]
fn current_owner_drift_maps_to_distinct_typed_repairs() -> Result<(), Box<dyn Error>> {
    for (owner_fact, expected_reason, expected_retry) in [
        (
            "policy",
            GuardVerificationRepairReason::PolicyChanged,
            GuardVerificationRetryPolicy::RepairRequired,
        ),
        (
            "hook_digest",
            GuardVerificationRepairReason::HookDefinitionChanged,
            GuardVerificationRetryPolicy::HookReviewRequired,
        ),
        (
            "revision",
            GuardVerificationRepairReason::IntegrationRevisionChanged,
            GuardVerificationRetryPolicy::RepairRequired,
        ),
    ] {
        let owner_fixture =
            VerificationFixture::new(&format!("guard-integration-status-{owner_fact}"))?;
        let owner_run = owner_fixture.begin()?;
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
            &owner_fixture.context()?,
            &owner_run.verification_id,
            &owner_fixture.caller(),
            "2026-07-23T00:00:04Z",
        )?;
        assert!(
            matches!(
                failed.workflow,
                IntegrationVerificationWorkflowState::RepairRequired {
                    reason,
                    retry_policy,
                    ..
                } if reason == expected_reason && retry_policy == expected_retry
            ),
            "{owner_fact} drift must require typed repair",
        );
        assert_eq!(
            owner_fixture.record(&owner_run.verification_id)?.status,
            "repair_required"
        );
    }
    Ok(())
}

#[test]
fn missing_synchronous_events_require_repair_on_the_one_allowed_read() -> Result<(), Box<dyn Error>>
{
    let fixture = VerificationFixture::new("guard-integration-status-missing-hooks")?;
    let run = fixture.begin()?;
    fixture.acknowledge(&run.verification_id, ACK_AT)?;
    let result = get_guard_integration_verification(
        &fixture.context()?,
        &run.verification_id,
        &fixture.caller(),
        "2026-07-23T00:00:04.001Z",
    )?;
    assert!(matches!(
        result.workflow,
        IntegrationVerificationWorkflowState::RepairRequired {
            reason: GuardVerificationRepairReason::HookEventNotObserved,
            retry_policy: GuardVerificationRetryPolicy::HostReloadRequired,
            ..
        }
    ));
    let stored = fixture.record(&run.verification_id)?;
    assert_eq!(stored.status_read_count, 1);
    assert_eq!(stored.status, "repair_required");
    Ok(())
}
