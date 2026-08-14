use crate::{
    operations::{parse_identity, select_document},
    ConfirmationDecision, ConfirmationRequestId, Error, GuardedEffectCategory, GuardedEffectDraft,
    GuardedRisk, LocalOperations, RequestingProvenance, RuntimeLayout,
};
use serde_json::{json, Value};
use std::{
    ffi::{OsStr, OsString},
    io::Write,
    path::PathBuf,
};
use volicord_context::{
    CanonicalRecordId, CheckpointDraft, CheckpointKind, ContextItemCorrectionDraft, ContextItemId,
    CorrectionKind, DecisionCorrectionDraft, DecisionId, Principal, PrincipalKind, ProjectId,
    SourceId, UserAcceptanceFact, UserAcceptanceState, UserReviewFact, UserReviewState,
    VerificationFact, VerificationState, WorkState,
};
use volicord_privacy::{
    ProviderIntentProvenance, ProviderOptInPolicy, ProviderRetentionPolicy, SecretFilteringPolicy,
    SourceExclusionPolicy,
};
use volicord_projections::{
    DocumentKind, DocumentRequest, FixedLocale, GeneratorIdentity, OutputFormat,
    RequestedDestination,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CliExit(i32);

impl CliExit {
    pub const SUCCESS: Self = Self(0);
    pub const USAGE: Self = Self(2);
    pub const FAILURE: Self = Self(1);
    pub const fn code(self) -> i32 {
        self.0
    }
}

pub fn run_cli<I, S>(args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    match execute(args.into_iter().map(Into::into).collect(), stdout) {
        Ok(()) => CliExit::SUCCESS,
        Err(error) => {
            let _ = writeln!(stderr, "{}", error.message());
            if error.message().starts_with("usage:") {
                CliExit::USAGE
            } else {
                CliExit::FAILURE
            }
        }
    }
}

fn execute(mut args: Vec<OsString>, stdout: &mut dyn Write) -> Result<(), Error> {
    let runtime = if args
        .first()
        .is_some_and(|value| value == OsStr::new("--runtime"))
    {
        if args.len() < 3 {
            return Err(usage("--runtime requires an absolute path and a command"));
        }
        let value = PathBuf::from(args.remove(1));
        args.remove(0);
        RuntimeLayout::new(value)?
    } else {
        RuntimeLayout::from_environment()?
    };
    let operations = LocalOperations::new(runtime);
    let mut cursor = Cursor::new(args);
    let command = cursor.next("command")?;
    let value = match command.as_str() {
        "project" => project(&operations, &mut cursor)?,
        "health" => health(&operations, &mut cursor)?,
        "analyze" => analyze(&operations, &mut cursor, false)?,
        "rebuild" => rebuild(&operations, &mut cursor)?,
        "reindex" => reindex(&operations, &mut cursor)?,
        "repair" => repair(&operations, &mut cursor)?,
        "portable" => portable(&operations, &mut cursor)?,
        "canonical" => canonical(&operations, &mut cursor)?,
        "candidates" => candidates(&operations, &mut cursor)?,
        "privacy" => privacy(&operations, &mut cursor)?,
        "recall" => recall(&operations, &mut cursor)?,
        "documents" => documents(&operations, &mut cursor)?,
        "inquiry" => inquiry(&operations, &mut cursor)?,
        "checkpoint" => checkpoint(&operations, &mut cursor)?,
        "guarded" => guarded(&operations, &mut cursor)?,
        "help" | "--help" | "-h" => json!({"usage": USAGE}),
        _ => return Err(usage("unknown command")),
    };
    cursor.done()?;
    serde_json::to_writer_pretty(&mut *stdout, &value)
        .map_err(|error| Error::with_source("cannot render CLI result", error))?;
    writeln!(stdout).map_err(|error| Error::with_source("cannot write CLI result", error))?;
    Ok(())
}

const USAGE: &str = "volicord [--runtime ABSOLUTE_PATH] <project|health|analyze|rebuild|reindex|repair|portable|canonical|candidates|privacy|recall|documents|inquiry|checkpoint|guarded> ...";

fn usage(detail: &str) -> Error {
    Error::new(format!("usage: {USAGE}\n{detail}"))
}

fn project(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    match cursor.next("project command")?.as_str() {
        "init" => {
            let name = cursor.next("project display name")?;
            let repository = if cursor.peek("--repository") {
                cursor.next("--repository")?;
                Some(PathBuf::from(cursor.next("repository path")?))
            } else {
                None
            };
            let value = operations.initialize_project(name, repository.as_deref())?;
            Ok(json!({
                "operation": "project_init",
                "project_id": value.project.id.to_string(),
                "display_name": value.project.display_name,
                "revision": value.project.revision,
                "binding": value.binding.map(|binding| json!({"path": binding.binding.absolute_path, "revision": binding.binding.revision, "clone_identity": binding.clone_identity, "worktree_identity": binding.worktree_identity})),
            }))
        }
        "bind" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let path = PathBuf::from(cursor.next("repository path")?);
            let revision = if cursor.peek("--revision") {
                cursor.next("--revision")?;
                Some(number(&cursor.next("binding revision")?)?)
            } else {
                None
            };
            let value = operations.bind_project(project, revision, &path)?;
            Ok(
                json!({"operation":"project_bind", "project_id":project.to_string(), "path":value.binding.absolute_path, "revision":value.binding.revision, "clone_identity":value.clone_identity, "worktree_identity":value.worktree_identity}),
            )
        }
        _ => Err(usage("project requires init or bind")),
    }
}

