use serde_json::{json, Value};
use std::{fs, process::Command};
use tempfile::tempdir;
use volicord_host::HostAdapter;
use volicord_operations::{LocalOperations, RuntimeLayout};

#[test]
fn installed_schema_matches_tools_list_without_runtime_access() {
    let temporary = tempdir().expect("temporary directory");
    let runtime = temporary.path().join("runtime");
    let mut adapter = HostAdapter::new(LocalOperations::new(
        RuntimeLayout::new(&runtime).expect("Runtime layout"),
    ));
    let listed = adapter
        .handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
        .expect("tools/list response");
    let forbidden_runtime = temporary.path().join("not-a-directory");
    fs::write(&forbidden_runtime, "sentinel").expect("Runtime blocker");
    for name in [
        "materiality_review",
        "engineering_choice_discovery",
        "work_explanation",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_volicord-mcp"))
            .args(["--tool-schema", name])
            .env("VOLICORD_RUNTIME_DIR", forbidden_runtime.join("child"))
            .env_remove("HOME")
            .env_remove("XDG_DATA_HOME")
            .output()
            .expect("metadata command");
        assert!(output.status.success(), "{:?}", output);
        assert!(output.stderr.is_empty());
        let definition: Value = serde_json::from_slice(&output.stdout).expect("complete JSON");
        let expected = listed["result"]["tools"]
            .as_array()
            .expect("tool list")
            .iter()
            .find(|tool| tool["name"] == name)
            .expect("listed tool");
        assert_eq!(&definition, expected);
    }
    assert!(!runtime.exists(), "metadata must not initialize a Runtime");
    assert_eq!(fs::read_to_string(&forbidden_runtime).unwrap(), "sentinel");
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 1);
}

#[test]
fn invalid_schema_requests_fail_without_starting_stdio_or_runtime() {
    let temporary = tempdir().expect("temporary directory");
    let runtime = temporary.path().join("runtime");
    for arguments in [
        vec!["--tool-schema"],
        vec!["--tool-schema", "nonexistent"],
        vec!["--tool-schema", "materiality_review", "extra"],
        vec!["--unknown"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_volicord-mcp"))
            .args(arguments)
            .env("VOLICORD_RUNTIME_DIR", &runtime)
            .output()
            .expect("invalid metadata command");
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    assert!(!runtime.exists());
}
