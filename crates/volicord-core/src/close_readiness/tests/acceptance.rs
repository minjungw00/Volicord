use super::*;
use crate::close_readiness::test_support;
use volicord_types::ids::{ChangeUnitId, ProjectId, RecordId, RiskId, TaskId, UserActionOptionId};
use volicord_types::schema::{
    CurrentCloseBasis, RequiredNullable, ResidualRisk, StateRecordRef, UserActionBasis,
    UserActionBasisCoordinates, UserActionChoiceBasis, UserActionResolutionBody,
};
use volicord_types::values::{
    JudgmentResolutionOutcome, StateRecordKind, TaskControlLevel, UserActionBasisStatus,
    UserActionOptionAction, UserActionStatus, UserActionVerificationBasis,
};
use volicord_user_action_service::UserActionAuthority;

#[test]
fn sensitive_control_requires_approval_and_a_ticket_backed_basis() {
    let mut facts = test_support::facts();
    assert!(!sensitive_approval_required(&facts).expect("valid light control"));
    assert!(!sensitive_action_basis_missing(&facts).expect("valid light control"));

    facts.task.effective_control_level = TaskControlLevel::Sensitive;
    assert!(sensitive_approval_required(&facts).expect("valid sensitive control"));
    assert!(sensitive_action_basis_missing(&facts).expect("valid sensitive control"));
}

#[test]
fn light_acceptance_policy_matrix_is_owned_by_acceptance() {
    let allowed = LightAcceptancePolicyFacts {
        light_enabled: true,
        final_acceptance_policy: AcceptancePolicy::PolicyDependent,
        has_pending_user_action: false,
        has_current_close_basis: true,
        has_acceptance_required_risk: false,
        has_sensitive_result: false,
        has_unresolved_change: false,
        has_evidence_blocker: false,
        writes_are_current_and_authorized: true,
    };
    assert!(light_acceptance_can_be_omitted(allowed));

    let cases = [
        (
            "light policy disabled",
            LightAcceptancePolicyFacts {
                light_enabled: false,
                ..allowed
            },
        ),
        (
            "final acceptance required",
            LightAcceptancePolicyFacts {
                final_acceptance_policy: AcceptancePolicy::Required,
                ..allowed
            },
        ),
        (
            "pending user action",
            LightAcceptancePolicyFacts {
                has_pending_user_action: true,
                ..allowed
            },
        ),
        (
            "missing current close basis",
            LightAcceptancePolicyFacts {
                has_current_close_basis: false,
                ..allowed
            },
        ),
        (
            "current risk",
            LightAcceptancePolicyFacts {
                has_acceptance_required_risk: true,
                ..allowed
            },
        ),
        (
            "non-write sensitive result",
            LightAcceptancePolicyFacts {
                has_sensitive_result: true,
                ..allowed
            },
        ),
        (
            "unresolved change",
            LightAcceptancePolicyFacts {
                has_unresolved_change: true,
                ..allowed
            },
        ),
        (
            "narrowed write policy",
            LightAcceptancePolicyFacts {
                writes_are_current_and_authorized: false,
                ..allowed
            },
        ),
        (
            "evidence blocker",
            LightAcceptancePolicyFacts {
                has_evidence_blocker: true,
                ..allowed
            },
        ),
    ];

    for (name, facts) in cases {
        assert!(
            !light_acceptance_can_be_omitted(facts),
            "{name} must require final acceptance"
        );
    }
}

fn run_ref() -> StateRecordRef {
    StateRecordRef {
        record_kind: StateRecordKind::Run,
        record_id: RecordId::new("run-acceptance"),
        project_id: ProjectId::new("project-acceptance"),
        task_id: Some(TaskId::new("task-acceptance")).into(),
        produced_at_state_version: Some(7).into(),
    }
}

