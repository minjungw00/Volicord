use std::{
    ffi::OsString,
    fmt, fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use volicord_core::{
    validate_authority_status, AuthorityStatusExpectation, CoreService, InvocationContext,
};
use volicord_store::{
    agent_connections::{
        agent_connection_project_access_read_only, agent_connection_record_read_only,
        CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW,
    },
    bootstrap::{project_record_by_repo_root_read_only, ProjectRecord, ACTIVE_PROJECT_STATUS},
    core_pipeline::CoreProjectStore,
    guards::guard_installation,
    runtime_home::resolve_runtime_home,
};
#[cfg(test)]
use volicord_types::HOST_HOOK_CAPABILITY_SCHEMA;
use volicord_types::{
    canonical_json_bare_sha256, canonical_json_string, host_hook_capability_matches_owner_binding,
    ActorSource, AuthorityReceipt, EffectKind, HostHookCapabilityOwnerBinding, IntegrationProfile,
    OperationCategory, ProjectId, RequestId, ResponseKind, StateRecordKind, StatusInclude,
    StatusRequest, StatusResult, TaskId, ToolEnvelope,
    VERIFICATION_BASIS_REGISTERED_HOST_STOP_HOOK_CONNECTION_BINDING,
};

use crate::guard_integration::{
    audit::policy_hash,
    files::VOLICORD_POLICY_FILE,
    git_exclude::git_exclude_path,
    policy::{recorded_local_policy, RecordedLocalPolicy},
};

/// Maximum complete host-native final-output response size, including its final LF.
pub const MAX_FINAL_AUTHORITY_HOST_RESPONSE_BYTES: usize = 8 * 1024;

const AUTHORITY_RECEIPT_PREFIX: &str = "Volicord authority receipt: ";
const GENERIC_FALLBACK_WIRE: &str = "{\"continue\":true,\"systemMessage\":\"Volicord authority disclosure unavailable (rendering_unavailable). Inspect current authority with `volicord status --json`.\"}\n";

/// Supported managed host for a final-output authority disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedFinalOutputHost {
    Codex,
    ClaudeCode,
}

impl ManagedFinalOutputHost {
    fn from_cli(value: &str, option: &'static str) -> Result<Self, FinalOutputCommandError> {
        match value {
            "codex" => Ok(Self::Codex),
            "claude-code" => Ok(Self::ClaudeCode),
            _ => Err(FinalOutputCommandError::Usage(format!(
                "{option} must be codex or claude-code"
            ))),
        }
    }

    /// Returns the Registry host-kind value.
    pub const fn registry_value(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude_code",
        }
    }

    /// Returns the managed policy and CLI host label.
    pub const fn cli_value(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
        }
    }
}

/// Safe coordinates available to a bounded authority-disclosure fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalAuthorityCoordinates {
    pub project_id: String,
    pub task_id: Option<String>,
    pub state_version: Option<u64>,
}

/// Closed, non-private reason classes for a final-output fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalAuthorityFallbackReason {
    EventUnavailable,
    BindingUnavailable,
    AdapterUnavailable,
    StatusRefreshUnavailable,
    StatusValidationFailed,
    ReceiptExceedsHostUiBudget,
    RenderingUnavailable,
}

impl FinalAuthorityFallbackReason {
    /// Returns the stable diagnostic label rendered in the fixed UI fallback.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EventUnavailable => "event_unavailable",
            Self::BindingUnavailable => "binding_unavailable",
            Self::AdapterUnavailable => "adapter_unavailable",
            Self::StatusRefreshUnavailable => "status_refresh_unavailable",
            Self::StatusValidationFailed => "status_validation_failed",
            Self::ReceiptExceedsHostUiBudget => "receipt_exceeds_host_ui_budget",
            Self::RenderingUnavailable => "rendering_unavailable",
        }
    }
}

/// Fresh read-only authority projection prepared for a host fixed-UI response.
#[derive(Debug, Clone, PartialEq)]
pub enum FinalAuthorityProjection {
    Receipt(Box<AuthorityReceipt>),
    NoActiveTask(FinalAuthorityCoordinates),
    Fallback {
        reason: FinalAuthorityFallbackReason,
        coordinates: Option<FinalAuthorityCoordinates>,
    },
}

impl FinalAuthorityProjection {
    pub(crate) fn fallback(
        reason: FinalAuthorityFallbackReason,
        coordinates: Option<FinalAuthorityCoordinates>,
    ) -> Self {
        Self::Fallback {
            reason,
            coordinates,
        }
    }
}

/// Result of rendering one complete host-native authority response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFinalAuthorityOutput {
    pub stdout: String,
    pub receipt_displayed: bool,
    pub fallback_reason: Option<FinalAuthorityFallbackReason>,
}

/// Rendering failure that carries no receipt, response, event, or private error text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalAuthorityRenderError {
    HostResponseIsNotObject,
    SerializationFailed,
    FallbackExceedsHostUiBudget,
}

impl fmt::Display for FinalAuthorityRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::HostResponseIsNotObject => "host response is not an object",
            Self::SerializationFailed => "host response serialization failed",
            Self::FallbackExceedsHostUiBudget => "host fallback exceeds the UI byte budget",
        })
    }
}

impl std::error::Error for FinalAuthorityRenderError {}

/// Hidden command outcome. Managed final-output fallbacks are successful host responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalOutputCommandOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Hidden command error. Runtime failures are rendered as safe fallbacks, not returned here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinalOutputCommandError {
    Usage(String),
}

