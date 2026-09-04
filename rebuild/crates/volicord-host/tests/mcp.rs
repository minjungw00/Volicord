use rusqlite::Connection;
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs,
    process::Command,
    sync::{Arc, Barrier},
    thread,
};
use tempfile::tempdir;
use volicord_context::{
    ApplicabilityScope, OperationId, Principal, PrincipalKind, TimestampMicros,
};
use volicord_host::{run_stdio, HostAdapter, HOST_TOOL_NAMES};
use volicord_inquiry::{
    BatchResponseItem, CandidateCollectionMode, CandidateCollectionScope, CandidateContent,
    CandidateDraft, CandidateKind, CandidateObservationBasis, CandidateOrigin, CandidateRetention,
    CandidateStore, CurrentHostResponse, DisplayedQuestion, ResponseMapping, SubmissionOutcome,
};
use volicord_operations::{
    EngineeringAlternative, EngineeringChoice, EngineeringChoiceDiscoveryDraft,
    EngineeringChoiceEvidenceState, EngineeringChoiceRelationship, EngineeringEffectCategory,
    LocalOperations, MaterialBoundaryConclusion, MaterialBoundaryReview,
    MaterialOutcomeOwnershipAssessment, MaterialityDimension, MaterialityDisposition,
    MaterialityReviewDraft, RuntimeLayout, WorkAuthorityBasis, WorkAuthorityBasisKind,
};
use volicord_privacy::{
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedKind, ManagedDerivedState,
    PrivacyStore, ProviderIntentProvenance, ProviderOptInPolicy, ProviderRetentionPolicy,
    SecretFilteringPolicy, SourceExclusionPolicy,
};
use volicord_projections::{
    NARRATIVE_PLAN_SOURCE_TEXT_BYTE_LIMIT, RENDERED_DOCUMENT_FIELD_BYTE_LIMIT,
};

#[test]
fn canonical_forgetting_mcp_cleans_linked_local_content() {
    let (_temporary, mut adapter, project) = setup();
    let project_id = parse_project(&project);
    let target = adapter
        .operations()
        .record_user_source(
            project_id,
            "codex".into(),
            "mcp-forgetting".into(),
            "linked target".into(),
        )
        .expect("target Source");
    let unrelated = adapter
        .operations()
        .record_user_source(
            project_id,
            "codex".into(),
            "mcp-forgetting".into(),
            "unrelated target".into(),
        )
        .expect("unrelated Source");
    let target_id = parse_source_identity(&target.identity);
    let unrelated_id = parse_source_identity(&unrelated.identity);
    let related_candidate = store_forgetting_candidate(&adapter, project_id, target_id, "related");
    let unrelated_candidate =
        store_forgetting_candidate(&adapter, project_id, unrelated_id, "unrelated");
    let mut privacy =
        PrivacyStore::open(adapter.operations().layout().privacy_store()).expect("privacy store");
    let related_derived = privacy
        .record_managed_derived(forgetting_derived(project_id, target_id, "related"))
        .expect("related Derived")
        .id;
    let unrelated_derived = privacy
        .record_managed_derived(forgetting_derived(project_id, unrelated_id, "unrelated"))
        .expect("unrelated Derived")
        .id;
    drop(privacy);

    let response = call(
        &mut adapter,
        "canonical_mutate",
        json!({
            "action":"forget",
            "project_id":project,
            "record_kind":"source",
            "record_id":target.identity,
            "user_turn":"Forget this exact linked Source"
        }),
    );
    assert_eq!(response["result"]["isError"], false, "{response}");
    let result = structured(&response);
    assert_eq!(result["state"], "completed");
    assert_eq!(result["canonical_committed"], true);
    assert_eq!(result["candidate_cleanup_completed"], true);
    assert_eq!(result["managed_derived_cleanup_completed"], true);
    assert_eq!(result["residue_verified"], true);

    let candidates = CandidateStore::open(adapter.operations().layout().candidate_store())
        .expect("Candidate store");
    assert!(candidates
        .get(project_id, related_candidate)
        .expect("related Candidate")
        .content
        .is_none());
    assert!(candidates
        .get(project_id, unrelated_candidate)
        .expect("unrelated Candidate")
        .content
        .is_some());
    let privacy =
        PrivacyStore::open(adapter.operations().layout().privacy_store()).expect("privacy store");
    assert_eq!(
        privacy
            .get_derived(project_id, related_derived)
            .expect("related Derived")
            .state,
        ManagedDerivedState::Deleted
    );
    assert_eq!(
        privacy
            .get_derived(project_id, unrelated_derived)
            .expect("unrelated Derived")
            .state,
        ManagedDerivedState::Current
    );
}

fn store_forgetting_candidate(
    adapter: &HostAdapter,
    project_id: volicord_context::ProjectId,
    source_id: volicord_context::SourceId,
    summary: &str,
) -> volicord_inquiry::CandidateId {
    let outcome = adapter
        .operations()
        .submit_candidate(CandidateDraft {
            project_id,
            kind: CandidateKind::Observation,
            collection_mode: CandidateCollectionMode::Automatic,
            origin: CandidateOrigin {
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "mcp-test-agent".into(),
                },
                subsystem: "mcp-forgetting-test".into(),
                session: Some("mcp-forgetting".into()),
                provenance_summary: "MCP forgetting fixture".into(),
            },
            collection_scope: CandidateCollectionScope {
                project_id,
                session: Some("mcp-forgetting".into()),
                source_operation: Some("fixture".into()),
                candidate_kind: CandidateKind::Observation,
            },
            observation_basis: CandidateObservationBasis {
                source_basis: vec![source_id],
                ..CandidateObservationBasis::default()
            },
            observed_at: TimestampMicros::from_unix_micros(1),
            retention: CandidateRetention {
                retained_until: None,
                basis: "retain for MCP forgetting test".into(),
            },
            content: CandidateContent {
                bounded_summary: summary.into(),
                question: None,
                engineering_choice_discovery: None,
                materiality_review: None,
                learning_deliberation: None,
            },
        })
        .expect("submit Candidate");
    match outcome {
        SubmissionOutcome::Stored(candidate) => candidate.id,
        SubmissionOutcome::CollectionDisabled { .. } => panic!("Candidate collection disabled"),
    }
}

fn forgetting_derived(
    project_id: volicord_context::ProjectId,
    source_id: volicord_context::SourceId,
    content: &str,
) -> ManagedDerivedDraft {
    ManagedDerivedDraft {
        project_id,
        kind: ManagedDerivedKind::CachedSummary,
        provider: None,
        model: None,
        purpose: "MCP forgetting fixture".into(),
        analysis_snapshot: None,
        included_sources: Vec::new(),
        canonical_links: vec![ManagedCanonicalLink::Source(source_id)],
        content: content.into(),
        uncertainty: None,
        retained_until: None,
        retention_basis: "rebuildable fixture".into(),
    }
}

fn setup() -> (tempfile::TempDir, HostAdapter, String) {
    let temporary = tempdir().expect("temporary directory");
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository).expect("repository");
    let operations = LocalOperations::new(
        RuntimeLayout::new(temporary.path().join("runtime")).expect("runtime layout"),
    );
    let project = operations
        .initialize_project("Host Project", Some(&repository))
        .expect("initialize Project")
        .project
        .id
        .to_string();
    (temporary, HostAdapter::new(operations), project)
}

struct FixtureEngineeringChoice<'a> {
    id: &'a str,
    affected_scope: &'a str,
    effect_category: EngineeringEffectCategory,
}

fn record_fixture_discovery(
    adapter: &HostAdapter,
    project: &str,
    goal_context_id: &str,
    baseline_analysis_snapshot_id: &str,
    source_id: &str,
    choice: FixtureEngineeringChoice<'_>,
) -> String {
    let source_id = parse_source_identity(source_id);
    let choices = vec![EngineeringChoice {
        choice_id: choice.id.into(),
        summary: choice.id.into(),
        affected_scope: vec![choice.affected_scope.into()],
        alternatives: vec![
            EngineeringAlternative {
                alternative_id: "first".into(),
                summary: "first credible approach".into(),
                technical_consequences: vec!["first bounded consequence".into()],
            },
            EngineeringAlternative {
                alternative_id: "second".into(),
                summary: "second credible approach".into(),
                technical_consequences: vec!["second bounded consequence".into()],
            },
        ],
        technical_consequences: vec!["the selected approach changes the work".into()],
        source_basis: vec![source_id],
        effect_categories: vec![choice.effect_category],
        relationship: EngineeringChoiceRelationship::Independent,
        evidence_state: EngineeringChoiceEvidenceState::Sufficient,
    }];
    let material_boundary_review = complete_material_boundary_review(&choices, source_id);
    let discovery = adapter
        .operations()
        .record_engineering_choice_discovery(EngineeringChoiceDiscoveryDraft {
            project_id: parse_project(project),
            goal_context_id: parse_context_identity(goal_context_id),
            baseline_analysis_snapshot_id:
                volicord_repository_intelligence::AnalysisSnapshotId::from_hex(
                    baseline_analysis_snapshot_id,
                )
                .expect("baseline Analysis Snapshot identity"),
            session: "mcp-mechanical-discovery-fixture".into(),
            source_operation: "engineering-choice-discovery-fixture".into(),
            summary: format!("discover {}", choice.id),
            choices,
            material_boundary_review,
        })
        .expect("mechanical host discovery fixture");
    discovery.discovery_candidate_id.to_string()
}

fn complete_material_boundary_review(
    choices: &[EngineeringChoice],
    source_id: volicord_context::SourceId,
) -> Vec<MaterialBoundaryReview> {
    EngineeringEffectCategory::ALL
        .into_iter()
        .map(|effect_category| {
            let choice_ids = choices
                .iter()
                .filter(|choice| choice.effect_categories.contains(&effect_category))
                .map(|choice| choice.choice_id.clone())
                .collect::<Vec<_>>();
            MaterialBoundaryReview {
                effect_category,
                conclusion: if choice_ids.is_empty() {
                    MaterialBoundaryConclusion::NoIndependentFork {
                        rationale: "the fixture has no independent outcome in this category".into(),
                    }
                } else {
                    MaterialBoundaryConclusion::RepresentedByChoices { choice_ids }
                },
                source_basis: vec![source_id],
            }
        })
        .collect()
}

fn complete_material_boundary_review_json(
    source_id: &str,
    represented: &[(&str, &[&str])],
) -> Value {
    let categories = [
        "public_api_shape_or_semantics",
        "compatibility",
        "failure_or_error_semantics",
        "persistence_or_lifetime",
        "privacy_or_disclosure",
        "security",
        "user_visible_behavior_or_default",
        "performance_or_resource_behavior",
        "concurrency_or_operability",
        "maintenance_or_support",
        "implementation_internal",
    ];
    Value::Array(
        categories
            .into_iter()
            .map(|effect_category| {
                let conclusion = represented
                    .iter()
                    .find(|(category, _)| *category == effect_category)
                    .map_or_else(
                        || {
                            json!({
                                "state":"no_independent_fork",
                                "rationale":"The bounded fixture has no separate material outcome in this category."
                            })
                        },
                        |(_, choice_ids)| {
                            json!({"state":"represented_by_choices","choice_ids":choice_ids})
                        },
                    );
                json!({
                    "effect_category":effect_category,
                    "conclusion":conclusion,
                    "source_ids":[source_id],
                })
            })
            .collect(),
    )
}

fn fill_draft_variant(
    variants: &[Value],
    discriminator_field: &str,
    discriminator_value: &str,
    semantic_fields: Value,
) -> Value {
    let variant = variants
        .iter()
        .find(|variant| {
            variant["bounded_allowed_values"][discriminator_field] == discriminator_value
        })
        .expect("draft variant");
    let mut result = variant["bounded_allowed_values"]
        .as_object()
        .expect("bounded allowed values")
        .clone();
    result.extend(
        semantic_fields
            .as_object()
            .expect("semantic fields")
            .clone(),
    );
    for required in variant["required_fields"]
        .as_array()
        .expect("required fields")
        .iter()
        .filter_map(Value::as_str)
    {
        assert!(
            result.contains_key(required),
            "missing {required}: {variant}"
        );
    }
    for forbidden in variant["forbidden_fields"]
        .as_array()
        .expect("forbidden fields")
        .iter()
        .filter_map(Value::as_str)
    {
        assert!(!result.contains_key(forbidden), "forbidden {forbidden}");
    }
    Value::Object(result)
}

fn draft_learning_value(draft: &Value, state: &str, semantic_fields: Value) -> Value {
    fill_draft_variant(
        draft["learning_value_input_alternatives"]
            .as_array()
            .expect("learning-value alternatives"),
        "state",
        state,
        semantic_fields,
    )
}

fn draft_learning_participation(draft: &Value, state: &str, semantic_fields: Value) -> Value {
    fill_draft_variant(
        draft["learning_participation"]["input_alternatives"]
            .as_array()
            .expect("learning-participation alternatives"),
        "state",
        state,
        semantic_fields,
    )
}

fn draft_judgment(
    draft: &Value,
    choice_id: &str,
    variant_id: &str,
    semantic_fields: Value,
) -> Value {
    let template = draft["judgment_templates"]
        .as_array()
        .expect("judgment templates")
        .iter()
        .find(|template| template["discovery_owned"]["choice_id"] == choice_id)
        .expect("choice template");
    assert!(
        template["caller_owned_judgment"]["legal_judgment_variant_ids"]
            .as_array()
            .expect("legal judgment variants")
            .iter()
            .any(|candidate| candidate == variant_id)
    );
    let contract = draft["judgment_contracts"]
        .as_array()
        .expect("judgment contracts")
        .iter()
        .find(|contract| contract["variant_id"] == variant_id)
        .expect("judgment contract");
    let mut result = template["caller_owned_judgment"]["prefilled_fields"]
        .as_object()
        .expect("prefilled judgment fields")
        .clone();
    result.extend(
        contract["bounded_allowed_values"]
            .as_object()
            .expect("bounded judgment values")
            .clone(),
    );
    result.extend(
        semantic_fields
            .as_object()
            .expect("judgment semantic fields")
            .clone(),
    );
    result.entry("authority_counterfactual").or_insert_with(|| {
        json!("The exact outcome, Goal alternatives, and selecting authority or authority gap were evaluated for this fixture.")
    });
    result
        .entry("materially_varying_outcomes")
        .or_insert_with(|| json!(["the bounded fixture outcome selected by the alternatives"]));
    let contains_user_owned_outcome = !matches!(
        variant_id,
        "repository_or_environment_fact"
            | "agent_owned_implementation_choice"
            | "exploratory_uncertainty_research_required"
            | "exploratory_uncertainty_prototype_required"
            | "exploratory_uncertainty_deferred"
            | "exploratory_uncertainty_resolved"
    );
    result
        .entry("contains_user_owned_outcome")
        .or_insert_with(|| json!(contains_user_owned_outcome));
    result.entry("user_owned_outcomes").or_insert_with(|| {
        if contains_user_owned_outcome {
            json!(["the bounded user-owned fixture policy"])
        } else {
            json!([])
        }
    });
    result
        .entry("ownership_rationale")
        .or_insert_with(|| json!("The fixture explicitly assesses who owns the varying outcome."));
    if !contains_user_owned_outcome {
        result
            .entry("bounded_implementation_discretion_rationale")
            .or_insert_with(|| {
                json!("Every remaining alternative stays inside the settled fixture behavior.")
            });
    }
    result.entry("ownership_source_ids").or_insert_with(|| {
        json!(contract["server_derived_identities"]["current_goal_user_turn_source_ids"])
    });
    for required in contract["required_fields"]
        .as_array()
        .expect("required judgment fields")
        .iter()
        .filter_map(Value::as_str)
    {
        assert!(
            result.contains_key(required),
            "missing {required}: {contract}"
        );
    }
    for forbidden in contract["forbidden_fields"]
        .as_array()
        .expect("forbidden judgment fields")
        .iter()
        .filter_map(Value::as_str)
    {
        assert!(!result.contains_key(forbidden), "forbidden {forbidden}");
    }
    Value::Object(result)
}

fn draft_request(
    draft: &Value,
    rationale: &str,
    learning_participation: Value,
    judgments: Vec<Value>,
) -> Value {
    let mut request = draft["record_request"]["prefilled_fields"]
        .as_object()
        .expect("prefilled request fields")
        .clone();
    request.insert("rationale".into(), json!(rationale));
    request.insert("learning_participation".into(), learning_participation);
    request.insert("judgments".into(), Value::Array(judgments));
    Value::Object(request)
}

fn record_host_question_decision(
    adapter: &mut HostAdapter,
    project: &str,
    source_id: &str,
    materiality_basis: Option<(&str, &str)>,
    delegation: bool,
) -> String {
    let submit = match materiality_basis {
        Some((review_candidate_id, dimension_id)) => json!({
            "action":"submit_question_from_materiality",
            "project_id":project,
            "review_candidate_id":review_candidate_id,
            "dimension_id":dimension_id,
            "research_state":"ready_to_ask",
            "research_state_basis":"Repository facts cannot select the remaining outcome",
            "retention_basis":"Retain through the explicit response",
            "bounded_summary":"Bounded materiality authority fixture",
            "prompt":"Who should choose the bounded implementation outcome?",
            "why_now":"The current review requires exact authority",
            "alternatives":[
                {"key":"first","label":"First approach","consequence":"Use the first bounded approach"},
                {"key":"second","label":"Second approach","consequence":"Use the second bounded approach"}
            ],
            "recommendation_key":"first",
            "recommendation_rationale":"The first approach has the smaller bounded cost",
            "duplicate_basis":"No current Question covers this dimension",
            "presentation_order":1
        }),
        None => json!({
            "action":"submit_question",
            "project_id":project,
            "source_ids":[source_id],
            "source_operation":"settled Decision fixture",
            "affected_scope":["src/lib.rs"],
            "bounded_summary":"Pre-existing bounded authority fixture",
            "prompt":"Which existing bounded approach should apply?",
            "why_now":"The choice establishes reusable current authority",
            "alternatives":[
                {"key":"first","label":"First approach","consequence":"Use the first bounded approach"},
                {"key":"second","label":"Second approach","consequence":"Use the second bounded approach"}
            ],
            "recommendation_key":"first",
            "recommendation_rationale":"The first approach has the smaller bounded cost",
            "research_state":"ready_to_ask",
            "research_state_basis":"Repository research is complete",
            "duplicate_basis":"No current Question covers this scope",
            "materiality_rationale":"The bounded outcome is materially distinct",
            "retention_basis":"Retain through the explicit response",
            "presentation_order":1
        }),
    };
    let submitted = structured(&call(adapter, "candidate_manage", submit)).clone();
    let candidate_id = submitted["candidate_id"]
        .as_str()
        .expect("Question Candidate identity");
    let promoted = structured(&call(
        adapter,
        "candidate_manage",
        json!({"action":"promote_question","project_id":project,"candidate_id":candidate_id}),
    ))
    .clone();
    let question_id = promoted["question_id"].as_str().expect("Question identity");
    let frontier = structured(&call(
        adapter,
        "inquiry_frontier",
        json!({"project_id":project}),
    ))
    .clone();
    let displayed = frontier["questions"]
        .as_array()
        .expect("frontier Questions")
        .iter()
        .find(|question| question["identity"] == question_id)
        .expect("displayed Question");
    let revision = displayed["revision"].as_u64().expect("Question revision");
    if delegation {
        let turn = "I delegate this displayed bounded choice to the implementation owner";
        let source = adapter
            .operations()
            .record_user_source(
                parse_project(project),
                "codex".into(),
                "host-materiality-contract".into(),
                turn.into(),
            )
            .expect("delegation response Source");
        let result = adapter
            .operations()
            .record_inquiry_responses(
                parse_project(project),
                vec![BatchResponseItem {
                    operation_id: OperationId::from_bytes([73; 16]),
                    response: CurrentHostResponse {
                        project_id: parse_project(project),
                        source_id: parse_source_identity(&source.identity),
                        host: "codex".into(),
                        session: "host-materiality-contract".into(),
                        turn: turn.into(),
                        displayed: DisplayedQuestion {
                            question_id: parse_question_identity(question_id),
                            revision,
                            alternative_keys: vec!["first".into(), "second".into()],
                            recommendation_key: Some("first".into()),
                        },
                        mapping: ResponseMapping::ExplicitDelegation {
                            delegate_to: "implementation-owner".into(),
                            user_rationale: Some("Choose within the displayed scope".into()),
                        },
                        applicability: ApplicabilityScope {
                            paths: Vec::new(),
                            components: Vec::new(),
                            work_contexts: Vec::new(),
                        },
                        assumptions: Vec::new(),
                        revisit_triggers: Vec::new(),
                    },
                }],
            )
            .expect("record delegation Decision");
        assert!(result.all_succeeded());
    } else {
        let decision = call(
            adapter,
            "decision_record",
            json!({
                "project_id":project,
                "question_id":question_id,
                "question_revision":revision,
                "alternative_key":"first",
                "user_turn":"Use the first displayed bounded approach"
            }),
        );
        assert_eq!(decision["result"]["isError"], false, "{decision}");
    }
    adapter
        .operations()
        .canonical_basis(parse_project(project))
        .expect("canonical basis")
        .active_decisions
        .iter()
        .find(|decision| decision.decision.question_id.to_string() == question_id)
        .expect("active Decision")
        .decision
        .id
        .to_string()
}

