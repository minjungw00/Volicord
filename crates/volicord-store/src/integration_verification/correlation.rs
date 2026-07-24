use std::path::Path;

use serde_json::Value;
use volicord_host_contract::{
    parse_callable_name, HostCallableIdentity, HostCallableName, HostNativeCorrelation,
    McpServerKey, McpToolCatalog,
};
use volicord_types::AgentToolId;
use volicord_types::GuardProbeObservationStage;

use super::{
    observation::{observations_for_run, GuardProbeObservationRecord},
    row::{
        active_run_for_event, complete_run, expire_active_runs, parse_timestamp, run_by_id,
        ActiveEventRunLookup, CorrelatedEventIds,
    },
    GuardIntegrationVerificationRunRecord,
};
use crate::{
    agent_connections::agent_connection_record_read_only,
    guards::{
        guard_event, guard_events_for_integration_verification, GuardEventRecord,
        GuardIntegrationVerificationEventQuery,
    },
    operational_sessions::current_managed_mcp_runtime_session_for_connection,
    sqlite::{begin_immediate_transaction, open_registry_database, registry_db_path},
    StoreError, StoreResult,
};

const COMPATIBLE_CONTRACT: &str = "compatible";

/// Re-evaluates an active run after one compatible Guard event is persisted.
pub fn refresh_guard_integration_verification_for_event(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    guard_event_id: &str,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    let runtime_home = runtime_home.as_ref();
    let Some(trigger) = guard_event(runtime_home, project_id, guard_event_id)? else {
        return Ok(None);
    };
    let Some(correlation) = trigger.correlation.as_ref() else {
        return Ok(None);
    };
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    expire_active_runs(&tx, &trigger.occurred_at)?;
    let run = active_run_for_event(
        &tx,
        ActiveEventRunLookup {
            connection_internal_id: &trigger.connection_internal_id,
            host_session_id: correlation.session_id().as_str(),
            host_turn_id: correlation.turn_id().as_str(),
            guard_installation_id: &trigger.guard_installation_id,
            integration_revision: &trigger.integration_revision,
            policy_hash: &trigger.policy_hash,
        },
    )?;
    let Some(run) = run else {
        tx.commit()?;
        return Ok(None);
    };
    if current_managed_mcp_runtime_session_for_connection(
        runtime_home,
        &run.runtime_session_id,
        &run.connection_internal_id,
    )
    .is_err()
    {
        tx.commit()?;
        return Ok(Some(run));
    }
    let events = guard_events_for_integration_verification(
        runtime_home,
        GuardIntegrationVerificationEventQuery {
            project_id,
            connection_internal_id: &run.connection_internal_id,
            session_id: trigger.session_id.as_deref().unwrap_or_default(),
            host_turn_id: &run.host_turn_id,
            guard_installation_id: &run.guard_installation_id,
            policy_hash: &run.policy_hash,
            integration_revision: &run.integration_revision,
        },
    )?;
    let connection = agent_connection_record_read_only(runtime_home, &run.connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: run.connection_internal_id.clone(),
        })?;
    let server = McpServerKey::parse(connection.server_name).map_err(|_| {
        StoreError::corrupt_stored_value("registry", "agent_connections.server_name")
    })?;
    let catalog = McpToolCatalog::for_server(&server, AgentToolId::ALL).map_err(|_| {
        StoreError::corrupt_stored_value("registry", "agent_connections.server_name")
    })?;
    let probe = catalog
        .require(&server, AgentToolId::GUARD_PROBE)
        .map_err(|_| {
            StoreError::corrupt_stored_value("registry", "agent_connections.server_name")
        })?;
    let observations = observations_for_run(&tx, &run.verification_id)?;
    let Some((prompt, pre, post)) =
        correlated_event_triple(&run, &events, &observations, &catalog, probe)?
    else {
        tx.commit()?;
        return Ok(Some(run));
    };
    complete_run(
        &tx,
        &run.verification_id,
        &post.occurred_at,
        CorrelatedEventIds {
            prompt: &prompt.guard_event_id,
            pre_tool: &pre.guard_event_id,
            post_tool: &post.guard_event_id,
        },
    )?;
    let updated = run_by_id(&tx, &run.verification_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "guard_integration_verification",
        id: run.verification_id,
    })?;
    tx.commit()?;
    Ok(Some(updated))
}

