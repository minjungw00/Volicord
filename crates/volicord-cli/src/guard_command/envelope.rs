use std::{path::Path, str::FromStr, time::SystemTime};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::Value;
use volicord_store::bootstrap::ProjectRecord;
use volicord_types::{
    managed_host_session_id, validate_managed_host_session_id, HostKind, IntegrationProfile,
    MANAGED_HOST_SESSION_ID_PREFIX,
};

use super::{
    args::{GuardInput, GuardOptions, GuardPhase},
    stable_id, GuardCommandError, DEFAULT_INTEGRATION_PROFILE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuardEnvelope {
    pub(super) event_id: String,
    pub(super) session_id: Option<String>,
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
    phase: GuardPhase,
    options: &GuardOptions,
    input: &GuardInput,
    project: &ProjectRecord,
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
            .unwrap_or_else(|| "generic".to_owned()),
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
    let session_id = if is_managed_builtin_host(&host_kind) {
        Some(managed_builtin_session_id(
            &host_kind,
            &connection_id,
            options.session_id.as_deref(),
            &input.raw_value,
        )?)
    } else {
        let session_id = options.session_id.clone().or_else(|| {
            event_string(
                &input.raw_value,
                &[
                    &["session_id"],
                    &["session", "id"],
                    &["conversation_id"],
                    &["transcript_id"],
                ],
            )
        });
        let session_id = match (phase, session_id) {
            (GuardPhase::SessionStart | GuardPhase::PromptCapture, None) => Some(stable_id(
                "agent_session",
                &[
                    phase.command_name(),
                    &connection_id,
                    &project.project_id,
                    &input.raw_sha256,
                ],
            )),
            (_, value) => value,
        };
        if session_id
            .as_deref()
            .is_some_and(|session_id| session_id.starts_with(MANAGED_HOST_SESSION_ID_PREFIX))
        {
            return Err(GuardCommandError::Usage(
                "mhs_ session ids are reserved for managed Codex and Claude Code bindings"
                    .to_owned(),
            ));
        }
        session_id
    };
    let derived_event_id = || {
        stable_id(
            "guard_event",
            &[
                phase.command_name(),
                &connection_id,
                session_id.as_deref().unwrap_or(""),
                &project.project_id,
                &input.raw_sha256,
            ],
        )
    };
    let event_id = if is_managed_builtin_host(&host_kind) {
        derived_event_id()
    } else {
        event_string(
            &input.raw_value,
            &[
                &["guard_event_id"],
                &["event_id"],
                &["hook_event_id"],
                &["tool_call_id"],
                &["id"],
            ],
        )
        .unwrap_or_else(derived_event_id)
    };
    let occurred_at = event_string(
        &input.raw_value,
        &[&["occurred_at"], &["timestamp"], &["time"]],
    )
    .unwrap_or_else(current_timestamp);
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
    if is_managed_builtin_host(&host_kind) {
        let native_session_id = managed_native_session_id(&host_kind, &input.raw_value)?;
        if connection_id.contains(native_session_id)
            || occurred_at.contains(native_session_id)
            || guard_installation_id
                .as_deref()
                .is_some_and(|value| value.contains(native_session_id))
        {
            return Err(GuardCommandError::Usage(
                "managed host event reuses its native session id in an internal coordinate"
                    .to_owned(),
            ));
        }
    }
    Ok(GuardEnvelope {
        event_id,
        session_id,
        connection_id,
        guard_installation_id,
        host_kind,
        guard_mode,
        occurred_at,
    })
}

pub(super) fn is_managed_builtin_host(host_kind: &str) -> bool {
    matches!(host_kind, "codex" | "claude_code")
}

pub(super) fn managed_native_session_id<'a>(
    host_kind: &str,
    event: &'a Value,
) -> Result<&'a str, GuardCommandError> {
    let paths: &[&[&str]] = match host_kind {
        "codex" => &[&["session_id"], &["thread_id"]],
        "claude_code" => &[&["session_id"]],
        _ => {
            return Err(GuardCommandError::Usage(
                "managed native session extraction requires codex or claude_code".to_owned(),
            ));
        }
    };
    consistent_exact_event_string(event, paths, "native session id")
}

