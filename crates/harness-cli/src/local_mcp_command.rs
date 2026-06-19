use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fmt, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use harness_store::runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError};
use serde_json::{json, Value};

use crate::{
    host_config::{
        binding_name, config_file_name, path_text, pretty_json, render_configs, GeneratedConfig,
    },
    setup::{
        apply_local_mcp_setup_plan, plan_local_mcp_setup, LocalMcpSetupOptions, LocalMcpSetupPlan,
        SetupAction, SetupActionKind, SetupActionTarget, SetupApplyError, SetupConflict,
        SetupPlanError, SetupResource, SetupSurfaceBinding, AGENT_SURFACE_ID,
        AGENT_SURFACE_INSTANCE_ID, USER_INTERACTION_SURFACE_ID,
        USER_INTERACTION_SURFACE_INSTANCE_ID,
    },
};

const HARNESS_HOME: &str = "HARNESS_HOME";
const PATH_ENV: &str = "PATH";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalMcpCommandError {
    Usage(String),
    Runtime(String),
}

impl LocalMcpCommandError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for LocalMcpCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for LocalMcpCommandError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedLocalMcpOptions {
    runtime_home: Option<PathBuf>,
    repo_root: Option<PathBuf>,
    project_id: Option<String>,
    include_user_interaction: bool,
    mcp_command: Option<PathBuf>,
    config_dir: Option<PathBuf>,
    output: OutputFormat,
    dry_run: bool,
    replace_conflicting_surfaces: bool,
    overwrite_config: bool,
}

