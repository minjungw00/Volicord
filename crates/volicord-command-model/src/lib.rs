//! The complete command-line declaration for the `volicord` binary.

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, error::Error, ffi::OsString, fmt, path::PathBuf};

use clap::{
    error::ErrorKind, Arg, ArgAction, ArgGroup, Args, Command as ClapCommand, CommandFactory,
    Parser, Subcommand, ValueEnum,
};

#[derive(Debug, Parser)]
#[command(
    name = "volicord",
    about = "Local Volicord administration and managed stdio MCP",
    disable_version_flag = true,
    arg_required_else_help = true,
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    /// Print the Volicord version.
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

impl Cli {
    /// Parses one `volicord` invocation with the canonical command model.
    pub fn try_parse_from<I, T>(arguments: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        <Self as Parser>::try_parse_from(arguments)
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Show the Volicord version and build provenance.
    Version(VersionArgs),
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
    /// Reconcile Unrecorded Changes in the Product Repository.
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

#[derive(Debug, Args)]
pub struct VersionArgs {
    #[command(flatten)]
    pub output: ReportOutputArgs,
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
    pub output: ReportOutputArgs,
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
    #[command(flatten)]
    pub output: ReportOutputArgs,
    #[arg(long, conflicts_with = "verbose")]
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
    #[command(flatten)]
    pub output: ReportOutputArgs,
}

#[derive(Debug, Args)]
pub struct PolicyValidateArgs {
    #[arg(long, value_parser = nonempty_path)]
    pub file: PathBuf,
    #[arg(long)]
    pub json: bool,
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

#[derive(Debug, Args, Default, PartialEq, Eq)]
pub struct ReportOutputArgs {
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
    pub output: ReportOutputArgs,
}

#[derive(Debug, Args)]
pub struct ConnectionListArgs {
    #[arg(long, value_parser = nonempty_path)]
    pub repo: Option<PathBuf>,
    #[command(flatten)]
    pub runtime_home: RuntimeHomeArgs,
    #[command(flatten)]
    pub output: ReportOutputArgs,
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
    pub output: ReportOutputArgs,
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
    pub output: ReportOutputArgs,
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
    pub output: ReportOutputArgs,
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
    pub output: ReportOutputArgs,
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
    /// Reconcile Unrecorded Changes for one Task.
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

/// Typed answer arguments for one current inbox-resolution invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboxResolutionArguments {
    /// Identify the selected request without preselecting answer arguments.
    Pending,
    /// Submit one stored choice and an optional private note.
    Choice {
        choice: String,
        note: Option<String>,
    },
    /// Submit one evidence observation over typed target and artifact coordinates.
    EvidenceObservation {
        target: InboxEvidenceTarget,
        artifact_ids: Vec<String>,
        summary: String,
        contradicted: bool,
    },
}

/// Typed target selector accepted by an evidence-observation resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboxEvidenceTarget {
    AcceptanceCriterion(String),
    EvidenceClaim(String),
}

/// One typed invocation of the current `inbox resolve` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxResolveInvocation {
    user_action_request_id: String,
    resolution: InboxResolutionArguments,
}

impl InboxResolveInvocation {
    /// Constructs a typed current invocation.
    pub fn new(
        user_action_request_id: impl Into<String>,
        resolution: InboxResolutionArguments,
    ) -> Self {
        Self {
            user_action_request_id: user_action_request_id.into(),
            resolution,
        }
    }

    /// Returns the request coordinate carried by this invocation.
    pub fn user_action_request_id(&self) -> &str {
        &self.user_action_request_id
    }

    /// Returns the typed resolution arguments carried by this invocation.
    pub const fn resolution(&self) -> &InboxResolutionArguments {
        &self.resolution
    }

