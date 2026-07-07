use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde_json::json;
use volicord_store::{bootstrap::RuntimeHomeRecord, runtime_home::resolve_runtime_home};

use crate::setup_report::{SetupSectionStatus, SetupStatus};

use super::{
    absolute_path, path_text, setup_usage, CommandStatus, SetupCommandError, SetupProcess,
};

const SETUP_CREATED_BY: &str = "volicord_cli_setup";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedSetupOptions {
    pub(super) runtime_home: Option<PathBuf>,
    pub(super) link_bin: Option<PathBuf>,
    pub(super) mcp_command: Option<PathBuf>,
    pub(super) output: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutputFormat {
    Text,
    Json,
}

pub(super) fn command_status(status: SetupStatus) -> CommandStatus {
    match status {
        SetupStatus::Complete => CommandStatus::Complete,
        SetupStatus::ActionRequired => CommandStatus::ActionRequired,
        SetupStatus::Failed => CommandStatus::Failed,
    }
}

pub(super) fn parse_setup_options(
    args: &[String],
    current_dir: &Path,
) -> Result<ParsedSetupOptions, SetupCommandError> {
    let mut parsed = ParsedSetupOptions {
        runtime_home: None,
        link_bin: None,
        mcp_command: None,
        output: OutputFormat::Text,
    };
    let mut seen = BTreeMap::<String, ()>::new();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(SetupCommandError::Usage(setup_usage()));
        }
        if !token.starts_with("--") {
            return Err(SetupCommandError::Usage(format!(
                "unexpected argument: {token}"
            )));
        }
        let without_prefix = &token[2..];
        let (name, value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name.to_owned(), Some(value.to_owned()))
        } else if without_prefix == "json" {
            (without_prefix.to_owned(), None)
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(SetupCommandError::Usage(format!(
                    "missing value for --{without_prefix}"
                )));
            };
            (without_prefix.to_owned(), Some(value.clone()))
        };
        if seen.insert(name.clone(), ()).is_some() {
            return Err(SetupCommandError::Usage(format!(
                "duplicate option: --{name}"
            )));
        }
        match name.as_str() {
            "home" => parsed.runtime_home = Some(value_path(&name, value.as_deref(), current_dir)?),
            "link-bin" => parsed.link_bin = Some(value_path(&name, value.as_deref(), current_dir)?),
            "mcp-command" => {
                parsed.mcp_command = Some(value_path(&name, value.as_deref(), current_dir)?)
            }
            "json" => {
                if value.is_some() {
                    return Err(SetupCommandError::Usage(
                        "--json does not accept a value".to_owned(),
                    ));
                }
                parsed.output = OutputFormat::Json;
            }
            _ => {
                return Err(SetupCommandError::Usage(format!(
                    "unknown option: --{name}"
                )));
            }
        }
        index += 1;
    }
    Ok(parsed)
}

fn value_path(
    name: &str,
    value: Option<&str>,
    current_dir: &Path,
) -> Result<PathBuf, SetupCommandError> {
    let value =
        value.ok_or_else(|| SetupCommandError::Usage(format!("missing value for --{name}")))?;
    if value.trim().is_empty() {
        return Err(SetupCommandError::Usage(format!(
            "--{name} must not be empty"
        )));
    }
    Ok(absolute_path(current_dir, PathBuf::from(value)))
}

pub(super) fn resolve_setup_runtime_home(
    parsed: &ParsedSetupOptions,
    current_dir: &Path,
    process: &impl SetupProcess,
) -> Result<PathBuf, SetupCommandError> {
    if let Some(path) = &parsed.runtime_home {
        Ok(path.clone())
    } else {
        resolve_runtime_home(|name| process.env_var(name), current_dir).map_err(Into::into)
    }
}

pub(super) fn runtime_home_report_section(record: &RuntimeHomeRecord) -> SetupSectionStatus {
    SetupSectionStatus::complete(
        "Runtime Home registry is ready",
        json!({
            "runtime_home": path_text(&record.runtime_home),
            "registry_db": path_text(&record.registry_db_path),
            "runtime_home_id": record.runtime_home_id,
        }),
    )
}

pub(super) fn installation_profile_failed(
    summary: impl Into<String>,
    error: &SetupCommandError,
) -> SetupSectionStatus {
    SetupSectionStatus::failed(summary, json!({ "detail": error.to_string() }))
}

pub(super) fn is_help_request(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    )
}

pub(super) fn setup_metadata_json(
    volicord_source: &str,
    mcp_source: &str,
    link_bin: Option<&Path>,
    link_results: &BTreeMap<String, String>,
) -> Result<String, SetupCommandError> {
    serde_json::to_string(&json!({
        "created_by": SETUP_CREATED_BY,
        "volicord_command_source": volicord_source,
        "volicord_mcp_command_source": mcp_source,
        "link_bin": link_bin.map(path_text),
        "link_bin_requested": link_bin.is_some(),
        "link_results": link_results,
    }))
    .map_err(|error| SetupCommandError::Runtime(error.to_string()))
}
