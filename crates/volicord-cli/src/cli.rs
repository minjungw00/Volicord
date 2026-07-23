//! The complete command-line declaration for the `volicord` binary.

use std::path::PathBuf;

use clap::{ArgAction, ArgGroup, Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "volicord",
    about = "Local Volicord administration and managed stdio MCP",
    disable_version_flag = true,
    arg_required_else_help = true,
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    /// Print the exact build identity.
    #[arg(
        short = 'V',
        long = "version",
        action = ArgAction::SetTrue,
        exclusive = true
    )]
    pub version: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize a Codex Record connection.
    Init(InitArgs),
    /// Show current project workflow status.
    Status(StatusArgs),
    /// Inspect the local installation and managed integrations.
    Doctor(DoctorArgs),
    /// Read bounded local diagnostic data.
    Diagnostics(DiagnosticsArgs),
    /// Manage the authoritative project workflow policy.
    Policy(PolicyArgs),
    /// Manage Codex Agent Connections.
    Connection(ConnectionArgs),
    /// Manage registered Product Repositories.
    Project(ProjectArgs),
    /// Inspect or manually serve the local stdio MCP adapter.
    Mcp(McpArgs),
    /// Export local authority records.
    Export(ExportArgs),
    /// Reconcile observed product changes.
    Changes(ChangesArgs),
    /// List or resolve pending UserAction requests.
    Inbox(InboxArgs),
    /// Fulfill an authorized evidence-capture intent.
    Evidence(EvidenceArgs),
    /// Internal managed Codex hook entry point.
    #[command(name = "_hook", hide = true)]
    Hook(HookArgs),
    /// Internal managed MCP host launcher.
    #[command(name = "_host-launch", hide = true)]
    HostLaunch(HostLaunchArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CodexHost {
    Codex,
}

impl CodexHost {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
        }
    }
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("binding")
        .required(true)
        .args(["connection", "discover_repository"])
))]
pub struct HostLaunchArgs {
    #[arg(value_enum)]
    pub host: CodexHost,
    #[arg(long, value_parser = nonempty_string, conflicts_with = "discover_repository")]
    pub connection: Option<String>,
    #[arg(long, conflicts_with = "connection")]
    pub discover_repository: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RecordProfile {
    Record,
}

impl RecordProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Record => "record",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ConnectionMode {
    Workflow,
    ReadOnly,
}

impl ConnectionMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::ReadOnly => "read_only",
        }
    }
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long, value_enum)]
    pub host: CodexHost,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: PathBuf,
    #[arg(long)]
    pub shared: bool,
    #[arg(long, value_enum, default_value_t = RecordProfile::Record)]
    pub profile: RecordProfile,
    #[command(flatten)]
    pub runtime_home: RuntimeHomeArgs,
    #[arg(long, value_parser = nonempty_path)]
    pub mcp_command: Option<PathBuf>,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub output: ConnectionReportOutputArgs,
}

#[derive(Debug, Args, Default)]
pub struct RuntimeHomeArgs {
    #[arg(long, value_parser = nonempty_path)]
    pub home: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[arg(long, value_parser = nonempty_string, default_value = "active")]
    pub task: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub privacy_footprint: bool,
}

#[derive(Debug, Args)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct DiagnosticsArgs {
    #[command(subcommand)]
    pub command: DiagnosticsCommand,
}

#[derive(Debug, Subcommand)]
pub enum DiagnosticsCommand {
    /// Show one structured diagnostic finding and its bounded cause chain.
    Show(DiagnosticsShowArgs),
    /// Show one authoritative MCP runtime session and its findings.
    Session(DiagnosticsSessionArgs),
    /// Read workflow metrics for one registered Product Repository.
    WorkflowMetrics(DiagnosticsWorkflowMetricsArgs),
}

