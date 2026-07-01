use std::{
    collections::BTreeMap,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_store::{
    agent_connections::CONNECTION_MODE_WORKFLOW,
    bootstrap::{
        initialize_runtime_home, write_installation_profile, InstallationProfileRecord,
        InstallationProfileRegistration, RuntimeHomeRecord,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};

use crate::managed_block::{self, ManagedBlockWrite};
use crate::registration::ADMIN_METADATA_JSON;
#[cfg(test)]
use crate::shell_path::mcp_binary_name;
pub(crate) use crate::shell_path::{is_executable_file, volicord_binary_name};
use crate::{
    setup_report::{
        CommandAvailability, SetupAction, SetupActionKind, SetupReport, SetupSectionStatus,
        SetupStatus,
    },
    shell_path::{
        detect_command_on_path, path_directory_is_on_path, paths_equivalent,
        setup_link_dir_candidates, verify_directory_writable, SetupLinkDirCandidate, PATH_ENV,
    },
};

const INSTALLATION_ID: &str = "default";
const SETUP_CREATED_BY: &str = "volicord_cli_setup";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    Complete,
    ActionRequired,
    Failed,
}

impl CommandStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::ActionRequired => "action_required",
            Self::Failed => "failed",
        }
    }

    pub const fn exits_failure(self) -> bool {
        matches!(self, Self::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    pub status: CommandStatus,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupCommandError {
    Usage(String),
    Runtime(String),
}

impl std::fmt::Display for SetupCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for SetupCommandError {}

impl From<StoreError> for SetupCommandError {
    fn from(error: StoreError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for SetupCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<io::Error> for SetupCommandError {
    fn from(error: io::Error) -> Self {
        Self::Runtime(error.to_string())
    }
}

pub trait SetupProcess {
    fn env_var(&self, name: &str) -> Option<std::ffi::OsString>;
    fn current_exe(&self) -> Result<PathBuf, String>;
}

pub struct ProductionSetupProcess;

impl SetupProcess for ProductionSetupProcess {
    fn env_var(&self, name: &str) -> Option<std::ffi::OsString> {
        std::env::var_os(name)
    }

    fn current_exe(&self) -> Result<PathBuf, String> {
        std::env::current_exe()
            .map_err(|error| format!("failed to read current executable: {error}"))
    }
}

pub trait SetupTerminal {
    fn write_str(&mut self, text: &str) -> io::Result<()>;
    fn read_line(&mut self, input: &mut String) -> io::Result<usize>;
}

pub struct StdioSetupTerminal {
    stdin: io::Stdin,
    stdout: io::Stdout,
}

impl StdioSetupTerminal {
    pub fn new() -> Self {
        Self {
            stdin: io::stdin(),
            stdout: io::stdout(),
        }
    }
}

impl Default for StdioSetupTerminal {
    fn default() -> Self {
        Self::new()
    }
}

impl SetupTerminal for StdioSetupTerminal {
    fn write_str(&mut self, text: &str) -> io::Result<()> {
        self.stdout.write_all(text.as_bytes())?;
        self.stdout.flush()
    }

    fn read_line(&mut self, input: &mut String) -> io::Result<usize> {
        self.stdin.read_line(input)
    }
}

pub struct ClosureSetupProcess<'a, F>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    env_var: &'a F,
}

impl<'a, F> ClosureSetupProcess<'a, F>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    pub fn new(env_var: &'a F) -> Self {
        Self { env_var }
    }
}

impl<F> SetupProcess for ClosureSetupProcess<'_, F>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    fn env_var(&self, name: &str) -> Option<std::ffi::OsString> {
        (self.env_var)(name)
    }

    fn current_exe(&self) -> Result<PathBuf, String> {
        std::env::current_exe()
            .map_err(|error| format!("failed to read current executable: {error}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedSetupOptions {
    runtime_home: Option<PathBuf>,
    link_bin: Option<PathBuf>,
    mcp_command: Option<PathBuf>,
    output: OutputFormat,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiscoveredCommand {
    path: PathBuf,
    source: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShellStartupPlan {
    shell_name: String,
    target_file: PathBuf,
    block: String,
    command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InteractiveSetupChoice {
    LinkOnly(PathBuf),
    LinkAndShell {
        link_bin: PathBuf,
        shell: ShellStartupPlan,
    },
    Manual {
        link_bin: Option<PathBuf>,
        command: String,
    },
    Skip,
    Cancelled(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InteractiveMenuChoice {
    number: usize,
    label: String,
    choice: InteractiveSetupChoice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InteractiveMenuPlan {
    choices: Vec<InteractiveMenuChoice>,
    shell_unavailable: Option<String>,
}

pub fn setup_usage() -> String {
    "volicord setup [--home PATH] [--link-bin PATH] [--mcp-command PATH] [--json]\n".to_owned()
}

pub fn run_setup_command(
    args: &[String],
    current_dir: &Path,
    process: &impl SetupProcess,
) -> Result<CommandOutcome, SetupCommandError> {
    run_setup_command_inner(args, current_dir, process, None)
}

pub fn run_setup_command_interactive(
    args: &[String],
    current_dir: &Path,
    process: &impl SetupProcess,
    terminal: &mut dyn SetupTerminal,
) -> Result<CommandOutcome, SetupCommandError> {
    run_setup_command_inner(args, current_dir, process, Some(terminal))
}

fn run_setup_command_inner(
    args: &[String],
    current_dir: &Path,
    process: &impl SetupProcess,
    mut terminal: Option<&mut dyn SetupTerminal>,
) -> Result<CommandOutcome, SetupCommandError> {
    if is_help_request(args) {
        return Ok(CommandOutcome {
            status: CommandStatus::Complete,
            output: setup_usage(),
        });
    }
    let mut parsed = parse_setup_options(args, current_dir)?;
    let runtime_home = resolve_setup_runtime_home(&parsed, current_dir, process)?;
    let runtime_home_id = runtime_home_id_for_path(&runtime_home)?;
    let runtime_home_record =
        initialize_runtime_home(&runtime_home, &runtime_home_id, ADMIN_METADATA_JSON)?;
    let runtime_home_section = runtime_home_report_section(&runtime_home_record);
    let mut checks =
        vec![
            DiagnosticCheck::passed("runtime_home", "Runtime Home registry is ready").with_details(
                json!({
                    "runtime_home": path_text(&runtime_home_record.runtime_home),
                    "registry_db": path_text(&runtime_home_record.registry_db_path),
                    "runtime_home_id": runtime_home_record.runtime_home_id,
                }),
            ),
        ];
    let mut actions_required = Vec::new();
    let mut actions_optional = Vec::new();
    let mut actions_performed = vec![SetupAction::performed(
        "runtime_home_ready",
        SetupActionKind::RuntimeHomeReady,
        "Runtime Home registry is ready.",
    )
    .with_path(&runtime_home_record.runtime_home)];
    let mut link_results = BTreeMap::new();
    let mut shell_startup_plan = None;
    let mut interactive_notes = Vec::new();
    let mut command_links_ready = false;

    let volicord_command = match discover_volicord_command(process) {
        Ok(command) => {
            checks.push(
                DiagnosticCheck::passed("volicord_command", "volicord command was discovered")
                    .with_details(json!({
                        "path": path_text(&command.path),
                        "source": command.source,
                    })),
            );
            command
        }
        Err(error) => {
            checks.push(
                DiagnosticCheck::failed("volicord_command", "volicord command was not discovered")
                    .with_details(json!({ "detail": error.to_string() })),
            );
            actions_required.push(SetupAction::required(
                "run_setup_from_volicord",
                SetupActionKind::CommandAvailability,
                "Run volicord setup from an accessible volicord executable.",
            ));
            let report = SetupReport::new(
                runtime_home_section,
                installation_profile_failed("installation profile was not saved", &error),
                vec![missing_command_availability(
                    "volicord_command",
                    &volicord_binary_name(),
                )],
                actions_required,
                actions_optional,
                actions_performed,
            );
            let status = command_status(report.status);
            return Ok(CommandOutcome {
                status,
                output: render_setup_output(
                    parsed.output,
                    &report,
                    &runtime_home_record,
                    None,
                    &checks,
                )?,
            });
        }
    };

    let volicord_mcp = match discover_mcp_command(&parsed, process, &volicord_command) {
        Ok(command) => {
            checks.push(
                DiagnosticCheck::passed(
                    "volicord_mcp_command",
                    "MCP launch command was discovered",
                )
                .with_details(json!({
                    "path": path_text(&command.path),
                    "source": command.source,
                })),
            );
            command
        }
        Err(error) => {
            checks.push(
                DiagnosticCheck::failed(
                    "volicord_mcp_command",
                    "MCP launch command was not discovered",
                )
                .with_details(json!({ "detail": error.to_string() })),
            );
            actions_required.push(
                SetupAction::required(
                    "select_mcp_command",
                    SetupActionKind::SelectMcpCommand,
                    "Run volicord setup --mcp-command PATH with an executable volicord path.",
                )
                .with_command("volicord setup --mcp-command PATH"),
            );
            let path_env = process.env_var(PATH_ENV);
            let commands = vec![command_availability(
                "volicord_command",
                &volicord_binary_name(),
                &volicord_command,
                path_env.as_deref(),
            )];
            push_command_availability_checks(&commands, &mut checks);
            plan_setup_actions(
                &commands,
                &parsed,
                process,
                None,
                &mut actions_required,
                &mut actions_optional,
            );
            let report = SetupReport::new(
                runtime_home_section,
                installation_profile_failed("installation profile was not saved", &error),
                commands,
                actions_required,
                actions_optional,
                actions_performed,
            );
            let status = command_status(report.status);
            return Ok(CommandOutcome {
                status,
                output: render_setup_output(
                    parsed.output,
                    &report,
                    &runtime_home_record,
                    None,
                    &checks,
                )?,
            });
        }
    };

    if parsed.output == OutputFormat::Text && parsed.link_bin.is_none() {
        let path_env = process.env_var(PATH_ENV);
        let commands = vec![command_availability(
            "volicord_command",
            &volicord_binary_name(),
            &volicord_command,
            path_env.as_deref(),
        )];
        if commands
            .iter()
            .any(|command| !command.selected_path_ready())
        {
            if let Some(terminal) = terminal.as_mut() {
                match prompt_command_availability_choice(
                    *terminal,
                    process,
                    &commands,
                    [&volicord_command.path, &volicord_mcp.path],
                )? {
                    InteractiveSetupChoice::LinkOnly(link_bin) => {
                        parsed.link_bin = Some(link_bin);
                    }
                    InteractiveSetupChoice::LinkAndShell { link_bin, shell } => {
                        parsed.link_bin = Some(link_bin);
                        shell_startup_plan = Some(shell);
                    }
                    InteractiveSetupChoice::Manual { link_bin, command } => {
                        if let Some(link_bin) = link_bin {
                            parsed.link_bin = Some(link_bin);
                        }
                        interactive_notes.push(format!("manual_path_command: {command}"));
                    }
                    InteractiveSetupChoice::Skip => {
                        interactive_notes.push("command linking was skipped".to_owned());
                    }
                    InteractiveSetupChoice::Cancelled(message) => {
                        interactive_notes.push(message);
                    }
                }
            }
        }
    }

    let bin_dir = parsed
        .link_bin
        .clone()
        .unwrap_or_else(|| command_parent(&volicord_command.path));
    let mut link_bin_on_path = None;

    if let Some(link_bin) = &parsed.link_bin {
        let link_bin = absolute_path(current_dir, link_bin.clone());
        let mut link_bin_usable = false;
        match prepare_link_bin(&link_bin) {
            Ok(()) => {
                link_bin_usable = true;
                let volicord_link = install_command_link(
                    &link_bin,
                    &volicord_binary_name(),
                    &volicord_command.path,
                );
                command_links_ready = link_ready_for_path(&volicord_link);
                push_link_check(
                    "link_volicord",
                    "volicord command link",
                    &link_bin,
                    &volicord_binary_name(),
                    &volicord_link,
                    LinkCheckOutputs {
                        checks: &mut checks,
                        actions_required: &mut actions_required,
                        actions_performed: &mut actions_performed,
                    },
                );
                link_results.insert("volicord".to_owned(), link_volicord_status(&volicord_link));
            }
            Err((summary, detail)) => {
                checks.push(
                    DiagnosticCheck::failed("link_bin", summary)
                        .with_details(json!({ "path": path_text(&link_bin), "detail": detail })),
                );
                actions_required.push(
                    SetupAction::required(
                        "repair_link_bin",
                        SetupActionKind::CommandLinks,
                        format!(
                            "Fix write access for {}, then rerun volicord setup --link-bin {}.",
                            link_bin.display(),
                            link_bin.display()
                        ),
                    )
                    .with_path(&link_bin),
                );
                link_results.insert("volicord".to_owned(), "failed".to_owned());
            }
        }
        let on_path = path_directory_is_on_path(process.env_var(PATH_ENV).as_deref(), &link_bin);
        link_bin_on_path = Some(on_path);
        if !on_path {
            let mut shell_startup_ready = false;
            if link_bin_usable && command_links_ready {
                if let Some(plan) = shell_startup_plan.as_ref() {
                    match managed_block::write_managed_block(&plan.target_file, &plan.block) {
                        Ok(ManagedBlockWrite::Created(path))
                        | Ok(ManagedBlockWrite::Updated(path)) => {
                            shell_startup_ready = true;
                            checks.push(
                                DiagnosticCheck::passed(
                                    "shell_startup_path",
                                    "shell startup PATH block was written",
                                )
                                .with_details(json!({
                                    "shell": plan.shell_name,
                                    "path": path_text(&path),
                                })),
                            );
                            actions_performed.push(
                                SetupAction::performed(
                                    "write_shell_startup_path",
                                    SetupActionKind::ShellStartup,
                                    format!(
                                        "Shell startup PATH block was written to {}.",
                                        path.display()
                                    ),
                                )
                                .with_path(&path),
                            );
                        }
                        Ok(ManagedBlockWrite::Unchanged(path)) => {
                            shell_startup_ready = true;
                            checks.push(
                                DiagnosticCheck::passed(
                                    "shell_startup_path",
                                    "shell startup PATH block already matches",
                                )
                                .with_details(json!({
                                    "shell": plan.shell_name,
                                    "path": path_text(&path),
                                })),
                            );
                            actions_performed.push(
                                SetupAction::performed(
                                    "reuse_shell_startup_path",
                                    SetupActionKind::ShellStartup,
                                    format!(
                                        "Shell startup PATH block already matches {}.",
                                        path.display()
                                    ),
                                )
                                .with_path(&path),
                            );
                        }
                        Err(error) => {
                            checks.push(
                                DiagnosticCheck::failed(
                                    "shell_startup_path",
                                    "shell startup PATH block could not be written",
                                )
                                .with_details(json!({
                                    "shell": plan.shell_name,
                                    "path": path_text(&plan.target_file),
                                    "detail": error.to_string(),
                                })),
                            );
                            actions_required.push(
                                SetupAction::required(
                                    "repair_shell_startup_path",
                                    SetupActionKind::ShellStartup,
                                    format!(
                                        "Add {} to PATH manually or fix write access for {}.",
                                        link_bin.display(),
                                        plan.target_file.display()
                                    ),
                                )
                                .with_command(plan.command.clone())
                                .with_path(&plan.target_file),
                            );
                        }
                    }
                }
            }
            if link_bin_usable && command_links_ready {
                if shell_startup_ready {
                    actions_required.push(
                        SetupAction::required(
                            "open_new_shell_for_path",
                            SetupActionKind::PathUpdate,
                            format!(
                                "Open a new shell or restart MCP hosts so PATH includes {}.",
                                link_bin.display()
                            ),
                        )
                        .with_path(&link_bin),
                    )
                } else {
                    actions_required.push(
                        SetupAction::required(
                            "add_link_bin_to_path",
                            SetupActionKind::PathUpdate,
                            format!(
                                "Add {} to PATH before starting new shells or MCP hosts.",
                                link_bin.display()
                            ),
                        )
                        .with_command(shell_path_command(process, &link_bin)?)
                        .with_path(&link_bin),
                    )
                }
            }
            checks.push(
                DiagnosticCheck::warning(
                    "link_bin_path",
                    "link directory is not currently on PATH",
                )
                .with_details(json!({ "link_bin": path_text(&link_bin) })),
            );
        } else {
            checks.push(
                DiagnosticCheck::passed("link_bin_path", "link directory is on PATH")
                    .with_details(json!({ "link_bin": path_text(&link_bin) })),
            );
        }
    }

    let path_env = process.env_var(PATH_ENV);
    let commands = vec![command_availability(
        "volicord_command",
        &volicord_binary_name(),
        &volicord_command,
        path_env.as_deref(),
    )];
    push_command_availability_checks(&commands, &mut checks);
    plan_setup_actions(
        &commands,
        &parsed,
        process,
        link_bin_on_path,
        &mut actions_required,
        &mut actions_optional,
    );

    let metadata_json = setup_metadata_json(
        volicord_command.source,
        volicord_mcp.source,
        parsed.link_bin.as_deref(),
        &link_results,
    )?;
    let profile = write_installation_profile(
        &runtime_home,
        InstallationProfileRegistration {
            installation_id: INSTALLATION_ID.to_owned(),
            volicord_command: path_text(&volicord_command.path),
            volicord_mcp_command: path_text(&volicord_mcp.path),
            bin_dir,
            default_connection_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            metadata_json,
        },
    )?;
    checks.push(
        DiagnosticCheck::passed("installation_profile", "installation profile was saved")
            .with_details(profile_json(&profile)),
    );
    actions_performed.push(
        SetupAction::performed(
            "installation_profile_saved",
            SetupActionKind::InstallationProfileSaved,
            "Installation profile was saved.",
        )
        .with_path(&runtime_home_record.registry_db_path),
    );

    let report = SetupReport::new(
        runtime_home_section,
        SetupSectionStatus::complete("installation profile was saved", profile_json(&profile)),
        commands,
        actions_required,
        actions_optional,
        actions_performed,
    );
    let status = command_status(report.status);
    let output = append_interactive_notes(
        render_setup_output(
            parsed.output,
            &report,
            &runtime_home_record,
            Some(&profile),
            &checks,
        )?,
        parsed.output,
        &interactive_notes,
    );
    Ok(CommandOutcome { status, output })
}

fn prepare_link_bin(link_bin: &Path) -> Result<(), (&'static str, String)> {
    fs::create_dir_all(link_bin)
        .map_err(|error| ("link directory could not be created", error.to_string()))?;
    verify_directory_writable(link_bin)
        .map_err(|error| ("link directory is not writable", error.to_string()))
}

fn runtime_home_report_section(record: &RuntimeHomeRecord) -> SetupSectionStatus {
    SetupSectionStatus::complete(
        "Runtime Home registry is ready",
        json!({
            "runtime_home": path_text(&record.runtime_home),
            "registry_db": path_text(&record.registry_db_path),
            "runtime_home_id": record.runtime_home_id,
        }),
    )
}

fn installation_profile_failed(
    summary: impl Into<String>,
    error: &SetupCommandError,
) -> SetupSectionStatus {
    SetupSectionStatus::failed(summary, json!({ "detail": error.to_string() }))
}

fn command_availability(
    id: impl Into<String>,
    command_name: &str,
    discovered: &DiscoveredCommand,
    path_env: Option<&std::ffi::OsStr>,
) -> CommandAvailability {
    let path_match = detect_command_on_path(command_name, path_env);
    let discovered_dir = command_parent(&discovered.path);
    let discovered_directory_on_path = path_directory_is_on_path(path_env, &discovered_dir);
    let path_matches_discovered = path_match
        .as_deref()
        .map(|path| paths_equivalent(path, &discovered.path))
        .unwrap_or(false);
    CommandAvailability {
        id: id.into(),
        command_name: command_name.to_owned(),
        discovered: true,
        discovered_path: Some(path_text(&discovered.path)),
        discovery_source: Some(discovered.source.to_owned()),
        available_on_path: path_match.is_some(),
        path_matches_discovered,
        discovered_directory_on_path,
        path_match: path_match.as_deref().map(path_text),
    }
}

fn missing_command_availability(id: impl Into<String>, command_name: &str) -> CommandAvailability {
    CommandAvailability {
        id: id.into(),
        command_name: command_name.to_owned(),
        discovered: false,
        discovered_path: None,
        discovery_source: None,
        available_on_path: false,
        path_matches_discovered: false,
        discovered_directory_on_path: false,
        path_match: None,
    }
}

fn push_command_availability_checks(
    commands: &[CommandAvailability],
    checks: &mut Vec<DiagnosticCheck>,
) {
    for command in commands {
        if !command.discovered {
            checks.push(DiagnosticCheck::failed(
                format!("{}_availability", command.id),
                format!("{} command was not discovered", command.command_name),
            ));
        } else if command.selected_path_ready() {
            checks.push(
                DiagnosticCheck::passed(
                    format!("{}_availability", command.id),
                    format!(
                        "{} resolves to the selected executable on PATH",
                        command.command_name
                    ),
                )
                .with_details(command_availability_details(command)),
            );
        } else if command.available_on_path {
            checks.push(
                DiagnosticCheck::warning(
                    format!("{}_availability", command.id),
                    format!(
                        "{} resolves to a different executable on PATH",
                        command.command_name
                    ),
                )
                .with_details(command_availability_details(command)),
            );
        } else {
            checks.push(
                DiagnosticCheck::warning(
                    format!("{}_availability", command.id),
                    format!("{} is not available on PATH", command.command_name),
                )
                .with_details(command_availability_details(command)),
            );
        }
    }
}

fn command_availability_details(command: &CommandAvailability) -> Value {
    json!({
        "command_name": &command.command_name,
        "discovered_path": &command.discovered_path,
        "discovery_source": &command.discovery_source,
        "available_on_path": command.available_on_path,
        "path_matches_discovered": command.path_matches_discovered,
        "discovered_directory_on_path": command.discovered_directory_on_path,
        "path_match": &command.path_match,
    })
}

fn plan_setup_actions(
    commands: &[CommandAvailability],
    parsed: &ParsedSetupOptions,
    process: &impl SetupProcess,
    link_bin_on_path: Option<bool>,
    actions_required: &mut Vec<SetupAction>,
    actions_optional: &mut Vec<SetupAction>,
) {
    let link_bin_requested_but_not_on_path = link_bin_on_path == Some(false);
    for command in commands {
        if command.selected_path_ready() || link_bin_requested_but_not_on_path {
            continue;
        }
        if command.available_on_path {
            push_unique_action(
                actions_required,
                SetupAction::required(
                    format!("resolve_{}_path_mismatch", command.id),
                    SetupActionKind::CommandAvailability,
                    format!(
                        "Update PATH so {} resolves to the selected executable before starting new shells or MCP hosts.",
                        command.command_name
                    ),
                ),
            );
        } else if command.discovered {
            let mut action = SetupAction::required(
                format!("make_{}_available", command.id),
                SetupActionKind::CommandAvailability,
                format!(
                    "Make {} available on PATH before starting new shells or MCP hosts.",
                    command.command_name
                ),
            );
            if let Some(discovered_path) = command.discovered_path.as_deref() {
                let discovered_path = Path::new(discovered_path);
                if discovered_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == command.command_name)
                {
                    let parent = command_parent(discovered_path);
                    action =
                        action.with_command(format!("export PATH=\"{}:$PATH\"", parent.display()));
                }
            }
            push_unique_action(actions_required, action);
        }
    }

    if parsed.link_bin.is_none()
        && commands
            .iter()
            .any(|command| !command.selected_path_ready())
    {
        let mut action = SetupAction::optional(
            "create_command_links",
            SetupActionKind::CommandLinks,
            "Create command links with --link-bin; setup will not modify shell startup files.",
        );
        if let Some(link_bin) = suggested_link_bin(process) {
            action = action
                .with_command(format!("volicord setup --link-bin {}", link_bin.display()))
                .with_path(&link_bin);
        }
        push_unique_action(actions_optional, action);
    }
}

fn prompt_command_availability_choice(
    terminal: &mut dyn SetupTerminal,
    process: &impl SetupProcess,
    commands: &[CommandAvailability],
    selected_paths: [&Path; 2],
) -> Result<InteractiveSetupChoice, SetupCommandError> {
    terminal.write_str(
        "Volicord setup can help make these commands available on PATH for future shells and MCP hosts.\n",
    )?;
    for command in commands {
        if !command.selected_path_ready() {
            terminal.write_str(&format!(
                "- {}: {}\n",
                command.command_name,
                command_availability_summary(command)
            ))?;
        }
    }

    let menu = plan_interactive_menu_choices(process, selected_paths)?;

    if let Some(reason) = &menu.shell_unavailable {
        terminal.write_str(&format!("Shell startup update is unavailable: {reason}\n"))?;
    }
    terminal.write_str("Choices:\n")?;
    for choice in &menu.choices {
        terminal.write_str(&format!("  {}. {}\n", choice.number, choice.label))?;
    }

    let skip_number = menu
        .choices
        .iter()
        .find(|choice| matches!(choice.choice, InteractiveSetupChoice::Skip))
        .map(|choice| choice.number)
        .unwrap_or(menu.choices.len());
    loop {
        terminal.write_str(&format!("Choice [{skip_number}]: "))?;
        let Some(input) = read_prompt_line(terminal)? else {
            return Ok(InteractiveSetupChoice::Cancelled(
                "setup prompt cancelled; no command links or shell startup files were changed"
                    .to_owned(),
            ));
        };
        let selected_number = if input.trim().is_empty() {
            skip_number
        } else if let Ok(number) = input.trim().parse::<usize>() {
            number
        } else {
            terminal.write_str("Enter the number of one setup choice.\n")?;
            continue;
        };
        let Some(choice) = menu
            .choices
            .iter()
            .find(|choice| choice.number == selected_number)
            .map(|choice| choice.choice.clone())
        else {
            terminal.write_str("Enter one of the listed setup choice numbers.\n")?;
            continue;
        };
        return confirm_interactive_choice(terminal, choice);
    }
}

fn plan_interactive_menu_choices(
    process: &impl SetupProcess,
    selected_paths: [&Path; 2],
) -> Result<InteractiveMenuPlan, SetupCommandError> {
    let path_env = process.env_var(PATH_ENV);
    let link_candidate = suggested_link_bin_candidate(process);
    let mut shell_unavailable = None;
    let mut choices = Vec::new();
    if let Some(link_candidate) = link_candidate {
        let link_bin = link_candidate.path().to_path_buf();
        let requires_creation = link_candidate.requires_creation();
        let link_bin_on_path = path_directory_is_on_path(path_env.as_deref(), &link_bin);
        if link_bin_on_path {
            push_menu_choice(
                &mut choices,
                link_only_label(&link_bin, requires_creation, "already on PATH"),
                InteractiveSetupChoice::LinkOnly(link_bin.clone()),
            );
        } else {
            match shell_startup_plan(process, &link_bin) {
                Ok(plan) => push_menu_choice(
                    &mut choices,
                    link_and_shell_label(&link_bin, requires_creation, &plan.target_file),
                    InteractiveSetupChoice::LinkAndShell {
                        link_bin: link_bin.clone(),
                        shell: plan,
                    },
                ),
                Err(reason) => shell_unavailable = Some(reason),
            }
            push_menu_choice(
                &mut choices,
                link_only_label(&link_bin, requires_creation, "PATH still needs an update"),
                InteractiveSetupChoice::LinkOnly(link_bin.clone()),
            );
        }

        push_menu_choice(
            &mut choices,
            manual_link_label(&link_bin, requires_creation),
            InteractiveSetupChoice::Manual {
                link_bin: Some(link_bin.clone()),
                command: shell_path_command(process, &link_bin)?,
            },
        );
    } else {
        let command =
            shell_path_command_for_selected_dirs(process, &selected_command_dirs(selected_paths))?;
        push_menu_choice(
            &mut choices,
            "Print the PATH command without modifying files.".to_owned(),
            InteractiveSetupChoice::Manual {
                link_bin: None,
                command,
            },
        );
    }
    push_menu_choice(
        &mut choices,
        "Skip command linking for now.".to_owned(),
        InteractiveSetupChoice::Skip,
    );

    Ok(InteractiveMenuPlan {
        choices,
        shell_unavailable,
    })
}

fn link_and_shell_label(link_bin: &Path, requires_creation: bool, target_file: &Path) -> String {
    if requires_creation {
        format!(
            "Create {}, create links, and add a managed PATH block to {}.",
            link_bin.display(),
            target_file.display()
        )
    } else {
        format!(
            "Create links and add a managed PATH block to {}.",
            target_file.display()
        )
    }
}

fn link_only_label(link_bin: &Path, requires_creation: bool, path_status: &str) -> String {
    if requires_creation {
        format!(
            "Create {} and command links; {path_status}.",
            link_bin.display()
        )
    } else {
        format!(
            "Create command links in {}; {path_status}.",
            link_bin.display()
        )
    }
}

fn manual_link_label(link_bin: &Path, requires_creation: bool) -> String {
    if requires_creation {
        format!(
            "Create {}, create links, and print the PATH command.",
            link_bin.display()
        )
    } else {
        format!(
            "Create command links in {} and print the PATH command.",
            link_bin.display()
        )
    }
}

fn confirm_interactive_choice(
    terminal: &mut dyn SetupTerminal,
    choice: InteractiveSetupChoice,
) -> Result<InteractiveSetupChoice, SetupCommandError> {
    match choice {
        InteractiveSetupChoice::LinkAndShell { link_bin, shell } => {
            terminal.write_str(&format!(
                "Shell startup file:\n  {}\n\nManaged block to write:\n{}",
                shell.target_file.display(),
                shell.block
            ))?;
            terminal.write_str("Write this managed block? [y/N]: ")?;
            let Some(answer) = read_prompt_line(terminal)? else {
                return Ok(InteractiveSetupChoice::Cancelled(
                    "setup prompt cancelled; no command links or shell startup files were changed"
                        .to_owned(),
                ));
            };
            if is_yes(&answer) {
                Ok(InteractiveSetupChoice::LinkAndShell { link_bin, shell })
            } else {
                Ok(InteractiveSetupChoice::Cancelled(
                    "shell startup update was not approved; no command links or shell startup files were changed"
                        .to_owned(),
                ))
            }
        }
        InteractiveSetupChoice::Manual { link_bin, command } => {
            terminal.write_str(&format!("Run this command after setup:\n  {command}\n"))?;
            Ok(InteractiveSetupChoice::Manual { link_bin, command })
        }
        other => Ok(other),
    }
}

fn push_menu_choice(
    choices: &mut Vec<InteractiveMenuChoice>,
    label: String,
    choice: InteractiveSetupChoice,
) {
    choices.push(InteractiveMenuChoice {
        number: choices.len() + 1,
        label,
        choice,
    });
}

fn read_prompt_line(terminal: &mut dyn SetupTerminal) -> Result<Option<String>, SetupCommandError> {
    let mut input = String::new();
    match terminal.read_line(&mut input) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(input.trim().to_owned())),
        Err(error) if error.kind() == io::ErrorKind::Interrupted => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn is_yes(input: &str) -> bool {
    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn suggested_link_bin(process: &impl SetupProcess) -> Option<PathBuf> {
    suggested_link_bin_candidate(process).map(|candidate| candidate.path().to_path_buf())
}

fn suggested_link_bin_candidate(process: &impl SetupProcess) -> Option<SetupLinkDirCandidate> {
    setup_link_dir_candidates(&|name| process.env_var(name))
        .into_iter()
        .find(SetupLinkDirCandidate::is_usable)
}

fn shell_startup_plan(
    process: &impl SetupProcess,
    link_bin: &Path,
) -> Result<ShellStartupPlan, String> {
    #[cfg(not(unix))]
    {
        let _ = (process, link_bin);
        Err("shell startup file updates are not supported on this platform".to_owned())
    }
    #[cfg(unix)]
    {
        let shell_path = process
            .env_var("SHELL")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "SHELL is not set".to_owned())?;
        let shell_name = PathBuf::from(shell_path.clone())
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "SHELL does not name a supported shell".to_owned())?
            .to_owned();
        let home = process
            .env_var("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set".to_owned())?;
        let target_file = match shell_name.as_str() {
            "bash" => home.join(".bashrc"),
            "zsh" => home.join(".zshrc"),
            "sh" => home.join(".profile"),
            other => {
                return Err(format!(
                    "{other} is not supported for automatic shell startup updates"
                ))
            }
        };
        let path_expr =
            shell_path_expression(process, link_bin).map_err(|error| error.to_string())?;
        let command = shell_path_command(process, link_bin).map_err(|error| error.to_string())?;
        Ok(ShellStartupPlan {
            shell_name,
            target_file,
            block: managed_block::path_export_block(&path_expr),
            command,
        })
    }
}

fn shell_path_command(
    process: &impl SetupProcess,
    dir: &Path,
) -> Result<String, SetupCommandError> {
    shell_path_command_for_selected_dirs(process, &[dir.to_path_buf()])
}

fn shell_path_command_for_selected_dirs(
    process: &impl SetupProcess,
    dirs: &[PathBuf],
) -> Result<String, SetupCommandError> {
    if dirs.is_empty() {
        return Err(SetupCommandError::Runtime(
            "no PATH directory is available for a shell command".to_owned(),
        ));
    }
    #[cfg(windows)]
    {
        let rendered = dirs
            .iter()
            .map(|dir| dir.display().to_string())
            .collect::<Vec<_>>()
            .join(";");
        let _ = process;
        Ok(format!("set \"PATH={rendered};%PATH%\""))
    }
    #[cfg(not(windows))]
    {
        let rendered = dirs
            .iter()
            .map(|dir| shell_path_expression(process, dir))
            .collect::<Result<Vec<_>, _>>()?
            .join(":");
        Ok(format!("export PATH=\"{rendered}:$PATH\""))
    }
}

fn shell_path_expression(
    process: &impl SetupProcess,
    dir: &Path,
) -> Result<String, SetupCommandError> {
    if let Some(home) = process
        .env_var("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        if let Ok(relative) = dir.strip_prefix(&home) {
            if relative.as_os_str().is_empty() {
                return Ok("$HOME".to_owned());
            }
            let relative = relative.to_str().ok_or_else(|| {
                SetupCommandError::Runtime(
                    "PATH directory must be valid UTF-8 for shell command output".to_owned(),
                )
            })?;
            return Ok(format!("$HOME/{}", escape_double_quoted_shell(relative)));
        }
    }
    let dir = dir.to_str().ok_or_else(|| {
        SetupCommandError::Runtime(
            "PATH directory must be valid UTF-8 for shell command output".to_owned(),
        )
    })?;
    Ok(escape_double_quoted_shell(dir))
}

fn escape_double_quoted_shell(text: &str) -> String {
    let mut escaped = String::new();
    for ch in text.chars() {
        if matches!(ch, '\\' | '"' | '$' | '`') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

fn selected_command_dirs(paths: [&Path; 2]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for path in paths {
        let dir = command_parent(path);
        if !dirs.iter().any(|existing| paths_equivalent(existing, &dir)) {
            dirs.push(dir);
        }
    }
    dirs
}

fn push_unique_action(actions: &mut Vec<SetupAction>, action: SetupAction) {
    if !actions.iter().any(|existing| existing.id == action.id) {
        actions.push(action);
    }
}

fn command_status(status: SetupStatus) -> CommandStatus {
    match status {
        SetupStatus::Complete => CommandStatus::Complete,
        SetupStatus::ActionRequired => CommandStatus::ActionRequired,
        SetupStatus::Failed => CommandStatus::Failed,
    }
}

fn parse_setup_options(
    args: &[String],
    current_dir: &Path,
) -> Result<ParsedSetupOptions, SetupCommandError> {
    let mut parsed = ParsedSetupOptions {
        runtime_home: None,
        link_bin: None,
        mcp_command: None,
        output: OutputFormat::Text,
    };
    let mut seen = BTreeMap::<String, ()>::new();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(SetupCommandError::Usage(setup_usage()));
        }
        if !token.starts_with("--") {
            return Err(SetupCommandError::Usage(format!(
                "unexpected argument: {token}"
            )));
        }
        let without_prefix = &token[2..];
        let (name, value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name.to_owned(), Some(value.to_owned()))
        } else if without_prefix == "json" {
            (without_prefix.to_owned(), None)
        } else {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(SetupCommandError::Usage(format!(
                    "missing value for --{without_prefix}"
                )));
            };
            (without_prefix.to_owned(), Some(value.clone()))
        };
        if seen.insert(name.clone(), ()).is_some() {
            return Err(SetupCommandError::Usage(format!(
                "duplicate option: --{name}"
            )));
        }
        match name.as_str() {
            "home" => parsed.runtime_home = Some(value_path(&name, value.as_deref(), current_dir)?),
            "link-bin" => parsed.link_bin = Some(value_path(&name, value.as_deref(), current_dir)?),
            "mcp-command" => {
                parsed.mcp_command = Some(value_path(&name, value.as_deref(), current_dir)?)
            }
            "json" => {
                if value.is_some() {
                    return Err(SetupCommandError::Usage(
                        "--json does not accept a value".to_owned(),
                    ));
                }
                parsed.output = OutputFormat::Json;
            }
            _ => {
                return Err(SetupCommandError::Usage(format!(
                    "unknown option: --{name}"
                )))
            }
        }
        index += 1;
    }
    Ok(parsed)
}

fn value_path(
    name: &str,
    value: Option<&str>,
    current_dir: &Path,
) -> Result<PathBuf, SetupCommandError> {
    let value =
        value.ok_or_else(|| SetupCommandError::Usage(format!("missing value for --{name}")))?;
    if value.trim().is_empty() {
        return Err(SetupCommandError::Usage(format!(
            "--{name} must not be empty"
        )));
    }
    Ok(absolute_path(current_dir, PathBuf::from(value)))
}

fn resolve_setup_runtime_home(
    parsed: &ParsedSetupOptions,
    current_dir: &Path,
    process: &impl SetupProcess,
) -> Result<PathBuf, SetupCommandError> {
    if let Some(path) = &parsed.runtime_home {
        Ok(path.clone())
    } else {
        resolve_runtime_home(|name| process.env_var(name), current_dir).map_err(Into::into)
    }
}

fn discover_volicord_command(
    process: &impl SetupProcess,
) -> Result<DiscoveredCommand, SetupCommandError> {
    let current_exe = process.current_exe().map_err(SetupCommandError::Runtime)?;
    let path = canonical_existing_file(&current_exe, "volicord command")?;
    Ok(DiscoveredCommand {
        path,
        source: "current_exe",
    })
}

fn discover_mcp_command(
    parsed: &ParsedSetupOptions,
    process: &impl SetupProcess,
    volicord_command: &DiscoveredCommand,
) -> Result<DiscoveredCommand, SetupCommandError> {
    if let Some(command) = &parsed.mcp_command {
        let path = canonical_existing_executable(command, "MCP launch command")?;
        return Ok(DiscoveredCommand {
            path,
            source: "explicit",
        });
    }

    let _ = process;
    Ok(DiscoveredCommand {
        path: volicord_command.path.clone(),
        source: "volicord",
    })
}

fn canonical_existing_file(path: &Path, label: &'static str) -> Result<PathBuf, SetupCommandError> {
    let metadata = fs::metadata(path).map_err(|error| {
        SetupCommandError::Runtime(format!("{label} is not accessible: {error}"))
    })?;
    if !metadata.is_file() {
        return Err(SetupCommandError::Runtime(format!(
            "{label} must be a file: {}",
            path.display()
        )));
    }
    Ok(fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()))
}

fn canonical_existing_executable(
    path: &Path,
    label: &'static str,
) -> Result<PathBuf, SetupCommandError> {
    let path = canonical_existing_file(path, label)?;
    if is_executable_file(&path) {
        Ok(path)
    } else {
        Err(SetupCommandError::Runtime(format!(
            "{label} must be executable: {}",
            path.display()
        )))
    }
}

fn is_help_request(args: &[String]) -> bool {
    matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LinkInstallResult {
    Created(PathBuf),
    Existing(PathBuf),
    UnsafeExisting(PathBuf),
    #[cfg_attr(unix, allow(dead_code))]
    Unsupported(PathBuf),
    Failed {
        path: PathBuf,
        detail: String,
    },
}

fn install_command_link(link_bin: &Path, name: &str, target: &Path) -> LinkInstallResult {
    let link_path = link_bin.join(name);
    install_command_link_inner(&link_path, target)
}

#[cfg(unix)]
fn install_command_link_inner(link_path: &Path, target: &Path) -> LinkInstallResult {
    use std::os::unix::fs::symlink;

    match fs::symlink_metadata(link_path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                match fs::read_link(link_path) {
                    Ok(existing_target) if existing_target == target => {
                        LinkInstallResult::Existing(link_path.to_path_buf())
                    }
                    Ok(existing_target) => {
                        match (fs::canonicalize(existing_target), fs::canonicalize(target)) {
                            (Ok(existing), Ok(expected)) if existing == expected => {
                                LinkInstallResult::Existing(link_path.to_path_buf())
                            }
                            _ => LinkInstallResult::UnsafeExisting(link_path.to_path_buf()),
                        }
                    }
                    Err(error) => LinkInstallResult::Failed {
                        path: link_path.to_path_buf(),
                        detail: error.to_string(),
                    },
                }
            } else {
                LinkInstallResult::UnsafeExisting(link_path.to_path_buf())
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => match symlink(target, link_path) {
            Ok(()) => LinkInstallResult::Created(link_path.to_path_buf()),
            Err(error) => LinkInstallResult::Failed {
                path: link_path.to_path_buf(),
                detail: error.to_string(),
            },
        },
        Err(error) => LinkInstallResult::Failed {
            path: link_path.to_path_buf(),
            detail: error.to_string(),
        },
    }
}

#[cfg(not(unix))]
fn install_command_link_inner(link_path: &Path, _target: &Path) -> LinkInstallResult {
    LinkInstallResult::Unsupported(link_path.to_path_buf())
}

struct LinkCheckOutputs<'a> {
    checks: &'a mut Vec<DiagnosticCheck>,
    actions_required: &'a mut Vec<SetupAction>,
    actions_performed: &'a mut Vec<SetupAction>,
}

fn push_link_check(
    check_id: &str,
    label: &str,
    link_bin: &Path,
    name: &str,
    result: &LinkInstallResult,
    outputs: LinkCheckOutputs<'_>,
) {
    match result {
        LinkInstallResult::Created(path) => {
            outputs.checks.push(
                DiagnosticCheck::passed(check_id, format!("{label} was created"))
                    .with_details(json!({ "path": path_text(path) })),
            );
            outputs.actions_performed.push(
                SetupAction::performed(
                    format!("create_{name}_link"),
                    SetupActionKind::CommandLinks,
                    format!("{label} was created."),
                )
                .with_path(path),
            );
        }
        LinkInstallResult::Existing(path) => {
            outputs.checks.push(
                DiagnosticCheck::passed(
                    check_id,
                    format!("{label} already points to the selected executable"),
                )
                .with_details(json!({ "path": path_text(path) })),
            );
            outputs.actions_performed.push(
                SetupAction::performed(
                    format!("reuse_{name}_link"),
                    SetupActionKind::CommandLinks,
                    format!("{label} already points to the selected executable."),
                )
                .with_path(path),
            );
        }
        LinkInstallResult::Unsupported(path) => {
            outputs.checks.push(
                DiagnosticCheck::warning(
                    check_id,
                    format!("{label} was not created on this platform"),
                )
                .with_details(json!({ "path": path_text(path) })),
            );
            outputs.actions_required.push(
                SetupAction::required(
                    format!("create_{name}_shim"),
                    SetupActionKind::CommandLinks,
                    format!(
                        "Create a command shim for {name} under {} if your shell cannot find it.",
                        link_bin.display()
                    ),
                )
                .with_path(path),
            );
        }
        LinkInstallResult::UnsafeExisting(path) => {
            outputs.checks.push(
                DiagnosticCheck::failed(
                    check_id,
                    format!(
                        "{label} was not replaced because an existing path is not Volicord-managed"
                    ),
                )
                .with_details(json!({ "path": path_text(path) })),
            );
            outputs.actions_required.push(
                SetupAction::required(
                    format!("repair_{name}_link"),
                    SetupActionKind::CommandLinks,
                    format!(
                        "Move or remove the existing {} path, then rerun volicord setup --link-bin {}.",
                        path.display(),
                        link_bin.display()
                    ),
                )
                .with_path(path),
            );
        }
        LinkInstallResult::Failed { path, detail } => {
            outputs.checks.push(
                DiagnosticCheck::failed(check_id, format!("{label} could not be created"))
                    .with_details(json!({ "path": path_text(path), "detail": detail })),
            );
            outputs.actions_required.push(
                SetupAction::required(
                    format!("repair_{name}_link"),
                    SetupActionKind::CommandLinks,
                    format!(
                        "Fix write access for {}, then rerun volicord setup --link-bin {}.",
                        path.display(),
                        link_bin.display()
                    ),
                )
                .with_path(path),
            );
        }
    }
}

fn link_volicord_status(result: &LinkInstallResult) -> String {
    match result {
        LinkInstallResult::Created(_) => "created",
        LinkInstallResult::Existing(_) => "existing",
        LinkInstallResult::UnsafeExisting(_) => "unsafe_existing",
        LinkInstallResult::Unsupported(_) => "unsupported",
        LinkInstallResult::Failed { .. } => "failed",
    }
    .to_owned()
}

fn link_ready_for_path(result: &LinkInstallResult) -> bool {
    matches!(
        result,
        LinkInstallResult::Created(_) | LinkInstallResult::Existing(_)
    )
}

fn render_setup_output(
    output: OutputFormat,
    report: &SetupReport,
    runtime_home: &RuntimeHomeRecord,
    profile: Option<&InstallationProfileRecord>,
    checks: &[DiagnosticCheck],
) -> Result<String, SetupCommandError> {
    match output {
        OutputFormat::Json => serde_json::to_string_pretty(&json!({
            "status": report.status.as_str(),
            "status_meaning": setup_status_meaning(report.status),
            "runtime_home": path_text(&runtime_home.runtime_home),
            "registry_db": path_text(&runtime_home.registry_db_path),
            "installation_profile": profile.map(profile_json),
            "states": setup_states_json(report),
            "setup_report": report,
            "commands": &report.commands,
            "checks": checks,
            "actions": &report.actions_required,
            "actions_required": &report.actions_required,
            "actions_optional": &report.actions_optional,
            "actions_performed": &report.actions_performed,
            "primary_next_action": primary_setup_action(report),
        }))
        .map(|text| format!("{text}\n"))
        .map_err(|error| SetupCommandError::Runtime(error.to_string())),
        OutputFormat::Text => {
            let mut text = format!(
                "Volicord setup {}\nstatus_meaning: {}\nruntime_home_state: {}\nruntime_home: {}\nregistry_db: {}\ninstallation_profile_state: {}\ncommand_state: {}\nhost_reload_required: {}\n",
                report.status.as_str(),
                setup_status_meaning(report.status),
                report.runtime_home.status.as_str(),
                runtime_home.runtime_home.display(),
                runtime_home.registry_db_path.display(),
                report.installation_profile.status.as_str(),
                setup_command_state(report),
                yes_no(setup_host_reload_required(report)),
            );
            if let Some(profile) = profile {
                text.push_str(&format!(
                    "volicord_command: {}\nvolicord_mcp_command: {}\ndefault_connection_mode: {}\n",
                    profile.volicord_command,
                    profile.volicord_mcp_command,
                    profile.default_connection_mode
                ));
            }
            let not_passed = checks
                .iter()
                .filter(|check| check.status != "passed")
                .collect::<Vec<_>>();
            if let Some(check) = not_passed.first() {
                text.push_str(&format!(
                    "blocking_check: {} ({})\n",
                    check.summary, check.status
                ));
            }
            append_setup_next_action(&mut text, report);
            text.push_str(&format!(
                "optional_action_count: {}\n",
                report.actions_optional.len()
            ));
            Ok(text)
        }
    }
}

fn setup_states_json(report: &SetupReport) -> Value {
    json!({
        "runtime_home": report.runtime_home.status.as_str(),
        "installation_profile": report.installation_profile.status.as_str(),
        "command_availability": setup_command_state(report),
        "host_reload_required": setup_host_reload_required(report),
    })
}

fn setup_command_state(report: &SetupReport) -> &'static str {
    if report.commands.iter().any(|command| !command.discovered) {
        "not_found"
    } else if report
        .commands
        .iter()
        .any(|command| !command.selected_path_ready())
    {
        "action_required"
    } else {
        "ready"
    }
}

fn setup_host_reload_required(report: &SetupReport) -> bool {
    report.actions_required.iter().any(|action| {
        matches!(
            action.kind,
            SetupActionKind::CommandAvailability
                | SetupActionKind::CommandLinks
                | SetupActionKind::PathUpdate
                | SetupActionKind::ShellStartup
        )
    })
}

fn primary_setup_action(report: &SetupReport) -> Option<&SetupAction> {
    report.actions_required.first()
}

fn append_setup_next_action(output: &mut String, report: &SetupReport) {
    match primary_setup_action(report) {
        Some(action) => output.push_str(&format!("next_action: {}\n", action.instruction)),
        None => output.push_str("next_action: none\n"),
    }
}

fn setup_status_meaning(status: SetupStatus) -> &'static str {
    match status {
        SetupStatus::Complete => "installation profile setup is complete",
        SetupStatus::ActionRequired => "installation profile setup needs a named user action",
        SetupStatus::Failed => "installation profile setup could not complete",
    }
}

fn append_interactive_notes(mut output: String, format: OutputFormat, notes: &[String]) -> String {
    if format == OutputFormat::Text && !notes.is_empty() {
        output.push_str("interactive_setup:\n");
        for note in notes {
            output.push_str(&format!("- {note}\n"));
        }
    }
    output
}

fn command_availability_summary(command: &CommandAvailability) -> String {
    if !command.discovered {
        "not discovered".to_owned()
    } else if command.selected_path_ready() {
        match &command.discovered_path {
            Some(path) => format!("ready on PATH ({path})"),
            None => "ready on PATH".to_owned(),
        }
    } else if let Some(path_match) = &command.path_match {
        format!("PATH resolves {path_match}, not the selected executable")
    } else {
        match &command.discovered_path {
            Some(path) => format!("selected executable is {path}; not on PATH"),
            None => "not on PATH".to_owned(),
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

pub(crate) fn profile_json(profile: &InstallationProfileRecord) -> Value {
    json!({
        "installation_id": profile.installation_id,
        "runtime_home_id": profile.runtime_home_id,
        "volicord_command": profile.volicord_command,
        "volicord_mcp_command": profile.volicord_mcp_command,
        "bin_dir": path_text(&profile.bin_dir),
        "default_connection_mode": profile.default_connection_mode,
        "created_at": profile.created_at,
        "updated_at": profile.updated_at,
    })
}

fn setup_metadata_json(
    volicord_source: &str,
    mcp_source: &str,
    link_bin: Option<&Path>,
    link_results: &BTreeMap<String, String>,
) -> Result<String, SetupCommandError> {
    serde_json::to_string(&json!({
        "created_by": SETUP_CREATED_BY,
        "volicord_command_source": volicord_source,
        "volicord_mcp_command_source": mcp_source,
        "link_bin": link_bin.map(path_text),
        "link_bin_requested": link_bin.is_some(),
        "link_results": link_results,
    }))
    .map_err(|error| SetupCommandError::Runtime(error.to_string()))
}

pub(crate) fn runtime_home_id_for_path(path: &Path) -> Result<String, SetupCommandError> {
    let path_text = path.to_str().ok_or_else(|| {
        SetupCommandError::Runtime("Runtime Home path must be valid UTF-8".to_owned())
    })?;
    let digest = Sha256::digest(path_text.as_bytes());
    Ok(format!(
        "runtime_home_{:016x}",
        u64::from_be_bytes([
            digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
        ])
    ))
}

fn command_parent(path: &Path) -> PathBuf {
    path.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(crate) fn path_text(path: &Path) -> String {
    path.display().to_string()
}

pub(crate) fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        env,
        ffi::OsString,
        io::{self, Write},
    };

    use rusqlite::Connection;
    use volicord_store::{bootstrap::installation_profile, sqlite::registry_db_path};
    use volicord_test_support::TempRuntimeHome;

    use super::*;

    #[derive(Debug)]
    struct FakeProcess {
        exe: PathBuf,
        env: BTreeMap<String, OsString>,
    }

    impl SetupProcess for FakeProcess {
        fn env_var(&self, name: &str) -> Option<OsString> {
            self.env.get(name).cloned()
        }

        fn current_exe(&self) -> Result<PathBuf, String> {
            Ok(self.exe.clone())
        }
    }

    #[derive(Debug)]
    enum FakeTerminalInput {
        Line(String),
        MenuChoiceContaining(String),
    }

    impl FakeTerminalInput {
        fn line(line: impl Into<String>) -> Self {
            Self::Line(line.into())
        }

        fn menu_choice_containing(label: impl Into<String>) -> Self {
            Self::MenuChoiceContaining(label.into())
        }
    }

    #[derive(Debug)]
    struct FakeTerminal {
        input: VecDeque<FakeTerminalInput>,
        output: String,
    }

    impl FakeTerminal {
        fn new(lines: &[&str]) -> Self {
            Self {
                input: lines
                    .iter()
                    .map(|line| FakeTerminalInput::line(*line))
                    .collect(),
                output: String::new(),
            }
        }

        fn with_inputs(inputs: Vec<FakeTerminalInput>) -> Self {
            Self {
                input: inputs.into(),
                output: String::new(),
            }
        }

        fn output(&self) -> &str {
            &self.output
        }
    }

    impl SetupTerminal for FakeTerminal {
        fn write_str(&mut self, text: &str) -> io::Result<()> {
            self.output.push_str(text);
            Ok(())
        }

        fn read_line(&mut self, input: &mut String) -> io::Result<usize> {
            let Some(next_input) = self.input.pop_front() else {
                return Ok(0);
            };
            let line = match next_input {
                FakeTerminalInput::Line(line) => line,
                FakeTerminalInput::MenuChoiceContaining(label) => {
                    menu_choice_number_containing(&self.output, &label)
                        .unwrap_or_else(|| panic!("menu choice containing {label:?} not found"))
                        .to_string()
                }
            };
            let line = format!("{line}\n");
            input.push_str(&line);
            Ok(line.len())
        }
    }

    fn menu_choice_number_containing(output: &str, label_fragment: &str) -> Option<usize> {
        output.lines().find_map(|line| {
            let trimmed = line.trim_start();
            let (number, label) = trimmed.split_once(". ")?;
            label
                .contains(label_fragment)
                .then(|| number.parse::<usize>().ok())
                .flatten()
        })
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum InteractiveChoiceKind {
        LinkOnly,
        LinkAndShell,
        Manual,
        Skip,
        Cancelled,
    }

    fn interactive_choice_kinds(choices: &[InteractiveMenuChoice]) -> Vec<InteractiveChoiceKind> {
        choices
            .iter()
            .map(|choice| match &choice.choice {
                InteractiveSetupChoice::LinkOnly(_) => InteractiveChoiceKind::LinkOnly,
                InteractiveSetupChoice::LinkAndShell { .. } => InteractiveChoiceKind::LinkAndShell,
                InteractiveSetupChoice::Manual { .. } => InteractiveChoiceKind::Manual,
                InteractiveSetupChoice::Skip => InteractiveChoiceKind::Skip,
                InteractiveSetupChoice::Cancelled(_) => InteractiveChoiceKind::Cancelled,
            })
            .collect()
    }

    #[test]
    fn setup_action_planner_reports_stable_action_kinds() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = TempRuntimeHome::new("setup-action-planner-kinds")?;
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        fs::create_dir_all(&home)?;
        let command_path = fixture.path().join("exe").join(volicord_binary_name());
        let process = FakeProcess {
            exe: command_path.clone(),
            env: BTreeMap::from([("HOME".to_owned(), home.clone().into_os_string())]),
        };
        let parsed = ParsedSetupOptions {
            runtime_home: None,
            link_bin: None,
            mcp_command: None,
            output: OutputFormat::Text,
        };
        let commands = vec![CommandAvailability {
            id: "volicord_command".to_owned(),
            command_name: volicord_binary_name(),
            discovered: true,
            discovered_path: Some(path_text(&command_path)),
            discovery_source: Some("test".to_owned()),
            available_on_path: false,
            path_matches_discovered: false,
            discovered_directory_on_path: false,
            path_match: None,
        }];
        let mut actions_required = Vec::new();
        let mut actions_optional = Vec::new();

        plan_setup_actions(
            &commands,
            &parsed,
            &process,
            None,
            &mut actions_required,
            &mut actions_optional,
        );

        assert_eq!(
            actions_required
                .iter()
                .map(|action| action.kind)
                .collect::<Vec<_>>(),
            vec![SetupActionKind::CommandAvailability]
        );
        assert_eq!(
            actions_optional
                .iter()
                .map(|action| action.kind)
                .collect::<Vec<_>>(),
            vec![SetupActionKind::CommandLinks]
        );
        assert_eq!(actions_optional[0].path, Some(path_text(&local_bin)));
        assert!(!local_bin.exists());
        Ok(())
    }

    #[test]
    fn setup_action_planner_uses_home_bin_when_local_bin_is_unavailable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-action-planner-home-bin")?;
        let home = fixture.path().join("home");
        let home_bin = home.join("bin");
        fs::create_dir_all(&home)?;
        fs::write(home.join(".local"), "not a directory")?;
        let command_path = fixture.path().join("exe").join(volicord_binary_name());
        let process = FakeProcess {
            exe: command_path.clone(),
            env: BTreeMap::from([("HOME".to_owned(), home.clone().into_os_string())]),
        };
        let parsed = ParsedSetupOptions {
            runtime_home: None,
            link_bin: None,
            mcp_command: None,
            output: OutputFormat::Text,
        };
        let commands = vec![CommandAvailability {
            id: "volicord_command".to_owned(),
            command_name: volicord_binary_name(),
            discovered: true,
            discovered_path: Some(path_text(&command_path)),
            discovery_source: Some("test".to_owned()),
            available_on_path: false,
            path_matches_discovered: false,
            discovered_directory_on_path: false,
            path_match: None,
        }];
        let mut actions_required = Vec::new();
        let mut actions_optional = Vec::new();

        plan_setup_actions(
            &commands,
            &parsed,
            &process,
            None,
            &mut actions_required,
            &mut actions_optional,
        );

        assert_eq!(
            actions_optional
                .iter()
                .map(|action| action.kind)
                .collect::<Vec<_>>(),
            vec![SetupActionKind::CommandLinks]
        );
        assert_eq!(actions_optional[0].path, Some(path_text(&home_bin)));
        assert!(actions_optional[0]
            .command
            .as_deref()
            .is_some_and(|command| command.contains("--link-bin")));
        assert!(!home_bin.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn interactive_menu_plan_prefers_existing_path_link() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = TempRuntimeHome::new("setup-menu-path-dir")?;
        let path_dir = fixture.path().join("path-bin");
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        fs::create_dir_all(&path_dir)?;
        fs::create_dir_all(&local_bin)?;
        let process = FakeProcess {
            exe: fixture.path().join("volicord"),
            env: BTreeMap::from([
                (PATH_ENV.to_owned(), env::join_paths([path_dir.as_path()])?),
                ("HOME".to_owned(), home.into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let selected = [
            fixture.path().join(volicord_binary_name()),
            fixture.path().join(mcp_binary_name()),
        ];

        let menu = plan_interactive_menu_choices(
            &process,
            [selected[0].as_path(), selected[1].as_path()],
        )?;

        assert_eq!(
            interactive_choice_kinds(&menu.choices),
            vec![
                InteractiveChoiceKind::LinkOnly,
                InteractiveChoiceKind::Manual,
                InteractiveChoiceKind::Skip,
            ]
        );
        assert!(menu.choices[0].label.contains("already on PATH"));
        assert!(menu.shell_unavailable.is_none());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn interactive_menu_plan_orders_shell_update_before_user_bin_only(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-menu-shell-order")?;
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        fs::create_dir_all(&home)?;
        let process = FakeProcess {
            exe: fixture.path().join("volicord"),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let selected = [
            fixture.path().join(volicord_binary_name()),
            fixture.path().join(mcp_binary_name()),
        ];

        let menu = plan_interactive_menu_choices(
            &process,
            [selected[0].as_path(), selected[1].as_path()],
        )?;

        assert_eq!(
            interactive_choice_kinds(&menu.choices),
            vec![
                InteractiveChoiceKind::LinkAndShell,
                InteractiveChoiceKind::LinkOnly,
                InteractiveChoiceKind::Manual,
                InteractiveChoiceKind::Skip,
            ]
        );
        assert!(menu.choices[0].label.contains("managed PATH block"));
        assert!(menu.choices[0].label.contains("Create "));
        assert!(menu.choices[1].label.contains("PATH still needs an update"));
        assert!(menu.shell_unavailable.is_none());
        assert!(!local_bin.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn interactive_menu_plan_uses_home_bin_when_local_bin_is_unavailable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-menu-home-bin")?;
        let home = fixture.path().join("home");
        let home_bin = home.join("bin");
        fs::create_dir_all(&home)?;
        fs::write(home.join(".local"), "not a directory")?;
        let process = FakeProcess {
            exe: fixture.path().join("volicord"),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let selected = [
            fixture.path().join(volicord_binary_name()),
            fixture.path().join(mcp_binary_name()),
        ];

        let menu = plan_interactive_menu_choices(
            &process,
            [selected[0].as_path(), selected[1].as_path()],
        )?;

        assert_eq!(
            interactive_choice_kinds(&menu.choices),
            vec![
                InteractiveChoiceKind::LinkAndShell,
                InteractiveChoiceKind::LinkOnly,
                InteractiveChoiceKind::Manual,
                InteractiveChoiceKind::Skip,
            ]
        );
        match &menu.choices[0].choice {
            InteractiveSetupChoice::LinkAndShell { link_bin, .. } => {
                assert_eq!(link_bin, &home_bin);
            }
            other => panic!("expected link-and-shell choice, got {other:?}"),
        }
        assert!(menu.choices[0].label.contains(&path_text(&home_bin)));
        assert!(menu.choices[1].label.contains("PATH still needs an update"));
        assert!(menu.shell_unavailable.is_none());
        assert!(!home_bin.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn interactive_menu_plan_keeps_manual_and_skip_when_shell_is_unsupported(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-menu-unsupported-shell")?;
        let home = fixture.path().join("home");
        fs::create_dir_all(home.join(".local").join("bin"))?;
        let process = FakeProcess {
            exe: fixture.path().join("volicord"),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/fish")),
            ]),
        };
        let selected = [
            fixture.path().join(volicord_binary_name()),
            fixture.path().join(mcp_binary_name()),
        ];

        let menu = plan_interactive_menu_choices(
            &process,
            [selected[0].as_path(), selected[1].as_path()],
        )?;

        assert_eq!(
            interactive_choice_kinds(&menu.choices),
            vec![
                InteractiveChoiceKind::LinkOnly,
                InteractiveChoiceKind::Manual,
                InteractiveChoiceKind::Skip,
            ]
        );
        assert!(menu
            .shell_unavailable
            .as_deref()
            .is_some_and(|reason| reason.contains("fish is not supported")));
        assert!(menu.choices[1].label.contains("print the PATH command"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_creates_links_in_writable_path_dir(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-path-dir")?;
        let exe_dir = fixture.path().join("exe");
        let path_dir = fixture.path().join("path-bin");
        let home = fixture.path().join("home");
        fs::create_dir_all(&path_dir)?;
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        let mcp = write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([
                (PATH_ENV.to_owned(), env::join_paths([path_dir.as_path()])?),
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal =
            FakeTerminal::with_inputs(vec![FakeTerminalInput::menu_choice_containing(
                "already on PATH",
            )]);

        let outcome = run_setup_command_interactive(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::Complete);
        assert!(terminal.output().contains("Choices:"));
        assert_eq!(
            fs::canonicalize(path_dir.join(volicord_binary_name()))?,
            volicord
        );
        assert_eq!(fs::canonicalize(path_dir.join(mcp_binary_name()))?, mcp);
        assert!(!home.join(".zshrc").exists());
        Ok(())
    }

    #[test]
    fn setup_interactive_json_never_prompts() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-json")?;
        let bin_dir = fixture.path().join("bin");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::new(),
        };
        let mut terminal = FakeTerminal::new(&[]);

        let outcome = run_setup_command_interactive(
            &[
                "--home".to_owned(),
                path_text(fixture.path()),
                "--json".to_owned(),
            ],
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert_eq!(terminal.output(), "");
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "action_required");
        Ok(())
    }

    #[test]
    fn setup_json_reports_missing_user_bin_action_without_creating_it(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-json-missing-user-bin")?;
        let bin_dir = fixture.path().join("bin");
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([("HOME".to_owned(), home.clone().into_os_string())]),
        };

        let outcome = run_setup_command(
            &[
                "--home".to_owned(),
                path_text(fixture.path()),
                "--json".to_owned(),
            ],
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "action_required");
        assert!(value["actions_optional"]
            .as_array()
            .expect("actions_optional should be an array")
            .iter()
            .any(|action| action["id"] == "create_command_links"
                && action["path"] == path_text(&local_bin)
                && action["command"]
                    .as_str()
                    .is_some_and(|command| command.contains("--link-bin"))));
        assert!(!local_bin.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_link_bin_never_prompts() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-link-bin")?;
        let bin_dir = fixture.path().join("bin");
        let link_bin = fixture.path().join("links");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([(PATH_ENV.to_owned(), env::join_paths([link_bin.as_path()])?)]),
        };
        let mut terminal = FakeTerminal::new(&[]);

        let outcome = run_setup_command_interactive(
            &[
                "--home".to_owned(),
                path_text(fixture.path()),
                "--mcp-command".to_owned(),
                path_text(&mcp),
                "--link-bin".to_owned(),
                path_text(&link_bin),
            ],
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::Complete);
        assert_eq!(terminal.output(), "");
        assert!(link_bin.join(volicord_binary_name()).exists());
        assert!(link_bin.join(mcp_binary_name()).exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_writes_shell_startup_block_idempotently(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-shell")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        let link_bin = home.join(".local").join("bin");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        let mcp = write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };

        let mut first_terminal = FakeTerminal::with_inputs(vec![
            FakeTerminalInput::menu_choice_containing("managed PATH block"),
            FakeTerminalInput::line("y"),
        ]);
        let first = run_setup_command_interactive(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
            &mut first_terminal,
        )?;
        assert_eq!(first.status, CommandStatus::ActionRequired);
        assert!(first_terminal.output().contains("Managed block to write"));

        assert_eq!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            process.exe
        );
        assert_eq!(fs::canonicalize(link_bin.join(mcp_binary_name()))?, mcp);
        let zshrc = home.join(".zshrc");
        let first_text = fs::read_to_string(&zshrc)?;
        assert!(first_text.contains("# >>> volicord setup >>>"));
        assert!(first_text.contains("export PATH=\"$HOME/.local/bin:$PATH\""));

        let mut second_terminal = FakeTerminal::with_inputs(vec![
            FakeTerminalInput::menu_choice_containing("managed PATH block"),
            FakeTerminalInput::line("y"),
        ]);
        run_setup_command_interactive(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
            &mut second_terminal,
        )?;
        let second_text = fs::read_to_string(&zshrc)?;
        assert_eq!(second_text.matches("# >>> volicord setup >>>").count(), 1);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_does_not_add_shell_block_for_unmanaged_link(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-unsafe-link")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        let link_bin = home.join(".local/bin");
        fs::create_dir_all(&link_bin)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&link_bin, &volicord_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal = FakeTerminal::with_inputs(vec![
            FakeTerminalInput::menu_choice_containing("managed PATH block"),
            FakeTerminalInput::line("y"),
        ]);

        let outcome = run_setup_command_interactive(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(terminal.output().contains("Managed block to write"));
        assert!(outcome.output.contains("Move or remove the existing"));
        assert!(!outcome.output.contains("Open a new shell"));
        assert_ne!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            process.exe
        );
        assert!(!home.join(".zshrc").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_unsupported_shell_uses_manual_action(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-unsupported-shell")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/fish")),
            ]),
        };
        let mut terminal =
            FakeTerminal::with_inputs(vec![FakeTerminalInput::menu_choice_containing(
                "print the PATH command",
            )]);

        let outcome = run_setup_command_interactive(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(terminal
            .output()
            .contains("Shell startup update is unavailable"));
        assert!(terminal.output().contains("Run this command after setup"));
        assert!(!home.join(".config/fish/config.fish").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_skip_reports_action_required_without_links(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-skip")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal =
            FakeTerminal::with_inputs(vec![FakeTerminalInput::menu_choice_containing(
                "Skip command linking",
            )]);

        let outcome = run_setup_command_interactive(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(terminal.output().contains("Skip command linking for now."));
        assert!(outcome.output.contains("command linking was skipped"));
        assert!(!home.join(".local").exists());
        assert!(!home.join(".zshrc").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_link_only_creates_links_without_shell_startup_when_path_needs_update(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-link-only")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        let mcp = write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal =
            FakeTerminal::with_inputs(vec![FakeTerminalInput::menu_choice_containing(
                "PATH still needs an update",
            )]);

        let outcome = run_setup_command_interactive(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(!terminal.output().contains("Managed block to write"));
        let link_bin = home.join(".local/bin");
        assert!(link_bin.is_dir());
        assert_eq!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            volicord
        );
        assert_eq!(fs::canonicalize(link_bin.join(mcp_binary_name()))?, mcp);
        assert!(!home.join(".zshrc").exists());
        assert!(outcome.output.contains("Add "));
        assert!(outcome.output.contains(".local/bin"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_link_only_creates_home_bin_when_local_bin_is_unavailable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-home-bin")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        let local = home.join(".local");
        let link_bin = home.join("bin");
        fs::create_dir_all(&home)?;
        fs::write(&local, "not a directory")?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        let mcp = write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal =
            FakeTerminal::with_inputs(vec![FakeTerminalInput::menu_choice_containing(
                "PATH still needs an update",
            )]);

        let outcome = run_setup_command_interactive(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(terminal.output().contains(&path_text(&link_bin)));
        assert!(!terminal.output().contains("Managed block to write"));
        assert!(link_bin.is_dir());
        assert_eq!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            volicord
        );
        assert_eq!(fs::canonicalize(link_bin.join(mcp_binary_name()))?, mcp);
        assert!(fs::metadata(&local)?.is_file());
        assert!(!home.join(".zshrc").exists());
        assert!(outcome.output.contains("Add "));
        assert!(outcome.output.contains(&path_text(&link_bin)));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_declined_shell_startup_update_leaves_files_unchanged(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-decline-shell")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        let zshrc = home.join(".zshrc");
        let original_zshrc = "export PATH=\"$HOME/bin:$PATH\"\n";
        fs::write(&zshrc, original_zshrc)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal = FakeTerminal::with_inputs(vec![
            FakeTerminalInput::menu_choice_containing("managed PATH block"),
            FakeTerminalInput::line("n"),
        ]);

        let outcome = run_setup_command_interactive(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(terminal.output().contains("Managed block to write"));
        assert!(outcome
            .output
            .contains("shell startup update was not approved"));
        assert_eq!(fs::read_to_string(&zshrc)?, original_zshrc);
        assert!(!home.join(".local").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_interactive_eof_cancels_command_linking() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-interactive-eof")?;
        let exe_dir = fixture.path().join("exe");
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&exe_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::from([
                ("HOME".to_owned(), home.clone().into_os_string()),
                ("SHELL".to_owned(), OsString::from("/bin/zsh")),
            ]),
        };
        let mut terminal = FakeTerminal::new(&[]);

        let outcome = run_setup_command_interactive(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
            &mut terminal,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        assert!(outcome.output.contains("setup prompt cancelled"));
        assert!(!home.join(".local").exists());
        assert!(!home.join(".zshrc").exists());
        Ok(())
    }

    #[test]
    fn setup_records_explicit_mcp_command() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-explicit")?;
        let bin_dir = fixture.path().join("bin");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, "custom-volicord")?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::new(),
        };

        let outcome = run_setup_command(
            &[
                "--home".to_owned(),
                path_text(fixture.path()),
                "--mcp-command".to_owned(),
                path_text(&mcp),
                "--json".to_owned(),
            ],
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "action_required");
        assert_eq!(
            value["setup_report"]["installation_profile"]["status"],
            "complete"
        );
        assert!(value["checks"]
            .as_array()
            .expect("checks should be an array")
            .iter()
            .any(|check| {
                check["id"] == "volicord_mcp_command"
                    && check["status"] == "passed"
                    && check["details"]["path"] == path_text(&mcp)
                    && check["details"]["source"] == "explicit"
            }));
        assert!(value["actions_required"]
            .as_array()
            .expect("actions_required should be an array")
            .iter()
            .any(|action| action["id"] == "make_volicord_command_available"));
        let profile = installation_profile(fixture.path())?.expect("profile should be stored");
        assert_eq!(profile.volicord_mcp_command, path_text(&mcp));
        assert_eq!(profile.default_connection_mode, CONNECTION_MODE_WORKFLOW);
        assert!(registry_db_path(fixture.path()).exists());
        Ok(())
    }

    #[test]
    fn setup_uses_volicord_as_default_mcp_launch_command() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = TempRuntimeHome::new("setup-default-mcp")?;
        let bin_dir = fixture.path().join("bin");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::new(),
        };

        run_setup_command(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
        )?;

        let profile = installation_profile(fixture.path())?.expect("profile should be stored");
        assert_eq!(profile.volicord_mcp_command, path_text(&volicord));
        Ok(())
    }

    #[test]
    fn setup_keeps_default_mcp_launch_bound_to_current_volicord(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-path")?;
        let exe_dir = fixture.path().join("exe");
        let path_dir = fixture.path().join("path-bin");
        let volicord = write_executable(&exe_dir, &volicord_binary_name())?;
        write_executable(&path_dir, &volicord_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([(PATH_ENV.to_owned(), env::join_paths([path_dir.as_path()])?)]),
        };

        run_setup_command(
            &["--home".to_owned(), path_text(fixture.path())],
            fixture.path(),
            &process,
        )?;

        let profile = installation_profile(fixture.path())?.expect("profile should be stored");
        assert_eq!(profile.volicord_mcp_command, path_text(&volicord));
        Ok(())
    }

    #[test]
    fn setup_json_does_not_require_separate_mcp_executable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-single-executable")?;
        let bin_dir = fixture.path().join("bin");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::new(),
        };

        let outcome = run_setup_command(
            &[
                "--home".to_owned(),
                path_text(fixture.path()),
                "--json".to_owned(),
            ],
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "action_required");
        assert_eq!(
            value["setup_report"]["installation_profile"]["status"],
            "complete"
        );
        let profile = installation_profile(fixture.path())?.expect("profile should be stored");
        assert_eq!(profile.volicord_mcp_command, path_text(&volicord));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_creates_requested_links() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-links")?;
        let bin_dir = fixture.path().join("bin");
        let link_bin = fixture.path().join("links");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::from([(PATH_ENV.to_owned(), env::join_paths([link_bin.as_path()])?)]),
        };

        let outcome = run_setup_command(
            &[
                "--home".to_owned(),
                path_text(fixture.path()),
                "--mcp-command".to_owned(),
                path_text(&mcp),
                "--link-bin".to_owned(),
                path_text(&link_bin),
                "--json".to_owned(),
            ],
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::Complete);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "complete");
        assert!(value["actions_performed"]
            .as_array()
            .expect("actions_performed should be an array")
            .iter()
            .any(|action| action["id"] == "create_volicord_link"));
        assert_eq!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            volicord
        );
        assert_eq!(fs::canonicalize(link_bin.join(mcp_binary_name()))?, mcp);
        let profile = installation_profile(fixture.path())?.expect("profile should be stored");
        let metadata: Value = serde_json::from_str(&profile.metadata_json)?;
        assert_eq!(metadata["link_bin"], path_text(&link_bin));
        assert_eq!(metadata["link_bin_requested"], true);
        assert_eq!(metadata["link_results"]["volicord"], "created");
        assert!(metadata["link_results"]["volicord_mcp"].is_null());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_link_bin_reports_path_action_without_prompting(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-links-path-action")?;
        let bin_dir = fixture.path().join("bin");
        let link_bin = fixture.path().join("links");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord.clone(),
            env: BTreeMap::new(),
        };

        let outcome = run_setup_command(
            &[
                "--home".to_owned(),
                path_text(fixture.path()),
                "--mcp-command".to_owned(),
                path_text(&mcp),
                "--link-bin".to_owned(),
                path_text(&link_bin),
                "--json".to_owned(),
            ],
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(value["status"], "action_required");
        assert!(value["actions_required"]
            .as_array()
            .expect("actions_required should be an array")
            .iter()
            .any(|action| action["id"] == "add_link_bin_to_path"));
        assert_eq!(
            fs::canonicalize(link_bin.join(volicord_binary_name()))?,
            volicord
        );
        assert_eq!(fs::canonicalize(link_bin.join(mcp_binary_name()))?, mcp);
        Ok(())
    }

    #[test]
    fn setup_link_bin_error_still_saves_profile_when_possible(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("setup-link-bin-file")?;
        let bin_dir = fixture.path().join("bin");
        let link_bin = fixture.path().join("not-a-directory");
        fs::write(&link_bin, "not a directory")?;
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::new(),
        };

        let outcome = run_setup_command(
            &[
                "--home".to_owned(),
                path_text(fixture.path()),
                "--mcp-command".to_owned(),
                path_text(&mcp),
                "--link-bin".to_owned(),
                path_text(&link_bin),
                "--json".to_owned(),
            ],
            fixture.path(),
            &process,
        )?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert_eq!(
            value["setup_report"]["installation_profile"]["status"],
            "complete"
        );
        assert!(value["checks"]
            .as_array()
            .expect("checks should be an array")
            .iter()
            .any(|check| check["id"] == "link_bin"
                && check["summary"] == "link directory could not be created"
                && check["details"]["detail"]
                    .as_str()
                    .is_some_and(|detail| !detail.is_empty())));
        assert!(value["actions_required"]
            .as_array()
            .expect("actions_required should be an array")
            .iter()
            .any(|action| action["id"] == "repair_link_bin"));
        assert!(!value["actions_required"]
            .as_array()
            .expect("actions_required should be an array")
            .iter()
            .any(|action| action["id"] == "add_link_bin_to_path"));
        assert!(!link_bin.join(volicord_binary_name()).exists());
        assert!(!link_bin.join(mcp_binary_name()).exists());
        assert!(installation_profile(fixture.path())?.is_some());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn setup_link_bin_probe_failure_reports_repair_action() -> Result<(), Box<dyn std::error::Error>>
    {
        use std::os::unix::fs::PermissionsExt;

        let fixture = TempRuntimeHome::new("setup-link-bin-probe-fails")?;
        let bin_dir = fixture.path().join("bin");
        let link_bin = fixture.path().join("links");
        fs::create_dir_all(&link_bin)?;
        let mut permissions = fs::metadata(&link_bin)?.permissions();
        permissions.set_mode(0o555);
        fs::set_permissions(&link_bin, permissions)?;
        if crate::shell_path::path_directory_is_verified_writable(&link_bin) {
            restore_writable_dir(&link_bin)?;
            return Ok(());
        }

        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::new(),
        };

        let outcome = run_setup_command(
            &[
                "--home".to_owned(),
                path_text(fixture.path()),
                "--mcp-command".to_owned(),
                path_text(&mcp),
                "--link-bin".to_owned(),
                path_text(&link_bin),
                "--json".to_owned(),
            ],
            fixture.path(),
            &process,
        );
        restore_writable_dir(&link_bin)?;
        let outcome = outcome?;

        assert_eq!(outcome.status, CommandStatus::ActionRequired);
        let value: Value = serde_json::from_str(&outcome.output)?;
        assert!(value["checks"]
            .as_array()
            .expect("checks should be an array")
            .iter()
            .any(|check| check["id"] == "link_bin"
                && check["summary"] == "link directory is not writable"));
        assert!(value["actions_required"]
            .as_array()
            .expect("actions_required should be an array")
            .iter()
            .any(|action| action["id"] == "repair_link_bin"));
        assert!(!link_bin.join(volicord_binary_name()).exists());
        assert!(!link_bin.join(mcp_binary_name()).exists());
        assert_eq!(fs::read_dir(&link_bin)?.count(), 0);
        Ok(())
    }

    fn write_executable(dir: &Path, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        fs::create_dir_all(dir)?;
        let path = dir.join(name);
        let mut file = fs::File::create(&path)?;
        writeln!(file, "#!/bin/sh")?;
        make_executable(&path)?;
        Ok(path)
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
        Ok(())
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    #[cfg(unix)]
    fn restore_writable_dir(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
        Ok(())
    }

    #[test]
    fn runtime_home_id_is_stable_for_same_path() {
        let path = Path::new("/tmp/volicord-id-test");

        assert_eq!(
            runtime_home_id_for_path(path).unwrap(),
            runtime_home_id_for_path(path).unwrap()
        );
    }

    #[test]
    fn installation_profile_table_can_be_read_after_setup() -> Result<(), Box<dyn std::error::Error>>
    {
        let fixture = TempRuntimeHome::new("setup-sql")?;
        let bin_dir = fixture.path().join("bin");
        let volicord = write_executable(&bin_dir, &volicord_binary_name())?;
        let mcp = write_executable(&bin_dir, &mcp_binary_name())?;
        let process = FakeProcess {
            exe: volicord,
            env: BTreeMap::new(),
        };

        run_setup_command(
            &[
                "--home".to_owned(),
                path_text(fixture.path()),
                "--mcp-command".to_owned(),
                path_text(&mcp),
            ],
            fixture.path(),
            &process,
        )?;

        let conn = Connection::open(registry_db_path(fixture.path()))?;
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM installation_profile", [], |row| {
                row.get(0)
            })?;
        assert_eq!(count, 1);
        Ok(())
    }
}
