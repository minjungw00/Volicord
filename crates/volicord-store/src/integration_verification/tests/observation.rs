use std::error::Error;

use volicord_host_contract::{
    CanonicalToolName, CodexHookToolCorrelation, HostNativeCorrelation, HostSessionId,
    HostToolUseId, HostTurnId,
};
use volicord_types::{
    AgentToolId, GuardHookPhase, GuardProbeObservationStage, GuardVerificationRepairReason,
    IntegrationVerificationWorkflowState,
};

use super::support::*;
use crate::integration_verification::{
    get_guard_integration_verification, guard_probe_observations,
    observe_unbound_guard_probe_hook_event, GuardProbeHookEvidence,
    UnboundGuardProbeHookObservation,
};

#[test]
fn current_pre_and_post_events_record_bounded_stages_and_complete() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-observation-current")?;
    let run = fixture.begin()?;
    let probe = host_callable_name(AgentToolId::GUARD_PROBE);
    fixture.insert_tool_event(ToolEventFixture {
        event_id: "guard_event_pre_current",
        phase: "pre_tool",
        turn: HOST_TURN_ID,
        tool_use_id: "tool_use_current",
        tool_name: probe.as_str(),
        verification_id: &run.verification_id,
        occurred_at: "2026-07-23T00:00:03.500Z",
        digest: None,
        policy_hash: None,
        integration_revision: None,
    })?;
    fixture.acknowledge(&run.verification_id, ACK_AT)?;
    fixture.insert_tool_event(ToolEventFixture {
        event_id: "guard_event_post_current",
        phase: "post_tool",
        turn: HOST_TURN_ID,
        tool_use_id: "tool_use_current",
        tool_name: probe.as_str(),
        verification_id: &run.verification_id,
        occurred_at: "2026-07-23T00:00:04.500Z",
        digest: None,
        policy_hash: None,
        integration_revision: None,
    })?;

    let observations = guard_probe_observations(fixture.runtime_home.path(), &run.verification_id)?;
    let stages = observations
        .iter()
        .map(|observation| observation.stage)
        .collect::<Vec<_>>();
    assert!(stages.contains(&GuardProbeObservationStage::PreToolMatched));
    assert!(stages.contains(&GuardProbeObservationStage::ProbeAcknowledged));
    assert!(stages.contains(&GuardProbeObservationStage::PostToolMatched));
    assert!(!stages.contains(&GuardProbeObservationStage::HookEventNotObserved));
    for observation in observations {
        assert_eq!(
            observation.expected_agent_tool_id,
            AgentToolId::GUARD_PROBE.wire_name()
        );
        assert_eq!(
            observation.expected_host_callable_name,
            run.expected_host_callable_name
        );
        assert!(observation
            .observed_callable_name
            .as_deref()
            .is_none_or(|name| name.len() <= 256));
    }
    assert!(matches!(
        get_guard_integration_verification(
            fixture.runtime_home.path(),
            &run.verification_id,
            &fixture.caller(),
            "2026-07-23T00:00:05Z",
        )?
        .workflow,
        IntegrationVerificationWorkflowState::Complete { .. }
    ));
    Ok(())
}

#[test]
fn routed_other_and_unknown_same_server_tools_do_not_satisfy_probe() -> Result<(), Box<dyn Error>> {
    for (suffix, tool_name, expected_stage) in [
        (
            "other",
            host_callable_name(AgentToolId::STATUS).into_inner(),
            GuardProbeObservationStage::CallableIdentityMismatch,
        ),
        (
            "unknown",
            "mcp__volicord_verification__unknown_same_server_tool".to_owned(),
            GuardProbeObservationStage::CallableIdentityUnknown,
        ),
    ] {
        let fixture = VerificationFixture::new(&format!("guard-observation-{suffix}"))?;
        let run = fixture.begin()?;
        fixture.acknowledge(&run.verification_id, ACK_AT)?;
        fixture.insert_tool_event(ToolEventFixture {
            event_id: "guard_event_other",
            phase: "pre_tool",
            turn: HOST_TURN_ID,
            tool_use_id: "tool_use_other",
            tool_name: &tool_name,
            verification_id: &run.verification_id,
            occurred_at: "2026-07-23T00:00:04.100Z",
            digest: None,
            policy_hash: None,
            integration_revision: None,
        })?;
        let observations =
            guard_probe_observations(fixture.runtime_home.path(), &run.verification_id)?;
        let observed = observations
            .iter()
            .find(|observation| observation.stage == expected_stage)
            .expect("typed mismatch observation");
        assert_eq!(
            observed.observed_callable_name.as_deref(),
            Some(tool_name.as_str())
        );
        assert!(matches!(
            get_guard_integration_verification(
                fixture.runtime_home.path(),
                &run.verification_id,
                &fixture.caller(),
                "2026-07-23T00:00:05Z",
            )?
            .workflow,
            IntegrationVerificationWorkflowState::RepairRequired {
                reason: GuardVerificationRepairReason::CallableIdentityMismatch,
                ..
            }
        ));
    }
    Ok(())
}

