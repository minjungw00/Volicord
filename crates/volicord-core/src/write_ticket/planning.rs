use crate::identity::allocate_write_ticket_id;
use crate::pipeline::{CorePipelineError, VerifiedInvocationContext};
use crate::policy::effect_contract::{product_write_violations, EffectContractViolation};
use crate::policy::workflow::{
    acceptance_policy_for_control, project_workflow_policy, resolve_task_control_authority,
    ProjectWorkflowPolicy,
};
use crate::product_path::{observe_product_paths, ProductPathValidationError};
use crate::projection::guarantee_display_for_invocation;
use crate::record_refs::{change_unit_ref, state_ref};
use crate::write_ticket::{
    baseline_matches, change_unit_effect_contract, matching_sensitive_approval,
    paths_match_current_change_unit, resolve_prepare_write_task,
    validate_prepare_write_change_unit, workspace_context_matches, SensitiveApprovalSearch,
};
use crate::write_ticket::{
    normalized_string_set, prepare_write_decision, write_decision_reason,
    write_ticket_is_idle_expired,
};
use chrono::Duration;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use volicord_platform_fs::PlatformDiagnosticClass;
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, CoreStorageMutation, ProjectStateHeader,
    TaskControlLevelUpdate, TaskMutation, TaskRecord, WriteTicketByIdInvalidation,
    WriteTicketInsert, WriteTicketMutation, WriteTicketRecord,
};
use volicord_store::diagnostics::{
    record_core_rejection_diagnostic, CoreRejectionDiagnostic, CoreRejectionReason,
};
use volicord_store::error::StoreError;
use volicord_types::ids::{ChangeUnitId, DurableIdGenerator, TaskId, WriteTicketId};
use volicord_types::methods::PrepareWriteRequest;
use volicord_types::product_path::{path_is_within, ProductRelativePath};
use volicord_types::schema::{
    GuaranteeDisplay, JsonObject, StateRecordRef, WriteDecisionReason, WriteTicketAttemptScope,
    WriteTicketValidityBasis,
};
use volicord_types::values::{
    AcceptancePolicy, MethodName, PrepareWriteDecision, StateRecordKind, TaskControlLevel,
    TaskMode, UserActionKind, UserActionRequiredFor, UtcTimestamp, WorkPhase,
    WriteDecisionCategory, WriteTicketEffect, WriteTicketInvalidationReason, WriteTicketStatus,
};
use volicord_user_action_service::{
    current_sensitive_approval, pending_user_action_authorities, user_action_authority_from_record,
    user_action_blocks_operation, SensitiveApprovalRequirement, UserActionOperation,
    UserActionOperationContext,
};

use super::WriteTicketPlanningError;
struct PrepareWriteRawRequest {
    request: PrepareWriteRequest,
    plan_now: UtcTimestamp,
}

impl PrepareWriteRawRequest {
    fn new(request: PrepareWriteRequest, operation_now: &UtcTimestamp) -> Self {
        Self {
            request,
            plan_now: operation_now.clone(),
        }
    }
}

struct PrepareWriteNormalizedRequest {
    raw: PrepareWriteRawRequest,
    planned_state_version: u64,
    intended_operation: String,
    intended_paths: Vec<String>,
    sensitive_categories: Vec<String>,
}

struct PrepareWriteResolvedContext {
    normalized: PrepareWriteNormalizedRequest,
    task_id: TaskId,
    task: TaskRecord,
    change_unit: ChangeUnitRecord,
    reasons: Vec<WriteDecisionReason>,
}

struct PrepareWritePolicyDecision {
    normalized: PrepareWriteNormalizedRequest,
    task_id: TaskId,
    task: TaskRecord,
    change_unit: ChangeUnitRecord,
    reasons: Vec<WriteDecisionReason>,
    workflow_policy: ProjectWorkflowPolicy,
    sensitive_approval_required: bool,
    control_mutations: Vec<CoreStorageMutation>,
}

