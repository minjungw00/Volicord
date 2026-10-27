//! Authored semantic expectations; real Product storage and public consumers.
//! Test prose is structural support, never active-host or human qualification.
use serde_json::{json, Value};
use std::{error::Error, fs, path::Path};
use volicord_context::*;
use volicord_host::HostAdapter;
use volicord_inquiry::{
    CandidateCollectionMode, CandidateCollectionScope, CandidateContent, CandidateDraft,
    CandidateKind, CandidateObservationBasis, CandidateOrigin, CandidateRetention,
};
use volicord_operations::{
    run_cli, CliExit, FileObservationRequest, LocalOperations, RuntimeLayout,
};
use volicord_projections::*;
use volicord_viewer::{ViewerAdapter, ViewerLocale, ViewerRequest, ViewerServer, ViewerView};

#[path = "support/code_text.rs"]
mod code_text;
use code_text::code_texts;

fn seed(root: &Path, comparison: &str) -> Result<Value, Box<dyn Error>> {
    seed_with_goal(root, comparison, "Explain why value returns 5, using observed code and any retained comparison. Do not infer execution success.")
}
fn seed_with_goal(root: &Path, comparison: &str, goal: &str) -> Result<Value, Box<dyn Error>> {
    let repo = root.join("repo");
    fs::create_dir_all(&repo)?;
    fs::write(repo.join("main.py"), "def value():\n    return 4\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(root.join("runtime"))?);
    let project = ops
        .initialize_project("Understand a constant return", Some(&repo))?
        .project
        .id;
    let work = ops
        .record_current_host_user_context(
            project,
            "authored-test-input".into(),
            "reader-fixture".into(),
            goal.into(),
            ContextItemRole::Goal,
            goal.into(),
        )?
        .context_item_id;
    let old_analysis = ops
        .analyze(project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let before = if comparison == "missing_before_state" {
        None
    } else {
        if comparison == "no_change" {
            fs::write(repo.join("main.py"), "def value():\n    return 5\n")?;
        }
        let analysis = if comparison == "no_change" {
            ops.analyze(project, vec![])?
                .value
                .ok_or("analysis")?
                .analysis
                .identity
        } else {
            old_analysis
        };
        Some(
            ops.observe_work_file(FileObservationRequest {
                byte_range: None,
                project_id: project,
                work_item_id: work,
                analysis_snapshot_id: analysis,
                path: "main.py".into(),
                before_source_id: None,
                retain_seconds: 604800,
            })?
            .source_id,
        )
    };
    fs::write(repo.join("main.py"), "def value():\n    return 5\n")?;
    let analysis = ops
        .analyze(project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let after = ops
        .observe_work_file(FileObservationRequest {
            byte_range: None,
            project_id: project,
            work_item_id: work,
            analysis_snapshot_id: analysis,
            path: "main.py".into(),
            before_source_id: before,
            retain_seconds: 604800,
        })?
        .source_id;
    Ok(
        json!({"runtime":ops.layout().root(),"repository":repo,"project":project.to_string(),
        "work":work.to_string(),"goal":goal,"before":before.map(|id|id.to_string()),"after":after.to_string(),
        "comparison":comparison,"input_provenance":"authored-test-support","host_output":"not_generated"}),
    )
}
fn ops(f: &Value) -> Result<LocalOperations, Box<dyn Error>> {
    Ok(LocalOperations::new(RuntimeLayout::new(
        f["runtime"].as_str().ok_or("runtime")?,
    )?))
}
fn project(f: &Value) -> Result<ProjectId, Box<dyn Error>> {
    Ok(serde_json::from_value(json!(hex_bytes(
        f["project"].as_str().ok_or("project")?
    )?))?)
}
fn hex_bytes(id: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let id = format!("{id:0>32}");
    (0..id.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&id[i..i + 2], 16).map_err(Into::into))
        .collect()
}
fn work(f: &Value) -> Result<ContextItemId, Box<dyn Error>> {
    Ok(serde_json::from_value(json!(hex_bytes(
        f["work"].as_str().ok_or("work")?
    )?))?)
}
fn call(host: &mut HostAdapter, tool: &str, args: Value) -> Result<Value, Box<dyn Error>> {
    let result = host
        .handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":tool,"arguments":args}}))
        .ok_or("response")?;
    if result["result"]["isError"] != false {
        return Err(result.to_string().into());
    }
    Ok(result["result"]["structuredContent"].clone())
}
fn record(f: &Value, host: &mut HostAdapter) -> Result<(), Box<dyn Error>> {
    record_with_purpose(f, host, "Understand the constant return in value().")
}
fn record_with_purpose(
    f: &Value,
    host: &mut HostAdapter,
    purpose: &str,
) -> Result<(), Box<dyn Error>> {
    record_in_language(f, host, purpose, "en")
}
fn record_in_language(
    f: &Value,
    host: &mut HostAdapter,
    purpose: &str,
    language: &str,
) -> Result<(), Box<dyn Error>> {
    let base = json!({"project_id":f["project"],"work_item_id":f["work"],"language":language,"action":"generate"});
    let generation = call(host, "work_explanation", base.clone())?;
    let plan: ExplanationPlan = serde_json::from_value(generation["plan"].clone())?;
    let mut citations = Vec::new();
    let text = if language == "ko" {
        "관측된 함수는 상수 5를 반환합니다. 실행과 사용자 수락은 확인되지 않았습니다."
    } else {
        "The observed function returns the constant 5. Execution and user acceptance are not established."
    };
    let claim_end = text.find('.').ok_or("first claim")? + 1;
    for (source, use_kind) in [
        (f["after"].clone(), ExplanationSourceUse::CurrentCode),
        (f["before"].clone(), ExplanationSourceUse::BeforeCode),
    ] {
        if source.is_null() {
            continue;
        }
        let read = call(
            host,
            "work_explanation",
            json!({"action":"read_source","project_id":f["project"],
            "work_item_id":f["work"],"generation_id":generation["generation_id"],"source_id":source}),
        )?;
        let mut c = read["citation_basis"].clone();
        let body = read["observation"]["detail"]["body"]
            .as_str()
            .ok_or("body")?;
        c["locator"] = json!("main.py");
        c["field"] = json!("body");
        let region = &read["observation"]["detail"]["region"];
        c["start_byte"] = json!(region["start_byte"].as_u64().unwrap_or(0));
        c["end_byte"] = json!(region["end_byte"].as_u64().unwrap_or(body.len() as u64));
        c["start_line"] = json!(region["start_line"].as_u64().unwrap_or(1));
        c["end_line"] = json!(region["end_line"].as_u64().unwrap_or(2));
        c["range_sha256"] = json!(explanation_text_hash(body));
        c["claim_start_byte"] = json!(0);
        c["claim_end_byte"] = json!(claim_end);
        c["claim_sha256"] = json!(explanation_text_hash(&text[..claim_end]));
        c["use_kind"] = serde_json::to_value(use_kind)?;
        citations.push(serde_json::from_value(c)?);
    }
    let realization = ExplanationRealization {
        diagrams: Vec::new(),
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint,
        language: language.into(),
        generator: ExplanationGenerator {
            host: "authored-structural-control".into(),
            session: "reader-test".into(),
            agent: None,
            model: None,
        },
        paragraphs: [
            (ExplanationQuestion::Purpose, "goal", purpose),
            (ExplanationQuestion::ReportedChange, "goal", text),
            (
                ExplanationQuestion::ExpectedEffect,
                "goal",
                if language == "ko" {
                    "실행 중 효과는 검증되지 않았습니다."
                } else {
                    "No runtime effect has been verified."
                },
            ),
            (
                ExplanationQuestion::Verification,
                "goal",
                if language == "ko" {
                    "실행 검증은 기록되지 않았습니다."
                } else {
                    "No execution verification was recorded."
                },
            ),
            (
                ExplanationQuestion::NextStep,
                "next_step",
                if language == "ko" {
                    "다음 행동은 기록되지 않았습니다."
                } else {
                    "No next action has been recorded."
                },
            ),
        ]
        .into_iter()
        .map(|(question, key, text)| ExplanationParagraph {
            question,
            text: text.into(),
            evidence_keys: vec![key.into()],
            citations: if question == ExplanationQuestion::ReportedChange {
                citations.clone()
            } else {
                vec![]
            },
        })
        .collect(),
    };
    call(
        host,
        "work_explanation",
        json!({"action":"record","project_id":f["project"],"work_item_id":f["work"],
        "generation_id":generation["generation_id"],"realization":realization}),
    )?;
    Ok(())
}
fn request(f: &Value) -> Result<ViewerRequest, Box<dyn Error>> {
    Ok(ViewerRequest {
        project_id: project(f)?,
        locale: ViewerLocale::English,
        view: ViewerView::Work {
            work: Some(work(f)?),
        },
        requested_language: "en".into(),
        guarded_request: None,
    })
}
fn get(f: &Value) -> Result<String, Box<dyn Error>> {
    let server = ViewerServer::new(
        ViewerAdapter::new(ops(f)?),
        project(f)?,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let http = format!(
        "GET /?view=work&work={} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n",
        f["work"].as_str().ok_or("work")?
    );
    let mut output = Vec::new();
    server.serve_connection(&mut http.as_bytes(), &mut output)?;
    Ok(String::from_utf8(output)?)
}

#[test]
fn overview_uses_purpose_and_recent_cards_without_empty_legacy_sections(
) -> Result<(), Box<dyn Error>> {
    for recorded in [false, true] {
        let temp = tempfile::tempdir()?;
        let capture = std::env::var_os("VOLICORD_OVERVIEW_CAPTURE").map(std::path::PathBuf::from);
        let root = capture
            .as_ref()
            .map(|root| root.join(if recorded { "recorded" } else { "goal-only" }));
        if let Some(root) = &root {
            assert!(
                root.is_absolute(),
                "capture requires an absolute ignored destination"
            );
        }
        let mut f = seed_with_goal(
            root.as_deref().unwrap_or(temp.path()),
            "missing_before_state",
            "Inspect the actual observed return value and its callers before explaining what changed, without assuming successful execution. 긴 목표도 원문 그대로 읽을 수 있어야 합니다.\n\nKeep missing verification and Checkpoint direction explicit.",
        )?;
        if recorded {
            record(&f, &mut HostAdapter::new(ops(&f)?))?;
            f["host_output"] = json!("authored-control-recorded");
        }
        if let Some(root) = &root {
            fs::write(root.join("fixture.json"), serde_json::to_vec_pretty(&f)?)?;
        }
        let before = ops(&f)?.canonical_basis(project(&f)?)?;
        assert!(before.checkpoint_history.is_empty());
        for (locale, key, missing_purpose) in [
            (
                ViewerLocale::English,
                "en",
                "No Project purpose has been recorded.",
            ),
            (
                ViewerLocale::Korean,
                "ko",
                "기록된 프로젝트 목적이 없습니다.",
            ),
        ] {
            let r = ViewerRequest {
                locale,
                view: ViewerView::Overview,
                ..request(&f)?
            };
            let adapter = ViewerAdapter::new(ops(&f)?);
            let server = ViewerServer::new(
                adapter,
                project(&f)?,
                locale,
                ViewerView::Overview,
                "en".into(),
                "127.0.0.1:3219".parse()?,
            )?;
            let http = format!("GET /?view=overview&locale={key}&language=en HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n");
            let mut bytes = Vec::new();
            server.serve_connection(&mut http.as_bytes(), &mut bytes)?;
            let live = String::from_utf8(bytes)?;
            assert!(live.starts_with("HTTP/1.1 200"));
            let snapshot = ViewerAdapter::new(ops(&f)?)
                .render_snapshot(&r, TimestampMicros::from_unix_micros(123))?
                .html;
            for (mode, page) in [("live", &live), ("snapshot", &snapshot)] {
                let overview = page
                    .split("id=\"overview\"")
                    .nth(1)
                    .ok_or("overview")?
                    .split("id=\"overview-issues\"")
                    .next()
                    .ok_or("overview end")?;
                let card = overview.find("data-work-id=").ok_or("available Work")?;
                assert!(
                    overview.find(missing_purpose).ok_or("missing Purpose")? < card,
                    "{mode}/{key}: reference purpose card precedes recent Work"
                );
                assert!(!overview.contains("id=\"overview-current\""));
                assert!(!overview.contains("id=\"overview-completed\""));
                assert!(!overview.contains("overview-jumps"));
                assert!(overview.contains("data-title-basis=\"neutral-label\""));
                assert!(
                    !overview.contains("data-question=\"Synopsis\""),
                    "Purpose-only controls cannot acquire a synthesized synopsis"
                );
                let card = overview
                    .split(if mode == "live" {
                        "class=\"work-row\""
                    } else {
                        "class=\"work-card\""
                    })
                    .nth(1)
                    .ok_or("Work entry")?
                    .split(if mode == "live" { "</a>" } else { "</article>" })
                    .next()
                    .ok_or("Work entry end")?;
                assert!(!card.contains(f["goal"].as_str().ok_or("original Goal")?));
                assert!(card.contains(if mode == "snapshot" {
                    "content-unavailable"
                } else if key == "en" {
                    "Work summary is unavailable"
                } else {
                    "작업 요약을 사용할 수 없습니다"
                }));
                assert!(overview.contains("data-work-state=\"open\""));
                assert!(!overview.contains("data-work-state=\"completed\""));
                assert!(page.contains(if mode == "live" {
                    "view=work&amp;work="
                } else {
                    "href=\"#work-"
                }));
                if let Some(root) = std::env::var_os("VOLICORD_OVERVIEW_CAPTURE") {
                    let root = std::path::PathBuf::from(root);
                    fs::create_dir_all(&root)?;
                    fs::write(
                        root.join(format!(
                            "{}-{key}-{mode}.html",
                            if recorded { "recorded" } else { "goal-only" }
                        )),
                        page,
                    )?;
                }
            }
        }
        assert_eq!(before, ops(&f)?.canonical_basis(project(&f)?)?);
    }
    Ok(())
}

#[test]
fn missing_checkpoint_direction_does_not_erase_goal_or_promote_advice() -> Result<(), Box<dyn Error>>
{
    let temp = tempfile::tempdir()?;
    let f = seed(temp.path(), "missing_before_state")?;
    record(&f, &mut HostAdapter::new(ops(&f)?))?;
    let op = ops(&f)?;
    let canonical = op.canonical_basis(project(&f)?)?;
    let projection = op.project_projection(project(&f)?)?;
    let selected = projection.selected_work.as_ref().ok_or("Work")?;
    let mut realization = selected.reading.explanations[0]
        .content
        .as_ref()
        .ok_or("current output")?
        .realization
        .clone();
    let advice = "Consider inspecting callers; this is an agent suggestion.";
    realization.generator.host = "authored-authority-control".into();
    realization
        .paragraphs
        .iter_mut()
        .find(|p| p.question == ExplanationQuestion::NextStep)
        .ok_or("next step")?
        .text = advice.into();
    op.record_explanation(
        project(&f)?,
        ExplanationSubject::Work(work(&f)?),
        "en",
        realization,
    )?;
    for stale in [false, true] {
        if stale {
            fs::write(
                Path::new(f["repository"].as_str().ok_or("repository")?).join("main.py"),
                "def value():\n    return 6\n",
            )?;
        }
        let projection = op.project_projection(project(&f)?)?;
        let selected = projection.selected_work.as_ref().ok_or("Work")?;
        let answers = work_answers(selected, "en", FixedLocale::English);
        assert!(answers.recorded_next_action().is_none());
        assert_eq!(
            answers.next_step_answer().ok_or("availability")?.question,
            "NextStepAvailability"
        );
        // Repository change does not erase independently retained host advice.
        assert!(answers.prose.iter().any(|a| a.text == advice));
        for (locale, _gap, _heading) in [
            (
                ViewerLocale::English,
                "No Checkpoint next action is recorded.",
                "Host interpretation: next steps / uncertainty",
            ),
            (
                ViewerLocale::Korean,
                "Checkpoint에 기록된 다음 행동이 없습니다.",
                "호스트 해석: 다음 단계 / 불확실성",
            ),
        ] {
            let mut r = request(&f)?;
            r.locale = locale;
            let adapter = ViewerAdapter::new(ops(&f)?);
            let detail = adapter.render(&r, "test-authenticity")?.html;
            r.view = ViewerView::Overview;
            let overview = adapter.render(&r, "test-authenticity")?.html;
            let snapshot = adapter
                .render_snapshot(&r, TimestampMicros::from_unix_micros(123))?
                .html;
            assert!(!overview.contains("data-question=\"RecordedNextStep\""));
            assert!(
                snapshot.contains(
                    &work_answers(
                        selected,
                        "en",
                        if locale == ViewerLocale::English {
                            FixedLocale::English
                        } else {
                            FixedLocale::Korean
                        }
                    )
                    .next_step_answer()
                    .ok_or("recorded direction")?
                    .text
                ),
                "the complete recorded direction remains available in document evidence"
            );
            assert!(!overview.contains("<blockquote class=\"work-summary\""));
            assert!(detail.contains("Explain why value returns 5"));
            for page in [&detail, &snapshot] {
                assert!(page.contains(advice));
                assert!(page.contains("data-question=\"NextStep\""));
            }
        }
    }
    assert_eq!(canonical, op.canonical_basis(project(&f)?)?);
    Ok(())
}

#[test]
fn observed_changed_equal_and_missing_before_code_reach_every_reader() -> Result<(), Box<dyn Error>>
{
    for comparison in ["changed", "no_change", "missing_before_state"] {
        let temp = tempfile::tempdir()?;
        let f = seed(temp.path(), comparison)?;
        let mut host = HostAdapter::new(ops(&f)?);
        record(&f, &mut host)?;
        let before = ops(&f)?.canonical_basis(project(&f)?)?;
        let privacy = ops(&f)?.privacy_status(project(&f)?)?;
        let projection = ops(&f)?.project_projection(project(&f)?)?;
        let selected = projection
            .work_history
            .iter()
            .find(|w| w.work_item_id == work(&f).unwrap())
            .ok_or("work")?;
        let answers = work_answers(selected, "en", FixedLocale::English);
        assert_eq!(answers.explanation_state, ExplanationState::Current);
        assert_eq!(
            answers.source_excerpts[0].text,
            "def value():\n    return 5\n"
        );
        assert_eq!(
            answers.source_excerpts[0].comparison.as_deref(),
            Some(comparison)
        );
        assert_eq!(answers.source_excerpts[0].work_revision, Some(1));
        assert_eq!(
            answers.source_excerpts.len(),
            if comparison == "missing_before_state" {
                1
            } else {
                2
            }
        );
        let fingerprint = &answers
            .provenance
            .as_ref()
            .ok_or("provenance")?
            .realization_fingerprint;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let args = [
            "--runtime",
            f["runtime"].as_str().ok_or("runtime")?,
            "--project",
            f["project"].as_str().ok_or("project")?,
            "status",
            "--json",
        ];
        assert_eq!(run_cli(args, &mut stdout, &mut stderr), CliExit::SUCCESS);
        assert!(String::from_utf8(stdout)?.contains(fingerprint));
        let mcp = call(
            &mut host,
            "repository_understanding",
            json!({"project_id":f["project"]}),
        )?;
        assert!(mcp.to_string().contains(fingerprint));
        assert!(mcp.to_string().contains("return 5"));
        let page = get(&f)?;
        assert!(code_texts(&page)
            .iter()
            .any(|s| s == "def value():\n    return 5\n"));
        assert!(page.contains("data-title-basis=\"neutral-label\" lang=\"en\">Recorded Work</h1>"));
        assert_eq!(
            page.matches("Understand the constant return in value().")
                .count(),
            2 // article plus exact evidence-index quotation behind disclosure
        );
        assert!(page.contains(
            "aria-label=\"Inspect The observed function returns the constant 5. · Observed file in selected snapshot · Source 1 ↗ · main.py:1–2\""
        ));
        assert!(page.contains("<small class=\"source-label\">main.py:1–2</small>"));
        assert!(page.contains("Observed file in selected snapshot"));
        assert_eq!(
            page.contains("Before: historical observation"),
            comparison != "missing_before_state"
        );
        let mut korean_request = request(&f)?;
        korean_request.locale = ViewerLocale::Korean;
        let korean = ViewerAdapter::new(ops(&f)?)
            .render(&korean_request, "test-authenticity")?
            .html;
        assert!(korean.contains("선택 스냅샷에서 관측한 파일 · 근거 1 ↗"));
        assert!(korean.contains("선택 스냅샷에서 관측한 파일"));
        assert!(
            page.find("class=\"record-facts\"").ok_or("facts")?
                < page.find("class=\"appendix\"").ok_or("sources")?
        );
        assert!(page.contains("class=\"claim-quote\" tabindex=\"0\" lang=\"en\">The observed function returns the constant 5.</blockquote>"));
        assert!(page.contains("<pre class=\"code-region\" tabindex=\"0\" role=\"region\""));
        assert_eq!(
            page.contains("Read the retained before observation"),
            comparison != "missing_before_state"
        );
        assert!(page.contains(fingerprint));
        let code_request = request(&f)?;

        let code = ViewerAdapter::new(ops(&f)?)
            .render(&code_request, "test-form-authenticity")?
            .html;
        assert!(code_texts(&code)
            .iter()
            .any(|s| s == "def value():\n    return 5\n"));
        assert!(code
            .contains("data-question=\"ReportedChange\" lang=\"en\" data-content-language=\"en\""));
        assert!(code.contains(fingerprint));
        let destination = temp.path().join("snapshot.html");
        ViewerAdapter::new(ops(&f)?).export_snapshot(&request(&f)?, &destination)?;
        let snapshot = fs::read_to_string(destination)?;
        assert!(snapshot.contains(fingerprint));
        assert!(code_texts(&snapshot).iter().any(|s| s.contains("return 5")));
        assert!(!snapshot.contains("<script"));
        assert!(!snapshot.contains("<form"));
        assert_eq!(before, ops(&f)?.canonical_basis(project(&f)?)?);
        let after_reads = ops(&f)?.privacy_status(project(&f)?)?;
        assert_eq!(privacy, after_reads);
        fs::write(
            temp.path().join("repo/main.py"),
            "def value():\n    return 6\n",
        )?;
        let stale = get(&f)?;
        assert!(code_texts(&stale).iter().any(|s| s.contains("return 5")));
        assert!(!code_texts(&stale).iter().any(|s| s.contains("return 6")));
        assert!(stale.contains("data-comparison-state=\"changed\""));
    }
    Ok(())
}

// Comparison instants belong to each read, outside retained publication identity.
// Preserve every other field, including typed applicability and exact claim/Source
// bindings; missing or invalid verified-read times remain assertion failures.
fn assert_same_answer_basis(mut left: Value, mut right: Value) {
    fn normalize(value: &mut Value) {
        let applicability = &mut value["applicability"];
        if applicability["repository_comparison"] == "verified_at_read" {
            assert!(applicability["checked_at_unix_micros"]
                .as_i64()
                .is_some_and(|v| v > 0));
        }
        assert!(
            applicability["checked_at_unix_micros"].is_null()
                || applicability["checked_at_unix_micros"]
                    .as_i64()
                    .is_some_and(|v| v > 0)
        );
        applicability["checked_at_unix_micros"] = Value::Null;
        for scope in ["work_name", "work_summary"] {
            if value[scope].is_object() {
                normalize(&mut value[scope]);
            }
        }
    }
    normalize(&mut left);
    normalize(&mut right);
    assert_eq!(
        left, right,
        "same public Work publication and applicability basis"
    );
}

fn public_answers(f: &Value, host: &mut HostAdapter) -> Result<Value, Box<dyn Error>> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                f["runtime"].as_str().ok_or("runtime")?,
                "--project",
                f["project"].as_str().ok_or("project")?,
                "--json",
                "status",
            ],
            &mut stdout,
            &mut stderr,
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&stderr)
    );
    let cli: Value = serde_json::from_slice(&stdout)?;
    let mcp = call(
        host,
        "repository_understanding",
        json!({"project_id":f["project"]}),
    )?;
    let select = |result: &Value| -> Result<Value, Box<dyn Error>> {
        Ok(result["work_history"]
            .as_array()
            .ok_or("work history")?
            .iter()
            .find(|w| w["work_item_id"] == f["work"])
            .ok_or("selected work")?["answers"]
            .clone())
    };
    let answers = select(&cli)?;
    assert_same_answer_basis(answers.clone(), select(&mcp)?);
    Ok(answers)
}

