#[path = "support/scoped_check_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use fixture::*;
use std::{error::Error, fs};
use volicord_context::*;

#[test]
fn captured_checkpoints_preserve_actual_assertions_targets_and_retest_across_restart_and_bundle(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let basis = f.ops.canonical_basis(f.project)?;
    assert_eq!(basis.checkpoint_history.len(), 2);
    let first = &basis.checkpoint_history[0];
    let second = &basis.checkpoint_history[1];
    assert_eq!(first.id, f.failed.checkpoint_id);
    assert_eq!(second.id, f.passed.checkpoint_id);
    assert_eq!(first.verification[0].state, VerificationState::Failed);
    assert_eq!(second.verification[0].state, VerificationState::Passed);
    let failed = first.verification[0]
        .scoped_run
        .as_ref()
        .ok_or("first run")?;
    let passed = second.verification[0].scoped_run.as_ref().ok_or("repeat")?;
    assert_eq!(failed.assertion_state, AssertionState::Failed);
    assert_eq!(passed.assertion_state, AssertionState::Passed);
    assert_ne!(failed.artifacts[0].sha256, passed.artifacts[0].sha256);
    assert_eq!(failed.artifacts[1], passed.artifacts[1]);
    assert_eq!(failed.invocation_fingerprint, passed.invocation_fingerprint);
    assert_ne!(failed.operation_id, passed.operation_id);
    assert!(failed.target_unchanged_during_run && passed.target_unchanged_during_run);
    assert!(failed.ended_at_unix_micros < passed.started_at_unix_micros);
    for (receipt, code, stdout) in [
        (&f.failed, 1, "actual=3; expected=7\n"),
        (&f.passed, 0, "actual=7; expected=7\n"),
    ] {
        let read = f
            .ops
            .read_explanation_source(f.project, f.work, receipt.source_id)?;
        assert_eq!(read["observation"]["detail"]["exit_code"], code);
        assert_eq!(read["observation"]["detail"]["stdout"]["body"], stdout);
        assert!(
            matches!(basis.sources.iter().find(|s|s.source.id==receipt.source_id).map(|s| &s.source.payload),Some(SourcePayload::CommandExecution{outcome,..}) if outcome.exit_code==Some(code))
        );
    }
    let restarted = volicord_operations::LocalOperations::new(f.ops.layout().clone());
    assert_eq!(basis, restarted.canonical_basis(f.project)?);
    let path = f.temp.path().join("bundle.json");
    f.ops.export_bundle(f.project, &path)?;
    let imported = volicord_operations::LocalOperations::new(
        volicord_operations::RuntimeLayout::new(f.temp.path().join("imported"))?,
    );
    imported.import_bundle(&path)?;
    assert_eq!(
        imported.canonical_basis(f.project)?.checkpoint_history,
        basis.checkpoint_history
    );
    let mut old: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
    old["format_version"] = serde_json::json!(BUNDLE_FORMAT_VERSION - 1);
    let bad = f.temp.path().join("old.json");
    fs::write(&bad, serde_json::to_vec(&old)?)?;
    assert!(imported.import_bundle(&bad).is_err());
    // Optional retained local evidence is never a claim of ordinary-user capture.
    if let Some(root) = std::env::var_os("VOLICORD_SCOPED_CHECK_CAPTURE") {
        let root = std::path::PathBuf::from(root);
        assert!(root.is_absolute());
        fs::create_dir(&root)?;
        fs::write(root.join("classification.txt"),"Task-owned controlled execution with actual Python assertions. Explicit local authorization; trusted SessionStart capture Blocked (absent). Ordinary-user capture, human review and acceptance not observed.\n")?;
        fs::write(
            root.join("original-checkpoints.txt"),
            format!("{:#?}", basis.checkpoint_history),
        )?;
        fs::write(
            root.join("verification-report.json"),
            serde_json::to_vec_pretty(&f.ops.read_work_verification(f.project, f.work)?)?,
        )?;
        for (name, receipt) in [("failed", &f.failed), ("passed", &f.passed)] {
            fs::write(
                root.join(format!("original-{name}-source.json")),
                serde_json::to_vec_pretty(&f.ops.read_explanation_source(
                    f.project,
                    f.work,
                    receipt.source_id,
                )?)?,
            )?;
        }
    }
    Ok(())
}
#[test]
fn attachment_rejects_wrong_scope_missing_bytes_and_forged_assertion_without_changing_history(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let before = f.ops.canonical_basis(f.project)?;
    let mut wrong = recording(f.other, f.passed.source_id, true);
    assert!(f.ops.record_work_check(f.project, wrong.clone()).is_err());
    wrong.work_item_id = f.work;
    wrong.assertion_evidence.as_mut().ok_or("evidence")?.sha256 = "0".repeat(64);
    assert!(f.ops.record_work_check(f.project, wrong).is_err());
    assert!(f
        .ops
        .record_work_check(f.project, recording(f.work, f.failed.source_id, true))
        .is_err());
    let project = f.ops.initialize_project("Foreign", None)?.project.id;
    assert!(f
        .ops
        .record_work_check(project, recording(f.work, f.passed.source_id, true))
        .is_err());
    f.ops
        .delete_work_source_observation(f.project, f.work, f.passed.source_id)?;
    assert!(f
        .ops
        .record_work_check(f.project, recording(f.work, f.passed.source_id, true))
        .is_err());
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn explicit_cli_capture_and_attachment_preserve_independent_receipts() -> Result<(), Box<dyn Error>>
{
    let f = fixture()?;
    let declaration_path = f.temp.path().join("declaration.json");
    fs::write(&declaration_path, serde_json::to_vec(&declaration())?)?;
    let command = command(&f.ops, f.project, f.work)?;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args([
            "--runtime",
            f.ops.layout().root().to_str().ok_or("runtime")?,
            "--project",
            &f.project.to_string(),
            "--json",
            "work",
            "source",
            "run",
            "--work",
            &f.work.to_string(),
            "--analysis",
            &command.analysis_snapshot_id.to_string(),
            "--retain-seconds",
            "3600",
            "--timeout-seconds",
            "5",
            "--label",
            "explicit value check",
            "--check-input",
            declaration_path.to_str().ok_or("input")?,
            "--",
            "python3",
            "-B",
            "check.py",
        ])
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let source = serde_json::from_value(receipt["observation"]["source_id"].clone())?;
    let request = recording(f.work, source, true);
    let input = f.temp.path().join("recording.json");
    fs::write(&input, serde_json::to_vec(&request)?)?;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args([
            "--runtime",
            f.ops.layout().root().to_str().ok_or("runtime")?,
            "--project",
            &f.project.to_string(),
            "--json",
            "work",
            "check",
            "--input",
            input.to_str().ok_or("input")?,
        ])
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let cp = f
        .ops
        .canonical_basis(f.project)?
        .latest_checkpoint
        .ok_or("checkpoint")?;
    assert_eq!(
        receipt["receipt"]["checkpoint_id"],
        serde_json::json!(cp.id.to_string())
    );
    assert_eq!(cp.verification[0].source_id, Some(source));
    Ok(())
}

#[test]
fn comparison_requires_explicit_identity_coverage_environment_and_measured_target_relation(
) -> Result<(), Box<dyn Error>> {
    use volicord_projections::{
        ComparisonLimit, EvidenceDetail, ReportedCheckState, RetestComparison,
    };
    let f = fixture()?;
    let original = f.ops.read_work_verification(f.project, f.work)?;
    assert_eq!(original.checks.len(), 2);
    assert_eq!(
        original.checks[0].reported_state,
        ReportedCheckState::Failed
    );
    assert_eq!(
        original.checks[0].retest_comparison,
        RetestComparison::Unresolved
    );
    assert!(matches!(
        original.checks[1].retest_comparison,
        RetestComparison::Comparable {
            target_relation: CheckTargetRelation::ChangedArtifacts,
            earlier_assertion: AssertionState::Failed,
            later_assertion: AssertionState::Passed,
            ..
        }
    ));
    assert!(f
        .ops
        .read_work_verification(f.project, f.other)?
        .checks
        .is_empty());
    let before = f.ops.canonical_basis(f.project)?;
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    for variant in [
        "unlinked",
        "identity",
        "purpose",
        "coverage",
        "environment",
        "unknown_environment",
        "relation",
        "unknown_assertion",
    ] {
        let mut declared = declaration();
        match variant {
            "identity" => declared.key = "other.check".into(),
            "purpose" => declared.purpose = "Other unrelated purpose".into(),
            "coverage" => declared.coverage = None,
            "environment" => declared.environment = Some("Other environment".into()),
            "unknown_environment" => declared.environment = None,
            _ => {}
        }
        let source = f
            .ops
            .run_work_check(command(&f.ops, f.project, f.work)?, declared)?
            .source_id;
        let mut request = recording(f.work, source, true);
        if variant != "unlinked" {
            request.retest = Some(CheckRetestBasis {
                checkpoint_id: f.failed.checkpoint_id,
                checkpoint_revision: 1,
                verification_index: 0,
                source_id: f.failed.source_id,
                target_relation: if variant == "relation" {
                    CheckTargetRelation::SameArtifacts
                } else {
                    CheckTargetRelation::ChangedArtifacts
                },
                rationale: "Explicit comparison request; not an assertion of equivalence".into(),
            });
        }
        if variant == "unknown_assertion" {
            request.assertion_state = AssertionState::NotObserved;
            request.assertion_evidence = None;
        }
        let receipt = f.ops.record_work_check(f.project, request)?;
        let report = f.ops.read_work_verification(f.project, f.work)?;
        let check = report
            .checks
            .iter()
            .find(|c| c.original.checkpoint_id == receipt.checkpoint_id.to_string())
            .ok_or("check")?;
        let expected = match variant {
            "unlinked" => RetestComparison::Unresolved,
            _ => RetestComparison::NotComparable {
                previous: original.checks[0].original.clone(),
                reason: match variant {
                    "identity" | "purpose" => ComparisonLimit::CheckIdentityOrPurposeChanged,
                    "coverage" => ComparisonLimit::CoverageMissingOrDifferent,
                    "environment" | "unknown_environment" => {
                        ComparisonLimit::EnvironmentMissingOrDifferent
                    }
                    "relation" => ComparisonLimit::TargetRelationContradicted,
                    _ => ComparisonLimit::AssertionNotObserved,
                },
            },
        };
        assert_eq!(check.retest_comparison, expected, "{variant}");
        if variant == "unknown_assertion" {
            assert_eq!(check.assertion_result, EvidenceDetail::NotObserved);
            assert_eq!(check.reported_state, ReportedCheckState::Partial);
        }
        assert_eq!(report.checks[0], original.checks[0]);
        assert_eq!(
            report
                .executions
                .iter()
                .find(|e| e.source_id == f.failed.source_id.to_string())
                .ok_or("failure")?
                .exit_code,
            Some(1)
        );
    }
    // Canonical scoped facts and comparison survive disposal; no output revival.
    let report = f.ops.read_work_verification(f.project, f.work)?;
    f.ops
        .delete_work_source_observation(f.project, f.work, f.failed.source_id)?;
    f.ops
        .delete_work_source_observation(f.project, f.work, f.passed.source_id)?;
    let disposed = f.ops.read_work_verification(f.project, f.work)?;
    assert_eq!(report.checks, disposed.checks);
    assert!(matches!(
        disposed.checks[1].scoped_run,
        EvidenceDetail::Observed(_)
    ));
    assert_eq!(
        disposed.executions[0].observation,
        EvidenceDetail::Unavailable
    );
    assert_eq!(
        disposed.executions[1].observation,
        EvidenceDetail::Unavailable
    );
    Ok(())
}
