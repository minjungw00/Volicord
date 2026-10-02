//! Independent synthetic counterexamples through the actual Local Operations API.
#[path = "support/reading_fixture.rs"]
mod reading_fixture;
use reading_fixture::{fixture, SCENARIO};
use serde_json::Value;
use std::{collections::BTreeSet, fs};
use volicord_context::*;
use volicord_operations::{run_cli, CliExit};
use volicord_projections::*;
use volicord_repository_intelligence::{
    AnalysisProvenance, AnalyzerIdentity, CodeEntityKind, ProvenanceClass, RelationTarget,
    SemanticAnalysisResult, SemanticProvenance, SemanticRelation, SemanticRelationKind,
};

const EXPECTED: &str = include_str!(
    "../../../validation/end-to-end/multi-repository/fixtures/viewer-reading/expected.json"
);

#[test]
fn exact_work_reads_complete_history_before_all_bounds_and_keeps_independent_states(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let expected: Value = serde_json::from_str(EXPECTED)?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let default = fixture.operations.project_projection(fixture.project)?;
    assert!(!default
        .checkpoint_timeline
        .iter()
        .any(|entry| entry.checkpoint.id == fixture.checkpoints["change"]));
    let projection = fixture.operations.project_projection_selected(
        fixture.project,
        WorkSelector::ExactWork(fixture.goals["older"]),
    )?;
    let understanding = build_project_understanding(
        &projection,
        UnderstandingBound {
            max_items_per_section: 1,
        },
    );
    let work = understanding
        .selected_work
        .as_ref()
        .ok_or("selected Work missing")?;
    assert_eq!(work.work_item_id, fixture.goals["older"]);
    assert_eq!(
        work.checkpoint_ids,
        expected["exact_older"]["checkpoint_keys"]
            .as_array()
            .ok_or("expected keys")?
            .iter()
            .map(|key| fixture.checkpoints[key.as_str().expect("key")])
            .collect::<Vec<_>>()
    );
    assert_eq!(
        work.changed_paths,
        expected["exact_older"]["paths"]
            .as_array()
            .ok_or("expected paths")?
            .iter()
            .map(|p| p.as_str().expect("path").to_owned())
            .collect::<Vec<_>>()
    );
    assert!(!work.decision_ids.contains(&fixture.decisions["other_work"]));
    for key in expected["exact_older"]["decision_keys"]
        .as_array()
        .ok_or("expected Decisions")?
    {
        assert!(work
            .decision_ids
            .contains(&fixture.decisions[key.as_str().expect("key")]));
    }
    assert_eq!(work.state, UnderstandingWorkState::Completed);
    assert_eq!(
        work.reading.states[0].verification[0].state,
        VerificationState::Passed
    );
    assert_eq!(
        work.reading.states[1].verification[0].state,
        VerificationState::Failed
    );
    assert_eq!(
        work.reading.states[2].verification[0].state,
        VerificationState::NotRun
    );
    assert_eq!(
        work.reading.states[0].user_review.state,
        UserReviewState::Pending
    );
    assert_eq!(
        work.reading.states[1].user_review.state,
        UserReviewState::Reviewed
    );
    assert_eq!(
        work.reading.states[2].user_acceptance.state,
        UserAcceptanceState::Rejected
    );
    assert_eq!(
        work.reading.states[0].later_changed_checkpoint_ids,
        [fixture.checkpoints["later_change"]]
    );
    assert_eq!(
        work.reading.states[1].later_changed_checkpoint_ids,
        [fixture.checkpoints["later_change"]]
    );
    let first = &work.reading.changes[0];
    assert_eq!(first.representation, ReadingRepresentation::Excerpt);
    assert!(!first.semantic_summary_available);
    assert!(first.display_english.starts_with("Excerpt"));
    assert!(first.display_korean.starts_with("발췌"));
    assert!(first
        .display_english
        .contains("aabbccddeeff00112233445566778899"));
    assert!(first
        .original_text
        .as_ref()
        .ok_or("original")?
        .contains("원문 근거"));
    let original = first.original_text.as_ref().ok_or("original")?;
    let prefix = first
        .display_english
        .strip_prefix("Excerpt (original language): ")
        .ok_or("excerpt label")?;
    assert_eq!(first.omitted_utf8_bytes, original.len() - prefix.len());
    assert_eq!(
        first.omitted_characters,
        original.chars().count() - prefix.chars().count()
    );
    assert_eq!(
        first.basis.record,
        ReadingRecord::Checkpoint(fixture.checkpoints["change"])
    );
    assert_eq!(first.basis.field, "state_change");
    assert_eq!(
        work.reading.changes[1].representation,
        ReadingRepresentation::Unavailable
    );
    let explicit = understanding
        .selected_work_decisions
        .iter()
        .find(|d| d.decision.decision_id == fixture.decisions["explicit"])
        .ok_or("selected Decision")?;
    assert!(explicit.decision.user_rationale.is_none());
    assert_eq!(
        explicit.reading.user_rationale.availability,
        ReadingAvailability::Unavailable
    );
    assert_eq!(
        explicit.decision.chosen_alternative_key.as_deref(),
        Some("local")
    );
    assert_eq!(
        explicit.decision.recommended_alternative_key.as_deref(),
        Some("remote")
    );
    assert_ne!(
        explicit.reading.user_rationale.basis.source_basis,
        explicit.reading.recommendation_rationale.basis.source_basis
    );
    assert_eq!(explicit.decision.state, BriefDecisionState::ReviewRequired);
    assert!(understanding
        .selected_work_decisions
        .iter()
        .any(|d| d.decision.work_scope == DecisionWorkScope::ProjectWide));
    assert!(understanding
        .selected_work_decisions
        .iter()
        .any(|d| d.decision.work_scope == DecisionWorkScope::Unresolved));
    assert!(understanding
        .selected_work_decisions
        .iter()
        .any(|d| d.decision.state == BriefDecisionState::Superseded));
    assert!(understanding
        .unresolved_work_grouping
        .iter()
        .any(|g| g.identity == fixture.decisions["unresolved"].to_string()));
    assert!(understanding
        .architecture
        .components
        .iter()
        .all(|entity| !entity.locator.contains("unrelated")));
    assert!(projection.current_work_code.iter().any(|link| link
        .changed_path_basis
        .iter()
        .any(|basis| basis.checkpoint_id == fixture.checkpoints["change"]
            && basis.path == "python/worker.py")));
    assert!(projection.current_work_code.iter().all(|link| link
        .changed_path_basis
        .iter()
        .all(|basis| basis.checkpoint_id != fixture.checkpoints["verification_only"])));
    for locale in [FixedLocale::English, FixedLocale::Korean] {
        let documents = generate_documents(
            &projection,
            &DocumentRequest {
                requested_language: if locale == FixedLocale::English {
                    "en"
                } else {
                    "ko"
                }
                .into(),
                fixed_locale: locale,
                generated_at: TimestampMicros::from_unix_micros(1),
                generator: GeneratorIdentity {
                    generator: "fixture".into(),
                    agent: None,
                    model: None,
                },
                requested_destinations: Vec::new(),
            },
        )?;
        for document in [
            &documents.project_architecture_guide,
            &documents.decision_report,
            &documents.implementation_plan,
            &documents.handoff_resume,
        ] {
            let claim = document
                .body
                .sections
                .iter()
                .flat_map(|s| &s.claims)
                .find(|claim| claim.identity == format!("work-summary:{}", work.work_item_id))
                .ok_or("Work summary")?;
            assert_eq!(claim.class, ClaimClass::DeterministicDerived);
            assert_eq!(claim.analysis_basis, work.reading.analysis_snapshot_basis);
            assert!(!claim.analysis_basis.is_empty());
            assert!(claim.text.contains(if locale == FixedLocale::English {
                "rejected"
            } else {
                "거부됨"
            }));
            assert!(claim.text.contains(if locale == FixedLocale::English {
                "does not establish coverage of later changes"
            } else {
                "이후 변경의 검증 범위를 입증하지 않습니다"
            }));
            assert!(claim.text.contains(if locale == FixedLocale::English {
                "not run"
            } else {
                "실행하지 않음"
            }));
        }
    }
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}

