//! Renderer-supplied binding changes independently of human/browser judgments.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use serde_json::{json, Value};
use volicord_context::*;
use volicord_projections::*;
use volicord_viewer::*;
fn read(
    f: &reading_fixture::Fixture,
    locale: ViewerLocale,
    view: ViewerView,
) -> Result<Value, Box<dyn std::error::Error>> {
    let page = ViewerAdapter::new(volicord_operations::LocalOperations::new(
        f.operations.layout().clone(),
    ))
    .render(
        &ViewerRequest {
            project_id: f.project,
            locale,
            view,
            requested_language: if locale == ViewerLocale::English {
                "en"
            } else {
                "ko"
            }
            .into(),
            guarded_request: None,
        },
        "test-authenticity",
    )?;
    let encoded = page
        .html
        .split("<meta name=\"volicord-observation\" content=\"")
        .nth(1)
        .ok_or("context missing")?
        .split("\">")
        .next()
        .ok_or("context end")?;
    Ok(serde_json::from_str(
        &encoded
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&"),
    )?)
}
fn record(
    f: &reading_fixture::Fixture,
    subject: ExplanationSubject,
    language: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let plan = f
        .operations
        .prepare_explanation(f.project, subject, language)?;
    let questions = if matches!(subject, ExplanationSubject::Work(_)) {
        vec![
            ExplanationQuestion::Purpose,
            ExplanationQuestion::ReportedChange,
            ExplanationQuestion::ExpectedEffect,
            ExplanationQuestion::Verification,
            ExplanationQuestion::NextStep,
        ]
    } else {
        vec![
            ExplanationQuestion::UserRationale,
            ExplanationQuestion::Recommendation,
            ExplanationQuestion::Consequences,
            ExplanationQuestion::Applicability,
        ]
    };
    let response = ExplanationRealization {
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint,
        language: language.into(),
        generator: ExplanationGenerator {
            host: "context-test".into(),
            session: "synthetic".into(),
            agent: None,
            model: None,
        },
        paragraphs: questions
            .into_iter()
            .map(|question| ExplanationParagraph {
                question,
                text: "Labeled structural fixture".into(),
                evidence_keys: plan.evidence.iter().map(|e| e.key.clone()).collect(),
            })
            .collect(),
    };
    f.operations
        .record_explanation(f.project, subject, language, response)?;
    Ok(())
}
fn state(context: &Value, identity: &str) -> Value {
    context["explanations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["identity"] == identity)
        .unwrap()["state"]
        .clone()
}
#[test]
fn rendered_context_preserves_absent_current_stale_regenerated_and_both_locales(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?;
    let work = f.goals["relay"];
    let subject = ExplanationSubject::Work(work);
    let view = ViewerView::Work { work: Some(work) };
    let absent = read(&f, ViewerLocale::English, view.clone())?;
    assert_eq!(absent["selected_work"], work.to_string());
    assert_eq!(state(&absent, &work.to_string()), "unavailable");
    assert!(absent["process"]["start_ticks"]
        .as_u64()
        .is_some_and(|n| n > 0));
    let before = f.operations.canonical_basis(f.project)?;
    record(&f, subject, "en")?;
    record(&f, subject, "ko")?;
    let current = read(&f, ViewerLocale::English, view.clone())?;
    let korean = read(&f, ViewerLocale::Korean, view.clone())?;
    assert_eq!(state(&current, &work.to_string()), "current");
    assert_eq!(state(&korean, &work.to_string()), "current");
    assert_eq!(
        current["canonical_read_fingerprint"],
        absent["canonical_read_fingerprint"]
    );
    assert_ne!(current["render_id"], korean["render_id"]);
    assert_eq!(korean["locale"], "ko");
    assert_eq!(korean["language"], "ko");
    let source = before
        .context_items
        .iter()
        .find(|c| c.id == work)
        .ok_or("goal")?
        .source_basis[0];
    let text = before
        .context_items
        .iter()
        .find(|c| c.id == work)
        .ok_or("goal")?
        .statement
        .clone();
    f.operations.correct_context_item(
        f.project,
        work,
        ContextItemCorrectionDraft {
            expected_revision: 1,
            corrected_statement: text + ".",
            user_authorization_source_id: source,
            kind: CorrectionKind::Expression,
        },
    )?;
    let stale = read(&f, ViewerLocale::English, view.clone())?;
    assert_eq!(state(&stale, &work.to_string()), "stale");
    assert_ne!(
        stale["canonical_read_fingerprint"],
        current["canonical_read_fingerprint"]
    );
    record(&f, subject, "en")?;
    let regenerated = read(&f, ViewerLocale::English, view.clone())?;
    assert_eq!(state(&regenerated, &work.to_string()), "current");
    assert_ne!(regenerated["explanations"], current["explanations"]);
    assert_eq!(
        state(
            &read(&f, ViewerLocale::Korean, view.clone())?,
            &work.to_string()
        ),
        "stale"
    );
    f.operations.delete_explanations(f.project, subject)?;
    assert_eq!(
        state(&read(&f, ViewerLocale::English, view)?, &work.to_string()),
        "unavailable"
    );
    let decision = f.decisions["project"];
    record(&f, ExplanationSubject::Decision(decision), "ko")?;
    let detail = read(
        &f,
        ViewerLocale::Korean,
        ViewerView::Decisions {
            decision: Some(decision),
        },
    )?;
    assert_eq!(detail["selected_decision"], decision.to_string());
    assert_eq!(state(&detail, &decision.to_string()), "current");
    assert_eq!(
        detail["view"],
        json!({"view":"decisions","decision":decision.to_string()})
    );
    Ok(())
}