impl Default for ParsedLocalMcpOptions {
    fn default() -> Self {
        Self {
            runtime_home: None,
            repo_root: None,
            project_id: None,
            include_user_interaction: false,
            mcp_command: None,
            config_dir: None,
            output: OutputFormat::Text,
            dry_run: false,
            replace_conflicting_surfaces: false,
            overwrite_config: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightEnvironment {
    pub runtime_home: PathBuf,
    pub project_id: String,
    pub surface_id: String,
    pub surface_instance_id: String,
}

impl PreflightEnvironment {
    fn for_binding(runtime_home: &Path, project_id: &str, binding: SetupSurfaceBinding) -> Self {
        Self {
            runtime_home: runtime_home.to_path_buf(),
            project_id: project_id.to_owned(),
            surface_id: binding.surface_id().to_owned(),
            surface_instance_id: binding.surface_instance_id().to_owned(),
        }
    }

    fn env_vars(&self) -> [(&'static str, OsString); 4] {
        [
            (HARNESS_HOME, self.runtime_home.as_os_str().to_os_string()),
            (
                "HARNESS_PROJECT_ID",
                OsString::from(self.project_id.clone()),
            ),
            (
                "HARNESS_SURFACE_ID",
                OsString::from(self.surface_id.clone()),
            ),
            (
                "HARNESS_SURFACE_INSTANCE_ID",
                OsString::from(self.surface_instance_id.clone()),
            ),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightProcessOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait LocalMcpProcess {
    fn env_var(&self, name: &str) -> Option<OsString>;
    fn current_exe(&self) -> Result<PathBuf, String>;
    fn run_preflight(
        &mut self,
        command: &Path,
        environment: &PreflightEnvironment,
    ) -> Result<PreflightProcessOutput, String>;
}

pub struct ProductionLocalMcpProcess;

impl LocalMcpProcess for ProductionLocalMcpProcess {
    fn env_var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }

    fn current_exe(&self) -> Result<PathBuf, String> {
        std::env::current_exe()
            .map_err(|error| format!("failed to read current executable: {error}"))
    }

    fn run_preflight(
        &mut self,
        command: &Path,
        environment: &PreflightEnvironment,
    ) -> Result<PreflightProcessOutput, String> {
        let mut child = Command::new(command);
        child.arg("--check");
        child.stdin(Stdio::null());
        for (name, value) in environment.env_vars() {
            child.env(name, value);
        }
        let output = child
            .output()
            .map_err(|error| format!("failed to run {} --check: {error}", command.display()))?;
        Ok(PreflightProcessOutput {
            success: output.status.success(),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

pub fn setup_usage() -> String {
    "harness setup local-mcp [OPTIONS]\n".to_owned()
}

pub fn local_mcp_usage() -> String {
    "harness setup local-mcp [--runtime-home PATH] [--repo-root PATH] [--project-id ID] [--with-user-interaction] [--mcp-command PATH] [--config-dir PATH] [--output text|json] [--dry-run] [--replace-conflicting-surfaces] [--overwrite-config]\n"
        .to_owned()
}

pub fn run_setup_command(
    args: &[String],
    current_dir: &Path,
    process: &mut impl LocalMcpProcess,
) -> Result<String, LocalMcpCommandError> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Ok(setup_usage());
    };

    match subcommand {
        "-h" | "--help" | "help" => {
            if args.len() == 1 {
                Ok(setup_usage())
            } else {
                Err(LocalMcpCommandError::usage(format!(
                    "unexpected argument: {}\n\n{}",
                    args[1],
                    setup_usage()
                )))
            }
        }
        "local-mcp" => {
            if matches!(
                args.get(1).map(String::as_str),
                Some("-h" | "--help" | "help")
            ) {
                if args.len() == 2 {
                    return Ok(local_mcp_usage());
                }
                return Err(LocalMcpCommandError::usage(format!(
                    "unexpected argument: {}\n\n{}",
                    args[2],
                    local_mcp_usage()
                )));
            }
            run_local_mcp_command(&args[1..], current_dir, process)
        }
        other => Err(LocalMcpCommandError::usage(format!(
            "unknown setup command: {other}\n\n{}",
            setup_usage()
        ))),
    }
}

fn run_local_mcp_command(
    args: &[String],
    current_dir: &Path,
    process: &mut impl LocalMcpProcess,
) -> Result<String, LocalMcpCommandError> {
    let parsed = parse_local_mcp_options(args)?;
    let runtime_home = resolve_setup_runtime_home(&parsed, current_dir, process)?;
    let repo_root = parsed
        .repo_root
        .clone()
        .unwrap_or_else(|| current_dir.to_path_buf());
    let repo_root = absolute_path(current_dir, repo_root);
    let config_dir = parsed
        .config_dir
        .as_ref()
        .map(|path| absolute_path(current_dir, path.clone()));

    let mcp_command = discover_selected_mcp_command(&parsed, current_dir, process)?;

    let mut setup_options = LocalMcpSetupOptions::new(&runtime_home, &repo_root);
    setup_options.project_id = parsed.project_id.clone();
    setup_options.include_user_interaction = parsed.include_user_interaction;
    setup_options.replace_conflicting_surfaces = parsed.replace_conflicting_surfaces;
    let plan = plan_local_mcp_setup(setup_options).map_err(plan_error)?;
    if !plan.conflicts.is_empty() {
        return Err(LocalMcpCommandError::runtime(format_conflicts(
            &plan.conflicts,
        )));
    }

    validate_config_destinations(
        config_dir.as_deref(),
        parsed.include_user_interaction,
        parsed.overwrite_config,
    )?;

    let project_id = plan
        .selected_project_id
        .as_deref()
        .ok_or_else(|| LocalMcpCommandError::runtime("setup plan has no selected project_id"))?;
    let configs = render_configs(
        parsed.include_user_interaction,
        &runtime_home,
        project_id,
        &mcp_command,
        config_dir.as_deref(),
    );

    if parsed.dry_run {
        return render_success_output(SuccessOutput {
            status: SetupStatus::DryRun,
            runtime_home: &runtime_home,
            repo_root: &plan.repo_root,
            project_id,
            mcp_command: &mcp_command,
            actions: plan.ordered_actions(),
            preflights: planned_preflights(parsed.include_user_interaction),
            configs: &configs,
            output: parsed.output,
        });
    }

    let result = apply_local_mcp_setup_plan(&plan).map_err(apply_error)?;
    let mut preflights = Vec::new();
    for binding in setup_bindings_from_plan(&plan) {
        match run_preflight(
            &mcp_command,
            &runtime_home,
            &result.project_id,
            binding,
            process,
        ) {
            Ok(preflight) => preflights.push(preflight),
            Err(error) => {
                let completed = format_action_lines(result.completed_actions.as_slice());
                return Err(LocalMcpCommandError::runtime(format!(
                    "{error}\ncompleted registration actions:\n{completed}"
                )));
            }
        }
    }

    write_config_files(&configs, parsed.overwrite_config)?;

    render_success_output(SuccessOutput {
        status: SetupStatus::Complete,
        runtime_home: &result.runtime_home,
        repo_root: &result.repo_root,
        project_id: &result.project_id,
        mcp_command: &mcp_command,
        actions: result.completed_actions,
        preflights,
        configs: &configs,
        output: parsed.output,
    })
}

fn parse_local_mcp_options(args: &[String]) -> Result<ParsedLocalMcpOptions, LocalMcpCommandError> {
    let mut parsed = ParsedLocalMcpOptions::default();
    let mut seen = BTreeMap::<String, ()>::new();
    let mut index = 0;

    while index < args.len() {
        let token = &args[index];
        if !token.starts_with("--") {
            return Err(LocalMcpCommandError::usage(format!(
                "unexpected argument: {token}"
            )));
        }

        let without_prefix = &token[2..];
        let (name, attached_value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name, Some(value))
        } else {
            (without_prefix, None)
        };

        if !is_allowed_option(name) {
            return Err(LocalMcpCommandError::usage(format!(
                "unknown option: --{name}"
            )));
        }
        if seen.insert(name.to_owned(), ()).is_some() {
            return Err(LocalMcpCommandError::usage(format!(
                "duplicate option: --{name}"
            )));
        }

        if is_boolean_option(name) {
            if attached_value.is_some() {
                return Err(LocalMcpCommandError::usage(format!(
                    "--{name} does not take a value"
                )));
            }
            set_boolean_option(&mut parsed, name);
            index += 1;
            continue;
        }

        let value = if let Some(value) = attached_value {
            value.to_owned()
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(LocalMcpCommandError::usage(format!(
                    "missing value for --{name}"
                )));
            };
            if value.starts_with("--") {
                return Err(LocalMcpCommandError::usage(format!(
                    "missing value for --{name}"
                )));
            }
            value.clone()
        };
        set_value_option(&mut parsed, name, value)?;
        index += 1;
    }

    if parsed.overwrite_config && parsed.config_dir.is_none() {
        return Err(LocalMcpCommandError::usage(
            "--overwrite-config requires --config-dir",
        ));
    }

    Ok(parsed)
}

fn is_allowed_option(name: &str) -> bool {
    matches!(
        name,
        "runtime-home"
            | "repo-root"
            | "project-id"
            | "with-user-interaction"
            | "mcp-command"
            | "config-dir"
            | "output"
            | "dry-run"
            | "replace-conflicting-surfaces"
            | "overwrite-config"
    )
}

fn is_boolean_option(name: &str) -> bool {
    matches!(
        name,
        "with-user-interaction" | "dry-run" | "replace-conflicting-surfaces" | "overwrite-config"
    )
}

fn set_boolean_option(parsed: &mut ParsedLocalMcpOptions, name: &str) {
    match name {
        "with-user-interaction" => parsed.include_user_interaction = true,
        "dry-run" => parsed.dry_run = true,
        "replace-conflicting-surfaces" => parsed.replace_conflicting_surfaces = true,
        "overwrite-config" => parsed.overwrite_config = true,
        _ => unreachable!("unknown boolean option validated earlier"),
    }
}

fn set_value_option(
    parsed: &mut ParsedLocalMcpOptions,
    name: &str,
    value: String,
) -> Result<(), LocalMcpCommandError> {
    match name {
        "runtime-home" => {
            reject_empty_path(name, &value)?;
            parsed.runtime_home = Some(PathBuf::from(value));
        }
        "repo-root" => {
            reject_empty_path(name, &value)?;
            parsed.repo_root = Some(PathBuf::from(value));
        }
        "project-id" => {
            if value.trim().is_empty() {
                return Err(LocalMcpCommandError::usage(
                    "--project-id must not be empty",
                ));
            }
            parsed.project_id = Some(value);
        }
        "mcp-command" => {
            reject_empty_path(name, &value)?;
            parsed.mcp_command = Some(PathBuf::from(value));
        }
        "config-dir" => {
            reject_empty_path(name, &value)?;
            parsed.config_dir = Some(PathBuf::from(value));
        }
        "output" => {
            parsed.output = match value.as_str() {
                "text" => OutputFormat::Text,
                "json" => OutputFormat::Json,
                other => {
                    return Err(LocalMcpCommandError::usage(format!(
                        "unsupported output format: {other}"
                    )));
                }
            };
        }
        _ => unreachable!("unknown value option validated earlier"),
    }
    Ok(())
}

fn reject_empty_path(name: &str, value: &str) -> Result<(), LocalMcpCommandError> {
    if value.is_empty() {
        Err(LocalMcpCommandError::usage(format!(
            "--{name} must not be empty"
        )))
    } else {
        Ok(())
    }
}

fn resolve_setup_runtime_home(
    parsed: &ParsedLocalMcpOptions,
    current_dir: &Path,
    process: &impl LocalMcpProcess,
) -> Result<PathBuf, LocalMcpCommandError> {
    let resolved = resolve_runtime_home(
        |name| {
            if name == HARNESS_HOME {
                parsed
                    .runtime_home
                    .as_ref()
                    .map(|path| path.as_os_str().to_os_string())
                    .or_else(|| process.env_var(name))
            } else {
                process.env_var(name)
            }
        },
        current_dir,
    )
    .map_err(runtime_home_error)?;
    if resolved.is_absolute() {
        Ok(resolved)
    } else {
        Ok(current_dir.join(resolved))
    }
}

fn runtime_home_error(error: RuntimeHomeResolutionError) -> LocalMcpCommandError {
    LocalMcpCommandError::runtime(error.to_string())
}

fn discover_selected_mcp_command(
    parsed: &ParsedLocalMcpOptions,
    current_dir: &Path,
    process: &impl LocalMcpProcess,
) -> Result<PathBuf, LocalMcpCommandError> {
    let current_exe = if parsed.mcp_command.is_some() {
        None
    } else {
        Some(
            process
                .current_exe()
                .map_err(LocalMcpCommandError::runtime)?,
        )
    };
    discover_mcp_command(McpDiscoveryInputs {
        explicit_command: parsed.mcp_command.as_deref(),
        current_exe: current_exe.as_deref(),
        current_dir,
        path_env: process.env_var(PATH_ENV),
    })
}

#[derive(Debug, Clone)]
struct McpDiscoveryInputs<'a> {
    explicit_command: Option<&'a Path>,
    current_exe: Option<&'a Path>,
    current_dir: &'a Path,
    path_env: Option<OsString>,
}

fn discover_mcp_command(inputs: McpDiscoveryInputs<'_>) -> Result<PathBuf, LocalMcpCommandError> {
    if let Some(command) = inputs.explicit_command {
        return validate_explicit_command(inputs.current_dir, command);
    }

    if let Some(current_exe) = inputs.current_exe {
        let current_exe = absolute_path(inputs.current_dir, current_exe.to_path_buf());
        if let Some(parent) = current_exe.parent() {
            for name in executable_file_names("harness-mcp") {
                let candidate = parent.join(name);
                if candidate.is_dir() {
                    return Err(LocalMcpCommandError::runtime(format!(
                        "harness-mcp discovery found a directory: {}",
                        candidate.display()
                    )));
                }
                if candidate.is_file() {
                    return canonical_file(&candidate, "harness-mcp sibling");
                }
            }
        }
    }

    if let Some(path_env) = inputs.path_env {
        for directory in std::env::split_paths(&path_env) {
            let directory = absolute_path(inputs.current_dir, directory);
            for name in executable_file_names("harness-mcp") {
                let candidate = directory.join(name);
                if candidate.is_file() {
                    return canonical_file(&candidate, "harness-mcp PATH entry");
                }
            }
        }
    }

    Err(LocalMcpCommandError::runtime(
        "could not discover harness-mcp executable",
    ))
}

fn validate_explicit_command(
    current_dir: &Path,
    command: &Path,
) -> Result<PathBuf, LocalMcpCommandError> {
    let command = absolute_path(current_dir, command.to_path_buf());
    if command.is_dir() {
        return Err(LocalMcpCommandError::runtime(format!(
            "mcp-command must not be a directory: {}",
            command.display()
        )));
    }
    if !command.exists() {
        return Err(LocalMcpCommandError::runtime(format!(
            "mcp-command does not exist: {}",
            command.display()
        )));
    }
    canonical_file(&command, "mcp-command")
}

fn canonical_file(path: &Path, label: &str) -> Result<PathBuf, LocalMcpCommandError> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        LocalMcpCommandError::runtime(format!("{label} is not accessible: {error}"))
    })?;
    if canonical.is_file() {
        Ok(canonical)
    } else {
        Err(LocalMcpCommandError::runtime(format!(
            "{label} must be a file: {}",
            canonical.display()
        )))
    }
}

fn executable_file_names(base: &str) -> Vec<OsString> {
    let suffix = std::env::consts::EXE_SUFFIX;
    if suffix.is_empty() {
        vec![OsString::from(base)]
    } else {
        vec![
            OsString::from(format!("{base}{suffix}")),
            OsString::from(base),
        ]
    }
}

fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BindingPreflight {
    binding: SetupSurfaceBinding,
    status: PreflightStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreflightStatus {
    Planned,
    Passed,
}

fn planned_preflights(include_user_interaction: bool) -> Vec<BindingPreflight> {
    setup_bindings_from_include(include_user_interaction)
        .into_iter()
        .map(|binding| BindingPreflight {
            binding,
            status: PreflightStatus::Planned,
        })
        .collect()
}

fn setup_bindings_from_plan(plan: &LocalMcpSetupPlan) -> Vec<SetupSurfaceBinding> {
    setup_bindings_from_include(plan.include_user_interaction)
}

fn setup_bindings_from_include(include_user_interaction: bool) -> Vec<SetupSurfaceBinding> {
    let mut bindings = vec![SetupSurfaceBinding::Agent];
    if include_user_interaction {
        bindings.push(SetupSurfaceBinding::UserInteraction);
    }
    bindings
}

fn run_preflight(
    command: &Path,
    runtime_home: &Path,
    project_id: &str,
    binding: SetupSurfaceBinding,
    process: &mut impl LocalMcpProcess,
) -> Result<BindingPreflight, String> {
    let environment = PreflightEnvironment::for_binding(runtime_home, project_id, binding);
    let output = process
        .run_preflight(command, &environment)
        .map_err(|message| format!("preflight failed for {}: {message}", binding_name(binding)))?;

    if !output.success {
        return Err(format!(
            "preflight failed for {}: process exited {}; stderr: {}",
            binding_name(binding),
            status_text(output.status_code),
            compact_stream(&output.stderr)
        ));
    }

    validate_preflight_report(binding, &environment, &output.stdout).map_err(|message| {
        let stderr = compact_stream(&output.stderr);
        if stderr.is_empty() {
            format!("preflight failed for {}: {message}", binding_name(binding))
        } else {
            format!(
                "preflight failed for {}: {message}; stderr: {stderr}",
                binding_name(binding)
            )
        }
    })?;

    Ok(BindingPreflight {
        binding,
        status: PreflightStatus::Passed,
    })
}

fn status_text(status_code: Option<i32>) -> String {
    status_code
        .map(|code| format!("with status {code}"))
        .unwrap_or_else(|| "without an exit status".to_owned())
}

fn compact_stream(text: &str) -> String {
    text.trim().replace('\n', " | ")
}

fn validate_preflight_report(
    binding: SetupSurfaceBinding,
    environment: &PreflightEnvironment,
    stdout: &str,
) -> Result<(), String> {
    let report = parse_preflight_report(stdout)?;
    expect_field(&report, "configuration", "valid")?;
    expect_field(
        &report,
        "runtime_home",
        &path_text(&environment.runtime_home),
    )?;
    expect_field(&report, "project_id", &environment.project_id)?;
    expect_field(&report, "surface_id", &environment.surface_id)?;
    expect_field(
        &report,
        "surface_instance_id",
        &environment.surface_instance_id,
    )?;
    expect_field(
        &report,
        "interaction_role",
        binding.interaction_role().as_str(),
    )?;
    let expected_baseline = match binding {
        SetupSurfaceBinding::Agent => "full",
        SetupSurfaceBinding::UserInteraction => "not_applicable",
    };
    expect_field(&report, "baseline_workflow_access", expected_baseline)?;
    Ok(())
}

fn parse_preflight_report(stdout: &str) -> Result<BTreeMap<String, String>, String> {
    let mut report = BTreeMap::new();
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let Some((key, value)) = line.split_once(':') else {
            return Err(format!("malformed preflight line: {line}"));
        };
        let key = key.trim();
        let value = value.trim_start();
        if key.is_empty() {
            return Err("malformed preflight line with empty key".to_owned());
        }
        if report.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!("duplicate preflight field: {key}"));
        }
    }
    Ok(report)
}

