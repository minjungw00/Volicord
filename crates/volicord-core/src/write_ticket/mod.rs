mod admission;
pub(crate) mod approval;
pub(crate) mod current_validity;
mod facts;
mod planning;
mod policy;
pub(crate) mod read_model;
pub(crate) mod selection;
pub(crate) mod semantic;
pub(crate) mod service;
pub(crate) mod summary;

use crate::pipeline::CorePipelineError;
use volicord_store::StoreError;
use volicord_types::ids::{ChangeUnitId, TaskId, UserActionRequestId};
use volicord_types::values::{WriteDecisionCategory, WriteTicketInvalidationReason};
use volicord_user_action_service::UserActionServiceError;

#[derive(Debug)]
pub(crate) enum WriteTicketPlanningError {
    Core(CorePipelineError),
    Store(StoreError),
    UserAction(UserActionServiceError),
    NoActiveTask,
    CurrentChangeUnitRequired {
        task_id: TaskId,
    },
    Validation {
        field: WriteTicketField,
        message: &'static str,
    },
    ProductPathContainment {
        message: &'static str,
    },
    Invariant {
        detail: String,
    },
}

impl From<CorePipelineError> for WriteTicketPlanningError {
    fn from(error: CorePipelineError) -> Self {
        match error {
            CorePipelineError::Store(error) => Self::Store(error),
            error => Self::Core(error),
        }
    }
}

impl From<StoreError> for WriteTicketPlanningError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<UserActionServiceError> for WriteTicketPlanningError {
    fn from(error: UserActionServiceError) -> Self {
        Self::UserAction(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteTicketField {
    IntendedOperation,
    IntendedPaths,
    TaskId,
}

impl WriteTicketField {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::IntendedOperation => "intended_operation",
            Self::IntendedPaths => "intended_paths",
            Self::TaskId => "task_id",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteTicketDecisionCode {
    ScopeNotCurrent,
    PathOutOfScope,
    SensitiveApprovalMissing,
    UserActionUnresolved,
    BaselineMismatch,
    WorkspaceContextMismatch,
    EffectContractForbidsProductFileWrite,
    EffectContractEffectNotAllowed,
    EffectContractPathNotAllowed,
    ProductWriteFlagMismatch,
}

impl WriteTicketDecisionCode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ScopeNotCurrent => "scope_not_current",
            Self::PathOutOfScope => "path_out_of_scope",
            Self::SensitiveApprovalMissing => "sensitive_approval_missing",
            Self::UserActionUnresolved => "user_action_unresolved",
            Self::BaselineMismatch => "baseline_mismatch",
            Self::WorkspaceContextMismatch => "workspace_context_mismatch",
            Self::EffectContractForbidsProductFileWrite => {
                "effect_contract_forbids_product_file_write"
            }
            Self::EffectContractEffectNotAllowed => "effect_contract_effect_not_allowed",
            Self::EffectContractPathNotAllowed => "effect_contract_path_not_allowed",
            Self::ProductWriteFlagMismatch => "product_write_flag_mismatch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WriteTicketRelatedRecord {
    Task(TaskId),
    CurrentChangeUnit {
        task_id: TaskId,
        change_unit_id: ChangeUnitId,
    },
    UserActionRequest {
        task_id: TaskId,
        request_id: UserActionRequestId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteTicketDecisionReason {
    pub(crate) category: WriteDecisionCategory,
    pub(crate) code: WriteTicketDecisionCode,
    pub(crate) message: &'static str,
    pub(crate) related_records: Vec<WriteTicketRelatedRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteTicketInvalidReason {
    Missing,
    Incompatible,
    Consumed,
    Invalidated(WriteTicketInvalidationReason),
    Revoked,
    WorkspaceContextMismatch,
    PolicyAuthorityMismatch,
    TaskMismatch,
    ChangeUnitMismatch,
    ScopeRevisionChanged,
    BaselineMismatch,
    ProductWriteFlagMismatch,
    OperationMismatch,
    SensitiveCategoryMismatch,
    PathMismatch,
    ApprovalBasisChanged,
}

impl WriteTicketInvalidReason {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Incompatible => "incompatible",
            Self::Consumed => "consumed",
            Self::Invalidated(reason) => reason.as_str(),
            Self::Revoked => "revoked",
            Self::WorkspaceContextMismatch => "workspace_context_mismatch",
            Self::PolicyAuthorityMismatch => "policy_authority_mismatch",
            Self::TaskMismatch => "task_mismatch",
            Self::ChangeUnitMismatch => "change_unit_mismatch",
            Self::ScopeRevisionChanged => "scope_revision_changed",
            Self::BaselineMismatch => "baseline_mismatch",
            Self::ProductWriteFlagMismatch => "product_write_flag_mismatch",
            Self::OperationMismatch => "operation_mismatch",
            Self::SensitiveCategoryMismatch => "sensitive_category_mismatch",
            Self::PathMismatch => "path_mismatch",
            Self::ApprovalBasisChanged => "approval_basis_changed",
        }
    }
}

pub(crate) use admission::{admit_record_run, RecordRunWriteAdmission, WriteTicketAdmissionError};
pub(crate) use facts::{
    baseline_matches, load_prepare_write_task, paths_match_current_change_unit,
    validate_prepare_write_change_unit, workspace_context_matches,
};
pub(crate) use planning::{
    materialize_planned_write_ticket, plan_prepare_write, planned_write_ticket_mutation,
    PrepareWriteInput, PrepareWritePlanningOutcome,
};
pub(crate) use policy::{
    normalized_string_set, prepare_write_decision, run_write_ticket_mismatch,
    write_decision_reason, write_ticket_is_idle_expired, RunWriteTicketAttempt,
};
