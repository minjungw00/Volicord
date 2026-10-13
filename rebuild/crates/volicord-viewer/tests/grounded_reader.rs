//! Authored semantic expectations; real Product storage and public consumers.
//! Test prose is structural support, never active-host or human qualification.
use serde_json::{json, Value};
use std::{error::Error, fs, path::Path};
use volicord_context::*;
use volicord_host::HostAdapter;
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
        c["start_byte"] = json!(0);
        c["end_byte"] = json!(body.len());
        c["start_line"] = json!(1);
        c["end_line"] = json!(2);
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
            (
                ExplanationQuestion::Purpose,
                "goal",
                "Understand the constant return in value().",
            ),
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
        assert!(page.contains("<h3 lang=\"en\">Understand the constant return in value().</h3>"));
        assert!(page.contains(fingerprint));
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
