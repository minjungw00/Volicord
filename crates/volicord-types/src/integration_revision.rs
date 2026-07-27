//! Typed, canonical integration-revision identities.

use std::{error::Error, fmt};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    canonical::{canonical_json_sha256, is_canonical_sha256_digest},
    ids::ConnectionIntegrationInstanceId,
};

const CONNECTION_REVISION_DOMAIN: &str = "volicord.connection-integration-revision";
const PROJECT_REVISION_DOMAIN: &str = "volicord.project-integration-revision";

/// Origin of one authoritative MCP runtime session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpRuntimeSessionSource {
    /// MCP process authorized by a consumed managed-host launch lease.
    ManagedHost,
    /// MCP process launched through the public manual stdio command.
    ManualCli,
    /// MCP process launched by the CLI preflight path.
    CliPreflight,
    /// MCP process launched by a dedicated integration probe.
    IntegrationProbe,
}

impl McpRuntimeSessionSource {
    /// Returns the exact persisted value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ManagedHost => "managed_host",
            Self::ManualCli => "manual_cli",
            Self::CliPreflight => "cli_preflight",
            Self::IntegrationProbe => "integration_probe",
        }
    }
}

#[cfg(test)]
mod runtime_session_source_tests {
    use super::McpRuntimeSessionSource;

    #[test]
    fn runtime_session_sources_have_exact_current_values() {
        assert_eq!(
            McpRuntimeSessionSource::ManagedHost.as_str(),
            "managed_host"
        );
        assert_eq!(McpRuntimeSessionSource::ManualCli.as_str(), "manual_cli");
        assert_eq!(
            McpRuntimeSessionSource::CliPreflight.as_str(),
            "cli_preflight"
        );
        assert_eq!(
            McpRuntimeSessionSource::IntegrationProbe.as_str(),
            "integration_probe"
        );
    }
}

/// Current connection-owned inputs that define one managed integration revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectionIntegrationRevisionBasis<'a> {
    pub connection_internal_id: &'a str,
    pub integration_instance_id: &'a ConnectionIntegrationInstanceId,
    pub host_kind: &'a str,
    pub intent: &'a str,
    pub host_scope: &'a str,
    pub mode: &'a str,
    pub server_name: &'a str,
    pub config_target: &'a str,
    pub managed_configuration_fingerprint: &'a str,
    pub integration_generation: i64,
}

/// Current project-owned additions to a connection integration revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectIntegrationRevisionBasis<'a> {
    pub connection_integration_revision: &'a str,
    pub project_id: &'a str,
    pub policy_fingerprint: &'a str,
    pub guard_installation_id: Option<&'a str>,
    pub guard_policy_hash: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct DomainRevisionBasis<'a, T> {
    domain: &'a str,
    basis: T,
}

/// A validated canonical SHA-256 integration revision.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct IntegrationRevision(String);

impl IntegrationRevision {
    /// Constructs the current connection integration revision deterministically.
    pub fn for_connection(
        basis: ConnectionIntegrationRevisionBasis<'_>,
    ) -> Result<Self, IntegrationRevisionError> {
        validate_connection_basis(&basis)?;
        digest(CONNECTION_REVISION_DOMAIN, basis)
    }

    /// Constructs the current project integration revision deterministically.
    pub fn for_project(
        basis: ProjectIntegrationRevisionBasis<'_>,
    ) -> Result<Self, IntegrationRevisionError> {
        validate_project_basis(&basis)?;
        digest(PROJECT_REVISION_DOMAIN, basis)
    }

    /// Validates and retains a persisted integration revision.
    pub fn parse(value: impl Into<String>) -> Result<Self, IntegrationRevisionError> {
        let value = value.into();
        if !is_canonical_sha256_digest(&value) {
            return Err(IntegrationRevisionError::InvalidDigest);
        }
        Ok(Self(value))
    }