impl fmt::Display for FinalOutputCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for FinalOutputCommandError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FinalOutputOptions {
    event_file: Option<PathBuf>,
    repo: PathBuf,
    connection_id: String,
    guard_installation_id: String,
    host: ManagedFinalOutputHost,
    profile: IntegrationProfile,
    policy_hash: String,
    host_output: ManagedFinalOutputHost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingFailure {
    Unavailable,
    AdapterUnavailable,
}

struct VerifiedFinalOutputBinding {
    project: ProjectRecord,
    connection_id: String,
}

/// Returns the hidden managed final-output command usage.
pub fn final_output_usage() -> String {
    "volicord _final-output [--file PATH] --repo PATH --connection ID --guard-installation ID --host codex|claude-code --integration-profile record|detective --policy-hash HASH --host-output codex|claude-code\n".to_owned()
}

/// Runs the non-observing managed final-output authority-disclosure command.
pub fn run_final_output_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<FinalOutputCommandOutcome, FinalOutputCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    if matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    ) {
        if args.len() == 1 {
            return Ok(FinalOutputCommandOutcome {
                stdout: final_output_usage(),
                stderr: String::new(),
                exit_code: 0,
            });
        }
        return Err(FinalOutputCommandError::Usage(format!(
            "unexpected argument: {}\n\n{}",
            args[1],
            final_output_usage()
        )));
    }
    let options = parse_options(args)?;
    let event_available = drain_event(options.event_file.as_deref()).is_ok();

    let Ok(runtime_home) = resolve_runtime_home(env_var, current_dir) else {
        return Ok(command_outcome(FinalAuthorityProjection::fallback(
            if event_available {
                FinalAuthorityFallbackReason::BindingUnavailable
            } else {
                FinalAuthorityFallbackReason::EventUnavailable
            },
            None,
        )));
    };
    let project = project_record_by_repo_root_read_only(&runtime_home, &options.repo)
        .ok()
        .flatten();
    let coordinates = project
        .as_ref()
        .and_then(|project| read_final_authority_coordinates(&runtime_home, project).ok());

    if !event_available {
        return Ok(command_outcome(FinalAuthorityProjection::fallback(
            FinalAuthorityFallbackReason::EventUnavailable,
            coordinates,
        )));
    }
    let Some(_project) = project else {
        return Ok(command_outcome(FinalAuthorityProjection::fallback(
            FinalAuthorityFallbackReason::BindingUnavailable,
            None,
        )));
    };
    Ok(command_outcome(project_managed_final_authority(
        &runtime_home,
        &options.repo,
        &options.connection_id,
        &options.guard_installation_id,
        options.host,
        options.profile,
        &options.policy_hash,
        options.host_output,
        coordinates,
    )))
}

/// Verifies one managed binding and projects one fresh authority disclosure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn project_managed_final_authority(
    runtime_home: &Path,
    repo: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    host: ManagedFinalOutputHost,
    profile: IntegrationProfile,
    expected_policy_hash: &str,
    host_output: ManagedFinalOutputHost,
    fallback_coordinates: Option<FinalAuthorityCoordinates>,
) -> FinalAuthorityProjection {
    match verify_managed_final_output_binding(
        runtime_home,
        repo,
        connection_id,
        guard_installation_id,
        host,
        profile,
        expected_policy_hash,
        host_output,
    ) {
        Ok(binding) => {
            refresh_final_authority_projection(runtime_home, binding, fallback_coordinates)
        }
        Err(reason) => FinalAuthorityProjection::fallback(reason, fallback_coordinates),
    }
}

/// Returns whether the complete managed binding is currently verified.
#[allow(clippy::too_many_arguments)]
pub(crate) fn managed_final_output_binding_is_verified(
    runtime_home: &Path,
    repo: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    host: ManagedFinalOutputHost,
    profile: IntegrationProfile,
    expected_policy_hash: &str,
    host_output: ManagedFinalOutputHost,
) -> bool {
    verify_managed_final_output_binding(
        runtime_home,
        repo,
        connection_id,
        guard_installation_id,
        host,
        profile,
        expected_policy_hash,
        host_output,
    )
    .is_ok()
}

/// Performs one fresh Core status read for a just-verified managed binding.
///
/// This function does not parse or retain a host event and performs no guard,
/// session, activation, watcher, diagnostic, event, replay, or Core mutation write.
fn refresh_final_authority_projection(
    runtime_home: &Path,
    binding: VerifiedFinalOutputBinding,
    fallback_coordinates: Option<FinalAuthorityCoordinates>,
) -> FinalAuthorityProjection {
    let project = &binding.project;
    let connection_id = &binding.connection_id;
    let request_digest = canonical_json_bare_sha256(&json!({
        "project_id": project.project_id,
        "connection_id": connection_id,
        "purpose": "final_authority_status"
    }))
    .unwrap_or_else(|_| "000000000000000000000000".to_owned());
    let request_id = format!(
        "req_final_authority_status_{}",
        &request_digest[..request_digest.len().min(24)]
    );
    let response = match CoreService::new(runtime_home).status(
        StatusRequest {
            envelope: ToolEnvelope {
                project_id: ProjectId::new(&project.project_id),
                task_id: None.into(),
                request_id: RequestId::new(request_id),
                idempotency_key: None.into(),
                expected_state_version: None.into(),
                dry_run: false,
                locale: None.into(),
            },
            include: StatusInclude {
                task: true,
                pending_user_actions: true,
                write_ticket: true,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: false,
            },
        },
        InvocationContext::new(
            ProjectId::new(&project.project_id),
            ActorSource::agent_connection(connection_id.to_owned()),
            OperationCategory::Read,
            VERIFICATION_BASIS_REGISTERED_HOST_STOP_HOOK_CONNECTION_BINDING,
        ),
    ) {
        Ok(response) => response,
        Err(_) => {
            return FinalAuthorityProjection::fallback(
                FinalAuthorityFallbackReason::StatusRefreshUnavailable,
                fallback_coordinates,
            )
        }
    };

    projection_from_status(&response.response_value, project, fallback_coordinates)
}

/// Read-only verifies the complete managed binding used by a final-output adapter.
///
/// The failure classification contains no Store, policy, response, event, or
/// private error text. Only this module can turn the opaque success into the
/// controlled registered-host verification basis.
#[allow(clippy::too_many_arguments)]
fn verify_managed_final_output_binding(
    runtime_home: &Path,
    repo: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    host: ManagedFinalOutputHost,
    profile: IntegrationProfile,
    expected_policy_hash: &str,
    host_output: ManagedFinalOutputHost,
) -> Result<VerifiedFinalOutputBinding, FinalAuthorityFallbackReason> {
    if host != host_output {
        return Err(FinalAuthorityFallbackReason::BindingUnavailable);
    }
    let project = project_record_by_repo_root_read_only(runtime_home, repo)
        .map_err(|_| FinalAuthorityFallbackReason::BindingUnavailable)?
        .ok_or(FinalAuthorityFallbackReason::BindingUnavailable)?;
    let options = FinalOutputOptions {
        event_file: None,
        repo: repo.to_path_buf(),
        connection_id: connection_id.to_owned(),
        guard_installation_id: guard_installation_id.to_owned(),
        host,
        profile,
        policy_hash: expected_policy_hash.to_owned(),
        host_output,
    };
    verify_binding(runtime_home, &project, &options).map_err(|error| match error {
        BindingFailure::Unavailable => FinalAuthorityFallbackReason::BindingUnavailable,
        BindingFailure::AdapterUnavailable => FinalAuthorityFallbackReason::AdapterUnavailable,
    })?;
    Ok(VerifiedFinalOutputBinding {
        project,
        connection_id: connection_id.to_owned(),
    })
}

