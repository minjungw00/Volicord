//! Shared bounded diagnostic envelope.
//!
//! This module owns the cross-crate finding and report representation. Domain
//! crates retain ownership of their closed diagnostic-code vocabularies and
//! exhaustive error-to-finding conversions. Typed domain fact structs opt in
//! to [`DiagnosticFactSource`] and pass through the single projection here;
//! callers cannot construct an unbounded [`DiagnosticFacts`] value directly.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    str::FromStr,
};

use schemars::JsonSchema;
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::{
    AgentConnectionId, AgentRuntimeSessionId, IntegrationRevision, ProjectId, UtcTimestamp,
};

/// The only current JSON representation version for [`DiagnosticReport`].
pub const DIAGNOSTIC_REPORT_SCHEMA_VERSION: u32 = 1;
/// Stable fallback code for failures that cannot be classified more narrowly.
pub const INTERNAL_UNEXPECTED_FAILURE_CODE: &str = "internal.unexpected_failure";
/// Maximum UTF-8 byte length of a namespaced diagnostic code.
pub const MAX_DIAGNOSTIC_CODE_BYTES: usize = 192;
/// Maximum UTF-8 byte length of a finding or correlation identifier.
pub const MAX_DIAGNOSTIC_IDENTIFIER_BYTES: usize = 192;
/// Maximum UTF-8 byte length of a diagnostic domain, stage, or fact key.
pub const MAX_DIAGNOSTIC_NAME_BYTES: usize = 128;
/// Maximum UTF-8 byte length retained for one projected fact string.
pub const MAX_DIAGNOSTIC_FACT_STRING_BYTES: usize = 1_024;
/// Maximum entries retained in one projected fact collection.
pub const MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS: usize = 32;
/// Maximum nested object/array depth retained in projected facts.
pub const MAX_DIAGNOSTIC_FACT_DEPTH: usize = 4;
/// Maximum serialized byte length of one projected fact set.
pub const MAX_DIAGNOSTIC_FACT_BYTES: usize = 16 * 1_024;
/// Maximum number of cause edges on one finding.
pub const MAX_DIAGNOSTIC_CAUSES: usize = 32;
/// Maximum number of recommended actions on one finding.
pub const MAX_DIAGNOSTIC_ACTIONS: usize = 32;
/// Maximum number of findings in one report.
pub const MAX_DIAGNOSTIC_FINDINGS: usize = 128;
/// Maximum number of explicitly identified independent root causes.
pub const MAX_DIAGNOSTIC_ROOT_CAUSES: usize = 64;
/// Maximum number of report limitations.
pub const MAX_DIAGNOSTIC_LIMITS: usize = 32;
/// Maximum serialized byte length of one complete report.
pub const MAX_DIAGNOSTIC_REPORT_BYTES: usize = 1024 * 1024;

const REDACTED_VALUE: &str = "[redacted]";
const DEPTH_LIMIT_VALUE: &str = "[depth limit]";
const TRUNCATED_SUFFIX: &str = "...[truncated]";

/// Stable identifier for one diagnostic finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct DiagnosticFindingId(String);

impl DiagnosticFindingId {
    /// Validates one bounded stable finding identifier.
    pub fn parse(value: impl Into<String>) -> Result<Self, DiagnosticError> {
        let value = value.into();
        validate_stable_identifier("diagnostic finding id", &value)?;
        Ok(Self(value))
    }

