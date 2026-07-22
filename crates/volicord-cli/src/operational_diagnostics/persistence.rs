//! Owner-scoped current-condition activation and resolution.

use std::collections::BTreeSet;

use volicord_store::diagnostic_findings::{
    active_current_findings_for_scope, resolve_current_finding, upsert_current_snapshot,
};
use volicord_types::{
    CurrentDiagnosticFinding, CurrentDiagnosticKey, DiagnosticFindingId, UtcTimestamp,
};

use crate::connection_command::ConnectionCommandError;

use super::definitions::{OperationalDiagnostic, RevisionDiagnostic};

/// A current-state owner reconciled as one complete observation set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurrentOperationalOwner {
    ManagedConfiguration,
    Trust,
    HostRevision,
    VerificationTool,
    Guard,
}

impl CurrentOperationalOwner {
    fn owns(self, diagnostic: OperationalDiagnostic) -> bool {
        match self {
            Self::ManagedConfiguration => {
                matches!(diagnostic, OperationalDiagnostic::ManagedConfig(_))
            }
            Self::Trust => matches!(diagnostic, OperationalDiagnostic::Trust(_)),
            Self::HostRevision => matches!(
                diagnostic,
                OperationalDiagnostic::Revision(RevisionDiagnostic::IntegrationStale)
            ),
            Self::VerificationTool => {
                matches!(diagnostic, OperationalDiagnostic::ToolVerification(_))
            }
            Self::Guard => matches!(
                diagnostic,
                OperationalDiagnostic::Guard(_)
                    | OperationalDiagnostic::Revision(RevisionDiagnostic::ObservationMismatch)
            ),
        }
    }
}

/// Reconciles an owner set even when its successful observation set is empty.
pub(crate) fn reconcile_current_findings_for_scope(
    runtime_home: &std::path::Path,
    scope: &volicord_types::DiagnosticScope,
    owners: &[CurrentOperationalOwner],
    findings: &[CurrentDiagnosticFinding],
    resolved_at: UtcTimestamp,
) -> Result<Vec<DiagnosticFindingId>, ConnectionCommandError> {
    if owners.is_empty() {
        return Err(ConnectionCommandError::runtime(
            "current-condition reconciliation requires at least one owner",
        ));
    }
    let mut current_ids = BTreeSet::new();
    for finding in findings {
        if finding.key().scope() != scope
            || !owners
                .iter()
                .any(|owner| key_diagnostic(finding.key()).is_some_and(|value| owner.owns(value)))
        {
            return Err(ConnectionCommandError::runtime(
                "current operational finding is outside the reconciled owner or scope",
            ));
        }
        upsert_current_snapshot(runtime_home, finding)?;
        current_ids.insert(finding.id().clone());
    }

    for existing in active_current_findings_for_scope(runtime_home, scope)? {
        let Some(diagnostic) = key_diagnostic(existing.key()) else {
            continue;
        };
        if owners.iter().any(|owner| owner.owns(diagnostic)) && !current_ids.contains(existing.id())
        {
            resolve_current_finding(runtime_home, existing.key(), resolved_at.clone())?;
        }
    }
    Ok(current_ids.into_iter().collect())
}

fn key_diagnostic(key: &CurrentDiagnosticKey) -> Option<OperationalDiagnostic> {
    OperationalDiagnostic::ALL.into_iter().find(|diagnostic| {
        let definition = diagnostic.definition();
        key.code().as_str() == definition.code()
            && key.domain().as_str() == definition.domain()
            && key.stage().as_str() == definition.stage()
            && key.source().as_str() == definition.source()
    })
}

#[cfg(test)]
mod tests {
    use volicord_store::{
        agent_connections::agent_connection_record_read_only,
        diagnostic_findings::{active_current_findings_for_scope, diagnostic_findings_by_ids},
    };
    use volicord_test_support::core_fixtures::CoreFixture;
    use volicord_types::{CurrentDiagnosticStatus, GuardHookPhase, UtcTimestamp};

    use super::*;
    use crate::operational_diagnostics::{
        current_connection_finding, GuardDiagnostic, GuardPhaseFacts, GuardPhaseSubject,
        OperationalCheckState,
    };

