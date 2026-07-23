//! Authoritative MCP runtime-session lifecycle records.

use std::{path::Path, str::FromStr};

use rusqlite::{params, Connection, OptionalExtension, Row};
use volicord_host_contract::{CodexMcpCorrelation, HostNativeCorrelation, HostSessionId};
use volicord_types::{
    project_agent_session_id, AgentRuntimeSessionId, ConnectionIntegrationRevisionBasis,
    DiagnosticFindingId, DurableIdGenerator, DurableIdKind, IntegrationRevision,
    ManagedMcpClientInfo, McpRuntimeSessionSource, OccurrenceDiagnosticFinding,
    RandomDurableIdGenerator, ToolVerificationRole, UtcTimestamp, DURABLE_ID_RETRY_LIMIT,
};

use crate::{
    agent_connections::{raw_agent_connection_record_from_conn, AgentConnectionRecord},
    bootstrap::raw_project_record_from_conn,
    diagnostic_findings::insert_and_link_runtime_terminal_occurrence,
    sqlite::{
        begin_immediate_transaction, open_registry_database, open_registry_database_read_only,
        registry_db_path,
    },
    StoreError, StoreResult,
};

const MAX_DIAGNOSTIC_FIELD_BYTES: usize = 1024;
const MAX_PROTOCOL_FIELD_BYTES: usize = 256;
const MAX_MCP_TOOL_NAME_BYTES: usize = 128;
const MAX_RETURNED_TOOL_IDENTITIES: usize = 256;

/// MCP process-start facts used to create an authoritative runtime session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRuntimeSessionStart {
    pub connection_internal_id: String,
    pub session_source: McpRuntimeSessionSource,
    pub observed_host_executable_version: Option<String>,
    pub process_id: u32,
    pub process_started_at: String,
}

/// Canonical Registry row for one MCP process launch and its observed milestones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRuntimeSessionRecord {
    pub runtime_session_id: String,
    pub connection_internal_id: String,
    pub session_source: McpRuntimeSessionSource,
    pub connection_integration_revision: String,
    pub observed_host_executable_version: Option<String>,
    pub attempted_client_name: Option<String>,
    pub attempted_client_version: Option<String>,
    pub requested_protocol_version: Option<String>,
    pub selected_protocol_version: Option<String>,
    pub negotiated_protocol_version: Option<String>,
    pub process_id: u32,
    pub process_started_at: String,
    pub initialize_completed_at: Option<String>,
    pub initialized_notification_at: Option<String>,
    pub tools_list_observed_at: Option<String>,
    pub returned_tool_identities: Option<Vec<String>>,
    pub required_tools_present: Option<bool>,
    pub required_tools_validated_at: Option<String>,
    pub verification_tool_name: Option<String>,
    pub verification_tool_observed_at: Option<String>,
    pub last_observed_at: String,
    pub terminal_finding_id: Option<String>,
    pub graceful_close_at: Option<String>,
}

/// Authoritative MCP peer facts observed from one runtime's initialize exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedPeerObservation {
    pub client_info: ManagedMcpClientInfo,
    pub requested_protocol_revision: String,
    pub selected_protocol_revision: Option<String>,
    pub negotiated_protocol_revision: Option<String>,
}

/// Typed lifecycle evidence for exactly one authoritative MCP runtime session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpSessionMilestones {
    pub runtime_session_id: AgentRuntimeSessionId,
    pub source: McpRuntimeSessionSource,
    pub connection_id: String,
    pub integration_revision: IntegrationRevision,
    pub process_started_at: UtcTimestamp,
    pub managed_peer: Option<ManagedPeerObservation>,
    pub initialize_completed_at: Option<UtcTimestamp>,
    pub initialized_notification_at: Option<UtcTimestamp>,
    pub tools_list_observed_at: Option<UtcTimestamp>,
    pub returned_tool_identities: Option<Vec<String>>,
    pub required_tools_present: Option<bool>,
    pub required_tools_validated_at: Option<UtcTimestamp>,
    pub verification_tool_name: Option<String>,
    pub verification_tool_observed_at: Option<UtcTimestamp>,
    pub terminal_finding: Option<DiagnosticFindingId>,
    pub last_observed_at: UtcTimestamp,
}

/// A complete same-session managed-host capability proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedCapabilityProof {
    milestones: McpSessionMilestones,
}

impl ManagedCapabilityProof {
    /// Returns the complete same-session milestones carried by this proof.
    pub fn milestones(&self) -> &McpSessionMilestones {
        &self.milestones
    }
}

/// Deterministic current-revision managed-host evidence roles.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpSessionEvidenceSelection {
    pub latest_attempt: Option<McpSessionMilestones>,
    pub latest_complete_proof: Option<ManagedCapabilityProof>,
}

