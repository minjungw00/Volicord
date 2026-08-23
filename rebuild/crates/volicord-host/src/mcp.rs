use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
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
    DisplayedQuestion, DuplicateAssessment, MaterialityAssessment, MaterialityStatus,
    QuestionCandidate, ResponseMapping, SubmissionOutcome,
};
use volicord_operations::{
    AnalysisSnapshotId, BackgroundProviderOperationDraft, CandidateRepositoryResearchDraft,
    CommandVerificationDraft, ConfirmationDecision, ConfirmationRejection, ConfirmationRequestId,
    FilterOutcome, GroundedCheckpointDraft, GuardedOperationId, GuardedOperationOutcome,
    GuardedProviderInspection, GuardedProviderPreparation, GuardedProviderPreparationOutcome,
    HealthState, LocalOperations, ProjectResolution, ProviderRequestId, ProviderRequestOutcome,
    ProviderRequestRecord, RequestingProvenance, ScopeOutcome, SourceClass, TransmissionOutcome,
};
use volicord_projections::{
    CandidateDependencyState, DocumentKind, DocumentRequest, FixedLocale, GeneratorIdentity,
    NarrativePlan, NarrativeRealization, NarrativeRealizationState, OutputFormat,
    RealizedNarrativeClaim, RealizedNarrativeSection,
};

