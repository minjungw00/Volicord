use std::{fs, path::Path};

use serde_json::Value;

use crate::{
    guard_integration::{
        files::{
            managed_block_conflict, managed_json_projection_merge, plan_managed_exact_json_file,
            plan_managed_script_file, plan_policy_file, FilePlanStatus, GeneratedFileWriteKind,
        },
        GuardIntegrationError, GuardIntegrationPlan, ManagedJsonProjection,
    },
    host_integration::HostIntegrationFileKind,
    managed_block::{self, ManagedBlockWrite},
};

pub(crate) fn apply_guard_integration(
    mut plan: GuardIntegrationPlan,
) -> Result<GuardIntegrationPlan, GuardIntegrationError> {
    for file in &mut plan.generated_files {
        file.status = match file.write_kind {
            GeneratedFileWriteKind::Block {
                start_marker,
                end_marker,
                require_existing_marker,
            } => write_managed_markdown_file(
                &file.path,
                &file.content,
                start_marker,
                end_marker,
                require_existing_marker,
            )?,
            GeneratedFileWriteKind::Json => {
                write_managed_json_file(&file.path, &file.policy_value()?)?
            }
            GeneratedFileWriteKind::ExactJson => {
                write_managed_exact_json_file(&file.path, &file.policy_value()?, file.kind)?
            }
            GeneratedFileWriteKind::JsonProjection { projection } => {
                write_managed_json_projection_file(&file.path, &file.policy_value()?, projection)?
            }
            GeneratedFileWriteKind::Script => {
                write_managed_script_file(&file.path, &file.content, file.kind)?
            }
        };
    }
    Ok(plan)
}

pub(crate) fn write_managed_markdown_file(
    path: &Path,
    block: &str,
    start_marker: &'static str,
    end_marker: &'static str,
    require_existing_marker: bool,
) -> Result<FilePlanStatus, GuardIntegrationError> {
    if require_existing_marker && path.exists() {
        let existing = fs::read_to_string(path).map_err(|error| {
            GuardIntegrationError::runtime(format!("failed to read {}: {error}", path.display()))
        })?;
        if !existing.contains(start_marker) {
            return Err(GuardIntegrationError::runtime(format!(
                "{} already exists without a Volicord-managed block",
                path.display()
            )));
        }
    }
    match managed_block::write_managed_block_with_markers(path, block, start_marker, end_marker)
        .map_err(|error| {
            GuardIntegrationError::runtime(format!("failed to write {}: {error}", path.display()))
        })? {
        Ok(ManagedBlockWrite::Created(_)) => Ok(FilePlanStatus::Created),
        Ok(ManagedBlockWrite::Updated(_)) => Ok(FilePlanStatus::Updated),
        Ok(ManagedBlockWrite::Unchanged(_)) => Ok(FilePlanStatus::Unchanged),
        Err(error) => Err(managed_block_conflict(error)),
    }
}

pub(crate) fn write_managed_json_file(
    path: &Path,
    value: &Value,
) -> Result<FilePlanStatus, GuardIntegrationError> {
    let mut content = serde_json::to_string_pretty(value)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    content.push('\n');
    let planned = plan_policy_file(path, value)?;
    if planned.status == FilePlanStatus::Unchanged {
        return Ok(FilePlanStatus::Unchanged);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to create {}: {error}",
                parent.display()
            ))
        })?;
    }
    fs::write(path, content).map_err(|error| {
        GuardIntegrationError::runtime(format!("failed to write {}: {error}", path.display()))
    })?;
    Ok(match planned.status {
        FilePlanStatus::PlannedCreate => FilePlanStatus::Created,
        FilePlanStatus::PlannedUpdate => FilePlanStatus::Updated,
        other => other,
    })
}

pub(crate) fn write_managed_exact_json_file(
    path: &Path,
    value: &Value,
    kind: HostIntegrationFileKind,
) -> Result<FilePlanStatus, GuardIntegrationError> {
    let mut content = serde_json::to_string_pretty(value)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    content.push('\n');
    let planned = plan_managed_exact_json_file(kind, path, value)?;
    if planned.status == FilePlanStatus::Unchanged {
        return Ok(FilePlanStatus::Unchanged);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to create {}: {error}",
                parent.display()
            ))
        })?;
    }
    fs::write(path, content).map_err(|error| {
        GuardIntegrationError::runtime(format!("failed to write {}: {error}", path.display()))
    })?;
    Ok(match planned.status {
        FilePlanStatus::PlannedCreate => FilePlanStatus::Created,
        FilePlanStatus::PlannedUpdate => FilePlanStatus::Updated,
        other => other,
    })
}

pub(crate) fn write_managed_json_projection_file(
    path: &Path,
    value: &Value,
    projection: ManagedJsonProjection,
) -> Result<FilePlanStatus, GuardIntegrationError> {
    let mut existed = true;
    let existing = match fs::read_to_string(path) {
        Ok(text) => {
            let value = serde_json::from_str::<Value>(&text).map_err(|error| {
                GuardIntegrationError::runtime(format!(
                    "existing JSON configuration is not valid JSON: {} ({error})",
                    path.display()
                ))
            })?;
            Some(value)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            existed = false;
            None
        }
        Err(error) => {
            return Err(GuardIntegrationError::runtime(format!(
                "failed to read {}: {error}",
                path.display()
            )));
        }
    };
    let current = existing.unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let merged = managed_json_projection_merge(&current, value, projection)?;
    if merged == current {
        return Ok(FilePlanStatus::Unchanged);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to create {}: {error}",
                parent.display()
            ))
        })?;
    }
    let mut text = serde_json::to_string_pretty(&merged)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    text.push('\n');
    fs::write(path, text).map_err(|error| {
        GuardIntegrationError::runtime(format!("failed to write {}: {error}", path.display()))
    })?;
    Ok(if existed {
        FilePlanStatus::Updated
    } else {
        FilePlanStatus::Created
    })
}

pub(crate) fn write_managed_script_file(
    path: &Path,
    content: &str,
    kind: HostIntegrationFileKind,
) -> Result<FilePlanStatus, GuardIntegrationError> {
    let planned = plan_managed_script_file(path, content, kind)?;
    if planned.status != FilePlanStatus::Unchanged {
        let existing_matches = fs::read_to_string(path)
            .map(|existing| existing == content)
            .unwrap_or(false);
        if !existing_matches {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    GuardIntegrationError::runtime(format!(
                        "failed to create {}: {error}",
                        parent.display()
                    ))
                })?;
            }
            fs::write(path, content).map_err(|error| {
                GuardIntegrationError::runtime(format!(
                    "failed to write {}: {error}",
                    path.display()
                ))
            })?;
        }
        set_script_executable(path)?;
    }
    Ok(match planned.status {
        FilePlanStatus::PlannedCreate => FilePlanStatus::Created,
        FilePlanStatus::PlannedUpdate => FilePlanStatus::Updated,
        other => other,
    })
}

#[cfg(unix)]
pub(crate) fn set_script_executable(path: &Path) -> Result<(), GuardIntegrationError> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to inspect {} permissions: {error}",
                path.display()
            ))
        })?
        .permissions();
    let mode = permissions.mode();
    if mode & 0o100 == 0 {
        permissions.set_mode(mode | 0o755);
        fs::set_permissions(path, permissions).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "failed to make {} executable: {error}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn set_script_executable(_path: &Path) -> Result<(), GuardIntegrationError> {
    Ok(())
}
