use std::path::Path;

use chrono::Duration;
use volicord_host_contract::{project_mcp_tool, HostContractProfileId, McpServerKey};
use volicord_types::{
    guard_manifest_from_json, AgentToolId, BeginIntegrationVerificationResult, DurableIdGenerator,
    DurableIdKind, RandomDurableIdGenerator, GUARD_INTEGRATION_VERIFICATION_TTL_SECONDS,
};

use super::{
    coordinate::{
        VerificationCallerCoordinate, VerificationCurrentCoordinate, VerificationStoredCoordinate,
    },
    correlation::prompt_event_matches,
    row::{
        expire_active_runs, insert_run, parse_timestamp, resumable_run, run_by_id,
        NewVerificationRun,
    },
    status::begin_result_from_record,
    BeginGuardIntegrationVerificationInput, GuardIntegrationVerificationRunRecord,
};
use crate::{
    agent_connections::agent_connection_record_read_only,
    bootstrap::project_record_for_execution,
    guards::{
        agent_session, agent_session_matches_current_integration,
        guard_events_for_integration_verification, list_guard_installations,
        GuardIntegrationVerificationEventQuery,
    },
    operational_sessions::current_managed_mcp_runtime_session_for_connection,
    sqlite::{begin_immediate_transaction, open_registry_database, registry_db_path},
    StoreError, StoreResult,
};

/// Begins or idempotently resumes the exact current managed-session verification.
pub fn begin_guard_integration_verification(
    runtime_home: impl AsRef<Path>,
    input: BeginGuardIntegrationVerificationInput,
) -> StoreResult<BeginIntegrationVerificationResult> {
    let runtime_home = runtime_home.as_ref();
    let observed_at = input.observed_at.clone();
    let record = begin_guard_integration_verification_with_generator(
        runtime_home,
        input,
        &RandomDurableIdGenerator,
    )?;
    begin_result_from_record(runtime_home, &record, &observed_at)
}

