//! Independently authored lifecycle controls; no model or human-success claim.
#[path = "support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use std::error::Error;
use volicord_context::*;
use volicord_operations::*;
use volicord_projections::*;

fn draft(plan: &ExplanationPlan, text: &str) -> WorkExplanationDraft {
    WorkExplanationDraft {
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
    let f = reading_fixture::fixture()?;
    let id = f.goals["older"];
    let name = ExplanationSubject::WorkName(id);
    let summary = ExplanationSubject::WorkSummary(id);
    let title = "Keep the latest query response";
    let synopsis = "The query race was investigated; the recorded result is distinct from verification and acceptance.";
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
    record(&f, name, "en", "Investigate query ordering")?;
    let replaced = work(&f, id)?;
    assert_eq!(
        replaced.reading.name_explanations[0]
            .content
            .as_ref()
            .ok_or("name")?
            .realization
            .paragraphs[0]
            .text,
        "Investigate query ordering"
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
    let f = reading_fixture::fixture()?;
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
    let f = reading_fixture::fixture()?;
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
    let f = reading_fixture::fixture()?;
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
