use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

use serde_json::Value;

use super::envelope::event_string;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolClassification {
    ReadOnly,
    Mutating,
    UnknownMutationRisk,
    Unknown,
}

impl ToolClassification {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::Mutating => "mutating",
            Self::UnknownMutationRisk => "unknown_mutation_risk",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PathAssessment {
    pub(super) raw: String,
    pub(super) normalized: Option<String>,
    pub(super) inside_repo: bool,
}

pub(super) fn classify_tool(tool_name: Option<&str>, command: Option<&str>) -> ToolClassification {
    let normalized_tool = tool_name.unwrap_or("").trim().to_ascii_lowercase();
    if matches!(
        normalized_tool.as_str(),
        "read" | "view" | "grep" | "search" | "list" | "glob"
    ) {
        return ToolClassification::ReadOnly;
    }
    if matches!(
        normalized_tool.as_str(),
        "edit" | "write" | "write_file" | "apply_patch" | "patch" | "notebook_edit"
    ) {
        return ToolClassification::Mutating;
    }
    let Some(command) = command.map(str::trim).filter(|value| !value.is_empty()) else {
        return if normalized_tool.is_empty() {
            ToolClassification::Unknown
        } else {
            ToolClassification::UnknownMutationRisk
        };
    };
    if shell_command_has_complex_syntax(command) {
        return ToolClassification::UnknownMutationRisk;
    }
    if shell_command_is_clearly_mutating(command) {
        return ToolClassification::Mutating;
    }
    if shell_command_is_read_only(command) {
        return ToolClassification::ReadOnly;
    }
    ToolClassification::UnknownMutationRisk
}

fn shell_command_has_complex_syntax(command: &str) -> bool {
    command.chars().any(|character| {
        matches!(
            character,
            '\n' | '\r' | '|' | ';' | '&' | '<' | '>' | '`' | '\'' | '"' | '\\'
        )
    }) || command.contains("$(")
}

fn shell_command_is_clearly_mutating(command: &str) -> bool {
    let words = command_words(command);
    let Some(first) = words.first().map(String::as_str) else {
        return false;
    };
    if matches!(
        first,
        "rm" | "mv"
            | "cp"
            | "touch"
            | "mkdir"
            | "rmdir"
            | "ln"
            | "chmod"
            | "chown"
            | "truncate"
            | "install"
            | "cargo-fmt"
            | "tee"
    ) {
        return true;
    }
    matches!(
        (first, words.get(1).map(String::as_str)),
        ("sed", Some("-i"))
            | ("perl", Some("-pi"))
            | ("cargo", Some("fmt"))
            | (
                "git",
                Some(
                    "add"
                        | "commit"
                        | "reset"
                        | "clean"
                        | "checkout"
                        | "switch"
                        | "rm"
                        | "mv"
                        | "apply"
                        | "merge"
                        | "rebase"
                        | "cherry-pick"
                        | "restore"
                )
            )
    )
}

fn shell_command_is_read_only(command: &str) -> bool {
    let words = command_words(command);
    let Some(first) = words.first().map(String::as_str) else {
        return false;
    };
    match first {
        "pwd" | "ls" | "cat" | "rg" | "grep" | "wc" | "head" | "tail" => true,
        "git" => matches!(
            words.get(1).map(String::as_str),
            Some("status" | "diff" | "log" | "show" | "rev-parse")
        ),
        "cargo" => words.get(1).map(String::as_str) == Some("metadata"),
        _ => false,
    }
}

fn command_words(command: &str) -> Vec<String> {
    let mut words = command.split_whitespace();
    let Some(first) = words.next() else {
        return Vec::new();
    };
    if first == "sudo" || first == "command" {
        words.map(str::to_owned).collect()
    } else {
        std::iter::once(first.to_owned())
            .chain(words.map(str::to_owned))
            .collect()
    }
}

pub(super) fn collect_path_assessments(
    event: &Value,
    repo_root: &Path,
    changed_only: bool,
) -> Vec<PathAssessment> {
    let mut raw_paths = BTreeSet::new();
    collect_paths_recursive(event, changed_only, &mut raw_paths);
    if !changed_only {
        raw_paths.extend(paths_from_apply_patch(event));
        if let Some(command) = event_string(
            event,
            &[
                &["command"],
                &["tool_input", "command"],
                &["input", "command"],
                &["tool", "input", "command"],
            ],
        ) {
            raw_paths.extend(paths_from_redirection(&command));
        }
    }
    raw_paths
        .into_iter()
        .map(|raw| assess_path(repo_root, &raw))
        .collect()
}

pub(super) fn collect_structured_path_assessments(
    event: &Value,
    repo_root: &Path,
    changed_only: bool,
) -> Vec<PathAssessment> {
    let mut raw_paths = BTreeSet::new();
    collect_paths_recursive(event, changed_only, &mut raw_paths);
    if !changed_only {
        raw_paths.extend(paths_from_apply_patch(event));
    }
    raw_paths
        .into_iter()
        .map(|raw| assess_path(repo_root, &raw))
        .collect()
}

pub(super) fn has_structured_changed_paths(event: &Value) -> bool {
    has_path_field_recursive(event, true)
}

fn paths_from_apply_patch(event: &Value) -> Vec<String> {
    let tool_name = event_string(
        event,
        &[&["tool_name"], &["tool", "name"], &["tool", "tool_name"]],
    )
    .unwrap_or_default()
    .to_ascii_lowercase();
    if !matches!(tool_name.as_str(), "apply_patch" | "patch") {
        return Vec::new();
    }
    let Some(patch) = event_string(
        event,
        &[
            &["tool_input", "command"],
            &["tool_input", "patch"],
            &["input", "command"],
            &["input", "patch"],
            &["tool", "input", "command"],
            &["tool", "input", "patch"],
        ],
    ) else {
        return Vec::new();
    };
    let lines = patch.lines().collect::<Vec<_>>();
    if lines.first().map(|line| line.trim_end()) != Some("*** Begin Patch")
        || lines.last().map(|line| line.trim_end()) != Some("*** End Patch")
    {
        return Vec::new();
    }
    let mut paths = BTreeSet::new();
    for line in &lines[1..lines.len().saturating_sub(1)] {
        for prefix in [
            "*** Update File: ",
            "*** Add File: ",
            "*** Delete File: ",
            "*** Move to: ",
        ] {
            if let Some(path) = line.strip_prefix(prefix) {
                let path = path.trim();
                if !path.is_empty() {
                    paths.insert(path.to_owned());
                }
            }
        }
    }
    paths.into_iter().collect()
}

fn has_path_field_recursive(value: &Value, changed_only: bool) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            let path_key = if changed_only {
                matches!(
                    key.as_str(),
                    "changed_paths" | "observed_paths" | "modified_paths"
                )
            } else {
                matches!(
                    key.as_str(),
                    "paths"
                        | "path"
                        | "file_path"
                        | "target_path"
                        | "changed_paths"
                        | "observed_paths"
                        | "modified_paths"
                )
            };
            (path_key && structured_path_value(value))
                || has_path_field_recursive(value, changed_only)
        }),
        Value::Array(values) => values
            .iter()
            .any(|value| has_path_field_recursive(value, changed_only)),
        _ => false,
    }
}