    /// Returns the stable identifier spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the identifier.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for DiagnosticFindingId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl fmt::Display for DiagnosticFindingId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for DiagnosticFindingId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for DiagnosticFindingId {
    type Err = DiagnosticError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Validated stable namespaced diagnostic code.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct DiagnosticCode(String);

impl DiagnosticCode {
    /// Validates `[a-z][a-z0-9_]*` segments separated by dots.
    pub fn parse(value: impl Into<String>) -> Result<Self, DiagnosticError> {
        let value = value.into();
        if value.len() > MAX_DIAGNOSTIC_CODE_BYTES {
            return Err(invalid(format!(
                "diagnostic code exceeds {MAX_DIAGNOSTIC_CODE_BYTES} UTF-8 bytes"
            )));
        }
        let mut segments = value.split('.');
        let Some(first) = segments.next() else {
            return Err(invalid("diagnostic code must not be empty"));
        };
        validate_name("diagnostic code segment", first)?;
        let mut segment_count = 1;
        for segment in segments {
            validate_name("diagnostic code segment", segment)?;
            segment_count += 1;
        }
        if segment_count < 2 {
            return Err(invalid(
                "diagnostic code must contain at least two dot-separated segments",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the validated stable spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the first namespaced code segment.
    pub fn namespace(&self) -> &str {
        self.0
            .split_once('.')
            .map_or(self.as_str(), |(value, _)| value)
    }

    /// Consumes the wrapper and returns the code.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for DiagnosticCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for DiagnosticCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for DiagnosticCode {
    type Err = DiagnosticError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Bounded owner domain for a diagnostic finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct DiagnosticDomain(String);

impl DiagnosticDomain {
    /// Validates one stable lowercase domain name.
    pub fn parse(value: impl Into<String>) -> Result<Self, DiagnosticError> {
        let value = value.into();
        validate_name("diagnostic domain", &value)?;
        Ok(Self(value))
    }

    /// Returns the stable domain spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for DiagnosticDomain {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl fmt::Display for DiagnosticDomain {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DiagnosticDomain {
    type Err = DiagnosticError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Bounded execution stage at which a finding was observed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct DiagnosticStage(String);

impl DiagnosticStage {
    /// Validates one stable lowercase stage name.
    pub fn parse(value: impl Into<String>) -> Result<Self, DiagnosticError> {
        let value = value.into();
        validate_name("diagnostic stage", &value)?;
        Ok(Self(value))
    }

    /// Returns the stable stage spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for DiagnosticStage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl fmt::Display for DiagnosticStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DiagnosticStage {
    type Err = DiagnosticError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Severity of one diagnostic finding.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    /// Informational observation or successful check fact.
    Info,
    /// Actionable or incomplete observation that did not itself stop the operation.
    Warning,
    /// Failure that stopped or invalidated the observed operation.
    Error,
}

/// Bounded subject identified by a stable kind and safe reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct DiagnosticSubject {
    kind: String,
    reference: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticSubjectWire {
    kind: String,
    reference: String,
}

impl<'de> Deserialize<'de> for DiagnosticSubject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticSubjectWire::deserialize(deserializer)?;
        Self::try_new(wire.kind, wire.reference).map_err(de::Error::custom)
    }
}

impl DiagnosticSubject {
    /// Validates one bounded diagnostic subject.
    pub fn try_new(
        kind: impl Into<String>,
        reference: impl Into<String>,
    ) -> Result<Self, DiagnosticError> {
        let kind = kind.into();
        let reference = reference.into();
        validate_name("diagnostic subject kind", &kind)?;
        validate_bounded_text(
            "diagnostic subject reference",
            &reference,
            MAX_DIAGNOSTIC_FACT_STRING_BYTES,
        )?;
        Ok(Self { kind, reference })
    }

    /// Returns the stable subject kind.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the bounded safe subject reference.
    pub fn reference(&self) -> &str {
        &self.reference
    }
}

/// Marker implemented explicitly by each closed typed fact struct.
///
/// The marker deliberately has no blanket implementation. This prevents
/// callers from passing `serde_json::Value` or another arbitrary JSON carrier
/// to [`DiagnosticFacts::project`] without first defining an owner type.
pub trait DiagnosticFactSource: Serialize {}

/// Bounded, redacted projection of one typed diagnostic fact struct.
///
/// Projection sorts object keys, truncates individual strings, collections,
/// and nested containers at the public limits above, and rejects a projection
/// that still exceeds the total serialized-byte limit. Fields representing
/// credentials, secrets, environment values, request bodies, tool arguments,
/// unrestricted stderr, or filesystem contents are replaced by a fixed marker
/// and listed by safe field path.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct DiagnosticFacts {
    data: BTreeMap<String, Value>,
    redacted_fields: Vec<String>,
    truncated: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticFactsWire {
    data: BTreeMap<String, Value>,
    redacted_fields: Vec<String>,
    truncated: bool,
}

impl<'de> Deserialize<'de> for DiagnosticFacts {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticFactsWire::deserialize(deserializer)?;
        let mut declared_redacted_fields = wire.redacted_fields;
        for field in &declared_redacted_fields {
            validate_bounded_text(
                "diagnostic redacted-field path",
                field,
                MAX_DIAGNOSTIC_FACT_STRING_BYTES,
            )
            .map_err(de::Error::custom)?;
        }
        declared_redacted_fields.sort();
        if declared_redacted_fields
            .windows(2)
            .any(|pair| pair[0] == pair[1])
        {
            return Err(de::Error::custom(
                "diagnostic facts contain duplicate redacted-field paths",
            ));
        }
        let projected = Self::project_map(wire.data).map_err(de::Error::custom)?;
        if declared_redacted_fields != projected.redacted_fields {
            return Err(de::Error::custom(
                "diagnostic redacted-field paths do not match projected data",
            ));
        }
        let facts = Self {
            truncated: wire.truncated || projected.truncated,
            ..projected
        };
        facts.validate_size().map_err(de::Error::custom)?;
        Ok(facts)
    }
}

impl DiagnosticFacts {
    /// Projects a typed owner fact struct through the shared bounding and
    /// redaction policy.
    pub fn project<T: DiagnosticFactSource>(source: &T) -> Result<Self, DiagnosticError> {
        let value = serde_json::to_value(source)
            .map_err(|_| invalid("typed diagnostic facts could not be serialized"))?;
        let Value::Object(map) = value else {
            return Err(invalid(
                "typed diagnostic facts must serialize as one JSON object",
            ));
        };
        Self::project_map(map.into_iter().collect())
    }

    /// Returns an empty bounded fact projection.
    pub fn empty() -> Self {
        Self {
            data: BTreeMap::new(),
            redacted_fields: Vec::new(),
            truncated: false,
        }
    }

    /// Returns projected fact data in deterministic key order.
    pub fn data(&self) -> &BTreeMap<String, Value> {
        &self.data
    }

    /// Returns the bounded paths whose values were removed by policy.
    pub fn redacted_fields(&self) -> &[String] {
        &self.redacted_fields
    }

    /// Returns whether any string, collection, or nested value was truncated.
    pub const fn truncated(&self) -> bool {
        self.truncated
    }

    fn project_map(data: BTreeMap<String, Value>) -> Result<Self, DiagnosticError> {
        let mut state = FactProjectionState::default();
        let projected = project_object(data, 0, "", &mut state)?;
        let mut redacted_fields = state.redacted_fields.into_iter().collect::<Vec<_>>();
        if redacted_fields.len() > MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS {
            redacted_fields.truncate(MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS);
            state.truncated = true;
        }
        let facts = Self {
            data: projected,
            redacted_fields,
            truncated: state.truncated,
        };
        facts.validate_size()?;
        Ok(facts)
    }

    fn validate_size(&self) -> Result<(), DiagnosticError> {
        let size = serde_json::to_vec(self)
            .map_err(|_| invalid("diagnostic facts could not be serialized"))?
            .len();
        if size > MAX_DIAGNOSTIC_FACT_BYTES {
            Err(invalid(format!(
                "diagnostic facts exceed {MAX_DIAGNOSTIC_FACT_BYTES} serialized bytes"
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
struct FactProjectionState {
    redacted_fields: BTreeSet<String>,
    truncated: bool,
}

fn project_object(
    data: BTreeMap<String, Value>,
    depth: usize,
    path: &str,
    state: &mut FactProjectionState,
) -> Result<BTreeMap<String, Value>, DiagnosticError> {
    let mut projected = BTreeMap::new();
    let original_len = data.len();
    for (key, value) in data.into_iter().take(MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS) {
        validate_fact_key(&key)?;
        let child_path = fact_path(path, &key);
        if is_sensitive_fact_key(&key) {
            state.redacted_fields.insert(child_path);
            projected.insert(key, Value::String(REDACTED_VALUE.to_owned()));
        } else {
            projected.insert(
                key,
                project_fact_value(value, depth + 1, &child_path, state)?,
            );
        }
    }
    if original_len > MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS {
        state.truncated = true;
    }
    Ok(projected)
}

fn project_fact_value(
    value: Value,
    depth: usize,
    path: &str,
    state: &mut FactProjectionState,
) -> Result<Value, DiagnosticError> {
    match value {
        Value::Object(_) if depth > MAX_DIAGNOSTIC_FACT_DEPTH => {
            state.truncated = true;
            Ok(Value::String(DEPTH_LIMIT_VALUE.to_owned()))
        }
        Value::Array(_) if depth > MAX_DIAGNOSTIC_FACT_DEPTH => {
            state.truncated = true;
            Ok(Value::String(DEPTH_LIMIT_VALUE.to_owned()))
        }
        Value::Object(map) => project_object(map.into_iter().collect(), depth, path, state)
            .map(|object| Value::Object(object.into_iter().collect())),
        Value::Array(values) => {
            let original_len = values.len();
            let mut projected = Vec::new();
            for (index, value) in values
                .into_iter()
                .take(MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS)
                .enumerate()
            {
                projected.push(project_fact_value(
                    value,
                    depth + 1,
                    &format!("{path}[{index}]"),
                    state,
                )?);
            }
            if original_len > MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS {
                state.truncated = true;
            }
            Ok(Value::Array(projected))
        }
        Value::String(value) => {
            let (value, truncated) = truncate_utf8(&value, MAX_DIAGNOSTIC_FACT_STRING_BYTES);
            state.truncated |= truncated;
            Ok(Value::String(value))
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(value),
    }
}

fn validate_fact_key(value: &str) -> Result<(), DiagnosticError> {
    if value.is_empty()
        || value.len() > MAX_DIAGNOSTIC_NAME_BYTES
        || value.chars().any(char::is_control)
    {
        Err(invalid(format!(
            "diagnostic fact keys must be 1 through {MAX_DIAGNOSTIC_NAME_BYTES} UTF-8 bytes and contain no control characters"
        )))
    } else {
        Ok(())
    }
}

fn fact_path(parent: &str, key: &str) -> String {
    let path = if parent.is_empty() {
        key.to_owned()
    } else {
        format!("{parent}.{key}")
    };
    truncate_utf8(&path, MAX_DIAGNOSTIC_FACT_STRING_BYTES).0
}

fn is_sensitive_fact_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| match character {
            '-' | ' ' => '_',
            other => other,
        })
        .collect::<String>();
    matches!(
        normalized.as_str(),
        "secret"
            | "secrets"
            | "secret_key"
            | "secretkey"
            | "token"
            | "tokens"
            | "api_key"
            | "apikey"
            | "api_token"
            | "apitoken"
            | "access_key"
            | "accesskey"
            | "access_token"
            | "accesstoken"
            | "password"
            | "passwd"
            | "credential"
            | "credentials"
            | "authorization"
            | "cookie"
            | "headers"
            | "request_headers"
            | "requestheaders"
            | "http_headers"
            | "httpheaders"
            | "environment"
            | "environment_values"
            | "environmentvalues"
            | "env"
            | "env_vars"
            | "envvars"
            | "request"
            | "request_body"
            | "requestbody"
            | "raw_request"
            | "rawrequest"
            | "body"
            | "tool_arguments"
            | "toolarguments"
            | "arguments"
            | "args"
            | "stderr"
            | "file_contents"
            | "filecontents"
            | "filesystem_contents"
            | "filesystemcontents"
            | "raw_contents"
            | "rawcontents"
    ) || normalized.ends_with("_secret")
        || normalized.ends_with("_token")
        || normalized.ends_with("_password")
        || normalized.ends_with("_credential")
        || normalized.ends_with("_credentials")
}

fn truncate_utf8(value: &str, max_bytes: usize) -> (String, bool) {
    if value.len() <= max_bytes {
        return (value.to_owned(), false);
    }
    let suffix = if TRUNCATED_SUFFIX.len() <= max_bytes {
        TRUNCATED_SUFFIX
    } else {
        ""
    };
    let mut end = max_bytes - suffix.len();
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    let mut output = value[..end].to_owned();
    output.push_str(suffix);
    (output, true)
}

/// One cause edge to another finding in the same report.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticCause {
    finding_id: DiagnosticFindingId,
}

impl DiagnosticCause {
    /// Creates one cause edge.
    pub fn new(finding_id: DiagnosticFindingId) -> Self {
        Self { finding_id }
    }

    /// Returns the referenced cause finding ID.
    pub fn finding_id(&self) -> &DiagnosticFindingId {
        &self.finding_id
    }
}

impl From<DiagnosticFindingId> for DiagnosticCause {
    fn from(finding_id: DiagnosticFindingId) -> Self {
        Self::new(finding_id)
    }
}

/// Bounded recommended action attached to one finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct DiagnosticAction {
    code: DiagnosticCode,
    summary: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticActionWire {
    code: DiagnosticCode,
    summary: String,
}

impl<'de> Deserialize<'de> for DiagnosticAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticActionWire::deserialize(deserializer)?;
        Self::try_new(wire.code, wire.summary).map_err(de::Error::custom)
    }
}

impl DiagnosticAction {
    /// Validates one recommended action.
    pub fn try_new(
        code: DiagnosticCode,
        summary: impl Into<String>,
    ) -> Result<Self, DiagnosticError> {
        let summary = summary.into();
        validate_bounded_text(
            "diagnostic action summary",
            &summary,
            MAX_DIAGNOSTIC_FACT_STRING_BYTES,
        )?;
        Ok(Self { code, summary })
    }

    /// Returns the stable action code.
    pub fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    /// Returns the bounded safe action summary.
    pub fn summary(&self) -> &str {
        &self.summary
    }
}

/// One shared structured diagnostic finding.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct DiagnosticFinding {
    id: DiagnosticFindingId,
    code: DiagnosticCode,
    domain: DiagnosticDomain,
    stage: DiagnosticStage,
    severity: DiagnosticSeverity,
    subject: DiagnosticSubject,
    facts: DiagnosticFacts,
    causes: Vec<DiagnosticCause>,
    actions: Vec<DiagnosticAction>,
    observed_at: UtcTimestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connection_id: Option<AgentConnectionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<ProjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime_session_id: Option<AgentRuntimeSessionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    integration_revision: Option<IntegrationRevision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticFindingWire {
    id: DiagnosticFindingId,
    code: DiagnosticCode,
    domain: DiagnosticDomain,
    stage: DiagnosticStage,
    severity: DiagnosticSeverity,
    subject: DiagnosticSubject,
    facts: DiagnosticFacts,
    causes: Vec<DiagnosticCause>,
    actions: Vec<DiagnosticAction>,
    observed_at: UtcTimestamp,
    correlation_id: Option<String>,
    connection_id: Option<AgentConnectionId>,
    project_id: Option<ProjectId>,
    runtime_session_id: Option<AgentRuntimeSessionId>,
    integration_revision: Option<IntegrationRevision>,
}

impl<'de> Deserialize<'de> for DiagnosticFinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticFindingWire::deserialize(deserializer)?;
        let finding = Self::try_new(
            wire.id,
            wire.code,
            wire.domain,
            wire.stage,
            wire.severity,
            wire.subject,
            wire.facts,
            wire.observed_at,
        )
        .map_err(de::Error::custom)?;
        let finding = finding
            .with_causes(wire.causes)
            .map_err(de::Error::custom)?;
        let finding = finding
            .with_actions(wire.actions)
            .map_err(de::Error::custom)?;
        let finding = finding
            .with_optional_correlation_id(wire.correlation_id)
            .map_err(de::Error::custom)?;
        let finding = finding
            .with_optional_connection_id(wire.connection_id)
            .map_err(de::Error::custom)?;
        let finding = finding
            .with_optional_project_id(wire.project_id)
            .map_err(de::Error::custom)?;
        let finding = finding
            .with_optional_runtime_session_id(wire.runtime_session_id)
            .map_err(de::Error::custom)?;
        Ok(finding.with_optional_integration_revision(wire.integration_revision))
    }
}

impl DiagnosticFinding {
    /// Constructs one finding without cause edges, actions, or correlation.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: DiagnosticFindingId,
        code: DiagnosticCode,
        domain: DiagnosticDomain,
        stage: DiagnosticStage,
        severity: DiagnosticSeverity,
        subject: DiagnosticSubject,
        facts: DiagnosticFacts,
        observed_at: UtcTimestamp,
    ) -> Result<Self, DiagnosticError> {
        validate_timestamp("diagnostic finding observed_at", &observed_at)?;
        facts.validate_size()?;
        Ok(Self {
            id,
            code,
            domain,
            stage,
            severity,
            subject,
            facts,
            causes: Vec::new(),
            actions: Vec::new(),
            observed_at,
            correlation_id: None,
            connection_id: None,
            project_id: None,
            runtime_session_id: None,
            integration_revision: None,
        })
    }

    /// Constructs the bounded fallback finding for an unclassified unexpected
    /// failure. Callers must supply a safe message rather than a raw request,
    /// environment, tool argument set, or unrestricted process output.
    #[allow(clippy::too_many_arguments)]
    pub fn unexpected_failure(
        id: DiagnosticFindingId,
        owner: impl Into<String>,
        operation: impl Into<String>,
        stage: DiagnosticStage,
        correlation_id: impl Into<String>,
        safe_message: impl Into<String>,
        observed_at: UtcTimestamp,
    ) -> Result<Self, DiagnosticError> {
        let owner = owner.into();
        let operation = operation.into();
        validate_name("unexpected-failure owner", &owner)?;
        validate_name("unexpected-failure operation", &operation)?;
        let facts = DiagnosticFacts::project(&UnexpectedFailureFacts {
            owner: owner.clone(),
            operation: operation.clone(),
            message: safe_message.into(),
        })?;
        Self::try_new(
            id,
            DiagnosticCode::parse(INTERNAL_UNEXPECTED_FAILURE_CODE)?,
            DiagnosticDomain::parse(owner)?,
            stage,
            DiagnosticSeverity::Error,
            DiagnosticSubject::try_new("operation", operation)?,
            facts,
            observed_at,
        )?
        .with_correlation_id(correlation_id)
    }

    /// Adds and canonically orders cause edges.
    pub fn with_causes(
        mut self,
        mut causes: Vec<DiagnosticCause>,
    ) -> Result<Self, DiagnosticError> {
        if causes.len() > MAX_DIAGNOSTIC_CAUSES {
            return Err(invalid(format!(
                "diagnostic finding has more than {MAX_DIAGNOSTIC_CAUSES} causes"
            )));
        }
        causes.sort();
        let mut previous: Option<&DiagnosticFindingId> = None;
        for cause in &causes {
            if cause.finding_id == self.id {
                return Err(invalid(format!(
                    "diagnostic finding {} cannot cause itself",
                    self.id
                )));
            }
            if previous == Some(&cause.finding_id) {
                return Err(invalid(format!(
                    "diagnostic finding {} contains duplicate cause {}",
                    self.id, cause.finding_id
                )));
            }
            previous = Some(&cause.finding_id);
        }
        self.causes = causes;
        Ok(self)
    }

    /// Adds and canonically orders recommended actions.
    pub fn with_actions(
        mut self,
        mut actions: Vec<DiagnosticAction>,
    ) -> Result<Self, DiagnosticError> {
        if actions.len() > MAX_DIAGNOSTIC_ACTIONS {
            return Err(invalid(format!(
                "diagnostic finding has more than {MAX_DIAGNOSTIC_ACTIONS} actions"
            )));
        }
        actions.sort_by(|left, right| {
            left.code
                .cmp(&right.code)
                .then_with(|| left.summary.cmp(&right.summary))
        });
        if actions.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid("diagnostic finding contains a duplicate action"));
        }
        self.actions = actions;
        Ok(self)
    }