#[test]
fn mcp_workflow_guides_material_question_to_explicit_decision_and_ready_work() {
    let (_temporary, mut adapter, project) = setup();
    let goal = call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":"Teach me through the choices. Decide the user-visible failure mode, then implement it",
            "role":"goal",
            "statement":"Decide the user-visible failure mode, then implement it",
        }),
    );
    let goal = structured(&goal);
    assert_eq!(goal["workflow"]["stage"], "repository_baseline");
    assert_eq!(
        goal["workflow"]["required_next_action"]["tool"],
        "repository_analyze"
    );
    let goal_context_id = goal["context_item_id"].as_str().expect("Goal identity");
    let goal_source_id = goal["source_id"].as_str().expect("Goal Source identity");

    let analyzed = call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    );
    let analyzed = structured(&analyzed);
    assert_eq!(
        analyzed["workflow"]["stage"],
        "engineering_choice_discovery"
    );
    assert_eq!(
        analyzed["workflow"]["disposition"],
        "engineering_choice_discovery_required"
    );
    let baseline = analyzed["analysis_snapshot_id"]
        .as_str()
        .expect("baseline identity");
    let repository_source_id = analyzed["repository_source_id"]
        .as_str()
        .expect("repository Source identity");
    let discovery_id = record_fixture_discovery(
        &adapter,
        &project,
        goal_context_id,
        baseline,
        repository_source_id,
        FixtureEngineeringChoice {
            id: "failure-mode",
            affected_scope: "mcp-errors",
            effect_category: EngineeringEffectCategory::FailureOrErrorSemantics,
        },
    );

    let review = call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
            "rationale":"The failure mode is independently user-visible and user-owned.",
            "learning_participation":{"state":"active","user_turn_source_id":goal_source_id,"verbatim_statement":"Teach me through the choices"},
            "judgments":[{
                "choice_id":"failure-mode",
                "disposition":"unresolved_user_owned_outcome",
                "learning_value":{"state":"deliberation_worthy","rationale":"Failure semantics are worth learning, but user authority takes priority.","consequence_significance":["Callers observe different failures"],"transferable_principles":["Error contracts are API contracts"],"non_obvious_trade_offs":["More diagnostic detail can expose implementation structure"],"interruption_counterfactual":"Without participation, the requested understanding of public error contracts would be lost.","participation_scope_alignment":"The Goal asks to learn through meaningful public behavior choices."},
                "basis_summary":"No accepted authority selects the outcome",
                "authority_counterfactual":"Failure behavior varies materially, the Goal permits multiple outcomes, and no exact authority selects among them.",
                "materially_varying_outcomes":["the public failure contract and disclosure behavior"],
                "contains_user_owned_outcome":true,
                "user_owned_outcomes":["the public failure policy"],
                "ownership_rationale":"callers observe materially different product behavior",
                "ownership_source_ids":[repository_source_id]
            }]
        }),
    );
    let review = structured(&review);
    assert_eq!(
        review["workflow"]["stage"], "question_candidate",
        "{review}"
    );
    assert_eq!(
        review["workflow"]["required_next_action"]["action"],
        "submit_question_from_materiality"
    );
    let unresolved_candidates = structured(&call(
        &mut adapter,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    assert!(!unresolved_candidates["candidates"]
        .as_array()
        .expect("Candidate array")
        .iter()
        .any(|candidate| candidate["learning_deliberation"].is_object()));
    let review_id = review["review_candidate_id"]
        .as_str()
        .expect("review Candidate identity");

    let candidate = call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"submit_question_from_materiality",
            "project_id":project,
            "review_candidate_id":review_id,
            "dimension_id":"failure-mode",
            "research_state":"ready_to_ask",
            "research_state_basis":"Repository facts cannot choose the user-owned outcome",
            "retention_basis":"Retain through explicit resolution",
            "bounded_summary":"User-visible failure-mode choice",
            "prompt":"Which failure mode should users observe?",
            "why_now":"Implementation would otherwise choose the outcome",
            "alternatives":[
                {"key":"structured","label":"Structured error","consequence":"Return bounded structured guidance"},
                {"key":"plain","label":"Plain error","consequence":"Return only error text"}
            ],
            "recommendation_key":"structured",
            "recommendation_rationale":"Structured guidance is actionable",
            "duplicate_basis":"No current Question covers this dimension",
            "presentation_order":1
        }),
    );
    let candidate = structured(&candidate);
    assert_eq!(
        candidate["workflow"]["disposition"], "candidate_promotion_required",
        "{candidate}"
    );
    let candidate_id = candidate["candidate_id"]
        .as_str()
        .expect("Question Candidate identity");

    let promoted = call(
        &mut adapter,
        "candidate_manage",
        json!({"action":"promote_question","project_id":project,"candidate_id":candidate_id}),
    );
    let promoted = structured(&promoted);
    assert_eq!(
        promoted["workflow"]["required_next_action"]["tool"],
        "inquiry_frontier"
    );
    let question_id = promoted["question_id"].as_str().expect("Question identity");

    let frontier = call(
        &mut adapter,
        "inquiry_frontier",
        json!({"project_id":project}),
    );
    let frontier = structured(&frontier);
    assert_eq!(frontier["workflow"]["stage"], "decision");
    assert_eq!(
        frontier["workflow"]["required_next_action"]["tool"],
        "decision_record"
    );
    let revision = frontier["questions"][0]["revision"]
        .as_u64()
        .expect("Question revision");

    let decision = call(
        &mut adapter,
        "decision_record",
        json!({
            "project_id":project,
            "question_id":question_id,
            "question_revision":revision,
            "alternative_key":"structured",
            "user_turn":"Use the structured error",
        }),
    );
    let decision = structured(&decision);
    assert_eq!(
        decision["workflow"]["disposition"],
        "review_revision_required"
    );
    let canonical = adapter
        .operations()
        .canonical_basis(parse_project(&project))
        .expect("canonical basis");
    let decision_id = canonical
        .active_decisions
        .iter()
        .find(|decision| decision.decision.question_id.to_string() == question_id)
        .expect("active Decision")
        .decision
        .id
        .to_string();

    let revision_draft = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"draft",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
        }),
    ))
    .clone();
    assert_eq!(revision_draft["record_request"]["action"], "revise");
    assert_eq!(
        revision_draft["record_request"]["prefilled_fields"]["review_candidate_id"],
        review_id
    );
    let revised_learning = draft_learning_value(
        &revision_draft,
        "deliberation_worthy",
        json!({
            "rationale":"The learning value remains meaningful while the canonical Decision resolves user authority.",
            "consequence_significance":["Callers observe different failures"],
            "transferable_principles":["Error contracts are API contracts"],
            "non_obvious_trade_offs":["More diagnostic detail can expose implementation structure"],
            "interruption_counterfactual":"Without participation, the requested understanding of public error contracts would be lost.",
            "participation_scope_alignment":"The Goal asks to learn through meaningful public behavior choices."
        }),
    );
    let revised_judgment = draft_judgment(
        &revision_draft,
        "failure-mode",
        "resolved_user_owned_outcome",
        json!({
            "resolution_decision_id":decision_id,
            "learning_value":revised_learning,
            "basis_summary":"The current explicit Decision resolves this dimension",
            "authority_counterfactual":"The Decision now selects the exact failure outcome that the broad Goal did not select."
        }),
    );
    let revised_participation = draft_learning_participation(
        &revision_draft,
        "active",
        json!({
            "user_turn_source_id":goal_source_id,
            "verbatim_statement":"Teach me through the choices"
        }),
    );
    let revised = call(
        &mut adapter,
        "materiality_review",
        draft_request(
            &revision_draft,
            "The explicit current-host response now resolves the outcome.",
            revised_participation,
            vec![revised_judgment],
        ),
    );
    let revised = structured(&revised).clone();
    let revised = structured(&bind_recorded_scope(&mut adapter, &revised, &["src"])).clone();
    assert_eq!(revised["workflow"]["stage"], "ready_for_work");
    assert_eq!(revised["workflow"]["blocks_ordinary_work"], false);
    assert_eq!(
        revised["workflow"]["required_next_action"]["tool"],
        "checkpoint_record"
    );
    assert!(revised["workflow"]["satisfied_basis_identities"]
        .as_array()
        .expect("basis identities")
        .iter()
        .any(|basis| basis["kind"] == "decision" && basis["identity"] == decision_id));
}

#[test]
fn materiality_draft_surfaces_current_user_ownership_and_hidden_boundaries() {
    let (_temporary, mut adapter, project) = setup();
    let goal_turn =
        "Implement the feature, but leave the exit and background-running policy for me to choose.";
    let goal = structured(&call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":goal_turn,
            "role":"goal",
            "statement":goal_turn,
        }),
    ))
    .clone();
    let goal_context_id = goal["context_item_id"].as_str().expect("Goal identity");
    let goal_source_id = goal["source_id"].as_str().expect("Goal Source identity");
    let analyzed = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    let baseline = analyzed["analysis_snapshot_id"]
        .as_str()
        .expect("baseline identity");
    let repository_source = analyzed["repository_source_id"]
        .as_str()
        .expect("repository Source identity");
    let discovery = structured(&call(
        &mut adapter,
        "engineering_choice_discovery",
        json!({
            "project_id":project,
            "goal_context_id":goal_context_id,
            "baseline_analysis_snapshot_id":baseline,
            "source_operation":"hidden material-boundary discovery",
            "summary":"Three independent observable policy boundaries were found during repository research.",
            "choices":[
                {
                    "choice_id":"persistent-default-scope",
                    "summary":"Choose the persistent default scope",
                    "affected_scope":["configuration"],
                    "alternatives":[
                        {"alternative_id":"project","summary":"Persist per Project","technical_consequences":["All sessions inherit the Project default"]},
                        {"alternative_id":"session","summary":"Persist per session","technical_consequences":["Each session can select a different default"]}
                    ],
                    "technical_consequences":["The visible default and support commitment differ"],
                    "source_ids":[repository_source],
                    "effect_categories":["persistence_or_lifetime","user_visible_behavior_or_default","maintenance_or_support"],
                    "relationship":{"state":"independent"},
                    "evidence_state":"sufficient"
                },
                {
                    "choice_id":"signed-link-replay-policy",
                    "summary":"Choose public signed-link replay semantics",
                    "affected_scope":["public links"],
                    "alternatives":[
                        {"alternative_id":"single-use","summary":"Reject every replay","technical_consequences":["A consumed link cannot be reused"]},
                        {"alternative_id":"bounded-replay","summary":"Allow bounded replay","technical_consequences":["Reliability improves while exposure lasts longer"]}
                    ],
                    "technical_consequences":["Public security and replay behavior differ"],
                    "source_ids":[repository_source],
                    "effect_categories":["public_api_shape_or_semantics","security"],
                    "relationship":{"state":"independent"},
                    "evidence_state":"sufficient"
                },
                {
                    "choice_id":"exit-background-policy",
                    "summary":"Choose whether close exits or keeps background work running",
                    "affected_scope":["process lifecycle"],
                    "alternatives":[
                        {"alternative_id":"exit","summary":"Exit immediately","technical_consequences":["Background work stops"]},
                        {"alternative_id":"continue","summary":"Continue in background","technical_consequences":["Work remains active after close"]}
                    ],
                    "technical_consequences":["The user-visible close policy differs"],
                    "source_ids":[repository_source],
                    "effect_categories":["user_visible_behavior_or_default","concurrency_or_operability"],
                    "relationship":{"state":"independent"},
                    "evidence_state":"sufficient"
                }
            ],
            "material_boundary_review":complete_material_boundary_review_json(repository_source, &[
                ("public_api_shape_or_semantics", &["signed-link-replay-policy"]),
                ("persistence_or_lifetime", &["persistent-default-scope"]),
                ("security", &["signed-link-replay-policy"]),
                ("user_visible_behavior_or_default", &["persistent-default-scope", "exit-background-policy"]),
                ("concurrency_or_operability", &["exit-background-policy"]),
                ("maintenance_or_support", &["persistent-default-scope"]),
            ])
        }),
    ))
    .clone();
    let draft = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"draft",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery["discovery_candidate_id"],
        }),
    ))
    .clone();

    assert_eq!(draft["current_goal"]["goal_context_id"], goal_context_id);
    assert_eq!(draft["current_goal"]["statement"], goal_turn);
    assert_eq!(
        draft["current_goal"]["current_host_user_turn_source_ids"],
        json!([goal_source_id])
    );
    let authority_input = draft["current_goal_authority_inputs"]
        .as_array()
        .expect("current Goal authority inputs")
        .iter()
        .find(|candidate| candidate["dimension_id"] == "signed-link-replay-policy")
        .expect("signed-link authority input");
    assert_eq!(authority_input["goal_context_id"], goal_context_id);
    assert_eq!(authority_input["user_turn_source_id"], goal_source_id);
    assert_eq!(authority_input["exact_goal_text"], goal_turn);
    assert_eq!(authority_input["affected_scope"], json!(["public links"]));
    assert_eq!(
        authority_input["effect_categories"],
        json!(["public_api_shape_or_semantics", "security"])
    );
    assert!(authority_input["authority_boundary"]
        .as_str()
        .is_some_and(|notice| notice.contains("not delegation evidence")));
    assert!(draft["current_goal"]["ownership_notice"]
        .as_str()
        .is_some_and(|notice| notice.contains("do not downgrade")));
    assert!(
        draft["authority_decision_checklist"]["counterfactual_questions"]
            .as_array()
            .is_some_and(|questions| questions.len() == 6)
    );
    assert_eq!(
        draft["authority_decision_checklist"]["not_authority"],
        json!([
            "authority to perform the overall feature request",
            "imperative wording in the overall Goal",
            "implementation preference",
            "agent recommendation",
            "library or repository convention"
        ])
    );
    assert!(
        draft["authority_decision_checklist"]["outcomes"]["unresolved_user_owned_outcome"]
            .as_str()
            .is_some_and(|guidance| guidance.contains("no exact authority"))
    );
    assert!(
        draft["authority_decision_checklist"]["subordinate_boundary_instruction"]
            .as_str()
            .is_some_and(|guidance| guidance.contains("overall Goal is not blanket authority"))
    );
    assert!(
        draft["authority_decision_checklist"]["outcomes"]["agent_owned_implementation_choice"]
            .as_str()
            .is_some_and(|guidance| guidance.contains("material user-facing policy is settled"))
    );
    assert!(
        draft["authority_decision_checklist"]["authority_revision_chronology"]
            .as_str()
            .is_some_and(|guidance| guidance.contains("blocking readiness"))
    );
    let templates = draft["judgment_templates"]
        .as_array()
        .expect("judgment templates");
    assert_eq!(templates.len(), 3);
    assert!(templates.iter().all(|template| {
        template["discovery_owned"].get("disposition").is_none()
            && template["caller_owned_judgment"]["prefilled_fields"]["choice_id"].is_string()
            && template["caller_owned_judgment"]["legal_judgment_variant_ids"]
                .as_array()
                .is_some_and(|variants| variants.len() == 13)
    }));
    let unresolved = draft["judgment_contracts"]
        .as_array()
        .expect("judgment contracts")
        .iter()
        .find(|contract| contract["variant_id"] == "unresolved_user_owned_outcome")
        .expect("unresolved contract");
    assert!(unresolved["forbidden_fields"]
        .as_array()
        .is_some_and(|fields| fields.iter().any(|field| field == "contract_basis")));
    assert!(unresolved["caller_must_semantically_provide"]
        .as_array()
        .is_some_and(|fields| fields
            .iter()
            .any(|field| field == "authority_counterfactual")));
    assert!(unresolved["caller_may_provide"]
        .as_array()
        .is_some_and(|fields| fields
            .iter()
            .any(|field| field == "evidence_completion_basis")));
    assert!(draft["evidence_state_precedence"]["rule"]
        .as_str()
        .is_some_and(|rule| rule.contains("blocks ordinary work")));
    assert!(unresolved["caller_may_provide"]
        .as_array()
        .is_some_and(|fields| fields
            .iter()
            .any(|field| field == "evidence_completion_basis")));
    assert!(draft["evidence_state_precedence"]["rule"]
        .as_str()
        .is_some_and(|rule| rule.contains("blocks ordinary work")));
    assert_eq!(
        draft["learning_value_input_alternatives"][1]["required_fields"],
        json!([
            "state",
            "rationale",
            "consequence_significance",
            "transferable_principles",
            "non_obvious_trade_offs",
            "interruption_counterfactual",
            "participation_scope_alignment"
        ])
    );
    assert_eq!(
        draft["learning_participation"]["input_alternatives"][1]["required_fields"],
        json!(["state", "user_turn_source_id", "verbatim_statement"])
    );
    assert!(
        draft["learning_value_interruption_contract"]["counterfactual"]
            .as_str()
            .is_some_and(|value| value.contains("what meaningful transferable understanding"))
    );
    assert!(
        draft["learning_value_interruption_contract"]["source_to_consider"]
            .as_str()
            .is_some_and(|value| value.contains("narrowing clause"))
    );
    assert!(draft["authority_learning_routing"]["scope_rule"]
        .as_str()
        .is_some_and(|value| value.contains("generic alternative count is not enough")));
    assert!(draft["record_request"]["input_schema"].is_object());
}

#[test]
fn materiality_draft_has_one_record_path_for_every_disposition() {
    let cases = vec![
        (
            "repository-fact",
            "repository_or_environment_fact",
            json!({
                "basis_summary":"The repository fixes the only viable value.",
                "authority_counterfactual":"Repository evidence establishes one outcome, so the Goal is not used as delegation.",
                "authority_coverage":"The observed repository representation fixes the complete bounded choice.",
                "remaining_credible_alternatives":[],
                "unique_outcome_rationale":"Only the observed representation is mechanically valid for this repository."
            }),
            "ready_for_work",
        ),
        (
            "settled-contract",
            "settled_authority_by_contract",
            json!({
                "basis_summary":"The active owner settles this exact dimension.",
                "authority_counterfactual":"The accepted owner contract selects the exact outcome independently of the Goal.",
                "authority_coverage":"The accepted contract specifies the complete bounded choice.",
                "remaining_credible_alternatives":[],
                "unique_outcome_rationale":"The contract normatively requires one exact outcome.",
                "contract_basis":["rebuild/docs/design/inquiry-and-decision.md"]
            }),
            "ready_for_work",
        ),
        (
            "agent-owned",
            "agent_owned_implementation_choice",
            json!({
                "basis_summary":"No user-facing policy varies across the alternatives.",
                "authority_counterfactual":"Only mechanically equivalent internal implementation details vary, so no material user outcome needs authority."
            }),
            "ready_for_work",
        ),
        (
            "current-task-delegation",
            "delegated_implementation_choice_current_task",
            json!({
                "basis_summary":"The exact current Goal delegates this bounded implementation choice.",
                "authority_counterfactual":"The verbatim Goal statement explicitly delegates this exact bounded internal choice, not merely the encompassing work.",
                "delegation_statement":"I delegate the bounded-choice implementation to you",
                "delegated_scope":["src/lib.rs"]
            }),
            "ready_for_work",
        ),
        (
            "research-required",
            "exploratory_uncertainty_research_required",
            json!({
                "basis_summary":"Repository evidence is still required.",
                "authority_counterfactual":"Empirical repository evidence is needed before the reality or consequence of the alternatives is known.",
                "research_basis":["Inspect the current adapter behavior"]
            }),
            "research_or_prototype",
        ),
        (
            "prototype-required",
            "exploratory_uncertainty_prototype_required",
            json!({
                "basis_summary":"A bounded prototype is still required.",
                "authority_counterfactual":"A prototype must establish the materially relevant behavior before any authority choice remains.",
                "research_basis":["Prototype both observable behaviors"]
            }),
            "research_or_prototype",
        ),
        (
            "deferred-with-revisit",
            "exploratory_uncertainty_deferred_with_revisit",
            json!({
                "basis_summary":"The choice is intentionally deferred with an inspectable trigger.",
                "authority_counterfactual":"The exact outcome remains open under an explicit bounded defer-and-revisit basis.",
                "research_basis":["Revisit when the bounded collection exceeds 16 entries"]
            }),
            "ready_for_work",
        ),
        (
            "resolved-by-research",
            "exploratory_uncertainty_resolved_by_research",
            json!({
                "basis_summary":"Repository research removed the uncertainty.",
                "authority_counterfactual":"Research establishes the supported outcome, so no user preference is inferred from the Goal.",
                "research_basis":["The retained snapshot proves only one supported behavior"]
            }),
            "ready_for_work",
        ),
        (
            "unresolved-user-owned",
            "unresolved_user_owned_outcome",
            json!({
                "basis_summary":"No exact authority settles the materially different outcomes.",
                "authority_counterfactual":"The Goal permits multiple materially different outcomes and no fact, contract, Decision, or exact delegation selects one."
            }),
            "question_candidate",
        ),
    ];

    for (label, variant_id, semantic_fields, expected_stage) in cases {
        let (_temporary, mut adapter, project) = setup();
        let goal_turn =
            "Implement the change; I delegate the bounded-choice implementation to you.";
        let goal = structured(&call(
            &mut adapter,
            "context_record",
            json!({"project_id":project,"user_turn":goal_turn,"role":"goal","statement":goal_turn}),
        ))
        .clone();
        let analyzed = structured(&call(
            &mut adapter,
            "repository_analyze",
            json!({"project_id":project}),
        ))
        .clone();
        let discovery_id = record_fixture_discovery(
            &adapter,
            &project,
            goal["context_item_id"].as_str().expect("Goal identity"),
            analyzed["analysis_snapshot_id"]
                .as_str()
                .expect("baseline identity"),
            analyzed["repository_source_id"]
                .as_str()
                .expect("repository Source identity"),
            FixtureEngineeringChoice {
                id: "bounded-choice",
                affected_scope: "src/lib.rs",
                effect_category: EngineeringEffectCategory::ImplementationInternal,
            },
        );
        let draft = structured(&call(
            &mut adapter,
            "materiality_review",
            json!({
                "action":"draft",
                "project_id":project,
                "engineering_choice_discovery_candidate_id":discovery_id,
            }),
        ))
        .clone();
        let learning_value = draft_learning_value(
            &draft,
            "routine",
            json!({"rationale":format!("one-pass {label} learning assessment")}),
        );
        let mut semantic_fields = semantic_fields
            .as_object()
            .expect("case semantic fields")
            .clone();
        semantic_fields.insert("learning_value".into(), learning_value);
        let judgment = draft_judgment(
            &draft,
            "bounded-choice",
            variant_id,
            Value::Object(semantic_fields),
        );
        let participation = draft_learning_participation(&draft, "inactive", json!({}));
        let recorded = call(
            &mut adapter,
            "materiality_review",
            draft_request(
                &draft,
                &format!("one-pass {label} review"),
                participation,
                vec![judgment],
            ),
        );
        assert_eq!(recorded["result"]["isError"], false, "{label}: {recorded}");
        if expected_stage == "ready_for_work" {
            assert_eq!(
                structured(&recorded)["workflow"]["stage"],
                "materiality_review"
            );
            let bound = bind_recorded_scope(&mut adapter, &recorded, &["src/lib.rs"]);
            assert_eq!(
                structured(&bound)["workflow"]["stage"],
                expected_stage,
                "{label}: {bound}"
            );
        } else {
            assert_eq!(
                structured(&recorded)["workflow"]["stage"],
                expected_stage,
                "{label}: {recorded}"
            );
        }
    }
}

#[test]
fn broad_feature_goals_require_exact_authority_for_hidden_material_outcomes() {
    for (goal_turn, choice_id, scope, effect_category) in [
        (
            "Add automatic expiry and cleanup.",
            "expiry-trigger-lifetime-recovery",
            "automatic cleanup policy",
            EngineeringEffectCategory::PersistenceOrLifetime,
        ),
        (
            "Add npm update availability.",
            "npm-activation-network-default-support",
            "npm update policy",
            EngineeringEffectCategory::UserVisibleBehaviorOrDefault,
        ),
    ] {
        let (_temporary, mut adapter, project) = setup();
        let goal = structured(&call(
            &mut adapter,
            "context_record",
            json!({"project_id":project,"user_turn":goal_turn,"role":"goal","statement":goal_turn}),
        ))
        .clone();
        let analyzed = structured(&call(
            &mut adapter,
            "repository_analyze",
            json!({"project_id":project}),
        ))
        .clone();
        let discovery_id = record_fixture_discovery(
            &adapter,
            &project,
            goal["context_item_id"].as_str().expect("Goal identity"),
            analyzed["analysis_snapshot_id"]
                .as_str()
                .expect("baseline identity"),
            analyzed["repository_source_id"]
                .as_str()
                .expect("repository Source identity"),
            FixtureEngineeringChoice {
                id: choice_id,
                affected_scope: scope,
                effect_category,
            },
        );
        let draft = structured(&call(
            &mut adapter,
            "materiality_review",
            json!({
                "action":"draft",
                "project_id":project,
                "engineering_choice_discovery_candidate_id":discovery_id,
            }),
        ))
        .clone();

        assert!(draft.get("delegation_evidence_candidates").is_none());
        assert!(
            draft["current_goal_authority_inputs"][0]["authority_boundary"]
                .as_str()
                .is_some_and(|notice| notice.contains("not delegation evidence"))
        );
        assert!(draft["current_goal"]["ownership_notice"]
            .as_str()
            .is_some_and(
                |notice| notice.contains("not every subordinate material product outcome")
            ));

        let unexamined_delegation = structured(&call(
            &mut adapter,
            "materiality_review",
            json!({
                "action":"record",
                "project_id":project,
                "engineering_choice_discovery_candidate_id":discovery_id,
                "rationale":"The broad request is incorrectly claimed as blanket delegation.",
                "learning_participation":{"state":"inactive"},
                "judgments":[{
                    "choice_id":choice_id,
                    "disposition":"delegated_implementation_choice",
                    "basis_summary":"The broad Goal requested the feature.",
                    "delegation_statement":goal_turn,
                    "delegated_scope":[scope],
                    "learning_value":{"state":"routine","rationale":"Authority and learning are independent."}
                }]
            }),
        ))
        .clone();
        assert!(unexamined_delegation["details"]["problems"]
            .as_array()
            .is_some_and(|problems| problems
                .iter()
                .any(|problem| problem
                    == "arguments.judgments[0].authority_counterfactual is required")));

        let unresolved = structured(&call(
            &mut adapter,
            "materiality_review",
            json!({
                "action":"record",
                "project_id":project,
                "engineering_choice_discovery_candidate_id":discovery_id,
                "rationale":"Credible alternatives change a material product policy that the broad Goal does not select.",
                "learning_participation":{"state":"inactive"},
                "judgments":[{
                    "choice_id":choice_id,
                    "disposition":"unresolved_user_owned_outcome",
                    "basis_summary":"No exact authority selects among the materially different outcomes.",
                    "authority_counterfactual":"The Goal can be satisfied by multiple materially different outcomes; no repository fact, accepted contract, applicable Decision, or exact explicit delegation selects among them, so this remains user-owned.",
                    "materially_varying_outcomes":["the public behavior selected by the credible alternatives"],
                    "contains_user_owned_outcome":true,
                    "user_owned_outcomes":["the public product policy and its compatibility lifetime"],
                    "ownership_rationale":"the credible alternatives change behavior owned by the user rather than private implementation mechanics",
                    "ownership_source_ids":[analyzed["repository_source_id"]],
                    "learning_value":{"state":"routine","rationale":"Authority routing is independent of learning participation."}
                }]
            }),
        ))
        .clone();
        assert_eq!(unresolved["workflow"]["stage"], "question_candidate");
        assert_eq!(
            unresolved["workflow"]["required_next_action"]["action"],
            "submit_question_from_materiality"
        );
        assert_eq!(unresolved["workflow"]["blocks_ordinary_work"], true);
    }
}

