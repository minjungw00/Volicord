use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{json, Value};
use volicord_store::{
    agent_connections::{CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW},
    inspection::{
        inspect_runtime_home, DatabaseInspection, InspectionSchemaState,
        InstallationProfileInspectionRecord, RegistryInspectionSnapshot,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
};
use volicord_types::{GuardInstallationStatus, GuardMode};

use crate::{
    setup_command::{path_text, CommandOutcome, CommandStatus},
    shell_path::{
        detect_command_on_path, is_executable_file, mcp_binary_name, path_directory_is_on_path,
        paths_equivalent, volicord_binary_name, PATH_ENV,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorCommandError {
    Usage(String),
    Runtime(String),
}

impl std::fmt::Display for DoctorCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for DoctorCommandError {}

impl From<RuntimeHomeResolutionError> for DoctorCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DiagnosticCheck {
    id: String,
    status: String,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

impl DiagnosticCheck {
    fn passed(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "passed".to_owned(),
            summary: summary.into(),
            details: None,
        }
    }

    fn warning(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "warning".to_owned(),
            summary: summary.into(),
            details: None,
        }
    }

    fn skipped(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "skipped".to_owned(),
            summary: summary.into(),
            details: None,
        }
    }

    fn failed(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: "failed".to_owned(),
            summary: summary.into(),
            details: None,
        }
    }

    fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DiagnosticAction {
    id: String,
    instruction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
}

pub fn doctor_usage() -> String {
    "volicord doctor [--json]\n".to_owned()
}

pub fn run_doctor_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<CommandOutcome, DoctorCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    if is_help_request(args) {
        return Ok(CommandOutcome {
            status: CommandStatus::Complete,
            output: doctor_usage(),
        });
    }
    let output = parse_doctor_options(args)?;
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let mut checks = Vec::new();
    let mut actions = Vec::new();

    inspect_runtime_home_path(&runtime_home, &mut checks, &mut actions);
    let inspection = inspect_runtime_home(&runtime_home);
    let mut profile = None;
    let mut project_count = None;
    let mut connection_count = None;
    let mut guard_installation_count = None;

    match &inspection.registry {
        DatabaseInspection::Missing { path } => {
            checks.push(
                DiagnosticCheck::failed("registry", "Runtime Home registry is missing")
                    .with_details(json!({ "path": path_text(path) })),
            );
            actions.push(run_init_action());
        }
        DatabaseInspection::Present(snapshot) => {
            inspect_registry_snapshot(snapshot, &mut checks);
            profile = snapshot.installation_profile.as_ref();
            project_count = Some(snapshot.projects.len());
            connection_count = Some(snapshot.agent_connections.len());
            guard_installation_count = Some(snapshot.guard_installations.len());
            inspect_guard_installations(snapshot, &mut checks, &mut actions);
        }
        DatabaseInspection::Unsupported {
            path,
            detected_version,
            latest_supported_version,
            detail,
        } => {
            checks.push(
                DiagnosticCheck::failed(
                    "registry",
                    "Runtime Home registry uses an unsupported schema",
                )
                .with_details(json!({
                    "path": path_text(path),
                    "detected_version": detected_version,
                    "latest_supported_version": latest_supported_version,
                    "detail": detail,
                })),
            );
        }
        DatabaseInspection::Malformed { path, detail } => {
            checks.push(
                DiagnosticCheck::failed("registry", "Runtime Home registry is malformed")
                    .with_details(json!({ "path": path_text(path), "detail": detail })),
            );
        }
        DatabaseInspection::Unreadable { path, detail } => {
            checks.push(
                DiagnosticCheck::failed("registry", "Runtime Home registry is unreadable")
                    .with_details(json!({ "path": path_text(path), "detail": detail })),
            );
        }
    }

    if let Some(profile) = profile {
        inspect_installation_profile(profile, &env_var, &mut checks, &mut actions);
    } else {
        checks.push(
            DiagnosticCheck::failed("installation_profile", "installation profile is missing")
                .with_details(json!({ "runtime_home": path_text(&runtime_home) })),
        );
        if !actions.iter().any(|action| action.id == "run_init") {
            actions.push(run_init_action());
        }
        checks.push(DiagnosticCheck::skipped(
            "volicord_command",
            "volicord command check needs an installation profile",
        ));
        checks.push(DiagnosticCheck::skipped(
            "volicord_mcp_command",
            "MCP launch command check needs an installation profile",
        ));
        checks.push(DiagnosticCheck::skipped(
            "path_or_shim",
            "PATH and shim check needs an installation profile",
        ));
    }

    checks.push(
        DiagnosticCheck::skipped(
            "host_detection",
            "supported host detection is reported by init or connection verification",
        )
        .with_details(json!({ "supported_hosts": ["codex", "claude-code"] })),
    );
    if let (Some(projects), Some(connections), Some(guard_installations)) =
        (project_count, connection_count, guard_installation_count)
    {
        checks.push(
            DiagnosticCheck::passed("registry_counts", "registry records are readable")
                .with_details(json!({
                    "projects": projects,
                    "connections": connections,
                    "guard_installations": guard_installations,
                })),
        );
    } else {
        checks.push(DiagnosticCheck::skipped(
            "registry_counts",
            "project and connection counts are unavailable until the registry is readable",
        ));
    }

    let status = doctor_status(&checks);
    Ok(CommandOutcome {
        status,
        output: render_doctor_output(output, status, &runtime_home, &checks, &actions)?,
    })
}

