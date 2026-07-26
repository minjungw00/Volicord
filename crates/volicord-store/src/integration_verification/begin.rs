use chrono::Duration;
use volicord_host_contract::{
    project_mcp_tool, HostContractProfileId, HostSessionId, HostTurnId, McpServerKey,
};
use volicord_types::guard_manifest::guard_manifest_from_json;
use volicord_types::ids::{
    AgentConnectionId, AgentRuntimeSessionId, DurableIdGenerator, DurableIdKind,
    GuardInstallationId, ProjectId, RandomDurableIdGenerator,
};
use volicord_types::integration_verification::{
    BeginIntegrationVerificationResult, GuardVerificationRepairReason,
    GuardVerificationRetryPolicy, GUARD_INTEGRATION_VERIFICATION_CLEANUP_SECONDS,
};
use volicord_types::tool_names::AgentToolId;

use super::{
    coordinate::{
        VerificationCallerCoordinate, VerificationCurrentCoordinate, VerificationStoredCoordinate,
    },
    correlation::prompt_event_matches,
    row::{
        insert_run, latest_run_for_project, mark_repair_required, parse_status, parse_timestamp,
        run_by_id, run_for_coordinate, NewVerificationRun,
    },
    status::begin_result_from_record,
    BeginGuardIntegrationVerificationInput, GuardIntegrationVerificationRunRecord,
    GuardVerificationCoordinate,
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
    sqlite::{begin_immediate_transaction, open_registry_database_for_mutation},
    RuntimeHomeMutationContext, StoreError, StoreResult,
};

/// Begins or idempotently resumes the exact current managed-session verification.
pub fn begin_guard_integration_verification(
    context: &RuntimeHomeMutationContext<'_>,
    input: BeginGuardIntegrationVerificationInput,
) -> StoreResult<BeginIntegrationVerificationResult> {
    let runtime_home = context.runtime_home().as_path();
    let observed_at = input.observed_at.clone();
    let record = begin_guard_integration_verification_with_generator(
        context,
        input,
        &RandomDurableIdGenerator,
    )?;
    begin_result_from_record(runtime_home, &record, &observed_at)
}

