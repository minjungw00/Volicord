use std::{
    collections::BTreeSet,
    ffi::OsString,
    fmt, fs,
    path::Path,
    time::{Instant, SystemTime},
};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use volicord_core::{Clock, CorePipelineError, SystemClock};
use volicord_host_contract::{HostContractProfileId, HostNativeCorrelation};
use volicord_store::{
    bootstrap::{
        project_record_for_execution, project_record_for_execution_admitted, ProjectRecord,
    },
    core_pipeline::CoreProjectStore,
    diagnostic_findings::insert_occurrence_finding,
    diagnostics::{
        record_diagnostic_event, record_workflow_metric_event, start_diagnostic_session,
        DiagnosticEvent, DiagnosticEventKind, DiagnosticHostKind, DiagnosticOutcome,
        DiagnosticSessionStart, DiagnosticTransport, DiagnosticUserChannelKind,
        WorkflowMetricDecision, WorkflowMetricEvent, WorkflowMetricKind, WorkflowMetricOutcome,
    },
    guards::{
        current_project_agent_session_coordinates, guard_event, guard_installation,
        insert_guard_event, observe_host_correlation, GuardEventInsert, HostCorrelationObservation,
    },
    integration_verification::{
        observe_guard_probe_hook_event, observe_unbound_guard_probe_hook_event,
        GuardProbeHookEvidence, UnboundGuardProbeHookObservation,
    },
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    RuntimeHomeMutationContext, StoreError, StoreResult,
};
use volicord_types::{
    canonical_json_bare_sha256, canonical_json_bytes, guard_manifest_from_json, DiagnosticCode,
    DiagnosticDomain, DiagnosticFacts, DiagnosticFindingData, DiagnosticSeverity, DiagnosticSource,
    DiagnosticStage, DiagnosticSubject, GuardDecision, GuardHookContractStatus,
    GuardHookDiagnostic, GuardHookDiagnosticCode, GuardHookDiagnosticFacts, GuardHookOutcome,
    GuardHookPhase, GuardHostFeedback, GuardManagedArtifact, GuardObservationOutcome,
    GuardPolicyDecision, IntegrationProfile, ObservationConfidence, OccurrenceDiagnosticFinding,
    UtcTimestamp,
};

use crate::cli::{HookArgs, HookCommand};
use crate::disclosure::cooperative_host_decision_disclosure_json;
use crate::mutation_admission::{with_cli_runtime_home_mutation, CliMutationAdmissionError};
use crate::project_context::{
    registered_project_for_repo, resolve_repository_root, ProjectCommandError,
};
const DEFAULT_INTEGRATION_PROFILE: &str = "record";
const EXPECTED_WRITE_TTL_MINUTES: i64 = 15;

mod args;
mod codex_output;
mod context;
mod envelope;
mod mutation;
mod phase;
mod prompt_capture;
mod render;
mod tool_observation;
mod write_ticket;

