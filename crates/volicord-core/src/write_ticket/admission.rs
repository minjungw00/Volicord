use crate::pipeline::{CorePipelineError, GitWorkspaceContext};
use crate::write_ticket::workspace_context_matches;
use crate::write_ticket::{
    run_write_ticket_mismatch, RunWriteTicketAttempt, WriteTicketInvalidReason,
};
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, StoredWriteTicket, TaskRecord,
};
use volicord_store::error::StoreError;
use volicord_types::ids::{BaselineRef, ChangeUnitId, TaskId, WriteTicketId};
use volicord_types::product_path::path_is_within;
use volicord_types::schema::ObservedChanges;
use volicord_types::values::{TaskControlLevel, UserActionKind, UtcTimestamp};
use volicord_user_action_service::{user_action_authority_from_record, UserActionServiceError};

use super::approval::{
    assess_write_ticket_approval, CurrentSensitiveApprovals, WriteTicketApprovalRequirement,
};
use super::current_validity::{
    evaluate_active_candidate, pre_evaluate_stored_write_ticket, ActiveStoredWriteTicketCandidate,
    ActiveStoredWriteTicketEvaluation, ReusableStoredWriteTicket, StoredTicketPreEvaluation,
    StoredWriteTicketStateError, TerminalStoredWriteTicketEvaluation, WriteTicketAuthorityState,
};
use super::read_model::{
    stored_write_ticket_facts, WriteTicketCurrentFacts, WriteTicketTaskFacts,
    WriteTicketWorkflowFacts,
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
    pub(crate) task_id: &'a TaskId,
    pub(crate) change_unit_id: &'a ChangeUnitId,
    pub(crate) baseline_ref: &'a BaselineRef,
    pub(crate) performed_operation: Option<&'a str>,
    pub(crate) task: &'a TaskRecord,
    pub(crate) change_unit: &'a ChangeUnitRecord,
    pub(crate) git_workspace_context: Option<&'a GitWorkspaceContext>,
    pub(crate) observed_changes: &'a ObservedChanges,
}

pub(crate) struct RecordRunTicketCurrentness<'a> {
    pub(crate) store: &'a CoreProjectStore<'a>,
    pub(crate) task_id: &'a TaskId,
    pub(crate) task: &'a TaskRecord,
    pub(crate) write_authority_fingerprint: &'a str,
    pub(crate) observed_at: &'a UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdmissibleStoredWriteTicket {
    reusable: ReusableStoredWriteTicket,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompatibleRecordRunAttempt {
    write_ticket_id: WriteTicketId,
}

impl AdmissibleStoredWriteTicket {
    pub(crate) fn write_ticket_id(&self) -> &volicord_types::ids::WriteTicketId {
        self.reusable.write_ticket_id()
    }

    pub(crate) fn semantic_facts(&self) -> &super::semantic::WriteTicketSemanticFacts {
        self.reusable.semantic_facts()
    }

    pub(crate) fn reusable(&self) -> &ReusableStoredWriteTicket {
        &self.reusable
    }
}

pub(crate) fn admit_record_run(
    reusable: ReusableStoredWriteTicket,
    compatible_attempt: CompatibleRecordRunAttempt,
) -> Result<AdmissibleStoredWriteTicket, WriteTicketAdmissionError> {
    if reusable.write_ticket_id() != &compatible_attempt.write_ticket_id {
        return Err(WriteTicketAdmissionError::Core(
            CorePipelineError::Invariant {
                detail: "Record Run compatibility proof names another reusable Write Ticket"
                    .to_owned(),
            },
        ));
    }
    Ok(AdmissibleStoredWriteTicket { reusable })
}

fn validate_record_run_attempt(
    ticket_id: &WriteTicketId,
    ticket: &super::semantic::WriteTicketSemanticFacts,
    context: &RecordRunWriteAdmission<'_>,
) -> Result<CompatibleRecordRunAttempt, WriteTicketAdmissionError> {
    let RecordRunWriteAdmission {
        task_id,
        change_unit_id,
        baseline_ref,
        performed_operation,
        task,
        change_unit: _,
        git_workspace_context,
        observed_changes,
    } = *context;
    let validity_basis = &ticket.validity_basis;
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
    let scope = &ticket.attempt_scope;
    let scope_paths = ticket
        .allowed_path_prefixes
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
        ticket
            .denied_path_prefixes
            .iter()
            .any(|denied| path_is_within(path, denied.as_str()))
    }) {
        return write_ticket_mismatch(
            WriteTicketInvalidReason::PathMismatch,
            "write ticket denied path prefixes do not cover the recorded run",
        );
    }
    Ok(CompatibleRecordRunAttempt {
        write_ticket_id: ticket_id.clone(),
    })
}

pub(crate) fn active_record_run_candidate(
    record: &StoredWriteTicket,
    observed_at: &UtcTimestamp,
) -> Result<ActiveStoredWriteTicketCandidate, WriteTicketAdmissionError> {
    match pre_evaluate_stored_write_ticket(stored_write_ticket_facts(record), observed_at)
        .map_err(stored_state_error)?
    {
        StoredTicketPreEvaluation::NeedsCurrentFacts(candidate) => Ok(candidate),
        StoredTicketPreEvaluation::Complete(terminal) => Err(terminal_admission_error(terminal)),
    }
}

