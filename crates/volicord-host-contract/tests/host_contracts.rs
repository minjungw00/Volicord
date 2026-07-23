use std::{collections::BTreeSet, fs, path::PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_host_contract::{
    CodexHookEvent, CodexHooksV1, CodexMcpTurnMetadataV1, HostContractErrorCode,
    HostContractProfileId, HostNativeCorrelation,
};

#[derive(Debug, Deserialize)]
struct Manifest {
    manifest_version: u64,
    host: String,
    fixtures: Vec<Fixture>,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    path: String,
    profile_id: String,
    contract_generation: u64,
    purpose: String,
    source_reference: String,
    checksum_sha256: String,
    production_covered: bool,
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/codex-host")
}

fn read_json(path: &str) -> Value {
    serde_json::from_slice(&fs::read(fixture_root().join(path)).expect("fixture bytes"))
        .expect("fixture JSON")
}

#[test]
fn pinned_fixture_checksums_and_profile_coverage_are_exact() {
    let root = fixture_root();
    let manifest: Manifest =
        toml::from_str(&fs::read_to_string(root.join("manifest.toml")).expect("host manifest"))
            .expect("host manifest shape");
    assert_eq!(manifest.manifest_version, 1);
    assert_eq!(manifest.host, "codex");
    let mut covered = BTreeSet::new();
    for fixture in manifest.fixtures {
        assert!(fixture.contract_generation > 0);
        assert!(!fixture.purpose.trim().is_empty());
        assert!(!fixture.source_reference.trim().is_empty());
        assert!(fixture.production_covered);
        let bytes = fs::read(root.join(&fixture.path)).expect("pinned fixture");
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            fixture.checksum_sha256
        );
        covered.insert(fixture.profile_id.clone());
        let payload: Value = serde_json::from_slice(
            &fs::read(root.join(&fixture.path)).expect("pinned fixture bytes"),
        )
        .expect("pinned fixture JSON");
        match fixture.profile_id.as_str() {
            "codex-hooks-v1" => {
                CodexHooksV1
                    .parse(&payload)
                    .expect("hook fixture must parse");
            }
            "codex-mcp-2025-06-18-v1" => {
                CodexMcpTurnMetadataV1
                    .parse_tools_call(&payload)
                    .expect("MCP fixture must parse");
            }
            profile => panic!("unexpected fixture profile: {profile}"),
        }
    }
    assert_eq!(
        covered,
        HostContractProfileId::ALL
            .iter()
            .map(|profile| profile.as_str().to_owned())
            .collect()
    );
}

#[test]
fn hooks_parse_without_thread_and_keep_source_specific_correlation() {
    let prompt = CodexHooksV1
        .parse(&read_json("hooks-v1/user-prompt-submit.json"))
        .expect("prompt fixture");
    assert!(matches!(
        prompt.correlation(),
        HostNativeCorrelation::CodexHookPrompt(_)
    ));
    let pre = CodexHooksV1
        .parse(&read_json("hooks-v1/pre-tool-use-bash.json"))
        .expect("pre fixture");
    let post = CodexHooksV1
        .parse(&read_json("hooks-v1/post-tool-use-bash.json"))
        .expect("post fixture");
    let (
        CodexHookEvent::PreToolUse {
            correlation: pre, ..
        },
        CodexHookEvent::PostToolUse {
            correlation: post, ..
        },
    ) = (pre, post)
    else {
        panic!("tool fixtures must retain their event variants");
    };
    assert_eq!(pre, post);
    assert_eq!(pre.tool_name.as_str(), "Bash");
}

#[test]
fn guard_probe_hook_fixtures_preserve_exact_typed_correlation() {
    let pre = CodexHooksV1
        .parse(&read_json("hooks-v1/pre-tool-use-mcp.json"))
        .expect("Guard probe pre-tool fixture");
    let post = CodexHooksV1
        .parse(&read_json("hooks-v1/post-tool-use-mcp.json"))
        .expect("Guard probe post-tool fixture");
    let (
        CodexHookEvent::PreToolUse {
            correlation: pre,
            tool_input: pre_input,
            ..
        },
        CodexHookEvent::PostToolUse {
            correlation: post,
            tool_input: post_input,
            ..
        },
    ) = (pre, post)
    else {
        panic!("Guard probe fixtures must retain their event variants");
    };
    assert_eq!(pre, post);
    assert_eq!(pre.tool_name.as_str(), "mcp__volicord__guard_probe");
    assert_eq!(
        pre_input.as_value(),
        &json!({"verification_id": "guard_verification_fixture_001"})
    );
    assert_eq!(pre_input, post_input);
}

