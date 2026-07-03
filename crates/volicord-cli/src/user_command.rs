use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};
use volicord_core::{CorePipelineError, CoreService, InvocationContext, PipelineResponse};
use volicord_store::{
    core_pipeline::{CoreProjectStore, UserJudgmentRecord},
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};
use volicord_types::{
    ActorSource, IdempotencyKey, JudgmentKind, JudgmentRationale, JudgmentResolutionOutcome,
    OperationCategory, PersistedUserJudgmentOptions, ProjectId, RecordUserJudgmentPayload,
    RecordUserJudgmentRequest, RequestId, SensitiveActionScope, StatusInclude, StatusRequest,
    SummaryCard, TaskId, ToolEnvelope, UserJudgmentContext, UserJudgmentId, UserJudgmentOption,
    VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL, VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB,
    VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL, VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
};

use crate::disclosure::{render_action_guidance_text, USER_CHANNEL_NON_GUARANTEE_TEXT};
use crate::project_context::{
    registered_project_for_repo, resolve_repository_root, ProjectCommandError,
};
use crate::summary_card::{
    count_state_text, render_coverage_summary_text, render_summary_card_text,
    summary_card_from_response, USER_CHANNEL_SUMMARY_GUARANTEE,
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

#[derive(Debug, Clone, Default)]
struct ParsedInboxOptions {
    repo: Option<PathBuf>,
    task: TaskSelector,
    choice: Option<String>,
    note: Option<String>,
    output: OutputFormat,
    positionals: Vec<String>,
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

#[derive(Debug, Clone)]
pub(crate) struct JudgmentRecordingInput<'a> {
    pub runtime_home: &'a Path,
    pub project_id: &'a str,
    pub expected_state_version: Option<u64>,
    pub record: &'a UserJudgmentRecord,
    pub selected_option: &'a UserJudgmentOption,
    pub note: Option<String>,
    pub verification_basis: &'a str,
    pub request_id: Option<String>,
    pub idempotency_key: Option<String>,
}

pub fn status_usage() -> String {
    "volicord status [--repo PATH] [--task active|ID] [--json]\n".to_owned()
}

pub fn inbox_usage() -> String {
    concat!(
        "volicord inbox [--repo PATH] [--task active|ID] [--json]\n",
        "volicord inbox answer <judgment-id> --choice <choice> [--repo PATH] [--note TEXT] [--json]\n",
        "volicord inbox open <judgment-id> [--repo PATH] [--json]\n"
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
        Some("answer") => command_inbox_answer(&args[1..], env_var, current_dir),
        Some("open") => command_inbox_open(&args[1..], env_var, current_dir),
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
    let store = CoreProjectStore::open(
        &resolved.runtime_home,
        &ProjectId::new(&resolved.project_id),
    )?;
    let task_id = selected_or_active_task_id(&store, &parsed.task)?;
    let response = CoreService::new(&resolved.runtime_home).status(
        StatusRequest {
            envelope: envelope(
                &resolved.project_id,
                task_id.as_deref(),
                generated_id("req_user_status"),
                None,
                None,
            ),
            include: StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: true,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: true,
            },
        },
        invocation(&resolved.project_id, OperationCategory::Read),
    )?;
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
    let parsed = parse_inbox_options(args, true, false, false, 0, current_dir)?;
    let resolved = resolve_user_project(parsed.repo.as_deref(), env_var, current_dir)?;
    let store = CoreProjectStore::open(
        &resolved.runtime_home,
        &ProjectId::new(&resolved.project_id),
    )?;
    let selected_task_id = selected_or_active_task_id(&store, &parsed.task)?;
    let has_selected_task = selected_task_id.is_some();
    let records = if let Some(task_id) = selected_task_id {
        store.pending_user_judgment_records(&TaskId::new(task_id))?
    } else {
        Vec::new()
    };
    render_inbox_items(&records, parsed.output, has_selected_task)
}

fn command_inbox_answer<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let parsed = parse_inbox_options(args, false, true, true, 1, current_dir)?;
    let judgment_id = required_inbox_positional(&parsed, 0, "judgment-id")?;
    let choice = parsed
        .choice
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| UserCommandError::Usage("missing required option: --choice".to_owned()))?;
    let resolved = resolve_user_project(parsed.repo.as_deref(), env_var, current_dir)?;
    let store = CoreProjectStore::open(
        &resolved.runtime_home,
        &ProjectId::new(&resolved.project_id),
    )?;
    let state_version = store.project_state()?.state_version;
    let record = store
        .user_judgment_record(judgment_id)?
        .ok_or_else(|| UserCommandError::Runtime("selected judgment was not found".to_owned()))?;
    if record.status != "pending" {
        return Err(UserCommandError::Runtime(format!(
            "selected judgment is not pending (status: {}); refresh `volicord inbox`",
            record.status
        )));
    }
    let options = decode_options(&record)?;
    let selected_option = select_option(&options, choice)?;
    let response = record_user_judgment_from_record(JudgmentRecordingInput {
        runtime_home: &resolved.runtime_home,
        project_id: &resolved.project_id,
        expected_state_version: Some(state_version),
        record: &record,
        selected_option: &selected_option,
        note: parsed.note,
        verification_basis: VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
        request_id: None,
        idempotency_key: None,
    })?;
    render_inbox_record_response(&response, parsed.output, &selected_option)
}

