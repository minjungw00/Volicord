use rusqlite::Connection;
use std::{ffi::OsString, fs, path::Path, process::Command, time::Duration};
use tempfile::TempDir;
use volicord_context::{
    CheckpointKind, ContextItemId, ContextItemRole, PrincipalKind, ProjectId, SourcePayload,
    VerificationState, WorkState,
};
use volicord_local_platform::{CancellationFlag, ProcessTermination, ProcessTreeCleanup};
use volicord_operations::{
    CommandVerificationDraft, GroundedCheckpointDraft, HealthState, LocalOperations,
    OperationState, ProjectResolution, RuntimeLayout,
};
use volicord_projections::{CandidateDependencyState, ProjectionHealth, ProjectionIssueKind};
use volicord_repository_intelligence::AnalysisSnapshotId;

#[cfg(target_os = "linux")]
use std::os::unix::fs::{symlink, PermissionsExt};

fn fixture() -> Result<(TempDir, LocalOperations, std::path::PathBuf), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src"))?;
    fs::write(
        repository.join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    )?;
    fs::write(
        repository.join("src/lib.rs"),
        "pub fn answer() -> u32 { 42 }\n",
    )?;
    let runtime = temporary.path().join("runtime");
    let operations = LocalOperations::new(RuntimeLayout::new(runtime)?);
    Ok((temporary, operations, repository))
}

#[test]
fn repository_initialization_prefers_local_git_identity_then_falls_back_and_preserves_explicit_names(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let source_repository = temporary.path().join("tree-sitter");
    fs::create_dir_all(&source_repository)?;
    assert!(Command::new("git")
        .arg("-C")
        .arg(&source_repository)
        .args(["init", "-q"])
        .status()?
        .success());
    let derived_repository = temporary.path().join("campaign").join("repository");
    fs::create_dir_all(derived_repository.parent().ok_or("clone parent")?)?;
    assert!(Command::new("git")
        .args(["clone", "-q"])
        .arg(&source_repository)
        .arg(&derived_repository)
        .status()?
        .success());
    let fallback_repository = temporary
        .path()
        .join("misleading-Volicord")
        .join("rebuild")
        .join("fallback-name");
    fs::create_dir_all(&fallback_repository)?;
    fs::create_dir_all(&derived_repository)?;
    let explicit_repository = temporary
        .path()
        .join("another-outer-name")
        .join("derived-looking-name");
    fs::create_dir_all(&explicit_repository)?;
    let operations = LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime"))?);

    let derived = operations.initialize_project_from_repository(&derived_repository)?;
    assert_eq!(derived.project.display_name, "tree-sitter");
    assert_eq!(
        derived
            .binding
            .ok_or("derived Project was not bound")?
            .binding
            .absolute_path,
        fs::canonicalize(&derived_repository)?,
    );

    let fallback = operations.initialize_project_from_repository(&fallback_repository)?;
    assert_eq!(fallback.project.display_name, "fallback-name");

    let explicit = operations.initialize_project("User Chosen Name", Some(&explicit_repository))?;
    assert_eq!(explicit.project.display_name, "User Chosen Name");
    assert_eq!(
        explicit
            .binding
            .ok_or("explicit Project was not bound")?
            .binding
            .absolute_path,
        fs::canonicalize(&explicit_repository)?,
    );
    Ok(())
}

