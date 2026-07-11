use anyhow::{Context, Result};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Component, Path, PathBuf};
use toml_edit::{DocumentMut, Item};

const DOC_INDEX_PATH: &str = "docs/doc-index.yaml";
const TERMINOLOGY_MAP_PATH: &str = "docs/terminology-map.yaml";
const OPERATION_CATEGORY_DOC_PATHS: &[&str] = &[
    "docs/en/reference/api/schema-value-sets.md",
    "docs/ko/reference/api/schema-value-sets.md",
];
const OPERATION_CATEGORY_ANCHOR: &str = "operation-category-values";
const OPERATION_CATEGORY_TERM_KEY: &str = "operation_category";
const STORAGE_REGISTRY_SQL_PATH: &str = "crates/volicord-store/src/schema/registry.sql";
const STORAGE_PROJECT_SQL_PATH: &str = "crates/volicord-store/src/schema/project.sql";
const STORAGE_DDL_DOC_PATHS: &[&str] = &[
    "docs/en/reference/storage-ddl.md",
    "docs/ko/reference/storage-ddl.md",
];
const SURFACE_STABILITY_LABELS: &[&str] = &["stable", "beta", "internal", "diagnostic"];
const REQUIRED_SURFACE_STABILITY_DOCS: &[SurfaceStabilityRequirement] = &[
    SurfaceStabilityRequirement {
        path: "docs/en/reference/admin-cli.md",
        required_labels: &["stable", "beta", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/ko/reference/admin-cli.md",
        required_labels: &["stable", "beta", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/en/reference/api/methods.md",
        required_labels: &["stable"],
    },
    SurfaceStabilityRequirement {
        path: "docs/ko/reference/api/methods.md",
        required_labels: &["stable"],
    },
    SurfaceStabilityRequirement {
        path: "docs/en/reference/mcp-transport.md",
        required_labels: &["stable", "beta", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/ko/reference/mcp-transport.md",
        required_labels: &["stable", "beta", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/en/reference/conformance.md",
        required_labels: &["stable", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/ko/reference/conformance.md",
        required_labels: &["stable", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/en/reference/storage-ddl.md",
        required_labels: &["stable", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/ko/reference/storage-ddl.md",
        required_labels: &["stable", "internal", "diagnostic"],
    },
];

const TOP_LEVEL_REQUIRED: &[&str] = &[
    "version",
    "metadata",
    "language_retrieval",
    "owner_areas",
    "applicability",
    "entry_schema",
    "shared_documents",
    "documents",
];
const TOP_LEVEL_ALLOWED: &[&str] = TOP_LEVEL_REQUIRED;
const CATALOG_ENTRY_ALLOWED: &[&str] = &["description"];
const SHARED_REQUIRED: &[&str] = &[
    "doc_id",
    "path",
    "kind",
    "summary",
    "normative_level",
    "owner_area",
    "created_on",
    "last_updated_on",
    "last_verified_on",
    "applies_to",
];
const PAIRED_REQUIRED: &[&str] = &[
    "doc_id",
    "path_en",
    "path_ko",
    "kind",
    "summary",
    "normative_level",
    "translation_policy",
    "owner_area",
    "created_on",
    "last_updated_on",
    "last_verified_on",
    "applies_to",
];
const OPTIONAL_FIELDS: &[&str] = &[
    "primary_audience",
    "journeys",
    "canonical_for",
    "depends_on",
];
const SHARED_ALLOWED: &[&str] = &[
    "doc_id",
    "path",
    "kind",
    "summary",
    "normative_level",
    "owner_area",
    "created_on",
    "last_updated_on",
    "last_verified_on",
    "applies_to",
    "primary_audience",
    "journeys",
    "canonical_for",
    "depends_on",
];
const PAIRED_ALLOWED: &[&str] = &[
    "doc_id",
    "path_en",
    "path_ko",
    "kind",
    "summary",
    "normative_level",
    "translation_policy",
    "owner_area",
    "created_on",
    "last_updated_on",
    "last_verified_on",
    "applies_to",
    "primary_audience",
    "journeys",
    "canonical_for",
    "depends_on",
];
const KINDS: &[&str] = &[
    "landing",
    "tutorial",
    "how_to",
    "explanation",
    "reference",
    "maintenance",
];
const READER_JOURNEYS: &[&str] = &[
    "evaluate",
    "install",
    "operate",
    "learn",
    "implement",
    "maintain",
];
const NORMATIVE_LEVELS: &[&str] = &["contract", "guide", "example", "maintenance"];
const TRANSLATION_POLICIES: &[&str] = &["semantic_parity"];
const ROOT_README_EN_PATH: &str = "README.md";
const ROOT_README_KO_PATH: &str = "README.ko.md";
const REQUIRED_SHARED_PATHS: &[&str] = &[
    "AGENTS.md",
    "docs/AGENTS.md",
    "crates/AGENTS.md",
    ROOT_README_EN_PATH,
    "docs/README.md",
    "docs/doc-index.yaml",
    "docs/terminology-map.yaml",
];
const PUBLIC_LANGUAGE_SOURCE_ROOTS: &[&str] = &[
    "crates/volicord-cli/src/connection_command.rs",
    "crates/volicord-cli/src/connection_command",
    "crates/volicord-cli/src/doctor_command.rs",
    "crates/volicord-cli/src/guard_command.rs",
    "crates/volicord-cli/src/guard_command",
    "crates/volicord-cli/src/guard_integration",
    "crates/volicord-cli/src/setup_command.rs",
    "crates/volicord-cli/src/setup_command",
    "crates/volicord-cli/src/user_command.rs",
    "crates/volicord-mcp/src",
];
const MAINTAINABILITY_TOP_N: usize = 10;
const MAINTAINABILITY_SKIP_DIRS: &[&str] = &[".git", "target"];
const API_METHODS_PATH: &str = "docs/en/reference/api/methods.md";
const ADMIN_CLI_REFERENCE_PATH: &str = "docs/en/reference/admin-cli.md";
const CORE_METHOD_TEST_HINT_DIR: &str = "crates/volicord-core/src/methods/tests";
const CLI_BINARY_TEST_HINT_DIR: &str = "crates/volicord-cli/tests";
const CLI_MAIN_PATH: &str = "crates/volicord-cli/src/main.rs";
const PUBLIC_UNQUALIFIED_SECURITY_WORDS: &[&str] = &["safe", "secure", "protected"];
const PUBLIC_DOCUMENT_DISALLOWED_TERMS: &[PublicDocumentDisallowedTerm] = &[
    PublicDocumentDisallowedTerm {
        term: "write-readiness",
        replacement: "write-ticket",
    },
    PublicDocumentDisallowedTerm {
        term: "write readiness",
        replacement: "write ticket",
    },
];
const TERMINOLOGY_ALLOWED_ROLES: &[&str] = &[
    "public_user_term",
    "storage_internal_identifier",
    "storage_record",
    "mcp_process_binding",
    "diagnostic_field",
    "mcp_public_selector",
];
const REQUIRED_TERMINOLOGY_ROLES: &[RequiredTerminologyRoles] = &[
    RequiredTerminologyRoles {
        term_key: "connection_internal_id",
        display: "connection_internal_id",
        roles: &["storage_internal_identifier"],
    },
    RequiredTerminologyRoles {
        term_key: "project_internal_id",
        display: "project_internal_id",
        roles: &["storage_internal_identifier"],
    },
    RequiredTerminologyRoles {
        term_key: "connection_id",
        display: "connection_id",
        roles: &["mcp_process_binding", "diagnostic_field"],
    },
    RequiredTerminologyRoles {
        term_key: "project_id",
        display: "project_id",
        roles: &["diagnostic_field"],
    },
    RequiredTerminologyRoles {
        term_key: "project_selector",
        display: "project_selector",
        roles: &["mcp_public_selector"],
    },
    RequiredTerminologyRoles {
        term_key: "installation_profile",
        display: "installation_profile",
        roles: &["storage_record"],
    },
    RequiredTerminologyRoles {
        term_key: "volicord_runtime_home",
        display: "Volicord Runtime Home",
        roles: &["public_user_term"],
    },
];

struct RequiredTerminologyRoles {
    term_key: &'static str,
    display: &'static str,
    roles: &'static [&'static str],
}

struct PublicDocumentDisallowedTerm {
    term: &'static str,
    replacement: &'static str,
}

struct SurfaceStabilityRequirement {
    path: &'static str,
    required_labels: &'static [&'static str],
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ValidationError {
    file: String,
    category: &'static str,
    message: String,
}

impl ValidationError {
    fn new(file: impl Into<String>, category: &'static str, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            category,
            message: message.into(),
        }
    }

    pub fn file(&self) -> &str {
        &self.file
    }

    pub fn category(&self) -> &'static str {
        self.category
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Ord for ValidationError {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.file, self.category, &self.message).cmp(&(
            &other.file,
            other.category,
            &other.message,
        ))
    }
}

impl PartialOrd for ValidationError {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}: {}", self.file, self.category, self.message)
    }
}

#[derive(Debug, Clone)]
pub struct CheckReport {
    errors: Vec<ValidationError>,
}

impl CheckReport {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReleaseVersionReport {
    workspace_version: String,
    member_package_count: usize,
    checked_tag: Option<String>,
}

impl ReleaseVersionReport {
    pub fn workspace_version(&self) -> &str {
        &self.workspace_version
    }

    pub fn member_package_count(&self) -> usize {
        self.member_package_count
    }

    pub fn checked_tag(&self) -> Option<&str> {
        self.checked_tag.as_deref()
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

#[derive(Debug, Clone)]
struct DocIndex {
    indexed_paths: BTreeSet<String>,
    paired_paths: BTreeMap<String, (String, String)>,
    path_doc_ids: BTreeMap<String, String>,
    paired_documents: BTreeMap<String, PairedDocument>,
}

#[derive(Debug, Clone)]
struct PairedDocument {
    doc_id: String,
    path_en: String,
    path_ko: String,
}

#[derive(Debug, Clone)]
struct DocEntry {
    doc_id: String,
    depends_on: Vec<String>,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct IsoDate {
    year: u16,
    month: u8,
    day: u8,
}

#[derive(Debug, Clone)]
struct DateError {
    category: &'static str,
    message: String,
}

#[derive(Debug, Clone)]
struct LinkFailure {
    category: &'static str,
    message: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct SemanticLinkKey {
    target: SemanticLinkTarget,
    fragment: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum SemanticLinkTarget {
    DocId(String),
    RepositoryPath(String),
}

#[derive(Debug, Clone)]
struct MarkdownAnchors {
    anchors: BTreeSet<String>,
}

#[derive(Default)]
struct AnchorCache {
    files: BTreeMap<String, MarkdownAnchors>,
}

#[derive(Debug, Clone)]
struct VolicordCommandExample {
    line: usize,
    command: String,
    tokens: std::result::Result<Vec<String>, String>,
}

#[derive(Debug, Clone)]
struct PendingShellCommand {
    line: usize,
    text: String,
}

#[derive(Debug, Clone)]
struct ActiveFence {
    marker: char,
    length: usize,
    shell: bool,
}

#[derive(Debug, Clone, Default)]
struct ParsedCommandArgs {
    options: BTreeSet<String>,
    value_options: BTreeMap<String, Vec<String>>,
    positionals: Vec<String>,
}

pub fn run_release_version_check(
    root: &Path,
    release_tag: Option<&str>,
) -> Result<ReleaseVersionReport> {
    let root = normalize_existing_root(root)?;
    let root_manifest_path = root.join("Cargo.toml");
    let root_manifest = read_toml_document(&root_manifest_path, "root Cargo.toml")?;
    let Some(workspace_version) = root_manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(Item::as_str)
    else {
        anyhow::bail!(
            "release-version-check requires [workspace.package].version in the root Cargo.toml"
        );
    };
    let workspace_version = workspace_version.to_owned();
    let Some(members) = root_manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(Item::as_array)
    else {
        anyhow::bail!(
            "release-version-check requires an explicit [workspace].members array in the root Cargo.toml"
        );
    };

    let mut member_package_count = 0usize;
    for member in members.iter() {
        let Some(member) = member.as_str() else {
            anyhow::bail!("workspace member entries must be strings");
        };
        let manifest_path = root.join(member).join("Cargo.toml");
        let relative_manifest = repo_relative(&root, &manifest_path);
        let manifest = read_toml_document(&manifest_path, &relative_manifest)?;
        let package_name = manifest
            .get("package")
            .and_then(|package| package.get("name"))
            .and_then(Item::as_str)
            .unwrap_or(member);
        let inherits_workspace_version = manifest
            .get("package")
            .and_then(|package| package.get("version"))
            .and_then(|version| version.get("workspace"))
            .and_then(Item::as_bool);
        if inherits_workspace_version != Some(true) {
            anyhow::bail!(
                "workspace package {package_name} in {relative_manifest} must set version.workspace = true"
            );
        }
        member_package_count += 1;
    }
    if member_package_count == 0 {
        anyhow::bail!("release-version-check requires at least one workspace member package");
    }

    if let Some(release_tag) = release_tag {
        let expected_tag = format!("v{workspace_version}");
        if release_tag != expected_tag {
            anyhow::bail!(
                "release tag {release_tag:?} does not match workspace package version; expected {expected_tag:?}"
            );
        }
    }

    Ok(ReleaseVersionReport {
        workspace_version,
        member_package_count,
        checked_tag: release_tag.map(str::to_owned),
    })
}

fn read_toml_document(path: &Path, label: &str) -> Result<DocumentMut> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read {label} at {}", path.display()))?;
    contents
        .parse::<DocumentMut>()
        .with_context(|| format!("failed to parse {label} at {}", path.display()))
}

pub fn run_docs_check(root: &Path) -> Result<CheckReport> {
    let root = normalize_existing_root(root)?;
    let doc_index_path = root.join(DOC_INDEX_PATH);
    if !doc_index_path.exists() {
        anyhow::bail!(
            "docs-check must run from the repository root; missing {}",
            DOC_INDEX_PATH
        );
    }

    let mut errors = Vec::new();
    let index = validate_doc_index(&root, &mut errors);

    if let Some(index) = index.as_ref() {
        validate_document_coverage(&root, index, &mut errors);
        validate_markdown_links(&root, index, &mut errors);
        validate_bilingual_link_parity(&root, index, &mut errors);
        validate_terminology_paths(&root, index, &mut errors);
        validate_volicord_command_examples(&root, index, &mut errors);
        validate_public_document_language(&root, index, &mut errors);
    }
    validate_surface_stability_sections(&root, &mut errors);
    validate_public_language_claims(&root, &mut errors);
    validate_storage_ddl_sql_blocks(&root, &mut errors);
    validate_operation_category_values(&root, &mut errors);

    errors.sort();
    errors.dedup();

    Ok(CheckReport { errors })
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

fn validate_storage_ddl_sql_blocks(root: &Path, errors: &mut Vec<ValidationError>) {
    let schema_sources = [
        ("registry", STORAGE_REGISTRY_SQL_PATH),
        ("project", STORAGE_PROJECT_SQL_PATH),
    ];
    if !schema_sources
        .iter()
        .any(|(_, relative_path)| root.join(relative_path).exists())
        && !STORAGE_DDL_DOC_PATHS
            .iter()
            .any(|relative_path| root.join(relative_path).exists())
    {
        return;
    }

    for (label, schema_path) in schema_sources {
        let expected = match fs::read_to_string(root.join(schema_path)) {
            Ok(contents) => normalize_canonical_sql_block(&contents),
            Err(error) => {
                errors.push(ValidationError::new(
                    schema_path,
                    "storage_ddl_sql.read",
                    format!("failed to read canonical storage SQL: {error}"),
                ));
                continue;
            }
        };

        for doc_path in STORAGE_DDL_DOC_PATHS {
            let contents = match fs::read_to_string(root.join(doc_path)) {
                Ok(contents) => contents,
                Err(error) => {
                    errors.push(ValidationError::new(
                        *doc_path,
                        "storage_ddl_sql.read",
                        format!("failed to read Storage DDL document: {error}"),
                    ));
                    continue;
                }
            };
            match extract_canonical_storage_sql_block(&contents, label) {
                Some(actual) if normalize_canonical_sql_block(&actual) == expected => {}
                Some(_) => errors.push(ValidationError::new(
                    *doc_path,
                    "storage_ddl_sql.drift",
                    format!("canonical {label} SQL block differs from {schema_path}"),
                )),
                None => errors.push(ValidationError::new(
                    *doc_path,
                    "storage_ddl_sql.missing",
                    format!("missing canonical {label} SQL block"),
                )),
            }
        }
    }
}

fn validate_surface_stability_sections(root: &Path, errors: &mut Vec<ValidationError>) {
    for requirement in REQUIRED_SURFACE_STABILITY_DOCS {
        let path = root.join(requirement.path);
        if !path.exists() {
            continue;
        }
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationError::new(
                    requirement.path,
                    "surface_stability.read",
                    format!("failed to read required surface stability document: {error}"),
                ));
                continue;
            }
        };

        let Some(section) = extract_surface_stability_section(&contents) else {
            errors.push(ValidationError::new(
                requirement.path,
                "surface_stability.missing_section",
                "missing required <a id=\"surface-stability\"></a> Surface Stability section",
            ));
            continue;
        };

        if !section.contains("documentation-policy.md#surface-stability-labels") {
            errors.push(ValidationError::new(
                requirement.path,
                "surface_stability.missing_link",
                "Surface Stability section must link to the canonical documentation policy vocabulary",
            ));
        }

        let labels = extract_surface_stability_labels(section);
        for label in &labels {
            if !SURFACE_STABILITY_LABELS.contains(&label.as_str()) {
                errors.push(ValidationError::new(
                    requirement.path,
                    "surface_stability.invalid_label",
                    format!("Surface Stability section uses unsupported label `{label}`"),
                ));
            }
        }
        for required_label in requirement.required_labels {
            if !labels.contains(*required_label) {
                errors.push(ValidationError::new(
                    requirement.path,
                    "surface_stability.missing_label",
                    format!(
                        "Surface Stability section is missing required `{required_label}` label"
                    ),
                ));
            }
        }
    }
}

fn extract_surface_stability_section(contents: &str) -> Option<&str> {
    let marker = "<a id=\"surface-stability\"></a>";
    let start = contents.find(marker)?;
    let after_marker = &contents[start..];
    let mut offset = 0;
    let mut heading_count = 0;

    for line in after_marker.split_inclusive('\n') {
        if line.trim_start().starts_with("## ") {
            heading_count += 1;
            if heading_count == 2 {
                return Some(&after_marker[..offset]);
            }
        }
        offset += line.len();
    }

    Some(after_marker)
}

fn extract_surface_stability_labels(section: &str) -> BTreeSet<String> {
    let mut labels = BTreeSet::new();
    for line in section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            continue;
        }
        let cells = markdown_table_cells(trimmed);
        if cells.len() < 2 || is_markdown_table_separator(&cells) {
            continue;
        }
        for label in extract_backtick_values(cells[1]) {
            labels.insert(label);
        }
    }
    labels
}

fn markdown_table_cells(line: &str) -> Vec<&str> {
    line.trim_matches('|').split('|').map(str::trim).collect()
}

fn is_markdown_table_separator(cells: &[&str]) -> bool {
    cells.iter().all(|cell| {
        cell.chars()
            .all(|character| matches!(character, '-' | ':' | ' '))
    })
}

fn extract_backtick_values(contents: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remaining = contents;

    while let Some(start) = remaining.find('`') {
        let after_start = &remaining[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        values.push(after_start[..end].to_string());
        remaining = &after_start[end + 1..];
    }

    values
}

fn validate_operation_category_values(root: &Path, errors: &mut Vec<ValidationError>) {
    if !OPERATION_CATEGORY_DOC_PATHS
        .iter()
        .any(|path| root.join(path).exists())
    {
        return;
    }

    let mut documented_values = Vec::new();
    for relative_path in OPERATION_CATEGORY_DOC_PATHS {
        let contents = match fs::read_to_string(root.join(relative_path)) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationError::new(
                    *relative_path,
                    "operation_category_values.read",
                    format!("failed to read operation category owner: {error}"),
                ));
                continue;
            }
        };
        let Some(section) = extract_anchored_markdown_section(&contents, OPERATION_CATEGORY_ANCHOR)
        else {
            errors.push(ValidationError::new(
                *relative_path,
                "operation_category_values.missing_section",
                format!("missing anchored operation category section #{OPERATION_CATEGORY_ANCHOR}"),
            ));
            continue;
        };
        let values = extract_first_column_identifier_values(section);
        if values.is_empty() {
            errors.push(ValidationError::new(
                *relative_path,
                "operation_category_values.invalid_table",
                "operation category section must contain a Markdown table with backticked values in its first column",
            ));
            continue;
        }
        documented_values.push((*relative_path, values));
    }

    if let [(en_path, en_values), (ko_path, ko_values)] = documented_values.as_slice() {
        if en_values != ko_values {
            let missing_from_ko = en_values.difference(ko_values).cloned().collect();
            let missing_from_en = ko_values.difference(en_values).cloned().collect();
            errors.push(ValidationError::new(
                *ko_path,
                "operation_category_values.language_drift",
                format!(
                    "operation category value sets differ between {en_path} and {ko_path}; missing from Korean owner: {}; missing from English owner: {}",
                    format_backticked_values(&missing_from_ko),
                    format_backticked_values(&missing_from_en),
                ),
            ));
        }
    }

    let mut required_identifiers = BTreeSet::from([OPERATION_CATEGORY_TERM_KEY.to_string()]);
    for (_, values) in &documented_values {
        required_identifiers.extend(values.iter().cloned());
    }
    validate_operation_category_terminology(root, &required_identifiers, errors);
}

fn extract_anchored_markdown_section<'a>(contents: &'a str, anchor: &str) -> Option<&'a str> {
    let marker = format!("<a id=\"{anchor}\"></a>");
    let section_start = contents.find(&marker)? + marker.len();
    let remaining = &contents[section_start..];
    let section_end = remaining.find("\n<a id=\"").unwrap_or(remaining.len());
    Some(&remaining[..section_end])
}