#[test]
fn constraining_architecture_and_convention_cannot_claim_exact_authority() {
    let cases = [
        (
            "Add automatic Candidate expiry cleanup without mutating read-only projections.",
            "candidate-expiry-cleanup-trigger",
            "Candidate lifecycle mutation boundary",
            EngineeringEffectCategory::PersistenceOrLifetime,
            "settled_authority_by_contract",
            json!({
                "basis_summary":"Architecture assigns cleanup mutation to Inquiry and excludes read-only projection mutation.",
                "authority_counterfactual":"The architecture is relevant but does not choose among materially different Inquiry-owned cleanup triggers.",
                "authority_coverage":"Ownership of Candidate cleanup and the prohibition on projection-side mutation.",
                "remaining_credible_alternatives":["synchronous cleanup during an Inquiry mutation","periodic Inquiry-owned retention cleanup"],
                "unique_outcome_rationale":"No unique trigger is selected because both alternatives satisfy the cited architecture constraints.",
                "contract_basis":["rebuild/docs/design/architecture.md","rebuild/docs/design/inquiry-and-decision.md"]
            }),
        ),
        (
            "Add a Project-local token-file contract for the local client.",
            "project-local-token-file-contract",
            "public credential file contract",
            EngineeringEffectCategory::Security,
            "repository_or_environment_fact",
            json!({
                "basis_summary":"Existing libraries and repository conventions provide a strong token-file precedent.",
                "authority_counterfactual":"The convention is relevant but has not been adopted as the exact public filename, representation, and failure policy.",
                "authority_coverage":"Existing library and repository convention for local credential files.",
                "remaining_credible_alternatives":["a fixed plaintext token filename with strict failure","a structured credential file with recoverable missing-file behavior"],
                "unique_outcome_rationale":"No unique public contract is selected because both policies remain compatible with the observed repository.",
            }),
        ),
    ];

    for (goal_turn, choice_id, scope, effect_category, variant_id, settling_fields) in cases {
        let (_temporary, mut adapter, project) = setup();
        let goal = structured(&call(
            &mut adapter,
            "context_record",
            json!({"project_id":project,"user_turn":goal_turn,"role":"goal","statement":goal_turn}),
        ))
        .clone();
        let analyzed = structured(&call(
            &mut adapter,
            "repository_analyze",
            json!({"project_id":project}),
        ))
        .clone();
        let discovery_id = record_fixture_discovery(
            &adapter,
            &project,
            goal["context_item_id"].as_str().expect("Goal identity"),
            analyzed["analysis_snapshot_id"]
                .as_str()
                .expect("baseline identity"),
            analyzed["repository_source_id"]
                .as_str()
                .expect("repository Source identity"),
            FixtureEngineeringChoice {
                id: choice_id,
                affected_scope: scope,
                effect_category,
            },
        );
        let draft = structured(&call(
            &mut adapter,
            "materiality_review",
            json!({
                "action":"draft",
                "project_id":project,
                "engineering_choice_discovery_candidate_id":discovery_id,
            }),
        ))
        .clone();
        assert!(
            draft["authority_decision_checklist"]["remaining_alternatives_rule"]
                .as_str()
                .is_some_and(|rule| rule.contains("constrained rather than settled"))
        );
        assert_eq!(
            draft["exact_authority_sufficiency_contract"]["semantic_owner"],
            "active_agent"
        );

        let mut settling_fields = settling_fields
            .as_object()
            .expect("settling fields")
            .clone();
        settling_fields.insert(
            "learning_value".into(),
            draft_learning_value(
                &draft,
                "routine",
                json!({"rationale":"Authority routing is independent of learning."}),
            ),
        );
        let rejected = call(
            &mut adapter,
            "materiality_review",
            draft_request(
                &draft,
                "Test whether constraining evidence uniquely settles the exact dimension.",
                draft_learning_participation(&draft, "inactive", json!({})),
                vec![draft_judgment(
                    &draft,
                    choice_id,
                    variant_id,
                    Value::Object(settling_fields),
                )],
            ),
        );
        assert_eq!(rejected["result"]["isError"], true, "{rejected}");
        assert!(structured(&rejected)["error"]
            .as_str()
            .is_some_and(|error| error.contains("credible alternatives remain")));

        let unresolved = call(
            &mut adapter,
            "materiality_review",
            draft_request(
                &draft,
                "No exact authority uniquely selects the material outcome.",
                draft_learning_participation(&draft, "inactive", json!({})),
                vec![draft_judgment(
                    &draft,
                    choice_id,
                    "unresolved_user_owned_outcome",
                    json!({
                        "basis_summary":"Multiple materially different credible outcomes remain after all valid constraints are applied.",
                        "authority_counterfactual":"The cited evidence constrains the alternatives but no exact repository fact, contract, Decision, or delegation selects one.",
                        "learning_value":{"state":"routine","rationale":"User-owned authority stays on the Question path."}
                    }),
                )],
            ),
        );
        assert_eq!(unresolved["result"]["isError"], false, "{unresolved}");
        assert_eq!(
            structured(&unresolved)["workflow"]["stage"],
            "question_candidate"
        );
        assert_eq!(
            structured(&unresolved)["workflow"]["required_next_action"]["action"],
            "submit_question_from_materiality"
        );
    }
}

#[test]
fn materiality_draft_one_call_supports_decision_and_inquiry_delegation_variants() {
    let (_temporary, mut adapter, project) = setup();
    let goal_turn = "Implement the bounded choice and preserve explicit authority";
    let goal = structured(&call(
        &mut adapter,
        "context_record",
        json!({"project_id":project,"user_turn":goal_turn,"role":"goal","statement":goal_turn}),
    ))
    .clone();
    let analyzed = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    let repository_source_id = analyzed["repository_source_id"]
        .as_str()
        .expect("repository Source identity");
    let settled_decision_id =
        record_host_question_decision(&mut adapter, &project, repository_source_id, None, false);
    let settled_discovery_id = record_fixture_discovery(
        &adapter,
        &project,
        goal["context_item_id"].as_str().expect("Goal identity"),
        analyzed["analysis_snapshot_id"]
            .as_str()
            .expect("baseline identity"),
        repository_source_id,
        FixtureEngineeringChoice {
            id: "settled-by-decision",
            affected_scope: "src/lib.rs",
            effect_category: EngineeringEffectCategory::ImplementationInternal,
        },
    );
    let settled_draft = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"draft",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":settled_discovery_id,
        }),
    ))
    .clone();
    let settled_learning = draft_learning_value(
        &settled_draft,
        "routine",
        json!({"rationale":"Applying the current Decision is routine."}),
    );
    let settled_judgment = draft_judgment(
        &settled_draft,
        "settled-by-decision",
        "settled_authority_by_decision",
        json!({
            "basis_summary":"The current applicable Decision settles this exact scope.",
            "authority_counterfactual":"The applicable Decision selects the outcome independently of the encompassing Goal.",
            "authority_coverage":"The Decision applies to the complete discovered dimension.",
            "remaining_credible_alternatives":[],
            "unique_outcome_rationale":"The applicable user Decision selects one exact outcome.",
            "decision_ids":[settled_decision_id],
            "learning_value":settled_learning,
        }),
    );
    let settled = call(
        &mut adapter,
        "materiality_review",
        draft_request(
            &settled_draft,
            "Draft-driven settled Decision review",
            draft_learning_participation(&settled_draft, "inactive", json!({})),
            vec![settled_judgment],
        ),
    );
    assert_eq!(settled["result"]["isError"], false, "{settled}");
    let settled = bind_recorded_scope(&mut adapter, &settled, &["src/lib.rs"]);
    assert_eq!(structured(&settled)["workflow"]["stage"], "ready_for_work");

    let (_temporary, mut adapter, project) = setup();
    let goal_turn = "Implement the bounded choice after any required authority is resolved";
    let goal = structured(&call(
        &mut adapter,
        "context_record",
        json!({"project_id":project,"user_turn":goal_turn,"role":"goal","statement":goal_turn}),
    ))
    .clone();
    let analyzed = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    let discovery_id = record_fixture_discovery(
        &adapter,
        &project,
        goal["context_item_id"].as_str().expect("Goal identity"),
        analyzed["analysis_snapshot_id"]
            .as_str()
            .expect("baseline identity"),
        analyzed["repository_source_id"]
            .as_str()
            .expect("repository Source identity"),
        FixtureEngineeringChoice {
            id: "inquiry-delegated-choice",
            affected_scope: "src/lib.rs",
            effect_category: EngineeringEffectCategory::ImplementationInternal,
        },
    );
    let initial_draft = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"draft",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
        }),
    ))
    .clone();
    let unresolved = draft_judgment(
        &initial_draft,
        "inquiry-delegated-choice",
        "unresolved_user_owned_outcome",
        json!({
            "basis_summary":"No exact authority currently settles this bounded outcome.",
            "authority_counterfactual":"The Goal can be fulfilled through different outcomes and no exact authority selects one yet.",
            "learning_value":draft_learning_value(
                &initial_draft,
                "routine",
                json!({"rationale":"Canonical authority takes priority."}),
            ),
        }),
    );
    let recorded = structured(&call(
        &mut adapter,
        "materiality_review",
        draft_request(
            &initial_draft,
            "Initial unresolved review",
            draft_learning_participation(&initial_draft, "inactive", json!({})),
            vec![unresolved],
        ),
    ))
    .clone();
    assert_eq!(recorded["workflow"]["stage"], "question_candidate");
    let review_id = recorded["review_candidate_id"]
        .as_str()
        .expect("review identity");
    let delegation_decision_id = record_host_question_decision(
        &mut adapter,
        &project,
        analyzed["repository_source_id"]
            .as_str()
            .expect("repository Source identity"),
        Some((review_id, "inquiry-delegated-choice")),
        true,
    );
    let revision_draft = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"draft",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
        }),
    ))
    .clone();
    assert_eq!(revision_draft["record_request"]["action"], "revise");
    let delegated = draft_judgment(
        &revision_draft,
        "inquiry-delegated-choice",
        "delegated_implementation_choice_inquiry_time",
        json!({
            "basis_summary":"The exact current Question response delegates this dimension.",
            "authority_counterfactual":"The current Question response explicitly delegates this exact dimension rather than merely requesting the broader work.",
            "decision_ids":[delegation_decision_id],
            "learning_value":draft_learning_value(
                &revision_draft,
                "routine",
                json!({"rationale":"The delegated bounded implementation is routine."}),
            ),
        }),
    );
    let revised = call(
        &mut adapter,
        "materiality_review",
        draft_request(
            &revision_draft,
            "Draft-driven Inquiry-time delegation revision",
            draft_learning_participation(&revision_draft, "inactive", json!({})),
            vec![delegated],
        ),
    );
    assert_eq!(revised["result"]["isError"], false, "{revised}");
    let revised = bind_recorded_scope(&mut adapter, &revised, &["src/lib.rs"]);
    assert_eq!(
        structured(&revised)["workflow"]["stage"],
        "ready_for_work",
        "{revised}"
    );
}

#[test]
fn materiality_validation_reports_exact_correction_context() {
    let (_temporary, mut adapter, project) = setup();
    let goal = structured(&call(
        &mut adapter,
        "context_record",
        json!({"project_id":project,"user_turn":"Implement the bounded choice","role":"goal","statement":"Implement the bounded choice"}),
    ))
    .clone();
    let analyzed = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    let current_source_id = analyzed["repository_source_id"].clone();
    let discovery_id = record_fixture_discovery(
        &adapter,
        &project,
        goal["context_item_id"].as_str().expect("Goal identity"),
        analyzed["analysis_snapshot_id"]
            .as_str()
            .expect("baseline identity"),
        analyzed["repository_source_id"]
            .as_str()
            .expect("repository Source identity"),
        FixtureEngineeringChoice {
            id: "bounded-choice",
            affected_scope: "src/lib.rs",
            effect_category: EngineeringEffectCategory::ImplementationInternal,
        },
    );

    let forbidden = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
            "rationale":"invalid field combination",
            "learning_participation":{"state":"inactive"},
            "judgments":[{
                "choice_id":"bounded-choice",
                "disposition":"agent_owned_implementation_choice",
                "basis_summary":"implementation discretion",
                "authority_counterfactual":"No material product outcome varies in this invalid-field fixture.",
                "contract_basis":["must be forbidden here"],
                "learning_value":{"state":"routine","rationale":"routine"}
            }]
        }),
    ))
    .clone();
    assert!(forbidden["details"]["problems"]
        .as_array()
        .expect("schema problems")
        .iter()
        .any(|problem| problem.as_str().is_some_and(
            |problem| problem == "arguments.judgments[0].contract_basis is not allowed"
        )));
    assert_eq!(
        forbidden["details"]["materiality_context"]["bound_identities"]
            ["engineering_choice_discovery_candidate_id"],
        discovery_id
    );
    assert_eq!(
        forbidden["details"]["materiality_context"]["next_supported_action"]["action"],
        "draft"
    );

    for (disposition, forbidden_field, judgment) in [
        (
            "repository_or_environment_fact",
            "research_basis",
            json!({
                "choice_id":"bounded-choice",
                "disposition":"repository_or_environment_fact",
                "basis_summary":"repository fact",
                "authority_counterfactual":"The repository would select the exact outcome in this invalid-field fixture.",
                "authority_coverage":"The complete bounded choice.",
                "remaining_credible_alternatives":[],
                "unique_outcome_rationale":"The fixture claims one mechanically valid outcome.",
                "research_basis":["not legal for this disposition"],
                "learning_value":{"state":"routine","rationale":"routine"}
            }),
        ),
        (
            "delegated_implementation_choice",
            "contract_basis",
            json!({
                "choice_id":"bounded-choice",
                "disposition":"delegated_implementation_choice",
                "basis_summary":"invalid mixed authority",
                "authority_counterfactual":"The statement would need to delegate the exact outcome; this payload intentionally mixes forbidden authority.",
                "delegation_statement":"Implement the bounded choice",
                "delegated_scope":["src/lib.rs"],
                "contract_basis":["not delegation"],
                "learning_value":{"state":"routine","rationale":"routine"}
            }),
        ),
        (
            "exploratory_uncertainty",
            "resolution_decision_id",
            json!({
                "choice_id":"bounded-choice",
                "disposition":"exploratory_uncertainty",
                "exploratory_disposition":"research_required",
                "basis_summary":"research remains",
                "authority_counterfactual":"Empirical evidence is required before an authority choice can be identified.",
                "research_basis":["inspect the repository"],
                "resolution_decision_id":"00000000000000000000000000000000",
                "learning_value":{"state":"routine","rationale":"routine"}
            }),
        ),
    ] {
        let rejected = structured(&call(
            &mut adapter,
            "materiality_review",
            json!({
                "action":"record",
                "project_id":project,
                "engineering_choice_discovery_candidate_id":discovery_id,
                "rationale":format!("reject {disposition} cross-field"),
                "learning_participation":{"state":"inactive"},
                "judgments":[judgment]
            }),
        ))
        .clone();
        let exact_problem = format!("arguments.judgments[0].{forbidden_field} is not allowed");
        assert!(rejected["details"]["problems"]
            .as_array()
            .expect("schema problems")
            .iter()
            .any(|problem| problem == &exact_problem));
    }

    let unknown = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
            "rationale":"unknown choice identity",
            "learning_participation":{"state":"inactive"},
            "judgments":[{
                "choice_id":"not-in-discovery",
                "disposition":"agent_owned_implementation_choice",
                "basis_summary":"invalid choice",
                "authority_counterfactual":"This invalid identity cannot be evaluated against the discovered material outcome.",
                "materially_varying_outcomes":["the bounded fixture outcome"],
                "contains_user_owned_outcome":false,
                "user_owned_outcomes":[],
                "ownership_rationale":"the invalid fixture claims bounded implementation ownership",
                "bounded_implementation_discretion_rationale":"all alternatives preserve settled behavior",
                "ownership_source_ids":[current_source_id],
                "learning_value":{"state":"routine","rationale":"routine"}
            }]
        }),
    ))
    .clone();
    assert_eq!(
        unknown["details"]["field_path"],
        "arguments.judgments[0].choice_id"
    );
    assert_eq!(unknown["details"]["invalid_value"], "not-in-discovery");
    assert_eq!(
        unknown["details"]["allowed_values"],
        json!(["bounded-choice"])
    );
    assert_eq!(
        unknown["details"]["next_supported_action"]["action"],
        "draft"
    );

    let invalid_decision = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
            "rationale":"inactive Decision authority",
            "learning_participation":{"state":"inactive"},
            "judgments":[{
                "choice_id":"bounded-choice",
                "disposition":"settled_authority",
                "basis_summary":"The claimed Decision does not exist in the current Project.",
                "authority_counterfactual":"The claimed exact authority is invalid and therefore cannot select the outcome.",
                "materially_varying_outcomes":["the bounded fixture outcome"],
                "contains_user_owned_outcome":true,
                "user_owned_outcomes":["the bounded fixture policy"],
                "ownership_rationale":"the alternatives change user-owned product policy",
                "ownership_source_ids":[current_source_id],
                "authority_coverage":"The complete bounded choice.",
                "remaining_credible_alternatives":[],
                "unique_outcome_rationale":"The claimed Decision would select one outcome if it were applicable.",
                "decision_ids":["00000000000000000000000000000000"],
                "learning_value":{"state":"routine","rationale":"routine"}
            }]
        }),
    ))
    .clone();
    assert_eq!(
        invalid_decision["details"]["diagnostic"],
        "materiality_contract_failure"
    );
    assert!(invalid_decision["details"]["problem"]
        .as_str()
        .is_some_and(|problem| problem.contains("authority Decision")
            && problem.contains("not active in the current Project")));
    assert_eq!(
        invalid_decision["details"]["next_supported_action"]["action"],
        "draft"
    );
}

