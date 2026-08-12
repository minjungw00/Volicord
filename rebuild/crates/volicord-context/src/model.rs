use crate::{
    CheckpointId, ContextItemId, DecisionId, LocalBindingId, ProjectId, QuestionId, SourceId,
    TimestampMicros,
};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Availability {
    Available,
    Unavailable,
    Stale,
    Unknown,
}

impl Availability {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "available" => Some(Self::Available),
            "unavailable" => Some(Self::Unavailable),
            "stale" => Some(Self::Stale),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrincipalKind {
    User,
    Agent,
    Repository,
    Command,
    Provider,
    Generator,
    Importer,
}

impl PrincipalKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
            Self::Repository => "repository",
            Self::Command => "command",
            Self::Provider => "provider",
            Self::Generator => "generator",
            Self::Importer => "importer",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "agent" => Some(Self::Agent),
            "repository" => Some(Self::Repository),
            "command" => Some(Self::Command),
            "provider" => Some(Self::Provider),
            "generator" => Some(Self::Generator),
            "importer" => Some(Self::Importer),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Principal {
    pub kind: PrincipalKind,
    pub identity: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandTermination {
    Exited,
    Signaled,
    SpawnFailed,
    Indeterminate,
}

impl CommandTermination {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Exited => "exited",
            Self::Signaled => "signaled",
            Self::SpawnFailed => "spawn_failed",
            Self::Indeterminate => "indeterminate",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "exited" => Some(Self::Exited),
            "signaled" => Some(Self::Signaled),
            "spawn_failed" => Some(Self::SpawnFailed),
            "indeterminate" => Some(Self::Indeterminate),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandOutcome {
    pub exit_code: Option<i32>,
    pub termination: CommandTermination,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourcePayload {
    RepositorySnapshot {
        revision: String,
    },
    RepositoryCommit {
        commit: String,
    },
    File {
        locator: String,
        snapshot: String,
    },
    Symbol {
        locator: String,
        snapshot: String,
    },
    CommandExecution {
        command_label: String,
        outcome: CommandOutcome,
    },
    CurrentHostUserTurn {
        host: String,
        session: String,
        turn: String,
    },
    Url {
        url: String,
    },
    AdoptedArtifact {
        locator: String,
        revision: String,
    },
}

impl SourcePayload {
    pub(crate) const fn kind(&self) -> &'static str {
        match self {
            Self::RepositorySnapshot { .. } => "repository_snapshot",
            Self::RepositoryCommit { .. } => "repository_commit",
            Self::File { .. } => "file",
            Self::Symbol { .. } => "symbol",
            Self::CommandExecution { .. } => "command_execution",
            Self::CurrentHostUserTurn { .. } => "current_host_user_turn",
            Self::Url { .. } => "url",
            Self::AdoptedArtifact { .. } => "adopted_artifact",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDraft {
    pub expected_project_revision: u64,
    pub payload: SourcePayload,
    pub actor: Principal,
    pub observer: Option<Principal>,
    pub availability: Availability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub id: ProjectId,
    pub display_name: String,
    pub revision: u64,
    pub created_at: TimestampMicros,
    pub updated_at: TimestampMicros,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalBinding {
    pub id: LocalBindingId,
    pub project_id: ProjectId,
    pub absolute_path: PathBuf,
    pub availability: Availability,
    pub revision: u64,
    pub bound_at: TimestampMicros,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Source {
    pub id: SourceId,
    pub project_id: ProjectId,
    pub payload: SourcePayload,
    pub actor: Principal,
    pub observer: Option<Principal>,
    pub availability: Availability,
    pub recorded_at: TimestampMicros,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceRelationKind {
    DerivedFrom,
    SupportedBy,
}

impl SourceRelationKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::DerivedFrom => "derived_from",
            Self::SupportedBy => "supported_by",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "derived_from" => Some(Self::DerivedFrom),
            "supported_by" => Some(Self::SupportedBy),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRelation {
    pub project_id: ProjectId,
    pub from_source_id: SourceId,
    pub kind: SourceRelationKind,
    pub to_source_id: SourceId,
    pub recorded_at: TimestampMicros,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationResult<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestionDependency {
    pub question_id: QuestionId,
    pub required_revision: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestionAlternative {
    pub key: String,
    pub label: String,
    pub consequence: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentRecommendation {
    pub alternative_key: Option<String>,
    pub rationale: String,
    pub source_basis: Vec<SourceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestionDraft {
    pub expected_project_revision: u64,
    pub prompt_basis: String,
    pub source_basis: Vec<SourceId>,
    pub dependencies: Vec<QuestionDependency>,
    pub alternatives: Vec<QuestionAlternative>,
    pub recommendation: AgentRecommendation,
    pub trade_offs: Vec<String>,
    pub uncertainty: Vec<String>,
    pub material_scope: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuestionTerminalOutcome {
    Answered,
    Delegated,
    ResolvedByResearch,
    RequiresPrototype,
    Deferred,
    OutOfScope,
    Superseded,
}

impl QuestionTerminalOutcome {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Answered => "answered",
            Self::Delegated => "delegated",
            Self::ResolvedByResearch => "resolved_by_research",
            Self::RequiresPrototype => "requires_prototype",
            Self::Deferred => "deferred",
            Self::OutOfScope => "out_of_scope",
            Self::Superseded => "superseded",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "answered" => Some(Self::Answered),
            "delegated" => Some(Self::Delegated),
            "resolved_by_research" => Some(Self::ResolvedByResearch),
            "requires_prototype" => Some(Self::RequiresPrototype),
            "deferred" => Some(Self::Deferred),
            "out_of_scope" => Some(Self::OutOfScope),
            "superseded" => Some(Self::Superseded),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuestionState {
    Open,
    Terminal(QuestionTerminalOutcome),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Question {
    pub id: QuestionId,
    pub project_id: ProjectId,
    pub revision: u64,
    pub prompt_basis: String,
    pub source_basis: Vec<SourceId>,
    pub dependencies: Vec<QuestionDependency>,
    pub alternatives: Vec<QuestionAlternative>,
    pub recommendation: AgentRecommendation,
    pub trade_offs: Vec<String>,
    pub uncertainty: Vec<String>,
    pub material_scope: Vec<String>,
    pub state: QuestionState,
    pub created_at: TimestampMicros,
    pub updated_at: TimestampMicros,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ApplicabilityScope {
    pub paths: Vec<String>,
    pub components: Vec<String>,
    pub work_contexts: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionChoice {
    Alternative { alternative_key: String },
    Delegation { delegate_to: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decision {
    pub id: DecisionId,
    pub project_id: ProjectId,
    pub question_id: QuestionId,
    pub question_revision: u64,
    pub user_turn_source_id: SourceId,
    pub choice: DecisionChoice,
    pub user_rationale: Option<String>,
    pub displayed_alternatives: Vec<QuestionAlternative>,
    pub displayed_recommendation: AgentRecommendation,
    pub applicability: ApplicabilityScope,
    pub assumptions: Vec<String>,
    pub revisit_triggers: Vec<String>,
    pub recorded_at: TimestampMicros,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UserTurnSource {
    Existing(SourceId),
    Create(SourceDraft),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExplicitQuestionResponse {
    Choice {
        alternative_key: String,
        user_rationale: Option<String>,
    },
    Delegation {
        delegate_to: String,
        user_rationale: Option<String>,
    },
    Terminal {
        outcome: QuestionTerminalOutcome,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestionResponseDraft {
    pub expected_project_revision: u64,
    pub question_id: QuestionId,
    pub question_revision: u64,
    pub user_turn_source: UserTurnSource,
    pub displayed_alternative_keys: Vec<String>,
    pub displayed_recommendation_key: Option<String>,
    pub response: ExplicitQuestionResponse,
    pub applicability: ApplicabilityScope,
    pub assumptions: Vec<String>,
    pub revisit_triggers: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestionResponseResult {
    pub question: Question,
    pub user_turn_source: Source,
    pub decision: Option<Decision>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextItemRole {
    Goal,
    Fact,
    Assumption,
    Constraint,
    Preference,
    Risk,
    Learning,
    KnownLimit,
}

impl ContextItemRole {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Goal => "goal",
            Self::Fact => "fact",
            Self::Assumption => "assumption",
            Self::Constraint => "constraint",
            Self::Preference => "preference",
            Self::Risk => "risk",
            Self::Learning => "learning",
            Self::KnownLimit => "known_limit",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "goal" => Some(Self::Goal),
            "fact" => Some(Self::Fact),
            "assumption" => Some(Self::Assumption),
            "constraint" => Some(Self::Constraint),
            "preference" => Some(Self::Preference),
            "risk" => Some(Self::Risk),
            "learning" => Some(Self::Learning),
            "known_limit" => Some(Self::KnownLimit),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatementProvenanceRole {
    UserStatement,
    Observed,
    AgentStatement,
    GeneratedInterpretation,
}

impl StatementProvenanceRole {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::UserStatement => "user_statement",
            Self::Observed => "observed",
            Self::AgentStatement => "agent_statement",
            Self::GeneratedInterpretation => "generated_interpretation",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "user_statement" => Some(Self::UserStatement),
            "observed" => Some(Self::Observed),
            "agent_statement" => Some(Self::AgentStatement),
            "generated_interpretation" => Some(Self::GeneratedInterpretation),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextItemDraft {
    pub expected_project_revision: u64,
    pub role: ContextItemRole,
    pub statement: String,
    pub provenance_role: StatementProvenanceRole,
    pub author: Principal,
    pub source_basis: Vec<SourceId>,
    pub applicability: ApplicabilityScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextItem {
    pub id: ContextItemId,
    pub project_id: ProjectId,
    pub revision: u64,
    pub role: ContextItemRole,
    pub statement: String,
    pub provenance_role: StatementProvenanceRole,
    pub author: Principal,
    pub source_basis: Vec<SourceId>,
    pub applicability: ApplicabilityScope,
    pub recorded_at: TimestampMicros,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointKind {
    Completion,
    Pause,
    Handoff,
}

impl CheckpointKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Completion => "completion",
            Self::Pause => "pause",
            Self::Handoff => "handoff",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "completion" => Some(Self::Completion),
            "pause" => Some(Self::Pause),
            "handoff" => Some(Self::Handoff),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkState {
    InProgress,
    Paused,
    Completed,
    Abandoned,
    Superseded,
}

impl WorkState {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
            Self::Superseded => "superseded",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "in_progress" => Some(Self::InProgress),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            "abandoned" => Some(Self::Abandoned),
            "superseded" => Some(Self::Superseded),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationState {
    NotRun,
    Partial,
    Passed,
    Failed,
}

impl VerificationState {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::NotRun => "not_run",
            Self::Partial => "partial",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "not_run" => Some(Self::NotRun),
            "partial" => Some(Self::Partial),
            "passed" => Some(Self::Passed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserReviewState {
    NotRequested,
    Pending,
    Reviewed,
}

impl UserReviewState {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::NotRequested => "not_requested",
            Self::Pending => "pending",
            Self::Reviewed => "reviewed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "not_requested" => Some(Self::NotRequested),
            "pending" => Some(Self::Pending),
            "reviewed" => Some(Self::Reviewed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserAcceptanceState {
    NotRequested,
    Pending,
    Accepted,
    Rejected,
}

impl UserAcceptanceState {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::NotRequested => "not_requested",
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "not_requested" => Some(Self::NotRequested),
            "pending" => Some(Self::Pending),
            "accepted" => Some(Self::Accepted),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationFact {
    pub state: VerificationState,
    pub source_id: Option<SourceId>,
    pub outcome: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserReviewFact {
    pub state: UserReviewState,
    pub source_id: Option<SourceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserAcceptanceFact {
    pub state: UserAcceptanceState,
    pub source_id: Option<SourceId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuestionReference {
    pub question_id: QuestionId,
    pub revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointDraft {
    pub expected_project_revision: u64,
    pub kind: CheckpointKind,
    pub goal: String,
    pub work_state: WorkState,
    pub state_change: Option<String>,
    pub source_basis: Vec<SourceId>,
    pub changed_source_basis: Vec<SourceId>,
    pub changed_paths: Vec<String>,
    pub applied_decisions: Vec<DecisionId>,
    pub verification: Vec<VerificationFact>,
    pub user_review: UserReviewFact,
    pub user_acceptance: UserAcceptanceFact,
    pub known_limits: Vec<String>,
    pub non_goals: Vec<String>,
    pub open_questions: Vec<QuestionReference>,
    pub next_step: String,
    pub handoff_to: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub project_id: ProjectId,
    pub revision: u64,
    pub kind: CheckpointKind,
    pub goal: String,
    pub work_state: WorkState,
    pub state_change: Option<String>,
    pub source_basis: Vec<SourceId>,
    pub changed_source_basis: Vec<SourceId>,
    pub changed_paths: Vec<String>,
    pub applied_decisions: Vec<DecisionId>,
    pub verification: Vec<VerificationFact>,
    pub user_review: UserReviewFact,
    pub user_acceptance: UserAcceptanceFact,
    pub known_limits: Vec<String>,
    pub non_goals: Vec<String>,
    pub open_questions: Vec<QuestionReference>,
    pub next_step: String,
    pub handoff_to: Option<String>,
    pub recorded_at: TimestampMicros,
}
