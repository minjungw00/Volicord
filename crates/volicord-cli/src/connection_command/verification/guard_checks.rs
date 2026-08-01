//! Guard file, hook-execution, and observation checks.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GuardCheckEvaluationUnavailableCause {
    ProjectStore,
    GuardState,
}

#[derive(Debug)]
pub(super) struct GuardCheckEvaluationUnavailable {
    pub(super) cause: GuardCheckEvaluationUnavailableCause,
    source: ConnectionCommandError,
}

impl GuardCheckEvaluationUnavailable {
    fn guard_state(source: impl Into<ConnectionCommandError>) -> Self {
        Self {
            cause: GuardCheckEvaluationUnavailableCause::GuardState,
            source: source.into(),
        }
    }

    fn project_store(source: impl Into<ConnectionCommandError>) -> Self {
        Self {
            cause: GuardCheckEvaluationUnavailableCause::ProjectStore,
            source: source.into(),
        }
    }

    pub(super) fn into_source(self) -> ConnectionCommandError {
        self.source
    }
}

impl From<ConnectionCommandError> for GuardCheckEvaluationUnavailable {
    fn from(source: ConnectionCommandError) -> Self {
        Self::guard_state(source)
    }
}

impl From<StoreError> for GuardCheckEvaluationUnavailable {
    fn from(source: StoreError) -> Self {
        Self::guard_state(ConnectionCommandError::from(source))
    }
}

