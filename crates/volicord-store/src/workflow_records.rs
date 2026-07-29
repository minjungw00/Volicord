//! Store-owned workflow-policy records.

use std::collections::BTreeSet;

use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use volicord_types::canonical::{canonical_json_sha256, canonical_json_string};
use volicord_types::schema::{WORKFLOW_POLICY_CONTRACT_ID, WRITE_AUTHORITY_CONTRACT_ID};
use volicord_types::values::{
    AcceptancePolicy, ActorSource, OperationCategory, RequestedControlLevel, TaskControlLevel,
    TaskMode, UtcTimestamp,
};
use volicord_types::workflow_policy::{ProjectWorkflowPolicy, ProjectWorkflowPolicySource};

use crate::{
    core_pipeline::{
        active_write_ticket_authority_bindings_in_tx, invalidate_active_write_ticket_ids_in_tx,
        CommitMutationInput, CoreProjectStore, CoreStorageMutation, MutationCommitOutcome,
        PendingTaskEvent, TaskRecord, VerifiedReplayContext,
    },
    sqlite::begin_immediate_transaction,
    StoreError, StoreResult,
};

pub const POLICY_CONTROL_REEVALUATION_METADATA_KEY: &str = "policy_control_reevaluation";
const POLICY_APPLIED_EVENT_KIND: &str = "project_workflow_policy_applied";

/// Workflow-policy mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowPolicyMutation {
    Apply(ProjectWorkflowPolicyMutation),
}

/// Storage input for one authority-bound project workflow-policy replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkflowPolicyMutation {
    pub policy_version: u64,
    pub policy: ProjectWorkflowPolicy,
    pub policy_fingerprint: String,
    pub source: ProjectWorkflowPolicySource,
    pub expected_prior_fingerprint: Option<String>,
}

impl WorkflowPolicyMutation {
    pub(crate) fn apply(
        &self,
        context: &crate::core_pipeline::mutations::MutationContext<'_>,
    ) -> StoreResult<ProjectWorkflowPolicyMutationEffect> {
        match self {
            Self::Apply(input) => apply_project_workflow_policy_mutation(
                context.transaction(),
                context.project_id(),
                context.committed_at(),
                input,
            ),
        }
    }
}

/// Authoritative project workflow-policy replacement input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkflowPolicyUpsert {
    pub policy_version: u64,
    pub policy: ProjectWorkflowPolicy,
    pub policy_fingerprint: String,
    pub source: ProjectWorkflowPolicySource,
    pub applied_at: UtcTimestamp,
    pub created_at: UtcTimestamp,
}

/// Atomic administrative workflow-policy authority application input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkflowPolicyAuthorityApply {
    pub policy_version: u64,
    pub policy: ProjectWorkflowPolicy,
    pub policy_fingerprint: String,
    pub source: ProjectWorkflowPolicySource,
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

/// Closed durable active-Task policy control reevaluation mark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskPolicyControlReevaluation {
    pub policy_version: u64,
    pub policy_fingerprint: String,
    pub required_effective_control_level: TaskControlLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_acceptance_policy: Option<AcceptancePolicy>,
    pub marked_at: UtcTimestamp,
}

/// Current authoritative project workflow-policy row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkflowPolicyRecord {
    pub project_id: String,
    pub policy_version: u64,
    pub policy: ProjectWorkflowPolicy,
    pub policy_fingerprint: String,
    pub source: ProjectWorkflowPolicySource,
    pub applied_at: UtcTimestamp,
    pub created_at: UtcTimestamp,
    pub write_authority_fingerprint: String,
}

#[derive(Debug)]
struct ProjectWorkflowPolicyRow {
    project_id: String,
    policy_schema: String,
    policy_version: i64,
    policy_json: String,
    policy_fingerprint: String,
    source: String,
    applied_at: String,
    created_at: String,
}

