use crate::{ErrorKind, QuestionPresentation};
use volicord_context::{
    ApplicabilityScope, Availability, CanonicalReadBasis, CanonicalReadOptions,
    ExplicitQuestionResponse, OperationId, PrincipalKind, ProjectId, QuestionId,
    QuestionResponseDraft, QuestionResponseResult, SourceId, SourcePayload, Store, UserTurnSource,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayedQuestion {
    pub question_id: QuestionId,
    pub revision: u64,
    pub alternative_keys: Vec<String>,
    pub recommendation_key: Option<String>,
}

impl From<&QuestionPresentation> for DisplayedQuestion {
    fn from(value: &QuestionPresentation) -> Self {
        Self {
            question_id: value.question_id,
            revision: value.displayed_revision,
            alternative_keys: value
                .alternatives
                .iter()
                .map(|alternative| alternative.key.clone())
                .collect(),
            recommendation_key: value.recommendation.alternative_key.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResponseMapping {
    ExplicitAlternative {
        alternative_key: String,
        user_rationale: Option<String>,
    },
    ExplicitDelegation {
        delegate_to: String,
        user_rationale: Option<String>,
    },
    Ambiguous,
    RecommendationEcho,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentHostResponse {
    pub project_id: ProjectId,
    pub source_id: SourceId,
    pub host: String,
    pub session: String,
    pub turn: String,
    pub displayed: DisplayedQuestion,
    pub mapping: ResponseMapping,
    pub applicability: ApplicabilityScope,
    pub assumptions: Vec<String>,
    pub revisit_triggers: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseRejection {
    WrongProject,
    MissingQuestion,
    StaleDisplayedRevision,
    TerminalOrSupersededQuestion,
    DisplayBasisMismatch,
    UnverifiedCurrentHostSource,
    AmbiguousResponse,
    RecommendationWithoutUserChoice,
    InvalidExplicitMapping,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResponseInterpretation {
    Accepted(Box<QuestionResponseDraft>),
    Rejected {
        reason: ResponseRejection,
        clarification: String,
    },
}

fn rejected(reason: ResponseRejection, clarification: impl Into<String>) -> ResponseInterpretation {
    ResponseInterpretation::Rejected {
        reason,
        clarification: clarification.into(),
    }
}

/// Interprets only an explicitly linked current-host turn. This read step does
/// not create a Source, Decision, or Question transition.
pub fn interpret_current_host_response(
    canonical: &CanonicalReadBasis,
    response: &CurrentHostResponse,
) -> ResponseInterpretation {
    if canonical.project.id != response.project_id {
        return rejected(
            ResponseRejection::WrongProject,
            "response Project does not match",
        );
    }
    let active = canonical
        .active_questions
        .iter()
        .find(|question| question.id == response.displayed.question_id);
    let question = if let Some(question) = active {
        question
    } else if canonical
        .terminal_question_history
        .iter()
        .any(|question| question.id == response.displayed.question_id)
    {
        return rejected(
            ResponseRejection::TerminalOrSupersededQuestion,
            "the Question no longer accepts a response",
        );
    } else {
        return rejected(
            ResponseRejection::MissingQuestion,
            "Question identity is not current",
        );
    };
    if question.revision != response.displayed.revision {
        return rejected(
            ResponseRejection::StaleDisplayedRevision,
            "display the current Question revision before responding",
        );
    }
    let alternative_keys = question
        .alternatives
        .iter()
        .map(|alternative| alternative.key.clone())
        .collect::<Vec<_>>();
    if alternative_keys != response.displayed.alternative_keys
        || question.recommendation.alternative_key != response.displayed.recommendation_key
    {
        return rejected(
            ResponseRejection::DisplayBasisMismatch,
            "displayed alternatives or recommendation are not the current revision",
        );
    }
    let source = canonical
        .sources
        .iter()
        .find(|basis| basis.source.id == response.source_id);
    let verified = source.is_some_and(|basis| {
        basis.source.project_id == response.project_id
            && basis.source.actor.kind == PrincipalKind::User
            && basis.source.observer.as_ref().is_some_and(|observer| {
                observer.kind == PrincipalKind::Agent && !observer.identity.trim().is_empty()
            })
            && basis.availability == Availability::Available
            && matches!(
                &basis.source.payload,
                SourcePayload::CurrentHostUserTurn { host, session, turn }
                    if host == &response.host
                        && session == &response.session
                        && turn == &response.turn
            )
    });
    if !verified {
        return rejected(
            ResponseRejection::UnverifiedCurrentHostSource,
            "response requires verified host, session, user-turn, and observer provenance",
        );
    }
    let explicit = match &response.mapping {
        ResponseMapping::ExplicitAlternative {
            alternative_key,
            user_rationale,
        } if alternative_keys.contains(alternative_key) => ExplicitQuestionResponse::Choice {
            alternative_key: alternative_key.clone(),
            user_rationale: user_rationale.clone(),
        },
        ResponseMapping::ExplicitDelegation {
            delegate_to,
            user_rationale,
        } if !delegate_to.trim().is_empty() => ExplicitQuestionResponse::Delegation {
            delegate_to: delegate_to.clone(),
            user_rationale: user_rationale.clone(),
        },
        ResponseMapping::Ambiguous => {
            return rejected(
                ResponseRejection::AmbiguousResponse,
                "identify one displayed alternative or an explicit delegate",
            )
        }
        ResponseMapping::RecommendationEcho => {
            return rejected(
                ResponseRejection::RecommendationWithoutUserChoice,
                "an echoed recommendation is not an explicit user choice",
            )
        }
        _ => {
            return rejected(
                ResponseRejection::InvalidExplicitMapping,
                "the explicit response does not match a displayed alternative or delegate",
            )
        }
    };
    ResponseInterpretation::Accepted(Box::new(QuestionResponseDraft {
        expected_project_revision: canonical.project.revision,
        question_id: question.id,
        question_revision: question.revision,
        user_turn_source: UserTurnSource::Existing(response.source_id),
        displayed_alternative_keys: alternative_keys,
        displayed_recommendation_key: question.recommendation.alternative_key.clone(),
        response: explicit,
        applicability: response.applicability.clone(),
        assumptions: response.assumptions.clone(),
        revisit_triggers: response.revisit_triggers.clone(),
    }))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchResponseItem {
    pub operation_id: OperationId,
    pub response: CurrentHostResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchResponseOutcome {
    Succeeded(QuestionResponseResult),
    Replayed(QuestionResponseResult),
    Rejected {
        reason: ResponseRejection,
        clarification: String,
    },
    Failed {
        kind: ErrorKind,
        message: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchResponseResult {
    pub project_id: ProjectId,
    pub items: Vec<(QuestionId, u64, BatchResponseOutcome)>,
}

impl BatchResponseResult {
    pub fn all_succeeded(&self) -> bool {
        self.items.iter().all(|(_, _, outcome)| {
            matches!(
                outcome,
                BatchResponseOutcome::Succeeded(_) | BatchResponseOutcome::Replayed(_)
            )
        })
    }
}

/// Processes one current-host turn as independent per-Question Kernel
/// operations. A stale or failed item never rolls back or disguises another
/// item's committed result.
pub fn record_response_batch(
    context: &mut Store,
    project_id: ProjectId,
    items: Vec<BatchResponseItem>,
) -> BatchResponseResult {
    let mut outcomes = Vec::with_capacity(items.len());
    for item in items {
        let identity = item.response.displayed.question_id;
        let revision = item.response.displayed.revision;
        let canonical = context.read_canonical_basis(project_id, CanonicalReadOptions::default());
        let outcome = match canonical {
            Ok(canonical) => match interpret_current_host_response(&canonical, &item.response) {
                ResponseInterpretation::Rejected {
                    reason,
                    clarification,
                } => {
                    if reason == ResponseRejection::TerminalOrSupersededQuestion {
                        replay_terminal_response(
                            context,
                            &canonical,
                            item.operation_id,
                            project_id,
                            &item.response,
                        )
                        .unwrap_or(BatchResponseOutcome::Rejected {
                            reason,
                            clarification,
                        })
                    } else {
                        BatchResponseOutcome::Rejected {
                            reason,
                            clarification,
                        }
                    }
                }
                ResponseInterpretation::Accepted(draft) => {
                    match context.record_question_response(item.operation_id, project_id, *draft) {
                        Ok(result) if result.replayed => {
                            BatchResponseOutcome::Replayed(result.value)
                        }
                        Ok(result) => BatchResponseOutcome::Succeeded(result.value),
                        Err(error) => BatchResponseOutcome::Failed {
                            kind: map_context_error(error.kind()),
                            message: error.to_string(),
                        },
                    }
                }
            },
            Err(error) => BatchResponseOutcome::Failed {
                kind: map_context_error(error.kind()),
                message: error.to_string(),
            },
        };
        outcomes.push((identity, revision, outcome));
    }
    BatchResponseResult {
        project_id,
        items: outcomes,
    }
}

fn replay_terminal_response(
    context: &mut Store,
    canonical: &CanonicalReadBasis,
    operation_id: OperationId,
    project_id: ProjectId,
    response: &CurrentHostResponse,
) -> Option<BatchResponseOutcome> {
    let terminal = canonical
        .terminal_question_history
        .iter()
        .find(|question| question.id == response.displayed.question_id)?
        .clone();
    let mut replay_basis = canonical.clone();
    replay_basis
        .terminal_question_history
        .retain(|question| question.id != terminal.id);
    replay_basis.active_questions.push(terminal);
    let ResponseInterpretation::Accepted(draft) =
        interpret_current_host_response(&replay_basis, response)
    else {
        return None;
    };
    match context.record_question_response(operation_id, project_id, *draft) {
        Ok(result) if result.replayed => Some(BatchResponseOutcome::Replayed(result.value)),
        _ => None,
    }
}

fn map_context_error(kind: volicord_context::ErrorKind) -> ErrorKind {
    use volicord_context::ErrorKind as Context;
    match kind {
        Context::InvalidInput => ErrorKind::InvalidInput,
        Context::NotFound => ErrorKind::NotFound,
        Context::WrongProject => ErrorKind::WrongProject,
        Context::StaleBasis => ErrorKind::StaleBasis,
        Context::UnsupportedVersion => ErrorKind::UnsupportedVersion,
        Context::AlreadyExists | Context::DomainConflict => ErrorKind::DomainConflict,
        Context::CorruptState | Context::IntegrityFailure | Context::RepairRequired => {
            ErrorKind::CorruptState
        }
        Context::StorageUnavailable => ErrorKind::StorageUnavailable,
        Context::TransactionFailure | Context::IndeterminateOutcome => {
            ErrorKind::TransactionFailure
        }
    }
}
