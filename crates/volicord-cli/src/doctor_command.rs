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
    core_pipeline::CoreProjectStore,
    guards::{guard_installation, guard_observation_summary},
    inspection::{
        inspect_runtime_home, DatabaseInspection, InspectionSchemaState,
        InstallationProfileInspectionRecord, RegistryInspectionSnapshot, RuntimeHomeInspection,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError, StoreFailureRoute,
};
use volicord_types::{
    canonical_json_sha256, guard_manifest_from_json, GuardHookPhase, IntegrationProfile, ProjectId,
    SummaryCard,
};

use crate::{
    cli::DoctorArgs,
    guard_integration::audit::{
        all_recorded_values_true, guard_file_findings_for_inspection,
        guard_manifest_binding_valid_for_inspection, missing_required_hooks_from_manifest_json,
        GuardFileFindings,
    },
    guard_integration::git_exclude::{always_local_paths, git_exclude_path, personal_only_paths},
    guard_integration::policy::validate_policy_schema,
    host_integration::HostKind,
    policy_command::read_validated_policy_file,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectPolicyAuthorityState {
    Matches,
    AuthorityMissing,
    AuthorityCorrupt,
    AuthorityUnavailable,
    ManagedFileMissing,
    ManagedFileInvalid,
    ManagedFileUnavailable,
    ManagedFileStale,
}

impl ProjectPolicyAuthorityState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Matches => "matches",
            Self::AuthorityMissing => "authority_missing",
            Self::AuthorityCorrupt => "authority_corrupt",
            Self::AuthorityUnavailable => "authority_unavailable",
            Self::ManagedFileMissing => "managed_file_missing",
            Self::ManagedFileInvalid => "managed_file_invalid",
            Self::ManagedFileUnavailable => "managed_file_unavailable",
            Self::ManagedFileStale => "managed_file_stale",
        }
    }
}

pub fn run_doctor_command<F>(
    args: DoctorArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<CommandOutcome, DoctorCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let options = DoctorOptions {
        output: if args.json {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
        privacy_footprint: args.privacy_footprint,
    };
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
    let mut absent_profile_state = "missing";
    let mut absent_profile_summary = "installation profile is missing";
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
            inspect_integration_intent_drift(snapshot, &mut checks, &mut actions);
            inspect_project_policy_authority(&runtime_home, snapshot, &mut checks, &mut actions);
        }
        DatabaseInspection::Unsupported { path, detail } => {
            absent_profile_state = "corrupt";
            absent_profile_summary =
                "installation profile cannot be read from an invalid registry schema";
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
            absent_profile_state = "corrupt";
            absent_profile_summary = "installation profile cannot be read from a corrupt registry";
            checks.push(
                DiagnosticCheck::failed("registry", "Runtime Home registry is malformed")
                    .with_details(json!({ "path": path_text(path), "detail": detail })),
            );
        }
        DatabaseInspection::Unreadable { path, detail } => {
            absent_profile_state = "unavailable";
            absent_profile_summary = "installation profile is currently unavailable";
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
            DiagnosticCheck::failed("installation_profile", absent_profile_summary).with_details(
                json!({
                    "state": absent_profile_state,
                    "runtime_home": path_text(&runtime_home),
                }),
            ),
        );
        if absent_profile_state == "missing"
            && !actions.iter().any(|action| action.id == "run_init")
        {
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

    checks.push(host_detection_check());
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
        "Codex Record Guard installation records, capability metadata, policy hashes, hook observation timestamps, and prompt-capture availability state",
        "project state records for tasks, change units, write tickets, evidence metadata, close-readiness records, User Channel actions, and artifacts when those features are used",
        "bounded diagnostics.sqlite session, connection, project, transport, host, build, tool, categorical outcome, counter, byte-size, and latency observations when diagnostics are present",
    ]
}

fn privacy_does_not_store() -> Vec<&'static str> {
    vec![
        "Guard prompt-capture observations do not include raw prompt text by default",
        "diagnostics.sqlite does not store prompt bodies, Product Repository paths or file contents, error bodies, secrets, or user-action question, choice, rationale, note, or observation-summary text",
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
        match local_policy_connection_intent(&project.repo_root) {
            Ok(intent) => {
                let personal = intent == "personal";
                effective_personal_project_count += usize::from(personal);
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
            }
        }
        // Audit both intent projections regardless of the policy's current
        // intent. A failed or interrupted migration can leave an opposite-
        // intent local file behind, and that file must not become trackable.
        let local_paths = always_local_paths()
            .iter()
            .copied()
            .chain(personal_only_paths().iter().copied());
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalPolicyAudit {
    host: String,
    connection_intent: String,
    selected_profile: String,
    connection_id: String,
    guard_installation_id: String,
}

fn local_policy_connection_intent(repo_root: &Path) -> Result<String, String> {
    local_policy_audit(repo_root).map(|policy| policy.connection_intent)
}

fn local_policy_audit(repo_root: &Path) -> Result<LocalPolicyAudit, String> {
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
    let required = |field: &str| {
        policy
            .get(field)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("the local policy is missing {field}"))
    };
    Ok(LocalPolicyAudit {
        host: required("host")?,
        connection_intent: connection_intent.to_owned(),
        selected_profile: required("selected_profile")?,
        connection_id: required("connection_id")?,
        guard_installation_id: required("guard_installation_id")?,
    })
}

