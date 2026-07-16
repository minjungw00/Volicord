use serde::{Deserialize, Serialize};

use crate::{HostFeatureSupportStatus, IntegrationProfile, UtcTimestamp};

/// Exact reviewed Codex display and release-evidence coordinate.
pub const REVIEWED_CODEX_HOST_VERSION: &str = "0.144.4";

/// Exact MCP `clientInfo.name` emitted by the reviewed Codex host.
pub const REVIEWED_CODEX_MCP_CLIENT_NAME: &str = "codex-mcp-client";

const CODEX_VERSION_PROBE_PREFIX: &str = "codex-cli ";

const RECORD_FINAL_OUTPUT_SUBCAPABILITIES: [FinalOutputSubcapability; 2] = [
    FinalOutputSubcapability::AuthorityDisplay,
    FinalOutputSubcapability::AuthenticatedExactReplay,
];
const DETECTIVE_FINAL_OUTPUT_SUBCAPABILITIES: [FinalOutputSubcapability; 3] = [
    FinalOutputSubcapability::AuthorityDisplay,
    FinalOutputSubcapability::AuthenticatedExactReplay,
    FinalOutputSubcapability::BlockFinalization,
];

const NATIVE_USER_ACTION_PROBES: [HostRuntimeProbeId; 1] =
    [HostRuntimeProbeId::ModelSeparatedUserActionUi];
const LOCAL_WEB_USER_CHANNEL_PROBES: [HostRuntimeProbeId; 2] = [
    HostRuntimeProbeId::ModelSeparatedUserActionUi,
    HostRuntimeProbeId::McpCapabilityAdvertisedAndExercised,
];
const VERIFIED_TOOL_PRODUCER_PROBES: [HostRuntimeProbeId; 3] = [
    HostRuntimeProbeId::LifecycleHookDelivery,
    HostRuntimeProbeId::PreToolStructuredTargetPaths,
    HostRuntimeProbeId::PostToolStructuredChangedPaths,
];
const REGISTERED_CONNECTION_OBSERVATION_PROBES: [HostRuntimeProbeId; 2] = [
    HostRuntimeProbeId::LifecycleHookDelivery,
    HostRuntimeProbeId::PostToolStructuredChangedPaths,
];
const RECORD_FINAL_OUTPUT_PROBES: [HostRuntimeProbeId; 2] = [
    HostRuntimeProbeId::FixedUiAuthorityDisclosure,
    HostRuntimeProbeId::StopDeliveryAndReplay,
];
const DETECTIVE_FINAL_OUTPUT_PROBES: [HostRuntimeProbeId; 2] = [
    HostRuntimeProbeId::FixedUiAuthorityDisclosure,
    HostRuntimeProbeId::StopDeliveryAndReplay,
];
const AUTHORITY_DISPLAY_PROBES: [HostRuntimeProbeId; 1] =
    [HostRuntimeProbeId::FixedUiAuthorityDisclosure];
const AUTHENTICATED_EXACT_REPLAY_PROBES: [HostRuntimeProbeId; 1] =
    [HostRuntimeProbeId::StopDeliveryAndReplay];
const BLOCK_FINALIZATION_PROBES: [HostRuntimeProbeId; 1] =
    [HostRuntimeProbeId::StopDeliveryAndReplay];

/// Schema identifier for the current bounded runtime-probe snapshot.
pub const HOST_RUNTIME_PROBE_SNAPSHOT_SCHEMA: &str = "volicord-host-runtime-probes-v1";

/// Closed identifiers for actual managed-host runtime capability probes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostRuntimeProbeId {
    LifecycleHookDelivery,
    PreToolStructuredTargetPaths,
    PostToolStructuredChangedPaths,
    ModelSeparatedUserActionUi,
    StopDeliveryAndReplay,
    FixedUiAuthorityDisclosure,
    McpCapabilityAdvertisedAndExercised,
}