    #[test]
    fn repair_resolves_and_recurrence_reactivates_the_same_key() {
        let fixture = CoreFixture::new("operational-current-reconciliation").expect("fixture");
        let connection =
            agent_connection_record_read_only(fixture.runtime_home_path(), fixture.connection_id())
                .expect("connection lookup")
                .expect("connection");
        let subject =
            GuardPhaseSubject::for_connection(fixture.connection_id(), GuardHookPhase::PreTool)
                .expect("subject");
        let first = current_connection_finding(
            &connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::RequiredPhaseNotObserved),
            &subject,
            &GuardPhaseFacts::new(GuardHookPhase::PreTool),
            OperationalCheckState::Pending,
            UtcTimestamp::parse("2026-07-22T01:02:03Z").expect("time"),
        )
        .expect("finding");
        let id = first.id().clone();
        reconcile_current_findings_for_scope(
            fixture.runtime_home_path(),
            first.key().scope(),
            &[CurrentOperationalOwner::Guard],
            std::slice::from_ref(&first),
            UtcTimestamp::parse("2026-07-22T01:02:04Z").expect("time"),
        )
        .expect("activate");
        assert_eq!(
            active_current_findings_for_scope(fixture.runtime_home_path(), first.key().scope())
                .expect("active lookup")
                .len(),
            1
        );

        reconcile_current_findings_for_scope(
            fixture.runtime_home_path(),
            first.key().scope(),
            &[CurrentOperationalOwner::Guard],
            &[],
            UtcTimestamp::parse("2026-07-22T02:03:04Z").expect("time"),
        )
        .expect("resolve");
        assert!(active_current_findings_for_scope(
            fixture.runtime_home_path(),
            first.key().scope()
        )
        .expect("active lookup")
        .is_empty());
        let resolved =
            diagnostic_findings_by_ids(fixture.runtime_home_path(), std::slice::from_ref(&id))
                .expect("resolved exact read");
        assert_eq!(resolved.len(), 1);
        assert!(resolved[0].actions().is_empty());

        let recurrent = current_connection_finding(
            &connection,
            OperationalDiagnostic::Guard(GuardDiagnostic::RequiredPhaseNotObserved),
            &subject,
            &GuardPhaseFacts::new(GuardHookPhase::PreTool),
            OperationalCheckState::Pending,
            UtcTimestamp::parse("2026-07-22T03:04:05Z").expect("time"),
        )
        .expect("recurrent finding");
        assert_eq!(recurrent.id(), &id);
        reconcile_current_findings_for_scope(
            fixture.runtime_home_path(),
            recurrent.key().scope(),
            &[CurrentOperationalOwner::Guard],
            std::slice::from_ref(&recurrent),
            UtcTimestamp::parse("2026-07-22T03:04:06Z").expect("time"),
        )
        .expect("reactivate");
        let active =
            active_current_findings_for_scope(fixture.runtime_home_path(), recurrent.key().scope())
                .expect("active lookup");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id(), &id);
        assert_eq!(
            active[0].snapshot().status(),
            CurrentDiagnosticStatus::Active
        );
    }

    #[test]
    fn same_code_guard_phases_retain_distinct_typed_subjects() {
        let fixture = CoreFixture::new("operational-guard-phase-subjects").expect("fixture");
        let connection =
            agent_connection_record_read_only(fixture.runtime_home_path(), fixture.connection_id())
                .expect("connection lookup")
                .expect("connection");
        let diagnostic = OperationalDiagnostic::Guard(GuardDiagnostic::RequiredPhaseNotObserved);
        let observed_at = UtcTimestamp::parse("2026-07-22T01:02:03Z").expect("time");
        let pre_subject =
            GuardPhaseSubject::for_connection(fixture.connection_id(), GuardHookPhase::PreTool)
                .expect("pre-tool subject");
        let post_subject =
            GuardPhaseSubject::for_connection(fixture.connection_id(), GuardHookPhase::PostTool)
                .expect("post-tool subject");
        let pre = current_connection_finding(
            &connection,
            diagnostic,
            &pre_subject,
            &GuardPhaseFacts::new(GuardHookPhase::PreTool),
            OperationalCheckState::Pending,
            observed_at.clone(),
        )
        .expect("pre-tool finding");
        let post = current_connection_finding(
            &connection,
            diagnostic,
            &post_subject,
            &GuardPhaseFacts::new(GuardHookPhase::PostTool),
            OperationalCheckState::Pending,
            observed_at,
        )
        .expect("post-tool finding");

        assert_eq!(pre.key().code(), post.key().code());
        assert_ne!(pre.id(), post.id());
        assert_ne!(pre.key().subject(), post.key().subject());
    }
}
