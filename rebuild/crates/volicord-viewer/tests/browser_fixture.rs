//! Runtime construction only: browser expectations are independently authored in V11.
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use std::{env, fs};
use volicord_context::*;

#[test]
fn seed_browser_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let disposable = if env::var_os("VOLICORD_VIEWER_FIXTURE_ROOT").is_none() {
        Some(tempfile::tempdir()?)
    } else {
        None
    };
    let output = env::var_os("VOLICORD_VIEWER_FIXTURE_ROOT")
        .map(std::path::PathBuf::from)
        .or_else(|| disposable.as_ref().map(|d| d.path().to_owned()))
        .ok_or("fixture output")?;
    let fixture = reading_fixture::fixture_in(&output)?;
    let prefix = "relay_boundary_with_a_very_long_common_prefix_for_distinguishing_labels_";
    let alpha = format!("{prefix}alpha");
    let beta = format!("{prefix}beta");
    let wide = "릴레이경계".repeat(10);
    let broad = format!("{}suffix", "W".repeat(60));
    fs::write(
        fixture.repository.join("unsafe<&>.py"),
        format!("def {alpha}():\n    return {beta}()\n\ndef {beta}():\n    return {alpha}()\n\ndef {wide}():\n    return 1\n\ndef {broad}():\n    return 2\n"),
    )?;
    let canonical = fixture.operations.canonical_basis(fixture.project)?;
    let decision_sources = fixture
        .decisions
        .iter()
        .map(|(key, id)| {
            let decision = canonical
                .active_decisions
                .iter()
                .chain(&canonical.superseded_decisions)
                .find(|d| d.decision.id == *id)
                .ok_or("fixture Decision")?;
            Ok((
                key.clone(),
                serde_json::json!({
                    "user":decision.decision.user_turn_source_id.to_string(),
                    "recommendation":decision.decision.displayed_recommendation.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
                }),
            ))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>, &str>>()?;
    let source = canonical
        .sources
        .iter()
        .find(|s| matches!(s.source.payload, SourcePayload::CurrentHostUserTurn { .. }))
        .ok_or("user Source")?
        .source
        .id;
    let mut store = Store::open(fixture.operations.layout().canonical_store())?;
    let mut preceding = 0;
    for n in 0_u128..4096 {
        let goal = store
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
                        identity: "browser-fixture".into(),
                    },
                    source_basis: vec![source],
                    applicability: ApplicabilityScope::default(),
                },
            )?
            .value
            .id;
        preceding += usize::from(goal < fixture.goals["older"]);
        if n >= 70 && preceding >= 65 {
            break;
        }
    }
    assert!(
        preceding >= 65,
        "fixture must put older Work beyond the first 64 identities"
    );
    drop(store);
    let absent = fixture
        .operations
        .initialize_project("Purpose absent", None)?
        .project;
    let mut store = Store::open(fixture.operations.layout().canonical_store())?;
    let author = Principal {
        kind: PrincipalKind::User,
        identity: "browser-fixture".into(),
    };
    let source = store
        .record_source(
            OperationId::from_bytes(100_001_u128.to_le_bytes()),
            absent.id,
            SourceDraft {
                expected_project_revision: absent.revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "test".into(),
                    session: "synthetic".into(),
                    turn: "A Goal never supplies missing Project Purpose".into(),
                },
                actor: author.clone(),
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value
        .id;
    store.record_context_item(
        OperationId::from_bytes(100_002_u128.to_le_bytes()),
        absent.id,
        ContextItemDraft {
            expected_project_revision: absent.revision,
            role: ContextItemRole::Goal,
            statement: "Goal must not replace Project Purpose".into(),
            provenance_role: StatementProvenanceRole::UserStatement,
            author,
            source_basis: vec![source],
            applicability: ApplicabilityScope::default(),
        },
    )?;
    drop(store);
    let analysis = fixture
        .operations
        .analyze(fixture.project, Vec::new())?
        .value
        .ok_or("analysis")?
        .analysis;
    let entities: Vec<_> = analysis
        .structural_facts
        .iter()
        .map(|f| {
            serde_json::json!({
                "id": f.entity.identity, "name": f.entity.display_name, "path": f.entity.area.path,
                "source": f.entity.source, "range": f.entity.source_range,
            })
        })
        .collect();
    let relations: Vec<_> = analysis
        .structural_facts
        .iter()
        .flat_map(|f| &f.relations)
        .collect();
    let manifest = serde_json::json!({
        "kind":"viewer_browser_runtime", "project": fixture.project.to_string(),
        "runtime": fixture.operations.layout().root(), "repository": fixture.repository,
        "goals": fixture.goals.iter().map(|(k,v)|(k,v.to_string())).collect::<std::collections::BTreeMap<_,_>>(), "checkpoints": fixture.checkpoints.iter().map(|(k,v)|(k,v.to_string())).collect::<std::collections::BTreeMap<_,_>>(), "decisions": fixture.decisions.iter().map(|(k,v)|(k,v.to_string())).collect::<std::collections::BTreeMap<_,_>>(),
        "purpose":fixture.purpose.to_string(), "purpose_absent_project":absent.id.to_string(), "analysis_directory":fixture.operations.layout().analysis_project_dir(fixture.project), "entities": entities, "relations":relations, "decision_sources":decision_sources,
        "analysis_snapshot":analysis.identity.to_string(),"repository_snapshot":analysis.repository_snapshot.to_string(),
    });
    fs::write(
        output.join("fixture.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    if disposable.is_none() {
        let _retained = fixture._temporary.keep();
    }
    Ok(())
}
