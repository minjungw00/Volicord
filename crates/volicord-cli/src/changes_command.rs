use std::{
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use volicord_core::{CorePipelineError, CoreService, InvocationContext, PipelineResponse};
use volicord_store::{
    core_pipeline::CoreProjectStore,
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    RuntimeHomeMutationContext, StoreError,
};
use volicord_types::{
    ActorSource, IdempotencyKey, OperationCategory, ProjectId, ReconcileChangesRequest, RequestId,
    TaskId, ToolEnvelope, VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
};

use crate::cli::{ChangesArgs, ChangesCommand, ChangesReconcileArgs};
use crate::disclosure::does_not_prove_line;
use crate::mutation_admission::{with_cli_runtime_home_mutation, CliMutationAdmissionError};
use crate::project_context::{
    registered_project_for_repo, resolve_repository_root, ProjectCommandError,
};
use crate::summary_card::{
    render_close_and_next_action_totals_text, render_summary_card_text, summary_card_from_response,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangesCommandError {
    Usage(String),
    Runtime(String),
    FailureOutput(String),
    MutationAdmission(CliMutationAdmissionError),
}

impl fmt::Display for ChangesCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) | Self::FailureOutput(message) => {
                formatter.write_str(message)
            }
            Self::MutationAdmission(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ChangesCommandError {}

impl From<CliMutationAdmissionError> for ChangesCommandError {
    fn from(error: CliMutationAdmissionError) -> Self {
        Self::MutationAdmission(error)
    }
}

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

pub fn run_changes_command<F>(
    args: ChangesArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, ChangesCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    match args.command {
        ChangesCommand::Reconcile(options) => command_reconcile(options, env_var, current_dir),
    }
}

fn command_reconcile<F>(
    options: ChangesReconcileArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<String, ChangesCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let repo = options
        .repo
        .as_ref()
        .map(|path| absolute_path(current_dir, path.clone()));
    let repo_root = resolve_repository_root(current_dir, repo.as_deref())?;
    with_cli_runtime_home_mutation(&runtime_home, "cli.changes.reconcile", |context| {
        command_reconcile_admitted(context, &runtime_home, &repo_root, &options)
            .map_err(|error| CliMutationAdmissionError::Operation(error.to_string()))
    })
    .map_err(Into::into)
}

fn command_reconcile_admitted(
    context: &RuntimeHomeMutationContext<'_>,
    runtime_home: &Path,
    repo_root: &Path,
    options: &ChangesReconcileArgs,
) -> Result<String, ChangesCommandError> {
    let project = registered_project_for_repo(&runtime_home, &repo_root)?;
    let project_id = ProjectId::new(project.project_id.clone());
    let store = CoreProjectStore::open_for_mutation(context, &project_id)?;
    let task_id = match options.task.as_str() {
        "active" => store
            .active_task_record()?
            .map(|task| task.task_id)
            .ok_or_else(|| ChangesCommandError::Runtime("no active Task for project".to_owned()))?,
        value => value.to_owned(),
    };
    let state_version = store.project_state()?.state_version;
    let response = CoreService::new(&runtime_home).reconcile_changes(
        context,
        ReconcileChangesRequest {
            envelope: ToolEnvelope {
                project_id: project_id.clone(),
                task_id: Some(TaskId::new(task_id.clone())).into(),
                request_id: RequestId::new(generated_id("req_changes_reconcile")),
                idempotency_key: if options.dry_run {
                    None.into()
                } else {
                    Some(IdempotencyKey::new(generated_id("idem_changes_reconcile"))).into()
                },
                expected_state_version: Some(state_version).into(),
                dry_run: options.dry_run,
                locale: None.into(),
            },
            task_id: TaskId::new(task_id),
            resolution_requests: Vec::new(),
        },
        InvocationContext::new(
            project_id,
            ActorSource::LocalUser,
            OperationCategory::LocalRecovery,
            VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
        ),
    )?;
    render_reconcile_response(
        &response,
        if options.json {
            OutputFormat::Json
        } else {
            OutputFormat::Text
        },
    )
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
    if response.response_value["base"]["response_kind"].as_str() == Some("rejected") {
        let rendered = serde_json::to_string_pretty(&response.response_value)
            .map(|value| format!("{value}\n"))
            .map_err(|error| ChangesCommandError::Runtime(error.to_string()))?;
        return Err(ChangesCommandError::FailureOutput(rendered));
    }
    if output == OutputFormat::Json {
        return serde_json::to_string_pretty(&response.response_value)
            .map(|value| format!("{value}\n"))
            .map_err(|error| ChangesCommandError::Runtime(error.to_string()));
    }
    if response.response_value["base"]["response_kind"].as_str() == Some("dry_run") {
        return Ok(render_reconcile_dry_run_text(&response.response_value));
    }
    let mut output = String::from("Changes reconciliation\n");
    if let Some(card) = summary_card_from_response(&response.response_value) {
        output.push_str(&render_summary_card_text(&card));
    }
    output.push_str(&render_close_and_next_action_totals_text(
        &response.response_value,
    ));
    Ok(output)
}

fn render_reconcile_dry_run_text(value: &serde_json::Value) -> String {
    let mut output = String::from("Changes reconciliation (dry run)\n");
    let summary = &value["dry_run_summary"];
    let planned_effects = summary["planned_effects"]
        .as_array()
        .map(|values| values.as_slice())
        .unwrap_or(&[]);
    if planned_effects.is_empty() {
        output.push_str("Planned: none\n");
    } else {
        for effect in planned_effects {
            let target = text_value(effect.get("target_kind"));
            let action = text_value(effect.get("action"));
            let description = text_value(effect.get("description"));
            output.push_str(&format!("Planned: {target}.{action}: {description}\n"));
        }
    }
    let blockers = summary["would_blockers"]
        .as_array()
        .map(|values| values.as_slice())
        .unwrap_or(&[]);
    output.push_str(&format!(
        "Close readiness blockers that would remain (total): {}\n",
        blockers.len()
    ));
    if !blockers.is_empty() {
        let codes = blockers
            .iter()
            .map(|blocker| text_value(blocker.get("code")))
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!(
            "Close readiness blocker codes that would remain: {codes}\n"
        ));
    }
    for diagnostic in summary["diagnostics"]
        .as_array()
        .map(|values| values.as_slice())
        .unwrap_or(&[])
        .iter()
        .filter_map(serde_json::Value::as_str)
    {
        output.push_str(&format!("Diagnostic: {diagnostic}\n"));
    }
    let next_actions = summary["next_actions"]
        .as_array()
        .map(|values| values.as_slice())
        .unwrap_or(&[]);
    output.push_str(&format!(
        "Projected next actions (total): {}\n",
        next_actions.len()
    ));
    for action in next_actions {
        output.push_str(&format!(
            "Projected next action: {}\n",
            text_value(action.get("label"))
        ));
    }
    output.push_str(&does_not_prove_line(
        "actor identity proof, intent proof, correctness proof, test sufficiency proof, human review completion, or that a product-file write occurred",
    ));
    output
}

fn text_value(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(value) => value.to_string(),
        None => "unknown".to_owned(),
    }
}

fn generated_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}_{nanos}")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn rejected_reconcile_response_is_failure_output() {
        let response_value = json!({
            "base": {
                "response_kind": "rejected",
                "effect_kind": "no_effect",
                "dry_run": false,
                "state_version": 3,
                "events": []
            },
            "errors": [{
                "code": "INVOCATION_CONTEXT_MISMATCH",
                "message": "invocation context does not match Core preflight requirements",
                "retryable": false,
                "details": {}
            }]
        });
        let response = PipelineResponse {
            response_json: response_value.to_string(),
            response_value,
            operation_result_ref: None,
            verified_invocation: None,
            resolved_task_id: None,
            replayed: false,
        };

        let error = render_reconcile_response(&response, OutputFormat::Text)
            .expect_err("rejected Core response should fail the command");
        match error {
            ChangesCommandError::FailureOutput(output) => {
                assert!(output.contains("\"response_kind\": \"rejected\""));
                assert!(output.contains("INVOCATION_CONTEXT_MISMATCH"));
            }
            other => panic!("expected failure output, got {other:?}"),
        }
    }
}
