use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::Serialize;
use serde_json::{json, Value};
use volicord_store::{
    agent_connections::{CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW},
    inspection::{
        inspect_runtime_home, DatabaseInspection, InspectionSchemaState,
        InstallationProfileInspectionRecord, RegistryInspectionSnapshot, RuntimeHomeInspection,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    session_watch::{
        default_watch_excluded_paths, latest_watch_baseline_for_connection,
        watch_scan_summary_from_entries_json, DEFAULT_MAX_FILE_HASH_BYTES,
        DEFAULT_MAX_SCAN_FILE_COUNT,
    },
};
use volicord_types::{GuardInstallationStatus, IntegrationProfile, SummaryCard};

use crate::{
    disclosure::detective_observation_disclosure_json,
    guard_integration::audit::{
        all_recorded_values_true, guard_file_findings_for_inspection,
        missing_required_hooks_from_capability_json, GuardFileFindings,
    },
    guard_integration::git_exclude::{always_local_paths, git_exclude_path, personal_only_paths},
    guard_integration::policy::validate_policy_schema,
    setup_command::{path_text, CommandOutcome, CommandStatus},
    shell_path::{
        detect_command_on_path, is_executable_file, mcp_binary_name, path_directory_is_on_path,
        paths_equivalent, volicord_binary_name, PATH_ENV,
    },
    summary_card::{render_summary_card_text, DIAGNOSTIC_SUMMARY_GUARANTEE},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DoctorOptions {
    output: OutputFormat,
    privacy_footprint: bool,
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
    "volicord doctor [--json] [--privacy-footprint]\n".to_owned()
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
    let options = parse_doctor_options(args)?;
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let inspection = inspect_runtime_home(&runtime_home);
    if options.privacy_footprint {
        return Ok(CommandOutcome {
            status: CommandStatus::Complete,
            output: render_privacy_footprint_output(options.output, &runtime_home, &inspection)?,
        });
    }
    let mut checks = Vec::new();
    let mut actions = Vec::new();

    inspect_build_identity(&mut checks);
    inspect_runtime_home_path(&runtime_home, &mut checks, &mut actions);
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
            inspect_personal_local_git_tracking(snapshot, &mut checks, &mut actions);
            inspect_session_watch_baselines(&runtime_home, snapshot, &mut checks);
        }
        DatabaseInspection::Unsupported { path, detail } => {
            checks.push(
                DiagnosticCheck::failed(
                    "registry",
                    "Runtime Home registry uses an unsupported schema",
                )
                .with_details(json!({
                    "path": path_text(path),
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
        output: render_doctor_output(options.output, status, &runtime_home, &checks, &actions)?,
    })
}

fn parse_doctor_options(args: &[String]) -> Result<DoctorOptions, DoctorCommandError> {
    let mut output = OutputFormat::Text;
    let mut privacy_footprint = false;
    for token in args {
        match token.as_str() {
            "-h" | "--help" | "help" => return Err(DoctorCommandError::Usage(doctor_usage())),
            "--json" => output = OutputFormat::Json,
            "--privacy-footprint" => privacy_footprint = true,
            option if option.starts_with("--json=") => {
                return Err(DoctorCommandError::Usage(
                    "--json does not accept a value".to_owned(),
                ))
            }
            option if option.starts_with("--privacy-footprint=") => {
                return Err(DoctorCommandError::Usage(
                    "--privacy-footprint does not accept a value".to_owned(),
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
    Ok(DoctorOptions {
        output,
        privacy_footprint,
    })
}

fn inspect_build_identity(checks: &mut Vec<DiagnosticCheck>) {
    let build = volicord_mcp::build_info();
    let git_metadata_known = build.git_commit != "unknown"
        && build.git_dirty.is_some()
        && build.metadata_source != "unknown";
    let compilation_metadata_known =
        build.target_triple != "unknown" && build.opt_level != "unknown" && build.debug.is_some();
    let exact_clean_identity = git_metadata_known
        && build.git_dirty == Some(false)
        && build.profile_exact
        && build.build_profile.is_some()
        && compilation_metadata_known;
    let summary = if !git_metadata_known {
        "build descriptor reports unknown Git metadata"
    } else if build.git_dirty == Some(true) {
        "build descriptor reports a dirty source tree without identifying its exact contents"
    } else if !build.profile_exact || build.build_profile.is_none() {
        "build descriptor reports only an approximate Cargo profile class"
    } else if !compilation_metadata_known {
        "build descriptor reports incomplete compilation metadata"
    } else {
        "build descriptor reports a clean source commit and exact build profile"
    };
    let check = if exact_clean_identity {
        DiagnosticCheck::passed("build_identity", summary)
    } else {
        DiagnosticCheck::warning("build_identity", summary)
    };
    checks.push(check.with_details(
        serde_json::to_value(build).expect("BuildInfo serialization should be infallible"),
    ));
}

fn render_privacy_footprint_output(
    output: OutputFormat,
    runtime_home: &Path,
    inspection: &RuntimeHomeInspection,
) -> Result<String, DoctorCommandError> {
    let registry_state = privacy_registry_state(&inspection.registry);
    let record_counts = privacy_record_counts(&inspection.registry);
    let stores = privacy_stores();
    let does_not_store = privacy_does_not_store();
    let does_not_prove = privacy_does_not_prove();

    match output {
        OutputFormat::Json => serde_json::to_string_pretty(&json!({
            "status": CommandStatus::Complete.as_str(),
            "runtime_home": path_text(runtime_home),
            "privacy_footprint": {
                "registry_state": registry_state,
                "registry_db_path": path_text(&inspection.registry_db_path),
                "record_counts": record_counts,
                "stores": stores,
                "does_not_store": does_not_store,
                "does_not_prove": does_not_prove,
                "doctor_output_scope": "category and count summary only; this command does not print stored row bodies",
            }
        }))
        .map(|text| format!("{text}\n"))
        .map_err(|error| DoctorCommandError::Runtime(error.to_string())),
        OutputFormat::Text => {
            let counts = record_counts
                .as_object()
                .map(|counts| {
                    counts
                        .iter()
                        .map(|(key, value)| format!("{key}={}", value.as_u64().unwrap_or(0)))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "unavailable".to_owned());
            Ok(format!(
                "Volicord Runtime Home privacy footprint\nruntime_home: {}\nregistry_state: {}\nregistry_db_path: {}\nrecord_counts: {}\nstores: {}\ndoes_not_store: {}\ndoes_not_prove: {}\ndoctor_output_scope: category and count summary only; this command does not print stored row bodies\n",
                runtime_home.display(),
                registry_state,
                inspection.registry_db_path.display(),
                counts,
                stores.join("; "),
                does_not_store.join("; "),
                does_not_prove.join("; "),
            ))
        }
    }
}

fn privacy_registry_state(
    registry: &DatabaseInspection<RegistryInspectionSnapshot>,
) -> &'static str {
    match registry {
        DatabaseInspection::Missing { .. } => "missing",
        DatabaseInspection::Present(_) => "present",
        DatabaseInspection::Unsupported { .. } => "unsupported",
        DatabaseInspection::Malformed { .. } => "malformed",
        DatabaseInspection::Unreadable { .. } => "unreadable",
    }
}

fn privacy_record_counts(registry: &DatabaseInspection<RegistryInspectionSnapshot>) -> Value {
    match registry {
        DatabaseInspection::Present(snapshot) => json!({
            "projects": snapshot.projects.len(),
            "agent_connections": snapshot.agent_connections.len(),
            "connection_projects": snapshot.connection_projects.len(),
            "guard_installations": snapshot.guard_installations.len(),
            "project_state_databases": snapshot.projects.len(),
        }),
        _ => Value::Null,
    }
}

fn privacy_stores() -> Vec<&'static str> {
    vec![
        "Runtime Home identity, registry path, storage profile, installation profile, command paths, and setup metadata",
        "Product Repository registrations, project home paths, project state database paths, and Agent Connection records",
        "detective host-hook installation records, capability metadata, policy hashes, hook observation timestamps, and prompt-capture availability state",
        "project state records for tasks, change units, write tickets, evidence metadata, close-readiness records, User Channel judgments, and artifacts when those features are used",
        "session-watch baselines and observations with relative paths, file hashes, file sizes, skip reasons, scan summaries, timestamps, and observation links",
    ]
}

fn privacy_does_not_store() -> Vec<&'static str> {
    vec![
        "session-watch snapshots do not store Product Repository file contents",
        "prompt-capture availability and verification-code records do not include raw prompt text by default",
        "doctor --privacy-footprint reports categories and counts, not stored row bodies",
    ]
}

fn privacy_does_not_prove() -> Vec<&'static str> {
    vec![
        "actor attribution",
        "write prevention",
        "tamper-proof audit",
        "full filesystem monitoring",
        "OS enforcement or security isolation",
        "product correctness, test sufficiency, human review, final acceptance, or residual-risk acceptance",
    ]
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
        InspectionSchemaState::Current => checks.push(
            DiagnosticCheck::passed("registry_schema", "Runtime Home registry schema is current")
                .with_details(json!({
                    "path": path_text(&snapshot.path),
                    "storage_profile": snapshot.runtime_home.storage_profile,
                })),
        ),
    }
}

const MAX_PERSONAL_GIT_PROJECTS: usize = 32;
const MAX_PERSONAL_GIT_FINDINGS: usize = 64;
const MAX_LOCAL_POLICY_BYTES: u64 = 1024 * 1024;

fn inspect_personal_local_git_tracking(
    snapshot: &RegistryInspectionSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    actions: &mut Vec<DiagnosticAction>,
) {
    let connected_connections = snapshot
        .agent_connections
        .iter()
        .map(|connection| connection.connection_internal_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut project_internal_ids = snapshot
        .connection_projects
        .iter()
        .filter(|membership| {
            connected_connections.contains(membership.connection_internal_id.as_str())
        })
        .map(|membership| membership.project_internal_id.as_str())
        .collect::<BTreeSet<_>>();
    project_internal_ids.extend(
        snapshot
            .agent_connections
            .iter()
            .filter_map(|connection| connection.project_internal_id.as_deref()),
    );
    let mut projects = snapshot
        .projects
        .iter()
        .filter(|project| project_internal_ids.contains(project.project_internal_id.as_str()))
        .collect::<Vec<_>>();
    projects.sort_by(|left, right| left.project_id.cmp(&right.project_id));
    if projects.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "personal_local_git_tracking",
            "no repository connection is recorded",
        ));
        return;
    }

    let project_count = projects.len();
    let mut truncated = project_count > MAX_PERSONAL_GIT_PROJECTS;
    projects.truncate(MAX_PERSONAL_GIT_PROJECTS);
    let mut tracked_paths = Vec::new();
    let mut unignored_paths = Vec::new();
    let mut audit_errors = Vec::new();
    let mut effective_personal_project_count = 0usize;

    'projects: for project in &projects {
        let exclude_path = match git_exclude_path(&project.repo_root) {
            Ok(Some(path)) => path,
            Ok(None) => continue,
            Err(error) => {
                push_bounded_git_finding(
                    &mut audit_errors,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "detail": error.to_string(),
                    }),
                    &mut truncated,
                );
                continue;
            }
        };
        let include_personal_paths = match local_policy_connection_intent(&project.repo_root) {
            Ok(intent) => {
                let personal = intent == "personal";
                effective_personal_project_count += usize::from(personal);
                personal
            }
            Err(detail) => {
                push_bounded_git_finding(
                    &mut audit_errors,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "path": "/.volicord/policy.json",
                        "detail": detail,
                    }),
                    &mut truncated,
                );
                true
            }
        };
        let local_paths = always_local_paths().iter().copied().chain(
            include_personal_paths
                .then_some(personal_only_paths().iter().copied())
                .into_iter()
                .flatten(),
        );
        for local_path in local_paths {
            if tracked_paths.len() + unignored_paths.len() + audit_errors.len()
                >= MAX_PERSONAL_GIT_FINDINGS
            {
                truncated = true;
                break 'projects;
            }
            let pathspec = local_path.trim_start_matches('/').trim_end_matches('/');
            let ignore_probe = if local_path.ends_with('/') {
                format!("{pathspec}/policy.json")
            } else {
                pathspec.to_owned()
            };
            let tracked = match git_path_predicate(
                &project.repo_root,
                true,
                &["ls-files", "--error-unmatch", "--", pathspec],
            ) {
                Ok(value) => value,
                Err(detail) => {
                    push_bounded_git_finding(
                        &mut audit_errors,
                        json!({
                            "project_id": project.project_id,
                            "repo_root": path_text(&project.repo_root),
                            "detail": detail,
                        }),
                        &mut truncated,
                    );
                    continue 'projects;
                }
            };
            if tracked {
                push_bounded_git_finding(
                    &mut tracked_paths,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "path": local_path,
                        "exclude_path": path_text(&exclude_path),
                    }),
                    &mut truncated,
                );
                continue;
            }
            match fs::symlink_metadata(project.repo_root.join(pathspec)) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    push_bounded_git_finding(
                        &mut audit_errors,
                        json!({
                            "project_id": project.project_id,
                            "repo_root": path_text(&project.repo_root),
                            "path": local_path,
                            "detail": format!("failed to inspect the local path: {error}"),
                        }),
                        &mut truncated,
                    );
                    continue;
                }
            }
            let ignored = match git_path_predicate(
                &project.repo_root,
                false,
                &["check-ignore", "--quiet", "--no-index", "--", &ignore_probe],
            ) {
                Ok(value) => value,
                Err(detail) => {
                    push_bounded_git_finding(
                        &mut audit_errors,
                        json!({
                            "project_id": project.project_id,
                            "repo_root": path_text(&project.repo_root),
                            "path": local_path,
                            "detail": detail,
                        }),
                        &mut truncated,
                    );
                    continue 'projects;
                }
            };
            if !ignored {
                push_bounded_git_finding(
                    &mut unignored_paths,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "path": local_path,
                        "exclude_path": path_text(&exclude_path),
                    }),
                    &mut truncated,
                );
            }
        }
    }

    let details = json!({
        "connected_project_count": project_count,
        "effective_personal_project_count": effective_personal_project_count,
        "audited_project_count": projects.len(),
        "tracked_paths": tracked_paths,
        "unignored_existing_paths": unignored_paths,
        "audit_errors": audit_errors,
        "truncated": truncated,
        "reads_local_policy_file": true,
        "does_not_read_other_local_integration_file_contents": true,
    });
    let has_warning = details["tracked_paths"]
        .as_array()
        .is_some_and(|values| !values.is_empty())
        || details["unignored_existing_paths"]
            .as_array()
            .is_some_and(|values| !values.is_empty())
        || details["audit_errors"]
            .as_array()
            .is_some_and(|values| !values.is_empty())
        || truncated;
    let check = if has_warning {
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "protect_personal_local_files".to_owned(),
                instruction: "Review the listed repositories, rerun init with the intended connection intent to restore repository-local excludes, and remove any listed local-only paths from the Git index without deleting their working-tree files."
                    .to_owned(),
                command: None,
            },
        );
        DiagnosticCheck::warning(
            "personal_local_git_tracking",
            "local integration files need Git tracking follow-up",
        )
    } else {
        DiagnosticCheck::passed(
            "personal_local_git_tracking",
            "local integration files are outside the Git index and ignored",
        )
    };
    checks.push(check.with_details(details));
}

