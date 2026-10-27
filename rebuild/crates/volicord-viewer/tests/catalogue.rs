#[path = "support/catalogue_fixture.rs"]
mod support;
use support::*;
use volicord_context::*;
use volicord_operations::*;
use volicord_projections::*;
use volicord_viewer::*;

#[test]
fn catalogue_searches_the_authorized_dataset_before_paging_and_never_prepares_articles(
) -> TestResult<()> {
    let f = fixture(33)?;
    let before = f.operations.canonical_basis(f.project)?;
    let query = WorkCatalogueQuery {
        term: "NEEDLE".into(),
        ..Default::default()
    };
    let first = f.operations.query_work_catalogue(f.project, &query, "en")?;
    assert_eq!(first.matched_count, 17);
    assert_eq!(first.entries.len(), 8);
    let mut expected = (0..33)
        .filter(|i| i % 2 == 0)
        .map(|i| f.goals[&format!("control-{i:03}")])
        .collect::<Vec<_>>();
    expected.reverse(); // Recording order is independent of random canonical identity.
    let mut actual = Vec::new();
    for page in 0..3 {
        let q = WorkCatalogueQuery {
            page,
            expected_basis: Some(first.basis.clone()),
            ..query.clone()
        };
        let result = f.operations.query_work_catalogue(f.project, &q, "en")?;
        actual.extend(result.entries.iter().map(|e| e.id));
        assert_eq!(result.cost.article_preparations, 0);
        assert_eq!(result.cost.source_body_reads, 0);
        assert_eq!(
            result.current.as_ref().map(|e| e.state),
            Some(UnderstandingWorkState::InProgress)
        );
    }
    assert_eq!(actual, expected);
    let query = WorkCatalogueQuery {
        term: "needle".into(),
        state: Some(UnderstandingWorkState::Completed),
        order: WorkCatalogueOrder::Oldest,
        ..Default::default()
    };
    let selected = f.operations.query_work_catalogue(f.project, &query, "en")?;
    assert_eq!(
        selected.entries.iter().map(|e| e.id).collect::<Vec<_>>(),
        (0..33)
            .filter(|i| i % 2 == 0 && i % 3 == 2)
            .map(|i| f.goals[&format!("control-{i:03}")])
            .collect::<Vec<_>>()
    );
    assert_eq!(
        f.operations
            .query_work_catalogue(f.project, &query, "ko")?
            .matched_count,
        0
    );
    let viewer = ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone()));
    let request = ViewerRequest {
        project_id: f.project,
        locale: ViewerLocale::Korean,
        requested_language: "en".into(),
        view: ViewerView::WorkQuery { query },
        guarded_request: None,
    };
    let (page, profile) = viewer.render_profiled(&request, "test")?;
    assert!(page.html.contains("role=\"search\""));
    assert!(!page.html.contains("class=\"document-nav\""));
    assert_eq!(profile.projection.work_read_cost.materialized_works, 0);
    assert_eq!(profile.projection.explanation_basis_preparations, 0);
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn replacement_deletion_foreign_scope_and_equal_names_reset_continuation_safely() -> TestResult<()>
{
    let f = fixture(17)?;
    for id in f.goals.values() {
        record(
            &f,
            ExplanationSubject::WorkName(*id),
            ExplanationQuestion::Heading,
            "Same 주제",
        )?;
    }
    let query = WorkCatalogueQuery {
        term: "same 주제".into(),
        order: WorkCatalogueOrder::Name,
        ..Default::default()
    };
    let first = f.operations.query_work_catalogue(f.project, &query, "en")?;
    let mut ids = f.goals.values().copied().collect::<Vec<_>>();
    ids.sort();
    assert_eq!(
        first.entries.iter().map(|e| e.id).collect::<Vec<_>>(),
        ids[..8]
    );
    let continuation = WorkCatalogueQuery {
        page: 1,
        expected_basis: Some(first.basis.clone()),
        ..query
    };
    f.operations
        .delete_explanations(f.project, ExplanationSubject::WorkName(ids[0]))?;
    let changed = f
        .operations
        .query_work_catalogue(f.project, &continuation, "en")?;
    assert!(changed.refreshed);
    assert_eq!(changed.query.page, 0);
    assert_eq!(changed.matched_count, 16);
    assert!(!changed.entries.iter().any(|e| e.id == ids[0]));
    let other = f
        .operations
        .initialize_project("Same display name", None)?
        .project
        .id;
    assert_eq!(
        f.operations
            .query_work_catalogue(other, &continuation, "en")?
            .matched_count,
        0
    );
    let source = f
        .operations
        .canonical_basis(f.project)?
        .sources
        .into_iter()
        .find(|s| matches!(s.source.payload, SourcePayload::CurrentHostUserTurn { .. }))
        .ok_or("user Source")?
        .source
        .id;
    let checkpoints = f
        .operations
        .canonical_basis(f.project)?
        .checkpoint_history
        .into_iter()
        .filter(|c| c.work_item_id == Some(ids[1]))
        .map(|c| c.id)
        .collect::<Vec<_>>();
    for checkpoint in checkpoints {
        f.operations
            .forget_record(f.project, CanonicalRecordId::Checkpoint(checkpoint), source)?;
    }
    f.operations
        .forget_record(f.project, CanonicalRecordId::ContextItem(ids[1]), source)?;
    let changed = f
        .operations
        .query_work_catalogue(f.project, &continuation, "en")?;
    assert!(changed.refreshed);
    assert_eq!(changed.matched_count, 15);
    assert!(!changed.entries.iter().any(|e| e.id == ids[1]));
    let goal = f
        .operations
        .canonical_basis(f.project)?
        .context_items
        .into_iter()
        .find(|g| g.id == ids[2])
        .ok_or("goal")?;
    f.operations.correct_context_item(
        f.project,
        ids[2],
        ContextItemCorrectionDraft {
            expected_revision: goal.revision,
            corrected_statement: format!("{}.", goal.statement),
            user_authorization_source_id: source,
            kind: CorrectionKind::Expression,
        },
    )?;
    let corrected = f
        .operations
        .query_work_catalogue(f.project, &continuation, "en")?;
    assert_eq!(corrected.matched_count, 14);
    assert!(!corrected.entries.iter().any(|e| e.id == ids[2]));

    Ok(())
}

