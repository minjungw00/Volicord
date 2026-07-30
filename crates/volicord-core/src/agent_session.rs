//! Core validation for current managed Agent Sessions.

use std::{error::Error, fmt};

use volicord_store::{
    agent_connections::{agent_connection_record_read_only, is_agent_connection_project_allowed},
    guards::{agent_session, agent_session_matches_current_integration, list_guard_installations},
    operational_sessions::{
        current_managed_mcp_runtime_session_for_connection, mcp_runtime_project_session_binding,
    },
    StoreError, StoreFailureRoute,
};
use volicord_types::ids::{AgentConnectionId, AgentRuntimeSessionId, AgentSessionId, ProjectId};
use volicord_types::integration_revision::IntegrationRevision;
use volicord_types::values::{AgentConnectionMode, FailureCategory, OperationCategory};

use crate::CoreService;

/// Current operational Agent Session validated by Core for one operation category.
///
/// This type is deliberately not serializable. Adapters can obtain it only by
/// asking Core to compare the supplied coordinates with current Store authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedAgentSession {
    connection_id: AgentConnectionId,
    project_id: ProjectId,
    runtime_session_id: AgentRuntimeSessionId,
    project_session_id: AgentSessionId,
    integration_revision: IntegrationRevision,
}

impl ValidatedAgentSession {
    /// Returns the Connection that owns this session.
    pub const fn connection_id(&self) -> &AgentConnectionId {
        &self.connection_id
    }

    /// Returns the project authorized for this session.
    pub const fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    /// Returns the managed runtime-session coordinate.
    pub const fn runtime_session_id(&self) -> &AgentRuntimeSessionId {
        &self.runtime_session_id
    }

    /// Returns the project-scoped Agent Session coordinate.
    pub const fn project_session_id(&self) -> &AgentSessionId {
        &self.project_session_id
    }

    /// Returns the current project integration revision.
    pub const fn integration_revision(&self) -> &IntegrationRevision {
        &self.integration_revision
    }

    pub(crate) fn verification_basis(&self) -> String {
        format!(
            "connection:{}/session:{}/revision:{}",
            self.connection_id,
            self.project_session_id,
            self.integration_revision.as_str()
        )
    }
}

/// Machine-readable failure from current Agent Session validation.
#[derive(Debug)]
pub struct AgentSessionValidationError {
    category: FailureCategory,
    reason: &'static str,
    source: Option<StoreError>,
}

impl AgentSessionValidationError {
    fn rejected(reason: &'static str) -> Self {
        Self {
            category: FailureCategory::Rejected,
            reason,
            source: None,
        }
    }

    fn store(error: StoreError) -> Self {
        let category = match error.classification().route {
            StoreFailureRoute::PersistedDataCorrupt => FailureCategory::Corrupt,
            StoreFailureRoute::InvalidEnvironment => FailureCategory::Rejected,
            StoreFailureRoute::InvocationContextMismatch => FailureCategory::Rejected,
            StoreFailureRoute::OperationalUnavailable => FailureCategory::Unavailable,
        };
        Self {
            category,
            reason: "agent_session_authority_unavailable",
            source: Some(error),
        }
    }

    /// Returns the product-wide failure category.
    pub const fn category(&self) -> FailureCategory {
        self.category
    }

    /// Returns the stable domain reason.
    pub const fn reason(&self) -> &'static str {
        self.reason
    }
}

impl fmt::Display for AgentSessionValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason)
    }
}

impl Error for AgentSessionValidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|error| error as &dyn Error)
    }
}

