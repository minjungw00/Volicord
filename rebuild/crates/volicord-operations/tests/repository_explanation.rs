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
fn current_repository_budget_measures_distinct_claims_and_editorial_overhead(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = source(&observe(&f, "main.py", None)?)?;
    let plan =
        f.ops
            .prepare_explanation(f.project, ExplanationSubject::Repository(f.analysis), "en")?;
    let mut r = response(&f, id)?;
    // An authored pressure control, not a model response or a quality oracle.
    // Separate subclaims of the same Source retain separate complete bindings.
    for index in 1..24 {
        let mut p = r.paragraphs[0].clone();
        p.text =
            format!("Authored claim {index}: greet appends the supplied name to a salutation.");
        p.citations[0].claim_end_byte = p.text.len();
        p.citations[0].claim_sha256 = explanation_text_hash(&p.text);
        r.paragraphs.push(p);
    }
    let bytes = serde_json::to_vec(&r)?.len();
    validate_explanation(&plan, &r)?;
    let retained = retain_explanation(&plan, r.clone(), i64::MIN);
    let retained_bytes = encode_retained_explanation(&retained)?.len();
    assert_eq!(
        retained_bytes - bytes,
        plan.retention_budget.metadata_byte_reserve
    );
    assert_eq!(r.generated_units().count(), 24);
    assert_eq!(r.citations().count(), 24);
    assert_eq!(
        r.citations()
            .map(|c| &c.source_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        1
    );
    // Generic navigation still has the v6 grounding obligation. A self-declared
    // heading cannot introduce an uncited technical claim via a role exemption.
    for text in [
        "Responsibilities",
        "All remote calls are authenticated",
        "모든 데이터는 암호화됩니다",
    ] {
        let mut heading = r.paragraphs[0].clone();
        heading.question = ExplanationQuestion::Heading;
        heading.text = text.into();
        heading.citations.clear();
        let mut bad = r.clone();
        bad.paragraphs.insert(0, heading);
        assert!(validate_explanation(&plan, &bad).is_err());
    }
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        r,
    )?;
    let read = f
        .ops
        .read_repository_explanation(f.project, "en", FixedLocale::English)?;
    assert_eq!(read.prose.len(), 24);
    assert_eq!(
        read.prose.iter().map(|p| p.citations.len()).sum::<usize>(),
        24
    );
    println!("repository pressure control: units=24 bindings=24 distinct_sources=1 response_bytes={bytes} retained_bytes={retained_bytes} metadata_bytes={}", plan.retention_budget.metadata_byte_reserve);
    Ok(())
}
#[test]
fn source_rich_repository_publication_preserves_content_above_40_kib_and_refuses_overflow(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = source(&observe(&f, "main.py", None)?)?;
    let subject = ExplanationSubject::Repository(f.analysis);
    let mut realization = response(&f, id)?;
    // Authored resource control, never a repository-breadth or entailment oracle.
    // Distinct claim bindings and UTF-8 prose traverse the actual publication read.
    for index in 1..24 {
        let mut unit = realization.paragraphs[0].clone();
        unit.text = format!(
            "Observed claim {index}: greet appends a supplied name. {}",
            "현재 Source 관측은 실행 결과나 사용자 선택을 증명하지 않습니다. ".repeat(11)
        );
        unit.citations[0].claim_end_byte = unit.text.len();
        unit.citations[0].claim_sha256 = explanation_text_hash(&unit.text);
        realization.paragraphs.push(unit);
    }
    let bytes = serde_json::to_vec(&realization)?.len();
    assert!((40_961..=49_152).contains(&bytes), "control bytes: {bytes}");
    f.ops
        .record_explanation(f.project, subject, "en", realization.clone())?;
    let read = f
        .ops
        .read_repository_explanation(f.project, "en", FixedLocale::English)?;
    assert_eq!(read.explanation_state, ExplanationState::Current);
    assert_eq!(
        read.prose
            .iter()
            .map(|u| (&u.text, &u.citations))
            .collect::<Vec<_>>(),
        realization
            .paragraphs
            .iter()
            .map(|u| (&u.text, &u.citations))
            .collect::<Vec<_>>()
    );
    let before = f.ops.privacy_status(f.project)?;
    let mut oversized = realization.clone();
    oversized.paragraphs[0].text.push_str(&"x".repeat(10_000));
    oversized.paragraphs[0].citations[0].claim_end_byte = oversized.paragraphs[0].text.len();
    oversized.paragraphs[0].citations[0].claim_sha256 =
        explanation_text_hash(&oversized.paragraphs[0].text);
    let error = f
        .ops
        .record_explanation(f.project, subject, "en", oversized)
        .err()
        .ok_or("overflow admitted")?;
    assert!(error.message().contains("compact realization JSON"));
    assert_eq!(f.ops.privacy_status(f.project)?, before);
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "en", FixedLocale::English)?
            .prose,
        read.prose
    );
    f.ops
        .delete_repository_source_observation(f.project, f.analysis, id)?;
    let unavailable = f
        .ops
        .read_repository_explanation(f.project, "en", FixedLocale::English)?;
    assert_ne!(unavailable.explanation_state, ExplanationState::Current);
    assert!(unavailable.provenance.is_none());
    assert!(unavailable.source_excerpts.is_empty());
    assert!(!unavailable
        .prose
        .iter()
        .any(|p| p.role == AnswerRole::GeneratedInterpretation));
    Ok(())
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
    assert!(inventory["investigation"]
        .as_str()
        .is_some_and(|s| s.contains("explicit Source")));
    let instructions = prepared["plan"]["instructions"]
        .as_str()
        .ok_or("instructions")?;
    for obligation in [
        "independently identify",
        "two materially distinct",
        "failure/exit",
        "unknown/omitted",
        "canonical Project Purpose",
        "core domain record relationships before storage",
        "required callees/callbacks",
        "earlier language is never the expected answer",
        "effect_review.py",
    ] {
        assert!(
            instructions.contains(obligation),
            "missing Repository investigation obligation: {obligation}"
        );
    }
    assert!(!instructions.contains("one complete representative operation"));
    // Repository guidance must not route investigation through Work correction.
    assert!(!instructions.contains("Corrections need evidenced symptom"));
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
            diagrams: Vec::new(),
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
fn repository_flow_steps_preserve_grounding_order_and_publication_lifetime(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = source(&observe(&f, "main.py", None)?)?;
    let subject = ExplanationSubject::Repository(f.analysis);
    let plan = f.ops.prepare_explanation(f.project, subject, "en")?;
    assert!(plan
        .question_grounding
        .iter()
        .any(|r| r.question == ExplanationQuestion::FlowStep));
    let mut flow = response(&f, id)?;
    flow.paragraphs[0].question = ExplanationQuestion::FlowStep;
    let mut second = flow.paragraphs[0].clone();
    second.text = "Return the greeting with the supplied name.".into();
    second.citations[0].claim_end_byte = second.text.len();
    second.citations[0].claim_sha256 = explanation_text_hash(&second.text);
    flow.paragraphs.push(second);
    f.ops
        .record_explanation(f.project, subject, "en", flow.clone())?;
    let before = f.ops.privacy_status(f.project)?;
    let read = f
        .ops
        .read_repository_explanation(f.project, "en", FixedLocale::English)?;
    assert_eq!(
        read.prose
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>(),
        flow.paragraphs
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
    );
    assert!(read.prose.iter().all(|p| p.question == "FlowStep"));
    let mut page = f
        .ops
        .read_explanation_page(f.project, subject, "en", 0, None)?;
    let mut complete = String::new();
    loop {
        complete.push_str(page["chunk"].as_str().ok_or("chunk")?);
        let Some(offset) = page["next_offset"].as_u64() else {
            break;
        };
        page = f.ops.read_explanation_page(
            f.project,
            subject,
            "en",
            offset as usize,
            Some(page["fingerprint"].as_str().ok_or("fingerprint")?),
        )?;
    }
    let document: Value = serde_json::from_str(&complete)?;
    assert_eq!(document["units"][1]["question"], "flow_step");
    assert_eq!(document["units"][1]["text"], flow.paragraphs[1].text);
    assert_eq!(f.ops.privacy_status(f.project)?, before);
    for failure in ["uncited", "claim", "binding", "format"] {
        let mut bad = flow.clone();
        match failure {
            "uncited" => bad.paragraphs[1].citations.clear(),
            "claim" => bad.paragraphs[1].text.push_str("Unsupported addition"),
            "binding" => bad.paragraphs[1].citations[0].analysis_snapshot_id = "0".repeat(64),
            _ => bad.format_version += 1,
        }
        if failure == "claim" {
            // Alter an actually bound span, rather than append an uncited claim
            // whose truth cannot be decided by structural validation.
            bad.paragraphs[1].text.replace_range(..1, "X");
        }
        assert!(
            f.ops
                .record_explanation(f.project, subject, "en", bad)
                .is_err(),
            "{failure}"
        );
        assert_eq!(f.ops.privacy_status(f.project)?, before);
    }
    f.ops
        .delete_repository_source_observation(f.project, f.analysis, id)?;
    assert!(f
        .ops
        .read_explanation_page(f.project, subject, "en", 0, None)
        .is_err());
    let read = f
        .ops
        .read_repository_explanation(f.project, "en", FixedLocale::English)?;
    assert_ne!(read.explanation_state, ExplanationState::Current);
    assert!(read.prose.iter().all(|p| p.question != "FlowStep"));
    assert!(read.provenance.is_none());
    Ok(())
}

