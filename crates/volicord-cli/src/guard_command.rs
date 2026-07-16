use std::{collections::BTreeSet, ffi::OsString, fmt, fs, path::Path, time::Instant};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use volicord_core::{Clock, CorePipelineError, SystemClock};
use volicord_store::{
    agent_connections::agent_connection_record_read_only,
    bootstrap::{project_record_for_execution, ProjectRecord},
    core_pipeline::CoreProjectStore,
    diagnostics::{
        record_diagnostic_event, record_workflow_metric_event, start_diagnostic_session,
        DiagnosticEvent, DiagnosticEventKind, DiagnosticHostKind, DiagnosticOutcome,
        DiagnosticSessionStart, DiagnosticTransport, DiagnosticUserChannelKind,
        WorkflowMetricDecision, WorkflowMetricEvent, WorkflowMetricKind, WorkflowMetricOutcome,
    },
    guards::{
        agent_session, guard_event, insert_agent_session, insert_guard_event,
        observe_guard_installation, prior_guard_event_exists_for_session_kind, AgentSessionInsert,
        GuardEventInsert, GuardInstallationObservation,
    },
    host_runtime_probes::record_host_runtime_probe_observation,
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError, StoreResult,
};
use volicord_types::{
    canonical_json_bare_sha256, canonical_json_bytes, GuardDecision, HostRuntimeProbeFailureClass,
    HostRuntimeProbeId, HostRuntimeProbeObservation, HostRuntimeProbeOutcome, IntegrationProfile,
    ObservationConfidence, UtcTimestamp,
    VERIFICATION_BASIS_REGISTERED_HOST_STOP_HOOK_CONNECTION_BINDING,
    VERIFICATION_BASIS_UNREGISTERED_HOST_HOOK_EVENT,
};

use crate::disclosure::cooperative_host_decision_disclosure_json;
use crate::final_output_command::{
    managed_final_output_binding_is_verified, project_managed_final_authority,
    read_final_authority_coordinates, render_final_authority_response,
    FinalAuthorityFallbackReason, FinalAuthorityProjection, ManagedFinalOutputHost,
};
use crate::project_context::{
    registered_project_for_repo, resolve_repository_root, ProjectCommandError,
};
const DEFAULT_INTEGRATION_PROFILE: &str = "detective";
const VOLICORD_POLICY_FILE: &str = ".volicord/policy.json";
const EXPECTED_WRITE_TTL_MINUTES: i64 = 15;
const SESSION_WATCH_METADATA_SOURCE: &str = "volicord_session_watch";

mod args;
mod context;
mod envelope;
mod mutation;
mod phase;
mod prompt_capture;
mod prompt_command;
mod render;
mod tool_observation;
mod write_ticket;

pub use args::guard_usage;
use args::{
    parse_guard_options, read_guard_input, GuardInput, GuardOptions, GuardPhase, HostOutputMode,
    OutputFormat,
};
use envelope::{
    event_path_field, event_string, guard_envelope, is_managed_builtin_host,
    managed_native_session_id, GuardEnvelope,
};
use phase::{pre_tool::persist_expected_write, GuardPhaseResult};
use prompt_capture::handle_prompt_capture;
use render::{render_guard_output, RenderedGuardOutput};
use tool_observation::{tool_name_is_direct_write, tool_observation, ToolObservation};

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
}

impl fmt::Display for GuardCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for GuardCommandError {}

