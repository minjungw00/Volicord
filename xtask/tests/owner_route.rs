use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

struct Fixture {
    _directory: TempDir,
    root: PathBuf,
    base: String,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary repository");
        let root = directory.path().to_path_buf();
        write(
            &root,
            "Cargo.toml",
            r#"[workspace]
members = ["crates/sample"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.85"
"#,
        );
        write(
            &root,
            "crates/sample/Cargo.toml",
            r#"[package]
name = "sample"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
"#,
        );
        write(&root, "crates/sample/src/lib.rs", "pub fn sample() {}\n");
        write(&root, "AGENTS.md", "# Root rules\n");
        write(&root, "crates/AGENTS.md", "# Rust rules\n");
        write(&root, "docs/AGENTS.md", "# Documentation rules\n");
        write(&root, "docs/en/maintain/validation.md", "# Validation\n");
        write(&root, "docs/ko/maintain/validation.md", "# 검증\n");
        write(&root, "docs/en/maintain/other.md", "# Other owner\n");
        write(&root, "docs/ko/maintain/other.md", "# 다른 담당\n");
        write(&root, "docs/sample.txt", "sample route\n");
        write(&root, "docs/delete.txt", "delete route\n");
        write(&root, "docs/rename.txt", "rename route\n");
        write(&root, "docs/copy-source.txt", "copy route\n");
        write(&root, "docs/type.txt", "type route\n");
        write(
            &root,
            "docs/doc-index.yaml",
            r#"shared_documents:
  - doc_id: agents.root
    path: AGENTS.md
    summary: Root rules.
    owner_area: repository_guidance
  - doc_id: agents.crates
    path: crates/AGENTS.md
    summary: Rust rules.
    owner_area: repository_guidance
  - doc_id: agents.docs
    path: docs/AGENTS.md
    summary: Documentation rules.
    owner_area: repository_guidance
documents:
  - doc_id: maintain.validation
    path_en: docs/en/maintain/validation.md
    path_ko: docs/ko/maintain/validation.md
    summary: Validation owner.
    owner_area: documentation_maintenance
    canonical_for:
      - repository validation
  - doc_id: maintain.other
    path_en: docs/en/maintain/other.md
    path_ko: docs/ko/maintain/other.md
    summary: Other owner.
    owner_area: documentation_maintenance
"#,
        );
        write(
            &root,
            "docs/owner-routing.yaml",
            r#"validation_classes:
  architecture: Architecture checks.
  documentation: Documentation checks.
  mcp-spec: MCP checks.
  release: Release checks.
  repository-hygiene: Hygiene checks.
  rust: Rust checks.
  workflow: Workflow checks.
instruction_scopes:
  - path_prefix: ""
    instruction: AGENTS.md
  - path_prefix: "crates/"
    instruction: crates/AGENTS.md
  - path_prefix: "docs/"
    instruction: docs/AGENTS.md
path_routes:
  - path: AGENTS.md
    owner_doc_ids: [maintain.validation]
    validation_classes: [documentation, repository-hygiene]
  - path: Cargo.lock
    owner_doc_ids: [maintain.validation]
    validation_classes: [repository-hygiene, rust]
  - path: Cargo.toml
    owner_doc_ids: [maintain.validation]
    validation_classes: [architecture, repository-hygiene, rust]
  - path: .dockerignore
    owner_doc_ids: [maintain.validation]
    validation_classes: [release, repository-hygiene]
  - path: .gitattributes
    owner_doc_ids: [maintain.validation]
    validation_classes: [release, repository-hygiene]
  - path: Dockerfile
    owner_doc_ids: [maintain.validation]
    validation_classes: [release, repository-hygiene, workflow]
  - path: Dockerfile.release
    owner_doc_ids: [maintain.validation]
    validation_classes: [release, repository-hygiene, workflow]
  - path_prefix: ".github/"
    owner_doc_ids: [maintain.validation]
    validation_classes: [workflow]
  - path_prefix: "docs/"
    owner_doc_ids: [maintain.validation]
    validation_classes: [documentation]
tracked_path_exemptions: []
ci_trigger_policy:
  workflow: .github/workflows/ci.yml
  repository_changes: all
package_defaults:
  instruction_paths: [crates/AGENTS.md]
  owner_doc_ids: [maintain.validation]
  validation_classes: [repository-hygiene, rust]
package_routes:
  sample:
    owner_doc_ids: [maintain.validation]
    validation_classes: [architecture]
"#,
        );
        write(&root, ".dockerignore", "target\n");
        write(&root, ".gitattributes", "* text=auto\n");
        write(&root, "Dockerfile", "FROM scratch\n");
        write(&root, "Dockerfile.release", "FROM scratch\n");
        write(
            &root,
            ".github/workflows/ci.yml",
            r#"name: CI
on:
  pull_request:
  push:
    branches: [main]
jobs:
  checks:
    steps:
      - id: validation-base
        run: cargo run -p xtask -- ci-base --event-name "$GITHUB_EVENT_NAME" --event-path "$GITHUB_EVENT_PATH" --head HEAD --github-output "$GITHUB_OUTPUT"
      - run: cargo run -p xtask -- validate final --base "${{ steps.validation-base.outputs.base }}"
"#,
        );

        git(&root, &["init", "-q"]);
        git(
            &root,
            &["config", "user.email", "owner-route@example.invalid"],
        );
        git(&root, &["config", "user.name", "Owner Route Test"]);
        command(&root, "cargo", &["generate-lockfile"]);
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "fixture: base"]);
        let base = git_output(&root, &["rev-parse", "HEAD"]);

        Self {
            _directory: directory,
            root,
            base,
        }
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn status(&self) -> Vec<u8> {
        git_bytes(self.root(), &["status", "--porcelain", "-z"])
    }

    fn worktrees(&self) -> Vec<u8> {
        git_bytes(self.root(), &["worktree", "list", "--porcelain", "-z"])
    }
}