use args::{guard_options, read_guard_input, GuardInput, GuardOptions};
use envelope::{
    event_path_field, event_string, guard_envelope, is_managed_builtin_host, GuardEnvelope,
    GuardEnvelopeError,
};
use phase::{pre_tool::persist_expected_write, GuardPhaseResult};
use prompt_capture::handle_prompt_capture;
use render::{render_guard_output, RenderedGuardOutput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardCommandOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardCommandError {
    Usage(String),
    Runtime(String),
    Persistence(String),
}

impl fmt::Display for GuardCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) | Self::Persistence(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for GuardCommandError {}

impl From<StoreError> for GuardCommandError {
    fn from(error: StoreError) -> Self {
        Self::Persistence(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for GuardCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<ProjectCommandError> for GuardCommandError {
    fn from(error: ProjectCommandError) -> Self {
        match error {
            ProjectCommandError::Usage(message) => Self::Usage(message),
            ProjectCommandError::Runtime(message) => Self::Runtime(message),
            ProjectCommandError::MutationAdmission(error) => Self::Persistence(error.to_string()),
        }
    }
}

impl From<CorePipelineError> for GuardCommandError {
    fn from(error: CorePipelineError) -> Self {
        Self::Runtime(error.to_string())
    }
}

fn core_current_timestamp(store: &CoreProjectStore) -> StoreResult<UtcTimestamp> {
    SystemClock.project_now(store)
}

#[cfg(test)]
mod admission_tests {
    use volicord_test_support::{core_fixtures::CoreFixture, TestRuntimeHomeSetup};

    use super::*;
    use crate::cli::{HookEventArgs, HookOutput};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct GuardMutationSnapshot {
        guard_events: i64,
        prompt_captures: i64,
        expected_writes: i64,
        unrecorded_changes: i64,
        state_version: i64,
    }

    fn guard_mutation_snapshot(
        fixture: &CoreFixture,
    ) -> Result<GuardMutationSnapshot, Box<dyn std::error::Error>> {
        let conn = fixture.conn()?;
        Ok(GuardMutationSnapshot {
            guard_events: conn
                .query_row("SELECT COUNT(*) FROM guard_events", [], |row| row.get(0))?,
            prompt_captures: conn
                .query_row("SELECT COUNT(*) FROM prompt_captures", [], |row| row.get(0))?,
            expected_writes: conn
                .query_row("SELECT COUNT(*) FROM expected_writes", [], |row| row.get(0))?,
            unrecorded_changes: conn.query_row(
                "SELECT COUNT(*) FROM unrecorded_changes",
                [],
                |row| row.get(0),
            )?,
            state_version: conn.query_row(
                "SELECT state_version FROM project_state",
                [],
                |row| row.get(0),
            )?,
        })
    }

    #[test]
    fn record_hook_continues_without_persisting_while_setup_is_exclusive(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut fixture = CoreFixture::new("guard-record-setup-busy")?;
        fs::create_dir(fixture.product_repo_path().join(".git"))?;
        let policy_path = GuardManagedArtifact::VolicordPolicy
            .expected_path(&fixture.product_repo_path(), None)
            .expect("fixture Guard policy path");
        fs::create_dir_all(
            policy_path
                .parent()
                .expect("fixture Guard policy has a parent"),
        )?;
        fs::write(&policy_path, "{}")?;
        let policy_hash = sha256_text("{}");
        let guard_installation_id = "guard_setup_busy";
        volicord_store::guards::upsert_guard_installation(
            &fixture.mutation_context()?,
            volicord_store::guards::GuardInstallationUpsert {
                guard_installation_id: guard_installation_id.to_owned(),
                connection_internal_id: fixture.connection_id().to_owned(),
                project_id: fixture.project_id().to_owned(),
                manifest_json: volicord_test_support::test_guard_manifest_json(
                    fixture.runtime_home_path(),
                    &fixture.product_repo_path(),
                    fixture.project_id(),
                    fixture.connection_id(),
                    guard_installation_id,
                    &policy_hash,
                ),
            },
        )?;
        let event_path = fixture.product_repo_path().join("guard-event.json");
        fs::write(&event_path, "{}")?;
        let before = guard_mutation_snapshot(&fixture)?;
        fixture.release_mutation_admission();
        let setup = TestRuntimeHomeSetup::acquire(fixture.runtime_home_path())?;
        let env = |name: &str| {
            (name == "VOLICORD_HOME").then(|| OsString::from(fixture.runtime_home_path()))
        };

        let outcome = run_guard_command(
            HookArgs {
                command: HookCommand::PreTool(HookEventArgs {
                    event_file: Some(event_path.clone()),
                    repo: Some(fixture.product_repo_path()),
                    connection: Some(fixture.connection_id().to_owned()),
                    guard_installation: Some(guard_installation_id.to_owned()),
                    host: None,
                    integration_profile: None,
                    policy_hash: Some(policy_hash.clone()),
                    output: Some(HookOutput::VolicordJson),
                    host_output: None,
                }),
            },
            env,
            fixture.product_repo_path().as_path(),
        )?;

        assert_eq!(outcome.exit_code, 0);
        assert!(outcome
            .stdout
            .contains("runtime_home.mutation.setup_in_progress"));
        assert!(outcome.stdout.contains("\"persisted\": false"));
        assert!(!outcome.stdout.contains("\"decision\":\"deny\""));
        assert!(outcome
            .stdout
            .contains("guard.event.persistence_unavailable"));
        assert!(!outcome
            .stdout
            .contains("\"observation_outcome\": \"observed\""));
        assert_eq!(guard_mutation_snapshot(&fixture)?, before);
        drop(setup);

        let retry = run_guard_command(
            HookArgs {
                command: HookCommand::PreTool(HookEventArgs {
                    event_file: Some(event_path),
                    repo: Some(fixture.product_repo_path()),
                    connection: Some(fixture.connection_id().to_owned()),
                    guard_installation: Some(guard_installation_id.to_owned()),
                    host: None,
                    integration_profile: None,
                    policy_hash: Some(policy_hash),
                    output: Some(HookOutput::VolicordJson),
                    host_output: None,
                }),
            },
            env,
            fixture.product_repo_path().as_path(),
        )?;
        assert_eq!(retry.exit_code, 0);
        assert!(
            retry
                .stdout
                .contains("\"observation_outcome\": \"incompatible_recorded\""),
            "{}",
            retry.stdout
        );
        assert!(!retry
            .stdout
            .contains("\"observation_outcome\": \"observed\""));
        let after_retry = guard_mutation_snapshot(&fixture)?;
        assert_eq!(after_retry.guard_events, before.guard_events + 1);
        assert_eq!(after_retry.prompt_captures, before.prompt_captures);
        assert_eq!(after_retry.expected_writes, before.expected_writes);
        assert_eq!(after_retry.unrecorded_changes, before.unrecorded_changes);
        assert_eq!(after_retry.state_version, before.state_version);
        Ok(())
    }
}

pub fn run_guard_command<F>(
    args: HookArgs,
    env_var: F,
    current_dir: &Path,
) -> Result<GuardCommandOutcome, GuardCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let (phase, options) = match args.command {
        HookCommand::PreTool(options) => (GuardHookPhase::PreTool, options),
        HookCommand::PostTool(options) => (GuardHookPhase::PostTool, options),
        HookCommand::PromptCapture(options) => (GuardHookPhase::PromptCapture, options),
    };
    let diagnostic_started = Instant::now();
    let options = guard_options(options);
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let input = read_guard_input(options.event_file.as_deref())?;
    let project = resolve_guard_project(&runtime_home, current_dir, &options, &input.raw_value)?;
    let output_format = options.output;
    let admitted =
        with_cli_runtime_home_mutation(&runtime_home, "guard.hook_observation", |context| {
            (|| -> Result<GuardCommandOutcome, GuardCommandError> {
                let project = project_record_for_execution_admitted(context, &project.project_id)?
                    .ok_or_else(|| {
                        GuardCommandError::Runtime(format!(
                            "project not found after Runtime Home mutation admission: {}",
                            project.project_id
                        ))
                    })?;
                let mut envelope = match guard_envelope(phase, &options, &input, &project) {
                    Ok(envelope) => envelope,
                    Err(failure) => {
                        let outcome = match record_guard_hook_contract_failure(
                            context,
                            &project,
                            phase,
                            &options,
                            &input,
                            GuardHookContractStatus::Malformed,
                            failure,
                        ) {
                            Ok(facts) => GuardHookOutcome::new(
                                GuardObservationOutcome::IncompatibleRecorded,
                                None,
                                [GuardHookDiagnostic {
                                    code: GuardHookDiagnosticCode::HostContractIncompatible,
                                    facts,
                                }],
                                Some(GuardHostFeedback::Warning),
                            ),
                            Err(_) => GuardHookOutcome::new(
                                GuardObservationOutcome::PersistenceUnavailable,
                                None,
                                [GuardHookDiagnostic {
                                    code: GuardHookDiagnosticCode::EventPersistenceUnavailable,
                                    facts: guard_diagnostic_facts(
                                        phase, &options, None, None, None,
                                    ),
                                }],
                                Some(GuardHostFeedback::Warning),
                            ),
                        };
                        record_guard_findings_best_effort(context, &outcome);
                        let result = hook_outcome_result(&outcome);
                        let rendered = render_guard_command_output(
                            context, phase, &outcome, None, result, &options,
                        )?;
                        return Ok(GuardCommandOutcome {
                            stdout: rendered.stdout,
                            stderr: rendered.stderr,
                            exit_code: rendered.exit_code,
                        });
                    }
                };
                if let Err(error) =
                    bind_guard_envelope(context, &project, phase, &input, &mut envelope)
                {
                    if matches!(phase, GuardHookPhase::PreTool | GuardHookPhase::PostTool) {
                        if let Some(guard_installation_id) = envelope.guard_installation_id.clone()
                        {
                            let _ = observe_unbound_guard_probe_hook_event(
                                context,
                                &project.project_id,
                                UnboundGuardProbeHookObservation {
                                    connection_internal_id: envelope.connection_id.clone(),
                                    guard_installation_id,
                                    correlation: envelope.correlation.clone(),
                                    phase,
                                    evidence: guard_probe_hook_evidence(&input),
                                    observed_at: envelope.occurred_at.clone(),
                                },
                            );
                        }
                    }
                    return host_guard_failure(context, phase, &options, Some(&envelope), &error);
                }
                let input = match protect_managed_guard_input(input, &envelope) {
                    Ok(input) => input,
                    Err(error) if options.output == args::OutputFormat::HostNative => {
                        return host_guard_failure(
                            context,
                            phase,
                            &options,
                            Some(&envelope),
                            &error,
                        );
                    }
                    Err(error) => return Err(error),
                };
                let subject = guard_subject(phase, &input, &envelope, &project);
                if phase == GuardHookPhase::PostTool {
                    if let Some(replayed) =
                        replayed_guard_phase_result(context, &project, &envelope, phase, &subject)?
                    {
                        record_guard_diagnostic_best_effort(
                            context,
                            &project,
                            &envelope,
                            phase,
                            diagnostic_started,
                            input.raw_text.len() as u64,
                            &replayed.result,
                        );
                        record_guard_workflow_metrics_best_effort(
                            context,
                            &envelope,
                            phase,
                            replayed.decision,
                            &replayed.result,
                            true,
                        );
                        let outcome = compatible_hook_outcome(
                            phase,
                            replayed.decision,
                            &options,
                            Some(&envelope),
                        );
                        let rendered = render_guard_command_output(
                            context,
                            phase,
                            &outcome,
                            Some(&envelope),
                            replayed.result,
                            &options,
                        )?;
                        return Ok(GuardCommandOutcome {
                            stdout: rendered.stdout,
                            stderr: rendered.stderr,
                            exit_code: rendered.exit_code,
                        });
                    }
                }
                if phase == GuardHookPhase::PromptCapture {
                    let _ =
                        start_guard_diagnostic_session_best_effort(context, &project, &envelope);
                }
                let phase_result = match phase {
                    GuardHookPhase::PreTool => {
                        phase::pre_tool::handle_pre_tool(context, &project, &envelope, &input)
                    }
                    GuardHookPhase::PostTool => {
                        phase::post_tool::handle_post_tool(context, &project, &envelope, &input)
                    }
                    GuardHookPhase::PromptCapture => handle_prompt_capture(
                        context, &project, &envelope, &input,
                    )
                    .map(|(decision, result, _exits_failure)| {
                        GuardPhaseResult::new(decision, result)
                    }),
                };
                let mut phase_result = match phase_result {
                    Ok(result) => result,
                    Err(error) if options.output == args::OutputFormat::HostNative => {
                        return host_guard_failure(
                            context,
                            phase,
                            &options,
                            Some(&envelope),
                            &error,
                        );
                    }
                    Err(error) => return Err(error),
                };
                attach_guard_disclosure(&mut phase_result.result);

                let outcome = compatible_hook_outcome(
                    phase,
                    phase_result.decision,
                    &options,
                    Some(&envelope),
                );
                attach_hook_outcome(&mut phase_result.result, &outcome);

                if persist_guard_event(
                    context,
                    &project,
                    &envelope,
                    GuardEventPersistence {
                        phase,
                        guard_input: &input,
                        subject,
                        phase_result: &phase_result,
                        options: &options,
                    },
                )
                .is_err()
                {
                    let persistence_outcome = GuardHookOutcome::new(
                        GuardObservationOutcome::PersistenceUnavailable,
                        Some(phase_result.decision),
                        [GuardHookDiagnostic {
                            code: GuardHookDiagnosticCode::EventPersistenceUnavailable,
                            facts: guard_diagnostic_facts(
                                phase,
                                &options,
                                envelope.guard_installation_id.as_deref(),
                                envelope.integration_revision.as_deref(),
                                Some(&envelope.event_id),
                            ),
                        }],
                        Some(GuardHostFeedback::Warning),
                    );
                    record_guard_findings_best_effort(context, &persistence_outcome);
                    let rendered = render_guard_command_output(
                        context,
                        phase,
                        &persistence_outcome,
                        Some(&envelope),
                        hook_outcome_result(&persistence_outcome),
                        &options,
                    )?;
                    return Ok(GuardCommandOutcome {
                        stdout: rendered.stdout,
                        stderr: rendered.stderr,
                        exit_code: rendered.exit_code,
                    });
                }
                if let Some(expected_write) = phase_result.expected_write {
                    if persist_expected_write(context, &project, expected_write).is_err() {
                        let persistence_outcome = GuardHookOutcome::new(
                            GuardObservationOutcome::PersistenceUnavailable,
                            Some(phase_result.decision),
                            [GuardHookDiagnostic {
                                code: GuardHookDiagnosticCode::EventPersistenceUnavailable,
                                facts: guard_diagnostic_facts(
                                    phase,
                                    &options,
                                    envelope.guard_installation_id.as_deref(),
                                    envelope.integration_revision.as_deref(),
                                    Some(&envelope.event_id),
                                ),
                            }],
                            Some(GuardHostFeedback::Warning),
                        );
                        record_guard_findings_best_effort(context, &persistence_outcome);
                        let rendered = render_guard_command_output(
                            context,
                            phase,
                            &persistence_outcome,
                            Some(&envelope),
                            hook_outcome_result(&persistence_outcome),
                            &options,
                        )?;
                        return Ok(GuardCommandOutcome {
                            stdout: rendered.stdout,
                            stderr: rendered.stderr,
                            exit_code: rendered.exit_code,
                        });
                    }
                }
                record_guard_findings_best_effort(context, &outcome);
                record_guard_diagnostic_best_effort(
                    context,
                    &project,
                    &envelope,
                    phase,
                    diagnostic_started,
                    input.raw_text.len() as u64,
                    &phase_result.result,
                );
                record_guard_workflow_metrics_best_effort(
                    context,
                    &envelope,
                    phase,
                    phase_result.decision,
                    &phase_result.result,
                    false,
                );
                let rendered = render_guard_command_output(
                    context,
                    phase,
                    &outcome,
                    Some(&envelope),
                    phase_result.result,
                    &options,
                )?;
                Ok(GuardCommandOutcome {
                    stdout: rendered.stdout,
                    stderr: rendered.stderr,
                    exit_code: rendered.exit_code,
                })
            })()
            .map_err(|error| CliMutationAdmissionError::Operation(error.to_string()))
        });
    match admitted {
        Ok(outcome) => Ok(outcome),
        Err(CliMutationAdmissionError::SetupInProgress(condition)) => {
            let outcome = GuardHookOutcome::new(
                GuardObservationOutcome::PersistenceUnavailable,
                None,
                [GuardHookDiagnostic {
                    code: GuardHookDiagnosticCode::EventPersistenceUnavailable,
                    facts: GuardHookDiagnosticFacts::default(),
                }],
                Some(GuardHostFeedback::Warning),
            );
            let rendered = render_guard_output(
                phase,
                &outcome,
                None,
                json!({
                    "condition": condition.code(),
                    "canonical_runtime_home": condition.runtime_home().as_path().display().to_string(),
                    "mutation_domain": condition.mutation_domain(),
                    "requested_mode": condition.requested_mode().as_str(),
                    "wait_policy": condition.wait_policy().as_str(),
                    "elapsed_millis": u64::try_from(condition.elapsed().as_millis()).unwrap_or(u64::MAX),
                    "retryable": condition.retryable(),
                    "persisted": false
                }),
                output_format,
            )?;
            Ok(GuardCommandOutcome {
                stdout: rendered.stdout,
                stderr: rendered.stderr,
                exit_code: 0,
            })
        }
        Err(error) => Err(GuardCommandError::Persistence(error.to_string())),
    }
}

fn record_guard_hook_contract_failure(
    context: &RuntimeHomeMutationContext<'_>,
    project: &ProjectRecord,
    phase: GuardHookPhase,
    options: &GuardOptions,
    input: &GuardInput,
    contract_status: GuardHookContractStatus,
    failure: GuardEnvelopeError,
) -> Result<GuardHookDiagnosticFacts, GuardCommandError> {
    let runtime_home = context.runtime_home().as_path();
    let connection_id = options.connection_id.as_deref().ok_or_else(|| {
        GuardCommandError::Runtime("Guard connection identity is unavailable".to_owned())
    })?;
    let guard_installation_id = options.guard_installation_id.as_deref().ok_or_else(|| {
        GuardCommandError::Runtime("Guard installation identity is unavailable".to_owned())
    })?;
    let installation =
        guard_installation(runtime_home, guard_installation_id)?.ok_or_else(|| {
            GuardCommandError::Runtime("Guard installation is unavailable".to_owned())
        })?;
    let manifest = guard_manifest_from_json(&installation.manifest_json).map_err(|_| {
        GuardCommandError::Runtime("current Guard installation manifest is malformed".to_owned())
    })?;
    let current_policy_hash = current_policy_hash(project)?;
    let current_owner = installation.connection_internal_id == connection_id
        && installation.project_id == project.project_id
        && manifest.connection_id.as_str() == connection_id
        && manifest.guard_installation_id.as_str() == guard_installation_id
        && manifest.project_id.as_str() == project.project_id
        && current_policy_hash.as_deref() == Some(manifest.policy_hash.as_str())
        && options
            .policy_hash
            .as_deref()
            .is_none_or(|hash| hash == manifest.policy_hash.as_str());
    if !current_owner {
        return Err(GuardCommandError::Runtime(
            "Guard installation ownership is unavailable".to_owned(),
        ));
    }

    let event_id = stable_id(
        "guard_event",
        &[
            phase.command_name(),
            connection_id,
            &project.project_id,
            &input.raw_sha256,
            contract_status.as_str(),
        ],
    );
    if guard_event(runtime_home, &project.project_id, &event_id)?.is_some() {
        observe_guard_probe_event_if_applicable(
            context,
            &project.project_id,
            &event_id,
            phase,
            input,
        )?;
        return Ok(GuardHookDiagnosticFacts {
            contract_profile: Some(HostContractProfileId::CodexCommandHooks.as_str().to_owned()),
            hook_event_kind: Some(phase.as_str().to_owned()),
            field_category: Some(failure.field_category.to_owned()),
            field: Some(failure.field.to_owned()),
            guard_installation_id: Some(guard_installation_id.to_owned()),
            integration_revision: Some(manifest.integration_revision.as_str().to_owned()),
            guard_event_id: Some(event_id),
        });
    }
    let occurred_at =
        UtcTimestamp::from_datetime(DateTime::<Utc>::from(SystemTime::now())).to_canonical_string();
    let subject_json = object_text(json!({
        "lifecycle_phase": phase.as_str(),
        "host_kind": manifest.host_kind.as_str(),
        "connection_id": connection_id,
        "project_id": project.project_id,
        "repo_root": project.repo_root.display().to_string(),
        "raw_event_sha256": input.raw_sha256,
    }))?;
    let source_payload_sha256 = guard_event_source_payload_sha256(
        None,
        connection_id,
        Some(guard_installation_id),
        phase.as_str(),
        &subject_json,
    )?;
    insert_guard_event(
        context,
        &project.project_id,
        GuardEventInsert {
            guard_event_id: event_id.clone(),
            correlation: None,
            connection_internal_id: connection_id.to_owned(),
            guard_installation_id: guard_installation_id.to_owned(),
            policy_hash: manifest.policy_hash.as_str().to_owned(),
            integration_revision: manifest.integration_revision.as_str().to_owned(),
            event_kind: phase.as_str().to_owned(),
            contract_status: contract_status.as_str().to_owned(),
            decision: GuardDecision::Warn.as_str().to_owned(),
            subject_json,
            result_json: object_text(json!({
                "decision": GuardDecision::Warn.as_str(),
                "observation_outcome": GuardObservationOutcome::IncompatibleRecorded.as_str(),
                "policy_decision": Value::Null,
                "allowed": true,
                "contract_status": contract_status.as_str(),
                "diagnostics": [{
                    "code": GuardHookDiagnosticCode::HostContractIncompatible.as_str(),
                    "facts": {
                        "contract_profile": HostContractProfileId::CodexCommandHooks.as_str(),
                        "hook_event_kind": phase.as_str(),
                        "field_category": failure.field_category,
                        "field": failure.field,
                        "guard_installation_id": guard_installation_id,
                        "integration_revision": manifest.integration_revision.as_str(),
                        "guard_event_id": event_id,
                    }
                }],
                "enforcement_level": "cooperative_guard",
            }))?,
            occurred_at,
            metadata_json: json!({
                "source": "volicord_guard_cli",
                "source_payload_sha256": source_payload_sha256,
                "host_contract_digest": HostContractProfileId::CodexCommandHooks.contract_digest(),
                "cooperative_guard": true,
            })
            .to_string(),
        },
    )?;
    observe_guard_probe_event_if_applicable(context, &project.project_id, &event_id, phase, input)?;
    Ok(GuardHookDiagnosticFacts {
        contract_profile: Some(HostContractProfileId::CodexCommandHooks.as_str().to_owned()),
        hook_event_kind: Some(phase.as_str().to_owned()),
        field_category: Some(failure.field_category.to_owned()),
        field: Some(failure.field.to_owned()),
        guard_installation_id: Some(guard_installation_id.to_owned()),
        integration_revision: Some(manifest.integration_revision.as_str().to_owned()),
        guard_event_id: Some(event_id),
    })
}

fn render_guard_command_output(
    context: &RuntimeHomeMutationContext<'_>,
    phase: GuardHookPhase,
    outcome: &GuardHookOutcome,
    envelope: Option<&GuardEnvelope>,
    result: Value,
    options: &GuardOptions,
) -> Result<RenderedGuardOutput, GuardCommandError> {
    match render_guard_output(phase, outcome, envelope, result, options.output) {
        Ok(rendered) => Ok(rendered),
        Err(error) if options.output == args::OutputFormat::HostNative => {
            let projection_outcome = GuardHookOutcome::new(
                outcome.observation,
                outcome.policy,
                [GuardHookDiagnostic {
                    code: GuardHookDiagnosticCode::HostOutputProjectionFailure,
                    facts: guard_diagnostic_facts(
                        phase,
                        options,
                        envelope.and_then(|value| value.guard_installation_id.as_deref()),
                        envelope.and_then(|value| value.integration_revision.as_deref()),
                        envelope.map(|value| value.event_id.as_str()),
                    ),
                }],
                Some(GuardHostFeedback::Warning),
            );
            record_guard_findings_best_effort(context, &projection_outcome);
            let _ = error;
            Ok(codex_output::render_codex_projection_failure(
                phase,
                outcome.policy,
            ))
        }
        Err(error) => Err(error),
    }
}

fn host_guard_failure(
    context: &RuntimeHomeMutationContext<'_>,
    phase: GuardHookPhase,
    options: &GuardOptions,
    envelope: Option<&GuardEnvelope>,
    error: &GuardCommandError,
) -> Result<GuardCommandOutcome, GuardCommandError> {
    let (observation, code) = match error {
        GuardCommandError::Persistence(_) => (
            GuardObservationOutcome::PersistenceUnavailable,
            GuardHookDiagnosticCode::EventPersistenceUnavailable,
        ),
        GuardCommandError::Usage(_) | GuardCommandError::Runtime(_) => (
            GuardObservationOutcome::PersistenceUnavailable,
            GuardHookDiagnosticCode::UnexpectedInternalFailure,
        ),
    };
    let outcome = GuardHookOutcome::new(
        observation,
        None,
        [GuardHookDiagnostic {
            code,
            facts: guard_diagnostic_facts(
                phase,
                options,
                envelope.and_then(|value| value.guard_installation_id.as_deref()),
                envelope.and_then(|value| value.integration_revision.as_deref()),
                envelope.map(|value| value.event_id.as_str()),
            ),
        }],
        Some(GuardHostFeedback::Warning),
    );
    record_guard_findings_best_effort(context, &outcome);
    let rendered = render_guard_command_output(
        context,
        phase,
        &outcome,
        envelope,
        hook_outcome_result(&outcome),
        options,
    )?;
    Ok(GuardCommandOutcome {
        stdout: rendered.stdout,
        stderr: rendered.stderr,
        exit_code: rendered.exit_code,
    })
}

fn compatible_hook_outcome(
    phase: GuardHookPhase,
    policy: GuardPolicyDecision,
    options: &GuardOptions,
    envelope: Option<&GuardEnvelope>,
) -> GuardHookOutcome {
    let diagnostics = (policy == GuardPolicyDecision::Deny)
        .then(|| GuardHookDiagnostic {
            code: GuardHookDiagnosticCode::PolicyDenied,
            facts: guard_diagnostic_facts(
                phase,
                options,
                envelope.and_then(|value| value.guard_installation_id.as_deref()),
                envelope.and_then(|value| value.integration_revision.as_deref()),
                envelope.map(|value| value.event_id.as_str()),
            ),
        })
        .into_iter();
    let feedback = match policy {
        GuardPolicyDecision::Continue => None,
        GuardPolicyDecision::ContinueWithContext => Some(GuardHostFeedback::Context),
        GuardPolicyDecision::ContinueWithWarning | GuardPolicyDecision::Deny => {
            Some(GuardHostFeedback::Warning)
        }
    };
    GuardHookOutcome::new(
        GuardObservationOutcome::CompatibleRecorded,
        Some(policy),
        diagnostics,
        feedback,
    )
}

fn guard_diagnostic_facts(
    phase: GuardHookPhase,
    options: &GuardOptions,
    installation_id: Option<&str>,
    integration_revision: Option<&str>,
    event_id: Option<&str>,
) -> GuardHookDiagnosticFacts {
    GuardHookDiagnosticFacts {
        contract_profile: Some(HostContractProfileId::CodexCommandHooks.as_str().to_owned()),
        hook_event_kind: Some(phase.as_str().to_owned()),
        field_category: None,
        field: None,
        guard_installation_id: installation_id
            .or(options.guard_installation_id.as_deref())
            .map(str::to_owned),
        integration_revision: integration_revision.map(str::to_owned),
        guard_event_id: event_id
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
    }
}

fn record_guard_findings_best_effort(
    context: &RuntimeHomeMutationContext<'_>,
    outcome: &GuardHookOutcome,
) {
    for diagnostic in &outcome.diagnostics {
        let stage = match diagnostic.code {
            GuardHookDiagnosticCode::HostContractIncompatible => "host_contract",
            GuardHookDiagnosticCode::EventPersistenceUnavailable => "event_persistence",
            GuardHookDiagnosticCode::PolicyDenied => "policy",
            GuardHookDiagnosticCode::HostOutputProjectionFailure => "host_output",
            GuardHookDiagnosticCode::UnexpectedInternalFailure => "internal",
        };
        let severity = match diagnostic.code {
            GuardHookDiagnosticCode::PolicyDenied => DiagnosticSeverity::Warning,
            GuardHookDiagnosticCode::HostContractIncompatible
            | GuardHookDiagnosticCode::EventPersistenceUnavailable
            | GuardHookDiagnosticCode::HostOutputProjectionFailure
            | GuardHookDiagnosticCode::UnexpectedInternalFailure => DiagnosticSeverity::Error,
        };
        let subject_identity = diagnostic
            .facts
            .guard_event_id
            .as_deref()
            .or(diagnostic.facts.hook_event_kind.as_deref())
            .unwrap_or("guard_hook");
        let finding = (|| {
            let data = DiagnosticFindingData::try_new(
                DiagnosticCode::parse(diagnostic.code.as_str())?,
                DiagnosticDomain::parse("guard")?,
                DiagnosticStage::parse(stage)?,
                severity,
                DiagnosticSource::parse("guard_hook")?,
                DiagnosticSubject::try_new("guard_event", subject_identity)?,
                DiagnosticFacts::project(&diagnostic.facts)?,
                UtcTimestamp::from_datetime(DateTime::<Utc>::from(SystemTime::now())),
            )?;
            OccurrenceDiagnosticFinding::try_new(data, None)
        })();
        if let Ok(finding) = finding {
            let _ = insert_occurrence_finding(context, &finding);
        }
    }
}

fn hook_outcome_result(outcome: &GuardHookOutcome) -> Value {
    json!({
        "observation_outcome": outcome.observation.as_str(),
        "policy_decision": outcome.policy.map(GuardPolicyDecision::as_str),
        "allowed": outcome.policy != Some(GuardPolicyDecision::Deny),
        "diagnostics": outcome.diagnostics,
        "enforcement_level": "cooperative_guard",
    })
}

fn attach_hook_outcome(result: &mut Value, outcome: &GuardHookOutcome) {
    if let Some(object) = result.as_object_mut() {
        object.insert(
            "observation_outcome".to_owned(),
            Value::String(outcome.observation.as_str().to_owned()),
        );
        object.insert(
            "policy_decision".to_owned(),
            outcome
                .policy
                .map(|decision| Value::String(decision.as_str().to_owned()))
                .unwrap_or(Value::Null),
        );
        object.insert(
            "guard_diagnostics".to_owned(),
            serde_json::to_value(&outcome.diagnostics).unwrap_or_else(|_| Value::Array(Vec::new())),
        );
    }
}

const fn stored_guard_decision(policy: GuardPolicyDecision) -> GuardDecision {
    match policy {
        GuardPolicyDecision::Continue => GuardDecision::Allow,
        GuardPolicyDecision::ContinueWithContext => GuardDecision::InjectContext,
        GuardPolicyDecision::ContinueWithWarning => GuardDecision::Warn,
        GuardPolicyDecision::Deny => GuardDecision::Deny,
    }
}

fn replayed_guard_phase_result(
    context: &RuntimeHomeMutationContext<'_>,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardHookPhase,
    subject: &Value,
) -> Result<Option<GuardPhaseResult>, GuardCommandError> {
    let runtime_home = context.runtime_home().as_path();
    let Some(existing) = guard_event(runtime_home, &project.project_id, &envelope.event_id)? else {
        return Ok(None);
    };
    let existing_source = guard_event_source_payload_sha256(
        existing.session_id.as_deref(),
        &existing.connection_internal_id,
        Some(existing.guard_installation_id.as_str()),
        &existing.event_kind,
        &existing.subject_json,
    )?;
    let requested_source = guard_event_source_payload_sha256(
        envelope.session_id.as_deref(),
        &envelope.connection_id,
        envelope.guard_installation_id.as_deref(),
        phase.as_str(),
        &object_text(subject.clone())?,
    )?;
    if existing_source != requested_source {
        return Err(GuardCommandError::Runtime(format!(
            "guard event {} conflicts with a different source payload hash",
            envelope.event_id
        )));
    }
    let decision = match existing.decision.as_str() {
        "allow" => GuardPolicyDecision::Continue,
        "deny" => GuardPolicyDecision::Deny,
        "warn" => GuardPolicyDecision::ContinueWithWarning,
        "inject_context" => GuardPolicyDecision::ContinueWithContext,
        _ => {
            return Err(GuardCommandError::Runtime(format!(
                "guard event {} contains an unsupported stored decision",
                envelope.event_id
            )))
        }
    };
    let result = serde_json::from_str::<Value>(&existing.result_json).map_err(json_error)?;
    if !result.is_object() {
        return Err(GuardCommandError::Runtime(format!(
            "guard event {} contains a malformed stored result",
            envelope.event_id
        )));
    }
    Ok(Some(GuardPhaseResult::new(decision, result)))
}

#[allow(clippy::too_many_arguments)]
fn record_guard_diagnostic_best_effort(
    context: &RuntimeHomeMutationContext<'_>,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardHookPhase,
    started: Instant,
    request_bytes: u64,
    result: &Value,
) {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return;
    };
    let authoritative_refresh_failure = result
        .get("reasons")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|reason| {
            reason.get("code").and_then(Value::as_str) == Some("authoritative_refresh_failed")
        });
    let suppression_unavailable = result
        .pointer("/recorded_change_suppression_outcome/status")
        .and_then(Value::as_str)
        == Some("unavailable");
    let prompt_capture_recorded = phase == GuardHookPhase::PromptCapture
        && result
            .get("recognized_user_action_command")
            .is_some_and(|value| !value.is_null());
    let prompt_capture_replayed = prompt_capture_recorded
        && result
            .pointer("/recognized_user_action_command/replayed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let product_file_write_count = (phase == GuardHookPhase::PostTool
        && result
            .pointer("/tool/changed_paths")
            .and_then(Value::as_array)
            .is_some_and(|paths| {
                paths
                    .iter()
                    .any(|path| path.get("inside_repo").and_then(Value::as_bool) == Some(true))
            })) as u64;
    let core_reached = prompt_capture_recorded;
    let core_committed = prompt_capture_recorded && !prompt_capture_replayed;
    let response_bytes = serde_json::to_vec(result)
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    let elapsed = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    let outcome = if authoritative_refresh_failure || suppression_unavailable {
        DiagnosticOutcome::Unavailable
    } else if result.get("allowed").and_then(Value::as_bool) == Some(false) {
        DiagnosticOutcome::Rejected
    } else {
        DiagnosticOutcome::Success
    };
    if !start_guard_diagnostic_session_best_effort(context, project, envelope) {
        return;
    }
    let _ = record_diagnostic_event(
        context,
        DiagnosticEvent {
            session_id,
            event_kind: DiagnosticEventKind::GuardHook,
            tool_name: None,
            latency_micros: elapsed,
            request_bytes,
            response_bytes,
            validation_failure: false,
            core_reached,
            core_committed,
            replayed: prompt_capture_replayed,
            user_channel_kind: prompt_capture_recorded
                .then_some(DiagnosticUserChannelKind::PromptCapture),
            fallback_kind: None,
            product_file_write_count,
            authoritative_refresh_failure,
            outcome,
        },
    );
}

fn start_guard_diagnostic_session_best_effort(
    context: &RuntimeHomeMutationContext<'_>,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
) -> bool {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return false;
    };
    let build = volicord_mcp::build_info();
    start_diagnostic_session(
        context,
        DiagnosticSessionStart {
            session_id,
            connection_id: Some(&envelope.connection_id),
            project_id: Some(&project.project_id),
            transport: DiagnosticTransport::GuardHook,
            host_kind: DiagnosticHostKind::from_connection_host_kind(&envelope.host_kind),
            package_version: build.package_version,
            build_id: &build.build_id,
        },
    )
    .is_ok()
}