#[test]
fn installed_mcp_learning_deliberation_is_ordered_restartable_and_not_a_decision() {
    let (temporary, mut adapter, project) = setup();
    let user_turn =
        "Teach me through meaningful technical choices while we implement the cache boundary";
    let goal = structured(&call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":user_turn,
            "role":"goal",
            "statement":user_turn,
        }),
    ))
    .clone();
    let goal_context_id = goal["context_item_id"].as_str().expect("Goal identity");
    let goal_source_id = goal["source_id"].as_str().expect("Goal Source identity");
    let analyzed = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    let baseline = analyzed["analysis_snapshot_id"]
        .as_str()
        .expect("baseline identity");
    let repository_source = analyzed["repository_source_id"]
        .as_str()
        .expect("repository Source identity");
    assert_eq!(
        analyzed["workflow"]["input_guidance"]["available_identities"]["goal_context_id"],
        goal_context_id
    );
    assert!(
        analyzed["workflow"]["input_guidance"]["discovery_completion_counterfactual"]
            .as_str()
            .is_some_and(|value| value.contains("subordinate product outcomes"))
    );
    assert_eq!(
        analyzed["workflow"]["input_guidance"]["material_boundary_review"]["semantic_owner"],
        "active_agent"
    );

    let discovery = structured(&call(
        &mut adapter,
        "engineering_choice_discovery",
        json!({
            "project_id":project,
            "goal_context_id":goal_context_id,
            "baseline_analysis_snapshot_id":baseline,
            "source_operation":"installed MCP learning discovery",
            "summary":"Choose the cache invalidation boundary",
            "choices":[{
                "choice_id":"cache-invalidation-boundary",
                "summary":"Place invalidation at mutation sites or behind a versioned cache facade",
                "affected_scope":["cache","mutation paths"],
                "alternatives":[
                    {"alternative_id":"mutation-sites","summary":"Invalidate at each mutation site","technical_consequences":["Simple reads but distributed invalidation obligations"]},
                    {"alternative_id":"versioned-facade","summary":"Use a versioned cache facade","technical_consequences":["Centralized correctness with indirection and version bookkeeping"]}
                ],
                "technical_consequences":["The boundary changes consistency reasoning and future extension cost"],
                "source_ids":[repository_source],
                "effect_categories":["maintenance_or_support","implementation_internal"],
                "relationship":{"state":"independent"},
                "evidence_state":"sufficient"
            }],
            "material_boundary_review":complete_material_boundary_review_json(repository_source, &[
                ("maintenance_or_support", &["cache-invalidation-boundary"]),
                ("implementation_internal", &["cache-invalidation-boundary"]),
            ])
        }),
    ))
    .clone();
    let discovery_id = discovery["discovery_candidate_id"]
        .as_str()
        .expect("discovery identity");
    assert_eq!(discovery["workflow"]["stage"], "materiality_review");
    assert_eq!(
        discovery["workflow"]["input_guidance"]["draft_call"]["action"],
        "draft"
    );

    let draft = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"draft",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
        }),
    ))
    .clone();
    assert_eq!(draft["goal_context_id"], goal_context_id);
    assert_eq!(
        draft["judgment_templates"][0]["discovery_owned"]["choice_id"],
        "cache-invalidation-boundary"
    );
    assert_eq!(
        draft["authority_learning_routing"]["assessment_owner"],
        "active_agent"
    );
    assert!(
        draft["authority_learning_routing"]["learning_requests_not_user_ownership"]
            .as_array()
            .expect("learning requests")
            .iter()
            .any(|request| request == "ask_to_select_an_implementation_approach_for_learning")
    );
    assert_eq!(
        draft["authority_learning_routing"]["routes"][1]["required_path"],
        "learning_deliberation"
    );
    assert_eq!(
        draft["authority_learning_routing"]["routes"][1]["canonical_decision"],
        false
    );

    let review = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
            "rationale":"The cache boundary is agent-owned but worth reasoning through.",
            "learning_participation":{"state":"active","user_turn_source_id":goal_source_id,"verbatim_statement":"Teach me through meaningful technical choices"},
            "judgments":[{
                "choice_id":"cache-invalidation-boundary",
                "disposition":"agent_owned_implementation_choice",
                "basis_summary":"No user-owned outcome changes; implementation authority remains with the agent.",
                "authority_counterfactual":"The alternatives vary an internal consistency mechanism, not a user-owned material policy.",
                "materially_varying_outcomes":["the private cache invalidation mechanism"],
                "contains_user_owned_outcome":false,
                "user_owned_outcomes":[],
                "ownership_rationale":"all alternatives preserve the settled observable cache behavior",
                "bounded_implementation_discretion_rationale":"the choice changes only the private consistency mechanism",
                "ownership_source_ids":[analyzed["repository_source_id"]],
                "learning_value":{"state":"deliberation_worthy","rationale":"The consistency boundary illustrates a reusable design principle.","consequence_significance":["Missed invalidation can serve stale data"],"transferable_principles":["Centralize invariants when mutation sites multiply"],"non_obvious_trade_offs":["Local simplicity can create distributed correctness obligations"],"interruption_counterfactual":"Without participation, the requested understanding of consistency ownership would be lost.","participation_scope_alignment":"The Goal explicitly requests learning through meaningful technical boundaries."}
            }]
        }),
    ))
    .clone();
    let review_id = review["review_candidate_id"]
        .as_str()
        .expect("review identity");
    assert_eq!(review["workflow"]["stage"], "learning_deliberation");
    assert_eq!(
        review["workflow"]["input_guidance"]["interaction_kind"],
        "learning_participation_not_canonical_decision"
    );
    assert_eq!(
        review["workflow"]["input_guidance"]["learning_selection_contract"]["canonical_decision"],
        false
    );
    assert_eq!(
        review["workflow"]["input_guidance"]["learning_selection_contract"]
            ["forbidden_substitute_operations"],
        json!([
            "candidate_manage.submit_question_from_materiality",
            "decision_record"
        ])
    );
    let unsupported_downgrade = call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"revise",
            "project_id":project,
            "review_candidate_id":review_id,
            "rationale":"A user Goal Source cannot be relabeled as research evidence to bypass deliberation.",
            "learning_participation":{"state":"active","user_turn_source_id":goal_source_id,"verbatim_statement":"Teach me through meaningful technical choices"},
            "judgments":[{
                "choice_id":"cache-invalidation-boundary",
                "disposition":"agent_owned_implementation_choice",
                "basis_summary":"The implementation authority remains agent-owned.",
                "authority_counterfactual":"The internal mechanism remains materially equivalent from the user's product-policy perspective.",
                "materially_varying_outcomes":["the private cache invalidation mechanism"],
                "contains_user_owned_outcome":false,
                "user_owned_outcomes":[],
                "ownership_rationale":"all alternatives preserve the settled observable cache behavior",
                "bounded_implementation_discretion_rationale":"the choice changes only the private consistency mechanism",
                "ownership_source_ids":[analyzed["repository_source_id"]],
                "learning_value":{"state":"routine","rationale":"Unsupported downgrade without repository or prototype evidence."}
            }],
            "learning_value_revision_bases":[{
                "dimension_id":"cache-invalidation-boundary",
                "kind":"research_evidence",
                "source_ids":[goal_source_id],
                "evidence_basis":["The original user request is not repository research."],
                "rationale":"This deliberately invalid basis must not bypass Learning Deliberation."
            }]
        }),
    );
    assert_eq!(
        unsupported_downgrade["result"]["isError"], true,
        "{unsupported_downgrade}"
    );
    assert!(structured(&unsupported_downgrade)["error"]
        .as_str()
        .is_some_and(|error| error.contains("current non-user evidence Sources")));
    let after_rejected_downgrade = structured(&call(
        &mut adapter,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    let preserved_review = after_rejected_downgrade["candidates"]
        .as_array()
        .expect("Candidate array")
        .iter()
        .find(|candidate| candidate["identity"] == review_id)
        .expect("preserved Materiality Review");
    assert_eq!(
        preserved_review["materiality_review"]["dimensions"][0]["learning_value"]["state"],
        "deliberation_worthy"
    );
    let before_deliberation = structured(&call(
        &mut adapter,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    assert!(!before_deliberation["candidates"]
        .as_array()
        .expect("Candidate array")
        .iter()
        .any(|candidate| candidate["kind"] == "question"));

    let begun = structured(&call(
        &mut adapter,
        "learning_deliberation",
        json!({
            "action":"begin",
            "project_id":project,
            "review_candidate_id":review_id,
            "dimension_id":"cache-invalidation-boundary",
            "source_operation":"installed MCP pre-work deliberation",
            "problem":"Which invalidation boundary makes the consistency invariant easiest to preserve?",
            "established_facts":["Mutation sites can grow independently of cache readers"]
        }),
    ))
    .clone();
    let deliberation_id = begun["deliberation_candidate_id"]
        .as_str()
        .expect("Learning Deliberation identity");
    assert_eq!(begun["state"]["state"], "awaiting_initial_response");
    assert_eq!(begun["canonical_decision"], false);
    assert_eq!(begun["rounds"], json!([]));
    assert!(begun.get("agent_recommendation").is_none());

    let premature_feedback = structured(&call(
        &mut adapter,
        "learning_deliberation",
        json!({
            "action":"feedback",
            "project_id":project,
            "deliberation_candidate_id":deliberation_id,
            "feedback":"Premature feedback must fail",
            "recommendation_selections":[{"choice_id":"cache-invalidation-boundary","alternative_id":"versioned-facade"}],
            "recommendation_rationale":"This must not be recorded before user reasoning."
        }),
    ))
    .clone();
    assert!(premature_feedback["error"]
        .as_str()
        .is_some_and(|error| error.contains("feedback")));

    let responded = structured(&call(
        &mut adapter,
        "learning_deliberation",
        json!({
            "action":"respond_select",
            "project_id":project,
            "deliberation_candidate_id":deliberation_id,
            "user_turn":"I choose the versioned facade because one explicit invariant is easier to audit.",
            "user_rationale":"One explicit invariant is easier to audit.",
            "selections":[{"choice_id":"cache-invalidation-boundary","alternative_id":"versioned-facade"}]
        }),
    ))
    .clone();
    assert_eq!(responded["state"]["state"], "awaiting_agent_feedback");
    assert_eq!(
        responded["rounds"][0]["user_rationale"],
        "One explicit invariant is easier to audit."
    );

    let feedback = structured(&call(
        &mut adapter,
        "learning_deliberation",
        json!({
            "action":"feedback",
            "project_id":project,
            "deliberation_candidate_id":deliberation_id,
            "feedback":"That choice centralizes the invariant; the cost is version bookkeeping and a required facade for every cache access.",
            "recommendation_selections":[{"choice_id":"cache-invalidation-boundary","alternative_id":"versioned-facade"}],
            "recommendation_rationale":"The repository has multiple mutation paths, so centralizing invalidation reduces omission risk."
        }),
    ))
    .clone();
    assert_eq!(feedback["state"]["state"], "feedback_provided");
    assert!(feedback["rounds"][0]["agent_recommendation"].is_object());

    let completed = structured(&call(
        &mut adapter,
        "learning_deliberation",
        json!({"action":"complete","project_id":project,"deliberation_candidate_id":deliberation_id}),
    ))
    .clone();
    assert_eq!(completed["state"]["state"], "completed");
    assert_eq!(completed["workflow"]["stage"], "materiality_review");
    let bound = structured(&bind_recorded_scope(&mut adapter, &review, &["src"])).clone();
    assert_eq!(bound["workflow"]["stage"], "ready_for_work");

    let runtime = RuntimeLayout::new(temporary.path().join("runtime")).expect("restart runtime");
    let mut restarted = HostAdapter::new(LocalOperations::new(runtime));
    let inspected = structured(&call(
        &mut restarted,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    let preserved = inspected["candidates"]
        .as_array()
        .expect("Candidate array")
        .iter()
        .find(|candidate| candidate["identity"] == deliberation_id)
        .expect("preserved Learning Deliberation");
    assert_eq!(
        preserved["learning_deliberation"]["state"]["state"],
        "completed"
    );
    let canonical = structured(&call(
        &mut restarted,
        "canonical_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    assert!(!canonical["records"]
        .as_array()
        .expect("canonical records")
        .iter()
        .any(|record| record["kind"] == "decision"));
    let recalled = structured(&call(
        &mut restarted,
        "recall",
        json!({"project_id":project}),
    ))
    .clone();
    assert_eq!(
        recalled["learning_context"][0]["learning_deliberation"]["state"]["state"],
        "completed"
    );
}

#[test]
fn aggregate_schema_diagnostic_reports_independent_discovery_problems() {
    let (_temporary, mut adapter, project) = setup();
    let invalid = structured(&call(
        &mut adapter,
        "engineering_choice_discovery",
        json!({"project_id":project,"unexpected":true}),
    ))
    .clone();
    assert_eq!(
        invalid["details"]["diagnostic"],
        "aggregate_schema_validation"
    );
    let problems = invalid["details"]["problems"]
        .as_array()
        .expect("aggregate problems");
    assert!(problems.len() >= 5, "{invalid}");
    assert!(problems
        .iter()
        .any(|problem| problem == "arguments.goal_context_id is required"));
    assert!(problems
        .iter()
        .any(|problem| problem == "arguments.choices is required"));
    assert!(problems
        .iter()
        .any(|problem| problem == "arguments.material_boundary_review is required"));
    assert!(problems
        .iter()
        .any(|problem| problem == "arguments.unexpected is not allowed"));
}

#[test]
fn active_learning_respects_non_interruption_for_routine_wording_and_tests() {
    let (_temporary, mut adapter, project) = setup();
    let user_turn = "Teach me meaningful architecture and flow choices, but do not interrupt me for routine wording or test synchronization details";
    let goal = structured(&call(
        &mut adapter,
        "context_record",
        json!({"project_id":project,"user_turn":user_turn,"role":"goal","statement":user_turn}),
    ))
    .clone();
    let analyzed = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    let discovery = structured(&call(
        &mut adapter,
        "engineering_choice_discovery",
        json!({
            "project_id":project,
            "goal_context_id":goal["context_item_id"],
            "baseline_analysis_snapshot_id":analyzed["analysis_snapshot_id"],
            "source_operation":"routine wording and test synchronization control",
            "summary":"Synchronize a maintenance diagnostic and its exact test assertion",
            "choices":[{
                "choice_id":"diagnostic-test-wording",
                "summary":"Update the diagnostic first or update the fixture assertion first",
                "affected_scope":["private diagnostic wording","test fixture assertion"],
                "alternatives":[
                    {"alternative_id":"wording-first","summary":"Change wording before synchronizing the assertion","technical_consequences":["The test is briefly stale during the edit"]},
                    {"alternative_id":"test-first","summary":"Change the assertion before synchronizing the wording","technical_consequences":["The test briefly anticipates the maintenance wording"]}
                ],
                "technical_consequences":["Only the order of a small synchronized maintenance edit differs"],
                "source_ids":[analyzed["repository_source_id"]],
                "effect_categories":["implementation_internal"],
                "relationship":{"state":"independent"},
                "evidence_state":"sufficient"
            }],
            "material_boundary_review":complete_material_boundary_review_json(
                analyzed["repository_source_id"].as_str().expect("repository Source"),
                &[("implementation_internal", &["diagnostic-test-wording"])],
            )
        }),
    ))
    .clone();
    let review = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery["discovery_candidate_id"],
            "rationale":"The wording and test synchronization detail is agent-owned and routine within the user's explicit non-interruption boundary.",
            "learning_participation":{"state":"active","user_turn_source_id":goal["source_id"],"verbatim_statement":user_turn},
            "judgments":[{
                "choice_id":"diagnostic-test-wording",
                "disposition":"agent_owned_implementation_choice",
                "basis_summary":"This is internal agent-owned discretion.",
                "authority_counterfactual":"The synchronized edit order changes no material product outcome, so the detail remains agent-owned.",
                "materially_varying_outcomes":["the order of a private synchronized maintenance edit"],
                "contains_user_owned_outcome":false,
                "user_owned_outcomes":[],
                "ownership_rationale":"the alternatives are mechanically equivalent to users",
                "bounded_implementation_discretion_rationale":"both orders produce the same public wording and passing test",
                "ownership_source_ids":[analyzed["repository_source_id"]],
                "learning_value":{"state":"routine","rationale":"No meaningful transferable understanding would be lost, and the user explicitly excluded routine wording and test synchronization from interruptions."}
            }]
        }),
    ))
    .clone();
    assert_eq!(review["workflow"]["stage"], "materiality_review");
    let bound = structured(&bind_recorded_scope(&mut adapter, &review, &["src"])).clone();
    assert_eq!(bound["workflow"]["stage"], "ready_for_work");
    assert_eq!(bound["workflow"]["blocks_ordinary_work"], false);
    let inspected = structured(&call(
        &mut adapter,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    assert!(!inspected["candidates"]
        .as_array()
        .expect("Candidate array")
        .iter()
        .any(|candidate| candidate["learning_deliberation"].is_object()));
}

#[test]
fn active_learning_on_current_task_delegation_uses_non_decision_deliberation() {
    let (_temporary, mut adapter, project) = setup();
    let goal_turn =
        "Teach me through the trade-offs; choose the internal retry schedule for this task.";
    let goal = structured(&call(
        &mut adapter,
        "context_record",
        json!({"project_id":project,"user_turn":goal_turn,"role":"goal","statement":goal_turn}),
    ))
    .clone();
    let analyzed = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    let discovery_id = record_fixture_discovery(
        &adapter,
        &project,
        goal["context_item_id"].as_str().expect("Goal identity"),
        analyzed["analysis_snapshot_id"]
            .as_str()
            .expect("baseline identity"),
        analyzed["repository_source_id"]
            .as_str()
            .expect("repository Source identity"),
        FixtureEngineeringChoice {
            id: "retry-schedule",
            affected_scope: "retry-jitter",
            effect_category: EngineeringEffectCategory::ImplementationInternal,
        },
    );
    let review = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
            "rationale":"The current Goal delegates the bounded internal schedule while retaining active learning participation.",
            "learning_participation":{"state":"active","user_turn_source_id":goal["source_id"],"verbatim_statement":"Teach me through the trade-offs"},
            "judgments":[{
                "choice_id":"retry-schedule",
                "disposition":"delegated_implementation_choice",
                "basis_summary":"Exact current-task delegation covers the bounded internal retry schedule.",
                "authority_counterfactual":"The Goal explicitly says to choose this exact internal schedule; it does not rely on the broader feature request.",
                "materially_varying_outcomes":["the delegated retry scheduling behavior"],
                "contains_user_owned_outcome":true,
                "user_owned_outcomes":["selection of the explicitly delegated retry schedule"],
                "ownership_rationale":"the current user owns and explicitly delegates this bounded selection",
                "ownership_source_ids":[analyzed["repository_source_id"]],
                "delegation_statement":"choose the internal retry schedule",
                "delegated_scope":["retry-jitter"],
                "learning_value":{"state":"deliberation_worthy","rationale":"Retry scheduling exposes reusable load-shaping trade-offs.","consequence_significance":["Synchronized retries can amplify transient load"],"transferable_principles":["Randomization can decorrelate distributed work"],"non_obvious_trade_offs":["More jitter reduces synchronization but broadens completion latency"],"interruption_counterfactual":"Without participation, the requested understanding of retry load shaping would be lost.","participation_scope_alignment":"The active learning scope includes meaningful distributed-operability choices."}
            }]
        }),
    ))
    .clone();
    assert_eq!(review["workflow"]["stage"], "learning_deliberation");
    assert_eq!(
        review["workflow"]["input_guidance"]["learning_selection_contract"]["authority"],
        "agent_owned_or_explicitly_delegated"
    );
    let review_id = review["review_candidate_id"]
        .as_str()
        .expect("review identity");
    let begun = structured(&call(
        &mut adapter,
        "learning_deliberation",
        json!({
            "action":"begin",
            "project_id":project,
            "review_candidate_id":review_id,
            "dimension_id":"retry-schedule",
            "source_operation":"delegated learning control",
            "problem":"Which retry schedule best balances load spreading and completion latency?",
            "established_facts":["The choice is internal and explicitly delegated for this task"]
        }),
    ))
    .clone();
    let deliberation_id = begun["deliberation_candidate_id"]
        .as_str()
        .expect("deliberation identity");
    let responded = structured(&call(
        &mut adapter,
        "learning_deliberation",
        json!({
            "action":"respond_select",
            "project_id":project,
            "deliberation_candidate_id":deliberation_id,
            "user_turn":"I select the first approach after comparing the load-shaping consequence.",
            "user_rationale":"It keeps the bounded implementation easy to inspect.",
            "selections":[{"choice_id":"retry-schedule","alternative_id":"first"}]
        }),
    ))
    .clone();
    assert_eq!(responded["state"]["state"], "awaiting_agent_feedback");
    let feedback = structured(&call(
        &mut adapter,
        "learning_deliberation",
        json!({
            "action":"feedback",
            "project_id":project,
            "deliberation_candidate_id":deliberation_id,
            "feedback":"The selected bounded schedule is easy to inspect; its load-spreading limit remains explicit.",
            "recommendation_selections":[{"choice_id":"retry-schedule","alternative_id":"first"}],
            "recommendation_rationale":"The bounded implementation favors inspectability for this scope."
        }),
    ))
    .clone();
    assert_eq!(feedback["state"]["state"], "feedback_provided");
    let completed = structured(&call(
        &mut adapter,
        "learning_deliberation",
        json!({"action":"complete","project_id":project,"deliberation_candidate_id":deliberation_id}),
    ))
    .clone();
    assert_eq!(completed["workflow"]["stage"], "materiality_review");
    let bound = structured(&bind_recorded_scope(&mut adapter, &review, &["src"])).clone();
    assert_eq!(bound["workflow"]["stage"], "ready_for_work");

    let canonical = structured(&call(
        &mut adapter,
        "canonical_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    assert!(!canonical["records"]
        .as_array()
        .expect("canonical records")
        .iter()
        .any(|record| record["kind"] == "decision"));
    let candidates = structured(&call(
        &mut adapter,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    assert!(!candidates["candidates"]
        .as_array()
        .expect("Candidate array")
        .iter()
        .any(|candidate| candidate["kind"] == "question"));
}

#[test]
fn exact_current_task_delegation_can_cover_one_material_outcome() {
    let (temporary, mut adapter, project) = setup();
    let goal_turn =
        "Add npm update availability; choose whether update checks are automatic or opt-in.";
    let goal = structured(&call(
        &mut adapter,
        "context_record",
        json!({"project_id":project,"user_turn":goal_turn,"role":"goal","statement":goal_turn}),
    ))
    .clone();
    let analyzed = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    let discovery_id = record_fixture_discovery(
        &adapter,
        &project,
        goal["context_item_id"].as_str().expect("Goal identity"),
        analyzed["analysis_snapshot_id"]
            .as_str()
            .expect("baseline identity"),
        analyzed["repository_source_id"]
            .as_str()
            .expect("repository Source identity"),
        FixtureEngineeringChoice {
            id: "npm-update-activation",
            affected_scope: "npm update activation policy",
            effect_category: EngineeringEffectCategory::UserVisibleBehaviorOrDefault,
        },
    );
    let review = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
            "rationale":"The current Goal explicitly delegates this one material activation outcome.",
            "learning_participation":{"state":"inactive"},
            "judgments":[{
                "choice_id":"npm-update-activation",
                "disposition":"delegated_implementation_choice",
                "basis_summary":"Exact current-task delegation covers this material activation policy.",
                "authority_counterfactual":"Automatic and opt-in checks are materially different, but the current-host statement explicitly delegates the exact choice between those outcomes.",
                "materially_varying_outcomes":["whether update checks activate automatically or only by opt-in"],
                "contains_user_owned_outcome":true,
                "user_owned_outcomes":["the update-check activation policy"],
                "ownership_rationale":"the alternatives change a user-visible default delegated by the user",
                "ownership_source_ids":[analyzed["repository_source_id"]],
                "delegation_statement":"choose whether update checks are automatic or opt-in",
                "delegated_scope":["npm update activation policy"],
                "learning_value":{"state":"routine","rationale":"Learning participation is inactive and does not alter authority."}
            }]
        }),
    ))
    .clone();

    assert_eq!(review["workflow"]["stage"], "materiality_review");
    let bound = structured(&bind_recorded_scope(&mut adapter, &review, &["src"])).clone();
    assert_eq!(bound["workflow"]["stage"], "ready_for_work");
    assert_eq!(bound["workflow"]["blocks_ordinary_work"], false);

    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("docs")).expect("docs directory");
    fs::create_dir_all(repository.join("tests")).expect("tests directory");
    fs::write(repository.join("docs/z.md"), "# uncovered\n").expect("docs change");
    fs::write(repository.join("tests/a.rs"), "#[test] fn uncovered() {}\n").expect("test change");
    let rejected = call(
        &mut adapter,
        "checkpoint_record",
        json!({
            "project_id":project,
            "goal_context_id":review["goal_context_id"],
            "baseline_analysis_snapshot_id":review["baseline_analysis_snapshot_id"],
            "kind":"handoff",
            "work_state":"paused",
            "applied_decision_ids":[],
            "decision_components":["transport-core","release-core"],
            "work_contexts":["transport","release"],
            "verification":[{"state":"not_run"}],
            "next_step":"Correct the bounded executable scope",
            "handoff_to":"next session"
        }),
    );
    assert_eq!(rejected["result"]["isError"], true, "{rejected}");
    let details = &structured(&rejected)["details"];
    assert_eq!(
        details["scope_violations"]["uncovered_paths"],
        json!(["docs/z.md", "tests/a.rs"])
    );
    assert_eq!(
        details["scope_violations"]["uncovered_components"],
        json!(["release-core", "transport-core"])
    );
    assert_eq!(
        details["scope_violations"]["uncovered_work_contexts"],
        json!(["release", "transport"])
    );
    assert_eq!(
        details["scope_violations"]["executable_scope"]["paths"],
        json!(["src"])
    );
    assert_eq!(
        details["scope_violations"]["required_next_action"],
        json!({"tool":"materiality_review","action":"inspect"})
    );
    assert_eq!(details["workflow"]["disposition"], "review_invalid");
}

#[test]
fn active_learning_keeps_exploratory_uncertainty_on_the_research_path() {
    let (_temporary, mut adapter, project) = setup();
    let goal_turn = "Teach me while you investigate the bounded retry behavior.";
    let goal = structured(&call(
        &mut adapter,
        "context_record",
        json!({"project_id":project,"user_turn":goal_turn,"role":"goal","statement":goal_turn}),
    ))
    .clone();
    let analyzed = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    let discovery_id = record_fixture_discovery(
        &adapter,
        &project,
        goal["context_item_id"].as_str().expect("Goal identity"),
        analyzed["analysis_snapshot_id"]
            .as_str()
            .expect("baseline identity"),
        analyzed["repository_source_id"]
            .as_str()
            .expect("repository Source identity"),
        FixtureEngineeringChoice {
            id: "retry-observation",
            affected_scope: "retry-runtime",
            effect_category: EngineeringEffectCategory::ImplementationInternal,
        },
    );
    let review = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
            "rationale":"Repository evidence is still required; learning participation does not settle that uncertainty.",
            "learning_participation":{"state":"active","user_turn_source_id":goal["source_id"],"verbatim_statement":"Teach me while you investigate"},
            "judgments":[{
                "choice_id":"retry-observation",
                "disposition":"exploratory_uncertainty",
                "exploratory_disposition":"research_required",
                "basis_summary":"Measure current runtime behavior before choosing an approach.",
                "authority_counterfactual":"Empirical retry behavior must be established before any remaining material authority choice is known.",
                "materially_varying_outcomes":["the private retry observation mechanism under investigation"],
                "contains_user_owned_outcome":false,
                "user_owned_outcomes":[],
                "ownership_rationale":"current evidence identifies an implementation uncertainty, not a product-policy selection",
                "bounded_implementation_discretion_rationale":"the research precedes any selection and all observed mechanisms remain inside settled behavior",
                "ownership_source_ids":[analyzed["repository_source_id"]],
                "research_basis":["Inspect retained runtime observations for retry correlation"],
                "learning_value":{"state":"deliberation_worthy","rationale":"The evidence can illustrate retry correlation.","consequence_significance":["Correlated retries can amplify load"],"transferable_principles":["Observe uncertain behavior before selecting policy"],"non_obvious_trade_offs":["Extra observation delays implementation but avoids a speculative choice"],"interruption_counterfactual":"Without participation, the requested understanding of evidence-first retry design would be lost.","participation_scope_alignment":"The active learning scope includes meaningful operability evidence choices."}
            }]
        }),
    ))
    .clone();
    assert_eq!(review["workflow"]["stage"], "research_or_prototype");
    assert_eq!(review["workflow"]["disposition"], "research_required");
    let candidates = structured(&call(
        &mut adapter,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    assert!(!candidates["candidates"]
        .as_array()
        .expect("Candidate array")
        .iter()
        .any(|candidate| candidate["learning_deliberation"].is_object()
            || candidate["kind"] == "question"));
    let canonical = structured(&call(
        &mut adapter,
        "canonical_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    assert!(!canonical["records"]
        .as_array()
        .expect("canonical records")
        .iter()
        .any(|record| record["kind"] == "decision"));
}

#[test]
fn mcp_preserves_bounded_verbatim_current_task_delegation_for_inspection() {
    let (temporary, mut adapter, project) = setup();
    let goal_turn = "Implement the change; choose the internal module name.";
    let goal = structured(&call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":goal_turn,
            "role":"goal",
            "statement":goal_turn,
        }),
    ))
    .clone();
    let goal_context_id = goal["context_item_id"].as_str().expect("Goal identity");
    let goal_source_id = goal["source_id"].as_str().expect("Goal Source identity");
    let analyzed = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    let discovery_id = record_fixture_discovery(
        &adapter,
        &project,
        goal_context_id,
        analyzed["analysis_snapshot_id"]
            .as_str()
            .expect("baseline Analysis Snapshot"),
        goal_source_id,
        FixtureEngineeringChoice {
            id: "internal-module-name",
            affected_scope: "src/lib.rs",
            effect_category: EngineeringEffectCategory::ImplementationInternal,
        },
    );
    let review = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":discovery_id,
            "rationale":"The exact Goal delegates only the internal name.",
            "learning_participation":{"state":"inactive"},
            "judgments":[{
                "choice_id":"internal-module-name",
                "disposition":"delegated_implementation_choice",
                "basis_summary":"Exact bounded current-task delegation",
                "authority_counterfactual":"The Goal explicitly delegates the exact internal module name, not merely the encompassing change.",
                "materially_varying_outcomes":["the explicitly delegated internal module name"],
                "contains_user_owned_outcome":true,
                "user_owned_outcomes":["selection of the delegated internal name"],
                "ownership_rationale":"the user explicitly retained and delegated this bounded choice",
                "ownership_source_ids":[analyzed["repository_source_id"]],
                "learning_value":{"state":"routine","rationale":"An internal module name is routine."},
                "delegation_statement":"choose the internal module name",
                "delegated_scope":["src/lib.rs"]
            }]
        }),
    ))
    .clone();
    assert!(review["review_candidate_id"].is_string(), "{review}");

    let inspected = structured(&call(
        &mut adapter,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    let evidence = inspected["candidates"]
        .as_array()
        .expect("Candidate array")
        .iter()
        .filter_map(|candidate| candidate["explicit_delegation_evidence"].as_array())
        .flatten()
        .next()
        .expect("delegation evidence");
    assert_eq!(evidence["dimension_id"], "internal-module-name");
    assert_eq!(evidence["bound_dimension_id"], "internal-module-name");
    assert_eq!(evidence["discovered_choice_ids"][0], "internal-module-name");
    assert_eq!(evidence["goal_context_id"], goal_context_id);
    assert_eq!(evidence["user_turn_source_id"], goal_source_id);
    assert_eq!(
        evidence["verbatim_statement"],
        "choose the internal module name"
    );
    assert_eq!(
        evidence["authority_kind"],
        "explicit_current_task_delegation"
    );
    assert!(evidence["semantic_rationale"]
        .as_str()
        .is_some_and(|rationale| rationale.contains("exact internal module name")));
    assert_eq!(evidence["effect_categories"][0], "implementation_internal");
    assert!(evidence["material_consequences"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    assert!(inspected
        .to_string()
        .contains("choose the internal module name"));
    assert!(!inspected.to_string().contains(goal_turn));

    fs::create_dir_all(temporary.path().join("repository/src")).expect("resume source directory");
    fs::write(
        temporary.path().join("repository/src/lib.rs"),
        "mod chosen_name {}\n",
    )
    .expect("prior bounded work");
    let layout = adapter.operations().layout().clone();
    let mut adapter = HostAdapter::new(LocalOperations::new(layout));
    let resumed = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    assert_eq!(resumed["workflow"]["stage"], "engineering_choice_discovery");
    let no_new = &resumed["workflow"]["input_guidance"]["when_no_new_material_choice"];
    assert_eq!(no_new["empty_discovery_submission"]["valid"], false);
    assert_eq!(no_new["inspect_previous"]["tool"], "candidate_inspect");
    assert!(no_new["continued_work_path"]
        .as_array()
        .is_some_and(|steps| steps.iter().any(|step| step
            .as_str()
            .is_some_and(|text| text.contains("same stable choice identities")))));

    let resumed_discovery_id = record_fixture_discovery(
        &adapter,
        &project,
        goal_context_id,
        resumed["analysis_snapshot_id"]
            .as_str()
            .expect("resume baseline Analysis Snapshot"),
        resumed["repository_source_id"]
            .as_str()
            .expect("resume repository Source"),
        FixtureEngineeringChoice {
            id: "internal-module-name",
            affected_scope: "src/lib.rs",
            effect_category: EngineeringEffectCategory::ImplementationInternal,
        },
    );
    let resumed_draft = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"draft",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":resumed_discovery_id,
        }),
    ))
    .clone();
    let reusable = resumed_draft["current_goal_authority_inputs"]
        .as_array()
        .expect("resume current Goal authority inputs")
        .first()
        .expect("resume delegation candidate");
    assert_eq!(reusable["goal_context_id"], goal_context_id);
    assert_eq!(reusable["user_turn_source_id"], goal_source_id);
    assert_eq!(reusable["dimension_id"], "internal-module-name");
    assert_eq!(reusable["affected_scope"], json!(["src/lib.rs"]));
    let resumed_review = structured(&call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "engineering_choice_discovery_candidate_id":resumed_discovery_id,
            "rationale":"The retained choice remains exactly delegated on the fresh baseline.",
            "learning_participation":{"state":"inactive"},
            "judgments":[{
                "choice_id":"internal-module-name",
                "disposition":"delegated_implementation_choice",
                "basis_summary":"Re-evaluated exact bounded current-task delegation",
                "authority_counterfactual":"The retained verbatim statement still explicitly delegates this exact internal module-name dimension.",
                "materially_varying_outcomes":["the explicitly delegated internal module name"],
                "contains_user_owned_outcome":true,
                "user_owned_outcomes":["selection of the delegated internal name"],
                "ownership_rationale":"the user explicitly retained and delegated this bounded choice",
                "ownership_source_ids":[resumed["repository_source_id"]],
                "learning_value":{"state":"routine","rationale":"The retained internal module choice remains routine."},
                "delegation_statement":"choose the internal module name",
                "delegated_scope":["src/lib.rs"]
            }]
        }),
    ))
    .clone();
    let rebound = structured(&bind_recorded_scope(
        &mut adapter,
        &resumed_review,
        &["src/lib.rs"],
    ))
    .clone();
    assert_eq!(rebound["workflow"]["stage"], "ready_for_work");
    assert_eq!(rebound["workflow"]["blocks_ordinary_work"], false);
}