const MAX_INTENT_DRIFT_FINDINGS: usize = 64;

fn inspect_integration_intent_drift(
    snapshot: &RegistryInspectionSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    actions: &mut Vec<DiagnosticAction>,
) {
    let mut projects = connected_enabled_projects(snapshot);
    projects.sort_by(|left, right| left.project_id.cmp(&right.project_id));
    if projects.is_empty() {
        checks.push(DiagnosticCheck::skipped(
            "integration_intent_drift",
            "no enabled repository integration is recorded",
        ));
        return;
    }

    let connected_project_count = projects.len();
    let mut truncated = projects.len() > MAX_PERSONAL_GIT_PROJECTS;
    projects.truncate(MAX_PERSONAL_GIT_PROJECTS);
    let mut findings = Vec::new();
    let mut audit_errors = Vec::new();
    let active_installations = active_guard_installations(snapshot);
    let mut first_repair_command = None;

    for project in &projects {
        let policy = match local_policy_audit(&project.repo_root) {
            Ok(policy) => policy,
            Err(detail) => {
                push_bounded_intent_finding(
                    &mut audit_errors,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "detail": detail,
                    }),
                    &mut truncated,
                );
                continue;
            }
        };
        first_repair_command.get_or_insert_with(|| integration_repair_command(&policy, project));

        if policy.host != HostKind::Codex.as_str()
            || policy.selected_profile != IntegrationProfile::Record.as_str()
        {
            push_bounded_intent_finding(
                &mut findings,
                json!({
                    "project_id": project.project_id,
                    "repo_root": path_text(&project.repo_root),
                    "kind": "policy_scope_mismatch",
                    "policy_host": policy.host,
                    "selected_profile": policy.selected_profile,
                    "expected_host": HostKind::Codex.as_str(),
                    "expected_profile": IntegrationProfile::Record.as_str(),
                }),
                &mut truncated,
            );
        }

        let attached = snapshot
            .agent_connections
            .iter()
            .filter(|connection| {
                connection.enabled
                    && connection.host_kind == HostKind::Codex.as_str()
                    && matches!(connection.intent.as_str(), "personal" | "shared")
                    && connection_is_attached_to_project(snapshot, connection, project)
            })
            .collect::<Vec<_>>();
        let expected = attached
            .iter()
            .find(|connection| connection.connection_internal_id == policy.connection_id);
        match expected {
            None => push_bounded_intent_finding(
                &mut findings,
                json!({
                    "project_id": project.project_id,
                    "repo_root": path_text(&project.repo_root),
                    "kind": "policy_connection_missing",
                    "policy_connection_id": policy.connection_id,
                    "policy_intent": policy.connection_intent,
                    "host": policy.host,
                    "policy_host": policy.host,
                }),
                &mut truncated,
            ),
            Some(connection) if connection.host_kind != HostKind::Codex.as_str() => {
                push_bounded_intent_finding(
                    &mut findings,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "kind": "policy_connection_host_mismatch",
                        "policy_connection_id": policy.connection_id,
                        "policy_host": policy.host,
                        "recorded_host": connection.host_kind,
                    }),
                    &mut truncated,
                );
            }
            Some(connection) if connection.intent != policy.connection_intent => {
                push_bounded_intent_finding(
                    &mut findings,
                    json!({
                        "project_id": project.project_id,
                        "repo_root": path_text(&project.repo_root),
                        "kind": "policy_connection_intent_mismatch",
                        "policy_intent": policy.connection_intent,
                        "recorded_intent": connection.intent,
                        "host": policy.host,
                        "policy_host": policy.host,
                    }),
                    &mut truncated,
                );
            }
            Some(_) => {}
        }
        for connection in attached.iter().filter(|connection| {
            connection.connection_internal_id != policy.connection_id
                || connection.intent != policy.connection_intent
        }) {
            push_bounded_intent_finding(
                &mut findings,
                json!({
                    "project_id": project.project_id,
                    "repo_root": path_text(&project.repo_root),
                    "kind": "additional_active_intent_projection",
                    "policy_connection_id": policy.connection_id,
                    "connection_id": connection.connection_internal_id,
                    "policy_intent": policy.connection_intent,
                    "recorded_intent": connection.intent,
                    "host": HostKind::Codex.as_str(),
                }),
                &mut truncated,
            );
        }

        let guard_matches = active_installations.iter().any(|installation| {
            let manifest = guard_manifest_from_json(&installation.manifest_json).ok();
            installation.guard_installation_id == policy.guard_installation_id
                && installation.connection_internal_id == policy.connection_id
                && manifest.as_ref().is_some_and(|manifest| {
                    manifest.integration_profile == IntegrationProfile::Record
                })
                && installation.project_internal_id == project.project_internal_id
        });
        if !guard_matches {
            push_bounded_intent_finding(
                &mut findings,
                json!({
                    "project_id": project.project_id,
                    "repo_root": path_text(&project.repo_root),
                    "kind": "policy_guard_inventory_mismatch",
                    "guard_installation_id": policy.guard_installation_id,
                    "selected_profile": policy.selected_profile,
                    "host": policy.host,
                }),
                &mut truncated,
            );
        }
    }

    let has_warning = !findings.is_empty() || !audit_errors.is_empty() || truncated;
    let details = json!({
        "connected_project_count": connected_project_count,
        "audited_project_count": projects.len(),
        "findings": findings,
        "audit_errors": audit_errors,
        "truncated": truncated,
        "max_projects": MAX_PERSONAL_GIT_PROJECTS,
        "max_findings": MAX_INTENT_DRIFT_FINDINGS,
    });
    if has_warning {
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_integration_intent_drift".to_owned(),
                instruction: "Rerun Codex Record init with the intended connection intent so the local policy and enabled inventory converge."
                    .to_owned(),
                command: first_repair_command,
            },
        );
        checks.push(
            DiagnosticCheck::warning(
                "integration_intent_drift",
                "one or more Codex Record repository integrations have intent or inventory drift",
            )
            .with_details(details),
        );
    } else {
        checks.push(
            DiagnosticCheck::passed(
                "integration_intent_drift",
                "Codex Record repository integration intent and inventory are converged",
            )
            .with_details(details),
        );
    }
}