    /// Produces canonical argument tokens from the actual Clap declaration.
    pub fn canonical_arguments(&self) -> Result<Vec<String>, CommandIntrospectionError> {
        let root = root_command();
        let command_path = find_unique_command_path_with_argument(&root, "user_action_request_id")
            .ok_or_else(|| CommandIntrospectionError {
                command_path: vec!["inbox-resolution".to_owned()],
            })?;
        let command =
            find_command(&root, &command_path).ok_or_else(|| CommandIntrospectionError {
                command_path: command_path.clone(),
            })?;
        let mut arguments = std::iter::once(root.get_name().to_owned())
            .chain(command_path.iter().cloned())
            .chain(std::iter::once(self.user_action_request_id.clone()))
            .collect::<Vec<_>>();

        match &self.resolution {
            InboxResolutionArguments::Pending => {}
            InboxResolutionArguments::Choice { choice, note } => {
                append_declared_option_value(
                    command,
                    &command_path,
                    "choice",
                    choice,
                    &mut arguments,
                )?;
                if let Some(note) = note {
                    append_declared_option_value(
                        command,
                        &command_path,
                        "note",
                        note,
                        &mut arguments,
                    )?;
                }
            }
            InboxResolutionArguments::EvidenceObservation {
                target,
                artifact_ids,
                summary,
                contradicted,
            } => {
                match target {
                    InboxEvidenceTarget::AcceptanceCriterion(id) => {
                        append_declared_option_value(
                            command,
                            &command_path,
                            "criterion",
                            id,
                            &mut arguments,
                        )?;
                    }
                    InboxEvidenceTarget::EvidenceClaim(id) => {
                        append_declared_option_value(
                            command,
                            &command_path,
                            "claim",
                            id,
                            &mut arguments,
                        )?;
                    }
                }
                for artifact_id in artifact_ids {
                    append_declared_option_value(
                        command,
                        &command_path,
                        "artifact",
                        artifact_id,
                        &mut arguments,
                    )?;
                }
                append_declared_option_value(
                    command,
                    &command_path,
                    "summary",
                    summary,
                    &mut arguments,
                )?;
                if *contradicted {
                    append_declared_flag(command, &command_path, "contradicted", &mut arguments)?;
                }
            }
        }

        Cli::try_parse_from(&arguments).map_err(|_| CommandIntrospectionError {
            command_path: command_path.clone(),
        })?;
        Ok(arguments)
    }
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

/// Returns a fresh root Clap command for the complete `volicord` surface.
pub fn root_command() -> ClapCommand {
    <Cli as CommandFactory>::command()
}

/// Visibility inherited by a declared command path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandVisibility {
    /// An ordinary command exposed through public traversal and help.
    Public,
    /// An internal command hidden by its own declaration or a hidden ancestor.
    Hidden,
}

/// Introspection data for one explicitly declared command path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPath {
    components: Vec<String>,
    visibility: CommandVisibility,
    invocable: bool,
    synopsis: String,
}

impl CommandPath {
    /// Returns the command names below the `volicord` root.
    pub fn components(&self) -> &[String] {
        &self.components
    }

    /// Returns the visibility inherited from the actual Clap command tree.
    pub const fn visibility(&self) -> CommandVisibility {
        self.visibility
    }

    /// Returns whether this exact path can run without selecting another subcommand.
    pub const fn is_invocable(&self) -> bool {
        self.invocable
    }

    /// Returns the canonical synopsis rendered by Clap for this path.
    pub fn synopsis(&self) -> &str {
        &self.synopsis
    }
}

/// Returns every explicitly declared command path, including hidden paths.
pub fn command_paths() -> Vec<CommandPath> {
    let root = root_command();
    let mut paths = Vec::new();
    collect_command_paths(
        &root,
        &mut Vec::new(),
        CommandVisibility::Public,
        &mut paths,
    );
    paths
}

/// Returns every public command path and excludes complete hidden subtrees.
pub fn public_command_paths() -> Vec<CommandPath> {
    command_paths()
        .into_iter()
        .filter(|path| path.visibility == CommandVisibility::Public)
        .collect()
}

/// Syntax-only introspection data for the root command or one public command path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSyntax {
    components: Vec<String>,
    synopsis: String,
    arguments: Vec<String>,
}

impl CommandSyntax {
    /// Returns the command names below the `volicord` root.
    ///
    /// The root command has no components.
    pub fn components(&self) -> &[String] {
        &self.components
    }

    /// Returns the canonical synopsis rendered by Clap.
    pub fn synopsis(&self) -> &str {
        &self.synopsis
    }

    /// Returns the visible positional, option, and flag forms declared by Clap.
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
}

/// Returns syntax-only data for the root and every public command path.
pub fn public_command_syntax() -> Vec<CommandSyntax> {
    let mut root = root_command();
    root.build();
    let mut syntax = vec![command_syntax(&root, &[])];
    syntax.extend(public_command_paths().into_iter().map(|path| {
        let command = find_command(&root, path.components())
            .expect("public command paths originate from this command declaration");
        command_syntax(command, path.components())
    }));
    syntax
}

/// Exact syntax and closed values for one stable semantic CLI contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliContractDescriptor {
    id: String,
    syntax: BTreeSet<String>,
    values: BTreeSet<String>,
    related_contracts: Vec<String>,
}

impl CliContractDescriptor {
    /// Returns the stable semantic contract identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns exact command paths, options, and positional argument identifiers.
    pub const fn syntax(&self) -> &BTreeSet<String> {
        &self.syntax
    }

    /// Returns exact closed command values.
    pub const fn values(&self) -> &BTreeSet<String> {
        &self.values
    }

    /// Returns deliberate semantic relationships to child command contracts.
    pub fn related_contracts(&self) -> &[String] {
        &self.related_contracts
    }
}