#[test]
fn checkpoint_refusal_returns_bounded_actionable_workflow_guidance() {
    let (_temporary, mut adapter, project) = setup();
    let goal = call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":"Pause after checking work authority",
            "role":"goal",
            "statement":"Pause after checking work authority",
        }),
    );
    let goal_id = structured(&goal)["context_item_id"]
        .as_str()
        .expect("Goal identity");
    let baseline = call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    );
    let baseline_id = structured(&baseline)["analysis_snapshot_id"]
        .as_str()
        .expect("baseline identity");
    let refused = call(
        &mut adapter,
        "checkpoint_record",
        json!({
            "project_id":project,
            "goal_context_id":goal_id,
            "baseline_analysis_snapshot_id":baseline_id,
            "kind":"pause",
            "work_state":"paused",
            "applied_decision_ids":[],
            "verification":[{"state":"not_run"}],
            "next_step":"Record the required Materiality Review",
        }),
    );
    assert_eq!(refused["result"]["isError"], true, "{refused}");
    let refused = structured(&refused);
    assert!(
        refused["error"]
            .as_str()
            .expect("error text")
            .contains("work authority is not resolved"),
        "{refused}"
    );
    assert_eq!(
        refused["details"]["workflow"]["stage"],
        "engineering_choice_discovery"
    );
    assert_eq!(
        refused["details"]["workflow"]["disposition"],
        "engineering_choice_discovery_required"
    );
    assert_eq!(
        refused["details"]["workflow"]["required_next_action"],
        json!({"tool":"engineering_choice_discovery","action":"record"})
    );
    assert_eq!(refused["details"]["workflow"]["blocks_ordinary_work"], true);
}

#[test]
fn concurrent_mcp_writers_preserve_every_committed_context() {
    const WRITERS: usize = 8;
    let (_temporary, adapter, project) = setup();
    let runtime = adapter.operations().layout().root().to_path_buf();
    drop(adapter);
    let barrier = Arc::new(Barrier::new(WRITERS));
    let mut writers = Vec::new();
    for index in 0..WRITERS {
        let runtime = runtime.clone();
        let project = project.clone();
        let barrier = Arc::clone(&barrier);
        writers.push(thread::spawn(move || {
            let operations = LocalOperations::new(RuntimeLayout::new(runtime).expect("runtime"));
            let mut adapter = HostAdapter::new(operations);
            let statement = format!("bounded concurrent MCP context {index}");
            barrier.wait();
            let response = call(
                &mut adapter,
                "context_record",
                json!({
                    "project_id":project,
                    "user_turn":statement,
                    "role":"goal",
                    "statement":statement,
                }),
            );
            assert_eq!(response["result"]["isError"], false, "{response}");
            structured(&response)["context_item_id"]
                .as_str()
                .expect("Context identity")
                .to_owned()
        }));
    }
    let identities = writers
        .into_iter()
        .map(|writer| writer.join().expect("MCP writer"))
        .collect::<BTreeSet<_>>();
    assert_eq!(identities.len(), WRITERS);
    let restarted = LocalOperations::new(RuntimeLayout::new(runtime).expect("runtime"));
    let canonical = restarted
        .canonical_basis(parse_project(&project))
        .expect("canonical after concurrent MCP writes");
    let persisted = canonical
        .context_items
        .iter()
        .map(|item| item.id.to_string())
        .collect::<BTreeSet<_>>();
    assert!(identities
        .iter()
        .all(|identity| persisted.contains(identity)));
}

#[test]
fn candidate_inspection_distinguishes_empty_unavailable_corrupt_and_unsupported_dependencies() {
    let (_healthy_root, mut healthy, healthy_project) = setup();
    let healthy_response = call(
        &mut healthy,
        "candidate_inspect",
        json!({"project_id":healthy_project}),
    );
    let healthy_result = structured(&healthy_response);
    assert_eq!(healthy_result["health"], "available");
    assert_eq!(healthy_result["candidates"], json!([]));
    assert_eq!(healthy_result["issues"], json!([]));

    let (_unsupported_root, mut unsupported, unsupported_project) = setup();
    Connection::open(unsupported.operations().layout().candidate_store())
        .expect("open Candidate store")
        .execute(
            "UPDATE metadata SET value = '999' WHERE key = 'schema_version'",
            [],
        )
        .expect("set unsupported Candidate schema");
    assert_candidate_mcp_dependency(&mut unsupported, &unsupported_project, "unsupported");

    let (_corrupt_root, mut corrupt, corrupt_project) = setup();
    Connection::open(corrupt.operations().layout().candidate_store())
        .expect("open Candidate store")
        .execute("DROP TABLE candidates", [])
        .expect("remove required Candidate table");
    assert_candidate_mcp_dependency(&mut corrupt, &corrupt_project, "corrupt");

    let (_unavailable_root, mut unavailable, unavailable_project) = setup();
    let candidate_path = unavailable.operations().layout().candidate_store();
    fs::remove_file(&candidate_path).expect("remove Candidate store");
    fs::create_dir(&candidate_path).expect("replace Candidate store with unavailable path");
    assert_candidate_mcp_dependency(&mut unavailable, &unavailable_project, "unavailable");
}

fn assert_candidate_mcp_dependency(adapter: &mut HostAdapter, project: &str, expected: &str) {
    let response = call(adapter, "candidate_inspect", json!({"project_id":project}));
    assert_eq!(response["result"]["isError"], false, "{response}");
    let result = structured(&response);
    assert_eq!(result["health"], expected, "{result}");
    assert_eq!(result["candidates"], json!([]), "{result}");
    assert!(
        result["issues"]
            .as_array()
            .is_some_and(|issues| issues.iter().any(|issue| {
                issue["scope"] == "candidate_inspection"
                    && issue["kind"]
                        .as_str()
                        .is_some_and(|kind| kind.contains(expected))
            })),
        "{result}"
    );

    let understanding_response = call(
        adapter,
        "repository_understanding",
        json!({"project_id":project}),
    );
    let understanding = structured(&understanding_response);
    assert_eq!(understanding["health"], "degraded", "{understanding}");
    assert_eq!(understanding["candidate_dependency"], expected);

    let preview_response = call(
        adapter,
        "document_preview",
        json!({
            "project_id":project,
            "kind":"handoff-resume",
            "format":"markdown",
            "language":"en",
            "locale":"en"
        }),
    );
    let preview = structured(&preview_response);
    assert!(
        preview["content"]
            .as_str()
            .is_some_and(|content| content.to_lowercase().contains(expected)),
        "{preview}"
    );
}

#[test]
fn initializes_and_discovers_only_high_level_product_capabilities() {
    let (_temporary, mut adapter, _project) = setup();
    let initialized = adapter
        .handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}))
        .expect("initialize response");
    assert_eq!(initialized["result"]["serverInfo"]["name"], "volicord");
    let listed = adapter
        .handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))
        .expect("tools response");
    let names = listed["result"]["tools"]
        .as_array()
        .expect("tool array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, HOST_TOOL_NAMES);
    assert!(!listed.to_string().contains("database"));
    assert!(!listed.to_string().contains("legacy"));
    for tool in listed["result"]["tools"].as_array().expect("tool array") {
        assert_schema_is_closed_and_described(&tool["inputSchema"]);
        assert_eq!(
            schema_shapes(&tool["inputSchema"]),
            expected_shapes(tool["name"].as_str().expect("tool name")),
            "schema/handler field contract drift for {}",
            tool["name"]
        );
    }
}

#[test]
fn instructions_and_descriptions_define_resolution_recall_and_user_decision_boundaries() {
    let (_temporary, mut adapter, _project) = setup();
    let initialized = adapter
        .handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}))
        .expect("initialize response");
    let instructions = initialized["result"]["instructions"]
        .as_str()
        .expect("server instructions");
    assert!(instructions.contains("Project-scoped repository work starts with project_resolve"));
    assert!(instructions.contains("workflow.required_next_action"));
    assert!(instructions.contains("do not bypass a blocking workflow transition"));
    assert!(instructions.contains("Relevant evidence is not exact settling authority"));
    assert!(instructions.contains("exact dimension or a bounded containing scope"));
    assert!(instructions.contains("explicit response from the current host"));
    assert!(instructions.contains("separate exact authorization"));
    assert!(instructions.contains("actually observed command outcomes"));
    assert!(
        instructions.len() < 768,
        "server instructions should stay compact"
    );

    let listed = adapter
        .handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))
        .expect("tools response");
    let descriptions = listed["result"]["tools"]
        .as_array()
        .expect("tool array")
        .iter()
        .map(|tool| {
            (
                tool["name"].as_str().expect("tool name"),
                tool["description"].as_str().expect("tool description"),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert!(descriptions["project_resolve"]
        .to_lowercase()
        .contains("read-only"));
    assert!(descriptions["project_initialize"].contains("after resolution"));
    assert!(descriptions["project_initialize"].contains("local Git origin lineage"));
    assert!(descriptions["project_initialize"].contains("immediate origin hint"));
    assert!(descriptions["project_initialize"].contains("canonical repository-root basename"));
    assert!(descriptions["project_initialize"].contains("preserve it exactly"));
    assert!(descriptions["recall"].contains("Recall must succeed before repository inspection"));
    assert!(descriptions["inquiry_frontier"].contains("candidate_manage"));
    assert!(descriptions["inquiry_frontier"].contains("present each actual alternative"));
    assert!(descriptions["decision_record"].contains("not a user Decision"));
    assert!(descriptions["candidate_manage"].contains("submit a Candidate"));
    assert!(descriptions["candidate_manage"].contains("attach source-grounded repository research"));
    assert!(descriptions["candidate_manage"].contains("mark sufficient research ready"));
    assert!(
        descriptions["candidate_manage"].contains("explicitly promote a reviewed ready Candidate")
    );
    assert!(descriptions["decision_record"].contains("explicit current-host user response"));
    assert!(descriptions["decision_record"].contains("current Question revision"));
    assert!(descriptions["materiality_review"].contains("broad Goal alone is not delegation"));
    assert!(descriptions["materiality_review"].contains("semantic rationale"));
    assert!(
        descriptions["materiality_review"].contains("may constrain alternatives without settling")
    );
    assert!(
        descriptions["materiality_review"].contains("remaining after that authority is applied")
    );
    assert!(descriptions["context_record"].contains("caller-reported"));
    assert!(descriptions["context_record"].contains("does not authenticate"));
    assert!(descriptions["context_record"].contains("One turn may require repeated calls"));
    assert!(descriptions["context_record"].contains("could change authority"));
    assert!(descriptions["context_record"].contains("do not collapse the whole turn"));
    assert!(descriptions["recall"].contains("Learning, Preference, and Constraint"));
    assert!(descriptions["repository_analyze"].contains("authorized local repository"));
    assert!(descriptions["repository_analyze"].contains("source-semantic analysis"));
    assert!(descriptions["repository_analyze"].contains("source-semantic analyzer is local"));
    assert!(
        descriptions["repository_analyze"].contains("before the first ordinary repository write")
    );
    assert!(descriptions["repository_analyze"].contains("pre-work Checkpoint baseline"));
    assert!(descriptions["repository_analyze"]
        .contains("repository_source_id as the canonical source_ids basis"));
    assert!(descriptions["repository_analyze"]
        .contains("no background-provider or network transmission"));
    assert!(descriptions["repository_analyze"].contains("local Runtime Home"));
    assert!(descriptions["repository_analyze"].contains("background_semantic_operation"));
    assert!(descriptions["checkpoint_record"].contains("numeric exit status"));
    assert!(descriptions["checkpoint_record"].contains("exact transient command_invocation"));
    assert!(descriptions["checkpoint_record"].contains("presentation-only command_label"));
    assert!(descriptions["checkpoint_record"].contains("without retaining raw arguments"));
    assert!(descriptions["checkpoint_record"].contains("output-only text is insufficient"));
    assert!(descriptions["checkpoint_record"]
        .contains("first captured after the bounded work is conceptually invalid"));
}

#[test]
fn repository_analysis_exposes_its_canonical_source_identity_without_display_parsing() {
    use volicord_context::SourcePayload;
    use volicord_repository_intelligence::AnalysisSnapshot;

    let (temporary, mut adapter, project) = setup();
    let repository = temporary.path().join("repository");
    fs::write(repository.join("README.md"), "# Host analysis fixture\n")
        .expect("repository fixture");
    let project_id = parse_project(&project);

    let response = call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    );
    assert_eq!(response["result"]["isError"], false, "{response}");
    let result = structured(&response);
    assert_eq!(result["project_id"], project);
    assert!(matches!(
        result["state"].as_str(),
        Some("succeeded" | "partial")
    ));
    let analysis_id = result["analysis_snapshot_id"]
        .as_str()
        .expect("Analysis Snapshot identity");
    let repository_snapshot_id = result["repository_snapshot_id"]
        .as_str()
        .expect("Repository Snapshot identity");
    let repository_source_id = result["repository_source_id"]
        .as_str()
        .expect("repository Source identity");

    let analysis_path = adapter
        .operations()
        .layout()
        .analysis_project_dir(project_id)
        .join(format!("{analysis_id}.json"));
    let analysis: AnalysisSnapshot =
        serde_json::from_slice(&fs::read(analysis_path).expect("published Analysis Snapshot"))
            .expect("supported Analysis Snapshot");
    assert_eq!(analysis.identity.to_string(), analysis_id);
    assert_eq!(
        analysis.repository_snapshot.to_string(),
        repository_snapshot_id
    );
    assert_eq!(
        analysis.repository_source.identity().to_string(),
        repository_source_id
    );

    let canonical = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical analysis basis");
    let repository_source = canonical
        .sources
        .iter()
        .find(|basis| basis.source.id.to_string() == repository_source_id)
        .expect("canonical repository Source recorded for the analysis");
    assert!(matches!(
        &repository_source.source.payload,
        SourcePayload::RepositorySnapshot { .. }
    ));

    let persisted_again: AnalysisSnapshot = serde_json::from_slice(
        &fs::read(
            adapter
                .operations()
                .layout()
                .analysis_project_dir(project_id)
                .join(format!("{analysis_id}.json")),
        )
        .expect("same published Analysis Snapshot"),
    )
    .expect("same supported Analysis Snapshot");
    assert_eq!(
        persisted_again.repository_source.identity().to_string(),
        repository_source_id,
        "the structured Source identity is stable for the returned analysis"
    );
}

#[test]
fn repository_analysis_preserves_typed_degradation_and_bounded_diagnostics() {
    let (temporary, mut adapter, project) = setup();
    let repository = temporary.path().join("repository");
    fs::create_dir(repository.join("src")).expect("Rust source directory");
    fs::write(
        repository.join("Cargo.toml"),
        "[package]\nname='host-analysis-fixture'\nversion='0.1.0'\n",
    )
    .expect("Cargo manifest");
    fs::write(
        repository.join("src/lib.rs"),
        "pub fn answer() -> u32 { 42 }\n#[cfg(test)]\nmod tests { #[test] fn answer_is_stable() { assert_eq!(super::answer(), 42); } }\n",
    )
    .expect("Rust source");

    let response = call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    );
    assert_eq!(response["result"]["isError"], false, "{response}");
    let result = structured(&response);
    assert_eq!(result["state"], "partial", "{result}");
    assert!(result["diagnostic"]
        .as_str()
        .is_some_and(|diagnostic| diagnostic.contains("inspect capability_reports")));
    assert!(result["partial_scopes"]
        .as_array()
        .is_some_and(|scopes| scopes.iter().any(|scope| scope
            .as_str()
            .is_some_and(|scope| scope.contains("Structural:Some(Rust)")))));
    assert!(!result["failed_scopes"]
        .as_array()
        .expect("failed scopes")
        .iter()
        .any(|scope| scope
            .as_str()
            .is_some_and(|scope| scope.contains("Structural:Some(Rust)"))));

    let reports = result["capability_reports"]
        .as_array()
        .expect("capability reports");
    let report = |capability: &str, language: Option<&str>| {
        reports
            .iter()
            .find(|report| {
                report["capability"] == capability
                    && match language {
                        Some(language) => report["language"]["kind"] == language,
                        None => report["language"].is_null(),
                    }
            })
            .unwrap_or_else(|| panic!("missing {capability} report for {language:?}"))
    };
    let rust_structural = report("structural", Some("rust"));
    assert_eq!(rust_structural["state"], "partial");
    assert!(rust_structural["coverage"]["covered_entity_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert!(rust_structural["reason"].is_string());
    assert!(rust_structural["usable_remainder"].is_string());
    assert_eq!(rust_structural["recovery_owner"], "repository_intelligence");
    assert!(rust_structural["safe_next_action"]
        .as_str()
        .is_some_and(|action| action.contains("Use the reported usable remainder")));
    assert!(rust_structural["adapter"].is_object());
    assert!(rust_structural["analyzer"].is_object());

    let cargo_ecosystem = report("ecosystem", Some("rust"));
    assert!(matches!(
        cargo_ecosystem["state"].as_str(),
        Some("available" | "partial")
    ));
    assert!(cargo_ecosystem["coverage"]["included_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert!(cargo_ecosystem["usable_remainder"].is_string());

    let rust_semantic = report("semantic", Some("rust"));
    assert!(matches!(
        rust_semantic["state"].as_str(),
        Some("available" | "partial")
    ));
    assert!(rust_semantic["coverage"]["covered_relation_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert_eq!(
        rust_semantic["analyzer"]["name"],
        "volicord-source-semantic-index"
    );

    let agent_assisted = report("agent_assisted", Some("rust"));
    assert_eq!(agent_assisted["state"], "unavailable");
    assert!(agent_assisted["reason"]
        .as_str()
        .is_some_and(|reason| reason.contains("no interactive host interpretation")));
    assert!(agent_assisted["safe_next_action"].is_string());

    let toml_structural = report("structural", Some("toml"));
    assert_eq!(toml_structural["state"], "unsupported");
    assert!(toml_structural["reason"].is_string());
    assert!(toml_structural["safe_next_action"]
        .as_str()
        .is_some_and(|action| action.contains("do not retry")));

    let diagnostics = result["diagnostics"].as_array().expect("diagnostics");
    assert!(diagnostics.len() <= 64);
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic["code"] == "structural.construct_limit"));
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic["identity"].is_string()
            && diagnostic["severity"].is_string()
            && diagnostic["message"].is_string()
            && diagnostic["affected_area"].is_object()
    }));
    assert!(result["diagnostics_omitted_count"].is_u64());
}

#[test]
fn failed_repository_analysis_does_not_fabricate_a_source_identity() {
    let (temporary, mut adapter, project) = setup();
    fs::remove_dir(temporary.path().join("repository")).expect("remove bound repository");

    let response = call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    );
    assert_eq!(response["result"]["isError"], true, "{response}");
    assert!(structured(&response).get("repository_source_id").is_none());
}

#[test]
fn tool_annotations_match_the_pinned_mcp_effect_and_world_contract() {
    let (_temporary, mut adapter, _project) = setup();
    let listed = adapter
        .handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))
        .expect("tools response");
    let tools = listed["result"]["tools"]
        .as_array()
        .expect("tool array")
        .iter()
        .map(|tool| (tool["name"].as_str().expect("tool name"), tool))
        .collect::<std::collections::BTreeMap<_, _>>();

    for name in [
        "project_resolve",
        "project_health",
        "recall",
        "repository_understanding",
        "inquiry_frontier",
        "canonical_inspect",
        "candidate_inspect",
        "privacy_status",
        "document_preview",
    ] {
        assert_eq!(
            tools[name]["annotations"],
            json!({"readOnlyHint":true,"openWorldHint":false}),
            "{name}"
        );
    }
    for name in [
        "project_initialize",
        "repository_analyze",
        "decision_record",
        "context_record",
        "checkpoint_record",
        "guarded_interaction",
    ] {
        assert_eq!(
            tools[name]["annotations"],
            json!({"readOnlyHint":false,"destructiveHint":false,"idempotentHint":false,"openWorldHint":false}),
            "{name}"
        );
    }
    for name in ["canonical_mutate", "candidate_manage"] {
        assert_eq!(
            tools[name]["annotations"],
            json!({"readOnlyHint":false,"destructiveHint":true,"idempotentHint":false,"openWorldHint":false}),
            "{name}"
        );
    }
    assert_eq!(
        tools["background_semantic_operation"]["annotations"],
        json!({"readOnlyHint":false,"destructiveHint":false,"idempotentHint":false,"openWorldHint":true})
    );

    for tool in tools.values() {
        for key in tool["annotations"]
            .as_object()
            .expect("tool annotations")
            .keys()
        {
            assert!(
                [
                    "readOnlyHint",
                    "destructiveHint",
                    "idempotentHint",
                    "openWorldHint"
                ]
                .contains(&key.as_str()),
                "unsupported pinned annotation: {key}"
            );
        }
    }
}