fn expect_field(
    report: &BTreeMap<String, String>,
    key: &str,
    expected: &str,
) -> Result<(), String> {
    match report.get(key) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!("expected {key}: {expected}, got {actual}")),
        None => Err(format!("missing preflight field: {key}")),
    }
}

fn validate_config_destinations(
    config_dir: Option<&Path>,
    include_user_interaction: bool,
    overwrite: bool,
) -> Result<(), LocalMcpCommandError> {
    let Some(config_dir) = config_dir else {
        return Ok(());
    };

    if config_dir.exists() && !config_dir.is_dir() {
        return Err(LocalMcpCommandError::runtime(format!(
            "config-dir must be a directory: {}",
            config_dir.display()
        )));
    }

    for binding in setup_bindings_from_include(include_user_interaction) {
        let path = config_dir.join(config_file_name(binding));
        validate_config_destination(&path, overwrite)?;
    }
    Ok(())
}

fn validate_config_destination(path: &Path, overwrite: bool) -> Result<(), LocalMcpCommandError> {
    if path.exists() {
        if path.is_dir() {
            return Err(LocalMcpCommandError::runtime(format!(
                "configuration destination is a directory: {}",
                path.display()
            )));
        }
        if !overwrite {
            return Err(LocalMcpCommandError::runtime(format!(
                "configuration file already exists: {}",
                path.display()
            )));
        }
    }
    if let Some(parent) = path.parent() {
        if parent.exists() && !parent.is_dir() {
            return Err(LocalMcpCommandError::runtime(format!(
                "configuration parent is not a directory: {}",
                parent.display()
            )));
        }
    }
    Ok(())
}

