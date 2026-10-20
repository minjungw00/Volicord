//! Authored, source-first public consumer controls; not a model or human verdict.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use serde_json::{json, Value};
use std::error::Error;
use volicord_context::*;
use volicord_host::HostAdapter;
use volicord_operations::*;
use volicord_projections::*;
use volicord_viewer::*;

fn call(host: &mut HostAdapter, tool: &str, args: Value) -> Result<Value, Box<dyn Error>> {
    let result = host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":tool,"arguments":args}})).ok_or("MCP response")?;
    assert_eq!(result["result"]["isError"], false, "{result}");
    Ok(result["result"]["structuredContent"].clone())
}
fn fixture() -> Result<reading_fixture::Fixture, Box<dyn Error>> {
    let mut input: Value = serde_json::from_str(reading_fixture::SCENARIO)?;
    input["prior_checkpoint_count"] = json!(0);
    input["later_checkpoint_count"] = json!(0);
    input["decisions"] = json!([{"key":"explicit","scope":"relay","user_rationale":null,"recommendation_rationale":"Keep the observed boundary local","state":"ReviewDue"}]);
    input["unassociated_checkpoint"] = json!(false);
    input["works"] = json!([{"key":"relay","title":"Investigate whether readQuery changes the supplied value.","paths":["runtime/query_boundary.ts"],"checkpoints":[{
        "key":"inspection","work":"Paused","verification":"NotRun","review":"Pending","acceptance":"NotRequested",
        "state_change":"Inspected readQuery: it returns the supplied value. No patch was made.","next_step":"Ask for review of the investigation","limits":["No automated verification was run."]}]}]);
    reading_fixture::fixture_scenario(input)
}
fn get(f: &reading_fixture::Fixture, path: &str) -> Result<String, Box<dyn Error>> {
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone())),
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n");
    let mut out = Vec::new();
    server.serve_connection(&mut request.as_bytes(), &mut out)?;
    let html = String::from_utf8(out)?;
    assert!(html.starts_with("HTTP/1.1 200"), "{html}");
    Ok(html)
}
fn cli(f: &reading_fixture::Fixture, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let argv = [
        "--runtime".to_owned(),
        f.operations.layout().root().to_string_lossy().into_owned(),
        "--project".into(),
        f.project.to_string(),
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_cli(
            argv.iter().map(String::as_str).chain(args.iter().copied()),
            &mut out,
            &mut err
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&err)
    );
    Ok(String::from_utf8(out)?)
}
fn produce(
    host: &mut HostAdapter,
    f: &reading_fixture::Fixture,
    language: &str,
    scope: &str,
    source: SourceId,
    text: &[&str],
) -> Result<Value, Box<dyn Error>> {
    let id = f.goals["relay"];
    let base =
        json!({"project_id":f.project.to_string(),"work_item_id":id.to_string(),"scope":scope});
    let mut generate = base.clone();
    generate["action"] = json!("generate");
    generate["language"] = json!(language);
    let request = call(host, "work_explanation", generate)?;
    assert_eq!(request["generation_status"], "awaiting_host_interpretation");
    let mut args = base.clone();
    args["generation_id"] = request["generation_id"].clone();
    let mut units = match scope {
        "name" => {
            vec![json!({"question":"heading","text":text[0],"evidence_keys":["goal"],"sources":[]})]
        }
        "summary" => vec![
            json!({"question":"synopsis","text":text[1],"evidence_keys":["goal","result"],"sources":[]}),
        ],
        _ => vec![
            json!({"question":"heading","text":if language=="ko" {"관측한 쿼리 반환 구조"}else{"Observed query return mechanism"},"evidence_keys":["goal"],"sources":[]}),
            json!({"question":"synopsis","text":text[1],"evidence_keys":["goal","result"],"sources":[]}),
        ],
    };
    if scope == "article" {
        let mut read = args.clone();
        read["action"] = json!("read_source");
        read["source_id"] = json!(source.to_string());
        let observed = call(host, "work_explanation", read)?;
        assert_eq!(
            observed["observation"]["detail"]["body"],
            "export function readQuery(value: unknown) { return value; }\n"
        );
        units.push(json!({"question":"observation","text":text[2],"evidence_keys":["goal","result"],"sources":[{"source_id":source.to_string(),"field":"body","use_kind":"current_code","start_byte":0,"end_byte":57,"claim":null}]}));
    }
    args["action"] = json!("compose");
    args["draft"] = json!({"plan_fingerprint":request["plan"]["fingerprint"],"language":language,"generator":{"host":"authored-public-consumer-control","session":"scope-isolation","agent":null,"model":null},"paragraphs":units});
    let composed = call(host, "work_explanation", args.clone())?;
    args.as_object_mut().ok_or("args")?.remove("draft");
    args["action"] = json!("record");
    args["realization"] = composed["realization"].clone();
    // A current request cannot silently become a different publication.
    let mut wrong = args.clone();
    wrong["scope"] = json!(if scope == "article" {
        "summary"
    } else {
        "article"
    });
    let denied=host.handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"work_explanation","arguments":wrong}})).ok_or("denial")?;
    assert_eq!(denied["result"]["isError"], true);
    call(host, "work_explanation", args)
}