impl McpSessionMilestones {
    /// Validates one persisted row and converts it to typed single-session evidence.
    pub fn try_from_record(record: &McpRuntimeSessionRecord) -> StoreResult<Self> {
        let integration_revision =
            IntegrationRevision::parse(record.connection_integration_revision.clone())
                .map_err(|_| corrupt(record, "connection_integration_revision"))?;
        let process_started_at =
            parse_stored_timestamp(record, "process_started_at", &record.process_started_at)?;
        let initialize_completed_at = parse_optional_stored_timestamp(
            record,
            "initialize_completed_at",
            record.initialize_completed_at.as_deref(),
        )?;
        let initialized_notification_at = parse_optional_stored_timestamp(
            record,
            "initialized_notification_at",
            record.initialized_notification_at.as_deref(),
        )?;
        let tools_list_observed_at = parse_optional_stored_timestamp(
            record,
            "tools_list_observed_at",
            record.tools_list_observed_at.as_deref(),
        )?;
        let required_tools_validated_at = parse_optional_stored_timestamp(
            record,
            "required_tools_validated_at",
            record.required_tools_validated_at.as_deref(),
        )?;
        let verification_tool_observed_at = parse_optional_stored_timestamp(
            record,
            "verification_tool_observed_at",
            record.verification_tool_observed_at.as_deref(),
        )?;
        let last_observed_at =
            parse_stored_timestamp(record, "last_observed_at", &record.last_observed_at)?;
        let managed_peer = match (
            record.attempted_client_name.as_deref(),
            record.attempted_client_version.as_deref(),
            record.requested_protocol_version.as_deref(),
        ) {
            (None, None, None) => None,
            (Some(name), Some(version), Some(requested)) => {
                let client_info = ManagedMcpClientInfo::new(name, version)
                    .map_err(|_| corrupt(record, "attempted_client_name"))?;
                validate_text(
                    "requested_protocol_version",
                    requested,
                    MAX_PROTOCOL_FIELD_BYTES,
                )
                .map_err(|_| corrupt(record, "requested_protocol_version"))?;
                if let Some(selected) = record.selected_protocol_version.as_deref() {
                    validate_text(
                        "selected_protocol_version",
                        selected,
                        MAX_PROTOCOL_FIELD_BYTES,
                    )
                    .map_err(|_| corrupt(record, "selected_protocol_version"))?;
                }
                if let Some(negotiated) = record.negotiated_protocol_version.as_deref() {
                    validate_text(
                        "negotiated_protocol_version",
                        negotiated,
                        MAX_PROTOCOL_FIELD_BYTES,
                    )
                    .map_err(|_| corrupt(record, "negotiated_protocol_version"))?;
                }
                Some(ManagedPeerObservation {
                    client_info,
                    requested_protocol_revision: requested.to_owned(),
                    selected_protocol_revision: record.selected_protocol_version.clone(),
                    negotiated_protocol_revision: record.negotiated_protocol_version.clone(),
                })
            }
            _ => return Err(corrupt(record, "attempted_client_name")),
        };
        if initialize_completed_at.is_some() != record.selected_protocol_version.is_some()
            || initialize_completed_at.is_some() && managed_peer.is_none()
        {
            return Err(corrupt(record, "initialize_completed_at"));
        }
        if initialized_notification_at.is_some() != record.negotiated_protocol_version.is_some()
            || initialized_notification_at.is_some() && initialize_completed_at.is_none()
            || record.negotiated_protocol_version.as_deref()
                != record.selected_protocol_version.as_deref()
                && record.negotiated_protocol_version.is_some()
        {
            return Err(corrupt(record, "negotiated_protocol_version"));
        }
        match (
            tools_list_observed_at.as_ref(),
            record.returned_tool_identities.as_ref(),
            record.required_tools_present,
            required_tools_validated_at.as_ref(),
        ) {
            (None, None, None, None) => {}
            (Some(_), Some(identities), Some(false), None) => {
                validate_returned_tool_identities(identities)
                    .map_err(|_| corrupt(record, "returned_tool_identities_json"))?;
            }
            (Some(_), Some(identities), Some(true), Some(_)) => {
                validate_returned_tool_identities(identities)
                    .map_err(|_| corrupt(record, "returned_tool_identities_json"))?;
            }
            _ => return Err(corrupt(record, "required_tools_validated_at")),
        }
        if tools_list_observed_at.is_some() && initialize_completed_at.is_none() {
            return Err(corrupt(record, "tools_list_observed_at"));
        }
        if last_observed_at < process_started_at
            || initialize_completed_at
                .as_ref()
                .is_some_and(|value| value < &process_started_at)
            || initialized_notification_at.as_ref().is_some_and(|value| {
                initialize_completed_at
                    .as_ref()
                    .is_none_or(|initialize| value < initialize)
            })
            || tools_list_observed_at.as_ref().is_some_and(|value| {
                initialize_completed_at
                    .as_ref()
                    .is_none_or(|initialize| value < initialize)
            })
            || required_tools_validated_at.as_ref().is_some_and(|value| {
                tools_list_observed_at
                    .as_ref()
                    .is_none_or(|tools| value < tools)
            })
            || verification_tool_observed_at.as_ref().is_some_and(|value| {
                required_tools_validated_at
                    .as_ref()
                    .is_none_or(|required| value < required)
                    || initialized_notification_at
                        .as_ref()
                        .is_none_or(|initialized| value < initialized)
            })
        {
            return Err(corrupt(record, "last_observed_at"));
        }
        match (
            record.verification_tool_name.as_deref(),
            verification_tool_observed_at.as_ref(),
        ) {
            (None, None) => {}
            (Some(name), Some(_)) => {
                validate_mcp_tool_name(name)
                    .map_err(|_| corrupt(record, "verification_tool_name"))?;
                if required_tools_validated_at.is_none() {
                    return Err(corrupt(record, "verification_tool_observed_at"));
                }
            }
            _ => return Err(corrupt(record, "verification_tool_name")),
        }
        let terminal_finding = record
            .terminal_finding_id
            .as_ref()
            .map(|value| {
                DiagnosticFindingId::parse(value.clone())
                    .map_err(|_| corrupt(record, "terminal_finding_id"))
            })
            .transpose()?;
        Ok(Self {
            runtime_session_id: AgentRuntimeSessionId::new(record.runtime_session_id.clone()),
            source: record.session_source,
            connection_id: record.connection_internal_id.clone(),
            integration_revision,
            process_started_at,
            managed_peer,
            initialize_completed_at,
            initialized_notification_at,
            tools_list_observed_at,
            returned_tool_identities: record.returned_tool_identities.clone(),
            required_tools_present: record.required_tools_present,
            required_tools_validated_at,
            verification_tool_name: record.verification_tool_name.clone(),
            verification_tool_observed_at,
            terminal_finding,
            last_observed_at,
        })
    }

    /// Returns whether this runtime has a linked terminal failure.
    pub fn terminally_failed(&self) -> bool {
        self.terminal_finding.is_some()
    }
}

impl ManagedCapabilityProof {
    /// Accepts only the full readiness chain from one current managed-host session.
    pub fn try_new(milestones: McpSessionMilestones) -> StoreResult<Self> {
        let expected_tool = ToolVerificationRole::ManagedHostRoundTrip
            .tool()
            .wire_name();
        if milestones.source != McpRuntimeSessionSource::ManagedHost {
            return Err(StoreError::InvalidInput {
                detail: "managed capability proof requires session_source=managed_host".to_owned(),
            });
        }
        if milestones.initialize_completed_at.is_none()
            || milestones.initialized_notification_at.is_none()
            || milestones.tools_list_observed_at.is_none()
            || milestones.returned_tool_identities.is_none()
            || milestones.required_tools_present != Some(true)
            || milestones.required_tools_validated_at.is_none()
            || milestones.verification_tool_name.as_deref() != Some(expected_tool)
            || milestones.verification_tool_observed_at.is_none()
        {
            return Err(StoreError::InvalidInput {
                detail: "managed capability proof requires one session's complete initialize, tools/list, required-tool, and verification-tool chain".to_owned(),
            });
        }
        Ok(Self { milestones })
    }
}

