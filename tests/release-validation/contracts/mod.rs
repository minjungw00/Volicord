use std::{collections::BTreeSet, fmt, fs, path::Path};

use chrono::{DateTime, SecondsFormat};
use serde::{
    de::{MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

use crate::{
    error::{ValidationError, ValidationResult},
    hosts::codex::has_exact_first_release_capabilities,
    scenarios::{scenarios_for_wsl2, BASE_SCENARIOS},
    schema::{
        CodexCapability, CodexReleaseCell, CodexReleaseScenarioResult,
        CodexReleaseValidationEvidence, PlatformEnvironment, ScenarioStatus,
        TestOnlyCodexDescriptor, ValidationEvidenceStatus,
    },
};

pub const CHECKED_IN_CODEX_RELEASE_MANIFEST_PATH: &str =
    "tests/release-validation/contracts/codex-release-manifest.json";
pub const UNSUPPORTED_HOST_ARTIFACT_REASON: &str = "unsupported_host_artifact";
const CHECKED_IN_MANIFEST_BYTES: &[u8] = include_bytes!("codex-release-manifest.json");
const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;
const EVIDENCE_DIGEST_DOMAIN: &[u8] = b"volicord.codex-release-validation-evidence\0";

const CELL_FIELDS: &[&str] = &[
    "artifact_digest",
    "platform",
    "observed_capabilities",
    "integration_profile",
    "validation_evidence",
];
const EVIDENCE_FIELDS: &[&str] = &[
    "status",
    "artifact_digest",
    "platform",
    "observed_capabilities",
    "integration_profile",
    "volicord_artifact_digest",
    "runner",
    "scenario_results",
    "evidence_digest",
    "observed_at",
];
const RUNNER_FIELDS: &[&str] = &[
    "runner_id",
    "target_triple",
    "architecture",
    "os_release",
    "environment_image",
];
const SCENARIO_RESULT_FIELDS: &[&str] = &[
    "scenario_id",
    "status",
    "reason",
    "evidence_digest",
    "observed_at",
];
const TEST_ONLY_DESCRIPTOR_FIELDS: &[&str] = &[
    "test_only",
    "fixture_id",
    "artifact_digest",
    "platform",
    "observed_capabilities",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexReleaseManifest {
    cells: Vec<CodexReleaseCell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformReleaseStatus {
    Passed,
    Failed,
    Unavailable,
    NotRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedHostArtifact;

impl UnsupportedHostArtifact {
    pub const fn reason(self) -> &'static str {
        UNSUPPORTED_HOST_ARTIFACT_REASON
    }
}

impl fmt::Display for UnsupportedHostArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(UNSUPPORTED_HOST_ARTIFACT_REASON)
    }
}

impl std::error::Error for UnsupportedHostArtifact {}

impl CodexReleaseManifest {
    pub fn cells(&self) -> &[CodexReleaseCell] {
        &self.cells
    }

    pub fn platform_status(&self, platform: PlatformEnvironment) -> PlatformReleaseStatus {
        self.cells
            .iter()
            .find(|cell| cell.platform == platform)
            .map(|cell| match cell.validation_evidence.status {
                ValidationEvidenceStatus::Passed => PlatformReleaseStatus::Passed,
                ValidationEvidenceStatus::Failed => PlatformReleaseStatus::Failed,
                ValidationEvidenceStatus::Unavailable => PlatformReleaseStatus::Unavailable,
            })
            .unwrap_or(PlatformReleaseStatus::NotRun)
    }

    pub fn has_four_passing_platforms(&self) -> bool {
        PlatformEnvironment::ALL
            .into_iter()
            .all(|platform| self.platform_status(platform) == PlatformReleaseStatus::Passed)
    }

    pub fn lookup_supported_cell(
        &self,
        artifact_digest: &str,
        platform: PlatformEnvironment,
        observed_capabilities: &[CodexCapability],
        integration_profile: &str,
    ) -> Result<&CodexReleaseCell, UnsupportedHostArtifact> {
        if !has_exact_first_release_capabilities(observed_capabilities)
            || integration_profile != "record"
        {
            return Err(UnsupportedHostArtifact);
        }

        self.cells
            .iter()
            .find(|cell| {
                cell.validation_evidence.status == ValidationEvidenceStatus::Passed
                    && cell.artifact_digest == artifact_digest
                    && cell.platform == platform
                    && cell.observed_capabilities == observed_capabilities
                    && cell.integration_profile.as_str() == integration_profile
            })
            .ok_or(UnsupportedHostArtifact)
    }
}

pub fn checked_in_manifest() -> ValidationResult<CodexReleaseManifest> {
    parse_manifest(CHECKED_IN_MANIFEST_BYTES)
}

pub fn load_manifest(path: &Path) -> ValidationResult<CodexReleaseManifest> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(ValidationError::new(
            "Codex release manifest must be a regular file",
        ));
    }
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(ValidationError::new(
            "Codex release manifest exceeds its byte bound",
        ));
    }
    parse_manifest(&fs::read(path)?)
}