/// Adds a complete receipt or bounded fallback to a host-native response object.
///
/// The returned string is compact JSON with one terminating LF. The byte budget
/// applies to that whole wire form. Receipt JSON is never partially emitted.
pub fn render_final_authority_response(
    base_response: &Value,
    projection: &FinalAuthorityProjection,
) -> Result<RenderedFinalAuthorityOutput, FinalAuthorityRenderError> {
    render_final_authority_response_with_limit(
        base_response,
        projection,
        MAX_FINAL_AUTHORITY_HOST_RESPONSE_BYTES,
    )
}

fn render_final_authority_response_with_limit(
    base_response: &Value,
    projection: &FinalAuthorityProjection,
    max_bytes: usize,
) -> Result<RenderedFinalAuthorityOutput, FinalAuthorityRenderError> {
    match projection {
        FinalAuthorityProjection::Receipt(receipt) => {
            let canonical_receipt = canonical_json_string(receipt)
                .map_err(|_| FinalAuthorityRenderError::SerializationFailed)?;
            let message = format!("{AUTHORITY_RECEIPT_PREFIX}{canonical_receipt}");
            let candidate = serialize_host_response(base_response, &message)?;
            if candidate.len() <= max_bytes {
                return Ok(RenderedFinalAuthorityOutput {
                    stdout: candidate,
                    receipt_displayed: true,
                    fallback_reason: None,
                });
            }
            let coordinates = FinalAuthorityCoordinates {
                project_id: receipt.project_id.as_str().to_owned(),
                task_id: Some(receipt.task_ref.record_id.as_str().to_owned()),
                state_version: Some(receipt.state_version),
            };
            render_fallback(
                base_response,
                FinalAuthorityFallbackReason::ReceiptExceedsHostUiBudget,
                Some(&coordinates),
                max_bytes,
            )
        }
        FinalAuthorityProjection::NoActiveTask(coordinates) => {
            render_no_active_task(base_response, Some(coordinates), None, max_bytes)
        }
        FinalAuthorityProjection::Fallback {
            reason,
            coordinates,
        } => render_fallback(base_response, *reason, coordinates.as_ref(), max_bytes),
    }
}

fn projection_from_status(
    response: &Value,
    project: &ProjectRecord,
    fallback_coordinates: Option<FinalAuthorityCoordinates>,
) -> FinalAuthorityProjection {
    let Ok(status) = serde_json::from_value::<StatusResult>(response.clone()) else {
        return FinalAuthorityProjection::fallback(
            FinalAuthorityFallbackReason::StatusValidationFailed,
            fallback_coordinates,
        );
    };
    if status.base.response_kind != ResponseKind::Result
        || status.base.effect_kind != EffectKind::ReadOnly
        || status.base.dry_run
    {
        return FinalAuthorityProjection::fallback(
            FinalAuthorityFallbackReason::StatusValidationFailed,
            fallback_coordinates,
        );
    }
    let Some(state_version) = status.base.state_version else {
        return FinalAuthorityProjection::fallback(
            FinalAuthorityFallbackReason::StatusValidationFailed,
            fallback_coordinates,
        );
    };
    let Some(active_task) = status.active_task.as_ref() else {
        if status.authority_receipt.is_some() {
            return FinalAuthorityProjection::fallback(
                FinalAuthorityFallbackReason::StatusValidationFailed,
                fallback_coordinates,
            );
        }
        return FinalAuthorityProjection::NoActiveTask(FinalAuthorityCoordinates {
            project_id: project.project_id.clone(),
            task_id: None,
            state_version: Some(state_version),
        });
    };
    let Some(task_ref) = active_task.task_ref.as_ref() else {
        return FinalAuthorityProjection::fallback(
            FinalAuthorityFallbackReason::StatusValidationFailed,
            fallback_coordinates,
        );
    };
    if task_ref.record_kind != StateRecordKind::Task
        || task_ref.project_id.as_str() != project.project_id
    {
        return FinalAuthorityProjection::fallback(
            FinalAuthorityFallbackReason::StatusValidationFailed,
            fallback_coordinates,
        );
    }
    let task_id = TaskId::new(task_ref.record_id.as_str());
    let expectation = AuthorityStatusExpectation::new(ProjectId::new(&project.project_id), task_id);
    match validate_authority_status(response, &expectation) {
        Ok(validated) => {
            FinalAuthorityProjection::Receipt(Box::new(validated.authority_receipt().clone()))
        }
        Err(_) => FinalAuthorityProjection::fallback(
            FinalAuthorityFallbackReason::StatusValidationFailed,
            fallback_coordinates,
        ),
    }
}

pub(crate) fn read_final_authority_coordinates(
    runtime_home: &Path,
    project: &ProjectRecord,
) -> Result<FinalAuthorityCoordinates, ()> {
    let store =
        CoreProjectStore::open_read_only(runtime_home, &ProjectId::new(project.project_id.clone()))
            .map_err(|_| ())?;
    let state = store.project_state().map_err(|_| ())?;
    Ok(FinalAuthorityCoordinates {
        project_id: project.project_id.clone(),
        task_id: state.active_task_id,
        state_version: Some(state.state_version),
    })
}

