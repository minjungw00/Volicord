//! Expected claims come from canonical fields, not production reading helpers.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use reading_fixture::{fixture_scenario, SCENARIO};
use serde_json::{json, Value};
use volicord_operations::LocalOperations;
use volicord_viewer::{ViewerAdapter, ViewerLocale, ViewerServer, ViewerView};

fn get(server: &ViewerServer, path: &str) -> String {
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n");
    let mut out = Vec::new();
    server
        .serve_connection(&mut request.as_bytes(), &mut out)
        .expect("HTTP");
    String::from_utf8(out).expect("UTF-8")
}
fn server(f: &reading_fixture::Fixture) -> ViewerServer {
    ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(f.operations.layout().clone())),
        f.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse().expect("address"),
    )
    .expect("server")
}
fn scenario() -> Value {
    let mut input: Value = serde_json::from_str(SCENARIO).expect("fixture");
    input["prior_checkpoint_count"] = json!(0);
    input["later_checkpoint_count"] = json!(0);
    input
}
// A disclosed quotation establishes selection/evidence, never ordinary-reading
// explanation success. That separate claim is exercised by the actual-host browser.
#[test]
fn selected_result_and_state_have_independent_prefix_basis(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::{ExplanationSubject, ReadingRecord, WorkSelector};
    for (prefix, result_key, state, verification) in [
        (0, None, "Open", None),
        (1, Some("change"), "Completed", Some("Passed")),
        (2, Some("change"), "Paused", Some("Failed")),
        (3, Some("later_change"), "Completed", Some("NotRun")),
    ] {
        let mut input = scenario();
        input["works"][0]["checkpoints"]
            .as_array_mut()
            .ok_or("history")?
            .truncate(prefix);
        let f = fixture_scenario(input)?;
        let work = f.goals["older"];
        let p = f
            .operations
            .project_projection_selected(f.project, WorkSelector::ExactWork(work))?;
        let w = p.selected_work.ok_or("selected Work")?;
        assert_eq!(format!("{:?}", w.state), state);
        match result_key {
            Some(key) => assert_eq!(
                w.reading
                    .answers
                    .result
                    .as_ref()
                    .ok_or("selected result")?
                    .basis
                    .record,
                ReadingRecord::Checkpoint(f.checkpoints[key])
            ),
            None => assert!(w.reading.answers.result.is_none()),
        }
        let observed = w
            .reading
            .answers
            .verification
            .as_ref()
            .and_then(|s| s.verification.first())
            .map(|v| format!("{:?}", v.state));
        assert_eq!(observed.as_deref(), verification);
        let plan =
            f.operations
                .prepare_explanation(f.project, ExplanationSubject::Work(work), "en")?;
        if let Some(key) = result_key {
            let basis = plan
                .evidence
                .iter()
                .find(|e| e.key == "result")
                .ok_or("result")?;
            assert_eq!(basis.identity, f.checkpoints[key].to_string());
            assert_eq!(basis.revision, 1);
            assert_eq!(basis.field, "state_change");
            assert!(!basis.sources.is_empty());
        }
        let page = get(&server(&f), &format!("/?view=work&work={work}"));
        assert!(page.contains("Interpretation has not been generated"));
        assert!(!page.contains("class=\"work-explanation\""));
        assert_eq!(page.contains("class=\"result-evidence\""), prefix > 0);
    }
    Ok(())
}
#[test]
fn korean_visible_state_answers_are_localized() -> Result<(), Box<dyn std::error::Error>> {
    let f = fixture_scenario(scenario())?;
    let page = get(
        &server(&f),
        &format!("/?view=work&work={}&locale=ko", f.goals["older"]),
    );
    let states = page
        .split("class=\"fact-states\"")
        .nth(1)
        .ok_or("states")?
        .split("</div>")
        .next()
        .ok_or("state end")?;
    for expected in ["완료", "실행하지 않음", "요청하지 않음", "거부됨"] {
        assert!(states.contains(expected), "missing {expected}: {states}");
    }
    for wire in ["Completed", "NotRun", "NotRequested", "Rejected"] {
        assert!(
            !states.contains(wire),
            "wire name in visible state panel: {states}"
        );
    }
    Ok(())
}
#[test]
fn current_work_survives_a_catalog_full_of_completed_work() -> Result<(), Box<dyn std::error::Error>>
{
    let mut input = scenario();
    let template = input["works"][1].clone();
    input["works"] = json!([]);
    input["decisions"] = json!([]);
    for n in 0..90 {
        let mut work = template.clone();
        work["key"] = json!(format!("catalog-{n}"));
        work["title"] = json!(format!("Catalog Work {n}"));
        work["checkpoints"] = json!([]);
        input["works"].as_array_mut().ok_or("works")?.push(work);
    }
    let f = fixture_scenario(input)?;
    let canonical = f.operations.canonical_basis(f.project)?;
    let source = canonical
        .sources
        .iter()
        .find(|s| {
            matches!(
                s.source.payload,
                volicord_context::SourcePayload::RepositoryCommit { .. }
            )
        })
        .ok_or("Source")?
        .source
        .id;
    let current = *f.goals.values().max().ok_or("Goal")?;
    let mut store = volicord_context::Store::open(f.operations.layout().canonical_store())?;
    for (n, id) in f
        .goals
        .values()
        .filter(|id| **id != current)
        .chain(std::iter::once(&current))
        .enumerate()
    {
        store.record_checkpoint(
            volicord_context::OperationId::from_bytes((200_000_u128 + n as u128).to_le_bytes()),
            f.project,
            volicord_context::CheckpointDraft {
                expected_project_revision: canonical.project.revision,
                work_item_id: Some(*id),
                kind: volicord_context::CheckpointKind::Handoff,
                goal: "Catalog".into(),
                work_state: if *id == current {
                    volicord_context::WorkState::InProgress
                } else {
                    volicord_context::WorkState::Completed
                },
                state_change: Some("Reported catalog change".into()),
                source_basis: vec![source],
                changed_source_basis: Vec::new(),
                changed_paths: Vec::new(),
                applied_decisions: Vec::new(),
                verification: Vec::new(),
                user_review: volicord_context::UserReviewFact {
                    state: volicord_context::UserReviewState::NotRequested,
                    source_id: None,
                },
                user_acceptance: volicord_context::UserAcceptanceFact {
                    state: volicord_context::UserAcceptanceState::NotRequested,
                    source_id: None,
                },
                known_limits: Vec::new(),
                non_goals: Vec::new(),
                open_questions: Vec::new(),
                next_step: "Continue this Work".into(),
                handoff_to: Some("next-agent".into()),
            },
        )?;
    }
    drop(store);
    let history = f.operations.canonical_basis(f.project)?.checkpoint_history;
    assert!(
        history.windows(2).any(|p| p[0].id > p[1].id),
        "fixture must reverse chronological and identity order"
    );
    let page = get(&server(&f), "/?view=overview");
    let current_section = page
        .split("Current Work</h3>")
        .nth(1)
        .ok_or("current section")?
        .split("Recent outcomes</h3>")
        .next()
        .ok_or("end")?;
    assert!(
        current_section.contains(&current.to_string()),
        "current Work lost to catalog pagination"
    );
    use volicord_projections::{
        build_project_understanding, ProjectionDetail, UnderstandingBound, WorkSelector,
    };
    let mut seen = None;
    for page in [0, 1] {
        let (projection, _) = f.operations.project_projection_detail_profiled(
            f.project,
            WorkSelector::Repository,
            ProjectionDetail {
                work_page: page,
                ..Default::default()
            },
        )?;
        let understanding = build_project_understanding(&projection, UnderstandingBound::default());
        assert_eq!(understanding.work_overview.current.total, 1);
        assert_eq!(understanding.work_overview.current.omitted, 0);
        assert!(understanding.work_overview.current.complete);
        assert_eq!(understanding.work_overview.remaining.total, 1);
        assert_eq!(understanding.work_overview.remaining.omitted, 0);
        assert!(understanding.work_overview.remaining.complete);
        assert_eq!(understanding.work_overview.completed.total, 89);
        assert_eq!(understanding.work_overview.completed.items.len(), 8);
        assert_eq!(understanding.work_overview.completed.omitted, 81);
        assert!(understanding.work_overview.completed.complete);
        for pair in understanding.work_overview.completed.items.windows(2) {
            assert!(
                pair[0].reading.answers.result_observed_at
                    >= pair[1].reading.answers.result_observed_at
            );
        }
        if let Some(previous) = &seen {
            assert_eq!(previous, &understanding.work_overview);
        }
        seen = Some(understanding.work_overview);
    }
    Ok(())
}

#[test]
fn decision_facts_name_user_choice_and_agent_recommendation_independently(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = fixture_scenario(scenario())?;
    for (locale, choice, recommendation) in [
        (
            "en",
            "User choice: Local [local]",
            "Agent recommendation: Remote [remote]",
        ),
        (
            "ko",
            "사용자 선택: Local [local]",
            "에이전트 권고: Remote [remote]",
        ),
    ] {
        let page = get(
            &server(&f),
            &format!(
                "/?view=decisions&decision={}&locale={locale}",
                f.decisions["explicit"]
            ),
        );
        let facts = page
            .split("class=\"fact-states\"")
            .nth(1)
            .ok_or("facts")?
            .split("</div>")
            .next()
            .ok_or("end")?;
        assert!(facts.contains(choice), "unattributed user choice: {facts}");
        assert!(
            facts.contains(recommendation),
            "unattributed agent recommendation: {facts}"
        );
    }
    Ok(())
}
