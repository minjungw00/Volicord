use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use volicord_context::{
    Availability, CanonicalReadOptions, DeterministicIdGenerator, FixedClock, OperationId,
    Principal, PrincipalKind, SourceDraft, SourcePayload, Store, TimestampMicros,
};
use volicord_repository_intelligence::{
    analyze_repository, canonical_json, inventory_repository, CanonicalGrounding, Capability,
    CapabilityState, EcosystemObservationKind, InventoryClassification, InventoryRequest, Language,
    ProvenanceClass, SemanticAnalysisResult, StructuralAnalysisRequest, StructuralFact,
};

const OBSERVED_AT: i64 = 1_725_000_000_000_000;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../validation/repository-intelligence/polyglot-structural/fixtures")
        .join(name)
}

fn request(root: &Path) -> Result<InventoryRequest<'_>, Box<dyn Error>> {
    let canonical = support::repository_grounding(0x11, 0x22)?;
    Ok(InventoryRequest::new(
        root,
        &canonical.grounding,
        canonical.source_id,
        OBSERVED_AT,
    )?)
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(arguments)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "Git fixture failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn git_request(root: &Path) -> Result<InventoryRequest<'_>, Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    let status = git(root, &["status", "--porcelain=v1"])?;
    assert!(status.is_empty(), "fixture worktree must be clean");
    Ok(request(root)?.with_repository_worktree(
        volicord_repository_intelligence::RepositoryWorktreeObservation::Git {
            status_fingerprint: format!("sha256:{:x}", Sha256::digest(status.as_bytes())),
            dirty_paths: Vec::new(),
        },
    ))
}

#[test]
fn linked_worktree_observes_common_loose_and_packed_refs_and_detached_head(
) -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let primary = temporary.path().join("primary");
    let linked = temporary.path().join("linked");
    fs::create_dir(&primary)?;
    git(&primary, &["init", "-q", "-b", "main"])?;
    fs::write(primary.join("main.py"), "VALUE = 1\n")?;
    git(&primary, &["add", "main.py"])?;
    git(&primary, &["commit", "-qm", "initial"])?;
    let initial_head = git(&primary, &["rev-parse", "HEAD"])?;
    let (normal, _) = inventory_repository(git_request(&primary)?)?;
    let normal_git = normal
        .observation_basis
        .git
        .as_ref()
        .ok_or("normal Git observation missing")?;
    assert_eq!(normal_git.head, initial_head);
    assert_eq!(normal_git.reference.as_deref(), Some("refs/heads/main"));

    git(
        &primary,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "linked",
            linked.to_str().ok_or("non-UTF8 fixture path")?,
        ],
    )?;
    assert!(linked.join(".git").is_file());
    assert!(primary.join(".git/refs/heads/linked").is_file());
    let (first, analysis) = inventory_repository(git_request(&linked)?)?;
    let observed = first
        .observation_basis
        .git
        .as_ref()
        .ok_or("linked Git observation missing")?;
    assert_eq!(observed.head, initial_head);
    assert_eq!(observed.reference.as_deref(), Some("refs/heads/linked"));
    let basis = first
        .observation_equivalence_basis(&analysis)
        .ok_or("linked equivalence missing")?;
    let (repeated, repeated_analysis) = inventory_repository(git_request(&linked)?)?;
    assert_eq!(
        Some(basis.clone()),
        repeated.observation_equivalence_basis(&repeated_analysis)
    );

    // An empty commit changes Git provenance while included source content is unchanged.
    git(
        &linked,
        &["commit", "--allow-empty", "-qm", "linked change"],
    )?;
    let changed_head = git(&linked, &["rev-parse", "HEAD"])?;
    assert_ne!(initial_head, changed_head);
    let (changed, changed_analysis) = inventory_repository(git_request(&linked)?)?;
    let changed_git = changed
        .observation_basis
        .git
        .as_ref()
        .ok_or("changed linked Git observation missing")?;
    assert_eq!(changed_git.head, changed_head);
    assert_eq!(changed_git.reference.as_deref(), Some("refs/heads/linked"));
    assert_ne!(first.identity, changed.identity);
    let changed_basis = changed
        .observation_equivalence_basis(&changed_analysis)
        .ok_or("changed linked equivalence missing")?;
    assert_ne!(basis, changed_basis);

    git(&primary, &["pack-refs", "--all", "--prune"])?;
    assert!(!primary.join(".git/refs/heads/linked").exists());
    assert!(primary.join(".git/packed-refs").is_file());
    let (packed, packed_analysis) = inventory_repository(git_request(&linked)?)?;
    assert_eq!(packed.observation_basis.git, changed.observation_basis.git);
    assert_eq!(
        Some(changed_basis),
        packed.observation_equivalence_basis(&packed_analysis)
    );
    let (normal_packed, _) = inventory_repository(git_request(&primary)?)?;
    assert_eq!(
        normal.observation_basis.git,
        normal_packed.observation_basis.git
    );

    git(&linked, &["checkout", "-q", "--detach"])?;
    let (detached, detached_analysis) = inventory_repository(git_request(&linked)?)?;
    let detached_git = detached
        .observation_basis
        .git
        .as_ref()
        .ok_or("detached Git observation missing")?;
    assert_eq!(detached_git.head, changed_head);
    assert_eq!(detached_git.reference, None);
    assert!(detached
        .observation_equivalence_basis(&detached_analysis)
        .is_some());

    // The same per-worktree ref name can hold a different value in each worktree.
    git(
        &primary,
        &["update-ref", "refs/worktree/current", &initial_head],
    )?;
    git(
        &linked,
        &["update-ref", "refs/worktree/current", &changed_head],
    )?;
    git(&linked, &["symbolic-ref", "HEAD", "refs/worktree/current"])?;
    let (local_ref, _) = inventory_repository(git_request(&linked)?)?;
    let local_git = local_ref
        .observation_basis
        .git
        .as_ref()
        .ok_or("per-worktree ref missing")?;
    assert_eq!(local_git.head, changed_head);
    assert_eq!(
        local_git.reference.as_deref(),
        Some("refs/worktree/current")
    );
    Ok(())
}