fn write_config_files(
    configs: &[GeneratedConfig],
    overwrite: bool,
) -> Result<(), LocalMcpCommandError> {
    let targets = configs
        .iter()
        .filter_map(|config| config.output_path.as_ref().map(|path| (config, path)))
        .collect::<Vec<_>>();
    if targets.is_empty() {
        return Ok(());
    }

    for (_, target) in &targets {
        validate_config_destination(target, overwrite)?;
    }
    for (_, target) in &targets {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                LocalMcpCommandError::runtime(format!(
                    "failed to create configuration directory {}: {error}",
                    parent.display()
                ))
            })?;
        }
    }
    for (config, target) in targets {
        validate_config_destination(target, overwrite)?;
        let text = pretty_json(&config.value).map_err(|error| {
            LocalMcpCommandError::runtime(format!("failed to render configuration JSON: {error}"))
        })?;
        write_replaced_file(target, text.as_bytes(), overwrite)?;
    }
    Ok(())
}

fn write_replaced_file(
    target: &Path,
    content: &[u8],
    overwrite: bool,
) -> Result<(), LocalMcpCommandError> {
    let (temp_path, mut file) = create_temp_file_for(target)?;
    let write_result = (|| -> io::Result<()> {
        file.write_all(content)?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    })();
    drop(file);

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(LocalMcpCommandError::runtime(format!(
            "failed to write temporary configuration file {}: {error}",
            temp_path.display()
        )));
    }

    if !overwrite && target.exists() {
        let _ = fs::remove_file(&temp_path);
        return Err(LocalMcpCommandError::runtime(format!(
            "configuration file already exists: {}",
            target.display()
        )));
    }

    match fs::rename(&temp_path, target) {
        Ok(()) => Ok(()),
        Err(_error) if overwrite && target.exists() && target.is_file() => {
            fs::remove_file(target).map_err(|remove_error| {
                let _ = fs::remove_file(&temp_path);
                LocalMcpCommandError::runtime(format!(
                    "failed to replace existing configuration file {}: {remove_error}",
                    target.display()
                ))
            })?;
            fs::rename(&temp_path, target).map_err(|rename_error| {
                let _ = fs::remove_file(&temp_path);
                LocalMcpCommandError::runtime(format!(
                    "failed to move configuration file into place after removing {}: {rename_error}",
                    target.display()
                ))
            })
        }
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            Err(LocalMcpCommandError::runtime(format!(
                "failed to move configuration file into place at {}: {error}",
                target.display()
            )))
        }
    }
}

fn create_temp_file_for(target: &Path) -> Result<(PathBuf, fs::File), LocalMcpCommandError> {
    let parent = target.parent().ok_or_else(|| {
        LocalMcpCommandError::runtime(format!(
            "configuration destination has no parent directory: {}",
            target.display()
        ))
    })?;
    let file_name = target.file_name().unwrap_or_else(|| OsStr::new("config"));
    for attempt in 0..1000u32 {
        let mut temp_name = OsString::from(".");
        temp_name.push(file_name);
        temp_name.push(format!(".tmp-{}-{attempt}", std::process::id()));
        let temp_path = parent.join(temp_name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(LocalMcpCommandError::runtime(format!(
                    "failed to create temporary configuration file {}: {error}",
                    temp_path.display()
                )));
            }
        }
    }
    Err(LocalMcpCommandError::runtime(format!(
        "failed to allocate a temporary configuration file for {}",
        target.display()
    )))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SetupStatus {
    Complete,
    DryRun,
}

struct SuccessOutput<'a> {
    status: SetupStatus,
    runtime_home: &'a Path,
    repo_root: &'a Path,
    project_id: &'a str,
    mcp_command: &'a Path,
    actions: Vec<SetupAction>,
    preflights: Vec<BindingPreflight>,
    configs: &'a [GeneratedConfig],
    output: OutputFormat,
}

fn render_success_output(output: SuccessOutput<'_>) -> Result<String, LocalMcpCommandError> {
    match output.output {
        OutputFormat::Text => render_text_output(output),
        OutputFormat::Json => render_json_output(output),
    }
}