fn connected_enabled_projects(
    snapshot: &RegistryInspectionSnapshot,
) -> Vec<&volicord_store::inspection::ProjectInspectionRecord> {
    snapshot
        .projects
        .iter()
        .filter(|project| {
            snapshot.agent_connections.iter().any(|connection| {
                connection.enabled
                    && connection.host_kind == HostKind::Codex.as_str()
                    && matches!(connection.intent.as_str(), "personal" | "shared")
                    && connection_is_attached_to_project(snapshot, connection, project)
            })
        })
        .collect()
}

fn inspect_project_policy_authority(
    runtime_home: &Path,
    snapshot: &RegistryInspectionSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    actions: &mut Vec<DiagnosticAction>,
) {
    let mut projects = connected_enabled_projects(snapshot);
    projects.sort_by(|left, right| left.project_id.cmp(&right.project_id));
    let project_count = projects.len();
    let mut truncated = project_count > MAX_PERSONAL_GIT_PROJECTS;
    projects.truncate(MAX_PERSONAL_GIT_PROJECTS);
    let mut findings = Vec::new();
    for project in projects {
        let file_path = project.repo_root.join(".volicord/policy.json");
        let state = project_policy_authority_state(runtime_home, project, &file_path);
        if state != ProjectPolicyAuthorityState::Matches {
            truncated |= findings.len() >= MAX_INTENT_DRIFT_FINDINGS;
            if findings.len() < MAX_INTENT_DRIFT_FINDINGS {
                findings.push(json!({
                    "project_id": project.project_id,
                    "repo_root": path_text(&project.repo_root),
                    "status": state.as_str(),
                }));
            }
            push_unique_diagnostic_action(
                actions,
                DiagnosticAction {
                    id: "repair_project_policy".to_owned(),
                    instruction: "Inspect the authoritative project policy and apply one validated canonical policy file to repair the database/file mismatch."
                        .to_owned(),
                    command: Some(format!(
                        "volicord policy show --repo {} --json",
                        doctor_shell_word(&path_text(&project.repo_root))
                    )),
                },
            );
        }
    }
    let details = json!({
        "project_count": project_count,
        "findings": findings,
        "truncated": truncated,
        "scan_state": if truncated { "bounded_incomplete" } else { "complete" },
    });
    let finding_count = details["findings"].as_array().map_or(0, Vec::len);
    match project_policy_authority_result(finding_count, truncated) {
        "passed" => checks.push(
            DiagnosticCheck::passed(
                "project_policy_authority",
                "managed project policies match authoritative database fingerprints",
            )
            .with_details(details),
        ),
        "warning" => checks.push(
            DiagnosticCheck::warning(
                "project_policy_authority",
                "the bounded project-policy audit did not inspect every connected project",
            )
            .with_details(details),
        ),
        _ => checks.push(
            DiagnosticCheck::failed(
                "project_policy_authority",
                "one or more managed project policies do not match database authority",
            )
            .with_details(details),
        ),
    }
}

