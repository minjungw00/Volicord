use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error as StdError,
    fmt,
    io::{BufRead, Write},
    path::PathBuf,
};
use volicord_context::{
    AgentRecommendation, ApplicabilityScope, CanonicalRecordId, CheckpointId, CheckpointKind,
    Clock, CommandTermination, ContextItemCorrectionDraft, ContextItemId, ContextItemRole,
    CorrectionKind, DecisionCorrectionDraft, DecisionId, NonUserQuestionOutcome, OperationId,
    Principal, PrincipalKind, ProjectId, QuestionAlternative, QuestionEstablishedFact,
    QuestionEvidenceFreshness, QuestionId, QuestionResearchState, SourceId, SystemClock,
    TimestampMicros, VerificationState, WorkState,
};
use volicord_inquiry::{
    BatchResponseItem, CandidateCollectionMode, CandidateCollectionScope, CandidateContent,
    CandidateDisposition, CandidateDraft, CandidateFreshness, CandidateId, CandidateKind,
    CandidateObservationBasis, CandidateOrigin, CandidateRetention, CurrentHostResponse,
    DisplayedQuestion, DuplicateAssessment, EngineeringAlternative, EngineeringChoice,
    EngineeringChoiceEvidenceState, EngineeringChoiceRelationship, EngineeringEffectCategory,
    LearningAlternativeSelection, LearningDeliberation, LearningDeliberationState,
    LearningInitialResponse, LearningParticipation, LearningRecommendation,
    LearningValueAssessment, MaterialityAssessment, MaterialityStatus, QuestionCandidate,
    ResponseMapping, SubmissionOutcome,
};
use volicord_operations::{
    AnalysisSnapshotId, BackgroundProviderOperationDraft, CandidateRepositoryResearchDraft,
    CommandVerificationDraft, ConfirmationDecision, ConfirmationRejection, ConfirmationRequestId,
    EngineeringChoiceDiscoveryDraft, ExploratoryDisposition, FilterOutcome,
    GroundedCheckpointDraft, GuardedOperationId, GuardedOperationOutcome,
    GuardedProviderInspection, GuardedProviderPreparation, GuardedProviderPreparationOutcome,
    HealthState, LearningDeliberationDraft, LearningFeedbackDraft, LearningReconsiderationDraft,
    LearningResponseDraft, LocalOperations, MaterialOutcomeSignal, MaterialityDimension,
    MaterialityDisposition, MaterialityReviewDraft, MaterialityReviewRevisionDraft,
    ProjectResolution, ProviderRequestId, ProviderRequestOutcome, ProviderRequestRecord,
    RequestingProvenance, ScopeOutcome, SourceClass, TransmissionOutcome, WorkAuthorityBasis,
    WorkAuthorityBasisKind, WorkflowDirective, WorkflowDisposition, WorkflowStage,
};
use volicord_projections::{
    CandidateDependencyState, DocumentKind, DocumentRequest, FixedLocale, GeneratorIdentity,
    NarrativePlan, NarrativeRealization, NarrativeRealizationState, OutputFormat,
    RealizedNarrativeClaim, RealizedNarrativeSection,
};

pub const HOST_TOOL_NAMES: [&str; 21] = [
    "project_resolve",
    "project_initialize",
    "project_health",
    "recall",
    "repository_understanding",
    "repository_analyze",
    "engineering_choice_discovery",
    "materiality_review",
    "learning_deliberation",
    "inquiry_frontier",
    "decision_record",
    "context_record",
    "checkpoint_record",
    "canonical_inspect",
    "canonical_mutate",
    "candidate_inspect",
    "candidate_manage",
    "privacy_status",
    "background_semantic_operation",
    "document_preview",
    "guarded_interaction",
];

fn server_instructions() -> String {
    "Volicord is active. Project-scoped repository work starts with project_resolve. Follow workflow.required_next_action; do not bypass a blocking workflow transition. At Materiality Review, learning participation or an implementation selection does not establish user-owned authority. Active deliberation-worthy learning on agent-owned or delegated choices uses learning_deliberation, not Question/decision_record. Genuine user-owned material outcomes require Question/Decision and an explicit response from the current host. Background transmission requires separate exact authorization. Checkpoints report only actually observed command outcomes. Non-project requests need no ceremony.".into()
}

#[derive(Debug)]
pub struct HostError {
    message: String,
    details: Option<Value>,
}

