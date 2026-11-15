//! Synthetic, repository-independent technical controls. No model/semantic verdict.
use std::{error::Error, fs};
use volicord_operations::*;
use volicord_projections::*;
use volicord_viewer::*;

#[test]
fn repository_introduction_is_not_user_purpose_and_formatting_keeps_lifecycle(
) -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let output = std::env::var_os("VOLICORD_EDITORIAL_REVIEW_ROOT").map(std::path::PathBuf::from);
    if let Some(output) = &output {
        fs::create_dir(output)?;
    }
    let repository = temp.path().join("greeting-engine");
    fs::create_dir(&repository)?;
    let code = "def greet(name):\n    return 'hello ' + name\n";
    fs::write(repository.join("main.py"), code)?;
    let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
    let project = ops
        .initialize_project("Greeting engine", Some(&repository))?
        .project
        .id;
    let analysis = ops
        .analyze(project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis
        .identity;
    let receipt = ops.observe_repository_file(RepositoryFileObservationRequest {
        project_id: project,
        analysis_snapshot_id: analysis,
        path: "main.py".into(),
        byte_range: None,
        retain_seconds: 3600,
    })?;
    let source = receipt["source_id"].as_str().ok_or("source")?.to_owned();
    let mut source_bytes = [0u8; 16];
    for (i, byte) in source_bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&source[i * 2..i * 2 + 2], 16)?;
    }
    let source_id = volicord_context::SourceId::from_bytes(source_bytes);
    let subject = ExplanationSubject::Repository(analysis);
    for language in ["en", "ko"] {
        let plan = ops.prepare_explanation(project, subject, language)?;
        let synopsis = if language == "en" {
            "A small greeting engine joins a greeting to an input name."
        } else {
            "입력한 이름에 인사말을 붙여 주는 작은 프로그램입니다."
        };
        let paragraphs=[
            (ExplanationQuestion::Heading,"Greeting engine"),
            (ExplanationQuestion::Synopsis,synopsis),
            (ExplanationQuestion::Narrative,"Illustrative input and output, inferred from the inspected function; no execution is claimed."),
            (ExplanationQuestion::ReadingPoint,"```json\n{\"input\":\"Ada\",\"result\":\"hello Ada\",\"literal\":\"<script>\"}\n```"),
            (ExplanationQuestion::Narrative,"| Input | Expected prefix |\n| --- | --- |\n| Ada | hello |\n| λ \\| 이름 | hello |"),
            (ExplanationQuestion::Limits,"The example explains the selected function, not execution or whole-repository coverage."),
        ].into_iter().map(|(question,text)|{
            let claims=if text.starts_with('|') {text.split_inclusive('\n').scan(0,|offset,line|{let start=*offset;*offset+=line.len();Some((start,*offset))}).skip(2).map(|(start,end)|Some(FileByteRange{start_byte:start,end_byte:end})).collect()}else{vec![None]};
            RepositoryReaderUnit{question,text:text.into(),evidence_keys:vec!["repository_basis".into()],sources:if question==ExplanationQuestion::Limits {vec![]}else{claims.into_iter().map(|claim|RepositoryClaimRange{source_id,start_byte:0,end_byte:code.len(),claim}).collect()}}
        }).collect();
        let response = ops.compose_repository_explanation(
            project,
            analysis,
            RepositoryExplanationDraft {
                diagrams: vec![],
                plan_fingerprint: plan.fingerprint,
                language: language.into(),
                generator: ExplanationGenerator {
                    host: "synthetic-editorial-test".into(),
                    session: "format-and-lifecycle".into(),
                    agent: None,
                    model: None,
                },
                paragraphs,
            },
        )?;
        ops.record_explanation(project, subject, language, response)?;
        let before = ops.canonical_basis(project)?;
        let privacy = ops.privacy_status(project)?;
        let query = WorkCatalogueQuery::default();
        let (catalogue, profile) =
            ops.project_catalogue_projection_profiled(project, &query, language)?;
        assert!(catalogue.repository_explanations.is_empty());
        assert_eq!(profile.explanation_basis_preparations, 0);
        let (overview, profile) =
            ops.project_overview_projection_profiled(project, &query, language)?;
        assert_eq!(overview.repository_explanations.len(), 1);
        assert_eq!(overview.repository_explanations[0].language, language);
        assert_eq!(profile.explanation_basis_preparations, 1);
        assert_eq!(profile.analysis_snapshot_decodes, 0);
        assert_eq!(profile.candidate_reads, 0);
        let viewer = ViewerAdapter::new(LocalOperations::new(ops.layout().clone()));
        let request = ViewerRequest {
            project_id: project,
            locale: if language == "ko" {
                ViewerLocale::Korean
            } else {
                ViewerLocale::English
            },
            requested_language: language.into(),
            view: ViewerView::Overview,
            guarded_request: None,
        };
        let html = viewer.render(&request, "intro-control")?.html;
        let card = html
            .split("class=\"repository-card\"")
            .nth(1)
            .ok_or("repository card")?
            .split("</section>")
            .next()
            .ok_or("card end")?;
        assert!(card.contains(synopsis));
        assert!(html.contains(if language == "ko" {
            "기록된 프로젝트 목적이 없습니다"
        } else {
            "No Project purpose has been recorded"
        }));
        assert!(!before
            .context_items
            .iter()
            .any(|item| item.role == volicord_context::ContextItemRole::ProjectPurpose));
        let snapshot = viewer
            .render_snapshot(
                &request,
                volicord_context::TimestampMicros::from_unix_micros(123),
            )?
            .html;
        assert!(snapshot.contains("class=\"authored-table\""));
        assert!(snapshot.contains("class=\"authored-example\""));
        let introduction = snapshot
            .split("class=\"repository-card\"")
            .nth(1)
            .ok_or("native intro")?
            .split("</section>")
            .next()
            .ok_or("end intro")?;
        assert!(
            introduction.contains(synopsis),
            "native Overview must share the live introduction"
        );
        let claim = snapshot
            .split("class=\"evidence-claim\"")
            .skip(1)
            .any(|part| {
                part.split("<template")
                    .next()
                    .is_some_and(|s| s.contains("class=\"authored-table\""))
            });
        assert!(
            claim,
            "table context is formatted in actual evidence claims"
        );
        assert!(snapshot.contains("class=\"claim-selected\" aria-current=\"true\""));
        assert!(snapshot.contains("class=\"table-evidence-actions\""));
        assert!(snapshot.contains("class=\"claim-format\""));
        assert!(!snapshot.contains("<script>"));
        if let Some(output) = &output {
            fs::write(output.join(format!("{language}.html")), &snapshot)?;
        }
        assert_eq!(before, ops.canonical_basis(project)?);
        assert_eq!(privacy, ops.privacy_status(project)?);
    }
    let viewer = ViewerAdapter::new(LocalOperations::new(ops.layout().clone()));
    let request = ViewerRequest {
        project_id: project,
        locale: ViewerLocale::English,
        requested_language: "fr".into(),
        view: ViewerView::Overview,
        guarded_request: None,
    };
    let unavailable = viewer.render(&request, "no-language-fallback")?.html;
    let card = unavailable
        .split("class=\"repository-card\"")
        .nth(1)
        .ok_or("card")?
        .split("</section>")
        .next()
        .ok_or("end")?;
    assert!(!card.contains("joins a greeting"));
    assert!(card.contains("No repository introduction is available"));
    ops.delete_explanations(project, subject)?;
    let request = ViewerRequest {
        requested_language: "en".into(),
        ..request
    };
    let removed = viewer
        .render_snapshot(
            &request,
            volicord_context::TimestampMicros::from_unix_micros(123),
        )?
        .html;
    assert!(!removed.contains("joins a greeting"));
    assert!(!removed.contains("class=\"authored-example\""));
    assert!(!removed.contains("class=\"authored-table\""));
    if let Some(output) = &output {
        fs::write(output.join("deleted.html"), removed)?;
    }
    Ok(())
}