fn health(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    let project = cursor
        .optional()
        .map(|value| project_id(&value))
        .transpose()?;
    let report = operations.health(project);
    Ok(json!({
        "operation":"health", "state":debug_name(report.state), "runtime_root":report.runtime_root,
        "canonical_available":report.canonical_available, "candidate_available":report.candidate_available,
        "privacy_available":report.privacy_available, "guarded_available":report.guarded_available, "repository_available":report.repository_available,
        "issues":report.issues.into_iter().map(|issue| json!({"kind":debug_name(issue.kind),"scope":issue.scope,"detail":issue.detail})).collect::<Vec<_>>()
    }))
}

fn analyze(
    operations: &LocalOperations,
    cursor: &mut Cursor,
    rebuild: bool,
) -> Result<Value, Error> {
    let project = project_id(&cursor.next("Project ID")?)?;
    let mut excludes = Vec::new();
    while cursor.peek("--exclude") {
        cursor.next("--exclude")?;
        excludes.push(cursor.next("excluded path")?);
    }
    let result = if rebuild {
        operations.rebuild_analysis(project, excludes)?
    } else {
        operations.analyze(project, excludes)?
    };
    let analysis = result
        .value
        .as_ref()
        .ok_or_else(|| Error::new("analysis ended without an inspectable result"))?;
    Ok(json!({
        "operation":if rebuild {"analysis_rebuild"} else {"analyze"}, "operation_id":result.operation_id.to_string(), "state":debug_name(result.state),
        "duration_micros":result.duration_micros, "repository_snapshot":analysis.repository.identity.to_string(), "analysis_snapshot":analysis.analysis.identity.to_string(),
        "stored_at":analysis.stored_at, "completed_scopes":result.partial.completed_scopes, "failed_scopes":result.partial.failed_scopes,
        "omitted_scopes":result.partial.omitted_scopes, "diagnostic":result.diagnostic
    }))
}

fn rebuild(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    if cursor.next("rebuild target")? != "analysis" {
        return Err(usage(
            "rebuild currently supports only: rebuild analysis PROJECT",
        ));
    }
    analyze(operations, cursor, true)
}

fn reindex(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    let project = project_id(&cursor.next("Project ID")?)?;
    let excludes = excluded_paths(cursor)?;
    repair_json("reindex", operations.reindex(project, excludes)?)
}

fn repair(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    let project = project_id(&cursor.next("Project ID")?)?;
    let scope = cursor.next("repair scope")?;
    let excludes = excluded_paths(cursor)?;
    repair_json("repair", operations.repair(project, scope, excludes)?)
}

