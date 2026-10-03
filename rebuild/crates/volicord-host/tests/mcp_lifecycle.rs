//! Production executable registration is independent of the stdio protocol.
#![cfg(target_os = "linux")]
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};
#[test]
fn eof_registers_real_executable_and_shutdown_without_body_retention(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let runtime = root.path().join("runtime");
    fs::create_dir(&runtime)?;
    let secret = "private-RPC-and-environment-SENTINEL-194851";
    let mut process = Command::new(env!("CARGO_BIN_EXE_volicord-mcp"))
        .env("VOLICORD_RUNTIME_DIR", &runtime)
        .env("PRIVATE_TEST_SENTINEL", secret)
        .current_dir(root.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let pid = process.id();
    let input = json!({"jsonrpc":"2.0","id":1,"method":"initialize", "params":{"clientInfo":{"name":secret,"version":"test"}}});
    let mut stdin = process.stdin.take().ok_or("stdin")?;
    writeln!(stdin, "{input}")?;
    drop(stdin);
    let output = process.wait_with_output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());
    let entries = fs::read_dir(runtime.join("observations/mcp"))?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(entries.len(), 1);
    let bytes = fs::read(entries[0].path())?;
    let entry: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(entry["pid"], pid);
    assert_eq!(entry["state"], "stopped");
    assert!(entry["start_ticks"].as_u64().is_some_and(|n| n > 0));
    use sha2::{Digest, Sha256};
    assert_eq!(
        entry["executable_sha256"],
        format!(
            "{:x}",
            Sha256::digest(fs::read(env!("CARGO_BIN_EXE_volicord-mcp"))?)
        )
    );
    assert!(!String::from_utf8(bytes)?.contains(secret));
    assert!(
        !runtime.join("canonical.sqlite3").exists(),
        "registration invented canonical authority"
    );
    Ok(())
}
#[test]
fn registration_failure_preserves_protocol_and_creates_no_repository_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    fs::write(root.path().join("observations"), "blocked")?;
    let mut process = Command::new(env!("CARGO_BIN_EXE_volicord-mcp"))
        .env("VOLICORD_RUNTIME_DIR", root.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = process.stdin.take().ok_or("stdin")?;
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}})
    )?;
    drop(stdin);
    let output = process.wait_with_output()?;
    assert!(output.status.success());
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?["id"], 1);
    assert!(String::from_utf8(output.stderr)?.contains("lifecycle observation unavailable"));
    assert!(!root.path().join("canonical.sqlite3").exists());
    Ok(())
}
