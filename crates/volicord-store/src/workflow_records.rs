//! Store-owned workflow policy and managed session-end receipt records.

use std::{cell::RefCell, collections::BTreeSet};

use rusqlite::{params, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use volicord_types::{
    canonical_json_sha256, canonical_json_string, validate_managed_host_session_id,
    AcceptancePolicy, AuthorityNextActor, RequestedControlLevel, SessionEndTaskState,
    TaskControlLevel, TaskMode, UtcTimestamp,
};

use crate::{
    core_pipeline::{
        CommitMutationInput, CoreProjectStore, MutationCommitOutcome, PendingTaskEvent,
        ProjectWorkflowPolicyMutation, TaskRecord, VerifiedReplayContext,
    },
    sqlite::begin_immediate_transaction,
    StoreError, StoreResult,
};

pub const POLICY_CONTROL_REEVALUATION_METADATA_KEY: &str = "policy_control_reevaluation";
const POLICY_APPLIED_EVENT_KIND: &str = "project_workflow_policy_applied";
const WRITE_AUTHORITY_FINGERPRINT_SCHEMA: &str = "volicord-write-authority-v1";

/// Authoritative project workflow-policy replacement input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkflowPolicyUpsert {
    pub policy_version: u64,
    pub policy_json: String,
    pub policy_fingerprint: String,
    pub source: String,
    pub applied_at: String,
    pub created_at: String,
}

/// Atomic administrative workflow-policy authority application input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkflowPolicyAuthorityApply {
    pub policy_version: u64,
    pub policy_json: String,
    pub policy_fingerprint: String,
    pub source: String,
    pub expected_prior_fingerprint: Option<String>,
}

/// Result of an atomic or idempotent workflow-policy authority application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkflowPolicyApplyResult {
    pub policy: ProjectWorkflowPolicyRecord,
    pub database_changed: bool,
    pub basis_state_version: u64,
    pub resulting_state_version: u64,
    pub active_task_requires_escalation: bool,
    pub active_task_requires_policy_reevaluation: bool,
    pub write_authority_changed: bool,
    pub prior_write_authority_fingerprint: String,
    pub resulting_write_authority_fingerprint: String,
    pub affected_task_ids: Vec<String>,
    pub invalidated_write_ticket_ids: Vec<String>,
}

struct WorkflowPolicyApplyObservation {
    state_version: u64,
    prior: Option<ProjectWorkflowPolicyRecord>,
    active_task_requires_escalation: bool,
    active_task_requires_policy_reevaluation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectWorkflowPolicyMutationEffect {
    pub(crate) write_authority_changed: bool,
    pub(crate) prior_write_authority_fingerprint: String,
    pub(crate) resulting_write_authority_fingerprint: String,
    pub(crate) affected_task_ids: Vec<String>,
    pub(crate) invalidated_write_ticket_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ProjectWriteAuthorityFingerprintBasis {
    schema: &'static str,
    default_direct_control: TaskControlLevel,
    default_work_control: TaskControlLevel,
    light: ProjectWriteAuthorityLightBasis,
    write_ticket: ProjectWriteAuthorityTicketBasis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ProjectWriteAuthorityLightBasis {
    enabled: bool,
    max_intended_paths: u64,
    allowed_path_patterns: Vec<String>,
    denied_path_patterns: Vec<String>,
    final_acceptance: AcceptancePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ProjectWriteAuthorityTicketBasis {
    idle_timeout_minutes: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct StoredProjectWriteAuthorityPolicy {
    workflow: StoredProjectWriteAuthorityWorkflow,
}

#[derive(Debug, Deserialize)]
struct StoredProjectWriteAuthorityWorkflow {
    default_direct_control: TaskControlLevel,
    default_work_control: TaskControlLevel,
    light: StoredProjectWriteAuthorityLight,
    write_ticket: StoredProjectWriteAuthorityTicket,
}

#[derive(Debug, Deserialize)]
struct StoredProjectWriteAuthorityLight {
    enabled: bool,
    max_intended_paths: u64,
    allowed_path_patterns: Vec<String>,
    denied_path_patterns: Vec<String>,
    final_acceptance: AcceptancePolicy,
}

#[derive(Debug, Deserialize)]
struct StoredProjectWriteAuthorityTicket {
    idle_timeout_minutes: Option<u64>,
}

/// Closed durable active-Task policy control reevaluation mark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskPolicyControlReevaluation {
    pub policy_version: u64,
    pub policy_fingerprint: String,
    pub required_effective_control_level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_acceptance_policy: Option<String>,
    pub marked_at: String,
}

/// Current authoritative project workflow-policy row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkflowPolicyRecord {
    pub project_id: String,
    pub policy_schema: String,
    pub policy_version: u64,
    pub policy_json: String,
    pub policy_fingerprint: String,
    pub source: String,
    pub applied_at: String,
    pub created_at: String,
}

/// Insert input for one durable managed session-end authority receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEndReceiptInsert {
    pub session_end_receipt_id: String,
    pub managed_session_id: String,
    pub active_task_id: Option<String>,
    pub task_state: SessionEndTaskState,
    pub close_blocker_codes_json: String,
    pub next_actor: AuthorityNextActor,
    pub completion_claim_allowed: bool,
    pub authority_refresh_succeeded: bool,
    pub created_at: String,
}

/// One durable managed session-end authority receipt row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEndReceiptRecord {
    pub project_id: String,
    pub session_end_receipt_id: String,
    pub managed_session_id: String,
    pub active_task_id: Option<String>,
    pub task_state: SessionEndTaskState,
    pub close_blocker_codes_json: String,
    pub next_actor: AuthorityNextActor,
    pub completion_claim_allowed: bool,
    pub authority_refresh_succeeded: bool,
    pub created_at: String,
}

/// Derives the write-authority digest from an optional stored workflow-policy copy.
pub fn project_write_authority_fingerprint(policy_json: Option<&str>) -> StoreResult<String> {
    let mut basis = match policy_json {
        Some(policy_json) => {
            let stored: StoredProjectWriteAuthorityPolicy = serde_json::from_str(policy_json)
                .map_err(|_| StoreError::InvalidInput {
                    detail: "project workflow policy cannot produce a write-authority binding"
                        .to_owned(),
                })?;
            if stored.workflow.light.max_intended_paths == 0
                || stored
                    .workflow
                    .write_ticket
                    .idle_timeout_minutes
                    .is_some_and(|minutes| minutes == 0)
            {
                return Err(StoreError::InvalidInput {
                    detail: "project workflow policy has an invalid write-authority value"
                        .to_owned(),
                });
            }
            ProjectWriteAuthorityFingerprintBasis {
                schema: WRITE_AUTHORITY_FINGERPRINT_SCHEMA,
                default_direct_control: stored.workflow.default_direct_control,
                default_work_control: stored.workflow.default_work_control,
                light: ProjectWriteAuthorityLightBasis {
                    enabled: stored.workflow.light.enabled,
                    max_intended_paths: stored.workflow.light.max_intended_paths,
                    allowed_path_patterns: stored.workflow.light.allowed_path_patterns,
                    denied_path_patterns: stored.workflow.light.denied_path_patterns,
                    final_acceptance: stored.workflow.light.final_acceptance,
                },
                write_ticket: ProjectWriteAuthorityTicketBasis {
                    idle_timeout_minutes: stored.workflow.write_ticket.idle_timeout_minutes,
                },
            }
        }
        None => ProjectWriteAuthorityFingerprintBasis {
            schema: WRITE_AUTHORITY_FINGERPRINT_SCHEMA,
            default_direct_control: TaskControlLevel::Tracked,
            default_work_control: TaskControlLevel::Tracked,
            light: ProjectWriteAuthorityLightBasis {
                enabled: false,
                max_intended_paths: 3,
                allowed_path_patterns: Vec::new(),
                denied_path_patterns: Vec::new(),
                final_acceptance: AcceptancePolicy::PolicyDependent,
            },
            write_ticket: ProjectWriteAuthorityTicketBasis {
                idle_timeout_minutes: None,
            },
        },
    };
    basis.light.allowed_path_patterns.sort();
    basis.light.allowed_path_patterns.dedup();
    basis.light.denied_path_patterns.sort();
    basis.light.denied_path_patterns.dedup();
    canonical_json_sha256(&basis)
        .map(|fingerprint| fingerprint.into_inner())
        .map_err(|_| StoreError::InvalidInput {
            detail: "project write-authority binding cannot be canonicalized".to_owned(),
        })
}

impl CoreProjectStore {
    /// Reads the current authoritative workflow-policy copy, when configured.
    pub fn project_workflow_policy(&self) -> StoreResult<Option<ProjectWorkflowPolicyRecord>> {
        project_workflow_policy_from_conn(&self.conn, &self.project.project_id)
    }

    /// Applies a monotonic authoritative workflow-policy replacement.
    pub fn upsert_project_workflow_policy(
        &mut self,
        input: ProjectWorkflowPolicyUpsert,
    ) -> StoreResult<ProjectWorkflowPolicyRecord> {
        validate_project_workflow_policy(&input)?;
        let prior = self.project_workflow_policy()?;
        let applied_at =
            UtcTimestamp::parse(&input.applied_at).map_err(|_| StoreError::InvalidInput {
                detail: "applied_at must be a canonical RFC 3339 UTC timestamp".to_owned(),
            })?;
        let created_at =
            UtcTimestamp::parse(&input.created_at).map_err(|_| StoreError::InvalidInput {
                detail: "created_at must be a canonical RFC 3339 UTC timestamp".to_owned(),
            })?;
        self.apply_project_workflow_policy_authority_inner(
            ProjectWorkflowPolicyAuthorityApply {
                policy_version: input.policy_version,
                policy_json: input.policy_json,
                policy_fingerprint: input.policy_fingerprint,
                source: input.source,
                expected_prior_fingerprint: prior
                    .as_ref()
                    .map(|record| record.policy_fingerprint.clone()),
            },
            Some(std::cmp::max(applied_at, created_at).to_string()),
        )
        .map(|result| result.policy)
    }

