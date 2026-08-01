use serde::Serialize;
use volicord_types::{
    connection_verification::{
        ConnectionCheck, ConnectionCheckStatus, ConnectionStatus, ConnectionVerificationReport,
        HookActivationState, IntegrationActivationState,
    },
    mcp_verification_evidence::{
        McpActiveVerificationEvidence, McpActiveVerificationSource, McpEvidenceCheckStatus,
    },
    values::UtcTimestamp,
};

use crate::connection_command::ConnectionCommandError;

pub(super) const fn connection_status_label(status: ConnectionStatus) -> &'static str {
    match status {
        ConnectionStatus::Complete => "ready",
        ConnectionStatus::ActionRequired => "action required",
        ConnectionStatus::Failed => "failed",
    }
}

pub(super) const fn integration_activation_label(
    state: IntegrationActivationState,
) -> &'static str {
    match state {
        IntegrationActivationState::Configured => "configured",
        IntegrationActivationState::HostReloadRequired => "host reload required",
        IntegrationActivationState::HookReviewRequiredOrUnknown => {
            "hook review required or unknown"
        }
        IntegrationActivationState::McpObservationRequired => "MCP observation required",
        IntegrationActivationState::GuardVerificationRequired => "Guard verification required",
        IntegrationActivationState::Complete => "complete",
        IntegrationActivationState::Failed => "failed",
    }
}

pub(super) const fn hook_activation_label(state: HookActivationState) -> &'static str {
    match state {
        HookActivationState::Unknown => "unknown",
        HookActivationState::ReviewRequiredBySetup => "review required by setup",
        HookActivationState::EffectiveByObservation => "effective by observation",
        HookActivationState::ManagedByPolicy => "managed by policy",
        HookActivationState::BypassedForInvocation => "bypassed for this invocation",
        HookActivationState::Disabled => "disabled",
    }
}

