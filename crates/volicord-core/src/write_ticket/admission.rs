use crate::pipeline::{CorePipelineError, GitWorkspaceContext};
use crate::write_ticket::workspace_context_matches;
use crate::write_ticket::{
    run_write_ticket_mismatch, write_ticket_is_idle_expired, RunWriteTicketAttempt,
    WriteTicketInvalidReason,
};
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, StoredWriteTicket, TaskRecord,
};
use volicord_store::error::StoreError;
use volicord_types::ids::{BaselineRef, ChangeUnitId, ProjectId, TaskId};
use volicord_types::product_path::path_is_within;
use volicord_types::schema::{ObservedChanges, WriteTicketAttemptScope};
use volicord_types::values::{
    TaskControlLevel, UserActionKind, UtcTimestamp, WriteTicketInvalidationReason,
    WriteTicketStatus,
};
use volicord_user_action_service::{user_action_authority_from_record, UserActionServiceError};

use super::approval::{
    assess_write_ticket_approval, ApprovalBasisChangeReason, CurrentSensitiveApprovals,
    WriteTicketApprovalAssessment, WriteTicketApprovalRequirement,
};

#[derive(Debug)]
pub(crate) enum WriteTicketAdmissionError {
    Core(CorePipelineError),
    Store(StoreError),
    UserAction(UserActionServiceError),
    Invalid {
        reason: WriteTicketInvalidReason,
        message: &'static str,
    },
}

impl From<CorePipelineError> for WriteTicketAdmissionError {
    fn from(error: CorePipelineError) -> Self {
        match error {
            CorePipelineError::Store(error) => Self::Store(error),
            error => Self::Core(error),
        }
    }
}

impl From<StoreError> for WriteTicketAdmissionError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<serde_json::Error> for WriteTicketAdmissionError {
    fn from(error: serde_json::Error) -> Self {
        Self::Core(CorePipelineError::from(error))
    }
}

impl From<UserActionServiceError> for WriteTicketAdmissionError {
    fn from(error: UserActionServiceError) -> Self {
        Self::UserAction(error)
    }
}

pub(crate) struct RecordRunWriteAdmission<'a> {
    pub(crate) store: &'a CoreProjectStore<'a>,
    pub(crate) task_id: &'a TaskId,
    pub(crate) change_unit_id: &'a ChangeUnitId,
    pub(crate) baseline_ref: &'a BaselineRef,
    pub(crate) performed_operation: Option<&'a str>,
    pub(crate) task: &'a TaskRecord,
    pub(crate) change_unit: &'a ChangeUnitRecord,
    pub(crate) git_workspace_context: Option<&'a GitWorkspaceContext>,
    pub(crate) observed_changes: &'a ObservedChanges,
    pub(crate) write_authority_fingerprint: &'a str,
    pub(crate) observed_at: &'a UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RecordRunApprovalAdmission {
    Admitted,
    Rejected(ApprovalBasisChangeReason),
}

pub(crate) fn record_run_approval_admission(
    assessment: WriteTicketApprovalAssessment,
) -> RecordRunApprovalAdmission {
    match assessment {
        WriteTicketApprovalAssessment::Current { .. }
        | WriteTicketApprovalAssessment::NotRequired => RecordRunApprovalAdmission::Admitted,
        WriteTicketApprovalAssessment::Changed { reason } => {
            RecordRunApprovalAdmission::Rejected(reason)
        }
    }
}

