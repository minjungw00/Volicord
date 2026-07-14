use serde::{Deserialize, Serialize};

use crate::{HostFeatureSupportStatus, IntegrationProfile};

/// Exact reviewed Codex host version used by managed-host compatibility policy.
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
}

impl HostFeatureEvaluationInput {
    pub const fn new(
        exact_evidence: ExactLiveEvidenceState,
        runtime_readiness: CurrentRuntimeReadiness,
    ) -> Self {
        Self {
            exact_evidence,
            runtime_readiness,
        }
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

/// Returns the reviewed exact-version fact, falling back to the host-kind table.
pub fn host_feature_implementation_for_version(
    host_kind: &str,
    host_version: Option<&str>,
    feature: HostFeature,
) -> HostFeatureImplementation {
    if host_kind == "codex"
        && host_version == Some(REVIEWED_CODEX_HOST_VERSION)
        && feature == HostFeature::LocalWebUserChannel
    {
        HostFeatureImplementation::UnsupportedByHost
    } else {
        host_feature_implementation(host_kind, feature)
    }
}

/// Returns the current built-in adapter implementation fact for one final-output capability.
pub fn host_final_output_subcapability_implementation(
    host_kind: &str,
    subcapability: FinalOutputSubcapability,
) -> HostFeatureImplementation {
    match host_kind {
        "codex" => match subcapability {
            FinalOutputSubcapability::AuthorityDisplay => HostFeatureImplementation::Implemented,
            FinalOutputSubcapability::AuthenticatedExactReplay
            | FinalOutputSubcapability::BlockFinalization => {
                HostFeatureImplementation::UnsupportedByHost
            }
        },
        "claude_code" => HostFeatureImplementation::Implemented,
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
    match implementation {
        HostFeatureImplementation::UnsupportedByHost => HostFeatureSupportStatus::UnsupportedByHost,
        HostFeatureImplementation::Implemented => match input.exact_evidence {
            ExactLiveEvidenceState::Missing | ExactLiveEvidenceState::StaleOrMismatched => {
                HostFeatureSupportStatus::ImplementedUnverified
            }
            ExactLiveEvidenceState::Current => match input.runtime_readiness {
                CurrentRuntimeReadiness::Ready => HostFeatureSupportStatus::Verified,
                CurrentRuntimeReadiness::TemporarilyUnavailable => {
                    HostFeatureSupportStatus::TemporarilyUnavailable
                }
            },
        },
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
    fn reviewed_codex_version_implementation_matrix_is_exact() {
        let expected = [
            HostFeatureImplementation::Implemented,
            HostFeatureImplementation::UnsupportedByHost,
            HostFeatureImplementation::Implemented,
            HostFeatureImplementation::Implemented,
            HostFeatureImplementation::UnsupportedByHost,
            HostFeatureImplementation::UnsupportedByHost,
        ];
        for (feature, expected) in HostFeature::ALL.into_iter().zip(expected) {
            assert_eq!(
                host_feature_implementation_for_version(
                    "codex",
                    Some(REVIEWED_CODEX_HOST_VERSION),
                    feature,
                ),
                expected,
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
            HostFeatureSupportStatus::UnsupportedByHost
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
}
