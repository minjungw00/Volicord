//! Authored structural controls exercise real Product production and storage.
//! They do not qualify live model generation or independently approve meaning.
use serde_json::{json, Value};
use std::{error::Error, fs};
use volicord_context::{ContextItemId, ContextItemRole, ProjectId, SourceId};
use volicord_host::HostAdapter;
use volicord_operations::{FileObservationRequest, LocalOperations, RuntimeLayout};
use volicord_projections::*;

fn call(host: &mut HostAdapter, args: Value) -> Value {
    host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"work_explanation","arguments":args}})).unwrap_or(Value::Null)
}
fn success(host: &mut HostAdapter, args: Value) -> Result<Value, Box<dyn Error>> {
    let response = call(host, args);
    if response["result"]["isError"] != false {
        return Err(response.to_string().into());
    }
    Ok(response["result"]["structuredContent"].clone())
}
struct Fixture {
    temp: tempfile::TempDir,
    host: HostAdapter,
    project: ProjectId,
    work: ContextItemId,
    source: SourceId,
}
impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        Self::with_goal("Explain value; investigate before recommending a change.")
    }
    fn with_goal(goal: &str) -> Result<Self, Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("repo");
        fs::create_dir(&root)?;
        fs::write(root.join("main.py"), "def value():\n    return 7\n")?;
        // Inventory/source capability is independent of structural language support.
        fs::write(root.join("caller.custom"), "value()\n")?;
        fs::write(
            root.join("test.py"),
            "from main import value\nassert value() == 7\n",
        )?;
        let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
        let project = ops
            .initialize_project("explain actual value", Some(&root))?
            .project
            .id;
        let work = ops
            .record_current_host_user_context(
                project,
                "fixture".into(),
                "structural-control".into(),
                format!("{goal} Unrelated private turn content: PRIVATE_TURN_SENTINEL."),
                ContextItemRole::Goal,
                goal.into(),
            )?
            .context_item_id;
        let analysis = ops
            .analyze(project, vec![])?
            .value
            .ok_or("analysis")?
            .analysis
            .identity;
        let source = ops
            .observe_work_file(FileObservationRequest {
                byte_range: None,
                project_id: project,
                work_item_id: work,
                analysis_snapshot_id: analysis,
                path: "main.py".into(),
                before_source_id: None,
                retain_seconds: 3600,
            })?
            .source_id;
        Ok(Self {
            temp,
            host: HostAdapter::new(ops),
            project,
            work,
            source,
        })
    }
    fn args(&self, action: &str) -> Value {
        json!({"action":action,"project_id":self.project.to_string(),"work_item_id":self.work.to_string()})
    }
    fn generate(&mut self) -> Result<Value, Box<dyn Error>> {
        let mut args = self.args("generate");
        args["language"] = json!("en");
        success(&mut self.host, args)
    }
    fn read(&mut self, generation: &Value, source: SourceId) -> Result<Value, Box<dyn Error>> {
        let mut args = self.args("read_source");
        args["generation_id"] = generation["generation_id"].clone();
        args["source_id"] = json!(source.to_string());
        success(&mut self.host, args)
    }
    fn record_args(
        &self,
        generation: &Value,
        response: &ExplanationRealization,
    ) -> Result<Value, Box<dyn Error>> {
        let mut args = self.args("record");
        args["generation_id"] = generation["generation_id"].clone();
        args["realization"] = serde_json::to_value(response)?;
        Ok(args)
    }
}
fn response(generation: &Value, read: &Value) -> Result<ExplanationRealization, Box<dyn Error>> {
    let plan: ExplanationPlan = serde_json::from_value(generation["plan"].clone())?;
    let observation = &read["observation"];
    let text="The observed function returns the constant 7. No code change or execution result is established.";
    let evidence = observation["detail"]["body"].as_str().ok_or("body")?;
    let citation = ExplanationCitation {
        source_id: read["citation_basis"]["source_id"]
            .as_str()
            .ok_or("source")?
            .into(),
        source_revision: 1,
        observation_fingerprint: read["observation_fingerprint"]
            .as_str()
            .ok_or("fingerprint")?
            .into(),
        analysis_snapshot_id: observation["analysis_snapshot_id"]
            .as_str()
            .ok_or("analysis")?
            .into(),
        repository_snapshot_id: observation["repository_snapshot_id"]
            .as_str()
            .ok_or("repository")?
            .into(),
        repository_source_id: read["citation_basis"]["repository_source_id"]
            .as_str()
            .ok_or("repository Source")?
            .into(),
        locator: "main.py".into(),
        field: "body".into(),
        start_line: observation["detail"]["region"]["start_line"]
            .as_u64()
            .unwrap_or(1) as usize
            + 1,
        end_line: observation["detail"]["region"]["start_line"]
            .as_u64()
            .unwrap_or(1) as usize
            + 1,
        start_byte: observation["detail"]["region"]["start_byte"]
            .as_u64()
            .unwrap_or(0) as usize
            + 13,
        end_byte: observation["detail"]["region"]["start_byte"]
            .as_u64()
            .unwrap_or(0) as usize
            + evidence.len(),
        range_sha256: explanation_text_hash(&evidence[13..]),
        claim_start_byte: 0,
        claim_end_byte: 43,
        claim_sha256: explanation_text_hash(&text[..43]),
        use_kind: ExplanationSourceUse::CurrentCode,
    };
    let paragraphs = [
        (
            ExplanationQuestion::Purpose,
            "goal",
            "Investigate the value behavior.",
        ),
        (ExplanationQuestion::ReportedChange, "goal", text),
        (
            ExplanationQuestion::ExpectedEffect,
            "goal",
            "No implemented change is reported.",
        ),
        (
            ExplanationQuestion::Verification,
            "goal",
            "Execution verification is not recorded.",
        ),
        (
            ExplanationQuestion::NextStep,
            "next_step",
            "No independently accepted next step is recorded.",
        ),
    ]
    .into_iter()
    .map(|(question, key, text)| ExplanationParagraph {
        question,
        text: text.into(),
        evidence_keys: vec![key.into()],
        citations: if question == ExplanationQuestion::ReportedChange {
            vec![citation.clone()]
        } else {
            vec![]
        },
    })
    .collect();
    Ok(ExplanationRealization {
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint,
        language: plan.requested_language,
        generator: ExplanationGenerator {
            host: "authored-structural-control".into(),
            session: "fixture".into(),
            agent: None,
            model: None,
        },
        paragraphs,
    })
}

