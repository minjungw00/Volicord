use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use volicord_cli::host_integration::capability_status::HostFeature;
use volicord_types::HostFeatureSupportStatus;

pub const CANDIDATE_SCHEMA: &str = "volicord-release-candidate-v1";
pub const CELL_SCHEMA: &str = "volicord-host-release-cell-v1";
pub const MANIFEST_SCHEMA: &str = "volicord-host-release-manifest-v1";
pub const AUDIT_SCHEMA: &str = "volicord-host-release-audit-v1";
pub const SOURCE_ARCHIVE_ALGORITHM: &str = "git_archive_tar_sha256_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequiredNullable<T>(pub Option<T>);

impl<T> RequiredNullable<T> {
    pub const fn null() -> Self {
        Self(None)
    }

    pub const fn some(value: T) -> Self {
        Self(Some(value))
    }

    pub const fn as_ref(&self) -> Option<&T> {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateBuildEnvironment {
    pub runner_os: String,
    pub runner_os_version: String,
    pub runner_arch: String,
    pub git_version: String,
    pub rustc_version: String,
    pub cargo_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub schema: String,
    pub candidate_id: String,
    pub candidate_path: String,
    pub source_revision: String,
    pub source_clean: bool,
    pub source_archive_algorithm: String,
    pub source_archive_sha256: String,
    pub target_triple: String,
    pub release_profile: String,
    pub binary_sha256: String,
    pub build_environment: CandidateBuildEnvironment,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostKind {
    Codex,
    ClaudeCode,
}

impl HostKind {
    pub const ALL: [Self; 2] = [Self::Codex, Self::ClaudeCode];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude_code",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImplementationDisposition {
    Implemented,
    UnsupportedByHost,
}

impl ImplementationDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::UnsupportedByHost => "unsupported_by_host",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Completed,
    Running,
    Ignored,
    NotApplicable,
}

impl RunState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Running => "running",
            Self::Ignored => "ignored",
            Self::NotApplicable => "not_applicable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellEnvironment {
    pub runner_os: String,
    pub runner_os_version: String,
    pub runner_arch: String,
    pub host_executable_sha256: RequiredNullable<String>,
    pub host_kind: HostKind,
    pub host_version: RequiredNullable<String>,
    pub adapter_profile: String,
    pub adapter_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellAssertion {
    pub assertion_id: String,
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding_codes: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub schema: String,
    pub candidate_id: String,
    pub binary_sha256: String,
    pub source_revision: String,
    pub target_triple: String,
    pub release_profile: String,
    pub host_kind: HostKind,
    pub host_version: RequiredNullable<String>,
    pub adapter_profile: String,
    pub adapter_version: String,
    pub feature: HostFeature,
    pub implementation_disposition: ImplementationDisposition,
    pub requested_verified: bool,
    pub claimed_status: HostFeatureSupportStatus,
    pub run_state: RunState,
    pub started_at: String,
    pub recorded_at: String,
    pub environment: CellEnvironment,
    pub assertions: Vec<CellAssertion>,
    pub evidence_artifact_path: RequiredNullable<String>,
    pub evidence_artifact_sha256: RequiredNullable<String>,
}

impl Cell {
    pub fn key(&self) -> String {
        format!(
            "{}/{}/{}",
            self.host_kind.as_str(),
            self.host_version
                .as_ref()
                .map(String::as_str)
                .unwrap_or("unavailable"),
            self.feature.as_str()
        )
    }

    pub fn matrix_key(&self) -> String {
        format!("{}/{}", self.host_kind.as_str(), self.feature.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestCell {
    pub raw: Cell,
    pub derived_status: HostFeatureSupportStatus,
    pub finding_codes: Vec<String>,
}

impl Serialize for ManifestCell {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut value = serde_json::to_value(&self.raw).map_err(serde::ser::Error::custom)?;
        let object = value
            .as_object_mut()
            .expect("Cell always serializes as an object");
        object.insert(
            "derived_status".to_owned(),
            serde_json::to_value(self.derived_status).map_err(serde::ser::Error::custom)?,
        );
        object.insert(
            "finding_codes".to_owned(),
            serde_json::to_value(&self.finding_codes).map_err(serde::ser::Error::custom)?,
        );
        value.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ManifestCell {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut value = Value::deserialize(deserializer)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| D::Error::custom("manifest cell must be an object"))?;
        let derived_status = object
            .remove("derived_status")
            .ok_or_else(|| D::Error::missing_field("derived_status"))?;
        let finding_codes = object
            .remove("finding_codes")
            .ok_or_else(|| D::Error::missing_field("finding_codes"))?;
        let raw = serde_json::from_value(value).map_err(D::Error::custom)?;
        let derived_status = serde_json::from_value(derived_status).map_err(D::Error::custom)?;
        let finding_codes = serde_json::from_value(finding_codes).map_err(D::Error::custom)?;
        Ok(Self {
            raw,
            derived_status,
            finding_codes,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateVerdict {
    Pass,
    PassWithDowngrades,
    Fail,
}

impl GateVerdict {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::PassWithDowngrades => "pass_with_downgrades",
            Self::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub schema: String,
    pub candidate: Candidate,
    pub evaluated_at: String,
    pub cells: Vec<ManifestCell>,
    pub requested_verified_claims: Vec<String>,
    pub downgrades: Vec<String>,
    pub invariant_findings: Vec<String>,
    pub verdict: GateVerdict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditInvariantResult {
    pub invariant_id: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecalculatedCell {
    pub host_kind: HostKind,
    pub host_version: RequiredNullable<String>,
    pub feature: HostFeature,
    pub derived_status: HostFeatureSupportStatus,
    pub finding_codes: Vec<String>,
}

impl From<&ManifestCell> for RecalculatedCell {
    fn from(cell: &ManifestCell) -> Self {
        Self {
            host_kind: cell.raw.host_kind,
            host_version: cell.raw.host_version.clone(),
            feature: cell.raw.feature,
            derived_status: cell.derived_status,
            finding_codes: cell.finding_codes.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditExclusion {
    pub check_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditVerdict {
    Pass,
    Fail,
}

impl AuditVerdict {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseAudit {
    pub schema: String,
    pub manifest_path: String,
    pub manifest_sha256: String,
    pub cell_directory: String,
    pub cell_inputs_sha256: String,
    pub candidate_path: String,
    pub candidate_sha256: String,
    pub started_at: String,
    pub evaluated_at: String,
    pub invariant_results: Vec<AuditInvariantResult>,
    pub recalculated_cells: Vec<RecalculatedCell>,
    pub findings: Vec<String>,
    pub exclusions: Vec<AuditExclusion>,
    pub recalculated_verdict: GateVerdict,
    pub audit_verdict: AuditVerdict,
}

pub fn expected_assertion_ids(
    disposition: ImplementationDisposition,
    feature: HostFeature,
) -> Vec<&'static str> {
    let mut ids = if disposition == ImplementationDisposition::UnsupportedByHost {
        vec!["static_unsupported_by_host"]
    } else {
        match feature {
            HostFeature::NativeUserAction => vec![
                "actual_host_session",
                "native_user_selector_observed",
                "operator_choice_confirmed",
                "same_connection_resume",
                "authority_receipt_observed",
            ],
            HostFeature::LocalWebUserChannel => vec![
                "actual_host_session",
                "trusted_capability_current",
                "host_owned_surface_observed",
                "model_visible_payload_absence_observed",
                "browser_submission_observed",
                "same_connection_resume",
                "strong_evidence_close_chain",
            ],
            HostFeature::VerifiedToolProducer => vec![
                "actual_host_tool_event",
                "intent_precedes_source",
                "exact_session_connection_actor_scope_baseline",
                "capture_receipt_bound",
                "strong_producer_chain",
                "criterion_coverage_projected",
                "negative_rejections_zero_effect",
            ],
            HostFeature::RegisteredConnectionObservation => vec![
                "actual_host_connection_event",
                "intent_precedes_source",
                "exact_session_connection_actor_scope_baseline",
                "capture_receipt_bound",
                "strong_producer_chain",
                "criterion_coverage_projected",
                "negative_rejections_zero_effect",
            ],
            HostFeature::RecordFinalOutput => vec![
                "actual_host_session",
                "authority_display_observed",
                "authenticated_exact_replay_observed",
            ],
            HostFeature::DetectiveFinalOutput => vec![
                "actual_host_session",
                "authority_display_observed",
                "authenticated_exact_replay_observed",
                "block_finalization_observed",
            ],
        }
    };
    ids.sort_unstable();
    ids
}
