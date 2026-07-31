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
use volicord_types::ids::{IdempotencyKey, ProjectId, RequestId, TaskId};
use volicord_types::methods::{
    ReconcileChangesRequest, ReconcileChangesResponse, ReconcileChangesResult,
};
use volicord_types::schema::{DryRunSummary, PreviewableToolResponse, ToolEnvelope};
use volicord_types::values::{OperationCategory, UserActionChannelKind};

use crate::mutation_admission::{with_cli_runtime_home_mutation, CliMutationAdmissionError};
use crate::presentation::{
    ActionHint, BulletList, CollectionItem, Document, Field, HumanValue, Section,
};
use crate::project_context::{
    registered_project_for_repo_admitted, resolve_repository_root, ProjectCommandError,
};
use volicord_command_model::{ChangesArgs, ChangesCommand, ChangesReconcileArgs};

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
        command_reconcile_admitted(context, &repo_root, &options)
            .map_err(|error| CliMutationAdmissionError::Operation(error.to_string()))
    })
    .map_err(Into::into)
}

fn command_reconcile_admitted(
    context: &RuntimeHomeMutationContext<'_>,
    repo_root: &Path,
    options: &ChangesReconcileArgs,
) -> Result<String, ChangesCommandError> {
    let project = registered_project_for_repo_admitted(context, repo_root)?;
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
    let response = CoreService::for_mutation(context).reconcile_changes(
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
                dry_run: volicord_types::schema::DryRunIntent::from_wire_bool(options.dry_run),
                locale: None.into(),
            },
            task_id: TaskId::new(task_id),
            resolution_requests: Vec::new(),
        },
        InvocationContext::local_user(
            project_id,
            OperationCategory::LocalRecovery,
            UserActionChannelKind::Cli,
        ),
    )?;
    let typed_response =
        serde_json::from_value::<ReconcileChangesResponse>(response.response_value.clone())
            .map_err(|error| ChangesCommandError::Runtime(error.to_string()))?;
    render_reconcile_response(
        &response,
        &typed_response,
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
    typed_response: &ReconcileChangesResponse,
    output: OutputFormat,
) -> Result<String, ChangesCommandError> {
    if output == OutputFormat::Json {
        return serde_json::to_string_pretty(&response.response_value)
            .map(|value| format!("{value}\n"))
            .map_err(|error| ChangesCommandError::Runtime(error.to_string()));
    }
    match typed_response {
        PreviewableToolResponse::Rejected(_) => {
            let rendered = serde_json::to_string_pretty(&response.response_value)
                .map(|value| format!("{value}\n"))
                .map_err(|error| ChangesCommandError::Runtime(error.to_string()))?;
            Err(ChangesCommandError::FailureOutput(rendered))
        }
        PreviewableToolResponse::DryRun(response) => {
            Ok(render_reconcile_dry_run_text(response.dry_run_summary()))
        }
        PreviewableToolResponse::Result(result) => Ok(render_reconcile_result_text(result)),
    }
}

fn render_reconcile_result_text(result: &ReconcileChangesResult) -> String {
    Document::new(
        "Changes reconciliation",
        vec![
            Field::new(
                "Unrecorded changes",
                HumanValue::Count(result.unresolved_changes.len()),
            )
            .into(),
            Field::new(
                "Resolved changes",
                HumanValue::Count(result.resolved_changes.len()),
            )
            .into(),
            Field::new(
                "Pending user actions",
                HumanValue::Count(result.pending_user_action_summaries.len()),
            )
            .into(),
            Field::new(
                "Close readiness blockers",
                HumanValue::Count(result.close_blockers.len()),
            )
            .into(),
            ActionHint::new(&result.summary_card.next).into(),
        ],
    )
    .render()
}

