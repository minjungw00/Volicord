#![forbid(unsafe_code)]

use std::{
    env, fmt,
    io::{self, IsTerminal},
    path::Path,
    process,
};

use volicord_cli::{
    connection_command::{
        connect_usage, connection_usage, connections_usage, init_usage, run_connect_command,
        run_connection_command, run_connections_command, run_init_command, ConnectionCommandError,
        ProductionConnectionProcess,
    },
    doctor_command::{doctor_usage, run_doctor_command, DoctorCommandError},
    export_command::{export_usage, run_export_command, ExportCommandError},
    guard_command::{guard_usage, run_guard_command, GuardCommandError},
    project_context::{project_usage, run_project_command, ProjectCommandError},
    serve_command::{run_serve_command, serve_usage, ServeCommand, ServeCommandError},
    setup_command::{
        run_setup_command, run_setup_command_interactive, setup_usage, ClosureSetupProcess,
        CommandOutcome, SetupCommandError, StdioSetupTerminal,
    },
    user_command::{run_user_command, user_usage, UserCommandError},
};
use volicord_store::bootstrap::installation_profile;
use volicord_store::runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError};

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let current_dir = match env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("error: failed to read current directory: {error}");
            process::exit(1);
        }
    };

    match run_cli(args, |name| env::var_os(name), &current_dir) {
        Ok(output) => print!("{output}"),
        Err(CliError::McpStdio { connection_id }) => {
            if let Err(error) = volicord_mcp::run_stdio_from_env(&connection_id) {
                eprintln!("error: {error}");
                process::exit(1);
            }
        }
        Err(CliError::ServeStreamableHttp { config }) => {
            if let Err(error) = volicord_mcp::run_streamable_http_server(*config) {
                eprintln!("error: {error}");
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
    }
}

fn run_cli<I, S, F>(args: I, env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let command = args.get(1).map(String::as_str).unwrap_or("--help");

    match command {
        "-h" | "--help" | "help" => Ok(usage()),
        "-V" | "--version" => {
            if args.len() == 2 {
                Ok(version())
            } else {
                Err(CliError::usage(format!(
                    "unexpected argument for {command}\n\n{}",
                    usage()
                )))
            }
        }
        "setup" => {
            let process = ClosureSetupProcess::new(&env_var);
            if setup_prompt_allowed(&args[2..])
                && io::stdin().is_terminal()
                && io::stdout().is_terminal()
            {
                let mut terminal = StdioSetupTerminal::new();
                command_outcome(run_setup_command_interactive(
                    &args[2..],
                    current_dir,
                    &process,
                    &mut terminal,
                )?)
            } else {
                command_outcome(run_setup_command(&args[2..], current_dir, &process)?)
            }
        }
        "doctor" => command_outcome(run_doctor_command(&args[2..], &env_var, current_dir)?),
        "export" => run_export_command(&args[2..], &env_var, current_dir).map_err(CliError::from),
        "mcp" => command_mcp(&args[2..], env_var, current_dir),
        "serve" => command_serve(&args[2..], env_var, current_dir),
        "init" => {
            let mut connection_process = ProductionConnectionProcess;
            run_init_command(&args[2..], current_dir, &mut connection_process)
                .map_err(CliError::from)
        }
        "guard" => {
            if !guard_help_requested(&args[2..]) {
                require_setup_completed(&env_var, current_dir)?;
            }
            guard_command_outcome(run_guard_command(&args[2..], env_var, current_dir)?)
        }
        "connect" => {
            if !simple_help_requested(&args[2..]) {
                require_setup_completed(&env_var, current_dir)?;
            }
            let mut connection_process = ProductionConnectionProcess;
            run_connect_command(&args[2..], current_dir, &mut connection_process)
                .map_err(CliError::from)
        }
        "connections" => {
            if !simple_help_requested(&args[2..]) {
                require_setup_completed(&env_var, current_dir)?;
            }
            let mut connection_process = ProductionConnectionProcess;
            run_connections_command(&args[2..], current_dir, &mut connection_process)
                .map_err(CliError::from)
        }
        "connection" => {
            if !connection_help_requested(&args[2..]) {
                require_setup_completed(&env_var, current_dir)?;
            }
            let mut connection_process = ProductionConnectionProcess;
            run_connection_command(&args[2..], current_dir, &mut connection_process)
                .map_err(CliError::from)
        }
        "user" => {
            if user_subcommand_requires_setup(&args[2..]) {
                require_setup_completed(&env_var, current_dir)?;
            }
            run_user_command(&args[2..], env_var, current_dir).map_err(CliError::from)
        }
        "project" => {
            if project_subcommand_requires_setup(&args[2..]) {
                require_setup_completed(&env_var, current_dir)?;
            }
            command_project(&args[2..], env_var, current_dir)
        }
        other => Err(CliError::usage(format!(
            "unknown command: {other}\n\n{}",
            usage()
        ))),
    }
}

fn user_subcommand_requires_setup(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        Some("status" | "judgments" | "judgment")
    )
}