pub(crate) fn admit_record_run(
    record: &StoredWriteTicket,
    context: RecordRunWriteAdmission<'_>,
) -> Result<WriteTicketAttemptScope, WriteTicketAdmissionError> {
    let RecordRunWriteAdmission {
        store,
        task_id,
        change_unit_id,
        baseline_ref,
        performed_operation,
        task,
        change_unit,
        git_workspace_context,
        observed_changes,
        write_authority_fingerprint,
        observed_at,
    } = context;
    if record.status() != WriteTicketStatus::Active {
        let reason = match record.status() {
            WriteTicketStatus::Consumed => WriteTicketInvalidReason::Consumed,
            WriteTicketStatus::Invalidated => record
                .invalidation_reason()
                .map(WriteTicketInvalidReason::Invalidated)
                .ok_or_else(|| CorePipelineError::Invariant {
                    detail: "typed invalidated write ticket lacks an invalidation reason"
                        .to_owned(),
                })?,
            WriteTicketStatus::Revoked => WriteTicketInvalidReason::Revoked,
            WriteTicketStatus::Active => unreachable!("active status was matched above"),
        };
        return Err(write_ticket_invalid(reason, "write ticket is not active"));
    }
    if write_ticket_is_idle_expired(record.idle_expires_at(), observed_at) {
        return Err(write_ticket_invalid(
            WriteTicketInvalidReason::Invalidated(WriteTicketInvalidationReason::IdleTimeout),
            "write ticket crossed its configured idle-timeout boundary",
        ));
    }
    if !workspace_context_matches(change_unit, git_workspace_context) {
        return write_ticket_mismatch(
            WriteTicketInvalidReason::WorkspaceContextMismatch,
            "current Git workspace context differs from the write ticket Change Unit basis",
        );
    }
    let validity_basis = record.validity_basis();
    if validity_basis.write_authority_fingerprint != write_authority_fingerprint {
        return write_ticket_mismatch(
            WriteTicketInvalidReason::PolicyAuthorityMismatch,
            "write ticket policy authority is no longer current",
        );
    }
    let current_workspace_sha256 = git_workspace_context
        .map(volicord_types::canonical::canonical_json_bare_sha256)
        .transpose()?;
    if &validity_basis.task_id != task_id {
        return write_ticket_mismatch(
            WriteTicketInvalidReason::TaskMismatch,
            "write ticket validity basis names another Task",
        );
    }
    if &validity_basis.change_unit_id != change_unit_id {
        return write_ticket_mismatch(
            WriteTicketInvalidReason::ChangeUnitMismatch,
            "write ticket validity basis names another Change Unit",
        );
    }
    if validity_basis.scope_revision != task.scope_revision {
        return write_ticket_mismatch(
            WriteTicketInvalidReason::ScopeRevisionChanged,
            "write ticket scope revision is no longer current",
        );
    }
    if validity_basis.baseline_ref.as_ref() != Some(baseline_ref) {
        return write_ticket_mismatch(
            WriteTicketInvalidReason::BaselineMismatch,
            "write ticket baseline is no longer current",
        );
    }
    if validity_basis.workspace_context_sha256 != current_workspace_sha256 {
        return write_ticket_mismatch(
            WriteTicketInvalidReason::WorkspaceContextMismatch,
            "write ticket workspace context is no longer current",
        );
    }
    let scope = record.attempt_scope();
    let scope_paths = record
        .allowed_path_prefixes()
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect::<Vec<_>>();
    if let Some(mismatch) = run_write_ticket_mismatch(
        scope,
        RunWriteTicketAttempt {
            task_id,
            change_unit_id,
            baseline_ref,
            performed_operation,
            performed_operation_required: !observed_changes.product_file_write_observed
                && task.effective_control_level == TaskControlLevel::Sensitive,
            observed_changes,
            normalized_scope_paths: &scope_paths,
        },
    ) {
        return write_ticket_mismatch(mismatch.reason, mismatch.message);
    }
    if observed_changes.changed_paths.iter().any(|path| {
        record
            .denied_path_prefixes()
            .iter()
            .any(|denied| path_is_within(path, denied.as_str()))
    }) {
        return write_ticket_mismatch(
            WriteTicketInvalidReason::PathMismatch,
            "write ticket denied path prefixes do not cover the recorded run",
        );
    }
    let authorities = store
        .resolved_user_action_records(task_id, UserActionKind::SensitiveApproval, observed_at)?
        .iter()
        .map(user_action_authority_from_record)
        .collect::<Result<Vec<_>, _>>()?;
    let ticket_project_id = ProjectId::new(record.project_id());
    let approval_requirement = WriteTicketApprovalRequirement::new(
        &ticket_project_id,
        task.scope_revision,
        task.effective_control_level,
        scope,
        observed_at,
    );
    let current_approvals = CurrentSensitiveApprovals::new(&authorities, &approval_requirement);
    let approval_assessment = assess_write_ticket_approval(
        &approval_requirement,
        &current_approvals,
        &validity_basis.approval_basis_refs,
    );
    if matches!(
        record_run_approval_admission(approval_assessment),
        RecordRunApprovalAdmission::Rejected(_)
    ) {
        return write_ticket_mismatch(
            WriteTicketInvalidReason::ApprovalBasisChanged,
            "write ticket approval basis is no longer current",
        );
    }
    Ok(scope.clone())
}

fn write_ticket_mismatch(
    reason: WriteTicketInvalidReason,
    message: &'static str,
) -> Result<WriteTicketAttemptScope, WriteTicketAdmissionError> {
    Err(write_ticket_invalid(reason, message))
}

fn write_ticket_invalid(
    reason: WriteTicketInvalidReason,
    message: &'static str,
) -> WriteTicketAdmissionError {
    WriteTicketAdmissionError::Invalid { reason, message }
}
