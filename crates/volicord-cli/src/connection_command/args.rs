use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use volicord_store::agent_connections::{
    CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW, HOST_KIND_CLAUDE_CODE, HOST_KIND_CODEX,
};
use volicord_types::IntegrationProfile;

use crate::host_integration::HostKind;

use super::ConnectionCommandError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ParsedConnectionOptions {
    pub(super) host_kind: Option<HostKind>,
    pub(super) repo: Option<PathBuf>,
    pub(super) shared: bool,
    pub(super) global: bool,
    pub(super) read_only: bool,
    pub(super) dry_run: bool,
    pub(super) json: bool,
    pub(super) positionals: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum InitMode {
    #[default]
    Record,
    Detective,
}

impl InitMode {
    pub(super) fn profile_value(self) -> &'static str {
        match self {
            Self::Record => IntegrationProfile::Record.as_str(),
            Self::Detective => IntegrationProfile::Detective.as_str(),
        }
    }

    pub(super) fn guard_value(self) -> &'static str {
        self.profile_value()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ParsedInitOptions {
    pub(super) host_kind: Option<HostKind>,
    pub(super) repo: Option<PathBuf>,
    pub(super) runtime_home: Option<PathBuf>,
    pub(super) mcp_command: Option<PathBuf>,
    pub(super) mode: InitMode,
    pub(super) dry_run: bool,
    pub(super) json: bool,
}

pub fn init_usage() -> String {
    "volicord init --host codex|claude-code --repo PATH [--profile record|detective] [--home PATH] [--mcp-command PATH] [--dry-run] [--json]\n"
        .to_owned()
}

pub fn connect_usage() -> String {
    connection_add_usage()
}

pub(super) fn connection_add_usage() -> String {
    "volicord connection add [HOST] [--repo PATH] [--shared|--global] [--read-only] [--dry-run] [--json]\n"
        .to_owned()
}

pub fn connections_usage() -> String {
    connection_list_usage()
}

pub(super) fn connection_list_usage() -> String {
    "volicord connection list [--repo PATH] [--json]\n".to_owned()
}

pub fn connection_usage() -> String {
    format!(
        "{}{}{}{}{}{}",
        connection_add_usage(),
        connection_list_usage(),
        connection_status_usage(),
        connection_verify_usage(),
        connection_mode_usage(),
        connection_remove_usage()
    )
}

pub(super) fn connection_status_usage() -> String {
    "volicord connection status [HOST] [--repo PATH] [--shared|--global] [--json]\n".to_owned()
}

pub(super) fn connection_verify_usage() -> String {
    "volicord connection verify [HOST] [--repo PATH] [--shared|--global] [--json]\n".to_owned()
}

pub(super) fn connection_mode_usage() -> String {
    "volicord connection mode [HOST] workflow|read-only [--repo PATH] [--shared|--global] [--json]\n"
        .to_owned()
}

pub(super) fn connection_remove_usage() -> String {
    "volicord connection remove [HOST] [--repo PATH] [--shared|--global] [--dry-run] [--json]\n"
        .to_owned()
}

pub(super) fn is_help_request(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    )
}

pub(super) fn parse_connection_options(
    args: &[String],
    allowed: &[&str],
    max_positionals: usize,
) -> Result<ParsedConnectionOptions, ConnectionCommandError> {
    let mut parsed = ParsedConnectionOptions::default();
    let mut seen = BTreeSet::new();
    let mut index = 0;

    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(ConnectionCommandError::usage(connection_usage()));
        }
        if !token.starts_with("--") {
            parsed.positionals.push(token.clone());
            index += 1;
            continue;
        }
        let without_prefix = &token[2..];
        let (name, value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name.to_owned(), Some(value.to_owned()))
        } else if is_boolean_connection_option(without_prefix) {
            (without_prefix.to_owned(), None)
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(ConnectionCommandError::usage(format!(
                    "missing value for --{without_prefix}"
                )));
            };
            (without_prefix.to_owned(), Some(value.clone()))
        };

        if !allowed.iter().any(|allowed_name| *allowed_name == name) {
            return Err(ConnectionCommandError::usage(format!(
                "unknown option: --{name}"
            )));
        }
        if !seen.insert(name.clone()) {
            return Err(ConnectionCommandError::usage(format!(
                "duplicate option: --{name}"
            )));
        }
        set_connection_option(&mut parsed, &name, value.as_deref())?;
        index += 1;
    }

    if parsed.positionals.len() > max_positionals {
        return Err(ConnectionCommandError::usage(format!(
            "unexpected argument: {}",
            parsed.positionals[max_positionals]
        )));
    }
    if max_positionals == 1 {
        if let Some(host) = parsed.positionals.first() {
            parsed.host_kind = Some(parse_public_host_kind(host)?);
        }
    }
    if parsed.shared && parsed.global {
        return Err(ConnectionCommandError::usage(
            "--shared and --global are mutually exclusive",
        ));
    }
    Ok(parsed)
}