fn extract_first_column_identifier_values(section: &str) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    for line in section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            continue;
        }
        let cells = markdown_table_cells(trimmed);
        if cells.is_empty() || is_markdown_table_separator(&cells) {
            continue;
        }
        let identifiers = extract_backtick_values(cells[0]);
        if let [identifier] = identifiers.as_slice() {
            values.insert(identifier.clone());
        }
    }
    values
}

fn validate_operation_category_terminology(
    root: &Path,
    required_identifiers: &BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    let contents = match fs::read_to_string(root.join(TERMINOLOGY_MAP_PATH)) {
        Ok(contents) => contents,
        Err(error) => {
            errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                "operation_category_values.terminology_read",
                format!("failed to read terminology map: {error}"),
            ));
            return;
        }
    };
    let value: Value = match serde_yaml::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                "operation_category_values.terminology_yaml",
                format!("failed to parse terminology map YAML: {error}"),
            ));
            return;
        }
    };

    let preserved = value
        .as_mapping()
        .and_then(|top| mapping_get(top, "terms"))
        .and_then(Value::as_mapping)
        .and_then(|terms| mapping_get(terms, OPERATION_CATEGORY_TERM_KEY))
        .and_then(Value::as_mapping)
        .and_then(|entry| mapping_get(entry, "preserve_as_identifier"))
        .and_then(sequence_strings);
    let Some(preserved) = preserved else {
        errors.push(ValidationError::new(
            TERMINOLOGY_MAP_PATH,
            "operation_category_values.terminology_shape",
            format!(
                "terms.{OPERATION_CATEGORY_TERM_KEY}.preserve_as_identifier must be a sequence of strings"
            ),
        ));
        return;
    };
    let preserved: BTreeSet<_> = preserved.into_iter().collect();
    let missing: BTreeSet<_> = required_identifiers
        .difference(&preserved)
        .cloned()
        .collect();
    if !missing.is_empty() {
        errors.push(ValidationError::new(
            TERMINOLOGY_MAP_PATH,
            "operation_category_values.terminology_missing",
            format!(
                "terms.{OPERATION_CATEGORY_TERM_KEY}.preserve_as_identifier is missing {}",
                format_backticked_values(&missing)
            ),
        ));
    }
}