/// Deterministic-generator variant for durable tests.
pub fn begin_guard_integration_verification_with_generator(
    context: &RuntimeHomeMutationContext<'_>,
    input: BeginGuardIntegrationVerificationInput,
    generator: &dyn DurableIdGenerator,
) -> StoreResult<GuardIntegrationVerificationRunRecord> {
    let runtime_home = context.runtime_home().as_path();
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
    let host_contract_profile = HostContractProfileId::CodexCommandHooks;
    let expected_digest = host_contract_profile.contract_digest();
    if manifest.host_contract_profile != host_contract_profile.as_str()
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
    let cleanup_after = now
        .checked_add(Duration::seconds(
            GUARD_INTEGRATION_VERIFICATION_CLEANUP_SECONDS,
        ))
        .map_err(|_| StoreError::InvalidInput {
            detail: "verification cleanup bound is outside the supported timestamp range"
                .to_owned(),
        })?
        .to_canonical_string();
    let observation_policy = host_contract_profile
        .hook_observation_policy()
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "Guard hook contract has no semantic observation policy".to_owned(),
        })?;
    if observation_policy.allowed_status_reads() == 0 {
        return Err(StoreError::InvalidInput {
            detail: "Guard hook observation policy must allow a bounded status read".to_owned(),
        });
    }
    let semantic = GuardVerificationCoordinate {
        connection_id: AgentConnectionId::new(caller.connection_internal_id()),
        project_id: ProjectId::new(&project.project_id),
        runtime_session_id: AgentRuntimeSessionId::new(caller.runtime_session_id()),
        host_session_id: HostSessionId::parse(caller.host_session_id()).map_err(|_| {
            StoreError::InvalidInput {
                detail: "host_session_id must be a canonical host session identifier".to_owned(),
            }
        })?,
        host_turn_id: HostTurnId::parse(caller.host_turn_id()).map_err(|_| {
            StoreError::InvalidInput {
                detail: "host_turn_id must be a canonical host turn identifier".to_owned(),
            }
        })?,
        integration_revision: manifest.integration_revision.clone(),
        guard_installation_id: GuardInstallationId::new(&installation.guard_installation_id),
        host_contract_profile,
        hook_definition_digest: expected_digest,
        policy_digest: manifest.policy_hash.clone(),
    };
    let current = VerificationCurrentCoordinate::new(
        caller,
        &project.project_internal_id,
        semantic,
        expected_host_callable_name,
        observation_policy,
    );

    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    if let Some(existing) = run_for_coordinate(&tx, &current)? {
        VerificationStoredCoordinate::from_run(&existing)?.require_current(&current)?;
        tx.commit()?;
        return Ok(existing);
    }
    if let Some(previous) = latest_run_for_project(
        &tx,
        current.caller().connection_internal_id(),
        current.project_internal_id(),
    )? {
        let previous_status = parse_status(&previous.status)?;
        if matches!(
            previous_status,
            volicord_types::integration_verification::GuardIntegrationVerificationStatus::AwaitingProbe
                | volicord_types::integration_verification::GuardIntegrationVerificationStatus::AwaitingObservation
        ) {
            let (reason, retry_policy) = coordinate_change_repair(&previous, &current);
            mark_repair_required(
                &tx,
                &previous.verification_id,
                &input.observed_at,
                reason,
                retry_policy,
                reason.as_str(),
                "The immutable verification coordinate changed before the attempt completed.",
            )?;
        } else if previous_status
            == volicord_types::integration_verification::GuardIntegrationVerificationStatus::RepairRequired
        {
            require_retry_eligibility(&previous, &current)?;
        }
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
            cleanup_after: &cleanup_after,
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

fn coordinate_change_repair(
    previous: &GuardIntegrationVerificationRunRecord,
    current: &VerificationCurrentCoordinate,
) -> (GuardVerificationRepairReason, GuardVerificationRetryPolicy) {
    if previous.runtime_session_id != current.caller().runtime_session_id()
        || previous.host_session_id != current.caller().host_session_id()
    {
        return (
            GuardVerificationRepairReason::SessionMismatch,
            GuardVerificationRetryPolicy::HostReloadRequired,
        );
    }
    if previous.host_turn_id != current.caller().host_turn_id() {
        return (
            GuardVerificationRepairReason::TurnMismatch,
            GuardVerificationRetryPolicy::NewTurnRequired,
        );
    }
    if previous.integration_revision != current.integration_revision() {
        return (
            GuardVerificationRepairReason::IntegrationRevisionChanged,
            GuardVerificationRetryPolicy::RepairRequired,
        );
    }
    if previous.guard_installation_id != current.guard_installation_id()
        || previous.host_contract_profile != current.host_contract_profile().as_str()
        || previous.hook_definition_digest != current.hook_definition_digest()
    {
        return (
            GuardVerificationRepairReason::HookDefinitionChanged,
            GuardVerificationRetryPolicy::HookReviewRequired,
        );
    }
    if previous.policy_digest != current.policy_digest() {
        return (
            GuardVerificationRepairReason::PolicyChanged,
            GuardVerificationRetryPolicy::RepairRequired,
        );
    }
    (
        GuardVerificationRepairReason::HookEventNotObserved,
        GuardVerificationRetryPolicy::NoAutomaticRetry,
    )
}

fn require_retry_eligibility(
    previous: &GuardIntegrationVerificationRunRecord,
    current: &VerificationCurrentCoordinate,
) -> StoreResult<()> {
    let policy = previous
        .retry_policy
        .as_deref()
        .and_then(GuardVerificationRetryPolicy::from_storage_str)
        .ok_or_else(|| {
            StoreError::corrupt_stored_value(
                "registry",
                "guard_integration_verification_runs.retry_policy",
            )
        })?;
    let eligible = match policy {
        GuardVerificationRetryPolicy::NoAutomaticRetry => false,
        GuardVerificationRetryPolicy::NewTurnRequired => {
            previous.host_turn_id != current.caller().host_turn_id()
        }
        GuardVerificationRetryPolicy::HostReloadRequired => {
            previous.runtime_session_id != current.caller().runtime_session_id()
        }
        GuardVerificationRetryPolicy::HookReviewRequired => {
            previous.guard_installation_id != current.guard_installation_id()
                || previous.host_contract_profile != current.host_contract_profile().as_str()
                || previous.hook_definition_digest != current.hook_definition_digest()
        }
        GuardVerificationRetryPolicy::RepairRequired => {
            previous.integration_revision != current.integration_revision()
                || previous.guard_installation_id != current.guard_installation_id()
                || previous.hook_definition_digest != current.hook_definition_digest()
                || previous.policy_digest != current.policy_digest()
        }
    };
    if eligible {
        Ok(())
    } else {
        Err(current.caller().conflict(format!(
            "previous verification requires typed retry eligibility `{}`",
            policy.as_str()
        )))
    }
}
