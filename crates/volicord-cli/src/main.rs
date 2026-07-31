#![forbid(unsafe_code)]

use std::{
    env,
    ffi::{OsStr, OsString},
    fmt, fs,
    path::Path,
    process,
};

use volicord_cli::{
    changes_command::{run_changes_command, ChangesCommandError},
    connection_command::{
        connection_setup_required_message, run_connection_command, run_init_command,
        ConnectionCommandError, ProductionConnectionProcess,
    },
    diagnostics_command::{run_diagnostics_command, DiagnosticsCommandError},
    doctor_command::{run_doctor_command, DoctorCommandError},
    evidence_command::{run_evidence_command, EvidenceCommandError},
    export_command::{run_export_command, ExportCommandError},
    guard_command::{run_guard_command, GuardCommandError},
    host_integration::{MANAGED_WRAPPER_ENV, MANAGED_WRAPPER_VALUE},
    host_launch::{run_host_launch, HostLaunchBinding},
    policy_command::{run_policy_command, PolicyCommandError},
    project_context::{run_project_command, ProjectCommandError},
    setup_command::CommandOutcome,
    user_command::{run_inbox_command, run_status_command, UserCommandError},
    version_command::{concise_version, run_version_command, VersionCommandError},
};
use volicord_command_model::{
    Cli, CodexHost, Command as CliCommand, McpArgs, McpBindingArgs, McpCommand,
    PolicyCommand as CliPolicyCommand,
};
use volicord_store::bootstrap::installation_profile;
use volicord_store::runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError};

fn main() {
    let args = env::args_os();
    let current_dir = match env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("error: failed to read current directory: {error}");
            process::exit(1);
        }
    };

    match run_cli(args, |name| env::var_os(name), &current_dir) {
        Ok(output) => print!("{output}"),
        Err(CliError::McpStdio {
            connection_id,
            project_id,
        }) => {
            if let Err(error) =
                volicord_mcp::run_stdio_from_env(&connection_id, project_id.as_deref(), None)
            {
                eprintln!("{}", volicord_mcp::bootstrap_diagnostic_envelope(&error));
                process::exit(1);
            }
        }
        Err(CliError::McpRepositoryStdio { host }) => {
            if let Err(error) = volicord_mcp::run_stdio_discover_repository_from_env(host, None) {
                eprintln!("{}", volicord_mcp::bootstrap_diagnostic_envelope(&error));
                process::exit(1);
            }
        }
        Err(CliError::HostLaunch { host, binding }) => {
            if let Err(error) =
                run_host_launch(host, binding, |name| env::var_os(name), &current_dir)
            {
                eprintln!("error: managed host launch failed: {error}");
                process::exit(1);
            }
        }
        Err(CliError::Usage(message)) => {
            eprintln!("{message}");
            process::exit(2);
        }
        Err(CliError::Runtime(message)) => {
            eprintln!("error: {message}");
            process::exit(1);
        }
        Err(CliError::FailureOutput(output)) => {
            print!("{output}");
            process::exit(1);
        }
        Err(CliError::ProcessOutput {
            stdout,
            stderr,
            exit_code,
        }) => {
            print!("{stdout}");
            eprint!("{stderr}");
            process::exit(exit_code);
        }
    }
}

