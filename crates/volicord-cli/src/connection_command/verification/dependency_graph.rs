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
            if matches!(
                check.status(),
                ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable
            ) {
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
    let mut actions = BTreeMap::<ConnectionActionKind, &str>::new();
    for check in checks {
        match (check.id(), check.status()) {
            (ConnectionCheckKind::ManagedConfig, ConnectionCheckStatus::Failed) => {
                actions.insert(
                    ConnectionActionKind::RepairManagedConfig,
                    "Repair or recreate the Volicord-managed Codex MCP entry",
                );
            }
            (ConnectionCheckKind::HostExecutable, ConnectionCheckStatus::Failed) => {
                actions.insert(
                    ConnectionActionKind::InstallOrRepairCodex,
                    "Install or repair Codex so `codex --version` succeeds on PATH",
                );
            }
            (ConnectionCheckKind::McpServer, ConnectionCheckStatus::Failed) => {
                actions.insert(
                    ConnectionActionKind::RepairMcpServer,
                    "Repair the Volicord MCP configuration or storage error and verify again",
                );
            }
            (ConnectionCheckKind::ProjectTrust, ConnectionCheckStatus::Pending) => {
                actions.insert(
                    ConnectionActionKind::HostTrustRequired,
                    "Trust the project in Codex, then restart or reload Codex",
                );
            }
            (
                ConnectionCheckKind::HostSession
                | ConnectionCheckKind::RequiredTools
                | ConnectionCheckKind::ToolRoundTrip
                | ConnectionCheckKind::GuardObservation,
                ConnectionCheckStatus::Pending,
            ) => {
                actions.insert(
                    ConnectionActionKind::ObserveCodex,
                    "Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool so actual Codex connection and Guard activity can be observed",
                );
            }
            (
                ConnectionCheckKind::HostSession
                | ConnectionCheckKind::RequiredTools
                | ConnectionCheckKind::ToolRoundTrip
                | ConnectionCheckKind::GuardObservation,
                ConnectionCheckStatus::Failed,
            ) => {
                actions.insert(
                    ConnectionActionKind::InspectCodexProtocol,
                    "Inspect the recorded Codex protocol failure, repair the incompatible configuration or behavior, then verify again",
                );
            }
            (ConnectionCheckKind::GuardFiles, ConnectionCheckStatus::Failed) => {
                actions.insert(
                    ConnectionActionKind::RepairGuard,
                    "Repair the Volicord Guard integration and verify the connection again",
                );
            }
            _ => {}
        }
    }
    if actions.contains_key(&ConnectionActionKind::RepairManagedConfig) {
        actions.remove(&ConnectionActionKind::ObserveCodex);
        actions.remove(&ConnectionActionKind::InspectCodexProtocol);
    }
    actions
        .into_iter()
        .map(|(id, instruction)| {
            ConnectionAction::try_new(id, instruction).map_err(ConnectionCommandError::from)
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
