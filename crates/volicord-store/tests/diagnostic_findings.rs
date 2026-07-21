use std::{collections::BTreeMap, error::Error};

use serde::Serialize;
use volicord_store::{
    diagnostic_findings::{
        current_diagnostic_findings_for_connection, diagnostic_cause_chain, diagnostic_finding,
        diagnostic_findings_for_runtime_session, insert_diagnostic_finding,
        insert_diagnostic_finding_graph, link_mcp_runtime_session_terminal_finding,
        MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH,
    },
    operational_sessions::{
        connection_integration_revision, mcp_runtime_session, start_mcp_runtime_session,
        McpRuntimeSessionStart,
    },
    sqlite::{enable_foreign_keys, registry_db_path},
};
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::{
    AgentConnectionId, AgentRuntimeSessionId, DiagnosticCause, DiagnosticCode, DiagnosticDomain,
    DiagnosticFactSource, DiagnosticFacts, DiagnosticFinding, DiagnosticFindingId,
    DiagnosticSeverity, DiagnosticSource, DiagnosticStage, DiagnosticSubject, IntegrationRevision,
    McpRuntimeSessionSource, UtcTimestamp,
};

const OBSERVED: &str = "2026-07-21T01:02:03Z";

#[derive(Serialize)]
struct SafeFacts {
    expected: String,
    actual: String,
}

impl DiagnosticFactSource for SafeFacts {}

fn finding(
    fixture: &CoreFixture,
    id: &str,
    causes: &[&str],
) -> Result<DiagnosticFinding, Box<dyn Error>> {
    let connection = volicord_store::agent_connections::agent_connection_record_read_only(
        fixture.runtime_home_path(),
        fixture.connection_id(),
    )?
    .ok_or("connection")?;
    let revision = connection_integration_revision(&connection)?;
    Ok(DiagnosticFinding::try_new(
        DiagnosticFindingId::parse(id)?,
        DiagnosticCode::parse("store.test_finding")?,
        DiagnosticDomain::parse("store")?,
        DiagnosticStage::parse("test")?,
        DiagnosticSeverity::Error,
        DiagnosticSource::parse("store_test")?,
        DiagnosticSubject::try_new("test_case", id)?,
        DiagnosticFacts::project(&SafeFacts {
            expected: "present".to_owned(),
            actual: "missing".to_owned(),
        })?,
        UtcTimestamp::parse(OBSERVED)?,
    )?
    .with_causes(
        causes
            .iter()
            .map(|cause| DiagnosticFindingId::parse(*cause).map(DiagnosticCause::new))
            .collect::<Result<Vec<_>, _>>()?,
    )?
    .with_connection_id(AgentConnectionId::new(fixture.connection_id()))?
    .with_project_id(volicord_types::ProjectId::new(fixture.project_id()))?
    .with_integration_revision(revision))
}

fn runtime_finding(
    fixture: &CoreFixture,
    runtime_session_id: &str,
) -> Result<DiagnosticFinding, Box<dyn Error>> {
    let runtime =
        mcp_runtime_session(fixture.runtime_home_path(), runtime_session_id)?.ok_or("runtime")?;
    Ok(DiagnosticFinding::try_new(
        DiagnosticFindingId::parse("finding.runtime")?,
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
    .with_project_id(volicord_types::ProjectId::new(fixture.project_id()))?
    .with_runtime_session_id(AgentRuntimeSessionId::new(runtime_session_id))?
    .with_integration_revision(IntegrationRevision::parse(
        runtime.connection_integration_revision,
    )?))
}

#[test]
fn finding_graph_round_trips_and_queries_by_current_coordinates() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-finding-round-trip")?;
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
    let root = finding(&fixture, "finding.root", &[])?;
    let child = finding(&fixture, "finding.child", &["finding.root"])?;
    let runtime_finding = runtime_finding(&fixture, &runtime.runtime_session_id)?;

    let inserted = insert_diagnostic_finding_graph(
        fixture.runtime_home_path(),
        &[child.clone(), root.clone(), runtime_finding.clone()],
    )?;
    assert_eq!(
        inserted,
        vec![child.clone(), root.clone(), runtime_finding.clone()]
    );
    assert_eq!(
        diagnostic_finding(fixture.runtime_home_path(), child.id())?,
        Some(child)
    );
    assert_eq!(
        diagnostic_findings_for_runtime_session(
            fixture.runtime_home_path(),
            &runtime.runtime_session_id,
        )?,
        vec![runtime_finding]
    );
    assert_eq!(
        current_diagnostic_findings_for_connection(
            fixture.runtime_home_path(),
            fixture.connection_id(),
        )?
        .len(),
        3
    );
    Ok(())
}