#[test]
fn incomplete_git_state_has_no_observation_or_equivalence() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    git(root, &["init", "-q", "-b", "main"])?;
    let (unborn, analysis) = inventory_repository(git_request(root)?)?;
    assert!(unborn.observation_basis.git.is_none());
    assert!(unborn.observation_equivalence_basis(&analysis).is_none());
    // Present unreadable or malformed loose state cannot fall back to a stale packed value.
    fs::create_dir_all(root.join(".git/refs/heads"))?;
    fs::write(
        root.join(".git/packed-refs"),
        format!("{} refs/heads/main\n", "a".repeat(40)),
    )?;
    for bytes in [Vec::new(), vec![0xff], b"incomplete".to_vec()] {
        fs::write(root.join(".git/refs/heads/main"), bytes)?;
        let (snapshot, analysis) = inventory_repository(request(root)?.with_repository_worktree(
            volicord_repository_intelligence::RepositoryWorktreeObservation::Git {
                status_fingerprint: format!("sha256:{}", "0".repeat(64)),
                dirty_paths: Vec::new(),
            },
        ))?;
        assert!(snapshot.observation_basis.git.is_none());
        assert!(snapshot.observation_equivalence_basis(&analysis).is_none());
    }
    fs::remove_file(root.join(".git/refs/heads/main"))?;
    fs::write(root.join(".git/HEAD"), "not-an-object-id\n")?;
    let (snapshot, _) = inventory_repository(request(root)?)?;
    assert!(snapshot.observation_basis.git.is_none());
    fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n")?;
    fs::write(root.join(".git/commondir"), "missing-common-directory\n")?;
    let (snapshot, _) = inventory_repository(request(root)?)?;
    assert!(snapshot.observation_basis.git.is_none());
    Ok(())
}

