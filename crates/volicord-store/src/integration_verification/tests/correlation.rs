use std::error::Error;

use volicord_host_contract::HostContractProfileId;
use volicord_types::{AgentToolId, IntegrationVerificationWorkflowState};

use super::support::*;
use crate::integration_verification::{
    current_guard_integration_verification_workflow, get_guard_integration_verification,
    refresh_guard_integration_verification_for_event,
};

#[test]
fn exact_prompt_and_exact_pre_post_tool_use_complete() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-correlation-exact")?;
    let run = fixture.begin()?;
    fixture.acknowledge(&run.verification_id, ACK_AT)?;
    fixture.insert_exact_tool_events(&run.verification_id)?;
    let updated = fixture.record(&run.verification_id)?;
    assert_eq!(updated.status, "complete");
    assert_eq!(
        updated.matched_prompt_event_id.as_deref(),
        Some("guard_event_prompt")
    );
    assert_eq!(
        updated.matched_pre_tool_event_id.as_deref(),
        Some("guard_event_pre")
    );
    assert_eq!(
        updated.matched_post_tool_event_id.as_deref(),
        Some("guard_event_post")
    );
    Ok(())
}

#[test]
fn mismatched_tool_identity_owner_and_contract_facts_never_complete() -> Result<(), Box<dyn Error>>
{
    let probe_name = host_callable_name(AgentToolId::GUARD_PROBE);
    let status_name = host_callable_name(AgentToolId::STATUS);
    for (index, mismatch) in [
        "turn",
        "tool_use",
        "tool_name",
        "verification_id",
        "hook_digest",
        "policy_hash",
        "integration_revision",
    ]
    .into_iter()
    .enumerate()
    {
        let fixture = VerificationFixture::new(&format!("guard-integration-mismatch-{index}"))?;
        let run = fixture.begin()?;
        fixture.acknowledge(&run.verification_id, ACK_AT)?;
        let turn = if mismatch == "turn" {
            "other_turn"
        } else {
            HOST_TURN_ID
        };
        let post_use = if mismatch == "tool_use" {
            "tool_use_post"
        } else {
            "tool_use_pre"
        };
        let tool_name = if mismatch == "tool_name" {
            status_name.as_str()
        } else {
            probe_name.as_str()
        };
        let verification_id = if mismatch == "verification_id" {
            "guard_verification_other"
        } else {
            &run.verification_id
        };
        let digest = (mismatch == "hook_digest").then_some(STALE_HASH);
        let policy_hash = (mismatch == "policy_hash").then_some(STALE_HASH);
        let integration_revision = (mismatch == "integration_revision").then_some(STALE_HASH);
        let mut owner_mismatch_rejected = false;
        for (event_id, phase, tool_use_id, occurred_at) in [
            (
                "guard_event_pre_bad",
                "pre_tool",
                "tool_use_pre",
                "2026-07-23T00:00:03.500Z",
            ),
            (
                "guard_event_post_bad",
                "post_tool",
                post_use,
                "2026-07-23T00:00:04.500Z",
            ),
        ] {
            let inserted = fixture.insert_tool_event(ToolEventFixture {
                event_id,
                phase,
                turn,
                tool_use_id,
                tool_name,
                verification_id,
                occurred_at,
                digest,
                policy_hash,
                integration_revision,
            });
            if matches!(mismatch, "policy_hash" | "integration_revision") {
                assert!(
                    inserted.is_err(),
                    "{mismatch} must be rejected at insertion"
                );
                owner_mismatch_rejected = true;
                break;
            }
            inserted?;
        }
        if !owner_mismatch_rejected {
            refresh_guard_integration_verification_for_event(
                &fixture.context()?,
                PROJECT_ID,
                "guard_event_post_bad",
            )?;
        }
        assert!(
            matches!(
                get_guard_integration_verification(
                    &fixture.context()?,
                    &run.verification_id,
                    &fixture.caller(),
                    "2026-07-23T00:00:05Z",
                )?
                .workflow,
                IntegrationVerificationWorkflowState::RepairRequired { .. }
            ),
            "{mismatch} must not complete",
        );
    }
    Ok(())
}

#[test]
fn timestamp_ordering_and_unrelated_history_are_excluded() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-integration-correlation-order")?;
    let run = fixture.begin()?;
    fixture.acknowledge(&run.verification_id, ACK_AT)?;
    let probe_name = host_callable_name(AgentToolId::GUARD_PROBE);

    for (event_id, phase, occurred_at) in [
        (
            "guard_event_historical_pre",
            "pre_tool",
            "2026-07-23T00:00:00.500Z",
        ),
        (
            "guard_event_historical_post",
            "post_tool",
            "2026-07-23T00:00:00.750Z",
        ),
    ] {
        fixture.insert_tool_event(ToolEventFixture {
            event_id,
            phase,
            turn: HOST_TURN_ID,
            tool_use_id: "tool_use_historical",
            tool_name: probe_name.as_str(),
            verification_id: &run.verification_id,
            occurred_at,
            digest: None,
            policy_hash: None,
            integration_revision: None,
        })?;
    }
    refresh_guard_integration_verification_for_event(
        &fixture.context()?,
        PROJECT_ID,
        "guard_event_historical_post",
    )?;
    assert!(matches!(
        current_guard_integration_verification_workflow(
            fixture.runtime_home.path(),
            &fixture.record(&run.verification_id)?,
            "2026-07-23T00:00:04.100Z",
        )?,
        IntegrationVerificationWorkflowState::AwaitingObservation { .. }
    ));

    fixture.insert_tool_event(ToolEventFixture {
        event_id: "guard_event_current_pre",
        phase: "pre_tool",
        turn: HOST_TURN_ID,
        tool_use_id: "tool_use_current",
        tool_name: probe_name.as_str(),
        verification_id: &run.verification_id,
        occurred_at: "2026-07-23T00:00:03.500Z",
        digest: None,
        policy_hash: None,
        integration_revision: None,
    })?;
    fixture.insert_tool_event(ToolEventFixture {
        event_id: "guard_event_post_before_ack",
        phase: "post_tool",
        turn: HOST_TURN_ID,
        tool_use_id: "tool_use_current",
        tool_name: probe_name.as_str(),
        verification_id: &run.verification_id,
        occurred_at: "2026-07-23T00:00:03.750Z",
        digest: Some(
            HostContractProfileId::CodexCommandHooks
                .contract_digest()
                .as_str(),
        ),
        policy_hash: None,
        integration_revision: None,
    })?;
    refresh_guard_integration_verification_for_event(
        &fixture.context()?,
        PROJECT_ID,
        "guard_event_post_before_ack",
    )?;
    assert!(matches!(
        current_guard_integration_verification_workflow(
            fixture.runtime_home.path(),
            &fixture.record(&run.verification_id)?,
            "2026-07-23T00:00:04.200Z",
        )?,
        IntegrationVerificationWorkflowState::AwaitingObservation { .. }
    ));
    Ok(())
}
