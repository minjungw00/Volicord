use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde_json::json;
use volicord_store::{
    agent_connections::CONNECTION_MODE_WORKFLOW,
    bootstrap::{
        initialize_runtime_home_with_installation, inspect_runtime_home_bootstrap,
        InstallationProfileRecord, InstallationProfileRegistration, RuntimeHomeBootstrapState,
        RuntimeHomeRecord,
    },
    runtime_home::resolve_runtime_home,
    sqlite::registry_db_path,
    StoreError,
};

use crate::{
    registration::ADMIN_METADATA_JSON,
    setup_report::{SetupAction, SetupActionKind, SetupReport, SetupSectionStatus, SetupStatus},
    shell_path::{path_directory_is_on_path, PATH_ENV},
};

use super::{
    absolute_path, command_parent,
    discovery::{
        command_availability, discover_mcp_command, discover_volicord_command,
        missing_command_availability, plan_setup_actions, push_command_availability_checks,
        DiscoveredCommand,
    },
    interactive::{prompt_command_availability_choice, InteractiveSetupChoice},
    linking::{
        install_command_link, link_ready_for_path, link_volicord_status, prepare_link_bin,
        push_link_check, LinkCheckOutputs,
    },
    output::{profile_json, DiagnosticCheck},
    path_text, runtime_home_id_for_path,
    shell_startup::{shell_path_command, write_shell_startup_block, ShellStartupPlan},
    volicord_binary_name, CommandStatus, SetupCommandError, SetupProcess, SetupTerminal,
    SetupWorkflowOptions,
};