fn parse_doctor_options(args: &[String]) -> Result<OutputFormat, DoctorCommandError> {
    let mut output = OutputFormat::Text;
    for token in args {
        match token.as_str() {
            "-h" | "--help" | "help" => return Err(DoctorCommandError::Usage(doctor_usage())),
            "--json" => output = OutputFormat::Json,
            option if option.starts_with("--json=") => {
                return Err(DoctorCommandError::Usage(
                    "--json does not accept a value".to_owned(),
                ))
            }
            option if option.starts_with('-') => {
                return Err(DoctorCommandError::Usage(format!(
                    "unknown option: {option}"
                )))
            }
            argument => {
                return Err(DoctorCommandError::Usage(format!(
                    "unexpected argument: {argument}"
                )))
            }
        }
    }
    Ok(output)
}

fn inspect_runtime_home_path(
    runtime_home: &Path,
    checks: &mut Vec<DiagnosticCheck>,
    actions: &mut Vec<DiagnosticAction>,
) {
    match fs::metadata(runtime_home) {
        Ok(metadata) if metadata.is_dir() => checks.push(
            DiagnosticCheck::passed(
                "runtime_home_access",
                "Runtime Home directory is accessible",
            )
            .with_details(json!({ "path": path_text(runtime_home) })),
        ),
        Ok(_) => {
            checks.push(
                DiagnosticCheck::failed(
                    "runtime_home_access",
                    "Runtime Home path is not a directory",
                )
                .with_details(json!({ "path": path_text(runtime_home) })),
            );
            actions.push(run_init_action());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            checks.push(
                DiagnosticCheck::failed("runtime_home_access", "Runtime Home directory is missing")
                    .with_details(json!({ "path": path_text(runtime_home) })),
            );
            actions.push(run_init_action());
        }
        Err(error) => {
            checks.push(
                DiagnosticCheck::failed(
                    "runtime_home_access",
                    "Runtime Home directory is not accessible",
                )
                .with_details(
                    json!({ "path": path_text(runtime_home), "detail": error.to_string() }),
                ),
            );
        }
    }
}

fn inspect_registry_snapshot(
    snapshot: &RegistryInspectionSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
) {
    match snapshot.schema {
        InspectionSchemaState::Current { version } => checks.push(
            DiagnosticCheck::passed("registry_schema", "Runtime Home registry schema is current")
                .with_details(json!({
                    "path": path_text(&snapshot.path),
                    "version": version,
                    "storage_profile": snapshot.runtime_home.storage_profile,
                })),
        ),
        InspectionSchemaState::MigrationRequired {
            detected_version,
            latest_supported_version,
        } => checks.push(
            DiagnosticCheck::failed("registry_schema", "Runtime Home registry needs migration")
                .with_details(json!({
                    "path": path_text(&snapshot.path),
                    "detected_version": detected_version,
                    "latest_supported_version": latest_supported_version,
                })),
        ),
    }
}

