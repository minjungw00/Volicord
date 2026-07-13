use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use volicord_core::{
    Clock, CorePipelineError, CoreService, InvocationContext, PipelineResponse, SystemClock,
    UserChannelInboxProjection, UserChannelInboxProjectionRequest,
};
use volicord_store::{
    core_pipeline::{CoreProjectStore, EffectiveUserActionRecord},
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};
use volicord_types::{
    ActorSource, ArtifactId, EvidenceRelevanceStatus, EvidenceTarget, IdempotencyKey,
    OperationCategory, PersistedUserActionRequest, ProjectId, RequestId, ResolveUserActionRequest,
    StatusInclude, StatusRequest, SummaryCard, TaskId, ToolEnvelope, UserActionInboxForm,
    UserActionInboxItem, UserActionPresentationForm, UserActionPresentationPlan,
    UserActionRequestId, UserActionResolutionInput, UserActionStatus,
    VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
};

use crate::project_context::{
    registered_project_for_repo, resolve_repository_root, ProjectCommandError,
};
use crate::summary_card::{
    count_state_text, render_close_and_next_action_totals_text, render_coverage_summary_text,
    render_summary_card_text, summary_card_from_response, USER_CHANNEL_SUMMARY_GUARANTEE,
};

type UserOptions = BTreeMap<String, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserCommandError {
    Usage(String),
    Runtime(String),
}

impl fmt::Display for UserCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for UserCommandError {}

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
    positionals: Vec<String>,
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
            positionals: Vec::new(),
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
    pub runtime_home: &'a Path,
    pub project_id: &'a str,
    pub record: &'a EffectiveUserActionRecord,
    pub resolution: UserActionResolutionInput,
    pub verification_basis: &'a str,
    pub request_id: Option<String>,
    pub channel_submission_id: Option<String>,
}

pub fn status_usage() -> String {
    "volicord status [--repo PATH] [--task active|ID] [--json]\n".to_owned()
}

pub fn inbox_usage() -> String {
    concat!(
        "volicord inbox [--repo PATH] [--task active|ID] [--json]\n",
        "volicord inbox resolve <user-action-request-id> --choice <choice> [--repo PATH] [--note TEXT] [--json]\n",
        "volicord inbox resolve <user-action-request-id> (--criterion ID | --claim ID) --artifact ID [--artifact ID ...] --summary TEXT [--contradicted] [--repo PATH] [--json]\n"
    )
    .to_owned()
}

pub fn run_status_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    if matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    ) {
        if args.len() == 1 {
            return Ok(status_usage());
        }
        return Err(UserCommandError::Usage(format!(
            "unexpected argument: {}\n\n{}",
            args[1],
            status_usage()
        )));
    }
    command_status(args, env_var, current_dir)
}

pub fn run_inbox_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    match args.first().map(String::as_str) {
        Some("-h" | "--help" | "help") => {
            if args.len() == 1 {
                Ok(inbox_usage())
            } else {
                Err(UserCommandError::Usage(format!(
                    "unexpected argument: {}\n\n{}",
                    args[1],
                    inbox_usage()
                )))
            }
        }
        Some("resolve")
            if matches!(
                args.get(1).map(String::as_str),
                Some("-h" | "--help" | "help")
            ) =>
        {
            if args.len() == 2 {
                Ok(inbox_usage())
            } else {
                Err(UserCommandError::Usage(format!(
                    "unexpected argument: {}\n\n{}",
                    args[2],
                    inbox_usage()
                )))
            }
        }
        Some("resolve") => command_inbox_resolve(&args[1..], env_var, current_dir),
        Some(token) if !token.starts_with('-') => Err(UserCommandError::Usage(format!(
            "unknown inbox command: {token}\n\n{}",
            inbox_usage()
        ))),
        _ => command_inbox_list(args, env_var, current_dir),
    }
}