    /// Applies one project workflow-policy authority change as a normal
    /// state-versioned authority commit, or returns an exact-fingerprint no-op.
    pub fn apply_project_workflow_policy_authority(
        &mut self,
        input: ProjectWorkflowPolicyAuthorityApply,
    ) -> StoreResult<ProjectWorkflowPolicyApplyResult> {
        self.apply_project_workflow_policy_authority_inner(input, None)
    }

    fn apply_project_workflow_policy_authority_inner(
        &mut self,
        input: ProjectWorkflowPolicyAuthorityApply,
        clock_floor: Option<String>,
    ) -> StoreResult<ProjectWorkflowPolicyApplyResult> {
        require_writable(self)?;
        validate_project_workflow_policy_fields(
            input.policy_version,
            &input.policy_json,
            &input.policy_fingerprint,
            &input.source,
        )?;
        let observation = self.workflow_policy_apply_observation()?;
        if let Some(prior) = observation.prior.as_ref() {
            if prior.policy_fingerprint == input.policy_fingerprint {
                let write_authority_fingerprint =
                    project_write_authority_fingerprint(Some(prior.policy_json.as_str()))?;
                return Ok(ProjectWorkflowPolicyApplyResult {
                    policy: prior.clone(),
                    database_changed: false,
                    basis_state_version: observation.state_version,
                    resulting_state_version: observation.state_version,
                    active_task_requires_escalation: observation.active_task_requires_escalation,
                    active_task_requires_policy_reevaluation: observation
                        .active_task_requires_policy_reevaluation,
                    write_authority_changed: false,
                    prior_write_authority_fingerprint: write_authority_fingerprint.clone(),
                    resulting_write_authority_fingerprint: write_authority_fingerprint,
                    affected_task_ids: Vec::new(),
                    invalidated_write_ticket_ids: Vec::new(),
                });
            }
        }
        validate_policy_replacement_basis(
            &self.project.project_id,
            observation.prior.as_ref(),
            input.policy_version,
            input.expected_prior_fingerprint.as_deref(),
        )?;

        let prior_write_authority_fingerprint = project_write_authority_fingerprint(
            observation
                .prior
                .as_ref()
                .map(|record| record.policy_json.as_str()),
        )?;
        let resulting_write_authority_fingerprint =
            project_write_authority_fingerprint(Some(&input.policy_json))?;
        let write_authority_changed =
            prior_write_authority_fingerprint != resulting_write_authority_fingerprint;
        let payload = canonical_json_string(&json!({
            "policy_schema": "volicord-policy-v2",
            "policy_version": input.policy_version,
            "policy_fingerprint": input.policy_fingerprint,
            "write_authority_fingerprint": resulting_write_authority_fingerprint,
            "write_authority_changed": write_authority_changed,
        }))
        .map_err(|_| StoreError::InvalidInput {
            detail: "policy authority event payload cannot be canonicalized".to_owned(),
        })?;
        let request_hash = canonical_json_sha256(&json!({
            "policy_version": input.policy_version,
            "policy_fingerprint": input.policy_fingerprint,
            "expected_prior_fingerprint": input.expected_prior_fingerprint,
        }))
        .map_err(|_| StoreError::InvalidInput {
            detail: "policy authority request identity cannot be computed".to_owned(),
        })?
        .into_inner();
        let fingerprint_suffix = input
            .policy_fingerprint
            .strip_prefix("sha256:")
            .expect("validated policy fingerprint prefix");
        let event_id = format!(
            "evt_policy_{}_{}",
            input.policy_version,
            &fingerprint_suffix[..24]
        );
        let mutation = ProjectWorkflowPolicyMutation {
            policy_version: input.policy_version,
            policy_json: input.policy_json,
            policy_fingerprint: input.policy_fingerprint,
            source: input.source,
            expected_prior_fingerprint: input.expected_prior_fingerprint,
        };
        let commit_input = CommitMutationInput {
            project_id: self.project.project_id.clone(),
            tool_name: "policy_apply".to_owned(),
            idempotency_key: None,
            request_hash,
            replay_context: Some(VerifiedReplayContext {
                actor_source: "local_user".to_owned(),
                operation_category: "admin_local".to_owned(),
                verification_basis: Some("local_admin_policy_apply".to_owned()),
                git_workspace_context_json: None,
            }),
            expected_state_version: None,
            clock_floor,
            include_live_storage_time: true,
            events: vec![PendingTaskEvent {
                event_id,
                task_id: None,
                change_unit_id: None,
                event_kind: POLICY_APPLIED_EVENT_KIND.to_owned(),
                event_payload_json: payload,
            }],
        };
        let mutation_effect = RefCell::new(None);
        let outcome = self.commit_mutation(
            commit_input,
            |project_mutation, facts| {
                let _ = facts.committed_state_version;
                let effect =
                    project_mutation.apply_project_workflow_policy_with_effect(&mutation)?;
                mutation_effect.replace(Some(effect));
                Ok(())
            },
            |_| Ok("{}".to_owned()),
        )?;
        let (basis_state_version, resulting_state_version) = match outcome {
            MutationCommitOutcome::Committed {
                basis_state_version,
                committed_state_version,
                ..
            } => (basis_state_version, committed_state_version),
            _ => {
                return Err(StoreError::schema_invariant(
                    "project_state",
                    "non-replayed administrative policy commit returned a non-commit outcome",
                ))
            }
        };
        let policy = self.project_workflow_policy()?.ok_or_else(|| {
            StoreError::schema_invariant("project_state", "workflow policy write vanished")
        })?;
        let mutation_effect = mutation_effect.into_inner().ok_or_else(|| {
            StoreError::schema_invariant(
                "project_state",
                "workflow policy commit returned no mutation effect",
            )
        })?;
        Ok(ProjectWorkflowPolicyApplyResult {
            policy,
            database_changed: true,
            basis_state_version,
            resulting_state_version,
            active_task_requires_escalation: active_task_has_policy_reevaluation(self)?,
            active_task_requires_policy_reevaluation: active_task_has_any_policy_reevaluation(
                self,
            )?,
            write_authority_changed: mutation_effect.write_authority_changed,
            prior_write_authority_fingerprint: mutation_effect.prior_write_authority_fingerprint,
            resulting_write_authority_fingerprint: mutation_effect
                .resulting_write_authority_fingerprint,
            affected_task_ids: mutation_effect.affected_task_ids,
            invalidated_write_ticket_ids: mutation_effect.invalidated_write_ticket_ids,
        })
    }

    fn workflow_policy_apply_observation(&mut self) -> StoreResult<WorkflowPolicyApplyObservation> {
        self.workflow_policy_apply_observation_with_hook(|| {})
    }

    fn workflow_policy_apply_observation_with_hook(
        &mut self,
        after_state_read: impl FnOnce(),
    ) -> StoreResult<WorkflowPolicyApplyObservation> {
        let project_id = self.project.project_id.clone();
        let tx = begin_immediate_transaction(&mut self.conn)?;
        let state_version = tx
            .query_row(
                "SELECT state_version FROM project_state WHERE project_id = ?1",
                [&project_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound {
                entity: "project_state",
                id: project_id.clone(),
            })?;
        let state_version = u64::try_from(state_version).map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "project_state",
                project_id.clone(),
                "state_version",
            )
        })?;
        after_state_read();
        let prior = project_workflow_policy_from_conn(&tx, &project_id)?;
        let active_task_requires_escalation =
            active_task_has_policy_reevaluation_from_conn(&tx, &project_id)?;
        let active_task_requires_policy_reevaluation =
            active_task_has_any_policy_reevaluation_from_conn(&tx, &project_id)?;
        tx.commit()?;
        Ok(WorkflowPolicyApplyObservation {
            state_version,
            prior,
            active_task_requires_escalation,
            active_task_requires_policy_reevaluation,
        })
    }

    /// Inserts one durable managed session-end authority receipt.
    pub fn insert_session_end_receipt(
        &mut self,
        input: SessionEndReceiptInsert,
    ) -> StoreResult<SessionEndReceiptRecord> {
        require_writable(self)?;
        validate_session_end_receipt(&input)?;
        let tx = begin_immediate_transaction(&mut self.conn)?;
        tx.execute(
            "INSERT INTO session_end_receipts (
                project_id, session_end_receipt_id, session_id, active_task_id,
                task_state, close_blocker_codes_json, next_actor,
                completion_claim_allowed, authority_refresh_succeeded, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                self.project.project_id,
                input.session_end_receipt_id,
                input.managed_session_id,
                input.active_task_id,
                input.task_state.as_str(),
                input.close_blocker_codes_json,
                input.next_actor.as_str(),
                i64::from(input.completion_claim_allowed),
                i64::from(input.authority_refresh_succeeded),
                input.created_at
            ],
        )?;
        tx.commit()?;
        self.session_end_receipt(&input.session_end_receipt_id)?
            .ok_or_else(|| {
                StoreError::schema_invariant("project_state", "session-end receipt write vanished")
            })
    }

    /// Reads one managed session-end receipt by project-local identity.
    pub fn session_end_receipt(
        &self,
        session_end_receipt_id: &str,
    ) -> StoreResult<Option<SessionEndReceiptRecord>> {
        validate_identifier("session_end_receipt_id", session_end_receipt_id)?;
        session_end_receipt_from_conn(&self.conn, &self.project.project_id, session_end_receipt_id)
    }

    /// Reads the latest receipt for one managed session using semantic UTC ordering.
    pub fn latest_session_end_receipt_for_session(
        &self,
        managed_session_id: &str,
    ) -> StoreResult<Option<SessionEndReceiptRecord>> {
        validate_managed_session_id(managed_session_id)?;
        let row = self
            .conn
            .query_row(
                &format!(
                    "{SESSION_END_RECEIPT_SELECT}
                       WHERE project_id = ?1
                         AND session_id = ?2
                       ORDER BY volicord_utc_seconds(created_at) DESC,
                                volicord_utc_subsec_nanos(created_at) DESC,
                                session_end_receipt_id DESC
                       LIMIT 1"
                ),
                params![self.project.project_id, managed_session_id],
                session_end_receipt_raw_from_row,
            )
            .optional()?;
        row.map(session_end_receipt_from_raw).transpose()
    }
}