fn close_basis() -> CurrentCloseBasis {
    CurrentCloseBasis {
        close_basis_revision: 4,
        scope_revision: 3,
        task_id: TaskId::new("task-acceptance"),
        change_unit_id: ChangeUnitId::new("change-acceptance"),
        baseline_ref: RequiredNullable::null(),
        result_summary: "current result".to_owned(),
        result_refs: vec![run_ref()],
        evidence_refs: Vec::new(),
        evidence_summary_ref: RequiredNullable::null(),
        residual_risks: vec![ResidualRisk {
            risk_id: RiskId::new("risk-acceptance"),
            summary: "visible risk".to_owned(),
            consequence: "known consequence".to_owned(),
            acceptance_required: true,
            source_refs: vec![run_ref()],
        }],
        sensitive_categories: Vec::new(),
        sensitive_action_requirements: Vec::new(),
        recovery_constraints: Vec::new(),
        source_run_ref: RequiredNullable::some(StateRecordRef {
            record_kind: StateRecordKind::Run,
            record_id: RecordId::new("run-acceptance"),
            project_id: ProjectId::new("project-acceptance"),
            task_id: Some(TaskId::new("task-acceptance")).into(),
            produced_at_state_version: Some(7).into(),
        }),
        shaping_checkpoint_ref: RequiredNullable::null(),
        shaping_decision_application_refs: Vec::new(),
        updated_at: UtcTimestamp::parse("2026-07-27T00:00:00Z").unwrap(),
    }
}

fn accepted_authority(kind: UserActionKind, risk_ids: Vec<RiskId>) -> UserActionAuthority {
    let basis = close_basis();
    UserActionAuthority {
        project_id: ProjectId::new("project-acceptance"),
        user_action_request_id: volicord_types::ids::UserActionRequestId::new("action-acceptance"),
        user_action_resolution_id: Some(volicord_types::ids::UserActionResolutionId::new(
            "resolution-acceptance",
        )),
        task_id: basis.task_id.clone(),
        action_kind: kind,
        status: UserActionStatus::Resolved,
        required_for: vec![UserActionRequiredFor::CloseComplete],
        affected_refs: Vec::new(),
        machine_action: Some(UserActionOptionAction::Accept),
        resolution_outcome: Some(JudgmentResolutionOutcome::Accepted),
        resolved_by_actor_source: Some(ActorSource::LocalUser),
        resolved_verification_basis: Some(UserActionVerificationBasis::CliDirectUserChannel),
        resolved_assurance_level: Some("direct_user_input".to_owned()),
        basis_status: UserActionBasisStatus::Current,
        basis: Some(UserActionBasis::Choice(Box::new(UserActionChoiceBasis {
            coordinates: UserActionBasisCoordinates {
                task_id: basis.task_id.clone(),
                change_unit_id: Some(basis.change_unit_id.clone()).into(),
                scope_revision: basis.scope_revision,
                baseline_ref: basis.baseline_ref.clone(),
                created_at_state_version: 7,
                compatibility_status: UserActionBasisStatus::Current,
            },
            close_basis_revision: Some(basis.close_basis_revision).into(),
            result_refs: basis.result_refs.clone(),
            residual_risk_ids: basis
                .residual_risks
                .iter()
                .map(|risk| risk.risk_id.clone())
                .collect(),
            sensitive_action_scope: RequiredNullable::null(),
        }))),
        resolution: Some(UserActionResolutionBody::Choice {
            selected_option_id: UserActionOptionId::new("accept"),
            machine_action: UserActionOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            note: RequiredNullable::null(),
            accepted_risk_ids: risk_ids,
        }),
        expires_at: None,
    }
}

#[test]
fn final_and_residual_risk_acceptance_require_current_exact_user_authority() {
    let basis = close_basis();
    let final_acceptance = accepted_authority(UserActionKind::FinalAcceptance, Vec::new());
    assert!(current_final_acceptance(
        &final_acceptance,
        &final_acceptance_requirement(&basis)
    ));

    let mut stale_final = final_acceptance;
    let Some(UserActionBasis::Choice(stale_basis)) = stale_final.basis.as_mut() else {
        panic!("test authority must use a choice basis");
    };
    stale_basis.coordinates.scope_revision -= 1;
    assert!(!current_final_acceptance(
        &stale_final,
        &final_acceptance_requirement(&basis)
    ));

    let risk_id = basis.residual_risks[0].risk_id.clone();
    let risk_acceptance = accepted_authority(
        UserActionKind::ResidualRiskAcceptance,
        vec![risk_id.clone()],
    );
    let coverage = current_residual_risk_acceptance_coverage(
        &ProjectId::new("project-acceptance"),
        &basis.task_id,
        7,
        &basis,
        &[risk_acceptance],
    );
    assert_eq!(coverage.len(), 1);
    assert!(coverage[0].accepted);

    let missing = current_residual_risk_acceptance_coverage(
        &ProjectId::new("project-acceptance"),
        &basis.task_id,
        7,
        &basis,
        &[],
    );
    assert!(!missing[0].accepted);
}