#[test]
fn candidate_store_states_remain_typed_in_partial_project_projections(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_healthy_root, healthy, healthy_repository) = fixture()?;
    let healthy_project = healthy
        .initialize_project("Healthy empty Candidates", Some(&healthy_repository))?
        .project
        .id;
    assert!(healthy
        .candidate_basis(healthy_project)?
        .candidates
        .is_empty());
    let healthy_projection = healthy.project_projection(healthy_project)?;
    assert_eq!(
        healthy_projection.candidate_dependency,
        CandidateDependencyState::Available
    );
    assert_eq!(healthy_projection.health, ProjectionHealth::Complete);
    assert!(healthy_projection.candidate_inspection.is_empty());

    for detected in ["0", "999"] {
        let (_unsupported_root, unsupported, unsupported_repository) = fixture()?;
        let unsupported_project = unsupported
            .initialize_project("Unsupported Candidates", Some(&unsupported_repository))?
            .project
            .id;
        Connection::open(unsupported.layout().candidate_store())?.execute(
            "UPDATE metadata SET value = ?1 WHERE key = 'schema_version'",
            [detected],
        )?;
        assert!(unsupported.candidate_basis(unsupported_project).is_err());
        assert_candidate_dependency(
            &unsupported.project_projection(unsupported_project)?,
            CandidateDependencyState::Unsupported,
            ProjectionIssueKind::CandidateUnsupported,
        );
    }

    let (_corrupt_root, corrupt, corrupt_repository) = fixture()?;
    let corrupt_project = corrupt
        .initialize_project("Corrupt Candidates", Some(&corrupt_repository))?
        .project
        .id;
    Connection::open(corrupt.layout().candidate_store())?.execute("DROP TABLE candidates", [])?;
    assert!(corrupt.candidate_basis(corrupt_project).is_err());
    assert_candidate_dependency(
        &corrupt.project_projection(corrupt_project)?,
        CandidateDependencyState::Corrupt,
        ProjectionIssueKind::CandidateCorrupt,
    );

    let (_unavailable_root, unavailable, unavailable_repository) = fixture()?;
    let unavailable_project = unavailable
        .initialize_project("Unavailable Candidates", Some(&unavailable_repository))?
        .project
        .id;
    let candidate_path = unavailable.layout().candidate_store();
    fs::remove_file(&candidate_path)?;
    fs::create_dir(&candidate_path)?;
    assert!(unavailable.candidate_basis(unavailable_project).is_err());
    assert_candidate_dependency(
        &unavailable.project_projection(unavailable_project)?,
        CandidateDependencyState::Unavailable,
        ProjectionIssueKind::CandidateUnavailable,
    );
    Ok(())
}

fn assert_candidate_dependency(
    projection: &volicord_projections::ProjectProjection,
    state: CandidateDependencyState,
    issue_kind: ProjectionIssueKind,
) {
    assert_eq!(projection.candidate_dependency, state);
    assert_eq!(projection.health, ProjectionHealth::Degraded);
    assert!(projection.candidate_inspection.is_empty());
    assert!(!projection.canonical_inspection.is_empty());
    assert!(projection.issues.iter().any(|issue| {
        issue.kind == issue_kind && issue.affected_scope == "candidate_inspection"
    }));
}

#[cfg(target_os = "linux")]
#[test]
fn runtime_initialization_enforces_private_managed_paths() -> Result<(), Box<dyn std::error::Error>>
{
    let (temporary, operations, _repository) = fixture()?;
    fs::create_dir(operations.layout().root())?;
    fs::set_permissions(
        operations.layout().root(),
        fs::Permissions::from_mode(0o777),
    )?;
    operations.initialize_runtime()?;

    for directory in [
        operations.layout().root().to_path_buf(),
        operations.layout().derived_dir(),
        operations.layout().analysis_dir(),
        operations.layout().artifacts_dir(),
    ] {
        assert_eq!(
            fs::symlink_metadata(&directory)?.permissions().mode() & 0o7777,
            0o700,
            "{}",
            directory.display()
        );
    }
    for file in [
        operations.layout().canonical_store(),
        operations.layout().candidate_store(),
        operations.layout().privacy_store(),
        operations.layout().guarded_store(),
        operations.layout().forgetting_store(),
        operations.layout().mutation_lock(),
    ] {
        assert_eq!(
            fs::symlink_metadata(&file)?.permissions().mode() & 0o7777,
            0o600,
            "{}",
            file.display()
        );
    }

    let unsafe_runtime = temporary.path().join("unsafe-runtime");
    let target = temporary.path().join("unsafe-target");
    fs::create_dir(&target)?;
    symlink(&target, &unsafe_runtime)?;
    let unsafe_operations = LocalOperations::new(RuntimeLayout::new(&unsafe_runtime)?);
    assert_eq!(unsafe_operations.health(None).state, HealthState::Failed);
    assert!(!target.join("canonical.sqlite3").exists());
    Ok(())
}

