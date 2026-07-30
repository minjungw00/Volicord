use std::path::Path;

use serde_json::Value;
use volicord_host_contract::{
    CodexCommandHooks, CodexGuardToolEffectContract, CodexHookEvent, GuardToolIdentity,
    McpServerKey, ToolTargetPathUnavailableReason, ToolTargetPaths,
};
use volicord_types::tool_names::ProductRepositoryEffect;

use super::{
    mutation::{assess_decoded_paths, PathAssessment},
    GuardCommandError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ToolObservation {
    pub(super) tool_name: Option<String>,
    pub(super) identity: GuardToolIdentity,
    pub(super) host_invocation_id: Option<String>,
    pub(super) command: Option<String>,
    pub(super) prospective_effect: ProductRepositoryEffect,
    pub(super) target_path_unavailable_reason: Option<ToolTargetPathUnavailableReason>,
    pub(super) paths: Vec<PathAssessment>,
    pub(super) structured_paths: Vec<PathAssessment>,
    pub(super) changed_paths: Vec<PathAssessment>,
    pub(super) changed_paths_reported: bool,
    pub(super) exit_code: Option<i64>,
    pub(super) success: Option<bool>,
    pub(super) status: Option<String>,
}

impl ToolObservation {
    pub(super) fn deterministic_write_attempt(&self) -> bool {
        self.prospective_effect == ProductRepositoryEffect::MayWriteProduct
            && !self.structured_paths.is_empty()
    }

    pub(super) fn deterministic_product_write_attempt(&self) -> bool {
        self.deterministic_write_attempt()
            && self.structured_paths.iter().any(|path| path.inside_repo)
    }

    pub(super) fn confidence(&self) -> &'static str {
        if self.changed_paths_reported {
            "confirmed"
        } else if self.prospective_effect != ProductRepositoryEffect::UnknownProductEffect {
            "structured"
        } else {
            "unknown"
        }
    }

    pub(super) fn observed_effect(&self) -> &'static str {
        if self.changed_paths.iter().any(|path| path.inside_repo) {
            "product_file_write"
        } else if self.changed_paths_reported {
            "no_product_write"
        } else {
            "unknown_product_effect"
        }
    }

    pub(super) fn target_path_status(&self) -> &'static str {
        if self.target_path_unavailable_reason.is_some() {
            "unavailable"
        } else if self.structured_paths.is_empty() {
            "not_applicable"
        } else {
            "exact"
        }
    }

    pub(super) fn identity_kind(&self) -> &'static str {
        match self.identity {
            GuardToolIdentity::CodexNative(_) => "codex_native",
            GuardToolIdentity::VolicordMcp(_) => "volicord_mcp",
            GuardToolIdentity::Foreign => "foreign",
        }
    }

    pub(super) fn canonical_identity(&self) -> Option<&str> {
        match &self.identity {
            GuardToolIdentity::CodexNative(tool) => Some(tool.as_str()),
            GuardToolIdentity::VolicordMcp(identity) => Some(identity.tool().wire_name()),
            GuardToolIdentity::Foreign => None,
        }
    }
}

