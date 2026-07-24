use std::path::Path;

use rusqlite::OptionalExtension;
use volicord_host_contract::HostContractProfileId;
use volicord_types::{
    guard_manifest_from_json, BeginIntegrationVerificationResult, GetIntegrationVerificationResult,
    GuardIntegrationVerificationFinding, GuardIntegrationVerificationId,
    GuardIntegrationVerificationPhaseStatus, GuardIntegrationVerificationPhases,
    GuardIntegrationVerificationStatus, GuardProbeObservationStage, GuardProbeToolReference,
    GuardVerificationRepairReason, GuardVerificationRetryPolicy, IntegrationRevision,
    IntegrationVerificationStatusToolReference, IntegrationVerificationWorkflowState, UtcTimestamp,
};

use super::{
    coordinate::{VerificationCallerCoordinate, VerificationStoredCoordinate},
    observation::observations_for_run,
    row::{
        increment_status_read_count, latest_run_for_connection, mark_repair_required, parse_status,
        parse_timestamp, run_by_id,
    },
    GuardIntegrationVerificationCaller, GuardIntegrationVerificationRunRecord,
};
use crate::{
    operational_sessions::current_managed_mcp_runtime_session_for_connection,
    sqlite::{
        begin_immediate_transaction, open_registry_database, open_registry_database_read_only,
        registry_db_path,
    },
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
    let mut conn = open_registry_database(registry_db_path(runtime_home))?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let mut run = run_by_id(&tx, verification_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "guard_integration_verification",
        id: verification_id.to_owned(),
    })?;
    VerificationStoredCoordinate::from_run(&run)?.require_caller(&caller)?;
    let status = parse_status(&run.status)?;
    if matches!(
        status,
        GuardIntegrationVerificationStatus::AwaitingProbe
            | GuardIntegrationVerificationStatus::AwaitingObservation
    ) {
        if let Some((reason, retry_policy)) = current_coordinate_repair(runtime_home, &tx, &run)? {
            persist_repair(&tx, &run, &now, reason, retry_policy)?;
            run = run_by_id(&tx, verification_id)?.ok_or_else(|| StoreError::NotFound {
                entity: "guard_integration_verification",
                id: verification_id.to_owned(),
            })?;
        } else if status == GuardIntegrationVerificationStatus::AwaitingObservation {
            increment_status_read_count(&tx, verification_id)?;
            run = run_by_id(&tx, verification_id)?.ok_or_else(|| StoreError::NotFound {
                entity: "guard_integration_verification",
                id: verification_id.to_owned(),
            })?;
            if observation_is_exhausted(&run, &now)? {
                let (reason, retry_policy) = observation_repair(&tx, &run, &now)?;
                persist_repair(&tx, &run, &now, reason, retry_policy)?;
                run = run_by_id(&tx, verification_id)?.ok_or_else(|| StoreError::NotFound {
                    entity: "guard_integration_verification",
                    id: verification_id.to_owned(),
                })?;
            }
        }
    }
    let workflow = workflow_state_from_record(&run, parse_status(&run.status)?)?;
    let result = result_from_record(&run, workflow);
    tx.commit()?;
    Ok(result)
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

/// Projects the persisted workflow state without consuming an observation read.
pub fn current_guard_integration_verification_workflow(
    runtime_home: impl AsRef<Path>,
    run: &GuardIntegrationVerificationRunRecord,
    observed_at: &str,
) -> StoreResult<IntegrationVerificationWorkflowState> {
    let _ = runtime_home.as_ref();
    parse_timestamp("observed_at", observed_at)?;
    workflow_state_from_record(run, parse_status(&run.status)?)
}

pub(super) fn effective_status(
    _runtime_home: &Path,
    run: &GuardIntegrationVerificationRunRecord,
    _now: &UtcTimestamp,
) -> StoreResult<GuardIntegrationVerificationStatus> {
    parse_status(&run.status)
}