const SESSION_END_RECEIPT_SELECT: &str = "SELECT
    project_id, session_end_receipt_id, session_id, active_task_id, task_state,
    close_blocker_codes_json, next_actor, completion_claim_allowed,
    authority_refresh_succeeded, created_at
  FROM session_end_receipts";

type SessionEndReceiptRaw = (
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    i64,
    i64,
    String,
);

fn project_workflow_policy_from_conn(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> StoreResult<Option<ProjectWorkflowPolicyRecord>> {
    let raw = conn
        .query_row(
            "SELECT project_id, policy_schema, policy_version, policy_json,
                    policy_fingerprint, source, applied_at, created_at
               FROM project_workflow_policies
              WHERE project_id = ?1",
            [project_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional()?;
    raw.map(|raw| {
        let policy_version = u64::try_from(raw.2).map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "project_workflow_policies",
                raw.0.clone(),
                "policy_version",
            )
        })?;
        if raw.1 != "volicord-policy-v2" || policy_version == 0 {
            return Err(StoreError::corrupt_owner_state_value(
                "project_workflow_policies",
                raw.0,
                "policy_schema",
            ));
        }
        let record = ProjectWorkflowPolicyRecord {
            project_id: raw.0,
            policy_schema: raw.1,
            policy_version,
            policy_json: raw.3,
            policy_fingerprint: raw.4,
            source: raw.5,
            applied_at: raw.6,
            created_at: raw.7,
        };
        validate_project_workflow_policy(&ProjectWorkflowPolicyUpsert {
            policy_version: record.policy_version,
            policy_json: record.policy_json.clone(),
            policy_fingerprint: record.policy_fingerprint.clone(),
            source: record.source.clone(),
            applied_at: record.applied_at.clone(),
            created_at: record.created_at.clone(),
        })
        .map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "project_workflow_policies",
                record.project_id.clone(),
                "policy_json",
            )
        })?;
        Ok(record)
    })
    .transpose()
}

