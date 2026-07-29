use super::blockers::close_blocker;
use super::change_control::task_ref_for_close;
use super::facts::{workflow_policy_for_close_context, CloseReadinessFacts};
use super::guidance::{close_guidance, CloseGuidance};
use super::service::CloseReadinessRequest;
use super::CloseReadinessError;
use crate::pipeline::{CorePipelineError, CoreResult};
use crate::policy::close_readiness::{
    current_final_acceptance, current_residual_risk_acceptance_coverage,
    final_acceptance_requirement,
};
use crate::record_refs::{change_unit_ref, state_ref, stored_refs_to_state_refs};
use volicord_store::core_pipeline::{CoreProjectStore, ProjectStateHeader};
use volicord_types::ids::{BaselineRef, ChangeUnitId};
use volicord_types::product_path::{path_is_within, paths_are_authorized};
use volicord_types::schema::{CloseReadinessBlocker, RiskAcceptanceCoverage, StateRecordRef};
use volicord_types::values::{
    AcceptancePolicy, ActorSource, CloseIntent, CloseReadinessBlockerCategory, JudgmentKind,
    JudgmentResolutionOutcome, StateRecordKind, TaskControlLevel, UserActionKind,
    UserActionRequiredFor, UtcTimestamp, WriteTicketStatus,
};
use volicord_user_action_service::{
    current_cancellation_authority, current_sensitive_approval, pending_user_action_authorities,
    resolved_user_action_authorities, user_action_blocks_operation, user_action_has_current_basis,
    user_action_required_for, verified_user_channel_provenance, CancellationAuthorityRequirement,
    SensitiveApprovalRequirement, UserActionAuthority, UserActionOperation,
    UserActionOperationContext,
};

pub(super) fn terminal_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
    now: &UtcTimestamp,
) -> Result<Vec<CloseReadinessBlocker>, CloseReadinessError> {
    match request.intent {
        CloseIntent::Cancel => {
            Ok(
                cancellation_authority_blocker(store, project_state, request, context)?
                    .into_iter()
                    .collect(),
            )
        }
        CloseIntent::Supersede => {
            let pending_refs = pending_user_action_refs_for_close_operation(
                store,
                project_state,
                request,
                context,
                UserActionOperation::CloseSupersede,
                now,
            )?;
            if pending_refs.is_empty() {
                return Ok(Vec::new());
            }
            Ok(vec![close_blocker(
                CloseReadinessBlockerCategory::PendingUserAction,
                "pending_user_action",
                "A user action required before superseding this Task is still pending.",
                pending_refs,
                vec![close_guidance(
                    CloseGuidance::ResolvePendingUserAction,
                    Vec::new(),
                )],
            )])
        }
        CloseIntent::Check | CloseIntent::Complete => Ok(Vec::new()),
    }
}

pub(super) fn completion_authority_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
    now: &UtcTimestamp,
) -> Result<Vec<CloseReadinessBlocker>, CloseReadinessError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);
    let change_unit_ref = context.current_change_unit.as_ref().map(|record| {
        change_unit_ref(
            &request.project_id,
            &request.task_id,
            record,
            project_state.state_version,
        )
    });

    let close_complete_pending_refs = pending_user_action_refs_for_close_operation(
        store,
        project_state,
        request,
        context,
        UserActionOperation::CloseComplete,
        now,
    )?;
    if !close_complete_pending_refs.is_empty() {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::PendingUserAction,
            "pending_user_action",
            "A user action required before close is still pending.",
            close_complete_pending_refs,
            vec![close_guidance(
                CloseGuidance::ResolvePendingUserAction,
                Vec::new(),
            )],
        ));
    }

    if sensitive_action_basis_missing(context)? {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::SensitiveApproval,
            "missing_sensitive_action_basis",
            "The effective sensitive Task has no ticket-backed sensitive-action basis for close.",
            change_unit_ref
                .clone()
                .into_iter()
                .chain(std::iter::once(task_ref.clone()))
                .collect(),
            vec![close_guidance(
                CloseGuidance::PrepareSensitiveAction,
                vec![task_ref.clone()],
            )],
        ));
    } else if sensitive_approval_required(context)?
        && !has_current_sensitive_approval_for_close(store, project_state, request, context, now)?
    {
        let related_refs = refs_with_context(
            change_unit_ref.clone().into_iter().collect(),
            non_current_judgment_refs_for_plan(
                store,
                project_state,
                request,
                context,
                JudgmentKind::SensitiveApproval,
            )?,
        );
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::SensitiveApproval,
            "missing_sensitive_approval",
            "A documented sensitive-action approval required for close is missing.",
            related_refs,
            vec![close_guidance(
                CloseGuidance::RequestSensitiveApproval,
                vec![task_ref.clone()],
            )],
        ));
    }

    Ok(blockers)
}

