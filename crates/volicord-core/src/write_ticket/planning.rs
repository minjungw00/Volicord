use crate::pipeline::{CorePipelineError, GitWorkspaceContext};
use crate::policy::effect_contract::{product_write_violations, EffectContractViolation};
use crate::policy::workflow::{
    acceptance_policy_for_control, project_workflow_policy, resolve_task_control_authority,
    ProjectWorkflowPolicy,
};
use crate::product_path::{observe_product_paths, ProductPathValidationError};
use crate::record_refs::state_ref;
use crate::write_ticket::{
    baseline_matches, load_prepare_write_task, paths_match_current_change_unit,
    validate_prepare_write_change_unit, workspace_context_matches,
};
use crate::write_ticket::{normalized_string_set, prepare_write_decision, write_decision_reason};
use chrono::Duration;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use volicord_platform_fs::PlatformDiagnosticClass;
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, CoreStorageMutation, TaskControlLevelUpdate, TaskMutation,
    TaskRecord, WriteTicketByIdInvalidation, WriteTicketInsert, WriteTicketMutation,
};
use volicord_store::diagnostics::{
    record_core_rejection_diagnostic, CoreRejectionDiagnostic, CoreRejectionReason,
};
use volicord_store::error::StoreError;
use volicord_types::ids::{
    BaselineRef, ChangeUnitId, ProjectId, TaskId, UserActionRequestId, UserActionResolutionId,
    WriteTicketId,
};
use volicord_types::product_path::{
    ProductRelativePath, WriteTicketPathScope, WriteTicketPathScopeError,
};
use volicord_types::schema::{JsonObject, WriteTicketAttemptScope, WriteTicketValidityBasis};
use volicord_types::values::{
    AcceptancePolicy, ActorSource, MethodName, PrepareWriteDecision, StateRecordKind,
    TaskControlLevel, TaskMode, UserActionKind, UtcTimestamp, WorkPhase, WriteDecisionCategory,
    WriteTicketEffect, WriteTicketInvalidationReason,
};
use volicord_user_action_service::{
    pending_user_action_authorities, user_action_authority_from_record,
    user_action_blocks_operation, UserActionAuthority, UserActionOperation,
    UserActionOperationContext,
};

use super::approval::{
    assess_write_ticket_approval, CurrentSensitiveApprovals, NonEmptyApprovalBasis,
    WriteTicketApprovalRequirement,
};
use super::current_validity::{
    evaluate_active_candidate, pre_evaluate_stored_write_ticket, ActiveStoredWriteTicketEvaluation,
    ReusableStoredWriteTicket, StoredTicketPreEvaluation, StoredWriteTicketStateError,
    WriteTicketAuthorityState,
};
use super::read_model::{
    stored_write_ticket_facts, WriteTicketCurrentFacts, WriteTicketTaskFacts,
    WriteTicketWorkflowFacts,
};
use super::{
    WriteTicketDecisionCode, WriteTicketDecisionReason, WriteTicketField, WriteTicketPlanningError,
    WriteTicketRelatedRecord,
};

pub(crate) struct PrepareWriteInput {
    project_id: ProjectId,
    task_id: TaskId,
    task_is_current: bool,
    requested_change_unit_id: Option<ChangeUnitId>,
    intended_operation: String,
    intended_paths: Vec<String>,
    product_file_write_intended: bool,
    sensitive_categories: Vec<String>,
    baseline_ref: BaselineRef,
    actor_source: ActorSource,
    git_workspace_context: Option<GitWorkspaceContext>,
    verification_basis: String,
}

impl PrepareWriteInput {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        project_id: ProjectId,
        task_id: TaskId,
        task_is_current: bool,
        requested_change_unit_id: Option<ChangeUnitId>,
        intended_operation: String,
        intended_paths: Vec<String>,
        product_file_write_intended: bool,
        sensitive_categories: Vec<String>,
        baseline_ref: BaselineRef,
        actor_source: ActorSource,
        git_workspace_context: Option<GitWorkspaceContext>,
        verification_basis: String,
    ) -> Self {
        Self {
            project_id,
            task_id,
            task_is_current,
            requested_change_unit_id,
            intended_operation,
            intended_paths,
            product_file_write_intended,
            sensitive_categories,
            baseline_ref,
            actor_source,
            git_workspace_context,
            verification_basis,
        }
    }
}

struct PrepareWriteRawInput {
    input: PrepareWriteInput,
    plan_now: UtcTimestamp,
}

impl PrepareWriteRawInput {
    fn new(input: PrepareWriteInput, operation_now: &UtcTimestamp) -> Self {
        Self {
            input,
            plan_now: operation_now.clone(),
        }
    }
}

struct PrepareWriteNormalizedInput {
    raw: PrepareWriteRawInput,
    intended_operation: String,
    intended_paths: Vec<String>,
    sensitive_categories: Vec<String>,
}

struct PrepareWriteResolvedContext {
    normalized: PrepareWriteNormalizedInput,
    task: TaskRecord,
    change_unit: ChangeUnitRecord,
    reasons: Vec<WriteTicketDecisionReason>,
}

