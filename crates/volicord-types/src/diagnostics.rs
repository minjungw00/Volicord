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
use sha2::{Digest, Sha256};

use crate::{
    AgentConnectionId, AgentRuntimeSessionId, ConnectionCheck, ConnectionCheckStatus,
    ConnectionStatus, DurableIdGenerator, DurableIdKind, IntegrationRevision, JsonObject,
    ProjectId, RandomDurableIdGenerator, UtcTimestamp,
};

/// The only current JSON representation version for [`DiagnosticReport`].
pub const DIAGNOSTIC_REPORT_SCHEMA_VERSION: u32 = 2;
/// The only current JSON representation version for [`DiagnosticLookupReport`].
pub const DIAGNOSTIC_LOOKUP_REPORT_SCHEMA_VERSION: u32 = 1;
/// Stable prefix for one pre-Registry stderr diagnostic line.
pub const BOOTSTRAP_DIAGNOSTIC_ENVELOPE_PREFIX: &str = "VOLICORD_DIAGNOSTIC_V1";
/// Maximum UTF-8 byte length of one complete pre-Registry stderr envelope.
pub const MAX_BOOTSTRAP_DIAGNOSTIC_ENVELOPE_BYTES: usize = 64 * 1024;
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
/// Maximum cause-edge depth followed while selecting diagnostic root causes.
pub const MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH: usize = 32;
/// Maximum number of report limitations.
pub const MAX_DIAGNOSTIC_LIMITS: usize = 32;
/// Maximum serialized byte length of one complete report.
pub const MAX_DIAGNOSTIC_REPORT_BYTES: usize = 1024 * 1024;
/// Maximum UTF-8 byte length of one current diagnostic scope identity.
pub const MAX_DIAGNOSTIC_SCOPE_IDENTITY_BYTES: usize = 1_024;

const CURRENT_DIAGNOSTIC_ID_PREFIX: &str = "finding.current.sha256:";
const CURRENT_DIAGNOSTIC_KEY_DOMAIN: &[u8] = b"volicord.diagnostic.current-key";
const CURRENT_DIAGNOSTIC_KEY_VERSION: u16 = 2;
const DIAGNOSTIC_SUBJECT_IDENTITY_PREFIX: &str = "sha256:";
const DIAGNOSTIC_SUBJECT_IDENTITY_HEX_BYTES: usize = 64;

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

/// Bounded producer identity for a diagnostic finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct DiagnosticSource(String);

impl DiagnosticSource {
    /// Validates one stable lowercase producer name.
    pub fn parse(value: impl Into<String>) -> Result<Self, DiagnosticError> {
        let value = value.into();
        validate_name("diagnostic source", &value)?;
        Ok(Self(value))
    }

    /// Returns the stable producer spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for DiagnosticSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl fmt::Display for DiagnosticSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DiagnosticSource {
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

impl DiagnosticSeverity {
    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
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

/// Opaque stable semantic identity for one diagnostic subject.
///
/// The token contains only a domain-separated SHA-256 digest of canonical
/// bytes owned by the typed subject family. It is distinct from the bounded
/// safe [`DiagnosticSubject`] presentation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct DiagnosticSubjectIdentity(String);

impl DiagnosticSubjectIdentity {
    /// Derives one opaque identity token from owner-provided canonical bytes.
    pub fn from_canonical_bytes(canonical_identity_bytes: &[u8]) -> Self {
        Self(format!(
            "{DIAGNOSTIC_SUBJECT_IDENTITY_PREFIX}{}",
            lowercase_digest(&Sha256::digest(canonical_identity_bytes))
        ))
    }

    /// Validates one persisted opaque subject identity token.
    pub fn parse_persisted(value: impl Into<String>) -> Result<Self, DiagnosticError> {
        let value = value.into();
        let Some(digest) = value.strip_prefix(DIAGNOSTIC_SUBJECT_IDENTITY_PREFIX) else {
            return Err(invalid(
                "diagnostic subject identity must use the sha256 algorithm",
            ));
        };
        if digest.len() != DIAGNOSTIC_SUBJECT_IDENTITY_HEX_BYTES
            || !digest
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(invalid(
                "diagnostic subject identity must contain exactly 64 lowercase hexadecimal SHA-256 characters",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the validated opaque token spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the canonical token bytes used by current-key encoding.
    pub fn canonical_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl<'de> Deserialize<'de> for DiagnosticSubjectIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse_persisted(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl fmt::Display for DiagnosticSubjectIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DiagnosticSubjectIdentity {
    type Err = DiagnosticError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse_persisted(value)
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
        || normalized.ends_with("secret")
        || normalized.ends_with("_token")
        || normalized.ends_with("token")
        || normalized.ends_with("_password")
        || normalized.ends_with("password")
        || normalized.ends_with("_credential")
        || normalized.ends_with("credential")
        || normalized.ends_with("_credentials")
        || normalized.ends_with("credentials")
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

/// Persisted lifecycle of one structured diagnostic finding.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticFindingLifecycle {
    /// One immutable event-like observation.
    Occurrence,
    /// One replaceable snapshot for a stable current diagnostic key.
    CurrentState,
}

impl DiagnosticFindingLifecycle {
    /// Returns the exact persisted lifecycle spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Occurrence => "occurrence",
            Self::CurrentState => "current_state",
        }
    }
}

/// Closed scope kinds accepted by a current diagnostic key.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticScopeKind {
    /// One Agent Connection.
    Connection,
    /// One registered project.
    Project,
    /// One Volicord Runtime Home.
    RuntimeHome,
    /// One Volicord installation.
    Installation,
    /// One executable process.
    Process,
}

impl DiagnosticScopeKind {
    /// Returns the exact canonical and persisted scope spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Connection => "connection",
            Self::Project => "project",
            Self::RuntimeHome => "runtime_home",
            Self::Installation => "installation",
            Self::Process => "process",
        }
    }
}

/// Typed scope coordinate for one current diagnostic condition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
pub struct DiagnosticScope {
    kind: DiagnosticScopeKind,
    identity: String,
}

impl DiagnosticScope {
    /// Validates one complete diagnostic scope coordinate.
    pub fn try_new(
        kind: DiagnosticScopeKind,
        identity: impl Into<String>,
    ) -> Result<Self, DiagnosticError> {
        let identity = identity.into();
        validate_bounded_text(
            "diagnostic scope identity",
            &identity,
            MAX_DIAGNOSTIC_SCOPE_IDENTITY_BYTES,
        )?;
        Ok(Self { kind, identity })
    }

    /// Returns the scope kind.
    pub const fn kind(&self) -> DiagnosticScopeKind {
        self.kind
    }

    /// Returns the complete opaque scope identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticScopeWire {
    kind: DiagnosticScopeKind,
    identity: String,
}

impl<'de> Deserialize<'de> for DiagnosticScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticScopeWire::deserialize(deserializer)?;
        Self::try_new(wire.kind, wire.identity).map_err(de::Error::custom)
    }
}

/// Immutable definition, subject, observation, and correlation data for an occurrence.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticFindingData {
    code: DiagnosticCode,
    domain: DiagnosticDomain,
    stage: DiagnosticStage,
    severity: DiagnosticSeverity,
    source: DiagnosticSource,
    subject: DiagnosticSubject,
    facts: DiagnosticFacts,
    causes: Vec<DiagnosticCause>,
    actions: Vec<DiagnosticAction>,
    observed_at: UtcTimestamp,
    correlation_id: Option<String>,
    connection_id: Option<AgentConnectionId>,
    project_id: Option<ProjectId>,
    integration_revision: Option<IntegrationRevision>,
}

