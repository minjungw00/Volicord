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
    TaskId, ToolEnvelope, UserJudgmentContext, UserJudgmentId, UserJudgmentOption,
    VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
};

use crate::project_context::{
    registered_project_for_repo, resolve_repository_root, ProjectCommandError,
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
struct ParsedUserOptions {
    repo: Option<PathBuf>,
    task: TaskSelector,
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
struct SelectedJudgment {
    record: UserJudgmentRecord,
    display_index: Option<usize>,
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

pub fn user_usage() -> String {
    concat!(
        "volicord user status [--repo PATH] [--task active|ID] [--json]\n",
        "volicord user judgments [--repo PATH] [--task active|ID] [--json]\n",
        "volicord user judgment show INDEX_OR_ID [--repo PATH] [--json]\n",
        "volicord user judgment answer INDEX_OR_ID OPTION_INDEX_OR_ID [--repo PATH] [--note TEXT] [--json]\n"
    )
    .to_owned()
}

pub fn run_user_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Ok(user_usage());
    };

    match subcommand {
        "-h" | "--help" | "help" => {
            if args.len() == 1 {
                Ok(user_usage())
            } else {
                Err(UserCommandError::Usage(format!(
                    "unexpected argument: {}\n\n{}",
                    args[1],
                    user_usage()
                )))
            }
        }
        "status" => command_status(&args[1..], env_var, current_dir),
        "judgments" => command_judgments(&args[1..], env_var, current_dir),
        "judgment" => command_judgment(&args[1..], env_var, current_dir),
        other => Err(UserCommandError::Usage(format!(
            "unknown user command: {other}\n\n{}",
            user_usage()
        ))),
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
    let parsed = parse_user_options(args, true, false, 0, current_dir)?;
    let resolved = resolve_user_project(&parsed, env_var, current_dir)?;
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
                write_check: true,
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

fn command_judgments<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let parsed = parse_user_options(args, true, false, 0, current_dir)?;
    let resolved = resolve_user_project(&parsed, env_var, current_dir)?;
    let store = CoreProjectStore::open(
        &resolved.runtime_home,
        &ProjectId::new(&resolved.project_id),
    )?;
    let records = pending_judgment_records_for_task(&store, &parsed.task)?;
    render_judgment_records(&records, parsed.output)
}

fn command_judgment<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Err(UserCommandError::Usage(user_usage()));
    };
    match subcommand {
        "show" => command_judgment_show(&args[1..], env_var, current_dir),
        "answer" => command_judgment_answer(&args[1..], env_var, current_dir),
        "-h" | "--help" | "help" => Ok(user_usage()),
        other => Err(UserCommandError::Usage(format!(
            "unknown user judgment command: {other}\n\n{}",
            user_usage()
        ))),
    }
}

fn command_judgment_show<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let parsed = parse_user_options(args, false, false, 1, current_dir)?;
    let selector = required_positional(&parsed, 0, "INDEX_OR_ID")?;
    let resolved = resolve_user_project(&parsed, env_var, current_dir)?;
    let store = CoreProjectStore::open(
        &resolved.runtime_home,
        &ProjectId::new(&resolved.project_id),
    )?;
    let selected = select_judgment(&store, selector, false)?;
    render_judgment_record(&selected.record, selected.display_index, parsed.output)
}

fn command_judgment_answer<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let parsed = parse_user_options(args, false, true, 2, current_dir)?;
    let judgment_selector = required_positional(&parsed, 0, "INDEX_OR_ID")?;
    let option_selector = required_positional(&parsed, 1, "OPTION_INDEX_OR_ID")?;
    let resolved = resolve_user_project(&parsed, env_var, current_dir)?;
    let store = CoreProjectStore::open(
        &resolved.runtime_home,
        &ProjectId::new(&resolved.project_id),
    )?;
    let state_version = store.project_state()?.state_version;
    let selected_judgment = select_judgment(&store, judgment_selector, true)?;
    let record = selected_judgment.record;
    if record.status != "pending" {
        return Err(UserCommandError::Runtime(format!(
            "selected judgment is not pending (status: {}); refresh `volicord user judgments`",
            record.status
        )));
    }
    let options = decode_options(&record)?;
    let selected_option = select_option(&options, option_selector)?;
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
    render_record_response(&response, parsed.output, &selected_option)
}