#[allow(clippy::too_many_arguments)]
fn record_guard_workflow_metrics_best_effort(
    context: &RuntimeHomeMutationContext<'_>,
    envelope: &GuardEnvelope,
    phase: GuardHookPhase,
    decision: GuardPolicyDecision,
    result: &Value,
    _repeated: bool,
) {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return;
    };
    let integration_profile = (envelope.guard_mode == IntegrationProfile::Record.as_str())
        .then_some(IntegrationProfile::Record);
    let record = |metric_kind: WorkflowMetricKind,
                  value: u64,
                  metric_decision: Option<WorkflowMetricDecision>,
                  observation_confidence: Option<ObservationConfidence>,
                  outcome: Option<WorkflowMetricOutcome>| {
        let _ = record_workflow_metric_event(
            context,
            &WorkflowMetricEvent {
                session_id: session_id.to_owned(),
                metric_kind,
                value,
                method_name: None,
                integration_profile,
                decision: metric_decision,
                observation_confidence,
                outcome,
            },
        );
    };

    match phase {
        GuardHookPhase::PreTool => {
            let confidence = result
                .pointer("/tool/confidence")
                .and_then(Value::as_str)
                .and_then(workflow_observation_confidence);
            let metric_decision = match decision {
                GuardPolicyDecision::Continue => Some(WorkflowMetricDecision::Allow),
                GuardPolicyDecision::ContinueWithWarning => Some(WorkflowMetricDecision::Warn),
                GuardPolicyDecision::Deny => Some(WorkflowMetricDecision::Deny),
                GuardPolicyDecision::ContinueWithContext => None,
            };
            if let (Some(metric_decision), Some(confidence)) = (metric_decision, confidence) {
                record(
                    WorkflowMetricKind::PreToolDecision,
                    1,
                    Some(metric_decision),
                    Some(confidence),
                    None,
                );
            }
            let task_level = result
                .pointer("/context/active_task_effective_control_level")
                .and_then(Value::as_str);
            let structured_product_write = confidence == Some(ObservationConfidence::Structured)
                && result.pointer("/tool/effect").and_then(Value::as_str)
                    == Some("product_file_write");
            if decision == GuardPolicyDecision::Deny
                && structured_product_write
                && matches!(task_level, Some("light" | "tracked"))
            {
                record(
                    WorkflowMetricKind::ConfirmedStructuredWriteDeny,
                    1,
                    None,
                    None,
                    Some(WorkflowMetricOutcome::Rejected),
                );
            }
        }
        GuardHookPhase::PostTool => {
            let confidence = result
                .pointer("/tool/confidence")
                .and_then(Value::as_str)
                .and_then(workflow_observation_confidence);
            let effect = result
                .pointer("/tool/effect")
                .and_then(Value::as_str)
                .and_then(workflow_observation_effect);
            if let (Some(confidence), Some(effect)) = (confidence, effect) {
                record(
                    WorkflowMetricKind::ObservationAssessment,
                    1,
                    None,
                    Some(confidence),
                    Some(effect),
                );
            }
            let out_of_scope_count = result
                .get("unrecorded_changes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|change| {
                    change.get("confidence").and_then(Value::as_str) == Some("confirmed")
                        && matches!(
                            change.get("correlation_status").and_then(Value::as_str),
                            Some("out_of_scope_expected_write" | "out_of_scope_write_ticket")
                        )
                })
                .count() as u64;
            if out_of_scope_count > 0 {
                record(
                    WorkflowMetricKind::ConfirmedOutOfScopeWrite,
                    out_of_scope_count,
                    None,
                    None,
                    Some(WorkflowMetricOutcome::Success),
                );
            }
            let suspected_resolved_no_change = result
                .get("resolved_suspected_changes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|change| {
                    change.get("confidence").and_then(Value::as_str) == Some("suspected")
                        && change.get("resolution_basis").and_then(Value::as_str)
                            == Some("invalid_observation")
                })
                .count() as u64;
            if suspected_resolved_no_change > 0 {
                record(
                    WorkflowMetricKind::SuspectedResolvedNoChange,
                    suspected_resolved_no_change,
                    None,
                    None,
                    Some(WorkflowMetricOutcome::Success),
                );
            }
        }
        GuardHookPhase::PromptCapture => {}
    }
}