fn project_subcommand_requires_setup(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        Some("use" | "current" | "list" | "rename" | "forget")
    )
}

fn simple_help_requested(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        None | Some("-h" | "--help" | "help")
    )
}

fn setup_prompt_allowed(args: &[String]) -> bool {
    !matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    ) && !args.iter().any(|arg| {
        matches!(arg.as_str(), "--json" | "--link-bin")
            || arg.starts_with("--json=")
            || arg.starts_with("--link-bin=")
    })
}

fn connection_help_requested(args: &[String]) -> bool {
    match args {
        [] => true,
        [first] if matches!(first.as_str(), "-h" | "--help" | "help") => true,
        [subcommand, help, ..]
            if matches!(subcommand.as_str(), "status" | "verify" | "mode" | "remove")
                && matches!(help.as_str(), "-h" | "--help" | "help") =>
        {
            true
        }
        _ => false,
    }
}

fn guard_help_requested(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        None | Some("-h" | "--help" | "help")
    ) || matches!(
        args.get(1).map(String::as_str),
        Some("-h" | "--help" | "help")
    )
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
    if outcome.exits_failure {
        Err(CliError::FailureOutput(outcome.output))
    } else {
        Ok(outcome.output)
    }
}

fn require_setup_completed<F>(env_var: &F, current_dir: &Path) -> Result<(), CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let runtime_home = resolve_runtime_home(|name| env_var(name), current_dir)?;
    match installation_profile(&runtime_home) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(CliError::runtime(setup_required_message(&runtime_home))),
        Err(error) => Err(CliError::runtime(format!(
            "{}; {}",
            error,
            setup_required_message(&runtime_home)
        ))),
    }
}

fn setup_required_message(runtime_home: &Path) -> String {
    if !runtime_home.exists() {
        format!(
            "RUNTIME_HOME_MISSING: Runtime Home {} is missing; run `volicord setup` before project, connection, export, MCP, serve, or user workflows",
            runtime_home.display()
        )
    } else {
        format!(
            "SETUP_REQUIRED: installation profile is missing for Runtime Home {}; run `volicord setup` before project, connection, export, MCP, serve, or user workflows",
            runtime_home.display()
        )
    }
}

fn command_serve<F>(args: &[String], env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    match run_serve_command(args, env_var, current_dir)? {
        ServeCommand::Help => Ok(serve_usage()),
        ServeCommand::Version => Ok(version()),
        ServeCommand::StreamableHttp { config } => Err(CliError::ServeStreamableHttp {
            config: Box::new(config),
        }),
    }
}

fn command_project<F>(args: &[String], env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    run_project_command(args, env_var, current_dir).map_err(CliError::from)
}