impl HostError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            details: None,
        }
    }

    fn with_details(message: impl Into<String>, details: Value) -> Self {
        Self {
            message: message.into(),
            details: Some(details),
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for HostError {}

pub struct HostAdapter {
    operations: LocalOperations,
    initialized: bool,
    client_supports_elicitation: bool,
    host_session: String,
    pending_provider_operations: BTreeMap<ConfirmationRequestId, GuardedProviderPreparation>,
}

impl HostAdapter {
    pub fn new(operations: LocalOperations) -> Self {
        Self {
            operations,
            initialized: false,
            client_supports_elicitation: false,
            host_session: new_identity_text().unwrap_or_else(|_| "unavailable-session".into()),
            pending_provider_operations: BTreeMap::new(),
        }
    }

    pub fn operations(&self) -> &LocalOperations {
        &self.operations
    }

    pub fn handle(&mut self, message: Value) -> Option<Value> {
        self.cleanup_expired_provider_operations();
        let method = message.get("method").and_then(Value::as_str)?;
        let id = message.get("id").cloned();
        if id.is_none() {
            if method == "notifications/initialized" {
                self.initialized = true;
            }
            return None;
        }
        let id = id.unwrap_or(Value::Null);
        let response = match method {
            "initialize" => self.initialize(message.get("params")),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tool_catalog()})),
            "tools/call" => self.call_tool(message.get("params")),
            _ => return Some(rpc_error(id, -32601, "method not found")),
        };
        Some(match response {
            Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
            Err(error) => rpc_error(id, -32602, &error.to_string()),
        })
    }

    fn initialize(&mut self, params: Option<&Value>) -> Result<Value, HostError> {
        self.client_supports_elicitation = params
            .and_then(|value| value.get("capabilities"))
            .and_then(|value| value.get("elicitation"))
            .is_some();
        Ok(json!({
            "protocolVersion": params.and_then(|value| value.get("protocolVersion")).and_then(Value::as_str).unwrap_or("2025-06-18"),
            "capabilities":{"tools":{"listChanged":false}},
            "serverInfo":{"name":"volicord","version":env!("CARGO_PKG_VERSION")},
            "instructions":server_instructions()
        }))
    }

    fn call_tool(&mut self, params: Option<&Value>) -> Result<Value, HostError> {
        let params = params.ok_or_else(|| HostError::new("tools/call params are required"))?;
        let name = required_str(params, "name")?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let result = match tool_contract(name) {
            Some(contract) => contract.validate(&arguments).and_then(|()| match name {
                "project_resolve" => self.project_resolve(&arguments),
                "project_initialize" => self.project_initialize(&arguments),
                "project_health" => self.project_health(&arguments),
                "recall" => self.recall(&arguments),
                "repository_understanding" => self.repository_understanding(&arguments),
                "repository_analyze" => self.repository_analyze(&arguments),
                "engineering_choice_discovery" => self.engineering_choice_discovery(&arguments),
                "materiality_review" => self.materiality_review(&arguments),
                "learning_deliberation" => self.learning_deliberation(&arguments),
                "inquiry_frontier" => self.inquiry_frontier(&arguments),
                "decision_record" => self.decision_record(&arguments),
                "context_record" => self.context_record(&arguments),
                "checkpoint_record" => self.checkpoint_record(&arguments),
                "canonical_inspect" => self.canonical_inspect(&arguments),
                "canonical_mutate" => self.canonical_mutate(&arguments),
                "candidate_inspect" => self.candidate_inspect(&arguments),
                "candidate_manage" => self.candidate_manage(&arguments),
                "privacy_status" => self.privacy_status(&arguments),
                "background_semantic_operation" => self.background_semantic_operation(&arguments),
                "document_preview" => self.document_preview(&arguments),
                "guarded_interaction" => self.guarded_interaction(&arguments),
                _ => Err(HostError::new("tool contract has no handler")),
            }),
            None => Err(HostError::new("unknown high-level tool")),
        };
        match result {
            Ok(value) => Ok(tool_result(value, false)),
            Err(mut error) => {
                if name == "materiality_review" && error.details.is_none() {
                    error.details = Some(json!({
                        "diagnostic":"materiality_contract_failure",
                        "problem":error.to_string(),
                        "bound_identities":{
                            "project_id":arguments.get("project_id"),
                            "engineering_choice_discovery_candidate_id":arguments.get("engineering_choice_discovery_candidate_id"),
                            "materiality_review_candidate_id":arguments.get("review_candidate_id"),
                        },
                        "missing_prerequisite_or_evidence":"The problem names the failed typed invariant; call draft to recover current identities and disposition contracts.",
                        "next_supported_action":{"tool":"materiality_review","action":"draft"},
                    }));
                }
                let mut payload = json!({"error":error.to_string()});
                if let (Some(object), Some(details)) = (payload.as_object_mut(), error.details) {
                    object.insert("details".into(), details);
                }
                Ok(tool_result(payload, true))
            }
        }
    }

    fn cleanup_expired_provider_operations(&mut self) {
        let Ok(now) = SystemClock.now() else {
            return;
        };
        self.pending_provider_operations
            .retain(|_, preparation| now < preparation.candidate.expires_at);
    }

    fn project_initialize(&self, args: &Value) -> Result<Value, HostError> {
        let repository = args
            .get("repository")
            .and_then(Value::as_str)
            .map(PathBuf::from);
        let display_name = optional_string(args, "display_name")?;
        let value = match (display_name, repository.as_deref()) {
            (Some(display_name), repository) => {
                self.operations.initialize_project(display_name, repository)
            }
            (None, Some(repository)) => self
                .operations
                .initialize_project_from_repository(repository),
            (None, None) => return Err(HostError::new("display_name or repository is required")),
        }
        .map_err(operation_error)?;
        let workflow = LocalOperations::workflow_after_initialization(value.project.id);
        Ok(with_workflow(
            json!({"project_id":value.project.id.to_string(),"display_name":value.project.display_name,"binding":value.binding.map(|binding| binding.binding.absolute_path)}),
            workflow,
        ))
    }

    fn project_resolve(&self, args: &Value) -> Result<Value, HostError> {
        let value = self
            .operations
            .resolve_project(&PathBuf::from(required_str(args, "repository")?))
            .map_err(operation_error)?;
        Ok(match value {
            ProjectResolution::Found { project, binding } => with_workflow(
                json!({
                    "status":"found",
                    "project_id":project.id.to_string(),
                    "display_name":project.display_name,
                    "project_revision":project.revision,
                    "binding":{
                        "binding_id":binding.binding.id.to_string(),
                        "revision":binding.binding.revision,
                        "canonical_repository_path":binding.binding.absolute_path,
                        "availability":format!("{:?}", binding.binding.availability).to_lowercase(),
                        "clone_identity":binding.clone_identity,
                        "worktree_identity":binding.worktree_identity,
                    }
                }),
                LocalOperations::workflow_project_found(project.id),
            ),
            ProjectResolution::NotFound {
                canonical_repository_path,
            } => with_workflow(
                json!({
                    "status":"not_found",
                    "canonical_repository_path":canonical_repository_path,
                }),
                LocalOperations::workflow_project_not_found(),
            ),
        })
    }

    fn project_health(&self, args: &Value) -> Result<Value, HostError> {
        let project = optional_project(args, "project_id")?;
        let report = self.operations.health(project);
        Ok(json!({
            "connection":"connected",
            "capability_state":match report.state { HealthState::Healthy=>"healthy",HealthState::Degraded=>"degraded",HealthState::Failed=>"failed" },
            "runtime_root":report.runtime_root,
            "canonical_available":report.canonical_available,
            "repository_available":report.repository_available,
            "issues":report.issues.into_iter().map(|issue| json!({"kind":format!("{:?}",issue.kind).to_lowercase(),"scope":issue.scope,"detail":issue.detail})).collect::<Vec<_>>()
        }))
    }

    fn recall(&self, args: &Value) -> Result<Value, HostError> {
        let project_id = project(args)?;
        let brief = self
            .operations
            .recall(project_id)
            .map_err(operation_error)?;
        let (learning_context, learning_context_health) =
            match self.operations.project_projection(project_id) {
                Ok(projection) => (
                    projection
                        .candidate_inspection
                        .into_iter()
                        .filter(|candidate| candidate.learning_deliberation.is_some())
                        .take(64)
                        .map(candidate_inspection_json)
                        .collect::<Vec<_>>(),
                    json!({"state":"available"}),
                ),
                Err(error) => (
                    Vec::new(),
                    json!({"state":"degraded","reason":error.to_string()}),
                ),
            };
        let checkpoint = brief.latest_meaningful_checkpoint.map(|value| json!({
            "identity":value.id.to_string(),
            "revision":value.revision,
            "kind":checkpoint_kind_name(value.kind),
            "goal":value.goal,
            "work_state":work_state_name(value.work_state),
            "state_change":value.state_change,
            "source_basis":value.source_basis.into_iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            "changed_source_basis":value.changed_source_basis.into_iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            "changed_paths":value.changed_paths,
            "applied_decisions":value.applied_decisions.into_iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            "verification":value.verification.into_iter().map(|fact| json!({"state":verification_state_name(fact.state),"source_id":fact.source_id.map(|id| id.to_string()),"outcome":fact.outcome})).collect::<Vec<_>>(),
            "user_review":{"state":user_review_state_name(value.user_review.state),"source_id":value.user_review.source_id.map(|id| id.to_string())},
            "user_acceptance":{"state":user_acceptance_state_name(value.user_acceptance.state),"source_id":value.user_acceptance.source_id.map(|id| id.to_string())},
            "known_limits":value.known_limits,
            "non_goals":value.non_goals,
            "open_questions":value.open_questions.into_iter().map(|question| json!({"identity":question.question_id.to_string(),"revision":question.revision})).collect::<Vec<_>>(),
            "next_step":value.next_step,
            "handoff_to":value.handoff_to,
            "recorded_at_unix_micros":value.recorded_at.as_unix_micros(),
        }));
        let workflow = self
            .operations
            .workflow_after_recall(brief.project_id)
            .map_err(operation_error)?;
        Ok(with_workflow(
            json!({
                "project_id":brief.project_id.to_string(),"project_name":brief.project_name,
                "goals":brief.goals_and_why.into_iter().map(|value| value.statement).collect::<Vec<_>>(),
                "decisions":brief.decisions.into_iter().map(|value| json!({"identity":value.decision_id.to_string(),"revision":value.revision,"state":format!("{:?}",value.state).to_lowercase(),"choice":format!("{:?}",value.choice),"rationale":value.user_rationale})).collect::<Vec<_>>(),
                "open_questions":brief.open_questions.into_iter().map(|value| json!({"identity":value.question_id.to_string(),"revision":value.revision,"prompt":value.prompt})).collect::<Vec<_>>(),
                "known_limits":brief.known_limits,"next_step":brief.next_meaningful_step,"checkpoint":checkpoint,"omitted_count":brief.omitted_count,
                "learning_context":learning_context,
                "learning_context_health":learning_context_health,
                "read_only":true
            }),
            workflow,
        ))
    }

    fn repository_understanding(&self, args: &Value) -> Result<Value, HostError> {
        let projection = self
            .operations
            .project_projection(project(args)?)
            .map_err(operation_error)?;
        Ok(json!({
            "health":format!("{:?}",projection.health).to_lowercase(),
            "candidate_dependency":candidate_dependency_key(projection.candidate_dependency),
            "overview":{"name":projection.overview.project_name,"goals":projection.overview.current_goals,"active_decisions":projection.overview.active_decision_count,"open_questions":projection.overview.open_question_count},
            "repository_map":{"entity_count":projection.repository_map.entities.len(),"relation_count":projection.repository_map.relations.len(),"entities":projection.repository_map.entities.into_iter().take(64).map(|value| json!({"identity":value.identity,"name":value.display_name,"kind":format!("{:?}",value.kind),"language":format!("{:?}",value.language),"source_id":value.source_id.to_string(),"freshness":format!("{:?}",value.freshness.state)})).collect::<Vec<_>>(),"gaps":projection.repository_map.gaps.into_iter().map(|value| json!({"state":format!("{:?}",value.state).to_lowercase(),"capability":format!("{:?}",value.capability).to_lowercase(),"area":value.area,"reason":value.reason})).collect::<Vec<_>>()},
            "decision_context_code":projection.decision_context_code.into_iter().map(|value| json!({"decision_id":value.decision_id.to_string(),"revision":value.decision_revision,"paths":value.declared_paths,"code_entities":value.related_code_entities,"uncertainty":value.missing_or_uncertain_links})).collect::<Vec<_>>(),
            "issues":projection.issues.into_iter().map(|value| json!({"kind":format!("{:?}",value.kind).to_lowercase(),"scope":value.affected_scope,"reason":value.reason})).collect::<Vec<_>>(),
            "read_only":true
        }))
    }

    fn repository_analyze(&self, args: &Value) -> Result<Value, HostError> {
        let project_id = project(args)?;
        let excludes = args
            .get("excluded_paths")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let result = self
            .operations
            .analyze(project_id, excludes)
            .map_err(operation_error)?;
        let analysis_snapshot_id = result
            .value
            .as_ref()
            .map(|value| value.analysis.identity.to_string());
        let repository_snapshot_id = result
            .value
            .as_ref()
            .map(|value| value.repository.identity.to_string());
        let repository_source_id = result
            .value
            .as_ref()
            .map(|value| value.analysis.repository_source.identity().to_string());
        let workflow = match result.value.as_ref() {
            Some(value) => self
                .operations
                .workflow_after_analysis(project_id, value.analysis.identity)
                .map_err(operation_error)?,
            None => self
                .operations
                .workflow_after_recall(project_id)
                .map_err(operation_error)?,
        };
        Ok(with_workflow(
            json!({"project_id":project_id.to_string(),"operation_id":result.operation_id.to_string(),"state":format!("{:?}",result.state).to_lowercase(),"duration_micros":result.duration_micros,"analysis_snapshot_id":analysis_snapshot_id,"repository_snapshot_id":repository_snapshot_id,"repository_source_id":repository_source_id,"completed_scopes":result.partial.completed_scopes,"failed_scopes":result.partial.failed_scopes,"omitted_scopes":result.partial.omitted_scopes,"diagnostic":result.diagnostic}),
            workflow,
        ))
    }

    fn engineering_choice_discovery(&self, args: &Value) -> Result<Value, HostError> {
        let project_id = project(args)?;
        let goal_context_id = parse_context_item(required_str(args, "goal_context_id")?)?;
        let baseline_analysis_snapshot_id =
            parse_analysis_snapshot(required_str(args, "baseline_analysis_snapshot_id")?)?;
        let outcome = self
            .operations
            .record_engineering_choice_discovery(EngineeringChoiceDiscoveryDraft {
                project_id,
                goal_context_id,
                baseline_analysis_snapshot_id,
                session: self.host_session.clone(),
                source_operation: required_str(args, "source_operation")?.to_owned(),
                summary: required_str(args, "summary")?.to_owned(),
                choices: engineering_choices(args)?,
            })
            .map_err(operation_error)?;
        let workflow = self
            .operations
            .workflow_for_work_basis(
                project_id,
                goal_context_id,
                baseline_analysis_snapshot_id,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
            .map_err(operation_error)?;
        Ok(with_workflow(
            json!({
                "action":"record",
                "project_id":project_id.to_string(),
                "goal_context_id":outcome.goal_context_id.to_string(),
                "baseline_analysis_snapshot_id":outcome.baseline_analysis_snapshot_id.to_string(),
                "discovery_candidate_id":outcome.discovery_candidate_id.to_string(),
                "canonical_mutation":false,
            }),
            workflow,
        ))
    }

    fn materiality_review(&self, args: &Value) -> Result<Value, HostError> {
        let project_id = project(args)?;
        match required_str(args, "action")? {
            "draft" => {
                let candidate_id = parse_candidate(required_str(
                    args,
                    "engineering_choice_discovery_candidate_id",
                )?)?;
                let candidate = self
                    .operations
                    .inspect_workflow_candidate(project_id, candidate_id)
                    .map_err(operation_error)?;
                let discovery = candidate
                    .content
                    .as_ref()
                    .and_then(|content| content.engineering_choice_discovery.as_ref())
                    .ok_or_else(|| {
                        HostError::new("Engineering Choice Discovery content is unavailable")
                    })?;
                let canonical = self
                    .operations
                    .canonical_basis(project_id)
                    .map_err(operation_error)?;
                let current_review = self
                    .operations
                    .candidate_basis(project_id)
                    .map_err(operation_error)?
                    .candidates
                    .into_iter()
                    .filter(|candidate| {
                        candidate.kind == CandidateKind::MaterialityReview
                            && candidate.content.as_ref().is_some_and(|content| {
                                content.materiality_review.as_ref().is_some_and(|review| {
                                    review.engineering_choice_discovery_candidate_id == candidate_id
                                })
                            })
                    })
                    .max_by_key(|candidate| {
                        (candidate.revision, candidate.created_at, candidate.id)
                    })
                    .map(|candidate| candidate.id);
                Ok(materiality_draft_json(
                    project_id,
                    candidate_id,
                    discovery,
                    &canonical,
                    current_review,
                ))
            }
            "record" => {
                let discovery_candidate_id = parse_candidate(required_str(
                    args,
                    "engineering_choice_discovery_candidate_id",
                )?)?;
                let discovery_candidate = self
                    .operations
                    .inspect_workflow_candidate(project_id, discovery_candidate_id)
                    .map_err(operation_error)?;
                let discovery = discovery_candidate
                    .content
                    .as_ref()
                    .and_then(|content| content.engineering_choice_discovery.as_ref())
                    .ok_or_else(|| {
                        HostError::new("Engineering Choice Discovery content is unavailable")
                    })?;
                let canonical = self
                    .operations
                    .canonical_basis(project_id)
                    .map_err(operation_error)?;
                let dimensions = materiality_judgments(args, discovery, &canonical)?;
                let outcome =
                    self.operations
                        .record_materiality_review(MaterialityReviewDraft {
                            project_id,
                            goal_context_id: discovery.goal_context_id,
                            baseline_analysis_snapshot_id: discovery
                                .baseline_analysis_snapshot_id,
                            session: self.host_session.clone(),
                            source_operation: format!(
                                "MCP Materiality Review for Engineering Choice Discovery {discovery_candidate_id}"
                            ),
                            rationale: required_str(args, "rationale")?.to_owned(),
                            learning_participation: learning_participation(args)?,
                            engineering_choice_discovery_candidate_id: discovery_candidate_id,
                            dimensions,
                        })
                        .map_err(operation_error)?;
                let workflow = self
                    .operations
                    .workflow_for_review_candidate(project_id, outcome.review_candidate_id)
                    .map_err(operation_error)?;
                Ok(with_workflow(
                    materiality_review_outcome_json("record", outcome),
                    workflow,
                ))
            }
            "revise" => {
                let review_candidate_id =
                    parse_candidate(required_str(args, "review_candidate_id")?)?;
                let review_candidate = self
                    .operations
                    .inspect_workflow_candidate(project_id, review_candidate_id)
                    .map_err(operation_error)?;
                let review = review_candidate
                    .content
                    .as_ref()
                    .and_then(|content| content.materiality_review.as_ref())
                    .ok_or_else(|| HostError::new("Materiality Review content is unavailable"))?;
                let discovery_candidate = self
                    .operations
                    .inspect_workflow_candidate(
                        project_id,
                        review.engineering_choice_discovery_candidate_id,
                    )
                    .map_err(operation_error)?;
                let discovery = discovery_candidate
                    .content
                    .as_ref()
                    .and_then(|content| content.engineering_choice_discovery.as_ref())
                    .ok_or_else(|| {
                        HostError::new("Engineering Choice Discovery content is unavailable")
                    })?;
                let canonical = self
                    .operations
                    .canonical_basis(project_id)
                    .map_err(operation_error)?;
                let dimensions = materiality_judgments(args, discovery, &canonical)?;
                let outcome = self
                    .operations
                    .revise_materiality_review(MaterialityReviewRevisionDraft {
                        project_id,
                        review_candidate_id,
                        rationale: required_str(args, "rationale")?.to_owned(),
                        learning_participation: learning_participation(args)?,
                        dimensions,
                    })
                    .map_err(operation_error)?;
                let workflow = self
                    .operations
                    .workflow_for_review_candidate(project_id, outcome.review_candidate_id)
                    .map_err(operation_error)?;
                Ok(with_workflow(
                    materiality_review_outcome_json("revise", outcome),
                    workflow,
                ))
            }
            "inspect" => {
                let goal_context_id = parse_context_item(required_str(args, "goal_context_id")?)?;
                let baseline_analysis_snapshot_id =
                    parse_analysis_snapshot(required_str(args, "baseline_analysis_snapshot_id")?)?;
                let workflow = self
                    .operations
                    .workflow_for_work_basis(
                        project_id,
                        goal_context_id,
                        baseline_analysis_snapshot_id,
                        string_array(args, "paths")?,
                        string_array(args, "components")?,
                        string_array(args, "work_contexts")?,
                        string_array(args, "met_revisit_triggers")?,
                    )
                    .map_err(operation_error)?;
                Ok(with_workflow(
                    json!({
                        "action":"inspect",
                        "project_id":project_id.to_string(),
                        "goal_context_id":goal_context_id.to_string(),
                        "baseline_analysis_snapshot_id":baseline_analysis_snapshot_id.to_string(),
                        "read_only":true,
                    }),
                    workflow,
                ))
            }
            _ => Err(HostError::new("unknown Materiality Review action")),
        }
    }

    fn learning_deliberation(&self, args: &Value) -> Result<Value, HostError> {
        let project_id = project(args)?;
        let action = required_str(args, "action")?;
        let candidate_id = match action {
            "begin" => {
                self.operations
                    .begin_learning_deliberation(LearningDeliberationDraft {
                        project_id,
                        review_candidate_id: parse_candidate(required_str(
                            args,
                            "review_candidate_id",
                        )?)?,
                        dimension_id: required_str(args, "dimension_id")?.to_owned(),
                        session: self.host_session.clone(),
                        source_operation: required_str(args, "source_operation")?.to_owned(),
                        problem: required_str(args, "problem")?.to_owned(),
                        established_facts: string_array(args, "established_facts")?,
                    })
                    .map_err(operation_error)?
                    .deliberation_candidate_id
            }
            "inspect" => parse_candidate(required_str(args, "deliberation_candidate_id")?)?,
            "respond_select"
            | "respond_delegate"
            | "respond_skip"
            | "respond_research_or_prototype" => {
                let deliberation_candidate_id =
                    parse_candidate(required_str(args, "deliberation_candidate_id")?)?;
                let response = match action {
                    "respond_select" => LearningInitialResponse::Select {
                        selections: learning_selections(args, "selections")?,
                    },
                    "respond_delegate" => LearningInitialResponse::DelegateToAgent,
                    "respond_skip" => LearningInitialResponse::Skip,
                    "respond_research_or_prototype" => {
                        LearningInitialResponse::RequestResearchOrPrototype {
                            evidence_state: engineering_evidence_state(required_str(
                                args,
                                "evidence_state",
                            )?)?,
                        }
                    }
                    _ => {
                        return Err(HostError::new(
                            "unknown Learning Deliberation response action",
                        ))
                    }
                };
                self.operations
                    .record_learning_response(LearningResponseDraft {
                        project_id,
                        deliberation_candidate_id,
                        host: "codex".into(),
                        session: self.host_session.clone(),
                        user_turn: required_str(args, "user_turn")?.to_owned(),
                        response,
                        user_rationale: optional_string(args, "user_rationale")?,
                    })
                    .map_err(operation_error)?
                    .deliberation_candidate_id
            }
            "feedback" => {
                let deliberation_candidate_id =
                    parse_candidate(required_str(args, "deliberation_candidate_id")?)?;
                self.operations
                    .provide_learning_feedback(LearningFeedbackDraft {
                        project_id,
                        deliberation_candidate_id,
                        feedback: required_str(args, "feedback")?.to_owned(),
                        recommendation: LearningRecommendation {
                            selections: learning_selections(args, "recommendation_selections")?,
                            rationale: required_str(args, "recommendation_rationale")?.to_owned(),
                        },
                    })
                    .map_err(operation_error)?
                    .deliberation_candidate_id
            }
            "complete" => {
                let deliberation_candidate_id =
                    parse_candidate(required_str(args, "deliberation_candidate_id")?)?;
                self.operations
                    .complete_learning_deliberation(project_id, deliberation_candidate_id)
                    .map_err(operation_error)?
                    .deliberation_candidate_id
            }
            "reconsider" => {
                let deliberation_candidate_id =
                    parse_candidate(required_str(args, "deliberation_candidate_id")?)?;
                self.operations
                    .reconsider_learning_deliberation(LearningReconsiderationDraft {
                        project_id,
                        deliberation_candidate_id,
                        host: "codex".into(),
                        session: self.host_session.clone(),
                        user_turn: required_str(args, "user_turn")?.to_owned(),
                        rationale: required_str(args, "rationale")?.to_owned(),
                    })
                    .map_err(operation_error)?
                    .deliberation_candidate_id
            }
            _ => return Err(HostError::new("unknown Learning Deliberation action")),
        };
        let candidate = self
            .operations
            .inspect_workflow_candidate(project_id, candidate_id)
            .map_err(operation_error)?;
        let deliberation = candidate
            .content
            .as_ref()
            .and_then(|content| content.learning_deliberation.as_ref())
            .ok_or_else(|| HostError::new("Learning Deliberation content is unavailable"))?;
        let workflow = self
            .operations
            .workflow_for_review_candidate(project_id, deliberation.materiality_review_candidate_id)
            .map_err(operation_error)?;
        Ok(with_workflow(
            learning_deliberation_json(action, candidate_id, candidate.revision, deliberation),
            workflow,
        ))
    }

    fn inquiry_frontier(&self, args: &Value) -> Result<Value, HostError> {
        let project_id = project(args)?;
        let scope = args
            .get("material_scope")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let value = self
            .operations
            .inquiry_frontier(project_id, scope)
            .map_err(operation_error)?;
        let first_question_id = value.questions.first().map(|question| question.question_id);
        let response = json!({"questions":value.questions.into_iter().map(|question| json!({"identity":question.question_id.to_string(),"revision":question.displayed_revision,"prompt":question.prompt_basis,"why_now":question.why_it_matters_now,"alternatives":question.alternatives.into_iter().map(|alternative| json!({"key":alternative.key,"label":alternative.label,"consequence":alternative.consequence})).collect::<Vec<_>>(),"recommendation":question.recommendation.alternative_key,"what_unlocks":question.what_the_answer_unlocks})).collect::<Vec<_>>(),"diagnostics":value.diagnostics.into_iter().map(|diagnostic| diagnostic.detail).collect::<Vec<_>>() });
        match first_question_id {
            Some(question_id) => Ok(
                match self
                    .operations
                    .workflow_for_question(project_id, question_id, true)
                {
                    Ok(workflow) => with_workflow(response, workflow),
                    Err(_) => response,
                },
            ),
            None => Ok(response),
        }
    }

    fn decision_record(&self, args: &Value) -> Result<Value, HostError> {
        let project_id = project(args)?;
        let question_id = parse_question(required_str(args, "question_id")?)?;
        let revision = required_u64(args, "question_revision")?;
        let alternative = required_str(args, "alternative_key")?.to_owned();
        let turn = required_str(args, "user_turn")?.to_owned();
        let frontier = self
            .operations
            .inquiry_frontier(project_id, Vec::new())
            .map_err(operation_error)?;
        let displayed = frontier
            .questions
            .into_iter()
            .find(|value| value.question_id == question_id && value.displayed_revision == revision)
            .ok_or_else(|| {
                HostError::new("the exact current Question revision is not on the frontier")
            })?;
        let source = self
            .operations
            .record_user_source(
                project_id,
                "codex".into(),
                self.host_session.clone(),
                turn.clone(),
            )
            .map_err(operation_error)?;
        let source_id = parse_source(&source.identity)?;
        let result = self
            .operations
            .record_inquiry_responses(
                project_id,
                vec![BatchResponseItem {
                    operation_id: new_operation_id()?,
                    response: CurrentHostResponse {
                        project_id,
                        source_id,
                        host: "codex".into(),
                        session: self.host_session.clone(),
                        turn,
                        displayed: DisplayedQuestion {
                            question_id,
                            revision,
                            alternative_keys: displayed
                                .alternatives
                                .iter()
                                .map(|value| value.key.clone())
                                .collect(),
                            recommendation_key: displayed.recommendation.alternative_key,
                        },
                        mapping: ResponseMapping::ExplicitAlternative {
                            alternative_key: alternative,
                            user_rationale: args
                                .get("user_rationale")
                                .and_then(Value::as_str)
                                .map(ToOwned::to_owned),
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
            .map_err(operation_error)?;
        let response = json!({"project_id":project_id.to_string(),"user_response_source_id":source_id.to_string(),"all_succeeded":result.all_succeeded(),"outcomes":result.items.into_iter().map(|(id,revision,outcome)| json!({"question_id":id.to_string(),"revision":revision,"outcome":format!("{:?}",outcome)})).collect::<Vec<_>>() });
        Ok(
            match self
                .operations
                .workflow_for_question(project_id, question_id, false)
            {
                Ok(workflow) => with_workflow(response, workflow),
                Err(_) => response,
            },
        )
    }

    fn checkpoint_record(&self, args: &Value) -> Result<Value, HostError> {
        let project_id = project(args)?;
        let goal_context_id = parse_context_item(required_str(args, "goal_context_id")?)?;
        let baseline_analysis_snapshot_id =
            parse_analysis_snapshot(required_str(args, "baseline_analysis_snapshot_id")?)?;
        let decision_components = string_array(args, "decision_components")?;
        let work_contexts = string_array(args, "work_contexts")?;
        let met_revisit_triggers = string_array(args, "met_revisit_triggers")?;
        let verification = args
            .get("verification")
            .and_then(Value::as_array)
            .ok_or_else(|| HostError::new("verification must be an array"))?
            .iter()
            .map(command_verification)
            .collect::<Result<Vec<_>, _>>()?;
        let result = self
            .operations
            .record_grounded_checkpoint(GroundedCheckpointDraft {
                project_id,
                goal_context_id,
                baseline_analysis_snapshot_id,
                kind: checkpoint_kind(required_str(args, "kind")?)?,
                work_state: work_state(required_str(args, "work_state")?)?,
                state_change: optional_string(args, "state_change")?,
                applied_decisions: decision_ids(args, "applied_decision_ids")?,
                decision_components: decision_components.clone(),
                work_contexts: work_contexts.clone(),
                met_revisit_triggers: met_revisit_triggers.clone(),
                verification,
                known_limits: string_array(args, "known_limits")?,
                non_goals: string_array(args, "non_goals")?,
                next_step: required_str(args, "next_step")?.to_owned(),
                handoff_to: optional_string(args, "handoff_to")?,
            });
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                let workflow = self
                    .operations
                    .workflow_for_work_basis(
                        project_id,
                        goal_context_id,
                        baseline_analysis_snapshot_id,
                        Vec::new(),
                        decision_components,
                        work_contexts,
                        met_revisit_triggers,
                    )
                    .map(workflow_json)
                    .unwrap_or_else(|workflow_error| {
                        json!({
                            "reason":"work-authority guidance could not be evaluated",
                            "diagnostic":workflow_error.to_string(),
                        })
                    });
                return Err(HostError::with_details(
                    error.to_string(),
                    json!({"workflow":workflow}),
                ));
            }
        };
        let workflow = LocalOperations::workflow_after_checkpoint(
            project_id,
            result.checkpoint_id,
            result.goal_context_id,
            result.baseline_analysis_snapshot_id,
        );
        Ok(with_workflow(
            json!({
                "checkpoint_id":result.checkpoint_id.to_string(),
                "revision":result.checkpoint_revision,
                "goal_context_id":result.goal_context_id.to_string(),
                "baseline_analysis_snapshot_id":result.baseline_analysis_snapshot_id.to_string(),
                "current_analysis_snapshot_id":result.current_analysis_snapshot_id.to_string(),
                "baseline_repository_snapshot_id":result.baseline_repository_snapshot_id.to_string(),
                "current_repository_snapshot_id":result.current_repository_snapshot_id.to_string(),
                "pre_existing_dirty_paths":result.pre_existing_dirty_paths,
                "changed_paths":result.changed_paths,
                "applied_decision_ids":result.applied_decisions.into_iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                "verification_source_ids":result.verification_source_ids.into_iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            }),
            workflow,
        ))
    }

    fn context_record(&self, args: &Value) -> Result<Value, HostError> {
        let role = context_item_role(required_str(args, "role")?)?;
        let project_id = project(args)?;
        let result = self
            .operations
            .record_current_host_user_context(
                project_id,
                "codex".into(),
                self.host_session.clone(),
                required_str(args, "user_turn")?.to_owned(),
                role,
                required_str(args, "statement")?.to_owned(),
            )
            .map_err(operation_error)?;
        let response = json!({
            "project_id": project_id.to_string(),
            "source_id": result.source_id.to_string(),
            "context_item_id": result.context_item_id.to_string(),
            "revision": result.context_item_revision,
            "role": context_item_role_name(result.role),
        });
        Ok(if result.role == ContextItemRole::Goal {
            with_workflow(
                response,
                LocalOperations::workflow_after_goal(project_id, result.context_item_id),
            )
        } else {
            response
        })
    }

    fn canonical_inspect(&self, args: &Value) -> Result<Value, HostError> {
        let projection = self
            .operations
            .project_projection(project(args)?)
            .map_err(operation_error)?;
        Ok(
            json!({"records":projection.canonical_inspection.into_iter().map(|record| json!({"kind":format!("{:?}",record.kind).to_lowercase(),"identity":record.identity,"revision":record.revision,"lifecycle_state":record.lifecycle_state,"statement_role":record.statement_role,"summary":record.summary,"source_basis":record.source_basis.into_iter().map(|source| source.to_string()).collect::<Vec<_>>()})).collect::<Vec<_>>(),"read_only":true}),
        )
    }

    fn canonical_mutate(&self, args: &Value) -> Result<Value, HostError> {
        let project_id = project(args)?;
        let source = self
            .operations
            .record_user_source(
                project_id,
                "codex".into(),
                self.host_session.clone(),
                required_str(args, "user_turn")?.to_owned(),
            )
            .map_err(operation_error)?;
        let authorization = parse_source(&source.identity)?;
        let action = required_str(args, "action")?;
        if action == "forget" {
            let bytes = parse_identity(required_str(args, "record_id")?)?;
            let record = match required_str(args, "record_kind")? {
                "source" => CanonicalRecordId::Source(SourceId::from_bytes(bytes)),
                "question" => CanonicalRecordId::Question(QuestionId::from_bytes(bytes)),
                "decision" => CanonicalRecordId::Decision(DecisionId::from_bytes(bytes)),
                "context_item" => CanonicalRecordId::ContextItem(ContextItemId::from_bytes(bytes)),
                "checkpoint" => CanonicalRecordId::Checkpoint(CheckpointId::from_bytes(bytes)),
                _ => return Err(HostError::new("record kind is not forgettable")),
            };
            let outcome = self
                .operations
                .forget_record(project_id, record, authorization)
                .map_err(operation_error)?;
            return Ok(json!({
                "action": action,
                "record_kind": outcome.record_kind,
                "identity": outcome.identity,
                "operation_id": outcome.operation_id.to_string(),
                "state": format!("{:?}", outcome.state).to_lowercase(),
                "canonical_committed": outcome.canonical_committed,
                "candidate_cleanup_completed": outcome.candidate_cleanup_completed,
                "managed_derived_cleanup_completed": outcome.managed_derived_cleanup_completed,
                "residue_verified": outcome.residue_verified,
                "replayed": outcome.replayed,
                "provider_deletion": format!("{:?}", outcome.provider_deletion).to_lowercase(),
                "diagnostic": outcome.diagnostic,
                "user_response_source_id": authorization.to_string(),
            }));
        }
        let outcome = match action {
            "correct_context" => self.operations.correct_context_item(
                project_id,
                ContextItemId::from_bytes(parse_identity(required_str(args, "record_id")?)?),
                ContextItemCorrectionDraft {
                    expected_revision: required_u64(args, "expected_revision")?,
                    corrected_statement: required_str(args, "corrected_text")?.to_owned(),
                    kind: CorrectionKind::Expression,
                    user_authorization_source_id: authorization,
                },
            ),
            "correct_decision" => self.operations.correct_decision(
                project_id,
                DecisionId::from_bytes(parse_identity(required_str(args, "record_id")?)?),
                DecisionCorrectionDraft {
                    expected_revision: required_u64(args, "expected_revision")?,
                    corrected_user_rationale: Some(
                        required_str(args, "corrected_text")?.to_owned(),
                    ),
                    kind: CorrectionKind::Expression,
                    user_authorization_source_id: authorization,
                },
            ),
            "supersede_decision" => self.operations.supersede_decision_choice(
                project_id,
                DecisionId::from_bytes(parse_identity(required_str(args, "record_id")?)?),
                authorization,
                required_str(args, "alternative_key")?.to_owned(),
                args.get("rationale")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
            ),
            _ => return Err(HostError::new("unknown canonical mutation action")),
        }
        .map_err(operation_error)?;
        Ok(
            json!({"action":action,"record_kind":outcome.record_kind,"identity":outcome.identity,"revision":outcome.revision,"user_response_source_id":authorization.to_string()}),
        )
    }

    fn candidate_inspect(&self, args: &Value) -> Result<Value, HostError> {
        let projection = self
            .operations
            .project_projection(project(args)?)
            .map_err(operation_error)?;
        Ok(json!({
            "health": candidate_dependency_key(projection.candidate_dependency),
            "issues": projection
                .issues
                .iter()
                .filter(|issue| issue.affected_scope == "candidate_inspection")
                .map(|issue| json!({
                    "kind": format!("{:?}", issue.kind).to_lowercase(),
                    "scope": issue.affected_scope,
                    "reason": issue.reason,
                    "omitted_count": issue.omitted_count,
                }))
                .collect::<Vec<_>>(),
            "candidates": projection
                .candidate_inspection
                .into_iter()
                .map(candidate_inspection_json)
                .collect::<Vec<_>>(),
            "read_only": true
        }))
    }

    fn candidate_manage(&self, args: &Value) -> Result<Value, HostError> {
        let project_id = project(args)?;
        match required_str(args, "action")? {
            "submit_question" => {
                let source_basis = source_ids(args, "source_ids")?;
                let research_state =
                    question_research_state(required_str(args, "research_state")?)?;
                let research_state_basis = required_str(args, "research_state_basis")?.to_owned();
                let draft = question_candidate_draft(
                    args,
                    project_id,
                    &self.host_session,
                    source_basis,
                    required_str(args, "source_operation")?.to_owned(),
                    string_array(args, "affected_scope")?,
                    required_str(args, "materiality_rationale")?.to_owned(),
                    research_state,
                    &research_state_basis,
                )?;
                match self
                    .operations
                    .submit_candidate(draft)
                    .map_err(operation_error)?
                {
                    SubmissionOutcome::Stored(candidate) => Ok(json!({
                        "action": "submit_question",
                        "state": "stored",
                        "candidate_id": candidate.id.to_string(),
                        "candidate_revision": candidate.revision,
                        "research_state": question_research_state_name(research_state),
                        "research_state_basis": research_state_basis,
                        "collection_mode": "automatic",
                        "disposition": candidate_disposition_json(&candidate.disposition),
                        "canonical_mutation": false
                    })),
                    SubmissionOutcome::CollectionDisabled { matching_scopes } => Ok(json!({
                        "action": "submit_question",
                        "state": "collection_disabled",
                        "matching_opt_out_scopes": matching_scopes.into_iter().map(collection_opt_out_json).collect::<Vec<_>>(),
                        "canonical_mutation": false
                    })),
                }
            }
            "submit_question_from_materiality" => {
                let review_candidate_id =
                    parse_candidate(required_str(args, "review_candidate_id")?)?;
                let dimension_id = required_str(args, "dimension_id")?;
                let research_state =
                    question_research_state(required_str(args, "research_state")?)?;
                let research_state_basis = required_str(args, "research_state_basis")?.to_owned();
                let draft = question_candidate_draft(
                    args,
                    project_id,
                    &self.host_session,
                    Vec::new(),
                    format!("Materiality Review {review_candidate_id} dimension {dimension_id}"),
                    Vec::new(),
                    format!("reused Materiality Review dimension {dimension_id}"),
                    research_state,
                    &research_state_basis,
                )?;
                match self
                    .operations
                    .submit_materiality_question_candidate(review_candidate_id, dimension_id, draft)
                    .map_err(operation_error)?
                {
                    SubmissionOutcome::Stored(candidate) => {
                        let workflow = self
                            .operations
                            .workflow_for_question_candidate(project_id, candidate.id)
                            .map_err(operation_error)?;
                        Ok(with_workflow(
                            json!({
                                "action":"submit_question_from_materiality",
                                "state":"stored",
                                "review_candidate_id":review_candidate_id.to_string(),
                                "dimension_id":dimension_id,
                                "candidate_id":candidate.id.to_string(),
                                "candidate_revision":candidate.revision,
                                "research_state":question_research_state_name(research_state),
                                "research_state_basis":research_state_basis,
                                "collection_mode":"automatic",
                                "disposition":candidate_disposition_json(&candidate.disposition),
                                "canonical_mutation":false,
                            }),
                            workflow,
                        ))
                    }
                    SubmissionOutcome::CollectionDisabled { matching_scopes } => Ok(json!({
                        "action":"submit_question_from_materiality",
                        "state":"collection_disabled",
                        "matching_opt_out_scopes":matching_scopes.into_iter().map(collection_opt_out_json).collect::<Vec<_>>(),
                        "canonical_mutation":false,
                    })),
                }
            }
            "attach_repository_research" => {
                let candidate_id = parse_candidate(required_str(args, "candidate_id")?)?;
                let candidate = self
                    .operations
                    .attach_candidate_repository_research(
                        project_id,
                        candidate_id,
                        CandidateRepositoryResearchDraft {
                            capability: required_str(args, "capability")?.to_owned(),
                            coverage: required_str(args, "coverage")?.to_owned(),
                            freshness: candidate_freshness(required_str(args, "freshness")?)?,
                            source_basis: source_ids(args, "source_ids")?,
                            sufficient: required_str(args, "evidence_assessment")? == "sufficient",
                            limits: string_array(args, "limits")?,
                        },
                    )
                    .map_err(operation_error)?;
                let response =
                    candidate_research_lifecycle_json("attach_repository_research", &candidate)?;
                Ok(
                    match self
                        .operations
                        .workflow_for_question_candidate(project_id, candidate.id)
                    {
                        Ok(workflow) => with_workflow(response, workflow),
                        Err(_) => response,
                    },
                )
            }
            "mark_research_ready" => {
                let candidate_id = parse_candidate(required_str(args, "candidate_id")?)?;
                let candidate = self
                    .operations
                    .mark_candidate_ready_to_ask(project_id, candidate_id)
                    .map_err(operation_error)?;
                let response =
                    candidate_research_lifecycle_json("mark_research_ready", &candidate)?;
                Ok(
                    match self
                        .operations
                        .workflow_for_question_candidate(project_id, candidate.id)
                    {
                        Ok(workflow) => with_workflow(response, workflow),
                        Err(_) => response,
                    },
                )
            }
            "promote_question" => {
                let candidate_id = parse_candidate(required_str(args, "candidate_id")?)?;
                let result = self
                    .operations
                    .promote_question_candidate(project_id, candidate_id)
                    .map_err(operation_error)?;
                let response = json!({
                    "action": "promote_question",
                    "candidate_id": result.candidate_id.to_string(),
                    "question_id": result.question_id.to_string(),
                    "canonical_replayed": result.canonical_replayed,
                    "candidate_reconciled": result.candidate_reconciled
                });
                Ok(
                    match self.operations.workflow_for_question(
                        project_id,
                        result.question_id,
                        false,
                    ) {
                        Ok(workflow) => with_workflow(response, workflow),
                        Err(_) => response,
                    },
                )
            }
            "dismiss" => {
                let candidate = self
                    .operations
                    .dismiss_candidate(
                        project_id,
                        parse_candidate(required_str(args, "candidate_id")?)?,
                        required_str(args, "reason")?,
                    )
                    .map_err(operation_error)?;
                Ok(json!({
                    "action": "dismiss",
                    "candidate_id": candidate.id.to_string(),
                    "candidate_revision": candidate.revision,
                    "disposition": candidate_disposition_json(&candidate.disposition),
                    "canonical_mutation": false
                }))
            }
            "delete" => {
                let candidate = self
                    .operations
                    .delete_candidate(
                        project_id,
                        parse_candidate(required_str(args, "candidate_id")?)?,
                        required_str(args, "basis")?,
                    )
                    .map_err(operation_error)?;
                Ok(json!({
                    "action": "delete",
                    "candidate_id": candidate.id.to_string(),
                    "candidate_revision": candidate.revision,
                    "content_cleaned": candidate.content.is_none(),
                    "disposition": candidate_disposition_json(&candidate.disposition),
                    "canonical_mutation": false
                }))
            }
            _ => Err(HostError::new("unknown Candidate lifecycle action")),
        }
    }

    fn privacy_status(&self, args: &Value) -> Result<Value, HostError> {
        let status = self
            .operations
            .privacy_status(project(args)?)
            .map_err(operation_error)?;
        Ok(
            json!({"configuration_state":format!("{:?}",status.configuration_state).to_lowercase(),"policy":status.current_opt_in.as_ref().map(|event| json!({"state":format!("{:?}",event.state).to_lowercase(),"provider":event.policy.provider,"model":event.policy.model,"scope":event.policy.allowed_source_scopes})),"requests":status.requests.len(),"managed_derived":status.managed_derived.len(),"local_only":status.current_opt_in.is_none()}),
        )
    }

    fn background_semantic_operation(&mut self, args: &Value) -> Result<Value, HostError> {
        match required_str(args, "action")? {
            "prepare" => {
                let expires_at = required_u64(args, "expiration_unix_micros")?;
                let expires_at = i64::try_from(expires_at)
                    .map(TimestampMicros::from_unix_micros)
                    .map_err(|_| {
                        HostError::new("expiration exceeds the supported timestamp range")
                    })?;
                let source_paths = required_strings(args, "source_paths")?;
                let outcome = self
                    .operations
                    .prepare_guarded_provider_operation(BackgroundProviderOperationDraft {
                        project_id: project(args)?,
                        provider: required_str(args, "provider")?.to_owned(),
                        model: required_str(args, "model")?.to_owned(),
                        purpose: required_str(args, "purpose")?.to_owned(),
                        requested_capability: required_str(args, "requested_capability")?
                            .to_owned(),
                        source_paths,
                        expires_at,
                        requesting_provenance: RequestingProvenance {
                            actor: Principal {
                                kind: PrincipalKind::Agent,
                                identity: "codex".into(),
                            },
                            host: Some("codex".into()),
                            session: Some(self.host_session.clone()),
                            basis: vec![
                                "Codex requested a bounded background semantic operation".into()
                            ],
                        },
                    })
                    .map_err(operation_error)?;
                match outcome {
                    GuardedProviderPreparationOutcome::Rejected(record) => Ok(json!({
                        "state":"not_prepared",
                        "provider_request":provider_request_json(&record),
                        "guarded_request":Value::Null,
                        "dispatch_occurred":false,
                        "next_safe_action":"inspect privacy status and establish matching Project opt-in before preparing a new operation"
                    })),
                    GuardedProviderPreparationOutcome::Ready(preparation) => {
                        let preparation = *preparation;
                        let candidate = preparation.candidate.clone();
                        let provider_request = preparation.provider_request.clone();
                        if self
                            .pending_provider_operations
                            .insert(candidate.confirmation_request_identity, preparation)
                            .is_some()
                        {
                            return Err(HostError::new(
                                "generated Guarded request identity collided with a live preparation",
                            ));
                        }
                        Ok(json!({
                            "state":"awaiting_exact_confirmation",
                            "provider_request":provider_request_json(&provider_request),
                            "guarded_request":guarded_candidate_json(&candidate),
                            "dispatch_occurred":false,
                            "next_safe_action":"inspect and answer this exact request with guarded_interaction, then dispatch this live preparation"
                        }))
                    }
                }
            }
            "dispatch" => {
                let request_id =
                    parse_confirmation(required_str(args, "confirmation_request_id")?)?;
                let request_revision = required_u64(args, "request_revision")?;
                let effect_fingerprint = required_str(args, "effect_fingerprint")?;
                let preparation = self
                    .pending_provider_operations
                    .get_mut(&request_id)
                    .ok_or_else(|| {
                        HostError::new(
                            "live provider preparation is unavailable; no provider dispatch occurred",
                        )
                    })?;
                let project_id = preparation.candidate.project_id;
                let provider_request_id = preparation.provider_request.id;
                let dispatch = self
                    .operations
                    .dispatch_guarded_provider_with_configured_adapter(
                        preparation,
                        request_revision,
                        effect_fingerprint,
                    );
                let payload_consumed = !preparation.retains_authorized_payload();
                let terminal = dispatch.as_ref().is_ok_and(provider_dispatch_is_terminal);
                if payload_consumed || terminal {
                    self.pending_provider_operations.remove(&request_id);
                }
                let result = dispatch.map_err(operation_error)?;
                let inspected = self
                    .operations
                    .inspect_guarded_provider_operation(
                        project_id,
                        result.operation_identity,
                        provider_request_id,
                    )
                    .map_err(operation_error)?;
                Ok(guarded_provider_inspection_json(&inspected))
            }
            "inspect" => {
                let inspected = self
                    .operations
                    .inspect_guarded_provider_operation(
                        project(args)?,
                        parse_guarded_operation(required_str(args, "operation_id")?)?,
                        parse_provider_request(required_str(args, "provider_request_id")?)?,
                    )
                    .map_err(operation_error)?;
                Ok(guarded_provider_inspection_json(&inspected))
            }
            _ => Err(HostError::new(
                "background semantic operation action must be prepare, dispatch, or inspect",
            )),
        }
    }

    fn document_preview(&self, args: &Value) -> Result<Value, HostError> {
        let kind = document_kind(required_str(args, "kind")?)?;
        let format = if args.get("format").and_then(Value::as_str) == Some("html") {
            OutputFormat::Html
        } else {
            OutputFormat::Markdown
        };
        let request = DocumentRequest {
            requested_language: args
                .get("language")
                .and_then(Value::as_str)
                .unwrap_or("en")
                .to_owned(),
            fixed_locale: if args.get("locale").and_then(Value::as_str) == Some("ko") {
                FixedLocale::Korean
            } else {
                FixedLocale::English
            },
            generated_at: SystemClock
                .now()
                .map_err(|error| HostError::new(error.to_string()))?,
            generator: GeneratorIdentity {
                generator: "volicord-codex-host".into(),
                agent: Some("codex".into()),
                model: None,
            },
            requested_destinations: Vec::new(),
        };
        let project_id = project(args)?;
        if let Some(value) = args.get("realization") {
            let realization = narrative_realization(value)?;
            let document = self
                .operations
                .realize_document_narrative(project_id, &request, kind, &realization)
                .map_err(operation_error)?;
            let content = if format == OutputFormat::Html {
                document.html.content
            } else {
                document.markdown.content
            };
            return Ok(json!({
                "outcome":"realized",
                "kind":kind.slug(),
                "format":format!("{:?}",format).to_lowercase(),
                "requested_language":document.metadata.requested_language,
                "generator":{
                    "generator":document.metadata.generator.generator,
                    "agent":document.metadata.generator.agent,
                    "model":document.metadata.generator.model,
                },
                "content":content,
                "canonical_mutation":false
            }));
        }
        let set = self
            .operations
            .documents(project_id, &request)
            .map_err(operation_error)?;
        let document = match kind {
            DocumentKind::ProjectArchitectureGuide => set.project_architecture_guide,
            DocumentKind::DecisionReport => set.decision_report,
            DocumentKind::ImplementationPlan => set.implementation_plan,
            DocumentKind::HandoffResume => set.handoff_resume,
        };
        if matches!(
            document.metadata.narrative_realization,
            NarrativeRealizationState::Unavailable { .. }
        ) {
            let plan = self
                .operations
                .document_narrative_plan(project_id, &request, kind)
                .map_err(operation_error)?;
            return Ok(json!({
                "outcome":"realization_required",
                "kind":kind.slug(),
                "format":format!("{:?}",format).to_lowercase(),
                "requested_language":request.requested_language,
                "plan":narrative_plan_json(&plan),
                "canonical_mutation":false
            }));
        }
        let content = if format == OutputFormat::Html {
            document.html.content
        } else {
            document.markdown.content
        };
        Ok(
            json!({"outcome":"fixed_locale","kind":kind.slug(),"format":format!("{:?}",format).to_lowercase(),"content":content,"canonical_mutation":false}),
        )
    }

    fn guarded_interaction(&mut self, args: &Value) -> Result<Value, HostError> {
        let request_id = parse_confirmation(required_str(args, "confirmation_request_id")?)?;
        let request = self
            .operations
            .guarded_request(request_id)
            .map_err(operation_error)?;
        let decision = args.get("decision").and_then(Value::as_str);
        if decision.is_none() {
            return Ok(json!({
                "confirmation_request_id":request.confirmation_request_identity.to_string(),"request_revision":request.request_revision,"effect_fingerprint":request.effect_fingerprint,
                "exact_action":request.exact_action,"target":request.target,"expected_effect":request.expected_effect,"risk":format!("{:?}: {}",request.risk.category,request.risk.concrete_consequence),"scope":request.scope,"expiration_unix_micros":request.expires_at.as_unix_micros(),
                "host_elicitation_available":self.client_supports_elicitation,
                "fallback":if self.client_supports_elicitation { Value::Null } else { json!({"viewer":["volicord-viewer","--project",request.project_id.to_string()],"cli":["volicord","guarded","confirm",request.confirmation_request_identity.to_string(),request.request_revision.to_string(),request.effect_fingerprint.clone(),"codex",self.host_session.clone(),"EXPLICIT_USER_RESPONSE"]}) }
            }));
        }
        let revision = required_u64(args, "request_revision")?;
        let fingerprint = required_str(args, "effect_fingerprint")?;
        let user_turn = required_str(args, "user_turn")?.to_owned();
        let decision = match decision {
            Some("confirm") => ConfirmationDecision::Confirmed,
            Some("deny") => ConfirmationDecision::Denied,
            _ => return Err(HostError::new("decision must be confirm or deny")),
        };
        let response = self
            .operations
            .record_confirmation(
                request_id,
                revision,
                fingerprint,
                decision,
                "codex".into(),
                self.host_session.clone(),
                user_turn,
            )
            .map_err(operation_error)?;
        if response.decision == ConfirmationDecision::Denied {
            self.pending_provider_operations.remove(&request_id);
        }
        Ok(
            json!({"confirmation_request_id":response.confirmation_request_identity.to_string(),"request_revision":response.request_revision,"effect_fingerprint":response.effect_fingerprint,"decision":format!("{:?}",response.decision).to_lowercase(),"user_response_source_id":response.user_response_source_id.to_string()}),
        )
    }
}

const fn candidate_dependency_key(state: CandidateDependencyState) -> &'static str {
    match state {
        CandidateDependencyState::Available => "available",
        CandidateDependencyState::Unavailable => "unavailable",
        CandidateDependencyState::Unsupported => "unsupported",
        CandidateDependencyState::Corrupt => "corrupt",
        CandidateDependencyState::RepairRequired => "repair_required",
        CandidateDependencyState::Failed => "failed",
    }
}

fn provider_dispatch_is_terminal(result: &volicord_operations::GuardedOperationResult) -> bool {
    match &result.outcome {
        GuardedOperationOutcome::NotDispatched {
            rejection:
                Some(
                    ConfirmationRejection::Stale
                    | ConfirmationRejection::Expired
                    | ConfirmationRejection::Denied
                    | ConfirmationRejection::Reused
                    | ConfirmationRejection::InvalidUserSource,
                ),
            ..
        } => true,
        GuardedOperationOutcome::NotDispatched { .. } => false,
        GuardedOperationOutcome::DispatchedAndCompleted { .. }
        | GuardedOperationOutcome::DispatchedAndFailed { .. }
        | GuardedOperationOutcome::ExecutionOutcomeIndeterminate { .. } => true,
    }
}

pub fn run_stdio(
    adapter: &mut HostAdapter,
    reader: impl BufRead,
    mut writer: impl Write,
) -> Result<(), HostError> {
    for line in reader.lines() {
        let line =
            line.map_err(|error| HostError::new(format!("cannot read MCP input: {error}")))?;
        if line.trim().is_empty() {
            continue;
        }
        let message: Value = serde_json::from_str(&line)
            .map_err(|error| HostError::new(format!("invalid MCP JSON: {error}")))?;
        if let Some(response) = adapter.handle(message) {
            serde_json::to_writer(&mut writer, &response)
                .map_err(|error| HostError::new(format!("cannot encode MCP response: {error}")))?;
            writer
                .write_all(b"\n")
                .and_then(|_| writer.flush())
                .map_err(|error| HostError::new(format!("cannot write MCP response: {error}")))?;
        }
    }
    Ok(())
}

fn tool_catalog() -> Vec<Value> {
    HOST_TOOL_NAMES
        .iter()
        .filter_map(|name| tool_contract(name))
        .map(|contract| {
            json!({
                "name": contract.name,
                "description": contract.description,
                "inputSchema": contract.input_schema,
                "annotations": contract.behavior.annotations(),
            })
        })
        .collect()
}

struct ToolContract {
    name: &'static str,
    description: &'static str,
    input_schema: Value,
    behavior: ToolBehavior,
}

#[derive(Clone, Copy)]
enum ToolBehavior {
    ReadOnlyClosed,
    AdditiveClosed,
    DestructiveClosed,
    AdditiveOpen,
}

impl ToolBehavior {
    fn annotations(self) -> Value {
        match self {
            Self::ReadOnlyClosed => json!({
                "readOnlyHint": true,
                "openWorldHint": false,
            }),
            Self::AdditiveClosed => json!({
                "readOnlyHint": false,
                "destructiveHint": false,
                "idempotentHint": false,
                "openWorldHint": false,
            }),
            Self::DestructiveClosed => json!({
                "readOnlyHint": false,
                "destructiveHint": true,
                "idempotentHint": false,
                "openWorldHint": false,
            }),
            Self::AdditiveOpen => json!({
                "readOnlyHint": false,
                "destructiveHint": false,
                "idempotentHint": false,
                "openWorldHint": true,
            }),
        }
    }
}

impl ToolContract {
    fn validate(&self, arguments: &Value) -> Result<(), HostError> {
        validate_schema(&self.input_schema, arguments, "arguments").map_err(|problems| {
            let summary = problems.join("; ");
            let materiality_context = (self.name == "materiality_review").then(|| json!({
                "bound_identities":{
                    "project_id":arguments.get("project_id"),
                    "engineering_choice_discovery_candidate_id":arguments.get("engineering_choice_discovery_candidate_id"),
                    "materiality_review_candidate_id":arguments.get("review_candidate_id"),
                },
                "missing_or_invalid":"See exact field paths in problems; bounded enum failures include allowed values.",
                "allowed_dispositions":["repository_or_environment_fact","settled_authority","agent_owned_implementation_choice","delegated_implementation_choice","exploratory_uncertainty","unresolved_user_owned_outcome"],
                "next_supported_action":{"tool":"materiality_review","action":"draft","requires":["project_id","engineering_choice_discovery_candidate_id"]},
            }));
            HostError::with_details(
                format!("invalid {} arguments: {summary}", self.name),
                json!({
                    "diagnostic":"aggregate_schema_validation",
                    "tool":self.name,
                    "problems":problems,
                    "materiality_context":materiality_context,
                }),
            )
        })
    }
}

fn tool_contract(name: &str) -> Option<ToolContract> {
    let (description, input_schema, behavior) = match name {
        "project_resolve" => (
            "Read-only resolve the existing Volicord Project bound to an absolute local repository path. In a fresh project-scoped session do this before repository inspection, edits, or continuation; after a found result, Recall must succeed before that work. Initialize explicitly only after a not_found result.",
            object_schema(
                vec![("repository", text_schema("Absolute local repository path to canonicalize and resolve", 1, 4096))],
                &["repository"],
            ),
            ToolBehavior::ReadOnlyClosed,
        ),
        "project_initialize" => (
            "Explicitly create and optionally bind a new Volicord Project after resolution found no existing repository binding. When repository is supplied without display_name, prefer the strongest bounded repository slug from local Git origin lineage and fall back through the immediate origin hint to the canonical repository-root basename; when the user supplied display_name, preserve it exactly.",
            json!({"oneOf": [
                object_schema(
                    vec![
                        ("display_name", text_schema("User-supplied Project display name", 1, 1024)),
                        ("repository", text_schema("Optional absolute repository path", 1, 4096)),
                    ],
                    &["display_name"],
                ),
                object_schema(
                    vec![("repository", text_schema("Absolute repository path used to derive a local repository-native Project display name", 1, 4096))],
                    &["repository"],
                ),
            ]}),
            ToolBehavior::AdditiveClosed,
        ),
        "project_health" => (
            "Distinguish MCP connection from Project capability health.",
            object_schema(
                vec![("project_id", identity_schema("Optional Project identity"))],
                &[],
            ),
            ToolBehavior::ReadOnlyClosed,
        ),
        "recall" => (
            "Read a bounded source-grounded Project resume brief. In every fresh session with a successfully resolved Project, Recall must succeed before repository inspection, edits, or continued work.",
            project_schema(),
            ToolBehavior::ReadOnlyClosed,
        ),
        "repository_understanding" => (
            "Read the Project overview, repository map, Decision-context-code links, gaps, and degraded states.",
            project_schema(),
            ToolBehavior::ReadOnlyClosed,
        ),
        "repository_analyze" => (
            "Run authorized local repository inventory and structural analysis. In every fresh initialized or resumed meaningful work session, call this after initialization or successful Recall and before the first ordinary repository write; retain the returned analysis_snapshot_id as that bounded session's pre-work Checkpoint baseline. This operation creates local repository-observation Sources and publishes analysis state only in the local Runtime Home; use the returned repository_source_id as the canonical source_ids basis for source-grounded repository research. It performs no background-provider or network transmission. background_semantic_operation is the separate explicit provider boundary.",
            object_schema(
                vec![
                    ("project_id", identity_schema("Project identity")),
                    ("excluded_paths", string_array_schema("Repository-relative paths to exclude")),
                ],
                &["project_id"],
            ),
            ToolBehavior::AdditiveClosed,
        ),
        "engineering_choice_discovery" => (
            "Record one bounded Engineering Choice Discovery for the current Goal and exact pre-work Analysis Snapshot. Include only consequence-bearing forks with credible alternatives; preserve independent choices separately and declare genuinely coupled peers symmetrically. This is discovery, not authority or a user Decision.",
            engineering_choice_discovery_schema(),
            ToolBehavior::AdditiveClosed,
        ),
        "materiality_review" => (
            "Draft, record, revise, or inspect the typed pre-work Materiality Review for one Goal and exact baseline Analysis Snapshot. Start with draft to receive product-owned identities, exact current Goal/user-turn provenance, the material-outcome counterfactual, machine-readable authority-versus-learning routing, every discovered choice, and the validator-owned closed schema variants needed to assemble one record or revise request without a failed call. Classify authority and learning value independently; requests to learn, compare, reason, or select an implementation for learning do not establish user-owned product authority. Agent-owned or explicitly delegated active deliberation-worthy learning routes to learning_deliberation, while genuine user-owned material outcomes route to Question/current-host Decision.",
            json!({"oneOf": materiality_review_schemas()}),
            ToolBehavior::AdditiveClosed,
        ),
        "learning_deliberation" => (
            "Expose one learning-participation interaction for an agent-owned deliberation-worthy choice. Begin or inspect presents facts, alternatives, and consequences without a recommendation; record the current user's initial response before agent feedback, then complete or reconsider. Delegate and skip are terminal learning states. This tool never creates or resolves a canonical Decision; user-owned choices use inquiry_frontier and decision_record.",
            json!({"oneOf": learning_deliberation_schemas()}),
            ToolBehavior::AdditiveClosed,
        ),
        "inquiry_frontier" => (
            "Read current promoted material Questions. Before choosing a genuinely material user-owned unresolved outcome, present each actual alternative, recommendation, and trade-off and obtain an explicit current-host response. Repository-resolvable facts remain research; accepted Decisions and contracts are applied; delegated choices stay agent-owned; exploratory uncertainty may use research, prototype, deferment, or revisit. Submit, attach source-grounded research, review, mark ready, and explicitly promote material Question Candidates through candidate_manage first.",
            object_schema(
                vec![
                    ("project_id", identity_schema("Project identity")),
                    ("material_scope", string_array_schema("Material scope filters")),
                ],
                &["project_id"],
            ),
            ToolBehavior::ReadOnlyClosed,
        ),
        "decision_record" => (
            "Record one explicit current-host user response against one current Question revision; an agent recommendation or implementation preference is not a user Decision.",
            object_schema(
                vec![
                    ("project_id", identity_schema("Project identity")),
                    ("question_id", identity_schema("Question identity")),
                    ("question_revision", unsigned_schema("Displayed Question revision", 1)),
                    ("alternative_key", text_schema("Displayed alternative key", 1, 1024)),
                    ("user_turn", user_turn_schema()),
                    ("user_rationale", text_schema("Optional user rationale", 1, 16_384)),
                ],
                &["project_id", "question_id", "question_revision", "alternative_key", "user_turn"],
            ),
            ToolBehavior::AdditiveClosed,
        ),
        "context_record" => (
            "Record one verbatim statement from the exact current-host user turn as canonical Project Context.",
            object_schema(
                vec![
                    ("project_id", identity_schema("Project identity")),
                    ("user_turn", user_turn_schema()),
                    ("role", enum_schema("User-statement Context role", &["goal", "assumption", "constraint", "preference", "risk", "learning", "known_limit"])),
                    ("statement", text_schema("Verbatim bounded statement from the current-host user turn", 1, 16_384)),
                ],
                &["project_id", "user_turn", "role", "statement"],
            ),
            ToolBehavior::AdditiveClosed,
        ),
        "checkpoint_record" => (
            "Record a grounded Checkpoint from a canonical Goal, the exact pre-work Analysis Snapshot retained for this bounded session, current analysis, applicable Decisions, and truthful verification evidence. The baseline_analysis_snapshot_id must identify an analysis captured after initialization or successful Recall and before the first ordinary repository write; a snapshot first captured after the bounded work is conceptually invalid even when snapshot provenance cannot prove edit ordering. Every executed verification requires the exact transient command_invocation separately from the presentation-only command_label so Volicord can derive a durable fingerprint without retaining raw arguments. A passed or failed verification also requires the numeric exit status from that same actually observed execution; output-only text is insufficient. Incidental inspection commands need not be Checkpoint verification facts.",
            object_schema(
                vec![
                    ("project_id", identity_schema("Project identity")),
                    ("goal_context_id", identity_schema("Canonical user-stated Goal Context identity")),
                    ("baseline_analysis_snapshot_id", digest_identity_schema("Baseline Analysis Snapshot identity")),
                    ("kind", enum_schema("Checkpoint kind", &["completion", "pause", "handoff"])),
                    ("work_state", enum_schema("Independent work state", &["in_progress", "paused", "completed", "abandoned", "superseded"])),
                    ("state_change", text_schema("Optional meaningful state change", 1, 16_384)),
                    ("applied_decision_ids", identity_array_schema("Explicit applied Decision identities", 0)),
                    ("decision_components", string_array_schema("Current components used to evaluate Decision applicability")),
                    ("work_contexts", string_array_schema("Current work contexts used to evaluate Decision applicability")),
                    ("met_revisit_triggers", string_array_schema("Decision revisit triggers known to be met")),
                    ("verification", checkpoint_verification_schema()),
                    ("next_step", text_schema("Next meaningful step", 1, 16_384)),
                    ("known_limits", string_array_schema("Known limits")),
                    ("non_goals", string_array_schema("Explicit non-goals")),
                    ("handoff_to", text_schema("Required target for handoff checkpoints", 1, 4096)),
                ],
                &["project_id", "goal_context_id", "baseline_analysis_snapshot_id", "kind", "work_state", "applied_decision_ids", "verification", "next_step"],
            ),
            ToolBehavior::AdditiveClosed,
        ),
        "canonical_inspect" => (
            "Inspect canonical memory without mutation.",
            project_schema(),
            ToolBehavior::ReadOnlyClosed,
        ),
        "canonical_mutate" => (
            "Correct, supersede, or forget canonical memory through Local Operations using an explicit current-host user turn.",
            json!({"oneOf": canonical_mutation_schemas()}),
            ToolBehavior::DestructiveClosed,
        ),
        "candidate_inspect" => (
            "Inspect bounded Candidate lifecycle state without mutation.",
            project_schema(),
            ToolBehavior::ReadOnlyClosed,
        ),
        "candidate_manage" => (
            "Own the Question Candidate lifecycle when material user authority remains unresolved: submit a Candidate, attach source-grounded repository research when required, mark sufficient research ready, explicitly promote a reviewed ready Candidate to a Question, or disposition Candidate-local content without creating a user Decision. Never use a Question Candidate to ask for a repository fact or to add ceremony to delegated, exploratory, or trivial choices.",
            json!({"oneOf": candidate_management_schemas()}),
            ToolBehavior::DestructiveClosed,
        ),
        "privacy_status" => (
            "Inspect Project background-provider consent and local-only state.",
            project_schema(),
            ToolBehavior::ReadOnlyClosed,
        ),
        "background_semantic_operation" => (
            "Prepare, Guarded-dispatch, or durably inspect one privacy-authorized background semantic-provider operation.",
            json!({"oneOf": background_semantic_operation_schemas()}),
            ToolBehavior::AdditiveOpen,
        ),
        "document_preview" => (
            "Preview one of four grounded documents without repository write or adoption.",
            object_schema(
                vec![
                    ("project_id", identity_schema("Project identity")),
                    ("kind", enum_schema("Grounded document kind", &["project-architecture-guide", "decision-report", "implementation-plan", "handoff-resume"])),
                    ("format", enum_schema("Preview format", &["markdown", "html"])),
                    ("language", text_schema("Requested generated-content language", 1, 128)),
                    ("locale", enum_schema("Bundled fixed-text locale", &["en", "ko"])),
                    ("realization", narrative_realization_schema()),
                ],
                &["project_id", "kind"],
            ),
            ToolBehavior::ReadOnlyClosed,
        ),
        "guarded_interaction" => (
            "Inspect or answer one exact Guarded request/revision; returns viewer/CLI fallback when host elicitation is unavailable.",
            json!({"oneOf": guarded_interaction_schemas()}),
            ToolBehavior::AdditiveClosed,
        ),
        _ => return None,
    };
    Some(ToolContract {
        name: HOST_TOOL_NAMES
            .iter()
            .copied()
            .find(|candidate| *candidate == name)?,
        description,
        input_schema,
        behavior,
    })
}

fn narrative_realization_schema() -> Value {
    let mut claim = object_schema(
        vec![
            (
                "identity",
                text_schema("Exact grounded claim identity", 1, 4096),
            ),
            ("text", text_schema("Realized claim text", 1, 4096)),
        ],
        &["identity", "text"],
    );
    claim["description"] = json!("One exact realized grounded claim");
    let mut claims = json!({"type":"array","minItems":0,"maxItems":64,"items":claim});
    claims["description"] = json!("Exact ordered realized claims for the grounded section");
    let mut section = object_schema(
        vec![
            ("key", text_schema("Exact grounded section key", 1, 256)),
            ("title", text_schema("Realized section title", 1, 4096)),
            ("claims", claims),
        ],
        &["key", "title", "claims"],
    );
    section["description"] = json!("One exact realized grounded section");
    let mut sections = json!({"type":"array","minItems":1,"maxItems":16,"items":section});
    sections["description"] = json!("Exact ordered realized sections for the grounded plan");
    let mut generator = object_schema(
        vec![
            ("generator", text_schema("Host realizer identity", 1, 256)),
            ("agent", text_schema("Active agent identity", 1, 256)),
            ("model", text_schema("Active model identity", 1, 256)),
        ],
        &["generator", "agent", "model"],
    );
    generator["description"] = json!("Active host/model generator provenance");
    let mut realization = object_schema(
        vec![
            (
                "plan_fingerprint",
                text_schema("Exact prepared narrative plan fingerprint", 71, 71),
            ),
            ("title", text_schema("Realized document title", 1, 4096)),
            ("sections", sections),
            ("generator", generator),
        ],
        &["plan_fingerprint", "title", "sections", "generator"],
    );
    realization["description"] =
        json!("Active-host natural-language realization of one prepared grounded plan");
    realization
}

fn background_semantic_operation_schemas() -> Vec<Value> {
    vec![
        object_schema(
            vec![
                (
                    "action",
                    enum_schema("Provider operation action", &["prepare"]),
                ),
                ("project_id", identity_schema("Project identity")),
                (
                    "provider",
                    text_schema("Provider identity from the Project opt-in", 1, 16_384),
                ),
                (
                    "model",
                    text_schema("Model identity from the Project opt-in", 1, 16_384),
                ),
                (
                    "purpose",
                    text_schema("Exact background analysis purpose", 1, 16_384),
                ),
                (
                    "requested_capability",
                    text_schema("Exact requested semantic capability", 1, 16_384),
                ),
                (
                    "source_paths",
                    nonempty_string_array_schema("Explicit repository-relative source paths"),
                ),
                (
                    "expiration_unix_micros",
                    unsigned_schema("Guarded confirmation expiration", 1),
                ),
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
        object_schema(
            vec![
                (
                    "action",
                    enum_schema("Provider operation action", &["dispatch"]),
                ),
                (
                    "confirmation_request_id",
                    identity_schema("Guarded confirmation request identity"),
                ),
                (
                    "request_revision",
                    unsigned_schema("Exact displayed request revision", 1),
                ),
                ("effect_fingerprint", fingerprint_schema()),
            ],
            &[
                "action",
                "confirmation_request_id",
                "request_revision",
                "effect_fingerprint",
            ],
        ),
        object_schema(
            vec![
                (
                    "action",
                    enum_schema("Provider operation action", &["inspect"]),
                ),
                ("project_id", identity_schema("Project identity")),
                (
                    "operation_id",
                    identity_schema("Guarded operation identity"),
                ),
                (
                    "provider_request_id",
                    identity_schema("Provider request identity"),
                ),
            ],
            &[
                "action",
                "project_id",
                "operation_id",
                "provider_request_id",
            ],
        ),
    ]
}

fn engineering_choice_discovery_schema() -> Value {
    let alternative = object_schema(
        vec![
            (
                "alternative_id",
                text_schema("Stable alternative identity within this choice", 1, 256),
            ),
            (
                "summary",
                text_schema(
                    "Credible technical approach without authority claims",
                    1,
                    4096,
                ),
            ),
            (
                "technical_consequences",
                nonempty_string_array_schema("Consequences specific to this alternative"),
            ),
        ],
        &["alternative_id", "summary", "technical_consequences"],
    );
    let relationship = json!({"description":"Whether this choice is independent or necessarily coupled to exact peer choices","oneOf":[
        object_schema(
            vec![("state", enum_schema("Choice relationship", &["independent"]))],
            &["state"],
        ),
        object_schema(
            vec![
                ("state", enum_schema("Choice relationship", &["coupled"])),
                ("choice_ids", nonempty_string_array_schema("Exact peer choice identities that must be resolved jointly")),
                ("rationale", text_schema("Why these exact consequences require a joint outcome", 1, 4096)),
            ],
            &["state", "choice_ids", "rationale"],
        ),
    ]});
    let choices = json!({
        "type":"array",
        "description":"Bounded consequence-bearing engineering forks; omit mechanically equivalent or trivial syntax/naming details",
        "minItems":1,
        "maxItems":64,
        "items":object_schema(
            vec![
                ("choice_id", text_schema("Stable discovered choice identity", 1, 256)),
                ("summary", text_schema("Bounded technical choice summary", 1, 4096)),
                ("affected_scope", nonempty_string_array_schema("Repository, product, or operational scope affected by this choice")),
                ("alternatives", json!({"type":"array","description":"Credible alternatives; two are required when evidence is sufficient","minItems":0,"maxItems":64,"items":alternative})),
                ("technical_consequences", nonempty_string_array_schema("Consequences that make this a real engineering fork")),
                ("source_ids", identity_array_schema("Current canonical Source identities grounding discovery", 1)),
                ("effect_categories", json!({"type":"array","description":"Non-authoritative effect signals","minItems":1,"maxItems":11,"items":enum_schema("Engineering effect category", &[
                    "public_api_shape_or_semantics", "compatibility", "failure_or_error_semantics",
                    "persistence_or_lifetime", "privacy_or_disclosure", "security",
                    "user_visible_behavior_or_default", "performance_or_resource_behavior",
                    "concurrency_or_operability", "maintenance_or_support", "implementation_internal"
                ])})),
                ("relationship", relationship),
                ("evidence_state", enum_schema("Current alternative evidence state", &["sufficient", "research_required", "prototype_required"])),
            ],
            &["choice_id", "summary", "affected_scope", "alternatives", "technical_consequences", "source_ids", "effect_categories", "relationship", "evidence_state"],
        ),
    });
    object_schema(
        vec![
            ("project_id", identity_schema("Current Project identity")),
            (
                "goal_context_id",
                identity_schema("Current canonical Goal Context identity"),
            ),
            (
                "baseline_analysis_snapshot_id",
                digest_identity_schema("Exact retained pre-work Analysis Snapshot identity"),
            ),
            (
                "source_operation",
                text_schema("Inspectable discovery operation or bounded scope", 1, 4096),
            ),
            ("summary", text_schema("Bounded discovery summary", 1, 4096)),
            ("choices", choices),
        ],
        &[
            "project_id",
            "goal_context_id",
            "baseline_analysis_snapshot_id",
            "source_operation",
            "summary",
            "choices",
        ],
    )
}

fn learning_participation_schema() -> Value {
    json!({"description":"Explicit bounded learning participation; inactive is the default and active requires current-host user provenance","oneOf":[
        object_schema(
            vec![("state", enum_schema("Learning participation for this bounded review", &["inactive"]))],
            &["state"],
        ),
        object_schema(
            vec![
                ("state", enum_schema("Learning participation for this bounded review", &["active"])),
                ("user_turn_source_id", identity_schema("Exact current-host user-turn Source containing the explicit opt-in")),
                ("verbatim_statement", text_schema("Non-empty verbatim learning-participation statement from that Source", 1, 4096)),
            ],
            &["state", "user_turn_source_id", "verbatim_statement"],
        ),
    ]})
}

fn learning_value_schema() -> Value {
    json!({"description":"Learning value assessed independently from authority","oneOf":[
        object_schema(
            vec![
                ("state", enum_schema("Independent learning-value assessment", &["routine"])),
                ("rationale", text_schema("Why this choice does not justify interruption", 1, 4096)),
            ],
            &["state", "rationale"],
        ),
        object_schema(
            vec![
                ("state", enum_schema("Independent learning-value assessment", &["deliberation_worthy"])),
                ("rationale", text_schema("Why pre-work user reasoning is worthwhile", 1, 4096)),
                ("consequence_significance", nonempty_string_array_schema("Significant consequences worth understanding")),
                ("transferable_principles", nonempty_string_array_schema("Principles transferable to future engineering work")),
                ("non_obvious_trade_offs", nonempty_string_array_schema("Non-obvious trade-offs between credible alternatives")),
            ],
            &["state", "rationale", "consequence_significance", "transferable_principles", "non_obvious_trade_offs"],
        ),
    ]})
}

fn learning_deliberation_schemas() -> Vec<Value> {
    let project_candidate = || {
        vec![
            ("project_id", identity_schema("Project identity")),
            (
                "deliberation_candidate_id",
                identity_schema("Exact Learning Deliberation Candidate identity"),
            ),
        ]
    };
    let selections = || {
        json!({
            "type":"array",
            "description":"One exact alternative selection for every discovered choice in the deliberation",
            "minItems":1,
            "maxItems":64,
            "items":object_schema(
                vec![
                    ("choice_id", text_schema("Exact discovered choice identity", 1, 256)),
                    ("alternative_id", text_schema("Exact alternative identity", 1, 256)),
                ],
                &["choice_id", "alternative_id"],
            ),
        })
    };
    let simple_candidate = |action: &'static str| {
        let mut fields = project_candidate();
        fields.push((
            "action",
            enum_schema("Learning Deliberation action", &[action]),
        ));
        object_schema(
            fields,
            &["action", "project_id", "deliberation_candidate_id"],
        )
    };
    let user_response = |action: &'static str,
                         mut extra: Vec<(&'static str, Value)>,
                         mut required: Vec<&'static str>| {
        let mut fields = project_candidate();
        fields.push((
            "action",
            enum_schema("Learning Deliberation action", &[action]),
        ));
        fields.push(("user_turn", user_turn_schema()));
        fields.push((
            "user_rationale",
            text_schema(
                "Optional current-user reasoning preserved before agent feedback",
                1,
                16_384,
            ),
        ));
        fields.append(&mut extra);
        required.extend([
            "action",
            "project_id",
            "deliberation_candidate_id",
            "user_turn",
        ]);
        object_schema(fields, &required)
    };
    vec![
        object_schema(
            vec![
                (
                    "action",
                    enum_schema("Learning Deliberation action", &["begin"]),
                ),
                ("project_id", identity_schema("Project identity")),
                (
                    "review_candidate_id",
                    identity_schema("Exact Materiality Review Candidate identity"),
                ),
                (
                    "dimension_id",
                    text_schema(
                        "Exact agent-owned deliberation-worthy dimension identity",
                        1,
                        256,
                    ),
                ),
                (
                    "source_operation",
                    text_schema("Inspectable bounded learning operation", 1, 4096),
                ),
                (
                    "problem",
                    text_schema(
                        "Neutral problem statement without agent recommendation",
                        1,
                        4096,
                    ),
                ),
                (
                    "established_facts",
                    nonempty_string_array_schema(
                        "Established facts available before user reasoning",
                    ),
                ),
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
        simple_candidate("inspect"),
        user_response(
            "respond_select",
            vec![("selections", selections())],
            vec!["selections"],
        ),
        user_response("respond_delegate", Vec::new(), Vec::new()),
        user_response("respond_skip", Vec::new(), Vec::new()),
        user_response(
            "respond_research_or_prototype",
            vec![(
                "evidence_state",
                enum_schema(
                    "Requested evidence path",
                    &["research_required", "prototype_required"],
                ),
            )],
            vec!["evidence_state"],
        ),
        {
            let mut fields = project_candidate();
            fields.extend([
                (
                    "action",
                    enum_schema("Learning Deliberation action", &["feedback"]),
                ),
                (
                    "feedback",
                    text_schema(
                        "Educational feedback recorded only after the initial user response",
                        1,
                        16_384,
                    ),
                ),
                ("recommendation_selections", selections()),
                (
                    "recommendation_rationale",
                    text_schema(
                        "Bounded post-response agent recommendation rationale",
                        1,
                        16_384,
                    ),
                ),
            ]);
            object_schema(
                fields,
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "feedback",
                    "recommendation_selections",
                    "recommendation_rationale",
                ],
            )
        },
        simple_candidate("complete"),
        {
            let mut fields = project_candidate();
            fields.extend([
                (
                    "action",
                    enum_schema("Learning Deliberation action", &["reconsider"]),
                ),
                ("user_turn", user_turn_schema()),
                (
                    "rationale",
                    text_schema("Explicit current-user reconsideration rationale", 1, 16_384),
                ),
            ]);
            object_schema(
                fields,
                &[
                    "action",
                    "project_id",
                    "deliberation_candidate_id",
                    "user_turn",
                    "rationale",
                ],
            )
        },
    ]
}

fn materiality_judgment_schema(
    disposition: &'static str,
    mut fields: Vec<(&'static str, Value)>,
    required_fields: &[&'static str],
) -> Value {
    let mut common = vec![
        (
            "choice_id",
            text_schema("Exact Engineering Choice Discovery identity", 1, 256),
        ),
        (
            "disposition",
            enum_schema("Authority disposition", &[disposition]),
        ),
        (
            "basis_summary",
            text_schema("Why the exact authority disposition applies", 1, 4096),
        ),
        (
            "additional_source_ids",
            identity_array_schema(
                "Additional current canonical Source identities beyond discovery-owned Sources",
                0,
            ),
        ),
        ("learning_value", learning_value_schema()),
    ];
    common.append(&mut fields);
    let mut required = vec![
        "choice_id",
        "disposition",
        "basis_summary",
        "learning_value",
    ];
    required.extend_from_slice(required_fields);
    object_schema(common, &required)
}

#[derive(Clone)]
struct MaterialityJudgmentContract {
    variant_id: &'static str,
    schema: Value,
}

fn materiality_judgment_contracts() -> Vec<MaterialityJudgmentContract> {
    let contract_basis =
        || nonempty_string_array_schema("Exact accepted contract references settling this choice");
    let decision_ids = || identity_array_schema("Exact current applicable Decision identities", 1);
    let research_basis =
        || nonempty_string_array_schema("Bounded research, prototype, defer, or revisit basis");
    let delegation_statement = || {
        text_schema(
            "Bounded verbatim statement from the draft's exact current Goal/user-turn claimed as delegation; semantic judgment remains explicit",
            1,
            4096,
        )
    };
    let delegated_scope = || {
        nonempty_string_array_schema(
            "Bounded scope delegated by the verbatim current-task statement",
        )
    };
    let contract = |variant_id, schema| MaterialityJudgmentContract { variant_id, schema };
    vec![
        contract(
            "repository_or_environment_fact",
            materiality_judgment_schema("repository_or_environment_fact", vec![], &[]),
        ),
        contract(
            "settled_authority_by_contract",
            materiality_judgment_schema(
                "settled_authority",
                vec![("contract_basis", contract_basis())],
                &["contract_basis"],
            ),
        ),
        contract(
            "settled_authority_by_decision",
            materiality_judgment_schema(
                "settled_authority",
                vec![("decision_ids", decision_ids())],
                &["decision_ids"],
            ),
        ),
        contract(
            "settled_authority_by_contract_and_decision",
            materiality_judgment_schema(
                "settled_authority",
                vec![
                    ("contract_basis", contract_basis()),
                    ("decision_ids", decision_ids()),
                ],
                &["contract_basis", "decision_ids"],
            ),
        ),
        contract(
            "agent_owned_implementation_choice",
            materiality_judgment_schema("agent_owned_implementation_choice", vec![], &[]),
        ),
        contract(
            "delegated_implementation_choice_current_task",
            materiality_judgment_schema(
                "delegated_implementation_choice",
                vec![
                    ("delegation_statement", delegation_statement()),
                    ("delegated_scope", delegated_scope()),
                ],
                &["delegation_statement", "delegated_scope"],
            ),
        ),
        contract(
            "delegated_implementation_choice_inquiry_time",
            materiality_judgment_schema(
                "delegated_implementation_choice",
                vec![("decision_ids", decision_ids())],
                &["decision_ids"],
            ),
        ),
        contract(
            "exploratory_uncertainty_research_required",
            materiality_judgment_schema(
                "exploratory_uncertainty",
                vec![
                    (
                        "exploratory_disposition",
                        enum_schema("Exploratory treatment", &["research_required"]),
                    ),
                    ("research_basis", research_basis()),
                ],
                &["exploratory_disposition", "research_basis"],
            ),
        ),
        contract(
            "exploratory_uncertainty_prototype_required",
            materiality_judgment_schema(
                "exploratory_uncertainty",
                vec![
                    (
                        "exploratory_disposition",
                        enum_schema("Exploratory treatment", &["prototype_required"]),
                    ),
                    ("research_basis", research_basis()),
                ],
                &["exploratory_disposition", "research_basis"],
            ),
        ),
        contract(
            "exploratory_uncertainty_deferred_with_revisit",
            materiality_judgment_schema(
                "exploratory_uncertainty",
                vec![
                    (
                        "exploratory_disposition",
                        enum_schema("Exploratory treatment", &["deferred_with_revisit"]),
                    ),
                    ("research_basis", research_basis()),
                ],
                &["exploratory_disposition", "research_basis"],
            ),
        ),
        contract(
            "exploratory_uncertainty_resolved_by_research",
            materiality_judgment_schema(
                "exploratory_uncertainty",
                vec![
                    (
                        "exploratory_disposition",
                        enum_schema("Exploratory treatment", &["resolved_by_research"]),
                    ),
                    ("research_basis", research_basis()),
                ],
                &["exploratory_disposition", "research_basis"],
            ),
        ),
        contract(
            "unresolved_user_owned_outcome",
            materiality_judgment_schema("unresolved_user_owned_outcome", vec![], &[]),
        ),
        contract(
            "resolved_user_owned_outcome",
            materiality_judgment_schema(
                "unresolved_user_owned_outcome",
                vec![(
                    "resolution_decision_id",
                    identity_schema("Exact applicable current-host Decision resolving this choice"),
                )],
                &["resolution_decision_id"],
            ),
        ),
    ]
}

fn materiality_judgment_schemas() -> Vec<Value> {
    materiality_judgment_contracts()
        .into_iter()
        .map(|contract| contract.schema)
        .collect()
}

fn materiality_judgments_schema() -> Value {
    json!({
        "type":"array",
        "description":"One caller-owned authority and learning judgment for every discovery-owned choice",
        "minItems":1,
        "maxItems":64,
        "items":{"oneOf":materiality_judgment_schemas()},
    })
}

fn materiality_record_schema() -> Value {
    object_schema(
        vec![
            (
                "action",
                enum_schema("Materiality Review action", &["record"]),
            ),
            ("project_id", identity_schema("Project identity")),
            (
                "engineering_choice_discovery_candidate_id",
                identity_schema("Exact Engineering Choice Discovery Candidate identity"),
            ),
            (
                "rationale",
                text_schema("Bounded review rationale", 1, 4096),
            ),
            ("learning_participation", learning_participation_schema()),
            ("judgments", materiality_judgments_schema()),
        ],
        &[
            "action",
            "project_id",
            "engineering_choice_discovery_candidate_id",
            "rationale",
            "learning_participation",
            "judgments",
        ],
    )
}

fn materiality_revise_schema() -> Value {
    object_schema(
        vec![
            (
                "action",
                enum_schema("Materiality Review action", &["revise"]),
            ),
            ("project_id", identity_schema("Project identity")),
            (
                "review_candidate_id",
                identity_schema("Materiality Review Candidate identity"),
            ),
            (
                "rationale",
                text_schema("Bounded revision rationale", 1, 4096),
            ),
            ("learning_participation", learning_participation_schema()),
            ("judgments", materiality_judgments_schema()),
        ],
        &[
            "action",
            "project_id",
            "review_candidate_id",
            "rationale",
            "learning_participation",
            "judgments",
        ],
    )
}

fn materiality_review_schemas() -> Vec<Value> {
    let mut inspect_fields = vec![
        ("project_id", identity_schema("Project identity")),
        (
            "goal_context_id",
            identity_schema("Canonical Goal Context identity"),
        ),
        (
            "baseline_analysis_snapshot_id",
            digest_identity_schema("Exact pre-work Analysis Snapshot identity"),
        ),
    ];
    inspect_fields.extend([
        (
            "action",
            enum_schema("Materiality Review action", &["inspect"]),
        ),
        (
            "paths",
            string_array_schema("Affected repository paths for Decision applicability"),
        ),
        (
            "components",
            string_array_schema("Affected components for Decision applicability"),
        ),
        (
            "work_contexts",
            string_array_schema("Current work contexts for Decision applicability"),
        ),
        (
            "met_revisit_triggers",
            string_array_schema("Known met Decision revisit triggers"),
        ),
    ]);
    vec![
        object_schema(
            vec![
                (
                    "action",
                    enum_schema("Materiality Review action", &["draft"]),
                ),
                ("project_id", identity_schema("Project identity")),
                (
                    "engineering_choice_discovery_candidate_id",
                    identity_schema("Exact Engineering Choice Discovery Candidate identity"),
                ),
            ],
            &[
                "action",
                "project_id",
                "engineering_choice_discovery_candidate_id",
            ],
        ),
        materiality_record_schema(),
        materiality_revise_schema(),
        object_schema(
            inspect_fields,
            &[
                "action",
                "project_id",
                "goal_context_id",
                "baseline_analysis_snapshot_id",
            ],
        ),
    ]
}

fn candidate_management_schemas() -> Vec<Value> {
    let submit = object_schema(
        vec![
            (
                "action",
                enum_schema("Candidate lifecycle action", &["submit_question"]),
            ),
            ("project_id", identity_schema("Project identity")),
            (
                "source_ids",
                identity_array_schema("Canonical Source identities supporting the Candidate", 1),
            ),
            (
                "source_operation",
                text_schema(
                    "Inspectable operation or inquiry scope that collected the Candidate",
                    1,
                    4096,
                ),
            ),
            (
                "repository_snapshot",
                text_schema("Optional repository snapshot basis", 1, 4096),
            ),
            (
                "research_state",
                enum_schema(
                    "Explicit repository/environment research requirement",
                    &["research_required", "ready_to_ask"],
                ),
            ),
            (
                "research_state_basis",
                text_schema(
                    "Why repository/environment research is required or unnecessary",
                    1,
                    4096,
                ),
            ),
            (
                "retention_basis",
                text_schema("Candidate retention-policy basis", 1, 4096),
            ),
            (
                "bounded_summary",
                text_schema("Bounded Candidate summary", 1, 4096),
            ),
            (
                "prompt",
                text_schema("Proposed material Question wording", 1, 4096),
            ),
            (
                "why_now",
                text_schema(
                    "Why an externally meaningful user-owned outcome materially matters now",
                    1,
                    4096,
                ),
            ),
            (
                "affected_scope",
                nonempty_string_array_schema("Affected material scopes"),
            ),
            (
                "established_facts",
                string_array_schema("Source-grounded established facts"),
            ),
            ("assumptions", string_array_schema("Known assumptions")),
            ("uncertainty", string_array_schema("Known uncertainty")),
            ("alternatives", candidate_alternatives_schema()),
            (
                "recommendation_key",
                text_schema("Agent-recommended submitted alternative key", 1, 1024),
            ),
            (
                "recommendation_rationale",
                text_schema("Agent recommendation rationale", 1, 4096),
            ),
            (
                "trade_offs",
                string_array_schema("Trade-offs of the material choice"),
            ),
            (
                "known_limits",
                string_array_schema("Known limits of the Candidate basis"),
            ),
            (
                "what_unlocks",
                string_array_schema("Work unlocked by an explicit response"),
            ),
            (
                "materiality_rationale",
                text_schema(
                    "Consequence-and-ownership rationale after owner, Decision, contract, and repository-fact inspection",
                    1,
                    4096,
                ),
            ),
            (
                "duplicate_basis",
                text_schema(
                    "Basis for concluding no canonical duplicate exists",
                    1,
                    4096,
                ),
            ),
            (
                "presentation_order",
                unsigned_schema("Explicit deterministic presentation order", 1),
            ),
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
    );
    let candidate_action = |action: &'static str, detail: (&'static str, Value)| {
        let (detail_name, detail_schema) = detail;
        object_schema(
            vec![
                (
                    "action",
                    enum_schema("Candidate lifecycle action", &[action]),
                ),
                ("project_id", identity_schema("Project identity")),
                ("candidate_id", identity_schema("Candidate identity")),
                (detail_name, detail_schema),
            ],
            &["action", "project_id", "candidate_id", detail_name],
        )
    };
    vec![
        submit,
        object_schema(
            vec![
                (
                    "action",
                    enum_schema(
                        "Candidate lifecycle action",
                        &["submit_question_from_materiality"],
                    ),
                ),
                ("project_id", identity_schema("Project identity")),
                (
                    "review_candidate_id",
                    identity_schema("Source Materiality Review Candidate identity"),
                ),
                (
                    "dimension_id",
                    text_schema("Unresolved user-owned Materiality Review dimension", 1, 256),
                ),
                (
                    "research_state",
                    enum_schema(
                        "Explicit repository/environment research requirement",
                        &["research_required", "ready_to_ask"],
                    ),
                ),
                (
                    "research_state_basis",
                    text_schema(
                        "Why repository/environment research is required or unnecessary",
                        1,
                        4096,
                    ),
                ),
                (
                    "retention_basis",
                    text_schema("Candidate retention-policy basis", 1, 4096),
                ),
                (
                    "bounded_summary",
                    text_schema("Bounded Candidate summary", 1, 4096),
                ),
                (
                    "prompt",
                    text_schema("Proposed material Question wording", 1, 4096),
                ),
                (
                    "why_now",
                    text_schema("Why the reviewed user-owned outcome matters now", 1, 4096),
                ),
                (
                    "established_facts",
                    string_array_schema("Source-grounded established facts"),
                ),
                ("assumptions", string_array_schema("Known assumptions")),
                ("uncertainty", string_array_schema("Known uncertainty")),
                ("alternatives", candidate_alternatives_schema()),
                (
                    "recommendation_key",
                    text_schema("Agent-recommended submitted alternative key", 1, 1024),
                ),
                (
                    "recommendation_rationale",
                    text_schema("Agent recommendation rationale", 1, 4096),
                ),
                (
                    "trade_offs",
                    string_array_schema("Trade-offs of the material choice"),
                ),
                (
                    "known_limits",
                    string_array_schema("Known limits of the Candidate basis"),
                ),
                (
                    "what_unlocks",
                    string_array_schema("Work unlocked by an explicit response"),
                ),
                (
                    "duplicate_basis",
                    text_schema(
                        "Basis for concluding no canonical duplicate exists",
                        1,
                        4096,
                    ),
                ),
                (
                    "presentation_order",
                    unsigned_schema("Explicit deterministic presentation order", 1),
                ),
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
        object_schema(
            vec![
                (
                    "action",
                    enum_schema(
                        "Candidate lifecycle action",
                        &["attach_repository_research"],
                    ),
                ),
                ("project_id", identity_schema("Project identity")),
                ("candidate_id", identity_schema("Candidate identity")),
                (
                    "capability",
                    text_schema("Repository Intelligence capability used", 1, 4096),
                ),
                (
                    "coverage",
                    text_schema("Inspectable research coverage", 1, 4096),
                ),
                (
                    "freshness",
                    enum_schema("Research freshness", &["current", "stale", "unknown"]),
                ),
                (
                    "source_ids",
                    identity_array_schema("Canonical Sources supporting the research", 1),
                ),
                (
                    "evidence_assessment",
                    enum_schema(
                        "Whether this evidence is sufficient to finish research",
                        &["sufficient", "insufficient"],
                    ),
                ),
                ("limits", string_array_schema("Known research limits")),
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
        object_schema(
            vec![
                (
                    "action",
                    enum_schema("Candidate lifecycle action", &["mark_research_ready"]),
                ),
                ("project_id", identity_schema("Project identity")),
                ("candidate_id", identity_schema("Candidate identity")),
            ],
            &["action", "project_id", "candidate_id"],
        ),
        object_schema(
            vec![
                (
                    "action",
                    enum_schema("Candidate lifecycle action", &["promote_question"]),
                ),
                ("project_id", identity_schema("Project identity")),
                ("candidate_id", identity_schema("Candidate identity")),
            ],
            &["action", "project_id", "candidate_id"],
        ),
        candidate_action(
            "dismiss",
            (
                "reason",
                text_schema("Explicit non-promotion disposition reason", 1, 4096),
            ),
        ),
        candidate_action(
            "delete",
            (
                "basis",
                text_schema("Explicit Candidate-local content deletion basis", 1, 4096),
            ),
        ),
    ]
}

fn canonical_mutation_schemas() -> Vec<Value> {
    let common = || {
        vec![
            ("project_id", identity_schema("Project identity")),
            ("user_turn", user_turn_schema()),
            ("record_id", identity_schema("Canonical record identity")),
        ]
    };
    let correction = |action: &'static str, description: &'static str| {
        let mut fields = common();
        fields.push((
            "action",
            enum_schema("Canonical mutation action", &[action]),
        ));
        fields.push((
            "expected_revision",
            unsigned_schema("Expected record revision", 1),
        ));
        fields.push(("corrected_text", text_schema(description, 1, 16_384)));
        object_schema(
            fields,
            &[
                "action",
                "project_id",
                "user_turn",
                "record_id",
                "expected_revision",
                "corrected_text",
            ],
        )
    };
    let mut supersede = common();
    supersede.push((
        "action",
        enum_schema("Canonical mutation action", &["supersede_decision"]),
    ));
    supersede.push((
        "alternative_key",
        text_schema("New displayed alternative key", 1, 1024),
    ));
    supersede.push((
        "rationale",
        text_schema("Optional user rationale", 1, 16_384),
    ));
    let mut forget = common();
    forget.push((
        "action",
        enum_schema("Canonical mutation action", &["forget"]),
    ));
    forget.push((
        "record_kind",
        enum_schema(
            "Forgettable canonical record kind",
            &[
                "source",
                "question",
                "decision",
                "context_item",
                "checkpoint",
            ],
        ),
    ));
    vec![
        correction("correct_context", "Corrected Context Item statement"),
        correction("correct_decision", "Corrected Decision rationale"),
        object_schema(
            supersede,
            &[
                "action",
                "project_id",
                "user_turn",
                "record_id",
                "alternative_key",
            ],
        ),
        object_schema(
            forget,
            &[
                "action",
                "project_id",
                "user_turn",
                "record_id",
                "record_kind",
            ],
        ),
    ]
}

fn guarded_interaction_schemas() -> Vec<Value> {
    vec![
        object_schema(
            vec![(
                "confirmation_request_id",
                identity_schema("Guarded confirmation request identity"),
            )],
            &["confirmation_request_id"],
        ),
        object_schema(
            vec![
                (
                    "confirmation_request_id",
                    identity_schema("Guarded confirmation request identity"),
                ),
                (
                    "request_revision",
                    unsigned_schema("Exact displayed request revision", 1),
                ),
                ("effect_fingerprint", fingerprint_schema()),
                (
                    "decision",
                    enum_schema("Explicit current-host response", &["confirm", "deny"]),
                ),
                ("user_turn", user_turn_schema()),
            ],
            &[
                "confirmation_request_id",
                "request_revision",
                "effect_fingerprint",
                "decision",
                "user_turn",
            ],
        ),
    ]
}

fn project_schema() -> Value {
    object_schema(
        vec![("project_id", identity_schema("Project identity"))],
        &["project_id"],
    )
}

fn object_schema(fields: Vec<(&str, Value)>, required: &[&str]) -> Value {
    let properties = fields
        .into_iter()
        .map(|(name, schema)| (name.to_owned(), schema))
        .collect::<serde_json::Map<_, _>>();
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

fn identity_schema(description: &str) -> Value {
    json!({
        "type": "string",
        "description": description,
        "pattern": "^[0-9a-fA-F]{32}$",
    })
}

fn digest_identity_schema(description: &str) -> Value {
    json!({
        "type": "string",
        "description": description,
        "pattern": "^[0-9a-fA-F]{64}$",
    })
}

fn fingerprint_schema() -> Value {
    json!({
        "type": "string",
        "description": "Exact Guarded effect fingerprint",
        "pattern": "^sha256:[0-9a-f]{64}$",
    })
}

fn text_schema(description: &str, minimum: usize, maximum: usize) -> Value {
    json!({
        "type": "string",
        "description": description,
        "minLength": minimum,
        "maxLength": maximum,
    })
}

fn user_turn_schema() -> Value {
    text_schema("Explicit current-host user turn", 1, 16_384)
}

fn unsigned_schema(description: &str, minimum: u64) -> Value {
    json!({
        "type": "integer",
        "description": description,
        "minimum": minimum,
    })
}

fn string_array_schema(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": {"type": "string", "minLength": 1, "maxLength": 4096},
    })
}

fn nonempty_string_array_schema(description: &str) -> Value {
    let mut schema = string_array_schema(description);
    schema["minItems"] = json!(1);
    schema
}

fn identity_array_schema(description: &str, minimum: usize) -> Value {
    json!({
        "type": "array",
        "description": description,
        "minItems": minimum,
        "items": identity_schema(description),
    })
}

fn checkpoint_verification_schema() -> Value {
    json!({
        "type": "array",
        "description": "Independent verification observations; executed states require command evidence",
        "minItems": 1,
        "items": {
            "oneOf": [
                object_schema(
                    vec![("state", enum_schema("Verification state", &["not_run"]))],
                    &["state"],
                ),
                object_schema(
                    vec![
                        ("state", enum_schema("Incomplete observed verification state", &["partial"])),
                        ("command_label", text_schema("Bounded human-readable command description; presentation only, never the machine correlation key", 1, 1024)),
                        ("command_invocation", text_schema("Exact transient host command invocation used only by Volicord to derive the persisted SHA-256 correlation fingerprint; raw invocation is not retained", 1, 16_384)),
                        ("exit_code", unsigned_schema("Actual process exit code when termination is exited", 0)),
                        ("termination", enum_schema("Actual command termination", &["exited", "signaled", "spawn_failed", "indeterminate"])),
                        ("outcome", text_schema("Observed verification outcome", 1, 16_384)),
                    ],
                    &["state", "command_label", "command_invocation", "termination", "outcome"],
                ),
                object_schema(
                    vec![
                        ("state", enum_schema("Executed verification state with a numeric observed exit status", &["passed", "failed"])),
                        ("command_label", text_schema("Bounded human-readable command description; presentation only, never the machine correlation key", 1, 1024)),
                        ("command_invocation", text_schema("Exact transient host command invocation used only by Volicord to derive the persisted SHA-256 correlation fingerprint; raw invocation is not retained", 1, 16_384)),
                        ("exit_code", unsigned_schema("Numeric exit status from this same observed command execution", 0)),
                        ("termination", enum_schema("Observed command termination", &["exited"])),
                        ("outcome", text_schema("Observed verification outcome; output text alone is insufficient", 1, 16_384)),
                    ],
                    &["state", "command_label", "command_invocation", "exit_code", "termination", "outcome"],
                ),
            ]
        }
    })
}

fn candidate_alternatives_schema() -> Value {
    json!({
        "type": "array",
        "description": "Displayed Question alternatives",
        "minItems": 1,
        "items": object_schema(
            vec![
                ("key", text_schema("Stable alternative key", 1, 1024)),
                ("label", text_schema("User-facing alternative label", 1, 4096)),
                ("consequence", text_schema("Consequence of choosing the alternative", 1, 4096)),
            ],
            &["key", "label", "consequence"],
        ),
    })
}

fn enum_schema(description: &str, values: &[&str]) -> Value {
    json!({
        "type": "string",
        "description": description,
        "enum": values,
    })
}

fn validate_schema(schema: &Value, value: &Value, path: &str) -> Result<(), Vec<String>> {
    if let Some(variants) = schema.get("oneOf").and_then(Value::as_array) {
        let outcomes = variants
            .iter()
            .map(|variant| validate_schema(variant, value, path))
            .collect::<Vec<_>>();
        let success_count = outcomes.iter().filter(|outcome| outcome.is_ok()).count();
        return match success_count {
            1 => Ok(()),
            0 => {
                let selected = ["action", "disposition", "state"]
                    .into_iter()
                    .find_map(|key| {
                        let discriminator = value.get(key).and_then(Value::as_str)?;
                        let matched = variants
                            .iter()
                            .zip(outcomes.iter())
                            .filter(|(variant, _)| {
                                variant_discriminator_matches(variant, key, discriminator)
                            })
                            .filter_map(|(_, outcome)| outcome.as_ref().err().cloned())
                            .flatten()
                            .collect::<Vec<_>>();
                        (!matched.is_empty()).then_some(matched)
                    });
                let mut problems = selected.unwrap_or_else(|| {
                    outcomes
                        .into_iter()
                        .filter_map(Result::err)
                        .flatten()
                        .collect()
                });
                problems.truncate(16);
                if problems.is_empty() {
                    problems.push(format!("{path} does not match any allowed shape"));
                }
                Err(problems)
            }
            _ => Err(vec![format!("{path} matches more than one allowed shape")]),
        };
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => validate_object(schema, value, path),
        Some("string") => validate_string(schema, value, path),
        Some("array") => validate_array(schema, value, path),
        Some("integer") => validate_integer(schema, value, path),
        Some(kind) => Err(vec![format!("{path} uses unsupported schema type {kind}")]),
        None => Err(vec![format!("{path} schema has no type")]),
    }
}

fn variant_discriminator_matches(schema: &Value, key: &str, value: &str) -> bool {
    schema
        .get("properties")
        .and_then(|properties| properties.get(key))
        .and_then(|discriminator_schema| discriminator_schema.get("enum"))
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values
                .iter()
                .any(|candidate| candidate.as_str() == Some(value))
        })
}

fn validate_object(schema: &Value, value: &Value, path: &str) -> Result<(), Vec<String>> {
    let Some(object) = value.as_object() else {
        return Err(vec![format!("{path} must be an object")]);
    };
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return Err(vec![format!("{path} schema has no properties")]);
    };
    let mut problems = Vec::new();
    if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
        for name in object.keys().filter(|name| !properties.contains_key(*name)) {
            problems.push(format!("{path}.{name} is not allowed"));
        }
    }
    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for name in required.iter().filter_map(Value::as_str) {
            if !object.contains_key(name) {
                problems.push(format!("{path}.{name} is required"));
            }
        }
    }
    for (name, child) in object {
        if let Some(child_schema) = properties.get(name) {
            if let Err(mut child_problems) =
                validate_schema(child_schema, child, &format!("{path}.{name}"))
            {
                problems.append(&mut child_problems);
            }
        }
    }
    problems.truncate(16);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

fn validate_string(schema: &Value, value: &Value, path: &str) -> Result<(), Vec<String>> {
    let Some(text) = value.as_str() else {
        return Err(vec![format!("{path} must be a string")]);
    };
    let length = text.chars().count() as u64;
    if let Some(minimum) = schema.get("minLength").and_then(Value::as_u64) {
        if length < minimum {
            return Err(vec![format!("{path} is shorter than {minimum} characters")]);
        }
    }
    if let Some(maximum) = schema.get("maxLength").and_then(Value::as_u64) {
        if length > maximum {
            return Err(vec![format!("{path} exceeds {maximum} characters")]);
        }
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        if !values
            .iter()
            .any(|candidate| candidate.as_str() == Some(text))
        {
            let allowed = values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(vec![format!(
                "{path} value {text:?} is not an allowed value; allowed values: {allowed}"
            )]);
        }
    }
    match schema.get("pattern").and_then(Value::as_str) {
        Some("^[0-9a-fA-F]{32}$") if !is_hex(text, 32, true) => {
            Err(vec![format!("{path} must contain 32 hexadecimal digits")])
        }
        Some("^sha256:[0-9a-f]{64}$")
            if !text
                .strip_prefix("sha256:")
                .is_some_and(|value| is_hex(value, 64, false)) =>
        {
            Err(vec![format!("{path} must be a sha256 fingerprint")])
        }
        Some("^[0-9a-fA-F]{64}$") if !is_hex(text, 64, true) => {
            Err(vec![format!("{path} must contain 64 hexadecimal digits")])
        }
        Some("^[0-9a-fA-F]{32}$" | "^[0-9a-fA-F]{64}$" | "^sha256:[0-9a-f]{64}$") | None => Ok(()),
        Some(pattern) => Err(vec![format!(
            "{path} uses unsupported schema pattern {pattern}"
        )]),
    }
}

fn validate_array(schema: &Value, value: &Value, path: &str) -> Result<(), Vec<String>> {
    let Some(values) = value.as_array() else {
        return Err(vec![format!("{path} must be an array")]);
    };
    let mut problems = Vec::new();
    if let Some(minimum) = schema.get("minItems").and_then(Value::as_u64) {
        if values.len() < minimum as usize {
            problems.push(format!("{path} must contain at least {minimum} items"));
        }
    }
    if let Some(maximum) = schema.get("maxItems").and_then(Value::as_u64) {
        if values.len() > maximum as usize {
            problems.push(format!("{path} must contain at most {maximum} items"));
        }
    }
    let Some(item_schema) = schema.get("items") else {
        problems.push(format!("{path} schema has no item contract"));
        return Err(problems);
    };
    for (index, item) in values.iter().enumerate() {
        if let Err(mut child_problems) =
            validate_schema(item_schema, item, &format!("{path}[{index}]"))
        {
            problems.append(&mut child_problems);
        }
    }
    problems.truncate(16);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

fn validate_integer(schema: &Value, value: &Value, path: &str) -> Result<(), Vec<String>> {
    let Some(number) = value.as_u64() else {
        return Err(vec![format!("{path} must be an unsigned integer")]);
    };
    if let Some(minimum) = schema.get("minimum").and_then(Value::as_u64) {
        if number < minimum {
            return Err(vec![format!("{path} must be at least {minimum}")]);
        }
    }
    Ok(())
}

fn is_hex(value: &str, length: usize, uppercase_allowed: bool) -> bool {
    value.len() == length
        && value.bytes().all(|byte| {
            byte.is_ascii_digit()
                || (b'a'..=b'f').contains(&byte)
                || (uppercase_allowed && (b'A'..=b'F').contains(&byte))
        })
}

fn tool_result(value: Value, is_error: bool) -> Value {
    json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".into())}],"structuredContent":value,"isError":is_error})
}

fn guarded_candidate_json(candidate: &volicord_operations::GuardedEffectCandidate) -> Value {
    json!({
        "confirmation_request_id":candidate.confirmation_request_identity.to_string(),
        "request_revision":candidate.request_revision,
        "project_id":candidate.project_id.to_string(),
        "exact_action":candidate.exact_action,
        "target":candidate.target,
        "expected_effect":candidate.expected_effect,
        "risk_category":"personal_data_or_source_code_external_transmission",
        "risk_consequence":candidate.risk.concrete_consequence,
        "scope":candidate.scope,
        "expiration_unix_micros":candidate.expires_at.as_unix_micros(),
        "effect_fingerprint":candidate.effect_fingerprint,
        "requesting_actor":format!("{:?}:{}", candidate.requesting_provenance.actor.kind, candidate.requesting_provenance.actor.identity),
        "requesting_provenance":candidate.requesting_provenance.basis,
    })
}

fn provider_request_json(record: &ProviderRequestRecord) -> Value {
    json!({
        "provider_request_id":record.id.to_string(),
        "project_id":record.project_id.to_string(),
        "opt_in_revision":record.opt_in_revision,
        "repository_snapshot":record.repository_snapshot.to_string(),
        "analysis_snapshot":record.analysis_snapshot.to_string(),
        "provider":record.provider,
        "model":record.model,
        "purpose":record.purpose,
        "requested_capability":record.requested_capability,
        "requested_source_scopes":record.requested_source_scopes,
        "outcome":provider_outcome_name(record.outcome),
        "diagnostic":record.diagnostic,
        "requested_at_unix_micros":record.requested_at.as_unix_micros(),
        "completed_at_unix_micros":record.completed_at.map(TimestampMicros::as_unix_micros),
        "manifest":record.manifest.iter().map(|entry| json!({
            "source_id":entry.source.identity().to_string(),
            "locator":entry.locator,
            "class":source_class_name(entry.class),
            "scope_outcome":scope_outcome_name(entry.scope_outcome),
            "filter_outcome":filter_outcome_name(entry.filter_outcome),
            "transmission_outcome":if entry.transmission_outcome == TransmissionOutcome::Transmitted { "transmitted" } else { "not_transmitted" },
            "original_bytes":entry.original_bytes,
            "transmitted_bytes":entry.transmitted_bytes,
            "filtered_line_count":entry.filtered_line_count,
            "reason":entry.reason,
        })).collect::<Vec<_>>()
    })
}

fn guarded_provider_inspection_json(inspection: &GuardedProviderInspection) -> Value {
    let (outcome, next_safe_action) = match &inspection.operation.outcome {
        GuardedOperationOutcome::NotDispatched {
            rejection,
            confirmation_consumed,
            diagnostic,
        } => (
            json!({
                "kind":"not_dispatched",
                "rejection":rejection.map(confirmation_rejection_name),
                "confirmation_consumed":confirmation_consumed,
                "diagnostic":diagnostic,
            }),
            match rejection {
                Some(ConfirmationRejection::Missing) => "record an exact current-host confirmation, then explicitly dispatch again",
                Some(ConfirmationRejection::Reused) => "do not retry; the confirmation is single-use and already consumed",
                Some(_) => "prepare or revise the exact request and obtain a new explicit confirmation",
                None => "inspect the provider outcome; any new attempt requires a new preparation and confirmation",
            },
        ),
        GuardedOperationOutcome::DispatchedAndCompleted { diagnostic } => (
            json!({"kind":"dispatched_and_completed","diagnostic":diagnostic}),
            "no retry is required",
        ),
        GuardedOperationOutcome::DispatchedAndFailed { diagnostic } => (
            json!({"kind":"dispatched_and_failed","diagnostic":diagnostic}),
            "review the provider failure before preparing any explicit new operation",
        ),
        GuardedOperationOutcome::ExecutionOutcomeIndeterminate { diagnostic } => (
            json!({"kind":"execution_outcome_indeterminate","diagnostic":diagnostic}),
            "do not retry; reconcile external provider state before any new operation",
        ),
    };
    json!({
        "operation_id":inspection.operation.operation_identity.to_string(),
        "confirmation_request_id":inspection.operation.confirmation_request_identity.to_string(),
        "request_revision":inspection.operation.request_revision,
        "user_response_source_id":inspection.operation.user_response_source_id.map(|source| source.to_string()),
        "guarded_outcome":outcome,
        "provider_request":provider_request_json(&inspection.provider_request),
        "exact_request":guarded_candidate_json(&inspection.request),
        "next_safe_action":next_safe_action,
    })
}

const fn confirmation_rejection_name(rejection: ConfirmationRejection) -> &'static str {
    match rejection {
        ConfirmationRejection::Missing => "missing",
        ConfirmationRejection::Denied => "denied",
        ConfirmationRejection::Stale => "stale",
        ConfirmationRejection::Expired => "expired",
        ConfirmationRejection::Mismatched => "mismatched",
        ConfirmationRejection::Reused => "reused",
        ConfirmationRejection::InvalidUserSource => "invalid_user_source",
    }
}

const fn provider_outcome_name(outcome: ProviderRequestOutcome) -> &'static str {
    match outcome {
        ProviderRequestOutcome::Prepared => "prepared",
        ProviderRequestOutcome::NotAuthorized => "not_authorized",
        ProviderRequestOutcome::NotTransmitted => "not_transmitted",
        ProviderRequestOutcome::ProviderUnavailable => "provider_unavailable",
        ProviderRequestOutcome::ProviderFailed => "provider_failed",
        ProviderRequestOutcome::ProviderTimedOut => "provider_timed_out",
        ProviderRequestOutcome::ProviderCancelled => "provider_cancelled",
        ProviderRequestOutcome::Completed => "completed",
        ProviderRequestOutcome::Partial => "partial",
        ProviderRequestOutcome::Stale => "stale",
    }
}

const fn source_class_name(class: SourceClass) -> &'static str {
    match class {
        SourceClass::Source => "source",
        SourceClass::Generated => "generated",
        SourceClass::Vendor => "vendor",
        SourceClass::Binary => "binary",
        SourceClass::Configuration => "configuration",
        SourceClass::Document => "document",
    }
}

const fn scope_outcome_name(outcome: ScopeOutcome) -> &'static str {
    match outcome {
        ScopeOutcome::Included => "included",
        ScopeOutcome::Excluded => "excluded",
        ScopeOutcome::OutsideRequestedScope => "outside_requested_scope",
        ScopeOutcome::OutsideOptInScope => "outside_opt_in_scope",
    }
}

const fn filter_outcome_name(outcome: FilterOutcome) -> &'static str {
    match outcome {
        FilterOutcome::NotApplied => "not_applied",
        FilterOutcome::NoMatch => "no_match",
        FilterOutcome::Filtered => "filtered",
    }
}

fn candidate_inspection_json(candidate: volicord_projections::CandidateInspection) -> Value {
    let candidate_id = candidate.candidate_id;
    let candidate_revision = candidate.revision;
    let origin = candidate.origin.map(|origin| {
        json!({
            "actor_kind": format!("{:?}", origin.actor.kind).to_lowercase(),
            "actor_identity": origin.actor.identity,
            "subsystem": origin.subsystem,
            "session": origin.session,
            "provenance_summary": origin.provenance_summary,
        })
    });
    let collection_scope = candidate.collection_scope.map(|scope| {
        json!({
            "project_id": scope.project_id.to_string(),
            "session": scope.session,
            "source_operation": scope.source_operation,
            "candidate_kind": format!("{:?}", scope.candidate_kind).to_lowercase(),
        })
    });
    let observation_basis = candidate.observation_basis.map(|basis| {
        json!({
            "source_ids": basis.source_basis.into_iter().map(|source| source.to_string()).collect::<Vec<_>>(),
            "repository_snapshot": basis.repository_snapshot,
            "analysis_snapshot": basis.analysis_snapshot,
            "execution": basis.execution,
            "host_turn": basis.host_turn,
            "other": basis.other,
        })
    });
    let retention = candidate.retention.map(|retention| match retention {
        volicord_projections::RetentionInspection::RetainedIndefinitely { basis } => {
            json!({"state":"retained_indefinitely","basis":basis})
        }
        volicord_projections::RetentionInspection::RetainedUntil {
            retained_until,
            expired_at_observation,
            basis,
        } => json!({
            "state":"retained_until",
            "retained_until_unix_micros":retained_until.as_unix_micros(),
            "expired_at_observation":expired_at_observation,
            "basis":basis
        }),
    });
    let cleanup = candidate.cleanup.map(|cleanup| {
        json!({
            "kind": format!("{:?}", cleanup.kind).to_lowercase(),
            "basis": cleanup.basis,
            "cleaned_at_unix_micros": cleanup.cleaned_at.as_unix_micros(),
        })
    });
    let repository_research = candidate
        .repository_research_basis
        .into_iter()
        .map(repository_research_json)
        .collect::<Vec<_>>();
    let explicit_delegation_evidence = candidate
        .explicit_delegation_evidence
        .into_iter()
        .map(|inspection| {
            json!({
                "dimension_id":inspection.dimension_id,
                "goal_context_id":inspection.evidence.goal_context_id.to_string(),
                "user_turn_source_id":inspection.evidence.user_turn_source_id.to_string(),
                "verbatim_statement":inspection.evidence.verbatim_statement,
                "bound_dimension_id":inspection.evidence.dimension_id,
                "discovered_choice_ids":inspection.evidence.discovered_choice_ids,
                "affected_scope":inspection.evidence.affected_scope,
                "material_consequences":inspection.evidence.material_consequences,
                "effect_categories":inspection.evidence.effect_categories.into_iter().map(engineering_effect_category_name).collect::<Vec<_>>(),
                "authority_kind":"explicit_current_task_delegation",
            })
        })
        .collect::<Vec<_>>();
    let engineering_choice_discovery = candidate.engineering_choice_discovery.map(|discovery| {
        json!({
            "goal_context_id":discovery.goal_context_id.to_string(),
            "baseline_analysis_snapshot_id":discovery.baseline_analysis_snapshot_id.to_string(),
            "choices":discovery.choices.iter().map(engineering_choice_json).collect::<Vec<_>>(),
        })
    });
    let materiality_review = candidate.materiality_review.map(|review| json!({
        "goal_context_id":review.goal_context_id.to_string(),
        "baseline_analysis_snapshot_id":review.baseline_analysis_snapshot_id.to_string(),
        "engineering_choice_discovery_candidate_id":review.engineering_choice_discovery_candidate_id.to_string(),
        "learning_participation":match review.learning_participation {
            LearningParticipation::Inactive => json!({"state":"inactive"}),
            LearningParticipation::Active { user_turn_source_id, verbatim_statement } => json!({"state":"active","user_turn_source_id":user_turn_source_id.to_string(),"verbatim_statement":verbatim_statement}),
        },
        "late_authority_corrections":review.late_authority_corrections.iter().map(|correction| json!({
            "dimension_id":correction.dimension_id,
            "detected_analysis_snapshot_id":correction.detected_analysis_snapshot_id.to_string(),
            "affected_changed_paths":correction.affected_changed_paths,
            "authority_effect":"later authority is prospective and cannot certify the earlier affected work",
        })).collect::<Vec<_>>(),
        "dimensions":review.dimensions.iter().map(|dimension| json!({
            "dimension_id":dimension.dimension_id,
            "discovered_choice_ids":dimension.discovered_choice_ids,
            "summary":dimension.summary,
            "affected_scope":dimension.affected_scope,
            "authority_disposition":materiality_disposition_json(&dimension.disposition),
            "learning_value":learning_value_json(&dimension.learning_value),
        })).collect::<Vec<_>>(),
    }));
    let learning_deliberation = candidate
        .learning_deliberation
        .as_ref()
        .zip(candidate_revision)
        .map(|(deliberation, revision)| {
            learning_deliberation_json("inspect", candidate_id, revision, deliberation)
        });
    json!({
        "identity":candidate_id.to_string(),
        "exists":candidate.exists,
        "health":format!("{:?}",candidate.health).to_lowercase(),
        "revision":candidate.revision,
        "kind":candidate.kind.map(|value| format!("{:?}",value).to_lowercase()),
        "origin":origin,
        "collection_scope":collection_scope,
        "observation_basis":observation_basis,
        "created_at_unix_micros":candidate.created_at.map(|value| value.as_unix_micros()),
        "observed_at_unix_micros":candidate.observed_at.map(|value| value.as_unix_micros()),
        "retention":retention,
        "summary":candidate.bounded_summary,
        "research_state":candidate.question_research_state.map(question_research_state_name),
        "repository_research":repository_research,
        "explicit_delegation_evidence":explicit_delegation_evidence,
        "engineering_choice_discovery":engineering_choice_discovery,
        "materiality_review":materiality_review,
        "learning_deliberation":learning_deliberation,
        "content_omission":candidate.content_omission.map(|value| format!("{:?}",value).to_lowercase()),
        "content_cleaned":candidate.content_cleaned,
        "cleanup":cleanup,
        "disposition":candidate.promotion_disposition.as_ref().map(candidate_disposition_json),
        "promotion_target":candidate.promotion_target.map(|value| value.to_string()),
        "applicable_opt_out":candidate.current_applicable_opt_out.into_iter().map(collection_opt_out_json).collect::<Vec<_>>(),
    })
}

fn candidate_research_lifecycle_json(
    action: &str,
    candidate: &volicord_inquiry::CandidateRecord,
) -> Result<Value, HostError> {
    let question = candidate
        .content
        .as_ref()
        .and_then(|content| content.question.as_ref())
        .ok_or_else(|| HostError::new("Question Candidate research content is unavailable"))?;
    Ok(json!({
        "action":action,
        "candidate_id":candidate.id.to_string(),
        "candidate_revision":candidate.revision,
        "research_state":question_research_state_name(question.research_state),
        "repository_research":question.repository_basis.iter().cloned().map(repository_research_json).collect::<Vec<_>>(),
        "disposition":candidate_disposition_json(&candidate.disposition),
        "canonical_mutation":false,
        "promoted":false,
    }))
}

fn repository_research_json(basis: volicord_inquiry::RepositoryResearchBasis) -> Value {
    json!({
        "repository_snapshot":basis.repository_snapshot,
        "analysis_snapshot":basis.analysis_snapshot,
        "capability":basis.capability,
        "coverage":basis.coverage,
        "freshness":candidate_freshness_name(basis.freshness),
        "source_ids":basis.source_basis.into_iter().map(|source| source.to_string()).collect::<Vec<_>>(),
        "evidence_assessment":if basis.sufficient { "sufficient" } else { "insufficient" },
        "limits":basis.limits,
    })
}

const fn question_research_state_name(state: QuestionResearchState) -> &'static str {
    match state {
        QuestionResearchState::ReadyToAsk => "ready_to_ask",
        QuestionResearchState::ResearchRequired => "research_required",
    }
}