fn inspect_guard_installations(
    snapshot: &RegistryInspectionSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    actions: &mut Vec<DiagnosticAction>,
) {
    if snapshot.guard_installations.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "guard_files_installed",
            "no guard installations are recorded",
        ));
        checks.push(DiagnosticCheck::skipped(
            "guard_host_reload_required",
            "no guard installation needs host reload",
        ));
        checks.push(DiagnosticCheck::skipped(
            "guard_hook_observed",
            "no guard hook observation is recorded",
        ));
        checks.push(DiagnosticCheck::skipped(
            "guard_status_active",
            "no guard installation status is recorded",
        ));
        checks.push(DiagnosticCheck::skipped(
            "prompt_capture_available",
            "no prompt capture availability is recorded",
        ));
        return;
    }

    let guarded = snapshot
        .guard_installations
        .iter()
        .filter(|installation| installation.guard_mode != GuardMode::McpOnly.as_str())
        .collect::<Vec<_>>();
    let missing_files = snapshot
        .guard_installations
        .iter()
        .flat_map(|installation| guard_missing_files(&installation.host_capability_json))
        .collect::<Vec<_>>();
    if missing_files.is_empty() {
        checks.push(
            DiagnosticCheck::passed("guard_files_installed", "guard files are installed")
                .with_details(json!({ "guard_installations": snapshot.guard_installations.len() })),
        );
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "guard_files_installed",
                "one or more guard files are missing",
            )
            .with_details(json!({ "missing_files": missing_files })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_guard_files".to_owned(),
                instruction:
                    "Run volicord init again for affected guarded projects to reinstall missing guard files."
                        .to_owned(),
                command: Some("volicord init --host HOST --repo PATH".to_owned()),
            },
        );
    }

    let reload_required = guarded.iter().any(|installation| {
        installation.installation_status == GuardInstallationStatus::ReloadRequired.as_str()
    });
    if reload_required {
        checks.push(
            DiagnosticCheck::warning(
                "guard_host_reload_required",
                "one or more guard installations need host reload",
            )
            .with_details(json!({ "reload_required": true })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "reload_guard_host".to_owned(),
                instruction:
                    "Restart or reload affected agent hosts so they load the Volicord guard configuration."
                        .to_owned(),
                command: None,
            },
        );
    } else {
        checks.push(DiagnosticCheck::passed(
            "guard_host_reload_required",
            "no recorded guard installation requires host reload",
        ));
    }

    let observed_count = guarded
        .iter()
        .filter(|installation| installation.last_seen_at.is_some())
        .count();
    if guarded.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "guard_hook_observed",
            "guard hook observation is not applicable to mcp-only installations",
        ));
    } else if observed_count == guarded.len() {
        checks.push(
            DiagnosticCheck::passed("guard_hook_observed", "guard hooks have been observed")
                .with_details(json!({ "observed": observed_count, "guarded": guarded.len() })),
        );
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "guard_hook_observed",
                "one or more guarded installations have not been observed",
            )
            .with_details(json!({ "observed": observed_count, "guarded": guarded.len() })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "observe_guard_hook".to_owned(),
                instruction:
                    "Start, restart, or reload affected agent hosts so the Volicord guard hook runs."
                        .to_owned(),
                command: None,
            },
        );
    }

    let status_counts = guard_status_counts(&snapshot.guard_installations);
    let problem_status = ["broken", "stale", "degraded"].iter().find(|status| {
        status_counts
            .get(**status)
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    });
    if let Some(status) = problem_status {
        checks.push(
            DiagnosticCheck::warning(
                "guard_status_active",
                format!("one or more guard installations are {status}"),
            )
            .with_details(json!({ "status_counts": status_counts })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_guard_status".to_owned(),
                instruction:
                    "Repair or reinstall affected guard integrations before relying on guarded close readiness."
                        .to_owned(),
                command: Some("volicord init --host HOST --repo PATH".to_owned()),
            },
        );
    } else if guarded.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "guard_status_active",
            "guard active status is not applicable to mcp-only installations",
        ));
    } else if guarded.iter().all(|installation| {
        installation.installation_status == GuardInstallationStatus::Active.as_str()
            && installation.last_seen_at.is_some()
    }) {
        checks.push(
            DiagnosticCheck::passed("guard_status_active", "guard status is active and observed")
                .with_details(json!({ "status_counts": status_counts })),
        );
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "guard_status_active",
                "guard status is not active for one or more guarded installations",
            )
            .with_details(json!({ "status_counts": status_counts })),
        );
    }

    inspect_prompt_capture_availability(&guarded, checks);
}

