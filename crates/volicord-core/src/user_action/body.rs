use super::model::{UserActionBodyFacts, ValidatedUserAction, ValidatedUserActionIntent};
use super::validation::UserActionDomainError;
use crate::methods::normalize_display_text;
use volicord_types::ids::UserActionOptionId;
use volicord_types::schema::{
    UserActionBasis, UserActionChoiceBasis, UserActionChoiceDraft, UserActionChoiceRequestBody,
    UserActionDraft, UserActionEvidenceObservationBasis, UserActionEvidenceObservationDraft,
    UserActionEvidenceObservationRequestBody, UserActionOption, UserActionRequestBody,
};
use volicord_types::values::{JudgmentKind, JudgmentResolutionOutcome, UserActionOptionAction};

pub(super) fn construct_canonical_body(
    validated: ValidatedUserActionIntent,
    body_facts: UserActionBodyFacts,
    locale: Option<&str>,
) -> Result<ValidatedUserAction, UserActionDomainError> {
    let ValidatedUserActionIntent {
        task_id,
        coordinate_change_unit_id,
        action,
        coordinates,
        required_for,
        expires_at,
        created_at,
    } = validated;
    let (body, basis) = match (action, body_facts) {
        (
            UserActionDraft::Choice(choice),
            UserActionBodyFacts::Choice {
                close_basis_revision,
                result_refs,
                residual_risk_ids,
            },
        ) => {
            let UserActionChoiceDraft {
                judgment_kind,
                presentation,
                question,
                options,
                context,
                affected_refs,
                sensitive_action_scope,
            } = *choice;
            let options = canonical_choice_options(
                judgment_kind,
                options.as_ref().map(Vec::as_slice).unwrap_or_default(),
                locale,
            );
            (
                UserActionRequestBody::Choice(Box::new(UserActionChoiceRequestBody {
                    judgment_kind,
                    presentation,
                    question: normalize_display_text(&question),
                    options,
                    context,
                    affected_refs,
                    sensitive_action_scope: sensitive_action_scope.clone(),
                })),
                UserActionBasis::Choice(Box::new(UserActionChoiceBasis {
                    coordinates,
                    close_basis_revision: close_basis_revision.into(),
                    result_refs,
                    residual_risk_ids,
                    sensitive_action_scope,
                })),
            )
        }
        (
            UserActionDraft::EvidenceObservation(observation),
            UserActionBodyFacts::EvidenceObservation {
                artifact_candidates,
            },
        ) => {
            let UserActionEvidenceObservationDraft {
                question,
                context_summary,
                target_candidates,
                artifact_candidate_ids: _,
            } = observation;
            (
                UserActionRequestBody::EvidenceObservation(
                    UserActionEvidenceObservationRequestBody {
                        question: normalize_display_text(&question),
                        context_summary: normalize_display_text(&context_summary),
                        target_candidates: target_candidates.clone(),
                        artifact_candidates: artifact_candidates.clone(),
                    },
                ),
                UserActionBasis::EvidenceObservation(UserActionEvidenceObservationBasis {
                    coordinates,
                    target_candidates,
                    artifact_candidates,
                }),
            )
        }
        _ => {
            return Err(UserActionDomainError::new(
                "action.action_type",
                "validated action facts must match the semantic action family",
            ));
        }
    };
    body.capture_form()
        .map_err(|error| UserActionDomainError::new(error.field(), error.message()))?;
    Ok(ValidatedUserAction {
        task_id,
        coordinate_change_unit_id,
        body,
        basis,
        required_for,
        expires_at,
        created_at,
    })
}

fn canonical_choice_options(
    judgment_kind: JudgmentKind,
    caller_options: &[volicord_types::schema::UserActionOptionInput],
    locale: Option<&str>,
) -> Vec<UserActionOption> {
    let authority_bearing = matches!(
        judgment_kind,
        JudgmentKind::ScopeDecision
            | JudgmentKind::SensitiveApproval
            | JudgmentKind::FinalAcceptance
            | JudgmentKind::ResidualRiskAcceptance
            | JudgmentKind::Cancellation
    );
    if authority_bearing {
        return [
            UserActionOptionAction::Accept,
            UserActionOptionAction::Reject,
            UserActionOptionAction::Defer,
        ]
        .into_iter()
        .map(|machine_action| {
            let (label, description, consequence) =
                authority_option_copy(judgment_kind, machine_action, locale);
            UserActionOption {
                option_id: UserActionOptionId::new(match machine_action {
                    UserActionOptionAction::Accept => "accept",
                    UserActionOptionAction::Reject => "reject",
                    UserActionOptionAction::Defer => "defer",
                }),
                label,
                description,
                consequence,
                machine_action,
                resolution_outcome: machine_action.resolution_outcome(),
                is_default: machine_action == UserActionOptionAction::Accept,
            }
        })
        .collect();
    }
    caller_options
        .iter()
        .map(|option| UserActionOption {
            option_id: option.option_id.clone(),
            label: option.label.clone(),
            description: option.description.clone(),
            consequence: option.consequence.clone(),
            machine_action: UserActionOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            is_default: option.is_default,
        })
        .collect()
}

fn authority_option_copy(
    judgment_kind: JudgmentKind,
    action: UserActionOptionAction,
    locale: Option<&str>,
) -> (String, String, String) {
    let korean = locale
        .map(|locale| locale.to_ascii_lowercase().replace('_', "-"))
        .is_some_and(|locale| locale == "ko" || locale.starts_with("ko-"));
    let subject_en = match judgment_kind {
        JudgmentKind::ScopeDecision => "scope decision",
        JudgmentKind::SensitiveApproval => "sensitive action",
        JudgmentKind::FinalAcceptance => "final acceptance",
        JudgmentKind::ResidualRiskAcceptance => "residual risk",
        JudgmentKind::Cancellation => "task cancellation",
        JudgmentKind::ProductDecision => "product decision",
        JudgmentKind::TechnicalDecision => "technical decision",
    };
    let subject_ko = match judgment_kind {
        JudgmentKind::ScopeDecision => "범위 결정",
        JudgmentKind::SensitiveApproval => "민감 작업",
        JudgmentKind::FinalAcceptance => "최종 수락",
        JudgmentKind::ResidualRiskAcceptance => "잔여 위험",
        JudgmentKind::Cancellation => "작업 취소",
        JudgmentKind::ProductDecision => "제품 결정",
        JudgmentKind::TechnicalDecision => "기술 결정",
    };
    if korean {
        let (label, verb, outcome) = match action {
            UserActionOptionAction::Accept => ("수락", "수락합니다", "수락됨"),
            UserActionOptionAction::Reject => ("거부", "거부합니다", "거부됨"),
            UserActionOptionAction::Defer => ("보류", "나중으로 보류합니다", "보류됨"),
        };
        (
            label.to_owned(),
            format!("현재 근거에 따라 {subject_ko}을(를) {verb}."),
            format!("이 사용자 작업은 {outcome} 상태로 해결됩니다."),
        )
    } else {
        let (label, verb, outcome) = match action {
            UserActionOptionAction::Accept => ("Accept", "Accept", "accepted"),
            UserActionOptionAction::Reject => ("Reject", "Reject", "rejected"),
            UserActionOptionAction::Defer => ("Defer", "Defer", "deferred"),
        };
        (
            label.to_owned(),
            format!("{verb} the {subject_en} on the current basis."),
            format!("This user action resolves as {outcome}."),
        )
    }
}