struct PrepareWritePolicyDecision {
    normalized: PrepareWriteNormalizedInput,
    task: TaskRecord,
    change_unit: ChangeUnitRecord,
    reasons: Vec<WriteTicketDecisionReason>,
    workflow_policy: ProjectWorkflowPolicy,
    control_mutations: Vec<CoreStorageMutation>,
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedWriteTicket {
    project_id: ProjectId,
    write_ticket_id: WriteTicketId,
    basis_state_version: u64,
    validity_basis: WriteTicketValidityBasis,
    path_scope: WriteTicketPathScope,
    attempt_scope: WriteTicketAttemptScope,
    created_by_actor_source: ActorSource,
    created_by_user_action_resolution_id: Option<UserActionResolutionId>,
    idle_expires_at: Option<UtcTimestamp>,
    created_at: UtcTimestamp,
    metadata: JsonObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlannedWriteTicketError {
    EmptyIdentity(&'static str),
    InvalidBasisStateVersion,
    InvalidScopeRevision,
    EmptyOperation,
    InvalidAuthorityFingerprint,
    InvalidWorkspaceDigest,
    TaskIdentityMismatch,
    ChangeUnitIdentityMismatch,
    ScopeRevisionExceedsBasis,
    BaselineMismatch,
    TimestampOrder,
    DuplicateIntendedPaths,
    IntendedPathNotAuthorized,
    ProductWriteIntentMismatch,
}

struct PlannedWriteTicketInput {
    project_id: ProjectId,
    write_ticket_id: WriteTicketId,
    basis_state_version: u64,
    validity_basis: WriteTicketValidityBasis,
    path_scope: WriteTicketPathScope,
    attempt_scope: WriteTicketAttemptScope,
    created_by_actor_source: ActorSource,
    created_by_user_action_resolution_id: Option<UserActionResolutionId>,
    idle_expires_at: Option<UtcTimestamp>,
    created_at: UtcTimestamp,
    metadata: JsonObject,
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedWriteTicketDraft {
    project_id: ProjectId,
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    scope_revision: u64,
    baseline_ref: BaselineRef,
    workspace_context_sha256: Option<String>,
    write_authority_fingerprint: String,
    approval_basis: Option<NonEmptyApprovalBasis>,
    path_scope: WriteTicketPathScope,
    attempt_scope: WriteTicketAttemptScope,
    created_by_actor_source: ActorSource,
    created_by_user_action_resolution_id: Option<UserActionResolutionId>,
    idle_expires_at: Option<UtcTimestamp>,
    created_at: UtcTimestamp,
    metadata: JsonObject,
}

impl PlannedWriteTicket {
    fn new(input: PlannedWriteTicketInput) -> Result<Self, PlannedWriteTicketError> {
        let ticket = Self {
            project_id: input.project_id,
            write_ticket_id: input.write_ticket_id,
            basis_state_version: input.basis_state_version,
            validity_basis: input.validity_basis,
            path_scope: input.path_scope,
            attempt_scope: input.attempt_scope,
            created_by_actor_source: input.created_by_actor_source,
            created_by_user_action_resolution_id: input.created_by_user_action_resolution_id,
            idle_expires_at: input.idle_expires_at,
            created_at: input.created_at,
            metadata: input.metadata,
        };
        ticket.validate()?;
        Ok(ticket)
    }

    fn validate(&self) -> Result<(), PlannedWriteTicketError> {
        for (field, value) in [
            ("project_id", self.project_id.as_str()),
            ("task_id", self.validity_basis.task_id.as_str()),
            (
                "change_unit_id",
                self.validity_basis.change_unit_id.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(PlannedWriteTicketError::EmptyIdentity(field));
            }
        }
        if self.write_ticket_id.as_str().trim().is_empty() {
            return Err(PlannedWriteTicketError::EmptyIdentity("write_ticket_id"));
        }
        if self
            .created_by_user_action_resolution_id
            .as_ref()
            .is_some_and(|id| id.as_str().trim().is_empty())
        {
            return Err(PlannedWriteTicketError::EmptyIdentity(
                "created_by_user_action_resolution_id",
            ));
        }
        if self.basis_state_version == 0 {
            return Err(PlannedWriteTicketError::InvalidBasisStateVersion);
        }
        if self.validity_basis.scope_revision == 0 {
            return Err(PlannedWriteTicketError::InvalidScopeRevision);
        }
        if self.attempt_scope.intended_operation.trim().is_empty() {
            return Err(PlannedWriteTicketError::EmptyOperation);
        }
        if !canonical_write_authority_fingerprint(&self.validity_basis.write_authority_fingerprint)
        {
            return Err(PlannedWriteTicketError::InvalidAuthorityFingerprint);
        }
        if self
            .validity_basis
            .workspace_context_sha256
            .as_ref()
            .is_some_and(|digest| !lowercase_sha256_hex(digest))
        {
            return Err(PlannedWriteTicketError::InvalidWorkspaceDigest);
        }
        if self.validity_basis.task_id != self.attempt_scope.task_id {
            return Err(PlannedWriteTicketError::TaskIdentityMismatch);
        }
        if self.validity_basis.change_unit_id != self.attempt_scope.change_unit_id {
            return Err(PlannedWriteTicketError::ChangeUnitIdentityMismatch);
        }
        if self.validity_basis.scope_revision > self.basis_state_version {
            return Err(PlannedWriteTicketError::ScopeRevisionExceedsBasis);
        }
        if self.validity_basis.baseline_ref != self.attempt_scope.baseline_ref {
            return Err(PlannedWriteTicketError::BaselineMismatch);
        }
        if self
            .idle_expires_at
            .as_ref()
            .is_some_and(|expires_at| expires_at <= &self.created_at)
            || self
                .created_at
                .ensure_canonical_rfc3339_representable()
                .is_err()
            || self.idle_expires_at.as_ref().is_some_and(|expires_at| {
                expires_at.ensure_canonical_rfc3339_representable().is_err()
            })
        {
            return Err(PlannedWriteTicketError::TimestampOrder);
        }
        let intended_paths = self
            .attempt_scope
            .intended_paths
            .iter()
            .collect::<BTreeSet<_>>();
        if intended_paths.len() != self.attempt_scope.intended_paths.len() {
            return Err(PlannedWriteTicketError::DuplicateIntendedPaths);
        }
        if self.attempt_scope.intended_paths.iter().any(|intended| {
            !self
                .path_scope
                .allowed()
                .iter()
                .any(|allowed| intended.is_within(allowed))
                || self
                    .path_scope
                    .denied()
                    .iter()
                    .any(|denied| intended.is_within(denied))
        }) {
            return Err(PlannedWriteTicketError::IntendedPathNotAuthorized);
        }
        if self.attempt_scope.product_file_write_intended
            != !self.attempt_scope.intended_paths.is_empty()
        {
            return Err(PlannedWriteTicketError::ProductWriteIntentMismatch);
        }
        Ok(())
    }

    fn persistence_input(&self) -> WriteTicketInsert {
        WriteTicketInsert {
            write_ticket_id: self.write_ticket_id.clone(),
            task_id: self.validity_basis.task_id.clone(),
            change_unit_id: self.validity_basis.change_unit_id.clone(),
            validity_basis: self.validity_basis.clone(),
            path_scope: self.path_scope.clone(),
            attempt_scope: self.attempt_scope.clone(),
            created_by_actor_source: self.created_by_actor_source.clone(),
            created_by_user_action_resolution_id: self.created_by_user_action_resolution_id.clone(),
            idle_expires_at: self.idle_expires_at.clone(),
            created_at: self.created_at.clone(),
            metadata: self.metadata.clone(),
        }
    }

    pub(crate) fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub(crate) fn write_ticket_id(&self) -> &WriteTicketId {
        &self.write_ticket_id
    }

    pub(crate) fn basis_state_version(&self) -> u64 {
        self.basis_state_version
    }

    pub(crate) fn validity_basis(&self) -> &WriteTicketValidityBasis {
        &self.validity_basis
    }

    pub(crate) fn path_scope(&self) -> &WriteTicketPathScope {
        &self.path_scope
    }

    pub(crate) fn attempt_scope(&self) -> &WriteTicketAttemptScope {
        &self.attempt_scope
    }

    pub(crate) fn idle_expires_at(&self) -> Option<&UtcTimestamp> {
        self.idle_expires_at.as_ref()
    }
}

fn planned_write_ticket_invariant(error: PlannedWriteTicketError) -> WriteTicketPlanningError {
    WriteTicketPlanningError::Invariant {
        detail: format!("planned Write Ticket is internally inconsistent: {error:?}"),
    }
}

pub(crate) fn materialize_planned_write_ticket(
    draft: PlannedWriteTicketDraft,
    write_ticket_id: WriteTicketId,
    basis_state_version: u64,
    approval_projection_state_version: u64,
) -> Result<PlannedWriteTicket, WriteTicketPlanningError> {
    let approval_basis_refs = draft
        .approval_basis
        .as_ref()
        .map(|basis| basis.resolution_refs(approval_projection_state_version))
        .unwrap_or_default();
    PlannedWriteTicket::new(PlannedWriteTicketInput {
        project_id: draft.project_id,
        write_ticket_id,
        basis_state_version,
        validity_basis: WriteTicketValidityBasis {
            task_id: draft.task_id,
            change_unit_id: draft.change_unit_id,
            scope_revision: draft.scope_revision,
            baseline_ref: Some(draft.baseline_ref),
            workspace_context_sha256: draft.workspace_context_sha256,
            write_authority_fingerprint: draft.write_authority_fingerprint,
            approval_basis_refs,
        },
        path_scope: draft.path_scope,
        attempt_scope: draft.attempt_scope,
        created_by_actor_source: draft.created_by_actor_source,
        created_by_user_action_resolution_id: draft.created_by_user_action_resolution_id,
        idle_expires_at: draft.idle_expires_at,
        created_at: draft.created_at,
        metadata: draft.metadata,
    })
    .map_err(planned_write_ticket_invariant)
}

pub(crate) fn planned_write_ticket_mutation(plan: &PlannedWriteTicket) -> CoreStorageMutation {
    CoreStorageMutation::WriteTicket(WriteTicketMutation::insert(plan.persistence_input()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteDecisionPathFacts {
    path_scope: WriteTicketPathScope,
}

impl WriteDecisionPathFacts {
    fn new(path_scope: WriteTicketPathScope) -> Self {
        Self { path_scope }
    }

    pub(crate) fn path_scope(&self) -> &WriteTicketPathScope {
        &self.path_scope
    }
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PrepareWriteTicketPlan {
    Issue(PlannedWriteTicketDraft),
    Reuse(ReusableStoredWriteTicket),
    NoTicket(WriteDecisionPathFacts),
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MaterializedPrepareWriteTicket {
    Issued(PlannedWriteTicket),
    Reused(ReusableStoredWriteTicket),
    None(WriteDecisionPathFacts),
}

impl MaterializedPrepareWriteTicket {
    pub(crate) fn write_ticket_effect(&self) -> WriteTicketEffect {
        match self {
            Self::Issued(_) => WriteTicketEffect::Issued,
            Self::Reused(_) => WriteTicketEffect::Reused,
            Self::None(_) => WriteTicketEffect::None,
        }
    }

    pub(crate) fn write_ticket_id(&self) -> Option<&WriteTicketId> {
        match self {
            Self::Issued(ticket) => Some(ticket.write_ticket_id()),
            Self::Reused(ticket) => Some(ticket.write_ticket_id()),
            Self::None(_) => None,
        }
    }

    pub(crate) fn path_scope(&self) -> &WriteTicketPathScope {
        match self {
            Self::Issued(ticket) => ticket.path_scope(),
            Self::Reused(ticket) => ticket.path_scope(),
            Self::None(facts) => facts.path_scope(),
        }
    }

    pub(crate) fn persistence_mutation(&self) -> Option<CoreStorageMutation> {
        match self {
            Self::Issued(ticket) => Some(planned_write_ticket_mutation(ticket)),
            Self::Reused(_) | Self::None(_) => None,
        }
    }
}

pub(crate) struct PrepareWriteCommonFacts {
    pub(crate) task_id: TaskId,
    pub(crate) task: TaskRecord,
    pub(crate) change_unit: ChangeUnitRecord,
    pub(crate) reasons: Vec<WriteTicketDecisionReason>,
    pub(crate) decision: PrepareWriteDecision,
    pub(crate) pending_user_action_request_ids: Vec<UserActionRequestId>,
    pub(crate) approval_basis: Option<NonEmptyApprovalBasis>,
}

pub(crate) struct PrepareWriteMutationFacts {
    pub(crate) storage_mutations: Vec<CoreStorageMutation>,
}

#[must_use]
pub(crate) struct PrepareWritePlanningOutcome {
    pub(crate) common: PrepareWriteCommonFacts,
    pub(crate) ticket: PrepareWriteTicketPlan,
    pub(crate) mutations: PrepareWriteMutationFacts,
}

pub(crate) fn plan_prepare_write(
    store: &CoreProjectStore,
    input: PrepareWriteInput,
    operation_now: &UtcTimestamp,
) -> Result<PrepareWritePlanningOutcome, WriteTicketPlanningError> {
    let raw = PrepareWriteRawInput::new(input, operation_now);
    let normalized = normalize_prepare_write_input(store, raw)?;
    let resolved = resolve_prepare_write_context(store, normalized)?;
    let policy = decide_prepare_write_policy(store, resolved)?;
    plan_prepare_write_mutations(store, policy)
}

fn resolve_prepare_write_context(
    store: &CoreProjectStore,
    normalized: PrepareWriteNormalizedInput,
) -> Result<PrepareWriteResolvedContext, WriteTicketPlanningError> {
    let input = &normalized.raw.input;
    let task_id = &input.task_id;
    let (task, mut reasons) = load_prepare_write_task(store, task_id, input.task_is_current)?;
    let change_unit = match store.current_change_unit(task_id)? {
        Some(change_unit) => change_unit,
        None => {
            if let Some(context) = store.mutation_context() {
                let _ = record_core_rejection_diagnostic(
                    context,
                    CoreRejectionDiagnostic {
                        project_id: input.project_id.as_str(),
                        task_id: task_id.as_str(),
                        method_name: MethodName::PrepareWrite,
                        reason: CoreRejectionReason::CurrentChangeUnitRequired,
                        occurred_at: &normalized.raw.plan_now,
                    },
                );
            }
            return Err(WriteTicketPlanningError::CurrentChangeUnitRequired {
                task_id: task_id.clone(),
            });
        }
    };
    validate_prepare_write_change_unit(
        input.requested_change_unit_id.as_ref(),
        task_id,
        &change_unit,
        &mut reasons,
    );

    Ok(PrepareWriteResolvedContext {
        normalized,
        task,
        change_unit,
        reasons,
    })
}

fn normalize_prepare_write_input(
    store: &CoreProjectStore,
    raw: PrepareWriteRawInput,
) -> Result<PrepareWriteNormalizedInput, WriteTicketPlanningError> {
    if raw.input.intended_operation.trim().is_empty() {
        return prepare_write_validation_error(
            WriteTicketField::IntendedOperation,
            "intended_operation must not be empty",
        );
    }
    let intended_operation = raw.input.intended_operation.trim().to_owned();
    let sensitive_categories = normalized_string_set(&raw.input.sensitive_categories);
    let intended_paths =
        match observe_product_paths(&store.project_record().repo_root, &raw.input.intended_paths) {
            Ok(paths) => paths,
            Err(ProductPathValidationError::Lexical(_)) => {
                return Err(WriteTicketPlanningError::Validation {
                    field: WriteTicketField::IntendedPaths,
                    message: "intended_paths must be normalized relative Product Repository paths",
                })
            }
            Err(error @ ProductPathValidationError::Platform(_))
                if error.platform_class() == Some(PlatformDiagnosticClass::Rejected) =>
            {
                return Err(WriteTicketPlanningError::ProductPathContainment {
                    message: "intended_paths resolve outside the Product Repository",
                })
            }
            Err(ProductPathValidationError::Platform(error)) => {
                return Err(CorePipelineError::from(error).into())
            }
        };
    Ok(PrepareWriteNormalizedInput {
        raw,
        intended_operation,
        intended_paths,
        sensitive_categories,
    })
}

fn decide_prepare_write_policy(
    store: &CoreProjectStore,
    resolved: PrepareWriteResolvedContext,
) -> Result<PrepareWritePolicyDecision, WriteTicketPlanningError> {
    let PrepareWriteResolvedContext {
        normalized,
        mut task,
        change_unit,
        reasons,
    } = resolved;
    if task.mode == TaskMode::Advisor {
        return prepare_write_validation_error(
            WriteTicketField::TaskId,
            "advisor Task mode does not support write preparation",
        );
    }
    let workflow_policy = project_workflow_policy(store)?;
    let current_control = task.effective_control_level;
    let current_acceptance = task.acceptance_policy;
    let resolved_control =
        resolve_task_control_authority(&task, &workflow_policy).map_err(CorePipelineError::from)?;
    let resolved_base_control = resolved_control.effective_control_level;
    let mut next_control = resolved_base_control;
    if next_control == TaskControlLevel::Observe {
        return prepare_write_validation_error(
            WriteTicketField::TaskId,
            "observe control does not permit product write preparation",
        );
    }
    let has_policy_denied_path = workflow_policy.has_denied_path(&normalized.intended_paths);
    if next_control == TaskControlLevel::Light
        && !workflow_policy.light_paths_are_allowed(&normalized.intended_paths)
    {
        next_control = TaskControlLevel::Tracked;
    }
    if has_policy_denied_path || !normalized.sensitive_categories.is_empty() {
        next_control = TaskControlLevel::Sensitive;
    }
    let control_acceptance = acceptance_policy_for_control(next_control, &workflow_policy);
    let next_acceptance = if acceptance_policy_rank(resolved_control.acceptance_policy)
        >= acceptance_policy_rank(control_acceptance)
    {
        resolved_control.acceptance_policy
    } else {
        control_acceptance
    };
    let acceptance_raised =
        acceptance_policy_rank(next_acceptance) > acceptance_policy_rank(current_acceptance);
    let control_raised = next_control > current_control;
    let next_control_reason = if control_raised {
        if has_policy_denied_path {
            "Core raised control to `sensitive` because an intended path matches a denied project-policy prefix."
                .to_owned()
        } else if next_control == TaskControlLevel::Sensitive {
            "Core raised control to `sensitive` for declared sensitive write effects.".to_owned()
        } else if resolved_control.pending_policy_reevaluation
            && next_control == resolved_base_control
        {
            resolved_control.control_level_reason.clone()
        } else if current_control == TaskControlLevel::Light
            && next_control == TaskControlLevel::Tracked
        {
            "Core raised control to `tracked` because intended paths exceed the Light project policy."
                .to_owned()
        } else {
            resolved_control.control_level_reason.clone()
        }
    } else {
        task.control_level_reason.clone()
    };
    let next_acceptance_reason = if acceptance_raised
        && next_control == resolved_base_control
        && resolved_control.acceptance_raised
    {
        resolved_control.acceptance_policy_reason.clone()
    } else {
        format!(
            "Effective control `{}` requires final acceptance for the current close basis.",
            next_control.as_str()
        )
    };
    let mut control_mutations = Vec::new();
    if control_raised || acceptance_raised || resolved_control.policy_reevaluation_marked {
        control_mutations.push(CoreStorageMutation::Task(TaskMutation::UpdateControlLevel(
            TaskControlLevelUpdate {
                task_id: task.task_id.clone(),
                effective_control_level: next_control,
                control_level_reason: next_control_reason.clone(),
                acceptance_policy: acceptance_raised.then_some(next_acceptance),
                acceptance_policy_reason: acceptance_raised.then(|| next_acceptance_reason.clone()),
            },
        )));
        task.effective_control_level = next_control;
        task.control_level_reason = next_control_reason;
        if acceptance_raised {
            task.acceptance_policy = next_acceptance;
            task.acceptance_policy_reason = next_acceptance_reason;
        }
    }
    if task.work_phase != WorkPhase::Implementation {
        return prepare_write_validation_error(
            WriteTicketField::TaskId,
            "write preparation requires work_phase=implementation",
        );
    }
    Ok(PrepareWritePolicyDecision {
        normalized,
        task,
        change_unit,
        reasons,
        workflow_policy,
        control_mutations,
    })
}

fn plan_prepare_write_mutations(
    store: &CoreProjectStore,
    policy: PrepareWritePolicyDecision,
) -> Result<PrepareWritePlanningOutcome, WriteTicketPlanningError> {
    let PrepareWritePolicyDecision {
        normalized,
        task,
        change_unit,
        mut reasons,
        workflow_policy,
        control_mutations,
    } = policy;
    let PrepareWriteNormalizedInput {
        raw,
        intended_operation: normalized_operation,
        intended_paths: normalized_paths,
        sensitive_categories: normalized_sensitive_categories,
    } = normalized;
    let PrepareWriteRawInput { input, plan_now } = raw;
    let task_id = input.task_id.clone();
    if input.product_file_write_intended == normalized_paths.is_empty() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::WriteCompatibility,
            WriteTicketDecisionCode::ProductWriteFlagMismatch,
            "product_file_write_intended must match the intended Product Repository paths.",
            Vec::new(),
        ));
    }

    let change_unit_record = WriteTicketRelatedRecord::CurrentChangeUnit {
        task_id: task_id.clone(),
        change_unit_id: ChangeUnitId::new(change_unit.change_unit_id.clone()),
    };
    if !workspace_context_matches(&change_unit, input.git_workspace_context.as_ref()) {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Workspace,
            WriteTicketDecisionCode::WorkspaceContextMismatch,
            "The current Git workspace does not match the Change Unit baseline context.",
            vec![change_unit_record.clone()],
        ));
    }
    if !baseline_matches(&change_unit, &task, &input.baseline_ref)? {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Baseline,
            WriteTicketDecisionCode::BaselineMismatch,
            "baseline_ref does not match the current write-compatibility basis.",
            vec![change_unit_record.clone()],
        ));
    }

    if !paths_match_current_change_unit(&normalized_paths, &change_unit) {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            WriteTicketDecisionCode::PathOutOfScope,
            "One or more intended paths are outside the current Change Unit path scope.",
            vec![change_unit_record.clone()],
        ));
    }