fn command_inbox_open<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let parsed = parse_inbox_options(args, false, false, false, 1, current_dir)?;
    let judgment_id = required_inbox_positional(&parsed, 0, "judgment-id")?;
    let resolved = resolve_user_project(parsed.repo.as_deref(), env_var, current_dir)?;
    let store = CoreProjectStore::open(
        &resolved.runtime_home,
        &ProjectId::new(&resolved.project_id),
    )?;
    let record = store
        .user_judgment_record(judgment_id)?
        .ok_or_else(|| UserCommandError::Runtime("selected judgment was not found".to_owned()))?;
    if record.status != "pending" {
        return Err(UserCommandError::Runtime(format!(
            "selected judgment is not pending (status: {}); refresh `volicord inbox`",
            record.status
        )));
    }
    if parsed.output == OutputFormat::Json {
        return serde_json::to_string_pretty(&json!({
            "opened": false,
            "judgment_id": judgment_id,
            "reason": "local_web_consent_url_unavailable",
            "fallback_command": format!("volicord inbox answer {judgment_id} --choice <choice>")
        }))
        .map(|text| format!("{text}\n"))
        .map_err(|error| UserCommandError::Runtime(error.to_string()));
    }
    Ok(format!(
        "Judgment Inbox open action_required\n{}",
        render_action_guidance_text(
            "action_required (not a fatal CLI error)",
            "No local consent URL is available from this CLI process.",
            &format!(
                "Use the URL shown in the MCP Judgment Inbox item, or run volicord inbox answer {judgment_id} --choice <choice>."
            ),
            USER_CHANNEL_NON_GUARANTEE_TEXT,
        )
    ))
}

fn parse_status_options(
    args: &[String],
    current_dir: &Path,
) -> Result<ParsedStatusOptions, UserCommandError> {
    let options = parse_status_raw_options(args)?;
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
        task: match options.value("task").as_deref() {
            None | Some("active") => TaskSelector::Active,
            Some(value) if value.trim().is_empty() => {
                return Err(UserCommandError::Usage(
                    "--task must not be empty".to_owned(),
                ));
            }
            Some(value) => TaskSelector::Id(value.to_owned()),
        },
        output: if options.value("json").is_some() {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
    })
}