fn guard_missing_files(capability_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(capability_json) else {
        return Vec::new();
    };
    value
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|file| file.get("path").and_then(Value::as_str))
        .filter(|path| !Path::new(path).exists())
        .map(str::to_owned)
        .collect()
}

fn inspect_prompt_capture_availability(
    guarded: &[&volicord_store::inspection::GuardInstallationInspectionRecord],
    checks: &mut Vec<DiagnosticCheck>,
) {
    if guarded.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "prompt_capture_available",
            "prompt capture is not applicable to mcp-only installations",
        ));
        return;
    }
    let configured = guarded
        .iter()
        .filter(|installation| guard_prompt_capture_configured(&installation.host_capability_json))
        .count();
    let observed = guarded
        .iter()
        .filter(|installation| {
            installation.last_seen_at.is_some()
                && guard_prompt_capture_configured(&installation.host_capability_json)
        })
        .count();
    if observed > 0 {
        checks.push(
            DiagnosticCheck::passed("prompt_capture_available", "prompt capture is available")
                .with_details(json!({ "configured": configured, "observed": observed })),
        );
    } else if configured > 0 {
        checks.push(
            DiagnosticCheck::warning(
                "prompt_capture_available",
                "prompt capture is configured but no guard hook observation is recorded",
            )
            .with_details(json!({ "configured": configured, "observed": observed })),
        );
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "prompt_capture_available",
                "prompt capture is not configured for recorded guarded installations",
            )
            .with_details(json!({ "configured": configured, "observed": observed })),
        );
    }
}

fn guard_prompt_capture_configured(capability_json: &str) -> bool {
    serde_json::from_str::<Value>(capability_json)
        .ok()
        .and_then(|value| value.get("prompt_capture").and_then(Value::as_bool))
        .unwrap_or(false)
}

fn guard_status_counts(
    installations: &[volicord_store::inspection::GuardInstallationInspectionRecord],
) -> serde_json::Map<String, Value> {
    let mut counts = serde_json::Map::new();
    for installation in installations {
        let count = counts
            .get(&installation.installation_status)
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1;
        counts.insert(installation.installation_status.clone(), json!(count));
    }
    counts
}

fn inspect_installation_profile<F>(
    profile: &InstallationProfileInspectionRecord,
    env_var: &F,
    checks: &mut Vec<DiagnosticCheck>,
    actions: &mut Vec<DiagnosticAction>,
) where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let mode_supported = matches!(
        profile.default_connection_mode.as_str(),
        CONNECTION_MODE_WORKFLOW | CONNECTION_MODE_READ_ONLY
    );
    if mode_supported {
        checks.push(
            DiagnosticCheck::passed("installation_profile", "installation profile is present")
                .with_details(json!({
                    "installation_id": profile.installation_id,
                    "default_connection_mode": profile.default_connection_mode,
                    "bin_dir": path_text(&profile.bin_dir),
                })),
        );
    } else {
        checks.push(
            DiagnosticCheck::failed(
                "installation_profile",
                "installation profile has an unsupported default connection mode",
            )
            .with_details(json!({
                "installation_id": profile.installation_id,
                "default_connection_mode": profile.default_connection_mode,
            })),
        );
        actions.push(run_init_action());
    }
    inspect_command_path(
        "volicord_command",
        "volicord command",
        &PathBuf::from(&profile.volicord_command),
        checks,
        actions,
    );
    inspect_command_path(
        "volicord_mcp_command",
        "MCP launch command",
        &PathBuf::from(&profile.volicord_mcp_command),
        checks,
        actions,
    );
    let path_env = env_var(PATH_ENV);
    inspect_command_availability(
        "volicord_command_availability",
        &volicord_binary_name(),
        &PathBuf::from(&profile.volicord_command),
        path_env.as_deref(),
        checks,
        actions,
    );
    inspect_command_availability(
        "volicord_mcp_command_availability",
        &mcp_binary_name(),
        &PathBuf::from(&profile.volicord_mcp_command),
        path_env.as_deref(),
        checks,
        actions,
    );
    inspect_path_or_shim(profile, path_env.as_deref(), checks, actions);
}