#[test]
fn project_resolve_reports_not_found_then_current_binding_without_mutation() {
    let temporary = tempdir().expect("temporary directory");
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository).expect("repository");
    let operations = LocalOperations::new(
        RuntimeLayout::new(temporary.path().join("runtime")).expect("runtime layout"),
    );
    let mut adapter = HostAdapter::new(operations);

    assert!(!adapter.operations().layout().root().exists());
    let not_found = call(
        &mut adapter,
        "project_resolve",
        json!({"repository":repository}),
    );
    assert_eq!(not_found["result"]["isError"], false, "{not_found}");
    assert_eq!(structured(&not_found)["status"], "not_found");
    assert_eq!(
        structured(&not_found)["workflow"]["stage"],
        "project_initialization"
    );
    assert_eq!(
        structured(&not_found)["workflow"]["required_next_action"]["tool"],
        "project_initialize"
    );
    assert_eq!(
        structured(&not_found)["canonical_repository_path"],
        json!(fs::canonicalize(&repository).expect("canonical repository path"))
    );
    assert!(!adapter.operations().layout().root().exists());
    for runtime_state in [
        adapter.operations().layout().canonical_store(),
        adapter.operations().layout().candidate_store(),
        adapter.operations().layout().privacy_store(),
        adapter.operations().layout().guarded_store(),
        adapter.operations().layout().derived_dir(),
        adapter.operations().layout().artifacts_dir(),
    ] {
        assert!(
            !runtime_state.exists(),
            "{} was created",
            runtime_state.display()
        );
    }

    let initialized = call(
        &mut adapter,
        "project_initialize",
        json!({"repository":repository}),
    );
    assert_eq!(initialized["result"]["isError"], false, "{initialized}");
    assert_eq!(structured(&initialized)["display_name"], "repository");
    let project = structured(&initialized)["project_id"]
        .as_str()
        .expect("Project identity")
        .to_owned();
    let project_id = parse_project(&project);
    let before = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("before resolution");
    let found = call(
        &mut adapter,
        "project_resolve",
        json!({"repository":repository}),
    );
    assert_eq!(found["result"]["isError"], false, "{found}");
    assert_eq!(structured(&found)["status"], "found");
    assert_eq!(structured(&found)["workflow"]["stage"], "recall");
    assert_eq!(
        structured(&found)["workflow"]["required_next_action"]["tool"],
        "recall"
    );
    assert_eq!(structured(&found)["project_id"], project);
    assert_eq!(structured(&found)["binding"]["revision"], 1);
    assert_eq!(
        structured(&found)["binding"]["canonical_repository_path"],
        json!(fs::canonicalize(&repository).expect("canonical repository path"))
    );
    assert_eq!(
        before,
        adapter
            .operations()
            .canonical_basis(project_id)
            .expect("after resolution")
    );
}

#[test]
fn project_initialize_prefers_repository_native_identity_unless_name_is_explicit() {
    let temporary = tempdir().expect("temporary directory");
    let source = temporary.path().join("polyglot-medium");
    fs::create_dir_all(&source).expect("source repository");
    assert!(Command::new("git")
        .arg("-C")
        .arg(&source)
        .args(["init", "-q"])
        .status()
        .expect("git init")
        .success());
    assert!(Command::new("git")
        .arg("-C")
        .arg(&source)
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/tree-sitter/tree-sitter.git",
        ])
        .status()
        .expect("upstream origin")
        .success());
    let repository = temporary
        .path()
        .join("misleading-Volicord")
        .join("repository");
    fs::create_dir_all(repository.parent().expect("clone parent")).expect("outer repository");
    assert!(Command::new("git")
        .args(["clone", "-q"])
        .arg(&source)
        .arg(&repository)
        .status()
        .expect("git clone")
        .success());
    let explicit_source = temporary.path().join("small-python");
    fs::create_dir(&explicit_source).expect("explicit source repository");
    assert!(Command::new("git")
        .arg("-C")
        .arg(&explicit_source)
        .args(["init", "-q"])
        .status()
        .expect("explicit git init")
        .success());
    assert!(Command::new("git")
        .arg("-C")
        .arg(&explicit_source)
        .args([
            "remote",
            "add",
            "origin",
            "git@github.com:pallets/itsdangerous.git",
        ])
        .status()
        .expect("explicit upstream origin")
        .success());
    let explicit_repository = temporary.path().join("explicit-cycle").join("repository");
    fs::create_dir_all(explicit_repository.parent().expect("explicit clone parent"))
        .expect("explicit clone parent");
    assert!(Command::new("git")
        .args(["clone", "-q"])
        .arg(&explicit_source)
        .arg(&explicit_repository)
        .status()
        .expect("explicit git clone")
        .success());
    let operations = LocalOperations::new(
        RuntimeLayout::new(temporary.path().join("runtime")).expect("runtime layout"),
    );
    let mut adapter = HostAdapter::new(operations);

    let missing = call(&mut adapter, "project_initialize", json!({}));
    assert_eq!(missing["result"]["isError"], true, "{missing}");
    assert!(structured(&missing)["error"]
        .as_str()
        .is_some_and(|error| error.contains("repository is required")));
    assert!(!adapter.operations().layout().root().exists());

    let derived = call(
        &mut adapter,
        "project_initialize",
        json!({"repository":repository}),
    );
    assert_eq!(derived["result"]["isError"], false, "{derived}");
    assert_eq!(structured(&derived)["display_name"], "tree-sitter");
    assert_eq!(
        structured(&derived)["binding"],
        json!(fs::canonicalize(&repository).expect("canonical nested repository")),
    );

    let explicit = call(
        &mut adapter,
        "project_initialize",
        json!({"display_name":"User Chosen Name","repository":explicit_repository}),
    );
    assert_eq!(explicit["result"]["isError"], false, "{explicit}");
    assert_eq!(structured(&explicit)["display_name"], "User Chosen Name");
}

#[test]
fn schema_validation_rejects_unknown_missing_and_malformed_arguments_before_mutation() {
    let (_temporary, mut adapter, project) = setup();
    let project_id = parse_project(&project);
    let before = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical basis before invalid calls");

    for name in HOST_TOOL_NAMES {
        let response = call(&mut adapter, name, json!({"unexpected":true}));
        assert_eq!(response["result"]["isError"], true, "{name}: {response}");
        assert!(structured(&response)["error"]
            .as_str()
            .is_some_and(|error| error.contains("is not allowed")));
    }

    for (name, arguments, expected) in [
        ("recall", json!({}), "project_id is required"),
        (
            "decision_record",
            json!({"project_id":project}),
            "question_id is required",
        ),
        (
            "context_record",
            json!({"project_id":project}),
            "user_turn is required",
        ),
        (
            "canonical_mutate",
            json!({"action":"forget","project_id":project,"user_turn":"forget"}),
            "record_id is required",
        ),
        (
            "document_preview",
            json!({"project_id":project,"kind":"handoff-resume","format":"pdf"}),
            "is not an allowed value",
        ),
        (
            "guarded_interaction",
            json!({"confirmation_request_id":"00000000000000000000000000000000","decision":"confirm"}),
            "request_revision is required",
        ),
    ] {
        let response = call(&mut adapter, name, arguments);
        assert_eq!(response["result"]["isError"], true, "{response}");
        assert!(
            structured(&response)["error"]
                .as_str()
                .is_some_and(|error| error.contains(expected)),
            "{response}"
        );
    }

    let health = call(&mut adapter, "project_health", json!({}));
    assert_eq!(health["result"]["isError"], false, "{health}");
    let recall = call(&mut adapter, "recall", json!({"project_id":project}));
    assert_eq!(recall["result"]["isError"], false, "{recall}");

    let after = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical basis after invalid calls");
    assert_eq!(before, after);
}

#[test]
fn health_distinguishes_connection_from_degraded_capability() {
    let (temporary, mut adapter, project) = setup();
    fs::remove_dir(temporary.path().join("repository")).expect("remove bound repository");
    let response = call(
        &mut adapter,
        "project_health",
        json!({"project_id":project}),
    );
    let content = structured(&response);
    assert_eq!(content["connection"], "connected");
    assert_eq!(content["capability_state"], "degraded");
    assert_eq!(content["repository_available"], false);
}

#[test]
fn stdio_ends_cleanly_at_eof_and_preserves_ordered_responses() {
    let (_temporary, mut adapter, project) = setup();
    let input = format!(
        "{}\n{}\n{}\n",
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"project_health","arguments":{"project_id":project}}})
    );
    let mut output = Vec::new();
    run_stdio(&mut adapter, input.as_bytes(), &mut output).expect("stdio completes at EOF");
    let lines = String::from_utf8(output)
        .expect("UTF-8 output")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("JSON response"))
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["id"], 1);
    assert_eq!(lines[1]["id"], 2);
}

#[test]
fn recall_documents_and_inspection_are_read_only_host_calls() {
    let (_temporary, mut adapter, project) = setup();
    let before = adapter
        .operations()
        .canonical_basis(parse_project(&project))
        .expect("before");
    for tool in [
        "recall",
        "repository_understanding",
        "canonical_inspect",
        "candidate_inspect",
        "privacy_status",
    ] {
        let response = call(&mut adapter, tool, json!({"project_id":project}));
        assert_eq!(response["result"]["isError"], false, "{tool}: {response}");
    }
    let prepared = call(
        &mut adapter,
        "document_preview",
        json!({"project_id":project,"kind":"handoff-resume","language":"es"}),
    );
    assert_eq!(prepared["result"]["isError"], false, "{prepared}");
    let prepared_result = structured(&prepared);
    assert_eq!(prepared_result["outcome"], "realization_required");
    assert!(prepared_result.get("content").is_none());
    for claim in prepared_result["plan"]["sections"]
        .as_array()
        .expect("plan sections")
        .iter()
        .flat_map(|section| section["claims"].as_array().expect("plan claims"))
    {
        assert!(claim["source_text"]
            .as_str()
            .is_some_and(|text| text.len() <= NARRATIVE_PLAN_SOURCE_TEXT_BYTE_LIMIT));
        assert!(claim.get("source_text_omission").is_some());
        assert!(claim["omitted_protected_term_count"].is_number());
    }
    let realization = spanish_realization(&prepared_result["plan"]);
    assert!(realization["sections"]
        .as_array()
        .expect("realized sections")
        .iter()
        .flat_map(|section| section["claims"].as_array().expect("realized claims"))
        .all(|claim| claim["text"]
            .as_str()
            .is_some_and(|text| text.len() <= RENDERED_DOCUMENT_FIELD_BYTE_LIMIT)));
    let realized = call(
        &mut adapter,
        "document_preview",
        json!({
            "project_id":project,
            "kind":"handoff-resume",
            "language":"es",
            "realization":realization
        }),
    );
    assert_eq!(realized["result"]["isError"], false, "{realized}");
    let realized_result = structured(&realized);
    assert_eq!(realized_result["outcome"], "realized");
    assert_eq!(
        realized_result["generator"]["model"],
        "fixture-spanish-model"
    );
    assert!(realized_result["content"]
        .as_str()
        .is_some_and(|value| value.contains("Explicación en español")));
    let unsafe_language = "fr-CA\" data-unsafe=\"<&";
    let response = call(
        &mut adapter,
        "document_preview",
        json!({
            "project_id":project,
            "kind":"handoff-resume",
            "format":"html",
            "language":unsafe_language
        }),
    );
    let unsafe_result = structured(&response);
    assert_eq!(unsafe_result["outcome"], "realization_required");
    assert_eq!(unsafe_result["requested_language"], unsafe_language);
    assert!(unsafe_result.get("content").is_none());
    let after = adapter
        .operations()
        .canonical_basis(parse_project(&project))
        .expect("after");
    assert_eq!(before, after);
}

fn spanish_realization(plan: &Value) -> Value {
    json!({
        "plan_fingerprint":plan["plan_fingerprint"],
        "title":"Comprensión del proyecto",
        "generator":{
            "generator":"volicord-codex-host",
            "agent":"codex",
            "model":"fixture-spanish-model"
        },
        "sections":plan["sections"].as_array().expect("plan sections").iter().map(|section| json!({
            "key":section["key"],
            "title":format!("Explicación en español — {}", section["source_title"].as_str().expect("source title")),
            "claims":section["claims"].as_array().expect("plan claims").iter().map(|claim| json!({
                "identity":claim["identity"],
                "text":format!("Explicación en español: {}", claim["source_text"].as_str().expect("source text"))
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    })
}

#[test]
fn guarded_transport_and_fallback_keep_one_exact_logical_request() {
    use volicord_context::{Clock, Principal, PrincipalKind, SystemClock, TimestampMicros};
    use volicord_operations::{
        GuardedEffectCategory, GuardedEffectDraft, GuardedRisk, RequestingProvenance,
    };
    let (_temporary, mut adapter, project) = setup();
    adapter.handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}));
    let project_id = parse_project(&project);
    let now = SystemClock.now().expect("clock");
    let request = adapter
        .operations()
        .create_guarded_request(GuardedEffectDraft {
            project_id,
            exact_action: "publish".into(),
            target: "registry/example".into(),
            expected_effect: "public release".into(),
            risk: GuardedRisk {
                category: GuardedEffectCategory::ExternalDeploymentOrPublicPublication,
                concrete_consequence: "public artifact".into(),
            },
            scope: vec!["release:example".into()],
            expires_at: TimestampMicros::from_unix_micros(now.as_unix_micros() + 60_000_000),
            requesting_provenance: RequestingProvenance {
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".into(),
                },
                host: Some("codex".into()),
                session: Some("host-test".into()),
                basis: vec!["test".into()],
            },
        })
        .expect("Guarded request");
    let shown = call(
        &mut adapter,
        "guarded_interaction",
        json!({"confirmation_request_id":request.confirmation_request_identity.to_string()}),
    );
    let shown = structured(&shown);
    assert_eq!(shown["host_elicitation_available"], false);
    assert_eq!(
        shown["confirmation_request_id"],
        request.confirmation_request_identity.to_string()
    );
    assert_eq!(shown["request_revision"], request.request_revision);
    assert_eq!(shown["effect_fingerprint"], request.effect_fingerprint);
    assert_eq!(shown["fallback"]["cli"][2], "confirm");

    let confirmed = call(
        &mut adapter,
        "guarded_interaction",
        json!({
            "confirmation_request_id":request.confirmation_request_identity.to_string(),
            "request_revision":request.request_revision,
            "effect_fingerprint":request.effect_fingerprint,
            "decision":"confirm",
            "user_turn":"confirm this exact release"
        }),
    );
    let confirmed = structured(&confirmed);
    assert_eq!(
        confirmed["confirmation_request_id"],
        request.confirmation_request_identity.to_string()
    );
    assert_eq!(confirmed["request_revision"], request.request_revision);
    let source = confirmed["user_response_source_id"]
        .as_str()
        .expect("response Source");
    let canonical = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical");
    assert!(canonical
        .sources
        .iter()
        .any(|basis| basis.source.id.to_string() == source));
}

#[test]
fn codex_host_exposes_guarded_provider_dispatch_and_durable_unavailable_outcome() {
    use volicord_context::{Clock, Principal, PrincipalKind, SystemClock};

    let (temporary, mut adapter, project) = setup();
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src")).expect("source directory");
    fs::write(
        repository.join("src/lib.rs"),
        "// SECRET=host-fixture\npub fn host_path() {}\n",
    )
    .expect("source file");
    let project_id = parse_project(&project);
    adapter
        .operations()
        .analyze(project_id, Vec::new())
        .expect("analysis");
    let opt_in_source = adapter
        .operations()
        .record_user_source(
            project_id,
            "codex".into(),
            "privacy-host".into(),
            "enable provider".into(),
        )
        .expect("opt-in source");
    adapter
        .operations()
        .enable_provider(
            ProviderOptInPolicy {
                project_id,
                provider: "configured-provider".into(),
                model: "configured-model".into(),
                purpose: "background semantic analysis".into(),
                requested_capability: "semantic".into(),
                allowed_source_scopes: vec!["src/lib.rs".into()],
                exclusions: SourceExclusionPolicy {
                    path_prefixes: Vec::new(),
                    file_classes: Vec::new(),
                    basis: "host fixture".into(),
                },
                filtering: SecretFilteringPolicy {
                    enabled: true,
                    line_markers: vec!["SECRET".into()],
                    replacement: "[filtered]".into(),
                    known_limits: vec!["marker filtering is incomplete".into()],
                },
                retention: ProviderRetentionPolicy {
                    local_annotation_retained_until: None,
                    local_basis: "until explicit deletion".into(),
                    provider_expectation: "configured provider policy".into(),
                    provider_known_limits: Vec::new(),
                },
            },
            ProviderIntentProvenance {
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "current-host-user".into(),
                },
                host: "codex".into(),
                session: "privacy-host".into(),
                user_turn_source: parse_source_identity(&opt_in_source.identity),
                basis: "explicit host fixture opt-in".into(),
            },
        )
        .expect("provider opt-in");
    adapter.handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{"elicitation":{}}}}));
    let now = SystemClock.now().expect("clock");
    let prepared = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"prepare",
            "project_id":project,
            "provider":"configured-provider",
            "model":"configured-model",
            "purpose":"background semantic analysis",
            "requested_capability":"semantic",
            "source_paths":["src/lib.rs"],
            "expiration_unix_micros":u64::try_from(now.as_unix_micros() + 60_000_000).expect("positive expiration")
        }),
    );
    assert_eq!(prepared["result"]["isError"], false, "{prepared}");
    let prepared = structured(&prepared);
    assert_eq!(prepared["state"], "awaiting_exact_confirmation");
    assert_eq!(prepared["dispatch_occurred"], false);
    let request = prepared["guarded_request"].clone();
    let provider_request_id = prepared["provider_request"]["provider_request_id"]
        .as_str()
        .expect("provider request ID")
        .to_owned();

    let mismatched = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":format!("sha256:{}", "0".repeat(64))
        }),
    );
    assert_eq!(
        structured(&mismatched)["guarded_outcome"]["rejection"],
        "mismatched"
    );
    assert_eq!(
        structured(&mismatched)["provider_request"]["outcome"],
        "prepared"
    );

    let missing = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"]
        }),
    );
    assert_eq!(
        structured(&missing)["guarded_outcome"]["kind"],
        "not_dispatched"
    );
    assert_eq!(
        structured(&missing)["guarded_outcome"]["rejection"],
        "missing"
    );
    assert_eq!(
        structured(&missing)["provider_request"]["outcome"],
        "prepared"
    );

    let confirmed = call(
        &mut adapter,
        "guarded_interaction",
        json!({
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"],
            "decision":"confirm",
            "user_turn":"confirm this exact provider request"
        }),
    );
    assert_eq!(confirmed["result"]["isError"], false, "{confirmed}");
    let dispatched = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"]
        }),
    );
    let dispatched = structured(&dispatched);
    assert_eq!(dispatched["guarded_outcome"]["kind"], "not_dispatched");
    assert_eq!(dispatched["guarded_outcome"]["confirmation_consumed"], true);
    assert_eq!(
        dispatched["provider_request"]["outcome"],
        "provider_unavailable"
    );
    assert!(dispatched["provider_request"]["manifest"]
        .as_array()
        .expect("manifest")
        .iter()
        .all(|entry| entry["transmission_outcome"] == "not_transmitted"));
    let operation_id = dispatched["operation_id"].as_str().expect("operation ID");

    let inspected = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"inspect",
            "project_id":project,
            "operation_id":operation_id,
            "provider_request_id":provider_request_id
        }),
    );
    assert_eq!(structured(&inspected), dispatched);

    let reused = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"]
        }),
    );
    assert_eq!(reused["result"]["isError"], true, "{reused}");
    assert!(structured(&reused)["error"]
        .as_str()
        .is_some_and(|error| error.contains("live provider preparation is unavailable")));
    let local = call(
        &mut adapter,
        "canonical_inspect",
        json!({"project_id":project}),
    );
    assert_eq!(local["result"]["isError"], false, "{local}");
    let structural = call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project,"excluded_paths":[]}),
    );
    assert_eq!(structural["result"]["isError"], false, "{structural}");
}

#[test]
fn explicit_provider_denial_discards_the_live_preparation() {
    let (_temporary, mut adapter, project, request) = setup_guarded_provider_request(60_000_000);
    let denied = call(
        &mut adapter,
        "guarded_interaction",
        json!({
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"],
            "decision":"deny",
            "user_turn":"deny this exact provider request"
        }),
    );
    assert_eq!(structured(&denied)["decision"], "denied", "{denied}");

    let dispatch = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"]
        }),
    );
    assert_eq!(dispatch["result"]["isError"], true, "{dispatch}");
    assert!(structured(&dispatch)["error"]
        .as_str()
        .is_some_and(|error| error.contains("live provider preparation is unavailable")));
    assert_eq!(
        call(
            &mut adapter,
            "canonical_inspect",
            json!({"project_id":project})
        )["result"]["isError"],
        false
    );
}

#[test]
fn subsequent_host_interaction_cleans_an_expired_provider_preparation() {
    use volicord_context::{Clock, SystemClock};

    let (_temporary, mut adapter, project, request) = setup_guarded_provider_request(2_000_000);
    let expiration = request["expiration_unix_micros"]
        .as_i64()
        .expect("prepared request expiration");
    let after_preparation = SystemClock.now().expect("clock after preparation");
    assert!(
        after_preparation.as_unix_micros() < expiration,
        "provider preparation must succeed while its request is still valid"
    );
    let remaining = u64::try_from(expiration - after_preparation.as_unix_micros())
        .expect("positive remaining preparation lifetime");
    std::thread::sleep(
        std::time::Duration::from_micros(remaining)
            .saturating_add(std::time::Duration::from_millis(50)),
    );
    assert!(
        SystemClock
            .now()
            .expect("clock after expiration wait")
            .as_unix_micros()
            >= expiration,
        "cleanup interaction must occur only after the prepared request expires"
    );

    let health = call(
        &mut adapter,
        "project_health",
        json!({"project_id":project}),
    );
    assert_eq!(health["result"]["isError"], false, "{health}");

    let dispatch = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"dispatch",
            "confirmation_request_id":request["confirmation_request_id"],
            "request_revision":request["request_revision"],
            "effect_fingerprint":request["effect_fingerprint"]
        }),
    );
    assert_eq!(dispatch["result"]["isError"], true, "{dispatch}");
    assert!(structured(&dispatch)["error"]
        .as_str()
        .is_some_and(|error| error.contains("live provider preparation is unavailable")));
}