fn excluded_paths(cursor: &mut Cursor) -> Result<Vec<String>, Error> {
    let mut excludes = Vec::new();
    while cursor.peek("--exclude") {
        cursor.next("--exclude")?;
        excludes.push(cursor.next("excluded path")?);
    }
    Ok(excludes)
}

fn repair_json(operation: &str, result: crate::RepairOutcome) -> Result<Value, Error> {
    let analysis =
        result.operation.value.as_ref().ok_or_else(|| {
            Error::new("derived reconstruction ended without an inspectable result")
        })?;
    Ok(json!({
        "operation":operation,
        "operation_id":result.operation.operation_id.to_string(),
        "state":debug_name(result.operation.state),
        "kind":debug_name(result.kind),
        "scope":result.affected_scope,
        "diagnosis":result.diagnosis,
        "discarded_entries":result.discarded_entries,
        "analysis_snapshot":analysis.analysis.identity.to_string(),
        "stored_at":analysis.stored_at,
        "completed_scopes":result.operation.partial.completed_scopes,
        "failed_scopes":result.operation.partial.failed_scopes,
        "omitted_scopes":result.operation.partial.omitted_scopes,
        "diagnostic":result.operation.diagnostic,
    }))
}

fn portable(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    match cursor.next("portable command")?.as_str() {
        "export" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let destination = absolute_path(&cursor.next("bundle destination")?)?;
            let result = operations.export_bundle(project, &destination)?;
            Ok(
                json!({"operation":"portable_export","project_id":result.project_id.to_string(),"path":result.path,"checksum":result.checksum,"history_basis":result.history_basis,"bytes_written":result.bytes_written}),
            )
        }
        "import" => {
            let source = absolute_path(&cursor.next("bundle source")?)?;
            let result = operations.import_bundle(&source)?;
            Ok(
                json!({"operation":"portable_import","project_id":result.project_id.to_string(),"checksum":result.checksum,"history_basis":result.history_basis,"status":debug_name(result.status)}),
            )
        }
        _ => Err(usage("portable requires export or import")),
    }
}

