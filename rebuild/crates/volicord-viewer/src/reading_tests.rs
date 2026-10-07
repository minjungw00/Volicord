use super::*;
#[cfg(test)]
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;

#[cfg(test)]
mod learning_tests {
    use super::*;
    use volicord_inquiry::*;
    use volicord_projections::*;

    // Authored retained input tests the actual Viewer consumer and privacy reader.
    // Operations work_authority separately exercises the real Learning producer.
    #[test]
    fn work_learning_inspection_distinguishes_learning_decisions_and_protected_content(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let f = reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?;
        for (learning_present, decision_present) in [(true, false), (false, true), (true, true)] {
            let work_id = f.goals[if decision_present { "older" } else { "relay" }];
            let mut p = f
                .operations
                .project_projection_selected(f.project, WorkSelector::ExactWork(work_id))?;
            let canonical = f.operations.canonical_basis(f.project)?;
            let source = canonical
                .context_items
                .iter()
                .find(|c| c.id == work_id)
                .ok_or("Goal")?
                .source_basis[0];
            let candidate_id = CandidateId::from_bytes([0xac; 16]);
            let selection = LearningAlternativeSelection {
                choice_id: "representation".into(),
                alternative_id: "tagged".into(),
            };
            let alternatives = ["tagged", "side-table"]
                .into_iter()
                .map(|id| EngineeringAlternative {
                    alternative_id: id.into(),
                    summary: format!("Recorded {id} alternative"),
                    technical_consequences: vec![format!("Expected {id} allocation consequence")],
                    material_decomposition: MaterialDecomposition::MateriallyAtomic {
                        rationale: "Authored renderer control for one internal representation"
                            .into(),
                        residual_fork_closure: ResidualForkClosure {
                            interaction_comparisons: Vec::new(),
                            fixed_outcome: "Externally fixed result".into(),
                            credible_implementations: vec!["inline".into(), "helper".into()],
                            remaining_material_outcomes: Vec::new(),
                            source_basis: vec![source],
                        },
                    },
                })
                .collect();
            let learning = LearningDeliberation { goal_context_id:work_id,
                baseline_analysis_snapshot_id:volicord_repository_intelligence::AnalysisSnapshotId::from_hex(&"03".repeat(32))?,
                engineering_choice_discovery_candidate_id:CandidateId::from_bytes([4;16]),
                materiality_review_candidate_id:CandidateId::from_bytes([5;16]), dimension_id:"representation".into(),
                discovered_choice_ids:vec!["representation".into()], affected_scope:vec!["runtime/query_boundary.ts".into()],
                problem:"Compare retained <tagged> provenance with a side table".into(),
                established_facts:vec!["Externally fixed behavior is unchanged".into()],
                choices:vec![EngineeringChoice { choice_id:"representation".into(), summary:"Internal representation choice".into(),
                    affected_scope:vec!["runtime/query_boundary.ts".into()], alternatives,
                    technical_consequences:vec!["Representation allocation trade-off".into()], source_basis:vec![source],
                    effect_categories:vec![EngineeringEffectCategory::ImplementationInternal],
                    relationship:EngineeringChoiceRelationship::Independent, evidence_state:EngineeringChoiceEvidenceState::Sufficient }],
                rounds:vec![LearningDeliberationRound { initial_response_source_id:source,
                    response:LearningInitialResponse::Select { selections:vec![selection.clone()] },
                    user_rationale:Some("I can follow the invariant beside the value".into()),
                    agent_feedback:Some("Both alternatives preserve behavior; tagged provenance costs an allocation".into()),
                    agent_recommendation:Some(LearningRecommendation { selections:vec![selection.clone()],
                        rationale:"Explicit provenance helps inspect the invariant".into() }),
                    reconsideration_source_id:None, reconsideration_rationale:None }],
                state:LearningDeliberationState::Completed { round:1, selected_alternatives:vec![selection] },
            };
            let candidate = CandidateRecord {
                id: candidate_id,
                project_id: f.project,
                revision: 4,
                kind: CandidateKind::LearningDeliberation,
                collection_mode: CandidateCollectionMode::ExplicitUserDirected,
                origin: CandidateOrigin {
                    actor: volicord_context::Principal {
                        kind: volicord_context::PrincipalKind::Agent,
                        identity: "authored-control".into(),
                    },
                    subsystem: "inquiry".into(),
                    session: Some("renderer".into()),
                    provenance_summary: "Authored retained Learning input".into(),
                },
                collection_scope: CandidateCollectionScope {
                    project_id: f.project,
                    session: Some("renderer".into()),
                    source_operation: Some("learning".into()),
                    candidate_kind: CandidateKind::LearningDeliberation,
                },
                observation_basis: CandidateObservationBasis {
                    source_basis: vec![source],
                    ..Default::default()
                },
                created_at: canonical.project.updated_at,
                observed_at: canonical.project.updated_at,
                retention: CandidateRetention {
                    retained_until: None,
                    basis: "Authored consumer control".into(),
                },
                disposition: CandidateDisposition::PendingOrRetained,
                cleanup: None,
                promotion_target: None,
                opt_out_state_at_collection: Vec::new(),
                content: Some(CandidateContent {
                    bounded_summary: "Learning about representation".into(),
                    question: None,
                    engineering_choice_discovery: None,
                    materiality_review: None,
                    learning_deliberation: Some(learning),
                }),
            };
            let basis = CandidateReadBasis {
                project_id: f.project,
                candidates: vec![candidate],
                collection_policies: Vec::new(),
                withheld_for_canonical_forgetting: Vec::new(),
            };
            for mode in 0..5 {
                let mut basis = basis.clone();
                match mode {
                    1 => {
                        basis.candidates[0]
                            .content
                            .as_mut()
                            .ok_or("content")?
                            .learning_deliberation
                            .as_mut()
                            .ok_or("learning")?
                            .goal_context_id = f.goals["same_title"]
                    }
                    2 => basis.withheld_for_canonical_forgetting.push(candidate_id),
                    3 => {
                        basis.candidates[0].retention.retained_until =
                            Some(canonical.project.updated_at)
                    }
                    _ => {}
                }
                p.candidate_inspection = if learning_present {
                    vec![inspect_candidate(
                        &basis,
                        candidate_id,
                        if mode == 4 {
                            CandidateContentAccess::PolicyWithheld
                        } else {
                            CandidateContentAccess::AllowBoundedSummary
                        },
                        canonical.project.updated_at,
                    )]
                } else {
                    Vec::new()
                };
                for locale in [ViewerLocale::English, ViewerLocale::Korean] {
                    let r = ViewerRequest {
                        project_id: f.project,
                        locale,
                        view: ViewerView::Work {
                            work: Some(work_id),
                        },
                        requested_language: if locale == ViewerLocale::English {
                            "en"
                        } else {
                            "ko"
                        }
                        .into(),
                        guarded_request: None,
                    };
                    let w = p.selected_work.as_ref().ok_or("Work")?;
                    let mut html = String::new();
                    work_detail(&mut html, &r, &p, w, false);
                    assert_eq!(
                        html.contains("data-candidate-id="),
                        learning_present && mode == 0
                    );
                    assert_eq!(
                        html.contains("tagged provenance costs an allocation"),
                        learning_present && mode == 0
                    );
                    assert_eq!(html.contains("data-decision-id="), decision_present);
                    if learning_present && mode == 0 {
                        assert!(html.contains("&lt;tagged&gt;"));
                        assert!(html.contains("side-table alternative"));
                        assert!(html.contains("I can follow the invariant"));
                        assert!(html.contains("Explicit provenance helps inspect"));
                        assert!(html.contains("session-candidate"));
                        assert!(html.contains(&format!("id=\"learning-{candidate_id}\"")));
                    }
                    let mut summary = String::new();
                    work_summary(&mut summary, &r, &p, w, true);
                    assert_eq!(
                        summary.contains("learning-inspection-reference"),
                        learning_present && mode == 0
                    );
                    assert!(!summary.contains("tagged provenance costs an allocation"));
                }
            }
        }
        Ok(())
    }
}
