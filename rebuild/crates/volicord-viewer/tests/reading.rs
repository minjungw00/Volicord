#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
mod reading_fixture;
use reading_fixture::fixture;
use volicord_context::{ContextItemId, DecisionId};
use volicord_operations::LocalOperations;
use volicord_viewer::{
    ViewerAdapter, ViewerLocale, ViewerRequest, ViewerServer, ViewerTool, ViewerView,
};

fn exchange(server: &ViewerServer, path: &str) -> String {
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:3219\r\n\r\n");
    let mut output = Vec::new();
    server
        .serve_connection(&mut request.as_bytes(), &mut output)
        .expect("HTTP");
    String::from_utf8(output).expect("UTF-8")
}
#[test]
fn exact_routes_keep_older_work_failed_states_and_decision_rationales(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let viewer = ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone()));
    let server = ViewerServer::new(
        viewer,
        fixture.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    for locale in ["en", "ko"] {
        let page = exchange(
            &server,
            &format!(
                "/?view=work&work={}&locale={locale}&language=fr-CA",
                fixture.goals["older"]
            ),
        );
        assert!(page.starts_with("HTTP/1.1 200"));
        for key in ["change", "verification_only", "later_change"] {
            assert!(page.contains(&fixture.checkpoints[key].to_string()));
        }
        assert!(page.contains("Failed"));
        assert!(page.contains("Rejected"));
        assert!(page.contains("NotRun"));
        assert!(page.contains("Semantic summary unavailable") || page.contains("의미 요약"));
        assert!(!page.contains(&fixture.decisions["other_work"].to_string()));
        let decision = exchange(
            &server,
            &format!(
                "/?view=decisions&decision={}&locale={locale}",
                fixture.decisions["explicit"]
            ),
        );
        assert!(decision.starts_with("HTTP/1.1 200"));
        assert!(decision.contains("user_rationale"));
        assert!(decision.contains("recommendation_rationale"));
    }
    let goal_only = exchange(
        &server,
        &format!("/?view=work&work={}", fixture.goals["goal_only"]),
    );
    assert!(goal_only.contains("Goal only"));
    let code = exchange(
        &server,
        &format!("/?view=code&scope=work&work={}", fixture.goals["older"]),
    );
    assert!(code.contains("Selected Work context"));
    assert!(code.contains("python/worker.py"));
    assert!(code.contains("CallsSyntactically"));
    let without_purpose = exchange(&server, "/?view=overview");
    assert!(without_purpose.contains("Project Understanding"));
    assert!(!without_purpose.contains("request_authenticity"));
    assert!(fixture.repository.is_dir());
    assert!(before.context_items.iter().any(|c| c.id == fixture.purpose));
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}
#[test]
fn malformed_removed_and_foreign_selectors_never_fallback() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = fixture()?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone())),
        fixture.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    for path in [
        "/?level=overview",
        "/?view=invalid",
        "/?view=work&work=bad",
        "/?view=overview&work=bad",
        "/?view=code",
        "/?view=code&scope=repository&work=bad",
        "/?view=tools&tool=invalid",
        "/?view=decisions&decision=bad",
        "/?view=code&scope=repository&entity=",
    ] {
        assert!(
            exchange(&server, path).starts_with("HTTP/1.1 400"),
            "{path}"
        );
    }
    for path in [
        format!("/?view=work&work={}", ContextItemId::from_bytes([250; 16])),
        format!("/?view=work&work={}", fixture.purpose),
        format!(
            "/?view=decisions&decision={}",
            DecisionId::from_bytes([250; 16])
        ),
        "/?view=code&scope=repository&entity=foreign-entity".into(),
    ] {
        assert!(
            exchange(&server, &path).starts_with("HTTP/1.1 404"),
            "{path}"
        );
    }
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}
#[test]
fn bounded_whole_snapshot_has_only_unique_existing_fragment_targets(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let viewer = ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone()));
    let request = ViewerRequest {
        project_id: fixture.project,
        locale: ViewerLocale::English,
        view: ViewerView::Tools {
            tool: ViewerTool::Documents,
        },
        requested_language: "en".into(),
        guarded_request: None,
    };
    let page = viewer
        .render_snapshot(
            &request,
            volicord_context::TimestampMicros::from_unix_micros(123),
        )?
        .html;
    let mut ids = std::collections::BTreeSet::new();
    for rest in page.split(" id=\"").skip(1) {
        assert!(ids.insert(rest.split('"').next().ok_or("id")?));
    }
    for rest in page.split(" href=\"").skip(1) {
        let href = rest.split('"').next().ok_or("href")?;
        assert!(href.starts_with('#'));
        assert!(ids.contains(&href[1..]), "{href}");
    }
    assert_eq!(page.matches("class=\"document-preview\"").count(), 4);
    for forbidden in [
        "<form",
        "<script",
        " src=",
        "request_authenticity",
        "href=\"/?",
        "href=\"http",
    ] {
        assert!(!page.contains(forbidden));
    }
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}

