use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
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

const REQUIRED_GUARD_HOOK_PHASES: &[&str] = &[
    "session_start_hook",
    "pre_tool_hook",
    "post_tool_hook",
    "user_prompt_submit_hook",
    "stop_hook",
];

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
            "guard_required_hooks_supported",
            "no guard hook capability record is available",
        ));
        checks.push(DiagnosticCheck::skipped(
            "guard_status_active",
            "no guard installation status is recorded",
        ));
        checks.push(
            DiagnosticCheck::skipped(
                "prompt_capture_available",
                "no prompt capture availability is recorded",
            )
            .with_details(json!({
                "state": "not_recorded",
                "configured": 0,
                "observed": 0,
            })),
        );
        return;
    }

    let guarded = snapshot
        .guard_installations
        .iter()
        .filter(|installation| installation.guard_mode != GuardMode::McpOnly.as_str())
        .collect::<Vec<_>>();
    let mut file_findings = DoctorGuardFileFindings::default();
    for installation in &snapshot.guard_installations {
        file_findings.merge(doctor_guard_file_findings(
            &installation.host_capability_json,
        ));
    }
    file_findings.sort_dedup();
    let guard_file_problem = !file_findings.missing_files.is_empty()
        || !file_findings.stale_files.is_empty()
        || !file_findings.broken_files.is_empty();
    if !guard_file_problem {
        checks.push(
            DiagnosticCheck::passed("guard_files_installed", "guard files are installed")
                .with_details(doctor_guard_file_details(&file_findings)),
        );
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "guard_files_installed",
                "one or more guard files are missing, stale, or broken",
            )
            .with_details(doctor_guard_file_details(&file_findings)),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_guard_files".to_owned(),
                instruction:
                    "Run volicord init again for affected guarded projects to reinstall or refresh guard files."
                        .to_owned(),
                command: Some("volicord init --host HOST --repo PATH".to_owned()),
            },
        );
    }

    let missing_required_hooks = guarded
        .iter()
        .flat_map(|installation| guard_missing_required_hooks(&installation.host_capability_json))
        .collect::<Vec<_>>();

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

    if guarded.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "guard_required_hooks_supported",
            "guard hook capability is not applicable to mcp-only installations",
        ));
    } else if missing_required_hooks.is_empty() {
        checks.push(DiagnosticCheck::passed(
            "guard_required_hooks_supported",
            "required guard hook capabilities are recorded",
        ));
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "guard_required_hooks_supported",
                "one or more guarded installations are missing required hook capabilities",
            )
            .with_details(json!({ "missing_required_hooks": missing_required_hooks })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_guard_required_hooks".to_owned(),
                instruction:
                    "Run volicord init again with a host adapter that supports every required guard hook, or use mcp-only mode."
                        .to_owned(),
                command: Some("volicord init --host HOST --repo PATH".to_owned()),
            },
        );
    }

    let observed_count = guarded
        .iter()
        .filter(|installation| guard_observation_current(installation))
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
    } else if guarded
        .iter()
        .all(|installation| guard_effective_active(installation))
    {
        checks.push(
            DiagnosticCheck::passed("guard_status_active", "effective guard status is active")
                .with_details(json!({ "status_counts": status_counts })),
        );
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "guard_status_active",
                "effective guard status is not active for one or more guarded installations",
            )
            .with_details(json!({
                "status_counts": status_counts,
                "effective_active": guarded.iter().filter(|installation| guard_effective_active(installation)).count(),
                "guarded": guarded.len(),
            })),
        );
    }

    inspect_prompt_capture_availability(&guarded, checks);
}

#[derive(Debug, Default)]
struct DoctorGuardFileFindings {
    missing_files: Vec<String>,
    stale_files: Vec<String>,
    broken_files: Vec<String>,
    file_states: BTreeMap<String, String>,
}

impl DoctorGuardFileFindings {
    fn merge(&mut self, other: Self) {
        self.missing_files.extend(other.missing_files);
        self.stale_files.extend(other.stale_files);
        self.broken_files.extend(other.broken_files);
        for (kind, state) in other.file_states {
            self.set_file_state(&kind, &state);
        }
    }

    fn sort_dedup(&mut self) {
        self.missing_files.sort();
        self.missing_files.dedup();
        self.stale_files.sort();
        self.stale_files.dedup();
        self.broken_files.sort();
        self.broken_files.dedup();
    }

