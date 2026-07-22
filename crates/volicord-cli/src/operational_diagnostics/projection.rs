//! Lifecycle projection and explicit current-report finding selection.

use std::collections::{BTreeMap, BTreeSet};

use volicord_store::{
    agent_connections::AgentConnectionRecord,
    diagnostic_findings::{
        reportable_diagnostic_findings_by_ids, stored_diagnostic_findings_by_ids,
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
        subject.subject_identity().clone(),
    );
    let revision = connection_integration_revision(connection)?;
    let snapshot = CurrentDiagnosticSnapshot::try_new(
        subject.safe_display_subject().clone(),
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

/// Provenance attached to one finding reference in a current evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticFindingProvenance {
    InlineEvaluation,
    PersistedStore,
}

/// Combines findings calculated by the current evaluation with explicit Store seeds.
#[derive(Debug, Clone, Default)]
pub(crate) struct DiagnosticFindingOverlay {
    inline_findings: BTreeMap<DiagnosticFindingId, DiagnosticFinding>,
    persisted_finding_seed_ids: BTreeSet<DiagnosticFindingId>,
    reference_provenance: BTreeMap<DiagnosticFindingId, DiagnosticFindingProvenance>,
}

impl DiagnosticFindingOverlay {
    pub(crate) fn insert_inline_current(&mut self, finding: &CurrentDiagnosticFinding) {
        self.insert_inline(finding.to_diagnostic_finding());
    }

    pub(crate) fn insert_inline(&mut self, finding: DiagnosticFinding) {
        let id = finding.id().clone();
        self.inline_findings.insert(id.clone(), finding);
        self.reference_provenance
            .insert(id, DiagnosticFindingProvenance::InlineEvaluation);
    }

    pub(crate) fn extend_inline_current<'a>(
        &mut self,
        findings: impl IntoIterator<Item = &'a CurrentDiagnosticFinding>,
    ) {
        for finding in findings {
            self.insert_inline_current(finding);
        }
    }

    pub(crate) fn insert_persisted_seed(&mut self, finding_id: DiagnosticFindingId) {
        self.persisted_finding_seed_ids.insert(finding_id.clone());
        self.reference_provenance
            .entry(finding_id)
            .or_insert(DiagnosticFindingProvenance::PersistedStore);
    }

    pub(crate) fn extend_persisted_seeds(
        &mut self,
        finding_ids: impl IntoIterator<Item = DiagnosticFindingId>,
    ) {
        for finding_id in finding_ids {
            self.insert_persisted_seed(finding_id);
        }
    }

    pub(crate) fn merge(&mut self, other: Self) {
        for finding in other.inline_findings.into_values() {
            self.insert_inline(finding);
        }
        self.extend_persisted_seeds(other.persisted_finding_seed_ids);
        for (finding_id, provenance) in other.reference_provenance {
            self.reference_provenance
                .entry(finding_id)
                .or_insert(provenance);
        }
    }

    pub(crate) fn inline_findings(&self) -> &BTreeMap<DiagnosticFindingId, DiagnosticFinding> {
        &self.inline_findings
    }

    pub(crate) fn persisted_finding_seed_ids(&self) -> &BTreeSet<DiagnosticFindingId> {
        &self.persisted_finding_seed_ids
    }

    pub(crate) fn provenance(
        &self,
        finding_id: &DiagnosticFindingId,
    ) -> Option<DiagnosticFindingProvenance> {
        self.reference_provenance.get(finding_id).copied()
    }
}

/// Resolves failed/blocked check references through the current evaluation overlay.
pub(crate) fn current_report_findings_with_overlay(
    runtime_home: &std::path::Path,
    connection: &AgentConnectionRecord,
    checks: &[volicord_types::ConnectionCheck],
    overlay: &DiagnosticFindingOverlay,
) -> Result<(Vec<DiagnosticFinding>, IntegrationRevision), ConnectionCommandError> {
    let selected = checks
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
    let mut provenance = overlay.reference_provenance.clone();
    for finding_id in selected {
        resolve_overlay_reference(
            runtime_home,
            connection,
            overlay,
            &mut provenance,
            &finding_id,
            0,
            &mut BTreeSet::new(),
            &mut findings,
        )?;
    }
    Ok((
        findings.into_values().collect(),
        connection_integration_revision(connection)?,
    ))
}