fn run_cli<I, S, F>(args: I, env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString> + Clone,
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let parsed = match Cli::try_parse_from(args) {
        Ok(parsed) => parsed,
        Err(error) if !error.use_stderr() => return Ok(error.to_string()),
        Err(error) => return Err(CliError::usage(error.to_string())),
    };
    if parsed.version {
        return Ok(concise_version());
    }
    let command = parsed
        .command
        .expect("the clap declaration requires a command or --version");

    match command {
        CliCommand::Version(options) => run_version_command(options).map_err(CliError::from),
        CliCommand::Doctor(options) => {
            command_outcome(run_doctor_command(options, &env_var, current_dir)?)
        }
        CliCommand::Diagnostics(options) => {
            require_setup_completed(&env_var, current_dir)?;
            run_diagnostics_command(options, env_var, current_dir).map_err(CliError::from)
        }
        CliCommand::Mcp(options) => command_mcp(options, env_var, current_dir),
        CliCommand::Init(options) => {
            let mut connection_process = ProductionConnectionProcess;
            run_init_command(options, current_dir, &mut connection_process).map_err(CliError::from)
        }
        CliCommand::Hook(options) => {
            require_explicit_runtime_home_binding(&env_var)?;
            require_setup_completed(&env_var, current_dir)?;
            guard_command_outcome(run_guard_command(options, env_var, current_dir)?)
        }
        CliCommand::HostLaunch(options) => Err(CliError::HostLaunch {
            host: match options.host {
                CodexHost::Codex => volicord_types::values::HostKind::Codex,
            },
            binding: options.connection.map_or(
                HostLaunchBinding::DiscoverRepository,
                HostLaunchBinding::Connection,
            ),
        }),
        CliCommand::Connection(options) => {
            let mut connection_process = ProductionConnectionProcess;
            run_connection_command(options, current_dir, &mut connection_process)
                .map_err(CliError::from)
        }
        CliCommand::Evidence(options) => {
            require_setup_completed(&env_var, current_dir)?;
            run_evidence_command(options, env_var, current_dir).map_err(CliError::from)
        }
        CliCommand::Changes(options) => {
            require_setup_completed(&env_var, current_dir)?;
            run_changes_command(options, env_var, current_dir).map_err(CliError::from)
        }
        CliCommand::Export(options) => {
            run_export_command(options, env_var, current_dir).map_err(CliError::from)
        }
        CliCommand::Status(options) => {
            require_setup_completed(&env_var, current_dir)?;
            run_status_command(options, env_var, current_dir).map_err(CliError::from)
        }
        CliCommand::Inbox(options) => {
            require_setup_completed(&env_var, current_dir)?;
            run_inbox_command(options, env_var, current_dir).map_err(CliError::from)
        }
        CliCommand::Project(options) => {
            require_setup_completed(&env_var, current_dir)?;
            run_project_command(options, env_var, current_dir).map_err(CliError::from)
        }
        CliCommand::Policy(options) => {
            if matches!(
                &options.command,
                CliPolicyCommand::Show(_) | CliPolicyCommand::Apply(_)
            ) {
                require_setup_completed(&env_var, current_dir)?;
            }
            run_policy_command(options, env_var, current_dir).map_err(CliError::from)
        }
    }
}

fn command_outcome(outcome: CommandOutcome) -> Result<String, CliError> {
    if outcome.status.exits_failure() {
        Err(CliError::FailureOutput(outcome.output))
    } else {
        Ok(outcome.output)
    }
}

fn guard_command_outcome(
    outcome: volicord_cli::guard_command::GuardCommandOutcome,
) -> Result<String, CliError> {
    if outcome.exit_code == 0 && outcome.stderr.is_empty() {
        Ok(outcome.stdout)
    } else {
        Err(CliError::ProcessOutput {
            stdout: outcome.stdout,
            stderr: outcome.stderr,
            exit_code: outcome.exit_code,
        })
    }
}

fn require_setup_completed<F>(env_var: &F, current_dir: &Path) -> Result<(), CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let runtime_home = resolve_runtime_home(|name| env_var(name), current_dir)?;
    match installation_profile(&runtime_home) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(CliError::runtime(connection_setup_required_message(
            &runtime_home,
        ))),
        Err(error) => Err(CliError::runtime(format!(
            "{}; {}",
            error,
            connection_setup_required_message(&runtime_home)
        ))),
    }
}

