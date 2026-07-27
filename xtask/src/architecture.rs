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
const ARCHITECTURE_DOC_ID: &str = "architecture-guide.architecture";
const ARCHITECTURE_BEGIN_MARKER: &str = "<!-- BEGIN GENERATED: workspace-package-architecture -->";
const ARCHITECTURE_END_MARKER: &str = "<!-- END GENERATED: workspace-package-architecture -->";
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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchitecturePackageDeclaration {
    group: String,
    description: String,
    description_ko: String,
    kind: ArchitecturePackageKind,
    production: bool,
    boundary: ArchitectureBoundaryKind,
    normal: Vec<String>,
    development: Vec<String>,
    build: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum ArchitecturePackageKind {
    Adapter,
    Application,
    Infrastructure,
    Presentation,
    Schema,
    TestSupport,
    Validation,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum ArchitectureBoundaryKind {
    Core,
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
            let allowed_packages = allowed_packages(source_declaration, dependency.kind);

            if !allowed_packages
                .iter()
                .any(|package| package == &dependency.package)
            {
                let allowed = if allowed_packages.is_empty() {
                    "none".to_owned()
                } else {
                    allowed_packages.join(", ")
                };
                issues.push(ValidationIssue::new(
                    &source.manifest_path,
                    "architecture.dependency.disallowed",
                    format!(
                        "{} dependency {source_name} ({}) -> {} ({}) is not allowed; permitted target packages: {allowed}",
                        dependency.kind.label(),
                        source_declaration.group,
                        dependency.package,
                        target_declaration.group
                    ),
                ));
            }

            if source_declaration.production
                && target_declaration.kind == ArchitecturePackageKind::TestSupport
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

            if matches!(
                source_declaration.boundary,
                ArchitectureBoundaryKind::Core | ArchitectureBoundaryKind::CoreFacing
            ) && target_declaration.boundary == ArchitectureBoundaryKind::Adapter
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

            validate_required_boundary(
                source_name,
                source_declaration,
                &dependency.package,
                target_declaration,
                dependency.kind,
                &source.manifest_path,
                "architecture.dependency",
                &mut issues,
            );
        }
    }

    validate_deployable_cycles(graph, "architecture.dependency.cycle", &mut issues);
    issues
}

