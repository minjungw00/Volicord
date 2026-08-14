use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};
use tempfile::TempDir;
use volicord_context::{
    ApplicabilityScope, Availability, Clock, ContextItemDraft, ContextItemRole, OperationId,
    Principal, PrincipalKind, SourceDraft, SourcePayload, StatementProvenanceRole, Store,
};
use volicord_operations::{
    ConfirmationDecision, GuardedEffectCategory, GuardedEffectDraft, GuardedRisk, GuardedStore,
    LocalOperations, RequestingProvenance, RuntimeLayout,
};

struct ViewerProcess {
    child: Child,
}

impl Drop for ViewerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn reserve_loopback_address() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve loopback port");
    listener.local_addr().expect("loopback address").to_string()
}

fn start_viewer(runtime: &Path, project: &str, address: &str) -> ViewerProcess {
    let child = Command::new(env!("CARGO_BIN_EXE_volicord-viewer"))
        .args([
            "--runtime",
            runtime.to_str().expect("runtime path"),
            "--project",
            project,
            "--bind",
            address,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn real viewer executable");
    for _ in 0..100 {
        if let Ok(mut stream) = TcpStream::connect(address) {
            let request = format!(
                "GET /?level=overview HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
            );
            if stream.write_all(request.as_bytes()).is_ok() {
                let mut response = String::new();
                if stream.read_to_string(&mut response).is_ok()
                    && response.starts_with("HTTP/1.1 200 OK")
                {
                    return ViewerProcess { child };
                }
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("real viewer executable did not open its listener");
}

fn exchange(address: &str, request: &str) -> String {
    let mut stream = TcpStream::connect(address).expect("connect to viewer listener");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("bound response read");
    stream
        .write_all(request.as_bytes())
        .expect("write HTTP request");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read complete HTTP response");
    response
}

fn get(address: &str, target: &str) -> String {
    exchange(
        address,
        &format!("GET {target} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"),
    )
}

fn post(address: &str, target: &str, body: &str, request_authenticity: &str) -> String {
    let body = format!("{body}&request_authenticity={request_authenticity}");
    exchange(
        address,
        &format!(
            "POST {target} HTTP/1.1\r\nHost: {address}\r\nOrigin: http://{address}\r\nSec-Fetch-Site: same-origin\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
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

fn export(
    operations: &LocalOperations,
    project: volicord_context::ProjectId,
    path: &Path,
) -> Vec<u8> {
    operations
        .export_bundle(project, path)
        .expect("export canonical bundle");
    fs::read(path).expect("read canonical bundle")
}

#[test]
fn real_listener_is_live_mutable_strict_and_exact_for_guarded_fallback() {
    let temporary = TempDir::new().expect("temporary root");
    let runtime = temporary.path().join("runtime");
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository).expect("repository");
    fs::write(repository.join("main.py"), "VALUE = 1\n").expect("fixture source");
    let operations = LocalOperations::new(RuntimeLayout::new(&runtime).expect("runtime layout"));
    let project = operations
        .initialize_project("Executable HTTP", Some(&repository))
        .expect("initialize Project")
        .project
        .id;
    operations
        .analyze(project, Vec::new())
        .expect("analyze fixture");

    let address = reserve_loopback_address();
    let _viewer = start_viewer(&runtime, &project.to_string(), &address);

    let overview = get(&address, "/?level=overview&locale=en&language=fr-CA");
    let working = get(&address, "/?level=working&locale=ko&language=ko");
    let deep = get(&address, "/?level=deep&locale=en&language=ja");
    assert!(overview.starts_with("HTTP/1.1 200 OK"), "{overview}");
    assert!(overview.contains("data-explanation-level=\"overview\""));
    assert!(working.contains("data-explanation-level=\"working\""));
    assert!(working.contains("프로젝트 개요"));
    assert!(deep.contains("data-explanation-level=\"deep\""));
    assert!(deep.contains("main.py"));
    assert_ne!(overview, working);
    assert_ne!(working, deep);
    let request_authenticity = request_authenticity(&overview);
    let rebound = exchange(
        &address,
        "GET /?level=deep HTTP/1.1\r\nHost: attacker.example\r\nConnection: close\r\n\r\n",
    );
    assert!(rebound.starts_with("HTTP/1.1 421 Misdirected Request"));
    assert!(!rebound.contains("request_authenticity"));

    let mut store = Store::open(operations.layout().canonical_store()).expect("canonical store");
    let project_revision = store.get_project(project).expect("Project").revision;
    let basis = store
        .record_source(
            OperationId::from_bytes([81; 16]),
            project,
            SourceDraft {
                expected_project_revision: project_revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "fixture-host".into(),
                    session: "after-viewer-start".into(),
                    turn: "remember the live viewer constraint".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "fixture-user".into(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "viewer-executable-test".into(),
                }),
                availability: Availability::Available,
            },
        )
        .expect("record source after launch")
        .value;
    let context = store
        .record_context_item(
            OperationId::from_bytes([82; 16]),
            project,
            ContextItemDraft {
                expected_project_revision: project_revision,
                role: ContextItemRole::Constraint,
                statement: "viewer state created after startup".into(),
                provenance_role: StatementProvenanceRole::UserStatement,
                author: basis.actor.clone(),
                source_basis: vec![basis.id],
                applicability: ApplicabilityScope::default(),
            },
        )
        .expect("record context after launch")
        .value;
    drop(store);
    assert!(!overview.contains(&context.id.to_string()));
    let refreshed = get(&address, "/?level=deep&locale=en&language=en");
    assert!(refreshed.contains(&context.id.to_string()));
    assert!(refreshed.contains("viewer state created after startup"));

    let corrected = post(
        &address,
        "/memory/context/correct",
        &format!(
            "record_id={}&expected_revision={}&corrected_text=state+created+after+viewer+startup&user_turn=Correct+this+viewer+memory&level=deep&locale=en&language=en",
            context.id, context.revision
        ),
        &request_authenticity,
    );
    assert!(
        corrected.starts_with("HTTP/1.1 303 See Other"),
        "{corrected}"
    );
    let independent = LocalOperations::new(RuntimeLayout::new(&runtime).expect("second layout"));
    let canonical = independent
        .canonical_basis(project)
        .expect("independent read");
    let corrected_context = canonical
        .context_items
        .iter()
        .find(|item| item.id == context.id)
        .expect("corrected Context Item");
    assert_eq!(corrected_context.revision, 2);
    assert_eq!(
        corrected_context.statement,
        "state created after viewer startup"
    );
    assert!(get(&address, "/?level=deep").contains("state created after viewer startup"));

    let before_invalid = export(
        &independent,
        project,
        &temporary.path().join("before-invalid.json"),
    );
    assert!(get(&address, "/?level=unsupported").starts_with("HTTP/1.1 400 Bad Request"));
    assert!(get(&address, "/missing").starts_with("HTTP/1.1 404 Not Found"));
    assert!(post(
        &address,
        "/memory/forget",
        "record_kind=source&record_id=00000000000000000000000000000000&user_turn=no&unexpected=true",
        &request_authenticity,
    )
    .starts_with("HTTP/1.1 400 Bad Request"));
    let after_invalid = export(
        &independent,
        project,
        &temporary.path().join("after-invalid.json"),
    );
    assert_eq!(before_invalid, after_invalid);

    let now = volicord_context::SystemClock
        .now()
        .expect("clock")
        .as_unix_micros();
    let draft = |expected_effect: &str| GuardedEffectDraft {
        project_id: project,
        exact_action: "publish release".into(),
        target: "registry/example".into(),
        expected_effect: expected_effect.into(),
        risk: GuardedRisk {
            category: GuardedEffectCategory::ExternalDeploymentOrPublicPublication,
            concrete_consequence: "public artifact".into(),
        },
        scope: vec!["release:example".into()],
        expires_at: volicord_context::TimestampMicros::from_unix_micros(now + 600_000_000),
        requesting_provenance: RequestingProvenance {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "codex".into(),
            },
            host: Some("codex".into()),
            session: Some("viewer-fallback".into()),
            basis: vec!["executable listener validation".into()],
        },
    };
    let first = independent
        .create_guarded_request(draft("public release v1"))
        .expect("initial Guarded request");
    let current = independent
        .revise_guarded_request(
            first.confirmation_request_identity,
            first.request_revision,
            draft("public release v2"),
        )
        .expect("revised Guarded request");
    let shown = get(
        &address,
        &format!(
            "/guarded/{}?level=working",
            current.confirmation_request_identity
        ),
    );
    assert!(shown.contains(&current.confirmation_request_identity.to_string()));
    assert!(shown.contains(&current.effect_fingerprint));
    assert!(shown.contains(&format!(
        "request_revision\" value=\"{}",
        current.request_revision
    )));

    let submit = |revision: u64, fingerprint: &str, turn: &str| {
        post(
            &address,
            "/guarded/confirm",
            &format!(
                "confirmation_request_id={}&request_revision={revision}&effect_fingerprint={fingerprint}&decision=confirm&user_turn={turn}&guarded={}",
                current.confirmation_request_identity, current.confirmation_request_identity
            ),
            &request_authenticity,
        )
    };
    assert!(submit(
        first.request_revision,
        &first.effect_fingerprint,
        "stale+response"
    )
    .starts_with("HTTP/1.1 422 Unprocessable Content"));
    assert!(submit(
        current.request_revision,
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "mismatched+response",
    )
    .starts_with("HTTP/1.1 422 Unprocessable Content"));
    let accepted = submit(
        current.request_revision,
        &current.effect_fingerprint,
        "confirm+this+exact+revision",
    );
    assert!(accepted.starts_with("HTTP/1.1 303 See Other"), "{accepted}");
    assert!(submit(
        current.request_revision,
        &current.effect_fingerprint,
        "reused+response",
    )
    .starts_with("HTTP/1.1 422 Unprocessable Content"));

    let guarded = GuardedStore::open(independent.layout().guarded_store()).expect("Guarded store");
    assert!(guarded
        .response(first.confirmation_request_identity, first.request_revision)
        .expect("old response read")
        .is_none());
    let response = guarded
        .response(
            current.confirmation_request_identity,
            current.request_revision,
        )
        .expect("current response read")
        .expect("accepted exact response");
    assert_eq!(response.decision, ConfirmationDecision::Confirmed);
    assert_eq!(
        response.confirmation_request_identity,
        current.confirmation_request_identity
    );
    assert_eq!(response.request_revision, current.request_revision);
    assert_eq!(response.effect_fingerprint, current.effect_fingerprint);
    assert!(independent
        .canonical_basis(project)
        .expect("response Source read")
        .sources
        .iter()
        .any(|source| source.source.id == response.user_response_source_id));
}