fn project_policy_authority_result(finding_count: usize, truncated: bool) -> &'static str {
    if finding_count > 0 {
        "failed"
    } else if truncated {
        "warning"
    } else {
        "passed"
    }
}

fn project_policy_authority_state(
    runtime_home: &Path,
    project: &volicord_store::inspection::ProjectInspectionRecord,
    file_path: &Path,
) -> ProjectPolicyAuthorityState {
    let store = match CoreProjectStore::open_read_only(
        runtime_home,
        &ProjectId::new(project.project_id.clone()),
    ) {
        Ok(store) => store,
        Err(error) => return project_policy_store_failure_state(&error),
    };
    let authority = match store.project_workflow_policy() {
        Ok(Some(authority)) => authority,
        Ok(None) => return ProjectPolicyAuthorityState::AuthorityMissing,
        Err(error) => return project_policy_store_failure_state(&error),
    };
    if authority.policy_schema != volicord_types::WORKFLOW_POLICY_CONTRACT_ID
        || authority.source != "project_database"
    {
        return ProjectPolicyAuthorityState::AuthorityCorrupt;
    }
    let authority_value = match serde_json::from_str::<Value>(&authority.policy_json) {
        Ok(value) => value,
        Err(_) => return ProjectPolicyAuthorityState::AuthorityCorrupt,
    };
    let Some(connection_intent) = authority_value
        .get("connection_intent")
        .and_then(Value::as_str)
    else {
        return ProjectPolicyAuthorityState::AuthorityCorrupt;
    };
    if validate_policy_schema(&authority_value, connection_intent).is_err() {
        return ProjectPolicyAuthorityState::AuthorityCorrupt;
    }
    let authority_fingerprint = match canonical_json_sha256(&authority_value) {
        Ok(fingerprint) => fingerprint,
        Err(_) => return ProjectPolicyAuthorityState::AuthorityCorrupt,
    };
    if authority_fingerprint.as_str() != authority.policy_fingerprint {
        return ProjectPolicyAuthorityState::AuthorityCorrupt;
    }

    match fs::symlink_metadata(file_path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ProjectPolicyAuthorityState::ManagedFileMissing;
        }
        Err(_) => return ProjectPolicyAuthorityState::ManagedFileUnavailable,
        Ok(_) => {}
    }
    let managed = match read_validated_policy_file(file_path) {
        Ok(managed) => managed,
        Err(crate::policy_command::PolicyCommandError::Validation { .. }) => {
            return ProjectPolicyAuthorityState::ManagedFileInvalid;
        }
        Err(_) => return ProjectPolicyAuthorityState::ManagedFileUnavailable,
    };
    if managed.fingerprint == authority.policy_fingerprint {
        ProjectPolicyAuthorityState::Matches
    } else {
        ProjectPolicyAuthorityState::ManagedFileStale
    }
}

fn project_policy_store_failure_state(error: &StoreError) -> ProjectPolicyAuthorityState {
    match error.classification().route {
        StoreFailureRoute::PersistedDataCorrupt => ProjectPolicyAuthorityState::AuthorityCorrupt,
        StoreFailureRoute::OperationalUnavailable
        | StoreFailureRoute::InvalidEnvironment
        | StoreFailureRoute::InvocationContextMismatch => {
            ProjectPolicyAuthorityState::AuthorityUnavailable
        }
    }
}

fn connection_is_attached_to_project(
    snapshot: &RegistryInspectionSnapshot,
    connection: &volicord_store::inspection::AgentConnectionInspectionRecord,
    project: &volicord_store::inspection::ProjectInspectionRecord,
) -> bool {
    connection.project_internal_id.as_deref() == Some(project.project_internal_id.as_str())
        || snapshot.connection_projects.iter().any(|membership| {
            membership.connection_internal_id == connection.connection_internal_id
                && membership.project_internal_id == project.project_internal_id
        })
}

fn active_guard_installations(
    snapshot: &RegistryInspectionSnapshot,
) -> Vec<volicord_store::inspection::GuardInstallationInspectionRecord> {
    snapshot
        .guard_installations
        .iter()
        .filter(|installation| {
            let Some(connection) = snapshot.agent_connections.iter().find(|connection| {
                connection.enabled
                    && connection.connection_internal_id == installation.connection_internal_id
            }) else {
                return false;
            };
            {
                let project_internal_id = installation.project_internal_id.as_str();
                connection.project_internal_id.as_deref() == Some(project_internal_id)
                    || snapshot.connection_projects.iter().any(|membership| {
                        membership.connection_internal_id == connection.connection_internal_id
                            && membership.project_internal_id == project_internal_id
                    })
            }
        })
        .cloned()
        .collect()
}