    fn set_file_state(&mut self, kind: &str, state: &str) {
        let update = self
            .file_states
            .get(kind)
            .is_none_or(|current| doctor_file_state_rank(state) > doctor_file_state_rank(current));
        if update {
            self.file_states.insert(kind.to_owned(), state.to_owned());
        }
    }
}

fn doctor_file_state_rank(value: &str) -> u8 {
    match value {
        "broken" => 5,
        "missing" => 4,
        "stale" => 3,
        "installed" => 2,
        "missing_required_hooks" | "unsupported_by_host" | "not_recorded" => 1,
        _ => 0,
    }
}

fn doctor_guard_file_details(findings: &DoctorGuardFileFindings) -> Value {
    json!({
        "missing_files": &findings.missing_files,
        "stale_files": &findings.stale_files,
        "broken_files": &findings.broken_files,
        "file_states": &findings.file_states,
    })
}

fn doctor_guard_file_findings(capability_json: &str) -> DoctorGuardFileFindings {
    let mut findings = DoctorGuardFileFindings::default();
    let Ok(value) = serde_json::from_str::<Value>(capability_json) else {
        findings
            .broken_files
            .push("guard_capability_json".to_owned());
        return findings;
    };
    if value
        .get("host_capabilities")
        .and_then(|capabilities| capabilities.get("rule_file_support"))
        .and_then(Value::as_bool)
        == Some(false)
    {
        findings.set_file_state("host_rule_instruction", "unsupported_by_host");
    }
    if guard_missing_required_hooks(capability_json).is_empty() {
        findings.set_file_state("host_hook_config", "not_recorded");
    } else {
        findings.set_file_state("host_hook_config", "missing_required_hooks");
    }
    value
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .for_each(|file| doctor_verify_guard_file(file, &mut findings));
    findings
}

fn doctor_verify_guard_file(file: &Value, findings: &mut DoctorGuardFileFindings) {
    let kind = file
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let Some(path_text) = file.get("path").and_then(Value::as_str) else {
        findings
            .broken_files
            .push("guard_capability_json:files.path".to_owned());
        findings.set_file_state(kind, "broken");
        return;
    };
    let text = match fs::read_to_string(path_text) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            findings.missing_files.push(path_text.to_owned());
            findings.set_file_state(kind, "missing");
            return;
        }
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            findings.set_file_state(kind, "broken");
            return;
        }
    };
    let expected_hash = file
        .get("content_hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match file.get("ownership").and_then(Value::as_str) {
        Some("managed_block") => {
            doctor_verify_managed_block(file, kind, path_text, &text, expected_hash, findings)
        }
        Some("managed_json") => {
            if sha256_text(&text) == expected_hash {
                findings.set_file_state(kind, "installed");
            } else {
                findings.stale_files.push(path_text.to_owned());
                findings.set_file_state(kind, "stale");
            }
        }
        Some("managed_json_projection") => doctor_verify_managed_json_projection(
            file,
            kind,
            path_text,
            &text,
            expected_hash,
            findings,
        ),
        _ => {
            findings.broken_files.push(path_text.to_owned());
            findings.set_file_state(kind, "broken");
        }
    }
}

fn doctor_verify_managed_json_projection(
    file: &Value,
    kind: &str,
    path_text: &str,
    text: &str,
    expected_hash: &str,
    findings: &mut DoctorGuardFileFindings,
) {
    let Some(expected_projection_json) =
        file.get("managed_projection_json").and_then(Value::as_str)
    else {
        findings.broken_files.push(path_text.to_owned());
        findings.set_file_state(kind, "broken");
        return;
    };
    if sha256_text(expected_projection_json) != expected_hash {
        findings.broken_files.push(path_text.to_owned());
        findings.set_file_state(kind, "broken");
        return;
    }
    let actual = match serde_json::from_str::<Value>(text) {
        Ok(actual) => actual,
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            findings.set_file_state(kind, "broken");
            return;
        }
    };
    let desired = match serde_json::from_str::<Value>(expected_projection_json) {
        Ok(desired) => desired,
        Err(_) => {
            findings.broken_files.push(path_text.to_owned());
            findings.set_file_state(kind, "broken");
            return;
        }
    };
    if managed_projection_present(&actual, &desired) {
        findings.set_file_state(kind, "installed");
    } else {
        findings.stale_files.push(path_text.to_owned());
        findings.set_file_state(kind, "stale");
    }
}

