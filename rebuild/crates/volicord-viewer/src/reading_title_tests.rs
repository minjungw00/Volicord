//! Independently authored hostile title inputs; byte bindings are not entailment.
use super::*;
use volicord_projections::*;

const GOAL: &str = "Investigate the observed return value.";
const INVENTED_EFFECT: &str = "This change prevents every downstream failure.";

fn fixture() -> Result<(UnderstandingWork, ViewerRequest), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture()?;
    let id = f.goals["older"];
    let p = f
        .operations
        .project_projection_selected(f.project, WorkSelector::ExactWork(id))?;
    let mut w = p.selected_work.ok_or("selected Work")?;
    w.reading.goal.original_text = Some(GOAL.into());
    w.reading.goal.display_english = GOAL.into();
    w.reading.goal.display_korean = GOAL.into();
    w.reading.explanations.clear();
    Ok((
        w,
        ViewerRequest {
            project_id: f.project,
            locale: ViewerLocale::English,
            view: ViewerView::Work { work: Some(id) },
            requested_language: "en".into(),
            guarded_request: None,
        },
    ))
}

fn reading(w: &UnderstandingWork, r: &ViewerRequest, claim: &str) -> ExplanationReading {
    let code = "return 2";
    let citation = ExplanationCitation {
        source_id: "authored-current-source".into(),
        source_revision: 1,
        observation_fingerprint: "authored-observation".into(),
        analysis_snapshot_id: "authored-analysis".into(),
        repository_snapshot_id: "authored-repository".into(),
        repository_source_id: "authored-repository-source".into(),
        locator: "src/change.py".into(),
        start_line: 2,
        end_line: 2,
        field: "file".into(),
        start_byte: 21,
        end_byte: 29,
        range_sha256: explanation_text_hash(code),
        claim_start_byte: 0,
        claim_end_byte: claim.len(),
        claim_sha256: explanation_text_hash(claim),
        use_kind: ExplanationSourceUse::CurrentCode,
    };
    ExplanationReading {
        language: "en".into(),
        state: ExplanationState::Current,
        content: Some(RetainedExplanation {
            realization: ExplanationRealization {
                format_kind: EXPLANATION_KIND.into(),
                format_version: EXPLANATION_VERSION,
                plan_fingerprint: "authored-plan".into(),
                language: "en".into(),
                generator: ExplanationGenerator {
                    host: "authored-control".into(),
                    session: "local-title-control".into(),
                    agent: None,
                    model: None,
                },
                paragraphs: vec![ExplanationParagraph {
                    question: ExplanationQuestion::ReportedChange,
                    text: claim.into(),
                    evidence_keys: vec!["reported_change".into()],
                    citations: vec![citation.clone()],
                }],
            },
            project_id: r.project_id.to_string(),
            subject: ExplanationSubject::Work(w.work_item_id),
            question: "Explain the observed change.".into(),
            evidence: Vec::new(),
            source_status: Vec::new(),
            conflicts: Vec::new(),
            generated_at_unix_micros: 1,
            generator_identity_status: "self_reported_not_independently_verified".into(),
            semantic_approval_status: "not_assessed".into(),
        }),
        diagnostic: None,
        source_excerpts: vec![ExplanationSourceExcerpt {
            citation,
            work_revision: 1,
            observed_at_unix_micros: 1,
            text: code.into(),
            displayed_end_byte: 29,
            displayed_end_line: 2,
            omitted_utf8_bytes: 0,
            comparison: None,
            before_source_id: None,
            source_representation: Some("full_file".into()),
            observation_region: None,
            execution: None,
        }],
        omitted_source_excerpt_count: 0,
    }
}

fn assert_goal(w: &UnderstandingWork, r: &ViewerRequest) {
    let (title, basis, language) = work_reading_heading(w, r);
    assert_eq!(title, "Recorded Work");
    assert_eq!(basis, "neutral-label");
    assert_eq!(
        language, "en",
        "fixed label language is independent of Goal"
    );
    let mut html = String::new();
    work_goal_opening(&mut html, r, w);
    assert!(html.contains(GOAL));
    assert!(html.contains("lang=\"\" data-statement-role=\"verified-canonical\""));
}

#[test]
fn unavailable_and_stale_interpretations_cannot_supply_effect_headlines(
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut w, r) = fixture()?;
    for state in [
        ExplanationState::Stale,
        ExplanationState::Missing,
        ExplanationState::Deleted,
        ExplanationState::Withheld,
        ExplanationState::Unavailable,
        ExplanationState::Unsupported,
        ExplanationState::Corrupt,
    ] {
        let mut e = reading(&w, &r, INVENTED_EFFECT);
        e.state = state;
        w.reading.explanations = vec![e];
        assert_goal(&w, &r);
    }
    Ok(())
}