fn verify_binding(
    runtime_home: &Path,
    project: &ProjectRecord,
    options: &FinalOutputOptions,
) -> Result<(), BindingFailure> {
    if options.host != options.host_output
        || project.status != ACTIVE_PROJECT_STATUS
        || project.repo_root != options.repo
    {
        return Err(BindingFailure::Unavailable);
    }
    let connection = agent_connection_record_read_only(runtime_home, &options.connection_id)
        .map_err(|_| BindingFailure::Unavailable)?
        .ok_or(BindingFailure::Unavailable)?;
    if !connection.enabled
        || !matches!(
            connection.mode.as_str(),
            CONNECTION_MODE_READ_ONLY | CONNECTION_MODE_WORKFLOW
        )
        || connection.host_kind != options.host.registry_value()
        || connection
            .project_internal_id
            .as_deref()
            .is_some_and(|id| id != project.project_internal_id)
    {
        return Err(BindingFailure::Unavailable);
    }
    let access = agent_connection_project_access_read_only(
        runtime_home,
        &options.connection_id,
        &project.project_id,
    )
    .map_err(|_| BindingFailure::Unavailable)?
    .ok_or(BindingFailure::Unavailable)?;
    if !access.connection_enabled || !access.project_allowed {
        return Err(BindingFailure::Unavailable);
    }
    let access_project = access.project.ok_or(BindingFailure::Unavailable)?;
    if access_project.project_internal_id != project.project_internal_id
        || access_project.repo_root != project.repo_root
    {
        return Err(BindingFailure::Unavailable);
    }

    let recorded = recorded_local_policy(&project.repo_root)
        .map_err(|_| BindingFailure::Unavailable)?
        .ok_or(BindingFailure::Unavailable)?;
    verify_recorded_policy(&recorded, project, &connection.intent, options)?;
    let policy_text = fs::read_to_string(project.repo_root.join(VOLICORD_POLICY_FILE))
        .map_err(|_| BindingFailure::Unavailable)?;
    let policy_value =
        serde_json::from_str::<Value>(&policy_text).map_err(|_| BindingFailure::Unavailable)?;
    let observed_policy_hash =
        policy_hash(&policy_value).map_err(|_| BindingFailure::Unavailable)?;
    if observed_policy_hash != options.policy_hash {
        return Err(BindingFailure::Unavailable);
    }

    let installation = guard_installation(runtime_home, &options.guard_installation_id)
        .map_err(|_| BindingFailure::AdapterUnavailable)?
        .ok_or(BindingFailure::AdapterUnavailable)?;
    if installation.connection_internal_id != options.connection_id
        || installation.project_id.as_deref() != Some(project.project_id.as_str())
        || installation.project_internal_id.as_deref() != Some(project.project_internal_id.as_str())
        || installation.host_kind != options.host.registry_value()
        || installation.guard_mode != options.profile.as_str()
        || !matches!(
            installation.installation_status.as_str(),
            "configured" | "reload_required" | "active"
        )
    {
        return Err(BindingFailure::AdapterUnavailable);
    }
    let capability = serde_json::from_str::<Value>(&installation.host_capability_json)
        .map_err(|_| BindingFailure::AdapterUnavailable)?;
    let project_git_info_exclude_path =
        git_exclude_path(&project.repo_root).map_err(|_| BindingFailure::AdapterUnavailable)?;
    if !host_hook_capability_matches_owner_binding(
        &capability,
        HostHookCapabilityOwnerBinding {
            row_host_kind: &installation.host_kind,
            row_guard_mode: &installation.guard_mode,
            row_guard_installation_id: &installation.guard_installation_id,
            connection_internal_id: &connection.connection_internal_id,
            connection_host_kind: &connection.host_kind,
            connection_intent: &connection.intent,
            project_repo_root: Some(&project.repo_root),
            project_git_info_exclude_path: project_git_info_exclude_path.as_deref(),
        },
    ) || capability.get("policy_hash").and_then(Value::as_str)
        != Some(observed_policy_hash.as_str())
        || capability.get("selected_profile").and_then(Value::as_str)
            != Some(options.profile.as_str())
        || capability
            .get("final_output_authority_disclosure_implementation_available")
            .and_then(Value::as_bool)
            != Some(true)
        || capability
            .get("native_host_output_adapter")
            .and_then(Value::as_str)
            != Some(options.host.cli_value())
        || capability
            .get("native_host_output_adapter_config_verified")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err(BindingFailure::AdapterUnavailable);
    }
    Ok(())
}

fn verify_recorded_policy(
    recorded: &RecordedLocalPolicy,
    project: &ProjectRecord,
    connection_intent: &str,
    options: &FinalOutputOptions,
) -> Result<(), BindingFailure> {
    if recorded.host != options.host.cli_value()
        || recorded.repo_root != project.repo_root
        || recorded.connection_intent.as_str() != connection_intent
        || recorded.selected_profile != options.profile
        || recorded.connection_id != options.connection_id
        || recorded.guard_installation_id != options.guard_installation_id
    {
        return Err(BindingFailure::Unavailable);
    }
    Ok(())
}

fn render_fallback(
    base_response: &Value,
    reason: FinalAuthorityFallbackReason,
    coordinates: Option<&FinalAuthorityCoordinates>,
    max_bytes: usize,
) -> Result<RenderedFinalAuthorityOutput, FinalAuthorityRenderError> {
    if coordinates.is_some_and(|coordinates| coordinates.task_id.is_none()) {
        return render_no_active_task(base_response, coordinates, Some(reason), max_bytes);
    }
    let message = match coordinates.and_then(|coordinates| {
        coordinates
            .task_id
            .as_deref()
            .map(|task_id| (coordinates, task_id))
    }) {
        Some((coordinates, task_id)) => format!(
            "Volicord authority disclosure unavailable ({}). project_id={}; task_id={}; state_version={}. Inspect current authority with `volicord status --task {} --json`.",
            reason.as_str(),
            coordinates.project_id,
            task_id,
            display_state_version(coordinates.state_version),
            task_id
        ),
        None => format!(
            "Volicord authority disclosure unavailable ({}). Current Task coordinates could not be verified. Inspect current authority with `volicord status --json`.",
            reason.as_str()
        ),
    };
    rendered_fallback(base_response, &message, Some(reason), max_bytes)
}

fn render_no_active_task(
    base_response: &Value,
    coordinates: Option<&FinalAuthorityCoordinates>,
    reason: Option<FinalAuthorityFallbackReason>,
    max_bytes: usize,
) -> Result<RenderedFinalAuthorityOutput, FinalAuthorityRenderError> {
    let reason_text = reason
        .map(|reason| format!(" unavailable ({})", reason.as_str()))
        .unwrap_or_default();
    let coordinate_text = coordinates
        .map(|coordinates| {
            format!(
                " project_id={}; state_version={}.",
                coordinates.project_id,
                display_state_version(coordinates.state_version)
            )
        })
        .unwrap_or_default();
    let message = format!(
        "Volicord authority disclosure{reason_text}: no active Task is available.{coordinate_text} Inspect current authority with `volicord status --json`."
    );
    rendered_fallback(base_response, &message, reason, max_bytes)
}