fn command_status<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let parsed = parse_status_options(args, current_dir)?;
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
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let parsed = parse_inbox_options(args, true, 0, current_dir)?;
    reject_resolution_flags_for_list(&parsed)?;
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
                ActorSource::LocalUser,
                VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
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
    CoreService::new(&resolved.runtime_home)
        .status(
            StatusRequest {
                envelope: envelope(
                    &resolved.project_id,
                    task_id,
                    generated_id("req_user_status"),
                    None,
                ),
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

pub(crate) fn canonical_user_action_inbox_items(
    runtime_home: &Path,
    project_id: &str,
    task_id: &str,
    actor_source: ActorSource,
    verification_basis: &str,
    session_id: Option<&str>,
) -> Result<Vec<UserActionInboxItem>, UserCommandError> {
    user_channel_inbox_projection(
        runtime_home,
        project_id,
        task_id,
        actor_source,
        verification_basis,
        session_id,
    )?
    .map(|projection| {
        projection
            .items
            .into_iter()
            .map(|item| item.inbox_item)
            .collect()
    })
    .ok_or_else(|| {
        UserCommandError::Runtime(
            "Core denied the canonical User Channel inbox projection".to_owned(),
        )
    })
}

fn user_channel_inbox_projection(
    runtime_home: &Path,
    project_id: &str,
    task_id: &str,
    actor_source: ActorSource,
    verification_basis: &str,
    session_id: Option<&str>,
) -> Result<Option<UserChannelInboxProjection>, UserCommandError> {
    let invocation = InvocationContext::new(
        ProjectId::new(project_id),
        actor_source,
        OperationCategory::Read,
        verification_basis,
    );
    let invocation = match session_id {
        Some(session_id) => invocation.with_session_id(session_id),
        None => invocation,
    };
    CoreService::new(runtime_home)
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
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let parsed = parse_inbox_options(args, false, 1, current_dir)?;
    let request_id = required_inbox_positional(&parsed, 0, "user-action-request-id")?;
    let resolved = resolve_user_project(parsed.repo.as_deref(), env_var, current_dir)?;
    let store = CoreProjectStore::open_read_only(
        &resolved.runtime_home,
        &ProjectId::new(&resolved.project_id),
    )?;
    let now = SystemClock.project_now(&store)?;
    let record = store.user_action_record(request_id, &now)?.ok_or_else(|| {
        UserCommandError::Runtime("selected user action was not found".to_owned())
    })?;
    let resolution = match record.status {
        UserActionStatus::Pending => {
            let items = canonical_user_action_inbox_items(
                &resolved.runtime_home,
                &resolved.project_id,
                &record.request.task_id,
                ActorSource::LocalUser,
                VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
                None,
            )?;
            let item = canonical_inbox_item(items, request_id)?;
            resolution_from_form(&item.form, &parsed)?
        }
        _ if record.resolution.is_some() => resolution_from_immutable_request(&record, &parsed)?,
        status => {
            return Err(UserCommandError::Runtime(format!(
                "selected user action is not pending (status: {}); refresh `volicord inbox`",
                enum_text(status)
            )));
        }
    };
    let (stable_request_id, channel_submission_id) = stable_cli_resolution_ids(request_id);
    let response = resolve_user_action_from_record(UserActionResolutionRecordingInput {
        runtime_home: &resolved.runtime_home,
        project_id: &resolved.project_id,
        record: &record,
        resolution,
        verification_basis: VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
        request_id: Some(stable_request_id),
        channel_submission_id: Some(channel_submission_id),
    })?;
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

fn canonical_inbox_item(
    items: Vec<UserActionInboxItem>,
    request_id: &str,
) -> Result<UserActionInboxItem, UserCommandError> {
    items
        .into_iter()
        .find(|item| item.user_action_request_id.as_str() == request_id)
        .ok_or_else(|| {
            UserCommandError::Runtime(
                "selected user action is no longer in the canonical pending inbox; refresh `volicord inbox`"
                    .to_owned(),
            )
        })
}

pub(crate) fn select_inbox_choice(
    choices: &[volicord_types::UserActionInboxChoice],
    selector: &str,
) -> Result<volicord_types::UserActionInboxChoice, UserCommandError> {
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
    candidates: &[volicord_types::ArtifactRef],
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

fn reject_resolution_flags_for_list(parsed: &ParsedInboxOptions) -> Result<(), UserCommandError> {
    if parsed.choice.is_some()
        || parsed.note.is_some()
        || parsed.acceptance_criterion_id.is_some()
        || parsed.evidence_claim_id.is_some()
        || !parsed.artifact_ids.is_empty()
        || parsed.summary.is_some()
        || parsed.relevance_status == EvidenceRelevanceStatus::Contradicted
    {
        return Err(UserCommandError::Usage(
            "resolution flags require `volicord inbox resolve <user-action-request-id>`".to_owned(),
        ));
    }
    Ok(())
}

fn parse_status_options(
    args: &[String],
    current_dir: &Path,
) -> Result<ParsedStatusOptions, UserCommandError> {
    let options = parse_raw_options(args, true, false)?;
    if !options.positionals.is_empty() {
        return Err(UserCommandError::Usage(format!(
            "unexpected argument: {}",
            options.positionals[0]
        )));
    }
    Ok(ParsedStatusOptions {
        repo: options
            .value("repo")
            .map(PathBuf::from)
            .map(|path| absolute_path(current_dir, path)),
        task: parse_task_selector(options.value("task"))?,
        output: output_format(&options),
    })
}

fn parse_inbox_options(
    args: &[String],
    allow_task: bool,
    max_positionals: usize,
    current_dir: &Path,
) -> Result<ParsedInboxOptions, UserCommandError> {
    let options = parse_raw_options(args, allow_task, true)?;
    if options.positionals.len() > max_positionals {
        return Err(UserCommandError::Usage(format!(
            "unexpected argument: {}",
            options.positionals[max_positionals]
        )));
    }
    Ok(ParsedInboxOptions {
        repo: options
            .value("repo")
            .map(PathBuf::from)
            .map(|path| absolute_path(current_dir, path)),
        task: parse_task_selector(options.value("task"))?,
        choice: options.value("choice"),
        note: options.value("note"),
        acceptance_criterion_id: options.value("criterion"),
        evidence_claim_id: options.value("claim"),
        artifact_ids: options.values("artifact"),
        summary: options.value("summary"),
        relevance_status: if options.has("contradicted") {
            EvidenceRelevanceStatus::Contradicted
        } else {
            EvidenceRelevanceStatus::Supported
        },
        output: output_format(&options),
        positionals: options.positionals,
    })
}

#[derive(Debug, Default)]
struct ParsedRawOptions {
    values: UserOptions,
    positionals: Vec<String>,
}

impl ParsedRawOptions {
    fn value(&self, name: &str) -> Option<String> {
        self.values
            .get(name)
            .and_then(|values| values.first())
            .cloned()
    }

    fn values(&self, name: &str) -> Vec<String> {
        self.values.get(name).cloned().unwrap_or_default()
    }

    fn has(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }
}

fn parse_raw_options(
    args: &[String],
    allow_task: bool,
    allow_resolution: bool,
) -> Result<ParsedRawOptions, UserCommandError> {
    let mut parsed = ParsedRawOptions::default();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if matches!(token.as_str(), "-h" | "--help" | "help") {
            return Err(UserCommandError::Usage(if allow_resolution {
                inbox_usage()
            } else {
                status_usage()
            }));
        }
        if token == "--json" || token == "--contradicted" {
            let name = &token[2..];
            if name == "contradicted" && !allow_resolution {
                return Err(UserCommandError::Usage(format!("unknown option: {token}")));
            }
            set_option(&mut parsed.values, name, "true".to_owned(), false)?;
        } else if let Some((name, value)) =
            token.strip_prefix("--").and_then(|raw| raw.split_once('='))
        {
            parse_named_value(&mut parsed, name, value, allow_task, allow_resolution)?;
        } else if let Some(name) = token.strip_prefix("--") {
            if matches!(name, "json" | "contradicted") {
                unreachable!("boolean options handled above");
            }
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| UserCommandError::Usage(format!("missing value for --{name}")))?;
            parse_named_value(&mut parsed, name, value, allow_task, allow_resolution)?;
        } else {
            parsed.positionals.push(token.clone());
        }
        index += 1;
    }
    Ok(parsed)
}

fn parse_named_value(
    parsed: &mut ParsedRawOptions,
    name: &str,
    value: &str,
    allow_task: bool,
    allow_resolution: bool,
) -> Result<(), UserCommandError> {
    let allowed = match name {
        "repo" => true,
        "task" => allow_task,
        "choice" | "note" | "criterion" | "claim" | "artifact" | "summary" => allow_resolution,
        _ => false,
    };
    if !allowed {
        return Err(UserCommandError::Usage(format!("unknown option: --{name}")));
    }
    if value.trim().is_empty() {
        return Err(UserCommandError::Usage(format!(
            "--{name} must not be empty"
        )));
    }
    set_option(
        &mut parsed.values,
        name,
        value.to_owned(),
        name == "artifact",
    )
}

fn set_option(
    options: &mut UserOptions,
    name: &str,
    value: String,
    repeated: bool,
) -> Result<(), UserCommandError> {
    if repeated {
        options.entry(name.to_owned()).or_default().push(value);
        return Ok(());
    }
    if options.insert(name.to_owned(), vec![value]).is_some() {
        return Err(UserCommandError::Usage(format!(
            "option --{name} may be specified only once"
        )));
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

fn output_format(options: &ParsedRawOptions) -> OutputFormat {
    if options.has("json") {
        OutputFormat::Json
    } else {
        OutputFormat::Text
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
    CoreService::new(input.runtime_home)
        .resolve_user_action(
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
            invocation_with_basis(
                input.project_id,
                OperationCategory::UserOnly,
                input.verification_basis,
            ),
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
    invocation_with_basis(
        project_id,
        operation_category,
        VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
    )
}

fn invocation_with_basis(
    project_id: &str,
    operation_category: OperationCategory,
    verification_basis: &str,
) -> InvocationContext {
    InvocationContext::new(
        ProjectId::new(project_id),
        ActorSource::LocalUser,
        operation_category,
        verification_basis,
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
    if let Some(coverage) = render_coverage_summary_text(&response.response_value) {
        output.push_str(&coverage);
    }
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
    let mut fragments = Vec::new();
    for (kind, label) in [
        ("mcp_elicitation", "host prompt"),
        ("prompt_capture", "chat capture"),
        ("local_web_consent", "local consent"),
        ("cli", "CLI inbox"),
    ] {
        let Some(path) = paths
            .iter()
            .find(|path| path["kind"].as_str() == Some(kind))
        else {
            continue;
        };
        fragments.push(format!(
            "{label} {}",
            if path["available"].as_bool().unwrap_or(false) {
                "available"
            } else {
                "unavailable"
            }
        ));
    }
    (!fragments.is_empty()).then(|| format!("Available resolve paths: {}\n", fragments.join("; ")))
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

fn required_inbox_positional<'a>(
    parsed: &'a ParsedInboxOptions,
    index: usize,
    label: &'static str,
) -> Result<&'a str, UserCommandError> {
    parsed
        .positionals
        .get(index)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| UserCommandError::Usage(format!("missing required argument: {label}")))
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
        .unwrap_or_default();
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
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_parser_preserves_observation_candidates() {
        let parsed = parse_inbox_options(
            &[
                "ua_1".to_owned(),
                "--criterion=criterion_1".to_owned(),
                "--artifact=artifact_1".to_owned(),
                "--artifact".to_owned(),
                "artifact_2".to_owned(),
                "--summary=Checked".to_owned(),
                "--contradicted".to_owned(),
            ],
            false,
            1,
            Path::new("/repo"),
        )
        .expect("valid observation form");

        assert_eq!(parsed.positionals, ["ua_1"]);
        assert_eq!(parsed.artifact_ids, ["artifact_1", "artifact_2"]);
        assert_eq!(
            parsed.relevance_status,
            EvidenceRelevanceStatus::Contradicted
        );
    }

    #[test]
    fn old_inbox_subcommands_have_no_compatibility_alias() {
        for old in ["answer", "open", "observe"] {
            let error = run_inbox_command(&[old.to_owned()], |_| None, Path::new("/repo"))
                .expect_err("old command must be rejected before runtime lookup");
            assert!(error.to_string().contains("unknown inbox command"));
        }
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
}