impl From<StoreError> for GuardCommandError {
    fn from(error: StoreError) -> Self {
        Self::Runtime(error.to_string())
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

pub fn run_guard_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<GuardCommandOutcome, GuardCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let Some(subcommand) = args.first().map(String::as_str) else {
        return Ok(GuardCommandOutcome {
            stdout: guard_usage(),
            stderr: String::new(),
            exit_code: 0,
        });
    };
    if matches!(subcommand, "-h" | "--help" | "help") {
        if args.len() == 1 {
            return Ok(GuardCommandOutcome {
                stdout: guard_usage(),
                stderr: String::new(),
                exit_code: 0,
            });
        }
        return Err(GuardCommandError::Usage(format!(
            "unexpected argument: {}\n\n{}",
            args[1],
            guard_usage()
        )));
    }

    let phase = match subcommand {
        "session-start" => GuardPhase::SessionStart,
        "pre-tool" => GuardPhase::PreTool,
        "post-tool" => GuardPhase::PostTool,
        "prompt-capture" => GuardPhase::PromptCapture,
        "stop" => GuardPhase::Stop,
        other => {
            return Err(GuardCommandError::Usage(format!(
                "unknown _hook command: {other}\n\n{}",
                guard_usage()
            )))
        }
    };
    let diagnostic_started = Instant::now();
    let options = parse_guard_options(&args[1..])?;
    let runtime_home = resolve_runtime_home(env_var, current_dir)?;
    let input = read_guard_input(options.event_file.as_deref())?;
    let project = resolve_guard_project(&runtime_home, current_dir, &options, &input.raw_value)?;
    let envelope = guard_envelope(phase, &options, &input, &project)?;
    let input = protect_managed_guard_input(input, &envelope)?;
    validate_existing_managed_session_binding(&runtime_home, &project, &envelope)?;
    let subject = guard_subject(phase, &input, &envelope, &project);
    if matches!(phase, GuardPhase::PostTool | GuardPhase::Stop) {
        if let Some(replayed) =
            replayed_guard_phase_result(&runtime_home, &project, &envelope, phase, &subject)?
        {
            record_guard_runtime_probes_best_effort(
                &runtime_home,
                &project,
                &envelope,
                phase,
                &input.raw_value,
                phase == GuardPhase::Stop,
            );
            record_guard_diagnostic_best_effort(
                &runtime_home,
                &project,
                &envelope,
                phase,
                diagnostic_started,
                input.raw_text.len() as u64,
                &replayed.result,
            );
            record_guard_workflow_metrics_best_effort(
                &runtime_home,
                &envelope,
                phase,
                replayed.decision,
                &replayed.result,
                true,
            );
            let rendered = render_guard_command_output(
                phase,
                replayed.decision,
                &envelope,
                replayed.result,
                &options,
                &runtime_home,
                &project,
            )?;
            return Ok(GuardCommandOutcome {
                stdout: rendered.stdout,
                stderr: rendered.stderr,
                exit_code: rendered.exit_code,
            });
        }
    }
    ensure_required_session(&runtime_home, &project, &envelope, phase)?;
    if phase == GuardPhase::PromptCapture {
        let _ = start_guard_diagnostic_session_best_effort(&runtime_home, &project, &envelope);
    }
    let _activation =
        observe_guard_installation_activation(&runtime_home, &project, &envelope, phase, &options)?;
    let stop_invocation_binding_basis = (phase == GuardPhase::Stop)
        .then(|| stop_invocation_binding_basis(&runtime_home, &project, &envelope, &options));
    let mut phase_result = match phase {
        GuardPhase::SessionStart => {
            phase::session_start::handle_session_start(&runtime_home, &project, &envelope, &input)?
        }
        GuardPhase::PreTool => {
            phase::pre_tool::handle_pre_tool(&runtime_home, &project, &envelope, &input)?
        }
        GuardPhase::PostTool => {
            phase::post_tool::handle_post_tool(&runtime_home, &project, &envelope, &input)?
        }
        GuardPhase::PromptCapture => {
            let (decision, result, _exits_failure) =
                handle_prompt_capture(&runtime_home, &project, &envelope, &input)?;
            GuardPhaseResult::new(decision, result)
        }
        GuardPhase::Stop => phase::stop::handle_stop(
            &runtime_home,
            &project,
            &envelope,
            &input,
            stop_invocation_binding_basis
                .expect("Stop phase always derives an invocation binding basis"),
        )?,
    };
    attach_guard_disclosure(&mut phase_result.result);

    persist_guard_event(
        &runtime_home,
        &project,
        &envelope,
        phase,
        phase_result.decision,
        subject,
        phase_result.result.clone(),
    )?;
    record_guard_runtime_probes_best_effort(
        &runtime_home,
        &project,
        &envelope,
        phase,
        &input.raw_value,
        false,
    );
    if let Some(expected_write) = phase_result.expected_write {
        persist_expected_write(&runtime_home, &project, expected_write)?;
    }
    record_guard_diagnostic_best_effort(
        &runtime_home,
        &project,
        &envelope,
        phase,
        diagnostic_started,
        input.raw_text.len() as u64,
        &phase_result.result,
    );
    record_guard_workflow_metrics_best_effort(
        &runtime_home,
        &envelope,
        phase,
        phase_result.decision,
        &phase_result.result,
        false,
    );
    let rendered = render_guard_command_output(
        phase,
        phase_result.decision,
        &envelope,
        phase_result.result,
        &options,
        &runtime_home,
        &project,
    )?;
    if phase == GuardPhase::Stop && matches!(options.output, OutputFormat::HostNative(_)) {
        record_guard_probe_results_best_effort(
            &runtime_home,
            &envelope,
            &[(
                HostRuntimeProbeId::FixedUiAuthorityDisclosure,
                HostRuntimeProbeOutcome::Unavailable,
                HostRuntimeProbeFailureClass::ProbeNotRun,
            )],
        );
    }
    Ok(GuardCommandOutcome {
        stdout: rendered.stdout,
        stderr: rendered.stderr,
        exit_code: rendered.exit_code,
    })
}

fn render_guard_command_output(
    phase: GuardPhase,
    decision: GuardDecision,
    envelope: &GuardEnvelope,
    result: Value,
    options: &GuardOptions,
    runtime_home: &Path,
    project: &ProjectRecord,
) -> Result<RenderedGuardOutput, GuardCommandError> {
    let mut rendered = render_guard_output(phase, decision, envelope, result, options.output)?;
    let OutputFormat::HostNative(host_output) = options.output else {
        return Ok(rendered);
    };
    if phase != GuardPhase::Stop {
        return Ok(rendered);
    }

    let projection =
        guard_final_authority_projection(runtime_home, project, envelope, options, host_output);
    let base_response =
        serde_json::from_str::<Value>(rendered.stdout.trim()).map_err(json_error)?;
    match render_final_authority_response(&base_response, &projection) {
        Ok(authority_output) => {
            rendered.stdout = authority_output.stdout;
        }
        Err(_) => {
            let minimal_base = json!({"continue": true});
            let safe_projection = FinalAuthorityProjection::fallback(
                FinalAuthorityFallbackReason::RenderingUnavailable,
                None,
            );
            rendered.stdout = render_final_authority_response(&minimal_base, &safe_projection)
                .map_err(|error| {
                    GuardCommandError::Runtime(format!(
                        "failed to render the bounded final authority fallback: {error}"
                    ))
                })?
                .stdout;
        }
    }
    Ok(rendered)
}

fn guard_final_authority_projection(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    options: &GuardOptions,
    host_output: HostOutputMode,
) -> FinalAuthorityProjection {
    let coordinates = read_final_authority_coordinates(runtime_home, project).ok();
    let binding = match guard_final_output_binding_input(envelope, options, host_output) {
        Ok(binding) => binding,
        Err(reason) => return FinalAuthorityProjection::fallback(reason, coordinates),
    };
    project_managed_final_authority(
        runtime_home,
        &project.repo_root,
        &envelope.connection_id,
        binding.guard_installation_id,
        binding.host,
        IntegrationProfile::Detective,
        binding.expected_policy_hash,
        binding.output_host,
        coordinates,
    )
}

fn stop_invocation_binding_basis(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    options: &GuardOptions,
) -> &'static str {
    let OutputFormat::HostNative(host_output) = options.output else {
        return VERIFICATION_BASIS_UNREGISTERED_HOST_HOOK_EVENT;
    };
    let Ok(binding) = guard_final_output_binding_input(envelope, options, host_output) else {
        return VERIFICATION_BASIS_UNREGISTERED_HOST_HOOK_EVENT;
    };
    if managed_final_output_binding_is_verified(
        runtime_home,
        &project.repo_root,
        &envelope.connection_id,
        binding.guard_installation_id,
        binding.host,
        IntegrationProfile::Detective,
        binding.expected_policy_hash,
        binding.output_host,
    ) {
        VERIFICATION_BASIS_REGISTERED_HOST_STOP_HOOK_CONNECTION_BINDING
    } else {
        VERIFICATION_BASIS_UNREGISTERED_HOST_HOOK_EVENT
    }
}

