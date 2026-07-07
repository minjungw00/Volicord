use std::{path::Path, str::FromStr, time::SystemTime};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::Value;
use volicord_store::bootstrap::ProjectRecord;
use volicord_types::{HostKind, IntegrationProfile};

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
    let event_id = event_string(
        &input.raw_value,
        &[
            &["guard_event_id"],
            &["event_id"],
            &["hook_event_id"],
            &["tool_call_id"],
            &["id"],
        ],
    )
    .unwrap_or_else(|| {
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
    });
    let occurred_at = event_string(
        &input.raw_value,
        &[&["occurred_at"], &["timestamp"], &["time"]],
    )
    .unwrap_or_else(current_timestamp);
    Ok(GuardEnvelope {
        event_id,
        session_id,
        connection_id,
        guard_installation_id: options.guard_installation_id.clone().or_else(|| {
            event_string(
                &input.raw_value,
                &[
                    &["guard_installation_id"],
                    &["host_hook", "installation_id"],
                    &["volicord", "guard_installation_id"],
                ],
            )
        }),
        host_kind,
        guard_mode,
        occurred_at,
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
    DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub(super) fn event_time_or_now(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .unwrap_or_else(|_| DateTime::<Utc>::from(SystemTime::now()))
}