impl McpSessionEvidenceSelection {
    /// Selects fixed evidence roles without combining milestones across sessions.
    pub fn select(
        current_revision: &IntegrationRevision,
        sessions: &[McpRuntimeSessionRecord],
    ) -> StoreResult<Self> {
        let mut current = sessions
            .iter()
            .filter(|session| {
                session.session_source == McpRuntimeSessionSource::ManagedHost
                    && session.connection_integration_revision == current_revision.as_str()
            })
            .map(McpSessionMilestones::try_from_record)
            .collect::<StoreResult<Vec<_>>>()?;
        current.sort_by(|left, right| {
            right
                .last_observed_at
                .cmp(&left.last_observed_at)
                .then_with(|| {
                    right
                        .runtime_session_id
                        .as_str()
                        .cmp(left.runtime_session_id.as_str())
                })
        });
        let latest_attempt = current.first().cloned();
        let latest_complete_proof = current
            .into_iter()
            .find_map(|milestones| ManagedCapabilityProof::try_new(milestones).ok());
        Ok(Self {
            latest_attempt,
            latest_complete_proof,
        })
    }
}

/// Registry reservation joining one managed runtime to one project Agent Session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRuntimeProjectSessionBindingRecord {
    pub runtime_session_id: String,
    pub connection_internal_id: String,
    pub project_id: String,
    pub session_id: String,
    pub project_integration_revision: String,
    pub host_session_id: String,
    pub bound_at: String,
}

/// Derives the deterministic current integration revision for one Agent Connection.
pub fn connection_integration_revision(
    connection: &AgentConnectionRecord,
) -> StoreResult<IntegrationRevision> {
    IntegrationRevision::for_connection(ConnectionIntegrationRevisionBasis {
        connection_internal_id: &connection.connection_internal_id,
        integration_instance_id: &connection.integration_instance_id,
        host_kind: &connection.host_kind,
        intent: &connection.intent,
        host_scope: &connection.host_scope,
        mode: &connection.mode,
        server_name: &connection.server_name,
        config_target: &connection.config_target,
        managed_configuration_fingerprint: &connection.managed_fingerprint,
        integration_generation: connection.integration_generation,
    })
    .map_err(|error| StoreError::InvalidInput {
        detail: format!("Agent Connection cannot produce an integration revision: {error}"),
    })
}

/// Creates a new runtime session at MCP process startup.
pub fn start_mcp_runtime_session(
    runtime_home: impl AsRef<Path>,
    input: McpRuntimeSessionStart,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_start(&input)?;
    if input.session_source == McpRuntimeSessionSource::ManagedHost {
        return Err(StoreError::InvalidInput {
            detail: "managed_host runtime creation requires atomic launch-lease consumption"
                .to_owned(),
        });
    }
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let generator = RandomDurableIdGenerator;
    for _ in 0..DURABLE_ID_RETRY_LIMIT {
        let runtime_session_id = generator
            .generate(DurableIdKind::McpRuntimeSession)
            .map_err(|error| StoreError::InvalidInput {
                detail: format!("could not generate MCP runtime session id: {error}"),
            })?;
        let tx = begin_immediate_transaction(&mut conn)?;
        if let Some(record) =
            insert_mcp_runtime_session_in_transaction(&tx, &runtime_session_id, &input, None)?
        {
            tx.commit()?;
            return Ok(record);
        }
        tx.rollback()?;
    }
    Err(StoreError::Conflict {
        entity: "mcp_runtime_session",
        id: "generated".to_owned(),
        detail: "durable id collision retry limit was exhausted".to_owned(),
    })
}

#[cfg(test)]
pub(crate) fn start_mcp_runtime_session_for_test(
    runtime_home: impl AsRef<Path>,
    input: McpRuntimeSessionStart,
) -> StoreResult<McpRuntimeSessionRecord> {
    if input.session_source != McpRuntimeSessionSource::ManagedHost {
        return start_mcp_runtime_session(runtime_home, input);
    }
    let runtime_home = runtime_home.as_ref();
    let connection = crate::agent_connections::agent_connection_record_read_only(
        runtime_home,
        &input.connection_internal_id,
    )?
    .ok_or_else(|| StoreError::NotFound {
        entity: "agent_connection",
        id: input.connection_internal_id.clone(),
    })?;
    let revision = connection_integration_revision(&connection)?;
    let lease = crate::managed_launch_leases::issue_managed_mcp_launch_lease(
        runtime_home,
        crate::managed_launch_leases::ManagedMcpLaunchLeaseIssue {
            connection_internal_id: connection.connection_internal_id,
            host_kind: volicord_types::HostKind::Codex,
            expected_integration_revision: revision.as_str().to_owned(),
            expected_launch_fingerprint: connection.managed_fingerprint,
        },
    )?;
    crate::managed_launch_leases::consume_managed_mcp_launch_lease_and_start_runtime(
        runtime_home,
        crate::managed_launch_leases::ManagedMcpLaunchLeaseConsumption {
            launch_lease_id: lease.launch_lease_id,
            connection_internal_id: lease.connection_internal_id,
            host_kind: lease.host_kind,
            expected_integration_revision: lease.expected_integration_revision,
            expected_launch_fingerprint: lease.expected_launch_fingerprint,
        },
        input,
    )
}