#[test]
fn tracked_root_maintenance_files_have_owner_and_validation_routes() {
    let fixture = Fixture::new();
    for path in [
        ".dockerignore",
        ".gitattributes",
        "Dockerfile",
        "Dockerfile.release",
    ] {
        write(fixture.root(), path, "changed\n");
    }

    let report = xtask::run_owner_route(fixture.root(), None).expect("route root maintenance");
    let human = report.render_human();
    let json = serde_json::to_string_pretty(&report).expect("render JSON");

    assert_eq!(
        report.changed_paths(),
        [
            ".dockerignore",
            ".gitattributes",
            "Dockerfile",
            "Dockerfile.release",
        ]
    );
    assert!(report.unknown_paths.is_empty());
    assert!(!report.owner_documents.is_empty());
    assert!(report.validation_classes.contains(&"release".to_owned()));
    assert!(report
        .validation_classes
        .contains(&"repository-hygiene".to_owned()));
    assert!(report.validation_classes.contains(&"workflow".to_owned()));
    for path in &report.changed_paths() {
        assert!(human.contains(path));
        assert!(json.contains(path));
    }
}

#[test]
fn tracked_unrouted_paths_fail_metadata_validation_in_stable_path_order() {
    let fixture = Fixture::new();
    write(fixture.root(), "z-root-maintenance", "z\n");
    write(fixture.root(), "a-root-maintenance", "a\n");
    git(
        fixture.root(),
        &["add", "z-root-maintenance", "a-root-maintenance"],
    );

    let error = xtask::run_owner_route(fixture.root(), None)
        .expect_err("tracked unrouted paths must fail")
        .to_string();
    assert!(error.contains(
        "tracked path(s) without a maintained document, workspace package, explicit route, or justified current exemption: a-root-maintenance, z-root-maintenance"
    ));
}