#[test]
fn publication_requests_recheck_sources_beyond_the_returned_page() -> Result<(), Box<dyn Error>> {
    for change in ["delete", "expire", "corrupt", "forget"] {
        let mut f = fixture()?;
        fs::write(f.root.join("support.py"), "def support():\n    return 11\n")?;
        f.analysis = f
            .ops
            .analyze(f.project, vec!["secret.py".into()])?
            .value
            .ok_or("analysis")?
            .analysis
            .identity;
        let first = source(&observe(&f, "main.py", None)?)?;
        let late = source(&observe(&f, "support.py", None)?)?;
        let subject = ExplanationSubject::Repository(f.analysis);
        let plan = f.ops.prepare_explanation(f.project, subject, "en")?;
        let realization = f.ops.compose_repository_explanation(
            f.project,
            f.analysis,
            RepositoryExplanationDraft {
                diagrams: Vec::new(),
                plan_fingerprint: plan.fingerprint,
                language: "en".into(),
                generator: ExplanationGenerator {
                    host: "authored-control".into(),
                    session: "request-sharing".into(),
                    agent: None,
                    model: None,
                },
                paragraphs: vec![
                    RepositoryReaderUnit {
                        question: ExplanationQuestion::Observation,
                        text: format!("Greeting context: {}", "안녕 \"quoted\"\\\n".repeat(650)),
                        evidence_keys: vec!["repository_basis".into()],
                        sources: vec![RepositoryClaimRange {
                            source_id: first,
                            start_byte: 0,
                            end_byte: 15,
                            claim: None,
                        }],
                    },
                    RepositoryReaderUnit {
                        question: ExplanationQuestion::Observation,
                        text: "The separate support function returns 11.".into(),
                        evidence_keys: vec!["repository_basis".into()],
                        sources: vec![RepositoryClaimRange {
                            source_id: late,
                            start_byte: 0,
                            end_byte: 29,
                            claim: None,
                        }],
                    },
                ],
            },
        )?;
        f.ops
            .record_explanation(f.project, subject, "en", realization)?;
        let page = f
            .ops
            .read_explanation_page(f.project, subject, "en", 0, None)?;
        let offset = page["next_offset"].as_u64().ok_or("partial page")? as usize;
        let fingerprint = page["fingerprint"].as_str().ok_or("fingerprint")?;
        // Warm the same owner with both exact Source and continuation reads.
        f.ops
            .read_repository_explanation_source(f.project, f.analysis, late)?;
        f.ops
            .read_explanation_page(f.project, subject, "en", offset, Some(fingerprint))?;
        match change {
            "delete" => {
                f.ops
                    .delete_repository_source_observation(f.project, f.analysis, late)?;
            }
            "forget" => {
                let authorization = f.ops.record_user_source(
                    f.project,
                    "authored-control".into(),
                    "request-sharing".into(),
                    "Forget the support Source".into(),
                )?;
                let auth = source(&serde_json::json!({"source_id":authorization.identity}))?;
                f.ops.forget_record(
                    f.project,
                    volicord_context::CanonicalRecordId::Source(late),
                    auth,
                )?;
            }
            _ => {
                // Out-of-band lifetime/integrity mutation cannot be hidden by
                // earlier request inputs or canonical revision equality.
                let db = rusqlite::Connection::open(f.ops.layout().privacy_store())?;
                let (id, raw): (Vec<u8>, String) = db.query_row(
                    "SELECT id,record_json FROM managed_derived WHERE record_json LIKE ?1",
                    [format!("%repository-source:{}:{}%", f.analysis, late)],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                let mut record: Value = serde_json::from_str(&raw)?;
                if change == "expire" {
                    record["retained_until"] = serde_json::json!(1);
                } else {
                    record["content"] = serde_json::json!("invalid Source JSON");
                }
                db.execute(
                    "UPDATE managed_derived SET record_json=?1 WHERE id=?2",
                    rusqlite::params![serde_json::to_string(&record)?, id],
                )?;
            }
        }
        let before = f.ops.privacy_status(f.project)?;
        for offset in [0, offset] {
            let result = f.ops.read_explanation_page(
                f.project,
                subject,
                "en",
                offset,
                (offset > 0).then_some(fingerprint),
            );
            // Authorized retained-reading policy: body expiry/corruption is
            // range-local; explicit removal/forgetting still denies all meaning.
            if matches!(change, "expire" | "corrupt") {
                let result = result?;
                assert_eq!(result["fingerprint"], fingerprint);
                assert!(result["applicability"]["source_ranges"]
                    .as_array()
                    .ok_or("ranges")?
                    .iter()
                    .any(|a| a["available"] == false));
            } else {
                assert!(result.is_err(), "{change}");
            }
        }
        assert_eq!(f.ops.privacy_status(f.project)?, before);
    }
    Ok(())
}

#[test]
fn confirmed_repository_changes_are_distinct_from_unavailable_comparison(
) -> Result<(), Box<dyn Error>> {
    for change in ["cited", "nearby_noncited", "unrelated_included", "missing"] {
        let f = fixture()?;
        let id = source(&observe(&f, "main.py", None)?)?;
        f.ops.record_explanation(
            f.project,
            ExplanationSubject::Repository(f.analysis),
            "en",
            response(&f, id)?,
        )?;
        let before = f.ops.privacy_status(f.project)?;
        match change {
            "cited" => fs::write(
                f.root.join("main.py"),
                "def salute(name):\n    return '안녕 ' + name\n",
            )?,
            "nearby_noncited" => {
                let text = fs::read_to_string(f.root.join("main.py"))?;
                fs::write(
                    f.root.join("main.py"),
                    format!("{text}\n# outside the citation\n"),
                )?;
            }
            "unrelated_included" => {
                fs::write(f.root.join(".gitignore"), "ignored.py\n# comment\n")?
            }
            _ => fs::rename(&f.root, f.root.with_extension("unavailable"))?,
        }
        let read = f
            .ops
            .read_repository_explanation(f.project, "en", FixedLocale::English)?;
        let unavailable = change == "missing";
        assert_eq!(
            read.explanation_state,
            ExplanationState::Current,
            "{change}"
        );
        assert_eq!(
            read.applicability.limitation,
            Some(if unavailable {
                ExplanationFailureReason::ComparisonUnavailable
            } else {
                ExplanationFailureReason::Changed
            })
        );
        assert_eq!(
            read.applicability.repository_comparison,
            if unavailable {
                RepositoryComparisonState::Unavailable
            } else {
                RepositoryComparisonState::Changed
            }
        );
        assert!(read.provenance.is_some() && !read.source_excerpts.is_empty());
        assert_eq!(
            read.applicability.publication,
            PublicationReadability::Readable
        );
        assert_eq!(f.ops.privacy_status(f.project)?, before);
    }
    Ok(())
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
    let reading = f
        .ops
        .read_repository_explanation(f.project, "en", FixedLocale::English)?;
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
                .read_repository_explanation(f.project, "en", FixedLocale::English)?
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
            .read_repository_explanation(f.project, "ko", FixedLocale::English)?
            .explanation_state,
        ExplanationState::Missing
    );
    // Copy deletion immediately withholds every generated field including headings.
    f.ops
        .delete_repository_source_observation(f.project, f.analysis, id)?;
    let unavailable = f
        .ops
        .read_repository_explanation(f.project, "en", FixedLocale::English)?;
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
            .read_repository_explanation(f.project, "en", FixedLocale::English)?
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
    // Ordinary repository changes retain the exact admitted interpretation.
    assert!(f
        .ops
        .read_repository_explanation(f.project, "en", FixedLocale::English)?
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
fn unavailable_ranges_preserve_prose_but_damaged_publications_never_supply_it(
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
        let reading = f
            .ops
            .read_repository_explanation(f.project, "en", FixedLocale::English)?;
        if variant.ends_with("output") {
            assert_ne!(
                reading.explanation_state,
                ExplanationState::Current,
                "{variant}"
            );
            assert!(!reading
                .prose
                .iter()
                .any(|p| p.role == AnswerRole::GeneratedInterpretation));
        } else {
            assert_eq!(
                reading.explanation_state,
                ExplanationState::Current,
                "{variant}"
            );
            assert!(!reading.applicability.source_ranges[0].available);
        }
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
            .read_repository_explanation(f.project, "en", FixedLocale::English)?
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
    let absent = f
        .ops
        .read_repository_explanation(f.project, "en", FixedLocale::English)?;
    assert_eq!(absent.explanation_state, ExplanationState::Current);
    assert_eq!(
        absent.applicability.repository_comparison,
        RepositoryComparisonState::Unavailable
    );
    assert!(absent.source_excerpts.is_empty());
    assert!(f
        .ops
        .read_repository_explanation_source(f.project, f.analysis, id)
        .is_err());
    fs::rename(&hidden, &path)?;
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "en", FixedLocale::English)?
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
            .read_repository_explanation(f.project, "en", FixedLocale::English)?
            .explanation_state,
        ExplanationState::Unavailable
    );
    assert_eq!(fs::read(&database)?, b"Authored unavailable SQLite control");
    fs::remove_file(&database)?;
    fs::rename(&original, &database)?;
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "en", FixedLocale::English)?
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
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "en", FixedLocale::English)?
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
            summary["coverage"]["included"]["area_count"]
                .as_u64()
                .map(|n| n as usize),
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
fn large_polyglot_inventory_keeps_exact_scope_outside_bounded_reader_plan(
) -> Result<(), Box<dyn Error>> {
    let mut f = fixture()?;
    // Independent input cardinalities/paths; no generated answer supplied.
    for n in 0..800 {
        let dir = f.root.join(format!(
            "components/independently_owned_component_{n:04}/implementation"
        ));
        fs::create_dir_all(&dir)?;
        let (name, body) = match n % 3 {
            0 => ("entry.rs", "pub fn value() -> u32 { 1 }\n"),
            1 => ("entry.js", "export function value() { return 1; }\n"),
            _ => ("entry.py", "def value():\n    return 1\n"),
        };
        fs::write(dir.join(name), body)?;
    }
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
    let metadata = &full["analysis_metadata"];
    let entries = metadata["inventory"]["entries"]
        .as_array()
        .ok_or("entries")?;
    let reports = metadata["capabilities"].as_array().ok_or("reports")?;
    let full_coverage_bytes: usize = reports
        .iter()
        .map(|r| r["coverage"].to_string().len())
        .sum();
    assert!(full_coverage_bytes > EXPLANATION_RETAINED_BYTE_LIMIT);
    let plan =
        f.ops
            .prepare_explanation(f.project, ExplanationSubject::Repository(f.analysis), "en")?;
    let basis = &plan
        .evidence
        .iter()
        .find(|e| e.key == "repository_basis")
        .ok_or("basis")?
        .content;
    assert_eq!(basis["inventory_scope"]["entry_count"], entries.len());
    assert_eq!(
        basis["inventory_scope"]["sha256"],
        explanation_text_hash(&metadata["inventory"].to_string())
    );
    assert_eq!(
        basis["scope_detail"]["analysis_snapshot_id"],
        f.analysis.to_string()
    );
    let compact = basis["capabilities"].as_array().ok_or("summaries")?;
    assert_eq!(compact.len(), reports.len());
    for (summary, original) in compact.iter().zip(reports) {
        for field in [
            "language",
            "area",
            "capability",
            "state",
            "reason",
            "freshness",
            "adapter",
            "analyzer",
            "usable_remainder",
            "user_visible_consequence",
        ] {
            assert_eq!(summary[field], original[field], "{field}");
        }
        for field in [
            "included",
            "excluded",
            "unsupported",
            "unavailable",
            "failed",
            "stale",
        ] {
            let areas = original["coverage"][field].as_array().ok_or("areas")?;
            assert_eq!(summary["coverage"][field]["area_count"], areas.len());
            if !areas.is_empty() {
                assert_eq!(
                    summary["coverage"][field]["sha256"],
                    explanation_text_hash(&original["coverage"][field].to_string())
                );
            }
        }
    }
    assert!(serde_json::to_vec(&plan)?.len() <= EXPLANATION_PLAN_BYTE_LIMIT);
    assert_eq!(
        plan.retention_budget.response_byte_capacity,
        EXPLANATION_BYTE_LIMIT
    );
    let id = source(&observe(&f, "main.py", None)?)?;
    let realization = response(&f, id)?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        realization,
    )?;
    let before = f.ops.canonical_basis(f.project)?;
    let answers = f
        .ops
        .read_repository_explanation(f.project, "en", FixedLocale::English)?;
    assert_eq!(answers.explanation_state, ExplanationState::Current);
    assert!(answers.facts.iter().any(|a| a.text.contains("802 files")));
    assert!(answers.facts.iter().any(|a| a
        .text
        .contains("Full inventory and coverage paths are excluded")));
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    fs::write(f.root.join("main.py"), "def changed(): pass\n")?;
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "en", FixedLocale::English)?
            .explanation_state,
        ExplanationState::Current
    );
    // Historical retained Source remains retrievable, never labeled current code.
    assert!(f
        .ops
        .read_repository_explanation_source(f.project, f.analysis, id)
        .is_ok());
    println!(
        "full coverage={full_coverage_bytes} bytes; bounded plan={} bytes",
        serde_json::to_vec(&plan)?.len()
    );
    Ok(())
}

