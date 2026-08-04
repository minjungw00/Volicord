use std::{
    collections::BTreeSet,
    fmt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
use volicord_command_model::{InboxArgs, InboxCommand, InboxResolveArgs, StatusArgs};
use volicord_core::{CorePipelineError, CoreService, InvocationContext, PipelineResponse};
use volicord_store::{
    core_pipeline::{CoreProjectStore, StoredUserActionRecordSet},
    diagnostics::{start_diagnostic_session, DiagnosticSessionStart, DiagnosticTransport},
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    RuntimeHomeMutationContext, StoreError,
};
use volicord_types::ids::{
    ArtifactId, IdempotencyKey, ProjectId, RequestId, TaskId, UserActionRequestId,
};
use volicord_types::methods::{
    ResolveUserActionRequest, ResolveUserActionResponse, ResolveUserActionResult, StatusInclude,
    StatusRequest, StatusResponse, StatusResult,
};
use volicord_types::schema::{
    EvidenceTarget, PreviewableToolResponse, SummaryCard, ToolEnvelope, ToolResultOrRejected,
    UserActionResolutionBody, UserActionResolutionChoice, UserActionResolutionForm,
    UserActionResolutionInput, WorkflowProjection,
};
use volicord_types::values::{
    ArtifactAvailability, ArtifactIntegrityStatus, EvidenceRelevanceStatus, MethodName,
    OperationCategory, RedactionState, UserActionChannelKind, UserActionStatus,
};
use volicord_user_action_presentation::{
    cli_inbox_item, cli_user_channel_availability, CliUserActionInboxItem,
    CliUserActionInboxResponse, CliUserChannelAvailability,
};
use volicord_user_action_service::{
    PendingUserActionFacts, PendingUserActionFactsRequest, UserActionResolutionAvailability,
    UserActionResolutionUnavailableReason,
};

use crate::mutation_admission::{with_cli_runtime_home_mutation_result, CliMutationAdmissionError};
use crate::project_context::{
    registered_project_for_repo, registered_project_for_repo_admitted, resolve_repository_root,
    ProjectCommandError,
};
use crate::{
    presentation::{ActionHint, CollectionItem, Document, Field, HumanValue, Section},
    summary_card::{count_state_text, USER_CHANNEL_SUMMARY_GUARANTEE},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserCommandError {
    Usage(String),
    Runtime(String),
    MutationAdmission(CliMutationAdmissionError),
}

impl fmt::Display for UserCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
            Self::MutationAdmission(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for UserCommandError {}

impl From<CliMutationAdmissionError> for UserCommandError {
    fn from(error: CliMutationAdmissionError) -> Self {
        Self::MutationAdmission(error)
    }
}

impl From<StoreError> for UserCommandError {
    fn from(error: StoreError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for UserCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<CorePipelineError> for UserCommandError {
    fn from(error: CorePipelineError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<ProjectCommandError> for UserCommandError {
    fn from(error: ProjectCommandError) -> Self {
        match error {
            ProjectCommandError::Usage(message) => Self::Usage(message),
            ProjectCommandError::Runtime(message) => Self::Runtime(message),
            ProjectCommandError::MutationAdmission(error) => Self::MutationAdmission(error),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone, Default)]
struct ParsedStatusOptions {
    repo: Option<PathBuf>,
    task: TaskSelector,
    output: OutputFormat,
}

#[derive(Debug, Clone)]
struct ParsedInboxOptions {
    repo: Option<PathBuf>,
    task: TaskSelector,
    choice: Option<String>,
    note: Option<String>,
    acceptance_criterion_id: Option<String>,
    evidence_claim_id: Option<String>,
    artifact_ids: Vec<String>,
    summary: Option<String>,
    relevance_status: EvidenceRelevanceStatus,
    output: OutputFormat,
}

impl Default for ParsedInboxOptions {
    fn default() -> Self {
        Self {
            repo: None,
            task: TaskSelector::Active,
            choice: None,
            note: None,
            acceptance_criterion_id: None,
            evidence_claim_id: None,
            artifact_ids: Vec::new(),
            summary: None,
            relevance_status: EvidenceRelevanceStatus::Supported,
            output: OutputFormat::Text,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum TaskSelector {
    #[default]
    Active,
    Id(String),
}

#[derive(Debug, Clone)]
struct ResolvedUserProject {
    runtime_home: PathBuf,
    project_id: String,
}

pub(crate) struct UserActionResolutionRecordingInput<'a> {
    pub project_id: &'a str,
    pub record: &'a StoredUserActionRecordSet,
    pub resolution: UserActionResolutionInput,
    pub request_id: Option<String>,
    pub channel_submission_id: Option<String>,
    pub session_id: Option<&'a str>,
}

pub fn run_status_command<F>(
    args: StatusArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    command_status(args, env_var, current_dir)
}

pub fn run_inbox_command<F>(
    args: InboxArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    match args.command {
        Some(InboxCommand::Resolve(options)) => {
            command_inbox_resolve(options, env_var, current_dir)
        }
        None => command_inbox_list(args, env_var, current_dir),
    }
}

fn command_status<F>(
    args: StatusArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let parsed = ParsedStatusOptions {
        repo: args.repo.map(|path| absolute_path(current_dir, path)),
        task: parse_task_selector(Some(args.task))?,
        output: if args.json {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
    };
    let resolved = resolve_user_project(parsed.repo.as_deref(), env_var, current_dir)?;
    let store = CoreProjectStore::open_read_only(
        &resolved.runtime_home,
        &ProjectId::new(&resolved.project_id),
    )?;
    let task_id = selected_or_active_task_id(&store, &parsed.task)?;
    let (response, typed_response) = status_response(&resolved, task_id.as_deref())?;
    render_status_response(&response, &typed_response, parsed.output)
}

fn command_inbox_list<F>(
    args: InboxArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let parsed = ParsedInboxOptions {
        repo: args.repo.map(|path| absolute_path(current_dir, path)),
        task: parse_task_selector(Some(args.task))?,
        output: if args.json {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
        ..ParsedInboxOptions::default()
    };
    let resolved = resolve_user_project(parsed.repo.as_deref(), env_var, current_dir)?;
    let store = CoreProjectStore::open_read_only(
        &resolved.runtime_home,
        &ProjectId::new(&resolved.project_id),
    )?;
    let task_id = selected_or_active_task_id(&store, &parsed.task)?;
    let facts = task_id
        .as_deref()
        .map(|task_id| {
            pending_user_action_facts(&resolved.runtime_home, &resolved.project_id, task_id, None)
        })
        .transpose()?
        .flatten();
    render_inbox_response(facts.as_ref(), parsed.output, task_id.is_some())
}

fn status_response(
    resolved: &ResolvedUserProject,
    task_id: Option<&str>,
) -> Result<(PipelineResponse, StatusResponse), UserCommandError> {
    let response = CoreService::for_read_only(&resolved.runtime_home)
        .status(
            StatusRequest {
                envelope: envelope(
                    &resolved.project_id,
                    task_id,
                    generated_id("req_user_status"),
                    None,
                ),
                continuity_page: None,
                include: StatusInclude {
                    task: true,
                    pending_user_actions: true,
                    write_ticket: true,
                    evidence: true,
                    close: true,
                    guarantees: true,
                    continuity: true,
                },
            },
            invocation(&resolved.project_id, OperationCategory::Read),
        )
        .map_err(UserCommandError::from)?;
    let typed_response = serde_json::from_value::<StatusResponse>(response.response_value.clone())
        .map_err(|error| UserCommandError::Runtime(error.to_string()))?;
    Ok((response, typed_response))
}

fn pending_user_action_facts(
    runtime_home: &Path,
    project_id: &str,
    task_id: &str,
    session_id: Option<&str>,
) -> Result<Option<PendingUserActionFacts>, UserCommandError> {
    let invocation = InvocationContext::local_user(
        ProjectId::new(project_id),
        OperationCategory::Read,
        UserActionChannelKind::Cli,
    );
    let invocation = match session_id {
        Some(session_id) => invocation.with_session_id(session_id),
        None => invocation,
    };
    CoreService::for_read_only(runtime_home)
        .pending_user_action_facts(
            PendingUserActionFactsRequest {
                project_id: ProjectId::new(project_id),
                task_id: TaskId::new(task_id),
            },
            invocation,
        )
        .map_err(Into::into)
}

fn command_inbox_resolve<F>(
    args: InboxResolveArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let request_id = args.user_action_request_id.clone();
    let parsed = ParsedInboxOptions {
        repo: args.repo.map(|path| absolute_path(current_dir, path)),
        task: TaskSelector::Active,
        choice: args.choice,
        note: args.note,
        acceptance_criterion_id: args.criterion,
        evidence_claim_id: args.claim,
        artifact_ids: args.artifact,
        summary: args.summary,
        relevance_status: if args.contradicted {
            EvidenceRelevanceStatus::Contradicted
        } else {
            EvidenceRelevanceStatus::Supported
        },
        output: if args.json {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
    };
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let repo_root = resolve_repository_root(current_dir, parsed.repo.as_deref())?;
    with_cli_runtime_home_mutation_result(&runtime_home, "cli.inbox.resolve", |context| {
        command_inbox_resolve_admitted(context, &repo_root, &request_id, &parsed)
    })
    .map_err(UserCommandError::from)?
}

fn command_inbox_resolve_admitted(
    context: &RuntimeHomeMutationContext<'_>,
    repo_root: &Path,
    request_id: &str,
    parsed: &ParsedInboxOptions,
) -> Result<String, UserCommandError> {
    let project = registered_project_for_repo_admitted(context, repo_root)?;
    if project.repo_root != repo_root {
        return Err(UserCommandError::Runtime(
            "registered project does not match the requested repository".to_owned(),
        ));
    }
    let project_id = ProjectId::new(&project.project_internal_id);
    let store = CoreProjectStore::open_for_mutation(context, &project_id)?;
    if store.runtime_home() != context.runtime_home().as_path()
        || store.project_record().project_internal_id != project.project_internal_id
        || store.project_record().repo_root != repo_root
    {
        return Err(UserCommandError::Runtime(
            "admitted project does not match the requested Runtime Home and repository".to_owned(),
        ));
    }

    let service = CoreService::for_mutation(context);
    let snapshot = service
        .pending_user_action_resolution_snapshot_from_store(
            &store,
            &UserActionRequestId::new(request_id),
            invocation(&project.project_internal_id, OperationCategory::Read),
        )?
        .ok_or_else(|| {
            UserCommandError::Runtime("selected user action was not found".to_owned())
        })?;
    let resolution = match snapshot.resolution_availability {
        UserActionResolutionAvailability::Available => {
            let pending_actions = snapshot.pending_actions.as_ref().ok_or_else(|| {
                UserCommandError::Runtime(
                    "selected user action is no longer in the canonical pending inbox; refresh `volicord inbox`"
                        .to_owned(),
                )
            })?;
            let action = pending_actions
                .actions
                .iter()
                .find(|item| {
                    item.request.user_action_request_id.as_str() == request_id
                })
                .ok_or_else(|| {
                    UserCommandError::Runtime(
                        "selected user action is no longer in the canonical pending inbox; refresh `volicord inbox`"
                        .to_owned(),
                    )
                })?;
            let form = action.request.body.resolution_form().map_err(|error| {
                UserCommandError::Runtime(format!("invalid pending user-action facts: {error}"))
            })?;
            resolution_from_form(&form, parsed)?
        }
        UserActionResolutionAvailability::Unavailable(
            UserActionResolutionUnavailableReason::AlreadyResolved,
        ) if snapshot.record.resolution().is_some() => {
            resolution_from_immutable_request(&snapshot.record, parsed)?
        }
        UserActionResolutionAvailability::Unavailable(reason) => {
            return Err(cli_resolution_unavailable_error(reason));
        }
    };
    let (stable_request_id, channel_submission_id) = stable_cli_resolution_ids(request_id);
    let diagnostic_session_id = generated_id("diag_cli_inbox");
    drop(store);

    let build = volicord_mcp::build_info();
    let _ = start_diagnostic_session(
        context,
        DiagnosticSessionStart {
            session_id: &diagnostic_session_id,
            connection_id: None,
            project_id: Some(&project.project_internal_id),
            transport: DiagnosticTransport::CliInbox,
            host_kind: None,
            package_version: build.package_version,
            build_id: &build.build_id,
        },
    );
    let response = resolve_user_action_from_record(
        context,
        UserActionResolutionRecordingInput {
            project_id: &project.project_internal_id,
            record: &snapshot.record,
            resolution,
            request_id: Some(stable_request_id),
            channel_submission_id: Some(channel_submission_id),
            session_id: Some(&diagnostic_session_id),
        },
    )?;
    render_resolve_response(&response, parsed.output)
}

fn cli_resolution_unavailable_error(
    reason: UserActionResolutionUnavailableReason,
) -> UserCommandError {
    let status = match reason {
        UserActionResolutionUnavailableReason::AlreadyResolved => UserActionStatus::Resolved,
        UserActionResolutionUnavailableReason::Stale => UserActionStatus::Stale,
        UserActionResolutionUnavailableReason::Superseded => UserActionStatus::Superseded,
        UserActionResolutionUnavailableReason::Expired => UserActionStatus::Expired,
    };
    UserCommandError::Runtime(format!(
        "selected user action is not pending (status: {}); refresh `volicord inbox`",
        status.as_str()
    ))
}

fn resolution_from_immutable_request(
    record: &StoredUserActionRecordSet,
    parsed: &ParsedInboxOptions,
) -> Result<UserActionResolutionInput, UserCommandError> {
    let request = record.request().request();
    let form = request.body.resolution_form().map_err(|error| {
        UserCommandError::Runtime(format!(
            "invalid immutable user-action request for replay: {error}"
        ))
    })?;
    resolution_from_form(&form, parsed)
}

fn resolution_from_form(
    form: &UserActionResolutionForm,
    parsed: &ParsedInboxOptions,
) -> Result<UserActionResolutionInput, UserCommandError> {
    match form {
        UserActionResolutionForm::Choice { choices, .. } => {
            reject_observation_flags(parsed)?;
            let selector = parsed
                .choice
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| UserCommandError::Usage("a choice answer is required".to_owned()))?;
            let selected = select_inbox_choice(choices, selector)?;
            Ok(UserActionResolutionInput::Choice {
                selected_option_id: selected.choice_id,
                note: parsed.note.clone().into(),
            })
        }
        UserActionResolutionForm::EvidenceObservation {
            target_candidates,
            artifact_candidates,
            ..
        } => {
            if parsed.choice.is_some() || parsed.note.is_some() {
                return Err(UserCommandError::Usage(
                    "choice and note arguments are valid only for a choice user action".to_owned(),
                ));
            }
            if parsed.acceptance_criterion_id.is_some() == parsed.evidence_claim_id.is_some() {
                return Err(UserCommandError::Usage(
                    "exactly one evidence target is required".to_owned(),
                ));
            }
            if parsed.artifact_ids.is_empty() {
                return Err(UserCommandError::Usage(
                    "at least one non-empty artifact identifier is required".to_owned(),
                ));
            }
            let summary = parsed
                .summary
                .as_ref()
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .ok_or_else(|| {
                    UserCommandError::Usage(
                        "a non-empty observation summary is required".to_owned(),
                    )
                })?;
            let target = selected_target(parsed, target_candidates)?;
            validate_artifact_selection(&parsed.artifact_ids, artifact_candidates)?;
            Ok(UserActionResolutionInput::EvidenceObservation {
                target,
                artifact_ids: parsed.artifact_ids.iter().map(ArtifactId::new).collect(),
                relevance_status: parsed.relevance_status,
                summary,
            })
        }
    }
}

pub(crate) fn select_inbox_choice(
    choices: &[UserActionResolutionChoice],
    selector: &str,
) -> Result<UserActionResolutionChoice, UserCommandError> {
    if let Some(index) = parse_positive_index(selector)? {
        let choice = choices.get(index - 1).cloned().ok_or_else(|| {
            UserCommandError::Usage(format!(
                "option number {index} is out of range for the selected user action"
            ))
        })?;
        if let Some(by_id) = choices
            .iter()
            .find(|candidate| candidate.choice_id.as_str() == selector)
        {
            if by_id.choice_id != choice.choice_id {
                return Err(UserCommandError::Usage(format!(
                    "option selector `{selector}` is ambiguous: it matches an option number and an explicit option id"
                )));
            }
        }
        return Ok(choice);
    }
    choices
        .iter()
        .find(|choice| choice.choice_id.as_str() == selector)
        .cloned()
        .ok_or_else(|| {
            UserCommandError::Usage(format!(
                "option selector `{selector}` does not match a numbered option or option id"
            ))
        })
}

fn selected_target(
    parsed: &ParsedInboxOptions,
    candidates: &[EvidenceTarget],
) -> Result<EvidenceTarget, UserCommandError> {
    if let Some(criterion_id) = parsed.acceptance_criterion_id.as_deref() {
        return candidates
            .iter()
            .find(|target| {
                matches!(
                    target,
                    EvidenceTarget::AcceptanceCriterion { acceptance_criterion_id }
                        if acceptance_criterion_id.as_str() == criterion_id
                )
            })
            .cloned()
            .ok_or_else(|| {
                UserCommandError::Usage(format!(
                    "criterion `{criterion_id}` is not a candidate for the selected user action"
                ))
            });
    }
    let claim_id = parsed
        .evidence_claim_id
        .as_deref()
        .expect("caller checked exactly one target selector");
    candidates
        .iter()
        .find(|target| {
            matches!(
                target,
                EvidenceTarget::SupplementalClaim { evidence_claim_id, .. }
                    if evidence_claim_id.as_str() == claim_id
            )
        })
        .cloned()
        .ok_or_else(|| {
            UserCommandError::Usage(format!(
                "claim `{claim_id}` is not a candidate for the selected user action"
            ))
        })
}

fn validate_artifact_selection(
    artifact_ids: &[String],
    candidates: &[volicord_types::schema::ArtifactRef],
) -> Result<(), UserCommandError> {
    let mut seen = BTreeSet::new();
    for artifact_id in artifact_ids {
        if artifact_id.trim().is_empty() {
            return Err(UserCommandError::Usage(
                "artifact identifiers must not be empty".to_owned(),
            ));
        }
        if !seen.insert(artifact_id) {
            return Err(UserCommandError::Usage(format!(
                "artifact `{artifact_id}` was selected more than once"
            )));
        }
        if !candidates
            .iter()
            .any(|candidate| candidate.artifact_id.as_str() == artifact_id)
        {
            return Err(UserCommandError::Usage(format!(
                "artifact `{artifact_id}` is not a candidate for the selected user action"
            )));
        }
    }
    Ok(())
}

fn reject_observation_flags(parsed: &ParsedInboxOptions) -> Result<(), UserCommandError> {
    if parsed.acceptance_criterion_id.is_some()
        || parsed.evidence_claim_id.is_some()
        || !parsed.artifact_ids.is_empty()
        || parsed.summary.is_some()
        || parsed.relevance_status == EvidenceRelevanceStatus::Contradicted
    {
        return Err(UserCommandError::Usage(
            "observation flags are valid only for an evidence-observation user action".to_owned(),
        ));
    }
    Ok(())
}

fn parse_task_selector(value: Option<String>) -> Result<TaskSelector, UserCommandError> {
    match value.as_deref() {
        None | Some("active") => Ok(TaskSelector::Active),
        Some(value) if value.trim().is_empty() => Err(UserCommandError::Usage(
            "the task selector must not be empty".to_owned(),
        )),
        Some(value) => Ok(TaskSelector::Id(value.to_owned())),
    }
}

fn resolve_user_project<F>(
    repo: Option<&Path>,
    env_var: F,
    current_dir: &Path,
) -> Result<ResolvedUserProject, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let repo_root = resolve_repository_root(current_dir, repo)?;
    let project = registered_project_for_repo(&runtime_home, &repo_root)?;
    Ok(ResolvedUserProject {
        runtime_home,
        project_id: project.project_internal_id,
    })
}

fn selected_or_active_task_id(
    store: &CoreProjectStore,
    selected: &TaskSelector,
) -> Result<Option<String>, UserCommandError> {
    match selected {
        TaskSelector::Active => Ok(store.project_state()?.active_task_id),
        TaskSelector::Id(task_id) => Ok(Some(task_id.clone())),
    }
}

fn parse_positive_index(selector: &str) -> Result<Option<usize>, UserCommandError> {
    if selector.is_empty() || !selector.chars().all(|ch| ch.is_ascii_digit()) {
        return Ok(None);
    }
    let index = selector.parse::<usize>().map_err(|_| {
        UserCommandError::Usage(format!("selector `{selector}` is not a valid list number"))
    })?;
    if index == 0 {
        return Err(UserCommandError::Usage(
            "list numbers start at 1".to_owned(),
        ));
    }
    Ok(Some(index))
}

pub(crate) fn resolve_user_action_from_record(
    context: &RuntimeHomeMutationContext<'_>,
    input: UserActionResolutionRecordingInput<'_>,
) -> Result<PipelineResponse, UserCommandError> {
    if input.record.status() != UserActionStatus::Pending && input.channel_submission_id.is_none() {
        return Err(UserCommandError::Runtime(format!(
            "selected user action is not pending (status: {}); refresh `volicord inbox`",
            input.record.status().as_str()
        )));
    }
    let request_id = input
        .request_id
        .unwrap_or_else(|| generated_id("req_user_action_resolve"));
    let channel_submission_id = input
        .channel_submission_id
        .unwrap_or_else(|| generated_id("submission_user_action"));
    let invocation = invocation(input.project_id, OperationCategory::UserOnly);
    let invocation = match input.session_id {
        Some(session_id) => invocation.with_session_id(session_id),
        None => invocation,
    };
    let response = CoreService::for_mutation(context)
        .resolve_user_action(
            context,
            ResolveUserActionRequest {
                envelope: envelope(
                    input.project_id,
                    Some(input.record.request().task_id()),
                    request_id,
                    Some(channel_submission_id.clone()),
                ),
                user_action_request_id: UserActionRequestId::new(
                    input.record.request().user_action_request_id(),
                ),
                channel_submission_id,
                resolution: input.resolution,
            },
            invocation,
        )
        .map_err(UserCommandError::from)?;
    serde_json::from_value::<ResolveUserActionResponse>(response.response_value.clone())
        .map_err(|error| UserCommandError::Runtime(error.to_string()))?;
    Ok(response)
}

fn envelope(
    project_id: &str,
    task_id: Option<&str>,
    request_id: String,
    idempotency_key: Option<String>,
) -> ToolEnvelope {
    ToolEnvelope {
        project_id: ProjectId::new(project_id),
        task_id: task_id.map(TaskId::new).into(),
        request_id: RequestId::new(request_id),
        idempotency_key: idempotency_key.map(IdempotencyKey::new).into(),
        expected_state_version: None.into(),
        dry_run: volicord_types::schema::DryRunIntent::NotRequested,
        locale: None.into(),
    }
}

fn invocation(project_id: &str, operation_category: OperationCategory) -> InvocationContext {
    InvocationContext::local_user(
        ProjectId::new(project_id),
        operation_category,
        UserActionChannelKind::Cli,
    )
}

fn render_status_response(
    response: &PipelineResponse,
    typed_response: &StatusResponse,
    output: OutputFormat,
) -> Result<String, UserCommandError> {
    if output == OutputFormat::Json {
        return pretty_response(response);
    }
    match typed_response {
        ToolResultOrRejected::Result(result) => Ok(render_status_result(result)),
        ToolResultOrRejected::Rejected(_) => render_rejected_or_json(response),
    }
}

fn render_status_result(result: &StatusResult) -> String {
    let profile = Field::new("Profile", HumanValue::text(&result.summary_card.profile)).into();
    let pending_count = result.pending_user_action_summaries.len();
    let changes = Field::new(
        "Unrecorded changes",
        HumanValue::text(&result.summary_card.changes),
    )
    .into();
    let next = ActionHint::new(&result.summary_card.next).into();

    let Some(task) = &result.active_task else {
        return Document::new(
            "No active Task.",
            vec![
                profile,
                Field::new("Pending user actions", pending_count_value(pending_count)).into(),
                changes,
                next,
            ],
        )
        .render();
    };

    let mut body = vec![profile];
    let mut task_fields = Vec::new();
    if let Some(lifecycle) = &task.lifecycle {
        task_fields.push(Field::new(
            "Lifecycle",
            HumanValue::text(lifecycle.lifecycle_phase.as_str()),
        ));
    }
    if let Some(work_phase) = task.work_phase {
        let work_phase = match work_phase {
            volicord_types::values::WorkPhase::Shaping => "shaping",
            volicord_types::values::WorkPhase::Implementation => "implementation",
        };
        task_fields.push(Field::new("Work phase", HumanValue::text(work_phase)));
    }
    if let Some(goal) = task.goal_summary.as_deref() {
        task_fields.push(Field::new("Goal", HumanValue::text(goal)));
    }
    if task_fields.is_empty() {
        task_fields.push(Field::new(
            "State",
            HumanValue::text(&result.summary_card.task),
        ));
    }
    body.push(CollectionItem::new("Task", task_fields).into());

    body.push(
        Section::new(
            "Write Ticket",
            vec![Field::new(
                "Status",
                HumanValue::text(&result.summary_card.write_ticket),
            )
            .into()],
        )
        .into(),
    );

    if result.evidence_summary.is_some() || result.evidence_gate.is_some() {
        let mut evidence = Vec::new();
        if let Some(summary) = result
            .evidence_summary
            .as_ref()
            .and_then(|summary| summary.as_ref())
        {
            let status = match summary.status {
                volicord_types::values::EvidenceStatus::Unknown => "unknown",
                volicord_types::values::EvidenceStatus::Insufficient => "insufficient",
                volicord_types::values::EvidenceStatus::Sufficient => "sufficient",
                volicord_types::values::EvidenceStatus::Blocked => "blocked",
            };
            evidence.push(Field::new("Status", HumanValue::text(status)).into());
            evidence.push(
                Field::new(
                    "Coverage items",
                    HumanValue::Count(summary.coverage_items.len()),
                )
                .into(),
            );
        } else {
            evidence.push(Field::new("Status", HumanValue::None).into());
        }
        if let Some(gate) = result.evidence_gate.as_ref().and_then(|gate| gate.as_ref()) {
            evidence.push(Field::new("Gate", HumanValue::text(gate.state.as_str())).into());
        }
        body.push(Section::new("Evidence", evidence).into());
    }

    body.push(
        Section::new(
            "Pending UserActions",
            vec![Field::new("Count", pending_count_value(pending_count)).into()],
        )
        .into(),
    );
    body.push(changes);

    if result.close_state.is_some() || result.close_blockers.is_some() {
        let mut close = Vec::new();
        if let Some(state) = result.close_state {
            close.push(Field::new("State", HumanValue::text(state.as_str())).into());
        }
        if let Some(blockers) = &result.close_blockers {
            close.push(Field::new("Blockers", HumanValue::Count(blockers.len())).into());
        }
        body.push(Section::new("Close readiness", close).into());
    }
    body.push(next);
    Document::new("Current Task status", body).render()
}

fn pending_count_value(count: usize) -> HumanValue {
    if count == 0 {
        HumanValue::None
    } else {
        HumanValue::Count(count)
    }
}

fn render_inbox_response(
    facts: Option<&PendingUserActionFacts>,
    output: OutputFormat,
    has_selected_task: bool,
) -> Result<String, UserCommandError> {
    let items = facts
        .map(|facts| {
            facts
                .actions
                .iter()
                .map(|action| {
                    cli_inbox_item(action.request_ref.clone(), action.request.clone())
                        .map_err(|error| UserCommandError::Runtime(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let availability = facts.map(|_| cli_user_channel_availability());
    let summary_card = inbox_summary_card(&items, has_selected_task);
    let response = CliUserActionInboxResponse {
        summary_card,
        user_channel_availability: availability.into(),
        pending_user_action_inbox_items: items,
    };
    render_cli_inbox_response(&response, output)
}

fn render_cli_inbox_response(
    response: &CliUserActionInboxResponse,
    output: OutputFormat,
) -> Result<String, UserCommandError> {
    if output == OutputFormat::Json {
        return serde_json::to_string_pretty(response)
            .map(|text| format!("{text}\n"))
            .map_err(|error| UserCommandError::Runtime(error.to_string()));
    }
    if response.pending_user_action_inbox_items.is_empty() {
        return Ok(Document::new(
            "No pending user actions.",
            vec![
                Field::new("Task", HumanValue::text(&response.summary_card.task)).into(),
                ActionHint::new("none").into(),
            ],
        )
        .render());
    }
    let mut text = Document::new(
        format!(
            "Pending user actions ({})",
            response.pending_user_action_inbox_items.len()
        ),
        vec![
            Field::new("Task", HumanValue::text(&response.summary_card.task)).into(),
            Field::new("Channel", HumanValue::text("User Channel")).into(),
        ],
    )
    .render();
    text.push('\n');
    if let Some(line) =
        render_user_channel_availability_text(response.user_channel_availability.as_ref())
    {
        text.push_str(&line);
    }
    for (index, item) in response.pending_user_action_inbox_items.iter().enumerate() {
        text.push_str(&format!("{}. {}\n", index + 1, item.question));
        text.push_str(&format!("   id: {}\n", item.user_action_request_id));
        text.push_str(&format!("   kind: {}\n", item.action_kind.as_str()));
        if !item.context_summary.trim().is_empty() {
            text.push_str(&format!("   context: {}\n", item.context_summary));
        }
        match &item.resolution_form {
            UserActionResolutionForm::Choice {
                choices,
                note_allowed,
                note_max_chars,
            } => {
                text.push_str("   choices:\n");
                for choice in choices {
                    text.push_str(&format!(
                        "   - {}: {} - {}\n     consequence: {}\n     default: {}\n",
                        choice.choice_id,
                        choice.label,
                        choice.description,
                        choice.consequence,
                        choice.is_default
                    ));
                }
                text.push_str(&format!(
                    "   note allowed: {note_allowed}; max characters: {note_max_chars}\n"
                ));
                append_resolution_command(&mut text, item);
            }
            UserActionResolutionForm::EvidenceObservation {
                target_candidates,
                artifact_candidates,
                relevance_options,
                summary_max_chars,
            } => {
                text.push_str("   target candidates:\n");
                for target in target_candidates {
                    match target {
                        EvidenceTarget::AcceptanceCriterion {
                            acceptance_criterion_id,
                        } => text.push_str(&format!(
                            "   - Acceptance criterion {acceptance_criterion_id}\n     target kind: acceptance_criterion\n"
                        )),
                        EvidenceTarget::SupplementalClaim {
                            evidence_claim_id,
                            statement,
                        } => text.push_str(&format!(
                            "   - Supplemental claim {evidence_claim_id}: {statement}\n     target kind: supplemental_claim\n"
                        )),
                    }
                }
                text.push_str("   artifact candidates:\n");
                for artifact in artifact_candidates {
                    text.push_str(&format!(
                        concat!(
                            "   - {}: {}\n",
                            "     project: {}\n",
                            "     task: {}\n",
                            "     content type: {}\n",
                            "     sha256: {}\n",
                            "     size bytes: {}\n",
                            "     integrity: {}\n",
                            "     redaction: {}\n",
                            "     availability: {}\n",
                            "     created by run: {}\n",
                            "     created by actor: {}\n",
                            "     storage ref: {}\n",
                        ),
                        artifact.artifact_id,
                        artifact.display_name,
                        artifact.project_id,
                        artifact.task_id,
                        artifact
                            .content_type
                            .as_ref()
                            .map_or("none", String::as_str),
                        artifact.sha256.as_ref().map_or("none", String::as_str),
                        artifact
                            .size_bytes
                            .as_ref()
                            .map_or_else(|| "none".to_owned(), u64::to_string),
                        artifact_integrity_text(artifact.integrity_status),
                        redaction_state_text(artifact.redaction_state),
                        artifact_availability_text(artifact.availability),
                        artifact
                            .created_by_run_ref
                            .as_ref()
                            .map_or("none", |record| record.record_id.as_str()),
                        artifact
                            .created_by_actor_source
                            .as_ref()
                            .map_or_else(|| "none".to_owned(), ToString::to_string),
                        artifact
                            .storage_ref
                            .as_ref()
                            .map_or("none", |storage_ref| storage_ref.as_str()),
                    ));
                }
                text.push_str(&format!(
                    "   relevance options: {}\n   summary max characters: {}\n",
                    relevance_options
                        .iter()
                        .map(|status| status.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    summary_max_chars
                ));
                append_resolution_command(&mut text, item);
            }
        }
    }
    Ok(text)
}

const fn artifact_integrity_text(status: ArtifactIntegrityStatus) -> &'static str {
    match status {
        ArtifactIntegrityStatus::Verified => "verified",
        ArtifactIntegrityStatus::Corrupt => "corrupt",
    }
}

const fn redaction_state_text(state: RedactionState) -> &'static str {
    match state {
        RedactionState::None => "none",
        RedactionState::Redacted => "redacted",
        RedactionState::SecretOmitted => "secret_omitted",
        RedactionState::Blocked => "blocked",
    }
}

const fn artifact_availability_text(availability: ArtifactAvailability) -> &'static str {
    match availability {
        ArtifactAvailability::Available => "available",
        ArtifactAvailability::Unavailable => "unavailable",
        ArtifactAvailability::Missing => "missing",
        ArtifactAvailability::IntegrityFailed => "integrity_failed",
        ArtifactAvailability::Blocked => "blocked",
        ArtifactAvailability::Unusable => "unusable",
    }
}

fn inbox_summary_card(items: &[CliUserActionInboxItem], has_selected_task: bool) -> SummaryCard {
    SummaryCard {
        task: if has_selected_task {
            "selected"
        } else {
            "none"
        }
        .to_owned(),
        recording: "read_only".to_owned(),
        profile: "not_selected".to_owned(),
        write_ticket: "not_selected".to_owned(),
        evidence: "not_selected".to_owned(),
        user_action: count_state_text("pending", items.len()),
        changes: "not_selected".to_owned(),
        close_status: "not_selected".to_owned(),
        transport: "User Channel".to_owned(),
        next: items
            .first()
            .map(|item| {
                format!(
                    "resolve pending user action {}",
                    item.user_action_request_id
                )
            })
            .unwrap_or_else(|| "none".to_owned()),
        next_action: None,
        guarantee: USER_CHANNEL_SUMMARY_GUARANTEE.to_owned(),
    }
}

fn append_resolution_command(text: &mut String, item: &CliUserActionInboxItem) {
    if let Some(command) = item.capture_path.command() {
        text.push_str(&format!("   resolve:\n     {command}\n"));
    }
}

fn render_resolve_response(
    response: &PipelineResponse,
    output: OutputFormat,
) -> Result<String, UserCommandError> {
    if output == OutputFormat::Json {
        return pretty_response(response);
    }
    let typed_response =
        serde_json::from_value::<ResolveUserActionResponse>(response.response_value.clone())
            .map_err(|error| UserCommandError::Runtime(error.to_string()))?;
    match typed_response {
        PreviewableToolResponse::Result(result) => Ok(render_resolve_result(&result)),
        PreviewableToolResponse::Rejected(_) | PreviewableToolResponse::DryRun(_) => {
            render_rejected_or_json(response)
        }
    }
}

fn render_resolve_result(result: &ResolveUserActionResult) -> String {
    let (resolution_line, authority_effect) = match &result.user_action_resolution.body {
        UserActionResolutionBody::Choice {
            resolution_outcome, ..
        } => {
            let outcome = match resolution_outcome {
                volicord_types::values::JudgmentResolutionOutcome::Accepted => "accepted",
                volicord_types::values::JudgmentResolutionOutcome::Rejected => "rejected",
                volicord_types::values::JudgmentResolutionOutcome::Deferred => "deferred",
            };
            let authority_effect = if outcome == "accepted" {
                "accepted outcome recorded; current semantic owner determines applicability"
            } else {
                "none"
            };
            (format!("Resolution outcome: {outcome}"), authority_effect)
        }
        UserActionResolutionBody::EvidenceObservation { .. } => (
            "Resolution type: evidence_observation".to_owned(),
            "not applicable",
        ),
    };
    let workflow = &result.state.workflow;
    let required_action = workflow
        .required_action()
        .map(|method| method.as_str())
        .unwrap_or("none");
    format!(
        "User action resolution recorded\nRequest status: {}\n{}\nAuthority effect: {}\nShaping application: none (`volicord.resolve_user_action` does not apply shaping decisions)\nWorkflow: {}\nNext actor: {}\nRequired action: {}\n",
        result.user_action_request.status.as_str(),
        resolution_line,
        authority_effect,
        workflow_kind(workflow),
        workflow.next_actor().as_str(),
        required_action,
    )
}

fn workflow_kind(workflow: &WorkflowProjection) -> &'static str {
    match workflow {
        WorkflowProjection::NoActiveTask { .. } => "no_active_task",
        WorkflowProjection::ShapingRequired { .. } => "shaping_required",
        WorkflowProjection::AwaitingUserAction { .. } => "awaiting_user_action",
        WorkflowProjection::DecisionRecoveryRequired { .. } => "decision_recovery_required",
        WorkflowProjection::ReadyToApplyDecisions { .. } => "ready_to_apply_decisions",
        WorkflowProjection::ReadyForChangeUnit { .. } => "ready_for_change_unit",
        WorkflowProjection::ReadyToFinalizeAdvice { .. } => "ready_to_finalize_advice",
        WorkflowProjection::ReadyForImplementation { .. } => "ready_for_implementation",
        WorkflowProjection::Implementation { .. } => "implementation",
        WorkflowProjection::CloseReview { .. } => "close_review",
        WorkflowProjection::Terminal { .. } => "terminal",
    }
}

fn render_user_channel_availability_text(
    availability: Option<&CliUserChannelAvailability>,
) -> Option<String> {
    let path = availability?
        .paths
        .iter()
        .find(|path| path.kind() == UserActionChannelKind::Cli)?;
    Some(format!(
        "CLI inbox {}\n",
        if path.is_available() {
            "available"
        } else {
            "unavailable"
        }
    ))
}

fn pretty_response(response: &PipelineResponse) -> Result<String, UserCommandError> {
    serde_json::to_string_pretty(&response.response_value)
        .map(|text| format!("{text}\n"))
        .map_err(|error| UserCommandError::Runtime(error.to_string()))
}

fn render_rejected_or_json(response: &PipelineResponse) -> Result<String, UserCommandError> {
    if let Some(errors) = response.response_value["errors"].as_array() {
        let mut output = String::from("Core request rejected\n");
        for error in errors {
            output.push_str(&format!(
                "{}: {}\n",
                error["code"].as_str().unwrap_or("ERROR"),
                error["message"].as_str().unwrap_or("request rejected")
            ));
        }
        Ok(output)
    } else {
        pretty_response(response)
    }
}

fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

fn generated_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .expect("system time must be at or after the Unix epoch");
    format!("{prefix}_{nanos}")
}

fn stable_cli_resolution_ids(user_action_request_id: &str) -> (String, String) {
    let mut hasher = Sha256::new();
    for part in [
        UserActionChannelKind::Cli.verification_basis().as_str(),
        MethodName::ResolveUserAction.as_str(),
        user_action_request_id,
    ] {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = format!("{:x}", hasher.finalize());
    let short = &digest[..24];
    (
        format!("req_cli_user_action_{short}"),
        format!("submission_cli_user_action_{short}"),
    )
}

#[cfg(test)]
mod tests {
    use std::{error::Error, ffi::OsString, fs};

    use serde_json::Value;
    use volicord_store::diagnostics::diagnostics_db_path;
    use volicord_test_support::{
        core_fixtures::{
            artifact_input_for_handle, CoreFixture, ManagedPlanningFixture,
            ObservationUserActionFixture, UpdateScopeFixture, UserActionFixture,
        },
        seed_test_agent_session, TestRuntimeHomeSetup,
    };
    use volicord_types::ids::{
        AgentConnectionId, BaselineRef, ChangeUnitId, EvidenceClaimId, ShapingCheckpointId,
    };
    use volicord_types::methods::{AdvanceTaskRequest, RecordShapingCheckpointRequest};
    use volicord_types::schema::{
        ChangeUnitEffectContract, RequiredNullable, ShapingCheckpointOperation, ShapingGapInput,
        ShapingUserActionDraft, StagedArtifactHandle,
    };
    use volicord_types::values::{
        ChangeUnitEffectKind, ChangeUnitOperation, JudgmentKind, RequestedMode, ShapingGapKind,
    };

    use super::*;

    struct PendingChoiceFixture {
        fixture: CoreFixture,
        task_id: String,
        request_id: String,
    }

    struct PendingObservationFixture {
        fixture: CoreFixture,
        request_id: String,
        claim_id: String,
        artifact_id: String,
    }

    struct PendingShapingChoiceFixture {
        fixture: ManagedPlanningFixture,
        task_id: String,
        request_id: String,
    }

    fn pending_shaping_choice_fixture(
        prefix: &str,
        mode: RequestedMode,
        judgment_kind: JudgmentKind,
    ) -> Result<PendingShapingChoiceFixture, Box<dyn Error>> {
        let fixture = ManagedPlanningFixture::new(prefix, "2026-06-18T00:00:00Z")?;
        let core_fixture = fixture.core();
        let core = CoreService::for_mutation(&core_fixture.mutation_context()?);
        let validated = core.validate_agent_session(
            AgentConnectionId::new(core_fixture.connection_id()),
            ProjectId::new(core_fixture.project_id()),
            fixture.session().runtime_session_id.clone(),
            fixture.session().project_session_id.clone(),
            OperationCategory::AgentWorkflow,
        )?;
        let invocation =
            InvocationContext::agent_connection(OperationCategory::AgentWorkflow, validated);
        let mut intake_request = core_fixture.intake_request(
            &format!("req_{prefix}_intake"),
            &format!("idem_{prefix}_intake"),
            false,
            Some(0),
        );
        intake_request.requested_mode = mode;
        intake_request.plain_language_request =
            "Prepare one bounded change from the planning documents.".to_owned();
        let intake = core.intake(
            &core_fixture.mutation_context()?,
            intake_request,
            invocation.clone(),
        )?;
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("planning intake task")
            .to_owned();
        let mut scope_request = core_fixture.update_scope_request(UpdateScopeFixture {
            request_id: &format!("req_{prefix}_scope"),
            idempotency_key: &format!("idem_{prefix}_scope"),
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Keep the planning implementation inside one neutral path.",
        });
        if mode == RequestedMode::Advisor {
            scope_request
                .change_unit
                .fields
                .insert("affected_paths".to_owned(), serde_json::json!([]));
            scope_request.change_unit.effect_contract = Some(ChangeUnitEffectContract {
                allowed_effects: vec![
                    ChangeUnitEffectKind::ArtifactRegistration,
                    ChangeUnitEffectKind::UserActionRequest,
                    ChangeUnitEffectKind::EvidenceUpdate,
                ],
                forbidden_effects: vec![
                    ChangeUnitEffectKind::ProductFileWrite,
                    ChangeUnitEffectKind::RunRecording,
                    ChangeUnitEffectKind::SensitiveAction,
                    ChangeUnitEffectKind::ExternalNetwork,
                    ChangeUnitEffectKind::SecretAccess,
                ],
                allowed_paths: Vec::new(),
                expected_outputs: vec!["Advice result".to_owned()],
                invariants: vec!["Observe only".to_owned()],
                evidence_expectations: Vec::new(),
                sensitive_action_expectations: Vec::new(),
            });
        }
        let scoped = core.update_scope(
            &core_fixture.mutation_context()?,
            scope_request,
            invocation.clone(),
        )?;
        let change_unit_id = scoped.response_value["change_unit_ref"]["record_id"]
            .as_str()
            .unwrap_or_else(|| panic!("planning Change Unit: {}", scoped.response_value))
            .to_owned();
        let gap_kind = match judgment_kind {
            JudgmentKind::ProductDecision => ShapingGapKind::UserProductDecisionRequired,
            JudgmentKind::TechnicalDecision => ShapingGapKind::UserTechnicalDecisionRequired,
            JudgmentKind::ScopeDecision => ShapingGapKind::UserScopeDecisionRequired,
            JudgmentKind::SensitiveApproval => ShapingGapKind::SensitiveApprovalRequired,
            other => panic!("unsupported shaping fixture judgment kind: {other:?}"),
        };
        let action = core_fixture
            .user_action_request(UserActionFixture {
                request_id: "unused",
                idempotency_key: "unused",
                dry_run: false,
                expected_state_version: Some(2),
                task_id: &task_id,
                change_unit_id: Some(&change_unit_id),
                judgment_kind,
            })
            .action;
        let shaped = core.record_shaping_checkpoint(
            &core_fixture.mutation_context()?,
            RecordShapingCheckpointRequest {
                envelope: core_fixture.envelope(
                    &format!("req_{prefix}_shaping"),
                    Some(&format!("idem_{prefix}_shaping")),
                    false,
                    Some(2),
                    Some(&task_id),
                ),
                task_id: TaskId::new(&task_id),
                checkpoint_operation: ShapingCheckpointOperation::CreateInitial,
                scope_revision: 1,
                baseline_ref: RequiredNullable::some(BaselineRef::new(
                    volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                )),
                summary: "The plan has one exact user-owned shaping boundary.".to_owned(),
                implementation_boundary: RequiredNullable::some(
                    "Apply only the exact resolved decision before implementation.".to_owned(),
                ),
                gaps: vec![ShapingGapInput {
                    gap_kind,
                    summary: "The user owns this exact shaping decision.".to_owned(),
                    affected_refs: Vec::new(),
                    user_action: RequiredNullable::some(ShapingUserActionDraft {
                        action,
                        expires_at: RequiredNullable::null(),
                    }),
                }],
                source_refs: Vec::new(),
                evidence_refs: Vec::new(),
            },
            invocation,
        )?;
        let request_id = shaped.response_value["created_user_action_request_refs"][0]["record_id"]
            .as_str()
            .expect("record_shaping_checkpoint UserAction request")
            .to_owned();
        Ok(PendingShapingChoiceFixture {
            fixture,
            task_id,
            request_id,
        })
    }

    fn resolve_shaping_choice(
        fixture: &PendingShapingChoiceFixture,
        choice: &str,
    ) -> Result<Value, Box<dyn Error>> {
        let core_fixture = fixture.fixture.core();
        let output = command_inbox_resolve(
            InboxResolveArgs {
                user_action_request_id: fixture.request_id.clone(),
                choice: Some(choice.to_owned()),
                note: None,
                criterion: None,
                claim: None,
                artifact: Vec::new(),
                summary: None,
                contradicted: false,
                repo: Some(core_fixture.product_repo_path()),
                json: true,
            },
            |name| {
                (name == "VOLICORD_HOME").then(|| OsString::from(core_fixture.runtime_home_path()))
            },
            &core_fixture.product_repo_path(),
        )?;
        Ok(serde_json::from_str(&output)?)
    }

    fn resolve_shaping_choice_text(
        fixture: &PendingShapingChoiceFixture,
        choice: &str,
    ) -> Result<String, UserCommandError> {
        let core_fixture = fixture.fixture.core();
        command_inbox_resolve(
            InboxResolveArgs {
                user_action_request_id: fixture.request_id.clone(),
                choice: Some(choice.to_owned()),
                note: None,
                criterion: None,
                claim: None,
                artifact: Vec::new(),
                summary: None,
                contradicted: false,
                repo: Some(core_fixture.product_repo_path()),
                json: false,
            },
            |name| {
                (name == "VOLICORD_HOME").then(|| OsString::from(core_fixture.runtime_home_path()))
            },
            &core_fixture.product_repo_path(),
        )
    }

    fn pending_choice_fixture(prefix: &str) -> Result<PendingChoiceFixture, Box<dyn Error>> {
        pending_choice_fixture_for_kind(prefix, JudgmentKind::ProductDecision)
    }

    fn pending_choice_fixture_for_kind(
        prefix: &str,
        judgment_kind: JudgmentKind,
    ) -> Result<PendingChoiceFixture, Box<dyn Error>> {
        let fixture = CoreFixture::new(prefix)?;
        fs::create_dir_all(fixture.product_repo_path().join(".git"))?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let session = seed_test_agent_session(
            fixture.runtime_home_path(),
            fixture.project_id(),
            fixture.connection_id(),
            None,
        )?;
        let validated = core.validate_agent_session(
            AgentConnectionId::new(fixture.connection_id()),
            ProjectId::new(fixture.project_id()),
            session.runtime_session_id,
            session.project_session_id,
            OperationCategory::AgentWorkflow,
        )?;
        let invocation =
            InvocationContext::agent_connection(OperationCategory::AgentWorkflow, validated);
        let intake = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_cli_inbox_intake",
                "idem_cli_inbox_intake",
                false,
                Some(0),
            ),
            invocation.clone(),
        )?;
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("intake should identify its task")
            .to_owned();
        let state_version = intake.response_value["base"]["state_version"]
            .as_u64()
            .expect("intake should expose its committed state version");
        let requested = core.request_user_action(
            &fixture.mutation_context()?,
            fixture.user_action_request(UserActionFixture {
                request_id: "req_cli_inbox_user_action",
                idempotency_key: "idem_cli_inbox_user_action",
                dry_run: false,
                expected_state_version: Some(state_version),
                task_id: &task_id,
                change_unit_id: None,
                judgment_kind,
            }),
            invocation,
        )?;
        let request_id = requested.response_value["user_action_request_summary"]
            ["user_action_request_id"]
            .as_str()
            .unwrap_or_else(|| {
                panic!(
                    "request result should identify the pending user action: {}",
                    requested.response_value
                )
            })
            .to_owned();
        Ok(PendingChoiceFixture {
            fixture,
            task_id,
            request_id,
        })
    }

    fn pending_observation_fixture(
        prefix: &str,
    ) -> Result<PendingObservationFixture, Box<dyn Error>> {
        let fixture = CoreFixture::new(prefix)?;
        fs::create_dir_all(fixture.product_repo_path().join(".git"))?;
        let core = CoreService::for_mutation(&fixture.mutation_context()?);
        let session = seed_test_agent_session(
            fixture.runtime_home_path(),
            fixture.project_id(),
            fixture.connection_id(),
            None,
        )?;
        let validated = core.validate_agent_session(
            AgentConnectionId::new(fixture.connection_id()),
            ProjectId::new(fixture.project_id()),
            session.runtime_session_id,
            session.project_session_id,
            OperationCategory::AgentWorkflow,
        )?;
        let invocation =
            InvocationContext::agent_connection(OperationCategory::AgentWorkflow, validated);
        let intake = core.intake(
            &fixture.mutation_context()?,
            fixture.intake_request(
                "req_cli_observation_intake",
                "idem_cli_observation_intake",
                false,
                Some(0),
            ),
            invocation.clone(),
        )?;
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("intake should identify its task")
            .to_owned();
        let intake_state_version = intake.response_value["base"]["state_version"]
            .as_u64()
            .expect("intake should expose its committed state version");
        let scope = core.update_scope(
            &fixture.mutation_context()?,
            fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_cli_observation_scope",
                idempotency_key: "idem_cli_observation_scope",
                dry_run: false,
                expected_state_version: Some(intake_state_version),
                task_id: &task_id,
                operation: ChangeUnitOperation::CreateCurrent,
                scope_summary: "Exercise CLI evidence-observation resolution.",
            }),
            invocation.clone(),
        )?;
        let change_unit_id = scope.response_value["change_unit_ref"]["record_id"]
            .as_str()
            .expect("scope update should identify its Change Unit")
            .to_owned();
        let scope_state_version = scope.response_value["base"]["state_version"]
            .as_u64()
            .expect("scope update should expose its committed state version");
        let shaped = core.record_shaping_checkpoint(
            &fixture.mutation_context()?,
            RecordShapingCheckpointRequest {
                envelope: fixture.envelope(
                    "req_cli_observation_shaping",
                    Some("idem_cli_observation_shaping"),
                    false,
                    Some(scope_state_version),
                    Some(&task_id),
                ),
                task_id: TaskId::new(&task_id),
                checkpoint_operation:
                    volicord_types::schema::ShapingCheckpointOperation::CreateInitial,
                scope_revision: 1,
                baseline_ref: RequiredNullable::some(BaselineRef::new(
                    volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                )),
                summary: "The CLI evidence-observation boundary is ready.".to_owned(),
                implementation_boundary: RequiredNullable::some(
                    "Register only the scoped observation artifact.".to_owned(),
                ),
                gaps: Vec::new(),
                source_refs: Vec::new(),
                evidence_refs: Vec::new(),
            },
            invocation.clone(),
        )?;
        let shaped_state_version = shaped.response_value["base"]["state_version"]
            .as_u64()
            .expect("record_shaping_checkpoint should expose its committed state version");
        let checkpoint_id = shaped.response_value["shaping_checkpoint"]["shaping_checkpoint_id"]
            .as_str()
            .expect("record_shaping_checkpoint should identify its checkpoint");
        let advanced = core.advance_task(
            &fixture.mutation_context()?,
            AdvanceTaskRequest {
                envelope: fixture.envelope(
                    "req_cli_observation_advance",
                    Some("idem_cli_observation_advance"),
                    false,
                    Some(shaped_state_version),
                    Some(&task_id),
                ),
                task_id: TaskId::new(&task_id),
                shaping_checkpoint_id: ShapingCheckpointId::new(checkpoint_id),
                change_unit_id: ChangeUnitId::new(&change_unit_id),
                scope_revision: 1,
                baseline_ref: BaselineRef::new(
                    volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                ),
                user_action_resolution_ids: Vec::new(),
            },
            invocation.clone(),
        )?;
        let advanced_state_version = advanced.response_value["base"]["state_version"]
            .as_u64()
            .expect("advance_task should expose its committed state version");
        let staged = core.stage_artifact(
            &fixture.mutation_context()?,
            fixture.stage_artifact_request(
                "req_cli_observation_stage",
                Some("idem_cli_observation_stage"),
                false,
                Some(advanced_state_version),
                &task_id,
            ),
            invocation.clone(),
        )?;
        let handle: StagedArtifactHandle =
            serde_json::from_value(staged.response_value["staged_artifact_handle"].clone())?;
        let staged_state_version = staged.response_value["base"]["state_version"]
            .as_u64()
            .expect("staging should expose its committed state version");
        let claim_statement = "Classify the registered artifact.";
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut claim_id = String::from("claim_");
        claim_id.reserve(claim_statement.len() * 2);
        for byte in claim_statement.as_bytes() {
            claim_id.push(HEX[usize::from(byte >> 4)] as char);
            claim_id.push(HEX[usize::from(byte & 0x0f)] as char);
        }
        let mut record_run = fixture.record_run_request(
            "req_cli_observation_run",
            "idem_cli_observation_run",
            false,
            Some(staged_state_version),
            &task_id,
            &change_unit_id,
        );
        record_run.artifact_inputs = vec![artifact_input_for_handle(
            "artifact_input_cli_observation",
            handle,
            Some("user_action_candidate"),
            Some(claim_statement),
        )];
        let recorded =
            core.record_run(&fixture.mutation_context()?, record_run, invocation.clone())?;
        let artifact_id = recorded.response_value["registered_artifacts"][0]["artifact_id"]
            .as_str()
            .expect("record_run should register the staged artifact")
            .to_owned();
        let recorded_state_version = recorded.response_value["base"]["state_version"]
            .as_u64()
            .expect("record_run should expose its committed state version");
        let requested = core.request_user_action(
            &fixture.mutation_context()?,
            fixture.observation_user_action_request(ObservationUserActionFixture {
                request_id: "req_cli_observation_user_action",
                idempotency_key: "idem_cli_observation_user_action",
                dry_run: false,
                expected_state_version: Some(recorded_state_version),
                task_id: &task_id,
                change_unit_id: &change_unit_id,
                target_candidates: vec![EvidenceTarget::SupplementalClaim {
                    evidence_claim_id: EvidenceClaimId::new(&claim_id),
                    statement: claim_statement.to_owned(),
                }],
                artifact_candidate_ids: vec![ArtifactId::new(&artifact_id)],
            }),
            invocation,
        )?;
        let request_id = requested.response_value["user_action_request_summary"]
            ["user_action_request_id"]
            .as_str()
            .unwrap_or_else(|| {
                panic!(
                    "request result should identify the pending user action: {}",
                    requested.response_value
                )
            })
            .to_owned();
        Ok(PendingObservationFixture {
            fixture,
            request_id,
            claim_id,
            artifact_id,
        })
    }

    fn choice_args(fixture: &PendingChoiceFixture, json: bool, choice: &str) -> InboxResolveArgs {
        InboxResolveArgs {
            user_action_request_id: fixture.request_id.clone(),
            choice: Some(choice.to_owned()),
            note: None,
            criterion: None,
            claim: None,
            artifact: Vec::new(),
            summary: None,
            contradicted: false,
            repo: Some(fixture.fixture.product_repo_path()),
            json,
        }
    }

    fn resolve_choice(
        fixture: &PendingChoiceFixture,
        json: bool,
        choice: &str,
    ) -> Result<String, UserCommandError> {
        resolve_choice_at_runtime_home(fixture, fixture.fixture.runtime_home_path(), json, choice)
    }

    fn resolve_choice_at_runtime_home(
        fixture: &PendingChoiceFixture,
        runtime_home: &Path,
        json: bool,
        choice: &str,
    ) -> Result<String, UserCommandError> {
        command_inbox_resolve(
            choice_args(fixture, json, choice),
            |name| (name == "VOLICORD_HOME").then(|| OsString::from(runtime_home)),
            &fixture.fixture.product_repo_path(),
        )
    }

    fn resolve_observation(
        fixture: &PendingObservationFixture,
        claim_id: &str,
        artifact_id: &str,
    ) -> Result<String, UserCommandError> {
        resolve_observation_at_runtime_home(
            fixture,
            fixture.fixture.runtime_home_path(),
            claim_id,
            artifact_id,
        )
    }

    fn resolve_observation_at_runtime_home(
        fixture: &PendingObservationFixture,
        runtime_home: &Path,
        claim_id: &str,
        artifact_id: &str,
    ) -> Result<String, UserCommandError> {
        command_inbox_resolve(
            InboxResolveArgs {
                user_action_request_id: fixture.request_id.clone(),
                choice: None,
                note: None,
                criterion: None,
                claim: Some(claim_id.to_owned()),
                artifact: vec![artifact_id.to_owned()],
                summary: Some("The artifact supports the requested observation.".to_owned()),
                contradicted: false,
                repo: Some(fixture.fixture.product_repo_path()),
                json: true,
            },
            |name| (name == "VOLICORD_HOME").then(|| OsString::from(runtime_home)),
            &fixture.fixture.product_repo_path(),
        )
    }

    #[test]
    fn selector_rejects_zero() {
        let error = select_inbox_choice(&[], "0").expect_err("zero is not a list number");
        assert!(error.to_string().contains("start at 1"));
    }

    #[test]
    fn cli_retry_identity_is_stable_and_resolution_independent() {
        let first = stable_cli_resolution_ids("ua_1");
        let retry = stable_cli_resolution_ids("ua_1");
        let other_request = stable_cli_resolution_ids("ua_2");

        assert_eq!(first, retry);
        assert_ne!(first, other_request);
        assert_ne!(first.0, first.1);
        assert!(!first.0.contains("Approved locally"));
    }

    #[test]
    fn neutral_resolution_unavailability_becomes_a_cli_refresh_error() {
        let cases = [
            (
                UserActionResolutionUnavailableReason::AlreadyResolved,
                "resolved",
            ),
            (UserActionResolutionUnavailableReason::Stale, "stale"),
            (
                UserActionResolutionUnavailableReason::Superseded,
                "superseded",
            ),
            (UserActionResolutionUnavailableReason::Expired, "expired"),
        ];

        for (reason, expected_status) in cases {
            let error = cli_resolution_unavailable_error(reason);
            assert!(matches!(error, UserCommandError::Runtime(_)));
            assert_eq!(
                error.to_string(),
                format!(
                    "selected user action is not pending (status: {expected_status}); refresh `volicord inbox`"
                )
            );
        }
    }

    #[test]
    fn active_status_renders_only_applicable_typed_sections_and_counts(
    ) -> Result<(), Box<dyn Error>> {
        let pending = pending_choice_fixture("cli-status-contextual")?;
        let store = CoreProjectStore::open_read_only(
            pending.fixture.runtime_home_path(),
            &ProjectId::new(pending.fixture.project_id()),
        )?;
        let task_id = store
            .project_state()?
            .active_task_id
            .expect("fixture should have an active task");
        let resolved = ResolvedUserProject {
            runtime_home: pending.fixture.runtime_home_path().to_path_buf(),
            project_id: pending.fixture.project_id().to_owned(),
        };
        let (_, typed) = status_response(&resolved, Some(&task_id))?;
        let mut result = match typed {
            ToolResultOrRejected::Result(result) => result,
            ToolResultOrRejected::Rejected(rejection) => {
                panic!("status should succeed: {rejection:?}")
            }
        };

        assert_eq!(result.pending_user_action_summaries.len(), 1);
        let text = render_status_result(&result);
        for section in [
            "Current Task status",
            "Task",
            "Write Ticket",
            "Evidence",
            "Pending UserActions",
            "Unrecorded changes",
            "Close readiness",
            "Next action",
        ] {
            assert!(text.contains(section), "missing {section}: {text}");
        }
        assert!(text.contains("Count: 1"), "{text}");
        assert!(!text.contains("not shown in this view"), "{text}");
        assert!(!text.contains("pending (0)"), "{text}");

        result.pending_user_action_summaries.clear();
        result.close_state = None;
        result.close_blockers = None;
        let without_close = render_status_result(&result);
        assert!(without_close.contains("Pending UserActions\n  Count: none"));
        assert!(!without_close.contains("Close readiness"));
        assert!(!without_close.contains("blockers (total)"));
        assert!(without_close.ends_with('\n'));
        assert!(!without_close.ends_with("\n\n"));
        assert!(!without_close.contains('\t'));
        Ok(())
    }

    #[test]
    fn inbox_rendering_uses_the_command_model_resolution_invocation() -> Result<(), Box<dyn Error>>
    {
        let pending = pending_choice_fixture("cli-inbox-command-model")?;
        let store = CoreProjectStore::open_read_only(
            pending.fixture.runtime_home_path(),
            &ProjectId::new(pending.fixture.project_id()),
        )?;
        let task_id = store
            .project_state()?
            .active_task_id
            .expect("fixture should have an active task");
        let facts = pending_user_action_facts(
            pending.fixture.runtime_home_path(),
            pending.fixture.project_id(),
            &task_id,
            None,
        )?
        .expect("fixture should have pending user-action facts");
        let json_output = render_inbox_response(Some(&facts), OutputFormat::Json, true)?;
        let typed: CliUserActionInboxResponse = serde_json::from_str(&json_output)?;
        assert_eq!(typed.pending_user_action_inbox_items.len(), 1);
        assert!(typed.pending_user_action_inbox_items[0].is_required());
        let rendered = render_cli_inbox_response(&typed, OutputFormat::Text)?;
        let directly_rendered = render_inbox_response(Some(&facts), OutputFormat::Text, true)?;
        assert_eq!(directly_rendered, rendered);
        let expected = volicord_command_model::InboxResolveInvocation::new(
            &pending.request_id,
            volicord_command_model::InboxResolutionArguments::Choice {
                choice: "<choice>".to_owned(),
                note: None,
            },
        )
        .canonical_arguments()?
        .join(" ");

        assert!(rendered.contains(&expected));
        Ok(())
    }

    #[test]
    fn inbox_choice_resolution_and_replay_use_the_admitted_lexical_runtime_home(
    ) -> Result<(), Box<dyn Error>> {
        let pending = pending_choice_fixture("cli-inbox-choice-lexical-alias")?;
        let runtime_home = pending.fixture.runtime_home_path();
        let alias = runtime_home
            .parent()
            .expect("fixture Runtime Home has a parent")
            .join(".")
            .join(
                runtime_home
                    .file_name()
                    .expect("fixture Runtime Home has a file name"),
            );
        let before = pending.fixture.counts()?;

        let first = resolve_choice_at_runtime_home(&pending, &alias, true, "accept")?;
        let after_first = pending.fixture.counts()?;
        let replay = resolve_choice_at_runtime_home(&pending, &alias, true, "accept")?;
        let after_replay = pending.fixture.counts()?;

        assert_eq!(replay, first);
        assert_eq!(after_replay, after_first);
        assert_eq!(after_first.state_version, before.state_version + 1);
        assert_eq!(
            pending.fixture.user_action_status(&pending.request_id)?,
            "resolved"
        );
        let diagnostic = volicord_store::diagnostics::read_diagnostic_session(
            pending.fixture.runtime_home_path(),
            None,
        )?
        .expect("alias resolution should write its diagnostic session to the admitted home");
        assert_eq!(diagnostic.transport, "cli_inbox");
        assert_eq!(
            diagnostic.project_id.as_deref(),
            Some(pending.fixture.project_id())
        );
        Ok(())
    }

    #[test]
    fn inbox_rejected_choice_surfaces_exact_outcome_and_cannot_be_resolved_again(
    ) -> Result<(), Box<dyn Error>> {
        let pending = pending_choice_fixture_for_kind(
            "cli-inbox-rejected-choice",
            JudgmentKind::ScopeDecision,
        )?;
        let before = pending.fixture.counts()?;

        let output = resolve_choice(&pending, true, "reject")?;
        let response: Value = serde_json::from_str(&output)?;
        assert_eq!(
            response["user_action_resolution"]["body"]["machine_action"],
            "reject"
        );
        assert_eq!(
            response["user_action_resolution"]["body"]["resolution_outcome"],
            "rejected"
        );
        assert_eq!(
            pending
                .fixture
                .user_action_resolution_outcome(&pending.request_id)?,
            Some("rejected".to_owned())
        );
        let after = pending.fixture.counts()?;
        assert_eq!(after.state_version, before.state_version + 1);

        let conflicting = resolve_choice(&pending, true, "defer")?;
        let conflicting: Value = serde_json::from_str(&conflicting)?;
        assert_eq!(conflicting["base"]["response_kind"], "rejected");
        assert_eq!(conflicting["base"]["effect_kind"], "no_effect");
        assert_eq!(conflicting["errors"][0]["code"], "STATE_VERSION_CONFLICT");
        assert_eq!(pending.fixture.counts()?, after);
        Ok(())
    }

    #[test]
    fn inbox_rejected_choice_text_surfaces_no_authority_and_current_workflow(
    ) -> Result<(), Box<dyn Error>> {
        let pending = pending_choice_fixture_for_kind(
            "cli-inbox-rejected-choice-text",
            JudgmentKind::ScopeDecision,
        )?;

        let output = resolve_choice(&pending, false, "reject")?;

        assert!(output.contains("Request status: resolved"));
        assert!(output.contains("Resolution outcome: rejected"));
        assert!(output.contains("Authority effect: none"));
        assert!(output.contains("Shaping application: none"));
        assert!(
            output.contains("Workflow: shaping_required"),
            "unexpected CLI resolution output: {output}"
        );
        assert!(output.contains("Next actor: agent"));
        assert!(output.contains("Required action: volicord.record_shaping_checkpoint"));
        Ok(())
    }

    #[test]
    fn shaping_rejection_text_surfaces_no_authority_and_exact_recovery_owner(
    ) -> Result<(), Box<dyn Error>> {
        let pending = pending_shaping_choice_fixture(
            "cli_shaping_scope_rejected_text",
            RequestedMode::Work,
            JudgmentKind::ScopeDecision,
        )?;

        let output = resolve_shaping_choice_text(&pending, "reject")?;

        assert!(output.contains("Request status: resolved"));
        assert!(output.contains("Resolution outcome: rejected"));
        assert!(output.contains("Authority effect: none"));
        assert!(output.contains("Shaping application: none"));
        assert!(output.contains("Workflow: decision_recovery_required"));
        assert!(output.contains("Next actor: agent"));
        assert!(output.contains("Required action: volicord.record_shaping_checkpoint"));
        Ok(())
    }

    #[test]
    fn shaping_cli_outcome_matrix_matches_immediate_status_and_exact_owner(
    ) -> Result<(), Box<dyn Error>> {
        for mode in [RequestedMode::Work, RequestedMode::Advisor] {
            for judgment_kind in [
                JudgmentKind::ProductDecision,
                JudgmentKind::TechnicalDecision,
                JudgmentKind::ScopeDecision,
                JudgmentKind::SensitiveApproval,
            ] {
                let label = format!("cli_{mode:?}_{judgment_kind:?}_accepted").to_lowercase();
                let pending = pending_shaping_choice_fixture(&label, mode, judgment_kind)?;
                let before = pending.fixture.core().counts()?;
                assert_eq!(
                    pending
                        .fixture
                        .core()
                        .user_action_status(&pending.request_id)?,
                    "pending"
                );

                let response = resolve_shaping_choice(&pending, "accept")?;
                let resolved_project = ResolvedUserProject {
                    runtime_home: pending.fixture.core().runtime_home_path().to_path_buf(),
                    project_id: pending.fixture.core().project_id().to_owned(),
                };
                let (status, _) = status_response(&resolved_project, Some(&pending.task_id))?;
                let returned_workflow = &response["state"]["workflow"];
                let status_workflow = &status.response_value["active_task"]["workflow"];
                for field in ["kind", "next_actor", "required_action", "checkpoint"] {
                    assert_eq!(
                        returned_workflow[field], status_workflow[field],
                        "{label}: {field}"
                    );
                }
                assert_eq!(
                    response["user_action_resolution"]["body"]["resolution_outcome"], "accepted",
                    "{label}"
                );
                assert_eq!(
                    status_workflow["checkpoint"]["gaps"][0]["status"],
                    "accepted"
                );
                let expected_owner = match (mode, judgment_kind) {
                    (_, JudgmentKind::ScopeDecision) => "volicord.update_scope",
                    (RequestedMode::Advisor, _) => "volicord.finalize_advice",
                    _ => "volicord.advance_task",
                };
                assert_eq!(
                    status_workflow["required_action"], expected_owner,
                    "{label}"
                );
                assert_eq!(
                    status_workflow["checkpoint"]["current_application_refs"],
                    serde_json::json!([]),
                    "accepted authority is not applied by resolution alone: {label}"
                );
                let after = pending.fixture.core().counts()?;
                assert_eq!(after.state_version, before.state_version + 1, "{label}");
                assert_eq!(after.write_tickets, 0, "{label}");
                assert_eq!(
                    pending.fixture.core().conn()?.query_row::<u64, _, _>(
                        "SELECT COUNT(*) FROM shaping_decision_applications",
                        [],
                        |row| row.get(0),
                    )?,
                    0,
                    "resolution alone creates no application: {label}"
                );
                assert!(
                    pending.fixture.repository().status_bytes()?.is_empty(),
                    "{label}"
                );
            }
        }

        for mode in [RequestedMode::Work, RequestedMode::Advisor] {
            for judgment_kind in [JudgmentKind::ScopeDecision, JudgmentKind::SensitiveApproval] {
                for (choice, outcome) in [("reject", "rejected"), ("defer", "deferred")] {
                    let label = format!("cli_{mode:?}_{judgment_kind:?}_{outcome}").to_lowercase();
                    let pending = pending_shaping_choice_fixture(&label, mode, judgment_kind)?;
                    let before = pending.fixture.core().counts()?;
                    let response = resolve_shaping_choice(&pending, choice)?;
                    let resolved_project = ResolvedUserProject {
                        runtime_home: pending.fixture.core().runtime_home_path().to_path_buf(),
                        project_id: pending.fixture.core().project_id().to_owned(),
                    };
                    let (status, _) = status_response(&resolved_project, Some(&pending.task_id))?;
                    let returned_workflow = &response["state"]["workflow"];
                    let status_workflow = &status.response_value["active_task"]["workflow"];
                    assert_eq!(
                        response["user_action_resolution"]["body"]["resolution_outcome"], outcome,
                        "{label}"
                    );
                    for field in ["kind", "next_actor", "required_action", "checkpoint"] {
                        assert_eq!(
                            returned_workflow[field], status_workflow[field],
                            "{label}: {field}"
                        );
                    }
                    assert_eq!(status_workflow["kind"], "decision_recovery_required");
                    assert_eq!(status_workflow["next_actor"], "agent");
                    assert_eq!(
                        status_workflow["required_action"],
                        "volicord.record_shaping_checkpoint"
                    );
                    assert_eq!(
                        status_workflow["checkpoint"]["decision_recovery_requirements"][0]
                            ["disposition"],
                        outcome
                    );
                    assert_eq!(
                        status_workflow["checkpoint"]["current_application_refs"],
                        serde_json::json!([]),
                        "{label} grants no authority"
                    );
                    assert_ne!(
                        status.response_value["active_task"]["lifecycle"]["lifecycle_phase"],
                        "waiting_user",
                        "{label} must not remain waiting_user"
                    );
                    let after = pending.fixture.core().counts()?;
                    assert_eq!(after.state_version, before.state_version + 1, "{label}");
                    assert_eq!(after.write_tickets, 0, "{label}");
                    assert_eq!(
                        status.response_value["active_task"]["work_phase"], "shaping",
                        "{label}"
                    );
                    let conn = pending.fixture.core().conn()?;
                    assert_eq!(
                        conn.query_row::<u64, _, _>(
                            "SELECT COUNT(*) FROM shaping_decision_applications",
                            [],
                            |row| row.get(0),
                        )?,
                        0,
                        "{label} grants no application authority"
                    );
                    assert_eq!(
                        conn.query_row::<u64, _, _>(
                            "SELECT COUNT(*) FROM unrecorded_changes",
                            [],
                            |row| row.get(0),
                        )?,
                        0,
                        "{label} creates no Unrecorded Change"
                    );
                    assert!(
                        pending.fixture.repository().status_bytes()?.is_empty(),
                        "{label}"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn inbox_resolution_rejects_a_stale_user_action_without_an_effect() -> Result<(), Box<dyn Error>>
    {
        let pending = pending_choice_fixture("cli-inbox-stale-resolution")?;
        resolve_choice(&pending, true, "accept")?;

        let session = seed_test_agent_session(
            pending.fixture.runtime_home_path(),
            pending.fixture.project_id(),
            pending.fixture.connection_id(),
            None,
        )?;
        let core = CoreService::for_mutation(&pending.fixture.mutation_context()?);
        let validated = core.validate_agent_session(
            AgentConnectionId::new(pending.fixture.connection_id()),
            ProjectId::new(pending.fixture.project_id()),
            session.runtime_session_id,
            session.project_session_id,
            OperationCategory::AgentWorkflow,
        )?;
        let state_version = pending.fixture.counts()?.state_version;
        let scope = core.update_scope(
            &pending.fixture.mutation_context()?,
            pending.fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_cli_inbox_stale_scope",
                idempotency_key: "idem_cli_inbox_stale_scope",
                dry_run: false,
                expected_state_version: Some(state_version),
                task_id: &pending.task_id,
                operation: ChangeUnitOperation::CreateCurrent,
                scope_summary: "Replace the basis after the original decision.",
            }),
            InvocationContext::agent_connection(OperationCategory::AgentWorkflow, validated),
        )?;
        assert_eq!(scope.response_value["base"]["response_kind"], "result");
        assert_eq!(
            pending.fixture.user_action_status(&pending.request_id)?,
            "stale"
        );
        let before_retry = pending.fixture.counts()?;

        let error = resolve_choice(&pending, true, "accept")
            .expect_err("a stale CLI resolution must be rejected");

        assert!(matches!(error, UserCommandError::Runtime(_)));
        assert!(error.to_string().contains("status: stale"));
        assert!(error.to_string().contains("refresh `volicord inbox`"));
        assert_eq!(pending.fixture.counts()?, before_retry);
        Ok(())
    }

    #[test]
    fn inbox_evidence_observation_uses_the_admitted_lexical_runtime_home(
    ) -> Result<(), Box<dyn Error>> {
        let pending = pending_observation_fixture("cli-inbox-observation-lexical-alias")?;
        let runtime_home = pending.fixture.runtime_home_path();
        let alias = runtime_home
            .parent()
            .expect("fixture Runtime Home has a parent")
            .join(".")
            .join(
                runtime_home
                    .file_name()
                    .expect("fixture Runtime Home has a file name"),
            );

        let output = resolve_observation_at_runtime_home(
            &pending,
            &alias,
            &pending.claim_id,
            &pending.artifact_id,
        )?;
        let response: Value = serde_json::from_str(&output)?;

        assert_eq!(response["base"]["response_kind"], "result");
        assert_eq!(
            response["user_action_resolution"]["body"]["resolution_type"],
            "evidence_observation"
        );
        assert_eq!(
            response["user_action_resolution"]["body"]["observation"]["target"]
                ["evidence_claim_id"],
            pending.claim_id
        );
        assert_eq!(
            response["user_action_resolution"]["body"]["observation"]["output_artifact_refs"][0]
                ["artifact_id"],
            pending.artifact_id
        );
        assert_eq!(
            pending.fixture.user_action_status(&pending.request_id)?,
            "resolved"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn inbox_choice_resolution_uses_the_admitted_symlink_runtime_home() -> Result<(), Box<dyn Error>>
    {
        use std::os::unix::fs::symlink;

        let pending = pending_choice_fixture("cli-inbox-choice-symlink-alias")?;
        let link = pending
            .fixture
            .runtime_home_path()
            .parent()
            .expect("fixture Runtime Home has a parent")
            .join("runtime-home-symlink");
        symlink(pending.fixture.runtime_home_path(), &link)?;

        let output = resolve_choice_at_runtime_home(&pending, &link, true, "accept")?;
        let response: Value = serde_json::from_str(&output)?;

        assert_eq!(response["base"]["response_kind"], "result");
        assert_eq!(
            pending.fixture.user_action_status(&pending.request_id)?,
            "resolved"
        );
        Ok(())
    }

    #[test]
    fn inbox_resolve_admits_before_project_reads_and_retries_after_setup(
    ) -> Result<(), Box<dyn Error>> {
        let mut pending = pending_choice_fixture("cli-inbox-admission")?;
        let state_path = pending
            .fixture
            .runtime_home_path()
            .join("projects")
            .join(pending.fixture.project_id())
            .join("state.sqlite");
        let unavailable_path = state_path.with_extension("sqlite.setup-busy");
        let diagnostics_path = diagnostics_db_path(pending.fixture.runtime_home_path());
        assert!(!diagnostics_path.exists());
        pending.fixture.release_mutation_admission();
        let setup = TestRuntimeHomeSetup::acquire(pending.fixture.runtime_home_path())?;
        fs::rename(&state_path, &unavailable_path)?;

        let error = resolve_choice(&pending, true, "accept")
            .expect_err("exclusive setup must reject the mutation before project reads");
        let UserCommandError::MutationAdmission(CliMutationAdmissionError::SetupInProgress(
            condition,
        )) = error
        else {
            panic!("inbox resolution must return the typed setup condition");
        };
        assert_eq!(condition.code(), "runtime_home.mutation.setup_in_progress");
        assert_eq!(condition.mutation_domain(), "cli.inbox.resolve");
        assert!(!diagnostics_path.exists());

        fs::rename(&unavailable_path, &state_path)?;
        drop(setup);
        let output = resolve_choice(&pending, true, "accept")?;
        let response: Value = serde_json::from_str(&output)?;
        assert_eq!(response["base"]["response_kind"], "result");
        assert_eq!(
            pending.fixture.user_action_status(&pending.request_id)?,
            "resolved"
        );
        Ok(())
    }

    #[test]
    fn inbox_resolve_setup_busy_is_no_effect_then_json_replays_exactly(
    ) -> Result<(), Box<dyn Error>> {
        let mut pending = pending_choice_fixture("cli-inbox-replay")?;
        let before = pending.fixture.authority_snapshot()?;
        let before_counts = pending.fixture.counts()?;
        let diagnostics_path = diagnostics_db_path(pending.fixture.runtime_home_path());
        assert!(!diagnostics_path.exists());
        pending.fixture.release_mutation_admission();
        let setup = TestRuntimeHomeSetup::acquire(pending.fixture.runtime_home_path())?;

        let error = resolve_choice(&pending, true, "accept")
            .expect_err("exclusive setup must reject inbox resolution without effects");
        assert!(matches!(
            error,
            UserCommandError::MutationAdmission(CliMutationAdmissionError::SetupInProgress(_))
        ));
        assert_eq!(pending.fixture.authority_snapshot()?, before);
        assert_eq!(pending.fixture.counts()?, before_counts);
        assert!(!diagnostics_path.exists());
        drop(setup);

        let first = resolve_choice(&pending, true, "accept")?;
        let response: Value = serde_json::from_str(&first)?;
        assert_eq!(response["base"]["response_kind"], "result");
        assert_eq!(
            pending
                .fixture
                .user_action_resolution_outcome(&pending.request_id)?,
            Some("accepted".to_owned())
        );
        let after_resolution = pending.fixture.authority_snapshot()?;
        let after_resolution_counts = pending.fixture.counts()?;
        assert_eq!(after_resolution.state_version, before.state_version + 1);
        assert!(diagnostics_path.is_file());

        let replay = resolve_choice(&pending, true, "accept")?;
        assert_eq!(replay, first);
        assert_eq!(
            pending.fixture.authority_snapshot()?,
            after_resolution,
            "exact replay must not create a second authority mutation"
        );
        assert_eq!(
            pending.fixture.counts()?,
            after_resolution_counts,
            "exact replay must not create a second invocation, event, or resolution"
        );
        Ok(())
    }

    #[test]
    fn inbox_resolve_preserves_text_output_and_best_effort_diagnostics(
    ) -> Result<(), Box<dyn Error>> {
        let mut pending = pending_choice_fixture("cli-inbox-diagnostic-failure")?;
        pending.fixture.release_mutation_admission();
        fs::create_dir_all(diagnostics_db_path(pending.fixture.runtime_home_path()))?;

        let output = resolve_choice(&pending, false, "accept")?;

        assert!(output.contains("Request status: resolved"));
        assert!(output.contains("Resolution outcome: accepted"));
        assert!(output.contains(
            "Authority effect: accepted outcome recorded; current semantic owner determines applicability"
        ));
        assert!(output.contains("Shaping application: none"));
        assert!(output.contains("Workflow:"));
        assert!(output.contains("Next actor:"));
        assert!(output.contains("Required action:"));
        assert_eq!(
            pending.fixture.user_action_status(&pending.request_id)?,
            "resolved"
        );
        Ok(())
    }

    #[test]
    fn inbox_resolve_preserves_canonical_choice_validation_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let mut pending = pending_choice_fixture("cli-inbox-choice-validation")?;
        let before = pending.fixture.authority_snapshot()?;
        let before_counts = pending.fixture.counts()?;
        pending.fixture.release_mutation_admission();

        let error = resolve_choice(&pending, true, "not-a-candidate")
            .expect_err("an unknown canonical option must be rejected");

        assert!(matches!(error, UserCommandError::Usage(_)));
        assert!(error.to_string().contains("does not match"));
        assert_eq!(pending.fixture.authority_snapshot()?, before);
        assert_eq!(pending.fixture.counts()?, before_counts);
        assert_eq!(
            pending.fixture.user_action_status(&pending.request_id)?,
            "pending"
        );
        assert!(!diagnostics_db_path(pending.fixture.runtime_home_path()).exists());
        Ok(())
    }

    #[test]
    fn inbox_resolve_validates_observation_candidates_without_effect() -> Result<(), Box<dyn Error>>
    {
        let mut pending = pending_observation_fixture("cli-inbox-observation-validation")?;
        let before = pending.fixture.authority_snapshot()?;
        let before_counts = pending.fixture.counts()?;
        pending.fixture.release_mutation_admission();

        let invalid_target =
            resolve_observation(&pending, "claim_not_a_candidate", &pending.artifact_id)
                .expect_err("an unknown evidence target must be rejected");
        assert!(matches!(invalid_target, UserCommandError::Usage(_)));
        assert!(invalid_target.to_string().contains("is not a candidate"));
        assert_eq!(pending.fixture.authority_snapshot()?, before);
        assert_eq!(pending.fixture.counts()?, before_counts);

        let invalid_artifact =
            resolve_observation(&pending, &pending.claim_id, "artifact_not_a_candidate")
                .expect_err("an unknown artifact candidate must be rejected");
        assert!(matches!(invalid_artifact, UserCommandError::Usage(_)));
        assert!(invalid_artifact.to_string().contains("is not a candidate"));
        assert_eq!(pending.fixture.authority_snapshot()?, before);
        assert_eq!(pending.fixture.counts()?, before_counts);
        assert_eq!(
            pending.fixture.user_action_status(&pending.request_id)?,
            "pending"
        );
        assert!(!diagnostics_db_path(pending.fixture.runtime_home_path()).exists());
        Ok(())
    }
}