#[test]
fn newly_routed_top_level_directory_needs_no_ci_filter_metadata() {
    let fixture = Fixture::new();
    let routing_path = fixture.root().join("docs/owner-routing.yaml");
    let routing = fs::read_to_string(&routing_path).expect("read routing metadata");
    fs::write(
        &routing_path,
        routing.replace(
            "tracked_path_exemptions: []",
            r#"  - path_prefix: "future/"
    owner_doc_ids: [maintain.validation]
    validation_classes: [repository-hygiene]
tracked_path_exemptions: []"#,
        ),
    )
    .expect("add future route");
    write(fixture.root(), "future/maintenance.txt", "future\n");
    git(
        fixture.root(),
        &["add", "docs/owner-routing.yaml", "future/maintenance.txt"],
    );

    let report = xtask::run_owner_route(fixture.root(), None)
        .expect("always-on CI policy admits a new top-level route");
    assert!(report.unknown_paths.is_empty());
    assert!(report
        .changed_paths()
        .contains(&"future/maintenance.txt".to_owned()));
}

#[test]
fn deletion_only_and_rename_only_changes_need_no_ci_filter_metadata() {
    let deleted = Fixture::new();
    fs::remove_file(deleted.root().join("docs/delete.txt")).expect("delete routed path");
    git(deleted.root(), &["add", "-A"]);
    let report = xtask::run_owner_route(deleted.root(), Some(&deleted.base))
        .expect("always-on CI policy admits deletion-only change");
    assert_eq!(report.changes.len(), 1);
    assert_eq!(report.changes[0].kind, xtask::RepositoryChangeKind::Deleted);

    let renamed = Fixture::new();
    git(
        renamed.root(),
        &["mv", "docs/rename.txt", "docs/renamed.txt"],
    );
    let report = xtask::run_owner_route(renamed.root(), Some(&renamed.base))
        .expect("always-on CI policy admits rename-only change");
    assert_eq!(report.changes.len(), 1);
    assert_eq!(report.changes[0].kind, xtask::RepositoryChangeKind::Renamed);
}

#[test]
fn pull_request_and_push_reject_paths_and_paths_ignore_filters() {
    for (event, filter) in [
        ("pull_request", "paths"),
        ("pull_request", "paths-ignore"),
        ("push", "paths"),
        ("push", "paths-ignore"),
    ] {
        let fixture = Fixture::new();
        let before = if event == "pull_request" {
            "  pull_request:\n"
        } else {
            "  push:\n    branches: [main]\n"
        };
        let after = if event == "pull_request" {
            format!("  pull_request:\n    {filter}: [\"docs/**\"]\n")
        } else {
            format!("  push:\n    branches: [main]\n    {filter}: [\"docs/**\"]\n")
        };
        replace(fixture.root(), ".github/workflows/ci.yml", before, &after);

        let error = xtask::run_owner_route(fixture.root(), Some(&fixture.base))
            .expect_err("repository path filters must fail the CI contract")
            .to_string();
        assert!(
            error.contains(&format!(
                "CI workflow event {event} must not declare {filter}"
            )),
            "unexpected error: {error}"
        );
    }
}

#[test]
fn ci_base_resolution_must_precede_the_base_fed_final_validation() {
    let fixture = Fixture::new();
    replace(
        fixture.root(),
        ".github/workflows/ci.yml",
        "      - id: validation-base\n",
        "      - id: misplaced-base\n",
    );

    let error = xtask::run_owner_route(fixture.root(), Some(&fixture.base))
        .expect_err("missing event-specific base step must fail")
        .to_string();
    assert!(error.contains("must resolve one event-specific validation-base with ci-base"));
}