pub(super) fn tool_observation(
    event: &Value,
    repo_root: &Path,
    server: &McpServerKey,
) -> Result<ToolObservation, GuardCommandError> {
    let event = CodexCommandHooks
        .parse(event)
        .map_err(|error| GuardCommandError::Runtime(error.to_string()))?;
    let (correlation, tool_input, tool_response) = match event {
        CodexHookEvent::PreToolUse {
            correlation,
            tool_input,
            ..
        } => (correlation, tool_input, None),
        CodexHookEvent::PostToolUse {
            correlation,
            tool_input,
            tool_response,
            ..
        } => (correlation, tool_input, Some(tool_response)),
        CodexHookEvent::UserPromptSubmit { .. } => {
            return Err(GuardCommandError::Runtime(
                "tool observation requires a Codex tool hook event".to_owned(),
            ));
        }
    };
    let contract = CodexGuardToolEffectContract::for_server(server)
        .map_err(|error| GuardCommandError::Runtime(error.to_string()))?;
    let assessment = contract.assess(&correlation.tool_name, tool_input.as_value());
    let paths = assess_decoded_paths(repo_root, assessment.target_paths().exact());
    let target_path_unavailable_reason = match assessment.target_paths() {
        ToolTargetPaths::Unavailable(reason) => Some(*reason),
        ToolTargetPaths::NotApplicable | ToolTargetPaths::Exact(_) => None,
    };
    let (success, exit_code, status) = tool_response
        .as_ref()
        .and_then(|response| response.as_value().as_object())
        .map(|response| {
            (
                response.get("success").and_then(Value::as_bool),
                response.get("exit_code").and_then(Value::as_i64),
                response
                    .get("status")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            )
        })
        .unwrap_or((None, None, None));
    Ok(ToolObservation {
        tool_name: Some(correlation.tool_name.as_str().to_owned()),
        identity: assessment.identity().clone(),
        host_invocation_id: Some(correlation.tool_use_id.as_str().to_owned()),
        command: assessment.command().map(str::to_owned),
        prospective_effect: assessment.effect(),
        target_path_unavailable_reason,
        paths: paths.clone(),
        structured_paths: paths,
        changed_paths: Vec::new(),
        changed_paths_reported: false,
        exit_code,
        success,
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(tool_name: &str, tool_input: Value) -> Value {
        serde_json::json!({
            "hook_event_name": "PreToolUse",
            "session_id": "session-1",
            "turn_id": "turn-1",
            "tool_use_id": "tool-1",
            "tool_name": tool_name,
            "tool_input": tool_input
        })
    }

    #[test]
    fn maintained_post_tool_fixture_uses_exact_response_fields() {
        let fixture = include_str!(
            "../../../../tests/conformance/codex-host/command-hooks/post-tool-use-bash.json"
        );
        let event: Value = serde_json::from_str(fixture).expect("fixture JSON");
        let server = McpServerKey::parse("volicord").unwrap();
        let observation = tool_observation(&event, Path::new("/repo"), &server).unwrap();
        assert_eq!(observation.success, Some(true));
        assert_eq!(observation.exit_code, Some(0));
        assert_eq!(
            observation.prospective_effect,
            ProductRepositoryEffect::UnknownProductEffect
        );
    }

    #[test]
    fn exact_write_target_decoding_rejects_external_and_ignores_recursive_fields() {
        let server = McpServerKey::parse("volicord").unwrap();
        let external = tool_observation(
            &event(
                "Write",
                serde_json::json!({"file_path": "/outside/file.rs"}),
            ),
            Path::new("/repo"),
            &server,
        )
        .unwrap();
        assert_eq!(
            external.prospective_effect,
            ProductRepositoryEffect::MayWriteProduct
        );
        assert_eq!(external.structured_paths.len(), 1);
        assert!(!external.structured_paths[0].inside_repo);

        let recursive = tool_observation(
            &event(
                "Write",
                serde_json::json!({
                    "metadata": {
                        "file_path": "src/lib.rs",
                        "path": "src/path.rs",
                        "changed_paths": ["src/changed.rs"]
                    }
                }),
            ),
            Path::new("/repo"),
            &server,
        )
        .unwrap();
        assert!(recursive.structured_paths.is_empty());
        assert_eq!(
            recursive.target_path_unavailable_reason,
            Some(ToolTargetPathUnavailableReason::MissingRequiredField)
        );
    }

    #[test]
    fn known_mcp_runtime_mutations_are_not_product_write_attempts() {
        let server = McpServerKey::parse("registered-server").unwrap();
        let contract = CodexGuardToolEffectContract::for_server(&server).unwrap();
        for tool in volicord_types::tool_names::AgentToolId::ALL {
            let callable = contract
                .mcp_catalog()
                .require(&server, tool)
                .unwrap()
                .callable_name()
                .as_str()
                .to_owned();
            let observation = tool_observation(
                &event(&callable, serde_json::json!({})),
                Path::new("/repo"),
                &server,
            )
            .unwrap();
            assert_eq!(
                observation.prospective_effect,
                ProductRepositoryEffect::NoProductWrite
            );
            assert!(!observation.deterministic_write_attempt());
        }
    }
}