pub fn parse_manifest(bytes: &[u8]) -> ValidationResult<CodexReleaseManifest> {
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(ValidationError::new(
            "Codex release manifest exceeds its byte bound",
        ));
    }

    let ordered = parse_ordered_json(bytes)?;
    validate_manifest_json_shape(&ordered)?;
    let cells: Vec<CodexReleaseCell> = serde_json::from_value(ordered.into_json())?;
    validate_manifest_cells(&cells)?;
    Ok(CodexReleaseManifest { cells })
}

pub fn parse_test_only_descriptor(bytes: &[u8]) -> ValidationResult<TestOnlyCodexDescriptor> {
    let ordered = parse_ordered_json(bytes)?;
    require_exact_fields(
        &ordered,
        TEST_ONLY_DESCRIPTOR_FIELDS,
        "TestOnlyCodexDescriptor",
    )?;
    let descriptor: TestOnlyCodexDescriptor = serde_json::from_value(ordered.into_json())?;
    if !descriptor.test_only {
        return Err(ValidationError::new(
            "TestOnlyCodexDescriptor.test_only must be true",
        ));
    }
    validate_raw_sha256(
        "TestOnlyCodexDescriptor.artifact_digest",
        &descriptor.artifact_digest,
    )?;
    Ok(descriptor)
}