fn rendered_fallback(
    base_response: &Value,
    message: &str,
    reason: Option<FinalAuthorityFallbackReason>,
    max_bytes: usize,
) -> Result<RenderedFinalAuthorityOutput, FinalAuthorityRenderError> {
    let stdout = serialize_host_response(base_response, message)?;
    if stdout.len() > max_bytes {
        return Err(FinalAuthorityRenderError::FallbackExceedsHostUiBudget);
    }
    Ok(RenderedFinalAuthorityOutput {
        stdout,
        receipt_displayed: false,
        fallback_reason: reason,
    })
}

fn serialize_host_response(
    base_response: &Value,
    system_message: &str,
) -> Result<String, FinalAuthorityRenderError> {
    let mut response = base_response.clone();
    response
        .as_object_mut()
        .ok_or(FinalAuthorityRenderError::HostResponseIsNotObject)?
        .insert(
            "systemMessage".to_owned(),
            Value::String(system_message.to_owned()),
        );
    let mut stdout = serde_json::to_string(&response)
        .map_err(|_| FinalAuthorityRenderError::SerializationFailed)?;
    stdout.push('\n');
    Ok(stdout)
}

fn command_outcome(projection: FinalAuthorityProjection) -> FinalOutputCommandOutcome {
    let rendered = render_final_authority_response(&json!({"continue": true}), &projection);
    FinalOutputCommandOutcome {
        stdout: rendered
            .map(|rendered| rendered.stdout)
            .unwrap_or_else(|_| GENERIC_FALLBACK_WIRE.to_owned()),
        stderr: String::new(),
        exit_code: 0,
    }
}

fn display_state_version(state_version: Option<u64>) -> String {
    state_version
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn drain_event(path: Option<&Path>) -> io::Result<()> {
    match path {
        Some(path) => drain_reader(fs::File::open(path)?),
        None => drain_reader(io::stdin().lock()),
    }
}

fn drain_reader(mut reader: impl Read) -> io::Result<()> {
    io::copy(&mut reader, &mut io::sink()).map(|_| ())
}

fn parse_options(args: &[String]) -> Result<FinalOutputOptions, FinalOutputCommandError> {
    let mut event_file = None;
    let mut repo = None;
    let mut connection_id = None;
    let mut guard_installation_id = None;
    let mut host = None;
    let mut profile = None;
    let mut expected_policy_hash = None;
    let mut host_output = None;
    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        let (option, inline_value) = token
            .split_once('=')
            .map_or((token.as_str(), None), |(option, value)| {
                (option, Some(value))
            });
        let value = if inline_value.is_some() {
            inline_value
        } else if matches!(
            option,
            "--file"
                | "--repo"
                | "--connection"
                | "--guard-installation"
                | "--host"
                | "--integration-profile"
                | "--policy-hash"
                | "--host-output"
        ) {
            index += 1;
            Some(
                args.get(index)
                    .ok_or_else(|| {
                        FinalOutputCommandError::Usage(format!("{option} requires a value"))
                    })?
                    .as_str(),
            )
        } else {
            None
        };
        let value = value.ok_or_else(|| {
            FinalOutputCommandError::Usage(if option.starts_with('-') {
                format!("unknown option: {option}")
            } else {
                format!("unexpected argument: {option}")
            })
        })?;
        if value.is_empty() || value.starts_with('-') {
            return Err(FinalOutputCommandError::Usage(format!(
                "{option} requires a value"
            )));
        }
        match option {
            "--file" => set_once(&mut event_file, PathBuf::from(value), option)?,
            "--repo" => set_once(&mut repo, PathBuf::from(value), option)?,
            "--connection" => set_once(&mut connection_id, value.to_owned(), option)?,
            "--guard-installation" => {
                set_once(&mut guard_installation_id, value.to_owned(), option)?
            }
            "--host" => set_once(
                &mut host,
                ManagedFinalOutputHost::from_cli(value, "--host")?,
                option,
            )?,
            "--integration-profile" => {
                let parsed = match value {
                    "record" => IntegrationProfile::Record,
                    "detective" => IntegrationProfile::Detective,
                    _ => {
                        return Err(FinalOutputCommandError::Usage(
                            "--integration-profile must be record or detective".to_owned(),
                        ))
                    }
                };
                set_once(&mut profile, parsed, option)?;
            }
            "--policy-hash" => set_once(&mut expected_policy_hash, value.to_owned(), option)?,
            "--host-output" => set_once(
                &mut host_output,
                ManagedFinalOutputHost::from_cli(value, "--host-output")?,
                option,
            )?,
            _ => {
                return Err(FinalOutputCommandError::Usage(format!(
                    "unknown option: {option}"
                )))
            }
        }
        index += 1;
    }

    let host = required(host, "--host")?;
    let host_output = required(host_output, "--host-output")?;
    Ok(FinalOutputOptions {
        event_file,
        repo: required(repo, "--repo")?,
        connection_id: required(connection_id, "--connection")?,
        guard_installation_id: required(guard_installation_id, "--guard-installation")?,
        host,
        profile: required(profile, "--integration-profile")?,
        policy_hash: required(expected_policy_hash, "--policy-hash")?,
        host_output,
    })
}

fn set_once<T>(
    slot: &mut Option<T>,
    value: T,
    option: &str,
) -> Result<(), FinalOutputCommandError> {
    if slot.is_some() {
        return Err(FinalOutputCommandError::Usage(format!(
            "{option} was supplied more than once"
        )));
    }
    *slot = Some(value);
    Ok(())
}

