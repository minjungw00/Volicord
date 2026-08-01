use std::error::Error;

use volicord_host_contract::{
    CanonicalToolName, CodexHookToolCorrelation, HostHookMatcherStrategy, HostNativeCorrelation,
    HostSessionId, HostToolUseId, HostTurnId, McpServerKey, McpToolCatalog,
};
use volicord_types::integration_verification::{
    GuardProbeEventRelevance, GuardProbeObservationStage, GuardVerificationRepairReason,
    IntegrationVerificationWorkflowState,
};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::GuardHookPhase;

use super::support::*;
use crate::integration_verification::{
    get_guard_integration_verification, guard_probe_observations,
    observation::classify_routed_tool_relevance, observe_unbound_guard_probe_hook_event,
    GuardProbeHookEvidence, UnboundGuardProbeHookObservation,
};
use crate::{sqlite::registry_db_path, StoreError};

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
            &fixture.context()?,
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
fn observation_before_attempt_creation_is_corrupt_data() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-observation-invalid-chronology")?;
    let run = fixture.begin()?;
    fixture.acknowledge(&run.verification_id, ACK_AT)?;
    let conn = crate::sqlite::open_registry_database_for_test(registry_db_path(
        fixture.runtime_home.path(),
    ))?;
    conn.execute(
        "UPDATE guard_probe_observations
            SET observed_at = '2026-07-23T00:00:02Z'
          WHERE verification_id = ?1
            AND stage = 'probe_acknowledged'",
        [&run.verification_id],
    )?;
    drop(conn);

    assert!(matches!(
        guard_probe_observations(fixture.runtime_home.path(), &run.verification_id),
        Err(StoreError::CorruptStoredValue {
            field: "guard_probe_observations.lifecycle_timestamp_order",
            ..
        })
    ));
    Ok(())
}

#[test]
fn catalog_roles_drive_routed_tool_relevance_before_probe_coordinates() -> Result<(), Box<dyn Error>>
{
    let server = McpServerKey::parse(SERVER_KEY)?;
    let matcher = HostHookMatcherStrategy::codex_guard(&server)?;
    let catalog = McpToolCatalog::for_server(&server, AgentToolId::ALL)?;
    for (tool, expected) in [
        (
            AgentToolId::GUARD_PROBE,
            GuardProbeEventRelevance::ProbeTarget {
                tool: AgentToolId::GUARD_PROBE,
            },
        ),
        (
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION,
            GuardProbeEventRelevance::WorkflowControl {
                tool: AgentToolId::BEGIN_INTEGRATION_VERIFICATION,
            },
        ),
        (
            AgentToolId::GET_INTEGRATION_VERIFICATION,
            GuardProbeEventRelevance::WorkflowControl {
                tool: AgentToolId::GET_INTEGRATION_VERIFICATION,
            },
        ),
        (
            AgentToolId::STATUS,
            GuardProbeEventRelevance::UnrelatedKnownTool {
                tool: AgentToolId::STATUS,
            },
        ),
    ] {
        let observed = CanonicalToolName::parse(host_callable_name(tool).as_str())?;
        assert_eq!(
            classify_routed_tool_relevance(&matcher, &catalog, &server, &observed),
            expected
        );
    }
    assert_eq!(
        classify_routed_tool_relevance(
            &matcher,
            &catalog,
            &server,
            &CanonicalToolName::parse("mcp__volicord_verification__unknown_same_server_tool")?,
        ),
        GuardProbeEventRelevance::UnknownSameServerCallable
    );
    assert_eq!(
        classify_routed_tool_relevance(
            &matcher,
            &catalog,
            &server,
            &CanonicalToolName::parse("mcp__foreign__volicord_guard_probe")?,
        ),
        GuardProbeEventRelevance::NotRouted
    );
    assert_eq!(
        classify_routed_tool_relevance(
            &matcher,
            &catalog,
            &server,
            &CanonicalToolName::parse("Bash")?,
        ),
        GuardProbeEventRelevance::NotRouted
    );
    Ok(())
}