impl CoreService {
    /// Validates current Connection, membership, mode, runtime, project session,
    /// and integration revisions for one Agent operation.
    pub fn validate_agent_session(
        &self,
        connection_id: AgentConnectionId,
        project_id: ProjectId,
        runtime_session_id: AgentRuntimeSessionId,
        project_session_id: AgentSessionId,
        operation_category: OperationCategory,
    ) -> Result<ValidatedAgentSession, AgentSessionValidationError> {
        let connection =
            agent_connection_record_read_only(self.runtime_home(), connection_id.as_str())
                .map_err(AgentSessionValidationError::store)?
                .ok_or_else(|| {
                    AgentSessionValidationError::rejected("agent_connection_not_current")
                })?;
        if !connection.enabled {
            return Err(AgentSessionValidationError::rejected(
                "agent_connection_not_enabled",
            ));
        }
        let mode = match connection.mode.as_str() {
            "read_only" => AgentConnectionMode::ReadOnly,
            "workflow" => AgentConnectionMode::Workflow,
            _ => {
                return Err(AgentSessionValidationError::rejected(
                    "agent_connection_mode_invalid",
                ))
            }
        };
        if !mode.allows_operation_category(operation_category) {
            return Err(AgentSessionValidationError::rejected(
                "agent_connection_mode_not_allowed",
            ));
        }
        if !is_agent_connection_project_allowed(
            self.runtime_home(),
            connection_id.as_str(),
            project_id.as_str(),
        )
        .map_err(AgentSessionValidationError::store)?
        {
            return Err(AgentSessionValidationError::rejected(
                "connection_project_not_current",
            ));
        }

        let runtime = current_managed_mcp_runtime_session_for_connection(
            self.runtime_home(),
            runtime_session_id.as_str(),
            connection_id.as_str(),
        )
        .map_err(|error| match error {
            StoreError::NotFound { .. } | StoreError::Conflict { .. } => {
                AgentSessionValidationError::rejected("agent_runtime_session_not_current")
            }
            error => AgentSessionValidationError::store(error),
        })?;
        if runtime.terminal_finding_id.is_some() || runtime.graceful_close_at.is_some() {
            return Err(AgentSessionValidationError::rejected(
                "agent_runtime_session_terminal",
            ));
        }

        let session = agent_session(
            self.runtime_home(),
            project_id.as_str(),
            project_session_id.as_str(),
        )
        .map_err(AgentSessionValidationError::store)?
        .ok_or_else(|| {
            AgentSessionValidationError::rejected("agent_project_session_not_current")
        })?;
        let Some(bound_runtime_session_id) = session.runtime_session_id.as_deref() else {
            return Err(AgentSessionValidationError::rejected(
                "agent_project_session_unbound",
            ));
        };
        if session.project_id != project_id.as_str()
            || session.connection_internal_id != connection_id.as_str()
            || bound_runtime_session_id != runtime_session_id.as_str()
        {
            return Err(AgentSessionValidationError::rejected(
                "agent_project_session_scope_mismatch",
            ));
        }
        let binding = mcp_runtime_project_session_binding(
            self.runtime_home(),
            project_id.as_str(),
            project_session_id.as_str(),
        )
        .map_err(AgentSessionValidationError::store)?
        .ok_or_else(|| {
            AgentSessionValidationError::rejected("agent_project_session_binding_missing")
        })?;
        if binding.runtime_session_id != runtime_session_id.as_str()
            || binding.connection_internal_id != connection_id.as_str()
            || binding.project_id != project_id.as_str()
            || binding.session_id != project_session_id.as_str()
            || binding.project_integration_revision != session.project_integration_revision
            || binding.host_session_id != session.host_session_id
        {
            return Err(AgentSessionValidationError::rejected(
                "agent_project_session_binding_mismatch",
            ));
        }
        let guard_installations = list_guard_installations(
            self.runtime_home(),
            connection_id.as_str(),
            Some(project_id.as_str()),
        )
        .map_err(AgentSessionValidationError::store)?;
        let guard_installation_id = match guard_installations.as_slice() {
            [] => None,
            [installation] => Some(installation.guard_installation_id.as_str()),
            _ => {
                return Err(AgentSessionValidationError::rejected(
                    "agent_project_guard_ownership_ambiguous",
                ))
            }
        };
        let current = agent_session_matches_current_integration(
            self.runtime_home(),
            &session,
            guard_installation_id,
        )
        .map_err(AgentSessionValidationError::store)?;
        if !current {
            return Err(AgentSessionValidationError::rejected(
                "agent_project_session_revision_stale",
            ));
        }
        let integration_revision = IntegrationRevision::parse(session.project_integration_revision)
            .map_err(|_| {
                AgentSessionValidationError::rejected("agent_project_session_revision_invalid")
            })?;

        Ok(ValidatedAgentSession {
            connection_id,
            project_id,
            runtime_session_id,
            project_session_id,
            integration_revision,
        })
    }
}

#[cfg(test)]
pub(crate) fn validated_agent_session_for_test(
    connection_id: &str,
    project_id: &str,
) -> ValidatedAgentSession {
    validated_agent_session_for_test_with_project_session(
        connection_id,
        project_id,
        "agent_test_project_session",
    )
}

#[cfg(test)]
pub(crate) fn validated_agent_session_for_test_with_project_session(
    connection_id: &str,
    project_id: &str,
    project_session_id: &str,
) -> ValidatedAgentSession {
    let integration_revision = IntegrationRevision::for_project(
        volicord_types::integration_revision::ProjectIntegrationRevisionBasis {
            connection_integration_revision:
                "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            project_id,
            policy_fingerprint:
                "sha256:2222222222222222222222222222222222222222222222222222222222222222",
            guard_installation_id: None,
            guard_policy_hash: None,
            repository_observer_contract_digest:
                "sha256:3333333333333333333333333333333333333333333333333333333333333333",
            product_repository_effect_catalog_digest:
                "sha256:4444444444444444444444444444444444444444444444444444444444444444",
        },
    )
    .expect("test project revision must be valid");
    ValidatedAgentSession {
        connection_id: AgentConnectionId::new(connection_id),
        project_id: ProjectId::new(project_id),
        runtime_session_id: AgentRuntimeSessionId::new("mcp_test_runtime_session"),
        project_session_id: AgentSessionId::new(project_session_id),
        integration_revision,
    }
}