#[test]
fn real_http_query_forms_are_bounded_and_preserve_exact_project() -> TestResult<()> {
    let f = fixture(17)?;
    let other = f
        .operations
        .initialize_project("Empty Project", None)?
        .project
        .id;
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone())),
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let read = |query: &str| -> TestResult<String> {
        let mut bytes = Vec::new();
        server.serve_connection(
            &mut std::io::Cursor::new(format!(
                "GET /?{query} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n"
            )),
            &mut bytes,
        )?;
        Ok(String::from_utf8(bytes)?)
    };
    let response = read("view=work&q=needle&sort=oldest")?;
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("data-result-count=\"9\""));
    assert_eq!(response.matches("class=\"work-row\"").count(), 8);
    assert!(response.contains("Cache-Control: no-store"));
    let empty = read(&format!("project={other}&view=work&q=needle"))?;
    assert!(empty.contains("data-result-count=\"0\""));
    assert!(!empty.contains("Synthetic summary"));
    for query in [
        "view=work&q=%00",
        "view=work&language=%00",
        "view=work&sort=unknown",
        "view=work&page=-1",
        "view=work&page=1000000",
        "view=work&basis=wrong",
        "view=overview&q=x",
        "view=work&q=x&q=y",
        "view=work&state=unknown",
    ] {
        assert!(read(query)?.starts_with("HTTP/1.1 400"), "{query}");
    }
    assert!(read(&format!("view=work&q={}", "x".repeat(129)))?.starts_with("HTTP/1.1 400"));
    Ok(())
}