pub(crate) fn insert_mcp_runtime_session_in_transaction(
    tx: &Connection,
    runtime_session_id: &str,
    input: &McpRuntimeSessionStart,
    expected_integration_revision: Option<&str>,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    validate_start(input)?;
    let connection = raw_agent_connection_record_from_conn(tx, &input.connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: input.connection_internal_id.clone(),
        })?;
    let revision = connection_integration_revision(&connection)?;
    if expected_integration_revision.is_some_and(|expected| expected != revision.as_str()) {
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: "current".to_owned(),
            detail: "Connection integration revision changed before runtime creation".to_owned(),
        });
    }
    let inserted = tx.execute(
        "INSERT OR IGNORE INTO mcp_runtime_sessions (
            runtime_session_id, connection_internal_id, session_source,
            connection_integration_revision, observed_host_executable_version,
            process_id, process_started_at, last_observed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            runtime_session_id,
            input.connection_internal_id,
            input.session_source.as_str(),
            revision.as_str(),
            input.observed_host_executable_version,
            i64::from(input.process_id),
            input.process_started_at,
        ],
    )?;
    if inserted == 0 {
        return Ok(None);
    }
    runtime_session_from_conn(tx, runtime_session_id)
}

/// Reads one authoritative runtime session without consulting diagnostics.
pub fn mcp_runtime_session(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    validate_text(
        "runtime_session_id",
        runtime_session_id,
        MAX_DIAGNOSTIC_FIELD_BYTES,
    )?;
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    runtime_session_from_conn(&conn, runtime_session_id)
}

/// Resolves a managed runtime session only when it is still owned by the expected
/// Connection and represents that Connection's current integration revision.
pub fn current_managed_mcp_runtime_session_for_connection(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    connection_internal_id: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    let path = registry_db_path(runtime_home);
    let conn = open_registry_database_read_only(path)?;
    let session = runtime_session_from_conn(&conn, runtime_session_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
        }
    })?;
    if session.connection_internal_id != connection_internal_id
        || session.session_source != McpRuntimeSessionSource::ManagedHost
    {
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail: "runtime session is not a managed-host launch owned by the expected Agent Connection".to_owned(),
        });
    }
    let connection = raw_agent_connection_record_from_conn(&conn, connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })?;
    if !connection.enabled {
        return Err(StoreError::Conflict {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
            detail: "Agent Connection is disabled".to_owned(),
        });
    }
    if connection_integration_revision(&connection)?.as_str()
        != session.connection_integration_revision
    {
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail:
                "runtime session does not represent the current Connection integration revision"
                    .to_owned(),
        });
    }
    Ok(session)
}

/// Exact Phase 2 inputs from one validated project Agent Session anchor.
#[derive(Debug, Clone, Copy)]
pub(crate) struct McpRuntimeProjectSessionReservation<'a> {
    pub runtime_session_id: &'a str,
    pub connection_internal_id: &'a str,
    pub project_id: &'a str,
    pub asserted_guard_installation_id: Option<&'a str>,
    pub expected_coordinates: &'a crate::guards::ProjectAgentSessionCoordinates,
    pub correlation: &'a CodexMcpCorrelation,
    pub bound_at: &'a str,
}

