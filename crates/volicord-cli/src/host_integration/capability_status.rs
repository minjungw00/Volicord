use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use volicord_types::{HostFeatureSupportStatus, IntegrationProfile};

use super::HostKind;

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

/// Dynamic evidence and readiness inputs for each final-output subcapability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FinalOutputEvaluationInputs {
    pub authority_display: HostFeatureEvaluationInput,
    pub authenticated_exact_replay: HostFeatureEvaluationInput,
    pub block_finalization: HostFeatureEvaluationInput,
}

impl FinalOutputEvaluationInputs {
    pub const fn uniform(input: HostFeatureEvaluationInput) -> Self {
        Self {
            authority_display: input,
            authenticated_exact_replay: input,
            block_finalization: input,
        }
    }

    const fn for_subcapability(
        self,
        subcapability: FinalOutputSubcapability,
    ) -> HostFeatureEvaluationInput {
        match subcapability {
            FinalOutputSubcapability::AuthorityDisplay => self.authority_display,
            FinalOutputSubcapability::AuthenticatedExactReplay => self.authenticated_exact_replay,
            FinalOutputSubcapability::BlockFinalization => self.block_finalization,
        }
    }
}

/// Typed support result for one profile's final-output capability set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalOutputSupportEvaluation {
    pub support_status: HostFeatureSupportStatus,
    pub authority_display: HostFeatureSupportStatus,
    pub authenticated_exact_replay: HostFeatureSupportStatus,
    pub block_finalization: Option<HostFeatureSupportStatus>,
}

/// Dynamic evidence and readiness inputs for the complete managed-host feature matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HostFeatureMatrixInputs {
    pub native_user_action: HostFeatureEvaluationInput,
    pub local_web_user_channel: HostFeatureEvaluationInput,
    pub verified_tool_producer: HostFeatureEvaluationInput,
    pub registered_connection_observation: HostFeatureEvaluationInput,
    pub record_final_output: FinalOutputEvaluationInputs,
    pub detective_final_output: FinalOutputEvaluationInputs,
}

/// One canonical evaluation of all six managed-host features for a host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostFeatureSupportMatrix {
    pub native_user_action: HostFeatureSupportStatus,
    pub local_web_user_channel: HostFeatureSupportStatus,
    pub verified_tool_producer: HostFeatureSupportStatus,
    pub registered_connection_observation: HostFeatureSupportStatus,
    pub record_final_output: FinalOutputSupportEvaluation,
    pub detective_final_output: FinalOutputSupportEvaluation,
}

/// One canonical administrative projection of host support plus selected-profile detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostFeatureDiagnosticProjection {
    matrix: HostFeatureSupportMatrix,
    profile: IntegrationProfile,
    final_output: FinalOutputSupportEvaluation,
    configured: bool,
    configuration_verified: bool,
}

impl HostFeatureSupportMatrix {
    pub const fn status_for(self, feature: HostFeature) -> HostFeatureSupportStatus {
        match feature {
            HostFeature::NativeUserAction => self.native_user_action,
            HostFeature::LocalWebUserChannel => self.local_web_user_channel,
            HostFeature::VerifiedToolProducer => self.verified_tool_producer,
            HostFeature::RegisteredConnectionObservation => self.registered_connection_observation,
            HostFeature::RecordFinalOutput => self.record_final_output.support_status,
            HostFeature::DetectiveFinalOutput => self.detective_final_output.support_status,
        }
    }

    /// Returns all six canonical feature/status pairs in stable contract order.
    pub fn rows(self) -> [(HostFeature, HostFeatureSupportStatus); HostFeature::ALL.len()] {
        HostFeature::ALL.map(|feature| (feature, self.status_for(feature)))
    }
}

impl FinalOutputSupportEvaluation {
    pub const fn status_for(
        self,
        subcapability: FinalOutputSubcapability,
    ) -> Option<HostFeatureSupportStatus> {
        match subcapability {
            FinalOutputSubcapability::AuthorityDisplay => Some(self.authority_display),
            FinalOutputSubcapability::AuthenticatedExactReplay => {
                Some(self.authenticated_exact_replay)
            }
            FinalOutputSubcapability::BlockFinalization => self.block_finalization,
        }
    }
}

