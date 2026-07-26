//! Verification dependency graph, blocking, activation planning, and canonical checks.

use super::*;

pub(super) fn milestone_terminal_cause_ids(
    session: Option<&McpSessionMilestones>,
) -> Vec<DiagnosticFindingId> {
    session
        .and_then(|session| session.terminal_finding.clone())
        .into_iter()
        .collect()
}

pub(super) fn with_direct_causes(
    check: ConnectionCheck,
    cause_finding_ids: Vec<DiagnosticFindingId>,
) -> Result<ConnectionCheck, ConnectionCommandError> {
    if check.status() == ConnectionCheckStatus::Failed && !cause_finding_ids.is_empty() {
        check
            .with_cause_finding_ids(cause_finding_ids)
            .map_err(ConnectionCommandError::from)
    } else {
        Ok(check)
    }
}

pub(super) fn finalize_check_graph(
    checks: Vec<ConnectionCheck>,
    findings: &[DiagnosticFinding],
) -> Result<Vec<ConnectionCheck>, ConnectionCommandError> {
    let mut rooted = Vec::with_capacity(checks.len());
    for check in checks {
        if matches!(
            check.status(),
            ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
        ) && !check.cause_finding_ids().is_empty()
        {
            let mut roots = BTreeSet::new();
            for finding_id in check.cause_finding_ids() {
                if findings.iter().any(|finding| finding.id() == finding_id) {
                    roots.extend(
                        volicord_types::diagnostics::diagnostic_root_cause_ids(
                            findings,
                            std::slice::from_ref(finding_id),
                            MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
                        )
                        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
                    );
                } else {
                    roots.insert(finding_id.clone());
                }
            }
            rooted.push(check.with_cause_finding_ids(roots.into_iter().collect())?);
        } else {
            rooted.push(check);
        }
    }

    block_failed_dependencies(rooted)
}