    /// Adds a bounded general correlation identifier.
    pub fn with_correlation_id(
        self,
        correlation_id: impl Into<String>,
    ) -> Result<Self, DiagnosticError> {
        self.with_optional_correlation_id(Some(correlation_id.into()))
    }

    /// Adds optional Agent Connection correlation.
    pub fn with_connection_id(
        self,
        connection_id: AgentConnectionId,
    ) -> Result<Self, DiagnosticError> {
        self.with_optional_connection_id(Some(connection_id))
    }

    /// Adds optional project correlation.
    pub fn with_project_id(self, project_id: ProjectId) -> Result<Self, DiagnosticError> {
        self.with_optional_project_id(Some(project_id))
    }

    /// Adds optional runtime-session correlation.
    pub fn with_runtime_session_id(
        self,
        runtime_session_id: AgentRuntimeSessionId,
    ) -> Result<Self, DiagnosticError> {
        self.with_optional_runtime_session_id(Some(runtime_session_id))
    }

    /// Adds optional typed integration-revision correlation.
    pub fn with_integration_revision(mut self, revision: IntegrationRevision) -> Self {
        self.integration_revision = Some(revision);
        self
    }

    fn with_optional_correlation_id(
        mut self,
        correlation_id: Option<String>,
    ) -> Result<Self, DiagnosticError> {
        if let Some(value) = correlation_id.as_deref() {
            validate_stable_identifier("diagnostic correlation id", value)?;
        }
        self.correlation_id = correlation_id;
        Ok(self)
    }