fn denied_public_readers(
    f: &Value,
    host: &mut HostAdapter,
    state: &str,
    label: &str,
    output: &Path,
) -> Result<(), Box<dyn Error>> {
    let answers = public_answers(f, host)?;
    assert_eq!(answers["explanation_state"], state);
    let prose = answers["prose"].as_array().ok_or("prose")?;
    assert_eq!(prose.len(), 1, "only the lifecycle notice remains");
    assert_eq!(prose[0]["question"], "ExplanationAvailability");
    assert_eq!(prose[0]["citations"], json!([]));
    assert_eq!(answers["source_excerpts"], json!([]));
    assert!(answers["provenance"].is_null());
    let page = get(f)?;
    ViewerAdapter::new(ops(f)?).export_snapshot(&request(f)?, output)?;
    for html in [page, fs::read_to_string(output)?] {
        assert!(html.contains(label), "missing lifecycle notice: {label}");
        assert!(
            html.contains("Explain why value returns 5"),
            "canonical Goal remains readable"
        );
        assert!(!html.contains("The observed function returns the constant 5."));
        assert!(!html.contains("class=\"source-excerpt\""));
    }
    Ok(())
}

#[test]
fn corrupt_and_unsupported_latest_outputs_never_fall_back_across_public_readers(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let f = seed(temp.path(), "missing_before_state")?;
    let mut host = HostAdapter::new(ops(&f)?);
    record(&f, &mut host)?;
    let operations = ops(&f)?;
    let project = project(&f)?;
    let work = work(&f)?;
    let canonical = operations.canonical_basis(project)?;
    let grounding =
        volicord_repository_intelligence::CanonicalGrounding::from_read_basis(&canonical)?;
    let source = canonical
        .context_items
        .iter()
        .find(|c| c.id == work)
        .ok_or("goal")?
        .source_basis[0];
    let original = public_answers(&f, &mut host)?;
    assert_eq!(original["explanation_state"], "current");
    assert!(!original["source_excerpts"]
        .as_array()
        .ok_or("excerpts")?
        .is_empty());
    for (state, label, content) in [
        (
            "corrupt",
            "Retained content is corrupt.",
            "not JSON".to_owned(),
        ),
        (
            "unsupported",
            "The retained format is unsupported.",
            json!({"realization": {
                "format_kind": EXPLANATION_KIND, "format_version": EXPLANATION_VERSION + 1
            }})
            .to_string(),
        ),
    ] {
        // A newer disposable output must hide the older valid one. The malformed
        // bodies are independently authored, not encoded by the explanation producer.
        let mut privacy =
            volicord_privacy::PrivacyStore::open(operations.layout().privacy_store())?;
        privacy.record_managed_derived(volicord_privacy::ManagedDerivedDraft {
            project_id: project,
            kind: volicord_privacy::ManagedDerivedKind::CachedSummary,
            provider: None,
            model: None,
            purpose: format!("explanation:work:{work}:en"),
            analysis_snapshot: None,
            included_sources: vec![grounding.source_reference(source)?],
            canonical_links: vec![volicord_privacy::ManagedCanonicalLink::ContextItem(work)],
            content,
            uncertainty: None,
            retained_until: None,
            retention_basis: "Authored invalid Derived output; no model or human assessment".into(),
        })?;
        drop(privacy);
        let before = operations.privacy_status(project)?;
        denied_public_readers(
            &f,
            &mut host,
            state,
            label,
            &temp.path().join(format!("{state}.html")),
        )?;
        assert_eq!(
            before,
            operations.privacy_status(project)?,
            "reads cannot repair, replace or renew a bad output"
        );
        assert_eq!(canonical, operations.canonical_basis(project)?);
        record(&f, &mut host)?;
        let recovered = public_answers(&f, &mut host)?;
        for field in ["prose", "source_excerpts", "recorded_next_step"] {
            assert_eq!(recovered[field], original[field], "recovered {field}");
        }
        assert_eq!(recovered["explanation_state"], "current");
        assert_eq!(
            recovered["provenance"]["realization_fingerprint"],
            original["provenance"]["realization_fingerprint"]
        );
        assert_eq!(
            operations.privacy_status(project)?.managed_derived.len(),
            before.managed_derived.len() + 1,
            "recovery retains original attempts instead of rewriting history"
        );
    }
    Ok(())
}