impl DiagnosticFindingData {
    /// Constructs validated diagnostic data without actions, causes, or correlation coordinates.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        code: DiagnosticCode,
        domain: DiagnosticDomain,
        stage: DiagnosticStage,
        severity: DiagnosticSeverity,
        source: DiagnosticSource,
        subject: DiagnosticSubject,
        facts: DiagnosticFacts,
        observed_at: UtcTimestamp,
    ) -> Result<Self, DiagnosticError> {
        validate_timestamp("diagnostic finding observed_at", &observed_at)?;
        facts.validate_size()?;
        Ok(Self {
            code,
            domain,
            stage,
            severity,
            source,
            subject,
            facts,
            causes: Vec::new(),
            actions: Vec::new(),
            observed_at,
            correlation_id: None,
            connection_id: None,
            project_id: None,
            integration_revision: None,
        })
    }

    /// Adds and canonically orders outgoing cause edges.
    pub fn with_causes(mut self, causes: Vec<DiagnosticCause>) -> Result<Self, DiagnosticError> {
        self.causes = canonical_diagnostic_causes(causes)?;
        Ok(self)
    }

    /// Adds and canonically orders remediation actions.
    pub fn with_actions(mut self, actions: Vec<DiagnosticAction>) -> Result<Self, DiagnosticError> {
        self.actions = canonical_diagnostic_actions(actions)?;
        Ok(self)
    }

    /// Adds a bounded general correlation identifier.
    pub fn with_correlation_id(
        mut self,
        correlation_id: impl Into<String>,
    ) -> Result<Self, DiagnosticError> {
        let correlation_id = correlation_id.into();
        validate_stable_identifier("diagnostic correlation id", &correlation_id)?;
        self.correlation_id = Some(correlation_id);
        Ok(self)
    }

    /// Adds Agent Connection correlation.
    pub fn with_connection_id(
        mut self,
        connection_id: AgentConnectionId,
    ) -> Result<Self, DiagnosticError> {
        validate_correlation_value("diagnostic connection id", connection_id.as_str())?;
        self.connection_id = Some(connection_id);
        Ok(self)
    }

    /// Adds project correlation.
    pub fn with_project_id(mut self, project_id: ProjectId) -> Result<Self, DiagnosticError> {
        validate_correlation_value("diagnostic project id", project_id.as_str())?;
        self.project_id = Some(project_id);
        Ok(self)
    }

    /// Adds typed integration-revision correlation.
    pub fn with_integration_revision(mut self, revision: IntegrationRevision) -> Self {
        self.integration_revision = Some(revision);
        self
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

    /// Returns the severity.
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    /// Returns the producer source.
    pub fn source(&self) -> &DiagnosticSource {
        &self.source
    }

    /// Returns the typed subject.
    pub fn subject(&self) -> &DiagnosticSubject {
        &self.subject
    }

    /// Returns bounded safe facts.
    pub fn facts(&self) -> &DiagnosticFacts {
        &self.facts
    }

    /// Returns outgoing causes in canonical order.
    pub fn causes(&self) -> &[DiagnosticCause] {
        &self.causes
    }

    /// Returns remediation actions in canonical order.
    pub fn actions(&self) -> &[DiagnosticAction] {
        &self.actions
    }

    /// Returns the observation time.
    pub fn observed_at(&self) -> &UtcTimestamp {
        &self.observed_at
    }

    /// Returns the optional general correlation ID.
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }

    /// Returns optional Agent Connection correlation.
    pub fn connection_id(&self) -> Option<&AgentConnectionId> {
        self.connection_id.as_ref()
    }

    /// Returns optional project correlation.
    pub fn project_id(&self) -> Option<&ProjectId> {
        self.project_id.as_ref()
    }

    /// Returns optional integration-revision correlation.
    pub fn integration_revision(&self) -> Option<&IntegrationRevision> {
        self.integration_revision.as_ref()
    }

    /// Projects this data into the shared read-only report shape.
    ///
    /// Persistence callers must use a lifecycle-specific finding type instead.
    pub fn to_read_projection(
        &self,
        id: DiagnosticFindingId,
        runtime_session_id: Option<AgentRuntimeSessionId>,
    ) -> DiagnosticFinding {
        self.to_projection(id, runtime_session_id)
    }

    fn to_projection(
        &self,
        id: DiagnosticFindingId,
        runtime_session_id: Option<AgentRuntimeSessionId>,
    ) -> DiagnosticFinding {
        DiagnosticFinding {
            id,
            code: self.code.clone(),
            domain: self.domain.clone(),
            stage: self.stage.clone(),
            severity: self.severity,
            source: self.source.clone(),
            subject: self.subject.clone(),
            facts: self.facts.clone(),
            causes: self.causes.clone(),
            actions: self.actions.clone(),
            observed_at: self.observed_at.clone(),
            correlation_id: self.correlation_id.clone(),
            connection_id: self.connection_id.clone(),
            project_id: self.project_id.clone(),
            runtime_session_id,
            integration_revision: self.integration_revision.clone(),
        }
    }
}

/// Strict generated ID for one immutable diagnostic occurrence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct DiagnosticOccurrenceId(String);

impl DiagnosticOccurrenceId {
    /// Parses one canonical generated diagnostic occurrence ID.
    pub fn parse(value: impl Into<String>) -> Result<Self, DiagnosticError> {
        let value = value.into();
        let prefix = DurableIdKind::DiagnosticOccurrence.prefix();
        let Some(suffix) = value.strip_prefix(prefix) else {
            return Err(invalid(
                "diagnostic occurrence id must use the generated occurrence prefix",
            ));
        };
        validate_uuid_version_four_suffix("diagnostic occurrence id", suffix)?;
        Ok(Self(value))
    }

    /// Generates one new opaque durable occurrence ID.
    pub fn generate(generator: &dyn DurableIdGenerator) -> Result<Self, DiagnosticError> {
        let value = generator
            .generate(DurableIdKind::DiagnosticOccurrence)
            .map_err(|error| {
                invalid(format!(
                    "could not generate diagnostic occurrence id: {error}"
                ))
            })?;
        Self::parse(value)
    }

    /// Returns the canonical generated spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn finding_id(&self) -> DiagnosticFindingId {
        DiagnosticFindingId::parse(self.0.clone())
            .expect("generated diagnostic occurrence IDs are stable finding IDs")
    }
}

impl<'de> Deserialize<'de> for DiagnosticOccurrenceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl fmt::Display for DiagnosticOccurrenceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One immutable diagnostic event observation.
#[derive(Debug, Clone, PartialEq)]
pub struct OccurrenceDiagnosticFinding {
    id: DiagnosticOccurrenceId,
    data: DiagnosticFindingData,
    runtime_session_id: Option<AgentRuntimeSessionId>,
}

impl OccurrenceDiagnosticFinding {
    /// Creates one occurrence with a newly generated opaque durable ID.
    pub fn try_new(
        data: DiagnosticFindingData,
        runtime_session_id: Option<AgentRuntimeSessionId>,
    ) -> Result<Self, DiagnosticError> {
        Self::try_new_with_generator(data, runtime_session_id, &RandomDurableIdGenerator)
    }

    /// Creates one occurrence with the supplied durable-ID generator.
    pub fn try_new_with_generator(
        data: DiagnosticFindingData,
        runtime_session_id: Option<AgentRuntimeSessionId>,
        generator: &dyn DurableIdGenerator,
    ) -> Result<Self, DiagnosticError> {
        let id = DiagnosticOccurrenceId::generate(generator)?;
        Self::from_persisted_parts(id, data, runtime_session_id)
    }

    /// Reconstructs one occurrence from its validated persisted parts.
    #[doc(hidden)]
    pub fn from_persisted_parts(
        id: DiagnosticOccurrenceId,
        data: DiagnosticFindingData,
        runtime_session_id: Option<AgentRuntimeSessionId>,
    ) -> Result<Self, DiagnosticError> {
        if runtime_session_id.is_some()
            && (data.connection_id().is_none() || data.integration_revision().is_none())
        {
            return Err(invalid(
                "runtime-correlated diagnostic occurrence requires Connection and integration revision coordinates",
            ));
        }
        let finding_id = id.finding_id();
        if data
            .causes()
            .iter()
            .any(|cause| cause.finding_id() == &finding_id)
        {
            return Err(invalid(format!(
                "diagnostic finding {finding_id} cannot cause itself"
            )));
        }
        Ok(Self {
            id,
            data,
            runtime_session_id,
        })
    }

    /// Returns the opaque occurrence ID.
    pub fn occurrence_id(&self) -> &DiagnosticOccurrenceId {
        &self.id
    }

    /// Returns the ID in the shared finding-ID type.
    pub fn id(&self) -> DiagnosticFindingId {
        self.id.finding_id()
    }

    /// Returns immutable occurrence data.
    pub fn data(&self) -> &DiagnosticFindingData {
        &self.data
    }

    /// Returns optional runtime-session correlation.
    pub fn runtime_session_id(&self) -> Option<&AgentRuntimeSessionId> {
        self.runtime_session_id.as_ref()
    }

    /// Projects this occurrence into the shared read-only report shape.
    pub fn to_diagnostic_finding(&self) -> DiagnosticFinding {
        self.data
            .to_projection(self.id(), self.runtime_session_id.clone())
    }
}

/// Complete immutable identity for one replaceable current diagnostic condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentDiagnosticKey {
    scope: DiagnosticScope,
    code: DiagnosticCode,
    domain: DiagnosticDomain,
    stage: DiagnosticStage,
    source: DiagnosticSource,
    subject_identity: DiagnosticSubjectIdentity,
}

impl CurrentDiagnosticKey {
    /// Constructs one complete current diagnostic identity.
    pub fn new(
        scope: DiagnosticScope,
        code: DiagnosticCode,
        domain: DiagnosticDomain,
        stage: DiagnosticStage,
        source: DiagnosticSource,
        subject_identity: DiagnosticSubjectIdentity,
    ) -> Self {
        Self {
            scope,
            code,
            domain,
            stage,
            source,
            subject_identity,
        }
    }

    /// Returns the diagnostic scope.
    pub fn scope(&self) -> &DiagnosticScope {
        &self.scope
    }

    /// Returns the complete diagnostic code identity.
    pub fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    /// Returns the owner domain identity.
    pub fn domain(&self) -> &DiagnosticDomain {
        &self.domain
    }

    /// Returns the stage identity.
    pub fn stage(&self) -> &DiagnosticStage {
        &self.stage
    }

    /// Returns the producer identity.
    pub fn source(&self) -> &DiagnosticSource {
        &self.source
    }

    /// Returns the opaque semantic subject identity.
    pub fn subject_identity(&self) -> &DiagnosticSubjectIdentity {
        &self.subject_identity
    }

    /// Returns the versioned, domain-separated, length-prefixed canonical identity bytes.
    pub fn canonical_identity_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_length_prefixed(&mut bytes, CURRENT_DIAGNOSTIC_KEY_DOMAIN);
        bytes.extend_from_slice(&CURRENT_DIAGNOSTIC_KEY_VERSION.to_be_bytes());
        push_length_prefixed(&mut bytes, self.scope.kind().as_str().as_bytes());
        push_length_prefixed(&mut bytes, self.scope.identity().as_bytes());
        push_length_prefixed(&mut bytes, self.code.as_str().as_bytes());
        push_length_prefixed(&mut bytes, self.domain.as_str().as_bytes());
        push_length_prefixed(&mut bytes, self.stage.as_str().as_bytes());
        push_length_prefixed(&mut bytes, self.source.as_str().as_bytes());
        push_length_prefixed(&mut bytes, self.subject_identity.canonical_bytes());
        bytes
    }

    /// Returns the full lowercase SHA-256 digest of the canonical identity bytes.
    pub fn identity_digest(&self) -> String {
        lowercase_digest(&Sha256::digest(self.canonical_identity_bytes()))
    }

    /// Returns the only valid path-opaque finding ID for this key.
    pub fn finding_id(&self) -> DiagnosticFindingId {
        DiagnosticFindingId::parse(format!(
            "{CURRENT_DIAGNOSTIC_ID_PREFIX}{}",
            self.identity_digest()
        ))
        .expect("current diagnostic digest IDs satisfy the stable identifier contract")
    }
}

/// Active or resolved state of one persisted current diagnostic snapshot.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CurrentDiagnosticStatus {
    /// The condition is currently observed.
    Active,
    /// The condition was explicitly resolved.
    Resolved,
}