fn command_mcp<F>(args: &[String], env_var: F, current_dir: &Path) -> Result<String, CliError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    match dispatch_mcp_args(args)? {
        McpCommand::Help => Ok(mcp_usage()),
        McpCommand::Version => Ok(version()),
        McpCommand::Check {
            connection_id,
            project_id,
        } => volicord_mcp::preflight_check(
            env_var,
            current_dir,
            &connection_id,
            project_id.as_deref(),
        )
        .map_err(|error| CliError::runtime(error.to_string())),
        McpCommand::Stdio { connection_id } => Err(CliError::McpStdio { connection_id }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum McpCommand {
    Stdio {
        connection_id: String,
    },
    Help,
    Version,
    Check {
        connection_id: String,
        project_id: Option<String>,
    },
}

fn dispatch_mcp_args(args: &[String]) -> Result<McpCommand, CliError> {
    match args {
        [option] if option == "-h" || option == "--help" || option == "help" => {
            return Ok(McpCommand::Help)
        }
        [option] if option == "-V" || option == "--version" => return Ok(McpCommand::Version),
        _ => {}
    }

    let mut stdio = false;
    let mut check = false;
    let mut connection_id = None;
    let mut project_id = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--stdio" => {
                if stdio {
                    return Err(CliError::usage("--stdio was supplied more than once"));
                }
                stdio = true;
                index += 1;
            }
            "--check" => {
                if check {
                    return Err(CliError::usage("--check was supplied more than once"));
                }
                check = true;
                index += 1;
            }
            "--connection" => {
                if connection_id.is_some() {
                    return Err(CliError::usage("--connection was supplied more than once"));
                }
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| CliError::usage("--connection requires a value"))?;
                if value.starts_with('-') {
                    return Err(CliError::usage("--connection requires a value"));
                }
                connection_id = Some(value.clone());
                index += 1;
            }
            "--project" => {
                if project_id.is_some() {
                    return Err(CliError::usage("--project was supplied more than once"));
                }
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| CliError::usage("--project requires a value"))?;
                if value.starts_with('-') {
                    return Err(CliError::usage("--project requires a value"));
                }
                project_id = Some(value.clone());
                index += 1;
            }
            "-h" | "--help" | "help" | "-V" | "--version" => {
                return Err(CliError::usage(
                    "cannot combine volicord mcp command-line modes",
                ))
            }
            option if option.starts_with('-') => {
                return Err(CliError::usage(format!("unknown option: {option}")));
            }
            argument => return Err(CliError::usage(format!("unexpected argument: {argument}"))),
        }
    }

    if stdio && check {
        return Err(CliError::usage("cannot combine --stdio and --check"));
    }
    if project_id.is_some() && !check {
        return Err(CliError::usage("--project is only valid with --check"));
    }
    if !stdio && !check {
        return Err(CliError::usage(
            "MCP mode is required; use --stdio or --check",
        ));
    }

    let connection_id = connection_id.ok_or_else(|| {
        CliError::usage("--connection is required for connection-bound MCP startup")
    })?;

    if check {
        Ok(McpCommand::Check {
            connection_id,
            project_id,
        })
    } else {
        Ok(McpCommand::Stdio { connection_id })
    }
}

#[cfg(test)]
fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn usage() -> String {
    format!(
        "Usage:\n  volicord --help\n  volicord --version\n{}{}{}{}{}{}{}{}{}{}{}{}\nEnvironment:\n  VOLICORD_HOME  Override Runtime Home path (default: $HOME/.volicord)\n\nAgent Connection commands manage local MCP host connections. User Channel commands record local user judgments.\nThese are local administrative commands, not public Volicord API methods.\n",
        indent_usage_block(&setup_usage()),
        indent_usage_block(&init_usage()),
        indent_usage_block(&doctor_usage()),
        indent_usage_block(&export_usage()),
        indent_usage_block(&mcp_usage()),
        indent_usage_block(&serve_usage()),
        indent_usage_block(&guard_usage()),
        indent_usage_block(&connect_usage()),
        indent_usage_block(&connections_usage()),
        indent_usage_block(&connection_usage()),
        indent_usage_block(&user_usage()),
        indent_usage_block(&project_usage())
    )
}

fn indent_usage_block(block: &str) -> String {
    block.lines().map(|line| format!("  {line}\n")).collect()
}

fn mcp_usage() -> String {
    "volicord mcp --stdio --connection <connection_id>\nvolicord mcp --check --connection <connection_id>\nvolicord mcp --check --connection <connection_id> --project <project_id>\n".to_owned()
}

