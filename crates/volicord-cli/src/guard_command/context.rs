use std::{collections::BTreeSet, path::Path};

use volicord_core::GitWorkspaceContext;
use volicord_platform_fs::capture_git_workspace_snapshot;
use volicord_store::{
    bootstrap::ProjectRecord,
    core_pipeline::CoreProjectStore,
    guards::list_unresolved_unrecorded_changes,
    workflow_records::{
        project_write_authority_fingerprint, task_policy_control_reevaluation,
        TaskPolicyControlReevaluation,
    },
    RuntimeHomeMutationContext,
};
use volicord_types::canonical::canonical_json_bare_sha256;
use volicord_types::ids::{BaselineRef, ProjectId, TaskId};
use volicord_types::product_path::path_is_within;
use volicord_types::schema::{
    UserActionBasis, UserActionResolutionBody, WriteTicketAttemptScope, WriteTicketValidityBasis,
};
use volicord_types::values::{
    AcceptancePolicy, ActorSource, JudgmentResolutionOutcome, PromptCaptureStatus, StateRecordKind,
    TaskControlLevel, UnrecordedChangeConfidence, UserActionBasisStatus, UserActionKind,
    UserActionOptionAction, UserActionRequiredFor, UtcTimestamp, WriteTicketStatus,
};