#[test]
fn detail_timeline_and_native_return_keep_typed_query_scope_without_invented_history(
) -> TestResult<()> {
    let f = fixture(17)?;
    let before = f.operations.canonical_basis(f.project)?;
    let query = WorkCatalogueQuery {
        term: "needle".into(),
        order: WorkCatalogueOrder::Oldest,
        page: 1,
        ..Default::default()
    };
    let catalogue = f.operations.query_work_catalogue(f.project, &query, "en")?;
    let work = catalogue.entries[0].id;
    let other = f.operations.initialize_project("Foreign", None)?.project.id;
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone())),
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let read = |query: &str| -> TestResult<String> {
        let mut bytes = Vec::new();
        server.serve_connection(
            &mut std::io::Cursor::new(format!(
                "GET /?{query} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n"
            )),
            &mut bytes,
        )?;
        Ok(String::from_utf8(bytes)?)
    };
    let suffix = format!(
        "&from=list&q=needle&sort=oldest&page=1&basis={}",
        catalogue.basis
    );
    let detail = read(&format!("view=work&work={work}{suffix}"))?;
    assert!(detail.starts_with("HTTP/1.1 200"));
    assert!(detail.contains("Recorded timeline"));
    assert!(detail.contains("These are recorded reports; opening this page runs no checks."));
    assert_eq!(detail.matches("<li data-checkpoint-id=").count(), 1);
    assert!(detail.contains("Synthetic report 016; no actual execution"));
    let breadcrumb = detail
        .split("class=\"breadcrumb\"")
        .nth(1)
        .ok_or("breadcrumb")?
        .split("</p>")
        .next()
        .ok_or("breadcrumb end")?;
    assert!(breadcrumb.contains("q=needle"));
    assert!(breadcrumb.contains("page=1"));
    assert!(breadcrumb.contains("sort=oldest"));
    assert!(breadcrumb.contains(&format!("#row-{work}")));
    assert!(!breadcrumb.contains("from=list"));
    assert!(!breadcrumb.contains("work="));
    let direct = read(&format!("view=work&work={work}"))?;
    let direct_breadcrumb = direct
        .split("class=\"breadcrumb\"")
        .nth(1)
        .ok_or("direct breadcrumb")?
        .split("</p>")
        .next()
        .ok_or("end")?;
    assert!(!direct_breadcrumb.contains("q=needle"));
    assert!(!direct_breadcrumb.contains("#row-"));
    for request in [
        format!("view=work&work={}&{suffix}", "0".repeat(32)),
        format!("project={other}&view=work&work={work}{suffix}"),
    ] {
        let missing = read(&request.replace("&&", "&"))?;
        assert!(missing.starts_with("HTTP/1.1 404"));
        assert!(missing.contains("q=needle"));
        assert!(missing.contains("page=1"));
        assert!(!missing.contains("Synthetic report 016"));
        assert!(!missing.contains("class=\"document-nav\""));
    }
    for request in [
        format!("view=work&work={work}&from=https://elsewhere"),
        "view=work&from=list".into(),
        format!("view=work&work={work}&q=needle"),
        format!("view=work&work={work}&from=list&page=1000000"),
    ] {
        assert!(read(&request)?.starts_with("HTTP/1.1 400"));
    }
    assert_eq!(before, f.operations.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn single_summaryless_work_missing_dates_and_article_storage_stay_separate() -> TestResult<()> {
    let f = fixture(0)?; // The shared producer also records one unassociated Goal.
    let one = f
        .operations
        .query_work_catalogue(f.project, &WorkCatalogueQuery::default(), "en")?;
    assert_eq!(one.matched_count, 1);
    assert!(one.entries[0].name.is_none());
    assert!(one.current.is_none());
    let id = one.entries[0].id;
    let plan = f
        .operations
        .prepare_explanation(f.project, ExplanationSubject::Work(id), "en")?;
    let realization = f.operations.compose_work_explanation(
        f.project,
        id,
        WorkExplanationDraft {
            diagrams: vec![],
            plan_fingerprint: plan.fingerprint,
            language: "en".into(),
            generator: ExplanationGenerator {
                host: "synthetic-control".into(),
                session: "catalogue-cost".into(),
                agent: None,
                model: None,
            },
            paragraphs: vec![
                WorkReaderUnit {
                    question: ExplanationQuestion::Heading,
                    text: "Synthetic article".into(),
                    evidence_keys: vec!["goal".into()],
                    sources: vec![],
                },
                WorkReaderUnit {
                    question: ExplanationQuestion::Synopsis,
                    text: "Synthetic record-only control, no code claim.".into(),
                    evidence_keys: vec!["goal".into()],
                    sources: vec![],
                },
            ],
        },
    )?;
    let retained = f.operations.record_explanation(
        f.project,
        ExplanationSubject::Work(id),
        "en",
        realization,
    )?;
    let records = volicord_privacy::PrivacyStore::open(f.operations.layout().privacy_store())?
        .inspect_work_catalogue_with_invalidations(f.project, &[])?;
    assert!(records
        .managed_derived
        .iter()
        .find(|r| Some(r.id.to_string()) == retained.publication_id)
        .ok_or("article metadata")?
        .content
        .is_none());
    let after =
        f.operations
            .query_work_catalogue(f.project, &WorkCatalogueQuery::default(), "en")?;
    assert_eq!(after.entries[0].article, CatalogueArticle::Recorded);
    assert!(after.entries[0].name.is_none());
    assert!(after.entries[0].summary.is_none());
    assert_eq!(after.cost.article_preparations, 0);
    assert_eq!(after.cost.source_body_reads, 0);
    let mut canonical = f.operations.canonical_basis(f.project)?;
    canonical.revisions.clear();
    let no_dates = work_catalogue_metadata(&canonical);
    assert_eq!(no_dates[0].created_at, None);
    assert_eq!(no_dates[0].updated_at, None);
    Ok(())
}

#[test]
#[ignore = "explicit isolated browser seed; never writes the user's Runtime"]
fn seed_catalogue_browser() -> TestResult<()> {
    let output = std::path::PathBuf::from(
        std::env::var_os("VOLICORD_CATALOGUE_FIXTURE").ok_or("new output required")?,
    );
    std::fs::create_dir(&output)?;
    let count = std::env::var("VOLICORD_CATALOGUE_WORKS")
        .ok()
        .map(|n| n.parse::<usize>())
        .transpose()?
        .unwrap_or(33);
    if !(1..=512).contains(&count) {
        return Err("control count must be 1 to 512".into());
    }
    let f = fixture(count)?;
    let empty = f
        .operations
        .initialize_project("Empty Project", None)?
        .project
        .id;
    let canonical = f.operations.canonical_basis(f.project)?;
    let source = canonical
        .sources
        .iter()
        .find(|s| matches!(s.source.payload, SourcePayload::CurrentHostUserTurn { .. }))
        .ok_or("source")?
        .source
        .id;
    let page =
        f.operations
            .query_work_catalogue(f.project, &WorkCatalogueQuery::default(), "en")?;
    std::fs::write(
        output.join("input.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"kind":"synthetic_validation_only","count":count+1,"runtime":f.operations.layout().root(),"project":f.project.to_string(),"empty":empty.to_string(),"source":source.to_string(),"ids":f.goals.iter().map(|(k,id)|(k,id.to_string())).collect::<std::collections::BTreeMap<_,_>>(),"cost":{"classified":page.cost.classified_works,"storage_bytes":page.cost.storage_metadata_and_card_bytes,"article_preparations":page.cost.article_preparations,"source_reads":page.cost.source_body_reads}}),
        )?,
    )?;
    // Preserve only this explicitly requested disposable input, never a real home.
    let _ = f._temporary.keep();
    Ok(())
}

#[test]
fn equal_recorded_dates_use_identity_ties_in_every_order_through_ordinary_recording(
) -> TestResult<()> {
    let root = tempfile::tempdir()?;
    let ops = LocalOperations::new(RuntimeLayout::new(root.path().join("runtime"))?);
    let project = ops
        .initialize_project("Synthetic equal-date control", None)?
        .project;
    let mut store = Store::open_with(
        ops.layout().canonical_store(),
        SystemIdGenerator,
        FixedClock::new(TimestampMicros::from_unix_micros(1_790_000_000_000_000)),
    )?;
    let actor = Principal {
        kind: PrincipalKind::User,
        identity: "synthetic-date-control".into(),
    };
    let source = store
        .record_source(
            OperationId::from_bytes([40; 16]),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "validation".into(),
                    session: "equal-dates".into(),
                    turn: "Record three explicitly synthetic Goals with equal timestamps".into(),
                },
                actor: actor.clone(),
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value
        .id;
    let mut ids = Vec::new();
    for index in 41..44 {
        ids.push(
            store
                .record_context_item(
                    OperationId::from_bytes([index; 16]),
                    project.id,
                    ContextItemDraft {
                        expected_project_revision: project.revision,
                        role: ContextItemRole::Goal,
                        statement: "Synthetic equal-date request".into(),
                        provenance_role: StatementProvenanceRole::UserStatement,
                        author: actor.clone(),
                        source_basis: vec![source],
                        applicability: ApplicabilityScope::default(),
                    },
                )?
                .value
                .id,
        );
    }
    drop(store);
    ids.sort();
    for id in &ids {
        let subject = ExplanationSubject::WorkName(*id);
        let plan = ops.prepare_explanation(project.id, subject, "en")?;
        let realization = ops.compose_subject_explanation(
            project.id,
            subject,
            WorkExplanationDraft {
                diagrams: vec![],
                plan_fingerprint: plan.fingerprint,
                language: "en".into(),
                generator: ExplanationGenerator {
                    host: "synthetic-control".into(),
                    session: "equal-dates".into(),
                    agent: None,
                    model: None,
                },
                paragraphs: vec![WorkReaderUnit {
                    question: ExplanationQuestion::Heading,
                    text: "Repeated name".into(),
                    evidence_keys: vec!["goal".into()],
                    sources: vec![],
                }],
            },
        )?;
        ops.record_explanation(project.id, subject, "en", realization)?;
    }
    for order in [
        WorkCatalogueOrder::Recent,
        WorkCatalogueOrder::Oldest,
        WorkCatalogueOrder::Name,
    ] {
        let page = ops.query_work_catalogue(
            project.id,
            &WorkCatalogueQuery {
                order,
                ..Default::default()
            },
            "en",
        )?;
        assert_eq!(page.entries.iter().map(|e| e.id).collect::<Vec<_>>(), ids);
        assert!(page
            .entries
            .iter()
            .all(|e| e.updated_at == Some(1_790_000_000_000_000)));
    }
    Ok(())
}