impl HostRuntimeProbeId {
    pub const ALL: [Self; 7] = [
        Self::LifecycleHookDelivery,
        Self::PreToolStructuredTargetPaths,
        Self::PostToolStructuredChangedPaths,
        Self::ModelSeparatedUserActionUi,
        Self::StopDeliveryAndReplay,
        Self::FixedUiAuthorityDisclosure,
        Self::McpCapabilityAdvertisedAndExercised,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LifecycleHookDelivery => "lifecycle_hook_delivery",
            Self::PreToolStructuredTargetPaths => "pre_tool_structured_target_paths",
            Self::PostToolStructuredChangedPaths => "post_tool_structured_changed_paths",
            Self::ModelSeparatedUserActionUi => "model_separated_user_action_ui",
            Self::StopDeliveryAndReplay => "stop_delivery_and_replay",
            Self::FixedUiAuthorityDisclosure => "fixed_ui_authority_disclosure",
            Self::McpCapabilityAdvertisedAndExercised => "mcp_capability_advertised_and_exercised",
        }
    }
}

/// Closed result values for one bounded runtime probe observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostRuntimeProbeOutcome {
    Passed,
    Failed,
    Unavailable,
    Unsupported,
}

/// Closed, content-free reason values retained with a runtime probe result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostRuntimeProbeFailureClass {
    None,
    ExplicitCapabilityAbsent,
    ConfigurationUnavailable,
    BindingMismatch,
    ApprovalRequired,
    ListenerUnavailable,
    EventDeliveryFailed,
    StructuredPathsMissing,
    ModelSeparationUnconfirmed,
    ReplayFailed,
    SecondStopRequested,
    FixedUiUnconfirmed,
    CapabilityNotAdvertised,
    CapabilityNotExercised,
    ProbeNotRun,
}

/// One content-free, connection-bound observation of an actual host surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostRuntimeProbeObservation {
    pub probe_id: HostRuntimeProbeId,
    pub outcome: HostRuntimeProbeOutcome,
    pub failure_class: HostRuntimeProbeFailureClass,
    pub connection_internal_id: String,
    pub host_kind: String,
    pub host_version: Option<String>,
    pub client_name: Option<String>,
    pub client_version: Option<String>,
    pub adapter_profile: IntegrationProfile,
    pub adapter_version: String,
    pub managed_fingerprint: String,
    pub observed_at: UtcTimestamp,
    pub expires_at: UtcTimestamp,
}

/// Current bounded observations embedded in one Agent Connection verification report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostRuntimeProbeSnapshot {
    pub schema: String,
    pub observations: Vec<HostRuntimeProbeObservation>,
}

impl Default for HostRuntimeProbeSnapshot {
    fn default() -> Self {
        Self {
            schema: HOST_RUNTIME_PROBE_SNAPSHOT_SCHEMA.to_owned(),
            observations: Vec::new(),
        }
    }
}

/// Exact managed-host features with independently reported support state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostFeature {
    NativeUserAction,
    LocalWebUserChannel,
    VerifiedToolProducer,
    RegisteredConnectionObservation,
    RecordFinalOutput,
    DetectiveFinalOutput,
}

impl HostFeature {
    pub const ALL: [Self; 6] = [
        Self::NativeUserAction,
        Self::LocalWebUserChannel,
        Self::VerifiedToolProducer,
        Self::RegisteredConnectionObservation,
        Self::RecordFinalOutput,
        Self::DetectiveFinalOutput,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NativeUserAction => "native_user_action",
            Self::LocalWebUserChannel => "local_web_user_channel",
            Self::VerifiedToolProducer => "verified_tool_producer",
            Self::RegisteredConnectionObservation => "registered_connection_observation",
            Self::RecordFinalOutput => "record_final_output",
            Self::DetectiveFinalOutput => "detective_final_output",
        }
    }
}

/// Profile-applicable capabilities required by managed final-output support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalOutputSubcapability {
    AuthorityDisplay,
    AuthenticatedExactReplay,
    BlockFinalization,
}

impl FinalOutputSubcapability {
    pub const ALL: [Self; 3] = [
        Self::AuthorityDisplay,
        Self::AuthenticatedExactReplay,
        Self::BlockFinalization,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthorityDisplay => "authority_display",
            Self::AuthenticatedExactReplay => "authenticated_exact_replay",
            Self::BlockFinalization => "block_finalization",
        }
    }
}

/// Static implementation fact supplied by the built-in host adapter contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostFeatureImplementation {
    Implemented,
    UnsupportedByHost,
}

/// Freshness and exact-artifact match state of live evidence for one feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactLiveEvidenceState {
    Missing,
    StaleOrMismatched,
    Current,
}

