use crate::diagnostics::ValidationIssue;
use crate::repository::{normalize_existing_root, repo_relative};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAINTAINABILITY_TOP_N: usize = 10;
const MAINTAINABILITY_SKIP_DIRS: &[&str] = &[".git", "target"];
const ARCHITECTURE_OWNER_PATH: &str = "Cargo.toml";
const API_METHODS_PATH: &str = "docs/en/reference/api/methods.md";
const ADMIN_CLI_REFERENCE_PATH: &str = "docs/en/reference/admin-cli.md";
const CORE_METHOD_TEST_HINT_DIR: &str = "crates/volicord-core/src/methods/tests";
const CLI_BINARY_TEST_HINT_DIR: &str = "crates/volicord-cli/tests";
const CLI_MAIN_PATH: &str = "crates/volicord-cli/src/main.rs";

#[derive(Debug, Deserialize)]
struct RootArchitectureManifest {
    workspace: WorkspaceArchitectureMetadata,
}

#[derive(Debug, Deserialize)]
struct WorkspaceArchitectureMetadata {
    metadata: ArchitectureMetadata,
}

#[derive(Debug, Deserialize)]
struct ArchitectureMetadata {
    architecture: ArchitectureOwner,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchitectureOwner {
    packages: BTreeMap<String, ArchitecturePackageDeclaration>,
    groups: BTreeMap<String, ArchitectureGroupDeclaration>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchitecturePackageDeclaration {
    group: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchitectureGroupDeclaration {
    description: String,
    kind: ArchitectureGroupKind,
    boundary: ArchitectureBoundaryKind,
    normal: Vec<String>,
    development: Vec<String>,
    build: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum ArchitectureGroupKind {
    Production,
    TestSupport,
    TestSuite,
    RepositoryTool,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum ArchitectureBoundaryKind {
    CoreFacing,
    Adapter,
    Neutral,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum ArchitectureDependencyKind {
    Normal,
    Development,
    Build,
}

impl ArchitectureDependencyKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Development => "development",
            Self::Build => "build",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ArchitectureDependency {
    package: String,
    kind: ArchitectureDependencyKind,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ArchitecturePackage {
    manifest_path: String,
    dependencies: Vec<ArchitectureDependency>,
}

type ArchitectureGraph = BTreeMap<String, ArchitecturePackage>;

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CargoMetadataPackage {
    id: String,
    name: String,
    manifest_path: PathBuf,
    dependencies: Vec<CargoMetadataDependency>,
    targets: Vec<CargoMetadataTarget>,
}

#[derive(Debug, Deserialize)]
struct CargoMetadataDependency {
    kind: Option<String>,
    path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct CargoMetadataTarget {
    src_path: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WorkspacePackageInput {
    name: String,
    manifest_path: String,
    source_roots: Vec<String>,
    target_source_paths: Vec<String>,
}

impl WorkspacePackageInput {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn manifest_path(&self) -> &str {
        &self.manifest_path
    }

    pub fn source_roots(&self) -> &[String] {
        &self.source_roots
    }

    pub fn target_source_paths(&self) -> &[String] {
        &self.target_source_paths
    }
}

pub fn run_architecture_check(root: &Path) -> Result<crate::diagnostics::CheckReport> {
    let root = normalize_existing_root(root)?;
    let owner_path = root.join(ARCHITECTURE_OWNER_PATH);
    if !owner_path.exists() {
        anyhow::bail!(
            "architecture-check must run from the repository root; missing {ARCHITECTURE_OWNER_PATH}"
        );
    }

    let owner = read_architecture_owner(&owner_path)?;
    let graph = read_workspace_graph(&root)?;
    let mut issues = validate_architecture_graph(&owner, &graph);
    issues.sort();
    issues.dedup();

    Ok(crate::diagnostics::CheckReport { issues })
}

pub fn derive_workspace_package_inputs(root: &Path) -> Result<Vec<WorkspacePackageInput>> {
    let root = normalize_existing_root(root)?;
    let metadata = read_cargo_metadata(&root)?;
    workspace_package_inputs_from_metadata(&root, &metadata)
}

fn read_architecture_owner(path: &Path) -> Result<ArchitectureOwner> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read architecture owner {}", path.display()))?;
    let manifest =
        toml_edit::de::from_str::<RootArchitectureManifest>(&contents).with_context(|| {
            format!(
                "failed to parse workspace.metadata.architecture from {}",
                path.display()
            )
        })?;
    Ok(manifest.workspace.metadata.architecture)
}

fn read_workspace_graph(root: &Path) -> Result<ArchitectureGraph> {
    workspace_graph_from_metadata(root, read_cargo_metadata(root)?)
}

fn read_cargo_metadata(root: &Path) -> Result<CargoMetadata> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let output = Command::new(cargo)
        .current_dir(root)
        .args([
            "metadata",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
            "Cargo.toml",
        ])
        .output()
        .context("failed to execute cargo metadata for architecture-check")?;
    if !output.status.success() {
        anyhow::bail!(
            "cargo metadata failed for architecture-check: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let metadata = serde_json::from_slice::<CargoMetadata>(&output.stdout)
        .context("failed to decode cargo metadata for architecture-check")?;
    Ok(metadata)
}

fn workspace_package_inputs_from_metadata(
    root: &Path,
    metadata: &CargoMetadata,
) -> Result<Vec<WorkspacePackageInput>> {
    let workspace_members = metadata.workspace_members.iter().collect::<BTreeSet<_>>();
    let mut inputs = Vec::new();

    for package in metadata
        .packages
        .iter()
        .filter(|package| workspace_members.contains(&package.id))
    {
        let mut target_source_paths = package
            .targets
            .iter()
            .map(|target| repo_relative(root, &target.src_path))
            .collect::<Vec<_>>();
        target_source_paths.sort();
        target_source_paths.dedup();

        let mut source_roots = package
            .targets
            .iter()
            .filter_map(|target| target.src_path.parent())
            .map(|path| repo_relative(root, path))
            .collect::<Vec<_>>();
        source_roots.sort();
        source_roots.dedup();

        inputs.push(WorkspacePackageInput {
            name: package.name.clone(),
            manifest_path: repo_relative(root, &package.manifest_path),
            source_roots,
            target_source_paths,
        });
    }
    inputs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(inputs)
}

fn workspace_graph_from_metadata(
    root: &Path,
    metadata: CargoMetadata,
) -> Result<ArchitectureGraph> {
    let workspace_members = metadata
        .workspace_members
        .into_iter()
        .collect::<BTreeSet<_>>();
    let packages = metadata
        .packages
        .into_iter()
        .filter(|package| workspace_members.contains(&package.id))
        .collect::<Vec<_>>();
    let mut member_directories = BTreeMap::new();

    for package in &packages {
        let directory = package
            .manifest_path
            .parent()
            .context("workspace package manifest has no parent directory")?
            .to_path_buf();
        if let Some(existing) = member_directories.insert(directory, package.name.clone()) {
            anyhow::bail!(
                "workspace packages {existing} and {} share one manifest directory",
                package.name
            );
        }
    }

    let mut graph = BTreeMap::new();
    for package in packages {
        let mut dependencies = Vec::new();
        for dependency in package.dependencies {
            let Some(path) = dependency.path else {
                continue;
            };
            let Some(target) = member_directories.get(&path) else {
                continue;
            };
            let kind = match dependency.kind.as_deref() {
                None => ArchitectureDependencyKind::Normal,
                Some("dev") => ArchitectureDependencyKind::Development,
                Some("build") => ArchitectureDependencyKind::Build,
                Some(kind) => anyhow::bail!(
                    "cargo metadata reported unsupported dependency kind {kind:?} for {} -> {target}",
                    package.name
                ),
            };
            dependencies.push(ArchitectureDependency {
                package: target.clone(),
                kind,
            });
        }
        dependencies
            .sort_by(|left, right| (&left.package, left.kind).cmp(&(&right.package, right.kind)));
        dependencies.dedup();
        let manifest_path = repo_relative(root, &package.manifest_path);
        if graph
            .insert(
                package.name.clone(),
                ArchitecturePackage {
                    manifest_path,
                    dependencies,
                },
            )
            .is_some()
        {
            anyhow::bail!(
                "cargo metadata reported duplicate workspace package name {}",
                package.name
            );
        }
    }
    Ok(graph)
}

fn validate_architecture_graph(
    owner: &ArchitectureOwner,
    graph: &ArchitectureGraph,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    validate_architecture_owner(owner, &mut issues);

    for (package, actual) in graph {
        if !owner.packages.contains_key(package) {
            issues.push(ValidationIssue::new(
                &actual.manifest_path,
                "architecture.package.undeclared",
                format!(
                    "workspace package {package} is not declared in workspace.metadata.architecture.packages"
                ),
            ));
        }
    }
    for package in owner.packages.keys() {
        if !graph.contains_key(package) {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.package.missing",
                format!(
                    "architecture owner declares package {package}, but Cargo metadata does not report it as a workspace member"
                ),
            ));
        }
    }

    for (source_name, source) in graph {
        let Some(source_declaration) = owner.packages.get(source_name) else {
            continue;
        };
        let Some(source_group) = owner.groups.get(&source_declaration.group) else {
            continue;
        };

        for dependency in &source.dependencies {
            let Some(target_declaration) = owner.packages.get(&dependency.package) else {
                issues.push(ValidationIssue::new(
                    &source.manifest_path,
                    "architecture.dependency.undeclared_target",
                    format!(
                        "{} dependency {source_name} -> {} targets an undeclared internal package",
                        dependency.kind.label(),
                        dependency.package
                    ),
                ));
                continue;
            };
            let Some(target_group) = owner.groups.get(&target_declaration.group) else {
                continue;
            };
            let allowed_groups = allowed_groups(source_group, dependency.kind);

            if !allowed_groups
                .iter()
                .any(|group| group == &target_declaration.group)
            {
                let allowed = if allowed_groups.is_empty() {
                    "none".to_owned()
                } else {
                    allowed_groups.join(", ")
                };
                issues.push(ValidationIssue::new(
                    &source.manifest_path,
                    "architecture.dependency.disallowed",
                    format!(
                        "{} dependency {source_name} ({}) -> {} ({}) is not allowed; permitted target groups: {allowed}",
                        dependency.kind.label(),
                        source_declaration.group,
                        dependency.package,
                        target_declaration.group
                    ),
                ));
            }

            if source_group.kind == ArchitectureGroupKind::Production
                && target_group.kind == ArchitectureGroupKind::TestSupport
                && dependency.kind != ArchitectureDependencyKind::Development
            {
                issues.push(ValidationIssue::new(
                    &source.manifest_path,
                    "architecture.dependency.production_test_support",
                    format!(
                        "production package {source_name} has a {} dependency on test-support package {}; test-support is permitted only as a development dependency",
                        dependency.kind.label(),
                        dependency.package
                    ),
                ));
            }

            if source_group.boundary == ArchitectureBoundaryKind::CoreFacing
                && target_group.boundary == ArchitectureBoundaryKind::Adapter
            {
                issues.push(ValidationIssue::new(
                    &source.manifest_path,
                    "architecture.dependency.core_adapter",
                    format!(
                        "Core-facing package {source_name} has a {} dependency on adapter package {}; Core-facing packages must remain adapter-independent",
                        dependency.kind.label(),
                        dependency.package
                    ),
                ));
            }
        }
    }

    issues
}

fn validate_architecture_owner(owner: &ArchitectureOwner, issues: &mut Vec<ValidationIssue>) {
    for (package, declaration) in &owner.packages {
        if !owner.groups.contains_key(&declaration.group) {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.owner.package_group",
                format!(
                    "package {package} references undeclared architecture group {}",
                    declaration.group
                ),
            ));
        }
    }

    for (group_name, group) in &owner.groups {
        if group.description.trim().is_empty() {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.owner.group_description",
                format!("architecture group {group_name} has an empty description"),
            ));
        }
        for (kind, targets) in [
            (ArchitectureDependencyKind::Normal, &group.normal),
            (ArchitectureDependencyKind::Development, &group.development),
            (ArchitectureDependencyKind::Build, &group.build),
        ] {
            let mut seen = BTreeSet::new();
            for target in targets {
                if !seen.insert(target) {
                    issues.push(ValidationIssue::new(
                        ARCHITECTURE_OWNER_PATH,
                        "architecture.owner.duplicate_direction",
                        format!(
                            "architecture group {group_name} repeats {target} in its {} dependency directions",
                            kind.label()
                        ),
                    ));
                }
                if !owner.groups.contains_key(target) {
                    issues.push(ValidationIssue::new(
                        ARCHITECTURE_OWNER_PATH,
                        "architecture.owner.unknown_direction",
                        format!(
                            "architecture group {group_name} allows an undeclared {target} target for {} dependencies",
                            kind.label()
                        ),
                    ));
                }
            }
        }
    }
}

fn allowed_groups(
    group: &ArchitectureGroupDeclaration,
    kind: ArchitectureDependencyKind,
) -> &[String] {
    match kind {
        ArchitectureDependencyKind::Normal => &group.normal,
        ArchitectureDependencyKind::Development => &group.development,
        ArchitectureDependencyKind::Build => &group.build,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn declaration(group: &str) -> ArchitecturePackageDeclaration {
        ArchitecturePackageDeclaration {
            group: group.to_owned(),
        }
    }

    fn group(
        kind: ArchitectureGroupKind,
        boundary: ArchitectureBoundaryKind,
        normal: &[&str],
        development: &[&str],
        build: &[&str],
    ) -> ArchitectureGroupDeclaration {
        ArchitectureGroupDeclaration {
            description: "Synthetic responsibility group.".to_owned(),
            kind,
            boundary,
            normal: normal.iter().map(|value| (*value).to_owned()).collect(),
            development: development
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            build: build.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn package(dependencies: &[(&str, ArchitectureDependencyKind)]) -> ArchitecturePackage {
        ArchitecturePackage {
            manifest_path: "components/synthetic/Cargo.toml".to_owned(),
            dependencies: dependencies
                .iter()
                .map(|(package, kind)| ArchitectureDependency {
                    package: (*package).to_owned(),
                    kind: *kind,
                })
                .collect(),
        }
    }

    #[test]
    fn synthetic_graph_accepts_kind_specific_directions() {
        let owner = ArchitectureOwner {
            packages: BTreeMap::from([
                ("engine".to_owned(), declaration("application")),
                ("foundation".to_owned(), declaration("foundation")),
                ("fixture-kit".to_owned(), declaration("fixtures")),
                ("code-generator".to_owned(), declaration("build-tooling")),
            ]),
            groups: BTreeMap::from([
                (
                    "application".to_owned(),
                    group(
                        ArchitectureGroupKind::Production,
                        ArchitectureBoundaryKind::Neutral,
                        &["foundation"],
                        &["fixtures"],
                        &["build-tooling"],
                    ),
                ),
                (
                    "foundation".to_owned(),
                    group(
                        ArchitectureGroupKind::Production,
                        ArchitectureBoundaryKind::Neutral,
                        &[],
                        &[],
                        &[],
                    ),
                ),
                (
                    "fixtures".to_owned(),
                    group(
                        ArchitectureGroupKind::TestSupport,
                        ArchitectureBoundaryKind::Neutral,
                        &[],
                        &[],
                        &[],
                    ),
                ),
                (
                    "build-tooling".to_owned(),
                    group(
                        ArchitectureGroupKind::RepositoryTool,
                        ArchitectureBoundaryKind::Neutral,
                        &[],
                        &[],
                        &[],
                    ),
                ),
            ]),
        };
        let graph = BTreeMap::from([
            (
                "engine".to_owned(),
                package(&[
                    ("foundation", ArchitectureDependencyKind::Normal),
                    ("fixture-kit", ArchitectureDependencyKind::Development),
                    ("code-generator", ArchitectureDependencyKind::Build),
                ]),
            ),
            ("foundation".to_owned(), package(&[])),
            ("fixture-kit".to_owned(), package(&[])),
            ("code-generator".to_owned(), package(&[])),
        ]);

        assert!(validate_architecture_graph(&owner, &graph).is_empty());
    }

    #[test]
    fn synthetic_graph_rejects_undeclared_packages_and_disallowed_kinds() {
        let owner = ArchitectureOwner {
            packages: BTreeMap::from([
                ("engine".to_owned(), declaration("application")),
                ("foundation".to_owned(), declaration("foundation")),
            ]),
            groups: BTreeMap::from([
                (
                    "application".to_owned(),
                    group(
                        ArchitectureGroupKind::Production,
                        ArchitectureBoundaryKind::Neutral,
                        &["foundation"],
                        &[],
                        &[],
                    ),
                ),
                (
                    "foundation".to_owned(),
                    group(
                        ArchitectureGroupKind::Production,
                        ArchitectureBoundaryKind::Neutral,
                        &[],
                        &[],
                        &[],
                    ),
                ),
            ]),
        };
        let graph = BTreeMap::from([
            (
                "engine".to_owned(),
                package(&[("foundation", ArchitectureDependencyKind::Build)]),
            ),
            ("foundation".to_owned(), package(&[])),
            ("unexpected-tool".to_owned(), package(&[])),
        ]);

        let issues = validate_architecture_graph(&owner, &graph);

        assert!(issues.iter().any(|issue| {
            issue.category() == "architecture.package.undeclared"
                && issue.message().contains("unexpected-tool")
        }));
        assert!(issues.iter().any(|issue| {
            issue.category() == "architecture.dependency.disallowed"
                && issue.message().contains("build dependency")
        }));
    }

    #[test]
    fn synthetic_graph_rejects_production_and_core_boundary_violations() {
        let owner = ArchitectureOwner {
            packages: BTreeMap::from([
                ("engine".to_owned(), declaration("core-services")),
                ("terminal".to_owned(), declaration("adapter")),
                ("fixture-kit".to_owned(), declaration("fixtures")),
            ]),
            groups: BTreeMap::from([
                (
                    "core-services".to_owned(),
                    group(
                        ArchitectureGroupKind::Production,
                        ArchitectureBoundaryKind::CoreFacing,
                        &["adapter", "fixtures"],
                        &[],
                        &[],
                    ),
                ),
                (
                    "adapter".to_owned(),
                    group(
                        ArchitectureGroupKind::Production,
                        ArchitectureBoundaryKind::Adapter,
                        &[],
                        &[],
                        &[],
                    ),
                ),
                (
                    "fixtures".to_owned(),
                    group(
                        ArchitectureGroupKind::TestSupport,
                        ArchitectureBoundaryKind::Neutral,
                        &[],
                        &[],
                        &[],
                    ),
                ),
            ]),
        };
        let graph = BTreeMap::from([
            (
                "engine".to_owned(),
                package(&[
                    ("terminal", ArchitectureDependencyKind::Normal),
                    ("fixture-kit", ArchitectureDependencyKind::Normal),
                ]),
            ),
            ("terminal".to_owned(), package(&[])),
            ("fixture-kit".to_owned(), package(&[])),
        ]);

        let issues = validate_architecture_graph(&owner, &graph);

        assert!(issues
            .iter()
            .any(|issue| issue.category() == "architecture.dependency.core_adapter"));
        assert!(issues.iter().any(|issue| {
            issue.category() == "architecture.dependency.production_test_support"
        }));
    }
}