#[test]
fn nested_ignore_rules_match_git_and_do_not_leak_into_sibling_directories(
) -> Result<(), Box<dyn Error>> {
    let repository = tempfile::tempdir()?;
    let root = repository.path();
    assert!(std::process::Command::new("git")
        .args(["init", "-q"])
        .arg(root)
        .status()?
        .success());
    fs::create_dir_all(root.join("nested/deeper"))?;
    fs::create_dir_all(root.join("sibling"))?;
    fs::create_dir_all(root.join("nested/blocked"))?;
    fs::write(root.join(".gitignore"), "*.log\n")?;
    fs::write(
        root.join("nested/.gitignore"),
        "ignored.py\n!keep.log\n/root-only.py\n[a-b].py\nblocked/\n\\#literal.py\n",
    )?;
    fs::write(root.join("nested/deeper/.gitignore"), "!ignored.py\n")?;
    fs::write(root.join("nested/blocked/.gitignore"), "!never.py\n")?;
    let paths = [
        "nested/ignored.py",
        "nested/keep.log",
        "sibling/keep.log",
        "sibling/ignored.py",
        "nested/root-only.py",
        "nested/deeper/root-only.py",
        "nested/a.py",
        "nested/c.py",
        "nested/deeper/ignored.py",
        "nested/blocked/never.py",
        "nested/#literal.py",
    ];
    for path in paths {
        fs::write(root.join(path), "def marker():\n    pass\n")?;
    }
    let (_, analysis) = analyze_repository(StructuralAnalysisRequest::new(request(root)?))?;
    for path in paths {
        let ignored = std::process::Command::new("git")
            .current_dir(root)
            .args(["check-ignore", "--no-index", "-q", path])
            .status()?
            .success();
        let entry = analysis
            .inventory
            .entries
            .iter()
            .find(|entry| entry.area.path == path);
        if path == "nested/blocked/never.py" {
            assert!(ignored && entry.is_none());
        } else {
            let entry = entry.ok_or("fixture entry missing")?;
            assert_eq!(
                entry
                    .classifications
                    .contains(&InventoryClassification::Ignored),
                ignored,
                "{path}"
            );
            if ignored {
                assert!(
                    entry.content_sha256.is_none(),
                    "ignored content was read: {path}"
                );
            }
        }
        if ignored {
            assert!(!analysis
                .structural_facts
                .iter()
                .any(|fact| fact.entity.area.path == path));
        }
    }
    let (_, repeated) = analyze_repository(StructuralAnalysisRequest::new(request(root)?))?;
    assert_eq!(canonical_json(&analysis)?, canonical_json(&repeated)?);
    Ok(())
}

#[test]
fn unreadable_ignore_rules_are_explicit_partial_inventory() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    fs::write(root.path().join(".gitignore"), [0xff, 0xfe])?;
    fs::write(root.path().join("main.py"), "VALUE = 1\n")?;
    let (_, analysis) = inventory_repository(request(root.path())?)?;
    assert!(analysis
        .diagnostics
        .iter()
        .any(|item| item.code == "ignore_rules_unavailable"));
    assert!(analysis
        .capabilities
        .iter()
        .any(|item| item.capability == Capability::Inventory
            && item.state == CapabilityState::Partial));
    Ok(())
}

#[test]
fn maintained_fixtures_recognize_all_seven_gate_languages() -> Result<(), Box<dyn Error>> {
    let matrix = [
        ("java", Language::Java),
        ("python", Language::Python),
        ("javascript", Language::JavaScript),
        ("typescript", Language::TypeScript),
        ("c", Language::C),
        ("cpp", Language::Cpp),
        ("rust", Language::Rust),
    ];

    for (name, expected_language) in matrix {
        let (repository, analysis) = inventory_repository(request(&fixture(name))?)?;
        assert_eq!(analysis.repository_snapshot, repository.identity);
        assert!(analysis.inventory.languages.contains(&expected_language));
        let structural = analysis
            .capabilities
            .iter()
            .find(|report| {
                report.language.as_ref() == Some(&expected_language)
                    && report.capability == Capability::Structural
            })
            .ok_or("missing structural capability report")?;
        assert_eq!(structural.state, CapabilityState::Unavailable);
        assert!(!structural.coverage.unavailable.is_empty());
        assert!(analysis.structural_facts.is_empty());
        assert!(analysis.semantic_results.is_empty());
        if name == "rust" {
            assert!(analysis
                .inventory
                .ecosystem_observations
                .iter()
                .any(|observation| {
                    observation.area.path == "Cargo.toml"
                        && observation.kind == EcosystemObservationKind::WorkspaceManifest
                }));
        }
    }
    Ok(())
}

#[test]
fn out_of_set_text_language_keeps_inventory_with_honest_fallback() -> Result<(), Box<dyn Error>> {
    let (_, analysis) = inventory_repository(request(&fixture("out_of_set"))?)?;
    assert!(analysis.inventory.languages.contains(&Language::Go));

    let inventory = capability(&analysis, &Language::Go, Capability::Inventory)?;
    let structural = capability(&analysis, &Language::Go, Capability::Structural)?;
    let semantic = capability(&analysis, &Language::Go, Capability::Semantic)?;
    let ecosystem = capability(&analysis, &Language::Go, Capability::Ecosystem)?;
    assert_eq!(inventory.state, CapabilityState::Available);
    assert_eq!(structural.state, CapabilityState::Unsupported);
    assert_eq!(semantic.state, CapabilityState::Unsupported);
    assert_eq!(ecosystem.state, CapabilityState::Partial);
    assert!(!structural.coverage.unsupported.is_empty());
    assert!(analysis
        .inventory
        .ecosystem_observations
        .iter()
        .any(|observation| observation.area.path == "go.mod"));
    Ok(())
}

