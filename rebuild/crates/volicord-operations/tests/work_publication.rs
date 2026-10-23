//! Independently authored lifecycle controls; no model or human-success claim.
#[path = "support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use std::error::Error;
use volicord_context::*;
use volicord_operations::*;
use volicord_projections::*;

fn fixture() -> Result<reading_fixture::Fixture, Box<dyn Error>> {
    let mut input: serde_json::Value = serde_json::from_str(reading_fixture::SCENARIO)?;
    input["prior_checkpoint_count"] = serde_json::json!(0);
    input["later_checkpoint_count"] = serde_json::json!(0);
    input["unassociated_checkpoint"] = serde_json::json!(false);
    for w in input["works"].as_array_mut().ok_or("Works")? {
        if w["key"] == "older" || w["key"] == "same_title" {
            w["title"] = serde_json::json!(
                "Investigate whether readQuery preserves the supplied value without patching it."
            );
        }
        if w["key"] == "older" {
            w["checkpoints"] = serde_json::json!([{"key":"inspection","work":"Paused","verification":"NotRun","state_change":"Inspection found readQuery returns its input unchanged. No patch was made.","next_step":"Review the investigation"}]);
        }
    }
    reading_fixture::fixture_scenario(input)
}

fn draft(plan: &ExplanationPlan, text: &str) -> WorkExplanationDraft {
    WorkExplanationDraft {
        diagrams: Vec::new(),
        plan_fingerprint: plan.fingerprint.clone(),
        language: plan.requested_language.clone(),
        generator: ExplanationGenerator {
            host: "authored-control".into(),
            session: "publication".into(),
            agent: None,
            model: None,
        },
        paragraphs: vec![WorkReaderUnit {
            question: if matches!(plan.subject, ExplanationSubject::WorkName(_)) {
                ExplanationQuestion::Heading
            } else {
                ExplanationQuestion::Synopsis
            },
            text: text.into(),
            evidence_keys: plan.evidence.iter().map(|e| e.key.clone()).collect(),
            sources: vec![],
        }],
    }
}
fn record(
    f: &reading_fixture::Fixture,
    subject: ExplanationSubject,
    language: &str,
    text: &str,
) -> Result<RetainedExplanation, Box<dyn Error>> {
    let plan = f
        .operations
        .prepare_explanation(f.project, subject, language)?;
    let response =
        f.operations
            .compose_subject_explanation(f.project, subject, draft(&plan, text))?;
    Ok(f.operations
        .record_explanation(f.project, subject, language, response)?)
}
fn work(
    f: &reading_fixture::Fixture,
    id: ContextItemId,
) -> Result<UnderstandingWork, Box<dyn Error>> {
    Ok(f.operations
        .project_projection_selected(f.project, WorkSelector::exact(&id.to_string())?)?
        .selected_work
        .ok_or("selected Work")?)
}

