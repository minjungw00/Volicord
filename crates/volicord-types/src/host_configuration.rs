//! Generic managed host configuration selectors.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Connection intent for managed configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionIntent {
    /// User-owned configuration serving a personal connection.
    Personal,
    /// Project-owned configuration serving a shared connection.
    Shared,
}

impl ConnectionIntent {
    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Shared => "shared",
        }
    }
}

/// Owner of a managed host configuration target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HostScope {
    /// User-owned personal configuration.
    User,
    /// Project-owned shared configuration.
    Project,
}

impl HostScope {
    /// Returns the exact wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }
}
