//! Authored contract controls, not a generated repository explanation or prose review.
use serde_json::Value;
use std::{error::Error, fs};
use volicord_context::{ProjectId, SourceId};
use volicord_operations::*;
use volicord_projections::*;
use volicord_repository_intelligence::AnalysisSnapshotId;

struct Fixture {
    _temp: tempfile::TempDir,
    root: std::path::PathBuf,
    ops: LocalOperations,
    project: ProjectId,
    analysis: AnalysisSnapshotId,
}
fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repository");
    fs::create_dir(&root)?;
    fs::write(
        root.join("main.py"),
        "def greet(name):\n    return '안녕 ' + name\n",
    )?;
    fs::write(root.join("secret.py"), "secret = True\n")?;
    fs::write(root.join(".gitignore"), "ignored.py\n")?;
    fs::write(root.join("ignored.py"), "private = True\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let project = ops
        .initialize_project("source reading", Some(&root))?
        .project
        .id;
    let analysis = ops
        .analyze(project, vec!["secret.py".into()])?
        .value
        .ok_or("analysis missing")?
        .analysis
        .identity;
    Ok(Fixture {
        _temp: temp,
        root,
        ops,
        project,
        analysis,
    })
}
fn observe(
    f: &Fixture,
    path: &str,
    range: Option<FileByteRange>,
) -> Result<Value, volicord_operations::Error> {
    f.ops
        .observe_repository_file(RepositoryFileObservationRequest {
            project_id: f.project,
            analysis_snapshot_id: f.analysis,
            path: path.into(),
            byte_range: range,
            retain_seconds: 3600,
        })
}
fn source(receipt: &Value) -> Result<SourceId, Box<dyn Error>> {
    Ok(serde_json::from_value(receipt["source_id"].clone())?)
}
fn response(f: &Fixture, source: SourceId) -> Result<ExplanationRealization, Box<dyn Error>> {
    let plan =
        f.ops
            .prepare_explanation(f.project, ExplanationSubject::Repository(f.analysis), "en")?;
    Ok(f.ops.compose_repository_explanation(
        f.project,
        f.analysis,
        RepositoryExplanationDraft {
            plan_fingerprint: plan.fingerprint,
            language: "en".into(),
            generator: ExplanationGenerator {
                host: "authored-contract-control".into(),
                session: "fixture".into(),
                agent: None,
                model: None,
            },
            paragraphs: vec![RepositoryReaderUnit {
                question: ExplanationQuestion::Observation,
                text: "Authored control: greet builds a greeting from the supplied name.".into(),
                evidence_keys: vec!["repository_basis".into()],
                sources: vec![RepositoryClaimRange {
                    source_id: source,
                    start_byte: 0,
                    end_byte: 15,
                    claim: None,
                }],
            }],
        },
    )?)
}
#[test]
fn work_free_retention_reading_and_failed_write_atomicity() -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let first =
        f.ops
            .prepare_explanation(f.project, ExplanationSubject::Repository(f.analysis), "en")?;
    assert_eq!(
        first,
        f.ops
            .prepare_explanation(f.project, ExplanationSubject::Repository(f.analysis), "en")?
    );
    let id = source(&observe(&f, "main.py", None)?)?;
    let r = response(&f, id)?;
    assert_eq!(r.paragraphs[0].citations[0].start_line, 1);
    let retained = f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        r.clone(),
    )?;
    let before = f.ops.canonical_basis(f.project)?;
    let reading = f.ops.read_repository_explanation(f.project, "en")?;
    assert_eq!(reading.explanation_state, ExplanationState::Current);
    assert_eq!(reading.prose[0].text, r.paragraphs[0].text);
    assert_eq!(reading.source_excerpts[0].work_revision, None);
    assert_eq!(reading.source_excerpts[0].text, "def greet(name)");
    for mutation in 0..8 {
        let mut bad = r.clone();
        let c = &mut bad.paragraphs[0].citations[0];
        match mutation {
            0 => c.claim_sha256 = "sha256:bad".into(),
            1 => c.range_sha256 = "sha256:bad".into(),
            2 => c.analysis_snapshot_id = "ab".repeat(32),
            3 => c.repository_snapshot_id = "ab".repeat(32),
            4 => c.end_byte = usize::MAX,
            5 => c.start_line = 7,
            6 => bad.format_version = EXPLANATION_VERSION - 1,
            _ => bad.paragraphs[0].evidence_keys = vec!["unprepared".into()],
        }
        assert!(f
            .ops
            .record_explanation(
                f.project,
                ExplanationSubject::Repository(f.analysis),
                "en",
                bad
            )
            .is_err());
        assert_eq!(
            f.ops
                .read_repository_explanation(f.project, "en")?
                .provenance
                .as_ref()
                .map(|p| &p.fingerprint),
            Some(&retained.realization.plan_fingerprint)
        );
    }
    let after = f.ops.canonical_basis(f.project)?;
    assert_eq!(before, after);
    assert!(after.context_items.is_empty());
    assert!(after.latest_checkpoint.is_none());
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "ko")?
            .explanation_state,
        ExplanationState::Missing
    );
    // Copy deletion immediately withholds every generated field including headings.
    f.ops
        .delete_repository_source_observation(f.project, f.analysis, id)?;
    let unavailable = f.ops.read_repository_explanation(f.project, "en")?;
    assert_ne!(unavailable.explanation_state, ExplanationState::Current);
    assert!(!unavailable
        .prose
        .iter()
        .any(|p| p.role == AnswerRole::GeneratedInterpretation));
    assert!(f
        .ops
        .read_repository_explanation_source(f.project, f.analysis, id)
        .is_err());
    f.ops
        .delete_explanations(f.project, ExplanationSubject::Repository(f.analysis))?;
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "en")?
            .explanation_state,
        ExplanationState::Deleted
    );
    Ok(())
}
#[test]
fn repository_authority_rejects_foreign_snapshots_paths_and_work_borrowing(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = source(&observe(&f, "main.py", None)?)?;
    for path in [
        "secret.py",
        "ignored.py",
        "../repository/main.py",
        "/etc/passwd",
        "missing.py",
    ] {
        assert!(observe(&f, path, None).is_err(), "accepted {path}");
    }
    let other_root = f._temp.path().join("other");
    fs::create_dir(&other_root)?;
    fs::write(other_root.join("main.py"), "def value(): return 2\n")?;
    let other = f
        .ops
        .initialize_project("other", Some(&other_root))?
        .project
        .id;
    assert!(f
        .ops
        .prepare_explanation(other, ExplanationSubject::Repository(f.analysis), "en")
        .is_err());
    assert!(f
        .ops
        .read_repository_explanation_source(other, f.analysis, id)
        .is_err());
    let other_analysis = f
        .ops
        .analyze(other, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    assert!(f
        .ops
        .read_repository_explanation_source(other, other_analysis, id)
        .is_err());
    let work = f
        .ops
        .record_current_host_user_context(
            f.project,
            "test".into(),
            "control".into(),
            "investigate".into(),
            volicord_context::ContextItemRole::Goal,
            "investigate".into(),
        )?
        .context_item_id;
    assert!(f.ops.read_explanation_source(f.project, work, id).is_err());
    let work_source = f.ops.observe_work_file(FileObservationRequest {
        project_id: f.project,
        work_item_id: work,
        analysis_snapshot_id: f.analysis,
        path: "main.py".into(),
        byte_range: None,
        before_source_id: None,
        retain_seconds: 3600,
    })?;
    assert!(f
        .ops
        .read_repository_explanation_source(f.project, f.analysis, work_source.source_id)
        .is_err());
    fs::write(
        f.root.join("main.py"),
        "def greet(name):\n    return 'changed'\n",
    )?;
    assert!(observe(&f, "main.py", None).is_err());
    assert!(f
        .ops
        .prepare_explanation(f.project, ExplanationSubject::Repository(f.analysis), "en")
        .is_err());
    // Historical bytes remain exact, without a current-file fallback.
    assert!(f
        .ops
        .read_repository_explanation_source(f.project, f.analysis, id)?["observation"]["detail"]
        ["body"]
        .as_str()
        .ok_or("body")?
        .contains("안녕"));
    Ok(())
}
#[test]
fn bounded_ranges_full_file_changes_and_symlinks_are_enforced() -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    for (start, end) in [(0, 0), (8, 3), (0, 9999), (29, 30)] {
        // 28 splits the Korean UTF-8 sequence.
        assert!(
            observe(
                &f,
                "main.py",
                Some(FileByteRange {
                    start_byte: start,
                    end_byte: end
                })
            )
            .is_err(),
            "{start}..{end}"
        );
    }
    let id = source(&observe(
        &f,
        "main.py",
        Some(FileByteRange {
            start_byte: 0,
            end_byte: 15,
        }),
    )?)?;
    let r = response(&f, id)?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        r,
    )?;
    fs::write(
        f.root.join("main.py"),
        "def greet(name):\n    return 'later'\n",
    )?;
    assert!(!f
        .ops
        .read_repository_explanation(f.project, "en")?
        .prose
        .iter()
        .any(|p| p.role == AnswerRole::GeneratedInterpretation));
    assert_eq!(
        f.ops
            .read_repository_explanation_source(f.project, f.analysis, id)?["observation"]
            ["detail"]["body"],
        "def greet(name)"
    );
    #[cfg(unix)]
    {
        fs::remove_file(f.root.join("main.py"))?;
        std::os::unix::fs::symlink("secret.py", f.root.join("main.py"))?;
        assert!(observe(&f, "main.py", None).is_err());
    }
    Ok(())
}