#[test]
fn unavailable_explanation_storage_preserves_canonical_reading_and_recovers_without_generation(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let f = seed(temp.path(), "missing_before_state")?;
    let mut host = HostAdapter::new(ops(&f)?);
    record(&f, &mut host)?;
    let operations = ops(&f)?;
    let project = project(&f)?;
    let canonical = operations.canonical_basis(project)?;
    let privacy = operations.privacy_status(project)?;
    let original = public_answers(&f, &mut host)?;
    let database = operations.layout().privacy_store();
    let hidden = temp.path().join("original-privacy.sqlite3");
    fs::rename(&database, &hidden)?;
    let unavailable = b"Authored unreadable SQLite fixture";
    fs::write(&database, unavailable)?;
    let result = denied_public_readers(
        &f,
        &mut host,
        "unavailable",
        "Explanation is unavailable",
        &temp.path().join("unavailable.html"),
    );
    let retained = fs::read(&database)?;
    fs::remove_file(&database)?;
    fs::rename(&hidden, &database)?;
    result?;
    assert_eq!(
        retained, unavailable,
        "read failures do not replace the unavailable store"
    );
    assert_eq!(canonical, operations.canonical_basis(project)?);
    assert_eq!(privacy, operations.privacy_status(project)?);
    assert_same_answer_basis(public_answers(&f, &mut host)?, original);
    Ok(())
}

#[test]
fn long_original_explanation_is_retained_once_below_a_goal_title() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let f = seed(temp.path(), "missing_before_state")?;
    let mut host = HostAdapter::new(ops(&f)?);
    // Independently authored long prose checks retention and hierarchy only.
    // Its repeated paragraphs deliberately must not be silently deduplicated.
    let purpose = format!(
        "{}\n\n{}\n\nNo execution or user rationale is supplied.",
        "Authored explanation of the observed return and its limits. ".repeat(18),
        "Authored explanation of the observed return and its limits. ".repeat(18)
    );
    record_with_purpose(&f, &mut host, &purpose)?;
    let before = ops(&f)?.canonical_basis(project(&f)?)?;
    let page = get(&f)?;
    assert_eq!(
        page.matches(&format!("data-content-language=\"en\">{purpose}</p>"))
            .count(),
        1,
        "ordinary prose occurs once; Source panels quote their exact bound claims separately"
    );
    assert!(!page.contains(&format!("{purpose}</h3>")));
    assert!(page.contains("data-title-basis=\"neutral-label\""));
    assert!(page.contains(
        "Host interpretation; semantic accuracy has not been independently established."
    ));
    let projection =
        ops(&f)?.project_projection_selected(project(&f)?, WorkSelector::ExactWork(work(&f)?))?;
    let answers = work_answers(
        projection.selected_work.as_ref().ok_or("selected Work")?,
        "en",
        FixedLocale::English,
    );
    assert_eq!(answers.prose[0].text, purpose);
    assert_eq!(
        answers.source_excerpts[0].text,
        "def value():\n    return 5\n"
    );
    assert_eq!(before, ops(&f)?.canonical_basis(project(&f)?)?);
    if let Ok(output) = std::env::var("VOLICORD_LONG_READER_OUTPUT") {
        let output = Path::new(&output);
        fs::create_dir_all(output)?;
        let snapshot = output.join("long-explanation.html");
        ViewerAdapter::new(ops(&f)?).export_snapshot(&request(&f)?, &snapshot)?;
        fs::write(
            output.join("browser-config.json"),
            serde_json::to_vec_pretty(&json!({
                "cases": [f], "languages": ["en"], "basis": {f["project"].as_str().ok_or("project")?: {"en": answers}},
                "snapshots": {f["project"].as_str().ok_or("project")?: {"en": snapshot}},
                "output": output, "offline": true,
                "fixture_authority": "authored structural support; no model or human assessment"
            }))?,
        )?;
    }
    Ok(())
}

#[test]
fn seed_grounded_reader_runtime() -> Result<(), Box<dyn Error>> {
    let Ok(output) = std::env::var("VOLICORD_GROUNDED_READER_ROOT") else {
        return Ok(());
    };
    let output = Path::new(&output);
    fs::create_dir_all(output)?;
    let mut cases = Vec::new();
    for comparison in ["changed", "no_change", "missing_before_state"] {
        let root = output.join(comparison);
        fs::create_dir(&root)?;
        cases.push(seed(&root, comparison)?);
    }
    fs::write(
        output.join("fixture.json"),
        serde_json::to_vec_pretty(&json!({"cases":cases}))?,
    )?;
    Ok(())
}

#[test]
fn deleted_interpretation_is_distinct_from_never_generated_and_reads_stay_empty(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let f = seed(temp.path(), "missing_before_state")?;
    let mut host = HostAdapter::new(ops(&f)?);
    let missing = call(
        &mut host,
        "repository_understanding",
        json!({"project_id":f["project"]}),
    )?;
    assert_eq!(
        missing["work_history"][0]["answers"]["explanation_state"],
        "missing"
    );
    record(&f, &mut host)?;
    call(
        &mut host,
        "work_explanation",
        json!({"action":"delete","project_id":f["project"],"work_item_id":f["work"]}),
    )?;
    let data = call(
        &mut host,
        "repository_understanding",
        json!({"project_id":f["project"]}),
    )?;
    assert_eq!(
        data["work_history"][0]["answers"]["explanation_state"],
        "deleted"
    );
    let page = get(&f)?;
    assert!(page.contains("The publication or a required Source was deleted."));
    assert!(!page.contains("return 5</code>"));
    assert!(!page.contains("Understand the constant return in value()."));
    Ok(())
}