#[test]
fn correlation_failures_map_to_distinct_terminal_repair_reasons() -> Result<(), Box<dyn Error>> {
    for (case, expected_reason) in [
        (
            "payload",
            GuardVerificationRepairReason::HookPayloadIncompatible,
        ),
        (
            "callable",
            GuardVerificationRepairReason::CallableIdentityMismatch,
        ),
        (
            "verification",
            GuardVerificationRepairReason::VerificationIdMismatch,
        ),
        ("session", GuardVerificationRepairReason::SessionMismatch),
        ("turn", GuardVerificationRepairReason::TurnMismatch),
        ("tool_use", GuardVerificationRepairReason::ToolUseMismatch),
    ] {
        let fixture = VerificationFixture::new(&format!("guard-observation-repair-{case}"))?;
        let run = fixture.begin()?;
        fixture.acknowledge(&run.verification_id, ACK_AT)?;
        let probe = host_callable_name(AgentToolId::GUARD_PROBE);
        let status = host_callable_name(AgentToolId::STATUS);
        match case {
            "payload" => fixture.insert_incompatible_tool_event(
                "guard_event_malformed",
                "pre_tool",
                "2026-07-23T00:00:04.100Z",
            )?,
            "callable" => fixture.insert_tool_event(ToolEventFixture {
                event_id: "guard_event_callable",
                phase: "pre_tool",
                turn: HOST_TURN_ID,
                tool_use_id: "tool_use_callable",
                tool_name: status.as_str(),
                verification_id: &run.verification_id,
                occurred_at: "2026-07-23T00:00:04.100Z",
                digest: None,
                policy_hash: None,
                integration_revision: None,
            })?,
            "verification" => fixture.insert_tool_event(ToolEventFixture {
                event_id: "guard_event_verification",
                phase: "pre_tool",
                turn: HOST_TURN_ID,
                tool_use_id: "tool_use_verification",
                tool_name: probe.as_str(),
                verification_id: "guard_verification_other",
                occurred_at: "2026-07-23T00:00:04.100Z",
                digest: None,
                policy_hash: None,
                integration_revision: None,
            })?,
            "session" | "turn" => observe_unbound_guard_probe_hook_event(
                fixture.runtime_home.path(),
                PROJECT_ID,
                UnboundGuardProbeHookObservation {
                    connection_internal_id: CONNECTION_ID.to_owned(),
                    guard_installation_id: INSTALLATION_ID.to_owned(),
                    correlation: HostNativeCorrelation::CodexHookTool(CodexHookToolCorrelation {
                        session_id: HostSessionId::parse(if case == "session" {
                            "other_session"
                        } else {
                            HOST_SESSION_ID
                        })?,
                        turn_id: HostTurnId::parse(if case == "turn" {
                            "other_turn"
                        } else {
                            HOST_TURN_ID
                        })?,
                        tool_use_id: HostToolUseId::parse(format!("tool_use_{case}"))?,
                        tool_name: CanonicalToolName::parse(probe.as_str())?,
                    }),
                    phase: GuardHookPhase::PreTool,
                    evidence: GuardProbeHookEvidence::present(Some(run.verification_id.clone())),
                    observed_at: "2026-07-23T00:00:04.100Z".to_owned(),
                },
            )
            .map(|_| ())?,
            "tool_use" => {
                fixture.insert_tool_event(ToolEventFixture {
                    event_id: "guard_event_pre_tool_use",
                    phase: "pre_tool",
                    turn: HOST_TURN_ID,
                    tool_use_id: "tool_use_pre",
                    tool_name: probe.as_str(),
                    verification_id: &run.verification_id,
                    occurred_at: "2026-07-23T00:00:03.500Z",
                    digest: None,
                    policy_hash: None,
                    integration_revision: None,
                })?;
                fixture.insert_tool_event(ToolEventFixture {
                    event_id: "guard_event_post_tool_use",
                    phase: "post_tool",
                    turn: HOST_TURN_ID,
                    tool_use_id: "tool_use_post",
                    tool_name: probe.as_str(),
                    verification_id: &run.verification_id,
                    occurred_at: "2026-07-23T00:00:04.500Z",
                    digest: None,
                    policy_hash: None,
                    integration_revision: None,
                })?;
            }
            _ => unreachable!(),
        }
        let result = get_guard_integration_verification(
            fixture.runtime_home.path(),
            &run.verification_id,
            &fixture.caller(),
            "2026-07-23T00:00:05Z",
        )?;
        assert!(
            matches!(
                result.workflow,
                IntegrationVerificationWorkflowState::RepairRequired { reason, .. }
                    if reason == expected_reason
            ),
            "{case} must map to {expected_reason:?}",
        );
    }
    Ok(())
}

