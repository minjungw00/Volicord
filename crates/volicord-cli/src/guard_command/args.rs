use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::cli::{HookEventArgs, HookOutput};

use super::{redact_event_value, sha256_text, GuardCommandError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutputFormat {
    VolicordJson,
    Text,
    HostNative,
}

impl OutputFormat {
    pub(super) const fn default_host_kind(self) -> Option<&'static str> {
        match self {
            Self::HostNative => Some("codex"),
            Self::VolicordJson | Self::Text => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuardOptions {
    pub(super) event_file: Option<PathBuf>,
    pub(super) repo: Option<PathBuf>,
    pub(super) connection_id: Option<String>,
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

pub(super) fn guard_options(args: HookEventArgs) -> GuardOptions {
    GuardOptions {
        event_file: args.event_file,
        repo: args.repo,
        connection_id: args.connection,
        guard_installation_id: args.guard_installation,
        host_kind: args.host.map(|host| host.as_str().to_owned()),
        guard_mode: args
            .integration_profile
            .map(|profile| profile.as_str().to_owned()),
        policy_hash: args.policy_hash,
        output: if args.host_output.is_some() {
            OutputFormat::HostNative
        } else {
            match args.output.unwrap_or(HookOutput::VolicordJson) {
                HookOutput::VolicordJson => OutputFormat::VolicordJson,
                HookOutput::Text => OutputFormat::Text,
            }
        },
    }
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