pub(super) fn begin_result_from_record(
    runtime_home: &Path,
    run: &GuardIntegrationVerificationRunRecord,
    observed_at: &str,
) -> StoreResult<BeginIntegrationVerificationResult> {
    let _ = runtime_home;
    parse_timestamp("observed_at", observed_at)?;
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
        workflow: workflow_state_from_record(run, parse_status(&run.status)?)?,
        matched_prompt_event_id,
    })
}

pub(super) fn workflow_state_from_record(
    run: &GuardIntegrationVerificationRunRecord,
    status: GuardIntegrationVerificationStatus,
) -> StoreResult<IntegrationVerificationWorkflowState> {
    let completed_at = || {
        run.completed_at
            .as_deref()
            .ok_or_else(|| StoreError::CorruptStoredValue {
                database_kind: "registry",
                field: "guard_integration_verification_runs.completed_at",
            })
            .and_then(|value| parse_timestamp("completed_at", value))
    };
    match status {
        GuardIntegrationVerificationStatus::AwaitingProbe => {
            Ok(IntegrationVerificationWorkflowState::AwaitingProbe {
                tool: GuardProbeToolReference::new(),
            })
        }
        GuardIntegrationVerificationStatus::AwaitingObservation => {
            let acknowledged_at = run.probe_acknowledged_at.as_deref().ok_or_else(|| {
                StoreError::corrupt_stored_value(
                    "registry",
                    "guard_integration_verification_runs.probe_acknowledged_at",
                )
            })?;
            Ok(IntegrationVerificationWorkflowState::AwaitingObservation {
                tool: IntegrationVerificationStatusToolReference::new(),
                acknowledged_at: parse_timestamp("probe_acknowledged_at", acknowledged_at)?,
                remaining_status_reads: run
                    .allowed_status_reads
                    .saturating_sub(run.status_read_count),
            })
        }
        GuardIntegrationVerificationStatus::Complete => {
            Ok(IntegrationVerificationWorkflowState::Complete {
                completed_at: completed_at()?,
            })
        }
        GuardIntegrationVerificationStatus::RepairRequired => {
            let reason = run
                .repair_reason
                .as_deref()
                .and_then(GuardVerificationRepairReason::from_storage_str)
                .ok_or_else(|| {
                    StoreError::corrupt_stored_value(
                        "registry",
                        "guard_integration_verification_runs.repair_reason",
                    )
                })?;
            let retry_policy = run
                .retry_policy
                .as_deref()
                .and_then(GuardVerificationRetryPolicy::from_storage_str)
                .ok_or_else(|| {
                    StoreError::corrupt_stored_value(
                        "registry",
                        "guard_integration_verification_runs.retry_policy",
                    )
                })?;
            let finding = GuardIntegrationVerificationFinding {
                code: run.terminal_finding_code.clone().ok_or_else(|| {
                    StoreError::corrupt_stored_value(
                        "registry",
                        "guard_integration_verification_runs.terminal_finding_code",
                    )
                })?,
                summary: run.terminal_finding_summary.clone().ok_or_else(|| {
                    StoreError::corrupt_stored_value(
                        "registry",
                        "guard_integration_verification_runs.terminal_finding_summary",
                    )
                })?,
            };
            Ok(IntegrationVerificationWorkflowState::RepairRequired {
                reason,
                retry_policy,
                finding,
            })
        }
    }
}

