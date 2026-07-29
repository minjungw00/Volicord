use crate::pipeline::{CorePipelineError, VerifiedInvocationContext};
use crate::write_ticket::workspace_context_matches;
use crate::write_ticket::{
    run_write_ticket_mismatch, write_ticket_is_idle_expired, RunWriteTicketAttempt,
};
use chrono::{DateTime, Utc};
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, TaskRecord, WriteTicketRecord,
};
use volicord_types::ids::{BaselineRef, ChangeUnitId, ProjectId, TaskId};
use volicord_types::product_path::path_is_within;
use volicord_types::schema::{ObservedChanges, WriteTicketAttemptScope, WriteTicketValidityBasis};
use volicord_types::values::{
    StateRecordKind, TaskControlLevel, UserActionKind, UserActionRequiredFor, UtcTimestamp,
    WriteTicketInvalidationReason, WriteTicketStatus,
};
use volicord_user_action_service::{
    current_sensitive_approval, user_action_authority_from_record, SensitiveApprovalRequirement,
    UserActionServiceError,
};

#[derive(Debug)]
pub(crate) enum WriteTicketAdmissionError {
    Core(CorePipelineError),
    UserAction(UserActionServiceError),
    Invalid {
        reason: &'static str,
        message: &'static str,
    },
}

impl From<CorePipelineError> for WriteTicketAdmissionError {
    fn from(error: CorePipelineError) -> Self {
        Self::Core(error)
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
    pub(crate) project_id: &'a ProjectId,
    pub(crate) task_id: &'a TaskId,
    pub(crate) change_unit_id: &'a ChangeUnitId,
    pub(crate) baseline_ref: &'a BaselineRef,
    pub(crate) performed_operation: Option<&'a str>,
    pub(crate) task: &'a TaskRecord,
    pub(crate) change_unit: &'a ChangeUnitRecord,
    pub(crate) verified_invocation: &'a VerifiedInvocationContext,
    pub(crate) observed_changes: &'a ObservedChanges,
    pub(crate) write_authority_fingerprint: &'a str,
    pub(crate) now: DateTime<Utc>,
}

pub(crate) fn admit_record_run(
    record: &WriteTicketRecord,
    context: RecordRunWriteAdmission<'_>,
) -> Result<WriteTicketAttemptScope, WriteTicketAdmissionError> {
    let RecordRunWriteAdmission {
        store,
        project_id,
        task_id,
        change_unit_id,
        baseline_ref,
        performed_operation,
        task,
        change_unit,
        verified_invocation,
        observed_changes,
        write_authority_fingerprint,
        now,
    } = context;
    if record.status != WriteTicketStatus::Active {
        let reason = match record.status {
            WriteTicketStatus::Consumed => "consumed",
            WriteTicketStatus::Invalidated => record
                .invalidation_reason
                .map(WriteTicketInvalidationReason::as_str)
                .ok_or_else(|| CorePipelineError::Invariant {
                    detail: "typed invalidated write ticket lacks an invalidation reason"
                        .to_owned(),
                })?,
            WriteTicketStatus::Revoked => "revoked",
            WriteTicketStatus::Active => "incompatible",
        };
        return Err(write_ticket_invalid(reason, "write ticket is not active"));
    }
    if write_ticket_is_idle_expired(record, now).map_err(CorePipelineError::from)? {
        return Err(write_ticket_invalid(
            "idle_timeout",
            "write ticket crossed its configured idle-timeout boundary",
        ));
    }
    if !workspace_context_matches(change_unit, verified_invocation)? {
        return write_ticket_mismatch(
            "workspace_context_mismatch",
            "current Git workspace context differs from the write ticket Change Unit basis",
        );
    }
    let validity_basis = &record.validity_basis;
    if validity_basis.write_authority_fingerprint != write_authority_fingerprint {
        return write_ticket_mismatch(
            "policy_authority_mismatch",
            "write ticket policy authority is no longer current",
        );
    }
    let current_workspace_sha256 = verified_invocation
        .git_workspace_context
        .as_ref()
        .map(volicord_types::canonical::canonical_json_bare_sha256)
        .transpose()?;
    if &validity_basis.task_id != task_id {
        return write_ticket_mismatch(
            "task_mismatch",
            "write ticket validity basis names another Task",
        );
    }
    if &validity_basis.change_unit_id != change_unit_id {
        return write_ticket_mismatch(
            "change_unit_mismatch",
            "write ticket validity basis names another Change Unit",
        );
    }
    if validity_basis.scope_revision != task.scope_revision {
        return write_ticket_mismatch(
            "scope_revision_changed",
            "write ticket scope revision is no longer current",
        );
    }
    if validity_basis.baseline_ref.as_ref() != Some(baseline_ref) {
        return write_ticket_mismatch(
            "baseline_mismatch",
            "write ticket baseline is no longer current",
        );
    }
    if validity_basis.workspace_context_sha256 != current_workspace_sha256 {
        return write_ticket_mismatch(
            "workspace_context_mismatch",
            "write ticket workspace context is no longer current",
        );
    }
    let scope = &record.attempt_scope;
    let scope_paths = record
        .allowed_path_prefixes
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect::<Vec<_>>();
    if let Some(mismatch) = run_write_ticket_mismatch(
        record,
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
            .denied_path_prefixes
            .iter()
            .any(|denied| path_is_within(path, denied.as_str()))
    }) {
        return write_ticket_mismatch(
            "path_mismatch",
            "write ticket denied path prefixes do not cover the recorded run",
        );
    }
    let authority_now = UtcTimestamp::from_datetime(now);
    if !write_ticket_approval_basis_is_current(WriteTicketApprovalBasisContext {
        store,
        project_id,
        task_id,
        change_unit_id,
        task,
        scope,
        validity_basis,
        now: &authority_now,
    })? {
        return write_ticket_mismatch(
            "approval_basis_changed",
            "write ticket approval basis is no longer current",
        );
    }
    Ok(scope.clone())
}

struct WriteTicketApprovalBasisContext<'a> {
    store: &'a CoreProjectStore<'a>,
    project_id: &'a ProjectId,
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
        project_id,
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
    let records = store
        .resolved_user_action_records(task_id, UserActionKind::SensitiveApproval, now)
        .map_err(CorePipelineError::from)?;
    let mut current_resolution_ids = BTreeSet::new();
    for record in records {
        let authority = user_action_authority_from_record(&record)?;
        if current_sensitive_approval(&authority, &requirement) {
            if let Some(resolution_id) = authority.user_action_resolution_id {
                current_resolution_ids.insert(resolution_id);
            }
        }
    }

    Ok(!current_resolution_ids.is_empty()
        && validity_basis.approval_basis_refs.iter().all(|reference| {
            reference.record_kind == StateRecordKind::UserActionResolution
                && &reference.project_id == project_id
                && reference.task_id.as_ref() == Some(task_id)
                && current_resolution_ids.contains(reference.record_id.as_str())
        }))
}

fn write_ticket_mismatch(
    reason: &'static str,
    message: &'static str,
) -> Result<WriteTicketAttemptScope, WriteTicketAdmissionError> {
    Err(write_ticket_invalid(reason, message))
}

fn write_ticket_invalid(reason: &'static str, message: &'static str) -> WriteTicketAdmissionError {
    WriteTicketAdmissionError::Invalid { reason, message }
}
