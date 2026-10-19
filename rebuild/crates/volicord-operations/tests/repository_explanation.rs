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
    let value = receipt["source_id"].as_str().ok_or("Source ID")?;
    let mut bytes = [0u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)?;
    }
    Ok(SourceId::from_bytes(bytes))
}
#[test]
fn cli_preparation_routes_work_free_repository_evidence_without_granting_body_access(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let call = |project: ProjectId, args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_volicord"))
            .args(["--runtime"])
            .arg(f.ops.layout().root())
            .args(["--project", &project.to_string(), "--json"])
            .args(args)
            .output()
    };
    let analysis = f.analysis.to_string();
    let before = f.ops.canonical_basis(f.project)?;
    let prepared = call(
        f.project,
        &["repository", "explain", "prepare", "--analysis", &analysis],
    )?;
    assert!(prepared.status.success(), "{:?}", prepared);
    let prepared: Value = serde_json::from_slice(&prepared.stdout)?;
    let inventory = &prepared["source_inventory"];
    assert_eq!(inventory["scope"], "exact_repository_analysis");
    assert_eq!(inventory["analysis_snapshot_id"], analysis);
    assert_eq!(inventory["body_authority"], "not_granted");
    assert!(!prepared.to_string().contains("안녕"));
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    assert!(before.context_items.is_empty());
    // Follow the actual returned entry points; preparation is not observation/read.
    let entry = |key: &str| -> Result<Vec<&str>, Box<dyn Error>> {
        Ok(inventory[key]
            .as_str()
            .ok_or("entry point")?
            .split_whitespace()
            .collect())
    };
    let mut args = entry("analysis_entrypoint")?;
    args.extend(["--analysis", &analysis]);
    let evidence = call(f.project, &args)?;
    assert!(evidence.status.success());
    assert!(!String::from_utf8(evidence.stdout)?.contains("안녕"));
    let mut args = entry("observation_entrypoint")?;
    args.extend([
        "--analysis",
        &analysis,
        "--path",
        "main.py",
        "--retain-seconds",
        "3600",
    ]);
    let observed = call(f.project, &args)?;
    assert!(observed.status.success());
    let observed: Value = serde_json::from_slice(&observed.stdout)?;
    let source = observed["observation"]["source_id"]
        .as_str()
        .ok_or("Source")?;
    let mut args = entry("read_entrypoint")?;
    args.extend(["--analysis", &analysis, "--source", source]);
    let read = call(f.project, &args)?;
    assert!(read.status.success());
    let read: Value = serde_json::from_slice(&read.stdout)?;
    assert_eq!(
        read["observation"]["detail"]["body"],
        fs::read_to_string(f.root.join("main.py"))?
    );
    let other_root = f._temp.path().join("other");
    fs::create_dir(&other_root)?;
    let other = f
        .ops
        .initialize_project("other", Some(&other_root))?
        .project
        .id;
    for (project, selected) in [(other, analysis.as_str()), (f.project, &"ff".repeat(32))] {
        let output = call(
            project,
            &["repository", "explain", "prepare", "--analysis", selected],
        )?;
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    assert!(f.ops.canonical_basis(f.project)?.context_items.is_empty());
    Ok(())
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
    for seconds in [0, 604801] {
        assert!(f
            .ops
            .observe_repository_file(RepositoryFileObservationRequest {
                project_id: f.project,
                analysis_snapshot_id: f.analysis,
                path: "main.py".into(),
                byte_range: None,
                retain_seconds: seconds
            })
            .is_err());
    }
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

#[test]
fn original_project_purpose_and_source_forgetting_stay_work_free() -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let purpose_text = "Keep the recorded project rationale available across clones.";
    let purpose = f.ops.record_current_host_user_context(
        f.project,
        "authored-control".into(),
        "purpose".into(),
        purpose_text.into(),
        volicord_context::ContextItemRole::ProjectPurpose,
        purpose_text.into(),
    )?;
    let id = source(&observe(&f, "main.py", None)?)?;
    let r = response(&f, id)?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        r,
    )?;
    let before = f.ops.canonical_basis(f.project)?;
    let original = before
        .context_items
        .iter()
        .find(|i| i.id == purpose.context_item_id)
        .ok_or("Purpose")?
        .clone();
    assert_eq!(original.statement, purpose_text);
    assert!(before
        .context_items
        .iter()
        .all(|i| i.role != volicord_context::ContextItemRole::Goal));
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    let authorization = f.ops.record_user_source(
        f.project,
        "authored-control".into(),
        "forget".into(),
        "Forget this observed file Source.".into(),
    )?;
    let authorization = source(&serde_json::json!({"source_id":authorization.identity}))?;
    let result = f.ops.forget_record(
        f.project,
        volicord_context::CanonicalRecordId::Source(id),
        authorization,
    )?;
    assert!(
        result.canonical_committed
            && result.managed_derived_cleanup_completed
            && result.residue_verified
    );
    assert!(f
        .ops
        .read_repository_explanation_source(f.project, f.analysis, id)
        .is_err());
    assert_ne!(
        f.ops
            .read_repository_explanation(f.project, "en")?
            .explanation_state,
        ExplanationState::Current
    );
    let after = f.ops.canonical_basis(f.project)?;
    assert_eq!(
        after.context_items.iter().find(|i| i.id == original.id),
        Some(&original)
    );
    assert!(after.latest_checkpoint.is_none());
    Ok(())
}

