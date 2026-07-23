//! Durable, session-coherent Guard integration verification.

use std::path::Path;

use chrono::Duration;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde_json::Value;
use volicord_host_contract::{codex_hook_tool_name, HostContractProfileId, HostNativeCorrelation};
use volicord_types::{
    guard_manifest_from_json, AgentToolId, DurableIdGenerator, DurableIdKind,
    GetIntegrationVerificationResult, GuardIntegrationVerificationFinding,
    GuardIntegrationVerificationId, GuardIntegrationVerificationPhaseStatus,
    GuardIntegrationVerificationPhases, GuardIntegrationVerificationStatus, GuardProbeResult,
    IntegrationRevision, RandomDurableIdGenerator, UtcTimestamp,
    GUARD_INTEGRATION_VERIFICATION_TTL_SECONDS,
};

use crate::{
    bootstrap::project_record_for_execution,
    guards::{
        agent_session, agent_session_matches_current_integration, guard_event,
        guard_events_for_integration_verification, list_guard_installations, GuardEventRecord,
        GuardIntegrationVerificationEventQuery,
    },
    operational_sessions::current_managed_mcp_runtime_session_for_connection,
    sqlite::{
        begin_immediate_transaction, open_registry_database, open_registry_database_read_only,
        registry_db_path,
    },
    StoreError, StoreResult,
};

const COMPATIBLE_CONTRACT: &str = "compatible";
const ACTIVE_STATUS: &str = "active";

/// Exact managed caller coordinate supplied by the MCP session boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardIntegrationVerificationCaller {
    pub connection_internal_id: String,
    pub runtime_session_id: String,
    pub host_session_id: String,
    pub host_turn_id: String,
}

/// Input used to create or resume one verification run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeginGuardIntegrationVerificationInput {
    pub caller: GuardIntegrationVerificationCaller,
    pub project_id: String,
    pub project_session_id: String,
    pub observed_at: String,
}

/// Registry-owned durable verification row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardIntegrationVerificationRunRecord {
    pub verification_id: String,
    pub connection_internal_id: String,
    pub project_internal_id: String,
    pub runtime_session_id: String,
    pub host_session_id: String,
    pub host_turn_id: String,
    pub guard_installation_id: String,
    pub integration_revision: String,
    pub policy_hash: String,
    pub hook_contract_digest: String,
    pub expected_probe_tool: String,
    pub created_at: String,
    pub expires_at: String,
    pub status: String,
    pub probe_acknowledged_at: Option<String>,
    pub completed_at: Option<String>,
    pub matched_prompt_event_id: Option<String>,
    pub matched_pre_tool_event_id: Option<String>,
    pub matched_post_tool_event_id: Option<String>,
    pub terminal_finding_code: Option<String>,
    pub terminal_finding_summary: Option<String>,
}

/// Begins or idempotently resumes the exact current managed-session verification.
pub fn begin_guard_integration_verification(
    runtime_home: impl AsRef<Path>,
    input: BeginGuardIntegrationVerificationInput,
) -> StoreResult<GuardIntegrationVerificationRunRecord> {
    begin_guard_integration_verification_with_generator(
        runtime_home,
        input,
        &RandomDurableIdGenerator,
    )
}

/// Deterministic-generator variant for durable tests.
pub fn begin_guard_integration_verification_with_generator(
    runtime_home: impl AsRef<Path>,
    input: BeginGuardIntegrationVerificationInput,
    generator: &dyn DurableIdGenerator,
) -> StoreResult<GuardIntegrationVerificationRunRecord> {
    let runtime_home = runtime_home.as_ref();
    validate_caller(&input.caller)?;
    let now = parse_timestamp("observed_at", &input.observed_at)?;
    current_managed_mcp_runtime_session_for_connection(
        runtime_home,
        &input.caller.runtime_session_id,
        &input.caller.connection_internal_id,
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
    if session.connection_internal_id != input.caller.connection_internal_id
        || session.runtime_session_id.as_deref() != Some(input.caller.runtime_session_id.as_str())
        || session.host_session_id != input.caller.host_session_id
        || session.last_host_turn_id != input.caller.host_turn_id
    {
        return Err(coordinate_conflict(
            &input.caller,
            "project Agent Session does not match the current managed runtime and native turn",
        ));
    }
    let installations = list_guard_installations(
        runtime_home,
        &input.caller.connection_internal_id,
        Some(&project.project_id),
    )?;
    let [installation] = installations.as_slice() else {
        return Err(StoreError::Conflict {
            entity: "guard_integration_verification",
            id: input.caller.runtime_session_id.clone(),
            detail: "verification requires exactly one current Guard installation".to_owned(),
        });
    };
    if !agent_session_matches_current_integration(
        runtime_home,
        &session,
        Some(&installation.guard_installation_id),
    )? {
        return Err(coordinate_conflict(
            &input.caller,
            "project Agent Session does not match the current Guard installation",
        ));
    }
    let manifest = guard_manifest_from_json(&installation.manifest_json).map_err(|_| {
        StoreError::CorruptOwnerStateJson {
            database_kind: "registry",
            table: "guard_installations",
            record_ref: installation.guard_installation_id.clone(),
            logical_column: "manifest_json",
        }
    })?;
    let expected_digest = HostContractProfileId::CodexHooksV1.contract_digest();
    if manifest.host_contract_profile != HostContractProfileId::CodexHooksV1.as_str()
        || manifest.host_contract_digest != expected_digest
    {
        return Err(StoreError::Conflict {
            entity: "guard_integration_verification",
            id: input.caller.runtime_session_id.clone(),
            detail: "Guard installation revision or hook contract is not current".to_owned(),
        });
    }
    let events = guard_events_for_integration_verification(
        runtime_home,
        GuardIntegrationVerificationEventQuery {
            project_id: &project.project_id,
            connection_internal_id: &input.caller.connection_internal_id,
            session_id: &session.session_id,
            host_turn_id: &input.caller.host_turn_id,
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
            id: input.caller.host_turn_id.clone(),
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

    let mut conn = open_registry_database(registry_db_path(runtime_home))?;
    let tx = begin_immediate_transaction(&mut conn)?;
    expire_active_runs(&tx, &input.observed_at)?;
    if let Some(existing) = resumable_run_for_coordinate(
        &tx,
        &input.caller.connection_internal_id,
        &input.caller.runtime_session_id,
        &input.caller.host_turn_id,
        manifest.integration_revision.as_str(),
    )? {
        require_run_coordinate(
            &existing,
            &input.caller,
            &project.project_internal_id,
            &installation.guard_installation_id,
            manifest.policy_hash.as_str(),
            &expected_digest,
        )?;
        tx.commit()?;
        return Ok(existing);
    }
    let verification_id = generator
        .generate(DurableIdKind::GuardIntegrationVerification)
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("could not generate verification ID: {error}"),
        })?;
    tx.execute(
        "INSERT INTO guard_integration_verification_runs (
            verification_id, connection_internal_id, project_internal_id,
            runtime_session_id, host_session_id, host_turn_id,
            guard_installation_id, integration_revision, policy_hash,
            hook_contract_digest, expected_probe_tool, created_at, expires_at,
            status, matched_prompt_event_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                   'active', ?14)",
        params![
            verification_id,
            input.caller.connection_internal_id,
            project.project_internal_id,
            input.caller.runtime_session_id,
            input.caller.host_session_id,
            input.caller.host_turn_id,
            installation.guard_installation_id,
            manifest.integration_revision.as_str(),
            manifest.policy_hash.as_str(),
            expected_digest,
            AgentToolId::GUARD_PROBE.wire_name(),
            input.observed_at,
            expires_at,
            prompt.guard_event_id,
        ],
    )?;
    let record = run_from_conn(&tx, &verification_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "guard_integration_verification",
        id: verification_id,
    })?;
    tx.commit()?;
    Ok(record)
}