fn integration_repair_command(
    policy: &LocalPolicyAudit,
    project: &volicord_store::inspection::ProjectInspectionRecord,
) -> String {
    let shared = if policy.connection_intent == "shared" {
        " --shared"
    } else {
        ""
    };
    format!(
        "volicord init --host codex --repo {}{} --profile record",
        doctor_shell_word(&path_text(&project.repo_root)),
        shared,
    )
}

fn doctor_shell_word(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&byte))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn push_bounded_intent_finding(values: &mut Vec<Value>, value: Value, truncated: &mut bool) {
    if values.len() < MAX_INTENT_DRIFT_FINDINGS {
        values.push(value);
    } else {
        *truncated = true;
    }
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
    let installations = active_guard_installations(snapshot);
    if installations.is_empty() {
        for (id, summary) in [
            ("guard_files", "no Codex Record Guard manifest is recorded"),
            ("guard_observation", "no Guard observation is recorded"),
        ] {
            checks.push(DiagnosticCheck::skipped(id, summary));
        }
        checks.push(
            DiagnosticCheck::skipped("control_surface", "no integration profile is recorded")
                .with_details(json!({
                    "selected_profile": "not_checked",
                    "control_surface": {
                        "selected_profile": "not_checked",
                        "host_hooks_active": false,
                        "cooperative_pre_tool_warning_available": false,
                        "cooperative_pre_tool_denial_available": false,
                        "unrecorded_changes_detectable": false,
                        "actor_identity_provable": false,
                        "os_enforced": false,
                    },
                })),
        );
        return;
    }

    let invalid_scope_count = installations
        .iter()
        .filter(|installation| {
            guard_manifest_from_json(&installation.manifest_json).is_err_and(|_| true)
                || guard_manifest_from_json(&installation.manifest_json).is_ok_and(|manifest| {
                    manifest.host_kind.as_str() != HostKind::Codex.as_str()
                        || manifest.integration_profile != IntegrationProfile::Record
                })
        })
        .count();
    let binding_valid_installations = installations
        .iter()
        .filter(|installation| {
            snapshot
                .agent_connections
                .iter()
                .find(|connection| {
                    connection.connection_internal_id == installation.connection_internal_id
                })
                .is_some_and(|connection| {
                    guard_manifest_binding_valid_for_inspection(
                        installation,
                        connection,
                        &snapshot.projects,
                    )
                })
        })
        .collect::<Vec<_>>();
    let binding_invalid_count = installations.len() - binding_valid_installations.len();

    let mut file_findings = GuardFileFindings::default();
    for installation in &installations {
        let connection = snapshot
            .agent_connections
            .iter()
            .find(|connection| {
                connection.connection_internal_id == installation.connection_internal_id
            })
            .expect("validated inspection snapshot retains the Guard owner");
        file_findings.merge(guard_file_findings_for_inspection(
            installation,
            connection,
            &snapshot.projects,
        ));
    }
    file_findings.sort_dedup();

    let missing_required_hooks = binding_valid_installations
        .iter()
        .flat_map(|installation| {
            missing_required_hooks_from_manifest_json(&installation.manifest_json)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let observation_summaries = binding_valid_installations
        .iter()
        .filter_map(|installation| guard_observation(snapshot, installation))
        .collect::<Vec<_>>();
    let observed_count = observation_summaries
        .iter()
        .filter(|summary| summary.all_required_phases_observed())
        .count();
    let incompatible_observations = observation_summaries
        .iter()
        .map(|summary| summary.incompatible_event_ids.len())
        .sum::<usize>();
    let prompt_capture_configured = binding_valid_installations
        .iter()
        .filter(|installation| guard_prompt_capture_configured(&installation.manifest_json))
        .count();
    let prompt_capture_observed = observation_summaries
        .iter()
        .filter(|summary| summary.prompt_capture_observed())
        .count();
    let host_hooks_active = invalid_scope_count == 0
        && binding_invalid_count == 0
        && missing_required_hooks.is_empty()
        && observed_count == installations.len()
        && installations
            .iter()
            .all(|installation| guard_effective_active(snapshot, installation))
        && file_findings.generated_config_verified()
        && file_findings.direct_file_write_matcher_coverage();
    let selected_profile = if invalid_scope_count == 0 {
        IntegrationProfile::Record.as_str()
    } else {
        "invalid"
    };
    let control_check = if invalid_scope_count == 0 {
        DiagnosticCheck::passed("control_surface", "selected integration profile is record")
    } else {
        DiagnosticCheck::warning(
            "control_surface",
            "one or more Guard records are outside the Codex Record release scope",
        )
    };
    checks.push(control_check.with_details(json!({
        "selected_profile": selected_profile,
        "control_surface": {
            "selected_profile": selected_profile,
            "host_hooks_active": host_hooks_active,
            "cooperative_pre_tool_warning_available": host_hooks_active,
            "cooperative_pre_tool_denial_available": host_hooks_active,
            "unrecorded_changes_detectable": host_hooks_active,
            "actor_identity_provable": false,
            "os_enforced": false,
        },
    })));

    let file_problem = invalid_scope_count > 0
        || binding_invalid_count > 0
        || !missing_required_hooks.is_empty()
        || !file_findings.missing_files.is_empty()
        || !file_findings.stale_files.is_empty()
        || !file_findings.broken_files.is_empty();
    let file_check = if file_problem {
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_guard_files".to_owned(),
                instruction: "Reinstall the Codex Record Guard files for the affected repository."
                    .to_owned(),
                command: Some("volicord init --host codex --repo PATH --profile record".to_owned()),
            },
        );
        DiagnosticCheck::failed(
            "guard_files",
            "one or more Codex Record Guard files are missing, stale, or broken",
        )
    } else {
        DiagnosticCheck::passed("guard_files", "Codex Record Guard files are installed")
    };
    let mut file_details = doctor_guard_file_details(&file_findings);
    if let Some(details) = file_details.as_object_mut() {
        details.insert(
            "missing_required_hooks".to_owned(),
            json!(missing_required_hooks),
        );
        details.insert("binding_invalid".to_owned(), json!(binding_invalid_count));
        details.insert(
            "outside_release_scope".to_owned(),
            json!(invalid_scope_count),
        );
    }
    checks.push(file_check.with_details(file_details));

    if !matches!(
        file_findings.hook_path_safety_state().as_str(),
        "ok" | "not_recorded" | "not_checked" | "not_applicable"
    ) {
        push_unique_diagnostic_action(
            actions,
            DiagnosticAction {
                id: "repair_guard_hook_path_safety".to_owned(),
                instruction:
                    "Regenerate cwd-independent Codex Record Guard commands for the affected repository."
                        .to_owned(),
                command: Some(
                    "volicord init --host codex --repo PATH --profile record".to_owned(),
                ),
            },
        );
    }

    if binding_invalid_count == 0
        && incompatible_observations == 0
        && observed_count == installations.len()
    {
        checks.push(
            DiagnosticCheck::passed(
                "guard_observation",
                "all current Codex Record Guard hook phases were observed",
            )
            .with_details(json!({
                "observed": observed_count,
                "installations": installations.len(),
                "incompatible_events": incompatible_observations,
                "prompt_capture_configured": prompt_capture_configured,
                "prompt_capture_observed": prompt_capture_observed,
            })),
        );
    } else {
        let observation_check = if incompatible_observations > 0 {
            DiagnosticCheck::failed(
                "guard_observation",
                "a current Guard event reported a malformed or incompatible hook contract",
            )
        } else {
            DiagnosticCheck::warning(
                "guard_observation",
                "one or more Codex Record Guard installations are awaiting current hook observations",
            )
        };
        checks.push(observation_check.with_details(json!({
            "observed": observed_count,
            "installations": installations.len(),
            "binding_invalid": binding_invalid_count,
            "incompatible_events": incompatible_observations,
            "prompt_capture_configured": prompt_capture_configured,
            "prompt_capture_observed": prompt_capture_observed,
        })));
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
        "hook_path_safety": findings.hook_path_safety_state(),
        "hook_commands_cwd_independent": all_recorded_values_true(&findings.hook_cwd_independent_values),
        "hook_commands_subdirectory_safe": all_recorded_values_true(&findings.hook_subdirectory_safe_values),
        "hook_path_safety_details": &findings.hook_path_safety_details,
        "direct_file_write_matcher_coverage": findings.direct_file_write_matcher_coverage(),
    })
}

