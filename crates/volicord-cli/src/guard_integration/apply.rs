use crate::{
    guard_integration::{
        files::{
            apply_managed_file_retirement, ensure_generated_file_plan_fresh,
            generated_file_plan_matches_artifact_spec, managed_block_conflict,
            write_managed_file_if_fresh, FilePlanStatus, GeneratedFilePlan, GeneratedFileWriteKind,
        },
        git_exclude::plan_git_excludes,
        GuardIntegrationError, GuardIntegrationPlan,
    },
    host_integration::ConnectionIntent,
    managed_block,
};
use volicord_types::{GuardManagedArtifact, IntegrationProfile};

pub(crate) fn apply_guard_migration_protection(
    plan: &mut GuardIntegrationPlan,
) -> Result<(), GuardIntegrationError> {
    if plan.migration_protection_applied {
        return Ok(());
    }
    if let Some(protection) = plan.migration_protection.as_mut() {
        protection.status = apply_generated_file(protection)?;
    }
    plan.migration_protection_applied = true;
    Ok(())
}

pub(crate) fn apply_guard_integration(
    mut plan: GuardIntegrationPlan,
) -> Result<GuardIntegrationPlan, GuardIntegrationError> {
    apply_guard_migration_protection(&mut plan)?;
    for file in &mut plan.generated_files {
        if matches!(
            file.artifact,
            GuardManagedArtifact::GitInfoExclude | GuardManagedArtifact::VolicordPolicy
        ) {
            continue;
        }
        file.status = apply_generated_file(file)?;
    }
    for retirement in &mut plan.retired_files {
        retirement.status = apply_managed_file_retirement(retirement)?;
    }
    for file in &mut plan.generated_files {
        if file.artifact == GuardManagedArtifact::VolicordPolicy {
            file.status = apply_generated_file(file)?;
        }
    }
    let connection_intent = parse_connection_intent(&plan.connection_intent)?;
    let profile = parse_integration_profile(&plan.guard_profile)?;
    if let Some(mut final_exclude) = plan_git_excludes(&plan.repo_root, connection_intent, profile)?
    {
        final_exclude.status = apply_generated_file(&final_exclude)?;
        if let Some(recorded) = plan
            .generated_files
            .iter_mut()
            .find(|file| file.artifact == GuardManagedArtifact::GitInfoExclude)
        {
            *recorded = final_exclude;
        } else {
            plan.generated_files.insert(0, final_exclude);
        }
    }
    Ok(plan)
}

fn parse_connection_intent(value: &str) -> Result<ConnectionIntent, GuardIntegrationError> {
    match value {
        "personal" => Ok(ConnectionIntent::Personal),
        "shared" => Ok(ConnectionIntent::Shared),
        _ => Err(GuardIntegrationError::runtime(
            "guard integration has an unsupported connection intent",
        )),
    }
}

fn parse_integration_profile(value: &str) -> Result<IntegrationProfile, GuardIntegrationError> {
    match value {
        "record" => Ok(IntegrationProfile::Record),
        _ => Err(GuardIntegrationError::runtime(
            "guard integration has an unsupported profile",
        )),
    }
}

pub(crate) fn apply_generated_file(
    file: &GeneratedFilePlan,
) -> Result<FilePlanStatus, GuardIntegrationError> {
    if !generated_file_plan_matches_artifact_spec(file) {
        return Err(GuardIntegrationError::runtime(
            "managed file plan does not match the Guard artifact registry",
        ));
    }
    ensure_generated_file_plan_fresh(file)?;
    if file.status == FilePlanStatus::Unchanged {
        return Ok(FilePlanStatus::Unchanged);
    }

    let content = match file.write_kind {
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
            managed_block::apply_managed_block_with_markers(
                existing,
                &file.content,
                start_marker,
                end_marker,
            )
            .map_err(managed_block_conflict)?
        }
        GeneratedFileWriteKind::Json | GeneratedFileWriteKind::ExactJson => file.content.clone(),
        GeneratedFileWriteKind::Script => file.content.clone(),
    };
    let executable = file.artifact.spec().executable_required;
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