#[test]
fn selection_rejects_invalid_identity_and_goal_only_work_preserves_code_gap(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    assert_eq!(
        WorkSelector::exact("bad"),
        Err(WorkSelectionError::InvalidIdentity)
    );
    assert_eq!(
        WorkSelector::exact(&"한".repeat(32)),
        Err(WorkSelectionError::InvalidIdentity)
    );
    for id in [ContextItemId::from_bytes([0; 16]), fixture.purpose] {
        let error = fixture
            .operations
            .project_projection_selected(fixture.project, WorkSelector::ExactWork(id))
            .expect_err("not a Work");
        assert_eq!(
            error.work_selection_cause(),
            Some(&WorkSelectionError::WorkNotFound {
                project_id: fixture.project,
                work_item_id: id
            })
        );
    }
    let foreign = fixture
        .operations
        .initialize_project("Foreign Project", None)?
        .project
        .id;
    let error = fixture
        .operations
        .project_projection_selected(foreign, WorkSelector::ExactWork(fixture.goals["older"]))
        .expect_err("foreign Work");
    assert!(matches!(
        error.work_selection_cause(),
        Some(WorkSelectionError::WorkNotFound { .. })
    ));
    let projection = fixture.operations.project_projection_selected(
        fixture.project,
        WorkSelector::ExactWork(fixture.goals["goal_only"]),
    )?;
    assert!(projection.checkpoint_timeline.is_empty());
    assert!(projection.current_work_topology.entities.is_empty());
    assert!(projection.current_work_topology.relations.is_empty());
    let work = projection.selected_work.as_ref().ok_or("Goal-only Work")?;
    assert_eq!(work.state, UnderstandingWorkState::Open);
    assert!(work.reading.code_gap.is_some());
    assert_eq!(
        work.reading.next_step.availability,
        ReadingAvailability::Unavailable
    );
    assert!(work.decision_ids.is_empty());
    let latest = fixture.operations.project_projection(fixture.project)?;
    assert!(latest.selected_work.is_none());
    assert!(matches!(
        latest.selection.basis,
        WorkSelectionBasis::UnassociatedCheckpoint { .. }
    ));
    let understanding = build_project_understanding(&latest, UnderstandingBound::default());
    assert!(understanding
        .unresolved_work_grouping
        .iter()
        .any(|g| g.identity == fixture.checkpoints["unassociated"].to_string()));
    let older = fixture.operations.project_projection_selected(
        fixture.project,
        WorkSelector::ExactWork(fixture.goals["older"]),
    )?;
    let other = fixture.operations.project_projection_selected(
        fixture.project,
        WorkSelector::ExactWork(fixture.goals["same_title"]),
    )?;
    assert_eq!(
        older.selected_work.as_ref().ok_or("older")?.title,
        other.selected_work.as_ref().ok_or("other")?.title
    );
    assert_ne!(
        older.selected_work.as_ref().ok_or("older")?.work_item_id,
        other.selected_work.as_ref().ok_or("other")?.work_item_id
    );
    assert_eq!(
        other.selected_work.as_ref().ok_or("other")?.checkpoint_ids,
        [fixture.checkpoints["other"]]
    );
    let repository = fixture
        .operations
        .project_projection_selected(fixture.project, WorkSelector::Repository)?;
    assert!(repository.selected_work.is_none());
    assert_eq!(repository.selection.basis, WorkSelectionBasis::Repository);
    assert!(
        !build_project_understanding(&repository, UnderstandingBound::default())
            .architecture
            .components
            .is_empty()
    );
    Ok(())
}

