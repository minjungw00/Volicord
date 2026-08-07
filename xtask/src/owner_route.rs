use crate::architecture::{derive_workspace_package_inputs, WorkspacePackageInput};
use crate::diagnostics::ValidationIssue;
use crate::repository::{normalize_existing_root, path_to_slash};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};
use std::process::Command;

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
    pub changed_paths: Vec<String>,
    pub instructions: Vec<RoutedInstruction>,
    pub workspace_packages: Vec<RoutedPackage>,
    pub maintained_documents: Vec<RoutedDocument>,
    pub owner_documents: Vec<RoutedOwnerDocument>,
    pub validation_classes: Vec<String>,
    pub unknown_paths: Vec<String>,
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
        render_list(&mut output, "changed paths", &self.changed_paths);
        render_list(
            &mut output,
            "instructions",
            &self
                .instructions
                .iter()
                .map(|item| format!("{} <- {}", item.path, item.reasons.join(", ")))
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
                        "{}; manifest={}; changed={}",
                        item.name,
                        item.manifest_path,
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
                        "{}; paths={}; changed={}; owner_area={}; summary={}; canonical_for={}; depends_on={}",
                        item.doc_id,
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
                        "{}; paths={}; reasons={}",
                        item.doc_id,
                        item.paths.join(", "),
                        item.reasons.join(", ")
                    )
                })
                .collect::<Vec<_>>(),
        );
        render_list(&mut output, "validation classes", &self.validation_classes);
        render_list(&mut output, "unknown paths", &self.unknown_paths);
        output
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RoutedInstruction {
    pub path: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RoutedPackage {
    pub name: String,
    pub manifest_path: String,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RoutedDocument {
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
    ci_trigger_policy: CiTriggerPolicy,
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
    paths: Vec<String>,
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

pub fn run_owner_route(root: &Path, base: Option<&str>) -> Result<OwnerRouteReport> {
    let root = normalize_existing_root(root)?;
    ensure_git_repository_root(&root)?;
    let base_revision = base
        .map(|revision| resolve_commit(&root, revision))
        .transpose()?;
    let changed_paths = discover_changed_paths(&root, base_revision.as_deref())?;
    route_paths(&root, base_revision, changed_paths)
}

pub(crate) fn validate_owner_routing(root: &Path, issues: &mut Vec<ValidationIssue>) {
    let result = (|| {
        let metadata = load_routing_metadata(root)?;
        let documents = load_document_routes(root)?;
        let packages = derive_workspace_package_inputs(root)?;
        validate_routing_metadata(root, &metadata, &documents, &packages)
    })();
    if let Err(error) = result {
        issues.push(ValidationIssue::new(
            OWNER_ROUTING_PATH,
            "metadata.owner_routing",
            format!("{error:#}"),
        ));
    }
}

fn route_paths(
    root: &Path,
    base_revision: Option<String>,
    changed_paths: Vec<String>,
) -> Result<OwnerRouteReport> {
    let metadata = load_routing_metadata(root)?;
    let documents = load_document_routes(root)?;
    let packages = derive_workspace_package_inputs(root)?;
    validate_routing_metadata(root, &metadata, &documents, &packages)?;

    let document_by_path = documents
        .values()
        .flat_map(|document| {
            document
                .paths
                .iter()
                .map(move |path| (path.clone(), document.doc_id.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut instruction_reasons = BTreeMap::<String, BTreeSet<String>>::new();
    let mut package_paths = BTreeMap::<String, BTreeSet<String>>::new();
    let mut document_paths = BTreeMap::<String, BTreeSet<String>>::new();
    let mut owner_reasons = BTreeMap::<String, BTreeSet<String>>::new();
    let mut validation_classes = BTreeSet::new();
    let mut unknown_paths = BTreeSet::new();

    for path in &changed_paths {
        let recognized = classify_path(
            path,
            &document_by_path,
            &packages,
            &metadata.path_routes,
            &metadata.tracked_path_exemptions,
        )
        .is_recognized();
        for scope in &metadata.instruction_scopes {
            if path_matches_prefix(path, &scope.path_prefix) {
                instruction_reasons
                    .entry(scope.instruction.clone())
                    .or_default()
                    .insert(format!("path:{path}"));
            }
        }
        if let Some(doc_id) = document_by_path.get(path) {
            document_paths
                .entry(doc_id.clone())
                .or_default()
                .insert(path.clone());
        }
        for package in &packages {
            if package_contains_path(package, path) {
                package_paths
                    .entry(package.name().to_owned())
                    .or_default()
                    .insert(path.clone());
            }
        }
        for route in &metadata.path_routes {
            if route.matches(path) {
                add_route(
                    &mut owner_reasons,
                    &mut validation_classes,
                    &route.owner_doc_ids,
                    &route.validation_classes,
                    &format!("path:{path}"),
                );
            }
        }
        if !recognized {
            unknown_paths.insert(path.clone());
        }
    }

    if !package_paths.is_empty() {
        add_package_route(
            &metadata.package_defaults,
            "workspace-package-default",
            &mut instruction_reasons,
            &mut owner_reasons,
            &mut validation_classes,
        );
    }
    for package in package_paths.keys() {
        let route = metadata
            .package_routes
            .get(package)
            .with_context(|| format!("routing metadata has no package route for {package}"))?;
        add_package_route(
            route,
            &format!("package:{package}"),
            &mut instruction_reasons,
            &mut owner_reasons,
            &mut validation_classes,
        );
    }

    let workspace_packages = packages
        .iter()
        .filter_map(|package| {
            package_paths
                .get(package.name())
                .map(|paths| RoutedPackage {
                    name: package.name().to_owned(),
                    manifest_path: package.manifest_path().to_owned(),
                    changed_paths: paths.iter().cloned().collect(),
                })
        })
        .collect();
    let maintained_documents = document_paths
        .into_iter()
        .map(|(doc_id, changed)| {
            let document = &documents[&doc_id];
            RoutedDocument {
                doc_id,
                paths: document.paths.clone(),
                changed_paths: changed.into_iter().collect(),
                owner_area: document.owner_area.clone(),
                summary: document.summary.clone(),
                canonical_for: document.canonical_for.clone(),
                depends_on: document.depends_on.clone(),
            }
        })
        .collect();
    let owner_documents = owner_reasons
        .into_iter()
        .map(|(doc_id, reasons)| {
            let document = &documents[&doc_id];
            RoutedOwnerDocument {
                doc_id,
                paths: document.paths.clone(),
                reasons: reasons.into_iter().collect(),
            }
        })
        .collect();
    let instructions = instruction_reasons
        .into_iter()
        .map(|(path, reasons)| RoutedInstruction {
            path,
            reasons: reasons.into_iter().collect(),
        })
        .collect();

    Ok(OwnerRouteReport {
        base_revision,
        changed_paths,
        instructions,
        workspace_packages,
        maintained_documents,
        owner_documents,
        validation_classes: validation_classes.into_iter().collect(),
        unknown_paths: unknown_paths.into_iter().collect(),
    })
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
    owner_reasons: &mut BTreeMap<String, BTreeSet<String>>,
    validation: &mut BTreeSet<String>,
    owners: &[String],
    classes: &[String],
    reason: &str,
) {
    for owner in owners {
        owner_reasons
            .entry(owner.clone())
            .or_default()
            .insert(reason.to_owned());
    }
    validation.extend(classes.iter().cloned());
}

fn add_package_route(
    route: &PackageRoute,
    reason: &str,
    instructions: &mut BTreeMap<String, BTreeSet<String>>,
    owners: &mut BTreeMap<String, BTreeSet<String>>,
    validation: &mut BTreeSet<String>,
) {
    for instruction in &route.instruction_paths {
        instructions
            .entry(instruction.clone())
            .or_default()
            .insert(reason.to_owned());
    }
    add_route(
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

fn validate_routing_metadata(
    root: &Path,
    metadata: &RoutingMetadata,
    documents: &BTreeMap<String, DocumentRouteEntry>,
    packages: &[WorkspacePackageInput],
) -> Result<()> {
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
    let tracked_paths = tracked_git_paths(root)?;
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
    validate_ci_trigger_policy(root, &metadata.ci_trigger_policy, &tracked_paths)?;
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

fn validate_ci_trigger_policy(
    root: &Path,
    policy: &CiTriggerPolicy,
    tracked_paths: &BTreeSet<String>,
) -> Result<()> {
    validate_relative_path(&policy.workflow, "CI trigger workflow")?;
    if !root.join(&policy.workflow).is_file() {
        bail!("CI trigger workflow {} does not exist", policy.workflow);
    }
    let mut canonical_paths = BTreeSet::new();
    for pattern in &policy.paths {
        validate_ci_trigger_pattern(pattern)?;
        if !canonical_paths.insert(pattern.clone()) {
            bail!("CI trigger path pattern {pattern:?} is duplicated");
        }
    }
    if canonical_paths.is_empty() {
        bail!("CI trigger policy must declare at least one path pattern");
    }
    let uncovered = tracked_paths
        .iter()
        .filter(|path| {
            !canonical_paths
                .iter()
                .any(|pattern| ci_trigger_matches(pattern, path))
        })
        .cloned()
        .collect::<Vec<_>>();
    if !uncovered.is_empty() {
        bail!(
            "CI trigger policy does not cover tracked path(s): {}",
            uncovered.join(", ")
        );
    }

    let workflow_path = root.join(&policy.workflow);
    let contents = fs::read_to_string(&workflow_path)
        .with_context(|| format!("failed to read {}", workflow_path.display()))?;
    let workflow: serde_yaml::Value = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse {}", workflow_path.display()))?;
    let pull_request = workflow_trigger_paths(&workflow, "pull_request")?;
    let push = workflow_trigger_paths(&workflow, "push")?;
    if pull_request != push {
        bail!(
            "{} pull_request and push path filters differ; pull_request={}, push={}",
            policy.workflow,
            pull_request.iter().cloned().collect::<Vec<_>>().join(", "),
            push.iter().cloned().collect::<Vec<_>>().join(", ")
        );
    }
    if pull_request != canonical_paths {
        bail!(
            "{} path filters differ from the canonical CI trigger policy; expected {}, found {}",
            policy.workflow,
            canonical_paths
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            pull_request.iter().cloned().collect::<Vec<_>>().join(", ")
        );
    }
    Ok(())
}

fn validate_ci_trigger_pattern(pattern: &str) -> Result<()> {
    if pattern == "*" {
        return Ok(());
    }
    let Some(prefix) = pattern.strip_suffix("**") else {
        bail!("CI trigger path pattern {pattern:?} must be `*` or end with `/**`");
    };
    if !prefix.ends_with('/')
        || prefix
            .chars()
            .any(|character| matches!(character, '*' | '?' | '[' | ']'))
    {
        bail!("CI trigger path pattern {pattern:?} must be `*` or end with `/**`");
    }
    validate_relative_path(prefix, "CI trigger path prefix")
}

fn ci_trigger_matches(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return !path.contains('/');
    }
    pattern
        .strip_suffix("**")
        .is_some_and(|prefix| path.starts_with(prefix))
}

fn workflow_trigger_paths(workflow: &serde_yaml::Value, event: &str) -> Result<BTreeSet<String>> {
    workflow["on"][event]["paths"]
        .as_sequence()
        .with_context(|| format!("CI workflow event {event} must declare path filters"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .with_context(|| format!("CI workflow event {event} has a non-string path filter"))
        })
        .collect()
}

fn tracked_git_paths(root: &Path) -> Result<BTreeSet<String>> {
    let output = git_output(root, &["ls-files", "-z", "--"])?;
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

fn discover_changed_paths(root: &Path, base: Option<&str>) -> Result<Vec<String>> {
    let comparison = base.unwrap_or("HEAD");
    let diff = git_output(
        root,
        &[
            "diff",
            "--name-only",
            "-z",
            "--no-renames",
            comparison,
            "--",
        ],
    )?;
    let untracked = git_output(
        root,
        &["ls-files", "--others", "--exclude-standard", "-z", "--"],
    )?;
    let mut paths = BTreeSet::new();
    for raw in diff
        .split(|byte| *byte == 0)
        .chain(untracked.split(|byte| *byte == 0))
    {
        if raw.is_empty() {
            continue;
        }
        let path = std::str::from_utf8(raw).context("Git returned a non-UTF-8 changed path")?;
        validate_relative_path(path, "changed path")?;
        paths.insert(path.to_owned());
    }
    Ok(paths.into_iter().collect())
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
