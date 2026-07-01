use std::{
    collections::BTreeMap,
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use volicord_core::{CorePipelineError, CoreService, InvocationContext, PipelineResponse};
use volicord_store::{
    core_pipeline::CoreProjectStore,
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};
use volicord_types::{
    ActorSource, IdempotencyKey, OperationCategory, ProjectId, ReconcileChangesRequest, RequestId,
    TaskId, ToolEnvelope, VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
};

use crate::project_context::{
    registered_project_for_repo, resolve_repository_root, ProjectCommandError,
};

type RawOptions = BTreeMap<String, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangesCommandError {
    Usage(String),
    Runtime(String),
}

impl fmt::Display for ChangesCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ChangesCommandError {}

impl From<StoreError> for ChangesCommandError {
    fn from(error: StoreError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for ChangesCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<CorePipelineError> for ChangesCommandError {
    fn from(error: CorePipelineError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<ProjectCommandError> for ChangesCommandError {
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
struct ParsedChangesOptions {
    repo: Option<PathBuf>,
    task_id: Option<String>,
    output: OutputFormat,
}

pub fn changes_usage() -> String {
    "volicord changes reconcile [--repo PATH] [--task active|ID] [--json]\n".to_owned()
}

pub fn run_changes_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, ChangesCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Ok(changes_usage());
    };
    match subcommand {
        "reconcile" => command_reconcile(&args[1..], env_var, current_dir),
        "-h" | "--help" | "help" => Ok(changes_usage()),
        other => Err(ChangesCommandError::Usage(format!(
            "unknown changes command: {other}\n\n{}",
            changes_usage()
        ))),
    }
}

fn command_reconcile<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<String, ChangesCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let parsed = parse_changes_options(args, current_dir)?;
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let repo_root = resolve_repository_root(current_dir, parsed.repo.as_deref())?;
    let project = registered_project_for_repo(&runtime_home, &repo_root)?;
    let project_id = ProjectId::new(project.project_id.clone());
    let store = CoreProjectStore::open(&runtime_home, &project_id)?;
    let task_id = match parsed.task_id.as_deref() {
        Some("active") | None => store
            .active_task_record()?
            .map(|task| task.task_id)
            .ok_or_else(|| ChangesCommandError::Runtime("no active Task for project".to_owned()))?,
        Some(value) if value.trim().is_empty() => {
            return Err(ChangesCommandError::Usage(
                "--task must not be empty".to_owned(),
            ))
        }
        Some(value) => value.to_owned(),
    };
    let state_version = store.project_state()?.state_version;
    let response = CoreService::new(&runtime_home).reconcile_changes(
        ReconcileChangesRequest {
            envelope: ToolEnvelope {
                project_id: project_id.clone(),
                task_id: Some(TaskId::new(task_id.clone())).into(),
                request_id: RequestId::new(generated_id("req_changes_reconcile")),
                idempotency_key: Some(IdempotencyKey::new(generated_id("idem_changes_reconcile")))
                    .into(),
                expected_state_version: Some(state_version).into(),
                dry_run: false,
                locale: None.into(),
            },
            task_id: TaskId::new(task_id),
            resolution_requests: Vec::new(),
        },
        InvocationContext::new(
            project_id,
            ActorSource::LocalUser,
            OperationCategory::AgentWorkflow,
            VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
        ),
    )?;
    render_reconcile_response(&response, parsed.output)
}

fn parse_changes_options(
    args: &[String],
    current_dir: &Path,
) -> Result<ParsedChangesOptions, ChangesCommandError> {
    let mut raw = RawOptions::new();
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        if token == "-h" || token == "--help" || token == "help" {
            return Err(ChangesCommandError::Usage(changes_usage()));
        }
        if token == "--json" {
            set_option(&mut raw, "json", "true".to_owned())?;
        } else if token == "--repo" {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(ChangesCommandError::Usage(
                    "missing value for --repo".to_owned(),
                ));
            };
            set_nonempty_option(&mut raw, "repo", value)?;
        } else if let Some(value) = token.strip_prefix("--repo=") {
            set_nonempty_option(&mut raw, "repo", value)?;
        } else if token == "--task" {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(ChangesCommandError::Usage(
                    "missing value for --task".to_owned(),
                ));
            };
            set_nonempty_option(&mut raw, "task", value)?;
        } else if let Some(value) = token.strip_prefix("--task=") {
            set_nonempty_option(&mut raw, "task", value)?;
        } else if token.starts_with("--") {
            return Err(ChangesCommandError::Usage(format!(
                "unknown option: {token}"
            )));
        } else {
            return Err(ChangesCommandError::Usage(format!(
                "unexpected argument: {token}"
            )));
        }
        index += 1;
    }
    Ok(ParsedChangesOptions {
        repo: option_value(&raw, "repo")
            .map(PathBuf::from)
            .map(|path| absolute_path(current_dir, path)),
        task_id: option_value(&raw, "task"),
        output: if option_value(&raw, "json").is_some() {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
    })
}

fn set_option(
    options: &mut RawOptions,
    name: &'static str,
    value: String,
) -> Result<(), ChangesCommandError> {
    let values = options.entry(name.to_owned()).or_default();
    if !values.is_empty() {
        return Err(ChangesCommandError::Usage(format!(
            "--{name} was supplied more than once"
        )));
    }
    values.push(value);
    Ok(())
}

fn set_nonempty_option(
    options: &mut RawOptions,
    name: &'static str,
    value: &str,
) -> Result<(), ChangesCommandError> {
    if value.trim().is_empty() {
        return Err(ChangesCommandError::Usage(format!(
            "--{name} must not be empty"
        )));
    }
    set_option(options, name, value.to_owned())
}

fn option_value(options: &RawOptions, name: &str) -> Option<String> {
    options.get(name).and_then(|values| values.first()).cloned()
}

fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

fn render_reconcile_response(
    response: &PipelineResponse,
    output: OutputFormat,
) -> Result<String, ChangesCommandError> {
    if output == OutputFormat::Json
        || response.response_value["base"]["response_kind"].as_str() == Some("rejected")
    {
        return serde_json::to_string_pretty(&response.response_value)
            .map(|value| format!("{value}\n"))
            .map_err(|error| ChangesCommandError::Runtime(error.to_string()));
    }
    let resolved = response.response_value["resolved_changes"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    let pending = response.response_value["pending_user_judgment_refs"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    let unresolved = response.response_value["unresolved_changes"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    let mut output = format!(
        "reconciled changes: {resolved} resolved, {pending} pending user judgment(s), {unresolved} unresolved\n"
    );
    if pending > 0 {
        output.push_str("next: run `volicord user judgments` and answer the pending judgment, then run `volicord changes reconcile` again\n");
    }
    Ok(output)
}

fn generated_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}_{nanos}")
}
