//! Exact review lifecycle; these protocol fixtures never certify semantic quality.
#[path = "support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use std::error::Error;
use volicord_operations::*;
use volicord_projections::*;

fn fixture() -> Result<reading_fixture::Fixture, Box<dyn Error>> {
    let mut input: serde_json::Value = serde_json::from_str(reading_fixture::SCENARIO)?;
    input["prior_checkpoint_count"] = 0.into();
    input["later_checkpoint_count"] = 0.into();
    input["unassociated_checkpoint"] = false.into();
    for work in input["works"].as_array_mut().ok_or("works")? {
        if work["key"] == "older" {
            work["checkpoints"] = serde_json::json!([{"key":"inspection","work":"Paused","verification":"NotRun","state_change":"Inspection found readQuery returns its input unchanged. No patch was made.","next_step":"Inspect the actual caller"}]);
        }
    }
    reading_fixture::fixture_scenario(input)
}

fn realization(
    f: &reading_fixture::Fixture,
    subject: ExplanationSubject,
    plan: &ExplanationPlan,
    extra: &str,
) -> Result<ExplanationRealization, Box<dyn Error>> {
    let body = if plan.requested_language == "ko" {
        "readQuery 조사에서 입력값을 그대로 반환함을 확인했고 코드를 바꾸지 않았다. 작업은 일시 중지이며 다음에는 실제 호출자를 살펴본다. 정적 조사로 성능이나 사용자 수락을 입증하지 않는다."
    } else {
        "Inspection found readQuery returns its input unchanged; no patch was made. Work is paused; inspect the actual caller next. Static inspection proves neither performance nor user acceptance."
    };
    Ok(f.operations.compose_subject_explanation(
        f.project,
        subject,
        WorkExplanationDraft {
            diagrams: vec![],
            plan_fingerprint: plan.fingerprint.clone(),
            language: plan.requested_language.clone(),
            generator: ExplanationGenerator {
                host: "controlled-host".into(),
                session: "reader-review-protocol".into(),
                agent: None,
                model: None,
            },
            paragraphs: vec![
                WorkReaderUnit {
                    question: ExplanationQuestion::Heading,
                    text: "Inspect value preservation".into(),
                    evidence_keys: vec!["goal".into()],
                    sources: vec![],
                },
                WorkReaderUnit {
                    question: ExplanationQuestion::Synopsis,
                    text: format!("{body}{extra}"),
                    evidence_keys: plan.evidence.iter().map(|e| e.key.clone()).collect(),
                    sources: vec![],
                },
            ],
        },
    )?)
}

fn review(realization: ExplanationRealization) -> ExplanationReaderReview {
    let checks = [
        ("problem_change_result", ReaderReviewOutcome::Addressed, "The static inspection reports unchanged behavior and no patch."),
        ("use_and_effect", ReaderReviewOutcome::NotApplicable, "This bounded no-change investigation introduces no command or changed user-visible API to exemplify."),
        ("state_and_next_action", ReaderReviewOutcome::Addressed, "The paused investigation names caller inspection next."),
        ("related_decisions", ReaderReviewOutcome::NotApplicable, "The controlled no-change text claims no new technical choice or user rationale."),
        ("grounding_and_limits", ReaderReviewOutcome::Addressed, "The summary binds actual report facts and makes no performance or acceptance claim."),
        ("readable_structure", ReaderReviewOutcome::Addressed, "A heading and one connected paragraph suffice for this controlled inspection; no table quota."),
        ("language_review", ReaderReviewOutcome::Addressed, "This exact language's complete units were checked as a protocol fixture, not a Human verdict."),
    ].into_iter().map(|(obligation,outcome,rationale)|ReaderReviewCheck { obligation:obligation.into(),outcome,rationale:rationale.into(),unit_indices:if obligation=="language_review"{(0..realization.paragraphs.len()).collect()}else{vec![1]} }).collect();
    ExplanationReaderReview {
        realization,
        checks,
    }
}

fn begin(
    f: &reading_fixture::Fixture,
    subject: ExplanationSubject,
    language: &str,
) -> Result<(String, ExplanationPlan), Box<dyn Error>> {
    let started = f.operations.begin_explanation_production(
        f.project,
        subject,
        language,
        ExplanationProductionReason::Refinement,
    )?;
    let id = started["production_id"]
        .as_str()
        .ok_or("production")?
        .to_owned();
    f.operations.assess_explanation_diagrams(f.project,subject,language,&id,ExplanationDiagramAssessment { outcome:DiagramAssessmentOutcome::NotNeeded,rationale:"This small static no-change inspection has no separately evidenced relationship needing a diagram.".into(),source_ids:vec![] })?;
    Ok((id, serde_json::from_value(started["plan"].clone())?))
}

