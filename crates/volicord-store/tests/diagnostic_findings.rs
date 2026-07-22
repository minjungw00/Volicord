use std::{collections::BTreeMap, error::Error};

use serde::Serialize;
use volicord_store::{
    diagnostic_findings::{
        active_current_findings_for_scope, bounded_diagnostic_graph_from_seeds,
        diagnostic_findings_by_ids, diagnostic_occurrences_for_runtime_session,
        diagnostic_root_cause_ids, insert_and_link_runtime_terminal_occurrence,
        insert_occurrence_finding, insert_occurrence_finding_graph,
        reportable_diagnostic_findings_by_ids, resolve_current_finding, upsert_current_snapshot,
        MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH,
    },
    operational_sessions::{
        connection_integration_revision, mcp_runtime_session, start_mcp_runtime_session,
        McpRuntimeSessionStart,
    },
    sqlite::{enable_foreign_keys, registry_db_path},
    StoreError,
};
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::{
    AgentConnectionId, AgentRuntimeSessionId, CurrentDiagnosticFinding, CurrentDiagnosticKey,
    CurrentDiagnosticSnapshot, CurrentDiagnosticStatus, DiagnosticAction, DiagnosticCause,
    DiagnosticCode, DiagnosticDomain, DiagnosticFactSource, DiagnosticFacts, DiagnosticFindingData,
    DiagnosticFindingId, DiagnosticScope, DiagnosticScopeKind, DiagnosticSeverity,
    DiagnosticSource, DiagnosticStage, DiagnosticSubject, DiagnosticSubjectIdentity,
    IntegrationRevision, McpRuntimeSessionSource, OccurrenceDiagnosticFinding, ProjectId,
    UtcTimestamp,
};

const OBSERVED: &str = "2026-07-21T01:02:03Z";

#[derive(Serialize)]
struct SafeFacts<'a> {
    expected: &'a str,
    actual: &'a str,
}

impl DiagnosticFactSource for SafeFacts<'_> {}

fn connection_revision(fixture: &CoreFixture) -> Result<IntegrationRevision, Box<dyn Error>> {
    let connection = volicord_store::agent_connections::agent_connection_record_read_only(
        fixture.runtime_home_path(),
        fixture.connection_id(),
    )?
    .ok_or("connection")?;
    Ok(connection_integration_revision(&connection)?)
}

fn finding_data(
    fixture: &CoreFixture,
    subject_reference: &str,
    actual: &str,
    observed_at: &str,
    causes: Vec<DiagnosticCause>,
) -> Result<DiagnosticFindingData, Box<dyn Error>> {
    Ok(DiagnosticFindingData::try_new(
        DiagnosticCode::parse("store.test_finding")?,
        DiagnosticDomain::parse("store")?,
        DiagnosticStage::parse("test")?,
        DiagnosticSeverity::Error,
        DiagnosticSource::parse("store_test")?,
        DiagnosticSubject::try_new("test_case", subject_reference)?,
        DiagnosticFacts::project(&SafeFacts {
            expected: "present",
            actual,
        })?,
        UtcTimestamp::parse(observed_at)?,
    )?
    .with_causes(causes)?
    .with_actions(vec![DiagnosticAction::try_new(
        DiagnosticCode::parse("action.store.repair")?,
        "Repair the bounded test condition",
    )?])?
    .with_connection_id(AgentConnectionId::new(fixture.connection_id()))?
    .with_project_id(ProjectId::new(fixture.project_id()))?
    .with_integration_revision(connection_revision(fixture)?))
}

fn occurrence(
    fixture: &CoreFixture,
    subject_reference: &str,
    causes: Vec<DiagnosticCause>,
) -> Result<OccurrenceDiagnosticFinding, Box<dyn Error>> {
    Ok(OccurrenceDiagnosticFinding::try_new(
        finding_data(fixture, subject_reference, "missing", OBSERVED, causes)?,
        None,
    )?)
}