fn inspect_command_path(
    id: &str,
    label: &str,
    command: &Path,
    checks: &mut Vec<DiagnosticCheck>,
    actions: &mut Vec<DiagnosticAction>,
) {
    if is_executable_file(command) {
        checks.push(
            DiagnosticCheck::passed(id, format!("{label} is executable"))
                .with_details(json!({ "path": path_text(command) })),
        );
    } else {
        checks.push(
            DiagnosticCheck::failed(id, format!("{label} is missing or not executable"))
                .with_details(json!({ "path": path_text(command) })),
        );
        actions.push(DiagnosticAction {
            id: format!("repair_{id}"),
            instruction:
                "Run volicord init --host <host> --repo <path> --mcp-command PATH after selecting an executable MCP launch command."
                    .to_owned(),
            command: Some("volicord init --host <host> --repo <path> --mcp-command PATH".to_owned()),
        });
    }
}

fn inspect_command_availability(
    id: &str,
    command_name: &str,
    profile_command: &Path,
    path_env: Option<&OsStr>,
    checks: &mut Vec<DiagnosticCheck>,
    actions: &mut Vec<DiagnosticAction>,
) {
    let path_match = detect_command_on_path(command_name, path_env);
    let profile_command_directory_on_path = profile_command
        .parent()
        .is_some_and(|directory| path_directory_is_on_path(path_env, directory));
    let path_matches_profile = path_match
        .as_deref()
        .is_some_and(|path| paths_equivalent(path, profile_command));
    let details = json!({
        "command_name": command_name,
        "profile_command": path_text(profile_command),
        "available_on_path": path_match.is_some(),
        "path_matches_profile": path_matches_profile,
        "profile_command_directory_on_path": profile_command_directory_on_path,
        "path_match": path_match.as_deref().map(path_text),
        "agent_host_restart_or_reload_may_be_needed": !path_matches_profile,
    });

    if path_matches_profile {
        checks.push(
            DiagnosticCheck::passed(
                id,
                format!("{command_name} resolves to the installation profile command on PATH"),
            )
            .with_details(details),
        );
    } else if path_match.is_some() {
        checks.push(
            DiagnosticCheck::warning(
                id,
                format!("{command_name} resolves to a different executable on PATH"),
            )
            .with_details(details),
        );
        push_command_availability_action(actions);
    } else {
        checks.push(
            DiagnosticCheck::warning(id, format!("{command_name} is not available on PATH"))
                .with_details(details),
        );
        push_command_availability_action(actions);
    }
}