fn managed_projection_present(actual: &Value, desired: &Value) -> bool {
    let Some(desired_object) = desired.as_object() else {
        return actual == desired;
    };
    desired_object.iter().all(|(key, desired_value)| {
        let Some(actual_value) = actual.get(key) else {
            return false;
        };
        if key == "hooks" || key == "mcpServers" {
            return managed_projection_object_present(actual_value, desired_value);
        }
        managed_projection_present(actual_value, desired_value)
    })
}

fn managed_projection_object_present(actual: &Value, desired: &Value) -> bool {
    let (Some(actual_object), Some(desired_object)) = (actual.as_object(), desired.as_object())
    else {
        return false;
    };
    desired_object.iter().all(|(key, desired_value)| {
        let Some(actual_value) = actual_object.get(key) else {
            return false;
        };
        match (actual_value.as_array(), desired_value.as_array()) {
            (Some(actual_array), Some(desired_array)) => desired_array.iter().all(|desired_item| {
                let desired_count = desired_array
                    .iter()
                    .filter(|item| *item == desired_item)
                    .count();
                let actual_count = actual_array
                    .iter()
                    .filter(|item| *item == desired_item)
                    .count();
                actual_count == desired_count
            }),
            _ => actual_value == desired_value,
        }
    })
}

fn doctor_verify_managed_block(
    file: &Value,
    kind: &str,
    path_text: &str,
    text: &str,
    expected_hash: &str,
    findings: &mut DoctorGuardFileFindings,
) {
    let Some(start_marker) = file.get("managed_marker_start").and_then(Value::as_str) else {
        findings.broken_files.push(path_text.to_owned());
        findings.set_file_state(kind, "broken");
        return;
    };
    let Some(end_marker) = file.get("managed_marker_end").and_then(Value::as_str) else {
        findings.broken_files.push(path_text.to_owned());
        findings.set_file_state(kind, "broken");
        return;
    };
    if marker_count(text, start_marker) != 1 || marker_count(text, end_marker) != 1 {
        findings.broken_files.push(path_text.to_owned());
        findings.set_file_state(kind, "broken");
        return;
    }
    let Some(block) = managed_block_slice(text, start_marker, end_marker) else {
        findings.broken_files.push(path_text.to_owned());
        findings.set_file_state(kind, "broken");
        return;
    };
    if sha256_text(block) == expected_hash {
        findings.set_file_state(kind, "installed");
    } else {
        findings.stale_files.push(path_text.to_owned());
        findings.set_file_state(kind, "stale");
    }
}

fn marker_count(text: &str, marker: &str) -> usize {
    text.match_indices(marker).count()
}

fn managed_block_slice<'a>(text: &'a str, start_marker: &str, end_marker: &str) -> Option<&'a str> {
    let start = text.find(start_marker)?;
    let end = start + text[start..].find(end_marker)? + end_marker.len();
    let end = if text[end..].starts_with('\n') {
        end + 1
    } else {
        end
    };
    text.get(start..end)
}

fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{}", hex_bytes(&hasher.finalize()))
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn guard_missing_required_hooks(capability_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(capability_json) else {
        return Vec::new();
    };
    let configured_required_hooks = value
        .get("required_guard_phases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    let mut missing_required_hooks = value
        .get("missing_required_hooks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for required_hook in REQUIRED_GUARD_HOOK_PHASES {
        if !configured_required_hooks.contains(required_hook) {
            missing_required_hooks.push((*required_hook).to_owned());
        }
    }
    missing_required_hooks.sort();
    missing_required_hooks.dedup();
    missing_required_hooks
}

fn guard_expected_policy_hash(capability_json: &str) -> Option<String> {
    serde_json::from_str::<Value>(capability_json)
        .ok()
        .and_then(|value| {
            value
                .get("policy_hash")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned)
        })
}