fn structured_path_value(value: &Value) -> bool {
    value.as_array().is_some_and(|items| {
        items
            .iter()
            .all(|item| item.as_str().is_some_and(|path| !path.trim().is_empty()))
    })
}

pub(super) fn assess_reported_path(repo_root: &Path, raw: &str) -> PathAssessment {
    assess_path(repo_root, raw)
}

fn collect_paths_recursive(value: &Value, changed_only: bool, paths: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                let path_key = if changed_only {
                    matches!(
                        key.as_str(),
                        "changed_paths" | "observed_paths" | "modified_paths"
                    )
                } else {
                    matches!(
                        key.as_str(),
                        "paths"
                            | "path"
                            | "file_path"
                            | "target_path"
                            | "changed_paths"
                            | "observed_paths"
                            | "modified_paths"
                    )
                };
                if path_key {
                    collect_string_values(value, paths);
                }
                collect_paths_recursive(value, changed_only, paths);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_paths_recursive(value, changed_only, paths);
            }
        }
        _ => {}
    }
}

fn collect_string_values(value: &Value, values: &mut BTreeSet<String>) {
    match value {
        Value::String(text) if !text.trim().is_empty() => {
            values.insert(text.to_owned());
        }
        Value::Array(items) => {
            for item in items {
                collect_string_values(item, values);
            }
        }
        _ => {}
    }
}

fn paths_from_redirection(command: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let words = command.split_whitespace().collect::<Vec<_>>();
    for (index, word) in words.iter().enumerate() {
        if matches!(*word, ">" | ">>") {
            if let Some(path) = words.get(index + 1) {
                paths.push(path.trim_matches('"').trim_matches('\'').to_owned());
            }
        }
    }
    paths
}