pub(super) fn completion_acceptance_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
    risk_acceptance_coverage: &[RiskAcceptanceCoverage],
    has_evidence_blockers: bool,
) -> Result<Vec<CloseReadinessBlocker>, CloseReadinessError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);

    if let Some(blocker) = final_acceptance_blocker(
        store,
        project_state,
        request,
        context,
        has_evidence_blockers,
    )? {
        blockers.push(blocker);
    }

    let residual_risk = residual_risk_state(context);
    if residual_risk.known && !residual_risk.visible {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ResidualRiskVisibility,
            "residual_risk_not_visible",
            "Residual risk exists but is not visible in the close basis.",
            vec![task_ref.clone()],
            vec![close_guidance(
                CloseGuidance::MakeResidualRiskVisible,
                vec![task_ref.clone()],
            )],
        ));
    }
    if residual_risk.known
        && residual_risk.visible
        && risk_acceptance_coverage
            .iter()
            .any(|coverage| !coverage.accepted)
    {
        let stale_refs = non_current_judgment_refs_for_plan(
            store,
            project_state,
            request,
            context,
            JudgmentKind::ResidualRiskAcceptance,
        )?;
        let (code, message) = if stale_refs.is_empty() {
            (
                "missing_residual_risk_acceptance",
                "Visible residual risk requires distinct residual-risk acceptance.",
            )
        } else {
            (
                "stale_residual_risk_acceptance",
                "The available residual-risk acceptance is stale or incompatible with the current close basis.",
            )
        };
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ResidualRiskAcceptance,
            code,
            message,
            refs_with_context(vec![task_ref.clone()], stale_refs),
            vec![close_guidance(
                CloseGuidance::RequestResidualRiskAcceptance,
                vec![task_ref],
            )],
        ));
    }

    Ok(blockers)
}

fn pending_user_action_refs_for_close_operation(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
    operation: UserActionOperation,
    now: &UtcTimestamp,
) -> Result<Vec<StateRecordRef>, CloseReadinessError> {
    let authorities =
        pending_user_action_authorities_for_context(store, project_state, request, context)?;
    let current_change_unit_id = context
        .current_change_unit
        .as_ref()
        .map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let operation_refs = close_operation_refs(request, project_state, context);
    let mut refs = Vec::new();
    for authority in &authorities {
        let blocks = if operation == UserActionOperation::CloseComplete
            && authority.action_kind == UserActionKind::SensitiveApproval
        {
            pending_sensitive_judgment_blocks_close(
                request,
                context,
                authority,
                current_change_unit_id.as_ref(),
                &operation_refs,
                now,
            )
        } else {
            let operation_context = UserActionOperationContext {
                operation,
                task_id: &request.task_id,
                change_unit_id: current_change_unit_id.as_ref(),
                scope_revision: context.task.scope_revision,
                close_basis: context.current_close_basis.as_ref(),
                operation_refs: &operation_refs,
                sensitive_approval: None,
            };
            user_action_blocks_operation(authority, &operation_context)
        };
        if blocks {
            refs.push(state_ref(
                StateRecordKind::UserActionRequest,
                &authority.user_action_request_id,
                &request.project_id,
                Some(&request.task_id),
                Some(project_state.state_version),
            ));
        }
    }
    Ok(refs)
}

fn pending_user_action_authorities_for_context(
    store: &CoreProjectStore,
    _project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
) -> Result<Vec<UserActionAuthority>, CloseReadinessError> {
    if let Some(authorities) = &context.pending_user_action_authorities {
        return Ok(authorities.clone());
    }
    let authorities = pending_user_action_authorities(store, &request.task_id, &context.now)?;
    context.pending_user_action_authorities = Some(authorities.clone());
    Ok(authorities)
}

