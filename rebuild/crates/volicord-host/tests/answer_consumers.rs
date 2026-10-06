//! Transport probes use labeled test interpretations; capability evidence is
//! independently generated through the active host and installed browser path.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use serde_json::{json, Value};
use volicord_host::HostAdapter;
use volicord_operations::LocalOperations;
use volicord_projections::*;

#[test]
fn mcp_consumers_preserve_question_answers_and_separate_original_evidence(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = reading_fixture::rich_scenario()?;
    let works = input["works"].as_array_mut().ok_or("works")?;
    let n = works
        .iter()
        .position(|w| w["key"] == "relay")
        .ok_or("relay")?;
    let last = works.remove(n);
    works.push(last);
    let f = reading_fixture::fixture_scenario(input)?;
    let work = f.goals["relay"];
    let subject = ExplanationSubject::Work(work);
    let plan = f.operations.prepare_explanation(f.project, subject, "ko")?;
    let response = ExplanationRealization {
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint.clone(),
        language: "ko".into(),
        generator: ExplanationGenerator {
            host: "transport-test".into(),
            session: "synthetic".into(),
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
        .map(|(q, key)| ExplanationParagraph {
            question: q,
            text: format!("Transport-only Korean-requested paragraph {q:?}"),
            evidence_keys: vec![key.into()],
        })
        .collect(),
    };
    let retained = f
        .operations
        .record_explanation(f.project, subject, "ko", response)?;
    assert_eq!(retained.evidence.len(), plan.evidence.len());
    let provenance = ExplanationProvenance::from(&retained);
    let keys: std::collections::BTreeSet<_> =
        provenance.evidence.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(
        keys,
        ["goal", "result", "verification", "next_step"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        provenance.uncited_evidence_count,
        plan.evidence.len() - keys.len()
    );
    for reference in &provenance.evidence {
        let original = retained
            .evidence
            .iter()
            .find(|e| e.key == reference.key)
            .ok_or("retained reference")?;
        assert_eq!(reference, original);
    }
    let before = f.operations.canonical_basis(f.project)?;
    let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    for tool in ["recall", "repository_understanding"] {
        for locale in ["en", "ko"] {
            let result=host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":tool,"arguments":{"project_id":f.project.to_string(),"requested_language":"ko","fixed_locale":locale}}})).ok_or("MCP response")?;
            assert_eq!(result["result"]["isError"], false, "{result}");
            let data: Value = result["result"]["structuredContent"].clone();
            let selected = &data["selected_work"];
            assert_eq!(selected["work_item_id"], work.to_string(), "{data}");
            assert_eq!(
                selected["answers"]["prose"][1]["text"],
                "Transport-only Korean-requested paragraph ReportedChange"
            );
            assert_eq!(selected["answers"]["provenance"]["subject"]["kind"], "work");
            assert_eq!(
                selected["answers"]["provenance"]["fingerprint"],
                plan.fingerprint
            );
            assert!(selected["answers"].to_string().contains(if locale == "ko" {
                "통과"
            } else {
                "passed"
            }));
            assert!(!selected["answers"].to_string().contains("감사 참고"));
            let basis = selected["answers"]["provenance"]["evidence"]
                .as_array()
                .ok_or("answer basis")?
                .iter()
                .find(|e| e["key"] == "result")
                .ok_or("exact result basis")?;
            assert_eq!(basis["identity"], f.checkpoints["relay-change"].to_string());
            assert_eq!(basis["revision"], 1);
            assert_eq!(basis["field"], "state_change");
            if let Some(original) = selected["evidence"]["result"]["original_text"].as_str() {
                assert!(original.contains("감사 참고"));
            } else {
                assert_eq!(
                    selected["evidence"]["transport_omission"]["reason"],
                    "serialized_byte_budget"
                );
                assert!(
                    selected["evidence"]["transport_omission"]["omitted_field_count"]
                        .as_u64()
                        .ok_or("omission count")?
                        > 0
                );
            }
        }
    }
    // Deleting the retained answer must not restore the retired excerpt reader.
    f.operations.delete_explanations(f.project, subject)?;
    let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let result=host.handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"recall","arguments":{"project_id":f.project.to_string(),"requested_language":"ko","fixed_locale":"ko"}}})).ok_or("MCP response")?;
    let selected = &result["result"]["structuredContent"]["selected_work"];
    assert_eq!(selected["answers"]["explanation_state"], "unavailable");
    assert!(!selected["answers"].to_string().contains("감사 참고"));
    assert!(!selected["answers"].to_string().contains("Transport-only"));
    assert!(
        selected["evidence"]["result"]["original_text"].is_string()
            || selected["evidence"]["transport_omission"]["omitted_field_count"]
                .as_u64()
                .is_some_and(|n| n > 0)
    );
    assert_eq!(f.operations.canonical_basis(f.project)?, before);
    Ok(())
}