#[test]
fn forgetting_barrier_withholds_code_and_prose_until_cleanup_without_renewal(
) -> Result<(), Box<dyn Error>> {
    for excerpt in [false, true] {
        forgetting_barrier(excerpt)?;
    }
    Ok(())
}
fn forgetting_barrier(excerpt: bool) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let mut f = seed(temp.path(), "missing_before_state")?;
    let mut host = HostAdapter::new(ops(&f)?);
    if excerpt {
        let prefix = "# context\n".repeat(2000);
        let body = "def value():\n    return 5\n";
        fs::write(temp.path().join("repo/main.py"), format!("{prefix}{body}"))?;
        let a = ops(&f)?
            .analyze(project(&f)?, vec![])?
            .value
            .ok_or("analysis")?;
        let receipt = call(
            &mut host,
            "work_source",
            json!({"action":"observe_file","project_id":f["project"],"work_item_id":f["work"],"path":"main.py",
            "analysis_snapshot_id":a.analysis.identity.to_string(),"retain_seconds":3600,
            "byte_range":{"start_byte":prefix.len(),"end_byte":prefix.len()+body.len()}}),
        )?;
        f["after"] = receipt["source_id"].clone();
    }
    record(&f, &mut host)?;
    let project = project(&f)?;
    let work = work(&f)?;
    let canonical = ops(&f)?.canonical_basis(project)?;
    let auth = canonical
        .context_items
        .iter()
        .find(|c| c.id == work)
        .ok_or("goal")?
        .source_basis[0];
    let source: SourceId =
        serde_json::from_value(json!(hex_bytes(f["after"].as_str().ok_or("source")?)?))?;
    // A real retained SQLite reader forces incomplete sanitation after canonical commit.
    ops(&f)?.submit_candidate(CandidateDraft {
        project_id: project,
        kind: CandidateKind::Observation,
        collection_mode: CandidateCollectionMode::Automatic,
        origin: CandidateOrigin {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "authored-reader-control".into(),
            },
            subsystem: "reader-test".into(),
            session: Some("reader".into()),
            provenance_summary: "sanitation control".into(),
        },
        collection_scope: CandidateCollectionScope {
            project_id: project,
            session: Some("reader".into()),
            source_operation: Some("control".into()),
            candidate_kind: CandidateKind::Observation,
        },
        observation_basis: CandidateObservationBasis {
            source_basis: vec![source],
            ..CandidateObservationBasis::default()
        },
        observed_at: TimestampMicros::from_unix_micros(1),
        retention: CandidateRetention {
            retained_until: None,
            basis: "test control".into(),
        },
        content: CandidateContent {
            bounded_summary: "Retained Source sanitation control".into(),
            question: None,
            engineering_choice_discovery: None,
            materiality_review: None,
            learning_deliberation: None,
        },
    })?;
    let blocker = rusqlite::Connection::open(ops(&f)?.layout().candidate_store())?;
    blocker.execute_batch("BEGIN DEFERRED")?;
    let _: String = blocker.query_row("SELECT record_json FROM candidates LIMIT 1", [], |r| {
        r.get(0)
    })?;
    let outcome = ops(&f)?.forget_record(project, CanonicalRecordId::Source(source), auth)?;
    assert!(outcome.canonical_committed);
    assert_eq!(
        outcome.state,
        volicord_operations::ForgettingState::RepairRequired
    );
    let privacy = ops(&f)?.privacy_status(project)?;
    assert!(!privacy.withheld_for_canonical_forgetting.is_empty());
    let data = call(
        &mut host,
        "repository_understanding",
        json!({"project_id":f["project"]}),
    )?;
    assert_eq!(
        data["work_history"][0]["answers"]["explanation_state"],
        "withheld"
    );
    let page = get(&f)?;
    assert!(page.contains("Content is withheld by the forgetting barrier."));
    assert!(code_texts(&page).is_empty());
    assert!(!page.contains("Understand the constant return in value()."));
    assert_eq!(privacy, ops(&f)?.privacy_status(project)?);
    blocker.execute_batch("ROLLBACK")?;
    drop(blocker);
    let repaired = ops(&f)?.repair_forgetting(project, outcome.operation_id)?;
    assert_eq!(
        repaired.state,
        volicord_operations::ForgettingState::Completed
    );
    let page = get(&f)?;
    assert!(page.contains("The publication or a required Source was deleted."));
    Ok(())
}

#[test]
fn large_excerpt_reaches_cli_mcp_live_and_snapshot_without_surrounding_bytes(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let mut f = seed(temp.path(), "missing_before_state")?;
    let prefix = "# unretained context\n".repeat(2000);
    let mechanism = "def value():\n    return 5\n";
    let full = format!("{prefix}{mechanism}# unretained suffix\n");
    fs::write(temp.path().join("repo/main.py"), &full)?;
    let operations = ops(&f)?;
    let a = operations
        .analyze(project(&f)?, vec![])?
        .value
        .ok_or("analysis")?;
    let mut host = HostAdapter::new(ops(&f)?);
    let receipt = call(
        &mut host,
        "work_source",
        json!({"action":"observe_file","project_id":f["project"],"work_item_id":f["work"],
        "path":"main.py","analysis_snapshot_id":a.analysis.identity.to_string(),"retain_seconds":3600,
        "byte_range":{"start_byte":prefix.len(),"end_byte":prefix.len()+mechanism.len()}}),
    )?;
    f["after"] = receipt["source_id"].clone();
    record(&f, &mut host)?;
    let canonical = operations.canonical_basis(project(&f)?)?;
    let privacy = operations.privacy_status(project(&f)?)?;
    let mut selector = receipt["inspect"].clone();
    selector.as_object_mut().ok_or("selector")?.remove("tool");
    let observed = call(&mut host, "canonical_inspect", selector)?;
    let observation: Value = serde_json::from_str(observed["chunk"].as_str().ok_or("chunk")?)?;
    assert_eq!(observation["detail"]["body"], mechanism);
    assert_eq!(observation["detail"]["region"]["start_line"], 2001);
    assert_eq!(observation["detail"]["region"]["end_line"], 2002);
    let public = public_answers(&f, &mut host)?;
    let inspected = operations.read_explanation_citation(
        project(&f)?,
        ExplanationSubject::Work(work(&f)?),
        "en",
        &volicord_operations::ExplanationCitationSelection {
            publication_id: public["provenance"]["publication_id"]
                .as_str()
                .ok_or("publication identity")?
                .into(),
            realization_fingerprint: public["provenance"]["realization_fingerprint"]
                .as_str()
                .ok_or("output")?
                .into(),
            unit_index: 1,
            citation_index: 0,
            offset: 0,
        },
    )?;
    assert_eq!(inspected["chunk"]["text"], mechanism);
    assert_eq!(inspected["chunk"]["displayed_start_byte"], prefix.len());
    assert_eq!(
        inspected["chunk"]["displayed_end_byte"],
        prefix.len() + mechanism.len()
    );
    assert_eq!(inspected["chunk"]["displayed_start_line"], 2001);
    assert_eq!(inspected["chunk"]["displayed_end_line"], 2002);
    for json_mode in [false, true] {
        let mut args = vec![
            "--runtime",
            f["runtime"].as_str().ok_or("runtime")?,
            "--project",
            f["project"].as_str().ok_or("project")?,
            "status",
        ];
        if json_mode {
            args.push("--json");
        }
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(run_cli(args, &mut stdout, &mut stderr), CliExit::SUCCESS);
        let text = String::from_utf8(stdout)?;
        assert!(text.contains("return 5"));
        assert!(text.contains(if json_mode {
            "bounded_excerpt"
        } else {
            "Bounded file observation"
        }));
        assert!(!text.contains("# unretained"));
    }
    let data = call(
        &mut host,
        "repository_understanding",
        json!({"project_id":f["project"]}),
    )?;
    assert!(data.to_string().contains("bounded_excerpt"));
    assert!(!data.to_string().contains("# unretained"));
    let page = get(&f)?;
    assert!(page.contains("main.py:2001–2002"));
    assert!(page.contains("Bounded file observation: surrounding code was not retained"));
    assert!(code_texts(&page)
        .iter()
        .any(|s| s == "def value():\n    return 5\n"));
    assert!(!page.contains("# unretained"));
    let destination = temp.path().join("large.html");
    ViewerAdapter::new(ops(&f)?).export_snapshot(&request(&f)?, &destination)?;
    let snapshot = fs::read_to_string(destination)?;
    assert!(snapshot.contains("main.py:2001–2002"));
    assert!(snapshot.contains("Bounded file observation"));
    assert!(!snapshot.contains("# unretained"));
    assert_eq!(canonical, operations.canonical_basis(project(&f)?)?);
    assert_eq!(privacy, operations.privacy_status(project(&f)?)?);
    let projection = operations.project_projection(project(&f)?)?;
    let selected = projection
        .work_history
        .iter()
        .find(|w| w.work_item_id == work(&f).unwrap())
        .ok_or("work")?;
    let provenance = work_answers(selected, "en", FixedLocale::English)
        .provenance
        .ok_or("provenance")?;
    fs::write(
        temp.path().join("repo/main.py"),
        full.replacen("unretained context", "CHANGED context", 1),
    )?;
    let denied = temp.path().join("denied.html");
    assert!(operations
        .publish_viewer_snapshot(
            &page,
            &denied,
            project(&f)?,
            &projection.canonical_read_fingerprint,
            &[provenance],
            &[],
        )
        .is_ok());
    assert!(denied.exists());
    assert!(!get(&f)?.contains("# unretained"));
    assert!(get(&f)?.contains("data-comparison-state=\"changed\""));
    assert!(code_texts(&get(&f)?).iter().any(|s| s.contains("return 5")));
    Ok(())
}

// Representative source is maintained production code, not padding around a
// two-line function. Expectations are independently authored structural controls.
const REPRESENTATIVE_SOURCE: &str =
    include_str!("../../volicord-operations/src/explanation_sources.rs");
const REPRESENTATIVE_GOAL: &str =
    "Explain retained Source retrieval and current-code validation without claiming execution success.";

fn seed_representative(root: &Path) -> Result<Value, Box<dyn Error>> {
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("src"))?;
    let path = "src/explanation_sources.rs";
    fs::write(repo.join(path), REPRESENTATIVE_SOURCE)?;
    assert!(REPRESENTATIVE_SOURCE.len() > 16_384);
    let operations = LocalOperations::new(RuntimeLayout::new(root.join("runtime"))?);
    let project = operations
        .initialize_project("Read retained evidence safely", Some(&repo))?
        .project
        .id;
    let work = operations
        .record_current_host_user_context(
            project,
            "authored-structural-control".into(),
            "representative-reader".into(),
            REPRESENTATIVE_GOAL.into(),
            ContextItemRole::Goal,
            REPRESENTATIVE_GOAL.into(),
        )?
        .context_item_id;
    let analysis = operations
        .analyze(project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let mut expected = Vec::new();
    for (start_marker, end_marker) in [
        (
            "    fn explanation_source(",
            "    pub(crate) fn validate_subject_sources(",
        ),
        (
            "                    let current_hash =",
            "                    // A corrupt comparison",
        ),
    ] {
        let start = REPRESENTATIVE_SOURCE.find(start_marker).ok_or("start")?;
        let end = REPRESENTATIVE_SOURCE[start..]
            .find(end_marker)
            .ok_or("end")?
            + start;
        let observe = |before_source_id| {
            operations.observe_work_file(FileObservationRequest {
                project_id: project,
                work_item_id: work,
                analysis_snapshot_id: analysis,
                path: path.into(),
                byte_range: Some(volicord_operations::FileByteRange {
                    start_byte: start,
                    end_byte: end,
                }),
                before_source_id,
                retain_seconds: 3600,
            })
        };
        let before = observe(None)?;
        let after = observe(Some(before.source_id))?;
        for (receipt, use_kind) in [(after, "current_code"), (before, "before_code")] {
            expected.push(json!({"source_id":receipt.source_id.to_string(),"text":&REPRESENTATIVE_SOURCE[start..end],
                "start_byte":start,"end_byte":end,
                "start_line":REPRESENTATIVE_SOURCE[..start].bytes().filter(|b|*b==b'\n').count()+1,
                "end_line":REPRESENTATIVE_SOURCE[..end-1].bytes().filter(|b|*b==b'\n').count()+1,
                "use_kind":use_kind}));
        }
    }
    Ok(
        json!({"runtime":operations.layout().root(),"repository":repo,
        "project":project.to_string(),"work":work.to_string(),"path":path,
        "after":expected[0]["source_id"],"before":expected[1]["source_id"],
        "comparison":"no_change","goal":REPRESENTATIVE_GOAL,
        "expected_sources":expected,"input_provenance":"authored-structural-control","host_output":"not_generated"}),
    )
}