/// Reserves one runtime/host session for the exact validated project anchor.
/// This phase helper is crate-private so callers cannot bypass project validation.
pub(crate) fn reserve_mcp_runtime_project_session(
    runtime_home: impl AsRef<Path>,
    input: McpRuntimeProjectSessionReservation<'_>,
) -> StoreResult<McpRuntimeProjectSessionBindingRecord> {
    let McpRuntimeProjectSessionReservation {
        runtime_session_id,
        connection_internal_id,
        project_id,
        asserted_guard_installation_id,
        expected_coordinates,
        correlation,
        bound_at,
    } = input;
    let host_session_id = correlation.session_id.as_str();
    validate_timestamp("bound_at", bound_at)?;
    for (field, value) in [
        ("runtime_session_id", runtime_session_id),
        ("connection_internal_id", connection_internal_id),
        ("project_id", project_id),
        ("host_session_id", host_session_id),
    ] {
        validate_text(field, value, MAX_DIAGNOSTIC_FIELD_BYTES)?;
    }
    let runtime_home = runtime_home.as_ref();
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let runtime = runtime_session_from_conn(&tx, runtime_session_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
        }
    })?;
    let connection = raw_agent_connection_record_from_conn(&tx, connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })?;
    if !connection.enabled
        || runtime.connection_internal_id != connection_internal_id
        || runtime.session_source != McpRuntimeSessionSource::ManagedHost
        || runtime.connection_integration_revision
            != connection_integration_revision(&connection)?.as_str()
    {
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail: "runtime session is not a current managed-host launch owned by an enabled Agent Connection".to_owned(),
        });
    }
    let project =
        raw_project_record_from_conn(&tx, project_id)?.ok_or_else(|| StoreError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        })?;
    let membership: i64 = tx.query_row(
        "SELECT COUNT(*) FROM connection_projects
          WHERE connection_internal_id = ?1 AND project_internal_id = ?2",
        params![connection_internal_id, project.project_internal_id],
        |row| row.get(0),
    )?;
    if membership != 1 {
        return Err(StoreError::Conflict {
            entity: "connection_project",
            id: format!("{connection_internal_id}/{project_id}"),
            detail: "project is not a current member of the Agent Connection".to_owned(),
        });
    }
    let coordinates = crate::guards::current_project_agent_session_coordinates(
        runtime_home,
        project_id,
        connection_internal_id,
        asserted_guard_installation_id,
        &HostNativeCorrelation::CodexMcp(correlation.clone()),
    )?;
    if coordinates != *expected_coordinates {
        return Err(StoreError::Conflict {
            entity: "agent_session",
            id: expected_coordinates.session_id.clone(),
            detail: "current project Agent Session coordinates changed before Registry reservation"
                .to_owned(),
        });
    }
    let session_id = coordinates.session_id.as_str();
    let existing_project_session = tx
        .query_row(
            "SELECT runtime_session_id, connection_internal_id,
                    project_integration_revision, host_session_id, bound_at
               FROM mcp_runtime_project_session_bindings
              WHERE project_internal_id = ?1 AND session_id = ?2",
            params![project.project_internal_id, session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    if let Some((
        existing_runtime,
        existing_connection,
        existing_revision,
        existing_host_session,
        existing_bound_at,
    )) = existing_project_session
    {
        if existing_runtime == runtime_session_id
            && existing_connection == connection_internal_id
            && existing_revision == coordinates.project_integration_revision
            && existing_host_session == host_session_id
        {
            tx.commit()?;
            return Ok(McpRuntimeProjectSessionBindingRecord {
                runtime_session_id: runtime_session_id.to_owned(),
                connection_internal_id: connection_internal_id.to_owned(),
                project_id: project.project_internal_id,
                session_id: coordinates.session_id,
                project_integration_revision: coordinates.project_integration_revision,
                host_session_id: host_session_id.to_owned(),
                bound_at: existing_bound_at,
            });
        }
        return Err(StoreError::Conflict {
            entity: "agent_session",
            id: session_id.to_owned(),
            detail: "project Agent Session is already reserved for another runtime, Connection, or host session".to_owned(),
        });
    }
    let existing_runtime_host = tx
        .query_row(
            "SELECT connection_internal_id, project_internal_id, session_id
               FROM mcp_runtime_project_session_bindings
              WHERE runtime_session_id = ?1 AND host_session_id = ?2",
            params![runtime_session_id, host_session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    if existing_runtime_host.is_some() {
        return Err(StoreError::Conflict {
            entity: "agent_session",
            id: session_id.to_owned(),
            detail: "runtime host session is already reserved for another project or Connection"
                .to_owned(),
        });
    }
    tx.execute(
        "INSERT INTO mcp_runtime_project_session_bindings (
            runtime_session_id, connection_internal_id, project_internal_id,
            session_id, project_integration_revision, host_session_id, bound_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            runtime_session_id,
            connection_internal_id,
            project.project_internal_id,
            session_id,
            coordinates.project_integration_revision,
            host_session_id,
            bound_at
        ],
    )?;
    tx.commit()?;
    Ok(McpRuntimeProjectSessionBindingRecord {
        runtime_session_id: runtime_session_id.to_owned(),
        connection_internal_id: connection_internal_id.to_owned(),
        project_id: project.project_internal_id,
        session_id: coordinates.session_id,
        project_integration_revision: coordinates.project_integration_revision,
        host_session_id: host_session_id.to_owned(),
        bound_at: bound_at.to_owned(),
    })
}

/// Reads the exact Registry reservation for one project Agent Session.
pub fn mcp_runtime_project_session_binding(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    session_id: &str,
) -> StoreResult<Option<McpRuntimeProjectSessionBindingRecord>> {
    validate_text("project_id", project_id, MAX_DIAGNOSTIC_FIELD_BYTES)?;
    validate_text("session_id", session_id, MAX_DIAGNOSTIC_FIELD_BYTES)?;
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    let record = conn
        .query_row(
            "SELECT b.runtime_session_id, b.connection_internal_id, p.project_internal_id,
                    b.session_id, b.project_integration_revision, b.host_session_id, b.bound_at
               FROM mcp_runtime_project_session_bindings AS b
               JOIN projects AS p ON p.project_internal_id = b.project_internal_id
              WHERE p.project_internal_id = ?1 AND b.session_id = ?2",
            params![project_id, session_id],
            |row| {
                Ok(McpRuntimeProjectSessionBindingRecord {
                    runtime_session_id: row.get(0)?,
                    connection_internal_id: row.get(1)?,
                    project_id: row.get(2)?,
                    session_id: row.get(3)?,
                    project_integration_revision: row.get(4)?,
                    host_session_id: row.get(5)?,
                    bound_at: row.get(6)?,
                })
            },
        )
        .optional()?;
    record
        .map(|record| {
            let corrupt = |field| {
                StoreError::corrupt_owner_state_value(
                    "mcp_runtime_project_session_bindings",
                    record.session_id.clone(),
                    field,
                )
            };
            validate_timestamp("bound_at", &record.bound_at).map_err(|_| corrupt("bound_at"))?;
            for (field, value) in [
                ("runtime_session_id", record.runtime_session_id.as_str()),
                (
                    "connection_internal_id",
                    record.connection_internal_id.as_str(),
                ),
                ("project_id", record.project_id.as_str()),
                ("session_id", record.session_id.as_str()),
                (
                    "project_integration_revision",
                    record.project_integration_revision.as_str(),
                ),
            ] {
                validate_text(field, value, MAX_DIAGNOSTIC_FIELD_BYTES)
                    .map_err(|_| corrupt(field))?;
            }
            HostSessionId::parse(record.host_session_id.clone())
                .map_err(|_| corrupt("host_session_id"))?;
            if project_agent_session_id(
                &record.connection_internal_id,
                &record.project_integration_revision,
                &record.host_session_id,
            )
            .map_err(|_| corrupt("session_id"))?
                != record.session_id
            {
                return Err(corrupt("session_id"));
            }
            Ok(record)
        })
        .transpose()
}

/// Records parsed client/request data even when initialize later fails.
pub fn record_mcp_initialize_attempt(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    client_info: &ManagedMcpClientInfo,
    requested_protocol_version: &str,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_text(
        "requested_protocol_version",
        requested_protocol_version,
        MAX_PROTOCOL_FIELD_BYTES,
    )?;
    validate_timestamp("initialize_attempted_at", observed_at)?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.attempted_client_name.is_some() || prior.requested_protocol_version.is_some() {
            return Err(StoreError::Conflict {
                entity: "mcp_runtime_session",
                id: runtime_session_id.to_owned(),
                detail: "initialize attempt is already recorded".to_owned(),
            });
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET attempted_client_name = ?2, attempted_client_version = ?3,
                    requested_protocol_version = ?4, last_observed_at = ?5
              WHERE runtime_session_id = ?1",
            params![
                runtime_session_id,
                client_info.name(),
                client_info.version(),
                requested_protocol_version,
                observed_at
            ],
        )?;
        Ok(())
    })
}

/// Records initialize completion and the server-selected revision before response emission.
pub fn record_mcp_initialize_completion(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    selected_protocol_version: &str,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_text(
        "selected_protocol_version",
        selected_protocol_version,
        MAX_PROTOCOL_FIELD_BYTES,
    )?;
    validate_timestamp("initialize_completed_at", observed_at)?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.attempted_client_name.is_none() || prior.requested_protocol_version.is_none() {
            return Err(milestone_order(
                runtime_session_id,
                "initialize completion requires a parsed initialize attempt",
            ));
        }
        if prior.initialize_completed_at.is_some() {
            return Err(StoreError::Conflict {
                entity: "mcp_runtime_session",
                id: runtime_session_id.to_owned(),
                detail: "initialize has already completed".to_owned(),
            });
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET selected_protocol_version = ?2,
                    initialize_completed_at = ?3, last_observed_at = ?3
              WHERE runtime_session_id = ?1",
            params![runtime_session_id, selected_protocol_version, observed_at],
        )?;
        Ok(())
    })
}

