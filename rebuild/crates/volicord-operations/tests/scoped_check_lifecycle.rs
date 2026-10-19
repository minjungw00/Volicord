//! Controlled adverse lifecycle cases. Actual child outcomes are not user capture.
#[path = "support/scoped_check_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use fixture::*;
use std::{error::Error, fs, time::Duration};
use volicord_context::*;
use volicord_operations::*;
use volicord_projections::*;
fn partial(work: ContextItemId, source: SourceId) -> WorkCheckRecording {
    WorkCheckRecording {
        work_item_id: work,
        source_id: source,
        assertion_state: AssertionState::NotObserved,
        assertion_evidence: None,
        retest: None,
        outcome: "Execution observed; no reliable assertion claimed".into(),
        next_step: "Inspect original limitations".into(),
    }
}
fn link(receipt: &WorkCheckReceipt, relation: CheckTargetRelation) -> CheckRetestBasis {
    CheckRetestBasis {
        checkpoint_id: receipt.checkpoint_id,
        checkpoint_revision: receipt.checkpoint_revision,
        verification_index: 0,
        source_id: receipt.source_id,
        target_relation: relation,
        rationale: "Explicit task-owned comparison request; original evidence determines support"
            .into(),
    }
}
#[test]
fn spawn_failure_timeout_oversized_output_and_exit_zero_without_assertion_are_distinct(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let mut missing = command(&f.ops, f.project, f.work)?;
    missing.program = "/definitely-absent-volicord-check-program".into();
    let before = f.ops.canonical_basis(f.project)?;
    assert!(f.ops.run_work_check(missing, declaration()).is_err());
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    for timed in [false, true] {
        let mut request = command(&f.ops, f.project, f.work)?;
        request.arguments = vec![
            "-B".into(),
            "-c".into(),
            if timed {
                "import time; print('started',flush=True); time.sleep(30)".into()
            } else {
                "print('output only; no assertions'); print('x'*20000)".into()
            },
        ];
        if timed {
            request.timeout = Duration::from_millis(150);
        }
        let source = f.ops.run_work_check(request, declaration())?.source_id;
        let read = f.ops.read_explanation_source(f.project, f.work, source)?;
        assert_eq!(read["observation"]["detail"]["timeout_detected"], timed);
        if !timed {
            assert_eq!(read["observation"]["detail"]["exit_code"], 0);
            assert_eq!(
                read["observation"]["detail"]["stdout"]["representation"],
                "bounded_excerpt"
            );
            assert!(
                read["observation"]["detail"]["stdout"]["omitted_bytes"]
                    .as_u64()
                    .ok_or("omission")?
                    > 0
            );
        }
        f.ops
            .record_work_check(f.project, partial(f.work, source))?;
        let report = f.ops.read_work_verification(f.project, f.work)?;
        let check = report.checks.last().ok_or("check")?;
        assert_eq!(check.reported_state, ReportedCheckState::Partial);
        assert_eq!(check.assertion_result, EvidenceDetail::NotObserved);
        assert_eq!(report.checks[0].reported_state, ReportedCheckState::Failed);
        assert_eq!(
            f.ops
                .canonical_basis(f.project)?
                .latest_checkpoint
                .ok_or("cp")?
                .user_acceptance
                .state,
            UserAcceptanceState::NotRequested
        );
    }
    let undeclared = f
        .ops
        .run_work_command(command(&f.ops, f.project, f.work)?)?
        .source_id;
    assert!(f
        .ops
        .record_work_check(f.project, recording(f.work, undeclared, true))
        .is_err());
    Ok(())
}
#[test]
fn stale_selected_target_never_executes_and_target_changes_cannot_support_comparison(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let root = f.temp.path().join("repo");
    let mut stale = command(&f.ops, f.project, f.work)?;
    stale.arguments = vec![
        "-c".into(),
        "from pathlib import Path; Path('should-not-execute').touch()".into(),
    ];
    fs::write(root.join("main.py"), "def value():\n    return 8\n")?;
    let before = f.ops.canonical_basis(f.project)?;
    assert!(f.ops.run_work_check(stale, declaration()).is_err());
    assert!(!root.join("should-not-execute").exists());
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    fs::write(root.join("main.py"), "def value():\n    return 7\n")?;
    let mut changing = command(&f.ops, f.project, f.work)?;
    changing.arguments=vec!["-c".into(),"from pathlib import Path; Path('main.py').write_text('def value():\\n    return 9\\n'); print('target changed during execution')".into()];
    let source = f.ops.run_work_check(changing, declaration())?.source_id;
    let mut request = partial(f.work, source);
    request.retest = Some(link(&f.failed, CheckTargetRelation::ChangedArtifacts));
    let receipt = f.ops.record_work_check(f.project, request)?;
    let report = f.ops.read_work_verification(f.project, f.work)?;
    let check = report
        .checks
        .iter()
        .find(|c| c.original.checkpoint_id == receipt.checkpoint_id.to_string())
        .ok_or("check")?;
    let EvidenceDetail::Observed(run) = &check.scoped_run else {
        return Err("run".into());
    };
    assert!(!run.target_unchanged_during_run);
    assert!(matches!(
        check.retest_comparison,
        RetestComparison::NotComparable { .. }
    ));
    assert_eq!(
        report
            .executions
            .iter()
            .find(|e| e.source_id == source.to_string())
            .ok_or("run")?
            .exit_code,
        Some(0)
    );
    Ok(())
}
#[test]
fn no_change_repeat_is_explicitly_comparable_but_correction_does_not_rebind_old_runs(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let source = f
        .ops
        .run_work_check(command(&f.ops, f.project, f.work)?, declaration())?
        .source_id;
    let mut request = recording(f.work, source, true);
    request.retest = Some(link(&f.passed, CheckTargetRelation::SameArtifacts));
    let receipt = f.ops.record_work_check(f.project, request)?;
    let report = f.ops.read_work_verification(f.project, f.work)?;
    assert!(matches!(
        report.checks.last().ok_or("check")?.retest_comparison,
        RetestComparison::Comparable {
            target_relation: CheckTargetRelation::SameArtifacts,
            ..
        }
    ));
    let before = f.ops.canonical_basis(f.project)?;
    assert!(before
        .checkpoint_history
        .iter()
        .find(|c| c.id == receipt.checkpoint_id)
        .ok_or("cp")?
        .changed_paths
        .is_empty());
    let goal = before
        .context_items
        .iter()
        .find(|c| c.id == f.work)
        .ok_or("goal")?;
    f.ops.correct_context_item(
        f.project,
        f.work,
        ContextItemCorrectionDraft {
            expected_revision: goal.revision,
            corrected_statement: format!("{}.", goal.statement),
            user_authorization_source_id: goal.source_basis[0],
            kind: CorrectionKind::Expression,
        },
    )?;
    assert!(f
        .ops
        .record_work_check(f.project, recording(f.work, source, true))
        .is_err());
    let after = f.ops.read_work_verification(f.project, f.work)?;
    assert_eq!(after.checks, report.checks);
    assert_eq!(
        after
            .executions
            .iter()
            .find(|e| e.source_id == source.to_string())
            .ok_or("execution")?
            .body_state,
        CommandBodyState::Changed
    );
    let source = f
        .ops
        .run_work_check(command(&f.ops, f.project, f.work)?, declaration())?
        .source_id;
    let mut request = recording(f.work, source, true);
    request.retest = Some(link(&f.passed, CheckTargetRelation::SameArtifacts));
    f.ops.record_work_check(f.project, request)?;
    let report = f.ops.read_work_verification(f.project, f.work)?;
    assert!(matches!(
        report.checks.last().ok_or("check")?.retest_comparison,
        RetestComparison::NotComparable {
            reason: ComparisonLimit::PriorRunUnavailable,
            ..
        }
    ));
    Ok(())
}
#[test]
fn real_expiry_and_explicit_forgetting_keep_historical_reports_without_reviving_output(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let mut command = command(&f.ops, f.project, f.work)?;
    command.retain_seconds = 1;
    let source = f.ops.run_work_check(command, declaration())?.source_id;
    let receipt = f
        .ops
        .record_work_check(f.project, recording(f.work, source, true))?;
    let before = f.ops.read_work_verification(f.project, f.work)?;
    let canonical = f.ops.canonical_basis(f.project)?;
    std::thread::sleep(Duration::from_millis(1100));
    let after = f.ops.read_work_verification(f.project, f.work)?;
    assert_eq!(before.checks, after.checks);
    assert_eq!(canonical, f.ops.canonical_basis(f.project)?);
    let execution = after
        .executions
        .iter()
        .find(|e| e.source_id == source.to_string())
        .ok_or("execution")?;
    assert_eq!(execution.body_state, CommandBodyState::Expired);
    assert_eq!(execution.observation, EvidenceDetail::Unavailable);
    assert_eq!(execution.exit_code, Some(0));
    assert!(f
        .ops
        .record_work_check(f.project, recording(f.work, source, true))
        .is_err());
    let auth = canonical
        .context_items
        .iter()
        .find(|c| c.id == f.work)
        .ok_or("goal")?
        .source_basis[0];
    f.ops.forget_record(
        f.project,
        CanonicalRecordId::Source(f.failed.source_id),
        auth,
    )?;
    let report = f.ops.read_work_verification(f.project, f.work)?;
    assert_eq!(report.checks[0].reported_state, ReportedCheckState::Failed);
    assert!(matches!(
        report.checks[1].retest_comparison,
        RetestComparison::NotComparable {
            reason: ComparisonLimit::PriorRunUnavailable,
            ..
        }
    ));
    assert_eq!(
        report
            .checks
            .iter()
            .find(|c| c.original.checkpoint_id == receipt.checkpoint_id.to_string())
            .ok_or("expired canonical check")?
            .reported_state,
        ReportedCheckState::Passed
    );
    let bundle = f.temp.path().join("forgotten-source.json");
    f.ops.export_bundle(f.project, &bundle)?;
    let imported = LocalOperations::new(RuntimeLayout::new(f.temp.path().join("imported"))?);
    imported.import_bundle(&bundle)?;
    assert_eq!(
        imported.canonical_basis(f.project)?.checkpoint_history,
        f.ops.canonical_basis(f.project)?.checkpoint_history
    );
    Ok(())
}

#[test]
fn same_named_check_in_another_work_cannot_retest_or_resolve_the_original_failure(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let original = f.ops.read_work_verification(f.project, f.work)?;
    let source = f
        .ops
        .run_work_check(command(&f.ops, f.project, f.other)?, declaration())?
        .source_id;
    let mut request = recording(f.other, source, true);
    request.retest = Some(link(&f.failed, CheckTargetRelation::ChangedArtifacts));
    let before = f.ops.canonical_basis(f.project)?;
    assert!(f.ops.record_work_check(f.project, request.clone()).is_err());
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    request.retest = None;
    f.ops.record_work_check(f.project, request)?;
    let other = f.ops.read_work_verification(f.project, f.other)?;
    assert_eq!(other.checks.len(), 1);
    assert_eq!(other.checks[0].source_id, Some(source.to_string()));
    assert_eq!(
        other.checks[0].retest_comparison,
        RetestComparison::Unresolved
    );
    assert_eq!(f.ops.read_work_verification(f.project, f.work)?, original);
    Ok(())
}
