use std::path::{Path, PathBuf};

use serde_json::json;

use crate::{
    managed_block::{self, ManagedBlockWrite},
    setup_report::{SetupAction, SetupActionKind},
    shell_path::paths_equivalent,
};

use super::{command_parent, output::DiagnosticCheck, path_text, SetupCommandError, SetupProcess};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ShellStartupPlan {
    pub(super) shell_name: String,
    pub(super) target_file: PathBuf,
    pub(super) block: String,
    pub(super) command: String,
}

pub(super) fn shell_startup_plan(
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
                ));
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

pub(super) fn write_shell_startup_block(
    plan: &ShellStartupPlan,
    link_bin: &Path,
    checks: &mut Vec<DiagnosticCheck>,
    actions_required: &mut Vec<SetupAction>,
    actions_performed: &mut Vec<SetupAction>,
) -> bool {
    match managed_block::write_managed_block(&plan.target_file, &plan.block) {
        Ok(ManagedBlockWrite::Created(path)) | Ok(ManagedBlockWrite::Updated(path)) => {
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
            true
        }
        Ok(ManagedBlockWrite::Unchanged(path)) => {
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
            true
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
            false
        }
    }
}

pub(super) fn shell_path_command(
    process: &impl SetupProcess,
    dir: &Path,
) -> Result<String, SetupCommandError> {
    shell_path_command_for_selected_dirs(process, &[dir.to_path_buf()])
}

pub(super) fn shell_path_command_for_selected_dirs(
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

#[cfg(not(windows))]
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

#[cfg(not(windows))]
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

pub(super) fn selected_command_dirs(paths: [&Path; 2]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for path in paths {
        let dir = command_parent(path);
        if !dirs.iter().any(|existing| paths_equivalent(existing, &dir)) {
            dirs.push(dir);
        }
    }
    dirs
}