fn runtime_occurrence(
    fixture: &CoreFixture,
    runtime_session_id: &str,
) -> Result<OccurrenceDiagnosticFinding, Box<dyn Error>> {
    let runtime =
        mcp_runtime_session(fixture.runtime_home_path(), runtime_session_id)?.ok_or("runtime")?;
    let data = DiagnosticFindingData::try_new(
        DiagnosticCode::parse("mcp.runtime_failed")?,
        DiagnosticDomain::parse("mcp")?,
        DiagnosticStage::parse("transport")?,
        DiagnosticSeverity::Error,
        DiagnosticSource::parse("mcp_stdio")?,
        DiagnosticSubject::try_new("runtime_session", runtime_session_id)?,
        DiagnosticFacts::empty(),
        UtcTimestamp::parse(OBSERVED)?,
    )?
    .with_connection_id(AgentConnectionId::new(runtime.connection_internal_id))?
    .with_project_id(ProjectId::new(fixture.project_id()))?
    .with_integration_revision(IntegrationRevision::parse(
        runtime.connection_integration_revision,
    )?);
    Ok(OccurrenceDiagnosticFinding::try_new(
        data,
        Some(AgentRuntimeSessionId::new(runtime_session_id)),
    )?)
}

fn current_key(fixture: &CoreFixture, subject_reference: &str) -> CurrentDiagnosticKey {
    CurrentDiagnosticKey::new(
        DiagnosticScope::try_new(DiagnosticScopeKind::Connection, fixture.connection_id())
            .expect("scope"),
        DiagnosticCode::parse("guard.managed_file.missing").expect("code"),
        DiagnosticDomain::parse("guard").expect("domain"),
        DiagnosticStage::parse("guard_files").expect("stage"),
        DiagnosticSource::parse("store_test").expect("source"),
        DiagnosticSubjectIdentity::from_canonical_bytes(
            format!("volicord.store-test.guard-managed-artifact:{subject_reference}").as_bytes(),
        ),
    )
}

fn current_finding(
    fixture: &CoreFixture,
    key: CurrentDiagnosticKey,
    display_reference: &str,
    actual: &str,
    observed_at: &str,
    causes: Vec<DiagnosticCause>,
) -> Result<CurrentDiagnosticFinding, Box<dyn Error>> {
    let snapshot = CurrentDiagnosticSnapshot::try_new(
        DiagnosticSubject::try_new("guard_managed_artifact", display_reference)?,
        DiagnosticSeverity::Error,
        DiagnosticFacts::project(&SafeFacts {
            expected: "present",
            actual,
        })?,
        UtcTimestamp::parse(observed_at)?,
    )?
    .with_causes(causes)?
    .with_actions(vec![DiagnosticAction::try_new(
        DiagnosticCode::parse("action.guard.repair")?,
        "Repair the managed guard artifact",
    )?])?
    .with_connection_id(AgentConnectionId::new(fixture.connection_id()))?
    .with_project_id(ProjectId::new(fixture.project_id()))?
    .with_integration_revision(connection_revision(fixture)?);
    Ok(CurrentDiagnosticFinding::try_new(key, snapshot)?)
}

#[test]
fn occurrence_graph_is_insert_only_without_runtime_heuristics() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-occurrence-round-trip")?;
    let first = occurrence(&fixture, "first", Vec::new())?;
    let repeated_observation = occurrence(&fixture, "first", Vec::new())?;
    assert_ne!(first.id(), repeated_observation.id());

    let child = occurrence(&fixture, "child", vec![DiagnosticCause::new(first.id())])?;
    insert_occurrence_finding_graph(
        fixture.runtime_home_path(),
        &[child.clone(), first.clone(), repeated_observation.clone()],
    )?;

    let stored = diagnostic_findings_by_ids(
        fixture.runtime_home_path(),
        &[child.id(), first.id(), repeated_observation.id()],
    )?;
    assert_eq!(stored.len(), 3);
    assert!(stored
        .iter()
        .all(|finding| finding.runtime_session_id().is_none()));
    assert!(insert_occurrence_finding(fixture.runtime_home_path(), &first).is_err());

    let conn = rusqlite::Connection::open(registry_db_path(fixture.runtime_home_path()))?;
    assert!(conn
        .execute(
            "UPDATE diagnostic_findings SET facts_json = '{}' WHERE finding_id = ?1",
            [first.id().as_str()],
        )
        .is_err());
    assert!(conn
        .execute(
            "UPDATE diagnostic_findings
                SET lifecycle = 'current_state',
                    current_identity_digest = ?2,
                    diagnostic_scope_kind = 'connection',
                    diagnostic_scope_identity = 'forbidden-upsert',
                    current_state_status = 'active'
              WHERE finding_id = ?1",
            [first.id().as_str(), &"a".repeat(64)],
        )
        .is_err());
    Ok(())
}