fn question_research_state(value: &str) -> Result<QuestionResearchState, HostError> {
    match value {
        "ready_to_ask" => Ok(QuestionResearchState::ReadyToAsk),
        "research_required" => Ok(QuestionResearchState::ResearchRequired),
        _ => Err(HostError::new(
            "research_state must be ready_to_ask or research_required",
        )),
    }
}

const fn context_item_role_name(role: ContextItemRole) -> &'static str {
    match role {
        ContextItemRole::Goal => "goal",
        ContextItemRole::Fact => "fact",
        ContextItemRole::Assumption => "assumption",
        ContextItemRole::Constraint => "constraint",
        ContextItemRole::Preference => "preference",
        ContextItemRole::Risk => "risk",
        ContextItemRole::Learning => "learning",
        ContextItemRole::KnownLimit => "known_limit",
    }
}

fn context_item_role(value: &str) -> Result<ContextItemRole, HostError> {
    match value {
        "goal" => Ok(ContextItemRole::Goal),
        "fact" => Ok(ContextItemRole::Fact),
        "assumption" => Ok(ContextItemRole::Assumption),
        "constraint" => Ok(ContextItemRole::Constraint),
        "preference" => Ok(ContextItemRole::Preference),
        "risk" => Ok(ContextItemRole::Risk),
        "learning" => Ok(ContextItemRole::Learning),
        "known_limit" => Ok(ContextItemRole::KnownLimit),
        _ => Err(HostError::new("unknown user Context role")),
    }
}