fn parse_inbox_options(
    args: &[String],
    allow_task: bool,
    allow_note: bool,
    allow_choice: bool,
    max_positionals: usize,
    current_dir: &Path,
) -> Result<ParsedInboxOptions, UserCommandError> {
    let mut parsed = ParsedRawOptions::default();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(UserCommandError::Usage(inbox_usage()));
        }
        if token == "--json" {
            set_option(&mut parsed.values, "json", "true".to_owned())?;
        } else if token.starts_with("--json=") {
            return Err(UserCommandError::Usage(
                "--json does not accept a value".to_owned(),
            ));
        } else if token == "--repo" {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(UserCommandError::Usage(
                    "missing value for --repo".to_owned(),
                ));
            };
            set_nonempty_option(&mut parsed.values, "repo", value)?;
        } else if let Some(value) = token.strip_prefix("--repo=") {
            set_nonempty_option(&mut parsed.values, "repo", value)?;
        } else if token == "--task" {
            if !allow_task {
                return Err(UserCommandError::Usage("unknown option: --task".to_owned()));
            }
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(UserCommandError::Usage(
                    "missing value for --task".to_owned(),
                ));
            };
            set_nonempty_option(&mut parsed.values, "task", value)?;
        } else if let Some(value) = token.strip_prefix("--task=") {
            if !allow_task {
                return Err(UserCommandError::Usage("unknown option: --task".to_owned()));
            }
            set_nonempty_option(&mut parsed.values, "task", value)?;
        } else if token == "--choice" {
            if !allow_choice {
                return Err(UserCommandError::Usage(
                    "unknown option: --choice".to_owned(),
                ));
            }
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(UserCommandError::Usage(
                    "missing value for --choice".to_owned(),
                ));
            };
            set_nonempty_option(&mut parsed.values, "choice", value)?;
        } else if let Some(value) = token.strip_prefix("--choice=") {
            if !allow_choice {
                return Err(UserCommandError::Usage(
                    "unknown option: --choice".to_owned(),
                ));
            }
            set_nonempty_option(&mut parsed.values, "choice", value)?;
        } else if token == "--note" {
            if !allow_note {
                return Err(UserCommandError::Usage("unknown option: --note".to_owned()));
            }
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(UserCommandError::Usage(
                    "missing value for --note".to_owned(),
                ));
            };
            set_option(&mut parsed.values, "note", value.clone())?;
        } else if let Some(value) = token.strip_prefix("--note=") {
            if !allow_note {
                return Err(UserCommandError::Usage("unknown option: --note".to_owned()));
            }
            set_option(&mut parsed.values, "note", value.to_owned())?;
        } else if token.starts_with("--") {
            return Err(UserCommandError::Usage(format!("unknown option: {token}")));
        } else {
            parsed.positionals.push(token.clone());
        }
        index += 1;
    }
    if parsed.positionals.len() > max_positionals {
        return Err(UserCommandError::Usage(format!(
            "unexpected argument: {}",
            parsed.positionals[max_positionals]
        )));
    }
    Ok(ParsedInboxOptions {
        repo: parsed
            .value("repo")
            .map(PathBuf::from)
            .map(|path| absolute_path(current_dir, path)),
        task: match parsed.value("task").as_deref() {
            None | Some("active") => TaskSelector::Active,
            Some(value) if value.trim().is_empty() => {
                return Err(UserCommandError::Usage(
                    "--task must not be empty".to_owned(),
                ));
            }
            Some(value) => TaskSelector::Id(value.to_owned()),
        },
        choice: parsed.value("choice"),
        note: parsed.value("note"),
        output: if parsed.value("json").is_some() {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
        positionals: parsed.positionals,
    })
}

#[derive(Debug, Clone, Default)]
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
}