fn local_policy_connection_intent(repo_root: &Path) -> Result<String, String> {
    let policy_dir = repo_root.join(".volicord");
    let policy_path = policy_dir.join("policy.json");
    let directory_metadata = fs::symlink_metadata(&policy_dir)
        .map_err(|error| format!("failed to inspect the local policy directory: {error}"))?;
    if directory_metadata.file_type().is_symlink() || !directory_metadata.is_dir() {
        return Err("the local policy directory is not a regular directory".to_owned());
    }
    let metadata = fs::symlink_metadata(&policy_path)
        .map_err(|error| format!("failed to inspect the local policy file: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("the local policy file is not a regular file".to_owned());
    }
    if metadata.len() > MAX_LOCAL_POLICY_BYTES {
        return Err(format!(
            "the local policy file exceeds the {MAX_LOCAL_POLICY_BYTES}-byte audit limit"
        ));
    }
    let text = fs::read_to_string(&policy_path)
        .map_err(|error| format!("failed to read the local policy file: {error}"))?;
    let policy = serde_json::from_str::<Value>(&text)
        .map_err(|error| format!("the local policy file is not valid JSON: {error}"))?;
    let connection_intent = policy
        .get("connection_intent")
        .and_then(Value::as_str)
        .ok_or_else(|| "the local policy is missing connection_intent".to_owned())?;
    validate_policy_schema(&policy, connection_intent)
        .map_err(|error| format!("the local policy schema is invalid: {error}"))?;
    Ok(connection_intent.to_owned())
}

fn push_bounded_git_finding(values: &mut Vec<Value>, value: Value, truncated: &mut bool) {
    if values.len() < MAX_PERSONAL_GIT_FINDINGS {
        values.push(value);
    } else {
        *truncated = true;
    }
}

fn git_path_predicate(
    repo_root: &Path,
    literal_pathspecs: bool,
    args: &[&str],
) -> Result<bool, String> {
    let mut command = Command::new("git");
    if literal_pathspecs {
        command.arg("--literal-pathspecs");
    }
    command
        .args(args)
        .current_dir(repo_root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for name in [
        "GIT_COMMON_DIR",
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_WORK_TREE",
    ] {
        command.env_remove(name);
    }
    let status = command
        .status()
        .map_err(|error| format!("failed to run the local Git tracking check: {error}"))?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        code => Err(format!(
            "the local Git tracking check exited with status {}",
            code.map_or_else(|| "signal".to_owned(), |value| value.to_string())
        )),
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
            "no detective installations are recorded",
        ));
        checks.push(DiagnosticCheck::skipped(
            "guard_host_reload_required",
            "no detective installation needs host reload",
        ));
        checks.push(DiagnosticCheck::skipped(
            "guard_hook_observed",
            "no detective host-hook observation is recorded",
        ));
        checks.push(DiagnosticCheck::skipped(
            "guard_required_hooks_supported",
            "no detective host-hook capability record is available",
        ));
        checks.push(DiagnosticCheck::skipped(
            "guard_status_active",
            "no detective installation status is recorded",
        ));
        checks.push(
            DiagnosticCheck::skipped("control_surface", "no integration profile is recorded")
                .with_details(json!({
                    "selected_profile": "not_checked",
                    "control_surface": {
                        "selected_profile": "not_checked",
                        "host_hooks_active": false,
                        "session_watcher_active": false,
                        "cooperative_pre_tool_warning_available": false,
                        "cooperative_pre_tool_denial_available": false,
                        "unrecorded_changes_detectable": false,
                        "actor_identity_provable": false,
                        "os_enforced": false,
                    },
                })),
        );
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

    let observed_profile_installations = snapshot
        .guard_installations
        .iter()
        .filter(|installation| installation.guard_mode == IntegrationProfile::Detective.as_str())
        .collect::<Vec<_>>();
    let mut file_findings = GuardFileFindings::default();
    for installation in &snapshot.guard_installations {
        file_findings.merge(guard_file_findings_for_inspection(
            installation,
            &snapshot.projects,
        ));
    }
    file_findings.sort_dedup();
    let mut detective_file_findings = GuardFileFindings::default();
    for installation in &observed_profile_installations {
        detective_file_findings.merge(guard_file_findings_for_inspection(
            installation,
            &snapshot.projects,
        ));
    }
    detective_file_findings.sort_dedup();
    let selected_profile =
        doctor_selected_profile_state(&snapshot.guard_installations, &file_findings);
    let missing_required_hooks = observed_profile_installations
        .iter()
        .flat_map(|installation| {
            missing_required_hooks_from_capability_json(&installation.host_capability_json)
        })
        .collect::<Vec<_>>();
    let observed_count = observed_profile_installations
        .iter()
        .filter(|installation| guard_observation_current(installation))
        .count();
    let control_surface = doctor_control_surface_summary(
        &selected_profile,
        &observed_profile_installations,
        &file_findings,
        observed_count,
        missing_required_hooks.is_empty(),
    );
    let control_surface_check = if selected_profile == "mixed" {
        DiagnosticCheck::warning(
            "control_surface",
            "detective installations record mixed integration profiles",
        )
    } else {
        DiagnosticCheck::passed(
            "control_surface",
            format!("selected integration profile is {selected_profile}"),
        )
    };
    checks.push(control_surface_check.with_details(json!({
        "selected_profile": selected_profile,
        "control_surface": control_surface,
    })));
    let guard_file_problem = !detective_file_findings.missing_files.is_empty()
        || !detective_file_findings.stale_files.is_empty()
        || !detective_file_findings.broken_files.is_empty();
    if observed_profile_installations.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "guard_files_installed",
            "detective host-hook files are not applicable to record-profile installations",
        ));
    } else if !guard_file_problem {
        checks.push(
            DiagnosticCheck::passed(
                "guard_files_installed",
                "detective host-hook files are installed",
            )
            .with_details(doctor_guard_file_details(&detective_file_findings)),
        );
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "guard_files_installed",
                "one or more detective host-hook files are missing, stale, or broken",
            )
            .with_details(doctor_guard_file_details(&detective_file_findings)),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_guard_files".to_owned(),
                instruction:
                    "Reinstall or refresh detective host-hook files for affected detective-profile projects."
                        .to_owned(),
                command: Some("volicord init --host HOST --repo PATH".to_owned()),
            },
        );
    }
    if !observed_profile_installations.is_empty()
        && detective_file_findings.hook_path_safety_state() != "ok"
        && !matches!(
            detective_file_findings.hook_path_safety_state().as_str(),
            "not_recorded" | "not_checked" | "not_applicable"
        )
    {
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_guard_hook_path_safety".to_owned(),
                instruction:
                    "Regenerate cwd-independent hook commands for affected detective-profile projects."
                        .to_owned(),
                command: Some("volicord init --host HOST --repo PATH".to_owned()),
            },
        );
    }

    let reload_required = observed_profile_installations.iter().any(|installation| {
        installation.installation_status == GuardInstallationStatus::ReloadRequired.as_str()
    });
    if reload_required {
        checks.push(
            DiagnosticCheck::warning(
                "guard_host_reload_required",
                "one or more detective installations need host reload",
            )
            .with_details(json!({ "reload_required": true })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "reload_guard_host".to_owned(),
                instruction:
                    "Restart or reload affected agent hosts so they load the Volicord host hook configuration."
                        .to_owned(),
                command: None,
            },
        );
    } else {
        checks.push(DiagnosticCheck::passed(
            "guard_host_reload_required",
            "no recorded detective installation requires host reload",
        ));
    }

    if observed_profile_installations.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "guard_required_hooks_supported",
            "host hook capability is not applicable to record-profile installations",
        ));
    } else if missing_required_hooks.is_empty() {
        checks.push(DiagnosticCheck::passed(
            "guard_required_hooks_supported",
            "required detective host-hook capabilities are recorded",
        ));
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "guard_required_hooks_supported",
                "one or more detective-profile installations are missing required hook capabilities",
            )
            .with_details(json!({ "missing_required_hooks": missing_required_hooks })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_guard_required_hooks".to_owned(),
                instruction:
                    "Use a host adapter that supports every required detective host hook, or use the record profile."
                        .to_owned(),
                command: Some("volicord init --host HOST --repo PATH".to_owned()),
            },
        );
    }

    if observed_profile_installations.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "guard_hook_observed",
            "detective host-hook observation is not applicable to record-profile installations",
        ));
    } else if observed_count == observed_profile_installations.len() {
        checks.push(
            DiagnosticCheck::passed("guard_hook_observed", "detective host hooks have been observed")
                .with_details(json!({ "observed": observed_count, "detective": observed_profile_installations.len() })),
        );
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "guard_hook_observed",
                "one or more detective-profile installations have not been observed",
            )
            .with_details(json!({ "observed": observed_count, "detective": observed_profile_installations.len() })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "detective_host_hook".to_owned(),
                instruction:
                    "Start, restart, or reload affected agent hosts so the Volicord detective host hook runs."
                        .to_owned(),
                command: None,
            },
        );
    }

    let status_counts = guard_status_counts_for_refs(&observed_profile_installations);
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
                format!("one or more detective installations are {status}"),
            )
            .with_details(json!({ "status_counts": status_counts })),
        );
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_guard_status".to_owned(),
                instruction:
                    "Repair or reinstall affected detective-profile integrations before relying on host-hook observation."
                        .to_owned(),
                command: Some("volicord init --host HOST --repo PATH".to_owned()),
            },
        );
    } else if observed_profile_installations.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "guard_status_active",
            "detective signal active status is not applicable to record-profile installations",
        ));
    } else if observed_profile_installations
        .iter()
        .all(|installation| guard_effective_active(installation))
    {
        checks.push(
            DiagnosticCheck::passed(
                "guard_status_active",
                "effective detective signal status is active",
            )
            .with_details(json!({ "status_counts": status_counts })),
        );
    } else {
        checks.push(
            DiagnosticCheck::warning(
                "guard_status_active",
                "effective detective signal status is not active for one or more detective-profile installations",
            )
            .with_details(json!({
                "status_counts": status_counts,
                "effective_active": observed_profile_installations.iter().filter(|installation| guard_effective_active(installation)).count(),
                "detective": observed_profile_installations.len(),
            })),
        );
    }

    inspect_prompt_capture_availability(&observed_profile_installations, checks);
}