#[test]
fn routes_dirty_rust_docs_guidance_workflow_and_unknown_paths_without_mutation() {
    let fixture = Fixture::new();
    write(
        fixture.root(),
        "crates/sample/src/lib.rs",
        "pub fn changed() {}\n",
    );
    write(fixture.root(), "AGENTS.md", "# Changed root rules\n");
    write(
        fixture.root(),
        "docs/en/maintain/validation.md",
        "# Changed validation\n",
    );
    let workflow =
        fs::read_to_string(fixture.root().join(".github/workflows/ci.yml")).expect("read workflow");
    write(
        fixture.root(),
        ".github/workflows/ci.yml",
        &(workflow + "\n# changed\n"),
    );
    write(fixture.root(), "unknown.bin", "unknown\n");
    let status_before = fixture.status();
    let worktrees_before = fixture.worktrees();

    let report = xtask::run_owner_route(fixture.root(), None).expect("route dirty paths");

    assert_eq!(fixture.status(), status_before, "routing must be read-only");
    assert_eq!(
        fixture.worktrees(),
        worktrees_before,
        "routing must not add a Git worktree"
    );
    assert_eq!(
        report.changed_paths(),
        [
            ".github/workflows/ci.yml",
            "AGENTS.md",
            "crates/sample/src/lib.rs",
            "docs/en/maintain/validation.md",
            "unknown.bin",
        ]
    );
    assert_eq!(report.workspace_packages[0].name, "sample");
    let validation_document = report
        .maintained_documents
        .iter()
        .find(|document| document.doc_id == "maintain.validation")
        .expect("paired validation document is routed");
    assert_eq!(
        validation_document.paths,
        [
            "docs/en/maintain/validation.md",
            "docs/ko/maintain/validation.md"
        ]
    );
    assert_eq!(
        report
            .instructions
            .iter()
            .map(|item| (item.routing_basis, item.path.as_str()))
            .collect::<Vec<_>>(),
        [
            (xtask::RoutingBasis::Base, "AGENTS.md"),
            (xtask::RoutingBasis::Base, "crates/AGENTS.md"),
            (xtask::RoutingBasis::Base, "docs/AGENTS.md"),
            (xtask::RoutingBasis::Current, "AGENTS.md"),
            (xtask::RoutingBasis::Current, "crates/AGENTS.md"),
            (xtask::RoutingBasis::Current, "docs/AGENTS.md"),
        ]
    );
    assert_eq!(
        report
            .unknown_paths
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>(),
        ["unknown.bin"]
    );
    assert!(report.validation_classes.contains(&"workflow".to_owned()));
    assert!(report.validation_classes.contains(&"rust".to_owned()));
}

#[test]
fn explicit_base_includes_committed_and_dirty_paths_in_stable_human_json_order() {
    let fixture = Fixture::new();
    let workflow =
        fs::read_to_string(fixture.root().join(".github/workflows/ci.yml")).expect("read workflow");
    write(
        fixture.root(),
        ".github/workflows/ci.yml",
        &(workflow + "\n# committed change\n"),
    );
    git(fixture.root(), &["add", ".github/workflows/ci.yml"]);
    git(fixture.root(), &["commit", "-q", "-m", "test: workflow"]);
    write(
        fixture.root(),
        "docs/ko/maintain/validation.md",
        "# 변경한 검증\n",
    );

    let report = xtask::run_owner_route(fixture.root(), Some(&fixture.base))
        .expect("route base and dirty paths");
    let json = serde_json::to_string_pretty(&report).expect("render JSON");
    let human = report.render_human();

    assert_eq!(report.base_revision.as_deref(), Some(fixture.base.as_str()));
    assert_eq!(
        report.changed_paths(),
        [".github/workflows/ci.yml", "docs/ko/maintain/validation.md"]
    );
    for value in report
        .changed_paths()
        .iter()
        .chain(report.instructions.iter().map(|item| &item.path))
        .chain(report.validation_classes.iter())
    {
        assert!(json.contains(value), "JSON omits {value}");
        assert!(human.contains(value), "human output omits {value}");
    }
    for document in &report.maintained_documents {
        assert!(json.contains(&document.doc_id));
        assert!(human.contains(&document.doc_id));
        for path in &document.paths {
            assert!(json.contains(path));
            assert!(human.contains(path));
        }
    }
    let rerun = xtask::run_owner_route(fixture.root(), Some(&fixture.base))
        .expect("rerun deterministic route");
    assert_eq!(report, rerun);
}

