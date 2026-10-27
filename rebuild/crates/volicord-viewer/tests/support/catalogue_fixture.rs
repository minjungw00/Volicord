//! Explicit synthetic scale controls, through ordinary canonical and publication APIs.
#[path = "../../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
pub mod reading_fixture;
use volicord_operations::*;
use volicord_projections::*;
pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub fn fixture(count: usize) -> TestResult<reading_fixture::Fixture> {
    let mut scenario: serde_json::Value = serde_json::from_str(reading_fixture::SCENARIO)?;
    scenario["prior_checkpoint_count"] = 0.into();
    scenario["later_checkpoint_count"] = 0.into();
    scenario["unassociated_checkpoint"] = false.into();
    scenario["decisions"] = serde_json::json!([]);
    scenario["works"]=serde_json::json!((0..count).map(|i|serde_json::json!({"key":format!("control-{i:03}"),"title":format!("Synthetic bounded catalogue request {i:03}"),"paths":[],"checkpoints":[{"key":format!("cp-{i}"),"work":if i%3==0 {"InProgress"}else if i%3==1 {"Paused"}else{"Completed"},"state_change":format!("Synthetic report {i:03}; no actual execution"),"next_step":"Synthetic control only"}]})).collect::<Vec<_>>());
    let f = reading_fixture::fixture_scenario(scenario)?;
    for (key, id) in &f.goals {
        let i = key
            .strip_prefix("control-")
            .ok_or("key")?
            .parse::<usize>()?;
        for (subject, question, value) in [
            (
                ExplanationSubject::WorkName(*id),
                ExplanationQuestion::Heading,
                format!(
                    "{} {i:03}",
                    if i % 2 == 0 {
                        "Repeated 주제"
                    } else {
                        "Control"
                    }
                ),
            ),
            (
                ExplanationSubject::WorkSummary(*id),
                ExplanationQuestion::Synopsis,
                format!(
                    "Synthetic summary {} {i:03} — no actual tests or acceptance.",
                    if i % 2 == 0 { "needle" } else { "other" }
                ),
            ),
        ] {
            record(&f, subject, question, &value)?;
        }
    }
    Ok(f)
}
pub fn record(
    f: &reading_fixture::Fixture,
    subject: ExplanationSubject,
    question: ExplanationQuestion,
    value: &str,
) -> TestResult<()> {
    let plan = f.operations.prepare_explanation(f.project, subject, "en")?;
    let realization = f.operations.compose_subject_explanation(
        f.project,
        subject,
        WorkExplanationDraft {
            diagrams: vec![],
            plan_fingerprint: plan.fingerprint,
            language: "en".into(),
            generator: ExplanationGenerator {
                host: "synthetic-catalogue-control".into(),
                session: "validation-only".into(),
                agent: None,
                model: None,
            },
            paragraphs: vec![WorkReaderUnit {
                question,
                text: value.into(),
                evidence_keys: plan.evidence.into_iter().map(|e| e.key).collect(),
                sources: vec![],
            }],
        },
    )?;
    f.operations
        .record_explanation(f.project, subject, "en", realization)?;
    Ok(())
}
