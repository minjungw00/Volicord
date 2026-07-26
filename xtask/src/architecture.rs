use crate::diagnostics::ValidationIssue;
use crate::repository::{normalize_existing_root, repo_relative};
use crate::workspace_manifests::{dependency_names, read_toml_document};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const MAINTAINABILITY_TOP_N: usize = 10;
const MAINTAINABILITY_SKIP_DIRS: &[&str] = &[".git", "target"];
const API_METHODS_PATH: &str = "docs/en/reference/api/methods.md";
const ADMIN_CLI_REFERENCE_PATH: &str = "docs/en/reference/admin-cli.md";
const CORE_METHOD_TEST_HINT_DIR: &str = "crates/volicord-core/src/methods/tests";
const CLI_BINARY_TEST_HINT_DIR: &str = "crates/volicord-cli/tests";
const CLI_MAIN_PATH: &str = "crates/volicord-cli/src/main.rs";
const XTASK_FORBIDDEN_RUNTIME_DEPENDENCIES: &[&str] = &[
    "volicord-cli",
    "volicord-core",
    "volicord-mcp",
    "volicord-platform-fs",
    "volicord-platform-process",
    "volicord-store",
];

pub(crate) fn validate_xtask_dependency_boundary(
    manifest_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    let manifest = match read_toml_document(manifest_path, "xtask Cargo.toml") {
        Ok(manifest) => manifest,
        Err(error) => {
            issues.push(ValidationIssue::new(
                "xtask/Cargo.toml",
                "architecture_dependency.manifest",
                format!("failed to inspect xtask dependencies: {error:#}"),
            ));
            return;
        }
    };
    for dependency in dependency_names(&manifest) {
        if XTASK_FORBIDDEN_RUNTIME_DEPENDENCIES.contains(&dependency.as_str()) {
            issues.push(ValidationIssue::new(
                "xtask/Cargo.toml",
                "architecture_dependency.runtime_boundary",
                format!(
                    "xtask must not depend on runtime crate {dependency}; use a lightweight contract or command-model crate"
                ),
            ));
        }
    }
}
#[derive(Debug, Clone)]
pub struct MaintainabilityReport {
    largest_rust_files: Vec<FileMetric>,
    largest_test_files: Vec<FileMetric>,
    largest_markdown_files: Vec<FileMetric>,
    mixed_signal_files: Vec<MixedSignalFile>,
    method_test_hints: Vec<CoverageHint>,
    command_test_hints: Vec<CoverageHint>,
}

impl MaintainabilityReport {
    pub fn render(&self) -> String {
        let mut output = String::new();
        output.push_str("Maintainability report\n");
        output.push_str(
            "Informational only: this report does not enforce LOC limits, define LOC exception allowlists, mark long cohesive files invalid, or require splitting files by line count.\n",
        );
        output.push_str(
            "The command exits with failure only when it cannot inspect the repository.\n\n",
        );

        render_file_metrics(
            &mut output,
            "Largest Rust files by physical lines",
            &self.largest_rust_files,
        );
        render_file_metrics(
            &mut output,
            "Largest Rust test files by physical lines",
            &self.largest_test_files,
        );
        render_file_metrics(
            &mut output,
            "Largest Markdown files by physical lines",
            &self.largest_markdown_files,
        );
        render_mixed_signals(&mut output, &self.mixed_signal_files);
        render_coverage_hints(
            &mut output,
            "Public API method test hints",
            "Every public method has an obvious nearby test signal.",
            &self.method_test_hints,
        );
        render_coverage_hints(
            &mut output,
            "User-facing CLI command test hints",
            "Every documented command has an obvious binary or render test signal.",
            &self.command_test_hints,
        );

        output
    }

    pub fn largest_rust_files(&self) -> &[FileMetric] {
        &self.largest_rust_files
    }

    pub fn largest_test_files(&self) -> &[FileMetric] {
        &self.largest_test_files
    }

    pub fn largest_markdown_files(&self) -> &[FileMetric] {
        &self.largest_markdown_files
    }

    pub fn mixed_signal_files(&self) -> &[MixedSignalFile] {
        &self.mixed_signal_files
    }

    pub fn method_test_hints(&self) -> &[CoverageHint] {
        &self.method_test_hints
    }

