use super::{CommandInvocation, ValidationProfile};
use crate::architecture::{derive_workspace_package_inputs, WorkspacePackageInput};
use crate::owner_route::OwnerRouteReport;
use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

pub(crate) const RUN_DIRECTORY_PLACEHOLDER: &str = "{validation_run_directory}";

#[derive(Clone, Debug)]
pub(crate) enum CommandKind {
    Process,
    Internal {
        stdout: String,
        stderr: String,
        exit_code: i32,
    },
    ExactAggregate,
}

#[derive(Clone, Debug)]
pub(crate) struct CommandSpec {
    pub id: String,
    pub label: String,
    pub invocation: CommandInvocation,
    pub kind: CommandKind,
    pub decomposed: bool,
    pub aggregate_attempt: Option<u8>,
}

#[derive(Debug)]
pub(crate) struct ValidationPlan {
    pub base_revision: String,
    pub head_revision: String,
    pub changed_paths: Vec<String>,
    pub changed_packages: Vec<String>,
    pub validation_classes: Vec<String>,
    pub commands: Vec<CommandSpec>,
}

pub(crate) fn build_validation_plan(
    root: &Path,
    profile: ValidationProfile,
    route: OwnerRouteReport,
) -> Result<ValidationPlan> {
    let base_revision = route
        .base_revision
        .clone()
        .context("validation requires an explicit base revision")?;
    let head_revision = git_text(root, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let changed_packages = route
        .workspace_packages
        .iter()
        .map(|package| package.name.clone())
        .collect::<Vec<_>>();
    let validation_classes = route.validation_classes.clone();
    let scope_issues = commit_scope_issues(root, &base_revision)?;
    let mut raw = Vec::new();
    raw.push(internal_owner_routing_spec(root, &route.unknown_paths));
    raw.push(internal_commit_scope_spec(
        root,
        &base_revision,
        scope_issues,
    ));
    match profile {
        ValidationProfile::Focused => {
            raw.extend(focused_specs(
                root,
                &base_revision,
                &changed_packages,
                &validation_classes,
                &route.changed_paths,
            ));
        }
        ValidationProfile::Final => raw.extend(final_specs(root, &base_revision)),
    }
    let commands = assign_ids(deduplicate_specs(raw));

    Ok(ValidationPlan {
        base_revision,
        head_revision,
        changed_paths: route.changed_paths,
        changed_packages,
        validation_classes,
        commands,
    })
}

fn focused_specs(
    root: &Path,
    base: &str,
    changed_packages: &[String],
    validation_classes: &[String],
    changed_paths: &[String],
) -> Vec<CommandSpec> {
    let classes = validation_classes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let workspace_inputs_changed = changed_paths
        .iter()
        .any(|path| matches!(path.as_str(), "Cargo.toml" | "Cargo.lock"));
    let mut specs = Vec::new();
    if classes.contains("repository-hygiene") {
        specs.push(process(
            root,
            "changed diff hygiene",
            "git",
            &["diff", "--check", base, "--"],
        ));
    }
    if classes.contains("rust") {
        specs.push(process(
            root,
            "Rust formatting",
            "cargo",
            &["fmt", "--all", "--check"],
        ));
    }
    if classes.contains("architecture") || workspace_inputs_changed {
        specs.push(process(
            root,
            "workspace architecture",
            "cargo",
            &["run", "--locked", "-p", "xtask", "--", "architecture-check"],
        ));
    }
    if workspace_inputs_changed {
        specs.push(process(
            root,
            "workspace compilation",
            "cargo",
            &[
                "check",
                "--locked",
                "--workspace",
                "--all-targets",
                "--all-features",
            ],
        ));
    }
    if classes.contains("documentation") {
        specs.push(process(
            root,
            "documentation owners and generated drift",
            "cargo",
            &["run", "--locked", "-p", "xtask", "--", "docs-check"],
        ));
    }
    if classes.contains("mcp-spec") {
        specs.push(process(
            root,
            "pinned MCP specification fixtures",
            "cargo",
            &["run", "--locked", "-p", "xtask", "--", "mcp-spec-check"],
        ));
    }
    if classes.contains("release") || classes.contains("workflow") {
        specs.push(process(
            root,
            "release and workflow integrity",
            "cargo",
            &[
                "test",
                "--locked",
                "-p",
                "volicord-release-integrity-tests",
                "--all-targets",
                "--all-features",
            ],
        ));
    }
    if !changed_packages.is_empty() {
        let mut clippy_args = vec!["clippy".to_owned(), "--locked".to_owned()];
        for package in changed_packages {
            clippy_args.extend(["-p".to_owned(), package.clone()]);
        }
        clippy_args.extend([
            "--all-targets".to_owned(),
            "--all-features".to_owned(),
            "--".to_owned(),
            "-D".to_owned(),
            "warnings".to_owned(),
        ]);
        specs.push(process_owned(
            root,
            "changed-package lint",
            "cargo",
            clippy_args,
        ));

        let mut test_args = vec!["test".to_owned(), "--locked".to_owned()];
        for package in changed_packages {
            test_args.extend(["-p".to_owned(), package.clone()]);
        }
        test_args.extend(["--all-targets".to_owned(), "--all-features".to_owned()]);
        specs.push(process_owned(
            root,
            "changed-package tests",
            "cargo",
            test_args,
        ));
    }
    specs
}

fn final_specs(root: &Path, base: &str) -> Vec<CommandSpec> {
    let source_bundle_output = format!("{RUN_DIRECTORY_PLACEHOLDER}/volicord-source.zip");
    let binary = if cfg!(windows) {
        "target/debug/volicord.exe"
    } else {
        "target/debug/volicord"
    };
    let mut specs = vec![
        process(
            root,
            "change-series diff hygiene",
            "git",
            &["diff", "--check", base, "--"],
        ),
        process(
            root,
            "Rust formatting",
            "cargo",
            &["fmt", "--all", "--check"],
        ),
        process(
            root,
            "workspace architecture",
            "cargo",
            &["run", "--locked", "-p", "xtask", "--", "architecture-check"],
        ),
        process(
            root,
            "workspace lint",
            "cargo",
            &[
                "clippy",
                "--locked",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        ),
        process(
            root,
            "documentation owners and generated drift",
            "cargo",
            &["run", "--locked", "-p", "xtask", "--", "docs-check"],
        ),
        process_owned(
            root,
            "committed source bundle",
            "cargo",
            vec![
                "run".to_owned(),
                "--locked".to_owned(),
                "-p".to_owned(),
                "xtask".to_owned(),
                "--".to_owned(),
                "source-bundle".to_owned(),
                "--output".to_owned(),
                source_bundle_output,
            ],
        ),
        process(
            root,
            "pinned MCP specification fixtures",
            "cargo",
            &["run", "--locked", "-p", "xtask", "--", "mcp-spec-check"],
        ),
        process(
            root,
            "maintainability report",
            "cargo",
            &[
                "run",
                "--locked",
                "-p",
                "xtask",
                "--",
                "maintainability-report",
            ],
        ),
        process(
            root,
            "registry-driven MCP protocol conformance",
            "cargo",
            &[
                "test",
                "--locked",
                "-p",
                "volicord-mcp",
                "--test",
                "protocol_conformance",
            ],
        ),
        process(
            root,
            "public contract snapshots",
            "cargo",
            &[
                "test",
                "--locked",
                "-p",
                "volicord-integration-tests",
                "--test",
                "public_contract_snapshots",
            ],
        ),
        process(
            root,
            "storage DDL contract",
            "cargo",
            &[
                "test",
                "--locked",
                "-p",
                "volicord-store",
                "--test",
                "storage_ddl_contract",
            ],
        ),
        process(
            root,
            "MCP stdio contract",
            "cargo",
            &[
                "test",
                "--locked",
                "-p",
                "volicord-cli",
                "--test",
                "mcp_transport",
            ],
        ),
        process(
            root,
            "MCP Agent Connection contract",
            "cargo",
            &[
                "test",
                "--locked",
                "-p",
                "volicord-integration-tests",
                "--test",
                "mcp_connection",
            ],
        ),
        process(
            root,
            "local Volicord binary build",
            "cargo",
            &[
                "build",
                "--locked",
                "-p",
                "volicord-cli",
                "--bin",
                "volicord",
            ],
        ),
        process(
            root,
            "local Volicord binary smoke",
            "cargo",
            &[
                "run",
                "--locked",
                "-p",
                "volicord-release-smoke",
                "--",
                "--bin",
                binary,
            ],
        ),
        process(
            root,
            "release integrity",
            "cargo",
            &[
                "test",
                "--locked",
                "-p",
                "volicord-release-integrity-tests",
                "--all-targets",
                "--all-features",
            ],
        ),
    ];
    let mut aggregate = process(
        root,
        "exact workspace aggregate",
        "cargo",
        &[
            "test",
            "--locked",
            "--workspace",
            "--all-targets",
            "--all-features",
        ],
    );
    aggregate.kind = CommandKind::ExactAggregate;
    aggregate.aggregate_attempt = Some(1);
    specs.push(aggregate);
    specs
}

fn internal_owner_routing_spec(root: &Path, unknown_paths: &[String]) -> CommandSpec {
    let (stdout, stderr, exit_code) = if unknown_paths.is_empty() {
        (
            "owner-routing coverage check passed\n".to_owned(),
            String::new(),
            0,
        )
    } else {
        (
            String::new(),
            format!(
                "focused validation cannot route changed path(s): {}. Add or correct a maintained path route in docs/owner-routing.yaml before relying on focused validation.\n",
                unknown_paths.join(", ")
            ),
            1,
        )
    };
    CommandSpec {
        id: String::new(),
        label: "owner-routing coverage".to_owned(),
        invocation: CommandInvocation {
            program: "repository-policy".to_owned(),
            args: vec!["owner-routing-coverage".to_owned()],
            working_directory: root.display().to_string(),
        },
        kind: CommandKind::Internal {
            stdout,
            stderr,
            exit_code,
        },
        decomposed: false,
        aggregate_attempt: None,
    }
}

fn internal_commit_scope_spec(root: &Path, base: &str, issues: Vec<String>) -> CommandSpec {
    let (stdout, stderr, exit_code) = if issues.is_empty() {
        (
            "commit-type scope check passed\n".to_owned(),
            String::new(),
            0,
        )
    } else {
        (String::new(), format!("{}\n", issues.join("\n")), 1)
    };
    CommandSpec {
        id: String::new(),
        label: "commit-type scope".to_owned(),
        invocation: CommandInvocation {
            program: "repository-policy".to_owned(),
            args: vec![
                "commit-scope".to_owned(),
                "--base".to_owned(),
                base.to_owned(),
            ],
            working_directory: root.display().to_string(),
        },
        kind: CommandKind::Internal {
            stdout,
            stderr,
            exit_code,
        },
        decomposed: false,
        aggregate_attempt: None,
    }
}

pub(crate) fn process(root: &Path, label: &str, program: &str, args: &[&str]) -> CommandSpec {
    process_owned(
        root,
        label,
        program,
        args.iter().map(|value| (*value).to_owned()).collect(),
    )
}

pub(crate) fn process_owned(
    root: &Path,
    label: &str,
    program: &str,
    args: Vec<String>,
) -> CommandSpec {
    CommandSpec {
        id: String::new(),
        label: label.to_owned(),
        invocation: CommandInvocation {
            program: program.to_owned(),
            args,
            working_directory: root.display().to_string(),
        },
        kind: CommandKind::Process,
        decomposed: false,
        aggregate_attempt: None,
    }
}

pub(crate) fn assign_dynamic_id(spec: &mut CommandSpec, index: usize) {
    spec.id = format!("{index:03}-{}", slug(&spec.label));
}

fn assign_ids(mut specs: Vec<CommandSpec>) -> Vec<CommandSpec> {
    for (index, spec) in specs.iter_mut().enumerate() {
        assign_dynamic_id(spec, index + 1);
    }
    specs
}

fn deduplicate_specs(specs: Vec<CommandSpec>) -> Vec<CommandSpec> {
    let mut seen = BTreeSet::new();
    specs
        .into_iter()
        .filter(|spec| {
            if matches!(spec.kind, CommandKind::Internal { .. }) {
                return true;
            }
            seen.insert((
                spec.invocation.program.clone(),
                spec.invocation.args.clone(),
            ))
        })
        .collect()
}

fn slug(value: &str) -> String {
    let mut slug = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_owned()
}

fn commit_scope_issues(root: &Path, base: &str) -> Result<Vec<String>> {
    let packages = derive_workspace_package_inputs(root)?;
    let production = architecture_production_map(root, &packages)?;
    let commits = git_lines(root, &["rev-list", "--reverse", &format!("{base}..HEAD")])?;
    let mut issues = Vec::new();
    for commit in commits {
        let subject = git_text(root, &["show", "-s", "--format=%s", &commit])?;
        let paths = git_nul(
            root,
            &[
                "diff-tree",
                "--no-commit-id",
                "--name-only",
                "-r",
                "-z",
                "--no-renames",
                &commit,
            ],
        )?;
        let manifest_test_only =
            production_manifest_classifications(root, &commit, &paths, &packages, &production)?;
        for issue in scope_issues_for_commit(
            &subject,
            &paths,
            &packages,
            &production,
            &manifest_test_only,
        ) {
            issues.push(format!("commit {commit} ({subject}): {issue}"));
        }
    }
    Ok(issues)
}

fn scope_issues_for_commit(
    subject: &str,
    paths: &[String],
    packages: &[WorkspacePackageInput],
    production: &BTreeMap<String, bool>,
    manifest_test_only: &BTreeMap<String, bool>,
) -> Vec<String> {
    let Some(commit_type) = conventional_commit_type(subject) else {
        return Vec::new();
    };
    if commit_type == "docs" {
        return paths
            .iter()
            .filter(|path| docs_commit_changes_implementation(path))
            .map(|path| format!("docs: commit changes implementation path {path}"))
            .collect();
    }
    if commit_type == "test" {
        let non_lock = paths
            .iter()
            .filter(|path| path.as_str() != "Cargo.lock")
            .collect::<Vec<_>>();
        let mut issues = non_lock
            .iter()
            .filter(|path| {
                !test_commit_path_scope(path, packages, production, manifest_test_only).allowed
            })
            .map(|path| format!("test: commit changes production path {path}"))
            .collect::<Vec<_>>();
        if paths.iter().any(|path| path == "Cargo.lock")
            && (non_lock.is_empty()
                || non_lock.iter().any(|path| {
                    !test_commit_path_scope(path, packages, production, manifest_test_only)
                        .test_only
                }))
        {
            issues.push(
                "test: commit changes Cargo.lock without only associated test-target, test-path, dev-dependency, or non-production-package changes"
                    .to_owned(),
            );
        }
        return issues;
    }
    Vec::new()
}

fn conventional_commit_type(subject: &str) -> Option<&str> {
    let (header, description) = subject.split_once(": ")?;
    if description.trim().is_empty() {
        return None;
    }
    let header = header.strip_suffix('!').unwrap_or(header);
    let (commit_type, has_scope) = match header.split_once('(') {
        Some((commit_type, scope)) => {
            let scope = scope.strip_suffix(')')?;
            if scope.is_empty()
                || scope
                    .chars()
                    .any(|character| matches!(character, '(' | ')' | ':'))
            {
                return None;
            }
            (commit_type, true)
        }
        None => (header, false),
    };
    if commit_type.is_empty()
        || !commit_type.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        || (!has_scope && header.contains(')'))
    {
        return None;
    }
    Some(commit_type)
}

fn docs_commit_changes_implementation(path: &str) -> bool {
    path == "Cargo.toml"
        || path == "Cargo.lock"
        || path == "Dockerfile"
        || path == "Dockerfile.release"
        || path.starts_with("crates/")
        || path.starts_with("tests/")
        || path.starts_with("xtask/")
        || path.starts_with("scripts/")
        || path.starts_with(".github/")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TestCommitPathScope {
    allowed: bool,
    test_only: bool,
}

fn test_commit_path_scope(
    path: &str,
    packages: &[WorkspacePackageInput],
    production: &BTreeMap<String, bool>,
    manifest_test_only: &BTreeMap<String, bool>,
) -> TestCommitPathScope {
    if is_test_only_path(path) {
        return TestCommitPathScope {
            allowed: true,
            test_only: true,
        };
    }
    let package = packages
        .iter()
        .find(|package| package_contains(package, path));
    match package {
        Some(package) if !production.get(package.name()).copied().unwrap_or(true) => {
            TestCommitPathScope {
                allowed: true,
                test_only: true,
            }
        }
        Some(package) if path == package.manifest_path() => {
            let test_only = manifest_test_only.get(path).copied().unwrap_or(false);
            TestCommitPathScope {
                allowed: test_only,
                test_only,
            }
        }
        Some(_) => TestCommitPathScope {
            allowed: false,
            test_only: false,
        },
        None => TestCommitPathScope {
            allowed: !(matches!(
                path,
                "Cargo.toml" | "Cargo.lock" | "Dockerfile" | "Dockerfile.release"
            ) || path.starts_with("scripts/")
                || path.starts_with(".github/")),
            test_only: false,
        },
    }
}

fn production_manifest_classifications(
    root: &Path,
    commit: &str,
    paths: &[String],
    packages: &[WorkspacePackageInput],
    production: &BTreeMap<String, bool>,
) -> Result<BTreeMap<String, bool>> {
    let mut classifications = BTreeMap::new();
    for package in packages {
        let path = package.manifest_path();
        if !paths.iter().any(|changed| changed == path)
            || !production.get(package.name()).copied().unwrap_or(true)
        {
            continue;
        }
        let parent = format!("{commit}^");
        let before = git_file(root, &parent, path)?;
        let after = git_file(root, commit, path)?;
        let test_only = match (before, after) {
            (Some(before), Some(after)) => manifest_change_is_test_only(&before, &after)
                .with_context(|| format!("failed to classify test commit manifest {path}"))?,
            _ => false,
        };
        classifications.insert(path.to_owned(), test_only);
    }
    Ok(classifications)
}

fn manifest_change_is_test_only(before: &str, after: &str) -> Result<bool> {
    let mut before = toml_edit::de::from_str::<serde_json::Value>(before)?;
    let mut after = toml_edit::de::from_str::<serde_json::Value>(after)?;
    remove_test_only_manifest_fields(&mut before);
    remove_test_only_manifest_fields(&mut after);
    Ok(before == after)
}

fn remove_test_only_manifest_fields(value: &mut serde_json::Value) {
    let Some(root) = value.as_object_mut() else {
        return;
    };
    root.remove("dev-dependencies");
    root.remove("test");
    let remove_target = if let Some(targets) = root
        .get_mut("target")
        .and_then(serde_json::Value::as_object_mut)
    {
        targets.retain(|_, target| {
            if let Some(table) = target.as_object_mut() {
                table.remove("dev-dependencies");
                !table.is_empty()
            } else {
                true
            }
        });
        targets.is_empty()
    } else {
        false
    };
    if remove_target {
        root.remove("target");
    }
}

fn git_file(root: &Path, revision: &str, path: &str) -> Result<Option<String>> {
    let object = format!("{revision}:{path}");
    let output = Command::new("git")
        .current_dir(root)
        .args(["show", &object])
        .output()
        .with_context(|| format!("failed to execute git show {object}"))?;
    if output.status.success() {
        return Ok(Some(String::from_utf8(output.stdout)?));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("does not exist in") || stderr.contains("exists on disk, but not in") {
        return Ok(None);
    }
    bail!("git show {object} failed: {}", stderr.trim())
}

fn is_test_only_path(path: &str) -> bool {
    path.starts_with("tests/")
        || path.contains("/tests/")
        || path.contains("/fixtures/")
        || path.contains("/snapshots/")
        || path.ends_with("/tests.rs")
        || path.ends_with("_test.rs")
        || path.ends_with("_tests.rs")
}

fn package_contains(package: &WorkspacePackageInput, path: &str) -> bool {
    if path == package.manifest_path() {
        return true;
    }
    let Some(parent) = Path::new(package.manifest_path()).parent() else {
        return false;
    };
    let prefix = parent.to_string_lossy().replace('\\', "/") + "/";
    path.starts_with(&prefix)
}

fn architecture_production_map(
    root: &Path,
    packages: &[WorkspacePackageInput],
) -> Result<BTreeMap<String, bool>> {
    let contents = fs::read_to_string(root.join("Cargo.toml"))?;
    let manifest = contents.parse::<toml_edit::DocumentMut>()?;
    let table = manifest["workspace"]["metadata"]["architecture"]["packages"]
        .as_table()
        .context("Cargo.toml is missing workspace.metadata.architecture.packages")?;
    let mut result = BTreeMap::new();
    for package in packages {
        let production = table[package.name()]["production"]
            .as_bool()
            .with_context(|| {
                format!(
                    "Cargo.toml architecture metadata has no production flag for {}",
                    package.name()
                )
            })?;
        result.insert(package.name().to_owned(), production);
    }
    Ok(result)
}

fn git_text(root: &Path, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8(git_bytes(root, args)?)?.trim().to_owned())
}

fn git_nul(root: &Path, args: &[&str]) -> Result<Vec<String>> {
    git_bytes(root, args)?
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| Ok(std::str::from_utf8(part)?.to_owned()))
        .collect()
}

fn git_lines(root: &Path, args: &[&str]) -> Result<Vec<String>> {
    Ok(String::from_utf8(git_bytes(root, args)?)?
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

fn git_bytes(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn packages() -> Vec<WorkspacePackageInput> {
        vec![WorkspacePackageInput::for_validation_test(
            "production",
            "crates/production/Cargo.toml",
        )]
    }

    #[test]
    fn focused_plan_selects_changed_packages_without_workspace_aggregate() {
        let root = Path::new("/repository");
        let specs = focused_specs(
            root,
            "base",
            &["xtask".to_owned(), "volicord-types".to_owned()],
            &[
                "documentation".to_owned(),
                "repository-hygiene".to_owned(),
                "rust".to_owned(),
            ],
            &["xtask/src/validation/plan.rs".to_owned()],
        );
        let invocations = specs
            .iter()
            .map(|spec| spec.invocation.args.join(" "))
            .collect::<Vec<_>>();
        assert!(invocations
            .iter()
            .any(|args| args == "run --locked -p xtask -- docs-check"));
        assert!(invocations.iter().any(|args| {
            args.contains("-p xtask -p volicord-types") && args.starts_with("test ")
        }));
        assert!(!invocations.iter().any(|args| args.contains("--workspace")));
    }

    #[test]
    fn final_plan_contains_one_exact_aggregate_and_no_global_serialization() {
        let specs = final_specs(Path::new("/repository"), "base");
        let aggregates = specs
            .iter()
            .filter(|spec| matches!(spec.kind, CommandKind::ExactAggregate))
            .collect::<Vec<_>>();
        assert_eq!(aggregates.len(), 1);
        assert_eq!(
            aggregates[0].invocation.args.join(" "),
            "test --locked --workspace --all-targets --all-features"
        );
        assert!(specs.iter().all(|spec| {
            let command = spec.invocation.args.join(" ");
            !command.contains("--test-threads=1") && !command.contains("RUST_TEST_THREADS")
        }));
    }

    #[test]
    fn commit_type_scope_rejects_production_changes_but_accepts_tests() {
        let packages = packages();
        let production = BTreeMap::from([("production".to_owned(), true)]);
        let manifests = BTreeMap::new();
        assert!(scope_issues_for_commit(
            "test(core)!: add cases",
            &["crates/production/src/lib.rs".to_owned()],
            &packages,
            &production,
            &manifests,
        )
        .iter()
        .any(|issue| issue.contains("production path")));
        assert!(scope_issues_for_commit(
            "test(core): add cases",
            &["crates/production/tests/case.rs".to_owned()],
            &packages,
            &production,
            &manifests,
        )
        .is_empty());
        assert!(scope_issues_for_commit(
            "docs(validation)!: update policy",
            &["crates/production/src/lib.rs".to_owned()],
            &packages,
            &production,
            &manifests,
        )
        .iter()
        .any(|issue| issue.contains("implementation path")));
    }

    #[test]
    fn conventional_commit_subject_parser_accepts_scoped_and_breaking_forms() {
        assert_eq!(
            conventional_commit_type("test(scope): cover it"),
            Some("test")
        );
        assert_eq!(conventional_commit_type("docs!: explain it"), Some("docs"));
        assert_eq!(
            conventional_commit_type("fix(scope)!: repair it"),
            Some("fix")
        );
        assert_eq!(conventional_commit_type("test: "), None);
        assert_eq!(conventional_commit_type("test(scope: broken"), None);
    }

    #[test]
    fn production_manifest_allows_only_test_targets_and_dev_dependencies() {
        let before = r#"[package]
name = "production"
version = "0.1.0"

[dependencies]
serde = "1"
"#;
        let test_only = r#"[package]
name = "production"
version = "0.1.0"

[dependencies]
serde = "1"

[dev-dependencies]
tempfile = "3"

[[test]]
name = "contract"
path = "tests/contract.rs"
"#;
        let runtime = r#"[package]
name = "production"
version = "0.1.0"

[dependencies]
serde = "1"
anyhow = "1"
"#;
        assert!(manifest_change_is_test_only(before, test_only).expect("classify test manifest"));
        assert!(!manifest_change_is_test_only(before, runtime).expect("classify runtime manifest"));
    }

    #[test]
    fn lockfile_requires_only_associated_test_changes() {
        let packages = packages();
        let production = BTreeMap::from([("production".to_owned(), true)]);
        let manifests = BTreeMap::from([("crates/production/Cargo.toml".to_owned(), true)]);
        assert!(scope_issues_for_commit(
            "test(deps): add test dependency",
            &[
                "Cargo.lock".to_owned(),
                "crates/production/Cargo.toml".to_owned(),
            ],
            &packages,
            &production,
            &manifests,
        )
        .is_empty());
        assert!(scope_issues_for_commit(
            "test(deps): update lock",
            &["Cargo.lock".to_owned(), "docs/note.md".to_owned()],
            &packages,
            &production,
            &manifests,
        )
        .iter()
        .any(|issue| issue.contains("without only associated")));
    }

    #[test]
    fn workspace_inputs_select_architecture_and_compilation_without_test_aggregate() {
        for path in ["Cargo.toml", "Cargo.lock"] {
            let specs = focused_specs(
                Path::new("/repository"),
                "base",
                &[],
                &["rust".to_owned()],
                &[path.to_owned()],
            );
            let invocations = specs
                .iter()
                .map(|spec| spec.invocation.args.join(" "))
                .collect::<Vec<_>>();
            assert!(invocations
                .iter()
                .any(|args| args == "run --locked -p xtask -- architecture-check"));
            assert!(invocations
                .iter()
                .any(|args| args == "check --locked --workspace --all-targets --all-features"));
            assert!(!invocations
                .iter()
                .any(|args| args.starts_with("test --locked --workspace")));
        }
    }

    #[test]
    fn unknown_paths_create_an_actionable_failed_preflight() {
        let spec = internal_owner_routing_spec(
            Path::new("/repository"),
            &["unrouted/input.bin".to_owned()],
        );
        let CommandKind::Internal {
            stderr, exit_code, ..
        } = spec.kind
        else {
            panic!("routing preflight must be internal");
        };
        assert_eq!(exit_code, 1);
        assert!(stderr.contains("docs/owner-routing.yaml"));
        assert!(stderr.contains("unrouted/input.bin"));
    }

    #[test]
    fn focused_plan_order_is_deterministic() {
        let changed = vec!["Cargo.lock".to_owned(), "Cargo.toml".to_owned()];
        let first = focused_specs(
            Path::new("/repository"),
            "base",
            &[],
            &["rust".to_owned(), "architecture".to_owned()],
            &changed,
        );
        let second = focused_specs(
            Path::new("/repository"),
            "base",
            &[],
            &["architecture".to_owned(), "rust".to_owned()],
            &changed,
        );
        assert_eq!(
            first
                .iter()
                .map(|spec| (&spec.label, &spec.invocation.args))
                .collect::<Vec<_>>(),
            second
                .iter()
                .map(|spec| (&spec.label, &spec.invocation.args))
                .collect::<Vec<_>>()
        );
    }
}