use super::{
    args::GuardInput,
    core_current_timestamp,
    envelope::GuardEnvelope,
    json_error,
    prompt_capture::{
        pending_agent_user_action_summaries, prompt_capture_availability_for_event,
        GuardPendingUserActionSummary,
    },
    GuardCommandError,
};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct GuardStateSummary {
    pub(super) project_id: String,
    pub(super) project_name: String,
    pub(super) repo_root: String,
    pub(super) state_version: u64,
    pub(super) active_task_id: Option<String>,
    pub(super) active_task_effective_control_level: Option<String>,
    pub(super) policy_control_reevaluation: Option<GuardPolicyControlReevaluationSummary>,
    pub(super) active_change_unit_id: Option<String>,
    pub(super) prompt_capture_status: PromptCaptureStatus,
    pub(super) prompt_capture_operational: bool,
    pub(super) current_write_ticket_ids: Vec<String>,
    pub(super) stale_write_ticket_ids: Vec<String>,
    pub(super) uncertain_write_ticket_ids: Vec<String>,
    pub(super) active_write_tickets: Vec<ActiveWriteTicketSummary>,
    pub(super) policy_stale_write_tickets: Vec<PolicyStaleWriteTicketSummary>,
    pub(super) pending_user_action_count: usize,
    pub(super) pending_user_actions: Vec<GuardPendingUserActionSummary>,
    pub(super) active_blocker_count: usize,
    pub(super) unresolved_unrecorded_change_count: usize,
    pub(super) suspected_unrecorded_change_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuardPolicyControlReevaluationSummary {
    pub(super) required_effective_control_level: String,
    pub(super) required_acceptance_policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActiveWriteTicketSummary {
    pub(super) write_ticket_id: String,
    pub(super) change_unit_id: String,
    pub(super) intended_paths: Vec<String>,
    pub(super) denied_paths: Vec<String>,
    pub(super) idle_expires_at: Option<String>,
    pub(super) workspace_validity_uncertain: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PolicyStaleWriteTicketSummary {
    pub(super) write_ticket_id: String,
    pub(super) intended_paths: Vec<String>,
    pub(super) denied_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuardReason {
    pub(super) code: &'static str,
    pub(super) message: String,
    pub(super) severity: &'static str,
}

pub(super) fn guard_state_summary(
    context: &RuntimeHomeMutationContext<'_>,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    input: &GuardInput,
) -> Result<GuardStateSummary, GuardCommandError> {
    let runtime_home = context.runtime_home().as_path();
    let store = CoreProjectStore::open_for_mutation(context, &ProjectId::new(&project.project_id))?;
    let project_state = store.project_state()?;
    let workflow_policy = store.project_workflow_policy()?;
    let current_write_authority_fingerprint =
        project_write_authority_fingerprint(workflow_policy.as_ref().map(|policy| &policy.policy))?;
    let now_timestamp = core_current_timestamp(&store)?;
    let mut current_write_ticket_ids = Vec::new();
    let mut stale_write_ticket_ids = Vec::new();
    let mut uncertain_write_ticket_ids = Vec::new();
    let mut active_write_tickets = Vec::new();
    let mut policy_stale_write_tickets = Vec::new();
    let mut active_task_effective_control_level = None;
    let mut policy_control_reevaluation = None;
    let mut active_change_unit_id = None;
    let mut pending_user_action_count = 0;
    let mut pending_user_actions = Vec::new();
    let mut active_blocker_count = 0;
    let prompt_capture_availability =
        prompt_capture_availability_for_event(runtime_home, project, envelope)?;
    let prompt_capture_status = prompt_capture_availability.status;
    let prompt_capture_operational = prompt_capture_availability.is_operational();
    if let Some(active_task_id) = project_state.active_task_id.as_deref() {
        let task_id = TaskId::new(active_task_id);
        let mut current_task_sensitive = false;
        let current_task = store.task_record(&task_id)?;
        if let Some(task) = current_task.as_ref() {
            current_task_sensitive = task.effective_control_level == TaskControlLevel::Sensitive;
            active_task_effective_control_level =
                Some(task.effective_control_level.as_str().to_owned());
            policy_control_reevaluation = pending_policy_control_reevaluation(task)?;
            active_change_unit_id = task.current_change_unit_id.clone();
        }
        let current_change_unit = store.current_change_unit(&task_id)?;
        let current_ticket_basis = current_task
            .as_ref()
            .map(|task| {
                current_write_ticket_basis(task, current_change_unit.as_ref(), &project.repo_root)
            })
            .transpose()?;
        let current_sensitive_approvals =
            current_sensitive_approvals(&store, &task_id, &now_timestamp)?;
        for record in store.write_tickets_for_task(&task_id)? {
            let validity_basis = record.validity_basis();
            let policy_binding_is_current =
                validity_basis.write_authority_fingerprint == current_write_authority_fingerprint;
            if !policy_binding_is_current {
                stale_write_ticket_ids.push(record.write_ticket_id().to_owned());
                if record.status() != WriteTicketStatus::Consumed {
                    let intended_paths = record
                        .allowed_path_prefixes()
                        .iter()
                        .map(|path| path.as_str().to_owned())
                        .collect::<Vec<_>>();
                    let denied_paths = record
                        .denied_path_prefixes()
                        .iter()
                        .map(|path| path.as_str().to_owned())
                        .collect::<Vec<_>>();
                    if !intended_paths.is_empty() {
                        policy_stale_write_tickets.push(PolicyStaleWriteTicketSummary {
                            write_ticket_id: record.write_ticket_id().to_owned(),
                            intended_paths,
                            denied_paths,
                        });
                    }
                }
                continue;
            }
            if record.status() != WriteTicketStatus::Active {
                stale_write_ticket_ids.push(record.write_ticket_id().to_owned());
                continue;
            }
            let attempt_scope = record.attempt_scope();
            let not_idle_expired = record
                .idle_expires_at()
                .is_none_or(|expires_at| now_timestamp < *expires_at);
            let approval_basis_current = write_ticket_approval_basis_is_current(
                validity_basis,
                attempt_scope,
                current_task_sensitive,
                &current_sensitive_approvals,
            );
            let owner_basis_status = current_ticket_basis.as_ref().map_or(
                WriteTicketOwnerBasisStatus::Stale,
                |current| {
                    write_ticket_owner_basis_status(
                        record.task_id(),
                        Some(record.change_unit_id()),
                        validity_basis,
                        attempt_scope,
                        current,
                    )
                },
            );
            if policy_control_reevaluation.is_none()
                && not_idle_expired
                && approval_basis_current
                && owner_basis_status != WriteTicketOwnerBasisStatus::Stale
            {
                let write_ticket_id = record.write_ticket_id().to_owned();
                current_write_ticket_ids.push(write_ticket_id.clone());
                let workspace_validity_uncertain =
                    owner_basis_status == WriteTicketOwnerBasisStatus::WorkspaceUncertain;
                if workspace_validity_uncertain {
                    uncertain_write_ticket_ids.push(write_ticket_id.clone());
                }
                let intended_paths = record
                    .allowed_path_prefixes()
                    .iter()
                    .map(|path| path.as_str().to_owned())
                    .collect::<Vec<_>>();
                let denied_paths = record
                    .denied_path_prefixes()
                    .iter()
                    .map(|path| path.as_str().to_owned())
                    .collect::<Vec<_>>();
                if !intended_paths.is_empty() {
                    active_write_tickets.push(ActiveWriteTicketSummary {
                        write_ticket_id,
                        change_unit_id: record.change_unit_id().to_owned(),
                        intended_paths,
                        denied_paths,
                        idle_expires_at: record.idle_expires_at().map(ToString::to_string),
                        workspace_validity_uncertain,
                    });
                }
            } else {
                stale_write_ticket_ids.push(record.write_ticket_id().to_owned());
            }
        }
        pending_user_action_count = store
            .pending_user_action_records(&task_id, &now_timestamp)?
            .len();
        pending_user_actions =
            pending_agent_user_action_summaries(&store, &task_id, envelope, &now_timestamp)?;
        active_blocker_count = store
            .active_blocker_refs(&task_id, project_state.state_version)?
            .len();
    }
    let unresolved_unrecorded_changes = list_unresolved_unrecorded_changes(
        runtime_home,
        &project.project_id,
        Some(&envelope.connection_id),
    )?;
    let unresolved_unrecorded_change_count = unresolved_unrecorded_changes
        .iter()
        .filter(|record| record.confidence == UnrecordedChangeConfidence::Confirmed)
        .count();
    let suspected_unrecorded_change_count = unresolved_unrecorded_changes
        .iter()
        .filter(|record| record.confidence == UnrecordedChangeConfidence::Suspected)
        .count();
    let _ = input.raw_text.len();
    Ok(GuardStateSummary {
        project_id: project.project_id.clone(),
        project_name: project.project_name.clone(),
        repo_root: project.repo_root.display().to_string(),
        state_version: project_state.state_version,
        active_task_id: project_state.active_task_id,
        active_task_effective_control_level,
        policy_control_reevaluation,
        active_change_unit_id,
        prompt_capture_status,
        prompt_capture_operational,
        current_write_ticket_ids,
        stale_write_ticket_ids,
        uncertain_write_ticket_ids,
        active_write_tickets,
        policy_stale_write_tickets,
        pending_user_action_count,
        pending_user_actions,
        active_blocker_count,
        unresolved_unrecorded_change_count,
        suspected_unrecorded_change_count,
    })
}

fn pending_policy_control_reevaluation(
    task: &volicord_store::core_pipeline::TaskRecord,
) -> Result<Option<GuardPolicyControlReevaluationSummary>, GuardCommandError> {
    let Some(mark) = task_policy_control_reevaluation(task)? else {
        return Ok(None);
    };
    let current_control = task.effective_control_level;
    let required_control = mark.required_effective_control_level;
    let current_acceptance = task.acceptance_policy;
    let acceptance_escalation = mark.required_acceptance_policy.is_some_and(|required| {
        acceptance_policy_rank(required) > acceptance_policy_rank(current_acceptance)
    });
    if required_control <= current_control && !acceptance_escalation {
        return Ok(None);
    }
    Ok(Some(policy_reevaluation_summary(mark)))
}

fn policy_reevaluation_summary(
    mark: TaskPolicyControlReevaluation,
) -> GuardPolicyControlReevaluationSummary {
    GuardPolicyControlReevaluationSummary {
        required_effective_control_level: mark.required_effective_control_level.as_str().to_owned(),
        required_acceptance_policy: mark
            .required_acceptance_policy
            .map(|policy| match policy {
                AcceptancePolicy::NotRequired => "not_required",
                AcceptancePolicy::PolicyDependent => "policy_dependent",
                AcceptancePolicy::Required => "required",
            })
            .map(str::to_owned),
    }
}

const fn acceptance_policy_rank(policy: AcceptancePolicy) -> u8 {
    match policy {
        AcceptancePolicy::NotRequired => 0,
        AcceptancePolicy::PolicyDependent => 1,
        AcceptancePolicy::Required => 2,
    }
}

type StableApprovalIdentity = (String, String, String);

#[derive(Debug, Clone)]
struct CurrentSensitiveApproval {
    identity: StableApprovalIdentity,
    basis: UserActionBasis,
    required_for: Vec<UserActionRequiredFor>,
}

fn current_sensitive_approvals(
    store: &CoreProjectStore,
    task_id: &TaskId,
    now: &UtcTimestamp,
) -> Result<Vec<CurrentSensitiveApproval>, GuardCommandError> {
    let mut approvals = Vec::new();
    for record in
        store.resolved_user_action_records(task_id, UserActionKind::SensitiveApproval, now)?
    {
        let Some(resolution) = record.resolution() else {
            continue;
        };
        let request = record.request().request();
        let basis = record.request().basis();
        let resolution_body = resolution.resolution();
        let accepted = matches!(
            resolution_body,
            UserActionResolutionBody::Choice {
                machine_action: UserActionOptionAction::Accept,
                resolution_outcome: JudgmentResolutionOutcome::Accepted,
                ..
            }
        );
        let scope_current = basis.sensitive_action_scope().is_some_and(|scope| {
            scope
                .expires_at
                .as_ref()
                .is_none_or(|expires_at| now < expires_at)
        });
        if basis.compatibility_status() == UserActionBasisStatus::Current
            && accepted
            && resolution.resolved_by_actor_source() == &ActorSource::LocalUser
            && request
                .required_for
                .contains(&UserActionRequiredFor::PrepareWrite)
            && scope_current
        {
            approvals.push(CurrentSensitiveApproval {
                identity: (
                    resolution.project_id().to_owned(),
                    record.request().task_id().to_owned(),
                    resolution.user_action_resolution_id().to_owned(),
                ),
                basis: basis.clone(),
                required_for: request.required_for.clone(),
            });
        }
    }
    Ok(approvals)
}

fn write_ticket_approval_basis_is_current(
    validity_basis: &WriteTicketValidityBasis,
    attempt_scope: &WriteTicketAttemptScope,
    current_task_sensitive: bool,
    current_sensitive_approvals: &[CurrentSensitiveApproval],
) -> bool {
    if validity_basis.approval_basis_refs.is_empty() {
        return !current_task_sensitive && attempt_scope.sensitive_categories.is_empty();
    }

    validity_basis.approval_basis_refs.iter().all(|reference| {
        reference.record_kind == StateRecordKind::UserActionResolution
            && reference.task_id.as_ref() == Some(&validity_basis.task_id)
            && current_sensitive_approvals.iter().any(|approval| {
                approval.identity
                    == (
                        reference.project_id.as_str().to_owned(),
                        validity_basis.task_id.as_str().to_owned(),
                        reference.record_id.as_str().to_owned(),
                    )
                    && sensitive_approval_matches_ticket(approval, validity_basis, attempt_scope)
            })
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteTicketOwnerBasisStatus {
    Current,
    WorkspaceUncertain,
    Stale,
}

#[derive(Debug, Clone)]
struct CurrentWriteTicketBasis {
    task_id: String,
    change_unit_id: Option<String>,
    scope_revision: u64,
    task_baseline_ref: Option<BaselineRef>,
    change_unit_baseline_ref: Option<BaselineRef>,
    change_unit_workspace_sha256: Option<String>,
    workspace_probe: CurrentWorkspaceProbe,
}

#[derive(Debug, Clone)]
enum CurrentWorkspaceProbe {
    Available(Option<String>),
    Unavailable,
}

fn current_write_ticket_basis(
    task: &volicord_store::core_pipeline::TaskRecord,
    change_unit: Option<&volicord_store::core_pipeline::ChangeUnitRecord>,
    repo_root: &Path,
) -> Result<CurrentWriteTicketBasis, GuardCommandError> {
    let task_baseline_ref = task.shaping.baseline_ref.clone();
    let (change_unit_id, change_unit_baseline_ref, change_unit_workspace_sha256) = match change_unit
    {
        Some(change_unit) => {
            let baseline_ref = change_unit.write_basis.baseline_ref.clone();
            let workspace_context =
                change_unit
                    .write_basis
                    .git_workspace_context
                    .as_ref()
                    .map(|context| GitWorkspaceContext {
                        git_common_dir: context.git_common_dir.clone(),
                        worktree_id: context.worktree_id.clone(),
                        branch_ref: context.branch_ref.clone(),
                        head_sha: context.head_sha.clone(),
                        workspace_fingerprint: context.workspace_fingerprint.clone(),
                    });
            let workspace_sha256 = workspace_context
                .as_ref()
                .map(canonical_json_bare_sha256)
                .transpose()
                .map_err(json_error)?;
            (
                Some(change_unit.change_unit_id.clone()),
                baseline_ref,
                workspace_sha256,
            )
        }
        None => (None, None, None),
    };
    let workspace_probe = match capture_git_workspace_snapshot(repo_root) {
        Ok(Some(snapshot)) => {
            let context = GitWorkspaceContext {
                git_common_dir: snapshot.layout.common_dir.display().to_string(),
                worktree_id: snapshot.worktree_id,
                branch_ref: snapshot.branch_ref,
                head_sha: snapshot.head_sha,
                workspace_fingerprint: snapshot.workspace_fingerprint,
            };
            CurrentWorkspaceProbe::Available(Some(
                canonical_json_bare_sha256(&context).map_err(json_error)?,
            ))
        }
        Ok(None) => CurrentWorkspaceProbe::Available(None),
        Err(_) => CurrentWorkspaceProbe::Unavailable,
    };
    Ok(CurrentWriteTicketBasis {
        task_id: task.task_id.clone(),
        change_unit_id,
        scope_revision: task.scope_revision,
        task_baseline_ref,
        change_unit_baseline_ref,
        change_unit_workspace_sha256,
        workspace_probe,
    })
}

fn write_ticket_owner_basis_status(
    record_task_id: &str,
    record_change_unit_id: Option<&str>,
    validity_basis: &WriteTicketValidityBasis,
    attempt_scope: &WriteTicketAttemptScope,
    current: &CurrentWriteTicketBasis,
) -> WriteTicketOwnerBasisStatus {
    if record_task_id != current.task_id
        || validity_basis.task_id.as_str() != current.task_id
        || attempt_scope.task_id.as_str() != current.task_id
        || record_change_unit_id != current.change_unit_id.as_deref()
        || Some(validity_basis.change_unit_id.as_str()) != current.change_unit_id.as_deref()
        || Some(attempt_scope.change_unit_id.as_str()) != current.change_unit_id.as_deref()
        || validity_basis.scope_revision != current.scope_revision
        || validity_basis.baseline_ref != current.task_baseline_ref
        || validity_basis.baseline_ref != current.change_unit_baseline_ref
        || attempt_scope.baseline_ref != validity_basis.baseline_ref
        || validity_basis.workspace_context_sha256 != current.change_unit_workspace_sha256
    {
        return WriteTicketOwnerBasisStatus::Stale;
    }
    match &current.workspace_probe {
        CurrentWorkspaceProbe::Available(workspace_sha256)
            if workspace_sha256 != &validity_basis.workspace_context_sha256 =>
        {
            WriteTicketOwnerBasisStatus::Stale
        }
        CurrentWorkspaceProbe::Unavailable if validity_basis.workspace_context_sha256.is_some() => {
            WriteTicketOwnerBasisStatus::WorkspaceUncertain
        }
        _ => WriteTicketOwnerBasisStatus::Current,
    }
}

fn sensitive_approval_matches_ticket(
    approval: &CurrentSensitiveApproval,
    validity_basis: &WriteTicketValidityBasis,
    attempt_scope: &WriteTicketAttemptScope,
) -> bool {
    if !approval
        .required_for
        .contains(&UserActionRequiredFor::PrepareWrite)
    {
        return false;
    }
    let coordinates = approval.basis.coordinates();
    if coordinates.task_id != validity_basis.task_id
        || coordinates.change_unit_id.as_ref() != Some(&validity_basis.change_unit_id)
        || coordinates.scope_revision != validity_basis.scope_revision
        || coordinates.baseline_ref.as_ref() != validity_basis.baseline_ref.as_ref()
        || attempt_scope.task_id != validity_basis.task_id
        || attempt_scope.change_unit_id != validity_basis.change_unit_id
    {
        return false;
    }
    let Some(scope) = approval.basis.sensitive_action_scope() else {
        return false;
    };
    let approved_categories = scope
        .sensitive_categories
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    scope.action_kind == attempt_scope.intended_operation
        && attempt_scope
            .sensitive_categories
            .iter()
            .all(|category| approved_categories.contains(category.as_str()))
        && attempt_scope.intended_paths.iter().all(|path| {
            scope
                .intended_paths
                .iter()
                .any(|approved| path_is_within(path.as_str(), approved))
        })
}