    pub fn command_test_hints(&self) -> &[CoverageHint] {
        &self.command_test_hints
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileMetric {
    path: String,
    lines: usize,
}

impl FileMetric {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn lines(&self) -> usize {
        self.lines
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MixedSignalFile {
    path: String,
    lines: usize,
    signals: Vec<&'static str>,
}

impl MixedSignalFile {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn lines(&self) -> usize {
        self.lines
    }

    pub fn signals(&self) -> &[&'static str] {
        &self.signals
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CoverageHint {
    item: String,
    message: String,
}

impl CoverageHint {
    pub fn item(&self) -> &str {
        &self.item
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

pub fn run_maintainability_report(root: &Path) -> Result<MaintainabilityReport> {
    let root = normalize_existing_root(root)?;
    if !root.join("Cargo.toml").exists() {
        anyhow::bail!(
            "maintainability-report must run from the repository root; missing Cargo.toml"
        );
    }

    let mut paths = Vec::new();
    collect_repository_files(&root, &root, &mut paths)?;

    let mut largest_rust_files = Vec::new();
    let mut largest_test_files = Vec::new();
    let mut largest_markdown_files = Vec::new();
    let mut mixed_signal_files = Vec::new();

    for path in paths {
        let relative = repo_relative(&root, &path);
        if relative.ends_with(".rs") {
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("failed to read Rust source {relative}"))?;
            let metric = FileMetric {
                path: relative.clone(),
                lines: physical_line_count(&contents),
            };
            if is_test_rust_file(&relative, &contents) {
                largest_test_files.push(metric.clone());
            }
            if let Some(signals) = mixed_command_signals(&contents) {
                mixed_signal_files.push(MixedSignalFile {
                    path: relative,
                    lines: metric.lines,
                    signals,
                });
            }
            largest_rust_files.push(metric);
        } else if relative.ends_with(".md") {
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("failed to read Markdown source {relative}"))?;
            largest_markdown_files.push(FileMetric {
                path: relative,
                lines: physical_line_count(&contents),
            });
        }
    }

    sort_largest_metrics(&mut largest_rust_files);
    sort_largest_metrics(&mut largest_test_files);
    sort_largest_metrics(&mut largest_markdown_files);
    sort_mixed_signals(&mut mixed_signal_files);

    Ok(MaintainabilityReport {
        largest_rust_files,
        largest_test_files,
        largest_markdown_files,
        mixed_signal_files,
        method_test_hints: public_method_test_hints(&root)?,
        command_test_hints: cli_command_test_hints(&root)?,
    })
}

fn collect_repository_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        entries
            .push(entry.with_context(|| format!("failed to read entry under {}", dir.display()))?);
    }
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", entry.path().display()))?;
        if file_type.is_dir() {
            let name = entry.file_name();
            if should_skip_maintainability_dir(&name.to_string_lossy()) {
                continue;
            }
            collect_repository_files(root, &entry.path(), files)?;
        } else if file_type.is_file() {
            let relative = repo_relative(root, &entry.path());
            if !relative.is_empty() {
                files.push(entry.path());
            }
        }
    }

    Ok(())
}

fn should_skip_maintainability_dir(name: &str) -> bool {
    MAINTAINABILITY_SKIP_DIRS.contains(&name)
}

fn physical_line_count(contents: &str) -> usize {
    contents.lines().count()
}

fn is_test_rust_file(path: &str, _contents: &str) -> bool {
    path.starts_with("tests/") || path.contains("/tests/")
}

fn sort_largest_metrics(metrics: &mut Vec<FileMetric>) {
    metrics.sort_by(|left, right| {
        right
            .lines
            .cmp(&left.lines)
            .then_with(|| left.path.cmp(&right.path))
    });
    metrics.truncate(MAINTAINABILITY_TOP_N);
}

fn sort_mixed_signals(files: &mut Vec<MixedSignalFile>) {
    files.sort_by(|left, right| {
        right
            .signals
            .len()
            .cmp(&left.signals.len())
            .then_with(|| right.lines.cmp(&left.lines))
            .then_with(|| left.path.cmp(&right.path))
    });
    files.truncate(MAINTAINABILITY_TOP_N);
}

fn mixed_command_signals(contents: &str) -> Option<Vec<&'static str>> {
    let lower = contents.to_ascii_lowercase();
    let mut signals = Vec::new();

    if contains_any(
        &lower,
        &[
            "dispatch",
            "parsedcommandargs",
            "positionals",
            "subcommand",
            "usage",
        ],
    ) {
        signals.push("command parsing");
    }
    if contains_any(
        &lower,
        &[
            "command::new",
            "process::exit",
            "spawn",
            ".output(",
            ".status(",
            ".wait(",
        ],
    ) {
        signals.push("execution");
    }
    if contains_any(
        &lower,
        &[
            "format!",
            "println!",
            "eprintln!",
            "stdout",
            "stderr",
            "summary_card",
            "render",
            "usage",
            "write!",
        ],
    ) {
        signals.push("rendering");
    }

    if signals.len() == 3 {
        Some(signals)
    } else {
        None
    }
}

fn contains_any(contents: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| contents.contains(needle))
}

fn public_method_test_hints(root: &Path) -> Result<Vec<CoverageHint>> {
    let methods_path = root.join(API_METHODS_PATH);
    if !methods_path.exists() {
        return Ok(Vec::new());
    }

    let methods = extract_backtick_values_with_prefix(
        &fs::read_to_string(&methods_path)
            .with_context(|| format!("failed to read {API_METHODS_PATH}"))?,
        "volicord.",
    );
    let corpus = read_text_corpus(
        root,
        &[
            CORE_METHOD_TEST_HINT_DIR,
            "tests/conformance",
            "tests/integration",
        ],
        ".rs",
    )?;

    Ok(methods
        .into_iter()
        .filter(|method| !method_has_obvious_test_hint(method, &corpus))
        .map(|method| CoverageHint {
            item: method,
            message: format!(
                "no obvious dedicated test signal found under {CORE_METHOD_TEST_HINT_DIR}, tests/conformance, or tests/integration"
            ),
        })
        .collect())
}

fn cli_command_test_hints(root: &Path) -> Result<Vec<CoverageHint>> {
    let admin_cli_path = root.join(ADMIN_CLI_REFERENCE_PATH);
    if !admin_cli_path.exists() {
        return Ok(Vec::new());
    }

    let commands = extract_admin_cli_commands(
        &fs::read_to_string(&admin_cli_path)
            .with_context(|| format!("failed to read {ADMIN_CLI_REFERENCE_PATH}"))?,
    );
    let corpus = read_text_corpus(root, &[CLI_BINARY_TEST_HINT_DIR, CLI_MAIN_PATH], ".rs")?;

    Ok(commands
        .into_iter()
        .filter(|command| !command_has_obvious_test_hint(command, &corpus))
        .map(|command| CoverageHint {
            item: command,
            message: format!(
                "no obvious binary or render test signal found under {CLI_BINARY_TEST_HINT_DIR} or {CLI_MAIN_PATH}"
            ),
        })
        .collect())
}

fn read_text_corpus(
    root: &Path,
    relative_paths: &[&str],
    required_suffix: &str,
) -> Result<Vec<(String, String)>> {
    let mut corpus = Vec::new();
    for relative_path in relative_paths {
        let path = root.join(relative_path);
        if !path.exists() {
            continue;
        }
        if path.is_file() {
            if relative_path.ends_with(required_suffix) {
                corpus.push((
                    (*relative_path).to_string(),
                    fs::read_to_string(&path)
                        .with_context(|| format!("failed to read {relative_path}"))?,
                ));
            }
            continue;
        }

        let mut files = Vec::new();
        collect_repository_files(root, &path, &mut files)?;
        for file in files {
            let relative = repo_relative(root, &file);
            if relative.ends_with(required_suffix) {
                corpus.push((
                    relative.clone(),
                    fs::read_to_string(&file)
                        .with_context(|| format!("failed to read {relative}"))?,
                ));
            }
        }
    }
    Ok(corpus)
}

fn method_has_obvious_test_hint(method: &str, corpus: &[(String, String)]) -> bool {
    let method_key = method.strip_prefix("volicord.").unwrap_or(method);
    let compact_key = method_key.replace('_', "");
    corpus.iter().any(|(path, contents)| {
        let file_stem = Path::new(path)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        path.contains(method_key)
            || file_stem == method_key
            || file_stem.replace('_', "") == compact_key
            || contents.contains(method)
            || contents.contains(method_key)
    })
}

fn command_has_obvious_test_hint(command: &str, corpus: &[(String, String)]) -> bool {
    let tokens = command.split_whitespace().skip(1).collect::<Vec<_>>();
    corpus.iter().any(|(_, contents)| {
        if contents.contains(command) {
            return true;
        }
        match tokens.as_slice() {
            [single] => contains_quoted_token(contents, single),
            [first, second] => {
                contents.contains(&format!("\"{first}\", \"{second}\""))
                    || contents.contains(&format!("\"{first}\",\n            \"{second}\""))
                    || contains_quoted_token(contents, first)
                        && contents.contains(&format!("{first}{}", "_usage"))
                        && contains_quoted_token(contents, second)
            }
            _ => false,
        }
    })
}

fn contains_quoted_token(contents: &str, token: &str) -> bool {
    contents.contains(&format!("\"{token}\""))
}

fn extract_backtick_values_with_prefix(contents: &str, prefix: &str) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    let mut remaining = contents;
    let needle = format!("`{prefix}");