/// Records the initialized notification. A duplicate valid notification is idempotent.
pub fn record_mcp_initialized_notification(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    negotiated_protocol_version: &str,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_text(
        "negotiated_protocol_version",
        negotiated_protocol_version,
        MAX_PROTOCOL_FIELD_BYTES,
    )?;
    validate_timestamp("initialized_notification_at", observed_at)?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.initialize_completed_at.is_none() {
            return Err(milestone_order(
                runtime_session_id,
                "initialized notification requires initialize completion",
            ));
        }
        if prior.selected_protocol_version.as_deref() != Some(negotiated_protocol_version) {
            return Err(StoreError::Conflict {
                entity: "mcp_runtime_session",
                id: runtime_session_id.to_owned(),
                detail: "negotiated protocol version differs from the server-selected revision"
                    .to_owned(),
            });
        }
        if let Some(prior_version) = prior.negotiated_protocol_version.as_deref() {
            if prior_version != negotiated_protocol_version {
                return Err(StoreError::Conflict {
                    entity: "mcp_runtime_session",
                    id: runtime_session_id.to_owned(),
                    detail:
                        "initialized notification conflicts with the negotiated protocol version"
                            .to_owned(),
                });
            }
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET negotiated_protocol_version = COALESCE(negotiated_protocol_version, ?2),
                    initialized_notification_at = COALESCE(initialized_notification_at, ?3),
                    last_observed_at = ?3
              WHERE runtime_session_id = ?1",
            params![runtime_session_id, negotiated_protocol_version, observed_at],
        )?;
        Ok(())
    })
}

/// Records one actual tools/list response and its required-tool-set fact.
pub fn record_mcp_tools_list(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    returned_tool_identities: &[String],
    required_tools_present: bool,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_timestamp("tools_list_observed_at", observed_at)?;
    let mut returned_tool_identities = returned_tool_identities.to_vec();
    returned_tool_identities.sort();
    returned_tool_identities.dedup();
    validate_returned_tool_identities(&returned_tool_identities)?;
    let returned_tool_identities_json =
        serde_json::to_string(&returned_tool_identities).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("returned MCP tool identities cannot be encoded: {error}"),
            }
        })?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.initialize_completed_at.is_none() {
            return Err(milestone_order(
                runtime_session_id,
                "tools/list requires initialize completion",
            ));
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET tools_list_observed_at = ?2, returned_tool_identities_json = ?3,
                    required_tools_present = ?4,
                    required_tools_validated_at = CASE WHEN ?4 = 1 THEN ?2 ELSE NULL END,
                    last_observed_at = ?2
              WHERE runtime_session_id = ?1",
            params![
                runtime_session_id,
                observed_at,
                returned_tool_identities_json,
                bool_i64(required_tools_present)
            ],
        )?;
        Ok(())
    })
}

/// Records successful completion of the exact tool selected for MCP verification.
pub fn record_mcp_verification_tool_observation(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    let verification_tool_name = ToolVerificationRole::ManagedHostRoundTrip
        .tool()
        .wire_name();
    validate_timestamp("verification_tool_observed_at", observed_at)?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.initialized_notification_at.is_none() {
            return Err(milestone_order(
                runtime_session_id,
                "verification tool success requires the initialized notification",
            ));
        }
        if prior.required_tools_present != Some(true) || prior.required_tools_validated_at.is_none()
        {
            return Err(milestone_order(
                runtime_session_id,
                "verification tool success requires same-session required-tool validation",
            ));
        }
        if prior.terminal_finding_id.is_some() {
            return Err(milestone_order(
                runtime_session_id,
                "verification tool success cannot follow terminal failure",
            ));
        }
        let connection = raw_agent_connection_record_from_conn(tx, &prior.connection_internal_id)?
            .ok_or_else(|| StoreError::NotFound {
                entity: "agent_connection",
                id: prior.connection_internal_id.clone(),
            })?;
        if !connection.enabled
            || prior.session_source != McpRuntimeSessionSource::ManagedHost
            || prior.connection_integration_revision
                != connection_integration_revision(&connection)?.as_str()
        {
            return Err(StoreError::Conflict {
                entity: "mcp_runtime_session",
                id: runtime_session_id.to_owned(),
                detail: "verification tool observation requires a current managed-host launch owned by an enabled Agent Connection".to_owned(),
            });
        }
        if prior
            .verification_tool_name
            .as_deref()
            .is_some_and(|prior_name| prior_name != verification_tool_name)
        {
            return Err(StoreError::Conflict {
                entity: "mcp_runtime_session",
                id: runtime_session_id.to_owned(),
                detail: "runtime session already records a different verification tool name"
                    .to_owned(),
            });
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET verification_tool_name = ?2,
                    verification_tool_observed_at = ?3,
                    last_observed_at = ?3
              WHERE runtime_session_id = ?1",
            params![runtime_session_id, verification_tool_name, observed_at],
        )?;
        Ok(())
    })
}

/// Atomically inserts and links one structured terminal finding.
pub fn record_mcp_terminal_finding(
    runtime_home: impl AsRef<Path>,
    finding: &OccurrenceDiagnosticFinding,
) -> StoreResult<McpRuntimeSessionRecord> {
    let runtime_session_id = finding
        .runtime_session_id()
        .map(|value| value.as_str())
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "terminal finding requires runtime_session_id".to_owned(),
        })?;
    insert_and_link_runtime_terminal_occurrence(&runtime_home, finding)?;
    mcp_runtime_session(runtime_home, runtime_session_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "mcp_runtime_session",
        id: runtime_session_id.to_owned(),
    })
}

/// Records observable graceful transport close. A duplicate close is idempotent.
pub fn record_mcp_graceful_close(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_timestamp("graceful_close_at", observed_at)?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.terminal_finding_id.is_some() {
            return Err(milestone_order(
                runtime_session_id,
                "graceful close cannot follow terminal failure",
            ));
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET graceful_close_at = COALESCE(graceful_close_at, ?2), last_observed_at = ?2
              WHERE runtime_session_id = ?1",
            params![runtime_session_id, observed_at],
        )?;
        Ok(())
    })
}