/// Records the exact bounded, idempotent MCP probe acknowledgement.
pub fn acknowledge_guard_integration_probe(
    runtime_home: impl AsRef<Path>,
    verification_id: &str,
    caller: &GuardIntegrationVerificationCaller,
    observed_at: &str,
) -> StoreResult<GuardProbeResult> {
    let runtime_home = runtime_home.as_ref();
    validate_caller(caller)?;
    let now = parse_timestamp("observed_at", observed_at)?;
    current_managed_mcp_runtime_session_for_connection(
        runtime_home,
        &caller.runtime_session_id,
        &caller.connection_internal_id,
    )?;
    let mut conn = open_registry_database(registry_db_path(runtime_home))?;
    let tx = begin_immediate_transaction(&mut conn)?;
    expire_active_runs(&tx, observed_at)?;
    let run = run_from_conn(&tx, verification_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "guard_integration_verification",
        id: verification_id.to_owned(),
    })?;
    require_active_caller(&run, caller, &now)?;
    let acknowledged_at = run
        .probe_acknowledged_at
        .clone()
        .unwrap_or_else(|| observed_at.to_owned());
    tx.execute(
        "UPDATE guard_integration_verification_runs
            SET probe_acknowledged_at = COALESCE(probe_acknowledged_at, ?2)
          WHERE verification_id = ?1 AND status = 'active'",
        params![verification_id, observed_at],
    )?;
    tx.commit()?;
    Ok(GuardProbeResult {
        verification_id: GuardIntegrationVerificationId::new(verification_id),
        status: GuardIntegrationVerificationStatus::Active,
        acknowledged_at: parse_timestamp("probe_acknowledged_at", &acknowledged_at)?,
    })
}