fn grounded_draft(
    project_id: ProjectId,
    goal_context_id: ContextItemId,
    baseline_analysis_snapshot_id: AnalysisSnapshotId,
) -> GroundedCheckpointDraft {
    GroundedCheckpointDraft {
        project_id,
        goal_context_id,
        baseline_analysis_snapshot_id,
        kind: CheckpointKind::Handoff,
        work_state: WorkState::Paused,
        state_change: Some("Recorded the bounded repository work".into()),
        applied_decisions: Vec::new(),
        decision_components: Vec::new(),
        work_contexts: Vec::new(),
        met_revisit_triggers: Vec::new(),
        verification: vec![CommandVerificationDraft {
            state: VerificationState::NotRun,
            command_label: None,
            exit_code: None,
            termination: None,
            outcome: None,
        }],
        known_limits: Vec::new(),
        non_goals: Vec::new(),
        next_step: "Continue from the grounded Checkpoint".into(),
        handoff_to: Some("next Codex session".into()),
    }
}

fn goal_and_baseline(
    operations: &LocalOperations,
    project_id: ProjectId,
) -> Result<(ContextItemId, AnalysisSnapshotId), Box<dyn std::error::Error>> {
    let goal = operations.record_current_host_user_context(
        project_id,
        "codex".into(),
        "baseline-dirty-regression".into(),
        "Record the bounded repository work".into(),
        ContextItemRole::Goal,
        "Record the bounded repository work".into(),
    )?;
    let baseline = operations
        .analyze(project_id, Vec::new())?
        .value
        .ok_or("baseline analysis has no value")?
        .analysis;
    Ok((goal.context_item_id, baseline.identity))
}

#[cfg(unix)]
#[test]
fn repository_bound_project_resolution_is_normalized_explicit_and_read_only(
) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::symlink;

    let (temporary, operations, repository) = fixture()?;
    let runtime_root = operations.layout().root().to_path_buf();
    let canonical_store = operations.layout().canonical_store();

    assert!(!runtime_root.exists());
    assert_eq!(
        operations.resolve_project(&repository)?,
        ProjectResolution::NotFound {
            canonical_repository_path: fs::canonicalize(&repository)?,
        }
    );
    assert!(!runtime_root.exists());
    for runtime_state in [
        canonical_store.clone(),
        operations.layout().candidate_store(),
        operations.layout().privacy_store(),
        operations.layout().guarded_store(),
        operations.layout().forgetting_store(),
        operations.layout().derived_dir(),
        operations.layout().artifacts_dir(),
    ] {
        assert!(
            !runtime_state.exists(),
            "{} was created",
            runtime_state.display()
        );
    }

    let initialized = operations.initialize_project("Bound", Some(&repository))?;
    let bound = initialized.binding.ok_or("Project was not bound")?;
    let before_resolution = operations.canonical_basis(initialized.project.id)?;
    let store_bytes_before_resolution = fs::read(&canonical_store)?;

    let unbound_repository = temporary.path().join("unbound-repository");
    fs::create_dir(&unbound_repository)?;
    assert_eq!(
        operations.resolve_project(&unbound_repository)?,
        ProjectResolution::NotFound {
            canonical_repository_path: fs::canonicalize(&unbound_repository)?,
        }
    );
    assert_eq!(store_bytes_before_resolution, fs::read(&canonical_store)?);

    let alias = temporary.path().join("repository-alias");
    symlink(&repository, &alias)?;
    let resolved = operations.resolve_project(&alias)?;
    let ProjectResolution::Found { project, binding } = resolved else {
        return Err("bound repository was not resolved".into());
    };
    assert_eq!(project, initialized.project);
    assert_eq!(binding, bound);
    assert_eq!(
        binding.binding.absolute_path,
        fs::canonicalize(&repository)?
    );

    let after_resolution = operations.canonical_basis(initialized.project.id)?;
    assert_eq!(before_resolution, after_resolution);
    assert_eq!(store_bytes_before_resolution, fs::read(&canonical_store)?);
    Ok(())
}

#[cfg(target_os = "linux")]
fn initialize_git(repository: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    for arguments in [
        vec!["init", "--quiet"],
        vec!["config", "user.email", "fixture@example.invalid"],
        vec!["config", "user.name", "Volicord Fixture"],
        vec!["add", "."],
        vec!["commit", "--quiet", "-m", "fixture baseline"],
    ] {
        if !Command::new("git")
            .arg("-C")
            .arg(repository)
            .args(arguments)
            .status()?
            .success()
        {
            return Err("Git fixture setup failed".into());
        }
    }
    Ok(())
}

