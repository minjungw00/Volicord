use serde_json::{json, Value};
use std::{
    error::Error as StdError,
    fmt,
    io::{BufRead, Write},
    path::PathBuf,
};
use volicord_context::{
    ApplicabilityScope, CanonicalRecordId, CheckpointDraft, CheckpointId, CheckpointKind, Clock,
    ContextItemCorrectionDraft, ContextItemId, CorrectionKind, DecisionCorrectionDraft, DecisionId,
    OperationId, ProjectId, QuestionId, SourceId, SystemClock, UserAcceptanceFact,
    UserAcceptanceState, UserReviewFact, UserReviewState, VerificationFact, VerificationState,
    WorkState,
};
use volicord_inquiry::{
    BatchResponseItem, CurrentHostResponse, DisplayedQuestion, ResponseMapping,
};
use volicord_operations::{
    ConfirmationDecision, ConfirmationRequestId, HealthState, LocalOperations,
};
use volicord_projections::{
    DocumentKind, DocumentRequest, FixedLocale, GeneratorIdentity, OutputFormat,
};

pub const HOST_TOOL_NAMES: [&str; 14] = [
    "project_initialize",
    "project_health",
    "recall",
    "repository_understanding",
    "repository_analyze",
    "inquiry_frontier",
    "decision_record",
    "checkpoint_record",
    "canonical_inspect",
    "canonical_mutate",
    "candidate_inspect",
    "privacy_status",
    "document_preview",
    "guarded_interaction",
];

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
}

impl HostAdapter {
    pub fn new(operations: LocalOperations) -> Self {
        Self {
            operations,
            initialized: false,
            client_supports_elicitation: false,
            host_session: new_identity_text().unwrap_or_else(|_| "unavailable-session".into()),
        }
    }

    pub fn operations(&self) -> &LocalOperations {
        &self.operations
    }