#[test]
fn retrieved_source_cannot_replace_required_canonical_grounding() -> Result<(), Box<dyn Error>> {
    let mut f = Fixture::new()?;
    let generation = f.generate()?;
    assert_eq!(
        generation["plan"]["question_grounding"],
        json!([
            {"question":"purpose","required_any_evidence_keys":["goal"]},
            {"question":"reported_change","required_any_evidence_keys":["goal"]},
            {"question":"expected_effect","required_any_evidence_keys":["goal"]},
            {"question":"verification","required_any_evidence_keys":["goal"]},
            {"question":"next_step","required_any_evidence_keys":["next_step"]},
            {"question":"limits","required_any_evidence_keys":[]}
        ])
    );
    assert_eq!(generation["plan"]["evidence"][1]["content"], Value::Null);
    let read = f.read(&generation, f.source)?;
    let valid = response(&generation, &read)?;
    let mut original = valid.clone();
    original.paragraphs[1].evidence_keys.clear();
    let args = f.record_args(&generation, &original)?;
    let rejected = call(&mut f.host, args);
    assert_eq!(rejected["result"]["isError"], true);
    assert!(
        rejected["result"]["structuredContent"]["details"]["problems"]
            .as_array()
            .ok_or("schema problems")?
            .iter()
            .any(|p| {
                p.as_str().is_some_and(|p| {
                    p.contains("paragraphs[1].evidence_keys must contain at least 1")
                })
            })
    );
    println!("First submission rejected despite exact retrieved Source citation: {rejected}");
    let projection = f.host.operations().project_projection(f.project)?;
    let work = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("Work")?;
    assert_eq!(
        work_answers(work, "en", FixedLocale::English).explanation_state,
        ExplanationState::Missing
    );

    // This explicitly separate opportunity does not rewrite the first failure.
    let next = f.generate()?;
    f.read(&next, f.source)?;
    let args = f.record_args(&next, &valid)?;
    success(&mut f.host, args)?;
    let previous = f
        .host
        .operations()
        .privacy_status(f.project)?
        .managed_derived;
    for variant in 0..5 {
        let attempt = f.generate()?;
        f.read(&attempt, f.source)?;
        let mut invalid = valid.clone();
        match variant {
            0 => invalid.paragraphs[1].evidence_keys.clear(),
            1 => invalid.paragraphs[1].evidence_keys = vec!["fabricated".into()],
            2 => invalid.paragraphs[0].evidence_keys = vec!["next_step".into()],
            3 => invalid.paragraphs.push(ExplanationParagraph {
                question: ExplanationQuestion::Recommendation,
                text: "Host advice is not a Work question.".into(),
                evidence_keys: vec!["goal".into()],
                citations: vec![],
            }),
            _ => {
                let mut malformed = f.record_args(&attempt, &valid)?;
                malformed["realization"]["paragraphs"][1]
                    .as_object_mut()
                    .ok_or("paragraph")?
                    .remove("evidence_keys");
                assert_eq!(call(&mut f.host, malformed)["result"]["isError"], true);
                assert_eq!(
                    previous,
                    f.host
                        .operations()
                        .privacy_status(f.project)?
                        .managed_derived
                );
                continue;
            }
        }
        let args = f.record_args(&attempt, &invalid)?;
        assert_eq!(
            call(&mut f.host, args)["result"]["isError"],
            true,
            "variant {variant}"
        );
        assert_eq!(
            previous,
            f.host
                .operations()
                .privacy_status(f.project)?
                .managed_derived
        );
    }
    Ok(())
}

fn citation_for(
    read: &Value,
    field: &str,
    selected: &str,
    paragraph: &str,
    claim: &str,
    use_kind: ExplanationSourceUse,
) -> Result<ExplanationCitation, Box<dyn Error>> {
    let observation = &read["observation"];
    let detail = &observation["detail"];
    let body = if field == "body" {
        &detail["body"]
    } else {
        &detail[field]["body"]
    }
    .as_str()
    .ok_or("body")?;
    let start = body.find(selected).ok_or("selected bytes")?;
    let claim_start = paragraph.find(claim).ok_or("claim bytes")?;
    let locator = if field == "body" {
        "main.py"
    } else {
        "Check changed value"
    };
    Ok(ExplanationCitation {
        source_id: read["citation_basis"]["source_id"]
            .as_str()
            .ok_or("source")?
            .into(),
        source_revision: 1,
        observation_fingerprint: read["observation_fingerprint"]
            .as_str()
            .ok_or("fingerprint")?
            .into(),
        analysis_snapshot_id: observation["analysis_snapshot_id"]
            .as_str()
            .ok_or("analysis")?
            .into(),
        repository_snapshot_id: observation["repository_snapshot_id"]
            .as_str()
            .ok_or("repository")?
            .into(),
        repository_source_id: read["citation_basis"]["repository_source_id"]
            .as_str()
            .ok_or("repository source")?
            .into(),
        locator: locator.into(),
        field: field.into(),
        start_byte: start,
        end_byte: start + selected.len(),
        start_line: 1 + body[..start].bytes().filter(|b| *b == b'\n').count(),
        end_line: 1 + body[..start + selected.len() - 1]
            .bytes()
            .filter(|b| *b == b'\n')
            .count(),
        range_sha256: explanation_text_hash(selected),
        claim_start_byte: claim_start,
        claim_end_byte: claim_start + claim.len(),
        claim_sha256: explanation_text_hash(claim),
        use_kind,
    })
}

