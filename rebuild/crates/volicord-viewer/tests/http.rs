use std::io::Cursor;
use tempfile::tempdir;
use volicord_context::{Clock, Principal, PrincipalKind, ProjectId, SystemClock, TimestampMicros};
use volicord_operations::{
    ConfirmationDecision, GuardedEffectCategory, GuardedEffectDraft, GuardedRisk, GuardedStore,
    LocalOperations, RequestingProvenance, RuntimeLayout,
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
    assert!(deep.contains("절대 대상 경로"));
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
fn post_routes_one_real_memory_mutation_through_local_operations() {
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

    let refreshed = exchange(
        &server,
        format!("GET /?level=deep HTTP/1.1\r\nHost: {AUTHORITY}\r\n\r\n"),
    );
    assert!(!refreshed.contains(&format!("Source · {} r1", source.identity)));
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