fn render_text_output(output: SuccessOutput<'_>) -> Result<String, LocalMcpCommandError> {
    let mut text = String::new();
    text.push_str(&format!("setup: {}\n", setup_status_text(output.status)));
    text.push_str(&format!(
        "runtime_home: {}\n",
        output.runtime_home.display()
    ));
    text.push_str(&format!("project_id: {}\n", output.project_id));
    text.push_str(&format!("repo_root: {}\n", output.repo_root.display()));
    text.push_str(&format!("agent_surface_id: {AGENT_SURFACE_ID}\n"));
    text.push_str(&format!(
        "agent_surface_instance_id: {AGENT_SURFACE_INSTANCE_ID}\n"
    ));
    if output
        .configs
        .iter()
        .any(|config| config.binding == SetupSurfaceBinding::UserInteraction)
    {
        text.push_str(&format!(
            "user_interaction_surface_id: {USER_INTERACTION_SURFACE_ID}\n"
        ));
        text.push_str(&format!(
            "user_interaction_surface_instance_id: {USER_INTERACTION_SURFACE_INSTANCE_ID}\n"
        ));
    }
    text.push_str(&format!("mcp_command: {}\n", output.mcp_command.display()));
    text.push_str(&format!(
        "preflight: {}\n",
        overall_preflight_text(&output.preflights)
    ));
    for preflight in &output.preflights {
        text.push_str(&format!(
            "{}_preflight: {}\n",
            binding_name(preflight.binding),
            preflight_status_text(preflight.status)
        ));
    }
    text.push_str("actions:\n");
    text.push_str(&format_action_lines(&output.actions));

    let config_files = output
        .configs
        .iter()
        .all(|config| config.output_path.is_some());
    if config_files {
        text.push_str("generated_config_files:\n");
        for config in output.configs {
            if let Some(path) = &config.output_path {
                text.push_str(&format!(
                    "  {}: {}\n",
                    binding_name(config.binding),
                    path.display()
                ));
            }
        }
    } else {
        for config in output.configs {
            text.push_str(&format!("{}_config_json:\n", binding_name(config.binding)));
            text.push_str(&pretty_json(&config.value).map_err(|error| {
                LocalMcpCommandError::runtime(format!(
                    "failed to render configuration JSON: {error}"
                ))
            })?);
        }
    }

    Ok(text)
}

fn render_json_output(output: SuccessOutput<'_>) -> Result<String, LocalMcpCommandError> {
    let project_action = output
        .actions
        .iter()
        .find(|action| action.target == SetupActionTarget::Project)
        .map(action_json)
        .unwrap_or_else(|| json!({"action": "skipped"}));
    let surfaces = output
        .actions
        .iter()
        .filter(|action| {
            matches!(
                action.target,
                SetupActionTarget::AgentSurface | SetupActionTarget::UserInteractionSurface
            )
        })
        .map(surface_json)
        .collect::<Vec<_>>();
    let preflight = output
        .preflights
        .iter()
        .map(|preflight| {
            json!({
                "binding": binding_name(preflight.binding),
                "status": preflight_status_text(preflight.status),
            })
        })
        .collect::<Vec<_>>();
    let generated_configs = output
        .configs
        .iter()
        .map(|config| {
            json!({
                "binding": binding_name(config.binding),
                "output_path": config.output_path.as_ref().map(|path| path_text(path)),
                "config": config.value.clone(),
            })
        })
        .collect::<Vec<_>>();
    let actions = output
        .actions
        .iter()
        .map(|action| {
            json!({
                "target": action_target_name(action.target),
                "action": action_kind_text(action.kind),
            })
        })
        .collect::<Vec<_>>();

    let value = json!({
        "status": setup_status_text(output.status),
        "runtime_home": path_text(output.runtime_home),
        "project": {
            "project_id": output.project_id,
            "repo_root": path_text(output.repo_root),
            "action": project_action["action"].clone(),
        },
        "surfaces": surfaces,
        "mcp_command": path_text(output.mcp_command),
        "preflight": preflight,
        "generated_configs": generated_configs,
        "actions": actions,
        "warnings": [],
    });
    let mut text = serde_json::to_string_pretty(&value).map_err(|error| {
        LocalMcpCommandError::runtime(format!("failed to render JSON output: {error}"))
    })?;
    text.push('\n');
    Ok(text)
}

fn action_json(action: &SetupAction) -> Value {
    json!({
        "target": action_target_name(action.target),
        "action": action_kind_text(action.kind),
    })
}

fn surface_json(action: &SetupAction) -> Value {
    let (binding, surface_id, surface_instance_id) = match &action.resource {
        SetupResource::Surface {
            binding,
            surface_id,
            surface_instance_id,
            ..
        } => (*binding, surface_id.as_str(), surface_instance_id.as_str()),
        _ => unreachable!("surface action must contain surface resource"),
    };
    json!({
        "binding": binding_name(binding),
        "surface_id": surface_id,
        "surface_instance_id": surface_instance_id,
        "action": action_kind_text(action.kind),
    })
}

fn format_action_lines(actions: &[SetupAction]) -> String {
    let mut text = String::new();
    for action in actions {
        text.push_str(&format!(
            "  {}: {}\n",
            action_target_name(action.target),
            action_kind_text(action.kind)
        ));
    }
    text
}

fn setup_status_text(status: SetupStatus) -> &'static str {
    match status {
        SetupStatus::Complete => "complete",
        SetupStatus::DryRun => "dry_run",
    }
}

fn overall_preflight_text(preflights: &[BindingPreflight]) -> &'static str {
    if preflights
        .iter()
        .any(|preflight| preflight.status == PreflightStatus::Planned)
    {
        "planned"
    } else {
        "passed"
    }
}

fn preflight_status_text(status: PreflightStatus) -> &'static str {
    match status {
        PreflightStatus::Planned => "planned",
        PreflightStatus::Passed => "passed",
    }
}

fn action_kind_text(kind: SetupActionKind) -> &'static str {
    match kind {
        SetupActionKind::Create => "created",
        SetupActionKind::Reuse => "reused",
        SetupActionKind::Update => "updated",
        SetupActionKind::Conflict => "skipped",
    }
}

fn action_target_name(target: SetupActionTarget) -> &'static str {
    match target {
        SetupActionTarget::RuntimeHome => "runtime_home",
        SetupActionTarget::Project => "project",
        SetupActionTarget::AgentSurface => "agent_surface",
        SetupActionTarget::UserInteractionSurface => "user_interaction_surface",
    }
}

fn plan_error(error: SetupPlanError) -> LocalMcpCommandError {
    match error {
        SetupPlanError::InvalidOptions { detail } => LocalMcpCommandError::usage(detail),
        other => LocalMcpCommandError::runtime(other.to_string()),
    }
}

fn apply_error(error: SetupApplyError) -> LocalMcpCommandError {
    let completed = format_action_lines(error.completed_actions());
    match error {
        SetupApplyError::UnresolvedConflicts { conflicts } => {
            LocalMcpCommandError::runtime(format_conflicts(&conflicts))
        }
        SetupApplyError::InvalidPlan { message, .. } => LocalMcpCommandError::runtime(format!(
            "{message}\ncompleted registration actions:\n{completed}"
        )),
        SetupApplyError::OperationFailed { source, .. } => LocalMcpCommandError::runtime(format!(
            "{source}\ncompleted registration actions:\n{completed}"
        )),
    }
}