fn format_backticked_values(values: &BTreeSet<String>) -> String {
    if values.is_empty() {
        return "none".to_string();
    }
    values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn extract_canonical_storage_sql_block(contents: &str, label: &str) -> Option<String> {
    let start_marker = format!("<!-- canonical-storage-sql: {label} start -->");
    let end_marker = format!("<!-- canonical-storage-sql: {label} end -->");
    let start = contents.find(&start_marker)? + start_marker.len();
    let after_start = &contents[start..];
    let fence_start = after_start.find("```sql")? + "```sql".len();
    let after_fence = &after_start[fence_start..];
    let after_fence = after_fence.strip_prefix('\n').unwrap_or(after_fence);
    let fence_end = after_fence.find("```")?;
    let block = &after_fence[..fence_end];
    let after_block = &after_fence[fence_end + "```".len()..];
    if after_block.find(&end_marker).is_some() {
        Some(block.to_owned())
    } else {
        None
    }
}

fn normalize_canonical_sql_block(sql: &str) -> String {
    let normalized = sql.replace("\r\n", "\n");
    format!("{}\n", normalized.trim_end())
}

fn normalize_existing_root(root: &Path) -> Result<PathBuf> {
    root.canonicalize()
        .with_context(|| format!("failed to resolve repository root {}", root.display()))
}

fn validate_doc_index(root: &Path, errors: &mut Vec<ValidationError>) -> Option<DocIndex> {
    let doc_index = root.join(DOC_INDEX_PATH);
    let contents = match fs::read_to_string(&doc_index) {
        Ok(contents) => contents,
        Err(error) => {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.read",
                format!("failed to read doc index: {error}"),
            ));
            return None;
        }
    };

    let value: Value = match serde_yaml::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.yaml",
                format!("failed to parse YAML: {error}"),
            ));
            return None;
        }
    };

    let Some(top) = value.as_mapping() else {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            "doc index must be a YAML mapping",
        ));
        return None;
    };

    for field in TOP_LEVEL_REQUIRED {
        if mapping_get(top, field).is_none() {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.missing_field",
                format!("doc index is missing required top-level field {field}"),
            ));
        }
    }

    for field in top.keys().filter_map(Value::as_str) {
        if !TOP_LEVEL_ALLOWED.contains(&field) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.unknown_field",
                format!("doc index uses unsupported top-level field {field}"),
            ));
        }
    }

    match mapping_get(top, "version").and_then(Value::as_i64) {
        Some(3) => {}
        Some(version) => errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.version",
            format!("expected version 3, found {version}"),
        )),
        None => errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.version",
            "missing numeric version 3",
        )),
    }

    validate_top_level_mapping(top, "metadata", errors);
    validate_top_level_mapping(top, "language_retrieval", errors);
    validate_top_level_mapping(top, "entry_schema", errors);
    let owner_areas = validate_catalog(top, "owner_areas", errors);
    let applicability = validate_catalog(top, "applicability", errors);

    let mut entries = Vec::new();
    let mut doc_ids = BTreeSet::new();
    let mut indexed_paths = BTreeSet::new();
    let mut paired_paths = BTreeMap::new();
    let mut path_doc_ids = BTreeMap::new();
    let mut paired_documents = BTreeMap::new();

    validate_entries(
        root,
        top,
        "shared_documents",
        EntryMode::Shared,
        &mut entries,
        &mut doc_ids,
        &mut indexed_paths,
        &mut paired_paths,
        &mut path_doc_ids,
        &mut paired_documents,
        &owner_areas,
        &applicability,
        errors,
    );
    validate_entries(
        root,
        top,
        "documents",
        EntryMode::Paired,
        &mut entries,
        &mut doc_ids,
        &mut indexed_paths,
        &mut paired_paths,
        &mut path_doc_ids,
        &mut paired_documents,
        &owner_areas,
        &applicability,
        errors,
    );

    for required_path in REQUIRED_SHARED_PATHS {
        if !indexed_paths.contains(*required_path) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "coverage.missing_shared_index",
                format!("shared maintained path is not indexed: {required_path}"),
            ));
        }
        if !root.join(required_path).exists() {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "coverage.missing_shared_path",
                format!("shared maintained path does not exist: {required_path}"),
            ));
        }
    }

    for entry in &entries {
        for depends_on in &entry.depends_on {
            if !doc_ids.contains(depends_on) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_depends_on",
                    format!("{} depends on unknown doc_id {depends_on}", entry.doc_id),
                ));
            }
        }
    }

    Some(DocIndex {
        indexed_paths,
        paired_paths,
        path_doc_ids,
        paired_documents,
    })
}

fn validate_top_level_mapping(top: &Mapping, key: &'static str, errors: &mut Vec<ValidationError>) {
    if let Some(value) = mapping_get(top, key) {
        if !value.is_mapping() {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.shape",
                format!("{key} must be a mapping"),
            ));
        }
    }
}

fn validate_catalog(
    top: &Mapping,
    key: &'static str,
    errors: &mut Vec<ValidationError>,
) -> BTreeSet<String> {
    let mut identifiers = BTreeSet::new();
    let Some(value) = mapping_get(top, key) else {
        return identifiers;
    };
    let Some(catalog) = value.as_mapping() else {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            format!("{key} must be a mapping"),
        ));
        return identifiers;
    };

    if catalog.is_empty() {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.catalog",
            format!("{key} must not be empty"),
        ));
    }

    for (identifier, value) in catalog {
        let Some(identifier) = identifier.as_str() else {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key} identifiers must be strings"),
            ));
            continue;
        };
        if !is_catalog_identifier(identifier) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key} identifier {identifier} must use lowercase letters, digits, or underscores"),
            ));
        }
        identifiers.insert(identifier.to_string());

        let Some(entry) = value.as_mapping() else {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key}.{identifier} must be a mapping"),
            ));
            continue;
        };
        for field in entry.keys().filter_map(Value::as_str) {
            if !CATALOG_ENTRY_ALLOWED.contains(&field) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.unknown_field",
                    format!("{key}.{identifier} uses unsupported field {field}"),
                ));
            }
        }
        match mapping_get(entry, "description").and_then(Value::as_str) {
            Some(description) if !description.trim().is_empty() => {}
            Some(_) => errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key}.{identifier} description must not be empty"),
            )),
            None => errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.catalog",
                format!("{key}.{identifier} is missing string description"),
            )),
        }
    }

    identifiers
}

fn is_catalog_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

#[derive(Copy, Clone)]
enum EntryMode {
    Shared,
    Paired,
}

#[allow(clippy::too_many_arguments)]
fn validate_entries(
    root: &Path,
    top: &Mapping,
    key: &'static str,
    mode: EntryMode,
    entries: &mut Vec<DocEntry>,
    doc_ids: &mut BTreeSet<String>,
    indexed_paths: &mut BTreeSet<String>,
    paired_paths: &mut BTreeMap<String, (String, String)>,
    path_doc_ids: &mut BTreeMap<String, String>,
    paired_documents: &mut BTreeMap<String, PairedDocument>,
    owner_areas: &BTreeSet<String>,
    applicability: &BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    let Some(value) = mapping_get(top, key) else {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            format!("missing {key} sequence"),
        ));
        return;
    };
    let Some(sequence) = value.as_sequence() else {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.shape",
            format!("{key} must be a sequence"),
        ));
        return;
    };

    for (index, value) in sequence.iter().enumerate() {
        let label = format!("{key}[{index}]");
        let Some(entry) = value.as_mapping() else {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.entry_shape",
                format!("{label} must be a mapping"),
            ));
            continue;
        };

        let required = match mode {
            EntryMode::Shared => SHARED_REQUIRED,
            EntryMode::Paired => PAIRED_REQUIRED,
        };
        let allowed = match mode {
            EntryMode::Shared => SHARED_ALLOWED,
            EntryMode::Paired => PAIRED_ALLOWED,
        };

        for field in required {
            if mapping_get(entry, field).is_none() {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.missing_field",
                    format!("{label} is missing required field {field}"),
                ));
            }
        }

        for field in entry.keys().filter_map(Value::as_str) {
            if !allowed.contains(&field) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.unknown_field",
                    format!("{label} uses unsupported field {field}"),
                ));
            }
        }

        let doc_id = string_field(entry, "doc_id", &label, errors)
            .unwrap_or_else(|| format!("{key}.{index}"));
        if !doc_ids.insert(doc_id.clone()) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.duplicate_doc_id",
                format!("duplicate doc_id {doc_id}"),
            ));
        }

        let kind = string_field(entry, "kind", &label, errors);
        if let Some(kind) = kind.as_deref() {
            if !KINDS.contains(&kind) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_kind",
                    format!("{doc_id} uses unsupported kind {kind}"),
                ));
            }
        }

        let normative_level = string_field(entry, "normative_level", &label, errors);
        if let Some(normative_level) = normative_level.as_deref() {
            if !NORMATIVE_LEVELS.contains(&normative_level) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_normative_level",
                    format!("{doc_id} uses unsupported normative_level {normative_level}"),
                ));
            }
        }

        let translation_policy = mapping_get(entry, "translation_policy")
            .and_then(|_| string_field(entry, "translation_policy", &label, errors));
        if let Some(translation_policy) = translation_policy.as_deref() {
            if !TRANSLATION_POLICIES.contains(&translation_policy) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_translation_policy",
                    format!("{doc_id} uses unsupported translation_policy {translation_policy}"),
                ));
            }
        }

        for list_field in OPTIONAL_FIELDS {
            if let Some(items) = mapping_get(entry, list_field) {
                if sequence_strings(items).is_none() {
                    errors.push(ValidationError::new(
                        DOC_INDEX_PATH,
                        "metadata.invalid_list",
                        format!("{doc_id} field {list_field} must be a list of strings"),
                    ));
                }
            }
        }

        if let Some(journeys_value) = mapping_get(entry, "journeys") {
            if let Some(journeys) = sequence_strings(journeys_value) {
                for journey in journeys {
                    if !READER_JOURNEYS.contains(&journey.as_str()) {
                        errors.push(ValidationError::new(
                            DOC_INDEX_PATH,
                            "metadata.invalid_journey",
                            format!("{doc_id} uses unsupported journey {journey}"),
                        ));
                    }
                }
            }
        }

        let owner_area = string_field(entry, "owner_area", &label, errors);
        if let Some(owner_area) = owner_area.as_deref() {
            if !owner_areas.contains(owner_area) {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_owner_area",
                    format!("{doc_id} uses unknown owner_area {owner_area}"),
                ));
            }
        }

        let created_on = date_field(entry, "created_on", &label, errors);
        let last_updated_on = date_field(entry, "last_updated_on", &label, errors);
        let last_verified_on = date_field(entry, "last_verified_on", &label, errors);
        if let (Some(created_on), Some(last_updated_on), Some(last_verified_on)) =
            (created_on, last_updated_on, last_verified_on)
        {
            if created_on > last_updated_on {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_date_order",
                    format!("{doc_id} has created_on after last_updated_on"),
                ));
            }
            if last_updated_on > last_verified_on {
                errors.push(ValidationError::new(
                    DOC_INDEX_PATH,
                    "metadata.invalid_date_order",
                    format!("{doc_id} has last_updated_on after last_verified_on"),
                ));
            }
        }

        validate_applies_to(entry, &doc_id, applicability, errors);

        let mut paired_document = None;
        let paths = match mode {
            EntryMode::Shared => string_field(entry, "path", &label, errors)
                .into_iter()
                .collect::<Vec<_>>(),
            EntryMode::Paired => {
                let path_en = string_field(entry, "path_en", &label, errors);
                let path_ko = string_field(entry, "path_ko", &label, errors);
                if let (Some(path_en), Some(path_ko)) = (&path_en, &path_ko) {
                    validate_paired_paths(&doc_id, path_en, path_ko, errors);
                    paired_paths.insert(path_en.clone(), (path_en.clone(), path_ko.clone()));
                    paired_paths.insert(path_ko.clone(), (path_en.clone(), path_ko.clone()));
                    paired_document = Some(PairedDocument {
                        doc_id: doc_id.clone(),
                        path_en: path_en.clone(),
                        path_ko: path_ko.clone(),
                    });
                }
                path_en.into_iter().chain(path_ko).collect::<Vec<_>>()
            }
        };

        for path in &paths {
            validate_indexed_path(root, &doc_id, path, indexed_paths, errors);
            path_doc_ids.insert(path.clone(), doc_id.clone());
        }

        if let Some(paired_document) = paired_document {
            paired_documents.insert(doc_id.clone(), paired_document);
        }

        let depends_on = mapping_get(entry, "depends_on")
            .and_then(sequence_strings)
            .unwrap_or_default();

        entries.push(DocEntry { doc_id, depends_on });
    }
}