#[derive(Debug, Default)]
struct DoctorWatcherScanAggregate {
    baseline_count: u64,
    missing_baseline_count: u64,
    files_scanned: u64,
    files_skipped: u64,
    unreadable_paths_count: u64,
    degraded_reason_counts: BTreeMap<String, u64>,
    skipped_paths_sample: Vec<String>,
    skipped_paths_truncated: bool,
    baseline_status_counts: BTreeMap<String, u64>,
    coverage_basis_values: BTreeSet<String>,
    partial_coverage_warnings: BTreeSet<String>,
    latest_baseline_created_at: Option<String>,
    latest_coverage_start_at: Option<String>,
    read_errors: Vec<String>,
}

fn inspect_session_watch_baselines(
    runtime_home: &Path,
    snapshot: &RegistryInspectionSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let detective_installations = snapshot
        .guard_installations
        .iter()
        .filter(|installation| installation.guard_mode == IntegrationProfile::Detective.as_str())
        .collect::<Vec<_>>();
    if detective_installations.is_empty() {
        checks.push(
            DiagnosticCheck::skipped(
                "watcher_scan_summary",
                "no detective session-watch baseline is applicable",
            )
            .with_details(doctor_watcher_details_json(
                "not_applicable",
                &DoctorWatcherScanAggregate::default(),
            )),
        );
        return;
    }

    let mut aggregate = DoctorWatcherScanAggregate::default();
    for installation in detective_installations {
        let Some(project_id) = installation.project_id.as_deref() else {
            aggregate.read_errors.push(format!(
                "{}: detective installation has no project id",
                installation.guard_installation_id
            ));
            continue;
        };
        match latest_watch_baseline_for_connection(
            runtime_home,
            project_id,
            &installation.connection_internal_id,
        ) {
            Ok(Some(baseline)) => {
                if let Err(error) = doctor_merge_watcher_baseline(&mut aggregate, &baseline) {
                    aggregate
                        .read_errors
                        .push(format!("{}: {error}", installation.guard_installation_id));
                }
            }
            Ok(None) => aggregate.missing_baseline_count += 1,
            Err(error) => aggregate
                .read_errors
                .push(format!("{}: {error}", installation.guard_installation_id)),
        }
    }

    let watcher_status = doctor_watcher_status(&aggregate);
    let details = doctor_watcher_details_json(&watcher_status, &aggregate);
    let check = if !aggregate.read_errors.is_empty() {
        DiagnosticCheck::warning(
            "watcher_scan_summary",
            "one or more session-watch baselines could not be read",
        )
    } else if aggregate.baseline_count == 0 {
        DiagnosticCheck::warning(
            "watcher_scan_summary",
            "no session-watch baseline is recorded for detective installations",
        )
    } else if aggregate.files_skipped > 0 || !aggregate.degraded_reason_counts.is_empty() {
        DiagnosticCheck::warning(
            "watcher_scan_summary",
            "session watcher scan recorded skipped or degraded coverage",
        )
    } else {
        DiagnosticCheck::passed(
            "watcher_scan_summary",
            "session watcher scan summary is recorded",
        )
    };
    checks.push(check.with_details(details));
}