#[test]
fn project_analysis_recall_and_portable_io_use_current_owners(
) -> Result<(), Box<dyn std::error::Error>> {
    let (temporary, operations, repository) = fixture()?;
    let initialized = operations.initialize_project("Fixture", Some(&repository))?;
    assert!(initialized.binding.is_some());

    let analysis = operations.analyze(initialized.project.id, Vec::new())?;
    assert!(matches!(
        analysis.state,
        OperationState::Succeeded | OperationState::Partial
    ));
    let analysis_value = analysis.value.as_ref().ok_or("missing analysis value")?;
    assert!(analysis_value
        .stored_at
        .starts_with(operations.layout().analysis_dir()));
    assert!(!analysis_value.analysis.inventory.entries.is_empty());

    let before = operations.canonical_basis(initialized.project.id)?;
    let recall = operations.recall(initialized.project.id)?;
    assert_eq!(recall.project_id, initialized.project.id);
    let after = operations.canonical_basis(initialized.project.id)?;
    assert_eq!(
        before.stable_ordering_identity,
        after.stable_ordering_identity
    );

    let bundle = temporary.path().join("fixture.volicord.json");
    let exported = operations.export_bundle(initialized.project.id, &bundle)?;
    assert_eq!(exported.project_id, initialized.project.id);
    assert!(bundle.is_file());
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn clean_git_baseline_attributes_a_later_changed_file() -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, repository) = fixture()?;
    initialize_git(&repository)?;
    let project = operations
        .initialize_project("Clean Git Fixture", Some(&repository))?
        .project;
    let (goal, baseline) = goal_and_baseline(&operations, project.id)?;
    fs::write(
        repository.join("src/lib.rs"),
        "pub fn changed() -> u32 { 43 }\n",
    )?;

    let checkpoint =
        operations.record_grounded_checkpoint(grounded_draft(project.id, goal, baseline))?;
    assert_eq!(checkpoint.changed_paths, ["src/lib.rs"]);
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn resumed_pre_write_baseline_attributes_source_document_and_configuration_without_python_caches(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, repository) = fixture()?;
    initialize_git(&repository)?;
    let project = operations
        .initialize_project("Resume attribution fixture", Some(&repository))?
        .project;
    let goal = operations.record_current_host_user_context(
        project.id,
        "codex".into(),
        "fresh-resume-attribution".into(),
        "Continue the bounded repository work".into(),
        ContextItemRole::Goal,
        "Continue the bounded repository work".into(),
    )?;

    let ProjectResolution::Found {
        project: resolved, ..
    } = operations.resolve_project(&repository)?
    else {
        return Err("fresh resume did not resolve the existing Project".into());
    };
    assert_eq!(resolved.id, project.id);
    assert_eq!(
        operations.recall(resolved.id)?.goals_and_why[0].statement,
        "Continue the bounded repository work"
    );
    let baseline = operations
        .analyze(resolved.id, Vec::new())?
        .value
        .ok_or("resume baseline analysis has no value")?
        .analysis
        .identity;

    fs::write(
        repository.join("src/lib.rs"),
        "pub fn resumed() -> u32 { 43 }\n",
    )?;
    fs::write(repository.join("README.md"), "# Resumed work\n")?;
    fs::write(
        repository.join("pyproject.toml"),
        "[tool.pytest.ini_options]\n",
    )?;
    fs::create_dir_all(repository.join(".pytest_cache/v/cache"))?;
    fs::write(repository.join(".pytest_cache/v/cache/nodeids"), "[]\n")?;
    fs::create_dir_all(repository.join(".mypy_cache/3.12"))?;
    fs::write(repository.join(".mypy_cache/3.12/module.json"), "{}\n")?;

    let checkpoint = operations.record_grounded_checkpoint(grounded_draft(
        resolved.id,
        goal.context_item_id,
        baseline,
    ))?;
    assert_eq!(
        checkpoint.changed_paths,
        ["README.md", "pyproject.toml", "src/lib.rs"]
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn generated_python_cache_only_activity_has_no_meaningful_checkpoint_changed_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, repository) = fixture()?;
    initialize_git(&repository)?;
    let project = operations
        .initialize_project("Cache-only attribution fixture", Some(&repository))?
        .project;
    let (goal, baseline) = goal_and_baseline(&operations, project.id)?;

    fs::create_dir_all(repository.join(".pytest_cache/v/cache"))?;
    fs::write(repository.join(".pytest_cache/v/cache/nodeids"), "[]\n")?;
    fs::create_dir_all(repository.join(".mypy_cache/3.12"))?;
    fs::write(repository.join(".mypy_cache/3.12/module.json"), "{}\n")?;

    let checkpoint =
        operations.record_grounded_checkpoint(grounded_draft(project.id, goal, baseline))?;
    assert!(checkpoint.changed_paths.is_empty());
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn unchanged_pre_existing_dirty_path_is_not_current_work() -> Result<(), Box<dyn std::error::Error>>
{
    let (_temporary, operations, repository) = fixture()?;
    initialize_git(&repository)?;
    let project = operations
        .initialize_project("Dirty Git Fixture", Some(&repository))?
        .project;
    fs::write(repository.join("src/lib.rs"), "pub fn dirty_before() {}\n")?;
    let (goal, baseline) = goal_and_baseline(&operations, project.id)?;

    let checkpoint =
        operations.record_grounded_checkpoint(grounded_draft(project.id, goal, baseline))?;
    assert!(checkpoint.changed_paths.is_empty());
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn pre_existing_tracked_path_changed_again_is_ambiguous() -> Result<(), Box<dyn std::error::Error>>
{
    let (_temporary, operations, repository) = fixture()?;
    initialize_git(&repository)?;
    let project = operations
        .initialize_project("Tracked Ambiguity Fixture", Some(&repository))?
        .project;
    fs::write(repository.join("src/lib.rs"), "pub fn dirty_before() {}\n")?;
    let (goal, baseline) = goal_and_baseline(&operations, project.id)?;
    fs::write(repository.join("src/lib.rs"), "pub fn changed_again() {}\n")?;

    let before = operations.canonical_basis(project.id)?.latest_checkpoint;
    let error = operations
        .record_grounded_checkpoint(grounded_draft(project.id, goal, baseline))
        .expect_err("a baseline-dirty tracked path changed again must be ambiguous");
    assert!(error
        .message()
        .contains("dirty at the baseline changed again"));
    assert_eq!(
        operations.canonical_basis(project.id)?.latest_checkpoint,
        before
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn pre_existing_untracked_path_changed_again_is_ambiguous() -> Result<(), Box<dyn std::error::Error>>
{
    let (_temporary, operations, repository) = fixture()?;
    initialize_git(&repository)?;
    let project = operations
        .initialize_project("Untracked Ambiguity Fixture", Some(&repository))?
        .project;
    fs::write(repository.join("draft.rs"), "fn first_untracked() {}\n")?;
    let (goal, baseline) = goal_and_baseline(&operations, project.id)?;
    fs::write(repository.join("draft.rs"), "fn changed_untracked() {}\n")?;

    let error = operations
        .record_grounded_checkpoint(grounded_draft(project.id, goal, baseline))
        .expect_err("a baseline-untracked path changed again must be ambiguous");
    assert!(error
        .message()
        .contains("dirty at the baseline changed again"));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn unrelated_clean_path_remains_attributable_with_unchanged_dirty_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, repository) = fixture()?;
    initialize_git(&repository)?;
    let project = operations
        .initialize_project("Mixed Dirty Fixture", Some(&repository))?
        .project;
    fs::write(repository.join("src/lib.rs"), "pub fn dirty_before() {}\n")?;
    let (goal, baseline) = goal_and_baseline(&operations, project.id)?;
    fs::write(repository.join("src/new.rs"), "pub fn bounded_work() {}\n")?;

    let checkpoint =
        operations.record_grounded_checkpoint(grounded_draft(project.id, goal, baseline))?;
    assert_eq!(checkpoint.changed_paths, ["src/new.rs"]);
    Ok(())
}

#[test]
fn non_git_repository_keeps_snapshot_delta_attribution() -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, repository) = fixture()?;
    let project = operations
        .initialize_project("Non-Git Fixture", Some(&repository))?
        .project;
    let (goal, baseline) = goal_and_baseline(&operations, project.id)?;
    fs::write(
        repository.join("src/lib.rs"),
        "pub fn non_git_change() {}\n",
    )?;

    let checkpoint =
        operations.record_grounded_checkpoint(grounded_draft(project.id, goal, baseline))?;
    assert_eq!(checkpoint.changed_paths, ["src/lib.rs"]);
    Ok(())
}