struct GuardFinalOutputBindingInput<'a> {
    host: ManagedFinalOutputHost,
    output_host: ManagedFinalOutputHost,
    guard_installation_id: &'a str,
    expected_policy_hash: &'a str,
}

fn guard_final_output_binding_input<'a>(
    envelope: &'a GuardEnvelope,
    options: &'a GuardOptions,
    host_output: HostOutputMode,
) -> Result<GuardFinalOutputBindingInput<'a>, FinalAuthorityFallbackReason> {
    let Some(host) = managed_final_output_host(&envelope.host_kind) else {
        return Err(FinalAuthorityFallbackReason::BindingUnavailable);
    };
    let output_host = match host_output {
        HostOutputMode::Codex => ManagedFinalOutputHost::Codex,
        HostOutputMode::ClaudeCode => ManagedFinalOutputHost::ClaudeCode,
    };
    if envelope.guard_mode != IntegrationProfile::Detective.as_str() {
        return Err(FinalAuthorityFallbackReason::BindingUnavailable);
    }
    let (Some(guard_installation_id), Some(expected_policy_hash)) = (
        envelope.guard_installation_id.as_deref(),
        options.policy_hash.as_deref(),
    ) else {
        return Err(FinalAuthorityFallbackReason::BindingUnavailable);
    };
    Ok(GuardFinalOutputBindingInput {
        host,
        output_host,
        guard_installation_id,
        expected_policy_hash,
    })
}

fn managed_final_output_host(host_kind: &str) -> Option<ManagedFinalOutputHost> {
    match host_kind {
        "codex" => Some(ManagedFinalOutputHost::Codex),
        "claude_code" => Some(ManagedFinalOutputHost::ClaudeCode),
        _ => None,
    }
}