#[test]
fn runtime_terminal_occurrence_is_inserted_and_linked_atomically() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-terminal-occurrence")?;
    let runtime = start_mcp_runtime_session(
        fixture.runtime_home_path(),
        McpRuntimeSessionStart {
            connection_internal_id: fixture.connection_id().to_owned(),
            session_source: McpRuntimeSessionSource::ManagedHost,
            observed_host_executable_version: None,
            process_id: 42,
            process_started_at: "2026-07-21T01:02:00Z".to_owned(),
        },
    )?;
    let finding = runtime_occurrence(&fixture, &runtime.runtime_session_id)?;

    let current = current_finding(
        &fixture,
        current_key(&fixture, "runtime-terminal-current"),
        "runtime-terminal-current",
        "must_not_link",
        OBSERVED,
        Vec::new(),
    )?;
    upsert_current_snapshot(fixture.runtime_home_path(), &current)?;
    let conn = rusqlite::Connection::open(registry_db_path(fixture.runtime_home_path()))?;
    enable_foreign_keys(&conn)?;
    assert!(conn
        .execute(
            "UPDATE mcp_runtime_sessions SET terminal_finding_id = ?2
              WHERE runtime_session_id = ?1",
            [runtime.runtime_session_id.as_str(), current.id().as_str()],
        )
        .is_err());
    assert!(
        mcp_runtime_session(fixture.runtime_home_path(), &runtime.runtime_session_id)?
            .ok_or("unlinked runtime")?
            .terminal_finding_id
            .is_none()
    );

    insert_and_link_runtime_terminal_occurrence(fixture.runtime_home_path(), &finding)?;

    let linked = mcp_runtime_session(fixture.runtime_home_path(), &runtime.runtime_session_id)?
        .ok_or("linked runtime")?;
    assert_eq!(
        linked.terminal_finding_id.as_deref(),
        Some(finding.id().as_str())
    );
    assert_eq!(
        diagnostic_occurrences_for_runtime_session(
            fixture.runtime_home_path(),
            &runtime.runtime_session_id,
        )?,
        vec![finding]
    );
    Ok(())
}

#[test]
fn occurrence_graph_rejects_missing_causes_atomically() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-occurrence-atomic")?;
    let missing =
        DiagnosticFindingId::parse("finding.occurrence_00000000-0000-4000-8000-000000000001")?;
    let child = occurrence(
        &fixture,
        "missing-child",
        vec![DiagnosticCause::new(missing)],
    )?;
    assert!(insert_occurrence_finding(fixture.runtime_home_path(), &child).is_err());
    assert!(diagnostic_findings_by_ids(
        fixture.runtime_home_path(),
        std::slice::from_ref(&child.id()),
    )?
    .is_empty());
    Ok(())
}

