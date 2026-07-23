use std::path::Path;

use volicord_host_contract::HostContractProfileId;
use volicord_types::{
    guard_manifest_from_json, BeginIntegrationVerificationResult,
    BeginIntegrationVerificationToolReference, GetIntegrationVerificationResult,
    GuardIntegrationVerificationFinding, GuardIntegrationVerificationId,
    GuardIntegrationVerificationPhaseStatus, GuardIntegrationVerificationPhases,
    GuardIntegrationVerificationStatus, GuardProbeToolReference, IntegrationRevision,
    IntegrationVerificationRestartReason, IntegrationVerificationStatusToolReference,
    IntegrationVerificationWorkflowState, UtcTimestamp,
};

use super::{
    coordinate::{VerificationCallerCoordinate, VerificationStoredCoordinate},
    row::{latest_run_for_connection, parse_status, parse_timestamp, run_by_id},
    GuardIntegrationVerificationCaller, GuardIntegrationVerificationRunRecord,
};
use crate::{
    operational_sessions::current_managed_mcp_runtime_session_for_connection,
    sqlite::{open_registry_database_read_only, registry_db_path},
    StoreError, StoreResult,
};

/// Returns one verification using only its exact current managed caller coordinate.
pub fn get_guard_integration_verification(
    runtime_home: impl AsRef<Path>,
    verification_id: &str,
    caller: &GuardIntegrationVerificationCaller,
    observed_at: &str,
) -> StoreResult<GetIntegrationVerificationResult> {
    let runtime_home = runtime_home.as_ref();
    let caller = VerificationCallerCoordinate::from_caller(caller)?;
    let now = parse_timestamp("observed_at", observed_at)?;
    let conn = open_registry_database_read_only(registry_db_path(runtime_home))?;
    let run = run_by_id(&conn, verification_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "guard_integration_verification",
        id: verification_id.to_owned(),
    })?;
    VerificationStoredCoordinate::from(&run).require_caller(&caller)?;
    let effective = effective_status(runtime_home, &run, &now)?;
    let workflow = workflow_state_from_record(&run, effective)?;
    Ok(result_from_record(&run, workflow))
}

/// Reads the newest verification row for the current Connection revision.
pub fn latest_guard_integration_verification_for_connection(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
    integration_revision: &IntegrationRevision,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    latest_run_for_connection(&conn, connection_internal_id, integration_revision.as_str())
}

/// Projects the current authoritative workflow state without mutating the run.
pub fn current_guard_integration_verification_workflow(
    runtime_home: impl AsRef<Path>,
    run: &GuardIntegrationVerificationRunRecord,
    observed_at: &str,
) -> StoreResult<IntegrationVerificationWorkflowState> {
    let effective = effective_status(
        runtime_home.as_ref(),
        run,
        &parse_timestamp("observed_at", observed_at)?,
    )?;
    workflow_state_from_record(run, effective)
}

pub(super) fn effective_status(
    runtime_home: &Path,
    run: &GuardIntegrationVerificationRunRecord,
    now: &UtcTimestamp,
) -> StoreResult<GuardIntegrationVerificationStatus> {
    let stored = parse_status(&run.status)?;
    if matches!(
        stored,
        GuardIntegrationVerificationStatus::Failed | GuardIntegrationVerificationStatus::Expired
    ) {
        return Ok(stored);
    }
    if stored == GuardIntegrationVerificationStatus::Active
        && parse_timestamp("expires_at", &run.expires_at)? <= *now
    {
        return Ok(GuardIntegrationVerificationStatus::Expired);
    }
    if current_managed_mcp_runtime_session_for_connection(
        runtime_home,
        &run.runtime_session_id,
        &run.connection_internal_id,
    )
    .is_err()
    {
        return Ok(GuardIntegrationVerificationStatus::Failed);
    }
    let Some(installation) =
        crate::guards::guard_installation(runtime_home, &run.guard_installation_id)?
    else {
        return Ok(GuardIntegrationVerificationStatus::Failed);
    };
    let manifest = match guard_manifest_from_json(&installation.manifest_json) {
        Ok(manifest) => manifest,
        Err(_) => return Ok(GuardIntegrationVerificationStatus::Failed),
    };
    if installation.connection_internal_id != run.connection_internal_id
        || installation.project_internal_id != run.project_internal_id
        || manifest.integration_revision.as_str() != run.integration_revision
        || manifest.policy_hash.as_str() != run.policy_hash
        || manifest.host_contract_digest != run.hook_contract_digest
        || run.hook_contract_digest != HostContractProfileId::CodexHooksV1.contract_digest()
    {
        return Ok(GuardIntegrationVerificationStatus::Failed);
    }
    Ok(stored)
}

