use crate::{InquiryScope, QuestionPresentation};
use std::collections::{BTreeMap, BTreeSet};
use volicord_context::{
    CanonicalReadBasis, CanonicalRecordKind, Question, QuestionId, QuestionMateriality,
    QuestionResearchState, QuestionState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierDiagnosticKind {
    DependencyCycle,
    MissingPrerequisite,
    UnsatisfiedOutcome,
    BlockedByPrerequisite,
    InvalidDependencyRevision,
    InvalidDependencyBasis,
    SupersededByPrerequisite,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierDiagnostic {
    pub kind: FrontierDiagnosticKind,
    pub question_id: QuestionId,
    pub prerequisite_question_id: Option<QuestionId>,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierRead {
    pub project_id: volicord_context::ProjectId,
    pub questions: Vec<QuestionPresentation>,
    pub diagnostics: Vec<FrontierDiagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeFrontier {
    pub recomputed: FrontierRead,
    pub historical_checkpoint_questions: Vec<volicord_context::QuestionReference>,
    pub differs_from_checkpoint_observation: bool,
}

/// Recomputes resume state from canonical Questions and Decisions. A latest
/// Checkpoint is retained only as a historical observation for comparison.
pub fn recompute_frontier_for_resume(
    canonical: &CanonicalReadBasis,
    scope: &InquiryScope,
) -> ResumeFrontier {
    let recomputed = compute_frontier(canonical, scope);
    let historical_checkpoint_questions = canonical
        .latest_checkpoint
        .as_ref()
        .map(|checkpoint| checkpoint.open_questions.clone())
        .unwrap_or_default();
    let current = recomputed
        .questions
        .iter()
        .map(|question| volicord_context::QuestionReference {
            question_id: question.question_id,
            revision: question.displayed_revision,
        })
        .collect::<Vec<_>>();
    let differs_from_checkpoint_observation = current != historical_checkpoint_questions;
    ResumeFrontier {
        recomputed,
        historical_checkpoint_questions,
        differs_from_checkpoint_observation,
    }
}

/// Computes the current Inquiry frontier without mutating canonical or
/// Candidate state. Diagnostics are preserved even when no Question is ready.
pub fn compute_frontier(canonical: &CanonicalReadBasis, scope: &InquiryScope) -> FrontierRead {
    let mut diagnostics = Vec::new();
    if canonical.project.id != scope.project_id {
        return FrontierRead {
            project_id: scope.project_id,
            questions: Vec::new(),
            diagnostics: vec![FrontierDiagnostic {
                kind: FrontierDiagnosticKind::InvalidDependencyBasis,
                question_id: QuestionId::from_bytes([0; 16]),
                prerequisite_question_id: None,
                detail: "canonical read basis belongs to a different Project".to_owned(),
            }],
        };
    }

    let all = canonical
        .active_questions
        .iter()
        .chain(canonical.terminal_question_history.iter())
        .map(|question| (question.id, question))
        .collect::<BTreeMap<_, _>>();
    let source_ids = canonical
        .sources
        .iter()
        .map(|source| source.source.id)
        .collect::<BTreeSet<_>>();
    let cycle_members = dependency_cycle_members(&all);
    for question_id in &cycle_members {
        diagnostics.push(FrontierDiagnostic {
            kind: FrontierDiagnosticKind::DependencyCycle,
            question_id: *question_id,
            prerequisite_question_id: None,
            detail: "Question dependency graph contains a cycle".to_owned(),
        });
    }

    let mut ready = Vec::new();
    for question in &canonical.active_questions {
        if question.project_id != scope.project_id
            || question.materiality != QuestionMateriality::Material
            || question.state != QuestionState::Open
            || question.research_state == QuestionResearchState::ResearchRequired
            || cycle_members.contains(&question.id)
            || !in_scope(question, scope)
        {
            continue;
        }
        let mut blocked = false;
        for dependency in &question.dependencies {
            let Some(prerequisite) = all.get(&dependency.question_id).copied() else {
                diagnostics.push(FrontierDiagnostic {
                    kind: FrontierDiagnosticKind::MissingPrerequisite,
                    question_id: question.id,
                    prerequisite_question_id: Some(dependency.question_id),
                    detail: "dependency names no current canonical Question".to_owned(),
                });
                blocked = true;
                continue;
            };
            let revision_is_valid = canonical.revisions.iter().any(|basis| {
                basis.record_kind == CanonicalRecordKind::Question
                    && basis.record_identity == dependency.question_id.to_string()
                    && basis.revisions.contains(&dependency.required_revision)
            });
            if !revision_is_valid || prerequisite.revision < dependency.required_revision {
                diagnostics.push(FrontierDiagnostic {
                    kind: FrontierDiagnosticKind::InvalidDependencyRevision,
                    question_id: question.id,
                    prerequisite_question_id: Some(dependency.question_id),
                    detail: format!(
                        "required prerequisite revision {} is not a valid canonical basis",
                        dependency.required_revision
                    ),
                });
                blocked = true;
                continue;
            }
            if dependency
                .assessment_source_basis
                .iter()
                .chain(dependency.required_source_basis.iter())
                .any(|source| !source_ids.contains(source))
            {
                diagnostics.push(FrontierDiagnostic {
                    kind: FrontierDiagnosticKind::InvalidDependencyBasis,
                    question_id: question.id,
                    prerequisite_question_id: Some(dependency.question_id),
                    detail:
                        "dependency assessment or resulting basis references an unavailable Source"
                            .to_owned(),
                });
                blocked = true;
                continue;
            }
            let Some(disposition) = prerequisite.terminal_disposition.as_ref() else {
                diagnostics.push(FrontierDiagnostic {
                    kind: FrontierDiagnosticKind::UnsatisfiedOutcome,
                    question_id: question.id,
                    prerequisite_question_id: Some(dependency.question_id),
                    detail: format!(
                        "prerequisite is open; required outcome is {:?}",
                        dependency.required_outcome
                    ),
                });
                blocked = true;
                continue;
            };
            let outcome_matches = disposition.outcome == dependency.required_outcome;
            let source_matches = dependency
                .required_source_basis
                .iter()
                .all(|source| disposition.source_basis.contains(source));
            if !outcome_matches || !source_matches {
                let outcome_blocked = dependency.blocked_outcomes.contains(&disposition.outcome);
                let superseding = dependency
                    .superseding_outcomes
                    .contains(&disposition.outcome);
                diagnostics.push(FrontierDiagnostic {
                    kind: if outcome_blocked {
                        FrontierDiagnosticKind::BlockedByPrerequisite
                    } else if superseding {
                        FrontierDiagnosticKind::SupersededByPrerequisite
                    } else {
                        FrontierDiagnosticKind::UnsatisfiedOutcome
                    },
                    question_id: question.id,
                    prerequisite_question_id: Some(dependency.question_id),
                    detail: format!(
                        "prerequisite outcome {:?} and Source basis do not satisfy required outcome {:?}",
                        disposition.outcome, dependency.required_outcome
                    ),
                });
                blocked = true;
            }
        }
        if !blocked {
            ready.push(question);
        }
    }

    ready.sort_by_key(|question| (question.presentation_order, question.id));
    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic.question_id,
            diagnostic.prerequisite_question_id,
            diagnostic.kind as u8,
        )
    });
    FrontierRead {
        project_id: scope.project_id,
        questions: ready.into_iter().map(presentation).collect(),
        diagnostics,
    }
}

fn in_scope(question: &Question, scope: &InquiryScope) -> bool {
    scope.material_scope.is_empty()
        || question
            .material_scope
            .iter()
            .any(|value| scope.material_scope.contains(value))
}

fn presentation(question: &Question) -> QuestionPresentation {
    QuestionPresentation {
        question_id: question.id,
        displayed_revision: question.revision,
        prompt_basis: question.prompt_basis.clone(),
        why_it_matters_now: question.why_it_matters_now.clone(),
        material_scope: question.material_scope.clone(),
        established_facts: question.established_facts.clone(),
        alternatives: question.alternatives.clone(),
        recommendation: question.recommendation.clone(),
        trade_offs: question.trade_offs.clone(),
        uncertainty: question.uncertainty.clone(),
        known_limits: question.known_limits.clone(),
        prerequisites: question.dependencies.clone(),
        what_the_answer_unlocks: question.what_the_answer_unlocks.clone(),
        allowed_non_choice_dispositions: question.allowed_non_choice_dispositions.clone(),
    }
}

fn dependency_cycle_members(all: &BTreeMap<QuestionId, &Question>) -> BTreeSet<QuestionId> {
    fn visit(
        current: QuestionId,
        all: &BTreeMap<QuestionId, &Question>,
        visiting: &mut Vec<QuestionId>,
        finished: &mut BTreeSet<QuestionId>,
        cycles: &mut BTreeSet<QuestionId>,
    ) {
        if let Some(position) = visiting.iter().position(|value| *value == current) {
            cycles.extend(visiting[position..].iter().copied());
            return;
        }
        if finished.contains(&current) {
            return;
        }
        visiting.push(current);
        if let Some(question) = all.get(&current) {
            for dependency in &question.dependencies {
                if all.contains_key(&dependency.question_id) {
                    visit(dependency.question_id, all, visiting, finished, cycles);
                }
            }
        }
        let _ = visiting.pop();
        finished.insert(current);
    }

    let mut cycles = BTreeSet::new();
    let mut finished = BTreeSet::new();
    for id in all.keys().copied() {
        visit(id, all, &mut Vec::new(), &mut finished, &mut cycles);
    }
    cycles
}