#[test]
fn mcp_and_hook_correlation_are_not_interchangeable() {
    let mcp = CodexMcpTurnMetadataV1
        .parse_tools_call(&read_json("mcp-turn-metadata-v1/tools-call.json"))
        .expect("MCP fixture");
    let hook = CodexHooksV1
        .parse(&read_json("hooks-v1/pre-tool-use-mcp.json"))
        .expect("hook fixture");
    assert!(matches!(
        HostNativeCorrelation::CodexMcp(mcp),
        HostNativeCorrelation::CodexMcp(_)
    ));
    assert!(matches!(
        hook.correlation(),
        HostNativeCorrelation::CodexHookTool(_)
    ));
}

#[test]
fn required_hook_fields_and_tool_use_ids_fail_closed() {
    let mut missing_session = read_json("hooks-v1/pre-tool-use-mcp.json");
    missing_session
        .as_object_mut()
        .unwrap()
        .remove("session_id");
    let error = CodexHooksV1.parse(&missing_session).unwrap_err();
    assert_eq!(error.code(), HostContractErrorCode::MissingRequiredField);
    assert_eq!(error.field(), "session_id");

    let mut missing_turn = read_json("hooks-v1/user-prompt-submit.json");
    missing_turn.as_object_mut().unwrap().remove("turn_id");
    let error = CodexHooksV1.parse(&missing_turn).unwrap_err();
    assert_eq!(error.code(), HostContractErrorCode::MissingRequiredField);
    assert_eq!(error.field(), "turn_id");

    let mut malformed_tool = read_json("hooks-v1/pre-tool-use-bash.json");
    malformed_tool["tool_use_id"] = json!("tool use with spaces");
    let error = CodexHooksV1.parse(&malformed_tool).unwrap_err();
    assert_eq!(error.code(), HostContractErrorCode::InvalidField);
    assert_eq!(error.field(), "tool_use_id");
}

#[test]
fn hook_profiles_allow_additive_fields_but_bound_tool_payloads() {
    let mut additive = read_json("hooks-v1/pre-tool-use-bash.json");
    additive["future_optional_field"] = json!({"reviewed": true});
    CodexHooksV1
        .parse(&additive)
        .expect("unknown additive fields remain compatible");

    additive["tool_input"] = json!({"oversized": "x".repeat(65_536)});
    let error = CodexHooksV1.parse(&additive).unwrap_err();
    assert_eq!(error.code(), HostContractErrorCode::PayloadTooLarge);
    assert_eq!(error.field(), "tool_input");

    let mut additive_mcp = read_json("mcp-turn-metadata-v1/tools-call.json");
    additive_mcp["params"]["_meta"]["future_optional_field"] = json!({"reviewed": true});
    CodexMcpTurnMetadataV1
        .parse_tools_call(&additive_mcp)
        .expect("unknown additive MCP metadata remains compatible");
}

#[test]
fn managed_mcp_still_requires_thread_consistency() {
    let mut missing = read_json("mcp-turn-metadata-v1/tools-call.json");
    missing["params"]["_meta"]
        .as_object_mut()
        .unwrap()
        .remove("threadId");
    assert_eq!(
        CodexMcpTurnMetadataV1
            .parse_tools_call(&missing)
            .unwrap_err()
            .field(),
        "threadId"
    );

    let mut mismatch = read_json("mcp-turn-metadata-v1/tools-call.json");
    mismatch["params"]["_meta"]["threadId"] = json!("another-thread");
    assert_eq!(
        CodexMcpTurnMetadataV1
            .parse_tools_call(&mismatch)
            .unwrap_err()
            .code(),
        HostContractErrorCode::InconsistentCorrelation
    );
}

#[test]
fn pre_and_post_tool_names_remain_part_of_typed_correlation() {
    let pre = CodexHooksV1
        .parse(&read_json("hooks-v1/pre-tool-use-bash.json"))
        .unwrap();
    let mut mismatched = read_json("hooks-v1/post-tool-use-bash.json");
    mismatched["tool_name"] = json!("apply_patch");
    let post = CodexHooksV1.parse(&mismatched).unwrap();
    assert_ne!(pre.correlation(), post.correlation());
}