    while let Some(start) = remaining.find(&needle) {
        let value_start = start + 1;
        let after_start = &remaining[value_start..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        values.insert(after_start[..end].to_string());
        remaining = &after_start[end + 1..];
    }

    values
}

fn extract_admin_cli_commands(contents: &str) -> BTreeSet<String> {
    let mut commands = BTreeSet::new();
    let mut in_text_fence = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_text_fence {
                in_text_fence = false;
            } else {
                in_text_fence = trimmed.trim_start_matches('`').trim() == "text";
            }
            continue;
        }

        if in_text_fence && trimmed.starts_with("volicord ") {
            if let Some(command) = admin_cli_command_key(trimmed) {
                commands.insert(command);
            }
        }
    }

    commands
}

fn admin_cli_command_key(line: &str) -> Option<String> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    let first = *tokens.first()?;
    if first != "volicord" {
        return None;
    }
    let second = normalize_command_token(tokens.get(1)?)?;
    if second.starts_with("--") {
        return Some(format!("volicord {second}"));
    }
    let grouped = matches!(second, "changes" | "connection" | "inbox" | "project");
    if grouped {
        if let Some(third) = tokens
            .get(2)
            .and_then(|token| normalize_command_token(token))
        {
            if !third.starts_with('-') && !third.starts_with('<') && !third.contains('[') {
                return Some(format!("volicord {second} {third}"));
            }
        }
    }
    Some(format!("volicord {second}"))
}