pub(crate) fn evaluate_record_run_candidate(
    candidate: ActiveStoredWriteTicketCandidate,
    context: RecordRunTicketCurrentness<'_>,
    admission: &RecordRunWriteAdmission<'_>,
) -> Result<(ReusableStoredWriteTicket, CompatibleRecordRunAttempt), WriteTicketAdmissionError> {
    let RecordRunTicketCurrentness {
        store,
        task_id,
        task,
        write_authority_fingerprint,
        observed_at,
    } = context;
    let ticket = candidate.semantic_facts();
    if !workspace_context_matches(admission.change_unit, admission.git_workspace_context) {
        return Err(write_ticket_invalid(
            WriteTicketInvalidReason::WorkspaceContextMismatch,
            "current Git workspace context differs from the write ticket Change Unit basis",
        ));
    }
    if ticket.validity_basis.write_authority_fingerprint != write_authority_fingerprint {
        return Err(write_ticket_invalid(
            WriteTicketInvalidReason::PolicyAuthorityMismatch,
            "write ticket policy authority is no longer current",
        ));
    }
    let compatible_attempt =
        validate_record_run_attempt(candidate.write_ticket_id(), ticket, admission)?;
    let approval_requirement = WriteTicketApprovalRequirement::new(
        &ticket.project_id,
        task.scope_revision,
        task.effective_control_level,
        &ticket.attempt_scope,
        observed_at,
    );
    let authorities = store
        .resolved_user_action_records(task_id, UserActionKind::SensitiveApproval, observed_at)?
        .iter()
        .map(user_action_authority_from_record)
        .collect::<Result<Vec<_>, _>>()?;
    let current_approvals = CurrentSensitiveApprovals::new(&authorities, &approval_requirement);
    let approval_assessment = assess_write_ticket_approval(
        &approval_requirement,
        &current_approvals,
        &ticket.validity_basis.approval_basis_refs,
    );
    let current = WriteTicketCurrentFacts {
        task: WriteTicketTaskFacts {
            scope_revision: task.scope_revision,
            effective_control_level: task.effective_control_level,
            pending_policy_reevaluation: false,
        },
        workflow: WriteTicketWorkflowFacts {
            write_authority_fingerprint: write_authority_fingerprint.to_owned(),
        },
        sensitive_approvals: authorities,
        observed_at: observed_at.clone(),
    };
    match evaluate_active_candidate(candidate, &current, approval_assessment) {
        ActiveStoredWriteTicketEvaluation::Reusable(reusable) => Ok((reusable, compatible_attempt)),
        ActiveStoredWriteTicketEvaluation::Invalidated(ticket)
            if matches!(
                ticket.authority(),
                WriteTicketAuthorityState::WriteAuthorityChanged
                    | WriteTicketAuthorityState::PendingPolicyReevaluation
            ) =>
        {
            Err(write_ticket_invalid(
                WriteTicketInvalidReason::PolicyAuthorityMismatch,
                "write ticket policy authority is no longer current",
            ))
        }
        ActiveStoredWriteTicketEvaluation::Invalidated(_) => Err(write_ticket_invalid(
            WriteTicketInvalidReason::ApprovalBasisChanged,
            "write ticket approval basis is no longer current",
        )),
    }
}

fn terminal_admission_error(
    terminal: TerminalStoredWriteTicketEvaluation,
) -> WriteTicketAdmissionError {
    match terminal {
        TerminalStoredWriteTicketEvaluation::Invalidated(ticket) => write_ticket_invalid(
            WriteTicketInvalidReason::Invalidated(ticket.invalidation()),
            "write ticket is invalidated",
        ),
        TerminalStoredWriteTicketEvaluation::Consumed(_) => write_ticket_invalid(
            WriteTicketInvalidReason::Consumed,
            "write ticket is already consumed",
        ),
        TerminalStoredWriteTicketEvaluation::Revoked(_) => {
            write_ticket_invalid(WriteTicketInvalidReason::Revoked, "write ticket is revoked")
        }
    }
}

fn stored_state_error(error: StoredWriteTicketStateError) -> WriteTicketAdmissionError {
    WriteTicketAdmissionError::Core(CorePipelineError::Invariant {
        detail: format!(
            "Store-validated Write Ticket could not enter the Core stored type-state family: {error:?}"
        ),
    })
}

fn write_ticket_mismatch(
    reason: WriteTicketInvalidReason,
    message: &'static str,
) -> Result<CompatibleRecordRunAttempt, WriteTicketAdmissionError> {
    Err(write_ticket_invalid(reason, message))
}

fn write_ticket_invalid(
    reason: WriteTicketInvalidReason,
    message: &'static str,
) -> WriteTicketAdmissionError {
    WriteTicketAdmissionError::Invalid { reason, message }
}