impl CurrentDiagnosticStatus {
    /// Returns the exact persisted status spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Resolved => "resolved",
        }
    }
}

/// Replaceable observation fields for one current diagnostic key.
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentDiagnosticSnapshot {
    subject: DiagnosticSubject,
    severity: DiagnosticSeverity,
    facts: DiagnosticFacts,
    causes: Vec<DiagnosticCause>,
    actions: Vec<DiagnosticAction>,
    correlation_id: Option<String>,
    connection_id: Option<AgentConnectionId>,
    project_id: Option<ProjectId>,
    integration_revision: Option<IntegrationRevision>,
    observed_at: UtcTimestamp,
    status: CurrentDiagnosticStatus,
    resolved_at: Option<UtcTimestamp>,
}

impl CurrentDiagnosticSnapshot {
    /// Creates one active current snapshot without actions, causes, or correlation coordinates.
    pub fn try_new(
        subject: DiagnosticSubject,
        severity: DiagnosticSeverity,
        facts: DiagnosticFacts,
        observed_at: UtcTimestamp,
    ) -> Result<Self, DiagnosticError> {
        validate_timestamp("current diagnostic observed_at", &observed_at)?;
        facts.validate_size()?;
        Ok(Self {
            subject,
            severity,
            facts,
            causes: Vec::new(),
            actions: Vec::new(),
            correlation_id: None,
            connection_id: None,
            project_id: None,
            integration_revision: None,
            observed_at,
            status: CurrentDiagnosticStatus::Active,
            resolved_at: None,
        })
    }

    /// Returns the bounded safe subject presentation.
    pub fn subject(&self) -> &DiagnosticSubject {
        &self.subject
    }

    /// Adds and canonically orders outgoing causes.
    pub fn with_causes(mut self, causes: Vec<DiagnosticCause>) -> Result<Self, DiagnosticError> {
        self.causes = canonical_diagnostic_causes(causes)?;
        Ok(self)
    }

    /// Adds and canonically orders current remediation actions.
    pub fn with_actions(mut self, actions: Vec<DiagnosticAction>) -> Result<Self, DiagnosticError> {
        self.actions = canonical_diagnostic_actions(actions)?;
        Ok(self)
    }

    /// Adds a bounded general correlation ID.
    pub fn with_correlation_id(
        mut self,
        correlation_id: impl Into<String>,
    ) -> Result<Self, DiagnosticError> {
        let correlation_id = correlation_id.into();
        validate_stable_identifier("diagnostic correlation id", &correlation_id)?;
        self.correlation_id = Some(correlation_id);
        Ok(self)
    }

    /// Adds Agent Connection correlation.
    pub fn with_connection_id(
        mut self,
        connection_id: AgentConnectionId,
    ) -> Result<Self, DiagnosticError> {
        validate_correlation_value("diagnostic connection id", connection_id.as_str())?;
        self.connection_id = Some(connection_id);
        Ok(self)
    }

    /// Adds project correlation.
    pub fn with_project_id(mut self, project_id: ProjectId) -> Result<Self, DiagnosticError> {
        validate_correlation_value("diagnostic project id", project_id.as_str())?;
        self.project_id = Some(project_id);
        Ok(self)
    }

    /// Adds typed integration-revision correlation.
    pub fn with_integration_revision(mut self, revision: IntegrationRevision) -> Self {
        self.integration_revision = Some(revision);
        self
    }

    /// Marks a reconstructed snapshot resolved at the supplied canonical time.
    #[doc(hidden)]
    pub fn with_persisted_state(
        mut self,
        status: CurrentDiagnosticStatus,
        resolved_at: Option<UtcTimestamp>,
    ) -> Result<Self, DiagnosticError> {
        match (status, resolved_at.as_ref()) {
            (CurrentDiagnosticStatus::Active, None) => {}
            (CurrentDiagnosticStatus::Resolved, Some(value)) => {
                validate_timestamp("current diagnostic resolved_at", value)?;
                if !self.actions.is_empty() || !self.causes.is_empty() {
                    return Err(invalid(
                        "resolved current diagnostics cannot retain actions or outgoing causes",
                    ));
                }
            }
            _ => {
                return Err(invalid(
                    "current diagnostic status and resolved_at do not correspond",
                ))
            }
        }
        self.status = status;
        self.resolved_at = resolved_at;
        Ok(self)
    }

    /// Returns the severity.
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    /// Returns bounded safe facts.
    pub fn facts(&self) -> &DiagnosticFacts {
        &self.facts
    }

    /// Returns outgoing causes in canonical order.
    pub fn causes(&self) -> &[DiagnosticCause] {
        &self.causes
    }

    /// Returns current remediation actions in canonical order.
    pub fn actions(&self) -> &[DiagnosticAction] {
        &self.actions
    }

    /// Returns the optional general correlation ID.
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }

    /// Returns optional Agent Connection correlation.
    pub fn connection_id(&self) -> Option<&AgentConnectionId> {
        self.connection_id.as_ref()
    }

    /// Returns optional project correlation.
    pub fn project_id(&self) -> Option<&ProjectId> {
        self.project_id.as_ref()
    }

    /// Returns optional integration-revision correlation.
    pub fn integration_revision(&self) -> Option<&IntegrationRevision> {
        self.integration_revision.as_ref()
    }

    /// Returns the observation time.
    pub fn observed_at(&self) -> &UtcTimestamp {
        &self.observed_at
    }

    /// Returns active or resolved state.
    pub const fn status(&self) -> CurrentDiagnosticStatus {
        self.status
    }

    /// Returns resolution time for a resolved snapshot.
    pub fn resolved_at(&self) -> Option<&UtcTimestamp> {
        self.resolved_at.as_ref()
    }
}

/// One current diagnostic key and its replaceable snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentDiagnosticFinding {
    key: CurrentDiagnosticKey,
    snapshot: CurrentDiagnosticSnapshot,
    id: DiagnosticFindingId,
    identity_digest: String,
}

impl CurrentDiagnosticFinding {
    /// Constructs a current finding and derives its ID solely from the complete key.
    pub fn try_new(
        key: CurrentDiagnosticKey,
        snapshot: CurrentDiagnosticSnapshot,
    ) -> Result<Self, DiagnosticError> {
        let id = key.finding_id();
        if snapshot
            .causes()
            .iter()
            .any(|cause| cause.finding_id() == &id)
        {
            return Err(invalid(format!(
                "diagnostic finding {id} cannot cause itself"
            )));
        }
        let identity_digest = key.identity_digest();
        Ok(Self {
            key,
            snapshot,
            id,
            identity_digest,
        })
    }

    /// Returns the derived path-opaque finding ID.
    pub fn id(&self) -> &DiagnosticFindingId {
        &self.id
    }

    /// Returns the full current identity digest.
    pub fn identity_digest(&self) -> &str {
        &self.identity_digest
    }

    /// Returns the immutable current key.
    pub fn key(&self) -> &CurrentDiagnosticKey {
        &self.key
    }

    /// Returns the replaceable snapshot.
    pub fn snapshot(&self) -> &CurrentDiagnosticSnapshot {
        &self.snapshot
    }

    /// Projects this current finding into the shared read-only report shape.
    pub fn to_diagnostic_finding(&self) -> DiagnosticFinding {
        DiagnosticFinding {
            id: self.id.clone(),
            code: self.key.code.clone(),
            domain: self.key.domain.clone(),
            stage: self.key.stage.clone(),
            severity: self.snapshot.severity,
            source: self.key.source.clone(),
            subject: self.snapshot.subject.clone(),
            facts: self.snapshot.facts.clone(),
            causes: self.snapshot.causes.clone(),
            actions: self.snapshot.actions.clone(),
            observed_at: self.snapshot.observed_at.clone(),
            correlation_id: self.snapshot.correlation_id.clone(),
            connection_id: self.snapshot.connection_id.clone(),
            project_id: self.snapshot.project_id.clone(),
            runtime_session_id: None,
            integration_revision: self.snapshot.integration_revision.clone(),
        }
    }
}

/// One lifecycle-aware finding reconstructed from strict persisted state.
#[derive(Debug, Clone, PartialEq)]
pub enum StoredDiagnosticFinding {
    /// One immutable occurrence record.
    Occurrence(OccurrenceDiagnosticFinding),
    /// One active or resolved current-state record.
    Current(CurrentDiagnosticFinding),
}

impl StoredDiagnosticFinding {
    /// Returns the exact stored lifecycle.
    pub const fn lifecycle(&self) -> DiagnosticFindingLifecycle {
        match self {
            Self::Occurrence(_) => DiagnosticFindingLifecycle::Occurrence,
            Self::Current(_) => DiagnosticFindingLifecycle::CurrentState,
        }
    }

    /// Returns the stable finding ID.
    pub fn id(&self) -> DiagnosticFindingId {
        match self {
            Self::Occurrence(finding) => finding.id(),
            Self::Current(finding) => finding.id().clone(),
        }
    }

    /// Returns the lifecycle-specific occurrence when present.
    pub fn occurrence(&self) -> Option<&OccurrenceDiagnosticFinding> {
        match self {
            Self::Occurrence(finding) => Some(finding),
            Self::Current(_) => None,
        }
    }

    /// Returns the lifecycle-specific current-state record when present.
    pub fn current(&self) -> Option<&CurrentDiagnosticFinding> {
        match self {
            Self::Occurrence(_) => None,
            Self::Current(finding) => Some(finding),
        }
    }

    /// Projects record data into the shared report-only finding shape.
    pub fn to_diagnostic_finding(&self) -> DiagnosticFinding {
        match self {
            Self::Occurrence(finding) => finding.to_diagnostic_finding(),
            Self::Current(finding) => finding.to_diagnostic_finding(),
        }
    }
}

#[derive(Serialize)]
struct StoredOccurrenceWire {
    lifecycle: DiagnosticFindingLifecycle,
    finding: DiagnosticFinding,
}

#[derive(Serialize)]
struct StoredCurrentWire<'a> {
    lifecycle: DiagnosticFindingLifecycle,
    current_state_status: CurrentDiagnosticStatus,
    resolved_at: Option<&'a UtcTimestamp>,
    finding: DiagnosticFinding,
}