#[test]
fn changed_work_preparation_exposes_actual_question_grounding_for_first_record(
) -> Result<(), Box<dyn Error>> {
    use std::time::Duration;
    use volicord_context::*;
    use volicord_operations::WorkCommandRequest;
    let mut f = Fixture::with_goal("Change the returned value from 7 to 9 and check it.")?;
    let before_source = f.source;
    fs::write(
        f.temp.path().join("repo/main.py"),
        "def value():\n    return 9\n",
    )?;
    let analysis = f
        .host
        .operations()
        .analyze(f.project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    f.source = f
        .host
        .operations()
        .observe_work_file(FileObservationRequest {
            byte_range: None,
            project_id: f.project,
            work_item_id: f.work,
            analysis_snapshot_id: analysis,
            path: "main.py".into(),
            before_source_id: Some(before_source),
            retain_seconds: 3600,
        })?
        .source_id;
    let execution = f
        .host
        .operations()
        .run_work_command(WorkCommandRequest {
            project_id: f.project,
            work_item_id: f.work,
            analysis_snapshot_id: analysis,
            command_label: "Check changed value".into(),
            program: "python3".into(),
            arguments: vec![
                "-c".into(),
                "from main import value; assert value() == 9; print('값: 9')".into(),
            ],
            timeout: Duration::from_secs(5),
            retain_seconds: 3600,
        })?
        .source_id;
    // Explicit authored canonical report through the Kernel's public producer;
    // actual observed before/after bytes and execution remain separate Sources.
    let canonical = f.host.operations().canonical_basis(f.project)?;
    let mut store = Store::open(f.host.operations().layout().canonical_store())?;
    let checkpoint = store
        .record_checkpoint(
            OperationId::from_bytes([0xa7; 16]),
            f.project,
            CheckpointDraft {
                expected_project_revision: canonical.project.revision,
                work_item_id: Some(f.work),
                kind: CheckpointKind::Handoff,
                goal: "Change the returned value from 7 to 9 and check it.".into(),
                work_state: WorkState::Paused,
                state_change: Some(
                    "Changed value from 7 to 9; callers receive the new constant.".into(),
                ),
                source_basis: vec![f.source, before_source, execution],
                changed_source_basis: vec![f.source],
                changed_paths: vec!["main.py".into()],
                applied_decisions: vec![],
                verification: vec![VerificationFact {
                    state: VerificationState::Passed,
                    source_id: Some(execution),
                    outcome: Some("Value assertion passed; stdout reports 값: 9.".into()),
                }],
                user_review: UserReviewFact {
                    state: UserReviewState::NotRequested,
                    source_id: None,
                },
                user_acceptance: UserAcceptanceFact {
                    state: UserAcceptanceState::NotRequested,
                    source_id: None,
                },
                known_limits: vec!["Caller integration was not executed.".into()],
                non_goals: vec![],
                open_questions: vec![],
                next_step: "Check caller integration.".into(),
                handoff_to: Some("caller integration follow-up".into()),
            },
        )?
        .value;
    drop(store);
    let canonical = f.host.operations().canonical_basis(f.project)?;
    let privacy = f.host.operations().privacy_status(f.project)?;
    let generation = f.generate()?;
    let rules = &generation["plan"]["question_grounding"];
    let change_key = format!("change:{}", checkpoint.id);
    assert_eq!(
        rules,
        &json!([
            {"question":"purpose","required_any_evidence_keys":["goal"]},
            {"question":"reported_change","required_any_evidence_keys":["result",change_key]},
            {"question":"expected_effect","required_any_evidence_keys":["result",change_key]},
            {"question":"verification","required_any_evidence_keys":["verification"]},
            {"question":"next_step","required_any_evidence_keys":["next_step"]},
            {"question":"limits","required_any_evidence_keys":[]}
        ])
    );
    let current = f.read(&generation, f.source)?;
    let before = f.read(&generation, before_source)?;
    let command = f.read(&generation, execution)?;
    assert_eq!(canonical, f.host.operations().canonical_basis(f.project)?);
    assert_eq!(privacy, f.host.operations().privacy_status(f.project)?);
    let mut realization = response(&generation, &current)?;
    realization.paragraphs[0].text =
        "The requested Work raises the returned value from 7 to 9.".into();
    let changed =
        "The Work reports changing the constant: previously it returned 7; now it returns 9.";
    realization.paragraphs[1] = ExplanationParagraph {
        question: ExplanationQuestion::ReportedChange,
        text: changed.into(),
        evidence_keys: vec!["result".into()],
        citations: vec![
            citation_for(
                &before,
                "body",
                "return 7",
                changed,
                "previously it returned 7",
                ExplanationSourceUse::BeforeCode,
            )?,
            citation_for(
                &current,
                "body",
                "return 9",
                changed,
                "now it returns 9",
                ExplanationSourceUse::CurrentCode,
            )?,
        ],
    };
    let effect = "The reported update changes the expected caller value to 9; the current function returns 9.";
    realization.paragraphs[2] = ExplanationParagraph {
        question: ExplanationQuestion::ExpectedEffect,
        text: effect.into(),
        evidence_keys: vec![change_key],
        citations: vec![citation_for(
            &current,
            "body",
            "return 9",
            effect,
            "the current function returns 9",
            ExplanationSourceUse::CurrentCode,
        )?],
    };
    let verified = "기록된 검증은 값 검사를 통과했다. 관측 stdout은 값: 9이다. Caller integration remains unverified.";
    realization.paragraphs[3] = ExplanationParagraph {
        question: ExplanationQuestion::Verification,
        text: verified.into(),
        evidence_keys: vec!["verification".into(), format!("limits:{}", checkpoint.id)],
        citations: vec![citation_for(
            &command,
            "stdout",
            "값: 9",
            verified,
            "관측 stdout은 값: 9",
            ExplanationSourceUse::ObservedExecution,
        )?],
    };
    realization.paragraphs[4].text =
        "The recorded next action is to check caller integration.".into();
    let first = f.record_args(&generation, &realization)?;
    let retained = success(&mut f.host, first)?;
    assert_eq!(
        retained["explanation"]["realization"],
        serde_json::to_value(&realization)?
    );
    assert_eq!(
        retained["explanation"]["semantic_approval_status"],
        "not_established"
    );
    let projection = f.host.operations().project_projection(f.project)?;
    let work = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("Work")?;
    let answers = work_answers(work, "en", FixedLocale::English);
    assert_eq!(answers.explanation_state, ExplanationState::Current);
    assert!(answers.text().contains("관측 stdout은 값: 9"));
    assert_eq!(canonical, f.host.operations().canonical_basis(f.project)?);

    let previous = f
        .host
        .operations()
        .privacy_status(f.project)?
        .managed_derived;
    let other = f
        .host
        .operations()
        .record_current_host_user_context(
            f.project,
            "fixture".into(),
            "foreign-work".into(),
            "Inspect unrelated work.".into(),
            ContextItemRole::Goal,
            "Inspect unrelated work.".into(),
        )?
        .context_item_id;
    let mut store = Store::open(f.host.operations().layout().canonical_store())?;
    let foreign_checkpoint = store
        .record_checkpoint(
            OperationId::from_bytes([0xa8; 16]),
            f.project,
            CheckpointDraft {
                expected_project_revision: canonical.project.revision,
                work_item_id: Some(other),
                kind: checkpoint.kind,
                goal: "Inspect unrelated work.".into(),
                work_state: checkpoint.work_state,
                state_change: Some("Unrelated report with overlapping paths.".into()),
                source_basis: checkpoint.source_basis.clone(),
                changed_source_basis: checkpoint.changed_source_basis.clone(),
                changed_paths: checkpoint.changed_paths.clone(),
                applied_decisions: vec![],
                verification: checkpoint.verification.clone(),
                user_review: checkpoint.user_review.clone(),
                user_acceptance: checkpoint.user_acceptance.clone(),
                known_limits: vec![],
                non_goals: vec![],
                open_questions: vec![],
                next_step: "Continue unrelated work.".into(),
                handoff_to: Some("unrelated".into()),
            },
        )?
        .value;
    drop(store);
    for question in 0..7 {
        let attempt = f.generate()?;
        for source in [f.source, before_source, execution] {
            f.read(&attempt, source)?;
        }
        let mut wrong = realization.clone();
        if question < 5 {
            wrong.paragraphs[question].evidence_keys =
                vec![if question == 0 { "result" } else { "goal" }.into()];
        } else {
            wrong.paragraphs[1].evidence_keys = vec![if question == 5 {
                "latest_state".into()
            } else {
                format!("change:{}", foreign_checkpoint.id)
            }];
        }
        let args = f.record_args(&attempt, &wrong)?;
        assert_eq!(
            call(&mut f.host, args)["result"]["isError"],
            true,
            "question {question}"
        );
        assert_eq!(
            previous,
            f.host
                .operations()
                .privacy_status(f.project)?
                .managed_derived
        );
    }
    Ok(())
}

#[test]
fn explicit_generation_retrieval_record_read_and_delete_are_separate() -> Result<(), Box<dyn Error>>
{
    let mut f = Fixture::new()?;
    let before = f.host.operations().canonical_basis(f.project)?;
    let records = f
        .host
        .operations()
        .privacy_status(f.project)?
        .managed_derived;
    let mut prepare = f.args("prepare");
    prepare["language"] = json!("en");
    let prepared = success(&mut f.host, prepare)?;
    assert!(!prepared.to_string().contains("PRIVATE_TURN_SENTINEL"));
    assert_eq!(prepared["generation_status"], "not_requested");
    assert_eq!(prepared["read_only"], true);
    assert!(prepared.get("generation_id").is_none());
    let mut unrequested = f.args("read_source");
    unrequested["generation_id"] = json!("prepare-is-not-generation");
    unrequested["source_id"] = json!(f.source.to_string());
    assert_eq!(call(&mut f.host, unrequested)["result"]["isError"], true);
    // Reading a missing explanation stays missing; it cannot manufacture prose.
    let projection = f.host.operations().project_projection(f.project)?;
    let work = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("Work reading")?;
    assert_eq!(
        work_answers(work, "en", FixedLocale::English).explanation_state,
        ExplanationState::Missing
    );
    let privacy_before = f.host.operations().privacy_status(f.project)?;
    assert!(privacy_before.current_opt_in.is_none());
    assert!(privacy_before.requests.is_empty());
    let generation = f.generate()?;
    assert!(!generation.to_string().contains("PRIVATE_TURN_SENTINEL"));
    assert_eq!(
        generation["generation_status"],
        "awaiting_host_interpretation"
    );
    assert_eq!(before, f.host.operations().canonical_basis(f.project)?);
    assert_eq!(
        records,
        f.host
            .operations()
            .privacy_status(f.project)?
            .managed_derived
    );
    let read = f.read(&generation, f.source)?;
    // An explicit explanation request does not supply background transmission consent.
    assert_eq!(
        privacy_before,
        f.host.operations().privacy_status(f.project)?
    );
    let realization = response(&generation, &read)?;
    let record = f.record_args(&generation, &realization)?;
    let recorded = success(&mut f.host, record.clone())?;
    assert_eq!(
        recorded["generation_status"],
        "recorded_host_interpretation"
    );
    assert_eq!(
        recorded["explanation"]["semantic_approval_status"],
        "not_established"
    );
    assert_eq!(
        recorded["explanation"]["generator_identity_status"],
        "self_reported_not_independently_verified"
    );
    assert_eq!(call(&mut f.host, record)["result"]["isError"], true);
    let projection = f.host.operations().project_projection(f.project)?;
    let work = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("Work reading")?;
    let answers = work_answers(work, "en", FixedLocale::English);
    assert_eq!(answers.explanation_state, ExplanationState::Current);
    assert!(answers.text().contains("main.py:2–2"));
    assert!(!answers
        .provenance
        .as_ref()
        .ok_or("provenance")?
        .citations
        .is_empty());
    let canonical = f.host.operations().canonical_basis(f.project)?;
    assert_eq!(canonical, before);
    assert!(canonical.active_decisions.is_empty());
    assert!(canonical
        .context_items
        .iter()
        .all(|i| i.role != ContextItemRole::Learning));
    let deleted = f.args("delete");
    success(&mut f.host, deleted)?;
    assert_eq!(f.host.operations().canonical_basis(f.project)?, before);
    assert!(f
        .host
        .operations()
        .privacy_status(f.project)?
        .managed_derived
        .iter()
        .filter(|r| r.purpose.starts_with("explanation:"))
        .all(|r| r.content.is_none()));
    Ok(())
}

#[test]
fn citation_metadata_uses_response_headroom_without_expanding_source_authority(
) -> Result<(), Box<dyn Error>> {
    let mut f = Fixture::new()?;
    let canonical = f.host.operations().canonical_basis(f.project)?;
    let provider_before = f.host.operations().privacy_status(f.project)?;
    let generation = f.generate()?;
    let read = f.read(&generation, f.source)?;
    let mut realization = response(&generation, &read)?;
    let citation = realization.paragraphs[1].citations[0].clone();
    // Authored size control: repeated valid bindings reproduce citation overhead,
    // not additional facts, independently approved meaning, or live generation.
    realization.paragraphs[1].citations = vec![citation; 24];
    let bytes = serde_json::to_vec(&realization)?.len();
    assert!(bytes > 16_384 && bytes <= 32_768);
    assert_eq!(
        generation["plan"]["retention_budget"]["retained_byte_limit"],
        147_456
    );
    let args = f.record_args(&generation, &realization)?;
    let retained = success(&mut f.host, args)?;
    assert_eq!(
        retained["explanation"]["realization"],
        serde_json::to_value(&realization)?
    );
    let projection = f.host.operations().project_projection(f.project)?;
    let work = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("Work")?;
    let answers = work_answers(work, "en", FixedLocale::English);
    assert_eq!(answers.explanation_state, ExplanationState::Current);
    assert_eq!(answers.source_excerpts.len(), 1);
    let before_rejection = f.host.operations().privacy_status(f.project)?;
    let next = f.generate()?;
    f.read(&next, f.source)?;
    realization.paragraphs[1].citations[0].claim_sha256 = explanation_text_hash("forged claim");
    let args = f.record_args(&next, &realization)?;
    assert_eq!(call(&mut f.host, args)["result"]["isError"], true);
    assert_eq!(
        before_rejection,
        f.host.operations().privacy_status(f.project)?
    );
    assert_eq!(canonical, f.host.operations().canonical_basis(f.project)?);
    let privacy = f.host.operations().privacy_status(f.project)?;
    assert_eq!(privacy.current_opt_in, provider_before.current_opt_in);
    assert_eq!(privacy.requests, provider_before.requests);
    Ok(())
}

#[test]
fn wrong_scopes_ranges_hashes_bindings_and_denied_retrieval_preserve_previous_record(
) -> Result<(), Box<dyn Error>> {
    let mut f = Fixture::new()?;
    let generation = f.generate()?;
    let read = f.read(&generation, f.source)?;
    let valid = response(&generation, &read)?;
    let args = f.record_args(&generation, &valid)?;
    success(&mut f.host, args)?;
    let previous = f
        .host
        .operations()
        .privacy_status(f.project)?
        .managed_derived;
    let next = f.generate()?;
    // Neither another runtime nor a fresh server can inherit a live request.
    for runtime in ["foreign-runtime", "runtime"] {
        let ops = LocalOperations::new(RuntimeLayout::new(f.temp.path().join(runtime))?);
        let mut foreign_host = HostAdapter::new(ops);
        let mut prepare = f.args("prepare");
        prepare["language"] = json!("en");
        assert_eq!(
            call(&mut foreign_host, prepare)["result"]["isError"],
            runtime == "foreign-runtime"
        );
        let mut read = f.args("read_source");
        read["generation_id"] = next["generation_id"].clone();
        read["source_id"] = json!(f.source.to_string());
        assert_eq!(call(&mut foreign_host, read)["result"]["isError"], true);
        let record = f.record_args(&next, &valid)?;
        assert_eq!(call(&mut foreign_host, record)["result"]["isError"], true);
    }
    // A known valid fingerprint from a previous request is not retrieval here.
    let args = f.record_args(&next, &valid)?;
    assert_eq!(call(&mut f.host, args)["result"]["isError"], true);
    f.read(&next, f.source)?;
    for variant in 0..11 {
        let mut invalid = valid.clone();
        let c = &mut invalid.paragraphs[1].citations[0];
        match variant {
            0 => c.source_revision = 2,
            1 => c.start_byte = usize::MAX,
            2 => c.range_sha256 = explanation_text_hash("wrong bytes"),
            3 => c.claim_sha256 = explanation_text_hash("wrong wording"),
            4 => c.observation_fingerprint = explanation_text_hash("wrong retrieval"),
            5 => c.locator = "test.py".into(),
            6 => c.use_kind = ExplanationSourceUse::BeforeCode,
            7 => c.field = "user_turn".into(),
            8 => c.repository_snapshot_id = "0".repeat(64),
            9 => c.start_line = 1,
            _ => invalid.paragraphs[1].text = "Edited wording requires new claim bindings.".into(),
        }
        let args = f.record_args(&next, &invalid)?;
        assert_eq!(
            call(&mut f.host, args)["result"]["isError"],
            true,
            "variant {variant}"
        );
        assert_eq!(
            f.host
                .operations()
                .privacy_status(f.project)?
                .managed_derived,
            previous
        );
    }
    let other = f
        .host
        .operations()
        .initialize_project("foreign", None)?
        .project
        .id;
    let mut foreign = f.record_args(&next, &valid)?;
    foreign["project_id"] = json!(other.to_string());
    assert_eq!(call(&mut f.host, foreign)["result"]["isError"], true);
    let args = f.record_args(&next, &valid)?;
    success(&mut f.host, args)?;
    Ok(())
}

#[test]
fn changed_and_deleted_sources_cannot_publish_current_prose_and_recovery_is_explicit(
) -> Result<(), Box<dyn Error>> {
    let mut f = Fixture::new()?;
    let generation = f.generate()?;
    let read = f.read(&generation, f.source)?;
    let valid = response(&generation, &read)?;
    let args = f.record_args(&generation, &valid)?;
    success(&mut f.host, args)?;
    let projection_before = f.host.operations().project_projection(f.project)?;
    let work_before = projection_before
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("Work")?;
    let provenance = work_answers(work_before, "en", FixedLocale::English)
        .provenance
        .ok_or("provenance")?;
    let previous = f
        .host
        .operations()
        .privacy_status(f.project)?
        .managed_derived;
    let next = f.generate()?;
    f.read(&next, f.source)?;
    fs::write(
        f.temp.path().join("repo/main.py"),
        "def value():\n    return 9\n",
    )?;
    let stale_destination = f.temp.path().join("stale-snapshot.html");
    assert!(f
        .host
        .operations()
        .publish_viewer_snapshot(
            "<p>Previously rendered explanation</p>",
            &stale_destination,
            f.project,
            &projection_before.canonical_read_fingerprint,
            &[provenance]
        )
        .is_err());
    assert!(!stale_destination.exists());
    let args = f.record_args(&next, &valid)?;
    assert_eq!(call(&mut f.host, args)["result"]["isError"], true);
    assert_eq!(
        f.host
            .operations()
            .privacy_status(f.project)?
            .managed_derived,
        previous
    );
    let projection = f.host.operations().project_projection(f.project)?;
    let work = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("Work")?;
    let reading = work_answers(work, "en", FixedLocale::English);
    assert_eq!(reading.explanation_state, ExplanationState::Stale);
    assert!(reading.provenance.is_none());
    assert!(!reading.facts.is_empty());
    fs::write(
        f.temp.path().join("repo/main.py"),
        "def value():\n    return 7\n",
    )?;
    f.host
        .operations()
        .delete_work_source_observation(f.project, f.work, f.source)?;
    let args = f.record_args(&next, &valid)?;
    assert_eq!(call(&mut f.host, args)["result"]["isError"], true);
    assert!(f.read(&next, f.source).is_err());
    fs::remove_file(f.temp.path().join("repo/main.py"))?;
    let projection = f.host.operations().project_projection(f.project)?;
    let work = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("Work")?;
    assert_ne!(
        work_answers(work, "en", FixedLocale::English).explanation_state,
        ExplanationState::Current
    );
    Ok(())
}

#[test]
fn explicit_equal_pair_expiry_and_semantic_limit_survive_real_storage() -> Result<(), Box<dyn Error>>
{
    for excerpt in [false, true] {
        explicit_pair_expiry(excerpt)?;
    }
    Ok(())
}
fn explicit_pair_expiry(excerpt: bool) -> Result<(), Box<dyn Error>> {
    let mut f = Fixture::new()?;
    let byte_range = if excerpt {
        let prefix = "# context\n".repeat(2000);
        let body = "def value():\n    return 7\n";
        fs::write(
            f.temp.path().join("repo/main.py"),
            format!("{prefix}{body}"),
        )?;
        let a = f
            .host
            .operations()
            .analyze(f.project, vec![])?
            .value
            .ok_or("analysis")?;
        let range = volicord_operations::FileByteRange {
            start_byte: prefix.len(),
            end_byte: prefix.len() + body.len(),
        };
        f.source = f
            .host
            .operations()
            .observe_work_file(FileObservationRequest {
                project_id: f.project,
                work_item_id: f.work,
                analysis_snapshot_id: a.analysis.identity,
                path: "main.py".into(),
                byte_range: Some(range),
                before_source_id: None,
                retain_seconds: 3600,
            })?
            .source_id;
        Some(range)
    } else {
        None
    };
    let generation = f.generate()?;
    let before = f.read(&generation, f.source)?;
    let analysis = volicord_operations::AnalysisSnapshotId::from_hex(
        before["observation"]["analysis_snapshot_id"]
            .as_str()
            .ok_or("analysis")?,
    )?;
    let after = f
        .host
        .operations()
        .observe_work_file(FileObservationRequest {
            byte_range,
            project_id: f.project,
            work_item_id: f.work,
            analysis_snapshot_id: analysis,
            path: "main.py".into(),
            before_source_id: Some(f.source),
            retain_seconds: 1,
        })?
        .source_id;
    let after_read = f.read(&generation, after)?;
    assert_eq!(
        after_read["observation"]["detail"]["comparison"],
        "no_change"
    );
    let mut realized = response(&generation, &after_read)?;
    let mut prior = response(&generation, &before)?.paragraphs[1]
        .citations
        .remove(0);
    prior.use_kind = ExplanationSourceUse::BeforeCode;
    realized.paragraphs[1].citations.push(prior);
    // Eligible bytes cannot certify causality. This authored false interpretation
    // is intentionally structurally eligible and remains explicitly unapproved.
    realized.paragraphs[1].text =
        "The observation proves the entire application has no defects.".into();
    let new_claim = realized.paragraphs[1].text.clone();
    for c in &mut realized.paragraphs[1].citations {
        c.claim_start_byte = 0;
        c.claim_end_byte = new_claim.len();
        c.claim_sha256 = explanation_text_hash(&new_claim);
    }
    let mut continuation = realized.paragraphs[0].clone();
    continuation.text = "Additional reader context remains an interpretation.".into();
    realized.paragraphs.insert(1, continuation);
    let args = f.record_args(&generation, &realized)?;
    let recorded = success(&mut f.host, args)?;
    assert_eq!(
        recorded["explanation"]["semantic_approval_status"],
        "not_established"
    );
    let retained = f
        .host
        .operations()
        .privacy_status(f.project)?
        .managed_derived
        .into_iter()
        .find(|r| r.purpose.starts_with("explanation:"))
        .ok_or("record")?;
    assert!(retained.retained_until.is_some());
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let projection = f.host.operations().project_projection(f.project)?;
    let work = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("Work")?;
    let reading = work_answers(work, "en", FixedLocale::English);
    assert_ne!(reading.explanation_state, ExplanationState::Current);
    assert!(reading.provenance.is_none());
    Ok(())
}

#[test]
fn source_scope_and_command_failure_remain_actual_observations() -> Result<(), Box<dyn Error>> {
    let mut f = Fixture::new()?;
    let generation = f.generate()?;
    let read = f.read(&generation, f.source)?;
    let analysis = volicord_operations::AnalysisSnapshotId::from_hex(
        read["observation"]["analysis_snapshot_id"]
            .as_str()
            .ok_or("analysis")?,
    )?;
    let command = f
        .host
        .operations()
        .run_work_command(volicord_operations::WorkCommandRequest {
            project_id: f.project,
            work_item_id: f.work,
            analysis_snapshot_id: analysis,
            command_label: "observed failed experiment".into(),
            program: "python3".into(),
            arguments: vec![
                "-c".into(),
                "import sys; print('failure evidence'); sys.exit(3)".into(),
            ],
            timeout: std::time::Duration::from_secs(5),
            retain_seconds: 3600,
        })?
        .source_id;
    let observed = f.read(&generation, command)?;
    assert_eq!(observed["observation"]["detail"]["exit_code"], 3);
    // Resolve the command locator through the same public, Work-scoped read
    // available to the generating host, rather than copying the producer input.
    let metadata = f
        .host
        .handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"canonical_inspect","arguments":{"project_id":f.project.to_string(),
        "work_item_id":f.work.to_string(),"record_kind":"source","record_id":command.to_string(),
        "revision":1,"field":"observation"}}}))
        .ok_or("metadata response")?;
    assert_eq!(metadata["result"]["isError"], false, "{metadata}");
    let observation: Value = serde_json::from_str(
        metadata["result"]["structuredContent"]["chunk"]
            .as_str()
            .ok_or("metadata chunk")?,
    )?;
    let mut realized = response(&generation, &read)?;
    let mut citation = realized.paragraphs[1].citations[0].clone();
    let basis = &observed["citation_basis"];
    citation.source_id = command.to_string();
    citation.observation_fingerprint = basis["observation_fingerprint"]
        .as_str()
        .ok_or("fingerprint")?
        .into();
    citation.analysis_snapshot_id = basis["analysis_snapshot_id"]
        .as_str()
        .ok_or("analysis")?
        .into();
    citation.repository_snapshot_id = basis["repository_snapshot_id"]
        .as_str()
        .ok_or("repository")?
        .into();
    citation.repository_source_id = basis["repository_source_id"]
        .as_str()
        .ok_or("repository Source")?
        .into();
    citation.locator = observation["payload"]["command_label"]
        .as_str()
        .ok_or("command locator")?
        .into();
    citation.field = "stdout".into();
    citation.use_kind = ExplanationSourceUse::ObservedExecution;
    let output = observed["observation"]["detail"]["stdout"]["body"]
        .as_str()
        .ok_or("stdout")?;
    citation.start_byte = 0;
    citation.end_byte = output.len();
    citation.range_sha256 = explanation_text_hash(output);
    citation.start_line = 1;
    citation.end_line = 1;
    realized.paragraphs[3].text="The observed experiment exited with code 3; this is a failure, not implementation verification.".into();
    citation.claim_start_byte = 0;
    citation.claim_end_byte = realized.paragraphs[3].text.len();
    citation.claim_sha256 = explanation_text_hash(&realized.paragraphs[3].text);
    realized.paragraphs[3].citations = vec![citation];
    let args = f.record_args(&generation, &realized)?;
    success(&mut f.host, args)?;
    let previous = f
        .host
        .operations()
        .privacy_status(f.project)?
        .managed_derived;
    let negative = f.generate()?;
    f.read(&negative, f.source)?;
    f.read(&negative, command)?;
    for wrong_role in [false, true] {
        let mut invalid = realized.clone();
        if wrong_role {
            invalid.paragraphs[3].citations[0].use_kind = ExplanationSourceUse::CurrentCode;
        } else {
            invalid.paragraphs[3].citations[0].locator = "unobserved command label".into();
        }
        let args = f.record_args(&negative, &invalid)?;
        assert_eq!(call(&mut f.host, args)["result"]["isError"], true);
        assert_eq!(
            previous,
            f.host
                .operations()
                .privacy_status(f.project)?
                .managed_derived
        );
    }
    // Separate unsupported-language callers are permitted without a parser.
    let custom = f
        .host
        .operations()
        .observe_work_file(FileObservationRequest {
            byte_range: None,
            project_id: f.project,
            work_item_id: f.work,
            analysis_snapshot_id: analysis,
            path: "caller.custom".into(),
            before_source_id: None,
            retain_seconds: 3600,
        })?
        .source_id;
    let next = f.generate()?;
    assert_eq!(
        f.read(&next, custom)?["observation"]["detail"]["body"],
        "value()\n"
    );
    let other = f
        .host
        .operations()
        .record_current_host_user_context(
            f.project,
            "fixture".into(),
            "other".into(),
            "Separate Work".into(),
            ContextItemRole::Goal,
            "Separate Work".into(),
        )?
        .context_item_id;
    let mut args = f.args("read_source");
    args["work_item_id"] = json!(other.to_string());
    args["generation_id"] = next["generation_id"].clone();
    args["source_id"] = json!(f.source.to_string());
    assert_eq!(call(&mut f.host, args)["result"]["isError"], true);
    Ok(())
}