#[derive(Debug, Args)]
pub struct DiagnosticsShowArgs {
    #[arg(value_parser = nonempty_string)]
    pub finding_id: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct DiagnosticsSessionArgs {
    #[arg(value_parser = nonempty_string)]
    pub runtime_session_id: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct DiagnosticsWorkflowMetricsArgs {
    #[arg(long, value_parser = nonempty_path)]
    pub repo: PathBuf,
    #[arg(long, required = true)]
    pub json: bool,
}

#[derive(Debug, Args)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct PolicyArgs {
    #[command(subcommand)]
    pub command: PolicyCommand,
}

#[derive(Debug, Subcommand)]
pub enum PolicyCommand {
    /// Show the authoritative workflow policy.
    Show(PolicyShowArgs),
    /// Validate a workflow-policy file without effects.
    Validate(PolicyValidateArgs),
    /// Atomically apply a workflow-policy file.
    Apply(PolicyApplyArgs),
}

#[derive(Debug, Args)]
pub struct PolicyShowArgs {
    #[arg(long, value_parser = nonempty_path)]
    pub repo: PathBuf,
    #[arg(long, required = true)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct PolicyValidateArgs {
    #[arg(long, value_parser = nonempty_path)]
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct PolicyApplyArgs {
    #[arg(long, value_parser = nonempty_path)]
    pub repo: PathBuf,
    #[arg(long, value_parser = nonempty_path)]
    pub file: PathBuf,
    #[arg(long, required = true)]
    pub json: bool,
}

#[derive(Debug, Args)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct ConnectionArgs {
    #[command(subcommand)]
    pub command: ConnectionCommand,
}

#[derive(Debug, Subcommand)]
pub enum ConnectionCommand {
    /// Add a personal or shared Codex connection.
    Add(ConnectionAddArgs),
    /// List registered Agent Connections.
    List(ConnectionListArgs),
    /// Show current connection state.
    Status(ConnectionSelectArgs),
    /// Verify current managed configuration and stdio startup.
    #[command(
        long_about = "Actively verify managed configuration and stdio startup.\n\nEffects: rollback-only Store writeability probes, disposable protocol conformance, diagnostic reconciliation, and verification-report persistence. This does not prove managed-host operation or future availability."
    )]
    Verify(ConnectionSelectArgs),
    /// Set the exposed connection tool mode.
    Mode(ConnectionModeArgs),
    /// Remove one connection membership and matching managed content.
    Remove(ConnectionRemoveArgs),
}

#[derive(Debug, Args, Default)]
pub struct ConnectionReportOutputArgs {
    #[arg(long, conflicts_with = "verbose")]
    pub json: bool,
    #[arg(long, conflicts_with = "json")]
    pub verbose: bool,
}

#[derive(Debug, Args)]
pub struct ConnectionAddArgs {
    #[arg(value_enum)]
    pub host: Option<CodexHost>,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[command(flatten)]
    pub runtime_home: RuntimeHomeArgs,
    #[arg(long)]
    pub shared: bool,
    #[arg(long)]
    pub read_only: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub output: ConnectionReportOutputArgs,
}

#[derive(Debug, Args)]
pub struct ConnectionListArgs {
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[command(flatten)]
    pub runtime_home: RuntimeHomeArgs,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ConnectionSelectArgs {
    #[arg(value_enum)]
    pub host: Option<CodexHost>,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[command(flatten)]
    pub runtime_home: RuntimeHomeArgs,
    #[arg(long)]
    pub shared: bool,
    #[command(flatten)]
    pub output: ConnectionReportOutputArgs,
}

#[derive(Debug, Args)]
#[command(allow_missing_positional = true)]
pub struct ConnectionModeArgs {
    #[arg(value_enum)]
    pub host: Option<CodexHost>,
    #[arg(value_enum)]
    pub mode: ConnectionMode,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[command(flatten)]
    pub runtime_home: RuntimeHomeArgs,
    #[arg(long)]
    pub shared: bool,
    #[command(flatten)]
    pub output: ConnectionReportOutputArgs,
}

#[derive(Debug, Args)]
pub struct ConnectionRemoveArgs {
    #[arg(value_enum)]
    pub host: Option<CodexHost>,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[command(flatten)]
    pub runtime_home: RuntimeHomeArgs,
    #[arg(long)]
    pub shared: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub output: ConnectionReportOutputArgs,
}

#[derive(Debug, Args)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub command: ProjectCommand,
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommand {
    /// Register or select a Product Repository.
    Use(ProjectUseArgs),
    /// Show the Product Repository selected by the current directory.
    Current(JsonArgs),
    /// List registered Product Repositories.
    List(JsonArgs),
    /// Rename a registered Product Repository.
    Rename(ProjectRenameArgs),
    /// Forget a registered Product Repository.
    Forget(ProjectForgetArgs),
}

#[derive(Debug, Args)]
pub struct JsonArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ProjectUseArgs {
    #[arg(value_parser = nonempty_path)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ProjectRenameArgs {
    #[arg(value_parser = nonempty_string)]
    pub name: String,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ProjectForgetArgs {
    #[arg(value_parser = nonempty_string)]
    pub selector: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: McpCommand,
}

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// Inspect the canonical MCP launch and read surfaces without side effects.
    #[command(
        long_about = "Inspect the canonical managed entry, Registry, project state, protocol profiles, tool schemas, and host contracts.\n\nSide effects: none. Writeability is not checked and requires active verification."
    )]
    Preflight(McpPreflightArgs),
    /// Run the manual stdio MCP server.
    #[command(
        long_about = "Run the manual stdio MCP server.\n\nEffects: may create and update a manual_cli runtime session, lifecycle observations, and a terminal finding in the selected Runtime Home. This command can never create a managed_host session."
    )]
    Serve(McpBindingArgs),
}