fn workflow_observation_confidence(value: &str) -> Option<ObservationConfidence> {
    match value {
        "confirmed" => Some(ObservationConfidence::Confirmed),
        "structured" => Some(ObservationConfidence::Structured),
        "heuristic" => Some(ObservationConfidence::Heuristic),
        "unknown" => Some(ObservationConfidence::Unknown),
        _ => None,
    }
}

fn workflow_observation_effect(value: &str) -> Option<WorkflowMetricOutcome> {
    match value {
        "read_only" => Some(WorkflowMetricOutcome::ReadOnly),
        "product_file_write" => Some(WorkflowMetricOutcome::ProductFileWrite),
        "non_product_write" => Some(WorkflowMetricOutcome::NonProductWrite),
        "external_effect" => Some(WorkflowMetricOutcome::ExternalEffect),
        "unknown" => Some(WorkflowMetricOutcome::Unknown),
        _ => None,
    }
}

fn attach_guard_disclosure(result: &mut Value) {
    if let Some(object) = result.as_object_mut() {
        object.insert(
            "disclosure".to_owned(),
            cooperative_host_decision_disclosure_json(),
        );
    }
}

fn resolve_guard_project(
    runtime_home: &Path,
    current_dir: &Path,
    options: &GuardOptions,
    event: &Value,
) -> Result<ProjectRecord, GuardCommandError> {
    if let Some(repo) = options
        .repo
        .as_deref()
        .or_else(|| event_path_field(event, &[&["repo_root"], &["repository_root"], &["cwd"]]))
    {
        let repo_root = resolve_repository_root(current_dir, Some(repo))?;
        return registered_project_for_repo(runtime_home, &repo_root).map_err(Into::into);
    }
    if let Some(project_id) = event_string(event, &[&["project_id"], &["project", "id"]]) {
        return project_record_for_execution(runtime_home, &project_id)?
            .ok_or_else(|| GuardCommandError::Runtime(format!("project not found: {project_id}")));
    }
    let repo_root = resolve_repository_root(current_dir, None)?;
    registered_project_for_repo(runtime_home, &repo_root).map_err(Into::into)
}