    /// Returns the canonical digest string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns its canonical digest string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// Failure to construct or decode a typed integration revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationRevisionError {
    EmptyField(&'static str),
    InvalidGeneration,
    InvalidDigest,
    CanonicalEncoding,
}

impl fmt::Display for IntegrationRevisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(field) => write!(
                formatter,
                "integration revision field {field} must not be empty"
            ),
            Self::InvalidGeneration => {
                formatter.write_str("integration revision generation must be nonnegative")
            }
            Self::InvalidDigest => {
                formatter.write_str("integration revision digest must be canonical sha256")
            }
            Self::CanonicalEncoding => {
                formatter.write_str("integration revision basis could not be canonically encoded")
            }
        }
    }
}

impl Error for IntegrationRevisionError {}

fn digest<T: Serialize>(
    domain: &'static str,
    basis: T,
) -> Result<IntegrationRevision, IntegrationRevisionError> {
    canonical_json_sha256(&DomainRevisionBasis { domain, basis })
        .map(|digest| IntegrationRevision(digest.into_inner()))
        .map_err(|_| IntegrationRevisionError::CanonicalEncoding)
}

fn require_nonempty(field: &'static str, value: &str) -> Result<(), IntegrationRevisionError> {
    if value.is_empty() || value.as_bytes().contains(&0) {
        Err(IntegrationRevisionError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn validate_connection_basis(
    basis: &ConnectionIntegrationRevisionBasis<'_>,
) -> Result<(), IntegrationRevisionError> {
    for (field, value) in [
        ("connection_internal_id", basis.connection_internal_id),
        ("host_kind", basis.host_kind),
        ("intent", basis.intent),
        ("host_scope", basis.host_scope),
        ("mode", basis.mode),
        ("server_name", basis.server_name),
        ("config_target", basis.config_target),
        (
            "managed_configuration_fingerprint",
            basis.managed_configuration_fingerprint,
        ),
    ] {
        require_nonempty(field, value)?;
    }
    if basis.integration_generation < 0 {
        return Err(IntegrationRevisionError::InvalidGeneration);
    }
    Ok(())
}

fn validate_project_basis(
    basis: &ProjectIntegrationRevisionBasis<'_>,
) -> Result<(), IntegrationRevisionError> {
    IntegrationRevision::parse(basis.connection_integration_revision)?;
    require_nonempty("project_id", basis.project_id)?;
    if !is_canonical_sha256_digest(basis.policy_fingerprint) {
        return Err(IntegrationRevisionError::InvalidDigest);
    }
    match (basis.guard_installation_id, basis.guard_policy_hash) {
        (None, None) => Ok(()),
        (Some(installation_id), Some(policy_hash)) => {
            require_nonempty("guard_installation_id", installation_id)?;
            if is_canonical_sha256_digest(policy_hash) {
                Ok(())
            } else {
                Err(IntegrationRevisionError::InvalidDigest)
            }
        }
        _ => Err(IntegrationRevisionError::EmptyField("guard_ownership_pair")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn integration_instance_id() -> ConnectionIntegrationInstanceId {
        ConnectionIntegrationInstanceId::parse(
            "connection_instance_00112233-4455-4abb-8cdd-eeff10203040",
        )
        .unwrap()
    }

    fn connection_basis<'a>(
        fingerprint: &'a str,
        integration_instance_id: &'a ConnectionIntegrationInstanceId,
    ) -> ConnectionIntegrationRevisionBasis<'a> {
        ConnectionIntegrationRevisionBasis {
            connection_internal_id: "connection.alpha",
            integration_instance_id,
            host_kind: "codex",
            intent: "personal",
            host_scope: "project",
            mode: "workflow",
            server_name: "volicord",
            config_target: "/config/config.toml",
            managed_configuration_fingerprint: fingerprint,
            integration_generation: 0,
        }
    }

    #[test]
    fn current_integration_revision_is_deterministic() {
        let instance = integration_instance_id();
        let first = IntegrationRevision::for_connection(connection_basis("managed:a", &instance))
            .expect("valid connection basis");
        let replay = IntegrationRevision::for_connection(connection_basis("managed:a", &instance))
            .expect("same basis");
        assert_eq!(first, replay);
        assert_ne!(
            first,
            IntegrationRevision::for_connection(connection_basis("managed:b", &instance))
                .expect("changed configuration")
        );
        let mut next_generation = connection_basis("managed:a", &instance);
        next_generation.integration_generation = 1;
        assert_ne!(
            first,
            IntegrationRevision::for_connection(next_generation).expect("next generation")
        );

        let recreated_instance = ConnectionIntegrationInstanceId::parse(
            "connection_instance_11223344-5566-4abb-8cdd-eeff10203040",
        )
        .unwrap();
        let recreated =
            IntegrationRevision::for_connection(connection_basis("managed:a", &recreated_instance))
                .expect("recreated physical instance");
        assert_ne!(first, recreated);

        let mut invalid_generation = connection_basis("managed:a", &instance);
        invalid_generation.integration_generation = -1;
        assert_eq!(
            IntegrationRevision::for_connection(invalid_generation),
            Err(IntegrationRevisionError::InvalidGeneration)
        );

        let policy_fingerprint = format!("sha256:{}", "a".repeat(64));
        let guard_policy_hash = format!("sha256:{}", "b".repeat(64));
        let project_basis = ProjectIntegrationRevisionBasis {
            connection_integration_revision: first.as_str(),
            project_id: "project.alpha",
            policy_fingerprint: &policy_fingerprint,
            guard_installation_id: Some("guard.alpha"),
            guard_policy_hash: Some(&guard_policy_hash),
        };
        let project =
            IntegrationRevision::for_project(project_basis.clone()).expect("valid project basis");
        assert_eq!(
            project,
            IntegrationRevision::for_project(project_basis).expect("same project basis")
        );
        let recreated_project = IntegrationRevision::for_project(ProjectIntegrationRevisionBasis {
            connection_integration_revision: recreated.as_str(),
            project_id: "project.alpha",
            policy_fingerprint: &format!("sha256:{}", "a".repeat(64)),
            guard_installation_id: Some("guard.alpha"),
            guard_policy_hash: Some(&format!("sha256:{}", "b".repeat(64))),
        })
        .expect("recreated project basis");
        let other_project = IntegrationRevision::for_project(ProjectIntegrationRevisionBasis {
            connection_integration_revision: first.as_str(),
            project_id: "project.beta",
            policy_fingerprint: &format!("sha256:{}", "a".repeat(64)),
            guard_installation_id: Some("guard.alpha"),
            guard_policy_hash: Some(&format!("sha256:{}", "b".repeat(64))),
        })
        .expect("other project basis");
        let native_session = "native.session.same";
        let session = crate::managed_mcp_client_info::project_agent_session_id(
            "connection.alpha",
            project.as_str(),
            native_session,
        )
        .expect("current project session");
        assert_ne!(
            session,
            crate::managed_mcp_client_info::project_agent_session_id(
                "connection.alpha",
                recreated_project.as_str(),
                native_session,
            )
            .expect("recreated project session")
        );
        assert_ne!(
            session,
            crate::managed_mcp_client_info::project_agent_session_id(
                "connection.alpha",
                other_project.as_str(),
                native_session,
            )
            .expect("other project session")
        );
    }

    #[test]
    fn host_observation_data_is_not_an_integration_revision_input() {
        let instance = integration_instance_id();
        let basis = connection_basis("managed:exact-entry", &instance);
        let revision = IntegrationRevision::for_connection(basis.clone()).expect("valid basis");
        let _diagnostic_host_version = "future-host-999.1";
        let _diagnostic_executable_path = "/diagnostic/host/path";
        assert_eq!(
            revision,
            IntegrationRevision::for_connection(basis).expect("observation-independent basis")
        );
    }
}