const fn checkpoint_kind_name(value: CheckpointKind) -> &'static str {
    match value {
        CheckpointKind::Completion => "completion",
        CheckpointKind::Pause => "pause",
        CheckpointKind::Handoff => "handoff",
    }
}

fn checkpoint_kind(value: &str) -> Result<CheckpointKind, HostError> {
    match value {
        "completion" => Ok(CheckpointKind::Completion),
        "pause" => Ok(CheckpointKind::Pause),
        "handoff" => Ok(CheckpointKind::Handoff),
        _ => Err(HostError::new("unknown Checkpoint kind")),
    }
}

const fn work_state_name(value: WorkState) -> &'static str {
    match value {
        WorkState::InProgress => "in_progress",
        WorkState::Paused => "paused",
        WorkState::Completed => "completed",
        WorkState::Abandoned => "abandoned",
        WorkState::Superseded => "superseded",
    }
}

fn work_state(value: &str) -> Result<WorkState, HostError> {
    match value {
        "in_progress" => Ok(WorkState::InProgress),
        "paused" => Ok(WorkState::Paused),
        "completed" => Ok(WorkState::Completed),
        "abandoned" => Ok(WorkState::Abandoned),
        "superseded" => Ok(WorkState::Superseded),
        _ => Err(HostError::new("unknown work state")),
    }
}

