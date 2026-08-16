use std::{ffi::OsString, fs, process::Command, time::Duration};
use tempfile::TempDir;
use volicord_context::{
    CheckpointKind, ContextItemId, ContextItemRole, PrincipalKind, ProjectId, SourcePayload,
    VerificationState, WorkState,
};
use volicord_local_platform::{CancellationFlag, ProcessTermination, ProcessTreeCleanup};
use volicord_operations::{
    CommandVerificationDraft, GroundedCheckpointDraft, HealthState, LocalOperations,
    OperationState, RuntimeLayout,
};
use volicord_repository_intelligence::AnalysisSnapshotId;

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