pub(crate) fn apply_project_workflow_policy_mutation(
    tx: &Transaction<'_>,
    project_id: &str,
    committed_at: &str,
    input: &ProjectWorkflowPolicyMutation,
) -> StoreResult<ProjectWorkflowPolicyMutationEffect> {
    validate_project_workflow_policy_fields(
        input.policy_version,
        &input.policy_json,
        &input.policy_fingerprint,
        &input.source,
    )?;
    let existing = project_workflow_policy_from_conn(tx, project_id)?;
    validate_policy_replacement_basis(
        project_id,
        existing.as_ref(),
        input.policy_version,
        input.expected_prior_fingerprint.as_deref(),
    )?;
    let prior_write_authority_fingerprint = project_write_authority_fingerprint(
        existing.as_ref().map(|record| record.policy_json.as_str()),
    )?;
    let resulting_write_authority_fingerprint =
        project_write_authority_fingerprint(Some(&input.policy_json))?;
    let write_authority_changed =
        prior_write_authority_fingerprint != resulting_write_authority_fingerprint;
    let policy_version =
        i64::try_from(input.policy_version).map_err(|_| StoreError::InvalidInput {
            detail: "policy_version is outside the supported SQLite integer range".to_owned(),
        })?;
    let created_at = existing
        .as_ref()
        .map(|record| record.created_at.as_str())
        .unwrap_or(committed_at);
    tx.execute(
        "INSERT INTO project_workflow_policies (
            project_id, policy_schema, policy_version, policy_json,
            policy_fingerprint, source, applied_at, created_at
         ) VALUES (?1, 'volicord-policy-v2', ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(project_id) DO UPDATE SET
            policy_schema = excluded.policy_schema,
            policy_version = excluded.policy_version,
            policy_json = excluded.policy_json,
            policy_fingerprint = excluded.policy_fingerprint,
            source = excluded.source,
            applied_at = excluded.applied_at,
            created_at = excluded.created_at",
        params![
            project_id,
            policy_version,
            input.policy_json,
            input.policy_fingerprint,
            input.source,
            committed_at,
            created_at,
        ],
    )?;
    let reevaluated_task_id = merge_active_task_policy_reevaluation(
        tx,
        project_id,
        committed_at,
        input,
        write_authority_changed,
    )?;
    let (affected_task_ids, invalidated_write_ticket_ids) = if write_authority_changed {
        invalidate_incompatible_write_tickets(
            tx,
            project_id,
            &resulting_write_authority_fingerprint,
            reevaluated_task_id.as_deref(),
        )?
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(ProjectWorkflowPolicyMutationEffect {
        write_authority_changed,
        prior_write_authority_fingerprint,
        resulting_write_authority_fingerprint,
        affected_task_ids,
        invalidated_write_ticket_ids,
    })
}

/// Strict-decodes the durable policy-control reevaluation mark on one Task.
pub fn task_policy_control_reevaluation(
    task: &TaskRecord,
) -> StoreResult<Option<TaskPolicyControlReevaluation>> {
    task_policy_control_reevaluation_from_metadata(&task.metadata_json, &task.task_id)
}

pub(crate) fn clear_satisfied_task_policy_reevaluation(
    metadata_json: &str,
    task_id: &str,
    effective_control_level: &str,
    acceptance_policy: &str,
) -> StoreResult<String> {
    let Some(mark) = task_policy_control_reevaluation_from_metadata(metadata_json, task_id)? else {
        return Ok(metadata_json.to_owned());
    };
    let effective = parse_control_level(
        effective_control_level,
        "tasks",
        task_id,
        "effective_control_level",
    )?;
    let required = parse_control_level(
        &mark.required_effective_control_level,
        "tasks",
        task_id,
        POLICY_CONTROL_REEVALUATION_METADATA_KEY,
    )?;
    let acceptance_satisfied = if let Some(required) = mark.required_acceptance_policy.as_deref() {
        acceptance_policy_rank(acceptance_policy)? >= acceptance_policy_rank(required)?
    } else {
        true
    };
    if effective < required || !acceptance_satisfied {
        return Ok(metadata_json.to_owned());
    }
    let mut metadata = task_metadata_object(metadata_json, task_id)?;
    metadata.remove(POLICY_CONTROL_REEVALUATION_METADATA_KEY);
    canonical_json_string(&Value::Object(metadata))
        .map_err(|_| StoreError::corrupt_owner_state_json("tasks", task_id, "metadata_json"))
}

fn active_task_has_policy_reevaluation(store: &CoreProjectStore) -> StoreResult<bool> {
    active_task_has_policy_reevaluation_from_conn(&store.conn, &store.project.project_id)
}

fn active_task_has_any_policy_reevaluation(store: &CoreProjectStore) -> StoreResult<bool> {
    active_task_has_any_policy_reevaluation_from_conn(&store.conn, &store.project.project_id)
}

fn active_task_has_any_policy_reevaluation_from_conn(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> StoreResult<bool> {
    let active = conn
        .query_row(
            "SELECT t.task_id, t.metadata_json
               FROM project_state AS ps
               JOIN tasks AS t
                 ON t.project_id = ps.project_id
                AND t.task_id = ps.active_task_id
              WHERE ps.project_id = ?1",
            [project_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((task_id, metadata_json)) = active else {
        return Ok(false);
    };
    Ok(task_policy_control_reevaluation_from_metadata(&metadata_json, &task_id)?.is_some())
}

fn active_task_has_policy_reevaluation_from_conn(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> StoreResult<bool> {
    let active = conn
        .query_row(
            "SELECT t.task_id, t.effective_control_level,
                    t.acceptance_policy, t.metadata_json
               FROM project_state AS ps
               JOIN tasks AS t
                 ON t.project_id = ps.project_id
                AND t.task_id = ps.active_task_id
              WHERE ps.project_id = ?1",
            [project_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((task_id, effective_control_level, acceptance_policy, metadata_json)) = active else {
        return Ok(false);
    };
    let Some(mark) = task_policy_control_reevaluation_from_metadata(&metadata_json, &task_id)?
    else {
        return Ok(false);
    };
    let current = parse_control_level(
        &effective_control_level,
        "tasks",
        &task_id,
        "effective_control_level",
    )?;
    let required = parse_control_level(
        &mark.required_effective_control_level,
        "tasks",
        &task_id,
        POLICY_CONTROL_REEVALUATION_METADATA_KEY,
    )?;
    let acceptance_escalation = if let Some(required) = mark.required_acceptance_policy.as_deref() {
        acceptance_policy_rank(required)? > acceptance_policy_rank(&acceptance_policy)?
    } else {
        false
    };
    Ok(required > current || acceptance_escalation)
}

fn merge_active_task_policy_reevaluation(
    tx: &Transaction<'_>,
    project_id: &str,
    committed_at: &str,
    input: &ProjectWorkflowPolicyMutation,
    write_authority_changed: bool,
) -> StoreResult<Option<String>> {
    if !write_authority_changed {
        return Ok(None);
    }
    type ActiveTaskPolicyFacts = (String, String, String, String, String, String);
    let active: Option<ActiveTaskPolicyFacts> = tx
        .query_row(
            "SELECT t.task_id, t.mode, t.requested_control_level,
                    t.effective_control_level, t.acceptance_policy, t.metadata_json
               FROM project_state AS ps
               JOIN tasks AS t
                 ON t.project_id = ps.project_id
                AND t.task_id = ps.active_task_id
              WHERE ps.project_id = ?1",
            [project_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .optional()?;
    let Some((task_id, mode, requested, current, current_acceptance, metadata_json)) = active
    else {
        return Ok(None);
    };
    let current_level =
        parse_control_level(&current, "tasks", &task_id, "effective_control_level")?;
    let required_level =
        required_control_for_policy(project_id, &input.policy_json, &mode, &requested)?;
    let required_acceptance =
        required_acceptance_for_policy(project_id, &input.policy_json, required_level)?;
    let current_acceptance_rank = acceptance_policy_rank(&current_acceptance)?;
    let existing_mark = task_policy_control_reevaluation_from_metadata(&metadata_json, &task_id)?;
    let existing_required = existing_mark
        .as_ref()
        .map(|mark| {
            parse_control_level(
                &mark.required_effective_control_level,
                "tasks",
                &task_id,
                POLICY_CONTROL_REEVALUATION_METADATA_KEY,
            )
        })
        .transpose()?;
    let existing_required_acceptance = existing_mark
        .as_ref()
        .and_then(|mark| mark.required_acceptance_policy.as_deref())
        .map(acceptance_policy_rank)
        .transpose()?;

    let combined_required_level =
        std::cmp::max(required_level, existing_required.unwrap_or(current_level));
    let combined_control_acceptance =
        required_acceptance_for_policy(project_id, &input.policy_json, combined_required_level)?;
    let combined_required_acceptance = std::cmp::max(
        std::cmp::max(
            acceptance_policy_rank(acceptance_policy_name(required_acceptance))?,
            acceptance_policy_rank(acceptance_policy_name(combined_control_acceptance))?,
        ),
        existing_required_acceptance.unwrap_or(current_acceptance_rank),
    );
    let next_mark = Some(TaskPolicyControlReevaluation {
        policy_version: input.policy_version,
        policy_fingerprint: input.policy_fingerprint.clone(),
        required_effective_control_level: combined_required_level.as_str().to_owned(),
        required_acceptance_policy: Some(
            acceptance_policy_name(acceptance_policy_from_rank(combined_required_acceptance))
                .to_owned(),
        ),
        marked_at: committed_at.to_owned(),
    });
    if next_mark == existing_mark {
        return Ok(Some(task_id));
    }
    let mut metadata = task_metadata_object(&metadata_json, &task_id)?;
    match next_mark {
        Some(mark) => {
            metadata.insert(
                POLICY_CONTROL_REEVALUATION_METADATA_KEY.to_owned(),
                serde_json::to_value(mark).map_err(|_| StoreError::InvalidInput {
                    detail: "Task policy reevaluation mark cannot be serialized".to_owned(),
                })?,
            );
        }
        None => {
            metadata.remove(POLICY_CONTROL_REEVALUATION_METADATA_KEY);
        }
    }
    let metadata_json = canonical_json_string(&Value::Object(metadata))
        .map_err(|_| StoreError::corrupt_owner_state_json("tasks", &task_id, "metadata_json"))?;
    tx.execute(
        "UPDATE tasks
            SET metadata_json = ?3,
                updated_at = ?4
          WHERE project_id = ?1
            AND task_id = ?2",
        params![project_id, task_id, metadata_json, committed_at],
    )?;
    Ok(Some(task_id))
}

fn invalidate_incompatible_write_tickets(
    tx: &Transaction<'_>,
    project_id: &str,
    resulting_write_authority_fingerprint: &str,
    reevaluated_task_id: Option<&str>,
) -> StoreResult<(Vec<String>, Vec<String>)> {
    let active_tickets = {
        let mut statement = tx.prepare(
            "SELECT write_ticket_id, task_id, validity_basis_json
               FROM write_tickets
              WHERE project_id = ?1
                AND status = 'active'
              ORDER BY write_ticket_id",
        )?;
        let rows = statement.query_map([project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let mut affected_task_ids = BTreeSet::new();
    if let Some(task_id) = reevaluated_task_id {
        affected_task_ids.insert(task_id.to_owned());
    }
    let mut invalidated_write_ticket_ids = Vec::new();
    for (write_ticket_id, task_id, validity_basis_json) in active_tickets {
        let stored_write_authority_fingerprint =
            serde_json::from_str::<Value>(&validity_basis_json)
                .ok()
                .and_then(|basis| {
                    basis
                        .get("write_authority_fingerprint")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                });
        let policy_binding_mismatch = stored_write_authority_fingerprint.as_deref()
            != Some(resulting_write_authority_fingerprint);
        let task_reevaluation_pending = reevaluated_task_id == Some(task_id.as_str());
        if !policy_binding_mismatch && !task_reevaluation_pending {
            continue;
        }
        let updated = tx.execute(
            "UPDATE write_tickets
                SET status = 'invalidated',
                    invalidation_reason = 'explicit_revoke'
              WHERE project_id = ?1
                AND write_ticket_id = ?2
                AND status = 'active'",
            params![project_id, write_ticket_id],
        )?;
        if updated == 1 {
            affected_task_ids.insert(task_id);
            invalidated_write_ticket_ids.push(write_ticket_id);
        }
    }
    Ok((
        affected_task_ids.into_iter().collect(),
        invalidated_write_ticket_ids,
    ))
}

fn required_control_for_policy(
    project_id: &str,
    policy_json: &str,
    mode: &str,
    requested: &str,
) -> StoreResult<TaskControlLevel> {
    let policy: Value = serde_json::from_str(policy_json).map_err(|_| {
        StoreError::corrupt_owner_state_json("project_workflow_policies", project_id, "policy_json")
    })?;
    let workflow = policy
        .get("workflow")
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "policy workflow is required for active-Task reevaluation".to_owned(),
        })?;
    let control = |field: &str| -> StoreResult<TaskControlLevel> {
        let value = workflow.get(field).and_then(Value::as_str).ok_or_else(|| {
            StoreError::InvalidInput {
                detail: format!("policy workflow {field} is invalid"),
            }
        })?;
        serde_json::from_value(Value::String(value.to_owned())).map_err(|_| {
            StoreError::InvalidInput {
                detail: format!("policy workflow {field} is invalid"),
            }
        })
    };
    let direct_default = control("default_direct_control")?;
    let work_default = control("default_work_control")?;
    let light_enabled = workflow
        .get("light")
        .and_then(|light| light.get("enabled"))
        .and_then(Value::as_bool)
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "policy workflow light.enabled is invalid".to_owned(),
        })?;
    let mode: TaskMode = serde_json::from_value(Value::String(mode.to_owned()))
        .map_err(|_| StoreError::corrupt_owner_state_value("tasks", "active", "mode"))?;
    let requested: RequestedControlLevel =
        serde_json::from_value(Value::String(requested.to_owned())).map_err(|_| {
            StoreError::corrupt_owner_state_value("tasks", "active", "requested_control_level")
        })?;
    if mode == TaskMode::Advisor {
        return Ok(TaskControlLevel::Observe);
    }
    let requested_level = match requested {
        RequestedControlLevel::Auto => match mode {
            TaskMode::Advisor => TaskControlLevel::Observe,
            TaskMode::Direct => direct_default,
            TaskMode::Work => work_default,
        },
        RequestedControlLevel::Observe => TaskControlLevel::Observe,
        RequestedControlLevel::Light => TaskControlLevel::Light,
        RequestedControlLevel::Tracked => TaskControlLevel::Tracked,
        RequestedControlLevel::Sensitive => TaskControlLevel::Sensitive,
    };
    let minimum = match mode {
        TaskMode::Advisor => TaskControlLevel::Observe,
        TaskMode::Direct => direct_default,
        TaskMode::Work => std::cmp::max(work_default, TaskControlLevel::Tracked),
    };
    let mut required = std::cmp::max(requested_level, minimum);
    if required == TaskControlLevel::Light && !light_enabled {
        required = TaskControlLevel::Tracked;
    }
    Ok(required)
}

fn required_acceptance_for_policy(
    project_id: &str,
    policy_json: &str,
    required_control: TaskControlLevel,
) -> StoreResult<AcceptancePolicy> {
    match required_control {
        TaskControlLevel::Observe => Ok(AcceptancePolicy::NotRequired),
        TaskControlLevel::Tracked | TaskControlLevel::Sensitive => Ok(AcceptancePolicy::Required),
        TaskControlLevel::Light => {
            let policy: Value = serde_json::from_str(policy_json).map_err(|_| {
                StoreError::corrupt_owner_state_json(
                    "project_workflow_policies",
                    project_id,
                    "policy_json",
                )
            })?;
            let value = policy
                .get("workflow")
                .and_then(|workflow| workflow.get("light"))
                .and_then(|light| light.get("final_acceptance"))
                .cloned()
                .ok_or_else(|| StoreError::InvalidInput {
                    detail: "policy workflow light.final_acceptance is invalid".to_owned(),
                })?;
            serde_json::from_value(value).map_err(|_| StoreError::InvalidInput {
                detail: "policy workflow light.final_acceptance is invalid".to_owned(),
            })
        }
    }
}

fn acceptance_policy_rank(value: &str) -> StoreResult<u8> {
    match value {
        "not_required" => Ok(0),
        "policy_dependent" => Ok(1),
        "required" => Ok(2),
        _ => Err(StoreError::corrupt_owner_state_value(
            "tasks",
            "active",
            "acceptance_policy",
        )),
    }
}

fn acceptance_policy_from_rank(rank: u8) -> AcceptancePolicy {
    match rank {
        0 => AcceptancePolicy::NotRequired,
        1 => AcceptancePolicy::PolicyDependent,
        _ => AcceptancePolicy::Required,
    }
}

fn acceptance_policy_name(policy: AcceptancePolicy) -> &'static str {
    match policy {
        AcceptancePolicy::NotRequired => "not_required",
        AcceptancePolicy::PolicyDependent => "policy_dependent",
        AcceptancePolicy::Required => "required",
    }
}

fn task_policy_control_reevaluation_from_metadata(
    metadata_json: &str,
    task_id: &str,
) -> StoreResult<Option<TaskPolicyControlReevaluation>> {
    let metadata = task_metadata_object(metadata_json, task_id)?;
    let Some(value) = metadata.get(POLICY_CONTROL_REEVALUATION_METADATA_KEY) else {
        return Ok(None);
    };
    let mark: TaskPolicyControlReevaluation = serde_json::from_value(value.clone())
        .map_err(|_| StoreError::corrupt_owner_state_json("tasks", task_id, "metadata_json"))?;
    if mark.policy_version == 0 || !canonical_sha256(&mark.policy_fingerprint) {
        return Err(StoreError::corrupt_owner_state_json(
            "tasks",
            task_id,
            "metadata_json",
        ));
    }
    parse_control_level(
        &mark.required_effective_control_level,
        "tasks",
        task_id,
        POLICY_CONTROL_REEVALUATION_METADATA_KEY,
    )?;
    if let Some(required) = mark.required_acceptance_policy.as_deref() {
        acceptance_policy_rank(required)?;
    }
    validate_timestamp("marked_at", &mark.marked_at)
        .map_err(|_| StoreError::corrupt_owner_state_json("tasks", task_id, "metadata_json"))?;
    Ok(Some(mark))
}

fn task_metadata_object(
    metadata_json: &str,
    task_id: &str,
) -> StoreResult<serde_json::Map<String, Value>> {
    match serde_json::from_str::<Value>(metadata_json) {
        Ok(Value::Object(metadata)) => Ok(metadata),
        _ => Err(StoreError::corrupt_owner_state_json(
            "tasks",
            task_id,
            "metadata_json",
        )),
    }
}

fn parse_control_level(
    value: &str,
    entity: &'static str,
    id: &str,
    field: &'static str,
) -> StoreResult<TaskControlLevel> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| StoreError::corrupt_owner_state_value(entity, id, field))
}

fn canonical_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_policy_replacement_basis(
    project_id: &str,
    existing: Option<&ProjectWorkflowPolicyRecord>,
    policy_version: u64,
    expected_prior_fingerprint: Option<&str>,
) -> StoreResult<()> {
    let actual_prior = existing.map(|record| record.policy_fingerprint.as_str());
    if actual_prior != expected_prior_fingerprint {
        return Err(StoreError::Conflict {
            entity: "project_workflow_policy",
            id: project_id.to_owned(),
            detail: "authoritative policy fingerprint changed before apply".to_owned(),
        });
    }
    let expected_version = match existing {
        Some(record) => {
            record
                .policy_version
                .checked_add(1)
                .ok_or_else(|| StoreError::Conflict {
                    entity: "project_workflow_policy",
                    id: project_id.to_owned(),
                    detail: "policy_version is exhausted".to_owned(),
                })?
        }
        None => 1,
    };
    if policy_version != expected_version {
        return Err(StoreError::Conflict {
            entity: "project_workflow_policy",
            id: project_id.to_owned(),
            detail: "changed policy must use exactly the next policy_version".to_owned(),
        });
    }
    Ok(())
}

fn session_end_receipt_from_conn(
    conn: &rusqlite::Connection,
    project_id: &str,
    receipt_id: &str,
) -> StoreResult<Option<SessionEndReceiptRecord>> {
    let raw = conn
        .query_row(
            &format!(
                "{SESSION_END_RECEIPT_SELECT}
                   WHERE project_id = ?1
                     AND session_end_receipt_id = ?2"
            ),
            params![project_id, receipt_id],
            session_end_receipt_raw_from_row,
        )
        .optional()?;
    raw.map(session_end_receipt_from_raw).transpose()
}

fn session_end_receipt_raw_from_row(row: &Row<'_>) -> rusqlite::Result<SessionEndReceiptRaw> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
    ))
}

fn session_end_receipt_from_raw(raw: SessionEndReceiptRaw) -> StoreResult<SessionEndReceiptRecord> {
    let task_state = SessionEndTaskState::from_stable_str(&raw.4).ok_or_else(|| {
        StoreError::corrupt_owner_state_value("session_end_receipts", raw.1.clone(), "task_state")
    })?;
    let next_actor = AuthorityNextActor::from_stable_str(&raw.6).ok_or_else(|| {
        StoreError::corrupt_owner_state_value("session_end_receipts", raw.1.clone(), "next_actor")
    })?;
    let completion_claim_allowed = sqlite_bool(&raw.1, "completion_claim_allowed", raw.7)?;
    let authority_refresh_succeeded = sqlite_bool(&raw.1, "authority_refresh_succeeded", raw.8)?;
    let record = SessionEndReceiptRecord {
        project_id: raw.0,
        session_end_receipt_id: raw.1,
        managed_session_id: raw.2,
        active_task_id: raw.3,
        task_state,
        close_blocker_codes_json: raw.5,
        next_actor,
        completion_claim_allowed,
        authority_refresh_succeeded,
        created_at: raw.9,
    };
    validate_session_end_receipt(&SessionEndReceiptInsert {
        session_end_receipt_id: record.session_end_receipt_id.clone(),
        managed_session_id: record.managed_session_id.clone(),
        active_task_id: record.active_task_id.clone(),
        task_state: record.task_state,
        close_blocker_codes_json: record.close_blocker_codes_json.clone(),
        next_actor: record.next_actor,
        completion_claim_allowed: record.completion_claim_allowed,
        authority_refresh_succeeded: record.authority_refresh_succeeded,
        created_at: record.created_at.clone(),
    })
    .map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "session_end_receipts",
            record.session_end_receipt_id.clone(),
            "receipt_basis",
        )
    })?;
    Ok(record)
}

