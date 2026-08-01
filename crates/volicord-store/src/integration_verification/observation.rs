use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Row};
use sha2::{Digest, Sha256};
use volicord_host_contract::{
    parse_callable_name, HostCallableName, HostHookMatcherStrategy, HostNativeCorrelation,
    McpServerKey, McpToolCatalog,
};
use volicord_types::guard_manifest::guard_manifest_from_json;
use volicord_types::integration_verification::{
    GuardProbeEventRelevance, GuardProbeObservationStage,
};
use volicord_types::tool_names::{AgentToolId, IntegrationVerificationToolRole};
use volicord_types::values::{GuardHookPhase, UtcTimestamp};

use super::{
    correlation::refresh_guard_integration_verification_for_event,
    row::{
        active_run_for_acquisition, parse_status, parse_timestamp, run_by_id,
        ActiveAcquisitionRunLookup,
    },
    status::workflow_state_from_record,
    GuardIntegrationVerificationRunRecord,
};
use crate::{
    agent_connections::agent_connection_record_read_only,
    bootstrap::project_record_for_execution,
    guards::{guard_event, guard_installation, GuardEventRecord},
    sqlite::{begin_immediate_transaction, open_registry_database_for_mutation, registry_db_path},
    RuntimeHomeMutationContext, StoreError, StoreResult,
};

/// Bounded semantic evidence extracted from one routed hook event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardProbeHookEvidence {
    pub verification_id_present: bool,
    pub verification_id: Option<String>,
}

/// One decoded routed hook event that could not acquire current session binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnboundGuardProbeHookObservation {
    pub connection_internal_id: String,
    pub guard_installation_id: String,
    pub correlation: HostNativeCorrelation,
    pub phase: GuardHookPhase,
    pub evidence: GuardProbeHookEvidence,
    pub observed_at: String,
}

impl GuardProbeHookEvidence {
    pub fn absent() -> Self {
        Self {
            verification_id_present: false,
            verification_id: None,
        }
    }

    pub fn present(value: Option<String>) -> Self {
        Self {
            verification_id_present: true,
            verification_id: value,
        }
    }
}

/// One persisted bounded acquisition fact for a Guard verification run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardProbeObservationRecord {
    pub observation_id: String,
    pub verification_id: String,
    pub guard_event_id: Option<String>,
    pub stage: GuardProbeObservationStage,
    pub expected_agent_tool_id: String,
    pub expected_host_callable_name: String,
    pub observed_callable_name: Option<String>,
    pub hook_event_kind: Option<String>,
    pub verification_id_present: bool,
    pub verification_id_matches: bool,
    pub guard_installation_id: String,
    pub integration_revision: String,
    pub observed_at: String,
}

/// Records one routed event after semantic hook decoding.
pub fn observe_guard_probe_hook_event(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    guard_event_id: &str,
    evidence: GuardProbeHookEvidence,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    let runtime_home = context.runtime_home().as_path();
    let Some(event) = guard_event(runtime_home, project_id, guard_event_id)? else {
        return Ok(None);
    };
    let project = project_record_for_execution(runtime_home, project_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        }
    })?;
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let run = active_run_for_acquisition(
        &tx,
        ActiveAcquisitionRunLookup {
            connection_internal_id: &event.connection_internal_id,
            project_internal_id: &project.project_internal_id,
            guard_installation_id: &event.guard_installation_id,
            integration_revision: &event.integration_revision,
            policy_digest: &event.policy_hash,
        },
    )?;
    let Some(run) = run else {
        tx.commit()?;
        return Ok(None);
    };
    let classification =
        classify_hook_event(runtime_home, project_id, &tx, &run, &event, &evidence)?;
    if let Some(classification) = classification {
        insert_observation(
            &tx,
            &run,
            Some(guard_event_id),
            classification.stage,
            classification.observed_callable_name,
            classification.hook_event_kind,
            evidence.verification_id_present,
            classification.verification_id_matches,
            &event.occurred_at,
        )?;
    }
    tx.commit()?;
    if classification.is_some_and(|classification| {
        matches!(
            classification.stage,
            GuardProbeObservationStage::PreToolMatched
                | GuardProbeObservationStage::PostToolMatched
        )
    }) {
        refresh_guard_integration_verification_for_event(context, project_id, guard_event_id)
    } else {
        Ok(Some(run))
    }
}

