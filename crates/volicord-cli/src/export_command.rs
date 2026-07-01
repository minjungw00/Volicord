use std::{
    collections::BTreeSet,
    ffi::OsString,
    fmt, fs,
    path::{Path, PathBuf},
};

use serde_json::json;
use sha2::{Digest, Sha256};
use volicord_store::{
    agent_connections::{
        add_connection_project, ensure_agent_connection, list_agent_connections,
        AgentConnectionRecord, AgentConnectionRegistration, ConnectionProjectRegistration,
        CONNECTION_INTENT_PERSONAL, CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW,
        HOST_KIND_GENERIC, HOST_SCOPE_EXPORT, VERIFIED_STATUS_ACTION_REQUIRED,
    },
    bootstrap::{
        ensure_project_for_repo, initialize_runtime_home, installation_profile,
        InstallationProfileRecord, ProjectRecord, RepoProjectRegistration, ACTIVE_PROJECT_STATUS,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};

use crate::host_integration::{
    generic::{GenericAdapter, GenericExportRequest},
    HostAdapter, HostConfigError, HostKind, HostScope, InstallationProfile, PlannedChange,
};

const EXPORT_METADATA_CREATED_BY: &str = "volicord_cli_export_mcp_config";
const EXPORT_RUNTIME_HOME_ID: &str = "runtime_home_export";
const DEFAULT_EXPORT_FILE: &str = "volicord.mcp.json";
const DEFAULT_SERVER_NAME: &str = "volicord";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportCommandError {
    Usage(String),
    Runtime(String),
}

impl ExportCommandError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for ExportCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ExportCommandError {}

impl From<StoreError> for ExportCommandError {
    fn from(error: StoreError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for ExportCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<HostConfigError> for ExportCommandError {
    fn from(error: HostConfigError) -> Self {
        Self::runtime(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExportOptions {
    output: Option<PathBuf>,
    repo: Option<PathBuf>,
    read_only: bool,
    json: bool,
}

pub fn export_usage() -> String {
    "volicord export mcp-config [--output PATH] [--repo PATH] [--read-only] [--json]\n".to_owned()
}

pub fn run_export_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, ExportCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Ok(export_usage());
    };
    if matches!(subcommand, "-h" | "--help" | "help") {
        if args.len() == 1 {
            return Ok(export_usage());
        }
        return Err(ExportCommandError::usage(format!(
            "unexpected argument: {}\n\n{}",
            args[1],
            export_usage()
        )));
    }
    match subcommand {
        "mcp-config" => run_mcp_config_export(&args[1..], env_var, current_dir),
        other => Err(ExportCommandError::usage(format!(
            "unknown export command: {other}\n\n{}",
            export_usage()
        ))),
    }
}

fn run_mcp_config_export<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, ExportCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    if matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    ) {
        return Ok(export_usage());
    }
    let options = parse_export_options(args)?;
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let installation_profile = required_installation_profile(&runtime_home)?;
    initialize_runtime_home(
        &runtime_home,
        EXPORT_RUNTIME_HOME_ID,
        metadata_json_base()?.as_str(),
    )?;

    let repo_root = resolve_repository_root(current_dir, options.repo.as_deref())?;
    let output_path =
        resolve_export_output_path(current_dir, &repo_root, options.output.as_deref());
    let project = ensure_project_for_repo(
        &runtime_home,
        RepoProjectRegistration {
            project_name: None,
            project_alias: None,
            repo_root,
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: metadata_json_base()?,
        },
    )?;
    let config_target = path_text(&output_path);
    let existing = connection_for_export_target(&runtime_home, &config_target)?;
    let connection_internal_id = existing
        .as_ref()
        .map(|connection| connection.connection_internal_id.clone())
        .unwrap_or_else(|| deterministic_connection_id(&config_target));
    let expected_fingerprint = existing
        .as_ref()
        .map(|connection| connection.managed_fingerprint.as_str());
    let mode = if options.read_only {
        CONNECTION_MODE_READ_ONLY
    } else {
        CONNECTION_MODE_WORKFLOW
    };

    let adapter = GenericAdapter;
    let plan = adapter.plan_export(GenericExportRequest {
        connection_id: &connection_internal_id,
        installation_profile: installation_profile_context(&runtime_home, &installation_profile),
        mode,
        target_path: &output_path,
        expected_fingerprint,
    })?;
    if let Some(conflict) = plan.conflicts.first() {
        return Err(ExportCommandError::runtime(conflict.message.clone()));
    }

    let connection_status = connection_status(existing.as_ref(), &plan.fingerprint, mode);
    let metadata_json = connection_metadata_json(&output_path, &plan, &runtime_home)?;
    let connection = ensure_agent_connection(
        &runtime_home,
        AgentConnectionRegistration {
            connection_internal_id: connection_internal_id.clone(),
            host_kind: HOST_KIND_GENERIC.to_owned(),
            intent: CONNECTION_INTENT_PERSONAL.to_owned(),
            host_scope: HOST_SCOPE_EXPORT.to_owned(),
            server_name: plan.server_name.clone(),
            config_target: config_target.clone(),
            mode: mode.to_owned(),
            enabled: true,
            managed_fingerprint: plan.fingerprint.clone(),
            last_verification_status: VERIFIED_STATUS_ACTION_REQUIRED.to_owned(),
            last_verification_report_json: export_verification_report_json()?,
            last_user_actions_json: user_actions_json(&plan.user_actions)?,
            metadata_json,
        },
    )?;
    add_connection_project(
        &runtime_home,
        ConnectionProjectRegistration {
            connection_internal_id: connection.connection_internal_id.clone(),
            project_id: project.project_id.clone(),
        },
    )?;

    let mut adapter = GenericAdapter;
    adapter.apply(&plan)?;

    render_export_output(ExportRenderData {
        format: output_format(&options),
        output_path: &output_path,
        project: &project,
        mode,
        connection_status,
        connection: &connection,
        planned_change: plan.change,
        mcp_command: &plan.entry.command,
        mcp_args: &plan.entry.args,
        mcp_env: &plan.entry.env,
    })
}

fn parse_export_options(args: &[String]) -> Result<ExportOptions, ExportCommandError> {
    let mut options = ExportOptions::default();
    let mut seen = BTreeSet::new();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(ExportCommandError::usage(export_usage()));
        }
        if !token.starts_with("--") {
            return Err(ExportCommandError::usage(format!(
                "unexpected argument: {token}"
            )));
        }
        let without_prefix = &token[2..];
        let (name, value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name.to_owned(), Some(value.to_owned()))
        } else if is_boolean_option(without_prefix) {
            (without_prefix.to_owned(), None)
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(ExportCommandError::usage(format!(
                    "missing value for --{without_prefix}"
                )));
            };
            (without_prefix.to_owned(), Some(value.clone()))
        };
        if !matches!(name.as_str(), "output" | "repo" | "read-only" | "json") {
            return Err(ExportCommandError::usage(format!(
                "unknown option: --{name}"
            )));
        }
        if !seen.insert(name.clone()) {
            return Err(ExportCommandError::usage(format!(
                "duplicate option: --{name}"
            )));
        }
        match name.as_str() {
            "output" => options.output = Some(value_path(&name, value.as_deref())?),
            "repo" => options.repo = Some(value_path(&name, value.as_deref())?),
            "read-only" => {
                reject_boolean_value(&name, value.as_deref())?;
                options.read_only = true;
            }
            "json" => {
                reject_boolean_value(&name, value.as_deref())?;
                options.json = true;
            }
            _ => unreachable!("option name is checked before dispatch"),
        }
        index += 1;
    }
    Ok(options)
}

