use std::collections::BTreeSet;

use volicord_types::ids::RunId;
use volicord_types::values::{
    TaskControlLevel, UserActionRequiredFor, WriteTicketInvalidationReason, WriteTicketStatus,
};
use volicord_user_action_service::{current_sensitive_approval, SensitiveApprovalRequirement};

use super::planning::PlannedWriteTicket;
use super::policy::write_ticket_is_idle_expired;
use super::read_model::{WriteTicketCurrentFacts, WriteTicketTaskFacts, WriteTicketWorkflowFacts};
use super::semantic::{
    planned_write_ticket_semantic_facts, StoredWriteTicketFacts, WriteTicketEvaluationIdentity,
    WriteTicketSemanticFacts,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteTicketAuthorityState {
    NotApplicable,
    Current,
    WriteAuthorityChanged,
    PendingPolicyReevaluation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteTicketApprovalState {
    NotApplicable,
    NotRequired,
    Current,
    Changed,
}

/// Fully evaluated Write Ticket state consumed by selection and summary mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvaluatedWriteTicket {
    pub(crate) identity: WriteTicketEvaluationIdentity,
    pub(crate) ticket: WriteTicketSemanticFacts,
    pub(crate) effective_status: WriteTicketStatus,
    pub(crate) invalidation: Option<WriteTicketInvalidationReason>,
    pub(crate) authority: WriteTicketAuthorityState,
    pub(crate) approval: WriteTicketApprovalState,
    pub(crate) consumed_by_run_id: Option<RunId>,
}

impl EvaluatedWriteTicket {
    pub(crate) fn stored_write_ticket_id(&self) -> Option<&volicord_types::ids::WriteTicketId> {
        match &self.identity {
            WriteTicketEvaluationIdentity::Stored { write_ticket_id } => Some(write_ticket_id),
            WriteTicketEvaluationIdentity::Planned { .. } => None,
        }
    }
}

/// Evaluates lifecycle facts that do not require current Store-backed context.
///
/// `None` means the ticket remains active and needs current authority and
/// approval facts before it is fully evaluated.
pub(crate) fn evaluate_terminal_write_ticket(
    ticket: StoredWriteTicketFacts,
    observed_at: &volicord_types::values::UtcTimestamp,
) -> Option<EvaluatedWriteTicket> {
    if ticket.status != WriteTicketStatus::Active {
        let status = ticket.status;
        let invalidation_reason = ticket.invalidation_reason;
        return Some(evaluated_stored_ticket(
            ticket,
            status,
            invalidation_reason,
            WriteTicketAuthorityState::NotApplicable,
            WriteTicketApprovalState::NotApplicable,
        ));
    }
    if write_ticket_is_idle_expired(ticket.ticket.idle_expires_at.as_ref(), observed_at) {
        return Some(evaluated_stored_ticket(
            ticket,
            WriteTicketStatus::Invalidated,
            Some(WriteTicketInvalidationReason::IdleTimeout),
            WriteTicketAuthorityState::NotApplicable,
            WriteTicketApprovalState::NotApplicable,
        ));
    }
    None
}

pub(crate) fn evaluate_current_write_ticket(
    ticket: StoredWriteTicketFacts,
    current: &WriteTicketCurrentFacts,
) -> EvaluatedWriteTicket {
    let basis = &ticket.ticket.validity_basis;
    if basis.write_authority_fingerprint != current.workflow.write_authority_fingerprint {
        return evaluated_stored_ticket(
            ticket,
            WriteTicketStatus::Invalidated,
            Some(WriteTicketInvalidationReason::ExplicitRevoke),
            WriteTicketAuthorityState::WriteAuthorityChanged,
            WriteTicketApprovalState::NotApplicable,
        );
    }
    if current.task.pending_policy_reevaluation {
        return evaluated_stored_ticket(
            ticket,
            WriteTicketStatus::Invalidated,
            Some(WriteTicketInvalidationReason::ExplicitRevoke),
            WriteTicketAuthorityState::PendingPolicyReevaluation,
            WriteTicketApprovalState::NotApplicable,
        );
    }

    let approval = evaluate_approval(&ticket.ticket, current);
    if approval == WriteTicketApprovalState::Changed {
        return evaluated_stored_ticket(
            ticket,
            WriteTicketStatus::Invalidated,
            Some(WriteTicketInvalidationReason::ApprovalBasisChanged),
            WriteTicketAuthorityState::Current,
            approval,
        );
    }

    evaluated_stored_ticket(
        ticket,
        WriteTicketStatus::Active,
        None,
        WriteTicketAuthorityState::Current,
        approval,
    )
}

pub(crate) fn requires_sensitive_approval_facts(
    ticket: &StoredWriteTicketFacts,
    task: &WriteTicketTaskFacts,
    workflow: &WriteTicketWorkflowFacts,
    observed_at: &volicord_types::values::UtcTimestamp,
) -> bool {
    ticket.status == WriteTicketStatus::Active
        && !write_ticket_is_idle_expired(ticket.ticket.idle_expires_at.as_ref(), observed_at)
        && ticket.ticket.validity_basis.write_authority_fingerprint
            == workflow.write_authority_fingerprint
        && !task.pending_policy_reevaluation
        && !ticket.ticket.validity_basis.approval_basis_refs.is_empty()
}

pub(crate) fn evaluate_planned_write_ticket(plan: &PlannedWriteTicket) -> EvaluatedWriteTicket {
    let ticket = planned_write_ticket_semantic_facts(plan);
    let approval = if ticket.validity_basis.approval_basis_refs.is_empty() {
        WriteTicketApprovalState::NotRequired
    } else {
        WriteTicketApprovalState::Current
    };
    EvaluatedWriteTicket {
        identity: WriteTicketEvaluationIdentity::Planned {
            write_ticket_id: plan.write_ticket_id().clone(),
        },
        ticket,
        effective_status: WriteTicketStatus::Active,
        invalidation: None,
        authority: WriteTicketAuthorityState::Current,
        approval,
        consumed_by_run_id: None,
    }
}

pub(crate) fn evaluate_reused_write_ticket(ticket: StoredWriteTicketFacts) -> EvaluatedWriteTicket {
    debug_assert_eq!(ticket.status, WriteTicketStatus::Active);
    let approval = if ticket.ticket.validity_basis.approval_basis_refs.is_empty() {
        WriteTicketApprovalState::NotRequired
    } else {
        WriteTicketApprovalState::Current
    };
    evaluated_stored_ticket(
        ticket,
        WriteTicketStatus::Active,
        None,
        WriteTicketAuthorityState::Current,
        approval,
    )
}

pub(crate) fn evaluate_projected_write_ticket_consumption(
    ticket: StoredWriteTicketFacts,
    run_id: RunId,
) -> EvaluatedWriteTicket {
    let mut evaluated = evaluated_stored_ticket(
        ticket,
        WriteTicketStatus::Consumed,
        None,
        WriteTicketAuthorityState::NotApplicable,
        WriteTicketApprovalState::NotApplicable,
    );
    evaluated.consumed_by_run_id = Some(run_id);
    evaluated
}

fn evaluate_approval(
    ticket: &WriteTicketSemanticFacts,
    current: &WriteTicketCurrentFacts,
) -> WriteTicketApprovalState {
    let basis = &ticket.validity_basis;
    let scope = &ticket.attempt_scope;
    if basis.approval_basis_refs.is_empty() {
        return if scope.sensitive_categories.is_empty()
            && current.task.effective_control_level != TaskControlLevel::Sensitive
        {
            WriteTicketApprovalState::NotRequired
        } else {
            WriteTicketApprovalState::Changed
        };
    }

    let normalized_scope_paths = scope
        .intended_paths
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect::<Vec<_>>();
    let requirement = SensitiveApprovalRequirement {
        task_id: &basis.task_id,
        change_unit_id: &basis.change_unit_id,
        scope_revision: current.task.scope_revision,
        operation: &scope.intended_operation,
        normalized_paths: &normalized_scope_paths,
        sensitive_categories: &scope.sensitive_categories,
        baseline_ref: scope.baseline_ref.as_ref(),
        required_for: UserActionRequiredFor::PrepareWrite,
        now: &current.observed_at,
    };
    let current_resolution_identities = current
        .sensitive_approvals
        .iter()
        .filter(|authority| current_sensitive_approval(authority, &requirement))
        .filter_map(|authority| authority.resolution_identity())
        .collect::<BTreeSet<_>>();
    let approval_basis_is_current = !current_resolution_identities.is_empty()
        && basis
            .approval_basis_refs
            .iter()
            .all(|reference| current_resolution_identities.contains(&reference.identity()));
    if approval_basis_is_current {
        WriteTicketApprovalState::Current
    } else {
        WriteTicketApprovalState::Changed
    }
}

fn evaluated_stored_ticket(
    ticket: StoredWriteTicketFacts,
    effective_status: WriteTicketStatus,
    invalidation: Option<WriteTicketInvalidationReason>,
    authority: WriteTicketAuthorityState,
    approval: WriteTicketApprovalState,
) -> EvaluatedWriteTicket {
    EvaluatedWriteTicket {
        identity: WriteTicketEvaluationIdentity::Stored {
            write_ticket_id: ticket.write_ticket_id,
        },
        ticket: ticket.ticket,
        effective_status,
        invalidation,
        authority,
        approval,
        consumed_by_run_id: ticket.consumed_by_run_id,
    }
}

#[cfg(test)]
mod tests {
    use volicord_types::ids::{
        BaselineRef, ProjectId, TaskId, UserActionOptionId, UserActionRequestId,
        UserActionResolutionId,
    };
    use volicord_types::schema::{
        RequiredNullable, SensitiveActionScope, UserActionBasis, UserActionBasisCoordinates,
        UserActionChoiceBasis, UserActionResolutionBody, UserActionResolutionRef,
    };
    use volicord_types::values::{
        ActorSource, JudgmentResolutionOutcome, TaskControlLevel, UserActionBasisStatus,
        UserActionKind, UserActionOptionAction, UserActionRequiredFor, UserActionStatus,
        UserActionVerificationBasis, WriteTicketInvalidationReason, WriteTicketStatus,
    };
    use volicord_user_action_service::UserActionAuthority;

    use super::{
        evaluate_current_write_ticket, evaluate_terminal_write_ticket,
        requires_sensitive_approval_facts, WriteTicketApprovalState, WriteTicketAuthorityState,
    };
    use crate::write_ticket::read_model::{
        WriteTicketCurrentFacts, WriteTicketTaskFacts, WriteTicketWorkflowFacts,
    };
    use crate::write_ticket::semantic::test_support::{stored_facts, timestamp};

    fn task_facts() -> WriteTicketTaskFacts {
        WriteTicketTaskFacts {
            scope_revision: 3,
            effective_control_level: TaskControlLevel::Tracked,
            pending_policy_reevaluation: false,
        }
    }

    fn workflow_facts() -> WriteTicketWorkflowFacts {
        WriteTicketWorkflowFacts {
            write_authority_fingerprint: format!("sha256:{}", "0".repeat(64)),
        }
    }

    fn current_facts(sensitive_approvals: Vec<UserActionAuthority>) -> WriteTicketCurrentFacts {
        WriteTicketCurrentFacts {
            task: task_facts(),
            workflow: workflow_facts(),
            sensitive_approvals,
            observed_at: timestamp("2026-07-29T00:05:00Z"),
        }
    }

    fn accepted_sensitive_approval() -> UserActionAuthority {
        let coordinates = UserActionBasisCoordinates {
            task_id: TaskId::new("task-test"),
            change_unit_id: RequiredNullable::some(volicord_types::ids::ChangeUnitId::new(
                "change-test",
            )),
            scope_revision: 3,
            baseline_ref: RequiredNullable::some(BaselineRef::new("baseline-test")),
            created_at_state_version: 6,
            compatibility_status: UserActionBasisStatus::Current,
        };
        UserActionAuthority {
            project_id: ProjectId::new("project-test"),
            user_action_request_id: UserActionRequestId::new("request-approval"),
            user_action_resolution_id: Some(UserActionResolutionId::new("resolution-approval")),
            task_id: TaskId::new("task-test"),
            action_kind: UserActionKind::SensitiveApproval,
            status: UserActionStatus::Resolved,
            required_for: vec![UserActionRequiredFor::PrepareWrite],
            affected_refs: Vec::new(),
            machine_action: Some(UserActionOptionAction::Accept),
            resolution_outcome: Some(JudgmentResolutionOutcome::Accepted),
            resolved_by_actor_source: Some(ActorSource::LocalUser),
            resolved_verification_basis: Some(UserActionVerificationBasis::CliDirectUserChannel),
            resolved_assurance_level: Some("direct_user_input".to_owned()),
            basis_status: UserActionBasisStatus::Current,
            basis: Some(UserActionBasis::Choice(Box::new(UserActionChoiceBasis {
                coordinates,
                close_basis_revision: RequiredNullable::null(),
                result_refs: Vec::new(),
                residual_risk_ids: Vec::new(),
                sensitive_action_scope: RequiredNullable::some(SensitiveActionScope {
                    action_kind: "edit".to_owned(),
                    description: "Approve the exact test operation.".to_owned(),
                    intended_paths: vec!["src".to_owned()],
                    sensitive_categories: vec!["network".to_owned()],
                    command_or_tool_summary: RequiredNullable::null(),
                    network_or_host_summary: RequiredNullable::null(),
                    secret_or_credential_summary: RequiredNullable::null(),
                    capability_claim: "test approval".to_owned(),
                    expires_at: RequiredNullable::null(),
                }),
            }))),
            resolution: Some(UserActionResolutionBody::Choice {
                selected_option_id: UserActionOptionId::new("accept"),
                machine_action: UserActionOptionAction::Accept,
                resolution_outcome: JudgmentResolutionOutcome::Accepted,
                note: RequiredNullable::null(),
                accepted_risk_ids: Vec::new(),
            }),
            expires_at: None,
        }
    }

    #[test]
    fn terminal_status_and_idle_expiry_do_not_require_current_facts() {
        let revoked = stored_facts("ticket-revoked", WriteTicketStatus::Revoked, 7);
        let evaluated = evaluate_terminal_write_ticket(revoked, &timestamp("2026-07-29T00:05:00Z"))
            .expect("terminal ticket should evaluate");
        assert_eq!(evaluated.effective_status, WriteTicketStatus::Revoked);
        assert_eq!(
            evaluated.authority,
            WriteTicketAuthorityState::NotApplicable
        );

        let active = stored_facts("ticket-expired", WriteTicketStatus::Active, 7);
        let evaluated = evaluate_terminal_write_ticket(active, &timestamp("2026-07-29T00:15:00Z"))
            .expect("expired active ticket should evaluate");
        assert_eq!(evaluated.effective_status, WriteTicketStatus::Invalidated);
        assert_eq!(
            evaluated.invalidation,
            Some(WriteTicketInvalidationReason::IdleTimeout)
        );
    }

    #[test]
    fn authority_change_and_pending_reevaluation_have_typed_results() {
        let mut changed = current_facts(Vec::new());
        changed.workflow.write_authority_fingerprint = format!("sha256:{}", "1".repeat(64));
        let evaluated = evaluate_current_write_ticket(
            stored_facts("ticket-changed", WriteTicketStatus::Active, 7),
            &changed,
        );
        assert_eq!(evaluated.effective_status, WriteTicketStatus::Invalidated);
        assert_eq!(
            evaluated.authority,
            WriteTicketAuthorityState::WriteAuthorityChanged
        );
        assert_eq!(
            evaluated.invalidation,
            Some(WriteTicketInvalidationReason::ExplicitRevoke)
        );

        let mut pending = current_facts(Vec::new());
        pending.task.pending_policy_reevaluation = true;
        let evaluated = evaluate_current_write_ticket(
            stored_facts("ticket-pending", WriteTicketStatus::Active, 7),
            &pending,
        );
        assert_eq!(
            evaluated.authority,
            WriteTicketAuthorityState::PendingPolicyReevaluation
        );
        assert_eq!(evaluated.approval, WriteTicketApprovalState::NotApplicable);
    }

    #[test]
    fn sensitive_approval_basis_must_resolve_to_the_current_exact_scope() {
        let mut ticket = stored_facts("ticket-sensitive", WriteTicketStatus::Active, 7);
        ticket.ticket.attempt_scope.sensitive_categories = vec!["network".to_owned()];
        ticket.ticket.validity_basis.approval_basis_refs = vec![UserActionResolutionRef::new(
            ticket.ticket.project_id.clone(),
            ticket.ticket.validity_basis.task_id.clone(),
            UserActionResolutionId::new("resolution-approval"),
            Some(6),
        )];

        assert!(requires_sensitive_approval_facts(
            &ticket,
            &task_facts(),
            &workflow_facts(),
            &timestamp("2026-07-29T00:05:00Z")
        ));
        let changed = evaluate_current_write_ticket(ticket.clone(), &current_facts(Vec::new()));
        assert_eq!(changed.effective_status, WriteTicketStatus::Invalidated);
        assert_eq!(changed.approval, WriteTicketApprovalState::Changed);
        assert_eq!(
            changed.invalidation,
            Some(WriteTicketInvalidationReason::ApprovalBasisChanged)
        );

        let mut other_project = accepted_sensitive_approval();
        other_project.project_id = ProjectId::new("project-other");
        let changed =
            evaluate_current_write_ticket(ticket.clone(), &current_facts(vec![other_project]));
        assert_eq!(changed.approval, WriteTicketApprovalState::Changed);

        let mut other_task = accepted_sensitive_approval();
        other_task.task_id = TaskId::new("task-other");
        let changed =
            evaluate_current_write_ticket(ticket.clone(), &current_facts(vec![other_task]));
        assert_eq!(changed.approval, WriteTicketApprovalState::Changed);

        let current = evaluate_current_write_ticket(
            ticket,
            &current_facts(vec![accepted_sensitive_approval()]),
        );
        assert_eq!(current.effective_status, WriteTicketStatus::Active);
        assert_eq!(current.authority, WriteTicketAuthorityState::Current);
        assert_eq!(current.approval, WriteTicketApprovalState::Current);
    }

    #[test]
    fn authority_invalidation_short_circuits_sensitive_approval_acquisition() {
        let mut ticket = stored_facts("ticket-sensitive", WriteTicketStatus::Active, 7);
        ticket.ticket.validity_basis.approval_basis_refs = vec![UserActionResolutionRef::new(
            ticket.ticket.project_id.clone(),
            ticket.ticket.validity_basis.task_id.clone(),
            UserActionResolutionId::new("resolution-approval"),
            Some(6),
        )];
        let changed_workflow = WriteTicketWorkflowFacts {
            write_authority_fingerprint: format!("sha256:{}", "1".repeat(64)),
        };

        assert!(!requires_sensitive_approval_facts(
            &ticket,
            &task_facts(),
            &changed_workflow,
            &timestamp("2026-07-29T00:05:00Z")
        ));
    }
}
