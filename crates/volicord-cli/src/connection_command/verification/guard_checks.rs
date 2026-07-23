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
    let hook_execution_status =
        if files_status == ConnectionCheckStatus::Failed || !incompatible_event_ids.is_empty() {
            ConnectionCheckStatus::Failed
        } else if observed_phases.is_empty() {
            ConnectionCheckStatus::Pending
        } else {
            ConnectionCheckStatus::Passed
        };
    let hook_activation_state = HookActivationState::from_evidence(HookActivationEvidence {
        setup_changed_definition: false,
        host: None,
        current_definition_event_observed: !observed_phases.is_empty(),
    });
    let hook_activation_status = match hook_activation_state {
        HookActivationState::EffectiveByObservation | HookActivationState::ManagedByPolicy => {
            ConnectionCheckStatus::Passed
        }
        HookActivationState::Disabled => ConnectionCheckStatus::Failed,
        HookActivationState::Unknown
        | HookActivationState::ReviewRequiredBySetup
        | HookActivationState::BypassedForInvocation => ConnectionCheckStatus::Pending,
    };
    let mut hook_definition_hashes = installations
        .iter()
        .filter_map(|installation| {
            volicord_types::guard_manifest_from_json(&installation.manifest_json).ok()
        })
        .filter_map(|manifest| {
            manifest
                .managed_files
                .into_iter()
                .find(|file| file.artifact() == GuardManagedArtifact::HostHookConfig)
                .map(|file| file.content_hash().as_str().to_owned())
        })
        .collect::<Vec<_>>();
    hook_definition_hashes.sort();
    hook_definition_hashes.dedup();

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

    let verification_observed_at = current_timestamp().to_canonical_string();
    let current_revision = connection_integration_revision(connection)?;
    let verification_run = latest_guard_integration_verification_for_connection(
        runtime_home,
        &connection.connection_internal_id,
        &current_revision,
    )?;
    let verification_workflow = verification_run
        .as_ref()
        .map(|run| {
            current_guard_integration_verification_workflow(
                runtime_home,
                run,
                &verification_observed_at,
            )
        })
        .transpose()?;
    let verification_status = match verification_workflow {
        Some(IntegrationVerificationWorkflowState::Complete { .. }) => {
            ConnectionCheckStatus::Passed
        }
        Some(IntegrationVerificationWorkflowState::RestartRequired {
            reason: IntegrationVerificationRestartReason::Failed,
            ..
        }) => ConnectionCheckStatus::Failed,
        Some(
            IntegrationVerificationWorkflowState::AwaitingProbe { .. }
            | IntegrationVerificationWorkflowState::AwaitingHookCompletion { .. }
            | IntegrationVerificationWorkflowState::RestartRequired {
                reason: IntegrationVerificationRestartReason::Expired,
                ..
            },
        )
        | None => ConnectionCheckStatus::Pending,
    };

    let mut hook_execution_causes = guard_findings.files.clone();
    hook_execution_causes.extend(guard_findings.observation.iter().cloned());
    hook_execution_causes.sort();
    hook_execution_causes.dedup();
    let checks = block_failed_dependencies(vec![
        canonical_check(
            ConnectionCheckKind::HookSourceActivation,
            hook_activation_status,
            match hook_activation_state {
                HookActivationState::Unknown => "hook_source_activation_unknown",
                HookActivationState::ReviewRequiredBySetup => {
                    "hook_source_review_required_by_setup"
                }
                HookActivationState::EffectiveByObservation => {
                    "hook_source_effective_by_observation"
                }
                HookActivationState::ManagedByPolicy => "hook_source_managed_by_policy",
                HookActivationState::BypassedForInvocation => {
                    "hook_source_bypassed_for_invocation"
                }
                HookActivationState::Disabled => "hook_source_disabled",
            },
            match hook_activation_state {
                HookActivationState::Unknown => {
                    "Project-local hook-source activation is unknown"
                }
                HookActivationState::ReviewRequiredBySetup => {
                    "Current setup changed the project hook definition"
                }
                HookActivationState::EffectiveByObservation => {
                    "A current-definition project hook event was observed"
                }
                HookActivationState::ManagedByPolicy => {
                    "Current host evidence identifies managed hook policy"
                }
                HookActivationState::BypassedForInvocation => {
                    "One invocation bypassed hook trust without proving persisted activation"
                }
                HookActivationState::Disabled => {
                    "Current host configuration explicitly disables hooks"
                }
            },
            Some(json!({
                "activation_state": hook_activation_state.as_str(),
                "definition_hashes": hook_definition_hashes,
                "installation_ids": installation_ids,
                "last_current_observation_at": observed_at,
                "host_evidence": Value::Null,
            })),
            observed_at.as_deref(),
        )?,
        with_direct_causes(canonical_check(
            ConnectionCheckKind::GuardHookExecution,
            hook_execution_status,
            match hook_execution_status {
                ConnectionCheckStatus::Passed => "guard_hook_execution_observed",
                ConnectionCheckStatus::Pending => "guard_hook_execution_pending",
                ConnectionCheckStatus::Failed => "guard_hook_execution_failed",
                ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                    unreachable!("raw Guard hook execution does not block itself")
                }
            },
            match hook_execution_status {
                ConnectionCheckStatus::Passed => "A current managed Guard hook executed",
                ConnectionCheckStatus::Pending => {
                    "Current managed Guard hook execution has not been observed"
                }
                ConnectionCheckStatus::Failed => {
                    "Guard managed files or a current hook contract are incompatible"
                }
                ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                    unreachable!("raw Guard hook execution does not block itself")
                }
            },
            Some(json!({
                "installation_ids": installation_ids,
                "affected_paths": affected_paths,
                "artifact_issues": artifact_issues,
                "manifest_issues": manifest_issues,
                "configured_missing_phases": configured_phase_gaps,
                "ambient_observation": {
                    "status": match observation_status {
                        ConnectionCheckStatus::Passed => "passed",
                        ConnectionCheckStatus::Pending => "pending",
                        ConnectionCheckStatus::Failed => "failed",
                        ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => unreachable!(),
                    },
                    "required_phases": required_phases,
                    "missing_required_phases": missing_required_phases,
                    "incompatible_event_ids": incompatible_event_ids,
                    "prompt_capture": {
                        "host_supported": audit.prompt_capture_host_supported,
                        "configured": audit.prompt_capture_configured,
                        "observed": prompt_capture_observed,
                    },
                },
                "observed_phases": observed_phases,
                "last_current_observation_at": observed_at,
            })),
            observed_at.as_deref(),
        )?, hook_execution_causes)?,
        canonical_check(
            ConnectionCheckKind::GuardVerification,
            verification_status,
            match verification_status {
                ConnectionCheckStatus::Passed => "guard_verification_passed",
                ConnectionCheckStatus::Pending => "guard_verification_pending",
                ConnectionCheckStatus::Failed => "guard_verification_failed",
                ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                    unreachable!("raw Guard verification uses passed, pending, or failed")
                }
            },
            match verification_status {
                ConnectionCheckStatus::Passed => {
                    "One current managed Codex turn completed the correlated MCP and Guard verification"
                }
                ConnectionCheckStatus::Pending => {
                    "The current Connection has no completed correlated in-chat Guard verification"
                }
                ConnectionCheckStatus::Failed => {
                    "The newest correlated in-chat Guard verification no longer matches current integration ownership"
                }
                ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                    unreachable!("raw Guard verification uses passed, pending, or failed")
                }
            },
            Some(json!({
                "verification_id": verification_run.as_ref().map(|run| run.verification_id.as_str()),
                "runtime_session_id": verification_run.as_ref().map(|run| run.runtime_session_id.as_str()),
                "host_turn_id": verification_run.as_ref().map(|run| run.host_turn_id.as_str()),
                "matched_prompt_event_id": verification_run.as_ref().and_then(|run| run.matched_prompt_event_id.as_deref()),
                "matched_pre_tool_event_id": verification_run.as_ref().and_then(|run| run.matched_pre_tool_event_id.as_deref()),
                "matched_post_tool_event_id": verification_run.as_ref().and_then(|run| run.matched_post_tool_event_id.as_deref()),
            })),
            (verification_status == ConnectionCheckStatus::Passed)
                .then_some(verification_observed_at.as_str()),
        )?,
    ])?;
    Ok(ConnectionCheckEvaluation {
        checks,
        inline_findings: guard_findings.current,
        persisted_finding_seed_ids: Vec::new(),
    })
}