pub(crate) struct PrepareWritePlannedMutations {
    pub(crate) request: PrepareWriteRequest,
    pub(crate) planned_state_version: u64,
    pub(crate) plan_now: UtcTimestamp,
    pub(crate) task_id: TaskId,
    pub(crate) task: TaskRecord,
    pub(crate) change_unit: ChangeUnitRecord,
    pub(crate) reasons: Vec<WriteDecisionReason>,
    pub(crate) decision: PrepareWriteDecision,
    pub(crate) allowed: bool,
    pub(crate) pending_user_action_refs: Vec<StateRecordRef>,
    pub(crate) active_user_action_refs: Vec<StateRecordRef>,
    pub(crate) guarantee_display: Option<GuaranteeDisplay>,
    pub(crate) write_ticket_id: Option<WriteTicketId>,
    pub(crate) write_ticket_ref: Option<StateRecordRef>,
    pub(crate) planned_write_ticket_record: Option<WriteTicketRecord>,
    pub(crate) idle_expires_at: Option<UtcTimestamp>,
    pub(crate) write_ticket_effect: WriteTicketEffect,
    pub(crate) allowed_path_patterns: Vec<String>,
    pub(crate) denied_path_patterns: Vec<String>,
    pub(crate) storage_mutations: Vec<CoreStorageMutation>,
}

pub(crate) fn plan_prepare_write(
    id_generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: PrepareWriteRequest,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<PrepareWritePlannedMutations, WriteTicketPlanningError> {
    let raw = PrepareWriteRawRequest::new(request, operation_now);
    let normalized = normalize_prepare_write_request(store, project_state, raw)?;
    let resolved = resolve_prepare_write_context(store, project_state, normalized)?;
    let policy = decide_prepare_write_policy(store, project_state, resolved)?;
    plan_prepare_write_mutations(
        id_generator,
        store,
        project_state,
        verified_invocation,
        policy,
    )
}

fn resolve_prepare_write_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    normalized: PrepareWriteNormalizedRequest,
) -> Result<PrepareWriteResolvedContext, WriteTicketPlanningError> {
    let request = &normalized.raw.request;
    let (task_id, task, mut reasons) = resolve_prepare_write_task(store, project_state, request)?;
    let change_unit = match store
        .current_change_unit(&task_id)
        .map_err(CorePipelineError::from)?
    {
        Some(change_unit) => change_unit,
        None => {
            let _ = record_core_rejection_diagnostic(
                store
                    .mutation_context()
                    .expect("prepare_write planning retains a mutation context"),
                CoreRejectionDiagnostic {
                    project_id: request.envelope.project_id.as_str(),
                    task_id: task_id.as_str(),
                    method_name: MethodName::PrepareWrite,
                    reason: CoreRejectionReason::CurrentChangeUnitRequired,
                    occurred_at: &normalized.raw.plan_now,
                },
            );
            return Err(WriteTicketPlanningError::CurrentChangeUnitRequired { task_id });
        }
    };
    validate_prepare_write_change_unit(request, &task_id, &change_unit, &mut reasons);

    Ok(PrepareWriteResolvedContext {
        normalized,
        task_id,
        task,
        change_unit,
        reasons,
    })
}

fn normalize_prepare_write_request(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    raw: PrepareWriteRawRequest,
) -> Result<PrepareWriteNormalizedRequest, WriteTicketPlanningError> {
    if raw.request.intended_operation.trim().is_empty() {
        return prepare_write_validation_error(
            raw.request.envelope.dry_run,
            project_state.state_version,
            "intended_operation",
            "intended_operation must not be empty",
        );
    }
    let intended_operation = raw.request.intended_operation.trim().to_owned();
    let sensitive_categories = normalized_string_set(&raw.request.sensitive_categories);
    let intended_paths = match observe_product_paths(
        &store.project_record().repo_root,
        &raw.request.intended_paths,
    ) {
        Ok(paths) => paths,
        Err(ProductPathValidationError::Lexical(_)) => {
            return Err(WriteTicketPlanningError::Validation {
                field: "intended_paths",
                message: "intended_paths must be normalized relative Product Repository paths",
            })
        }
        Err(error @ ProductPathValidationError::Platform(_))
            if error.platform_class() == Some(PlatformDiagnosticClass::Rejected) =>
        {
            return Err(WriteTicketPlanningError::ProductPathContainment {
                field: "intended_paths",
                message: "intended_paths resolve outside the Product Repository",
            })
        }
        Err(ProductPathValidationError::Platform(error)) => {
            return Err(CorePipelineError::from(error).into())
        }
    };
    Ok(PrepareWriteNormalizedRequest {
        raw,
        planned_state_version: project_state.state_version + 1,
        intended_operation,
        intended_paths,
        sensitive_categories,
    })
}