#[test]
fn exact_current_host_goal_context_round_trips_through_recall(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, repository) = fixture()?;
    let initialized = operations.initialize_project("Context Fixture", Some(&repository))?;
    let user_turn = "The current goal is to expose exact user context to Recall.";
    let statement = "expose exact user context to Recall";
    let recorded = operations.record_current_host_user_context(
        initialized.project.id,
        "codex".into(),
        "session-context-fixture".into(),
        user_turn.into(),
        ContextItemRole::Goal,
        statement.into(),
    )?;
    let canonical = operations.canonical_basis(initialized.project.id)?;
    let item = canonical
        .context_items
        .iter()
        .find(|item| item.id == recorded.context_item_id)
        .ok_or("recorded Context Item is missing")?;
    assert_eq!(item.statement, statement);
    assert_eq!(item.source_basis, vec![recorded.source_id]);
    let source = canonical
        .sources
        .iter()
        .find(|source| source.source.id == recorded.source_id)
        .ok_or("recorded user Source is missing")?;
    assert_eq!(source.source.actor.kind, PrincipalKind::User);
    assert!(matches!(
        source.source.payload,
        SourcePayload::CurrentHostUserTurn { ref turn, .. } if turn == user_turn
    ));
    assert_eq!(
        operations.recall(initialized.project.id)?.goals_and_why[0].statement,
        statement
    );

    let before = operations
        .canonical_basis(initialized.project.id)?
        .sources
        .len();
    let error = operations
        .record_current_host_user_context(
            initialized.project.id,
            "codex".into(),
            "session-context-fixture".into(),
            "The user stated only this sentence.".into(),
            ContextItemRole::Goal,
            "Agent-authored text outside the user turn".into(),
        )
        .expect_err("non-verbatim agent text must be rejected");
    assert!(error.message().contains("occur verbatim"));
    assert_eq!(
        operations
            .canonical_basis(initialized.project.id)?
            .sources
            .len(),
        before
    );
    Ok(())
}