const fn verification_state_name(value: VerificationState) -> &'static str {
    match value {
        VerificationState::NotRun => "not_run",
        VerificationState::Partial => "partial",
        VerificationState::Passed => "passed",
        VerificationState::Failed => "failed",
    }
}

const fn user_review_state_name(value: volicord_context::UserReviewState) -> &'static str {
    match value {
        volicord_context::UserReviewState::NotRequested => "not_requested",
        volicord_context::UserReviewState::Pending => "pending",
        volicord_context::UserReviewState::Reviewed => "reviewed",
    }
}

const fn user_acceptance_state_name(value: volicord_context::UserAcceptanceState) -> &'static str {
    match value {
        volicord_context::UserAcceptanceState::NotRequested => "not_requested",
        volicord_context::UserAcceptanceState::Pending => "pending",
        volicord_context::UserAcceptanceState::Accepted => "accepted",
        volicord_context::UserAcceptanceState::Rejected => "rejected",
    }
}

fn command_verification(value: &Value) -> Result<CommandVerificationDraft, HostError> {
    let state = match required_str(value, "state")? {
        "not_run" => VerificationState::NotRun,
        "partial" => VerificationState::Partial,
        "passed" => VerificationState::Passed,
        "failed" => VerificationState::Failed,
        _ => return Err(HostError::new("unknown verification state")),
    };
    let termination = value
        .get("termination")
        .and_then(Value::as_str)
        .map(|value| match value {
            "exited" => Ok(CommandTermination::Exited),
            "signaled" => Ok(CommandTermination::Signaled),
            "spawn_failed" => Ok(CommandTermination::SpawnFailed),
            "indeterminate" => Ok(CommandTermination::Indeterminate),
            _ => Err(HostError::new("unknown command termination")),
        })
        .transpose()?;
    let exit_code = value
        .get("exit_code")
        .and_then(Value::as_u64)
        .map(|code| i32::try_from(code).map_err(|_| HostError::new("exit_code exceeds i32 range")))
        .transpose()?;
    Ok(CommandVerificationDraft {
        state,
        command_label: value
            .get("command_label")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        command_invocation: value
            .get("command_invocation")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        exit_code,
        termination,
        outcome: value
            .get("outcome")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    })
}