fn canonical(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    match cursor.next("canonical command")?.as_str() {
        "inspect" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let projection = operations.project_projection(project)?;
            Ok(
                json!({"operation":"canonical_inspect","project_id":project.to_string(),"health":debug_name(projection.health),"records":projection.canonical_inspection.into_iter().map(|item| json!({"kind":debug_name(item.kind),"identity":item.identity,"revision":item.revision,"lifecycle_state":item.lifecycle_state,"statement_role":item.statement_role,"summary":item.summary,"source_basis":item.source_basis.into_iter().map(|id| id.to_string()).collect::<Vec<_>>()})).collect::<Vec<_>>(),"issues":projection.issues.into_iter().map(|issue| json!({"kind":debug_name(issue.kind),"scope":issue.affected_scope,"reason":issue.reason})).collect::<Vec<_>>() }),
            )
        }
        "user-source" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let result = operations.record_user_source(
                project,
                cursor.next("host")?,
                cursor.next("session")?,
                cursor.next("turn text")?,
            )?;
            mutation_json("canonical_user_source", result)
        }
        "correct-context" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let item = ContextItemId::from_bytes(parse_identity(&cursor.next("Context Item ID")?)?);
            let revision = number(&cursor.next("expected revision")?)?;
            let source = source_id(&cursor.next("user Source ID")?)?;
            let text = cursor.next("corrected statement")?;
            mutation_json(
                "correct_context",
                operations.correct_context_item(
                    project,
                    item,
                    ContextItemCorrectionDraft {
                        expected_revision: revision,
                        corrected_statement: text,
                        kind: CorrectionKind::Expression,
                        user_authorization_source_id: source,
                    },
                )?,
            )
        }
        "correct-decision" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let decision = DecisionId::from_bytes(parse_identity(&cursor.next("Decision ID")?)?);
            let revision = number(&cursor.next("expected revision")?)?;
            let source = source_id(&cursor.next("user Source ID")?)?;
            let rationale = cursor.next("corrected rationale")?;
            mutation_json(
                "correct_decision",
                operations.correct_decision(
                    project,
                    decision,
                    DecisionCorrectionDraft {
                        expected_revision: revision,
                        corrected_user_rationale: Some(rationale),
                        kind: CorrectionKind::Expression,
                        user_authorization_source_id: source,
                    },
                )?,
            )
        }
        "supersede-decision" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let previous =
                DecisionId::from_bytes(parse_identity(&cursor.next("previous Decision ID")?)?);
            let source = source_id(&cursor.next("current-host user Source ID")?)?;
            let alternative = cursor.next("displayed alternative key")?;
            let rationale = cursor.optional();
            mutation_json(
                "supersede_decision",
                operations.supersede_decision_choice(
                    project,
                    previous,
                    source,
                    alternative,
                    rationale,
                )?,
            )
        }
        "forget" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let kind = cursor.next("record kind")?;
            let identity = parse_identity(&cursor.next("record ID")?)?;
            let authorization = source_id(&cursor.next("user authorization Source ID")?)?;
            let record = match kind.as_str() {
                "source" => CanonicalRecordId::Source(SourceId::from_bytes(identity)),
                "question" => CanonicalRecordId::Question(volicord_context::QuestionId::from_bytes(identity)),
                "decision" => CanonicalRecordId::Decision(DecisionId::from_bytes(identity)),
                "context_item" => CanonicalRecordId::ContextItem(ContextItemId::from_bytes(identity)),
                "checkpoint" => CanonicalRecordId::Checkpoint(volicord_context::CheckpointId::from_bytes(identity)),
                _ => return Err(usage("forgettable kind must be source, question, decision, context_item, or checkpoint")),
            };
            mutation_json(
                "canonical_forget",
                operations.forget_record(project, record, authorization)?,
            )
        }
        _ => Err(usage(
            "canonical requires inspect, user-source, correct-context, correct-decision, supersede-decision, or forget",
        )),
    }
}

fn candidates(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    let project = project_id(&cursor.next("Project ID")?)?;
    let projection = operations.project_projection(project)?;
    Ok(
        json!({"operation":"candidate_inspection","project_id":project.to_string(),"health":debug_name(projection.health),"candidates":projection.candidate_inspection.into_iter().map(|candidate| json!({"identity":candidate.candidate_id.to_string(),"exists":candidate.exists,"health":debug_name(candidate.health),"revision":candidate.revision,"kind":candidate.kind.map(debug_name),"summary":candidate.bounded_summary,"content_cleaned":candidate.content_cleaned,"promotion_disposition":candidate.promotion_disposition.map(debug_name)})).collect::<Vec<_>>() }),
    )
}