fn date_field(
    entry: &Mapping,
    key: &str,
    label: &str,
    errors: &mut Vec<ValidationError>,
) -> Option<IsoDate> {
    let value = string_field(entry, key, label, errors)?;
    match parse_iso_date(&value) {
        Ok(date) => Some(date),
        Err(error) => {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                error.category,
                format!("{label} field {key} {message}", message = error.message),
            ));
            None
        }
    }
}

fn parse_iso_date(value: &str) -> std::result::Result<IsoDate, DateError> {
    if value.len() != 10
        || value.as_bytes().get(4) != Some(&b'-')
        || value.as_bytes().get(7) != Some(&b'-')
        || !value
            .chars()
            .enumerate()
            .all(|(index, character)| matches!(index, 4 | 7) || character.is_ascii_digit())
    {
        return Err(DateError {
            category: "metadata.invalid_date_syntax",
            message: format!("must use YYYY-MM-DD, found {value}"),
        });
    }

    let year = value[0..4].parse::<u16>().map_err(|_| DateError {
        category: "metadata.invalid_date_syntax",
        message: format!("must use YYYY-MM-DD, found {value}"),
    })?;
    let month = value[5..7].parse::<u8>().map_err(|_| DateError {
        category: "metadata.invalid_date_syntax",
        message: format!("must use YYYY-MM-DD, found {value}"),
    })?;
    let day = value[8..10].parse::<u8>().map_err(|_| DateError {
        category: "metadata.invalid_date_syntax",
        message: format!("must use YYYY-MM-DD, found {value}"),
    })?;

    if year == 0 || month == 0 || month > 12 {
        return Err(DateError {
            category: "metadata.invalid_date_calendar",
            message: format!("is not a valid calendar date: {value}"),
        });
    }
    let max_day = days_in_month(year, month);
    if day == 0 || day > max_day {
        return Err(DateError {
            category: "metadata.invalid_date_calendar",
            message: format!("is not a valid calendar date: {value}"),
        });
    }

    Ok(IsoDate { year, month, day })
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn validate_applies_to(
    entry: &Mapping,
    doc_id: &str,
    applicability: &BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    let Some(value) = mapping_get(entry, "applies_to") else {
        return;
    };
    let Some(items) = sequence_strings(value) else {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.invalid_list",
            format!("{doc_id} field applies_to must be a list of strings"),
        ));
        return;
    };

    if items.is_empty() {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.invalid_applies_to",
            format!("{doc_id} field applies_to must not be empty"),
        ));
    }

    let mut seen = BTreeSet::new();
    for item in items {
        if !seen.insert(item.clone()) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.duplicate_applicability",
                format!("{doc_id} repeats applies_to value {item}"),
            ));
        }
        if !applicability.contains(&item) {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.invalid_applicability",
                format!("{doc_id} uses unknown applies_to value {item}"),
            ));
        }
    }
}

fn validate_paired_paths(
    doc_id: &str,
    path_en: &str,
    path_ko: &str,
    errors: &mut Vec<ValidationError>,
) {
    if is_mirrored_docs_pair(path_en, path_ko) || is_root_readme_pair(path_en, path_ko) {
        return;
    }

    errors.push(ValidationError::new(
        DOC_INDEX_PATH,
        "coverage.unmirrored_pair",
        format!("{doc_id} does not use mirrored language-relative paths: {path_en} <-> {path_ko}"),
    ));
}

fn is_mirrored_docs_pair(path_en: &str, path_ko: &str) -> bool {
    let en_relative = path_en.strip_prefix("docs/en/");
    let ko_relative = path_ko.strip_prefix("docs/ko/");
    matches!((en_relative, ko_relative), (Some(en), Some(ko)) if en == ko)
}

fn is_root_readme_pair(path_en: &str, path_ko: &str) -> bool {
    path_en == ROOT_README_EN_PATH && path_ko == ROOT_README_KO_PATH
}

fn validate_indexed_path(
    root: &Path,
    doc_id: &str,
    path: &str,
    indexed_paths: &mut BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    if path.starts_with('/') || path.contains('\\') || path.split('/').any(|part| part == "..") {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.invalid_path",
            format!("{doc_id} uses non repository-relative path {path}"),
        ));
        return;
    }

    if !indexed_paths.insert(path.to_string()) {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.duplicate_path",
            format!("indexed path appears more than once: {path}"),
        ));
    }

    if !root.join(path).exists() {
        errors.push(ValidationError::new(
            DOC_INDEX_PATH,
            "metadata.missing_path",
            format!("{doc_id} indexed path does not exist: {path}"),
        ));
    }
}

fn string_field(
    entry: &Mapping,
    key: &str,
    label: &str,
    errors: &mut Vec<ValidationError>,
) -> Option<String> {
    let value = mapping_get(entry, key)?;
    match value.as_str() {
        Some(value) => Some(value.to_string()),
        None => {
            errors.push(ValidationError::new(
                DOC_INDEX_PATH,
                "metadata.invalid_field_type",
                format!("{label} field {key} must be a string"),
            ));
            None
        }
    }
}

fn sequence_strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_sequence()?
        .iter()
        .map(|item| item.as_str().map(ToOwned::to_owned))
        .collect()
}

fn validate_document_coverage(root: &Path, index: &DocIndex, errors: &mut Vec<ValidationError>) {
    let en_files = markdown_files_under(root, "docs/en", errors);
    let ko_files = markdown_files_under(root, "docs/ko", errors);
    let ko_set: BTreeSet<_> = ko_files.iter().cloned().collect();
    let en_set: BTreeSet<_> = en_files.iter().cloned().collect();

    for en_path in en_files {
        let Some(relative) = en_path.strip_prefix("docs/en/") else {
            continue;
        };
        let ko_path = format!("docs/ko/{relative}");
        if !ko_set.contains(&ko_path) {
            errors.push(ValidationError::new(
                &en_path,
                "coverage.missing_pair",
                format!("missing Korean paired file {ko_path}"),
            ));
            continue;
        }
        if !index.paired_paths.contains_key(&en_path) {
            errors.push(ValidationError::new(
                &en_path,
                "coverage.unindexed_pair",
                format!("English maintained Markdown file is not indexed with pair {ko_path}"),
            ));
        }
    }

    for ko_path in ko_files {
        let Some(relative) = ko_path.strip_prefix("docs/ko/") else {
            continue;
        };
        let en_path = format!("docs/en/{relative}");
        if !en_set.contains(&en_path) {
            errors.push(ValidationError::new(
                &ko_path,
                "coverage.missing_pair",
                format!("missing English paired file {en_path}"),
            ));
            continue;
        }
        if !index.paired_paths.contains_key(&ko_path) {
            errors.push(ValidationError::new(
                &ko_path,
                "coverage.unindexed_pair",
                format!("Korean maintained Markdown file is not indexed with pair {en_path}"),
            ));
        }
    }

    validate_root_readme_pair_coverage(root, index, errors);
}

fn validate_root_readme_pair_coverage(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationError>,
) {
    if !root.join(ROOT_README_KO_PATH).exists() {
        return;
    }

    let indexed_as_root_pair = matches!(
        index.paired_paths.get(ROOT_README_KO_PATH),
        Some((path_en, path_ko)) if is_root_readme_pair(path_en, path_ko)
    );
    if !indexed_as_root_pair {
        errors.push(ValidationError::new(
            ROOT_README_KO_PATH,
            "coverage.unindexed_pair",
            format!(
                "{ROOT_README_KO_PATH} must be indexed with root README pair {ROOT_README_EN_PATH} <-> {ROOT_README_KO_PATH}"
            ),
        ));
    }
}

fn markdown_files_under(
    root: &Path,
    relative_dir: &str,
    errors: &mut Vec<ValidationError>,
) -> Vec<String> {
    let mut files = Vec::new();
    collect_markdown_files(root, &root.join(relative_dir), &mut files, errors);
    files.sort();
    files
}

fn collect_markdown_files(
    root: &Path,
    dir: &Path,
    files: &mut Vec<String>,
    errors: &mut Vec<ValidationError>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(ValidationError::new(
                repo_relative(root, dir),
                "coverage.read_dir",
                format!("failed to read documentation directory: {error}"),
            ));
            return;
        }
    };

    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(root, &path, files, errors);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(repo_relative(root, &path));
        }
    }
}

fn validate_markdown_links(root: &Path, index: &DocIndex, errors: &mut Vec<ValidationError>) {
    let mut cache = AnchorCache::default();
    for path in index
        .indexed_paths
        .iter()
        .filter(|path| path.ends_with(".md"))
    {
        let absolute_path = root.join(path);
        let contents = match fs::read_to_string(&absolute_path) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationError::new(
                    path,
                    "link.read",
                    format!("failed to read Markdown file: {error}"),
                ));
                continue;
            }
        };
        for link in markdown_links(&contents) {
            if is_ignored_link(&link) {
                continue;
            }
            if let Err(failure) = validate_local_target(root, path, &link, &mut cache) {
                errors.push(ValidationError::new(
                    path,
                    failure.category,
                    failure.message,
                ));
            }
        }
    }
}

fn validate_bilingual_link_parity(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationError>,
) {
    for paired in index.paired_documents.values() {
        let en_links = match collect_semantic_links(root, index, &paired.path_en) {
            Ok(links) => links,
            Err(error) => {
                errors.push(ValidationError::new(
                    &paired.path_en,
                    "bilingual_link.read",
                    error,
                ));
                continue;
            }
        };
        let ko_links = match collect_semantic_links(root, index, &paired.path_ko) {
            Ok(links) => links,
            Err(error) => {
                errors.push(ValidationError::new(
                    &paired.path_ko,
                    "bilingual_link.read",
                    error,
                ));
                continue;
            }
        };

        compare_semantic_link_multisets(paired, en_links, ko_links, errors);
    }
}

fn collect_semantic_links(
    root: &Path,
    index: &DocIndex,
    path: &str,
) -> std::result::Result<BTreeMap<SemanticLinkKey, usize>, String> {
    let contents = fs::read_to_string(root.join(path))
        .map_err(|error| format!("failed to read Markdown file: {error}"))?;
    let mut links = BTreeMap::new();
    for link in markdown_reader_links(&contents) {
        if is_ignored_link(&link) {
            continue;
        }
        if let Some(key) = normalize_semantic_link(root, index, path, &link) {
            *links.entry(key).or_insert(0) += 1;
        }
    }
    Ok(links)
}

fn normalize_semantic_link(
    root: &Path,
    index: &DocIndex,
    source: &str,
    link: &str,
) -> Option<SemanticLinkKey> {
    let resolved = resolve_link_target(root, source, link).ok()?;
    let target_absolute = root.join(&resolved.path);
    if !target_absolute.exists() {
        return None;
    }

    let indexed_lookup_path = indexed_target_lookup_path(root, &resolved.path);
    let target = index
        .path_doc_ids
        .get(&indexed_lookup_path)
        .cloned()
        .map(SemanticLinkTarget::DocId)
        .unwrap_or_else(|| SemanticLinkTarget::RepositoryPath(resolved.path));

    Some(SemanticLinkKey {
        target,
        fragment: resolved.fragment,
    })
}