/// Derives the write-authority digest from an optional typed workflow policy.
pub fn project_write_authority_fingerprint(
    policy: Option<&ProjectWorkflowPolicy>,
) -> StoreResult<String> {
    let mut basis = match policy {
        Some(policy) => {
            policy.validate().map_err(|_| StoreError::InvalidInput {
                detail: "project workflow policy cannot produce a write-authority binding"
                    .to_owned(),
            })?;
            ProjectWriteAuthorityFingerprintBasis {
                schema: WRITE_AUTHORITY_CONTRACT_ID,
                default_direct_control: policy.workflow.default_direct_control,
                default_work_control: policy.workflow.default_work_control,
                light: ProjectWriteAuthorityLightBasis {
                    enabled: policy.workflow.light.enabled,
                    max_intended_paths: policy.workflow.light.max_intended_paths,
                    allowed_path_patterns: policy
                        .workflow
                        .light
                        .allowed_path_patterns
                        .iter()
                        .map(|path| path.as_str().to_owned())
                        .collect(),
                    denied_path_patterns: policy
                        .workflow
                        .light
                        .denied_path_patterns
                        .iter()
                        .map(|path| path.as_str().to_owned())
                        .collect(),
                    final_acceptance: policy.workflow.light.final_acceptance,
                },
                write_ticket: ProjectWriteAuthorityTicketBasis {
                    idle_timeout_minutes: policy.workflow.write_ticket.idle_timeout_minutes,
                },
            }
        }
        None => ProjectWriteAuthorityFingerprintBasis {
            schema: WRITE_AUTHORITY_CONTRACT_ID,
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

impl CoreProjectStore<'_> {
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
        self.apply_project_workflow_policy_authority_inner(
            ProjectWorkflowPolicyAuthorityApply {
                policy_version: input.policy_version,
                policy: input.policy,
                policy_fingerprint: input.policy_fingerprint,
                source: input.source,
                expected_prior_fingerprint: prior
                    .as_ref()
                    .map(|record| record.policy_fingerprint.clone()),
            },
            Some(std::cmp::max(input.applied_at, input.created_at)),
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
        clock_floor: Option<UtcTimestamp>,
    ) -> StoreResult<ProjectWorkflowPolicyApplyResult> {
        require_writable(self)?;
        validate_project_workflow_policy_fields(
            input.policy_version,
            &input.policy,
            &input.policy_fingerprint,
            input.source,
        )?;
        let observation = self.workflow_policy_apply_observation()?;
        if let Some(prior) = observation.prior.as_ref() {
            if prior.policy_fingerprint == input.policy_fingerprint {
                let write_authority_fingerprint = prior.write_authority_fingerprint.clone();
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
            observation.prior.as_ref().map(|record| &record.policy),
        )?;
        let resulting_write_authority_fingerprint =
            project_write_authority_fingerprint(Some(&input.policy))?;
        let write_authority_changed =
            prior_write_authority_fingerprint != resulting_write_authority_fingerprint;
        let payload = canonical_json_string(&json!({
            "policy_schema": WORKFLOW_POLICY_CONTRACT_ID,
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
            policy: input.policy,
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
                actor_source: ActorSource::LocalUser,
                operation_category: OperationCategory::AdminLocal,
                verification_basis: Some("local_admin_policy_apply".to_owned()),
                git_workspace_context: None,
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
        let mutations = [CoreStorageMutation::WorkflowPolicy(
            WorkflowPolicyMutation::Apply(mutation),
        )];
        let (outcome, mutation_results) =
            self.commit_mutation_with_results(commit_input, &mutations, |_| Ok("{}".to_owned()))?;
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
        let mutation_effect = mutation_results
            .into_iter()
            .find_map(|result| match result {
                crate::core_pipeline::mutations::AggregateMutationResult::WorkflowPolicy(
                    effect,
                ) => Some(effect),
                crate::core_pipeline::mutations::AggregateMutationResult::Applied => None,
            })
            .ok_or_else(|| {
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
}

pub(crate) fn project_workflow_policy_from_conn(
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
                Ok(ProjectWorkflowPolicyRow {
                    project_id: row.get(0)?,
                    policy_schema: row.get(1)?,
                    policy_version: row.get(2)?,
                    policy_json: row.get(3)?,
                    policy_fingerprint: row.get(4)?,
                    source: row.get(5)?,
                    applied_at: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        )
        .optional()?;
    raw.map(|raw| {
        let corrupt_value = |column| {
            StoreError::corrupt_owner_state_value(
                "project_workflow_policies",
                raw.project_id.clone(),
                column,
            )
        };
        let corrupt_json = || {
            StoreError::corrupt_owner_state_json(
                "project_workflow_policies",
                raw.project_id.clone(),
                "policy_json",
            )
        };
        let policy_version =
            u64::try_from(raw.policy_version).map_err(|_| corrupt_value("policy_version"))?;
        if raw.policy_schema != WORKFLOW_POLICY_CONTRACT_ID || policy_version == 0 {
            return Err(corrupt_value("policy_schema"));
        }
        let policy: ProjectWorkflowPolicy =
            serde_json::from_str(&raw.policy_json).map_err(|_| corrupt_json())?;
        policy.validate().map_err(|_| corrupt_json())?;
        let canonical = canonical_json_string(&policy).map_err(|_| corrupt_json())?;
        if canonical != raw.policy_json {
            return Err(corrupt_json());
        }
        let fingerprint = canonical_json_sha256(&policy)
            .map_err(|_| corrupt_json())?
            .into_inner();
        if fingerprint != raw.policy_fingerprint {
            return Err(corrupt_value("policy_fingerprint"));
        }
        let source = serde_json::from_value(Value::String(raw.source))
            .map_err(|_| corrupt_value("source"))?;
        let applied_at =
            UtcTimestamp::parse(&raw.applied_at).map_err(|_| corrupt_value("applied_at"))?;
        let created_at =
            UtcTimestamp::parse(&raw.created_at).map_err(|_| corrupt_value("created_at"))?;
        let write_authority_fingerprint =
            project_write_authority_fingerprint(Some(&policy)).map_err(|_| corrupt_json())?;
        let record = ProjectWorkflowPolicyRecord {
            project_id: raw.project_id,
            policy_version,
            policy,
            policy_fingerprint: raw.policy_fingerprint,
            source,
            applied_at,
            created_at,
            write_authority_fingerprint,
        };
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
        &input.policy,
        &input.policy_fingerprint,
        input.source,
    )?;
    let existing = project_workflow_policy_from_conn(tx, project_id)?;
    validate_policy_replacement_basis(
        project_id,
        existing.as_ref(),
        input.policy_version,
        input.expected_prior_fingerprint.as_deref(),
    )?;
    let prior_write_authority_fingerprint =
        project_write_authority_fingerprint(existing.as_ref().map(|record| &record.policy))?;
    let resulting_write_authority_fingerprint =
        project_write_authority_fingerprint(Some(&input.policy))?;
    let write_authority_changed =
        prior_write_authority_fingerprint != resulting_write_authority_fingerprint;
    let policy_version =
        i64::try_from(input.policy_version).map_err(|_| StoreError::InvalidInput {
            detail: "policy_version is outside the supported SQLite integer range".to_owned(),
        })?;
    let created_at = existing
        .as_ref()
        .map(|record| record.created_at.to_string())
        .unwrap_or_else(|| committed_at.to_owned());
    let policy_json =
        canonical_json_string(&input.policy).map_err(|_| StoreError::InvalidInput {
            detail: "workflow policy cannot be serialized canonically".to_owned(),
        })?;
    tx.execute(
        "INSERT INTO project_workflow_policies (
            project_id, policy_schema, policy_version, policy_json,
            policy_fingerprint, source, applied_at, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
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
            WORKFLOW_POLICY_CONTRACT_ID,
            policy_version,
            policy_json,
            input.policy_fingerprint,
            input.source.as_str(),
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
    task_policy_control_reevaluation_from_object(&task.metadata, &task.task_id)
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
    let required = mark.required_effective_control_level;
    let acceptance = parse_acceptance_policy(acceptance_policy, task_id)?;
    let acceptance_satisfied = if let Some(required) = mark.required_acceptance_policy {
        acceptance_policy_rank(acceptance) >= acceptance_policy_rank(required)
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
    let required = mark.required_effective_control_level;
    let current_acceptance = parse_acceptance_policy(&acceptance_policy, &task_id)?;
    let acceptance_escalation = if let Some(required) = mark.required_acceptance_policy {
        acceptance_policy_rank(required) > acceptance_policy_rank(current_acceptance)
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
    let required_level = required_control_for_policy(&input.policy, &mode, &requested)?;
    let required_acceptance = required_acceptance_for_policy(&input.policy, required_level);
    let current_acceptance_rank =
        acceptance_policy_rank(parse_acceptance_policy(&current_acceptance, &task_id)?);
    let existing_mark = task_policy_control_reevaluation_from_metadata(&metadata_json, &task_id)?;
    let existing_required = existing_mark
        .as_ref()
        .map(|mark| mark.required_effective_control_level);
    let existing_required_acceptance = existing_mark
        .as_ref()
        .and_then(|mark| mark.required_acceptance_policy)
        .map(acceptance_policy_rank);

    let combined_required_level =
        std::cmp::max(required_level, existing_required.unwrap_or(current_level));
    let combined_control_acceptance =
        required_acceptance_for_policy(&input.policy, combined_required_level);
    let combined_required_acceptance = std::cmp::max(
        std::cmp::max(
            acceptance_policy_rank(required_acceptance),
            acceptance_policy_rank(combined_control_acceptance),
        ),
        existing_required_acceptance.unwrap_or(current_acceptance_rank),
    );
    let next_mark = Some(TaskPolicyControlReevaluation {
        policy_version: input.policy_version,
        policy_fingerprint: input.policy_fingerprint.clone(),
        required_effective_control_level: combined_required_level,
        required_acceptance_policy: Some(acceptance_policy_from_rank(combined_required_acceptance)),
        marked_at: UtcTimestamp::parse(committed_at).map_err(|_| {
            StoreError::schema_invariant(
                "project_state",
                "commit timestamp is not a valid RFC 3339 value",
            )
        })?,
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
    let active_tickets = active_write_ticket_authority_bindings_in_tx(tx, project_id)?;
    let mut affected_task_ids = BTreeSet::new();
    if let Some(task_id) = reevaluated_task_id {
        affected_task_ids.insert(task_id.to_owned());
    }
    let mut invalidation_candidates = Vec::new();
    for binding in active_tickets {
        let policy_binding_mismatch =
            binding.write_authority_fingerprint != resulting_write_authority_fingerprint;
        let task_reevaluation_pending = reevaluated_task_id == Some(binding.task_id.as_str());
        if !policy_binding_mismatch && !task_reevaluation_pending {
            continue;
        }
        affected_task_ids.insert(binding.task_id.into_inner());
        invalidation_candidates.push(binding.write_ticket_id);
    }
    let invalidated_write_ticket_ids = invalidate_active_write_ticket_ids_in_tx(
        tx,
        project_id,
        &invalidation_candidates,
        volicord_types::values::WriteTicketInvalidationReason::ExplicitRevoke,
    )?;
    Ok((
        affected_task_ids.into_iter().collect(),
        invalidated_write_ticket_ids,
    ))
}

fn required_control_for_policy(
    policy: &ProjectWorkflowPolicy,
    mode: &str,
    requested: &str,
) -> StoreResult<TaskControlLevel> {
    let direct_default = policy.workflow.default_direct_control;
    let work_default = policy.workflow.default_work_control;
    let light_enabled = policy.workflow.light.enabled;
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
    policy: &ProjectWorkflowPolicy,
    required_control: TaskControlLevel,
) -> AcceptancePolicy {
    match required_control {
        TaskControlLevel::Observe => AcceptancePolicy::NotRequired,
        TaskControlLevel::Tracked | TaskControlLevel::Sensitive => AcceptancePolicy::Required,
        TaskControlLevel::Light => policy.workflow.light.final_acceptance,
    }
}

fn acceptance_policy_rank(value: AcceptancePolicy) -> u8 {
    match value {
        AcceptancePolicy::NotRequired => 0,
        AcceptancePolicy::PolicyDependent => 1,
        AcceptancePolicy::Required => 2,
    }
}

fn acceptance_policy_from_rank(rank: u8) -> AcceptancePolicy {
    match rank {
        0 => AcceptancePolicy::NotRequired,
        1 => AcceptancePolicy::PolicyDependent,
        _ => AcceptancePolicy::Required,
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
    validate_timestamp("marked_at", &mark.marked_at)
        .map_err(|_| StoreError::corrupt_owner_state_json("tasks", task_id, "metadata_json"))?;
    Ok(Some(mark))
}

fn task_policy_control_reevaluation_from_object(
    metadata: &serde_json::Map<String, Value>,
    task_id: &str,
) -> StoreResult<Option<TaskPolicyControlReevaluation>> {
    let Some(value) = metadata.get(POLICY_CONTROL_REEVALUATION_METADATA_KEY) else {
        return Ok(None);
    };
    decode_task_policy_control_reevaluation(value, task_id)
}

fn decode_task_policy_control_reevaluation(
    value: &Value,
    task_id: &str,
) -> StoreResult<Option<TaskPolicyControlReevaluation>> {
    let mark: TaskPolicyControlReevaluation = serde_json::from_value(value.clone())
        .map_err(|_| StoreError::corrupt_owner_state_json("tasks", task_id, "metadata_json"))?;
    if mark.policy_version == 0 || !canonical_sha256(&mark.policy_fingerprint) {
        return Err(StoreError::corrupt_owner_state_json(
            "tasks",
            task_id,
            "metadata_json",
        ));
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

fn parse_acceptance_policy(value: &str, task_id: &str) -> StoreResult<AcceptancePolicy> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| StoreError::corrupt_owner_state_value("tasks", task_id, "acceptance_policy"))
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

fn validate_project_workflow_policy(input: &ProjectWorkflowPolicyUpsert) -> StoreResult<()> {
    validate_project_workflow_policy_fields(
        input.policy_version,
        &input.policy,
        &input.policy_fingerprint,
        input.source,
    )?;
    validate_timestamp("applied_at", &input.applied_at)?;
    validate_timestamp("created_at", &input.created_at)
}

fn validate_project_workflow_policy_fields(
    policy_version: u64,
    policy: &ProjectWorkflowPolicy,
    policy_fingerprint: &str,
    _source: ProjectWorkflowPolicySource,
) -> StoreResult<()> {
    if policy_version == 0 {
        return invalid("policy_version must be greater than zero");
    }
    policy
        .validate()
        .map_err(|error| StoreError::InvalidInput {
            detail: error.to_string(),
        })?;
    let fingerprint = canonical_json_sha256(policy)
        .map_err(|_| StoreError::InvalidInput {
            detail: "workflow policy fingerprint could not be computed".to_owned(),
        })?
        .as_str()
        .to_owned();
    if fingerprint != policy_fingerprint {
        return invalid("policy_fingerprint must match the typed workflow policy");
    }
    Ok(())
}

fn validate_timestamp(field: &str, value: &UtcTimestamp) -> StoreResult<()> {
    value
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::InvalidInput {
            detail: format!("{field} must be a canonical RFC 3339 UTC timestamp"),
        })
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
    use volicord_types::canonical::canonical_json_sha256;
    use volicord_types::ids::{ChangeUnitId, ProjectId, TaskId};
    use volicord_types::product_path::ProductRelativePath;
    use volicord_types::schema::{WriteTicketAttemptScope, WriteTicketValidityBasis};
    use volicord_types::values::{WriteTicketInvalidationReason, WriteTicketStatus};

    use super::*;
    use crate::{
        core_pipeline::{CoreStorageMutation, TaskMutation, WriteTicketInsert},
        mutation::TestRuntimeHomeAdmission,
    };

    fn workflow_policy(
        default_direct_control: &str,
        light_enabled: bool,
    ) -> Result<(ProjectWorkflowPolicy, String), Box<dyn Error>> {
        workflow_policy_with_acceptance(default_direct_control, light_enabled, "policy_dependent")
    }

    fn workflow_policy_with_acceptance(
        default_direct_control: &str,
        light_enabled: bool,
        final_acceptance: &str,
    ) -> Result<(ProjectWorkflowPolicy, String), Box<dyn Error>> {
        let value = json!({
            "schema": WORKFLOW_POLICY_CONTRACT_ID,
            "managed_by": "volicord",
            "storage_scope": "local_overlay",
            "connection_intent": "shared",
            "host": "codex",
            "repo_root": "/tmp/volicord-workflow-policy-test",
            "connection_id": "connection_workflow_policy_test",
            "guard_installation_id": "guard_workflow_policy_test",
            "selected_profile": "record",
            "mcp": {
                "command": "volicord-mcp",
                "args": [],
                "env": {}
            },
            "host_hook": {
                "enabled": true,
                "commands": {
                    "pre_tool": {"command": "volicord", "args": ["guard", "pre-tool"]},
                    "post_tool": {"command": "volicord", "args": ["guard", "post-tool"]},
                    "prompt_capture": {
                        "command": "volicord",
                        "args": ["guard", "prompt-capture"]
                    }
                }
            },
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
                "write_ticket": {"idle_timeout_minutes": null}
            }
        });
        let policy = serde_json::from_value::<ProjectWorkflowPolicy>(value.clone())?;
        policy.validate()?;
        Ok((policy, canonical_json_sha256(&value)?.into_inner()))
    }

    fn workflow_policy_with_write_authority(
        max_intended_paths: u64,
        allowed_path_patterns: Vec<&str>,
        denied_path_patterns: Vec<&str>,
        idle_timeout_minutes: Option<u64>,
    ) -> Result<(ProjectWorkflowPolicy, String), Box<dyn Error>> {
        let value = json!({
            "schema": WORKFLOW_POLICY_CONTRACT_ID,
            "managed_by": "volicord",
            "storage_scope": "local_overlay",
            "connection_intent": "shared",
            "host": "codex",
            "repo_root": "/tmp/volicord-workflow-policy-test",
            "connection_id": "connection_workflow_policy_test",
            "guard_installation_id": "guard_workflow_policy_test",
            "selected_profile": "record",
            "mcp": {
                "command": "volicord-mcp",
                "args": [],
                "env": {}
            },
            "host_hook": {
                "enabled": true,
                "commands": {
                    "pre_tool": {"command": "volicord", "args": ["guard", "pre-tool"]},
                    "post_tool": {"command": "volicord", "args": ["guard", "post-tool"]},
                    "prompt_capture": {
                        "command": "volicord",
                        "args": ["guard", "prompt-capture"]
                    }
                }
            },
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
                "write_ticket": {"idle_timeout_minutes": idle_timeout_minutes}
            }
        });
        let policy = serde_json::from_value::<ProjectWorkflowPolicy>(value.clone())?;
        policy.validate()?;
        Ok((policy, canonical_json_sha256(&value)?.into_inner()))
    }

    fn insert_current_change_unit(
        store: &CoreProjectStore,
        task_id: &str,
        change_unit_id: &str,
        basis_state_version: u64,
        timestamp: &str,
    ) -> Result<(), Box<dyn Error>> {
        store.conn.execute(
            "INSERT INTO change_units (
                project_id, change_unit_id, task_id, status, is_current,
                basis_state_version, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 'active', 1, ?4, ?5, ?5)",
            params![
                store.project.project_id,
                change_unit_id,
                task_id,
                i64::try_from(basis_state_version)?,
                timestamp
            ],
        )?;
        store.conn.execute(
            "UPDATE tasks
                SET current_change_unit_id = ?3
              WHERE project_id = ?1
                AND task_id = ?2",
            params![store.project.project_id, task_id, change_unit_id],
        )?;
        Ok(())
    }

    struct ActiveWriteTicketFixture<'a> {
        write_ticket_id: &'a str,
        task_id: &'a str,
        change_unit_id: &'a str,
        basis_state_version: u64,
        write_authority_fingerprint: String,
        intended_paths: &'a [&'a str],
        created_at: &'a str,
    }

    fn insert_active_write_ticket(
        store: &CoreProjectStore,
        input: ActiveWriteTicketFixture<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let intended_paths = input
            .intended_paths
            .iter()
            .map(|path| ProductRelativePath::parse(*path))
            .collect::<Result<Vec<_>, _>>()?;
        store.insert_write_ticket_fixture(
            &WriteTicketInsert {
                write_ticket_id: input.write_ticket_id.to_owned(),
                task_id: input.task_id.to_owned(),
                change_unit_id: input.change_unit_id.to_owned(),
                validity_basis: WriteTicketValidityBasis {
                    task_id: TaskId::new(input.task_id),
                    change_unit_id: ChangeUnitId::new(input.change_unit_id),
                    scope_revision: 1,
                    baseline_ref: None,
                    workspace_context_sha256: None,
                    write_authority_fingerprint: input.write_authority_fingerprint,
                    approval_basis_refs: Vec::new(),
                },
                allowed_path_prefixes: intended_paths.clone(),
                denied_path_prefixes: Vec::new(),
                attempt_scope: WriteTicketAttemptScope {
                    task_id: TaskId::new(input.task_id),
                    change_unit_id: ChangeUnitId::new(input.change_unit_id),
                    intended_operation: "workflow_policy_test".to_owned(),
                    product_file_write_intended: !intended_paths.is_empty(),
                    intended_paths,
                    sensitive_categories: Vec::new(),
                    baseline_ref: None,
                },
                created_by_actor_source: ActorSource::System,
                created_by_user_action_resolution_id: None,
                idle_expires_at: None,
                created_at: UtcTimestamp::parse(input.created_at)?,
                metadata: serde_json::Map::new(),
            },
            input.basis_state_version,
        )?;
        Ok(())
    }

    #[test]
    fn workflow_policy_round_trip() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("workflow-record-round-trip")?;
        let mutation = TestRuntimeHomeAdmission::shared(fixture.runtime_home_path())?;
        let context = mutation.context()?;
        let mut store =
            CoreProjectStore::open_for_mutation(&context, &ProjectId::new(fixture.project_id()))?;

        let (policy_value, policy_fingerprint) = workflow_policy("tracked", false)?;
        let policy = store.upsert_project_workflow_policy(ProjectWorkflowPolicyUpsert {
            policy_version: 1,
            policy: policy_value.clone(),
            policy_fingerprint: policy_fingerprint.clone(),
            source: ProjectWorkflowPolicySource::VolicordInit,
            applied_at: UtcTimestamp::parse("2026-07-16T00:00:00Z")?,
            created_at: UtcTimestamp::parse("2026-07-16T00:00:00Z")?,
        })?;
        assert_eq!(policy.policy, policy_value);
        assert_eq!(policy.policy_fingerprint, policy_fingerprint);
        assert_eq!(store.project_workflow_policy()?, Some(policy));
        Ok(())
    }

    #[test]
    fn workflow_policy_apply_is_atomic_versioned_and_preserves_stronger_task_mark(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("workflow-policy-atomic-authority")?;
        let mutation = TestRuntimeHomeAdmission::shared(fixture.runtime_home_path())?;
        let context = mutation.context()?;
        let mut store =
            CoreProjectStore::open_for_mutation(&context, &ProjectId::new(fixture.project_id()))?;
        store.conn.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode,
                requested_control_level, effective_control_level, control_level_reason,
                work_phase, acceptance_policy, acceptance_policy_reason,
                lifecycle_phase, created_at, updated_at
             ) VALUES (?1, 'task_policy_active', ?2, 'direct', 'auto', 'observe',
                       'Initial observe control.', 'implementation', 'not_required',
                       'Observe control needs no acceptance.', 'executing',
                       '2026-07-16T00:00:00Z', '2026-07-16T00:00:00Z')",
            params![fixture.project_id(), fixture.actor_source()],
        )?;
        store.conn.execute(
            "UPDATE project_state SET active_task_id = 'task_policy_active' WHERE project_id = ?1",
            [fixture.project_id()],
        )?;
        insert_current_change_unit(
            &store,
            "task_policy_active",
            "cu_policy_active",
            0,
            "2026-07-16T00:00:00Z",
        )?;

        let (observe_json, observe_fingerprint) = workflow_policy("observe", false)?;
        let initial =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 1,
                policy: observe_json.clone(),
                policy_fingerprint: observe_fingerprint.clone(),
                source: ProjectWorkflowPolicySource::ProjectDatabase,
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
        assert_eq!(
            event_created_at,
            initial.policy.applied_at.to_canonical_string()
        );
        assert_eq!(store.project_state()?.updated_at, initial.policy.applied_at);
        assert_eq!(store.effect_counts()?.authority_events, 1);

        let replay =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 1,
                policy: observe_json,
                policy_fingerprint: observe_fingerprint.clone(),
                source: ProjectWorkflowPolicySource::ProjectDatabase,
                expected_prior_fingerprint: Some(observe_fingerprint.clone()),
            })?;
        assert!(!replay.database_changed);
        assert_eq!(replay.resulting_state_version, 1);
        assert!(!replay.write_authority_changed);
        assert!(replay.invalidated_write_ticket_ids.is_empty());
        assert_eq!(store.effect_counts()?.authority_events, 1);
        let authority_event_count: i64 =
            store
                .conn
                .query_row("SELECT COUNT(*) FROM authority_events", [], |row| {
                    row.get(0)
                })?;
        assert_eq!(authority_event_count, 1);

        insert_active_write_ticket(
            &store,
            ActiveWriteTicketFixture {
                write_ticket_id: "ticket_policy_before_raise",
                task_id: "task_policy_active",
                change_unit_id: "cu_policy_active",
                basis_state_version: 1,
                write_authority_fingerprint: observe_fingerprint.clone(),
                intended_paths: &[],
                created_at: "2026-07-16T00:00:00Z",
            },
        )?;

        let (tracked_json, tracked_fingerprint) = workflow_policy("tracked", false)?;
        let strengthened =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 2,
                policy: tracked_json,
                policy_fingerprint: tracked_fingerprint.clone(),
                source: ProjectWorkflowPolicySource::ProjectDatabase,
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
        assert_eq!(
            marked.required_effective_control_level,
            TaskControlLevel::Tracked
        );
        assert_eq!(marked.marked_at, strengthened.policy.applied_at);
        let invalidated_ticket = store
            .write_ticket_record("ticket_policy_before_raise")?
            .expect("invalidated Write Ticket remains readable");
        assert_eq!(invalidated_ticket.status, WriteTicketStatus::Invalidated);
        assert_eq!(
            invalidated_ticket.invalidation_reason,
            Some(WriteTicketInvalidationReason::ExplicitRevoke)
        );

        let (relaxed_json, relaxed_fingerprint) = workflow_policy("light", true)?;
        let relaxed =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 3,
                policy: relaxed_json,
                policy_fingerprint: relaxed_fingerprint,
                source: ProjectWorkflowPolicySource::ProjectDatabase,
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
        assert_eq!(
            preserved.required_effective_control_level,
            TaskControlLevel::Tracked
        );

        let marker_commit = CommitMutationInput {
            project_id: fixture.project_id().to_owned(),
            tool_name: "store_test_policy_marker_raise".to_owned(),
            idempotency_key: None,
            request_hash: "request_policy_marker_raise".to_owned(),
            replay_context: Some(VerifiedReplayContext {
                actor_source: ActorSource::LocalUser,
                operation_category: OperationCategory::AdminLocal,
                verification_basis: Some("store_test".to_owned()),
                git_workspace_context: None,
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
            effective_control_level: TaskControlLevel::Tracked,
            control_level_reason: "Raised for pending project policy reevaluation.".to_owned(),
            acceptance_policy: Some(AcceptancePolicy::Required),
            acceptance_policy_reason: Some("Tracked control requires acceptance.".to_owned()),
        };
        let mutations = [CoreStorageMutation::Task(TaskMutation::UpdateControlLevel(
            marker_raise,
        ))];
        let outcome = store.commit_mutation(marker_commit, &mutations, |_| Ok("{}".to_owned()))?;
        assert!(matches!(outcome, MutationCommitOutcome::Committed { .. }));
        let raised = store.active_task_record()?.expect("active Task");
        assert_eq!(raised.effective_control_level, TaskControlLevel::Tracked);
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
            let mutation = TestRuntimeHomeAdmission::shared(fixture.runtime_home_path())?;
            let context = mutation.context()?;
            let mut store = CoreProjectStore::open_for_mutation(
                &context,
                &ProjectId::new(fixture.project_id()),
            )?;
            let (initial_json, initial_fingerprint) = workflow_policy_with_write_authority(
                case.initial_max_paths,
                case.initial_allowed,
                case.initial_denied,
                case.initial_timeout,
            )?;
            let initial = store.apply_project_workflow_policy_authority(
                ProjectWorkflowPolicyAuthorityApply {
                    policy_version: 1,
                    policy: initial_json,
                    policy_fingerprint: initial_fingerprint.clone(),
                    source: ProjectWorkflowPolicySource::ProjectDatabase,
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
                           'executing', '2026-07-17T00:00:00Z',
                           '2026-07-17T00:00:00Z')",
                params![fixture.project_id(), fixture.actor_source()],
            )?;
            store.conn.execute(
                "UPDATE project_state
                    SET active_task_id = 'task_policy_binding'
                  WHERE project_id = ?1",
                [fixture.project_id()],
            )?;
            insert_current_change_unit(
                &store,
                "task_policy_binding",
                "cu_policy_binding",
                1,
                "2026-07-17T00:00:00Z",
            )?;
            insert_active_write_ticket(
                &store,
                ActiveWriteTicketFixture {
                    write_ticket_id: "ticket_policy_binding",
                    task_id: "task_policy_binding",
                    change_unit_id: "cu_policy_binding",
                    basis_state_version: 1,
                    write_authority_fingerprint: initial
                        .resulting_write_authority_fingerprint
                        .clone(),
                    intended_paths: &case.ticket_paths,
                    created_at: "2026-07-17T00:00:00Z",
                },
            )?;

            let (tightened_json, tightened_fingerprint) = workflow_policy_with_write_authority(
                case.tightened_max_paths,
                case.tightened_allowed,
                case.tightened_denied,
                case.tightened_timeout,
            )?;
            let tightened = store.apply_project_workflow_policy_authority(
                ProjectWorkflowPolicyAuthorityApply {
                    policy_version: 2,
                    policy: tightened_json,
                    policy_fingerprint: tightened_fingerprint,
                    source: ProjectWorkflowPolicySource::ProjectDatabase,
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
            let ticket = store
                .write_ticket_record("ticket_policy_binding")?
                .expect("invalidated Write Ticket remains readable");
            assert_eq!(
                ticket.status,
                WriteTicketStatus::Invalidated,
                "{}",
                case.name
            );
            assert_eq!(
                ticket.invalidation_reason,
                Some(WriteTicketInvalidationReason::ExplicitRevoke),
                "{}",
                case.name
            );
            let active_task = store.active_task_record()?.expect("active Task");
            let mark = task_policy_control_reevaluation(&active_task)?
                .expect("write-authority changes must mark the active Task");
            assert_eq!(
                mark.required_effective_control_level,
                TaskControlLevel::Light,
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
        let mutation = TestRuntimeHomeAdmission::shared(fixture.runtime_home_path())?;
        let context = mutation.context()?;
        let mut store =
            CoreProjectStore::open_for_mutation(&context, &ProjectId::new(fixture.project_id()))?;
        let (initial_json, initial_fingerprint) = workflow_policy_with_write_authority(
            3,
            vec!["src/**", "tests/**"],
            vec!["target/**", "vendor/**"],
            Some(30),
        )?;
        let initial =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 1,
                policy: initial_json,
                policy_fingerprint: initial_fingerprint.clone(),
                source: ProjectWorkflowPolicySource::ProjectDatabase,
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
                       'executing', '2026-07-17T00:00:00Z',
                       '2026-07-17T00:00:00Z')",
            params![fixture.project_id(), fixture.actor_source()],
        )?;
        store.conn.execute(
            "UPDATE project_state
                SET active_task_id = 'task_policy_equivalent'
              WHERE project_id = ?1",
            [fixture.project_id()],
        )?;
        insert_current_change_unit(
            &store,
            "task_policy_equivalent",
            "cu_policy_equivalent",
            1,
            "2026-07-17T00:00:00Z",
        )?;
        insert_active_write_ticket(
            &store,
            ActiveWriteTicketFixture {
                write_ticket_id: "ticket_policy_equivalent",
                task_id: "task_policy_equivalent",
                change_unit_id: "cu_policy_equivalent",
                basis_state_version: 1,
                write_authority_fingerprint: initial.resulting_write_authority_fingerprint,
                intended_paths: &["src/export.rs"],
                created_at: "2026-07-17T00:00:00Z",
            },
        )?;

        let (equivalent_json, equivalent_fingerprint) = workflow_policy_with_write_authority(
            3,
            vec!["tests/**", "src/**", "src/**"],
            vec!["vendor/**", "target/**", "target/**"],
            Some(30),
        )?;
        assert_ne!(initial_fingerprint, equivalent_fingerprint);
        let equivalent =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 2,
                policy: equivalent_json.clone(),
                policy_fingerprint: equivalent_fingerprint.clone(),
                source: ProjectWorkflowPolicySource::ProjectDatabase,
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
        let ticket = store
            .write_ticket_record("ticket_policy_equivalent")?
            .expect("compatible Write Ticket remains readable");
        assert_eq!(ticket.status, WriteTicketStatus::Active);

        let replay =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 2,
                policy: equivalent_json,
                policy_fingerprint: equivalent_fingerprint.clone(),
                source: ProjectWorkflowPolicySource::ProjectDatabase,
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
        let mutation = TestRuntimeHomeAdmission::shared(fixture.runtime_home_path())?;
        let context = mutation.context()?;
        let mut store = CoreProjectStore::open_for_mutation(&context, &project_id)?;
        let (policy_json, policy_fingerprint) = workflow_policy("tracked", false)?;
        let initial =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 1,
                policy: policy_json.clone(),
                policy_fingerprint: policy_fingerprint.clone(),
                source: ProjectWorkflowPolicySource::ProjectDatabase,
                expected_prior_fingerprint: None,
            })?;
        assert_eq!(initial.resulting_state_version, 1);

        let concurrent_mutation = TestRuntimeHomeAdmission::shared(fixture.runtime_home_path())?;
        let concurrent_context = concurrent_mutation.context()?;
        let observation = store.workflow_policy_apply_observation_with_hook(|| {
            let mut concurrent = CoreProjectStore::open_for_mutation(
                &concurrent_context,
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
                policy: policy_json,
                policy_fingerprint: policy_fingerprint.clone(),
                source: ProjectWorkflowPolicySource::ProjectDatabase,
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
        let mutation = TestRuntimeHomeAdmission::shared(fixture.runtime_home_path())?;
        let context = mutation.context()?;
        let mut store =
            CoreProjectStore::open_for_mutation(&context, &ProjectId::new(fixture.project_id()))?;
        store.conn.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode,
                requested_control_level, effective_control_level, control_level_reason,
                work_phase, acceptance_policy, acceptance_policy_reason,
                lifecycle_phase, created_at, updated_at
             ) VALUES (?1, 'task_policy_acceptance', ?2, 'direct', 'light', 'light',
                       'Initial Light control.', 'implementation', 'policy_dependent',
                       'Initial policy-dependent acceptance.', 'executing',
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
                policy: initial_json,
                policy_fingerprint: initial_fingerprint.clone(),
                source: ProjectWorkflowPolicySource::ProjectDatabase,
                expected_prior_fingerprint: None,
            })?;
        assert!(!initial.active_task_requires_escalation);

        let (required_json, required_fingerprint) =
            workflow_policy_with_acceptance("light", true, "required")?;
        let strengthened =
            store.apply_project_workflow_policy_authority(ProjectWorkflowPolicyAuthorityApply {
                policy_version: 2,
                policy: required_json,
                policy_fingerprint: required_fingerprint.clone(),
                source: ProjectWorkflowPolicySource::ProjectDatabase,
                expected_prior_fingerprint: Some(initial_fingerprint),
            })?;

        assert!(strengthened.active_task_requires_escalation);
        let mark =
            task_policy_control_reevaluation(&store.active_task_record()?.expect("active Task"))?
                .expect("acceptance-only strengthening must mark the active Task");
        assert_eq!(
            mark.required_effective_control_level,
            TaskControlLevel::Light
        );
        assert_eq!(
            mark.required_acceptance_policy,
            Some(AcceptancePolicy::Required)
        );
        assert_eq!(mark.policy_fingerprint, required_fingerprint);
        let task = store.active_task_record()?.expect("active Task");
        let task_metadata_json = serde_json::to_string(&Value::Object(task.metadata.clone()))?;
        let still_marked = clear_satisfied_task_policy_reevaluation(
            &task_metadata_json,
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
