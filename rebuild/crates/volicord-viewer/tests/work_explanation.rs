//! Fake responses test lifecycle only. Actual host generation proof uses this
//! fresh canonical fixture, the public prepare/record CLI, and the browser driver.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use volicord_context::*;
use volicord_operations::{run_cli, CliExit, LocalOperations};
use volicord_projections::*;
use volicord_viewer::{ViewerAdapter, ViewerLocale, ViewerServer, ViewerView};

fn fake(plan: &ExplanationPlan) -> ExplanationRealization {
    ExplanationRealization {
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint.clone(),
        language: plan.requested_language.clone(),
        generator: ExplanationGenerator {
            host: "unit-test-fake".into(),
            session: "fake".into(),
            agent: None,
            model: None,
        },
        paragraphs: [
            (ExplanationQuestion::Purpose, "goal"),
            (ExplanationQuestion::ReportedChange, "result"),
            (ExplanationQuestion::ExpectedEffect, "result"),
            (ExplanationQuestion::Verification, "verification"),
            (ExplanationQuestion::NextStep, "next_step"),
        ]
        .into_iter()
        .map(|(question, key)| ExplanationParagraph {
            question,
            text: format!("Fake unit-test paragraph for {question:?}"),
            evidence_keys: vec![key.into()],
        })
        .collect(),
    }
}
fn get(f: &reading_fixture::Fixture, path: &str) -> String {
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone())),
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse().expect("address"),
    )
    .expect("server");
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n");
    let mut out = Vec::new();
    server
        .serve_connection(&mut request.as_bytes(), &mut out)
        .expect("HTTP");
    String::from_utf8(out).expect("UTF-8")
}
#[test]
fn preparation_record_read_correction_delete_and_forget_use_current_operations(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?;
    let work = f.goals["relay"];
    let before = f.operations.canonical_basis(f.project)?;
    let plan = f
        .operations
        .prepare_explanation(f.project, ExplanationSubject::Work(work), "ko")?;
    assert!(plan
        .evidence
        .iter()
        .any(|e| e.key == "result" && e.content.to_string().contains("sequence")));
    assert!(plan
        .evidence
        .iter()
        .any(|e| e.key == "verification" && e.content.to_string().contains("reordered-response")));
    assert!(f
        .operations
        .privacy_status(f.project)?
        .managed_derived
        .is_empty());
    let input = f.repository.join("fake-response.json");
    std::fs::write(&input, serde_json::to_vec(&fake(&plan))?)?;
    let args = vec![
        "--runtime".to_owned(),
        f.operations.layout().root().to_string_lossy().into_owned(),
        "--project".into(),
        f.project.to_string(),
        "--json".into(),
        "work".into(),
        "explain".into(),
        "record".into(),
        "--work".into(),
        work.to_string(),
        "--language".into(),
        "ko".into(),
        "--input".into(),
        input.to_string_lossy().into_owned(),
    ];
    let (mut output, mut errors) = (Vec::new(), Vec::new());
    assert_eq!(
        run_cli(args, &mut output, &mut errors),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&errors)
    );
    let stored: serde_json::Value = serde_json::from_slice(&output)?;
    assert_eq!(
        stored["explanation"]["generator_identity_status"],
        "self_reported_not_independently_verified"
    );
    let page = get(
        &f,
        &format!("/?view=work&work={work}&locale=ko&language=ko"),
    );
    assert!(page.contains("class=\"work-explanation\""));
    assert!(page.contains("Fake unit-test paragraph"));
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    // Exact requested language, not UI locale; no hidden generation on reads.
    let untranslated = get(
        &f,
        &format!("/?view=work&work={work}&locale=ko&language=fr"),
    );
    assert!(!untranslated.contains("Fake unit-test paragraph"));
    let source = before
        .context_items
        .iter()
        .find(|c| c.id == work)
        .ok_or("Goal")?
        .source_basis[0];
    f.operations.correct_context_item(
        f.project,
        work,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            corrected_statement: format!(
                "{}.",
                before
                    .context_items
                    .iter()
                    .find(|c| c.id == work)
                    .ok_or("Goal")?
                    .statement
            ),
            user_authorization_source_id: source,
            kind: CorrectionKind::Expression,
        },
    )?;
    let stale = get(
        &f,
        &format!("/?view=work&work={work}&locale=ko&language=ko"),
    );
    assert!(stale.contains("근거가 변경"));
    assert!(!stale.contains("Fake unit-test paragraph"));
    assert!(f
        .operations
        .record_explanation(f.project, ExplanationSubject::Work(work), "ko", fake(&plan))
        .is_err());
    assert_eq!(
        f.operations
            .delete_explanations(f.project, ExplanationSubject::Work(work))?,
        1
    );
    let fresh =
        f.operations
            .prepare_explanation(f.project, ExplanationSubject::Work(work), "ko")?;
    f.operations.record_explanation(
        f.project,
        ExplanationSubject::Work(work),
        "ko",
        fake(&fresh),
    )?;
    let cp = f.checkpoints["relay-change"];
    assert!(
        f.operations
            .forget_record(f.project, CanonicalRecordId::Checkpoint(cp), source)?
            .managed_derived_cleanup_completed
    );
    assert!(f
        .operations
        .privacy_status(f.project)?
        .managed_derived
        .iter()
        .all(|r| r.content.is_none()));
    let bytes = std::fs::read(f.operations.layout().privacy_store())?;
    assert!(!bytes
        .windows(b"Fake unit-test paragraph".len())
        .any(|window| window == b"Fake unit-test paragraph"));
    let page = get(&f, &format!("/?view=work&work={work}&language=ko"));
    assert!(!page.contains("Fake unit-test paragraph"));
    Ok(())
}
#[test]
fn malformed_language_foreign_basis_and_versions_cannot_be_recorded(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?;
    let work = f.goals["export"];
    let plan =
        f.operations
            .prepare_explanation(f.project, ExplanationSubject::Work(work), "fr-CA")?;
    assert!(plan
        .evidence
        .iter()
        .any(|e| e.key == "verification" && e.content.to_string().contains("Failed")));
    for mutation in 0..5 {
        let mut response = fake(&plan);
        match mutation {
            0 => response.format_version += 1,
            1 => response.language = "en".into(),
            2 => response.plan_fingerprint = "wrong".into(),
            3 => response.paragraphs[1].evidence_keys = vec!["foreign".into()],
            _ => response.paragraphs[1].evidence_keys = vec!["goal".into()],
        }
        assert!(f
            .operations
            .record_explanation(f.project, ExplanationSubject::Work(work), "fr-CA", response)
            .is_err());
    }
    assert!(f
        .operations
        .privacy_status(f.project)?
        .managed_derived
        .is_empty());
    Ok(())
}
#[test]
fn conflict_corrupt_cache_and_absent_verification_do_not_claim_success(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = reading_fixture::rich_scenario()?;
    input["works"][1]["checkpoints"][0]["verification"] = serde_json::Value::Null;
    let f = reading_fixture::fixture_scenario(input)?;
    for (locale, label) in [("en", "No verification record"), ("ko", "검증 기록 없음")] {
        let page = get(
            &f,
            &format!("/?view=work&work={}&locale={locale}", f.goals["same_title"]),
        );
        assert!(page.contains(label));
    }
    let work = f.goals["export"];
    let plan = f
        .operations
        .prepare_explanation(f.project, ExplanationSubject::Work(work), "en")?;
    f.operations.record_explanation(
        f.project,
        ExplanationSubject::Work(work),
        "en",
        fake(&plan),
    )?;
    let mut store = Store::open(f.operations.layout().canonical_store())?;
    store.record_contradiction(
        OperationId::from_bytes([0x81; 16]),
        f.project,
        CanonicalRecordId::ContextItem(work),
        CanonicalRecordId::ContextItem(f.goals["relay"]),
    )?;
    drop(store);
    let changed =
        f.operations
            .prepare_explanation(f.project, ExplanationSubject::Work(work), "en")?;
    assert!(!changed.conflicts.is_empty());
    let page = get(&f, &format!("/?view=work&work={work}"));
    assert!(page.contains("Explanation is stale"));
    assert!(!page.contains("Fake unit-test paragraph"));
    f.operations
        .delete_explanations(f.project, ExplanationSubject::Work(work))?;
    f.operations.record_explanation(
        f.project,
        ExplanationSubject::Work(work),
        "en",
        fake(&changed),
    )?;
    assert!(get(&f, &format!("/?view=work&work={work}"))
        .contains("includes contradiction or supersession"));
    f.operations
        .delete_explanations(f.project, ExplanationSubject::Work(work))?;
    let canonical = f.operations.canonical_basis(f.project)?;
    let source = canonical
        .context_items
        .iter()
        .find(|c| c.id == work)
        .ok_or("Goal")?
        .source_basis[0];
    let grounding =
        volicord_repository_intelligence::CanonicalGrounding::from_read_basis(&canonical)?;
    for (content,label) in [("broken JSON".to_owned(),"Explanation is corrupt"),
        (serde_json::json!({"realization":{"format_kind":EXPLANATION_KIND,"format_version":EXPLANATION_VERSION+1}}).to_string(),"Explanation format is unsupported")] {
        let mut privacy=volicord_privacy::PrivacyStore::open(f.operations.layout().privacy_store())?;
        privacy.record_managed_derived(volicord_privacy::ManagedDerivedDraft {
            project_id:f.project,kind:volicord_privacy::ManagedDerivedKind::CachedSummary,
            provider:None,model:None,purpose:format!("explanation:work:{work}:en"),analysis_snapshot:None,
            included_sources:vec![grounding.source_reference(source)?],canonical_links:vec![volicord_privacy::ManagedCanonicalLink::ContextItem(work)],
            content,uncertainty:None,retained_until:None,retention_basis:"Unit-test corrupt cache fixture".into() })?;
        drop(privacy);
        assert!(get(&f,&format!("/?view=work&work={work}")).contains(label));
        f.operations.delete_explanations(f.project,ExplanationSubject::Work(work))?;
    }
    assert_eq!(canonical, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn seed_work_explanation_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let disposable = tempfile::tempdir()?;
    let output = std::env::var_os("VOLICORD_EXPLANATION_FIXTURE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or(disposable.path().to_owned());
    std::fs::create_dir_all(&output)?;
    let f = reading_fixture::rich_fixture_in(&output)?;
    let manifest = serde_json::json!({"project":f.project.to_string(),"runtime":f.operations.layout().root(),"repository":f.repository,
        "decisions":f.decisions.iter().map(|(k,v)|(k,v.to_string())).collect::<std::collections::BTreeMap<_,_>>(),
        "goals":f.goals.iter().map(|(k,v)|(k,v.to_string())).collect::<std::collections::BTreeMap<_,_>>()});
    std::fs::write(
        output.join("fixture.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    assert!(f
        .operations
        .privacy_status(f.project)?
        .managed_derived
        .is_empty());
    if std::env::var_os("VOLICORD_EXPLANATION_FIXTURE_ROOT").is_some() {
        let _ = f._temporary.keep();
    }
    Ok(())
}

#[test]
fn shared_answers_survive_restart_and_block_deleted_document_and_snapshot_publication(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?;
    let work = f.goals["checksum"];
    let subject = ExplanationSubject::Work(work);
    let plan = f.operations.prepare_explanation(f.project, subject, "ko")?;
    f.operations
        .record_explanation(f.project, subject, "ko", fake(&plan))?;
    let decision = f.decisions["project"];
    let dp = f.operations.prepare_explanation(
        f.project,
        ExplanationSubject::Decision(decision),
        "ko",
    )?;
    let mut response = fake(&plan);
    response.plan_fingerprint = dp.fingerprint.clone();
    response.paragraphs = [
        (ExplanationQuestion::UserRationale, "user_rationale"),
        (ExplanationQuestion::Recommendation, "recommendation"),
        (ExplanationQuestion::Consequences, "consequences"),
        (ExplanationQuestion::Applicability, "applicability"),
    ]
    .into_iter()
    .map(|(question, key)| ExplanationParagraph {
        question,
        text: format!("Fake decision lifecycle paragraph {question:?}"),
        evidence_keys: vec![key.into()],
    })
    .collect();
    f.operations.record_explanation(
        f.project,
        ExplanationSubject::Decision(decision),
        "ko",
        response,
    )?;
    let restarted = LocalOperations::new(f.operations.layout().clone());
    let (p, profile) = restarted.project_projection_profiled(f.project)?;
    // Work appears in several sections and Decision in catalog/resume; each
    // retained subject/language freshness basis is prepared exactly once.
    assert_eq!(profile.explanation_basis_preparations, 2);
    let selected = p
        .work_history
        .iter()
        .find(|w| w.work_item_id == work)
        .ok_or("Work")?;
    assert_eq!(selected.work_item_id, work);
    let expected = work_answers(selected, "ko", FixedLocale::Korean);
    assert_eq!(
        expected,
        work_answers(
            restarted
                .recall(f.project)?
                .selected_work
                .as_ref()
                .ok_or("Recall Work")?,
            "ko",
            FixedLocale::Korean
        )
    );
    assert!(decision_answers(
        p.resume
            .decisions
            .iter()
            .find(|d| d.decision_id == decision)
            .ok_or("Decision")?,
        "ko",
        FixedLocale::Korean
    )
    .text()
    .contains("Fake decision lifecycle paragraph"));
    for command in ["status", "recall", "decisions"] {
        for locale in ["en", "ko"] {
            let args = [
                "--runtime".to_string(),
                f.operations.layout().root().to_string_lossy().into_owned(),
                "--project".into(),
                f.project.to_string(),
                "--locale".into(),
                locale.into(),
                command.into(),
                "--language".into(),
                "ko".into(),
            ];
            let (mut out, mut errors) = (Vec::new(), Vec::new());
            assert_eq!(
                run_cli(args, &mut out, &mut errors),
                CliExit::SUCCESS,
                "{}",
                String::from_utf8_lossy(&errors)
            );
            let human = String::from_utf8(out)?;
            assert!(human.contains(if command == "decisions" {
                "Fake decision lifecycle paragraph"
            } else {
                "Fake unit-test paragraph"
            }));
            assert!(!human.contains("Audit record"));
            assert!(!human.contains(&f.project.to_string()));
            assert!(!human.contains(&work.to_string()));
            assert!(!human.contains("source_details"));
            assert!(!human.contains("NotRequested"));
            assert!(human.contains("--json"));
            assert!(human.contains(if locale == "ko" {
                "호스트 해석"
            } else {
                "Host interpretation"
            }));
            assert!(human.contains(if locale == "ko" {
                "기록된 사실과 파생 상태"
            } else {
                "Recorded facts and derived states"
            }));
        }
    }
    let request = DocumentRequest {
        requested_language: "ko".into(),
        fixed_locale: FixedLocale::Korean,
        generated_at: TimestampMicros::from_unix_micros(123),
        generator: GeneratorIdentity {
            generator: "unit-test".into(),
            agent: None,
            model: None,
        },
        requested_destinations: Vec::new(),
    };
    let documents = restarted.documents(f.project, &request)?;
    for d in [
        &documents.project_architecture_guide,
        &documents.decision_report,
        &documents.implementation_plan,
        &documents.handoff_resume,
    ] {
        assert!(d
            .body
            .sections
            .iter()
            .filter(|s| s.role == DocumentSectionRole::Reading)
            .flat_map(|s| &s.claims)
            .any(|c| c.text.contains("Fake unit-test paragraph")));
        assert!(d
            .metadata
            .explanations
            .iter()
            .any(|e| e.subject == subject && e.language == "ko"));
        assert!(d
            .markdown
            .content
            .contains("self_reported_not_independently_verified"));
        assert!(!d
            .body
            .sections
            .iter()
            .filter(|s| s.role == DocumentSectionRole::Reading)
            .flat_map(|s| &s.claims)
            .any(|c| c.text.contains("Audit record")));
    }
    let viewer = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let vr = volicord_viewer::ViewerRequest {
        project_id: f.project,
        locale: ViewerLocale::Korean,
        view: ViewerView::Overview,
        requested_language: "ko".into(),
        guarded_request: None,
    };
    let page = viewer.render_snapshot(&vr, TimestampMicros::from_unix_micros(123))?;
    assert!(page.html.contains("Fake unit-test paragraph"));
    restarted.delete_explanations(f.project, subject)?;
    let doc_path = f.repository.join("deleted-document.html");
    assert!(restarted
        .publish_document(&documents.handoff_resume, OutputFormat::Html, &doc_path)
        .is_err());
    assert!(!doc_path.exists());
    let snapshot_path = f.repository.join("deleted-snapshot.html");
    assert!(restarted
        .publish_viewer_snapshot(
            &page.html,
            &snapshot_path,
            f.project,
            &page.canonical_read_fingerprint,
            &page.explanations
        )
        .is_err());
    assert!(!snapshot_path.exists());
    assert!(!restarted
        .documents(f.project, &request)?
        .handoff_resume
        .markdown
        .content
        .contains("Fake unit-test paragraph"));
    let fresh = restarted.prepare_explanation(f.project, subject, "ko")?;
    restarted.record_explanation(f.project, subject, "ko", fake(&fresh))?;
    assert!(viewer
        .render_snapshot(&vr, TimestampMicros::from_unix_micros(124))?
        .html
        .contains("Fake unit-test paragraph"));
    let source = restarted
        .canonical_basis(f.project)?
        .context_items
        .iter()
        .find(|c| c.id == work)
        .ok_or("Goal")?
        .source_basis[0];
    restarted.forget_record(
        f.project,
        CanonicalRecordId::Checkpoint(f.checkpoints["checksum-change"]),
        source,
    )?;
    assert!(!viewer
        .render_snapshot(&vr, TimestampMicros::from_unix_micros(125))?
        .html
        .contains("Fake unit-test paragraph"));
    assert!(restarted
        .publish_document(
            &documents.handoff_resume,
            OutputFormat::Markdown,
            &f.repository.join("forgotten.md")
        )
        .is_err());
    Ok(())
}
