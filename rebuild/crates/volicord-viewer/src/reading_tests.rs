use super::*;

#[cfg(test)]
#[path = "../../volicord-operations/tests/support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
#[cfg(test)]
#[path = "reading_title_tests.rs"]
mod reading_title_tests;

#[test]
fn publication_metadata_keeps_comparison_states_and_calendar_dates_distinct() {
    use volicord_projections::RepositoryComparisonState as C;
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        let r = ViewerRequest {
            project_id: ProjectId::from_bytes([1; 16]),
            locale,
            view: ViewerView::RepositoryExplanation,
            requested_language: locale_key(locale).into(),
            guarded_request: None,
        };
        for state in [C::VerifiedAtRead, C::Changed, C::Unavailable, C::NotChecked] {
            let mut answers = volicord_projections::explanation_answers(
                &[],
                &r.requested_language,
                locale.fixed(),
            );
            answers.applicability.generated_at_unix_micros = Some(0);
            answers.applicability.checked_at_unix_micros = Some(60_000_000);
            answers.applicability.repository_comparison = state;
            for snapshot in [false, true] {
                let reader = ReaderAnswers::new(
                    &answers,
                    r.project_id,
                    "repository",
                    &r.requested_language,
                    "article",
                );
                let mut html = String::new();
                render_applicability(&mut html, &r, &reader, snapshot);
                assert!(html.contains("1970.01.01 00:00 UTC"));
                assert!(html.contains("1970.01.01 00:01 UTC"));
                assert!(html
                    .contains("datetime=\"1970-01-01T00:01:00Z\" data-recorded-at=\"60000000\""));
                assert!(!html.contains("Unix"));
                assert!(html.contains("class=\"comparison-icon\""));
                assert!(html.contains(comparison_label(locale, snapshot, state)));
                assert!(
                    html.find("class=\"comparison-detail\"")
                        .is_some_and(|at| at > html.find("</summary>").unwrap_or(usize::MAX)),
                    "full caveat belongs inside the deliberately opened comparison control"
                );
            }
        }
    }
}

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
                work_review_binding: None,
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
                    work_summary(&mut summary, &r, w, true);
                    assert!(!summary.contains("learning-inspection-reference"));
                    assert!(!summary.contains("tagged provenance costs an allocation"));
                }
            }
        }
        Ok(())
    }
}