impl Serialize for StoredDiagnosticFinding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Occurrence(finding) => StoredOccurrenceWire {
                lifecycle: DiagnosticFindingLifecycle::Occurrence,
                finding: finding.to_diagnostic_finding(),
            }
            .serialize(serializer),
            Self::Current(finding) => StoredCurrentWire {
                lifecycle: DiagnosticFindingLifecycle::CurrentState,
                current_state_status: finding.snapshot().status(),
                resolved_at: finding.snapshot().resolved_at(),
                finding: finding.to_diagnostic_finding(),
            }
            .serialize(serializer),
        }
    }
}

/// One lifecycle-aware finding reached at its minimum cause depth.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredDiagnosticGraphEntry {
    depth: usize,
    finding: StoredDiagnosticFinding,
}

impl StoredDiagnosticGraphEntry {
    /// Constructs one bounded graph entry.
    pub fn try_new(
        depth: usize,
        finding: StoredDiagnosticFinding,
    ) -> Result<Self, DiagnosticError> {
        if depth > MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH {
            return Err(invalid(format!(
                "stored diagnostic graph depth exceeds {MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH}"
            )));
        }
        Ok(Self { depth, finding })
    }

    /// Returns the minimum depth from the requested seed set.
    pub const fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the lifecycle-aware stored record.
    pub fn finding(&self) -> &StoredDiagnosticFinding {
        &self.finding
    }
}

/// Deterministic bounded lifecycle-aware cause traversal.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoredDiagnosticGraph {
    entries: Vec<StoredDiagnosticGraphEntry>,
    depth_limit_reached: bool,
}

impl StoredDiagnosticGraph {
    /// Validates and canonically orders one stored diagnostic graph.
    pub fn try_new(
        mut entries: Vec<StoredDiagnosticGraphEntry>,
        depth_limit_reached: bool,
    ) -> Result<Self, DiagnosticError> {
        if entries.len() > MAX_DIAGNOSTIC_FINDINGS {
            return Err(invalid(format!(
                "stored diagnostic graph has more than {MAX_DIAGNOSTIC_FINDINGS} findings"
            )));
        }
        entries.sort_by(|left, right| {
            (left.depth, left.finding.id()).cmp(&(right.depth, right.finding.id()))
        });
        let mut ids = BTreeSet::new();
        if entries.iter().any(|entry| !ids.insert(entry.finding.id())) {
            return Err(invalid(
                "stored diagnostic graph contains duplicate finding ids",
            ));
        }
        Ok(Self {
            entries,
            depth_limit_reached,
        })
    }

    /// Returns an empty complete traversal.
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            depth_limit_reached: false,
        }
    }

    /// Returns entries in canonical depth-and-ID order.
    pub fn entries(&self) -> &[StoredDiagnosticGraphEntry] {
        &self.entries
    }

    /// Reports whether the selected depth omitted another cause edge.
    pub const fn depth_limit_reached(&self) -> bool {
        self.depth_limit_reached
    }
}

fn canonical_diagnostic_causes(
    mut causes: Vec<DiagnosticCause>,
) -> Result<Vec<DiagnosticCause>, DiagnosticError> {
    if causes.len() > MAX_DIAGNOSTIC_CAUSES {
        return Err(invalid(format!(
            "diagnostic finding has more than {MAX_DIAGNOSTIC_CAUSES} causes"
        )));
    }
    causes.sort();
    if causes.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(invalid("diagnostic finding contains a duplicate cause"));
    }
    Ok(causes)
}

fn canonical_diagnostic_actions(
    mut actions: Vec<DiagnosticAction>,
) -> Result<Vec<DiagnosticAction>, DiagnosticError> {
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
    Ok(actions)
}

fn push_length_prefixed(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u64).to_be_bytes());
    output.extend_from_slice(value);
}

fn lowercase_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 0x0f) as usize] as char);
    }
    value
}

fn validate_uuid_version_four_suffix(field: &str, value: &str) -> Result<(), DiagnosticError> {
    let bytes = value.as_bytes();
    if bytes.len() != 36
        || bytes[8] != b'-'
        || bytes[13] != b'-'
        || bytes[18] != b'-'
        || bytes[23] != b'-'
        || bytes[14] != b'4'
        || !matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
        || bytes.iter().enumerate().any(|(index, byte)| {
            !matches!(index, 8 | 13 | 18 | 23) && !matches!(byte, b'0'..=b'9' | b'a'..=b'f')
        })
    {
        Err(invalid(format!(
            "{field} must contain one canonical lowercase UUIDv4 suffix"
        )))
    } else {
        Ok(())
    }
}

/// Shared read-only diagnostic projection used by reports and lookup output.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct DiagnosticFinding {
    id: DiagnosticFindingId,
    code: DiagnosticCode,
    domain: DiagnosticDomain,
    stage: DiagnosticStage,
    severity: DiagnosticSeverity,
    source: DiagnosticSource,
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
    source: DiagnosticSource,
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
            wire.source,
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
        source: DiagnosticSource,
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
            source,
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
            DiagnosticDomain::parse(owner.clone())?,
            stage,
            DiagnosticSeverity::Error,
            DiagnosticSource::parse(owner.clone())?,
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

    /// Returns the stable finding producer.
    pub fn source(&self) -> &DiagnosticSource {
        &self.source
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

/// Serializes one shared finding as a bounded single-line stderr fallback.
pub fn format_bootstrap_diagnostic_envelope(
    finding: &DiagnosticFinding,
) -> Result<String, DiagnosticError> {
    let json = serde_json::to_string(finding)
        .map_err(|_| invalid("bootstrap diagnostic finding could not be serialized"))?;
    let envelope = format!("{BOOTSTRAP_DIAGNOSTIC_ENVELOPE_PREFIX} {json}");
    if envelope.len() > MAX_BOOTSTRAP_DIAGNOSTIC_ENVELOPE_BYTES {
        return Err(invalid(format!(
            "bootstrap diagnostic envelope exceeds {MAX_BOOTSTRAP_DIAGNOSTIC_ENVELOPE_BYTES} UTF-8 bytes"
        )));
    }
    Ok(envelope)
}

/// Parses one bounded single-line stderr fallback into the shared finding model.
pub fn parse_bootstrap_diagnostic_envelope(
    envelope: &str,
) -> Result<DiagnosticFinding, DiagnosticError> {
    if envelope.is_empty()
        || envelope.len() > MAX_BOOTSTRAP_DIAGNOSTIC_ENVELOPE_BYTES
        || envelope.contains(['\r', '\n'])
    {
        return Err(invalid(
            "bootstrap diagnostic envelope is not one bounded line",
        ));
    }
    let json = envelope
        .strip_prefix(BOOTSTRAP_DIAGNOSTIC_ENVELOPE_PREFIX)
        .and_then(|value| value.strip_prefix(' '))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid("bootstrap diagnostic envelope prefix is invalid"))?;
    serde_json::from_str(json).map_err(|_| invalid("bootstrap diagnostic finding is invalid"))
}

/// Operation projected by one current diagnostic report.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticOperation {
    Init,
    Add,
    Status,
    Verify,
    Mode,
    Remove,
    DiagnosticsShow,
    DiagnosticsSession,
}

impl DiagnosticOperation {
    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Add => "add",
            Self::Status => "status",
            Self::Verify => "verify",
            Self::Mode => "mode",
            Self::Remove => "remove",
            Self::DiagnosticsShow => "diagnostics_show",
            Self::DiagnosticsSession => "diagnostics_session",
        }
    }
}

/// Complete bounded Connection context for a diagnostic projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct DiagnosticConnectionContext {
    runtime_home: String,
    connection_id: String,
    host: String,
    scope: String,
    profile: String,
    mode: String,
    repository: Option<String>,
    config_target: Option<String>,
    integration_revision: Option<IntegrationRevision>,
    runtime_session_ids: Vec<AgentRuntimeSessionId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticConnectionContextWire {
    runtime_home: String,
    connection_id: String,
    host: String,
    scope: String,
    profile: String,
    mode: String,
    repository: Option<String>,
    config_target: Option<String>,
    integration_revision: Option<IntegrationRevision>,
    runtime_session_ids: Vec<AgentRuntimeSessionId>,
}

impl<'de> Deserialize<'de> for DiagnosticConnectionContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticConnectionContextWire::deserialize(deserializer)?;
        let supplied_runtime_session_ids = wire.runtime_session_ids.clone();
        let context = Self::try_new(
            wire.runtime_home,
            wire.connection_id,
            wire.host,
            wire.scope,
            wire.profile,
            wire.mode,
            wire.repository,
            wire.config_target,
            wire.integration_revision,
            wire.runtime_session_ids,
        )
        .map_err(de::Error::custom)?;
        if supplied_runtime_session_ids != context.runtime_session_ids {
            return Err(de::Error::custom(
                "diagnostic connection runtime_session_ids are not in canonical order",
            ));
        }
        Ok(context)
    }
}