fn resolved_judgment_authorities_for_context(
    store: &CoreProjectStore,
    _project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
    judgment_kind: JudgmentKind,
) -> Result<Vec<UserActionAuthority>, CloseReadinessError> {
    if let Some(authorities) = &context.resolved_judgment_authorities {
        return Ok(authorities
            .iter()
            .filter(|authority| authority.action_kind == judgment_kind.into())
            .cloned()
            .collect());
    }
    if let Some(authorities) = context
        .stored_resolved_judgment_authorities
        .get(&judgment_kind)
    {
        return Ok(authorities.clone());
    }
    let authorities =
        resolved_user_action_authorities(store, &request.task_id, judgment_kind, &context.now)?;
    context
        .stored_resolved_judgment_authorities
        .insert(judgment_kind, authorities.clone());
    Ok(authorities)
}

fn pending_sensitive_judgment_blocks_close(
    request: &CloseReadinessRequest,
    context: &CloseReadinessFacts,
    authority: &UserActionAuthority,
    current_change_unit_id: Option<&ChangeUnitId>,
    operation_refs: &[StateRecordRef],
    now: &UtcTimestamp,
) -> bool {
    let Some(close_basis) = context.current_close_basis.as_ref() else {
        return false;
    };
    close_basis
        .sensitive_action_requirements
        .iter()
        .any(|close_requirement| {
            let requirement = SensitiveApprovalRequirement {
                task_id: &request.task_id,
                change_unit_id: &close_requirement.change_unit_id,
                scope_revision: context.task.scope_revision,
                operation: &close_requirement.action_kind,
                normalized_paths: &close_requirement.normalized_paths,
                sensitive_categories: &close_requirement.sensitive_categories,
                baseline_ref: close_requirement.baseline_ref.as_ref(),
                required_for: UserActionRequiredFor::CloseComplete,
                now,
            };
            let operation_context = UserActionOperationContext {
                operation: UserActionOperation::CloseComplete,
                task_id: &request.task_id,
                change_unit_id: current_change_unit_id,
                scope_revision: context.task.scope_revision,
                close_basis: Some(close_basis),
                operation_refs,
                sensitive_approval: Some(&requirement),
            };
            user_action_blocks_operation(authority, &operation_context)
        })
}

fn close_operation_refs(
    request: &CloseReadinessRequest,
    project_state: &ProjectStateHeader,
    context: &CloseReadinessFacts,
) -> Vec<StateRecordRef> {
    let mut refs = vec![task_ref_for_close(request, project_state.state_version)];
    if let Some(change_unit) = context.current_change_unit.as_ref() {
        refs.push(change_unit_ref(
            &request.project_id,
            &request.task_id,
            change_unit,
            project_state.state_version,
        ));
    }
    if let Some(close_basis) = context.current_close_basis.as_ref() {
        refs.extend(close_basis.result_refs.clone());
        if let Some(evidence_ref) = close_basis.evidence_summary_ref.as_ref() {
            refs.push(evidence_ref.clone());
        }
        for risk in &close_basis.residual_risks {
            refs.extend(risk.source_refs.clone());
        }
    }
    refs
}

