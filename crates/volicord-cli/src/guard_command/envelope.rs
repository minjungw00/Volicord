use std::{path::Path, str::FromStr, time::SystemTime};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::Value;
use volicord_host_contract::{CodexHooksV1, HostContractError, HostNativeCorrelation};
use volicord_store::bootstrap::ProjectRecord;
use volicord_types::{GuardHookPhase, HostKind, IntegrationProfile};

use super::{
    args::{GuardInput, GuardOptions},
    GuardCommandError, DEFAULT_INTEGRATION_PROFILE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuardEnvelope {
    pub(super) event_id: String,
    pub(super) session_id: Option<String>,
    pub(super) correlation: HostNativeCorrelation,
    pub(super) connection_id: String,
    pub(super) guard_installation_id: Option<String>,
    pub(super) host_kind: String,
    pub(super) guard_mode: String,
    pub(super) integration_revision: Option<String>,
    pub(super) occurred_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GuardEnvelopeError {
    pub(super) field_category: &'static str,
    pub(super) field: &'static str,
}

impl From<HostContractError> for GuardEnvelopeError {
    fn from(error: HostContractError) -> Self {
        Self {
            field_category: error.code().as_str(),
            field: error.field(),
        }
    }
}

impl GuardEnvelopeError {
    const fn missing(field: &'static str) -> Self {
        Self {
            field_category: "missing_required_field",
            field,
        }
    }

    const fn invalid(field: &'static str) -> Self {
        Self {
            field_category: "invalid_field",
            field,
        }
    }

    const fn unexpected(field: &'static str) -> Self {
        Self {
            field_category: "unexpected_value",
            field,
        }
    }

    const fn inconsistent(field: &'static str) -> Self {
        Self {
            field_category: "inconsistent_correlation",
            field,
        }
    }
}

pub(super) fn event_path_field<'a>(event: &'a Value, paths: &[&[&str]]) -> Option<&'a Path> {
    for path in paths {
        if let Some(value) = value_at(event, path).and_then(Value::as_str) {
            if !value.trim().is_empty() {
                return Some(Path::new(value));
            }
        }
    }
    None
}

pub(super) fn guard_envelope(
    phase: GuardHookPhase,
    options: &GuardOptions,
    input: &GuardInput,
    _project: &ProjectRecord,
) -> Result<GuardEnvelope, GuardEnvelopeError> {
    if let Some(failure) = input.decode_failure {
        return Err(GuardEnvelopeError {
            field_category: failure.field_category,
            field: failure.field,
        });
    }
    let connection_id = options
        .connection_id
        .clone()
        .or_else(|| {
            event_string(
                &input.raw_value,
                &[
                    &["connection_id"],
                    &["connection_internal_id"],
                    &["connection", "id"],
                    &["volicord", "connection_id"],
                ],
            )
        })
        .ok_or_else(|| GuardEnvelopeError::missing("connection_id"))?;
    let host_kind = normalize_host_kind(
        options
            .host_kind
            .clone()
            .or_else(|| {
                event_string(
                    &input.raw_value,
                    &[
                        &["host_kind"],
                        &["host", "kind"],
                        &["source", "host_kind"],
                        &["source", "host"],
                    ],
                )
            })
            .or_else(|| options.output.default_host_kind().map(str::to_owned))
            .unwrap_or_else(|| "codex".to_owned()),
    )?;
    let guard_mode = normalize_guard_mode(
        options
            .guard_mode
            .clone()
            .or_else(|| {
                event_string(
                    &input.raw_value,
                    &[
                        &["integration_profile"],
                        &["profile"],
                        &["host_hook", "profile"],
                    ],
                )
            })
            .unwrap_or_else(|| DEFAULT_INTEGRATION_PROFILE.to_owned()),
    )?;
    if host_kind != "codex" {
        return Err(GuardEnvelopeError::unexpected("host_kind"));
    }
    let hook_event = CodexHooksV1.parse(&input.raw_value)?;
    let expected_event_name = match phase {
        GuardHookPhase::PromptCapture => "UserPromptSubmit",
        GuardHookPhase::PreTool => "PreToolUse",
        GuardHookPhase::PostTool => "PostToolUse",
    };
    if hook_event.event_name() != expected_event_name {
        return Err(GuardEnvelopeError::unexpected("hook_event_name"));
    }
    let correlation = hook_event.correlation();
    let occurred_at = event_timestamp_or_now(
        &input.raw_value,
        &[&["occurred_at"], &["timestamp"], &["time"]],
    )?;
    let guard_installation_id = options.guard_installation_id.clone().or_else(|| {
        event_string(
            &input.raw_value,
            &[
                &["guard_installation_id"],
                &["host_hook", "installation_id"],
                &["volicord", "guard_installation_id"],
            ],
        )
    });
    Ok(GuardEnvelope {
        event_id: String::new(),
        session_id: None,
        correlation,
        connection_id,
        guard_installation_id,
        host_kind,
        guard_mode,
        integration_revision: None,
        occurred_at,
    })
}

pub(super) fn is_managed_builtin_host(host_kind: &str) -> bool {
    host_kind == "codex"
}

