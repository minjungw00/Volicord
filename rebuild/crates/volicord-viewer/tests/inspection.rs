//! Independent HTTP allow/deny cases; test-only fault injection never grants runtime authority.
use serde_json::{json, Value};
use std::io::Cursor;
use volicord_context::{
    CanonicalRecordId, ContextItemCorrectionDraft, ContextItemId, CorrectionKind, Principal,
    PrincipalKind, ProjectId, SourceId, TimestampMicros,
};
use volicord_inquiry::*;
use volicord_operations::{LocalOperations, RuntimeLayout, WorkTransition};
use volicord_privacy::*;
use volicord_viewer::{ViewerAdapter, ViewerLocale, ViewerServer, ViewerView};
const AUTH: &str = "127.0.0.1:3219";
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
struct Fixture {
    _temp: tempfile::TempDir,
    server: ViewerServer,
    project: ProjectId,
}
impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
        let project = ops
            .initialize_project("Inspection controls", None)?
            .project
            .id;
        let server = ViewerServer::new(
            ViewerAdapter::new(ops),
            project,
            ViewerLocale::English,
            ViewerView::Overview,
            "en".into(),
            AUTH.parse()?,
        )?;
        Ok(Self {
            _temp: temp,
            server,
            project,
        })
    }
    fn ops(&self) -> &LocalOperations {
        self.server.adapter().operations()
    }
    fn raw(&self, target: &str, headers: &str) -> Result<String> {
        let mut output = Vec::new();
        self.server.serve_connection(
            &mut Cursor::new(format!("GET {target} HTTP/1.1\r\n{headers}\r\n")),
            &mut output,
        )?;
        Ok(String::from_utf8(output)?)
    }
    fn get(&self, query: &str) -> Result<String> {
        self.raw(
            &format!("/inspection.json?{query}"),
            &format!("Host: {AUTH}\r\n"),
        )
    }
    fn page(&self, query: &str) -> Result<Value> {
        let response = self.get(query)?;
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(response.contains("Cache-Control: no-store"));
        Ok(serde_json::from_str(
            response.split_once("\r\n\r\n").ok_or("HTTP body")?.1,
        )?)
    }
    fn goal(&self, project: ProjectId, text: &str) -> Result<ContextItemId> {
        Ok(self
            .ops()
            .transition_work(
                project,
                WorkTransition::StartNew {
                    host: "test".into(),
                    session: "controls".into(),
                    user_turn: text.into(),
                    statement: text.into(),
                },
            )?
            .context_item_id)
    }
    fn source(&self, project: ProjectId, text: &str) -> Result<SourceId> {
        let s = self.ops().record_user_source(
            project,
            "test".into(),
            "controls".into(),
            text.into(),
        )?;
        Ok(source_id(&s.identity))
    }
    fn candidate(&self, source: SourceId, text: &str) -> Result<CandidateId> {
        match self.ops().submit_candidate(CandidateDraft {
            project_id: self.project,
            kind: CandidateKind::Observation,
            collection_mode: CandidateCollectionMode::Automatic,
            origin: CandidateOrigin {
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "test".into(),
                },
                subsystem: "inspection-control".into(),
                session: None,
                provenance_summary: "metadata".into(),
            },
            collection_scope: CandidateCollectionScope {
                project_id: self.project,
                session: None,
                source_operation: None,
                candidate_kind: CandidateKind::Observation,
            },
            observation_basis: CandidateObservationBasis {
                source_basis: vec![source],
                ..Default::default()
            },
            observed_at: TimestampMicros::from_unix_micros(1),
            retention: CandidateRetention {
                retained_until: None,
                basis: "test retention".into(),
            },
            content: CandidateContent {
                bounded_summary: text.into(),
                question: None,
                engineering_choice_discovery: None,
                materiality_review: None,
                learning_deliberation: None,
            },
        })? {
            SubmissionOutcome::Stored(c) => Ok(c.id),
            _ => Err("collection disabled".into()),
        }
    }
    fn derived(&self, source: SourceId, text: &str) -> Result<ManagedDerivedId> {
        Ok(PrivacyStore::open(self.ops().layout().privacy_store())?
            .record_managed_derived(ManagedDerivedDraft {
                project_id: self.project,
                kind: ManagedDerivedKind::CachedSummary,
                provider: None,
                model: None,
                purpose: "excluded-purpose-secret".into(),
                analysis_snapshot: None,
                included_sources: Vec::new(),
                canonical_links: vec![ManagedCanonicalLink::Source(source)],
                content: text.into(),
                uncertainty: None,
                retained_until: None,
                retention_basis: "test".into(),
            })?
            .id)
    }
}
fn source_id(s: &str) -> SourceId {
    let bytes: Vec<u8> = (0..32)
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect();
    SourceId::from_bytes(bytes.try_into().unwrap())
}

