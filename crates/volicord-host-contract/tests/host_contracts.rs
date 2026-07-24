use std::{collections::BTreeSet, fs, path::PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_host_contract::{
    parse_callable_name, project_mcp_tool, CanonicalToolName, CodexCommandHooks, CodexHookEvent,
    CodexMcpTurnMetadata, HookObservationPolicy, HostCallableName, HostContractErrorCode,
    HostContractProfileId, HostHookMatcherStrategy, HostNativeCorrelation, McpServerKey,
    McpToolCatalog,
};
use volicord_types::AgentToolId;

const REVIEWED_GUARD_PROBE_CALLABLE: &str = "mcp__volicord__volicord_guard_probe";

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
    purpose: String,
    source_reference: String,
    checksum_sha256: String,
    production_covered: bool,
}

#[derive(Debug, Deserialize)]
struct CallableFixture {
    profile_id: String,
    catalog: Vec<CallableFixtureEntry>,
    normalization_cases: Vec<CallableFixtureEntry>,
}

#[derive(Debug, Deserialize)]
struct CallableFixtureEntry {
    server_key: String,
    raw_tool_name: String,
    expected_callable_name: String,
    purpose: String,
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
            "codex-command-hooks" => {
                CodexCommandHooks
                    .parse(&payload)
                    .expect("hook fixture must parse");
            }
            "codex-mcp-turn-metadata" => {
                CodexMcpTurnMetadata
                    .parse_tools_call(&payload)
                    .expect("MCP fixture must parse");
            }
            "codex-mcp-callable-names" => {
                let fixture: CallableFixture =
                    serde_json::from_value(payload).expect("callable fixture");
                assert_eq!(fixture.profile_id, "codex-mcp-callable-names");
                assert!(!fixture.catalog.is_empty());
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
    let prompt = CodexCommandHooks
        .parse(&read_json("command-hooks/user-prompt-submit.json"))
        .expect("prompt fixture");
    assert!(matches!(
        prompt.correlation(),
        HostNativeCorrelation::CodexHookPrompt(_)
    ));
    let pre = CodexCommandHooks
        .parse(&read_json("command-hooks/pre-tool-use-bash.json"))
        .expect("pre fixture");
    let post = CodexCommandHooks
        .parse(&read_json("command-hooks/post-tool-use-bash.json"))
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
    let pre = CodexCommandHooks
        .parse(&read_json("command-hooks/pre-tool-use-mcp.json"))
        .expect("Guard probe pre-tool fixture");
    let post = CodexCommandHooks
        .parse(&read_json("command-hooks/post-tool-use-mcp.json"))
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
    let server = McpServerKey::parse("volicord").unwrap();
    let catalog = McpToolCatalog::for_server(&server, AgentToolId::ALL).unwrap();
    let callable = HostCallableName::parse(pre.tool_name.as_str()).unwrap();
    assert_eq!(
        parse_callable_name(&callable, &catalog).unwrap().tool(),
        AgentToolId::GUARD_PROBE
    );
    assert_eq!(pre.tool_name.as_str(), REVIEWED_GUARD_PROBE_CALLABLE);
    assert_eq!(
        pre_input.as_value(),
        &json!({"verification_id": "guard_verification_fixture_001"})
    );
    assert_eq!(pre_input, post_input);
}

#[test]
fn reviewed_guard_fixture_covers_the_complete_callable_identity_chain() {
    let fixture: CallableFixture =
        serde_json::from_value(read_json("callable-names/mcp-tools.json")).unwrap();
    let reviewed = fixture
        .catalog
        .iter()
        .find(|entry| entry.raw_tool_name == "volicord.guard_probe")
        .expect("reviewed Guard probe callable fixture");
    assert_eq!(reviewed.server_key, "volicord");
    assert_eq!(reviewed.raw_tool_name, "volicord.guard_probe");
    assert_eq!(
        reviewed.expected_callable_name,
        REVIEWED_GUARD_PROBE_CALLABLE
    );

    let server = McpServerKey::parse(reviewed.server_key.clone()).unwrap();
    let tool = AgentToolId::from_wire_name(&reviewed.raw_tool_name).unwrap();
    assert_eq!(tool, AgentToolId::GUARD_PROBE);
    let catalog = McpToolCatalog::for_server(&server, AgentToolId::ALL).unwrap();
    let projected = catalog.require(&server, tool).unwrap();
    assert_eq!(projected.source().server(), &server);
    assert_eq!(projected.source().tool(), tool);
    assert_eq!(
        projected.source().raw_tool_name().as_str(),
        reviewed.raw_tool_name
    );
    assert_eq!(
        projected.callable_name().as_str(),
        reviewed.expected_callable_name
    );
    assert_eq!(
        parse_callable_name(projected.callable_name(), &catalog).unwrap(),
        projected.source().clone()
    );

    let matcher = HostHookMatcherStrategy::codex_guard(&server).unwrap();
    assert_eq!(
        matcher.codex_matcher().unwrap(),
        "Bash|apply_patch|Edit|Write|mcp__volicord__.*"
    );
    assert!(
        matcher.routes(&CanonicalToolName::parse(reviewed.expected_callable_name.clone()).unwrap())
    );

    let pre = CodexCommandHooks
        .parse(&read_json("command-hooks/pre-tool-use-mcp.json"))
        .expect("reviewed PreToolUse fixture");
    let post = CodexCommandHooks
        .parse(&read_json("command-hooks/post-tool-use-mcp.json"))
        .expect("reviewed PostToolUse fixture");
    let (
        CodexHookEvent::PreToolUse {
            correlation: pre, ..
        },
        CodexHookEvent::PostToolUse {
            correlation: post, ..
        },
    ) = (pre, post)
    else {
        panic!("reviewed Guard fixtures must remain tool-hook events");
    };
    for parsed in [&pre, &post] {
        assert_eq!(parsed.tool_name.as_str(), REVIEWED_GUARD_PROBE_CALLABLE);
        let callable = HostCallableName::parse(parsed.tool_name.as_str()).unwrap();
        let source = parse_callable_name(&callable, &catalog).unwrap();
        assert_eq!(source.server(), &server);
        assert_eq!(source.raw_tool_name().as_str(), reviewed.raw_tool_name);
        assert_eq!(source.tool(), AgentToolId::GUARD_PROBE);
    }
    assert_eq!(pre, post);
}

#[test]
fn reviewed_codex_hook_profile_uses_one_synchronous_status_read() {
    assert_eq!(
        HostContractProfileId::CodexCommandHooks.hook_observation_policy(),
        Some(HookObservationPolicy::Synchronous {
            allowed_status_reads: 1,
        })
    );
    assert!(HostContractProfileId::CodexMcpTurnMetadata
        .hook_observation_policy()
        .is_none());
    assert!(HostContractProfileId::CodexMcpCallableNames
        .hook_observation_policy()
        .is_none());
}

#[test]
fn mcp_and_hook_correlation_are_not_interchangeable() {
    let mcp = CodexMcpTurnMetadata
        .parse_tools_call(&read_json("mcp-turn-metadata/tools-call.json"))
        .expect("MCP fixture");
    let hook = CodexCommandHooks
        .parse(&read_json("command-hooks/pre-tool-use-mcp.json"))
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
    let mut missing_session = read_json("command-hooks/pre-tool-use-mcp.json");
    missing_session
        .as_object_mut()
        .unwrap()
        .remove("session_id");
    let error = CodexCommandHooks.parse(&missing_session).unwrap_err();
    assert_eq!(error.code(), HostContractErrorCode::MissingRequiredField);
    assert_eq!(error.field(), "session_id");

    let mut missing_turn = read_json("command-hooks/user-prompt-submit.json");
    missing_turn.as_object_mut().unwrap().remove("turn_id");
    let error = CodexCommandHooks.parse(&missing_turn).unwrap_err();
    assert_eq!(error.code(), HostContractErrorCode::MissingRequiredField);
    assert_eq!(error.field(), "turn_id");

    let mut malformed_tool = read_json("command-hooks/pre-tool-use-bash.json");
    malformed_tool["tool_use_id"] = json!("tool use with spaces");
    let error = CodexCommandHooks.parse(&malformed_tool).unwrap_err();
    assert_eq!(error.code(), HostContractErrorCode::InvalidField);
    assert_eq!(error.field(), "tool_use_id");
}

#[test]
fn hook_profiles_allow_additive_fields_but_bound_tool_payloads() {
    let mut additive = read_json("command-hooks/pre-tool-use-bash.json");
    additive["future_optional_field"] = json!({"reviewed": true});
    CodexCommandHooks
        .parse(&additive)
        .expect("unknown additive fields remain compatible");

    additive["tool_input"] = json!({"oversized": "x".repeat(65_536)});
    let error = CodexCommandHooks.parse(&additive).unwrap_err();
    assert_eq!(error.code(), HostContractErrorCode::PayloadTooLarge);
    assert_eq!(error.field(), "tool_input");

    let mut additive_mcp = read_json("mcp-turn-metadata/tools-call.json");
    additive_mcp["params"]["_meta"]["future_optional_field"] = json!({"reviewed": true});
    CodexMcpTurnMetadata
        .parse_tools_call(&additive_mcp)
        .expect("unknown additive MCP metadata remains compatible");
}

#[test]
fn managed_mcp_still_requires_thread_consistency() {
    let mut missing = read_json("mcp-turn-metadata/tools-call.json");
    missing["params"]["_meta"]
        .as_object_mut()
        .unwrap()
        .remove("threadId");
    assert_eq!(
        CodexMcpTurnMetadata
            .parse_tools_call(&missing)
            .unwrap_err()
            .field(),
        "threadId"
    );

    let mut mismatch = read_json("mcp-turn-metadata/tools-call.json");
    mismatch["params"]["_meta"]["threadId"] = json!("another-thread");
    assert_eq!(
        CodexMcpTurnMetadata
            .parse_tools_call(&mismatch)
            .unwrap_err()
            .code(),
        HostContractErrorCode::InconsistentCorrelation
    );
}

#[test]
fn pre_and_post_tool_names_remain_part_of_typed_correlation() {
    let pre = CodexCommandHooks
        .parse(&read_json("command-hooks/pre-tool-use-bash.json"))
        .unwrap();
    let mut mismatched = read_json("command-hooks/post-tool-use-bash.json");
    mismatched["tool_name"] = json!("apply_patch");
    let post = CodexCommandHooks.parse(&mismatched).unwrap();
    assert_ne!(pre.correlation(), post.correlation());
}

#[test]
fn server_key_and_complete_raw_tool_name_are_distinct_typed_coordinates() {
    let server = McpServerKey::parse("registered-server").unwrap();
    let projected = project_mcp_tool(&server, AgentToolId::GUARD_PROBE).unwrap();
    assert_eq!(
        projected.profile(),
        HostContractProfileId::CodexMcpCallableNames
    );
    assert_eq!(projected.source().server().as_str(), "registered-server");
    assert_eq!(
        projected.source().raw_tool_name().as_str(),
        "volicord.guard_probe"
    );
    assert_eq!(
        projected.callable_name().as_str(),
        "mcp__registered_server__volicord_guard_probe"
    );

    let unrelated = McpServerKey::parse("unrelated-server").unwrap();
    let catalog = McpToolCatalog::new([
        (server.clone(), AgentToolId::GUARD_PROBE),
        (unrelated.clone(), AgentToolId::GUARD_PROBE),
    ])
    .unwrap();
    let registered = catalog.require(&server, AgentToolId::GUARD_PROBE).unwrap();
    let unrelated = catalog
        .require(&unrelated, AgentToolId::GUARD_PROBE)
        .unwrap();
    assert_ne!(registered.callable_name(), unrelated.callable_name());
    assert_eq!(
        parse_callable_name(unrelated.callable_name(), &catalog)
            .unwrap()
            .server()
            .as_str(),
        "unrelated-server"
    );
}

#[test]
fn all_public_tools_form_one_collision_free_catalog_and_round_trip_exactly() {
    let server = McpServerKey::parse("volicord").unwrap();
    let catalog = McpToolCatalog::for_server(&server, AgentToolId::ALL).unwrap();
    assert_eq!(catalog.identities().len(), AgentToolId::ALL.len());
    for tool in AgentToolId::ALL {
        let identity = catalog.require(&server, tool).unwrap();
        assert_eq!(identity.source().raw_tool_name().as_str(), tool.wire_name());
        assert_eq!(
            parse_callable_name(identity.callable_name(), &catalog).unwrap(),
            identity.source().clone()
        );
    }
}

#[test]
fn normalized_server_collision_fails_catalog_construction_with_typed_error() {
    let error = McpToolCatalog::new([
        (
            McpServerKey::parse("collision.server").unwrap(),
            AgentToolId::STATUS,
        ),
        (
            McpServerKey::parse("collision-server").unwrap(),
            AgentToolId::STATUS,
        ),
    ])
    .unwrap_err();
    assert_eq!(error.code(), HostContractErrorCode::CallableNameCollision);
    assert_eq!(error.field(), "host_callable_name");
}

#[test]
fn malformed_and_unknown_callable_names_fail_with_distinct_typed_errors() {
    let malformed = HostCallableName::parse("mcp__bad.name").unwrap_err();
    assert_eq!(malformed.code(), HostContractErrorCode::InvalidField);

    let server = McpServerKey::parse("volicord").unwrap();
    let catalog = McpToolCatalog::for_server(&server, AgentToolId::ALL).unwrap();
    let unknown = HostCallableName::parse("mcp__volicord__unknown").unwrap();
    let error = parse_callable_name(&unknown, &catalog).unwrap_err();
    assert_eq!(error.code(), HostContractErrorCode::UnknownCallableName);
}

#[test]
fn callable_fixture_matches_the_complete_public_catalog_and_reviewed_cases() {
    let fixture: CallableFixture =
        serde_json::from_value(read_json("callable-names/mcp-tools.json")).unwrap();
    assert_eq!(
        fixture.profile_id,
        HostContractProfileId::CodexMcpCallableNames.as_str()
    );
    let fixture_tools = fixture
        .catalog
        .iter()
        .map(|entry| entry.raw_tool_name.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        fixture_tools,
        AgentToolId::ALL
            .into_iter()
            .map(AgentToolId::wire_name)
            .collect::<BTreeSet<_>>()
    );
    for entry in fixture.catalog.iter().chain(&fixture.normalization_cases) {
        assert!(!entry.purpose.trim().is_empty());
        let server = McpServerKey::parse(&entry.server_key).unwrap();
        let tool = AgentToolId::from_wire_name(&entry.raw_tool_name).unwrap();
        let projected = project_mcp_tool(&server, tool).unwrap();
        assert_eq!(
            projected.callable_name().as_str(),
            entry.expected_callable_name
        );
    }
    let server = McpServerKey::parse("volicord").unwrap();
    let matcher = HostHookMatcherStrategy::codex_guard(&server).unwrap();
    for entry in &fixture.catalog {
        assert!(
            matcher.routes(&CanonicalToolName::parse(&entry.expected_callable_name).unwrap()),
            "generated matcher and fixture projection differ for {}",
            entry.raw_tool_name
        );
    }
}

#[test]
fn callable_projection_is_bounded_and_semantic_profiles_have_no_numeric_variants() {
    let server = McpServerKey::parse("a".repeat(80)).unwrap();
    let first = project_mcp_tool(&server, AgentToolId::GET_INTEGRATION_VERIFICATION).unwrap();
    let second = project_mcp_tool(&server, AgentToolId::GET_INTEGRATION_VERIFICATION).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first.callable_name().as_str(),
        "mcp__aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa___b125f360c65b"
    );
    let strategy = HostHookMatcherStrategy::codex_guard(&server).unwrap();
    assert!(matches!(
        &strategy,
        HostHookMatcherStrategy::Union(strategies)
            if matches!(
                strategies.get(1),
                Some(HostHookMatcherStrategy::ExactCallables { callables })
                    if callables.len() == AgentToolId::ALL.len()
            )
    ));
    let rendered = strategy.codex_matcher().unwrap();
    assert_eq!(
        HostHookMatcherStrategy::parse_codex_guard(&rendered, &server).unwrap(),
        strategy
    );
    assert!(strategy.routes(&CanonicalToolName::parse(first.callable_name().as_str()).unwrap()));
    let foreign_server = McpServerKey::parse(format!("{}b", "a".repeat(79))).unwrap();
    let foreign =
        project_mcp_tool(&foreign_server, AgentToolId::GET_INTEGRATION_VERIFICATION).unwrap();
    assert!(!strategy.routes(&CanonicalToolName::parse(foreign.callable_name().as_str()).unwrap()));
    assert!(HostContractProfileId::ALL.iter().all(|profile| !profile
        .as_str()
        .chars()
        .any(|character| character.is_ascii_digit())));
}