pub(super) const fn connection_check_status_label(status: ConnectionCheckStatus) -> &'static str {
    match status {
        ConnectionCheckStatus::Passed => "passed",
        ConnectionCheckStatus::Blocked => "blocked",
        ConnectionCheckStatus::Pending => "pending",
        ConnectionCheckStatus::Failed => "failed",
        ConnectionCheckStatus::NotApplicable => "not applicable",
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub(super) struct ConnectionCheckCounts {
    pub(super) passed: usize,
    pub(super) blocked: usize,
    pub(super) pending: usize,
    pub(super) failed: usize,
    pub(super) not_applicable: usize,
}

impl ConnectionCheckCounts {
    pub(super) fn from_checks(checks: &[ConnectionCheck]) -> Self {
        checks.iter().fold(Self::default(), |mut counts, check| {
            match check.status() {
                ConnectionCheckStatus::Passed => counts.passed += 1,
                ConnectionCheckStatus::Blocked => counts.blocked += 1,
                ConnectionCheckStatus::Pending => counts.pending += 1,
                ConnectionCheckStatus::Failed => counts.failed += 1,
                ConnectionCheckStatus::NotApplicable => counts.not_applicable += 1,
            }
            counts
        })
    }

    pub(super) fn from_report(report: &ConnectionVerificationReport) -> Self {
        Self::from_checks(report.checks())
    }

    pub(super) const fn concise_fields(self) -> [(&'static str, usize); 4] {
        [
            ("Passed", self.passed),
            ("Blocked", self.blocked),
            ("Pending", self.pending),
            ("Failed", self.failed),
        ]
    }

    pub(super) fn render_concise_inline(self) -> String {
        self.concise_fields()
            .into_iter()
            .map(|(label, count)| format!("{count} {}", label.to_ascii_lowercase()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub(super) fn render_verbose_inline(self) -> String {
        format!(
            "{}, {} not applicable",
            self.render_concise_inline(),
            self.not_applicable
        )
    }

    pub(super) fn render_nonzero(self) -> String {
        [
            (self.passed, "passed"),
            (self.blocked, "blocked"),
            (self.pending, "pending"),
            (self.failed, "failed"),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, label)| format!("{count} {label}"))
        .collect::<Vec<_>>()
        .join(", ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActiveVerificationState {
    NotRun,
    Passed,
    Failed,
}

impl ActiveVerificationState {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::NotRun => "not run",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StorageWriteabilityState {
    NotChecked,
    Passed,
    Failed,
}

impl StorageWriteabilityState {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::NotChecked => "not checked",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

pub(super) const fn active_verification_source_label(
    source: McpActiveVerificationSource,
) -> &'static str {
    match source {
        McpActiveVerificationSource::ConnectionVerify => "connection verify",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActiveVerificationSnapshotSummary {
    pub(super) state: ActiveVerificationState,
    pub(super) storage_writeability: StorageWriteabilityState,
    pub(super) observed_at: Option<UtcTimestamp>,
    pub(super) source: Option<McpActiveVerificationSource>,
}

impl ActiveVerificationSnapshotSummary {
    const fn not_run() -> Self {
        Self {
            state: ActiveVerificationState::NotRun,
            storage_writeability: StorageWriteabilityState::NotChecked,
            observed_at: None,
            source: None,
        }
    }

    fn from_evidence(evidence: &McpActiveVerificationEvidence) -> Self {
        let stores_passed = evidence.registry_write() == McpEvidenceCheckStatus::Passed
            && evidence
                .project_writes()
                .iter()
                .all(|project| project.state_write() == McpEvidenceCheckStatus::Passed);
        let active_passed = stores_passed
            && evidence
                .protocol_conformance()
                .iter()
                .all(|probe| probe.probe().status() == McpEvidenceCheckStatus::Passed)
            && evidence
                .host_compatibility()
                .iter()
                .all(|probe| probe.probe().status() == McpEvidenceCheckStatus::Passed);
        Self {
            state: if active_passed {
                ActiveVerificationState::Passed
            } else {
                ActiveVerificationState::Failed
            },
            storage_writeability: if stores_passed {
                StorageWriteabilityState::Passed
            } else {
                StorageWriteabilityState::Failed
            },
            observed_at: Some(evidence.observed_at().clone()),
            source: Some(evidence.source()),
        }
    }
}

pub(super) fn active_verification_snapshot(
    checks: &[ConnectionCheck],
) -> Result<Option<ActiveVerificationSnapshotSummary>, ConnectionCommandError> {
    let Some(details) = checks
        .iter()
        .find(|check| {
            check.id() == volicord_types::connection_verification::ConnectionCheckKind::McpServer
        })
        .and_then(ConnectionCheck::details)
        .map(|details| details.as_object())
    else {
        return Ok(None);
    };
    if !details.contains_key("preflight") && !details.contains_key("last_active_verification") {
        return Ok(None);
    }
    let Some(value) = details.get("last_active_verification") else {
        return Ok(Some(ActiveVerificationSnapshotSummary::not_run()));
    };
    if value.is_null() {
        return Ok(Some(ActiveVerificationSnapshotSummary::not_run()));
    }
    let evidence = serde_json::from_value::<McpActiveVerificationEvidence>(value.clone()).map_err(
        |error| {
            ConnectionCommandError::runtime(format!(
                "current MCP active-verification evidence is invalid: {error}"
            ))
        },
    )?;
    Ok(Some(ActiveVerificationSnapshotSummary::from_evidence(
        &evidence,
    )))
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use volicord_types::{
        connection_verification::{
            derive_integration_activation_state, ConnectionCheckDetails, ConnectionCheckKind,
            IntegrationActivationPlan,
        },
        diagnostics::DiagnosticFindingId,
        values::UtcTimestamp,
    };

    use super::*;

    fn check(status: ConnectionCheckStatus) -> ConnectionCheck {
        let causes = (status == ConnectionCheckStatus::Blocked)
            .then(|| DiagnosticFindingId::parse("finding.managed_config").unwrap())
            .into_iter()
            .collect();
        ConnectionCheck::try_new(
            ConnectionCheckKind::ManagedConfig,
            status,
            causes,
            (status != ConnectionCheckStatus::Passed)
                .then(|| "managed_config_diagnostic".to_owned()),
            "Managed configuration state",
            None,
            None,
        )
        .unwrap()
    }

    fn mcp_check(last_active_verification: Value) -> ConnectionCheck {
        let details = ConnectionCheckDetails::try_new(
            json!({
                "preflight": {},
                "last_active_verification": last_active_verification,
            })
            .as_object()
            .unwrap()
            .clone(),
        )
        .unwrap();
        ConnectionCheck::try_new(
            ConnectionCheckKind::McpServer,
            ConnectionCheckStatus::Passed,
            Vec::new(),
            None,
            "MCP verification state",
            Some(details),
            None,
        )
        .unwrap()
    }

    fn probe(status: &str) -> Value {
        json!({
            "status": status,
            "requested_revision": "2025-06-18",
            "negotiated_revision": "2025-06-18",
            "initialize": true,
            "initialized_notification": true,
            "schema_validation": true,
            "tools_list_observed": true,
            "tools_returned": 1,
            "required_tools_validated": true,
            "safe_read_only_tool": "volicord.list_projects",
            "safe_read_only_tool_completed": true,
            "shutdown_completed": true
        })
    }

    fn active_evidence(registry: &str, project: &str, protocol: &str, host: &str) -> Value {
        let mut protocol_probe = probe(protocol);
        protocol_probe["revision"] = json!("2025-06-18");
        let mut host_probe = probe(host);
        host_probe["profile"] = json!("codex-current");
        host_probe["fixture"] = json!("codex-current.json");
        json!({
            "registry_write": registry,
            "project_writes": [{
                "project_id": "project_internal_1",
                "state_write": project
            }],
            "protocol_conformance": [protocol_probe],
            "host_compatibility": [host_probe],
            "observed_at": "2026-07-31T00:00:00Z",
            "source": "connection_verify",
            "side_effects": [
                "rollback_only_registry_write_probe",
                "rollback_only_project_write_probe",
                "disposable_protocol_conformance",
                "disposable_host_compatibility"
            ]
        })
    }

    #[test]
    fn closed_connection_enums_have_explicit_human_labels() {
        assert_eq!(connection_status_label(ConnectionStatus::Complete), "ready");
        assert_eq!(
            connection_status_label(ConnectionStatus::ActionRequired),
            "action required"
        );
        assert_eq!(connection_status_label(ConnectionStatus::Failed), "failed");

        let activation = [
            (IntegrationActivationState::Configured, "configured"),
            (
                IntegrationActivationState::HostReloadRequired,
                "host reload required",
            ),
            (
                IntegrationActivationState::HookReviewRequiredOrUnknown,
                "hook review required or unknown",
            ),
            (
                IntegrationActivationState::McpObservationRequired,
                "MCP observation required",
            ),
            (
                IntegrationActivationState::GuardVerificationRequired,
                "Guard verification required",
            ),
            (IntegrationActivationState::Complete, "complete"),
            (IntegrationActivationState::Failed, "failed"),
        ];
        for (state, expected) in activation {
            assert_eq!(integration_activation_label(state), expected);
        }

        let hook = [
            (HookActivationState::Unknown, "unknown"),
            (
                HookActivationState::ReviewRequiredBySetup,
                "review required by setup",
            ),
            (
                HookActivationState::EffectiveByObservation,
                "effective by observation",
            ),
            (HookActivationState::ManagedByPolicy, "managed by policy"),
            (
                HookActivationState::BypassedForInvocation,
                "bypassed for this invocation",
            ),
            (HookActivationState::Disabled, "disabled"),
        ];
        for (state, expected) in hook {
            assert_eq!(hook_activation_label(state), expected);
        }

        let checks = [
            (ConnectionCheckStatus::Passed, "passed"),
            (ConnectionCheckStatus::Blocked, "blocked"),
            (ConnectionCheckStatus::Pending, "pending"),
            (ConnectionCheckStatus::Failed, "failed"),
            (ConnectionCheckStatus::NotApplicable, "not applicable"),
        ];
        for (status, expected) in checks {
            assert_eq!(connection_check_status_label(status), expected);
        }
    }

    #[test]
    fn one_count_projection_handles_each_status_and_canonical_order() {
        let individual = [
            (
                ConnectionCheckStatus::Passed,
                ConnectionCheckCounts {
                    passed: 1,
                    ..ConnectionCheckCounts::default()
                },
            ),
            (
                ConnectionCheckStatus::Blocked,
                ConnectionCheckCounts {
                    blocked: 1,
                    ..ConnectionCheckCounts::default()
                },
            ),
            (
                ConnectionCheckStatus::Pending,
                ConnectionCheckCounts {
                    pending: 1,
                    ..ConnectionCheckCounts::default()
                },
            ),
            (
                ConnectionCheckStatus::Failed,
                ConnectionCheckCounts {
                    failed: 1,
                    ..ConnectionCheckCounts::default()
                },
            ),
            (
                ConnectionCheckStatus::NotApplicable,
                ConnectionCheckCounts {
                    not_applicable: 1,
                    ..ConnectionCheckCounts::default()
                },
            ),
        ];
        for (status, expected) in individual {
            assert_eq!(
                ConnectionCheckCounts::from_checks(&[check(status)]),
                expected
            );
        }

        let checks = vec![
            check(ConnectionCheckStatus::Passed),
            check(ConnectionCheckStatus::Blocked),
            check(ConnectionCheckStatus::Pending),
            check(ConnectionCheckStatus::Failed),
            check(ConnectionCheckStatus::NotApplicable),
            check(ConnectionCheckStatus::Passed),
        ];
        let counts = ConnectionCheckCounts::from_checks(&checks);
        assert_eq!(
            counts,
            ConnectionCheckCounts {
                passed: 2,
                blocked: 1,
                pending: 1,
                failed: 1,
                not_applicable: 1,
            }
        );
        assert_eq!(
            counts.render_nonzero(),
            "2 passed, 1 blocked, 1 pending, 1 failed"
        );
        assert_eq!(
            counts.concise_fields(),
            [("Passed", 2), ("Blocked", 1), ("Pending", 1), ("Failed", 1),]
        );
        assert_eq!(
            counts.render_concise_inline(),
            "2 passed, 1 blocked, 1 pending, 1 failed"
        );
        assert_eq!(
            counts.render_verbose_inline(),
            "2 passed, 1 blocked, 1 pending, 1 failed, 1 not applicable"
        );

        let report_checks = vec![check(ConnectionCheckStatus::Passed)];
        let activation =
            derive_integration_activation_state(&report_checks, HookActivationState::Unknown);
        let report = ConnectionVerificationReport::try_new(
            UtcTimestamp::parse("2026-07-31T00:00:00Z").unwrap(),
            report_checks,
            IntegrationActivationPlan::empty(activation),
        )
        .unwrap();
        assert_eq!(
            ConnectionCheckCounts::from_report(&report),
            ConnectionCheckCounts {
                passed: 1,
                ..ConnectionCheckCounts::default()
            }
        );
    }

    #[test]
    fn active_verification_snapshot_is_typed_and_component_complete() {
        let not_run = active_verification_snapshot(&[mcp_check(Value::Null)])
            .unwrap()
            .unwrap();
        assert_eq!(not_run.state, ActiveVerificationState::NotRun);
        assert_eq!(
            not_run.storage_writeability,
            StorageWriteabilityState::NotChecked
        );
        assert_eq!(not_run.observed_at, None);
        assert_eq!(not_run.source, None);

        let cases = [
            (
                active_evidence("passed", "passed", "passed", "passed"),
                ActiveVerificationState::Passed,
                StorageWriteabilityState::Passed,
            ),
            (
                active_evidence("failed", "passed", "passed", "passed"),
                ActiveVerificationState::Failed,
                StorageWriteabilityState::Failed,
            ),
            (
                active_evidence("passed", "failed", "passed", "passed"),
                ActiveVerificationState::Failed,
                StorageWriteabilityState::Failed,
            ),
            (
                active_evidence("passed", "passed", "failed", "passed"),
                ActiveVerificationState::Failed,
                StorageWriteabilityState::Passed,
            ),
            (
                active_evidence("passed", "passed", "passed", "failed"),
                ActiveVerificationState::Failed,
                StorageWriteabilityState::Passed,
            ),
        ];
        for (evidence, state, storage_writeability) in cases {
            let summary = active_verification_snapshot(&[mcp_check(evidence)])
                .unwrap()
                .unwrap();
            assert_eq!(summary.state, state);
            assert_eq!(summary.storage_writeability, storage_writeability);
            assert_eq!(
                summary.observed_at,
                Some(UtcTimestamp::parse("2026-07-31T00:00:00Z").unwrap())
            );
            assert_eq!(
                summary.source,
                Some(McpActiveVerificationSource::ConnectionVerify)
            );
        }
        assert_eq!(
            active_verification_source_label(McpActiveVerificationSource::ConnectionVerify),
            "connection verify"
        );
    }

    #[test]
    fn malformed_active_verification_evidence_fails_without_unknown_fallback() {
        let error =
            active_verification_snapshot(&[mcp_check(json!({"corrupt": true}))]).unwrap_err();
        assert!(error
            .to_string()
            .contains("current MCP active-verification evidence is invalid"));
    }
}