    pub fn handle(&mut self, message: Value) -> Option<Value> {
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
            "instructions":"Use high-level Project capabilities. User Decisions and Guarded confirmations require an explicit current-host user turn."
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
                "project_initialize" => self.project_initialize(&arguments),
                "project_health" => self.project_health(&arguments),
                "recall" => self.recall(&arguments),
                "repository_understanding" => self.repository_understanding(&arguments),
                "repository_analyze" => self.repository_analyze(&arguments),
                "inquiry_frontier" => self.inquiry_frontier(&arguments),
                "decision_record" => self.decision_record(&arguments),
                "checkpoint_record" => self.checkpoint_record(&arguments),
                "canonical_inspect" => self.canonical_inspect(&arguments),
                "canonical_mutate" => self.canonical_mutate(&arguments),
                "candidate_inspect" => self.candidate_inspect(&arguments),
                "privacy_status" => self.privacy_status(&arguments),
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
        Ok(json!({
            "project_id":brief.project_id.to_string(),"project_name":brief.project_name,
            "goals":brief.goals_and_why.into_iter().map(|value| value.statement).collect::<Vec<_>>(),
            "decisions":brief.decisions.into_iter().map(|value| json!({"identity":value.decision_id.to_string(),"revision":value.revision,"state":format!("{:?}",value.state).to_lowercase(),"choice":format!("{:?}",value.choice),"rationale":value.user_rationale})).collect::<Vec<_>>(),
            "open_questions":brief.open_questions.into_iter().map(|value| json!({"identity":value.question_id.to_string(),"revision":value.revision,"prompt":value.prompt})).collect::<Vec<_>>(),
            "known_limits":brief.known_limits,"next_step":brief.next_meaningful_step,"omitted_count":brief.omitted_count,
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
        Ok(
            json!({"operation_id":result.operation_id.to_string(),"state":format!("{:?}",result.state).to_lowercase(),"duration_micros":result.duration_micros,"completed_scopes":result.partial.completed_scopes,"failed_scopes":result.partial.failed_scopes,"omitted_scopes":result.partial.omitted_scopes,"diagnostic":result.diagnostic}),
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
        let source = self
            .operations
            .record_user_source(
                project_id,
                "codex".into(),
                self.host_session.clone(),
                required_str(args, "user_turn")?.to_owned(),
            )
            .map_err(operation_error)?;
        let source_id = parse_source(&source.identity)?;
        let canonical = self
            .operations
            .canonical_basis(project_id)
            .map_err(operation_error)?;
        let result = self
            .operations
            .record_checkpoint(
                project_id,
                CheckpointDraft {
                    expected_project_revision: canonical.project.revision,
                    kind: CheckpointKind::Handoff,
                    goal: required_str(args, "goal")?.to_owned(),
                    work_state: WorkState::Paused,
                    state_change: Some("explicit Codex host checkpoint".into()),
                    source_basis: vec![source_id],
                    changed_source_basis: Vec::new(),
                    changed_paths: Vec::new(),
                    applied_decisions: Vec::new(),
                    verification: vec![VerificationFact {
                        state: VerificationState::NotRun,
                        source_id: None,
                        outcome: None,
                    }],
                    user_review: UserReviewFact {
                        state: UserReviewState::NotRequested,
                        source_id: None,
                    },
                    user_acceptance: UserAcceptanceFact {
                        state: UserAcceptanceState::NotRequested,
                        source_id: None,
                    },
                    known_limits: args
                        .get("known_limits")
                        .and_then(Value::as_array)
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(Value::as_str)
                                .map(ToOwned::to_owned)
                                .collect()
                        })
                        .unwrap_or_default(),
                    non_goals: Vec::new(),
                    open_questions: Vec::new(),
                    next_step: required_str(args, "next_step")?.to_owned(),
                    handoff_to: Some("next Codex session".into()),
                },
            )
            .map_err(operation_error)?;
        Ok(
            json!({"checkpoint_id":result.identity,"revision":result.revision,"user_response_source_id":source_id.to_string()}),
        )
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
            "forget" => {
                let bytes = parse_identity(required_str(args, "record_id")?)?;
                let record = match required_str(args, "record_kind")? {
                    "source" => CanonicalRecordId::Source(SourceId::from_bytes(bytes)),
                    "question" => CanonicalRecordId::Question(QuestionId::from_bytes(bytes)),
                    "decision" => CanonicalRecordId::Decision(DecisionId::from_bytes(bytes)),
                    "context_item" => {
                        CanonicalRecordId::ContextItem(ContextItemId::from_bytes(bytes))
                    }
                    "checkpoint" => CanonicalRecordId::Checkpoint(CheckpointId::from_bytes(bytes)),
                    _ => return Err(HostError::new("record kind is not forgettable")),
                };
                self.operations
                    .forget_record(project_id, record, authorization)
            }
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
        Ok(
            json!({"candidates":projection.candidate_inspection.into_iter().map(|candidate| json!({"identity":candidate.candidate_id.to_string(),"exists":candidate.exists,"health":format!("{:?}",candidate.health).to_lowercase(),"kind":candidate.kind.map(|value| format!("{:?}",value).to_lowercase()),"summary":candidate.bounded_summary,"disposition":candidate.promotion_disposition.map(|value| format!("{:?}",value).to_lowercase()),"opt_out":candidate.current_applicable_opt_out.len()})).collect::<Vec<_>>(),"read_only":true}),
        )
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
        let set = self
            .operations
            .documents(project(args)?, &request)
            .map_err(operation_error)?;
        let document = match kind {
            DocumentKind::ProjectArchitectureGuide => set.project_architecture_guide,
            DocumentKind::DecisionReport => set.decision_report,
            DocumentKind::ImplementationPlan => set.implementation_plan,
            DocumentKind::HandoffResume => set.handoff_resume,
        };
        let content = if format == OutputFormat::Html {
            document.html.content
        } else {
            document.markdown.content
        };
        Ok(
            json!({"kind":kind.slug(),"format":format!("{:?}",format).to_lowercase(),"content":content,"canonical_mutation":false}),
        )
    }

    fn guarded_interaction(&self, args: &Value) -> Result<Value, HostError> {
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
        Ok(
            json!({"confirmation_request_id":response.confirmation_request_identity.to_string(),"request_revision":response.request_revision,"effect_fingerprint":response.effect_fingerprint,"decision":format!("{:?}",response.decision).to_lowercase(),"user_response_source_id":response.user_response_source_id.to_string()}),
        )
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
            })
        })
        .collect()
}

