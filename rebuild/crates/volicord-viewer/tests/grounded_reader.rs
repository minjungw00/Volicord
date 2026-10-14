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

fn seed(root: &Path, comparison: &str) -> Result<Value, Box<dyn Error>> {
    let repo = root.join("repo");
    fs::create_dir_all(&repo)?;
    fs::write(repo.join("main.py"), "def value():\n    return 4\n")?;
    let ops = LocalOperations::new(RuntimeLayout::new(root.join("runtime"))?);
    let project = ops
        .initialize_project("Understand a constant return", Some(&repo))?
        .project
        .id;
    let goal = "Explain why value returns 5, using observed code and any retained comparison. Do not infer execution success.";
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
        "work":work.to_string(),"before":before.map(|id|id.to_string()),"after":after.to_string(),
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
    let base = json!({"project_id":f["project"],"work_item_id":f["work"],"language":"en","action":"generate"});
    let generation = call(host, "work_explanation", base.clone())?;
    let plan: ExplanationPlan = serde_json::from_value(generation["plan"].clone())?;
    let mut citations = Vec::new();
    let text="The observed function returns the constant 5. Execution and user acceptance are not established.";
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
        c["claim_end_byte"] = json!(43);
        c["claim_sha256"] = json!(explanation_text_hash(&text[..43]));
        c["use_kind"] = serde_json::to_value(use_kind)?;
        citations.push(serde_json::from_value(c)?);
    }
    let realization = ExplanationRealization {
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint,
        language: "en".into(),
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
                "No runtime effect has been verified.",
            ),
            (
                ExplanationQuestion::Verification,
                "goal",
                "No execution verification was recorded.",
            ),
            (
                ExplanationQuestion::NextStep,
                "next_step",
                "No next action has been recorded.",
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
        assert_eq!(answers.source_excerpts[0].work_revision, 1);
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
        assert!(page.contains("<code lang=\"\">def value():\n    return 5\n</code>"));
        assert!(page.contains("data-title-basis=\"goal-quotation\">Explain why value returns 5, using observed code and any retained compar…</h3>"));
        assert_eq!(
            page.matches("Understand the constant return in value().")
                .count(),
            1
        );
        assert!(page.contains("Change / investigation · main.py:1–2</a>"));
        assert!(page.contains(fingerprint));
        let mut code_request = request(&f)?;
        code_request.view = ViewerView::Code {
            scope: volicord_viewer::CodeScope::Work(Some(work(&f)?)),
            entity: None,
        };
        let code = ViewerAdapter::new(ops(&f)?)
            .render(&code_request, "test-form-authenticity")?
            .html;
        assert!(code.contains("<code lang=\"\">def value():\n    return 5\n</code>"));
        assert!(code
            .contains("data-question=\"ReportedChange\" lang=\"en\" data-content-language=\"en\""));
        assert!(code.contains(fingerprint));
        let destination = temp.path().join("snapshot.html");
        ViewerAdapter::new(ops(&f)?).export_snapshot(&request(&f)?, &destination)?;
        let snapshot = fs::read_to_string(destination)?;
        assert!(snapshot.contains(fingerprint));
        assert!(snapshot.contains("return 5"));
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
        assert!(!stale.contains("<code lang=\"\">def value():"));
        assert!(stale.contains("Explanation is stale"));
    }
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
    assert_eq!(page.matches(&purpose).count(), 1);
    assert!(!page.contains(&format!("{purpose}</h3>")));
    assert!(page.contains("data-title-basis=\"goal-quotation\""));
    assert!(page.contains("class=\"explanation-authority\"><summary>About this explanation"));
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
    assert!(page.contains("Explanation was explicitly deleted"));
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
    assert!(page.contains("withheld by the canonical forgetting read barrier"));
    assert!(!page.contains("<code lang=\"\">def value():"));
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
    assert!(page.contains("Explanation was explicitly deleted"));
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
    assert!(page.contains("<code lang=\"\">def value():\n    return 5\n</code>"));
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
            &[provenance]
        )
        .is_err());
    assert!(!denied.exists());
    assert!(!get(&f)?.contains("<code lang=\"\">def value():"));
    Ok(())
}
