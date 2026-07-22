//! Lifecycle projection and explicit current-report finding selection.

use std::collections::{BTreeMap, BTreeSet};

use volicord_store::{
    agent_connections::AgentConnectionRecord,
    diagnostic_findings::{
        bounded_diagnostic_graph_from_seeds, diagnostic_findings_by_ids,
        reportable_diagnostic_findings_by_ids,
    },
    operational_sessions::connection_integration_revision,
};
use volicord_types::{
    AgentConnectionId, ConnectionCheckStatus, ConnectionVerificationReport,
    CurrentDiagnosticFinding, CurrentDiagnosticKey, CurrentDiagnosticSnapshot, DiagnosticAction,
    DiagnosticCode, DiagnosticDomain, DiagnosticFactSource, DiagnosticFacts, DiagnosticFinding,
    DiagnosticFindingData, DiagnosticFindingId, DiagnosticScopeKind, DiagnosticSource,
    DiagnosticStage, DiagnosticSubject, IntegrationRevision, OccurrenceDiagnosticFinding,
    UtcTimestamp, MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH, MAX_DIAGNOSTIC_FINDINGS,
};

use crate::connection_command::ConnectionCommandError;

use super::{
    actions::{actions_for, OperationalCheckState},
    definitions::OperationalDiagnostic,
    facts::{project_facts, TypedOperationalFacts},
    subjects::OperationalSubject,
};

/// Projects one state-like owner observation into a current finding.
pub(crate) fn current_connection_finding<S, F>(
    connection: &AgentConnectionRecord,
    diagnostic: OperationalDiagnostic,
    subject: &S,
    facts: &F,
    check_state: OperationalCheckState,
    observed_at: UtcTimestamp,
) -> Result<CurrentDiagnosticFinding, ConnectionCommandError>
where
    S: OperationalSubject,
    F: TypedOperationalFacts,
{
    if subject.scope().kind() != DiagnosticScopeKind::Connection
        || subject.scope().identity() != connection.connection_internal_id
    {
        return Err(ConnectionCommandError::runtime(
            "operational subject scope does not match the selected Agent Connection",
        ));
    }
    let definition = diagnostic.definition();
    let key = CurrentDiagnosticKey::new(
        subject.scope().clone(),
        DiagnosticCode::parse(definition.code())
            .expect("operational definition code is statically valid"),
        DiagnosticDomain::parse(definition.domain())
            .expect("operational definition domain is statically valid"),
        DiagnosticStage::parse(definition.stage())
            .expect("operational definition stage is statically valid"),
        DiagnosticSource::parse(definition.source())
            .expect("operational definition source is statically valid"),
        subject.safe_display_subject().clone(),
    );
    let revision = connection_integration_revision(connection)?;
    let snapshot = CurrentDiagnosticSnapshot::try_new(
        definition.severity(),
        project_facts(definition, facts)
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        observed_at,
    )
    .and_then(|snapshot| snapshot.with_actions(actions_for(definition, facts, check_state)))
    .and_then(|snapshot| {
        snapshot.with_connection_id(AgentConnectionId::new(
            connection.connection_internal_id.clone(),
        ))
    })
    .map(|snapshot| snapshot.with_integration_revision(revision))
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    CurrentDiagnosticFinding::try_new(key, snapshot)
        .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}

/// Projects one event-like owner observation with a generated occurrence ID.
pub(crate) fn occurrence_finding<S, F>(
    diagnostic: OperationalDiagnostic,
    subject: &S,
    facts: &F,
    check_state: OperationalCheckState,
    observed_at: UtcTimestamp,
) -> Result<DiagnosticFinding, volicord_types::DiagnosticError>
where
    S: OperationalSubject,
    F: TypedOperationalFacts,
{
    let definition = diagnostic.definition();
    let data = DiagnosticFindingData::try_new(
        DiagnosticCode::parse(definition.code())?,
        DiagnosticDomain::parse(definition.domain())?,
        DiagnosticStage::parse(definition.stage())?,
        definition.severity(),
        DiagnosticSource::parse(definition.source())?,
        subject.safe_display_subject().clone(),
        project_facts(definition, facts)?,
        observed_at,
    )?
    .with_actions(actions_for(definition, facts, check_state))?;
    OccurrenceDiagnosticFinding::try_new(data, None).map(|finding| finding.to_diagnostic_finding())
}

