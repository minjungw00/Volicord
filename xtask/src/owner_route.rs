use crate::architecture::{derive_workspace_package_inputs, WorkspacePackageInput};
use crate::diagnostics::ValidationIssue;
use crate::repository::{normalize_existing_root, path_to_slash};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path};
use std::process::{Command, Stdio};

const DOC_INDEX_PATH: &str = "docs/doc-index.yaml";
const OWNER_ROUTING_PATH: &str = "docs/owner-routing.yaml";
const SUPPORTED_VALIDATION_CLASSES: &[&str] = &[
    "architecture",
    "documentation",
    "mcp-spec",
    "release",
    "repository-hygiene",
    "rust",
    "workflow",
];

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct OwnerRouteReport {
    pub base_revision: Option<String>,
    pub changes: Vec<RepositoryChange>,
    pub instructions: Vec<RoutedInstruction>,
    pub workspace_packages: Vec<RoutedPackage>,
    pub maintained_documents: Vec<RoutedDocument>,
    pub owner_documents: Vec<RoutedOwnerDocument>,
    pub validation_classes: Vec<String>,
    pub unknown_paths: Vec<UnknownRoutedPath>,
}

impl OwnerRouteReport {
    pub fn render_human(&self) -> String {
        let mut output = String::from("owner-route\n");
        output.push_str(&format!(
            "base revision: {}\n",
            self.base_revision
                .as_deref()
                .unwrap_or("working tree vs HEAD")
        ));
        render_list(
            &mut output,
            "changes",
            &self
                .changes
                .iter()
                .map(RepositoryChange::render_human)
                .collect::<Vec<_>>(),
        );
        render_list(
            &mut output,
            "instructions",
            &self
                .instructions
                .iter()
                .map(|item| {
                    format!(
                        "{}; basis={}; reasons={}",
                        item.path,
                        item.routing_basis.label(),
                        item.reasons.join(", ")
                    )
                })
                .collect::<Vec<_>>(),
        );
        render_list(
            &mut output,
            "workspace packages",
            &self
                .workspace_packages
                .iter()
                .map(|item| {
                    format!(
                        "{}; manifest={}; basis={}; changed={}",
                        item.name,
                        item.manifest_path,
                        item.routing_basis.label(),
                        item.changed_paths.join(", ")
                    )
                })
                .collect::<Vec<_>>(),
        );
        render_list(
            &mut output,
            "maintained documents",
            &self
                .maintained_documents
                .iter()
                .map(|item| {
                    format!(
                        "{}; basis={}; paths={}; changed={}; owner_area={}; summary={}; canonical_for={}; depends_on={}",
                        item.doc_id,
                        item.routing_basis.label(),
                        item.paths.join(", "),
                        item.changed_paths.join(", "),
                        item.owner_area,
                        item.summary,
                        item.canonical_for.join(" | "),
                        item.depends_on.join(", ")
                    )
                })
                .collect::<Vec<_>>(),
        );
        render_list(
            &mut output,
            "owner documents",
            &self
                .owner_documents
                .iter()
                .map(|item| {
                    format!(
                        "{}; basis={}; paths={}; reasons={}",
                        item.doc_id,
                        item.routing_basis.label(),
                        item.paths.join(", "),
                        item.reasons.join(", ")
                    )
                })
                .collect::<Vec<_>>(),
        );
        render_list(&mut output, "validation classes", &self.validation_classes);
        render_list(
            &mut output,
            "unknown paths",
            &self
                .unknown_paths
                .iter()
                .map(|item| format!("{}; basis={}", item.path, item.routing_basis.label()))
                .collect::<Vec<_>>(),
        );
        output
    }

