use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};

use crate::{
    setup_report::{CommandAvailability, SetupAction, SetupActionKind},
    shell_path::{
        detect_command_on_path, path_directory_is_on_path, paths_equivalent,
        setup_link_dir_candidates, SetupLinkDirCandidate,
    },
};

use super::{
    command_parent, output::DiagnosticCheck, path_text, workflow::ParsedSetupOptions,
    SetupCommandError, SetupProcess,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiscoveredCommand {
    pub(super) path: PathBuf,
    pub(super) source: &'static str,
}

pub(super) fn command_availability(
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

pub(super) fn missing_command_availability(
    id: impl Into<String>,
    command_name: &str,
) -> CommandAvailability {
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

pub(super) fn push_command_availability_checks(
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

pub(super) fn plan_setup_actions(
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
            "Create command links manually in a PATH directory; Volicord will not modify shell startup files.",
        );
        if let Some(link_bin) = suggested_link_bin(process) {
            action = action.with_path(&link_bin);
        }
        push_unique_action(actions_optional, action);
    }
}

fn push_unique_action(actions: &mut Vec<SetupAction>, action: SetupAction) {
    if !actions.iter().any(|existing| existing.id == action.id) {
        actions.push(action);
    }
}

fn suggested_link_bin(process: &impl SetupProcess) -> Option<PathBuf> {
    suggested_link_bin_candidate(process).map(|candidate| candidate.path().to_path_buf())
}

pub(super) fn suggested_link_bin_candidate(
    process: &impl SetupProcess,
) -> Option<SetupLinkDirCandidate> {
    setup_link_dir_candidates(&|name| process.env_var(name))
        .into_iter()
        .find(SetupLinkDirCandidate::is_usable)
}

pub(super) fn discover_volicord_command(
    process: &impl SetupProcess,
) -> Result<DiscoveredCommand, SetupCommandError> {
    let current_exe = process.current_exe().map_err(SetupCommandError::Runtime)?;
    let path = canonical_existing_file(&current_exe, "volicord command")?;
    Ok(DiscoveredCommand {
        path,
        source: "current_exe",
    })
}

pub(super) fn discover_mcp_command(
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
    if super::is_executable_file(&path) {
        Ok(path)
    } else {
        Err(SetupCommandError::Runtime(format!(
            "{label} must be executable: {}",
            path.display()
        )))
    }
}

pub(super) fn command_availability_summary(command: &CommandAvailability) -> String {
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