fn validate_project_workflow_policy(input: &ProjectWorkflowPolicyUpsert) -> StoreResult<()> {
    validate_project_workflow_policy_fields(
        input.policy_version,
        &input.policy_json,
        &input.policy_fingerprint,
        &input.source,
    )?;
    validate_timestamp("applied_at", &input.applied_at)?;
    validate_timestamp("created_at", &input.created_at)
}

fn validate_project_workflow_policy_fields(
    policy_version: u64,
    policy_json: &str,
    policy_fingerprint: &str,
    source: &str,
) -> StoreResult<()> {
    if policy_version == 0 {
        return invalid("policy_version must be greater than zero");
    }
    if source.trim().is_empty() {
        return invalid("policy source must not be empty");
    }
    let policy: Value =
        serde_json::from_str(policy_json).map_err(|_| StoreError::InvalidInput {
            detail: "policy_json must be valid JSON".to_owned(),
        })?;
    let canonical = canonical_json_string(&policy).map_err(|_| StoreError::InvalidInput {
        detail: "policy_json cannot be canonicalized".to_owned(),
    })?;
    if canonical != policy_json {
        return invalid("policy_json must use canonical JSON serialization");
    }
    let fingerprint = canonical_json_sha256(&policy)
        .map_err(|_| StoreError::InvalidInput {
            detail: "policy_json fingerprint could not be computed".to_owned(),
        })?
        .as_str()
        .to_owned();
    if fingerprint != policy_fingerprint {
        return invalid("policy_fingerprint must match canonical policy_json");
    }
    Ok(())
}

fn validate_session_end_receipt(input: &SessionEndReceiptInsert) -> StoreResult<()> {
    validate_identifier("session_end_receipt_id", &input.session_end_receipt_id)?;
    validate_managed_session_id(&input.managed_session_id)?;
    if let Some(task_id) = &input.active_task_id {
        validate_identifier("active_task_id", task_id)?;
    }
    validate_timestamp("created_at", &input.created_at)?;
    let blocker_codes: Vec<String> = serde_json::from_str(&input.close_blocker_codes_json)
        .map_err(|_| StoreError::InvalidInput {
            detail: "close_blocker_codes_json must be a JSON string array".to_owned(),
        })?;
    if blocker_codes.iter().any(|code| code.trim().is_empty()) {
        return invalid("close blocker codes must not be empty");
    }
    let canonical =
        canonical_json_string(&blocker_codes).map_err(|_| StoreError::InvalidInput {
            detail: "close blocker codes cannot be canonicalized".to_owned(),
        })?;
    if canonical != input.close_blocker_codes_json {
        return invalid("close_blocker_codes_json must use canonical JSON serialization");
    }
    let refresh_matches_state = if input.authority_refresh_succeeded {
        input.task_state != SessionEndTaskState::AuthorityUnknown
    } else {
        input.task_state == SessionEndTaskState::AuthorityUnknown
    };
    if !refresh_matches_state {
        return invalid("authority refresh result and task_state are inconsistent");
    }
    if input.task_state == SessionEndTaskState::None && input.active_task_id.is_some() {
        return invalid("task_state none requires no active_task_id");
    }
    if !matches!(
        input.task_state,
        SessionEndTaskState::None | SessionEndTaskState::AuthorityUnknown
    ) && input.active_task_id.is_none()
    {
        return invalid("the selected task_state requires active_task_id");
    }
    if input.completion_claim_allowed
        && (!input.authority_refresh_succeeded
            || input.task_state != SessionEndTaskState::Ready
            || input.active_task_id.is_none()
            || !blocker_codes.is_empty())
    {
        return invalid("completion_claim_allowed lacks a ready blocker-free authority basis");
    }
    Ok(())
}

fn validate_managed_session_id(value: &str) -> StoreResult<()> {
    validate_managed_host_session_id(value).map_err(|_| StoreError::InvalidInput {
        detail: "managed_session_id must be a canonical managed-host session ID".to_owned(),
    })
}

fn validate_identifier(field: &str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() || value.as_bytes().contains(&0) {
        invalid(format!("{field} must not be empty or contain NUL"))
    } else {
        Ok(())
    }
}

fn validate_timestamp(field: &str, value: &str) -> StoreResult<()> {
    let timestamp = UtcTimestamp::parse(value).map_err(|_| StoreError::InvalidInput {
        detail: format!("{field} must be a canonical RFC 3339 UTC timestamp"),
    })?;
    timestamp
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::InvalidInput {
            detail: format!("{field} must be a canonical RFC 3339 UTC timestamp"),
        })
}

fn sqlite_bool(record_ref: &str, field: &'static str, value: i64) -> StoreResult<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(StoreError::corrupt_owner_state_value(
            "session_end_receipts",
            record_ref.to_owned(),
            field,
        )),
    }
}

fn require_writable(store: &CoreProjectStore) -> StoreResult<()> {
    if store.writable {
        Ok(())
    } else {
        invalid("the Core project store is read-only")
    }
}

