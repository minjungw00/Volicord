use std::path::Path;

use serde_json::Value;

use super::{
    envelope::{event_bool, event_i64, event_string},
    mutation::{
        classify_tool, collect_path_assessments, collect_structured_path_assessments,
        has_structured_changed_paths, PathAssessment, ToolClassification,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ToolObservation {
    pub(super) tool_name: Option<String>,
    pub(super) host_invocation_id: Option<String>,
    pub(super) command: Option<String>,
    pub(super) classification: ToolClassification,
    pub(super) paths: Vec<PathAssessment>,
    pub(super) structured_paths: Vec<PathAssessment>,
    pub(super) changed_paths: Vec<PathAssessment>,
    pub(super) changed_paths_reported: bool,
    pub(super) explicit_write_attempt: bool,
    pub(super) reported_effect: Option<String>,
    pub(super) exit_code: Option<i64>,
    pub(super) success: Option<bool>,
    pub(super) status: Option<String>,
}

impl ToolObservation {
    pub(super) fn deterministic_write_attempt(&self) -> bool {
        (self.explicit_write_attempt
            || self.structured_reported_effect() == Some("product_file_write")
            || tool_name_is_direct_write(self.tool_name.as_deref())
            || self.classification == ToolClassification::Mutating)
            && !self.structured_paths.is_empty()
    }

    pub(super) fn deterministic_product_write_attempt(&self) -> bool {
        self.deterministic_write_attempt()
            && self.structured_paths.iter().any(|path| path.inside_repo)
    }

    pub(super) fn confidence(&self) -> &'static str {
        if self.changed_paths_reported || !self.changed_paths.is_empty() {
            "confirmed"
        } else if self.structured_reported_effect().is_some()
            || self.deterministic_product_write_attempt()
            || self.classification == ToolClassification::ReadOnly
        {
            "structured"
        } else if matches!(
            self.classification,
            ToolClassification::Mutating | ToolClassification::UnknownMutationRisk
        ) {
            "heuristic"
        } else {
            "unknown"
        }
    }

    pub(super) fn effect(&self) -> &'static str {
        if self.changed_paths.iter().any(|path| path.inside_repo) {
            "product_file_write"
        } else if self.changed_paths_reported {
            if self.classification == ToolClassification::ReadOnly {
                "read_only"
            } else {
                "unknown"
            }
        } else if let Some(effect) = self.structured_reported_effect() {
            effect
        } else if self.deterministic_product_write_attempt() {
            "product_file_write"
        } else if !self.structured_paths.is_empty()
            && self.structured_paths.iter().all(|path| !path.inside_repo)
            && (self.explicit_write_attempt || tool_name_is_direct_write(self.tool_name.as_deref()))
        {
            "non_product_write"
        } else if self.classification == ToolClassification::ReadOnly {
            "read_only"
        } else {
            "unknown"
        }
    }

    pub(super) fn structured_reported_effect(&self) -> Option<&'static str> {
        if self.structured_paths.is_empty() {
            return None;
        }
        match self.reported_effect.as_deref() {
            Some("read_only") => Some("read_only"),
            Some("product_file_write") => Some("product_file_write"),
            Some("non_product_write") => Some("non_product_write"),
            Some("external_effect") => Some("external_effect"),
            _ => None,
        }
    }
}

pub(super) fn tool_name_is_direct_write(tool_name: Option<&str>) -> bool {
    tool_name
        .map(|name| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                "edit" | "write" | "write_file" | "apply_patch" | "patch" | "notebook_edit"
            )
        })
        .unwrap_or(false)
}