fn replayed_guard_phase_result(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
    subject: &Value,
) -> Result<Option<GuardPhaseResult>, GuardCommandError> {
    let Some(existing) = guard_event(runtime_home, &project.project_id, &envelope.event_id)? else {
        return Ok(None);
    };
    let existing_source = guard_event_source_payload_sha256(
        existing.session_id.as_deref(),
        &existing.connection_internal_id,
        existing.guard_installation_id.as_deref(),
        &existing.event_kind,
        &existing.subject_json,
    )?;
    let requested_source = guard_event_source_payload_sha256(
        envelope.session_id.as_deref(),
        &envelope.connection_id,
        envelope.guard_installation_id.as_deref(),
        phase.event_kind(),
        &object_text(subject.clone())?,
    )?;
    if existing_source != requested_source {
        return Err(GuardCommandError::Runtime(format!(
            "guard event {} conflicts with a different source payload hash",
            envelope.event_id
        )));
    }
    let decision = match existing.decision.as_str() {
        "allow" => GuardDecision::Allow,
        "deny" => GuardDecision::Deny,
        "warn" => GuardDecision::Warn,
        "inject_context" => GuardDecision::InjectContext,
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
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
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
    let prompt_capture_recorded = phase == GuardPhase::PromptCapture
        && result
            .get("recognized_user_action_command")
            .is_some_and(|value| !value.is_null());
    let prompt_capture_replayed = prompt_capture_recorded
        && result
            .pointer("/recognized_user_action_command/replayed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let product_file_write_count = (phase == GuardPhase::PostTool
        && result
            .pointer("/tool/changed_paths")
            .and_then(Value::as_array)
            .is_some_and(|paths| {
                paths
                    .iter()
                    .any(|path| path.get("inside_repo").and_then(Value::as_bool) == Some(true))
            })) as u64;
    let core_reached = prompt_capture_recorded
        || (phase == GuardPhase::Stop
            && result
                .pointer("/close_status/active_task")
                .is_some_and(|value| !value.is_null()));
    let core_committed = prompt_capture_recorded && !prompt_capture_replayed;
    let response_bytes = serde_json::to_vec(result)
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    let elapsed = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    let outcome = if authoritative_refresh_failure {
        DiagnosticOutcome::Unavailable
    } else if result.get("allowed").and_then(Value::as_bool) == Some(false) {
        DiagnosticOutcome::Rejected
    } else {
        DiagnosticOutcome::Success
    };
    if !start_guard_diagnostic_session_best_effort(runtime_home, project, envelope) {
        return;
    }
    let _ = record_diagnostic_event(
        runtime_home,
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
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
) -> bool {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return false;
    };
    let build = volicord_mcp::build_info();
    start_diagnostic_session(
        runtime_home,
        DiagnosticSessionStart {
            session_id,
            connection_id: Some(&envelope.connection_id),
            project_id: Some(&project.project_id),
            transport: DiagnosticTransport::GuardHook,
            host_kind: Some(DiagnosticHostKind::from_connection_host_kind(
                &envelope.host_kind,
            )),
            package_version: build.package_version,
            build_id: &build.build_id,
        },
    )
    .is_ok()
}

#[allow(clippy::too_many_arguments)]
fn record_guard_workflow_metrics_best_effort(
    runtime_home: &Path,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
    decision: GuardDecision,
    result: &Value,
    repeated_stop: bool,
) {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return;
    };
    let integration_profile = match envelope.guard_mode.as_str() {
        "record" => Some(IntegrationProfile::Record),
        "detective" => Some(IntegrationProfile::Detective),
        _ => None,
    };
    let record = |metric_kind: WorkflowMetricKind,
                  value: u64,
                  metric_decision: Option<WorkflowMetricDecision>,
                  observation_confidence: Option<ObservationConfidence>,
                  outcome: Option<WorkflowMetricOutcome>| {
        let _ = record_workflow_metric_event(
            runtime_home,
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
        GuardPhase::PreTool => {
            let confidence = result
                .pointer("/tool/confidence")
                .and_then(Value::as_str)
                .and_then(workflow_observation_confidence);
            let metric_decision = match decision {
                GuardDecision::Allow => Some(WorkflowMetricDecision::Allow),
                GuardDecision::Warn => Some(WorkflowMetricDecision::Warn),
                GuardDecision::Deny => Some(WorkflowMetricDecision::Deny),
                GuardDecision::InjectContext => None,
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
            if decision == GuardDecision::Deny
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
        GuardPhase::PostTool => {
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
        GuardPhase::Stop => {
            let refresh_succeeded = result
                .get("authoritative_refresh_succeeded")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let outcome = if refresh_succeeded {
                WorkflowMetricOutcome::Success
            } else {
                WorkflowMetricOutcome::Unavailable
            };
            record(WorkflowMetricKind::StopCall, 1, None, None, Some(outcome));
            if repeated_stop {
                record(WorkflowMetricKind::StopRepeat, 1, None, None, Some(outcome));
            } else {
                record(
                    WorkflowMetricKind::AuthorityRefresh,
                    1,
                    None,
                    None,
                    Some(outcome),
                );
            }
            if result
                .get("completion_claim_allowed")
                .and_then(Value::as_bool)
                == Some(false)
            {
                record(
                    WorkflowMetricKind::CompletionClaimSuppressed,
                    1,
                    None,
                    None,
                    Some(outcome),
                );
            }
        }
        GuardPhase::SessionStart | GuardPhase::PromptCapture => {}
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

fn ensure_required_session(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
) -> Result<(), GuardCommandError> {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return Ok(());
    };
    if agent_session(runtime_home, &project.project_id, session_id)?.is_some() {
        validate_existing_managed_session_binding(runtime_home, project, envelope)?;
        return Ok(());
    }
    if matches!(phase, GuardPhase::SessionStart | GuardPhase::PromptCapture)
        || envelope.session_id.is_some()
    {
        insert_agent_session(
            runtime_home,
            &project.project_id,
            AgentSessionInsert {
                session_id: session_id.to_owned(),
                connection_internal_id: envelope.connection_id.clone(),
                guard_installation_id: envelope.guard_installation_id.clone(),
                host_kind: envelope.host_kind.clone(),
                guard_mode: envelope.guard_mode.clone(),
                started_at: envelope.occurred_at.clone(),
                metadata_json: json!({
                    "source": "volicord_guard_cli"
                })
                .to_string(),
            },
        )?;
    }
    Ok(())
}

fn validate_existing_managed_session_binding(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
) -> Result<(), GuardCommandError> {
    if !is_managed_builtin_host(&envelope.host_kind) {
        return Ok(());
    }
    let Some(session_id) = envelope.session_id.as_deref() else {
        return Ok(());
    };
    let Some(existing) = agent_session(runtime_home, &project.project_id, session_id)? else {
        return Ok(());
    };
    if existing.connection_internal_id != envelope.connection_id
        || existing.host_kind != envelope.host_kind
    {
        return Err(GuardCommandError::Runtime(
            "MANAGED_HOST_SESSION_BINDING_CONFLICT: existing session ownership does not match this managed host connection"
                .to_owned(),
        ));
    }
    Ok(())
}

fn observe_guard_installation_activation(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
    options: &GuardOptions,
) -> Result<Option<volicord_store::guards::GuardInstallationRecord>, GuardCommandError> {
    if envelope.guard_mode == IntegrationProfile::Record.as_str() {
        return Ok(None);
    }
    let Some(guard_installation_id) = envelope.guard_installation_id.clone() else {
        return Ok(None);
    };
    let Some(observed_policy_hash) = current_policy_hash(project)? else {
        return Ok(None);
    };
    if options
        .policy_hash
        .as_deref()
        .is_some_and(|expected| expected != observed_policy_hash)
    {
        return Ok(None);
    }
    observe_guard_installation(
        runtime_home,
        GuardInstallationObservation {
            guard_installation_id,
            connection_internal_id: envelope.connection_id.clone(),
            project_id: project.project_id.clone(),
            host_kind: envelope.host_kind.clone(),
            guard_mode: envelope.guard_mode.clone(),
            observed_policy_hash,
            observed_binary_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            observed_phase: phase.event_kind().to_owned(),
            observed_at: envelope.occurred_at.clone(),
        },
    )
    .map_err(Into::into)
}

fn current_policy_hash(project: &ProjectRecord) -> Result<Option<String>, GuardCommandError> {
    let policy_path = project.repo_root.join(VOLICORD_POLICY_FILE);
    let text = match fs::read_to_string(&policy_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(GuardCommandError::Runtime(format!(
                "failed to read detective host hook policy {}: {error}",
                policy_path.display()
            )));
        }
    };
    let value = serde_json::from_str::<Value>(&text).map_err(|error| {
        GuardCommandError::Runtime(format!(
            "detective host hook policy is not valid JSON: {} ({error})",
            policy_path.display()
        ))
    })?;
    serde_json::to_string(&value)
        .map(|canonical| Some(sha256_text(&canonical)))
        .map_err(json_error)
}

fn persist_guard_event(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
    decision: GuardDecision,
    subject: Value,
    result: Value,
) -> Result<(), GuardCommandError> {
    let subject_json = object_text(subject)?;
    let source_payload_sha256 = guard_event_source_payload_sha256(
        envelope.session_id.as_deref(),
        &envelope.connection_id,
        envelope.guard_installation_id.as_deref(),
        phase.event_kind(),
        &subject_json,
    )?;
    let input = GuardEventInsert {
        guard_event_id: envelope.event_id.clone(),
        session_id: envelope.session_id.clone(),
        connection_internal_id: envelope.connection_id.clone(),
        guard_installation_id: envelope.guard_installation_id.clone(),
        event_kind: phase.event_kind().to_owned(),
        decision: decision.as_str().to_owned(),
        subject_json,
        result_json: object_text(result)?,
        occurred_at: envelope.occurred_at.clone(),
        metadata_json: json!({
            "source": "volicord_guard_cli",
            "source_payload_sha256": source_payload_sha256,
            "cooperative_detective": true
        })
        .to_string(),
    };
    if let Some(existing) = guard_event(runtime_home, &project.project_id, &envelope.event_id)? {
        if guard_event_record_payload_sha256(&existing)?
            == guard_event_insert_payload_sha256(&input)?
        {
            return Ok(());
        }
        return Err(GuardCommandError::Runtime(format!(
            "guard event {} conflicts with a different payload hash",
            envelope.event_id
        )));
    }
    insert_guard_event(runtime_home, &project.project_id, input)?;
    Ok(())
}

fn record_guard_runtime_probes_best_effort(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
    phase: GuardPhase,
    event: &Value,
    repeated_stop: bool,
) {
    let mut observations = vec![(
        HostRuntimeProbeId::LifecycleHookDelivery,
        HostRuntimeProbeOutcome::Passed,
        HostRuntimeProbeFailureClass::None,
    )];
    match phase {
        GuardPhase::PreTool => {
            if let Some(observation) = pre_tool_structured_paths_probe(project, event) {
                observations.push(observation);
            }
        }
        GuardPhase::PostTool => {
            if let Some(observation) = post_tool_structured_paths_probe(project, event) {
                observations.push(observation);
            }
        }
        GuardPhase::Stop => observations.push(
            if repeated_stop
                || event.get("stop_hook_active").and_then(Value::as_bool) == Some(true)
                || prior_stop_delivery_exists(runtime_home, project, envelope)
            {
                (
                    HostRuntimeProbeId::StopDeliveryAndReplay,
                    HostRuntimeProbeOutcome::Failed,
                    HostRuntimeProbeFailureClass::SecondStopRequested,
                )
            } else {
                (
                    HostRuntimeProbeId::StopDeliveryAndReplay,
                    HostRuntimeProbeOutcome::Unavailable,
                    HostRuntimeProbeFailureClass::ProbeNotRun,
                )
            },
        ),
        GuardPhase::SessionStart | GuardPhase::PromptCapture => {}
    }

    record_guard_probe_results_best_effort(runtime_home, envelope, &observations);
}

type GuardProbeResult = (
    HostRuntimeProbeId,
    HostRuntimeProbeOutcome,
    HostRuntimeProbeFailureClass,
);

fn pre_tool_structured_paths_probe(
    project: &ProjectRecord,
    event: &Value,
) -> Option<GuardProbeResult> {
    let observation = tool_observation(event, &project.repo_root);
    if !tool_name_is_direct_write(observation.tool_name.as_deref()) {
        return None;
    }
    Some(if observation.structured_paths.is_empty() {
        (
            HostRuntimeProbeId::PreToolStructuredTargetPaths,
            HostRuntimeProbeOutcome::Failed,
            HostRuntimeProbeFailureClass::StructuredPathsMissing,
        )
    } else {
        (
            HostRuntimeProbeId::PreToolStructuredTargetPaths,
            HostRuntimeProbeOutcome::Passed,
            HostRuntimeProbeFailureClass::None,
        )
    })
}

fn post_tool_structured_paths_probe(
    project: &ProjectRecord,
    event: &Value,
) -> Option<GuardProbeResult> {
    let observation = tool_observation(event, &project.repo_root);
    if !tool_name_is_direct_write(observation.tool_name.as_deref())
        || !post_tool_reports_successful_write(&observation)
        || (observation.changed_paths_reported && observation.changed_paths.is_empty())
    {
        return None;
    }
    Some(
        if observation.changed_paths_reported && !observation.changed_paths.is_empty() {
            (
                HostRuntimeProbeId::PostToolStructuredChangedPaths,
                HostRuntimeProbeOutcome::Passed,
                HostRuntimeProbeFailureClass::None,
            )
        } else {
            (
                HostRuntimeProbeId::PostToolStructuredChangedPaths,
                HostRuntimeProbeOutcome::Failed,
                HostRuntimeProbeFailureClass::StructuredPathsMissing,
            )
        },
    )
}

fn post_tool_reports_successful_write(observation: &ToolObservation) -> bool {
    !observation.changed_paths.is_empty()
        || observation.success == Some(true)
        || observation.exit_code == Some(0)
        || observation.status.as_deref().is_some_and(|status| {
            matches!(
                status.to_ascii_lowercase().as_str(),
                "complete" | "completed" | "ok" | "success" | "succeeded"
            )
        })
}

fn prior_stop_delivery_exists(
    runtime_home: &Path,
    project: &ProjectRecord,
    envelope: &GuardEnvelope,
) -> bool {
    let Some(session_id) = envelope.session_id.as_deref() else {
        return false;
    };
    prior_guard_event_exists_for_session_kind(
        runtime_home,
        &project.project_id,
        session_id,
        &envelope.connection_id,
        "stop",
        &envelope.event_id,
    )
    .unwrap_or(false)
}

fn record_guard_probe_results_best_effort(
    runtime_home: &Path,
    envelope: &GuardEnvelope,
    observations: &[(
        HostRuntimeProbeId,
        HostRuntimeProbeOutcome,
        HostRuntimeProbeFailureClass,
    )],
) {
    if !is_managed_builtin_host(&envelope.host_kind) {
        return;
    }
    let Ok(Some(connection)) =
        agent_connection_record_read_only(runtime_home, &envelope.connection_id)
    else {
        return;
    };
    let adapter_profile = match envelope.guard_mode.as_str() {
        "record" => IntegrationProfile::Record,
        "detective" => IntegrationProfile::Detective,
        _ => return,
    };
    let now = UtcTimestamp::from_datetime(DateTime::<Utc>::from(std::time::SystemTime::now()));
    let Ok(expires_at) = now.checked_add(chrono::Duration::hours(1)) else {
        return;
    };
    let host_version = serde_json::from_str::<Value>(&connection.last_verification_report_json)
        .ok()
        .and_then(|report| {
            report
                .pointer("/host/host_version")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    for &(probe_id, outcome, failure_class) in observations {
        let _ = record_host_runtime_probe_observation(
            runtime_home,
            HostRuntimeProbeObservation {
                probe_id,
                outcome,
                failure_class,
                connection_internal_id: connection.connection_internal_id.clone(),
                host_kind: connection.host_kind.clone(),
                host_version: host_version.clone(),
                client_name: None,
                client_version: None,
                adapter_profile,
                adapter_version: env!("CARGO_PKG_VERSION").to_owned(),
                managed_fingerprint: connection.managed_fingerprint.clone(),
                observed_at: now.clone(),
                expires_at: expires_at.clone(),
            },
        );
    }
}

fn guard_subject(
    phase: GuardPhase,
    input: &GuardInput,
    envelope: &GuardEnvelope,
    project: &ProjectRecord,
) -> Value {
    json!({
        "lifecycle_phase": phase.event_kind(),
        "host_kind": envelope.host_kind,
        "connection_id": envelope.connection_id,
        "project_id": project.project_id,
        "repo_root": project.repo_root.display().to_string(),
        "raw_event_sha256": input.raw_sha256,
        "tool_input_sha256": guard_event_tool_input(&input.raw_value).map(canonical_value_sha256),
        "tool_result_sha256": guard_event_tool_result(&input.raw_value).map(canonical_value_sha256),
        "tool_result_size_bytes": guard_event_tool_result(&input.raw_value)
            .and_then(|value| canonical_json_bytes(value).ok())
            .and_then(|bytes| u64::try_from(bytes.len()).ok()),
        "raw_event": input.redacted_value
    })
}

fn protect_managed_guard_input(
    mut input: GuardInput,
    envelope: &GuardEnvelope,
) -> Result<GuardInput, GuardCommandError> {
    if !is_managed_builtin_host(&envelope.host_kind) {
        return Ok(input);
    }
    let native_session_id =
        managed_native_session_id(&envelope.host_kind, &input.raw_value)?.to_owned();
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
) -> Result<String, GuardCommandError> {
    guard_event_payload_sha256(
        input.session_id.as_deref(),
        &input.connection_internal_id,
        input.guard_installation_id.as_deref(),
        &input.event_kind,
        &input.decision,
        &input.subject_json,
    )
}

fn guard_event_record_payload_sha256(
    record: &volicord_store::guards::GuardEventRecord,
) -> Result<String, GuardCommandError> {
    guard_event_payload_sha256(
        record.session_id.as_deref(),
        &record.connection_internal_id,
        record.guard_installation_id.as_deref(),
        &record.event_kind,
        &record.decision,
        &record.subject_json,
    )
}

fn guard_event_payload_sha256(
    session_id: Option<&str>,
    connection_id: &str,
    guard_installation_id: Option<&str>,
    event_kind: &str,
    decision: &str,
    subject_json: &str,
) -> Result<String, GuardCommandError> {
    let source_sha256 = guard_event_source_payload_sha256(
        session_id,
        connection_id,
        guard_installation_id,
        event_kind,
        subject_json,
    )?;
    Ok(canonical_value_sha256(&json!({
        "source_sha256": source_sha256,
        "decision": decision,
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

#[cfg(test)]
mod replay_tests {
    use std::error::Error;

    use super::*;
    use volicord_store::host_runtime_probes::host_runtime_probe_snapshot_read_only;
    use volicord_test_support::core_fixtures::CoreFixture;

    #[test]
    fn final_model_prose_is_redacted_from_guard_subjects() {
        let sentinel = "FORGED_AUTHORITY_RECEIPT_SENTINEL";
        let redacted = redact_event_value(&json!({
            "last_assistant_message": sentinel,
            "assistant_message": sentinel,
            "nested": {"transcript": sentinel}
        }));
        let serialized = serde_json::to_string(&redacted).expect("redacted event serializes");

        assert!(!serialized.contains(sentinel));
        assert_eq!(redacted["last_assistant_message"]["omitted"], true);
        assert_eq!(redacted["assistant_message"]["omitted"], true);
        assert_eq!(redacted["nested"]["transcript"]["omitted"], true);
    }

    #[test]
    fn managed_guard_events_publish_actual_runtime_probe_results() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("guard-runtime-probes")?;
        let envelope = GuardEnvelope {
            event_id: "guard_event_probe".to_owned(),
            session_id: Some("session_probe".to_owned()),
            connection_id: fixture.connection_id().to_owned(),
            guard_installation_id: Some("guard_probe".to_owned()),
            host_kind: "codex".to_owned(),
            guard_mode: "detective".to_owned(),
            occurred_at: "2026-07-16T00:00:00Z".to_owned(),
        };
        let project =
            project_record_for_execution(fixture.runtime_home_path(), fixture.project_id())?
                .expect("fixture project");
        assert_eq!(
            pre_tool_structured_paths_probe(
                &project,
                &json!({"tool_name": "bash", "command": "git status"}),
            ),
            None
        );
        assert_eq!(
            pre_tool_structured_paths_probe(&project, &json!({"tool_name": "edit"})),
            Some((
                HostRuntimeProbeId::PreToolStructuredTargetPaths,
                HostRuntimeProbeOutcome::Failed,
                HostRuntimeProbeFailureClass::StructuredPathsMissing,
            ))
        );
        assert_eq!(
            post_tool_structured_paths_probe(
                &project,
                &json!({"tool_name": "edit", "success": true}),
            ),
            Some((
                HostRuntimeProbeId::PostToolStructuredChangedPaths,
                HostRuntimeProbeOutcome::Failed,
                HostRuntimeProbeFailureClass::StructuredPathsMissing,
            ))
        );
        assert_eq!(
            post_tool_structured_paths_probe(
                &project,
                &json!({"tool_name": "edit", "success": true, "changed_paths": []}),
            ),
            None
        );
        record_guard_runtime_probes_best_effort(
            fixture.runtime_home_path(),
            &project,
            &envelope,
            GuardPhase::PreTool,
            &json!({"tool_name": "edit", "target_path": ["src/lib.rs"]}),
            false,
        );
        record_guard_runtime_probes_best_effort(
            fixture.runtime_home_path(),
            &project,
            &envelope,
            GuardPhase::PostTool,
            &json!({
                "tool_name": "edit",
                "success": true,
                "changed_paths": ["src/lib.rs"]
            }),
            false,
        );
        for (phase, event) in [
            (
                GuardPhase::PreTool,
                json!({"tool_name": "bash", "command": "git status"}),
            ),
            (
                GuardPhase::PostTool,
                json!({"tool_name": "bash", "command": "git status", "success": true}),
            ),
            (
                GuardPhase::PostTool,
                json!({"tool_name": "edit", "success": true, "changed_paths": []}),
            ),
        ] {
            record_guard_runtime_probes_best_effort(
                fixture.runtime_home_path(),
                &project,
                &envelope,
                phase,
                &event,
                false,
            );
        }
        record_guard_runtime_probes_best_effort(
            fixture.runtime_home_path(),
            &project,
            &envelope,
            GuardPhase::Stop,
            &json!({}),
            false,
        );

        let snapshot = host_runtime_probe_snapshot_read_only(
            fixture.runtime_home_path(),
            fixture.connection_id(),
        )?
        .expect("fixture connection has a probe snapshot");
        let outcome = |probe_id| {
            snapshot
                .observations
                .iter()
                .find(|observation| observation.probe_id == probe_id)
                .map(|observation| (observation.outcome, observation.failure_class))
        };
        assert_eq!(
            outcome(HostRuntimeProbeId::LifecycleHookDelivery),
            Some((
                HostRuntimeProbeOutcome::Passed,
                HostRuntimeProbeFailureClass::None,
            ))
        );
        assert_eq!(
            outcome(HostRuntimeProbeId::PreToolStructuredTargetPaths),
            Some((
                HostRuntimeProbeOutcome::Passed,
                HostRuntimeProbeFailureClass::None,
            ))
        );
        assert_eq!(
            outcome(HostRuntimeProbeId::PostToolStructuredChangedPaths),
            Some((
                HostRuntimeProbeOutcome::Passed,
                HostRuntimeProbeFailureClass::None,
            ))
        );
        assert_eq!(
            outcome(HostRuntimeProbeId::StopDeliveryAndReplay),
            Some((
                HostRuntimeProbeOutcome::Unavailable,
                HostRuntimeProbeFailureClass::ProbeNotRun,
            ))
        );
        Ok(())
    }

    #[test]
    fn managed_native_identifiers_are_opaque_across_every_event_value() {
        let native_session_id = "native.session:secret-1";
        let managed_session_id =
            "mhs_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let mut event = json!({
            "session_id": native_session_id,
            "thread_id": native_session_id,
            "event_id": "native-event-id",
            "tool_call_id": "native-tool-id",
            "host_invocation_id": "native-host-invocation-id",
            "tool_result": {"tool_call_id": "native-tool-id"},
            "tool": {"id": "native-tool-id"},
            "project": {"id": "project_canonical"},
            "connection": {"id": "connection_canonical"},
            "prompt_capture_id": "native-capture-id",
            "turn_id": "native-turn-id",
            "transcript_path": format!("/tmp/{native_session_id}.jsonl"),
            "repo_root": "/tmp/native-event-id/product-repositories/repo",
            "nested": [
                native_session_id,
                {"value": native_session_id},
                {"event_echo": "prefix-native-event-id-suffix"},
                {"tool_echo": "prefix-native-tool-id-suffix"},
                {"capture_echo": "prefix-native-capture-id-suffix"},
                {"turn_echo": "prefix-native-turn-id-suffix"},
                {"invocation_echo": "prefix-native-host-invocation-id-suffix"}
            ],
        });
        event.as_object_mut().expect("managed event object").insert(
            "dynamic-native-tool-id-key".to_owned(),
            json!({"value": "must be omitted with its native identifier key"}),
        );
        let replacements = managed_native_identifier_replacements(
            &event,
            managed_session_id,
            "guard_event_opaque",
            "connection_test",
            native_session_id,
        );
        let semantic_context = ManagedEventProtectionContext {
            managed_session_id,
            guard_event_id: "guard_event_opaque",
            connection_id: "connection_test",
            protection: ManagedEventProtection::Semantic,
            replacements: &replacements,
        };
        let semantic = protect_managed_event_value(&event, None, None, &semantic_context);
        let persistent_context = ManagedEventProtectionContext {
            protection: ManagedEventProtection::Persistent,
            ..semantic_context
        };
        let sanitized = protect_managed_event_value(&event, None, None, &persistent_context);
        let serialized = serde_json::to_string(&sanitized).expect("sanitized event serializes");

        assert_eq!(semantic["session_id"], managed_session_id);
        assert_eq!(
            semantic["transcript_path"],
            format!("/tmp/{native_session_id}.jsonl")
        );
        assert_eq!(
            semantic["repo_root"],
            "/tmp/native-event-id/product-repositories/repo"
        );
        assert!(!serialized.contains(native_session_id));
        for native_identifier in [
            "native-event-id",
            "native-tool-id",
            "native-capture-id",
            "native-turn-id",
            "native-host-invocation-id",
        ] {
            assert!(!serialized.contains(native_identifier));
        }
        assert_eq!(sanitized["session_id"], sanitized["thread_id"]);
        assert_eq!(sanitized["event_id"], "guard_event_opaque");
        assert_eq!(
            sanitized["tool_call_id"],
            sanitized["tool_result"]["tool_call_id"]
        );
        assert_eq!(sanitized["tool_call_id"], sanitized["tool"]["id"]);
        assert_eq!(sanitized["project"]["id"], "project_canonical");
        assert_eq!(sanitized["connection"]["id"], "connection_canonical");
    }

    #[test]
    fn guard_event_replay_hash_is_idempotent_for_same_source_and_conflicts_for_changed_payload() {
        let first = GuardEventInsert {
            guard_event_id: "guard_event_replay".to_owned(),
            session_id: Some("session_replay".to_owned()),
            connection_internal_id: "connection_replay".to_owned(),
            guard_installation_id: Some("guard_replay".to_owned()),
            event_kind: "post_tool".to_owned(),
            decision: "allow".to_owned(),
            subject_json: json!({
                "raw_event_sha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "raw_event": {"tool_use_id": "same"}
            })
            .to_string(),
            result_json: json!({"state": "first-render"}).to_string(),
            occurred_at: "2026-07-13T00:00:00Z".to_owned(),
            metadata_json: "{}".to_owned(),
        };
        let mut same_source = first.clone();
        same_source.result_json = json!({"state": "later-render"}).to_string();
        same_source.occurred_at = "2026-07-13T00:00:01Z".to_owned();
        assert_eq!(
            guard_event_insert_payload_sha256(&first).expect("first replay hash"),
            guard_event_insert_payload_sha256(&same_source).expect("same-source replay hash")
        );

        let mut changed_payload = first.clone();
        changed_payload.subject_json = json!({
            "raw_event_sha256": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "raw_event": {"tool_use_id": "changed"}
        })
        .to_string();
        assert_ne!(
            guard_event_insert_payload_sha256(&first).expect("first replay hash"),
            guard_event_insert_payload_sha256(&changed_payload)
                .expect("changed-payload replay hash")
        );

        let mut changed_decision = first.clone();
        changed_decision.decision = "deny".to_owned();
        assert_eq!(
            guard_event_source_payload_sha256(
                first.session_id.as_deref(),
                &first.connection_internal_id,
                first.guard_installation_id.as_deref(),
                &first.event_kind,
                &first.subject_json,
            )
            .expect("first source hash"),
            guard_event_source_payload_sha256(
                changed_decision.session_id.as_deref(),
                &changed_decision.connection_internal_id,
                changed_decision.guard_installation_id.as_deref(),
                &changed_decision.event_kind,
                &changed_decision.subject_json,
            )
            .expect("changed-decision source hash"),
            "an exact Stop replay reuses the immutable historical decision"
        );
        assert_ne!(
            guard_event_insert_payload_sha256(&first).expect("first replay hash"),
            guard_event_insert_payload_sha256(&changed_decision)
                .expect("changed-decision replay hash")
        );
    }
}