impl HostFeatureDiagnosticProjection {
    /// Evaluates the no-live-evidence baseline once for reuse by every CLI projection.
    pub fn baseline(
        host_kind: HostKind,
        profile: IntegrationProfile,
        configured: bool,
        configuration_verified: bool,
    ) -> Self {
        let matrix = default_host_feature_support_matrix(host_kind);
        Self::from_matrix(matrix, profile, configured, configuration_verified)
    }

    /// Selects exact profile detail from an already evaluated six-feature matrix.
    pub fn from_matrix(
        matrix: HostFeatureSupportMatrix,
        profile: IntegrationProfile,
        configured: bool,
        configuration_verified: bool,
    ) -> Self {
        let final_output = match profile {
            IntegrationProfile::Record => matrix.record_final_output,
            IntegrationProfile::Detective => matrix.detective_final_output,
        };
        Self {
            matrix,
            profile,
            final_output,
            configured,
            configuration_verified,
        }
    }

    /// Returns the exact six-key support map shared by connection and Doctor output.
    pub fn host_feature_support_json(self) -> Value {
        host_feature_support_json(self.matrix)
    }

    /// Returns selected-profile final-output detail without conflating configuration and support.
    pub fn final_output_authority_disclosure_json(self) -> Value {
        final_output_authority_disclosure_json(
            self.profile,
            self.final_output,
            self.configured,
            self.configuration_verified,
        )
    }

    /// Returns all six canonical feature/status pairs in stable contract order.
    pub fn host_feature_support_rows(
        self,
    ) -> [(HostFeature, HostFeatureSupportStatus); HostFeature::ALL.len()] {
        self.matrix.rows()
    }
}

/// Projects one evaluated matrix as the exact six-key administrative JSON map.
pub fn host_feature_support_json(matrix: HostFeatureSupportMatrix) -> Value {
    let mut features = Map::new();
    for feature in HostFeature::ALL {
        features.insert(
            feature.as_str().to_owned(),
            json!(matrix.status_for(feature)),
        );
    }
    Value::Object(features)
}

/// Evaluates and projects the no-live-evidence host baseline without selecting a profile.
pub fn default_host_feature_support_json(host_kind: HostKind) -> Value {
    host_feature_support_json(default_host_feature_support_matrix(host_kind))
}

fn final_output_authority_disclosure_json(
    profile: IntegrationProfile,
    evaluation: FinalOutputSupportEvaluation,
    configured: bool,
    configuration_verified: bool,
) -> Value {
    let required_subcapabilities = required_final_output_subcapabilities(profile);
    let mut subcapabilities = Map::new();
    for subcapability in required_subcapabilities {
        let status = evaluation
            .status_for(*subcapability)
            .expect("profile-required final-output subcapabilities are always evaluated");
        subcapabilities.insert(subcapability.as_str().to_owned(), json!(status));
    }
    json!({
        "support_status": evaluation.support_status,
        "configured": configured,
        "configuration_verified": configuration_verified,
        "required_subcapabilities": required_subcapabilities
            .iter()
            .map(|subcapability| subcapability.as_str())
            .collect::<Vec<_>>(),
        "subcapabilities": subcapabilities,
    })
}

/// Returns the exact final-output feature selected by an integration profile.
pub const fn final_output_feature(profile: IntegrationProfile) -> HostFeature {
    match profile {
        IntegrationProfile::Record => HostFeature::RecordFinalOutput,
        IntegrationProfile::Detective => HostFeature::DetectiveFinalOutput,
    }
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
    host_kind: HostKind,
    feature: HostFeature,
) -> HostFeatureImplementation {
    match feature {
        HostFeature::NativeUserAction
        | HostFeature::LocalWebUserChannel
        | HostFeature::VerifiedToolProducer
        | HostFeature::RegisteredConnectionObservation => match host_kind {
            HostKind::Codex | HostKind::ClaudeCode => HostFeatureImplementation::Implemented,
            HostKind::Generic => HostFeatureImplementation::UnsupportedByHost,
        },
        HostFeature::RecordFinalOutput => {
            final_output_profile_implementation(host_kind, IntegrationProfile::Record)
        }
        HostFeature::DetectiveFinalOutput => {
            final_output_profile_implementation(host_kind, IntegrationProfile::Detective)
        }
    }
}

