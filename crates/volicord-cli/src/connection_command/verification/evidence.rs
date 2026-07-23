//! Typed evidence projected into MCP-related connection-check details.

use serde::Serialize;
use volicord_store::operational_sessions::{
    ManagedCapabilityProof, ManagedPeerObservation, McpSessionMilestones,
};
use volicord_types::{IntegrationRevision, ToolVerificationRole, UtcTimestamp};

use super::{HostExecutableStatus, Verification};
use crate::host_integration::verification::HostExecutableProbe;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct HostExecutableProbeDetails {
    status: HostExecutableStatus,
    probe: HostExecutableProbe,
    diagnostic: String,
}

impl HostExecutableProbeDetails {
    pub(super) fn from_verification(host: &Verification) -> Self {
        Self {
            status: host.host_executable,
            probe: host.host_executable_probe(),
            diagnostic: host.host_executable_details.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ManagedPeerObservationDetails {
    client_info: ManagedPeerClientInfoDetails,
    requested_protocol_revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_protocol_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    negotiated_protocol_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ManagedPeerClientInfoDetails {
    name: String,
    version: String,
}

impl From<&ManagedPeerObservation> for ManagedPeerObservationDetails {
    fn from(peer: &ManagedPeerObservation) -> Self {
        Self {
            client_info: ManagedPeerClientInfoDetails {
                name: peer.client_info.name().to_owned(),
                version: peer.client_info.version().to_owned(),
            },
            requested_protocol_revision: peer.requested_protocol_revision.clone(),
            selected_protocol_revision: peer.selected_protocol_revision.clone(),
            negotiated_protocol_revision: peer.negotiated_protocol_revision.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ManagedSessionAttemptDetails {
    evidence_role: &'static str,
    current_integration_revision: IntegrationRevision,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<volicord_types::McpRuntimeSessionSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_integration_revision: Option<IntegrationRevision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    process_started_at: Option<UtcTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initialize_completed_at: Option<UtcTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initialized_notification_at: Option<UtcTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    managed_peer: Option<ManagedPeerObservationDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required_tools: Option<RequiredToolsAttemptEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_tool: Option<VerificationToolEvidence>,
    host_executable_probe: HostExecutableProbe,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal_finding_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_observed_at: Option<UtcTimestamp>,
}

impl ManagedSessionAttemptDetails {
    pub(super) fn new(
        current_revision: &IntegrationRevision,
        attempt: Option<&McpSessionMilestones>,
        host: &Verification,
    ) -> Self {
        Self {
            evidence_role: "latest_attempt",
            current_integration_revision: current_revision.clone(),
            runtime_session_id: attempt.map(|value| value.runtime_session_id.as_str().to_owned()),
            source: attempt.map(|value| value.source),
            observed_integration_revision: attempt.map(|value| value.integration_revision.clone()),
            process_started_at: attempt.map(|value| value.process_started_at.clone()),
            initialize_completed_at: attempt
                .and_then(|value| value.initialize_completed_at.clone()),
            initialized_notification_at: attempt
                .and_then(|value| value.initialized_notification_at.clone()),
            managed_peer: attempt
                .and_then(|value| value.managed_peer.as_ref())
                .map(ManagedPeerObservationDetails::from),
            required_tools: attempt.and_then(|value| {
                Some(RequiredToolsAttemptEvidence {
                    tools_list_observed_at: value.tools_list_observed_at.clone()?,
                    returned_tool_identities: value.returned_tool_identities.clone()?,
                    required_tools_present: value.required_tools_present?,
                    required_tools_validated_at: value.required_tools_validated_at.clone(),
                })
            }),
            verification_tool: attempt.and_then(|value| {
                Some(VerificationToolEvidence {
                    expected_tool_identity: expected_verification_tool_name().to_owned(),
                    observed_tool_identity: value.verification_tool_name.clone()?,
                    observed_at: value.verification_tool_observed_at.clone()?,
                })
            }),
            host_executable_probe: host.host_executable_probe(),
            terminal_finding_id: attempt
                .and_then(|value| value.terminal_finding.as_ref())
                .map(|value| value.as_str().to_owned()),
            last_observed_at: attempt.map(|value| value.last_observed_at.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RequiredToolsAttemptEvidence {
    tools_list_observed_at: UtcTimestamp,
    returned_tool_identities: Vec<String>,
    required_tools_present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    required_tools_validated_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct RequiredToolsEvidence {
    tools_list_observed_at: UtcTimestamp,
    returned_tool_identities: Vec<String>,
    required_tools_validated_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct VerificationToolEvidence {
    expected_tool_identity: String,
    observed_tool_identity: String,
    observed_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ManagedCapabilityProofDetails {
    evidence_role: &'static str,
    current_integration_revision: IntegrationRevision,
    runtime_session_id: String,
    source: volicord_types::McpRuntimeSessionSource,
    observed_integration_revision: IntegrationRevision,
    process_started_at: UtcTimestamp,
    initialize_completed_at: UtcTimestamp,
    initialized_notification_at: UtcTimestamp,
    managed_peer: ManagedPeerObservationDetails,
    required_tools: RequiredToolsEvidence,
    verification_tool: VerificationToolEvidence,
    host_executable_probe: HostExecutableProbe,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal_finding_id: Option<String>,
    last_observed_at: UtcTimestamp,
}

impl ManagedCapabilityProofDetails {
    pub(super) fn new(
        current_revision: &IntegrationRevision,
        proof: &ManagedCapabilityProof,
        host: &Verification,
    ) -> Self {
        let milestones = proof.milestones();
        Self {
            evidence_role: "latest_complete_proof",
            current_integration_revision: current_revision.clone(),
            runtime_session_id: milestones.runtime_session_id.as_str().to_owned(),
            source: milestones.source,
            observed_integration_revision: milestones.integration_revision.clone(),
            process_started_at: milestones.process_started_at.clone(),
            initialize_completed_at: milestones
                .initialize_completed_at
                .clone()
                .expect("complete proof has initialize completion"),
            initialized_notification_at: milestones
                .initialized_notification_at
                .clone()
                .expect("complete proof has initialized notification"),
            managed_peer: ManagedPeerObservationDetails::from(
                milestones
                    .managed_peer
                    .as_ref()
                    .expect("complete proof has managed peer observation"),
            ),
            required_tools: RequiredToolsEvidence {
                tools_list_observed_at: milestones
                    .tools_list_observed_at
                    .clone()
                    .expect("complete proof has tools/list observation"),
                returned_tool_identities: milestones
                    .returned_tool_identities
                    .clone()
                    .expect("complete proof has returned tool identities"),
                required_tools_validated_at: milestones
                    .required_tools_validated_at
                    .clone()
                    .expect("complete proof has required-tool validation"),
            },
            verification_tool: VerificationToolEvidence {
                expected_tool_identity: expected_verification_tool_name().to_owned(),
                observed_tool_identity: milestones
                    .verification_tool_name
                    .clone()
                    .expect("complete proof has verification-tool identity"),
                observed_at: milestones
                    .verification_tool_observed_at
                    .clone()
                    .expect("complete proof has verification-tool observation"),
            },
            host_executable_probe: host.host_executable_probe(),
            terminal_finding_id: milestones
                .terminal_finding
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            last_observed_at: milestones.last_observed_at.clone(),
        }
    }
}

pub(super) fn expected_verification_tool_name() -> &'static str {
    ToolVerificationRole::ManagedHostRoundTrip
        .tool()
        .wire_name()
}
