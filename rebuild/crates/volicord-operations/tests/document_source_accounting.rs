//! A display catalogue bound is not the membership of exact Work proof.
#[path = "support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use volicord_context::{Availability, SourceFreshness, SourceId, TimestampMicros};
use volicord_projections::{
    generate_documents, DocumentRequest, FixedLocale, GeneratorIdentity, ProjectProjection,
    WorkSelector,
};

fn source_state(
    p: &mut ProjectProjection,
    id: SourceId,
    availability: Option<Availability>,
    freshness: SourceFreshness,
) {
    for w in p
        .work_history
        .iter_mut()
        .chain(p.selected_work.iter_mut())
        .chain(&mut p.work_overview.current.items)
        .chain(&mut p.work_overview.completed.items)
        .chain(&mut p.work_overview.remaining.items)
        .chain(&mut p.work_overview.next_steps.items)
    {
        for s in &mut w.reading.evidence_source_status {
            if s.source_id == id {
                s.availability = availability;
                s.freshness = freshness;
            }
        }
    }
}
#[test]
fn known_work_sources_outside_catalogue_keep_documents_grounded_without_widening_it(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?;
    let mut p = f
        .operations
        .project_projection_selected(f.project, WorkSelector::ExactWork(f.goals["older"]))?;
    let request = DocumentRequest {
        requested_language: "en".into(),
        fixed_locale: FixedLocale::English,
        generated_at: TimestampMicros::from_unix_micros(1),
        generator: GeneratorIdentity {
            generator: "authored-source-bound-control".into(),
            agent: None,
            model: None,
        },
        requested_destinations: vec![],
    };
    // Preserve all Work/history/graph data and bound only the display catalogue.
    let id = p
        .selected_work
        .as_ref()
        .ok_or("Work")?
        .reading
        .evidence_source_status
        .iter()
        .find(|s| {
            s.availability.is_some()
                && p.source_catalog.iter().any(|c| c.source.id == s.source_id)
                && !p
                    .resume
                    .decisions
                    .iter()
                    .flat_map(|d| {
                        d.question_source_status
                            .iter()
                            .chain(&d.user_source_status)
                            .chain(&d.recommendation_source_status)
                    })
                    .any(|d| d.source_id == s.source_id)
        })
        .ok_or("Work-only canonical witness")?
        .source_id;
    p.source_catalog.retain(|source| source.source.id != id);
    let catalogue_bound = p.source_catalog.len();
    for (availability, freshness) in [
        (Availability::Available, SourceFreshness::Stale),
        (Availability::Unavailable, SourceFreshness::Unavailable),
    ] {
        source_state(&mut p, id, Some(availability), freshness);
        let docs = generate_documents(&p, &request)?;
        assert_eq!(
            p.source_catalog.len(),
            catalogue_bound,
            "no bound relaxation"
        );
        let used = [
            &docs.project_architecture_guide,
            &docs.decision_report,
            &docs.implementation_plan,
            &docs.handoff_resume,
        ]
        .into_iter()
        .flat_map(|d| &d.metadata.used_sources)
        .filter(|s| s.source_id == id)
        .collect::<Vec<_>>();
        assert!(
            !used.is_empty(),
            "the actual document retains its exact Source basis"
        );
        assert!(
            used.iter().all(|s| s.freshness == freshness),
            "no invented currentness"
        );
    }
    source_state(
        &mut p,
        id,
        Some(Availability::Available),
        SourceFreshness::Current,
    );
    p.selected_work
        .as_mut()
        .ok_or("Work")?
        .reading
        .evidence_source_status
        .iter_mut()
        .find(|s| s.source_id == id)
        .ok_or("witness")?
        .freshness = SourceFreshness::Stale;
    let conflicted = generate_documents(&p, &request)?;
    assert!(
        conflicted
            .handoff_resume
            .metadata
            .used_sources
            .iter()
            .filter(|s| s.source_id == id)
            .all(|s| s.freshness == SourceFreshness::Unknown),
        "conflicting witnesses cannot establish currentness"
    );
    source_state(&mut p, id, None, SourceFreshness::Unknown);
    let error = generate_documents(&p, &request)
        .err()
        .ok_or("missing Source witness was admitted")?;
    assert!(
        error.to_string().contains(&format!("unknown Source {id}")),
        "an ID alone cannot grant membership"
    );
    Ok(())
}
