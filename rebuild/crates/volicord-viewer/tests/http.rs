use rusqlite::{params, Connection};
use std::{
    collections::BTreeSet,
    io::Cursor,
    sync::{Arc, Barrier},
    thread,
};
use tempfile::tempdir;
use volicord_context::{Clock, Principal, PrincipalKind, ProjectId, SystemClock, TimestampMicros};
use volicord_inquiry::{
    CandidateCollectionMode, CandidateCollectionScope, CandidateContent, CandidateDraft,
    CandidateKind, CandidateObservationBasis, CandidateOrigin, CandidateRetention, CandidateStore,
    SubmissionOutcome,
};
use volicord_operations::{
    ConfirmationDecision, GuardedEffectCategory, GuardedEffectDraft, GuardedRisk, GuardedStore,
    LocalOperations, RequestingProvenance, RuntimeLayout,
};
use volicord_privacy::{
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedKind, ManagedDerivedState,
    PrivacyStore,
};
use volicord_viewer::{ExplanationLevel, ViewerAdapter, ViewerLocale, ViewerServer};

const AUTHORITY: &str = "127.0.0.1:3219";

fn setup() -> (tempfile::TempDir, ViewerServer, ProjectId) {
    let temporary = tempdir().expect("temporary directory");
    let repository = temporary.path().join("repository");
    std::fs::create_dir(&repository).expect("repository");
    let operations = LocalOperations::new(
        RuntimeLayout::new(temporary.path().join("runtime")).expect("runtime layout"),
    );
    let project = operations
        .initialize_project("HTTP Viewer", Some(&repository))
        .expect("initialize Project")
        .project
        .id;
    let server = ViewerServer::new(
        ViewerAdapter::new(operations),
        project,
        ViewerLocale::English,
        ExplanationLevel::Working,
        "en".into(),
        AUTHORITY.parse().expect("viewer authority"),
    )
    .expect("viewer server");
    (temporary, server, project)
}

fn exchange(server: &ViewerServer, request: impl AsRef<[u8]>) -> String {
    let mut input = Cursor::new(request.as_ref().to_vec());
    let mut output = Vec::new();
    server
        .serve_connection(&mut input, &mut output)
        .expect("serve HTTP request");
    String::from_utf8(output).expect("UTF-8 response")
}

fn post(path: &str, body: &str, request_authenticity: &str) -> String {
    let body = format!("{body}&request_authenticity={request_authenticity}");
    post_with_context(
        path,
        &body,
        Some(AUTHORITY),
        Some(&format!("http://{AUTHORITY}")),
        Some("same-origin"),
    )
}