fn doctor_merge_watcher_baseline(
    aggregate: &mut DoctorWatcherScanAggregate,
    baseline: &volicord_store::session_watch::WatchBaselineRecord,
) -> Result<(), String> {
    aggregate.baseline_count += 1;
    *aggregate
        .baseline_status_counts
        .entry(baseline.status.clone())
        .or_insert(0) += 1;
    if aggregate
        .latest_baseline_created_at
        .as_deref()
        .is_none_or(|current| baseline.created_at.as_str() > current)
    {
        aggregate.latest_baseline_created_at = Some(baseline.created_at.clone());
    }

    let metadata = serde_json::from_str::<Value>(&baseline.metadata_json).unwrap_or(Value::Null);
    if let Some(coverage_start_at) = metadata
        .get("coverage_start_at")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        if aggregate
            .latest_coverage_start_at
            .as_deref()
            .is_none_or(|current| coverage_start_at > current)
        {
            aggregate.latest_coverage_start_at = Some(coverage_start_at.to_owned());
        }
    }
    if let Some(coverage_basis) = metadata
        .get("coverage_basis")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        aggregate
            .coverage_basis_values
            .insert(coverage_basis.to_owned());
    }
    if let Some(warning) = metadata
        .get("partial_coverage_warning")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        aggregate
            .partial_coverage_warnings
            .insert(warning.to_owned());
    }

    let scan_summary = metadata
        .get("scan_summary")
        .and_then(|raw_summary| {
            serde_json::from_value::<volicord_store::session_watch::WatchScanSummary>(
                raw_summary.clone(),
            )
            .ok()
        })
        .map(Ok)
        .unwrap_or_else(|| watch_scan_summary_from_entries_json(&baseline.snapshot_entries_json))
        .map_err(|error| error.to_string())?;
    aggregate.files_scanned += scan_summary.files_scanned;
    aggregate.files_skipped += scan_summary.files_skipped;
    aggregate.unreadable_paths_count += scan_summary.unreadable_paths_count;
    for (reason, count) in scan_summary.degraded_reason_counts {
        *aggregate.degraded_reason_counts.entry(reason).or_insert(0) += count;
    }
    for path in scan_summary.skipped_paths_sample {
        if aggregate.skipped_paths_sample.len() < 20 {
            aggregate.skipped_paths_sample.push(path);
        } else {
            aggregate.skipped_paths_truncated = true;
            break;
        }
    }
    aggregate.skipped_paths_truncated |= scan_summary.skipped_paths_truncated;
    Ok(())
}