fn decide_prepare_write_policy(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    resolved: PrepareWriteResolvedContext,
) -> Result<PrepareWritePolicyDecision, WriteTicketPlanningError> {
    let PrepareWriteResolvedContext {
        normalized,
        task_id,
        mut task,
        change_unit,
        reasons,
    } = resolved;
    let dry_run = normalized.raw.request.envelope.dry_run;
    if task.mode == TaskMode::Advisor {
        return prepare_write_validation_error(
            dry_run,
            project_state.state_version,
            "task_id",
            "advisor Task mode does not support write preparation",
        );
    }
    let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;
    let current_control = task.effective_control_level;
    let current_acceptance = task.acceptance_policy;
    let resolved_control =
        resolve_task_control_authority(&task, &workflow_policy).map_err(CorePipelineError::from)?;
    let resolved_base_control = resolved_control.effective_control_level;
    let mut next_control = resolved_base_control;
    if next_control == TaskControlLevel::Observe {
        return prepare_write_validation_error(
            dry_run,
            project_state.state_version,
            "task_id",
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
            dry_run,
            project_state.state_version,
            "task_id",
            "write preparation requires work_phase=implementation",
        );
    }
    Ok(PrepareWritePolicyDecision {
        normalized,
        task_id,
        task,
        change_unit,
        reasons,
        workflow_policy,
        sensitive_approval_required: next_control == TaskControlLevel::Sensitive,
        control_mutations,
    })
}

