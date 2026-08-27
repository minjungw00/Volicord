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
use volicord_context::{Principal, PrincipalKind, TimestampMicros};
use volicord_host::{run_stdio, HostAdapter, HOST_TOOL_NAMES};
use volicord_inquiry::{
    CandidateCollectionMode, CandidateCollectionScope, CandidateContent, CandidateDraft,
    CandidateKind, CandidateObservationBasis, CandidateOrigin, CandidateRetention, CandidateStore,
    SubmissionOutcome,
};
use volicord_operations::{
    LocalOperations, MaterialityDimension, MaterialityDisposition, MaterialityReviewDraft,
    RuntimeLayout, WorkAuthorityBasis, WorkAuthorityBasisKind,
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
                materiality_review: None,
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

#[test]
fn mcp_workflow_guides_material_question_to_explicit_decision_and_ready_work() {
    let (_temporary, mut adapter, project) = setup();
    let goal = call(
        &mut adapter,
        "context_record",
        json!({
            "project_id":project,
            "user_turn":"Decide the user-visible failure mode, then implement it",
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

    let analyzed = call(
        &mut adapter,
        "repository_analyze",
        json!({"project_id":project}),
    );
    let analyzed = structured(&analyzed);
    assert_eq!(analyzed["workflow"]["stage"], "materiality_review");
    assert_eq!(analyzed["workflow"]["disposition"], "review_missing");
    assert_eq!(
        analyzed["workflow"]["required_next_action"]["action"],
        "record"
    );
    let baseline = analyzed["analysis_snapshot_id"]
        .as_str()
        .expect("baseline identity");
    let repository_source_id = analyzed["repository_source_id"]
        .as_str()
        .expect("repository Source identity");

    let review = call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"record",
            "project_id":project,
            "goal_context_id":goal_context_id,
            "baseline_analysis_snapshot_id":baseline,
            "source_operation":"MCP pre-work outcome review",
            "rationale":"The failure mode is independently user-visible and user-owned.",
            "dimensions":[{
                "dimension_id":"failure-mode",
                "summary":"Choose the user-visible failure mode",
                "affected_scope":["mcp-errors"],
                "material_consequences":["Changes observable failure behavior"],
                "observable_signals":["observable_failure_policy"],
                "disposition":"unresolved_user_owned_outcome",
                "basis":{
                    "kinds":["agent_recommendation"],
                    "summary":"No accepted authority selects the outcome",
                    "source_ids":[repository_source_id]
                }
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

    let revised = call(
        &mut adapter,
        "materiality_review",
        json!({
            "action":"revise",
            "project_id":project,
            "review_candidate_id":review_id,
            "rationale":"The explicit current-host response now resolves the outcome.",
            "dimensions":[{
                "dimension_id":"failure-mode",
                "summary":"Choose the user-visible failure mode",
                "affected_scope":["mcp-errors"],
                "material_consequences":["Changes observable failure behavior"],
                "observable_signals":["observable_failure_policy"],
                "disposition":"unresolved_user_owned_outcome",
                "resolution_decision_id":decision_id,
                "basis":{
                    "kinds":["applicable_decision"],
                    "summary":"The current explicit Decision resolves this dimension",
                    "source_ids":[repository_source_id],
                    "decision_ids":[decision_id]
                }
            }]
        }),
    );
    let revised = structured(&revised);
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
        "materiality_review"
    );
    assert_eq!(
        refused["details"]["workflow"]["disposition"],
        "review_missing"
    );
    assert_eq!(
        refused["details"]["workflow"]["required_next_action"],
        json!({"tool":"materiality_review","action":"record"})
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
    assert!(descriptions["repository_analyze"].contains("authorized local repository"));
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
        .is_some_and(|error| error.contains("does not match any allowed shape")));
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
            "does not match any allowed shape",
        ),
        (
            "document_preview",
            json!({"project_id":project,"kind":"handoff-resume","format":"pdf"}),
            "is not an allowed value",
        ),
        (
            "guarded_interaction",
            json!({"confirmation_request_id":"00000000000000000000000000000000","decision":"confirm"}),
            "does not match any allowed shape",
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
    adapter
        .operations()
        .record_materiality_review(MaterialityReviewDraft {
            project_id,
            goal_context_id: goal_context,
            baseline_analysis_snapshot_id:
                volicord_repository_intelligence::AnalysisSnapshotId::from_hex(&baseline_id)
                    .expect("typed baseline identity"),
            session: "mcp-checkpoint-fixture".into(),
            source_operation: "pre-work-review".into(),
            rationale: "the host fixture follows its accepted grounded-checkpoint contract".into(),
            dimensions: vec![MaterialityDimension {
                dimension_id: "grounded-checkpoint-contract".into(),
                summary: "grounded Checkpoint behavior".into(),
                affected_scope: vec!["host-checkpoint".into()],
                material_consequences: vec!["records bounded work and truthful evidence".into()],
                observable_signals: Vec::new(),
                disposition: MaterialityDisposition::SettledAuthority,
                basis: WorkAuthorityBasis {
                    kinds: vec![WorkAuthorityBasisKind::AcceptedContract],
                    summary: "accepted source-grounded Checkpoint contract".into(),
                    source_basis: vec![parse_source_identity(
                        baseline["repository_source_id"]
                            .as_str()
                            .expect("baseline repository Source"),
                    )],
                    contract_basis: vec!["rebuild/docs/design/inquiry-and-decision.md".into()],
                    decision_basis: Vec::new(),
                    research_basis: Vec::new(),
                },
            }],
        })
        .expect("pre-work Materiality Review");

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
        .contains("does not match any allowed shape"));
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
        .contains("does not match any allowed shape"));

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
        "materiality_review" => vec![
            shape(
                &[
                    "project_id",
                    "goal_context_id",
                    "baseline_analysis_snapshot_id",
                    "action",
                    "source_operation",
                    "rationale",
                    "dimensions",
                ],
                &[
                    "action",
                    "project_id",
                    "goal_context_id",
                    "baseline_analysis_snapshot_id",
                    "source_operation",
                    "rationale",
                    "dimensions",
                ],
            ),
            shape(
                &[
                    "action",
                    "project_id",
                    "review_candidate_id",
                    "rationale",
                    "dimensions",
                ],
                &[
                    "action",
                    "project_id",
                    "review_candidate_id",
                    "rationale",
                    "dimensions",
                ],
            ),
            shape(
                &[
                    "project_id",
                    "goal_context_id",
                    "baseline_analysis_snapshot_id",
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