#[test]
fn known_non_probe_tools_are_nonterminal_even_with_current_probe_coordinates(
) -> Result<(), Box<dyn Error>> {
    for tool in [
        AgentToolId::BEGIN_INTEGRATION_VERIFICATION,
        AgentToolId::GET_INTEGRATION_VERIFICATION,
        AgentToolId::STATUS,
    ] {
        let fixture =
            VerificationFixture::new(&format!("guard-observation-known-{}", tool.wire_name()))?;
        let run = fixture.begin()?;
        fixture.acknowledge(&run.verification_id, ACK_AT)?;
        let tool_name = host_callable_name(tool);
        for (phase, event_id, tool_use_id, observed_at) in [
            (
                "pre_tool",
                "guard_event_known_pre",
                "tool_use_known",
                "2026-07-23T00:00:04.100Z",
            ),
            (
                "post_tool",
                "guard_event_known_post",
                "tool_use_known",
                "2026-07-23T00:00:04.200Z",
            ),
        ] {
            fixture.insert_tool_event(ToolEventFixture {
                event_id,
                phase,
                turn: HOST_TURN_ID,
                tool_use_id,
                tool_name: tool_name.as_str(),
                verification_id: &run.verification_id,
                occurred_at: observed_at,
                digest: None,
                policy_hash: None,
                integration_revision: None,
            })?;
        }
        for (session, turn, tool_use_id, observed_at) in [
            (
                "other_session",
                HOST_TURN_ID,
                "tool_use_known_wrong_session",
                "2026-07-23T00:00:04.300Z",
            ),
            (
                HOST_SESSION_ID,
                "other_turn",
                "tool_use_known_wrong_turn",
                "2026-07-23T00:00:04.400Z",
            ),
        ] {
            observe_unbound_guard_probe_hook_event(
                &fixture.context()?,
                PROJECT_ID,
                UnboundGuardProbeHookObservation {
                    connection_internal_id: CONNECTION_ID.to_owned(),
                    guard_installation_id: INSTALLATION_ID.to_owned(),
                    correlation: HostNativeCorrelation::CodexHookTool(CodexHookToolCorrelation {
                        session_id: HostSessionId::parse(session)?,
                        turn_id: HostTurnId::parse(turn)?,
                        tool_use_id: HostToolUseId::parse(tool_use_id)?,
                        tool_name: CanonicalToolName::parse(tool_name.as_str())?,
                    }),
                    phase: GuardHookPhase::PreTool,
                    evidence: GuardProbeHookEvidence::present(Some(run.verification_id.clone())),
                    observed_at: observed_at.to_owned(),
                },
            )?;
        }
        let observations =
            guard_probe_observations(fixture.runtime_home.path(), &run.verification_id)?;
        assert_eq!(
            observations
                .iter()
                .filter(|observation| {
                    observation.stage == GuardProbeObservationStage::UnrelatedRoutedTool
                })
                .count(),
            3,
            "the two unbound coordinate variants share one bounded stage record"
        );
        assert!(!observations.iter().any(|observation| {
            matches!(
                observation.stage,
                GuardProbeObservationStage::CallableIdentityUnknown
                    | GuardProbeObservationStage::CallableIdentityMismatch
                    | GuardProbeObservationStage::SessionMismatch
                    | GuardProbeObservationStage::TurnMismatch
                    | GuardProbeObservationStage::VerificationIdMismatch
                    | GuardProbeObservationStage::PreToolMatched
                    | GuardProbeObservationStage::PostToolMatched
            )
        }));
        assert_eq!(fixture.record(&run.verification_id)?.status_read_count, 0);
        assert!(matches!(
            get_guard_integration_verification(
                &fixture.context()?,
                &run.verification_id,
                &fixture.caller(),
                "2026-07-23T00:00:05Z",
            )?
            .workflow,
            IntegrationVerificationWorkflowState::RepairRequired {
                reason: GuardVerificationRepairReason::HookEventNotObserved,
                ..
            }
        ));
    }
    Ok(())
}

#[test]
fn unknown_same_server_callable_is_terminal_only_when_it_claims_the_current_id(
) -> Result<(), Box<dyn Error>> {
    let unknown = CanonicalToolName::parse("mcp__volicord_verification__unknown_same_server_tool")?;
    for (suffix, evidence, expected_stage, expected_reason) in [
        (
            "unclaimed",
            GuardProbeHookEvidence::absent(),
            GuardProbeObservationStage::UnrelatedRoutedTool,
            GuardVerificationRepairReason::HookEventNotObserved,
        ),
        (
            "claimed",
            GuardProbeHookEvidence::present(Some(String::new())),
            GuardProbeObservationStage::CallableIdentityUnknown,
            GuardVerificationRepairReason::CallableIdentityMismatch,
        ),
    ] {
        let fixture = VerificationFixture::new(&format!("guard-observation-unknown-{suffix}"))?;
        let run = fixture.begin()?;
        fixture.acknowledge(&run.verification_id, ACK_AT)?;
        let evidence = if suffix == "claimed" {
            GuardProbeHookEvidence::present(Some(run.verification_id.clone()))
        } else {
            evidence
        };
        observe_unbound_guard_probe_hook_event(
            &fixture.context()?,
            PROJECT_ID,
            UnboundGuardProbeHookObservation {
                connection_internal_id: CONNECTION_ID.to_owned(),
                guard_installation_id: INSTALLATION_ID.to_owned(),
                correlation: HostNativeCorrelation::CodexHookTool(CodexHookToolCorrelation {
                    session_id: HostSessionId::parse(HOST_SESSION_ID)?,
                    turn_id: HostTurnId::parse(HOST_TURN_ID)?,
                    tool_use_id: HostToolUseId::parse(format!("tool_use_unknown_{suffix}"))?,
                    tool_name: unknown.clone(),
                }),
                phase: GuardHookPhase::PreTool,
                evidence,
                observed_at: "2026-07-23T00:00:04.100Z".to_owned(),
            },
        )?;
        let observations =
            guard_probe_observations(fixture.runtime_home.path(), &run.verification_id)?;
        assert!(observations
            .iter()
            .any(|observation| observation.stage == expected_stage));
        assert!(matches!(
            get_guard_integration_verification(
                &fixture.context()?,
                &run.verification_id,
                &fixture.caller(),
                "2026-07-23T00:00:05Z",
            )?
            .workflow,
            IntegrationVerificationWorkflowState::RepairRequired { reason, .. }
                if reason == expected_reason
        ));
    }
    Ok(())
}