fn parse_status_raw_options(args: &[String]) -> Result<ParsedRawOptions, UserCommandError> {
    let mut parsed = ParsedRawOptions::default();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(UserCommandError::Usage(status_usage()));
        }
        if token == "--json" {
            set_option(&mut parsed.values, "json", "true".to_owned())?;
        } else if token.starts_with("--json=") {
            return Err(UserCommandError::Usage(
                "--json does not accept a value".to_owned(),
            ));
        } else if token == "--repo" {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(UserCommandError::Usage(
                    "missing value for --repo".to_owned(),
                ));
            };
            set_nonempty_option(&mut parsed.values, "repo", value)?;
        } else if let Some(value) = token.strip_prefix("--repo=") {
            set_nonempty_option(&mut parsed.values, "repo", value)?;
        } else if token == "--task" {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(UserCommandError::Usage(
                    "missing value for --task".to_owned(),
                ));
            };
            set_nonempty_option(&mut parsed.values, "task", value)?;
        } else if let Some(value) = token.strip_prefix("--task=") {
            set_nonempty_option(&mut parsed.values, "task", value)?;
        } else if token.starts_with("--") {
            return Err(UserCommandError::Usage(format!("unknown option: {token}")));
        } else {
            parsed.positionals.push(token.clone());
        }
        index += 1;
    }
    Ok(parsed)
}

fn set_nonempty_option(
    options: &mut UserOptions,
    name: &'static str,
    value: &str,
) -> Result<(), UserCommandError> {
    if value.trim().is_empty() {
        return Err(UserCommandError::Usage(format!(
            "--{name} must not be empty"
        )));
    }
    set_option(options, name, value.to_owned())
}