fn privacy(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    match cursor.next("privacy command")?.as_str() {
        "status" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let status = operations.privacy_status(project)?;
            Ok(
                json!({"operation":"privacy_status","project_id":project.to_string(),"configuration_state":debug_name(status.configuration_state),"policy_revision":status.current_opt_in.as_ref().map(|value| value.revision),"policy_state":status.current_opt_in.as_ref().map(|value| debug_name(value.state)),"provider":status.current_opt_in.as_ref().map(|value| value.policy.provider.clone()),"model":status.current_opt_in.as_ref().map(|value| value.policy.model.clone()),"allowed_source_scopes":status.current_opt_in.as_ref().map(|value| value.policy.allowed_source_scopes.clone()),"request_count":status.requests.len(),"managed_derived_count":status.managed_derived.len()}),
            )
        }
        "enable" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let provider = cursor.next("provider")?;
            let model = cursor.next("model")?;
            let source = source_id(&cursor.next("current-host user Source ID")?)?;
            let scopes = cursor.remaining();
            if scopes.is_empty() {
                return Err(usage(
                    "privacy enable requires at least one explicit source scope",
                ));
            }
            let policy = ProviderOptInPolicy {
                project_id: project,
                provider,
                model,
                purpose: "background semantic analysis".into(),
                requested_capability: "semantic".into(),
                allowed_source_scopes: scopes,
                exclusions: SourceExclusionPolicy {
                    path_prefixes: Vec::new(),
                    file_classes: Vec::new(),
                    basis: "explicit CLI scope".into(),
                },
                filtering: SecretFilteringPolicy {
                    enabled: true,
                    line_markers: vec!["SECRET".into(), "TOKEN".into(), "PASSWORD".into()],
                    replacement: "[filtered]".into(),
                    known_limits: vec!["marker filtering is not complete secret detection".into()],
                },
                retention: ProviderRetentionPolicy {
                    local_annotation_retained_until: None,
                    local_basis: "until explicit deletion".into(),
                    provider_expectation: "provider policy applies".into(),
                    provider_known_limits: Vec::new(),
                },
            };
            let event = operations.enable_provider(
                policy,
                privacy_intent(source, "enable background semantic provider"),
            )?;
            Ok(
                json!({"operation":"privacy_enable","project_id":project.to_string(),"revision":event.revision,"state":debug_name(event.state),"provider":event.policy.provider,"model":event.policy.model,"allowed_source_scopes":event.policy.allowed_source_scopes}),
            )
        }
        "disable" | "revoke" => {
            let action = cursor.previous().unwrap_or_default();
            let project = project_id(&cursor.next("Project ID")?)?;
            let source = source_id(&cursor.next("current-host user Source ID")?)?;
            let event = if action == "disable" {
                operations.disable_provider(
                    project,
                    privacy_intent(source, "disable background semantic provider"),
                )?
            } else {
                operations.revoke_provider(
                    project,
                    privacy_intent(source, "revoke background semantic provider"),
                )?
            };
            Ok(
                json!({"operation":format!("privacy_{action}"),"project_id":project.to_string(),"revision":event.revision,"state":debug_name(event.state)}),
            )
        }
        _ => Err(usage("privacy requires status, enable, disable, or revoke")),
    }
}

fn recall(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    let project = project_id(&cursor.next("Project ID")?)?;
    let brief = operations.recall(project)?;
    Ok(
        json!({"operation":"recall","project_id":brief.project_id.to_string(),"project_name":brief.project_name,"goals":brief.goals_and_why.into_iter().map(|item| item.statement).collect::<Vec<_>>(),"active_decision_count":brief.decisions.len(),"open_questions":brief.open_questions.into_iter().map(|question| json!({"identity":question.question_id.to_string(),"revision":question.revision,"prompt":question.prompt})).collect::<Vec<_>>(),"known_limits":brief.known_limits,"next_step":brief.next_meaningful_step,"omitted_count":brief.omitted_count,"used_sources":brief.used_sources.into_iter().map(|source| source.source.id.to_string()).collect::<Vec<_>>() }),
    )
}

fn documents(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    let action = cursor.next("documents command")?;
    let project = project_id(&cursor.next("Project ID")?)?;
    let kind = document_kind(&cursor.next("document kind")?)?;
    let format = output_format(&cursor.next("output format")?)?;
    let destination = if action == "export" {
        Some(absolute_path(&cursor.next("destination")?)?)
    } else if action == "preview" {
        None
    } else {
        return Err(usage("documents requires preview or export"));
    };
    let language = cursor.optional().unwrap_or_else(|| "en".into());
    let requested_destinations = destination
        .as_ref()
        .map(|path| RequestedDestination {
            document_kind: kind,
            output_format: format,
            path: path.display().to_string(),
        })
        .into_iter()
        .collect();
    let request = DocumentRequest {
        requested_language: language,
        fixed_locale: FixedLocale::English,
        generated_at: now()?,
        generator: GeneratorIdentity {
            generator: "volicord-local-operations".into(),
            agent: None,
            model: None,
        },
        requested_destinations,
    };
    let set = operations.documents(project, &request)?;
    let document = select_document(&set, kind);
    let artifact = if format == OutputFormat::Markdown {
        &document.markdown
    } else {
        &document.html
    };
    if let Some(path) = destination {
        let published = operations.publish_document(document, format, &path)?;
        Ok(
            json!({"operation":"document_export","project_id":project.to_string(),"kind":kind.slug(),"format":debug_name(format),"destination":published.destination,"bytes":published.bytes,"durability":published.durability,"canonical_mutation":false}),
        )
    } else {
        Ok(
            json!({"operation":"document_preview","project_id":project.to_string(),"kind":kind.slug(),"format":debug_name(format),"content":artifact.content,"canonical_mutation":false}),
        )
    }
}