fn cancellation_authority_blocker(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
) -> Result<Option<CloseReadinessBlocker>, CloseReadinessError> {
    let current_change_unit_id = context
        .current_change_unit
        .as_ref()
        .map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let requirement = CancellationAuthorityRequirement {
        task_id: &request.task_id,
        change_unit_id: current_change_unit_id.as_ref(),
        scope_revision: context.task.scope_revision,
    };
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::Cancellation,
    )?;
    if authorities.iter().any(|authority| {
        user_action_required_for(authority, UserActionRequiredFor::CloseCancel)
            && current_cancellation_authority(authority, &requirement)
    }) {
        return Ok(None);
    }

    let mut stale_refs = Vec::new();
    let mut rejected_refs = Vec::new();
    for authority in &authorities {
        if !user_action_required_for(authority, UserActionRequiredFor::CloseCancel) {
            continue;
        }
        let user_action_request_ref = state_ref(
            StateRecordKind::UserActionRequest,
            &authority.user_action_request_id,
            &request.project_id,
            Some(&request.task_id),
            Some(project_state.state_version),
        );
        let current_basis_matches = authority.basis.as_ref().is_some_and(|basis| {
            let coordinates = basis.coordinates();
            coordinates.task_id == request.task_id
                && coordinates.scope_revision == context.task.scope_revision
                && coordinates.change_unit_id.as_ref() == current_change_unit_id.as_ref()
        });
        if !user_action_has_current_basis(authority) || !current_basis_matches {
            stale_refs.push(user_action_request_ref);
        } else if authority.resolution_outcome == Some(JudgmentResolutionOutcome::Rejected)
            && authority.resolved_by_actor_source == Some(ActorSource::LocalUser)
            && verified_user_channel_provenance(authority)
        {
            rejected_refs.push(user_action_request_ref);
        }
    }
    if stale_refs.is_empty() {
        stale_refs.extend(non_current_judgment_refs_for_plan(
            store,
            project_state,
            request,
            context,
            JudgmentKind::Cancellation,
        )?);
    }

    let task_ref = task_ref_for_close(request, project_state.state_version);
    let (code, message, related_refs) = if !rejected_refs.is_empty() {
        (
            "rejected_cancellation_authority",
            "The current user cancellation resolution rejected cancellation.",
            refs_with_context(vec![task_ref.clone()], rejected_refs),
        )
    } else if !stale_refs.is_empty() {
        (
            "stale_cancellation_authority",
            "The available cancellation resolution is stale or incompatible with the current Task scope.",
            refs_with_context(vec![task_ref.clone()], stale_refs),
        )
    } else {
        (
            "missing_cancellation_authority",
            "Cancelling the Task requires a current accepted user cancellation resolution.",
            vec![task_ref.clone()],
        )
    };
    Ok(Some(close_blocker(
        CloseReadinessBlockerCategory::UserAction,
        code,
        message,
        related_refs,
        vec![close_guidance(
            CloseGuidance::RequestCancellationAuthority,
            vec![task_ref],
        )],
    )))
}

fn final_acceptance_blocker(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
    has_evidence_blockers: bool,
) -> Result<Option<CloseReadinessBlocker>, CloseReadinessError> {
    let acceptance_policy = context.task.acceptance_policy;
    let control = context.task.effective_control_level;
    let acceptance_required = match control {
        TaskControlLevel::Observe => false,
        TaskControlLevel::Tracked | TaskControlLevel::Sensitive => true,
        TaskControlLevel::Light => match acceptance_policy {
            AcceptancePolicy::Required => true,
            AcceptancePolicy::NotRequired | AcceptancePolicy::PolicyDependent => {
                !light_completion_without_acceptance_allowed(
                    store,
                    request,
                    context,
                    has_evidence_blockers,
                )?
            }
        },
    };
    if !acceptance_required {
        return Ok(None);
    }
    let task_ref = task_ref_for_close(request, project_state.state_version);
    let Some(close_basis) = context.current_close_basis.clone() else {
        return Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::FinalAcceptance,
            "missing_final_acceptance",
            "Final acceptance is required before completing the Task.",
            vec![task_ref.clone()],
            vec![close_guidance(
                CloseGuidance::RequestFinalAcceptance,
                vec![task_ref],
            )],
        )));
    };
    let requirement = final_acceptance_requirement(&close_basis);
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::FinalAcceptance,
    )?;
    if authorities
        .iter()
        .any(|authority| current_final_acceptance(authority, &requirement))
    {
        return Ok(None);
    }

    let stale_refs = non_current_judgment_refs_for_plan(
        store,
        project_state,
        request,
        context,
        JudgmentKind::FinalAcceptance,
    )?;
    let (code, message, related_refs) = if stale_refs.is_empty() {
        (
            "missing_final_acceptance",
            "Final acceptance is required before completing the Task.",
            vec![task_ref.clone()],
        )
    } else {
        (
            "stale_final_acceptance",
            "The available final acceptance is stale or incompatible with the current close basis.",
            refs_with_context(vec![task_ref.clone()], stale_refs),
        )
    };
    Ok(Some(close_blocker(
        CloseReadinessBlockerCategory::FinalAcceptance,
        code,
        message,
        related_refs,
        vec![close_guidance(
            CloseGuidance::RequestFinalAcceptance,
            vec![task_ref],
        )],
    )))
}