#[test]
#[cfg(target_os = "linux")]
fn redirected_current_source_root_cannot_publish_identical_bytes() -> Result<(), Box<dyn Error>> {
    let mut f = Fixture::new()?;
    let generation = f.generate()?;
    let read = f.read(&generation, f.source)?;
    let valid = response(&generation, &read)?;
    let root = f.temp.path().join("repo");
    let moved = f.temp.path().join("moved");
    fs::rename(&root, &moved)?;
    std::os::unix::fs::symlink(&moved, &root)?;
    let args = f.record_args(&generation, &valid)?;
    assert_eq!(call(&mut f.host, args)["result"]["isError"], true);
    assert!(f
        .host
        .operations()
        .privacy_status(f.project)?
        .managed_derived
        .iter()
        .all(|r| !r.purpose.starts_with("explanation:")));
    Ok(())
}

#[test]
fn cited_code_is_utf8_bounded_deduplicated_and_counts_omissions() -> Result<(), Box<dyn Error>> {
    for excerpt in [false, true] {
        cited_code_bounds(excerpt)?;
    }
    Ok(())
}
fn cited_code_bounds(excerpt: bool) -> Result<(), Box<dyn Error>> {
    let mut f = Fixture::new()?;
    let body = format!("def value():\n    return 7 # {}\n", "한".repeat(4000));
    let prefix = if excerpt {
        "# context\n".repeat(2000)
    } else {
        String::new()
    };
    fs::write(
        f.temp.path().join("repo/main.py"),
        format!("{prefix}{body}"),
    )?;
    let analysis = f
        .host
        .operations()
        .analyze(f.project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    f.source = f
        .host
        .operations()
        .observe_work_file(FileObservationRequest {
            byte_range: excerpt.then_some(volicord_operations::FileByteRange {
                start_byte: prefix.len(),
                end_byte: prefix.len() + body.len(),
            }),
            project_id: f.project,
            work_item_id: f.work,
            analysis_snapshot_id: analysis,
            path: "main.py".into(),
            before_source_id: None,
            retain_seconds: 3600,
        })?
        .source_id;
    let generation = f.generate()?;
    let read = f.read(&generation, f.source)?;
    let mut realized = response(&generation, &read)?;
    let base = realized.paragraphs[1].citations[0].clone();
    for offset in [0, 13, 26, 28, 31, 34, 37] {
        let mut c = base.clone();
        c.start_byte = prefix.len() + offset;
        c.start_line = if excerpt { 2000 } else { 0 } + if offset == 0 { 1 } else { 2 };
        c.range_sha256 = explanation_text_hash(&body[offset..]);
        realized.paragraphs[1].citations.push(c);
    }
    let args = f.record_args(&generation, &realized)?;
    success(&mut f.host, args)?;
    let projection = f.host.operations().project_projection(f.project)?;
    let w = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("work")?;
    let answers = work_answers(w, "en", FixedLocale::English);
    assert_eq!(answers.explanation_state, ExplanationState::Current);
    let used: usize = answers.source_excerpts.iter().map(|e| e.text.len()).sum();
    assert!(used <= 16_384);
    assert!(answers.omitted_source_excerpt_count > 0);
    let mut keys = std::collections::BTreeSet::new();
    for e in &answers.source_excerpts {
        assert!(e.text.len() <= 4096);
        assert_eq!(
            e.text,
            body[e.citation.start_byte - prefix.len()..e.displayed_end_byte - prefix.len()]
        );
        assert_eq!(
            e.omitted_utf8_bytes,
            e.citation.end_byte - e.displayed_end_byte
        );
        assert_eq!(e.displayed_end_line, if excerpt { 2002 } else { 2 });
        assert!(keys.insert((e.citation.start_byte, e.citation.end_byte)));
    }
    assert_eq!(keys.len() + answers.omitted_source_excerpt_count, 7);
    Ok(())
}

#[test]
fn large_file_excerpt_mcp_citations_use_independent_absolute_coordinates(
) -> Result<(), Box<dyn Error>> {
    let mut f = Fixture::new()?;
    // 26,000 bytes / 1,000 lines before the mechanism. Unicode crosses read chunks.
    let prefix = "# surrounding 한 context\n".repeat(1000);
    let mechanism = "def value():\n    return 7 # 한\n";
    let file = format!("{prefix}{mechanism}# unobserved tail\n");
    assert!(file.len() > 16_384);
    let start = prefix.len();
    let end = start + mechanism.len();
    fs::write(f.temp.path().join("repo/main.py"), &file)?;
    let analysis = f
        .host
        .operations()
        .analyze(f.project, vec![])?
        .value
        .ok_or("analysis")?;
    let mut request = json!({"action":"observe_file","project_id":f.project.to_string(),
        "work_item_id":f.work.to_string(),"analysis_snapshot_id":analysis.analysis.identity.to_string(),
        "path":"main.py","retain_seconds":3600});
    let source_call = |host: &mut HostAdapter, args: Value| {
        host.handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"work_source","arguments":args}})).unwrap_or(Value::Null)
    };
    // Reproduce the whole-file limitation through the actual Product adapter.
    assert_eq!(
        source_call(&mut f.host, request.clone())["result"]["isError"],
        true
    );
    let canonical_before = f.host.operations().canonical_basis(f.project)?;
    for range in [
        json!({"start_byte":end,"end_byte":start}),
        json!({"start_byte":-1,"end_byte":end}),
        json!({"start_byte":1.5,"end_byte":end}),
        json!({"start_byte":start,"end_byte":end,"unapproved":true}),
    ] {
        request["byte_range"] = range;
        assert_eq!(
            source_call(&mut f.host, request.clone())["result"]["isError"],
            true
        );
    }
    assert_eq!(
        canonical_before,
        f.host.operations().canonical_basis(f.project)?
    );
    request["byte_range"] = json!({"start_byte":start,"end_byte":end});
    let receipt = source_call(&mut f.host, request.clone());
    assert_eq!(receipt["result"]["isError"], false, "{receipt}");
    let id = format!(
        "{:0>32}",
        receipt["result"]["structuredContent"]["source_id"]
            .as_str()
            .ok_or("source")?
    );
    let bytes = (0..32)
        .step_by(2)
        .map(|i| u8::from_str_radix(&id[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()?;
    f.source = serde_json::from_value(json!(bytes))?;
    // The supported Source reader supplies exactly the authorized region, no prefix/tail.
    let generation = f.generate()?;
    let before_reads = f.host.operations().privacy_status(f.project)?;
    let read = f.read(&generation, f.source)?;
    assert_eq!(read["observation"]["detail"]["body"], mechanism);
    assert!(!read.to_string().contains("surrounding"));
    assert!(!read.to_string().contains("unobserved tail"));
    assert_eq!(
        read["observation"]["detail"]["representation"],
        "bounded_excerpt"
    );
    let region = &read["observation"]["detail"]["region"];
    assert_eq!(region["start_byte"], 26_000);
    assert_eq!(region["end_byte"], 26_032);
    assert_eq!(region["start_line"], 1001);
    assert_eq!(region["end_line"], 1002);
    assert_eq!(region["file_utf8_bytes"], file.len());
    assert_eq!(
        read["observation"]["detail"]["sha256"],
        explanation_text_hash(&file).trim_start_matches("sha256:")
    );
    assert_eq!(f.host.operations().privacy_status(f.project)?, before_reads);
    let mut realized = response(&generation, &read)?;
    let c = &mut realized.paragraphs[1].citations[0];
    c.start_byte = start + 13;
    c.end_byte = end;
    c.start_line = 1002;
    c.end_line = 1002;
    for variant in 0..7 {
        let mut bad = realized.clone();
        let c = &mut bad.paragraphs[1].citations[0];
        match variant {
            0 => {
                c.start_byte = 13;
                c.end_byte = mechanism.len();
                c.start_line = 2;
                c.end_line = 2;
            }
            1 => c.start_byte = start - 1,
            2 => c.end_byte = end + 1,
            3 => c.start_line = 2,
            4 => c.use_kind = ExplanationSourceUse::BeforeCode,
            5 => c.range_sha256 = explanation_text_hash("altered excerpt"),
            _ => c.end_byte = end - 2, // Cuts the final three-byte character.
        }
        let args = f.record_args(&generation, &bad)?;
        assert_eq!(
            call(&mut f.host, args)["result"]["isError"],
            true,
            "variant {variant}"
        );
    }
    let args = f.record_args(&generation, &realized)?;
    success(&mut f.host, args)?;
    let projection = f.host.operations().project_projection(f.project)?;
    let w = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("work")?;
    let answers = work_answers(w, "en", FixedLocale::English);
    assert_eq!(answers.explanation_state, ExplanationState::Current);
    assert_eq!(answers.source_excerpts[0].text, &mechanism[13..]);
    assert_eq!(answers.source_excerpts[0].displayed_end_byte, end);
    assert_eq!(answers.source_excerpts[0].displayed_end_line, 1002);
    assert_eq!(
        answers.source_excerpts[0].observation_region.as_ref(),
        Some(region)
    );
    // Even changes outside the retained excerpt invalidate current evidence.
    fs::write(
        f.temp.path().join("repo/main.py"),
        file.replacen("surrounding", "SURROUNDING", 1),
    )?;
    assert_eq!(source_call(&mut f.host, request)["result"]["isError"], true);
    let projection = f.host.operations().project_projection(f.project)?;
    let w = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == f.work)
        .ok_or("work")?;
    assert_eq!(
        work_answers(w, "en", FixedLocale::English).explanation_state,
        ExplanationState::Stale
    );
    // Historical retained read never substitutes the changed filesystem version.
    assert_eq!(
        f.host
            .operations()
            .read_explanation_source(f.project, f.work, f.source)?["observation"]["detail"]["body"],
        mechanism
    );
    f.host
        .operations()
        .delete_work_source_observation(f.project, f.work, f.source)?;
    assert!(f
        .host
        .operations()
        .read_explanation_source(f.project, f.work, f.source)
        .is_err());
    Ok(())
}