fn render_reconcile_dry_run_text(summary: &DryRunSummary) -> String {
    let mut body = Vec::new();
    if summary.planned_effects.is_empty() {
        body.push(Field::new("Planned effects", HumanValue::None).into());
    } else {
        body.push(
            Section::new(
                "Planned effects",
                summary
                    .planned_effects
                    .iter()
                    .map(|effect| {
                        CollectionItem::new(
                            format!("{}.{}", effect.target_kind, effect.action),
                            vec![Field::new(
                                "Description",
                                HumanValue::text(&effect.description),
                            )],
                        )
                        .into()
                    })
                    .collect(),
            )
            .into(),
        );
    }
    body.push(
        Field::new(
            "Close readiness blockers that would remain",
            HumanValue::Count(summary.would_blockers.len()),
        )
        .into(),
    );
    if !summary.would_blockers.is_empty() {
        body.push(
            Section::new(
                "Blocker codes",
                vec![BulletList::new(
                    summary
                        .would_blockers
                        .iter()
                        .map(|blocker| blocker.code.as_str()),
                )
                .into()],
            )
            .into(),
        );
    }
    if !summary.diagnostics.is_empty() {
        body.push(
            Section::new(
                "Diagnostics",
                vec![BulletList::new(summary.diagnostics.iter().map(String::as_str)).into()],
            )
            .into(),
        );
    }
    body.push(
        Field::new(
            "Projected next actions",
            HumanValue::Count(summary.next_actions.len()),
        )
        .into(),
    );
    if !summary.next_actions.is_empty() {
        body.push(
            Section::new(
                "Next actions",
                vec![BulletList::new(
                    summary
                        .next_actions
                        .iter()
                        .map(|action| action.label.as_str()),
                )
                .into()],
            )
            .into(),
        );
    }
    body.push(
        Section::new(
            "Output scope",
            vec![Field::new(
                "Does not prove",
                HumanValue::text(
                    "actor identity, intent, correctness, test sufficiency, human review completion, or that a product-file write occurred",
                ),
            )
            .into()],
        )
        .into(),
    );
    Document::new("Changes reconciliation (dry run)", body).render()
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
    use std::{error::Error, ffi::OsString, fs};

    use volicord_test_support::{core_fixtures::CoreFixture, seed_test_agent_session};
    use volicord_types::ids::{AgentConnectionId, AgentRuntimeSessionId, AgentSessionId};
    use volicord_types::schema::{GuaranteeDisclosure, ToolError, ToolRejectedResponse};
    use volicord_types::values::ErrorCode;

    use super::*;

    #[test]
    fn rejected_reconcile_response_is_failure_output() {
        let response_value = serde_json::to_value(ToolRejectedResponse::new(
            volicord_types::schema::DryRunIntent::NotRequested,
            Some(3),
            GuaranteeDisclosure::authority_record(),
            vec![ToolError::new(
                ErrorCode::InvocationContextMismatch,
                "invocation context does not match Core preflight requirements",
                false,
                None,
            )],
        ))
        .expect("typed rejection should serialize");
        let response = PipelineResponse {
            response_json: response_value.to_string(),
            response_value: response_value.clone(),
            operation_result_ref: None,
            verified_invocation: None,
            resolved_task_id: None,
            replayed: false,
        };
        let typed_response: ReconcileChangesResponse =
            serde_json::from_value(response_value).expect("typed rejection should deserialize");

        let error = render_reconcile_response(&response, &typed_response, OutputFormat::Text)
            .expect_err("rejected Core response should fail the command");
        match error {
            ChangesCommandError::FailureOutput(output) => {
                assert!(output.contains("\"response_kind\": \"rejected\""));
                assert!(output.contains("INVOCATION_CONTEXT_MISMATCH"));
                let projected: serde_json::Value =
                    serde_json::from_str(&output).expect("CLI failure output should be JSON");
                assert_eq!(
                    projected["errors"][0]["code"],
                    "INVOCATION_CONTEXT_MISMATCH"
                );
                assert_eq!(projected["errors"][0]["category"], "rejected");
                assert_eq!(projected["errors"][0]["details"], serde_json::Value::Null);
            }
            other => panic!("expected failure output, got {other:?}"),
        }
    }

    #[test]
    fn changes_reconcile_uses_the_admitted_lexical_runtime_home() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("cli-changes-reconcile-lexical-alias")?;
        fs::create_dir_all(fixture.product_repo_path().join(".git"))?;
        let session = seed_test_agent_session(
            fixture.runtime_home_path(),
            fixture.project_id(),
            fixture.connection_id(),
            None,
        )?;
        let validated = CoreService::for_read_only(fixture.runtime_home_path())
            .validate_agent_session(
                AgentConnectionId::new(fixture.connection_id()),
                ProjectId::new(fixture.project_id()),
                AgentRuntimeSessionId::new(session.runtime_session_id.as_str()),
                AgentSessionId::new(session.project_session_id.as_str()),
                OperationCategory::AgentWorkflow,
            )?;
        let mutation_context = fixture.mutation_context()?;
        let intake = CoreService::for_mutation(&mutation_context).intake(
            &mutation_context,
            fixture.intake_request(
                "req_cli_changes_alias_intake",
                "idem_cli_changes_alias_intake",
                false,
                Some(0),
            ),
            InvocationContext::agent_connection(OperationCategory::AgentWorkflow, validated),
        )?;
        assert_eq!(intake.response_value["base"]["response_kind"], "result");

        let runtime_home = fixture.runtime_home_path();
        let alias = runtime_home
            .parent()
            .expect("fixture Runtime Home has a parent")
            .join(".")
            .join(
                runtime_home
                    .file_name()
                    .expect("fixture Runtime Home has a file name"),
            );
        let output = run_changes_command(
            ChangesArgs {
                command: ChangesCommand::Reconcile(ChangesReconcileArgs {
                    repo: Some(fixture.product_repo_path()),
                    task: "active".to_owned(),
                    dry_run: true,
                    json: true,
                }),
            },
            |name| (name == "VOLICORD_HOME").then(|| OsString::from(&alias)),
            &fixture.product_repo_path(),
        )?;
        let response: serde_json::Value = serde_json::from_str(&output)?;

        assert_eq!(response["base"]["response_kind"], "dry_run");
        Ok(())
    }
}