#[test]
fn grounded_checkpoint_rejects_passed_verification_without_execution_before_mutation(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, repository) = fixture()?;
    let project = operations
        .initialize_project("Verification Fixture", Some(&repository))?
        .project;
    let before = operations.canonical_basis(project.id)?;
    let error = operations
        .record_grounded_checkpoint(GroundedCheckpointDraft {
            project_id: project.id,
            goal_context_id: ContextItemId::from_bytes([91; 16]),
            baseline_analysis_snapshot_id: AnalysisSnapshotId::from_hex(&"00".repeat(32))?,
            kind: CheckpointKind::Handoff,
            work_state: WorkState::Paused,
            state_change: None,
            applied_decisions: Vec::new(),
            decision_components: Vec::new(),
            work_contexts: Vec::new(),
            met_revisit_triggers: Vec::new(),
            verification: vec![CommandVerificationDraft {
                state: VerificationState::Passed,
                command_label: None,
                exit_code: None,
                termination: None,
                outcome: None,
            }],
            known_limits: Vec::new(),
            non_goals: Vec::new(),
            next_step: "Run an actual verification command".into(),
            handoff_to: Some("next Codex session".into()),
        })
        .expect_err("passed verification without execution must be rejected");
    assert!(error.message().contains("command label"));
    assert_eq!(operations.canonical_basis(project.id)?, before);
    Ok(())
}

#[test]
fn repository_failure_degrades_health_without_canonical_loss(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, repository) = fixture()?;
    let initialized = operations.initialize_project("Fixture", Some(&repository))?;
    fs::remove_dir_all(&repository)?;

    assert!(operations
        .analyze(initialized.project.id, Vec::new())
        .is_err());
    let canonical = operations.canonical_basis(initialized.project.id)?;
    assert_eq!(canonical.project.display_name, "Fixture");
    let health = operations.health(Some(initialized.project.id));
    assert_eq!(health.state, HealthState::Degraded);
    assert_eq!(health.repository_available, Some(false));
    Ok(())
}