fn require_explicit_runtime_home_binding<F>(env_var: &F) -> Result<(), CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let runtime_home = env_var("VOLICORD_HOME")
        .filter(|value| !value.is_empty() && Path::new(value).is_absolute())
        .map(std::path::PathBuf::from)
        .ok_or_else(managed_process_binding_required)?;
    if env_var(MANAGED_WRAPPER_ENV).as_deref() != Some(OsStr::new(MANAGED_WRAPPER_VALUE)) {
        return Err(managed_process_binding_required());
    }
    let profile = installation_profile(&runtime_home)
        .map_err(|_| managed_process_binding_required())?
        .ok_or_else(managed_process_binding_required)?;
    let profile_command = Path::new(&profile.volicord_command);
    if !profile_command.is_absolute() {
        return Err(managed_process_binding_required());
    }
    let selected_command =
        fs::canonicalize(profile_command).map_err(|_| managed_process_binding_required())?;
    let current_command = env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|_| managed_process_binding_required())?;
    if current_command != selected_command {
        return Err(managed_process_binding_required());
    }
    Ok(())
}

fn managed_process_binding_required() -> CliError {
    CliError::runtime(
        "RUNTIME_HOME_BINDING_REQUIRED: managed host hook commands require a current generated wrapper to supply an absolute, non-empty VOLICORD_HOME, the managed process binding marker, and the installation profile's selected executable; rerun `volicord init` for this Product Repository and reload the host",
    )
}

fn command_mcp<F>(args: McpArgs, env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    match args.command {
        McpCommand::Serve(binding) => mcp_serve(binding),
        McpCommand::Preflight(options) => {
            let (runtime_home, connection_id, project_id) = if options.binding.discover_repository {
                let runtime_home =
                    volicord_mcp::resolve_repository_discovery_runtime_home(&env_var, current_dir)
                        .map_err(|error| CliError::runtime(error.to_string()))?;
                let resolution = volicord_mcp::RepositoryDiscoveryResolution::resolve(
                    runtime_home,
                    current_dir,
                    volicord_types::values::HostKind::Codex,
                )
                .map_err(|error| CliError::runtime(error.to_string()))?;
                (
                    resolution.runtime_home,
                    resolution.connection_internal_id.as_str().to_owned(),
                    Some(resolution.project_id.as_str().to_owned()),
                )
            } else {
                (
                    volicord_mcp::resolve_runtime_home(&env_var, current_dir)
                        .map_err(|error| CliError::runtime(error.to_string()))?,
                    options
                        .binding
                        .connection
                        .expect("clap requires an MCP binding"),
                    options.binding.project,
                )
            };
            volicord_cli::connection_command::validate_mcp_preflight_managed_entry(
                &runtime_home,
                &connection_id,
                project_id.as_deref(),
            )
            .map_err(|error| CliError::runtime(error.to_string()))?;
            let report = volicord_mcp::preflight_check(
                env_var,
                current_dir,
                &connection_id,
                project_id.as_deref(),
            )
            .map_err(|error| CliError::runtime(error.to_string()))?;
            if options.output.json {
                serde_json::to_string_pretty(&report)
                    .map(|output| format!("{output}\n"))
                    .map_err(|error| CliError::runtime(error.to_string()))
            } else {
                Ok(report.render_human(options.output.verbose))
            }
        }
    }
}

fn mcp_serve(binding: McpBindingArgs) -> Result<String, CliError> {
    if binding.discover_repository {
        return Err(CliError::McpRepositoryStdio {
            host: volicord_types::values::HostKind::Codex,
        });
    }
    Err(CliError::McpStdio {
        connection_id: binding.connection.expect("clap requires an MCP binding"),
        project_id: binding.project,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliError {
    Usage(String),
    Runtime(String),
    FailureOutput(String),
    ProcessOutput {
        stdout: String,
        stderr: String,
        exit_code: i32,
    },
    McpStdio {
        connection_id: String,
        project_id: Option<String>,
    },
    McpRepositoryStdio {
        host: volicord_types::values::HostKind,
    },
    HostLaunch {
        host: volicord_types::values::HostKind,
        binding: HostLaunchBinding,
    },
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) | Self::FailureOutput(message) => {
                formatter.write_str(message)
            }
            Self::ProcessOutput { stdout, .. } => formatter.write_str(stdout),
            Self::McpStdio { connection_id, .. } => {
                write!(
                    formatter,
                    "MCP stdio requested for connection {connection_id}"
                )
            }
            Self::McpRepositoryStdio { host } => write!(
                formatter,
                "MCP stdio requested through {} repository discovery",
                host.as_str()
            ),
            Self::HostLaunch { host, .. } => {
                write!(formatter, "managed {} host launch requested", host.as_str())
            }
        }
    }
}