#[test]
fn missing_analysis_rebinding_and_storage_failure_have_bounded_recovery(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = source(&observe(&f, "main.py", None)?)?;
    let r = response(&f, id)?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        r.clone(),
    )?;
    let path = f
        .ops
        .layout()
        .analysis_project_dir(f.project)
        .join(format!("{}.json", f.analysis));
    let hidden = path.with_extension("hidden");
    fs::rename(&path, &hidden)?;
    let absent = f.ops.read_repository_explanation(f.project, "en")?;
    assert_eq!(absent.explanation_state, ExplanationState::Unavailable);
    assert!(f
        .ops
        .read_repository_explanation_source(f.project, f.analysis, id)
        .is_err());
    fs::rename(&hidden, &path)?;
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "en")?
            .explanation_state,
        ExplanationState::Current
    );
    let before = f.ops.canonical_basis(f.project)?;
    let database = f.ops.layout().privacy_store();
    let original = database.with_extension("original");
    fs::rename(&database, &original)?;
    fs::write(&database, b"Authored unavailable SQLite control")?;
    assert!(observe(&f, "main.py", None).is_err());
    assert!(f
        .ops
        .record_explanation(
            f.project,
            ExplanationSubject::Repository(f.analysis),
            "en",
            r
        )
        .is_err());
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "en")?
            .explanation_state,
        ExplanationState::Unavailable
    );
    assert_eq!(fs::read(&database)?, b"Authored unavailable SQLite control");
    fs::remove_file(&database)?;
    fs::rename(&original, &database)?;
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "en")?
            .explanation_state,
        ExplanationState::Current
    );
    let other = f._temp.path().join("identical-other-repository");
    fs::create_dir(&other)?;
    for name in ["main.py", "secret.py", "ignored.py", ".gitignore"] {
        fs::copy(f.root.join(name), other.join(name))?;
    }
    f.ops.bind_project(f.project, Some(1), &other)?;
    assert!(f
        .ops
        .read_repository_explanation_source(f.project, f.analysis, id)
        .is_err());
    assert_ne!(
        f.ops
            .read_repository_explanation(f.project, "en")?
            .explanation_state,
        ExplanationState::Current
    );
    Ok(())
}

