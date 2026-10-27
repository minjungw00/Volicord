use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};
use tempfile::TempDir;
use volicord_context::{Clock, Principal, PrincipalKind, SystemClock, TimestampMicros};
use volicord_operations::{
    ConfirmationDecision, GuardedEffectCategory, GuardedEffectDraft, GuardedRisk, GuardedStore,
    LocalOperations, RequestingProvenance, RuntimeLayout,
};

struct ViewerProcess {
    child: Child,
    address: String,
}

impl Drop for ViewerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start_viewer(runtime: &Path, project: &str) -> ViewerProcess {
    let child = Command::new(env!("CARGO_BIN_EXE_volicord-viewer"))
        .args([
            "--runtime",
            runtime.to_str().expect("runtime path"),
            "--project",
            project,
            "--bind",
            "127.0.0.1:0",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn real viewer executable");
    let mut viewer = ViewerProcess {
        child,
        address: String::new(),
    };
    let mut startup = String::new();
    BufReader::new(viewer.child.stderr.take().expect("viewer stderr"))
        .read_line(&mut startup)
        .expect("read bound viewer authority");
    let address = startup
        .trim()
        .strip_prefix("Volicord local viewer: http://")
        .and_then(|value| value.strip_suffix('/'))
        .filter(|value| value.starts_with("127.0.0.1:") && !value.ends_with(":0"))
        .unwrap_or_else(|| panic!("viewer reports its actual ephemeral authority: {startup:?}"))
        .to_owned();
    viewer.address.clone_from(&address);
    for _ in 0..100 {
        if let Ok(mut stream) = TcpStream::connect(&address) {
            let request = format!(
                "GET /?view=overview HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
            );
            if stream.write_all(request.as_bytes()).is_ok() {
                let mut response = String::new();
                if stream.read_to_string(&mut response).is_ok()
                    && response.starts_with("HTTP/1.1 200 OK")
                {
                    return viewer;
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
    post_with_context(
        address,
        target,
        &body,
        address,
        Some(&format!("http://{address}")),
        Some("same-origin"),
    )
}

fn post_with_context(
    address: &str,
    target: &str,
    body: &str,
    host: &str,
    origin: Option<&str>,
    fetch_site: Option<&str>,
) -> String {
    let origin = origin.map_or(String::new(), |value| format!("Origin: {value}\r\n"));
    let fetch_site = fetch_site.map_or(String::new(), |value| {
        format!("Sec-Fetch-Site: {value}\r\n")
    });
    exchange(
        address,
        &format!(
            "POST {target} HTTP/1.1\r\nHost: {host}\r\n{origin}{fetch_site}Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
    )
}

#[test]
fn client_reset_does_not_end_the_listener_or_change_canonical_memory() {
    let temporary = TempDir::new().expect("temporary root");
    let runtime = temporary.path().join("runtime");
    let operations = LocalOperations::new(RuntimeLayout::new(&runtime).expect("runtime layout"));
    let project = operations
        .initialize_project("Disconnected client", None)
        .expect("initialize Project")
        .project
        .id;
    let before = operations
        .canonical_basis(project)
        .expect("canonical before");
    let mut viewer = start_viewer(&runtime, &project.to_string());
    let mut reset = TcpStream::connect(&viewer.address).expect("reset connection");
    rustix::net::sockopt::set_socket_linger(&reset, Some(Duration::ZERO)).expect("abortive close");
    // An incomplete header keeps the server reading until the abortive close.
    reset
        .write_all(b"GET /?view=tools&tool=memory HTTP/1.1\r\nHost:")
        .expect("partial request");
    drop(reset);
    let response = get(&viewer.address, "/");
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.contains("Disconnected client"));
    assert!(viewer.child.try_wait().expect("process state").is_none());
    assert_eq!(
        before,
        operations
            .canonical_basis(project)
            .expect("canonical after")
    );
}

#[test]
fn snapshot_mode_writes_one_static_file_and_exits_without_a_listener() {
    let temporary = TempDir::new().expect("temporary root");
    let runtime = temporary.path().join("runtime");
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository).expect("repository");
    fs::write(repository.join("main.py"), "VALUE = 1\n").expect("fixture source");
    let operations = LocalOperations::new(RuntimeLayout::new(&runtime).expect("runtime layout"));
    let project = operations
        .initialize_project("Executable snapshot", Some(&repository))
        .expect("initialize Project")
        .project
        .id;
    operations
        .analyze(project, Vec::new())
        .expect("analyze fixture");
    let canonical_before = operations
        .canonical_basis(project)
        .expect("canonical basis before snapshot");
    let destination = temporary.path().join("shared/viewer.html");

    let output = Command::new(env!("CARGO_BIN_EXE_volicord-viewer"))
        .args([
            "--runtime",
            runtime.to_str().expect("runtime path"),
            "--project",
            &project.to_string(),
            "--snapshot",
            destination.to_str().expect("snapshot destination"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run snapshot mode");
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .expect("snapshot stderr")
        .contains("Volicord Viewer snapshot:"));
    let html = fs::read_to_string(&destination).expect("read static snapshot");
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("data-viewer-mode=\"snapshot\""));
    assert!(
        !html.contains("main.py"),
        "uncited inventory is outside the reader snapshot"
    );
    assert!(html.contains("Snapshot basis"));
    for forbidden in [
        "<form",
        "request_authenticity",
        " src=",
        "<script",
        "/memory/",
        "/guarded/",
        "/documents/export",
    ] {
        assert!(!html.contains(forbidden), "found {forbidden}");
    }
    assert_eq!(
        operations
            .canonical_basis(project)
            .expect("canonical basis after snapshot"),
        canonical_before
    );

    let replacement = Command::new(env!("CARGO_BIN_EXE_volicord-viewer"))
        .args([
            "--runtime",
            runtime.to_str().expect("runtime path"),
            "--project",
            &project.to_string(),
            "--snapshot",
            destination.to_str().expect("snapshot destination"),
        ])
        .output()
        .expect("run replacement snapshot mode");
    assert!(!replacement.status.success());
    assert!(String::from_utf8(replacement.stderr)
        .expect("replacement stderr")
        .contains("already exists"));

    fs::remove_dir_all(&runtime).expect("remove Runtime after snapshot");
    assert!(fs::read_to_string(&destination)
        .expect("snapshot remains readable without Runtime")
        .contains("Executable snapshot"));
}

#[test]
fn real_listener_rejects_retired_surfaces_and_preserves_reading() {
    let temporary = TempDir::new().expect("temporary root");
    let runtime = temporary.path().join("runtime");
    let operations = LocalOperations::new(RuntimeLayout::new(&runtime).expect("layout"));
    let project = operations
        .initialize_project("Executable reader", None)
        .expect("Project")
        .project
        .id;
    let before = operations.canonical_basis(project).expect("basis");
    let viewer = start_viewer(&runtime, &project.to_string());
    for path in ["/?view=overview", "/?view=repository", "/?view=work"] {
        let page = get(&viewer.address, path);
        assert!(page.starts_with("HTTP/1.1 200"), "{page}");
        assert!(!page.contains("tools-menu"));
        let forms = page
            .split("<form")
            .skip(1)
            .map(|s| s.split('>').next().expect("form opening"))
            .collect::<Vec<_>>();
        assert_eq!(forms.len(), usize::from(path == "/?view=work"));
        for form in forms {
            assert!(form.contains("method=\"get\""));
            assert!(form.contains("action=\"/\""));
            assert!(form.contains("role=\"search\""));
        }
    }
    for path in ["/inspection", "/inspection.json", "/code", "/decisions"] {
        assert!(get(&viewer.address, path).starts_with("HTTP/1.1 404"));
    }
    for path in [
        "/?view=tools",
        "/?view=decisions",
        "/?view=code&scope=repository",
    ] {
        assert!(get(&viewer.address, path).starts_with("HTTP/1.1 404"));
    }
    for path in [
        "/memory/context/correct",
        "/memory/decision/correct",
        "/memory/decision/supersede",
        "/memory/forget",
        "/documents/export",
    ] {
        assert!(
            post(&viewer.address, path, "record_id=invalid", "unused").starts_with("HTTP/1.1 404")
        );
    }
    assert_eq!(
        before,
        operations
            .canonical_basis(project)
            .expect("unchanged basis")
    );
    let work = operations
        .transition_work(
            project,
            volicord_operations::WorkTransition::StartNew {
                host: "authored-control".into(),
                session: "fresh-reader".into(),
                user_turn: "Inspect the newly recorded reading request".into(),
                statement: "Inspect the newly recorded reading request".into(),
            },
        )
        .expect("new Work")
        .context_item_id;
    let fresh_basis = operations.canonical_basis(project).expect("fresh basis");
    let fresh = get(&viewer.address, &format!("/?view=work&work={work}"));
    assert!(fresh.starts_with("HTTP/1.1 200"));
    assert!(fresh.contains("Inspect the newly recorded reading request"));
    assert_eq!(
        fresh_basis,
        operations
            .canonical_basis(project)
            .expect("read is unchanged")
    );
}

#[test]
fn real_listener_preserves_exact_guarded_fallback_and_rejects_untrusted_responses() {
    let temporary = TempDir::new().expect("temporary root");
    let operations =
        LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime")).expect("layout"));
    let project = operations
        .initialize_project("Guarded reader", None)
        .expect("Project")
        .project
        .id;
    let now = SystemClock.now().expect("clock");
    let request = operations
        .create_guarded_request(GuardedEffectDraft {
            project_id: project,
            exact_action: "publish test artifact".into(),
            target: "registry/test".into(),
            expected_effect: "public artifact".into(),
            risk: GuardedRisk {
                category: GuardedEffectCategory::ExternalDeploymentOrPublicPublication,
                concrete_consequence: "public artifact".into(),
            },
            scope: vec!["release:test".into()],
            expires_at: TimestampMicros::from_unix_micros(now.as_unix_micros() + 60_000_000),
            requesting_provenance: RequestingProvenance {
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "authored-control".into(),
                },
                host: Some("codex".into()),
                session: None,
                basis: vec!["test only; no dispatch".into()],
            },
        })
        .expect("request");
    let viewer = start_viewer(operations.layout().root(), &project.to_string());
    let target = format!("/guarded/{}", request.confirmation_request_identity);
    let page = get(&viewer.address, &target);
    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    assert!(!page.contains("<nav"), "Guarded fallback is request scoped");
    let token = page
        .split("name=\"request_authenticity\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("token");
    assert!(!target.contains(token));
    let untrusted = exchange(
        &viewer.address,
        &format!("GET {target} HTTP/1.1\r\nHost: foreign.invalid\r\nConnection: close\r\n\r\n"),
    );
    assert!(untrusted.starts_with("HTTP/1.1 421"));
    assert!(!untrusted.contains(&request.effect_fingerprint));
    let body = format!(
        "confirmation_request_id={}&request_revision={}&effect_fingerprint={}&confirmation_decision=confirm&user_turn=Confirm+this+test+artifact&guarded={}",
        request.confirmation_request_identity, request.request_revision,
        request.effect_fingerprint, request.confirmation_request_identity,
    );
    let authenticated = format!("{body}&request_authenticity={token}");
    let origin = format!("http://{}", viewer.address);
    let before = operations.canonical_basis(project).expect("before");
    for (payload, host, request_origin, site) in [
        (
            body.clone(),
            viewer.address.as_str(),
            Some(origin.as_str()),
            Some("same-origin"),
        ),
        (
            format!("{body}&request_authenticity=wrong"),
            viewer.address.as_str(),
            Some(origin.as_str()),
            Some("same-origin"),
        ),
        (
            authenticated.clone(),
            "localhost:1",
            Some(origin.as_str()),
            Some("same-origin"),
        ),
        (
            authenticated.clone(),
            viewer.address.as_str(),
            Some("http://untrusted.invalid"),
            Some("same-origin"),
        ),
        (
            authenticated.clone(),
            viewer.address.as_str(),
            Some(origin.as_str()),
            Some("cross-site"),
        ),
    ] {
        let response = post_with_context(
            &viewer.address,
            "/guarded/confirm",
            &payload,
            host,
            request_origin,
            site,
        );
        let expected = if host == viewer.address {
            "HTTP/1.1 403"
        } else {
            "HTTP/1.1 421"
        };
        assert!(response.starts_with(expected), "{response}");
        assert_eq!(
            before,
            operations.canonical_basis(project).expect("rejected basis")
        );
        assert!(GuardedStore::open(operations.layout().guarded_store())
            .expect("store")
            .response(
                request.confirmation_request_identity,
                request.request_revision
            )
            .expect("response")
            .is_none());
    }
    let response = post(&viewer.address, "/guarded/confirm", &body, token);
    assert!(response.starts_with("HTTP/1.1 303"), "{response}");
    let stored = GuardedStore::open(operations.layout().guarded_store())
        .expect("store")
        .response(
            request.confirmation_request_identity,
            request.request_revision,
        )
        .expect("response")
        .expect("confirmed");
    assert_eq!(stored.decision, ConfirmationDecision::Confirmed);
    assert_eq!(stored.effect_fingerprint, request.effect_fingerprint);
    let confirmed = operations
        .canonical_basis(project)
        .expect("confirmed basis");
    assert!(confirmed
        .sources
        .iter()
        .any(|s| s.source.id == stored.user_response_source_id));
    let replay = post(&viewer.address, "/guarded/confirm", &body, token);
    assert!(!replay.starts_with("HTTP/1.1 303"), "{replay}");
    assert_eq!(
        confirmed,
        operations.canonical_basis(project).expect("replay basis")
    );
    assert!(!format!("{confirmed:?}").contains(token));
}

#[test]
fn direct_viewer_rejects_removed_levels_invalid_views_and_snapshot_selection() {
    for arguments in [
        vec!["--level", "overview"],
        vec!["--view", "invalid"],
        vec![
            "--project",
            "01010101010101010101010101010101",
            "--snapshot",
            "/tmp/viewer-rejected.html",
            "--view",
            "overview",
        ],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_volicord-viewer"))
            .args(arguments)
            .output()
            .expect("direct parser");
        assert!(!result.status.success());
        assert!(!result.stderr.is_empty());
    }
}