/// Returns exact descriptors derived from the current public Clap declaration.
pub fn public_cli_contract_descriptors() -> Vec<CliContractDescriptor> {
    let mut root = root_command();
    root.build();
    let mut descriptors = Vec::new();
    let public_paths = public_command_syntax();

    for syntax in &public_paths {
        let command = find_command(&root, &syntax.components)
            .expect("public command syntax originates from this command declaration");
        let mut syntax_identifiers = BTreeSet::new();
        let mut values = BTreeSet::new();
        let command_path = std::iter::once("volicord")
            .chain(syntax.components.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        syntax_identifiers.insert(command_path);

        for argument in command
            .get_arguments()
            .filter(|argument| !argument.is_hide_set() && !is_display_action(argument.get_action()))
        {
            if argument.is_positional() {
                syntax_identifiers.insert(argument.get_id().as_str().to_owned());
            }
            if let Some(long) = argument.get_long() {
                syntax_identifiers.insert(format!("--{long}"));
            }
            if let Some(short) = argument.get_short() {
                syntax_identifiers.insert(format!("-{short}"));
            }
            values.extend(
                argument
                    .get_possible_values()
                    .into_iter()
                    .filter(|value| !value.is_hide_set())
                    .map(|value| value.get_name().to_owned()),
            );
        }

        let related_contracts = public_paths
            .iter()
            .filter(|candidate| {
                candidate.components.len() == syntax.components.len() + 1
                    && candidate.components.starts_with(&syntax.components)
            })
            .map(|candidate| cli_contract_id(&candidate.components))
            .collect();
        descriptors.push(CliContractDescriptor {
            id: cli_contract_id(&syntax.components),
            syntax: syntax_identifiers,
            values,
            related_contracts,
        });
    }

    let mut public_syntax = BTreeSet::new();
    let mut public_values = BTreeSet::new();
    for descriptor in &descriptors {
        public_syntax.extend(descriptor.syntax.iter().cloned());
        public_values.extend(descriptor.values.iter().cloned());
    }
    descriptors.push(CliContractDescriptor {
        id: "cli.surface.public".to_owned(),
        syntax: public_syntax,
        values: public_values,
        related_contracts: descriptors
            .iter()
            .map(|descriptor| descriptor.id.clone())
            .collect(),
    });
    descriptors
}

fn cli_contract_id(components: &[String]) -> String {
    if components.is_empty() {
        "cli.command.root".to_owned()
    } else {
        format!("cli.command.{}", components.join(".").replace('-', "_"))
    }
}

fn command_syntax(command: &ClapCommand, components: &[String]) -> CommandSyntax {
    CommandSyntax {
        components: components.to_vec(),
        synopsis: canonical_synopsis(command, components),
        arguments: command
            .get_arguments()
            .filter(|argument| !argument.is_hide_set() && !is_display_action(argument.get_action()))
            .map(argument_syntax)
            .collect(),
    }
}

fn argument_syntax(argument: &Arg) -> String {
    let mut syntax = argument.to_string();
    if let (Some(short), Some(_)) = (argument.get_short(), argument.get_long()) {
        syntax = format!("-{short}, {syntax}");
    }

    let possible_values = argument
        .get_possible_values()
        .into_iter()
        .filter(|value| !value.is_hide_set())
        .map(|value| value.get_name().to_owned())
        .collect::<Vec<_>>();
    if !possible_values.is_empty() {
        syntax.push_str(" {");
        syntax.push_str(&possible_values.join("|"));
        syntax.push('}');
    }
    syntax
}

fn collect_command_paths(
    command: &ClapCommand,
    parent_components: &mut Vec<String>,
    parent_visibility: CommandVisibility,
    paths: &mut Vec<CommandPath>,
) {
    for subcommand in command.get_subcommands() {
        parent_components.push(subcommand.get_name().to_owned());
        let visibility =
            if parent_visibility == CommandVisibility::Hidden || subcommand.is_hide_set() {
                CommandVisibility::Hidden
            } else {
                CommandVisibility::Public
            };
        paths.push(CommandPath {
            components: parent_components.clone(),
            visibility,
            invocable: !subcommand.is_subcommand_required_set(),
            synopsis: canonical_synopsis(subcommand, parent_components),
        });
        collect_command_paths(subcommand, parent_components, visibility, paths);
        parent_components.pop();
    }
}

fn canonical_synopsis(command: &ClapCommand, components: &[String]) -> String {
    let bin_name = std::iter::once("volicord")
        .chain(components.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    command
        .clone()
        .bin_name(bin_name)
        .render_usage()
        .to_string()
}

/// One minimal public invocation generated from the command declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalInvocation {
    command_path: Vec<String>,
    arguments: Vec<String>,
}

impl CanonicalInvocation {
    /// Returns the command names below the `volicord` root.
    pub fn command_path(&self) -> &[String] {
        &self.command_path
    }

    /// Returns the complete argument vector, including the executable name.
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
}

/// Failure to derive a parseable canonical invocation from a declared path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandIntrospectionError {
    command_path: Vec<String>,
}