#[test]
fn mixed_projects_source_body_and_literal_search_do_not_leak_through_public_queries() -> Result {
    let f = Fixture::new()?;
    let own = f.goal(f.project, "Find permitted retry policy")?;
    let foreign = f
        .ops()
        .initialize_project("foreign-project-marker", None)?
        .project
        .id;
    let before = f.page("class=work")?;
    let hidden = f.goal(foreign, "foreign-needle-marker")?;
    let foreign_source = f.source(foreign, "foreign-source-body-marker")?;
    let source = f.source(f.project, "withheld-user-turn-marker")?;
    assert_eq!(
        before["page"]["fingerprint"],
        f.page("class=work")?["page"]["fingerprint"]
    );
    assert_eq!(
        f.page("class=work&q=foreign-needle-marker")?["page"]["permitted_match_count"],
        0
    );
    for query in [
        format!("class=work&id={hidden}"),
        format!("class=source&id={foreign_source}"),
        format!("class=source&id={}", "ff".repeat(16)),
    ] {
        let response = f.get(&query)?;
        assert!(response.starts_with("HTTP/1.1 404"), "{response}");
        assert!(!response.contains("foreign-"));
    }
    let source_rows = f.page("class=source")?;
    assert!(!source_rows
        .to_string()
        .contains("withheld-user-turn-marker"));
    assert_eq!(
        f.page("class=source&q=withheld-user-turn-marker")?["page"]["permitted_match_count"],
        0
    );
    let reference = f.page(&format!(
        "class=source&id={source}&revision=1&field=reference"
    ))?;
    assert!(!reference.to_string().contains("withheld-user-turn-marker"));
    let detail = f.page(&format!("class=source&id={source}&revision=1&field=body"))?;
    assert_eq!(detail["detail"]["state"], "unavailable");
    assert!(!detail.to_string().contains("withheld-user-turn-marker"));
    for query in [
        "class=work&q=%27%20OR%201%3D1%20--",
        "class=work&q=sqlite_master",
        "class=work&q=%3Cscript%3E",
    ] {
        assert_eq!(f.page(query)?["page"]["permitted_match_count"], 0);
    }
    assert_eq!(
        f.page("class=work&q=retry")?["page"]["rows"][0]["identity"],
        own.to_string()
    );
    Ok(())
}

#[test]
fn deterministic_paging_filters_and_changed_correction_forgetting_refuse_old_continuation() -> Result
{
    let f = Fixture::new()?;
    for i in 0..24 {
        f.goal(f.project, &format!("Recorded check {i:02}"))?;
    }
    let before = f.ops().canonical_basis(f.project)?;
    let privacy = f.ops().privacy_status(f.project)?;
    let first = f.page("class=work&limit=7&role=goal&state=current")?;
    let basis = first["page"]["fingerprint"].as_str().ok_or("basis")?;
    let mut ids = Vec::new();
    let mut offset = 0;
    loop {
        let p = f.page(&format!(
            "class=work&limit=7&role=goal&state=current&offset={offset}&basis={basis}"
        ))?;
        ids.extend(
            p["page"]["rows"]
                .as_array()
                .ok_or("rows")?
                .iter()
                .map(|r| r["identity"].as_str().unwrap().to_owned()),
        );
        if let Some(next) = p["page"]["next_offset"].as_u64() {
            offset = next;
        } else {
            break;
        }
    }
    assert_eq!(ids.len(), 24);
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(ids, sorted);
    assert_eq!(before, f.ops().canonical_basis(f.project)?);
    assert_eq!(privacy, f.ops().privacy_status(f.project)?);
    let id = ContextItemId::from_bytes(*source_id(&ids[0]).as_bytes());
    let auth = f.source(f.project, "Correct only this spelling")?;
    let corrected = format!(
        "{}.",
        before
            .context_items
            .iter()
            .find(|i| i.id == id)
            .ok_or("Goal")?
            .statement
    );
    f.ops().correct_context_item(
        f.project,
        id,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            kind: CorrectionKind::Expression,
            corrected_statement: corrected.clone(),
            user_authorization_source_id: auth,
        },
    )?;
    let stale = f.get(&format!(
        "class=work&limit=7&role=goal&state=current&offset=7&basis={basis}"
    ))?;
    assert!(stale.starts_with("HTTP/1.1 409"), "{stale}");
    assert!(f
        .get(&format!("class=work&id={id}&revision=1&field=statement"))?
        .starts_with("HTTP/1.1 409"));
    let current = f.page(&format!("class=work&id={id}&revision=2&field=statement"))?;
    assert!(current["detail"]["chunk"]
        .as_str()
        .unwrap()
        .contains(&corrected));
    f.ops()
        .forget_record(f.project, CanonicalRecordId::ContextItem(id), auth)?;
    assert!(f
        .get(&format!("class=work&id={id}"))?
        .starts_with("HTTP/1.1 404"));
    assert_eq!(
        f.page("class=work&q=Corrected")?["page"]["permitted_match_count"],
        0
    );
    Ok(())
}