fn doctor_watcher_status(aggregate: &DoctorWatcherScanAggregate) -> String {
    if aggregate.baseline_count == 0 {
        return "not_started".to_owned();
    }
    if aggregate.baseline_status_counts.len() == 1 {
        aggregate
            .baseline_status_counts
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "unknown".to_owned())
    } else {
        "mixed".to_owned()
    }
}

fn doctor_watcher_details_json(status: &str, aggregate: &DoctorWatcherScanAggregate) -> Value {
    let degraded_reasons = aggregate
        .degraded_reason_counts
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "watcher_status": status,
        "baseline_count": aggregate.baseline_count,
        "missing_baseline_count": aggregate.missing_baseline_count,
        "baseline_status_counts": aggregate.baseline_status_counts,
        "baseline_created_at": aggregate.latest_baseline_created_at,
        "coverage_start_at": aggregate.latest_coverage_start_at,
        "coverage_basis": doctor_single_or_list(&aggregate.coverage_basis_values),
        "partial_coverage_warning": doctor_single_or_list(&aggregate.partial_coverage_warnings),
        "scan_summary": {
            "files_scanned": aggregate.files_scanned,
            "files_skipped": aggregate.files_skipped,
            "unreadable_paths_count": aggregate.unreadable_paths_count,
            "degraded_reasons": degraded_reasons,
            "degraded_reason_counts": aggregate.degraded_reason_counts,
            "skipped_paths_sample": aggregate.skipped_paths_sample,
            "skipped_paths_truncated": aggregate.skipped_paths_truncated,
            "default_excluded_paths": default_watch_excluded_paths(),
            "max_file_size_bytes": DEFAULT_MAX_FILE_HASH_BYTES,
            "max_file_count": DEFAULT_MAX_SCAN_FILE_COUNT,
            "follows_symlinks": false,
            "not_full_filesystem_monitoring": true,
        },
        "read_errors": aggregate.read_errors,
    })
}

fn doctor_single_or_list(values: &BTreeSet<String>) -> Value {
    match values.len() {
        0 => Value::Null,
        1 => Value::String(values.iter().next().cloned().unwrap_or_default()),
        _ => json!(values.iter().cloned().collect::<Vec<_>>()),
    }
}

fn doctor_guard_file_details(findings: &GuardFileFindings) -> Value {
    json!({
        "missing_files": &findings.missing_files,
        "stale_files": &findings.stale_files,
        "broken_files": &findings.broken_files,
        "file_states": doctor_guard_file_states(findings),
        "selected_profiles": &findings.guard_profiles,
        "generated_config_verified": findings.generated_config_verified(),
        "native_host_output_adapter_verified": findings.native_host_output_adapter_verified(),
        "hook_path_safety": findings.hook_path_safety_state(),
        "hook_commands_cwd_independent": all_recorded_values_true(&findings.hook_cwd_independent_values),
        "hook_commands_subdirectory_safe": all_recorded_values_true(&findings.hook_subdirectory_safe_values),
        "hook_path_safety_details": &findings.hook_path_safety_details,
        "bash_shell_mutation_coverage": findings.bash_shell_mutation_coverage(),
        "direct_file_write_matcher_coverage": findings.direct_file_write_matcher_coverage(),
    })
}