pub fn compute_evidence_digest(
    evidence: &CodexReleaseValidationEvidence,
) -> ValidationResult<String> {
    let runner = record(vec![
        ("runner_id", string(&evidence.runner.runner_id)?),
        ("target_triple", string(&evidence.runner.target_triple)?),
        (
            "architecture",
            string(evidence.runner.architecture.as_str())?,
        ),
        ("os_release", string(&evidence.runner.os_release)?),
        (
            "environment_image",
            string(&evidence.runner.environment_image)?,
        ),
    ])?;

    let scenario_results = evidence
        .scenario_results
        .iter()
        .map(encode_scenario_result)
        .collect::<ValidationResult<Vec<_>>>()?;
    let capabilities = evidence
        .observed_capabilities
        .iter()
        .map(|capability| string(capability.as_str()))
        .collect::<ValidationResult<Vec<_>>>()?;

    let canonical = record(vec![
        ("status", string(evidence.status.as_str())?),
        ("artifact_digest", string(&evidence.artifact_digest)?),
        ("platform", string(evidence.platform.as_str())?),
        ("observed_capabilities", list(capabilities)?),
        (
            "integration_profile",
            string(evidence.integration_profile.as_str())?,
        ),
        (
            "volicord_artifact_digest",
            string(&evidence.volicord_artifact_digest)?,
        ),
        ("runner", runner),
        ("scenario_results", list(scenario_results)?),
        ("observed_at", string(&evidence.observed_at)?),
    ])?;

    let mut hasher = Sha256::new();
    hasher.update(EVIDENCE_DIGEST_DOMAIN);
    hasher.update(canonical);
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_manifest_json_shape(value: &OrderedJsonValue) -> ValidationResult<()> {
    let OrderedJsonValue::Array(cells) = value else {
        return Err(ValidationError::new(
            "Codex release manifest must be a JSON array",
        ));
    };
    for (index, cell) in cells.iter().enumerate() {
        let cell_values =
            require_exact_fields(cell, CELL_FIELDS, &format!("CodexReleaseCell[{index}]"))?;
        let evidence_values = require_exact_fields(
            cell_values[4],
            EVIDENCE_FIELDS,
            &format!("CodexReleaseCell[{index}].validation_evidence"),
        )?;
        require_exact_fields(
            evidence_values[6],
            RUNNER_FIELDS,
            &format!("CodexReleaseCell[{index}].validation_evidence.runner"),
        )?;
        let OrderedJsonValue::Array(results) = evidence_values[7] else {
            return Err(ValidationError::new(format!(
                "CodexReleaseCell[{index}].validation_evidence.scenario_results must be an array"
            )));
        };
        for (scenario_index, result) in results.iter().enumerate() {
            require_exact_fields(
                result,
                SCENARIO_RESULT_FIELDS,
                &format!(
                    "CodexReleaseCell[{index}].validation_evidence.scenario_results[{scenario_index}]"
                ),
            )?;
        }
    }
    Ok(())
}

fn validate_manifest_cells(cells: &[CodexReleaseCell]) -> ValidationResult<()> {
    if cells.len() > PlatformEnvironment::ALL.len() {
        return Err(ValidationError::new(
            "Codex release manifest may contain at most four cells",
        ));
    }

    let mut previous_platform_index = None;
    for cell in cells {
        let platform_index = PlatformEnvironment::ALL
            .iter()
            .position(|platform| platform == &cell.platform)
            .expect("PlatformEnvironment is closed");
        if previous_platform_index.is_some_and(|previous| previous >= platform_index) {
            return Err(ValidationError::new(
                "Codex release manifest cells must be unique and in linux, macos, native_windows, wsl2 order",
            ));
        }
        previous_platform_index = Some(platform_index);
        validate_cell(cell)?;
    }
    Ok(())
}

fn validate_cell(cell: &CodexReleaseCell) -> ValidationResult<()> {
    validate_raw_sha256("CodexReleaseCell.artifact_digest", &cell.artifact_digest)?;
    if !has_exact_first_release_capabilities(&cell.observed_capabilities) {
        return Err(ValidationError::new(
            "CodexReleaseCell.observed_capabilities must equal FirstReleaseCodexCapabilities",
        ));
    }

    let evidence = &cell.validation_evidence;
    if evidence.artifact_digest != cell.artifact_digest
        || evidence.platform != cell.platform
        || evidence.observed_capabilities != cell.observed_capabilities
        || evidence.integration_profile != cell.integration_profile
    {
        return Err(ValidationError::new(
            "Codex release evidence coordinates must exactly match the owning cell",
        ));
    }

    validate_evidence(evidence)
}

fn validate_evidence(evidence: &CodexReleaseValidationEvidence) -> ValidationResult<()> {
    validate_raw_sha256(
        "validation_evidence.artifact_digest",
        &evidence.artifact_digest,
    )?;
    validate_raw_sha256(
        "validation_evidence.volicord_artifact_digest",
        &evidence.volicord_artifact_digest,
    )?;
    validate_raw_sha256(
        "validation_evidence.evidence_digest",
        &evidence.evidence_digest,
    )?;
    validate_canonical_utc_timestamp("validation_evidence.observed_at", &evidence.observed_at)?;
    if !has_exact_first_release_capabilities(&evidence.observed_capabilities) {
        return Err(ValidationError::new(
            "validation_evidence.observed_capabilities must equal FirstReleaseCodexCapabilities",
        ));
    }

    validate_bounded_runner_string("runner.runner_id", &evidence.runner.runner_id, 256)?;
    validate_bounded_runner_string("runner.target_triple", &evidence.runner.target_triple, 256)?;
    validate_bounded_runner_string("runner.os_release", &evidence.runner.os_release, 512)?;
    validate_bounded_runner_string(
        "runner.environment_image",
        &evidence.runner.environment_image,
        512,
    )?;

    let expected_scenarios = if evidence.platform == PlatformEnvironment::Wsl2 {
        scenarios_for_wsl2()
    } else {
        BASE_SCENARIOS.to_vec()
    };
    let actual_scenarios = evidence
        .scenario_results
        .iter()
        .map(|result| result.scenario_id)
        .collect::<Vec<_>>();
    if actual_scenarios != expected_scenarios {
        return Err(ValidationError::new(
            "validation_evidence.scenario_results must contain the exact ordered platform catalog",
        ));
    }

    for result in &evidence.scenario_results {
        validate_scenario_result(result)?;
    }
    validate_evidence_status(evidence)?;

    let expected_digest = compute_evidence_digest(evidence)?;
    if evidence.evidence_digest != expected_digest {
        return Err(ValidationError::new(
            "validation_evidence.evidence_digest does not match canonical evidence bytes",
        ));
    }
    Ok(())
}

fn validate_scenario_result(result: &CodexReleaseScenarioResult) -> ValidationResult<()> {
    let reason = result.reason.as_ref();
    let digest = result.evidence_digest.as_ref();
    let observed_at = result.observed_at.as_ref();

    match result.status {
        ScenarioStatus::Passed => {
            if reason.is_some() || digest.is_none() || observed_at.is_none() {
                return Err(ValidationError::new(
                    "passed scenario requires null reason and non-null digest and observed_at",
                ));
            }
        }
        ScenarioStatus::Failed => {
            if reason.is_none() || digest.is_none() || observed_at.is_none() {
                return Err(ValidationError::new(
                    "failed scenario requires non-null reason, digest, and observed_at",
                ));
            }
        }
        ScenarioStatus::Unavailable => {
            if reason.is_none() || observed_at.is_none() {
                return Err(ValidationError::new(
                    "unavailable scenario requires non-null reason and observed_at",
                ));
            }
        }
        ScenarioStatus::NotRun => {
            if reason.is_none() || digest.is_some() || observed_at.is_some() {
                return Err(ValidationError::new(
                    "not_run scenario requires non-null reason and null digest and observed_at",
                ));
            }
        }
    }

    if let Some(reason) = reason {
        validate_reason(reason)?;
    }
    if let Some(digest) = digest {
        validate_raw_sha256("scenario_results[].evidence_digest", digest)?;
    }
    if let Some(observed_at) = observed_at {
        validate_canonical_utc_timestamp("scenario_results[].observed_at", observed_at)?;
    }
    Ok(())
}

fn validate_evidence_status(evidence: &CodexReleaseValidationEvidence) -> ValidationResult<()> {
    let has_failed = evidence
        .scenario_results
        .iter()
        .any(|result| result.status == ScenarioStatus::Failed);
    let has_unavailable = evidence
        .scenario_results
        .iter()
        .any(|result| result.status == ScenarioStatus::Unavailable);
    let all_passed = evidence
        .scenario_results
        .iter()
        .all(|result| result.status == ScenarioStatus::Passed);

    let valid = match evidence.status {
        ValidationEvidenceStatus::Passed => all_passed,
        ValidationEvidenceStatus::Failed => has_failed,
        ValidationEvidenceStatus::Unavailable => !has_failed && has_unavailable,
    };
    if !valid {
        return Err(ValidationError::new(
            "validation_evidence.status does not match its scenario results",
        ));
    }
    Ok(())
}

fn validate_raw_sha256(name: &str, value: &str) -> ValidationResult<()> {
    if value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        Ok(())
    } else {
        Err(ValidationError::new(format!(
            "{name} must be 64 lowercase hexadecimal characters"
        )))
    }
}