/// Present-time readiness of the runtime prerequisites for one feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentRuntimeReadiness {
    Ready,
    TemporarilyUnavailable,
}

/// Dynamic inputs evaluated after the static host implementation fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostFeatureEvaluationInput {
    pub exact_evidence: ExactLiveEvidenceState,
    pub runtime_readiness: CurrentRuntimeReadiness,
    pub explicit_capability_absence: bool,
}

impl HostFeatureEvaluationInput {
    pub const fn new(
        exact_evidence: ExactLiveEvidenceState,
        runtime_readiness: CurrentRuntimeReadiness,
    ) -> Self {
        Self {
            exact_evidence,
            runtime_readiness,
            explicit_capability_absence: false,
        }
    }

    pub const fn with_explicit_capability_absence(mut self) -> Self {
        self.explicit_capability_absence = true;
        self
    }
}

impl Default for HostFeatureEvaluationInput {
    fn default() -> Self {
        Self::new(
            ExactLiveEvidenceState::Missing,
            CurrentRuntimeReadiness::Ready,
        )
    }
}

/// Validates one canonical bare Codex host-version coordinate.
pub fn canonical_codex_host_version(version: &str) -> Option<&str> {
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
        || !version
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !version
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
    {
        return None;
    }
    Some(version)
}

/// Extracts the canonical bare Codex version from one exact version-probe envelope.
pub fn canonical_codex_host_version_from_probe(envelope: &str) -> Option<&str> {
    canonical_codex_host_version(envelope.strip_prefix(CODEX_VERSION_PROBE_PREFIX)?)
}

/// Returns only the subcapabilities applicable to the selected profile.
pub const fn required_final_output_subcapabilities(
    profile: IntegrationProfile,
) -> &'static [FinalOutputSubcapability] {
    match profile {
        IntegrationProfile::Record => &RECORD_FINAL_OUTPUT_SUBCAPABILITIES,
        IntegrationProfile::Detective => &DETECTIVE_FINAL_OUTPUT_SUBCAPABILITIES,
    }
}

/// Returns the exact runtime probes required by one managed-host feature.
pub const fn required_runtime_probes_for_feature(
    feature: HostFeature,
) -> &'static [HostRuntimeProbeId] {
    match feature {
        HostFeature::NativeUserAction => &NATIVE_USER_ACTION_PROBES,
        HostFeature::LocalWebUserChannel => &LOCAL_WEB_USER_CHANNEL_PROBES,
        HostFeature::VerifiedToolProducer => &VERIFIED_TOOL_PRODUCER_PROBES,
        HostFeature::RegisteredConnectionObservation => &REGISTERED_CONNECTION_OBSERVATION_PROBES,
        HostFeature::RecordFinalOutput => &RECORD_FINAL_OUTPUT_PROBES,
        HostFeature::DetectiveFinalOutput => &DETECTIVE_FINAL_OUTPUT_PROBES,
    }
}

/// Returns the exact runtime probes required by one final-output subcapability.
pub const fn required_runtime_probes_for_final_output_subcapability(
    subcapability: FinalOutputSubcapability,
) -> &'static [HostRuntimeProbeId] {
    match subcapability {
        FinalOutputSubcapability::AuthorityDisplay => &AUTHORITY_DISPLAY_PROBES,
        FinalOutputSubcapability::AuthenticatedExactReplay => &AUTHENTICATED_EXACT_REPLAY_PROBES,
        FinalOutputSubcapability::BlockFinalization => &BLOCK_FINALIZATION_PROBES,
    }
}