    fn with_optional_connection_id(
        mut self,
        connection_id: Option<AgentConnectionId>,
    ) -> Result<Self, DiagnosticError> {
        if let Some(value) = connection_id.as_ref() {
            validate_correlation_value("diagnostic connection id", value.as_str())?;
        }
        self.connection_id = connection_id;
        Ok(self)
    }

    fn with_optional_project_id(
        mut self,
        project_id: Option<ProjectId>,
    ) -> Result<Self, DiagnosticError> {
        if let Some(value) = project_id.as_ref() {
            validate_correlation_value("diagnostic project id", value.as_str())?;
        }
        self.project_id = project_id;
        Ok(self)
    }

    fn with_optional_runtime_session_id(
        mut self,
        runtime_session_id: Option<AgentRuntimeSessionId>,
    ) -> Result<Self, DiagnosticError> {
        if let Some(value) = runtime_session_id.as_ref() {
            validate_correlation_value("diagnostic runtime session id", value.as_str())?;
        }
        self.runtime_session_id = runtime_session_id;
        Ok(self)
    }

    fn with_optional_integration_revision(mut self, revision: Option<IntegrationRevision>) -> Self {
        self.integration_revision = revision;
        self
    }

    /// Returns the stable finding ID.
    pub fn id(&self) -> &DiagnosticFindingId {
        &self.id
    }