#[test]
fn same_work_independent_publications_reach_cli_mcp_http_documents_and_offline(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = f.goals["relay"];
    let analysis = f
        .operations
        .analyze(f.project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let source = f
        .operations
        .observe_work_file(FileObservationRequest {
            project_id: f.project,
            work_item_id: id,
            analysis_snapshot_id: analysis,
            path: "runtime/query_boundary.ts".into(),
            byte_range: None,
            before_source_id: None,
            retain_seconds: 3600,
        })?
        .source_id;
    let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let cases=[("en",ViewerLocale::English,["Investigate query value preservation","The investigation reports that readQuery preserves its input; no patch was made and automated verification was not run.","The observed implementation returns the input value directly."]),
        ("ko",ViewerLocale::Korean,["쿼리 입력값 보존 조사","조사 기록에 따르면 readQuery는 입력값을 그대로 반환합니다. 코드는 바꾸지 않았고 자동 검증은 실행하지 않았습니다.","관측한 구현은 입력값을 직접 반환합니다."])];
    for (language, _, text) in &cases {
        for scope in ["name", "summary", "article"] {
            produce(&mut host, &f, language, scope, source, text)?;
        }
    }
    let canonical = f.operations.canonical_basis(f.project)?;
    let privacy = f.operations.privacy_status(f.project)?;
    let adapter = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let mut normal_snapshots = Vec::new();
    for (language, locale, text) in &cases {
        let status: Value =
            serde_json::from_str(&cli(&f, &["--json", "status", "--language", language])?)?;
        let selected = &status["selected_work"];
        assert_eq!(selected["work_item_id"], id.to_string(), "{selected}");
        assert_eq!(selected["reading_title"], text[0], "{selected}");
        assert_eq!(
            selected["answers"]["work_summary"]["prose"][0]["text"],
            text[1]
        );
        assert_eq!(selected["answers"]["prose"][2]["text"], text[2]);
        assert_ne!(
            selected["creation_event"],
            selected["latest_work_record_event"]
        );
        assert!(cli(&f, &["status", "--language", language])?.contains(text[1]));
        for tool in ["recall", "repository_understanding"] {
            let read = call(
                &mut host,
                tool,
                json!({"project_id":f.project.to_string(),"requested_language":language,"fixed_locale":language}),
            )?;
            assert!(
                read.to_string().contains(text[0]) && read.to_string().contains(text[1]),
                "{tool}"
            );
        }
        for path in [
            format!("/?view=overview&language={language}&locale={language}"),
            format!("/?view=work&language={language}&locale={language}"),
            format!("/?view=work&work={id}&language={language}&locale={language}"),
        ] {
            let live = get(&f, &path)?;
            assert!(live.contains(text[0]) && live.contains(text[1]), "{path}");
            if path.contains("&work=") {
                assert!(live.contains(&f.decisions["explicit"].to_string()));
                let decision = get(
                    &f,
                    &format!("/?view=work&work={id}&locale={language}&language={language}",),
                )?;
                assert!(decision.contains("Keep the observed boundary local"));
            }
        }
        let request = ViewerRequest {
            project_id: f.project,
            locale: *locale,
            view: ViewerView::Work { work: Some(id) },
            requested_language: language.to_string(),
            guarded_request: None,
        };
        let snapshot = adapter.render_snapshot(&request, TimestampMicros::from_unix_micros(123))?;
        assert!(
            snapshot.html.contains(text[0])
                && snapshot.html.contains(text[1])
                && snapshot.html.contains(text[2])
        );
        assert!(!snapshot.html.contains("<script"));
        assert!(snapshot
            .explanations
            .iter()
            .any(|p| p.subject == ExplanationSubject::WorkName(id)));
        normal_snapshots.push((request, snapshot, selected.clone()));
    }
    assert_eq!(canonical, f.operations.canonical_basis(f.project)?);
    assert_eq!(privacy, f.operations.privacy_status(f.project)?);
    f.operations
        .delete_work_source_observation(f.project, id, source)?;
    for ((language, _, text), (request, normal, original)) in cases.iter().zip(normal_snapshots) {
        let destination = f
            ._temporary
            .path()
            .join(format!("obsolete-{language}.html"));
        assert!(f
            .operations
            .publish_viewer_snapshot(
                &normal.html,
                &destination,
                f.project,
                &normal.canonical_read_fingerprint,
                &normal.explanations,
                &normal.verification_basis
            )
            .is_err());
        assert!(!destination.exists());
        let status: Value =
            serde_json::from_str(&cli(&f, &["--json", "status", "--language", language])?)?;
        let selected = &status["selected_work"];
        assert_eq!(selected["work_item_id"], id.to_string(), "{selected}");
        assert_eq!(selected["reading_title"], text[0], "{selected}");
        assert_eq!(
            selected["answers"]["work_name"],
            original["answers"]["work_name"]
        );
        assert_eq!(
            selected["answers"]["work_summary"],
            original["answers"]["work_summary"]
        );
        assert_eq!(selected["answers"]["explanation_state"], "stale");
        assert!(!selected["answers"]["prose"].to_string().contains(text[2]));
        for field in [
            "state",
            "creation_event",
            "latest_work_record_event",
            "verification_report",
            "decision_ids",
        ] {
            assert_eq!(selected[field], original[field], "{field}");
        }
        let live = get(
            &f,
            &format!("/?view=work&work={id}&language={language}&locale={language}"),
        )?;
        assert!(live.contains(text[0]) && live.contains(text[1]));
        assert!(!live.contains(text[2]));
        assert!(live.contains(&f.decisions["explicit"].to_string()));
        let decision = get(
            &f,
            &format!("/?view=work&work={id}&locale={language}&language={language}",),
        )?;
        assert!(decision.contains("Keep the observed boundary local"));
        let snapshot = adapter.render_snapshot(&request, TimestampMicros::from_unix_micros(124))?;
        assert!(snapshot.html.contains(text[0]) && snapshot.html.contains(text[1]));
        assert!(!snapshot.html.contains(text[2]));
        let projection = f
            .operations
            .project_projection_selected(f.project, WorkSelector::exact(&id.to_string())?)?;
        let documents = f.operations.documents_from_projection(
            &projection,
            &DocumentRequest {
                requested_language: language.to_string(),
                fixed_locale: if *language == "ko" {
                    FixedLocale::Korean
                } else {
                    FixedLocale::English
                },
                generated_at: TimestampMicros::from_unix_micros(125),
                generator: GeneratorIdentity {
                    generator: "authored-consumer-test".into(),
                    agent: None,
                    model: None,
                },
                requested_destinations: vec![],
            },
        )?;
        for document in [
            &documents.project_architecture_guide,
            &documents.decision_report,
            &documents.implementation_plan,
            &documents.handoff_resume,
        ] {
            let rendered = &document.markdown.content;
            if document.metadata.document_kind != DocumentKind::DecisionReport {
                assert!(rendered.contains(text[0]) && rendered.contains(text[1]));
            }
            assert!(!rendered.contains(text[2]));
            assert!(document
                .metadata
                .explanations
                .iter()
                .all(|p| p.subject != ExplanationSubject::Work(id)));
        }
    }
    Ok(())
}

#[test]
fn cited_summary_has_its_own_exact_http_authority_and_cannot_borrow_article_identity(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = f.goals["relay"];
    let analysis = f
        .operations
        .analyze(f.project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let source = f
        .operations
        .observe_work_file(FileObservationRequest {
            project_id: f.project,
            work_item_id: id,
            analysis_snapshot_id: analysis,
            path: "runtime/query_boundary.ts".into(),
            byte_range: None,
            before_source_id: None,
            retain_seconds: 3600,
        })?
        .source_id;
    let plan =
        f.operations
            .prepare_explanation(f.project, ExplanationSubject::WorkSummary(id), "en")?;
    let response=f.operations.compose_subject_explanation(f.project,plan.subject,WorkExplanationDraft {
        plan_fingerprint:plan.fingerprint,language:"en".into(),generator:ExplanationGenerator {host:"authored-control".into(),session:"summary-citation".into(),agent:None,model:None},
        paragraphs:vec![WorkReaderUnit {question:ExplanationQuestion::Synopsis,text:"The report describes an unchanged input return; the observed implementation directly returns value.".into(),evidence_keys:vec!["goal".into(),"result".into()],sources:vec![WorkClaimRange {source_id:source.to_string(),field:"body".into(),use_kind:ExplanationSourceUse::CurrentCode,start_byte:0,end_byte:57,claim:None}]}]})?;
    let retained = f.operations.record_explanation(
        f.project,
        ExplanationSubject::WorkSummary(id),
        "en",
        response,
    )?;
    let provenance = ExplanationProvenance::from(&retained);
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone())),
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let request = |subject: &str| {
        format!("GET /citation?subject={subject}%3A{id}&language=en&realization={}&unit=0&citation=0&offset=0 HTTP/1.1\r\nHost: 127.0.0.1:3219\r\nOrigin: http://127.0.0.1:3219\r\nSec-Fetch-Site: same-origin\r\n\r\n",provenance.realization_fingerprint)
    };
    let read = |subject: &str| -> Result<String, Box<dyn Error>> {
        let mut output = Vec::new();
        server.serve_connection(&mut request(subject).as_bytes(), &mut output)?;
        Ok(String::from_utf8(output)?)
    };
    let canonical = f.operations.canonical_basis(f.project)?;
    let privacy = f.operations.privacy_status(f.project)?;
    let current = read("work_summary")?;
    assert!(current.starts_with("HTTP/1.1 200"));
    assert!(current.contains("return value"));
    let foreign = read("work")?;
    assert!(!foreign.starts_with("HTTP/1.1 200"));
    assert!(!foreign.contains("return value"));
    // A summary can remain readable independently of article availability.
    // Its Source pane must escape the article that becomes inert on narrow
    // screens; a focused descendant of an inert article is not interactive.
    {
        let viewer = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
        let request = ViewerRequest {
            project_id: f.project,
            locale: ViewerLocale::English,
            view: ViewerView::Work { work: Some(id) },
            requested_language: "en".into(),
            guarded_request: None,
        };
        for html in [
            get(&f, &format!("/?view=work&work={id}&language=en&locale=en"))?,
            viewer
                .render_snapshot(&request, TimestampMicros::from_unix_micros(123))?
                .html,
        ] {
            let selected = html
                .split(&format!("<article class=\"doc\" data-work-id=\"{id}\""))
                .nth(1)
                .ok_or("selected Work article")?;
            let (document, sources) = selected.split_once("</article>").ok_or("article end")?;
            assert!(
                !document.contains("class=\"source-point\""),
                "Source dialog inherits article inertness"
            );
            assert!(sources.contains("class=\"source-point\""));
        }
    }
    assert_eq!(canonical, f.operations.canonical_basis(f.project)?);
    assert_eq!(privacy, f.operations.privacy_status(f.project)?);
    let unsupported = get(&f, &format!("/?view=work&work={id}&language=fr&locale=en"))?;
    assert!(!unsupported.contains("The report describes an unchanged input return"));
    f.operations
        .delete_work_source_observation(f.project, id, source)?;
    let denied = read("work_summary")?;
    assert!(!denied.starts_with("HTTP/1.1 200"));
    assert!(!denied.contains("return value"));
    let projection = f
        .operations
        .project_projection_selected(f.project, WorkSelector::ExactWork(id))?;
    let answers = work_answers(
        projection.selected_work.as_ref().ok_or("Work")?,
        "en",
        FixedLocale::English,
    );
    assert!(answers.card_summary().is_none());
    assert_eq!(
        answers
            .work_summary
            .as_ref()
            .ok_or("summary")?
            .explanation_state,
        ExplanationState::Stale
    );
    assert_eq!(
        answers.explanation_state,
        ExplanationState::Missing,
        "an absent article cannot hide a stale summary"
    );
    Ok(())
}