fn bind_guard_envelope(
    context: &RuntimeHomeMutationContext<'_>,
    project: &ProjectRecord,
    phase: GuardHookPhase,
    input: &GuardInput,
    envelope: &mut GuardEnvelope,
) -> Result<(), GuardCommandError> {
    let runtime_home = context.runtime_home().as_path();
    let coordinates = current_project_agent_session_coordinates(
        runtime_home,
        &project.project_id,
        &envelope.connection_id,
        envelope.guard_installation_id.as_deref(),
        &envelope.correlation,
    )?;
    envelope.guard_installation_id = coordinates.guard_installation_id;
    let session = observe_host_correlation(
        context,
        &project.project_id,
        HostCorrelationObservation {
            connection_internal_id: envelope.connection_id.clone(),
            guard_installation_id: envelope.guard_installation_id.clone(),
            correlation: envelope.correlation.clone(),
            observed_at: envelope.occurred_at.clone(),
        },
    )?;
    if session.session_id != coordinates.session_id
        || session.project_integration_revision != coordinates.project_integration_revision
    {
        return Err(GuardCommandError::Runtime(
            "Store returned a project Agent Session outside the derived current revision"
                .to_owned(),
        ));
    }
    envelope.session_id = Some(session.session_id);
    envelope.integration_revision = Some(session.project_integration_revision.as_str().to_owned());
    envelope.event_id = stable_id(
        "guard_event",
        &[
            phase.command_name(),
            &envelope.connection_id,
            envelope.session_id.as_deref().unwrap_or(""),
            &project.project_id,
            &input.raw_sha256,
        ],
    );
    Ok(())
}

