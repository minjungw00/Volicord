use volicord_host_contract::{
    HookObservationPolicy, HostContractProfileId, HostSessionId, HostTurnId,
};
use volicord_types::{
    is_canonical_sha256_digest, AgentConnectionId, AgentRuntimeSessionId, AgentToolId,
    GuardInstallationId, IntegrationRevision, PolicyHash, ProjectId,
};

use super::{
    GuardIntegrationVerificationCaller, GuardIntegrationVerificationRunRecord,
    GuardVerificationCoordinate,
};
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VerificationCallerCoordinate {
    connection_internal_id: String,
    runtime_session_id: String,
    host_session_id: String,
    host_turn_id: String,
}

impl VerificationCallerCoordinate {
    pub(super) fn from_caller(caller: &GuardIntegrationVerificationCaller) -> StoreResult<Self> {
        for (field, value) in [
            (
                "connection_internal_id",
                caller.connection_internal_id.as_str(),
            ),
            ("runtime_session_id", caller.runtime_session_id.as_str()),
            ("host_session_id", caller.host_session_id.as_str()),
            ("host_turn_id", caller.host_turn_id.as_str()),
        ] {
            if value.is_empty() || value.trim() != value || value.contains('\0') {
                return Err(StoreError::InvalidInput {
                    detail: format!("{field} must be a non-empty canonical identifier"),
                });
            }
        }
        Ok(Self {
            connection_internal_id: caller.connection_internal_id.clone(),
            runtime_session_id: caller.runtime_session_id.clone(),
            host_session_id: caller.host_session_id.clone(),
            host_turn_id: caller.host_turn_id.clone(),
        })
    }

    pub(super) fn connection_internal_id(&self) -> &str {
        &self.connection_internal_id
    }

    pub(super) fn runtime_session_id(&self) -> &str {
        &self.runtime_session_id
    }

    pub(super) fn host_session_id(&self) -> &str {
        &self.host_session_id
    }

    pub(super) fn host_turn_id(&self) -> &str {
        &self.host_turn_id
    }

    pub(super) fn conflict(&self, detail: impl Into<String>) -> StoreError {
        StoreError::Conflict {
            entity: "guard_integration_verification",
            id: self.runtime_session_id.clone(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VerificationCurrentCoordinate {
    caller: VerificationCallerCoordinate,
    semantic: GuardVerificationCoordinate,
    project_internal_id: String,
    expected_probe_tool: String,
    expected_host_callable_name: String,
    observation_policy: HookObservationPolicy,
}

impl VerificationCurrentCoordinate {
    pub(super) fn new(
        caller: VerificationCallerCoordinate,
        project_internal_id: impl Into<String>,
        semantic: GuardVerificationCoordinate,
        expected_host_callable_name: impl Into<String>,
        observation_policy: HookObservationPolicy,
    ) -> Self {
        Self {
            caller,
            semantic,
            project_internal_id: project_internal_id.into(),
            expected_probe_tool: AgentToolId::GUARD_PROBE.wire_name().to_owned(),
            expected_host_callable_name: expected_host_callable_name.into(),
            observation_policy,
        }
    }

    pub(super) fn caller(&self) -> &VerificationCallerCoordinate {
        &self.caller
    }

    pub(super) fn project_internal_id(&self) -> &str {
        &self.project_internal_id
    }

    pub(super) fn semantic(&self) -> &GuardVerificationCoordinate {
        &self.semantic
    }

    pub(super) fn guard_installation_id(&self) -> &str {
        self.semantic.guard_installation_id.as_str()
    }

    pub(super) fn integration_revision(&self) -> &str {
        self.semantic.integration_revision.as_str()
    }

    pub(super) fn policy_digest(&self) -> &str {
        self.semantic.policy_digest.as_str()
    }

    pub(super) fn hook_definition_digest(&self) -> &str {
        &self.semantic.hook_definition_digest
    }

    pub(super) fn host_contract_profile(&self) -> HostContractProfileId {
        self.semantic.host_contract_profile
    }

    pub(super) fn expected_probe_tool(&self) -> &str {
        &self.expected_probe_tool
    }

    pub(super) fn expected_host_callable_name(&self) -> &str {
        &self.expected_host_callable_name
    }

    pub(super) const fn observation_policy(&self) -> HookObservationPolicy {
        self.observation_policy
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VerificationStoredCoordinate {
    semantic: GuardVerificationCoordinate,
    expected_probe_tool: String,
    expected_host_callable_name: String,
}

impl VerificationStoredCoordinate {
    pub(super) fn from_run(run: &GuardIntegrationVerificationRunRecord) -> StoreResult<Self> {
        if !is_canonical_sha256_digest(&run.hook_definition_digest) {
            return Err(StoreError::corrupt_stored_value(
                "registry",
                "guard_integration_verification_runs.hook_definition_digest",
            ));
        }
        Ok(Self {
            semantic: GuardVerificationCoordinate {
                connection_id: AgentConnectionId::new(&run.connection_internal_id),
                project_id: ProjectId::new(&run.project_id),
                runtime_session_id: AgentRuntimeSessionId::new(&run.runtime_session_id),
                host_session_id: HostSessionId::parse(&run.host_session_id).map_err(|_| {
                    StoreError::corrupt_stored_value(
                        "registry",
                        "guard_integration_verification_runs.host_session_id",
                    )
                })?,
                host_turn_id: HostTurnId::parse(&run.host_turn_id).map_err(|_| {
                    StoreError::corrupt_stored_value(
                        "registry",
                        "guard_integration_verification_runs.host_turn_id",
                    )
                })?,
                integration_revision: IntegrationRevision::parse(&run.integration_revision)
                    .map_err(|_| {
                        StoreError::corrupt_stored_value(
                            "registry",
                            "guard_integration_verification_runs.integration_revision",
                        )
                    })?,
                guard_installation_id: GuardInstallationId::new(&run.guard_installation_id),
                host_contract_profile: HostContractProfileId::parse(&run.host_contract_profile)
                    .map_err(|_| {
                        StoreError::corrupt_stored_value(
                            "registry",
                            "guard_integration_verification_runs.host_contract_profile",
                        )
                    })?,
                hook_definition_digest: run.hook_definition_digest.clone(),
                policy_digest: PolicyHash::parse(&run.policy_digest).map_err(|_| {
                    StoreError::corrupt_stored_value(
                        "registry",
                        "guard_integration_verification_runs.policy_digest",
                    )
                })?,
            },
            expected_probe_tool: run.expected_probe_tool.clone(),
            expected_host_callable_name: run.expected_host_callable_name.clone(),
        })
    }

    pub(super) fn require_caller(&self, caller: &VerificationCallerCoordinate) -> StoreResult<()> {
        if self.semantic.connection_id.as_str() != caller.connection_internal_id()
            || self.semantic.runtime_session_id.as_str() != caller.runtime_session_id()
            || self.semantic.host_session_id.as_str() != caller.host_session_id()
            || self.semantic.host_turn_id.as_str() != caller.host_turn_id()
            || self.expected_probe_tool != AgentToolId::GUARD_PROBE.wire_name()
        {
            return Err(
                caller.conflict("verification belongs to another managed session or native turn")
            );
        }
        Ok(())
    }

    pub(super) fn require_current(
        &self,
        current: &VerificationCurrentCoordinate,
    ) -> StoreResult<()> {
        if self.semantic != current.semantic
            || self.expected_probe_tool != current.expected_probe_tool
            || self.expected_host_callable_name != current.expected_host_callable_name
        {
            return Err(current.caller.conflict(
                "an active verification coordinate is owned by different current facts",
            ));
        }
        Ok(())
    }
}