fn plan_prepare_write_mutations(
    id_generator: &dyn DurableIdGenerator,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    policy: PrepareWritePolicyDecision,
) -> Result<PrepareWritePlannedMutations, WriteTicketPlanningError> {
    let PrepareWritePolicyDecision {
        normalized,
        task_id,
        task,
        change_unit,
        mut reasons,
        workflow_policy,
        sensitive_approval_required,
        control_mutations,
    } = policy;
    let PrepareWriteNormalizedRequest {
        raw,
        planned_state_version,
        intended_operation: normalized_operation,
        intended_paths: normalized_paths,
        sensitive_categories: normalized_sensitive_categories,
    } = normalized;
    let PrepareWriteRawRequest { request, plan_now } = raw;
    if request.product_file_write_intended == normalized_paths.is_empty() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::WriteCompatibility,
            "product_write_flag_mismatch",
            "product_file_write_intended must match the intended Product Repository paths.",
            Vec::new(),
        ));
    }

    if !workspace_context_matches(&change_unit, verified_invocation)? {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Workspace,
            "workspace_context_mismatch",
            "The current Git workspace does not match the Change Unit baseline context.",
            vec![change_unit_ref(
                &request.envelope.project_id,
                &task_id,
                &change_unit,
                project_state.state_version,
            )],
        ));
    }
    if !baseline_matches(&change_unit, &task, &request.baseline_ref)? {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Baseline,
            "baseline_mismatch",
            "baseline_ref does not match the current write-compatibility basis.",
            vec![change_unit_ref(
                &request.envelope.project_id,
                &task_id,
                &change_unit,
                project_state.state_version,
            )],
        ));
    }

    if !paths_match_current_change_unit(&normalized_paths, &change_unit)? {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            "path_out_of_scope",
            "One or more intended paths are outside the current Change Unit path scope.",
            vec![change_unit_ref(
                &request.envelope.project_id,
                &task_id,
                &change_unit,
                project_state.state_version,
            )],
        ));
    }

    if let Some(contract) = change_unit_effect_contract(&change_unit)? {
        let contract_violations = product_write_violations(
            &contract,
            request.product_file_write_intended,
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
                change_unit_ref(
                    &request.envelope.project_id,
                    &task_id,
                    &change_unit,
                    project_state.state_version,
                ),
            ));
        }
    }

    let current_change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let task_ref = state_ref(
        StateRecordKind::Task,
        task_id.as_str(),
        &request.envelope.project_id,
        Some(&task_id),
        Some(project_state.state_version),
    );
    let operation_refs = vec![
        task_ref.clone(),
        change_unit_ref(
            &request.envelope.project_id,
            &task_id,
            &change_unit,
            project_state.state_version,
        ),
    ];
    let sensitive_requirement = if !sensitive_approval_required {
        None
    } else {
        Some(SensitiveApprovalRequirement {
            task_id: &task_id,
            change_unit_id: &current_change_unit_id,
            scope_revision: task.scope_revision,
            operation: &normalized_operation,
            normalized_paths: &normalized_paths,
            sensitive_categories: &normalized_sensitive_categories,
            baseline_ref: Some(&request.baseline_ref),
            required_for: UserActionRequiredFor::PrepareWrite,
            now: &plan_now,
        })
    };
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
    let pending_user_action_refs = pending_authorities
        .iter()
        .filter(|authority| user_action_blocks_operation(authority, &operation_context))
        .map(|authority| {
            state_ref(
                StateRecordKind::UserActionRequest,
                &authority.user_action_request_id,
                &request.envelope.project_id,
                Some(&task_id),
                Some(project_state.state_version),
            )
        })
        .collect::<Vec<_>>();
    if !pending_user_action_refs.is_empty() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::UserAction,
            "user_action_unresolved",
            "A user action required before write preparation remains unresolved.",
            pending_user_action_refs.clone(),
        ));
    }

    let mut active_user_action_refs = Vec::new();
    let mut created_by_user_action_resolution_id = None;
    if sensitive_approval_required {
        let matching_sensitive_approval = matching_sensitive_approval(SensitiveApprovalSearch {
            store,
            request: &request,
            task_id: &task_id,
            task: &task,
            change_unit: &change_unit,
            intended_operation: &normalized_operation,
            normalized_paths: &normalized_paths,
            sensitive_categories: &normalized_sensitive_categories,
            now: &plan_now,
        })?;
        if let Some(record) = matching_sensitive_approval {
            if let Some(resolution) = record.resolution() {
                created_by_user_action_resolution_id =
                    Some(resolution.user_action_resolution_id().to_owned());
                active_user_action_refs.push(state_ref(
                    StateRecordKind::UserActionResolution,
                    resolution.user_action_resolution_id(),
                    &request.envelope.project_id,
                    Some(&task_id),
                    Some(project_state.state_version),
                ));
            }
        } else {
            reasons.push(write_decision_reason(
                WriteDecisionCategory::SensitiveApproval,
                "sensitive_approval_missing",
                "A matching sensitive-action approval is required before write ticket issuance.",
                Vec::new(),
            ));
        }
    }

    let guarantee_display = Some(guarantee_display_for_invocation(
        store,
        verified_invocation,
        planned_state_version,
    )?);
    let change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let typed_normalized_paths = typed_product_paths(&normalized_paths)?;
    let attempt_scope = WriteTicketAttemptScope {
        task_id: task_id.clone(),
        change_unit_id: change_unit_id.clone(),
        intended_operation: normalized_operation,
        intended_paths: typed_normalized_paths,
        product_file_write_intended: request.product_file_write_intended,
        sensitive_categories: normalized_sensitive_categories,
        baseline_ref: Some(request.baseline_ref.clone()),
    };
    let created_at = plan_now.clone();
    let validity_basis = WriteTicketValidityBasis {
        task_id: task_id.clone(),
        change_unit_id: change_unit_id.clone(),
        scope_revision: task.scope_revision,
        baseline_ref: Some(request.baseline_ref.clone()),
        workspace_context_sha256: verified_invocation
            .git_workspace_context
            .as_ref()
            .map(volicord_types::canonical::canonical_json_bare_sha256)
            .transpose()
            .map_err(CorePipelineError::from)?,
        write_authority_fingerprint: workflow_policy.write_authority_fingerprint.clone(),
        approval_basis_refs: active_user_action_refs.clone(),
    };
    let active_ticket_selection = select_active_write_tickets(
        store,
        project_state,
        &request,
        &task,
        ActiveWriteTicketRequirements {
            validity_basis: &validity_basis,
            attempt_scope: &attempt_scope,
            sensitive_approval_required,
        },
        &plan_now,
    )?;
    if active_ticket_selection.compatible.is_some() {
        reasons.retain(|reason| reason.code != "sensitive_approval_missing");
    }
    let decision = prepare_write_decision(&reasons);
    let allowed = reasons.is_empty();
    let compatible_ticket = allowed
        .then_some(active_ticket_selection.compatible)
        .flatten();
    let reuse_write_ticket =
        compatible_ticket.is_some() && request.envelope.dry_run.is_not_requested();
    let issue_write_ticket =
        allowed && compatible_ticket.is_none() && request.envelope.dry_run.is_not_requested();
    let write_ticket_id = if let Some(record) = compatible_ticket.as_ref() {
        (request.envelope.dry_run.is_not_requested())
            .then(|| WriteTicketId::new(record.write_ticket_id.clone()))
    } else if issue_write_ticket {
        Some(allocate_write_ticket_id(id_generator, store)?)
    } else {
        None
    };
    let idle_expires_at_timestamp = if issue_write_ticket {
        workflow_policy
            .write_ticket_idle_timeout_minutes
            .map(|minutes| {
                let minutes = i64::try_from(minutes).map_err(|_| {
                    CorePipelineError::Store(StoreError::InvalidInput {
                        detail: "workflow write-ticket idle timeout is outside the supported range"
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
            .map_err(WriteTicketPlanningError::Core)?
    } else {
        compatible_ticket
            .as_ref()
            .and_then(|record| record.idle_expires_at.clone())
    };
    let write_ticket_ref = write_ticket_id.as_ref().map(|write_ticket_id| {
        state_ref(
            StateRecordKind::WriteTicket,
            write_ticket_id.as_str(),
            &request.envelope.project_id,
            Some(&task_id),
            Some(planned_state_version),
        )
    });
    let denied_path_patterns = if let Some(record) = compatible_ticket.as_ref() {
        write_ticket_path_prefix_strings(record, false)?
    } else {
        denied_write_ticket_paths(&reasons, &normalized_paths)
    };
    let allowed_path_patterns = if let Some(record) = compatible_ticket.as_ref() {
        write_ticket_path_prefix_strings(record, true)?
    } else {
        normalized_paths
            .iter()
            .filter(|path| !denied_path_patterns.iter().any(|denied| denied == *path))
            .cloned()
            .collect::<Vec<_>>()
    };
    let typed_allowed_path_patterns = typed_product_paths(&allowed_path_patterns)?;
    let typed_denied_path_patterns = typed_product_paths(&denied_path_patterns)?;
    let planned_write_ticket_record = if let Some(record) = compatible_ticket.as_ref() {
        (request.envelope.dry_run.is_not_requested()).then(|| record.clone())
    } else {
        write_ticket_id
            .as_ref()
            .map(|write_ticket_id| WriteTicketRecord {
                project_id: request.envelope.project_id.as_str().to_owned(),
                write_ticket_id: write_ticket_id.as_str().to_owned(),
                task_id: task_id.as_str().to_owned(),
                change_unit_id: change_unit_id.as_str().to_owned(),
                basis_state_version: planned_state_version,
                status: WriteTicketStatus::Active,
                validity_basis: validity_basis.clone(),
                allowed_path_prefixes: typed_allowed_path_patterns.clone(),
                denied_path_prefixes: typed_denied_path_patterns.clone(),
                attempt_scope: attempt_scope.clone(),
                idle_expires_at: idle_expires_at_timestamp.clone(),
                invalidation_reason: None,
                created_at: created_at.clone(),
                consumed_by_run_id: None,
                consumed_at: None,
            })
    };

    let write_ticket_effect = if reuse_write_ticket {
        WriteTicketEffect::Reused
    } else if issue_write_ticket {
        WriteTicketEffect::Issued
    } else {
        WriteTicketEffect::None
    };
    let mut storage_mutations = control_mutations;
    if request.envelope.dry_run.is_not_requested() {
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
    }
    if write_ticket_effect == WriteTicketEffect::Issued {
        let write_ticket_id = write_ticket_id
            .as_ref()
            .expect("new ticket issuance has an allocated ID");
        storage_mutations.push(CoreStorageMutation::WriteTicket(
            WriteTicketMutation::insert(WriteTicketInsert {
                write_ticket_id: write_ticket_id.as_str().to_owned(),
                task_id: task_id.as_str().to_owned(),
                change_unit_id: change_unit_id.as_str().to_owned(),
                validity_basis,
                allowed_path_prefixes: typed_allowed_path_patterns,
                denied_path_prefixes: typed_denied_path_patterns,
                attempt_scope,
                created_by_actor_source: verified_invocation.actor_source.clone(),
                created_by_user_action_resolution_id,
                idle_expires_at: idle_expires_at_timestamp.clone(),
                created_at,
                metadata: json_object(json!({
                    "verification_basis": verified_invocation.verification_basis.clone()
                }))?,
            }),
        ));
    }
    Ok(PrepareWritePlannedMutations {
        request,
        planned_state_version,
        plan_now,
        task_id,
        task,
        change_unit,
        reasons,
        decision,
        allowed,
        pending_user_action_refs,
        active_user_action_refs,
        guarantee_display,
        write_ticket_id,
        write_ticket_ref,
        planned_write_ticket_record,
        idle_expires_at: idle_expires_at_timestamp,
        write_ticket_effect,
        allowed_path_patterns,
        denied_path_patterns,
        storage_mutations,
    })
}

fn prepare_write_validation_error<T>(
    _dry_run: volicord_types::schema::DryRunIntent,
    _state_version: u64,
    field: &'static str,
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
    compatible: Option<WriteTicketRecord>,
    stale_approval_ticket_ids: Vec<String>,
    stale_workspace_ticket_ids: Vec<String>,
    stale_policy_ticket_ids: Vec<String>,
}

struct ActiveWriteTicketRequirements<'a> {
    validity_basis: &'a WriteTicketValidityBasis,
    attempt_scope: &'a WriteTicketAttemptScope,
    sensitive_approval_required: bool,
}

fn select_active_write_tickets(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &PrepareWriteRequest,
    task: &TaskRecord,
    requirements: ActiveWriteTicketRequirements<'_>,
    now: &UtcTimestamp,
) -> Result<ActiveWriteTicketSelection, WriteTicketPlanningError> {
    let required_basis = requirements.validity_basis;
    let required_write_authority_fingerprint = &required_basis.write_authority_fingerprint;
    let required_scope = requirements.attempt_scope;
    let mut selection = ActiveWriteTicketSelection::default();
    for record in store
        .active_write_tickets(&required_basis.task_id)
        .map_err(CorePipelineError::from)?
    {
        if write_ticket_is_idle_expired(&record, *now.as_datetime())
            .map_err(CorePipelineError::from)?
        {
            continue;
        }
        let basis = &record.validity_basis;
        if basis.write_authority_fingerprint != *required_write_authority_fingerprint {
            selection
                .stale_policy_ticket_ids
                .push(record.write_ticket_id);
            continue;
        }
        let scope = &record.attempt_scope;
        if requirements.sensitive_approval_required
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
        let allowed = write_ticket_path_prefix_strings(&record, true)?;
        let denied = write_ticket_path_prefix_strings(&record, false)?;
        if !required_scope.intended_paths.iter().all(|path| {
            allowed
                .iter()
                .any(|prefix| path_is_within(path.as_str(), prefix))
                && !denied
                    .iter()
                    .any(|prefix| path_is_within(path.as_str(), prefix))
        }) {
            continue;
        }
        if basis.workspace_context_sha256 != required_basis.workspace_context_sha256 {
            selection
                .stale_workspace_ticket_ids
                .push(record.write_ticket_id);
            continue;
        }
        if !write_ticket_approval_basis_is_current_for_prepare(
            store,
            project_state,
            request,
            task,
            scope,
            basis,
            now,
        )? {
            selection
                .stale_approval_ticket_ids
                .push(record.write_ticket_id);
            continue;
        }
        if requirements.sensitive_approval_required
            && (required_basis.approval_basis_refs.is_empty()
                || basis.approval_basis_refs.is_empty()
                || !approval_basis_identity_matches(
                    &required_basis.approval_basis_refs,
                    &basis.approval_basis_refs,
                ))
        {
            continue;
        }
        if selection.compatible.is_none() {
            selection.compatible = Some(record);
        }
    }
    Ok(selection)
}

fn write_ticket_approval_basis_is_current_for_prepare(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &PrepareWriteRequest,
    task: &TaskRecord,
    scope: &WriteTicketAttemptScope,
    validity_basis: &WriteTicketValidityBasis,
    now: &UtcTimestamp,
) -> Result<bool, WriteTicketPlanningError> {
    if validity_basis.approval_basis_refs.is_empty() {
        return Ok(scope.sensitive_categories.is_empty());
    }

    let normalized_scope_paths = scope
        .intended_paths
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect::<Vec<_>>();
    let requirement = SensitiveApprovalRequirement {
        task_id: &validity_basis.task_id,
        change_unit_id: &validity_basis.change_unit_id,
        scope_revision: task.scope_revision,
        operation: &scope.intended_operation,
        normalized_paths: &normalized_scope_paths,
        sensitive_categories: &scope.sensitive_categories,
        baseline_ref: scope.baseline_ref.as_ref(),
        required_for: UserActionRequiredFor::PrepareWrite,
        now,
    };
    let records = store
        .resolved_user_action_records(
            &validity_basis.task_id,
            UserActionKind::SensitiveApproval,
            now,
        )
        .map_err(CorePipelineError::from)?;
    let mut current_resolution_refs = Vec::new();
    for record in records {
        let authority = user_action_authority_from_record(&record)?;
        if current_sensitive_approval(&authority, &requirement) {
            if let Some(resolution_id) = authority.user_action_resolution_id {
                current_resolution_refs.push(state_ref(
                    StateRecordKind::UserActionResolution,
                    &resolution_id,
                    &request.envelope.project_id,
                    Some(&validity_basis.task_id),
                    Some(project_state.state_version),
                ));
            }
        }
    }

    Ok(!current_resolution_refs.is_empty()
        && validity_basis.approval_basis_refs.iter().all(|stored| {
            current_resolution_refs
                .iter()
                .any(|current| state_ref_identity_matches(stored, current))
        }))
}

fn approval_basis_identity_matches(left: &[StateRecordRef], right: &[StateRecordRef]) -> bool {
    left.len() == right.len()
        && left.iter().all(|reference| {
            right
                .iter()
                .any(|candidate| state_ref_identity_matches(reference, candidate))
        })
}

fn state_ref_identity_matches(left: &StateRecordRef, right: &StateRecordRef) -> bool {
    left.record_kind == right.record_kind
        && left.record_id == right.record_id
        && left.project_id == right.project_id
        && left.task_id == right.task_id
}

fn write_ticket_path_prefix_strings(
    record: &WriteTicketRecord,
    allowed: bool,
) -> Result<Vec<String>, WriteTicketPlanningError> {
    let paths = if allowed {
        &record.allowed_path_prefixes
    } else {
        &record.denied_path_prefixes
    };
    Ok(paths.iter().map(|path| path.as_str().to_owned()).collect())
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
    change_unit_ref: StateRecordRef,
) -> WriteDecisionReason {
    match violation {
        EffectContractViolation::FileWriteForbidden => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            "effect_contract_forbids_product_file_write",
            "The current Change Unit effect contract forbids product-file writes.",
            vec![change_unit_ref],
        ),
        EffectContractViolation::FileWriteNotAllowed => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            "effect_contract_effect_not_allowed",
            "The current Change Unit effect contract does not allow product-file writes.",
            vec![change_unit_ref],
        ),
        EffectContractViolation::PathNotAllowed => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            "effect_contract_path_not_allowed",
            "One or more intended paths are outside the current Change Unit effect contract allowed paths.",
            vec![change_unit_ref],
        ),
    }
}

fn denied_write_ticket_paths(
    reasons: &[WriteDecisionReason],
    normalized_paths: &[String],
) -> Vec<String> {
    let path_denied = reasons.iter().any(|reason| {
        matches!(
            reason.code.as_str(),
            "path_out_of_scope"
                | "effect_contract_path_not_allowed"
                | "effect_contract_forbids_product_file_write"
                | "effect_contract_effect_not_allowed"
        )
    });
    if path_denied {
        normalized_paths.to_vec()
    } else {
        Vec::new()
    }
}