#[test]
fn current_snapshot_replaces_only_snapshot_and_supports_resolution_reactivation(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-current-lifecycle")?;
    let first_cause = occurrence(&fixture, "first-cause", Vec::new())?;
    let second_cause = occurrence(&fixture, "second-cause", Vec::new())?;
    insert_occurrence_finding_graph(
        fixture.runtime_home_path(),
        &[first_cause.clone(), second_cause.clone()],
    )?;

    let key = current_key(&fixture, ".volicord/guard-a.json");
    let first = current_finding(
        &fixture,
        key.clone(),
        "guard-a-redacted-primary",
        "missing",
        "2026-07-21T01:02:03Z",
        vec![DiagnosticCause::new(first_cause.id())],
    )?;
    upsert_current_snapshot(fixture.runtime_home_path(), &first)?;
    let changed = current_finding(
        &fixture,
        key.clone(),
        "guard-a-redacted-updated",
        "content_mismatch",
        "2026-07-21T02:03:04Z",
        vec![DiagnosticCause::new(second_cause.id())],
    )?;
    assert_eq!(first.id(), changed.id());
    upsert_current_snapshot(fixture.runtime_home_path(), &changed)?;

    let graph = bounded_diagnostic_graph_from_seeds(
        fixture.runtime_home_path(),
        std::slice::from_ref(changed.id()),
        1,
    )?;
    let graph_ids = graph
        .entries
        .iter()
        .map(|entry| entry.finding.id().to_string())
        .collect::<Vec<_>>();
    assert!(graph_ids.contains(&changed.id().to_string()));
    assert!(graph_ids.contains(&second_cause.id().to_string()));
    assert!(!graph_ids.contains(&first_cause.id().to_string()));

    let missing =
        DiagnosticFindingId::parse("finding.occurrence_00000000-0000-4000-8000-000000000099")?;
    let rejected = current_finding(
        &fixture,
        key.clone(),
        "guard-a-invalid-refresh",
        "invalid_refresh",
        "2026-07-21T02:04:05Z",
        vec![DiagnosticCause::new(missing.clone())],
    )?;
    assert!(upsert_current_snapshot(fixture.runtime_home_path(), &rejected).is_err());
    let preserved = diagnostic_findings_by_ids(
        fixture.runtime_home_path(),
        std::slice::from_ref(changed.id()),
    )?;
    assert_eq!(preserved.len(), 1);
    assert_eq!(
        preserved[0].subject().reference(),
        "guard-a-redacted-updated"
    );
    assert_eq!(preserved[0].facts().data()["actual"], "content_mismatch");
    let preserved_graph = bounded_diagnostic_graph_from_seeds(
        fixture.runtime_home_path(),
        std::slice::from_ref(changed.id()),
        1,
    )?;
    let preserved_ids = preserved_graph
        .entries
        .iter()
        .map(|entry| entry.finding.id().clone())
        .collect::<Vec<_>>();
    assert!(preserved_ids.contains(changed.id()));
    assert!(preserved_ids.contains(&second_cause.id()));
    assert!(!preserved_ids.contains(&first_cause.id()));
    assert!(!preserved_ids.contains(&missing));

    let scope = key.scope().clone();
    assert_eq!(
        active_current_findings_for_scope(fixture.runtime_home_path(), &scope)?.len(),
        1
    );
    let resolved = resolve_current_finding(
        fixture.runtime_home_path(),
        &key,
        UtcTimestamp::parse("2026-07-21T03:04:05Z")?,
    )?;
    assert_eq!(
        resolved.snapshot().status(),
        CurrentDiagnosticStatus::Resolved
    );
    assert_eq!(
        resolved.snapshot().facts().data()["actual"],
        "content_mismatch"
    );
    assert_eq!(
        resolved.snapshot().subject().reference(),
        "guard-a-redacted-updated"
    );
    assert!(resolved.snapshot().actions().is_empty());
    assert!(resolved.snapshot().causes().is_empty());
    assert!(active_current_findings_for_scope(fixture.runtime_home_path(), &scope)?.is_empty());

    let explicit = diagnostic_findings_by_ids(
        fixture.runtime_home_path(),
        std::slice::from_ref(changed.id()),
    )?;
    assert_eq!(explicit[0].facts().data()["actual"], "content_mismatch");
    assert!(explicit[0].actions().is_empty());
    assert!(explicit[0].causes().is_empty());
    assert!(reportable_diagnostic_findings_by_ids(
        fixture.runtime_home_path(),
        std::slice::from_ref(changed.id()),
    )?
    .is_empty());

    let reactivated = current_finding(
        &fixture,
        key,
        "guard-a-redacted-reactivated",
        "missing_again",
        "2026-07-21T04:05:06Z",
        vec![DiagnosticCause::new(first_cause.id())],
    )?;
    upsert_current_snapshot(fixture.runtime_home_path(), &reactivated)?;
    let active = active_current_findings_for_scope(fixture.runtime_home_path(), &scope)?;
    assert_eq!(active.len(), 1);
    assert_eq!(
        active[0].snapshot().status(),
        CurrentDiagnosticStatus::Active
    );
    assert!(active[0].snapshot().resolved_at().is_none());
    assert_eq!(
        active[0].snapshot().facts().data()["actual"],
        "missing_again"
    );
    assert_eq!(
        active[0].snapshot().subject().reference(),
        "guard-a-redacted-reactivated"
    );
    assert_eq!(
        reportable_diagnostic_findings_by_ids(
            fixture.runtime_home_path(),
            std::slice::from_ref(reactivated.id()),
        )?
        .len(),
        1
    );
    Ok(())
}