pub(super) fn parse_init_options(
    args: &[String],
    current_dir: &Path,
) -> Result<ParsedInitOptions, ConnectionCommandError> {
    let mut parsed = ParsedInitOptions {
        mode: InitMode::Record,
        ..ParsedInitOptions::default()
    };
    let mut seen = BTreeSet::new();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if matches!(token.as_str(), "-h" | "--help" | "help") {
            return Err(ConnectionCommandError::usage(init_usage()));
        }
        if !token.starts_with("--") {
            return Err(ConnectionCommandError::usage(format!(
                "unexpected argument: {token}"
            )));
        }
        let without_prefix = &token[2..];
        let (name, value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name.to_owned(), Some(value.to_owned()))
        } else if matches!(without_prefix, "dry-run" | "json") {
            (without_prefix.to_owned(), None)
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(ConnectionCommandError::usage(format!(
                    "missing value for --{without_prefix}"
                )));
            };
            (without_prefix.to_owned(), Some(value.clone()))
        };
        if !matches!(
            name.as_str(),
            "host" | "repo" | "profile" | "home" | "mcp-command" | "dry-run" | "json"
        ) {
            return Err(ConnectionCommandError::usage(format!(
                "unknown option: --{name}"
            )));
        }
        if !seen.insert(name.clone()) {
            return Err(ConnectionCommandError::usage(format!(
                "duplicate option: --{name}"
            )));
        }
        match name.as_str() {
            "host" => {
                parsed.host_kind = Some(parse_public_host_kind(&value_text(
                    &name,
                    value.as_deref(),
                )?)?)
            }
            "repo" => {
                parsed.repo = Some(absolute_path(
                    current_dir,
                    value_path(&name, value.as_deref())?,
                ))
            }
            "profile" => parsed.mode = parse_init_profile(&value_text(&name, value.as_deref())?)?,
            "home" => {
                parsed.runtime_home = Some(absolute_path(
                    current_dir,
                    value_path(&name, value.as_deref())?,
                ));
            }
            "mcp-command" => {
                parsed.mcp_command = Some(absolute_path(
                    current_dir,
                    value_path(&name, value.as_deref())?,
                ));
            }
            "dry-run" => {
                reject_boolean_value(&name, value.as_deref())?;
                parsed.dry_run = true;
            }
            "json" => {
                reject_boolean_value(&name, value.as_deref())?;
                parsed.json = true;
            }
            _ => unreachable!("validated option name"),
        }
        index += 1;
    }
    Ok(parsed)
}

fn parse_init_profile(value: &str) -> Result<InitMode, ConnectionCommandError> {
    match value {
        "record" => Ok(InitMode::Record),
        "detective" => Ok(InitMode::Detective),
        other => Err(ConnectionCommandError::usage(format!(
            "unknown integration profile: {other}; use record or detective"
        ))),
    }
}

pub(super) fn init_output_format(parsed: &ParsedInitOptions) -> OutputFormat {
    if parsed.json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    }
}

fn is_boolean_connection_option(name: &str) -> bool {
    matches!(name, "shared" | "global" | "read-only" | "dry-run" | "json")
}

fn set_connection_option(
    parsed: &mut ParsedConnectionOptions,
    name: &str,
    value: Option<&str>,
) -> Result<(), ConnectionCommandError> {
    match name {
        "repo" => parsed.repo = Some(value_path(name, value)?),
        "shared" => {
            reject_boolean_value(name, value)?;
            parsed.shared = true;
        }
        "global" => {
            reject_boolean_value(name, value)?;
            parsed.global = true;
        }
        "read-only" => {
            reject_boolean_value(name, value)?;
            parsed.read_only = true;
        }
        "dry-run" => {
            reject_boolean_value(name, value)?;
            parsed.dry_run = true;
        }
        "json" => {
            reject_boolean_value(name, value)?;
            parsed.json = true;
        }
        _ => {
            return Err(ConnectionCommandError::usage(format!(
                "unknown option: --{name}"
            )))
        }
    }
    Ok(())
}

fn reject_boolean_value(name: &str, value: Option<&str>) -> Result<(), ConnectionCommandError> {
    if value.is_some() {
        Err(ConnectionCommandError::usage(format!(
            "--{name} does not accept a value"
        )))
    } else {
        Ok(())
    }
}

pub(super) fn connection_output_format(parsed: &ParsedConnectionOptions) -> OutputFormat {
    if parsed.json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    }
}

pub(super) fn parse_public_host_kind(value: &str) -> Result<HostKind, ConnectionCommandError> {
    match value {
        HOST_KIND_CODEX => Ok(HostKind::Codex),
        "claude-code" | HOST_KIND_CLAUDE_CODE => Ok(HostKind::ClaudeCode),
        other => Err(ConnectionCommandError::usage(format!(
            "UNSUPPORTED_HOST: unknown host: {other}; choose `codex` or `claude-code`"
        ))),
    }
}

pub(super) fn parse_user_connection_mode(value: &str) -> Result<String, ConnectionCommandError> {
    match value {
        "workflow" => Ok(CONNECTION_MODE_WORKFLOW.to_owned()),
        "read-only" => Ok(CONNECTION_MODE_READ_ONLY.to_owned()),
        other => Err(ConnectionCommandError::usage(format!(
            "unknown connection mode: {other}; use `workflow` or `read-only`"
        ))),
    }
}

fn value_text(name: &str, value: Option<&str>) -> Result<String, ConnectionCommandError> {
    let value = value
        .ok_or_else(|| ConnectionCommandError::usage(format!("missing value for --{name}")))?;
    if value.trim().is_empty() {
        Err(ConnectionCommandError::usage(format!(
            "--{name} must not be empty"
        )))
    } else {
        Ok(value.to_owned())
    }
}

fn value_path(name: &str, value: Option<&str>) -> Result<PathBuf, ConnectionCommandError> {
    Ok(PathBuf::from(value_text(name, value)?))
}

pub(super) fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}
