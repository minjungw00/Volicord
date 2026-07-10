use serde_json::Value;

#[cfg(test)]
use std::path::Path;

use crate::{
    guard_integration::{
        files::{
            ensure_generated_file_plan_fresh, managed_block_conflict,
            managed_json_projection_merge, write_managed_file_if_fresh, FilePlanStatus,
            GeneratedFilePlan, GeneratedFileWriteKind,
        },
        GuardIntegrationError, GuardIntegrationPlan, ManagedJsonProjection,
    },
    managed_block,
};

pub(crate) fn apply_guard_integration(
    mut plan: GuardIntegrationPlan,
) -> Result<GuardIntegrationPlan, GuardIntegrationError> {
    for file in &mut plan.generated_files {
        file.status = apply_generated_file(file)?;
    }
    Ok(plan)
}

fn apply_generated_file(file: &GeneratedFilePlan) -> Result<FilePlanStatus, GuardIntegrationError> {
    ensure_generated_file_plan_fresh(file)?;
    if file.status == FilePlanStatus::Unchanged {
        return Ok(FilePlanStatus::Unchanged);
    }

    let (content, executable) = match file.write_kind {
        GeneratedFileWriteKind::Block {
            start_marker,
            end_marker,
            require_existing_marker,
        } => {
            let existing = file.target_snapshot.text().unwrap_or("");
            if require_existing_marker
                && file.target_snapshot.text().is_some()
                && !existing.contains(start_marker)
            {
                return Err(GuardIntegrationError::runtime(format!(
                    "{} already exists without a Volicord-managed block",
                    file.path.display()
                )));
            }
            let content = managed_block::apply_managed_block_with_markers(
                existing,
                &file.content,
                start_marker,
                end_marker,
            )
            .map_err(managed_block_conflict)?;
            (content, false)
        }
        GeneratedFileWriteKind::Json | GeneratedFileWriteKind::ExactJson => {
            (file.content.clone(), false)
        }
        GeneratedFileWriteKind::JsonProjection { projection } => {
            (render_json_projection(file, projection)?, false)
        }
        GeneratedFileWriteKind::Script => (file.content.clone(), true),
    };
    write_managed_file_if_fresh(file, &content, executable)?;
    Ok(match file.status {
        FilePlanStatus::PlannedCreate => FilePlanStatus::Created,
        FilePlanStatus::PlannedUpdate => FilePlanStatus::Updated,
        other => {
            return Err(GuardIntegrationError::runtime(format!(
                "managed file has non-applicable plan status {}: {}",
                other.as_str(),
                file.path.display()
            )));
        }
    })
}

fn render_json_projection(
    file: &GeneratedFilePlan,
    projection: ManagedJsonProjection,
) -> Result<String, GuardIntegrationError> {
    let current = match file.target_snapshot.text() {
        Some(text) => serde_json::from_str::<Value>(text).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "existing JSON configuration is not valid JSON: {} ({error})",
                file.path.display()
            ))
        })?,
        None => Value::Object(serde_json::Map::new()),
    };
    let merged = managed_json_projection_merge(&current, &file.policy_value()?, projection)?;
    let mut text = serde_json::to_string_pretty(&merged)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))?;
    text.push('\n');
    Ok(text)
}

#[cfg(all(test, unix))]
pub(crate) fn set_script_executable(path: &Path) -> Result<(), GuardIntegrationError> {
    use std::fs;
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

#[cfg(all(test, not(unix)))]
pub(crate) fn set_script_executable(_path: &Path) -> Result<(), GuardIntegrationError> {
    Ok(())
}