fn current_policy_hash(project: &ProjectRecord) -> Result<Option<String>, GuardCommandError> {
    let policy_path = GuardManagedArtifact::VolicordPolicy
        .expected_path(&project.repo_root, None)
        .expect("the Guard policy has a repository-owned path");
    let text = match fs::read_to_string(&policy_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(GuardCommandError::Runtime(format!(
                "failed to read host hook policy {}: {error}",
                policy_path.display()
            )));
        }
    };
    let value = serde_json::from_str::<Value>(&text).map_err(|error| {
        GuardCommandError::Runtime(format!(
            "host hook policy is not valid JSON: {} ({error})",
            policy_path.display()
        ))
    })?;
    serde_json::to_string(&value)
        .map(|canonical| Some(sha256_text(&canonical)))
        .map_err(json_error)
}

struct GuardEventPersistence<'a> {
    phase: GuardHookPhase,
    guard_input: &'a GuardInput,
    subject: Value,
    phase_result: &'a GuardPhaseResult,
    options: &'a GuardOptions,
}

fn persist_guard_event(
    context: &RuntimeHomeMutationContext<'_>,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    persistence: GuardEventPersistence<'_>,
) -> Result<(), GuardCommandError> {
    let runtime_home = context.runtime_home().as_path();
    let GuardEventPersistence {
        phase,
        guard_input,
        subject,
        phase_result,
        options,
    } = persistence;
    let guard_installation_id = envelope.guard_installation_id.as_deref().ok_or_else(|| {
        GuardCommandError::Runtime("Guard event has no installation identity".to_owned())
    })?;
    let installation =
        guard_installation(runtime_home, guard_installation_id)?.ok_or_else(|| {
            GuardCommandError::Runtime(format!(
                "Guard installation {guard_installation_id} is not registered"
            ))
        })?;
    let manifest = guard_manifest_from_json(&installation.manifest_json).map_err(|_| {
        GuardCommandError::Runtime("current Guard installation manifest is malformed".to_owned())
    })?;
    let current_policy_hash = current_policy_hash(project)?.ok_or_else(|| {
        GuardCommandError::Runtime("current Guard policy file is missing".to_owned())
    })?;
    if installation.connection_internal_id != envelope.connection_id
        || installation.project_id != project.project_id
        || manifest.policy_hash.as_str() != current_policy_hash
        || options
            .policy_hash
            .as_deref()
            .is_some_and(|hash| hash != manifest.policy_hash.as_str())
    {
        return Err(GuardCommandError::Runtime(
            "Guard event ownership does not match the current policy and manifest".to_owned(),
        ));
    }
    let subject_json = object_text(subject)?;
    let source_payload_sha256 = guard_event_source_payload_sha256(
        envelope.session_id.as_deref(),
        &envelope.connection_id,
        envelope.guard_installation_id.as_deref(),
        phase.as_str(),
        &subject_json,
    )?;
    let input = GuardEventInsert {
        guard_event_id: envelope.event_id.clone(),
        correlation: Some(envelope.correlation.clone()),
        connection_internal_id: envelope.connection_id.clone(),
        guard_installation_id: guard_installation_id.to_owned(),
        policy_hash: manifest.policy_hash.as_str().to_owned(),
        integration_revision: manifest.integration_revision.as_str().to_owned(),
        event_kind: phase.as_str().to_owned(),
        contract_status: GuardHookContractStatus::Compatible.as_str().to_owned(),
        decision: stored_guard_decision(phase_result.decision)
            .as_str()
            .to_owned(),
        subject_json,
        result_json: object_text(phase_result.result.clone())?,
        occurred_at: envelope.occurred_at.clone(),
        metadata_json: json!({
            "source": "volicord_guard_cli",
            "source_payload_sha256": source_payload_sha256,
            "host_contract_digest": HostContractProfileId::CodexCommandHooks.contract_digest(),
            "cooperative_guard": true
        })
        .to_string(),
    };
    if let Some(existing) = guard_event(runtime_home, &project.project_id, &envelope.event_id)? {
        if guard_event_record_payload_sha256(&existing)?
            == guard_event_insert_payload_sha256(&input, envelope.session_id.as_deref())?
        {
            observe_guard_probe_event_if_applicable(
                context,
                &project.project_id,
                &envelope.event_id,
                phase,
                guard_input,
            )?;
            return Ok(());
        }
        return Err(GuardCommandError::Runtime(format!(
            "guard event {} conflicts with a different payload hash",
            envelope.event_id
        )));
    }
    insert_guard_event(context, &project.project_id, input)?;
    observe_guard_probe_event_if_applicable(
        context,
        &project.project_id,
        &envelope.event_id,
        phase,
        guard_input,
    )?;
    Ok(())
}