    if let Some(contract) = change_unit.effect_contract.as_ref() {
        let contract_violations = product_write_violations(
            contract,
            input.product_file_write_intended,
            &normalized_paths,
        )
        .map_err(|_| CorePipelineError::Invariant {
            detail: format!(
                "typed Change Unit `{}` has an internally inconsistent effect contract",
                change_unit.change_unit_id
            ),
        })?;
        for violation in contract_violations {
            reasons.push(effect_contract_reason(
                violation,
                change_unit_record.clone(),
            ));
        }
    }

    let current_change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let task_ref = state_ref(
        StateRecordKind::Task,
        task_id.as_str(),
        &input.project_id,
        Some(&task_id),
        None,
    );
    let operation_refs = vec![
        task_ref.clone(),
        state_ref(
            StateRecordKind::ChangeUnit,
            current_change_unit_id.as_str(),
            &input.project_id,
            Some(&task_id),
            None,
        ),
    ];
    let change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let typed_normalized_paths = typed_product_paths(&normalized_paths)?;
    let attempt_scope = WriteTicketAttemptScope {
        task_id: task_id.clone(),
        change_unit_id: change_unit_id.clone(),
        intended_operation: normalized_operation,
        intended_paths: typed_normalized_paths,
        product_file_write_intended: input.product_file_write_intended,
        sensitive_categories: normalized_sensitive_categories,
        baseline_ref: Some(input.baseline_ref.clone()),
    };
    let approval_requirement = WriteTicketApprovalRequirement::new(
        &input.project_id,
        task.scope_revision,
        task.effective_control_level,
        &attempt_scope,
        &plan_now,
    );
    let sensitive_requirement = approval_requirement
        .is_required()
        .then(|| approval_requirement.sensitive_requirement());
    let pending_authorities = pending_user_action_authorities(store, &task_id, &plan_now)?;
    let operation_context = UserActionOperationContext {
        operation: UserActionOperation::PrepareWrite,
        task_id: &task_id,
        change_unit_id: Some(&current_change_unit_id),
        scope_revision: task.scope_revision,
        close_basis: None,
        operation_refs: &operation_refs,
        sensitive_approval: sensitive_requirement.as_ref(),
    };
    let pending_user_action_request_ids = pending_authorities
        .iter()
        .filter(|authority| user_action_blocks_operation(authority, &operation_context))
        .map(|authority| authority.user_action_request_id.clone())
        .collect::<Vec<_>>();
    if !pending_user_action_request_ids.is_empty() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::UserAction,
            WriteTicketDecisionCode::UserActionUnresolved,
            "A user action required before write preparation remains unresolved.",
            pending_user_action_request_ids
                .iter()
                .cloned()
                .map(|request_id| WriteTicketRelatedRecord::UserActionRequest {
                    task_id: task_id.clone(),
                    request_id,
                })
                .collect(),
        ));
    }

    let approval_authorities = resolved_sensitive_approval_authorities(store, &task_id, &plan_now)?;
    let current_approvals =
        CurrentSensitiveApprovals::new(&approval_authorities, &approval_requirement);
    let approval_basis = current_approvals.primary_basis();
    let created_by_user_action_resolution_id = approval_basis
        .as_ref()
        .map(|basis| basis.first_resolution_id().clone());
    if approval_requirement.is_required() && approval_basis.is_none() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::SensitiveApproval,
            WriteTicketDecisionCode::SensitiveApprovalMissing,
            "A matching sensitive-action approval is required before write ticket issuance.",
            Vec::new(),
        ));
    }
    let created_at = plan_now.clone();
    let validity_basis_for_selection = WriteTicketValidityBasis {
        task_id: task_id.clone(),
        change_unit_id: change_unit_id.clone(),
        scope_revision: task.scope_revision,
        baseline_ref: Some(input.baseline_ref.clone()),
        workspace_context_sha256: input
            .git_workspace_context
            .as_ref()
            .map(volicord_types::canonical::canonical_json_bare_sha256)
            .transpose()
            .map_err(CorePipelineError::from)?,
        write_authority_fingerprint: workflow_policy.write_authority_fingerprint.clone(),
        approval_basis_refs: Vec::new(),
    };
    let active_ticket_selection = select_active_write_tickets(
        store,
        &input.project_id,
        &task,
        ActiveWriteTicketRequirements {
            validity_basis: &validity_basis_for_selection,
            attempt_scope: &attempt_scope,
            approval_requirement: &approval_requirement,
            approval_authorities: &approval_authorities,
        },
        &plan_now,
    )?;
    if active_ticket_selection.compatible.is_some() {
        reasons.retain(|reason| reason.code != WriteTicketDecisionCode::SensitiveApprovalMissing);
    }
    let decision = prepare_write_decision(&reasons);
    let allowed = reasons.is_empty();
    let compatible_ticket = allowed
        .then_some(active_ticket_selection.compatible)
        .flatten();
    let ticket = if let Some(reusable) = compatible_ticket {
        PrepareWriteTicketPlan::Reuse(reusable)
    } else {
        let denied_paths = denied_write_ticket_paths(&reasons, &attempt_scope.intended_paths);
        let allowed_paths = attempt_scope
            .intended_paths
            .iter()
            .filter(|path| !denied_paths.contains(path))
            .cloned()
            .collect::<Vec<_>>();
        let path_scope = write_ticket_path_scope(allowed_paths, denied_paths)?;
        if allowed {
            let idle_expires_at = workflow_policy
                .write_ticket_idle_timeout_minutes
                .map(|minutes| {
                    let minutes = i64::try_from(minutes).map_err(|_| {
                        CorePipelineError::Store(StoreError::InvalidInput {
                            detail:
                                "workflow write-ticket idle timeout is outside the supported range"
                                    .to_owned(),
                        })
                    })?;
                    plan_now.checked_add(Duration::minutes(minutes)).map_err(|_| {
                        CorePipelineError::Store(StoreError::InvalidInput {
                            detail: "derived write-ticket idle timeout exceeds the supported timestamp range"
                                .to_owned(),
                        })
                    })
                })
                .transpose()
                .map_err(WriteTicketPlanningError::Core)?;
            let metadata = json_object(json!({
                "verification_basis": input.verification_basis.clone()
            }))?;
            PrepareWriteTicketPlan::Issue(PlannedWriteTicketDraft {
                project_id: input.project_id.clone(),
                task_id: task_id.clone(),
                change_unit_id,
                scope_revision: task.scope_revision,
                baseline_ref: input.baseline_ref.clone(),
                workspace_context_sha256: validity_basis_for_selection.workspace_context_sha256,
                write_authority_fingerprint: workflow_policy.write_authority_fingerprint,
                approval_basis: approval_basis.clone(),
                path_scope,
                attempt_scope,
                created_by_actor_source: input.actor_source,
                created_by_user_action_resolution_id,
                idle_expires_at,
                created_at,
                metadata,
            })
        } else {
            PrepareWriteTicketPlan::NoTicket(WriteDecisionPathFacts::new(path_scope))
        }
    };
    let mut storage_mutations = control_mutations;
    for write_ticket_id in active_ticket_selection.stale_approval_ticket_ids {
        storage_mutations.push(CoreStorageMutation::WriteTicket(
            WriteTicketMutation::InvalidateById(WriteTicketByIdInvalidation {
                write_ticket_id,
                invalidation_reason: WriteTicketInvalidationReason::ApprovalBasisChanged,
            }),
        ));
    }
    for write_ticket_id in active_ticket_selection.stale_workspace_ticket_ids {
        storage_mutations.push(CoreStorageMutation::WriteTicket(
            WriteTicketMutation::InvalidateById(WriteTicketByIdInvalidation {
                write_ticket_id,
                invalidation_reason: WriteTicketInvalidationReason::WorkspaceChanged,
            }),
        ));
    }
    for write_ticket_id in active_ticket_selection.stale_policy_ticket_ids {
        storage_mutations.push(CoreStorageMutation::WriteTicket(
            WriteTicketMutation::InvalidateById(WriteTicketByIdInvalidation {
                write_ticket_id,
                invalidation_reason: WriteTicketInvalidationReason::ExplicitRevoke,
            }),
        ));
    }
    Ok(PrepareWritePlanningOutcome {
        common: PrepareWriteCommonFacts {
            task_id,
            task,
            change_unit,
            reasons,
            decision,
            pending_user_action_request_ids,
            approval_basis,
        },
        ticket,
        mutations: PrepareWriteMutationFacts { storage_mutations },
    })
}