fn guard_observation_current(
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> bool {
    let Some(expected_policy_hash) = guard_expected_policy_hash(&installation.host_capability_json)
    else {
        return false;
    };
    installation.last_seen_at.is_some()
        && installation.observed_host_kind.as_deref() == Some(installation.host_kind.as_str())
        && installation.observed_policy_hash.as_deref() == Some(expected_policy_hash.as_str())
        && matches!(
            installation.last_seen_phase.as_deref(),
            Some("session_start" | "pre_tool" | "post_tool" | "prompt_capture" | "stop")
        )
}

fn guard_configuration_healthy(
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> bool {
    matches!(
        installation.installation_status.as_str(),
        "active" | "configured"
    ) && guard_missing_required_hooks(&installation.host_capability_json).is_empty()
}

fn guard_effective_active(
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> bool {
    guard_configuration_healthy(installation)
        && installation.installation_status == GuardInstallationStatus::Active.as_str()
        && guard_observation_current(installation)
}

fn inspect_prompt_capture_availability(
    guarded: &[&volicord_store::inspection::GuardInstallationInspectionRecord],
    checks: &mut Vec<DiagnosticCheck>,
) {
    if guarded.is_empty() {
        checks.push(
            DiagnosticCheck::skipped(
                "prompt_capture_available",
                "prompt capture is not applicable to mcp-only installations",
            )
            .with_details(json!({
                "state": "not_applicable",
                "configured": 0,
                "observed": 0,
            })),
        );
        return;
    }
    let configured = guarded
        .iter()
        .filter(|installation| guard_prompt_capture_configured(&installation.host_capability_json))
        .count();
    let host_supported = guarded
        .iter()
        .filter(|installation| {
            guard_prompt_capture_host_supported(&installation.host_capability_json)
        })
        .count();
    let observed = guarded
        .iter()
        .filter(|installation| {
            installation.last_seen_at.is_some()
                && guard_prompt_capture_configured(&installation.host_capability_json)
        })
        .count();
    if host_supported == 0 {
        checks.push(
            DiagnosticCheck::warning(
                "prompt_capture_available",
                "host does not support prompt capture for recorded guarded installations",
            )
            .with_details(json!({
                "state": "unsupported_by_host",
                "configured": configured,
                "observed": observed,
                "host_supported": host_supported,
            })),
        );
    } else if observed > 0 {
        checks.push(
            DiagnosticCheck::passed("prompt_capture_available", "prompt capture is available")
                .with_details(json!({
                    "state": "available",
                    "configured": configured,
                    "observed": observed,
                    "host_supported": host_supported,
                })),
        );
    } else if configured > 0 {
        checks.push(
            DiagnosticCheck::warning(
                "prompt_capture_available",
                "prompt capture is configured but no guard hook observation is recorded",
            )
            .with_details(json!({
                "state": "configured_unobserved",
                "configured": configured,
                "observed": observed,
                "host_supported": host_supported,
            })),
        );
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "prompt_capture_available",
                "prompt capture is not configured for recorded guarded installations",
            )
            .with_details(json!({
                "state": "not_configured",
                "configured": configured,
                "observed": observed,
                "host_supported": host_supported,
            })),
        );
    }
}

fn guard_prompt_capture_configured(capability_json: &str) -> bool {
    serde_json::from_str::<Value>(capability_json)
        .ok()
        .and_then(|value| value.get("prompt_capture").and_then(Value::as_bool))
        .unwrap_or(false)
}

fn guard_prompt_capture_host_supported(capability_json: &str) -> bool {
    serde_json::from_str::<Value>(capability_json)
        .ok()
        .and_then(|value| {
            value
                .get("host_capabilities")
                .and_then(|capabilities| capabilities.get("user_prompt_submit_hook"))
                .and_then(Value::as_bool)
        })
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
                "Volicord doctor {}\nstatus_meaning: {}\nruntime_home_state: {}\nruntime_home: {}\ninstallation_profile_state: {}\ncommand_state: {}\nproject_registration_state: {}\nconnection_state: {}\nmcp_config_state: {}\nguard_installation_state: {}\nguard_configuration_state: {}\nguard_observation_state: {}\nguard_effective_state: {}\nguard_files_state: {}\nagents_block_state: {}\nvolicord_policy_file_state: {}\nrule_instruction_config_state: {}\nhook_config_state: {}\nrequired_guard_phases_state: {}\nrequired_guard_phases_missing: {}\nguard_hook_observed: {}\nguard_status_state: {}\nprompt_capture_state: {}\nprompt_capture_health: {}\nhost_reload_required: {}\n",
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
                doctor_check_state(checks, "guard_required_hooks_supported"),
                doctor_check_state(checks, "guard_hook_observed"),
                doctor_check_state(checks, "guard_status_active"),
                doctor_check_state(checks, "guard_files_installed"),
                doctor_guard_file_kind_state(checks, "agents_managed_block"),
                doctor_guard_file_kind_state(checks, "volicord_policy"),
                doctor_guard_file_kind_state(checks, "host_rule_instruction"),
                doctor_guard_file_kind_state(checks, "host_hook_config"),
                doctor_required_guard_phases_state(checks),
                doctor_missing_required_hooks_text(checks),
                doctor_check_state(checks, "guard_hook_observed"),
                doctor_check_state(checks, "guard_status_active"),
                doctor_prompt_capture_status(checks),
                doctor_prompt_capture_health(checks),
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
        "guard_configuration": doctor_check_state(checks, "guard_required_hooks_supported"),
        "guard_observation": doctor_check_state(checks, "guard_hook_observed"),
        "guard_effective": doctor_check_state(checks, "guard_status_active"),
        "guard_files": doctor_check_state(checks, "guard_files_installed"),
        "agents_managed_block": doctor_guard_file_kind_state(checks, "agents_managed_block"),
        "volicord_policy_file": doctor_guard_file_kind_state(checks, "volicord_policy"),
        "rule_instruction_config": doctor_guard_file_kind_state(checks, "host_rule_instruction"),
        "hook_config": doctor_guard_file_kind_state(checks, "host_hook_config"),
        "required_guard_phases": doctor_required_guard_phases_state(checks),
        "missing_required_hooks": doctor_missing_required_hooks_value(checks),
        "guard_hook_observed": doctor_check_state(checks, "guard_hook_observed"),
        "guard_status": doctor_check_state(checks, "guard_status_active"),
        "prompt_capture": doctor_prompt_capture_health(checks),
        "prompt_capture_status": doctor_prompt_capture_status(checks),
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

fn doctor_guard_file_kind_state(checks: &[DiagnosticCheck], kind: &str) -> String {
    checks
        .iter()
        .find(|check| check.id == "guard_files_installed")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("file_states"))
        .and_then(|states| states.get(kind))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| match check_status(checks, "guard_files_installed") {
            Some("skipped") | None => "not_checked".to_owned(),
            _ => "not_configured".to_owned(),
        })
}

