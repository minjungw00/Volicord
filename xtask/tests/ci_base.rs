use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

struct Fixture {
    _directory: TempDir,
    root: PathBuf,
    events: PathBuf,
    base: String,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary fixture directory");
        let root = directory.path().join("repository");
        let events = directory.path().join("events");
        fs::create_dir_all(&root).expect("create repository");
        fs::create_dir_all(&events).expect("create event directory");
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
  - path: Cargo.lock
    owner_doc_ids: [maintain.validation]
    validation_classes: [repository-hygiene, rust]
  - path: Cargo.toml
    owner_doc_ids: [maintain.validation]
    validation_classes: [architecture, repository-hygiene, rust]
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
        git(&root, &["config", "user.email", "ci-base@example.invalid"]);
        git(&root, &["config", "user.name", "CI Base Test"]);
        command(&root, "cargo", &["generate-lockfile"]);
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "fixture: base"]);
        let base = git_output(&root, &["rev-parse", "HEAD"]);

        Self {
            _directory: directory,
            root,
            events,
            base,
        }
    }

    fn commit_change(&self, subject: &str) -> String {
        write(
            &self.root,
            "crates/sample/src/lib.rs",
            "pub fn changed() {}\n",
        );
        git(&self.root, &["add", "crates/sample/src/lib.rs"]);
        git(&self.root, &["commit", "-q", "-m", subject]);
        git_output(&self.root, &["rev-parse", "HEAD"])
    }

    fn event(&self, name: &str, value: Value) -> PathBuf {
        let path = self.events.join(name);
        fs::write(
            &path,
            serde_json::to_vec_pretty(&value).expect("serialize event"),
        )
        .expect("write event");
        path
    }
}

#[test]
fn pull_request_base_preserves_changed_paths_and_package_attribution() {
    let fixture = Fixture::new();
    let head = fixture.commit_change("fix: change package");
    let event = fixture.event(
        "pull-request.json",
        json!({"pull_request": {"base": {"sha": fixture.base}}}),
    );

    let resolution = xtask::resolve_ci_base(&fixture.root, "pull_request", &event, "HEAD")
        .expect("resolve pull-request base");
    assert_eq!(resolution.base_revision, fixture.base);
    assert_eq!(resolution.head_revision, head);
    assert_eq!(resolution.changed_paths, ["crates/sample/src/lib.rs"]);
    let github_output = fixture.events.join("github-output");
    xtask::append_github_output(&github_output, &resolution).expect("write GitHub output");
    assert_eq!(
        fs::read_to_string(github_output).expect("read GitHub output"),
        format!("base={}\n", fixture.base)
    );

    let route = xtask::run_owner_route(&fixture.root, Some(&resolution.base_revision))
        .expect("route resolved pull-request range");
    assert_eq!(route.changed_paths(), ["crates/sample/src/lib.rs"]);
    assert_eq!(route.workspace_packages[0].name, "sample");
}

#[test]
fn push_and_manual_events_select_their_explicit_series_bases() {
    for (event_name, payload) in [
        ("push", json!({"before": "BASE"})),
        ("workflow_dispatch", json!({"inputs": {"base": "BASE"}})),
    ] {
        let fixture = Fixture::new();
        fixture.commit_change("fix: change package");
        let payload =
            serde_json::from_str::<Value>(&payload.to_string().replace("BASE", &fixture.base))
                .expect("replace fixture base");
        let event = fixture.event(&format!("{event_name}.json"), payload);

        let resolution = xtask::resolve_ci_base(&fixture.root, event_name, &event, "HEAD")
            .expect("resolve event base");
        assert_eq!(resolution.base_revision, fixture.base);
        assert_eq!(resolution.changed_paths, ["crates/sample/src/lib.rs"]);
        let route = xtask::run_owner_route(&fixture.root, Some(&resolution.base_revision))
            .expect("route resolved event range");
        assert_eq!(route.workspace_packages[0].name, "sample");
    }
}

#[test]
fn zero_invalid_and_equal_bases_fail_closed() {
    let fixture = Fixture::new();
    let head = fixture.commit_change("fix: change package");
    let cases = [
        (
            "push",
            json!({"before": "0000000000000000000000000000000000000000"}),
            "zero object ID",
        ),
        (
            "push",
            json!({"before": "ffffffffffffffffffffffffffffffffffffffff"}),
            "missing or unreachable",
        ),
        ("push", json!({"before": head}), "HEAD..HEAD"),
        (
            "workflow_dispatch",
            json!({"inputs": {}}),
            "missing required inputs.base",
        ),
    ];
    for (index, (event_name, payload, expected)) in cases.into_iter().enumerate() {
        let event = fixture.event(&format!("invalid-{index}.json"), payload);
        let error = xtask::resolve_ci_base(&fixture.root, event_name, &event, "HEAD")
            .expect_err("invalid base must fail")
            .to_string();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn a_commit_only_range_without_changed_paths_fails_closed() {
    let fixture = Fixture::new();
    git(
        &fixture.root,
        &["commit", "--allow-empty", "-q", "-m", "fix: empty"],
    );
    let event = fixture.event("empty.json", json!({"before": fixture.base}));

    let error = xtask::resolve_ci_base(&fixture.root, "push", &event, "HEAD")
        .expect_err("empty changed-path range must fail")
        .to_string();
    assert!(error.contains("do not describe a nonempty changed-path series"));
}

#[test]
fn a_reachable_nonancestor_base_fails_closed() {
    let fixture = Fixture::new();
    let main_branch = git_output(&fixture.root, &["branch", "--show-current"]);
    git(
        &fixture.root,
        &["checkout", "-q", "-b", "unrelated", &fixture.base],
    );
    write(&fixture.root, "unrelated.txt", "unrelated\n");
    git(&fixture.root, &["add", "unrelated.txt"]);
    git(&fixture.root, &["commit", "-q", "-m", "fix: unrelated"]);
    let unrelated = git_output(&fixture.root, &["rev-parse", "HEAD"]);
    git(&fixture.root, &["checkout", "-q", &main_branch]);
    fixture.commit_change("fix: change package");
    let event = fixture.event("nonancestor.json", json!({"before": unrelated}));

    let error = xtask::resolve_ci_base(&fixture.root, "push", &event, "HEAD")
        .expect_err("nonancestor base must fail")
        .to_string();
    assert!(error.contains("is not an ancestor"));
}

#[test]
fn shallow_history_cannot_silently_accept_an_unreachable_base() {
    let fixture = Fixture::new();
    fixture.commit_change("fix: change package");
    let clone_parent = tempfile::tempdir().expect("clone parent");
    let clone = clone_parent.path().join("shallow");
    let source = format!("file://{}", fixture.root.display());
    let output = Command::new("git")
        .args(["clone", "--quiet", "--depth", "1", &source])
        .arg(&clone)
        .output()
        .expect("clone shallow repository");
    assert!(
        output.status.success(),
        "shallow clone failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let event = fixture.event("shallow.json", json!({"before": fixture.base}));

    let error = xtask::resolve_ci_base(&clone, "push", &event, "HEAD")
        .expect_err("shallow history must fail")
        .to_string();
    assert!(error.contains("missing or unreachable"));
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
    String::from_utf8(output.stdout)
        .expect("Git output is UTF-8")
        .trim()
        .to_owned()
}