fn prepare_write_validation_error<T>(
    field: WriteTicketField,
    message: &'static str,
) -> Result<T, WriteTicketPlanningError> {
    Err(WriteTicketPlanningError::Validation { field, message })
}

fn acceptance_policy_rank(policy: AcceptancePolicy) -> u8 {
    match policy {
        AcceptancePolicy::NotRequired => 0,
        AcceptancePolicy::PolicyDependent => 1,
        AcceptancePolicy::Required => 2,
    }
}

#[derive(Debug, Default)]
struct ActiveWriteTicketSelection {
    compatible: Option<ReusableStoredWriteTicket>,
    stale_approval_ticket_ids: Vec<String>,
    stale_workspace_ticket_ids: Vec<String>,
    stale_policy_ticket_ids: Vec<String>,
}

struct ActiveWriteTicketRequirements<'a> {
    validity_basis: &'a WriteTicketValidityBasis,
    attempt_scope: &'a WriteTicketAttemptScope,
    approval_requirement: &'a WriteTicketApprovalRequirement<'a>,
    approval_authorities: &'a [UserActionAuthority],
}

fn select_active_write_tickets(
    store: &CoreProjectStore,
    project_id: &ProjectId,
    task: &TaskRecord,
    requirements: ActiveWriteTicketRequirements<'_>,
    now: &UtcTimestamp,
) -> Result<ActiveWriteTicketSelection, WriteTicketPlanningError> {
    let required_basis = requirements.validity_basis;
    let required_write_authority_fingerprint = &required_basis.write_authority_fingerprint;
    let required_scope = requirements.attempt_scope;
    let mut selection = ActiveWriteTicketSelection::default();
    for record in store.active_write_tickets(&required_basis.task_id)? {
        let candidate =
            match pre_evaluate_stored_write_ticket(stored_write_ticket_facts(&record), now)
                .map_err(stored_ticket_state_error)?
            {
                StoredTicketPreEvaluation::Complete(_) => continue,
                StoredTicketPreEvaluation::NeedsCurrentFacts(candidate) => candidate,
            };
        let candidate_id = candidate.write_ticket_id().as_str().to_owned();
        let candidate_ticket = candidate.semantic_facts();
        let candidate_requirement = WriteTicketApprovalRequirement::new(
            project_id,
            task.scope_revision,
            task.effective_control_level,
            candidate_ticket.attempt_scope(),
            now,
        );
        let candidate_approvals = CurrentSensitiveApprovals::new(
            requirements.approval_authorities,
            &candidate_requirement,
        );
        let candidate_approval = assess_write_ticket_approval(
            &candidate_requirement,
            &candidate_approvals,
            &candidate_ticket.validity_basis().approval_basis_refs,
        );
        let current = WriteTicketCurrentFacts {
            task: WriteTicketTaskFacts {
                scope_revision: task.scope_revision,
                effective_control_level: task.effective_control_level,
                pending_policy_reevaluation: false,
            },
            workflow: WriteTicketWorkflowFacts {
                write_authority_fingerprint: required_write_authority_fingerprint.clone(),
            },
            sensitive_approvals: requirements.approval_authorities.to_vec(),
            observed_at: now.clone(),
        };
        let evaluated = evaluate_active_candidate(candidate, &current, candidate_approval);
        if matches!(
            &evaluated,
            ActiveStoredWriteTicketEvaluation::Invalidated(ticket)
                if matches!(
                    ticket.authority(),
                    WriteTicketAuthorityState::WriteAuthorityChanged
                        | WriteTicketAuthorityState::PendingPolicyReevaluation
                )
        ) {
            selection.stale_policy_ticket_ids.push(candidate_id);
            continue;
        }
        let basis = evaluated.semantic_facts().validity_basis();
        let scope = evaluated.semantic_facts().attempt_scope();
        if requirements.approval_requirement.is_required()
            && scope.intended_operation != required_scope.intended_operation
        {
            continue;
        }
        if basis.task_id != required_basis.task_id
            || basis.change_unit_id != required_basis.change_unit_id
            || basis.scope_revision != required_basis.scope_revision
            || basis.baseline_ref != required_basis.baseline_ref
        {
            continue;
        }
        if scope.task_id != required_scope.task_id
            || scope.change_unit_id != required_scope.change_unit_id
            || scope.product_file_write_intended != required_scope.product_file_write_intended
            || scope.baseline_ref != required_scope.baseline_ref
            || !category_set_for_reuse(&required_scope.sensitive_categories)
                .is_subset(&category_set_for_reuse(&scope.sensitive_categories))
        {
            continue;
        }
        let path_scope = evaluated.semantic_facts().path_scope();
        if !required_scope.intended_paths.iter().all(|path| {
            path_scope
                .allowed()
                .iter()
                .any(|prefix| path.is_within(prefix))
                && !path_scope
                    .denied()
                    .iter()
                    .any(|prefix| path.is_within(prefix))
        }) {
            continue;
        }
        if basis.workspace_context_sha256 != required_basis.workspace_context_sha256 {
            selection.stale_workspace_ticket_ids.push(candidate_id);
            continue;
        }
        match evaluated {
            ActiveStoredWriteTicketEvaluation::Reusable(ticket) => {
                if selection.compatible.is_none() {
                    selection.compatible = Some(ticket);
                }
            }
            ActiveStoredWriteTicketEvaluation::Invalidated(ticket)
                if ticket.invalidation() == WriteTicketInvalidationReason::ApprovalBasisChanged =>
            {
                selection
                    .stale_approval_ticket_ids
                    .push(ticket.write_ticket_id().as_str().to_owned());
            }
            ActiveStoredWriteTicketEvaluation::Invalidated(ticket) => {
                selection
                    .stale_policy_ticket_ids
                    .push(ticket.write_ticket_id().as_str().to_owned());
            }
        }
    }
    Ok(selection)
}

