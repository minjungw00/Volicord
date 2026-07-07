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
    if shell_command_is_clearly_mutating(command) {
        return ToolClassification::Mutating;
    }
    if shell_command_is_read_only(command) {
        return ToolClassification::ReadOnly;
    }
    ToolClassification::UnknownMutationRisk
}

fn shell_command_is_clearly_mutating(command: &str) -> bool {
    let compact = format!(" {command} ");
    if compact.contains(" > ") || compact.contains(" >> ") || compact.contains(" tee ") {
        return true;
    }
    if compact.contains(" sed -i ")
        || compact.contains(" perl -pi ")
        || compact.contains(" git add ")
        || compact.contains(" git commit ")
        || compact.contains(" git reset ")
        || compact.contains(" git clean ")
        || compact.contains(" git checkout ")
        || compact.contains(" git switch ")
    {
        return true;
    }
    command_segments(command).iter().any(|segment| {
        let first = first_command_word(segment);
        matches!(
            first.as_deref(),
            Some(
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
            )
        ) || segment.trim_start().starts_with("cargo fmt")
            || segment.trim_start().starts_with("npm install")
            || segment.trim_start().starts_with("pnpm install")
            || segment.trim_start().starts_with("yarn install")
    })
}

fn shell_command_is_read_only(command: &str) -> bool {
    command_segments(command).iter().all(|segment| {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            return true;
        }
        if trimmed.contains(" -delete") {
            return false;
        }
        let first = first_command_word(trimmed);
        matches!(
            first.as_deref(),
            Some(
                "pwd"
                    | "ls"
                    | "cat"
                    | "rg"
                    | "grep"
                    | "find"
                    | "wc"
                    | "head"
                    | "tail"
                    | "sed"
                    | "awk"
                    | "git"
                    | "cargo"
                    | "npm"
                    | "pnpm"
                    | "yarn"
                    | "node"
                    | "rustc"
            )
        ) && !trimmed.starts_with("cargo fmt")
            && !trimmed.starts_with("npm install")
            && !trimmed.starts_with("pnpm install")
            && !trimmed.starts_with("yarn install")
            && !trimmed.starts_with("git add")
            && !trimmed.starts_with("git commit")
            && !trimmed.starts_with("git reset")
            && !trimmed.starts_with("git clean")
            && !trimmed.starts_with("git checkout")
            && !trimmed.starts_with("git switch")
    })
}

fn command_segments(command: &str) -> Vec<&str> {
    command
        .split([';', '\n'])
        .flat_map(|part| part.split("&&"))
        .flat_map(|part| part.split("||"))
        .collect()
}

fn first_command_word(segment: &str) -> Option<String> {
    let mut words = segment.split_whitespace();
    let first = words.next()?;
    if first == "sudo" || first == "command" {
        words.next().map(str::to_owned)
    } else {
        Some(first.to_owned())
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
