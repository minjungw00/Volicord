use std::collections::BTreeMap;

use super::*;
use crate::pipeline::TransitionSubmission;
use volicord_types::values::{
    NoCommitPlannedBranch, WorkflowActionSemanticVariant, WorkflowTransitionEffectClass,
};

fn action_key(
    method: MethodName,
    semantic_variant: WorkflowActionSemanticVariant,
) -> WorkflowActionKey {
    WorkflowActionKey::new(method, semantic_variant).expect("test action key must be exact")
}

fn repository_snapshot(path: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn Error>> {
    fn collect(
        root: &Path,
        current: &Path,
        files: &mut BTreeMap<PathBuf, Vec<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                collect(root, &path, files)?;
            } else {
                files.insert(path.strip_prefix(root)?.to_path_buf(), fs::read(path)?);
            }
        }
        Ok(())
    }

    let mut files = BTreeMap::new();
    collect(path, path, &mut files)?;
    Ok(files)
}

#[test]
fn exact_no_commit_plans_name_every_accepted_pipeline_branch_without_effects(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "no_commit_branches")?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;
    let repo_root = product_repo_root(&harness)?;
    let before_repository = repository_snapshot(&repo_root)?;

    let record_run = record_run_request(
        "req_plan_commit",
        "idem_plan_commit",
        false,
        Some(before.state_version),
        &task_id,
        &change_unit_id,
    );
    let commit = harness.service.plan_transition_submission_no_commit(
        &harness.service.context(),
        action_key(
            MethodName::RecordRun,
            WorkflowActionSemanticVariant::RecordRun,
        ),
        TransitionSubmission::RecordRun(record_run),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(commit.planned_branch, NoCommitPlannedBranch::CommitMutation);
    assert_eq!(
        commit.effect_class,
        WorkflowTransitionEffectClass::ExecutionRecording
    );

    let close = close_task_request(CloseTaskFixture {
        request_id: "req_plan_no_effect",
        idempotency_key: Some("idem_plan_no_effect"),
        dry_run: false,
        expected_state_version: Some(before.state_version),
        task_id: &task_id,
        intent: CloseIntent::Complete,
        close_reason: Some(CloseReason::CompletedSelfChecked),
        superseding_task_id: None,
    });
    let no_effect = harness.service.plan_transition_submission_no_commit(
        &harness.service.context(),
        action_key(
            MethodName::CloseTask,
            WorkflowActionSemanticVariant::CloseTask,
        ),
        TransitionSubmission::CloseTask(close),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        no_effect.planned_branch,
        NoCommitPlannedBranch::NormalNoEffectResult
    );
    assert_eq!(
        no_effect.effect_class,
        WorkflowTransitionEffectClass::TerminalMutation
    );

    let check = check_close_request(CloseTaskFixture {
        request_id: "req_plan_read_only",
        idempotency_key: None,
        dry_run: false,
        expected_state_version: None,
        task_id: &task_id,
        intent: CloseIntent::Check,
        close_reason: None,
        superseding_task_id: None,
    });
    let read_only = harness.service.plan_transition_submission_no_commit(
        &harness.service.context(),
        action_key(
            MethodName::CheckClose,
            WorkflowActionSemanticVariant::CheckClose,
        ),
        TransitionSubmission::CheckClose(check),
        invocation(OperationCategory::Read),
    )?;
    assert_eq!(
        read_only.planned_branch,
        NoCommitPlannedBranch::ReadOnlyResult
    );
    assert_eq!(
        read_only.effect_class,
        WorkflowTransitionEffectClass::ReadOnlyAssessment
    );

    let artifact = stage_artifact_request(
        "req_plan_artifact",
        None,
        false,
        Some(before.state_version),
        &task_id,
    );
    let staging = harness.service.plan_transition_submission_no_commit(
        &harness.service.context(),
        action_key(
            MethodName::StageArtifact,
            WorkflowActionSemanticVariant::StageArtifact,
        ),
        TransitionSubmission::StageArtifact(artifact),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(
        staging.planned_branch,
        NoCommitPlannedBranch::ArtifactStaging
    );
    assert_eq!(
        staging.effect_class,
        WorkflowTransitionEffectClass::ArtifactStaging
    );

    assert_eq!(
        serde_json::to_value(&commit)?["planned_branch"],
        "commit_mutation"
    );
    assert_eq!(
        serde_json::to_value(&no_effect)?["planned_branch"],
        "normal_no_effect_result"
    );
    assert_eq!(
        serde_json::to_value(&read_only)?["planned_branch"],
        "read_only_result"
    );
    assert_eq!(
        serde_json::to_value(&staging)?["planned_branch"],
        "artifact_staging"
    );
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(repository_snapshot(&repo_root)?, before_repository);
    Ok(())
}

#[test]
fn incompatible_record_run_is_a_typed_no_commit_rejection_without_any_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "no_commit_run_rejection")?;
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;
    let repo_root = product_repo_root(&harness)?;
    let before_repository = repository_snapshot(&repo_root)?;
    let mut request = record_run_request(
        "req_plan_bad_run",
        "idem_plan_bad_run",
        false,
        Some(before.state_version),
        &task_id,
        &change_unit_id,
    );
    request.kind = RunKind::Direct;

    let error = harness
        .service
        .plan_transition_submission_no_commit(
            &harness.service.context(),
            action_key(
                MethodName::RecordRun,
                WorkflowActionSemanticVariant::RecordRun,
            ),
            TransitionSubmission::RecordRun(request),
            invocation(OperationCategory::AgentWorkflow),
        )
        .expect_err("rejected method response must not become a no-commit plan");
    let CorePipelineError::NoCommitSubmissionRejected(rejection) = error else {
        panic!("expected typed no-commit rejection, got {error}");
    };
    assert_eq!(
        rejection.action_key(),
        action_key(
            MethodName::RecordRun,
            WorkflowActionSemanticVariant::RecordRun
        )
    );
    assert_eq!(
        rejection.method_error_code(),
        ErrorCode::RunKindIncompatible
    );
    assert_eq!(rejection.basis_state_version(), before.state_version);
    assert!(!rejection.state_change_applied());
    assert!(!rejection.committed());
    let details = rejection
        .method_error_details()
        .expect("run-kind rejection must retain typed details");
    assert_eq!(details["reason"], "run_kind_incompatible");
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(repository_snapshot(&repo_root)?, before_repository);
    Ok(())
}

