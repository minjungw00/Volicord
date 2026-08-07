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
  paths: ["*", ".github/**", "crates/**", "docs/**"]
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
    paths: ["*", ".github/**", "crates/**", "docs/**"]
  push:
    paths: ["*", ".github/**", "crates/**", "docs/**"]
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
        report.changed_paths,
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
    for path in &report.changed_paths {
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
fn newly_routed_directory_must_also_join_the_canonical_ci_trigger_policy() {
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

    let error = xtask::run_owner_route(fixture.root(), None)
        .expect_err("CI trigger coverage must follow new tracked routes")
        .to_string();
    assert!(error.contains("CI trigger policy does not cover tracked path(s)"));
    assert!(error.contains("future/maintenance.txt"));
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
    write(
        fixture.root(),
        ".github/workflows/ci.yml",
        r#"name: Changed CI
on:
  pull_request:
    paths: ["*", ".github/**", "crates/**", "docs/**"]
  push:
    paths: ["*", ".github/**", "crates/**", "docs/**"]
"#,
    );
    write(fixture.root(), "unknown.bin", "unknown\n");
    let status_before = fixture.status();

    let report = xtask::run_owner_route(fixture.root(), None).expect("route dirty paths");

    assert_eq!(fixture.status(), status_before, "routing must be read-only");
    assert_eq!(
        report.changed_paths,
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
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>(),
        ["AGENTS.md", "crates/AGENTS.md", "docs/AGENTS.md"]
    );
    assert_eq!(report.unknown_paths, ["unknown.bin"]);
    assert!(report.validation_classes.contains(&"workflow".to_owned()));
    assert!(report.validation_classes.contains(&"rust".to_owned()));
}

#[test]
fn explicit_base_includes_committed_and_dirty_paths_in_stable_human_json_order() {
    let fixture = Fixture::new();
    write(
        fixture.root(),
        ".github/workflows/ci.yml",
        r#"name: Committed CI
on:
  pull_request:
    paths: ["*", ".github/**", "crates/**", "docs/**"]
  push:
    paths: ["*", ".github/**", "crates/**", "docs/**"]
"#,
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
        report.changed_paths,
        [".github/workflows/ci.yml", "docs/ko/maintain/validation.md"]
    );
    for value in report
        .changed_paths
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