#[test]
fn current_identity_columns_are_immutable_and_corrupt_digests_fail_reads(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-current-integrity")?;
    let current = current_finding(
        &fixture,
        current_key(&fixture, ".volicord/guard-a.json"),
        "guard-a-safe-display",
        "missing",
        OBSERVED,
        Vec::new(),
    )?;
    upsert_current_snapshot(fixture.runtime_home_path(), &current)?;

    let conn = rusqlite::Connection::open(registry_db_path(fixture.runtime_home_path()))?;
    enable_foreign_keys(&conn)?;
    assert!(conn
        .execute(
            "UPDATE diagnostic_findings SET stage = 'other_stage' WHERE finding_id = ?1",
            [current.id().as_str()],
        )
        .is_err());
    assert!(conn
        .execute(
            "UPDATE diagnostic_findings SET current_subject_identity = ?2 WHERE finding_id = ?1",
            [current.id().as_str(), &format!("sha256:{}", "0".repeat(64))],
        )
        .is_err());
    let persisted_subject_identity: String = conn.query_row(
        "SELECT current_subject_identity FROM diagnostic_findings WHERE finding_id = ?1",
        [current.id().as_str()],
        |row| row.get(0),
    )?;
    assert_eq!(
        persisted_subject_identity,
        current.key().subject_identity().as_str()
    );
    assert!(!persisted_subject_identity.contains(".volicord/guard-a.json"));

    let expected = current_finding(
        &fixture,
        current_key(&fixture, ".volicord/expected-identity.json"),
        "expected-safe-display",
        "expected",
        "2026-07-21T02:03:04Z",
        Vec::new(),
    )?;
    conn.execute(
        "INSERT INTO diagnostic_findings (
            finding_id, lifecycle, current_identity_digest, current_subject_identity,
            diagnostic_scope_kind, diagnostic_scope_identity,
            current_state_status, resolved_at,
            code, domain, stage, severity, source,
            subject_json, facts_json, actions_json, correlation_id,
            connection_internal_id, project_internal_id, runtime_session_id,
            integration_revision, observed_at
         )
         SELECT ?1, lifecycle, ?2, current_subject_identity,
                diagnostic_scope_kind, diagnostic_scope_identity,
                current_state_status, resolved_at,
                code, domain, stage, severity, source,
                subject_json, facts_json, actions_json, correlation_id,
                connection_internal_id, project_internal_id, runtime_session_id,
                integration_revision, observed_at
           FROM diagnostic_findings WHERE finding_id = ?3",
        [
            expected.id().as_str(),
            expected.identity_digest(),
            current.id().as_str(),
        ],
    )?;
    assert!(upsert_current_snapshot(fixture.runtime_home_path(), &expected).is_err());
    let mismatched_subject: String = conn.query_row(
        "SELECT subject_json FROM diagnostic_findings WHERE finding_id = ?1",
        [expected.id().as_str()],
        |row| row.get(0),
    )?;
    assert!(mismatched_subject.contains("guard-a-safe-display"));
    assert!(!mismatched_subject.contains("expected-safe-display"));

    let corrupt_digest = "f".repeat(64);
    let corrupt_id =
        DiagnosticFindingId::parse(format!("finding.current.sha256:{corrupt_digest}"))?;
    conn.execute(
        "INSERT INTO diagnostic_findings (
            finding_id, lifecycle, current_identity_digest, current_subject_identity,
            diagnostic_scope_kind, diagnostic_scope_identity,
            current_state_status, resolved_at,
            code, domain, stage, severity, source,
            subject_json, facts_json, actions_json, correlation_id,
            connection_internal_id, project_internal_id, runtime_session_id,
            integration_revision, observed_at
         )
         SELECT ?1, lifecycle, ?2, current_subject_identity,
                diagnostic_scope_kind, diagnostic_scope_identity,
                current_state_status, resolved_at,
                code, domain, stage, severity, source,
                subject_json, facts_json, actions_json, correlation_id,
                connection_internal_id, project_internal_id, runtime_session_id,
                integration_revision, observed_at
           FROM diagnostic_findings WHERE finding_id = ?3",
        [corrupt_id.as_str(), &corrupt_digest, current.id().as_str()],
    )?;
    let error = diagnostic_findings_by_ids(
        fixture.runtime_home_path(),
        std::slice::from_ref(&corrupt_id),
    )
    .expect_err("mismatched digest must fail closed");
    assert!(matches!(error, StoreError::CorruptOwnerStateValue { .. }));
    Ok(())
}