#[test]
fn unavailable_publications_have_distinct_stable_safe_reading_identity() {
    use super::ReaderAnswers;
    let answers = volicord_projections::explanation_answers(
        &[],
        "en",
        volicord_projections::FixedLocale::English,
    );
    let project = ProjectId::from_bytes([42; 16]);
    let mut anchors = std::collections::BTreeSet::new();
    for project in [project, ProjectId::from_bytes([43; 16])] {
        for subject in ["work:a", "work:b", "repository", "decision:a"] {
            for scope in ["name", "summary", "article"] {
                for language in ["en", "ko", "\"><script>bad</script>#"] {
                    let context = ReaderAnswers::new(&answers, project, subject, language, scope);
                    let id = super::reader_claim_anchor(&context, 0);
                    assert!(id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'));
                    assert!(anchors.insert(id.clone()), "duplicate {id}");
                    assert_eq!(
                        id,
                        super::reader_claim_anchor(
                            &ReaderAnswers::new(&answers, project, subject, language, scope),
                            0
                        )
                    );
                }
            }
        }
    }
}

#[test]
fn identical_unavailable_article_and_summary_render_without_duplicate_targets() {
    let mut answers = volicord_projections::explanation_answers(
        &[],
        "en",
        volicord_projections::FixedLocale::English,
    );
    answers.work_summary = Some(Box::new(answers.clone()));
    let request = ViewerRequest {
        project_id: ProjectId::from_bytes([42; 16]),
        locale: ViewerLocale::English,
        view: ViewerView::RepositoryExplanation,
        requested_language: "en".into(),
        guarded_request: None,
    };
    let mut html = String::new();
    super::render_answers(&mut html, &request, &answers, None, true);
    let ids: Vec<_> = html
        .split(" id=\"")
        .skip(1)
        .filter_map(|s| s.split('"').next())
        .collect();
    let unique: std::collections::BTreeSet<_> = ids.iter().collect();
    assert_eq!(
        ids.len(),
        unique.len(),
        "identical unavailable scopes collided"
    );
    for rest in html.split(" href=\"#").skip(1) {
        let target = rest.split('"').next().unwrap_or_default();
        if target != "overview" {
            assert_eq!(
                ids.iter().filter(|id| **id == target).count(),
                1,
                "{target}"
            );
        }
    }
}

#[test]
fn outline_uses_only_substantive_authored_headings_and_keeps_original_indices(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::*;
    for count in [0, 1, 12] {
        let mut answers = explanation_answers(&[], "en", FixedLocale::English);
        answers.prose.clear();
        for n in 0..count {
            let index = answers.prose.len();
            for (question, text) in [
                ("SectionHeading", format!("{n} · 긴 한국어 제목과 long English mechanism heading {} <script>hostile</script>", "구조와 반환 경계 ".repeat(8))),
                ("Narrative", "Authored layout control: no execution or semantic approval is claimed.".into()),
            ] {
                answers.prose.push(QuestionAnswer {question:question.into(),text,role:AnswerRole::GeneratedInterpretation,
                    evidence_keys:vec![],citations:vec![],recorded_action:None});
            }
            answers.sections.push(ArticleSection {
                layer: ArticleLayer::Article,
                heading_index: Some(index),
                unit_indices: vec![index + 1],
            });
        }
        let request = ViewerRequest {
            project_id: ProjectId::from_bytes([42; 16]),
            locale: ViewerLocale::English,
            view: ViewerView::RepositoryExplanation,
            requested_language: "en".into(),
            guarded_request: None,
        };
        let mut html = String::new();
        super::render_answers(&mut html, &request, &answers, None, true);
        if count == 0 {
            assert!(!html.contains("class=\"document-nav\""));
            continue;
        }
        let primary = html
            .split("class=\"toc-primary\"")
            .nth(1)
            .ok_or("primary list")?
            .split("</ol>")
            .next()
            .ok_or("list end")?;
        assert_eq!(primary.matches("<a ").count(), count);
        assert!(!primary.contains("Verification"));
        assert!(!html.contains("<script>hostile</script>"));
        for n in 0..count {
            let id = super::reader_claim_anchor(
                &super::ReaderAnswers::new(
                    &answers,
                    request.project_id,
                    "repository",
                    "en",
                    "article",
                ),
                2 * n,
            );
            let heading = format!(
                "{}-heading-{}",
                super::document_anchor(
                    &super::ReaderAnswers::new(
                        &answers,
                        request.project_id,
                        "repository",
                        "en",
                        "article"
                    ),
                    "contents"
                ),
                n + 1
            );
            assert!(primary.contains(&format!("href=\"#{heading}\"")));
            assert!(html.contains(&format!("id=\"{heading}\" tabindex=\"-1\"")));
            assert_eq!(html.matches(&format!("id=\"{id}\"")).count(), 1);
        }
        if let Some(root) = std::env::var_os("VOLICORD_OUTLINE_CONTROL_CAPTURE") {
            std::fs::create_dir_all(&root)?;
            std::fs::write(std::path::Path::new(&root).join(format!("outline-{count}.html")),format!("<!doctype html><html lang=\"en\"><head><meta name=\"viewport\" content=\"width=device-width\">{STYLE}</head><body><main class=\"shell\">{html}</main></body></html>"))?;
        }
    }
    Ok(())
}

#[test]
fn repository_analysis_failures_stay_visible_above_optional_coverage(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::*;
    // Authored read-model controls, not admitted content or host generation.
    for (state, freshness, warning) in [
        ("available", "current", false),
        ("unsupported", "current", false),
        ("failed", "current", true),
        ("available", "unknown", true),
    ] {
        let mut answers = explanation_answers(&[], "en", FixedLocale::English);
        answers.explanation_state = ExplanationState::Current;
        answers.provenance = Some(ExplanationProvenance {
            source_read_fingerprint: None,
            publication_id: None,
            project_id: "authored-project".into(),
            subject: ExplanationSubject::Repository(
                volicord_repository_intelligence::AnalysisSnapshotId::from_hex(&"44".repeat(32))?,
            ),
            question: "Explain the repository".into(),
            language: "en".into(),
            fingerprint: "authored-preparation".into(),
            realization_fingerprint: "authored-realization".into(),
            generated_at_unix_micros: 1,
            generator: ExplanationGenerator {
                host: "authored-presentation-control".into(),
                session: "test".into(),
                agent: None,
                model: None,
            },
            generator_identity_status: "self_reported".into(),
            semantic_approval_status: "not_assessed".into(),
            citations: vec![],
            evidence: vec![ExplanationEvidence {
                key: "repository_basis".into(),
                record_kind: "analysis".into(),
                identity: "authored-analysis".into(),
                revision: 1,
                field: "scope".into(),
                sources: vec![],
                content: serde_json::json!({"repository":{"included_areas":{"file_count":1}},
                    "capabilities":[{"state":state,"freshness":{"state":freshness}}]}),
            }],
            uncited_evidence_count: 0,
            source_status: vec![],
            conflicts: vec![],
        });
        answers.facts.push(QuestionAnswer {
            question: "RepositoryCoverage".into(),
            text: "Exact authored coverage detail".into(),
            role: AnswerRole::DeterministicFacts,
            evidence_keys: vec!["repository_basis".into()],
            citations: vec![],
            recorded_action: None,
        });
        let request = ViewerRequest {
            project_id: ProjectId::from_bytes([42; 16]),
            locale: ViewerLocale::English,
            view: ViewerView::RepositoryExplanation,
            requested_language: "en".into(),
            guarded_request: None,
        };
        let mut html = String::new();
        super::render_answers(&mut html, &request, &answers, None, true);
        let reading = super::document_anchor(
            &super::ReaderAnswers::new(&answers, request.project_id, "repository", "en", "article"),
            "reading",
        );
        let opening = html
            .split(&format!("id=\"{reading}\""))
            .next()
            .ok_or("opening")?;
        assert!(html.contains(&format!("data-toc-group-heading id=\"{reading}\"")));
        assert_eq!(opening.contains("data-warning=\"analysis\""), warning);
        assert!(opening.contains("Scope: the cited Sources"));
        assert!(!opening.contains("Exact authored coverage detail"));
        assert!(html.contains("Exact authored coverage detail"));
    }
    Ok(())
}

#[test]
fn failure_notices_own_actions_and_preserve_exact_report_targets(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::*;
    let f = reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?;
    let id = f.goals["older"];
    let p = f
        .operations
        .project_projection_selected(f.project, WorkSelector::ExactWork(id))?;
    let mut w = p.selected_work.clone().ok_or("Work")?;
    // Read-side adverse controls retain canonical originals; no new authority.
    for (index, c) in w.reading.verification_report.checks.iter_mut().enumerate() {
        c.reported_state = ReportedCheckState::Failed;
        c.applicability = VerificationApplicability::Unknown;
        c.retest_comparison = RetestComparison::Unresolved;
        c.intended_check =
            EvidenceDetail::Observed(format!("cargo test --case named-check-{index}"));
    }
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        let r = ViewerRequest {
            project_id: f.project,
            locale,
            view: ViewerView::Work { work: Some(id) },
            requested_language: locale_key(locale).into(),
            guarded_request: None,
        };
        for snapshot in [false, true] {
            let mut html = String::new();
            work_detail(&mut html, &r, &p, &w, snapshot);
            let warning = html
                .split("data-warning=\"failure\"")
                .nth(1)
                .ok_or("failure notice")?
                .split("</aside>")
                .next()
                .ok_or("notice end")?;
            assert!(warning.contains("class=\"notice-actions failure-check-links\""));
            assert_eq!(
                warning
                    .matches("<summary class=\"inspection-action\"")
                    .count(),
                1,
                "one closed native picker for every unresolved failure set"
            );
            assert!(warning.contains("<details class=\"notice-report-picker\"><summary"));
            assert!(warning.contains("cargo test --case named-check-0"));
            assert!(!warning.contains("Inspect failed report 1"));
            assert!(!warning.contains("실패 보고 확인 1"));
            for target in warning
                .split("href=\"#")
                .skip(1)
                .filter_map(|s| s.split('"').next())
            {
                assert_eq!(
                    html.matches(&format!("id=\"{target}\"")).count(),
                    1,
                    "exact report target"
                );
            }
            assert!(
                !html.contains("data-warning=\"history\""),
                "past adversity alone does not add another opening warning"
            );
            assert_eq!(
                html.matches("class=\"verification-status-badge\"").count(),
                1
            );
            assert!(html.contains(&format!(
                "data-unresolved-failed-reports=\"{}\"",
                w.reading.verification_report.checks.len()
            )));
            assert!(
                html.contains("Read recorded outcomes separately from explanation availability.")
                    || html.contains("기록된 결과와 설명의 읽기 상태를 구분해 확인하세요.")
            );
        }
    }
    let mut later_pass = w.clone();
    let after = later_pass
        .reading
        .verification_report
        .checks
        .iter()
        .map(|c| c.reported_at_unix_micros)
        .max()
        .ok_or("checks")?
        + 1;
    let latest = later_pass
        .reading
        .verification_report
        .checks
        .last_mut()
        .ok_or("check")?;
    latest.reported_state = ReportedCheckState::Passed;
    latest.reported_at_unix_micros = after;
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        let request = ViewerRequest {
            project_id: f.project,
            locale,
            view: ViewerView::Work { work: Some(id) },
            requested_language: "en".into(),
            guarded_request: None,
        };
        let mut html = String::new();
        work_detail(&mut html, &request, &p, &later_pass, false);
        assert!(html.contains(text(
            locale,
            "Retest comparison not established",
            "재검증 비교 미확인"
        )));
        assert!(html.contains(text(
            locale,
            "This Work also has passing reports.",
            "이 작업에는 통과 보고도 있습니다."
        )));
        assert!(
            html.contains("data-warning=\"failure\""),
            "an unrelated pass cannot erase the original failed report"
        );
    }
    let checks = &mut later_pass.reading.verification_report.checks;
    let failed = checks.first().ok_or("original failed report")?.clone();
    let passed = checks.last_mut().ok_or("passing report")?;
    passed.reported_at_unix_micros = failed.reported_at_unix_micros;
    passed.original = failed.original.clone();
    passed.original.verification_index += 1;
    passed.retest_comparison = RetestComparison::NotComparable {
        previous: failed.original,
        reason: ComparisonLimit::InvocationChanged,
    };
    for snapshot in [false, true] {
        let request = ViewerRequest {
            project_id: f.project,
            locale: ViewerLocale::English,
            view: ViewerView::Work { work: Some(id) },
            requested_language: "en".into(),
            guarded_request: None,
        };
        let mut html = String::new();
        work_detail(&mut html, &request, &p, &later_pass, snapshot);
        assert!(html.contains("This Work also has passing reports."));
        assert!(html.contains("data-warning=\"failure\" data-severity=\"caution\""));
        let comparison = html
            .split("class=\"verification-comparison-note\"")
            .nth(1)
            .ok_or("comparison note")?;
        assert!(comparison
            .split("</div>")
            .next()
            .unwrap_or("")
            .contains("Invocation fingerprint differs"));
    }
    let mut historical = w.clone();
    for check in &mut historical.reading.verification_report.checks {
        check.applicability = VerificationApplicability::HistoricalAfterRecordedChange;
    }
    let request = ViewerRequest {
        project_id: f.project,
        locale: ViewerLocale::English,
        view: ViewerView::Work { work: Some(id) },
        requested_language: "en".into(),
        guarded_request: None,
    };
    let mut historical_html = String::new();
    work_detail(&mut historical_html, &request, &p, &historical, false);
    assert!(!historical_html.contains("class=\"verification-status-badge\""));
    assert!(!historical_html.contains("data-warning=\"history\""));
    assert!(!historical_html.contains("data-warning=\"historical\""));
    assert!(
        !historical_html.contains("data-warning=\"failure\""),
        "historical-only failures stay with original records rather than the opening"
    );
    assert!(
        historical_html.contains("Historical"),
        "historical failed originals remain inspectable"
    );
    // Distinct unnamed reports at the same instant must remain distinguishable
    // without inventing names or substituting a semantic summary for the report.
    for (index, prefix) in [
        "Recorded escaping fixture failure: ",
        "Recorded documentation failure: ",
    ]
    .iter()
    .enumerate()
    {
        let check = w
            .reading
            .verification_report
            .checks
            .get_mut(index)
            .ok_or("two report controls")?;
        check.intended_check = EvidenceDetail::NotObserved;
        check.reported_at_unix_micros = 123;
        check.original_outcome = Some(format!(
            "{prefix}{} unique-complete-tail-{index}",
            "가".repeat(500)
        ));
    }
    let r = ViewerRequest {
        project_id: f.project,
        locale: ViewerLocale::English,
        view: ViewerView::Work { work: Some(id) },
        requested_language: "en".into(),
        guarded_request: None,
    };
    let mut quoted = String::new();
    work_detail(&mut quoted, &r, &p, &w, false);
    let warning = quoted
        .split("data-warning=\"failure\"")
        .nth(1)
        .ok_or("notice")?
        .split("</aside>")
        .next()
        .ok_or("notice end")?;
    assert!(warning.contains("Recorded escaping fixture failure: "));
    assert!(warning.contains("Recorded documentation failure: "));
    assert!(warning.contains("Original report quotation"));
    assert!(warning.contains("characters omitted"));
    assert!(!warning.contains("unique-complete-tail"));
    assert!(
        quoted.contains("unique-complete-tail-0") && quoted.contains("unique-complete-tail-1"),
        "the exact target retains complete originals beyond the chooser excerpt"
    );
    let long = "path/command/identifier".repeat(200);
    let mut html = String::new();
    reading_notice(
        &mut html,
        "historical",
        "history",
        &long,
        &long,
        "<a href=\"#exact\">Inspect</a>",
    );
    assert!(html.starts_with("<aside"));
    assert!(html.ends_with("</a></aside>"));
    assert!(html.contains(&long));
    Ok(())
}