#[cfg(unix)]
#[test]
fn discovers_closed_git_statuses_with_explicit_endpoint_bases() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    write(fixture.root(), "Dockerfile", "FROM busybox\n");
    fs::remove_file(fixture.root().join("docs/delete.txt")).expect("delete routed file");
    git(
        fixture.root(),
        &["mv", "docs/rename.txt", "docs/renamed.txt"],
    );
    fs::copy(
        fixture.root().join("docs/copy-source.txt"),
        fixture.root().join("docs/copied.txt"),
    )
    .expect("copy routed file");
    fs::remove_file(fixture.root().join("docs/type.txt")).expect("remove regular file");
    symlink("copy-source.txt", fixture.root().join("docs/type.txt")).expect("create symlink");
    git(fixture.root(), &["add", "-A"]);
    write(fixture.root(), "untracked.bin", "added\n");

    let report = xtask::run_owner_route(fixture.root(), Some(&fixture.base))
        .expect("route every supported change status");
    let by_kind = report
        .changes
        .iter()
        .map(|change| (change.kind, change))
        .collect::<std::collections::BTreeMap<_, _>>();

    let added = by_kind[&xtask::RepositoryChangeKind::Added];
    assert_eq!(added.old_path, None);
    assert_eq!(added.new_path.as_deref(), Some("untracked.bin"));
    assert_eq!(added.routing[0].routing_basis, xtask::RoutingBasis::Current);

    let modified = by_kind[&xtask::RepositoryChangeKind::Modified];
    assert_eq!(modified.new_path.as_deref(), Some("Dockerfile"));
    assert_eq!(
        modified
            .routing
            .iter()
            .map(|endpoint| endpoint.routing_basis)
            .collect::<Vec<_>>(),
        [xtask::RoutingBasis::Base, xtask::RoutingBasis::Current]
    );

    let deleted = by_kind[&xtask::RepositoryChangeKind::Deleted];
    assert_eq!(deleted.old_path.as_deref(), Some("docs/delete.txt"));
    assert_eq!(deleted.new_path, None);
    assert_eq!(deleted.routing[0].routing_basis, xtask::RoutingBasis::Base);

    let renamed = by_kind[&xtask::RepositoryChangeKind::Renamed];
    assert_eq!(renamed.old_path.as_deref(), Some("docs/rename.txt"));
    assert_eq!(renamed.new_path.as_deref(), Some("docs/renamed.txt"));
    assert_eq!(renamed.routing[0].routing_basis, xtask::RoutingBasis::Base);
    assert_eq!(
        renamed.routing[1].routing_basis,
        xtask::RoutingBasis::Current
    );

    let copied = by_kind[&xtask::RepositoryChangeKind::Copied];
    assert_eq!(copied.old_path.as_deref(), Some("docs/copy-source.txt"));
    assert_eq!(copied.new_path.as_deref(), Some("docs/copied.txt"));
    assert_eq!(copied.routing.len(), 1);
    assert_eq!(
        copied.routing[0].routing_basis,
        xtask::RoutingBasis::Current
    );

    let type_changed = by_kind[&xtask::RepositoryChangeKind::TypeChanged];
    assert_eq!(type_changed.old_path.as_deref(), Some("docs/type.txt"));
    assert_eq!(
        type_changed
            .routing
            .iter()
            .map(|endpoint| endpoint.routing_basis)
            .collect::<Vec<_>>(),
        [xtask::RoutingBasis::Base, xtask::RoutingBasis::Current]
    );
    assert!(report
        .unknown_paths
        .iter()
        .any(|item| item.path == "untracked.bin"
            && item.routing_basis == xtask::RoutingBasis::Current));

    let human = report.render_human();
    let json = serde_json::to_string_pretty(&report).expect("render JSON");
    for value in [
        "added",
        "modified",
        "deleted",
        "renamed",
        "copied",
        "type_changed",
        "docs/rename.txt",
        "docs/renamed.txt",
        "base",
        "current",
    ] {
        assert!(human.contains(value), "human report omits {value}");
        assert!(json.contains(value), "JSON report omits {value}");
    }
}