#[test]
fn work_index_pages_reach_items_omitted_from_initial_lists(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_context::*;
    let fixture = fixture()?;
    let canonical = fixture.operations.canonical_basis(fixture.project)?;
    let mut store = Store::open(fixture.operations.layout().canonical_store())?;
    let source = canonical
        .sources
        .iter()
        .find(|s| matches!(s.source.payload, SourcePayload::CurrentHostUserTurn { .. }))
        .ok_or("user Source")?
        .source
        .id;
    let mut goals = Vec::new();
    for n in 0_u128..70 {
        goals.push(
            store
                .record_context_item(
                    OperationId::from_bytes((90_000 + n).to_le_bytes()),
                    fixture.project,
                    ContextItemDraft {
                        expected_project_revision: canonical.project.revision,
                        role: ContextItemRole::Goal,
                        statement: format!("Paged Goal {n}"),
                        provenance_role: StatementProvenanceRole::UserStatement,
                        author: Principal {
                            kind: PrincipalKind::User,
                            identity: "fixture".into(),
                        },
                        source_basis: vec![source],
                        applicability: ApplicabilityScope::default(),
                    },
                )?
                .value
                .id,
        );
    }
    drop(store);
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone())),
        fixture.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let first = exchange(&server, "/?view=work&locale=ko&language=fr-CA");
    let second = exchange(&server, "/?view=work&page=1&locale=ko&language=fr-CA");
    assert!(first.contains("page=1"));
    assert!(first.contains("language=fr-CA"));
    let omitted = goals
        .iter()
        .find(|id| !first.contains(&format!("work={id}")))
        .ok_or("all unexpectedly included")?;
    assert!(second.contains(&format!("work={omitted}")));
    assert!(exchange(&server, &format!("/?view=work&work={omitted}")).starts_with("HTTP/1.1 200"));
    for path in [
        "/?view=overview&page=1",
        "/?view=work&page=-1",
        "/?view=work&page=1000000",
        "/?view=work&page=1&work=bad",
    ] {
        assert!(exchange(&server, path).starts_with("HTTP/1.1 400"));
    }
    Ok(())
}

#[test]
fn code_detail_selects_beyond_map_bounds_and_keeps_readable_real_endpoints(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    for n in 0..90 {
        std::fs::write(
            fixture
                .repository
                .join(format!("long_shared_module_prefix_{n:03}.py")),
            format!("def distinguishable_symbol_{n}():\n    return 1\n"),
        )?;
    }
    std::fs::write(
        fixture.repository.join("unsafe<&>.py"),
        "def safe():\n    return 1\n",
    )?;
    let analysis = fixture
        .operations
        .analyze(fixture.project, Vec::new())?
        .value
        .ok_or("analysis output")?
        .analysis;
    let projection = fixture.operations.project_projection_selected(
        fixture.project,
        volicord_projections::WorkSelector::Repository,
    )?;
    let outside = analysis
        .structural_facts
        .iter()
        .find(|f| {
            !projection
                .repository_map
                .entities
                .iter()
                .any(|e| e.identity == f.entity.identity)
        })
        .ok_or("no omitted entity")?;
    let encode = |id: &str| id.bytes().map(|b| format!("%{b:02X}")).collect::<String>();
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let server = ViewerServer::new(
        ViewerAdapter::new(LocalOperations::new(fixture.operations.layout().clone())),
        fixture.project,
        ViewerLocale::English,
        ViewerView::Overview,
        "en".into(),
        "127.0.0.1:3219".parse()?,
    )?;
    let page = exchange(
        &server,
        &format!(
            "/?view=code&scope=repository&entity={}",
            encode(&outside.entity.identity)
        ),
    );
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(page.contains("Incoming relationships"));
    assert!(page.contains("Outgoing relationships"));
    assert!(page.contains("Source locator and retained range"));
    assert!(page.contains(&outside.entity.identity));
    let unsafe_entity = analysis
        .structural_facts
        .iter()
        .find(|f| f.entity.area.path == "unsafe<&>.py")
        .ok_or("unsafe locator fixture")?;
    let unsafe_page = exchange(
        &server,
        &format!(
            "/?view=code&scope=repository&entity={}",
            encode(&unsafe_entity.entity.identity)
        ),
    );
    assert!(unsafe_page.starts_with("HTTP/1.1 200"));
    assert!(unsafe_page.contains("unsafe&lt;&amp;&gt;.py"));
    assert!(!unsafe_page.contains("unsafe<&>.py"));
    assert!(exchange(
        &server,
        &format!(
            "/?view=code&scope=work&work={}&entity={}",
            fixture.goals["older"],
            encode(&outside.entity.identity)
        )
    )
    .starts_with("HTTP/1.1 404"));
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}