#[test]
fn source_and_analysis_gaps_do_not_rewrite_selected_work_and_json_retains_basis(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let select = || {
        fixture.operations.project_projection_selected(
            fixture.project,
            WorkSelector::ExactWork(fixture.goals["older"]),
        )
    };
    let initial = select()?;
    fs::write(
        fixture.repository.join("native/query.c"),
        "void changed(void) {}\n",
    )?;
    let stale = select()?;
    assert_eq!(
        initial
            .selected_work
            .as_ref()
            .ok_or("initial")?
            .reading
            .states,
        stale.selected_work.as_ref().ok_or("stale")?.reading.states
    );
    assert_eq!(
        stale
            .selected_work
            .as_ref()
            .ok_or("stale")?
            .reading
            .code_availability,
        ReadingAvailability::Degraded
    );
    assert!(stale
        .resume
        .snapshots
        .iter()
        .any(|s| s.freshness.state == volicord_repository_intelligence::FreshnessState::Stale));
    fs::remove_dir_all(&fixture.repository)?;
    let unavailable = select()?;
    assert!(unavailable
        .resume
        .snapshots
        .iter()
        .any(|s| s.freshness.state == volicord_repository_intelligence::FreshnessState::Unknown));
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    let mut raw = fixture.operations.canonical_basis(fixture.project)?;
    raw.context_items
        .retain(|item| item.role != ContextItemRole::ProjectPurpose);
    let candidates = fixture.operations.candidate_basis(fixture.project)?;
    let projection = build_project_projection(ProjectProjectionInputs {
        detail: volicord_projections::ProjectionDetail::default(),
        selection: WorkSelector::ExactWork(fixture.goals["older"]),
        canonical: &raw,
        analyses: &[],
        analysis_issues: &[],
        applicability: volicord_inquiry::ApplicabilityQuery {
            project_id: fixture.project,
            paths: Vec::new(),
            components: Vec::new(),
            work_contexts: Vec::new(),
            current_assumptions: Vec::new(),
            met_revisit_triggers: Vec::new(),
        },
        candidates: CandidateProjectionInput::Available(&candidates),
        candidate_content_access: CandidateContentAccess::PolicyWithheld,
        observed_at: raw.project.updated_at,
        bound: ProjectionBound {
            max_items_per_section: 1,
        },
    })?;
    assert!(projection.resume.project_purpose.is_empty());
    assert!(projection
        .selected_work
        .as_ref()
        .ok_or("canonical Work")?
        .reading
        .code_gap
        .as_ref()
        .is_some_and(|gap| *gap == WorkCodeGap::AnalysisUnavailable));
    assert_eq!(
        projection
            .selected_work_decisions
            .iter()
            .map(|d| d.decision.decision_id)
            .collect::<BTreeSet<_>>(),
        initial
            .selected_work_decisions
            .iter()
            .map(|d| d.decision.decision_id)
            .collect::<BTreeSet<_>>()
    );
    assert_eq!(
        projection
            .selected_work
            .as_ref()
            .ok_or("selected")?
            .checkpoint_ids
            .len(),
        3
    );
    assert_eq!(projection.checkpoint_timeline.len(), 1);
    assert!(projection
        .issues
        .iter()
        .any(|i| i.affected_scope == "checkpoint_timeline" && i.omitted_count == 2));
    let args = vec![
        "--runtime".into(),
        fixture
            .operations
            .layout()
            .root()
            .to_string_lossy()
            .into_owned(),
        "--json".into(),
        "--project".into(),
        fixture.project.to_string(),
        "status".into(),
    ];
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let result = run_cli(args, &mut output, &mut errors);
    assert_eq!(
        result,
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&errors)
    );
    let json: Value = serde_json::from_slice(&output)?;
    let work = json["work_history"]
        .as_array()
        .ok_or("JSON Work list")?
        .iter()
        .find(|work| work["work_item_id"] == fixture.goals["older"].to_string())
        .ok_or("JSON older")?;
    assert_eq!(
        work["reading"]["states"][0]["checkpoint_id"],
        fixture.checkpoints["change"].to_string()
    );
    assert_eq!(
        work["reading"]["states"][2]["verification"][0]["state"],
        "not_run"
    );
    assert!(work["reading"]["changes"][0]["original_text"]
        .as_str()
        .is_some_and(|text| text.contains("aabbccddeeff00112233445566778899")));
    Ok(())
}