    pub fn changed_paths(&self) -> Vec<String> {
        self.changes
            .iter()
            .flat_map(|change| [change.old_path.as_ref(), change.new_path.as_ref()])
            .flatten()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

fn render_list(output: &mut String, heading: &str, values: &[String]) {
    output.push_str(heading);
    output.push_str(":\n");
    if values.is_empty() {
        output.push_str("- none\n");
    } else {
        for value in values {
            output.push_str("- ");
            output.push_str(value);
            output.push('\n');
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
}

impl RepositoryChangeKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Renamed => "renamed",
            Self::Copied => "copied",
            Self::TypeChanged => "type_changed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingBasis {
    Base,
    Current,
}

impl RoutingBasis {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Current => "current",
        }
    }
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ChangeRoutingEndpoint {
    pub routing_basis: RoutingBasis,
    pub path: String,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RepositoryChange {
    pub kind: RepositoryChangeKind,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub routing: Vec<ChangeRoutingEndpoint>,
}

impl RepositoryChange {
    fn render_human(&self) -> String {
        let routing = self
            .routing
            .iter()
            .map(|endpoint| format!("{}:{}", endpoint.routing_basis.label(), endpoint.path))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "kind={}; old={}; new={}; routing={routing}",
            self.kind.label(),
            self.old_path.as_deref().unwrap_or("none"),
            self.new_path.as_deref().unwrap_or("none")
        )
    }
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct UnknownRoutedPath {
    pub routing_basis: RoutingBasis,
    pub path: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RoutedInstruction {
    pub routing_basis: RoutingBasis,
    pub path: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RoutedPackage {
    pub routing_basis: RoutingBasis,
    pub name: String,
    pub manifest_path: String,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RoutedDocument {
    pub routing_basis: RoutingBasis,
    pub doc_id: String,
    pub paths: Vec<String>,
    pub changed_paths: Vec<String>,
    pub owner_area: String,
    pub summary: String,
    pub canonical_for: Vec<String>,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RoutedOwnerDocument {
    pub routing_basis: RoutingBasis,
    pub doc_id: String,
    pub paths: Vec<String>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutingMetadata {
    validation_classes: BTreeMap<String, String>,
    instruction_scopes: Vec<InstructionScope>,
    path_routes: Vec<PathRoute>,
    tracked_path_exemptions: Vec<TrackedPathExemption>,
    ci_trigger_policy: serde_yaml::Value,
    package_defaults: PackageRoute,
    package_routes: BTreeMap<String, PackageRoute>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstructionScope {
    path_prefix: String,
    instruction: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PathRoute {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    path_prefix: Option<String>,
    #[serde(default)]
    owner_doc_ids: Vec<String>,
    #[serde(default)]
    validation_classes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackedPathExemption {
    path: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CiTriggerPolicy {
    workflow: String,
    repository_changes: RepositoryChangeTrigger,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum RepositoryChangeTrigger {
    All,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageRoute {
    #[serde(default)]
    instruction_paths: Vec<String>,
    #[serde(default)]
    owner_doc_ids: Vec<String>,
    #[serde(default)]
    validation_classes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RoutingDocIndex {
    shared_documents: Vec<SharedRoutingDocument>,
    documents: Vec<PairedRoutingDocument>,
}

#[derive(Debug, Deserialize)]
struct SharedRoutingDocument {
    doc_id: String,
    path: String,
    summary: String,
    owner_area: String,
    #[serde(default)]
    canonical_for: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PairedRoutingDocument {
    doc_id: String,
    path_en: String,
    path_ko: String,
    summary: String,
    owner_area: String,
    #[serde(default)]
    canonical_for: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
}

#[derive(Debug, Clone)]
struct DocumentRouteEntry {
    doc_id: String,
    paths: Vec<String>,
    summary: String,
    owner_area: String,
    canonical_for: Vec<String>,
    depends_on: Vec<String>,
}

struct RoutingSnapshot {
    basis: RoutingBasis,
    metadata: RoutingMetadata,
    documents: BTreeMap<String, DocumentRouteEntry>,
    document_by_path: BTreeMap<String, String>,
    packages: Vec<WorkspacePackageInput>,
    tracked_paths: BTreeSet<String>,
}

impl RoutingSnapshot {
    fn load(root: &Path, basis: RoutingBasis, tracked_paths: BTreeSet<String>) -> Result<Self> {
        let metadata = load_routing_metadata(root)?;
        let documents = load_document_routes(root)?;
        let document_by_path = documents
            .values()
            .flat_map(|document| {
                document
                    .paths
                    .iter()
                    .map(move |path| (path.clone(), document.doc_id.clone()))
            })
            .collect();
        Ok(Self {
            basis,
            metadata,
            documents,
            document_by_path,
            packages: derive_workspace_package_inputs(root)?,
            tracked_paths,
        })
    }
}

pub fn run_owner_route(root: &Path, base: Option<&str>) -> Result<OwnerRouteReport> {
    let root = normalize_existing_root(root)?;
    ensure_git_repository_root(&root)?;
    let comparison_revision = resolve_commit(&root, base.unwrap_or("HEAD"))?;
    let base_revision = base.map(|_| comparison_revision.clone());
    let changes = discover_repository_changes(&root, &comparison_revision)?;
    let base_snapshot = load_revision_snapshot(&root, &comparison_revision)?;
    let current_snapshot = RoutingSnapshot::load(
        &root,
        RoutingBasis::Current,
        current_tracked_git_paths(&root)?,
    )?;
    validate_routing_metadata(&root, &current_snapshot)?;
    route_changes(base_revision, changes, &base_snapshot, &current_snapshot)
}

pub(crate) fn validate_owner_routing(root: &Path, issues: &mut Vec<ValidationIssue>) {
    let result = (|| {
        let snapshot = RoutingSnapshot::load(
            root,
            RoutingBasis::Current,
            current_tracked_git_paths(root)?,
        )?;
        validate_routing_metadata(root, &snapshot)
    })();
    if let Err(error) = result {
        issues.push(ValidationIssue::new(
            OWNER_ROUTING_PATH,
            "metadata.owner_routing",
            format!("{error:#}"),
        ));
    }
}

fn route_changes(
    base_revision: Option<String>,
    changes: Vec<RepositoryChange>,
    base: &RoutingSnapshot,
    current: &RoutingSnapshot,
) -> Result<OwnerRouteReport> {
    let mut instruction_reasons = BTreeMap::<(RoutingBasis, String), BTreeSet<String>>::new();
    let mut package_paths = BTreeMap::<(RoutingBasis, String, String), BTreeSet<String>>::new();
    let mut document_paths = BTreeMap::<(RoutingBasis, String), BTreeSet<String>>::new();
    let mut owner_reasons = BTreeMap::<(RoutingBasis, String), BTreeSet<String>>::new();
    let mut validation_classes = BTreeSet::new();
    let mut unknown_paths = BTreeSet::new();

    for change in &changes {
        for endpoint in &change.routing {
            let snapshot = match endpoint.routing_basis {
                RoutingBasis::Base => base,
                RoutingBasis::Current => current,
            };
            route_endpoint(
                snapshot,
                &endpoint.path,
                &mut instruction_reasons,
                &mut package_paths,
                &mut document_paths,
                &mut owner_reasons,
                &mut validation_classes,
                &mut unknown_paths,
            );
        }
    }

    for (basis, package, _) in package_paths.keys() {
        let snapshot = match basis {
            RoutingBasis::Base => base,
            RoutingBasis::Current => current,
        };
        add_package_route(
            *basis,
            &snapshot.metadata.package_defaults,
            "workspace-package-default",
            &mut instruction_reasons,
            &mut owner_reasons,
            &mut validation_classes,
        );
        let route = snapshot
            .metadata
            .package_routes
            .get(package)
            .with_context(|| format!("routing metadata has no package route for {package}"))?;
        add_package_route(
            *basis,
            route,
            &format!("package:{package}"),
            &mut instruction_reasons,
            &mut owner_reasons,
            &mut validation_classes,
        );
    }

    let current_package_identities = package_paths
        .keys()
        .filter(|(basis, _, _)| *basis == RoutingBasis::Current)
        .map(|(_, name, manifest)| (name.clone(), manifest.clone()))
        .collect::<BTreeSet<_>>();
    if package_paths.keys().any(|(basis, name, manifest)| {
        *basis == RoutingBasis::Base
            && !current_package_identities.contains(&(name.clone(), manifest.clone()))
    }) {
        validation_classes.extend(
            ["architecture", "repository-hygiene", "rust"]
                .into_iter()
                .map(str::to_owned),
        );
    }

    let workspace_packages = package_paths
        .into_iter()
        .map(
            |((routing_basis, name, manifest_path), paths)| RoutedPackage {
                routing_basis,
                name,
                manifest_path,
                changed_paths: paths.into_iter().collect(),
            },
        )
        .collect();
    let maintained_documents = document_paths
        .into_iter()
        .map(|((routing_basis, doc_id), changed)| {
            let snapshot = match routing_basis {
                RoutingBasis::Base => base,
                RoutingBasis::Current => current,
            };
            let document = snapshot.documents.get(&doc_id).with_context(|| {
                format!(
                    "{} routing snapshot has no maintained document {doc_id}",
                    routing_basis.label()
                )
            })?;
            Ok(RoutedDocument {
                routing_basis,
                doc_id,
                paths: document.paths.clone(),
                changed_paths: changed.into_iter().collect(),
                owner_area: document.owner_area.clone(),
                summary: document.summary.clone(),
                canonical_for: document.canonical_for.clone(),
                depends_on: document.depends_on.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let owner_documents = owner_reasons
        .into_iter()
        .map(|((routing_basis, doc_id), reasons)| {
            let snapshot = match routing_basis {
                RoutingBasis::Base => base,
                RoutingBasis::Current => current,
            };
            let document = snapshot.documents.get(&doc_id).with_context(|| {
                format!(
                    "{} routing snapshot refers to unknown owner document {doc_id}",
                    routing_basis.label()
                )
            })?;
            Ok(RoutedOwnerDocument {
                routing_basis,
                doc_id,
                paths: document.paths.clone(),
                reasons: reasons.into_iter().collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let instructions = instruction_reasons
        .into_iter()
        .map(|((routing_basis, path), reasons)| RoutedInstruction {
            routing_basis,
            path,
            reasons: reasons.into_iter().collect(),
        })
        .collect();

    Ok(OwnerRouteReport {
        base_revision,
        changes,
        instructions,
        workspace_packages,
        maintained_documents,
        owner_documents,
        validation_classes: validation_classes.into_iter().collect(),
        unknown_paths: unknown_paths.into_iter().collect(),
    })
}

#[allow(clippy::too_many_arguments)]
fn route_endpoint(
    snapshot: &RoutingSnapshot,
    path: &str,
    instruction_reasons: &mut BTreeMap<(RoutingBasis, String), BTreeSet<String>>,
    package_paths: &mut BTreeMap<(RoutingBasis, String, String), BTreeSet<String>>,
    document_paths: &mut BTreeMap<(RoutingBasis, String), BTreeSet<String>>,
    owner_reasons: &mut BTreeMap<(RoutingBasis, String), BTreeSet<String>>,
    validation_classes: &mut BTreeSet<String>,
    unknown_paths: &mut BTreeSet<UnknownRoutedPath>,
) {
    let basis = snapshot.basis;
    let reason = format!("{}:{path}", basis.label());
    let recognized = classify_path(
        path,
        &snapshot.document_by_path,
        &snapshot.packages,
        &snapshot.metadata.path_routes,
        &snapshot.metadata.tracked_path_exemptions,
    )
    .is_recognized();
    for scope in &snapshot.metadata.instruction_scopes {
        if path_matches_prefix(path, &scope.path_prefix) {
            instruction_reasons
                .entry((basis, scope.instruction.clone()))
                .or_default()
                .insert(reason.clone());
        }
    }
    if let Some(doc_id) = snapshot.document_by_path.get(path) {
        document_paths
            .entry((basis, doc_id.clone()))
            .or_default()
            .insert(path.to_owned());
    }
    for package in &snapshot.packages {
        if package_contains_path(package, path) {
            package_paths
                .entry((
                    basis,
                    package.name().to_owned(),
                    package.manifest_path().to_owned(),
                ))
                .or_default()
                .insert(path.to_owned());
        }
    }
    for route in &snapshot.metadata.path_routes {
        if route.matches(path) {
            add_route(
                basis,
                owner_reasons,
                validation_classes,
                &route.owner_doc_ids,
                &route.validation_classes,
                &reason,
            );
        }
    }
    if !recognized {
        unknown_paths.insert(UnknownRoutedPath {
            routing_basis: basis,
            path: path.to_owned(),
        });
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PathClassification {
    maintained_document: bool,
    workspace_package: bool,
    explicit_route: bool,
    tracked_exemption: bool,
}

impl PathClassification {
    const fn is_recognized(self) -> bool {
        self.maintained_document
            || self.workspace_package
            || self.explicit_route
            || self.tracked_exemption
    }
}

fn classify_path(
    path: &str,
    document_by_path: &BTreeMap<String, String>,
    packages: &[WorkspacePackageInput],
    path_routes: &[PathRoute],
    exemptions: &[TrackedPathExemption],
) -> PathClassification {
    PathClassification {
        maintained_document: document_by_path.contains_key(path),
        workspace_package: packages
            .iter()
            .any(|package| package_contains_path(package, path)),
        explicit_route: path_routes.iter().any(|route| route.matches(path)),
        tracked_exemption: exemptions.iter().any(|item| item.path == path),
    }
}

impl PathRoute {
    fn matches(&self, candidate: &str) -> bool {
        self.path.as_deref() == Some(candidate)
            || self
                .path_prefix
                .as_deref()
                .is_some_and(|prefix| path_matches_prefix(candidate, prefix))
    }
}

fn add_route(
    basis: RoutingBasis,
    owner_reasons: &mut BTreeMap<(RoutingBasis, String), BTreeSet<String>>,
    validation: &mut BTreeSet<String>,
    owners: &[String],
    classes: &[String],
    reason: &str,
) {
    for owner in owners {
        owner_reasons
            .entry((basis, owner.clone()))
            .or_default()
            .insert(reason.to_owned());
    }
    validation.extend(classes.iter().cloned());
}

fn add_package_route(
    basis: RoutingBasis,
    route: &PackageRoute,
    reason: &str,
    instructions: &mut BTreeMap<(RoutingBasis, String), BTreeSet<String>>,
    owners: &mut BTreeMap<(RoutingBasis, String), BTreeSet<String>>,
    validation: &mut BTreeSet<String>,
) {
    for instruction in &route.instruction_paths {
        instructions
            .entry((basis, instruction.clone()))
            .or_default()
            .insert(reason.to_owned());
    }
    add_route(
        basis,
        owners,
        validation,
        &route.owner_doc_ids,
        &route.validation_classes,
        reason,
    );
}

fn package_contains_path(package: &WorkspacePackageInput, path: &str) -> bool {
    if path == package.manifest_path() {
        return true;
    }
    let Some(directory) = Path::new(package.manifest_path()).parent() else {
        return false;
    };
    let directory = path_to_slash(directory);
    path_matches_prefix(path, &format!("{directory}/"))
}

fn path_matches_prefix(path: &str, prefix: &str) -> bool {
    prefix.is_empty() || path.starts_with(prefix)
}

fn load_routing_metadata(root: &Path) -> Result<RoutingMetadata> {
    let path = root.join(OWNER_ROUTING_PATH);
    let contents =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_yaml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))
}

fn load_document_routes(root: &Path) -> Result<BTreeMap<String, DocumentRouteEntry>> {
    let path = root.join(DOC_INDEX_PATH);
    let contents =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let source: RoutingDocIndex = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse routing fields from {}", path.display()))?;
    let mut entries = BTreeMap::new();
    for item in source.shared_documents {
        insert_document(
            &mut entries,
            DocumentRouteEntry {
                doc_id: item.doc_id,
                paths: vec![item.path],
                summary: item.summary,
                owner_area: item.owner_area,
                canonical_for: item.canonical_for,
                depends_on: item.depends_on,
            },
        )?;
    }
    for item in source.documents {
        insert_document(
            &mut entries,
            DocumentRouteEntry {
                doc_id: item.doc_id,
                paths: vec![item.path_en, item.path_ko],
                summary: item.summary,
                owner_area: item.owner_area,
                canonical_for: item.canonical_for,
                depends_on: item.depends_on,
            },
        )?;
    }
    Ok(entries)
}

fn insert_document(
    entries: &mut BTreeMap<String, DocumentRouteEntry>,
    mut entry: DocumentRouteEntry,
) -> Result<()> {
    entry.paths.sort();
    entry.depends_on.sort();
    entry.depends_on.dedup();
    if entries.insert(entry.doc_id.clone(), entry).is_some() {
        bail!("{DOC_INDEX_PATH} contains a duplicate doc_id");
    }
    Ok(())
}

fn validate_routing_metadata(root: &Path, snapshot: &RoutingSnapshot) -> Result<()> {
    let metadata = &snapshot.metadata;
    let documents = &snapshot.documents;
    let packages = &snapshot.packages;
    let expected_classes = SUPPORTED_VALIDATION_CLASSES
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<BTreeSet<_>>();
    let actual_classes = metadata
        .validation_classes
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual_classes != expected_classes {
        bail!(
            "{OWNER_ROUTING_PATH} validation_classes differ from the supported catalog; expected {}, found {}",
            expected_classes.into_iter().collect::<Vec<_>>().join(", "),
            actual_classes.into_iter().collect::<Vec<_>>().join(", ")
        );
    }
    if metadata
        .validation_classes
        .values()
        .any(|description| description.trim().is_empty())
    {
        bail!("{OWNER_ROUTING_PATH} validation class descriptions must be non-empty");
    }

    let mut instruction_paths = BTreeSet::new();
    let mut instruction_scopes = BTreeSet::new();
    for scope in &metadata.instruction_scopes {
        if !scope.path_prefix.is_empty() {
            validate_relative_path(&scope.path_prefix, "instruction path prefix")?;
            if !scope.path_prefix.ends_with('/') {
                bail!(
                    "instruction path prefix {:?} must end with /",
                    scope.path_prefix
                );
            }
        }
        validate_relative_path(&scope.instruction, "instruction")?;
        if !root.join(&scope.instruction).is_file() {
            bail!("routing instruction {} does not exist", scope.instruction);
        }
        if !scope.instruction.ends_with("AGENTS.md") {
            bail!(
                "routing instruction {} is not an AGENTS.md",
                scope.instruction
            );
        }
        if !instruction_scopes.insert((scope.path_prefix.clone(), scope.instruction.clone())) {
            bail!(
                "routing instruction scope {:?} -> {} is duplicated",
                scope.path_prefix,
                scope.instruction
            );
        }
        instruction_paths.insert(scope.instruction.clone());
    }
    if !metadata
        .instruction_scopes
        .iter()
        .any(|scope| scope.path_prefix.is_empty() && scope.instruction == "AGENTS.md")
    {
        bail!("{OWNER_ROUTING_PATH} must route every path through root AGENTS.md");
    }

    for route in &metadata.path_routes {
        if route.path.is_some() == route.path_prefix.is_some() {
            bail!("each path route must declare exactly one of path or path_prefix");
        }
        if let Some(path) = route.path.as_deref() {
            validate_relative_path(path, "path route")?;
            if fs::symlink_metadata(root.join(path)).is_err() {
                bail!("exact path route {path} does not name a current path");
            }
        }
        if let Some(prefix) = route.path_prefix.as_deref() {
            validate_relative_path(prefix, "path prefix")?;
            if prefix.is_empty() || !prefix.ends_with('/') {
                bail!("path route prefix {prefix:?} must be non-empty and end with /");
            }
        }
        validate_route_values(
            route.owner_doc_ids.iter(),
            route.validation_classes.iter(),
            documents,
        )?;
    }
    let tracked_paths = &snapshot.tracked_paths;
    let document_by_path = documents
        .values()
        .flat_map(|document| {
            document
                .paths
                .iter()
                .map(move |path| (path.clone(), document.doc_id.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut exemption_paths = BTreeSet::new();
    for exemption in &metadata.tracked_path_exemptions {
        validate_relative_path(&exemption.path, "tracked path exemption")?;
        if exemption.reason.trim().is_empty() {
            bail!(
                "tracked path exemption {} must have a non-empty reason",
                exemption.path
            );
        }
        if !exemption_paths.insert(exemption.path.clone()) {
            bail!("tracked path exemption {} is duplicated", exemption.path);
        }
        if !tracked_paths.contains(&exemption.path) {
            bail!(
                "tracked path exemption {} does not name a current tracked path",
                exemption.path
            );
        }
        let classification = classify_path(
            &exemption.path,
            &document_by_path,
            packages,
            &metadata.path_routes,
            &[],
        );
        if classification.is_recognized() {
            bail!(
                "tracked path exemption {} is redundant because the path already has a maintained route",
                exemption.path
            );
        }
    }
    let unknown_tracked_paths = tracked_paths
        .iter()
        .filter(|path| {
            !classify_path(
                path,
                &document_by_path,
                packages,
                &metadata.path_routes,
                &metadata.tracked_path_exemptions,
            )
            .is_recognized()
        })
        .cloned()
        .collect::<Vec<_>>();
    if !unknown_tracked_paths.is_empty() {
        bail!(
            "{OWNER_ROUTING_PATH} has tracked path(s) without a maintained document, workspace package, explicit route, or justified current exemption: {}",
            unknown_tracked_paths.join(", ")
        );
    }
    let ci_trigger_policy: CiTriggerPolicy =
        serde_yaml::from_value(metadata.ci_trigger_policy.clone())
            .context("failed to parse current CI trigger policy")?;
    validate_ci_trigger_policy(root, &ci_trigger_policy)?;
    validate_package_route(&metadata.package_defaults, documents, &instruction_paths)?;
    for route in metadata.package_routes.values() {
        validate_package_route(route, documents, &instruction_paths)?;
    }
    let expected_packages = packages
        .iter()
        .map(|package| package.name().to_owned())
        .collect::<BTreeSet<_>>();
    let actual_packages = metadata
        .package_routes
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual_packages != expected_packages {
        bail!(
            "{OWNER_ROUTING_PATH} package_routes differ from Cargo workspace packages; expected {}, found {}",
            expected_packages.into_iter().collect::<Vec<_>>().join(", "),
            actual_packages.into_iter().collect::<Vec<_>>().join(", ")
        );
    }
    Ok(())
}

fn validate_ci_trigger_policy(root: &Path, policy: &CiTriggerPolicy) -> Result<()> {
    validate_relative_path(&policy.workflow, "CI trigger workflow")?;
    if !root.join(&policy.workflow).is_file() {
        bail!("CI trigger workflow {} does not exist", policy.workflow);
    }
    if policy.repository_changes != RepositoryChangeTrigger::All {
        bail!("CI trigger policy must cover all repository changes");
    }

    let workflow_path = root.join(&policy.workflow);
    let contents = fs::read_to_string(&workflow_path)
        .with_context(|| format!("failed to read {}", workflow_path.display()))?;
    let workflow: serde_yaml::Value = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse {}", workflow_path.display()))?;
    let pull_request = workflow_repository_change_policy(&workflow, "pull_request")?;
    let push = workflow_repository_change_policy(&workflow, "push")?;
    if pull_request != push {
        bail!(
            "{} pull_request and push repository-change policies differ",
            policy.workflow
        );
    }
    if pull_request != RepositoryChangeTrigger::All {
        bail!(
            "{} pull_request and push events must cover all repository changes",
            policy.workflow
        );
    }
    validate_ci_base_before_final(&workflow, &policy.workflow)?;
    Ok(())
}

fn workflow_repository_change_policy(
    workflow: &serde_yaml::Value,
    event: &str,
) -> Result<RepositoryChangeTrigger> {
    let events = workflow["on"]
        .as_mapping()
        .context("CI workflow must declare an event mapping")?;
    let event_key = serde_yaml::Value::String(event.to_owned());
    let configuration = events
        .get(&event_key)
        .with_context(|| format!("CI workflow must declare the {event} event"))?;
    if configuration.is_null() {
        return Ok(RepositoryChangeTrigger::All);
    }
    let mapping = configuration
        .as_mapping()
        .with_context(|| format!("CI workflow event {event} must be a mapping or null"))?;
    for filter in ["paths", "paths-ignore"] {
        if mapping.contains_key(serde_yaml::Value::String(filter.to_owned())) {
            bail!("CI workflow event {event} must not declare {filter}");
        }
    }
    Ok(RepositoryChangeTrigger::All)
}

fn validate_ci_base_before_final(workflow: &serde_yaml::Value, workflow_path: &str) -> Result<()> {
    let steps = workflow["jobs"]["checks"]["steps"]
        .as_sequence()
        .with_context(|| format!("{workflow_path} must declare jobs.checks.steps"))?;
    let base_steps = steps
        .iter()
        .enumerate()
        .filter(|(_, step)| step["id"].as_str() == Some("validation-base"))
        .filter_map(|(index, step)| step["run"].as_str().map(|run| (index, run)))
        .filter(|(_, run)| {
            [
                "cargo run",
                "-p xtask",
                "ci-base",
                "--event-name",
                "$GITHUB_EVENT_NAME",
                "--event-path",
                "$GITHUB_EVENT_PATH",
                "--head HEAD",
                "--github-output",
                "$GITHUB_OUTPUT",
            ]
            .iter()
            .all(|required| run.contains(required))
        })
        .collect::<Vec<_>>();
    if base_steps.len() != 1 {
        bail!("{workflow_path} must resolve one event-specific validation-base with ci-base");
    }
    let final_steps = steps
        .iter()
        .enumerate()
        .filter_map(|(index, step)| step["run"].as_str().map(|run| (index, run)))
        .filter(|(_, run)| {
            [
                "cargo run",
                "-p xtask",
                "validate final",
                "--base",
                "steps.validation-base.outputs.base",
            ]
            .iter()
            .all(|required| run.contains(required))
        })
        .collect::<Vec<_>>();
    if final_steps.len() != 1 {
        bail!(
            "{workflow_path} must run validate final once with the event-specific validation-base"
        );
    }
    if base_steps[0].0 >= final_steps[0].0 {
        bail!("{workflow_path} must resolve ci-base before validate final");
    }
    Ok(())
}

fn current_tracked_git_paths(root: &Path) -> Result<BTreeSet<String>> {
    let output = git_output(root, &["ls-files", "--cached", "-z", "--"])?;
    let mut paths = BTreeSet::new();
    for raw in output.split(|byte| *byte == 0) {
        if raw.is_empty() {
            continue;
        }
        let path = std::str::from_utf8(raw).context("Git returned a non-UTF-8 tracked path")?;
        validate_relative_path(path, "tracked path")?;
        if fs::symlink_metadata(root.join(path)).is_ok() {
            paths.insert(path.to_owned());
        }
    }
    Ok(paths)
}

fn revision_tracked_git_paths(root: &Path, revision: &str) -> Result<BTreeSet<String>> {
    let output = git_output(
        root,
        &["ls-tree", "-r", "--name-only", "-z", revision, "--"],
    )?;
    let mut paths = BTreeSet::new();
    for raw in output.split(|byte| *byte == 0) {
        if raw.is_empty() {
            continue;
        }
        let path = std::str::from_utf8(raw).context("Git returned a non-UTF-8 tracked path")?;
        validate_relative_path(path, "tracked path")?;
        paths.insert(path.to_owned());
    }
    Ok(paths)
}

fn validate_package_route(
    route: &PackageRoute,
    documents: &BTreeMap<String, DocumentRouteEntry>,
    instruction_paths: &BTreeSet<String>,
) -> Result<()> {
    for instruction in &route.instruction_paths {
        if !instruction_paths.contains(instruction) {
            bail!("package route refers to undeclared instruction {instruction}");
        }
    }
    validate_route_values(
        route.owner_doc_ids.iter(),
        route.validation_classes.iter(),
        documents,
    )
}

fn validate_route_values<'a>(
    owners: impl Iterator<Item = &'a String>,
    classes: impl Iterator<Item = &'a String>,
    documents: &BTreeMap<String, DocumentRouteEntry>,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for owner in owners {
        if !documents.contains_key(owner) {
            bail!("routing metadata refers to unknown owner document {owner}");
        }
        if !seen.insert(owner) {
            bail!("routing metadata duplicates owner document {owner}");
        }
    }
    let mut seen = BTreeSet::new();
    for class in classes {
        if !SUPPORTED_VALIDATION_CLASSES.contains(&class.as_str()) {
            bail!("routing metadata refers to unsupported validation class {class}");
        }
        if !seen.insert(class) {
            bail!("routing metadata duplicates validation class {class}");
        }
    }
    Ok(())
}

fn validate_relative_path(path: &str, label: &str) -> Result<()> {
    if path.is_empty() {
        bail!("{label} must not be empty");
    }
    let parsed = Path::new(path);
    if parsed.is_absolute()
        || parsed.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!("{label} {path:?} must be a safe repository-relative path");
    }
    Ok(())
}

fn ensure_git_repository_root(root: &Path) -> Result<()> {
    let output = git_output(root, &["rev-parse", "--show-toplevel"])?;
    let discovered = Path::new(std::str::from_utf8(&output)?.trim()).canonicalize()?;
    if discovered != root {
        bail!(
            "owner-route must run from the Git repository root {}; current root is {}",
            discovered.display(),
            root.display()
        );
    }
    Ok(())
}

fn resolve_commit(root: &Path, revision: &str) -> Result<String> {
    let expression = format!("{revision}^{{commit}}");
    let output = git_output(root, &["rev-parse", "--verify", &expression])?;
    Ok(std::str::from_utf8(&output)?.trim().to_owned())
}

fn load_revision_snapshot(root: &Path, revision: &str) -> Result<RoutingSnapshot> {
    let directory = tempfile::tempdir().context("failed to create temporary routing snapshot")?;
    let archive = git_output(root, &["archive", "--format=tar", revision])?;
    let mut extractor = Command::new("tar")
        .args(["-xf", "-", "-C"])
        .arg(directory.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to execute tar for temporary routing snapshot")?;
    extractor
        .stdin
        .take()
        .context("tar did not provide stdin")?
        .write_all(&archive)
        .context("failed to write temporary routing snapshot archive")?;
    let output = extractor
        .wait_with_output()
        .context("failed to wait for temporary routing snapshot extraction")?;
    if !output.status.success() {
        bail!(
            "failed to extract temporary routing snapshot: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    RoutingSnapshot::load(
        directory.path(),
        RoutingBasis::Base,
        revision_tracked_git_paths(root, revision)?,
    )
}

fn discover_repository_changes(root: &Path, comparison: &str) -> Result<Vec<RepositoryChange>> {
    let diff = git_output(
        root,
        &[
            "diff",
            "--name-status",
            "-z",
            "--find-renames",
            "--find-copies",
            "--find-copies-harder",
            comparison,
            "--",
        ],
    )?;
    let untracked = git_output(
        root,
        &["ls-files", "--others", "--exclude-standard", "-z", "--"],
    )?;
    let fields = diff
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(|field| {
            std::str::from_utf8(field)
                .context("Git returned a non-UTF-8 change entry")
                .map(str::to_owned)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut changes = BTreeSet::new();
    let mut index = 0;
    while index < fields.len() {
        let status = &fields[index];
        index += 1;
        let kind = status
            .chars()
            .next()
            .context("Git returned an empty change status")?;
        let path_count = if matches!(kind, 'R' | 'C') { 2 } else { 1 };
        if index + path_count > fields.len() {
            bail!("Git returned an incomplete {status} change entry");
        }
        let first = fields[index].clone();
        validate_relative_path(&first, "changed path")?;
        index += 1;
        let second = if path_count == 2 {
            let path = fields[index].clone();
            validate_relative_path(&path, "changed path")?;
            index += 1;
            Some(path)
        } else {
            None
        };
        changes.insert(repository_change(kind, first, second)?);
    }
    for raw in untracked.split(|byte| *byte == 0) {
        if raw.is_empty() {
            continue;
        }
        let path = std::str::from_utf8(raw).context("Git returned a non-UTF-8 changed path")?;
        validate_relative_path(path, "changed path")?;
        changes.insert(repository_change('A', path.to_owned(), None)?);
    }
    let mut changes = changes.into_iter().collect::<Vec<_>>();
    changes.sort_by(|left, right| {
        left.new_path
            .as_deref()
            .or(left.old_path.as_deref())
            .cmp(&right.new_path.as_deref().or(right.old_path.as_deref()))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.old_path.cmp(&right.old_path))
            .then_with(|| left.new_path.cmp(&right.new_path))
    });
    Ok(changes)
}

fn repository_change(
    status: char,
    first: String,
    second: Option<String>,
) -> Result<RepositoryChange> {
    let (kind, old_path, new_path, routing) = match status {
        'A' => (
            RepositoryChangeKind::Added,
            None,
            Some(first.clone()),
            vec![ChangeRoutingEndpoint {
                routing_basis: RoutingBasis::Current,
                path: first,
            }],
        ),
        'M' => (
            RepositoryChangeKind::Modified,
            Some(first.clone()),
            Some(first.clone()),
            paired_endpoints(first),
        ),
        'D' => (
            RepositoryChangeKind::Deleted,
            Some(first.clone()),
            None,
            vec![ChangeRoutingEndpoint {
                routing_basis: RoutingBasis::Base,
                path: first,
            }],
        ),
        'R' => {
            let current = second.context("Git rename entry has no destination")?;
            (
                RepositoryChangeKind::Renamed,
                Some(first.clone()),
                Some(current.clone()),
                vec![
                    ChangeRoutingEndpoint {
                        routing_basis: RoutingBasis::Base,
                        path: first,
                    },
                    ChangeRoutingEndpoint {
                        routing_basis: RoutingBasis::Current,
                        path: current,
                    },
                ],
            )
        }
        'C' => {
            let current = second.context("Git copy entry has no destination")?;
            (
                RepositoryChangeKind::Copied,
                Some(first),
                Some(current.clone()),
                vec![ChangeRoutingEndpoint {
                    routing_basis: RoutingBasis::Current,
                    path: current,
                }],
            )
        }
        'T' => (
            RepositoryChangeKind::TypeChanged,
            Some(first.clone()),
            Some(first.clone()),
            paired_endpoints(first),
        ),
        unsupported => bail!("unsupported Git change status {unsupported:?}"),
    };
    Ok(RepositoryChange {
        kind,
        old_path,
        new_path,
        routing,
    })
}

fn paired_endpoints(path: String) -> Vec<ChangeRoutingEndpoint> {
    vec![
        ChangeRoutingEndpoint {
            routing_basis: RoutingBasis::Base,
            path: path.clone(),
        },
        ChangeRoutingEndpoint {
            routing_basis: RoutingBasis::Current,
            path,
        },
    ]
}

fn git_output(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}