fn inquiry(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    if cursor.next("inquiry command")? != "frontier" {
        return Err(usage(
            "inquiry currently supports: inquiry frontier PROJECT [SCOPE ...]",
        ));
    }
    let project = project_id(&cursor.next("Project ID")?)?;
    let frontier = operations.inquiry_frontier(project, cursor.remaining())?;
    Ok(
        json!({"operation":"inquiry_frontier","project_id":project.to_string(),"questions":frontier.questions.into_iter().map(|question| json!({"identity":question.question_id.to_string(),"revision":question.displayed_revision,"prompt":question.prompt_basis,"why_now":question.why_it_matters_now,"material_scope":question.material_scope,"what_unlocks":question.what_the_answer_unlocks})).collect::<Vec<_>>(),"diagnostics":frontier.diagnostics.into_iter().map(|diagnostic| json!({"kind":debug_name(diagnostic.kind),"question_id":diagnostic.question_id.to_string(),"detail":diagnostic.detail})).collect::<Vec<_>>() }),
    )
}

fn checkpoint(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    if cursor.next("checkpoint command")? != "record" {
        return Err(usage(
            "checkpoint currently supports: checkpoint record PROJECT KIND SOURCE GOAL NEXT_STEP",
        ));
    }
    let project = project_id(&cursor.next("Project ID")?)?;
    let kind = match cursor.next("checkpoint kind")?.as_str() {
        "completion" => CheckpointKind::Completion,
        "pause" => CheckpointKind::Pause,
        "handoff" => CheckpointKind::Handoff,
        _ => {
            return Err(usage(
                "checkpoint kind must be completion, pause, or handoff",
            ))
        }
    };
    let source = source_id(&cursor.next("grounding Source ID")?)?;
    let goal = cursor.next("goal")?;
    let next_step = cursor.next("next step")?;
    let project_revision = operations.canonical_basis(project)?.project.revision;
    let work_state = match kind {
        CheckpointKind::Completion => WorkState::Completed,
        CheckpointKind::Pause | CheckpointKind::Handoff => WorkState::Paused,
    };
    let result = operations.record_checkpoint(
        project,
        CheckpointDraft {
            expected_project_revision: project_revision,
            kind,
            goal,
            work_state,
            state_change: Some("explicit CLI checkpoint".into()),
            source_basis: vec![source],
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
            known_limits: Vec::new(),
            non_goals: Vec::new(),
            open_questions: Vec::new(),
            next_step,
            handoff_to: None,
        },
    )?;
    mutation_json("checkpoint_record", result)
}