fn resolved_sensitive_approval_authorities(
    store: &CoreProjectStore,
    task_id: &TaskId,
    now: &UtcTimestamp,
) -> Result<Vec<UserActionAuthority>, WriteTicketPlanningError> {
    let records = store
        .resolved_user_action_records(task_id, UserActionKind::SensitiveApproval, now)
        .map_err(CorePipelineError::from)?;
    records
        .iter()
        .map(user_action_authority_from_record)
        .collect::<Result<Vec<_>, _>>()
        .map_err(WriteTicketPlanningError::from)
}

fn stored_ticket_state_error(error: StoredWriteTicketStateError) -> WriteTicketPlanningError {
    WriteTicketPlanningError::Core(CorePipelineError::Invariant {
        detail: format!(
            "Store-validated Write Ticket could not enter the Core stored type-state family: {error:?}"
        ),
    })
}

fn typed_product_paths(
    paths: &[String],
) -> Result<Vec<ProductRelativePath>, WriteTicketPlanningError> {
    paths
        .iter()
        .map(|path| {
            ProductRelativePath::parse(path).map_err(|error| {
                WriteTicketPlanningError::Core(CorePipelineError::Invariant {
                    detail: format!(
                        "a normalized Product Repository path became invalid before persistence: {error}"
                    ),
                })
            })
        })
        .collect()
}