fn observe_guard_probe_event_if_applicable(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    guard_event_id: &str,
    phase: GuardHookPhase,
    input: &GuardInput,
) -> Result<(), GuardCommandError> {
    if !matches!(phase, GuardHookPhase::PreTool | GuardHookPhase::PostTool) {
        return Ok(());
    }
    let evidence = guard_probe_hook_evidence(input);
    observe_guard_probe_hook_event(context, project_id, guard_event_id, evidence)?;
    Ok(())
}

fn guard_probe_hook_evidence(input: &GuardInput) -> GuardProbeHookEvidence {
    let Some(tool_input) = guard_event_tool_input(&input.raw_value).and_then(Value::as_object)
    else {
        return GuardProbeHookEvidence::absent();
    };
    let Some(value) = tool_input.get("verification_id") else {
        return GuardProbeHookEvidence::absent();
    };
    let bounded = value
        .as_str()
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 192
                && value.trim() == *value
                && !value.chars().any(char::is_control)
        })
        .map(str::to_owned);
    GuardProbeHookEvidence::present(bounded)
}

fn guard_subject(
    phase: GuardHookPhase,
    input: &GuardInput,
    envelope: &GuardEnvelope,
    project: &ProjectRecord,
) -> Value {
    let mut subject = json!({
        "lifecycle_phase": phase.as_str(),
        "host_kind": envelope.host_kind,
        "connection_id": envelope.connection_id,
        "project_id": project.project_id,
        "repo_root": project.repo_root.display().to_string(),
        "raw_event_sha256": input.raw_sha256,
        "tool_input_sha256": guard_event_tool_input(&input.raw_value).map(canonical_value_sha256),
        "tool_result_sha256": guard_event_tool_result(&input.raw_value).map(canonical_value_sha256),
        "tool_result_size_bytes": guard_event_tool_result(&input.raw_value)
            .map(|value| {
                canonical_json_bytes(value)
                    .expect("serde_json::Value always has a canonical JSON encoding")
            })
            .and_then(|bytes| u64::try_from(bytes.len()).ok())
    });
    let routed_mcp_event = matches!(
        &envelope.correlation,
        HostNativeCorrelation::CodexHookTool(tool)
            if tool.tool_name.as_str().starts_with("mcp__")
    );
    if !routed_mcp_event {
        subject
            .as_object_mut()
            .expect("Guard subject is an object")
            .insert("raw_event".to_owned(), input.redacted_value.clone());
    }
    subject
}

fn protect_managed_guard_input(
    mut input: GuardInput,
    envelope: &GuardEnvelope,
) -> Result<GuardInput, GuardCommandError> {
    if !is_managed_builtin_host(&envelope.host_kind) {
        return Ok(input);
    }
    let native_session_id = envelope.correlation.session_id().as_str().to_owned();
    let managed_session_id = envelope.session_id.as_deref().ok_or_else(|| {
        GuardCommandError::Runtime(
            "managed host event has no canonical managed session binding".to_owned(),
        )
    })?;
    let replacements = managed_native_identifier_replacements(
        &input.raw_value,
        managed_session_id,
        &envelope.event_id,
        &envelope.connection_id,
        &native_session_id,
    );
    let semantic_context = ManagedEventProtectionContext {
        managed_session_id,
        guard_event_id: &envelope.event_id,
        connection_id: &envelope.connection_id,
        protection: ManagedEventProtection::Semantic,
        replacements: &replacements,
    };
    input.raw_value = protect_managed_event_value(&input.raw_value, None, None, &semantic_context);
    let persistent_context = ManagedEventProtectionContext {
        protection: ManagedEventProtection::Persistent,
        ..semantic_context
    };
    input.redacted_value =
        protect_managed_event_value(&input.redacted_value, None, None, &persistent_context);
    Ok(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedNativeIdentifierKind {
    Session,
    Event,
    Correlation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManagedNativeIdentifierReplacement {
    raw: String,
    opaque: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedEventProtection {
    Semantic,
    Persistent,
}

#[derive(Debug, Clone, Copy)]
struct ManagedEventProtectionContext<'a> {
    managed_session_id: &'a str,
    guard_event_id: &'a str,
    connection_id: &'a str,
    protection: ManagedEventProtection,
    replacements: &'a [ManagedNativeIdentifierReplacement],
}

fn managed_native_identifier_replacements(
    value: &Value,
    managed_session_id: &str,
    guard_event_id: &str,
    connection_id: &str,
    native_session_id: &str,
) -> Vec<ManagedNativeIdentifierReplacement> {
    let mut session_ids = BTreeSet::from([native_session_id.to_owned()]);
    let mut event_ids = BTreeSet::new();
    let mut correlation_ids = BTreeSet::new();
    collect_managed_native_identifiers(
        value,
        None,
        None,
        &mut session_ids,
        &mut event_ids,
        &mut correlation_ids,
    );

    let mut replacements = Vec::new();
    let mut claimed = BTreeSet::new();
    for (ids, kind) in [
        (&session_ids, ManagedNativeIdentifierKind::Session),
        (&event_ids, ManagedNativeIdentifierKind::Event),
        (&correlation_ids, ManagedNativeIdentifierKind::Correlation),
    ] {
        for raw in ids {
            if raw.is_empty() || !claimed.insert(raw.clone()) {
                continue;
            }
            let opaque = opaque_managed_native_identifier(
                kind,
                raw,
                managed_session_id,
                guard_event_id,
                connection_id,
            );
            replacements.push(ManagedNativeIdentifierReplacement {
                raw: raw.clone(),
                opaque,
            });
        }
    }
    replacements.sort_by(|left, right| {
        right
            .raw
            .len()
            .cmp(&left.raw.len())
            .then_with(|| left.raw.cmp(&right.raw))
    });
    replacements
}

fn opaque_managed_native_identifier(
    kind: ManagedNativeIdentifierKind,
    raw: &str,
    managed_session_id: &str,
    guard_event_id: &str,
    connection_id: &str,
) -> String {
    match kind {
        ManagedNativeIdentifierKind::Session => managed_session_id.to_owned(),
        ManagedNativeIdentifierKind::Event => guard_event_id.to_owned(),
        ManagedNativeIdentifierKind::Correlation => stable_id(
            "managed_native_id",
            &[managed_session_id, connection_id, raw],
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_managed_native_identifiers(
    value: &Value,
    field: Option<&str>,
    parent_field: Option<&str>,
    session_ids: &mut BTreeSet<String>,
    event_ids: &mut BTreeSet<String>,
    correlation_ids: &mut BTreeSet<String>,
) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                collect_managed_native_identifiers(
                    value,
                    Some(key),
                    field,
                    session_ids,
                    event_ids,
                    correlation_ids,
                );
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_managed_native_identifiers(
                    value,
                    field,
                    parent_field,
                    session_ids,
                    event_ids,
                    correlation_ids,
                );
            }
        }
        Value::String(text) if !text.is_empty() => {
            match managed_native_identifier_kind(field, parent_field) {
                Some(ManagedNativeIdentifierKind::Session) => {
                    session_ids.insert(text.clone());
                }
                Some(ManagedNativeIdentifierKind::Event) => {
                    event_ids.insert(text.clone());
                }
                Some(ManagedNativeIdentifierKind::Correlation) => {
                    correlation_ids.insert(text.clone());
                }
                None => {}
            }
        }
        _ => {}
    }
}

fn managed_native_identifier_kind(
    field: Option<&str>,
    parent_field: Option<&str>,
) -> Option<ManagedNativeIdentifierKind> {
    match field {
        Some("session_id" | "thread_id") => Some(ManagedNativeIdentifierKind::Session),
        Some("guard_event_id" | "event_id" | "hook_event_id" | "native_event_id") => {
            Some(ManagedNativeIdentifierKind::Event)
        }
        Some(
            "tool_call_id"
            | "tool_use_id"
            | "tool_invocation_id"
            | "host_invocation_id"
            | "invocation_id"
            | "call_id"
            | "prompt_capture_id"
            | "capture_id"
            | "turn_id"
            | "transcript_id"
            | "conversation_id"
            | "native_session_id"
            | "native_tool_call_id"
            | "native_capture_id"
            | "native_turn_id"
            | "native_invocation_id",
        ) => Some(ManagedNativeIdentifierKind::Correlation),
        Some("id") if parent_field == Some("session") => Some(ManagedNativeIdentifierKind::Session),
        Some("id")
            if matches!(
                parent_field,
                None | Some(
                    "tool"
                        | "tool_use"
                        | "tool_result"
                        | "result"
                        | "event"
                        | "turn"
                        | "transcript"
                        | "conversation"
                        | "capture"
                        | "prompt_capture"
                )
            ) =>
        {
            Some(ManagedNativeIdentifierKind::Correlation)
        }
        _ => None,
    }
}

fn contains_managed_native_identifier(
    text: &str,
    replacements: &[ManagedNativeIdentifierReplacement],
) -> bool {
    replacements
        .iter()
        .any(|replacement| text.contains(&replacement.raw))
}

fn replace_managed_native_identifiers(
    text: &str,
    replacements: &[ManagedNativeIdentifierReplacement],
) -> String {
    replacements
        .iter()
        .fold(text.to_owned(), |rendered, replacement| {
            rendered.replace(&replacement.raw, &replacement.opaque)
        })
}

fn protect_managed_event_value(
    value: &Value,
    field: Option<&str>,
    parent_field: Option<&str>,
    context: &ManagedEventProtectionContext<'_>,
) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .enumerate()
                .map(|(index, (key, value))| {
                    let redact_key = context.protection == ManagedEventProtection::Persistent
                        && contains_managed_native_identifier(key, context.replacements);
                    let redacted_key = if redact_key {
                        format!("managed_host_field_{index}_omitted")
                    } else {
                        key.clone()
                    };
                    let opaque_value = if redact_key {
                        json!({ "omitted": true })
                    } else {
                        protect_managed_event_value(value, Some(key), field, context)
                    };
                    (redacted_key, opaque_value)
                })
                .collect::<Map<_, _>>(),
        ),
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| protect_managed_event_value(value, field, parent_field, context))
                .collect(),
        ),
        Value::String(text) => Value::String(
            managed_native_identifier_kind(field, parent_field).map_or_else(
                || match context.protection {
                    ManagedEventProtection::Semantic => text.clone(),
                    ManagedEventProtection::Persistent => {
                        replace_managed_native_identifiers(text, context.replacements)
                    }
                },
                |kind| {
                    opaque_managed_native_identifier(
                        kind,
                        text,
                        context.managed_session_id,
                        context.guard_event_id,
                        context.connection_id,
                    )
                },
            ),
        ),
        other => other.clone(),
    }
}