fn guarded(operations: &LocalOperations, cursor: &mut Cursor) -> Result<Value, Error> {
    match cursor.next("guarded command")?.as_str() {
        "request" => {
            let project = project_id(&cursor.next("Project ID")?)?;
            let category = guarded_category(&cursor.next("risk category")?)?;
            let exact_action = cursor.next("exact action")?;
            let target = cursor.next("exact target")?;
            let expected_effect = cursor.next("expected effect")?;
            let concrete_consequence = cursor.next("concrete risk")?;
            let expires_at = number(&cursor.next("expiration Unix microseconds")?)?;
            let expires_at = i64::try_from(expires_at)
                .map(volicord_context::TimestampMicros::from_unix_micros)
                .map_err(|_| Error::new("expiration exceeds the supported timestamp range"))?;
            let scope = cursor.remaining();
            if scope.is_empty() {
                return Err(usage("guarded request requires at least one bounded scope"));
            }
            let candidate = operations.create_guarded_request(GuardedEffectDraft {
                project_id: project,
                exact_action,
                target,
                expected_effect,
                risk: GuardedRisk {
                    category,
                    concrete_consequence,
                },
                scope,
                expires_at,
                requesting_provenance: RequestingProvenance {
                    actor: Principal {
                        kind: PrincipalKind::Agent,
                        identity: "volicord-cli".into(),
                    },
                    host: Some("cli".into()),
                    session: Some("cli".into()),
                    basis: vec!["explicit CLI Guarded Effect Candidate".into()],
                },
            })?;
            Ok(guarded_request_json("guarded_request", &candidate))
        }
        "show" => {
            let request = confirmation_request_id(&cursor.next("confirmation request ID")?)?;
            let candidate = operations.guarded_request(request)?;
            Ok(guarded_request_json("guarded_show", &candidate))
        }
        "confirm" | "deny" => {
            let decision_text = cursor.previous().unwrap_or_default();
            let request = confirmation_request_id(&cursor.next("confirmation request ID")?)?;
            let revision = number(&cursor.next("request revision")?)?;
            let fingerprint = cursor.next("effect fingerprint")?;
            let host = cursor.next("current host")?;
            let session = cursor.next("current session")?;
            let turn = cursor.next("explicit user response")?;
            let decision = if decision_text == "confirm" {
                ConfirmationDecision::Confirmed
            } else {
                ConfirmationDecision::Denied
            };
            let response = operations.record_confirmation(
                request,
                revision,
                &fingerprint,
                decision,
                host,
                session,
                turn,
            )?;
            Ok(json!({
                "operation":format!("guarded_{decision_text}"),
                "confirmation_request_identity":response.confirmation_request_identity.to_string(),
                "request_revision":response.request_revision,
                "effect_fingerprint":response.effect_fingerprint,
                "decision":debug_name(response.decision),
                "user_response_source_id":response.user_response_source_id.to_string(),
                "confirmation_response_identity":response.confirmation_response_identity.to_string()
            }))
        }
        _ => Err(usage("guarded requires request, show, confirm, or deny")),
    }
}

fn guarded_request_json(operation: &str, candidate: &crate::GuardedEffectCandidate) -> Value {
    json!({
        "operation":operation,
        "confirmation_request_identity":candidate.confirmation_request_identity.to_string(),
        "request_revision":candidate.request_revision,
        "project_id":candidate.project_id.to_string(),
        "exact_action":candidate.exact_action,
        "target":candidate.target,
        "expected_effect":candidate.expected_effect,
        "risk_category":debug_name(candidate.risk.category),
        "risk_consequence":candidate.risk.concrete_consequence,
        "scope":candidate.scope,
        "expiration_unix_micros":candidate.expires_at.as_unix_micros(),
        "requesting_actor":format!("{:?}:{}", candidate.requesting_provenance.actor.kind, candidate.requesting_provenance.actor.identity),
        "requesting_provenance":candidate.requesting_provenance.basis,
        "effect_fingerprint":candidate.effect_fingerprint
    })
}

fn mutation_json(operation: &str, value: crate::CanonicalMutationOutcome) -> Result<Value, Error> {
    Ok(
        json!({"operation":operation,"record_kind":value.record_kind,"identity":value.identity,"revision":value.revision,"replayed":value.replayed}),
    )
}

fn privacy_intent(source: SourceId, basis: &str) -> ProviderIntentProvenance {
    ProviderIntentProvenance {
        actor: Principal {
            kind: PrincipalKind::User,
            identity: "current-host-user".into(),
        },
        host: "cli".into(),
        session: "cli".into(),
        user_turn_source: source,
        basis: basis.into(),
    }
}

