use std::io::Cursor;
use tempfile::tempdir;
use volicord_context::{Clock, Principal, PrincipalKind, ProjectId, SystemClock, TimestampMicros};
use volicord_operations::{
    ConfirmationDecision, GuardedEffectCategory, GuardedEffectDraft, GuardedRisk, GuardedStore,
    LocalOperations, RequestingProvenance, RuntimeLayout,
};
use volicord_viewer::{ExplanationLevel, ViewerAdapter, ViewerLocale, ViewerServer};

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
    );
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

fn post(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

#[test]
fn routes_each_request_with_its_own_depth_and_fresh_state() {
    let (_temporary, server, project) = setup();
    let overview = exchange(
        &server,
        "GET /?level=overview&locale=en&language=fr-CA HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
    );
    let deep = exchange(
        &server,
        "GET /?level=deep&locale=ko&language=%ED%95%9C%EA%B5%AD%EC%96%B4 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
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
        "GET /?level=deep HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
    );
    assert!(refreshed.contains(&source.identity));
}

#[test]
fn post_routes_one_real_memory_mutation_through_local_operations() {
    let (_temporary, server, project) = setup();
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
        ),
    );
    assert!(response.starts_with("HTTP/1.1 303 See Other"), "{response}");
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
        "GET /?level=deep HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
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
            "GET /guarded/{}?level=working HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            request.confirmation_request_identity
        ),
    );
    assert!(shown.contains(&request.confirmation_request_identity.to_string()));
    assert!(shown.contains(&request.effect_fingerprint));
    assert!(shown.contains(&format!(
        "request_revision\" value=\"{}",
        request.request_revision
    )));

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
fn malformed_unknown_and_oversized_requests_fail_without_mutation() {
    let (_temporary, server, project) = setup();
    let before = server
        .adapter()
        .operations()
        .canonical_basis(project)
        .expect("basis before malformed requests");
    let malformed = exchange(
        &server,
        "GET /?level=impossible HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
    );
    assert!(malformed.starts_with("HTTP/1.1 400 Bad Request"));
    let unknown = exchange(
        &server,
        "GET /not-a-view HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
    );
    assert!(unknown.starts_with("HTTP/1.1 404 Not Found"));
    let method = exchange(&server, "PUT / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    assert!(method.starts_with("HTTP/1.1 405 Method Not Allowed"));
    let oversized = exchange(
        &server,
        "POST /memory/forget HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 65537\r\n\r\n",
    );
    assert!(oversized.starts_with("HTTP/1.1 413 Payload Too Large"));
    let after = server
        .adapter()
        .operations()
        .canonical_basis(project)
        .expect("basis after malformed requests");
    assert_eq!(before, after);
}