fn is_boolean_option(name: &str) -> bool {
    matches!(name, "read-only" | "json")
}

fn reject_boolean_value(name: &str, value: Option<&str>) -> Result<(), ExportCommandError> {
    if value.is_some() {
        Err(ExportCommandError::usage(format!(
            "--{name} does not accept a value"
        )))
    } else {
        Ok(())
    }
}

fn value_path(name: &str, value: Option<&str>) -> Result<PathBuf, ExportCommandError> {
    let value =
        value.ok_or_else(|| ExportCommandError::usage(format!("missing value for --{name}")))?;
    if value.trim().is_empty() {
        Err(ExportCommandError::usage(format!(
            "--{name} must not be empty"
        )))
    } else {
        Ok(PathBuf::from(value))
    }
}

fn output_format(options: &ExportOptions) -> OutputFormat {
    if options.json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    }
}

fn required_installation_profile(
    runtime_home: &Path,
) -> Result<InstallationProfileRecord, ExportCommandError> {
    installation_profile(runtime_home)?.ok_or_else(|| {
        ExportCommandError::runtime(format!(
            "SETUP_REQUIRED: installation profile is missing for Runtime Home {}; run `volicord init --host <host> --repo <path>` for the primary host setup. Use `volicord setup` only for installation-profile repair before export workflows.",
            runtime_home.display()
        ))
    })
}