#[test]
fn admitted_record_identity_survives_deleted_technical_source_and_article_replacement(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = f.goals["older"];
    let name = ExplanationSubject::WorkName(id);
    let summary = ExplanationSubject::WorkSummary(id);
    let title = "Inspect query value preservation";
    let synopsis =
        "The investigation reports that readQuery returns its input unchanged; no patch was made.";
    record(&f, name, "en", title)?;
    record(&f, summary, "en", synopsis)?;
    let analysis = f
        .operations
        .analyze(f.project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let observed = f.operations.observe_work_file(FileObservationRequest {
        project_id: f.project,
        work_item_id: id,
        analysis_snapshot_id: analysis,
        path: "runtime/query_boundary.ts".into(),
        byte_range: None,
        before_source_id: None,
        retain_seconds: 3600,
    })?;
    let plan = f
        .operations
        .prepare_explanation(f.project, ExplanationSubject::Work(id), "en")?;
    let mut article = draft(&plan, "Query boundary investigation");
    article.paragraphs[0].question = ExplanationQuestion::Heading;
    article.paragraphs.push(WorkReaderUnit {
        question: ExplanationQuestion::Synopsis,
        text: "A bounded source observation supports the technical reading.".into(),
        evidence_keys: vec!["goal".into()],
        sources: vec![],
    });
    let bytes = std::fs::read_to_string(f.repository.join("runtime/query_boundary.ts"))?;
    article.paragraphs.push(WorkReaderUnit {
        question: ExplanationQuestion::Observation,
        text: "The observed TypeScript boundary declares the query implementation.".into(),
        evidence_keys: vec!["goal".into()],
        sources: vec![WorkClaimRange {
            source_id: observed.source_id.to_string(),
            field: "body".into(),
            use_kind: ExplanationSourceUse::CurrentCode,
            start_byte: 0,
            end_byte: bytes.len(),
            claim: None,
        }],
    });
    let realization = f
        .operations
        .compose_work_explanation(f.project, id, article)?;
    f.operations
        .record_explanation(f.project, ExplanationSubject::Work(id), "en", realization)?;
    let before = work(&f, id)?;
    assert_eq!(
        before.reading.explanations[0].state,
        ExplanationState::Current
    );
    f.operations
        .delete_work_source_observation(f.project, id, observed.source_id)?;
    let degraded = work(&f, id)?;
    assert_eq!(
        degraded.reading.explanations[0].state,
        ExplanationState::Stale
    );
    assert!(degraded.reading.explanations[0].content.is_none());
    assert_eq!(
        degraded.reading.name_explanations[0]
            .content
            .as_ref()
            .ok_or("name")?
            .realization
            .paragraphs[0]
            .text,
        title
    );
    assert_eq!(
        degraded.reading.summary_explanations[0]
            .content
            .as_ref()
            .ok_or("summary")?
            .realization
            .paragraphs[0]
            .text,
        synopsis
    );
    assert_eq!(before.creation_event, degraded.creation_event);
    assert_eq!(
        before.latest_work_record_event,
        degraded.latest_work_record_event
    );
    assert_eq!(
        before.reading.verification_report,
        degraded.reading.verification_report
    );
    f.operations
        .delete_explanations(f.project, ExplanationSubject::Work(id))?;
    assert_eq!(
        work(&f, id)?.reading.name_explanations[0].state,
        ExplanationState::Current
    );
    // Replacing a name replaces only that scope and never silently revives history.
    record(&f, name, "en", "Inspect the query boundary")?;
    let replaced = work(&f, id)?;
    assert_eq!(
        replaced.reading.name_explanations[0]
            .content
            .as_ref()
            .ok_or("name")?
            .realization
            .paragraphs[0]
            .text,
        "Inspect the query boundary"
    );
    assert_eq!(
        replaced.reading.summary_explanations[0],
        degraded.reading.summary_explanations[0]
    );
    Ok(())
}

#[test]
fn goal_correction_invalidates_each_scope_and_failed_admission_preserves_prior_record(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = f.goals["goal_only"];
    let name = ExplanationSubject::WorkName(id);
    let summary = ExplanationSubject::WorkSummary(id);
    record(&f, name, "ko", "쿼리 경계 조사")?;
    record(
        &f,
        summary,
        "ko",
        "쿼리 경계 조사를 요청했으며 결과는 아직 기록되지 않았습니다.",
    )?;
    let before = work(&f, id)?;
    let plan = f.operations.prepare_explanation(f.project, name, "ko")?;
    assert_eq!(plan.evidence.len(), 1);
    assert_eq!(plan.evidence[0].key, "goal");
    let valid = f.operations.compose_subject_explanation(
        f.project,
        name,
        draft(&plan, "쿼리 경계 조사"),
    )?;
    for bad in [
        "old-format",
        "language",
        "foreign-plan",
        "oversized",
        "wrong-role",
    ] {
        let mut response = valid.clone();
        match bad {
            "old-format" => response.format_version = 5,
            "language" => response.language = "en".into(),
            "foreign-plan" => response.plan_fingerprint = "sha256:foreign".into(),
            "oversized" => response.paragraphs[0].text = "x".repeat(32769),
            _ => response.paragraphs[0].question = ExplanationQuestion::Synopsis,
        }
        assert!(
            f.operations
                .record_explanation(f.project, name, "ko", response)
                .is_err(),
            "{bad}"
        );
        assert_eq!(
            work(&f, id)?.reading.name_explanations,
            before.reading.name_explanations
        );
    }
    let canonical = f.operations.canonical_basis(f.project)?;
    let goal = canonical
        .context_items
        .iter()
        .find(|c| c.id == id)
        .ok_or("Goal")?;
    f.operations.correct_context_item(
        f.project,
        id,
        ContextItemCorrectionDraft {
            expected_revision: goal.revision,
            corrected_statement: format!("{}.", goal.statement),
            user_authorization_source_id: goal.source_basis[0],
            kind: CorrectionKind::Expression,
        },
    )?;
    let stale = work(&f, id)?;
    assert_eq!(
        stale.reading.name_explanations[0].state,
        ExplanationState::Stale
    );
    assert_eq!(
        stale.reading.summary_explanations[0].state,
        ExplanationState::Stale
    );
    assert!(f
        .operations
        .record_explanation(f.project, name, "ko", valid)
        .is_err());
    Ok(())
}

#[test]
fn new_report_stales_summary_but_not_goal_name_and_forgetting_cleans_only_linked_scopes(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = f.goals["goal_only"];
    let name = ExplanationSubject::WorkName(id);
    let summary = ExplanationSubject::WorkSummary(id);
    record(&f, name, "en", "Investigate the relay")?;
    record(
        &f,
        summary,
        "en",
        "Relay investigation requested; no result has been reported.",
    )?;
    let original = work(&f, id)?;
    let basis = f.operations.canonical_basis(f.project)?;
    let goal = basis
        .context_items
        .iter()
        .find(|c| c.id == id)
        .ok_or("Goal")?;
    let result_source = basis
        .sources
        .iter()
        .find(|s| matches!(s.source.payload, SourcePayload::RepositoryCommit { .. }))
        .ok_or("report Source")?
        .source
        .id;
    Store::open(f.operations.layout().canonical_store())?.record_checkpoint(
        OperationId::from_bytes([71; 16]),
        f.project,
        CheckpointDraft {
            expected_project_revision: basis.project.revision,
            work_item_id: Some(id),
            kind: CheckpointKind::Handoff,
            goal: goal.statement.clone(),
            work_state: WorkState::Paused,
            state_change: Some(
                "The relay was inspected without code changes; one boundary remains unresolved."
                    .into(),
            ),
            source_basis: vec![result_source],
            changed_source_basis: vec![],
            changed_paths: vec![],
            applied_decisions: vec![],
            verification: vec![],
            user_review: UserReviewFact {
                state: UserReviewState::NotRequested,
                source_id: None,
            },
            user_acceptance: UserAcceptanceFact {
                state: UserAcceptanceState::NotRequested,
                source_id: None,
            },
            known_limits: vec!["No automated assertions executed".into()],
            non_goals: vec![],
            open_questions: vec![],
            next_step: "Inspect the unresolved boundary".into(),
            handoff_to: Some("next-agent".into()),
        },
    )?;
    let after = work(&f, id)?;
    assert_eq!(
        after.reading.name_explanations,
        original.reading.name_explanations
    );
    assert_eq!(
        after.reading.summary_explanations[0].state,
        ExplanationState::Stale
    );
    record(&f,summary,"en","Relay inspection made no changes; one boundary remains unresolved and assertions were not run.")?;
    let current = work(&f, id)?;
    assert_eq!(
        current.reading.summary_explanations[0].state,
        ExplanationState::Current
    );
    let forgotten = f.operations.forget_record(
        f.project,
        CanonicalRecordId::Source(result_source),
        goal.source_basis[0],
    )?;
    assert_eq!(forgotten.state, ForgettingState::Completed);
    let after = work(&f, id)?;
    assert_eq!(
        after.reading.name_explanations,
        original.reading.name_explanations
    );
    assert_eq!(
        after.reading.summary_explanations[0].state,
        ExplanationState::Deleted
    );
    assert!(after.reading.summary_explanations[0].content.is_none());
    Ok(())
}

#[test]
fn genuinely_source_dependent_summary_expires_without_erasing_record_only_name(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = f.goals["goal_only"];
    let name = ExplanationSubject::WorkName(id);
    let summary = ExplanationSubject::WorkSummary(id);
    record(&f, name, "en", "Investigate the relay")?;
    let analysis = f
        .operations
        .analyze(f.project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let observed = f.operations.observe_work_file(FileObservationRequest {
        project_id: f.project,
        work_item_id: id,
        analysis_snapshot_id: analysis,
        path: "python/worker.py".into(),
        byte_range: None,
        before_source_id: None,
        retain_seconds: 1,
    })?;
    let plan = f.operations.prepare_explanation(f.project, summary, "en")?;
    let mut input = draft(&plan,"The requested relay investigation has a retained Python worker observation; no result is reported.");
    input.paragraphs[0].sources.push(WorkClaimRange {
        source_id: observed.source_id.to_string(),
        field: "body".into(),
        use_kind: ExplanationSourceUse::CurrentCode,
        start_byte: 0,
        end_byte: std::fs::read(f.repository.join("python/worker.py"))?.len(),
        claim: None,
    });
    let response = f
        .operations
        .compose_subject_explanation(f.project, summary, input)?;
    f.operations
        .record_explanation(f.project, summary, "en", response.clone())?;
    assert_eq!(
        work(&f, id)?.reading.summary_explanations[0].state,
        ExplanationState::Current
    );
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let expired = work(&f, id)?;
    assert_eq!(
        expired.reading.name_explanations[0].state,
        ExplanationState::Current
    );
    assert_eq!(
        expired.reading.summary_explanations[0].state,
        ExplanationState::Stale
    );
    assert!(expired.reading.summary_explanations[0].content.is_none());
    assert!(f
        .operations
        .record_explanation(f.project, summary, "en", response)
        .is_err());
    Ok(())
}

#[test]
fn same_goal_different_works_restart_and_source_independent_import_keep_exact_scope(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let a = f.goals["older"];
    let b = f.goals["same_title"];
    let basis = f.operations.canonical_basis(f.project)?;
    assert_eq!(
        basis
            .context_items
            .iter()
            .find(|c| c.id == a)
            .ok_or("A")?
            .statement,
        basis
            .context_items
            .iter()
            .find(|c| c.id == b)
            .ok_or("B")?
            .statement
    );
    let plan =
        f.operations
            .prepare_explanation(f.project, ExplanationSubject::WorkName(a), "en")?;
    let response = f.operations.compose_subject_explanation(
        f.project,
        plan.subject,
        draft(&plan, "Inspect query value preservation"),
    )?;
    assert!(f
        .operations
        .record_explanation(
            f.project,
            ExplanationSubject::WorkName(b),
            "en",
            response.clone()
        )
        .is_err());
    f.operations
        .record_explanation(f.project, plan.subject, "en", response)?;
    record(
        &f,
        ExplanationSubject::WorkName(b),
        "ko",
        "쿼리 입력값 보존 조사",
    )?;
    let restarted = LocalOperations::new(f.operations.layout().clone());
    let projection =
        restarted.project_projection_selected(f.project, WorkSelector::ExactWork(a))?;
    let selected = projection.selected_work.ok_or("A")?;
    let answers = work_answers(&selected, "en", FixedLocale::English);
    assert_eq!(
        answers.card_name(),
        Some("Inspect query value preservation")
    );
    assert!(work_answers(&selected, "ko", FixedLocale::Korean)
        .card_name()
        .is_none());
    assert!(work_answers(&work(&f, b)?, "en", FixedLocale::English)
        .card_name()
        .is_none());
    let bundle = f._temporary.path().join("portable.json");
    f.operations.export_bundle(f.project, &bundle)?;
    let imported = LocalOperations::new(RuntimeLayout::new(f._temporary.path().join("imported"))?);
    imported.import_bundle(&bundle)?;
    let selected = imported
        .project_projection_selected(f.project, WorkSelector::ExactWork(a))?
        .selected_work
        .ok_or("imported A")?;
    assert_eq!(selected.creation_event, work(&f, a)?.creation_event);
    assert_eq!(
        selected.latest_work_record_event,
        work(&f, a)?.latest_work_record_event
    );
    assert!(
        selected.reading.name_explanations.is_empty(),
        "Derived publications are not portable canonical truth"
    );
    let fresh = imported.prepare_explanation(f.project, ExplanationSubject::WorkName(a), "en")?;
    let response = imported.compose_subject_explanation(
        f.project,
        fresh.subject,
        draft(&fresh, "Inspect the query boundary"),
    )?;
    imported.record_explanation(f.project, fresh.subject, "en", response)?;
    assert_eq!(
        imported
            .project_projection_selected(f.project, WorkSelector::ExactWork(a))?
            .selected_work
            .ok_or("imported A")?
            .reading
            .name_explanations[0]
            .state,
        ExplanationState::Current
    );
    Ok(())
}

#[test]
fn out_of_order_report_changes_article_plan_without_replacing_summary_result(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = f.goals["older"];
    record(
        &f,
        ExplanationSubject::WorkName(id),
        "en",
        "Inspect query value preservation",
    )?;
    record(
        &f,
        ExplanationSubject::WorkSummary(id),
        "en",
        "Inspection reports unchanged input and no patch.",
    )?;
    let plan = f
        .operations
        .prepare_explanation(f.project, ExplanationSubject::Work(id), "en")?;
    let response = ExplanationRealization {
        diagrams: Vec::new(),
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint.clone(),
        language: "en".into(),
        generator: ExplanationGenerator {
            host: "authored-control".into(),
            session: "out-of-order-report".into(),
            agent: None,
            model: None,
        },
        paragraphs: vec![ExplanationParagraph {
            question: ExplanationQuestion::Heading,
            text: "Query boundary investigation".into(),
            evidence_keys: vec!["goal".into()],
            citations: vec![],
        }],
    };
    f.operations
        .record_explanation(f.project, plan.subject, "en", response)?;
    let before = work(&f, id)?;
    let basis = f.operations.canonical_basis(f.project)?;
    let cp = basis
        .checkpoint_history
        .iter()
        .chain(basis.latest_checkpoint.iter())
        .filter(|c| c.work_item_id == Some(id))
        .max_by_key(|c| (c.recorded_at, c.id))
        .ok_or("Checkpoint")?;
    let mut store = Store::open_with(
        f.operations.layout().canonical_store(),
        DeterministicIdGenerator::new([[96; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(
            cp.recorded_at.as_unix_micros() - 1,
        )),
    )?;
    store.record_checkpoint(
        OperationId::from_bytes([97; 16]),
        f.project,
        CheckpointDraft {
            expected_project_revision: basis.project.revision,
            work_item_id: Some(id),
            kind: cp.kind,
            goal: cp.goal.clone(),
            work_state: cp.work_state,
            state_change: Some("Earlier inspection remained incomplete.".into()),
            source_basis: cp.source_basis.clone(),
            changed_source_basis: vec![],
            changed_paths: vec![],
            applied_decisions: vec![],
            verification: vec![],
            user_review: cp.user_review.clone(),
            user_acceptance: cp.user_acceptance.clone(),
            known_limits: vec!["Earlier observation only".into()],
            non_goals: vec![],
            open_questions: vec![],
            next_step: cp.next_step.clone(),
            handoff_to: cp.handoff_to.clone(),
        },
    )?;
    drop(store);
    let after = work(&f, id)?;
    assert_eq!(
        after.reading.name_explanations,
        before.reading.name_explanations
    );
    assert_eq!(
        after.reading.summary_explanations,
        before.reading.summary_explanations
    );
    assert_eq!(
        after.latest_work_record_event,
        before.latest_work_record_event
    );
    assert_eq!(after.reading.explanations[0].state, ExplanationState::Stale);
    assert!(after.reading.explanations[0]
        .diagnostic
        .as_deref()
        .is_some_and(|d| d.contains("evidence changed")));
    Ok(())
}

#[test]
fn corrupt_foreign_old_and_changed_plan_name_records_never_revive_prior_text(
) -> Result<(), Box<dyn Error>> {
    use volicord_privacy::{ManagedDerivedDraft, ManagedDerivedKind, PrivacyStore};
    let f = fixture()?;
    let id = f.goals["older"];
    let name = ExplanationSubject::WorkName(id);
    record(
        &f,
        ExplanationSubject::WorkSummary(id),
        "en",
        "The recorded inspection reports an unchanged return value and no patch.",
    )?;
    for case in ["old", "corrupt", "foreign", "changed-plan", "overbound"] {
        let retained = record(&f, name, "en", "Inspect query value preservation")?;
        let summary = work(&f, id)?.reading.summary_explanations;
        let mut privacy = PrivacyStore::open(f.operations.layout().privacy_store())?;
        let previous = privacy
            .inspect_project(f.project)?
            .managed_derived
            .into_iter()
            .rev()
            .find(|r| r.purpose == format!("explanation:work_name:{id}:en"))
            .ok_or("name")?;
        let mut forged = retained.clone();
        forged.realization.paragraphs[0].text = "FOREIGN_UNSUPPORTED_CARD_SENTINEL".into();
        let content = match case {
            "old" => {
                forged.realization.format_version = 5;
                serde_json::to_string(&forged)?
            }
            "corrupt" => "{".into(),
            "foreign" => {
                forged.subject = ExplanationSubject::WorkName(f.goals["same_title"]);
                serde_json::to_string(&forged)?
            }
            "changed-plan" => {
                forged.realization.plan_fingerprint = "sha256:obsolete-plan".into();
                serde_json::to_string(&forged)?
            }
            _ => "x".repeat(EXPLANATION_RETAINED_BYTE_LIMIT + 1),
        };
        let result = privacy.record_managed_derived(ManagedDerivedDraft {
            project_id: f.project,
            kind: ManagedDerivedKind::CachedSummary,
            provider: None,
            model: None,
            purpose: previous.purpose,
            analysis_snapshot: None,
            included_sources: previous.included_sources,
            canonical_links: previous.canonical_links,
            content,
            uncertainty: None,
            retained_until: None,
            retention_basis: "Authored adversarial managed-content control".into(),
        });
        if case == "overbound" {
            assert!(result.is_err());
        } else {
            result?;
        }
        drop(privacy);
        let selected = work(&f, id)?;
        assert_eq!(selected.reading.summary_explanations, summary);
        let answers = work_answers(&selected, "en", FixedLocale::English);
        if case == "overbound" {
            assert_eq!(
                answers.card_name(),
                Some("Inspect query value preservation")
            );
        } else {
            assert!(answers.card_name().is_none(), "{case}");
            assert!(!answers.text().contains("FOREIGN_UNSUPPORTED_CARD_SENTINEL"));
            assert_eq!(
                selected.reading.name_explanations[0].state,
                match case {
                    "old" => ExplanationState::Unsupported,
                    "corrupt" => ExplanationState::Corrupt,
                    _ => ExplanationState::Stale,
                }
            );
        }
    }
    Ok(())
}

#[test]
fn goal_forgetting_deletes_every_linked_scope_and_language_without_erasing_other_work(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = f.goals["goal_only"];
    let other = f.goals["older"];
    record(
        &f,
        ExplanationSubject::WorkName(id),
        "en",
        "Inspect the relay boundary",
    )?;
    record(
        &f,
        ExplanationSubject::WorkName(id),
        "ko",
        "릴레이 경계 조사",
    )?;
    record(
        &f,
        ExplanationSubject::WorkSummary(id),
        "en",
        "Relay investigation was requested; no outcome is reported.",
    )?;
    record(
        &f,
        ExplanationSubject::WorkName(other),
        "en",
        "Investigate the query boundary",
    )?;
    let basis = f.operations.canonical_basis(f.project)?;
    let authorization = basis
        .context_items
        .iter()
        .find(|g| g.id == id)
        .ok_or("Goal")?
        .source_basis[0];
    let forgotten =
        f.operations
            .forget_record(f.project, CanonicalRecordId::ContextItem(id), authorization)?;
    assert_eq!(forgotten.state, ForgettingState::Completed);
    assert!(work(&f, id).is_err());
    let records = f.operations.privacy_status(f.project)?;
    let affected = records
        .managed_derived
        .iter()
        .filter(|r| r.purpose.contains(&id.to_string()))
        .collect::<Vec<_>>();
    assert_eq!(affected.len(), 3);
    assert!(affected
        .iter()
        .all(|r| r.content.is_none() && r.state == volicord_privacy::ManagedDerivedState::Deleted));
    assert_eq!(
        work(&f, other)?.reading.name_explanations[0].state,
        ExplanationState::Current
    );
    Ok(())
}

#[test]
fn blank_reports_select_current_progress_and_invalidate_each_material_qualification(
) -> Result<(), Box<dyn Error>> {
    use serde_json::json;
    let f = fixture()?;
    let id = f.goals["goal_only"];
    let summary = ExplanationSubject::WorkSummary(id);
    record(
        &f,
        ExplanationSubject::WorkName(id),
        "en",
        "Investigate relay progress",
    )?;
    record(
        &f,
        summary,
        "en",
        "Relay work is requested; no result is reported.",
    )?;
    record(
        &f,
        summary,
        "ko",
        "릴레이 작업을 요청했으며 결과는 아직 보고되지 않았습니다.",
    )?;
    let original_name = work(&f, id)?.reading.name_explanations;
    let basis = f.operations.canonical_basis(f.project)?;
    let goal = basis
        .context_items
        .iter()
        .find(|c| c.id == id)
        .ok_or("Goal")?;
    let user = goal.source_basis[0];
    let passed = basis
        .sources
        .iter()
        .find(|s| {
            matches!(&s.source.payload,
        SourcePayload::CommandExecution { outcome, .. } if outcome.exit_code == Some(0))
        })
        .ok_or("pass Source")?
        .source
        .id;
    let failed = basis.sources.iter().find(|s| matches!(&s.source.payload,
        SourcePayload::CommandExecution { outcome, .. } if outcome.exit_code.is_some_and(|code| code != 0))).ok_or("failure Source")?.source.id;
    let mut cp = CheckpointDraft {
        expected_project_revision: basis.project.revision,
        work_item_id: Some(id),
        kind: CheckpointKind::Handoff,
        goal: goal.statement.clone(),
        work_state: WorkState::InProgress,
        state_change: None,
        source_basis: vec![user],
        changed_source_basis: vec![],
        changed_paths: vec![],
        applied_decisions: vec![],
        verification: vec![VerificationFact {
            scoped_run: None,
            state: VerificationState::Passed,
            source_id: Some(passed),
            outcome: Some("Authored passing control".into()),
        }],
        user_review: UserReviewFact {
            state: UserReviewState::Reviewed,
            source_id: Some(user),
        },
        user_acceptance: UserAcceptanceFact {
            state: UserAcceptanceState::Accepted,
            source_id: Some(user),
        },
        known_limits: vec![],
        non_goals: vec![],
        open_questions: vec![],
        next_step: "Investigate the relay boundary".into(),
        handoff_to: Some("next-agent".into()),
    };
    for (index, case) in [
        "initial-progress",
        "blank-path-change",
        "failed-verification",
        "paused",
        "acceptance-rejected",
        "review-reset",
        "acceptance-reset",
        "limit",
        "new-direction",
    ]
    .iter()
    .enumerate()
    {
        match *case {
            "blank-path-change" => {
                cp.changed_paths = vec!["python/worker.py".into()];
                cp.changed_source_basis = vec![user];
                cp.verification.clear();
            }
            "failed-verification" => {
                cp.changed_paths.clear();
                cp.changed_source_basis.clear();
                cp.verification = vec![VerificationFact {
                    scoped_run: None,
                    state: VerificationState::Failed,
                    source_id: Some(failed),
                    outcome: Some("Authored failing control".into()),
                }];
            }
            "paused" => cp.work_state = WorkState::Paused,
            "acceptance-rejected" => cp.user_acceptance.state = UserAcceptanceState::Rejected,
            "review-reset" => {
                cp.user_review = UserReviewFact {
                    state: UserReviewState::NotRequested,
                    source_id: None,
                }
            }
            "acceptance-reset" => {
                cp.user_acceptance = UserAcceptanceFact {
                    state: UserAcceptanceState::NotRequested,
                    source_id: None,
                }
            }
            "limit" => cp.known_limits = vec!["Relay cancellation is still uninvestigated".into()],
            "new-direction" => cp.next_step = "Investigate cancellation propagation next".into(),
            _ => (),
        }
        cp.expected_project_revision = f.operations.canonical_basis(f.project)?.project.revision;
        Store::open(f.operations.layout().canonical_store())?.record_checkpoint(
            OperationId::from_bytes((5000_u128 + index as u128).to_be_bytes()),
            f.project,
            cp.clone(),
        )?;
        let stale = work(&f, id)?;
        assert_eq!(stale.reading.name_explanations, original_name, "{case}");
        assert!(
            stale
                .reading
                .summary_explanations
                .iter()
                .all(|r| r.state == ExplanationState::Stale),
            "{case}"
        );
        let plan = f.operations.prepare_explanation(f.project, summary, "en")?;
        assert!(
            !plan.evidence.iter().any(|e| e.key == "result"),
            "blank report must stay missing"
        );
        assert!(plan.evidence.len() <= 8, "no full history");
        let field = |key: &str| plan.evidence.iter().find(|e| e.key == key).ok_or("field");
        assert_eq!(field("latest_state")?.field, "work_state");
        assert_eq!(field("review")?.field, "user_review");
        assert_eq!(field("acceptance")?.field, "user_acceptance");
        assert_eq!(
            field("verification")?.sources,
            vec![
                if *case == "initial-progress" || *case == "blank-path-change" {
                    passed.to_string()
                } else {
                    failed.to_string()
                }
            ]
        );
        if *case == "blank-path-change" {
            assert_eq!(
                field("verification")?.content["coverage"],
                "historical_after_reported_change"
            );
            assert_eq!(
                field("verification_coverage")?.content["reported_change"],
                json!(null)
            );
            assert_eq!(
                field("verification_coverage")?.content["paths"],
                json!(["python/worker.py"])
            );
        }
        if *case == "review-reset" {
            assert_eq!(field("review")?.content["state"], "NotRequested");
        }
        if *case == "acceptance-reset" {
            assert_eq!(field("acceptance")?.content["state"], "NotRequested");
        }
        if *case == "new-direction" {
            assert_eq!(
                field("next_step")?.content,
                "Investigate cancellation propagation next"
            );
        }
        let mut incomplete = draft(&plan, "An incompatible Goal-only progress claim");
        incomplete.paragraphs[0].evidence_keys = vec!["goal".into()];
        assert!(f
            .operations
            .compose_subject_explanation(f.project, summary, incomplete)
            .is_err());
        let prose = match *case {
            "initial-progress" => "Relay investigation is in progress without a reported result; a check passed and review and acceptance were recorded, which do not establish completion.",
            "blank-path-change" => "Relay investigation is in progress without a result report. A worker change follows the historical passing check; current verification coverage is unknown.",
            "failed-verification" => "Relay investigation has no reported result and its latest check failed; the work remains in progress.",
            "paused" => "Relay investigation is paused without a reported result; its latest check failed.",
            "acceptance-rejected" => "Relay investigation is paused without a reported result, with failed verification and rejected acceptance.",
            "review-reset" => "Relay investigation is paused with failed verification and rejected acceptance; review has reset to not requested. No result is reported.",
            "acceptance-reset" => "Relay investigation is paused with failed verification. Review and acceptance are not requested; no result is reported.",
            "limit" => "Relay investigation is paused with failed verification and no result report. Cancellation remains uninvestigated; review and acceptance are not requested.",
            _ => "Relay investigation is paused with failed verification and no reported result. Cancellation remains uninvestigated, review and acceptance are not requested, and cancellation propagation is the next investigation.",
        };
        record(&f, summary, "en", prose)?;
        let current = work(&f, id)?;
        assert_eq!(
            work_answers(&current, "en", FixedLocale::English)
                .card_summary()
                .map(|s| s.text.as_str()),
            Some(prose)
        );
        assert_eq!(
            current
                .reading
                .summary_explanations
                .iter()
                .find(|r| r.language == "ko")
                .ok_or("ko")?
                .state,
            ExplanationState::Stale
        );
        record(
            &f,
            summary,
            "ko",
            "진행 근거를 갱신한 기술 검증용 요약입니다. 실제 사용자 평가가 아닙니다.",
        )?;
    }
    // The selected verification Source has normal canonical forgetting consequences.
    f.operations
        .forget_record(f.project, CanonicalRecordId::Source(failed), user)?;
    assert!(work(&f, id)?
        .reading
        .summary_explanations
        .iter()
        .all(|r| r.state == ExplanationState::Deleted));
    assert_eq!(work(&f, id)?.reading.name_explanations, original_name);
    Ok(())
}