#[test]
fn cause_edges_and_bounded_traversal_keep_graph_integrity() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-cause-integrity")?;
    let root = occurrence(&fixture, "root", Vec::new())?;
    let middle = occurrence(&fixture, "middle", vec![DiagnosticCause::new(root.id())])?;
    let leaf = occurrence(&fixture, "leaf", vec![DiagnosticCause::new(middle.id())])?;
    insert_occurrence_finding_graph(
        fixture.runtime_home_path(),
        &[leaf.clone(), root.clone(), middle.clone()],
    )?;

    let bounded = bounded_diagnostic_graph_from_seeds(
        fixture.runtime_home_path(),
        std::slice::from_ref(&leaf.id()),
        1,
    )?;
    assert_eq!(bounded.entries.len(), 2);
    assert!(bounded.depth_limit_reached);
    let complete = bounded_diagnostic_graph_from_seeds(
        fixture.runtime_home_path(),
        std::slice::from_ref(&leaf.id()),
        2,
    )?;
    assert_eq!(complete.entries.len(), 3);
    assert!(!complete.depth_limit_reached);
    assert_eq!(
        diagnostic_root_cause_ids(
            fixture.runtime_home_path(),
            std::slice::from_ref(&leaf.id()),
            2,
        )?,
        vec![root.id()]
    );
    assert!(bounded_diagnostic_graph_from_seeds(
        fixture.runtime_home_path(),
        std::slice::from_ref(&leaf.id()),
        MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH + 1,
    )
    .is_err());

    let conn = rusqlite::Connection::open(registry_db_path(fixture.runtime_home_path()))?;
    enable_foreign_keys(&conn)?;
    assert!(conn
        .execute(
            "INSERT INTO diagnostic_cause_edges (finding_id, cause_finding_id) VALUES (?1, ?2)",
            [root.id().as_str(), leaf.id().as_str()],
        )
        .is_err());
    assert!(conn
        .execute(
            "INSERT INTO diagnostic_cause_edges (finding_id, cause_finding_id) VALUES (?1, ?2)",
            [
                root.id().as_str(),
                "finding.occurrence_00000000-0000-4000-8000-000000000099"
            ],
        )
        .is_err());
    Ok(())
}

#[derive(Serialize)]
struct OversizedFacts {
    values: BTreeMap<String, String>,
}

impl DiagnosticFactSource for OversizedFacts {}

#[test]
fn fact_bounds_fail_before_any_registry_write() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-fact-bounds")?;
    let values = (0..32)
        .map(|index| (format!("field_{index}"), "x".repeat(1_024)))
        .collect();
    assert!(DiagnosticFacts::project(&OversizedFacts { values }).is_err());
    let conn = rusqlite::Connection::open(registry_db_path(fixture.runtime_home_path()))?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM diagnostic_findings", [], |row| {
        row.get(0)
    })?;
    assert_eq!(count, 0);
    Ok(())
}
