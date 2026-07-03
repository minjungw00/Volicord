use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use serde_json::Value;

use super::{redact_event_value, sha256_text, GuardCommandError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GuardPhase {
    SessionStart,
    PreTool,
    PostTool,
    PromptCapture,
    Stop,
}

impl GuardPhase {
    pub(super) fn event_kind(self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::PreTool => "pre_tool",
            Self::PostTool => "post_tool",
            Self::PromptCapture => "prompt_capture",
            Self::Stop => "stop",
        }
    }

    pub(super) fn command_name(self) -> &'static str {
        match self {
            Self::SessionStart => "session-start",
            Self::PreTool => "pre-tool",
            Self::PostTool => "post-tool",
            Self::PromptCapture => "prompt-capture",
            Self::Stop => "stop",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutputFormat {
    VolicordJson,
    Text,
    HostNative(HostOutputMode),
}

impl OutputFormat {
    pub(super) fn default_host_kind(self) -> Option<&'static str> {
        match self {
            Self::HostNative(HostOutputMode::Codex) => Some("codex"),
            Self::HostNative(HostOutputMode::ClaudeCode) => Some("claude_code"),
            Self::VolicordJson | Self::Text => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HostOutputMode {
    Codex,
    ClaudeCode,
}

impl HostOutputMode {
    fn from_cli(value: &str) -> Result<Self, GuardCommandError> {
        match value {
            "codex" => Ok(Self::Codex),
            "claude-code" | "claude_code" => Ok(Self::ClaudeCode),
            _ => Err(GuardCommandError::Usage(
                "--host-output must be codex or claude-code".to_owned(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuardOptions {
    pub(super) event_file: Option<PathBuf>,
    pub(super) repo: Option<PathBuf>,
    pub(super) connection_id: Option<String>,
    pub(super) session_id: Option<String>,
    pub(super) guard_installation_id: Option<String>,
    pub(super) host_kind: Option<String>,
    pub(super) guard_mode: Option<String>,
    pub(super) policy_hash: Option<String>,
    pub(super) output: OutputFormat,
}

impl Default for GuardOptions {
    fn default() -> Self {
        Self {
            event_file: None,
            repo: None,
            connection_id: None,
            session_id: None,
            guard_installation_id: None,
            host_kind: None,
            guard_mode: None,
            policy_hash: None,
            output: OutputFormat::VolicordJson,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct GuardInput {
    pub(super) raw_text: String,
    pub(super) raw_value: Value,
    pub(super) raw_sha256: String,
    pub(super) redacted_value: Value,
}

pub fn guard_usage() -> String {
    concat!(
        "volicord _hook session-start [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--integration-profile record|detective] [--policy-hash HASH] [--output volicord-json|text] [--host-output codex|claude-code]\n",
        "volicord _hook pre-tool [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--integration-profile record|detective] [--policy-hash HASH] [--output volicord-json|text] [--host-output codex|claude-code]\n",
        "volicord _hook post-tool [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--integration-profile record|detective] [--policy-hash HASH] [--output volicord-json|text] [--host-output codex|claude-code]\n",
        "volicord _hook prompt-capture [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--integration-profile record|detective] [--policy-hash HASH] [--output volicord-json|text] [--host-output codex|claude-code]\n",
        "volicord _hook stop [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--integration-profile record|detective] [--policy-hash HASH] [--output volicord-json|text] [--host-output codex|claude-code]\n",
    )
    .to_owned()
}

pub(super) fn parse_guard_options(args: &[String]) -> Result<GuardOptions, GuardCommandError> {
    let mut options = GuardOptions::default();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if matches!(token.as_str(), "-h" | "--help" | "help") {
            return Err(GuardCommandError::Usage(guard_usage()));
        }
        if let Some(value) = token.strip_prefix("--file=") {
            set_path_option(&mut options.event_file, "--file", value)?;
        } else if token == "--file" {
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| GuardCommandError::Usage("--file requires a value".to_owned()))?;
            set_path_option(&mut options.event_file, "--file", value)?;
        } else if let Some(value) = token.strip_prefix("--repo=") {
            set_path_option(&mut options.repo, "--repo", value)?;
        } else if token == "--repo" {
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| GuardCommandError::Usage("--repo requires a value".to_owned()))?;
            set_path_option(&mut options.repo, "--repo", value)?;
        } else if let Some(value) = token.strip_prefix("--connection=") {
            set_string_option(&mut options.connection_id, "--connection", value)?;
        } else if token == "--connection" {
            index += 1;
            let value = args.get(index).ok_or_else(|| {
                GuardCommandError::Usage("--connection requires a value".to_owned())
            })?;
            set_string_option(&mut options.connection_id, "--connection", value)?;
        } else if let Some(value) = token.strip_prefix("--session=") {
            set_string_option(&mut options.session_id, "--session", value)?;
        } else if token == "--session" {
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| GuardCommandError::Usage("--session requires a value".to_owned()))?;
            set_string_option(&mut options.session_id, "--session", value)?;
        } else if let Some(value) = token.strip_prefix("--guard-installation=") {
            set_string_option(
                &mut options.guard_installation_id,
                "--guard-installation",
                value,
            )?;
        } else if token == "--guard-installation" {
            index += 1;
            let value = args.get(index).ok_or_else(|| {
                GuardCommandError::Usage("--guard-installation requires a value".to_owned())
            })?;
            set_string_option(
                &mut options.guard_installation_id,
                "--guard-installation",
                value,
            )?;
        } else if let Some(value) = token.strip_prefix("--host=") {
            set_string_option(&mut options.host_kind, "--host", value)?;
        } else if token == "--host" {
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| GuardCommandError::Usage("--host requires a value".to_owned()))?;
            set_string_option(&mut options.host_kind, "--host", value)?;
        } else if let Some(value) = token.strip_prefix("--integration-profile=") {
            set_string_option(&mut options.guard_mode, "--integration-profile", value)?;
        } else if token == "--integration-profile" {
            index += 1;
            let value = args.get(index).ok_or_else(|| {
                GuardCommandError::Usage("--integration-profile requires a value".to_owned())
            })?;
            set_string_option(&mut options.guard_mode, "--integration-profile", value)?;
        } else if let Some(value) = token.strip_prefix("--policy-hash=") {
            set_string_option(&mut options.policy_hash, "--policy-hash", value)?;
        } else if token == "--policy-hash" {
            index += 1;
            let value = args.get(index).ok_or_else(|| {
                GuardCommandError::Usage("--policy-hash requires a value".to_owned())
            })?;
            set_string_option(&mut options.policy_hash, "--policy-hash", value)?;
        } else if token == "--text" {
            options.output = OutputFormat::Text;
        } else if token == "--json" {
            options.output = OutputFormat::VolicordJson;
        } else if let Some(value) = token.strip_prefix("--output=") {
            options.output = parse_output_format(value)?;
        } else if token == "--output" {
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| GuardCommandError::Usage("--output requires a value".to_owned()))?;
            options.output = parse_output_format(value)?;
        } else if let Some(value) = token.strip_prefix("--host-output=") {
            options.output = OutputFormat::HostNative(HostOutputMode::from_cli(value)?);
        } else if token == "--host-output" {
            index += 1;
            let value = args.get(index).ok_or_else(|| {
                GuardCommandError::Usage("--host-output requires a value".to_owned())
            })?;
            options.output = OutputFormat::HostNative(HostOutputMode::from_cli(value)?);
        } else if token.starts_with('-') {
            return Err(GuardCommandError::Usage(format!("unknown option: {token}")));
        } else {
            return Err(GuardCommandError::Usage(format!(
                "unexpected argument: {token}"
            )));
        }
        index += 1;
    }
    Ok(options)
}

fn parse_output_format(value: &str) -> Result<OutputFormat, GuardCommandError> {
    match value {
        "volicord-json" | "volicord_json" | "json" => Ok(OutputFormat::VolicordJson),
        "text" => Ok(OutputFormat::Text),
        other => Err(GuardCommandError::Usage(format!(
            "unsupported --output value: {other}"
        ))),
    }
}

fn set_path_option(
    slot: &mut Option<PathBuf>,
    option: &'static str,
    value: &str,
) -> Result<(), GuardCommandError> {
    if slot.is_some() {
        return Err(GuardCommandError::Usage(format!(
            "{option} was supplied more than once"
        )));
    }
    if value.trim().is_empty() {
        return Err(GuardCommandError::Usage(format!(
            "{option} requires a non-empty value"
        )));
    }
    *slot = Some(PathBuf::from(value));
    Ok(())
}

fn set_string_option(
    slot: &mut Option<String>,
    option: &'static str,
    value: &str,
) -> Result<(), GuardCommandError> {
    if slot.is_some() {
        return Err(GuardCommandError::Usage(format!(
            "{option} was supplied more than once"
        )));
    }
    if value.trim().is_empty() {
        return Err(GuardCommandError::Usage(format!(
            "{option} requires a non-empty value"
        )));
    }
    *slot = Some(value.to_owned());
    Ok(())
}

pub(super) fn read_guard_input(path: Option<&Path>) -> Result<GuardInput, GuardCommandError> {
    let raw_text = match path {
        Some(path) => fs::read_to_string(path).map_err(|error| {
            GuardCommandError::Runtime(format!(
                "failed to read host-hook event file {}: {error}",
                path.display()
            ))
        })?,
        None => {
            let mut text = String::new();
            io::stdin().read_to_string(&mut text).map_err(|error| {
                GuardCommandError::Runtime(format!("failed to read host-hook event stdin: {error}"))
            })?;
            text
        }
    };
    if raw_text.trim().is_empty() {
        return Err(GuardCommandError::Usage(
            "host-hook event JSON must not be empty".to_owned(),
        ));
    }
    let raw_value = serde_json::from_str::<Value>(&raw_text).map_err(|error| {
        GuardCommandError::Usage(format!("host-hook event must be JSON: {error}"))
    })?;
    let raw_sha256 = sha256_text(&raw_text);
    let redacted_value = redact_event_value(&raw_value);
    Ok(GuardInput {
        raw_text,
        raw_value,
        raw_sha256,
        redacted_value,
    })
}
