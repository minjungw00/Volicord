use crate::error::{UserActionServiceError, UserActionUnavailable};
use std::collections::BTreeSet;
use volicord_types::{
    schema::{
        ArtifactRef, CurrentCloseBasis, PersistedProjectContinuityMetadata,
        PersistedProjectContinuitySource, StateRecordRef, UserActionBasis, UserActionRequestBody,
        UserActionResolutionBody,
    },
    values::{
        JudgmentKind, JudgmentResolutionOutcome, ProjectContinuityKind, UserActionOptionAction,
    },
};

/// Current semantic facts used to derive durable continuity from a resolution.
pub struct UserActionContinuityInput<'a> {
    pub request_body: &'a UserActionRequestBody,
    pub basis: &'a UserActionBasis,
    pub resolution: &'a UserActionResolutionBody,
    pub resolution_ref: &'a StateRecordRef,
    pub applies_to_paths: Vec<String>,
    pub current_close_basis: Option<&'a CurrentCloseBasis>,
}

/// Service-owned semantic continuity draft consumed by Core persistence planning.
#[derive(Debug, Clone, PartialEq)]
pub struct UserActionContinuityDraft {
    pub kind: ProjectContinuityKind,
    pub title: String,
    pub summary: String,
    pub rationale: Option<String>,
    pub applies_to_paths: Vec<String>,
    pub applies_to_refs: Vec<StateRecordRef>,
    pub source_refs: Vec<StateRecordRef>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub supersedes_refs: Vec<StateRecordRef>,
    pub review_triggers: Vec<String>,
    pub metadata: PersistedProjectContinuityMetadata,
}

/// Derives the exact continuity facts authorized by one accepted UserAction.
pub fn derive_user_action_continuity(
    input: UserActionContinuityInput<'_>,
) -> Result<Vec<UserActionContinuityDraft>, UserActionServiceError> {
    let (
        UserActionRequestBody::Choice(choice),
        UserActionBasis::Choice(_),
        UserActionResolutionBody::Choice {
            selected_option_id,
            machine_action,
            resolution_outcome,
            accepted_risk_ids,
            ..
        },
    ) = (input.request_body, input.basis, input.resolution)
    else {
        return Ok(Vec::new());
    };
    if *machine_action != UserActionOptionAction::Accept
        || *resolution_outcome != JudgmentResolutionOutcome::Accepted
    {
        return Ok(Vec::new());
    }
    let Some(continuity_kind) = judgment_continuity_kind(choice.judgment_kind, *resolution_outcome)
    else {
        return Ok(Vec::new());
    };
    let selected = choice
        .options
        .iter()
        .find(|option| option.option_id == *selected_option_id)
        .ok_or(UserActionServiceError::Unavailable(
            UserActionUnavailable::StoredResolutionOptionMissing,
        ))?;

    match continuity_kind {
        ProjectContinuityKind::Decision => {
            let mut applies_to_refs = choice.affected_refs.clone();
            applies_to_refs.extend(choice.context.related_refs.clone());
            let mut source_refs = vec![input.resolution_ref.clone()];
            source_refs.extend(applies_to_refs.clone());
            Ok(vec![UserActionContinuityDraft {
                kind: ProjectContinuityKind::Decision,
                title: format!(
                    "{}: {}",
                    decision_title_prefix(choice.judgment_kind),
                    selected.label.trim()
                ),
                summary: format!(
                    "Selected option: {}. {}",
                    selected.label,
                    choice.context.summary.trim()
                ),
                rationale: None,
                applies_to_paths: input.applies_to_paths,
                applies_to_refs,
                source_refs,
                artifact_refs: choice.context.artifact_refs.clone(),
                supersedes_refs: Vec::new(),
                review_triggers: Vec::new(),
                metadata: PersistedProjectContinuityMetadata::ResolveUserActionDecision {
                    source: PersistedProjectContinuitySource::ResolveUserAction,
                    action_kind: input.request_body.action_kind(),
                    resolution_outcome: *resolution_outcome,
                    selected_option_id: selected_option_id.clone(),
                },
            }])
        }
        ProjectContinuityKind::AcceptedRisk => {
            if accepted_risk_ids.is_empty() {
                return Ok(Vec::new());
            }
            let close_basis =
                input
                    .current_close_basis
                    .ok_or(UserActionServiceError::Unavailable(
                        UserActionUnavailable::CurrentCloseBasisRequired,
                    ))?;
            let accepted = accepted_risk_ids.iter().collect::<BTreeSet<_>>();
            let risks = close_basis
                .residual_risks
                .iter()
                .filter(|risk| accepted.contains(&risk.risk_id))
                .collect::<Vec<_>>();
            if risks.len() != accepted.len() {
                return Err(UserActionServiceError::Unavailable(
                    UserActionUnavailable::AcceptedRiskIdentityMismatch,
                ));
            }
            Ok(risks
                .into_iter()
                .map(|risk| {
                    let mut source_refs = vec![input.resolution_ref.clone()];
                    source_refs.extend(risk.source_refs.clone());
                    let mut applies_to_refs = close_basis.result_refs.clone();
                    applies_to_refs.extend(risk.source_refs.clone());
                    UserActionContinuityDraft {
                        kind: ProjectContinuityKind::AcceptedRisk,
                        title: format!("Accepted residual risk: {}", risk.summary.trim()),
                        summary: risk.summary.clone(),
                        rationale: None,
                        applies_to_paths: input.applies_to_paths.clone(),
                        applies_to_refs,
                        source_refs,
                        artifact_refs: choice.context.artifact_refs.clone(),
                        supersedes_refs: Vec::new(),
                        review_triggers: Vec::new(),
                        metadata:
                            PersistedProjectContinuityMetadata::ResolveUserActionAcceptedRisk {
                                source: PersistedProjectContinuitySource::ResolveUserAction,
                                action_kind: input.request_body.action_kind(),
                                risk_id: risk.risk_id.clone(),
                                close_basis_revision: close_basis.close_basis_revision,
                            },
                    }
                })
                .collect())
        }
        _ => Ok(Vec::new()),
    }
}

fn judgment_continuity_kind(
    judgment_kind: JudgmentKind,
    outcome: JudgmentResolutionOutcome,
) -> Option<ProjectContinuityKind> {
    if outcome != JudgmentResolutionOutcome::Accepted {
        return None;
    }
    match judgment_kind {
        JudgmentKind::ProductDecision
        | JudgmentKind::TechnicalDecision
        | JudgmentKind::ScopeDecision => Some(ProjectContinuityKind::Decision),
        JudgmentKind::ResidualRiskAcceptance => Some(ProjectContinuityKind::AcceptedRisk),
        JudgmentKind::SensitiveApproval
        | JudgmentKind::FinalAcceptance
        | JudgmentKind::Cancellation => None,
    }
}

fn decision_title_prefix(judgment_kind: JudgmentKind) -> &'static str {
    match judgment_kind {
        JudgmentKind::ProductDecision => "Product decision",
        JudgmentKind::TechnicalDecision => "Technical decision",
        JudgmentKind::ScopeDecision => "Scope decision",
        JudgmentKind::ResidualRiskAcceptance => "Residual risk acceptance",
        JudgmentKind::SensitiveApproval => "Sensitive approval",
        JudgmentKind::FinalAcceptance => "Final acceptance",
        JudgmentKind::Cancellation => "Cancellation",
    }
}