#[test]
fn representative_production_code_preserves_grounding_across_public_readers(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let f = seed_representative(temp.path())?;
    let operations = ops(&f)?;
    let mut host = HostAdapter::new(ops(&f)?);
    let generation = call(
        &mut host,
        "work_explanation",
        json!({"action":"generate",
        "project_id":f["project"],"work_item_id":f["work"],"language":"en"}),
    )?;
    assert_eq!(
        generation["generation_status"],
        "awaiting_host_interpretation"
    );
    let text = "Retained Source retrieval reads the scoped observation body. Current-code validation checks the full file hash and selected repository snapshot before admitting prose.";
    let mut citations = Vec::new();
    for expected in f["expected_sources"].as_array().ok_or("sources")? {
        let read = call(
            &mut host,
            "work_explanation",
            json!({"action":"read_source",
            "project_id":f["project"],"work_item_id":f["work"],"generation_id":generation["generation_id"],
            "source_id":expected["source_id"]}),
        )?;
        assert_eq!(read["observation"]["detail"]["body"], expected["text"]);
        assert_eq!(
            read["observation"]["detail"]["representation"],
            "bounded_excerpt"
        );
        let mut citation = read["citation_basis"].clone();
        for field in [
            "start_byte",
            "end_byte",
            "start_line",
            "end_line",
            "use_kind",
        ] {
            citation[field] = expected[field].clone();
        }
        citation["locator"] = f["path"].clone();
        citation["field"] = json!("body");
        citation["range_sha256"] = json!(explanation_text_hash(
            expected["text"].as_str().ok_or("text")?
        ));
        citation["claim_start_byte"] = json!(0);
        citation["claim_end_byte"] = json!(text.len());
        citation["claim_sha256"] = json!(explanation_text_hash(text));
        citations.push(citation);
    }
    let paragraphs: Vec<_> = [
        ("purpose", "Understand how retained evidence can be read safely."),
        ("reported_change", text),
        ("reported_change", "Both pairs are retained observations of identical file bytes; the investigation does not establish an implementation change."),
        ("expected_effect", "Invalid or changed evidence must not supply current prose."),
        ("verification", "This authored control supplies no execution result for the observed implementation."),
        ("next_step", "No next action is recorded; interpretation cannot establish a user direction."),
        ("limits", "Only two explicit file regions were retained; surrounding code and semantic approval are unavailable."),
    ].into_iter().map(|(question,text)|json!({"question":question,"text":text,
        "evidence_keys":[if question=="next_step" {"next_step"} else {"goal"}],
        "citations":if question=="reported_change" {citations.iter().cloned().map(|mut citation| {
            citation["claim_end_byte"] = json!(text.len());
            citation["claim_sha256"] = json!(explanation_text_hash(text));
            citation
        }).collect::<Vec<_>>()} else {vec![]}})).collect();
    let realization = json!({"format_kind":"volicord_explanation","format_version":volicord_projections::EXPLANATION_VERSION,"diagrams":[],
        "plan_fingerprint":generation["plan"]["fingerprint"],"language":"en",
        "generator":{"host":"authored-structural-control","session":"representative-reader","agent":null,"model":null},
        "paragraphs":paragraphs});
    call(
        &mut host,
        "work_explanation",
        json!({"action":"record","project_id":f["project"],
        "work_item_id":f["work"],"generation_id":generation["generation_id"],"realization":realization.clone()}),
    )?;
    let canonical = operations.canonical_basis(project(&f)?)?;
    let privacy = operations.privacy_status(project(&f)?)?;
    let projection =
        operations.project_projection_selected(project(&f)?, WorkSelector::ExactWork(work(&f)?))?;
    let answers = work_answers(
        projection.selected_work.as_ref().ok_or("Work")?,
        "en",
        FixedLocale::English,
    );
    assert_eq!(answers.explanation_state, ExplanationState::Current);
    assert_eq!(answers.source_excerpts.len(), 4);
    for (excerpt, expected) in answers
        .source_excerpts
        .iter()
        .zip(f["expected_sources"].as_array().ok_or("sources")?)
    {
        let original = expected["text"].as_str().ok_or("text")?;
        assert!(
            !excerpt.text.is_empty() && excerpt.text.len() <= EXPLANATION_SOURCE_WINDOW_BYTE_LIMIT
        );
        assert_eq!(
            excerpt.text, original,
            "complete production function window, not a prefix"
        );
        assert_eq!(
            excerpt.displayed_end_byte,
            excerpt.citation.start_byte + excerpt.text.len()
        );
        assert_eq!(
            excerpt.citation.start_byte,
            expected["start_byte"].as_u64().ok_or("start")? as usize
        );
        assert_eq!(
            excerpt.citation.start_line,
            expected["start_line"].as_u64().ok_or("line")? as usize
        );
        assert_eq!(
            excerpt.omitted_utf8_bytes,
            original.len() - excerpt.text.len()
        );
    }
    let identity = &answers
        .provenance
        .as_ref()
        .ok_or("identity")?
        .realization_fingerprint;
    let mcp = call(
        &mut host,
        "repository_understanding",
        json!({"project_id":f["project"]}),
    )?;
    assert_same_answer_basis(
        mcp["selected_work"]["answers"].clone(),
        serde_json::to_value(&answers)?,
    );
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                f["runtime"].as_str().ok_or("runtime")?,
                "--project",
                f["project"].as_str().ok_or("project")?,
                "status",
                "--json"
            ],
            &mut stdout,
            &mut stderr
        ),
        CliExit::SUCCESS
    );
    let cli: Value = serde_json::from_slice(&stdout)?;
    let cli_work = cli["work_history"]
        .as_array()
        .ok_or("CLI Works")?
        .iter()
        .find(|item| item["work_item_id"] == f["work"])
        .ok_or("CLI Work")?;
    assert_same_answer_basis(cli_work["answers"].clone(), serde_json::to_value(&answers)?);
    let page = get(&f)?;
    assert!(page.contains(identity));
    assert!(page.contains("selected repository snapshot"));
    assert!(page.contains("identical bytes"));
    for (locale, title) in [
        (ViewerLocale::English, "Change / investigation"),
        (ViewerLocale::Korean, "구조와 동작"),
    ] {
        let mut code_request = request(&f)?;
        code_request.locale = locale;

        let code = ViewerAdapter::new(ops(&f)?)
            .render(&code_request, "test-form-authenticity")?
            .html;
        assert!(
            !code.contains(&format!("<h2>{title}</h2>")),
            "Work narrative is not grouped by record-role headings"
        );
        assert!(code.contains(text));
        assert!(code.contains("Both pairs are retained observations"));
    }
    let snapshot = temp.path().join("representative.html");
    ViewerAdapter::new(ops(&f)?).export_snapshot(&request(&f)?, &snapshot)?;
    let snapshot = fs::read_to_string(snapshot)?;
    assert!(snapshot.contains(identity));
    for (citation_index, expected) in f["expected_sources"]
        .as_array()
        .ok_or("sources")?
        .iter()
        .enumerate()
    {
        let source = expected["text"].as_str().ok_or("text")?;
        let mut selected = volicord_operations::ExplanationCitationSelection {
            publication_id: public_answers(&f, &mut host)?["provenance"]["publication_id"]
                .as_str()
                .ok_or("publication identity")?
                .into(),
            realization_fingerprint: identity.clone(),
            unit_index: 1,
            citation_index,
            offset: 0,
        };
        let mut complete = String::new();
        loop {
            let read = operations.read_explanation_citation(
                project(&f)?,
                ExplanationSubject::Work(work(&f)?),
                "en",
                &selected,
            )?;
            let chunk = read["chunk"]["text"].as_str().ok_or("selected bytes")?;
            assert!(!chunk.is_empty() && chunk.len() <= 4096);
            complete.push_str(chunk);
            match read["chunk"]["next_offset"].as_u64() {
                Some(offset) => {
                    assert!(offset as usize > selected.offset);
                    selected.offset = offset as usize;
                }
                None => break,
            }
        }
        assert_eq!(
            complete, source,
            "full original Source proof remains reachable across bounded reads"
        );
        let lines = format!(
            "{}:{}–{}",
            f["path"].as_str().ok_or("path")?,
            expected["start_line"],
            expected["end_line"]
        );
        for output in [&page, &snapshot] {
            let excerpt = answers
                .source_excerpts
                .iter()
                .find(|e| {
                    e.citation.source_id == expected["source_id"].as_str().unwrap_or_default()
                })
                .ok_or("selected initial excerpt")?;
            assert!(code_texts(output).iter().any(|s| s == &excerpt.text));
            if excerpt.omitted_utf8_bytes > 0 {
                assert!(output.contains("Omitted UTF-8 bytes in this cited range"));
            }
            assert!(output.contains(&lines));
            assert!(output.contains(expected["source_id"].as_str().ok_or("Source")?));
        }
    }
    assert_eq!(canonical, operations.canonical_basis(project(&f)?)?);
    assert_eq!(privacy, operations.privacy_status(project(&f)?)?);
    // A wording revision needs fresh request-local retrieval and new claim
    // coordinates; it cannot inherit the old output's publication identity.
    let next = call(
        &mut host,
        "work_explanation",
        json!({"action":"generate",
        "project_id":f["project"],"work_item_id":f["work"],"language":"en"}),
    )?;
    for expected in f["expected_sources"].as_array().ok_or("sources")? {
        call(
            &mut host,
            "work_explanation",
            json!({"action":"read_source",
            "project_id":f["project"],"work_item_id":f["work"],"generation_id":next["generation_id"],
            "source_id":expected["source_id"]}),
        )?;
    }
    let mut edited = realization;
    let changed_claim = format!("Observed implementation: {text}");
    edited["paragraphs"][1]["text"] = json!(changed_claim);
    let record_args = |realization| {
        json!({"action":"record","project_id":f["project"],
        "work_item_id":f["work"],"generation_id":next["generation_id"],"realization":realization})
    };
    let rejected = host
        .handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"work_explanation","arguments":record_args(edited.clone())}}))
        .ok_or("response")?;
    assert_eq!(rejected["result"]["isError"], true);
    assert_eq!(privacy, operations.privacy_status(project(&f)?)?);
    for citation in edited["paragraphs"][1]["citations"]
        .as_array_mut()
        .ok_or("citations")?
    {
        citation["claim_end_byte"] = json!(changed_claim.len());
        citation["claim_sha256"] = json!(explanation_text_hash(&changed_claim));
    }
    call(&mut host, "work_explanation", record_args(edited))?;
    let current =
        operations.project_projection_selected(project(&f)?, WorkSelector::ExactWork(work(&f)?))?;
    let revised = work_answers(
        current.selected_work.as_ref().ok_or("Work")?,
        "en",
        FixedLocale::English,
    );
    assert_ne!(
        revised
            .provenance
            .as_ref()
            .ok_or("identity")?
            .realization_fingerprint,
        *identity
    );
    assert_eq!(
        revised.provenance.as_ref().ok_or("identity")?.fingerprint,
        answers.provenance.as_ref().ok_or("identity")?.fingerprint
    );
    assert_eq!(revised.prose[1].text, changed_claim);
    let old_publication = temp.path().join("old-output.html");
    assert!(operations
        .publish_viewer_snapshot(
            &page,
            &old_publication,
            project(&f)?,
            &projection.canonical_read_fingerprint,
            &[answers.provenance.ok_or("identity")?],
            &[],
        )
        .is_err());
    assert!(!old_publication.exists());
    assert_eq!(canonical, operations.canonical_basis(project(&f)?)?);
    let page = get(&f)?;
    // An uncited repository change changes correspondence, not retained permission.
    fs::write(
        temp.path()
            .join("repo")
            .join(f["path"].as_str().ok_or("path")?),
        format!("{REPRESENTATIVE_SOURCE}\n// changed outside observations\n"),
    )?;
    assert!(!get(&f)?.contains("class=\"source-excerpt\""));
    let denied = temp.path().join("denied.html");
    assert!(operations
        .publish_viewer_snapshot(
            &page,
            &denied,
            project(&f)?,
            &current.canonical_read_fingerprint,
            &[revised.provenance.ok_or("identity")?],
            &[],
        )
        .is_ok());
    assert!(denied.exists());
    assert!(get(&f)?.contains("data-comparison-state=\"changed\""));
    Ok(())
}