/// Loads only IDs explicitly selected by failed/blocked checks and their bounded causes.
pub(crate) fn current_report_findings(
    runtime_home: &std::path::Path,
    connection: &AgentConnectionRecord,
    report: &ConnectionVerificationReport,
) -> Result<(Vec<DiagnosticFinding>, IntegrationRevision), ConnectionCommandError> {
    let selected = report
        .checks()
        .iter()
        .filter(|check| {
            matches!(
                check.status(),
                ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
            )
        })
        .flat_map(|check| check.cause_finding_ids().iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut findings = BTreeMap::new();
    for finding_id in selected {
        let reportable =
            reportable_diagnostic_findings_by_ids(runtime_home, std::slice::from_ref(&finding_id))?;
        if reportable.is_empty() {
            if diagnostic_findings_by_ids(runtime_home, std::slice::from_ref(&finding_id))?
                .is_empty()
            {
                findings.insert(
                    finding_id.clone(),
                    missing_diagnostic_record_finding(
                        finding_id,
                        connection,
                        connection_integration_revision(connection)?,
                    )?,
                );
            }
            continue;
        }

        let chain = bounded_diagnostic_graph_from_seeds(
            runtime_home,
            std::slice::from_ref(&finding_id),
            MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH,
        )?;
        let chain_ids = chain
            .entries
            .iter()
            .map(|entry| entry.finding.id().clone())
            .collect::<Vec<_>>();
        let reportable_chain = reportable_diagnostic_findings_by_ids(runtime_home, &chain_ids)?
            .into_iter()
            .map(|finding| (finding.id().clone(), finding))
            .collect::<BTreeMap<_, _>>();
        if reportable_chain.len() != chain.entries.len() {
            return Err(ConnectionCommandError::runtime(
                "current diagnostic cause graph references a resolved current condition",
            ));
        }
        findings.extend(reportable_chain);
        if findings.len() > MAX_DIAGNOSTIC_FINDINGS {
            return Err(ConnectionCommandError::runtime(
                "diagnostic projection exceeded the shared finding bound",
            ));
        }
    }
    Ok((
        findings.into_values().collect(),
        connection_integration_revision(connection)?,
    ))
}

#[derive(serde::Serialize)]
struct MissingDiagnosticRecordFacts<'a> {
    summary: &'static str,
    observation_state: &'static str,
    finding_id: &'a str,
    expected: &'static str,
    actual: &'static str,
}

impl DiagnosticFactSource for MissingDiagnosticRecordFacts<'_> {}

fn missing_diagnostic_record_finding(
    finding_id: DiagnosticFindingId,
    connection: &AgentConnectionRecord,
    integration_revision: IntegrationRevision,
) -> Result<DiagnosticFinding, ConnectionCommandError> {
    let facts = DiagnosticFacts::project(&MissingDiagnosticRecordFacts {
        summary: "the verification check references a diagnostic finding that is not persisted",
        observation_state: "absent",
        finding_id: finding_id.as_str(),
        expected: "persisted diagnostic finding",
        actual: "absent",
    })
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?;
    DiagnosticFinding::try_new(
        finding_id,
        DiagnosticCode::parse("diagnostics.finding_record_missing")
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        DiagnosticDomain::parse("diagnostics")
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        DiagnosticStage::parse("projection")
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        volicord_types::DiagnosticSeverity::Error,
        DiagnosticSource::parse("connection_diagnostic_projection")
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        DiagnosticSubject::try_new("finding", "persisted_record")
            .map_err(|error| ConnectionCommandError::runtime(error.to_string()))?,
        facts,
        super::current_timestamp(),
    )
    .and_then(|finding| {
        finding.with_actions(vec![DiagnosticAction::try_new(
            DiagnosticCode::parse("action.diagnostics.rebuild_current_observations")?,
            "Run connection verification to rebuild current diagnostic observations",
        )?])
    })
    .and_then(|finding| {
        finding.with_connection_id(AgentConnectionId::new(
            connection.connection_internal_id.clone(),
        ))
    })
    .map(|finding| finding.with_integration_revision(integration_revision))
    .map_err(|error| ConnectionCommandError::runtime(error.to_string()))
}