#[test]
fn document_rename_uses_base_and_current_document_snapshots() {
    let fixture = Fixture::new();
    git(
        fixture.root(),
        &[
            "mv",
            "docs/en/maintain/validation.md",
            "docs/en/maintain/validation-renamed.md",
        ],
    );
    git(
        fixture.root(),
        &[
            "mv",
            "docs/ko/maintain/validation.md",
            "docs/ko/maintain/validation-renamed.md",
        ],
    );
    replace(
        fixture.root(),
        "docs/doc-index.yaml",
        "docs/en/maintain/validation.md",
        "docs/en/maintain/validation-renamed.md",
    );
    replace(
        fixture.root(),
        "docs/doc-index.yaml",
        "docs/ko/maintain/validation.md",
        "docs/ko/maintain/validation-renamed.md",
    );
    git(fixture.root(), &["add", "docs/doc-index.yaml"]);

    let report = xtask::run_owner_route(fixture.root(), Some(&fixture.base))
        .expect("route renamed maintained document");
    let documents = report
        .maintained_documents
        .iter()
        .filter(|document| document.doc_id == "maintain.validation")
        .collect::<Vec<_>>();
    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0].routing_basis, xtask::RoutingBasis::Base);
    assert!(documents[0]
        .paths
        .contains(&"docs/en/maintain/validation.md".to_owned()));
    assert_eq!(documents[1].routing_basis, xtask::RoutingBasis::Current);
    assert!(documents[1]
        .paths
        .contains(&"docs/en/maintain/validation-renamed.md".to_owned()));
}

#[test]
fn package_directory_rename_routes_both_workspace_snapshots() {
    let fixture = Fixture::new();
    git(fixture.root(), &["mv", "crates/sample", "crates/renamed"]);
    replace(
        fixture.root(),
        "Cargo.toml",
        "crates/sample",
        "crates/renamed",
    );
    git(fixture.root(), &["add", "Cargo.toml"]);

    let report = xtask::run_owner_route(fixture.root(), Some(&fixture.base))
        .expect("route renamed package directory");
    assert!(report.workspace_packages.iter().any(|package| {
        package.routing_basis == xtask::RoutingBasis::Base
            && package.manifest_path == "crates/sample/Cargo.toml"
    }));
    assert!(report.workspace_packages.iter().any(|package| {
        package.routing_basis == xtask::RoutingBasis::Current
            && package.manifest_path == "crates/renamed/Cargo.toml"
    }));
    assert!(report
        .validation_classes
        .contains(&"architecture".to_owned()));
    assert!(report.validation_classes.contains(&"rust".to_owned()));
}

#[test]
fn deleted_package_is_base_owned_and_not_a_current_test_target() {
    let fixture = Fixture::new();
    git(fixture.root(), &["rm", "-r", "-q", "crates/sample"]);
    replace(
        fixture.root(),
        "Cargo.toml",
        "members = [\"crates/sample\"]",
        "members = []",
    );
    let routing = fs::read_to_string(fixture.root().join("docs/owner-routing.yaml"))
        .expect("read routing metadata");
    fs::write(
        fixture.root().join("docs/owner-routing.yaml"),
        routing.replace(
            "package_routes:\n  sample:\n    owner_doc_ids: [maintain.validation]\n    validation_classes: [architecture]\n",
            "package_routes: {}\n",
        ),
    )
    .expect("remove deleted package route");
    command(fixture.root(), "cargo", &["generate-lockfile"]);
    git(
        fixture.root(),
        &["add", "Cargo.toml", "Cargo.lock", "docs/owner-routing.yaml"],
    );

    let report = xtask::run_owner_route(fixture.root(), Some(&fixture.base))
        .expect("route deleted package from base snapshot");
    let packages = report
        .workspace_packages
        .iter()
        .filter(|package| package.name == "sample")
        .collect::<Vec<_>>();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].routing_basis, xtask::RoutingBasis::Base);
    for class in ["architecture", "repository-hygiene", "rust"] {
        assert!(report.validation_classes.contains(&class.to_owned()));
    }
}