#[test]
fn candidate_retention_and_derived_private_fields_are_not_searchable_or_hashed() -> Result {
    let f = Fixture::new()?;
    let s = f.source(f.project, "Source body private")?;
    let c = f.candidate(s, "candidate-expired-marker")?;
    let d = f.derived(s, "derived-body-marker")?;
    assert_eq!(
        f.page("class=candidate&q=candidate-expired-marker")?["page"]["permitted_match_count"],
        1
    );
    let db = rusqlite::Connection::open(f.ops().layout().candidate_store())?;
    let raw: String = db.query_row(
        "SELECT record_json FROM candidates WHERE id=?1",
        [c.as_bytes().as_slice()],
        |r| r.get(0),
    )?;
    let mut record: Value = serde_json::from_str(&raw)?;
    record["retention"]["retained_until"] = json!(2);
    db.execute(
        "UPDATE candidates SET record_json=?1 WHERE id=?2",
        rusqlite::params![record.to_string(), c.as_bytes().as_slice()],
    )?;
    let expired = f.page("class=candidate")?;
    assert_eq!(expired["page"]["rows"][0]["state"], "expired");
    assert!(f
        .get(&format!("class=candidate&id={c}&revision=1&field=summary"))?
        .starts_with("HTTP/1.1 403"));
    assert!(!expired.to_string().contains("candidate-expired-marker"));
    assert_eq!(
        f.page("class=candidate&q=candidate-expired-marker")?["page"]["permitted_match_count"],
        0
    );
    record["content"]["bounded_summary"] = json!("changed-hidden-marker");
    db.execute(
        "UPDATE candidates SET record_json=?1 WHERE id=?2",
        rusqlite::params![record.to_string(), c.as_bytes().as_slice()],
    )?;
    assert_eq!(
        expired["page"]["fingerprint"],
        f.page("class=candidate")?["page"]["fingerprint"]
    );
    let derived = f.page("class=derived")?;
    assert!(!derived.to_string().contains("derived-body-marker"));
    assert!(!derived.to_string().contains("excluded-purpose-secret"));
    assert_eq!(
        f.page("class=derived&q=derived-body-marker")?["page"]["permitted_match_count"],
        0
    );
    assert!(f
        .get(&format!("class=derived&id={d}&revision=1&field=content"))?
        .starts_with("HTTP/1.1 400"));
    Ok(())
}