#[test]
fn malformed_scope_cannot_default_to_article_or_consume_a_valid_host_request(
) -> Result<(), Box<dyn Error>> {
    let f = fixture()?;
    let id = f.goals["relay"];
    let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let before = f.operations.privacy_status(f.project)?;
    for scope in [json!(4), json!(null), json!("foreign"), json!(["name"])] {
        let result=host.handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"work_explanation","arguments":{"action":"generate","project_id":f.project.to_string(),"work_item_id":id.to_string(),"language":"en","scope":scope}}})).ok_or("response")?;
        assert_eq!(result["result"]["isError"], true, "{result}");
    }
    assert_eq!(before, f.operations.privacy_status(f.project)?);
    // Rejections create no retained state and do not prevent a valid explicit request.
    let result = call(
        &mut host,
        "work_explanation",
        json!({"action":"generate","project_id":f.project.to_string(),"work_item_id":id.to_string(),"language":"en","scope":"name"}),
    )?;
    assert_eq!(result["generation_status"], "awaiting_host_interpretation");
    Ok(())
}

/// Small current Product output for review; authored support, no human verdict.
#[test]
fn seed_reader_scope_review() -> Result<(), Box<dyn Error>> {
    let Some(root) = std::env::var_os("VOLICORD_READER_SCOPE_REVIEW") else {
        return Ok(());
    };
    let output = std::path::Path::new(&root);
    std::fs::create_dir_all(output)?;
    let f = fixture()?;
    for statement in [
        "Inspect missing error coverage before changing query handling.",
        "Document the boundary between query input and native execution.",
    ] {
        f.operations.transition_work(
            f.project,
            WorkTransition::StartNew {
                host: "authored-review-control".into(),
                session: "reader-scope".into(),
                user_turn: statement.into(),
                statement: statement.into(),
            },
        )?;
    }
    let analysis = f
        .operations
        .analyze(f.project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let source = f
        .operations
        .observe_work_file(FileObservationRequest {
            project_id: f.project,
            work_item_id: f.goals["relay"],
            analysis_snapshot_id: analysis,
            path: "runtime/query_boundary.ts".into(),
            byte_range: None,
            before_source_id: None,
            retain_seconds: 3600,
        })?
        .source_id;
    let mut host = HostAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let texts=[("en",["Investigate query value preservation","The investigation reports that readQuery preserves its input; no patch was made and automated verification was not run.","The observed implementation returns the input value directly."]),
        ("ko",["쿼리 입력값 보존 조사","조사 기록에 따르면 readQuery는 입력값을 그대로 반환합니다. 코드는 바꾸지 않았고 자동 검증은 실행하지 않았습니다.","관측한 구현은 입력값을 직접 반환합니다."])];
    for (language, text) in texts {
        for scope in ["name", "summary", "article"] {
            produce(&mut host, &f, language, scope, source, &text)?;
        }
    }
    let workless_repository = output.join("workless-repository");
    std::fs::create_dir_all(workless_repository.join("runtime"))?;
    std::fs::copy(
        f.repository.join("runtime/query_boundary.ts"),
        workless_repository.join("runtime/query_boundary.ts"),
    )?;
    let name = f
        .operations
        .canonical_basis(f.project)?
        .project
        .display_name;
    let workless = f
        .operations
        .initialize_project(&name, Some(&workless_repository))?
        .project
        .id;
    let mut cases = Vec::new();
    for (key, project) in [("populated", f.project), ("workless", workless)] {
        let analysis = f
            .operations
            .analyze(project, vec![])?
            .value
            .ok_or("analysis")?
            .analysis
            .identity;
        let retained = f
            .operations
            .observe_repository_file(RepositoryFileObservationRequest {
                project_id: project,
                analysis_snapshot_id: analysis,
                path: "runtime/query_boundary.ts".into(),
                byte_range: None,
                retain_seconds: 3600,
            })?;
        let source = source_id_from_hex(retained["source_id"].as_str().ok_or("Source")?)?;
        for language in ["en", "ko"] {
            let subject = ExplanationSubject::Repository(analysis);
            let plan = f
                .operations
                .prepare_explanation(project, subject, language)?;
            let realization=f.operations.compose_repository_explanation(project,analysis,RepositoryExplanationDraft {
                plan_fingerprint:plan.fingerprint,language:language.into(),generator:ExplanationGenerator {host:"authored-review-control".into(),session:"reader-scope".into(),agent:None,model:None},
                paragraphs:vec![RepositoryReaderUnit {question:ExplanationQuestion::Observation,text:if language=="ko" {"쿼리 경계는 입력값을 그대로 반환합니다. 이 관측은 TypeScript 진입점의 반환 동작만 설명하며 native 실행이나 전체 저장소 검증을 입증하지 않습니다."}else{"The query boundary returns its input unchanged. This observation explains the TypeScript entry point; it does not establish native execution or whole-repository verification."}.into(),evidence_keys:vec!["repository_basis".into()],sources:vec![RepositoryClaimRange {source_id:source,start_byte:0,end_byte:57,claim:None}]}],
            })?;
            f.operations
                .record_explanation(project, subject, language, realization)?;
        }
        let name = f.operations.canonical_basis(project)?.project.display_name;
        cases.push(json!({"key":key,"runtime":f.operations.layout().root(),"project":project.to_string(),"project_name":name,"purpose_count":if key=="populated" {1}else{0},"locale":"en","language":"en","evidence_class":"authored technical control; human acceptance unassessed","origin":"fresh Product fixture and current renderer","source_root":if key=="populated" {&f.repository}else{&workless_repository},"decision":if key=="populated" {Some(f.decisions["explicit"].to_string())}else{None},"work":if key=="populated" {Some(f.goals["relay"].to_string())}else{None},"normal_work":if key=="populated" {Some(f.goals["relay"].to_string())}else{None}}));
    }
    std::fs::write(
        output.join("cases.json"),
        serde_json::to_vec_pretty(&cases)?,
    )?;
    let _retained = f._temporary.keep();
    Ok(())
}
fn source_id_from_hex(value: &str) -> Result<SourceId, Box<dyn Error>> {
    let mut bytes = [0; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)?;
    }
    Ok(SourceId::from_bytes(bytes))
}
