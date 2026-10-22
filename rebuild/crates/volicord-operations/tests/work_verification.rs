//! Authored histories and actual disposable command runs; no genuine task or
//! human approval. Expectations come from inputs before any Product read.
#[path = "support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use serde_json::{json, Value};
use std::{error::Error, fs, process::Command, time::Duration};
use volicord_context::ContextItemRole;
use volicord_operations::{LocalOperations, RuntimeLayout, WorkCommandRequest};
use volicord_projections::*;

#[test]
fn original_reports_preserve_failure_unrelated_pass_skip_and_historical_coverage(
) -> Result<(), Box<dyn Error>> {
    let mut input: Value = serde_json::from_str(reading_fixture::SCENARIO)?;
    input["prior_checkpoint_count"] = json!(0);
    input["later_checkpoint_count"] = json!(0);
    input["works"][0]["checkpoints"] = json!([
        {"key":"failed","work":"Paused","verification":"Failed","outcome":"compile widget: failed; a free-text report, not an authoritative test ID","state_change":"Investigated widget failure","paths":[]},
        {"key":"independent","work":"InProgress","verification":"Passed","outcome":"Unrelated linter completed","state_change":null,"paths":[]},
        {"key":"skip","work":"Paused","verification":"NotRun","state_change":"Configuration changed after the earlier checks","paths":["python/worker.py"]}
    ]);
    let f = reading_fixture::fixture_scenario(input)?;
    let canonical = f.operations.canonical_basis(f.project)?;
    let work = f.goals["older"];
    let report = f.operations.read_work_verification(f.project, work)?;
    assert_eq!(report.checks.len(), 3);
    assert_eq!(report.report_count(ReportedCheckState::NotRun), 1);
    assert!(report
        .report_summary(FixedLocale::English)
        .contains("1 not run"));
    let empty = WorkVerificationReport::default();
    assert_eq!(empty.report_count(ReportedCheckState::Passed), 0);
    assert!(empty
        .report_summary(FixedLocale::English)
        .contains("none recorded"));
    assert!(!empty
        .report_summary(FixedLocale::English)
        .contains("0 tests"));
    let empty_json = serde_json::to_value(&empty)?;
    assert_eq!(empty_json["report_counts"]["total"], 0);
    assert!(empty_json.get("test_count").is_none());
    assert_eq!(
        report
            .checks
            .iter()
            .map(|c| c.reported_state)
            .collect::<Vec<_>>(),
        [
            ReportedCheckState::Failed,
            ReportedCheckState::Passed,
            ReportedCheckState::NotRun
        ]
    );
    assert_eq!(
        report.checks[0].original.checkpoint_id,
        f.checkpoints["failed"].to_string()
    );
    assert_eq!(
        report.checks[0].original_outcome.as_deref(),
        Some("compile widget: failed; a free-text report, not an authoritative test ID")
    );
    assert_eq!(
        report.checks[1].applicability,
        VerificationApplicability::HistoricalAfterRecordedChange
    );
    for check in &report.checks {
        assert_eq!(check.intended_check, EvidenceDetail::NotObserved);
        assert_eq!(check.assertion_result, EvidenceDetail::NotObserved);
        assert_eq!(check.retest_comparison, RetestComparison::Unresolved);
        assert_eq!(check.original.verification_index, 0);
    }
    assert_eq!(report.executions.len(), 2);
    assert!(report
        .executions
        .iter()
        .all(|e| e.body_state == CommandBodyState::NotRetained
            && e.observation == EvidenceDetail::NotObserved));
    let projection = f
        .operations
        .project_projection_selected(f.project, WorkSelector::ExactWork(work))?;
    let selected = projection.selected_work.ok_or("work")?;
    assert_eq!(
        selected
            .reading
            .answers
            .acceptance
            .as_ref()
            .ok_or("acceptance")?
            .user_acceptance
            .state,
        volicord_context::UserAcceptanceState::NotRequested
    );
    let answers = work_answers(&selected, "en", FixedLocale::English);
    assert_eq!(
        answers
            .facts
            .iter()
            .filter(|v| v.question == "VerificationState")
            .count(),
        3
    );
    assert_eq!(answers.cautions.len(), 5); // three reports + two unavailable bodies
    assert_eq!(canonical, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn genuine_process_observations_are_independent_of_articles_and_checkpoint_assertions(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    fs::create_dir(&root)?;
    fs::write(root.join("main.py"), "def value():\n    return 1\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let project = ops
        .initialize_project("authored execution control", Some(&root))?
        .project
        .id;
    let work = ops
        .record_current_host_user_context(
            project,
            "test".into(),
            "authored-control".into(),
            "Inspect retained runs".into(),
            ContextItemRole::Goal,
            "Inspect retained runs".into(),
        )?
        .context_item_id;
    let other = ops
        .record_current_host_user_context(
            project,
            "test".into(),
            "other".into(),
            "Other scope".into(),
            ContextItemRole::Goal,
            "Other scope".into(),
        )?
        .context_item_id;
    let analysis = ops
        .analyze(project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let mut ids = Vec::new();
    for (label, script) in [
        (
            "widget assertion",
            "raise AssertionError('widget remains broken')",
        ),
        (
            "unrelated arithmetic check",
            "assert 2 + 2 == 4; print('arithmetic check passed')",
        ),
        ("no assertions in this command", "print('status only')"),
    ] {
        ids.push(
            ops.run_work_command(WorkCommandRequest {
                project_id: project,
                work_item_id: work,
                analysis_snapshot_id: analysis,
                command_label: label.into(),
                program: "python3".into(),
                arguments: vec!["-B".into(), "-c".into(), script.into()],
                timeout: Duration::from_secs(3),
                retain_seconds: 3600,
            })?
            .source_id,
        );
    }
    let before = ops.privacy_status(project)?;
    let canonical = ops.canonical_basis(project)?;
    let report = ops.read_work_verification(project, work)?;
    if let Some(root) = std::env::var_os("VOLICORD_VERIFICATION_CONTROL_CAPTURE") {
        let root = std::path::PathBuf::from(root);
        if !root.is_absolute() {
            return Err("control capture must be absolute".into());
        }
        fs::create_dir(&root)?;
        fs::write(root.join("origin.txt"),"Authored technical control with real disposable process executions. No genuine user Work, model generation, semantic approval or human acceptance. Test source expectations precede all Product reads.\n")?;
        fs::write(
            root.join("verification-report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        for (index, id) in ids.iter().enumerate() {
            fs::write(
                root.join(format!("original-command-{index}.json")),
                serde_json::to_vec_pretty(&ops.read_explanation_source(project, work, *id)?)?,
            )?;
        }
    }
    assert!(report.checks.is_empty());
    assert_eq!(
        report
            .executions
            .iter()
            .map(|e| e.exit_code)
            .collect::<Vec<_>>(),
        [Some(1), Some(0), Some(0)]
    );
    for (e, id) in report.executions.iter().zip(&ids) {
        assert_eq!(e.source_id, id.to_string());
        assert_eq!(e.assertion_result, EvidenceDetail::NotObserved);
        assert_eq!(e.environment, EvidenceDetail::NotObserved);
        assert!(e.checkpoint_reports.is_empty());
        let EvidenceDetail::Observed(o) = &e.observation else {
            return Err("missing real observation".into());
        };
        assert_eq!(o.analysis_snapshot_id, analysis.to_string());
        assert!(o.started_at_unix_micros <= o.ended_at_unix_micros);
        assert!(!o.operation_id.is_empty());
        assert_eq!(
            o.target_state,
            ExecutionTargetState::SelectedAnalysisOnlyNotRechecked
        );
        assert_eq!(o.stdout.capture_completeness, CaptureCompleteness::Complete);
    }
    assert!(ops
        .read_work_verification(project, other)?
        .executions
        .is_empty());
    let binary = env!("CARGO_BIN_EXE_volicord");
    for json_mode in [false, true] {
        let mut command = Command::new(binary);
        command.args([
            "--runtime",
            ops.layout().root().to_str().ok_or("runtime")?,
            "--project",
            &project.to_string(),
        ]);
        if json_mode {
            command.arg("--json");
        }
        let output = command
            .args(["work", "verification", "--work", &work.to_string()])
            .output()?;
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        if json_mode {
            let read: Value = serde_json::from_slice(&output.stdout)?;
            assert_eq!(read["verification_report"], serde_json::to_value(&report)?);
        } else {
            assert!(String::from_utf8(output.stdout)?
                .contains("process exit is separate from assertions"));
        }
    }
    assert_eq!(canonical, ops.canonical_basis(project)?);
    assert_eq!(
        before.managed_derived,
        ops.privacy_status(project)?.managed_derived
    );
    // Adversarial copies exercise the independent report's own public consumer,
    // not merely the lower-level Source reader. Restore original bytes each time.
    let projection = ops.project_projection_selected(project, WorkSelector::ExactWork(work))?;
    let proofs = work_verification_publication_basis(&projection);
    let connection = rusqlite::Connection::open(ops.layout().privacy_store())?;
    let purpose = format!("work-source:{work}:{}", ids[2]);
    let (record_id, raw): (Vec<u8>, String) = connection.query_row(
        "SELECT id,record_json FROM managed_derived WHERE json_extract(record_json,'$.purpose')=?1",
        [purpose],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    for variant in [
        "expired",
        "unsupported",
        "corrupt",
        "wrong_work",
        "missing_snapshot",
        "operation_identity",
        "extended_retention",
        "foreign_integrity",
        "rehashed_stream",
    ] {
        let mut record: Value = serde_json::from_str(&raw)?;
        if variant == "expired" {
            record["retained_until"] = json!(1);
        } else {
            let mut body: Value = serde_json::from_str(record["content"].as_str().ok_or("body")?)?;
            match variant {
                "unsupported" => body["format_version"] = json!(999),
                "wrong_work" => body["work_item_id"] = serde_json::to_value(other)?,
                "missing_snapshot" => body["repository_snapshot_id"] = json!("missing-snapshot"),
                "operation_identity" => body["detail"]["operation_id"] = json!("00".repeat(16)),
                "extended_retention" => {
                    let later = body["retained_until_unix_micros"]
                        .as_i64()
                        .ok_or("expiry")?
                        + 1000;
                    body["retained_until_unix_micros"] = json!(later);
                    record["retained_until"] = json!(later);
                }
                "foreign_integrity" => body["integrity_source_id"] = serde_json::to_value(ids[1])?,
                "rehashed_stream" => {
                    body["detail"]["stdout"]["body"] = json!("forged data\n");
                    body["detail"]["stdout"]["observed_bytes"] = json!(12);
                    body["detail"]["stdout"]["omitted_bytes"] = json!(0);
                    body["detail"]["stdout"]["sha256"] =
                        json!(explanation_text_hash("forged data\n"));
                }
                _ => body["detail"]["stdout"]["body"] = json!("forged data\n"),
            }
            record["content"] = json!(body.to_string());
        }
        connection.execute(
            "UPDATE managed_derived SET record_json=?1 WHERE id=?2",
            rusqlite::params![record.to_string(), &record_id],
        )?;
        let denied = ops.read_work_verification(project, work)?;
        let run = &denied.executions[2];
        assert_eq!(run.observation, EvidenceDetail::Unavailable, "{variant}");
        assert!(run.observation_fingerprint.is_none(), "{variant}");
        assert_eq!(run.exit_code, Some(0)); // immutable historical process outcome
        let path = temp.path().join(format!("denied-{variant}.html"));
        assert!(
            ops.publish_viewer_snapshot(
                "<p>old execution metadata</p>",
                &path,
                project,
                &canonical_read_fingerprint(&canonical),
                &[],
                &proofs
            )
            .is_err(),
            "{variant}"
        );
        assert!(!path.exists());
        connection.execute(
            "UPDATE managed_derived SET record_json=?1 WHERE id=?2",
            rusqlite::params![&raw, &record_id],
        )?;
    }
    drop(connection);
    ops.delete_work_source_observation(project, work, ids[0])?;
    let deleted = ops.read_work_verification(project, work)?;
    assert_eq!(deleted.executions[0].body_state, CommandBodyState::Deleted);
    assert_eq!(
        deleted.executions[0].observation,
        EvidenceDetail::Unavailable
    );
    assert_eq!(deleted.executions[0].exit_code, Some(1));
    assert_eq!(deleted.executions[1].exit_code, Some(0));
    assert!(deleted.checks.is_empty()); // no automatic adoption or repair
    let read = ops.read_explanation_source(project, work, ids[1])?;
    let anchor: volicord_context::SourceId =
        serde_json::from_value(read["observation"]["integrity_source_id"].clone())?;
    let auth = canonical
        .context_items
        .iter()
        .find(|c| c.id == work)
        .ok_or("goal")?
        .source_basis[0];
    ops.forget_record(
        project,
        volicord_context::CanonicalRecordId::Source(anchor),
        auth,
    )?;
    let forgotten = ops.read_work_verification(project, work)?;
    let run = forgotten
        .executions
        .iter()
        .find(|r| r.source_id == ids[1].to_string())
        .ok_or("historical command")?;
    assert_eq!(run.exit_code, Some(0));
    assert_eq!(run.observation, EvidenceDetail::Unavailable);
    assert!(ops.read_explanation_source(project, work, ids[1]).is_err());
    Ok(())
}