pub(super) fn guard_checks_for_connection(
    runtime_home: &Path,
    connection: &AgentConnectionRecord,
    projects: &[ConnectionProjectRecord],
    current_revision: &IntegrationRevision,
    evaluated_at: &UtcTimestamp,
    selected_membership: Option<&ConnectionProjectRecord>,
) -> Result<ConnectionCheckEvaluation, GuardCheckEvaluationUnavailable> {
    let mut installations = Vec::new();
    for project in projects {
        installations.extend(list_guard_installations(
            runtime_home,
            &connection.connection_internal_id,
            Some(&project.project_id),
        )?);
    }

    let mut audit = GuardAuditFacts::default();
    let mut all_required_phases_observed = !installations.is_empty();
    let mut current_hook_definition_executed = false;
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
            guard_observation_summary(runtime_home, &installation.project_id, installation)
                .map_err(|error| {
                    GuardCheckEvaluationUnavailable::project_store(ConnectionCommandError::from(
                        error,
                    ))
                })?;
        required_phases.extend(observation.required_phases.iter().cloned());
        observed_phases.extend(observation.observed_phases.iter().cloned());
        incompatible_event_ids.extend(observation.incompatible_event_ids.iter().cloned());
        current_hook_definition_executed |=
            binding_is_current && !observation.observed_phases.is_empty();
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
    let files_status = if installations.is_empty() {
        ConnectionCheckStatus::Failed
    } else {
        match audit.hook_path_safety.state {
            HookPathSafetyState::Verified | HookPathSafetyState::NotApplicable
                if audit.generated_config_verified() && configured_phase_gaps.is_empty() =>
            {
                ConnectionCheckStatus::Passed
            }
            HookPathSafetyState::Failed => ConnectionCheckStatus::Failed,
            HookPathSafetyState::NotRecorded | HookPathSafetyState::NotChecked => {
                ConnectionCheckStatus::Pending
            }
            HookPathSafetyState::Verified | HookPathSafetyState::NotApplicable => {
                ConnectionCheckStatus::Failed
            }
        }
    };
    let observation_status = if !incompatible_event_ids.is_empty() {
        ConnectionCheckStatus::Failed
    } else if all_required_phases_observed {
        ConnectionCheckStatus::Passed
    } else {
        ConnectionCheckStatus::Pending
    };
    let ambient_coverage_status =
        if files_status == ConnectionCheckStatus::Failed || !incompatible_event_ids.is_empty() {
            ConnectionCheckStatus::Failed
        } else if files_status == ConnectionCheckStatus::Passed && all_required_phases_observed {
            ConnectionCheckStatus::Passed
        } else {
            ConnectionCheckStatus::Pending
        };
    let hook_activation_state = HookActivationState::from_evidence(HookActivationEvidence {
        setup_changed_definition: false,
        host: None,
        current_definition_event_observed: current_hook_definition_executed,
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
            volicord_types::guard_manifest::guard_manifest_from_json(&installation.manifest_json)
                .ok()
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

    let mut guard_findings = guard_boundary_findings(
        connection,
        &audit,
        &installation_ids,
        files_status == ConnectionCheckStatus::Failed,
        &missing_required_phases,
        &incompatible_event_ids,
        prompt_capture_observed,
        &observation_revision_mismatch_installation_ids,
        evaluated_at.clone(),
        current_revision,
    )?;

    let verification_run = match selected_membership {
        Some(membership) => latest_guard_integration_verification_for_membership(
            runtime_home,
            &connection.connection_internal_id,
            &membership.project_internal_id,
            current_revision,
        ),
        None => latest_guard_integration_verification_for_connection(
            runtime_home,
            &connection.connection_internal_id,
            current_revision,
        ),
    }?;
    let verification_workflow = verification_run
        .as_ref()
        .map(current_guard_integration_verification_workflow)
        .transpose()?;
    let verification_observations = verification_run
        .as_ref()
        .map(|run| guard_probe_observations(runtime_home, &run.verification_id))
        .transpose()?
        .unwrap_or_default();
    let completed_proof = match selected_membership {
        Some(membership) => latest_completed_guard_integration_verification_for_membership(
            runtime_home,
            &connection.connection_internal_id,
            &membership.project_internal_id,
            current_revision,
        ),
        None => latest_completed_guard_integration_verification_for_connection(
            runtime_home,
            &connection.connection_internal_id,
            current_revision,
        ),
    }?;
    let completed_proof_observations = completed_proof
        .as_ref()
        .map(|run| guard_probe_observations(runtime_home, &run.verification_id))
        .transpose()?
        .unwrap_or_default();
    let verification_status = match verification_workflow {
        Some(IntegrationVerificationWorkflowState::Complete { .. }) => {
            ConnectionCheckStatus::Passed
        }
        Some(IntegrationVerificationWorkflowState::RepairRequired { .. }) => {
            ConnectionCheckStatus::Failed
        }
        Some(
            IntegrationVerificationWorkflowState::AwaitingProbe { .. }
            | IntegrationVerificationWorkflowState::AwaitingObservation { .. },
        )
        | None => ConnectionCheckStatus::Pending,
    };
    let latest_attempt_evidence = verification_run
        .as_ref()
        .zip(verification_workflow.as_ref())
        .map(|(run, workflow)| {
            CorrelatedGuardAttemptEvidence::try_new(run, workflow, &verification_observations)
                .map_err(ConnectionCommandError::runtime)
        })
        .transpose()?;
    let latest_proof_evidence = completed_proof
        .as_ref()
        .map(|run| CorrelatedGuardProof::try_new(run, &completed_proof_observations))
        .transpose()
        .map_err(ConnectionCommandError::runtime)?;
    let correlated_evidence =
        CorrelatedGuardVerificationEvidence::new(latest_attempt_evidence, latest_proof_evidence);
    let verification_evidence_at = correlated_evidence
        .decisive_evidence_timestamp(verification_status)
        .map_err(ConnectionCommandError::runtime)?;
    let verification_causes = match (verification_run.as_ref(), verification_workflow.as_ref()) {
        (
            Some(run),
            Some(IntegrationVerificationWorkflowState::RepairRequired {
                reason,
                retry_policy,
                ..
            }),
        ) => {
            let observation = selected_guard_observation(
                GuardIntegrationVerificationStatus::RepairRequired,
                Some(*reason),
                &verification_observations,
            );
            let finding = guard_verification_repair_finding(
                connection,
                run,
                *reason,
                observation.map(|value| value.stage),
                *retry_policy,
                observation.and_then(|value| value.observed_callable_name.clone()),
                evaluated_at.clone(),
            )?;
            let finding_id = finding.id().clone();
            guard_findings.current.push(finding);
            vec![finding_id]
        }
        _ => Vec::new(),
    };

    let mut ambient_coverage_causes = guard_findings.files.clone();
    ambient_coverage_causes.extend(guard_findings.observation.iter().cloned());
    ambient_coverage_causes.sort();
    ambient_coverage_causes.dedup();
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
            ConnectionCheckKind::AmbientHookCoverage,
            ambient_coverage_status,
            match ambient_coverage_status {
                ConnectionCheckStatus::Passed => "ambient_hook_coverage_passed",
                ConnectionCheckStatus::Pending => "ambient_hook_coverage_pending",
                ConnectionCheckStatus::Failed => "ambient_hook_coverage_failed",
                ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                    unreachable!("raw ambient Guard coverage does not block itself")
                }
            },
            match ambient_coverage_status {
                ConnectionCheckStatus::Passed => "A current managed Guard hook executed",
                ConnectionCheckStatus::Pending => {
                    "Current hook installation has not observed every configured ambient phase"
                }
                ConnectionCheckStatus::Failed => {
                    "Guard managed files or a current hook contract are incompatible"
                }
                ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                    unreachable!("raw ambient Guard coverage does not block itself")
                }
            },
            Some(typed_details(&AmbientGuardCoverageEvidence::new(
                current_hook_definition_executed,
                observation_status == ConnectionCheckStatus::Passed,
                installation_ids,
                affected_paths,
                artifact_issues,
                manifest_issues,
                audit.hook_path_safety.clone(),
                configured_phase_gaps,
                required_phases,
                observed_phases,
                missing_required_phases,
                incompatible_event_ids,
                AmbientPromptCaptureEvidence::new(
                    audit.prompt_capture_host_supported,
                    audit.prompt_capture_configured,
                    prompt_capture_observed,
                ),
                observed_at.clone(),
            ))?),
            observed_at.as_deref(),
        )?, ambient_coverage_causes)?,
        with_direct_causes(canonical_check_at(
            ConnectionCheckKind::CorrelatedGuardVerification,
            verification_status,
            match verification_status {
                ConnectionCheckStatus::Passed => "correlated_guard_verification_passed",
                ConnectionCheckStatus::Pending => "correlated_guard_verification_pending",
                ConnectionCheckStatus::Failed => "correlated_guard_verification_failed",
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
                    "The latest correlated in-chat Guard verification attempt requires typed repair"
                }
                ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable => {
                    unreachable!("raw Guard verification uses passed, pending, or failed")
                }
            },
            Some(typed_details(&correlated_evidence)?),
            verification_evidence_at,
        )?, verification_causes)?,
    ])?;
    Ok(ConnectionCheckEvaluation {
        checks,
        inline_findings: guard_findings.current,
        persisted_finding_seed_ids: Vec::new(),
    })
}