#[test]
fn seed_representative_reader_runtime() -> Result<(), Box<dyn Error>> {
    let Ok(output) = std::env::var("VOLICORD_REPRESENTATIVE_READER_ROOT") else {
        return Ok(());
    };
    let root = Path::new(&output);
    fs::create_dir_all(root)?;
    let case = seed_representative(root)?;
    fs::write(
        root.join("fixture.json"),
        serde_json::to_vec_pretty(&json!({"cases":[case]}))?,
    )?;
    Ok(())
}

#[test]
fn shared_source_cards_retain_every_paragraph_claim_and_isolate_other_ranges(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let f = seed(temp.path(), "missing_before_state")?;
    let mut host = HostAdapter::new(ops(&f)?);
    let generation = call(
        &mut host,
        "work_explanation",
        json!({"action":"generate","project_id":f["project"],
            "work_item_id":f["work"],"language":"en"}),
    )?;
    let read = call(
        &mut host,
        "work_explanation",
        json!({"action":"read_source","project_id":f["project"],
            "work_item_id":f["work"],"generation_id":generation["generation_id"],
            "source_id":f["after"]}),
    )?;
    let body = read["observation"]["detail"]["body"]
        .as_str()
        .ok_or("body")?;
    assert_eq!(body, "def value():\n    return 5\n");
    let cases = [
        (
            "reported_change",
            "ReportedChange",
            2,
            "Observed: 한 The function returns 5.",
            "The function returns 5.",
            0,
            body.len(),
        ),
        (
            "expected_effect",
            "ExpectedEffect",
            3,
            "Effect hypothesis: This proves every caller is safe.",
            "This proves every caller is safe.",
            0,
            body.len(),
        ),
        (
            "expected_effect",
            "ExpectedEffect",
            4,
            "Limit: No execution result follows from this file.",
            "No execution result follows from this file.",
            0,
            body.len(),
        ),
        (
            "limits",
            "Limits",
            5,
            "Other range: The definition is named value.",
            "The definition is named value.",
            0,
            "def value():".len(),
        ),
    ];
    let mut paragraphs = vec![
        json!({"question":"purpose","text":"Inspect the constant-return definition.",
        "evidence_keys":["goal"],"citations":[]}),
    ];
    for (question, _, _, paragraph, claim, start, end) in cases {
        let offset = paragraph.find(claim).ok_or("claim substring")?;
        let mut citation = read["citation_basis"].clone();
        citation["field"] = json!("body");
        citation["locator"] = json!("main.py");
        citation["use_kind"] = json!("current_code");
        citation["start_byte"] = json!(start);
        citation["end_byte"] = json!(end);
        citation["start_line"] = json!(1);
        citation["end_line"] = json!(if end == body.len() { 2 } else { 1 });
        citation["range_sha256"] = json!(explanation_text_hash(&body[start..end]));
        citation["claim_start_byte"] = json!(offset);
        citation["claim_end_byte"] = json!(offset + claim.len());
        citation["claim_sha256"] = json!(explanation_text_hash(claim));
        paragraphs.push(json!({"question":question,"text":paragraph,
            "evidence_keys":["goal"],"citations":[citation]}));
    }
    paragraphs.extend([
        json!({"question":"verification","text":"No executed verification was recorded.","evidence_keys":["goal"],"citations":[]}),
        json!({"question":"next_step","text":"No recorded next action is supplied.","evidence_keys":["next_step"],"citations":[]}),
    ]);
    let realization = json!({"format_kind":EXPLANATION_KIND,"format_version":EXPLANATION_VERSION,"diagrams":[],
        "plan_fingerprint":generation["plan"]["fingerprint"],"language":"en",
        "generator":{"host":"independent-shared-range-control","session":"authored-source-claims",
            "agent":null,"model":null},"paragraphs":paragraphs});
    let recorded = call(
        &mut host,
        "work_explanation",
        json!({"action":"record","project_id":f["project"],"work_item_id":f["work"],
            "generation_id":generation["generation_id"],"realization":realization}),
    )?;
    assert_eq!(
        recorded["explanation"]["semantic_approval_status"],
        "not_established"
    );
    let operations = ops(&f)?;
    let canonical = operations.canonical_basis(project(&f)?)?;
    let privacy = operations.privacy_status(project(&f)?)?;
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        let mut req = request(&f)?;
        req.locale = locale;
        let server = ViewerServer::new(
            ViewerAdapter::new(ops(&f)?),
            project(&f)?,
            locale,
            ViewerView::Overview,
            "en".into(),
            "127.0.0.1:3219".parse()?,
        )?;
        let locale_tag = if locale == ViewerLocale::English {
            "en"
        } else {
            "ko"
        };
        let http = format!("GET /?view=work&work={}&locale={locale_tag}&language=en HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n", f["work"].as_str().ok_or("work")?);
        let mut live = Vec::new();
        server.serve_connection(&mut http.as_bytes(), &mut live)?;
        let live = String::from_utf8(live)?;
        assert!(live.starts_with("HTTP/1.1 200"));
        let destination = temp.path().join(format!("shared-claims-{locale_tag}.html"));
        ViewerAdapter::new(ops(&f)?).export_snapshot(&req, &destination)?;
        let snapshot = fs::read_to_string(destination)?;
        for page in [&live, &snapshot] {
            // Shared-code return targets are nested sections. Inspect whole
            // reading points, then follow their exact native alias to the one
            // physical payload; a closing inner section is not the point end.
            let mut points = Vec::new();
            let mut remaining = page.as_str();
            while let Some(start) = remaining.find("<section class=\"source-point\"") {
                let point = &remaining[start..];
                let mut cursor = "<section".len();
                let mut depth = 1;
                while depth > 0 {
                    let close = cursor + point[cursor..].find("</section>").ok_or("point end")?;
                    if let Some(open) = point[cursor..close].find("<section") {
                        depth += 1;
                        cursor += open + "<section".len();
                    } else {
                        depth -= 1;
                        cursor = close + "</section>".len();
                    }
                }
                points.push(&point[..cursor]);
                remaining = &point[cursor..];
            }
            assert_eq!(points.len(), 4, "one Source point per exact claim binding");
            for (_, question, ordinal, paragraph, claim, _, end) in cases {
                let offset = paragraph.find(claim).ok_or("claim")?;
                let marker=format!("data-question=\"{question}\" data-paragraph=\"{ordinal}\" data-claim-start=\"{offset}\" data-claim-end=\"{}\"",offset+claim.len());
                let matching: Vec<_> = points.iter().filter(|p| p.contains(&marker)).collect();
                assert_eq!(
                    matching.len(),
                    1,
                    "exact claim identity must not leak to another Source point"
                );
                let point = matching[0];
                assert!(point.contains(&format!("{claim}</blockquote>")));
                assert!(point.contains(&explanation_text_hash(claim)));
                assert!(point.contains("class=\"return\""));
                let citation: ExplanationCitation = serde_json::from_value(
                    realization["paragraphs"][ordinal - 1]["citations"][0].clone(),
                )?;
                let expected = format!("citation: {citation:#?}")
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
                    .replace('\'', "&#39;");
                assert!(
                    point.contains(&expected),
                    "complete exact citation retained"
                );
                let code = if end == body.len() {
                    "def value():\n    return 5\n"
                } else {
                    "def value():"
                };
                let payload = if let Some(reference) =
                    point.split("class=\"source-code-reference\"").nth(1)
                {
                    let alias = reference
                        .split_once("href=\"#")
                        .ok_or("code alias")?
                        .1
                        .split('"')
                        .next()
                        .ok_or("alias target")?;
                    let owner = points
                        .iter()
                        .find(|p| p.contains(&format!("id=\"{alias}\"")))
                        .ok_or("shared payload owner")?;
                    let target = owner
                        .split_once(&format!("id=\"{alias}\""))
                        .ok_or("native target")?
                        .1
                        .split('>')
                        .next()
                        .ok_or("target attributes")?;
                    assert!(target.contains(&format!("data-source-id=\"{}\"", citation.source_id)));
                    assert!(target
                        .contains(&format!("data-range-sha256=\"{}\"", citation.range_sha256)));
                    let point_id = point
                        .split_once("id=\"")
                        .ok_or("claim point ID")?
                        .1
                        .split('"')
                        .next()
                        .ok_or("claim point ID end")?;
                    assert!(
                        owner.contains(&format!(
                            "class=\"shared-source-return\" href=\"#{point_id}\""
                        )),
                        "shared bytes keep this exact claim's return"
                    );
                    *owner
                } else {
                    point
                };
                assert!(
                    code_texts(payload).iter().any(|s| s == code),
                    "claim reaches its complete original range"
                );
            }
            assert_eq!(
                code_texts(page)
                    .iter()
                    .filter(|s| *s == "def value():\n    return 5\n")
                    .count(),
                1,
                "repeated claims share one complete physical payload"
            );
        }
    }
    assert_eq!(canonical, operations.canonical_basis(project(&f)?)?);
    assert_eq!(privacy, operations.privacy_status(project(&f)?)?);
    Ok(())
}