fn current_coordinate_repair(
    runtime_home: &Path,
    conn: &rusqlite::Connection,
    run: &GuardIntegrationVerificationRunRecord,
) -> StoreResult<Option<(GuardVerificationRepairReason, GuardVerificationRetryPolicy)>> {
    if current_managed_mcp_runtime_session_for_connection(
        runtime_home,
        &run.runtime_session_id,
        &run.connection_internal_id,
    )
    .is_err()
    {
        return Ok(Some((
            GuardVerificationRepairReason::SessionMismatch,
            GuardVerificationRetryPolicy::HostReloadRequired,
        )));
    }
    let installation = conn
        .query_row(
            "SELECT connection_internal_id, project_internal_id, manifest_json
               FROM guard_installations
              WHERE guard_installation_id = ?1",
            [run.guard_installation_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((installation_connection_id, installation_project_id, manifest_json)) = installation
    else {
        return Ok(Some((
            GuardVerificationRepairReason::HookDefinitionChanged,
            GuardVerificationRetryPolicy::HookReviewRequired,
        )));
    };
    let manifest = guard_manifest_from_json(&manifest_json).map_err(|_| {
        StoreError::CorruptOwnerStateJson {
            database_kind: "registry",
            table: "guard_installations",
            record_ref: run.guard_installation_id.clone(),
            logical_column: "manifest_json",
        }
    })?;
    if installation_connection_id != run.connection_internal_id
        || installation_project_id != run.project_internal_id
        || manifest.integration_revision.as_str() != run.integration_revision
    {
        return Ok(Some((
            GuardVerificationRepairReason::IntegrationRevisionChanged,
            GuardVerificationRetryPolicy::RepairRequired,
        )));
    }
    if manifest.host_contract_profile != run.host_contract_profile
        || manifest.host_contract_digest != run.hook_definition_digest
    {
        return Ok(Some((
            GuardVerificationRepairReason::HookDefinitionChanged,
            GuardVerificationRetryPolicy::HookReviewRequired,
        )));
    }
    if manifest.policy_hash.as_str() != run.policy_digest {
        return Ok(Some((
            GuardVerificationRepairReason::PolicyChanged,
            GuardVerificationRetryPolicy::RepairRequired,
        )));
    }
    let profile = HostContractProfileId::parse(&run.host_contract_profile).map_err(|_| {
        StoreError::corrupt_stored_value(
            "registry",
            "guard_integration_verification_runs.host_contract_profile",
        )
    })?;
    if profile.contract_digest() != run.hook_definition_digest
        || profile.hook_observation_policy().is_none_or(|policy| {
            policy.kind() != run.observation_policy_kind
                || policy.allowed_status_reads() != run.allowed_status_reads
        })
    {
        return Ok(Some((
            GuardVerificationRepairReason::HookDefinitionChanged,
            GuardVerificationRetryPolicy::HookReviewRequired,
        )));
    }
    Ok(None)
}

fn observation_is_exhausted(
    run: &GuardIntegrationVerificationRunRecord,
    now: &UtcTimestamp,
) -> StoreResult<bool> {
    match run.observation_policy_kind.as_str() {
        "synchronous" => Ok(run.status_read_count >= run.allowed_status_reads),
        "deferred" => {
            let deadline = run.observation_deadline_at.as_deref().ok_or_else(|| {
                StoreError::corrupt_stored_value(
                    "registry",
                    "guard_integration_verification_runs.observation_deadline_at",
                )
            })?;
            Ok(
                parse_timestamp("observation_deadline_at", deadline)? <= *now
                    || run.status_read_count >= run.allowed_status_reads,
            )
        }
        _ => Err(StoreError::corrupt_stored_value(
            "registry",
            "guard_integration_verification_runs.observation_policy_kind",
        )),
    }
}

fn observation_repair(
    conn: &rusqlite::Connection,
    run: &GuardIntegrationVerificationRunRecord,
    now: &UtcTimestamp,
) -> StoreResult<(GuardVerificationRepairReason, GuardVerificationRetryPolicy)> {
    if run.observation_policy_kind == "deferred" {
        let deadline = run.observation_deadline_at.as_deref().ok_or_else(|| {
            StoreError::corrupt_stored_value(
                "registry",
                "guard_integration_verification_runs.observation_deadline_at",
            )
        })?;
        if parse_timestamp("observation_deadline_at", deadline)? <= *now {
            return Ok((
                GuardVerificationRepairReason::ObservationDeadlineExceeded,
                GuardVerificationRetryPolicy::NewTurnRequired,
            ));
        }
    }
    let stages = observations_for_run(conn, &run.verification_id)?
        .into_iter()
        .map(|observation| observation.stage)
        .collect::<Vec<_>>();
    for (stages_to_match, reason, retry_policy) in [
        (
            &[GuardProbeObservationStage::HookPayloadIncompatible][..],
            GuardVerificationRepairReason::HookPayloadIncompatible,
            GuardVerificationRetryPolicy::HookReviewRequired,
        ),
        (
            &[
                GuardProbeObservationStage::CallableIdentityUnknown,
                GuardProbeObservationStage::CallableIdentityMismatch,
            ][..],
            GuardVerificationRepairReason::CallableIdentityMismatch,
            GuardVerificationRetryPolicy::HookReviewRequired,
        ),
        (
            &[GuardProbeObservationStage::VerificationIdMismatch][..],
            GuardVerificationRepairReason::VerificationIdMismatch,
            GuardVerificationRetryPolicy::NewTurnRequired,
        ),
        (
            &[GuardProbeObservationStage::SessionMismatch][..],
            GuardVerificationRepairReason::SessionMismatch,
            GuardVerificationRetryPolicy::HostReloadRequired,
        ),
        (
            &[GuardProbeObservationStage::TurnMismatch][..],
            GuardVerificationRepairReason::TurnMismatch,
            GuardVerificationRetryPolicy::NewTurnRequired,
        ),
        (
            &[GuardProbeObservationStage::ToolUseMismatch][..],
            GuardVerificationRepairReason::ToolUseMismatch,
            GuardVerificationRetryPolicy::NewTurnRequired,
        ),
    ] {
        if stages.iter().any(|stage| stages_to_match.contains(stage)) {
            return Ok((reason, retry_policy));
        }
    }
    Ok((
        GuardVerificationRepairReason::HookEventNotObserved,
        GuardVerificationRetryPolicy::HostReloadRequired,
    ))
}

fn persist_repair(
    conn: &rusqlite::Connection,
    run: &GuardIntegrationVerificationRunRecord,
    now: &UtcTimestamp,
    reason: GuardVerificationRepairReason,
    retry_policy: GuardVerificationRetryPolicy,
) -> StoreResult<()> {
    let summary = match reason {
        GuardVerificationRepairReason::HookEventNotObserved => {
            "The synchronous Guard hook event was not observed."
        }
        GuardVerificationRepairReason::HookPayloadIncompatible => {
            "The Guard hook payload did not match the reviewed host contract."
        }
        GuardVerificationRepairReason::CallableIdentityMismatch => {
            "The routed host callable did not resolve to the expected AgentToolId."
        }
        GuardVerificationRepairReason::VerificationIdMismatch => {
            "The Guard hook carried a different verification ID."
        }
        GuardVerificationRepairReason::SessionMismatch => {
            "The Guard hook or current runtime session did not match this attempt."
        }
        GuardVerificationRepairReason::TurnMismatch => {
            "The Guard hook did not match this attempt's host turn."
        }
        GuardVerificationRepairReason::ToolUseMismatch => {
            "The Guard pre-tool and post-tool observations had different tool-use identities."
        }
        GuardVerificationRepairReason::IntegrationRevisionChanged => {
            "The managed integration revision changed during this attempt."
        }
        GuardVerificationRepairReason::HookDefinitionChanged => {
            "The Guard hook definition changed during this attempt."
        }
        GuardVerificationRepairReason::PolicyChanged => {
            "The Guard policy digest changed during this attempt."
        }
        GuardVerificationRepairReason::ObservationDeadlineExceeded => {
            "The host-contract observation deadline was exceeded."
        }
    };
    mark_repair_required(
        conn,
        &run.verification_id,
        &now.to_canonical_string(),
        reason,
        retry_policy,
        reason.as_str(),
        summary,
    )
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
