//! Verification dependency graph, blocking, actions, and canonical checks.

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
                        volicord_types::diagnostic_root_cause_ids(
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

pub(super) fn actions_for_checks(
    checks: &[ConnectionCheck],
) -> Result<Vec<ConnectionAction>, ConnectionCommandError> {
    let mut actions =
        BTreeMap::<ConnectionActionKind, (String, BTreeSet<DiagnosticFindingId>)>::new();
    let mut add = |kind: ConnectionActionKind, instruction: &str, check: &ConnectionCheck| {
        let entry = actions
            .entry(kind)
            .or_insert_with(|| (instruction.to_owned(), BTreeSet::new()));
        entry.1.extend(check.cause_finding_ids().iter().cloned());
    };
    for check in checks {
        match (check.id(), check.status()) {
            (ConnectionCheckKind::ManagedConfig, ConnectionCheckStatus::Failed) => {
                add(
                    ConnectionActionKind::RepairManagedConfiguration,
                    "Run the current Volicord setup command to repair or recreate the managed Codex configuration",
                    check,
                );
            }
            (
                ConnectionCheckKind::HostExecutable | ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Failed,
            ) => {
                add(
                    ConnectionActionKind::ReinstallCurrentBuild,
                    "Reinstall the current Volicord build, regenerate the managed integration, and inspect the separate Codex PATH probe",
                    check,
                );
            }
            (
                ConnectionCheckKind::HostReload | ConnectionCheckKind::ProjectTrust,
                ConnectionCheckStatus::Pending,
            ) => {
                add(
                    ConnectionActionKind::ReloadHost,
                    "Restart or reload Codex in this repository after completing any separately applicable project-trust step",
                    check,
                );
            }
            (ConnectionCheckKind::HookSourceActivation, ConnectionCheckStatus::Pending) => {
                add(
                    ConnectionActionKind::ReviewHooks,
                    "Review the current project hook definition in the Codex hook UI or with `/hooks`; Volicord does not approve hook trust",
                    check,
                );
            }
            (
                ConnectionCheckKind::HookSourceActivation | ConnectionCheckKind::GuardHookExecution,
                ConnectionCheckStatus::Failed,
            ) => {
                add(
                    ConnectionActionKind::InspectHookContract,
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
                    ConnectionActionKind::RunMcpVerification,
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
                    ConnectionActionKind::InspectRuntimeSession,
                    "Inspect the latest attempt and latest complete-proof runtime sessions, including actual MCP peer and PATH-probe facts",
                    check,
                );
            }
            (
                ConnectionCheckKind::GuardHookExecution | ConnectionCheckKind::GuardVerification,
                ConnectionCheckStatus::Pending,
            ) => {
                add(
                    ConnectionActionKind::RunGuardProbe,
                    &format!(
                        "Call `{}`, then follow its tagged workflow: `{}` uses `{}`, `{}` uses `{}`, `{}` uses `{}` after repair or expiry, and `{}` calls no verification tool",
                        AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
                        IntegrationVerificationWorkflowState::AWAITING_PROBE_KIND,
                        AgentToolId::GUARD_PROBE.wire_name(),
                        IntegrationVerificationWorkflowState::AWAITING_HOOK_COMPLETION_KIND,
                        AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
                        IntegrationVerificationWorkflowState::RESTART_REQUIRED_KIND,
                        AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
                        IntegrationVerificationWorkflowState::COMPLETE_KIND,
                    ),
                    check,
                );
            }
            (ConnectionCheckKind::GuardVerification, ConnectionCheckStatus::Failed) => {
                add(
                    ConnectionActionKind::InspectRuntimeSession,
                    "Inspect the failed correlated integration-verification record and its current managed runtime session",
                    check,
                );
            }
            _ => {}
        }
    }
    if actions.contains_key(&ConnectionActionKind::RepairManagedConfiguration) {
        actions.retain(|kind, _| {
            matches!(
                kind,
                ConnectionActionKind::RepairManagedConfiguration
                    | ConnectionActionKind::ReinstallCurrentBuild
            )
        });
    }
    actions
        .into_iter()
        .map(|(id, (instruction, roots))| {
            ConnectionAction::try_new(id, instruction)?
                .with_root_finding_ids(roots.into_iter().collect())
                .map_err(ConnectionCommandError::from)
        })
        .collect()
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