/// Disposable current prose for live/offline browser support. All wording is an
/// explicitly authored structural control; it supplies no model-quality evidence.
#[test]
fn seed_authored_reading_browser_runtime() -> Result<(), Box<dyn Error>> {
    let Some(root) = std::env::var_os("VOLICORD_AUTHORED_READER_ROOT") else {
        return Ok(());
    };
    let root = Path::new(&root);
    assert!(root.is_absolute());
    let mut cases = Vec::new();
    for comparison in ["changed", "no_change", "missing_before_state"] {
        let mut f = seed(&root.join(comparison), comparison)?;
        let mut host = HostAdapter::new(ops(&f)?);
        record_in_language(
            &f,
            &mut host,
            "Understand the constant return in value().",
            "en",
        )?;
        record_in_language(
            &f,
            &mut host,
            "value()의 상수 반환과 관측의 한계를 이해합니다.",
            "ko",
        )?;
        f["host_output"] = json!("authored-structural-control; not model-generated");
        cases.push(f);
    }
    let mut f = seed_representative(&root.join("representative"))?;
    let mut host = HostAdapter::new(ops(&f)?);
    for language in ["en", "ko"] {
        let generation = call(
            &mut host,
            "work_explanation",
            json!({"action":"generate", "project_id":f["project"], "work_item_id":f["work"], "language":language}),
        )?;
        let text = if language == "ko" {
            "관측된 코드는 보존된 Source를 읽고 현재 파일의 해시를 확인합니다. 두 관측은 동일한 파일 바이트를 보존하며 실행 성공은 입증하지 않습니다."
        } else {
            "The observed code reads retained Sources and checks the current file hash. Both observations retain identical file bytes and establish no execution success."
        };
        let mut citations = Vec::new();
        for expected in f["expected_sources"].as_array().ok_or("Sources")? {
            let read = call(
                &mut host,
                "work_explanation",
                json!({"action":"read_source", "project_id":f["project"], "work_item_id":f["work"], "generation_id":generation["generation_id"], "source_id":expected["source_id"]}),
            )?;
            assert_eq!(read["observation"]["detail"]["body"], expected["text"]);
            let mut citation = read["citation_basis"].clone();
            for field in [
                "start_byte",
                "end_byte",
                "start_line",
                "end_line",
                "use_kind",
            ] {
                citation[field] = expected[field].clone();
            }
            citation["locator"] = f["path"].clone();
            citation["field"] = json!("body");
            citation["range_sha256"] = json!(explanation_text_hash(
                expected["text"].as_str().ok_or("Source bytes")?
            ));
            citation["claim_start_byte"] = json!(0);
            citation["claim_end_byte"] = json!(text.len());
            citation["claim_sha256"] = json!(explanation_text_hash(text));
            citations.push(citation);
        }
        let paragraphs: Vec<_> = [
            ("heading", if language=="ko" {"보존된 Source를 읽는 경계"}else{"Reading retained Sources"}),
            ("synopsis", if language=="ko" {"보존된 근거 조회와 현재 파일의 해시 검증을 따라갑니다. 실행 효과는 이 관측으로 확인하지 않습니다."}else{"Follow retained evidence lookup and current-file hash validation. These observations do not verify runtime effects."}),
            ("purpose", if language == "ko" { "보존된 근거의 조회와 현재 코드 검증을 이해합니다." } else { "Understand retained evidence lookup and current-code validation." }),
            ("reported_change", text),
            ("comparison",if language=="ko" {"보존된 사본을 읽는 관측입니다. 새 명령 실행은 확인하지 않습니다."}else{"Reading a retained copy is an observation; it does not verify a new execution."}),
            ("comparison",if language=="ko" {"현재 파일의 해시를 비교하는 관측입니다. 줄별 변경을 추정하지 않습니다."}else{"The current-file hash is compared without inferring a line-by-line change."}),
            ("comparison",if language=="ko" {"두 범위 밖의 코드는 이 관측에 포함되지 않습니다."}else{"Code outside the two regions is not part of these observations."}),
            ("expected_effect", if language == "ko" { "실행 중 효과는 검증되지 않았습니다." } else { "No runtime effect is verified." }),
            ("verification", if language == "ko" { "이 테스트 문구는 실행 결과를 제공하지 않습니다." } else { "This authored control supplies no execution result." }),
            ("next_step", if language == "ko" { "다음 행동은 기록되지 않았습니다." } else { "No next action is recorded." }),
            ("limits", if language == "ko" { "두 파일 구간만 관측했습니다. 주변 코드와 의미적 승인은 확인되지 않았습니다." } else { "Only two file regions were observed. Surrounding code and semantic approval remain unavailable." }),
        ].into_iter().map(|(question, text)| json!({"question":question,"text":text,"evidence_keys":[if question == "next_step" { "next_step" } else { "goal" }], "citations":if matches!(question,"heading"|"synopsis"|"reported_change"|"comparison") { citations.iter().cloned().map(|mut citation|{citation["claim_start_byte"]=json!(0);citation["claim_end_byte"]=json!(text.len());citation["claim_sha256"]=json!(explanation_text_hash(text));citation}).collect::<Vec<_>>() } else { vec![] }})).collect();
        let realization = json!({"format_kind":EXPLANATION_KIND,"format_version":EXPLANATION_VERSION,"diagrams":[],"plan_fingerprint":generation["plan"]["fingerprint"],"language":language,"generator":{"host":"authored-structural-control","session":"representative-browser","agent":null,"model":null},"paragraphs":paragraphs});
        call(
            &mut host,
            "work_explanation",
            json!({"action":"record","project_id":f["project"],"work_item_id":f["work"],"generation_id":generation["generation_id"],"realization":realization}),
        )?;
    }
    f["host_output"] = json!("authored-structural-control; not model-generated");
    cases.push(f);
    fs::write(
        root.join("fixtures.json"),
        serde_json::to_vec_pretty(&json!({"cases":cases}))?,
    )?;
    Ok(())
}

#[test]
fn command_observation_is_independent_of_checkpoint_verification_on_public_readers(
) -> Result<(), Box<dyn Error>> {
    for exit_code in [0, 3] {
        let temp = tempfile::tempdir()?;
        let f = seed(temp.path(), "missing_before_state")?;
        let operations = ops(&f)?;
        let analysis = operations
            .analyze(project(&f)?, vec![])?
            .value
            .ok_or("analysis")?
            .analysis
            .identity;
        let command = operations
            .run_work_command(volicord_operations::WorkCommandRequest {
                project_id: project(&f)?,
                work_item_id: work(&f)?,
                analysis_snapshot_id: analysis,
                command_label: "bounded observation".into(),
                program: "python3".into(),
                arguments: vec![
                    "-c".into(),
                    format!("import sys; print('observation only'); sys.exit({exit_code})"),
                ],
                timeout: std::time::Duration::from_secs(5),
                retain_seconds: 3600,
            })?
            .source_id;
        let before = operations.canonical_basis(project(&f)?)?;
        assert!(before.checkpoint_history.is_empty());
        let mut host = HostAdapter::new(ops(&f)?);
        let base = json!({"project_id":f["project"],"work_item_id":f["work"],"language":"en"});
        let mut args = base.clone();
        args["action"] = json!("generate");
        let generation = call(&mut host, "work_explanation", args)?;
        let mut args = base.clone();
        args["action"] = json!("read_source");
        args.as_object_mut().ok_or("args")?.remove("language");
        args["generation_id"] = generation["generation_id"].clone();
        args["source_id"] = json!(command.to_string());
        let read = call(&mut host, "work_explanation", args)?;
        let output = read["observation"]["detail"]["stdout"]["body"]
            .as_str()
            .ok_or("output")?;
        assert_eq!(output, "observation only\n");
        assert_eq!(read["observation"]["detail"]["exit_code"], exit_code);
        let text = "The command emitted an observation; no test assertions are reported.";
        let mut citation = read["citation_basis"].clone();
        let object = citation.as_object_mut().ok_or("citation basis")?;
        object.retain(|key, _| {
            [
                "source_id",
                "source_revision",
                "observation_fingerprint",
                "analysis_snapshot_id",
                "repository_snapshot_id",
                "repository_source_id",
            ]
            .contains(&key.as_str())
        });
        object.extend(json!({"locator":"bounded observation","field":"stdout","start_byte":0,"end_byte":output.len(),
            "start_line":1,"end_line":1,"range_sha256":explanation_text_hash(output),"claim_start_byte":0,
            "claim_end_byte":text.len(),"claim_sha256":explanation_text_hash(text),"use_kind":"observed_execution"}).as_object().ok_or("fields")?.clone());
        let realization = json!({"format_kind":EXPLANATION_KIND,"format_version":EXPLANATION_VERSION,"diagrams":[],
            "plan_fingerprint":generation["plan"]["fingerprint"],"language":"en",
            "generator":{"host":"authored-control","session":"scope-test","agent":null,"model":null},
            "paragraphs":[{"question":"observation","text":text,"evidence_keys":["goal"],"citations":[citation]}]});
        let mut args = base.clone();
        args["action"] = json!("record");
        args.as_object_mut().ok_or("args")?.remove("language");
        args["generation_id"] = generation["generation_id"].clone();
        args["realization"] = realization;
        call(&mut host, "work_explanation", args)?;
        let answers = public_answers(&f, &mut host)?;
        let citation = operations.read_explanation_citation(
            project(&f)?,
            ExplanationSubject::Work(work(&f)?),
            "en",
            &volicord_operations::ExplanationCitationSelection {
                publication_id: answers["provenance"]["publication_id"]
                    .as_str()
                    .ok_or("publication identity")?
                    .into(),
                realization_fingerprint: answers["provenance"]["realization_fingerprint"]
                    .as_str()
                    .ok_or("output")?
                    .into(),
                unit_index: 0,
                citation_index: 0,
                offset: 0,
            },
        )?;
        assert_eq!(citation["chunk"]["text"], "observation only\n");
        assert_eq!(citation["chunk"]["execution"]["exit_code"], exit_code);
        assert_eq!(citation["semantic_approval"], "not_established");
        let facts = answers["facts"].as_array().ok_or("facts")?;
        let observation = facts
            .iter()
            .find(|a| a["question"] == "SourceExecutionObservation")
            .ok_or("command fact")?;
        assert!(observation["text"]
            .as_str()
            .ok_or("text")?
            .contains(&format!("exit {exit_code}")));
        assert!(observation["evidence_keys"][0]
            .as_str()
            .ok_or("key")?
            .contains(&command.to_string()));
        let verification = facts
            .iter()
            .find(|a| a["question"] == "VerificationState")
            .ok_or("verification")?;
        assert_eq!(
            verification["text"],
            "Checkpoint verification: No verification recorded in a Checkpoint"
        );
        assert!(facts
            .iter()
            .find(|a| a["question"] == "WorkState")
            .ok_or("state")?["text"]
            .as_str()
            .ok_or("state text")?
            .contains("Goal-only Work is open"));
        for page in [
            get(&f)?,
            ViewerAdapter::new(ops(&f)?)
                .render_snapshot(&request(&f)?, TimestampMicros::from_unix_micros(1))?
                .html,
        ] {
            assert!(page.contains("data-question=\"SourceExecutionObservation\""));
            assert!(
                page.contains("Checkpoint verification: No verification recorded in a Checkpoint")
            );
            assert!(page.contains("exit status alone establishes no test assertions"));
            assert!(!page.contains("User acceptance: accepted"));
        }
        assert_eq!(before, ops(&f)?.canonical_basis(project(&f)?)?);
        operations.delete_work_source_observation(project(&f)?, work(&f)?, command)?;
        let unavailable = public_answers(&f, &mut host)?;
        assert!(unavailable["facts"]
            .as_array()
            .ok_or("facts")?
            .iter()
            .all(|a| a["question"] != "SourceExecutionObservation"));
    }
    Ok(())
}

