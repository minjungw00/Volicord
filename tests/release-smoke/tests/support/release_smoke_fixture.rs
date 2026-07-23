use serde_json::Value;
use std::{
    env,
    io::{self, BufRead},
    process::{Command, ExitCode},
};
use volicord_release_smoke::CODEX_FIXTURE_VERSION;
use volicord_types::AgentToolId;

const FIXTURE_ARGUMENT: &str = "--release-smoke-test-fixture";
const FIXTURE_VERSION: &str = "volicord-release-smoke-test-fixture-current";

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [] => ExitCode::SUCCESS,
        [argument, scenario] if argument == FIXTURE_ARGUMENT && scenario == "version" => {
            println!("{FIXTURE_VERSION}");
            ExitCode::SUCCESS
        }
        [argument, scenario] if argument == FIXTURE_ARGUMENT && scenario == "hang" => loop {
            std::thread::park();
        },
        [argument] if argument == "--help" => {
            println!("release-smoke Volicord fixture");
            ExitCode::SUCCESS
        }
        [command, ..] if command == "init" => run_init(),
        [mcp, serve, ..] if mcp == "mcp" && serve == "serve" => run_mcp_serve(),
        _ => {
            eprintln!("unsupported release-smoke Volicord fixture invocation");
            ExitCode::from(2)
        }
    }
}

fn run_init() -> ExitCode {
    let output = match Command::new("codex").arg("--version").output() {
        Ok(output) => output,
        Err(error) => {
            eprintln!("failed to launch Codex fixture: {error}");
            return ExitCode::from(1);
        }
    };
    let expected = format!("{CODEX_FIXTURE_VERSION}\n").into_bytes();
    if !output.status.success() || output.stdout != expected || !output.stderr.is_empty() {
        eprintln!("Codex fixture returned an unexpected version result");
        return ExitCode::from(1);
    }
    println!(
        "{}",
        serde_json::json!({
            "connection": {"connection_id": "release-smoke-fixture-connection"}
        })
    );
    ExitCode::SUCCESS
}

fn run_mcp_serve() -> ExitCode {
    match read_revision() {
        Ok(revision) => {
            println!(
                "{}",
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {"protocolVersion": revision}
                })
            );
            let tools = AgentToolId::ALL
                .iter()
                .map(|tool| serde_json::json!({"name": tool.wire_name()}))
                .collect::<Vec<_>>();
            println!(
                "{}",
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "result": {"tools": tools}
                })
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn read_revision() -> Result<String, String> {
    let mut revision = None;
    let mut initialized = false;
    let mut tools_list = false;
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|error| format!("read fixture stdin: {error}"))?;
        let message: Value = serde_json::from_str(&line)
            .map_err(|error| format!("parse fixture JSON-RPC input: {error}"))?;
        match message.get("method").and_then(Value::as_str) {
            Some("initialize") => {
                revision = message
                    .pointer("/params/protocolVersion")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
            Some("notifications/initialized") => initialized = true,
            Some("tools/list") => tools_list = true,
            _ => {}
        }
    }
    if !initialized {
        return Err("fixture did not receive initialized notification".to_owned());
    }
    if !tools_list {
        return Err("fixture did not receive tools/list".to_owned());
    }
    revision.ok_or_else(|| "fixture did not receive initialize revision".to_owned())
}