fn indexed_target_lookup_path(root: &Path, path: &str) -> String {
    let absolute = root.join(path);
    if absolute.is_dir() {
        let readme = absolute.join("README.md");
        if readme.exists() {
            return repo_relative(root, &readme);
        }
    }
    path.to_string()
}

fn compare_semantic_link_multisets(
    paired: &PairedDocument,
    en_links: BTreeMap<SemanticLinkKey, usize>,
    ko_links: BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationError>,
) {
    let mut only_en = multiset_difference(&en_links, &ko_links);
    let mut only_ko = multiset_difference(&ko_links, &en_links);

    report_fragment_mismatches(paired, &mut only_en, &mut only_ko, errors);
    report_target_mismatches(paired, &mut only_en, &mut only_ko, errors);
    report_unpaired_semantic_links(paired, "bilingual_link.only_en", true, only_en, errors);
    report_unpaired_semantic_links(paired, "bilingual_link.only_ko", false, only_ko, errors);
}

fn multiset_difference(
    left: &BTreeMap<SemanticLinkKey, usize>,
    right: &BTreeMap<SemanticLinkKey, usize>,
) -> BTreeMap<SemanticLinkKey, usize> {
    let mut difference = BTreeMap::new();
    for (key, left_count) in left {
        let right_count = right.get(key).copied().unwrap_or(0);
        if *left_count > right_count {
            difference.insert(key.clone(), left_count - right_count);
        }
    }
    difference
}

fn report_fragment_mismatches(
    paired: &PairedDocument,
    only_en: &mut BTreeMap<SemanticLinkKey, usize>,
    only_ko: &mut BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationError>,
) {
    let en_keys = only_en.keys().cloned().collect::<Vec<_>>();
    for en_key in en_keys {
        while count_for(only_en, &en_key) > 0 {
            let Some(ko_key) = only_ko
                .keys()
                .find(|ko_key| ko_key.target == en_key.target && ko_key.fragment != en_key.fragment)
                .cloned()
            else {
                break;
            };
            let count = count_for(only_en, &en_key).min(count_for(only_ko, &ko_key));
            consume_count(only_en, &en_key, count);
            consume_count(only_ko, &ko_key, count);
            errors.push(ValidationError::new(
                &paired.path_en,
                "bilingual_link.fragment_mismatch",
                format!(
                    "{} has {count} paired local semantic link occurrence(s) to {} but different fragments: English {}, Korean {} ({} <-> {})",
                    paired.doc_id,
                    en_key.target.describe(),
                    describe_fragment(&en_key.fragment),
                    describe_fragment(&ko_key.fragment),
                    paired.path_en,
                    paired.path_ko
                ),
            ));
        }
    }
}

fn report_target_mismatches(
    paired: &PairedDocument,
    only_en: &mut BTreeMap<SemanticLinkKey, usize>,
    only_ko: &mut BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationError>,
) {
    let en_keys = only_en.keys().cloned().collect::<Vec<_>>();
    for en_key in en_keys {
        while count_for(only_en, &en_key) > 0 {
            let Some(ko_key) = only_ko
                .keys()
                .find(|ko_key| ko_key.fragment == en_key.fragment && ko_key.target != en_key.target)
                .cloned()
            else {
                break;
            };
            let count = count_for(only_en, &en_key).min(count_for(only_ko, &ko_key));
            consume_count(only_en, &en_key, count);
            consume_count(only_ko, &ko_key, count);
            errors.push(ValidationError::new(
                &paired.path_en,
                "bilingual_link.target_mismatch",
                format!(
                    "{} has {count} paired local semantic link occurrence(s) with {} but different normalized targets: English {}, Korean {} ({} <-> {})",
                    paired.doc_id,
                    describe_fragment(&en_key.fragment),
                    en_key.target.describe(),
                    ko_key.target.describe(),
                    paired.path_en,
                    paired.path_ko
                ),
            ));
        }
    }
}

fn report_unpaired_semantic_links(
    paired: &PairedDocument,
    category: &'static str,
    english_surplus: bool,
    links: BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationError>,
) {
    for (key, count) in links {
        let language = if english_surplus { "English" } else { "Korean" };
        let paired_language = if english_surplus { "Korean" } else { "English" };
        errors.push(ValidationError::new(
            &paired.path_en,
            category,
            format!(
                "{} has {count} more {language} occurrence(s) of local semantic link to {} than {paired_language} ({} <-> {})",
                paired.doc_id,
                key.describe(),
                paired.path_en,
                paired.path_ko
            ),
        ));
    }
}

fn count_for(links: &BTreeMap<SemanticLinkKey, usize>, key: &SemanticLinkKey) -> usize {
    links.get(key).copied().unwrap_or(0)
}

fn consume_count(
    links: &mut BTreeMap<SemanticLinkKey, usize>,
    key: &SemanticLinkKey,
    count: usize,
) {
    if let Some(current) = links.get_mut(key) {
        *current -= count;
        if *current == 0 {
            links.remove(key);
        }
    }
}

impl SemanticLinkKey {
    fn describe(&self) -> String {
        match &self.fragment {
            Some(fragment) => format!("{}#{fragment}", self.target.describe()),
            None => format!("{} without fragment", self.target.describe()),
        }
    }
}

impl SemanticLinkTarget {
    fn describe(&self) -> String {
        match self {
            SemanticLinkTarget::DocId(doc_id) => format!("target {doc_id}"),
            SemanticLinkTarget::RepositoryPath(path) => format!("repository path {path}"),
        }
    }
}

fn describe_fragment(fragment: &Option<String>) -> String {
    match fragment {
        Some(fragment) => format!("#{fragment}"),
        None => "no fragment".to_string(),
    }
}

fn markdown_links(contents: &str) -> Vec<String> {
    markdown_destinations(contents, true)
}

fn markdown_reader_links(contents: &str) -> Vec<String> {
    markdown_destinations(contents, false)
}

fn markdown_destinations(contents: &str, include_images: bool) -> Vec<String> {
    let mut links = Vec::new();
    let parser = Parser::new_ext(contents, markdown_options());
    for event in parser {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                links.push(dest_url.to_string());
            }
            Event::Start(Tag::Image { dest_url, .. }) if include_images => {
                links.push(dest_url.to_string());
            }
            _ => {}
        }
    }
    links
}

fn validate_local_target(
    root: &Path,
    source: &str,
    link: &str,
    cache: &mut AnchorCache,
) -> std::result::Result<(), LinkFailure> {
    let resolved = resolve_link_target(root, source, link).map_err(|message| LinkFailure {
        category: "link.invalid_target",
        message,
    })?;

    let target_absolute = root.join(&resolved.path);
    if !target_absolute.exists() {
        return Err(LinkFailure {
            category: "link.missing_target",
            message: format!("link {link} resolves to missing target {}", resolved.path),
        });
    }

    if let Some(fragment) = resolved.fragment {
        let anchor_path = if target_absolute.is_dir() {
            let readme = target_absolute.join("README.md");
            if readme.exists() {
                repo_relative(root, &readme)
            } else {
                return Err(LinkFailure {
                    category: "link.missing_fragment",
                    message: format!(
                        "link {link} has fragment #{fragment}, but {} is a directory without README.md",
                        resolved.path
                    ),
                });
            }
        } else {
            resolved.path.clone()
        };

        if !anchor_path.ends_with(".md") {
            return Err(LinkFailure {
                category: "link.missing_fragment",
                message: format!(
                    "link {link} has fragment #{fragment}, but {anchor_path} is not Markdown"
                ),
            });
        }

        let anchors = cache
            .anchors_for(root, &anchor_path)
            .map_err(|message| LinkFailure {
                category: "link.read",
                message,
            })?;
        if !anchors.contains_fragment(&fragment) {
            return Err(LinkFailure {
                category: "link.missing_fragment",
                message: format!(
                    "link {link} resolves to {anchor_path} without fragment #{fragment}"
                ),
            });
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct ResolvedLink {
    path: String,
    fragment: Option<String>,
}

fn resolve_link_target(
    root: &Path,
    source: &str,
    link: &str,
) -> std::result::Result<ResolvedLink, String> {
    let (path_part, fragment) = split_link(link);
    let path_part = percent_decode(&path_part)
        .map_err(|error| format!("link {link} has invalid percent encoding: {error}"))?;
    let fragment = fragment
        .map(|fragment| {
            percent_decode(&fragment).map(|decoded| decoded.trim_start_matches('#').to_string())
        })
        .transpose()
        .map_err(|error| format!("link {link} has invalid fragment percent encoding: {error}"))?;

    let source_parent = Path::new(source).parent().unwrap_or_else(|| Path::new(""));
    let joined = if path_part.is_empty() {
        root.join(source)
    } else if let Some(stripped) = path_part.strip_prefix('/') {
        root.join(stripped)
    } else {
        root.join(source_parent).join(path_part)
    };
    let normalized = normalize_path(&joined);
    let relative = normalized
        .strip_prefix(root)
        .map_err(|_| format!("link {link} resolves outside the repository"))?;

    Ok(ResolvedLink {
        path: path_to_slash(relative),
        fragment,
    })
}

fn split_link(link: &str) -> (String, Option<String>) {
    let without_query = link.split('?').next().unwrap_or(link);
    match without_query.split_once('#') {
        Some((path, fragment)) => (path.to_string(), Some(fragment.to_string())),
        None => (without_query.to_string(), None),
    }
}

impl AnchorCache {
    fn anchors_for(
        &mut self,
        root: &Path,
        path: &str,
    ) -> std::result::Result<&MarkdownAnchors, String> {
        if !self.files.contains_key(path) {
            let contents = fs::read_to_string(root.join(path))
                .map_err(|error| format!("failed to read Markdown target {path}: {error}"))?;
            let anchors = collect_markdown_anchors(&contents);
            self.files.insert(path.to_string(), anchors);
        }
        Ok(self.files.get(path).expect("anchor cache entry inserted"))
    }
}

impl MarkdownAnchors {
    fn contains_fragment(&self, fragment: &str) -> bool {
        self.anchors.contains(fragment)
            || fragment
                .strip_prefix("user-content-")
                .is_some_and(|stripped| self.anchors.contains(stripped))
    }
}

fn collect_markdown_anchors(contents: &str) -> MarkdownAnchors {
    let mut anchors = BTreeSet::new();
    let mut slug_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut heading_text = String::new();
    let mut in_heading = false;

    for event in Parser::new_ext(contents, markdown_options()) {
        match event {
            Event::Start(Tag::Heading { id, .. }) => {
                in_heading = true;
                heading_text.clear();
                if let Some(id) = id {
                    anchors.insert(id.to_string());
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let base = slugify_heading(&heading_text);
                if !base.is_empty() {
                    let count = slug_counts.entry(base.clone()).or_insert(0);
                    let anchor = if *count == 0 {
                        base
                    } else {
                        format!("{base}-{count}")
                    };
                    *count += 1;
                    anchors.insert(anchor);
                }
            }
            Event::Text(text) | Event::Code(text) if in_heading => {
                heading_text.push_str(&text);
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                for id in html_anchor_ids(&html) {
                    anchors.insert(id);
                }
            }
            _ => {}
        }
    }

    MarkdownAnchors { anchors }
}

fn markdown_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options
}

fn slugify_heading(heading: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for character in heading.trim().chars() {
        for lower in character.to_lowercase() {
            if lower.is_alphanumeric() {
                slug.push(lower);
                previous_dash = false;
            } else if lower.is_whitespace() || lower == '-' {
                if !previous_dash && !slug.is_empty() {
                    slug.push('-');
                    previous_dash = true;
                }
            } else if lower == '_' {
                slug.push(lower);
                previous_dash = false;
            }
        }
    }

    slug.trim_matches('-').to_string()
}

fn html_anchor_ids(html: &str) -> Vec<String> {
    let mut ids = Vec::new();
    ids.extend(html_attribute_values(html, "id"));
    if html.trim_start().to_ascii_lowercase().starts_with("<a") {
        ids.extend(html_attribute_values(html, "name"));
    }
    ids
}

fn html_attribute_values(html: &str, attribute: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut values = Vec::new();
    let mut search_start = 0;
    let needle = format!("{attribute}=");

    while let Some(offset) = lower[search_start..].find(&needle) {
        let value_start = search_start + offset + needle.len();
        let Some(quote) = html[value_start..].chars().next() else {
            break;
        };
        if quote != '"' && quote != '\'' {
            search_start = value_start;
            continue;
        }
        let content_start = value_start + quote.len_utf8();
        let Some(end_offset) = html[content_start..].find(quote) else {
            break;
        };
        values.push(html[content_start..content_start + end_offset].to_string());
        search_start = content_start + end_offset + quote.len_utf8();
    }

    values
}

fn validate_terminology_paths(root: &Path, index: &DocIndex, errors: &mut Vec<ValidationError>) {
    let path = root.join(TERMINOLOGY_MAP_PATH);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.read",
                format!("failed to read terminology map: {error}"),
            ));
            return;
        }
    };
    let value: Value = match serde_yaml::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.yaml",
                format!("failed to parse YAML: {error}"),
            ));
            return;
        }
    };

    validate_terminology_roles(&value, errors);

    let mut mentions = BTreeSet::new();
    collect_yaml_path_mentions(&value, &mut mentions);

    let mut cache = AnchorCache::default();
    for mention in mentions {
        if let Err(failure) = validate_terminology_target(root, index, &mention, &mut cache) {
            errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                failure.category,
                failure.message,
            ));
        }
    }
}