/// Loads a persisted-only current report through the same overlay boundary.
pub(crate) fn current_report_findings(
    runtime_home: &std::path::Path,
    connection: &AgentConnectionRecord,
    report: &ConnectionVerificationReport,
) -> Result<(Vec<DiagnosticFinding>, IntegrationRevision), ConnectionCommandError> {
    let mut overlay = DiagnosticFindingOverlay::default();
    overlay.extend_persisted_seeds(
        report
            .checks()
            .iter()
            .filter(|check| {
                matches!(
                    check.status(),
                    ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
                )
            })
            .flat_map(|check| check.cause_finding_ids().iter().cloned()),
    );
    current_report_findings_with_overlay(runtime_home, connection, report.checks(), &overlay)
}

#[allow(clippy::too_many_arguments)]
fn resolve_overlay_reference(
    runtime_home: &std::path::Path,
    connection: &AgentConnectionRecord,
    overlay: &DiagnosticFindingOverlay,
    provenance: &mut BTreeMap<DiagnosticFindingId, DiagnosticFindingProvenance>,
    finding_id: &DiagnosticFindingId,
    depth: usize,
    path: &mut BTreeSet<DiagnosticFindingId>,
    findings: &mut BTreeMap<DiagnosticFindingId, DiagnosticFinding>,
) -> Result<(), ConnectionCommandError> {
    if depth > MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH {
        return Err(ConnectionCommandError::runtime(format!(
            "diagnostic cause traversal exceeded depth {MAX_DIAGNOSTIC_CAUSE_TRAVERSAL_DEPTH}"
        )));
    }
    if findings.contains_key(finding_id) {
        return Ok(());
    }
    if !path.insert(finding_id.clone()) {
        return Err(ConnectionCommandError::runtime(format!(
            "diagnostic cause graph contains a cycle at {finding_id}"
        )));
    }

    let selected_provenance = provenance
        .get(finding_id)
        .copied()
        .or_else(|| overlay.provenance(finding_id))
        .ok_or_else(|| {
            ConnectionCommandError::runtime(format!(
                "diagnostic finding reference {finding_id} has no evaluation provenance"
            ))
        })?;
    let (finding, causes_are_persisted) =
        if let Some(inline) = overlay.inline_findings.get(finding_id) {
            (Some(inline.clone()), false)
        } else {
            match selected_provenance {
                DiagnosticFindingProvenance::InlineEvaluation => {
                    return Err(ConnectionCommandError::runtime(format!(
                    "inline diagnostic finding {finding_id} is absent from the current evaluation"
                )))
                }
                DiagnosticFindingProvenance::PersistedStore => {
                    let mut reportable = reportable_diagnostic_findings_by_ids(
                        runtime_home,
                        std::slice::from_ref(finding_id),
                    )?;
                    if let Some(stored) = reportable.pop() {
                        (Some(stored), true)
                    } else if stored_diagnostic_findings_by_ids(
                        runtime_home,
                        std::slice::from_ref(finding_id),
                    )?
                    .is_empty()
                    {
                        (
                            Some(missing_diagnostic_record_finding(
                                finding_id.clone(),
                                connection,
                                connection_integration_revision(connection)?,
                            )?),
                            true,
                        )
                    } else {
                        (None, true)
                    }
                }
            }
        };

    if let Some(finding) = finding {
        let causes = finding
            .causes()
            .iter()
            .map(|cause| cause.finding_id().clone())
            .collect::<Vec<_>>();
        findings.insert(finding_id.clone(), finding);
        if findings.len() > MAX_DIAGNOSTIC_FINDINGS {
            return Err(ConnectionCommandError::runtime(
                "diagnostic projection exceeded the shared finding bound",
            ));
        }
        for cause_id in causes {
            if overlay.inline_findings.contains_key(&cause_id) {
                provenance.insert(
                    cause_id.clone(),
                    DiagnosticFindingProvenance::InlineEvaluation,
                );
            } else if causes_are_persisted {
                provenance
                    .entry(cause_id.clone())
                    .or_insert(DiagnosticFindingProvenance::PersistedStore);
            } else if !provenance.contains_key(&cause_id) {
                return Err(ConnectionCommandError::runtime(format!(
                    "inline diagnostic finding {finding_id} has cause {cause_id} without explicit provenance"
                )));
            }
            resolve_overlay_reference(
                runtime_home,
                connection,
                overlay,
                provenance,
                &cause_id,
                depth + 1,
                path,
                findings,
            )?;
        }
    }
    path.remove(finding_id);
    Ok(())
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
