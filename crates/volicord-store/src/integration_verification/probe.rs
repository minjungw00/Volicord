use chrono::Duration;
use volicord_host_contract::{
    HookObservationPolicy, HostContractProfileId, ObservationDeadlinePolicy,
};
use volicord_types::{
    GuardIntegrationVerificationId, GuardIntegrationVerificationStatus, GuardProbeResult,
};

use super::{
    coordinate::{VerificationCallerCoordinate, VerificationStoredCoordinate},
    observation::record_probe_acknowledgement,
    row::{acknowledge_probe_first_write, parse_timestamp, run_by_id},
    status::{effective_status, workflow_state_from_record},
    GuardIntegrationVerificationCaller,
};
use crate::{
    operational_sessions::current_managed_mcp_runtime_session_for_connection,
    sqlite::{begin_immediate_transaction, open_registry_database_for_mutation},
    RuntimeHomeMutationContext, StoreError, StoreResult,
};

/// Records the exact bounded, idempotent MCP probe acknowledgement.
pub fn acknowledge_guard_integration_probe(
    context: &RuntimeHomeMutationContext<'_>,
    verification_id: &str,
    caller: &GuardIntegrationVerificationCaller,
    observed_at: &str,
) -> StoreResult<GuardProbeResult> {
    let runtime_home = context.runtime_home().as_path();
    let caller = VerificationCallerCoordinate::from_caller(caller)?;
    let now = parse_timestamp("observed_at", observed_at)?;
    current_managed_mcp_runtime_session_for_connection(
        runtime_home,
        caller.runtime_session_id(),
        caller.connection_internal_id(),
    )?;
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let run = run_by_id(&tx, verification_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "guard_integration_verification",
        id: verification_id.to_owned(),
    })?;
    VerificationStoredCoordinate::from_run(&run)?.require_caller(&caller)?;
    let effective = effective_status(runtime_home, &run, &now)?;
    if run.probe_acknowledged_at.is_some() {
        let result = GuardProbeResult {
            verification_id: GuardIntegrationVerificationId::new(verification_id),
            workflow: workflow_state_from_record(&run, effective)?,
        };
        tx.commit()?;
        return Ok(result);
    }
    if effective != GuardIntegrationVerificationStatus::AwaitingProbe {
        return Err(terminal_state_conflict(&caller, effective));
    }
    let profile = HostContractProfileId::parse(&run.host_contract_profile).map_err(|_| {
        StoreError::corrupt_stored_value(
            "registry",
            "guard_integration_verification_runs.host_contract_profile",
        )
    })?;
    let observation_deadline_at = match profile.hook_observation_policy() {
        Some(HookObservationPolicy::Synchronous { .. }) => None,
        Some(HookObservationPolicy::Deferred {
            deadline: ObservationDeadlinePolicy::AfterProbeAcknowledgement { seconds },
            ..
        }) => Some(
            now.checked_add(Duration::seconds(i64::from(seconds)))
                .map_err(|_| StoreError::InvalidInput {
                    detail:
                        "verification observation deadline is outside the supported timestamp range"
                            .to_owned(),
                })?
                .to_canonical_string(),
        ),
        None => {
            return Err(StoreError::corrupt_stored_value(
                "registry",
                "guard_integration_verification_runs.host_contract_profile",
            ))
        }
    };
    acknowledge_probe_first_write(
        &tx,
        verification_id,
        observed_at,
        observation_deadline_at.as_deref(),
    )?;
    let authoritative = run_by_id(&tx, verification_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "guard_integration_verification",
        id: verification_id.to_owned(),
    })?;
    if authoritative.probe_acknowledged_at.is_none() {
        return Err(terminal_state_conflict(&caller, effective));
    }
    record_probe_acknowledgement(&tx, &authoritative, observed_at)?;
    let effective = effective_status(runtime_home, &authoritative, &now)?;
    let result = GuardProbeResult {
        verification_id: GuardIntegrationVerificationId::new(verification_id),
        workflow: workflow_state_from_record(&authoritative, effective)?,
    };
    tx.commit()?;
    Ok(result)
}

fn terminal_state_conflict(
    caller: &VerificationCallerCoordinate,
    status: GuardIntegrationVerificationStatus,
) -> StoreError {
    caller.conflict(
        format!("verification is {status:?} and has no prior probe acknowledgement")
            .to_ascii_lowercase(),
    )
}
