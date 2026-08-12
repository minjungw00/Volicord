use crate::{LocalBindingId, ProjectId, SourceId, TimestampMicros};
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