pub const HOST_TOOL_NAMES: [&str; 18] = [
    "project_resolve",
    "project_initialize",
    "project_health",
    "recall",
    "repository_understanding",
    "repository_analyze",
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

const SERVER_INSTRUCTIONS: &str = "Volicord is active because this repository was explicitly authorized. For every fresh project-scoped session, STOP before repository inspection, edits, or continuation: call project_resolve first. When resolution finds a Project, recall must succeed before inspecting, editing, or continuing repository work. A not_found result requires explicit project_initialize, a current-host Goal through context_record, and a repository baseline through repository_analyze. After that baseline and before the first ordinary repository write, screen every unresolved choice relevant to the requested outcome into exactly one category: repository/environment fact--resolve through research, not a user Question; accepted repository/product contract--apply it and do not reopen it to manufacture a Question; delegated implementation choice--the agent may choose within the active contract; implementation choices explicitly delegated by active architecture/product contracts, including renderer/layout/detail choices, are not user Questions; or material user-owned outcome--STOP before implementing that outcome and use the existing Question and Decision path. Strong material signals include user-visible default behavior, CLI/API compatibility behavior, externally observable error or failure policy, privacy/security posture, maintenance/support policy, and any outcome where repository research leaves multiple viable policies that materially change what the user or downstream automation experiences. Public invalid-input behavior and batch-failure continuation policy are material observable outcomes when research leaves multiple viable policies. A library default, conventional behavior, implementation simplicity, or agent recommendation does not authorize selecting a material user-owned outcome. For such an outcome, use candidate_manage to submit the Question Candidate, attach source-grounded repository research, review materiality, mark it ready, and explicitly promote it. Then read inquiry_frontier, present the actual alternatives, recommendation, and trade-offs, obtain an explicit current-host user response, call decision_record, and only then apply that Decision. Repository/environment facts remain research and must not be asked of the user. Never substitute an agent recommendation or implementation preference for a user Decision. Once applicable Decisions and contracts resolve the material outcome, ordinary code edits require no new approval ceremony. repository_analyze is authorized local analysis, not background-provider transmission; background_semantic_operation is the separate explicit provider boundary. Record passed or failed Checkpoint verification only from the same actually observed command execution with a numeric exit status; output-only text is insufficient. Incidental inspection commands need not become Checkpoint verification facts. Meaningful completed or paused work uses a source-grounded Checkpoint. Non-project requests and unrelated greetings require no Volicord ceremony.";

#[derive(Debug)]
pub struct HostError {
    message: String,
}

impl HostError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
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
            "instructions":SERVER_INSTRUCTIONS
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
            Err(error) => Ok(tool_result(json!({"error":error.to_string()}), true)),
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
        let value = self
            .operations
            .initialize_project(required_str(args, "display_name")?, repository.as_deref())
            .map_err(operation_error)?;
        Ok(
            json!({"project_id":value.project.id.to_string(),"display_name":value.project.display_name,"binding":value.binding.map(|binding| binding.binding.absolute_path)}),
        )
    }

    fn project_resolve(&self, args: &Value) -> Result<Value, HostError> {
        let value = self
            .operations
            .resolve_project(&PathBuf::from(required_str(args, "repository")?))
            .map_err(operation_error)?;
        Ok(match value {
            ProjectResolution::Found { project, binding } => json!({
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
            ProjectResolution::NotFound {
                canonical_repository_path,
            } => json!({
                "status":"not_found",
                "canonical_repository_path":canonical_repository_path,
            }),
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
        let brief = self
            .operations
            .recall(project(args)?)
            .map_err(operation_error)?;
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
        Ok(json!({
            "project_id":brief.project_id.to_string(),"project_name":brief.project_name,
            "goals":brief.goals_and_why.into_iter().map(|value| value.statement).collect::<Vec<_>>(),
            "decisions":brief.decisions.into_iter().map(|value| json!({"identity":value.decision_id.to_string(),"revision":value.revision,"state":format!("{:?}",value.state).to_lowercase(),"choice":format!("{:?}",value.choice),"rationale":value.user_rationale})).collect::<Vec<_>>(),
            "open_questions":brief.open_questions.into_iter().map(|value| json!({"identity":value.question_id.to_string(),"revision":value.revision,"prompt":value.prompt})).collect::<Vec<_>>(),
            "known_limits":brief.known_limits,"next_step":brief.next_meaningful_step,"checkpoint":checkpoint,"omitted_count":brief.omitted_count,
            "read_only":true
        }))
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
            .analyze(project(args)?, excludes)
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
        Ok(
            json!({"operation_id":result.operation_id.to_string(),"state":format!("{:?}",result.state).to_lowercase(),"duration_micros":result.duration_micros,"analysis_snapshot_id":analysis_snapshot_id,"repository_snapshot_id":repository_snapshot_id,"repository_source_id":repository_source_id,"completed_scopes":result.partial.completed_scopes,"failed_scopes":result.partial.failed_scopes,"omitted_scopes":result.partial.omitted_scopes,"diagnostic":result.diagnostic}),
        )
    }

    fn inquiry_frontier(&self, args: &Value) -> Result<Value, HostError> {
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
            .inquiry_frontier(project(args)?, scope)
            .map_err(operation_error)?;
        Ok(
            json!({"questions":value.questions.into_iter().map(|question| json!({"identity":question.question_id.to_string(),"revision":question.displayed_revision,"prompt":question.prompt_basis,"why_now":question.why_it_matters_now,"alternatives":question.alternatives.into_iter().map(|alternative| json!({"key":alternative.key,"label":alternative.label,"consequence":alternative.consequence})).collect::<Vec<_>>(),"recommendation":question.recommendation.alternative_key,"what_unlocks":question.what_the_answer_unlocks})).collect::<Vec<_>>(),"diagnostics":value.diagnostics.into_iter().map(|diagnostic| diagnostic.detail).collect::<Vec<_>>() }),
        )
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
        Ok(
            json!({"project_id":project_id.to_string(),"user_response_source_id":source_id.to_string(),"all_succeeded":result.all_succeeded(),"outcomes":result.items.into_iter().map(|(id,revision,outcome)| json!({"question_id":id.to_string(),"revision":revision,"outcome":format!("{:?}",outcome)})).collect::<Vec<_>>() }),
        )
    }

    fn checkpoint_record(&self, args: &Value) -> Result<Value, HostError> {
        let project_id = project(args)?;
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
                goal_context_id: ContextItemId::from_bytes(parse_identity(required_str(
                    args,
                    "goal_context_id",
                )?)?),
                baseline_analysis_snapshot_id: AnalysisSnapshotId::from_hex(required_str(
                    args,
                    "baseline_analysis_snapshot_id",
                )?)
                .map_err(HostError::new)?,
                kind: checkpoint_kind(required_str(args, "kind")?)?,
                work_state: work_state(required_str(args, "work_state")?)?,
                state_change: optional_string(args, "state_change")?,
                applied_decisions: decision_ids(args, "applied_decision_ids")?,
                decision_components: string_array(args, "decision_components")?,
                work_contexts: string_array(args, "work_contexts")?,
                met_revisit_triggers: string_array(args, "met_revisit_triggers")?,
                verification,
                known_limits: string_array(args, "known_limits")?,
                non_goals: string_array(args, "non_goals")?,
                next_step: required_str(args, "next_step")?.to_owned(),
                handoff_to: optional_string(args, "handoff_to")?,
            })
            .map_err(operation_error)?;
        Ok(json!({
            "checkpoint_id":result.checkpoint_id.to_string(),
            "revision":result.checkpoint_revision,
            "goal_context_id":result.goal_context_id.to_string(),
            "baseline_analysis_snapshot_id":result.baseline_analysis_snapshot_id.to_string(),
            "current_analysis_snapshot_id":result.current_analysis_snapshot_id.to_string(),
            "baseline_repository_snapshot_id":result.baseline_repository_snapshot_id.to_string(),
            "current_repository_snapshot_id":result.current_repository_snapshot_id.to_string(),
            "changed_paths":result.changed_paths,
            "applied_decision_ids":result.applied_decisions.into_iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            "verification_source_ids":result.verification_source_ids.into_iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        }))
    }

    fn context_record(&self, args: &Value) -> Result<Value, HostError> {
        let role = context_item_role(required_str(args, "role")?)?;
        let result = self
            .operations
            .record_current_host_user_context(
                project(args)?,
                "codex".into(),
                self.host_session.clone(),
                required_str(args, "user_turn")?.to_owned(),
                role,
                required_str(args, "statement")?.to_owned(),
            )
            .map_err(operation_error)?;
        Ok(json!({
            "project_id": project(args)?.to_string(),
            "source_id": result.source_id.to_string(),
            "context_item_id": result.context_item_id.to_string(),
            "revision": result.context_item_revision,
            "role": context_item_role_name(result.role),
        }))
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
                let draft = CandidateDraft {
                    project_id,
                    kind: CandidateKind::QuestionCandidate,
                    collection_mode: CandidateCollectionMode::Automatic,
                    origin: CandidateOrigin {
                        actor: Principal {
                            kind: PrincipalKind::Agent,
                            identity: "codex".into(),
                        },
                        subsystem: "inquiry".into(),
                        session: Some(self.host_session.clone()),
                        provenance_summary: "explicit Codex Question Candidate submission".into(),
                    },
                    collection_scope: CandidateCollectionScope {
                        project_id,
                        session: Some(self.host_session.clone()),
                        source_operation: Some(required_str(args, "source_operation")?.to_owned()),
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
                            affected_scope: string_array(args, "affected_scope")?,
                            possible_prerequisites: Vec::new(),
                            source_basis: source_basis.clone(),
                            repository_basis: Vec::new(),
                            freshness: CandidateFreshness::Current,
                            duplicate_assessment: DuplicateAssessment::NoDuplicate {
                                basis: required_str(args, "duplicate_basis")?.to_owned(),
                            },
                            materiality: MaterialityAssessment {
                                status: MaterialityStatus::Material,
                                rationale: Some(
                                    required_str(args, "materiality_rationale")?.to_owned(),
                                ),
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
                                rationale: required_str(args, "recommendation_rationale")?
                                    .to_owned(),
                                source_basis,
                            },
                            trade_offs: string_array(args, "trade_offs")?,
                            known_limits: string_array(args, "known_limits")?,
                            what_the_answer_unlocks: string_array(args, "what_unlocks")?,
                            allowed_non_choice_dispositions: NonUserQuestionOutcome::ALL.to_vec(),
                            research_state,
                        }),
                    },
                };
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
                candidate_research_lifecycle_json("attach_repository_research", &candidate)
            }
            "mark_research_ready" => {
                let candidate_id = parse_candidate(required_str(args, "candidate_id")?)?;
                let candidate = self
                    .operations
                    .mark_candidate_ready_to_ask(project_id, candidate_id)
                    .map_err(operation_error)?;
                candidate_research_lifecycle_json("mark_research_ready", &candidate)
            }
            "promote_question" => {
                let candidate_id = parse_candidate(required_str(args, "candidate_id")?)?;
                let result = self
                    .operations
                    .promote_question_candidate(project_id, candidate_id)
                    .map_err(operation_error)?;
                Ok(json!({
                    "action": "promote_question",
                    "candidate_id": result.candidate_id.to_string(),
                    "question_id": result.question_id.to_string(),
                    "canonical_replayed": result.canonical_replayed,
                    "candidate_reconciled": result.candidate_reconciled
                }))
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
        validate_schema(&self.input_schema, arguments, "arguments")
            .map_err(|error| HostError::new(format!("invalid {} arguments: {error}", self.name)))
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
            "Explicitly create and optionally bind a new Volicord Project after resolution found no existing repository binding.",
            object_schema(
                vec![
                    ("display_name", text_schema("Project display name", 1, 1024)),
                    ("repository", text_schema("Optional absolute repository path", 1, 4096)),
                ],
                &["display_name"],
            ),
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
            "Run authorized local repository inventory and structural analysis. This operation creates local repository-observation Sources and publishes analysis state only in the local Runtime Home; use the returned repository_source_id as the canonical source_ids basis for source-grounded repository research. It performs no background-provider or network transmission. background_semantic_operation is the separate explicit provider boundary.",
            object_schema(
                vec![
                    ("project_id", identity_schema("Project identity")),
                    ("excluded_paths", string_array_schema("Repository-relative paths to exclude")),
                ],
                &["project_id"],
            ),
            ToolBehavior::AdditiveClosed,
        ),
        "inquiry_frontier" => (
            "Read current promoted material Questions. Before implementation, present each actual alternative, recommendation, and trade-off and obtain an explicit current-host response. Repository-resolvable facts remain research; submit, attach source-grounded research, review, mark ready, and explicitly promote material Question Candidates through candidate_manage first.",
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
            "Record a grounded Checkpoint from a canonical Goal, repository baseline/current analysis, applicable Decisions, and truthful verification evidence. A passed or failed verification requires the numeric exit status from the same actually observed command execution; output-only text is insufficient. Incidental inspection commands need not be Checkpoint verification facts.",
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
            "Explicitly submit and research an agent Question Candidate, promote a reviewed ready Candidate to a Question, or disposition Candidate-local content without creating a user Decision.",
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
                text_schema("Why the Question materially matters now", 1, 4096),
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
                text_schema("Agent materiality-assessment rationale", 1, 4096),
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
                        ("command_label", text_schema("Bounded cooperative command label", 1, 1024)),
                        ("exit_code", unsigned_schema("Actual process exit code when termination is exited", 0)),
                        ("termination", enum_schema("Actual command termination", &["exited", "signaled", "spawn_failed", "indeterminate"])),
                        ("outcome", text_schema("Observed verification outcome", 1, 16_384)),
                    ],
                    &["state", "command_label", "termination", "outcome"],
                ),
                object_schema(
                    vec![
                        ("state", enum_schema("Executed verification state with a numeric observed exit status", &["passed", "failed"])),
                        ("command_label", text_schema("Bounded cooperative command label", 1, 1024)),
                        ("exit_code", unsigned_schema("Numeric exit status from this same observed command execution", 0)),
                        ("termination", enum_schema("Observed command termination", &["exited"])),
                        ("outcome", text_schema("Observed verification outcome; output text alone is insufficient", 1, 16_384)),
                    ],
                    &["state", "command_label", "exit_code", "termination", "outcome"],
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

fn validate_schema(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    if let Some(variants) = schema.get("oneOf").and_then(Value::as_array) {
        let failures = variants
            .iter()
            .filter_map(|variant| validate_schema(variant, value, path).err())
            .collect::<Vec<_>>();
        return match variants.len().saturating_sub(failures.len()) {
            1 => Ok(()),
            0 => Err(format!(
                "{path} does not match any allowed shape: {}",
                failures.join("; ")
            )),
            _ => Err(format!("{path} matches more than one allowed shape")),
        };
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => validate_object(schema, value, path),
        Some("string") => validate_string(schema, value, path),
        Some("array") => validate_array(schema, value, path),
        Some("integer") => validate_integer(schema, value, path),
        Some(kind) => Err(format!("{path} uses unsupported schema type {kind}")),
        None => Err(format!("{path} schema has no type")),
    }
}

fn validate_object(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{path} must be an object"))?;
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{path} schema has no properties"))?;
    if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
        if let Some(name) = object.keys().find(|name| !properties.contains_key(*name)) {
            return Err(format!("{path}.{name} is not allowed"));
        }
    }
    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for name in required.iter().filter_map(Value::as_str) {
            if !object.contains_key(name) {
                return Err(format!("{path}.{name} is required"));
            }
        }
    }
    for (name, child) in object {
        if let Some(child_schema) = properties.get(name) {
            validate_schema(child_schema, child, &format!("{path}.{name}"))?;
        }
    }
    Ok(())
}

