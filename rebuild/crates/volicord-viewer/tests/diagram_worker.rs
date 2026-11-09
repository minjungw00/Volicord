//! Actual executable transport controls. No Runtime Home, authoring or Source read.
use std::{
    error::Error,
    io::{Read, Write},
    process::{Child, Command, Stdio},
};
const GRAPH: &[u8]=br#"{"id":"root","layoutOptions":{"elk.algorithm":"layered"},"children":[{"id":"a","width":20,"height":20}]}"#;
fn child(runtime: &std::path::Path) -> Result<Child, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_volicord-viewer"))
        .arg("--local-layout-worker")
        .env("VOLICORD_RUNTIME_DIR", runtime)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?)
}
fn response(child: &mut Child) -> Result<(u8, Vec<u8>), Box<dyn Error>> {
    let output = child.stderr.as_mut().ok_or("stderr")?;
    let mut header = [0; 5];
    output.read_exact(&mut header)?;
    let count = u32::from_be_bytes(header[1..].try_into()?) as usize;
    assert!(count <= 262144, "bounded executable output");
    let mut bytes = vec![0; count];
    output.read_exact(&mut bytes)?;
    Ok((header[0], bytes))
}
fn ready(child: &mut Child) -> Result<(), Box<dyn Error>> {
    let mut hello = [0; 18];
    child
        .stderr
        .as_mut()
        .ok_or("stderr")?
        .read_exact(&mut hello)?;
    assert_eq!(&hello, b"volicord-layout/1\n");
    assert_eq!(response(child)?, (0, vec![]));
    Ok(())
}
fn request(child: &mut Child, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let input = child.stdin.as_mut().ok_or("stdin")?;
    input.write_all(&(bytes.len() as u32).to_be_bytes())?;
    input.write_all(bytes)?;
    input.flush()?;
    Ok(())
}
#[test]
fn installed_entry_rejects_malformed_work_and_fresh_process_recovers_without_runtime_access(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let runtime = temp.path().join("must-not-be-created");
    let mut failed = child(&runtime)?;
    ready(&mut failed)?;
    request(&mut failed, b"not JSON")?;
    let (state, body) = response(&mut failed)?;
    assert_eq!(state, 1);
    assert!(!String::from_utf8(body)?.contains("not JSON"));
    assert_eq!(failed.wait()?.code(), Some(1));
    let mut healthy = child(&runtime)?;
    ready(&mut healthy)?;
    for _ in 0..3 {
        request(&mut healthy, GRAPH)?;
        let (state, body) = response(&mut healthy)?;
        assert_eq!(state, 0);
        let geometry: serde_json::Value = serde_json::from_slice(&body)?;
        assert_eq!(geometry["children"][0]["id"], "a");
        assert!(geometry["width"]
            .as_f64()
            .is_some_and(|n| n.is_finite() && n > 0.0));
    }
    drop(healthy.stdin.take());
    assert_eq!(healthy.wait()?.code(), Some(0));
    assert!(
        !runtime.exists(),
        "geometry execution never opens a Runtime Home"
    );
    Ok(())
}
#[test]
fn executable_refuses_input_budget_before_allocating_the_announced_body(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let mut process = child(&temp.path().join("absent"))?;
    ready(&mut process)?;
    process
        .stdin
        .as_mut()
        .ok_or("stdin")?
        .write_all(&65537u32.to_be_bytes())?;
    assert_eq!(process.wait()?.code(), Some(2));
    Ok(())
}