const fn candidate_freshness_name(state: CandidateFreshness) -> &'static str {
    match state {
        CandidateFreshness::Current => "current",
        CandidateFreshness::Stale => "stale",
        CandidateFreshness::Unknown => "unknown",
    }
}

fn candidate_freshness(value: &str) -> Result<CandidateFreshness, HostError> {
    match value {
        "current" => Ok(CandidateFreshness::Current),
        "stale" => Ok(CandidateFreshness::Stale),
        "unknown" => Ok(CandidateFreshness::Unknown),
        _ => Err(HostError::new(
            "freshness must be current, stale, or unknown",
        )),
    }
}

fn candidate_disposition_json(disposition: &CandidateDisposition) -> Value {
    match disposition {
        CandidateDisposition::PendingOrRetained => json!({"state":"pending_or_retained"}),
        CandidateDisposition::Promoted {
            canonical_question_id,
            promoted_at,
        } => json!({
            "state":"promoted",
            "question_id":canonical_question_id.to_string(),
            "at_unix_micros":promoted_at.as_unix_micros()
        }),
        CandidateDisposition::Dismissed {
            reason,
            dismissed_at,
        } => json!({
            "state":"dismissed",
            "reason":reason,
            "at_unix_micros":dismissed_at.as_unix_micros()
        }),
        CandidateDisposition::ExpiredOrRetentionCleaned => {
            json!({"state":"expired_or_retention_cleaned"})
        }
    }
}

fn collection_opt_out_json(policy: volicord_inquiry::CollectionOptOut) -> Value {
    json!({
        "project_id":policy.scope.project_id.to_string(),
        "session":policy.scope.session,
        "source_operation":policy.scope.source_operation,
        "candidate_kind":policy.scope.candidate_kind.map(|value| format!("{:?}",value).to_lowercase()),
        "opted_out":policy.opted_out,
        "effective_at_unix_micros":policy.effective_at.as_unix_micros(),
        "basis":policy.basis,
    })
}

fn string_array(value: &Value, key: &str) -> Result<Vec<String>, HostError> {
    value
        .get(key)
        .map(|items| {
            items
                .as_array()
                .ok_or_else(|| HostError::new(format!("{key} must be an array")))?
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(ToOwned::to_owned)
                        .ok_or_else(|| HostError::new(format!("{key} items must be strings")))
                })
                .collect()
        })
        .transpose()
        .map(Option::unwrap_or_default)
}

fn optional_string(value: &Value, key: &str) -> Result<Option<String>, HostError> {
    value
        .get(key)
        .map(|item| {
            item.as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| HostError::new(format!("{key} must be a string")))
        })
        .transpose()
}

fn decision_ids(value: &Value, key: &str) -> Result<Vec<DecisionId>, HostError> {
    string_array(value, key)?
        .into_iter()
        .map(|identity| Ok(DecisionId::from_bytes(parse_identity(&identity)?)))
        .collect()
}

fn source_ids(value: &Value, key: &str) -> Result<Vec<SourceId>, HostError> {
    string_array(value, key)?
        .into_iter()
        .map(|source| parse_source(&source))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn question_candidate_draft(
    args: &Value,
    project_id: ProjectId,
    host_session: &str,
    source_basis: Vec<SourceId>,
    source_operation: String,
    affected_scope: Vec<String>,
    materiality_rationale: String,
    research_state: QuestionResearchState,
    research_state_basis: &str,
) -> Result<CandidateDraft, HostError> {
    let now = SystemClock
        .now()
        .map_err(|error| HostError::new(error.to_string()))?;
    let alternatives = candidate_alternatives(args)?;
    let recommendation_key = required_str(args, "recommendation_key")?.to_owned();
    if !alternatives
        .iter()
        .any(|alternative| alternative.key == recommendation_key)
    {
        return Err(HostError::new(
            "recommendation_key must name one submitted alternative",
        ));
    }
    let established_facts = string_array(args, "established_facts")?
        .into_iter()
        .map(|statement| QuestionEstablishedFact {
            statement,
            source_basis: source_basis.clone(),
            capability: None,
            freshness: QuestionEvidenceFreshness::Current,
        })
        .collect();
    Ok(CandidateDraft {
        project_id,
        kind: CandidateKind::QuestionCandidate,
        collection_mode: CandidateCollectionMode::Automatic,
        origin: CandidateOrigin {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "codex".into(),
            },
            subsystem: "inquiry".into(),
            session: Some(host_session.to_owned()),
            provenance_summary: "explicit Codex Question Candidate submission".into(),
        },
        collection_scope: CandidateCollectionScope {
            project_id,
            session: Some(host_session.to_owned()),
            source_operation: Some(source_operation),
            candidate_kind: CandidateKind::QuestionCandidate,
        },
        observation_basis: CandidateObservationBasis {
            source_basis: source_basis.clone(),
            repository_snapshot: args
                .get("repository_snapshot")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            analysis_snapshot: None,
            execution: None,
            host_turn: None,
            other: Some(format!(
                "explicit agent Candidate operation; research state basis: {research_state_basis}"
            )),
        },
        observed_at: now,
        retention: CandidateRetention {
            retained_until: None,
            basis: required_str(args, "retention_basis")?.to_owned(),
        },
        content: CandidateContent {
            bounded_summary: required_str(args, "bounded_summary")?.to_owned(),
            question: Some(QuestionCandidate {
                prompt_basis: required_str(args, "prompt")?.to_owned(),
                known_facts: established_facts,
                assumptions: string_array(args, "assumptions")?,
                uncertainty: string_array(args, "uncertainty")?,
                affected_scope,
                possible_prerequisites: Vec::new(),
                source_basis: source_basis.clone(),
                repository_basis: Vec::new(),
                freshness: CandidateFreshness::Current,
                duplicate_assessment: DuplicateAssessment::NoDuplicate {
                    basis: required_str(args, "duplicate_basis")?.to_owned(),
                },
                materiality: MaterialityAssessment {
                    status: MaterialityStatus::Material,
                    rationale: Some(materiality_rationale),
                    source_basis: source_basis.clone(),
                    assessed_by: Some(Principal {
                        kind: PrincipalKind::Agent,
                        identity: "codex".into(),
                    }),
                    assessed_at: Some(now),
                },
                presentation_order: Some(required_u64(args, "presentation_order")?),
                why_it_matters_now: required_str(args, "why_now")?.to_owned(),
                alternatives,
                recommendation: AgentRecommendation {
                    alternative_key: Some(recommendation_key),
                    rationale: required_str(args, "recommendation_rationale")?.to_owned(),
                    source_basis,
                },
                trade_offs: string_array(args, "trade_offs")?,
                known_limits: string_array(args, "known_limits")?,
                what_the_answer_unlocks: string_array(args, "what_unlocks")?,
                allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
                research_state,
            }),
            engineering_choice_discovery: None,
            materiality_review: None,
            learning_deliberation: None,
        },
    })
}

fn candidate_alternatives(value: &Value) -> Result<Vec<QuestionAlternative>, HostError> {
    value
        .get("alternatives")
        .and_then(Value::as_array)
        .ok_or_else(|| HostError::new("alternatives are required"))?
        .iter()
        .map(|alternative| {
            Ok(QuestionAlternative {
                key: required_str(alternative, "key")?.to_owned(),
                label: required_str(alternative, "label")?.to_owned(),
                consequence: required_str(alternative, "consequence")?.to_owned(),
            })
        })
        .collect()
}

fn parse_context_item(value: &str) -> Result<ContextItemId, HostError> {
    Ok(ContextItemId::from_bytes(parse_identity(value)?))
}

fn parse_analysis_snapshot(value: &str) -> Result<AnalysisSnapshotId, HostError> {
    AnalysisSnapshotId::from_hex(value).map_err(HostError::new)
}

fn materiality_judgments(
    value: &Value,
    discovery: &volicord_inquiry::EngineeringChoiceDiscovery,
    canonical: &volicord_context::CanonicalReadBasis,
) -> Result<Vec<MaterialityDimension>, HostError> {
    let judgments = value
        .get("judgments")
        .and_then(Value::as_array)
        .ok_or_else(|| HostError::new("judgments are required"))?;
    let allowed_choice_ids = discovery
        .choices
        .iter()
        .map(|choice| choice.choice_id.clone())
        .collect::<Vec<_>>();
    let mut indexed = BTreeMap::new();
    for (index, judgment) in judgments.iter().enumerate() {
        let choice_id = required_str(judgment, "choice_id")?;
        if !allowed_choice_ids
            .iter()
            .any(|allowed| allowed == choice_id)
        {
            return Err(materiality_contract_error(
                format!("arguments.judgments[{index}].choice_id"),
                Some(choice_id),
                "judgment references a choice outside the bound Engineering Choice Discovery",
                &allowed_choice_ids,
                discovery,
            ));
        }
        if indexed.insert(choice_id.to_owned(), judgment).is_some() {
            return Err(materiality_contract_error(
                format!("arguments.judgments[{index}].choice_id"),
                Some(choice_id),
                "each discovered choice requires exactly one judgment",
                &allowed_choice_ids,
                discovery,
            ));
        }
    }
    let missing = allowed_choice_ids
        .iter()
        .filter(|choice_id| !indexed.contains_key(*choice_id))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(HostError::with_details(
            "Materiality Review omitted discovery-owned choices",
            json!({
                "diagnostic":"materiality_contract_validation",
                "field_path":"arguments.judgments",
                "missing_choice_ids":missing,
                "allowed_choice_ids":allowed_choice_ids,
                "bound_identities":{
                    "goal_context_id":discovery.goal_context_id.to_string(),
                    "baseline_analysis_snapshot_id":discovery.baseline_analysis_snapshot_id.to_string(),
                },
                "next_supported_action":{"tool":"materiality_review","action":"draft"},
            }),
        ));
    }

    discovery
        .choices
        .iter()
        .map(|choice| {
            let judgment = indexed
                .get(&choice.choice_id)
                .copied()
                .ok_or_else(|| HostError::new("materiality judgment indexing failed"))?;
            materiality_dimension_from_judgment(judgment, choice, discovery, canonical)
        })
        .collect()
}

fn materiality_dimension_from_judgment(
    value: &Value,
    choice: &EngineeringChoice,
    discovery: &volicord_inquiry::EngineeringChoiceDiscovery,
    canonical: &volicord_context::CanonicalReadBasis,
) -> Result<MaterialityDimension, HostError> {
    let mut source_basis = choice.source_basis.clone();
    source_basis.extend(source_ids(value, "additional_source_ids")?);
    let mut contract_basis = string_array(value, "contract_basis")?;
    let mut decision_basis = decision_ids(value, "decision_ids")?;
    let mut research_basis = string_array(value, "research_basis")?;
    let mut explicit_delegation = None;
    let disposition = match required_str(value, "disposition")? {
        "repository_or_environment_fact" => MaterialityDisposition::RepositoryOrEnvironmentFact,
        "settled_authority" => MaterialityDisposition::SettledAuthority,
        "agent_owned_implementation_choice" => {
            MaterialityDisposition::AgentOwnedImplementationChoice
        }
        "delegated_implementation_choice" => {
            if let Some(statement) = optional_string(value, "delegation_statement")? {
                let (goal, user_turn_source_id) = exact_current_goal_user_source(
                    canonical,
                    discovery.goal_context_id,
                    discovery,
                )?;
                source_basis.push(user_turn_source_id);
                explicit_delegation = Some(volicord_inquiry::ExplicitDelegationEvidence {
                    goal_context_id: goal.id,
                    user_turn_source_id,
                    verbatim_statement: statement,
                    dimension_id: choice.choice_id.clone(),
                    discovered_choice_ids: vec![choice.choice_id.clone()],
                    affected_scope: string_array(value, "delegated_scope")?,
                    material_consequences: choice.technical_consequences.clone(),
                    effect_categories: choice.effect_categories.clone(),
                });
            }
            MaterialityDisposition::DelegatedImplementationChoice
        }
        "exploratory_uncertainty" => MaterialityDisposition::ExploratoryUncertainty {
            disposition: match required_str(value, "exploratory_disposition")? {
                "research_required" => ExploratoryDisposition::ResearchRequired,
                "prototype_required" => ExploratoryDisposition::PrototypeRequired,
                "deferred_with_revisit" => ExploratoryDisposition::DeferredWithRevisit,
                "resolved_by_research" => ExploratoryDisposition::ResolvedByResearch,
                _ => return Err(HostError::new("unknown exploratory disposition")),
            },
        },
        "unresolved_user_owned_outcome" => {
            let resolution_decision_id = value
                .get("resolution_decision_id")
                .and_then(Value::as_str)
                .map(|identity| parse_identity(identity).map(DecisionId::from_bytes))
                .transpose()?;
            if let Some(decision_id) = resolution_decision_id {
                decision_basis.push(decision_id);
            }
            MaterialityDisposition::UnresolvedUserOwnedOutcome {
                resolution_decision_id,
            }
        }
        _ => return Err(HostError::new("unknown Materiality Review disposition")),
    };
    let kinds = match &disposition {
        MaterialityDisposition::RepositoryOrEnvironmentFact => {
            vec![WorkAuthorityBasisKind::RepositoryOrEnvironmentFact]
        }
        MaterialityDisposition::SettledAuthority => {
            let mut kinds = Vec::new();
            if !contract_basis.is_empty() {
                kinds.push(WorkAuthorityBasisKind::AcceptedContract);
            }
            if !decision_basis.is_empty() {
                kinds.push(WorkAuthorityBasisKind::ApplicableDecision);
            }
            kinds
        }
        MaterialityDisposition::AgentOwnedImplementationChoice => {
            vec![WorkAuthorityBasisKind::ImplementationPreference]
        }
        MaterialityDisposition::DelegatedImplementationChoice => {
            vec![WorkAuthorityBasisKind::ExplicitDelegation]
        }
        MaterialityDisposition::ExploratoryUncertainty { disposition } => vec![match disposition {
            ExploratoryDisposition::ResearchRequired
            | ExploratoryDisposition::ResolvedByResearch => {
                WorkAuthorityBasisKind::ResearchEvidence
            }
            ExploratoryDisposition::PrototypeRequired => WorkAuthorityBasisKind::PrototypeEvidence,
            ExploratoryDisposition::DeferredWithRevisit => {
                WorkAuthorityBasisKind::DeferOrRevisitBasis
            }
        }],
        MaterialityDisposition::UnresolvedUserOwnedOutcome {
            resolution_decision_id,
        } => {
            if resolution_decision_id.is_some() {
                vec![WorkAuthorityBasisKind::ApplicableDecision]
            } else {
                vec![WorkAuthorityBasisKind::NoSettlingAuthority]
            }
        }
    };
    if matches!(
        disposition,
        MaterialityDisposition::UnresolvedUserOwnedOutcome {
            resolution_decision_id: None
        }
    ) {
        contract_basis.clear();
        research_basis.clear();
    }
    source_basis.sort_unstable();
    source_basis.dedup();
    Ok(MaterialityDimension {
        dimension_id: choice.choice_id.clone(),
        discovered_choice_ids: vec![choice.choice_id.clone()],
        summary: choice.summary.clone(),
        affected_scope: choice.affected_scope.clone(),
        material_consequences: choice.technical_consequences.clone(),
        observable_signals: material_outcome_signals(&choice.effect_categories),
        disposition,
        basis: WorkAuthorityBasis {
            kinds,
            summary: required_str(value, "basis_summary")?.to_owned(),
            source_basis,
            contract_basis,
            decision_basis,
            research_basis,
            explicit_delegation,
        },
        learning_value: learning_value_assessment(value)?,
    })
}