#[test]
fn recorded_action_is_readable_without_generated_interpretation(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = reading_fixture::rich_scenario()?;
    let works = input["works"].as_array_mut().ok_or("works")?;
    let n = works
        .iter()
        .position(|w| w["key"] == "relay")
        .ok_or("relay")?;
    let last = works.remove(n);
    works.push(last);
    let f = reading_fixture::fixture_scenario(input)?;
    let expected = "Run a browser check with slow responses and confirm loading feedback.";
    let args = vec![
        "--runtime".into(),
        f.operations.layout().root().to_string_lossy().into_owned(),
        "--project".into(),
        f.project.to_string(),
        "--json".into(),
        "recall".into(),
    ];
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(
        volicord_operations::run_cli(args, &mut out, &mut err),
        volicord_operations::CliExit::SUCCESS
    );
    let cli: Value = serde_json::from_slice(&out)?;
    let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let response = host
        .handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"recall","arguments":{"project_id":f.project.to_string()}}}))
        .ok_or("MCP")?;
    let mcp = &response["result"]["structuredContent"];
    // Assert both real product reads before testing either transport's verdict.
    assert_eq!(cli["next_step"], mcp["next_step"]);
    assert_eq!(cli["next_step"], expected);
    let before = f.operations.canonical_basis(f.project)?;
    let work = f.goals["relay"];
    let subject = ExplanationSubject::Work(work);
    let cp = before
        .checkpoint_history
        .iter()
        .find(|c| c.id == f.checkpoints["relay-change"])
        .ok_or("Checkpoint")?;
    let check = |state: &str,
                 action: &str,
                 checkpoint: String|
     -> Result<(), Box<dyn std::error::Error>> {
        for locale in ["en", "ko"] {
            let args = vec![
                "--runtime".into(),
                f.operations.layout().root().to_string_lossy().into_owned(),
                "--project".into(),
                f.project.to_string(),
                "--locale".into(),
                locale.into(),
                "--json".into(),
                "recall".into(),
                "--language".into(),
                "en".into(),
            ];
            let (mut out, mut err) = (Vec::new(), Vec::new());
            assert_eq!(
                volicord_operations::run_cli(args, &mut out, &mut err),
                volicord_operations::CliExit::SUCCESS
            );
            let cli: Value = serde_json::from_slice(&out)?;
            let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
            for tool in ["recall", "repository_understanding"] {
                let response = host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                    "params":{"name":tool,"arguments":{"project_id":f.project.to_string(),"requested_language":"en","fixed_locale":locale}}})).ok_or("MCP")?;
                assert_eq!(response["result"]["isError"], false, "{response}");
                let data = &response["result"]["structuredContent"];
                let answers = &data["selected_work"]["answers"];
                assert_eq!(answers["explanation_state"], state);
                let fact = answers["facts"]
                    .as_array()
                    .ok_or("facts")?
                    .iter()
                    .find(|a| a["question"] == "RecordedNextStep")
                    .ok_or("recorded action")?;
                assert_eq!(fact["role"], "deterministic_facts");
                assert!(fact["text"]
                    .as_str()
                    .ok_or("action text")?
                    .ends_with(action));
                assert_eq!(fact["recorded_action"]["recorded_text"], action);
                assert_eq!(fact["recorded_action"]["work_item_id"], work.to_string());
                assert_eq!(fact["recorded_action"]["checkpoint_id"], checkpoint);
                assert_eq!(fact["recorded_action"]["revision"], 1);
                assert_eq!(fact["recorded_action"]["field"], "next_step");
                assert_eq!(
                    fact["evidence_keys"],
                    json!([format!("checkpoint:{checkpoint}@1:next_step")])
                );
                assert_eq!(
                    fact["recorded_action"]["source_ids"],
                    json!(cp
                        .source_basis
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>())
                );
                if tool == "recall" {
                    assert_eq!(data["next_step"], action);
                    assert_eq!(cli["next_step"], action);
                    assert_eq!(cli["selected_work"]["answers"], *answers);
                }
            }
        }
        Ok(())
    };
    check("unavailable", expected, cp.id.to_string())?;
    let realize = |plan: &ExplanationPlan| ExplanationRealization {
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint.clone(),
        language: "en".into(),
        generator: ExplanationGenerator {
            host: "unit-test-fake".into(),
            session: "synthetic".into(),
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
            text: if question == ExplanationQuestion::NextStep {
                "Check loading feedback in a slow-response browser session.".into()
            } else {
                format!("Synthetic {question:?}")
            },
            evidence_keys: vec![key.into()],
        })
        .collect(),
    };
    let plan = f.operations.prepare_explanation(f.project, subject, "en")?;
    f.operations
        .record_explanation(f.project, subject, "en", realize(&plan))?;
    check("current", expected, cp.id.to_string())?;
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    let goal = before
        .context_items
        .iter()
        .find(|g| g.id == work)
        .ok_or("Goal")?;
    f.operations.correct_context_item(
        f.project,
        work,
        volicord_context::ContextItemCorrectionDraft {
            expected_revision: goal.revision,
            corrected_statement: format!("{}.", goal.statement),
            user_authorization_source_id: goal.source_basis[0],
            kind: volicord_context::CorrectionKind::Expression,
        },
    )?;
    check("stale", expected, cp.id.to_string())?;
    assert!(f
        .operations
        .record_explanation(f.project, subject, "en", realize(&plan))
        .is_err());
    let fresh = f.operations.prepare_explanation(f.project, subject, "en")?;
    f.operations
        .record_explanation(f.project, subject, "en", realize(&fresh))?;
    check("current", expected, cp.id.to_string())?;
    // A new canonical direction invalidates the old interpretation and updates
    // the ordinary answer with the new Checkpoint's basis, even on restart.
    let mut store = volicord_context::Store::open(f.operations.layout().canonical_store())?;
    let next = store
        .record_checkpoint(
            volicord_context::OperationId::from_bytes([0xc9; 16]),
            f.project,
            volicord_context::CheckpointDraft {
                expected_project_revision: before.project.revision,
                work_item_id: Some(work),
                kind: cp.kind,
                goal: cp.goal.clone(),
                work_state: cp.work_state,
                state_change: None,
                source_basis: cp.source_basis.clone(),
                changed_source_basis: vec![],
                changed_paths: vec![],
                applied_decisions: cp.applied_decisions.clone(),
                verification: cp.verification.clone(),
                user_review: cp.user_review.clone(),
                user_acceptance: cp.user_acceptance.clone(),
                known_limits: vec![],
                non_goals: vec![],
                open_questions: vec![],
                next_step: "Inspect the pending indicator before changing request cancellation."
                    .into(),
                handoff_to: Some("next".into()),
            },
        )?
        .value;
    drop(store);
    check("stale", &next.next_step, next.id.to_string())?;
    f.operations.delete_explanations(f.project, subject)?;
    check("unavailable", &next.next_step, next.id.to_string())?;
    let supporting_source = cp
        .source_basis
        .iter()
        .find(|id| **id != goal.source_basis[0])
        .ok_or("supporting Source")?;
    f.operations.forget_record(
        f.project,
        volicord_context::CanonicalRecordId::Source(*supporting_source),
        goal.source_basis[0],
    )?;
    let recalled = f.operations.recall(f.project)?;
    let answers = work_answers(
        recalled.selected_work.as_ref().ok_or("Work")?,
        "en",
        FixedLocale::English,
    );
    let action = answers.recorded_next_action().ok_or("action")?;
    assert_eq!(action.recorded_text, next.next_step);
    // Forgetting scrubs the dependency from canonical provenance. Do not revive
    // its identity/status or body from previously generated evidence.
    assert!(!action.source_ids.contains(&supporting_source.to_string()));
    assert!(!action
        .source_status
        .iter()
        .any(|s| s["source_id"] == supporting_source.to_string()));
    assert_eq!(answers.provenance, None);
    Ok(())
}