#[test]
fn repository_reading_uses_verified_inventory_without_claiming_graph_integrity(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let directory = f
        .ops
        .layout()
        .root()
        .join("derived/analysis")
        .join(f.project.to_string());
    let manifest: Value =
        serde_json::from_slice(&fs::read(directory.join(format!("{}.json", f.analysis)))?)?;
    let shape = manifest["shape_blobs"][0].as_str().ok_or("shape")?;
    fs::write(
        directory.join("blobs").join(format!("{shape}.shape")),
        b"corrupt graph",
    )?;
    assert!(f
        .ops
        .repository_explanation_evidence(f.project, f.analysis)
        .is_err());
    let plan =
        f.ops
            .prepare_explanation(f.project, ExplanationSubject::Repository(f.analysis), "en")?;
    assert!(plan.evidence[0].content["scope_detail"]["validation"]
        .as_str()
        .ok_or("validation")?
        .contains("graph integrity and complete semantic understanding not established"));
    let id = source(&observe(&f, "main.py", None)?)?;
    let read = f
        .ops
        .read_repository_explanation_source(f.project, f.analysis, id)?;
    assert!(read["observation"]["detail"]["repository_worktree"].is_null());
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        response(&f, id)?,
    )?;
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "en", FixedLocale::English)?
            .explanation_state,
        ExplanationState::Current
    );
    let metadata = manifest["metadata_blob"].as_str().ok_or("metadata")?;
    fs::write(
        directory.join("blobs").join(format!("{metadata}.metadata")),
        b"corrupt metadata",
    )?;
    assert!(f
        .ops
        .prepare_explanation(f.project, ExplanationSubject::Repository(f.analysis), "en")
        .is_err());
    assert!(f
        .ops
        .read_repository_explanation_source(f.project, f.analysis, id)
        .is_err());
    assert_eq!(
        f.ops
            .read_repository_explanation(f.project, "en", FixedLocale::English)?
            .explanation_state,
        ExplanationState::Current
    );
    Ok(())
}