/// Records bounded correlation diagnostics when a decoded event cannot be bound.
pub fn observe_unbound_guard_probe_hook_event(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    input: UnboundGuardProbeHookObservation,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    let runtime_home = context.runtime_home().as_path();
    let project = project_record_for_execution(runtime_home, project_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        }
    })?;
    let installation =
        guard_installation(runtime_home, &input.guard_installation_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "guard_installation",
                id: input.guard_installation_id.clone(),
            }
        })?;
    let manifest = guard_manifest_from_json(&installation.manifest_json).map_err(|_| {
        StoreError::CorruptOwnerStateJson {
            database_kind: "registry",
            table: "guard_installations",
            record_ref: input.guard_installation_id.clone(),
            logical_column: "manifest_json",
        }
    })?;
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let run = active_run_for_acquisition(
        &tx,
        ActiveAcquisitionRunLookup {
            connection_internal_id: &input.connection_internal_id,
            project_internal_id: &project.project_internal_id,
            guard_installation_id: &input.guard_installation_id,
            integration_revision: manifest.integration_revision.as_str(),
            policy_digest: manifest.policy_hash.as_str(),
        },
    )?;
    let Some(run) = run else {
        tx.commit()?;
        return Ok(None);
    };
    let event_kind = input.phase.as_str().to_owned();
    let event = GuardEventRecord {
        project_id: project_id.to_owned(),
        guard_event_id: String::new(),
        session_id: None,
        correlation: Some(input.correlation),
        connection_internal_id: input.connection_internal_id,
        guard_installation_id: input.guard_installation_id,
        policy_hash: manifest.policy_hash.as_str().to_owned(),
        integration_revision: manifest.integration_revision.as_str().to_owned(),
        event_kind,
        contract_status: "compatible".to_owned(),
        decision: "warn".to_owned(),
        subject_json: "{}".to_owned(),
        result_json: "{}".to_owned(),
        occurred_at: input.observed_at,
        metadata_json: "{}".to_owned(),
    };
    let classification =
        classify_hook_event(runtime_home, project_id, &tx, &run, &event, &input.evidence)?;
    if let Some(classification) = classification {
        insert_observation(
            &tx,
            &run,
            None,
            classification.stage,
            classification.observed_callable_name,
            classification.hook_event_kind,
            input.evidence.verification_id_present,
            classification.verification_id_matches,
            &event.occurred_at,
        )?;
    }
    tx.commit()?;
    Ok(Some(run))
}

/// Lists the bounded acquisition history for one exact verification.
pub fn guard_probe_observations(
    runtime_home: impl AsRef<Path>,
    verification_id: &str,
) -> StoreResult<Vec<GuardProbeObservationRecord>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = crate::sqlite::open_registry_database_read_only(path)?;
    let observations = observations_for_run(&conn, verification_id)?;
    if let Some(run) = run_by_id(&conn, verification_id)? {
        workflow_state_from_record(&run, parse_status(&run.status)?)?;
        validate_stored_observations(&run, &observations)?;
    }
    Ok(observations)
}

fn validate_stored_observations(
    run: &GuardIntegrationVerificationRunRecord,
    observations: &[GuardProbeObservationRecord],
) -> StoreResult<()> {
    let created_at = stored_observation_timestamp("run_created_at", &run.created_at)?;
    let completed_at = run
        .completed_at
        .as_deref()
        .map(|value| stored_observation_timestamp("run_completed_at", value))
        .transpose()?;
    for observation in observations {
        if observation.verification_id != run.verification_id
            || observation.expected_agent_tool_id != run.expected_probe_tool
            || observation.expected_host_callable_name != run.expected_host_callable_name
            || observation.guard_installation_id != run.guard_installation_id
            || observation.integration_revision != run.integration_revision
        {
            return Err(StoreError::corrupt_stored_value(
                "registry",
                "guard_probe_observations.verification_identity",
            ));
        }
        let observed_at = stored_observation_timestamp("observed_at", &observation.observed_at)?;
        if observed_at < created_at
            || completed_at
                .as_ref()
                .is_some_and(|terminal_at| observed_at > *terminal_at)
        {
            return Err(StoreError::corrupt_stored_value(
                "registry",
                "guard_probe_observations.lifecycle_timestamp_order",
            ));
        }
    }
    Ok(())
}