fn post_with_context(
    path: &str,
    body: &str,
    host: Option<&str>,
    origin: Option<&str>,
    fetch_site: Option<&str>,
) -> String {
    let host = host.map_or(String::new(), |value| format!("Host: {value}\r\n"));
    let origin = origin.map_or(String::new(), |value| format!("Origin: {value}\r\n"));
    let fetch_site = fetch_site.map_or(String::new(), |value| {
        format!("Sec-Fetch-Site: {value}\r\n")
    });
    format!(
        "POST {path} HTTP/1.1\r\n{host}{origin}{fetch_site}Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn request_authenticity(response: &str) -> String {
    let marker = "name=\"request_authenticity\" value=\"";
    let start = response.find(marker).expect("request authenticity field") + marker.len();
    let end = response[start..]
        .find('"')
        .map(|offset| start + offset)
        .expect("request authenticity value end");
    response[start..end].to_owned()
}

#[test]
fn routes_each_request_with_its_own_depth_and_fresh_state() {
    let (_temporary, server, project) = setup();
    let overview = exchange(
        &server,
        format!(
            "GET /?level=overview&locale=en&language=fr-CA HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"
        ),
    );
    let deep = exchange(
        &server,
        format!("GET /?level=deep&locale=ko&language=%ED%95%9C%EA%B5%AD%EC%96%B4 HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    assert!(overview.starts_with("HTTP/1.1 200 OK"));
    assert!(overview.contains("data-explanation-level=\"overview\""));
    assert!(overview.contains("fr-CA"));
    assert!(deep.contains("data-explanation-level=\"deep\""));
    assert!(deep.contains("프로젝트 개요"));
    assert!(deep.contains("요청 언어 본문"));
    assert!(deep.contains("사용 불가"));
    assert!(!deep.contains("절대 대상 경로"));
    assert_ne!(overview, deep);

    let source = server
        .adapter()
        .operations()
        .record_user_source(
            project,
            "test-host".into(),
            "fresh-session".into(),
            "state added after the first request".into(),
        )
        .expect("record state between requests");
    assert!(!overview.contains(&source.identity));
    let refreshed = exchange(
        &server,
        format!("GET /?level=deep HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    assert!(refreshed.contains(&source.identity));
}

#[test]
fn forgetting_http_cleans_linked_local_content_and_preserves_unrelated() {
    let (_temporary, server, project) = setup();
    let page = exchange(
        &server,
        format!("GET /?level=deep HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    let request_authenticity = request_authenticity(&page);
    let source = server
        .adapter()
        .operations()
        .record_user_source(
            project,
            "test-host".into(),
            "memory-session".into(),
            "forget this memory".into(),
        )
        .expect("record forget target");
    let unrelated_source = server
        .adapter()
        .operations()
        .record_user_source(
            project,
            "test-host".into(),
            "memory-session".into(),
            "preserve this unrelated memory".into(),
        )
        .expect("record unrelated Source");
    let target_source_id = source_id(&source.identity);
    let unrelated_source_id = source_id(&unrelated_source.identity);
    let related_candidate =
        viewer_candidate(&server, project, target_source_id, "related Candidate");
    let unrelated_candidate =
        viewer_candidate(&server, project, unrelated_source_id, "unrelated Candidate");
    let mut privacy = PrivacyStore::open(server.adapter().operations().layout().privacy_store())
        .expect("privacy store");
    let related_derived = privacy
        .record_managed_derived(viewer_derived(project, target_source_id, "related Derived"))
        .expect("related Derived")
        .id;
    let unrelated_derived = privacy
        .record_managed_derived(viewer_derived(
            project,
            unrelated_source_id,
            "unrelated Derived",
        ))
        .expect("unrelated Derived")
        .id;
    drop(privacy);
    let response = exchange(
        &server,
        post(
            "/memory/forget",
            &format!(
                "record_kind=source&record_id={}&user_turn=Forget+this+exact+Source&level=deep&locale=en&language=en",
                source.identity
            ),
            &request_authenticity,
        ),
    );
    assert!(response.starts_with("HTTP/1.1 303 See Other"), "{response}");
    assert!(!response.contains(&request_authenticity));
    let canonical = server
        .adapter()
        .operations()
        .canonical_basis(project)
        .expect("canonical basis after forget");
    assert!(!canonical
        .sources
        .iter()
        .any(|basis| basis.source.id.to_string() == source.identity));
    assert!(canonical
        .sources
        .iter()
        .any(|basis| basis.source.id.to_string() == unrelated_source.identity));
    let candidates = CandidateStore::open(server.adapter().operations().layout().candidate_store())
        .expect("Candidate store");
    assert!(candidates
        .get(project, related_candidate)
        .expect("related Candidate")
        .content
        .is_none());
    assert!(candidates
        .get(project, unrelated_candidate)
        .expect("unrelated Candidate")
        .content
        .is_some());
    let privacy = PrivacyStore::open(server.adapter().operations().layout().privacy_store())
        .expect("privacy store");
    assert_eq!(
        privacy
            .get_derived(project, related_derived)
            .expect("related Derived")
            .state,
        ManagedDerivedState::Deleted
    );
    assert_eq!(
        privacy
            .get_derived(project, unrelated_derived)
            .expect("unrelated Derived")
            .state,
        ManagedDerivedState::Current
    );

    let refreshed = exchange(
        &server,
        format!("GET /?level=deep HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    assert!(!refreshed.contains(&format!("Source · {} r1", source.identity)));
}

#[test]
fn viewer_read_withholds_forgotten_content_during_repair_required_cleanup() {
    const RELATED: &str = "VIEWER-LIVE-FORGET-CANDIDATE-78c2";
    const UNRELATED: &str = "VIEWER-LIVE-KEEP-CANDIDATE-51af";
    let (_temporary, server, project) = setup();
    let initial = exchange(
        &server,
        format!("GET /?level=deep HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    let token = request_authenticity(&initial);
    let target = server
        .adapter()
        .operations()
        .record_user_source(
            project,
            "test-host".into(),
            "viewer-live-read".into(),
            "viewer live forgetting target".into(),
        )
        .expect("forget target");
    let unrelated_source = server
        .adapter()
        .operations()
        .record_user_source(
            project,
            "test-host".into(),
            "viewer-live-read".into(),
            "viewer live unrelated source".into(),
        )
        .expect("unrelated Source");
    let target_id = source_id(&target.identity);
    let unrelated_source_id = source_id(&unrelated_source.identity);
    let related_candidate = viewer_candidate(&server, project, target_id, RELATED);
    let unrelated_candidate = viewer_candidate(&server, project, unrelated_source_id, UNRELATED);
    let mut privacy = PrivacyStore::open(server.adapter().operations().layout().privacy_store())
        .expect("privacy store");
    let related_derived = privacy
        .record_managed_derived(viewer_derived(
            project,
            target_id,
            "viewer live related Derived",
        ))
        .expect("related Derived")
        .id;
    let unrelated_derived = privacy
        .record_managed_derived(viewer_derived(
            project,
            unrelated_source_id,
            "viewer live unrelated Derived",
        ))
        .expect("unrelated Derived")
        .id;
    drop(privacy);

    let completed_before_commit = exchange(
        &server,
        format!("GET /?level=deep HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    assert!(completed_before_commit.starts_with("HTTP/1.1 200 OK"));
    assert!(completed_before_commit.contains(RELATED));
    assert!(completed_before_commit.contains(UNRELATED));

    let candidate_path = server.adapter().operations().layout().candidate_store();
    let blocker = Connection::open(candidate_path).expect("Candidate reader");
    blocker
        .execute_batch("BEGIN DEFERRED")
        .expect("begin pre-commit reader");
    let precommit_snapshot: String = blocker
        .query_row(
            "SELECT record_json FROM candidates WHERE id = ?1",
            params![related_candidate.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .expect("read Candidate before canonical commit");
    assert!(precommit_snapshot.contains(RELATED));

    let conflict = exchange(
        &server,
        post(
            "/memory/forget",
            &format!(
                "record_kind=source&record_id={}&user_turn=Forget+the+live+Viewer+target&level=deep&locale=en&language=en",
                target.identity
            ),
            &token,
        ),
    );
    assert!(conflict.starts_with("HTTP/1.1 409 Conflict"), "{conflict}");
    let canonical = server
        .adapter()
        .operations()
        .canonical_basis(project)
        .expect("canonical basis after committed forgetting");
    assert!(!canonical
        .sources
        .iter()
        .any(|source| source.source.id == target_id));
    assert!(canonical
        .sources
        .iter()
        .any(|source| source.source.id == unrelated_source_id));

    let still_precommit_snapshot: String = blocker
        .query_row(
            "SELECT record_json FROM candidates WHERE id = ?1",
            params![related_candidate.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .expect("complete pre-commit Candidate snapshot");
    assert_eq!(still_precommit_snapshot, precommit_snapshot);
    assert!(completed_before_commit.contains(RELATED));

    let live_read = exchange(
        &server,
        format!("GET /?level=deep HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    assert!(live_read.starts_with("HTTP/1.1 200 OK"));
    assert!(!live_read.contains(RELATED));
    assert!(!live_read.contains(&target.identity));
    assert!(live_read.contains(UNRELATED));
    assert!(live_read.contains(&unrelated_source.identity));
    assert!(live_read.contains("Candidate data omitted"));

    let projection = server
        .adapter()
        .operations()
        .project_projection(project)
        .expect("live project projection");
    assert!(projection
        .candidate_inspection
        .iter()
        .find(|candidate| candidate.candidate_id == related_candidate)
        .is_some_and(|candidate| candidate.bounded_summary.is_none()));
    assert_eq!(
        projection
            .candidate_inspection
            .iter()
            .find(|candidate| candidate.candidate_id == unrelated_candidate)
            .and_then(|candidate| candidate.bounded_summary.as_deref()),
        Some(UNRELATED)
    );
    let privacy = server
        .adapter()
        .operations()
        .privacy_status(project)
        .expect("privacy status during repair");
    assert!(privacy
        .withheld_for_canonical_forgetting
        .contains(&related_derived));
    assert!(!privacy
        .withheld_for_canonical_forgetting
        .contains(&unrelated_derived));

    blocker.execute_batch("ROLLBACK").expect("release reader");
}

#[test]
fn concurrent_viewer_forgetting_preserves_cleanup_and_unrelated_controls() {
    const WRITERS: usize = 4;
    let (_temporary, server, project) = setup();
    let runtime = server.adapter().operations().layout().root().to_path_buf();
    let unrelated_source = server
        .adapter()
        .operations()
        .record_user_source(
            project,
            "test-host".into(),
            "viewer-concurrency".into(),
            "preserve unrelated viewer control".into(),
        )
        .expect("unrelated Source");
    let unrelated_source_id = source_id(&unrelated_source.identity);
    let unrelated_candidate = viewer_candidate(
        &server,
        project,
        unrelated_source_id,
        "unrelated concurrent Candidate",
    );
    let mut privacy = PrivacyStore::open(server.adapter().operations().layout().privacy_store())
        .expect("privacy store");
    let unrelated_derived = privacy
        .record_managed_derived(viewer_derived(
            project,
            unrelated_source_id,
            "unrelated concurrent Derived",
        ))
        .expect("unrelated Derived")
        .id;
    drop(privacy);

    let mut controls = Vec::new();
    for index in 0..WRITERS {
        let operations = LocalOperations::new(RuntimeLayout::new(&runtime).expect("runtime"));
        let worker = ViewerServer::new(
            ViewerAdapter::new(operations),
            project,
            ViewerLocale::English,
            ExplanationLevel::Working,
            "en".into(),
            AUTHORITY.parse().expect("viewer authority"),
        )
        .expect("worker viewer");
        let target = worker
            .adapter()
            .operations()
            .record_user_source(
                project,
                "test-host".into(),
                "viewer-concurrency".into(),
                format!("forget concurrent viewer target {index}"),
            )
            .expect("forget target");
        let target_id = source_id(&target.identity);
        let candidate = viewer_candidate(
            &worker,
            project,
            target_id,
            &format!("concurrent Candidate {index}"),
        );
        let mut privacy =
            PrivacyStore::open(worker.adapter().operations().layout().privacy_store())
                .expect("privacy store");
        let derived = privacy
            .record_managed_derived(viewer_derived(
                project,
                target_id,
                &format!("concurrent Derived {index}"),
            ))
            .expect("managed Derived")
            .id;
        drop(privacy);
        let page = exchange(
            &worker,
            format!("GET / HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
        );
        let token = request_authenticity(&page);
        controls.push((worker, token, target.identity, candidate, derived));
    }
    let barrier = Arc::new(Barrier::new(WRITERS));
    let writers = controls
        .into_iter()
        .map(|(worker, token, target, candidate, derived)| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let response = exchange(
                    &worker,
                    post(
                        "/memory/forget",
                        &format!(
                            "record_kind=source&record_id={target}&user_turn=Forget+this+concurrent+Viewer+target",
                        ),
                        &token,
                    ),
                );
                assert!(response.starts_with("HTTP/1.1 303 See Other"), "{response}");
                (target, candidate, derived)
            })
        })
        .collect::<Vec<_>>();
    let controls = writers
        .into_iter()
        .map(|writer| writer.join().expect("Viewer writer"))
        .collect::<Vec<_>>();

    let restarted = LocalOperations::new(RuntimeLayout::new(runtime).expect("runtime"));
    let canonical = restarted
        .canonical_basis(project)
        .expect("canonical after concurrent Viewer forgetting");
    let remaining_sources = canonical
        .sources
        .iter()
        .map(|basis| basis.source.id.to_string())
        .collect::<BTreeSet<_>>();
    assert!(remaining_sources.contains(&unrelated_source.identity));
    assert!(controls
        .iter()
        .all(|(target, _, _)| !remaining_sources.contains(target)));
    let candidates = CandidateStore::open(restarted.layout().candidate_store())
        .expect("Candidate store after restart");
    assert!(controls.iter().all(|(_, candidate, _)| candidates
        .get(project, *candidate)
        .expect("related Candidate")
        .content
        .is_none()));
    assert!(candidates
        .get(project, unrelated_candidate)
        .expect("unrelated Candidate")
        .content
        .is_some());
    let privacy =
        PrivacyStore::open(restarted.layout().privacy_store()).expect("privacy after restart");
    assert!(controls.iter().all(|(_, _, derived)| privacy
        .get_derived(project, *derived)
        .expect("related Derived")
        .state
        == ManagedDerivedState::Deleted));
    assert_eq!(
        privacy
            .get_derived(project, unrelated_derived)
            .expect("unrelated Derived")
            .state,
        ManagedDerivedState::Current
    );
}

fn source_id(value: &str) -> volicord_context::SourceId {
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] =
            u8::from_str_radix(std::str::from_utf8(pair).expect("hex pair"), 16).expect("hex");
    }
    volicord_context::SourceId::from_bytes(bytes)
}

fn viewer_candidate(
    server: &ViewerServer,
    project_id: ProjectId,
    source_id: volicord_context::SourceId,
    summary: &str,
) -> volicord_inquiry::CandidateId {
    match server
        .adapter()
        .operations()
        .submit_candidate(CandidateDraft {
            project_id,
            kind: CandidateKind::Observation,
            collection_mode: CandidateCollectionMode::Automatic,
            origin: CandidateOrigin {
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "viewer-test-agent".into(),
                },
                subsystem: "viewer-forgetting-test".into(),
                session: Some("viewer-forgetting".into()),
                provenance_summary: "viewer forgetting fixture".into(),
            },
            collection_scope: CandidateCollectionScope {
                project_id,
                session: Some("viewer-forgetting".into()),
                source_operation: Some("fixture".into()),
                candidate_kind: CandidateKind::Observation,
            },
            observation_basis: CandidateObservationBasis {
                source_basis: vec![source_id],
                ..CandidateObservationBasis::default()
            },
            observed_at: TimestampMicros::from_unix_micros(1),
            retention: CandidateRetention {
                retained_until: None,
                basis: "retain for viewer forgetting test".into(),
            },
            content: CandidateContent {
                bounded_summary: summary.into(),
                question: None,
                materiality_review: None,
            },
        })
        .expect("submit Candidate")
    {
        SubmissionOutcome::Stored(candidate) => candidate.id,
        SubmissionOutcome::CollectionDisabled { .. } => panic!("Candidate collection disabled"),
    }
}

fn viewer_derived(
    project_id: ProjectId,
    source_id: volicord_context::SourceId,
    content: &str,
) -> ManagedDerivedDraft {
    ManagedDerivedDraft {
        project_id,
        kind: ManagedDerivedKind::CachedSummary,
        provider: None,
        model: None,
        purpose: "viewer forgetting fixture".into(),
        analysis_snapshot: None,
        included_sources: Vec::new(),
        canonical_links: vec![ManagedCanonicalLink::Source(source_id)],
        content: content.into(),
        uncertainty: None,
        retained_until: None,
        retention_basis: "rebuildable fixture".into(),
    }
}

#[test]
fn guarded_http_fallback_preserves_exact_request_revision_and_source() {
    let (_temporary, server, project) = setup();
    let now = SystemClock.now().expect("clock");
    let request = server
        .adapter()
        .operations()
        .create_guarded_request(GuardedEffectDraft {
            project_id: project,
            exact_action: "publish release".into(),
            target: "registry/example".into(),
            expected_effect: "public release".into(),
            risk: GuardedRisk {
                category: GuardedEffectCategory::ExternalDeploymentOrPublicPublication,
                concrete_consequence: "public artifact".into(),
            },
            scope: vec!["release:example".into()],
            expires_at: TimestampMicros::from_unix_micros(now.as_unix_micros() + 60_000_000),
            requesting_provenance: RequestingProvenance {
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "test-agent".into(),
                },
                host: Some("codex".into()),
                session: Some("guarded-http".into()),
                basis: vec!["test".into()],
            },
        })
        .expect("create Guarded request");
    let shown = exchange(
        &server,
        format!(
            "GET /guarded/{}?level=working HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n",
            request.confirmation_request_identity
        ),
    );
    assert!(shown.contains(&request.confirmation_request_identity.to_string()));
    assert!(shown.contains(&request.effect_fingerprint));
    assert!(shown.contains(&format!(
        "request_revision\" value=\"{}",
        request.request_revision
    )));
    let request_authenticity = request_authenticity(&shown);
    let before_rejection = server
        .adapter()
        .operations()
        .canonical_basis(project)
        .expect("canonical basis before rejected Guarded response");
    let rejected = exchange(
        &server,
        post_with_context(
            "/guarded/confirm",
            &format!(
                "confirmation_request_id={}&request_revision={}&effect_fingerprint={}&decision=confirm&user_turn=Rejected+cross-site+response&guarded={}&request_authenticity={}",
                request.confirmation_request_identity,
                request.request_revision,
                request.effect_fingerprint,
                request.confirmation_request_identity,
                request_authenticity,
            ),
            Some(AUTHORITY),
            Some(&format!("http://{AUTHORITY}")),
            Some("cross-site"),
        ),
    );
    assert!(rejected.starts_with("HTTP/1.1 403 Forbidden"), "{rejected}");
    assert_eq!(
        server
            .adapter()
            .operations()
            .canonical_basis(project)
            .expect("canonical basis after rejected Guarded response"),
        before_rejection
    );
    assert!(
        GuardedStore::open(server.adapter().operations().layout().guarded_store())
            .expect("open Guarded store after rejection")
            .response(
                request.confirmation_request_identity,
                request.request_revision,
            )
            .expect("read rejected Guarded response")
            .is_none()
    );

    let response = exchange(
        &server,
        post(
            "/guarded/confirm",
            &format!(
                "confirmation_request_id={}&request_revision={}&effect_fingerprint={}&decision=confirm&user_turn=Confirm+this+exact+release&guarded={}",
                request.confirmation_request_identity,
                request.request_revision,
                request.effect_fingerprint,
                request.confirmation_request_identity,
            ),
            &request_authenticity,
        ),
    );
    assert!(response.starts_with("HTTP/1.1 303 See Other"), "{response}");
    let stored = GuardedStore::open(server.adapter().operations().layout().guarded_store())
        .expect("open Guarded store")
        .response(
            request.confirmation_request_identity,
            request.request_revision,
        )
        .expect("read exact response")
        .expect("stored response");
    assert_eq!(stored.decision, ConfirmationDecision::Confirmed);
    assert_eq!(stored.effect_fingerprint, request.effect_fingerprint);
    assert_eq!(stored.request_revision, request.request_revision);
    let canonical = server
        .adapter()
        .operations()
        .canonical_basis(project)
        .expect("canonical basis");
    assert!(canonical
        .sources
        .iter()
        .any(|basis| basis.source.id == stored.user_response_source_id));
}

#[test]
fn authority_origin_fetch_metadata_and_token_fail_before_memory_side_effects() {
    let (temporary, server, project) = setup();
    let target = server
        .adapter()
        .operations()
        .record_user_source(
            project,
            "test-host".into(),
            "authenticity-session".into(),
            "memory target".into(),
        )
        .expect("record memory target");
    let page = exchange(
        &server,
        format!("GET /?level=deep HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    let request_authenticity = request_authenticity(&page);
    assert_eq!(request_authenticity.len(), 64);
    let rebound = exchange(
        &server,
        "GET /?level=deep HTTP/1.1\r\nHost: attacker.example\r\n\r\n",
    );
    assert!(rebound.starts_with("HTTP/1.1 421 Misdirected Request"));
    assert!(!rebound.contains("request_authenticity"));

    let before = server
        .adapter()
        .operations()
        .canonical_basis(project)
        .expect("canonical basis before rejected mutations");
    let mutation = format!(
        "record_kind=source&record_id={}&user_turn=Forget+the+target&request_authenticity={}",
        target.identity, request_authenticity
    );
    let cases = [
        post_with_context(
            "/memory/forget",
            &mutation,
            Some("attacker.example"),
            Some(&format!("http://{AUTHORITY}")),
            Some("same-origin"),
        ),
        post_with_context(
            "/memory/forget",
            &mutation,
            Some(AUTHORITY),
            None,
            Some("same-origin"),
        ),
        post_with_context(
            "/memory/forget",
            &mutation,
            Some(AUTHORITY),
            Some("http://attacker.example"),
            Some("same-origin"),
        ),
        post_with_context(
            "/memory/forget",
            &mutation,
            Some(AUTHORITY),
            Some(&format!("http://{AUTHORITY}")),
            Some("cross-site"),
        ),
        post_with_context(
            "/memory/forget",
            &format!(
                "record_kind=source&record_id={}&user_turn=Forget+without+token",
                target.identity
            ),
            Some(AUTHORITY),
            Some(&format!("http://{AUTHORITY}")),
            Some("same-origin"),
        ),
        post_with_context(
            "/memory/forget",
            &format!(
                "record_kind=source&record_id={}&user_turn=Forget+with+wrong+token&request_authenticity={}",
                target.identity,
                "00".repeat(32)
            ),
            Some(AUTHORITY),
            Some(&format!("http://{AUTHORITY}")),
            Some("same-origin"),
        ),
    ];
    for (index, request) in cases.into_iter().enumerate() {
        let response = exchange(&server, request);
        let expected = if index == 0 {
            "HTTP/1.1 421 Misdirected Request"
        } else {
            "HTTP/1.1 403 Forbidden"
        };
        assert!(response.starts_with(expected), "{index}: {response}");
    }
    assert_eq!(
        server
            .adapter()
            .operations()
            .canonical_basis(project)
            .expect("canonical basis after rejected mutations"),
        before
    );
    let bundle = temporary.path().join("authenticity.volicord.json");
    server
        .adapter()
        .operations()
        .export_bundle(project, &bundle)
        .expect("export portable context");
    assert!(
        !String::from_utf8(std::fs::read(bundle).expect("read bundle"))
            .expect("bundle UTF-8")
            .contains(&request_authenticity)
    );
}

#[test]
fn rejected_document_export_has_no_filesystem_effect_and_authenticated_export_works() {
    let (temporary, server, _project) = setup();
    let page = exchange(
        &server,
        format!("GET /?level=working HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    let request_authenticity = request_authenticity(&page);
    let destination = temporary.path().join("new-parent").join("handoff.md");
    let body = format!(
        "kind=handoff-resume&format=markdown&destination={}&level=working&locale=en&language=en&request_authenticity={}",
        destination.display(),
        request_authenticity
    );
    let rejected = exchange(
        &server,
        post_with_context(
            "/documents/export",
            &body,
            Some(AUTHORITY),
            Some("http://attacker.example"),
            Some("cross-site"),
        ),
    );
    assert!(rejected.starts_with("HTTP/1.1 403 Forbidden"));
    assert!(!destination.exists());
    assert!(!destination.parent().expect("destination parent").exists());

    let accepted = exchange(
        &server,
        post_with_context(
            "/documents/export",
            &body,
            Some(AUTHORITY),
            Some(&format!("http://{AUTHORITY}")),
            Some("same-origin"),
        ),
    );
    assert!(accepted.starts_with("HTTP/1.1 303 See Other"), "{accepted}");
    assert!(destination.is_file());
    assert!(!accepted.contains(&request_authenticity));
    assert!(!std::fs::read_to_string(destination)
        .expect("read generated document")
        .contains(&request_authenticity));
}

#[test]
fn malformed_unknown_and_oversized_requests_fail_without_mutation() {
    let (_temporary, server, project) = setup();
    let before = server
        .adapter()
        .operations()
        .canonical_basis(project)
        .expect("basis before malformed requests");
    let malformed = exchange(
        &server,
        format!("GET /?level=impossible HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    assert!(malformed.starts_with("HTTP/1.1 400 Bad Request"));
    let unknown = exchange(
        &server,
        format!("GET /not-a-view HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    assert!(unknown.starts_with("HTTP/1.1 404 Not Found"));
    let method = exchange(
        &server,
        format!("PUT / HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    assert!(method.starts_with("HTTP/1.1 405 Method Not Allowed"));
    let oversized = exchange(
        &server,
        format!("POST /memory/forget HTTP/1.1\r\nHost: {AUTHORITY}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 65537\r\n\r\n"),
    );
    assert!(oversized.starts_with("HTTP/1.1 413 Payload Too Large"));
    let after = server
        .adapter()
        .operations()
        .canonical_basis(project)
        .expect("basis after malformed requests");
    assert_eq!(before, after);
}