fn light_completion_without_acceptance_allowed(
    store: &CoreProjectStore,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
    has_evidence_blockers: bool,
) -> Result<bool, CloseReadinessError> {
    if context.task.effective_control_level != TaskControlLevel::Light {
        return Ok(false);
    }
    let workflow_policy = workflow_policy_for_close_context(context)?.clone();
    let Some(close_basis) = context.current_close_basis.clone() else {
        return Ok(false);
    };
    let has_acceptance_required_risk = close_basis
        .residual_risks
        .iter()
        .any(|risk| risk.acceptance_required);
    if !light_acceptance_can_be_omitted(LightAcceptancePolicyFacts {
        light_enabled: workflow_policy.light.enabled,
        final_acceptance_policy: workflow_policy.light.final_acceptance,
        has_pending_user_action: !context.pending_user_action_refs.is_empty(),
        has_current_close_basis: true,
        has_acceptance_required_risk,
        has_sensitive_result: !close_basis.sensitive_categories.is_empty()
            || !close_basis.sensitive_action_requirements.is_empty(),
        has_unresolved_change: !context.unresolved_unrecorded_changes.is_empty(),
        has_evidence_blocker: has_evidence_blockers,
        writes_are_current_and_authorized: true,
    }) {
        return Ok(false);
    }

    if context.write_tickets.is_none() {
        context.write_tickets = Some(
            store
                .write_tickets_for_task(&request.task_id)
                .map_err(CorePipelineError::from)?,
        );
    }
    let tickets = context
        .write_tickets
        .as_ref()
        .expect("write-ticket facts are acquired before evaluation");
    for observed in store
        .run_observed_changes_for_task(&request.task_id)
        .map_err(CorePipelineError::from)?
    {
        if observed.status != volicord_store::core_pipeline::RunStatus::Recorded {
            return Ok(false);
        }
        if !observed.observed_changes.sensitive_categories.is_empty() {
            return Ok(false);
        }
        if !observed.observed_changes.product_file_write_observed {
            continue;
        }
        if !workflow_policy.light_paths_are_allowed(&observed.observed_changes.changed_paths) {
            return Ok(false);
        }
        let Some(run) = store
            .run_record(&observed.run_id)
            .map_err(CorePipelineError::from)?
        else {
            return Ok(false);
        };
        if run.scope_revision != context.task.scope_revision
            || run.change_unit_id.as_deref() != Some(close_basis.change_unit_id.as_str())
            || run.baseline_ref.as_ref().map(|value| value.as_str())
                != close_basis.baseline_ref.as_ref().map(BaselineRef::as_str)
        {
            return Ok(false);
        }
        let Some(ticket) = tickets.iter().find(|ticket| {
            ticket.status == WriteTicketStatus::Consumed
                && ticket.consumed_by_run_id.as_deref() == Some(observed.run_id.as_str())
        }) else {
            return Ok(false);
        };
        let validity_basis = &ticket.validity_basis;
        if validity_basis.task_id != request.task_id
            || validity_basis.change_unit_id != close_basis.change_unit_id
            || validity_basis.scope_revision != context.task.scope_revision
            || validity_basis.baseline_ref.as_ref() != close_basis.baseline_ref.as_ref()
        {
            return Ok(false);
        }
        let allowed = ticket
            .allowed_path_prefixes
            .iter()
            .map(|path| path.as_str().to_owned())
            .collect::<Vec<_>>();
        let denied = &ticket.denied_path_prefixes;
        if !paths_are_authorized(&observed.observed_changes.changed_paths, &allowed)
            || observed.observed_changes.changed_paths.iter().any(|path| {
                denied
                    .iter()
                    .any(|denied_prefix| path_is_within(path, denied_prefix.as_str()))
            })
        {
            return Ok(false);
        }
    }
    Ok(true)
}

#[derive(Debug, Clone, Copy)]
struct LightAcceptancePolicyFacts {
    light_enabled: bool,
    final_acceptance_policy: AcceptancePolicy,
    has_pending_user_action: bool,
    has_current_close_basis: bool,
    has_acceptance_required_risk: bool,
    has_sensitive_result: bool,
    has_unresolved_change: bool,
    has_evidence_blocker: bool,
    writes_are_current_and_authorized: bool,
}

fn light_acceptance_can_be_omitted(facts: LightAcceptancePolicyFacts) -> bool {
    facts.light_enabled
        && facts.final_acceptance_policy != AcceptancePolicy::Required
        && !facts.has_pending_user_action
        && facts.has_current_close_basis
        && !facts.has_acceptance_required_risk
        && !facts.has_sensitive_result
        && !facts.has_unresolved_change
        && !facts.has_evidence_blocker
        && facts.writes_are_current_and_authorized
}