#[test]
fn snapshot_identity_and_serialization_are_path_independent_and_repeatable(
) -> Result<(), Box<dyn Error>> {
    let first_binding = tempfile::tempdir()?;
    let second_binding = tempfile::tempdir()?;
    copy_tree(&fixture("polyglot"), first_binding.path())?;
    copy_tree(&fixture("polyglot"), second_binding.path())?;

    let (first_repository, first_analysis) = inventory_repository(request(first_binding.path())?)?;
    let (repeated_repository, repeated_analysis) =
        inventory_repository(request(first_binding.path())?)?;
    let (other_repository, other_analysis) = inventory_repository(request(second_binding.path())?)?;

    assert_eq!(first_repository.identity, repeated_repository.identity);
    assert_eq!(first_repository.identity, other_repository.identity);
    assert_eq!(first_analysis.identity, repeated_analysis.identity);
    assert_eq!(first_analysis.identity, other_analysis.identity);
    assert_eq!(
        canonical_json(&first_repository)?,
        canonical_json(&other_repository)?
    );
    assert_eq!(
        canonical_json(&first_analysis)?,
        canonical_json(&other_analysis)?
    );
    let serialized = String::from_utf8(canonical_json(&first_analysis)?)?;
    assert!(!serialized.contains(&first_binding.path().display().to_string()));
    assert!(!serialized.contains(&second_binding.path().display().to_string()));

    fs::write(
        first_binding.path().join("python/formatter.py"),
        "def format_greeting(name):\n    return name.upper()\n",
    )?;
    let (changed_repository, changed_analysis) =
        inventory_repository(request(first_binding.path())?)?;
    assert_ne!(first_repository.identity, changed_repository.identity);
    assert_ne!(first_analysis.identity, changed_analysis.identity);
    Ok(())
}

#[test]
fn observation_equivalence_preserves_scope_across_fresh_sources() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let other_root = tempfile::tempdir()?;
    copy_tree(&fixture("polyglot"), root.path())?;
    copy_tree(&fixture("polyglot"), other_root.path())?;
    let (repository, analysis) = inventory_repository(request(root.path())?)?;
    let basis = repository
        .observation_equivalence_basis(&analysis)
        .ok_or("basis missing")?;
    let fresh = support::repository_grounding(0x11, 0x33)?;
    let fresh_request = InventoryRequest::new(
        other_root.path(),
        &fresh.grounding,
        fresh.source_id,
        OBSERVED_AT + 1,
    )?;
    let (fresh_repository, fresh_analysis) = inventory_repository(fresh_request)?;
    assert_ne!(
        repository.repository_source,
        fresh_repository.repository_source
    );
    assert_ne!(repository.identity, fresh_repository.identity);
    assert_ne!(analysis.identity, fresh_analysis.identity);
    assert_eq!(
        Some(basis.clone()),
        fresh_repository.observation_equivalence_basis(&fresh_analysis)
    );
    assert!(
        repository
            .observation_equivalence_basis(&fresh_analysis)
            .is_none(),
        "mismatched Source-bound snapshots"
    );

    let mut excluded = request(root.path())?;
    excluded.excluded_paths = vec!["python".into()];
    let (excluded_repository, excluded_analysis) = inventory_repository(excluded)?;
    assert_ne!(
        Some(basis.clone()),
        excluded_repository.observation_equivalence_basis(&excluded_analysis)
    );
    let mut changed = analysis.clone();
    changed.capabilities[0]
        .adapter
        .as_mut()
        .ok_or("inventory adapter")?
        .version
        .push_str("-changed");
    assert_ne!(
        Some(basis.clone()),
        repository.observation_equivalence_basis(&changed)
    );
    changed = analysis.clone();
    changed.repository_worktree =
        volicord_repository_intelligence::RepositoryWorktreeObservation::Git {
            status_fingerprint: format!("sha256:{}", "0".repeat(64)),
            dirty_paths: vec![],
        };
    assert!(
        repository.observation_equivalence_basis(&changed).is_none(),
        "missing Git observation"
    );
    fs::write(root.path().join("python/formatter.py"), "VALUE = 2\n")?;
    let (changed_repository, changed_analysis) = inventory_repository(request(root.path())?)?;
    assert_ne!(
        Some(basis),
        changed_repository.observation_equivalence_basis(&changed_analysis)
    );
    Ok(())
}