#[test]
fn advisor_incompatible_change_unit_is_a_typed_no_commit_rejection_without_any_effect(
) -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let intake = harness.service.intake(
        intake_request(
            "req_plan_advisor_task",
            "idem_plan_advisor_task",
            false,
            Some(0),
            RequestedMode::Advisor,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let task_id = response_record_id(&intake.response_value, "task_ref");
    let before = harness.counts()?;
    let before_revision = task_revision(&harness, &task_id)?;
    let repo_root = product_repo_root(&harness)?;
    let before_repository = repository_snapshot(&repo_root)?;
    let request = update_scope_request(
        "req_plan_bad_advisor_scope",
        "idem_plan_bad_advisor_scope",
        false,
        Some(before.state_version),
        &task_id,
        ChangeUnitOperation::CreateCurrent,
        "A write-capable Change Unit is incompatible with Advisor mode.",
    );

    let error = harness
        .service
        .plan_transition_submission_no_commit(
            &harness.service.context(),
            action_key(
                MethodName::UpdateScope,
                WorkflowActionSemanticVariant::CreateCurrentChangeUnit,
            ),
            TransitionSubmission::UpdateScope(request),
            invocation(OperationCategory::AgentWorkflow),
        )
        .expect_err("Advisor-incompatible witness must not become a no-commit plan");
    let CorePipelineError::NoCommitSubmissionRejected(rejection) = error else {
        panic!("expected typed no-commit rejection, got {error}");
    };
    assert_eq!(
        rejection.action_key(),
        action_key(
            MethodName::UpdateScope,
            WorkflowActionSemanticVariant::CreateCurrentChangeUnit
        )
    );
    assert_eq!(rejection.basis_state_version(), before.state_version);
    assert!(rejection.method_error_details().is_some());
    assert!(!rejection.state_change_applied());
    assert!(!rejection.committed());
    assert_eq!(harness.counts()?, before);
    assert_eq!(task_revision(&harness, &task_id)?, before_revision);
    assert_eq!(repository_snapshot(&repo_root)?, before_repository);
    Ok(())
}