fn validate_terminology_roles(value: &Value, errors: &mut Vec<ValidationError>) {
    let Some(top) = value.as_mapping() else {
        errors.push(ValidationError::new(
            TERMINOLOGY_MAP_PATH,
            "terminology.shape",
            "terminology map must be a YAML mapping",
        ));
        return;
    };
    let Some(terms) = mapping_get(top, "terms") else {
        errors.push(ValidationError::new(
            TERMINOLOGY_MAP_PATH,
            "terminology.missing_terms",
            "terminology map is missing terms",
        ));
        return;
    };
    let Some(terms) = terms.as_mapping() else {
        errors.push(ValidationError::new(
            TERMINOLOGY_MAP_PATH,
            "terminology.shape",
            "terminology map terms must be a mapping",
        ));
        return;
    };

    let mut role_map = BTreeMap::new();
    for (term_key, entry) in terms {
        let Some(term_key) = term_key.as_str() else {
            continue;
        };
        let Some(entry) = entry.as_mapping() else {
            continue;
        };
        let Some(roles_value) = mapping_get(entry, "roles") else {
            continue;
        };

        let mut roles = BTreeSet::new();
        match roles_value.as_sequence() {
            Some(sequence) if !sequence.is_empty() => {
                for role in sequence {
                    let Some(role) = role.as_str() else {
                        errors.push(ValidationError::new(
                            TERMINOLOGY_MAP_PATH,
                            "terminology.invalid_role",
                            format!("{term_key} role values must be strings"),
                        ));
                        continue;
                    };
                    if !TERMINOLOGY_ALLOWED_ROLES.contains(&role) {
                        errors.push(ValidationError::new(
                            TERMINOLOGY_MAP_PATH,
                            "terminology.invalid_role",
                            format!("{term_key} uses unsupported terminology role {role}"),
                        ));
                    }
                    if !roles.insert(role.to_string()) {
                        errors.push(ValidationError::new(
                            TERMINOLOGY_MAP_PATH,
                            "terminology.invalid_role",
                            format!("{term_key} repeats terminology role {role}"),
                        ));
                    }
                }
            }
            Some(_) => errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.invalid_role",
                format!("{term_key} roles must not be empty"),
            )),
            None => errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.invalid_role",
                format!("{term_key} roles must be a list"),
            )),
        }
        role_map.insert(term_key.to_string(), roles);
    }

    for required in REQUIRED_TERMINOLOGY_ROLES {
        let Some(entry) = mapping_get(terms, required.term_key) else {
            errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.missing_required_term",
                format!("required terminology term {} is missing", required.display),
            ));
            continue;
        };
        if !entry.is_mapping() {
            errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.shape",
                format!(
                    "required terminology term {} must be a mapping",
                    required.display
                ),
            ));
            continue;
        }
        let Some(roles) = role_map.get(required.term_key) else {
            errors.push(ValidationError::new(
                TERMINOLOGY_MAP_PATH,
                "terminology.missing_role",
                format!(
                    "required terminology term {} is missing roles metadata",
                    required.display
                ),
            ));
            continue;
        };
        for role in required.roles {
            if !roles.contains(*role) {
                errors.push(ValidationError::new(
                    TERMINOLOGY_MAP_PATH,
                    "terminology.missing_role",
                    format!(
                        "required terminology term {} is missing role {}",
                        required.display, role
                    ),
                ));
            }
        }
    }
}

fn validate_terminology_target(
    root: &Path,
    index: &DocIndex,
    mention: &str,
    cache: &mut AnchorCache,
) -> std::result::Result<(), LinkFailure> {
    let (path, fragment) = split_link(mention);
    let path = percent_decode(&path).map_err(|error| LinkFailure {
        category: "terminology.invalid_target",
        message: format!("path {mention} has invalid percent encoding: {error}"),
    })?;
    if path.contains('{') || path.contains('}') || path.contains('*') {
        return Ok(());
    }
    if !is_repository_document_path(&path) {
        return Ok(());
    }
    let normalized = normalize_path(&PathBuf::from(&path));
    let path = path_to_slash(&normalized);
    if !root.join(&path).exists() {
        return Err(LinkFailure {
            category: "terminology.missing_target",
            message: format!("path reference does not exist: {mention}"),
        });
    }
    if !index.indexed_paths.contains(&path) {
        return Err(LinkFailure {
            category: "terminology.unindexed_target",
            message: format!("path reference is not indexed in docs/doc-index.yaml: {mention}"),
        });
    }
    if let Some(fragment) = fragment {
        let fragment = percent_decode(&fragment).map_err(|error| LinkFailure {
            category: "terminology.invalid_target",
            message: format!("path {mention} has invalid fragment percent encoding: {error}"),
        })?;
        if path.ends_with(".md") {
            let anchors = cache
                .anchors_for(root, &path)
                .map_err(|message| LinkFailure {
                    category: "terminology.read",
                    message,
                })?;
            if !anchors.contains_fragment(&fragment) {
                return Err(LinkFailure {
                    category: "terminology.missing_fragment",
                    message: format!(
                        "path reference {mention} points to missing fragment #{fragment}"
                    ),
                });
            }
        }
    }
    Ok(())
}

fn collect_yaml_path_mentions(value: &Value, mentions: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => {
            for mention in path_mentions_in_text(text) {
                mentions.insert(mention);
            }
        }
        Value::Sequence(items) => {
            for item in items {
                collect_yaml_path_mentions(item, mentions);
            }
        }
        Value::Mapping(mapping) => {
            for (key, value) in mapping {
                collect_yaml_path_mentions(key, mentions);
                collect_yaml_path_mentions(value, mentions);
            }
        }
        _ => {}
    }
}

fn validate_volicord_command_examples(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationError>,
) {
    for path in index
        .indexed_paths
        .iter()
        .filter(|path| path.ends_with(".md"))
    {
        let contents = match fs::read_to_string(root.join(path)) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationError::new(
                    path,
                    "command.read",
                    format!("failed to read Markdown file: {error}"),
                ));
                continue;
            }
        };

        for example in volicord_command_examples(&contents) {
            let result = match &example.tokens {
                Ok(tokens) => validate_volicord_command(tokens),
                Err(error) => Err(error.clone()),
            };
            if let Err(message) = result {
                errors.push(ValidationError::new(
                    path,
                    "command.invalid_example",
                    format!(
                        "line {} command `{}` is not supported: {}",
                        example.line, example.command, message
                    ),
                ));
            }
        }
    }
}

fn volicord_command_examples(contents: &str) -> Vec<VolicordCommandExample> {
    let mut examples = Vec::new();
    let mut active_fence = None;
    let mut pending = None;

    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        if let Some(fence) = active_fence.as_ref() {
            if is_closing_fence(line, fence) {
                active_fence = None;
                pending = None;
                continue;
            }
            if fence.shell {
                collect_shell_command_line(line, line_number, &mut pending, &mut examples);
            }
            continue;
        }

        if let Some(fence) = opening_fence(line) {
            active_fence = Some(fence);
        }
    }

    if let Some(pending) = pending {
        push_shell_command_candidate(pending.line, &pending.text, &mut examples);
    }

    examples
}

fn collect_shell_command_line(
    line: &str,
    line_number: usize,
    pending: &mut Option<PendingShellCommand>,
    examples: &mut Vec<VolicordCommandExample>,
) {
    let trimmed = strip_shell_prompt(line).trim();
    if pending.is_none() && (trimmed.is_empty() || trimmed.starts_with('#')) {
        return;
    }

    let (continued_text, continues) = split_shell_continuation(trimmed);
    if let Some(pending_command) = pending.as_mut() {
        if !continued_text.is_empty() {
            pending_command.text.push(' ');
            pending_command.text.push_str(continued_text);
        }
    } else {
        *pending = Some(PendingShellCommand {
            line: line_number,
            text: continued_text.to_string(),
        });
    }

    if !continues {
        if let Some(command) = pending.take() {
            push_shell_command_candidate(command.line, &command.text, examples);
        }
    }
}

fn split_shell_continuation(line: &str) -> (&str, bool) {
    let trimmed = line.trim_end();
    if trimmed.ends_with('\\') {
        (trimmed.trim_end_matches('\\').trim_end(), true)
    } else {
        (trimmed, false)
    }
}

fn push_shell_command_candidate(
    line: usize,
    command: &str,
    examples: &mut Vec<VolicordCommandExample>,
) {
    let tokens = shell_words_until_control(command);
    match &tokens {
        Ok(tokens) if volicord_command_start(tokens).is_none() => return,
        Err(_) if !starts_like_volicord_command(command) => return,
        _ => {}
    }

    examples.push(VolicordCommandExample {
        line,
        command: command.to_string(),
        tokens,
    });
}

fn opening_fence(line: &str) -> Option<ActiveFence> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let length = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    if length < 3 {
        return None;
    }
    let info = trimmed[length..].trim();
    Some(ActiveFence {
        marker,
        length,
        shell: is_shell_code_info(info),
    })
}

fn is_closing_fence(line: &str, fence: &ActiveFence) -> bool {
    let trimmed = line.trim_start();
    let length = trimmed
        .chars()
        .take_while(|character| *character == fence.marker)
        .count();
    length >= fence.length && trimmed[length..].trim().is_empty()
}

fn is_shell_code_info(info: &str) -> bool {
    let normalized = info
        .trim()
        .trim_matches('{')
        .trim_matches('}')
        .trim_start_matches('.')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "sh" | "shell" | "bash" | "zsh" | "console"
    )
}

fn strip_shell_prompt(line: &str) -> &str {
    let trimmed = line.trim_start();
    for prompt in ["$ ", "% ", "> "] {
        if let Some(command) = trimmed.strip_prefix(prompt) {
            return command;
        }
    }
    trimmed
}

