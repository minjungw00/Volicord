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
// Explicit baseline reproductions are excluded until their replacements connect.
#[test]
#[ignore = "R1 baseline reproduction; run explicitly before replacing selection"]
fn result_survives_null_and_blank_verification_prefixes() -> Result<(), Box<dyn std::error::Error>>
{
    for blank in [Value::Null, json!("  \n ")] {
        let mut input = scenario();
        input["works"][0]["checkpoints"]
            .as_array_mut()
            .ok_or("history")?
            .truncate(2);
        input["works"][0]["checkpoints"][0]["state_change"] =
            json!("Reported relay implementation change");
        input["works"][0]["checkpoints"][1]["state_change"] = blank;
        let f = fixture_scenario(input)?;
        let page = get(&server(&f), "/?view=overview");
        let card = page
            .split(&format!("data-work-id=\"{}\"", f.goals["older"]))
            .nth(1)
            .ok_or("Work omitted")?
            .split("</article>")
            .next()
            .ok_or("card")?;
        assert!(
            card.contains("Reported relay implementation change"),
            "recorded result disappeared: {card}"
        );
    }
    Ok(())
}
#[test]
#[ignore = "R3 baseline reproduction; run explicitly before localizing state answers"]
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
        .split("</dl>")
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
#[ignore = "R2 baseline reproduction; run explicitly before independent Overview selection"]
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
    Ok(())
}