    /// Returns the stable namespaced diagnostic code.
    pub fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    /// Returns the owner domain.
    pub fn domain(&self) -> &DiagnosticDomain {
        &self.domain
    }

    /// Returns the observation stage.
    pub fn stage(&self) -> &DiagnosticStage {
        &self.stage
    }

    /// Returns the finding severity.
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    /// Returns the bounded subject.
    pub fn subject(&self) -> &DiagnosticSubject {
        &self.subject
    }

    /// Returns the bounded redacted facts.
    pub fn facts(&self) -> &DiagnosticFacts {
        &self.facts
    }

    /// Returns cause edges in canonical finding-ID order.
    pub fn causes(&self) -> &[DiagnosticCause] {
        &self.causes
    }

    /// Returns recommended actions in canonical code/summary order.
    pub fn actions(&self) -> &[DiagnosticAction] {
        &self.actions
    }

    /// Returns the observation timestamp.
    pub fn observed_at(&self) -> &UtcTimestamp {
        &self.observed_at
    }

    /// Returns the optional general correlation ID.
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }

    /// Returns the optional Agent Connection correlation.
    pub fn connection_id(&self) -> Option<&AgentConnectionId> {
        self.connection_id.as_ref()
    }

    /// Returns the optional project correlation.
    pub fn project_id(&self) -> Option<&ProjectId> {
        self.project_id.as_ref()
    }

    /// Returns the optional runtime-session correlation.
    pub fn runtime_session_id(&self) -> Option<&AgentRuntimeSessionId> {
        self.runtime_session_id.as_ref()
    }

    /// Returns the optional integration-revision correlation.
    pub fn integration_revision(&self) -> Option<&IntegrationRevision> {
        self.integration_revision.as_ref()
    }
}

#[derive(Serialize)]
struct UnexpectedFailureFacts {
    owner: String,
    operation: String,
    message: String,
}

impl DiagnosticFactSource for UnexpectedFailureFacts {}

/// Aggregate status of one diagnostic report.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticReportStatus {
    /// The intended observations completed without an error finding.
    Complete,
    /// At least one named observation could not be completed.
    Incomplete,
    /// The observed operation or report construction failed.
    Failed,
}

/// Current shared JSON diagnostic report.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct DiagnosticReport {
    schema_version: u32,
    status: DiagnosticReportStatus,
    generated_at: UtcTimestamp,
    findings: Vec<DiagnosticFinding>,
    root_cause_ids: Vec<DiagnosticFindingId>,
    limits: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticReportWire {
    schema_version: u32,
    status: DiagnosticReportStatus,
    generated_at: UtcTimestamp,
    findings: Vec<DiagnosticFinding>,
    root_cause_ids: Vec<DiagnosticFindingId>,
    limits: Vec<String>,
}

impl<'de> Deserialize<'de> for DiagnosticReport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticReportWire::deserialize(deserializer)?;
        if wire.schema_version != DIAGNOSTIC_REPORT_SCHEMA_VERSION {
            return Err(de::Error::custom(format!(
                "diagnostic report schema_version must be {DIAGNOSTIC_REPORT_SCHEMA_VERSION}"
            )));
        }
        Self::try_new(
            wire.status,
            wire.generated_at,
            wire.findings,
            wire.root_cause_ids,
            wire.limits,
        )
        .map_err(de::Error::custom)
    }
}