fn stored_observation_timestamp(field: &'static str, value: &str) -> StoreResult<UtcTimestamp> {
    UtcTimestamp::parse(value).map_err(|_| {
        StoreError::corrupt_stored_value(
            "registry",
            match field {
                "run_created_at" => "guard_integration_verification_runs.created_at",
                "run_completed_at" => "guard_integration_verification_runs.completed_at",
                _ => "guard_probe_observations.observed_at",
            },
        )
    })
}

pub(super) fn record_probe_acknowledgement(
    conn: &Connection,
    run: &GuardIntegrationVerificationRunRecord,
    observed_at: &str,
) -> StoreResult<()> {
    insert_observation(
        conn,
        run,
        None,
        GuardProbeObservationStage::ProbeAcknowledged,
        None,
        None,
        true,
        true,
        observed_at,
    )?;
    let pre_event_observed = conn.query_row(
        "SELECT EXISTS (
            SELECT 1
              FROM guard_probe_observations
             WHERE verification_id = ?1
               AND stage = 'pre_tool_matched'
        )",
        [run.verification_id.as_str()],
        |row| row.get::<_, bool>(0),
    )?;
    if !pre_event_observed {
        insert_observation(
            conn,
            run,
            None,
            GuardProbeObservationStage::HookEventNotObserved,
            None,
            None,
            false,
            false,
            observed_at,
        )?;
    }
    Ok(())
}

pub(super) fn observations_for_run(
    conn: &Connection,
    verification_id: &str,
) -> StoreResult<Vec<GuardProbeObservationRecord>> {
    let mut statement = conn.prepare(
        "SELECT observation_id, verification_id, guard_event_id, stage,
                expected_agent_tool_id, expected_host_callable_name,
                observed_callable_name, hook_event_kind,
                verification_id_present, verification_id_matches,
                guard_installation_id, integration_revision, observed_at
           FROM guard_probe_observations
          WHERE verification_id = ?1
          ORDER BY observed_at, observation_id",
    )?;
    let rows = statement.query_map([verification_id], observation_from_row)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

#[derive(Debug, Clone, Copy)]
struct HookClassification<'a> {
    stage: GuardProbeObservationStage,
    observed_callable_name: Option<&'a str>,
    hook_event_kind: Option<&'a str>,
    verification_id_matches: bool,
}