/// Returns the latest successful managed-host observation for the current revision.
/// CLI preflight sessions and diagnostics are structurally outside this lookup.
pub fn latest_successful_managed_runtime_session(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    let connection = raw_agent_connection_record_from_conn(&conn, connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })?;
    let revision = connection_integration_revision(&connection)?;
    conn.query_row(
        &format!(
            "{RUNTIME_SESSION_SELECT}
          WHERE connection_internal_id = ?1
            AND session_source = 'managed_host'
            AND connection_integration_revision = ?2
            AND initialize_completed_at IS NOT NULL
            AND initialized_notification_at IS NOT NULL
            AND tools_list_observed_at IS NOT NULL
            AND returned_tool_identities_json IS NOT NULL
            AND required_tools_validated_at IS NOT NULL
            AND verification_tool_name IS NOT NULL
            AND verification_tool_observed_at IS NOT NULL
          ORDER BY verification_tool_observed_at DESC, runtime_session_id DESC
          LIMIT 1"
        ),
        params![connection_internal_id, revision.as_str()],
        runtime_session_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_runtime_session).transpose())
}

/// Returns the latest managed-host observation for a Connection, including an
/// older integration revision or a session that stopped before all milestones.
/// CLI preflight sessions are structurally excluded.
pub fn latest_managed_runtime_session(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    conn.query_row(
        &format!(
            "{RUNTIME_SESSION_SELECT}
          WHERE connection_internal_id = ?1
            AND session_source = 'managed_host'
          ORDER BY last_observed_at DESC, runtime_session_id DESC
          LIMIT 1"
        ),
        params![connection_internal_id],
        runtime_session_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_runtime_session).transpose())
}

/// Returns the runtime session created by one observed child process for an
/// Agent Connection. Process identifiers are scoped by the Connection and the
/// latest matching start wins if the operating system has reused an ID.
pub fn mcp_runtime_session_for_process(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
    process_id: u32,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    conn.query_row(
        &format!(
            "{RUNTIME_SESSION_SELECT}
          WHERE connection_internal_id = ?1
            AND process_id = ?2
          ORDER BY process_started_at DESC, runtime_session_id DESC
          LIMIT 1"
        ),
        params![connection_internal_id, i64::from(process_id)],
        runtime_session_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_runtime_session).transpose())
}

/// Returns the latest managed-host observation for the Connection's current
/// integration revision, whether complete, in progress, or terminally failed.
/// CLI preflight sessions are structurally excluded.
pub fn latest_current_managed_runtime_session(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    let connection = raw_agent_connection_record_from_conn(&conn, connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })?;
    let revision = connection_integration_revision(&connection)?;
    conn.query_row(
        &format!(
            "{RUNTIME_SESSION_SELECT}
          WHERE connection_internal_id = ?1
            AND session_source = 'managed_host'
            AND connection_integration_revision = ?2
          ORDER BY last_observed_at DESC, runtime_session_id DESC
          LIMIT 1"
        ),
        params![connection_internal_id, revision.as_str()],
        runtime_session_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_runtime_session).transpose())
}

/// Returns every managed-host observation for the Connection's current revision.
/// CLI preflight sessions are structurally excluded.
pub fn current_managed_runtime_sessions(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Vec<McpRuntimeSessionRecord>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    let connection = raw_agent_connection_record_from_conn(&conn, connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })?;
    let revision = connection_integration_revision(&connection)?;
    let mut stmt = conn.prepare(&format!(
        "{RUNTIME_SESSION_SELECT}
          WHERE connection_internal_id = ?1
            AND session_source = 'managed_host'
            AND connection_integration_revision = ?2
          ORDER BY last_observed_at DESC, runtime_session_id DESC"
    ))?;
    let rows = stmt.query_map(
        params![connection_internal_id, revision.as_str()],
        runtime_session_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(validate_runtime_session)
        .collect()
}

fn update_session<F>(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    update: F,
) -> StoreResult<McpRuntimeSessionRecord>
where
    F: FnOnce(&rusqlite::Transaction<'_>, &McpRuntimeSessionRecord) -> StoreResult<()>,
{
    validate_text(
        "runtime_session_id",
        runtime_session_id,
        MAX_DIAGNOSTIC_FIELD_BYTES,
    )?;
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let prior = runtime_session_from_conn(&tx, runtime_session_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
        }
    })?;
    update(&tx, &prior)?;
    let record = runtime_session_from_conn(&tx, runtime_session_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
        }
    })?;
    tx.commit()?;
    Ok(record)
}

const RUNTIME_SESSION_SELECT: &str = "SELECT
    runtime_session_id, connection_internal_id, session_source,
    connection_integration_revision, observed_host_executable_version,
    attempted_client_name, attempted_client_version,
    requested_protocol_version, selected_protocol_version,
    negotiated_protocol_version, process_id,
    process_started_at, initialize_completed_at, initialized_notification_at,
    tools_list_observed_at, returned_tool_identities_json,
    required_tools_present, required_tools_validated_at,
    verification_tool_name, verification_tool_observed_at, last_observed_at,
    terminal_finding_id, graceful_close_at
  FROM mcp_runtime_sessions";

pub(crate) fn runtime_session_from_conn(
    conn: &Connection,
    runtime_session_id: &str,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    conn.query_row(
        &format!("{RUNTIME_SESSION_SELECT} WHERE runtime_session_id = ?1"),
        [runtime_session_id],
        runtime_session_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_runtime_session).transpose())
}