fn assess_path(repo_root: &Path, raw: &str) -> PathAssessment {
    let path = Path::new(raw);
    let (inside_repo, normalized) = if path.is_absolute() {
        match path.strip_prefix(repo_root) {
            Ok(relative) => normalized_relative_path(relative)
                .map(|path| (true, Some(path)))
                .unwrap_or((false, None)),
            Err(_) => (false, None),
        }
    } else {
        normalized_relative_path(path)
            .map(|path| (true, Some(path)))
            .unwrap_or((false, None))
    };
    PathAssessment {
        raw: raw.to_owned(),
        normalized,
        inside_repo,
    }
}

fn normalized_relative_path(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_safe_subcommands_are_read_only() {
        for command in [
            "git status --short",
            "git diff --stat",
            "git log -1",
            "git show HEAD",
            "git rev-parse HEAD",
            "cargo metadata --no-deps",
        ] {
            assert_eq!(
                classify_tool(Some("bash"), Some(command)),
                ToolClassification::ReadOnly
            );
        }
        for command in [
            "git rm file.txt",
            "git mv old new",
            "git apply patch.diff",
            "git merge topic",
            "git rebase main",
            "git cherry-pick deadbeef",
            "git restore file.txt",
        ] {
            assert_eq!(
                classify_tool(Some("bash"), Some(command)),
                ToolClassification::Mutating
            );
        }
        for command in [
            "npm run build",
            "pnpm run build",
            "yarn build",
            "node script.js",
            "find . -fls inventory.txt",
        ] {
            assert_eq!(
                classify_tool(Some("bash"), Some(command)),
                ToolClassification::UnknownMutationRisk
            );
        }
    }

    #[test]
    fn apply_patch_headers_are_pre_tool_targets_but_not_actual_change_evidence() {
        let event = serde_json::json!({
            "tool_name": "apply_patch",
            "tool_input": {
                "command": "*** Begin Patch\n*** Update File: src/lib.rs\n*** Move to: src/new.rs\n@@\n-old\n+new\n*** End Patch"
            }
        });
        let paths = collect_structured_path_assessments(&event, Path::new("/repo"), false);
        assert_eq!(
            paths
                .iter()
                .filter_map(|path| path.normalized.as_deref())
                .collect::<Vec<_>>(),
            ["src/lib.rs", "src/new.rs"]
        );
        assert!(collect_path_assessments(&event, Path::new("/repo"), true).is_empty());
        assert!(collect_structured_path_assessments(&event, Path::new("/repo"), true).is_empty());
        assert!(!has_structured_changed_paths(&event));

        let bash = serde_json::json!({
            "tool_name": "bash",
            "tool_input": {"command": "*** Begin Patch\n*** Update File: src/lib.rs\n*** End Patch"}
        });
        assert!(collect_structured_path_assessments(&bash, Path::new("/repo"), false).is_empty());
        assert!(!has_structured_changed_paths(&bash));

        let incomplete = serde_json::json!({
            "tool_name": "apply_patch",
            "tool_input": {"command": "*** Begin Patch\n*** Update File: src/lib.rs"}
        });
        assert!(
            collect_structured_path_assessments(&incomplete, Path::new("/repo"), false).is_empty()
        );
    }

    #[test]
    fn shell_syntax_is_never_claimed_read_only() {
        for command in [
            "echo x>file",
            "rg needle | tee result.txt",
            "$(git status)",
            "git status && touch marker",
            "git status; touch marker",
            "git 'status'",
        ] {
            assert_eq!(
                classify_tool(Some("bash"), Some(command)),
                ToolClassification::UnknownMutationRisk
            );
        }
    }

    #[test]
    fn only_a_well_formed_changed_path_array_is_structured_evidence() {
        assert!(has_structured_changed_paths(&serde_json::json!({
            "changed_paths": []
        })));
        assert!(has_structured_changed_paths(&serde_json::json!({
            "changed_paths": ["src/lib.rs"]
        })));
        for malformed in [
            serde_json::json!({"changed_paths": null}),
            serde_json::json!({"changed_paths": "src/lib.rs"}),
            serde_json::json!({"changed_paths": [""]}),
            serde_json::json!({"changed_paths": [1]}),
        ] {
            assert!(!has_structured_changed_paths(&malformed));
        }
    }
}
