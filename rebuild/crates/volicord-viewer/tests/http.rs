use std::io::Cursor;
use tempfile::tempdir;
use volicord_context::{Clock, Principal, PrincipalKind, ProjectId, SystemClock, TimestampMicros};
use volicord_operations::{
    ConfirmationDecision, GuardedEffectCategory, GuardedEffectDraft, GuardedRisk, GuardedStore,
    LocalOperations, RequestingProvenance, RuntimeLayout,
};
use volicord_viewer::{ViewerAdapter, ViewerLocale, ViewerServer, ViewerView};

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
    operations
        .record_user_source(
            project,
            "test-host".into(),
            "test-session".into(),
            "Viewer mutation fixture".into(),
        )
        .expect("fixture Source");
    let server = ViewerServer::new(
        ViewerAdapter::new(operations),
        project,
        ViewerLocale::English,
        ViewerView::Overview,
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

fn new_guarded_request(
    server: &ViewerServer,
    project: ProjectId,
) -> volicord_operations::GuardedEffectCandidate {
    let now = SystemClock.now().expect("clock");
    server
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
        .expect("create Guarded request")
}

#[test]
fn guarded_http_fallback_preserves_exact_request_revision_and_source() {
    let (_temporary, server, project) = setup();
    let request = new_guarded_request(&server, project);
    let shown = exchange(
        &server,
        format!(
            "GET /guarded/{} HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n",
            request.confirmation_request_identity
        ),
    );
    assert!(shown.contains(&request.confirmation_request_identity.to_string()));
    assert!(shown.contains(&request.effect_fingerprint));
    assert!(shown.contains(&format!(
        "request_revision\" value=\"{}",
        request.request_revision
    )));
    assert!(!shown.contains("<nav"));
    assert!(!shown.contains("volicord-observation"));
    assert!(!shown.contains("No Work"));
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
                "confirmation_request_id={}&request_revision={}&effect_fingerprint={}&confirmation_decision=confirm&user_turn=Rejected+cross-site+response&guarded={}&request_authenticity={}",
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
                "confirmation_request_id={}&request_revision={}&effect_fingerprint={}&confirmation_decision=confirm&user_turn=Confirm+this+exact+release&guarded={}",
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
fn concurrent_guarded_http_responses_commit_one_source_and_preserve_unrelated_context() {
    let (_temporary, server, project) = setup();
    let operations = server.adapter().operations();
    let unrelated = operations
        .initialize_project("Unrelated confirmation control", None)
        .expect("unrelated Project")
        .project
        .id;
    let unrelated_before = operations
        .canonical_basis(unrelated)
        .expect("unrelated before");
    let request = new_guarded_request(&server, project);
    let before = operations.canonical_basis(project).expect("before");
    let runtime = operations.layout().root().to_path_buf();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles = (0..2).map(|_| {
        let runtime = runtime.clone();
        let request = request.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            let server = ViewerServer::new(
                ViewerAdapter::new(LocalOperations::new(RuntimeLayout::new(runtime).expect("layout"))),
                project, ViewerLocale::English, ViewerView::Overview, "en".into(),
                AUTHORITY.parse().expect("authority"),
            ).expect("server");
            let shown = exchange(&server, format!(
                "GET /guarded/{} HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n", request.confirmation_request_identity));
            let token = request_authenticity(&shown);
            let body = format!(
                "confirmation_request_id={}&request_revision={}&effect_fingerprint={}&confirmation_decision=confirm&user_turn=Confirm+the+exact+test+request&guarded={}",
                request.confirmation_request_identity, request.request_revision,
                request.effect_fingerprint, request.confirmation_request_identity,
            );
            barrier.wait();
            exchange(&server, post("/guarded/confirm", &body, &token))
        })
    }).collect::<Vec<_>>();
    let responses = handles
        .into_iter()
        .map(|h| h.join().expect("HTTP writer"))
        .collect::<Vec<_>>();
    assert_eq!(
        responses
            .iter()
            .filter(|r| r.starts_with("HTTP/1.1 303"))
            .count(),
        1
    );
    assert_eq!(
        responses
            .iter()
            .filter(|r| r
                .contains("a confirmation response already exists for this exact request revision"))
            .count(),
        1
    );
    let after = operations.canonical_basis(project).expect("after");
    assert_eq!(after.sources.len(), before.sources.len() + 1);
    let response = GuardedStore::open(operations.layout().guarded_store())
        .expect("store")
        .response(
            request.confirmation_request_identity,
            request.request_revision,
        )
        .expect("response")
        .expect("one committed response");
    assert!(after
        .sources
        .iter()
        .any(|s| s.source.id == response.user_response_source_id));
    assert_eq!(
        unrelated_before,
        operations
            .canonical_basis(unrelated)
            .expect("unrelated after")
    );
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
        format!("GET /?view=impossible HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
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

#[test]
fn retired_routes_never_read_or_mutate_even_with_old_forms() {
    let (_temp, server, project) = setup();
    let before = server
        .adapter()
        .operations()
        .canonical_basis(project)
        .expect("before");
    for path in [
        "/inspection",
        "/inspection.json",
        "/?view=tools",
        "/?view=decisions",
        "/?view=code&scope=repository",
    ] {
        let response = exchange(
            &server,
            format!("GET {path} HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
        );
        assert!(response.starts_with("HTTP/1.1 404"), "{response}");
        assert!(!response.contains("<nav"));
        assert!(!response.contains("request_authenticity"));
    }
    for path in [
        "/memory/context/correct",
        "/memory/decision/correct",
        "/memory/decision/supersede",
        "/memory/forget",
        "/documents/export",
    ] {
        for token in ["", "old-authenticity"] {
            let response = exchange(
                &server,
                post(path, "view=tools&tool=memory&user_turn=old+form", token),
            );
            assert!(response.starts_with("HTTP/1.1 404"), "{response}");
        }
    }
    assert_eq!(
        before,
        server
            .adapter()
            .operations()
            .canonical_basis(project)
            .expect("after")
    );
}

#[test]
fn work_reader_escapes_stored_text_and_keeps_host_origin_and_project_boundaries() {
    let (_temp, server, project) = setup();
    let work = server
        .adapter()
        .operations()
        .transition_work(
            project,
            volicord_operations::WorkTransition::StartNew {
                host: "test".into(),
                session: "escape".into(),
                user_turn: "Explain <script>stolen()</script>".into(),
                statement: "Explain <script>stolen()</script>".into(),
            },
        )
        .expect("Goal")
        .context_item_id;
    let before = server
        .adapter()
        .operations()
        .canonical_basis(project)
        .expect("before");
    let page = exchange(
        &server,
        format!("GET /?view=work&work={work} HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(page.contains("&lt;script&gt;stolen()&lt;/script&gt;"));
    assert!(!page.contains("<script>stolen()"));
    for (headers, expected) in [
        ("Host: foreign.invalid\r\n".to_owned(), 421),
        (
            format!("Host: {AUTHORITY}\r\nOrigin: https://foreign.invalid\r\n"),
            403,
        ),
        (
            format!("Host: {AUTHORITY}\r\nSec-Fetch-Site: cross-site\r\n"),
            403,
        ),
    ] {
        let response = exchange(
            &server,
            format!("GET /?view=work&work={work} HTTP/1.1\r\n{headers}\r\n"),
        );
        assert!(response.starts_with(&format!("HTTP/1.1 {expected}")));
        assert!(!response.contains("stolen"));
    }
    let foreign = server
        .adapter()
        .operations()
        .initialize_project("Foreign", None)
        .expect("Project")
        .project
        .id;
    let foreign_work = server
        .adapter()
        .operations()
        .transition_work(
            foreign,
            volicord_operations::WorkTransition::StartNew {
                host: "test".into(),
                session: "foreign".into(),
                user_turn: "private".into(),
                statement: "private".into(),
            },
        )
        .expect("foreign Goal")
        .context_item_id;
    let page = exchange(
        &server,
        format!("GET /?view=work&work={foreign_work} HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    assert!(page.starts_with("HTTP/1.1 404"));
    assert!(!page.contains("private"));
    assert_eq!(
        before,
        server
            .adapter()
            .operations()
            .canonical_basis(project)
            .expect("after")
    );
}