#[derive(Debug, Args)]
pub struct McpPreflightArgs {
    #[command(flatten)]
    pub binding: McpBindingArgs,
    #[command(flatten)]
    pub output: ConnectionReportOutputArgs,
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("binding")
        .required(true)
        .args(["connection", "discover_repository"])
))]
pub struct McpBindingArgs {
    #[arg(
        long,
        requires = "host",
        conflicts_with_all = ["connection", "project"]
    )]
    pub discover_repository: bool,
    #[arg(long, value_enum, requires = "discover_repository")]
    pub host: Option<CodexHost>,
    #[arg(
        long,
        value_parser = nonempty_string,
        conflicts_with = "discover_repository"
    )]
    pub connection: Option<String>,
    #[arg(long, value_parser = nonempty_string, requires = "connection")]
    pub project: Option<String>,
}

#[derive(Debug, Args)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct ExportArgs {
    #[command(subcommand)]
    pub command: ExportCommand,
}

#[derive(Debug, Subcommand)]
pub enum ExportCommand {
    /// Export an authority bundle to a new output path.
    AuthorityBundle(AuthorityBundleArgs),
}

#[derive(Debug, Args)]
pub struct AuthorityBundleArgs {
    #[arg(long, value_parser = nonempty_path)]
    pub output: PathBuf,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct ChangesArgs {
    #[command(subcommand)]
    pub command: ChangesCommand,
}

#[derive(Debug, Subcommand)]
pub enum ChangesCommand {
    /// Reconcile observed changes for one Task.
    Reconcile(ChangesReconcileArgs),
}

#[derive(Debug, Args)]
pub struct ChangesReconcileArgs {
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[arg(long, value_parser = nonempty_string, default_value = "active")]
    pub task: String,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct InboxArgs {
    #[command(subcommand)]
    pub command: Option<InboxCommand>,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[arg(long, value_parser = nonempty_string, default_value = "active")]
    pub task: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum InboxCommand {
    /// Resolve one pending UserAction request.
    Resolve(InboxResolveArgs),
}

#[derive(Debug, Args)]
pub struct InboxResolveArgs {
    #[arg(value_parser = nonempty_string)]
    pub user_action_request_id: String,
    #[arg(long, value_parser = nonempty_string)]
    pub choice: Option<String>,
    #[arg(long, value_parser = nonempty_string)]
    pub note: Option<String>,
    #[arg(long, value_parser = nonempty_string, conflicts_with = "claim")]
    pub criterion: Option<String>,
    #[arg(long, value_parser = nonempty_string, conflicts_with = "criterion")]
    pub claim: Option<String>,
    #[arg(long, value_parser = nonempty_string, action = ArgAction::Append)]
    pub artifact: Vec<String>,
    #[arg(long, value_parser = nonempty_string)]
    pub summary: Option<String>,
    #[arg(long)]
    pub contradicted: bool,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct EvidenceArgs {
    #[command(subcommand)]
    pub command: EvidenceCommand,
}

#[derive(Debug, Subcommand)]
pub enum EvidenceCommand {
    /// Run an authorized local command and record its bounded outcome.
    CaptureCommand(EvidenceCaptureCommandArgs),
    /// Bind one authorized capture intent to Guard tool events.
    CaptureTool(EvidenceCaptureToolArgs),
}

#[derive(Debug, Args)]
pub struct EvidenceCaptureCommandArgs {
    #[arg(long, value_parser = nonempty_string)]
    pub intent: String,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
    #[arg(last = true, required = true, num_args = 1.., allow_hyphen_values = true)]
    pub program: Vec<String>,
}

#[derive(Debug, Args)]
pub struct EvidenceCaptureToolArgs {
    #[arg(long, value_parser = nonempty_string)]
    pub intent: String,
    #[arg(long, value_parser = nonempty_string)]
    pub pre_event: String,
    #[arg(long, value_parser = nonempty_string)]
    pub post_event: String,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct HookArgs {
    #[command(subcommand)]
    pub command: HookCommand,
}

#[derive(Debug, Subcommand)]
pub enum HookCommand {
    PreTool(HookEventArgs),
    PostTool(HookEventArgs),
    PromptCapture(HookEventArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum HookOutput {
    VolicordJson,
    Text,
}

#[derive(Debug, Args)]
pub struct HookEventArgs {
    #[arg(long = "file", value_parser = nonempty_path)]
    pub event_file: Option<PathBuf>,
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[arg(long, value_parser = nonempty_string)]
    pub connection: Option<String>,
    #[arg(long, value_parser = nonempty_string)]
    pub guard_installation: Option<String>,
    #[arg(long, value_enum)]
    pub host: Option<CodexHost>,
    #[arg(long, value_enum)]
    pub integration_profile: Option<RecordProfile>,
    #[arg(long, value_parser = nonempty_string)]
    pub policy_hash: Option<String>,
    #[arg(long, value_enum, conflicts_with = "host_output")]
    pub output: Option<HookOutput>,
    #[arg(long, value_enum, conflicts_with = "output")]
    pub host_output: Option<CodexHost>,
}

fn nonempty_string(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Err("value must not be empty".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

fn nonempty_path(value: &str) -> Result<PathBuf, String> {
    nonempty_string(value).map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report_output_args(command: Command) -> ConnectionReportOutputArgs {
        match command {
            Command::Init(args) => args.output,
            Command::Connection(ConnectionArgs { command }) => match command {
                ConnectionCommand::Add(args) => args.output,
                ConnectionCommand::Status(args) | ConnectionCommand::Verify(args) => args.output,
                ConnectionCommand::Mode(args) => args.output,
                ConnectionCommand::Remove(args) => args.output,
                ConnectionCommand::List(_) => panic!("list has its own collection output"),
            },
            _ => panic!("expected a selected Connection command report"),
        }
    }

    fn runtime_home_args(command: Command) -> RuntimeHomeArgs {
        match command {
            Command::Init(args) => args.runtime_home,
            Command::Connection(ConnectionArgs { command }) => match command {
                ConnectionCommand::Add(args) => args.runtime_home,
                ConnectionCommand::List(args) => args.runtime_home,
                ConnectionCommand::Status(args) | ConnectionCommand::Verify(args) => {
                    args.runtime_home
                }
                ConnectionCommand::Mode(args) => args.runtime_home,
                ConnectionCommand::Remove(args) => args.runtime_home,
            },
            _ => panic!("expected a Runtime Home-selecting command"),
        }
    }

    fn report_command_args() -> [Vec<&'static str>; 6] {
        [
            vec!["volicord", "init", "--host", "codex", "--repo", "."],
            vec!["volicord", "connection", "add", "codex", "--repo", "."],
            vec!["volicord", "connection", "status", "codex", "--repo", "."],
            vec!["volicord", "connection", "verify", "codex", "--repo", "."],
            vec![
                "volicord",
                "connection",
                "mode",
                "codex",
                "workflow",
                "--repo",
                ".",
            ],
            vec!["volicord", "connection", "remove", "codex", "--repo", "."],
        ]
    }

    fn connection_command_args() -> [Vec<&'static str>; 6] {
        [
            vec!["volicord", "connection", "add", "codex", "--repo", "."],
            vec!["volicord", "connection", "list", "--repo", "."],
            vec!["volicord", "connection", "status", "codex", "--repo", "."],
            vec!["volicord", "connection", "verify", "codex", "--repo", "."],
            vec![
                "volicord",
                "connection",
                "mode",
                "codex",
                "workflow",
                "--repo",
                ".",
            ],
            vec!["volicord", "connection", "remove", "codex", "--repo", "."],
        ]
    }

    #[test]
    fn declaration_rejects_unknown_hosts_and_missing_values() {
        let host_error =
            Cli::try_parse_from(["volicord", "init", "--host", "unsupported", "--repo", "."])
                .expect_err("the host value set must stay closed");
        assert_eq!(host_error.kind(), clap::error::ErrorKind::InvalidValue);

        let value_error = Cli::try_parse_from(["volicord", "policy", "validate", "--file"])
            .expect_err("a missing option value must be a usage error");
        assert_eq!(value_error.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn noncanonical_cli_surfaces_have_no_command_or_value_aliases() {
        for command in ["serve", "storage", "_final-output"] {
            let error = Cli::try_parse_from(["volicord", command])
                .expect_err("noncanonical commands must not be accepted as hidden aliases");
            assert_eq!(error.kind(), clap::error::ErrorKind::InvalidSubcommand);
        }

        for args in [
            vec!["volicord", "init", "--host", "claude-code", "--repo", "."],
            vec![
                "volicord",
                "init",
                "--host",
                "codex",
                "--repo",
                ".",
                "--profile",
                "detective",
            ],
            vec!["volicord", "mcp", "serve", "--local-http"],
        ] {
            Cli::try_parse_from(args)
                .expect_err("noncanonical values and transports must be rejected");
        }
    }

    #[test]
    fn version_is_declared_as_an_exclusive_root_option() {
        let error = Cli::try_parse_from(["volicord", "--version", "status"])
            .expect_err("version must conflict with commands in the clap declaration");
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);

        for old_flag in ["--stdio", "--check"] {
            let mcp_error = Cli::try_parse_from([
                "volicord",
                "mcp",
                old_flag,
                "--connection",
                "connection_example",
            ])
            .expect_err("removed MCP mode flags must be rejected");
            assert_eq!(mcp_error.kind(), clap::error::ErrorKind::UnknownArgument);
        }
    }

    #[test]
    fn declaration_selects_nested_commands_without_token_dispatch() {
        let parsed = Cli::try_parse_from([
            "volicord",
            "connection",
            "mode",
            "codex",
            "read-only",
            "--repo",
            ".",
            "--json",
        ])
        .expect("nested command should parse");
        let Some(Command::Connection(ConnectionArgs {
            command:
                ConnectionCommand::Mode(ConnectionModeArgs {
                    host: Some(CodexHost::Codex),
                    mode: ConnectionMode::ReadOnly,
                    output:
                        ConnectionReportOutputArgs {
                            json: true,
                            verbose: false,
                        },
                    ..
                }),
        })) = parsed.command
        else {
            panic!("unexpected parsed command")
        };
    }

    #[test]
    fn selected_connection_reports_accept_each_output_mode() {
        for args in report_command_args() {
            let parsed = Cli::try_parse_from(&args).expect("default output should parse");
            let output = report_output_args(parsed.command.expect("selected command"));
            assert!(!output.json);
            assert!(!output.verbose);

            let mut verbose_args = args.clone();
            verbose_args.push("--verbose");
            let parsed = Cli::try_parse_from(verbose_args).expect("verbose output should parse");
            let output = report_output_args(parsed.command.expect("selected command"));
            assert!(!output.json);
            assert!(output.verbose);

            let mut json_args = args;
            json_args.push("--json");
            let parsed = Cli::try_parse_from(json_args).expect("JSON output should parse");
            let output = report_output_args(parsed.command.expect("selected command"));
            assert!(output.json);
            assert!(!output.verbose);
        }
    }

    #[test]
    fn selected_connection_report_output_modes_conflict_in_clap() {
        for mut args in report_command_args() {
            args.extend(["--verbose", "--json"]);
            let error = Cli::try_parse_from(args)
                .expect_err("verbose and JSON output must conflict in the declaration");
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        }
    }

    #[test]
    fn every_connection_command_accepts_one_nonempty_runtime_home() {
        for mut args in connection_command_args() {
            args.extend(["--home", "relative-runtime-home"]);
            let parsed = Cli::try_parse_from(args).expect("Runtime Home should parse");
            assert_eq!(
                runtime_home_args(parsed.command.expect("connection command")).home,
                Some(PathBuf::from("relative-runtime-home"))
            );
        }

        for mut args in connection_command_args() {
            args.extend(["--home", ""]);
            let error = Cli::try_parse_from(args)
                .expect_err("an empty explicit Runtime Home must be rejected");
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        }
    }

    #[test]
    fn runtime_home_selection_is_independent_of_connection_output_flags() {
        for mut args in connection_command_args() {
            args.extend(["--home", "relative-runtime-home", "--json"]);
            let parsed = Cli::try_parse_from(args)
                .expect("Runtime Home and JSON selection should parse together");
            assert_eq!(
                runtime_home_args(parsed.command.expect("connection command")).home,
                Some(PathBuf::from("relative-runtime-home"))
            );
        }

        for mut args in report_command_args().into_iter().skip(1) {
            args.extend(["--home", "relative-runtime-home", "--verbose"]);
            let parsed = Cli::try_parse_from(args)
                .expect("Runtime Home and verbose selection should parse together");
            assert!(
                runtime_home_args(parsed.command.expect("connection command"))
                    .home
                    .is_some()
            );
        }
    }

    #[test]
    fn connection_list_keeps_its_collection_output_contract() {
        let parsed = Cli::try_parse_from(["volicord", "connection", "list", "--json"])
            .expect("list JSON should remain available");
        let Some(Command::Connection(ConnectionArgs {
            command: ConnectionCommand::List(ConnectionListArgs { json: true, .. }),
        })) = parsed.command
        else {
            panic!("unexpected list command")
        };

        let error = Cli::try_parse_from(["volicord", "connection", "list", "--verbose"])
            .expect_err("list must not gain selected-report verbose output");
        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }
}