#[test]
fn grounded_checkpoint_preserves_repository_decision_verification_and_restart_readback() {
    use volicord_context::{
        AgentRecommendation, Availability, NonUserQuestionOutcome, OperationId, Principal,
        PrincipalKind, QuestionAlternative, QuestionDraft, QuestionMateriality,
        QuestionResearchState, SourceDraft, SourcePayload, Store,
    };

    let (temporary, mut adapter, project) = setup();
    let project_id = parse_project(&project);
    let repository = temporary.path().join("repository");
    assert!(Command::new("git")
        .arg("-C")
        .arg(&repository)
        .arg("init")
        .status()
        .expect("run git init")
        .success());
    fs::write(
        repository.join("pre-existing.txt"),
        "unrelated dirty content\n",
    )
    .expect("pre-existing dirty fixture");
    let mut store = Store::open(adapter.operations().layout().canonical_store())
        .expect("open canonical test support store");
    let canonical_project = store.get_project(project_id).expect("load Project");
    let basis = store
        .record_source(
            OperationId::from_bytes([201; 16]),
            project_id,
            SourceDraft {
                expected_project_revision: canonical_project.revision,
                payload: SourcePayload::File {
                    locator: "src/policy.rs".into(),
                    snapshot: "v08-fixture".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "v08-fixture".into(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )
        .expect("record Question basis")
        .value;
    let question = store
        .create_question(
            OperationId::from_bytes([202; 16]),
            project_id,
            QuestionDraft {
                expected_project_revision: canonical_project.revision,
                prompt_basis: "Choose the V08 storage boundary".into(),
                source_basis: vec![basis.id],
                dependencies: Vec::new(),
                alternatives: vec![
                    QuestionAlternative {
                        key: "local".into(),
                        label: "Local".into(),
                        consequence: "Keep canonical data local".into(),
                    },
                    QuestionAlternative {
                        key: "remote".into(),
                        label: "Remote".into(),
                        consequence: "Require a separate provider decision".into(),
                    },
                ],
                recommendation: AgentRecommendation {
                    alternative_key: Some("local".into()),
                    rationale: "The accepted product boundary is local-first".into(),
                    source_basis: vec![basis.id],
                },
                trade_offs: vec!["Remote augmentation remains separate".into()],
                uncertainty: Vec::new(),
                material_scope: vec!["storage".into()],
                materiality: QuestionMateriality::Material,
                presentation_order: 1,
                why_it_matters_now: "The host journey needs an exact user choice".into(),
                established_facts: Vec::new(),
                assumptions: Vec::new(),
                known_limits: Vec::new(),
                what_the_answer_unlocks: vec!["V08 Decision transport".into()],
                allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
                research_state: QuestionResearchState::ReadyToAsk,
            },
        )
        .expect("create Question")
        .value;
    drop(store);

    let decision = structured(&call(
        &mut adapter,
        "decision_record",
        json!({
            "project_id": project,
            "question_id": question.id.to_string(),
            "question_revision": question.revision,
            "alternative_key": "local",
            "user_turn": "Use the local storage boundary",
            "user_rationale": "Canonical project memory remains local"
        }),
    ))
    .clone();
    assert_eq!(decision["all_succeeded"], true, "{decision}");
    let decision_source = decision["user_response_source_id"]
        .as_str()
        .expect("Decision Source")
        .to_owned();
    let decision_id = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical after Decision")
        .active_decisions[0]
        .decision
        .id
        .to_string();

    let goal = structured(&call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":"Complete a grounded ordinary-work handoff for the next Codex session.",
            "role":"goal",
            "statement":"Complete a grounded ordinary-work handoff"
        }),
    ))
    .clone();
    let goal_context_id = goal["context_item_id"]
        .as_str()
        .expect("Goal identity")
        .to_owned();
    let goal_source = goal["source_id"].as_str().expect("Goal Source").to_owned();

    let baseline = structured(&call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    ))
    .clone();
    let baseline_id = baseline["analysis_snapshot_id"]
        .as_str()
        .expect("baseline Analysis Snapshot")
        .to_owned();
    let goal_context = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical Goal basis")
        .context_items
        .into_iter()
        .find(|item| item.id.to_string() == goal_context_id)
        .expect("typed Goal Context identity")
        .id;
    let baseline_identity =
        volicord_repository_intelligence::AnalysisSnapshotId::from_hex(&baseline_id)
            .expect("typed baseline identity");
    let repository_source_id = parse_source_identity(
        baseline["repository_source_id"]
            .as_str()
            .expect("baseline repository Source"),
    );
    let discovery = adapter
        .operations()
        .record_engineering_choice_discovery(EngineeringChoiceDiscoveryDraft {
            project_id,
            goal_context_id: goal_context,
            baseline_analysis_snapshot_id: baseline_identity,
            session: "mcp-checkpoint-fixture".into(),
            source_operation: "engineering-choice-discovery".into(),
            summary: "discover the grounded Checkpoint contract choice".into(),
            choices: vec![EngineeringChoice {
                choice_id: "grounded-checkpoint-contract".into(),
                summary: "grounded Checkpoint behavior".into(),
                affected_scope: vec!["host-checkpoint".into()],
                alternatives: vec![
                    EngineeringAlternative {
                        alternative_id: "grounded".into(),
                        summary: "ground the Checkpoint".into(),
                        technical_consequences: vec!["preserves truthful evidence".into()],
                    },
                    EngineeringAlternative {
                        alternative_id: "ungrounded".into(),
                        summary: "omit grounding".into(),
                        technical_consequences: vec!["loses truthful evidence".into()],
                    },
                ],
                technical_consequences: vec!["records bounded work and truthful evidence".into()],
                source_basis: vec![repository_source_id],
                effect_categories: vec![EngineeringEffectCategory::MaintenanceOrSupport],
                relationship: EngineeringChoiceRelationship::Independent,
                evidence_state: EngineeringChoiceEvidenceState::Sufficient,
            }],
            material_boundary_review: EngineeringEffectCategory::ALL
                .into_iter()
                .map(|effect_category| MaterialBoundaryReview {
                    effect_category,
                    conclusion: if effect_category
                        == EngineeringEffectCategory::MaintenanceOrSupport
                    {
                        MaterialBoundaryConclusion::RepresentedByChoices {
                            choice_ids: vec!["grounded-checkpoint-contract".into()],
                        }
                    } else {
                        MaterialBoundaryConclusion::NoIndependentFork {
                            rationale: "the fixture has no separate outcome in this category"
                                .into(),
                        }
                    },
                    source_basis: vec![repository_source_id],
                })
                .collect(),
        })
        .expect("pre-work Engineering Choice Discovery");
    let review = adapter
        .operations()
        .record_materiality_review(MaterialityReviewDraft {
            project_id,
            goal_context_id: goal_context,
            baseline_analysis_snapshot_id: baseline_identity,
            session: "mcp-checkpoint-fixture".into(),
            source_operation: "pre-work-review".into(),
            rationale: "the host fixture follows its accepted grounded-checkpoint contract".into(),
            learning_participation: volicord_operations::LearningParticipation::Inactive,
            engineering_choice_discovery_candidate_id: discovery.discovery_candidate_id,
            dimensions: vec![MaterialityDimension {
                dimension_id: "grounded-checkpoint-contract".into(),
                discovered_choice_ids: vec!["grounded-checkpoint-contract".into()],
                summary: "grounded Checkpoint behavior".into(),
                affected_scope: vec!["host-checkpoint".into()],
                material_consequences: vec!["records bounded work and truthful evidence".into()],
                observable_signals: Vec::new(),
                ownership: MaterialOutcomeOwnershipAssessment {
                    materially_varying_outcomes: vec![
                        "whether Checkpoint evidence is bounded and truthful".into(),
                    ],
                    contains_user_owned_outcome: true,
                    user_owned_outcomes: vec!["the accepted Checkpoint product contract".into()],
                    rationale: "the alternatives change the accepted user-visible contract".into(),
                    bounded_implementation_discretion_rationale: None,
                    source_basis: vec![repository_source_id],
                },
                disposition: MaterialityDisposition::SettledAuthority,
                basis: WorkAuthorityBasis {
                    kinds: vec![WorkAuthorityBasisKind::AcceptedContract],
                    summary: "accepted source-grounded Checkpoint contract".into(),
                    authority_counterfactual:
                        "The accepted contract selects the exact Checkpoint behavior.".into(),
                    exact_authority: Some(volicord_operations::ExactAuthoritySufficiency {
                        covered_outcome: "the complete grounded Checkpoint behavior".into(),
                        remaining_credible_alternatives: Vec::new(),
                        unique_outcome_rationale:
                            "the accepted contract explicitly requires this exact behavior".into(),
                    }),
                    source_basis: vec![repository_source_id],
                    contract_basis: vec!["rebuild/docs/design/inquiry-and-decision.md".into()],
                    decision_basis: Vec::new(),
                    research_basis: Vec::new(),
                    explicit_delegation: None,
                },
                learning_value: volicord_operations::LearningValueAssessment::Routine {
                    rationale: "the accepted contract leaves no learning fork".into(),
                },
            }],
        })
        .expect("pre-work Materiality Review");
    adapter
        .operations()
        .bind_executable_work_scope(
            project_id,
            goal_context,
            baseline_identity,
            review.review_candidate_id,
            ApplicabilityScope {
                paths: vec!["implemented.rs".into()],
                components: Vec::new(),
                work_contexts: Vec::new(),
            },
        )
        .expect("pre-work executable scope");

    let other_repository = temporary.path().join("other-repository");
    fs::create_dir(&other_repository).expect("other repository");
    fs::write(other_repository.join("other.txt"), "other Project\n").expect("other fixture");
    let other = adapter
        .operations()
        .initialize_project("Other Host Project", Some(&other_repository))
        .expect("other Project");
    let other_analysis = adapter
        .operations()
        .analyze(other.project.id, Vec::new())
        .expect("other analysis")
        .value
        .expect("other analysis value")
        .analysis
        .identity
        .to_string();

    let invalid_args =
        |project_id: String, goal_id: String, analysis_id: String, decisions: Value| {
            json!({
                "project_id":project_id,
                "goal_context_id":goal_id,
                "baseline_analysis_snapshot_id":analysis_id,
                "kind":"handoff",
                "work_state":"paused",
                "applied_decision_ids":decisions,
                "verification":[{"state":"not_run"}],
                "next_step":"Continue",
                "handoff_to":"next Codex session"
            })
        };
    let wrong_project = call(
        &mut adapter,
        "checkpoint_record",
        invalid_args(
            other.project.id.to_string(),
            goal_context_id.clone(),
            other_analysis.clone(),
            json!([]),
        ),
    );
    assert_eq!(wrong_project["result"]["isError"], true, "{wrong_project}");
    let wrong_baseline = call(
        &mut adapter,
        "checkpoint_record",
        invalid_args(
            project.clone(),
            goal_context_id.clone(),
            other_analysis,
            json!([]),
        ),
    );
    assert_eq!(
        wrong_baseline["result"]["isError"], true,
        "{wrong_baseline}"
    );

    let non_goal_response = call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":"This is a constraint, not the Goal.",
            "role":"constraint",
            "statement":"This is a constraint"
        }),
    );
    assert_eq!(
        non_goal_response["result"]["isError"], false,
        "{non_goal_response}"
    );
    let non_goal = structured(&non_goal_response).clone();
    let wrong_goal = call(
        &mut adapter,
        "checkpoint_record",
        invalid_args(
            project.clone(),
            non_goal["context_item_id"]
                .as_str()
                .expect("non-Goal ID")
                .into(),
            baseline_id.clone(),
            json!([]),
        ),
    );
    assert_eq!(wrong_goal["result"]["isError"], true, "{wrong_goal}");
    let wrong_decision = call(
        &mut adapter,
        "checkpoint_record",
        invalid_args(
            project.clone(),
            goal_context_id.clone(),
            baseline_id.clone(),
            json!(["eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"]),
        ),
    );
    assert_eq!(
        wrong_decision["result"]["isError"], true,
        "{wrong_decision}"
    );
    let unexecuted_pass = call(
        &mut adapter,
        "checkpoint_record",
        json!({
            "project_id":project,
            "goal_context_id":goal_context_id,
            "baseline_analysis_snapshot_id":baseline_id,
            "kind":"handoff",
            "work_state":"paused",
            "applied_decision_ids":[],
            "verification":[{"state":"passed"}],
            "next_step":"Continue",
            "handoff_to":"next Codex session"
        }),
    );
    assert_eq!(
        unexecuted_pass["result"]["isError"], true,
        "{unexecuted_pass}"
    );
    let asserted_digest = call(
        &mut adapter,
        "checkpoint_record",
        json!({
            "project_id":project,
            "goal_context_id":goal_context_id,
            "baseline_analysis_snapshot_id":baseline_id,
            "kind":"handoff",
            "work_state":"paused",
            "applied_decision_ids":[],
            "verification":[{
                "state":"passed",
                "command_label":"caller asserted digest",
                "invocation_fingerprint":format!("sha256:{}", "0".repeat(64)),
                "exit_code":0,
                "termination":"exited",
                "outcome":"caller claimed success"
            }],
            "next_step":"Continue",
            "handoff_to":"next Codex session"
        }),
    );
    assert_eq!(
        asserted_digest["result"]["isError"], true,
        "{asserted_digest}"
    );
    assert!(asserted_digest["result"]["content"][0]["text"]
        .as_str()
        .expect("asserted digest error text")
        .contains("invocation_fingerprint is not allowed"));
    let not_run_with_invocation = call(
        &mut adapter,
        "checkpoint_record",
        json!({
            "project_id":project,
            "goal_context_id":goal_context_id,
            "baseline_analysis_snapshot_id":baseline_id,
            "kind":"handoff",
            "work_state":"paused",
            "applied_decision_ids":[],
            "verification":[{
                "state":"not_run",
                "command_invocation":"cargo test -p never-ran"
            }],
            "next_step":"Continue",
            "handoff_to":"next Codex session"
        }),
    );
    assert_eq!(
        not_run_with_invocation["result"]["isError"], true,
        "{not_run_with_invocation}"
    );
    let output_only_failure = call(
        &mut adapter,
        "checkpoint_record",
        json!({
            "project_id":project,
            "goal_context_id":goal_context_id,
            "baseline_analysis_snapshot_id":baseline_id,
            "kind":"handoff",
            "work_state":"paused",
            "applied_decision_ids":[],
            "verification":[{
                "state":"failed",
                "command_label":"focused test suite",
                "command_invocation":"cargo test -p focused -- --exact privacy_secret_7f9d",
                "termination":"exited",
                "outcome":"test output reported a failure but no exit status was observed"
            }],
            "next_step":"Continue",
            "handoff_to":"next Codex session"
        }),
    );
    assert_eq!(
        output_only_failure["result"]["isError"], true,
        "{output_only_failure}"
    );
    assert!(output_only_failure["result"]["content"][0]["text"]
        .as_str()
        .expect("Checkpoint error text")
        .contains("exit_code is required"));

    fs::write(repository.join("implemented.rs"), "pub fn grounded() {}\n")
        .expect("ordinary work change");

    let checkpoint = structured(&call(
        &mut adapter,
        "checkpoint_record",
        json!({
            "project_id": project,
            "goal_context_id": goal_context_id,
            "baseline_analysis_snapshot_id": baseline_id,
            "kind":"handoff",
            "work_state":"paused",
            "state_change":"Implemented the grounded handoff path",
            "applied_decision_ids":[decision_id],
            "verification":[
                {"state":"passed","command_label":"focused test suite","command_invocation":"cargo test -p focused -- --exact privacy_secret_7f9d","exit_code":0,"termination":"exited","outcome":"focused test passed"},
                {"state":"failed","command_label":"known failure reproduction","command_invocation":"cargo test -p known-failure -- --exact fixture","exit_code":1,"termination":"exited","outcome":"known failure reproduced"},
                {"state":"not_run"}
            ],
            "next_step": "Run maintained V08 assertions",
            "known_limits": ["V11 is independent"],
            "handoff_to":"next Codex session"
        }),
    ))
    .clone();
    assert_eq!(
        checkpoint["changed_paths"],
        json!(["implemented.rs"]),
        "{checkpoint}"
    );
    assert_eq!(
        checkpoint["pre_existing_dirty_paths"],
        json!(["pre-existing.txt"]),
        "{checkpoint}"
    );
    assert_eq!(
        checkpoint["applied_decision_ids"],
        json!([decision_id]),
        "{checkpoint}"
    );
    assert_eq!(
        checkpoint["verification_source_ids"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert!(!checkpoint["changed_paths"]
        .as_array()
        .expect("paths")
        .contains(&json!("pre-existing.txt")));

    let canonical = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical basis");
    for source_id in [decision_source, goal_source] {
        let source = canonical
            .sources
            .iter()
            .find(|basis| basis.source.id.to_string() == source_id)
            .expect("current-host Source remains canonical");
        assert!(matches!(
            source.source.payload,
            SourcePayload::CurrentHostUserTurn { ref host, .. } if host == "codex"
        ));
        assert_eq!(source.source.actor.kind, PrincipalKind::User);
    }
    let saved = canonical
        .latest_checkpoint
        .as_ref()
        .expect("latest Checkpoint");
    assert_eq!(saved.goal, "Complete a grounded ordinary-work handoff");
    assert_eq!(saved.changed_paths, vec!["implemented.rs"]);
    assert_eq!(saved.applied_decisions[0].to_string(), decision_id);
    assert_eq!(
        saved.verification[0].state,
        volicord_context::VerificationState::Passed
    );
    assert_eq!(
        saved.verification[1].state,
        volicord_context::VerificationState::Failed
    );
    assert_eq!(
        saved.verification[2].state,
        volicord_context::VerificationState::NotRun
    );
    assert_eq!(saved.verification[2].source_id, None);
    let expected_fingerprints = [
        "sha256:bdbbfdc61bceaf88be737630d86472d17fc0b3d9dbf29ed79c5149b08bf45ac5",
        "sha256:fec2f891e196435819bb2ff7a83f6f1be031fe46099a0abe35d4e872d7cea653",
    ];
    for ((fact, exit_code), expected_fingerprint) in saved
        .verification
        .iter()
        .zip([0, 1])
        .zip(expected_fingerprints)
    {
        let source = canonical
            .sources
            .iter()
            .find(|basis| Some(basis.source.id) == fact.source_id)
            .expect("verification command Source");
        assert!(matches!(
            &source.source.payload,
            SourcePayload::CommandExecution {
                invocation_fingerprint,
                outcome: volicord_context::CommandOutcome {
                    exit_code: Some(code),
                    termination: volicord_context::CommandTermination::Exited,
                },
                ..
            } if *code == exit_code && invocation_fingerprint == expected_fingerprint
        ));
        assert_eq!(source.source.actor.kind, PrincipalKind::Command);
        assert_eq!(
            source.source.observer.as_ref().map(|value| value.kind),
            Some(PrincipalKind::Agent)
        );
    }
    let raw_invocations = [
        "cargo test -p focused -- --exact privacy_secret_7f9d",
        "cargo test -p known-failure -- --exact fixture",
    ];
    let canonical_bytes =
        fs::read(adapter.operations().layout().canonical_store()).expect("canonical store bytes");
    for invocation in raw_invocations {
        assert!(!canonical_bytes
            .windows(invocation.len())
            .any(|window| window == invocation.as_bytes()));
    }
    let portable_bundle = temporary.path().join("verification-context.json");
    adapter
        .operations()
        .export_bundle(project_id, &portable_bundle)
        .expect("portable verification export");
    let portable_bytes = fs::read(&portable_bundle).expect("portable verification bytes");
    for invocation in raw_invocations {
        assert!(!portable_bytes
            .windows(invocation.len())
            .any(|window| window == invocation.as_bytes()));
    }

    let imported_runtime = temporary.path().join("imported-runtime");
    let imported = LocalOperations::new(
        RuntimeLayout::new(&imported_runtime).expect("portable import runtime"),
    );
    imported
        .import_bundle(&portable_bundle)
        .expect("portable verification import");
    let imported_basis = imported
        .canonical_basis(project_id)
        .expect("imported canonical basis");
    for expected_fingerprint in expected_fingerprints {
        assert!(imported_basis.sources.iter().any(|basis| matches!(
            &basis.source.payload,
            SourcePayload::CommandExecution {
                invocation_fingerprint,
                ..
            } if invocation_fingerprint == expected_fingerprint
        )));
    }
    assert_eq!(
        saved.user_review.state,
        volicord_context::UserReviewState::NotRequested
    );
    assert_eq!(
        saved.user_acceptance.state,
        volicord_context::UserAcceptanceState::NotRequested
    );

    let mut restarted = HostAdapter::new(LocalOperations::new(
        RuntimeLayout::new(temporary.path().join("runtime")).expect("restart runtime"),
    ));
    let recalled = structured(&call(
        &mut restarted,
        "recall",
        json!({"project_id":project}),
    ))
    .clone();
    assert_eq!(
        recalled["goals"],
        json!(["Complete a grounded ordinary-work handoff"])
    );
    assert_eq!(recalled["checkpoint"]["work_state"], "paused");
    assert_eq!(
        recalled["checkpoint"]["changed_paths"],
        json!(["implemented.rs"])
    );
    assert_eq!(
        recalled["checkpoint"]["applied_decisions"],
        json!([decision_id])
    );
    assert_eq!(recalled["checkpoint"]["verification"][0]["state"], "passed");
    assert_eq!(recalled["checkpoint"]["verification"][1]["state"], "failed");
    assert_eq!(
        recalled["checkpoint"]["verification"][2]["state"],
        "not_run"
    );
    assert_eq!(
        recalled["decisions"][0]["rationale"],
        "Canonical project memory remains local"
    );
    assert_eq!(
        recalled["checkpoint"]["known_limits"],
        json!(["V11 is independent"])
    );
    assert_eq!(recalled["next_step"], "Run maintained V08 assertions");
}

#[test]
fn current_host_goal_context_is_canonical_and_recalled_from_exact_user_text() {
    let (_temporary, mut adapter, project) = setup();
    let user_turn =
        "For this work, make grounded checkpoints available to ordinary Codex sessions.";
    let statement = "make grounded checkpoints available to ordinary Codex sessions";
    let recorded = structured(&call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":user_turn,
            "role":"goal",
            "statement":statement,
        }),
    ))
    .clone();
    assert_eq!(recorded["role"], "goal", "{recorded}");
    assert_eq!(
        recorded["user_turn_content_provenance"], "caller_supplied_not_host_authenticated",
        "{recorded}"
    );
    assert_eq!(
        recorded["source_id"].as_str().map(str::len),
        Some(32),
        "{recorded}"
    );
    assert_eq!(
        recorded["context_item_id"].as_str().map(str::len),
        Some(32),
        "{recorded}"
    );

    let recall = structured(&call(&mut adapter, "recall", json!({"project_id":project}))).clone();
    assert_eq!(recall["goals"], json!([statement]), "{recall}");
    assert_eq!(
        recall["behaviorally_relevant_context"],
        json!([]),
        "{recall}"
    );

    let rejected = call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":"The user only stated a narrow goal.",
            "role":"goal",
            "statement":"An agent-authored expansion that the user did not state",
        }),
    );
    assert_eq!(rejected["result"]["isError"], true, "{rejected}");
    assert!(structured(&rejected)["error"]
        .as_str()
        .is_some_and(|error| error.contains("occur verbatim")));
}

#[test]
fn fresh_recall_recovers_separate_goal_learning_preference_and_constraint_context() {
    let (temporary, mut adapter, project) = setup();
    let user_turn = "Implement the bounded parser change. I want to learn the consequential trade-offs. Prefer concise explanations. Do not interrupt me for routine wording or test details.";
    for (role, statement) in [
        ("goal", "Implement the bounded parser change"),
        ("learning", "I want to learn the consequential trade-offs"),
        ("preference", "Prefer concise explanations"),
        (
            "constraint",
            "Do not interrupt me for routine wording or test details",
        ),
    ] {
        let recorded = call(
            &mut adapter,
            "context_record",
            json!({
                "project_id":project,
                "user_turn":user_turn,
                "role":role,
                "statement":statement,
            }),
        );
        assert_eq!(recorded["result"]["isError"], false, "{recorded}");
    }

    drop(adapter);
    let mut fresh = HostAdapter::new(LocalOperations::new(
        RuntimeLayout::new(temporary.path().join("runtime")).expect("restart runtime"),
    ));
    let recalled = structured(&call(&mut fresh, "recall", json!({"project_id":project}))).clone();
    assert_eq!(
        recalled["goals"],
        json!(["Implement the bounded parser change"]),
        "{recalled}"
    );
    let behavioral = recalled["behaviorally_relevant_context"]
        .as_array()
        .expect("behaviorally relevant Context array");
    assert_eq!(behavioral.len(), 3, "{recalled}");
    for (role, statement) in [
        ("learning", "I want to learn the consequential trade-offs"),
        ("preference", "Prefer concise explanations"),
        (
            "constraint",
            "Do not interrupt me for routine wording or test details",
        ),
    ] {
        let item = behavioral
            .iter()
            .find(|item| item["role"] == role)
            .unwrap_or_else(|| panic!("missing {role}: {recalled}"));
        assert_eq!(item["statement"], statement, "{item}");
        assert_eq!(item["identity"].as_str().map(str::len), Some(32), "{item}");
        assert_eq!(
            item["source_ids"][0].as_str().map(str::len),
            Some(32),
            "{item}"
        );
    }
}

#[test]
fn supported_candidate_research_is_source_grounded_and_separate_from_promotion_and_decision() {
    let (temporary, mut adapter, project) = setup();
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src")).expect("source directory");
    fs::write(
        repository.join("src/lib.rs"),
        "pub fn research_fixture() {}\n",
    )
    .expect("source file");
    let project_id = parse_project(&project);
    let analyzed = adapter
        .operations()
        .analyze(project_id, Vec::new())
        .expect("analysis")
        .value
        .expect("completed analysis");
    let repository_source = analyzed.analysis.repository_source.identity().to_string();
    let wrong_source = adapter
        .operations()
        .record_user_source(
            project_id,
            "codex".into(),
            "candidate-research".into(),
            "this user turn is not repository research".into(),
        )
        .expect("non-repository Source")
        .identity;
    let mut arguments =
        question_candidate_arguments(&project, &repository_source, 1, "Choose repository policy");
    arguments["research_state"] = json!("research_required");
    arguments["research_state_basis"] =
        json!("repository facts must be established before asking for user judgment");
    let submitted = structured(&call(&mut adapter, "candidate_manage", arguments)).clone();
    assert_eq!(submitted["research_state"], "research_required");
    let candidate_id = submitted["candidate_id"]
        .as_str()
        .expect("Candidate identity")
        .to_owned();
    assert!(adapter
        .operations()
        .inquiry_frontier(project_id, Vec::new())
        .expect("frontier before research")
        .questions
        .is_empty());

    let premature_promotion = call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"promote_question",
            "project_id":project,
            "candidate_id":candidate_id
        }),
    );
    assert_eq!(
        premature_promotion["result"]["isError"], true,
        "{premature_promotion}"
    );

    let premature = call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"mark_research_ready",
            "project_id":project,
            "candidate_id":candidate_id
        }),
    );
    assert_eq!(premature["result"]["isError"], true, "{premature}");

    let mismatched_source = call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"attach_repository_research",
            "project_id":project,
            "candidate_id":candidate_id,
            "capability":"structural",
            "coverage":"src/lib.rs",
            "freshness":"current",
            "source_ids":[wrong_source],
            "evidence_assessment":"sufficient",
            "limits":[]
        }),
    );
    assert_eq!(
        mismatched_source["result"]["isError"], true,
        "{mismatched_source}"
    );

    let insufficient = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"attach_repository_research",
            "project_id":project,
            "candidate_id":candidate_id,
            "capability":"structural",
            "coverage":"src/lib.rs declarations only",
            "freshness":"current",
            "source_ids":[repository_source],
            "evidence_assessment":"insufficient",
            "limits":["runtime behavior remains unknown"]
        }),
    ))
    .clone();
    assert_eq!(insufficient["research_state"], "research_required");
    assert_eq!(insufficient["promoted"], false);
    assert_eq!(insufficient["canonical_mutation"], false);
    assert!(adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical after research attachment")
        .active_questions
        .is_empty());

    let still_premature = call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"mark_research_ready",
            "project_id":project,
            "candidate_id":candidate_id
        }),
    );
    assert_eq!(
        still_premature["result"]["isError"], true,
        "{still_premature}"
    );

    let sufficient = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"attach_repository_research",
            "project_id":project,
            "candidate_id":candidate_id,
            "capability":"structural",
            "coverage":"current repository policy implementation and call sites",
            "freshness":"current",
            "source_ids":[repository_source],
            "evidence_assessment":"sufficient",
            "limits":["external runtime behavior is excluded"]
        }),
    ))
    .clone();
    assert_eq!(sufficient["research_state"], "research_required");
    assert_eq!(sufficient["promoted"], false);
    assert_eq!(
        sufficient["repository_research"][1]["analysis_snapshot"],
        analyzed.analysis.identity.to_string()
    );
    assert_eq!(
        sufficient["repository_research"][1]["repository_snapshot"],
        analyzed.analysis.repository_snapshot.to_string()
    );

    let ready = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"mark_research_ready",
            "project_id":project,
            "candidate_id":candidate_id
        }),
    ))
    .clone();
    assert_eq!(ready["research_state"], "ready_to_ask");
    assert_eq!(ready["promoted"], false);
    let inspected = structured(&call(
        &mut adapter,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    assert_eq!(inspected["candidates"][0]["research_state"], "ready_to_ask");
    assert_eq!(
        inspected["candidates"][0]["repository_research"]
            .as_array()
            .expect("research evidence")
            .len(),
        2
    );

    let promoted = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"promote_question",
            "project_id":project,
            "candidate_id":candidate_id
        }),
    ))
    .clone();
    let frontier = structured(&call(
        &mut adapter,
        "inquiry_frontier",
        json!({"project_id":project}),
    ))
    .clone();
    assert_eq!(
        frontier["questions"][0]["identity"],
        promoted["question_id"]
    );
    let decided = structured(&call(
        &mut adapter,
        "decision_record",
        json!({
            "project_id":project,
            "question_id":promoted["question_id"],
            "question_revision":frontier["questions"][0]["revision"],
            "alternative_key":"local",
            "user_turn":"Choose the local repository policy",
            "user_rationale":"Keep this Project local-first"
        }),
    ))
    .clone();
    assert_eq!(decided["all_succeeded"], true, "{decided}");
}