fn version() -> String {
    format!("volicord {}\n", env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliError {
    Usage(String),
    Runtime(String),
    FailureOutput(String),
    McpStdio {
        connection_id: String,
    },
    ServeStreamableHttp {
        config: Box<volicord_mcp::StreamableHttpServerConfig>,
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
            Self::McpStdio { connection_id } => {
                write!(
                    formatter,
                    "MCP stdio requested for connection {connection_id}"
                )
            }
            Self::ServeStreamableHttp { config } => {
                write!(
                    formatter,
                    "MCP HTTP serve requested for connection {}",
                    config.connection_id
                )
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
            ConnectionCommandError::Runtime(message) => Self::Runtime(message),
            ConnectionCommandError::FailureOutput(output) => Self::FailureOutput(output),
        }
    }
}

impl From<UserCommandError> for CliError {
    fn from(error: UserCommandError) -> Self {
        match error {
            UserCommandError::Usage(message) => Self::Usage(message),
            UserCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

impl From<ProjectCommandError> for CliError {
    fn from(error: ProjectCommandError) -> Self {
        match error {
            ProjectCommandError::Usage(message) => Self::Usage(message),
            ProjectCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

impl From<SetupCommandError> for CliError {
    fn from(error: SetupCommandError) -> Self {
        match error {
            SetupCommandError::Usage(message) => Self::Usage(message),
            SetupCommandError::Runtime(message) => Self::Runtime(message),
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

impl From<ExportCommandError> for CliError {
    fn from(error: ExportCommandError) -> Self {
        match error {
            ExportCommandError::Usage(message) => Self::Usage(message),
            ExportCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

impl From<GuardCommandError> for CliError {
    fn from(error: GuardCommandError) -> Self {
        match error {
            GuardCommandError::Usage(message) => Self::Usage(message),
            GuardCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

impl From<ServeCommandError> for CliError {
    fn from(error: ServeCommandError) -> Self {
        match error {
            ServeCommandError::Usage(message) => Self::Usage(message),
            ServeCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        fs,
        io::Write,
        path::{Path, PathBuf},
    };

    use serde_json::Value;
    use volicord_store::bootstrap::{
        installation_profile as read_installation_profile, list_projects,
    };
    use volicord_store::sqlite::registry_db_path;
    use volicord_test_support::TempRuntimeHome;

    use super::*;

    #[test]
    fn version_does_not_require_runtime_home_environment() {
        let output = run_cli(
            ["volicord", "--version"],
            |_| None,
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("version should not need Runtime Home");

        assert_eq!(output, format!("volicord {}\n", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn short_version_is_exact_alias() {
        let output = run_cli(
            ["volicord", "-V"],
            |_| None,
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("short version should not need Runtime Home");

        assert_eq!(output, version());
    }

    #[test]
    fn help_mentions_version_discovery() {
        let output = run_cli(
            ["volicord", "--help"],
            |_| None,
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("help should not need Runtime Home");

        assert!(output.contains("volicord --version"));
        assert!(output.contains("volicord setup"));
        assert!(output.contains("volicord init"));
        assert!(output.contains("volicord doctor"));
        assert!(output.contains("volicord mcp --stdio --connection <connection_id>"));
        assert!(output.contains("volicord serve --transport streamable-http"));
        assert!(output.contains("\n  volicord connection verify"));
        assert!(output.contains("\n  volicord user judgments"));
        assert!(!output.contains("\nvolicord connection verify"));
        assert!(!output.contains("\nvolicord user judgments"));
    }

    #[test]
    fn setup_prompt_gate_allows_plain_setup_but_not_noninteractive_options() {
        assert!(setup_prompt_allowed(&[]));
        assert!(setup_prompt_allowed(&[
            "--home".to_owned(),
            "runtime".to_owned()
        ]));
        assert!(!setup_prompt_allowed(&["--help".to_owned()]));
        assert!(!setup_prompt_allowed(&["--json".to_owned()]));
        assert!(!setup_prompt_allowed(&["--json=true".to_owned()]));
        assert!(!setup_prompt_allowed(&[
            "--link-bin".to_owned(),
            "bin".to_owned()
        ]));
        assert!(!setup_prompt_allowed(&["--link-bin=bin".to_owned()]));
    }

    #[test]
    fn unknown_top_level_command_is_usage_error() {
        let error = run_cli(
            ["volicord", "not-a-real-command"],
            |_| None,
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect_err("unknown command should be a usage error");

        assert_eq!(
            error,
            CliError::Usage(format!(
                "unknown command: not-a-real-command\n\n{}",
                usage()
            ))
        );
    }

    #[test]
    fn setup_respects_volicord_home_override() {
        let runtime_home = TempRuntimeHome::new("cli-setup").expect("temp runtime home");
        let mcp = write_fake_executable(runtime_home.path(), "volicord")
            .expect("fake mcp should be created");
        let output = run_cli(
            vec![
                "volicord".to_owned(),
                "setup".to_owned(),
                "--mcp-command".to_owned(),
                display_path(&mcp),
            ],
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(OsString::from(runtime_home.path()))
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("setup should succeed");

        assert!(output.contains("Volicord setup action_required\n"));
        assert!(output.contains("default_connection_mode: workflow\n"));
        assert!(output.contains("command_state: action_required\n"));
        assert!(output.contains("next_action: "));
        assert!(registry_db_path(runtime_home.path()).exists());
        assert!(read_installation_profile(runtime_home.path())
            .expect("profile read should work")
            .is_some());
    }

    #[test]
    fn default_runtime_home_uses_user_home() {
        let runtime_home = resolve_runtime_home(
            |name| {
                if name == "HOME" {
                    Some(OsString::from("/tmp/volicord-cli-home"))
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("default runtime home should resolve");

        assert_eq!(
            runtime_home,
            PathBuf::from("/tmp/volicord-cli-home/.volicord")
        );
    }

    #[test]
    fn setup_resolves_runtime_home_with_shared_resolver() {
        let current_dir = TempRuntimeHome::new("cli-cwd").expect("temp current dir");
        let runtime_home = resolve_runtime_home(
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(OsString::from("shared-runtime"))
                } else {
                    None
                }
            },
            current_dir.path(),
        )
        .expect("shared resolver should resolve relative Runtime Home");
        let mcp = write_fake_executable(current_dir.path(), "volicord")
            .expect("fake mcp should be created");

        let output = run_cli(
            vec![
                "volicord".to_owned(),
                "setup".to_owned(),
                "--mcp-command".to_owned(),
                display_path(&mcp),
            ],
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(OsString::from("shared-runtime"))
                } else {
                    None
                }
            },
            current_dir.path(),
        )
        .expect("setup should use shared Runtime Home resolution");

        assert!(output.contains(&format!("runtime_home: {}\n", display_path(&runtime_home))));
        assert!(registry_db_path(runtime_home).exists());
    }

    #[test]
    fn runtime_home_resolution_errors_are_runtime_errors() {
        let error = run_cli(
            ["volicord", "setup"],
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(OsString::new())
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect_err("empty VOLICORD_HOME should fail");

        assert!(matches!(error, CliError::Runtime(_)));
        assert!(error
            .to_string()
            .contains("VOLICORD_HOME must not be empty"));
    }

    #[test]
    fn project_current_reports_unregistered_repository_without_creating_project() {
        let runtime_home = TempRuntimeHome::new("cli-project-current").expect("temp runtime home");
        setup_runtime_home(&runtime_home).expect("setup should succeed");
        let repo_root = create_git_repo(&runtime_home, "current-repo");
        let nested = repo_root.join("src/nested");
        fs::create_dir_all(&nested).expect("nested repo fixture should be created");

        let output = run_with_home_at(
            runtime_home.path(),
            ["volicord", "project", "current"],
            &nested,
        )
        .expect("project current should report without registration");

        assert!(output.contains("project not registered\n"));
        assert!(output.contains(&format!("repo_root: {}\n", display_path(&repo_root))));
        assert!(list_projects(runtime_home.path())
            .expect("project list should read")
            .is_empty());
    }

    #[test]
    fn project_use_detects_nested_git_repository_and_hides_text_internal_id() {
        let runtime_home = TempRuntimeHome::new("cli-project-use").expect("temp runtime home");
        setup_runtime_home(&runtime_home).expect("setup should succeed");
        let repo_root = create_git_repo(&runtime_home, "product-repo");
        let nested = repo_root.join("src/nested");
        fs::create_dir_all(&nested).expect("nested repo fixture should be created");

        let output = run_with_home_at(
            runtime_home.path(),
            ["volicord", "project", "use", "--json"],
            &nested,
        )
        .expect("project use should succeed from nested directory");
        let value = json_value(&output);
        let project = &value["project"];
        let internal_id = project["project_internal_id"]
            .as_str()
            .expect("project_internal_id should be present");

        assert_eq!(value["status"], "registered");
        assert_eq!(project["project_name"], "product-repo");
        assert_eq!(project["repo_root"], display_path(&repo_root));
        assert!(internal_id.starts_with("prj_"));

        let projects = list_projects(runtime_home.path()).expect("registered project should list");
        assert_eq!(projects.len(), 1);
        assert!(projects[0].state_db_path.exists());

        let text = run_with_home_at(
            runtime_home.path(),
            ["volicord", "project", "current"],
            &nested,
        )
        .expect("current project should be registered");
        assert!(text.contains("project current\n"));
        assert!(text.contains("name: product-repo\n"));
        assert!(!text.contains(internal_id));
        assert!(!text.contains("project_internal_id"));
    }

    #[test]
    fn project_list_disambiguates_duplicate_basenames_by_path() {
        let runtime_home =
            TempRuntimeHome::new("cli-project-duplicates").expect("temp runtime home");
        setup_runtime_home(&runtime_home).expect("setup should succeed");
        let repo_a = create_git_repo(&runtime_home, "left/repo");
        let repo_b = create_git_repo(&runtime_home, "right/repo");

        let first = run_with_home_at(
            runtime_home.path(),
            [
                "volicord",
                "project",
                "use",
                repo_a.to_str().expect("utf8 repo path"),
                "--json",
            ],
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("first same-basename repo should register");
        let second = run_with_home_at(
            runtime_home.path(),
            [
                "volicord",
                "project",
                "use",
                repo_b.to_str().expect("utf8 repo path"),
                "--json",
            ],
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("second same-basename repo should register");
        let first_id = json_value(&first)["project"]["project_internal_id"]
            .as_str()
            .expect("first id")
            .to_owned();
        let second_id = json_value(&second)["project"]["project_internal_id"]
            .as_str()
            .expect("second id")
            .to_owned();

        let text = run_with_home(runtime_home.path(), ["volicord", "project", "list"])
            .expect("project list should succeed");
        let lines = text.lines().collect::<Vec<_>>();
        assert_eq!(lines[0], "name\trepo_root\tstatus");
        assert!(text.contains(&format!("repo\t{}\tactive", display_path(&repo_a))));
        assert!(text.contains(&format!("repo\t{}\tactive", display_path(&repo_b))));
        assert!(!text.contains(&first_id));
        assert!(!text.contains(&second_id));

        let json = run_with_home(
            runtime_home.path(),
            ["volicord", "project", "list", "--json"],
        )
        .expect("JSON project list should succeed");
        let value = json_value(&json);
        assert_eq!(
            value["projects"]
                .as_array()
                .expect("projects should be an array")
                .len(),
            2
        );
        assert!(json.contains("project_internal_id"));
    }

    #[test]
    fn project_rename_and_forget_select_without_user_supplied_ids() {
        let runtime_home = TempRuntimeHome::new("cli-project-rename").expect("temp runtime home");
        setup_runtime_home(&runtime_home).expect("setup should succeed");
        let repo_root = create_git_repo(&runtime_home, "rename-repo");

        run_with_home_at(
            runtime_home.path(),
            [
                "volicord",
                "project",
                "use",
                repo_root.to_str().expect("utf8 repo path"),
            ],
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("project use should register");
        let state_db_path = list_projects(runtime_home.path())
            .expect("project should list")
            .remove(0)
            .state_db_path;

        let renamed = run_with_home_at(
            runtime_home.path(),
            [
                "volicord",
                "project",
                "rename",
                "Renamed Project",
                "--repo",
                repo_root.to_str().expect("utf8 repo path"),
            ],
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("project rename should succeed");
        assert!(renamed.contains("project renamed\n"));
        assert!(renamed.contains("name: Renamed Project\n"));

        let forgotten = run_with_home(
            runtime_home.path(),
            ["volicord", "project", "forget", "Renamed Project"],
        )
        .expect("project forget by name should succeed");
        assert!(forgotten.contains("project forgotten\n"));
        assert!(forgotten.contains("project_state_deleted: false\n"));
        assert!(state_db_path.exists());
        assert!(list_projects(runtime_home.path())
            .expect("registry should remain readable")
            .is_empty());
    }

    #[test]
    fn project_use_rejects_repository_under_runtime_home_without_project_state() {
        let runtime_home = TempRuntimeHome::new("cli-project-boundary").expect("temp runtime home");
        setup_runtime_home(&runtime_home).expect("setup should succeed");
        let repo_root = runtime_home.path().join("product-repo");
        fs::create_dir_all(repo_root.join(".git")).expect("repo fixture should be created");

        let error = run_with_home_at(
            runtime_home.path(),
            [
                "volicord",
                "project",
                "use",
                repo_root.to_str().expect("utf8 path"),
            ],
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect_err("project use should reject Product Repository inside Runtime Home");

        assert!(matches!(error, CliError::Runtime(_)));
        assert!(error
            .to_string()
            .contains("Product Repository must not be inside Volicord Runtime Home"));
        assert!(list_projects(runtime_home.path())
            .expect("registry inspection should still work")
            .is_empty());
    }

    #[test]
    fn project_use_requires_setup_profile() {
        let runtime_home = TempRuntimeHome::new("cli-uninitialized").expect("temp runtime home");
        let repo_root = create_git_repo(&runtime_home, "missing-setup-repo");
        let error = run_with_home_at(
            runtime_home.path(),
            ["volicord", "project", "use"],
            &repo_root,
        )
        .expect_err("project use should require setup");

        assert!(matches!(error, CliError::Runtime(_)));
        assert!(error.to_string().contains("run `volicord setup`"));
    }

    fn run_with_home<const N: usize>(
        runtime_home: &Path,
        args: [&str; N],
    ) -> Result<String, CliError> {
        run_with_home_at(runtime_home, args, Path::new(env!("CARGO_MANIFEST_DIR")))
    }

    fn run_with_home_at<const N: usize>(
        runtime_home: &Path,
        args: [&str; N],
        current_dir: &Path,
    ) -> Result<String, CliError> {
        run_cli(
            args,
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(OsString::from(runtime_home))
                } else {
                    None
                }
            },
            current_dir,
        )
    }

    fn setup_runtime_home(runtime_home: &TempRuntimeHome) -> Result<String, CliError> {
        let mcp = write_fake_executable(runtime_home.path(), "volicord")
            .expect("fake mcp should be created");
        run_cli(
            vec![
                "volicord".to_owned(),
                "setup".to_owned(),
                "--mcp-command".to_owned(),
                display_path(&mcp),
            ],
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(OsString::from(runtime_home.path()))
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
    }

    fn json_value(text: &str) -> Value {
        serde_json::from_str(text).expect("output should be JSON")
    }

    fn create_git_repo(runtime_home: &TempRuntimeHome, name: &str) -> PathBuf {
        let repo_root = runtime_home
            .create_product_repo(name)
            .expect("repo fixture should be created");
        fs::create_dir_all(repo_root.join(".git")).expect("git marker should be created");
        repo_root
    }

    fn write_fake_executable(dir: &Path, name: &str) -> std::io::Result<PathBuf> {
        fs::create_dir_all(dir)?;
        let path = dir.join(name);
        let mut file = fs::File::create(&path)?;
        writeln!(file, "#!/bin/sh")?;
        make_executable(&path)?;
        Ok(path)
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) -> std::io::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &Path) -> std::io::Result<()> {
        Ok(())
    }
}