impl DiagnosticReport {
    /// Validates and canonically orders one current diagnostic report.
    pub fn try_new(
        status: DiagnosticReportStatus,
        generated_at: UtcTimestamp,
        mut findings: Vec<DiagnosticFinding>,
        mut root_cause_ids: Vec<DiagnosticFindingId>,
        mut limits: Vec<String>,
    ) -> Result<Self, DiagnosticError> {
        validate_timestamp("diagnostic report generated_at", &generated_at)?;
        if findings.len() > MAX_DIAGNOSTIC_FINDINGS {
            return Err(invalid(format!(
                "diagnostic report has more than {MAX_DIAGNOSTIC_FINDINGS} findings"
            )));
        }
        if root_cause_ids.len() > MAX_DIAGNOSTIC_ROOT_CAUSES {
            return Err(invalid(format!(
                "diagnostic report has more than {MAX_DIAGNOSTIC_ROOT_CAUSES} root causes"
            )));
        }
        if limits.len() > MAX_DIAGNOSTIC_LIMITS {
            return Err(invalid(format!(
                "diagnostic report has more than {MAX_DIAGNOSTIC_LIMITS} limits"
            )));
        }

        findings.sort_by(|left, right| left.id.cmp(&right.id));
        if findings.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(invalid("diagnostic report contains duplicate finding ids"));
        }
        root_cause_ids.sort();
        if root_cause_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid(
                "diagnostic report contains duplicate root-cause ids",
            ));
        }
        for limit in &limits {
            validate_bounded_text(
                "diagnostic report limit",
                limit,
                MAX_DIAGNOSTIC_FACT_STRING_BYTES,
            )?;
        }
        limits.sort();
        if limits.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid("diagnostic report contains duplicate limits"));
        }

        validate_cause_graph(&findings, &root_cause_ids)?;
        let report = Self {
            schema_version: DIAGNOSTIC_REPORT_SCHEMA_VERSION,
            status,
            generated_at,
            findings,
            root_cause_ids,
            limits,
        };
        let size = serde_json::to_vec(&report)
            .map_err(|_| invalid("diagnostic report could not be serialized"))?
            .len();
        if size > MAX_DIAGNOSTIC_REPORT_BYTES {
            return Err(invalid(format!(
                "diagnostic report exceeds {MAX_DIAGNOSTIC_REPORT_BYTES} serialized bytes"
            )));
        }
        Ok(report)
    }

    /// Returns the only current schema version.
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Returns the report status.
    pub const fn status(&self) -> DiagnosticReportStatus {
        self.status
    }

    /// Returns the report generation timestamp.
    pub fn generated_at(&self) -> &UtcTimestamp {
        &self.generated_at
    }

    /// Returns findings in canonical ID order.
    pub fn findings(&self) -> &[DiagnosticFinding] {
        &self.findings
    }

    /// Returns explicitly identified independent root causes in canonical ID order.
    pub fn root_cause_ids(&self) -> &[DiagnosticFindingId] {
        &self.root_cause_ids
    }

    /// Returns bounded report limitations in canonical order.
    pub fn limits(&self) -> &[String] {
        &self.limits
    }
}

fn validate_cause_graph(
    findings: &[DiagnosticFinding],
    root_cause_ids: &[DiagnosticFindingId],
) -> Result<(), DiagnosticError> {
    let indexes = findings
        .iter()
        .enumerate()
        .map(|(index, finding)| (finding.id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    for finding in findings {
        for cause in &finding.causes {
            if !indexes.contains_key(&cause.finding_id) {
                return Err(invalid(format!(
                    "diagnostic finding {} references unknown cause {}",
                    finding.id, cause.finding_id
                )));
            }
        }
    }
    for root_cause_id in root_cause_ids {
        let Some(index) = indexes.get(root_cause_id).copied() else {
            return Err(invalid(format!(
                "diagnostic report references unknown root cause {root_cause_id}"
            )));
        };
        if !findings[index].causes.is_empty() {
            return Err(invalid(format!(
                "root cause {root_cause_id} must not have its own cause edge"
            )));
        }
    }

    let mut marks = vec![0_u8; findings.len()];
    for index in 0..findings.len() {
        visit_cause(index, findings, &indexes, &mut marks)?;
    }
    Ok(())
}

fn visit_cause(
    index: usize,
    findings: &[DiagnosticFinding],
    indexes: &BTreeMap<DiagnosticFindingId, usize>,
    marks: &mut [u8],
) -> Result<(), DiagnosticError> {
    match marks[index] {
        1 => {
            return Err(invalid(format!(
                "diagnostic cause graph contains a cycle at {}",
                findings[index].id
            )))
        }
        2 => return Ok(()),
        _ => {}
    }
    marks[index] = 1;
    for cause in &findings[index].causes {
        let cause_index = indexes[&cause.finding_id];
        visit_cause(cause_index, findings, indexes, marks)?;
    }
    marks[index] = 2;
    Ok(())
}

fn validate_name(field: &str, value: &str) -> Result<(), DiagnosticError> {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return Err(invalid(format!("{field} must not be empty")));
    };
    if value.len() > MAX_DIAGNOSTIC_NAME_BYTES
        || !first.is_ascii_lowercase()
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        Err(invalid(format!(
            "{field} must be 1 through {MAX_DIAGNOSTIC_NAME_BYTES} ASCII bytes matching [a-z][a-z0-9_]*"
        )))
    } else {
        Ok(())
    }
}

fn validate_stable_identifier(field: &str, value: &str) -> Result<(), DiagnosticError> {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return Err(invalid(format!("{field} must not be empty")));
    };
    if value.len() > MAX_DIAGNOSTIC_IDENTIFIER_BYTES
        || !first.is_ascii_lowercase()
        || !bytes.all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b'.' | b':')
        })
        || !value
            .bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        Err(invalid(format!(
            "{field} must be a bounded lowercase stable identifier"
        )))
    } else {
        Ok(())
    }
}

fn validate_correlation_value(field: &str, value: &str) -> Result<(), DiagnosticError> {
    validate_bounded_text(field, value, MAX_DIAGNOSTIC_IDENTIFIER_BYTES)
}

