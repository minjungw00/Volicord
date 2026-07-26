use std::{
    collections::BTreeSet,
    fmt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_command_model::{InboxArgs, InboxCommand, InboxResolveArgs, StatusArgs};
use volicord_core::{
    CorePipelineError, CoreService, InvocationContext, PipelineResponse,
    UserChannelInboxProjection, UserChannelInboxProjectionRequest,
};
use volicord_store::{
    core_pipeline::{CoreProjectStore, EffectiveUserActionRecord},
    diagnostics::{start_diagnostic_session, DiagnosticSessionStart, DiagnosticTransport},
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    RuntimeHomeMutationContext, StoreError,
};
use volicord_types::ids::{
    ArtifactId, IdempotencyKey, ProjectId, RequestId, TaskId, UserActionRequestId,
};
use volicord_types::methods::{ResolveUserActionRequest, StatusInclude, StatusRequest};
use volicord_types::presentation::{UserActionPresentationForm, UserActionPresentationPlan};
use volicord_types::schema::{
    EvidenceTarget, PersistedUserActionRequest, SummaryCard, ToolEnvelope, UserActionInboxForm,
    UserActionInboxItem, UserActionResolutionInput,
};
use volicord_types::values::{
    EvidenceRelevanceStatus, OperationCategory, UserActionChannelKind, UserActionStatus,
};

use crate::mutation_admission::{with_cli_runtime_home_mutation_result, CliMutationAdmissionError};
use crate::project_context::{
    registered_project_for_repo, registered_project_for_repo_admitted, resolve_repository_root,
    ProjectCommandError,
};
use crate::summary_card::{
    count_state_text, render_close_and_next_action_totals_text, render_summary_card_text,
    summary_card_from_response, USER_CHANNEL_SUMMARY_GUARANTEE,
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
    pub record: &'a EffectiveUserActionRecord,
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
    let response = status_response(&resolved, task_id.as_deref())?;
    render_status_response(&response, parsed.output)
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
    let projection = task_id
        .as_deref()
        .map(|task_id| {
            user_channel_inbox_projection(
                &resolved.runtime_home,
                &resolved.project_id,
                task_id,
                None,
            )
        })
        .transpose()?
        .flatten();
    render_inbox_response(projection.as_ref(), parsed.output, task_id.is_some())
}

fn status_response(
    resolved: &ResolvedUserProject,
    task_id: Option<&str>,
) -> Result<PipelineResponse, UserCommandError> {
    CoreService::for_read_only(&resolved.runtime_home)
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
        .map_err(Into::into)
}

fn user_channel_inbox_projection(
    runtime_home: &Path,
    project_id: &str,
    task_id: &str,
    session_id: Option<&str>,
) -> Result<Option<UserChannelInboxProjection>, UserCommandError> {
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
        .user_channel_inbox_projection(
            UserChannelInboxProjectionRequest {
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
        .user_channel_inbox_resolution_snapshot_from_store(
            &store,
            &UserActionRequestId::new(request_id),
            invocation(&project.project_internal_id, OperationCategory::Read),
        )?
        .ok_or_else(|| {
            UserCommandError::Runtime("selected user action was not found".to_owned())
        })?;
    let resolution = match snapshot.record.status {
        UserActionStatus::Pending => {
            let projection = snapshot.pending_projection.as_ref().ok_or_else(|| {
                UserCommandError::Runtime(
                    "selected user action is no longer in the canonical pending inbox; refresh `volicord inbox`"
                        .to_owned(),
                )
            })?;
            let item = projection
                .items
                .iter()
                .find(|item| {
                    item.inbox_item.user_action_request_id.as_str() == request_id
                })
                .ok_or_else(|| {
                    UserCommandError::Runtime(
                        "selected user action is no longer in the canonical pending inbox; refresh `volicord inbox`"
                            .to_owned(),
                    )
                })?;
            resolution_from_form(&item.inbox_item.form, parsed)?
        }
        _ if snapshot.record.resolution.is_some() => {
            resolution_from_immutable_request(&snapshot.record, parsed)?
        }
        status => {
            return Err(UserCommandError::Runtime(format!(
                "selected user action is not pending (status: {}); refresh `volicord inbox`",
                enum_text(status)
            )));
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

fn resolution_from_immutable_request(
    record: &EffectiveUserActionRecord,
    parsed: &ParsedInboxOptions,
) -> Result<UserActionResolutionInput, UserCommandError> {
    let request: PersistedUserActionRequest = serde_json::from_str(&record.request.request_json)
        .map_err(|error| {
            UserCommandError::Runtime(format!(
                "failed to decode user_action_requests.request_json for replay: {error}"
            ))
        })?;
    let form = request.body.capture_form().map_err(|error| {
        UserCommandError::Runtime(format!(
            "invalid immutable user-action request for replay: {error}"
        ))
    })?;
    resolution_from_form(&form, parsed)
}

fn resolution_from_form(
    form: &UserActionInboxForm,
    parsed: &ParsedInboxOptions,
) -> Result<UserActionResolutionInput, UserCommandError> {
    match form {
        UserActionInboxForm::Choice { choices, .. } => {
            reject_observation_flags(parsed)?;
            let selector = parsed
                .choice
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    UserCommandError::Usage("missing required option: --choice".to_owned())
                })?;
            let selected = select_inbox_choice(choices, selector)?;
            Ok(UserActionResolutionInput::Choice {
                selected_option_id: selected.choice_id,
                note: parsed.note.clone().into(),
            })
        }
        UserActionInboxForm::EvidenceObservation {
            target_candidates,
            artifact_candidates,
            ..
        } => {
            if parsed.choice.is_some() || parsed.note.is_some() {
                return Err(UserCommandError::Usage(
                    "--choice and --note are valid only for a choice user action".to_owned(),
                ));
            }
            if parsed.acceptance_criterion_id.is_some() == parsed.evidence_claim_id.is_some() {
                return Err(UserCommandError::Usage(
                    "exactly one of --criterion or --claim is required".to_owned(),
                ));
            }
            if parsed.artifact_ids.is_empty() {
                return Err(UserCommandError::Usage(
                    "at least one non-empty --artifact is required".to_owned(),
                ));
            }
            let summary = parsed
                .summary
                .as_ref()
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .ok_or_else(|| {
                    UserCommandError::Usage("a non-empty --summary is required".to_owned())
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
    choices: &[volicord_types::schema::UserActionInboxChoice],
    selector: &str,
) -> Result<volicord_types::schema::UserActionInboxChoice, UserCommandError> {
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
                "--artifact must not be empty".to_owned(),
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
            "--task must not be empty".to_owned(),
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
    if input.record.status != UserActionStatus::Pending && input.channel_submission_id.is_none() {
        return Err(UserCommandError::Runtime(format!(
            "selected user action is not pending (status: {}); refresh `volicord inbox`",
            enum_text(input.record.status)
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
    CoreService::for_mutation(context)
        .resolve_user_action(
            context,
            ResolveUserActionRequest {
                envelope: envelope(
                    input.project_id,
                    Some(&input.record.request.task_id),
                    request_id,
                    Some(channel_submission_id.clone()),
                ),
                user_action_request_id: UserActionRequestId::new(
                    &input.record.request.user_action_request_id,
                ),
                channel_submission_id,
                resolution: input.resolution,
            },
            invocation,
        )
        .map_err(Into::into)
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
        dry_run: false,
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
    output: OutputFormat,
) -> Result<String, UserCommandError> {
    if output == OutputFormat::Json {
        return pretty_response(response);
    }
    if response_kind(response) != Some("result") {
        return render_rejected_or_json(response);
    }
    let mut output = String::from("User Channel status\n");
    if let Some(card) = summary_card_from_response(&response.response_value) {
        output.push_str(&render_summary_card_text(&card));
    }
    output.push_str(&render_close_and_next_action_totals_text(
        &response.response_value,
    ));
    Ok(output)
}

fn render_inbox_response(
    projection: Option<&UserChannelInboxProjection>,
    output: OutputFormat,
    has_selected_task: bool,
) -> Result<String, UserCommandError> {
    let items = projection
        .map(|projection| {
            projection
                .items
                .iter()
                .map(|item| &item.inbox_item)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let availability = projection.map(|projection| &projection.user_channel_availability);
    let summary_card = inbox_summary_card(&items, has_selected_task);
    if output == OutputFormat::Json {
        return serde_json::to_string_pretty(&json!({
            "summary_card": summary_card,
            "user_channel_availability": availability,
            "pending_user_action_inbox_items": items,
        }))
        .map(|text| format!("{text}\n"))
        .map_err(|error| UserCommandError::Runtime(error.to_string()));
    }
    let mut text = String::from("User Action Inbox\n");
    text.push_str(&render_summary_card_text(&summary_card));
    if items.is_empty() {
        text.push_str("No pending user actions.\n");
        return Ok(text);
    }
    let availability_value = availability
        .map(serde_json::to_value)
        .transpose()
        .map_err(|error| UserCommandError::Runtime(error.to_string()))?;
    if let Some(line) = render_user_channel_availability_text(availability_value.as_ref()) {
        text.push_str(&line);
    }
    for (index, item) in items.iter().enumerate() {
        text.push_str(&format!("{}. {}\n", index + 1, item.question));
        text.push_str(&format!("   id: {}\n", item.user_action_request_id));
        text.push_str(&format!("   kind: {}\n", enum_text(item.action_kind)));
        if !item.context_summary.trim().is_empty() {
            text.push_str(&format!("   context: {}\n", item.context_summary));
        }
        let presentation = UserActionPresentationPlan::from_form(&item.form)
            .map_err(|error| UserCommandError::Runtime(error.to_string()))?;
        match &presentation.form {
            UserActionPresentationForm::Choice {
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
                text.push_str(&format!(
                    "   resolve:\n     volicord inbox resolve {} --choice <choice>\n",
                    item.user_action_request_id
                ));
            }
            UserActionPresentationForm::EvidenceObservation {
                targets,
                artifacts,
                relevance_options,
                summary_max_chars,
            } => {
                text.push_str("   target candidates:\n");
                for target in targets {
                    text.push_str(&format!(
                        "   - {}: {}\n     metadata: {}\n",
                        target.selector, target.display_name, target.metadata_json
                    ));
                }
                text.push_str("   artifact candidates:\n");
                for artifact in artifacts {
                    text.push_str(&format!(
                        "   - {}: {}\n     metadata: {}\n",
                        artifact.artifact_id, artifact.display_name, artifact.metadata_json
                    ));
                }
                text.push_str(&format!(
                    "   relevance options: {}\n   summary max characters: {}\n",
                    relevance_options.join(", "),
                    summary_max_chars
                ));
                text.push_str(&format!(
                    "   resolve:\n     volicord inbox resolve {} (--criterion ID | --claim ID) --artifact ID --summary TEXT\n",
                    item.user_action_request_id
                ));
            }
        }
    }
    Ok(text)
}

fn inbox_summary_card(items: &[&UserActionInboxItem], has_selected_task: bool) -> SummaryCard {
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

fn render_resolve_response(
    response: &PipelineResponse,
    output: OutputFormat,
) -> Result<String, UserCommandError> {
    if output == OutputFormat::Json {
        return pretty_response(response);
    }
    if response_kind(response) != Some("result") {
        return render_rejected_or_json(response);
    }
    Ok("User action resolved\n".to_owned())
}

fn render_user_channel_availability_text(availability: Option<&Value>) -> Option<String> {
    let paths = availability?.get("paths")?.as_array()?;
    let path = paths
        .iter()
        .find(|path| path["kind"].as_str() == Some("cli"))?;
    Some(format!(
        "CLI inbox {}\n",
        if path["available"].as_bool().unwrap_or(false) {
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

fn response_kind(response: &PipelineResponse) -> Option<&str> {
    response.response_value["base"]["response_kind"].as_str()
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
        "cli_direct_user_channel",
        "resolve_user_action",
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

fn enum_text<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .expect("closed user-action enum serialization cannot fail")
        .as_str()
        .expect("closed user-action enums must serialize as strings")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::{error::Error, ffi::OsString, fs};

    use volicord_store::diagnostics::diagnostics_db_path;
    use volicord_test_support::{
        core_fixtures::{
            artifact_input_for_handle, CoreFixture, ObservationUserActionFixture,
            UpdateScopeFixture, UserActionFixture,
        },
        seed_test_agent_session, TestRuntimeHomeSetup,
    };
    use volicord_types::ids::{AgentConnectionId, EvidenceClaimId};
    use volicord_types::schema::StagedArtifactHandle;
    use volicord_types::values::{ChangeUnitOperation, JudgmentKind};

    use super::*;

    struct PendingChoiceFixture {
        fixture: CoreFixture,
        request_id: String,
    }

    struct PendingObservationFixture {
        fixture: CoreFixture,
        request_id: String,
        claim_id: String,
        artifact_id: String,
    }

    fn pending_choice_fixture(prefix: &str) -> Result<PendingChoiceFixture, Box<dyn Error>> {
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
                judgment_kind: JudgmentKind::ProductDecision,
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
        let staged = core.stage_artifact(
            &fixture.mutation_context()?,
            fixture.stage_artifact_request(
                "req_cli_observation_stage",
                Some("idem_cli_observation_stage"),
                false,
                Some(scope_state_version),
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
        let claim_id = format!(
            "claim_{}",
            claim_statement
                .as_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
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

        assert_eq!(output, "User action resolved\n");
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