fn inspect_path_or_shim(
    profile: &InstallationProfileInspectionRecord,
    path_env: Option<&OsStr>,
    checks: &mut Vec<DiagnosticCheck>,
    actions: &mut Vec<DiagnosticAction>,
) {
    let bin_dir_on_path = path_directory_is_on_path(path_env, &profile.bin_dir);
    let volicord_link = profile.bin_dir.join(volicord_binary_name());
    let mcp_link = profile.bin_dir.join(mcp_binary_name());
    let link_ready = is_executable_file(&volicord_link) && is_executable_file(&mcp_link);

    if bin_dir_on_path && link_ready {
        checks.push(
            DiagnosticCheck::passed(
                "path_or_shim",
                "profile command directory is on PATH and contains command links",
            )
            .with_details(json!({
                "bin_dir": path_text(&profile.bin_dir),
                "volicord": path_text(&volicord_link),
                "volicord_mcp": path_text(&mcp_link),
                "agent_host_restart_or_reload_may_be_needed": false,
            })),
        );
    } else if bin_dir_on_path {
        checks.push(
            DiagnosticCheck::warning(
                "path_or_shim",
                "profile command directory is on PATH, but command links are incomplete",
            )
            .with_details(json!({
                "bin_dir": path_text(&profile.bin_dir),
                "volicord_link_ready": is_executable_file(&volicord_link),
                "volicord_mcp_link_ready": is_executable_file(&mcp_link),
                "agent_host_restart_or_reload_may_be_needed": true,
            })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_command_links".to_owned(),
                instruction: format!(
                    "Use advanced repair command volicord setup --link-bin {} to repair command links; restart or reload existing agent hosts after command-link changes.",
                    profile.bin_dir.display()
                ),
                command: Some(format!(
                    "volicord setup --link-bin {}",
                    profile.bin_dir.display()
                )),
            },
        );
    } else if link_ready {
        checks.push(
            DiagnosticCheck::warning(
                "path_or_shim",
                "command links exist, but the link directory is not on PATH",
            )
            .with_details(json!({
                "bin_dir": path_text(&profile.bin_dir),
                "agent_host_restart_or_reload_may_be_needed": true,
            })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "add_link_bin_to_path".to_owned(),
                instruction: format!(
                    "Add {} to PATH before starting new shells or agent hosts; restart or reload existing agent hosts after the PATH change.",
                    profile.bin_dir.display()
                ),
                command: Some(format!(
                    "export PATH=\"{}:$PATH\"",
                    profile.bin_dir.display()
                )),
            },
        );
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "path_or_shim",
                "no command link directory is active for this shell",
            )
            .with_details(json!({
                "bin_dir": path_text(&profile.bin_dir),
                "agent_host_restart_or_reload_may_be_needed": true,
            })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "create_command_links".to_owned(),
                instruction:
                    "Use advanced repair command volicord setup --link-bin PATH for a command-link directory you keep on PATH; restart or reload existing agent hosts after PATH or command-link changes."
                        .to_owned(),
                command: Some("volicord setup --link-bin PATH".to_owned()),
            },
        );
    }
}

fn doctor_status(checks: &[DiagnosticCheck]) -> CommandStatus {
    if checks.iter().any(|check| {
        check.status == "failed"
            && !matches!(
                check.id.as_str(),
                "runtime_home_access" | "registry" | "installation_profile"
            )
    }) {
        CommandStatus::Failed
    } else if checks.iter().any(|check| check.status == "failed") {
        CommandStatus::ActionRequired
    } else {
        CommandStatus::Complete
    }
}