#[test]
fn related_decisions_share_work_outline_and_keep_canonical_meaning(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::*;
    let f = reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?;
    // An actual current producer publishes both languages for one Decision;
    // its peers deliberately retain missing prose as an independent baseline.
    let decision = f.decisions["explicit"];
    for language in ["en", "ko"] {
        let plan = f.operations.prepare_explanation(
            f.project,
            ExplanationSubject::Decision(decision),
            language,
        )?;
        let draft = serde_json::from_value(serde_json::json!({
            "plan_fingerprint":plan.fingerprint,"language":language,
            "generator":{"host":"authored-control","session":"reader-decision","agent":null,"model":null},
            "paragraphs":[
                {"question":"heading","text":"Bounded recommendation · 제한된 권고","evidence_keys":["choice"],"sources":[]},
                {"question":"synopsis","text":"Recorded choice and its independent interpretation · 기록된 선택과 독립 해석","evidence_keys":["choice"],"sources":[]},
                {"question":"section_heading","text":"Recommendation · 권고","evidence_keys":["recommendation"],"sources":[]},
                {"question":"narrative","text":"Prefer a bounded local boundary · 제한된 로컬 경계 권고","evidence_keys":["recommendation"],"sources":[]}
            ]
        }))?;
        let realized = f
            .operations
            .compose_decision_explanation(f.project, decision, draft)?;
        f.operations.record_explanation(
            f.project,
            ExplanationSubject::Decision(decision),
            language,
            realized,
        )?;
    }
    for key in ["older", "same_title", "goal_only"] {
        let id = f.goals[key];
        let p = f
            .operations
            .project_projection_selected(f.project, WorkSelector::ExactWork(id))?;
        let w = p.selected_work.as_ref().ok_or("Work")?;
        for locale in [ViewerLocale::English, ViewerLocale::Korean] {
            let r = ViewerRequest {
                project_id: f.project,
                locale,
                view: ViewerView::Work { work: Some(id) },
                requested_language: locale_key(locale).into(),
                guarded_request: None,
            };
            for snapshot in [false, true] {
                let mut html = String::new();
                work_detail(&mut html, &r, &p, w, snapshot);
                assert_eq!(
                    html.matches("class=\"document-nav\"").count(),
                    1,
                    "one Work outline"
                );
                assert_eq!(
                    html.matches("<article class=\"doc\"").count(),
                    1,
                    "no appended Decision page"
                );
                assert_eq!(html.matches("class=\"reader-metadata-groups\"").count(), 1);
                for scope in ["work", "publication"] {
                    assert_eq!(
                        html.matches(&format!("data-metadata-group=\"{scope}\""))
                            .count(),
                        1,
                        "Work and publication metadata are separate peer groups"
                    );
                }
                assert!(html.contains("class=\"publication-status\""));
                let mut seen = std::collections::BTreeSet::new();
                for d in p
                    .selected_work_decisions
                    .iter()
                    .chain(p.decision_catalog.iter().filter(|_| snapshot))
                    .filter(|d| w.decision_ids.contains(&d.decision.decision_id))
                    .filter(|d| seen.insert(d.decision.decision_id))
                {
                    let start = html
                        .find(&format!(
                            "class=\"decision-reading\" tabindex=\"-1\" data-decision-id=\"{}\"",
                            d.decision.decision_id
                        ))
                        .ok_or("related Decision")?;
                    let body = &html[start..];
                    let primary = body
                        .split("class=\"record-box decision-originals\"")
                        .next()
                        .ok_or("primary Decision reading")?;
                    let question = primary.find("<blockquote class=\"decision-opening-question\"");
                    if let Some(context) = &d.decision.question_context {
                        let opening = &primary[question.ok_or("visible opening question")?..];
                        assert!(opening.starts_with(&format!(
                            "<blockquote class=\"decision-opening-question\" lang=\"\">{}</blockquote>",
                            escape(&context.prompt)
                        )));
                        assert!(question < primary.find("class=\"decision-state-note\""));
                    } else {
                        assert!(question.is_none());
                    }
                    if d.decision.decision_id == decision {
                        let synopsis = "Recorded choice and its independent interpretation";
                        assert_eq!(primary.matches(synopsis).count(), 1);
                        assert!(
                            question
                                .ok_or("exact originating question for the explicit Decision")?
                                < primary.find(synopsis).ok_or("admitted synopsis")?
                        );
                    }
                    assert_eq!(primary.contains("class=\"scope-summary\""), !matches!(d.decision.work_scope, DecisionWorkScope::WorkItem(_)), "only broader or unresolved applicability is repeated in the primary reading");
                    assert!(
                        body.contains(&escape(&decision_choice_attribution(&d.decision, locale)))
                    );
                    for reason in [
                        &d.reading.user_rationale.original_text,
                        &d.reading.recommendation_rationale.original_text,
                    ]
                    .into_iter()
                    .flatten()
                    {
                        assert!(body.contains(&escape(reason)), "separate retained reasons");
                    }
                    assert!(body.contains("recorded-choice"));
                    assert!(
                        body.find("class=\"record-box decision-originals\"")
                            .ok_or("originals disclosure")?
                            < body
                                .find("class=\"reading-section recorded-choice\"")
                                .ok_or("canonical original choice")?,
                        "complete choice reports belong behind the originals disclosure"
                    );
                }
                let ids = html
                    .split(" id=\"")
                    .skip(1)
                    .filter_map(|s| s.split('"').next())
                    .collect::<Vec<_>>();
                assert_eq!(
                    ids.iter()
                        .copied()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len(),
                    ids.len(),
                    "unique scoped anchors"
                );
                for target in html
                    .split("href=\"#")
                    .skip(1)
                    .filter_map(|s| s.split('"').next())
                {
                    assert!(
                        target == "works" || ids.contains(&target),
                        "missing native target {target}"
                    );
                }
                assert!(html.matches("data-toc-entry").count() >= seen.len());
                for group in ["checks", "records"] {
                    assert_eq!(
                        html.matches(&format!("data-reading-group=\"{group}\""))
                            .count(),
                        1
                    );
                    assert_eq!(
                        html.matches(&format!("data-toc-group=\"{group}\"")).count(),
                        1
                    );
                }
                assert_eq!(
                    html.matches("data-reading-group=\"decisions\"").count(),
                    usize::from(!seen.is_empty())
                );
                assert_eq!(
                    html.matches("data-toc-group=\"decisions\"").count(),
                    usize::from(!seen.is_empty())
                );
                assert!(html.contains("class=\"reading-group-heading\" data-toc-group-heading"));
                assert_eq!(
                    html.matches("<details class=\"record-box related-decision\"><summary")
                        .count(),
                    seen.len(),
                    "each Decision is independently closed using the shared disclosure control"
                );
                assert_eq!(html.matches("data-toc-target=").count(), seen.len());
                for summary in html
                    .split("<details class=\"record-box related-decision\"><summary")
                    .skip(1)
                    .filter_map(|body| body.split("</summary>").next())
                {
                    assert!(!summary.split('>').next().ok_or("summary")?.contains("data-reader-publication"),
                        "publication withdrawal cannot replace the canonical disclosure control/state");
                    assert!(summary.contains("class=\"record-strip\""));
                }
                assert!(
                    !html.contains("data-toc-label"),
                    "no hidden prose copy outside its publication scope"
                );
                for target in html
                    .split("data-toc-target=\"")
                    .skip(1)
                    .filter_map(|s| s.split('"').next())
                {
                    assert!(
                        html.contains(&format!(
                            "class=\"related-decision-body\" id=\"{target}\" tabindex=\"-1\""
                        )),
                        "outline enters the folded Decision body"
                    );
                    assert!(
                        html.contains(&format!("href=\"#{target}\"")),
                        "one Work outline retains each Decision"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn timeline_keeps_complete_originals_folded_and_adverse_routes_visible(
) -> Result<(), Box<dyn std::error::Error>> {
    use volicord_projections::*;
    let f = reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?;
    let work = f.goals["older"];
    let p = f
        .operations
        .project_projection_selected(f.project, WorkSelector::ExactWork(work))?;
    let w = p.selected_work.as_ref().ok_or("Work")?;
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        let r = ViewerRequest {
            project_id: f.project,
            locale,
            view: ViewerView::Work { work: Some(work) },
            requested_language: locale_key(locale).into(),
            guarded_request: None,
        };
        let answers = work_answers(w, &r.requested_language, locale.fixed());
        let reader = ReaderAnswers::new(
            &answers,
            f.project,
            &format!("work:{work}"),
            &r.requested_language,
            "article",
        );
        let mut html = String::new();
        work_timeline::render(&mut html, &r, &reader, w);
        assert_eq!(
            html.matches("data-checkpoint-id=").count(),
            w.reading.states.len()
        );
        for state in &w.reading.states {
            let event = format!(
                "{}-{}-r{}",
                document_anchor(&reader, "event"),
                state.checkpoint_id,
                state.checkpoint_revision
            );
            assert!(html.contains(&format!("id=\"{event}\" tabindex=\"-1\"")));
            let body = html
                .split(&format!("id=\"{event}\""))
                .nth(1)
                .ok_or("event")?
                .split("</li>")
                .next()
                .ok_or("event end")?;
            let digest = body.split("<details").next().ok_or("digest")?;
            for fact in &state.verification {
                if let Some(outcome) = fact.outcome.as_deref().filter(|s| !s.is_empty()) {
                    assert!(body.contains(&escape(outcome)), "original report retained");
                    assert!(
                        !digest.contains(&escape(outcome)),
                        "outcome is not a semantic digest"
                    );
                }
            }
            if state
                .verification
                .iter()
                .any(|f| f.state == volicord_context::VerificationState::Failed)
            {
                assert!(
                    digest.contains("timeline-failure"),
                    "failed report route outside folded originals"
                );
            }
            for change in w
                .reading
                .changes
                .iter()
                .filter(|c| {
                    c.basis.record == ReadingRecord::Checkpoint(state.checkpoint_id)
                        && c.basis.revision == state.checkpoint_revision
                })
                .filter_map(|c| c.original_text.as_deref())
                .filter(|s| !s.trim().is_empty())
            {
                assert!(
                    body.contains(&escape(change)),
                    "complete original progress report"
                );
                assert!(
                    !digest.contains(&escape(change)),
                    "no raw/truncated prose as digest"
                );
            }
        }
        assert!(!html.contains("class=\"timeline-original\" open"));
    }
    Ok(())
}
