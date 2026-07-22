//! Guard file, hook-execution, and observation checks.

use super::*;

pub(super) fn guard_checks_for_connection(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
) -> Result<ConnectionCheckEvaluation, ConnectionCommandError> {
    let mut installations = Vec::new();
    for project in projects {
        installations.extend(list_guard_installations(
            runtime_home,
            &connection.connection_internal_id,
            Some(&project.project_id),
        )?);
    }
    if installations.is_empty() {
        installations =
            list_guard_installations(runtime_home, &connection.connection_internal_id, None)?;
    }

    let mut audit = GuardAuditFacts::default();
    let mut all_required_phases_observed = !installations.is_empty();
    let mut prompt_capture_observed = !installations.is_empty();
    let mut required_phases = Vec::new();
    let mut observed_phases = Vec::new();
    let mut incompatible_event_ids = Vec::new();
    let mut last_current_observation_at = None;
    let mut installation_ids = Vec::new();
    let mut observation_revision_mismatch_installation_ids = Vec::new();

    for installation in &installations {
        installation_ids.push(installation.guard_installation_id.clone());
        audit.merge(guard_file_findings_for_installation(
            runtime_home,
            installation,
            connection,
            projects,
        ));
        let binding_is_current =
            guard_manifest_binding_valid_for_installation(installation, connection, projects);
        if !binding_is_current {
            observation_revision_mismatch_installation_ids
                .push(installation.guard_installation_id.clone());
        }
        let observation =
            guard_observation_summary(runtime_home, &installation.project_id, installation)?;
        required_phases.extend(observation.required_phases.iter().cloned());
        observed_phases.extend(observation.observed_phases.iter().cloned());
        incompatible_event_ids.extend(observation.incompatible_event_ids.iter().cloned());
        let observation_is_current =
            binding_is_current && observation.all_required_phases_observed();
        all_required_phases_observed &= observation_is_current;
        prompt_capture_observed &= observation_is_current && observation.prompt_capture_observed();
        last_current_observation_at = latest_timestamp(
            last_current_observation_at,
            observation.last_observed_at.as_deref(),
        )?;
    }

    audit.sort_dedup();
    installation_ids.sort();
    installation_ids.dedup();
    observation_revision_mismatch_installation_ids.sort();
    observation_revision_mismatch_installation_ids.dedup();
    required_phases.sort();
    required_phases.dedup();
    observed_phases.sort();
    observed_phases.dedup();
    incompatible_event_ids.sort();
    incompatible_event_ids.dedup();

    let missing_required_phases = required_phases
        .iter()
        .filter(|phase| !observed_phases.contains(phase))
        .cloned()
        .collect::<Vec<_>>();
    let configured_phase_gaps = audit
        .missing_required_phases
        .iter()
        .map(|phase| phase.as_str().to_owned())
        .collect::<Vec<_>>();
    let files_status = if !installations.is_empty()
        && audit.generated_config_verified()
        && configured_phase_gaps.is_empty()
    {
        ConnectionCheckStatus::Passed
    } else {
        ConnectionCheckStatus::Failed
    };
    let observation_status = if !incompatible_event_ids.is_empty() {
        ConnectionCheckStatus::Failed
    } else if all_required_phases_observed {
        ConnectionCheckStatus::Passed
    } else {
        ConnectionCheckStatus::Pending
    };
    let hook_execution_status = if observed_phases.is_empty() && incompatible_event_ids.is_empty() {
        ConnectionCheckStatus::Pending
    } else {
        ConnectionCheckStatus::Passed
    };

    let artifact_issues = audit
        .findings
        .iter()
        .map(|finding| {
            json!({
                "artifact": guard_managed_artifact_name(finding.artifact),
                "path": finding.path.display().to_string(),
                "issue": guard_artifact_issue_name(finding.issue),
                "details": finding.details,
            })
        })
        .collect::<Vec<_>>();
    let manifest_issues = audit
        .manifest_issues
        .iter()
        .map(|issue| guard_manifest_issue_name(*issue))
        .collect::<Vec<_>>();
    let affected_paths = audit
        .affected_paths()
        .into_iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let observed_at = last_current_observation_at
        .as_ref()
        .map(UtcTimestamp::to_canonical_string);

    let guard_findings = guard_boundary_findings(
        connection,
        &audit,
        &installation_ids,
        files_status == ConnectionCheckStatus::Failed,
        &missing_required_phases,
        &incompatible_event_ids,
        prompt_capture_observed,
        &observation_revision_mismatch_installation_ids,
        current_timestamp(),
    )?;

    let checks = block_failed_dependencies(vec![
        with_direct_causes(
            canonical_check(
                ConnectionCheckKind::GuardFiles,
                files_status,
                "guard_files_failed",
                if files_status == ConnectionCheckStatus::Passed {
                    "Guard managed files match the current typed manifest expectations"
                } else {
                    "Guard managed files do not match the current typed manifest expectations"
                },
                Some(json!({
                    "installation_ids": installation_ids,
                    "affected_paths": affected_paths,
                    "artifact_issues": artifact_issues,
                    "manifest_issues": manifest_issues,
                    "missing_required_phases": configured_phase_gaps,
                })),
                None,
            )?,
            guard_findings.files,
        )?,
        canonical_check(
            ConnectionCheckKind::GuardHookExecution,
            hook_execution_status,
            if hook_execution_status == ConnectionCheckStatus::Passed {
                "guard_hook_execution_observed"
            } else {
                "guard_hook_execution_pending"
            },
            if hook_execution_status == ConnectionCheckStatus::Passed {
                "A current managed Guard hook executed"
            } else {
                "Current managed Guard hook execution has not been observed"
            },
            Some(json!({
                "observed_phases": observed_phases,
                "last_current_observation_at": observed_at,
            })),
            observed_at.as_deref(),
        )?,
        with_direct_causes(
            canonical_check(
                ConnectionCheckKind::GuardObservation,
                observation_status,
                match observation_status {
                    ConnectionCheckStatus::Passed => "guard_observation_passed",
                    ConnectionCheckStatus::Pending => "guard_observation_pending",
                    ConnectionCheckStatus::Failed => "guard_observation_failed",
                    ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                        unreachable!("raw Guard observation uses passed, pending, or failed")
                    }
                },
                match observation_status {
                    ConnectionCheckStatus::Passed => {
                        "Every current required Guard hook phase was observed"
                    }
                    ConnectionCheckStatus::Pending => {
                        "One or more current required Guard hook phases have not been observed"
                    }
                    ConnectionCheckStatus::Failed => {
                        "A current Guard event reported an incompatible hook contract"
                    }
                    ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                        unreachable!("raw Guard observation uses passed, pending, or failed")
                    }
                },
                Some(json!({
                    "required_phases": required_phases,
                    "observed_phases": observed_phases,
                    "missing_required_phases": missing_required_phases,
                    "incompatible_event_ids": incompatible_event_ids,
                    "prompt_capture": {
                        "host_supported": audit.prompt_capture_host_supported,
                        "configured": audit.prompt_capture_configured,
                        "observed": prompt_capture_observed,
                    },
                    "last_current_observation_at": observed_at,
                })),
                observed_at.as_deref(),
            )?,
            guard_findings.observation,
        )?,
    ])?;
    Ok(ConnectionCheckEvaluation {
        checks,
        inline_findings: guard_findings.current,
        persisted_finding_seed_ids: Vec::new(),
    })
}