fn guard_event_tool_input(event: &Value) -> Option<&Value> {
    event
        .get("tool_input")
        .or_else(|| event.get("input"))
        .or_else(|| event.pointer("/tool/input"))
        .or_else(|| event.pointer("/tool/arguments"))
        .or_else(|| event.pointer("/tool_use/input"))
}

fn guard_event_tool_result(event: &Value) -> Option<&Value> {
    event
        .get("tool_response")
        .or_else(|| event.get("tool_result"))
        .or_else(|| event.get("result"))
        .or_else(|| event.get("output"))
}

fn canonical_value_sha256(value: &Value) -> String {
    canonical_json_bare_sha256(value).expect("serde_json::Value must serialize")
}

fn guard_event_insert_payload_sha256(
    input: &GuardEventInsert,
    session_id: Option<&str>,
) -> Result<String, GuardCommandError> {
    guard_event_payload_sha256(GuardEventPayload {
        session_id,
        connection_id: &input.connection_internal_id,
        guard_installation_id: &input.guard_installation_id,
        event_kind: &input.event_kind,
        decision: &input.decision,
        subject_json: &input.subject_json,
        policy_hash: &input.policy_hash,
        integration_revision: &input.integration_revision,
        contract_status: &input.contract_status,
    })
}

fn guard_event_record_payload_sha256(
    record: &volicord_store::guards::GuardEventRecord,
) -> Result<String, GuardCommandError> {
    guard_event_payload_sha256(GuardEventPayload {
        session_id: record.session_id.as_deref(),
        connection_id: &record.connection_internal_id,
        guard_installation_id: &record.guard_installation_id,
        event_kind: &record.event_kind,
        decision: &record.decision,
        subject_json: &record.subject_json,
        policy_hash: &record.policy_hash,
        integration_revision: &record.integration_revision,
        contract_status: &record.contract_status,
    })
}

struct GuardEventPayload<'a> {
    session_id: Option<&'a str>,
    connection_id: &'a str,
    guard_installation_id: &'a str,
    event_kind: &'a str,
    decision: &'a str,
    subject_json: &'a str,
    policy_hash: &'a str,
    integration_revision: &'a str,
    contract_status: &'a str,
}

fn guard_event_payload_sha256(payload: GuardEventPayload<'_>) -> Result<String, GuardCommandError> {
    let source_sha256 = guard_event_source_payload_sha256(
        payload.session_id,
        payload.connection_id,
        Some(payload.guard_installation_id),
        payload.event_kind,
        payload.subject_json,
    )?;
    Ok(canonical_value_sha256(&json!({
        "source_sha256": source_sha256,
        "decision": payload.decision,
        "policy_hash": payload.policy_hash,
        "integration_revision": payload.integration_revision,
        "contract_status": payload.contract_status,
    })))
}

fn guard_event_source_payload_sha256(
    session_id: Option<&str>,
    connection_id: &str,
    guard_installation_id: Option<&str>,
    event_kind: &str,
    subject_json: &str,
) -> Result<String, GuardCommandError> {
    let subject: Value = serde_json::from_str(subject_json).map_err(json_error)?;
    let raw_event_sha256 = subject
        .get("raw_event_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            GuardCommandError::Runtime(
                "guard event subject has no raw_event_sha256 replay coordinate".to_owned(),
            )
        })?;
    Ok(canonical_value_sha256(&json!({
        "session_id": session_id,
        "connection_id": connection_id,
        "guard_installation_id": guard_installation_id,
        "event_kind": event_kind,
        "raw_event_sha256": raw_event_sha256,
    })))
}

fn object_text(value: Value) -> Result<String, GuardCommandError> {
    match value {
        Value::Object(_) => serde_json::to_string(&value).map_err(json_error),
        other => serde_json::to_string(&json!({ "value": other })).map_err(json_error),
    }
}

fn redact_event_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    if prompt_like_key(key) {
                        (key.clone(), redacted_prompt_value(value))
                    } else {
                        (key.clone(), redact_event_value(value))
                    }
                })
                .collect::<Map<_, _>>(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(redact_event_value).collect()),
        other => other.clone(),
    }
}

fn prompt_like_key(key: &str) -> bool {
    matches!(
        key,
        "prompt"
            | "user_prompt"
            | "message"
            | "messages"
            | "content"
            | "transcript"
            | "last_assistant_message"
            | "assistant_message"
            | "lastAssistantMessage"
            | "assistantMessage"
    )
}

fn redacted_prompt_value(value: &Value) -> Value {
    match value {
        Value::String(text) => json!({
            "omitted": true,
            "sha256": sha256_text(text),
            "size_bytes": text.len()
        }),
        Value::Array(values) => json!({
            "omitted": true,
            "sha256": sha256_text(&value.to_string()),
            "item_count": values.len()
        }),
        _ => json!({
            "omitted": true,
            "sha256": sha256_text(&value.to_string())
        }),
    }
}

fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{}", hex_bytes(&hasher.finalize()))
}

fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hex_bytes(&hasher.finalize());
    format!("{prefix}_{}", &digest[..16])
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn json_error(error: serde_json::Error) -> GuardCommandError {
    GuardCommandError::Runtime(format!("failed to serialize host-hook output: {error}"))
}