fn shell_words_until_control(command: &str) -> std::result::Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for character in command.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        match quote {
            Some('\'') => {
                if character == '\'' {
                    quote = None;
                } else {
                    current.push(character);
                }
            }
            Some('"') => match character {
                '"' => quote = None,
                '\\' => escaped = true,
                _ => current.push(character),
            },
            Some(_) => unreachable!("only shell quote markers are used"),
            None => match character {
                '\\' => escaped = true,
                '\'' | '"' => quote = Some(character),
                '#' if current.is_empty() => break,
                '|' | ';' | '&' | '<' | '>' => {
                    push_shell_word(&mut words, &mut current);
                    break;
                }
                character if character.is_whitespace() => {
                    push_shell_word(&mut words, &mut current);
                }
                _ => current.push(character),
            },
        }
    }

    if let Some(quote) = quote {
        return Err(format!("unterminated {quote} quote"));
    }
    if escaped {
        current.push('\\');
    }
    push_shell_word(&mut words, &mut current);
    Ok(words)
}

fn push_shell_word(words: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        words.push(std::mem::take(current));
    }
}

fn starts_like_volicord_command(command: &str) -> bool {
    let mut tokens = command.split_whitespace();
    let mut token = tokens.next();
    if token.is_some_and(|token| token == "env") {
        token = tokens.next();
    }
    while token.is_some_and(is_env_assignment) {
        token = tokens.next();
    }
    token.is_some_and(is_volicord_binary)
}

fn validate_volicord_command(tokens: &[String]) -> std::result::Result<(), String> {
    let Some(command_start) = volicord_command_start(tokens) else {
        return Ok(());
    };
    let args = &tokens[command_start + 1..];
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(());
    };

    match command {
        "-h" | "--help" | "help" => validate_no_more_args(args, "volicord help"),
        "-V" | "--version" => validate_no_more_args(args, "volicord version"),
        "status" => validate_status_command(&args[1..]),
        "doctor" => validate_doctor_command(&args[1..]),
        "mcp" => validate_mcp_command(&args[1..]),
        "serve" => validate_serve_command(&args[1..]),
        "init" => validate_init_command(&args[1..]),
        "connection" => validate_connection_command(&args[1..]),
        "project" => validate_project_command(&args[1..]),
        "inbox" => validate_inbox_command(&args[1..]),
        other => Err(format!(
            "unknown `volicord` command `{other}`; use a supported administrative command"
        )),
    }
}

fn volicord_command_start(tokens: &[String]) -> Option<usize> {
    let mut index = 0;
    if tokens.first().is_some_and(|token| token == "env") {
        index += 1;
    }
    while tokens
        .get(index)
        .is_some_and(|token| is_env_assignment(token))
    {
        index += 1;
    }
    tokens
        .get(index)
        .is_some_and(|token| is_volicord_binary(token))
        .then_some(index)
}

fn is_env_assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn is_volicord_binary(token: &str) -> bool {
    token == "volicord"
        || token == "$VOLICORD_BIN"
        || token == "${VOLICORD_BIN}"
        || token
            .rsplit('/')
            .next()
            .is_some_and(|name| name == "volicord")
}

fn validate_no_more_args(args: &[String], context: &str) -> std::result::Result<(), String> {
    if args.len() == 1 {
        Ok(())
    } else {
        Err(format!(
            "{context} does not accept trailing argument `{}`",
            args[1]
        ))
    }
}

fn validate_status_command(args: &[String]) -> std::result::Result<(), String> {
    if is_help_only(args) {
        return Ok(());
    }
    let parsed = parse_command_args(args, &["json"], &["repo", "task"])?;
    reject_positionals(&parsed, 0, "`volicord status`")
}

fn validate_doctor_command(args: &[String]) -> std::result::Result<(), String> {
    if is_help_only(args) {
        return Ok(());
    }
    let parsed = parse_command_args(args, &["json"], &[])?;
    reject_positionals(&parsed, 0, "`volicord doctor`")
}

fn validate_mcp_command(args: &[String]) -> std::result::Result<(), String> {
    if is_help_only(args)
        || matches!(args, [option] if matches!(option.as_str(), "-V" | "--version"))
    {
        return Ok(());
    }

    let parsed = parse_command_args(args, &["stdio", "check"], &["connection", "project"])?;
    reject_mutually_exclusive(&parsed, "stdio", "check")?;
    reject_positionals(&parsed, 0, "`volicord mcp`")?;

    let has_stdio = parsed.options.contains("stdio");
    let has_check = parsed.options.contains("check");
    if !has_stdio && !has_check {
        return Err("`volicord mcp` requires --stdio or --check".to_string());
    }
    if !parsed.options.contains("connection") {
        return Err("`volicord mcp` requires --connection".to_string());
    }
    Ok(())
}

fn validate_serve_command(args: &[String]) -> std::result::Result<(), String> {
    if is_help_only(args)
        || matches!(args, [option] if matches!(option.as_str(), "-V" | "--version"))
    {
        return Ok(());
    }

    let parsed = parse_command_args_with_repeatable_values(
        args,
        &["generate-token"],
        &[
            "transport",
            "listen",
            "home",
            "connection",
            "project",
            "token",
            "allow-origin",
        ],
        &["project", "allow-origin"],
    )?;
    reject_mutually_exclusive(&parsed, "token", "generate-token")?;
    reject_positionals(&parsed, 0, "`volicord serve`")?;
    if !parsed.options.contains("transport") {
        return Err("`volicord serve` requires --transport".to_string());
    }
    validate_serve_transport(&parsed)?;
    validate_serve_listen(&parsed)?;
    Ok(())
}

fn validate_init_command(args: &[String]) -> std::result::Result<(), String> {
    if is_help_only(args) {
        return Ok(());
    }
    let parsed = parse_command_args(
        args,
        &["shared", "dry-run", "json"],
        &["host", "repo", "profile", "home", "mcp-command"],
    )?;
    reject_positionals(&parsed, 0, "`volicord init`")?;
    if !parsed.options.contains("host") {
        return Err("`volicord init` requires --host".to_string());
    }
    if !parsed.options.contains("repo") {
        return Err("`volicord init` requires --repo".to_string());
    }
    validate_integration_profile_option(&parsed, "profile", "`volicord init`")?;
    Ok(())
}

fn validate_connection_command(args: &[String]) -> std::result::Result<(), String> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Ok(());
    };
    match subcommand {
        "-h" | "--help" | "help" => validate_no_more_args(args, "`volicord connection` help"),
        "add" => {
            if is_help_only(&args[1..]) {
                return Ok(());
            }
            let parsed = parse_command_args(
                &args[1..],
                &["shared", "global", "read-only", "dry-run", "json"],
                &["repo"],
            )?;
            reject_mutually_exclusive(&parsed, "shared", "global")?;
            validate_optional_host(&parsed, "`volicord connection add`")
        }
        "list" => {
            if is_help_only(&args[1..]) {
                return Ok(());
            }
            let parsed = parse_command_args(&args[1..], &["json"], &["repo"])?;
            reject_positionals(&parsed, 0, "`volicord connection list`")
        }
        "status" | "verify" => {
            if is_help_only(&args[1..]) {
                return Ok(());
            }
            let parsed = parse_command_args(&args[1..], &["shared", "global", "json"], &["repo"])?;
            reject_mutually_exclusive(&parsed, "shared", "global")?;
            validate_optional_host(&parsed, &format!("`volicord connection {subcommand}`"))
        }
        "mode" => {
            if is_help_only(&args[1..]) {
                return Ok(());
            }
            let parsed = parse_command_args(&args[1..], &["shared", "global", "json"], &["repo"])?;
            reject_mutually_exclusive(&parsed, "shared", "global")?;
            validate_connection_mode_positionals(&parsed)
        }
        "remove" => {
            if is_help_only(&args[1..]) {
                return Ok(());
            }
            let parsed = parse_command_args(
                &args[1..],
                &["shared", "global", "dry-run", "json"],
                &["repo"],
            )?;
            reject_mutually_exclusive(&parsed, "shared", "global")?;
            validate_optional_host(&parsed, "`volicord connection remove`")
        }
        other => Err(format!(
            "unknown `volicord connection` subcommand `{other}`; use add, list, status, verify, mode, or remove"
        )),
    }
}

fn validate_project_command(args: &[String]) -> std::result::Result<(), String> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Ok(());
    };
    match subcommand {
        "-h" | "--help" | "help" => validate_no_more_args(args, "`volicord project` help"),
        "use" => {
            if is_help_only(&args[1..]) {
                return Ok(());
            }
            let parsed = parse_command_args(&args[1..], &["json"], &[])?;
            reject_positionals(&parsed, 1, "`volicord project use`")
        }
        "current" | "list" => {
            if is_help_only(&args[1..]) {
                return Ok(());
            }
            let parsed = parse_command_args(&args[1..], &["json"], &[])?;
            reject_positionals(&parsed, 0, &format!("`volicord project {subcommand}`"))
        }
        "rename" => {
            if is_help_only(&args[1..]) {
                return Ok(());
            }
            let parsed = parse_command_args(&args[1..], &["json"], &["repo"])?;
            require_positionals(&parsed, 1, 1, "`volicord project rename`")
        }
        "forget" => {
            if is_help_only(&args[1..]) {
                return Ok(());
            }
            let parsed = parse_command_args(&args[1..], &["json"], &[])?;
            reject_positionals(&parsed, 1, "`volicord project forget`")
        }
        other => Err(format!(
            "unknown `volicord project` subcommand `{other}`; use use, current, list, rename, or forget"
        )),
    }
}

fn validate_inbox_command(args: &[String]) -> std::result::Result<(), String> {
    match args.first().map(String::as_str) {
        None => Ok(()),
        Some("-h" | "--help" | "help") => validate_no_more_args(args, "`volicord inbox` help"),
        Some("answer") => {
            if is_help_only(&args[1..]) {
                return Ok(());
            }
            let parsed = parse_command_args(&args[1..], &["json"], &["repo", "note", "choice"])?;
            require_positionals(&parsed, 1, 1, "`volicord inbox answer`")?;
            if !parsed.options.contains("choice") {
                return Err("`volicord inbox answer` requires --choice".to_string());
            }
            Ok(())
        }
        Some("open") => {
            if is_help_only(&args[1..]) {
                return Ok(());
            }
            let parsed = parse_command_args(&args[1..], &["json"], &["repo"])?;
            require_positionals(&parsed, 1, 1, "`volicord inbox open`")
        }
        Some(token) if token.starts_with('-') => {
            let parsed = parse_command_args(args, &["json"], &["repo", "task"])?;
            reject_positionals(&parsed, 0, "`volicord inbox`")
        }
        Some(other) => Err(format!(
            "unknown `volicord inbox` subcommand `{other}`; use answer or open"
        )),
    }
}

fn validate_public_language_claims(root: &Path, errors: &mut Vec<ValidationError>) {
    if !PUBLIC_LANGUAGE_SOURCE_ROOTS
        .iter()
        .any(|relative| root.join(relative).exists())
    {
        return;
    }

    let mut sources = BTreeSet::new();
    for relative in PUBLIC_LANGUAGE_SOURCE_ROOTS {
        collect_public_language_source_root(root, relative, &mut sources, errors);
    }

    for relative in sources {
        let path = root.join(&relative);
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationError::new(
                    relative.clone(),
                    "public_language.read",
                    format!("failed to read public output source: {error}"),
                ));
                continue;
            }
        };
        for (index, line) in contents.lines().enumerate() {
            for word in PUBLIC_UNQUALIFIED_SECURITY_WORDS {
                if contains_ascii_word_ignore_case(line, word) {
                    errors.push(ValidationError::new(
                        relative.clone(),
                        "public_language.security_claim",
                        format!(
                            "line {} uses unqualified `{word}` in public output source; use explicit guarantee disclosure wording",
                            index + 1
                        ),
                    ));
                }
            }
        }
    }
}

fn collect_public_language_source_root(
    root: &Path,
    relative: &str,
    sources: &mut BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    let path = root.join(relative);
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) => {
            errors.push(ValidationError::new(
                relative,
                "public_language.read",
                format!("failed to inspect public output source: {error}"),
            ));
            return;
        }
    };

    if metadata.is_dir() {
        collect_public_language_source_dir(root, &path, sources, errors);
    } else if metadata.is_file() && is_rust_source_path(&path) {
        sources.insert(relative.to_string());
    }
}