/// Returns one verification using only its exact current managed caller coordinate.
pub fn get_guard_integration_verification(
    runtime_home: impl AsRef<Path>,
    verification_id: &str,
    caller: &GuardIntegrationVerificationCaller,
    observed_at: &str,
) -> StoreResult<GetIntegrationVerificationResult> {
    let runtime_home = runtime_home.as_ref();
    validate_caller(caller)?;
    let now = parse_timestamp("observed_at", observed_at)?;
    let conn = open_registry_database_read_only(registry_db_path(runtime_home))?;
    let run = run_from_conn(&conn, verification_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "guard_integration_verification",
        id: verification_id.to_owned(),
    })?;
    if run.connection_internal_id != caller.connection_internal_id
        || run.runtime_session_id != caller.runtime_session_id
        || run.host_session_id != caller.host_session_id
        || run.host_turn_id != caller.host_turn_id
    {
        return Err(coordinate_conflict(
            caller,
            "verification belongs to another managed session or native turn",
        ));
    }
    let effective = effective_status(runtime_home, &run, &now)?;
    Ok(result_from_record(&run, effective))
}

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
    let run = tx
        .query_row(
            &format!(
                "{RUN_SELECT}
                  WHERE connection_internal_id = ?1
                    AND host_session_id = ?2
                    AND host_turn_id = ?3
                    AND guard_installation_id = ?4
                    AND integration_revision = ?5
                    AND policy_hash = ?6
                    AND status = 'active'
                  ORDER BY created_at DESC, verification_id DESC
                  LIMIT 1"
            ),
            params![
                trigger.connection_internal_id,
                correlation.session_id().as_str(),
                correlation.turn_id().as_str(),
                trigger.guard_installation_id,
                trigger.integration_revision,
                trigger.policy_hash,
            ],
            run_from_row,
        )
        .optional()?;
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
    let Some((prompt, pre, post)) = correlated_event_triple(&run, &events)? else {
        tx.commit()?;
        return Ok(Some(run));
    };
    tx.execute(
        "UPDATE guard_integration_verification_runs
            SET status = 'passed', completed_at = ?2,
                matched_prompt_event_id = ?3,
                matched_pre_tool_event_id = ?4,
                matched_post_tool_event_id = ?5
          WHERE verification_id = ?1 AND status = 'active'",
        params![
            run.verification_id,
            post.occurred_at,
            prompt.guard_event_id,
            pre.guard_event_id,
            post.guard_event_id,
        ],
    )?;
    let updated =
        run_from_conn(&tx, &run.verification_id)?.ok_or_else(|| StoreError::NotFound {
            entity: "guard_integration_verification",
            id: run.verification_id,
        })?;
    tx.commit()?;
    Ok(Some(updated))
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
    conn.query_row(
        &format!(
            "{RUN_SELECT}
              WHERE connection_internal_id = ?1
                AND integration_revision = ?2
              ORDER BY created_at DESC, verification_id DESC
              LIMIT 1"
        ),
        [connection_internal_id, integration_revision.as_str()],
        run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

/// Computes the current effective status for a report without mutating the run.
pub fn current_guard_integration_verification_status(
    runtime_home: impl AsRef<Path>,
    run: &GuardIntegrationVerificationRunRecord,
    observed_at: &str,
) -> StoreResult<GuardIntegrationVerificationStatus> {
    effective_status(
        runtime_home.as_ref(),
        run,
        &parse_timestamp("observed_at", observed_at)?,
    )
}

fn correlated_event_triple<'a>(
    run: &GuardIntegrationVerificationRunRecord,
    events: &'a [GuardEventRecord],
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
    let probe_name = codex_hook_tool_name(AgentToolId::GUARD_PROBE);
    for prompt in prompt {
        let prompt_at = parse_timestamp("occurred_at", &prompt.occurred_at)?;
        for pre in events.iter().filter(|event| {
            tool_event_matches(
                event,
                "pre_tool",
                probe_name.as_str(),
                &run.verification_id,
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
                    probe_name.as_str(),
                    &run.verification_id,
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

fn prompt_event_matches(event: &GuardEventRecord, digest: &str) -> bool {
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
    expected_name: &str,
    verification_id: &str,
    digest: &str,
) -> bool {
    let Some(HostNativeCorrelation::CodexHookTool(correlation)) = event.correlation.as_ref() else {
        return false;
    };
    event.event_kind == kind
        && event.contract_status == COMPATIBLE_CONTRACT
        && correlation.tool_name.as_str() == expected_name
        && event_contract_digest(event).as_deref() == Some(digest)
        && event_verification_id(event).as_deref() == Some(verification_id)
}

fn event_contract_digest(event: &GuardEventRecord) -> Option<String> {
    serde_json::from_str::<Value>(&event.metadata_json)
        .ok()?
        .get("host_contract_digest")?
        .as_str()
        .map(str::to_owned)
}

fn event_verification_id(event: &GuardEventRecord) -> Option<String> {
    let subject = serde_json::from_str::<Value>(&event.subject_json).ok()?;
    let raw = subject.get("raw_event")?;
    raw.get("tool_input")
        .or_else(|| raw.get("input"))
        .or_else(|| raw.pointer("/tool/input"))
        .or_else(|| raw.pointer("/tool/arguments"))
        .or_else(|| raw.pointer("/tool_use/input"))?
        .get("verification_id")?
        .as_str()
        .map(str::to_owned)
}

fn effective_status(
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

fn result_from_record(
    run: &GuardIntegrationVerificationRunRecord,
    status: GuardIntegrationVerificationStatus,
) -> GetIntegrationVerificationResult {
    let phase = |value: &Option<String>| {
        if value.is_some() {
            GuardIntegrationVerificationPhaseStatus::Matched
        } else {
            GuardIntegrationVerificationPhaseStatus::Pending
        }
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
    let next_action = match status {
        GuardIntegrationVerificationStatus::Active if run.probe_acknowledged_at.is_none() => {
            Some(format!(
                "Call {} with this verification_id.",
                AgentToolId::GUARD_PROBE.wire_name()
            ))
        }
        GuardIntegrationVerificationStatus::Active => Some(
            "Read this verification again after the host PostToolUse hook completes.".to_owned(),
        ),
        GuardIntegrationVerificationStatus::Failed
        | GuardIntegrationVerificationStatus::Expired => Some(
            "Begin a new verification in the current managed Codex turn after repairing the reported condition."
                .to_owned(),
        ),
        GuardIntegrationVerificationStatus::Passed => None,
    };
    GetIntegrationVerificationResult {
        verification_id: GuardIntegrationVerificationId::new(&run.verification_id),
        status,
        mcp_probe_acknowledged: run.probe_acknowledged_at.is_some(),
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
        completed_at: run
            .completed_at
            .as_deref()
            .and_then(|value| UtcTimestamp::parse(value).ok()),
        finding,
        next_action,
    }
}

fn require_active_caller(
    run: &GuardIntegrationVerificationRunRecord,
    caller: &GuardIntegrationVerificationCaller,
    now: &UtcTimestamp,
) -> StoreResult<()> {
    if run.status != ACTIVE_STATUS
        || parse_timestamp("expires_at", &run.expires_at)? <= *now
        || run.connection_internal_id != caller.connection_internal_id
        || run.runtime_session_id != caller.runtime_session_id
        || run.host_session_id != caller.host_session_id
        || run.host_turn_id != caller.host_turn_id
        || run.expected_probe_tool != AgentToolId::GUARD_PROBE.wire_name()
    {
        return Err(coordinate_conflict(
            caller,
            "verification is not active for this exact managed session and native turn",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn require_run_coordinate(
    run: &GuardIntegrationVerificationRunRecord,
    caller: &GuardIntegrationVerificationCaller,
    project_internal_id: &str,
    guard_installation_id: &str,
    policy_hash: &str,
    hook_contract_digest: &str,
) -> StoreResult<()> {
    if run.project_internal_id != project_internal_id
        || run.host_session_id != caller.host_session_id
        || run.guard_installation_id != guard_installation_id
        || run.policy_hash != policy_hash
        || run.hook_contract_digest != hook_contract_digest
        || run.expected_probe_tool != AgentToolId::GUARD_PROBE.wire_name()
    {
        return Err(coordinate_conflict(
            caller,
            "an active verification coordinate is owned by different current facts",
        ));
    }
    Ok(())
}

fn validate_caller(caller: &GuardIntegrationVerificationCaller) -> StoreResult<()> {
    for (field, value) in [
        (
            "connection_internal_id",
            caller.connection_internal_id.as_str(),
        ),
        ("runtime_session_id", caller.runtime_session_id.as_str()),
        ("host_session_id", caller.host_session_id.as_str()),
        ("host_turn_id", caller.host_turn_id.as_str()),
    ] {
        if value.is_empty() || value.trim() != value || value.contains('\0') {
            return Err(StoreError::InvalidInput {
                detail: format!("{field} must be a non-empty canonical identifier"),
            });
        }
    }
    Ok(())
}

fn coordinate_conflict(caller: &GuardIntegrationVerificationCaller, detail: &str) -> StoreError {
    StoreError::Conflict {
        entity: "guard_integration_verification",
        id: caller.runtime_session_id.clone(),
        detail: detail.to_owned(),
    }
}

fn expire_active_runs(conn: &Connection, observed_at: &str) -> StoreResult<()> {
    conn.execute(
        "UPDATE guard_integration_verification_runs
            SET status = 'expired', completed_at = ?1,
                terminal_finding_code = 'verification_expired',
                terminal_finding_summary = 'The bounded integration-verification window expired.'
          WHERE status = 'active' AND expires_at <= ?1",
        [observed_at],
    )?;
    Ok(())
}

fn resumable_run_for_coordinate(
    conn: &Connection,
    connection_internal_id: &str,
    runtime_session_id: &str,
    host_turn_id: &str,
    integration_revision: &str,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    conn.query_row(
        &format!(
            "{RUN_SELECT}
              WHERE connection_internal_id = ?1
                AND runtime_session_id = ?2
                AND host_turn_id = ?3
                AND integration_revision = ?4
                AND status IN ('active', 'passed')
              ORDER BY CASE status WHEN 'active' THEN 0 ELSE 1 END,
                       created_at DESC
              LIMIT 1"
        ),
        params![
            connection_internal_id,
            runtime_session_id,
            host_turn_id,
            integration_revision,
        ],
        run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

const RUN_SELECT: &str = "SELECT
    verification_id, connection_internal_id, project_internal_id,
    runtime_session_id, host_session_id, host_turn_id,
    guard_installation_id, integration_revision, policy_hash,
    hook_contract_digest, expected_probe_tool, created_at, expires_at, status,
    probe_acknowledged_at, completed_at, matched_prompt_event_id,
    matched_pre_tool_event_id, matched_post_tool_event_id,
    terminal_finding_code, terminal_finding_summary
  FROM guard_integration_verification_runs";

fn run_from_conn(
    conn: &Connection,
    verification_id: &str,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    conn.query_row(
        &format!("{RUN_SELECT} WHERE verification_id = ?1"),
        [verification_id],
        run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn run_from_row(row: &Row<'_>) -> rusqlite::Result<GuardIntegrationVerificationRunRecord> {
    Ok(GuardIntegrationVerificationRunRecord {
        verification_id: row.get(0)?,
        connection_internal_id: row.get(1)?,
        project_internal_id: row.get(2)?,
        runtime_session_id: row.get(3)?,
        host_session_id: row.get(4)?,
        host_turn_id: row.get(5)?,
        guard_installation_id: row.get(6)?,
        integration_revision: row.get(7)?,
        policy_hash: row.get(8)?,
        hook_contract_digest: row.get(9)?,
        expected_probe_tool: row.get(10)?,
        created_at: row.get(11)?,
        expires_at: row.get(12)?,
        status: row.get(13)?,
        probe_acknowledged_at: row.get(14)?,
        completed_at: row.get(15)?,
        matched_prompt_event_id: row.get(16)?,
        matched_pre_tool_event_id: row.get(17)?,
        matched_post_tool_event_id: row.get(18)?,
        terminal_finding_code: row.get(19)?,
        terminal_finding_summary: row.get(20)?,
    })
}

fn parse_timestamp(field: &str, value: &str) -> StoreResult<UtcTimestamp> {
    UtcTimestamp::parse(value).map_err(|_| StoreError::InvalidInput {
        detail: format!("{field} must be an RFC 3339 timestamp"),
    })
}

fn parse_status(value: &str) -> StoreResult<GuardIntegrationVerificationStatus> {
    match value {
        "active" => Ok(GuardIntegrationVerificationStatus::Active),
        "passed" => Ok(GuardIntegrationVerificationStatus::Passed),
        "failed" => Ok(GuardIntegrationVerificationStatus::Failed),
        "expired" => Ok(GuardIntegrationVerificationStatus::Expired),
        _ => Err(StoreError::CorruptStoredValue {
            database_kind: "registry",
            field: "guard_integration_verification_runs.status",
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, path::Path};

    use volicord_host_contract::{
        CanonicalToolName, CodexHookPromptCorrelation, CodexHookToolCorrelation,
        CodexMcpCorrelation, HostSessionId, HostThreadId, HostToolUseId, HostTurnId,
    };
    use volicord_test_support::TempRuntimeHome;
    use volicord_types::{
        GuardDecision, GuardHookContractStatus, McpRuntimeSessionSource, SequenceDurableIdGenerator,
    };

    use super::*;
    use crate::{
        agent_connections::{
            add_connection_project, agent_connection_record_read_only, ensure_agent_connection,
            AgentConnectionRegistration, ConnectionProjectRegistration, CONNECTION_INTENT_SHARED,
            CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT,
        },
        bootstrap::{
            initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
        },
        guards::{
            bind_agent_session_runtime, insert_guard_event, observe_host_correlation,
            test_guard_manifest_json, upsert_guard_installation, AgentSessionRuntimeBinding,
            GuardEventInsert, GuardInstallationUpsert, HostCorrelationObservation,
        },
        operational_sessions::{start_mcp_runtime_session_for_test, McpRuntimeSessionStart},
    };

    const POLICY_HASH: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    const PROJECT_ID: &str = "project_verification";
    const CONNECTION_ID: &str = "connection_verification";
    const INSTALLATION_ID: &str = "guard_installation_verification";
    const HOST_SESSION_ID: &str = "host_session_verification";
    const HOST_THREAD_ID: &str = "host_thread_verification";
    const HOST_TURN_ID: &str = "host_turn_verification";

    struct VerificationFixture {
        runtime_home: TempRuntimeHome,
        runtime_session_id: String,
        project_session_id: String,
        integration_revision: String,
    }

    struct ToolEventFixture<'a> {
        event_id: &'a str,
        phase: &'a str,
        turn: &'a str,
        tool_use_id: &'a str,
        tool_name: &'a str,
        verification_id: &'a str,
        occurred_at: &'a str,
        digest: Option<&'a str>,
    }

    impl VerificationFixture {
        fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new(prefix)?;
            initialize_runtime_home(runtime_home.path(), &format!("runtime_home_{prefix}"), "{}")?;
            let repo_root = runtime_home.create_product_repo("verification-repo")?;
            register_project(
                runtime_home.path(),
                ProjectRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    repo_root: repo_root.clone(),
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            ensure_agent_connection(
                runtime_home.path(),
                AgentConnectionRegistration {
                    connection_internal_id: CONNECTION_ID.to_owned(),
                    host_kind: HOST_KIND_CODEX.to_owned(),
                    intent: CONNECTION_INTENT_SHARED.to_owned(),
                    host_scope: HOST_SCOPE_PROJECT.to_owned(),
                    server_name: "volicord-verification".to_owned(),
                    config_target: runtime_home
                        .path()
                        .join("connection-verification")
                        .to_string_lossy()
                        .into_owned(),
                    mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                    enabled: true,
                    managed_fingerprint: "fingerprint:verification".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            add_connection_project(
                runtime_home.path(),
                ConnectionProjectRegistration {
                    connection_internal_id: CONNECTION_ID.to_owned(),
                    project_id: PROJECT_ID.to_owned(),
                },
            )?;
            let connection = agent_connection_record_read_only(runtime_home.path(), CONNECTION_ID)?
                .expect("test connection");
            upsert_guard_installation(
                runtime_home.path(),
                GuardInstallationUpsert {
                    guard_installation_id: INSTALLATION_ID.to_owned(),
                    connection_internal_id: CONNECTION_ID.to_owned(),
                    project_id: PROJECT_ID.to_owned(),
                    manifest_json: test_guard_manifest_json(
                        &connection,
                        PROJECT_ID,
                        &repo_root,
                        INSTALLATION_ID,
                        POLICY_HASH,
                    ),
                },
            )?;
            let integration_revision =
                crate::operational_sessions::connection_integration_revision(&connection)?
                    .as_str()
                    .to_owned();
            let runtime_session_id = start_mcp_runtime_session_for_test(
                runtime_home.path(),
                McpRuntimeSessionStart {
                    connection_internal_id: CONNECTION_ID.to_owned(),
                    session_source: McpRuntimeSessionSource::ManagedHost,
                    observed_host_executable_version: None,
                    process_id: 42,
                    process_started_at: "2026-07-23T00:00:00Z".to_owned(),
                },
            )?
            .runtime_session_id;

            let prompt = prompt_correlation(HOST_TURN_ID);
            observe_event_correlation(runtime_home.path(), prompt.clone(), "2026-07-23T00:00:01Z")?;
            insert_test_event(
                runtime_home.path(),
                &integration_revision,
                "guard_event_prompt",
                prompt,
                "prompt_capture",
                "2026-07-23T00:00:01Z",
                None,
                None,
            )?;
            let session = bind_agent_session_runtime(
                runtime_home.path(),
                PROJECT_ID,
                AgentSessionRuntimeBinding {
                    runtime_session_id: runtime_session_id.clone(),
                    connection_internal_id: CONNECTION_ID.to_owned(),
                    guard_installation_id: Some(INSTALLATION_ID.to_owned()),
                    correlation: mcp_correlation(HOST_TURN_ID),
                    observed_at: "2026-07-23T00:00:02Z".to_owned(),
                },
            )?;
            Ok(Self {
                runtime_home,
                runtime_session_id,
                project_session_id: session.session_id,
                integration_revision,
            })
        }

        fn caller(&self) -> GuardIntegrationVerificationCaller {
            GuardIntegrationVerificationCaller {
                connection_internal_id: CONNECTION_ID.to_owned(),
                runtime_session_id: self.runtime_session_id.clone(),
                host_session_id: HOST_SESSION_ID.to_owned(),
                host_turn_id: HOST_TURN_ID.to_owned(),
            }
        }

        fn begin(&self) -> StoreResult<GuardIntegrationVerificationRunRecord> {
            begin_guard_integration_verification_with_generator(
                self.runtime_home.path(),
                BeginGuardIntegrationVerificationInput {
                    caller: self.caller(),
                    project_id: PROJECT_ID.to_owned(),
                    project_session_id: self.project_session_id.clone(),
                    observed_at: "2026-07-23T00:00:03Z".to_owned(),
                },
                &SequenceDurableIdGenerator::new(["one"]),
            )
        }

        fn insert_tool_event(&self, event: ToolEventFixture<'_>) -> StoreResult<()> {
            let correlation = tool_correlation(event.turn, event.tool_use_id, event.tool_name);
            observe_event_correlation(
                self.runtime_home.path(),
                correlation.clone(),
                event.occurred_at,
            )?;
            insert_test_event(
                self.runtime_home.path(),
                &self.integration_revision,
                event.event_id,
                correlation,
                event.phase,
                event.occurred_at,
                Some(event.verification_id),
                event.digest,
            )?;
            Ok(())
        }
    }

    #[test]
    fn begin_resumes_probe_is_idempotent_and_exact_triple_passes() -> Result<(), Box<dyn Error>> {
        let fixture = VerificationFixture::new("guard-integration-success")?;
        let run = fixture.begin()?;
        let resumed = begin_guard_integration_verification_with_generator(
            fixture.runtime_home.path(),
            BeginGuardIntegrationVerificationInput {
                caller: fixture.caller(),
                project_id: PROJECT_ID.to_owned(),
                project_session_id: fixture.project_session_id.clone(),
                observed_at: "2026-07-23T00:00:03Z".to_owned(),
            },
            &SequenceDurableIdGenerator::new(Vec::<String>::new()),
        )?;
        assert_eq!(run.verification_id, resumed.verification_id);

        let first = acknowledge_guard_integration_probe(
            fixture.runtime_home.path(),
            &run.verification_id,
            &fixture.caller(),
            "2026-07-23T00:00:04Z",
        )?;
        let replay = acknowledge_guard_integration_probe(
            fixture.runtime_home.path(),
            &run.verification_id,
            &fixture.caller(),
            "2026-07-23T00:00:04.100Z",
        )?;
        assert_eq!(first.acknowledged_at, replay.acknowledged_at);

        let probe_name = codex_hook_tool_name(AgentToolId::GUARD_PROBE);
        fixture.insert_tool_event(ToolEventFixture {
            event_id: "guard_event_pre",
            phase: "pre_tool",
            turn: HOST_TURN_ID,
            tool_use_id: "tool_use_probe",
            tool_name: probe_name.as_str(),
            verification_id: &run.verification_id,
            occurred_at: "2026-07-23T00:00:03.500Z",
            digest: None,
        })?;
        fixture.insert_tool_event(ToolEventFixture {
            event_id: "guard_event_post",
            phase: "post_tool",
            turn: HOST_TURN_ID,
            tool_use_id: "tool_use_probe",
            tool_name: probe_name.as_str(),
            verification_id: &run.verification_id,
            occurred_at: "2026-07-23T00:00:04.500Z",
            digest: None,
        })?;
        let updated = refresh_guard_integration_verification_for_event(
            fixture.runtime_home.path(),
            PROJECT_ID,
            "guard_event_post",
        )?
        .expect("active verification");
        assert_eq!(updated.status, "passed");
        let result = get_guard_integration_verification(
            fixture.runtime_home.path(),
            &run.verification_id,
            &fixture.caller(),
            "2026-07-23T00:00:05Z",
        )?;
        assert_eq!(result.status, GuardIntegrationVerificationStatus::Passed);
        assert_eq!(
            result.guard_phases,
            GuardIntegrationVerificationPhases {
                prompt_capture: GuardIntegrationVerificationPhaseStatus::Matched,
                pre_tool: GuardIntegrationVerificationPhaseStatus::Matched,
                post_tool: GuardIntegrationVerificationPhaseStatus::Matched,
            }
        );
        let resumed_after_pass = begin_guard_integration_verification_with_generator(
            fixture.runtime_home.path(),
            BeginGuardIntegrationVerificationInput {
                caller: fixture.caller(),
                project_id: PROJECT_ID.to_owned(),
                project_session_id: fixture.project_session_id.clone(),
                observed_at: "2026-07-23T00:00:06Z".to_owned(),
            },
            &SequenceDurableIdGenerator::new(Vec::<String>::new()),
        )?;
        assert_eq!(run.verification_id, resumed_after_pass.verification_id);
        assert_eq!(resumed_after_pass.status, "passed");

        let conn = open_registry_database(registry_db_path(fixture.runtime_home.path()))?;
        conn.execute(
            "UPDATE guard_integration_verification_runs
                SET policy_hash = ?2
              WHERE verification_id = ?1",
            params![
                run.verification_id,
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            ],
        )?;
        let stale_pass = get_guard_integration_verification(
            fixture.runtime_home.path(),
            &run.verification_id,
            &fixture.caller(),
            "2026-07-23T00:00:07Z",
        )?;
        assert_eq!(
            stale_pass.status,
            GuardIntegrationVerificationStatus::Failed
        );
        Ok(())
    }

    #[test]
    fn mismatched_guard_events_and_expiry_never_pass() -> Result<(), Box<dyn Error>> {
        let probe_name = codex_hook_tool_name(AgentToolId::GUARD_PROBE);
        for (index, mismatch) in [
            "turn",
            "tool_use",
            "tool_name",
            "verification_id",
            "hook_digest",
        ]
        .into_iter()
        .enumerate()
        {
            let fixture = VerificationFixture::new(&format!("guard-integration-{index}"))?;
            let run = fixture.begin()?;
            acknowledge_guard_integration_probe(
                fixture.runtime_home.path(),
                &run.verification_id,
                &fixture.caller(),
                "2026-07-23T00:00:04Z",
            )?;
            let pre_turn = if mismatch == "turn" {
                "other_turn"
            } else {
                HOST_TURN_ID
            };
            let pre_use = "tool_use_pre";
            let post_use = if mismatch == "tool_use" {
                "tool_use_post"
            } else {
                pre_use
            };
            let name = if mismatch == "tool_name" {
                "mcp__volicord__status"
            } else {
                probe_name.as_str()
            };
            let verification_id = if mismatch == "verification_id" {
                "guard_verification_other"
            } else {
                &run.verification_id
            };
            let digest = (mismatch == "hook_digest").then_some(
                "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            );
            fixture.insert_tool_event(ToolEventFixture {
                event_id: "guard_event_pre_bad",
                phase: "pre_tool",
                turn: pre_turn,
                tool_use_id: pre_use,
                tool_name: name,
                verification_id,
                occurred_at: "2026-07-23T00:00:03.500Z",
                digest,
            })?;
            fixture.insert_tool_event(ToolEventFixture {
                event_id: "guard_event_post_bad",
                phase: "post_tool",
                turn: pre_turn,
                tool_use_id: post_use,
                tool_name: name,
                verification_id,
                occurred_at: "2026-07-23T00:00:04.500Z",
                digest,
            })?;
            let _ = refresh_guard_integration_verification_for_event(
                fixture.runtime_home.path(),
                PROJECT_ID,
                "guard_event_post_bad",
            )?;
            let result = get_guard_integration_verification(
                fixture.runtime_home.path(),
                &run.verification_id,
                &fixture.caller(),
                "2026-07-23T00:00:05Z",
            )?;
            assert_eq!(
                result.status,
                GuardIntegrationVerificationStatus::Active,
                "{mismatch} must not pass",
            );
        }

        let fixture = VerificationFixture::new("guard-integration-expired")?;
        let run = fixture.begin()?;
        let result = get_guard_integration_verification(
            fixture.runtime_home.path(),
            &run.verification_id,
            &fixture.caller(),
            "2026-07-23T00:05:03Z",
        )?;
        assert_eq!(result.status, GuardIntegrationVerificationStatus::Expired);
        Ok(())
    }

    #[test]
    fn ordered_current_events_are_required_instead_of_unrelated_history(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = VerificationFixture::new("guard-integration-event-order")?;
        let run = fixture.begin()?;
        acknowledge_guard_integration_probe(
            fixture.runtime_home.path(),
            &run.verification_id,
            &fixture.caller(),
            "2026-07-23T00:00:04Z",
        )?;
        let probe_name = codex_hook_tool_name(AgentToolId::GUARD_PROBE);

        for (event_id, phase, occurred_at) in [
            (
                "guard_event_historical_pre",
                "pre_tool",
                "2026-07-23T00:00:00.500Z",
            ),
            (
                "guard_event_historical_post",
                "post_tool",
                "2026-07-23T00:00:00.750Z",
            ),
        ] {
            fixture.insert_tool_event(ToolEventFixture {
                event_id,
                phase,
                turn: HOST_TURN_ID,
                tool_use_id: "tool_use_historical",
                tool_name: probe_name.as_str(),
                verification_id: &run.verification_id,
                occurred_at,
                digest: None,
            })?;
        }
        refresh_guard_integration_verification_for_event(
            fixture.runtime_home.path(),
            PROJECT_ID,
            "guard_event_historical_post",
        )?;
        assert_eq!(
            get_guard_integration_verification(
                fixture.runtime_home.path(),
                &run.verification_id,
                &fixture.caller(),
                "2026-07-23T00:00:04.100Z",
            )?
            .status,
            GuardIntegrationVerificationStatus::Active,
            "matching-looking events before the captured prompt cannot complete the run",
        );

        fixture.insert_tool_event(ToolEventFixture {
            event_id: "guard_event_current_pre",
            phase: "pre_tool",
            turn: HOST_TURN_ID,
            tool_use_id: "tool_use_current",
            tool_name: probe_name.as_str(),
            verification_id: &run.verification_id,
            occurred_at: "2026-07-23T00:00:03.500Z",
            digest: None,
        })?;
        fixture.insert_tool_event(ToolEventFixture {
            event_id: "guard_event_post_before_ack",
            phase: "post_tool",
            turn: HOST_TURN_ID,
            tool_use_id: "tool_use_current",
            tool_name: probe_name.as_str(),
            verification_id: &run.verification_id,
            occurred_at: "2026-07-23T00:00:03.750Z",
            digest: None,
        })?;
        refresh_guard_integration_verification_for_event(
            fixture.runtime_home.path(),
            PROJECT_ID,
            "guard_event_post_before_ack",
        )?;
        assert_eq!(
            get_guard_integration_verification(
                fixture.runtime_home.path(),
                &run.verification_id,
                &fixture.caller(),
                "2026-07-23T00:00:04.200Z",
            )?
            .status,
            GuardIntegrationVerificationStatus::Active,
            "a post-tool event before probe acknowledgement cannot complete the run",
        );

        fixture.insert_tool_event(ToolEventFixture {
            event_id: "guard_event_current_post",
            phase: "post_tool",
            turn: HOST_TURN_ID,
            tool_use_id: "tool_use_current",
            tool_name: probe_name.as_str(),
            verification_id: &run.verification_id,
            occurred_at: "2026-07-23T00:00:04.500Z",
            digest: None,
        })?;
        let updated = refresh_guard_integration_verification_for_event(
            fixture.runtime_home.path(),
            PROJECT_ID,
            "guard_event_current_post",
        )?
        .expect("current verification");
        assert_eq!(updated.status, "passed");
        Ok(())
    }

    #[test]
    fn manual_or_preflight_runtime_is_rejected_and_stale_owner_facts_fail(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = VerificationFixture::new("guard-integration-stale")?;
        let run = fixture.begin()?;
        let conn = open_registry_database(registry_db_path(fixture.runtime_home.path()))?;
        for (column, stale) in [
            (
                "policy_hash",
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            (
                "hook_contract_digest",
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            ),
            (
                "integration_revision",
                "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            ),
        ] {
            conn.execute(
                &format!(
                    "UPDATE guard_integration_verification_runs SET {column} = ?2 WHERE verification_id = ?1"
                ),
                params![run.verification_id, stale],
            )?;
            let result = get_guard_integration_verification(
                fixture.runtime_home.path(),
                &run.verification_id,
                &fixture.caller(),
                "2026-07-23T00:00:04Z",
            )?;
            assert_eq!(result.status, GuardIntegrationVerificationStatus::Failed);
            conn.execute(
                &format!(
                    "UPDATE guard_integration_verification_runs SET {column} = ?2 WHERE verification_id = ?1"
                ),
                params![
                    run.verification_id,
                    match column {
                        "policy_hash" => POLICY_HASH,
                        "hook_contract_digest" => &run.hook_contract_digest,
                        "integration_revision" => &run.integration_revision,
                        _ => unreachable!(),
                    }
                ],
            )?;
        }

        for (index, source) in [
            McpRuntimeSessionSource::ManualCli,
            McpRuntimeSessionSource::CliPreflight,
        ]
        .into_iter()
        .enumerate()
        {
            let runtime = start_mcp_runtime_session_for_test(
                fixture.runtime_home.path(),
                McpRuntimeSessionStart {
                    connection_internal_id: CONNECTION_ID.to_owned(),
                    session_source: source,
                    observed_host_executable_version: None,
                    process_id: 100 + index as u32,
                    process_started_at: "2026-07-23T00:00:10Z".to_owned(),
                },
            )?;
            let error = begin_guard_integration_verification_with_generator(
                fixture.runtime_home.path(),
                BeginGuardIntegrationVerificationInput {
                    caller: GuardIntegrationVerificationCaller {
                        runtime_session_id: runtime.runtime_session_id,
                        ..fixture.caller()
                    },
                    project_id: PROJECT_ID.to_owned(),
                    project_session_id: fixture.project_session_id.clone(),
                    observed_at: "2026-07-23T00:00:11Z".to_owned(),
                },
                &SequenceDurableIdGenerator::new(["rejected"]),
            )
            .expect_err("manual and preflight sessions cannot begin verification");
            assert!(matches!(error, StoreError::Conflict { .. }));
        }
        Ok(())
    }

    fn mcp_correlation(turn: &str) -> CodexMcpCorrelation {
        CodexMcpCorrelation {
            session_id: HostSessionId::parse(HOST_SESSION_ID).expect("session"),
            thread_id: HostThreadId::parse(HOST_THREAD_ID).expect("thread"),
            turn_id: HostTurnId::parse(turn).expect("turn"),
        }
    }

    fn prompt_correlation(turn: &str) -> HostNativeCorrelation {
        HostNativeCorrelation::CodexHookPrompt(CodexHookPromptCorrelation {
            session_id: HostSessionId::parse(HOST_SESSION_ID).expect("session"),
            turn_id: HostTurnId::parse(turn).expect("turn"),
        })
    }

    fn tool_correlation(turn: &str, tool_use: &str, tool_name: &str) -> HostNativeCorrelation {
        HostNativeCorrelation::CodexHookTool(CodexHookToolCorrelation {
            session_id: HostSessionId::parse(HOST_SESSION_ID).expect("session"),
            turn_id: HostTurnId::parse(turn).expect("turn"),
            tool_use_id: HostToolUseId::parse(tool_use).expect("tool use"),
            tool_name: CanonicalToolName::parse(tool_name).expect("tool name"),
        })
    }

    fn observe_event_correlation(
        runtime_home: &Path,
        correlation: HostNativeCorrelation,
        observed_at: &str,
    ) -> StoreResult<()> {
        observe_host_correlation(
            runtime_home,
            PROJECT_ID,
            HostCorrelationObservation {
                connection_internal_id: CONNECTION_ID.to_owned(),
                guard_installation_id: Some(INSTALLATION_ID.to_owned()),
                correlation,
                observed_at: observed_at.to_owned(),
            },
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_test_event(
        runtime_home: &Path,
        integration_revision: &str,
        event_id: &str,
        correlation: HostNativeCorrelation,
        phase: &str,
        occurred_at: &str,
        verification_id: Option<&str>,
        digest: Option<&str>,
    ) -> StoreResult<()> {
        insert_guard_event(
            runtime_home,
            PROJECT_ID,
            GuardEventInsert {
                guard_event_id: event_id.to_owned(),
                correlation: Some(correlation),
                connection_internal_id: CONNECTION_ID.to_owned(),
                guard_installation_id: INSTALLATION_ID.to_owned(),
                policy_hash: POLICY_HASH.to_owned(),
                integration_revision: integration_revision.to_owned(),
                event_kind: phase.to_owned(),
                contract_status: GuardHookContractStatus::Compatible.as_str().to_owned(),
                decision: GuardDecision::Allow.as_str().to_owned(),
                subject_json: serde_json::json!({
                    "raw_event": {
                        "tool_input": verification_id.map(|id| serde_json::json!({
                            "verification_id": id
                        }))
                    }
                })
                .to_string(),
                result_json: "{}".to_owned(),
                occurred_at: occurred_at.to_owned(),
                metadata_json: serde_json::json!({
                    "host_contract_digest": digest
                        .unwrap_or(&HostContractProfileId::CodexHooksV1.contract_digest())
                })
                .to_string(),
            },
        )?;
        Ok(())
    }
}