fn project_id(value: &str) -> Result<ProjectId, Error> {
    Ok(ProjectId::from_bytes(parse_identity(value)?))
}
fn confirmation_request_id(value: &str) -> Result<ConfirmationRequestId, Error> {
    Ok(ConfirmationRequestId::from_bytes(parse_identity(value)?))
}
fn source_id(value: &str) -> Result<SourceId, Error> {
    Ok(SourceId::from_bytes(parse_identity(value)?))
}
fn number(value: &str) -> Result<u64, Error> {
    value
        .parse()
        .map_err(|error| Error::with_source("expected an unsigned integer", error))
}
fn absolute_path(value: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(Error::new("path must be absolute"))
    }
}
fn now() -> Result<volicord_context::TimestampMicros, Error> {
    use volicord_context::Clock;
    volicord_context::SystemClock
        .now()
        .map_err(|error| Error::with_source("system clock is unavailable", error))
}

fn document_kind(value: &str) -> Result<DocumentKind, Error> {
    match value {
        "project-architecture-guide" => Ok(DocumentKind::ProjectArchitectureGuide),
        "decision-report" => Ok(DocumentKind::DecisionReport),
        "implementation-plan" => Ok(DocumentKind::ImplementationPlan),
        "handoff-resume" => Ok(DocumentKind::HandoffResume),
        _ => Err(usage("unknown document kind")),
    }
}
fn output_format(value: &str) -> Result<OutputFormat, Error> {
    match value {
        "markdown" => Ok(OutputFormat::Markdown),
        "html" => Ok(OutputFormat::Html),
        _ => Err(usage("output format must be markdown or html")),
    }
}
fn guarded_category(value: &str) -> Result<GuardedEffectCategory, Error> {
    match value {
        "destructive-delete" => Ok(GuardedEffectCategory::DestructiveFileOrDataDeletion),
        "migration" => Ok(GuardedEffectCategory::IrreversibleOrLargeScaleMigration),
        "external-publication" => Ok(GuardedEffectCategory::ExternalDeploymentOrPublicPublication),
        "cost" => Ok(GuardedEffectCategory::PaymentOrContinuingCost),
        "credential" => Ok(GuardedEffectCategory::SecretOrCredentialAccessOrChange),
        "external-source-transmission" => {
            Ok(GuardedEffectCategory::PersonalDataOrSourceCodeExternalTransmission)
        }
        "external-message" => Ok(GuardedEffectCategory::ExternalMessageEmailOrIssue),
        "production-data" => Ok(GuardedEffectCategory::ProductionDataChange),
        "security-setting" => {
            Ok(GuardedEffectCategory::PermissionAuthenticationOrSecuritySettingChange)
        }
        _ => Err(usage("unknown Guarded risk category")),
    }
}
fn debug_name(value: impl std::fmt::Debug) -> String {
    format!("{value:?}").to_lowercase()
}

struct Cursor {
    args: Vec<OsString>,
    index: usize,
    previous: Option<String>,
}
impl Cursor {
    fn new(args: Vec<OsString>) -> Self {
        Self {
            args,
            index: 0,
            previous: None,
        }
    }
    fn next(&mut self, label: &str) -> Result<String, Error> {
        let value = self
            .args
            .get(self.index)
            .ok_or_else(|| usage(&format!("missing {label}")))?;
        let value = value
            .to_str()
            .ok_or_else(|| Error::new(format!("{label} must be valid UTF-8")))?
            .to_owned();
        self.index += 1;
        self.previous = Some(value.clone());
        Ok(value)
    }
    fn optional(&mut self) -> Option<String> {
        if self.index < self.args.len() {
            self.next("argument").ok()
        } else {
            None
        }
    }
    fn peek(&self, value: &str) -> bool {
        self.args
            .get(self.index)
            .is_some_and(|arg| arg == OsStr::new(value))
    }
    fn remaining(&mut self) -> Vec<String> {
        let mut values = Vec::new();
        while self.index < self.args.len() {
            if let Ok(value) = self.next("argument") {
                values.push(value);
            }
        }
        values
    }
    fn previous(&self) -> Option<String> {
        self.previous.clone()
    }
    fn done(&self) -> Result<(), Error> {
        if self.index == self.args.len() {
            Ok(())
        } else {
            Err(usage("unexpected trailing arguments"))
        }
    }
}