#[test]
fn viewer_snapshot_publication_is_absolute_atomic_no_replace_and_noncanonical(
) -> Result<(), Box<dyn std::error::Error>> {
    let (temporary, operations, repository) = fixture()?;
    let project = operations
        .initialize_project("Snapshot publication", Some(&repository))?
        .project
        .id;
    let canonical_before = operations.canonical_basis(project)?;
    let destination = temporary.path().join("shared/viewer-snapshot.html");
    let html = "<!doctype html><html><body>read-only snapshot</body></html>";

    let published = operations.publish_viewer_snapshot(html, &destination)?;
    assert_eq!(published.destination, destination);
    assert_eq!(published.bytes, html.len() as u64);
    assert_eq!(fs::read_to_string(&destination)?, html);
    assert_eq!(operations.canonical_basis(project)?, canonical_before);

    let replacement = operations
        .publish_viewer_snapshot("replacement", &destination)
        .expect_err("snapshot publication must not replace an existing file");
    assert!(replacement
        .message()
        .contains("publication destination already exists"));
    assert_eq!(fs::read_to_string(&destination)?, html);

    let relative = operations
        .publish_viewer_snapshot(html, Path::new("viewer-snapshot.html"))
        .expect_err("relative snapshot destination must be rejected");
    assert!(relative.message().contains("must be absolute"));
    assert_eq!(operations.canonical_basis(project)?, canonical_before);
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn child_process_preserves_complete_streams_exit_and_duration(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, _repository) = fixture()?;
    let result = operations.run_child(
        "/bin/sh",
        [
            OsString::from("-c"),
            OsString::from("printf stdout; printf stderr >&2; exit 7"),
        ],
        None,
        vec!["fixture-command".into()],
        Duration::from_secs(2),
        CancellationFlag::default(),
    )?;
    assert_eq!(result.state, OperationState::Failed);
    assert!(result.duration_micros > 0);
    let value = result.value.as_ref().ok_or("missing child observation")?;
    assert_eq!(value.termination, ProcessTermination::ExitCode(7));
    assert_eq!(value.cleanup, ProcessTreeCleanup::NotRequired);
    assert_eq!(fs::read(value.stdout.path())?, b"stdout");
    assert_eq!(fs::read(value.stderr.path())?, b"stderr");
    assert_eq!(value.stdout.bytes(), 6);
    assert_eq!(value.stderr.bytes(), 6);
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn timeout_detection_remains_separate_from_confirmed_termination(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, _repository) = fixture()?;
    let result = operations.run_child(
        "/bin/sh",
        [
            OsString::from("-c"),
            OsString::from("printf partial; sleep 5"),
        ],
        None,
        vec!["timeout-fixture".into()],
        Duration::from_millis(20),
        CancellationFlag::default(),
    )?;
    assert_eq!(result.state, OperationState::TimedOut);
    let value = result.value.as_ref().ok_or("missing child observation")?;
    assert!(value.timeout_detected);
    assert_ne!(value.termination, ProcessTermination::Unknown);
    assert_eq!(fs::read(value.stdout.path())?, b"partial");
    Ok(())
}

#[test]
fn unsupported_repair_scope_fails_without_impersonating_canonical_repair(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, repository) = fixture()?;
    let initialized = operations.initialize_project("Fixture", Some(&repository))?;
    let error = operations
        .repair(initialized.project.id, "canonical", Vec::new())
        .expect_err("canonical repair must not be claimed");
    assert!(error.message().contains("unsupported repair scope"));
    Ok(())
}

#[test]
fn project_ids_remain_path_independent() -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, repository) = fixture()?;
    let initialized = operations.initialize_project("Fixture", Some(&repository))?;
    assert_ne!(initialized.project.id, ProjectId::from_bytes([0; 16]));
    assert!(!initialized
        .project
        .id
        .to_string()
        .contains(repository.to_string_lossy().as_ref()));
    Ok(())
}