fn render_doctor_output(
    output: OutputFormat,
    status: CommandStatus,
    runtime_home: &Path,
    checks: &[DiagnosticCheck],
    actions: &[DiagnosticAction],
) -> Result<String, DoctorCommandError> {
    match output {
        OutputFormat::Json => {
            let actions_required = if status == CommandStatus::Complete {
                Vec::new()
            } else {
                actions.iter().collect::<Vec<_>>()
            };
            let actions_recommended = if status == CommandStatus::Complete {
                actions.iter().collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            serde_json::to_string_pretty(&json!({
                "status": status.as_str(),
                "status_meaning": doctor_status_meaning(status, checks),
                "runtime_home": path_text(runtime_home),
                "states": doctor_states_json(runtime_home, checks, actions),
                "checks": checks,
                "warning_count": checks.iter().filter(|check| check.status == "warning").count(),
                "actions": actions,
                "actions_required": actions_required,
                "actions_recommended": actions_recommended,
                "primary_next_action": primary_doctor_action_json(status, actions),
            }))
            .map(|text| format!("{text}\n"))
            .map_err(|error| DoctorCommandError::Runtime(error.to_string()))
        }
        OutputFormat::Text => {
            let mut text = format!(
                "Volicord doctor {}\nstatus_meaning: {}\nruntime_home_state: {}\nruntime_home: {}\ninstallation_profile_state: {}\ncommand_state: {}\nproject_registration_state: {}\nconnection_state: {}\nmcp_config_state: {}\nguard_installation_state: {}\nguard_files_state: {}\nguard_hook_observed: {}\nguard_status_state: {}\nprompt_capture_state: {}\nhost_reload_required: {}\n",
                status.as_str(),
                doctor_status_meaning(status, checks),
                doctor_runtime_home_state(runtime_home, checks),
                runtime_home.display(),
                doctor_installation_profile_state(checks),
                doctor_command_state(checks),
                doctor_count_state(checks, "projects", "registered"),
                doctor_count_state(checks, "connections", "stored"),
                doctor_mcp_config_state(checks),
                doctor_count_state(checks, "guard_installations", "stored"),
                doctor_check_state(checks, "guard_files_installed"),
                doctor_check_state(checks, "guard_hook_observed"),
                doctor_check_state(checks, "guard_status_active"),
                doctor_prompt_capture_state(checks),
                yes_no(doctor_host_reload_required(checks, actions)),
            );
            append_doctor_next_action(&mut text, status, actions);
            Ok(text)
        }
    }
}

fn doctor_states_json(
    runtime_home: &Path,
    checks: &[DiagnosticCheck],
    actions: &[DiagnosticAction],
) -> Value {
    json!({
        "runtime_home": doctor_runtime_home_state(runtime_home, checks),
        "installation_profile": doctor_installation_profile_state(checks),
        "command_availability": doctor_command_state(checks),
        "project_registration": doctor_count_state(checks, "projects", "registered"),
        "connection": doctor_count_state(checks, "connections", "stored"),
        "mcp_config": doctor_mcp_config_state(checks),
        "guard_installation": doctor_count_state(checks, "guard_installations", "stored"),
        "guard_files": doctor_check_state(checks, "guard_files_installed"),
        "guard_hook_observed": doctor_check_state(checks, "guard_hook_observed"),
        "guard_status": doctor_check_state(checks, "guard_status_active"),
        "prompt_capture": doctor_prompt_capture_state(checks),
        "host_reload_required": doctor_host_reload_required(checks, actions),
    })
}

fn doctor_runtime_home_state(runtime_home: &Path, checks: &[DiagnosticCheck]) -> String {
    if !runtime_home.exists() {
        return "missing".to_owned();
    }
    match check_status(checks, "runtime_home_access") {
        Some("passed") => "ready".to_owned(),
        Some("failed") => "not_accessible".to_owned(),
        _ => "unknown".to_owned(),
    }
}

fn doctor_installation_profile_state(checks: &[DiagnosticCheck]) -> &'static str {
    match check_status(checks, "installation_profile") {
        Some("passed") => "present",
        Some("failed") => "missing_or_invalid",
        Some("skipped") => "not_checked",
        _ => "unknown",
    }
}

fn doctor_command_state(checks: &[DiagnosticCheck]) -> &'static str {
    if checks.iter().any(|check| {
        matches!(
            check.id.as_str(),
            "volicord_command" | "volicord_mcp_command"
        ) && check.status == "failed"
    }) {
        "not_found"
    } else if checks.iter().any(|check| {
        matches!(
            check.id.as_str(),
            "volicord_command_availability" | "volicord_mcp_command_availability" | "path_or_shim"
        ) && check.status == "warning"
    }) {
        "action_recommended"
    } else if checks.iter().any(|check| {
        matches!(
            check.id.as_str(),
            "volicord_command_availability" | "volicord_mcp_command_availability" | "path_or_shim"
        ) && check.status == "skipped"
    }) {
        "not_checked"
    } else {
        "ready"
    }
}

fn doctor_check_state(checks: &[DiagnosticCheck], id: &str) -> &'static str {
    match check_status(checks, id) {
        Some("passed") => "ready",
        Some("warning") => "action_recommended",
        Some("failed") => "failed",
        Some("skipped") => "not_checked",
        _ => "unknown",
    }
}