#[test]
fn exclusions_binary_vendor_generated_and_ignored_scopes_remain_visible(
) -> Result<(), Box<dyn Error>> {
    let repository = tempfile::tempdir()?;
    fs::create_dir_all(repository.path().join("vendor/lib"))?;
    fs::create_dir_all(repository.path().join("target/debug"))?;
    fs::create_dir_all(repository.path().join("python/.pytest_cache/v/cache"))?;
    fs::create_dir_all(repository.path().join("python/.mypy_cache/3.12"))?;
    fs::create_dir_all(
        repository
            .path()
            .join("python/.venv/lib/python3.12/site-packages/demo"),
    )?;
    fs::create_dir_all(repository.path().join("python/.ruff_cache/0.9.1"))?;
    fs::create_dir_all(repository.path().join("private"))?;
    fs::write(repository.path().join("main.py"), "print('ok')\n")?;
    fs::write(repository.path().join("image.bin"), [0_u8, 1, 2, 3])?;
    fs::write(repository.path().join("ignored.log"), "ignored\n")?;
    fs::write(repository.path().join(".gitignore"), "*.log\n")?;
    fs::write(repository.path().join("vendor/lib/code.js"), "export {};\n")?;
    fs::write(
        repository.path().join("target/debug/out.rs"),
        "fn generated() {}\n",
    )?;
    fs::write(
        repository
            .path()
            .join("python/.pytest_cache/v/cache/nodeids"),
        "[]\n",
    )?;
    fs::write(
        repository
            .path()
            .join("python/.mypy_cache/3.12/module.json"),
        "{}\n",
    )?;
    fs::write(
        repository
            .path()
            .join("python/.venv/lib/python3.12/site-packages/demo/__init__.py"),
        "def generated_environment_code(): pass\n",
    )?;
    fs::write(
        repository.path().join("python/.ruff_cache/0.9.1/cache-key"),
        "generated cache\n",
    )?;
    fs::write(
        repository.path().join("python/pyproject.toml"),
        "[tool.pytest.ini_options]\n",
    )?;
    fs::write(
        repository.path().join("python/test_feature.py"),
        "def test_feature():\n    assert True\n",
    )?;
    fs::write(
        repository.path().join("private/secret.txt"),
        "not inventoried\n",
    )?;

    let mut inventory_request = request(repository.path())?;
    inventory_request.excluded_paths = vec!["private".to_owned()];
    let (snapshot, analysis) = inventory_repository(inventory_request)?;

    assert_classification(&analysis, "ignored.log", InventoryClassification::Ignored)?;
    assert_classification(&analysis, "vendor", InventoryClassification::Vendor)?;
    assert_classification(&analysis, "target", InventoryClassification::Generated)?;
    assert_classification(
        &analysis,
        "python/.pytest_cache",
        InventoryClassification::Generated,
    )?;
    assert_classification(
        &analysis,
        "python/.mypy_cache",
        InventoryClassification::Generated,
    )?;
    assert_classification(
        &analysis,
        "python/.venv",
        InventoryClassification::Generated,
    )?;
    assert_classification(
        &analysis,
        "python/.ruff_cache",
        InventoryClassification::Generated,
    )?;
    assert_classification(&analysis, "private", InventoryClassification::Excluded)?;
    assert_classification(&analysis, "image.bin", InventoryClassification::Binary)?;
    assert!(snapshot
        .excluded_areas
        .iter()
        .any(|area| area.path == "ignored.log"));
    for meaningful in ["python/pyproject.toml", "python/test_feature.py"] {
        assert_classification(&analysis, meaningful, InventoryClassification::Included)?;
    }
    assert!(!analysis
        .inventory
        .entries
        .iter()
        .any(
            |entry| entry.area.path == "python/.pytest_cache/v/cache/nodeids"
                || entry.area.path == "python/.mypy_cache/3.12/module.json"
                || entry.area.path == "python/.venv/lib/python3.12/site-packages/demo/__init__.py"
                || entry.area.path == "python/.ruff_cache/0.9.1/cache-key"
        ));
    assert!(!analysis.structural_facts.iter().any(|fact| {
        fact.entity.area.path.starts_with("python/.pytest_cache/")
            || fact.entity.area.path.starts_with("python/.mypy_cache/")
            || fact.entity.area.path.starts_with("python/.venv/")
            || fact.entity.area.path.starts_with("python/.ruff_cache/")
    }));
    let overall = analysis
        .capabilities
        .iter()
        .find(|report| report.language.is_none() && report.capability == Capability::Inventory)
        .ok_or("missing repository inventory capability")?;
    assert!(overall.coverage.excluded.len() >= 4);
    Ok(())
}