#[test]
fn authored_heading_synopsis_and_reading_points_share_public_lifecycle(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let f = seed(temp.path(), "missing_before_state")?;
    let mut host = HostAdapter::new(ops(&f)?);
    record(&f, &mut host)?;
    let records = ops(&f)?.privacy_status(project(&f)?)?.managed_derived;
    let mut retained: RetainedExplanation = serde_json::from_str(
        records
            .iter()
            .find_map(|r| r.content.as_deref())
            .ok_or("retained output")?,
    )?;
    let template = retained.realization.paragraphs[1].clone();
    let cases = [
        (ExplanationQuestion::Heading, "A constant return, examined"),
        (ExplanationQuestion::Synopsis, "The observed definition returns 5. This investigation establishes no execution or patch."),
        (ExplanationQuestion::Purpose, "Inspect the observed return value before recommending a change."),
        (ExplanationQuestion::ReadingPoint, "Read the return expression to see the constant 5."),
        (ExplanationQuestion::Comparison, "The file observation has no retained before-state; there is no patch comparison."),
        (ExplanationQuestion::Comparison, "The observed return expression is a retained file observation, not an execution."),
        (ExplanationQuestion::Comparison, "No downstream behavior is verified by this code observation."),
        (ExplanationQuestion::Limits, "Execution and downstream effects remain unverified."),
    ];
    retained.realization.paragraphs = cases
        .into_iter()
        .map(|(question, text)| {
            let mut p = template.clone();
            p.question = question;
            p.text = text.into();
            for c in &mut p.citations {
                c.claim_start_byte = 0;
                c.claim_end_byte = text.len();
                c.claim_sha256 = explanation_text_hash(text);
            }
            p
        })
        .collect();
    let generation = call(
        &mut host,
        "work_explanation",
        json!({"action":"generate","project_id":f["project"],"work_item_id":f["work"],"language":"en"}),
    )?;
    call(
        &mut host,
        "work_explanation",
        json!({"action":"read_source","project_id":f["project"],"work_item_id":f["work"],"generation_id":generation["generation_id"],"source_id":f["after"]}),
    )?;
    call(
        &mut host,
        "work_explanation",
        json!({"action":"record","project_id":f["project"],"work_item_id":f["work"],"generation_id":generation["generation_id"],"realization":retained.realization}),
    )?;
    let answers = public_answers(&f, &mut host)?;
    assert_eq!(answers["prose"][0]["question"], "Heading");
    assert_ne!(
        answers["provenance"]["realization_fingerprint"],
        explanation_text_hash("")
    );
    for page in [
        get(&f)?,
        ViewerAdapter::new(ops(&f)?)
            .render_snapshot(&request(&f)?, TimestampMicros::from_unix_micros(1))?
            .html,
    ] {
        assert!(page.contains("data-title-basis=\"generated-interpretation\""));
        assert!(page.contains("data-question=\"Heading\" lang=\"en\" data-content-language=\"en\">A constant return, examined</h1>"));
        assert!(page.contains("class=\"lead\""));
        assert_eq!(page.matches("class=\"comparison-card\"").count(), 3);
        assert!(page.contains("class=\"comparisons\""));
        assert!(
            page.find("data-question=\"Purpose\"").ok_or("problem")?
                < page.find("data-question=\"Limits\"").ok_or("limit")?
        );
        assert!(page.contains("class=\"return\""));
        assert!(page.contains(f["goal"].as_str().ok_or("original Goal")?));
    }
    ops(&f)?.delete_work_source_observation(
        project(&f)?,
        work(&f)?,
        SourceId::from_bytes(
            hex_bytes(f["after"].as_str().ok_or("Source")?)?
                .try_into()
                .map_err(|_| "Source bytes")?,
        ),
    )?;
    let answers = public_answers(&f, &mut host)?;
    assert!(answers["provenance"].is_null());
    let page = get(&f)?;
    assert!(!page.contains("A constant return, examined"));
    assert!(!page.contains("data-question=\"Synopsis\""));
    assert!(page.contains(f["goal"].as_str().ok_or("Goal")?));
    Ok(())
}

#[test]
fn work_selected_citations_keep_observed_pairs_and_missing_before_distinct(
) -> Result<(), Box<dyn Error>> {
    for comparison in ["changed", "no_change", "missing_before_state"] {
        let temp = tempfile::tempdir()?;
        let f = seed(temp.path(), comparison)?;
        let mut host = HostAdapter::new(ops(&f)?);
        record(&f, &mut host)?;
        let operations = ops(&f)?;
        let answers = public_answers(&f, &mut host)?;
        let unit = answers["prose"]
            .as_array()
            .ok_or("prose")?
            .iter()
            .position(|p| p["question"] == "ReportedChange")
            .ok_or("change")?;
        let before = operations.canonical_basis(project(&f)?)?;
        let privacy = operations.privacy_status(project(&f)?)?;
        let count = if comparison == "missing_before_state" {
            1
        } else {
            2
        };
        for index in 0..count {
            let selected = volicord_operations::ExplanationCitationSelection {
                publication_id: answers["provenance"]["publication_id"]
                    .as_str()
                    .ok_or("publication identity")?
                    .into(),
                realization_fingerprint: answers["provenance"]["realization_fingerprint"]
                    .as_str()
                    .ok_or("output")?
                    .into(),
                unit_index: unit,
                citation_index: index,
                offset: 0,
            };
            let result = operations.read_explanation_citation(
                project(&f)?,
                ExplanationSubject::Work(work(&f)?),
                "en",
                &selected,
            )?;
            let server = ViewerServer::new(
                ViewerAdapter::new(ops(&f)?),
                project(&f)?,
                ViewerLocale::English,
                ViewerView::Overview,
                "en".into(),
                "127.0.0.1:3219".parse()?,
            )?;
            let target=format!("/citation?subject=work:{}&language=en&publication={}&realization={}&unit={unit}&citation={index}&offset=0",work(&f)?,selected.publication_id,selected.realization_fingerprint);
            let mut response = Vec::new();
            server.serve_connection(
                &mut format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n").as_bytes(),
                &mut response,
            )?;
            let response = String::from_utf8(response)?;
            assert!(response.starts_with("HTTP/1.1 200"));
            let mut decorated: Value =
                serde_json::from_str(response.split_once("\r\n\r\n").ok_or("body")?.1)?;
            assert_eq!(decorated["highlight"]["language"], "python");
            assert!(!decorated["highlight"]["tokens"]
                .as_array()
                .ok_or("tokens")?
                .is_empty());
            decorated
                .as_object_mut()
                .ok_or("object")?
                .remove("highlight");
            assert_same_answer_basis(decorated, result.clone());
            assert_eq!(
                result["chunk"]["text"],
                if index == 1 && comparison == "changed" {
                    "def value():\n    return 4\n"
                } else {
                    "def value():\n    return 5\n"
                }
            );
            if index == 0 {
                assert_eq!(result["chunk"]["comparison"], comparison);
            }
        }
        assert_eq!(before, operations.canonical_basis(project(&f)?)?);
        assert_eq!(privacy, operations.privacy_status(project(&f)?)?);
    }
    Ok(())
}

#[test]
fn authored_topics_drive_navigation_and_keep_verification_and_limits_in_separate_layers(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let f = seed(temp.path(), "missing_before_state")?;
    let mut host = HostAdapter::new(ops(&f)?);
    record(&f, &mut host)?;
    let generation = call(
        &mut host,
        "work_explanation",
        json!({"action":"generate",
        "project_id":f["project"],"work_item_id":f["work"],"language":"en"}),
    )?;
    call(
        &mut host,
        "work_explanation",
        json!({"action":"read_source","project_id":f["project"],
        "work_item_id":f["work"],"generation_id":generation["generation_id"],"source_id":f["after"]}),
    )?;
    let units = [
        ("heading", "Investigate a constant return"),
        (
            "synopsis",
            "A source investigation, with no patch or execution.",
        ),
        ("section_heading", "How the constant reaches the caller"),
        (
            "narrative",
            "The observed definition returns the literal 5.",
        ),
        ("section_heading", "What this inspection establishes"),
        (
            "narrative",
            "This is a static source reading, not a runtime result.",
        ),
        ("section_heading", "Execution was not observed"),
        (
            "verification",
            "No command was executed for this investigation.",
        ),
        ("section_heading", "No downstream coverage"),
        ("limits", "Caller behavior remains unverified."),
    ];
    let paragraphs: Vec<_> = units
        .iter()
        .map(|(q, t)| json!({"question":q,"text":t,"evidence_keys":["goal"],"sources":[]}))
        .collect();
    let composed = call(
        &mut host,
        "work_explanation",
        json!({"action":"compose","project_id":f["project"],
        "work_item_id":f["work"],"generation_id":generation["generation_id"],"draft":{
            "plan_fingerprint":generation["plan"]["fingerprint"],"language":"en",
            "generator":{"host":"authored-control","session":"topic-reader","agent":null,"model":null},"paragraphs":paragraphs}}),
    )?;
    call(
        &mut host,
        "work_explanation",
        json!({"action":"record","project_id":f["project"],
        "work_item_id":f["work"],"generation_id":generation["generation_id"],"realization":composed["realization"]}),
    )?;
    let public = public_answers(&f, &mut host)?;
    assert_eq!(public["sections"][0]["heading_index"], 2);
    assert_eq!(public["sections"][1]["unit_indices"], json!([5]));
    assert_eq!(public["sections"][2]["layer"], "verification");
    assert_eq!(public["sections"][3]["layer"], "limits");
    let canonical = ops(&f)?.canonical_basis(project(&f)?)?;
    let derived = ops(&f)?.privacy_status(project(&f)?)?.managed_derived;
    for page in [
        get(&f)?,
        ViewerAdapter::new(ops(&f)?)
            .render_snapshot(&request(&f)?, TimestampMicros::from_unix_micros(1))?
            .html,
    ] {
        let document = page
            .split(&format!(
                "<article class=\"doc\" data-work-id=\"{}\"",
                f["work"].as_str().ok_or("Work")?
            ))
            .nth(1)
            .ok_or("Work document")?
            .split("</details></div></article>")
            .next()
            .ok_or("document end")?;
        let page = document;
        let nav = page
            .split("class=\"document-nav\"")
            .nth(1)
            .ok_or("nav")?
            .split("</nav>")
            .next()
            .ok_or("nav end")?;
        assert_eq!(document.matches("class=\"document-nav\"").count(), 1);
        assert!(nav.contains("How the constant reaches the caller"));
        assert!(nav.contains("What this inspection establishes"));
        assert!(!nav.contains("Execution was not observed"));
        assert!(
            nav.contains("No downstream coverage"),
            "visible nested limit heading belongs in the full contents"
        );
        let article = page
            .find("How the constant reaches the caller</h2>")
            .ok_or("authored topic")?;
        let verification = page
            .find("class=\"reading-section verification-report\"")
            .ok_or("verification")?;
        let limits = page
            .find("class=\"reading-section limits-section\"")
            .ok_or("limits")?;
        let index = page
            .find("class=\"reading-section evidence-index\"")
            .ok_or("index")?;
        assert!(article < verification && verification < limits && limits < index);
        assert!(page.contains("Execution was not observed</h3>"));
        assert!(page.contains("No downstream coverage</h3>"));
        assert!(
            page[verification..limits].contains("No command was executed for this investigation.")
        );
        assert!(!page[article..verification]
            .contains("No command was executed for this investigation."));
    }
    assert_eq!(canonical, ops(&f)?.canonical_basis(project(&f)?)?);
    assert_eq!(
        derived,
        ops(&f)?.privacy_status(project(&f)?)?.managed_derived
    );
    Ok(())
}