#[test]
fn diagram_only_completion_cannot_replace_reader_review_and_short_no_change_is_valid(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let subject = ExplanationSubject::Work(f.goals["older"]);
    let legacy_plan = f.operations.prepare_explanation(f.project, subject, "en")?;
    let legacy = realization(&f, subject, &legacy_plan, "")?;
    f.operations
        .record_explanation(f.project, subject, "en", legacy)?;
    let retained = f.operations.privacy_status(f.project)?;
    let unreviewed = f
        .operations
        .explanation_production_status(f.project, subject, "en")?;
    assert_eq!(unreviewed["state"], "current");
    assert_eq!(unreviewed["content_complete"], false);
    assert_eq!(unreviewed["reader_review_required"], true);
    assert_eq!(
        retained,
        f.operations.privacy_status(f.project)?,
        "legacy readability never fabricates a review or renews retention"
    );
    let (id, plan) = begin(&f, subject, "en")?;
    let response = realization(&f, subject, &plan, "")?;
    let before = f.operations.privacy_status(f.project)?;
    assert!(f
        .operations
        .record_explanation_for_production(f.project, subject, "en", response.clone(), Some(&id))
        .is_err());
    assert_eq!(
        before,
        f.operations.privacy_status(f.project)?,
        "missing review refuses without mutation"
    );
    f.operations.assess_explanation_reader(
        f.project,
        subject,
        "en",
        &id,
        review(response.clone()),
    )?;
    f.operations.record_explanation_for_production(
        f.project,
        subject,
        "en",
        response,
        Some(&id),
    )?;
    let status = f
        .operations
        .explanation_production_status(f.project, subject, "en")?;
    assert_eq!(status["content_complete"], true);
    assert_eq!(status["reader_review_applies_to_publication"], true);
    assert_eq!(
        status["semantic_quality_verified"], false,
        "valid host declarations do not prove semantic quality"
    );
    let retained = f.operations.privacy_status(f.project)?;
    let skip = f.operations.begin_explanation_production(
        f.project,
        subject,
        "en",
        ExplanationProductionReason::Boundary,
    )?;
    assert_eq!(skip["generation_status"], "skipped_unchanged_basis");
    assert_eq!(
        retained,
        f.operations.privacy_status(f.project)?,
        "status/skip renew neither Source nor publication"
    );
    Ok(())
}

#[test]
fn changed_realization_language_subject_or_production_cannot_borrow_a_review(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let subject = ExplanationSubject::Work(f.goals["older"]);
    let (id, plan) = begin(&f, subject, "en")?;
    let response = realization(&f, subject, &plan, "")?;
    f.operations.assess_explanation_reader(
        f.project,
        subject,
        "en",
        &id,
        review(response.clone()),
    )?;
    let changed = realization(
        &f,
        subject,
        &plan,
        " Changed wording still needs an exact review.",
    )?;
    assert!(f
        .operations
        .record_explanation_for_production(f.project, subject, "en", changed.clone(), Some(&id))
        .is_err());
    let before = f.operations.privacy_status(f.project)?;
    assert!(f
        .operations
        .assess_explanation_reader(f.project, subject, "ko", &id, review(response.clone()))
        .is_err());
    assert!(f
        .operations
        .assess_explanation_reader(
            f.project,
            ExplanationSubject::Work(f.goals["same_title"]),
            "en",
            &id,
            review(response.clone())
        )
        .is_err());
    assert_eq!(before, f.operations.privacy_status(f.project)?);
    let (next, other_plan) = begin(&f, subject, "en")?;
    let other = realization(&f, subject, &other_plan, "")?;
    assert!(f
        .operations
        .record_explanation_for_production(f.project, subject, "en", other.clone(), Some(&next))
        .is_err());
    assert!(f
        .operations
        .assess_explanation_reader(f.project, subject, "en", &id, review(response))
        .is_err());
    f.operations.assess_explanation_reader(
        f.project,
        subject,
        "en",
        &next,
        review(other.clone()),
    )?;
    f.operations
        .record_explanation_for_production(f.project, subject, "en", other, Some(&next))?;
    let (_, ko_plan) = begin(&f, subject, "ko")?;
    assert_ne!(ko_plan.requested_language, other_plan.requested_language);
    assert_eq!(
        f.operations
            .explanation_production_status(f.project, subject, "ko")?["reader_review_required"],
        true
    );
    Ok(())
}

#[test]
fn incomplete_review_stays_readable_but_never_becomes_content_completion(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let subject = ExplanationSubject::Work(f.goals["older"]);
    let (id, plan) = begin(&f, subject, "en")?;
    let response = realization(&f, subject, &plan, "")?;
    let mut assessment = review(response.clone());
    assessment.checks[0].outcome = ReaderReviewOutcome::InsufficientEvidence;
    assessment.checks[0].rationale="The original before-state was not retained; do not convert missing evidence into a no-change assertion.".into();
    f.operations
        .assess_explanation_reader(f.project, subject, "en", &id, assessment)?;
    f.operations.record_explanation_for_production(
        f.project,
        subject,
        "en",
        response.clone(),
        Some(&id),
    )?;
    let status = f
        .operations
        .explanation_production_status(f.project, subject, "en")?;
    assert_eq!(status["state"], "complete");
    assert_eq!(status["content_complete"], false);
    assert_eq!(status["reader_review_required"], true);
    assert!(status["publication_id"].is_string());
    assert!(
        f.operations
            .assess_explanation_reader(f.project, subject, "en", &id, review(response))
            .is_err(),
        "completed token cannot retroactively certify an edition"
    );
    Ok(())
}