#[test]
fn same_named_populated_and_workless_projects_keep_their_own_repository_articles(
) -> Result<(), Box<dyn Error>> {
    use volicord_context::*;
    let f = fixture()?;
    let mut store = Store::open(f.ops.layout().canonical_store())?;
    let revision = f.ops.canonical_basis(f.project)?.project.revision;
    let principal = Principal {
        kind: PrincipalKind::User,
        identity: "authored-contract-control".into(),
    };
    let user = store
        .record_source(
            OperationId::from_bytes([31; 16]),
            f.project,
            SourceDraft {
                expected_project_revision: revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "test".into(),
                    session: "synthetic".into(),
                    turn: "Inspect the greeting function; this is an authored contract control."
                        .into(),
                },
                actor: principal.clone(),
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value
        .id;
    let goal = store
        .record_context_item(
            OperationId::from_bytes([32; 16]),
            f.project,
            ContextItemDraft {
                expected_project_revision: revision,
                role: ContextItemRole::Goal,
                statement: "Inspect the greeting function".into(),
                provenance_role: StatementProvenanceRole::UserStatement,
                author: principal,
                source_basis: vec![user],
                applicability: ApplicabilityScope {
                    paths: vec!["main.py".into()],
                    components: vec![],
                    work_contexts: vec![],
                },
            },
        )?
        .value
        .id;
    drop(store);
    let plan = f
        .ops
        .prepare_explanation(f.project, ExplanationSubject::Work(goal), "en")?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Work(goal),
        "en",
        ExplanationRealization {
            diagrams: Vec::new(),
            format_kind: EXPLANATION_KIND.into(),
            format_version: EXPLANATION_VERSION,
            plan_fingerprint: plan.fingerprint,
            language: "en".into(),
            generator: ExplanationGenerator {
                host: "authored-contract-control".into(),
                session: "synthetic".into(),
                agent: None,
                model: None,
            },
            paragraphs: vec![ExplanationParagraph {
                question: ExplanationQuestion::Purpose,
                text: "Inspect the greeting function; no outcome is recorded.".into(),
                evidence_keys: vec!["goal".into()],
                citations: vec![],
            }],
        },
    )?;
    let first_source = source(&observe(&f, "main.py", None)?)?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        response(&f, first_source)?,
    )?;
    let root = f._temp.path().join("same-name-workless");
    fs::create_dir(&root)?;
    fs::write(root.join("main.py"), "def value():\n    return 7\n")?;
    let other = f
        .ops
        .initialize_project("source reading", Some(&root))?
        .project
        .id;
    let analysis = f
        .ops
        .analyze(other, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let receipt = f
        .ops
        .observe_repository_file(RepositoryFileObservationRequest {
            project_id: other,
            analysis_snapshot_id: analysis,
            path: "main.py".into(),
            byte_range: None,
            retain_seconds: 3600,
        })?;
    let other_source = source(&receipt)?;
    let other_plan =
        f.ops
            .prepare_explanation(other, ExplanationSubject::Repository(analysis), "en")?;
    let draft = RepositoryExplanationDraft {
        diagrams: Vec::new(),
        plan_fingerprint: other_plan.fingerprint,
        language: "en".into(),
        generator: ExplanationGenerator {
            host: "authored-contract-control".into(),
            session: "synthetic".into(),
            agent: None,
            model: None,
        },
        paragraphs: vec![RepositoryReaderUnit {
            question: ExplanationQuestion::Observation,
            text: "The independent value function returns seven.".into(),
            evidence_keys: vec!["repository_basis".into()],
            sources: vec![RepositoryClaimRange {
                source_id: other_source,
                start_byte: 0,
                end_byte: 25,
                claim: None,
            }],
        }],
    };
    let mut foreign = draft.clone();
    foreign.paragraphs[0].sources[0].source_id = first_source;
    assert!(f
        .ops
        .compose_repository_explanation(other, analysis, foreign)
        .is_err());
    let realization = f
        .ops
        .compose_repository_explanation(other, analysis, draft)?;
    f.ops.record_explanation(
        other,
        ExplanationSubject::Repository(analysis),
        "en",
        realization,
    )?;
    let before = f.ops.canonical_basis(f.project)?;
    let other_before = f.ops.canonical_basis(other)?;
    assert_eq!(
        before.project.display_name,
        other_before.project.display_name
    );
    assert_ne!(f.project, other);
    assert!(other_before.context_items.is_empty());
    let populated = f
        .ops
        .project_projection_selected(f.project, WorkSelector::ExactWork(goal))?;
    assert_eq!(
        populated.selected_work.as_ref().ok_or("Work")?.work_item_id,
        goal
    );
    assert_eq!(
        explanation_answers(
            &populated
                .selected_work
                .as_ref()
                .ok_or("Work")?
                .reading
                .explanations,
            "en",
            FixedLocale::English
        )
        .explanation_state,
        ExplanationState::Current
    );
    for (project, snapshot, text) in [
        (f.project, f.analysis, "greet builds"),
        (other, analysis, "returns seven"),
    ] {
        let answers = f
            .ops
            .read_repository_explanation(project, "en", FixedLocale::English)?;
        assert_eq!(answers.explanation_state, ExplanationState::Current);
        let provenance = answers.provenance.ok_or("provenance")?;
        assert_eq!(provenance.project_id, project.to_string());
        assert_eq!(provenance.subject, ExplanationSubject::Repository(snapshot));
        assert!(answers.prose.iter().any(|p| p.text.contains(text)));
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_volicord"))
            .args([
                "--runtime",
                f.ops.layout().root().to_str().ok_or("runtime")?,
                "--project",
                &project.to_string(),
                "--json",
                "repository",
                "explain",
                "read",
            ])
            .output()?;
        assert!(output.status.success(), "{:?}", output);
        let read: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            read["answers"]["provenance"]["project_id"],
            project.to_string()
        );
        assert_eq!(read["answers"]["explanation_state"], "current");
    }
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    assert_eq!(other_before, f.ops.canonical_basis(other)?);
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
#[test]
fn repository_cli_keeps_fixed_locale_independent_from_publication_language(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = source(&observe(&f, "main.py", None)?)?;
    let realization = response(&f, id)?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        realization,
    )?;
    let read = |language: &str, locale: &str| -> Result<Value, Box<dyn Error>> {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = run_cli(
            [
                "--runtime",
                f.ops.layout().root().to_str().ok_or("runtime path")?,
                "--project",
                &f.project.to_string(),
                "--json",
                "--locale",
                locale,
                "repository",
                "explain",
                "read",
                "--language",
                language,
            ],
            &mut stdout,
            &mut stderr,
        );
        assert_eq!(
            exit,
            CliExit::SUCCESS,
            "{}",
            String::from_utf8_lossy(&stderr)
        );
        Ok(serde_json::from_slice::<Value>(&stdout)?["answers"].clone())
    };
    let before = f.ops.canonical_basis(f.project)?;
    let english = read("en", "en")?;
    let korean_facts = read("en", "ko")?;
    assert_eq!(english["explanation_state"], "current");
    assert_eq!(korean_facts["explanation_state"], "current");
    assert_eq!(english["prose"], korean_facts["prose"]);
    assert_eq!(english["provenance"], korean_facts["provenance"]);
    assert_eq!(english["source_excerpts"], korean_facts["source_excerpts"]);
    assert!(english["facts"][1]["text"]
        .as_str()
        .ok_or("English coverage")?
        .starts_with("Within the selected Analysis Snapshot"));
    assert!(korean_facts["facts"][1]["text"]
        .as_str()
        .ok_or("Korean coverage")?
        .starts_with("선택한 Analysis Snapshot"));
    let mut korean = response(&f, id)?;
    let plan =
        f.ops
            .prepare_explanation(f.project, ExplanationSubject::Repository(f.analysis), "ko")?;
    korean.language = "ko".into();
    korean.plan_fingerprint = plan.fingerprint;
    korean.paragraphs[0].text = "독립적으로 기록한 한국어 안내문입니다.".into();
    korean.paragraphs[0].citations[0].claim_end_byte = korean.paragraphs[0].text.len();
    korean.paragraphs[0].citations[0].claim_sha256 =
        explanation_text_hash(&korean.paragraphs[0].text);
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "ko",
        korean,
    )?;
    let korean_prose = read("ko", "en")?;
    let korean_both = read("ko", "ko")?;
    assert_eq!(korean_prose["prose"], korean_both["prose"]);
    assert_ne!(english["prose"], korean_prose["prose"]);
    assert_eq!(english["facts"], korean_prose["facts"]);
    assert_eq!(korean_facts["facts"], korean_both["facts"]);
    assert_eq!(korean_prose["provenance"], korean_both["provenance"]);
    // UI locale cannot translate, substitute or create a missing publication.
    for locale in ["en", "ko"] {
        let missing = read("fr", locale)?;
        assert_eq!(missing["explanation_state"], "missing");
        assert!(missing["prose"]
            .as_array()
            .ok_or("missing prose")?
            .iter()
            .all(|unit| unit["role"] != "generated_interpretation"));
        assert!(missing["provenance"].is_null());
        assert_eq!(missing["applicability"]["limitation"], "missing");
        assert_eq!(missing["source_excerpts"], serde_json::json!([]));
        assert_eq!(missing["diagrams"], serde_json::json!([]));
        let units = missing["prose"].as_array().ok_or("availability units")?;
        assert_eq!(units.len(), 1);
        let notice = &units[0];
        assert_eq!(notice["question"], "ExplanationAvailability");
        assert_eq!(notice["role"], "unavailable");
        assert_eq!(notice["citations"], serde_json::json!([]));
        assert_eq!(notice["evidence_keys"], serde_json::json!([]));
        let text = notice["text"].as_str().ok_or("availability notice")?;
        assert!(!text.trim().is_empty());
        assert_eq!(
            text.chars().any(|c| ('\u{ac00}'..='\u{d7a3}').contains(&c)),
            locale == "ko",
            "fixed UI notice follows locale, not the missing French publication"
        );
    }
    f.ops
        .delete_repository_source_observation(f.project, f.analysis, id)?;
    for locale in ["en", "ko"] {
        let unavailable = read("en", locale)?;
        assert_ne!(unavailable["explanation_state"], "current");
        assert!(unavailable["prose"]
            .as_array()
            .ok_or("unavailable prose")?
            .iter()
            .all(|unit| unit["role"] != "generated_interpretation"));
        assert!(unavailable["provenance"].is_null());
    }
    assert_eq!(before, f.ops.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn independent_publication_expiry_and_damaged_admission_links_remain_blocking(
) -> Result<(), Box<dyn Error>> {
    for variant in [
        "publication_expiry",
        "missing_source_link",
        "foreign_source_project",
        "changed_claim",
    ] {
        let f = fixture()?;
        let id = source(&observe(&f, "main.py", None)?)?;
        let subject = ExplanationSubject::Repository(f.analysis);
        f.ops
            .record_explanation(f.project, subject, "en", response(&f, id)?)?;
        let db = rusqlite::Connection::open(f.ops.layout().privacy_store())?;
        let (key,raw):(Vec<u8>,String) = db.query_row("SELECT id,record_json FROM managed_derived WHERE json_extract(record_json,'$.kind')='cached_summary'",[],|r|Ok((r.get(0)?,r.get(1)?)))?;
        let mut record: Value = serde_json::from_str(&raw)?;
        match variant {
            "publication_expiry" => record["retained_until"] = serde_json::json!(1),
            "missing_source_link" => record["canonical_links"] = serde_json::json!([]),
            "foreign_source_project" => {
                record["included_sources"][0]["project"] = serde_json::json!("11".repeat(16))
            }
            _ => {
                let mut body: Value =
                    serde_json::from_str(record["content"].as_str().ok_or("content")?)?;
                body["realization"]["paragraphs"][0]["text"] =
                    serde_json::json!("damaged published meaning");
                record["content"] = serde_json::json!(body.to_string());
            }
        }
        db.execute(
            "UPDATE managed_derived SET record_json=?1 WHERE id=?2",
            rusqlite::params![record.to_string(), key],
        )?;
        drop(db);
        let failed = f
            .ops
            .read_explanation_page(f.project, subject, "en", 0, None);
        assert!(failed.is_err(), "{variant}");
        let answers = f
            .ops
            .read_repository_explanation(f.project, "en", FixedLocale::English)?;
        assert!(
            answers.provenance.is_none() && answers.source_excerpts.is_empty(),
            "{variant}"
        );
        if variant == "publication_expiry" {
            assert_eq!(
                answers.applicability.limitation,
                Some(ExplanationFailureReason::Expired)
            );
            let mut privacy = volicord_privacy::PrivacyStore::open(f.ops.layout().privacy_store())?;
            assert_eq!(privacy.cleanup_expired(f.project)?.len(), 1);
            drop(privacy);
            let restarted = LocalOperations::new(f.ops.layout().clone());
            assert!(restarted
                .read_explanation_page(f.project, subject, "en", 0, None)
                .is_err());
        }
    }
    Ok(())
}