#[test]
fn naturalistic_consumer_checks_product_recalls_across_correction(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = reading_fixture::rich_scenario()?;
    let works = input["works"].as_array_mut().ok_or("works")?;
    let position = works
        .iter()
        .position(|w| w["key"] == "relay")
        .ok_or("relay")?;
    let last = works.remove(position);
    works.push(last);
    let f = reading_fixture::fixture_scenario(input)?;
    let work = f.goals["relay"];
    let subject = ExplanationSubject::Work(work);
    let basis = f.operations.canonical_basis(f.project)?;
    let goal = basis
        .context_items
        .iter()
        .find(|g| g.id == work)
        .ok_or("Goal")?;
    let cp = basis
        .checkpoint_history
        .iter()
        .find(|c| c.id == f.checkpoints["relay-change"])
        .ok_or("Checkpoint")?;
    let realize = |plan: &ExplanationPlan| ExplanationRealization {
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint.clone(),
        language: "en".into(),
        generator: ExplanationGenerator {
            host: "structural-support".into(),
            session: "authored-fixture".into(),
            agent: None,
            model: None,
        },
        paragraphs: [
            ExplanationQuestion::Purpose,
            ExplanationQuestion::ReportedChange,
            ExplanationQuestion::ExpectedEffect,
            ExplanationQuestion::Verification,
            ExplanationQuestion::NextStep,
        ]
        .into_iter()
        .map(|question| ExplanationParagraph {
            question,
            text: format!("Structural support {question:?}"),
            evidence_keys: plan.evidence.iter().map(|e| e.key.clone()).collect(),
        })
        .collect(),
    };
    let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let call = |host: &mut HostAdapter,
                name: &str,
                arguments: Value|
     -> Result<Value, Box<dyn std::error::Error>> {
        let response = host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call", "params":{"name":name,"arguments":arguments}})).ok_or("MCP")?;
        assert_eq!(response["result"]["isError"], false, "{response}");
        Ok(response["result"].clone())
    };
    let plan = f.operations.prepare_explanation(f.project, subject, "en")?;
    f.operations
        .record_explanation(f.project, subject, "en", realize(&plan))?;
    let before = call(
        &mut host,
        "recall",
        json!({"project_id":f.project.to_string(),"requested_language":"en"}),
    )?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    assert_eq!(
        volicord_operations::run_cli(
            vec![
                "--runtime".into(),
                f.operations.layout().root().to_string_lossy().into_owned(),
                "--project".into(),
                f.project.to_string(),
                "--json".into(),
                "recall".into(),
                "--language".into(),
                "en".into()
            ],
            &mut stdout,
            &mut stderr
        ),
        volicord_operations::CliExit::SUCCESS
    );
    let cli: Value = serde_json::from_slice(&stdout)?;
    assert_eq!(
        cli["selected_work"]["answers"],
        before["structuredContent"]["selected_work"]["answers"]
    );
    let args = json!({"project_id":f.project.to_string(),"action":"correct_context","record_id":work.to_string(),
        "expected_revision":1,"corrected_text":format!("{}.",goal.statement),"user_turn":"Correct punctuation only."});
    let mutation = call(&mut host, "canonical_mutate", args.clone())?;
    assert_eq!(mutation["structuredContent"]["revision"], 2);
    let corrected = f.operations.canonical_basis(f.project)?;
    let new_goal = corrected
        .context_items
        .iter()
        .find(|g| g.id == work)
        .ok_or("corrected Goal")?;
    assert_eq!(new_goal.source_basis, goal.source_basis);
    assert!(corrected.sources.iter().any(
        |s| s.source.id.to_string() == mutation["structuredContent"]["user_response_source_id"]
    ));
    assert!(!goal
        .source_basis
        .iter()
        .any(|s| s.to_string() == mutation["structuredContent"]["user_response_source_id"]));
    let plan = f.operations.prepare_explanation(f.project, subject, "en")?;
    f.operations
        .record_explanation(f.project, subject, "en", realize(&plan))?;
    let after = call(
        &mut host,
        "recall",
        json!({"project_id":f.project.to_string(),"requested_language":"en"}),
    )?;
    let bundle = f._temporary.path().join("final.bundle.json");
    f.operations.export_bundle(f.project, &bundle)?;
    let config = json!({"project":f.project.to_string(),"work":work.to_string(),"source":goal.source_basis[0].to_string(),
        "checkpoint":cp.id.to_string(),"next_step":cp.next_step,"before":before["structuredContent"],"before_cli":cli,
        "after":after["structuredContent"],"mutation_arguments":args,"mutation_result":mutation,"bundle":bundle});
    let config_path = f._temporary.path().join("consumer.json");
    std::fs::write(&config_path, serde_json::to_vec(&config)?)?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("crates")?
        .parent()
        .ok_or("rebuild")?;
    let output = std::process::Command::new("python3")
        .arg(root.join("validation/dogfood/answer_product_support.py"))
        .arg(config_path)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()?;
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn metadata_heavy_explanations_remain_bounded_without_replacing_recorded_action(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = reading_fixture::explanation_size_scenario("metadata_heavy")?;
    let works = input["works"].as_array_mut().ok_or("works")?;
    let n = works
        .iter()
        .position(|w| w["key"] == "relay")
        .ok_or("relay")?;
    let last = works.remove(n);
    works.push(last);
    let f = reading_fixture::fixture_scenario(input)?;
    let work = f.goals["relay"];
    let subject = ExplanationSubject::Work(work);
    let plan = f.operations.prepare_explanation(f.project, subject, "ko")?;
    let response = ExplanationRealization {
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint.clone(),
        language: "ko".into(),
        generator: ExplanationGenerator {
            host: "structural-host-control".into(),
            session: "disposable".into(),
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
            text: format!("구조 전송 검증: {question:?}"),
            evidence_keys: vec![key.into()],
        })
        .collect(),
    };
    let retained = f
        .operations
        .record_explanation(f.project, subject, "ko", response)?;
    assert!(serde_json::to_vec(&retained)?.len() > 16384);
    let expected = "Run a browser check with slow responses and confirm loading feedback.";
    let canonical = f.operations.canonical_basis(f.project)?;
    for retained in [true, false] {
        if !retained {
            f.operations.delete_explanations(f.project, subject)?;
        }
        let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
        for tool in ["recall", "repository_understanding"] {
            let response=host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":tool,
                "arguments":{"project_id":f.project.to_string(),"requested_language":"ko","fixed_locale":"ko"}}})).ok_or("MCP")?;
            assert_eq!(response["result"]["isError"], false, "{response}");
            let data = &response["result"]["structuredContent"];
            if tool == "recall" {
                assert!(
                    serde_json::to_vec(&response)?.len()
                        <= volicord_operations::HOST_READ_RESULT_BYTE_BUDGET
                );
                assert!(
                    serde_json::to_vec(data)?.len()
                        <= volicord_operations::HOST_READ_STRUCTURED_BYTE_BUDGET
                );
                assert_eq!(data["next_step"], expected);
                assert!(data.to_string().contains("serialized_byte_budget"));
            } else if retained {
                // repository_understanding is a full local read, not the bounded
                // Recall wire contract; no fictitious shared byte ceiling.
                assert_eq!(
                    data["selected_work"]["answers"]["explanation_state"],
                    "current"
                );
                assert_eq!(
                    data["selected_work"]["answers"]["provenance"]["fingerprint"],
                    plan.fingerprint
                );
            }
            assert_eq!(data["selected_work"]["work_item_id"], work.to_string());
            assert!(data.to_string().contains("RecordedNextStep"));
        }
    }
    assert_eq!(canonical, f.operations.canonical_basis(f.project)?);
    Ok(())
}
