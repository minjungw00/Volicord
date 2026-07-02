#![forbid(unsafe_code)]

use std::{
    error::Error,
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use volicord_store::{
    agent_connections::{add_connection_project, ConnectionProjectRegistration},
    bootstrap::{register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS},
};
use volicord_test_support::core_fixtures::CoreFixture;

const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);
const TEST_TOKEN: &str = "serve_test_token";

#[test]
fn volicord_serve_local_http_starts_with_secure_defaults() -> Result<(), Box<dyn Error>> {
    let fixture = ServeFixture::new("serve-bin-secure-defaults")?;
    let outside_project_id = "project_serve_outside";
    fixture.add_allowed_project(outside_project_id)?;
    let mut server = RunningServer::spawn(fixture.serve_command([
        "--transport",
        "local-http",
        "--listen",
        "127.0.0.1:0",
        "--connection",
        fixture.connection_id(),
        "--token",
        TEST_TOKEN,
        "--project",
        fixture.product_repo_path(),
    ]))?;
    let addr = match server.wait_for_listen_addr() {
        Ok(addr) => addr,
        Err(error) if local_bind_is_sandbox_blocked(error.as_ref()) => {
            eprintln!("skipping local TCP startup assertions: {error}");
            return Ok(());
        }
        Err(error) => return Err(error),
    };

    let unauth_health = send_http(addr, "GET", "/healthz", Vec::new(), None)?;
    assert!(unauth_health.starts_with("HTTP/1.1 401 Unauthorized"));
    assert_eq!(
        response_json(&unauth_health)["error"]["code"],
        "AUTH_REQUIRED"
    );

    let invalid_origin = send_http(
        addr,
        "POST",
        "/mcp",
        vec![
            ("Authorization", format!("Bearer {TEST_TOKEN}")),
            ("Accept", "application/json, text/event-stream".to_owned()),
            ("Content-Type", "application/json".to_owned()),
            ("Origin", "https://example.invalid".to_owned()),
        ],
        Some(initialize_request(1)),
    )?;
    assert!(invalid_origin.starts_with("HTTP/1.1 403 Forbidden"));
    assert_eq!(
        response_json(&invalid_origin)["error"]["code"],
        "ORIGIN_NOT_ALLOWED"
    );
    assert!(response_header(&invalid_origin, "Access-Control-Allow-Origin").is_none());

    let initialize = send_http(
        addr,
        "POST",
        "/mcp",
        json_headers(None),
        Some(initialize_request(2)),
    )?;
    assert!(initialize.starts_with("HTTP/1.1 200 OK"));
    let session_id =
        response_header(&initialize, "Mcp-Session-Id").ok_or("missing Mcp-Session-Id")?;

    let initialized = send_http(
        addr,
        "POST",
        "/mcp",
        json_headers(Some(&session_id)),
        Some(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        })),
    )?;
    assert!(initialized.starts_with("HTTP/1.1 202 Accepted"));

    let rejected_project = send_http(
        addr,
        "POST",
        "/mcp",
        json_headers(Some(&session_id)),
        Some(json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "volicord.status",
                "arguments": {
                    "detail": "workflow",
                    "project_selector": outside_project_id
                }
            }
        })),
    )?;
    assert!(rejected_project.starts_with("HTTP/1.1 200 OK"));
    let body = response_json(&rejected_project);
    assert_eq!(body["result"]["isError"], true);
    let error_text = body["result"]["content"][0]["text"]
        .as_str()
        .ok_or("tool error should contain text")?;
    assert!(error_text.contains("outside this HTTP serve project allowlist"));

    Ok(())
}

struct ServeFixture {
    fixture: CoreFixture,
    product_repo_path: String,
}