fn normalize_host_kind(value: String) -> Result<String, GuardEnvelopeError> {
    HostKind::from_str(&value).map_err(|_| GuardEnvelopeError::invalid("host_kind"))?;
    Ok(value)
}

fn normalize_guard_mode(value: String) -> Result<String, GuardEnvelopeError> {
    if value == IntegrationProfile::Record.as_str() {
        Ok(value)
    } else {
        Err(GuardEnvelopeError::unexpected("integration_profile"))
    }
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cursor = value;
    for key in path {
        cursor = cursor.get(*key)?;
    }
    Some(cursor)
}

pub(super) fn event_string(value: &Value, paths: &[&[&str]]) -> Option<String> {
    for path in paths {
        if let Some(text) = value_at(value, path).and_then(Value::as_str) {
            if !text.trim().is_empty() {
                return Some(text.to_owned());
            }
        }
    }
    None
}

pub(super) fn event_bool(value: &Value, paths: &[&[&str]]) -> Option<bool> {
    paths
        .iter()
        .find_map(|path| value_at(value, path).and_then(Value::as_bool))
}

pub(super) fn event_i64(value: &Value, paths: &[&[&str]]) -> Option<i64> {
    paths
        .iter()
        .find_map(|path| value_at(value, path).and_then(Value::as_i64))
}

fn current_timestamp() -> String {
    format_current_timestamp(DateTime::<Utc>::from(SystemTime::now()))
}

fn event_timestamp_or_now(event: &Value, paths: &[&[&str]]) -> Result<String, GuardEnvelopeError> {
    let mut selected: Option<&str> = None;
    for path in paths {
        let Some(value) = value_at(event, path) else {
            continue;
        };
        let value = value
            .as_str()
            .ok_or_else(|| GuardEnvelopeError::invalid("timestamp"))?;
        if value.is_empty() {
            return Err(GuardEnvelopeError::invalid("timestamp"));
        }
        if selected.is_some_and(|selected| selected != value) {
            return Err(GuardEnvelopeError::inconsistent("timestamp"));
        }
        selected = Some(value);
    }
    selected
        .map(|raw| {
            DateTime::parse_from_rfc3339(raw)
                .map(|timestamp| timestamp.with_timezone(&Utc))
                .map_err(|_| GuardEnvelopeError::invalid("timestamp"))
        })
        .transpose()
        .map(|timestamp| {
            timestamp
                .map(format_current_timestamp)
                .unwrap_or_else(current_timestamp)
        })
}

pub(super) fn event_time(raw: &str) -> Result<DateTime<Utc>, GuardCommandError> {
    parse_event_timestamp(raw)
}

fn format_current_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

fn parse_event_timestamp(raw: &str) -> Result<DateTime<Utc>, GuardCommandError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|_| {
            GuardCommandError::Usage(
                "managed host event timestamp must be a valid RFC 3339 instant".to_owned(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn selected_hook_profile_uses_source_specific_coordinates() {
        let event = CodexHooksV1
            .parse(&json!({
                "hook_event_name": "UserPromptSubmit",
                "session_id": "native.session:1",
                "turn_id": "native.turn:1",
                "prompt": "continue"
            }))
            .expect("current Codex hook profile should parse");
        assert_eq!(
            event.correlation().session_id().as_str(),
            "native.session:1"
        );
        assert!(matches!(
            event.correlation(),
            HostNativeCorrelation::CodexHookPrompt(_)
        ));
    }

    #[test]
    fn timestamp_aliases_are_typed_exact_and_missing_uses_the_current_sample() {
        let request_timestamp = DateTime::parse_from_rfc3339("2026-07-13T12:34:56.123Z")
            .expect("test timestamp should be RFC3339")
            .with_timezone(&Utc);
        let fallback_timestamp = DateTime::parse_from_rfc3339("2026-07-13T12:34:56.123456789Z")
            .expect("test timestamp should be RFC3339")
            .with_timezone(&Utc);

        let normalized = event_timestamp_or_now(
            &json!({"occurred_at":"2026-07-13T21:34:56.123456789+09:00"}),
            &[&["occurred_at"], &["timestamp"], &["time"]],
        )
        .expect("valid timestamp should normalize");
        assert_eq!(
            parse_event_timestamp(&normalized).expect("normalized timestamp"),
            fallback_timestamp
        );
        assert!(fallback_timestamp > request_timestamp);
        assert!(event_timestamp_or_now(
            &json!({"occurred_at": 1}),
            &[&["occurred_at"], &["timestamp"]]
        )
        .is_err());
        assert!(event_timestamp_or_now(
            &json!({"occurred_at":"not-a-timestamp"}),
            &[&["occurred_at"], &["timestamp"]]
        )
        .is_err());
        assert!(event_timestamp_or_now(
            &json!({
                "occurred_at":"2026-07-13T12:34:56Z",
                "timestamp":"2026-07-13T12:34:57Z"
            }),
            &[&["occurred_at"], &["timestamp"]]
        )
        .is_err());
        let missing = event_timestamp_or_now(&json!({}), &[&["occurred_at"], &["timestamp"]])
            .expect("missing optional timestamp should use a current sample");
        assert!(parse_event_timestamp(&missing).is_ok());
    }
}