fn invalid<T>(detail: impl Into<String>) -> StoreResult<T> {
    Err(StoreError::InvalidInput {
        detail: detail.into(),
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;
    use volicord_test_support::core_fixtures::CoreFixture;
    use volicord_types::{canonical_json_sha256, canonical_json_string, ProjectId};

    use super::*;
    use crate::core_pipeline::CoreStorageMutation;

    fn workflow_policy(
        default_direct_control: &str,
        light_enabled: bool,
    ) -> Result<(String, String), Box<dyn Error>> {
        workflow_policy_with_acceptance(default_direct_control, light_enabled, "policy_dependent")
    }

    fn workflow_policy_with_acceptance(
        default_direct_control: &str,
        light_enabled: bool,
        final_acceptance: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        let value = json!({
            "schema": "volicord-policy-v2",
            "workflow": {
                "default_direct_control": default_direct_control,
                "default_work_control": "tracked",
                "light": {
                    "enabled": light_enabled,
                    "max_intended_paths": 3,
                    "allowed_path_patterns": [],
                    "denied_path_patterns": [],
                    "final_acceptance": final_acceptance
                },
                "write_ticket": {"idle_timeout_minutes": null},
                "detective": {
                    "unknown_effect_behavior": "warn",
                    "stop_behavior": "allow_with_disclosure"
                }
            }
        });
        Ok((
            canonical_json_string(&value)?,
            canonical_json_sha256(&value)?.into_inner(),
        ))
    }

    fn workflow_policy_with_write_authority(
        max_intended_paths: u64,
        allowed_path_patterns: Vec<&str>,
        denied_path_patterns: Vec<&str>,
        idle_timeout_minutes: Option<u64>,
        unknown_effect_behavior: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        let value = json!({
            "schema": "volicord-policy-v2",
            "workflow": {
                "default_direct_control": "light",
                "default_work_control": "light",
                "light": {
                    "enabled": true,
                    "max_intended_paths": max_intended_paths,
                    "allowed_path_patterns": allowed_path_patterns,
                    "denied_path_patterns": denied_path_patterns,
                    "final_acceptance": "policy_dependent"
                },
                "write_ticket": {"idle_timeout_minutes": idle_timeout_minutes},
                "detective": {
                    "unknown_effect_behavior": unknown_effect_behavior,
                    "stop_behavior": "allow_with_disclosure"
                }
            }
        });
        Ok((
            canonical_json_string(&value)?,
            canonical_json_sha256(&value)?.into_inner(),
        ))
    }

    #[test]
    fn workflow_policy_and_session_end_receipt_round_trip() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("workflow-record-round-trip")?;
        let mut store = CoreProjectStore::open(
            fixture.runtime_home_path(),
            &ProjectId::new(fixture.project_id()),
        )?;

        let policy_value = json!({
            "schema": "volicord-policy-v2",
            "workflow": {
                "default_direct_control": "tracked",
                "default_work_control": "tracked",
                "light": {
                    "enabled": false,
                    "max_intended_paths": 3,
                    "allowed_path_patterns": [],
                    "denied_path_patterns": [],
                    "final_acceptance": "policy_dependent"
                },
                "write_ticket": {"idle_timeout_minutes": null},
                "detective": {
                    "unknown_effect_behavior": "warn",
                    "stop_behavior": "allow_with_disclosure"
                }
            }
        });
        let policy_json = canonical_json_string(&policy_value)?;
        let policy_fingerprint = canonical_json_sha256(&policy_value)?.as_str().to_owned();
        let policy = store.upsert_project_workflow_policy(ProjectWorkflowPolicyUpsert {
            policy_version: 1,
            policy_json: policy_json.clone(),
            policy_fingerprint: policy_fingerprint.clone(),
            source: "volicord_init".to_owned(),
            applied_at: "2026-07-16T00:00:00Z".to_owned(),
            created_at: "2026-07-16T00:00:00Z".to_owned(),
        })?;
        assert_eq!(policy.policy_schema, "volicord-policy-v2");
        assert_eq!(policy.policy_json, policy_json);
        assert_eq!(policy.policy_fingerprint, policy_fingerprint);
        assert_eq!(store.project_workflow_policy()?, Some(policy));

        let managed_session_id = format!("mhs_{}", "a".repeat(64));
        store.conn.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode,
                requested_control_level, effective_control_level, control_level_reason,
                work_phase, acceptance_policy, acceptance_policy_reason,
                lifecycle_phase, created_at, updated_at
             ) VALUES (?1, 'task_session_end', ?2, 'work', 'tracked', 'tracked',
                       'Session-end fixture control.', 'implementation', 'required',
                       'Session-end fixture acceptance.', 'implementation',
                       '2026-07-16T00:00:00Z', '2026-07-16T00:00:00Z')",
            params![fixture.project_id(), fixture.actor_source()],
        )?;
        store.conn.execute(
            "INSERT INTO agent_sessions (
                project_id, session_id, connection_internal_id, host_kind,
                guard_mode, started_at, metadata_json
             ) VALUES (?1, ?2, 'conn_session_end', 'codex', 'record',
                       '2026-07-16T00:00:00Z', '{}')",
            params![fixture.project_id(), managed_session_id],
        )?;

        let receipt = store.insert_session_end_receipt(SessionEndReceiptInsert {
            session_end_receipt_id: "session_end_receipt_a".to_owned(),
            managed_session_id: managed_session_id.clone(),
            active_task_id: Some("task_session_end".to_owned()),
            task_state: SessionEndTaskState::Ready,
            close_blocker_codes_json: "[]".to_owned(),
            next_actor: AuthorityNextActor::None,
            completion_claim_allowed: true,
            authority_refresh_succeeded: true,
            created_at: "2026-07-16T00:01:00Z".to_owned(),
        })?;
        assert!(receipt.completion_claim_allowed);
        assert_eq!(
            store.latest_session_end_receipt_for_session(&managed_session_id)?,
            Some(receipt.clone())
        );
        assert_eq!(
            store.session_end_receipt("session_end_receipt_a")?,
            Some(receipt)
        );

        let invalid_receipt = store.insert_session_end_receipt(SessionEndReceiptInsert {
            session_end_receipt_id: "session_end_receipt_invalid".to_owned(),
            managed_session_id,
            active_task_id: Some("task_session_end".to_owned()),
            task_state: SessionEndTaskState::Ready,
            close_blocker_codes_json: r#"["acceptance_required"]"#.to_owned(),
            next_actor: AuthorityNextActor::User,
            completion_claim_allowed: true,
            authority_refresh_succeeded: true,
            created_at: "2026-07-16T00:02:00Z".to_owned(),
        });
        assert!(matches!(
            invalid_receipt,
            Err(StoreError::InvalidInput { .. })
        ));
        Ok(())
    }

    #[test]
    fn workflow_policy_apply_is_atomic_versioned_and_preserves_stronger_task_mark(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("workflow-policy-atomic-authority")?;
        let mut store = CoreProjectStore::open(
            fixture.runtime_home_path(),
            &ProjectId::new(fixture.project_id()),
        )?;
        store.conn.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode,
                requested_control_level, effective_control_level, control_level_reason,
                work_phase, acceptance_policy, acceptance_policy_reason,
                lifecycle_phase, created_at, updated_at
             ) VALUES (?1, 'task_policy_active', ?2, 'direct', 'auto', 'observe',
                       'Initial observe control.', 'implementation', 'not_required',
                       'Observe control needs no acceptance.', 'implementation',
                       '2026-07-16T00:00:00Z', '2026-07-16T00:00:00Z')",
            params![fixture.project_id(), fixture.actor_source()],
        )?;
        store.conn.execute(
            "UPDATE project_state SET active_task_id = 'task_policy_active' WHERE project_id = ?1",
            [fixture.project_id()],
        )?;

        let (observe_json, observe_fingerprint) = workflow_policy("observe", false)?;
        let initial =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 1,
                policy_json: observe_json.clone(),
                policy_fingerprint: observe_fingerprint.clone(),
                source: "project_database".to_owned(),
                expected_prior_fingerprint: None,
            })?;
        assert!(initial.database_changed);
        assert_eq!(initial.basis_state_version, 0);
        assert_eq!(initial.resulting_state_version, 1);
        assert!(!initial.active_task_requires_escalation);
        assert!(initial.active_task_requires_policy_reevaluation);
        assert!(initial.write_authority_changed);
        let (event_task_id, event_created_at): (Option<String>, String) = store.conn.query_row(
            "SELECT task_id, created_at FROM authority_events WHERE event_seq = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(event_task_id, None);
        assert_eq!(event_created_at, initial.policy.applied_at);
        assert_eq!(store.project_state()?.updated_at, initial.policy.applied_at);
        assert_eq!(store.effect_counts()?.task_events, 0);

        let replay =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 1,
                policy_json: observe_json,
                policy_fingerprint: observe_fingerprint.clone(),
                source: "project_database".to_owned(),
                expected_prior_fingerprint: Some(observe_fingerprint.clone()),
            })?;
        assert!(!replay.database_changed);
        assert_eq!(replay.resulting_state_version, 1);
        assert!(!replay.write_authority_changed);
        assert!(replay.invalidated_write_ticket_ids.is_empty());
        assert_eq!(store.effect_counts()?.task_events, 0);
        let authority_event_count: i64 =
            store
                .conn
                .query_row("SELECT COUNT(*) FROM authority_events", [], |row| {
                    row.get(0)
                })?;
        assert_eq!(authority_event_count, 1);

        store.conn.execute(
            "INSERT INTO write_tickets (
                project_id, write_ticket_id, task_id, change_unit_id,
                basis_state_version, status, validity_basis_json,
                allowed_path_prefixes_json, denied_path_prefixes_json,
                attempt_scope_json, created_by_actor_source,
                created_by_user_action_resolution_id, idle_expires_at,
                invalidation_reason, consumed_by_run_id, consumed_at,
                revoked_at, created_at, metadata_json
             ) VALUES (?1, 'ticket_policy_before_raise', 'task_policy_active', NULL,
                       1, 'active', '{}', '[]', '[]', '{}', ?2,
                       NULL, NULL, NULL, NULL, NULL, NULL,
                       '2026-07-16T00:00:00Z', '{}')",
            params![fixture.project_id(), fixture.actor_source()],
        )?;

        let (tracked_json, tracked_fingerprint) = workflow_policy("tracked", false)?;
        let strengthened =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 2,
                policy_json: tracked_json,
                policy_fingerprint: tracked_fingerprint.clone(),
                source: "project_database".to_owned(),
                expected_prior_fingerprint: Some(observe_fingerprint),
            })?;
        assert_eq!(strengthened.resulting_state_version, 2);
        assert!(strengthened.active_task_requires_escalation);
        assert_eq!(
            strengthened.affected_task_ids,
            vec!["task_policy_active".to_owned()]
        );
        assert_eq!(
            strengthened.invalidated_write_ticket_ids,
            vec!["ticket_policy_before_raise".to_owned()]
        );
        let marked =
            task_policy_control_reevaluation(&store.active_task_record()?.expect("active Task"))?
                .expect("strengthened policy must mark the active Task");
        assert_eq!(marked.policy_version, 2);
        assert_eq!(marked.policy_fingerprint, tracked_fingerprint);
        assert_eq!(marked.required_effective_control_level, "tracked");
        assert_eq!(marked.marked_at, strengthened.policy.applied_at);
        let invalidated_ticket: (String, Option<String>) = store.conn.query_row(
            "SELECT status, invalidation_reason
               FROM write_tickets
              WHERE write_ticket_id = 'ticket_policy_before_raise'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(invalidated_ticket.0, "invalidated");
        assert_eq!(invalidated_ticket.1.as_deref(), Some("explicit_revoke"));

        let (relaxed_json, relaxed_fingerprint) = workflow_policy("light", true)?;
        let relaxed =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 3,
                policy_json: relaxed_json,
                policy_fingerprint: relaxed_fingerprint,
                source: "project_database".to_owned(),
                expected_prior_fingerprint: Some(tracked_fingerprint.clone()),
            })?;
        assert_eq!(relaxed.resulting_state_version, 3);
        assert!(relaxed.active_task_requires_escalation);
        let preserved =
            task_policy_control_reevaluation(&store.active_task_record()?.expect("active Task"))?
                .expect("relaxed policy must preserve the stronger mark");
        assert_eq!(preserved.policy_version, 3);
        assert_eq!(
            preserved.policy_fingerprint,
            relaxed.policy.policy_fingerprint
        );
        assert_eq!(preserved.required_effective_control_level, "tracked");

        let marker_commit = CommitMutationInput {
            project_id: fixture.project_id().to_owned(),
            tool_name: "store_test_policy_marker_raise".to_owned(),
            idempotency_key: None,
            request_hash: "request_policy_marker_raise".to_owned(),
            replay_context: Some(VerifiedReplayContext {
                actor_source: "local_user".to_owned(),
                operation_category: "admin_local".to_owned(),
                verification_basis: Some("store_test".to_owned()),
                git_workspace_context_json: None,
            }),
            expected_state_version: Some(3),
            clock_floor: None,
            include_live_storage_time: true,
            events: vec![PendingTaskEvent {
                event_id: "evt_policy_marker_raise".to_owned(),
                task_id: Some("task_policy_active".to_owned()),
                change_unit_id: None,
                event_kind: "store_test_policy_marker_raised".to_owned(),
                event_payload_json: "{}".to_owned(),
            }],
        };
        let marker_raise = crate::core_pipeline::TaskControlLevelUpdate {
            task_id: "task_policy_active".to_owned(),
            effective_control_level: "tracked".to_owned(),
            control_level_reason: "Raised for pending project policy reevaluation.".to_owned(),
            acceptance_policy: Some("required".to_owned()),
            acceptance_policy_reason: Some("Tracked control requires acceptance.".to_owned()),
        };
        let outcome = store.commit_mutation(
            marker_commit,
            |mutation, facts| {
                CoreStorageMutation::UpdateTaskControlLevel(marker_raise)
                    .apply(mutation, facts.committed_state_version)
            },
            |_| Ok("{}".to_owned()),
        )?;
        assert!(matches!(outcome, MutationCommitOutcome::Committed { .. }));
        let raised = store.active_task_record()?.expect("active Task");
        assert_eq!(raised.effective_control_level, "tracked");
        assert_eq!(task_policy_control_reevaluation(&raised)?, None);
        assert_eq!(store.project_state()?.state_version, 4);
        Ok(())
    }

    #[test]
    fn write_authority_tightening_atomically_revokes_active_tickets() -> Result<(), Box<dyn Error>>
    {
        struct Case {
            name: &'static str,
            initial_max_paths: u64,
            initial_allowed: Vec<&'static str>,
            initial_denied: Vec<&'static str>,
            initial_timeout: Option<u64>,
            ticket_paths: Vec<&'static str>,
            tightened_max_paths: u64,
            tightened_allowed: Vec<&'static str>,
            tightened_denied: Vec<&'static str>,
            tightened_timeout: Option<u64>,
        }

        let cases = [
            Case {
                name: "policy-binding-denied-path",
                initial_max_paths: 3,
                initial_allowed: vec!["src/**"],
                initial_denied: vec![],
                initial_timeout: None,
                ticket_paths: vec!["src/export.rs"],
                tightened_max_paths: 3,
                tightened_allowed: vec!["src/**"],
                tightened_denied: vec!["src/export.rs"],
                tightened_timeout: None,
            },
            Case {
                name: "policy-binding-allowed-path",
                initial_max_paths: 3,
                initial_allowed: vec!["src/**"],
                initial_denied: vec![],
                initial_timeout: None,
                ticket_paths: vec!["src/export.rs"],
                tightened_max_paths: 3,
                tightened_allowed: vec!["tests/**"],
                tightened_denied: vec![],
                tightened_timeout: None,
            },
            Case {
                name: "policy-binding-max-paths",
                initial_max_paths: 3,
                initial_allowed: vec!["src/**"],
                initial_denied: vec![],
                initial_timeout: None,
                ticket_paths: vec!["src/export.rs", "src/import.rs"],
                tightened_max_paths: 1,
                tightened_allowed: vec!["src/**"],
                tightened_denied: vec![],
                tightened_timeout: None,
            },
            Case {
                name: "policy-binding-timeout",
                initial_max_paths: 3,
                initial_allowed: vec!["src/**"],
                initial_denied: vec![],
                initial_timeout: None,
                ticket_paths: vec!["src/export.rs"],
                tightened_max_paths: 3,
                tightened_allowed: vec!["src/**"],
                tightened_denied: vec![],
                tightened_timeout: Some(5),
            },
        ];

        for case in cases {
            let ticket_path_count = u64::try_from(case.ticket_paths.len())?;
            assert!(ticket_path_count <= case.initial_max_paths, "{}", case.name);
            if case.name == "policy-binding-max-paths" {
                assert!(ticket_path_count > case.tightened_max_paths);
            }
            let fixture = CoreFixture::new(case.name)?;
            let mut store = CoreProjectStore::open(
                fixture.runtime_home_path(),
                &ProjectId::new(fixture.project_id()),
            )?;
            let (initial_json, initial_fingerprint) = workflow_policy_with_write_authority(
                case.initial_max_paths,
                case.initial_allowed,
                case.initial_denied,
                case.initial_timeout,
                "warn",
            )?;
            let initial = store.apply_project_workflow_policy_authority(
                ProjectWorkflowPolicyAuthorityApply {
                    policy_version: 1,
                    policy_json: initial_json,
                    policy_fingerprint: initial_fingerprint.clone(),
                    source: "project_database".to_owned(),
                    expected_prior_fingerprint: None,
                },
            )?;
            store.conn.execute(
                "INSERT INTO tasks (
                    project_id, task_id, created_by_actor_source, mode,
                    requested_control_level, effective_control_level, control_level_reason,
                    work_phase, acceptance_policy, acceptance_policy_reason,
                    lifecycle_phase, created_at, updated_at
                 ) VALUES (?1, 'task_policy_binding', ?2, 'direct', 'light', 'light',
                           'Light policy fixture control.', 'implementation',
                           'policy_dependent', 'Light policy fixture acceptance.',
                           'implementation', '2026-07-17T00:00:00Z',
                           '2026-07-17T00:00:00Z')",
                params![fixture.project_id(), fixture.actor_source()],
            )?;
            store.conn.execute(
                "UPDATE project_state
                    SET active_task_id = 'task_policy_binding'
                  WHERE project_id = ?1",
                [fixture.project_id()],
            )?;
            let validity_basis_json = canonical_json_string(&json!({
                "task_id": "task_policy_binding",
                "change_unit_id": "cu_policy_binding",
                "scope_revision": 1,
                "baseline_ref": null,
                "workspace_context_sha256": null,
                "write_authority_fingerprint": initial.resulting_write_authority_fingerprint,
                "approval_basis_refs": []
            }))?;
            let allowed_path_prefixes_json = canonical_json_string(&case.ticket_paths)?;
            store.conn.execute(
                "INSERT INTO write_tickets (
                    project_id, write_ticket_id, task_id, change_unit_id,
                    basis_state_version, status, validity_basis_json,
                    allowed_path_prefixes_json, denied_path_prefixes_json,
                    attempt_scope_json, created_by_actor_source,
                    created_by_user_action_resolution_id, idle_expires_at,
                    invalidation_reason, consumed_by_run_id, consumed_at,
                    revoked_at, created_at, metadata_json
                 ) VALUES (?1, 'ticket_policy_binding', 'task_policy_binding', NULL,
                           1, 'active', ?2, ?3, '[]', '{}', ?4,
                           NULL, NULL, NULL, NULL, NULL, NULL,
                           '2026-07-17T00:00:00Z', '{}')",
                params![
                    fixture.project_id(),
                    validity_basis_json,
                    allowed_path_prefixes_json,
                    fixture.actor_source()
                ],
            )?;

            let (tightened_json, tightened_fingerprint) = workflow_policy_with_write_authority(
                case.tightened_max_paths,
                case.tightened_allowed,
                case.tightened_denied,
                case.tightened_timeout,
                "warn",
            )?;
            let tightened = store.apply_project_workflow_policy_authority(
                ProjectWorkflowPolicyAuthorityApply {
                    policy_version: 2,
                    policy_json: tightened_json,
                    policy_fingerprint: tightened_fingerprint,
                    source: "project_database".to_owned(),
                    expected_prior_fingerprint: Some(initial_fingerprint),
                },
            )?;

            assert!(tightened.write_authority_changed, "{}", case.name);
            assert_ne!(
                tightened.prior_write_authority_fingerprint,
                tightened.resulting_write_authority_fingerprint,
                "{}",
                case.name
            );
            assert_eq!(
                tightened.affected_task_ids,
                vec!["task_policy_binding".to_owned()],
                "{}",
                case.name
            );
            assert_eq!(
                tightened.invalidated_write_ticket_ids,
                vec!["ticket_policy_binding".to_owned()],
                "{}",
                case.name
            );
            let (status, reason): (String, Option<String>) = store.conn.query_row(
                "SELECT status, invalidation_reason
                   FROM write_tickets
                  WHERE project_id = ?1
                    AND write_ticket_id = 'ticket_policy_binding'",
                [fixture.project_id()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            assert_eq!(status, "invalidated", "{}", case.name);
            assert_eq!(reason.as_deref(), Some("explicit_revoke"), "{}", case.name);
            let active_task = store.active_task_record()?.expect("active Task");
            let mark = task_policy_control_reevaluation(&active_task)?
                .expect("write-authority changes must mark the active Task");
            assert_eq!(
                mark.required_effective_control_level, "light",
                "{}",
                case.name
            );
            assert!(
                tightened.active_task_requires_policy_reevaluation,
                "{}",
                case.name
            );
            assert!(
                !tightened.active_task_requires_escalation,
                "same-level reevaluation is not a control escalation: {}",
                case.name
            );
        }
        Ok(())
    }

    #[test]
    fn normalized_equivalent_write_authority_preserves_compatible_active_ticket(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("policy-binding-normalized-idempotency")?;
        let mut store = CoreProjectStore::open(
            fixture.runtime_home_path(),
            &ProjectId::new(fixture.project_id()),
        )?;
        let (initial_json, initial_fingerprint) = workflow_policy_with_write_authority(
            3,
            vec!["src/**", "tests/**"],
            vec!["target/**", "vendor/**"],
            Some(30),
            "warn",
        )?;
        let initial =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 1,
                policy_json: initial_json,
                policy_fingerprint: initial_fingerprint.clone(),
                source: "project_database".to_owned(),
                expected_prior_fingerprint: None,
            })?;
        store.conn.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode,
                requested_control_level, effective_control_level, control_level_reason,
                work_phase, acceptance_policy, acceptance_policy_reason,
                lifecycle_phase, created_at, updated_at
             ) VALUES (?1, 'task_policy_equivalent', ?2, 'direct', 'light', 'light',
                       'Light policy fixture control.', 'implementation',
                       'policy_dependent', 'Light policy fixture acceptance.',
                       'implementation', '2026-07-17T00:00:00Z',
                       '2026-07-17T00:00:00Z')",
            params![fixture.project_id(), fixture.actor_source()],
        )?;
        store.conn.execute(
            "UPDATE project_state
                SET active_task_id = 'task_policy_equivalent'
              WHERE project_id = ?1",
            [fixture.project_id()],
        )?;
        let validity_basis_json = canonical_json_string(&json!({
            "task_id": "task_policy_equivalent",
            "change_unit_id": "cu_policy_equivalent",
            "scope_revision": 1,
            "baseline_ref": null,
            "workspace_context_sha256": null,
            "write_authority_fingerprint": initial.resulting_write_authority_fingerprint,
            "approval_basis_refs": []
        }))?;
        store.conn.execute(
            "INSERT INTO write_tickets (
                project_id, write_ticket_id, task_id, change_unit_id,
                basis_state_version, status, validity_basis_json,
                allowed_path_prefixes_json, denied_path_prefixes_json,
                attempt_scope_json, created_by_actor_source,
                created_by_user_action_resolution_id, idle_expires_at,
                invalidation_reason, consumed_by_run_id, consumed_at,
                revoked_at, created_at, metadata_json
             ) VALUES (?1, 'ticket_policy_equivalent', 'task_policy_equivalent', NULL,
                       1, 'active', ?2, '[\"src/export.rs\"]', '[]', '{}', ?3,
                       NULL, NULL, NULL, NULL, NULL, NULL,
                       '2026-07-17T00:00:00Z', '{}')",
            params![
                fixture.project_id(),
                validity_basis_json,
                fixture.actor_source()
            ],
        )?;

        let (equivalent_json, equivalent_fingerprint) = workflow_policy_with_write_authority(
            3,
            vec!["tests/**", "src/**", "src/**"],
            vec!["vendor/**", "target/**", "target/**"],
            Some(30),
            "warn",
        )?;
        assert_ne!(initial_fingerprint, equivalent_fingerprint);
        let equivalent =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 2,
                policy_json: equivalent_json.clone(),
                policy_fingerprint: equivalent_fingerprint.clone(),
                source: "project_database".to_owned(),
                expected_prior_fingerprint: Some(initial_fingerprint),
            })?;
        assert!(equivalent.database_changed);
        assert!(!equivalent.write_authority_changed);
        assert_eq!(
            equivalent.prior_write_authority_fingerprint,
            equivalent.resulting_write_authority_fingerprint
        );
        assert!(equivalent.affected_task_ids.is_empty());
        assert!(equivalent.invalidated_write_ticket_ids.is_empty());
        assert!(!equivalent.active_task_requires_policy_reevaluation);
        let status: String = store.conn.query_row(
            "SELECT status
               FROM write_tickets
              WHERE project_id = ?1
                AND write_ticket_id = 'ticket_policy_equivalent'",
            [fixture.project_id()],
            |row| row.get(0),
        )?;
        assert_eq!(status, "active");

        let replay =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 2,
                policy_json: equivalent_json,
                policy_fingerprint: equivalent_fingerprint.clone(),
                source: "project_database".to_owned(),
                expected_prior_fingerprint: Some(equivalent_fingerprint),
            })?;
        assert!(!replay.database_changed);
        assert!(!replay.write_authority_changed);
        assert_eq!(
            replay.resulting_write_authority_fingerprint,
            equivalent.resulting_write_authority_fingerprint
        );
        assert!(replay.invalidated_write_ticket_ids.is_empty());
        Ok(())
    }

    #[test]
    fn workflow_policy_exact_fingerprint_observation_excludes_concurrent_writers(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("workflow-policy-no-op-serialized-observation")?;
        let project_id = ProjectId::new(fixture.project_id());
        let mut store = CoreProjectStore::open(fixture.runtime_home_path(), &project_id)?;
        let (policy_json, policy_fingerprint) = workflow_policy("tracked", false)?;
        let initial =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 1,
                policy_json: policy_json.clone(),
                policy_fingerprint: policy_fingerprint.clone(),
                source: "project_database".to_owned(),
                expected_prior_fingerprint: None,
            })?;
        assert_eq!(initial.resulting_state_version, 1);

        let observation = store.workflow_policy_apply_observation_with_hook(|| {
            let mut concurrent = CoreProjectStore::open(
                fixture.runtime_home_path(),
                &ProjectId::new(fixture.project_id()),
            )
            .expect("concurrent store opens while the observation transaction is active");
            concurrent
                .conn
                .busy_timeout(std::time::Duration::ZERO)
                .expect("busy timeout is configurable");
            let concurrent_writer = begin_immediate_transaction(&mut concurrent.conn);
            assert!(
                concurrent_writer.is_err(),
                "the observation transaction must exclude a policy writer between its reads"
            );
        })?;
        assert_eq!(observation.state_version, 1);
        assert_eq!(
            observation
                .prior
                .as_ref()
                .map(|policy| policy.policy_fingerprint.as_str()),
            Some(policy_fingerprint.as_str())
        );

        let replay =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 1,
                policy_json,
                policy_fingerprint: policy_fingerprint.clone(),
                source: "project_database".to_owned(),
                expected_prior_fingerprint: Some(policy_fingerprint),
            })?;
        assert!(!replay.database_changed);
        assert_eq!(replay.basis_state_version, 1);
        assert_eq!(replay.resulting_state_version, 1);
        let authority_event_count: i64 =
            store
                .conn
                .query_row("SELECT COUNT(*) FROM authority_events", [], |row| {
                    row.get(0)
                })?;
        assert_eq!(authority_event_count, 1);
        Ok(())
    }

    #[test]
    fn workflow_policy_marks_acceptance_only_strengthening() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("workflow-policy-acceptance-strengthening")?;
        let mut store = CoreProjectStore::open(
            fixture.runtime_home_path(),
            &ProjectId::new(fixture.project_id()),
        )?;
        store.conn.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode,
                requested_control_level, effective_control_level, control_level_reason,
                work_phase, acceptance_policy, acceptance_policy_reason,
                lifecycle_phase, created_at, updated_at
             ) VALUES (?1, 'task_policy_acceptance', ?2, 'direct', 'light', 'light',
                       'Initial Light control.', 'implementation', 'policy_dependent',
                       'Initial policy-dependent acceptance.', 'implementation',
                       '2026-07-16T00:00:00Z', '2026-07-16T00:00:00Z')",
            params![fixture.project_id(), fixture.actor_source()],
        )?;
        store.conn.execute(
            "UPDATE project_state SET active_task_id = 'task_policy_acceptance' WHERE project_id = ?1",
            [fixture.project_id()],
        )?;
        let (initial_json, initial_fingerprint) =
            workflow_policy_with_acceptance("light", true, "policy_dependent")?;
        let initial =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 1,
                policy_json: initial_json,
                policy_fingerprint: initial_fingerprint.clone(),
                source: "project_database".to_owned(),
                expected_prior_fingerprint: None,
            })?;
        assert!(!initial.active_task_requires_escalation);

        let (required_json, required_fingerprint) =
            workflow_policy_with_acceptance("light", true, "required")?;
        let strengthened =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 2,
                policy_json: required_json,
                policy_fingerprint: required_fingerprint.clone(),
                source: "project_database".to_owned(),
                expected_prior_fingerprint: Some(initial_fingerprint),
            })?;

        assert!(strengthened.active_task_requires_escalation);
        let mark =
            task_policy_control_reevaluation(&store.active_task_record()?.expect("active Task"))?
                .expect("acceptance-only strengthening must mark the active Task");
        assert_eq!(mark.required_effective_control_level, "light");
        assert_eq!(mark.required_acceptance_policy.as_deref(), Some("required"));
        assert_eq!(mark.policy_fingerprint, required_fingerprint);
        let task = store.active_task_record()?.expect("active Task");
        let still_marked = clear_satisfied_task_policy_reevaluation(
            &task.metadata_json,
            &task.task_id,
            "light",
            "policy_dependent",
        )?;
        assert!(
            task_policy_control_reevaluation_from_metadata(&still_marked, &task.task_id)?.is_some()
        );
        let cleared = clear_satisfied_task_policy_reevaluation(
            &still_marked,
            &task.task_id,
            "light",
            "required",
        )?;
        assert!(task_policy_control_reevaluation_from_metadata(&cleared, &task.task_id)?.is_none());
        Ok(())
    }
}