#[test]
fn missing_duplicate_unknown_or_invalid_unit_checks_are_rejected_without_write(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let subject = ExplanationSubject::Work(f.goals["older"]);
    let (id, plan) = begin(&f, subject, "en")?;
    let base = review(realization(&f, subject, &plan, "")?);
    for case in 0..8 {
        let mut bad = base.clone();
        match case {
            0 => {
                bad.checks.pop();
            }
            1 => {
                bad.checks[1].obligation = bad.checks[0].obligation.clone();
            }
            2 => {
                bad.checks[0].obligation = "contains-enough-words".into();
            }
            3 => {
                bad.checks[0].unit_indices = vec![2];
            }
            4 => {
                bad.checks[0].unit_indices.clear();
            }
            5 => {
                bad.checks[0].rationale = " ".into();
            }
            6 => bad.checks[6].unit_indices = vec![1],
            _ => bad.checks[4].outcome = ReaderReviewOutcome::NotApplicable,
        }
        let before = f.operations.privacy_status(f.project)?;
        assert!(
            f.operations
                .assess_explanation_reader(f.project, subject, "en", &id, bad)
                .is_err(),
            "case {case}"
        );
        assert_eq!(before, f.operations.privacy_status(f.project)?);
    }
    Ok(())
}

#[test]
fn explicit_cli_review_records_exact_draft_and_corrupt_metadata_recovers_with_fresh_production(
) -> Result<(), Box<dyn Error>> {
    use volicord_privacy::{ManagedDerivedDraft, ManagedDerivedKind, PrivacyStore};
    let f = fixture()?;
    let subject = ExplanationSubject::Work(f.goals["older"]);
    for corrupt in [
        "{",
        "{\"format_kind\":\"volicord_explanation_reader_review\",\"format_version\":0}",
        "{\"format_kind\":\"volicord_explanation_reader_review\",\"format_version\":2}",
    ] {
        let (id, plan) = begin(&f, subject, "en")?;
        let response = realization(&f, subject, &plan, "")?;
        let assessment = review(response.clone());
        let realization_path = f._temporary.path().join("realization.json");
        let checks_path = f._temporary.path().join("review.json");
        std::fs::write(&realization_path, serde_json::to_vec(&response)?)?;
        std::fs::write(
            &checks_path,
            serde_json::to_vec(&serde_json::json!({"checks":assessment.checks}))?,
        )?;
        let mut output = vec![];
        let mut errors = vec![];
        assert_eq!(
            run_cli(
                vec![
                    "--runtime".into(),
                    f.operations
                        .layout()
                        .root()
                        .to_str()
                        .ok_or("runtime")?
                        .into(),
                    "--project".into(),
                    f.project.to_string(),
                    "--json".into(),
                    "work".into(),
                    "explain".into(),
                    "production-review".into(),
                    "--work".into(),
                    f.goals["older"].to_string(),
                    "--language".into(),
                    "en".into(),
                    "--production-id".into(),
                    id.clone(),
                    "--realization".into(),
                    realization_path.to_str().ok_or("path")?.into(),
                    "--input".into(),
                    checks_path.to_str().ok_or("path")?.into()
                ],
                &mut output,
                &mut errors
            ),
            CliExit::SUCCESS,
            "{}",
            String::from_utf8_lossy(&errors)
        );
        let reviewed: serde_json::Value = serde_json::from_slice(&output)?;
        assert_eq!(reviewed["metadata_written"], true);
        let mut privacy = PrivacyStore::open(f.operations.layout().privacy_store())?;
        let previous = privacy
            .inspect_project(f.project)?
            .managed_derived
            .into_iter()
            .rev()
            .find(|r| r.purpose == format!("explanation-reader-review:{id}"))
            .ok_or("review")?;
        privacy.record_managed_derived(ManagedDerivedDraft {
            project_id: f.project,
            kind: ManagedDerivedKind::CachedSummary,
            provider: None,
            model: None,
            purpose: previous.purpose,
            analysis_snapshot: None,
            included_sources: previous.included_sources,
            canonical_links: previous.canonical_links,
            content: corrupt.into(),
            uncertainty: None,
            retained_until: None,
            retention_basis: "Adversarial local metadata test".into(),
        })?;
        drop(privacy);
        let before = f.operations.privacy_status(f.project)?;
        assert!(f
            .operations
            .record_explanation_for_production(f.project, subject, "en", response, Some(&id))
            .is_err());
        assert_eq!(before, f.operations.privacy_status(f.project)?);
        let (fresh, plan) = begin(&f, subject, "en")?;
        assert_ne!(id, fresh);
        let response = realization(&f, subject, &plan, "")?;
        f.operations.assess_explanation_reader(
            f.project,
            subject,
            "en",
            &fresh,
            review(response.clone()),
        )?;
        f.operations.record_explanation_for_production(
            f.project,
            subject,
            "en",
            response,
            Some(&fresh),
        )?;
        assert_eq!(
            f.operations
                .explanation_production_status(f.project, subject, "en")?["content_complete"],
            true
        );
    }
    Ok(())
}
