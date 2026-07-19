use std::{path::Path, str::FromStr, time::SystemTime};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::Value;
use volicord_store::bootstrap::ProjectRecord;
use volicord_types::{
    validate_managed_host_native_session_id, GuardHookPhase, HostKind, IntegrationProfile,
};

use super::{
    args::{GuardInput, GuardOptions},
    GuardCommandError, DEFAULT_INTEGRATION_PROFILE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuardEnvelope {
    pub(super) event_id: String,
    pub(super) session_id: Option<String>,
    pub(super) host_session_id: String,
    pub(super) host_thread_id: String,
    pub(super) host_turn_id: String,
    pub(super) connection_id: String,
    pub(super) guard_installation_id: Option<String>,
    pub(super) host_kind: String,
    pub(super) guard_mode: String,
    pub(super) occurred_at: String,
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
    _phase: GuardHookPhase,
    options: &GuardOptions,
    input: &GuardInput,
    _project: &ProjectRecord,
) -> Result<GuardEnvelope, GuardCommandError> {
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
        .ok_or_else(|| {
            GuardCommandError::Usage(
                "host-hook command requires --connection or connection_id in the event".to_owned(),
            )
        })?;
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
    let host_session_id = managed_native_session_id(&host_kind, &input.raw_value)?.to_owned();
    let host_thread_id =
        consistent_exact_event_string(&input.raw_value, &[&["thread_id"]], "native thread id")?
            .to_owned();
    let host_turn_id =
        consistent_exact_event_string(&input.raw_value, &[&["turn_id"]], "native turn id")?
            .to_owned();
    for value in [&host_session_id, &host_thread_id, &host_turn_id] {
        validate_managed_host_native_session_id(value).map_err(|error| {
            GuardCommandError::Usage(format!(
                "managed Codex event has invalid correlation metadata: {error}"
            ))
        })?;
    }
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
        host_session_id,
        host_thread_id,
        host_turn_id,
        connection_id,
        guard_installation_id,
        host_kind,
        guard_mode,
        occurred_at,
    })
}

pub(super) fn is_managed_builtin_host(host_kind: &str) -> bool {
    host_kind == "codex"
}

pub(super) fn managed_native_session_id<'a>(
    host_kind: &str,
    event: &'a Value,
) -> Result<&'a str, GuardCommandError> {
    if host_kind != "codex" {
        return Err(GuardCommandError::Usage(
            "managed native session extraction requires codex".to_owned(),
        ));
    }
    let paths: &[&[&str]] = &[&["session_id"]];
    consistent_exact_event_string(event, paths, "native session id")
}

fn consistent_exact_event_string<'a>(
    event: &'a Value,
    paths: &[&[&str]],
    label: &str,
) -> Result<&'a str, GuardCommandError> {
    let mut selected = None;
    for path in paths {
        let Some(value) = value_at(event, path) else {
            continue;
        };
        let Some(value) = value.as_str() else {
            return Err(GuardCommandError::Usage(format!(
                "managed host event {label} must be a string"
            )));
        };
        if value.is_empty() {
            return Err(GuardCommandError::Usage(format!(
                "managed host event requires a non-empty {label}"
            )));
        }
        if let Some(selected) = selected {
            if selected != value {
                return Err(GuardCommandError::Usage(format!(
                    "managed host event contains inconsistent {label} fields"
                )));
            }
        } else {
            selected = Some(value);
        }
    }
    selected.ok_or_else(|| {
        GuardCommandError::Usage(format!(
            "managed host event requires an exact {label} field"
        ))
    })
}

fn normalize_host_kind(value: String) -> Result<String, GuardCommandError> {
    HostKind::from_str(&value).map_err(|error| GuardCommandError::Usage(error.to_string()))?;
    Ok(value)
}

fn normalize_guard_mode(value: String) -> Result<String, GuardCommandError> {
    if value == IntegrationProfile::Record.as_str() {
        Ok(value)
    } else {
        Err(GuardCommandError::Usage(
            "integration profile must be record".to_owned(),
        ))
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

fn event_timestamp_or_now(event: &Value, paths: &[&[&str]]) -> Result<String, GuardCommandError> {
    let mut selected: Option<&str> = None;
    for path in paths {
        let Some(value) = value_at(event, path) else {
            continue;
        };
        let value = value.as_str().ok_or_else(|| {
            GuardCommandError::Usage(
                "managed host event timestamp aliases must be RFC 3339 strings".to_owned(),
            )
        })?;
        if value.is_empty() {
            return Err(GuardCommandError::Usage(
                "managed host event timestamp must not be empty".to_owned(),
            ));
        }
        if selected.is_some_and(|selected| selected != value) {
            return Err(GuardCommandError::Usage(
                "managed host event contains conflicting timestamp aliases".to_owned(),
            ));
        }
        selected = Some(value);
    }
    selected
        .map(parse_event_timestamp)
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
    fn builtin_parser_retains_only_the_exact_native_session_coordinate() {
        let codex = json!({
            "session_id": "native.session:1",
            "thread_id": "different.subagent.thread"
        });
        assert_eq!(
            managed_native_session_id("codex", &codex).expect("valid Codex identity should parse"),
            "native.session:1"
        );
    }

    #[test]
    fn builtin_native_fields_and_internal_overrides_fail_closed() {
        for event in [json!({ "session_id": "native with space" }), json!({})] {
            assert!(
                managed_native_session_id("codex", &event)
                    .map(validate_managed_host_native_session_id)
                    .is_err()
                    || managed_native_session_id("codex", &event)
                        .is_ok_and(|value| validate_managed_host_native_session_id(value).is_err())
            );
        }

        let different_thread = json!({
            "session_id": "native-a",
            "thread_id": "native-b"
        });
        assert_eq!(
            managed_native_session_id("codex", &different_thread)
                .expect("thread identifiers do not replace the root session binding"),
            "native-a"
        );
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
