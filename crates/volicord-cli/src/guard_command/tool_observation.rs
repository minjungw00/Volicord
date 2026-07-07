use std::path::Path;

use serde_json::Value;

use super::{
    envelope::{event_bool, event_i64, event_string},
    mutation::{classify_tool, collect_path_assessments, PathAssessment, ToolClassification},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ToolObservation {
    pub(super) tool_name: Option<String>,
    pub(super) host_invocation_id: Option<String>,
    pub(super) command: Option<String>,
    pub(super) classification: ToolClassification,
    pub(super) paths: Vec<PathAssessment>,
    pub(super) changed_paths: Vec<PathAssessment>,
    pub(super) explicit_write_attempt: bool,
    pub(super) exit_code: Option<i64>,
    pub(super) success: Option<bool>,
    pub(super) status: Option<String>,
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
    let changed_paths = collect_path_assessments(event, repo_root, true);
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
    ToolObservation {
        tool_name,
        host_invocation_id: host_invocation_id(event),
        command,
        classification,
        paths,
        changed_paths,
        explicit_write_attempt,
        exit_code: event_i64(
            event,
            &[
                &["exit_code"],
                &["tool_result", "exit_code"],
                &["result", "exit_code"],
                &["output", "exit_code"],
            ],
        ),
        success: event_bool(
            event,
            &[
                &["success"],
                &["tool_result", "success"],
                &["result", "success"],
                &["output", "success"],
            ],
        ),
        status: event_string(
            event,
            &[
                &["status"],
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