fn doctor_guard_file_states(findings: &GuardFileFindings) -> BTreeMap<String, String> {
    let mut states = findings.file_kind_states.clone();
    if findings
        .broken_files
        .iter()
        .any(|file| file == "manifest_json")
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

fn guard_observation(
    snapshot: &RegistryInspectionSnapshot,
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> Option<volicord_store::guards::GuardObservationSummary> {
    guard_installation(
        &snapshot.runtime_home.runtime_home_path,
        &installation.guard_installation_id,
    )
    .ok()
    .flatten()
    .and_then(|record| {
        guard_observation_summary(
            &snapshot.runtime_home.runtime_home_path,
            &installation.project_id,
            &record,
        )
        .ok()
    })
}

fn guard_observation_current(
    snapshot: &RegistryInspectionSnapshot,
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> bool {
    guard_observation(snapshot, installation)
        .is_some_and(|summary| summary.all_required_phases_observed())
}

fn guard_configuration_healthy(
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> bool {
    guard_manifest_from_json(&installation.manifest_json).is_ok()
        && missing_required_hooks_from_manifest_json(&installation.manifest_json).is_empty()
}

fn guard_effective_active(
    snapshot: &RegistryInspectionSnapshot,
    installation: &volicord_store::inspection::GuardInstallationInspectionRecord,
) -> bool {
    guard_configuration_healthy(installation) && guard_observation_current(snapshot, installation)
}

fn guard_prompt_capture_configured(manifest_json: &str) -> bool {
    guard_manifest_from_json(manifest_json).is_ok_and(|manifest| {
        manifest
            .required_hook_phases
            .contains(&GuardHookPhase::PromptCapture)
    })
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
                    "state": "present",
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
                "state": "invalid",
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
    if command.is_absolute() && is_executable_file(command) {
        checks.push(
            DiagnosticCheck::passed(id, format!("{label} is executable"))
                .with_details(json!({ "path": path_text(command) })),
        );
    } else {
        checks.push(
            DiagnosticCheck::failed(id, format!("{label} is missing or not executable"))
                .with_details(json!({ "path": path_text(command) })),
        );
        let (instruction, command) = if id == "volicord_command" {
            (
                "Invoke a working Volicord executable and rerun init; init replaces an inaccessible, non-executable, or relative installation-profile volicord command with that running executable.",
                "volicord init --host codex --repo <path>",
            )
        } else {
            (
                "Select an executable MCP launch command, then rerun init with that command.",
                "volicord init --host codex --repo <path> --mcp-command PATH",
            )
        };
        actions.push(DiagnosticAction {
            id: format!("repair_{id}"),
            instruction: instruction.to_owned(),
            command: Some(command.to_owned()),
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

fn host_detection_check() -> DiagnosticCheck {
    DiagnosticCheck::skipped(
        "host_detection",
        "built-in host adapter detection is reported by init or connection verification",
    )
    .with_details(json!({
        "accepted_host_values": [HostKind::Codex.as_str()]
    }))
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
                "disclosure": {
                    "guarantee_class": "diagnostic_observation",
                    "non_guarantees": [
                        "NotOsSandbox",
                        "NotFullWritePrevention",
                        "NotActorAttributionProof",
                        "NotCorrectnessProof",
                    ],
                },
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
        (
            "Projects",
            doctor_count_state(checks, "projects", "registered"),
        ),
        (
            "Connections",
            doctor_count_state(checks, "connections", "stored"),
        ),
        ("MCP configuration", doctor_mcp_config_state(checks)),
        ("Profile", doctor_selected_profile_from_checks(checks)),
        (
            "Guard files",
            doctor_check_state(checks, "guard_files").to_owned(),
        ),
        (
            "Hook observation",
            doctor_check_state(checks, "guard_observation").to_owned(),
        ),
        ("Prompt capture", doctor_prompt_capture_status(checks)),
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
        "guard_configuration": doctor_check_state(checks, "guard_files"),
        "guard_observation": doctor_check_state(checks, "guard_observation"),
        "guard_effective": doctor_check_state(checks, "guard_observation"),
        "guard_files": doctor_check_state(checks, "guard_files"),
        "agents_managed_block": doctor_guard_file_kind_state(checks, "agents_managed_block"),
        "volicord_policy_file": doctor_guard_file_kind_state(checks, "volicord_policy"),
        "rule_instruction_config": doctor_guard_file_kind_state(checks, "host_rule_instruction"),
        "hook_config": doctor_guard_file_kind_state(checks, "host_hook_config"),
        "required_hook_phases": doctor_required_hook_phases_state(checks),
        "missing_required_hooks": doctor_missing_required_hooks_value(checks),
        "guard_hook_observed": doctor_check_state(checks, "guard_observation"),
        "guard_status": doctor_check_state(checks, "guard_observation"),
        "prompt_capture": doctor_prompt_capture_health(checks),
        "prompt_capture_status": doctor_prompt_capture_status(checks),
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
            "direct_file_write_matcher_coverage".to_owned(),
            Value::Bool(doctor_direct_file_write_matcher_coverage(checks)),
        );
    }
    states
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
    let Some(check) = checks
        .iter()
        .find(|check| check.id == "installation_profile")
    else {
        return "unknown";
    };
    if let Some(state) = check
        .details
        .as_ref()
        .and_then(|details| details.get("state"))
        .and_then(Value::as_str)
    {
        return match state {
            "present" => "present",
            "missing" => "missing",
            "invalid" => "invalid",
            "unavailable" => "unavailable",
            "corrupt" => "corrupt",
            _ => "unknown",
        };
    }
    match check.status.as_str() {
        "skipped" => "not_checked",
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
        .find(|check| check.id == "guard_files")
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.get("file_states"))
        .and_then(|states| states.get(kind))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| match check_status(checks, "guard_files") {
            Some("skipped") | None => "not_checked".to_owned(),
            _ => "not_configured".to_owned(),
        })
}

fn doctor_guard_file_bool_detail(checks: &[DiagnosticCheck], key: &str) -> bool {
    checks
        .iter()
        .find(|check| check.id == "guard_files")
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
        .find(|check| check.id == "guard_files")
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
    match check_status(checks, "guard_files") {
        Some("passed") => "configured",
        Some("warning") | Some("failed") => "missing",
        Some("skipped") => "not_checked",
        _ => "unknown",
    }
}

fn doctor_missing_required_hooks_value(checks: &[DiagnosticCheck]) -> Vec<String> {
    checks
        .iter()
        .find(|check| check.id == "guard_files")
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
    if check_status(checks, "guard_observation").is_none() {
        "not_checked"
    } else {
        doctor_check_state(checks, "guard_observation")
    }
}

fn doctor_prompt_capture_status(checks: &[DiagnosticCheck]) -> String {
    checks
        .iter()
        .find(|check| check.id == "guard_observation")
        .and_then(|check| check.details.as_ref())
        .map(|details| {
            let configured = details
                .get("prompt_capture_configured")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let observed = details
                .get("prompt_capture_observed")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if observed > 0 {
                "available"
            } else if configured > 0 {
                "configured_unobserved"
            } else {
                "not_configured"
            }
        })
        .unwrap_or("not_checked")
        .to_owned()
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
        user_action: "not_selected".to_owned(),
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
        instruction: "Initialize the Codex connection from the Product Repository.".to_owned(),
        command: Some("volicord init --host codex --repo <path>".to_owned()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_detection_reports_codex_only() {
        assert_eq!(
            host_detection_check().details,
            Some(json!({ "accepted_host_values": ["codex"] }))
        );
    }

    #[test]
    fn blocking_checks_determine_doctor_status() {
        assert_eq!(
            doctor_status(&[DiagnosticCheck::failed("installation_profile", "missing")]),
            CommandStatus::ActionRequired
        );
        assert_eq!(
            doctor_status(&[DiagnosticCheck::failed(
                "project_policy_authority",
                "invalid"
            )]),
            CommandStatus::Failed
        );
        assert_eq!(
            doctor_status(&[DiagnosticCheck::warning("path_or_shim", "not on PATH")]),
            CommandStatus::Complete
        );
    }

    #[test]
    fn installation_profile_state_does_not_collapse_missing_and_invalid() {
        let missing = DiagnosticCheck::failed("installation_profile", "missing")
            .with_details(json!({ "state": "missing" }));
        let invalid = DiagnosticCheck::failed("installation_profile", "invalid")
            .with_details(json!({ "state": "invalid" }));
        let unavailable = DiagnosticCheck::failed("installation_profile", "unavailable")
            .with_details(json!({ "state": "unavailable" }));
        let corrupt = DiagnosticCheck::failed("installation_profile", "corrupt")
            .with_details(json!({ "state": "corrupt" }));
        let unknown = DiagnosticCheck::failed("installation_profile", "unspecified");

        assert_eq!(doctor_installation_profile_state(&[missing]), "missing");
        assert_eq!(doctor_installation_profile_state(&[invalid]), "invalid");
        assert_eq!(
            doctor_installation_profile_state(&[unavailable]),
            "unavailable"
        );
        assert_eq!(doctor_installation_profile_state(&[corrupt]), "corrupt");
        assert_eq!(doctor_installation_profile_state(&[unknown]), "unknown");
    }

    #[test]
    fn policy_authority_store_failures_keep_corrupt_and_unavailable_distinct() {
        let corrupt = StoreError::corrupt_stored_json("project_state", "policy_json");
        let unavailable = StoreError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));

        assert_eq!(
            project_policy_store_failure_state(&corrupt),
            ProjectPolicyAuthorityState::AuthorityCorrupt
        );
        assert_eq!(
            project_policy_store_failure_state(&unavailable),
            ProjectPolicyAuthorityState::AuthorityUnavailable
        );
    }

    #[test]
    fn truncated_policy_authority_scan_never_reports_passed() {
        assert_eq!(project_policy_authority_result(0, false), "passed");
        assert_eq!(project_policy_authority_result(0, true), "warning");
        assert_eq!(project_policy_authority_result(1, false), "failed");
        assert_eq!(project_policy_authority_result(1, true), "failed");
    }

    #[test]
    fn default_control_surface_is_fail_closed() {
        assert_eq!(
            doctor_control_surface_value(&[]),
            json!({
                "selected_profile": "not_checked",
                "host_hooks_active": false,
                "cooperative_pre_tool_warning_available": false,
                "cooperative_pre_tool_denial_available": false,
                "unrecorded_changes_detectable": false,
                "actor_identity_provable": false,
                "os_enforced": false,
            })
        );
    }
}