fn doctor_guard_file_states(findings: &GuardFileFindings) -> BTreeMap<String, String> {
    let mut states = findings.file_kind_states.clone();
    if findings
        .broken_files
        .iter()
        .any(|file| file == "host_hook_capability_json")
    {
        return states;
    }
    states
        .entry("host_hook_config".to_owned())
        .or_insert_with(|| findings.hook_config_state(false));
    let rule_instruction_state = findings.rule_instruction_state(false);
    if rule_instruction_state != "not_configured" {
        states
            .entry("host_rule_instruction".to_owned())
            .or_insert(rule_instruction_state);
    }
    states
}

fn doctor_selected_profile_state(
    installations: &[volicord_store::inspection::GuardInstallationInspectionRecord],
    findings: &GuardFileFindings,
) -> String {
    if let Some(value) = single_or_mixed(&findings.guard_profiles) {
        return value;
    }
    match doctor_guard_mode_state(installations).as_str() {
        "record" => "record",
        "detective" => "detective",
        _ => "mixed",
    }
    .to_owned()
}

fn doctor_control_surface_summary(
    selected_profile: &str,
    detective_installations: &[&volicord_store::inspection::GuardInstallationInspectionRecord],
    findings: &GuardFileFindings,
    observed_count: usize,
    required_hooks_available: bool,
) -> Value {
    let host_hooks_active = selected_profile == IntegrationProfile::Detective.as_str()
        && !detective_installations.is_empty()
        && observed_count == detective_installations.len()
        && detective_installations
            .iter()
            .all(|installation| guard_effective_active(installation))
        && required_hooks_available
        && findings.generated_config_verified()
        && all_recorded_values_true(&findings.native_host_output_adapter_verified_values)
        && all_recorded_values_true(&findings.bash_shell_mutation_coverage_values)
        && all_recorded_values_true(&findings.direct_file_write_matcher_coverage_values);
    json!({
        "selected_profile": selected_profile,
        "host_hooks_active": host_hooks_active,
        "session_watcher_active": false,
        "cooperative_pre_tool_warning_available": host_hooks_active,
        "cooperative_pre_tool_denial_available": host_hooks_active,
        "unrecorded_changes_detectable": false,
        "actor_identity_provable": false,
        "os_enforced": false,
    })
}

fn doctor_guard_mode_state(
    installations: &[volicord_store::inspection::GuardInstallationInspectionRecord],
) -> String {
    let mut modes = installations
        .iter()
        .map(|installation| installation.guard_mode.as_str())
        .collect::<Vec<_>>();
    modes.sort_unstable();
    modes.dedup();
    if modes.len() == 1 {
        modes[0].to_owned()
    } else {
        "mixed".to_owned()
    }
}

fn single_or_mixed(values: &[String]) -> Option<String> {
    match values {
        [] => None,
        [value] => Some(value.clone()),
        _ => Some("mixed".to_owned()),
    }
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
    ) && missing_required_hooks_from_capability_json(&installation.host_capability_json).is_empty()
}

fn guard_effective_active(
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> bool {
    guard_configuration_healthy(installation)
        && installation.installation_status == GuardInstallationStatus::Active.as_str()
        && guard_observation_current(installation)
}

fn inspect_prompt_capture_availability(
    detective_installations: &[&volicord_store::inspection::GuardInstallationInspectionRecord],
    checks: &mut Vec<DiagnosticCheck>,
) {
    if detective_installations.is_empty() {
        checks.push(
            DiagnosticCheck::skipped(
                "prompt_capture_available",
                "prompt capture is not applicable to record-profile installations",
            )
            .with_details(json!({
                "state": "not_applicable",
                "configured": 0,
                "observed": 0,
            })),
        );
        return;
    }
    let configured = detective_installations
        .iter()
        .filter(|installation| guard_prompt_capture_configured(&installation.host_capability_json))
        .count();
    let host_supported = detective_installations
        .iter()
        .filter(|installation| {
            guard_prompt_capture_host_supported(&installation.host_capability_json)
        })
        .count();
    let observed = detective_installations
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
                "host does not support prompt capture for recorded detective-profile installations",
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
                "prompt capture is configured but no detective host-hook observation is recorded",
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
                "prompt capture is not configured for recorded detective-profile installations",
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

fn guard_status_counts_for_refs(
    installations: &[&volicord_store::inspection::GuardInstallationInspectionRecord],
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
                "Select an executable MCP launch command, then rerun init with that command."
                    .to_owned(),
            command: Some(
                "volicord init --host <host> --repo <path> --mcp-command PATH".to_owned(),
            ),
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
                    "Repair the command links in {} or reinstall the volicord executable on PATH; restart or reload existing agent hosts after command-link changes.",
                    profile.bin_dir.display()
                ),
                command: None,
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
                    "Install the volicord executable in a command directory you keep on PATH; restart or reload existing agent hosts after PATH or command-link changes."
                        .to_owned(),
                command: None,
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
    let summary_card = doctor_summary_card(status, checks, actions);
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
                "build_id": volicord_mcp::build_id(),
                "build": volicord_mcp::build_info(),
                "summary_card": &summary_card,
                "disclosure": detective_observation_disclosure_json(),
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
        OutputFormat::Text => Ok(render_compact_doctor_text(
            status,
            runtime_home,
            checks,
            actions,
        )),
    }
}

fn render_compact_doctor_text(
    status: CommandStatus,
    runtime_home: &Path,
    checks: &[DiagnosticCheck],
    actions: &[DiagnosticAction],
) -> String {
    let mut text_summary_card = doctor_summary_card(status, checks, actions);
    text_summary_card.next = doctor_next_summary_text(status, actions);
    let mut text = format!("Volicord doctor {}\n\n", status.as_str());
    text.push_str(&render_summary_card_text(&text_summary_card));
    text.push_str("\nStatus:\n");
    text.push_str(&format!(
        "  Installation profile: {}\n  Runtime Home: {}\n  Commands: {}\n  Host reload required: {}\n",
        doctor_status_meaning(status, checks),
        display_state_text(&doctor_runtime_home_state(runtime_home, checks)),
        display_state_text(doctor_command_state(checks)),
        yes_no(doctor_host_reload_required(checks, actions)),
    ));
    text.push_str(&format!("\nRuntime Home:\n  {}\n", runtime_home.display()));
    text.push_str(&format!("\nBuild:\n  {}\n", volicord_mcp::build_id()));
    append_doctor_check_summary(&mut text, checks, actions);
    append_doctor_next_actions(&mut text, status, actions);
    text.push_str(
        "\nLimits:\n  Local setup diagnostics are not OS enforcement, write prevention, actor attribution proof, correctness proof, test sufficiency proof, or review completion.\n\nDiagnostics:\n  Run:\n    volicord doctor --json\n",
    );
    text
}