#[test]
fn source_seeds_and_verification_only_latest_are_selected_before_timeline_truncation(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let mut canonical = fixture.operations.canonical_basis(fixture.project)?;
    let candidates = fixture.operations.candidate_basis(fixture.project)?;
    let mut checkpoints = canonical
        .checkpoint_history
        .iter()
        .filter(|cp| {
            cp.work_item_id == Some(fixture.goals["older"])
                && cp.id != fixture.checkpoints["later_change"]
        })
        .cloned()
        .collect::<Vec<_>>();
    checkpoints.sort_by_key(|cp| (cp.recorded_at, cp.id));
    canonical.latest_checkpoint = checkpoints.last().cloned();
    canonical.checkpoint_history = checkpoints;
    let repository_source = canonical
        .sources
        .iter()
        .find(|s| matches!(s.source.payload, SourcePayload::RepositorySnapshot { .. }))
        .ok_or("repository Source")?
        .source
        .id;
    let grounding =
        volicord_repository_intelligence::CanonicalGrounding::from_read_basis(&canonical)?;
    let (_, analysis) = volicord_repository_intelligence::analyze_repository(
        volicord_repository_intelligence::StructuralAnalysisRequest::new(
            volicord_repository_intelligence::InventoryRequest::new(
                &fixture.repository,
                &grounding,
                repository_source,
                1,
            )?,
        ),
    )?;
    let project = |canonical: &CanonicalReadBasis, selector| {
        build_project_projection(ProjectProjectionInputs {
            detail: volicord_projections::ProjectionDetail::default(),
            selection: selector,
            canonical,
            analyses: &[&analysis],
            analysis_issues: &[],
            applicability: volicord_inquiry::ApplicabilityQuery {
                project_id: fixture.project,
                paths: Vec::new(),
                components: Vec::new(),
                work_contexts: Vec::new(),
                current_assumptions: Vec::new(),
                met_revisit_triggers: Vec::new(),
            },
            candidates: CandidateProjectionInput::Available(&candidates),
            candidate_content_access: CandidateContentAccess::PolicyWithheld,
            observed_at: canonical.project.updated_at,
            bound: ProjectionBound {
                max_items_per_section: 2,
            },
        })
    };
    let selected = project(&canonical, WorkSelector::ExactWork(fixture.goals["older"]))?;
    assert_eq!(
        selected
            .selected_work
            .as_ref()
            .ok_or("Work")?
            .reading
            .states
            .last()
            .ok_or("latest state")?
            .checkpoint_id,
        fixture.checkpoints["verification_only"]
    );
    assert!(selected.current_work_code.iter().any(|link| link
        .changed_path_basis
        .iter()
        .any(|p| p.checkpoint_id == fixture.checkpoints["change"])));
    assert!(selected.current_work_code.iter().all(|link| link
        .changed_path_basis
        .iter()
        .all(|p| p.checkpoint_id != fixture.checkpoints["verification_only"])));
    // Explicit Source locator is a seed even without Goal path scope or Checkpoints.
    canonical.checkpoint_history.clear();
    canonical.latest_checkpoint = None;
    let file_source = SourceId::from_bytes([0x77; 16]);
    let mut source = canonical.sources[0].clone();
    source.source.id = file_source;
    source.source.payload = SourcePayload::File {
        locator: "python/worker.py".into(),
        snapshot: "fixture-source".into(),
    };
    canonical.sources.push(source);
    let goal = canonical
        .context_items
        .iter_mut()
        .find(|g| g.id == fixture.goals["goal_only"])
        .ok_or("Goal")?;
    goal.source_basis.push(file_source);
    let source_selected = project(
        &canonical,
        WorkSelector::ExactWork(fixture.goals["goal_only"]),
    )?;
    assert!(!source_selected.current_work_topology.entities.is_empty());
    assert!(source_selected
        .current_work_code
        .iter()
        .any(|link| link.goal_context_basis == [fixture.goals["goal_only"]]));
    assert!(!build_project_understanding(
        &source_selected,
        UnderstandingBound {
            max_items_per_section: 2
        }
    )
    .architecture
    .components
    .is_empty());
    // No Checkpoint: latest Goal uses its own recorded time/identity, never title ordering.
    let latest = WorkSelector::LatestWork.resolve(&canonical)?;
    let expected = canonical
        .context_items
        .iter()
        .filter(|g| g.role == ContextItemRole::Goal)
        .max_by_key(|g| (g.recorded_at, g.id))
        .ok_or("latest Goal")?;
    assert_eq!(latest.work_item_id, Some(expected.id));
    assert!(matches!(
        latest.basis,
        WorkSelectionBasis::LatestGoal { .. }
    ));
    // Available revision history survives even if parent inspection has a small bound.
    canonical
        .context_items
        .iter_mut()
        .find(|g| g.id == fixture.goals["goal_only"])
        .ok_or("Goal")?
        .revision = 2;
    canonical
        .revisions
        .iter_mut()
        .find(|r| {
            r.record_kind == CanonicalRecordKind::ContextItem
                && r.record_identity == fixture.goals["goal_only"].to_string()
        })
        .ok_or("revisions")?
        .revisions = vec![1, 2];
    let revised = project(
        &canonical,
        WorkSelector::ExactWork(fixture.goals["goal_only"]),
    )?;
    let goal_basis = &revised
        .selected_work
        .as_ref()
        .ok_or("revised Work")?
        .reading
        .goal
        .basis;
    assert_eq!(goal_basis.revision, 2);
    assert_eq!(goal_basis.available_revisions, [1, 2]);
    // Source availability remains independent even when a caller supplies current freshness.
    let source_id = goal_basis.source_basis[0];
    let source = canonical
        .sources
        .iter_mut()
        .find(|s| s.source.id == source_id)
        .ok_or("Goal Source")?;
    source.availability = Availability::Unavailable;
    source.freshness = SourceFreshness::Current;
    let unavailable = project(
        &canonical,
        WorkSelector::ExactWork(fixture.goals["goal_only"]),
    )?;
    let reading = &unavailable
        .selected_work
        .as_ref()
        .ok_or("unavailable Work")?
        .reading;
    assert_eq!(reading.goal.availability, ReadingAvailability::Degraded);
    assert_eq!(
        reading.goal.original_text.as_deref(),
        Some("Understand without code seeds / 코드 근거 없이 이해")
    );
    assert!(reading
        .status
        .display_english
        .contains("no Checkpoint work state"));
    assert!(reading.states.is_empty());
    Ok(())
}

