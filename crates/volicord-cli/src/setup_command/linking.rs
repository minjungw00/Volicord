use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde_json::json;

use crate::{
    setup_report::{SetupAction, SetupActionKind},
    shell_path::verify_directory_writable,
};

use super::{output::DiagnosticCheck, path_text};

pub(super) fn prepare_link_bin(link_bin: &Path) -> Result<(), (&'static str, String)> {
    fs::create_dir_all(link_bin)
        .map_err(|error| ("link directory could not be created", error.to_string()))?;
    verify_directory_writable(link_bin)
        .map_err(|error| ("link directory is not writable", error.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LinkInstallResult {
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

pub(super) fn install_command_link(
    link_bin: &Path,
    name: &str,
    target: &Path,
) -> LinkInstallResult {
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

pub(super) struct LinkCheckOutputs<'a> {
    pub(super) checks: &'a mut Vec<DiagnosticCheck>,
    pub(super) actions_required: &'a mut Vec<SetupAction>,
    pub(super) actions_performed: &'a mut Vec<SetupAction>,
}

pub(super) fn push_link_check(
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
                        "Move or remove the existing {} path after installing volicord in a writable PATH directory such as {}.",
                        path.display(),
                        link_bin.display()
                    ),
                )
                .with_command("volicord doctor")
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
                        "Fix write access for {} after installing volicord in a writable PATH directory such as {}.",
                        path.display(),
                        link_bin.display()
                    ),
                )
                .with_command("volicord doctor")
                .with_path(path),
            );
        }
    }
}

pub(super) fn link_volicord_status(result: &LinkInstallResult) -> String {
    match result {
        LinkInstallResult::Created(_) => "created",
        LinkInstallResult::Existing(_) => "existing",
        LinkInstallResult::UnsafeExisting(_) => "unsafe_existing",
        LinkInstallResult::Unsupported(_) => "unsupported",
        LinkInstallResult::Failed { .. } => "failed",
    }
    .to_owned()
}

pub(super) fn link_ready_for_path(result: &LinkInstallResult) -> bool {
    matches!(
        result,
        LinkInstallResult::Created(_) | LinkInstallResult::Existing(_)
    )
}