fn classify_hook_event<'a>(
    runtime_home: &Path,
    project_id: &str,
    conn: &Connection,
    run: &GuardIntegrationVerificationRunRecord,
    event: &'a GuardEventRecord,
    evidence: &GuardProbeHookEvidence,
) -> StoreResult<Option<HookClassification<'a>>> {
    let hook_event_kind = matches!(event.event_kind.as_str(), "pre_tool" | "post_tool")
        .then_some(event.event_kind.as_str());
    if event.contract_status != "compatible" {
        return Ok(Some(HookClassification {
            stage: GuardProbeObservationStage::HookPayloadIncompatible,
            observed_callable_name: None,
            hook_event_kind,
            verification_id_matches: false,
        }));
    }
    let Some(HostNativeCorrelation::CodexHookTool(correlation)) = event.correlation.as_ref() else {
        return Ok(Some(HookClassification {
            stage: GuardProbeObservationStage::HookPayloadIncompatible,
            observed_callable_name: None,
            hook_event_kind,
            verification_id_matches: false,
        }));
    };
    let observed_callable_name = Some(correlation.tool_name.as_str());
    let verification_id_matches = evidence
        .verification_id
        .as_deref()
        .is_some_and(|value| value == run.verification_id);
    let connection = agent_connection_record_read_only(runtime_home, &run.connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: run.connection_internal_id.clone(),
        })?;
    let server = McpServerKey::parse(connection.server_name).map_err(|_| {
        StoreError::corrupt_stored_value("registry", "agent_connections.server_name")
    })?;
    let matcher = HostHookMatcherStrategy::codex_guard(&server).map_err(|_| {
        StoreError::corrupt_stored_value("registry", "agent_connections.server_name")
    })?;
    let catalog = McpToolCatalog::for_server(&server, AgentToolId::ALL).map_err(|_| {
        StoreError::corrupt_stored_value("registry", "agent_connections.server_name")
    })?;
    let relevance =
        classify_routed_tool_relevance(&matcher, &catalog, &server, &correlation.tool_name);
    match relevance {
        GuardProbeEventRelevance::NotRouted => return Ok(None),
        GuardProbeEventRelevance::WorkflowControl { .. }
        | GuardProbeEventRelevance::UnrelatedKnownTool { .. } => {
            return Ok(Some(HookClassification {
                stage: GuardProbeObservationStage::UnrelatedRoutedTool,
                observed_callable_name,
                hook_event_kind,
                verification_id_matches,
            }));
        }
        GuardProbeEventRelevance::UnknownSameServerCallable if !verification_id_matches => {
            return Ok(Some(HookClassification {
                stage: GuardProbeObservationStage::UnrelatedRoutedTool,
                observed_callable_name,
                hook_event_kind,
                verification_id_matches: false,
            }));
        }
        GuardProbeEventRelevance::UnknownSameServerCallable => {
            return Ok(Some(HookClassification {
                stage: GuardProbeObservationStage::CallableIdentityUnknown,
                observed_callable_name,
                hook_event_kind,
                verification_id_matches: true,
            }));
        }
        GuardProbeEventRelevance::ProbeTarget { .. } => {}
    }
    if correlation.tool_name.as_str() != run.expected_host_callable_name {
        return Ok(Some(HookClassification {
            stage: GuardProbeObservationStage::CallableIdentityMismatch,
            observed_callable_name,
            hook_event_kind,
            verification_id_matches,
        }));
    }
    if correlation.session_id.as_str() != run.host_session_id {
        return Ok(Some(HookClassification {
            stage: GuardProbeObservationStage::SessionMismatch,
            observed_callable_name,
            hook_event_kind,
            verification_id_matches,
        }));
    }
    if correlation.turn_id.as_str() != run.host_turn_id {
        return Ok(Some(HookClassification {
            stage: GuardProbeObservationStage::TurnMismatch,
            observed_callable_name,
            hook_event_kind,
            verification_id_matches,
        }));
    }
    if !verification_id_matches {
        return Ok(Some(HookClassification {
            stage: GuardProbeObservationStage::VerificationIdMismatch,
            observed_callable_name,
            hook_event_kind,
            verification_id_matches: false,
        }));
    }
    let stage = match event.event_kind.as_str() {
        "pre_tool" => GuardProbeObservationStage::PreToolMatched,
        "post_tool" => {
            let pre_event_id = conn
                .query_row(
                    "SELECT guard_event_id
                       FROM guard_probe_observations
                      WHERE verification_id = ?1
                        AND stage = 'pre_tool_matched'
                        AND guard_event_id IS NOT NULL
                      ORDER BY observed_at DESC, observation_id DESC
                      LIMIT 1",
                    [run.verification_id.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            let Some(pre_event_id) = pre_event_id else {
                return Ok(Some(HookClassification {
                    stage: GuardProbeObservationStage::HookEventNotObserved,
                    observed_callable_name,
                    hook_event_kind,
                    verification_id_matches: true,
                }));
            };
            let Some(pre_event) = guard_event(runtime_home, project_id, &pre_event_id)? else {
                return Err(StoreError::CorruptStoredValue {
                    database_kind: "registry",
                    field: "guard_probe_observations.guard_event_id",
                });
            };
            let same_tool_use = matches!(
                pre_event.correlation,
                Some(HostNativeCorrelation::CodexHookTool(ref pre))
                    if pre.tool_use_id == correlation.tool_use_id
            );
            if same_tool_use {
                GuardProbeObservationStage::PostToolMatched
            } else {
                GuardProbeObservationStage::ToolUseMismatch
            }
        }
        _ => GuardProbeObservationStage::HookPayloadIncompatible,
    };
    Ok(Some(HookClassification {
        stage,
        observed_callable_name,
        hook_event_kind,
        verification_id_matches: true,
    }))
}

pub(super) fn classify_routed_tool_relevance(
    matcher: &HostHookMatcherStrategy,
    catalog: &McpToolCatalog,
    server: &McpServerKey,
    observed: &volicord_host_contract::CanonicalToolName,
) -> GuardProbeEventRelevance {
    if !matcher.routes_mcp_callable(observed) {
        return GuardProbeEventRelevance::NotRouted;
    }
    let Some(source) = HostCallableName::parse(observed.as_str())
        .ok()
        .and_then(|callable| parse_callable_name(&callable, catalog).ok())
    else {
        return GuardProbeEventRelevance::UnknownSameServerCallable;
    };
    if source.server() != server {
        return GuardProbeEventRelevance::NotRouted;
    }
    let tool = source.tool();
    match tool.integration_verification_role() {
        IntegrationVerificationToolRole::ProbeTarget => {
            GuardProbeEventRelevance::ProbeTarget { tool }
        }
        IntegrationVerificationToolRole::WorkflowControl => {
            GuardProbeEventRelevance::WorkflowControl { tool }
        }
        IntegrationVerificationToolRole::UnrelatedKnownTool => {
            GuardProbeEventRelevance::UnrelatedKnownTool { tool }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_observation(
    conn: &Connection,
    run: &GuardIntegrationVerificationRunRecord,
    guard_event_id: Option<&str>,
    stage: GuardProbeObservationStage,
    observed_callable_name: Option<&str>,
    hook_event_kind: Option<&str>,
    verification_id_present: bool,
    verification_id_matches: bool,
    observed_at: &str,
) -> StoreResult<()> {
    parse_timestamp("observed_at", observed_at)?;
    if observed_callable_name.is_some_and(|value| {
        value.is_empty()
            || value.len() > 256
            || value.trim() != value
            || value.chars().any(char::is_control)
    }) {
        return Err(StoreError::InvalidInput {
            detail: "observed callable name must be bounded diagnostic data".to_owned(),
        });
    }
    let observation_key = guard_event_id.unwrap_or(stage.as_str());
    let digest = Sha256::digest(
        format!(
            "{}\0{}\0{}",
            run.verification_id,
            observation_key,
            stage.as_str()
        )
        .as_bytes(),
    );
    let observation_id = format!("guard_probe_observation_{digest:x}");
    conn.execute(
        "INSERT INTO guard_probe_observations (
            observation_id, verification_id, guard_event_id, stage,
            expected_agent_tool_id, expected_host_callable_name,
            observed_callable_name, hook_event_kind,
            verification_id_present, verification_id_matches,
            guard_installation_id, integration_revision, observed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(observation_id) DO NOTHING",
        params![
            observation_id,
            run.verification_id,
            guard_event_id,
            stage.as_str(),
            run.expected_probe_tool,
            run.expected_host_callable_name,
            observed_callable_name,
            hook_event_kind,
            verification_id_present,
            verification_id_matches,
            run.guard_installation_id,
            run.integration_revision,
            observed_at,
        ],
    )?;
    Ok(())
}

fn observation_from_row(row: &Row<'_>) -> rusqlite::Result<GuardProbeObservationRecord> {
    let stage: String = row.get(3)?;
    let stage = GuardProbeObservationStage::from_storage_str(&stage).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Text,
            "unknown Guard probe observation stage".into(),
        )
    })?;
    Ok(GuardProbeObservationRecord {
        observation_id: row.get(0)?,
        verification_id: row.get(1)?,
        guard_event_id: row.get(2)?,
        stage,
        expected_agent_tool_id: row.get(4)?,
        expected_host_callable_name: row.get(5)?,
        observed_callable_name: row.get(6)?,
        hook_event_kind: row.get(7)?,
        verification_id_present: row.get(8)?,
        verification_id_matches: row.get(9)?,
        guard_installation_id: row.get(10)?,
        integration_revision: row.get(11)?,
        observed_at: row.get(12)?,
    })
}