fn append_doctor_check_summary(
    output: &mut String,
    checks: &[DiagnosticCheck],
    actions: &[DiagnosticAction],
) {
    output.push_str("\nChecks:\n");
    for (label, value) in doctor_compact_check_rows(checks, actions) {
        output.push_str(&format!("  {label}: {}\n", display_state_text(&value)));
    }
    let not_passed = checks
        .iter()
        .filter(|check| check.status != "passed")
        .collect::<Vec<_>>();
    if not_passed.is_empty() {
        output.push_str("  Detailed diagnostics: passed\n");
        return;
    }
    output.push_str("  Follow-up diagnostics:\n");
    for check in not_passed {
        output.push_str(&format!(
            "    - {} ({})\n",
            check.summary,
            display_state_text(&check.status)
        ));
    }
}

fn doctor_compact_check_rows(
    checks: &[DiagnosticCheck],
    actions: &[DiagnosticAction],
) -> Vec<(&'static str, String)> {
    vec![
        (
            "Installation profile",
            doctor_installation_profile_state(checks).to_owned(),
        ),
        ("Projects", doctor_count_state(checks, "projects", "registered")),
        (
            "Connections",
            doctor_count_state(checks, "connections", "stored"),
        ),
        ("MCP configuration", doctor_mcp_config_state(checks)),
        ("Profile", doctor_selected_profile_from_checks(checks)),
        (
            "Detective files",
            doctor_check_state(checks, "guard_files_installed").to_owned(),
        ),
        (
            "Hook observation",
            doctor_check_state(checks, "guard_hook_observed").to_owned(),
        ),
        ("Prompt capture", doctor_prompt_capture_status(checks)),
        (
            "Watcher",
            doctor_watcher_detail_text(checks, "watcher_status", "not_checked"),
        ),
        (
            "Watcher scan",
            format!(
                "files scanned {}; files skipped {}; unreadable paths {}; not full filesystem monitoring {}",
                doctor_watcher_scan_u64_text(checks, "files_scanned"),
                doctor_watcher_scan_u64_text(checks, "files_skipped"),
                doctor_watcher_scan_u64_text(checks, "unreadable_paths_count"),
                yes_no(doctor_watcher_not_full_filesystem_monitoring(checks)),
            ),
        ),
        (
            "Host reload",
            yes_no(doctor_host_reload_required(checks, actions)).to_owned(),
        ),
    ]
}

fn append_doctor_next_actions(
    output: &mut String,
    status: CommandStatus,
    actions: &[DiagnosticAction],
) {
    output.push_str("\nNext:\n");
    if actions.is_empty() {
        output.push_str("  none\n");
        return;
    }
    for (index, action) in actions.iter().enumerate() {
        let prefix = if status == CommandStatus::Complete {
            "Recommended: "
        } else {
            ""
        };
        output.push_str(&format!(
            "  {}. {}{}\n",
            index + 1,
            prefix,
            trimmed_sentence(&action.instruction)
        ));
        if let Some(command) = &action.command {
            output.push_str(&format!("     Run:\n       {command}\n"));
        }
    }
}

fn doctor_next_summary_text(status: CommandStatus, actions: &[DiagnosticAction]) -> String {
    match actions.first() {
        Some(action) if status == CommandStatus::Complete => {
            format!("recommended: {}", trimmed_sentence(&action.instruction))
        }
        Some(action) => trimmed_sentence(&action.instruction).to_owned(),
        None => "none".to_owned(),
    }
}

fn display_state_text(value: &str) -> String {
    value.replace('_', " ")
}

fn trimmed_sentence(value: &str) -> &str {
    value.trim().trim_end_matches('.')
}

fn doctor_states_json(
    runtime_home: &Path,
    checks: &[DiagnosticCheck],
    actions: &[DiagnosticAction],
) -> Value {
    let mut states = json!({
        "build_id": volicord_mcp::build_id(),
        "runtime_home": doctor_runtime_home_state(runtime_home, checks),
        "installation_profile": doctor_installation_profile_state(checks),
        "command_availability": doctor_command_state(checks),
        "project_registration": doctor_count_state(checks, "projects", "registered"),
        "connection": doctor_count_state(checks, "connections", "stored"),
        "mcp_config": doctor_mcp_config_state(checks),
        "guard_installation": doctor_count_state(checks, "guard_installations", "stored"),
        "selected_profile": doctor_selected_profile_from_checks(checks),
        "control_surface": doctor_control_surface_value(checks),
        "cooperative_pre_tool_warning_available": doctor_control_surface_bool(checks, "cooperative_pre_tool_warning_available"),
        "cooperative_pre_tool_denial_available": doctor_control_surface_bool(checks, "cooperative_pre_tool_denial_available"),
        "post_tool_correlation_available": doctor_host_hook_guard_available(checks),
        "bypass_detection_active": false,
        "prompt_capture_available": doctor_prompt_capture_available(checks),
        "local_web_consent_available": false,
        "guard_configuration": doctor_check_state(checks, "guard_required_hooks_supported"),
        "guard_observation": doctor_check_state(checks, "guard_hook_observed"),
        "guard_effective": doctor_check_state(checks, "guard_status_active"),
        "guard_files": doctor_check_state(checks, "guard_files_installed"),
        "agents_managed_block": doctor_guard_file_kind_state(checks, "agents_managed_block"),
        "volicord_policy_file": doctor_guard_file_kind_state(checks, "volicord_policy"),
        "rule_instruction_config": doctor_guard_file_kind_state(checks, "host_rule_instruction"),
        "hook_config": doctor_guard_file_kind_state(checks, "host_hook_config"),
        "required_hook_phases": doctor_required_hook_phases_state(checks),
        "missing_required_hooks": doctor_missing_required_hooks_value(checks),
        "guard_hook_observed": doctor_check_state(checks, "guard_hook_observed"),
        "guard_status": doctor_check_state(checks, "guard_status_active"),
        "prompt_capture": doctor_prompt_capture_health(checks),
        "prompt_capture_status": doctor_prompt_capture_status(checks),
        "watcher_status": doctor_watcher_detail_value(checks, "watcher_status"),
        "watcher_baseline_created_at": doctor_watcher_detail_value(checks, "baseline_created_at"),
        "watcher_coverage_start_at": doctor_watcher_detail_value(checks, "coverage_start_at"),
        "watcher_coverage_basis": doctor_watcher_detail_value(checks, "coverage_basis"),
        "watcher_partial_coverage_warning": doctor_watcher_detail_value(checks, "partial_coverage_warning"),
        "watcher_scan_summary": doctor_watcher_scan_summary_value(checks),
        "host_reload_required": doctor_host_reload_required(checks, actions),
    });
    if let Some(object) = states.as_object_mut() {
        object.insert(
            "generated_config_verified".to_owned(),
            Value::Bool(doctor_generated_config_verified_state(checks)),
        );
        object.insert(
            "hook_path_safety".to_owned(),
            Value::String(doctor_hook_path_safety_state(checks)),
        );
        object.insert(
            "hook_commands_cwd_independent".to_owned(),
            Value::Bool(doctor_hook_commands_cwd_independent(checks)),
        );
        object.insert(
            "hook_commands_subdirectory_safe".to_owned(),
            Value::Bool(doctor_hook_commands_subdirectory_safe(checks)),
        );
        object.insert(
            "native_host_output_adapter_verified".to_owned(),
            Value::Bool(doctor_native_host_output_adapter_verified(checks)),
        );
        object.insert(
            "bash_shell_mutation_coverage".to_owned(),
            Value::Bool(doctor_bash_shell_mutation_coverage(checks)),
        );
        object.insert(
            "direct_file_write_matcher_coverage".to_owned(),
            Value::Bool(doctor_direct_file_write_matcher_coverage(checks)),
        );
    }
    states
}