impl DiagnosticConnectionContext {
    /// Constructs one bounded current Connection context.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        runtime_home: impl Into<String>,
        connection_id: impl Into<String>,
        host: impl Into<String>,
        scope: impl Into<String>,
        profile: impl Into<String>,
        mode: impl Into<String>,
        repository: Option<String>,
        config_target: Option<String>,
        integration_revision: Option<IntegrationRevision>,
        mut runtime_session_ids: Vec<AgentRuntimeSessionId>,
    ) -> Result<Self, DiagnosticError> {
        let runtime_home = runtime_home.into();
        let connection_id = connection_id.into();
        let host = host.into();
        let scope = scope.into();
        let profile = profile.into();
        let mode = mode.into();
        validate_stable_identifier("diagnostic connection id", &connection_id)?;
        for (field, value) in [
            ("runtime_home", runtime_home.as_str()),
            ("host", host.as_str()),
            ("scope", scope.as_str()),
            ("profile", profile.as_str()),
            ("mode", mode.as_str()),
        ] {
            validate_bounded_text(field, value, 4_096)?;
        }
        for (field, value) in [
            ("repository", repository.as_deref()),
            ("config_target", config_target.as_deref()),
        ] {
            if let Some(value) = value {
                validate_bounded_text(field, value, 4_096)?;
            }
        }
        runtime_session_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        if runtime_session_ids
            .windows(2)
            .any(|pair| pair[0] == pair[1])
        {
            return Err(invalid(
                "diagnostic connection context contains duplicate runtime-session ids",
            ));
        }
        Ok(Self {
            runtime_home,
            connection_id,
            host,
            scope,
            profile,
            mode,
            repository,
            config_target,
            integration_revision,
            runtime_session_ids,
        })
    }

    pub fn runtime_home(&self) -> &str {
        &self.runtime_home
    }

    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    pub fn mode(&self) -> &str {
        &self.mode
    }

    pub fn repository(&self) -> Option<&str> {
        self.repository.as_deref()
    }

    pub fn config_target(&self) -> Option<&str> {
        self.config_target.as_deref()
    }

    pub fn integration_revision(&self) -> Option<&IntegrationRevision> {
        self.integration_revision.as_ref()
    }

    pub fn runtime_session_ids(&self) -> &[AgentRuntimeSessionId] {
        &self.runtime_session_ids
    }
}

/// Outcome of one exact bounded diagnostic-record lookup.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLookupStatus {
    /// The requested stored record was loaded and validated.
    Found,
    /// No stored record exists for the requested exact identifier.
    NotFound,
}

impl DiagnosticLookupStatus {
    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Found => "found",
            Self::NotFound => "not_found",
        }
    }
}

/// One lookup-specific envelope for a finding or runtime-session root.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiagnosticLookupReport<T> {
    schema_version: u32,
    operation: DiagnosticOperation,
    lookup_status: DiagnosticLookupStatus,
    requested_id: String,
    root: Option<T>,
    cause_graph: StoredDiagnosticGraph,
    context: Option<DiagnosticConnectionContext>,
    limits: Vec<String>,
}

impl<T> DiagnosticLookupReport<T>
where
    T: Serialize,
{
    /// Validates one bounded exact-lookup result without connection-check semantics.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        operation: DiagnosticOperation,
        lookup_status: DiagnosticLookupStatus,
        requested_id: impl Into<String>,
        root: Option<T>,
        cause_graph: StoredDiagnosticGraph,
        context: Option<DiagnosticConnectionContext>,
        mut limits: Vec<String>,
    ) -> Result<Self, DiagnosticError> {
        if !matches!(
            operation,
            DiagnosticOperation::DiagnosticsShow | DiagnosticOperation::DiagnosticsSession
        ) {
            return Err(invalid(
                "diagnostic lookup report requires a lookup operation",
            ));
        }
        let requested_id = requested_id.into();
        validate_stable_identifier("diagnostic lookup requested id", &requested_id)?;
        match (lookup_status, root.is_some()) {
            (DiagnosticLookupStatus::Found, true) | (DiagnosticLookupStatus::NotFound, false) => {}
            _ => {
                return Err(invalid(
                    "diagnostic lookup status and root presence do not correspond",
                ))
            }
        }
        if lookup_status == DiagnosticLookupStatus::NotFound
            && (!cause_graph.entries().is_empty() || cause_graph.depth_limit_reached())
        {
            return Err(invalid(
                "not-found diagnostic lookup cannot contain a cause graph",
            ));
        }
        if limits.len() > MAX_DIAGNOSTIC_LIMITS {
            return Err(invalid(format!(
                "diagnostic lookup report has more than {MAX_DIAGNOSTIC_LIMITS} limits"
            )));
        }
        for limit in &limits {
            validate_bounded_text(
                "diagnostic lookup report limit",
                limit,
                MAX_DIAGNOSTIC_FACT_STRING_BYTES,
            )?;
        }
        limits.sort();
        if limits.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid(
                "diagnostic lookup report contains duplicate limits",
            ));
        }
        let report = Self {
            schema_version: DIAGNOSTIC_LOOKUP_REPORT_SCHEMA_VERSION,
            operation,
            lookup_status,
            requested_id,
            root,
            cause_graph,
            context,
            limits,
        };
        let size = serde_json::to_vec(&report)
            .map_err(|_| invalid("diagnostic lookup report could not be serialized"))?
            .len();
        if size > MAX_DIAGNOSTIC_REPORT_BYTES {
            return Err(invalid(format!(
                "diagnostic lookup report exceeds {MAX_DIAGNOSTIC_REPORT_BYTES} serialized bytes"
            )));
        }
        Ok(report)
    }

    /// Returns the only current lookup schema version.
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Returns the exact lookup operation.
    pub const fn operation(&self) -> DiagnosticOperation {
        self.operation
    }

    /// Returns whether the exact requested record was loaded.
    pub const fn lookup_status(&self) -> DiagnosticLookupStatus {
        self.lookup_status
    }

    /// Returns the exact requested identifier.
    pub fn requested_id(&self) -> &str {
        &self.requested_id
    }

    /// Returns the typed root when found.
    pub fn root(&self) -> Option<&T> {
        self.root.as_ref()
    }

    /// Returns the lifecycle-aware bounded cause graph.
    pub fn cause_graph(&self) -> &StoredDiagnosticGraph {
        &self.cause_graph
    }

    /// Returns optional bounded Connection context.
    pub fn context(&self) -> Option<&DiagnosticConnectionContext> {
        self.context.as_ref()
    }

    /// Returns bounded lookup limitations.
    pub fn limits(&self) -> &[String] {
        &self.limits
    }
}

/// One deduplicated report action with the root causes it remediates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct DiagnosticReportAction {
    code: DiagnosticCode,
    summary: String,
    root_cause_ids: Vec<DiagnosticFindingId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticReportActionWire {
    code: DiagnosticCode,
    summary: String,
    root_cause_ids: Vec<DiagnosticFindingId>,
}

impl<'de> Deserialize<'de> for DiagnosticReportAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticReportActionWire::deserialize(deserializer)?;
        let supplied_root_cause_ids = wire.root_cause_ids.clone();
        let action = Self::try_new(wire.code, wire.summary, wire.root_cause_ids)
            .map_err(de::Error::custom)?;
        if supplied_root_cause_ids != action.root_cause_ids {
            return Err(de::Error::custom(
                "diagnostic action root_cause_ids are not unique and canonically ordered",
            ));
        }
        Ok(action)
    }
}

impl DiagnosticReportAction {
    pub fn try_new(
        code: DiagnosticCode,
        summary: impl Into<String>,
        mut root_cause_ids: Vec<DiagnosticFindingId>,
    ) -> Result<Self, DiagnosticError> {
        let summary = summary.into();
        validate_bounded_text(
            "diagnostic report action summary",
            &summary,
            MAX_DIAGNOSTIC_FACT_STRING_BYTES,
        )?;
        root_cause_ids.sort();
        root_cause_ids.dedup();
        if root_cause_ids.len() > MAX_DIAGNOSTIC_ROOT_CAUSES {
            return Err(invalid("diagnostic report action has too many root causes"));
        }
        Ok(Self {
            code,
            summary,
            root_cause_ids,
        })
    }

    pub fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn root_cause_ids(&self) -> &[DiagnosticFindingId] {
        &self.root_cause_ids
    }
}

/// Current shared JSON diagnostic report.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct DiagnosticReport {
    schema_version: u32,
    operation: DiagnosticOperation,
    status: ConnectionStatus,
    generated_at: UtcTimestamp,
    connection: Option<DiagnosticConnectionContext>,
    checks: Vec<ConnectionCheck>,
    findings: Vec<DiagnosticFinding>,
    root_cause_ids: Vec<DiagnosticFindingId>,
    actions: Vec<DiagnosticReportAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_details: Option<JsonObject>,
    limits: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticReportWire {
    schema_version: u32,
    operation: DiagnosticOperation,
    status: ConnectionStatus,
    generated_at: UtcTimestamp,
    connection: Option<DiagnosticConnectionContext>,
    checks: Vec<ConnectionCheck>,
    findings: Vec<DiagnosticFinding>,
    root_cause_ids: Vec<DiagnosticFindingId>,
    actions: Vec<DiagnosticReportAction>,
    operation_details: Option<JsonObject>,
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
        let supplied_root_cause_ids = wire.root_cause_ids;
        let report = Self::try_new(
            wire.operation,
            wire.status,
            wire.generated_at,
            wire.connection,
            wire.checks,
            wire.findings,
            wire.actions,
            wire.operation_details,
            wire.limits,
        )
        .map_err(de::Error::custom)?;
        if supplied_root_cause_ids != report.root_cause_ids {
            return Err(de::Error::custom(
                "diagnostic report root_cause_ids do not match the finding cause graph",
            ));
        }
        Ok(report)
    }
}