#[test]
fn no_current_code_citation_cannot_override_goal() -> Result<(), Box<dyn std::error::Error>> {
    let (mut w, r) = fixture()?;
    for kind in [
        ExplanationSourceUse::BeforeCode,
        ExplanationSourceUse::ObservedExecution,
    ] {
        let mut e = reading(&w, &r, INVENTED_EFFECT);
        let content = e.content.as_mut().ok_or("retained content")?;
        content.realization.paragraphs[0].citations[0].use_kind = kind;
        w.reading.explanations = vec![e];
        assert_goal(&w, &r);
    }
    let mut e = reading(&w, &r, INVENTED_EFFECT);
    e.content
        .as_mut()
        .ok_or("retained content")?
        .realization
        .paragraphs[0]
        .citations
        .clear();
    w.reading.explanations = vec![e];
    assert_goal(&w, &r);
    Ok(())
}

#[test]
fn current_label_without_permitted_current_source_cannot_supply_effect_title(
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut w, r) = fixture()?;
    let mut e = reading(&w, &r, INVENTED_EFFECT);
    e.source_excerpts.clear();
    e.omitted_source_excerpt_count = 1;
    e.content.as_mut().ok_or("retained content")?.source_status = vec![serde_json::json!({
        "source_id": "authored-current-source", "availability": "unavailable"
    })];
    // A supplied Current enum cannot turn absent permitted Source bytes into
    // evidence for the generated sentence or override the canonical Goal.
    w.reading.explanations = vec![e];
    assert_goal(&w, &r);
    Ok(())
}

#[test]
fn invalid_utf8_offsets_and_overlong_claims_keep_goal_fallback(
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut w, r) = fixture()?;
    for (start, end) in [(1, 9), (0, 1), (8, 3), (0, usize::MAX)] {
        let mut e = reading(&w, &r, "한 반환 값.");
        let c = &mut e
            .content
            .as_mut()
            .ok_or("retained content")?
            .realization
            .paragraphs[0]
            .citations[0];
        c.claim_start_byte = start;
        c.claim_end_byte = end;
        w.reading.explanations = vec![e];
        assert_goal(&w, &r);
    }
    for claim in [
        "한".repeat(97),
        "x".repeat(97),
        "return 2\nthen continue".into(),
    ] {
        w.reading.explanations = vec![reading(&w, &r, &claim)];
        assert_goal(&w, &r);
    }
    Ok(())
}

#[test]
fn effect_classification_is_not_a_reported_change_title() -> Result<(), Box<dyn std::error::Error>>
{
    let (mut w, r) = fixture()?;
    let mut e = reading(&w, &r, INVENTED_EFFECT);
    e.content
        .as_mut()
        .ok_or("retained content")?
        .realization
        .paragraphs[0]
        .question = ExplanationQuestion::ExpectedEffect;
    w.reading.explanations = vec![e];
    assert_goal(&w, &r);
    Ok(())
}

#[test]
fn mislabeled_causal_effect_with_matching_hashes_is_not_supported_by_return_two(
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut w, r) = fixture()?;
    let e = reading(&w, &r, INVENTED_EFFECT);
    // Independent semantic counterexample: a constant return establishes neither
    // downstream failure prevention nor universality. Reclassification and exact
    // wording hashes cannot supply that absent evidence.
    assert_eq!(e.source_excerpts[0].text, "return 2");
    let retained = e.content.as_ref().ok_or("retained content")?;
    assert_eq!(retained.semantic_approval_status, "not_assessed");
    assert_eq!(
        retained.realization.paragraphs[0].citations[0].claim_sha256,
        explanation_text_hash(INVENTED_EFFECT)
    );
    w.reading.explanations = vec![e];
    assert_goal(&w, &r);
    Ok(())
}

#[test]
fn purpose_classification_does_not_certify_an_invented_short_title(
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut w, r) = fixture()?;
    let mut e = reading(&w, &r, INVENTED_EFFECT);
    e.content.as_mut().ok_or("content")?.realization.paragraphs[0].question =
        ExplanationQuestion::Purpose;
    w.reading.explanations = vec![e];
    assert_goal(&w, &r);
    assert_eq!(w.reading.goal.original_text.as_deref(), Some(GOAL));
    Ok(())
}