/// Derives dynamic support inputs from fresh observations matching the current binding.
pub fn runtime_probe_evaluation_input(
    snapshot: &HostRuntimeProbeSnapshot,
    required_probes: &[HostRuntimeProbeId],
    host_kind: &str,
    managed_fingerprint: &str,
    adapter_profile: IntegrationProfile,
    now: &UtcTimestamp,
) -> HostFeatureEvaluationInput {
    let mut exact_evidence = ExactLiveEvidenceState::Current;
    let mut runtime_readiness = CurrentRuntimeReadiness::Ready;
    let mut explicit_capability_absence = false;

    for probe_id in required_probes {
        let observation = snapshot.observations.iter().find(|observation| {
            observation.probe_id == *probe_id
                && observation.host_kind == host_kind
                && observation.managed_fingerprint == managed_fingerprint
                && observation.adapter_profile == adapter_profile
                && observation.observed_at <= *now
                && *now < observation.expires_at
        });
        let Some(observation) = observation else {
            exact_evidence = ExactLiveEvidenceState::Missing;
            continue;
        };
        if observation.failure_class == HostRuntimeProbeFailureClass::ProbeNotRun {
            exact_evidence = ExactLiveEvidenceState::Missing;
            continue;
        }
        match observation.outcome {
            HostRuntimeProbeOutcome::Passed => {}
            HostRuntimeProbeOutcome::Failed | HostRuntimeProbeOutcome::Unavailable => {
                runtime_readiness = CurrentRuntimeReadiness::TemporarilyUnavailable;
            }
            HostRuntimeProbeOutcome::Unsupported => {
                explicit_capability_absence = true;
            }
        }
    }

    HostFeatureEvaluationInput {
        exact_evidence,
        runtime_readiness,
        explicit_capability_absence,
    }
}

/// Returns the current built-in adapter implementation fact for one feature.
pub fn host_feature_implementation(
    host_kind: &str,
    feature: HostFeature,
) -> HostFeatureImplementation {
    match feature {
        HostFeature::NativeUserAction
        | HostFeature::LocalWebUserChannel
        | HostFeature::VerifiedToolProducer
        | HostFeature::RegisteredConnectionObservation => match host_kind {
            "codex" | "claude_code" => HostFeatureImplementation::Implemented,
            _ => HostFeatureImplementation::UnsupportedByHost,
        },
        HostFeature::RecordFinalOutput => {
            final_output_profile_implementation(host_kind, IntegrationProfile::Record)
        }
        HostFeature::DetectiveFinalOutput => {
            final_output_profile_implementation(host_kind, IntegrationProfile::Detective)
        }
    }
}

/// Returns the host-kind implementation fact while retaining a display-only version coordinate.
pub fn host_feature_implementation_for_version(
    host_kind: &str,
    _host_version: Option<&str>,
    feature: HostFeature,
) -> HostFeatureImplementation {
    host_feature_implementation(host_kind, feature)
}

/// Returns the current built-in adapter implementation fact for one final-output capability.
pub fn host_final_output_subcapability_implementation(
    host_kind: &str,
    _subcapability: FinalOutputSubcapability,
) -> HostFeatureImplementation {
    match host_kind {
        "codex" | "claude_code" => HostFeatureImplementation::Implemented,
        _ => HostFeatureImplementation::UnsupportedByHost,
    }
}

fn final_output_profile_implementation(
    host_kind: &str,
    profile: IntegrationProfile,
) -> HostFeatureImplementation {
    if required_final_output_subcapabilities(profile)
        .iter()
        .all(|subcapability| {
            host_final_output_subcapability_implementation(host_kind, *subcapability)
                == HostFeatureImplementation::Implemented
        })
    {
        HostFeatureImplementation::Implemented
    } else {
        HostFeatureImplementation::UnsupportedByHost
    }
}

/// Evaluates one feature using the canonical support-state precedence.
pub const fn evaluate_support_status(
    implementation: HostFeatureImplementation,
    input: HostFeatureEvaluationInput,
) -> HostFeatureSupportStatus {
    if matches!(implementation, HostFeatureImplementation::UnsupportedByHost)
        || input.explicit_capability_absence
    {
        HostFeatureSupportStatus::UnsupportedByHost
    } else if matches!(
        input.runtime_readiness,
        CurrentRuntimeReadiness::TemporarilyUnavailable
    ) {
        HostFeatureSupportStatus::TemporarilyUnavailable
    } else {
        match input.exact_evidence {
            ExactLiveEvidenceState::Missing | ExactLiveEvidenceState::StaleOrMismatched => {
                HostFeatureSupportStatus::ImplementedUnverified
            }
            ExactLiveEvidenceState::Current => HostFeatureSupportStatus::Verified,
        }
    }
}