impl DiagnosticReport {
    /// Validates and canonically orders one current diagnostic report.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        operation: DiagnosticOperation,
        status: ConnectionStatus,
        generated_at: UtcTimestamp,
        connection: Option<DiagnosticConnectionContext>,
        mut checks: Vec<ConnectionCheck>,
        mut findings: Vec<DiagnosticFinding>,
        mut actions: Vec<DiagnosticReportAction>,
        operation_details: Option<JsonObject>,
        mut limits: Vec<String>,
    ) -> Result<Self, DiagnosticError> {
        validate_timestamp("diagnostic report generated_at", &generated_at)?;
        if checks.len() > 64 {
            return Err(invalid("diagnostic report has more than 64 checks"));
        }
        if findings.len() > MAX_DIAGNOSTIC_FINDINGS {
            return Err(invalid(format!(
                "diagnostic report has more than {MAX_DIAGNOSTIC_FINDINGS} findings"
            )));
        }
        if limits.len() > MAX_DIAGNOSTIC_LIMITS {
            return Err(invalid(format!(
                "diagnostic report has more than {MAX_DIAGNOSTIC_LIMITS} limits"
            )));
        }

        checks.sort_by_key(ConnectionCheck::id);
        if checks.windows(2).any(|pair| pair[0].id() == pair[1].id()) {
            return Err(invalid("diagnostic report contains duplicate check ids"));
        }
        findings.sort_by(|left, right| left.id.cmp(&right.id));
        if findings.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(invalid("diagnostic report contains duplicate finding ids"));
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

        validate_cause_graph(&findings)?;
        let selected = checks
            .iter()
            .filter(|check| {
                matches!(
                    check.status(),
                    ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
                )
            })
            .flat_map(|check| check.cause_finding_ids().iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let root_cause_ids = if selected.is_empty() {
            Vec::new()
        } else {
            diagnostic_root_cause_ids(&findings, &selected, MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH)?
        };
        actions.sort_by(|left, right| left.code.cmp(&right.code));
        if actions.windows(2).any(|pair| pair[0].code == pair[1].code) {
            return Err(invalid("diagnostic report contains duplicate action codes"));
        }
        let root_set = root_cause_ids.iter().collect::<BTreeSet<_>>();
        if actions.iter().any(|action| {
            action
                .root_cause_ids
                .iter()
                .any(|finding_id| !root_set.contains(finding_id))
        }) {
            return Err(invalid(
                "diagnostic report action references a non-root finding",
            ));
        }
        let report = Self {
            schema_version: DIAGNOSTIC_REPORT_SCHEMA_VERSION,
            operation,
            status,
            generated_at,
            connection,
            checks,
            findings,
            root_cause_ids,
            actions,
            operation_details,
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

    pub const fn operation(&self) -> DiagnosticOperation {
        self.operation
    }

    /// Returns the report status.
    pub const fn status(&self) -> ConnectionStatus {
        self.status
    }

    /// Returns the report generation timestamp.
    pub fn generated_at(&self) -> &UtcTimestamp {
        &self.generated_at
    }

    pub fn connection(&self) -> Option<&DiagnosticConnectionContext> {
        self.connection.as_ref()
    }

    pub fn checks(&self) -> &[ConnectionCheck] {
        &self.checks
    }

    /// Returns findings in canonical ID order.
    pub fn findings(&self) -> &[DiagnosticFinding] {
        &self.findings
    }

    /// Returns explicitly identified independent root causes in canonical ID order.
    pub fn root_cause_ids(&self) -> &[DiagnosticFindingId] {
        &self.root_cause_ids
    }

    pub fn actions(&self) -> &[DiagnosticReportAction] {
        &self.actions
    }

    pub fn operation_details(&self) -> Option<&JsonObject> {
        self.operation_details.as_ref()
    }

    /// Returns bounded report limitations in canonical order.
    pub fn limits(&self) -> &[String] {
        &self.limits
    }
}

fn validate_cause_graph(findings: &[DiagnosticFinding]) -> Result<(), DiagnosticError> {
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
    let mut marks = vec![0_u8; findings.len()];
    for index in 0..findings.len() {
        visit_cause(index, findings, &indexes, &mut marks)?;
    }
    Ok(())
}

/// Selects independent roots for explicitly identified findings by following
/// only typed cause edges.
///
/// Results are sorted by finding ID. Shared ancestors are deduplicated,
/// downstream symptoms are excluded, and unknown nodes, cycles, and traversal
/// beyond the caller-selected bound are rejected.
pub fn diagnostic_root_cause_ids(
    findings: &[DiagnosticFinding],
    selected_ids: &[DiagnosticFindingId],
    max_depth: usize,
) -> Result<Vec<DiagnosticFindingId>, DiagnosticError> {
    if max_depth > MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH {
        return Err(invalid(format!(
            "diagnostic root-cause depth must not exceed {MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH}"
        )));
    }
    if findings.len() > MAX_DIAGNOSTIC_FINDINGS {
        return Err(invalid(format!(
            "diagnostic root-cause graph has more than {MAX_DIAGNOSTIC_FINDINGS} findings"
        )));
    }
    let indexes = findings
        .iter()
        .enumerate()
        .map(|(index, finding)| (finding.id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    if indexes.len() != findings.len() {
        return Err(invalid(
            "diagnostic root-cause graph contains duplicate finding ids",
        ));
    }
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

    let mut roots = BTreeSet::new();
    let mut path = BTreeSet::new();
    let mut explored_depth = BTreeMap::<DiagnosticFindingId, usize>::new();
    for selected_id in selected_ids {
        let Some(index) = indexes.get(selected_id).copied() else {
            return Err(invalid(format!(
                "diagnostic root-cause selection references unknown finding {selected_id}"
            )));
        };
        visit_root_cause(
            index,
            0,
            max_depth,
            findings,
            &indexes,
            &mut path,
            &mut explored_depth,
            &mut roots,
        )?;
    }
    if roots.len() > MAX_DIAGNOSTIC_ROOT_CAUSES {
        return Err(invalid(format!(
            "diagnostic root-cause selection has more than {MAX_DIAGNOSTIC_ROOT_CAUSES} roots"
        )));
    }
    Ok(roots.into_iter().collect())
}

#[allow(clippy::too_many_arguments)]
fn visit_root_cause(
    index: usize,
    depth: usize,
    max_depth: usize,
    findings: &[DiagnosticFinding],
    indexes: &BTreeMap<DiagnosticFindingId, usize>,
    path: &mut BTreeSet<DiagnosticFindingId>,
    explored_depth: &mut BTreeMap<DiagnosticFindingId, usize>,
    roots: &mut BTreeSet<DiagnosticFindingId>,
) -> Result<(), DiagnosticError> {
    let finding = &findings[index];
    if !path.insert(finding.id.clone()) {
        return Err(invalid(format!(
            "diagnostic cause graph contains a cycle at {}",
            finding.id
        )));
    }
    if explored_depth
        .get(&finding.id)
        .is_some_and(|prior_depth| *prior_depth <= depth)
    {
        path.remove(&finding.id);
        return Ok(());
    }
    explored_depth.insert(finding.id.clone(), depth);
    if finding.causes.is_empty() {
        roots.insert(finding.id.clone());
    } else {
        if depth == max_depth {
            return Err(invalid(format!(
                "diagnostic root-cause traversal exceeded depth {max_depth} at {}",
                finding.id
            )));
        }
        for cause in &finding.causes {
            visit_root_cause(
                indexes[&cause.finding_id],
                depth + 1,
                max_depth,
                findings,
                indexes,
                path,
                explored_depth,
                roots,
            )?;
        }
    }
    path.remove(&finding.id);
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
    use crate::ConnectionCheckKind;

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

    #[derive(Serialize)]
    struct IdentityOrderFacts {
        values: HashMap<String, String>,
    }

    impl DiagnosticFactSource for IdentityOrderFacts {}

    #[allow(clippy::too_many_arguments)]
    fn current_key(
        scope_kind: DiagnosticScopeKind,
        scope_identity: &str,
        code: &str,
        domain: &str,
        stage: &str,
        source: &str,
        subject_namespace: &str,
        subject_identity_input: &str,
    ) -> CurrentDiagnosticKey {
        let subject_identity = DiagnosticSubjectIdentity::from_canonical_bytes(
            format!("volicord.test-subject:{subject_namespace}:{subject_identity_input}")
                .as_bytes(),
        );
        CurrentDiagnosticKey::new(
            DiagnosticScope::try_new(scope_kind, scope_identity).unwrap(),
            DiagnosticCode::parse(code).unwrap(),
            DiagnosticDomain::parse(domain).unwrap(),
            DiagnosticStage::parse(stage).unwrap(),
            DiagnosticSource::parse(source).unwrap(),
            subject_identity,
        )
    }

    #[test]
    fn diagnostic_subject_identity_validates_exact_persisted_token_spelling() {
        let identity = DiagnosticSubjectIdentity::from_canonical_bytes(b"owner canonical bytes");
        assert_eq!(identity.as_str().len(), "sha256:".len() + 64);
        assert_eq!(
            DiagnosticSubjectIdentity::parse_persisted(identity.as_str()).unwrap(),
            identity
        );
        for invalid in [
            "sha512:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "sha256:gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg",
        ] {
            assert!(DiagnosticSubjectIdentity::parse_persisted(invalid).is_err());
        }
    }

    #[test]
    fn current_diagnostic_key_encoding_is_deterministic_and_complete() {
        let baseline = current_key(
            DiagnosticScopeKind::Connection,
            "connection:opaque-01",
            "guard.managed_file.missing",
            "guard",
            "guard_files",
            "administrative_cli",
            "guard_managed_artifact",
            ".volicord/hooks/pre-commit",
        );
        let independently_constructed = current_key(
            DiagnosticScopeKind::Connection,
            "connection:opaque-01",
            "guard.managed_file.missing",
            "guard",
            "guard_files",
            "administrative_cli",
            "guard_managed_artifact",
            ".volicord/hooks/pre-commit",
        );
        assert_eq!(
            baseline.canonical_identity_bytes(),
            independently_constructed.canonical_identity_bytes()
        );
        assert_eq!(
            baseline.identity_digest(),
            independently_constructed.identity_digest()
        );
        assert_eq!(
            baseline.finding_id(),
            independently_constructed.finding_id()
        );

        let variants = [
            current_key(
                DiagnosticScopeKind::Project,
                "connection:opaque-01",
                "guard.managed_file.missing",
                "guard",
                "guard_files",
                "administrative_cli",
                "guard_managed_artifact",
                ".volicord/hooks/pre-commit",
            ),
            current_key(
                DiagnosticScopeKind::Connection,
                "connection:opaque-02",
                "guard.managed_file.missing",
                "guard",
                "guard_files",
                "administrative_cli",
                "guard_managed_artifact",
                ".volicord/hooks/pre-commit",
            ),
            current_key(
                DiagnosticScopeKind::Connection,
                "connection:opaque-01",
                "guard.managed_file.changed",
                "guard",
                "guard_files",
                "administrative_cli",
                "guard_managed_artifact",
                ".volicord/hooks/pre-commit",
            ),
            current_key(
                DiagnosticScopeKind::Connection,
                "connection:opaque-01",
                "guard.managed_file.missing",
                "host",
                "guard_files",
                "administrative_cli",
                "guard_managed_artifact",
                ".volicord/hooks/pre-commit",
            ),
            current_key(
                DiagnosticScopeKind::Connection,
                "connection:opaque-01",
                "guard.managed_file.missing",
                "guard",
                "host_observation",
                "administrative_cli",
                "guard_managed_artifact",
                ".volicord/hooks/pre-commit",
            ),
            current_key(
                DiagnosticScopeKind::Connection,
                "connection:opaque-01",
                "guard.managed_file.missing",
                "guard",
                "guard_files",
                "guard_audit",
                "guard_managed_artifact",
                ".volicord/hooks/pre-commit",
            ),
            current_key(
                DiagnosticScopeKind::Connection,
                "connection:opaque-01",
                "guard.managed_file.missing",
                "guard",
                "guard_files",
                "administrative_cli",
                "managed_file",
                ".volicord/hooks/pre-commit",
            ),
            current_key(
                DiagnosticScopeKind::Connection,
                "connection:opaque-01",
                "guard.managed_file.missing",
                "guard",
                "guard_files",
                "administrative_cli",
                "guard_managed_artifact",
                ".volicord/hooks/commit-msg",
            ),
        ];
        for variant in variants {
            assert_ne!(
                baseline.canonical_identity_bytes(),
                variant.canonical_identity_bytes()
            );
            assert_ne!(baseline.identity_digest(), variant.identity_digest());
            assert_ne!(baseline.finding_id(), variant.finding_id());
        }
    }

    #[test]
    fn current_diagnostic_ids_preserve_long_opaque_coordinates_without_leaking_them() {
        let coordinates = [
            (
                DiagnosticScopeKind::Connection,
                format!("connection::{}", "연결!?/[]{}".repeat(48)),
            ),
            (
                DiagnosticScopeKind::Project,
                format!("project::{}", "프로젝트!?/[]{}".repeat(40)),
            ),
        ];
        let subject = "managed/path/.volicord/긴-경로/subject with punctuation!?[]{}";
        let mut ids = Vec::new();

        for (scope_kind, scope_identity) in coordinates {
            assert!(scope_identity.len() <= MAX_DIAGNOSTIC_SCOPE_IDENTITY_BYTES);
            let dotted = current_key(
                scope_kind,
                &scope_identity,
                "a.b_c",
                "test",
                "verification",
                "test_runner",
                "managed_path",
                subject,
            );
            let underscored = current_key(
                scope_kind,
                &scope_identity,
                "a_b.c",
                "test",
                "verification",
                "test_runner",
                "managed_path",
                subject,
            );
            assert!(dotted
                .canonical_identity_bytes()
                .windows("a.b_c".len())
                .any(|window| window == b"a.b_c"));
            assert!(underscored
                .canonical_identity_bytes()
                .windows("a_b.c".len())
                .any(|window| window == b"a_b.c"));
            assert!(!dotted
                .canonical_identity_bytes()
                .windows(subject.len())
                .any(|window| window == subject.as_bytes()));
            assert_ne!(
                dotted.canonical_identity_bytes(),
                underscored.canonical_identity_bytes()
            );
            assert_ne!(dotted.identity_digest(), underscored.identity_digest());

            for key in [&dotted, &underscored] {
                let id = key.finding_id();
                assert_eq!(id.as_str().len(), CURRENT_DIAGNOSTIC_ID_PREFIX.len() + 64);
                let suffix = id
                    .as_str()
                    .strip_prefix(CURRENT_DIAGNOSTIC_ID_PREFIX)
                    .unwrap();
                assert!(suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')));
                for private_fragment in [
                    "connection::",
                    "project::",
                    "연결",
                    "프로젝트",
                    "managed_path",
                    ".volicord",
                    "긴-경로",
                ] {
                    assert!(!id.as_str().contains(private_fragment));
                }
                ids.push(id);
            }
        }

        let unique = ids.iter().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), ids.len());
    }

    #[test]
    fn current_finding_id_uses_subject_identity_not_safe_presentation() {
        let key = current_key(
            DiagnosticScopeKind::Connection,
            "connection:subject-presentation",
            "test.subject_presentation",
            "test",
            "projection",
            "test_runner",
            "managed_path",
            "/private/canonical/path",
        );
        let first = CurrentDiagnosticFinding::try_new(
            key.clone(),
            CurrentDiagnosticSnapshot::try_new(
                DiagnosticSubject::try_new("managed_path", "redacted-primary").unwrap(),
                DiagnosticSeverity::Warning,
                DiagnosticFacts::empty(),
                timestamp(),
            )
            .unwrap(),
        )
        .unwrap();
        let second = CurrentDiagnosticFinding::try_new(
            key,
            CurrentDiagnosticSnapshot::try_new(
                DiagnosticSubject::try_new("managed_path", "redacted-updated").unwrap(),
                DiagnosticSeverity::Warning,
                DiagnosticFacts::empty(),
                timestamp(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(first.id(), second.id());
        assert_ne!(first.snapshot().subject(), second.snapshot().subject());

        let other_namespace = current_key(
            DiagnosticScopeKind::Connection,
            "connection:subject-presentation",
            "test.subject_presentation",
            "test",
            "projection",
            "test_runner",
            "repository_trust",
            "/private/canonical/path",
        );
        assert_ne!(first.id(), &other_namespace.finding_id());
    }

    #[test]
    fn current_identity_is_independent_of_map_cause_and_action_construction_order() {
        let key = || {
            current_key(
                DiagnosticScopeKind::Connection,
                "connection:construction-order",
                "test.construction_order",
                "test",
                "verification",
                "test_runner",
                "managed_artifact",
                "opaque-subject",
            )
        };
        let cause_a = DiagnosticCause::new(DiagnosticFindingId::parse("finding.cause_a").unwrap());
        let cause_b = DiagnosticCause::new(DiagnosticFindingId::parse("finding.cause_b").unwrap());
        let action_a = DiagnosticAction::try_new(
            DiagnosticCode::parse("action.test.alpha").unwrap(),
            "Apply alpha repair",
        )
        .unwrap();
        let action_b = DiagnosticAction::try_new(
            DiagnosticCode::parse("action.test.beta").unwrap(),
            "Apply beta repair",
        )
        .unwrap();
        let mut first_values = HashMap::new();
        first_values.insert("alpha".to_owned(), "one".to_owned());
        first_values.insert("beta".to_owned(), "two".to_owned());
        let mut second_values = HashMap::new();
        second_values.insert("beta".to_owned(), "two".to_owned());
        second_values.insert("alpha".to_owned(), "one".to_owned());
        let first_snapshot = CurrentDiagnosticSnapshot::try_new(
            DiagnosticSubject::try_new("managed_artifact", "first display").unwrap(),
            DiagnosticSeverity::Warning,
            DiagnosticFacts::project(&IdentityOrderFacts {
                values: first_values,
            })
            .unwrap(),
            timestamp(),
        )
        .unwrap()
        .with_causes(vec![cause_b.clone(), cause_a.clone()])
        .unwrap()
        .with_actions(vec![action_b.clone(), action_a.clone()])
        .unwrap();
        let second_snapshot = CurrentDiagnosticSnapshot::try_new(
            DiagnosticSubject::try_new("managed_artifact", "second display").unwrap(),
            DiagnosticSeverity::Warning,
            DiagnosticFacts::project(&IdentityOrderFacts {
                values: second_values,
            })
            .unwrap(),
            timestamp(),
        )
        .unwrap()
        .with_causes(vec![cause_a, cause_b])
        .unwrap()
        .with_actions(vec![action_a, action_b])
        .unwrap();
        let first = CurrentDiagnosticFinding::try_new(key(), first_snapshot).unwrap();
        let second = CurrentDiagnosticFinding::try_new(key(), second_snapshot).unwrap();

        assert_eq!(
            first.key().canonical_identity_bytes(),
            second.key().canonical_identity_bytes()
        );
        assert_eq!(first.identity_digest(), second.identity_digest());
        assert_eq!(first.id(), second.id());
        assert_ne!(first.snapshot().subject(), second.snapshot().subject());
        assert_eq!(first.snapshot().facts(), second.snapshot().facts());
        assert_eq!(first.snapshot().causes(), second.snapshot().causes());
        assert_eq!(first.snapshot().actions(), second.snapshot().actions());
    }

    #[test]
    fn repeated_occurrences_receive_distinct_generated_ids() {
        let generator = crate::SequenceDurableIdGenerator::new([
            "00000000-0000-4000-8000-000000000001",
            "00000000-0000-4000-8000-000000000002",
        ]);
        let data = DiagnosticFindingData::try_new(
            DiagnosticCode::parse("test.repeated_occurrence").unwrap(),
            DiagnosticDomain::parse("test").unwrap(),
            DiagnosticStage::parse("verification").unwrap(),
            DiagnosticSeverity::Warning,
            DiagnosticSource::parse("test_runner").unwrap(),
            DiagnosticSubject::try_new("operation", "same-subject").unwrap(),
            DiagnosticFacts::empty(),
            timestamp(),
        )
        .unwrap();
        let first =
            OccurrenceDiagnosticFinding::try_new_with_generator(data.clone(), None, &generator)
                .unwrap();
        let second =
            OccurrenceDiagnosticFinding::try_new_with_generator(data, None, &generator).unwrap();
        assert_ne!(first.id(), second.id());
        assert!(first.id().as_str().starts_with("finding.occurrence_"));
        assert!(second.id().as_str().starts_with("finding.occurrence_"));
        assert!(!first
            .id()
            .as_str()
            .starts_with(CURRENT_DIAGNOSTIC_ID_PREFIX));
        assert_eq!(first.data().code(), second.data().code());
        assert_eq!(first.data().subject(), second.data().subject());
    }

    fn finding(id: &str) -> DiagnosticFinding {
        DiagnosticFinding::try_new(
            DiagnosticFindingId::parse(id).unwrap(),
            DiagnosticCode::parse("test.check_failed").unwrap(),
            DiagnosticDomain::parse("test").unwrap(),
            DiagnosticStage::parse("verification").unwrap(),
            DiagnosticSeverity::Error,
            DiagnosticSource::parse("test_runner").unwrap(),
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

    fn failed_check(cause_finding_ids: Vec<DiagnosticFindingId>) -> ConnectionCheck {
        ConnectionCheck::try_new(
            ConnectionCheckKind::HostSession,
            ConnectionCheckStatus::Failed,
            cause_finding_ids,
            Some("test_check_failed".to_owned()),
            "Test check failed",
            None,
            Some(timestamp()),
        )
        .unwrap()
    }

    fn report_with_findings(
        status: ConnectionStatus,
        findings: Vec<DiagnosticFinding>,
        selected_ids: Vec<DiagnosticFindingId>,
        limits: Vec<String>,
    ) -> Result<DiagnosticReport, DiagnosticError> {
        let checks = if selected_ids.is_empty() {
            Vec::new()
        } else {
            vec![failed_check(selected_ids)]
        };
        DiagnosticReport::try_new(
            DiagnosticOperation::Verify,
            status,
            timestamp(),
            None,
            checks,
            findings,
            Vec::new(),
            None,
            limits,
        )
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
        let first = report_with_findings(
            ConnectionStatus::Failed,
            vec![finding("finding.z"), finding("finding.a")],
            vec![
                DiagnosticFindingId::parse("finding.z").unwrap(),
                DiagnosticFindingId::parse("finding.a").unwrap(),
            ],
            vec!["second limit".to_owned(), "first limit".to_owned()],
        )
        .unwrap();
        let second = report_with_findings(
            ConnectionStatus::Failed,
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
    struct PropertyFacts {
        fields: BTreeMap<String, Value>,
    }

    impl DiagnosticFactSource for PropertyFacts {}

    #[test]
    fn property_fact_projection_enforces_bounds_and_redacts_secret_shapes() {
        let secret_keys = [
            "secret",
            "API-TOKEN",
            "access token",
            "customer_password",
            "buildCredential",
            "request-headers",
            "env vars",
            "toolArguments",
            "raw-request",
            "filesystem-contents",
        ];
        for seed in 0..256_usize {
            let secret_key = secret_keys[seed % secret_keys.len()];
            let secret = format!("property-secret-{seed}");
            let mut fields = BTreeMap::new();
            fields.insert(secret_key.to_owned(), json!(secret));
            fields.insert(
                "long_text".to_owned(),
                json!("가".repeat(MAX_DIAGNOSTIC_FACT_STRING_BYTES + seed + 1)),
            );
            fields.insert(
                "collection".to_owned(),
                Value::Array(
                    (0..MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS + seed % 17 + 1)
                        .map(|index| json!(format!("item-{seed}-{index}")))
                        .collect(),
                ),
            );
            fields.insert(
                "nested".to_owned(),
                json!({"a": {"b": {"c": {"d": {"e": {"f": seed}}}}}}),
            );

            let facts = DiagnosticFacts::project(&PropertyFacts { fields }).unwrap();
            let serialized = serde_json::to_string(&facts).unwrap();
            assert!(serialized.len() <= MAX_DIAGNOSTIC_FACT_BYTES, "seed {seed}");
            assert!(!serialized.contains(&secret), "seed {seed} leaked a secret");
            assert_eq!(facts.data()["fields"][secret_key], REDACTED_VALUE);
            assert!(facts
                .redacted_fields()
                .contains(&format!("fields.{secret_key}")));
            assert_projected_fact_bounds(&Value::Object(
                facts.data().clone().into_iter().collect(),
            ));
        }
    }

    fn assert_projected_fact_bounds(value: &Value) {
        match value {
            Value::String(value) => {
                assert!(value.len() <= MAX_DIAGNOSTIC_FACT_STRING_BYTES);
            }
            Value::Array(values) => {
                assert!(values.len() <= MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS);
                values.iter().for_each(assert_projected_fact_bounds);
            }
            Value::Object(values) => {
                assert!(values.len() <= MAX_DIAGNOSTIC_FACT_COLLECTION_ITEMS);
                values.values().for_each(assert_projected_fact_bounds);
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
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
        assert!(report_with_findings(
            ConnectionStatus::Failed,
            vec![missing],
            vec![DiagnosticFindingId::parse("finding.child").unwrap()],
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
        assert!(report_with_findings(
            ConnectionStatus::Failed,
            vec![left, right],
            vec![DiagnosticFindingId::parse("finding.left").unwrap()],
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
        let symptom = finding("finding.check")
            .with_causes(vec![
                DiagnosticCause::new(first_id.clone()),
                DiagnosticCause::new(second_id.clone()),
            ])
            .unwrap();
        let report = report_with_findings(
            ConnectionStatus::Failed,
            vec![
                finding(first_id.as_str()),
                finding(second_id.as_str()),
                symptom,
            ],
            vec![DiagnosticFindingId::parse("finding.check").unwrap()],
            vec!["one auxiliary observation was unavailable".to_owned()],
        )
        .unwrap();
        assert_eq!(report.root_cause_ids(), &[first_id, second_id]);
        assert_eq!(report.status(), ConnectionStatus::Failed);
    }

    #[test]
    fn root_selection_deduplicates_cause_chains_without_prose_or_input_order() {
        let root_id = DiagnosticFindingId::parse("finding.root").unwrap();
        let middle_id = DiagnosticFindingId::parse("finding.middle").unwrap();
        let middle = finding(middle_id.as_str())
            .with_causes(vec![DiagnosticCause::new(root_id.clone())])
            .unwrap();
        let first = finding("finding.first_symptom")
            .with_causes(vec![DiagnosticCause::new(middle_id.clone())])
            .unwrap();
        let second = finding("finding.second_symptom")
            .with_causes(vec![DiagnosticCause::new(middle_id)])
            .unwrap();
        let findings = vec![second, finding(root_id.as_str()), first, middle];
        let selected = vec![
            DiagnosticFindingId::parse("finding.second_symptom").unwrap(),
            DiagnosticFindingId::parse("finding.first_symptom").unwrap(),
        ];
        assert_eq!(
            diagnostic_root_cause_ids(&findings, &selected, 2).unwrap(),
            vec![root_id]
        );
        assert!(diagnostic_root_cause_ids(&findings, &selected, 1)
            .unwrap_err()
            .detail()
            .contains("exceeded depth"));
    }

    #[test]
    fn property_cause_graphs_stay_acyclic_and_root_selection_is_deterministic() {
        for seed in 0..96_usize {
            let length = 2 + seed % 10;
            let ids = (0..length)
                .map(|index| {
                    DiagnosticFindingId::parse(format!("finding.property_{seed:02}_{index:02}"))
                        .unwrap()
                })
                .collect::<Vec<_>>();
            let mut chain = Vec::new();
            for index in 0..length {
                let mut node = finding(ids[index].as_str());
                if index > 0 {
                    node = node
                        .with_causes(vec![DiagnosticCause::new(ids[index - 1].clone())])
                        .unwrap();
                }
                chain.push(node);
            }
            let selected = vec![ids.last().unwrap().clone()];
            let expected = vec![ids[0].clone()];
            assert_eq!(
                diagnostic_root_cause_ids(&chain, &selected, length).unwrap(),
                expected
            );

            let rotation = seed % length;
            chain.rotate_left(rotation);
            assert_eq!(
                diagnostic_root_cause_ids(&chain, &selected, length).unwrap(),
                expected,
                "seed {seed} changed root selection after input permutation"
            );

            let mut cyclic = chain;
            let root = cyclic.iter_mut().find(|node| node.id() == &ids[0]).unwrap();
            *root = root
                .clone()
                .with_causes(vec![DiagnosticCause::new(ids[length - 1].clone())])
                .unwrap();
            assert!(
                report_with_findings(ConnectionStatus::Failed, cyclic, selected, Vec::new(),)
                    .is_err()
            );
        }
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
            DiagnosticSource::parse("test_runner").unwrap(),
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
        let report = report_with_findings(
            ConnectionStatus::Complete,
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
        unsupported["schema_version"] = json!(1);
        assert!(serde_json::from_value::<DiagnosticReport>(unsupported).is_err());

        let context = DiagnosticConnectionContext::try_new(
            "/runtime",
            "connection_1",
            "codex",
            "user",
            "record",
            "workflow",
            None,
            None,
            None,
            vec![
                AgentRuntimeSessionId::new("runtime_session_b"),
                AgentRuntimeSessionId::new("runtime_session_a"),
            ],
        )
        .unwrap();
        let mut noncanonical_context = serde_json::to_value(context).unwrap();
        noncanonical_context["runtime_session_ids"] =
            json!(["runtime_session_b", "runtime_session_a"]);
        assert!(
            serde_json::from_value::<DiagnosticConnectionContext>(noncanonical_context).is_err()
        );

        let action = DiagnosticReportAction::try_new(
            DiagnosticCode::parse("action.connection.repair").unwrap(),
            "Repair both independent roots",
            vec![
                DiagnosticFindingId::parse("finding.root_b").unwrap(),
                DiagnosticFindingId::parse("finding.root_a").unwrap(),
            ],
        )
        .unwrap();
        let mut noncanonical_action = serde_json::to_value(action).unwrap();
        noncanonical_action["root_cause_ids"] = json!(["finding.root_b", "finding.root_a"]);
        assert!(serde_json::from_value::<DiagnosticReportAction>(noncanonical_action).is_err());
    }

    #[test]
    fn bootstrap_envelope_round_trips_the_shared_bounded_finding() {
        let envelope = format_bootstrap_diagnostic_envelope(&finding("finding.bootstrap")).unwrap();
        assert!(envelope.starts_with("VOLICORD_DIAGNOSTIC_V1 {"));
        assert!(envelope.len() <= MAX_BOOTSTRAP_DIAGNOSTIC_ENVELOPE_BYTES);
        assert_eq!(
            parse_bootstrap_diagnostic_envelope(&envelope).unwrap(),
            finding("finding.bootstrap")
        );
        assert!(parse_bootstrap_diagnostic_envelope(&format!("{envelope}\n")).is_err());
    }
}