/// Deterministic-generator variant for durable tests.
pub fn begin_guard_integration_verification_with_generator(
    runtime_home: impl AsRef<Path>,
    input: BeginGuardIntegrationVerificationInput,
    generator: &dyn DurableIdGenerator,
) -> StoreResult<GuardIntegrationVerificationRunRecord> {
    let runtime_home = runtime_home.as_ref();
    let caller = VerificationCallerCoordinate::from_caller(&input.caller)?;
    let now = parse_timestamp("observed_at", &input.observed_at)?;
    current_managed_mcp_runtime_session_for_connection(
        runtime_home,
        caller.runtime_session_id(),
        caller.connection_internal_id(),
    )?;
    let project =
        project_record_for_execution(runtime_home, &input.project_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "project",
                id: input.project_id.clone(),
            }
        })?;
    let session = agent_session(runtime_home, &project.project_id, &input.project_session_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_session",
            id: input.project_session_id.clone(),
        })?;
    if session.connection_internal_id != caller.connection_internal_id()
        || session.runtime_session_id.as_deref() != Some(caller.runtime_session_id())
        || session.host_session_id != caller.host_session_id()
        || session.last_host_turn_id != caller.host_turn_id()
    {
        return Err(caller.conflict(
            "project Agent Session does not match the current managed runtime and native turn",
        ));
    }
    let installations = list_guard_installations(
        runtime_home,
        caller.connection_internal_id(),
        Some(&project.project_id),
    )?;
    let [installation] = installations.as_slice() else {
        return Err(StoreError::Conflict {
            entity: "guard_integration_verification",
            id: caller.runtime_session_id().to_owned(),
            detail: "verification requires exactly one current Guard installation".to_owned(),
        });
    };
    if !agent_session_matches_current_integration(
        runtime_home,
        &session,
        Some(&installation.guard_installation_id),
    )? {
        return Err(
            caller.conflict("project Agent Session does not match the current Guard installation")
        );
    }
    let manifest = guard_manifest_from_json(&installation.manifest_json).map_err(|_| {
        StoreError::CorruptOwnerStateJson {
            database_kind: "registry",
            table: "guard_installations",
            record_ref: installation.guard_installation_id.clone(),
            logical_column: "manifest_json",
        }
    })?;
    let expected_digest = HostContractProfileId::CodexCommandHooks.contract_digest();
    if manifest.host_contract_profile != HostContractProfileId::CodexCommandHooks.as_str()
        || manifest.host_contract_digest != expected_digest
    {
        return Err(StoreError::Conflict {
            entity: "guard_integration_verification",
            id: caller.runtime_session_id().to_owned(),
            detail: "Guard installation revision or hook contract is not current".to_owned(),
        });
    }
    let connection =
        agent_connection_record_read_only(runtime_home, caller.connection_internal_id())?
            .ok_or_else(|| StoreError::NotFound {
                entity: "agent_connection",
                id: caller.connection_internal_id().to_owned(),
            })?;
    let server = McpServerKey::parse(connection.server_name).map_err(|_| {
        StoreError::corrupt_stored_value("registry", "agent_connections.server_name")
    })?;
    let expected_host_callable_name = project_mcp_tool(&server, AgentToolId::GUARD_PROBE)
        .map_err(|_| StoreError::corrupt_stored_value("registry", "agent_connections.server_name"))?
        .callable_name()
        .as_str()
        .to_owned();
    let events = guard_events_for_integration_verification(
        runtime_home,
        GuardIntegrationVerificationEventQuery {
            project_id: &project.project_id,
            connection_internal_id: caller.connection_internal_id(),
            session_id: &session.session_id,
            host_turn_id: caller.host_turn_id(),
            guard_installation_id: &installation.guard_installation_id,
            policy_hash: manifest.policy_hash.as_str(),
            integration_revision: manifest.integration_revision.as_str(),
        },
    )?;
    let prompt = events
        .iter()
        .rfind(|event| {
            prompt_event_matches(event, &expected_digest)
                && parse_timestamp("occurred_at", &event.occurred_at).is_ok_and(|at| at <= now)
        })
        .ok_or_else(|| StoreError::Conflict {
            entity: "guard_integration_verification",
            id: caller.host_turn_id().to_owned(),
            detail: "the current native turn has no compatible prompt-capture event".to_owned(),
        })?;
    let expires_at = now
        .checked_add(Duration::seconds(
            GUARD_INTEGRATION_VERIFICATION_TTL_SECONDS,
        ))
        .map_err(|_| StoreError::InvalidInput {
            detail: "verification expiry is outside the supported timestamp range".to_owned(),
        })?
        .to_canonical_string();
    let current = VerificationCurrentCoordinate::new(
        caller,
        &project.project_internal_id,
        &installation.guard_installation_id,
        manifest.integration_revision.as_str(),
        manifest.policy_hash.as_str(),
        &expected_digest,
        expected_host_callable_name,
    );

    let mut conn = open_registry_database(registry_db_path(runtime_home))?;
    let tx = begin_immediate_transaction(&mut conn)?;
    expire_active_runs(&tx, &input.observed_at)?;
    if let Some(existing) = resumable_run(&tx, &current)? {
        VerificationStoredCoordinate::from(&existing).require_current(&current)?;
        tx.commit()?;
        return Ok(existing);
    }
    let verification_id = generator
        .generate(DurableIdKind::GuardIntegrationVerification)
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("could not generate verification ID: {error}"),
        })?;
    insert_run(
        &tx,
        NewVerificationRun {
            verification_id: &verification_id,
            coordinate: &current,
            created_at: &input.observed_at,
            expires_at: &expires_at,
            matched_prompt_event_id: &prompt.guard_event_id,
        },
    )?;
    let record = run_by_id(&tx, &verification_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "guard_integration_verification",
        id: verification_id,
    })?;
    tx.commit()?;
    Ok(record)
}