fn correlated_event_triple<'a>(
    run: &GuardIntegrationVerificationRunRecord,
    events: &'a [GuardEventRecord],
    observations: &[GuardProbeObservationRecord],
    catalog: &McpToolCatalog,
    probe: &HostCallableIdentity,
) -> StoreResult<
    Option<(
        &'a GuardEventRecord,
        &'a GuardEventRecord,
        &'a GuardEventRecord,
    )>,
> {
    let Some(acknowledged_at) = run.probe_acknowledged_at.as_deref() else {
        return Ok(None);
    };
    let ack = parse_timestamp("probe_acknowledged_at", acknowledged_at)?;
    let prompt = events
        .iter()
        .filter(|event| prompt_event_matches(event, &run.hook_contract_digest))
        .filter(|event| run.matched_prompt_event_id.as_deref() == Some(&event.guard_event_id));
    for prompt in prompt {
        let prompt_at = parse_timestamp("occurred_at", &prompt.occurred_at)?;
        for pre in events.iter().filter(|event| {
            tool_event_matches(
                event,
                "pre_tool",
                GuardProbeObservationStage::PreToolMatched,
                observations,
                catalog,
                probe,
                &run.hook_contract_digest,
            )
        }) {
            let HostNativeCorrelation::CodexHookTool(pre_correlation) = pre
                .correlation
                .as_ref()
                .expect("matching tool event has correlation")
            else {
                continue;
            };
            let pre_at = parse_timestamp("occurred_at", &pre.occurred_at)?;
            if prompt_at > pre_at || pre_at > ack {
                continue;
            }
            for post in events.iter().filter(|event| {
                tool_event_matches(
                    event,
                    "post_tool",
                    GuardProbeObservationStage::PostToolMatched,
                    observations,
                    catalog,
                    probe,
                    &run.hook_contract_digest,
                )
            }) {
                let HostNativeCorrelation::CodexHookTool(post_correlation) = post
                    .correlation
                    .as_ref()
                    .expect("matching tool event has correlation")
                else {
                    continue;
                };
                let post_at = parse_timestamp("occurred_at", &post.occurred_at)?;
                if pre_correlation.tool_use_id == post_correlation.tool_use_id
                    && pre_at < post_at
                    && ack <= post_at
                {
                    return Ok(Some((prompt, pre, post)));
                }
            }
        }
    }
    Ok(None)
}

pub(super) fn prompt_event_matches(event: &GuardEventRecord, digest: &str) -> bool {
    event.event_kind == "prompt_capture"
        && event.contract_status == COMPATIBLE_CONTRACT
        && matches!(
            event.correlation,
            Some(HostNativeCorrelation::CodexHookPrompt(_))
        )
        && event_contract_digest(event).as_deref() == Some(digest)
}

fn tool_event_matches(
    event: &GuardEventRecord,
    kind: &str,
    expected_stage: GuardProbeObservationStage,
    observations: &[GuardProbeObservationRecord],
    catalog: &McpToolCatalog,
    expected: &HostCallableIdentity,
    digest: &str,
) -> bool {
    let Some(HostNativeCorrelation::CodexHookTool(correlation)) = event.correlation.as_ref() else {
        return false;
    };
    let callable = HostCallableName::parse(correlation.tool_name.as_str());
    let source_matches = callable
        .as_ref()
        .ok()
        .and_then(|callable| parse_callable_name(callable, catalog).ok())
        .is_some_and(|source| &source == expected.source());
    event.event_kind == kind
        && event.contract_status == COMPATIBLE_CONTRACT
        && source_matches
        && event_contract_digest(event).as_deref() == Some(digest)
        && observations.iter().any(|observation| {
            observation.guard_event_id.as_deref() == Some(event.guard_event_id.as_str())
                && observation.stage == expected_stage
                && observation.verification_id_present
                && observation.verification_id_matches
        })
}

fn event_contract_digest(event: &GuardEventRecord) -> Option<String> {
    serde_json::from_str::<Value>(&event.metadata_json)
        .ok()?
        .get("host_contract_digest")?
        .as_str()
        .map(str::to_owned)
}
