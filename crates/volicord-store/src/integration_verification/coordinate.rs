use volicord_types::AgentToolId;

use super::{GuardIntegrationVerificationCaller, GuardIntegrationVerificationRunRecord};
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
    project_internal_id: String,
    guard_installation_id: String,
    integration_revision: String,
    policy_hash: String,
    hook_contract_digest: String,
    expected_probe_tool: String,
    expected_host_callable_name: String,
}

impl VerificationCurrentCoordinate {
    pub(super) fn new(
        caller: VerificationCallerCoordinate,
        project_internal_id: impl Into<String>,
        guard_installation_id: impl Into<String>,
        integration_revision: impl Into<String>,
        policy_hash: impl Into<String>,
        hook_contract_digest: impl Into<String>,
        expected_host_callable_name: impl Into<String>,
    ) -> Self {
        Self {
            caller,
            project_internal_id: project_internal_id.into(),
            guard_installation_id: guard_installation_id.into(),
            integration_revision: integration_revision.into(),
            policy_hash: policy_hash.into(),
            hook_contract_digest: hook_contract_digest.into(),
            expected_probe_tool: AgentToolId::GUARD_PROBE.wire_name().to_owned(),
            expected_host_callable_name: expected_host_callable_name.into(),
        }
    }

    pub(super) fn caller(&self) -> &VerificationCallerCoordinate {
        &self.caller
    }

    pub(super) fn project_internal_id(&self) -> &str {
        &self.project_internal_id
    }

    pub(super) fn guard_installation_id(&self) -> &str {
        &self.guard_installation_id
    }

    pub(super) fn integration_revision(&self) -> &str {
        &self.integration_revision
    }

    pub(super) fn policy_hash(&self) -> &str {
        &self.policy_hash
    }

    pub(super) fn hook_contract_digest(&self) -> &str {
        &self.hook_contract_digest
    }

    pub(super) fn expected_probe_tool(&self) -> &str {
        &self.expected_probe_tool
    }

    pub(super) fn expected_host_callable_name(&self) -> &str {
        &self.expected_host_callable_name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VerificationStoredCoordinate {
    caller: VerificationCallerCoordinate,
    project_internal_id: String,
    guard_installation_id: String,
    integration_revision: String,
    policy_hash: String,
    hook_contract_digest: String,
    expected_probe_tool: String,
    expected_host_callable_name: String,
}

impl From<&GuardIntegrationVerificationRunRecord> for VerificationStoredCoordinate {
    fn from(run: &GuardIntegrationVerificationRunRecord) -> Self {
        Self {
            caller: VerificationCallerCoordinate {
                connection_internal_id: run.connection_internal_id.clone(),
                runtime_session_id: run.runtime_session_id.clone(),
                host_session_id: run.host_session_id.clone(),
                host_turn_id: run.host_turn_id.clone(),
            },
            project_internal_id: run.project_internal_id.clone(),
            guard_installation_id: run.guard_installation_id.clone(),
            integration_revision: run.integration_revision.clone(),
            policy_hash: run.policy_hash.clone(),
            hook_contract_digest: run.hook_contract_digest.clone(),
            expected_probe_tool: run.expected_probe_tool.clone(),
            expected_host_callable_name: run.expected_host_callable_name.clone(),
        }
    }
}

impl VerificationStoredCoordinate {
    pub(super) fn require_caller(&self, caller: &VerificationCallerCoordinate) -> StoreResult<()> {
        if self.caller != *caller
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
        if self.caller != current.caller
            || self.project_internal_id != current.project_internal_id
            || self.guard_installation_id != current.guard_installation_id
            || self.integration_revision != current.integration_revision
            || self.policy_hash != current.policy_hash
            || self.hook_contract_digest != current.hook_contract_digest
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