#[test]
fn route_reassignment_unions_old_and_current_owners() {
    let fixture = Fixture::new();
    replace(
        fixture.root(),
        "docs/owner-routing.yaml",
        "  - path: .dockerignore\n    owner_doc_ids: [maintain.validation]",
        "  - path: .dockerignore\n    owner_doc_ids: [maintain.other]",
    );
    write(fixture.root(), ".dockerignore", "target\nchanged\n");
    git(
        fixture.root(),
        &["add", ".dockerignore", "docs/owner-routing.yaml"],
    );

    let report = xtask::run_owner_route(fixture.root(), Some(&fixture.base))
        .expect("union reassigned owners");
    assert!(report.owner_documents.iter().any(|owner| {
        owner.routing_basis == xtask::RoutingBasis::Base
            && owner.doc_id == "maintain.validation"
            && owner
                .reasons
                .iter()
                .any(|reason| reason == "base:.dockerignore")
    }));
    assert!(report.owner_documents.iter().any(|owner| {
        owner.routing_basis == xtask::RoutingBasis::Current
            && owner.doc_id == "maintain.other"
            && owner
                .reasons
                .iter()
                .any(|reason| reason == "current:.dockerignore")
    }));
}

#[test]
fn file_and_exact_route_can_be_deleted_together() {
    let fixture = Fixture::new();
    git(fixture.root(), &["rm", "-q", ".dockerignore"]);
    remove_dockerignore_route(fixture.root());
    git(fixture.root(), &["add", "docs/owner-routing.yaml"]);

    let report = xtask::run_owner_route(fixture.root(), Some(&fixture.base))
        .expect("route deleted file through base metadata");
    let deleted = report
        .changes
        .iter()
        .find(|change| change.old_path.as_deref() == Some(".dockerignore"))
        .expect("deleted path");
    assert_eq!(deleted.kind, xtask::RepositoryChangeKind::Deleted);
    assert!(report.unknown_paths.is_empty());
}

#[test]
fn current_exact_route_to_an_absent_path_fails_metadata_integrity() {
    let fixture = Fixture::new();
    git(fixture.root(), &["rm", "-q", ".dockerignore"]);

    let error = xtask::run_owner_route(fixture.root(), Some(&fixture.base))
        .expect_err("stale current exact route must fail")
        .to_string();
    assert!(error.contains("exact path route .dockerignore does not name a current path"));
}

fn remove_dockerignore_route(root: &Path) {
    let path = root.join("docs/owner-routing.yaml");
    let routing = fs::read_to_string(&path).expect("read routing metadata");
    let block = r#"  - path: .dockerignore
    owner_doc_ids: [maintain.validation]
    validation_classes: [release, repository-hygiene]
"#;
    assert!(routing.contains(block));
    fs::write(path, routing.replace(block, "")).expect("remove exact route");
}

fn replace(root: &Path, relative: &str, before: &str, after: &str) {
    let path = root.join(relative);
    let contents = fs::read_to_string(&path).expect("read replacement source");
    assert!(
        contents.contains(before),
        "{relative} does not contain replacement source"
    );
    fs::write(path, contents.replacen(before, after, 1)).expect("write replacement");
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, contents).expect("write fixture file");
}

fn git(root: &Path, args: &[&str]) {
    command(root, "git", args);
}

fn command(root: &Path, program: &str, args: &[&str]) {
    let output = Command::new(program)
        .current_dir(root)
        .args(args)
        .output()
        .expect("execute fixture command");
    assert!(
        output.status.success(),
        "{program} {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(root: &Path, args: &[&str]) -> String {
    String::from_utf8(git_bytes(root, args))
        .expect("Git output is UTF-8")
        .trim()
        .to_owned()
}

fn git_bytes(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("execute Git fixture command");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}