const INSTALLATION_ID: &str = "default";
const SETUP_CREATED_BY: &str = "volicord_cli_setup";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedSetupOptions {
    pub(super) runtime_home: Option<PathBuf>,
    pub(super) link_bin: Option<PathBuf>,
    pub(super) mcp_command: Option<PathBuf>,
    pub(super) output: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutputFormat {
    Text,
    Json,
}

pub(super) struct SetupWorkflowOutcome {
    pub(super) output: OutputFormat,
    pub(super) runtime_home: RuntimeHomeRecord,
    pub(super) installation_profile: Option<InstallationProfileRecord>,
    pub(super) report: SetupReport,
    pub(super) checks: Vec<DiagnosticCheck>,
    pub(super) interactive_notes: Vec<String>,
}

struct SetupWorkflowState {
    checks: Vec<DiagnosticCheck>,
    actions_required: Vec<SetupAction>,
    actions_optional: Vec<SetupAction>,
    actions_performed: Vec<SetupAction>,
    link_results: BTreeMap<String, String>,
    shell_startup_plan: Option<ShellStartupPlan>,
    interactive_notes: Vec<String>,
    command_links_ready: bool,
}

impl SetupWorkflowState {
    fn new(runtime_home: &Path, registry_db: &Path, runtime_home_id: &str) -> Self {
        Self {
            checks: vec![
                DiagnosticCheck::passed("runtime_home", "Runtime Home registry is ready")
                    .with_details(json!({
                        "runtime_home": path_text(runtime_home),
                        "registry_db": path_text(registry_db),
                        "runtime_home_id": runtime_home_id,
                    })),
            ],
            actions_required: Vec::new(),
            actions_optional: Vec::new(),
            actions_performed: vec![SetupAction::performed(
                "runtime_home_ready",
                SetupActionKind::RuntimeHomeReady,
                "Runtime Home registry is ready.",
            )
            .with_path(runtime_home)],
            link_results: BTreeMap::new(),
            shell_startup_plan: None,
            interactive_notes: Vec::new(),
            command_links_ready: false,
        }
    }
}

pub(super) fn command_status(status: SetupStatus) -> CommandStatus {
    match status {
        SetupStatus::Complete => CommandStatus::Complete,
        SetupStatus::ActionRequired => CommandStatus::ActionRequired,
        SetupStatus::Failed => CommandStatus::Failed,
    }
}

pub(super) fn run_setup_workflow(
    mut parsed: ParsedSetupOptions,
    current_dir: &Path,
    process: &impl SetupProcess,
    terminal: Option<&mut dyn SetupTerminal>,
) -> Result<SetupWorkflowOutcome, SetupCommandError> {
    let output = parsed.output;
    let runtime_home = resolve_setup_runtime_home(&parsed, current_dir, process)?;
    let runtime_home_id = runtime_home_id_for_path(&runtime_home)?;
    let existing_runtime_home = match inspect_runtime_home_bootstrap(&runtime_home)? {
        RuntimeHomeBootstrapState::Absent => None,
        RuntimeHomeBootstrapState::Ready(record) => Some(record),
        RuntimeHomeBootstrapState::Incompatible(mismatch) => {
            return Err(StoreError::RuntimeHomeSchemaMismatch(Box::new(mismatch)).into());
        }
        RuntimeHomeBootstrapState::Corrupt(corruption) => {
            return Err(StoreError::RuntimeHomeCorruption(corruption).into());
        }
    };
    let registry_db = registry_db_path(&runtime_home);
    let mut state = SetupWorkflowState::new(&runtime_home, &registry_db, &runtime_home_id);

    let volicord_command = match discover_volicord_command(process) {
        Ok(command) => {
            state.checks.push(
                DiagnosticCheck::passed("volicord_command", "volicord command was discovered")
                    .with_details(json!({
                        "path": path_text(&command.path),
                        "source": command.source,
                    })),
            );
            command
        }
        Err(error) => {
            let Some(runtime_home_record) = existing_runtime_home.clone() else {
                return Err(error);
            };
            let runtime_home_section = runtime_home_report_section(&runtime_home_record);
            state.checks.push(
                DiagnosticCheck::failed("volicord_command", "volicord command was not discovered")
                    .with_details(json!({ "detail": error.to_string() })),
            );
            state.actions_required.push(SetupAction::required(
                "run_init_from_volicord",
                SetupActionKind::CommandAvailability,
                "Install or link an accessible volicord executable, then initialize the primary host connection from the Product Repository.",
            )
            .with_command("volicord init --host <host> --repo <path>"));
            let report = SetupReport::new(
                runtime_home_section,
                installation_profile_failed("installation profile was not saved", &error),
                vec![missing_command_availability(
                    "volicord_command",
                    &volicord_binary_name(),
                )],
                state.actions_required,
                state.actions_optional,
                state.actions_performed,
            );
            return Ok(SetupWorkflowOutcome {
                output,
                runtime_home: runtime_home_record,
                installation_profile: None,
                report,
                checks: state.checks,
                interactive_notes: state.interactive_notes,
            });
        }
    };

    let volicord_mcp = match discover_mcp_command(&parsed, process, &volicord_command) {
        Ok(command) => {
            state.checks.push(
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
            let Some(runtime_home_record) = existing_runtime_home.clone() else {
                return Err(error);
            };
            let runtime_home_section = runtime_home_report_section(&runtime_home_record);
            state.checks.push(
                DiagnosticCheck::failed(
                    "volicord_mcp_command",
                    "MCP launch command was not discovered",
                )
                .with_details(json!({ "detail": error.to_string() })),
            );
            state.actions_required.push(
                SetupAction::required(
                    "select_mcp_command",
                    SetupActionKind::SelectMcpCommand,
                    "Select an executable MCP launch command, then rerun init with that command.",
                )
                .with_command("volicord init --host <host> --repo <path> --mcp-command PATH"),
            );
            let commands = current_command_availability(process, &volicord_command);
            push_command_availability_checks(&commands, &mut state.checks);
            plan_setup_actions(
                &commands,
                &parsed,
                process,
                None,
                &mut state.actions_required,
                &mut state.actions_optional,
            );
            let report = SetupReport::new(
                runtime_home_section,
                installation_profile_failed("installation profile was not saved", &error),
                commands,
                state.actions_required,
                state.actions_optional,
                state.actions_performed,
            );
            return Ok(SetupWorkflowOutcome {
                output,
                runtime_home: runtime_home_record,
                installation_profile: None,
                report,
                checks: state.checks,
                interactive_notes: state.interactive_notes,
            });
        }
    };

    apply_interactive_choice(
        &mut parsed,
        process,
        terminal,
        &volicord_command,
        &volicord_mcp,
        &mut state,
    )?;

    install_requested_links(&parsed, current_dir, process, &volicord_command, &mut state)?;

    let commands = current_command_availability(process, &volicord_command);
    push_command_availability_checks(&commands, &mut state.checks);
    plan_setup_actions(
        &commands,
        &parsed,
        process,
        requested_link_bin_on_path(&parsed, process, current_dir),
        &mut state.actions_required,
        &mut state.actions_optional,
    );

    let metadata_json = setup_metadata_json(
        volicord_command.source,
        volicord_mcp.source,
        parsed.link_bin.as_deref(),
        &state.link_results,
    )?;
    let (runtime_home_record, profile) = initialize_runtime_home_with_installation(
        &runtime_home,
        &runtime_home_id,
        ADMIN_METADATA_JSON,
        InstallationProfileRegistration {
            installation_id: INSTALLATION_ID.to_owned(),
            volicord_command: path_text(&volicord_command.path),
            volicord_mcp_command: path_text(&volicord_mcp.path),
            bin_dir: selected_bin_dir(&parsed, &volicord_command),
            default_connection_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            metadata_json,
        },
    )?;
    let runtime_home_section = runtime_home_report_section(&runtime_home_record);
    state.checks.push(
        DiagnosticCheck::passed("installation_profile", "installation profile was saved")
            .with_details(profile_json(&profile)),
    );
    state.actions_performed.push(
        SetupAction::performed(
            "installation_profile_saved",
            SetupActionKind::InstallationProfileSaved,
            "Installation profile was saved.",
        )
        .with_path(&runtime_home_record.registry_db_path),
    );

    let SetupWorkflowState {
        checks,
        actions_required,
        actions_optional,
        actions_performed,
        interactive_notes,
        ..
    } = state;
    let report = SetupReport::new(
        runtime_home_section,
        SetupSectionStatus::complete("installation profile was saved", profile_json(&profile)),
        commands,
        actions_required,
        actions_optional,
        actions_performed,
    );
    Ok(SetupWorkflowOutcome {
        output,
        runtime_home: runtime_home_record,
        installation_profile: Some(profile),
        report,
        checks,
        interactive_notes,
    })
}

pub(super) fn setup_options(
    options: SetupWorkflowOptions,
    current_dir: &Path,
) -> ParsedSetupOptions {
    ParsedSetupOptions {
        runtime_home: options
            .runtime_home
            .map(|path| absolute_path(current_dir, path)),
        link_bin: options
            .link_bin
            .map(|path| absolute_path(current_dir, path)),
        mcp_command: options
            .mcp_command
            .map(|path| absolute_path(current_dir, path)),
        output: if options.json {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
    }
}

pub(super) fn resolve_setup_runtime_home(
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

pub(super) fn runtime_home_report_section(record: &RuntimeHomeRecord) -> SetupSectionStatus {
    SetupSectionStatus::complete(
        "Runtime Home registry is ready",
        json!({
            "runtime_home": path_text(&record.runtime_home),
            "registry_db": path_text(&record.registry_db_path),
            "runtime_home_id": record.runtime_home_id,
        }),
    )
}

fn apply_interactive_choice(
    parsed: &mut ParsedSetupOptions,
    process: &impl SetupProcess,
    terminal: Option<&mut dyn SetupTerminal>,
    volicord_command: &DiscoveredCommand,
    volicord_mcp: &DiscoveredCommand,
    state: &mut SetupWorkflowState,
) -> Result<(), SetupCommandError> {
    if parsed.output != OutputFormat::Text || parsed.link_bin.is_some() {
        return Ok(());
    }

    let commands = current_command_availability(process, volicord_command);
    if commands.iter().all(|command| command.selected_path_ready()) {
        return Ok(());
    }

    let Some(terminal) = terminal else {
        return Ok(());
    };
    match prompt_command_availability_choice(
        terminal,
        process,
        &commands,
        [&volicord_command.path, &volicord_mcp.path],
    )? {
        InteractiveSetupChoice::LinkOnly(link_bin) => {
            parsed.link_bin = Some(link_bin);
        }
        InteractiveSetupChoice::LinkAndShell { link_bin, shell } => {
            parsed.link_bin = Some(link_bin);
            state.shell_startup_plan = Some(shell);
        }
        InteractiveSetupChoice::Manual { link_bin, command } => {
            if let Some(link_bin) = link_bin {
                parsed.link_bin = Some(link_bin);
            }
            state
                .interactive_notes
                .push(format!("manual_path_command: {command}"));
        }
        InteractiveSetupChoice::Skip => {
            state
                .interactive_notes
                .push("command linking was skipped".to_owned());
        }
        InteractiveSetupChoice::Cancelled(message) => {
            state.interactive_notes.push(message);
        }
    }
    Ok(())
}

fn install_requested_links(
    parsed: &ParsedSetupOptions,
    current_dir: &Path,
    process: &impl SetupProcess,
    volicord_command: &DiscoveredCommand,
    state: &mut SetupWorkflowState,
) -> Result<(), SetupCommandError> {
    let Some(link_bin) = &parsed.link_bin else {
        return Ok(());
    };
    let link_bin = absolute_path(current_dir, link_bin.clone());
    let mut link_bin_usable = false;
    match prepare_link_bin(&link_bin) {
        Ok(()) => {
            link_bin_usable = true;
            let volicord_link =
                install_command_link(&link_bin, &volicord_binary_name(), &volicord_command.path);
            state.command_links_ready = link_ready_for_path(&volicord_link);
            push_link_check(
                "link_volicord",
                "volicord command link",
                &link_bin,
                &volicord_binary_name(),
                &volicord_link,
                LinkCheckOutputs {
                    checks: &mut state.checks,
                    actions_required: &mut state.actions_required,
                    actions_performed: &mut state.actions_performed,
                },
            );
            state
                .link_results
                .insert("volicord".to_owned(), link_volicord_status(&volicord_link));
        }
        Err((summary, detail)) => {
            state.checks.push(
                DiagnosticCheck::failed("link_bin", summary)
                    .with_details(json!({ "path": path_text(&link_bin), "detail": detail })),
            );
            state.actions_required.push(
                SetupAction::required(
                    "repair_link_bin",
                    SetupActionKind::CommandLinks,
                    format!(
                        "Fix write access for {} after installing volicord in a writable PATH directory.",
                        link_bin.display()
                    ),
                )
                .with_command("volicord doctor")
                .with_path(&link_bin),
            );
            state
                .link_results
                .insert("volicord".to_owned(), "failed".to_owned());
        }
    }

    let on_path = path_directory_is_on_path(process.env_var(PATH_ENV).as_deref(), &link_bin);
    push_link_bin_path_result(&link_bin, on_path, link_bin_usable, process, state)?;
    Ok(())
}

fn push_link_bin_path_result(
    link_bin: &Path,
    on_path: bool,
    link_bin_usable: bool,
    process: &impl SetupProcess,
    state: &mut SetupWorkflowState,
) -> Result<(), SetupCommandError> {
    if !on_path {
        let mut shell_startup_ready = false;
        if link_bin_usable && state.command_links_ready {
            if let Some(plan) = state.shell_startup_plan.as_ref() {
                shell_startup_ready = write_shell_startup_block(
                    plan,
                    link_bin,
                    &mut state.checks,
                    &mut state.actions_required,
                    &mut state.actions_performed,
                );
            }
        }
        if link_bin_usable && state.command_links_ready {
            if shell_startup_ready {
                state.actions_required.push(
                    SetupAction::required(
                        "open_new_shell_for_path",
                        SetupActionKind::PathUpdate,
                        format!(
                            "Open a new shell or restart MCP hosts so PATH includes {}.",
                            link_bin.display()
                        ),
                    )
                    .with_path(link_bin),
                )
            } else {
                state.actions_required.push(
                    SetupAction::required(
                        "add_link_bin_to_path",
                        SetupActionKind::PathUpdate,
                        format!(
                            "Add {} to PATH before starting new shells or MCP hosts.",
                            link_bin.display()
                        ),
                    )
                    .with_command(shell_path_command(process, link_bin)?)
                    .with_path(link_bin),
                )
            }
        }
        state.checks.push(
            DiagnosticCheck::warning("link_bin_path", "link directory is not currently on PATH")
                .with_details(json!({ "link_bin": path_text(link_bin) })),
        );
    } else {
        state.checks.push(
            DiagnosticCheck::passed("link_bin_path", "link directory is on PATH")
                .with_details(json!({ "link_bin": path_text(link_bin) })),
        );
    }
    Ok(())
}

fn requested_link_bin_on_path(
    parsed: &ParsedSetupOptions,
    process: &impl SetupProcess,
    current_dir: &Path,
) -> Option<bool> {
    parsed.link_bin.as_ref().map(|link_bin| {
        let link_bin = absolute_path(current_dir, link_bin.clone());
        path_directory_is_on_path(process.env_var(PATH_ENV).as_deref(), &link_bin)
    })
}

fn current_command_availability(
    process: &impl SetupProcess,
    volicord_command: &DiscoveredCommand,
) -> Vec<crate::setup_report::CommandAvailability> {
    let path_env = process.env_var(PATH_ENV);
    vec![command_availability(
        "volicord_command",
        &volicord_binary_name(),
        volicord_command,
        path_env.as_deref(),
    )]
}

fn selected_bin_dir(parsed: &ParsedSetupOptions, volicord_command: &DiscoveredCommand) -> PathBuf {
    parsed
        .link_bin
        .clone()
        .unwrap_or_else(|| command_parent(&volicord_command.path))
}

pub(super) fn installation_profile_failed(
    summary: impl Into<String>,
    error: &SetupCommandError,
) -> SetupSectionStatus {
    SetupSectionStatus::failed(summary, json!({ "detail": error.to_string() }))
}

pub(super) fn setup_metadata_json(
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