#[test]
fn guard_matcher_routes_typed_host_tools_and_the_registered_server_namespace() {
    let server = McpServerKey::parse("volicord").unwrap();
    let strategy = HostHookMatcherStrategy::codex_guard(&server).unwrap();
    let rendered = strategy.codex_matcher().unwrap();
    assert_eq!(rendered, "Bash|apply_patch|Edit|Write|mcp__volicord__.*");
    assert_eq!(
        HostHookMatcherStrategy::parse_codex_guard(&rendered, &server).unwrap(),
        strategy
    );

    let catalog = McpToolCatalog::for_server(&server, AgentToolId::ALL).unwrap();
    for tool in [AgentToolId::GUARD_PROBE, AgentToolId::STATUS] {
        let observed = CanonicalToolName::parse(
            catalog
                .require(&server, tool)
                .unwrap()
                .callable_name()
                .as_str(),
        )
        .unwrap();
        assert!(strategy.routes(&observed));
    }
    assert!(strategy.routes(&CanonicalToolName::parse("Bash").unwrap()));
    assert!(strategy
        .routes(&CanonicalToolName::parse("mcp__volicord__unknown_same_server_tool").unwrap()));
    assert!(
        !strategy.routes(&CanonicalToolName::parse("mcp__foreign__volicord_guard_probe").unwrap())
    );
}

#[test]
fn guard_matcher_reconstruction_rejects_drift_and_has_no_numeric_profile_branch() {
    let server = McpServerKey::parse("volicord").unwrap();
    for drifted in [
        "Bash|apply_patch|Edit|Write|mcp__foreign__.*",
        "Bash|apply_patch|Edit|Write|mcp__volicord__volicord_guard_probe",
        "Bash|apply_patch|Edit|mcp__volicord__.*",
    ] {
        let reconstructed = HostHookMatcherStrategy::parse_codex_guard(drifted, &server);
        assert!(
            reconstructed.is_err()
                || reconstructed.unwrap() != HostHookMatcherStrategy::codex_guard(&server).unwrap()
        );
    }
    assert!(HostContractProfileId::ALL.iter().all(|profile| !profile
        .as_str()
        .chars()
        .any(|character| character.is_ascii_digit())));
}