#[cfg(unix)]
#[test]
fn unavailable_entry_does_not_erase_successful_inventory() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::symlink;

    let repository = tempfile::tempdir()?;
    fs::write(repository.path().join("main.rs"), "fn main() {}\n")?;
    symlink("missing-target", repository.path().join("broken-link"))?;

    let (snapshot, analysis) = inventory_repository(request(repository.path())?)?;
    assert!(snapshot.observation_equivalence_basis(&analysis).is_none());
    assert!(analysis
        .inventory
        .entries
        .iter()
        .any(|entry| entry.area.path == "main.rs"));
    assert!(snapshot
        .unavailable_areas
        .iter()
        .any(|area| area.path == "broken-link"));
    let overall = analysis
        .capabilities
        .iter()
        .find(|report| report.language.is_none() && report.capability == Capability::Inventory)
        .ok_or("missing repository inventory capability")?;
    assert_eq!(overall.state, CapabilityState::Partial);
    assert!(!overall.coverage.included.is_empty());
    assert!(!overall.coverage.unavailable.is_empty());
    Ok(())
}

#[test]
fn repository_analysis_has_no_canonical_mutation_authority() -> Result<(), Box<dyn Error>> {
    let runtime = tempfile::tempdir()?;
    let mut store = Store::open_with(
        runtime.path().join("context.sqlite3"),
        DeterministicIdGenerator::new([[0x41; 16], [0x42; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(100)),
    )?;
    let project = store
        .create_project(OperationId::from_bytes([1; 16]), "Inventory purity")?
        .value;
    let source = store
        .record_source(
            OperationId::from_bytes([2; 16]),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "fixture-basis".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "test-repository".to_owned(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "test-agent".to_owned(),
                }),
                availability: Availability::Available,
            },
        )?
        .value;
    let before = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let grounding = CanonicalGrounding::from_read_basis(&before)?;

    let rust_fixture = fixture("rust");
    let inventory_request =
        InventoryRequest::new(&rust_fixture, &grounding, source.id, OBSERVED_AT)?;
    let (_, analysis) = analyze_repository(StructuralAnalysisRequest::new(inventory_request))?;
    let after = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;

    assert_eq!(before, after);
    assert!(after.active_questions.is_empty());
    assert!(after.active_decisions.is_empty());
    assert!(after.context_items.is_empty());
    assert!(after.latest_checkpoint.is_none());
    assert_eq!(analysis.project.identity(), project.id);
    assert_eq!(analysis.repository_source.identity(), source.id);
    Ok(())
}

#[test]
fn fact_result_and_interpretation_are_distinct_types_and_classes() {
    assert_ne!(
        std::any::TypeId::of::<StructuralFact>(),
        std::any::TypeId::of::<SemanticAnalysisResult>()
    );
    let classes = BTreeSet::from([
        ProvenanceClass::StructuralFact,
        ProvenanceClass::SemanticResult,
        ProvenanceClass::SemanticAnnotation,
        ProvenanceClass::AgentInterpretation,
    ]);
    assert_eq!(classes.len(), 4);
}

fn capability<'a>(
    analysis: &'a volicord_repository_intelligence::AnalysisSnapshot,
    language: &Language,
    capability: Capability,
) -> Result<&'a volicord_repository_intelligence::CapabilityReport, Box<dyn Error>> {
    analysis
        .capabilities
        .iter()
        .find(|report| {
            report.language.as_ref() == Some(language) && report.capability == capability
        })
        .ok_or_else(|| "missing capability report".into())
}

fn assert_classification(
    analysis: &volicord_repository_intelligence::AnalysisSnapshot,
    path: &str,
    expected: InventoryClassification,
) -> Result<(), Box<dyn Error>> {
    let entry = analysis
        .inventory
        .entries
        .iter()
        .find(|entry| entry.area.path == path)
        .ok_or("missing inventory entry")?;
    assert!(entry.classifications.contains(&expected));
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&destination_path)?;
            copy_tree(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}
mod support;
