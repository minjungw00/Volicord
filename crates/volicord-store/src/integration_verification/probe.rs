use std::path::Path;

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
    sqlite::{begin_immediate_transaction, open_registry_database, registry_db_path},
    StoreError, StoreResult,
};

/// Records the exact bounded, idempotent MCP probe acknowledgement.
pub fn acknowledge_guard_integration_probe(
    runtime_home: impl AsRef<Path>,
    verification_id: &str,
    caller: &GuardIntegrationVerificationCaller,
    observed_at: &str,
) -> StoreResult<GuardProbeResult> {
    let runtime_home = runtime_home.as_ref();
    let caller = VerificationCallerCoordinate::from_caller(caller)?;
    let now = parse_timestamp("observed_at", observed_at)?;
    current_managed_mcp_runtime_session_for_connection(
        runtime_home,
        caller.runtime_session_id(),
        caller.connection_internal_id(),
    )?;
    let mut conn = open_registry_database(registry_db_path(runtime_home))?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let run = run_by_id(&tx, verification_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "guard_integration_verification",
        id: verification_id.to_owned(),
    })?;
    VerificationStoredCoordinate::from(&run).require_caller(&caller)?;
    let effective = effective_status(runtime_home, &run, &now)?;
    if run.probe_acknowledged_at.is_some() {
        let result = GuardProbeResult {
            verification_id: GuardIntegrationVerificationId::new(verification_id),
            workflow: workflow_state_from_record(&run, effective)?,
        };
        tx.commit()?;
        return Ok(result);
    }
    if effective != GuardIntegrationVerificationStatus::Active {
        return Err(terminal_state_conflict(&caller, effective));
    }
    acknowledge_probe_first_write(&tx, verification_id, observed_at)?;
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