#[test]
fn runtime_session_links_only_to_its_persisted_terminal_finding() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-terminal-link")?;
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
    let finding = runtime_finding(&fixture, &runtime.runtime_session_id)?;
    insert_diagnostic_finding(fixture.runtime_home_path(), &finding)?;
    link_mcp_runtime_session_terminal_finding(
        fixture.runtime_home_path(),
        &runtime.runtime_session_id,
        finding.id(),
    )?;
    let linked = mcp_runtime_session(fixture.runtime_home_path(), &runtime.runtime_session_id)?
        .ok_or("linked runtime")?;
    assert_eq!(
        linked.terminal_finding_id.as_deref(),
        Some(finding.id().as_str())
    );
    link_mcp_runtime_session_terminal_finding(
        fixture.runtime_home_path(),
        &runtime.runtime_session_id,
        finding.id(),
    )?;
    Ok(())
}

#[test]
fn graph_insertion_is_atomic_for_missing_causes_and_cycles() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-finding-atomic")?;
    let missing = finding(&fixture, "finding.missing_child", &["finding.not_present"])?;
    assert!(insert_diagnostic_finding_graph(
        fixture.runtime_home_path(),
        std::slice::from_ref(&missing),
    )
    .is_err());
    assert!(diagnostic_finding(fixture.runtime_home_path(), missing.id())?.is_none());

    let left = finding(&fixture, "finding.left", &["finding.right"])?;
    let right = finding(&fixture, "finding.right", &["finding.left"])?;
    assert!(insert_diagnostic_finding_graph(fixture.runtime_home_path(), &[left, right]).is_err());
    assert!(diagnostic_finding(
        fixture.runtime_home_path(),
        &DiagnosticFindingId::parse("finding.left")?
    )?
    .is_none());
    Ok(())
}

#[test]
fn cause_edges_reject_duplicates_cycles_and_dangling_targets() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-cause-integrity")?;
    let left = finding(&fixture, "finding.left", &[])?;
    let right = finding(&fixture, "finding.right", &[])?;
    insert_diagnostic_finding(fixture.runtime_home_path(), &left)?;
    insert_diagnostic_finding(fixture.runtime_home_path(), &right)?;
    let conn = rusqlite::Connection::open(registry_db_path(fixture.runtime_home_path()))?;
    enable_foreign_keys(&conn)?;
    conn.execute(
        "INSERT INTO diagnostic_cause_edges (finding_id, cause_finding_id) VALUES (?1, ?2)",
        ["finding.left", "finding.right"],
    )?;
    assert!(conn
        .execute(
            "INSERT INTO diagnostic_cause_edges (finding_id, cause_finding_id) VALUES (?1, ?2)",
            ["finding.left", "finding.right"],
        )
        .is_err());
    assert!(conn
        .execute(
            "INSERT INTO diagnostic_cause_edges (finding_id, cause_finding_id) VALUES (?1, ?2)",
            ["finding.right", "finding.left"],
        )
        .is_err());
    assert!(conn
        .execute(
            "INSERT INTO diagnostic_cause_edges (finding_id, cause_finding_id) VALUES (?1, ?2)",
            ["finding.right", "finding.absent"],
        )
        .is_err());
    Ok(())
}

#[test]
fn cause_traversal_is_deterministic_and_depth_bounded() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("diagnostic-cause-depth")?;
    let root = finding(&fixture, "finding.root", &[])?;
    let middle = finding(&fixture, "finding.middle", &["finding.root"])?;
    let leaf = finding(&fixture, "finding.leaf", &["finding.middle"])?;
    insert_diagnostic_finding_graph(fixture.runtime_home_path(), &[leaf, root, middle])?;

    let bounded = diagnostic_cause_chain(
        fixture.runtime_home_path(),
        &DiagnosticFindingId::parse("finding.leaf")?,
        1,
    )?;
    assert_eq!(
        bounded
            .entries
            .iter()
            .map(|entry| (entry.depth, entry.finding.id().as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "finding.leaf"), (1, "finding.middle")]
    );
    assert!(bounded.depth_limit_reached);
    let complete = diagnostic_cause_chain(
        fixture.runtime_home_path(),
        &DiagnosticFindingId::parse("finding.leaf")?,
        2,
    )?;
    assert!(!complete.depth_limit_reached);
    assert_eq!(complete.entries.len(), 3);
    assert!(diagnostic_cause_chain(
        fixture.runtime_home_path(),
        &DiagnosticFindingId::parse("finding.leaf")?,
        MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH + 1,
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
