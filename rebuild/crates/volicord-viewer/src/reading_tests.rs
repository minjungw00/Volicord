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

#[test]
fn generated_code_reading_discloses_basis_and_withholds_unavailable_claims(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::{CodeExplanationState, MapInterpretation, WorkSelector};
    let fixture = reading_fixture::fixture()?;
    let before = fixture.operations.canonical_basis(fixture.project)?;
    let mut projection = fixture.operations.project_projection_selected(
        fixture.project,
        WorkSelector::ExactWork(fixture.goals["older"]),
    )?;
    let entity = projection
        .current_work_topology
        .entities
        .iter()
        .find(|e| e.locator == "python/worker.py" && e.kind == CodeEntityKind::Function)
        .ok_or("function")?
        .clone();
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        for state in [
            CodeExplanationState::Current,
            CodeExplanationState::Partial,
            CodeExplanationState::Stale,
            CodeExplanationState::Unsupported,
            CodeExplanationState::Unavailable,
        ] {
            // Authored read-model input exercises the actual Viewer consumer;
            // Projection tests separately exercise interpretation basis validation.
            projection.repository_map.agent_interpretations = vec![MapInterpretation {
                identity: "authored-render-control".into(),
                text: "AUTHORED_BODY_INTERPRETATION".into(),
                source_basis: vec![entity.source_id],
                entity_basis: vec![entity.identity.clone()],
                relation_basis: Vec::new(),
                source_ranges: entity.source_range.clone().into_iter().collect(),
                analysis_snapshot: entity.analysis_snapshot,
                repository_snapshot: entity.repository_snapshot,
                freshness: entity.freshness.clone(),
                state,
                producer: "test / test-host / test-session".into(),
                generated_at_unix_micros: 123,
                known_gaps: vec!["No runtime execution observation".into()],
                uncertainty: volicord_repository_intelligence::Uncertainty::none(),
            }];
            let understanding =
                build_project_understanding(&projection, UnderstandingBound::default());
            let request = ViewerRequest {
                project_id: fixture.project,
                locale,
                view: ViewerView::Code {
                    scope: CodeScope::Work(Some(fixture.goals["older"])),
                    entity: None,
                },
                requested_language: "en".into(),
                guarded_request: None,
            };
            let mut html = String::new();
            code(&mut html, &request, &projection, &understanding, false);
            let card = html
                .split("<details data-statement-role=\"generated-interpretation\"")
                .nth(1)
                .ok_or("generated disclosure")?
                .split("</details>")
                .next()
                .ok_or("card")?;
            assert!(card.contains(&format!("data-explanation-state=\"{state:?}\"")));
            assert_eq!(
                card.contains("AUTHORED_BODY_INTERPRETATION"),
                !matches!(
                    state,
                    CodeExplanationState::Unsupported | CodeExplanationState::Unavailable
                )
            );
            for basis in [
                entity.analysis_snapshot.to_string(),
                entity.repository_snapshot.to_string(),
                entity.source_id.to_string(),
                entity.identity.clone(),
                "test-host".into(),
                "ZeroBasedUtf8Byte".into(),
            ] {
                assert!(card.contains(&basis));
            }
            assert!(card.contains("No runtime execution observation"));
            assert!(!card.contains("verified-fact"));
        }
    }
    assert_eq!(before, fixture.operations.canonical_basis(fixture.project)?);
    Ok(())
}