impl ServeFixture {
    fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
        let fixture = CoreFixture::new(prefix)?;
        fs::create_dir_all(fixture.product_repo_path().join(".git"))?;
        let product_repo_path = fixture
            .product_repo_path()
            .to_str()
            .ok_or("fixture product repo path should be UTF-8")?
            .to_owned();
        Ok(Self {
            fixture,
            product_repo_path,
        })
    }

    fn connection_id(&self) -> &str {
        self.fixture.connection_id()
    }

    fn product_repo_path(&self) -> &str {
        &self.product_repo_path
    }

    fn serve_command<const N: usize>(&self, args: [&str; N]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_volicord"));
        command.env_clear();
        command.env("VOLICORD_HOME", self.fixture.runtime_home_path());
        command.current_dir(env!("CARGO_MANIFEST_DIR"));
        command.arg("serve");
        command.args(args);
        command
    }

    fn add_allowed_project(&self, project_id: &str) -> Result<(), Box<dyn Error>> {
        let repo_root = self
            .fixture
            .create_product_repo(format!("repo-{project_id}"))?;
        fs::create_dir_all(repo_root.join(".git"))?;
        register_project(
            self.fixture.runtime_home_path(),
            ProjectRegistration {
                project_id: project_id.to_owned(),
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        add_connection_project(
            self.fixture.runtime_home_path(),
            ConnectionProjectRegistration {
                connection_internal_id: self.fixture.connection_id().to_owned(),
                project_id: project_id.to_owned(),
            },
        )?;
        Ok(())
    }
}

struct RunningServer {
    child: Option<Child>,
    stderr_lines: Receiver<String>,
    stdout: Option<JoinHandle<io::Result<Vec<u8>>>>,
    stderr: Option<JoinHandle<io::Result<Vec<String>>>>,
}

impl RunningServer {
    fn spawn(mut command: Command) -> io::Result<Self> {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("stdout was not piped"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("stderr was not piped"))?;
        let stdout = thread::spawn(move || read_to_end(stdout));
        let (sender, stderr_lines) = mpsc::channel();
        let stderr = thread::spawn(move || {
            let mut lines = Vec::new();
            for line in BufReader::new(stderr).lines() {
                let line = line?;
                let _ = sender.send(line.clone());
                lines.push(line);
            }
            Ok(lines)
        });
        Ok(Self {
            child: Some(child),
            stderr_lines,
            stdout: Some(stdout),
            stderr: Some(stderr),
        })
    }

    fn wait_for_listen_addr(&mut self) -> Result<SocketAddr, Box<dyn Error>> {
        let started = Instant::now();
        let mut observed_stderr = Vec::new();
        loop {
            if let Some(child) = self.child.as_mut() {
                if let Some(status) = child.try_wait()? {
                    return Err(format!(
                        "server exited before startup: {status}\nstderr:\n{}",
                        observed_stderr.join("\n")
                    )
                    .into());
                }
            }
            let remaining = PROCESS_TIMEOUT
                .checked_sub(started.elapsed())
                .ok_or("server did not print listen address before timeout")?;
            let wait = remaining.min(Duration::from_millis(50));
            match self.stderr_lines.recv_timeout(wait) {
                Ok(line) => {
                    if let Some(addr) = parse_listen_addr(&line)? {
                        return Ok(addr);
                    }
                    observed_stderr.push(line);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(format!(
                        "server stderr closed before startup\nstderr:\n{}",
                        observed_stderr.join("\n")
                    )
                    .into());
                }
            }
        }
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(stdout) = self.stdout.take() {
            let _ = stdout.join();
        }
        if let Some(stderr) = self.stderr.take() {
            let _ = stderr.join();
        }
    }
}

fn parse_listen_addr(line: &str) -> Result<Option<SocketAddr>, Box<dyn Error>> {
    let Some(rest) = line.strip_prefix("volicord serve listening on http://") else {
        return Ok(None);
    };
    let Some(addr) = rest.strip_suffix("/mcp") else {
        return Ok(None);
    };
    Ok(Some(addr.parse()?))
}

fn local_bind_is_sandbox_blocked(error: &(dyn Error + 'static)) -> bool {
    error
        .to_string()
        .contains("Operation not permitted (os error 1)")
}

fn send_http(
    addr: SocketAddr,
    method: &str,
    target: &str,
    headers: Vec<(&str, String)>,
    body: Option<Value>,
) -> Result<String, Box<dyn Error>> {
    let body = match body {
        Some(value) => serde_json::to_vec(&value)?,
        None => Vec::new(),
    };
    let mut stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(PROCESS_TIMEOUT))?;
    write!(
        stream,
        "{method} {target} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    )?;
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    stream.write_all(b"\r\n")?;
    stream.write_all(&body)?;
    stream.flush()?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

fn json_headers(session_id: Option<&str>) -> Vec<(&'static str, String)> {
    let mut headers = vec![
        ("Authorization", format!("Bearer {TEST_TOKEN}")),
        ("Accept", "application/json, text/event-stream".to_owned()),
        ("Content-Type", "application/json".to_owned()),
    ];
    if let Some(session_id) = session_id {
        headers.push(("Mcp-Session-Id", session_id.to_owned()));
    }
    headers
}

fn initialize_request(id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {
                "name": "volicord-serve-test",
                "version": "0.0.0"
            }
        }
    })
}

fn response_header(response: &str, name: &str) -> Option<String> {
    let header_block = response.split("\r\n\r\n").next()?;
    for line in header_block.lines().skip(1) {
        let (header_name, value) = line.split_once(':')?;
        if header_name.eq_ignore_ascii_case(name) {
            return Some(value.trim().to_owned());
        }
    }
    None
}

fn response_json(response: &str) -> Value {
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .expect("HTTP response should include header/body separator");
    serde_json::from_str(body).expect("HTTP response body should be JSON")
}

fn read_to_end(mut reader: impl Read) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    Ok(output)
}