fn collect_public_language_source_dir(
    root: &Path,
    dir: &Path,
    sources: &mut BTreeSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    let mut entries = Vec::new();
    let read_dir = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(error) => {
            errors.push(ValidationError::new(
                repo_relative(root, dir),
                "public_language.read",
                format!("failed to read public output source directory: {error}"),
            ));
            return;
        }
    };

    for entry in read_dir {
        match entry {
            Ok(entry) => entries.push(entry),
            Err(error) => {
                errors.push(ValidationError::new(
                    repo_relative(root, dir),
                    "public_language.read",
                    format!("failed to read public output source directory entry: {error}"),
                ));
            }
        }
    }
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                errors.push(ValidationError::new(
                    repo_relative(root, &path),
                    "public_language.read",
                    format!("failed to inspect public output source: {error}"),
                ));
                continue;
            }
        };

        if file_type.is_dir() {
            collect_public_language_source_dir(root, &path, sources, errors);
        } else if file_type.is_file() && is_rust_source_path(&path) {
            sources.insert(repo_relative(root, &path));
        }
    }
}

fn is_rust_source_path(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("rs")
}

fn validate_public_document_language(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationError>,
) {
    for path in index
        .indexed_paths
        .iter()
        .filter(|path| path.ends_with(".md"))
    {
        let contents = match fs::read_to_string(root.join(path)) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationError::new(
                    path,
                    "public_language.read",
                    format!("failed to read Markdown file: {error}"),
                ));
                continue;
            }
        };

        let mut active_fence = None;
        for (index, line) in contents.lines().enumerate() {
            if let Some(fence) = active_fence.as_ref() {
                if is_closing_fence(line, fence) {
                    active_fence = None;
                }
                continue;
            }
            if let Some(fence) = opening_fence(line) {
                active_fence = Some(fence);
                continue;
            }

            let lower = line.to_ascii_lowercase();
            for disallowed in PUBLIC_DOCUMENT_DISALLOWED_TERMS {
                if lower.contains(disallowed.term) {
                    errors.push(ValidationError::new(
                        path,
                        "public_language.write_ticket_term",
                        format!(
                            "line {} uses `{}` in public documentation; use `{}` terminology for the write-ticket concept",
                            index + 1,
                            disallowed.term,
                            disallowed.replacement
                        ),
                    ));
                }
            }
        }
    }
}

fn contains_ascii_word_ignore_case(line: &str, word: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let word = word.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let needle = word.as_bytes();
    if needle.is_empty() || bytes.len() < needle.len() {
        return false;
    }
    for start in 0..=bytes.len() - needle.len() {
        if &bytes[start..start + needle.len()] != needle {
            continue;
        }
        let before = start
            .checked_sub(1)
            .and_then(|index| bytes.get(index))
            .copied();
        let after = bytes.get(start + needle.len()).copied();
        if !is_ascii_word_byte(before) && !is_ascii_word_byte(after) {
            return true;
        }
    }
    false
}

fn is_ascii_word_byte(byte: Option<u8>) -> bool {
    matches!(byte, Some(b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

fn is_help_only(args: &[String]) -> bool {
    matches!(
        args,
        [argument] if matches!(argument.as_str(), "-h" | "--help" | "help")
    )
}

fn parse_command_args(
    args: &[String],
    flags: &[&str],
    values: &[&str],
) -> std::result::Result<ParsedCommandArgs, String> {
    parse_command_args_with_repeatable_values(args, flags, values, &[])
}

fn parse_command_args_with_repeatable_values(
    args: &[String],
    flags: &[&str],
    values: &[&str],
    repeatable_values: &[&str],
) -> std::result::Result<ParsedCommandArgs, String> {
    let mut parsed = ParsedCommandArgs::default();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if !token.starts_with("--") {
            parsed.positionals.push(token.clone());
            index += 1;
            continue;
        }

        let option = token.trim_start_matches("--");
        let (name, inline_value) = match option.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (option, None),
        };
        if flags.contains(&name) {
            if inline_value.is_some() {
                return Err(format!("option `--{name}` does not accept a value"));
            }
            insert_option(&mut parsed, name)?;
        } else if values.contains(&name) {
            let value = if let Some(value) = inline_value {
                value.to_owned()
            } else {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(format!("missing value for option `--{name}`"));
                };
                value.clone()
            };
            reject_empty_option_value(name, &value)?;
            insert_value_option(&mut parsed, name, value, repeatable_values.contains(&name))?;
        } else {
            return Err(format!(
                "option `--{name}` is not supported in this command context"
            ));
        }
        index += 1;
    }
    Ok(parsed)
}

fn insert_option(parsed: &mut ParsedCommandArgs, name: &str) -> std::result::Result<(), String> {
    if !parsed.options.insert(name.to_string()) {
        return Err(format!("duplicate option `--{name}`"));
    }
    Ok(())
}

fn insert_value_option(
    parsed: &mut ParsedCommandArgs,
    name: &str,
    value: String,
    repeatable: bool,
) -> std::result::Result<(), String> {
    if !repeatable && parsed.options.contains(name) {
        return Err(format!("duplicate option `--{name}`"));
    }
    parsed.options.insert(name.to_string());
    parsed
        .value_options
        .entry(name.to_string())
        .or_default()
        .push(value);
    Ok(())
}

fn reject_empty_option_value(name: &str, value: &str) -> std::result::Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("option `--{name}` requires a non-empty value"))
    } else {
        Ok(())
    }
}

fn reject_mutually_exclusive(
    parsed: &ParsedCommandArgs,
    left: &str,
    right: &str,
) -> std::result::Result<(), String> {
    if parsed.options.contains(left) && parsed.options.contains(right) {
        Err(format!(
            "options `--{left}` and `--{right}` are mutually exclusive"
        ))
    } else {
        Ok(())
    }
}

fn validate_serve_transport(parsed: &ParsedCommandArgs) -> std::result::Result<(), String> {
    let transport = parsed
        .value_options
        .get("transport")
        .and_then(|values| values.first())
        .expect("transport presence already checked");
    if transport == "local-http" {
        Ok(())
    } else {
        Err(format!(
            "unsupported `volicord serve --transport {transport}`; use `local-http`"
        ))
    }
}

fn validate_serve_listen(parsed: &ParsedCommandArgs) -> std::result::Result<(), String> {
    for listen in parsed.value_options.get("listen").into_iter().flatten() {
        let addr = listen
            .parse::<SocketAddr>()
            .map_err(|error| format!("`volicord serve --listen` must be host:port: {error}"))?;
        if !serve_listen_is_supported_loopback(&addr) {
            return Err(format!(
                "`volicord serve --listen {listen}` is not supported; use 127.0.0.1 or [::1]"
            ));
        }
    }
    Ok(())
}

fn serve_listen_is_supported_loopback(addr: &SocketAddr) -> bool {
    matches!(addr.ip(), IpAddr::V4(address) if address == Ipv4Addr::LOCALHOST)
        || matches!(addr.ip(), IpAddr::V6(address) if address == Ipv6Addr::LOCALHOST)
}

fn reject_positionals(
    parsed: &ParsedCommandArgs,
    max: usize,
    context: &str,
) -> std::result::Result<(), String> {
    require_positionals(parsed, 0, max, context)
}

fn require_positionals(
    parsed: &ParsedCommandArgs,
    min: usize,
    max: usize,
    context: &str,
) -> std::result::Result<(), String> {
    if parsed.positionals.len() < min {
        return Err(format!("{context} requires {min} positional argument(s)"));
    }
    if parsed.positionals.len() > max {
        return Err(format!(
            "{context} accepts at most {max} positional argument(s), found `{}`",
            parsed.positionals[max]
        ));
    }
    Ok(())
}

fn validate_optional_host(
    parsed: &ParsedCommandArgs,
    context: &str,
) -> std::result::Result<(), String> {
    reject_positionals(parsed, 1, context)?;
    if let Some(host) = parsed.positionals.first() {
        validate_host(host)?;
    }
    Ok(())
}

fn validate_connection_mode_positionals(
    parsed: &ParsedCommandArgs,
) -> std::result::Result<(), String> {
    match parsed.positionals.as_slice() {
        [mode] => validate_connection_mode(mode),
        [host, mode] => {
            validate_host(host)?;
            validate_connection_mode(mode)
        }
        [] => Err("`volicord connection mode` requires workflow or read-only mode".to_string()),
        _ => Err("`volicord connection mode` accepts optional HOST and required mode".to_string()),
    }
}

fn validate_host(host: &str) -> std::result::Result<(), String> {
    if matches!(host, "codex" | "claude-code" | "claude_code") {
        Ok(())
    } else {
        Err(format!(
            "unsupported host `{host}`; use `codex` or `claude-code`"
        ))
    }
}

fn validate_integration_profile_option(
    parsed: &ParsedCommandArgs,
    option: &str,
    context: &str,
) -> std::result::Result<(), String> {
    for value in parsed.value_options.get(option).into_iter().flatten() {
        if !matches!(value.as_str(), "record" | "detective") {
            return Err(format!(
                "{context} option `--{option}` value `{value}` is unsupported; use `record` or `detective`"
            ));
        }
    }
    Ok(())
}

fn validate_connection_mode(mode: &str) -> std::result::Result<(), String> {
    if matches!(mode, "workflow" | "read-only") {
        Ok(())
    } else {
        Err(format!(
            "unsupported connection mode `{mode}`; use `workflow` or `read-only`"
        ))
    }
}

fn path_mentions_in_text(text: &str) -> Vec<String> {
    let prefixes = ["docs/", "AGENTS.md", "README.md", "crates/AGENTS.md"];
    let mut mentions = Vec::new();
    for prefix in prefixes {
        let mut start = 0;
        while let Some(offset) = text[start..].find(prefix) {
            let mention_start = start + offset;
            let mut mention_end = mention_start;
            for (char_offset, character) in text[mention_start..].char_indices() {
                if char_offset == 0 {
                    mention_end = mention_start + character.len_utf8();
                    continue;
                }
                if character.is_whitespace()
                    || matches!(
                        character,
                        ')' | ']' | '}' | '>' | '"' | '\'' | '`' | ',' | ';'
                    )
                {
                    break;
                }
                mention_end = mention_start + char_offset + character.len_utf8();
            }
            let mention = text[mention_start..mention_end]
                .trim_matches(|character: char| {
                    matches!(
                        character,
                        '.' | ':' | ')' | ']' | '}' | '>' | '"' | '\'' | '`'
                    )
                })
                .to_string();
            if !mention.is_empty() {
                mentions.push(mention);
            }
            start = mention_end;
        }
    }
    mentions
}

fn is_repository_document_path(path: &str) -> bool {
    path == "AGENTS.md"
        || path == "README.md"
        || path == "docs/AGENTS.md"
        || path == "crates/AGENTS.md"
        || path.starts_with("docs/")
}

fn is_ignored_link(link: &str) -> bool {
    let trimmed = link.trim();
    trimmed.is_empty() || has_uri_scheme(trimmed)
}

fn has_uri_scheme(link: &str) -> bool {
    let Some(colon_index) = link.find(':') else {
        return false;
    };
    let scheme = &link[..colon_index];
    !scheme.is_empty()
        && scheme.chars().enumerate().all(|(index, character)| {
            if index == 0 {
                character.is_ascii_alphabetic()
            } else {
                character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
            }
        })
}

fn mapping_get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_string()))
}

fn repo_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(path_to_slash)
        .unwrap_or_else(|_| path_to_slash(path))
}

fn path_to_slash(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            Component::CurDir => None,
            Component::ParentDir => Some("..".to_string()),
            Component::RootDir | Component::Prefix(_) => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
        }
    }
    normalized
}

fn percent_decode(value: &str) -> std::result::Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("truncated percent escape".to_string());
            }
            let high =
                hex_value(bytes[index + 1]).ok_or_else(|| "invalid percent escape".to_string())?;
            let low =
                hex_value(bytes[index + 2]).ok_or_else(|| "invalid percent escape".to_string())?;
            decoded.push(high << 4 | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).map_err(|error| error.to_string())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