impl CommandIntrospectionError {
    /// Returns the command path whose current declaration could not be materialized.
    pub fn command_path(&self) -> &[String] {
        &self.command_path
    }
}

impl fmt::Display for CommandIntrospectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "could not derive a canonical invocation for `volicord {}`",
            self.command_path.join(" ")
        )
    }
}

impl Error for CommandIntrospectionError {}

/// Generates one minimal parseable invocation for every invocable public path.
pub fn canonical_public_invocations() -> Result<Vec<CanonicalInvocation>, CommandIntrospectionError>
{
    let root = root_command();
    public_command_paths()
        .into_iter()
        .filter(CommandPath::is_invocable)
        .map(|path| {
            let command = find_command(&root, path.components())
                .expect("command paths originate from this command declaration");
            canonical_invocation(&root, &path, command).map(|arguments| CanonicalInvocation {
                command_path: path.components,
                arguments,
            })
        })
        .collect()
}

fn find_command<'a>(root: &'a ClapCommand, command_path: &[String]) -> Option<&'a ClapCommand> {
    let mut command = root;
    for component in command_path {
        command = command.find_subcommand(component)?;
    }
    Some(command)
}

fn find_unique_command_path_with_argument(
    root: &ClapCommand,
    argument_id: &str,
) -> Option<Vec<String>> {
    fn collect(
        command: &ClapCommand,
        argument_id: &str,
        path: &mut Vec<String>,
        matches: &mut Vec<Vec<String>>,
    ) {
        if command
            .get_arguments()
            .any(|argument| argument.get_id().as_str() == argument_id)
        {
            matches.push(path.clone());
        }
        for subcommand in command.get_subcommands() {
            path.push(subcommand.get_name().to_owned());
            collect(subcommand, argument_id, path, matches);
            path.pop();
        }
    }

    let mut matches = Vec::new();
    collect(root, argument_id, &mut Vec::new(), &mut matches);
    (matches.len() == 1).then(|| matches.remove(0))
}

fn declared_long_option<'a>(
    command: &'a ClapCommand,
    argument_id: &str,
    command_path: &[String],
) -> Result<&'a str, CommandIntrospectionError> {
    command
        .get_arguments()
        .find(|argument| argument.get_id().as_str() == argument_id)
        .and_then(Arg::get_long)
        .ok_or_else(|| CommandIntrospectionError {
            command_path: command_path.to_vec(),
        })
}

fn append_declared_option_value(
    command: &ClapCommand,
    command_path: &[String],
    argument_id: &str,
    value: &str,
    arguments: &mut Vec<String>,
) -> Result<(), CommandIntrospectionError> {
    let long = declared_long_option(command, argument_id, command_path)?;
    arguments.push(format!("--{long}"));
    arguments.push(value.to_owned());
    Ok(())
}

fn append_declared_flag(
    command: &ClapCommand,
    command_path: &[String],
    argument_id: &str,
    arguments: &mut Vec<String>,
) -> Result<(), CommandIntrospectionError> {
    let long = declared_long_option(command, argument_id, command_path)?;
    arguments.push(format!("--{long}"));
    Ok(())
}

fn canonical_invocation(
    root: &ClapCommand,
    command_path: &CommandPath,
    command: &ClapCommand,
) -> Result<Vec<String>, CommandIntrospectionError> {
    let mut arguments = vec![root.get_name().to_owned()];
    let mut current = root;
    for component in command_path.components() {
        append_required_arguments(current, &mut arguments);
        arguments.push(component.clone());
        current = current
            .find_subcommand(component)
            .expect("command paths originate from this command declaration");
    }
    append_required_arguments(current, &mut arguments);

    if Cli::try_parse_from(&arguments).is_ok() {
        return Ok(arguments);
    }

    let candidates = command
        .get_arguments()
        .filter(|argument| {
            !argument.is_required_set()
                && !argument.is_hide_set()
                && !is_display_action(argument.get_action())
        })
        .filter_map(argument_tokens)
        .collect::<Vec<_>>();

    for count in 1..=candidates.len() {
        for mask in 1_usize..(1_usize << candidates.len()) {
            if mask.count_ones() as usize != count {
                continue;
            }
            let mut attempt = arguments.clone();
            let insertion = attempt
                .iter()
                .position(|argument| argument == "--")
                .unwrap_or(attempt.len());
            let additions = candidates
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1_usize << index) != 0)
                .flat_map(|(_, tokens)| tokens.iter().cloned())
                .collect::<Vec<_>>();
            attempt.splice(insertion..insertion, additions);
            if Cli::try_parse_from(&attempt).is_ok() {
                return Ok(attempt);
            }
        }
    }

    Err(CommandIntrospectionError {
        command_path: command_path.components.clone(),
    })
}