fn exact_current_goal_user_source<'a>(
    canonical: &'a volicord_context::CanonicalReadBasis,
    goal_context_id: ContextItemId,
    discovery: &volicord_inquiry::EngineeringChoiceDiscovery,
) -> Result<(&'a volicord_context::ContextItem, SourceId), HostError> {
    let goal = canonical
        .context_items
        .iter()
        .find(|goal| goal.id == goal_context_id)
        .ok_or_else(|| HostError::new("bound current Goal Context is unavailable"))?;
    let sources = goal
        .source_basis
        .iter()
        .filter_map(|source_id| {
            canonical.sources.iter().find(|basis| {
                basis.source.id == *source_id
                    && basis.freshness == volicord_context::SourceFreshness::Current
                    && basis.source.actor.kind == PrincipalKind::User
                    && matches!(
                        basis.source.payload,
                        volicord_context::SourcePayload::CurrentHostUserTurn { .. }
                    )
            })
        })
        .collect::<Vec<_>>();
    if sources.len() != 1 {
        return Err(HostError::with_details(
            "current-task delegation needs one exact current-host Goal Source",
            json!({
                "diagnostic":"materiality_contract_validation",
                "field_path":"arguments.judgments[].delegation_statement",
                "missing_prerequisite":"one exact current-host user-turn Source for the bound Goal",
                "bound_identities":{
                    "goal_context_id":goal_context_id.to_string(),
                    "baseline_analysis_snapshot_id":discovery.baseline_analysis_snapshot_id.to_string(),
                },
                "next_supported_action":{"tool":"materiality_review","action":"draft"},
            }),
        ));
    }
    Ok((goal, sources[0].source.id))
}

fn material_outcome_signals(
    categories: &[EngineeringEffectCategory],
) -> Vec<MaterialOutcomeSignal> {
    let mut signals = categories
        .iter()
        .map(|category| match category {
            EngineeringEffectCategory::PublicApiShapeOrSemantics => {
                MaterialOutcomeSignal::PublicApiSemantics
            }
            EngineeringEffectCategory::FailureOrErrorSemantics => {
                MaterialOutcomeSignal::ObservableFailurePolicy
            }
            EngineeringEffectCategory::PrivacyOrDisclosure => {
                MaterialOutcomeSignal::PrivacyOrExternalDisclosure
            }
            EngineeringEffectCategory::Security => MaterialOutcomeSignal::SecurityPosture,
            EngineeringEffectCategory::UserVisibleBehaviorOrDefault => {
                MaterialOutcomeSignal::UserVisibleDefault
            }
            EngineeringEffectCategory::MaintenanceOrSupport => {
                MaterialOutcomeSignal::MaintenanceOrSupportPolicy
            }
            EngineeringEffectCategory::Compatibility
            | EngineeringEffectCategory::PersistenceOrLifetime
            | EngineeringEffectCategory::PerformanceOrResourceBehavior
            | EngineeringEffectCategory::ConcurrencyOrOperability
            | EngineeringEffectCategory::ImplementationInternal => {
                MaterialOutcomeSignal::OtherMaterialOutcome
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if signals.is_empty() {
        signals.push(MaterialOutcomeSignal::OtherMaterialOutcome);
    }
    signals
}

const fn material_outcome_signal_name(signal: MaterialOutcomeSignal) -> &'static str {
    match signal {
        MaterialOutcomeSignal::PublicApiSemantics => "public_api_semantics",
        MaterialOutcomeSignal::CliCompatibilityOrExitBehavior => {
            "cli_compatibility_or_exit_behavior"
        }
        MaterialOutcomeSignal::ObservableFailurePolicy => "observable_failure_policy",
        MaterialOutcomeSignal::PrivacyOrExternalDisclosure => "privacy_or_external_disclosure",
        MaterialOutcomeSignal::SecurityPosture => "security_posture",
        MaterialOutcomeSignal::UserVisibleDefault => "user_visible_default",
        MaterialOutcomeSignal::MaintenanceOrSupportPolicy => "maintenance_or_support_policy",
        MaterialOutcomeSignal::OtherMaterialOutcome => "other_material_outcome",
    }
}

fn materiality_contract_error(
    field_path: String,
    invalid_value: Option<&str>,
    problem: &str,
    allowed_choice_ids: &[String],
    discovery: &volicord_inquiry::EngineeringChoiceDiscovery,
) -> HostError {
    HostError::with_details(
        problem,
        json!({
            "diagnostic":"materiality_contract_validation",
            "field_path":field_path,
            "invalid_value":invalid_value,
            "allowed_values":allowed_choice_ids,
            "bound_identities":{
                "goal_context_id":discovery.goal_context_id.to_string(),
                "baseline_analysis_snapshot_id":discovery.baseline_analysis_snapshot_id.to_string(),
            },
            "next_supported_action":{"tool":"materiality_review","action":"draft"},
        }),
    )
}

fn engineering_choices(value: &Value) -> Result<Vec<EngineeringChoice>, HostError> {
    value
        .get("choices")
        .and_then(Value::as_array)
        .ok_or_else(|| HostError::new("choices must be an array"))?
        .iter()
        .map(|choice| {
            let relationship = choice
                .get("relationship")
                .ok_or_else(|| HostError::new("choice relationship is required"))?;
            let relationship = match required_str(relationship, "state")? {
                "independent" => EngineeringChoiceRelationship::Independent,
                "coupled" => EngineeringChoiceRelationship::Coupled {
                    choice_ids: string_array(relationship, "choice_ids")?,
                    rationale: required_str(relationship, "rationale")?.to_owned(),
                },
                _ => return Err(HostError::new("unknown engineering choice relationship")),
            };
            let alternatives = choice
                .get("alternatives")
                .and_then(Value::as_array)
                .ok_or_else(|| HostError::new("choice alternatives must be an array"))?
                .iter()
                .map(|alternative| {
                    Ok(EngineeringAlternative {
                        alternative_id: required_str(alternative, "alternative_id")?.to_owned(),
                        summary: required_str(alternative, "summary")?.to_owned(),
                        technical_consequences: string_array(
                            alternative,
                            "technical_consequences",
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, HostError>>()?;
            Ok(EngineeringChoice {
                choice_id: required_str(choice, "choice_id")?.to_owned(),
                summary: required_str(choice, "summary")?.to_owned(),
                affected_scope: string_array(choice, "affected_scope")?,
                alternatives,
                technical_consequences: string_array(choice, "technical_consequences")?,
                source_basis: source_ids(choice, "source_ids")?,
                effect_categories: required_strings(choice, "effect_categories")?
                    .into_iter()
                    .map(|category| engineering_effect_category(&category))
                    .collect::<Result<Vec<_>, _>>()?,
                relationship,
                evidence_state: engineering_evidence_state(required_str(
                    choice,
                    "evidence_state",
                )?)?,
            })
        })
        .collect()
}

fn engineering_effect_category(value: &str) -> Result<EngineeringEffectCategory, HostError> {
    match value {
        "public_api_shape_or_semantics" => Ok(EngineeringEffectCategory::PublicApiShapeOrSemantics),
        "compatibility" => Ok(EngineeringEffectCategory::Compatibility),
        "failure_or_error_semantics" => Ok(EngineeringEffectCategory::FailureOrErrorSemantics),
        "persistence_or_lifetime" => Ok(EngineeringEffectCategory::PersistenceOrLifetime),
        "privacy_or_disclosure" => Ok(EngineeringEffectCategory::PrivacyOrDisclosure),
        "security" => Ok(EngineeringEffectCategory::Security),
        "user_visible_behavior_or_default" => {
            Ok(EngineeringEffectCategory::UserVisibleBehaviorOrDefault)
        }
        "performance_or_resource_behavior" => {
            Ok(EngineeringEffectCategory::PerformanceOrResourceBehavior)
        }
        "concurrency_or_operability" => Ok(EngineeringEffectCategory::ConcurrencyOrOperability),
        "maintenance_or_support" => Ok(EngineeringEffectCategory::MaintenanceOrSupport),
        "implementation_internal" => Ok(EngineeringEffectCategory::ImplementationInternal),
        _ => Err(HostError::new("unknown engineering effect category")),
    }
}

fn engineering_evidence_state(value: &str) -> Result<EngineeringChoiceEvidenceState, HostError> {
    match value {
        "sufficient" => Ok(EngineeringChoiceEvidenceState::Sufficient),
        "research_required" => Ok(EngineeringChoiceEvidenceState::ResearchRequired),
        "prototype_required" => Ok(EngineeringChoiceEvidenceState::PrototypeRequired),
        _ => Err(HostError::new("unknown engineering choice evidence state")),
    }
}

fn learning_participation(value: &Value) -> Result<LearningParticipation, HostError> {
    let participation = value
        .get("learning_participation")
        .ok_or_else(|| HostError::new("learning_participation is required"))?;
    match required_str(participation, "state")? {
        "inactive" => Ok(LearningParticipation::Inactive),
        "active" => Ok(LearningParticipation::Active {
            user_turn_source_id: parse_source(required_str(participation, "user_turn_source_id")?)?,
            verbatim_statement: required_str(participation, "verbatim_statement")?.to_owned(),
        }),
        _ => Err(HostError::new("unknown learning participation state")),
    }
}

fn learning_selections(
    value: &Value,
    key: &str,
) -> Result<Vec<LearningAlternativeSelection>, HostError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| HostError::new(format!("{key} must be an array")))?
        .iter()
        .map(|selection| {
            Ok(LearningAlternativeSelection {
                choice_id: required_str(selection, "choice_id")?.to_owned(),
                alternative_id: required_str(selection, "alternative_id")?.to_owned(),
            })
        })
        .collect()
}

fn learning_value_assessment(value: &Value) -> Result<LearningValueAssessment, HostError> {
    let assessment = value
        .get("learning_value")
        .ok_or_else(|| HostError::new("dimension learning_value is required"))?;
    match required_str(assessment, "state")? {
        "routine" => Ok(LearningValueAssessment::Routine {
            rationale: required_str(assessment, "rationale")?.to_owned(),
        }),
        "deliberation_worthy" => Ok(LearningValueAssessment::DeliberationWorthy {
            rationale: required_str(assessment, "rationale")?.to_owned(),
            consequence_significance: string_array(assessment, "consequence_significance")?,
            transferable_principles: string_array(assessment, "transferable_principles")?,
            non_obvious_trade_offs: string_array(assessment, "non_obvious_trade_offs")?,
        }),
        _ => Err(HostError::new("unknown learning-value assessment")),
    }
}

fn schema_required_fields(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn schema_property_names(schema: &Value) -> Vec<String> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|properties| properties.keys().cloned())
        .collect()
}

fn singleton_enum_fields(schema: &Value) -> serde_json::Map<String, Value> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|properties| properties.iter())
        .filter_map(|(name, property)| {
            let values = property.get("enum")?.as_array()?;
            (values.len() == 1).then(|| (name.clone(), values[0].clone()))
        })
        .collect()
}

fn materiality_judgment_contract_json(
    contract: &MaterialityJudgmentContract,
    all_fields: &BTreeSet<String>,
    derived_identities: &Value,
) -> Value {
    let required_fields = schema_required_fields(&contract.schema);
    let allowed_fields = schema_property_names(&contract.schema);
    let fixed_fields = singleton_enum_fields(&contract.schema);
    let caller_must_semantically_provide = required_fields
        .iter()
        .filter(|field| field.as_str() != "choice_id" && !fixed_fields.contains_key(*field))
        .cloned()
        .collect::<Vec<_>>();
    let caller_may_provide = allowed_fields
        .iter()
        .filter(|field| !required_fields.contains(field))
        .cloned()
        .collect::<Vec<_>>();
    let forbidden_fields = all_fields
        .iter()
        .filter(|field| !allowed_fields.contains(field))
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "variant_id":contract.variant_id,
        "required_fields":required_fields,
        "forbidden_fields":forbidden_fields,
        "allowed_fields":allowed_fields,
        "bounded_allowed_values":fixed_fields,
        "server_derived_identities":derived_identities,
        "caller_must_semantically_provide":caller_must_semantically_provide,
        "caller_may_provide":caller_may_provide,
        "input_schema":contract.schema,
    })
}

fn schema_alternatives(schema: Value) -> Vec<Value> {
    let alternatives = schema
        .get("oneOf")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let all_fields = alternatives
        .iter()
        .flat_map(schema_property_names)
        .collect::<BTreeSet<_>>();
    alternatives
        .into_iter()
        .map(|input_schema| {
            let required_fields = schema_required_fields(&input_schema);
            let allowed_fields = schema_property_names(&input_schema);
            let forbidden_fields = all_fields
                .iter()
                .filter(|field| !allowed_fields.contains(field))
                .cloned()
                .collect::<Vec<_>>();
            let bounded_allowed_values = singleton_enum_fields(&input_schema);
            let caller_must_semantically_provide = required_fields
                .iter()
                .filter(|field| !bounded_allowed_values.contains_key(*field))
                .cloned()
                .collect::<Vec<_>>();
            json!({
                "required_fields":required_fields,
                "forbidden_fields":forbidden_fields,
                "allowed_fields":allowed_fields,
                "bounded_allowed_values":bounded_allowed_values,
                "caller_must_semantically_provide":caller_must_semantically_provide,
                "input_schema":input_schema,
            })
        })
        .collect()
}

fn materiality_draft_json(
    project_id: ProjectId,
    candidate_id: CandidateId,
    discovery: &volicord_inquiry::EngineeringChoiceDiscovery,
    canonical: &volicord_context::CanonicalReadBasis,
    current_review: Option<CandidateId>,
) -> Value {
    let goal = canonical
        .context_items
        .iter()
        .find(|goal| goal.id == discovery.goal_context_id);
    let current_host_user_turn_source_ids = goal
        .into_iter()
        .flat_map(|goal| goal.source_basis.iter())
        .filter_map(|source_id| {
            canonical
                .sources
                .iter()
                .find(|basis| {
                    basis.source.id == *source_id
                        && basis.freshness == volicord_context::SourceFreshness::Current
                        && basis.source.actor.kind == PrincipalKind::User
                        && matches!(
                            basis.source.payload,
                            volicord_context::SourcePayload::CurrentHostUserTurn { .. }
                        )
                })
                .map(|basis| basis.source.id.to_string())
        })
        .collect::<Vec<_>>();
    let contracts = materiality_judgment_contracts();
    let all_judgment_fields = contracts
        .iter()
        .flat_map(|contract| schema_property_names(&contract.schema))
        .collect::<BTreeSet<_>>();
    let derived_identities = json!({
        "project_id":project_id.to_string(),
        "goal_context_id":discovery.goal_context_id.to_string(),
        "baseline_analysis_snapshot_id":discovery.baseline_analysis_snapshot_id.to_string(),
        "engineering_choice_discovery_candidate_id":candidate_id.to_string(),
        "current_goal_user_turn_source_ids":current_host_user_turn_source_ids,
    });
    let judgment_contracts = contracts
        .iter()
        .map(|contract| {
            materiality_judgment_contract_json(contract, &all_judgment_fields, &derived_identities)
        })
        .collect::<Vec<_>>();
    let legal_judgment_variant_ids = contracts
        .iter()
        .map(|contract| contract.variant_id)
        .collect::<Vec<_>>();
    let judgment_templates = discovery
        .choices
        .iter()
        .map(|choice| {
            let observable_signals = material_outcome_signals(&choice.effect_categories)
                .into_iter()
                .map(material_outcome_signal_name)
                .collect::<Vec<_>>();
            json!({
                "discovery_owned":{
                    "choice_id":choice.choice_id,
                    "summary":choice.summary,
                    "affected_scope":choice.affected_scope,
                    "alternatives":choice.alternatives.iter().map(|alternative| json!({
                        "alternative_id":alternative.alternative_id,
                        "summary":alternative.summary,
                        "technical_consequences":alternative.technical_consequences,
                    })).collect::<Vec<_>>(),
                    "material_consequences":choice.technical_consequences,
                    "available_source_ids":choice.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    "effect_categories":choice.effect_categories.iter().copied().map(engineering_effect_category_name).collect::<Vec<_>>(),
                    "observable_signals":observable_signals,
                    "relationship":engineering_choice_json(choice)["relationship"].clone(),
                    "evidence_state":engineering_evidence_state_name(choice.evidence_state),
                },
                "caller_owned_judgment":{
                    "prefilled_fields":{"choice_id":choice.choice_id},
                    "legal_judgment_variant_ids":legal_judgment_variant_ids,
                    "assembly":"Choose one referenced judgment_contract, merge prefilled_fields and its bounded_allowed_values, then provide exactly its caller semantic fields. Do not submit any forbidden field.",
                }
            })
        })
        .collect::<Vec<_>>();
    let (request_action, request_identity_field, request_identity, request_schema) =
        match current_review {
            Some(review_candidate_id) => (
                "revise",
                "review_candidate_id",
                review_candidate_id.to_string(),
                materiality_revise_schema(),
            ),
            None => (
                "record",
                "engineering_choice_discovery_candidate_id",
                candidate_id.to_string(),
                materiality_record_schema(),
            ),
        };
    let mut request_prefilled_fields = serde_json::Map::new();
    request_prefilled_fields.insert("action".into(), json!(request_action));
    request_prefilled_fields.insert("project_id".into(), json!(project_id.to_string()));
    request_prefilled_fields.insert(request_identity_field.into(), json!(request_identity));
    json!({
        "action":"draft",
        "project_id":project_id.to_string(),
        "goal_context_id":discovery.goal_context_id.to_string(),
        "baseline_analysis_snapshot_id":discovery.baseline_analysis_snapshot_id.to_string(),
        "engineering_choice_discovery_candidate_id":candidate_id.to_string(),
        "current_goal":{
            "goal_context_id":discovery.goal_context_id.to_string(),
            "statement":goal.map(|goal| goal.statement.clone()),
            "current_host_user_turn_source_ids":current_host_user_turn_source_ids,
            "ownership_notice":"If this current Goal reserves an outcome for user control or asks the user to retain the choice, do not downgrade that exact dimension to implementation preference because an older contract or repository convention exists.",
        },
        "authority_decision_checklist":{
            "counterfactual_question":"Would credible alternatives change an externally observable contract, durable effect, compatibility/support commitment, privacy/security posture, user-visible default, observable failure policy, or another material product outcome?",
            "material_outcome_categories":[
                "externally_observable_contract",
                "durable_effect",
                "compatibility_or_support_commitment",
                "privacy_or_security_posture",
                "user_visible_default",
                "observable_failure_policy",
                "other_material_product_outcome"
            ],
            "exact_authority_required":[
                "current repository or environment fact settling this exact dimension",
                "accepted contract settling this exact dimension",
                "applicable Decision settling this exact dimension",
                "explicit delegation covering this exact dimension"
            ],
            "not_authority":[
                "overall feature request",
                "implementation preference",
                "agent recommendation",
                "library or repository convention"
            ],
            "outcomes":{
                "unresolved_user_owned_outcome":"Use when credible alternatives have materially different consequences and no exact authority settles the dimension.",
                "exploratory_uncertainty":"Use when evidence is still required to establish whether the alternatives or material consequences are real.",
                "agent_owned_implementation_choice":"Use only for bounded implementation discretion remaining after material user-facing policy is settled or credible alternatives do not vary that policy."
            },
            "hidden_boundary_instruction":"Examine every exact material dimension discovered during repository work; the overall Goal is not blanket authority for subordinate public, persistence, compatibility, privacy, security, default, failure, operational, or support semantics.",
            "authority_revision_chronology":"If a prior agent-owned or delegated assessment is corrected to user-owned after affected work, a later Decision is prospective and does not certify that earlier work. Production records this as late authority correction only when maintained baseline/current path evidence proves the chronology; otherwise rollout validation remains responsible for the ordering judgment.",
        },
        "authority_learning_routing":authority_learning_routing_json(),
        "learning_participation":{
            "input_alternatives":schema_alternatives(learning_participation_schema()),
            "derived_identity_options":{"current_goal_user_turn_source_ids":current_host_user_turn_source_ids},
            "assembly":"Choose inactive, or choose active and provide a verbatim explicit opt-in from one returned current Goal user-turn Source. Learning participation is independent of authority.",
        },
        "learning_value_input_alternatives":schema_alternatives(learning_value_schema()),
        "field_ownership":{
            "discovery_owned_derived_server_side":["goal_context_id","baseline_analysis_snapshot_id","dimension_id","discovered_choice_ids","summary","affected_scope","material_consequences","observable_signals","discovery_source_ids"],
            "caller_owned_semantic_judgments":["rationale","learning_participation","choice_id","disposition","basis_summary","additional authority evidence allowed for that disposition","learning_value"],
        },
        "work_authority_basis_kind_contract":{
            "repository_or_environment_fact":"derived only for repository_or_environment_fact",
            "accepted_contract":"derived only from non-empty contract_basis for settled_authority",
            "applicable_decision":"derived only from Decision identities for settled authority or a resolved user-owned outcome",
            "explicit_delegation":"derived only for current-task verbatim delegation or Inquiry-time delegation Decision",
            "research_evidence":"derived only for research-required/resolved exploratory treatment",
            "prototype_evidence":"derived only for prototype-required exploratory treatment",
            "defer_or_revisit_basis":"derived only for deferred_with_revisit exploratory treatment",
            "implementation_preference":"derived only for bounded agent_owned_implementation_choice",
            "no_settling_authority":"derived only for unresolved_user_owned_outcome without a Decision",
            "agent_recommendation":"never authority and not accepted as a record input",
            "library_or_convention":"never authority and not accepted as a record input",
            "invalid_combinations":[
                "contract or Decision evidence with agent-owned implementation preference",
                "accepted contract, recommendation, convention, or implementation preference as delegation",
                "current-task verbatim delegation combined with Inquiry-time Decision delegation",
                "resolution Decision on an unresolved user-owned judgment",
                "disposition-specific fields on any other disposition"
            ]
        },
        "judgment_contract_source":"The same closed schema variants validate materiality_review record and revise calls.",
        "judgment_contracts":judgment_contracts,
        "judgment_templates":judgment_templates,
        "record_request":{
            "action":request_action,
            "prefilled_fields":request_prefilled_fields,
            "caller_must_supply":["rationale","learning_participation","judgments"],
            "judgments_assembly":{
                "choice_order":discovery.choices.iter().map(|choice| choice.choice_id.clone()).collect::<Vec<_>>(),
                "exactly_one_judgment_per_choice":true,
                "steps":[
                    "For each judgment_template, choose one legal_judgment_variant_id without changing the semantic choice.",
                    "Merge caller_owned_judgment.prefilled_fields with the selected judgment_contract bounded_allowed_values.",
                    "Provide every caller_must_semantically_provide field, including one learning_value_input_alternative, and only desired caller_may_provide fields.",
                    "Place the assembled judgments in choice_order and merge them with record_request.prefilled_fields plus rationale and learning_participation."
                ]
            },
            "input_schema":request_schema,
        },
        "required_action":{"tool":"materiality_review","action":request_action},
        "canonical_mutation":false,
        "read_only":true,
    })
}