/// Returns the current built-in adapter implementation fact for one final-output capability.
pub const fn host_final_output_subcapability_implementation(
    host_kind: HostKind,
    subcapability: FinalOutputSubcapability,
) -> HostFeatureImplementation {
    match host_kind {
        HostKind::Codex => match subcapability {
            FinalOutputSubcapability::AuthorityDisplay => HostFeatureImplementation::Implemented,
            FinalOutputSubcapability::AuthenticatedExactReplay
            | FinalOutputSubcapability::BlockFinalization => {
                HostFeatureImplementation::UnsupportedByHost
            }
        },
        HostKind::ClaudeCode => HostFeatureImplementation::Implemented,
        HostKind::Generic => HostFeatureImplementation::UnsupportedByHost,
    }
}

fn final_output_profile_implementation(
    host_kind: HostKind,
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

/// Evaluates one of the six managed-host features for a current host.
pub fn evaluate_host_feature_support(
    host_kind: HostKind,
    feature: HostFeature,
    input: HostFeatureEvaluationInput,
) -> HostFeatureSupportStatus {
    evaluate_support_status(host_feature_implementation(host_kind, feature), input)
}

/// Aggregates a nonempty required-capability set with the canonical precedence.
pub fn aggregate_required_support_statuses(
    statuses: &[HostFeatureSupportStatus],
) -> Option<HostFeatureSupportStatus> {
    if statuses.is_empty() {
        return None;
    }
    for candidate in [
        HostFeatureSupportStatus::UnsupportedByHost,
        HostFeatureSupportStatus::ImplementedUnverified,
        HostFeatureSupportStatus::TemporarilyUnavailable,
    ] {
        if statuses.contains(&candidate) {
            return Some(candidate);
        }
    }
    Some(HostFeatureSupportStatus::Verified)
}

/// Evaluates and aggregates only the final-output capabilities required by the profile.
pub fn evaluate_final_output_support(
    host_kind: HostKind,
    profile: IntegrationProfile,
    inputs: FinalOutputEvaluationInputs,
) -> FinalOutputSupportEvaluation {
    let evaluate = |subcapability| {
        evaluate_support_status(
            host_final_output_subcapability_implementation(host_kind, subcapability),
            inputs.for_subcapability(subcapability),
        )
    };
    let authority_display = evaluate(FinalOutputSubcapability::AuthorityDisplay);
    let authenticated_exact_replay = evaluate(FinalOutputSubcapability::AuthenticatedExactReplay);
    let block_finalization = (profile == IntegrationProfile::Detective)
        .then(|| evaluate(FinalOutputSubcapability::BlockFinalization));
    let support_status = match block_finalization {
        Some(block_finalization) => aggregate_required_support_statuses(&[
            authority_display,
            authenticated_exact_replay,
            block_finalization,
        ]),
        None => {
            aggregate_required_support_statuses(&[authority_display, authenticated_exact_replay])
        }
    }
    .expect("final-output profiles always have required subcapabilities");

    FinalOutputSupportEvaluation {
        support_status,
        authority_display,
        authenticated_exact_replay,
        block_finalization,
    }
}

/// Evaluates all six feature states once for reuse by diagnostic projections.
pub fn evaluate_host_feature_support_matrix(
    host_kind: HostKind,
    inputs: HostFeatureMatrixInputs,
) -> HostFeatureSupportMatrix {
    HostFeatureSupportMatrix {
        native_user_action: evaluate_host_feature_support(
            host_kind,
            HostFeature::NativeUserAction,
            inputs.native_user_action,
        ),
        local_web_user_channel: evaluate_host_feature_support(
            host_kind,
            HostFeature::LocalWebUserChannel,
            inputs.local_web_user_channel,
        ),
        verified_tool_producer: evaluate_host_feature_support(
            host_kind,
            HostFeature::VerifiedToolProducer,
            inputs.verified_tool_producer,
        ),
        registered_connection_observation: evaluate_host_feature_support(
            host_kind,
            HostFeature::RegisteredConnectionObservation,
            inputs.registered_connection_observation,
        ),
        record_final_output: evaluate_final_output_support(
            host_kind,
            IntegrationProfile::Record,
            inputs.record_final_output,
        ),
        detective_final_output: evaluate_final_output_support(
            host_kind,
            IntegrationProfile::Detective,
            inputs.detective_final_output,
        ),
    }
}

/// Returns the no-live-evidence baseline for all six features of a host.
pub fn default_host_feature_support_matrix(host_kind: HostKind) -> HostFeatureSupportMatrix {
    evaluate_host_feature_support_matrix(host_kind, HostFeatureMatrixInputs::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MISSING_READY: HostFeatureEvaluationInput = HostFeatureEvaluationInput::new(
        ExactLiveEvidenceState::Missing,
        CurrentRuntimeReadiness::Ready,
    );
    const STALE_OR_MISMATCHED_READY: HostFeatureEvaluationInput = HostFeatureEvaluationInput::new(
        ExactLiveEvidenceState::StaleOrMismatched,
        CurrentRuntimeReadiness::Ready,
    );
    const CURRENT_READY: HostFeatureEvaluationInput = HostFeatureEvaluationInput::new(
        ExactLiveEvidenceState::Current,
        CurrentRuntimeReadiness::Ready,
    );
    const CURRENT_DOWN: HostFeatureEvaluationInput = HostFeatureEvaluationInput::new(
        ExactLiveEvidenceState::Current,
        CurrentRuntimeReadiness::TemporarilyUnavailable,
    );

    #[test]
    fn support_status_precedence_is_table_driven() {
        let cases = [
            (
                "unsupported ignores otherwise current inputs",
                HostFeatureImplementation::UnsupportedByHost,
                CURRENT_READY,
                HostFeatureSupportStatus::UnsupportedByHost,
            ),
            (
                "missing evidence",
                HostFeatureImplementation::Implemented,
                MISSING_READY,
                HostFeatureSupportStatus::ImplementedUnverified,
            ),
            (
                "stale or mismatched evidence",
                HostFeatureImplementation::Implemented,
                STALE_OR_MISMATCHED_READY,
                HostFeatureSupportStatus::ImplementedUnverified,
            ),
            (
                "current evidence with runtime down",
                HostFeatureImplementation::Implemented,
                CURRENT_DOWN,
                HostFeatureSupportStatus::TemporarilyUnavailable,
            ),
            (
                "current evidence with runtime ready",
                HostFeatureImplementation::Implemented,
                CURRENT_READY,
                HostFeatureSupportStatus::Verified,
            ),
        ];

        for (name, implementation, input, expected) in cases {
            assert_eq!(
                evaluate_support_status(implementation, input),
                expected,
                "{name}"
            );
        }
    }

    #[test]
    fn canonical_feature_and_subcapability_names_are_exact() {
        assert_eq!(
            HostFeature::ALL.map(HostFeature::as_str),
            [
                "native_user_action",
                "local_web_user_channel",
                "verified_tool_producer",
                "registered_connection_observation",
                "record_final_output",
                "detective_final_output",
            ]
        );
        assert_eq!(
            FinalOutputSubcapability::ALL.map(FinalOutputSubcapability::as_str),
            [
                "authority_display",
                "authenticated_exact_replay",
                "block_finalization",
            ]
        );
    }

    #[test]
    fn current_host_implementation_matrix_is_explicit() {
        for feature in &HostFeature::ALL[..4] {
            assert_eq!(
                host_feature_implementation(HostKind::Codex, *feature),
                HostFeatureImplementation::Implemented
            );
        }
        assert_eq!(
            host_final_output_subcapability_implementation(
                HostKind::Codex,
                FinalOutputSubcapability::AuthorityDisplay,
            ),
            HostFeatureImplementation::Implemented
        );
        for subcapability in [
            FinalOutputSubcapability::AuthenticatedExactReplay,
            FinalOutputSubcapability::BlockFinalization,
        ] {
            assert_eq!(
                host_final_output_subcapability_implementation(HostKind::Codex, subcapability),
                HostFeatureImplementation::UnsupportedByHost
            );
        }
        for feature in HostFeature::ALL {
            assert_eq!(
                host_feature_implementation(HostKind::ClaudeCode, feature),
                HostFeatureImplementation::Implemented
            );
            assert_eq!(
                host_feature_implementation(HostKind::Generic, feature),
                HostFeatureImplementation::UnsupportedByHost
            );
        }
        for subcapability in FinalOutputSubcapability::ALL {
            assert_eq!(
                host_final_output_subcapability_implementation(HostKind::ClaudeCode, subcapability,),
                HostFeatureImplementation::Implemented
            );
            assert_eq!(
                host_final_output_subcapability_implementation(HostKind::Generic, subcapability),
                HostFeatureImplementation::UnsupportedByHost
            );
        }
    }

    #[test]
    fn final_output_profile_requirements_are_exact() {
        assert_eq!(
            final_output_feature(IntegrationProfile::Record),
            HostFeature::RecordFinalOutput
        );
        assert_eq!(
            required_final_output_subcapabilities(IntegrationProfile::Record),
            &[
                FinalOutputSubcapability::AuthorityDisplay,
                FinalOutputSubcapability::AuthenticatedExactReplay,
            ]
        );
        assert_eq!(
            final_output_feature(IntegrationProfile::Detective),
            HostFeature::DetectiveFinalOutput
        );
        assert_eq!(
            required_final_output_subcapabilities(IntegrationProfile::Detective),
            &[
                FinalOutputSubcapability::AuthorityDisplay,
                FinalOutputSubcapability::AuthenticatedExactReplay,
                FinalOutputSubcapability::BlockFinalization,
            ]
        );
    }

    #[test]
    fn final_output_aggregation_uses_profile_and_host_facts() {
        let cases = [
            (
                "Codex Record lacks authenticated replay",
                HostKind::Codex,
                IntegrationProfile::Record,
                FinalOutputEvaluationInputs::uniform(CURRENT_READY),
                HostFeatureSupportStatus::UnsupportedByHost,
                None,
            ),
            (
                "Codex Detective lacks replay and block",
                HostKind::Codex,
                IntegrationProfile::Detective,
                FinalOutputEvaluationInputs::uniform(CURRENT_READY),
                HostFeatureSupportStatus::UnsupportedByHost,
                Some(HostFeatureSupportStatus::UnsupportedByHost),
            ),
            (
                "Claude Record defaults to implemented unverified",
                HostKind::ClaudeCode,
                IntegrationProfile::Record,
                FinalOutputEvaluationInputs::default(),
                HostFeatureSupportStatus::ImplementedUnverified,
                None,
            ),
            (
                "Claude Detective is verified only when every input is current and ready",
                HostKind::ClaudeCode,
                IntegrationProfile::Detective,
                FinalOutputEvaluationInputs::uniform(CURRENT_READY),
                HostFeatureSupportStatus::Verified,
                Some(HostFeatureSupportStatus::Verified),
            ),
            (
                "Claude Detective runtime outage remains temporary after exact evidence",
                HostKind::ClaudeCode,
                IntegrationProfile::Detective,
                FinalOutputEvaluationInputs {
                    block_finalization: CURRENT_DOWN,
                    ..FinalOutputEvaluationInputs::uniform(CURRENT_READY)
                },
                HostFeatureSupportStatus::TemporarilyUnavailable,
                Some(HostFeatureSupportStatus::TemporarilyUnavailable),
            ),
            (
                "stale evidence outranks a runtime outage",
                HostKind::ClaudeCode,
                IntegrationProfile::Detective,
                FinalOutputEvaluationInputs {
                    authenticated_exact_replay: STALE_OR_MISMATCHED_READY,
                    block_finalization: CURRENT_DOWN,
                    ..FinalOutputEvaluationInputs::uniform(CURRENT_READY)
                },
                HostFeatureSupportStatus::ImplementedUnverified,
                Some(HostFeatureSupportStatus::TemporarilyUnavailable),
            ),
            (
                "Generic Record is unsupported",
                HostKind::Generic,
                IntegrationProfile::Record,
                FinalOutputEvaluationInputs::uniform(CURRENT_READY),
                HostFeatureSupportStatus::UnsupportedByHost,
                None,
            ),
        ];

        for (name, host_kind, profile, inputs, expected, expected_block) in cases {
            let result = evaluate_final_output_support(host_kind, profile, inputs);
            assert_eq!(result.support_status, expected, "{name}");
            assert_eq!(result.block_finalization, expected_block, "{name}");
        }

        let record_ignores_block = evaluate_final_output_support(
            HostKind::ClaudeCode,
            IntegrationProfile::Record,
            FinalOutputEvaluationInputs {
                block_finalization: CURRENT_DOWN,
                ..FinalOutputEvaluationInputs::uniform(CURRENT_READY)
            },
        );
        assert_eq!(
            record_ignores_block.support_status,
            HostFeatureSupportStatus::Verified
        );
        assert_eq!(record_ignores_block.block_finalization, None);
    }

    #[test]
    fn aggregate_requires_a_nonempty_capability_set() {
        assert_eq!(aggregate_required_support_statuses(&[]), None);
        assert_eq!(
            aggregate_required_support_statuses(&[
                HostFeatureSupportStatus::Verified,
                HostFeatureSupportStatus::TemporarilyUnavailable,
                HostFeatureSupportStatus::ImplementedUnverified,
                HostFeatureSupportStatus::UnsupportedByHost,
            ]),
            Some(HostFeatureSupportStatus::UnsupportedByHost)
        );
    }

    #[test]
    fn complete_matrix_helper_preserves_one_evaluation_for_all_six_features() {
        let codex = default_host_feature_support_matrix(HostKind::Codex);
        for feature in &HostFeature::ALL[..4] {
            assert_eq!(
                codex.status_for(*feature),
                HostFeatureSupportStatus::ImplementedUnverified
            );
        }
        assert_eq!(
            codex.status_for(HostFeature::RecordFinalOutput),
            HostFeatureSupportStatus::UnsupportedByHost
        );
        assert_eq!(
            codex.status_for(HostFeature::DetectiveFinalOutput),
            HostFeatureSupportStatus::UnsupportedByHost
        );

        let claude_current = evaluate_host_feature_support_matrix(
            HostKind::ClaudeCode,
            HostFeatureMatrixInputs {
                native_user_action: CURRENT_READY,
                local_web_user_channel: CURRENT_READY,
                verified_tool_producer: CURRENT_READY,
                registered_connection_observation: CURRENT_READY,
                record_final_output: FinalOutputEvaluationInputs::uniform(CURRENT_READY),
                detective_final_output: FinalOutputEvaluationInputs::uniform(CURRENT_READY),
            },
        );
        for feature in HostFeature::ALL {
            assert_eq!(
                claude_current.status_for(feature),
                HostFeatureSupportStatus::Verified
            );
        }

        let generic = default_host_feature_support_matrix(HostKind::Generic);
        for feature in HostFeature::ALL {
            assert_eq!(
                generic.status_for(feature),
                HostFeatureSupportStatus::UnsupportedByHost
            );
        }
    }

    #[test]
    fn administrative_projection_shape_is_exact_and_table_driven() {
        let cases = [
            (
                "Codex Record",
                HostKind::Codex,
                IntegrationProfile::Record,
                HostFeatureSupportStatus::UnsupportedByHost,
                &["authority_display", "authenticated_exact_replay"][..],
            ),
            (
                "Claude Detective",
                HostKind::ClaudeCode,
                IntegrationProfile::Detective,
                HostFeatureSupportStatus::ImplementedUnverified,
                &[
                    "authority_display",
                    "authenticated_exact_replay",
                    "block_finalization",
                ][..],
            ),
        ];

        for (name, host_kind, profile, expected_support, required) in cases {
            let projection =
                HostFeatureDiagnosticProjection::baseline(host_kind, profile, true, false);
            let feature_map = projection.host_feature_support_json();
            assert_eq!(feature_map.as_object().map(Map::len), Some(6), "{name}");
            for feature in HostFeature::ALL {
                assert!(feature_map.get(feature.as_str()).is_some(), "{name}");
            }

            let final_output = projection.final_output_authority_disclosure_json();
            assert_eq!(final_output.as_object().map(Map::len), Some(5), "{name}");
            assert_eq!(final_output["support_status"], json!(expected_support));
            assert_eq!(final_output["configured"], true);
            assert_eq!(final_output["configuration_verified"], false);
            assert_eq!(final_output["required_subcapabilities"], json!(required));
            assert_eq!(
                final_output["subcapabilities"].as_object().map(Map::len),
                Some(required.len()),
                "{name}"
            );
            for subcapability in required {
                assert!(
                    final_output["subcapabilities"].get(subcapability).is_some(),
                    "{name}"
                );
            }
            assert!(final_output.get("supported").is_none(), "{name}");
            assert!(final_output.get("verified").is_none(), "{name}");
        }
    }

    #[test]
    fn connection_and_doctor_consumers_share_the_same_six_key_projection() {
        for host_kind in [HostKind::Codex, HostKind::ClaudeCode, HostKind::Generic] {
            let connection_projection = HostFeatureDiagnosticProjection::baseline(
                host_kind,
                IntegrationProfile::Record,
                false,
                false,
            )
            .host_feature_support_json();
            let doctor_projection = default_host_feature_support_json(host_kind);
            assert_eq!(connection_projection, doctor_projection);
        }
    }
}