#[test]
fn absence_malformed_payload_and_correlation_mismatches_are_distinct() -> Result<(), Box<dyn Error>>
{
    let fixture = VerificationFixture::new("guard-observation-distinct-stages")?;
    let run = fixture.begin()?;
    fixture.acknowledge(&run.verification_id, ACK_AT)?;
    fixture.insert_incompatible_tool_event(
        "guard_event_malformed",
        "pre_tool",
        "2026-07-23T00:00:04.100Z",
    )?;

    for (phase, session, turn, expected_stage) in [
        (
            GuardHookPhase::PreTool,
            "other_session",
            HOST_TURN_ID,
            GuardProbeObservationStage::SessionMismatch,
        ),
        (
            GuardHookPhase::PreTool,
            HOST_SESSION_ID,
            "other_turn",
            GuardProbeObservationStage::TurnMismatch,
        ),
    ] {
        observe_unbound_guard_probe_hook_event(
            fixture.runtime_home.path(),
            PROJECT_ID,
            UnboundGuardProbeHookObservation {
                connection_internal_id: CONNECTION_ID.to_owned(),
                guard_installation_id: INSTALLATION_ID.to_owned(),
                correlation: HostNativeCorrelation::CodexHookTool(CodexHookToolCorrelation {
                    session_id: HostSessionId::parse(session)?,
                    turn_id: HostTurnId::parse(turn)?,
                    tool_use_id: HostToolUseId::parse(format!(
                        "tool_use_{}",
                        expected_stage.as_str()
                    ))?,
                    tool_name: CanonicalToolName::parse(
                        host_callable_name(AgentToolId::GUARD_PROBE).as_str(),
                    )?,
                }),
                phase,
                evidence: GuardProbeHookEvidence::present(Some(run.verification_id.clone())),
                observed_at: "2026-07-23T00:00:04.200Z".to_owned(),
            },
        )?;
    }

    let observations = guard_probe_observations(fixture.runtime_home.path(), &run.verification_id)?;
    for expected in [
        GuardProbeObservationStage::ProbeAcknowledged,
        GuardProbeObservationStage::HookEventNotObserved,
        GuardProbeObservationStage::HookPayloadIncompatible,
        GuardProbeObservationStage::SessionMismatch,
        GuardProbeObservationStage::TurnMismatch,
    ] {
        assert!(
            observations
                .iter()
                .any(|observation| observation.stage == expected),
            "missing {expected:?}"
        );
    }
    Ok(())
}

#[test]
fn verification_and_tool_use_mismatches_have_distinct_typed_stages() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-observation-coordinate-mismatch")?;
    let run = fixture.begin()?;
    fixture.acknowledge(&run.verification_id, ACK_AT)?;
    let probe = host_callable_name(AgentToolId::GUARD_PROBE);
    fixture.insert_tool_event(ToolEventFixture {
        event_id: "guard_event_wrong_verification",
        phase: "pre_tool",
        turn: HOST_TURN_ID,
        tool_use_id: "tool_use_wrong_verification",
        tool_name: probe.as_str(),
        verification_id: "guard_verification_other",
        occurred_at: "2026-07-23T00:00:04.100Z",
        digest: None,
        policy_hash: None,
        integration_revision: None,
    })?;
    fixture.insert_tool_event(ToolEventFixture {
        event_id: "guard_event_pre_matching",
        phase: "pre_tool",
        turn: HOST_TURN_ID,
        tool_use_id: "tool_use_pre",
        tool_name: probe.as_str(),
        verification_id: &run.verification_id,
        occurred_at: "2026-07-23T00:00:04.200Z",
        digest: None,
        policy_hash: None,
        integration_revision: None,
    })?;
    fixture.insert_tool_event(ToolEventFixture {
        event_id: "guard_event_post_wrong_tool_use",
        phase: "post_tool",
        turn: HOST_TURN_ID,
        tool_use_id: "tool_use_post",
        tool_name: probe.as_str(),
        verification_id: &run.verification_id,
        occurred_at: "2026-07-23T00:00:04.500Z",
        digest: None,
        policy_hash: None,
        integration_revision: None,
    })?;
    let stages = guard_probe_observations(fixture.runtime_home.path(), &run.verification_id)?
        .into_iter()
        .map(|observation| observation.stage)
        .collect::<Vec<_>>();
    assert!(stages.contains(&GuardProbeObservationStage::VerificationIdMismatch));
    assert!(stages.contains(&GuardProbeObservationStage::ToolUseMismatch));
    Ok(())
}
