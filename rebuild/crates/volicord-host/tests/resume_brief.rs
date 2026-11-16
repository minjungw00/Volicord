//! Labeled reader-pressure controls over real canonical and publication APIs.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;

use serde_json::{json, Value};
use volicord_host::HostAdapter;
use volicord_operations::LocalOperations;
use volicord_projections::*;

#[test]
fn reader_detail_does_not_displace_canonical_resume_choices(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input: Value = serde_json::from_str(reading_fixture::SCENARIO)?;
    input["prior_checkpoint_count"] = json!(0);
    input["later_checkpoint_count"] = json!(0);
    input["unassociated_checkpoint"] = json!(false);
    let works = input["works"].as_array_mut().ok_or("works")?;
    let position = works
        .iter()
        .position(|w| w["key"] == "older")
        .ok_or("older")?;
    let selected = works.remove(position);
    works.push(selected);
    let f = reading_fixture::fixture_scenario(input)?;
    let work = f.goals["older"];
    let decision = f.decisions["explicit"];
    for language in ["en", "ko"] {
        for (subject, key) in [
            (ExplanationSubject::Work(work), "goal"),
            (ExplanationSubject::Decision(decision), "user_rationale"),
        ] {
            let plan = f
                .operations
                .prepare_explanation(f.project, subject, language)?;
            f.operations.record_explanation(
                f.project,
                subject,
                language,
                ExplanationRealization {
                    format_kind: EXPLANATION_KIND.into(),
                    format_version: EXPLANATION_VERSION,
                    plan_fingerprint: plan.fingerprint,
                    language: language.into(),
                    generator: ExplanationGenerator {
                        host: "transport-control".into(),
                        session: "disposable".into(),
                        agent: None,
                        model: None,
                    },
                    paragraphs: (0..10)
                        .map(|index| ExplanationParagraph {
                            question: ExplanationQuestion::Narrative,
                            text: format!(
                                "Labeled synthetic reader detail {index}: {}",
                                "reader-only fixture text; ".repeat(26)
                            ),
                            evidence_keys: vec![key.into()],
                            citations: Vec::new(),
                        })
                        .collect(),
                    diagrams: Vec::new(),
                },
            )?;
        }
    }
    let before = f.operations.canonical_basis(f.project)?;
    let brief = f.operations.recall(f.project)?;
    let original = brief
        .decisions
        .iter()
        .find(|d| d.decision_id == decision)
        .ok_or("Decision")?;
    for language in ["en", "ko"] {
        let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
        let result = host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":"recall","arguments":{"project_id":f.project.to_string(),"requested_language":language,"fixed_locale":language}
        }})).ok_or("Recall")?;
        assert_eq!(result["result"]["isError"], false, "{result}");
        let value = &result["result"]["structuredContent"];
        let actual = value["decisions"]
            .as_array()
            .ok_or("decisions")?
            .iter()
            .find(|d| d["identity"] == decision.to_string())
            .ok_or("selected Decision")?;
        assert_eq!(
            actual["chosen_alternative_key"],
            json!(original.chosen_alternative_key)
        );
        assert_eq!(
            actual["recommended_alternative_key"],
            json!(original.recommended_alternative_key)
        );
        assert_eq!(
            actual["displayed_alternatives"]
                .as_array()
                .ok_or("alternatives")?
                .len(),
            original.displayed_alternatives.len()
        );
        for (delivered, alternative) in actual["displayed_alternatives"]
            .as_array()
            .ok_or("alternatives")?
            .iter()
            .zip(&original.displayed_alternatives)
        {
            assert_eq!(delivered["alternative_key"], alternative.key);
            assert_eq!(delivered["label"], alternative.label);
            assert_eq!(delivered["expected_consequence"], alternative.consequence);
        }
        assert_eq!(
            actual["evidence"]["user_rationale"],
            json!(original.user_rationale)
        );
        assert_eq!(
            actual["evidence"]["recommendation_rationale"],
            original.recommendation_rationale
        );
        assert_eq!(
            actual["evidence"]["user_source_basis"],
            json!(original
                .user_source_basis
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>())
        );
        assert_eq!(
            actual["evidence"]["recommendation_source_basis"],
            json!(original
                .recommendation_source_basis
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>())
        );
        assert!(
            serde_json::to_vec(&result)?.len() <= volicord_operations::HOST_READ_RESULT_BYTE_BUDGET
        );
        // Full human reading is independently retained, including its final unit.
        let full = host.handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
            "name":"repository_understanding","arguments":{"project_id":f.project.to_string(),"requested_language":language,"fixed_locale":language}
        }})).ok_or("full reading")?;
        assert!(
            full["result"]["structuredContent"]["selected_work"]["answers"]["prose"]
                .to_string()
                .contains("reader detail 9:")
        );
    }
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}