pub(super) fn block_failed_dependencies(
    mut checks: Vec<ConnectionCheck>,
) -> Result<Vec<ConnectionCheck>, ConnectionCommandError> {
    for _ in 0..checks.len() {
        let states = checks
            .iter()
            .map(|check| {
                (
                    check.id(),
                    (check.status(), check.cause_finding_ids().to_vec()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut changed = false;
        for check in &mut checks {
            if matches!(check.status(), ConnectionCheckStatus::NotApplicable) {
                continue;
            }
            let causes = check
                .depends_on()
                .iter()
                .filter_map(|dependency| states.get(dependency))
                .filter(|(status, _)| {
                    matches!(
                        status,
                        ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
                    )
                })
                .flat_map(|(_, causes)| causes.iter().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            if causes.is_empty() {
                continue;
            }
            if check.status() == ConnectionCheckStatus::Blocked
                && check.cause_finding_ids() == causes
            {
                continue;
            }
            *check = check.clone().blocked_by(causes)?;
            changed = true;
        }
        if !changed {
            return Ok(checks);
        }
    }
    Err(ConnectionCommandError::runtime(
        "connection check dependency traversal exceeded the check bound",
    ))
}

pub(in crate::connection_command) fn activation_plan_for_checks(
    checks: &[ConnectionCheck],
) -> Result<IntegrationActivationPlan, ConnectionCommandError> {
    let hook_activation_state = checks
        .iter()
        .find(|check| check.id() == ConnectionCheckKind::HookSourceActivation)
        .and_then(ConnectionCheck::details)
        .and_then(|details| details.as_object().get("activation_state"))
        .and_then(Value::as_str)
        .and_then(HookActivationState::from_stable_str)
        .unwrap_or(HookActivationState::Unknown);
    activation_plan_for_checks_with_hook_state(checks, hook_activation_state)
}

pub(in crate::connection_command) fn activation_plan_for_checks_with_hook_state(
    checks: &[ConnectionCheck],
    hook_activation_state: HookActivationState,
) -> Result<IntegrationActivationPlan, ConnectionCommandError> {
    let state = derive_integration_activation_state(checks, hook_activation_state);
    let mut steps = BTreeMap::<ActivationStepId, (String, BTreeSet<DiagnosticFindingId>)>::new();
    let mut add = |id: ActivationStepId, instruction: &str, check: &ConnectionCheck| {
        let entry = steps
            .entry(id)
            .or_insert_with(|| (instruction.to_owned(), BTreeSet::new()));
        entry.1.extend(check.cause_finding_ids().iter().cloned());
    };
    for check in checks {
        match (check.id(), check.status()) {
            (ConnectionCheckKind::ManagedConfig, ConnectionCheckStatus::Failed) => {
                add(
                    ActivationStepId::RepairManagedConfiguration,
                    "Run the current Volicord setup command to repair or recreate the managed Codex configuration",
                    check,
                );
            }
            (
                ConnectionCheckKind::HostExecutable | ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Failed,
            ) => {
                add(
                    ActivationStepId::RepairManagedConfiguration,
                    "Reinstall the current Volicord build, regenerate the managed integration, and inspect the separate Codex PATH probe",
                    check,
                );
            }
            (
                ConnectionCheckKind::HostReload | ConnectionCheckKind::ProjectTrust,
                ConnectionCheckStatus::Pending,
            ) => {
                add(
                    ActivationStepId::ReloadCodex,
                    "Restart or reload Codex in this repository after completing any separately applicable project-trust step",
                    check,
                );
            }
            (ConnectionCheckKind::HookSourceActivation, ConnectionCheckStatus::Pending) => {
                add(
                    ActivationStepId::ReviewProjectHooks,
                    "Review the current project hook definition in the Codex hook UI or with `/hooks`; Volicord does not approve hook trust",
                    check,
                );
            }
            (
                ConnectionCheckKind::HookSourceActivation
                | ConnectionCheckKind::AmbientHookCoverage,
                ConnectionCheckStatus::Failed,
            ) => {
                add(
                    ActivationStepId::RepairHookContract,
                    "Inspect the current hook definition, explicit disabled state, and recorded contract facts before changing host-owned trust",
                    check,
                );
            }
            (
                ConnectionCheckKind::ManagedSessionHealth
                | ConnectionCheckKind::ManagedCapabilityProof,
                ConnectionCheckStatus::Pending,
            ) => {
                add(
                    ActivationStepId::RequestIntegrationVerification,
                    "Start a new managed Codex conversation and request `Run the Volicord integration verification.`",
                    check,
                );
            }
            (
                ConnectionCheckKind::ManagedSessionHealth
                | ConnectionCheckKind::ManagedCapabilityProof,
                ConnectionCheckStatus::Failed,
            ) => {
                add(
                    ActivationStepId::ReadConnectionStatus,
                    "Read current connection status and inspect the latest attempt and latest complete-proof runtime sessions",
                    check,
                );
            }
            (
                ConnectionCheckKind::AmbientHookCoverage
                | ConnectionCheckKind::CorrelatedGuardVerification,
                ConnectionCheckStatus::Pending,
            ) => {
                add(
                    ActivationStepId::RequestIntegrationVerification,
                    "Start a new managed Codex conversation and request `Run the Volicord integration verification.`; the agent follows the returned workflow state",
                    check,
                );
            }
            (ConnectionCheckKind::CorrelatedGuardVerification, ConnectionCheckStatus::Failed) => {
                let id =
                    guard_recovery_step(check).unwrap_or(ActivationStepId::ReadConnectionStatus);
                let instruction = match id {
                    ActivationStepId::ReloadCodex => {
                        "Reload Codex before starting a later correlated Guard verification attempt"
                    }
                    ActivationStepId::RequestIntegrationVerification => {
                        "Start a new Codex conversation and request the complete Volicord integration verification workflow"
                    }
                    ActivationStepId::RepairHookContract => {
                        "Inspect and repair the current hook contract before starting a later Guard verification attempt"
                    }
                    ActivationStepId::RepairManagedConfiguration => {
                        "Repair the current managed integration contract before starting a later Guard verification attempt"
                    }
                    ActivationStepId::ReadConnectionStatus => {
                        "Inspect the failed correlated integration-verification record and its current managed runtime session"
                    }
                    _ => {
                        "Inspect the failed correlated integration-verification record before retrying"
                    }
                };
                add(id, instruction, check);
            }
            _ => {}
        }
    }
    if steps.contains_key(&ActivationStepId::RepairManagedConfiguration) {
        steps.retain(|id, _| *id == ActivationStepId::RepairManagedConfiguration);
    }
    let mut required_steps = steps
        .into_iter()
        .map(|(id, (instruction, roots))| {
            ActivationStep::try_new(id, Vec::new(), instruction)?
                .with_root_finding_ids(roots.into_iter().collect())
                .map_err(ConnectionCommandError::from)
        })
        .collect::<Result<Vec<_>, _>>()?;

    if matches!(
        state,
        IntegrationActivationState::HostReloadRequired
            | IntegrationActivationState::HookReviewRequiredOrUnknown
    ) && !required_steps.iter().any(|step| {
        matches!(
            step.id(),
            ActivationStepId::RepairHookContract
                | ActivationStepId::RepairManagedConfiguration
                | ActivationStepId::ReadConnectionStatus
        )
    }) {
        required_steps = activation_journey_suffix(state, hook_activation_state)?;
    } else if matches!(
        state,
        IntegrationActivationState::McpObservationRequired
            | IntegrationActivationState::GuardVerificationRequired
    ) && !required_steps
        .iter()
        .any(|step| step.id() == ActivationStepId::RequestIntegrationVerification)
    {
        required_steps.push(ActivationStep::try_new(
            ActivationStepId::RequestIntegrationVerification,
            Vec::new(),
            "Start a new managed Codex conversation and request `Run the Volicord integration verification.`",
        )?);
    }

    let optional_diagnostics = if state == IntegrationActivationState::Complete {
        Vec::new()
    } else {
        vec![ActivationStep::try_new(
            ActivationStepId::RunOptionalActiveDiagnostics,
            Vec::new(),
            "Run `volicord connection verify` only when optional active diagnostics are needed",
        )?]
    };
    IntegrationActivationPlan::try_new(state, required_steps, optional_diagnostics)
        .map_err(ConnectionCommandError::from)
}

fn activation_journey_suffix(
    state: IntegrationActivationState,
    hook_activation_state: HookActivationState,
) -> Result<Vec<ActivationStep>, ConnectionCommandError> {
    let include_reload = state == IntegrationActivationState::HostReloadRequired;
    let include_review = include_reload
        || state == IntegrationActivationState::HookReviewRequiredOrUnknown
        || hook_activation_state == HookActivationState::ReviewRequiredBySetup;
    let mut steps = Vec::new();
    if include_reload {
        steps.push(ActivationStep::try_new(
            ActivationStepId::ReloadCodex,
            Vec::new(),
            "Restart or reload Codex in this repository.",
        )?);
    }
    if include_review {
        steps.push(ActivationStep::try_new(
            ActivationStepId::ReviewProjectHooks,
            include_reload
                .then_some(ActivationStepId::ReloadCodex)
                .into_iter()
                .collect(),
            "Review the current project hooks.",
        )?);
    }
    steps.push(ActivationStep::try_new(
        ActivationStepId::RequestIntegrationVerification,
        include_review
            .then_some(ActivationStepId::ReviewProjectHooks)
            .into_iter()
            .collect(),
        "Start a new Codex conversation and request: \"Run the Volicord integration verification.\"",
    )?);
    steps.push(ActivationStep::try_new(
        ActivationStepId::ReadConnectionStatus,
        vec![ActivationStepId::RequestIntegrationVerification],
        "After the agent finishes, read connection status.",
    )?);
    Ok(steps)
}

fn guard_recovery_step(check: &ConnectionCheck) -> Option<ActivationStepId> {
    let step = check
        .details()?
        .as_object()
        .get("latest_attempt")?
        .as_object()?
        .get("recovery_action")?
        .as_str()?;
    ActivationStepId::ALL
        .into_iter()
        .find(|id| id.as_str() == step)
}

pub(super) fn canonical_check(
    id: ConnectionCheckKind,
    status: ConnectionCheckStatus,
    code: &str,
    summary: &str,
    details: Option<Value>,
    observed_at: Option<&str>,
) -> Result<ConnectionCheck, ConnectionCommandError> {
    let details = details
        .map(compact_json_value)
        .map(|value| {
            let Value::Object(object) = value else {
                return Err(ConnectionCommandError::runtime(
                    "connection check details must be a JSON object",
                ));
            };
            ConnectionCheckDetails::try_new(object).map_err(ConnectionCommandError::from)
        })
        .transpose()?;
    let observed_at = observed_at
        .map(|value| {
            UtcTimestamp::from_str(value).map_err(|_| {
                ConnectionCommandError::runtime(format!(
                    "connection check observation time is invalid: {value}"
                ))
            })
        })
        .transpose()?;
    ConnectionCheck::try_new(
        id,
        status,
        Vec::new(),
        (status != ConnectionCheckStatus::Passed).then(|| code.to_owned()),
        summary,
        details,
        observed_at,
    )
    .map_err(ConnectionCommandError::from)
}

pub(super) fn typed_details<T: Serialize>(details: &T) -> Result<Value, ConnectionCommandError> {
    serde_json::to_value(details).map_err(|error| {
        ConnectionCommandError::runtime(format!(
            "typed connection-check details could not be serialized: {error}"
        ))
    })
}

pub(super) fn compact_json_value(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .filter_map(|(key, value)| {
                    (value != Value::Null).then(|| (key, compact_json_value(value)))
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(compact_json_value).collect()),
        other => other,
    }
}