fn has_current_sensitive_approval_for_close(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
    now: &UtcTimestamp,
) -> Result<bool, CloseReadinessError> {
    let Some(close_basis) = context.current_close_basis.clone() else {
        return Ok(false);
    };
    if close_basis.sensitive_action_requirements.is_empty() {
        return Ok(false);
    }
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::SensitiveApproval,
    )?;
    Ok(close_basis
        .sensitive_action_requirements
        .iter()
        .all(|close_requirement| {
            if close_requirement.change_unit_id != close_basis.change_unit_id {
                return false;
            }
            let requirement = SensitiveApprovalRequirement {
                task_id: &request.task_id,
                change_unit_id: &close_requirement.change_unit_id,
                scope_revision: context.task.scope_revision,
                operation: &close_requirement.action_kind,
                normalized_paths: &close_requirement.normalized_paths,
                sensitive_categories: &close_requirement.sensitive_categories,
                baseline_ref: close_requirement.baseline_ref.as_ref(),
                required_for: UserActionRequiredFor::CloseComplete,
                now,
            };
            authorities
                .iter()
                .any(|authority| current_sensitive_approval(authority, &requirement))
        }))
}

pub(super) fn risk_acceptance_coverage(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
) -> Result<Vec<RiskAcceptanceCoverage>, CloseReadinessError> {
    let Some(basis) = context.current_close_basis.clone() else {
        return Ok(Vec::new());
    };
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::ResidualRiskAcceptance,
    )?;
    let mut coverage = current_residual_risk_acceptance_coverage(
        &request.project_id,
        &request.task_id,
        project_state.state_version,
        &basis,
        &authorities,
    );
    let stale_refs = non_current_judgment_refs_for_plan(
        store,
        project_state,
        request,
        context,
        JudgmentKind::ResidualRiskAcceptance,
    )?;
    if !stale_refs.is_empty() {
        for item in coverage.iter_mut().filter(|item| !item.accepted) {
            item.missing_reason = Some("stale_acceptance".to_owned()).into();
        }
    }
    Ok(coverage)
}

fn non_current_judgment_refs_for_plan(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseReadinessRequest,
    context: &mut CloseReadinessFacts,
    judgment_kind: JudgmentKind,
) -> Result<Vec<StateRecordRef>, CloseReadinessError> {
    if let Some(refs) = context.non_current_judgment_refs.get(&judgment_kind) {
        return Ok(refs.clone());
    }
    let refs = store
        .non_current_user_action_refs(
            &request.task_id,
            judgment_kind.into(),
            project_state.state_version,
            &context.now,
        )
        .map_err(CorePipelineError::from)
        .map(stored_refs_to_state_refs)?;
    context
        .non_current_judgment_refs
        .insert(judgment_kind, refs.clone());
    Ok(refs)
}

fn refs_with_context(
    mut refs: Vec<StateRecordRef>,
    context_refs: Vec<StateRecordRef>,
) -> Vec<StateRecordRef> {
    refs.extend(context_refs);
    refs
}

fn sensitive_approval_required(context: &CloseReadinessFacts) -> CoreResult<bool> {
    Ok(
        context.task.effective_control_level == TaskControlLevel::Sensitive
            || context
                .current_close_basis
                .as_ref()
                .map(|basis| !basis.sensitive_action_requirements.is_empty())
                .unwrap_or(false),
    )
}

fn sensitive_action_basis_missing(context: &CloseReadinessFacts) -> CoreResult<bool> {
    Ok(
        context.task.effective_control_level == TaskControlLevel::Sensitive
            && context
                .current_close_basis
                .as_ref()
                .map(|basis| basis.sensitive_action_requirements.is_empty())
                .unwrap_or(true),
    )
}

#[derive(Debug, Clone, Copy)]
struct ResidualRiskState {
    known: bool,
    visible: bool,
}

fn residual_risk_state(context: &CloseReadinessFacts) -> ResidualRiskState {
    let known = context
        .current_close_basis
        .as_ref()
        .map(|basis| !basis.residual_risks.is_empty())
        .unwrap_or(false);
    ResidualRiskState {
        known,
        visible: known,
    }
}

#[cfg(test)]
#[path = "tests/acceptance.rs"]
mod tests;