#[test]
fn status_tool_self_observation_cannot_poison_missing_probe_result() -> Result<(), Box<dyn Error>> {
    let fixture = VerificationFixture::new("guard-observation-status-self")?;
    let run = fixture.begin()?;
    fixture.acknowledge(&run.verification_id, ACK_AT)?;
    let status_tool = host_callable_name(AgentToolId::GET_INTEGRATION_VERIFICATION);
    fixture.insert_tool_event(ToolEventFixture {
        event_id: "guard_event_status_pre",
        phase: "pre_tool",
        turn: HOST_TURN_ID,
        tool_use_id: "tool_use_status",
        tool_name: status_tool.as_str(),
        verification_id: &run.verification_id,
        occurred_at: "2026-07-23T00:00:04.100Z",
        digest: None,
        policy_hash: None,
        integration_revision: None,
    })?;
    let before_get = fixture.record(&run.verification_id)?;
    assert_eq!(before_get.status, "awaiting_observation");
    assert_eq!(before_get.status_read_count, 0);

    let first = get_guard_integration_verification(
        &fixture.context()?,
        &run.verification_id,
        &fixture.caller(),
        "2026-07-23T00:00:04.200Z",
    )?;
    assert!(matches!(
        first.workflow,
        IntegrationVerificationWorkflowState::RepairRequired {
            reason: GuardVerificationRepairReason::HookEventNotObserved,
            ..
        }
    ));
    fixture.insert_tool_event(ToolEventFixture {
        event_id: "guard_event_status_post",
        phase: "post_tool",
        turn: HOST_TURN_ID,
        tool_use_id: "tool_use_status",
        tool_name: status_tool.as_str(),
        verification_id: &run.verification_id,
        occurred_at: "2026-07-23T00:00:04.300Z",
        digest: None,
        policy_hash: None,
        integration_revision: None,
    })?;
    let second = get_guard_integration_verification(
        &fixture.context()?,
        &run.verification_id,
        &fixture.caller(),
        "2026-07-23T00:00:04.400Z",
    )?;
    assert_eq!(first.workflow, second.workflow);
    let stored = fixture.record(&run.verification_id)?;
    assert_eq!(stored.status, "repair_required");
    assert_eq!(stored.status_read_count, 1);
    let observations = guard_probe_observations(fixture.runtime_home.path(), &run.verification_id)?;
    assert_eq!(
        observations
            .iter()
            .filter(|observation| {
                observation.stage == GuardProbeObservationStage::UnrelatedRoutedTool
            })
            .count(),
        1,
        "the post-hook arrives after terminalization and cannot mutate the run"
    );
    assert!(!observations.iter().any(|observation| {
        matches!(
            observation.stage,
            GuardProbeObservationStage::CallableIdentityUnknown
                | GuardProbeObservationStage::CallableIdentityMismatch
        )
    }));
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
        match case {
            "payload" => fixture.insert_incompatible_tool_event(
                "guard_event_malformed",
                "pre_tool",
                "2026-07-23T00:00:04.100Z",
            )?,
            "callable" => {
                fixture.set_expected_host_callable_name(
                    &run.verification_id,
                    host_callable_name(AgentToolId::STATUS).as_str(),
                )?;
                fixture.insert_tool_event(ToolEventFixture {
                    event_id: "guard_event_callable",
                    phase: "pre_tool",
                    turn: HOST_TURN_ID,
                    tool_use_id: "tool_use_callable",
                    tool_name: probe.as_str(),
                    verification_id: &run.verification_id,
                    occurred_at: "2026-07-23T00:00:04.100Z",
                    digest: None,
                    policy_hash: None,
                    integration_revision: None,
                })?;
            }
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
                &fixture.context()?,
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
            &fixture.context()?,
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
            &fixture.context()?,
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