fn managed_builtin_session_id(
    host_kind: &str,
    connection_id: &str,
    session_override: Option<&str>,
    event: &Value,
) -> Result<String, GuardCommandError> {
    let native_session_id = managed_native_session_id(host_kind, event)?;
    let mapped = managed_host_session_id(host_kind, connection_id, native_session_id)
        .map_err(|error| GuardCommandError::Usage(error.to_string()))?;
    if let Some(session_override) = session_override {
        validate_managed_host_session_id(session_override)
            .map_err(|error| GuardCommandError::Usage(error.to_string()))?;
        if session_override != mapped {
            return Err(GuardCommandError::Usage(
                "--session does not match the canonical managed-host session binding".to_owned(),
            ));
        }
    }
    Ok(mapped)
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
    let normalized = match value.as_str() {
        "claude-code" => "claude_code".to_owned(),
        other => other.to_owned(),
    };
    HostKind::from_str(&normalized).map_err(|error| GuardCommandError::Usage(error.to_string()))?;
    Ok(normalized)
}

fn normalize_guard_mode(value: String) -> Result<String, GuardCommandError> {
    if matches!(
        value.as_str(),
        profile if profile == IntegrationProfile::Record.as_str()
            || profile == IntegrationProfile::Detective.as_str()
    ) {
        Ok(value)
    } else {
        Err(GuardCommandError::Usage(
            "integration profile must be record or detective".to_owned(),
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

pub(super) fn event_time_or_now(raw: &str) -> DateTime<Utc> {
    event_time_or_fallback(raw, DateTime::<Utc>::from(SystemTime::now()))
}

fn format_current_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

fn event_time_or_fallback(raw: &str, fallback: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builtin_native_fields_map_to_the_shared_opaque_session_id() {
        let codex = json!({
            "session_id": "native.session:1",
            "thread_id": "native.session:1"
        });
        let expected = managed_host_session_id("codex", "connection_alpha", "native.session:1")
            .expect("valid managed coordinates should bind");
        assert_eq!(
            managed_builtin_session_id("codex", "connection_alpha", None, &codex)
                .expect("consistent Codex fields should map"),
            expected
        );
        assert_eq!(
            managed_builtin_session_id("codex", "connection_alpha", Some(&expected), &codex,)
                .expect("matching canonical override should be accepted"),
            expected
        );

        let claude = json!({
            "session_id": "claude-native-1"
        });
        assert_eq!(
            managed_builtin_session_id("claude_code", "connection_alpha", None, &claude)
                .expect("valid Claude Code fields should map"),
            managed_host_session_id("claude_code", "connection_alpha", "claude-native-1",)
                .expect("valid managed coordinates should bind")
        );
    }

    #[test]
    fn builtin_native_fields_and_internal_overrides_fail_closed() {
        for event in [
            json!({ "session_id": "native with space" }),
            json!({}),
            json!({
                "session_id": "native-a",
                "thread_id": "native-b"
            }),
        ] {
            assert!(managed_builtin_session_id("codex", "connection", None, &event).is_err());
        }

        let event = json!({ "session_id": "native" });
        assert!(
            managed_builtin_session_id("codex", "connection", Some("native"), &event,).is_err()
        );
        let different = managed_host_session_id("codex", "other", "native")
            .expect("valid but different coordinates should bind");
        assert!(
            managed_builtin_session_id("codex", "connection", Some(&different), &event,).is_err()
        );
    }

    #[test]
    fn timestamp_fallbacks_preserve_order_after_a_same_millisecond_request() {
        let request_timestamp = DateTime::parse_from_rfc3339("2026-07-13T12:34:56.123Z")
            .expect("test timestamp should be RFC3339")
            .with_timezone(&Utc);
        let fallback_timestamp = DateTime::parse_from_rfc3339("2026-07-13T12:34:56.123456789Z")
            .expect("test timestamp should be RFC3339")
            .with_timezone(&Utc);

        let missing_event_timestamp =
            DateTime::parse_from_rfc3339(&format_current_timestamp(fallback_timestamp))
                .expect("formatted fallback should remain RFC3339")
                .with_timezone(&Utc);
        let invalid_event_timestamp = event_time_or_fallback("not-a-timestamp", fallback_timestamp);

        assert_eq!(missing_event_timestamp, fallback_timestamp);
        assert_eq!(invalid_event_timestamp, fallback_timestamp);
        assert!(missing_event_timestamp > request_timestamp);
        assert!(invalid_event_timestamp > request_timestamp);
    }
}
