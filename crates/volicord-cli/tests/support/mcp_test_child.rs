use std::{
    env,
    error::Error,
    ffi::OsStr,
    fs,
    io::{self, BufRead, Write},
    path::Path,
    process::{self, Command},
    thread,
};

use serde_json::{json, Value};
use volicord_types::{ADAPTER_UTILITY_TOOL_NAMES, READ_ONLY_METHOD_TOOL_NAMES};

const LARGE_STDERR_BYTES: usize = 8 * 1024;
const SUSTAINED_STDERR_BYTES: usize = 256 * 1024;
const TEST_CHILD_SCENARIO_ARGUMENT: &str = "--mcp-test-child-scenario";
const TEST_CHILD_VERSION: &str = "volicord-mcp-test-child-current";

fn main() {
    if let Err(error) = run() {
        eprintln!("MCP test child failed: {error}");
        process::exit(64);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new(TEST_CHILD_SCENARIO_ARGUMENT)) {
        return Ok(());
    }
    let scenario = args
        .next()
        .ok_or("missing test-child scenario")?
        .into_string()
        .map_err(|_| "test-child scenario was not UTF-8")?;
    match scenario.as_str() {
        "protocol-version" => println!("{TEST_CHILD_VERSION}"),
        "preflight-success" => println!("configuration: valid"),
        "preflight-descendant-output-hold" => {
            spawn_output_handle_descendant()?;
            println!("configuration: valid");
        }
        "hang-before-initialize" => {
            eprintln!("waiting for initialize");
            park_forever();
        }
        "exit-before-initialize" => {
            eprintln!("fixture startup failure");
            process::exit(23);
        }
        "malformed-json" => {
            let stdin = io::stdin();
            let mut input = stdin.lock();
            read_request(&mut input, "initialize")?;
            println!("{{not-json}}");
        }
        "large-stderr-exit" => {
            io::stderr().write_all(&vec![b'x'; LARGE_STDERR_BYTES])?;
            process::exit(19);
        }
        "stdio-success" => run_stdio(Scenario::Success)?,
        "tools-list-failure" => run_stdio(Scenario::ToolsListFailure)?,
        "missing-required-tools" => run_stdio(Scenario::MissingRequiredTools)?,
        "read-only-tool-failure" => run_stdio(Scenario::SafeToolFailure)?,
        "shutdown-failure" => run_stdio(Scenario::ShutdownFailure)?,
        "sustained-stderr" => run_stdio(Scenario::SustainedStderr)?,
        "graceful-eof" => {
            let marker = args.next().ok_or("missing graceful EOF marker path")?;
            run_stdio(Scenario::GracefulEof(Path::new(&marker)))?;
        }
        "descendant-output-hold" => run_stdio(Scenario::DescendantOutputHold)?,
        "hold-output-pipes" => park_forever(),
        _ => return Err(format!("unknown test-child scenario: {scenario}").into()),
    }
    if args.next().is_some() {
        return Err("unexpected test-child argument".into());
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Scenario<'a> {
    Success,
    ToolsListFailure,
    MissingRequiredTools,
    SafeToolFailure,
    ShutdownFailure,
    SustainedStderr,
    GracefulEof(&'a Path),
    DescendantOutputHold,
}

fn run_stdio(scenario: Scenario<'_>) -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();

    read_request(&mut input, "initialize")?;
    if matches!(scenario, Scenario::SustainedStderr) {
        io::stderr().write_all(&vec![b'x'; SUSTAINED_STDERR_BYTES])?;
    }
    write_json(
        &mut output,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"instructions": "fixture"},
        }),
    )?;
    read_request(&mut input, "notifications/initialized")?;
    read_request(&mut input, "tools/list")?;

    if matches!(scenario, Scenario::ToolsListFailure) {
        return write_json(
            &mut output,
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "error": {"code": -32603, "message": "fixture prose is not classified"},
            }),
        );
    }

    let tools = if matches!(scenario, Scenario::MissingRequiredTools) {
        vec!["fixture.alpha", "fixture.beta"]
    } else {
        READ_ONLY_METHOD_TOOL_NAMES
            .iter()
            .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
            .copied()
            .collect()
    };
    write_json(
        &mut output,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "tools": tools
                    .into_iter()
                    .map(|name| json!({"name": name}))
                    .collect::<Vec<_>>()
            },
        }),
    )?;
    if matches!(scenario, Scenario::MissingRequiredTools) {
        drain_to_eof(&mut input)?;
        return Ok(());
    }

    read_request(&mut input, "tools/call")?;
    let safe_response = if matches!(scenario, Scenario::SafeToolFailure) {
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "error": {"code": -32603, "message": "ignored child prose"},
        })
    } else {
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "result": {"content": []},
        })
    };
    write_json(&mut output, &safe_response)?;
    drain_to_eof(&mut input)?;

    match scenario {
        Scenario::ShutdownFailure => process::exit(17),
        Scenario::GracefulEof(marker) => fs::write(marker, "reaped")?,
        Scenario::DescendantOutputHold => spawn_output_handle_descendant()?,
        _ => {}
    }
    Ok(())
}

fn read_request(input: &mut impl BufRead, expected_method: &str) -> Result<(), Box<dyn Error>> {
    let mut line = String::new();
    if input.read_line(&mut line)? == 0 {
        return Err(format!("stdin ended before {expected_method}").into());
    }
    let value: Value = serde_json::from_str(&line)?;
    if value.get("method").and_then(Value::as_str) != Some(expected_method) {
        return Err(format!("expected {expected_method} request").into());
    }
    Ok(())
}

fn write_json(output: &mut impl Write, value: &Value) -> Result<(), Box<dyn Error>> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}

fn drain_to_eof(input: &mut impl BufRead) -> io::Result<()> {
    let mut line = String::new();
    while input.read_line(&mut line)? != 0 {
        line.clear();
    }
    Ok(())
}

fn spawn_output_handle_descendant() -> io::Result<()> {
    Command::new(env::current_exe()?)
        .arg(TEST_CHILD_SCENARIO_ARGUMENT)
        .arg("hold-output-pipes")
        .spawn()?;
    Ok(())
}

fn park_forever() -> ! {
    loop {
        thread::park();
    }
}