pub(super) fn begin_result_from_record(
    runtime_home: &Path,
    run: &GuardIntegrationVerificationRunRecord,
    observed_at: &str,
) -> StoreResult<BeginIntegrationVerificationResult> {
    let effective = effective_status(
        runtime_home,
        run,
        &parse_timestamp("observed_at", observed_at)?,
    )?;
    let matched_prompt_event_id = run
        .matched_prompt_event_id
        .as_deref()
        .map(volicord_types::GuardEventId::new)
        .ok_or_else(|| StoreError::CorruptStoredValue {
            database_kind: "registry",
            field: "guard_integration_verification_runs.matched_prompt_event_id",
        })?;
    Ok(BeginIntegrationVerificationResult {
        verification_id: GuardIntegrationVerificationId::new(&run.verification_id),
        workflow: workflow_state_from_record(run, effective)?,
        matched_prompt_event_id,
    })
}

pub(super) fn workflow_state_from_record(
    run: &GuardIntegrationVerificationRunRecord,
    status: GuardIntegrationVerificationStatus,
) -> StoreResult<IntegrationVerificationWorkflowState> {
    let expires_at = || parse_timestamp("expires_at", &run.expires_at);
    let completed_at = || {
        run.completed_at
            .as_deref()
            .ok_or_else(|| StoreError::CorruptStoredValue {
                database_kind: "registry",
                field: "guard_integration_verification_runs.completed_at",
            })
            .and_then(|value| parse_timestamp("completed_at", value))
    };
    let finding = match status {
        GuardIntegrationVerificationStatus::Failed => Some(GuardIntegrationVerificationFinding {
            code: run
                .terminal_finding_code
                .clone()
                .unwrap_or_else(|| "verification_coordinate_stale".to_owned()),
            summary: run.terminal_finding_summary.clone().unwrap_or_else(|| {
                "The managed runtime, Guard installation, policy, revision, or hook definition is no longer current."
                    .to_owned()
            }),
        }),
        GuardIntegrationVerificationStatus::Expired => {
            Some(GuardIntegrationVerificationFinding {
                code: run
                    .terminal_finding_code
                    .clone()
                    .unwrap_or_else(|| "verification_expired".to_owned()),
                summary: run
                    .terminal_finding_summary
                    .clone()
                    .unwrap_or_else(|| "The bounded integration-verification window expired.".to_owned()),
            })
        }
        GuardIntegrationVerificationStatus::Active
        | GuardIntegrationVerificationStatus::Passed => None,
    };
    match status {
        GuardIntegrationVerificationStatus::Active => {
            if let Some(acknowledged_at) = run.probe_acknowledged_at.as_deref() {
                Ok(
                    IntegrationVerificationWorkflowState::AwaitingHookCompletion {
                        tool: IntegrationVerificationStatusToolReference::new(),
                        acknowledged_at: parse_timestamp("probe_acknowledged_at", acknowledged_at)?,
                        expires_at: expires_at()?,
                    },
                )
            } else {
                Ok(IntegrationVerificationWorkflowState::AwaitingProbe {
                    tool: GuardProbeToolReference::new(),
                    expires_at: expires_at()?,
                })
            }
        }
        GuardIntegrationVerificationStatus::Passed => {
            Ok(IntegrationVerificationWorkflowState::Complete {
                completed_at: completed_at()?,
            })
        }
        GuardIntegrationVerificationStatus::Failed => {
            Ok(IntegrationVerificationWorkflowState::RestartRequired {
                reason: IntegrationVerificationRestartReason::Failed,
                tool: BeginIntegrationVerificationToolReference::new(),
                finding,
            })
        }
        GuardIntegrationVerificationStatus::Expired => {
            Ok(IntegrationVerificationWorkflowState::RestartRequired {
                reason: IntegrationVerificationRestartReason::Expired,
                tool: BeginIntegrationVerificationToolReference::new(),
                finding,
            })
        }
    }
}

pub(super) fn result_from_record(
    run: &GuardIntegrationVerificationRunRecord,
    workflow: IntegrationVerificationWorkflowState,
) -> GetIntegrationVerificationResult {
    let phase = |value: &Option<String>| {
        if value.is_some() {
            GuardIntegrationVerificationPhaseStatus::Matched
        } else {
            GuardIntegrationVerificationPhaseStatus::Pending
        }
    };
    GetIntegrationVerificationResult {
        verification_id: GuardIntegrationVerificationId::new(&run.verification_id),
        workflow,
        guard_phases: GuardIntegrationVerificationPhases {
            prompt_capture: phase(&run.matched_prompt_event_id),
            pre_tool: phase(&run.matched_pre_tool_event_id),
            post_tool: phase(&run.matched_post_tool_event_id),
        },
        matched_prompt_event_id: run
            .matched_prompt_event_id
            .as_deref()
            .map(volicord_types::GuardEventId::new),
        matched_pre_tool_event_id: run
            .matched_pre_tool_event_id
            .as_deref()
            .map(volicord_types::GuardEventId::new),
        matched_post_tool_event_id: run
            .matched_post_tool_event_id
            .as_deref()
            .map(volicord_types::GuardEventId::new),
    }
}