fn doctor_required_guard_phases_state(checks: &[DiagnosticCheck]) -> &'static str {
    match check_status(checks, "guard_required_hooks_supported") {
        Some("passed") => "configured",
        Some("warning") | Some("failed") => "missing",
        Some("skipped") => "not_checked",
        _ => "unknown",
    }
}

fn doctor_missing_required_hooks_value(checks: &[DiagnosticCheck]) -> Vec<String> {
    checks
        .iter()
        .find(|check| check.id == "guard_required_hooks_supported")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("missing_required_hooks"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn doctor_missing_required_hooks_text(checks: &[DiagnosticCheck]) -> String {
    let missing = doctor_missing_required_hooks_value(checks);
    if missing.is_empty() {
        "none".to_owned()
    } else {
        missing.join(",")
    }
}

fn doctor_prompt_capture_health(checks: &[DiagnosticCheck]) -> &'static str {
    if check_status(checks, "prompt_capture_available").is_none() {
        "not_checked"
    } else {
        doctor_check_state(checks, "prompt_capture_available")
    }
}

fn doctor_prompt_capture_status(checks: &[DiagnosticCheck]) -> String {
    checks
        .iter()
        .find(|check| check.id == "prompt_capture_available")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("state"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| "not_checked".to_owned())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_projection_presence_allows_unmanaged_hook_groups_but_rejects_duplicate_managed() {
        let desired = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Edit|Write|MultiEdit",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "volicord guard pre-tool --host claude-code --json",
                                "timeout": 30
                            }
                        ]
                    }
                ]
            }
        });
        let actual_with_unmanaged = json!({
            "theme": "dark",
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "echo keep"
                            }
                        ]
                    },
                    desired["hooks"]["PreToolUse"][0].clone()
                ]
            }
        });
        assert!(managed_projection_present(&actual_with_unmanaged, &desired));

        let actual_with_duplicate = json!({
            "hooks": {
                "PreToolUse": [
                    desired["hooks"]["PreToolUse"][0].clone(),
                    desired["hooks"]["PreToolUse"][0].clone()
                ]
            }
        });
        assert!(!managed_projection_present(
            &actual_with_duplicate,
            &desired
        ));
    }
}