fn required<T>(slot: Option<T>, option: &str) -> Result<T, FinalOutputCommandError> {
    slot.ok_or_else(|| FinalOutputCommandError::Usage(format!("{option} is required")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    use volicord_core::{CoreService, InvocationContext};
    use volicord_store::guards::{upsert_guard_installation, GuardInstallationUpsert};
    use volicord_test_support::core_fixtures::CoreFixture;
    use volicord_types::{ActorSource, OperationCategory, VERIFICATION_BASIS_TEST_FIXTURE_BINDING};

    fn fixture_policy(
        fixture: &CoreFixture,
        profile: IntegrationProfile,
        installation_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        let policy_command = |command_name: &str| {
            let (output_flag, output_format) = if profile == IntegrationProfile::Detective {
                ("--host-output", "codex")
            } else {
                ("--output", "volicord-json")
            };
            json!({
                "command": "volicord",
                "args": [
                    "_hook",
                    command_name,
                    "--repo",
                    fixture.product_repo_path().display().to_string(),
                    "--connection",
                    fixture.connection_id(),
                    "--guard-installation",
                    installation_id,
                    "--host",
                    "codex",
                    "--integration-profile",
                    profile.as_str(),
                    output_flag,
                    output_format,
                ],
            })
        };
        let commands = json!({
            "session_start": policy_command("session-start"),
            "pre_tool": policy_command("pre-tool"),
            "post_tool": policy_command("post-tool"),
            "prompt_capture": policy_command("prompt-capture"),
            "stop": policy_command("stop"),
        });
        let policy = json!({
            "schema": "volicord-policy-v1",
            "managed_by": "volicord",
            "storage_scope": "local_overlay",
            "connection_intent": "shared",
            "host": "codex",
            "repo_root": fixture.product_repo_path().display().to_string(),
            "connection_id": fixture.connection_id(),
            "guard_installation_id": installation_id,
            "selected_profile": profile.as_str(),
            "mcp": {"command": "volicord", "args": ["mcp", "--stdio"], "env": {}},
            "host_hook": {"enabled": profile == IntegrationProfile::Detective, "commands": commands}
        });
        let digest = policy_hash(&policy)?;
        let policy_dir = fixture.product_repo_path().join(".volicord");
        fs::create_dir_all(&policy_dir)?;
        fs::write(
            policy_dir.join("policy.json"),
            serde_json::to_string_pretty(&policy)?,
        )?;
        upsert_guard_installation(
            fixture.runtime_home_path(),
            GuardInstallationUpsert {
                guard_installation_id: installation_id.to_owned(),
                connection_internal_id: fixture.connection_id().to_owned(),
                project_id: Some(fixture.project_id().to_owned()),
                host_kind: "codex".to_owned(),
                guard_mode: profile.as_str().to_owned(),
                host_capability_json: json!({
                    "schema": HOST_HOOK_CAPABILITY_SCHEMA,
                    "policy_hash": digest,
                    "selected_profile": profile.as_str(),
                    "connection_intent": "shared",
                    "final_output_authority_disclosure_implementation_available": true,
                    "native_host_output_adapter": "codex",
                    "native_host_output_adapter_config_verified": true,
                    "bash_shell_mutation_coverage": false,
                    "direct_file_write_matcher_coverage": false,
                    "host_capabilities": {
                        "stdio_mcp": true,
                        "http_mcp": false,
                        "session_start_hook": true,
                        "pre_tool_hook": true,
                        "post_tool_hook": true,
                        "user_prompt_submit_hook": true,
                        "stop_hook": true,
                        "rule_file_support": true,
                        "project_local_configuration": true,
                    },
                    "required_hook_phases": [],
                    "missing_required_hooks": [],
                    "prompt_capture": false,
                    "files": [],
                    "host_hook_commands": [],
                    "hook_root_resolution": null,
                    "hook_path_safety": null,
                    "commands": commands,
                })
                .to_string(),
                installation_status: "configured".to_owned(),
                installed_at: (profile == IntegrationProfile::Detective)
                    .then(|| "2026-07-13T00:00:00Z".to_owned()),
                last_checked_at: "2026-07-13T00:00:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: "{}".to_owned(),
            },
        )?;
        Ok(digest)
    }

    fn command_args(
        fixture: &CoreFixture,
        event_file: &Path,
        profile: IntegrationProfile,
        installation_id: &str,
        digest: &str,
    ) -> Vec<String> {
        vec![
            "--file".to_owned(),
            event_file.display().to_string(),
            "--repo".to_owned(),
            fixture.product_repo_path().display().to_string(),
            "--connection".to_owned(),
            fixture.connection_id().to_owned(),
            "--guard-installation".to_owned(),
            installation_id.to_owned(),
            "--host".to_owned(),
            "codex".to_owned(),
            "--integration-profile".to_owned(),
            profile.as_str().to_owned(),
            "--policy-hash".to_owned(),
            digest.to_owned(),
            "--host-output".to_owned(),
            "codex".to_owned(),
        ]
    }

    fn downgrade_fixture_capability_to_v1(
        fixture: &CoreFixture,
        installation_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        mutate_fixture_capability(fixture, installation_id, |object| {
            object.insert(
                "schema".to_owned(),
                Value::String("volicord-host-hook-capability-v1".to_owned()),
            );
            object.remove("final_output_authority_disclosure_implementation_available");
            object.insert(
                "final_output_authority_disclosure_supported".to_owned(),
                Value::Bool(true),
            );
        })
    }

    fn mutate_fixture_capability(
        fixture: &CoreFixture,
        installation_id: &str,
        mutate: impl FnOnce(&mut serde_json::Map<String, Value>),
    ) -> Result<(), Box<dyn Error>> {
        let installation = guard_installation(fixture.runtime_home_path(), installation_id)?
            .expect("fixture guard installation");
        let mut capability: Value = serde_json::from_str(&installation.host_capability_json)?;
        let object = capability
            .as_object_mut()
            .expect("fixture capability should be an object");
        mutate(object);
        let registry = rusqlite::Connection::open(volicord_store::sqlite::registry_db_path(
            fixture.runtime_home_path(),
        ))?;
        let updated = registry.execute(
            "UPDATE guard_installations
                SET host_capability_json = ?1
              WHERE guard_installation_id = ?2",
            rusqlite::params![capability.to_string(), installation_id],
        )?;
        assert_eq!(updated, 1, "fixture capability row should exist");
        Ok(())
    }

    fn create_active_task(fixture: &CoreFixture) -> Result<String, Box<dyn Error>> {
        let response = CoreService::new(fixture.runtime_home_path()).intake(
            fixture.intake_request(
                "req_final_output_intake",
                "idem_final_output_intake",
                false,
                Some(0),
            ),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        Ok(response.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("intake task ref")
            .to_owned())
    }

    #[test]
    fn final_output_reads_fresh_receipt_without_observation_effects() -> Result<(), Box<dyn Error>>
    {
        let fixture = CoreFixture::new("final-output-receipt")?;
        let task_id = create_active_task(&fixture)?;
        let installation_id = "guard_final_output_record";
        let digest = fixture_policy(&fixture, IntegrationProfile::Record, installation_id)?;
        let event_file = fixture.runtime_home_path().join("final-event.json");
        let private_marker = "model-private-final-prose";
        fs::write(
            &event_file,
            format!("{{\"transcript\":\"{private_marker}\"}}"),
        )?;
        let before_counts = fixture.counts()?;
        let before_observations: (u64, u64) = fixture.conn()?.query_row(
            "SELECT (SELECT COUNT(*) FROM guard_events), (SELECT COUNT(*) FROM agent_sessions)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let outcome = run_final_output_command(
            &command_args(
                &fixture,
                &event_file,
                IntegrationProfile::Record,
                installation_id,
                &digest,
            ),
            |name| {
                (name == "VOLICORD_HOME")
                    .then(|| fixture.runtime_home_path().as_os_str().to_owned())
            },
            &fixture.product_repo_path(),
        )?;

        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.stderr.is_empty());
        assert!(outcome.stdout.ends_with('\n'));
        assert!(outcome.stdout.len() <= MAX_FINAL_AUTHORITY_HOST_RESPONSE_BYTES);
        assert!(!outcome.stdout.contains(private_marker));
        let response: Value = serde_json::from_str(&outcome.stdout)?;
        assert_eq!(response["continue"], true);
        let message = response["systemMessage"].as_str().expect("systemMessage");
        let receipt: AuthorityReceipt = serde_json::from_str(
            message
                .strip_prefix(AUTHORITY_RECEIPT_PREFIX)
                .expect("canonical receipt prefix"),
        )?;
        assert_eq!(receipt.task_ref.record_id.as_str(), task_id);
        assert_eq!(fixture.counts()?, before_counts);
        let after_observations: (u64, u64) = fixture.conn()?.query_row(
            "SELECT (SELECT COUNT(*) FROM guard_events), (SELECT COUNT(*) FROM agent_sessions)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(after_observations, before_observations);
        Ok(())
    }

    #[test]
    fn final_output_no_active_task_uses_exact_status_fallback() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("final-output-no-task")?;
        let installation_id = "guard_final_output_no_task";
        let digest = fixture_policy(&fixture, IntegrationProfile::Record, installation_id)?;
        let event_file = fixture.runtime_home_path().join("final-event.json");
        fs::write(&event_file, "{}")?;

        let outcome = run_final_output_command(
            &command_args(
                &fixture,
                &event_file,
                IntegrationProfile::Record,
                installation_id,
                &digest,
            ),
            |name| {
                (name == "VOLICORD_HOME")
                    .then(|| fixture.runtime_home_path().as_os_str().to_owned())
            },
            &fixture.product_repo_path(),
        )?;
        let response: Value = serde_json::from_str(&outcome.stdout)?;
        let message = response["systemMessage"].as_str().expect("systemMessage");
        assert!(message.contains("no active Task is available"));
        assert!(message.contains("`volicord status --json`"));
        assert!(!message.contains("volicord status --task"));
        Ok(())
    }

    #[test]
    fn final_output_rejects_v1_capability_instead_of_inferring_adapter_support(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("final-output-v1-capability")?;
        let task_id = create_active_task(&fixture)?;
        let installation_id = "guard_final_output_v1_capability";
        let digest = fixture_policy(&fixture, IntegrationProfile::Record, installation_id)?;
        downgrade_fixture_capability_to_v1(&fixture, installation_id)?;
        let event_file = fixture.runtime_home_path().join("final-event.json");
        fs::write(&event_file, "{}")?;

        let outcome = run_final_output_command(
            &command_args(
                &fixture,
                &event_file,
                IntegrationProfile::Record,
                installation_id,
                &digest,
            ),
            |name| {
                (name == "VOLICORD_HOME")
                    .then(|| fixture.runtime_home_path().as_os_str().to_owned())
            },
            &fixture.product_repo_path(),
        )?;

        assert_eq!(outcome.exit_code, 0);
        let response: Value = serde_json::from_str(&outcome.stdout)?;
        let message = response["systemMessage"].as_str().expect("systemMessage");
        assert!(message.contains("adapter_unavailable"));
        assert!(message.contains(&format!("`volicord status --task {task_id} --json`")));
        assert!(!message.contains(AUTHORITY_RECEIPT_PREFIX));
        Ok(())
    }

    #[test]
    fn final_output_rejects_v2_capability_with_retired_boolean() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("final-output-mixed-v2-capability")?;
        let task_id = create_active_task(&fixture)?;
        let installation_id = "guard_final_output_mixed_v2_capability";
        let digest = fixture_policy(&fixture, IntegrationProfile::Record, installation_id)?;
        mutate_fixture_capability(&fixture, installation_id, |object| {
            object.insert(
                "final_output_authority_disclosure_supported".to_owned(),
                Value::Bool(true),
            );
        })?;
        let event_file = fixture.runtime_home_path().join("final-event.json");
        fs::write(&event_file, "{}")?;

        let outcome = run_final_output_command(
            &command_args(
                &fixture,
                &event_file,
                IntegrationProfile::Record,
                installation_id,
                &digest,
            ),
            |name| {
                (name == "VOLICORD_HOME")
                    .then(|| fixture.runtime_home_path().as_os_str().to_owned())
            },
            &fixture.product_repo_path(),
        )?;

        let response: Value = serde_json::from_str(&outcome.stdout)?;
        let message = response["systemMessage"].as_str().expect("systemMessage");
        assert!(message.contains("adapter_unavailable"));
        assert!(message.contains(&format!("`volicord status --task {task_id} --json`")));
        assert!(!message.contains(AUTHORITY_RECEIPT_PREFIX));
        Ok(())
    }

    #[test]
    fn final_output_rejects_exact_capability_with_mismatched_owner_intent(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("final-output-binding-intent")?;
        let task_id = create_active_task(&fixture)?;
        let installation_id = "guard_final_output_binding_intent";
        let digest = fixture_policy(&fixture, IntegrationProfile::Record, installation_id)?;
        mutate_fixture_capability(&fixture, installation_id, |object| {
            object.insert(
                "connection_intent".to_owned(),
                Value::String("personal".to_owned()),
            );
        })?;
        let event_file = fixture.runtime_home_path().join("final-event.json");
        fs::write(&event_file, "{}")?;

        let outcome = run_final_output_command(
            &command_args(
                &fixture,
                &event_file,
                IntegrationProfile::Record,
                installation_id,
                &digest,
            ),
            |name| {
                (name == "VOLICORD_HOME")
                    .then(|| fixture.runtime_home_path().as_os_str().to_owned())
            },
            &fixture.product_repo_path(),
        )?;

        let response: Value = serde_json::from_str(&outcome.stdout)?;
        let message = response["systemMessage"].as_str().expect("systemMessage");
        assert!(message.contains("adapter_unavailable"));
        assert!(message.contains(&format!("`volicord status --task {task_id} --json`")));
        assert!(!message.contains(AUTHORITY_RECEIPT_PREFIX));
        Ok(())
    }

    #[test]
    fn final_output_adapter_mismatch_is_a_successful_bounded_fallback() -> Result<(), Box<dyn Error>>
    {
        let fixture = CoreFixture::new("final-output-adapter-mismatch")?;
        let task_id = create_active_task(&fixture)?;
        let installation_id = "guard_final_output_adapter_mismatch";
        let digest = fixture_policy(&fixture, IntegrationProfile::Record, installation_id)?;
        let event_file = fixture.runtime_home_path().join("final-event.json");
        fs::write(&event_file, "{}")?;
        let mut args = command_args(
            &fixture,
            &event_file,
            IntegrationProfile::Record,
            installation_id,
            &digest,
        );
        *args.last_mut().expect("host-output value") = "claude-code".to_owned();

        let outcome = run_final_output_command(
            &args,
            |name| {
                (name == "VOLICORD_HOME")
                    .then(|| fixture.runtime_home_path().as_os_str().to_owned())
            },
            &fixture.product_repo_path(),
        )?;

        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.stderr.is_empty());
        assert!(outcome.stdout.len() <= MAX_FINAL_AUTHORITY_HOST_RESPONSE_BYTES);
        let response: Value = serde_json::from_str(&outcome.stdout)?;
        let message = response["systemMessage"].as_str().expect("systemMessage");
        assert!(message.contains("binding_unavailable"));
        assert!(message.contains(&format!("`volicord status --task {task_id} --json`")));
        assert!(!message.contains(AUTHORITY_RECEIPT_PREFIX));
        Ok(())
    }

    #[test]
    fn renderer_accepts_8192_bytes_and_falls_back_at_8193() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("final-output-budget")?;
        create_active_task(&fixture)?;
        let installation_id = "guard_final_output_budget";
        let digest = fixture_policy(&fixture, IntegrationProfile::Record, installation_id)?;
        let project = project_record_by_repo_root_read_only(
            fixture.runtime_home_path(),
            fixture.product_repo_path(),
        )?
        .expect("registered project");
        let binding = verify_managed_final_output_binding(
            fixture.runtime_home_path(),
            &fixture.product_repo_path(),
            fixture.connection_id(),
            installation_id,
            ManagedFinalOutputHost::Codex,
            IntegrationProfile::Record,
            &digest,
            ManagedFinalOutputHost::Codex,
        )
        .expect("budget renderer fixture should have a verified managed binding");
        let projection = refresh_final_authority_projection(
            fixture.runtime_home_path(),
            binding,
            read_final_authority_coordinates(fixture.runtime_home_path(), &project).ok(),
        );
        assert!(matches!(projection, FinalAuthorityProjection::Receipt(_)));

        let mut padding = String::new();
        loop {
            let base = json!({"continue": true, "padding": padding});
            let rendered =
                render_final_authority_response_with_limit(&base, &projection, usize::MAX)?;
            match rendered
                .stdout
                .len()
                .cmp(&MAX_FINAL_AUTHORITY_HOST_RESPONSE_BYTES)
            {
                std::cmp::Ordering::Less => padding.push('x'),
                std::cmp::Ordering::Equal => break,
                std::cmp::Ordering::Greater => panic!("ASCII padding skipped the exact budget"),
            }
        }
        let exact_base = json!({"continue": true, "padding": padding});
        let exact = render_final_authority_response(&exact_base, &projection)?;
        assert_eq!(exact.stdout.len(), MAX_FINAL_AUTHORITY_HOST_RESPONSE_BYTES);
        assert!(exact.receipt_displayed);

        let one_over_base = json!({
            "continue": true,
            "padding": format!("{}x", exact_base["padding"].as_str().expect("padding"))
        });
        let one_over_unbounded =
            render_final_authority_response_with_limit(&one_over_base, &projection, usize::MAX)?;
        assert_eq!(
            one_over_unbounded.stdout.len(),
            MAX_FINAL_AUTHORITY_HOST_RESPONSE_BYTES + 1
        );
        let fallback = render_final_authority_response(&one_over_base, &projection)?;
        assert!(!fallback.receipt_displayed);
        assert_eq!(
            fallback.fallback_reason,
            Some(FinalAuthorityFallbackReason::ReceiptExceedsHostUiBudget)
        );
        assert!(fallback.stdout.len() <= MAX_FINAL_AUTHORITY_HOST_RESPONSE_BYTES);
        assert!(!fallback.stdout.contains(AUTHORITY_RECEIPT_PREFIX));
        Ok(())
    }

    #[test]
    fn parser_requires_supported_host_profile_and_complete_binding_options() {
        let error = parse_options(&["--host".to_owned(), "codex".to_owned()])
            .expect_err("incomplete managed binding must fail");
        assert!(error.to_string().contains("--host-output is required"));

        let args = vec![
            "--repo",
            "/repo",
            "--connection",
            "connection",
            "--guard-installation",
            "guard",
            "--host",
            "codex",
            "--integration-profile",
            "record",
            "--policy-hash",
            "sha256:x",
            "--host-output",
            "claude-code",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        let options = parse_options(&args)
            .expect("valid but mismatched host adapters must reach the safe binding fallback");
        assert_eq!(options.host, ManagedFinalOutputHost::Codex);
        assert_eq!(options.host_output, ManagedFinalOutputHost::ClaudeCode);
    }
}