#[test]
fn large_diagnostic_metadata_is_inspectable_without_bloating_the_reader_plan(
) -> Result<(), Box<dyn Error>> {
    let mut f = fixture()?;
    let rust=(0..256).map(|n|format!("fn value_{n}() {{ missing_a_{n}(); missing_b_{n}(); missing_c_{n}(); missing_d_{n}(); missing_e_{n}(); missing_f_{n}(); missing_g_{n}(); missing_h_{n}(); }}\n")).collect::<String>();
    fs::write(f.root.join("diagnostics.rs"), rust)?;
    f.analysis = f
        .ops
        .analyze(f.project, vec!["secret.py".into()])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let full = f
        .ops
        .repository_explanation_evidence(f.project, f.analysis)?;
    let reports = full["analysis_metadata"]["capabilities"]
        .as_array()
        .ok_or("reports")?;
    let repeated_diagnostic_bytes: usize = reports
        .iter()
        .map(|r| r["diagnostics"].to_string().len())
        .sum();
    assert!(
        repeated_diagnostic_bytes > EXPLANATION_RETAINED_BYTE_LIMIT,
        "control needs large repeated diagnostic IDs; got {repeated_diagnostic_bytes}"
    );
    let plan =
        f.ops
            .prepare_explanation(f.project, ExplanationSubject::Repository(f.analysis), "en")?;
    assert!(serde_json::to_vec(&plan)?.len() <= EXPLANATION_PLAN_BYTE_LIMIT);
    assert!(plan.retention_budget.response_byte_capacity >= EXPLANATION_MIN_RESPONSE_BYTE_CAPACITY);
    let compact = plan
        .evidence
        .iter()
        .find(|e| e.key == "repository_basis")
        .ok_or("basis")?
        .content["capabilities"]
        .as_array()
        .ok_or("compact reports")?;
    assert_eq!(compact.len(), reports.len());
    for (summary, original) in compact.iter().zip(reports) {
        assert_eq!(summary["state"], original["state"]);
        assert_eq!(
            summary["diagnostic_count"].as_u64(),
            original["diagnostics"].as_array().map(|d| d.len() as u64)
        );
        assert_eq!(
            summary["diagnostics_sha256"],
            explanation_text_hash(&original["diagnostics"].to_string())
        );
        assert_eq!(
            summary["coverage"]["included"].as_array().map(Vec::len),
            original["coverage"]["included"].as_array().map(Vec::len)
        );
    }
    println!(
        "diagnostic metadata={repeated_diagnostic_bytes} bytes; prepared reader={} bytes",
        serde_json::to_vec(&plan)?.len()
    );
    Ok(())
}

#[test]
fn repository_copy_metadata_tampering_is_rejected_before_disclosure() -> Result<(), Box<dyn Error>>
{
    for key in [
        "kind",
        "repository_worktree",
        "comparison",
        "retention_expiry",
    ] {
        let f = fixture()?;
        let id = source(&observe(&f, "main.py", None)?)?;
        let connection = rusqlite::Connection::open(f.ops.layout().privacy_store())?;
        let (record_id,raw):(Vec<u8>,String)=connection.query_row("SELECT id,record_json FROM managed_derived WHERE json_extract(record_json,'$.kind')='source_observation'",[],|r|Ok((r.get(0)?,r.get(1)?)))?;
        let mut record: Value = serde_json::from_str(&raw)?;
        let mut body: Value = serde_json::from_str(record["content"].as_str().ok_or("content")?)?;
        if key == "retention_expiry" {
            record["retained_until"] = serde_json::json!(
                body["retained_until_unix_micros"]
                    .as_i64()
                    .ok_or("expiry")?
                    + 86_400_000_000i64
            );
        } else {
            body["detail"][key] = serde_json::json!("forged provenance");
        }
        record["content"] = serde_json::json!(body.to_string());
        connection.execute(
            "UPDATE managed_derived SET record_json=?1 WHERE id=?2",
            rusqlite::params![record.to_string(), record_id],
        )?;
        drop(connection);
        assert!(
            f.ops
                .read_repository_explanation_source(f.project, f.analysis, id)
                .is_err(),
            "accepted {key}"
        );
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn denied_file_permissions_do_not_publish_an_observation() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture()?;
    let before = f.ops.canonical_basis(f.project)?;
    let file = f.root.join("main.py");
    let original = fs::metadata(&file)?.permissions();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o0))?;
    let denied = observe(&f, "main.py", None);
    fs::set_permissions(&file, original)?;
    assert!(
        denied.is_err(),
        "the permission control requires a non-root test process"
    );
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    Ok(())
}