fn validate_string(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let text = value
        .as_str()
        .ok_or_else(|| format!("{path} must be a string"))?;
    let length = text.chars().count() as u64;
    if let Some(minimum) = schema.get("minLength").and_then(Value::as_u64) {
        if length < minimum {
            return Err(format!("{path} is shorter than {minimum} characters"));
        }
    }
    if let Some(maximum) = schema.get("maxLength").and_then(Value::as_u64) {
        if length > maximum {
            return Err(format!("{path} exceeds {maximum} characters"));
        }
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        if !values
            .iter()
            .any(|candidate| candidate.as_str() == Some(text))
        {
            return Err(format!("{path} is not an allowed value"));
        }
    }
    match schema.get("pattern").and_then(Value::as_str) {
        Some("^[0-9a-fA-F]{32}$") if !is_hex(text, 32, true) => {
            Err(format!("{path} must contain 32 hexadecimal digits"))
        }
        Some("^sha256:[0-9a-f]{64}$")
            if !text
                .strip_prefix("sha256:")
                .is_some_and(|value| is_hex(value, 64, false)) =>
        {
            Err(format!("{path} must be a sha256 fingerprint"))
        }
        Some("^[0-9a-fA-F]{64}$") if !is_hex(text, 64, true) => {
            Err(format!("{path} must contain 64 hexadecimal digits"))
        }
        Some("^[0-9a-fA-F]{32}$" | "^[0-9a-fA-F]{64}$" | "^sha256:[0-9a-f]{64}$") | None => Ok(()),
        Some(pattern) => Err(format!("{path} uses unsupported schema pattern {pattern}")),
    }
}

fn validate_array(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{path} must be an array"))?;
    if let Some(minimum) = schema.get("minItems").and_then(Value::as_u64) {
        if values.len() < minimum as usize {
            return Err(format!("{path} must contain at least {minimum} items"));
        }
    }
    let item_schema = schema
        .get("items")
        .ok_or_else(|| format!("{path} schema has no item contract"))?;
    for (index, item) in values.iter().enumerate() {
        validate_schema(item_schema, item, &format!("{path}[{index}]"))?;
    }
    Ok(())
}

fn validate_integer(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let number = value
        .as_u64()
        .ok_or_else(|| format!("{path} must be an unsigned integer"))?;
    if let Some(minimum) = schema.get("minimum").and_then(Value::as_u64) {
        if number < minimum {
            return Err(format!("{path} must be at least {minimum}"));
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
    json!({
        "identity":candidate.candidate_id.to_string(),
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
                "protected_terms":claim.protected_terms,
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