fn canonical_write_authority_fingerprint(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn lowercase_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn json_object(value: Value) -> Result<JsonObject, WriteTicketPlanningError> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(CorePipelineError::Invariant {
            detail: "write-ticket planning expected a JSON object".to_owned(),
        }
        .into()),
    }
}

fn category_set_for_reuse(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn effect_contract_reason(
    violation: EffectContractViolation,
    change_unit_record: WriteTicketRelatedRecord,
) -> WriteTicketDecisionReason {
    match violation {
        EffectContractViolation::FileWriteForbidden => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            WriteTicketDecisionCode::EffectContractForbidsProductFileWrite,
            "The current Change Unit effect contract forbids product-file writes.",
            vec![change_unit_record],
        ),
        EffectContractViolation::FileWriteNotAllowed => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            WriteTicketDecisionCode::EffectContractEffectNotAllowed,
            "The current Change Unit effect contract does not allow product-file writes.",
            vec![change_unit_record],
        ),
        EffectContractViolation::PathNotAllowed => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            WriteTicketDecisionCode::EffectContractPathNotAllowed,
            "One or more intended paths are outside the current Change Unit effect contract allowed paths.",
            vec![change_unit_record],
        ),
    }
}

fn denied_write_ticket_paths(
    reasons: &[WriteTicketDecisionReason],
    normalized_paths: &[ProductRelativePath],
) -> Vec<ProductRelativePath> {
    let path_denied = reasons.iter().any(|reason| {
        matches!(
            reason.code,
            WriteTicketDecisionCode::PathOutOfScope
                | WriteTicketDecisionCode::EffectContractPathNotAllowed
                | WriteTicketDecisionCode::EffectContractForbidsProductFileWrite
                | WriteTicketDecisionCode::EffectContractEffectNotAllowed
        )
    });
    if path_denied {
        normalized_paths.to_vec()
    } else {
        Vec::new()
    }
}