fn append_required_arguments(command: &ClapCommand, arguments: &mut Vec<String>) {
    let required = command
        .get_arguments()
        .filter(|argument| argument.is_required_set())
        .collect::<Vec<_>>();
    for argument in required
        .iter()
        .copied()
        .filter(|argument| !argument.is_positional())
    {
        arguments.extend(
            argument_tokens(argument).expect("required command options have a canonical spelling"),
        );
    }
    for argument in required
        .iter()
        .copied()
        .filter(|argument| argument.is_positional())
    {
        arguments.extend(
            argument_tokens(argument)
                .expect("required positional arguments accept a canonical value"),
        );
    }
}

fn argument_tokens(argument: &Arg) -> Option<Vec<String>> {
    if is_display_action(argument.get_action()) {
        return None;
    }

    let mut tokens = Vec::new();
    if argument.is_positional() {
        if argument.is_last_set() {
            tokens.push("--".to_owned());
        }
    } else if let Some(long) = argument.get_long() {
        tokens.push(format!("--{long}"));
    } else if let Some(short) = argument.get_short() {
        tokens.push(format!("-{short}"));
    } else {
        return None;
    }

    if argument.get_action().takes_values() {
        let value_count = argument
            .get_num_args()
            .map_or(1, |range| range.min_values().max(1));
        let value = argument
            .get_possible_values()
            .into_iter()
            .find(|value| !value.is_hide_set())
            .map_or_else(|| "value".to_owned(), |value| value.get_name().to_owned());
        if argument.is_require_equals_set() && !argument.is_positional() {
            let option = tokens
                .pop()
                .expect("value-taking non-positional arguments have an option spelling");
            tokens.push(format!("{option}={value}"));
            tokens.extend(std::iter::repeat_n(value, value_count.saturating_sub(1)));
        } else {
            tokens.extend(std::iter::repeat_n(value, value_count));
        }
    }

    Some(tokens)
}

fn is_display_action(action: &ArgAction) -> bool {
    matches!(
        action,
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

/// Failure to validate a documented invocation against the public command surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicInvocationError {
    message: String,
}

impl fmt::Display for PublicInvocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PublicInvocationError {}

/// Validates syntax and rejects any invocation that enters a hidden command subtree.
pub fn validate_public_invocation<I, T>(arguments: I) -> Result<(), PublicInvocationError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let arguments = arguments
        .into_iter()
        .map(Into::into)
        .collect::<Vec<OsString>>();
    let parse_result = root_command().try_get_matches_from(arguments.clone());
    if let Err(error) = parse_result {
        if !matches!(
            error.kind(),
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
        ) {
            return Err(PublicInvocationError {
                message: error.to_string(),
            });
        }
    }

    if invocation_visibility(&arguments) == CommandVisibility::Hidden {
        return Err(PublicInvocationError {
            message: "hidden `volicord` commands are not part of the public command surface"
                .to_owned(),
        });
    }

    Ok(())
}