fn installation_profile_context<'a>(
    runtime_home: &'a Path,
    profile: &'a InstallationProfileRecord,
) -> InstallationProfile<'a> {
    InstallationProfile {
        runtime_home,
        volicord_command: Path::new(&profile.volicord_command),
        volicord_mcp_command: Path::new(&profile.volicord_mcp_command),
        default_connection_mode: &profile.default_connection_mode,
    }
}

fn resolve_export_output_path(
    current_dir: &Path,
    repo_context: &Path,
    output: Option<&Path>,
) -> PathBuf {
    match output {
        Some(output) => absolute_path(current_dir, output.to_path_buf()),
        None => default_export_output_path(repo_context),
    }
}

fn default_export_output_path(repo_context: &Path) -> PathBuf {
    repo_context.join(DEFAULT_EXPORT_FILE)
}

fn resolve_repository_root(
    current_dir: &Path,
    selected_path: Option<&Path>,
) -> Result<PathBuf, ExportCommandError> {
    let selected = selected_path.unwrap_or(current_dir);
    let absolute = absolute_path(current_dir, selected.to_path_buf());
    let canonical = fs::canonicalize(&absolute).map_err(|error| {
        ExportCommandError::runtime(format!(
            "repository path is not accessible: {} ({error})",
            absolute.display()
        ))
    })?;
    let metadata = fs::metadata(&canonical).map_err(|error| {
        ExportCommandError::runtime(format!(
            "repository path is not accessible: {} ({error})",
            canonical.display()
        ))
    })?;
    let mut cursor = if metadata.is_file() {
        canonical
            .parent()
            .ok_or_else(|| {
                ExportCommandError::runtime(format!(
                    "repository path has no parent directory: {}",
                    canonical.display()
                ))
            })?
            .to_path_buf()
    } else {
        canonical
    };

    loop {
        let git_path = cursor.join(".git");
        match git_path.try_exists() {
            Ok(true) => return Ok(cursor),
            Ok(false) => {}
            Err(error) => {
                return Err(ExportCommandError::runtime(format!(
                    "failed to inspect Git repository marker {}: {error}",
                    git_path.display()
                )));
            }
        }
        if !cursor.pop() {
            break;
        }
    }

    Err(ExportCommandError::runtime(format!(
        "no Git repository root found from {}; run `volicord project use PATH` from inside a Git repository or pass --repo PATH",
        absolute.display()
    )))
}

fn connection_for_export_target(
    runtime_home: &Path,
    config_target: &str,
) -> Result<Option<AgentConnectionRecord>, ExportCommandError> {
    let matches = list_agent_connections(runtime_home)?
        .into_iter()
        .filter(|connection| {
            connection.host_kind == HOST_KIND_GENERIC
                && connection.intent == CONNECTION_INTENT_PERSONAL
                && connection.host_scope == HOST_SCOPE_EXPORT
                && connection.config_target == config_target
                && connection.server_name == DEFAULT_SERVER_NAME
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [connection] => Ok(Some(connection.clone())),
        _ => Err(ExportCommandError::runtime(format!(
            "export target matches multiple Agent Connections: {config_target}"
        ))),
    }
}

fn connection_status(
    existing: Option<&AgentConnectionRecord>,
    next_fingerprint: &str,
    mode: &str,
) -> &'static str {
    match existing {
        None => "created",
        Some(connection)
            if connection.mode == mode
                && connection.managed_fingerprint == next_fingerprint
                && connection.enabled =>
        {
            "reused"
        }
        Some(_) => "updated",
    }
}