impl std::error::Error for CliError {}

impl From<volicord_store::StoreError> for CliError {
    fn from(error: volicord_store::StoreError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for CliError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<ConnectionCommandError> for CliError {
    fn from(error: ConnectionCommandError) -> Self {
        match error {
            ConnectionCommandError::Usage(message) => Self::Usage(message),
            ConnectionCommandError::Runtime(message)
            | ConnectionCommandError::ConcurrentModification(message) => Self::Runtime(message),
            ConnectionCommandError::FailureOutput(output) => Self::FailureOutput(output),
            ConnectionCommandError::MutationAdmission(error) => Self::Runtime(error.to_string()),
        }
    }
}

impl From<UserCommandError> for CliError {
    fn from(error: UserCommandError) -> Self {
        match error {
            UserCommandError::Usage(message) => Self::Usage(message),
            UserCommandError::Runtime(message) => Self::Runtime(message),
            UserCommandError::MutationAdmission(error) => Self::Runtime(error.to_string()),
        }
    }
}

impl From<ChangesCommandError> for CliError {
    fn from(error: ChangesCommandError) -> Self {
        match error {
            ChangesCommandError::Usage(message) => Self::Usage(message),
            ChangesCommandError::Runtime(message) => Self::Runtime(message),
            ChangesCommandError::FailureOutput(output) => Self::FailureOutput(output),
            ChangesCommandError::MutationAdmission(error) => Self::Runtime(error.to_string()),
        }
    }
}

impl From<ExportCommandError> for CliError {
    fn from(error: ExportCommandError) -> Self {
        match error {
            ExportCommandError::Usage(message) => Self::Usage(message),
            ExportCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

impl From<ProjectCommandError> for CliError {
    fn from(error: ProjectCommandError) -> Self {
        match error {
            ProjectCommandError::Usage(message) => Self::Usage(message),
            ProjectCommandError::Runtime(message) => Self::Runtime(message),
            ProjectCommandError::MutationAdmission(error) => Self::Runtime(error.to_string()),
        }
    }
}

impl From<DoctorCommandError> for CliError {
    fn from(error: DoctorCommandError) -> Self {
        match error {
            DoctorCommandError::Usage(message) => Self::Usage(message),
            DoctorCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

impl From<DiagnosticsCommandError> for CliError {
    fn from(error: DiagnosticsCommandError) -> Self {
        match error {
            DiagnosticsCommandError::Usage(message) => Self::Usage(message),
            DiagnosticsCommandError::Runtime(message) => Self::Runtime(message),
            DiagnosticsCommandError::NotFoundOutput(output) => Self::FailureOutput(output),
        }
    }
}

impl From<PolicyCommandError> for CliError {
    fn from(error: PolicyCommandError) -> Self {
        match error {
            PolicyCommandError::Usage(message) => Self::Usage(message),
            PolicyCommandError::Validation {
                code,
                field_path,
                message,
            } => Self::Runtime(format!("{code} at {field_path}: {message}")),
            PolicyCommandError::FailureOutput(output) => Self::FailureOutput(output),
            PolicyCommandError::Runtime(message) => Self::Runtime(message),
            PolicyCommandError::MutationAdmission(error) => Self::Runtime(error.to_string()),
        }
    }
}

impl From<GuardCommandError> for CliError {
    fn from(error: GuardCommandError) -> Self {
        match error {
            GuardCommandError::Usage(message) => Self::Usage(message),
            GuardCommandError::Runtime(message) | GuardCommandError::Persistence(message) => {
                Self::Runtime(message)
            }
        }
    }
}

impl From<EvidenceCommandError> for CliError {
    fn from(error: EvidenceCommandError) -> Self {
        match error {
            EvidenceCommandError::Usage(message) => Self::Usage(message),
            EvidenceCommandError::Runtime(message) => Self::Runtime(message),
            EvidenceCommandError::MutationAdmission(error) => Self::Runtime(error.to_string()),
        }
    }
}

impl From<VersionCommandError> for CliError {
    fn from(error: VersionCommandError) -> Self {
        Self::Runtime(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn version_commands_and_help_do_not_require_runtime_home() {
        let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
        let version = run_cli(["volicord", "--version"], |_| None, cwd)
            .expect("version should not need Runtime Home");
        assert_eq!(version, concise_version());

        let command_version = run_cli(["volicord", "version"], |_| None, cwd)
            .expect("version command should not need Runtime Home");
        assert_eq!(command_version, concise_version());

        let help = run_cli(["volicord", "--help"], |_| None, cwd)
            .expect("help should not need Runtime Home");
        for command in ["init", "connection", "evidence", "mcp", "inbox"] {
            assert!(help.contains(command), "missing command {command}");
        }
    }

    #[test]
    fn unknown_top_level_command_is_usage_error() {
        let error = run_cli(
            ["volicord", "not-a-real-command"],
            |_| None,
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect_err("unknown command should fail");

        assert!(matches!(error, CliError::Usage(_)));
        assert!(error.to_string().contains("unrecognized subcommand"));
    }

    #[test]
    fn platform_store_error_renders_canonical_code_and_bounded_detail() {
        let error = volicord_store::StoreError::UnsupportedPlatformEnvironment {
            diagnostic: volicord_platform_fs::PlatformDiagnostic::new(
                volicord_platform_fs::PlatformDiagnosticKind::UnsupportedTarget,
                "the running binary target is unsupported; use a supported Volicord platform target",
            ),
        };

        let rendered = CliError::from(error).to_string();
        let (code, detail) = rendered
            .split_once(": ")
            .expect("CLI platform error must include code and bounded detail");
        assert_eq!(code, "platform.target.unsupported");
        assert!(detail.contains("running binary target is unsupported"));
        assert!(detail.contains("supported Volicord platform target"));
    }

    #[test]
    fn core_operational_unavailability_maps_to_the_cli_runtime_diagnostic() {
        let core_error =
            volicord_core::CorePipelineError::from(volicord_store::StoreError::NotFound {
                entity: "project_state_database",
                id: "bounded-fixture-identity".to_owned(),
            });
        let user_error = UserCommandError::from(core_error);
        let error = CliError::from(user_error);

        let CliError::Runtime(message) = error else {
            panic!("Core operational unavailability must map to CLI runtime failure");
        };
        assert_eq!(
            message,
            "Core operation unavailable: operation=store_access, resource=project_store, retryable=true"
        );
        assert!(!message.contains("MCP"));
        assert!(!message.contains("bounded-fixture-identity"));
    }

    #[test]
    fn hidden_host_launcher_requires_exactly_one_binding() {
        let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
        let personal = run_cli(
            [
                "volicord",
                "_host-launch",
                "codex",
                "--connection",
                "connection_alpha",
            ],
            |_| None,
            cwd,
        )
        .expect_err("launcher dispatch should leave process-owned stdio to main");
        assert_eq!(
            personal,
            CliError::HostLaunch {
                host: volicord_types::values::HostKind::Codex,
                binding: HostLaunchBinding::Connection("connection_alpha".to_owned()),
            }
        );

        let shared = run_cli(
            ["volicord", "_host-launch", "codex", "--discover-repository"],
            |_| None,
            cwd,
        )
        .expect_err("launcher dispatch should leave process-owned stdio to main");
        assert_eq!(
            shared,
            CliError::HostLaunch {
                host: volicord_types::values::HostKind::Codex,
                binding: HostLaunchBinding::DiscoverRepository,
            }
        );

        for invalid in [
            vec!["volicord", "_host-launch", "codex"],
            vec![
                "volicord",
                "_host-launch",
                "codex",
                "--connection",
                "connection_alpha",
                "--discover-repository",
            ],
        ] {
            assert!(matches!(
                run_cli(invalid, |_| None, cwd),
                Err(CliError::Usage(_))
            ));
        }
        let help = run_cli(["volicord", "--help"], |_| None, cwd).expect("help");
        assert!(!help.contains("_host-launch"));
    }
}