fn format_conflicts(conflicts: &[SetupConflict]) -> String {
    let mut text = format!("setup conflict(s): {}\n", conflicts.len());
    for conflict in conflicts {
        text.push_str(&format!(
            "  {}: {}\n",
            action_target_name(conflict.target),
            conflict.message
        ));
    }
    text
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        error::Error,
        fs,
        path::{Path, PathBuf},
    };

    use harness_store::sqlite::registry_db_path;
    use harness_test_support::TempRuntimeHome;
    use serde_json::Value;

    use super::*;

    #[test]
    fn option_defaults_are_applied() -> Result<(), Box<dyn Error>> {
        let parsed = parse_local_mcp_options(&[])?;

        assert_eq!(parsed.runtime_home, None);
        assert_eq!(parsed.repo_root, None);
        assert_eq!(parsed.project_id, None);
        assert!(!parsed.include_user_interaction);
        assert_eq!(parsed.mcp_command, None);
        assert_eq!(parsed.config_dir, None);
        assert_eq!(parsed.output, OutputFormat::Text);
        assert!(!parsed.dry_run);
        assert!(!parsed.replace_conflicting_surfaces);
        assert!(!parsed.overwrite_config);
        Ok(())
    }

    #[test]
    fn boolean_options_are_presence_only() -> Result<(), Box<dyn Error>> {
        let parsed = parse_local_mcp_options(&args([
            "--with-user-interaction",
            "--dry-run",
            "--replace-conflicting-surfaces",
            "--config-dir",
            "out",
            "--overwrite-config",
        ]))?;

        assert!(parsed.include_user_interaction);
        assert!(parsed.dry_run);
        assert!(parsed.replace_conflicting_surfaces);
        assert!(parsed.overwrite_config);
        Ok(())
    }

    #[test]
    fn invalid_option_combinations_are_usage_errors() {
        assert_usage(["--unknown"], "unknown option");
        assert_usage(["--dry-run", "--dry-run"], "duplicate option");
        assert_usage(["--dry-run=true"], "does not take a value");
        assert_usage(["--repo-root"], "missing value");
        assert_usage(["--output", "yaml"], "unsupported output format");
        assert_usage(["--overwrite-config"], "requires --config-dir");
        assert_usage(["--interactive"], "unknown option");
        assert_usage(["extra"], "unexpected argument");
    }

    #[test]
    fn runtime_home_flag_takes_precedence_over_environment() -> Result<(), Box<dyn Error>> {
        let current = TempRuntimeHome::new("setup-runtime-precedence")?;
        let explicit = current.path().join("flag-home");
        let env_home = current.path().join("env-home");
        let parsed = parse_local_mcp_options(&args([
            "--runtime-home",
            explicit.to_str().expect("utf8 path"),
        ]))?;
        let mut process = FakeProcess::new(current.path());
        process
            .env
            .insert(HARNESS_HOME.to_owned(), env_home.as_os_str().to_os_string());

        let resolved = resolve_setup_runtime_home(&parsed, current.path(), &process)?;

        assert_eq!(resolved, explicit);
        Ok(())
    }

    #[test]
    fn repository_defaults_to_current_directory() -> Result<(), Box<dyn Error>> {
        let fixture = CommandFixture::new("setup-repo-default")?;
        let mut process = FakeProcess::new(fixture.repo_root());
        process.env.insert(
            HARNESS_HOME.to_owned(),
            fixture.runtime_home().as_os_str().to_os_string(),
        );
        let output = run_setup_command(
            &args([
                "local-mcp",
                "--mcp-command",
                fixture.mcp_command_text(),
                "--dry-run",
                "--output",
                "json",
            ]),
            fixture.repo_root(),
            &mut process,
        )?;
        let value: Value = serde_json::from_str(&output)?;

        assert_eq!(
            value["project"]["repo_root"],
            fs::canonicalize(fixture.repo_root())?.display().to_string()
        );
        Ok(())
    }

    #[test]
    fn explicit_mcp_command_is_canonicalized() -> Result<(), Box<dyn Error>> {
        let temp = TempRuntimeHome::new("setup-explicit-mcp")?;
        let command = write_dummy_command(temp.path(), "custom-mcp")?;

        let discovered = discover_mcp_command(McpDiscoveryInputs {
            explicit_command: Some(Path::new("custom-mcp")),
            current_exe: None,
            current_dir: temp.path(),
            path_env: None,
        })?;

        assert_eq!(discovered, fs::canonicalize(command)?);
        Ok(())
    }

    #[test]
    fn sibling_mcp_command_is_discovered_before_path() -> Result<(), Box<dyn Error>> {
        let temp = TempRuntimeHome::new("setup-sibling-mcp")?;
        let bin = temp.path().join("bin");
        let path_bin = temp.path().join("path-bin");
        fs::create_dir_all(&bin)?;
        fs::create_dir_all(&path_bin)?;
        let sibling = write_dummy_command(&bin, "harness-mcp")?;
        let path_match = write_dummy_command(&path_bin, "harness-mcp")?;
        let harness = write_dummy_command(&bin, "harness")?;

        let discovered = discover_mcp_command(McpDiscoveryInputs {
            explicit_command: None,
            current_exe: Some(&harness),
            current_dir: temp.path(),
            path_env: Some(std::env::join_paths([path_bin])?),
        })?;

        assert_eq!(discovered, fs::canonicalize(sibling)?);
        assert_ne!(discovered, fs::canonicalize(path_match)?);
        Ok(())
    }

    #[test]
    fn path_mcp_command_is_discovered() -> Result<(), Box<dyn Error>> {
        let temp = TempRuntimeHome::new("setup-path-mcp")?;
        let bin = temp.path().join("bin");
        let path_bin = temp.path().join("path-bin");
        fs::create_dir_all(&bin)?;
        fs::create_dir_all(&path_bin)?;
        let harness = write_dummy_command(&bin, "harness")?;
        let path_match = write_dummy_command(&path_bin, "harness-mcp")?;

        let discovered = discover_mcp_command(McpDiscoveryInputs {
            explicit_command: None,
            current_exe: Some(&harness),
            current_dir: temp.path(),
            path_env: Some(std::env::join_paths([path_bin])?),
        })?;

        assert_eq!(discovered, fs::canonicalize(path_match)?);
        Ok(())
    }

    #[test]
    fn discovery_failure_happens_before_registration_writes() -> Result<(), Box<dyn Error>> {
        let fixture = CommandFixture::new("setup-discovery-before-write")?;
        let missing = fixture.temp.path().join("missing-mcp");
        let mut process = FakeProcess::new(fixture.repo_root());

        let error = run_setup_command(
            &args([
                "local-mcp",
                "--runtime-home",
                fixture.runtime_home_text(),
                "--repo-root",
                fixture.repo_root_text(),
                "--mcp-command",
                missing.to_str().expect("utf8 path"),
            ]),
            fixture.repo_root(),
            &mut process,
        )
        .expect_err("missing mcp command should fail");

        assert!(matches!(error, LocalMcpCommandError::Runtime(_)));
        assert!(!registry_db_path(fixture.runtime_home()).exists());
        Ok(())
    }

    #[test]
    fn dry_run_with_absent_runtime_home_does_not_write_or_preflight() -> Result<(), Box<dyn Error>>
    {
        let fixture = CommandFixture::new("setup-dry-run-absent")?;
        let runtime_home = fixture.temp.path().join("absent-runtime");
        let mut process = FakeProcess::new(fixture.repo_root());
        let output = run_setup_command(
            &args([
                "local-mcp",
                "--runtime-home",
                runtime_home.to_str().expect("utf8 path"),
                "--repo-root",
                fixture.repo_root_text(),
                "--mcp-command",
                fixture.mcp_command_text(),
                "--dry-run",
            ]),
            fixture.repo_root(),
            &mut process,
        )?;

        assert!(output.contains("setup: dry_run\n"));
        assert!(output.contains("preflight: planned\n"));
        assert!(!registry_db_path(&runtime_home).exists());
        assert!(process.calls.is_empty());
        Ok(())
    }

    #[test]
    fn json_dry_run_stdout_is_parseable() -> Result<(), Box<dyn Error>> {
        let fixture = CommandFixture::new("setup-json-dry-run")?;
        let mut process = FakeProcess::new(fixture.repo_root());

        let output = run_setup_command(
            &args([
                "local-mcp",
                "--runtime-home",
                fixture.runtime_home_text(),
                "--repo-root",
                fixture.repo_root_text(),
                "--mcp-command",
                fixture.mcp_command_text(),
                "--dry-run",
                "--output",
                "json",
            ]),
            fixture.repo_root(),
            &mut process,
        )?;
        let value: Value = serde_json::from_str(&output)?;

        assert_eq!(value["status"], "dry_run");
        assert_eq!(value["preflight"][0]["status"], "planned");
        assert_eq!(value["generated_configs"][0]["binding"], "agent");
        assert!(output.ends_with('\n'));
        Ok(())
    }

    #[test]
    fn real_run_passes_expected_preflight_environment() -> Result<(), Box<dyn Error>> {
        let fixture = CommandFixture::new("setup-preflight-env")?;
        let mut process = FakeProcess::new(fixture.repo_root());

        run_setup_command(
            &args([
                "local-mcp",
                "--runtime-home",
                fixture.runtime_home_text(),
                "--repo-root",
                fixture.repo_root_text(),
                "--mcp-command",
                fixture.mcp_command_text(),
            ]),
            fixture.repo_root(),
            &mut process,
        )?;

        assert_eq!(process.calls.len(), 1);
        let call = &process.calls[0];
        assert_eq!(call.command, fs::canonicalize(fixture.mcp_command())?);
        assert_eq!(call.environment.runtime_home, fixture.runtime_home());
        assert_eq!(call.environment.project_id, "repo");
        assert_eq!(call.environment.surface_id, AGENT_SURFACE_ID);
        assert_eq!(
            call.environment.surface_instance_id,
            AGENT_SURFACE_INSTANCE_ID
        );
        Ok(())
    }

    #[test]
    fn successful_agent_and_user_preflight_reports_parse() {
        let runtime_home = Path::new("/runtime");
        let agent =
            PreflightEnvironment::for_binding(runtime_home, "project", SetupSurfaceBinding::Agent);
        let user = PreflightEnvironment::for_binding(
            runtime_home,
            "project",
            SetupSurfaceBinding::UserInteraction,
        );

        validate_preflight_report(SetupSurfaceBinding::Agent, &agent, &success_report(&agent))
            .expect("agent report should pass");
        validate_preflight_report(
            SetupSurfaceBinding::UserInteraction,
            &user,
            &success_report(&user),
        )
        .expect("user report should pass");
    }

    #[test]
    fn malformed_or_contradictory_preflight_reports_fail() {
        let env = PreflightEnvironment::for_binding(
            Path::new("/runtime"),
            "project",
            SetupSurfaceBinding::Agent,
        );

        assert!(validate_preflight_report(SetupSurfaceBinding::Agent, &env, "not report").is_err());
        assert!(validate_preflight_report(
            SetupSurfaceBinding::Agent,
            &env,
            &success_report(&PreflightEnvironment {
                surface_id: USER_INTERACTION_SURFACE_ID.to_owned(),
                surface_instance_id: USER_INTERACTION_SURFACE_INSTANCE_ID.to_owned(),
                ..env.clone()
            }),
        )
        .is_err());
        let wrong_profile = success_report(&env).replace(
            "baseline_workflow_access: full",
            "baseline_workflow_access: partial",
        );
        assert!(
            validate_preflight_report(SetupSurfaceBinding::Agent, &env, &wrong_profile).is_err()
        );
    }

    #[test]
    fn failed_subprocess_is_reported_after_registration() -> Result<(), Box<dyn Error>> {
        let fixture = CommandFixture::new("setup-preflight-failure")?;
        let mut process = FakeProcess::new(fixture.repo_root());
        process.outputs.push(PreflightProcessOutput {
            success: false,
            status_code: Some(7),
            stdout: String::new(),
            stderr: "boom".to_owned(),
        });

        let error = run_setup_command(
            &args([
                "local-mcp",
                "--runtime-home",
                fixture.runtime_home_text(),
                "--repo-root",
                fixture.repo_root_text(),
                "--mcp-command",
                fixture.mcp_command_text(),
            ]),
            fixture.repo_root(),
            &mut process,
        )
        .expect_err("failed subprocess should fail setup");

        assert!(error.to_string().contains("preflight failed for agent"));
        assert!(error.to_string().contains("completed registration actions"));
        assert!(registry_db_path(fixture.runtime_home()).exists());
        Ok(())
    }

    #[test]
    fn configuration_collision_is_detected_before_registration() -> Result<(), Box<dyn Error>> {
        let fixture = CommandFixture::new("setup-config-collision")?;
        let config_dir = fixture.temp.path().join("configs");
        fs::create_dir_all(&config_dir)?;
        fs::write(config_dir.join("harness-agent.mcp.json"), "{}")?;
        let mut process = FakeProcess::new(fixture.repo_root());

        let error = run_setup_command(
            &args([
                "local-mcp",
                "--runtime-home",
                fixture.runtime_home_text(),
                "--repo-root",
                fixture.repo_root_text(),
                "--mcp-command",
                fixture.mcp_command_text(),
                "--config-dir",
                config_dir.to_str().expect("utf8 path"),
            ]),
            fixture.repo_root(),
            &mut process,
        )
        .expect_err("existing config should fail");

        assert!(error.to_string().contains("already exists"));
        assert!(!registry_db_path(fixture.runtime_home()).exists());
        Ok(())
    }

    #[test]
    fn overwrite_replaces_existing_configuration_and_cleans_temp() -> Result<(), Box<dyn Error>> {
        let fixture = CommandFixture::new("setup-config-overwrite")?;
        let config_dir = fixture.temp.path().join("configs");
        fs::create_dir_all(&config_dir)?;
        let target = config_dir.join("harness-agent.mcp.json");
        fs::write(&target, "old")?;
        let mut process = FakeProcess::new(fixture.repo_root());

        run_setup_command(
            &args([
                "local-mcp",
                "--runtime-home",
                fixture.runtime_home_text(),
                "--repo-root",
                fixture.repo_root_text(),
                "--mcp-command",
                fixture.mcp_command_text(),
                "--config-dir",
                config_dir.to_str().expect("utf8 path"),
                "--overwrite-config",
            ]),
            fixture.repo_root(),
            &mut process,
        )?;

        let parsed: Value = serde_json::from_str(&fs::read_to_string(&target)?)?;
        assert_eq!(
            parsed["mcpServers"]["harness-agent"]["env"]["HARNESS_SURFACE_ID"],
            AGENT_SURFACE_ID
        );
        assert!(temporary_files(&config_dir)?.is_empty());
        Ok(())
    }

    #[test]
    fn preflight_failure_leaves_no_configuration_file() -> Result<(), Box<dyn Error>> {
        let fixture = CommandFixture::new("setup-no-config-on-preflight-fail")?;
        let config_dir = fixture.temp.path().join("configs");
        let mut process = FakeProcess::new(fixture.repo_root());
        process.outputs.push(PreflightProcessOutput {
            success: true,
            status_code: Some(0),
            stdout: "configuration: valid\ninteraction_role: agent\n".to_owned(),
            stderr: String::new(),
        });

        let error = run_setup_command(
            &args([
                "local-mcp",
                "--runtime-home",
                fixture.runtime_home_text(),
                "--repo-root",
                fixture.repo_root_text(),
                "--mcp-command",
                fixture.mcp_command_text(),
                "--config-dir",
                config_dir.to_str().expect("utf8 path"),
            ]),
            fixture.repo_root(),
            &mut process,
        )
        .expect_err("malformed preflight should fail");

        assert!(error.to_string().contains("missing preflight field"));
        assert!(!config_dir.join("harness-agent.mcp.json").exists());
        assert!(temporary_files(&config_dir).unwrap_or_default().is_empty());
        Ok(())
    }

    #[test]
    fn repeated_setup_reports_reused_resources() -> Result<(), Box<dyn Error>> {
        let fixture = CommandFixture::new("setup-repeat-reuse")?;
        let mut process = FakeProcess::new(fixture.repo_root());
        let command = args([
            "local-mcp",
            "--runtime-home",
            fixture.runtime_home_text(),
            "--repo-root",
            fixture.repo_root_text(),
            "--mcp-command",
            fixture.mcp_command_text(),
        ]);

        run_setup_command(&command, fixture.repo_root(), &mut process)?;
        let second = run_setup_command(&command, fixture.repo_root(), &mut process)?;

        assert!(second.contains("runtime_home: reused\n"));
        assert!(second.contains("project: reused\n"));
        assert!(second.contains("agent_surface: reused\n"));
        Ok(())
    }

    fn assert_usage<const N: usize>(arguments: [&str; N], expected: &str) {
        let error = parse_local_mcp_options(&args(arguments))
            .expect_err("arguments should be rejected as usage");
        assert!(matches!(error, LocalMcpCommandError::Usage(_)));
        assert!(
            error.to_string().contains(expected),
            "expected {expected:?} in {error}"
        );
    }

    fn args<const N: usize>(arguments: [&str; N]) -> Vec<String> {
        arguments
            .iter()
            .map(|argument| argument.to_string())
            .collect()
    }

    fn write_dummy_command(dir: &Path, name: &str) -> Result<PathBuf, Box<dyn Error>> {
        fs::create_dir_all(dir)?;
        let path = dir.join(name);
        fs::write(&path, "dummy")?;
        Ok(path)
    }

    fn temporary_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut files = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                files.push(entry.path());
            }
        }
        Ok(files)
    }

    fn success_report(environment: &PreflightEnvironment) -> String {
        let (role, baseline) = if environment.surface_id == USER_INTERACTION_SURFACE_ID {
            ("user_interaction", "not_applicable")
        } else {
            ("agent", "full")
        };
        format!(
            "configuration: valid\ntransport: stdio\nruntime_home: {}\nproject_id: {}\nsurface_id: {}\nsurface_instance_id: {}\ninteraction_role: {}\naccess_classes: read_status,core_mutation,write_authorization,artifact_registration,run_recording\nbaseline_workflow_access: {}\nmissing_access_classes: \n",
            environment.runtime_home.display(),
            environment.project_id,
            environment.surface_id,
            environment.surface_instance_id,
            role,
            baseline
        )
    }

    #[derive(Debug)]
    struct CommandFixture {
        temp: TempRuntimeHome,
        runtime_home: PathBuf,
        repo_root: PathBuf,
        mcp_command: PathBuf,
        runtime_home_text: String,
        repo_root_text: String,
        mcp_command_text: String,
    }

    impl CommandFixture {
        fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
            let temp = TempRuntimeHome::new(prefix)?;
            let runtime_home = temp.path().join("runtime-home");
            let repo_root = temp.path().join("repo");
            let bin = temp.path().join("bin");
            fs::create_dir_all(&repo_root)?;
            fs::create_dir_all(&bin)?;
            let mcp_command = write_dummy_command(&bin, "harness-mcp")?;
            let runtime_home_text = runtime_home.display().to_string();
            let repo_root_text = repo_root.display().to_string();
            let mcp_command_text = mcp_command.display().to_string();
            Ok(Self {
                temp,
                runtime_home,
                repo_root,
                mcp_command,
                runtime_home_text,
                repo_root_text,
                mcp_command_text,
            })
        }

        fn runtime_home(&self) -> &Path {
            &self.runtime_home
        }

        fn repo_root(&self) -> &Path {
            &self.repo_root
        }

        fn mcp_command(&self) -> &Path {
            &self.mcp_command
        }

        fn runtime_home_text(&self) -> &str {
            &self.runtime_home_text
        }

        fn repo_root_text(&self) -> &str {
            &self.repo_root_text
        }

        fn mcp_command_text(&self) -> &str {
            &self.mcp_command_text
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct RecordedCall {
        command: PathBuf,
        environment: PreflightEnvironment,
    }

    #[derive(Debug)]
    struct FakeProcess {
        env: BTreeMap<String, OsString>,
        current_exe: PathBuf,
        outputs: Vec<PreflightProcessOutput>,
        calls: Vec<RecordedCall>,
    }

    impl FakeProcess {
        fn new(current_dir: &Path) -> Self {
            Self {
                env: BTreeMap::new(),
                current_exe: current_dir.join("harness"),
                outputs: Vec::new(),
                calls: Vec::new(),
            }
        }
    }

    impl LocalMcpProcess for FakeProcess {
        fn env_var(&self, name: &str) -> Option<OsString> {
            self.env.get(name).cloned()
        }

        fn current_exe(&self) -> Result<PathBuf, String> {
            Ok(self.current_exe.clone())
        }

        fn run_preflight(
            &mut self,
            command: &Path,
            environment: &PreflightEnvironment,
        ) -> Result<PreflightProcessOutput, String> {
            self.calls.push(RecordedCall {
                command: command.to_path_buf(),
                environment: environment.clone(),
            });
            if self.outputs.is_empty() {
                Ok(PreflightProcessOutput {
                    success: true,
                    status_code: Some(0),
                    stdout: success_report(environment),
                    stderr: String::new(),
                })
            } else {
                Ok(self.outputs.remove(0))
            }
        }
    }
}