struct ToolContract {
    name: &'static str,
    description: &'static str,
    input_schema: Value,
}

impl ToolContract {
    fn validate(&self, arguments: &Value) -> Result<(), HostError> {
        validate_schema(&self.input_schema, arguments, "arguments")
            .map_err(|error| HostError::new(format!("invalid {} arguments: {error}", self.name)))
    }
}

fn tool_contract(name: &str) -> Option<ToolContract> {
    let (description, input_schema) = match name {
        "project_initialize" => (
            "Initialize and optionally bind a clean current Volicord Project.",
            object_schema(
                vec![
                    ("display_name", text_schema("Project display name", 1, 1024)),
                    ("repository", text_schema("Optional absolute repository path", 1, 4096)),
                ],
                &["display_name"],
            ),
        ),
        "project_health" => (
            "Distinguish MCP connection from Project capability health.",
            object_schema(
                vec![("project_id", identity_schema("Optional Project identity"))],
                &[],
            ),
        ),
        "recall" => (
            "Read a bounded source-grounded Project resume brief.",
            project_schema(),
        ),
        "repository_understanding" => (
            "Read the Project overview, repository map, Decision-context-code links, gaps, and degraded states.",
            project_schema(),
        ),
        "repository_analyze" => (
            "Run local repository inventory and structural analysis.",
            object_schema(
                vec![
                    ("project_id", identity_schema("Project identity")),
                    ("excluded_paths", string_array_schema("Repository-relative paths to exclude")),
                ],
                &["project_id"],
            ),
        ),
        "inquiry_frontier" => (
            "Read current material Questions and choices.",
            object_schema(
                vec![
                    ("project_id", identity_schema("Project identity")),
                    ("material_scope", string_array_schema("Material scope filters")),
                ],
                &["project_id"],
            ),
        ),
        "decision_record" => (
            "Record one exact current-host user response against one current Question revision.",
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
        ),
        "checkpoint_record" => (
            "Record a source-linked current-host handoff Checkpoint.",
            object_schema(
                vec![
                    ("project_id", identity_schema("Project identity")),
                    ("user_turn", user_turn_schema()),
                    ("goal", text_schema("Current Project goal", 1, 16_384)),
                    ("next_step", text_schema("Next meaningful step", 1, 16_384)),
                    ("known_limits", string_array_schema("Known limits")),
                ],
                &["project_id", "user_turn", "goal", "next_step"],
            ),
        ),
        "canonical_inspect" => (
            "Inspect canonical memory without mutation.",
            project_schema(),
        ),
        "canonical_mutate" => (
            "Correct, supersede, or forget canonical memory through Local Operations using an explicit current-host user turn.",
            json!({"oneOf": canonical_mutation_schemas()}),
        ),
        "candidate_inspect" => (
            "Inspect bounded Candidate lifecycle state without mutation.",
            project_schema(),
        ),
        "privacy_status" => (
            "Inspect Project background-provider consent and local-only state.",
            project_schema(),
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
                ],
                &["project_id", "kind"],
            ),
        ),
        "guarded_interaction" => (
            "Inspect or answer one exact Guarded request/revision; returns viewer/CLI fallback when host elicitation is unavailable.",
            json!({"oneOf": guarded_interaction_schemas()}),
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
    })
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
        Some("^[0-9a-fA-F]{32}$" | "^sha256:[0-9a-f]{64}$") | None => Ok(()),
        Some(pattern) => Err(format!("{path} uses unsupported schema pattern {pattern}")),
    }
}

fn validate_array(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{path} must be an array"))?;
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
fn parse_confirmation(value: &str) -> Result<ConfirmationRequestId, HostError> {
    Ok(ConfirmationRequestId::from_bytes(parse_identity(value)?))
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