fn write_ticket_path_scope(
    allowed: Vec<ProductRelativePath>,
    denied: Vec<ProductRelativePath>,
) -> Result<WriteTicketPathScope, WriteTicketPlanningError> {
    WriteTicketPathScope::new(allowed, denied).map_err(|error| {
        WriteTicketPlanningError::Invariant {
            detail: format!(
                "prepare-write path facts are internally inconsistent: {}",
                write_ticket_path_scope_error(error)
            ),
        }
    })
}

fn write_ticket_path_scope_error(error: WriteTicketPathScopeError) -> &'static str {
    match error {
        WriteTicketPathScopeError::DuplicateAllowedPath => "duplicate allowed path",
        WriteTicketPathScopeError::DuplicateDeniedPath => "duplicate denied path",
        WriteTicketPathScopeError::AllowedDeniedOverlap => "allowed/denied path overlap",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write_ticket::current_validity::{
        test_support::stored_evaluation, StoredWriteTicketEvaluation,
    };
    use volicord_types::{
        ids::BaselineRef,
        values::{ActorSource, WriteTicketStatus},
    };

    #[test]
    fn planned_ticket_derives_the_fully_typed_store_input() {
        let plan =
            valid_plan(WriteTicketId::new("write_ticket_planned")).expect("valid planned ticket");

        let insert = plan.persistence_input();
        let summary = crate::write_ticket::summary::project_planned_write_ticket_summary(
            crate::write_ticket::summary::PlannedWriteTicketSummaryInput {
                planned: &plan,
                state_version: 8,
                guarantee_display: None,
            },
        );

        assert_eq!(insert.write_ticket_id.as_str(), "write_ticket_planned");
        assert_eq!(insert.task_id, plan.validity_basis().task_id);
        assert_eq!(insert.change_unit_id, plan.validity_basis().change_unit_id);
        assert_eq!(insert.validity_basis, *plan.validity_basis());
        assert_eq!(insert.path_scope, *plan.path_scope());
        assert_eq!(insert.attempt_scope, *plan.attempt_scope());
        assert_eq!(insert.idle_expires_at.as_ref(), plan.idle_expires_at());
        let mutation = planned_write_ticket_mutation(&plan);
        let CoreStorageMutation::WriteTicket(WriteTicketMutation::Insert(mutation_insert)) =
            mutation
        else {
            panic!("a planned Write Ticket must materialize as exactly one insert");
        };
        assert_eq!(*mutation_insert, insert);
        assert_eq!(summary.status, WriteTicketStatus::Active);
        assert_eq!(
            summary
                .write_ticket_ref
                .as_ref()
                .map(|reference| reference.record_id.as_str()),
            Some("write_ticket_planned")
        );
        assert_eq!(summary.basis_state_version, Some(7));
        assert_eq!(summary.validity_basis.as_ref(), Some(plan.validity_basis()));
        assert_eq!(summary.idle_expires_at.as_ref(), plan.idle_expires_at());
        assert!(summary.observation_refs.is_empty());
    }

    #[test]
    fn semantic_draft_materializes_only_after_identity_is_supplied() {
        let draft = valid_draft();
        assert_eq!(draft.task_id.as_str(), "task_planned");
        assert_eq!(
            draft.attempt_scope.intended_paths,
            vec![ProductRelativePath::parse("src").expect("valid path")]
        );

        let plan = materialize_planned_write_ticket(
            draft,
            WriteTicketId::new("write_ticket_planned"),
            7,
            6,
        )
        .expect("identity completes materialization");
        assert_eq!(plan.write_ticket_id().as_str(), "write_ticket_planned");
        assert_eq!(plan.basis_state_version(), 7);
    }

    #[test]
    fn semantic_validation_error_contains_no_response_metadata() {
        assert!(matches!(
            prepare_write_validation_error::<()>(
                WriteTicketField::IntendedOperation,
                "intended_operation must not be empty",
            ),
            Err(WriteTicketPlanningError::Validation {
                field: WriteTicketField::IntendedOperation,
                message: "intended_operation must not be empty",
            })
        ));
    }

    #[test]
    fn planned_and_stored_tickets_share_only_immutable_semantic_facts() {
        let plan =
            valid_plan(WriteTicketId::new("write_ticket_shared")).expect("valid planned ticket");
        let planned = crate::write_ticket::semantic::planned_write_ticket_semantic_facts(&plan);
        let stored = crate::write_ticket::semantic::test_support::stored_facts_from_semantic(
            "write_ticket_shared",
            WriteTicketStatus::Active,
            planned.clone(),
        );

        assert_eq!(planned, *stored.semantic_facts());
        assert_eq!(plan.write_ticket_id(), stored.write_ticket_id());
    }

    #[test]
    fn planned_ticket_rejects_semantic_identity_disagreement() {
        let mut input = valid_input(WriteTicketId::new("write_ticket_planned"));
        input.attempt_scope.task_id = TaskId::new("task_other");

        assert_eq!(
            PlannedWriteTicket::new(input),
            Err(PlannedWriteTicketError::TaskIdentityMismatch)
        );
    }

    #[test]
    fn planned_ticket_rejects_duplicate_intended_paths() {
        let mut input = valid_input(WriteTicketId::new("write_ticket_planned"));
        input
            .attempt_scope
            .intended_paths
            .push(ProductRelativePath::parse("src").expect("valid path"));

        assert_eq!(
            PlannedWriteTicket::new(input),
            Err(PlannedWriteTicketError::DuplicateIntendedPaths)
        );
    }

    #[test]
    fn planning_ticket_behavior_is_one_closed_branch() {
        let reusable = match stored_evaluation("write_ticket_reused", WriteTicketStatus::Active, 6)
        {
            StoredWriteTicketEvaluation::Reusable(ticket) => ticket,
            _ => unreachable!("active test evaluation is reusable"),
        };
        let no_ticket_scope =
            WriteTicketPathScope::new(Vec::new(), Vec::new()).expect("empty decision scope");
        let branches = [
            PrepareWriteTicketPlan::Issue(valid_draft()),
            PrepareWriteTicketPlan::Reuse(reusable),
            PrepareWriteTicketPlan::NoTicket(WriteDecisionPathFacts::new(no_ticket_scope)),
        ];

        assert!(matches!(branches[0], PrepareWriteTicketPlan::Issue(_)));
        assert!(matches!(branches[1], PrepareWriteTicketPlan::Reuse(_)));
        assert!(matches!(branches[2], PrepareWriteTicketPlan::NoTicket(_)));
    }

    #[test]
    fn materialized_ticket_branches_preserve_identity_scope_and_persistence_effect() {
        let issued_plan =
            valid_plan(WriteTicketId::new("write_ticket_issued")).expect("issued plan");
        let issued_scope = issued_plan.path_scope().clone();
        let issued = MaterializedPrepareWriteTicket::Issued(issued_plan);

        let reusable = match stored_evaluation("write_ticket_reused", WriteTicketStatus::Active, 6)
        {
            StoredWriteTicketEvaluation::Reusable(ticket) => ticket,
            _ => unreachable!("active test evaluation is reusable"),
        };
        let reused_scope = reusable.path_scope().clone();
        let reused = MaterializedPrepareWriteTicket::Reused(reusable);

        let decision_scope =
            WriteTicketPathScope::new(Vec::new(), Vec::new()).expect("empty decision scope");
        let none = MaterializedPrepareWriteTicket::None(WriteDecisionPathFacts::new(
            decision_scope.clone(),
        ));

        assert_eq!(issued.write_ticket_effect(), WriteTicketEffect::Issued);
        assert_eq!(
            issued.write_ticket_id().map(WriteTicketId::as_str),
            Some("write_ticket_issued")
        );
        assert_eq!(issued.path_scope(), &issued_scope);
        assert!(issued.persistence_mutation().is_some());

        assert_eq!(reused.write_ticket_effect(), WriteTicketEffect::Reused);
        assert_eq!(
            reused.write_ticket_id().map(WriteTicketId::as_str),
            Some("write_ticket_reused")
        );
        assert_eq!(reused.path_scope(), &reused_scope);
        assert!(reused.persistence_mutation().is_none());

        assert_eq!(none.write_ticket_effect(), WriteTicketEffect::None);
        assert!(none.write_ticket_id().is_none());
        assert_eq!(none.path_scope(), &decision_scope);
        assert!(none.persistence_mutation().is_none());
    }

    fn valid_plan(
        write_ticket_id: WriteTicketId,
    ) -> Result<PlannedWriteTicket, PlannedWriteTicketError> {
        PlannedWriteTicket::new(valid_input(write_ticket_id))
    }

    fn valid_draft() -> PlannedWriteTicketDraft {
        let input = valid_input(WriteTicketId::new("write_ticket_planned"));
        PlannedWriteTicketDraft {
            project_id: input.project_id,
            task_id: input.validity_basis.task_id,
            change_unit_id: input.validity_basis.change_unit_id,
            scope_revision: input.validity_basis.scope_revision,
            baseline_ref: input
                .validity_basis
                .baseline_ref
                .expect("valid basis has baseline"),
            workspace_context_sha256: input.validity_basis.workspace_context_sha256,
            write_authority_fingerprint: input.validity_basis.write_authority_fingerprint,
            approval_basis: None,
            path_scope: input.path_scope,
            attempt_scope: input.attempt_scope,
            created_by_actor_source: input.created_by_actor_source,
            created_by_user_action_resolution_id: input.created_by_user_action_resolution_id,
            idle_expires_at: input.idle_expires_at,
            created_at: input.created_at,
            metadata: input.metadata,
        }
    }

    fn valid_input(write_ticket_id: WriteTicketId) -> PlannedWriteTicketInput {
        let task_id = TaskId::new("task_planned");
        let change_unit_id = ChangeUnitId::new("change_planned");
        let baseline_ref = BaselineRef::new("baseline_planned");
        let intended_path = ProductRelativePath::parse("src").expect("valid path");
        PlannedWriteTicketInput {
            project_id: ProjectId::new("project_planned"),
            write_ticket_id,
            basis_state_version: 7,
            validity_basis: WriteTicketValidityBasis {
                task_id: task_id.clone(),
                change_unit_id: change_unit_id.clone(),
                scope_revision: 3,
                baseline_ref: Some(baseline_ref.clone()),
                workspace_context_sha256: None,
                write_authority_fingerprint: format!("sha256:{}", "0".repeat(64)),
                approval_basis_refs: Vec::new(),
            },
            path_scope: WriteTicketPathScope::new(vec![intended_path.clone()], Vec::new())
                .expect("valid path scope"),
            attempt_scope: WriteTicketAttemptScope {
                task_id,
                change_unit_id,
                intended_operation: "edit".to_owned(),
                intended_paths: vec![intended_path],
                product_file_write_intended: true,
                sensitive_categories: Vec::new(),
                baseline_ref: Some(baseline_ref),
            },
            created_by_actor_source: ActorSource::System,
            created_by_user_action_resolution_id: None,
            idle_expires_at: Some(
                UtcTimestamp::parse("2026-07-29T00:15:00Z").expect("valid timestamp"),
            ),
            created_at: UtcTimestamp::parse("2026-07-29T00:00:00Z").expect("valid timestamp"),
            metadata: serde_json::Map::new(),
        }
    }
}