fn validate_bounded_text(
    field: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), DiagnosticError> {
    if value.is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        Err(invalid(format!(
            "{field} must be 1 through {max_bytes} UTF-8 bytes and contain no control characters"
        )))
    } else {
        Ok(())
    }
}

fn validate_timestamp(field: &str, value: &UtcTimestamp) -> Result<(), DiagnosticError> {
    value
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| invalid(format!("{field} is outside the canonical timestamp range")))
}

/// Validation or projection failure for the shared diagnostic model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticError {
    detail: String,
}

impl DiagnosticError {
    /// Returns bounded implementation-facing failure detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for DiagnosticError {}

fn invalid(detail: impl Into<String>) -> DiagnosticError {
    DiagnosticError {
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::Serialize;
    use serde_json::{json, Value};

    use super::*;

    fn timestamp() -> UtcTimestamp {
        UtcTimestamp::parse("2026-07-21T01:02:03Z").unwrap()
    }

    #[derive(Serialize)]
    struct SafeFacts {
        actual: String,
        expected: String,
        protocol_revision: String,
    }

    impl DiagnosticFactSource for SafeFacts {}

    fn finding(id: &str) -> DiagnosticFinding {
        DiagnosticFinding::try_new(
            DiagnosticFindingId::parse(id).unwrap(),
            DiagnosticCode::parse("test.check_failed").unwrap(),
            DiagnosticDomain::parse("test").unwrap(),
            DiagnosticStage::parse("verification").unwrap(),
            DiagnosticSeverity::Error,
            DiagnosticSubject::try_new("check", id).unwrap(),
            DiagnosticFacts::project(&SafeFacts {
                actual: "failed".to_owned(),
                expected: "passed".to_owned(),
                protocol_revision: "2025-11-25".to_owned(),
            })
            .unwrap(),
            timestamp(),
        )
        .unwrap()
    }

    #[test]
    fn stable_namespaced_codes_are_strictly_validated() {
        for valid in [
            "storage.sqlite.open_failed",
            "mcp.protocol.invalid_revision",
            INTERNAL_UNEXPECTED_FAILURE_CODE,
        ] {
            assert_eq!(DiagnosticCode::parse(valid).unwrap().as_str(), valid);
        }
        for invalid in [
            "open_failed",
            "Storage.sqlite.open_failed",
            "storage..open_failed",
            "storage.sqlite.open-failed",
            ".storage",
            "storage.",
        ] {
            assert!(DiagnosticCode::parse(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn serialization_is_deterministic_across_input_order() {
        let first = DiagnosticReport::try_new(
            DiagnosticReportStatus::Failed,
            timestamp(),
            vec![finding("finding.z"), finding("finding.a")],
            vec![
                DiagnosticFindingId::parse("finding.z").unwrap(),
                DiagnosticFindingId::parse("finding.a").unwrap(),
            ],
            vec!["second limit".to_owned(), "first limit".to_owned()],
        )
        .unwrap();
        let second = DiagnosticReport::try_new(
            DiagnosticReportStatus::Failed,
            timestamp(),
            vec![finding("finding.a"), finding("finding.z")],
            vec![
                DiagnosticFindingId::parse("finding.a").unwrap(),
                DiagnosticFindingId::parse("finding.z").unwrap(),
            ],
            vec!["first limit".to_owned(), "second limit".to_owned()],
        )
        .unwrap();
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
    }

    #[derive(Serialize)]
    struct OversizedFacts {
        message: String,
        values: Vec<String>,
    }

    impl DiagnosticFactSource for OversizedFacts {}

    #[test]
    fn strings_and_collections_are_bounded_deterministically() {
        let facts = DiagnosticFacts::project(&OversizedFacts {
            message: "가".repeat(MAX_DIAGNOSTIC_FACT_STRING_BYTES),
            values: (0..MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS + 5)
                .map(|index| format!("value-{index}"))
                .collect(),
        })
        .unwrap();
        assert!(facts.truncated());
        assert_eq!(
            facts.data()["values"].as_array().unwrap().len(),
            MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS
        );
        let message = facts.data()["message"].as_str().unwrap();
        assert!(message.len() <= MAX_DIAGNOSTIC_FACT_STRING_BYTES);
        assert!(message.ends_with(TRUNCATED_SUFFIX));
    }

    #[derive(Serialize)]
    struct LargeFacts {
        values: BTreeMap<String, String>,
    }

    impl DiagnosticFactSource for LargeFacts {}

    #[test]
    fn total_serialized_fact_size_is_enforced() {
        let values = (0..MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS)
            .map(|index| {
                (
                    format!("field_{index:02}"),
                    "x".repeat(MAX_DIAGNOSTIC_FACT_STRING_BYTES),
                )
            })
            .collect();
        let error = DiagnosticFacts::project(&LargeFacts { values }).unwrap_err();
        assert!(error.detail().contains("serialized bytes"));
    }

    #[derive(Serialize)]
    struct SensitiveFacts {
        expected: String,
        actual: String,
        api_token: String,
        credentials: HashMap<String, String>,
        environment: HashMap<String, String>,
        request_body: String,
        tool_arguments: Vec<String>,
        stderr: String,
        bounded_path: String,
    }

    impl DiagnosticFactSource for SensitiveFacts {}

    #[test]
    fn sensitive_fact_fields_are_redacted_while_safe_facts_remain() {
        let mut credentials = HashMap::new();
        credentials.insert("user".to_owned(), "private-user".to_owned());
        let mut environment = HashMap::new();
        environment.insert("HOME".to_owned(), "/private/home".to_owned());
        let facts = DiagnosticFacts::project(&SensitiveFacts {
            expected: "expected".to_owned(),
            actual: "actual".to_owned(),
            api_token: "token-secret".to_owned(),
            credentials,
            environment,
            request_body: "request-secret".to_owned(),
            tool_arguments: vec!["argument-secret".to_owned()],
            stderr: "stderr-secret".to_owned(),
            bounded_path: "src/lib.rs".to_owned(),
        })
        .unwrap();
        assert_eq!(facts.data()["expected"], "expected");
        assert_eq!(facts.data()["actual"], "actual");
        assert_eq!(facts.data()["bounded_path"], "src/lib.rs");
        for field in [
            "api_token",
            "credentials",
            "environment",
            "request_body",
            "tool_arguments",
            "stderr",
        ] {
            assert_eq!(facts.data()[field], REDACTED_VALUE);
            assert!(facts.redacted_fields().iter().any(|value| value == field));
        }
    }

    #[derive(Serialize)]
    struct NestedFacts {
        value: Value,
    }

    impl DiagnosticFactSource for NestedFacts {}

    #[test]
    fn nested_fact_depth_is_bounded() {
        let facts = DiagnosticFacts::project(&NestedFacts {
            value: json!({"a": {"b": {"c": {"d": {"e": true}}}}}),
        })
        .unwrap();
        assert!(facts.truncated());
        assert!(serde_json::to_string(&facts)
            .unwrap()
            .contains(DEPTH_LIMIT_VALUE));
    }

    #[test]
    fn cause_ids_must_exist_and_form_an_acyclic_graph() {
        let missing = finding("finding.child")
            .with_causes(vec![DiagnosticCause::new(
                DiagnosticFindingId::parse("finding.missing").unwrap(),
            )])
            .unwrap();
        assert!(DiagnosticReport::try_new(
            DiagnosticReportStatus::Failed,
            timestamp(),
            vec![missing],
            Vec::new(),
            Vec::new(),
        )
        .unwrap_err()
        .detail()
        .contains("unknown cause"));

        let self_id = DiagnosticFindingId::parse("finding.self").unwrap();
        assert!(finding("finding.self")
            .with_causes(vec![DiagnosticCause::new(self_id)])
            .is_err());

        let left = finding("finding.left")
            .with_causes(vec![DiagnosticCause::new(
                DiagnosticFindingId::parse("finding.right").unwrap(),
            )])
            .unwrap();
        let right = finding("finding.right")
            .with_causes(vec![DiagnosticCause::new(
                DiagnosticFindingId::parse("finding.left").unwrap(),
            )])
            .unwrap();
        assert!(DiagnosticReport::try_new(
            DiagnosticReportStatus::Failed,
            timestamp(),
            vec![left, right],
            Vec::new(),
            Vec::new(),
        )
        .unwrap_err()
        .detail()
        .contains("cycle"));
    }

    #[test]
    fn reports_represent_multiple_independent_root_causes() {
        let first_id = DiagnosticFindingId::parse("finding.first_root").unwrap();
        let second_id = DiagnosticFindingId::parse("finding.second_root").unwrap();
        let report = DiagnosticReport::try_new(
            DiagnosticReportStatus::Incomplete,
            timestamp(),
            vec![
                finding(first_id.as_str()),
                finding(second_id.as_str()),
                finding("finding.check"),
            ],
            vec![second_id.clone(), first_id.clone()],
            vec!["one auxiliary observation was unavailable".to_owned()],
        )
        .unwrap();
        assert_eq!(report.root_cause_ids(), &[first_id, second_id]);
        assert_eq!(report.status(), DiagnosticReportStatus::Incomplete);
    }

    #[test]
    fn unexpected_failure_projection_retains_only_bounded_safe_context() {
        let finding = DiagnosticFinding::unexpected_failure(
            DiagnosticFindingId::parse("finding.unexpected").unwrap(),
            "store",
            "open_database",
            DiagnosticStage::parse("startup").unwrap(),
            "correlation:123",
            "safe open failure",
            timestamp(),
        )
        .unwrap();
        assert_eq!(finding.code().as_str(), INTERNAL_UNEXPECTED_FAILURE_CODE);
        assert_eq!(finding.domain().as_str(), "store");
        assert_eq!(finding.stage().as_str(), "startup");
        assert_eq!(finding.correlation_id(), Some("correlation:123"));
        assert_eq!(finding.facts().data()["owner"], "store");
        assert_eq!(finding.facts().data()["operation"], "open_database");
        assert_eq!(finding.facts().data()["message"], "safe open failure");
    }

    #[test]
    fn serialized_findings_do_not_contain_secret_values() {
        let facts = DiagnosticFacts::project(&SensitiveFacts {
            expected: "present".to_owned(),
            actual: "missing".to_owned(),
            api_token: "needle-api-token".to_owned(),
            credentials: HashMap::from([("password".to_owned(), "needle-credential".to_owned())]),
            environment: HashMap::from([(
                "PRIVATE_VALUE".to_owned(),
                "needle-environment".to_owned(),
            )]),
            request_body: "needle-request".to_owned(),
            tool_arguments: vec!["needle-argument".to_owned()],
            stderr: "needle-stderr".to_owned(),
            bounded_path: "src/main.rs".to_owned(),
        })
        .unwrap();
        let finding = DiagnosticFinding::try_new(
            DiagnosticFindingId::parse("finding.redacted").unwrap(),
            DiagnosticCode::parse("security.redaction_applied").unwrap(),
            DiagnosticDomain::parse("security").unwrap(),
            DiagnosticStage::parse("projection").unwrap(),
            DiagnosticSeverity::Warning,
            DiagnosticSubject::try_new("report", "diagnostic-report").unwrap(),
            facts,
            timestamp(),
        )
        .unwrap();
        let serialized = serde_json::to_string(&finding).unwrap();
        for secret in [
            "needle-api-token",
            "needle-credential",
            "needle-environment",
            "needle-request",
            "needle-argument",
            "needle-stderr",
        ] {
            assert!(!serialized.contains(secret), "leaked {secret}");
        }
    }

    #[test]
    fn deserialization_accepts_only_the_current_report_schema() {
        let report = DiagnosticReport::try_new(
            DiagnosticReportStatus::Complete,
            timestamp(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(
            serde_json::from_value::<DiagnosticReport>(value.clone()).unwrap(),
            report
        );
        let mut unsupported = value;
        unsupported["schema_version"] = json!(2);
        assert!(serde_json::from_value::<DiagnosticReport>(unsupported).is_err());
    }
}