fn validate_architecture_owner(owner: &ArchitectureOwner, issues: &mut Vec<ValidationIssue>) {
    let mut group_owners = BTreeMap::new();
    for (package, declaration) in &owner.packages {
        if declaration.group.trim().is_empty()
            || !declaration
                .group
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '-')
        {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.owner.group",
                format!(
                    "package {package} has invalid semantic responsibility group {}; use a non-versioned lowercase kebab-case identifier",
                    declaration.group
                ),
            ));
        }
        if let Some(existing) = group_owners.insert(declaration.group.clone(), package) {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.owner.duplicate_group",
                format!(
                    "semantic responsibility group {} is assigned to both {existing} and {package}",
                    declaration.group
                ),
            ));
        }
        if declaration.description.trim().is_empty() {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.owner.description",
                format!("package {package} has an empty English responsibility description"),
            ));
        }
        if declaration.description_ko.trim().is_empty() {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.owner.description",
                format!("package {package} has an empty Korean responsibility description"),
            ));
        }
        if declaration.production
            && matches!(
                declaration.kind,
                ArchitecturePackageKind::TestSupport | ArchitecturePackageKind::Validation
            )
        {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.owner.classification",
                format!(
                    "package {package} is production but classified as {:?}",
                    declaration.kind
                ),
            ));
        }
        if !declaration.production
            && !matches!(
                declaration.kind,
                ArchitecturePackageKind::TestSupport | ArchitecturePackageKind::Validation
            )
        {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.owner.classification",
                format!(
                    "package {package} is non-production but classified as {:?}",
                    declaration.kind
                ),
            ));
        }
        if declaration.boundary == ArchitectureBoundaryKind::Core
            && (!declaration.production || declaration.kind != ArchitecturePackageKind::Application)
        {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.owner.core_boundary",
                format!(
                    "package {package} marks the Core boundary without production application ownership"
                ),
            ));
        }
        if declaration.boundary == ArchitectureBoundaryKind::Adapter
            && !matches!(
                declaration.kind,
                ArchitecturePackageKind::Adapter | ArchitecturePackageKind::Presentation
            )
        {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.owner.adapter_boundary",
                format!(
                    "package {package} marks the adapter boundary but is classified as {:?}",
                    declaration.kind
                ),
            ));
        }
        if matches!(
            declaration.kind,
            ArchitecturePackageKind::Adapter | ArchitecturePackageKind::Presentation
        ) && declaration.boundary != ArchitectureBoundaryKind::Adapter
        {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "architecture.owner.adapter_boundary",
                format!(
                    "package {package} is classified as {:?} without the adapter boundary",
                    declaration.kind
                ),
            ));
        }
        for (kind, targets) in [
            (ArchitectureDependencyKind::Normal, &declaration.normal),
            (
                ArchitectureDependencyKind::Development,
                &declaration.development,
            ),
            (ArchitectureDependencyKind::Build, &declaration.build),
        ] {
            let mut seen = BTreeSet::new();
            for target in targets {
                if !seen.insert(target) {
                    issues.push(ValidationIssue::new(
                        ARCHITECTURE_OWNER_PATH,
                        "architecture.owner.duplicate_direction",
                        format!(
                            "package {package} repeats {target} in its {} dependency directions",
                            kind.label()
                        ),
                    ));
                }
                let Some(target_declaration) = owner.packages.get(target) else {
                    issues.push(ValidationIssue::new(
                        ARCHITECTURE_OWNER_PATH,
                        "architecture.owner.unknown_direction",
                        format!(
                            "package {package} allows undeclared package {target} for {} dependencies",
                            kind.label()
                        ),
                    ));
                    continue;
                };
                if target == package {
                    issues.push(ValidationIssue::new(
                        ARCHITECTURE_OWNER_PATH,
                        "architecture.owner.self_dependency",
                        format!(
                            "package {package} allows itself as a {} dependency",
                            kind.label()
                        ),
                    ));
                }
                if declaration.production
                    && target_declaration.kind == ArchitecturePackageKind::TestSupport
                    && kind != ArchitectureDependencyKind::Development
                {
                    issues.push(ValidationIssue::new(
                        ARCHITECTURE_OWNER_PATH,
                        "architecture.owner.production_test_support",
                        format!(
                            "production package {package} allows test-support package {target} as a {} dependency",
                            kind.label()
                        ),
                    ));
                }
                if matches!(
                    declaration.boundary,
                    ArchitectureBoundaryKind::Core | ArchitectureBoundaryKind::CoreFacing
                ) && target_declaration.boundary == ArchitectureBoundaryKind::Adapter
                {
                    issues.push(ValidationIssue::new(
                        ARCHITECTURE_OWNER_PATH,
                        "architecture.owner.core_adapter",
                        format!(
                            "Core-facing package {package} allows adapter package {target} as a {} dependency",
                            kind.label()
                        ),
                    ));
                }
                validate_required_boundary(
                    package,
                    declaration,
                    target,
                    target_declaration,
                    kind,
                    ARCHITECTURE_OWNER_PATH,
                    "architecture.owner",
                    issues,
                );
            }
        }
    }

    let declared_graph = owner
        .packages
        .iter()
        .map(|(package, declaration)| {
            let dependencies = declaration
                .normal
                .iter()
                .map(|target| ArchitectureDependency {
                    package: target.clone(),
                    kind: ArchitectureDependencyKind::Normal,
                })
                .chain(
                    declaration
                        .build
                        .iter()
                        .map(|target| ArchitectureDependency {
                            package: target.clone(),
                            kind: ArchitectureDependencyKind::Build,
                        }),
                )
                .collect();
            (
                package.clone(),
                ArchitecturePackage {
                    manifest_path: ARCHITECTURE_OWNER_PATH.to_owned(),
                    dependencies,
                },
            )
        })
        .collect();
    validate_deployable_cycles(
        &declared_graph,
        "architecture.owner.dependency_cycle",
        issues,
    );
}