fn normalize_command_token(token: &str) -> Option<&str> {
    let token = token.trim_matches(|character: char| {
        matches!(character, '[' | ']' | ',' | '|' | '(' | ')' | '`')
    });
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn render_file_metrics(output: &mut String, heading: &str, metrics: &[FileMetric]) {
    output.push_str(heading);
    output.push('\n');
    if metrics.is_empty() {
        output.push_str("  none found\n\n");
        return;
    }
    for (index, metric) in metrics.iter().enumerate() {
        output.push_str(&format!(
            "  {:>2}. {:>5} lines  {}\n",
            index + 1,
            metric.lines,
            metric.path
        ));
    }
    output.push('\n');
}

fn render_mixed_signals(output: &mut String, files: &[MixedSignalFile]) {
    output.push_str("Mixed command parsing/execution/rendering signals (heuristic)\n");
    if files.is_empty() {
        output.push_str("  none found\n\n");
        return;
    }
    for (index, file) in files.iter().enumerate() {
        output.push_str(&format!(
            "  {:>2}. {:>5} lines  {}  [{}]\n",
            index + 1,
            file.lines,
            file.path,
            file.signals.join(", ")
        ));
    }
    output.push('\n');
}

fn render_coverage_hints(
    output: &mut String,
    heading: &str,
    empty_message: &str,
    hints: &[CoverageHint],
) {
    output.push_str(heading);
    output.push('\n');
    if hints.is_empty() {
        output.push_str(&format!("  {empty_message}\n\n"));
        return;
    }
    for hint in hints {
        output.push_str(&format!("  - {}: {}\n", hint.item, hint.message));
    }
    output.push('\n');
}