#[test]
fn expired_corrupt_and_unsupported_copies_and_outputs_never_supply_prose(
) -> Result<(), Box<dyn Error>> {
    for variant in [
        "expired",
        "corrupt_source",
        "unsupported_source",
        "unsupported_output",
        "corrupt_output",
    ] {
        let f = fixture()?;
        let id = source(&observe(&f, "main.py", None)?)?;
        let r = response(&f, id)?;
        f.ops.record_explanation(
            f.project,
            ExplanationSubject::Repository(f.analysis),
            "en",
            r,
        )?;
        let connection = rusqlite::Connection::open(f.ops.layout().privacy_store())?;
        let records: Vec<(Vec<u8>, String)> = connection
            .prepare("SELECT id, record_json FROM managed_derived")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let target = if variant.ends_with("output") {
            "cached_summary"
        } else {
            "source_observation"
        };
        let (record_id, raw) = records
            .iter()
            .find(|(_, raw)| serde_json::from_str::<Value>(raw).is_ok_and(|v| v["kind"] == target))
            .ok_or("record")?;
        let mut record: Value = serde_json::from_str(raw)?;
        if variant == "expired" {
            record["retained_until"] = serde_json::json!(1);
        } else if variant.starts_with("corrupt") {
            record["content"] = serde_json::json!("broken JSON");
        } else {
            let mut body: Value =
                serde_json::from_str(record["content"].as_str().ok_or("content")?)?;
            if variant.ends_with("output") {
                body["realization"]["format_version"] = serde_json::json!(EXPLANATION_VERSION - 1);
            } else {
                body["format_version"] = serde_json::json!(0);
            }
            record["content"] = serde_json::json!(body.to_string());
        }
        connection.execute(
            "UPDATE managed_derived SET record_json=?1 WHERE id=?2",
            rusqlite::params![record.to_string(), record_id],
        )?;
        drop(connection);
        let reading = f.ops.read_repository_explanation(f.project, "en")?;
        assert_ne!(
            reading.explanation_state,
            ExplanationState::Current,
            "{variant}"
        );
        assert!(
            !reading
                .prose
                .iter()
                .any(|p| p.role == AnswerRole::GeneratedInterpretation),
            "{variant}"
        );
        assert!(reading.source_excerpts.is_empty());
        if variant == "unsupported_output" {
            assert_eq!(reading.explanation_state, ExplanationState::Unsupported);
        }
        if variant == "corrupt_output" {
            assert_eq!(reading.explanation_state, ExplanationState::Corrupt);
        }
        if !variant.ends_with("output") {
            assert!(f
                .ops
                .read_repository_explanation_source(f.project, f.analysis, id)
                .is_err());
        }
    }
    Ok(())
}