fn authority_learning_routing_json() -> Value {
    json!({
        "assessment_owner":"active_agent",
        "independence_rule":"Learning participation and learning value are independent of authority ownership.",
        "learning_requests_not_user_ownership":[
            "ask_to_learn",
            "ask_to_compare_alternatives",
            "ask_to_reason_before_implementation",
            "ask_to_select_an_implementation_approach_for_learning"
        ],
        "ownership_test":{
            "kind":"material_consequence_and_exact_authority_counterfactual",
            "question":"Would credible alternatives change a material product outcome, and who retains authority over that exact dimension?",
            "semantic_assessment_required":true
        },
        "routes":[
            {
                "when":{"authority":"user_owned_material_outcome","learning_value":"any","learning_participation":"any"},
                "required_path":"question_then_current_host_decision",
                "canonical_decision":true
            },
            {
                "when":{"authority":["agent_owned","explicitly_delegated"],"learning_value":"deliberation_worthy","learning_participation":"active"},
                "required_path":"learning_deliberation",
                "canonical_decision":false
            },
            {
                "when":{"authority":["agent_owned","explicitly_delegated"],"learning_value":"routine","learning_participation":"any"},
                "required_path":"no_learning_interruption",
                "canonical_decision":false
            },
            {
                "when":{"authority":"exploratory_uncertainty","learning_participation":"any"},
                "required_path":"declared_research_prototype_defer_or_resolution",
                "canonical_decision":false
            }
        ],
        "learning_selection_authority":{
            "kind":"bounded_learning_and_implementation_basis",
            "canonical_decision":false,
            "does_not_resolve_user_owned_authority":true
        },
        "production_boundary":{
            "deterministically_verified":["typed_provenance","identity","scope","freshness","lifecycle","allowed_state_transition"],
            "semantic_assessment":"provided_by_active_agent",
            "forbidden_automation":["keyword_matching","regular_expression_ownership_detection","prompt_classifier","provider_semantic_classifier"]
        }
    })
}

fn learning_deliberation_json(
    action: &str,
    candidate_id: CandidateId,
    revision: u64,
    deliberation: &LearningDeliberation,
) -> Value {
    let rounds = deliberation
        .rounds
        .iter()
        .map(|round| {
            let mut value = json!({
                "initial_response_source_id":round.initial_response_source_id.to_string(),
                "response":learning_initial_response_json(&round.response),
                "user_rationale":round.user_rationale,
            });
            if let Some(object) = value.as_object_mut() {
                if let Some(feedback) = &round.agent_feedback {
                    object.insert("agent_feedback".into(), json!(feedback));
                }
                if let Some(recommendation) = &round.agent_recommendation {
                    object.insert(
                        "agent_recommendation".into(),
                        json!({
                            "selections":recommendation.selections.iter().map(learning_selection_json).collect::<Vec<_>>(),
                            "rationale":recommendation.rationale,
                        }),
                    );
                }
                if let Some(source_id) = round.reconsideration_source_id {
                    object.insert("reconsideration_source_id".into(), json!(source_id.to_string()));
                }
                if let Some(rationale) = &round.reconsideration_rationale {
                    object.insert("reconsideration_rationale".into(), json!(rationale));
                }
            }
            value
        })
        .collect::<Vec<_>>();
    json!({
        "action":action,
        "interaction_kind":"learning_participation",
        "canonical_decision":false,
        "authority_notice":"This Learning Deliberation is not a canonical Question or Decision. Route user-owned outcomes through inquiry_frontier and decision_record.",
        "deliberation_candidate_id":candidate_id.to_string(),
        "revision":revision,
        "goal_context_id":deliberation.goal_context_id.to_string(),
        "baseline_analysis_snapshot_id":deliberation.baseline_analysis_snapshot_id.to_string(),
        "engineering_choice_discovery_candidate_id":deliberation.engineering_choice_discovery_candidate_id.to_string(),
        "materiality_review_candidate_id":deliberation.materiality_review_candidate_id.to_string(),
        "dimension_id":deliberation.dimension_id,
        "discovered_choice_ids":deliberation.discovered_choice_ids,
        "affected_scope":deliberation.affected_scope,
        "problem":deliberation.problem,
        "established_facts":deliberation.established_facts,
        "choices":deliberation.choices.iter().map(engineering_choice_json).collect::<Vec<_>>(),
        "rounds":rounds,
        "state":learning_deliberation_state_json(&deliberation.state),
    })
}

fn engineering_choice_json(choice: &EngineeringChoice) -> Value {
    let relationship = match &choice.relationship {
        EngineeringChoiceRelationship::Independent => json!({"state":"independent"}),
        EngineeringChoiceRelationship::Coupled {
            choice_ids,
            rationale,
        } => {
            json!({"state":"coupled","choice_ids":choice_ids,"rationale":rationale})
        }
    };
    json!({
        "choice_id":choice.choice_id,
        "summary":choice.summary,
        "affected_scope":choice.affected_scope,
        "alternatives":choice.alternatives.iter().map(|alternative| json!({
            "alternative_id":alternative.alternative_id,
            "summary":alternative.summary,
            "technical_consequences":alternative.technical_consequences,
        })).collect::<Vec<_>>(),
        "technical_consequences":choice.technical_consequences,
        "source_ids":choice.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "effect_categories":choice.effect_categories.iter().copied().map(engineering_effect_category_name).collect::<Vec<_>>(),
        "relationship":relationship,
        "evidence_state":engineering_evidence_state_name(choice.evidence_state),
    })
}

fn learning_selection_json(selection: &LearningAlternativeSelection) -> Value {
    json!({"choice_id":selection.choice_id,"alternative_id":selection.alternative_id})
}

fn learning_initial_response_json(response: &LearningInitialResponse) -> Value {
    match response {
        LearningInitialResponse::Select { selections } => json!({
            "state":"selected",
            "selections":selections.iter().map(learning_selection_json).collect::<Vec<_>>(),
        }),
        LearningInitialResponse::DelegateToAgent => json!({"state":"delegated"}),
        LearningInitialResponse::Skip => json!({"state":"skipped"}),
        LearningInitialResponse::RequestResearchOrPrototype { evidence_state } => json!({
            "state":"research_or_prototype_requested",
            "evidence_state":engineering_evidence_state_name(*evidence_state),
        }),
    }
}

fn materiality_disposition_json(disposition: &MaterialityDisposition) -> Value {
    match disposition {
        MaterialityDisposition::RepositoryOrEnvironmentFact => {
            json!({"state":"repository_or_environment_fact"})
        }
        MaterialityDisposition::SettledAuthority => json!({"state":"settled_authority"}),
        MaterialityDisposition::AgentOwnedImplementationChoice => {
            json!({"state":"agent_owned_implementation_choice"})
        }
        MaterialityDisposition::DelegatedImplementationChoice => {
            json!({"state":"delegated_implementation_choice"})
        }
        MaterialityDisposition::ExploratoryUncertainty { disposition } => json!({
            "state":"exploratory_uncertainty",
            "exploratory_disposition":match disposition {
                ExploratoryDisposition::ResearchRequired => "research_required",
                ExploratoryDisposition::PrototypeRequired => "prototype_required",
                ExploratoryDisposition::DeferredWithRevisit => "deferred_with_revisit",
                ExploratoryDisposition::ResolvedByResearch => "resolved_by_research",
            },
        }),
        MaterialityDisposition::UnresolvedUserOwnedOutcome {
            resolution_decision_id,
        } => json!({
            "state":"unresolved_user_owned_outcome",
            "resolution_decision_id":resolution_decision_id.map(|identity| identity.to_string()),
        }),
    }
}

fn learning_value_json(assessment: &LearningValueAssessment) -> Value {
    match assessment {
        LearningValueAssessment::Routine { rationale } => {
            json!({"state":"routine","rationale":rationale})
        }
        LearningValueAssessment::DeliberationWorthy {
            rationale,
            consequence_significance,
            transferable_principles,
            non_obvious_trade_offs,
        } => json!({
            "state":"deliberation_worthy",
            "rationale":rationale,
            "consequence_significance":consequence_significance,
            "transferable_principles":transferable_principles,
            "non_obvious_trade_offs":non_obvious_trade_offs,
        }),
    }
}

fn learning_deliberation_state_json(state: &LearningDeliberationState) -> Value {
    match state {
        LearningDeliberationState::AwaitingInitialResponse => {
            json!({"state":"awaiting_initial_response","required_action":"respond"})
        }
        LearningDeliberationState::AwaitingAgentFeedback { round } => {
            json!({"state":"awaiting_agent_feedback","round":round,"required_action":"feedback"})
        }
        LearningDeliberationState::FeedbackProvided { round } => {
            json!({"state":"feedback_provided","round":round,"required_action":"complete_or_reconsider"})
        }
        LearningDeliberationState::Completed {
            round,
            selected_alternatives,
        } => {
            json!({"state":"completed","round":round,"selected_alternatives":selected_alternatives.iter().map(learning_selection_json).collect::<Vec<_>>() })
        }
        LearningDeliberationState::Delegated { round } => {
            json!({"state":"delegated","round":round})
        }
        LearningDeliberationState::Skipped { round } => json!({"state":"skipped","round":round}),
        LearningDeliberationState::ResearchOrPrototypeRequired {
            round,
            evidence_state,
        } => {
            json!({"state":"research_or_prototype_required","round":round,"evidence_state":engineering_evidence_state_name(*evidence_state)})
        }
        LearningDeliberationState::ReconsiderationRequested { round } => {
            json!({"state":"reconsideration_requested","round":round,"required_action":"respond"})
        }
    }
}

const fn engineering_evidence_state_name(state: EngineeringChoiceEvidenceState) -> &'static str {
    match state {
        EngineeringChoiceEvidenceState::Sufficient => "sufficient",
        EngineeringChoiceEvidenceState::ResearchRequired => "research_required",
        EngineeringChoiceEvidenceState::PrototypeRequired => "prototype_required",
    }
}

const fn engineering_effect_category_name(state: EngineeringEffectCategory) -> &'static str {
    match state {
        EngineeringEffectCategory::PublicApiShapeOrSemantics => "public_api_shape_or_semantics",
        EngineeringEffectCategory::Compatibility => "compatibility",
        EngineeringEffectCategory::FailureOrErrorSemantics => "failure_or_error_semantics",
        EngineeringEffectCategory::PersistenceOrLifetime => "persistence_or_lifetime",
        EngineeringEffectCategory::PrivacyOrDisclosure => "privacy_or_disclosure",
        EngineeringEffectCategory::Security => "security",
        EngineeringEffectCategory::UserVisibleBehaviorOrDefault => {
            "user_visible_behavior_or_default"
        }
        EngineeringEffectCategory::PerformanceOrResourceBehavior => {
            "performance_or_resource_behavior"
        }
        EngineeringEffectCategory::ConcurrencyOrOperability => "concurrency_or_operability",
        EngineeringEffectCategory::MaintenanceOrSupport => "maintenance_or_support",
        EngineeringEffectCategory::ImplementationInternal => "implementation_internal",
    }
}

fn materiality_review_outcome_json(
    action: &str,
    outcome: volicord_operations::MaterialityReviewOutcome,
) -> Value {
    json!({
        "action":action,
        "review_candidate_id":outcome.review_candidate_id.to_string(),
        "review_revision":outcome.review_revision,
        "goal_context_id":outcome.goal_context_id.to_string(),
        "baseline_analysis_snapshot_id":outcome.baseline_analysis_snapshot_id.to_string(),
        "review_analysis_snapshot_id":outcome.review_analysis_snapshot_id.to_string(),
        "canonical_mutation":false,
    })
}

fn with_workflow(mut value: Value, workflow: WorkflowDirective) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.insert("workflow".into(), workflow_json(workflow));
    }
    value
}

fn workflow_json(workflow: WorkflowDirective) -> Value {
    let input_guidance = workflow_input_guidance(&workflow);
    json!({
        "stage":workflow_stage_name(workflow.stage),
        "disposition":workflow_disposition_name(workflow.disposition),
        "required_next_action":workflow.required_next_action.map(|action| json!({
            "tool":action.tool,
            "action":action.action,
        })),
        "blocks_ordinary_work":workflow.blocks_ordinary_work,
        "reason":workflow.reason,
        "satisfied_basis_identities":workflow.satisfied_basis_identities.into_iter().map(|basis| json!({
            "kind":basis.kind,
            "identity":basis.identity,
        })).collect::<Vec<_>>(),
        "unresolved_requirements":workflow.unresolved_requirements.into_iter().map(|requirement| json!({
            "dimension_id":requirement.dimension_id,
            "reason":requirement.reason,
            "basis_identities":requirement.basis_identities.into_iter().map(|basis| json!({
                "kind":basis.kind,
                "identity":basis.identity,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "input_guidance":input_guidance,
    })
}

fn workflow_input_guidance(workflow: &WorkflowDirective) -> Value {
    let identity = |kind: &str| {
        workflow
            .satisfied_basis_identities
            .iter()
            .find(|basis| basis.kind == kind)
            .map(|basis| basis.identity.clone())
    };
    match workflow.stage {
        WorkflowStage::EngineeringChoiceDiscovery => json!({
            "required_action":{"tool":"engineering_choice_discovery","action":"record"},
            "available_identities":{
                "project_id":identity("project"),
                "goal_context_id":identity("goal_context"),
                "baseline_analysis_snapshot_id":identity("baseline_analysis_snapshot"),
            },
            "required_fields":["source_operation","summary","choices"],
            "choice_required_fields":["choice_id","summary","affected_scope","alternatives","technical_consequences","source_ids","effect_categories","relationship","evidence_state"],
            "allowable_values":{
                "evidence_state":["sufficient","research_required","prototype_required"],
                "relationship_state":["independent","coupled"],
            },
            "draft_note":"Use current repository/Goal Sources. Omit mechanically equivalent syntax, local naming, and private helper splits.",
        }),
        WorkflowStage::MaterialityReview => json!({
            "required_action":{"tool":"materiality_review","action":"draft_then_record_or_revise"},
            "available_identities":{
                "project_id":identity("project"),
                "goal_context_id":identity("goal_context"),
                "baseline_analysis_snapshot_id":identity("baseline_analysis_snapshot"),
                "engineering_choice_discovery_candidate_id":identity("engineering_choice_discovery_candidate"),
                "materiality_review_candidate_id":identity("materiality_review_candidate"),
            },
            "draft_call":{
                "tool":"materiality_review",
                "action":"draft",
                "project_id":identity("project"),
                "engineering_choice_discovery_candidate_id":identity("engineering_choice_discovery_candidate"),
            },
            "deterministic_path":["call_draft_with_returned_discovery_identity","copy_record_request_prefilled_fields","select_one_returned_schema_variant_per_choice","supply_only_that_variant_semantic_fields","submit_returned_record_or_revise_request_once"],
            "server_derived_discovery_fields":["Goal and baseline identities","dimension and discovered-choice identities","summary","affected scope","material consequences","observable signals","discovery Source basis"],
            "required_semantic_judgments":["authority disposition and allowed evidence for that disposition","independent learning value","explicit learning participation state"],
            "authority_learning_routing":authority_learning_routing_json(),
            "schema_source":"The draft projects the same closed variants used by Materiality record/revise validation.",
        }),
        WorkflowStage::LearningDeliberation => {
            let pending = workflow
                .satisfied_basis_identities
                .iter()
                .filter(|basis| basis.kind == "learning_deliberation_candidate")
                .map(|basis| basis.identity.clone())
                .collect::<Vec<_>>();
            json!({
                "interaction_kind":"learning_participation_not_canonical_decision",
                "required_action":if pending.is_empty() { json!({"tool":"learning_deliberation","action":"begin"}) } else { json!({"tool":"learning_deliberation","action":"inspect"}) },
                "available_identities":{
                    "project_id":identity("project"),
                    "goal_context_id":identity("goal_context"),
                    "baseline_analysis_snapshot_id":identity("baseline_analysis_snapshot"),
                    "engineering_choice_discovery_candidate_id":identity("engineering_choice_discovery_candidate"),
                    "materiality_review_candidate_id":identity("materiality_review_candidate"),
                    "learning_deliberation_candidate_ids":pending,
                },
                "ordering":["begin_or_inspect_without_agent_recommendation","record_current_user_initial_response","provide_agent_feedback_and_recommendation_after_selection","complete_or_reconsider"],
                "allowable_initial_responses":["select","delegate","skip","research_required","prototype_required"],
                "authority_learning_routing":authority_learning_routing_json(),
                "learning_selection_contract":{
                    "authority":"agent_owned_or_explicitly_delegated",
                    "selection_kind":"bounded_learning_and_implementation_basis",
                    "canonical_decision":false,
                    "forbidden_substitute_operations":["candidate_manage.submit_question_from_materiality","decision_record"],
                    "warning":"Do not create a Question Candidate or call decision_record merely to record this learning selection. A genuinely user-owned material outcome must have remained on the Question/current-host Decision path instead."
                },
            })
        }
        _ => Value::Null,
    }
}

fn workflow_stage_name(value: WorkflowStage) -> &'static str {
    match value {
        WorkflowStage::ProjectResolution => "project_resolution",
        WorkflowStage::ProjectInitialization => "project_initialization",
        WorkflowStage::Recall => "recall",
        WorkflowStage::Goal => "goal",
        WorkflowStage::RepositoryBaseline => "repository_baseline",
        WorkflowStage::EngineeringChoiceDiscovery => "engineering_choice_discovery",
        WorkflowStage::MaterialityReview => "materiality_review",
        WorkflowStage::LearningDeliberation => "learning_deliberation",
        WorkflowStage::ResearchOrPrototype => "research_or_prototype",
        WorkflowStage::QuestionCandidate => "question_candidate",
        WorkflowStage::Inquiry => "inquiry",
        WorkflowStage::Decision => "decision",
        WorkflowStage::ReadyForWork => "ready_for_work",
        WorkflowStage::Checkpoint => "checkpoint",
    }
}

fn workflow_disposition_name(value: WorkflowDisposition) -> &'static str {
    match value {
        WorkflowDisposition::ProjectNotFound => "project_not_found",
        WorkflowDisposition::RecallRequired => "recall_required",
        WorkflowDisposition::GoalRequired => "goal_required",
        WorkflowDisposition::BaselineRequired => "baseline_required",
        WorkflowDisposition::EngineeringChoiceDiscoveryRequired => {
            "engineering_choice_discovery_required"
        }
        WorkflowDisposition::ReviewMissing => "review_missing",
        WorkflowDisposition::ReviewInvalid => "review_invalid",
        WorkflowDisposition::LearningDeliberationPending => "learning_deliberation_pending",
        WorkflowDisposition::ResearchRequired => "research_required",
        WorkflowDisposition::QuestionRequired => "question_required",
        WorkflowDisposition::CandidateResearchRequired => "candidate_research_required",
        WorkflowDisposition::CandidatePromotionRequired => "candidate_promotion_required",
        WorkflowDisposition::UserResponseRequired => "user_response_required",
        WorkflowDisposition::ReviewRevisionRequired => "review_revision_required",
        WorkflowDisposition::ReadyForWork => "ready_for_work",
        WorkflowDisposition::CheckpointRecorded => "checkpoint_recorded",
    }
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
fn operation_error(error: volicord_operations::Error) -> HostError {
    HostError::new(error.to_string())
}
fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, HostError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| HostError::new(format!("{key} is required")))
}
fn required_u64(value: &Value, key: &str) -> Result<u64, HostError> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| HostError::new(format!("{key} must be an unsigned integer")))
}
fn required_strings(value: &Value, key: &str) -> Result<Vec<String>, HostError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| HostError::new(format!("{key} must be an array")))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| HostError::new(format!("{key} must contain only strings")))
        })
        .collect()
}
fn project(value: &Value) -> Result<ProjectId, HostError> {
    parse_project(required_str(value, "project_id")?)
}
fn optional_project(value: &Value, key: &str) -> Result<Option<ProjectId>, HostError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(parse_project)
        .transpose()
}
fn parse_project(value: &str) -> Result<ProjectId, HostError> {
    Ok(ProjectId::from_bytes(parse_identity(value)?))
}
fn parse_source(value: &str) -> Result<SourceId, HostError> {
    Ok(SourceId::from_bytes(parse_identity(value)?))
}
fn parse_question(value: &str) -> Result<volicord_context::QuestionId, HostError> {
    Ok(volicord_context::QuestionId::from_bytes(parse_identity(
        value,
    )?))
}
fn parse_candidate(value: &str) -> Result<CandidateId, HostError> {
    Ok(CandidateId::from_bytes(parse_identity(value)?))
}
fn parse_confirmation(value: &str) -> Result<ConfirmationRequestId, HostError> {
    Ok(ConfirmationRequestId::from_bytes(parse_identity(value)?))
}
fn parse_guarded_operation(value: &str) -> Result<GuardedOperationId, HostError> {
    Ok(GuardedOperationId::from_bytes(parse_identity(value)?))
}
fn parse_provider_request(value: &str) -> Result<ProviderRequestId, HostError> {
    Ok(ProviderRequestId::from_bytes(parse_identity(value)?))
}
fn parse_identity(value: &str) -> Result<[u8; 16], HostError> {
    if value.len() != 32 {
        return Err(HostError::new(
            "identity must contain 32 hexadecimal digits",
        ));
    }
    let mut bytes = [0; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(pair).map_err(|error| HostError::new(error.to_string()))?;
        bytes[index] = u8::from_str_radix(text, 16)
            .map_err(|_| HostError::new("identity contains a non-hexadecimal digit"))?;
    }
    Ok(bytes)
}
fn new_operation_id() -> Result<OperationId, HostError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|error| HostError::new(format!("cannot create operation identity: {error}")))?;
    Ok(OperationId::from_bytes(bytes))
}
fn new_identity_text() -> Result<String, HostError> {
    Ok(new_operation_id()?.to_string())
}
fn document_kind(value: &str) -> Result<DocumentKind, HostError> {
    match value {
        "project-architecture-guide" => Ok(DocumentKind::ProjectArchitectureGuide),
        "decision-report" => Ok(DocumentKind::DecisionReport),
        "implementation-plan" => Ok(DocumentKind::ImplementationPlan),
        "handoff-resume" => Ok(DocumentKind::HandoffResume),
        _ => Err(HostError::new("unknown document kind")),
    }
}

fn narrative_plan_json(plan: &NarrativePlan) -> Value {
    json!({
        "document_kind":plan.document_kind.slug(),
        "requested_language":plan.requested_language,
        "plan_fingerprint":plan.plan_fingerprint,
        "source_title":plan.source_title,
        "generator":{
            "generator":plan.generator.generator,
            "agent":plan.generator.agent,
            "model":plan.generator.model,
        },
        "sections":plan.sections.iter().map(|section| json!({
            "key":section.key,
            "source_title":section.source_title,
            "claims":section.claims.iter().map(|claim| json!({
                "identity":claim.identity,
                "source_text":claim.source_text,
                "source_text_omission":claim.source_text_omission.as_ref().map(|omission| json!({
                    "exact_source_utf8_bytes":omission.exact_source_utf8_bytes,
                    "exact_source_character_count":omission.exact_source_character_count,
                    "source_sha256":omission.source_sha256,
                })),
                "protected_terms":claim.protected_terms,
                "omitted_protected_term_count":claim.omitted_protected_term_count,
                "class":format!("{:?}",claim.class).to_lowercase(),
                "source_basis":claim.source_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "decision_basis":claim.decision_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "analysis_basis":claim.analysis_basis.iter().map(ToString::to_string).collect::<Vec<_>>(),
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    })
}

fn narrative_realization(value: &Value) -> Result<NarrativeRealization, HostError> {
    let sections = value
        .get("sections")
        .and_then(Value::as_array)
        .ok_or_else(|| HostError::new("realization sections are required"))?
        .iter()
        .map(|section| {
            let claims = section
                .get("claims")
                .and_then(Value::as_array)
                .ok_or_else(|| HostError::new("realization claims are required"))?
                .iter()
                .map(|claim| {
                    Ok(RealizedNarrativeClaim {
                        identity: required_str(claim, "identity")?.to_owned(),
                        text: required_str(claim, "text")?.to_owned(),
                    })
                })
                .collect::<Result<Vec<_>, HostError>>()?;
            Ok(RealizedNarrativeSection {
                key: required_str(section, "key")?.to_owned(),
                title: required_str(section, "title")?.to_owned(),
                claims,
            })
        })
        .collect::<Result<Vec<_>, HostError>>()?;
    let generator = value
        .get("generator")
        .ok_or_else(|| HostError::new("realization generator is required"))?;
    Ok(NarrativeRealization {
        plan_fingerprint: required_str(value, "plan_fingerprint")?.to_owned(),
        title: required_str(value, "title")?.to_owned(),
        sections,
        generator: GeneratorIdentity {
            generator: required_str(generator, "generator")?.to_owned(),
            agent: Some(required_str(generator, "agent")?.to_owned()),
            model: Some(required_str(generator, "model")?.to_owned()),
        },
    })
}