struct ExportRenderData<'a> {
    format: OutputFormat,
    output_path: &'a Path,
    project: &'a ProjectRecord,
    mode: &'a str,
    connection_status: &'a str,
    connection: &'a AgentConnectionRecord,
    planned_change: PlannedChange,
    mcp_command: &'a str,
    mcp_args: &'a [String],
    mcp_env: &'a std::collections::BTreeMap<String, String>,
}

fn render_export_output(data: ExportRenderData<'_>) -> Result<String, ExportCommandError> {
    match data.format {
        OutputFormat::Text => Ok(format!(
            "MCP configuration exported\noutput: {}\nproject: {}\nrepo: {}\nmode: {}\nconnection: {}\nplanned_change: {}\n",
            path_text(data.output_path),
            data.project.project_name,
            path_text(&data.project.repo_root),
            public_mode_text(data.mode),
            data.connection_status,
            planned_change_text(data.planned_change)
        )),
        OutputFormat::Json => {
            let value = json!({
                "action": "exported",
                "status": "complete",
                "output_path": path_text(data.output_path),
                "project": {
                    "project_id": data.project.project_id,
                    "project_name": data.project.project_name,
                    "repo_root": path_text(&data.project.repo_root),
                },
                "mode": data.mode,
                "connection": {
                    "status": data.connection_status,
                    "connection_id": data.connection.connection_internal_id,
                    "host_kind": data.connection.host_kind,
                    "host_scope": data.connection.host_scope,
                    "verification_status": data.connection.last_verification_status,
                    "config_target": data.connection.config_target,
                },
                "planned_change": planned_change_text(data.planned_change),
                "mcp": {
                    "server_name": data.connection.server_name,
                    "command": data.mcp_command,
                    "args": data.mcp_args,
                    "env": data.mcp_env,
                },
            });
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| ExportCommandError::runtime(error.to_string()))
        }
    }
}

fn planned_change_text(change: PlannedChange) -> &'static str {
    match change {
        PlannedChange::Create => "create",
        PlannedChange::Update => "update",
        PlannedChange::Remove => "remove",
        PlannedChange::Noop => "noop",
        PlannedChange::ExternalCommand => "external_command",
    }
}

fn public_mode_text(mode: &str) -> &str {
    match mode {
        CONNECTION_MODE_READ_ONLY => "read-only",
        CONNECTION_MODE_WORKFLOW => "workflow",
        other => other,
    }
}

fn connection_metadata_json(
    output_path: &Path,
    plan: &crate::host_integration::HostPlan,
    runtime_home: &Path,
) -> Result<String, ExportCommandError> {
    serde_json::to_string(&json!({
        "created_by": EXPORT_METADATA_CREATED_BY,
        "mcp_command": plan.entry.command,
        "connection_intent": plan.connection_intent.as_str(),
        "mode": plan.mode,
        "host_runtime_home": path_text(runtime_home),
        "target_kind": "export",
        "target_path": path_text(output_path),
    }))
    .map_err(|error| ExportCommandError::runtime(error.to_string()))
}

fn export_verification_report_json() -> Result<String, ExportCommandError> {
    serde_json::to_string(&json!({
        "host": {
            "status": VERIFIED_STATUS_ACTION_REQUIRED,
            "details": "generic export is user-managed by the target host"
        }
    }))
    .map_err(|error| ExportCommandError::runtime(error.to_string()))
}

fn user_actions_json(
    actions: &[crate::host_integration::UserAction],
) -> Result<String, ExportCommandError> {
    serde_json::to_string(actions).map_err(|error| ExportCommandError::runtime(error.to_string()))
}

fn metadata_json_base() -> Result<String, ExportCommandError> {
    serde_json::to_string(&json!({ "created_by": EXPORT_METADATA_CREATED_BY }))
        .map_err(|error| ExportCommandError::runtime(error.to_string()))
}

fn deterministic_connection_id(config_target: &str) -> String {
    let key = json!({
        "host_kind": HostKind::Generic.as_str(),
        "host_scope": HostScope::Export.as_str(),
        "config_target": config_target,
        "server_name": DEFAULT_SERVER_NAME,
    })
    .to_string();
    let suffix = short_hash(&key);
    format!("conn_generic_export_volicord_{suffix}")
}

fn short_hash(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut text = String::new();
    for byte in digest.iter().take(6) {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}