#[test]
fn supported_candidate_path_requires_explicit_promotion_and_current_host_decision() {
    use volicord_context::{
        Availability, OperationId, Principal, PrincipalKind, SourceDraft, SourcePayload, Store,
    };

    let (_temporary, mut adapter, project) = setup();
    let project_id = parse_project(&project);
    let mut store = Store::open(adapter.operations().layout().canonical_store())
        .expect("open canonical test support store");
    let canonical_project = store.get_project(project_id).expect("load Project");
    let source = store
        .record_source(
            OperationId::from_bytes([211; 16]),
            project_id,
            SourceDraft {
                expected_project_revision: canonical_project.revision,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "candidate-host-fixture".into(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "candidate-host-fixture".into(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".into(),
                }),
                availability: Availability::Available,
            },
        )
        .expect("record Candidate Source")
        .value;
    drop(store);

    let submitted = structured(&call(
        &mut adapter,
        "candidate_manage",
        question_candidate_arguments(&project, &source.id.to_string(), 1, "Choose storage"),
    ))
    .clone();
    assert_eq!(submitted["state"], "stored", "{submitted}");
    assert_eq!(submitted["research_state"], "ready_to_ask");
    assert_eq!(
        submitted["research_state_basis"],
        "the unresolved branch is purely a user judgment"
    );
    assert_eq!(submitted["canonical_mutation"], false);
    let candidate_id = submitted["candidate_id"]
        .as_str()
        .expect("Candidate identity")
        .to_owned();

    let before_promotion = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical before promotion");
    assert!(before_promotion.active_questions.is_empty());
    assert!(before_promotion.active_decisions.is_empty());
    let inspected = structured(&call(
        &mut adapter,
        "candidate_inspect",
        json!({"project_id":project}),
    ))
    .clone();
    let candidate = inspected["candidates"]
        .as_array()
        .expect("Candidate inspection")
        .iter()
        .find(|value| value["identity"] == candidate_id)
        .expect("submitted Candidate");
    assert_eq!(candidate["disposition"]["state"], "pending_or_retained");
    assert_eq!(candidate["origin"]["actor_kind"], "agent");
    assert_eq!(candidate["research_state"], "ready_to_ask");
    assert!(candidate["repository_research"]
        .as_array()
        .expect("repository research")
        .is_empty());
    assert_eq!(
        candidate["observation_basis"]["source_ids"][0],
        source.id.to_string()
    );
    assert!(candidate["observation_basis"]["other"]
        .as_str()
        .is_some_and(|basis| basis.contains("purely a user judgment")));
    assert_eq!(
        candidate["collection_scope"]["source_operation"],
        "design-review"
    );
    assert_eq!(inspected["read_only"], true);
    assert!(adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical after inspection")
        .active_questions
        .is_empty());

    let promoted = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"promote_question",
            "project_id":project,
            "candidate_id":candidate_id
        }),
    ))
    .clone();
    let question_id = promoted["question_id"]
        .as_str()
        .expect("promoted Question identity")
        .to_owned();
    let frontier = structured(&call(
        &mut adapter,
        "inquiry_frontier",
        json!({"project_id":project}),
    ))
    .clone();
    assert_eq!(frontier["questions"][0]["identity"], question_id);
    let after_promotion = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical after promotion");
    assert_eq!(after_promotion.active_questions.len(), 1);
    assert!(after_promotion.active_decisions.is_empty());

    let stale_decision = call(
        &mut adapter,
        "decision_record",
        json!({
            "project_id":project,
            "question_id":question_id,
            "question_revision":frontier["questions"][0]["revision"]
                .as_u64()
                .expect("Question revision") + 1,
            "alternative_key":"local",
            "user_turn":"Choose the local Candidate alternative"
        }),
    );
    assert_eq!(
        stale_decision["result"]["isError"], true,
        "{stale_decision}"
    );
    assert!(structured(&stale_decision)["error"]
        .as_str()
        .is_some_and(|error| error.contains("exact current Question revision")));
    assert!(adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical after stale Decision rejection")
        .active_decisions
        .is_empty());

    let decided = structured(&call(
        &mut adapter,
        "decision_record",
        json!({
            "project_id":project,
            "question_id":question_id,
            "question_revision":frontier["questions"][0]["revision"],
            "alternative_key":"local",
            "user_turn":"Choose the local Candidate alternative",
            "user_rationale":"Keep canonical state local"
        }),
    ))
    .clone();
    assert_eq!(decided["all_succeeded"], true, "{decided}");
    let after_decision = adapter
        .operations()
        .canonical_basis(project_id)
        .expect("canonical after explicit response");
    assert_eq!(after_decision.active_decisions.len(), 1);

    let second = structured(&call(
        &mut adapter,
        "candidate_manage",
        question_candidate_arguments(&project, &source.id.to_string(), 2, "Choose cache"),
    ))
    .clone();
    let dismissed = structured(&call(
        &mut adapter,
        "candidate_manage",
        json!({
            "action":"dismiss",
            "project_id":project,
            "candidate_id":second["candidate_id"],
            "reason":"not material to the current work"
        }),
    ))
    .clone();
    assert_eq!(dismissed["disposition"]["state"], "dismissed");
    assert_eq!(
        adapter
            .operations()
            .canonical_basis(project_id)
            .expect("canonical after dismissal")
            .active_questions
            .len(),
        0
    );
}

fn setup_guarded_provider_request(
    expiration_delta_micros: i64,
) -> (tempfile::TempDir, HostAdapter, String, Value) {
    use volicord_context::{Clock, Principal, PrincipalKind, SystemClock};

    let (temporary, mut adapter, project) = setup();
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src")).expect("source directory");
    fs::write(repository.join("src/lib.rs"), "pub fn host_path() {}\n").expect("source file");
    let project_id = parse_project(&project);
    adapter
        .operations()
        .analyze(project_id, Vec::new())
        .expect("analysis");
    let opt_in_source = adapter
        .operations()
        .record_user_source(
            project_id,
            "codex".into(),
            "privacy-host".into(),
            "enable provider".into(),
        )
        .expect("opt-in source");
    adapter
        .operations()
        .enable_provider(
            ProviderOptInPolicy {
                project_id,
                provider: "configured-provider".into(),
                model: "configured-model".into(),
                purpose: "background semantic analysis".into(),
                requested_capability: "semantic".into(),
                allowed_source_scopes: vec!["src/lib.rs".into()],
                exclusions: SourceExclusionPolicy {
                    path_prefixes: Vec::new(),
                    file_classes: Vec::new(),
                    basis: "host cleanup fixture".into(),
                },
                filtering: SecretFilteringPolicy {
                    enabled: false,
                    line_markers: Vec::new(),
                    replacement: "[filtered]".into(),
                    known_limits: Vec::new(),
                },
                retention: ProviderRetentionPolicy {
                    local_annotation_retained_until: None,
                    local_basis: "until explicit deletion".into(),
                    provider_expectation: "configured provider policy".into(),
                    provider_known_limits: Vec::new(),
                },
            },
            ProviderIntentProvenance {
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "current-host-user".into(),
                },
                host: "codex".into(),
                session: "privacy-host".into(),
                user_turn_source: parse_source_identity(&opt_in_source.identity),
                basis: "explicit host fixture opt-in".into(),
            },
        )
        .expect("provider opt-in");
    let now = SystemClock.now().expect("clock");
    let expiration = now
        .as_unix_micros()
        .checked_add(expiration_delta_micros)
        .and_then(|value| u64::try_from(value).ok())
        .expect("positive supported expiration");
    let prepared = call(
        &mut adapter,
        "background_semantic_operation",
        json!({
            "action":"prepare",
            "project_id":project,
            "provider":"configured-provider",
            "model":"configured-model",
            "purpose":"background semantic analysis",
            "requested_capability":"semantic",
            "source_paths":["src/lib.rs"],
            "expiration_unix_micros":expiration
        }),
    );
    assert_eq!(prepared["result"]["isError"], false, "{prepared}");
    let request = structured(&prepared)["guarded_request"].clone();
    (temporary, adapter, project, request)
}

fn question_candidate_arguments(project: &str, source: &str, order: u64, prompt: &str) -> Value {
    json!({
        "action":"submit_question",
        "project_id":project,
        "source_ids":[source],
        "source_operation":"design-review",
        "repository_snapshot":"candidate-host-fixture",
        "research_state":"ready_to_ask",
        "research_state_basis":"the unresolved branch is purely a user judgment",
        "retention_basis":"retain through explicit inquiry disposition",
        "bounded_summary":format!("material Candidate: {prompt}"),
        "prompt":prompt,
        "why_now":"the implementation result depends on this choice",
        "affected_scope":["storage"],
        "established_facts":["Canonical context is local"],
        "assumptions":["the Project remains local-first"],
        "uncertainty":["future scale is unknown"],
        "alternatives":[
            {"key":"local","label":"Local","consequence":"Keep canonical state local"},
            {"key":"remote","label":"Remote","consequence":"Require a separate provider boundary"}
        ],
        "recommendation_key":"local",
        "recommendation_rationale":"matches the local-first contract",
        "trade_offs":["remote augmentation remains separate"],
        "known_limits":["provider behavior is not evaluated"],
        "what_unlocks":["the storage implementation"],
        "materiality_rationale":"the choice changes durable behavior",
        "duplicate_basis":"canonical inspection found no matching Question",
        "presentation_order":order
    })
}

fn call(adapter: &mut HostAdapter, name: &str, arguments: Value) -> Value {
    adapter.handle(json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":name,"arguments":arguments}})).expect("tool response")
}

fn bind_recorded_scope(adapter: &mut HostAdapter, recorded: &Value, paths: &[&str]) -> Value {
    let recorded = recorded
        .get("result")
        .and_then(|result| result.get("structuredContent"))
        .unwrap_or(recorded);
    let project_id = recorded["workflow"]["satisfied_basis_identities"]
        .as_array()
        .and_then(|basis| basis.iter().find(|item| item["kind"] == "project"))
        .and_then(|item| item["identity"].as_str())
        .expect("recorded workflow Project identity");
    call(
        adapter,
        "materiality_review",
        json!({
            "action":"inspect",
            "project_id":project_id,
            "goal_context_id":recorded["goal_context_id"],
            "baseline_analysis_snapshot_id":recorded["baseline_analysis_snapshot_id"],
            "review_candidate_id":recorded["review_candidate_id"],
            "paths":paths,
            "components":[],
            "work_contexts":[],
            "met_revisit_triggers":[],
        }),
    )
}
fn structured(response: &Value) -> &Value {
    &response["result"]["structuredContent"]
}
fn parse_project(value: &str) -> volicord_context::ProjectId {
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] =
            u8::from_str_radix(std::str::from_utf8(pair).expect("hex pair"), 16).expect("hex");
    }
    volicord_context::ProjectId::from_bytes(bytes)
}

fn parse_source_identity(value: &str) -> volicord_context::SourceId {
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] =
            u8::from_str_radix(std::str::from_utf8(pair).expect("hex pair"), 16).expect("hex");
    }
    volicord_context::SourceId::from_bytes(bytes)
}

fn parse_context_identity(value: &str) -> volicord_context::ContextItemId {
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] =
            u8::from_str_radix(std::str::from_utf8(pair).expect("hex pair"), 16).expect("hex");
    }
    volicord_context::ContextItemId::from_bytes(bytes)
}

fn parse_question_identity(value: &str) -> volicord_context::QuestionId {
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] =
            u8::from_str_radix(std::str::from_utf8(pair).expect("hex pair"), 16).expect("hex");
    }
    volicord_context::QuestionId::from_bytes(bytes)
}

fn assert_schema_is_closed_and_described(schema: &Value) {
    if let Some(variants) = schema.get("oneOf").and_then(Value::as_array) {
        assert!(!variants.is_empty());
        for variant in variants {
            assert_schema_is_closed_and_described(variant);
        }
        return;
    }
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    let properties = schema["properties"].as_object().expect("properties");
    assert!(!properties.is_empty());
    for property in properties.values() {
        assert!(property["description"].as_str().is_some());
    }
}

fn schema_shapes(schema: &Value) -> Vec<(BTreeSet<String>, BTreeSet<String>)> {
    let variants = schema
        .get("oneOf")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_else(|| std::slice::from_ref(schema));
    variants
        .iter()
        .map(|variant| {
            let properties = variant["properties"]
                .as_object()
                .expect("properties")
                .keys()
                .cloned()
                .collect();
            let required = variant["required"]
                .as_array()
                .expect("required")
                .iter()
                .map(|value| value.as_str().expect("required name").to_owned())
                .collect();
            (properties, required)
        })
        .collect()
}

fn expected_shapes(name: &str) -> Vec<(BTreeSet<String>, BTreeSet<String>)> {
    let shape = |properties: &[&str], required: &[&str]| {
        (
            properties.iter().map(|value| (*value).to_owned()).collect(),
            required.iter().map(|value| (*value).to_owned()).collect(),
        )
    };
    match name {
        "project_resolve" => vec![shape(&["repository"], &["repository"])],
        "project_initialize" => vec![
            shape(&["display_name", "repository"], &["display_name"]),
            shape(&["repository"], &["repository"]),
        ],
        "project_health" => vec![shape(&["project_id"], &[])],
        "recall"
        | "repository_understanding"
        | "canonical_inspect"
        | "candidate_inspect"
        | "privacy_status" => {
            vec![shape(&["project_id"], &["project_id"])]
        }
        "repository_analyze" => vec![shape(&["project_id", "excluded_paths"], &["project_id"])],
        "engineering_choice_discovery" => vec![shape(
            &[
                "project_id",
                "goal_context_id",
                "baseline_analysis_snapshot_id",
                "source_operation",
                "summary",
                "choices",
                "material_boundary_review",
            ],
            &[
                "project_id",
                "goal_context_id",
                "baseline_analysis_snapshot_id",
                "source_operation",
                "summary",
                "choices",
                "material_boundary_review",
            ],
        )],
        "materiality_review" => vec![
            shape(
                &[
                    "action",
                    "project_id",
                    "engineering_choice_discovery_candidate_id",
                ],
                &[
                    "action",
                    "project_id",
                    "engineering_choice_discovery_candidate_id",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "engineering_choice_discovery_candidate_id",
                    "rationale",
                    "learning_participation",
                    "judgments",
                ],
                &[
                    "action",
                    "project_id",
                    "engineering_choice_discovery_candidate_id",
                    "rationale",
                    "learning_participation",
                    "judgments",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "review_candidate_id",
                    "rationale",
                    "learning_participation",
                    "judgments",
                    "learning_value_revision_bases",
                ],
                &[
                    "action",
                    "project_id",
                    "review_candidate_id",
                    "rationale",
                    "learning_participation",
                    "judgments",
                ],
            ),
            shape(
                &[
                    "project_id",
                    "goal_context_id",
                    "baseline_analysis_snapshot_id",
                    "review_candidate_id",
                    "action",
                    "paths",
                    "components",
                    "work_contexts",
                    "met_revisit_triggers",
                ],
                &[
                    "action",
                    "project_id",
                    "goal_context_id",
                    "baseline_analysis_snapshot_id",
                    "review_candidate_id",
                ],
            ),
        ],
        "learning_deliberation" => vec![
            shape(
                &[
                    "action",
                    "project_id",
                    "review_candidate_id",
                    "dimension_id",
                    "source_operation",
                    "problem",
                    "established_facts",
                ],
                &[
                    "action",
                    "project_id",
                    "review_candidate_id",
                    "dimension_id",
                    "source_operation",
                    "problem",
                    "established_facts",
                ],
            ),
            shape(
                &["action", "project_id", "deliberation_candidate_id"],
                &["action", "project_id", "deliberation_candidate_id"],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "user_turn",
                    "user_rationale",
                    "selections",
                ],
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "user_turn",
                    "selections",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "user_turn",
                    "user_rationale",
                ],
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "user_turn",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "user_turn",
                    "user_rationale",
                ],
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "user_turn",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "user_turn",
                    "user_rationale",
                    "evidence_state",
                ],
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "user_turn",
                    "evidence_state",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "feedback",
                    "recommendation_selections",
                    "recommendation_rationale",
                ],
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "feedback",
                    "recommendation_selections",
                    "recommendation_rationale",
                ],
            ),
            shape(
                &["action", "project_id", "deliberation_candidate_id"],
                &["action", "project_id", "deliberation_candidate_id"],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "user_turn",
                    "rationale",
                ],
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "user_turn",
                    "rationale",
                ],
            ),
        ],
        "background_semantic_operation" => vec![
            shape(
                &[
                    "action",
                    "project_id",
                    "provider",
                    "model",
                    "purpose",
                    "requested_capability",
                    "source_paths",
                    "expiration_unix_micros",
                ],
                &[
                    "action",
                    "project_id",
                    "provider",
                    "model",
                    "purpose",
                    "requested_capability",
                    "source_paths",
                    "expiration_unix_micros",
                ],
            ),
            shape(
                &[
                    "action",
                    "confirmation_request_id",
                    "request_revision",
                    "effect_fingerprint",
                ],
                &[
                    "action",
                    "confirmation_request_id",
                    "request_revision",
                    "effect_fingerprint",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "operation_id",
                    "provider_request_id",
                ],
                &[
                    "action",
                    "project_id",
                    "operation_id",
                    "provider_request_id",
                ],
            ),
        ],
        "inquiry_frontier" => vec![shape(&["project_id", "material_scope"], &["project_id"])],
        "decision_record" => vec![shape(
            &[
                "project_id",
                "question_id",
                "question_revision",
                "alternative_key",
                "user_turn",
                "user_rationale",
            ],
            &[
                "project_id",
                "question_id",
                "question_revision",
                "alternative_key",
                "user_turn",
            ],
        )],
        "context_record" => vec![shape(
            &["project_id", "user_turn", "role", "statement"],
            &["project_id", "user_turn", "role", "statement"],
        )],
        "checkpoint_record" => vec![shape(
            &[
                "project_id",
                "goal_context_id",
                "baseline_analysis_snapshot_id",
                "kind",
                "work_state",
                "state_change",
                "applied_decision_ids",
                "decision_components",
                "work_contexts",
                "met_revisit_triggers",
                "verification",
                "next_step",
                "known_limits",
                "non_goals",
                "handoff_to",
            ],
            &[
                "project_id",
                "goal_context_id",
                "baseline_analysis_snapshot_id",
                "kind",
                "work_state",
                "applied_decision_ids",
                "verification",
                "next_step",
            ],
        )],
        "canonical_mutate" => vec![
            shape(
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "expected_revision",
                    "corrected_text",
                ],
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "expected_revision",
                    "corrected_text",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "expected_revision",
                    "corrected_text",
                ],
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "expected_revision",
                    "corrected_text",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "alternative_key",
                    "rationale",
                ],
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "alternative_key",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "record_kind",
                ],
                &[
                    "action",
                    "project_id",
                    "user_turn",
                    "record_id",
                    "record_kind",
                ],
            ),
        ],
        "candidate_manage" => vec![
            shape(
                &[
                    "action",
                    "project_id",
                    "source_ids",
                    "source_operation",
                    "repository_snapshot",
                    "research_state",
                    "research_state_basis",
                    "retention_basis",
                    "bounded_summary",
                    "prompt",
                    "why_now",
                    "affected_scope",
                    "established_facts",
                    "assumptions",
                    "uncertainty",
                    "alternatives",
                    "recommendation_key",
                    "recommendation_rationale",
                    "trade_offs",
                    "known_limits",
                    "what_unlocks",
                    "materiality_rationale",
                    "duplicate_basis",
                    "presentation_order",
                ],
                &[
                    "action",
                    "project_id",
                    "source_ids",
                    "source_operation",
                    "research_state",
                    "research_state_basis",
                    "retention_basis",
                    "bounded_summary",
                    "prompt",
                    "why_now",
                    "affected_scope",
                    "alternatives",
                    "recommendation_key",
                    "recommendation_rationale",
                    "materiality_rationale",
                    "duplicate_basis",
                    "presentation_order",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "review_candidate_id",
                    "dimension_id",
                    "research_state",
                    "research_state_basis",
                    "retention_basis",
                    "bounded_summary",
                    "prompt",
                    "why_now",
                    "established_facts",
                    "assumptions",
                    "uncertainty",
                    "alternatives",
                    "recommendation_key",
                    "recommendation_rationale",
                    "trade_offs",
                    "known_limits",
                    "what_unlocks",
                    "duplicate_basis",
                    "presentation_order",
                ],
                &[
                    "action",
                    "project_id",
                    "review_candidate_id",
                    "dimension_id",
                    "research_state",
                    "research_state_basis",
                    "retention_basis",
                    "bounded_summary",
                    "prompt",
                    "why_now",
                    "alternatives",
                    "recommendation_key",
                    "recommendation_rationale",
                    "duplicate_basis",
                    "presentation_order",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "candidate_id",
                    "capability",
                    "coverage",
                    "freshness",
                    "source_ids",
                    "evidence_assessment",
                    "limits",
                ],
                &[
                    "action",
                    "project_id",
                    "candidate_id",
                    "capability",
                    "coverage",
                    "freshness",
                    "source_ids",
                    "evidence_assessment",
                ],
            ),
            shape(
                &["action", "project_id", "candidate_id"],
                &["action", "project_id", "candidate_id"],
            ),
            shape(
                &["action", "project_id", "candidate_id"],
                &["action", "project_id", "candidate_id"],
            ),
            shape(
                &["action", "project_id", "candidate_id", "reason"],
                &["action", "project_id", "candidate_id", "reason"],
            ),
            shape(
                &["action", "project_id", "candidate_id", "basis"],
                &["action", "project_id", "candidate_id", "basis"],
            ),
        ],
        "document_preview" => vec![shape(
            &[
                "project_id",
                "kind",
                "format",
                "language",
                "locale",
                "realization",
            ],
            &["project_id", "kind"],
        )],
        "guarded_interaction" => vec![
            shape(&["confirmation_request_id"], &["confirmation_request_id"]),
            shape(
                &[
                    "confirmation_request_id",
                    "request_revision",
                    "effect_fingerprint",
                    "decision",
                    "user_turn",
                ],
                &[
                    "confirmation_request_id",
                    "request_revision",
                    "effect_fingerprint",
                    "decision",
                    "user_turn",
                ],
            ),
        ],
        _ => panic!("unexpected public tool {name}"),
    }
}