fn invocation_visibility(arguments: &[OsString]) -> CommandVisibility {
    let Ok(matches) = root_command()
        .ignore_errors(true)
        .try_get_matches_from(arguments)
    else {
        return CommandVisibility::Public;
    };
    let root = root_command();
    let mut command = &root;
    let mut matches = &matches;
    let mut visibility = CommandVisibility::Public;

    while let Some((name, subcommand_matches)) = matches.subcommand() {
        let Some(subcommand) = command.find_subcommand(name) else {
            break;
        };
        if subcommand.is_hide_set() {
            visibility = CommandVisibility::Hidden;
        }
        command = subcommand;
        matches = subcommand_matches;
    }

    visibility
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn command_declaration_passes_clap_structural_assertions() {
        root_command().debug_assert();
    }

    #[test]
    fn public_commands_are_traversable_with_canonical_synopses() {
        let root = root_command();
        let public_paths = public_command_paths();
        assert!(!public_paths.is_empty());

        for path in &public_paths {
            let mut command = &root;
            for component in path.components() {
                command = command
                    .find_subcommand(component)
                    .expect("public traversal must resolve every declared component");
                assert!(!command.is_hide_set());
            }
            assert!(
                path.synopsis().starts_with("Usage: volicord "),
                "unexpected synopsis for {:?}: {}",
                path.components(),
                path.synopsis()
            );
        }

        let expected = command_paths()
            .into_iter()
            .filter(|path| path.visibility() == CommandVisibility::Public)
            .map(|path| path.components)
            .collect::<BTreeSet<_>>();
        let traversed = public_paths
            .into_iter()
            .map(|path| path.components)
            .collect::<BTreeSet<_>>();
        assert_eq!(traversed, expected);
    }

    #[test]
    fn hidden_command_subtrees_are_excluded_from_public_traversal() {
        let hidden = command_paths()
            .into_iter()
            .filter(|path| path.visibility() == CommandVisibility::Hidden)
            .map(|path| path.components)
            .collect::<BTreeSet<_>>();
        assert!(hidden.contains(&vec!["_hook".to_owned()]));
        assert!(hidden.contains(&vec!["_host-launch".to_owned()]));
        assert!(hidden
            .iter()
            .any(|path| path.starts_with(&["_hook".to_owned()]) && path.len() > 1));

        let public = public_command_paths()
            .into_iter()
            .map(|path| path.components)
            .collect::<BTreeSet<_>>();
        assert!(hidden.is_disjoint(&public));

        for arguments in [
            vec!["volicord", "_hook", "pre-tool"],
            vec![
                "volicord",
                "_host-launch",
                "codex",
                "--connection",
                "connection_1",
            ],
        ] {
            validate_public_invocation(arguments)
                .expect_err("internal commands must not validate as public invocations");
        }
    }

    #[test]
    fn public_syntax_is_derived_from_visible_clap_declarations() {
        let syntax = public_command_syntax();
        let paths = syntax
            .iter()
            .map(|command| command.components().to_vec())
            .collect::<BTreeSet<_>>();

        assert!(paths.contains(&Vec::new()));
        assert!(paths.contains(&vec!["evidence".to_owned()]));
        assert!(paths.contains(&vec!["evidence".to_owned(), "capture-command".to_owned()]));

        let doctor = syntax
            .iter()
            .find(|command| command.components() == ["doctor"])
            .expect("doctor syntax");
        assert!(doctor
            .arguments()
            .iter()
            .any(|argument| argument == "--privacy-footprint"));
        assert!(doctor
            .arguments()
            .iter()
            .any(|argument| argument == "--verbose"));

        for path in [
            ["policy", "show"].as_slice(),
            ["policy", "validate"].as_slice(),
            ["policy", "apply"].as_slice(),
        ] {
            let command = syntax
                .iter()
                .find(|command| command.components() == path)
                .expect("policy syntax");
            assert!(command
                .arguments()
                .iter()
                .any(|argument| argument == "--json"));
        }
        let policy_apply = syntax
            .iter()
            .find(|command| command.components() == ["policy", "apply"])
            .expect("policy apply syntax");
        assert!(policy_apply.synopsis().contains("--json"));

        let hidden = command_paths()
            .into_iter()
            .filter(|path| path.visibility() == CommandVisibility::Hidden)
            .map(|path| path.components)
            .collect::<BTreeSet<_>>();
        assert!(hidden.is_disjoint(&paths));
    }

    #[test]
    fn public_contract_descriptors_are_derived_from_visible_commands() {
        let descriptors = public_cli_contract_descriptors();
        let identifiers = descriptors
            .iter()
            .flat_map(|descriptor| descriptor.syntax().iter().chain(descriptor.values().iter()))
            .map(String::as_str)
            .collect::<BTreeSet<_>>();

        assert!(identifiers.contains("volicord policy show"));
        assert!(identifiers.contains("--repo"));
        assert!(identifiers.contains("workflow"));
        assert!(!identifiers.contains("_host-launch"));
        assert!(descriptors
            .iter()
            .any(|descriptor| descriptor.id() == "cli.command.policy.show"));
    }

    #[test]
    fn canonical_public_invocations_parse_with_the_same_model() {
        let invocations =
            canonical_public_invocations().expect("every public endpoint must materialize");
        assert!(!invocations.is_empty());

        let expected_paths = public_command_paths()
            .into_iter()
            .filter(CommandPath::is_invocable)
            .map(|path| path.components)
            .collect::<BTreeSet<_>>();
        let actual_paths = invocations
            .iter()
            .map(|invocation| invocation.command_path().to_vec())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual_paths, expected_paths);

        for invocation in invocations {
            Cli::try_parse_from(invocation.arguments()).unwrap_or_else(|error| {
                panic!(
                    "canonical invocation for {:?} did not parse: {error}",
                    invocation.command_path()
                )
            });
            validate_public_invocation(invocation.arguments()).unwrap_or_else(|error| {
                panic!(
                    "canonical invocation for {:?} was not public: {error}",
                    invocation.command_path()
                )
            });
        }
    }

    #[test]
    fn typed_inbox_resolution_invocations_round_trip_through_clap() {
        let invocations = [
            InboxResolveInvocation::new("user_action_pending", InboxResolutionArguments::Pending),
            InboxResolveInvocation::new(
                "user_action_choice",
                InboxResolutionArguments::Choice {
                    choice: "accept".to_owned(),
                    note: Some("approved locally".to_owned()),
                },
            ),
            InboxResolveInvocation::new(
                "user_action_observation",
                InboxResolutionArguments::EvidenceObservation {
                    target: InboxEvidenceTarget::EvidenceClaim("claim_1".to_owned()),
                    artifact_ids: vec!["artifact_1".to_owned(), "artifact_2".to_owned()],
                    summary: "Observed the current artifacts.".to_owned(),
                    contradicted: true,
                },
            ),
            InboxResolveInvocation::new(
                "user_action_criterion_observation",
                InboxResolutionArguments::EvidenceObservation {
                    target: InboxEvidenceTarget::AcceptanceCriterion("criterion_1".to_owned()),
                    artifact_ids: vec!["artifact_3".to_owned()],
                    summary: "Observed the current criterion.".to_owned(),
                    contradicted: false,
                },
            ),
        ];

        for invocation in invocations {
            let arguments = invocation
                .canonical_arguments()
                .expect("typed current invocation should materialize");
            let parsed = Cli::try_parse_from(&arguments)
                .expect("canonical tokens should parse through the actual command model");
            let Some(Command::Inbox(InboxArgs {
                command: Some(InboxCommand::Resolve(parsed)),
                ..
            })) = parsed.command
            else {
                panic!("canonical tokens selected the wrong command");
            };
            assert_eq!(
                parsed.user_action_request_id,
                invocation.user_action_request_id()
            );
            match invocation.resolution() {
                InboxResolutionArguments::Choice { choice, note } => {
                    assert_eq!(parsed.choice.as_deref(), Some(choice.as_str()));
                    assert_eq!(parsed.note.as_deref(), note.as_deref());
                    assert!(parsed.criterion.is_none());
                    assert!(parsed.claim.is_none());
                    assert!(parsed.artifact.is_empty());
                    assert!(parsed.summary.is_none());
                    assert!(!parsed.contradicted);
                }
                InboxResolutionArguments::EvidenceObservation {
                    target,
                    artifact_ids,
                    summary,
                    contradicted,
                } => {
                    match target {
                        InboxEvidenceTarget::AcceptanceCriterion(id) => {
                            assert_eq!(parsed.criterion.as_deref(), Some(id.as_str()));
                            assert!(parsed.claim.is_none());
                        }
                        InboxEvidenceTarget::EvidenceClaim(id) => {
                            assert_eq!(parsed.claim.as_deref(), Some(id.as_str()));
                            assert!(parsed.criterion.is_none());
                        }
                    }
                    assert_eq!(parsed.artifact.as_slice(), artifact_ids.as_slice());
                    assert_eq!(parsed.summary.as_deref(), Some(summary.as_str()));
                    assert_eq!(parsed.contradicted, *contradicted);
                    assert!(parsed.choice.is_none());
                    assert!(parsed.note.is_none());
                }
                InboxResolutionArguments::Pending => {
                    assert!(parsed.choice.is_none());
                    assert!(parsed.note.is_none());
                    assert!(parsed.criterion.is_none());
                    assert!(parsed.claim.is_none());
                    assert!(parsed.artifact.is_empty());
                    assert!(parsed.summary.is_none());
                    assert!(!parsed.contradicted);
                }
            }
        }
    }

    fn report_output_args(command: Command) -> ReportOutputArgs {
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
    fn required_arguments_and_closed_values_are_enforced_by_clap() {
        let required_error =
            Cli::try_parse_from(["volicord", "init", "--host", "codex", "--profile", "record"])
                .expect_err("init requires a repository");
        assert_eq!(
            required_error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );

        let host_error =
            Cli::try_parse_from(["volicord", "init", "--host", "unsupported", "--repo", "."])
                .expect_err("the host value set must stay closed");
        assert_eq!(host_error.kind(), clap::error::ErrorKind::InvalidValue);

        let value_error = Cli::try_parse_from(["volicord", "policy", "validate", "--file"])
            .expect_err("a missing option value must be a usage error");
        assert_eq!(value_error.kind(), clap::error::ErrorKind::InvalidValue);

        assert_eq!(
            CodexHost::value_variants(),
            &[CodexHost::Codex],
            "host values come from the command model"
        );
        assert_eq!(
            RecordProfile::value_variants(),
            &[RecordProfile::Record],
            "profile values come from the command model"
        );
        assert_eq!(
            ConnectionMode::value_variants(),
            &[ConnectionMode::Workflow, ConnectionMode::ReadOnly],
            "connection modes come from the command model"
        );
    }

    #[test]
    fn output_conflicts_are_enforced_by_clap() {
        let version_conflict = Cli::try_parse_from(["volicord", "version", "--verbose", "--json"])
            .expect_err("version output modes conflict");
        assert_eq!(
            version_conflict.kind(),
            clap::error::ErrorKind::ArgumentConflict
        );

        let conflict = Cli::try_parse_from([
            "volicord",
            "connection",
            "status",
            "codex",
            "--verbose",
            "--json",
        ])
        .expect_err("selected Connection output modes conflict");
        assert_eq!(conflict.kind(), clap::error::ErrorKind::ArgumentConflict);

        for arguments in [
            ["volicord", "doctor", "--verbose", "--json"].as_slice(),
            ["volicord", "doctor", "--privacy-footprint", "--verbose"].as_slice(),
        ] {
            let conflict = Cli::try_parse_from(arguments)
                .expect_err("doctor output modes must remain mutually exclusive");
            assert_eq!(conflict.kind(), clap::error::ErrorKind::ArgumentConflict);
        }

        for arguments in [
            ["volicord", "doctor"].as_slice(),
            ["volicord", "doctor", "--verbose"].as_slice(),
            ["volicord", "doctor", "--json"].as_slice(),
            ["volicord", "doctor", "--privacy-footprint"].as_slice(),
            ["volicord", "doctor", "--privacy-footprint", "--json"].as_slice(),
        ] {
            Cli::try_parse_from(arguments).expect("supported doctor output mode must parse");
        }
    }

    #[test]
    fn clap_rejects_unknown_commands_and_options_generically() {
        let command_error = Cli::try_parse_from(["volicord", "not-a-command"])
            .expect_err("an undeclared command must be rejected");
        assert_eq!(
            command_error.kind(),
            clap::error::ErrorKind::InvalidSubcommand
        );

        let option_error = Cli::try_parse_from(["volicord", "doctor", "--not-an-option"])
            .expect_err("an undeclared option must be rejected");
        assert_eq!(option_error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn version_is_declared_as_an_exclusive_root_option() {
        let error = Cli::try_parse_from(["volicord", "--version", "status"])
            .expect_err("version must conflict with commands in the clap declaration");
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn mcp_commands_parse_each_current_binding_form() {
        let serve = Cli::try_parse_from([
            "volicord",
            "mcp",
            "serve",
            "--connection",
            "connection_example",
        ])
        .expect("the public manual MCP subcommand must parse");
        assert!(matches!(
            serve.command,
            Some(Command::Mcp(McpArgs {
                command: McpCommand::Serve(_)
            }))
        ));

        let preflight = Cli::try_parse_from([
            "volicord",
            "mcp",
            "preflight",
            "--connection",
            "connection_example",
            "--json",
        ])
        .expect("the public MCP preflight subcommand must parse");
        assert!(matches!(
            preflight.command,
            Some(Command::Mcp(McpArgs {
                command: McpCommand::Preflight(_)
            }))
        ));

        Cli::try_parse_from([
            "volicord",
            "mcp",
            "serve",
            "--discover-repository",
            "--host",
            "codex",
        ])
        .expect("repository discovery binding should parse");
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
                        ReportOutputArgs {
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
    fn connection_list_uses_the_shared_report_output_contract() {
        let parsed = Cli::try_parse_from(["volicord", "connection", "list", "--json"])
            .expect("list JSON should remain available");
        let Some(Command::Connection(ConnectionArgs {
            command:
                ConnectionCommand::List(ConnectionListArgs {
                    output: ReportOutputArgs { json: true, .. },
                    ..
                }),
        })) = parsed.command
        else {
            panic!("unexpected list command")
        };

        Cli::try_parse_from(["volicord", "connection", "list", "--verbose"])
            .expect("list verbose output should use the shared report flag");
        let error = Cli::try_parse_from(["volicord", "connection", "list", "--json", "--verbose"])
            .expect_err("list JSON and verbose output must be mutually exclusive");
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn policy_show_uses_the_shared_report_output_contract() {
        for (flag, expected) in [
            (None, ReportOutputArgs::default()),
            (
                Some("--verbose"),
                ReportOutputArgs {
                    json: false,
                    verbose: true,
                },
            ),
            (
                Some("--json"),
                ReportOutputArgs {
                    json: true,
                    verbose: false,
                },
            ),
        ] {
            let mut arguments = vec!["volicord", "policy", "show", "--repo", "."];
            if let Some(flag) = flag {
                arguments.push(flag);
            }
            let parsed = Cli::try_parse_from(arguments).expect("policy show output should parse");
            let Some(Command::Policy(PolicyArgs {
                command: PolicyCommand::Show(PolicyShowArgs { output, .. }),
            })) = parsed.command
            else {
                panic!("unexpected policy show command")
            };
            assert_eq!(output, expected);
        }

        let error = Cli::try_parse_from([
            "volicord",
            "policy",
            "show",
            "--repo",
            ".",
            "--verbose",
            "--json",
        ])
        .expect_err("policy show JSON and verbose output must conflict");
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn policy_validate_accepts_only_optional_json_output() {
        for json in [false, true] {
            let mut arguments = vec!["volicord", "policy", "validate", "--file", "policy.json"];
            if json {
                arguments.push("--json");
            }
            let parsed =
                Cli::try_parse_from(arguments).expect("policy validation output should parse");
            let Some(Command::Policy(PolicyArgs {
                command:
                    PolicyCommand::Validate(PolicyValidateArgs {
                        json: parsed_json, ..
                    }),
            })) = parsed.command
            else {
                panic!("unexpected policy validate command")
            };
            assert_eq!(parsed_json, json);
        }

        assert!(Cli::try_parse_from([
            "volicord",
            "policy",
            "validate",
            "--file",
            "policy.json",
            "--verbose",
        ])
        .is_err());
    }
}