pub(super) fn tool_observation(event: &Value, repo_root: &Path) -> ToolObservation {
    let tool_name = event_string(
        event,
        &[
            &["tool_name"],
            &["tool", "name"],
            &["tool_use", "name"],
            &["tool"],
        ],
    );
    let command = event_string(
        event,
        &[
            &["command"],
            &["tool_input", "command"],
            &["input", "command"],
            &["tool", "input", "command"],
            &["tool", "arguments", "command"],
            &["tool_use", "input", "command"],
        ],
    );
    let classification = classify_tool(tool_name.as_deref(), command.as_deref());
    let paths = collect_path_assessments(event, repo_root, false);
    let structured_paths = collect_structured_path_assessments(event, repo_root, false);
    let changed_paths = collect_path_assessments(event, repo_root, true);
    let changed_paths_reported = has_structured_changed_paths(event);
    let explicit_write_attempt = event_bool(
        event,
        &[
            &["product_file_write_intended"],
            &["write_attempt"],
            &["mutates_files"],
            &["tool_input", "product_file_write_intended"],
            &["tool_input", "write_attempt"],
            &["input", "product_file_write_intended"],
            &["input", "write_attempt"],
        ],
    )
    .unwrap_or(false);
    let reported_effect = event_string(
        event,
        &[
            &["effect"],
            &["tool", "effect"],
            &["tool_input", "effect"],
            &["input", "effect"],
            &["mutation", "effect"],
        ],
    )
    .filter(|effect| {
        matches!(
            effect.as_str(),
            "read_only" | "product_file_write" | "non_product_write" | "external_effect"
        )
    });
    ToolObservation {
        tool_name,
        host_invocation_id: host_invocation_id(event),
        command,
        classification,
        paths,
        structured_paths,
        changed_paths,
        changed_paths_reported,
        explicit_write_attempt,
        reported_effect,
        exit_code: event_i64(
            event,
            &[
                &["exit_code"],
                &["tool_response", "exit_code"],
                &["tool_result", "exit_code"],
                &["result", "exit_code"],
                &["output", "exit_code"],
            ],
        ),
        success: event_bool(
            event,
            &[
                &["success"],
                &["tool_response", "success"],
                &["tool_result", "success"],
                &["result", "success"],
                &["output", "success"],
            ],
        ),
        status: event_string(
            event,
            &[
                &["status"],
                &["tool_response", "status"],
                &["tool_result", "status"],
                &["result", "status"],
                &["output", "status"],
            ],
        ),
    }
}

pub(super) fn host_invocation_id(event: &Value) -> Option<String> {
    event_string(
        event,
        &[
            &["tool_call_id"],
            &["tool_use_id"],
            &["tool_invocation_id"],
            &["invocation_id"],
            &["call_id"],
            &["tool", "call_id"],
            &["tool", "id"],
            &["tool_use", "id"],
            &["tool_result", "tool_call_id"],
            &["result", "tool_call_id"],
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maintained_post_tool_fixtures_read_tool_response_success_and_exit_code() {
        let fixture = include_str!(
            "../../../../tests/conformance/codex-host/hooks-v1/post-tool-use-bash.json"
        );
        let event: Value = serde_json::from_str(fixture).expect("fixture JSON");
        let observation = tool_observation(&event, Path::new("/repo"));
        assert_eq!(observation.success, Some(true));
        assert_eq!(observation.exit_code, Some(0));
    }

    #[test]
    fn unknown_tools_honor_closed_structured_effects_only_with_known_paths() {
        let product = tool_observation(
            &serde_json::json!({
                "tool_name": "custom_host_tool",
                "effect": "product_file_write",
                "paths": ["src/lib.rs"]
            }),
            Path::new("/repo"),
        );
        assert!(product.deterministic_write_attempt());
        assert!(product.deterministic_product_write_attempt());
        assert_eq!(product.effect(), "product_file_write");
        assert_eq!(product.confidence(), "structured");

        let external = tool_observation(
            &serde_json::json!({
                "tool_name": "custom_host_tool",
                "effect": "external_effect",
                "paths": ["https://example.invalid/resource"]
            }),
            Path::new("/repo"),
        );
        assert!(!external.deterministic_write_attempt());
        assert_eq!(external.effect(), "external_effect");
        assert_eq!(external.confidence(), "structured");

        let missing_paths = tool_observation(
            &serde_json::json!({
                "tool_name": "custom_host_tool",
                "effect": "product_file_write"
            }),
            Path::new("/repo"),
        );
        assert!(!missing_paths.deterministic_write_attempt());
        assert_eq!(missing_paths.effect(), "unknown");
    }
}
