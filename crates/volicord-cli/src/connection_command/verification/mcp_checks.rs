//! MCP preflight and active-verification check projection.

use super::*;

pub(in crate::connection_command) fn mcp_server_check(
    preflight: &VerificationStep,
    handshake: &McpVerification,
) -> Result<ConnectionCheck, ConnectionCommandError> {
    let step = &handshake.step;
    let (status, code, summary) = if preflight.status == StepStatus::Failed {
        (
            ConnectionCheckStatus::Failed,
            preflight.code.as_str(),
            "Volicord CLI MCP preflight failed",
        )
    } else if step.status == StepStatus::Passed {
        (
            ConnectionCheckStatus::Passed,
            step.code.as_str(),
            "Volicord MCP server active verification passed",
        )
    } else if step.status == StepStatus::Failed {
        (
            ConnectionCheckStatus::Failed,
            step.code.as_str(),
            "Volicord MCP server active verification failed",
        )
    } else {
        (
            ConnectionCheckStatus::Failed,
            "mcp_server_active_verification_not_run",
            "Volicord MCP server active verification did not run",
        )
    };
    canonical_check(
        ConnectionCheckKind::McpServer,
        status,
        code,
        summary,
        Some(json!({
            "preflight": {
                "status": preflight.status.as_str(),
                "code": preflight.code,
                "diagnostic": preflight.details,
                "evidence": preflight.preflight_evidence,
                "finding_id": preflight.diagnostic.as_ref().map(|diagnostic| diagnostic.finding_id.as_str()),
                "diagnostic_code": preflight.diagnostic.as_ref().map(|diagnostic| diagnostic.code.as_str()),
                "failure_stage": preflight.failure.as_ref().map(|failure| failure.stage().as_str()),
            },
            "last_active_verification": handshake.active_evidence,
        })),
        None,
    )
}

pub(super) fn mcp_server_finding_ids(
    preflight: &VerificationStep,
    handshake: &McpVerification,
) -> Result<Vec<DiagnosticFindingId>, ConnectionCommandError> {
    let mut ids = BTreeMap::<String, DiagnosticFindingId>::new();
    let mut insert = |value: &str| -> Result<(), ConnectionCommandError> {
        let id = DiagnosticFindingId::parse(value.to_owned())
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
        ids.insert(value.to_owned(), id);
        Ok(())
    };
    if preflight.status == StepStatus::Failed {
        if let Some(diagnostic) = preflight.diagnostic.as_ref() {
            insert(&diagnostic.finding_id)?;
        }
    }
    if handshake.step.status == StepStatus::Failed {
        if let Some(exchange) = handshake.exchange.as_ref() {
            if let Some(diagnostic) = exchange.diagnostic.as_ref() {
                insert(&diagnostic.finding_id)?;
            }
            for probe in &exchange.conformance {
                if let Some(diagnostic) = probe.diagnostic.as_ref() {
                    insert(&diagnostic.finding_id)?;
                }
            }
            for probe in &exchange.host_compatibility {
                if let Some(diagnostic) = probe.diagnostic.as_ref() {
                    insert(&diagnostic.finding_id)?;
                }
            }
        }
    }
    Ok(ids.into_values().collect())
}