fn doctor_prompt_capture_state(checks: &[DiagnosticCheck]) -> &'static str {
    if check_status(checks, "prompt_capture_available").is_none() {
        "not_checked"
    } else {
        doctor_check_state(checks, "prompt_capture_available")
    }
}

fn doctor_count_state(checks: &[DiagnosticCheck], key: &str, suffix: &str) -> String {
    checks
        .iter()
        .find(|check| check.id == "registry_counts")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get(key))
        .and_then(Value::as_u64)
        .map(|count| format!("{count} {suffix}"))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn doctor_mcp_config_state(checks: &[DiagnosticCheck]) -> String {
    checks
        .iter()
        .find(|check| check.id == "registry_counts")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("connections"))
        .and_then(Value::as_u64)
        .map(|count| {
            if count == 0 {
                "not_configured".to_owned()
            } else {
                format!("{count} stored")
            }
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn doctor_host_reload_required(checks: &[DiagnosticCheck], actions: &[DiagnosticAction]) -> bool {
    actions.iter().any(|action| {
        action
            .instruction
            .to_ascii_lowercase()
            .contains("restart or reload")
    }) || checks.iter().any(|check| {
        check
            .details
            .as_ref()
            .and_then(|details| details.get("agent_host_restart_or_reload_may_be_needed"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

fn check_status<'a>(checks: &'a [DiagnosticCheck], id: &str) -> Option<&'a str> {
    checks
        .iter()
        .find(|check| check.id == id)
        .map(|check| check.status.as_str())
}

fn primary_doctor_action_json(status: CommandStatus, actions: &[DiagnosticAction]) -> Value {
    let Some(action) = actions.first() else {
        return Value::Null;
    };
    let requirement = if status == CommandStatus::Complete {
        "recommended"
    } else {
        "required"
    };
    json!({
        "id": &action.id,
        "requirement": requirement,
        "instruction": &action.instruction,
        "command": &action.command,
    })
}

fn append_doctor_next_action(
    output: &mut String,
    status: CommandStatus,
    actions: &[DiagnosticAction],
) {
    match actions.first() {
        Some(action) if status == CommandStatus::Complete => {
            output.push_str(&format!(
                "next_action: recommended: {}\n",
                action.instruction
            ));
        }
        Some(action) => output.push_str(&format!("next_action: {}\n", action.instruction)),
        None => output.push_str("next_action: none\n"),
    }
}

fn doctor_status_meaning(status: CommandStatus, checks: &[DiagnosticCheck]) -> &'static str {
    match status {
        CommandStatus::Complete if checks.iter().any(|check| check.status == "warning") => {
            "installation profile is usable; warnings name recommended follow-up actions"
        }
        CommandStatus::Complete => "installation profile is usable",
        CommandStatus::ActionRequired => {
            "local init or profile repair is required before Volicord workflows are usable"
        }
        CommandStatus::Failed => "a blocking diagnostic failed before the profile is usable",
    }
}

fn run_init_action() -> DiagnosticAction {
    DiagnosticAction {
        id: "run_init".to_owned(),
        instruction:
            "Run volicord init --host <host> --repo <path> from the Product Repository to initialize the primary host connection."
                .to_owned(),
        command: Some("volicord init --host <host> --repo <path>".to_owned()),
    }
}

fn push_command_availability_action(actions: &mut Vec<DiagnosticAction>) {
    push_unique_diagnostic_action(
        actions,
        DiagnosticAction {
            id: "make_profile_commands_available".to_owned(),
        instruction:
                "Use advanced repair command volicord setup --link-bin PATH or update PATH so volicord resolves to the installation profile command; restart or reload existing agent hosts after PATH or command-link changes."
                    .to_owned(),
            command: Some("volicord setup --link-bin PATH".to_owned()),
        },
    );
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn push_unique_diagnostic_action(actions: &mut Vec<DiagnosticAction>, action: DiagnosticAction) {
    if !actions.iter().any(|existing| existing.id == action.id) {
        actions.push(action);
    }
}

fn is_help_request(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    )
}