fn validate_bounded_runner_string(
    name: &str,
    value: &str,
    max_bytes: usize,
) -> ValidationResult<()> {
    if value.is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        Err(ValidationError::new(format!(
            "{name} must be nonempty, control-free UTF-8 of at most {max_bytes} bytes"
        )))
    } else {
        Ok(())
    }
}

fn validate_reason(reason: &str) -> ValidationResult<()> {
    let bytes = reason.as_bytes();
    let valid = !bytes.is_empty()
        && bytes.len() <= 128
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .skip(1)
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(ValidationError::new(
            "scenario reason must match [a-z][a-z0-9_]{0,127}",
        ))
    }
}

fn validate_canonical_utc_timestamp(name: &str, value: &str) -> ValidationResult<()> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| ValidationError::new(format!("{name} must be canonical RFC 3339 UTC")))?;
    if !value.ends_with('Z') || parsed.to_rfc3339_opts(SecondsFormat::AutoSi, true) != value {
        return Err(ValidationError::new(format!(
            "{name} must be canonical RFC 3339 UTC"
        )));
    }
    Ok(())
}

fn encode_scenario_result(result: &CodexReleaseScenarioResult) -> ValidationResult<Vec<u8>> {
    record(vec![
        ("scenario_id", string(result.scenario_id.as_str())?),
        ("status", string(result.status.as_str())?),
        (
            "reason",
            nullable(
                result
                    .reason
                    .as_ref()
                    .map(|value| string(value))
                    .transpose()?,
            )?,
        ),
        (
            "evidence_digest",
            nullable(
                result
                    .evidence_digest
                    .as_ref()
                    .map(|value| string(value))
                    .transpose()?,
            )?,
        ),
        (
            "observed_at",
            nullable(
                result
                    .observed_at
                    .as_ref()
                    .map(|value| string(value))
                    .transpose()?,
            )?,
        ),
    ])
}

