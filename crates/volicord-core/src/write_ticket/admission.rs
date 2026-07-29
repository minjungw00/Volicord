use crate::pipeline::{CorePipelineError, GitWorkspaceContext};
use crate::write_ticket::workspace_context_matches;
use crate::write_ticket::{
    run_write_ticket_mismatch, write_ticket_is_idle_expired, RunWriteTicketAttempt,
    WriteTicketInvalidReason,
};
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, StoredWriteTicket, TaskRecord,
};
use volicord_store::error::StoreError;
use volicord_types::ids::{BaselineRef, ChangeUnitId, TaskId};
use volicord_types::product_path::path_is_within;
use volicord_types::schema::{ObservedChanges, WriteTicketAttemptScope, WriteTicketValidityBasis};
use volicord_types::values::{
    TaskControlLevel, UserActionKind, UserActionRequiredFor, UtcTimestamp,
    WriteTicketInvalidationReason, WriteTicketStatus,
};
use volicord_user_action_service::{
    current_sensitive_approval, user_action_authority_from_record, SensitiveApprovalRequirement,
    UserActionServiceError,
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
    if !write_ticket_approval_basis_is_current(WriteTicketApprovalBasisContext {
        store,
        task_id,
        change_unit_id,
        task,
        scope,
        validity_basis,
        now: observed_at,
    })? {
        return write_ticket_mismatch(
            WriteTicketInvalidReason::ApprovalBasisChanged,
            "write ticket approval basis is no longer current",
        );
    }
    Ok(scope.clone())
}

struct WriteTicketApprovalBasisContext<'a> {
    store: &'a CoreProjectStore<'a>,
    task_id: &'a TaskId,
    change_unit_id: &'a ChangeUnitId,
    task: &'a TaskRecord,
    scope: &'a WriteTicketAttemptScope,
    validity_basis: &'a WriteTicketValidityBasis,
    now: &'a UtcTimestamp,
}

fn write_ticket_approval_basis_is_current(
    context: WriteTicketApprovalBasisContext<'_>,
) -> Result<bool, WriteTicketAdmissionError> {
    let WriteTicketApprovalBasisContext {
        store,
        task_id,
        change_unit_id,
        task,
        scope,
        validity_basis,
        now,
    } = context;
    if validity_basis.approval_basis_refs.is_empty() {
        return Ok(scope.sensitive_categories.is_empty()
            && task.effective_control_level != TaskControlLevel::Sensitive);
    }

    let normalized_scope_paths = scope
        .intended_paths
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect::<Vec<_>>();
    let requirement = SensitiveApprovalRequirement {
        task_id,
        change_unit_id,
        scope_revision: task.scope_revision,
        operation: &scope.intended_operation,
        normalized_paths: &normalized_scope_paths,
        sensitive_categories: &scope.sensitive_categories,
        baseline_ref: scope.baseline_ref.as_ref(),
        required_for: UserActionRequiredFor::PrepareWrite,
        now,
    };
    let records =
        store.resolved_user_action_records(task_id, UserActionKind::SensitiveApproval, now)?;
    let mut current_resolution_identities = BTreeSet::new();
    for record in records {
        let authority = user_action_authority_from_record(&record)?;
        if current_sensitive_approval(&authority, &requirement) {
            if let Some(identity) = authority.resolution_identity() {
                current_resolution_identities.insert(identity);
            }
        }
    }

    Ok(!current_resolution_identities.is_empty()
        && validity_basis
            .approval_basis_refs
            .iter()
            .all(|reference| current_resolution_identities.contains(&reference.identity())))
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