fn allowed_packages(
    package: &ArchitecturePackageDeclaration,
    kind: ArchitectureDependencyKind,
) -> &[String] {
    match kind {
        ArchitectureDependencyKind::Normal => &package.normal,
        ArchitectureDependencyKind::Development => &package.development,
        ArchitectureDependencyKind::Build => &package.build,
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_required_boundary(
    source_name: &str,
    source: &ArchitecturePackageDeclaration,
    target_name: &str,
    target: &ArchitecturePackageDeclaration,
    kind: ArchitectureDependencyKind,
    path: &str,
    category_prefix: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let violation = if source.group == "user-action-service"
        && (target.group == "core" || target.boundary == ArchitectureBoundaryKind::Adapter)
    {
        Some((
            "user_action_boundary",
            "UserAction service packages cannot depend on Core or adapter packages",
        ))
    } else if source.group == "core" && target.group == "command-model" {
        Some((
            "core_layer",
            "Core packages cannot depend on the administrative command model",
        ))
    } else if source.group == "shared-types"
        && matches!(
            target.kind,
            ArchitecturePackageKind::Adapter
                | ArchitecturePackageKind::Application
                | ArchitecturePackageKind::Presentation
        )
    {
        Some((
            "types_layer",
            "shared types cannot depend on application or presentation layers",
        ))
    } else if source.group == "storage"
        && (target.group == "core" || target.boundary == ArchitectureBoundaryKind::Adapter)
    {
        Some((
            "store_layer",
            "Store packages cannot depend on Core or adapter packages",
        ))
    } else {
        None
    };

    if let Some((suffix, message)) = violation {
        let category = match (category_prefix, suffix) {
            ("architecture.owner", "user_action_boundary") => {
                "architecture.owner.user_action_boundary"
            }
            ("architecture.owner", "core_layer") => "architecture.owner.core_layer",
            ("architecture.owner", "types_layer") => "architecture.owner.types_layer",
            ("architecture.owner", "store_layer") => "architecture.owner.store_layer",
            (_, "user_action_boundary") => "architecture.dependency.user_action_boundary",
            (_, "core_layer") => "architecture.dependency.core_layer",
            (_, "types_layer") => "architecture.dependency.types_layer",
            (_, "store_layer") => "architecture.dependency.store_layer",
            _ => unreachable!("required architecture boundary categories are closed"),
        };
        issues.push(ValidationIssue::new(
            path,
            category,
            format!(
                "{} dependency {source_name} ({}) -> {target_name} ({}) violates the required boundary: {message}",
                kind.label(),
                source.group,
                target.group
            ),
        ));
    }
}

fn validate_deployable_cycles(
    graph: &ArchitectureGraph,
    category: &'static str,
    issues: &mut Vec<ValidationIssue>,
) {
    let mut states = BTreeMap::<String, u8>::new();
    let mut stack = Vec::new();
    let mut cycles = BTreeSet::new();

    for package in graph.keys() {
        if states.get(package).copied().unwrap_or_default() == 0 {
            find_deployable_cycles(package, graph, &mut states, &mut stack, &mut cycles);
        }
    }

    for cycle in cycles {
        let source = cycle
            .split(" -> ")
            .next()
            .and_then(|package| graph.get(package));
        issues.push(ValidationIssue::new(
            source
                .map(|package| package.manifest_path.as_str())
                .unwrap_or(ARCHITECTURE_OWNER_PATH),
            category,
            format!(
                "normal/build internal dependency cycle detected: {cycle}; development-only edges do not participate in the deployable graph"
            ),
        ));
    }
}

fn find_deployable_cycles(
    package: &str,
    graph: &ArchitectureGraph,
    states: &mut BTreeMap<String, u8>,
    stack: &mut Vec<String>,
    cycles: &mut BTreeSet<String>,
) {
    states.insert(package.to_owned(), 1);
    stack.push(package.to_owned());

    if let Some(node) = graph.get(package) {
        for dependency in node.dependencies.iter().filter(|dependency| {
            matches!(
                dependency.kind,
                ArchitectureDependencyKind::Normal | ArchitectureDependencyKind::Build
            )
        }) {
            let state = states.get(&dependency.package).copied().unwrap_or_default();
            if state == 0 {
                find_deployable_cycles(&dependency.package, graph, states, stack, cycles);
            } else if state == 1 {
                let start = stack
                    .iter()
                    .position(|entry| entry == &dependency.package)
                    .unwrap_or_default();
                let mut cycle = stack[start..].to_vec();
                cycle.push(dependency.package.clone());
                cycles.insert(cycle.join(" -> "));
            }
        }
    }

    stack.pop();
    states.insert(package.to_owned(), 2);
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ArchitectureDocumentLanguage {
    English,
    Korean,
}

impl ArchitecturePackageKind {
    const fn label(self, language: ArchitectureDocumentLanguage) -> &'static str {
        match (self, language) {
            (Self::Adapter, ArchitectureDocumentLanguage::English) => "adapter",
            (Self::Application, ArchitectureDocumentLanguage::English) => "application",
            (Self::Infrastructure, ArchitectureDocumentLanguage::English) => "infrastructure",
            (Self::Presentation, ArchitectureDocumentLanguage::English) => "presentation",
            (Self::Schema, ArchitectureDocumentLanguage::English) => "schema",
            (Self::TestSupport, ArchitectureDocumentLanguage::English) => "test support",
            (Self::Validation, ArchitectureDocumentLanguage::English) => "validation",
            (Self::Adapter, ArchitectureDocumentLanguage::Korean) => "어댑터",
            (Self::Application, ArchitectureDocumentLanguage::Korean) => "애플리케이션",
            (Self::Infrastructure, ArchitectureDocumentLanguage::Korean) => "인프라",
            (Self::Presentation, ArchitectureDocumentLanguage::Korean) => "표현",
            (Self::Schema, ArchitectureDocumentLanguage::Korean) => "스키마",
            (Self::TestSupport, ArchitectureDocumentLanguage::Korean) => "테스트 지원",
            (Self::Validation, ArchitectureDocumentLanguage::Korean) => "검증",
        }
    }
}

impl ArchitectureBoundaryKind {
    const fn label(self, language: ArchitectureDocumentLanguage) -> &'static str {
        match (self, language) {
            (Self::Core, ArchitectureDocumentLanguage::English) => "Core",
            (Self::CoreFacing, ArchitectureDocumentLanguage::English) => "Core-facing",
            (Self::Adapter, ArchitectureDocumentLanguage::English) => "adapter",
            (Self::Neutral, ArchitectureDocumentLanguage::English) => "neutral",
            (Self::Core, ArchitectureDocumentLanguage::Korean) => "Core",
            (Self::CoreFacing, ArchitectureDocumentLanguage::Korean) => "Core 지향",
            (Self::Adapter, ArchitectureDocumentLanguage::Korean) => "어댑터",
            (Self::Neutral, ArchitectureDocumentLanguage::Korean) => "중립",
        }
    }
}

pub(crate) fn sync_generated_architecture_regions(
    root: &Path,
    index: &crate::doc_index::DocIndex,
) -> Result<Vec<String>> {
    let Some(document) = index.paired_documents.get(ARCHITECTURE_DOC_ID) else {
        return Ok(Vec::new());
    };
    let owner = read_validated_architecture_owner(root)?;
    let mut candidates = Vec::new();

    for (relative, language) in [
        (&document.path_en, ArchitectureDocumentLanguage::English),
        (&document.path_ko, ArchitectureDocumentLanguage::Korean),
    ] {
        let path = root.join(relative);
        let contents = fs::read_to_string(&path).with_context(|| {
            format!(
                "failed to read generated architecture owner at {}",
                path.display()
            )
        })?;
        let expected = generated_architecture_region(&owner, language);
        let updated = replace_generated_architecture_region(&contents, &expected)
            .with_context(|| format!("invalid generated architecture region in {relative}"))?;
        candidates.push((relative.to_string(), path, contents, updated));
    }

    let mut updated_paths = Vec::new();
    for (relative, path, contents, updated) in candidates {
        if contents == updated {
            continue;
        }
        fs::write(&path, updated).with_context(|| {
            format!(
                "failed to update generated architecture owner at {}",
                path.display()
            )
        })?;
        updated_paths.push(relative);
    }
    Ok(updated_paths)
}

pub(crate) fn validate_generated_architecture_regions(
    root: &Path,
    index: &crate::doc_index::DocIndex,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(document) = index.paired_documents.get(ARCHITECTURE_DOC_ID) else {
        return;
    };
    let owner = match read_validated_architecture_owner(root) {
        Ok(owner) => owner,
        Err(error) => {
            issues.push(ValidationIssue::new(
                ARCHITECTURE_OWNER_PATH,
                "generated_architecture.owner",
                format!("cannot derive generated architecture documentation: {error:#}"),
            ));
            return;
        }
    };

    for (relative, language) in [
        (&document.path_en, ArchitectureDocumentLanguage::English),
        (&document.path_ko, ArchitectureDocumentLanguage::Korean),
    ] {
        let contents = match fs::read_to_string(root.join(relative)) {
            Ok(contents) => contents,
            Err(error) => {
                issues.push(ValidationIssue::new(
                    relative,
                    "generated_architecture.read",
                    format!("failed to read generated architecture owner: {error}"),
                ));
                continue;
            }
        };
        let range = match generated_architecture_region_range(&contents) {
            Ok(range) => range,
            Err(error) => {
                issues.push(ValidationIssue::new(
                    relative,
                    "generated_architecture.markers",
                    error.to_string(),
                ));
                continue;
            }
        };
        let expected = generated_architecture_region(&owner, language);
        if contents[range] != expected {
            issues.push(ValidationIssue::new(
                relative,
                "generated_architecture.drift",
                "generated package architecture differs from workspace metadata; run `cargo run -p xtask -- docs-sync`",
            ));
        }
    }
}

fn read_validated_architecture_owner(root: &Path) -> Result<ArchitectureOwner> {
    let owner = read_architecture_owner(&root.join(ARCHITECTURE_OWNER_PATH))?;
    let graph = read_workspace_graph(root)?;
    let issues = validate_architecture_graph(&owner, &graph);
    if !issues.is_empty() {
        let messages = issues
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::bail!(
            "workspace architecture must validate before documentation generation:\n{messages}"
        );
    }
    Ok(owner)
}

fn generated_architecture_region(
    owner: &ArchitectureOwner,
    language: ArchitectureDocumentLanguage,
) -> String {
    let mut output = String::new();
    output.push_str(ARCHITECTURE_BEGIN_MARKER);
    output.push('\n');
    output.push_str(
        "<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->\n\n",
    );

    match language {
        ArchitectureDocumentLanguage::English => {
            output.push_str("### Package responsibilities\n\n");
            output.push_str(
                "Each current workspace package has one responsibility entry in the root Cargo metadata.\n\n",
            );
            output.push_str("| Package | Group | Kind | Runtime | Boundary | Responsibility |\n");
            output.push_str("|---|---|---|---|---|---|\n");
        }
        ArchitectureDocumentLanguage::Korean => {
            output.push_str("### 패키지 책임\n\n");
            output.push_str(
                "현재 워크스페이스의 각 패키지는 루트 Cargo 메타데이터에 하나의 책임 항목을 가집니다.\n\n",
            );
            output.push_str("| 패키지 | 그룹 | 종류 | 런타임 | 경계 | 책임 |\n");
            output.push_str("|---|---|---|---|---|---|\n");
        }
    }

    for (package, declaration) in &owner.packages {
        let production = match (declaration.production, language) {
            (true, ArchitectureDocumentLanguage::English) => "production",
            (false, ArchitectureDocumentLanguage::English) => "non-production",
            (true, ArchitectureDocumentLanguage::Korean) => "프로덕션",
            (false, ArchitectureDocumentLanguage::Korean) => "비프로덕션",
        };
        let description = match language {
            ArchitectureDocumentLanguage::English => &declaration.description,
            ArchitectureDocumentLanguage::Korean => &declaration.description_ko,
        };
        output.push_str(&format!(
            "| `{package}` | `{}` | {} | {} | {} | {} |\n",
            declaration.group,
            declaration.kind.label(language),
            production,
            declaration.boundary.label(language),
            escape_markdown_table_cell(description)
        ));
    }

    match language {
        ArchitectureDocumentLanguage::English => {
            output.push_str("\n### Allowed internal dependency directions\n\n");
            output.push_str(
                "The lists are package-level allowlists by Cargo dependency kind. An em dash means that no internal package is allowed for that kind.\n\n",
            );
            output.push_str("| Package | Normal | Development | Build |\n");
            output.push_str("|---|---|---|---|\n");
        }
        ArchitectureDocumentLanguage::Korean => {
            output.push_str("\n### 허용되는 내부 의존 방향\n\n");
            output.push_str(
                "각 목록은 Cargo 의존 종류별 패키지 허용 목록입니다. 긴 대시는 해당 종류에 허용된 내부 패키지가 없음을 뜻합니다.\n\n",
            );
            output.push_str("| 패키지 | 일반 | 개발 | 빌드 |\n");
            output.push_str("|---|---|---|---|\n");
        }
    }

    for (package, declaration) in &owner.packages {
        output.push_str(&format!(
            "| `{package}` | {} | {} | {} |\n",
            render_dependency_list(&declaration.normal),
            render_dependency_list(&declaration.development),
            render_dependency_list(&declaration.build)
        ));
    }

    output.push('\n');
    output.push_str(ARCHITECTURE_END_MARKER);
    output
}

fn render_dependency_list(packages: &[String]) -> String {
    if packages.is_empty() {
        return "—".to_owned();
    }
    let mut packages = packages.to_vec();
    packages.sort();
    packages
        .iter()
        .map(|package| format!("`{package}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn escape_markdown_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn replace_generated_architecture_region(contents: &str, expected: &str) -> Result<String> {
    let range = generated_architecture_region_range(contents)?;
    let mut updated = String::with_capacity(contents.len() + expected.len());
    updated.push_str(&contents[..range.start]);
    updated.push_str(expected);
    updated.push_str(&contents[range.end..]);
    Ok(updated)
}

fn generated_architecture_region_range(contents: &str) -> Result<std::ops::Range<usize>> {
    let starts = contents
        .match_indices(ARCHITECTURE_BEGIN_MARKER)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let ends = contents
        .match_indices(ARCHITECTURE_END_MARKER)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if starts.len() != 1 || ends.len() != 1 {
        anyhow::bail!(
            "expected exactly one {ARCHITECTURE_BEGIN_MARKER} marker and one {ARCHITECTURE_END_MARKER} marker"
        );
    }
    if ends[0] <= starts[0] {
        anyhow::bail!("generated architecture end marker must follow its begin marker");
    }
    Ok(starts[0]..ends[0] + ARCHITECTURE_END_MARKER.len())
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

    fn declaration(
        group: &str,
        kind: ArchitecturePackageKind,
        boundary: ArchitectureBoundaryKind,
        normal: &[&str],
        development: &[&str],
        build: &[&str],
    ) -> ArchitecturePackageDeclaration {
        ArchitecturePackageDeclaration {
            group: group.to_owned(),
            description: "Synthetic package responsibility.".to_owned(),
            description_ko: "합성 패키지 책임입니다.".to_owned(),
            kind,
            production: !matches!(
                kind,
                ArchitecturePackageKind::TestSupport | ArchitecturePackageKind::Validation
            ),
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
                (
                    "engine".to_owned(),
                    declaration(
                        "application",
                        ArchitecturePackageKind::Application,
                        ArchitectureBoundaryKind::Neutral,
                        &["foundation"],
                        &["fixture-kit"],
                        &["code-generator"],
                    ),
                ),
                (
                    "foundation".to_owned(),
                    declaration(
                        "foundation",
                        ArchitecturePackageKind::Infrastructure,
                        ArchitectureBoundaryKind::Neutral,
                        &[],
                        &[],
                        &[],
                    ),
                ),
                (
                    "fixture-kit".to_owned(),
                    declaration(
                        "fixtures",
                        ArchitecturePackageKind::TestSupport,
                        ArchitectureBoundaryKind::Neutral,
                        &[],
                        &[],
                        &[],
                    ),
                ),
                (
                    "code-generator".to_owned(),
                    declaration(
                        "build-tooling",
                        ArchitecturePackageKind::Validation,
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
    fn neutral_synthetic_graph_rejects_invalid_dependency_type() {
        let owner = ArchitectureOwner {
            packages: BTreeMap::from([
                (
                    "engine".to_owned(),
                    declaration(
                        "application",
                        ArchitecturePackageKind::Application,
                        ArchitectureBoundaryKind::Neutral,
                        &["foundation"],
                        &[],
                        &[],
                    ),
                ),
                (
                    "foundation".to_owned(),
                    declaration(
                        "foundation",
                        ArchitecturePackageKind::Infrastructure,
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
        ]);

        let issues = validate_architecture_graph(&owner, &graph);

        assert!(issues.iter().any(|issue| {
            issue.category() == "architecture.dependency.disallowed"
                && issue.message().contains("build dependency")
        }));
    }

    #[test]
    fn synthetic_graph_rejects_unregistered_workspace_package() {
        let owner = ArchitectureOwner {
            packages: BTreeMap::from([(
                "foundation".to_owned(),
                declaration(
                    "foundation",
                    ArchitecturePackageKind::Infrastructure,
                    ArchitectureBoundaryKind::Neutral,
                    &[],
                    &[],
                    &[],
                ),
            )]),
        };
        let graph = BTreeMap::from([
            ("foundation".to_owned(), package(&[])),
            ("unexpected-tool".to_owned(), package(&[])),
        ]);

        let issues = validate_architecture_graph(&owner, &graph);

        assert!(issues.iter().any(|issue| {
            issue.category() == "architecture.package.undeclared"
                && issue.message().contains("unexpected-tool")
        }));
    }

    #[test]
    fn synthetic_graph_rejects_production_and_core_boundary_violations() {
        let owner = ArchitectureOwner {
            packages: BTreeMap::from([
                (
                    "engine".to_owned(),
                    declaration(
                        "core-services",
                        ArchitecturePackageKind::Application,
                        ArchitectureBoundaryKind::CoreFacing,
                        &["terminal", "fixture-kit"],
                        &[],
                        &[],
                    ),
                ),
                (
                    "terminal".to_owned(),
                    declaration(
                        "adapter",
                        ArchitecturePackageKind::Adapter,
                        ArchitectureBoundaryKind::Adapter,
                        &[],
                        &[],
                        &[],
                    ),
                ),
                (
                    "fixture-kit".to_owned(),
                    declaration(
                        "fixtures",
                        ArchitecturePackageKind::TestSupport,
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

    #[test]
    fn synthetic_graph_rejects_deployable_dependency_cycles() {
        let owner = ArchitectureOwner {
            packages: BTreeMap::from([
                (
                    "planner".to_owned(),
                    declaration(
                        "planning",
                        ArchitecturePackageKind::Application,
                        ArchitectureBoundaryKind::Neutral,
                        &["records"],
                        &[],
                        &[],
                    ),
                ),
                (
                    "records".to_owned(),
                    declaration(
                        "records",
                        ArchitecturePackageKind::Infrastructure,
                        ArchitectureBoundaryKind::Neutral,
                        &["planner"],
                        &[],
                        &[],
                    ),
                ),
            ]),
        };
        let graph = BTreeMap::from([
            (
                "planner".to_owned(),
                package(&[("records", ArchitectureDependencyKind::Normal)]),
            ),
            (
                "records".to_owned(),
                package(&[("planner", ArchitectureDependencyKind::Normal)]),
            ),
        ]);

        let issues = validate_architecture_graph(&owner, &graph);

        assert!(issues
            .iter()
            .any(|issue| issue.category() == "architecture.dependency.cycle"));
        assert!(issues
            .iter()
            .any(|issue| issue.category() == "architecture.owner.dependency_cycle"));
    }

    #[test]
    fn generated_architecture_output_is_deterministic() {
        let owner = ArchitectureOwner {
            packages: BTreeMap::from([(
                "foundation".to_owned(),
                declaration(
                    "foundation",
                    ArchitecturePackageKind::Infrastructure,
                    ArchitectureBoundaryKind::Neutral,
                    &[],
                    &[],
                    &[],
                ),
            )]),
        };

        let first = generated_architecture_region(&owner, ArchitectureDocumentLanguage::English);
        let second = generated_architecture_region(&owner, ArchitectureDocumentLanguage::English);

        assert_eq!(first, second);
        assert!(first.contains("| `foundation` | `foundation` | infrastructure | production |"));
    }

    #[test]
    fn generated_architecture_sync_is_idempotent_and_drift_is_detected() {
        let fixture = tempfile::tempdir().expect("architecture fixture");
        let root = fixture.path();
        fs::create_dir_all(root.join("components/foundation/src")).expect("package source");
        fs::create_dir_all(root.join("docs/en/architecture-guide")).expect("English docs");
        fs::create_dir_all(root.join("docs/ko/architecture-guide")).expect("Korean docs");
        fs::write(
            root.join("Cargo.toml"),
            r#"[workspace]
members = ["components/foundation"]
resolver = "2"

[workspace.metadata.architecture.packages.foundation]
group = "foundation"
description = "Synthetic foundation."
description_ko = "합성 기반 패키지입니다."
kind = "infrastructure"
production = true
boundary = "neutral"
normal = []
development = []
build = []
"#,
        )
        .expect("root manifest");
        fs::write(
            root.join("components/foundation/Cargo.toml"),
            r#"[package]
name = "foundation"
version = "0.0.0"
edition = "2021"
"#,
        )
        .expect("package manifest");
        fs::write(root.join("components/foundation/src/lib.rs"), "").expect("package source file");
        fs::write(
            root.join("Cargo.lock"),
            r#"# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 3

[[package]]
name = "foundation"
version = "0.0.0"
"#,
        )
        .expect("lockfile");
        let owner = format!("{ARCHITECTURE_BEGIN_MARKER}\n{ARCHITECTURE_END_MARKER}\n");
        fs::write(
            root.join("docs/en/architecture-guide/architecture.md"),
            &owner,
        )
        .expect("English owner");
        fs::write(
            root.join("docs/ko/architecture-guide/architecture.md"),
            &owner,
        )
        .expect("Korean owner");
        let document = crate::doc_index::PairedDocument {
            doc_id: ARCHITECTURE_DOC_ID.to_owned(),
            path_en: "docs/en/architecture-guide/architecture.md".to_owned(),
            path_ko: "docs/ko/architecture-guide/architecture.md".to_owned(),
            contracts: BTreeSet::new(),
        };
        let index = crate::doc_index::DocIndex {
            indexed_paths: BTreeSet::new(),
            paired_paths: BTreeMap::new(),
            path_doc_ids: BTreeMap::new(),
            paired_documents: BTreeMap::from([(ARCHITECTURE_DOC_ID.to_owned(), document)]),
        };

        let first =
            sync_generated_architecture_regions(root, &index).expect("first architecture sync");
        let second =
            sync_generated_architecture_regions(root, &index).expect("second architecture sync");

        assert_eq!(
            first,
            [
                "docs/en/architecture-guide/architecture.md",
                "docs/ko/architecture-guide/architecture.md"
            ]
        );
        assert!(second.is_empty());

        let path = root.join("docs/en/architecture-guide/architecture.md");
        let drifted = fs::read_to_string(&path)
            .expect("generated English owner")
            .replacen("Synthetic foundation.", "Drifted responsibility.", 1);
        fs::write(path, drifted).expect("drifted English owner");
        let mut issues = Vec::new();
        validate_generated_architecture_regions(root, &index, &mut issues);

        assert!(issues.iter().any(|issue| {
            issue.category() == "generated_architecture.drift"
                && issue.path() == "docs/en/architecture-guide/architecture.md"
        }));
    }
}