fn runtime_session_from_row(row: &Row<'_>) -> rusqlite::Result<McpRuntimeSessionRecord> {
    let source = match row.get::<_, String>(2)?.as_str() {
        "managed_host" => McpRuntimeSessionSource::ManagedHost,
        "manual_cli" => McpRuntimeSessionSource::ManualCli,
        "cli_preflight" => McpRuntimeSessionSource::CliPreflight,
        "integration_probe" => McpRuntimeSessionSource::IntegrationProbe,
        value => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                value.to_owned().into(),
            ))
        }
    };
    let process_id = u32::try_from(row.get::<_, i64>(10)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            10,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })?;
    let returned_tool_identities = row
        .get::<_, Option<String>>(15)?
        .map(|value| {
            let identities = serde_json::from_str::<Vec<String>>(&value).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    15,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            let canonical = serde_json::to_string(&identities).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    15,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            if canonical != value {
                return Err(rusqlite::Error::FromSqlConversionFailure(
                    15,
                    rusqlite::types::Type::Text,
                    "returned tool identities are not canonically encoded".into(),
                ));
            }
            Ok(identities)
        })
        .transpose()?;
    Ok(McpRuntimeSessionRecord {
        runtime_session_id: row.get(0)?,
        connection_internal_id: row.get(1)?,
        session_source: source,
        connection_integration_revision: row.get(3)?,
        observed_host_executable_version: row.get(4)?,
        attempted_client_name: row.get(5)?,
        attempted_client_version: row.get(6)?,
        requested_protocol_version: row.get(7)?,
        selected_protocol_version: row.get(8)?,
        negotiated_protocol_version: row.get(9)?,
        process_id,
        process_started_at: row.get(11)?,
        initialize_completed_at: row.get(12)?,
        initialized_notification_at: row.get(13)?,
        tools_list_observed_at: row.get(14)?,
        returned_tool_identities,
        required_tools_present: row.get::<_, Option<i64>>(16)?.map(|value| value == 1),
        required_tools_validated_at: row.get(17)?,
        verification_tool_name: row.get(18)?,
        verification_tool_observed_at: row.get(19)?,
        last_observed_at: row.get(20)?,
        terminal_finding_id: row.get(21)?,
        graceful_close_at: row.get(22)?,
    })
}

fn validate_runtime_session(
    record: McpRuntimeSessionRecord,
) -> StoreResult<McpRuntimeSessionRecord> {
    McpSessionMilestones::try_from_record(&record)?;
    if let Some(version) = record.observed_host_executable_version.as_deref() {
        validate_text(
            "observed_host_executable_version",
            version,
            MAX_DIAGNOSTIC_FIELD_BYTES,
        )
        .map_err(|_| corrupt(&record, "observed_host_executable_version"))?;
    }
    if let Some(graceful_close_at) = record.graceful_close_at.as_deref() {
        validate_timestamp("graceful_close_at", graceful_close_at)
            .map_err(|_| corrupt(&record, "graceful_close_at"))?;
    }
    if record.terminal_finding_id.is_some() && record.graceful_close_at.is_some() {
        return Err(corrupt(&record, "terminal_finding_id"));
    }
    Ok(record)
}

fn corrupt(record: &McpRuntimeSessionRecord, field: &'static str) -> StoreError {
    StoreError::CorruptOwnerStateValue {
        database_kind: "registry",
        table: "mcp_runtime_sessions",
        record_ref: record.runtime_session_id.clone(),
        logical_column: field,
    }
}

fn validate_start(input: &McpRuntimeSessionStart) -> StoreResult<()> {
    validate_text(
        "connection_internal_id",
        &input.connection_internal_id,
        MAX_DIAGNOSTIC_FIELD_BYTES,
    )?;
    if let Some(version) = input.observed_host_executable_version.as_deref() {
        validate_text(
            "observed_host_executable_version",
            version,
            MAX_DIAGNOSTIC_FIELD_BYTES,
        )?;
    }
    if input.process_id == 0 {
        return Err(StoreError::InvalidInput {
            detail: "process_id must be positive".to_owned(),
        });
    }
    validate_timestamp("process_started_at", &input.process_started_at)
}

fn validate_text(field: &'static str, value: &str, max: usize) -> StoreResult<()> {
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must be 1 through {max} non-control UTF-8 bytes"),
        });
    }
    Ok(())
}

fn validate_timestamp(field: &'static str, value: &str) -> StoreResult<()> {
    UtcTimestamp::from_str(value)
        .map(|_| ())
        .map_err(|_| StoreError::InvalidInput {
            detail: format!("{field} must be an RFC 3339 timestamp"),
        })
}

fn validate_mcp_tool_name(value: &str) -> StoreResult<()> {
    let valid = !value.is_empty()
        && value.len() <= MAX_MCP_TOOL_NAME_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'));
    if valid {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "verification_tool_name must be an MCP-compatible tool name of 1 through 128 ASCII bytes".to_owned(),
        })
    }
}

fn validate_returned_tool_identities(identities: &[String]) -> StoreResult<()> {
    if identities.len() > MAX_RETURNED_TOOL_IDENTITIES {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "returned MCP tool identities exceed the {MAX_RETURNED_TOOL_IDENTITIES}-item bound"
            ),
        });
    }
    for identity in identities {
        validate_mcp_tool_name(identity)?;
    }
    if identities
        .windows(2)
        .any(|pair| pair[0].as_bytes() >= pair[1].as_bytes())
    {
        return Err(StoreError::InvalidInput {
            detail: "returned MCP tool identities must be unique and sorted by UTF-8 bytes"
                .to_owned(),
        });
    }
    Ok(())
}

fn parse_stored_timestamp(
    record: &McpRuntimeSessionRecord,
    field: &'static str,
    value: &str,
) -> StoreResult<UtcTimestamp> {
    UtcTimestamp::parse(value).map_err(|_| corrupt(record, field))
}

fn parse_optional_stored_timestamp(
    record: &McpRuntimeSessionRecord,
    field: &'static str,
    value: Option<&str>,
) -> StoreResult<Option<UtcTimestamp>> {
    value
        .map(|value| parse_stored_timestamp(record, field, value))
        .transpose()
}

fn require_observation_time(
    record: &McpRuntimeSessionRecord,
    observed_at: &str,
) -> StoreResult<()> {
    if observed_at < record.last_observed_at.as_str() {
        Err(milestone_order(
            &record.runtime_session_id,
            "milestone timestamp precedes the last observation",
        ))
    } else {
        Ok(())
    }
}

fn milestone_order(runtime_session_id: &str, detail: &str) -> StoreError {
    StoreError::Conflict {
        entity: "mcp_runtime_session",
        id: runtime_session_id.to_owned(),
        detail: detail.to_owned(),
    }
}

const fn bool_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}
