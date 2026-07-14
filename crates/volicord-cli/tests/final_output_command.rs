#![forbid(unsafe_code)]

mod support;

use std::{error::Error, fs, io::Write, process::Command};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use support::binary_fixture::{prepare_runtime_home, volicord_bin};
use volicord_cli::host_integration::{MANAGED_PROCESS_BINDING_ENV, MANAGED_PROCESS_BINDING_V1};
use volicord_core::{CoreService, InvocationContext};
use volicord_store::guards::{upsert_guard_installation, GuardInstallationUpsert};
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::{
    ActorSource, AuthorityReceipt, OperationCategory, ProjectId,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

#[test]
fn binary_final_output_drains_stdin_and_projects_only_fresh_authority() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("binary-final-output")?;
    let task = CoreService::new(fixture.runtime_home_path()).intake(
        fixture.intake_request(
            "req_binary_final_output_intake",
            "idem_binary_final_output_intake",
            false,
            Some(0),
        ),
        InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::agent_connection(fixture.connection_id()),
            OperationCategory::AgentWorkflow,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        ),
    )?;
    let task_id = task.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("intake task ref")
        .to_owned();
    let installation_id = "guard_binary_final_output";
    let record_command_args = |command_name: &str| {
        json!([
            "_hook",
            command_name,
            "--repo",
            fixture.product_repo_path().display().to_string(),
            "--connection",
            fixture.connection_id(),
            "--guard-installation",
            installation_id,
            "--host",
            "codex",
            "--integration-profile",
            "record",
            "--output",
            "volicord-json"
        ])
    };
    let commands = json!({
        "session_start": {"command": "volicord", "args": record_command_args("session-start")},
        "pre_tool": {"command": "volicord", "args": record_command_args("pre-tool")},
        "post_tool": {"command": "volicord", "args": record_command_args("post-tool")},
        "prompt_capture": {"command": "volicord", "args": record_command_args("prompt-capture")},
        "stop": {"command": "volicord", "args": record_command_args("stop")}
    });
    let policy = json!({
        "schema": "volicord-policy-v1",
        "managed_by": "volicord",
        "storage_scope": "local_overlay",
        "connection_intent": "shared",
        "host": "codex",
        "repo_root": fixture.product_repo_path().display().to_string(),
        "connection_id": fixture.connection_id(),
        "guard_installation_id": installation_id,
        "selected_profile": "record",
        "mcp": {"command": "volicord", "args": ["mcp", "--stdio"], "env": {}},
        "host_hook": {"enabled": false, "commands": commands}
    });
    let policy_text = serde_json::to_string(&policy)?;
    let policy_hash = format!("sha256:{:x}", Sha256::digest(policy_text.as_bytes()));
    let policy_dir = fixture.product_repo_path().join(".volicord");
    fs::create_dir_all(&policy_dir)?;
    fs::write(
        policy_dir.join("policy.json"),
        serde_json::to_string_pretty(&policy)?,
    )?;
    upsert_guard_installation(
        fixture.runtime_home_path(),
        GuardInstallationUpsert {
            guard_installation_id: installation_id.to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            project_id: Some(fixture.project_id().to_owned()),
            host_kind: "codex".to_owned(),
            guard_mode: "record".to_owned(),
            host_capability_json: json!({
                "schema": "volicord-host-hook-capability-v2",
                "policy_hash": policy_hash.clone(),
                "selected_profile": "record",
                "connection_intent": "shared",
                "final_output_authority_disclosure_implementation_available": true,
                "native_host_output_adapter": "codex",
                "native_host_output_adapter_config_verified": true,
                "bash_shell_mutation_coverage": false,
                "direct_file_write_matcher_coverage": false,
                "host_capabilities": {
                    "stdio_mcp": true,
                    "http_mcp": false,
                    "session_start_hook": true,
                    "pre_tool_hook": true,
                    "post_tool_hook": true,
                    "user_prompt_submit_hook": true,
                    "stop_hook": true,
                    "rule_file_support": true,
                    "project_local_configuration": true,
                },
                "required_hook_phases": [],
                "missing_required_hooks": [],
                "prompt_capture": false,
                "files": [],
                "host_hook_commands": [],
                "hook_root_resolution": null,
                "hook_path_safety": null,
                "commands": commands,
            })
            .to_string(),
            installation_status: "configured".to_owned(),
            installed_at: None,
            last_checked_at: "2026-07-13T00:00:00Z".to_owned(),
            first_seen_at: None,
            last_seen_at: None,
            last_seen_phase: None,
            observed_host_kind: None,
            observed_policy_hash: None,
            observed_binary_version: None,
            metadata_json: "{}".to_owned(),
        },
    )?;
    let selected_volicord = fs::canonicalize(volicord_bin())?;
    prepare_runtime_home(fixture.runtime_home_path(), &selected_volicord)?;
    let before = fixture.counts()?;

    let mut child = Command::new(&selected_volicord)
        .args([
            "_final-output",
            "--repo",
            fixture
                .product_repo_path()
                .to_str()
                .expect("UTF-8 fixture repository"),
            "--connection",
            fixture.connection_id(),
            "--guard-installation",
            installation_id,
            "--host",
            "codex",
            "--integration-profile",
            "record",
            "--policy-hash",
            &policy_hash,
            "--host-output",
            "codex",
        ])
        .env("VOLICORD_HOME", fixture.runtime_home_path())
        .env(MANAGED_PROCESS_BINDING_ENV, MANAGED_PROCESS_BINDING_V1)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    let private_marker = "PRIVATE_MODEL_FINAL_PROSE_MUST_NOT_PROJECT";
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(format!("{{\"message\":\"{}\"}}", private_marker.repeat(4096)).as_bytes())?;
    let output = child.wait_with_output()?;

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.ends_with('\n'));
    assert!(stdout.len() <= 8192);
    assert!(!stdout.contains(private_marker));
    let response: Value = serde_json::from_str(&stdout)?;
    let message = response["systemMessage"].as_str().expect("systemMessage");
    let receipt: AuthorityReceipt = serde_json::from_str(
        message
            .strip_prefix("Volicord authority receipt: ")
            .expect("canonical receipt prefix"),
    )?;
    assert_eq!(receipt.task_ref.record_id.as_str(), task_id);
    assert_eq!(fixture.counts()?, before);
    Ok(())
}