fn set_option(
    options: &mut UserOptions,
    name: &'static str,
    value: String,
) -> Result<(), UserCommandError> {
    if options.insert(name.to_owned(), vec![value]).is_some() {
        return Err(UserCommandError::Usage(format!(
            "duplicate option: --{name}"
        )));
    }
    Ok(())
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
        project_id: project.project_internal_id.clone(),
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

pub(crate) fn select_option(
    options: &[UserJudgmentOption],
    selector: &str,
) -> Result<UserJudgmentOption, UserCommandError> {
    if let Some(index) = parse_positive_index(selector)? {
        let Some(option) = options.get(index - 1).cloned() else {
            return Err(UserCommandError::Usage(format!(
                "option number {index} is out of range for the selected judgment"
            )));
        };
        if let Some(by_id) = options
            .iter()
            .find(|option| option.option_id.as_str() == selector)
        {
            if by_id.option_id != option.option_id {
                return Err(UserCommandError::Usage(format!(
                    "option selector `{selector}` is ambiguous: it matches an option number and an explicit option id"
                )));
            }
        }
        return Ok(option);
    }

    options
        .iter()
        .find(|option| option.option_id.as_str() == selector)
        .cloned()
        .ok_or_else(|| {
            UserCommandError::Usage(format!(
                "option selector `{selector}` does not match a numbered option or option id"
            ))
        })
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

fn envelope(
    project_id: &str,
    task_id: Option<&str>,
    request_id: String,
    idempotency_key: Option<String>,
    expected_state_version: Option<u64>,
) -> ToolEnvelope {
    ToolEnvelope {
        project_id: ProjectId::new(project_id),
        task_id: task_id.map(TaskId::new).into(),
        request_id: RequestId::new(request_id),
        idempotency_key: idempotency_key.map(IdempotencyKey::new).into(),
        expected_state_version: expected_state_version.into(),
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

pub(crate) fn decode_options(
    record: &UserJudgmentRecord,
) -> Result<Vec<UserJudgmentOption>, UserCommandError> {
    decode_json::<PersistedUserJudgmentOptions>("options_json", &record.options_json)?
        .into_current_options()
        .map_err(|error| UserCommandError::Runtime(error.to_string()))
}

pub(crate) fn record_user_judgment_from_record(
    input: JudgmentRecordingInput<'_>,
) -> Result<PipelineResponse, UserCommandError> {
    if input.record.status != "pending" && input.idempotency_key.is_none() {
        return Err(UserCommandError::Runtime(format!(
            "selected judgment is not pending (status: {}); refresh `volicord inbox`",
            input.record.status
        )));
    }
    let judgment_kind = parse_judgment_kind(&input.record.judgment_kind)?;
    let context = decode_json::<UserJudgmentContext>("context_json", &input.record.context_json)?;
    let request_id = input
        .request_id
        .unwrap_or_else(|| generated_id("req_user_judgment_record"));
    let idempotency_key = input
        .idempotency_key
        .unwrap_or_else(|| generated_id("idem_user_judgment_record"));
    CoreService::new(input.runtime_home)
        .record_user_judgment(
            RecordUserJudgmentRequest {
                envelope: envelope(
                    input.project_id,
                    Some(&input.record.task_id),
                    request_id,
                    Some(idempotency_key),
                    input.expected_state_version,
                ),
                user_judgment_id: UserJudgmentId::new(&input.record.judgment_id),
                judgment_kind,
                selected_option_id: input.selected_option.option_id.clone(),
                answer: answer_payload_for_record(
                    judgment_kind,
                    input.selected_option,
                    input.record,
                    &context,
                )?,
                rationale: rationale_for_selected_option(judgment_kind, input.selected_option),
                note: input.note.into(),
                accepted_risks: accepted_risks_for_record(
                    judgment_kind,
                    input.selected_option,
                    &context,
                ),
            },
            invocation_with_basis(
                input.project_id,
                OperationCategory::UserOnly,
                input.verification_basis,
            ),
        )
        .map_err(Into::into)
}

fn decode_json<T>(field: &'static str, text: &str) -> Result<T, UserCommandError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(text).map_err(|error| {
        UserCommandError::Runtime(format!("failed to decode user_judgments.{field}: {error}"))
    })
}

fn parse_judgment_kind(raw: &str) -> Result<JudgmentKind, UserCommandError> {
    serde_json::from_value(Value::String(raw.to_owned())).map_err(|_| {
        UserCommandError::Runtime(format!(
            "stored user_judgments.judgment_kind is not supported: {raw}"
        ))
    })
}

fn answer_payload_for_record(
    judgment_kind: JudgmentKind,
    selected_option: &UserJudgmentOption,
    record: &UserJudgmentRecord,
    context: &UserJudgmentContext,
) -> Result<RecordUserJudgmentPayload, UserCommandError> {
    let mut payload = empty_answer_payload();
    let branch = json_object(json!({
        "summary": format!("User selected option {}", selected_option.option_id),
        "selected_option": selected_option.option_id.as_str(),
        "selected_option_label": selected_option.label,
        "selected_option_consequence": selected_option.consequence,
    }));
    match judgment_kind {
        JudgmentKind::ProductDecision => payload.product_decision = Some(branch).into(),
        JudgmentKind::TechnicalDecision => payload.technical_decision = Some(branch).into(),
        JudgmentKind::ScopeDecision => payload.scope_decision = Some(branch).into(),
        JudgmentKind::SensitiveApproval => {
            payload.sensitive_action_scope =
                Some(sensitive_action_scope_for_record(record)?).into();
        }
        JudgmentKind::FinalAcceptance => payload.final_acceptance = Some(branch).into(),
        JudgmentKind::ResidualRiskAcceptance => {
            payload.residual_risk_acceptance = Some(json_object(json!({
                "summary": format!("User selected option {}", selected_option.option_id),
                "selected_option": selected_option.option_id.as_str(),
                "risk_ids": accepted_risk_ids(selected_option, context),
            })))
            .into();
        }
        JudgmentKind::Cancellation => payload.cancellation = Some(branch).into(),
    }
    Ok(payload)
}

fn empty_answer_payload() -> RecordUserJudgmentPayload {
    RecordUserJudgmentPayload {
        product_decision: None.into(),
        technical_decision: None.into(),
        scope_decision: None.into(),
        sensitive_action_scope: None.into(),
        final_acceptance: None.into(),
        residual_risk_acceptance: None.into(),
        cancellation: None.into(),
    }
}

fn sensitive_action_scope_for_record(
    record: &UserJudgmentRecord,
) -> Result<SensitiveActionScope, UserCommandError> {
    serde_json::from_str(&record.sensitive_action_scope_json).map_err(|error| {
        UserCommandError::Runtime(format!(
            "pending sensitive approval is missing a valid sensitive action scope: {error}"
        ))
    })
}

fn rationale_for_selected_option(
    judgment_kind: JudgmentKind,
    selected_option: &UserJudgmentOption,
) -> JudgmentRationale {
    let accepted = selected_option.resolution_outcome == JudgmentResolutionOutcome::Accepted;
    JudgmentRationale {
        summary: format!(
            "User selected `{}` for `{}` through the User Channel.",
            selected_option.option_id,
            judgment_kind_value(judgment_kind)
        ),
        selected_reason: Some(format!(
            "{} {}",
            selected_option.description, selected_option.consequence
        ))
        .into(),
        considered_alternatives: Vec::new(),
        rejected_alternatives: Vec::new(),
        assumptions: vec!["The answer covers only the addressed Core UserJudgment.".to_owned()],
        tradeoffs: if accepted {
            vec![selected_option.consequence.clone()]
        } else {
            Vec::new()
        },
        uncertainties: Vec::new(),
        review_triggers: if accepted {
            vec!["Revisit if the captured judgment basis becomes stale or superseded.".to_owned()]
        } else {
            Vec::new()
        },
        related_refs: Vec::new(),
        artifact_refs: Vec::new(),
    }
}

fn accepted_risks_for_record(
    judgment_kind: JudgmentKind,
    selected_option: &UserJudgmentOption,
    context: &UserJudgmentContext,
) -> Vec<volicord_types::AcceptedRiskInput> {
    if judgment_kind == JudgmentKind::ResidualRiskAcceptance
        && selected_option.resolution_outcome == JudgmentResolutionOutcome::Accepted
    {
        context.visible_risks.clone()
    } else {
        Vec::new()
    }
}

fn accepted_risk_ids(
    selected_option: &UserJudgmentOption,
    context: &UserJudgmentContext,
) -> Vec<String> {
    if selected_option.resolution_outcome == JudgmentResolutionOutcome::Accepted {
        context
            .visible_risks
            .iter()
            .map(|risk| risk.risk_id.as_str().to_owned())
            .collect()
    } else {
        Vec::new()
    }
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
    if response
        .response_value
        .get("pending_judgment_inbox_items")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
    {
        if let Some(line) = render_user_channel_availability_text(
            response.response_value.get("user_channel_availability"),
        ) {
            output.push_str(&line);
        }
    }
    if let Some(coverage) = render_coverage_summary_text(&response.response_value) {
        output.push_str(&coverage);
    }
    Ok(output)
}

fn render_inbox_items(
    records: &[UserJudgmentRecord],
    output: OutputFormat,
    has_selected_task: bool,
) -> Result<String, UserCommandError> {
    let summary_card = inbox_summary_card(records, has_selected_task);
    let user_channel_availability = cli_only_user_channel_availability();
    if output == OutputFormat::Json {
        let values = records
            .iter()
            .map(inbox_item_json)
            .collect::<Result<Vec<_>, _>>()?;
        return serde_json::to_string_pretty(&json!({
            "summary_card": summary_card,
            "user_channel_availability": user_channel_availability,
            "pending_judgment_inbox_items": values,
        }))
        .map(|text| format!("{text}\n"))
        .map_err(|error| UserCommandError::Runtime(error.to_string()));
    }

    let mut text = String::from("Judgment Inbox\n");
    text.push_str(&render_summary_card_text(&summary_card));
    if records.is_empty() {
        text.push_str("No pending judgments.\n");
        return Ok(text);
    }
    if let Some(line) = render_user_channel_availability_text(Some(&user_channel_availability)) {
        text.push_str(&line);
    }

    for (index, record) in records.iter().enumerate() {
        let request: volicord_types::PersistedUserJudgmentRequest =
            decode_json("request_json", &record.request_json)?;
        let context: UserJudgmentContext = decode_json("context_json", &record.context_json)?;
        let options = decode_options(record)?;
        let requirement = if request.required_for.is_empty() {
            "optional"
        } else {
            "required"
        };
        text.push_str(&format!("{}. {}\n", index + 1, request.question));
        text.push_str(&format!("   id: {}\n", record.judgment_id));
        text.push_str(&format!("   status: {requirement}\n"));
        if !context.summary.trim().is_empty() {
            text.push_str(&format!("   context: {}\n", context.summary));
        }
        text.push_str("   choices:\n");
        for option in &options {
            text.push_str(&format!(
                "   - {}: {} - {}\n",
                option.option_id.as_str(),
                option.label,
                option.description
            ));
        }
        text.push_str(&format!(
            "   answer: volicord inbox answer {} --choice <choice>\n",
            record.judgment_id
        ));
    }
    Ok(text)
}

fn inbox_summary_card(records: &[UserJudgmentRecord], has_selected_task: bool) -> SummaryCard {
    SummaryCard {
        task: if has_selected_task {
            "selected".to_owned()
        } else {
            "none".to_owned()
        },
        recording: "read_only".to_owned(),
        profile: "not_selected".to_owned(),
        write_ticket: "not_selected".to_owned(),
        evidence: "not_selected".to_owned(),
        user_judgment: count_state_text("pending", records.len()),
        changes: "not_selected".to_owned(),
        close_status: "not_selected".to_owned(),
        transport: "User Channel".to_owned(),
        next: records
            .first()
            .map(|record| {
                format!(
                    "Run volicord inbox answer {} --choice <choice>.",
                    record.judgment_id
                )
            })
            .unwrap_or_else(|| "none".to_owned()),
        next_action: None,
        guarantee: USER_CHANNEL_SUMMARY_GUARANTEE.to_owned(),
    }
}

fn render_inbox_record_response(
    response: &PipelineResponse,
    output: OutputFormat,
    selected_option: &UserJudgmentOption,
) -> Result<String, UserCommandError> {
    if output == OutputFormat::Json {
        return pretty_response(response);
    }
    if response_kind(response) != Some("result") {
        return render_rejected_or_json(response);
    }
    let mut text = String::from("Judgment Inbox answer recorded\n");
    text.push_str(&format!("selected: {}\n", selected_option.label));
    Ok(text)
}

fn inbox_item_json(record: &UserJudgmentRecord) -> Result<Value, UserCommandError> {
    let request: volicord_types::PersistedUserJudgmentRequest =
        decode_json("request_json", &record.request_json)?;
    let context: UserJudgmentContext = decode_json("context_json", &record.context_json)?;
    let options = decode_options(record)?;
    let requirement_status = if request.required_for.is_empty() {
        "optional"
    } else {
        "required"
    };
    Ok(json!({
        "judgment_id": &record.judgment_id,
        "project_id": &record.project_id,
        "task_id": &record.task_id,
        "change_unit_id": &record.change_unit_id,
        "question": request.question,
        "context_summary": context.summary,
        "choices": options
            .iter()
            .map(inbox_choice_json)
            .collect::<Vec<_>>(),
        "answer_constraints": {
            "choice_required": true,
            "note_allowed": true,
            "note_max_chars": 4000
        },
        "required": !request.required_for.is_empty(),
        "requirement_status": requirement_status,
        "required_for": request.required_for,
        "status": &record.status,
        "answer_path_availability": cli_only_user_channel_availability(),
        "preferred_capture_path": {
            "kind": "cli",
            "label": "CLI inbox",
            "available": true,
            "command": format!("volicord inbox answer {} --choice <choice>", record.judgment_id),
            "url": null,
            "capture_basis": VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
            "expires_at": null,
            "detail": "Answer from the local terminal as the user."
        },
        "fallbacks": [],
        "expires_at": request.expires_at
    }))
}

fn cli_only_user_channel_availability() -> Value {
    json!({
        "paths": [
            {
                "kind": "mcp_elicitation",
                "label": "Host prompt input",
                "available": false,
                "status": "unavailable",
                "capture_basis": VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
                "detail": "Host prompt input is unavailable from this CLI process."
            },
            {
                "kind": "prompt_capture",
                "label": "Chat command capture",
                "available": false,
                "status": "unavailable",
                "capture_basis": VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK,
                "detail": "Chat command capture is unavailable from this CLI process."
            },
            {
                "kind": "local_web_consent",
                "label": "Local consent URL",
                "available": false,
                "status": "unavailable",
                "capture_basis": VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB,
                "detail": "No local consent URL is available from this CLI process."
            },
            {
                "kind": "cli",
                "label": "CLI inbox",
                "available": true,
                "status": "available",
                "capture_basis": VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
                "detail": "Answer from the local terminal as the user."
            }
        ],
        "recommended_path_kind": "cli",
        "recommended_path_label": "CLI inbox",
        "recommendation": "Use CLI inbox to answer pending judgments."
    })
}

fn render_user_channel_availability_text(availability: Option<&Value>) -> Option<String> {
    let paths = availability?.get("paths")?.as_array()?;
    let mut fragments = Vec::new();
    for kind in [
        "mcp_elicitation",
        "prompt_capture",
        "local_web_consent",
        "cli",
    ] {
        let Some(path) = paths
            .iter()
            .find(|path| path["kind"].as_str() == Some(kind))
        else {
            continue;
        };
        let available = path["available"].as_bool().unwrap_or(false);
        let status = path["status"].as_str().unwrap_or("unavailable");
        let fragment = match kind {
            "mcp_elicitation" => {
                format!(
                    "host prompt {}",
                    if available {
                        "available"
                    } else {
                        "unavailable"
                    }
                )
            }
            "prompt_capture" => {
                if available {
                    format!("chat capture {status}")
                } else {
                    "chat capture unavailable".to_owned()
                }
            }
            "local_web_consent" => {
                format!(
                    "local consent {}",
                    if available {
                        "available"
                    } else {
                        "unavailable"
                    }
                )
            }
            "cli" => {
                format!(
                    "CLI inbox {}",
                    if available {
                        "available"
                    } else {
                        "unavailable"
                    }
                )
            }
            _ => continue,
        };
        fragments.push(fragment);
    }
    (!fragments.is_empty()).then(|| format!("Available answer paths: {}\n", fragments.join("; ")))
}

fn inbox_choice_json(option: &UserJudgmentOption) -> Value {
    json!({
        "choice_id": option.option_id.as_str(),
        "label": &option.label,
        "description": &option.description,
        "consequence": &option.consequence,
        "is_default": option.is_default,
    })
}

fn pretty_response(response: &PipelineResponse) -> Result<String, UserCommandError> {
    serde_json::to_string_pretty(&response.response_value)
        .map(|text| format!("{text}\n"))
        .map_err(|error| UserCommandError::Runtime(error.to_string()))
}

fn render_rejected_or_json(response: &PipelineResponse) -> Result<String, UserCommandError> {
    if response.response_value["errors"].is_array() {
        let mut output = String::from("Core request rejected\n");
        for error in response.response_value["errors"]
            .as_array()
            .unwrap_or(&Vec::new())
        {
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

fn judgment_kind_value(value: JudgmentKind) -> &'static str {
    match value {
        JudgmentKind::ProductDecision => "product_decision",
        JudgmentKind::TechnicalDecision => "technical_decision",
        JudgmentKind::ScopeDecision => "scope_decision",
        JudgmentKind::SensitiveApproval => "sensitive_approval",
        JudgmentKind::FinalAcceptance => "final_acceptance",
        JudgmentKind::ResidualRiskAcceptance => "residual_risk_acceptance",
        JudgmentKind::Cancellation => "cancellation",
    }
}

fn json_object(value: Value) -> serde_json::Map<String, Value> {
    match value {
        Value::Object(object) => object,
        _ => serde_json::Map::new(),
    }
}