/// Evaluates one feature using an exact reviewed host version when one is available.
pub fn evaluate_host_feature_support_for_version(
    host_kind: &str,
    host_version: Option<&str>,
    feature: HostFeature,
    input: HostFeatureEvaluationInput,
) -> HostFeatureSupportStatus {
    evaluate_support_status(
        host_feature_implementation_for_version(host_kind, host_version, feature),
        input,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT_READY: HostFeatureEvaluationInput = HostFeatureEvaluationInput::new(
        ExactLiveEvidenceState::Current,
        CurrentRuntimeReadiness::Ready,
    );

    #[test]
    fn codex_version_probe_canonicalization_is_strict() {
        assert_eq!(
            canonical_codex_host_version(REVIEWED_CODEX_HOST_VERSION),
            Some(REVIEWED_CODEX_HOST_VERSION)
        );
        assert_eq!(
            canonical_codex_host_version_from_probe("codex-cli 0.144.4"),
            Some(REVIEWED_CODEX_HOST_VERSION)
        );
        assert_eq!(
            canonical_codex_host_version_from_probe("codex-cli 1.2.3-alpha+1"),
            Some("1.2.3-alpha+1")
        );

        for invalid in [
            "",
            "codex-cli ",
            "codex 0.144.4",
            "codex-cli 0.144.4\n",
            " codex-cli 0.144.4",
            "codex-cli 0.144.4 ",
            "codex-cli .0.144.4",
            "codex-cli 0.144.4-",
            "codex-cli 0/144/4",
            "codex-cli 버전",
        ] {
            assert_eq!(
                canonical_codex_host_version_from_probe(invalid),
                None,
                "{invalid:?}"
            );
        }
        let oversized = format!("codex-cli {}", "1".repeat(65));
        assert_eq!(canonical_codex_host_version_from_probe(&oversized), None);
    }

    #[test]
    fn reviewed_codex_version_is_a_coordinate_not_an_implementation_gate() {
        for feature in HostFeature::ALL {
            assert_eq!(
                host_feature_implementation_for_version(
                    "codex",
                    Some(REVIEWED_CODEX_HOST_VERSION),
                    feature,
                ),
                HostFeatureImplementation::Implemented,
                "{}",
                feature.as_str()
            );
        }
        assert_eq!(
            evaluate_host_feature_support_for_version(
                "codex",
                Some(REVIEWED_CODEX_HOST_VERSION),
                HostFeature::LocalWebUserChannel,
                CURRENT_READY,
            ),
            HostFeatureSupportStatus::Verified
        );
    }

    #[test]
    fn absent_unreviewed_and_other_host_tables_are_explicit() {
        for host_version in [None, Some("0.144.5"), Some("codex-cli 0.144.4")] {
            for feature in HostFeature::ALL {
                assert_eq!(
                    host_feature_implementation_for_version("codex", host_version, feature),
                    host_feature_implementation("codex", feature),
                    "host_version={host_version:?}, feature={}",
                    feature.as_str()
                );
            }
        }
        for feature in HostFeature::ALL {
            assert_eq!(
                host_feature_implementation("claude_code", feature),
                HostFeatureImplementation::Implemented
            );
            assert_eq!(
                host_feature_implementation("generic", feature),
                HostFeatureImplementation::UnsupportedByHost
            );
            assert_eq!(
                host_feature_implementation("unknown", feature),
                HostFeatureImplementation::UnsupportedByHost
            );
        }
    }

    fn observation(
        probe_id: HostRuntimeProbeId,
        outcome: HostRuntimeProbeOutcome,
        failure_class: HostRuntimeProbeFailureClass,
        observed_at: &str,
        expires_at: &str,
    ) -> HostRuntimeProbeObservation {
        HostRuntimeProbeObservation {
            probe_id,
            outcome,
            failure_class,
            connection_internal_id: "connection_001".to_owned(),
            host_kind: "codex".to_owned(),
            host_version: Some(REVIEWED_CODEX_HOST_VERSION.to_owned()),
            client_name: Some(REVIEWED_CODEX_MCP_CLIENT_NAME.to_owned()),
            client_version: Some(REVIEWED_CODEX_HOST_VERSION.to_owned()),
            adapter_profile: IntegrationProfile::Detective,
            adapter_version: "0.1.0".to_owned(),
            managed_fingerprint: "fingerprint_001".to_owned(),
            observed_at: UtcTimestamp::parse(observed_at).expect("valid observed timestamp"),
            expires_at: UtcTimestamp::parse(expires_at).expect("valid expiry timestamp"),
        }
    }

    #[test]
    fn runtime_probe_ids_and_feature_mapping_are_closed() {
        assert_eq!(
            HostRuntimeProbeId::ALL.map(HostRuntimeProbeId::as_str),
            [
                "lifecycle_hook_delivery",
                "pre_tool_structured_target_paths",
                "post_tool_structured_changed_paths",
                "model_separated_user_action_ui",
                "stop_delivery_and_replay",
                "fixed_ui_authority_disclosure",
                "mcp_capability_advertised_and_exercised",
            ]
        );
        assert_eq!(
            required_runtime_probes_for_feature(HostFeature::VerifiedToolProducer),
            &[
                HostRuntimeProbeId::LifecycleHookDelivery,
                HostRuntimeProbeId::PreToolStructuredTargetPaths,
                HostRuntimeProbeId::PostToolStructuredChangedPaths,
            ]
        );
        assert_eq!(
            required_runtime_probes_for_final_output_subcapability(
                FinalOutputSubcapability::AuthorityDisplay,
            ),
            &[HostRuntimeProbeId::FixedUiAuthorityDisclosure]
        );
        assert_eq!(
            required_runtime_probes_for_final_output_subcapability(
                FinalOutputSubcapability::AuthenticatedExactReplay,
            ),
            &[HostRuntimeProbeId::StopDeliveryAndReplay]
        );
    }

    #[test]
    fn runtime_probe_evaluation_uses_canonical_precedence_and_binding() {
        let now = UtcTimestamp::parse("2026-07-16T00:30:00Z").expect("valid now");
        let required = required_runtime_probes_for_feature(HostFeature::VerifiedToolProducer);
        let passed = |probe_id| {
            observation(
                probe_id,
                HostRuntimeProbeOutcome::Passed,
                HostRuntimeProbeFailureClass::None,
                "2026-07-16T00:00:00Z",
                "2026-07-16T01:00:00Z",
            )
        };
        let mut snapshot = HostRuntimeProbeSnapshot {
            schema: HOST_RUNTIME_PROBE_SNAPSHOT_SCHEMA.to_owned(),
            observations: required.iter().copied().map(passed).collect(),
        };
        let evaluate = |snapshot: &HostRuntimeProbeSnapshot| {
            evaluate_support_status(
                HostFeatureImplementation::Implemented,
                runtime_probe_evaluation_input(
                    snapshot,
                    required,
                    "codex",
                    "fingerprint_001",
                    IntegrationProfile::Detective,
                    &now,
                ),
            )
        };
        assert_eq!(evaluate(&snapshot), HostFeatureSupportStatus::Verified);

        snapshot.observations[1].outcome = HostRuntimeProbeOutcome::Unavailable;
        snapshot.observations[1].failure_class = HostRuntimeProbeFailureClass::ListenerUnavailable;
        snapshot.observations.pop();
        assert_eq!(
            evaluate(&snapshot),
            HostFeatureSupportStatus::TemporarilyUnavailable,
            "a current outage outranks another missing probe"
        );

        snapshot.observations[1].outcome = HostRuntimeProbeOutcome::Unsupported;
        snapshot.observations[1].failure_class =
            HostRuntimeProbeFailureClass::ExplicitCapabilityAbsent;
        assert_eq!(
            evaluate(&snapshot),
            HostFeatureSupportStatus::UnsupportedByHost,
            "current explicit absence outranks outage and missing evidence"
        );

        snapshot.observations[1].managed_fingerprint = "stale_fingerprint".to_owned();
        assert_eq!(
            evaluate(&snapshot),
            HostFeatureSupportStatus::ImplementedUnverified,
            "mismatched observations are not current absence"
        );

        snapshot.observations = required.iter().copied().map(passed).collect();
        snapshot.observations[0].outcome = HostRuntimeProbeOutcome::Unavailable;
        snapshot.observations[0].failure_class = HostRuntimeProbeFailureClass::ProbeNotRun;
        assert_eq!(
            evaluate(&snapshot),
            HostFeatureSupportStatus::ImplementedUnverified,
            "an explicitly unrun probe is missing evidence, not a runtime outage"
        );
    }
}