#[test]
fn selected_polyglot_flow_retains_declared_endpoints_and_snapshot_basis(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let canonical = fixture.operations.canonical_basis(fixture.project)?;
    let candidates = fixture.operations.candidate_basis(fixture.project)?;
    let repository_source = canonical
        .sources
        .iter()
        .find(|s| matches!(s.source.payload, SourcePayload::RepositorySnapshot { .. }))
        .ok_or("repository Source")?
        .source
        .id;
    let grounding =
        volicord_repository_intelligence::CanonicalGrounding::from_read_basis(&canonical)?;
    let (_, mut analysis) = volicord_repository_intelligence::analyze_repository(
        volicord_repository_intelligence::StructuralAnalysisRequest::new(
            volicord_repository_intelligence::InventoryRequest::new(
                &fixture.repository,
                &grounding,
                repository_source,
                1,
            )?,
        ),
    )?;
    // Structural analysis supplies no cross-language semantic claim. The declared
    // binding below is synthetic evidence, not an inferred analyzer capability.
    assert!(analysis.semantic_results.is_empty());
    assert!(analysis
        .structural_facts
        .iter()
        .flat_map(|f| &f.relations)
        .any(|r| { matches!(r.target, RelationTarget::Unresolved { .. }) }));
    let scenario: Value = serde_json::from_str(SCENARIO)?;
    let edge = &scenario["graph_cases"]["semantic_relation"];
    let source = analysis
        .structural_facts
        .iter()
        .find(|f| {
            f.entity.area.path == edge["source_path"].as_str().unwrap_or_default()
                && f.entity.kind == CodeEntityKind::File
        })
        .ok_or("native file")?
        .entity
        .clone();
    let target = analysis
        .structural_facts
        .iter()
        .find(|f| {
            f.entity.area.path == edge["target_path"].as_str().unwrap_or_default()
                && f.entity.kind == CodeEntityKind::File
        })
        .ok_or("runtime file")?
        .entity
        .identity
        .clone();
    let adapter = source
        .source_range
        .as_ref()
        .ok_or("Source range")?
        .adapter
        .clone();
    let analyzer = AnalyzerIdentity {
        name: "viewer-reading-fixture".into(),
        version: "1".into(),
    };
    analysis.semantic_results.push(SemanticAnalysisResult {
        relation: SemanticRelation {
            identity: edge["identity"].as_str().ok_or("edge identity")?.into(),
            repository_snapshot: analysis.repository_snapshot,
            analysis_snapshot: analysis.identity,
            source_entity: source.identity,
            target: RelationTarget::ResolvedEntity(target),
            kind: SemanticRelationKind::References,
            supporting_range: source.source_range,
            diagnostics: Vec::new(),
            uncertainty: source.uncertainty,
            freshness: source.freshness,
            extensions: Vec::new(),
        },
        provenance: SemanticProvenance {
            adapter: adapter.clone(),
            analyzer: analyzer.clone(),
            build_context: Some("declared synthetic polyglot binding".into()),
            resolution_basis: edge["basis"].as_str().ok_or("edge basis")?.into(),
            analysis: AnalysisProvenance {
                class: ProvenanceClass::SemanticResult,
                repository_snapshot: analysis.repository_snapshot,
                analysis_snapshot: analysis.identity,
                adapter: Some(adapter),
                analyzer: Some(analyzer),
                source_basis: vec![source.source],
                observed_or_generated_at_unix_micros: analysis.generated_at_unix_micros,
            },
        },
    });
    let projection = build_project_projection(ProjectProjectionInputs {
        detail: volicord_projections::ProjectionDetail::default(),
        selection: WorkSelector::ExactWork(fixture.goals["older"]),
        canonical: &canonical,
        analyses: &[&analysis],
        analysis_issues: &[],
        applicability: volicord_inquiry::ApplicabilityQuery {
            project_id: fixture.project,
            paths: Vec::new(),
            components: Vec::new(),
            work_contexts: Vec::new(),
            current_assumptions: Vec::new(),
            met_revisit_triggers: Vec::new(),
        },
        candidates: CandidateProjectionInput::Available(&candidates),
        candidate_content_access: CandidateContentAccess::PolicyWithheld,
        observed_at: canonical.project.updated_at,
        bound: ProjectionBound {
            max_items_per_section: 2,
        },
    })?;
    let expected: Value = serde_json::from_str(EXPECTED)?;
    let expected = &expected["semantic_edge"];
    let relation = projection
        .current_work_topology
        .relations
        .iter()
        .find(|r| r.identity == expected["identity"].as_str().unwrap_or_default())
        .ok_or("declared flow omitted")?;
    assert_eq!(relation.kind, expected["kind"]);
    assert_eq!(relation.analysis_snapshot, analysis.identity);
    assert_eq!(relation.repository_snapshot, analysis.repository_snapshot);
    let from = projection
        .current_work_topology
        .entities
        .iter()
        .find(|e| e.identity == relation.source_entity)
        .ok_or("from")?;
    let to = projection
        .current_work_topology
        .entities
        .iter()
        .find(|e| Some(&e.identity) == relation.target_entity.as_ref())
        .ok_or("to")?;
    assert_eq!(from.locator, expected["source_locator"]);
    assert_eq!(to.locator, expected["target_locator"]);
    assert!(projection.current_work_topology.omitted_entity_count > 0);
    assert!(projection
        .current_work_topology
        .relations
        .iter()
        .all(|r| r.kind != "References" || r.identity == relation.identity));
    Ok(())
}