#[test]
fn malformed_queries_cross_origin_and_wrong_authority_fail_before_reads() -> Result {
    let f = Fixture::new()?;
    let before = f.ops().canonical_basis(f.project)?;
    for query in [
        "class=sqlite_master",
        "class=work&limit=0",
        "class=work&limit=21",
        "class=work&offset=999999999999999999999999",
        "class=work&offset=100001",
        "class=work&q=%00",
        "class=work&state=body",
        "class=work&role=private",
        "class=work&id=wrong",
        "class=work&sql=select",
        "class=work&q=a&q=b",
        "class=work&field=statement",
        "class=work&limit=-1",
    ] {
        let r = f.get(query)?;
        assert!(r.starts_with("HTTP/1.1 400"), "{query}: {r}");
    }
    assert!(f.get("class=work&offset=1")?.starts_with("HTTP/1.1 409"));
    assert!(f
        .raw("/inspection.json?class=work", "Host: attacker.invalid\r\n")?
        .starts_with("HTTP/1.1 421"));
    assert!(f
        .raw(
            "/inspection.json?class=work",
            &format!("Host: {AUTH}\r\nOrigin: https://attacker.invalid\r\n")
        )?
        .starts_with("HTTP/1.1 403"));
    assert!(f
        .raw(
            "/inspection.json?class=work",
            &format!("Host: {AUTH}\r\nSec-Fetch-Site: cross-site\r\n")
        )?
        .starts_with("HTTP/1.1 403"));
    assert_eq!(before, f.ops().canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn pending_forgetting_hides_candidate_and_derived_content_before_cleanup() -> Result {
    let f = Fixture::new()?;
    let target = f.source(f.project, "Pending deletion Source")?;
    let candidate = f.candidate(target, "pending-candidate-secret")?;
    let derived = f.derived(target, "pending-derived-secret")?;
    let auth = f.source(f.project, "Forget this exact Source")?;
    let before = f.page("class=candidate")?;
    let blocker = rusqlite::Connection::open(f.ops().layout().candidate_store())?;
    blocker.execute_batch("BEGIN DEFERRED")?;
    let _: String = blocker.query_row(
        "SELECT record_json FROM candidates WHERE id=?1",
        [candidate.as_bytes().as_slice()],
        |r| r.get(0),
    )?;
    let forgotten = f
        .ops()
        .forget_record(f.project, CanonicalRecordId::Source(target), auth)?;
    assert_eq!(
        forgotten.state,
        volicord_operations::ForgettingState::RepairRequired
    );
    let page = f.page("class=candidate")?;
    assert_eq!(page["page"]["rows"][0]["state"], "withheld");
    assert!(!page.to_string().contains("pending-candidate-secret"));
    assert!(page["page"]["rows"][0]["relationships"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_ne!(page["page"]["fingerprint"], before["page"]["fingerprint"]);
    assert_eq!(
        f.page("class=candidate&q=pending-candidate-secret")?["page"]["permitted_match_count"],
        0
    );
    let dp = f.page(&format!("class=derived&id={derived}"))?;
    assert_eq!(dp["page"]["rows"][0]["state"], "withheld");
    assert!(!dp.to_string().contains("pending-derived-secret"));
    assert!(!f
        .get(&format!(
            "class=candidate&id={candidate}&revision=1&field=summary"
        ))?
        .contains("pending-candidate-secret"));
    blocker.execute_batch("ROLLBACK")?;
    Ok(())
}

#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
fn rich() -> Result<(reading_fixture::Fixture, Fixture)> {
    let control = reading_fixture::fixture()?;
    let project = control.project;
    let ops = LocalOperations::new(control.operations.layout().clone());
    let server = ViewerServer::new(
        ViewerAdapter::new(ops),
        project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        AUTH.parse()?,
    )?;
    Ok((
        control,
        Fixture {
            _temp: tempfile::tempdir()?,
            server,
            project,
        },
    ))
}
#[test]
fn explicit_relationships_keep_work_boundaries_and_all_pages_are_exact() -> Result {
    let (control, f) = rich()?;
    let older = control.goals["older"];
    let page = f.page(&format!("class=work&id={older}"))?;
    let basis = page["page"]["fingerprint"].as_str().ok_or("basis")?;
    let mut relation_ids = Vec::new();
    let mut offset = 0;
    loop {
        let p = f.page(&format!(
            "class=work&id={older}&relationships={offset}&basis={basis}"
        ))?;
        for rel in p["page"]["rows"][0]["relationships"]
            .as_array()
            .ok_or("relationships")?
        {
            let id = rel["identity"].as_str().ok_or("identity")?;
            let class = rel["class"].as_str().ok_or("class")?;
            assert_eq!(
                f.page(&format!("class={class}&id={id}"))?["page"]["rows"][0]["identity"],
                id
            );
            relation_ids.push(id.to_owned());
        }
        if let Some(next) = p["page"]["rows"][0]["next_relationship_offset"].as_u64() {
            offset = next;
        } else {
            break;
        }
    }
    assert!(relation_ids.contains(&control.decisions["explicit"].to_string()));
    assert!(!relation_ids.contains(&control.decisions["other_work"].to_string()));
    assert!(!relation_ids.contains(&control.goals["same_title"].to_string()));
    // An untrusted query cannot request an inferred join or alternate Project.
    assert!(f
        .get(&format!("class=work&id={older}&join=path"))?
        .starts_with("HTTP/1.1 400"));
    assert!(f
        .get(&format!("class=work&project={}", "11".repeat(16)))?
        .starts_with("HTTP/1.1 400"));
    Ok(())
}

#[test]
fn source_body_is_exact_work_scoped_and_rechecks_expiration_on_each_http_chunk() -> Result {
    let (control, f) = rich()?;
    let analysis = f
        .ops()
        .project_projection(f.project)?
        .repository_analysis
        .analysis_snapshot
        .ok_or("analysis")?;
    let work = control.goals["older"];
    let receipt = f
        .ops()
        .observe_work_file(volicord_operations::FileObservationRequest {
            byte_range: None,
            project_id: f.project,
            work_item_id: work,
            analysis_snapshot_id: analysis,
            path: "python/worker.py".into(),
            before_source_id: None,
            retain_seconds: 3600,
        })?;
    let source = receipt.source_id;
    let reference = f.page(&format!(
        "class=source&id={source}&revision={}&field=reference",
        receipt.source_revision
    ))?;
    assert!(reference["detail"]["chunk"]
        .as_str()
        .ok_or("reference")?
        .contains("python/worker.py"));
    assert!(!reference.to_string().contains("def "));
    let query = format!(
        "class=source&id={source}&revision={}&field=body&work={work}",
        receipt.source_revision
    );
    let positive = f.page(&query)?;
    assert_ne!(positive["detail"]["state"], "unavailable");
    assert!(positive["detail"]["chunk"]
        .as_str()
        .unwrap()
        .contains("def "));
    let withheld = f.page(&format!(
        "class=source&id={source}&revision={}&field=body",
        receipt.source_revision
    ))?;
    assert_eq!(withheld["detail"]["state"], "unavailable");
    assert!(!withheld.to_string().contains("def "));
    let wrong = f.get(&format!(
        "class=source&id={source}&revision={}&field=body&work={}",
        receipt.source_revision, control.goals["same_title"]
    ))?;
    assert!(!wrong.contains("def "));
    let db = rusqlite::Connection::open(f.ops().layout().privacy_store())?;
    let records: Vec<(Vec<u8>, String)> = db
        .prepare("SELECT id,record_json FROM managed_derived")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<std::result::Result<_, _>>()?;
    for (id, raw) in records {
        let mut record: Value = serde_json::from_str(&raw)?;
        if raw.contains(&source.to_string()) && record["kind"] == "source_observation" {
            record["retained_until"] = json!(1);
            db.execute(
                "UPDATE managed_derived SET record_json=?1 WHERE id=?2",
                rusqlite::params![record.to_string(), id],
            )?;
        }
    }
    let expired = f.page(&query)?;
    assert_eq!(expired["detail"]["state"], "unavailable");
    assert!(!expired.to_string().contains("def "));
    assert_eq!(
        f.page("class=source&q=def%20")?["page"]["permitted_match_count"],
        0
    );
    Ok(())
}

#[test]
fn foreign_relationship_targets_and_unavailable_dependencies_are_not_empty_success() -> Result {
    let f = Fixture::new()?;
    let foreign = f
        .ops()
        .initialize_project("Foreign relationship Project", None)?
        .project
        .id;
    let secret = f.source(foreign, "foreign relationship body")?;
    // Privacy metadata may retain an opaque reference; the inspector must admit its endpoint.
    let d = f.derived(secret, "not-readable-through-a-link")?;
    let row = f.page(&format!("class=derived&id={d}"))?;
    assert!(row["page"]["rows"][0]["relationships"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(row["page"]["rows"][0]["relationships_omitted"], true);
    assert!(!row.to_string().contains(&secret.to_string()));
    let canonical = f.ops().canonical_basis(f.project)?;
    // Non-current format fault injection uses the same owner boundary as other tests.
    let db = rusqlite::Connection::open(f.ops().layout().candidate_store())?;
    db.execute(
        "UPDATE metadata SET value='999' WHERE key='schema_version'",
        [],
    )?;
    let unavailable = f.get("class=candidate")?;
    assert!(unavailable.starts_with("HTTP/1.1 503"), "{unavailable}");
    assert!(!unavailable.contains("permitted_match_count"));
    assert!(!unavailable.contains("record_json"));
    assert!(f.get("class=work")?.starts_with("HTTP/1.1 200"));
    assert_eq!(canonical, f.ops().canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn html_search_exact_fields_and_receipts_escape_stored_content_and_keep_protected_actions() -> Result
{
    let f = Fixture::new()?;
    let goal = f.goal(
        f.project,
        "Inspect <script>window.stolen=1</script> & retained wording",
    )?;
    let list = f.raw(
        "/inspection?class=work&q=%3Cscript%3E",
        &format!("Host: {AUTH}\r\n"),
    )?;
    assert!(list.starts_with("HTTP/1.1 200"), "{list}");
    assert!(!list.contains("<script>"));
    assert!(list.contains("&lt;script&gt;"));
    assert!(!list.contains("<pre>"));
    assert!(list.contains("Read Work document"));
    let exact = f.raw(
        &format!("/inspection?class=work&id={goal}&revision=1&field=statement"),
        &format!("Host: {AUTH}\r\n"),
    )?;
    assert!(!exact.contains("<script>"));
    assert!(exact.contains("Back to results"));
    assert!(exact.contains("/memory/context/correct"));
    assert!(exact.contains("/memory/forget"));
    let localized = f.raw(
        &format!("/inspection?class=work&id={goal}&locale=ko&language=ja"),
        &format!("Host: {AUTH}\r\n"),
    )?;
    assert!(localized.contains(&format!("work={goal}&amp;locale=ko&amp;language=ja")));
    let marker = "name=\"request_authenticity\" value=\"";
    let token = exact
        .split_once(marker)
        .ok_or("authenticity")?
        .1
        .split('"')
        .next()
        .ok_or("token")?;
    let body=format!("record_kind=context_item&record_id={goal}&user_turn=Forget+this+exact+Goal&view=tools&tool=memory&inspection_class=work&request_authenticity={token}");
    let post = |origin: &str, token_body: &str| {
        format!("POST /memory/forget HTTP/1.1\r\nHost: {AUTH}\r\nOrigin: {origin}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{token_body}",token_body.len())
    };
    let mut output = Vec::new();
    f.server.serve_connection(
        &mut Cursor::new(post("https://attacker.invalid", &body)),
        &mut output,
    )?;
    assert!(String::from_utf8(output)?.starts_with("HTTP/1.1 403"));
    assert!(f
        .get(&format!("class=work&id={goal}"))?
        .starts_with("HTTP/1.1 200"));
    let mut output = Vec::new();
    f.server.serve_connection(
        &mut Cursor::new(post(&format!("http://{AUTH}"), &body)),
        &mut output,
    )?;
    let receipt = String::from_utf8(output)?;
    assert!(receipt.starts_with("HTTP/1.1 200"), "{receipt}");
    assert!(receipt.contains("Forgetting completed"));
    assert!(receipt.contains(&goal.to_string()));
    assert!(!receipt.contains("<script>"));
    assert!(!receipt.contains(token));
    assert!(f
        .get(&format!("class=work&id={goal}"))?
        .starts_with("HTTP/1.1 404"));
    Ok(())
}

#[test]
fn seed_inspection_review() -> Result {
    let Some(root) = std::env::var_os("VOLICORD_INSPECTION_REVIEW_ROOT") else {
        return Ok(());
    };
    let root = std::path::PathBuf::from(root);
    std::fs::create_dir_all(&root)?;
    let control = reading_fixture::scenario_in(&root, reading_fixture::rich_scenario()?)?;
    let ops = LocalOperations::new(control.operations.layout().clone());
    let server = ViewerServer::new(
        ViewerAdapter::new(ops),
        control.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        AUTH.parse()?,
    )?;
    let f = Fixture {
        _temp: tempfile::tempdir()?,
        server,
        project: control.project,
    };
    let s = f.source(f.project, "private-source-marker")?;
    let candidate = f.candidate(s, "Inspect a retained investigation candidate")?;
    let expired = f.candidate(s, "expired-review-secret")?;
    let derived = f.derived(s, "derived-review-secret")?;
    let db = rusqlite::Connection::open(f.ops().layout().candidate_store())?;
    let raw: String = db.query_row(
        "SELECT record_json FROM candidates WHERE id=?1",
        [expired.as_bytes().as_slice()],
        |r| r.get(0),
    )?;
    let mut record: Value = serde_json::from_str(&raw)?;
    record["retention"]["retained_until"] = json!(2);
    db.execute(
        "UPDATE candidates SET record_json=?1 WHERE id=?2",
        rusqlite::params![record.to_string(), expired.as_bytes().as_slice()],
    )?;
    let empty = f
        .ops()
        .initialize_project("Empty inspection control", None)?
        .project
        .id;
    let manifest = json!({"runtime":f.ops().layout().root(),"project":f.project.to_string(),"work":control.goals["older"].to_string(),
        "decision":control.decisions["explicit"].to_string(),"candidate":candidate.to_string(),"expired":expired.to_string(),"derived":derived.to_string(),
        "empty_project":empty.to_string(),"synthetic":true,"human_acceptance":"pending"});
    std::fs::write(
        root.join("fixture.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    let _kept = control._temporary.keep();
    Ok(())
}

#[test]
fn decision_correction_and_supersession_use_domain_operations_and_exact_receipts() -> Result {
    let (control, f) = rich()?;
    let id = control.decisions["project"];
    let basis = f.ops().canonical_basis(f.project)?;
    let original = &basis
        .active_decisions
        .iter()
        .find(|d| d.decision.id == id)
        .ok_or("Decision")?
        .decision;
    let corrected = format!(
        "{}.",
        original.user_rationale.as_deref().ok_or("rationale")?
    );
    let alternative=original.displayed_alternatives.iter().find(|a|!matches!(&original.choice,volicord_context::DecisionChoice::Alternative{alternative_key} if *alternative_key==a.key)).ok_or("different alternative")?.key.clone();
    let html = f.raw(
        &format!("/inspection?class=decision&id={id}"),
        &format!("Host: {AUTH}\r\n"),
    )?;
    let token = html
        .split_once("name=\"request_authenticity\" value=\"")
        .ok_or("token")?
        .1
        .split('"')
        .next()
        .ok_or("token")?;
    let encode = |s: &str| s.bytes().map(|b| format!("%{b:02X}")).collect::<String>();
    let post = |path: &str, body: String| -> Result<String> {
        let body = format!(
            "{body}&view=tools&tool=memory&inspection_class=decision&request_authenticity={token}"
        );
        let request=format!("POST {path} HTTP/1.1\r\nHost: {AUTH}\r\nOrigin: http://{AUTH}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",body.len());
        let mut output = Vec::new();
        f.server
            .serve_connection(&mut Cursor::new(request), &mut output)?;
        Ok(String::from_utf8(output)?)
    };
    let correction=post("/memory/decision/correct",format!("record_id={id}&expected_revision={}&corrected_text={}&user_turn=Correct+the+punctuation",original.revision,encode(&corrected)))?;
    assert!(correction.starts_with("HTTP/1.1 200"), "{correction}");
    assert!(correction.contains(&id.to_string()));
    assert!(correction.contains("revision 2"));
    let supersession = post(
        "/memory/decision/supersede",
        format!(
            "record_id={id}&alternative={}&user_turn=Choose+this+different+displayed+alternative",
            encode(&alternative)
        ),
    )?;
    assert!(supersession.starts_with("HTTP/1.1 200"), "{supersession}");
    let basis = f.ops().canonical_basis(f.project)?;
    let old = basis
        .superseded_decisions
        .iter()
        .find(|d| d.decision.id == id)
        .ok_or("superseded")?;
    let new = old.superseded_by.ok_or("successor")?;
    assert_ne!(new, id);
    assert!(supersession.contains(&new.to_string()));
    assert!(!supersession.contains(token));
    let successor = &basis
        .active_decisions
        .iter()
        .find(|d| d.decision.id == new)
        .ok_or("successor")?
        .decision;
    assert_eq!(successor.work_scope, original.work_scope);
    assert!(successor.user_rationale.is_none());
    assert_eq!(
        f.page(&format!("class=decision&id={id}"))?["page"]["rows"][0]["state"],
        "superseded"
    );
    assert_eq!(
        f.page(&format!("class=decision&id={new}"))?["page"]["rows"][0]["state"],
        "active"
    );
    Ok(())
}

/// Controlled store scaling, not naturalistic evidence or a release SLA. The
/// retained root reuses exactly the same SQLite inputs across old/new binaries.
#[test]
#[ignore = "explicit local profiling; retains synthetic inputs under the supplied ignored root"]
fn profile_inspection_scaling() -> Result {
    let root = std::path::PathBuf::from(
        std::env::var_os("VOLICORD_INSPECTION_PROFILE_ROOT").ok_or("profile root required")?,
    );
    for n in [10usize, 100, 1000] {
        let path = root.join(n.to_string());
        let project = if path.join("project.json").exists() {
            serde_json::from_slice(&std::fs::read(path.join("project.json"))?)?
        } else {
            std::fs::create_dir_all(&path)?;
            let ops = LocalOperations::new(RuntimeLayout::new(path.join("runtime"))?);
            let project = ops
                .initialize_project("Controlled inspection scale", None)?
                .project
                .id;
            ops.transition_work(
                project,
                WorkTransition::StartNew {
                    host: "test".into(),
                    session: "profile".into(),
                    user_turn: "Controlled work".into(),
                    statement: "Controlled work".into(),
                },
            )?;
            let db = rusqlite::Connection::open(ops.layout().canonical_store())?;
            // Clone the maintained producer's typed row, changing only identity/text.
            for i in 1..n {
                let id = (i as u128).to_be_bytes();
                db.execute("INSERT INTO context_items SELECT ?1,project_id,revision,role,?2,provenance_role,author_kind,author_identity,applicability_paths,applicability_components,applicability_work_contexts,recorded_at FROM context_items LIMIT 1", rusqlite::params![id.as_slice(), format!("Controlled work {i:04} 한글")])?;
                db.execute("INSERT INTO context_item_sources SELECT project_id,?1,source_id,position FROM context_item_sources LIMIT 1", [id.as_slice()])?;
            }
            std::fs::write(path.join("project.json"), serde_json::to_vec(&project)?)?;
            project
        };
        let ops = LocalOperations::new(RuntimeLayout::new(path.join("runtime"))?);
        let server = ViewerServer::new(
            ViewerAdapter::new(ops),
            project,
            ViewerLocale::English,
            ViewerView::Overview,
            "en".into(),
            AUTH.parse()?,
        )?;
        let f = Fixture {
            _temp: tempfile::tempdir()?,
            server,
            project,
        };
        let mut fingerprints = Vec::new();
        for run in 0..3 {
            let start = std::time::Instant::now();
            let response = f.get("class=work&limit=7")?;
            let micros = start.elapsed().as_micros();
            let page: Value =
                serde_json::from_str(response.split_once("\r\n\r\n").ok_or("body")?.1)?;
            assert_eq!(page["page"]["permitted_match_count"], n);
            assert_eq!(page["page"]["rows"].as_array().ok_or("rows")?.len(), 7);
            fingerprints.push(page["page"]["fingerprint"].clone());
            println!(
                "PROFILE n={n} run={run} elapsed_us={micros} http_bytes={} fingerprint={}",
                response.len(),
                fingerprints.last().unwrap()
            );
        }
        assert!(fingerprints.windows(2).all(|w| w[0] == w[1]));
    }
    Ok(())
}

// Test-only controlled scale injection clones actual maintained producer rows.
fn clone_row(
    db: &rusqlite::Connection,
    table: &str,
    changes: &[(&str, rusqlite::types::Value)],
) -> Result {
    let columns = db
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let selects = columns
        .iter()
        .map(|c| {
            changes
                .iter()
                .position(|(name, _)| c == name)
                .map_or_else(|| c.clone(), |i| format!("?{}", i + 1))
        })
        .collect::<Vec<_>>()
        .join(",");
    db.execute(
        &format!("INSERT INTO {table} SELECT {selects} FROM {table} LIMIT 1"),
        rusqlite::params_from_iter(changes.iter().map(|(_, v)| v)),
    )?;
    Ok(())
}

#[test]
#[ignore = "explicit controlled scale profiling on retained identical inputs"]
fn profile_inspection_related_scaling() -> Result {
    use rusqlite::types::Value as Sql;
    use sha2::{Digest, Sha256};
    let root = std::path::PathBuf::from(
        std::env::var_os("VOLICORD_INSPECTION_PROFILE_ROOT").ok_or("root")?,
    );
    for dense in [false, true] {
        if std::env::var("VOLICORD_INSPECTION_PROFILE_DENSE")
            .ok()
            .is_some_and(|v| v != dense.to_string())
        {
            continue;
        }
        for n in [10usize, 100, 1000] {
            if std::env::var("VOLICORD_INSPECTION_PROFILE_SIZE")
                .ok()
                .is_some_and(|v| v != n.to_string())
            {
                continue;
            }
            let path = root.join(format!("related-{dense}-{n}"));
            let project = if path.join("project.json").exists() {
                serde_json::from_slice(&std::fs::read(path.join("project.json"))?)?
            } else {
                std::fs::create_dir_all(&path)?;
                let control =
                    reading_fixture::scenario_in(&path, reading_fixture::rich_scenario()?)?;
                let project = control.project;
                let ops = LocalOperations::new(control.operations.layout().clone());
                let server = ViewerServer::new(
                    ViewerAdapter::new(ops),
                    project,
                    ViewerLocale::English,
                    ViewerView::Overview,
                    "en".into(),
                    AUTH.parse()?,
                )?;
                let f = Fixture {
                    _temp: tempfile::tempdir()?,
                    server,
                    project,
                };
                let source = f.source(project, "controlled private source")?;
                f.candidate(source, "Controlled candidate")?;
                f.derived(source, "controlled private Derived")?;
                let db = rusqlite::Connection::open(f.ops().layout().canonical_store())?;
                let work = control.goals["older"];
                for i in 1..=n {
                    let wid = ContextItemId::from_bytes((i as u128).to_be_bytes());
                    clone_row(
                        &db,
                        "context_items",
                        &[
                            ("id", Sql::Blob(wid.as_bytes().to_vec())),
                            ("role", Sql::Text("goal".into())),
                            ("statement", Sql::Text(format!("Scale work {i:04} 한글"))),
                        ],
                    )?;
                    clone_row(
                        &db,
                        "sources",
                        &[("id", Sql::Blob((i as u128).to_be_bytes().to_vec()))],
                    )?;
                    for j in 0..4 {
                        let id = ((i * 4 + j) as u128).to_be_bytes();
                        clone_row(
                            &db,
                            "checkpoints",
                            &[
                                ("id", Sql::Blob(id.to_vec())),
                                (
                                    "work_item_id",
                                    Sql::Blob(if dense { work } else { wid }.as_bytes().to_vec()),
                                ),
                            ],
                        )?;
                        // Give every Checkpoint a permitted Source and Question/Decision edges.
                        for table in [
                            "checkpoint_source_relations",
                            "checkpoint_decisions",
                            "checkpoint_questions",
                        ] {
                            let count: usize =
                                db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| {
                                    r.get(0)
                                })?;
                            if count > 0 {
                                clone_row(
                                    &db,
                                    table,
                                    &[("checkpoint_id", Sql::Blob(id.to_vec()))],
                                )?;
                            }
                        }
                    }
                }
                for (store, table) in [
                    (f.ops().layout().candidate_store(), "candidates"),
                    (f.ops().layout().privacy_store(), "managed_derived"),
                ] {
                    let db = rusqlite::Connection::open(store)?;
                    let raw: String = db.query_row(
                        &format!("SELECT record_json FROM {table} LIMIT 1"),
                        [],
                        |r| r.get(0),
                    )?;
                    for i in 1..=n {
                        let mut record: Value = serde_json::from_str(&raw)?;
                        let id = CandidateId::from_bytes((i as u128).to_be_bytes());
                        record["id"] = json!(id);
                        clone_row(
                            &db,
                            table,
                            &[
                                ("id", Sql::Blob(id.as_bytes().to_vec())),
                                ("record_json", Sql::Text(record.to_string())),
                            ],
                        )?;
                    }
                }
                std::fs::write(path.join("project.json"), serde_json::to_vec(&project)?)?;
                std::fs::write(
                    path.join("runtime-path.json"),
                    serde_json::to_vec(f.ops().layout().root())?,
                )?;
                let _keep = control._temporary.keep();
                project
            };
            let runtime: std::path::PathBuf =
                serde_json::from_slice(&std::fs::read(path.join("runtime-path.json"))?)?;
            let ops = LocalOperations::new(RuntimeLayout::new(runtime)?);
            let server = ViewerServer::new(
                ViewerAdapter::new(ops),
                project,
                ViewerLocale::English,
                ViewerView::Overview,
                "en".into(),
                AUTH.parse()?,
            )?;
            let f = Fixture {
                _temp: tempfile::tempdir()?,
                server,
                project,
            };
            let basis = f.ops().canonical_basis(project)?;
            assert_eq!(
                basis
                    .context_items
                    .iter()
                    .filter(|c| c.role == volicord_context::ContextItemRole::Goal)
                    .count(),
                n + 10
            );
            println!(
                "DATA dense={dense} n={n} works={} checkpoints={} sources={}",
                basis
                    .context_items
                    .iter()
                    .filter(|c| c.role == volicord_context::ContextItemRole::Goal)
                    .count(),
                basis.checkpoint_history.len(),
                basis.sources.len()
            );
            for class in [
                "work",
                "checkpoint",
                "decision",
                "question",
                "source",
                "context",
                "candidate",
                "derived",
            ] {
                if std::env::var("VOLICORD_INSPECTION_PROFILE_CLASS")
                    .ok()
                    .is_some_and(|v| v != class)
                {
                    continue;
                }
                for run in 0..std::env::var("VOLICORD_INSPECTION_PROFILE_RUNS")
                    .ok()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(3)
                {
                    let start = std::time::Instant::now();
                    let response = f.get(&format!("class={class}&limit=7"))?;
                    let micros = start.elapsed().as_micros();
                    if let Some(output) = std::env::var_os("VOLICORD_INSPECTION_PROFILE_OUTPUT") {
                        let output = std::path::PathBuf::from(output);
                        std::fs::create_dir_all(&output)?;
                        std::fs::write(
                            output.join(format!("{dense}-{n}-{class}-{run}.http")),
                            &response,
                        )?;
                    }
                    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
                    println!("RELATED dense={dense} n={n} class={class} run={run} elapsed_us={micros} bytes={} sha256={:x}", response.len(), Sha256::digest(response.as_bytes()));
                }
                let (_, cost) = f.ops().inspect_project_records_profiled(
                    project,
                    &volicord_operations::InspectionQuery {
                        class: volicord_operations::InspectionClass::parse(class)?,
                        limit: 7,
                        ..Default::default()
                    },
                )?;
                assert!(cost.retained_rows <= 7);
                println!("COST dense={dense} n={n} class={class} {cost:?}");
            }
        }
    }
    Ok(())
}

#[test]
#[ignore = "actual HTML consumer on an explicitly retained large profiling Project"]
fn profile_inspection_large_html() -> Result {
    let root = std::path::PathBuf::from(
        std::env::var_os("VOLICORD_INSPECTION_PROFILE_ROOT").ok_or("root")?,
    )
    .join("related-false-1000");
    let project = serde_json::from_slice(&std::fs::read(root.join("project.json"))?)?;
    let runtime: std::path::PathBuf =
        serde_json::from_slice(&std::fs::read(root.join("runtime-path.json"))?)?;
    let ops = LocalOperations::new(RuntimeLayout::new(runtime)?);
    let server = ViewerServer::new(
        ViewerAdapter::new(ops),
        project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        AUTH.parse()?,
    )?;
    let f = Fixture {
        _temp: tempfile::tempdir()?,
        server,
        project,
    };
    let before = f.ops().canonical_basis(project)?;
    for run in 0..2 {
        let start = std::time::Instant::now();
        let response = f.raw(
            "/inspection?class=work&limit=7",
            &format!("Host: {AUTH}\r\n"),
        )?;
        let elapsed = start.elapsed();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert_eq!(response.matches("class=\"inspection-record").count(), 7);
        assert!(!response.contains("controlled private source"));
        println!(
            "HTML n=1000 run={run} elapsed_us={} http_bytes={}",
            elapsed.as_micros(),
            response.len()
        );
    }
    assert_eq!(before, f.ops().canonical_basis(project)?);
    Ok(())
}