fn u32be(value: usize) -> ValidationResult<[u8; 4]> {
    let value = u32::try_from(value)
        .map_err(|_| ValidationError::new("canonical evidence length exceeds u32"))?;
    Ok(value.to_be_bytes())
}

fn blob(value: &[u8]) -> ValidationResult<Vec<u8>> {
    let mut encoded = Vec::with_capacity(4 + value.len());
    encoded.extend_from_slice(&u32be(value.len())?);
    encoded.extend_from_slice(value);
    Ok(encoded)
}

fn string(value: &str) -> ValidationResult<Vec<u8>> {
    blob(value.as_bytes())
}

fn list(items: Vec<Vec<u8>>) -> ValidationResult<Vec<u8>> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&u32be(items.len())?);
    for item in items {
        encoded.extend_from_slice(&blob(&item)?);
    }
    Ok(encoded)
}

fn record(fields: Vec<(&str, Vec<u8>)>) -> ValidationResult<Vec<u8>> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&u32be(fields.len())?);
    for (name, value) in fields {
        encoded.extend_from_slice(&string(name)?);
        encoded.extend_from_slice(&blob(&value)?);
    }
    Ok(encoded)
}

fn nullable(value: Option<Vec<u8>>) -> ValidationResult<Vec<u8>> {
    match value {
        None => Ok(vec![0]),
        Some(value) => {
            let mut encoded = vec![1];
            encoded.extend_from_slice(&blob(&value)?);
            Ok(encoded)
        }
    }
}

fn require_exact_fields<'a>(
    value: &'a OrderedJsonValue,
    expected: &[&str],
    name: &str,
) -> ValidationResult<Vec<&'a OrderedJsonValue>> {
    let OrderedJsonValue::Object(fields) = value else {
        return Err(ValidationError::new(format!("{name} must be an object")));
    };
    let actual = fields
        .iter()
        .map(|(field, _)| field.as_str())
        .collect::<Vec<_>>();
    if actual != expected {
        return Err(ValidationError::new(format!(
            "{name} fields must be present exactly once in canonical order"
        )));
    }
    Ok(fields.iter().map(|(_, value)| value).collect())
}

fn parse_ordered_json(bytes: &[u8]) -> ValidationResult<OrderedJsonValue> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = OrderedJsonValue::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}

#[derive(Debug, Clone, PartialEq)]
enum OrderedJsonValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl OrderedJsonValue {
    fn into_json(self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Bool(value) => Value::Bool(value),
            Self::Number(value) => Value::Number(value),
            Self::String(value) => Value::String(value),
            Self::Array(values) => Value::Array(values.into_iter().map(Self::into_json).collect()),
            Self::Object(fields) => Value::Object(
                fields
                    .into_iter()
                    .map(|(name, value)| (name, value.into_json()))
                    .collect::<Map<_, _>>(),
            ),
        }
    }
}

impl<'de> Deserialize<'de> for OrderedJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(OrderedJsonVisitor)
    }
}

struct OrderedJsonVisitor;

impl<'de> Visitor<'de> for OrderedJsonVisitor {
    type Value = OrderedJsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object fields")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(OrderedJsonValue::Number)
            .ok_or_else(|| E::custom("JSON number is not finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::String(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element()? {
            values.push(value);
        }
        Ok(OrderedJsonValue::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        let mut fields = Vec::new();
        while let Some((name, value)) = map.next_entry::<String, OrderedJsonValue>()? {
            if !seen.insert(name.clone()) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate JSON field {name}"
                )));
            }
            fields.push((name, value));
        }
        Ok(OrderedJsonValue::Object(fields))
    }
}