fn doctor_watcher_details(checks: &[DiagnosticCheck]) -> Option<&Value> {
    checks
        .iter()
        .find(|check| check.id == "watcher_scan_summary")
        .and_then(|check| check.details.as_ref())
}

fn doctor_watcher_detail_value(checks: &[DiagnosticCheck], key: &str) -> Value {
    doctor_watcher_details(checks)
        .and_then(|details| details.get(key))
        .cloned()
        .unwrap_or(Value::Null)
}

fn doctor_watcher_detail_text(checks: &[DiagnosticCheck], key: &str, fallback: &str) -> String {
    match doctor_watcher_detail_value(checks, key) {
        Value::String(value) if !value.trim().is_empty() => value,
        Value::Array(values) if !values.is_empty() => values
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(","),
        Value::Null => fallback.to_owned(),
        value => value.to_string(),
    }
}

fn doctor_watcher_scan_summary_value(checks: &[DiagnosticCheck]) -> Value {
    doctor_watcher_details(checks)
        .and_then(|details| details.get("scan_summary"))
        .cloned()
        .unwrap_or(Value::Null)
}

fn doctor_watcher_scan_u64_text(checks: &[DiagnosticCheck], key: &str) -> String {
    let summary = doctor_watcher_scan_summary_value(checks);
    summary
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn doctor_watcher_not_full_filesystem_monitoring(checks: &[DiagnosticCheck]) -> bool {
    let summary = doctor_watcher_scan_summary_value(checks);
    summary
        .get("not_full_filesystem_monitoring")
        .and_then(Value::as_bool)
        .unwrap_or(false)
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

fn doctor_guard_file_bool_detail(checks: &[DiagnosticCheck], key: &str) -> bool {
    checks
        .iter()
        .find(|check| check.id == "guard_files_installed")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn doctor_generated_config_verified_state(checks: &[DiagnosticCheck]) -> bool {
    doctor_guard_file_bool_detail(checks, "generated_config_verified")
}

fn doctor_hook_path_safety_state(checks: &[DiagnosticCheck]) -> String {
    checks
        .iter()
        .find(|check| check.id == "guard_files_installed")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("hook_path_safety"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| "not_checked".to_owned())
}

fn doctor_hook_commands_cwd_independent(checks: &[DiagnosticCheck]) -> bool {
    doctor_guard_file_bool_detail(checks, "hook_commands_cwd_independent")
}

fn doctor_hook_commands_subdirectory_safe(checks: &[DiagnosticCheck]) -> bool {
    doctor_guard_file_bool_detail(checks, "hook_commands_subdirectory_safe")
}

fn doctor_native_host_output_adapter_verified(checks: &[DiagnosticCheck]) -> bool {
    doctor_guard_file_bool_detail(checks, "native_host_output_adapter_verified")
}

fn doctor_bash_shell_mutation_coverage(checks: &[DiagnosticCheck]) -> bool {
    doctor_guard_file_bool_detail(checks, "bash_shell_mutation_coverage")
}

fn doctor_direct_file_write_matcher_coverage(checks: &[DiagnosticCheck]) -> bool {
    doctor_guard_file_bool_detail(checks, "direct_file_write_matcher_coverage")
}

fn doctor_selected_profile_from_checks(checks: &[DiagnosticCheck]) -> String {
    checks
        .iter()
        .find(|check| check.id == "control_surface")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("selected_profile"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| match check_status(checks, "control_surface") {
            Some("skipped") | None => "not_checked".to_owned(),
            _ => "unknown".to_owned(),
        })
}

fn doctor_control_surface_value(checks: &[DiagnosticCheck]) -> Value {
    checks
        .iter()
        .find(|check| check.id == "control_surface")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("control_surface"))
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "selected_profile": doctor_selected_profile_from_checks(checks),
                "host_hooks_active": false,
                "session_watcher_active": false,
                "cooperative_pre_tool_warning_available": false,
                "cooperative_pre_tool_denial_available": false,
                "unrecorded_changes_detectable": false,
                "actor_identity_provable": false,
                "os_enforced": false,
            })
        })
}

fn doctor_control_surface_bool(checks: &[DiagnosticCheck], key: &str) -> bool {
    doctor_control_surface_value(checks)
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn doctor_required_hook_phases_state(checks: &[DiagnosticCheck]) -> &'static str {
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

fn doctor_host_hook_guard_available(checks: &[DiagnosticCheck]) -> bool {
    doctor_control_surface_bool(checks, "host_hooks_active")
}

fn doctor_prompt_capture_available(checks: &[DiagnosticCheck]) -> bool {
    matches!(
        doctor_prompt_capture_status(checks).as_str(),
        "available" | "configured_unobserved"
    )
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

fn doctor_summary_card(
    status: CommandStatus,
    checks: &[DiagnosticCheck],
    actions: &[DiagnosticAction],
) -> SummaryCard {
    SummaryCard {
        task: "not_selected".to_owned(),
        recording: "diagnostic_observation".to_owned(),
        profile: doctor_selected_profile_from_checks(checks),
        write_ticket: "not_selected".to_owned(),
        evidence: "not_selected".to_owned(),
        user_judgment: "not_selected".to_owned(),
        changes: "not_selected".to_owned(),
        close_status: "not_selected".to_owned(),
        transport: "local CLI".to_owned(),
        next: match actions.first() {
            Some(action) if status == CommandStatus::Complete => {
                format!("recommended: {}", action.instruction)
            }
            Some(action) => action.instruction.clone(),
            None => "none".to_owned(),
        },
        next_action: None,
        guarantee: DIAGNOSTIC_SUMMARY_GUARANTEE.to_owned(),
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
        instruction: "Initialize the primary host connection from the Product Repository."
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
                "Install the volicord executable on PATH or update PATH so volicord resolves to the installation profile command; restart or reload existing agent hosts after PATH or command-link changes."
                    .to_owned(),
            command: None,
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