fn parse_user_options(
    args: &[String],
    allow_task: bool,
    allow_note: bool,
    max_positionals: usize,
    current_dir: &Path,
) -> Result<ParsedUserOptions, UserCommandError> {
    let options = parse_options(args, allow_task, allow_note)?;
    if options.positionals.len() > max_positionals {
        return Err(UserCommandError::Usage(format!(
            "unexpected argument: {}",
            options.positionals[max_positionals]
        )));
    }
    Ok(ParsedUserOptions {
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
        note: options.value("note"),
        output: if options.value("json").is_some() {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
        positionals: options.positionals,
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

fn parse_options(
    args: &[String],
    allow_task: bool,
    allow_note: bool,
) -> Result<ParsedRawOptions, UserCommandError> {
    let mut parsed = ParsedRawOptions::default();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(UserCommandError::Usage(user_usage()));
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
    parsed: &ParsedUserOptions,
    env_var: F,
    current_dir: &Path,
) -> Result<ResolvedUserProject, UserCommandError>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let repo_root = resolve_repository_root(current_dir, parsed.repo.as_deref())?;
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

fn pending_judgment_records_for_task(
    store: &CoreProjectStore,
    selected: &TaskSelector,
) -> Result<Vec<UserJudgmentRecord>, UserCommandError> {
    let Some(task_id) = selected_or_active_task_id(store, selected)? else {
        return Ok(Vec::new());
    };
    store
        .pending_user_judgment_records(&TaskId::new(task_id))
        .map_err(Into::into)
}

fn select_judgment(
    store: &CoreProjectStore,
    selector: &str,
    require_pending: bool,
) -> Result<SelectedJudgment, UserCommandError> {
    if let Some(index) = parse_positive_index(selector)? {
        let records = pending_judgment_records_for_task(store, &TaskSelector::Active)?;
        let Some(record) = records.get(index - 1).cloned() else {
            return Err(UserCommandError::Usage(format!(
                "judgment number {index} is out of range for the current pending list; run `volicord user judgments` to refresh"
            )));
        };
        if let Some(by_id) = store.user_judgment_record(selector)? {
            if by_id.judgment_id != record.judgment_id {
                return Err(UserCommandError::Usage(format!(
                    "judgment selector `{selector}` is ambiguous: it matches a list number and an explicit judgment id"
                )));
            }
        }
        return Ok(SelectedJudgment {
            record,
            display_index: Some(index),
        });
    }

    let record = store
        .user_judgment_record(selector)?
        .ok_or_else(|| UserCommandError::Runtime("selected judgment was not found".to_owned()))?;
    if require_pending && record.status != "pending" {
        return Err(UserCommandError::Runtime(format!(
            "selected judgment is not pending (status: {}); refresh `volicord user judgments`",
            record.status
        )));
    }
    Ok(SelectedJudgment {
        record,
        display_index: None,
    })
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
            "selected judgment is not pending (status: {}); refresh `volicord user judgments`",
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
    let mut output = String::new();
    output.push_str("User Channel status\n");
    if let Some(summary) = response.response_value["status_summary"].as_str() {
        output.push_str(&format!("summary: {summary}\n"));
    }
    output.push_str(&format!(
        "close_readiness: {}\n",
        response
            .response_value
            .get("close_state")
            .and_then(Value::as_str)
            .unwrap_or("not_available")
    ));
    let close_blockers = response.response_value["close_blockers"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    output.push_str(&format!("close_blockers: {}\n", close_blockers.len()));
    if let Some(blocker) = close_blockers.first() {
        if let Some(code) = blocker.get("code").and_then(Value::as_str) {
            output.push_str(&format!("first_close_blocker: {code}\n"));
        }
        if let Some(action) = blocker
            .get("next_actions")
            .and_then(Value::as_array)
            .and_then(|actions| actions.first())
            .and_then(action_label)
        {
            output.push_str(&format!("next_action: {action}\n"));
        }
    } else {
        output.push_str("next_action: none\n");
    }
    if let Some(guard_health) = response.response_value.get("guard_health") {
        output.push_str(&format!(
            "guard_mode: {}\nguard_strength: {}\nguard_capabilities: {}\nguard_effective_state: {}\nhook_path_safety: {}\nguard_observed: {}\nprompt_capture_state: {}\nprompt_capture_available: {}\nwatcher_status: {}\nwatcher_baseline_created_at: {}\nwatcher_coverage_start_at: {}\nwatcher_coverage_basis: {}\nwatcher_partial_coverage_warning: {}\nunresolved_unrecorded_changes: {}\n",
            text_field(guard_health, "guard_mode", "not_configured"),
            text_field(guard_health, "guard_strength", "not_checked"),
            guard_capabilities_text(guard_health),
            text_field(guard_health, "effective_guard_status", "not_checked"),
            text_field(guard_health, "hook_path_safety", "not_checked"),
            yes_no(
                guard_health
                    .get("guard_hook_observed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            ),
            text_field(guard_health, "prompt_capture_status", "not_checked"),
            yes_no(
                guard_health
                    .get("prompt_capture_available")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            ),
            text_field(guard_health, "session_watch_status", "not_checked"),
            text_field(guard_health, "session_watch_baseline_created_at", "none"),
            text_field(guard_health, "session_watch_coverage_start_at", "none"),
            text_field(guard_health, "session_watch_coverage_basis", "none"),
            text_field(
                guard_health,
                "session_watch_partial_coverage_warning",
                "none"
            ),
            guard_health
                .get("unresolved_unrecorded_change_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ));
    }
    let pending = response.response_value["pending_user_judgments"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    output.push_str(&format!("pending judgments: {}\n", pending.len()));
    if !pending.is_empty() {
        output.push_str(&format!(
            "judgment_path: {}\n",
            judgment_path_text(response.response_value.get("guard_health"))
        ));
    }
    Ok(output)
}

fn action_label(action: &Value) -> Option<&str> {
    action
        .get("blocking_question")
        .and_then(Value::as_str)
        .or_else(|| action.get("label").and_then(Value::as_str))
}

fn text_field<'a>(value: &'a Value, field: &str, fallback: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(fallback)
}

fn bool_field(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).unwrap_or(false)
}

fn guard_capabilities_text(guard_health: &Value) -> String {
    format!(
        "pre_tool_blocking={}, post_tool_correlation={}, hook_path_safety={}, bash_shell_mutation_coverage={}, bypass_detection={}, prompt_capture={}, local_web_consent={}, managed_distribution_verified={}",
        yes_no(bool_field(guard_health, "pre_tool_blocking_available")),
        yes_no(bool_field(guard_health, "post_tool_correlation_available")),
        text_field(guard_health, "hook_path_safety", "not_checked"),
        yes_no(bool_field(guard_health, "bash_shell_mutation_coverage")),
        yes_no(bool_field(guard_health, "bypass_detection_active")),
        yes_no(bool_field(guard_health, "prompt_capture_available")),
        yes_no(bool_field(guard_health, "local_web_consent_available")),
        yes_no(bool_field(guard_health, "managed_distribution_verified")),
    )
}

fn judgment_path_text(guard_health: Option<&Value>) -> &'static str {
    if guard_health
        .and_then(|value| value.get("mcp_connection_healthy"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "use MCP elicitation; terminal user commands are the local recovery path"
    } else if guard_health
        .and_then(|value| value.get("prompt_capture_available"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "use the prompt-capture chat command; terminal user commands are the local recovery path"
    } else {
        "use `volicord user judgments` and `volicord user judgment answer` as the local recovery path"
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn render_judgment_records(
    records: &[UserJudgmentRecord],
    output: OutputFormat,
) -> Result<String, UserCommandError> {
    if output == OutputFormat::Json {
        let values = records
            .iter()
            .enumerate()
            .map(|(index, record)| judgment_record_json(record, Some(index + 1)))
            .collect::<Result<Vec<_>, _>>()?;
        return serde_json::to_string_pretty(&json!({ "pending_user_judgments": values }))
            .map(|text| format!("{text}\n"))
            .map_err(|error| UserCommandError::Runtime(error.to_string()));
    }

    if records.is_empty() {
        return Ok("No pending judgments.\n".to_owned());
    }

    let mut text = String::from("Pending judgments\n");
    for (index, record) in records.iter().enumerate() {
        let request: volicord_types::PersistedUserJudgmentRequest =
            decode_json("request_json", &record.request_json)?;
        let options = decode_options(record)?;
        text.push_str(&format!("{}. {}\n", index + 1, request.question));
        text.push_str(&format!("   kind: {}\n", record.judgment_kind));
        text.push_str("   options:\n");
        for (option_index, option) in options.iter().enumerate() {
            text.push_str(&format!(
                "   {}. {} ({})\n",
                option_index + 1,
                option.label,
                outcome_value(option.resolution_outcome)
            ));
        }
    }
    Ok(text)
}

fn render_judgment_record(
    record: &UserJudgmentRecord,
    display_index: Option<usize>,
    output: OutputFormat,
) -> Result<String, UserCommandError> {
    if output == OutputFormat::Json {
        return serde_json::to_string_pretty(&judgment_record_json(record, display_index)?)
            .map(|text| format!("{text}\n"))
            .map_err(|error| UserCommandError::Runtime(error.to_string()));
    }
    let request: volicord_types::PersistedUserJudgmentRequest =
        decode_json("request_json", &record.request_json)?;
    let context: UserJudgmentContext = decode_json("context_json", &record.context_json)?;
    let options = decode_options(record)?;
    let heading = display_index
        .map(|index| format!("User judgment {index}\n"))
        .unwrap_or_else(|| "User judgment\n".to_owned());
    let mut text = format!(
        "{heading}status: {}\nkind: {}\nquestion: {}\ncontext: {}\n",
        record.status, record.judgment_kind, request.question, context.summary
    );
    text.push_str("options:\n");
    for (index, option) in options.iter().enumerate() {
        text.push_str(&format!(
            "{}. {} ({})\n   {}\n",
            index + 1,
            option.label,
            outcome_value(option.resolution_outcome),
            option.consequence
        ));
    }
    Ok(text)
}

fn render_record_response(
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
    let mut text = String::from("User Channel judgment recorded\n");
    text.push_str(&format!(
        "selected: {}\noutcome: {}\n",
        selected_option.label,
        outcome_value(selected_option.resolution_outcome)
    ));
    Ok(text)
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

fn judgment_record_json(
    record: &UserJudgmentRecord,
    display_index: Option<usize>,
) -> Result<Value, UserCommandError> {
    let request: volicord_types::PersistedUserJudgmentRequest =
        decode_json("request_json", &record.request_json)?;
    let context: UserJudgmentContext = decode_json("context_json", &record.context_json)?;
    let options = decode_options(record)?;
    Ok(json!({
        "index": display_index,
        "project_internal_id": &record.project_id,
        "judgment_id": &record.judgment_id,
        "task_id": &record.task_id,
        "change_unit_id": &record.change_unit_id,
        "judgment_kind": &record.judgment_kind,
        "status": &record.status,
        "basis_status": &record.basis_status,
        "question": request.question,
        "context_summary": context.summary,
        "options": options
            .iter()
            .enumerate()
            .map(|(index, option)| judgment_option_json(index + 1, option))
            .collect::<Vec<_>>(),
        "requested_by_actor_source": &record.requested_by_actor_source,
        "requested_at": &record.requested_at,
    }))
}

fn judgment_option_json(index: usize, option: &UserJudgmentOption) -> Value {
    json!({
        "index": index,
        "option_id": option.option_id.as_str(),
        "label": &option.label,
        "description": &option.description,
        "consequence": &option.consequence,
        "machine_action": option.machine_action,
        "resolution_outcome": option.resolution_outcome,
        "is_default": option.is_default,
    })
}

fn response_kind(response: &PipelineResponse) -> Option<&str> {
    response.response_value["base"]["response_kind"].as_str()
}

fn required_positional<'a>(
    parsed: &'a ParsedUserOptions,
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

fn outcome_value(value: JudgmentResolutionOutcome) -> &'static str {
    match value {
        JudgmentResolutionOutcome::Accepted => "accepted",
        JudgmentResolutionOutcome::Rejected => "rejected",
        JudgmentResolutionOutcome::Deferred => "deferred",
    }
}

fn json_object(value: Value) -> serde_json::Map<String, Value> {
    match value {
        Value::Object(object) => object,
        _ => serde_json::Map::new(),
    }
}
