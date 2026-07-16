use crate::adapter::*;
use crate::errors::{bound_mcp_tool_error_issue, McpAdapterError};
use crate::local_http::generate_bearer_token;
use crate::local_web_consent::start_stdio_local_web_consent_listener;
use crate::prelude::*;
use crate::repository_discovery::RepositoryDiscoveryHost;
use crate::routing::*;
use crate::util::*;
use sha2::{Digest, Sha256};
use volicord_types::{ManagedMcpClientInfo, REVIEWED_CODEX_MCP_CLIENT_NAME};

const VOLICORD_MCP_VERIFICATION: &str = "VOLICORD_MCP_VERIFICATION";
const VOLICORD_MCP_LAUNCH: &str = "VOLICORD_MCP_LAUNCH";
const VOLICORD_MCP_HOST: &str = "VOLICORD_MCP_HOST";
const VOLICORD_MCP_CONNECTION_ID: &str = "VOLICORD_MCP_CONNECTION_ID";
const VOLICORD_MCP_PROJECT_ID: &str = "VOLICORD_MCP_PROJECT_ID";
const MANAGED_HOST_LAUNCH_VALUE: &str = "managed_host";
const CODEX_HOST_VALUE: &str = "codex";
const CLAUDE_CODE_HOST_VALUE: &str = "claude_code";
const CLAUDECODE: &str = "CLAUDECODE";
const CLAUDE_CODE_SESSION_ID: &str = "CLAUDE_CODE_SESSION_ID";
const CODEX_TURN_METADATA_KEY: &str = "x-codex-turn-metadata";
const CODEX_THREAD_BINDING_DOMAIN: &[u8] = b"volicord-codex-mcp-thread-binding-v1\0";
pub(crate) const MAX_MCP_COMPACT_MUTATION_RESULT_BYTES: usize = 65_536;
pub(crate) const MAX_MCP_FULL_MUTATION_RESULT_BYTES: usize = 256 * 1024;
pub(crate) const MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES: usize = 512;
pub(crate) const MAX_MCP_ELICITATION_WIRE_BYTES: usize = USER_ACTION_FORM_MAX_BYTES;

pub fn run_stdio<R, W>(adapter: McpAdapter, reader: R, writer: W) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    run_stdio_with_options(adapter, reader, writer, StdioRunOptions::default())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StdioRunOptions {
    startup_session_watch: bool,
    launch_origin: McpLaunchOrigin,
    managed_host_session_id: Option<String>,
}

impl Default for StdioRunOptions {
    fn default() -> Self {
        Self {
            startup_session_watch: false,
            launch_origin: McpLaunchOrigin::ManualCli,
            managed_host_session_id: None,
        }
    }
}

fn run_stdio_with_options<R, W>(
    adapter: McpAdapter,
    reader: R,
    mut writer: W,
    options: StdioRunOptions,
) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    reject_invalid_managed_marker(options.launch_origin)?;
    let mut state =
        ConnectionState::for_launch_origin(options.launch_origin, options.managed_host_session_id);
    validate_managed_stdio_session_ownership(&adapter, &state)?;
    if !state.codex_binding.is_pending() && !state.managed_host_lifecycle_observations {
        let _ = start_transport_diagnostic_session(&adapter, &state);
    }
    let _startup_observation =
        if options.startup_session_watch && options.launch_origin != McpLaunchOrigin::ManagedHost {
            adapter.startup_session_watch_observation_best_effort_with_origin(
                &state.session_id,
                options.launch_origin.as_str(),
            )
        } else {
            StartupObservationResult::SkippedVerificationProbe
        };
    let mut lines = reader.lines();

    while let Some(line) = lines.next() {
        let line = line.map_err(McpAdapterError::Io)?;
        if line.trim().is_empty() {
            continue;
        }

        let message: Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(error) => {
                write_json_line(
                    &mut writer,
                    json_rpc_error(Value::Null, -32700, "Parse error", Some(error.to_string())),
                )?;
                continue;
            }
        };

        if let Some(response) =
            handle_json_rpc_message(&adapter, &mut state, message, &mut lines, &mut writer)?
        {
            write_json_line(&mut writer, response)?;
        }
    }

    writer.flush().map_err(McpAdapterError::Io)
}

/// Runs the MCP stdio adapter from process environment and stdin/stdout.
pub fn run_stdio_from_env(
    connection_id: &str,
    project_id: Option<&str>,
) -> Result<(), McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    let runtime_home = resolve_runtime_home(process_env_var, &current_dir)?;
    let project_allowlist = project_id
        .map(ProjectId::new)
        .into_iter()
        .collect::<Vec<_>>();
    validate_mcp_project_allowlist(&runtime_home, connection_id, &project_allowlist)?;
    let context = McpConnectionContext::resolve(&runtime_home, connection_id)?
        .with_project_allowlist(project_allowlist);
    let connection = agent_connection_record_read_only(&runtime_home, connection_id)
        .map_err(McpAdapterError::Store)?
        .ok_or_else(|| {
            McpAdapterError::Environment(format!(
                "MCP Agent Connection disappeared during startup: {connection_id}"
            ))
        })?;
    let launch_origin = classify_launch_origin(
        process_env_var,
        connection_id,
        project_id,
        Some(&connection.host_kind),
    );
    let managed_host_session_id = managed_host_session_id_from_env(
        &process_env_var,
        launch_origin,
        &connection.host_kind,
        connection_id,
    );
    let startup_session_watch = launch_origin == McpLaunchOrigin::ManagedHost;
    let local_web_consent =
        start_optional_stdio_protocol_after_marker_validation(launch_origin, || {
            start_stdio_local_web_consent_listener(&runtime_home, &context)
        })?;
    let mut adapter = McpAdapter::new(runtime_home, context);
    if let Some(local_web_consent) = local_web_consent {
        adapter = adapter.with_local_web_consent_readiness(
            local_web_consent.context,
            local_web_consent.readiness,
        );
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio_with_options(
        adapter,
        stdin.lock(),
        stdout.lock(),
        StdioRunOptions {
            startup_session_watch,
            launch_origin,
            managed_host_session_id,
        },
    )
}

/// Runs stdio from a clone-portable repository descriptor.
///
/// The descriptor carries only the host selector. Connection and project
/// identities are resolved from the current Git repository and the selected
/// local Runtime Home before the transport starts.
pub fn run_stdio_discover_repository_from_env(
    host: RepositoryDiscoveryHost,
) -> Result<(), McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    let runtime_home = resolve_repository_discovery_runtime_home(process_env_var, &current_dir)?;
    let resolution = RepositoryDiscoveryResolution::resolve(&runtime_home, &current_dir, host)?;
    let launch_origin = classify_launch_origin_with_descriptor(
        process_env_var,
        resolution.context.connection_internal_id.as_str(),
        Some(resolution.project_id.as_str()),
        Some(resolution.host.registry_host_kind()),
        true,
    );
    let managed_host_session_id = managed_host_session_id_from_env(
        &process_env_var,
        launch_origin,
        resolution.host.registry_host_kind(),
        resolution.context.connection_internal_id.as_str(),
    );
    let local_web_consent =
        start_optional_stdio_protocol_after_marker_validation(launch_origin, || {
            start_stdio_local_web_consent_listener(&runtime_home, &resolution.context)
        })?;
    let mut adapter = McpAdapter::new(runtime_home, resolution.context);
    if let Some(local_web_consent) = local_web_consent {
        adapter = adapter.with_local_web_consent_readiness(
            local_web_consent.context,
            local_web_consent.readiness,
        );
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio_with_options(
        adapter,
        stdin.lock(),
        stdout.lock(),
        StdioRunOptions {
            startup_session_watch: launch_origin == McpLaunchOrigin::ManagedHost,
            launch_origin,
            managed_host_session_id,
        },
    )
}

fn reject_invalid_managed_marker(launch_origin: McpLaunchOrigin) -> Result<(), McpAdapterError> {
    if launch_origin == McpLaunchOrigin::InvalidManagedMarker {
        return Err(McpAdapterError::Environment(
            "INVALID_MANAGED_MARKER: managed stdio launch markers are incomplete, invalid, or inconsistent"
                .to_owned(),
        ));
    }
    Ok(())
}

fn start_optional_stdio_protocol_after_marker_validation<T, F>(
    launch_origin: McpLaunchOrigin,
    start: F,
) -> Result<Option<T>, McpAdapterError>
where
    F: FnOnce() -> Result<T, McpAdapterError>,
{
    reject_invalid_managed_marker(launch_origin)?;
    Ok(start().ok())
}

#[cfg(test)]
mod managed_marker_protocol_order_tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn repository_descriptor_is_codex_launch_provenance_but_not_session_identity() {
        assert_eq!(
            classify_launch_origin_with_descriptor(
                |_| None,
                "connection.alpha",
                Some("project.alpha"),
                Some(CODEX_HOST_VALUE),
                true,
            ),
            McpLaunchOrigin::ManagedHost
        );
        assert!(managed_host_session_id_from_env(
            &|_| None,
            McpLaunchOrigin::ManagedHost,
            CODEX_HOST_VALUE,
            "connection.alpha",
        )
        .is_none());

        assert_eq!(
            classify_launch_origin_with_descriptor(
                |name| {
                    (name == "CODEX_THREAD_ID").then(|| OsString::from("ambient-not-a-binding"))
                },
                "connection.alpha",
                Some("project.alpha"),
                Some(CODEX_HOST_VALUE),
                true,
            ),
            McpLaunchOrigin::ManagedHost,
            "an ambient CODEX_THREAD_ID must not become a second provenance or binding input"
        );
    }

    #[test]
    fn repository_descriptor_does_not_invent_claude_or_cross_host_provenance() {
        assert_eq!(
            classify_launch_origin_with_descriptor(
                |_| None,
                "connection.alpha",
                Some("project.alpha"),
                Some(CLAUDE_CODE_HOST_VALUE),
                true,
            ),
            McpLaunchOrigin::ManualCli,
            "a descriptor without Claude's native markers remains a manual launch"
        );
        assert_eq!(
            classify_launch_origin_with_descriptor(
                |name| match name {
                    CLAUDECODE => Some(OsString::from("1")),
                    CLAUDE_CODE_SESSION_ID => Some(OsString::from("claude.session.alpha")),
                    _ => None,
                },
                "connection.alpha",
                Some("project.alpha"),
                Some(CODEX_HOST_VALUE),
                true,
            ),
            McpLaunchOrigin::InvalidManagedMarker
        );
    }

    #[test]
    fn invalid_managed_marker_is_rejected_before_optional_protocol_start() {
        let protocol_started = Cell::new(false);

        let error = start_optional_stdio_protocol_after_marker_validation(
            McpLaunchOrigin::InvalidManagedMarker,
            || {
                protocol_started.set(true);
                Ok(())
            },
        )
        .expect_err("an invalid managed marker must stop before protocol initialization");

        assert!(!protocol_started.get());
        assert!(error.to_string().contains("INVALID_MANAGED_MARKER"));
    }
}

fn resolve_repository_discovery_runtime_home<F>(
    env_var: F,
    current_dir: &Path,
) -> Result<PathBuf, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = env_var("VOLICORD_HOME").ok_or_else(|| {
        McpAdapterError::Environment(
            "repository discovery MCP startup requires VOLICORD_HOME; refusing to substitute the platform default Runtime Home"
            .to_owned(),
        )
    })?;
    if runtime_home.is_empty() {
        return Err(RuntimeHomeResolutionError::EmptyVolicordHome.into());
    }
    if !Path::new(&runtime_home).is_absolute() {
        return Err(McpAdapterError::Environment(
            "repository discovery MCP startup requires an absolute VOLICORD_HOME; refusing current-directory-relative Runtime Home selection"
                .to_owned(),
        ));
    }
    resolve_runtime_home(
        |name| (name == "VOLICORD_HOME").then(|| runtime_home.clone()),
        current_dir,
    )
}

#[cfg(test)]
pub(crate) fn run_stdio_with_env_marker<R, W, F>(
    adapter: McpAdapter,
    reader: R,
    writer: W,
    env_var: F,
) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
    F: Fn(&str) -> Option<OsString>,
{
    let launch_origin = classify_launch_origin_for_adapter(&adapter, &env_var);
    let host_kind = agent_connection_record_read_only(
        &adapter.runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .ok()
    .flatten()
    .map(|connection| connection.host_kind);
    let managed_host_session_id = host_kind.as_deref().and_then(|host_kind| {
        managed_host_session_id_from_env(
            &env_var,
            launch_origin,
            host_kind,
            adapter.context.connection_internal_id.as_str(),
        )
    });
    run_stdio_with_options(
        adapter,
        reader,
        writer,
        StdioRunOptions {
            startup_session_watch: launch_origin == McpLaunchOrigin::ManagedHost,
            launch_origin,
            managed_host_session_id,
        },
    )
}

/// Runs MCP startup validation from process environment.
pub fn run_preflight_check_from_env(
    connection_id: &str,
    project_id: Option<&str>,
) -> Result<String, McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    preflight_check(process_env_var, &current_dir, connection_id, project_id)
}

/// Runs MCP startup validation from injected process inputs.
pub fn preflight_check<F>(
    env_var: F,
    current_dir: &Path,
    connection_id: &str,
    project_id: Option<&str>,
) -> Result<String, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let detail_project_id = project_id.map(ProjectId::new);
    let inspection =
        McpConnectionStartupInspection::resolve(&runtime_home, connection_id, detail_project_id)?;
    Ok(inspection.preflight_report())
}

/// Resolves the Runtime Home used by the stdio entry point.
pub fn resolve_runtime_home_from_env<F>(env_var: F) -> Result<PathBuf, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    resolve_runtime_home(env_var, &current_dir)
}

/// Resolves the Runtime Home from injected process inputs.
pub fn resolve_runtime_home<F>(env_var: F, current_dir: &Path) -> Result<PathBuf, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    resolve_shared_runtime_home(env_var, current_dir).map_err(McpAdapterError::from)
}

fn mcp_verification_launch<F>(env_var: F) -> bool
where
    F: Fn(&str) -> Option<OsString>,
{
    env_var(VOLICORD_MCP_VERIFICATION).is_some_and(|value| value.to_str() == Some("1"))
}

#[cfg(test)]
mod repository_discovery_runtime_home_tests {
    use super::*;

    #[test]
    fn missing_runtime_home_fails_before_default_home_lookup() {
        let error = resolve_repository_discovery_runtime_home(
            |name| {
                if name == "VOLICORD_HOME" {
                    None
                } else {
                    panic!("repository discovery must not inspect default-home variable {name}")
                }
            },
            Path::new("/repo"),
        )
        .expect_err("missing VOLICORD_HOME must fail closed");

        assert!(matches!(error, McpAdapterError::Environment(_)));
        assert!(error.to_string().contains("requires VOLICORD_HOME"));
        assert!(error.to_string().contains("refusing to substitute"));
    }

    #[test]
    fn empty_runtime_home_fails_before_repository_discovery() {
        let error = resolve_repository_discovery_runtime_home(
            |name| (name == "VOLICORD_HOME").then(OsString::new),
            Path::new("/repo"),
        )
        .expect_err("empty VOLICORD_HOME must fail closed");

        assert!(matches!(error, McpAdapterError::Environment(_)));
        assert!(error
            .to_string()
            .contains("VOLICORD_HOME must not be empty"));
    }

    #[test]
    fn relative_runtime_home_fails_before_repository_discovery() {
        let error = resolve_repository_discovery_runtime_home(
            |name| (name == "VOLICORD_HOME").then(|| OsString::from("runtime")),
            Path::new("/repo"),
        )
        .expect_err("relative VOLICORD_HOME must fail closed");

        assert!(matches!(error, McpAdapterError::Environment(_)));
        assert!(error.to_string().contains("absolute VOLICORD_HOME"));
        assert!(error.to_string().contains("current-directory-relative"));
    }

    #[test]
    fn explicit_absolute_runtime_home_is_used_as_supplied() {
        let absolute = std::env::current_dir()
            .expect("current directory")
            .join("runtime-home");
        let runtime_home = resolve_repository_discovery_runtime_home(
            |name| (name == "VOLICORD_HOME").then(|| absolute.clone().into_os_string()),
            Path::new("ignored"),
        )
        .expect("explicit absolute repository discovery Runtime Home");

        assert_eq!(runtime_home, absolute);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpLaunchOrigin {
    CliVerification,
    ManagedHost,
    ManualCli,
    InvalidManagedMarker,
    Unknown,
}

impl McpLaunchOrigin {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::CliVerification => "cli_verification",
            Self::ManagedHost => "managed_host",
            Self::ManualCli => "manual_cli",
            Self::InvalidManagedMarker => "invalid_managed_marker",
            Self::Unknown => "unknown",
        }
    }
}

pub(crate) fn classify_launch_origin<F>(
    env_var: F,
    connection_id: &str,
    project_id: Option<&str>,
    expected_host_kind: Option<&str>,
) -> McpLaunchOrigin
where
    F: Fn(&str) -> Option<OsString>,
{
    classify_launch_origin_with_descriptor(
        env_var,
        connection_id,
        project_id,
        expected_host_kind,
        false,
    )
}

fn classify_launch_origin_with_descriptor<F>(
    env_var: F,
    connection_id: &str,
    project_id: Option<&str>,
    expected_host_kind: Option<&str>,
    repository_discovery_descriptor: bool,
) -> McpLaunchOrigin
where
    F: Fn(&str) -> Option<OsString>,
{
    if mcp_verification_launch(&env_var) {
        return McpLaunchOrigin::CliVerification;
    }

    let launch = env_text(&env_var, VOLICORD_MCP_LAUNCH);
    let host = env_text(&env_var, VOLICORD_MCP_HOST);
    let marker_connection_id = env_text(&env_var, VOLICORD_MCP_CONNECTION_ID);
    let marker_project_id = env_text(&env_var, VOLICORD_MCP_PROJECT_ID);
    let volicord_marker_present = [
        VOLICORD_MCP_LAUNCH,
        VOLICORD_MCP_HOST,
        VOLICORD_MCP_CONNECTION_ID,
        VOLICORD_MCP_PROJECT_ID,
    ]
    .into_iter()
    .any(|name| env_var(name).is_some());
    let native_marker_present = host_native_marker_present(&env_var);
    if !volicord_marker_present && !native_marker_present {
        return if repository_discovery_descriptor && expected_host_kind == Some(CODEX_HOST_VALUE) {
            McpLaunchOrigin::ManagedHost
        } else {
            McpLaunchOrigin::ManualCli
        };
    }

    let Some(expected_host_kind) = expected_host_kind.or(host.as_deref()) else {
        return McpLaunchOrigin::InvalidManagedMarker;
    };
    let native_marker_matches = host_native_marker_matches(&env_var, expected_host_kind);
    if !volicord_marker_present {
        return if repository_discovery_descriptor && native_marker_matches {
            McpLaunchOrigin::ManagedHost
        } else if repository_discovery_descriptor {
            McpLaunchOrigin::InvalidManagedMarker
        } else {
            McpLaunchOrigin::ManualCli
        };
    }

    let project_matches = match project_id {
        Some(project_id) => marker_project_id.as_deref() == Some(project_id),
        None => marker_project_id.is_none(),
    };
    if launch.as_deref() == Some(MANAGED_HOST_LAUNCH_VALUE)
        && host.as_deref() == Some(expected_host_kind)
        && marker_connection_id.as_deref() == Some(connection_id)
        && project_matches
        && native_marker_matches
    {
        McpLaunchOrigin::ManagedHost
    } else {
        McpLaunchOrigin::InvalidManagedMarker
    }
}

#[cfg(test)]
fn classify_launch_origin_for_adapter<F>(adapter: &McpAdapter, env_var: &F) -> McpLaunchOrigin
where
    F: Fn(&str) -> Option<OsString>,
{
    let project_id = adapter
        .context
        .project_allowlist
        .as_ref()
        .and_then(|project_ids| project_ids.as_slice().first())
        .map(|project_id| project_id.as_str());
    let host_kind = agent_connection_record_read_only(
        &adapter.runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .ok()
    .flatten()
    .map(|connection| connection.host_kind);
    classify_launch_origin(
        env_var,
        adapter.context.connection_internal_id.as_str(),
        project_id,
        host_kind.as_deref(),
    )
}

fn host_native_marker_present<F>(env_var: &F) -> bool
where
    F: Fn(&str) -> Option<OsString>,
{
    [CLAUDECODE, CLAUDE_CODE_SESSION_ID]
        .into_iter()
        .any(|name| env_var(name).is_some())
}

fn host_native_marker_matches<F>(env_var: &F, host_kind: &str) -> bool
where
    F: Fn(&str) -> Option<OsString>,
{
    match host_kind {
        CODEX_HOST_VALUE => {
            env_var(CLAUDECODE).is_none() && env_var(CLAUDE_CODE_SESSION_ID).is_none()
        }
        CLAUDE_CODE_HOST_VALUE => {
            env_text(env_var, CLAUDECODE).as_deref() == Some("1")
                && env_text(env_var, CLAUDE_CODE_SESSION_ID)
                    .is_some_and(|value| validate_managed_host_native_session_id(&value).is_ok())
        }
        _ => false,
    }
}

fn managed_host_session_id_from_env<F>(
    env_var: &F,
    launch_origin: McpLaunchOrigin,
    host_kind: &str,
    connection_internal_id: &str,
) -> Option<String>
where
    F: Fn(&str) -> Option<OsString>,
{
    if launch_origin != McpLaunchOrigin::ManagedHost {
        return None;
    }
    let native_session_id = match host_kind {
        CLAUDE_CODE_HOST_VALUE if env_text(env_var, CLAUDECODE).as_deref() == Some("1") => {
            env_text(env_var, CLAUDE_CODE_SESSION_ID)
        }
        _ => None,
    }?;
    validate_managed_host_native_session_id(&native_session_id).ok()?;
    managed_host_session_id(host_kind, connection_internal_id, &native_session_id).ok()
}

fn env_text<F>(env_var: &F, name: &str) -> Option<String>
where
    F: Fn(&str) -> Option<OsString>,
{
    env_var(name).and_then(|value| value.into_string().ok())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionPhase {
    AwaitingInitialize,
    AwaitingInitialized,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CodexManagedBinding {
    NotApplicable,
    Pending,
    Bound { thread_digest: [u8; 32] },
}

impl CodexManagedBinding {
    const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DeferredCodexLifecycle {
    initialize_observed: bool,
    tools_list_observed: bool,
    first_tools_list_serialized_bytes: Option<u64>,
    startup_materialized: bool,
    initialize_materialized: bool,
    tools_list_materialized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectionState {
    pub(crate) phase: ConnectionPhase,
    pub(crate) client_supports_elicitation: bool,
    pub(crate) client_supports_model_invisible_user_surface: bool,
    pub(crate) client_info: Option<ManagedMcpClientInfo>,
    pub(crate) next_server_request_id: u64,
    pub(crate) session_id: String,
    pub(crate) managed_host_lifecycle_observations: bool,
    pub(crate) launch_origin: &'static str,
    status_method_call_count: u64,
    codex_binding: CodexManagedBinding,
    deferred_codex_lifecycle: DeferredCodexLifecycle,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            phase: ConnectionPhase::AwaitingInitialize,
            client_supports_elicitation: false,
            client_supports_model_invisible_user_surface: false,
            client_info: None,
            next_server_request_id: 1,
            session_id: generated_metadata_id("session", "mcp", "stdio"),
            managed_host_lifecycle_observations: false,
            launch_origin: McpLaunchOrigin::Unknown.as_str(),
            status_method_call_count: 0,
            codex_binding: CodexManagedBinding::NotApplicable,
            deferred_codex_lifecycle: DeferredCodexLifecycle::default(),
        }
    }
}

impl ConnectionState {
    fn for_launch_origin(
        launch_origin: McpLaunchOrigin,
        managed_host_session_id: Option<String>,
    ) -> Self {
        let pending_codex =
            launch_origin == McpLaunchOrigin::ManagedHost && managed_host_session_id.is_none();
        let mut state = Self {
            managed_host_lifecycle_observations: launch_origin == McpLaunchOrigin::ManagedHost
                && managed_host_session_id.is_some(),
            launch_origin: launch_origin.as_str(),
            codex_binding: if pending_codex {
                CodexManagedBinding::Pending
            } else {
                CodexManagedBinding::NotApplicable
            },
            ..Self::default()
        };
        if let Some(managed_host_session_id) = managed_host_session_id {
            state.session_id = managed_host_session_id;
        } else if pending_codex {
            state.session_id.clear();
        }
        state
    }

    fn user_channel_capabilities(&self) -> McpUserChannelCapabilities {
        McpUserChannelCapabilities::new(
            self.client_supports_elicitation,
            self.client_supports_model_invisible_user_surface,
        )
        .with_stdio_session(
            if self.codex_binding.is_pending() {
                McpLaunchOrigin::Unknown.as_str()
            } else {
                self.launch_origin
            },
            self.client_info.as_ref().map(ManagedMcpClientInfo::name),
            self.client_info.as_ref().map(ManagedMcpClientInfo::version),
        )
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ClientMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

#[derive(Debug, PartialEq)]
pub(crate) struct JsonRpcRequest {
    pub(crate) id: Value,
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct JsonRpcNotification {
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct JsonRpcFailure {
    pub(crate) id: Value,
    pub(crate) code: i64,
    pub(crate) message: &'static str,
    pub(crate) data: Option<String>,
}

pub(crate) fn handle_json_rpc_message(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    message: Value,
    lines: &mut io::Lines<impl BufRead>,
    writer: &mut impl Write,
) -> Result<Option<Value>, McpAdapterError> {
    match parse_client_message(message) {
        Ok(ClientMessage::Request(request)) => {
            handle_json_rpc_request(adapter, state, request, lines, writer).map(Some)
        }
        Ok(ClientMessage::Notification(notification)) => {
            handle_json_rpc_notification(state, notification);
            Ok(None)
        }
        Err(error) => Ok(Some(json_rpc_error(
            error.id,
            error.code,
            error.message,
            error.data,
        ))),
    }
}

pub(crate) fn parse_client_message(message: Value) -> Result<ClientMessage, JsonRpcFailure> {
    let object = match message {
        Value::Object(object) => object,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            return Err(invalid_request(
                Value::Null,
                "message must be a JSON object",
            ));
        }
    };

    let id = match object.get("id") {
        Some(value) => Some(valid_request_id(value)?),
        None => None,
    };
    let response_id = id.clone().unwrap_or(Value::Null);

    match object.get("jsonrpc") {
        Some(Value::String(version)) if version == "2.0" => (),
        _ => {
            return Err(invalid_request(
                response_id,
                "jsonrpc must be exactly \"2.0\"",
            ));
        }
    }

    let Some(Value::String(method)) = object.get("method") else {
        return Err(invalid_request(response_id, "method must be a string"));
    };
    let params = object.get("params").cloned();

    if let Some(id) = id {
        Ok(ClientMessage::Request(JsonRpcRequest {
            id,
            method: method.clone(),
            params,
        }))
    } else {
        Ok(ClientMessage::Notification(JsonRpcNotification {
            method: method.clone(),
            params,
        }))
    }
}

pub(crate) fn valid_request_id(value: &Value) -> Result<Value, JsonRpcFailure> {
    match value {
        Value::String(_) => Ok(value.clone()),
        Value::Number(number) if number.is_i64() || number.is_u64() => Ok(value.clone()),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) => {
            Err(invalid_request(
                Value::Null,
                "id must be a string or integer",
            ))
        }
    }
}

pub(crate) fn handle_json_rpc_notification(
    state: &mut ConnectionState,
    notification: JsonRpcNotification,
) {
    if notification.method == "notifications/initialized"
        && state.phase == ConnectionPhase::AwaitingInitialized
        && notification_params_are_object_or_absent(notification.params.as_ref())
    {
        state.phase = ConnectionPhase::Ready;
    }
}

pub(crate) fn notification_params_are_object_or_absent(params: Option<&Value>) -> bool {
    matches!(params, None | Some(Value::Object(_)))
}

pub(crate) fn handle_json_rpc_request<R, W>(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    request: JsonRpcRequest,
    lines: &mut io::Lines<R>,
    writer: &mut W,
) -> Result<Value, McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    if let Some(error) = lifecycle_error(state.phase, &request) {
        return Ok(error);
    }

    let response_id = request.id.clone();
    let result = match request.method.as_str() {
        "initialize" => {
            let capabilities = match validate_initialize_params(&response_id, request.params) {
                Ok(capabilities) => capabilities,
                Err(error) => return Ok(error),
            };
            if !state.codex_binding.is_pending()
                && record_managed_initialize_event(adapter, state, &capabilities.client_info)
                    == StartupObservationResult::IdentityConflict
            {
                return Ok(invalid_request_response(
                    &response_id,
                    "initialize client identity conflicts with the existing managed-session identity",
                ));
            }
            state.client_supports_elicitation = capabilities.elicitation;
            state.client_supports_model_invisible_user_surface =
                capabilities.model_invisible_user_surface;
            state.client_info = Some(capabilities.client_info);
            state.phase = ConnectionPhase::AwaitingInitialized;
            if state.codex_binding.is_pending() {
                state.deferred_codex_lifecycle.initialize_observed = true;
            } else if state.managed_host_lifecycle_observations {
                let _ = start_transport_diagnostic_session(adapter, state);
            }
            initialize_result()
        }
        "ping" => {
            if let Err(error) =
                validate_optional_object_params(&response_id, request.params, "ping")
            {
                return Ok(error);
            }
            json!({})
        }
        "tools/list" => {
            if let Err(error) =
                validate_optional_object_params(&response_id, request.params, "tools/list")
            {
                return Ok(error);
            }
            match adapter.tools() {
                Ok(tools) => {
                    let result = json!({ "tools": tools });
                    let serialized_bytes = serde_json::to_vec(&result)
                        .ok()
                        .and_then(|bytes| u64::try_from(bytes.len()).ok());
                    if state.codex_binding.is_pending() {
                        state.deferred_codex_lifecycle.tools_list_observed = true;
                        if state
                            .deferred_codex_lifecycle
                            .first_tools_list_serialized_bytes
                            .is_none()
                        {
                            state
                                .deferred_codex_lifecycle
                                .first_tools_list_serialized_bytes = serialized_bytes;
                        }
                    } else {
                        record_managed_lifecycle_event(
                            adapter,
                            state,
                            ManagedLifecycleEvent::ToolsList,
                            None,
                        );
                        if let Some(serialized_bytes) = serialized_bytes {
                            record_tools_list_metric_best_effort(adapter, state, serialized_bytes);
                        }
                    }
                    result
                }
                Err(error) => return Ok(json_rpc_error_for_adapter(response_id, error)),
            }
        }
        "tools/call" => {
            match call_tool_result_with_elicitation(
                adapter,
                &response_id,
                request.params,
                state,
                lines,
                writer,
            )? {
                Ok(result) => result,
                Err(error) => return Ok(error),
            }
        }
        _ => {
            return Ok(json_rpc_error(
                response_id,
                -32601,
                "Method not found",
                Some(request.method),
            ))
        }
    };

    Ok(json!({
        "jsonrpc": "2.0",
        "id": response_id,
        "result": result
    }))
}

pub(crate) fn lifecycle_error(state: ConnectionPhase, request: &JsonRpcRequest) -> Option<Value> {
    match state {
        ConnectionPhase::AwaitingInitialize if request.method != "initialize" => Some(
            invalid_request_response(&request.id, "initialize must be the first request"),
        ),
        ConnectionPhase::AwaitingInitialize => None,
        ConnectionPhase::AwaitingInitialized => match request.method.as_str() {
            "initialize" => Some(invalid_request_response(
                &request.id,
                "initialize has already completed",
            )),
            "tools/list" => None,
            "tools/call" => Some(invalid_request_response(
                &request.id,
                "tools/call requires notifications/initialized",
            )),
            _ => None,
        },
        ConnectionPhase::Ready if request.method == "initialize" => Some(invalid_request_response(
            &request.id,
            "initialize has already completed",
        )),
        ConnectionPhase::Ready => None,
    }
}

fn record_managed_lifecycle_event(
    adapter: &McpAdapter,
    state: &ConnectionState,
    lifecycle_event: ManagedLifecycleEvent,
    tool_name: Option<&str>,
) {
    if !state.managed_host_lifecycle_observations {
        return;
    }
    let _observation = adapter.managed_lifecycle_observation_best_effort(
        &state.session_id,
        state.launch_origin,
        lifecycle_event,
        tool_name,
    );
}

fn record_managed_initialize_event(
    adapter: &McpAdapter,
    state: &ConnectionState,
    client_info: &ManagedMcpClientInfo,
) -> StartupObservationResult {
    if !state.managed_host_lifecycle_observations {
        return StartupObservationResult::NotAttempted;
    }
    adapter.managed_initialize_observation_best_effort(
        &state.session_id,
        state.launch_origin,
        client_info,
    )
}

fn bind_codex_managed_tool_call(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
    params: &Map<String, Value>,
) -> Result<(), &'static str> {
    if matches!(state.codex_binding, CodexManagedBinding::NotApplicable) {
        return Ok(());
    }
    if state.client_info.as_ref().map(ManagedMcpClientInfo::name)
        != Some(REVIEWED_CODEX_MCP_CLIENT_NAME)
    {
        return Err("managed Codex tools/call requires the Codex MCP client identity");
    }

    let binding =
        codex_managed_call_binding(params, adapter.context.connection_internal_id.as_str())?;
    match &state.codex_binding {
        CodexManagedBinding::Pending => {
            let mut candidate = state.clone();
            candidate.session_id = binding.managed_host_session_id;
            candidate.codex_binding = CodexManagedBinding::Bound {
                thread_digest: binding.thread_digest,
            };
            candidate.managed_host_lifecycle_observations = true;
            validate_managed_stdio_session_ownership(adapter, &candidate).map_err(|_| {
                "managed Codex call metadata conflicts with the registered connection session"
            })?;
            *state = candidate;
            Ok(())
        }
        CodexManagedBinding::Bound { thread_digest }
            if state.session_id == binding.managed_host_session_id
                && *thread_digest == binding.thread_digest =>
        {
            Ok(())
        }
        CodexManagedBinding::Bound { .. } => {
            Err("managed Codex call metadata changed session or thread binding")
        }
        CodexManagedBinding::NotApplicable => Ok(()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexManagedCallBinding {
    managed_host_session_id: String,
    thread_digest: [u8; 32],
}

fn codex_managed_call_binding(
    params: &Map<String, Value>,
    connection_internal_id: &str,
) -> Result<CodexManagedCallBinding, &'static str> {
    let metadata = params
        .get("_meta")
        .and_then(Value::as_object)
        .ok_or("managed Codex tools/call requires object params._meta")?;
    let flat_thread_id = metadata
        .get("threadId")
        .and_then(Value::as_str)
        .ok_or("managed Codex tools/call requires string params._meta.threadId")?;
    let turn_metadata = metadata
        .get(CODEX_TURN_METADATA_KEY)
        .and_then(Value::as_object)
        .ok_or("managed Codex tools/call requires object params._meta.x-codex-turn-metadata")?;
    let native_session_id = codex_turn_metadata_id(turn_metadata, "session_id")?;
    let nested_thread_id = codex_turn_metadata_id(turn_metadata, "thread_id")?;
    let _turn_id = codex_turn_metadata_id(turn_metadata, "turn_id")?;
    validate_managed_host_native_session_id(flat_thread_id)
        .map_err(|_| "managed Codex tools/call contains invalid native identity metadata")?;
    if flat_thread_id != nested_thread_id {
        return Err("managed Codex tools/call thread metadata is inconsistent");
    }
    let managed_host_session_id =
        managed_host_session_id(CODEX_HOST_VALUE, connection_internal_id, native_session_id)
            .map_err(|_| "managed Codex tools/call contains invalid native identity metadata")?;
    let thread_digest =
        codex_thread_binding_digest(connection_internal_id, native_session_id, nested_thread_id);
    Ok(CodexManagedCallBinding {
        managed_host_session_id,
        thread_digest,
    })
}

fn codex_turn_metadata_id<'a>(
    metadata: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a str, &'static str> {
    let value = metadata
        .get(field)
        .and_then(Value::as_str)
        .ok_or("managed Codex tools/call requires string session, thread, and turn metadata")?;
    validate_managed_host_native_session_id(value)
        .map_err(|_| "managed Codex tools/call contains invalid native identity metadata")?;
    Ok(value)
}

fn codex_thread_binding_digest(
    connection_internal_id: &str,
    native_session_id: &str,
    native_thread_id: &str,
) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(CODEX_THREAD_BINDING_DOMAIN);
    digest.update(connection_internal_id.as_bytes());
    digest.update([0]);
    digest.update(native_session_id.as_bytes());
    digest.update([0]);
    digest.update(native_thread_id.as_bytes());
    digest.finalize().into()
}

#[cfg(test)]
mod codex_call_binding_tests {
    use super::*;

    fn valid_params() -> Map<String, Value> {
        json!({
            "name": "volicord.status",
            "arguments": {},
            "_meta": {
                "threadId": "thread.alpha",
                "x-codex-turn-metadata": {
                    "session_id": "session.alpha",
                    "thread_id": "thread.alpha",
                    "turn_id": "turn.alpha",
                    "future_field": "allowed"
                },
                "future_meta": true
            }
        })
        .as_object()
        .expect("fixture object")
        .clone()
    }

    #[test]
    fn exact_codex_call_metadata_accepts_extensions_and_has_stable_bindings() {
        let first = codex_managed_call_binding(&valid_params(), "connection.alpha")
            .expect("exact metadata should bind");
        assert_eq!(
            first.managed_host_session_id,
            managed_host_session_id("codex", "connection.alpha", "session.alpha")
                .expect("expected session mapping")
        );
        let replay = codex_managed_call_binding(&valid_params(), "connection.alpha")
            .expect("same metadata should replay");
        assert_eq!(first, replay);

        let mut next_turn = valid_params();
        next_turn["_meta"][CODEX_TURN_METADATA_KEY]["turn_id"] = json!("turn.beta");
        assert_eq!(
            codex_managed_call_binding(&next_turn, "connection.alpha")
                .expect("turn changes do not change the session/thread binding"),
            first
        );
    }

    #[test]
    fn malformed_codex_call_metadata_is_rejected_table_driven() {
        let mut cases = Vec::<(&str, Map<String, Value>)>::new();
        let mut push = |label, value: Value| {
            cases.push((
                label,
                value.as_object().expect("negative fixture object").clone(),
            ));
        };
        push(
            "missing meta",
            json!({"name":"volicord.status","arguments":{}}),
        );
        push(
            "non-object meta",
            json!({"name":"volicord.status","arguments":{},"_meta":null}),
        );
        for (label, metadata) in [
            (
                "missing flat thread",
                json!({CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"t","turn_id":"u"}}),
            ),
            (
                "non-string flat thread",
                json!({"threadId":1,CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"t","turn_id":"u"}}),
            ),
            ("missing nested metadata", json!({"threadId":"t"})),
            (
                "non-object nested metadata",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:null}),
            ),
            (
                "missing session",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:{"thread_id":"t","turn_id":"u"}}),
            ),
            (
                "missing thread",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:{"session_id":"s","turn_id":"u"}}),
            ),
            (
                "missing turn",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"t"}}),
            ),
            (
                "invalid session",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:{"session_id":"bad session","thread_id":"t","turn_id":"u"}}),
            ),
            (
                "invalid thread",
                json!({"threadId":"bad/thread",CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"bad/thread","turn_id":"u"}}),
            ),
            (
                "invalid turn",
                json!({"threadId":"t",CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"t","turn_id":""}}),
            ),
            (
                "thread mismatch",
                json!({"threadId":"t1",CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"t2","turn_id":"u"}}),
            ),
            (
                "oversized thread",
                json!({"threadId":"x".repeat(257),CODEX_TURN_METADATA_KEY:{"session_id":"s","thread_id":"x".repeat(257),"turn_id":"u"}}),
            ),
        ] {
            push(
                label,
                json!({"name":"volicord.status","arguments":{},"_meta":metadata}),
            );
        }

        for (label, params) in cases {
            assert!(
                codex_managed_call_binding(&params, "connection.alpha").is_err(),
                "{label}"
            );
        }
    }
}

fn materialize_deferred_codex_lifecycle(
    adapter: &McpAdapter,
    state: &mut ConnectionState,
) -> Result<bool, &'static str> {
    if !matches!(state.codex_binding, CodexManagedBinding::Bound { .. }) {
        return Ok(true);
    }
    let mut lifecycle_ready = true;
    if state.deferred_codex_lifecycle.initialize_observed
        && !state.deferred_codex_lifecycle.initialize_materialized
    {
        let client_info = state
            .client_info
            .as_ref()
            .ok_or("managed Codex binding is missing its initialized client identity")?;
        let result = adapter.managed_initialize_observation_at_binding_best_effort(
            &state.session_id,
            state.launch_origin,
            client_info,
        );
        if result == StartupObservationResult::IdentityConflict {
            return Err(
                "managed Codex initialize identity conflicts with the existing managed-session identity",
            );
        }
        if result == StartupObservationResult::Recorded {
            state.deferred_codex_lifecycle.startup_materialized = true;
            state.deferred_codex_lifecycle.initialize_materialized = true;
        } else {
            lifecycle_ready = false;
        }
    } else if !state.deferred_codex_lifecycle.initialize_materialized {
        lifecycle_ready = false;
    }
    if state.deferred_codex_lifecycle.tools_list_observed
        && !state.deferred_codex_lifecycle.tools_list_materialized
    {
        let result = adapter.managed_lifecycle_observation_at_binding_best_effort(
            &state.session_id,
            state.launch_origin,
            ManagedLifecycleEvent::ToolsList,
            None,
        );
        if result == StartupObservationResult::Recorded {
            state.deferred_codex_lifecycle.tools_list_materialized = true;
        } else {
            lifecycle_ready = false;
        }
    }
    Ok(lifecycle_ready)
}

pub(crate) fn initialize_result() -> Value {
    let build = crate::build_info();
    let package_version = build.package_version;
    json!({
        "_meta": {
            "io.volicord/build": build
        },
        "protocolVersion": SUPPORTED_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": package_version
        },
        "instructions": SERVER_INSTRUCTIONS
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClientCapabilities {
    elicitation: bool,
    model_invisible_user_surface: bool,
    client_info: ManagedMcpClientInfo,
}

fn validate_initialize_params(
    id: &Value,
    params: Option<Value>,
) -> Result<ClientCapabilities, Value> {
    let object = required_object_params(id, params, "initialize")?;
    if !matches!(object.get("protocolVersion"), Some(Value::String(_))) {
        return Err(invalid_params_response(
            id,
            "initialize params.protocolVersion must be a string",
        ));
    }
    if !matches!(object.get("capabilities"), Some(Value::Object(_))) {
        return Err(invalid_params_response(
            id,
            "initialize params.capabilities must be an object",
        ));
    }
    let Some(Value::Object(client_info)) = object.get("clientInfo") else {
        return Err(invalid_params_response(
            id,
            "initialize params.clientInfo must be an object",
        ));
    };
    let Some(Value::String(client_name)) = client_info.get("name") else {
        return Err(invalid_params_response(
            id,
            "initialize params.clientInfo.name must be a string",
        ));
    };
    let Some(Value::String(client_version)) = client_info.get("version") else {
        return Err(invalid_params_response(
            id,
            "initialize params.clientInfo.version must be a string",
        ));
    };
    let client_info = ManagedMcpClientInfo::new(client_name.clone(), client_version.clone())
        .map_err(|error| invalid_params_response(id, error.to_string()))?;

    let elicitation = object
        .get("capabilities")
        .and_then(Value::as_object)
        .and_then(|capabilities| capabilities.get("elicitation"))
        .is_some_and(Value::is_object);

    let model_invisible_user_surface = object
        .get("capabilities")
        .and_then(Value::as_object)
        .and_then(|capabilities| capabilities.get("experimental"))
        .and_then(Value::as_object)
        .and_then(|experimental| experimental.get("io.volicord/user-channel"))
        .and_then(Value::as_object)
        .and_then(|user_channel| user_channel.get("model_invisible_user_surface"))
        == Some(&Value::Bool(true));

    Ok(ClientCapabilities {
        elicitation,
        model_invisible_user_surface,
        client_info,
    })
}

pub(crate) fn validate_optional_object_params(
    id: &Value,
    params: Option<Value>,
    method: &str,
) -> Result<(), Value> {
    match params {
        None | Some(Value::Object(_)) => Ok(()),
        Some(_) => Err(invalid_params_response(
            id,
            format!("{method} params must be an object"),
        )),
    }
}

pub(crate) fn required_object_params(
    id: &Value,
    params: Option<Value>,
    method: &str,
) -> Result<Map<String, Value>, Value> {
    match params {
        Some(Value::Object(object)) => Ok(object),
        None | Some(_) => Err(invalid_params_response(
            id,
            format!("{method} params must be an object"),
        )),
    }
}

pub(crate) fn call_tool_result_with_elicitation<R, W>(
    adapter: &McpAdapter,
    id: &Value,
    params: Option<Value>,
    state: &mut ConnectionState,
    lines: &mut io::Lines<R>,
    writer: &mut W,
) -> Result<Result<Value, Value>, McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let diagnostic_started = Instant::now();
    let diagnostic_request_bytes = params
        .as_ref()
        .and_then(|value| serde_json::to_vec(value).ok())
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    let diagnostic_tool_name = params
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|object| object.get("name"))
        .and_then(Value::as_str)
        .filter(|tool_name| is_known_mcp_tool(tool_name))
        .map(str::to_owned);
    let object = match required_object_params(id, params, "tools/call") {
        Ok(object) => object,
        Err(error) => {
            record_tool_diagnostic_best_effort(
                adapter,
                state,
                diagnostic_started,
                diagnostic_request_bytes,
                diagnostic_tool_name.as_deref(),
                Some(&error),
                ToolDiagnosticFacts::default(),
                true,
                DiagnosticOutcome::ValidationFailure,
            );
            return Ok(Err(error));
        }
    };
    if object.contains_key("task") {
        let error = invalid_params_response(id, "tools/call task augmentation is not supported");
        record_tool_diagnostic_best_effort(
            adapter,
            state,
            diagnostic_started,
            diagnostic_request_bytes,
            diagnostic_tool_name.as_deref(),
            Some(&error),
            ToolDiagnosticFacts::default(),
            true,
            DiagnosticOutcome::ValidationFailure,
        );
        return Ok(Err(error));
    }

    let tool_name = match object.get("name").and_then(Value::as_str) {
        Some(tool_name) => tool_name,
        None => {
            let error = invalid_params_response(id, "tools/call params.name must be a string");
            record_tool_diagnostic_best_effort(
                adapter,
                state,
                diagnostic_started,
                diagnostic_request_bytes,
                None,
                Some(&error),
                ToolDiagnosticFacts::default(),
                true,
                DiagnosticOutcome::ValidationFailure,
            );
            return Ok(Err(error));
        }
    };
    if !is_known_mcp_tool(tool_name) {
        let error = json_rpc_error(
            id.clone(),
            -32602,
            "Invalid params",
            Some(format!("unknown MCP tool: {tool_name}")),
        );
        record_tool_diagnostic_best_effort(
            adapter,
            state,
            diagnostic_started,
            diagnostic_request_bytes,
            None,
            Some(&error),
            ToolDiagnosticFacts::default(),
            true,
            DiagnosticOutcome::ValidationFailure,
        );
        return Ok(Err(error));
    }
    let arguments = match object.get("arguments") {
        None => json!({}),
        Some(Value::Object(_)) => object
            .get("arguments")
            .cloned()
            .expect("arguments object should be present"),
        Some(_) => {
            let error =
                invalid_params_response(id, "tools/call params.arguments must be an object");
            record_tool_diagnostic_best_effort(
                adapter,
                state,
                diagnostic_started,
                diagnostic_request_bytes,
                Some(tool_name),
                Some(&error),
                ToolDiagnosticFacts::default(),
                true,
                DiagnosticOutcome::ValidationFailure,
            );
            return Ok(Err(error));
        }
    };
    let codex_was_pending = state.codex_binding.is_pending();
    let pre_binding_state = codex_was_pending.then(|| state.clone());
    if let Err(error) = bind_codex_managed_tool_call(adapter, state, &object) {
        return Ok(Err(invalid_params_response(id, error)));
    }
    let managed_lifecycle_ready = match materialize_deferred_codex_lifecycle(adapter, state) {
        Ok(ready) => ready,
        Err(error) => {
            if let Some(pre_binding_state) = pre_binding_state {
                *state = pre_binding_state;
            }
            return Ok(Err(invalid_request_response(id, error)));
        }
    };
    if codex_was_pending {
        let _ = start_transport_diagnostic_session(adapter, state);
        if let Some(serialized_bytes) = state
            .deferred_codex_lifecycle
            .first_tools_list_serialized_bytes
            .take()
        {
            record_tools_list_metric_best_effort(adapter, state, serialized_bytes);
        }
    }
    if tool_name == STATUS_TOOL_NAME {
        state.status_method_call_count = state.status_method_call_count.saturating_add(1);
    }
    if managed_lifecycle_ready {
        record_managed_lifecycle_event(
            adapter,
            state,
            ManagedLifecycleEvent::ToolCallReceived,
            Some(tool_name),
        );
    }
    let mutation_detail = mutation_detail_for_tool(tool_name, &arguments);
    let allow_user_action_capture = tool_name == REQUEST_USER_ACTION_TOOL_NAME
        && arguments
            .pointer("/request/operation")
            .and_then(Value::as_str)
            == Some("create");

    let session_id = state.session_id.clone();
    let output = if PUBLIC_METHOD_TOOL_NAMES.contains(&tool_name) {
        match adapter.call_tool_for_session_with_user_channel_capabilities(
            tool_name,
            arguments,
            Some(&session_id),
            state.user_channel_capabilities(),
        ) {
            Ok(response) if tool_name == REQUEST_USER_ACTION_TOOL_NAME => {
                let pending_response = response.clone();
                match user_action_tool_output(
                    adapter,
                    response,
                    allow_user_action_capture,
                    state.user_channel_capabilities(),
                    &mut state.next_server_request_id,
                    lines,
                    writer,
                ) {
                    Ok(output) => output,
                    Err(_) => ToolCallOutput::from_pipeline_response(&pending_response)?
                        .with_post_effect_failure(
                            McpPostEffectFailureCode::McpPostEffectAdapterFailed,
                        ),
                }
            }
            Ok(response) if tool_name == GET_OPERATION_RESULT_TOOL_NAME => {
                ToolCallOutput::from_operation_result_response(&response)?
            }
            Ok(response) => ToolCallOutput::from_pipeline_response(&response)?,
            Err(error @ McpAdapterError::InvalidParams { .. }) => {
                let response = tool_execution_error_result(tool_name, &error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    true,
                    DiagnosticOutcome::ValidationFailure,
                );
                return Ok(Ok(response));
            }
            Err(error @ McpAdapterError::ToolExecution { .. }) => {
                let response = tool_execution_error_result(tool_name, &error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::ToolError,
                );
                return Ok(Ok(response));
            }
            Err(error) => {
                let response = json_rpc_error_for_adapter(id.clone(), error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::TransportError,
                );
                return Ok(Err(response));
            }
        }
    } else {
        let response = match adapter.call_adapter_tool(tool_name, arguments, Some(&session_id)) {
            Ok(response) => response,
            Err(error @ McpAdapterError::InvalidParams { .. }) => {
                let response = tool_execution_error_result(tool_name, &error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    true,
                    DiagnosticOutcome::ValidationFailure,
                );
                return Ok(Ok(response));
            }
            Err(error @ McpAdapterError::ToolExecution { .. }) => {
                let response = tool_execution_error_result(tool_name, &error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::ToolError,
                );
                return Ok(Ok(response));
            }
            Err(error) => {
                let response = json_rpc_error_for_adapter(id.clone(), error);
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&response),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::TransportError,
                );
                return Ok(Err(response));
            }
        };
        let text = serde_json::to_string(&response)
            .map_err(McpAdapterError::Json)
            .map_err(|error| json_rpc_error_for_adapter(id.clone(), error));
        match text {
            Ok(text) => ToolCallOutput::success(text)?,
            Err(error) => {
                record_tool_diagnostic_best_effort(
                    adapter,
                    state,
                    diagnostic_started,
                    diagnostic_request_bytes,
                    Some(tool_name),
                    Some(&error),
                    ToolDiagnosticFacts::default(),
                    false,
                    DiagnosticOutcome::TransportError,
                );
                return Ok(Err(error));
            }
        }
    };
    let output = finalize_mutation_output(adapter, state, tool_name, mutation_detail, output)?;
    let output = materialize_local_web_handoff(
        adapter,
        mutation_detail,
        &state.user_channel_capabilities(),
        output,
    )?;

    if managed_lifecycle_ready {
        record_managed_lifecycle_event(
            adapter,
            state,
            ManagedLifecycleEvent::ToolCallCompleted,
            Some(tool_name),
        );
    }
    let diagnostic_facts = output.diagnostic_facts();
    let diagnostic_outcome =
        if response_kind_from_structured_content(&output.structured_content) == Some("rejected") {
            DiagnosticOutcome::Rejected
        } else if output.is_error {
            DiagnosticOutcome::ToolError
        } else {
            DiagnosticOutcome::Success
        };
    let response = tool_call_result_from_output(output);
    record_tool_diagnostic_best_effort(
        adapter,
        state,
        diagnostic_started,
        diagnostic_request_bytes,
        Some(tool_name),
        Some(&response),
        diagnostic_facts,
        false,
        diagnostic_outcome,
    );
    Ok(Ok(response))
}

fn mutation_detail_for_tool(tool_name: &str, arguments: &Value) -> Option<MutationDetailLevel> {
    (!READ_ONLY_METHOD_TOOL_NAMES.contains(&tool_name)
        && PUBLIC_METHOD_TOOL_NAMES.contains(&tool_name))
    .then(|| {
        arguments
            .get("detail")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default()
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MutationRefreshContext {
    project_id: ProjectId,
    task_id: TaskId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeferredLocalWebHandoff {
    project_id: ProjectId,
    user_action_request_id: UserActionRequestId,
    form_digest: String,
}

impl MutationRefreshContext {
    fn from_pipeline_response(response: &PipelineResponse) -> Option<Self> {
        Some(Self {
            project_id: response.verified_invocation.as_ref()?.project_id.clone(),
            task_id: response.resolved_task_id.clone()?,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ToolDiagnosticFacts {
    core_reached: bool,
    core_committed: bool,
    replayed: bool,
    effect_kind: Option<EffectKind>,
    effect_applied: bool,
    effect_anchor: Option<String>,
    user_channel_kind: Option<DiagnosticUserChannelKind>,
    fallback_kind: Option<DiagnosticFallbackKind>,
    product_file_write_count: u64,
    authoritative_refresh_failure: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct CanonicalMcpMutationOutcome {
    tool_name: String,
    requested_detail: MutationDetailLevel,
    facts: ToolDiagnosticFacts,
    exact_method_result: Option<Value>,
    compact_method_result: Option<Value>,
    operation_result_ref: Option<OperationResultRef>,
    authority_receipt: Option<AuthorityReceipt>,
    next_actions: Vec<NextActionSummary>,
}

impl CanonicalMcpMutationOutcome {
    fn new(
        tool_name: &str,
        requested_detail: MutationDetailLevel,
        facts: ToolDiagnosticFacts,
        exact_method_result: Option<Value>,
        operation_result_ref: Option<OperationResultRef>,
    ) -> Self {
        let compact_method_result = exact_method_result
            .as_ref()
            .and_then(|result| compact_mutation_method_result(tool_name, result).ok());
        Self {
            tool_name: tool_name.to_owned(),
            requested_detail,
            facts,
            exact_method_result,
            compact_method_result,
            operation_result_ref,
            authority_receipt: None,
            next_actions: Vec::new(),
        }
    }

    fn set_authority_refresh(
        &mut self,
        authority_receipt: AuthorityReceipt,
        next_actions: Vec<NextActionSummary>,
    ) {
        self.authority_receipt = Some(authority_receipt);
        self.next_actions = next_actions;
    }

    fn recovery_candidates(
        &self,
        include_exact: bool,
    ) -> [Option<MutationRecoveryCandidate<'_>>; 5] {
        let receipt_and_exact = if include_exact {
            self.authority_receipt
                .as_ref()
                .zip(self.exact_method_result.as_ref())
                .map(|(receipt, method_result)| MutationRecoveryCandidate {
                    authority_receipt: Some(receipt),
                    method_result: Some(method_result),
                })
        } else {
            None
        };
        let receipt_and_compact = self
            .authority_receipt
            .as_ref()
            .zip(self.compact_method_result.as_ref())
            .map(|(receipt, method_result)| MutationRecoveryCandidate {
                authority_receipt: Some(receipt),
                method_result: Some(method_result),
            });
        let receipt_only =
            self.authority_receipt
                .as_ref()
                .map(|receipt| MutationRecoveryCandidate {
                    authority_receipt: Some(receipt),
                    method_result: None,
                });
        let compact_only =
            self.compact_method_result
                .as_ref()
                .map(|method_result| MutationRecoveryCandidate {
                    authority_receipt: None,
                    method_result: Some(method_result),
                });
        [
            receipt_and_exact,
            receipt_and_compact,
            receipt_only,
            compact_only,
            Some(MutationRecoveryCandidate {
                authority_receipt: None,
                method_result: None,
            }),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MutationRecoveryCandidate<'a> {
    authority_receipt: Option<&'a AuthorityReceipt>,
    method_result: Option<&'a Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolCallOutput {
    primary_text: String,
    structured_content: Value,
    extra_texts: Vec<String>,
    is_error: bool,
    diagnostic_facts: ToolDiagnosticFacts,
    operation_result_ref: Option<OperationResultRef>,
    mutation_refresh_context: Option<MutationRefreshContext>,
    post_effect_failure: Option<McpPostEffectFailureCode>,
    host_meta: Option<Value>,
    deferred_local_web_handoff: Option<DeferredLocalWebHandoff>,
}

impl ToolCallOutput {
    fn success(primary_text: String) -> Result<Self, McpAdapterError> {
        let structured_content: Value =
            serde_json::from_str(&primary_text).map_err(McpAdapterError::Json)?;
        if !structured_content.is_object() {
            return Err(McpAdapterError::Protocol(
                "successful MCP tool output must be a JSON object".to_owned(),
            ));
        }
        Ok(Self {
            primary_text,
            structured_content,
            extra_texts: Vec::new(),
            is_error: false,
            diagnostic_facts: ToolDiagnosticFacts::default(),
            operation_result_ref: None,
            mutation_refresh_context: None,
            post_effect_failure: None,
            host_meta: None,
            deferred_local_web_handoff: None,
        })
    }

    fn from_pipeline_response(response: &PipelineResponse) -> Result<Self, McpAdapterError> {
        let mut output = Self::success(response.response_json.clone())?;
        output.operation_result_ref = response.operation_result_ref.clone();
        output.apply_pipeline_diagnostics(response);
        Ok(output)
    }

    fn from_operation_result_response(
        response: &PipelineResponse,
    ) -> Result<Self, McpAdapterError> {
        let mut output = Self::from_pipeline_response(response)?;
        if output.structured_content["base"]["response_kind"].as_str() == Some("result") {
            let start = output.structured_content["start_offset_bytes"]
                .as_u64()
                .ok_or_else(|| {
                    McpAdapterError::Protocol(
                        "operation-result page must include start_offset_bytes".to_owned(),
                    )
                })?;
            let end = output.structured_content["end_offset_bytes"]
                .as_u64()
                .ok_or_else(|| {
                    McpAdapterError::Protocol(
                        "operation-result page must include end_offset_bytes".to_owned(),
                    )
                })?;
            let complete = output.structured_content["complete"]
                .as_bool()
                .ok_or_else(|| {
                    McpAdapterError::Protocol(
                        "operation-result page must include complete".to_owned(),
                    )
                })?;
            output.primary_text = bounded_mutation_compatibility_text(format!(
                "Volicord returned historical operation-result bytes [{start}, {end}); complete={complete}. Inspect structuredContent.chunk_utf8 and do not treat historical bytes as current authority."
            ));
            if rendered_tool_call_output_size(&output)? > MAX_MCP_COMPACT_MUTATION_RESULT_BYTES {
                return Err(McpAdapterError::Protocol(
                    "operation-result page exceeded its fixed MCP output budget".to_owned(),
                ));
            }
        }
        Ok(output)
    }

    fn with_operation_result_ref(
        mut self,
        operation_result_ref: Option<OperationResultRef>,
    ) -> Self {
        self.operation_result_ref = operation_result_ref;
        self
    }

    fn with_pipeline_diagnostics(mut self, response: &PipelineResponse) -> Self {
        self.operation_result_ref = response.operation_result_ref.clone();
        self.apply_pipeline_diagnostics(response);
        self
    }

    fn apply_pipeline_diagnostics(&mut self, response: &PipelineResponse) {
        self.diagnostic_facts.core_reached = response.verified_invocation.is_some();
        self.diagnostic_facts.effect_kind = response
            .response_value
            .pointer("/base/effect_kind")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        self.diagnostic_facts.core_committed = !response.replayed
            && self.diagnostic_facts.effect_kind == Some(EffectKind::CoreCommitted);
        self.diagnostic_facts.effect_applied = matches!(
            self.diagnostic_facts.effect_kind,
            Some(EffectKind::CoreCommitted | EffectKind::StagingCreated)
        );
        self.diagnostic_facts.effect_anchor = mutation_effect_anchor(response);
        self.diagnostic_facts.replayed = response.replayed;
        self.diagnostic_facts.product_file_write_count = response
            .response_value
            .pointer("/run_summary/observed_changes/product_file_write_observed")
            .and_then(Value::as_bool)
            .is_some_and(|observed| observed)
            as u64;
        self.mutation_refresh_context = MutationRefreshContext::from_pipeline_response(response);
    }

    fn with_user_channel(mut self, channel: DiagnosticUserChannelKind) -> Self {
        self.diagnostic_facts.user_channel_kind = Some(channel);
        self
    }

    fn with_post_effect_failure(mut self, code: McpPostEffectFailureCode) -> Self {
        self.post_effect_failure = Some(code);
        self
    }

    fn with_user_action_fallback(mut self, fallback: UserActionFallback) -> Self {
        self.diagnostic_facts.fallback_kind = Some(fallback.kind);
        self.extra_texts.extend(fallback.texts);
        self.deferred_local_web_handoff = fallback.deferred_local_web_handoff;
        self
    }

    fn diagnostic_facts(&self) -> ToolDiagnosticFacts {
        self.diagnostic_facts.clone()
    }

    fn with_extra(mut self, text: impl Into<String>) -> Self {
        self.extra_texts.push(text.into());
        self
    }

    fn with_extras(mut self, texts: impl IntoIterator<Item = String>) -> Self {
        self.extra_texts.extend(texts);
        self
    }
}

fn mutation_effect_anchor(response: &PipelineResponse) -> Option<String> {
    if let Some(event_id) = response
        .response_value
        .pointer("/base/events/0/event_id")
        .and_then(Value::as_str)
    {
        return Some(format!("authority_event:{event_id}"));
    }
    if let Some(handle_id) = response
        .response_value
        .pointer("/staged_artifact_handle/handle_id")
        .and_then(Value::as_str)
    {
        return Some(format!("staged_artifact:{handle_id}"));
    }
    let effect_kind = response
        .response_value
        .pointer("/base/effect_kind")
        .and_then(Value::as_str)?;
    if !matches!(effect_kind, "core_committed" | "staging_created") {
        return None;
    }
    let project_id = response.verified_invocation.as_ref()?.project_id.as_str();
    let state_version = response
        .response_value
        .pointer("/base/state_version")
        .and_then(Value::as_u64)?;
    Some(format!("state_effect:{project_id}:{state_version}"))
}

fn finalize_mutation_output(
    adapter: &McpAdapter,
    state: &ConnectionState,
    tool_name: &str,
    detail: Option<MutationDetailLevel>,
    output: ToolCallOutput,
) -> Result<ToolCallOutput, McpAdapterError> {
    finalize_mutation_output_with_refresh(tool_name, detail, output, |context| {
        adapter.refresh_authority_status_with_user_channel_capabilities(
            &context.project_id,
            &context.task_id,
            Some(&state.session_id),
            state.user_channel_capabilities(),
        )
    })
}

fn materialize_local_web_handoff(
    adapter: &McpAdapter,
    detail: Option<MutationDetailLevel>,
    capabilities: &McpUserChannelCapabilities,
    output: ToolCallOutput,
) -> Result<ToolCallOutput, McpAdapterError> {
    materialize_local_web_handoff_with_token_creator(
        adapter,
        detail,
        capabilities,
        output,
        |runtime_home, input| create_user_action_channel_token(runtime_home, input),
    )
}

fn materialize_local_web_handoff_with_token_creator(
    adapter: &McpAdapter,
    detail: Option<MutationDetailLevel>,
    capabilities: &McpUserChannelCapabilities,
    mut output: ToolCallOutput,
    create_token: impl FnOnce(
        &Path,
        UserActionChannelTokenCreate,
    ) -> Result<UserActionChannelTokenRecord, StoreError>,
) -> Result<ToolCallOutput, McpAdapterError> {
    let Some(deferred) = output.deferred_local_web_handoff.take() else {
        return Ok(output);
    };
    if !adapter.effective_local_web_consent_available(capabilities) {
        let (outcome, failure_class) = if !capabilities.model_invisible_user_surface {
            (
                HostRuntimeProbeOutcome::Unsupported,
                HostRuntimeProbeFailureClass::ExplicitCapabilityAbsent,
            )
        } else if !adapter.local_web_consent_listener_ready() {
            (
                HostRuntimeProbeOutcome::Unavailable,
                HostRuntimeProbeFailureClass::ListenerUnavailable,
            )
        } else {
            (
                HostRuntimeProbeOutcome::Unavailable,
                HostRuntimeProbeFailureClass::BindingMismatch,
            )
        };
        adapter.record_local_web_runtime_probe_best_effort(capabilities, outcome, failure_class);
        output.diagnostic_facts.fallback_kind = Some(DiagnosticFallbackKind::CliInbox);
        return Ok(output);
    }
    let Some(context) = adapter.local_web_consent.as_ref() else {
        output.diagnostic_facts.fallback_kind = Some(DiagnosticFallbackKind::CliInbox);
        return Ok(output);
    };
    let token = match generate_bearer_token() {
        Ok(token) => token,
        Err(_) => {
            adapter.record_local_web_runtime_probe_best_effort(
                capabilities,
                HostRuntimeProbeOutcome::Failed,
                HostRuntimeProbeFailureClass::ConfigurationUnavailable,
            );
            output.diagnostic_facts.fallback_kind = Some(DiagnosticFallbackKind::CliInbox);
            return Ok(output);
        }
    };
    let url = format!(
        "{}{}?project={}&token={}",
        context.base_url,
        LOCAL_WEB_CONSENT_PATH,
        percent_encode_query(deferred.project_id.as_str()),
        percent_encode_query(&token)
    );
    let worst_case_meta = local_web_host_meta(&url, "9999-12-31T23:59:59.999999999Z");
    output.host_meta = Some(worst_case_meta);
    let response_budget = match detail.unwrap_or(MutationDetailLevel::Summary) {
        MutationDetailLevel::Summary | MutationDetailLevel::Workflow => {
            MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        }
        MutationDetailLevel::Full => MAX_MCP_FULL_MUTATION_RESULT_BYTES,
    };
    if rendered_tool_call_output_size(&output)? > response_budget {
        adapter.record_local_web_runtime_probe_best_effort(
            capabilities,
            HostRuntimeProbeOutcome::Failed,
            HostRuntimeProbeFailureClass::ConfigurationUnavailable,
        );
        output.host_meta = None;
        output.diagnostic_facts.fallback_kind = Some(DiagnosticFallbackKind::CliInbox);
        return Ok(output);
    }
    output.host_meta = None;
    let Some(_issuance_lease) = adapter.local_web_consent_issuance_lease(capabilities) else {
        adapter.record_local_web_runtime_probe_best_effort(
            capabilities,
            HostRuntimeProbeOutcome::Unavailable,
            HostRuntimeProbeFailureClass::ListenerUnavailable,
        );
        output.diagnostic_facts.fallback_kind = Some(DiagnosticFallbackKind::CliInbox);
        return Ok(output);
    };
    let record = match create_token(
        adapter.runtime_home.as_path(),
        UserActionChannelTokenCreate {
            token,
            project_id: deferred.project_id.as_str().to_owned(),
            channel_kind: UserActionChannelKind::LocalWebConsent,
            connection_internal_id: adapter.context.connection_internal_id.to_string(),
            user_action_request_id: deferred.user_action_request_id.as_str().to_owned(),
            capture_basis: VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB.to_owned(),
            created_metadata_json: json!({
                "fallback_kind": "local_web_consent",
                "delivery_surface": LOCAL_WEB_CONSENT_DELIVERY_SURFACE,
                "endpoint": LOCAL_WEB_CONSENT_PATH,
                "form_digest": deferred.form_digest,
            })
            .to_string(),
        },
    ) {
        Ok(record) => record,
        Err(_) => {
            adapter.record_local_web_runtime_probe_best_effort(
                capabilities,
                HostRuntimeProbeOutcome::Failed,
                HostRuntimeProbeFailureClass::ConfigurationUnavailable,
            );
            output.diagnostic_facts.fallback_kind = Some(DiagnosticFallbackKind::CliInbox);
            return Ok(output);
        }
    };
    adapter.record_local_web_runtime_probe_best_effort(
        capabilities,
        HostRuntimeProbeOutcome::Passed,
        HostRuntimeProbeFailureClass::None,
    );
    output.host_meta = Some(local_web_host_meta(&url, &record.expires_at));
    output.diagnostic_facts.fallback_kind = Some(DiagnosticFallbackKind::LocalWebConsent);
    debug_assert!(rendered_tool_call_output_size(&output).is_ok_and(|size| size <= response_budget));
    Ok(output)
}

fn local_web_host_meta(url: &str, expires_at: &str) -> Value {
    json!({
        "io.volicord/user-channel": {
            "kind": "local_web_consent",
            "url": url,
            "expires_at": expires_at,
        }
    })
}

fn finalize_mutation_output_with_refresh<F>(
    tool_name: &str,
    detail: Option<MutationDetailLevel>,
    mut output: ToolCallOutput,
    refresh: F,
) -> Result<ToolCallOutput, McpAdapterError>
where
    F: FnOnce(&MutationRefreshContext) -> Result<PipelineResponse, McpAdapterError>,
{
    let Some(detail) = detail else {
        return Ok(output);
    };
    if output.is_error {
        return Ok(output);
    }
    if response_kind_from_structured_content(&output.structured_content) != Some("result") {
        output.primary_text = bounded_mutation_compatibility_text(format!(
            "Volicord {tool_name} returned response_kind={}; inspect structuredContent for the authoritative result.",
            response_kind_from_structured_content(&output.structured_content)
                .unwrap_or("unknown")
        ));
        return Ok(output);
    }

    let original_method_result = std::mem::take(&mut output.structured_content);
    let operation_result_ref = output.operation_result_ref.clone();
    let mut outcome = CanonicalMcpMutationOutcome::new(
        tool_name,
        detail,
        output.diagnostic_facts.clone(),
        Some(original_method_result),
        operation_result_ref,
    );
    let Some(context) = output.mutation_refresh_context.clone() else {
        return authoritative_refresh_failure_output(&outcome);
    };
    let (receipt, next_actions) = match refresh(&context) {
        Ok(response) => match validated_authority_refresh(&context, &response) {
            Ok(refreshed) => refreshed,
            Err(()) => return authoritative_refresh_failure_output(&outcome),
        },
        Err(_) => return authoritative_refresh_failure_output(&outcome),
    };
    outcome.set_authority_refresh(receipt, next_actions);
    let authority_receipt = outcome
        .authority_receipt
        .as_ref()
        .expect("validated canonical mutation outcome requires an authority receipt");

    if let Some(code) = output.post_effect_failure {
        return mutation_post_effect_failure_output(&outcome, code);
    }
    output.primary_text = match authority_receipt_compatibility_text(tool_name, authority_receipt) {
        Ok(text) => text,
        Err(_) => {
            return mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )
        }
    };
    output.mutation_refresh_context = None;
    let Some(compact_method_result) = outcome.compact_method_result.clone() else {
        return mutation_post_effect_failure_output(
            &outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        );
    };
    let method_result = match detail {
        MutationDetailLevel::Full => outcome
            .exact_method_result
            .clone()
            .expect("canonical mutation outcome requires an exact result"),
        MutationDetailLevel::Summary | MutationDetailLevel::Workflow => compact_method_result,
    };
    let projected = match detail {
        MutationDetailLevel::Summary => serde_json::to_value(McpMutationSummaryResponse {
            operation_result_ref: outcome.operation_result_ref.clone().into(),
            authority_receipt: authority_receipt.clone(),
            method_result,
        }),
        MutationDetailLevel::Workflow => serde_json::to_value(McpMutationWorkflowResponse {
            operation_result_ref: outcome.operation_result_ref.clone().into(),
            authority_receipt: authority_receipt.clone(),
            method_result,
            next_actions: outcome.next_actions.clone(),
        }),
        MutationDetailLevel::Full => serde_json::to_value(McpMutationFullResponse {
            operation_result_ref: outcome.operation_result_ref.clone().into(),
            authority_receipt: authority_receipt.clone(),
            method_result,
        }),
    };
    output.structured_content = match projected {
        Ok(projected) => projected,
        Err(_) => {
            return mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )
        }
    };

    let result = tool_call_result_from_output(output.clone());
    let response_budget = match detail {
        MutationDetailLevel::Summary | MutationDetailLevel::Workflow => {
            MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        }
        MutationDetailLevel::Full => MAX_MCP_FULL_MUTATION_RESULT_BYTES,
    };
    let rendered_size = match serde_json::to_vec(&result) {
        Ok(rendered) => rendered.len(),
        Err(_) => {
            return mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )
        }
    };
    if rendered_size > response_budget {
        return mutation_response_budget_exceeded_output(&outcome);
    }
    Ok(output)
}

fn response_kind_from_structured_content(value: &Value) -> Option<&str> {
    value
        .pointer("/agent_workflow_result/base/response_kind")
        .or_else(|| value.pointer("/base/response_kind"))
        .and_then(Value::as_str)
}

fn compact_mutation_method_result(
    tool_name: &str,
    method_result: &Value,
) -> Result<Value, McpAdapterError> {
    let effect = compact_mutation_effect(method_result)?;
    match tool_name {
        PREPARE_EVIDENCE_CAPTURE_TOOL_NAME => {
            let result: PrepareEvidenceCaptureResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpPrepareEvidenceCaptureCompactResult {
                effect,
                capture_intent_ref: result.capture_intent_ref,
                capture_intent: result.capture_intent,
                expires_at: result.expires_at,
            })
            .map_err(McpAdapterError::Json)
        }
        PREPARE_WRITE_TOOL_NAME => {
            let result: PrepareWriteResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpPrepareWriteCompactResult {
                effect,
                decision: result.decision,
                write_ticket_id: result.write_ticket_id,
                write_ticket_ref: result.write_ticket_ref,
                write_ticket: result.write_ticket,
                write_ticket_effect: result.write_ticket_effect,
                allowed_path_patterns: result.allowed_path_patterns,
                denied_path_patterns: result.denied_path_patterns,
                write_decision_reasons: result.write_decision_reasons,
                user_action_draft: result.user_action_draft,
            })
            .map_err(McpAdapterError::Json)
        }
        STAGE_ARTIFACT_TOOL_NAME => {
            let result: StageArtifactResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpStageArtifactCompactResult {
                effect,
                evidence_state: result.evidence_state,
                staged_artifact_handle: result.staged_artifact_handle,
                expires_at: result.expires_at,
            })
            .map_err(McpAdapterError::Json)
        }
        RECORD_RUN_TOOL_NAME => {
            let result: RecordRunResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            let evidence_observation_refs = result
                .evidence_observations
                .iter()
                .map(|observation| StateRecordRef {
                    record_kind: StateRecordKind::EvidenceObservation,
                    record_id: RecordId::new(observation.observation_id.as_str()),
                    project_id: observation.project_id.clone(),
                    task_id: Some(observation.task_id.clone()).into(),
                    produced_at_state_version: effect.state_version.into(),
                })
                .collect();
            let evidence_producer_refs = result
                .evidence_producers
                .iter()
                .map(|producer| StateRecordRef {
                    record_kind: StateRecordKind::EvidenceProducer,
                    record_id: RecordId::new(producer.evidence_producer_id.as_str()),
                    project_id: producer.project_id.clone(),
                    task_id: Some(producer.task_id.clone()).into(),
                    produced_at_state_version: effect.state_version.into(),
                })
                .collect();
            let close_basis_anchor =
                result
                    .current_close_basis
                    .map(|basis| McpRecordRunCloseBasisAnchor {
                        close_basis_revision: basis.close_basis_revision,
                        scope_revision: basis.scope_revision,
                        source_run_ref: basis.source_run_ref,
                        evidence_summary_ref: basis.evidence_summary_ref,
                    });
            serde_json::to_value(McpRecordRunCompactResult {
                effect,
                run_ref: result.run_summary.run_ref,
                registered_artifact_refs: result.registered_artifacts,
                evidence_observation_refs,
                evidence_producer_refs,
                close_basis_anchor: close_basis_anchor.into(),
            })
            .map_err(McpAdapterError::Json)
        }
        REQUEST_USER_ACTION_TOOL_NAME => compact_request_user_action_result(effect, method_result),
        RECONCILE_CHANGES_TOOL_NAME => {
            let result: ReconcileChangesResult =
                serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
            serde_json::to_value(McpReconcileChangesCompactResult {
                effect,
                unresolved_changes: result.unresolved_changes,
                resolved_changes: result.resolved_changes,
                pending_user_action_summaries: result.pending_user_action_summaries,
                rejected_resolution_requests: result.rejected_resolution_requests,
            })
            .map_err(McpAdapterError::Json)
        }
        INTAKE_TOOL_NAME | UPDATE_SCOPE_TOOL_NAME | CLOSE_TASK_TOOL_NAME => {
            serde_json::to_value(effect).map_err(McpAdapterError::Json)
        }
        _ => Err(McpAdapterError::Protocol(format!(
            "missing compact mutation result projection for {tool_name}"
        ))),
    }
}

fn compact_mutation_effect(
    method_result: &Value,
) -> Result<McpMutationEffectSummary, McpAdapterError> {
    let method_result = method_result
        .get("agent_workflow_result")
        .unwrap_or(method_result);
    let base: ToolResultBase =
        serde_json::from_value(method_result["base"].clone()).map_err(McpAdapterError::Json)?;
    Ok(McpMutationEffectSummary {
        effect_kind: base.effect_kind,
        state_version: base.state_version,
        events: base.events,
    })
}

fn compact_request_user_action_result(
    effect: McpMutationEffectSummary,
    method_result: &Value,
) -> Result<Value, McpAdapterError> {
    let compound: McpRequestUserActionResponse =
        serde_json::from_value(method_result.clone()).map_err(McpAdapterError::Json)?;
    let agent_result = match compound.agent_workflow_result {
        volicord_types::ToolResponse::Result(result) => result,
        _ => {
            return Err(McpAdapterError::Protocol(
                "request-user-action compact projection requires a result branch".to_owned(),
            ))
        }
    };
    let resolution_summary = compound
        .user_channel_resolution
        .as_ref()
        .map(|resolution| resolution.resolution_summary.clone());
    serde_json::to_value(McpRequestUserActionCompactResult {
        effect,
        agent_workflow_result_replayed: compound.agent_workflow_result_replayed,
        user_action_request_summary: agent_result.user_action_request_summary,
        current_projection_state_version: compound.current_projection_state_version,
        current_projection_observed_at: compound.current_projection_observed_at,
        user_action_resolution_ref: compound.user_channel_resolution_ref,
        status: compound.current_status,
        resolution_summary: resolution_summary.into(),
        derived_refs: compound.derived_refs,
    })
    .map_err(McpAdapterError::Json)
}

fn validated_authority_refresh(
    context: &MutationRefreshContext,
    response: &PipelineResponse,
) -> Result<(AuthorityReceipt, Vec<NextActionSummary>), ()> {
    validate_authority_status(
        &response.response_value,
        &AuthorityStatusExpectation::new(context.project_id.clone(), context.task_id.clone()),
    )
    .map_err(|_| ())
    .map(|validated| validated.into_authority_projection())
}

fn authority_receipt_compatibility_text(
    tool_name: &str,
    receipt: &AuthorityReceipt,
) -> Result<String, McpAdapterError> {
    let close_state = serde_json::to_value(receipt.close_state)
        .map_err(McpAdapterError::Json)?
        .as_str()
        .unwrap_or("unknown")
        .to_owned();
    let next_actor = serde_json::to_value(receipt.next_actor)
        .map_err(McpAdapterError::Json)?
        .as_str()
        .unwrap_or("unknown")
        .to_owned();
    Ok(bounded_mutation_compatibility_text(format!(
        "Volicord {tool_name} refreshed Task {} at state_version {}; close_state={close_state}; next_actor={next_actor}. Inspect structuredContent for the authority receipt.",
        receipt.task_ref.record_id.as_str(),
        receipt.state_version,
    )))
}

fn select_bounded_mutation_recovery<F>(
    outcome: &CanonicalMcpMutationOutcome,
    include_exact: bool,
    exhausted_message: &'static str,
    build_output: F,
) -> Result<ToolCallOutput, McpAdapterError>
where
    F: Fn(&MutationRecoveryCandidate<'_>) -> Result<ToolCallOutput, McpAdapterError>,
{
    for candidate in outcome
        .recovery_candidates(include_exact)
        .into_iter()
        .flatten()
    {
        let output = build_output(&candidate)?;
        if rendered_tool_call_output_size(&output)? <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES {
            return Ok(output);
        }
    }
    Err(McpAdapterError::Protocol(exhausted_message.to_owned()))
}

fn mutation_response_budget_exceeded_output(
    outcome: &CanonicalMcpMutationOutcome,
) -> Result<ToolCallOutput, McpAdapterError> {
    let tool_name = outcome.tool_name.as_str();
    let requested_detail = outcome.requested_detail;
    let method_name = method_name_for_tool(tool_name).ok_or_else(|| {
        McpAdapterError::Protocol(format!(
            "missing MethodName mapping for mutation tool {tool_name}"
        ))
    })?;
    let requested_detail_label = match requested_detail {
        MutationDetailLevel::Summary => "summary",
        MutationDetailLevel::Workflow => "workflow",
        MutationDetailLevel::Full => "full",
    };
    let mut facts = outcome.facts.clone();
    facts.authoritative_refresh_failure = false;
    let build_output =
        |candidate: &MutationRecoveryCandidate<'_>| -> Result<ToolCallOutput, McpAdapterError> {
            let receipt_preserved = candidate.authority_receipt.is_some();
            let method_result_preserved = candidate.method_result.is_some();
            let structured_content =
                serde_json::to_value(McpMutationResponseBudgetExceeded::<Value> {
                    code: McpMutationProjectionErrorCode::McpResponseBudgetExceeded,
                    tool_name: method_name,
                    requested_detail,
                    retryable: false,
                    reached_core: facts.core_reached,
                    committed: facts.core_committed,
                    effect_kind: facts.effect_kind.into(),
                    effect_applied: facts.effect_applied,
                    effect_anchor: facts.effect_anchor.clone().into(),
                    operation_result_ref: outcome.operation_result_ref.clone().into(),
                    authority_receipt: candidate.authority_receipt.cloned().into(),
                    method_result: RequiredNullable::new(candidate.method_result.cloned()),
                    authoritative_refresh_succeeded: true,
                    response_projection_omitted: true,
                    status_read_required: true,
                    completion_claim_withheld: true,
                })
                .map_err(McpAdapterError::Json)?;
            let preserved_guidance = match (receipt_preserved, method_result_preserved) {
                (true, true) => {
                    "The fresh authority receipt and compact method_result are preserved"
                }
                (true, false) => {
                    "The fresh authority receipt is preserved; the compact method_result exceeded the recovery budget"
                }
                (false, true) => {
                    "The compact method_result is preserved; the fresh authority receipt exceeded the recovery budget"
                }
                (false, false) => {
                    "The fresh authority receipt and compact method_result exceeded the recovery budget"
                }
            };
            let exact_result_guidance = if outcome.operation_result_ref.is_some() {
                " Retrieve the exact historical result with volicord.get_operation_result."
            } else {
                ""
            };
            Ok(ToolCallOutput {
                primary_text: bounded_mutation_compatibility_text(format!(
                    "Volicord {tool_name} reached Core (effect_applied={}, committed={}) and refreshed current authority, but the requested {requested_detail_label} projection exceeded the MCP response budget. {preserved_guidance}.{exact_result_guidance} Read volicord.status before making an authority claim. Do not retry this mutation.",
                    facts.effect_applied, facts.core_committed
                )),
                structured_content,
                extra_texts: Vec::new(),
                is_error: false,
                diagnostic_facts: facts.clone(),
                operation_result_ref: outcome.operation_result_ref.clone(),
                mutation_refresh_context: None,
                post_effect_failure: None,
                host_meta: None,
                deferred_local_web_handoff: None,
            })
        };
    select_bounded_mutation_recovery(
        outcome,
        false,
        "bounded mutation budget recovery exceeded its fixed output budget",
        build_output,
    )
}

fn mutation_post_effect_failure_output(
    outcome: &CanonicalMcpMutationOutcome,
    code: McpPostEffectFailureCode,
) -> Result<ToolCallOutput, McpAdapterError> {
    let tool_name = outcome.tool_name.as_str();
    let requested_detail = outcome.requested_detail;
    let method_name = method_name_for_tool(tool_name).ok_or_else(|| {
        McpAdapterError::Protocol(format!(
            "missing MethodName mapping for mutation tool {tool_name}"
        ))
    })?;
    let mut facts = outcome.facts.clone();
    facts.authoritative_refresh_failure = false;
    let exact_result_guidance = if outcome.operation_result_ref.is_some() {
        " Retrieve the exact historical result with volicord.get_operation_result."
    } else {
        ""
    };
    let build_output =
        |candidate: &MutationRecoveryCandidate<'_>| -> Result<ToolCallOutput, McpAdapterError> {
            let method_result = candidate
                .method_result
                .map(|method_result| {
                    method_result.as_object().cloned().ok_or_else(|| {
                        McpAdapterError::Protocol(
                            "post-effect method_result must remain a JSON object".to_owned(),
                        )
                    })
                })
                .transpose()?;
            let structured_content = serde_json::to_value(McpMutationPostEffectFailure {
                code,
                tool_name: method_name,
                requested_detail,
                retryable: false,
                reached_core: facts.core_reached,
                committed: facts.core_committed,
                effect_kind: facts.effect_kind.into(),
                effect_applied: facts.effect_applied,
                effect_anchor: facts.effect_anchor.clone().into(),
                operation_result_ref: outcome.operation_result_ref.clone().into(),
                authority_receipt: candidate.authority_receipt.cloned().into(),
                method_result: method_result.into(),
                authoritative_refresh_succeeded: true,
                response_projection_omitted: true,
                status_read_required: true,
                completion_claim_withheld: true,
            })
            .map_err(McpAdapterError::Json)?;
            Ok(ToolCallOutput {
                primary_text: bounded_mutation_compatibility_text(format!(
                    "Volicord {tool_name} observed an applied mutation effect and refreshed current authority, but post-effect adapter work could not produce the normal response projection. Do not retry this mutation; inspect structuredContent.{exact_result_guidance} Read volicord.status before acting."
                )),
                structured_content,
                extra_texts: Vec::new(),
                is_error: false,
                diagnostic_facts: facts.clone(),
                operation_result_ref: outcome.operation_result_ref.clone(),
                mutation_refresh_context: None,
                post_effect_failure: None,
                host_meta: None,
                deferred_local_web_handoff: None,
            })
        };
    select_bounded_mutation_recovery(
        outcome,
        true,
        "bounded post-effect recovery exceeded its fixed output budget",
        build_output,
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolCallResultRef<'a> {
    content: Vec<ToolCallTextContentRef<'a>>,
    structured_content: &'a Value,
    is_error: bool,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    host_meta: Option<&'a Value>,
}

#[derive(Serialize)]
struct ToolCallTextContentRef<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    text: &'a str,
}

fn rendered_tool_call_output_size(output: &ToolCallOutput) -> Result<usize, McpAdapterError> {
    let mut content = Vec::with_capacity(1 + output.extra_texts.len());
    content.push(ToolCallTextContentRef {
        kind: "text",
        text: output.primary_text.as_str(),
    });
    content.extend(
        output
            .extra_texts
            .iter()
            .map(|text| ToolCallTextContentRef {
                kind: "text",
                text: text.as_str(),
            }),
    );
    serde_json::to_vec(&ToolCallResultRef {
        content,
        structured_content: &output.structured_content,
        is_error: output.is_error,
        host_meta: output.host_meta.as_ref(),
    })
    .map(|rendered| rendered.len())
    .map_err(McpAdapterError::Json)
}

fn authoritative_refresh_failure_output(
    outcome: &CanonicalMcpMutationOutcome,
) -> Result<ToolCallOutput, McpAdapterError> {
    let tool_name = outcome.tool_name.as_str();
    let method_name = method_name_for_tool(tool_name).ok_or_else(|| {
        McpAdapterError::Protocol(format!(
            "missing MethodName mapping for mutation tool {tool_name}"
        ))
    })?;
    let mut facts = outcome.facts.clone();
    facts.authoritative_refresh_failure = true;
    let exact_result_guidance = if outcome.operation_result_ref.is_some() {
        " Retrieve the exact historical result with volicord.get_operation_result, then"
    } else {
        ""
    };
    let build_output =
        |candidate: &MutationRecoveryCandidate<'_>| -> Result<ToolCallOutput, McpAdapterError> {
            let method_result_preserved = candidate.method_result.is_some();
            let structured_content =
                serde_json::to_value(McpAuthoritativeRefreshFailure::<Value> {
                    code: ErrorCode::McpUnavailable,
                    tool_name: method_name,
                    retryable: false,
                    reached_core: facts.core_reached,
                    committed: facts.core_committed,
                    effect_kind: facts.effect_kind.into(),
                    effect_applied: facts.effect_applied,
                    effect_anchor: facts.effect_anchor.clone().into(),
                    operation_result_ref: outcome.operation_result_ref.clone().into(),
                    method_result: RequiredNullable::new(candidate.method_result.cloned()),
                    status_read_required: true,
                    completion_claim_withheld: true,
                })
                .map_err(McpAdapterError::Json)?;
            let method_result_guidance = if method_result_preserved {
                "The compact method_result is preserved"
            } else {
                "The compact method_result could not be included"
            };
            Ok(ToolCallOutput {
                primary_text: bounded_mutation_compatibility_text(format!(
                    "Volicord withheld the {tool_name} success or completion claim because authoritative status refresh was unavailable. {method_result_guidance}.{exact_result_guidance} Read volicord.status before acting. Do not retry this mutation."
                )),
                structured_content,
                extra_texts: Vec::new(),
                is_error: false,
                diagnostic_facts: facts.clone(),
                operation_result_ref: outcome.operation_result_ref.clone(),
                mutation_refresh_context: None,
                post_effect_failure: None,
                host_meta: None,
                deferred_local_web_handoff: None,
            })
        };
    select_bounded_mutation_recovery(
        outcome,
        false,
        "bounded authoritative refresh recovery exceeded its fixed output budget",
        build_output,
    )
}

fn method_name_for_tool(tool_name: &str) -> Option<MethodName> {
    match tool_name {
        INTAKE_TOOL_NAME => Some(MethodName::Intake),
        UPDATE_SCOPE_TOOL_NAME => Some(MethodName::UpdateScope),
        STATUS_TOOL_NAME => Some(MethodName::Status),
        GET_OPERATION_RESULT_TOOL_NAME => Some(MethodName::GetOperationResult),
        CHECK_CLOSE_TOOL_NAME => Some(MethodName::CheckClose),
        PREPARE_EVIDENCE_CAPTURE_TOOL_NAME => Some(MethodName::PrepareEvidenceCapture),
        PREPARE_WRITE_TOOL_NAME => Some(MethodName::PrepareWrite),
        STAGE_ARTIFACT_TOOL_NAME => Some(MethodName::StageArtifact),
        RECORD_RUN_TOOL_NAME => Some(MethodName::RecordRun),
        REQUEST_USER_ACTION_TOOL_NAME => Some(MethodName::RequestUserAction),
        RECONCILE_CHANGES_TOOL_NAME => Some(MethodName::ReconcileChanges),
        CLOSE_TASK_TOOL_NAME => Some(MethodName::CloseTask),
        _ => None,
    }
}

fn bounded_mutation_compatibility_text(mut text: String) -> String {
    if text.len() <= MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES {
        return text;
    }
    let mut boundary = MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES.saturating_sub(3);
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    text.truncate(boundary);
    text.push_str("...");
    text
}

fn validate_managed_stdio_session_ownership(
    adapter: &McpAdapter,
    state: &ConnectionState,
) -> Result<(), McpAdapterError> {
    if !state.managed_host_lifecycle_observations {
        return Ok(());
    }
    if validate_managed_host_session_id(&state.session_id).is_err() {
        return Err(McpAdapterError::Environment(
            "MANAGED_HOST_SESSION_BINDING_CONFLICT: managed stdio requires an opaque mhs_ session"
                .to_owned(),
        ));
    }
    let connection = agent_connection_record_read_only(
        &adapter.runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .map_err(McpAdapterError::Store)?
    .ok_or_else(|| {
        McpAdapterError::Environment(
            "MANAGED_HOST_SESSION_BINDING_CONFLICT: managed stdio connection is unavailable"
                .to_owned(),
        )
    })?;
    let project_ids = if let Some(project_ids) = adapter.context.project_allowlist.as_ref() {
        project_ids
            .iter()
            .map(|project_id| project_id.as_str().to_owned())
            .collect::<Vec<_>>()
    } else {
        list_connection_projects_read_only(
            &adapter.runtime_home,
            adapter.context.connection_internal_id.as_str(),
        )
        .map_err(McpAdapterError::Store)?
        .into_iter()
        .map(|project| project.project_id)
        .collect::<Vec<_>>()
    };
    for project_id in project_ids {
        if let Some(existing) = agent_session(&adapter.runtime_home, &project_id, &state.session_id)
            .map_err(McpAdapterError::Store)?
        {
            if existing.connection_internal_id != adapter.context.connection_internal_id.as_str()
                || existing.host_kind != connection.host_kind
            {
                return Err(McpAdapterError::Environment(
                    "MANAGED_HOST_SESSION_BINDING_CONFLICT: existing session ownership does not match this managed stdio connection"
                        .to_owned(),
                ));
            }
        }
    }

    Ok(())
}

fn stdio_diagnostic_project_id(adapter: &McpAdapter) -> Option<String> {
    adapter
        .context
        .project_allowlist
        .as_ref()
        .filter(|projects| projects.len() == 1)
        .and_then(|projects| projects.first())
        .map(|project| project.as_str().to_owned())
        .or_else(|| {
            list_connection_projects_read_only(
                &adapter.runtime_home,
                adapter.context.connection_internal_id.as_str(),
            )
            .ok()
            .filter(|projects| projects.len() == 1)
            .and_then(|projects| projects.first().map(|project| project.project_id.clone()))
        })
}

fn start_transport_diagnostic_session(
    adapter: &McpAdapter,
    state: &ConnectionState,
) -> Result<(), StoreError> {
    let connection = agent_connection_record_read_only(
        &adapter.runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .ok()
    .flatten();
    let host_kind = connection
        .as_ref()
        .map(|record| DiagnosticHostKind::from_connection_host_kind(&record.host_kind));
    let project_id = stdio_diagnostic_project_id(adapter);
    let transport = if state.launch_origin == McpLaunchOrigin::Unknown.as_str() {
        DiagnosticTransport::LocalHttp
    } else {
        DiagnosticTransport::McpStdio
    };
    let build = crate::build_info();
    start_diagnostic_session(
        &adapter.runtime_home,
        DiagnosticSessionStart {
            session_id: &state.session_id,
            connection_id: Some(adapter.context.connection_internal_id.as_str()),
            project_id: project_id.as_deref(),
            transport,
            host_kind,
            package_version: build.package_version,
            build_id: &build.build_id,
        },
    )
}

fn record_tools_list_metric_best_effort(
    adapter: &McpAdapter,
    state: &ConnectionState,
    serialized_bytes: u64,
) {
    if state.codex_binding.is_pending()
        || start_transport_diagnostic_session(adapter, state).is_err()
    {
        return;
    }
    let _ = record_workflow_metric_event(
        &adapter.runtime_home,
        &WorkflowMetricEvent {
            session_id: state.session_id.clone(),
            metric_kind: WorkflowMetricKind::ToolsListSerializedBytes,
            value: serialized_bytes,
            method_name: None,
            integration_profile: None,
            decision: None,
            observation_confidence: None,
            outcome: Some(WorkflowMetricOutcome::Success),
        },
    );
}

fn record_public_method_metrics_best_effort(
    adapter: &McpAdapter,
    state: &ConnectionState,
    tool_name: Option<&str>,
    outcome: DiagnosticOutcome,
) {
    let Some(method_name) = tool_name.and_then(method_name_for_tool) else {
        return;
    };
    let outcome = workflow_metric_outcome(outcome);
    let _ = record_workflow_metric_event(
        &adapter.runtime_home,
        &WorkflowMetricEvent {
            session_id: state.session_id.clone(),
            metric_kind: WorkflowMetricKind::McpMethodCall,
            value: 1,
            method_name: Some(method_name),
            integration_profile: None,
            decision: None,
            observation_confidence: None,
            outcome: Some(outcome),
        },
    );
    if method_name == MethodName::Status && state.status_method_call_count > 1 {
        let _ = record_workflow_metric_event(
            &adapter.runtime_home,
            &WorkflowMetricEvent {
                session_id: state.session_id.clone(),
                metric_kind: WorkflowMetricKind::StatusReread,
                value: 1,
                method_name: None,
                integration_profile: None,
                decision: None,
                observation_confidence: None,
                outcome: Some(outcome),
            },
        );
    }
}

const fn workflow_metric_outcome(outcome: DiagnosticOutcome) -> WorkflowMetricOutcome {
    match outcome {
        DiagnosticOutcome::Success => WorkflowMetricOutcome::Success,
        DiagnosticOutcome::Rejected => WorkflowMetricOutcome::Rejected,
        DiagnosticOutcome::ValidationFailure => WorkflowMetricOutcome::ValidationFailure,
        DiagnosticOutcome::ToolError => WorkflowMetricOutcome::ToolError,
        DiagnosticOutcome::TransportError => WorkflowMetricOutcome::TransportError,
        DiagnosticOutcome::Unavailable => WorkflowMetricOutcome::Unavailable,
    }
}

#[allow(clippy::too_many_arguments)]
fn record_tool_diagnostic_best_effort(
    adapter: &McpAdapter,
    state: &ConnectionState,
    started: Instant,
    request_bytes: u64,
    tool_name: Option<&str>,
    response: Option<&Value>,
    facts: ToolDiagnosticFacts,
    validation_failure: bool,
    outcome: DiagnosticOutcome,
) {
    if state.codex_binding.is_pending() {
        return;
    }
    let elapsed = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    let response_bytes = response
        .and_then(|value| serde_json::to_vec(value).ok())
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0);
    if start_transport_diagnostic_session(adapter, state).is_err() {
        return;
    }
    let _ = record_diagnostic_event(
        &adapter.runtime_home,
        DiagnosticEvent {
            session_id: &state.session_id,
            event_kind: DiagnosticEventKind::McpToolCall,
            tool_name,
            latency_micros: elapsed,
            request_bytes,
            response_bytes,
            validation_failure,
            core_reached: facts.core_reached,
            core_committed: facts.core_committed,
            replayed: facts.replayed,
            user_channel_kind: facts.user_channel_kind,
            fallback_kind: facts.fallback_kind,
            product_file_write_count: facts.product_file_write_count,
            authoritative_refresh_failure: facts.authoritative_refresh_failure,
            outcome,
        },
    );
    record_public_method_metrics_best_effort(adapter, state, tool_name, outcome);
}

pub(crate) fn tool_call_result_from_output(output: ToolCallOutput) -> Value {
    let host_meta = output.host_meta.clone();
    let mut content = vec![json!({
        "type": "text",
        "text": output.primary_text
    })];
    content.extend(output.extra_texts.into_iter().map(|text| {
        json!({
            "type": "text",
            "text": text
        })
    }));

    let mut result = json!({
        "content": content,
        "structuredContent": output.structured_content,
        "isError": output.is_error
    });
    if let Some(host_meta) = host_meta {
        result["_meta"] = host_meta;
    }
    result
}

pub(crate) fn user_action_tool_output<R, W>(
    adapter: &McpAdapter,
    pending_response: PipelineResponse,
    allow_user_action_capture: bool,
    capabilities: McpUserChannelCapabilities,
    server_request_sequence: &mut u64,
    lines: &mut io::Lines<R>,
    writer: &mut W,
) -> Result<ToolCallOutput, McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let Some(coordinate) = pending_user_action_coordinate_from_response(&pending_response)? else {
        return ToolCallOutput::from_pipeline_response(&pending_response);
    };
    let current = current_user_action_projection_for_coordinate(adapter, &coordinate)?;
    if !allow_user_action_capture {
        return compound_user_action_output(&pending_response, &current);
    }
    if current.status != UserActionStatus::Pending {
        return Ok(compound_user_action_output(&pending_response, &current)?.with_extra(format!(
            "Volicord did not open a new User Channel prompt because user action `{}` is currently {}.",
            coordinate.user_action_request_id.as_str(),
            user_action_status_text(current.status)
        )));
    }
    if pending_response.replayed {
        return Ok(compound_user_action_output(&pending_response, &current)?
            .with_user_action_fallback(cli_recovery_fallback()));
    }
    let Some(pending) = pending_user_action_from_response_with_capabilities(
        adapter,
        &pending_response,
        &capabilities,
    )?
    else {
        return Err(McpAdapterError::Protocol(
            "the trusted User Channel projection did not contain the newly pending request"
                .to_owned(),
        ));
    };

    if !capabilities.host_elicitation_available {
        let fallback = user_action_fallback(adapter, &pending, &capabilities)?;
        return Ok(compound_user_action_output(&pending_response, &current)?
            .with_user_action_fallback(fallback));
    }

    if !agent_facing_user_action_input_allowed(&pending) {
        let fallback = user_action_fallback(adapter, &pending, &capabilities)?;
        return Ok(compound_user_action_output(&pending_response, &current)?
            .with_extra(format!(
                "Volicord did not open host prompt input for pending user action `{}` because its complete presentation requires a user-only channel. No elicitation or prompt-capture presentation was opened. Do not ask the user to enter secrets, credentials, tokens, or private keys through agent-facing host input.",
                pending.request.user_action_request_id.as_str()
            ))
            .with_user_action_fallback(fallback));
    }

    let request_id = next_server_request_id("elicit_user_action", server_request_sequence);
    let Some(request) = elicitation_create_request(&request_id, &pending)? else {
        let fallback = user_action_fallback(adapter, &pending, &capabilities)?;
        return Ok(compound_user_action_output(&pending_response, &current)?
            .with_extra(format!(
                "Volicord did not open host prompt input for pending user action `{}` because the complete elicitation request exceeds the {}-byte wire budget; no partial form was sent.",
                pending.request.user_action_request_id.as_str(),
                MAX_MCP_ELICITATION_WIRE_BYTES
            ))
            .with_user_action_fallback(fallback));
    };
    write_json_line(writer, request)?;
    writer.flush().map_err(McpAdapterError::Io)?;

    match read_elicitation_response(&request_id, lines) {
        ElicitationReply::Accepted(content) => {
            let resolution = match resolution_from_elicitation(&pending, &content) {
                Ok(resolution) => resolution,
                Err(message) => {
                    let current = current_user_action_projection(adapter, &pending)?;
                    return Ok(compound_user_action_output(&pending_response, &current)?
                        .with_extra(format!(
                            "Volicord rejected the host prompt response: {message}. User action `{}` is currently {}.",
                            pending.request.user_action_request_id.as_str(),
                            user_action_status_text(current.status)
                        ))
                        .with_extras(pending_user_action_resume_texts(&pending, &current)));
                }
            };
            match resolve_elicited_user_action(adapter, &pending, resolution, &request_id)? {
            ElicitedResolutionOutcome::Committed(current) => Ok(
                compound_user_action_output(&pending_response, &current)?
                    .with_user_channel(DiagnosticUserChannelKind::McpElicitation)
                    .with_extra(format!(
                "Volicord resolved pending user action `{}` through host prompt input with User Channel basis `{}`.",
                pending.request.user_action_request_id.as_str(),
                VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
            )),
            ),
            ElicitedResolutionOutcome::NotCommitted(current) => Ok(
                compound_user_action_output(&pending_response, &current)?
                    .with_extra(format!(
                    "Volicord did not record the host prompt response; user action `{}` is currently {}. No second Agent Workflow request was created.",
                    pending.request.user_action_request_id.as_str(),
                    user_action_status_text(current.status)
                ))
                    .with_extras(pending_user_action_resume_texts(&pending, &current)),
            ),
            }
        }
        ElicitationReply::Declined => match reject_resolution(&pending) {
            Some(resolution) => match resolve_elicited_user_action(
                adapter,
                &pending,
                resolution,
                &request_id,
            )? {
                ElicitedResolutionOutcome::Committed(current) => Ok(
                    compound_user_action_output(&pending_response, &current)?
                        .with_user_channel(DiagnosticUserChannelKind::McpElicitation)
                        .with_extra(format!(
                    "Volicord resolved pending user action `{}` with its stored reject choice through host prompt input with User Channel basis `{}`.",
                    pending.request.user_action_request_id.as_str(),
                    VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
                )),
                ),
                ElicitedResolutionOutcome::NotCommitted(current) => Ok(
                    compound_user_action_output(&pending_response, &current)?
                        .with_extra(format!(
                        "Volicord did not record the declined host prompt response; user action `{}` is currently {}. No second Agent Workflow request was created.",
                        pending.request.user_action_request_id.as_str(),
                        user_action_status_text(current.status)
                    ))
                        .with_extras(pending_user_action_resume_texts(&pending, &current)),
                ),
            },
            None => {
                let current = current_user_action_projection(adapter, &pending)?;
                Ok(compound_user_action_output(&pending_response, &current)?
                    .with_extra(format!(
                    "The MCP client declined the host prompt request, but this user action has no stored reject choice. User action `{}` is currently {}.",
                    pending.request.user_action_request_id.as_str(),
                    user_action_status_text(current.status)
                ))
                    .with_extras(pending_user_action_resume_texts(&pending, &current)))
            },
        },
        ElicitationReply::Cancelled => {
            let current = current_user_action_projection(adapter, &pending)?;
            Ok(compound_user_action_output(&pending_response, &current)?
                .with_extra(format!(
                "The MCP client cancelled or dismissed host prompt input for user action `{}`. Its current status is {}.",
                pending.request.user_action_request_id.as_str(),
                user_action_status_text(current.status)
            ))
                .with_extras(pending_user_action_resume_texts(&pending, &current)))
        },
        ElicitationReply::Invalid(message) => {
            let current = current_user_action_projection(adapter, &pending)?;
            Ok(compound_user_action_output(&pending_response, &current)?
                .with_extra(format!(
                "Volicord rejected the host prompt response: {message}. User action `{}` is currently {}.",
                pending.request.user_action_request_id.as_str(),
                user_action_status_text(current.status)
            ))
                .with_extras(pending_user_action_resume_texts(&pending, &current)))
        },
        ElicitationReply::Unavailable(message) => {
            let current = current_user_action_projection(adapter, &pending)?;
            if current.status != UserActionStatus::Pending {
                return Ok(compound_user_action_output(&pending_response, &current)?.with_extra(
                    "Host prompt input became unavailable after the user action had already left pending status. No fallback request was created.",
                ));
            }
            let fallback = user_action_fallback(adapter, &pending, &capabilities)?;
            Ok(compound_user_action_output(&pending_response, &current)?
            .with_extra(format!(
                "Host prompt input was unavailable after the client advertised support: {message}."
            ))
            .with_user_action_fallback(fallback))
        }
    }
}

fn compound_user_action_output(
    pending_response: &PipelineResponse,
    current: &CurrentUserActionProjection,
) -> Result<ToolCallOutput, McpAdapterError> {
    let compound = McpRequestUserActionResponse {
        agent_workflow_result: serde_json::from_value::<RequestUserActionResponse>(
            pending_response.response_value.clone(),
        )
        .map_err(McpAdapterError::Json)?,
        agent_workflow_result_replayed: pending_response.replayed,
        current_projection_state_version: current.observed_state_version,
        current_projection_observed_at: current.observed_at.clone(),
        current_status: current.status,
        user_channel_resolution_ref: current.user_action_resolution_ref.clone().into(),
        user_channel_resolution: current.user_action_resolution.clone().into(),
        derived_refs: current.derived_refs.clone(),
    };
    let response_value = serde_json::to_value(compound).map_err(McpAdapterError::Json)?;
    let response_json = serde_json::to_string(&response_value).map_err(McpAdapterError::Json)?;
    Ok(ToolCallOutput::success(response_json)?
        .with_operation_result_ref(pending_response.operation_result_ref.clone())
        .with_pipeline_diagnostics(pending_response))
}

fn current_user_action_projection(
    adapter: &McpAdapter,
    pending: &PendingUserAction,
) -> Result<CurrentUserActionProjection, McpAdapterError> {
    current_user_action_projection_for_coordinate(
        adapter,
        &PendingUserActionCoordinate {
            project_id: pending.request.project_id.clone(),
            task_id: pending.request.task_id.clone(),
            user_action_request_id: pending.request.user_action_request_id.clone(),
        },
    )
}

fn current_user_action_projection_for_coordinate(
    adapter: &McpAdapter,
    coordinate: &PendingUserActionCoordinate,
) -> Result<CurrentUserActionProjection, McpAdapterError> {
    let current = adapter
        .core
        .current_user_action_projection(&coordinate.project_id, &coordinate.user_action_request_id)
        .map_err(McpAdapterError::Core)?
        .ok_or_else(|| {
            McpAdapterError::Protocol(
                "committed user-action request disappeared during current-state reread".to_owned(),
            )
        })?;
    if current.project_id != coordinate.project_id
        || current.user_action_request_id != coordinate.user_action_request_id
    {
        return Err(McpAdapterError::Protocol(
            "current user-action projection does not match the original request".to_owned(),
        ));
    }
    Ok(current)
}

fn user_action_status_text(status: UserActionStatus) -> &'static str {
    match status {
        UserActionStatus::Pending => "pending",
        UserActionStatus::Resolved => "resolved",
        UserActionStatus::Stale => "stale",
        UserActionStatus::Superseded => "superseded",
        UserActionStatus::Expired => "expired",
    }
}

fn pending_user_action_resume_texts(
    _pending: &PendingUserAction,
    current: &CurrentUserActionProjection,
) -> Vec<String> {
    if current.status != UserActionStatus::Pending {
        return Vec::new();
    }
    vec![generic_user_channel_fallback_text()]
}

#[derive(Debug, Clone)]
pub(crate) struct PendingUserAction {
    pub(crate) request: UserActionRequest,
    pub(crate) inbox_item: UserActionInboxItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingUserActionCoordinate {
    project_id: ProjectId,
    task_id: TaskId,
    user_action_request_id: UserActionRequestId,
}

fn pending_user_action_coordinate_from_response(
    response: &PipelineResponse,
) -> Result<Option<PendingUserActionCoordinate>, McpAdapterError> {
    if response.response_value["base"]["response_kind"].as_str() != Some("result") {
        return Ok(None);
    }
    let result = serde_json::from_value::<RequestUserActionResult>(response.response_value.clone())
        .map_err(McpAdapterError::Json)?;
    let invocation = response.verified_invocation.as_ref().ok_or_else(|| {
        McpAdapterError::Protocol(
            "successful request_user_action response omitted verified invocation facts".to_owned(),
        )
    })?;
    let task_id = response.resolved_task_id.clone().ok_or_else(|| {
        McpAdapterError::Protocol(
            "successful request_user_action response omitted its resolved Task".to_owned(),
        )
    })?;
    Ok(Some(PendingUserActionCoordinate {
        project_id: invocation.project_id.clone(),
        task_id,
        user_action_request_id: result.user_action_request_summary.user_action_request_id,
    }))
}

#[cfg(test)]
pub(crate) fn pending_user_action_from_response(
    adapter: &McpAdapter,
    response: &PipelineResponse,
) -> Result<Option<PendingUserAction>, McpAdapterError> {
    let Some(invocation) = response.verified_invocation.as_ref() else {
        return pending_user_action_from_response_with_capabilities(
            adapter,
            response,
            &McpUserChannelCapabilities::default(),
        );
    };
    let capabilities = McpUserChannelCapabilities::new(
        invocation.host_elicitation_available,
        invocation.local_web_consent_available,
    );
    pending_user_action_from_response_with_capabilities(adapter, response, &capabilities)
}

fn pending_user_action_from_response_with_capabilities(
    adapter: &McpAdapter,
    response: &PipelineResponse,
    capabilities: &McpUserChannelCapabilities,
) -> Result<Option<PendingUserAction>, McpAdapterError> {
    let Some(coordinate) = pending_user_action_coordinate_from_response(response)? else {
        return Ok(None);
    };
    let invocation = response
        .verified_invocation
        .as_ref()
        .expect("pending coordinate requires verified invocation");
    let Some(session_id) = invocation.session_id.as_deref() else {
        return Ok(None);
    };
    let Some(projection) = adapter.user_channel_inbox_projection(
        &coordinate.project_id,
        &coordinate.task_id,
        Some(session_id),
        capabilities.clone(),
    )?
    else {
        return Ok(None);
    };
    let Some(item) = projection
        .items
        .into_iter()
        .find(|item| item.request.user_action_request_id == coordinate.user_action_request_id)
    else {
        return Ok(None);
    };
    if item.request.project_id != coordinate.project_id
        || item.request.task_id != coordinate.task_id
        || item.request.status != UserActionStatus::Pending
        || item.inbox_item.user_action_request_id != coordinate.user_action_request_id
        || item.inbox_item.project_id != coordinate.project_id
        || item.inbox_item.task_id != coordinate.task_id
        || item.inbox_item.status != UserActionStatus::Pending
    {
        return Err(McpAdapterError::Protocol(
            "trusted User Channel projection coordinates do not match the agent request summary"
                .to_owned(),
        ));
    }
    let canonical_form = item.request.body.capture_form().map_err(|error| {
        McpAdapterError::Protocol(format!(
            "pending user-action request body cannot produce a canonical form: {error}"
        ))
    })?;
    item.inbox_item
        .form
        .validate_canonical_size()
        .map_err(|error| {
            McpAdapterError::Protocol(format!(
                "pending user-action inbox form is invalid: {error}"
            ))
        })?;
    if item.inbox_item.form != canonical_form {
        return Err(McpAdapterError::Protocol(
            "pending user-action inbox form does not match the canonical request-body projection"
                .to_owned(),
        ));
    }
    Ok(Some(PendingUserAction {
        request: item.request,
        inbox_item: item.inbox_item,
    }))
}

pub(crate) fn elicitation_create_request(
    id: &str,
    pending: &PendingUserAction,
) -> Result<Option<Value>, McpAdapterError> {
    let presentation = UserActionPresentationPlan::from_form(&pending.inbox_item.form)
        .map_err(McpAdapterError::Json)?;
    let form_text = presentation
        .render_plain_text()
        .map_err(McpAdapterError::Json)?;
    let (message, requested_schema) = match &presentation.form {
        UserActionPresentationForm::Choice {
            choices,
            note_allowed,
            note_max_chars,
        } => {
            let option_ids = choices
                .iter()
                .map(|choice| choice.choice_id.as_str())
                .collect::<Vec<_>>();
            let option_names = choices
                .iter()
                .map(|choice| {
                    format!(
                        "{} ({}){}",
                        choice.label,
                        choice.choice_id,
                        if choice.is_default { " [default]" } else { "" }
                    )
                })
                .collect::<Vec<_>>();
            let mut properties = json!({
                "selected_option_id": {
                    "type": "string",
                    "title": "User-action choice",
                    "description": "The exact stored choice_id selected by the user.",
                    "enum": option_ids,
                    "enumNames": option_names
                }
            });
            if *note_allowed {
                properties["note"] = json!({
                    "type": "string",
                    "title": "Optional note",
                    "description": "Optional user note. Do not include secrets, credentials, tokens, or private keys.",
                    "maxLength": note_max_chars
                });
            }
            (
                format!(
                    "Volicord needs one user-owned choice for Task `{}`.\n\nQuestion: {}\n\nContext: {}\n\nCanonical closed form:\n{}\n\nSelect exactly one stored choice. Do not enter secrets, credentials, tokens, private keys, or other private secret material.",
                    pending.request.task_id.as_str(), pending.inbox_item.question, pending.inbox_item.context_summary, form_text
                ),
                json!({"type":"object","properties":properties,"required":["selected_option_id"],"additionalProperties":false}),
            )
        }
        UserActionPresentationForm::EvidenceObservation {
            targets,
            artifacts,
            relevance_options,
            summary_max_chars,
        } => {
            let target_values = targets
                .iter()
                .map(|target| target.selector.as_str())
                .collect::<Vec<_>>();
            let target_names = targets
                .iter()
                .map(|target| format!("{} ({})", target.display_name, target.selector))
                .collect::<Vec<_>>();
            let artifact_ids = artifacts
                .iter()
                .map(|artifact| artifact.artifact_id.as_str())
                .collect::<Vec<_>>();
            let artifact_names = artifacts
                .iter()
                .map(|artifact| format!("{} ({})", artifact.display_name, artifact.artifact_id))
                .collect::<Vec<_>>();
            (
                format!(
                    "Volicord needs one user-owned evidence observation for Task `{}`.\n\nQuestion: {}\n\nContext: {}\n\nCanonical closed form:\n{}\n\nSelect one stored target, one or more stored artifacts, a relevance value, and enter a concise observation summary. Do not enter secrets, credentials, tokens, private keys, or other private secret material.",
                    pending.request.task_id.as_str(), pending.inbox_item.question, pending.inbox_item.context_summary, form_text
                ),
                json!({
                    "type":"object",
                    "properties":{
                        "selected_target": {"type":"string","title":"Evidence target","enum":target_values,"enumNames":target_names},
                        "selected_artifact_ids": {"type":"array","title":"Observed artifacts","items":{"type":"string","enum":artifact_ids,"enumNames":artifact_names},"minItems":1,"uniqueItems":true},
                        "relevance_status": {"type":"string","title":"Relevance","enum":relevance_options},
                        "summary": {"type":"string","title":"Observation summary","maxLength":summary_max_chars}
                    },
                    "required":["selected_target","selected_artifact_ids","relevance_status","summary"],
                    "additionalProperties":false
                }),
            )
        }
    };

    let request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": ELICITATION_CREATE_METHOD,
        "params": {
            "message": message,
            "requestedSchema": requested_schema
        }
    });
    let wire_bytes = serde_json::to_vec(&request)
        .map_err(McpAdapterError::Json)?
        .len()
        .saturating_add(1);
    Ok((wire_bytes <= MAX_MCP_ELICITATION_WIRE_BYTES).then_some(request))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ElicitationReply {
    Accepted(Value),
    Declined,
    Cancelled,
    Invalid(String),
    Unavailable(String),
}

pub(crate) fn read_elicitation_response<R: BufRead>(
    request_id: &str,
    lines: &mut io::Lines<R>,
) -> ElicitationReply {
    let Some(line) = lines.next() else {
        return ElicitationReply::Unavailable(
            "stdin closed before the client responded".to_owned(),
        );
    };
    let line = match line {
        Ok(line) => line,
        Err(error) => {
            return ElicitationReply::Unavailable(format!(
                "failed to read elicitation response: {error}"
            ))
        }
    };
    let value: Value = match serde_json::from_str(&line) {
        Ok(value) => value,
        Err(error) => {
            return ElicitationReply::Invalid(format!("response was not valid JSON: {error}"))
        }
    };
    let Some(object) = value.as_object() else {
        return ElicitationReply::Invalid("response must be a JSON-RPC object".to_owned());
    };
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return ElicitationReply::Invalid("response jsonrpc must be exactly \"2.0\"".to_owned());
    }
    if object.get("id").and_then(Value::as_str) != Some(request_id) {
        return ElicitationReply::Invalid(
            "response id did not match the elicitation request".to_owned(),
        );
    }
    if let Some(error) = object.get("error") {
        return ElicitationReply::Unavailable(format!(
            "client returned JSON-RPC error: {}",
            concise_json(error)
        ));
    }
    let Some(result) = object.get("result").and_then(Value::as_object) else {
        return ElicitationReply::Invalid("response result must be an object".to_owned());
    };
    match result.get("action").and_then(Value::as_str) {
        Some("accept") => {
            let Some(content) = result.get("content").and_then(Value::as_object) else {
                return ElicitationReply::Invalid(
                    "accepted elicitation must include object content".to_owned(),
                );
            };
            ElicitationReply::Accepted(Value::Object(content.clone()))
        }
        Some("decline") => ElicitationReply::Declined,
        Some("cancel") => ElicitationReply::Cancelled,
        Some(other) => {
            ElicitationReply::Invalid(format!("unsupported elicitation action `{other}`"))
        }
        None => ElicitationReply::Invalid("response result.action must be a string".to_owned()),
    }
}

pub(crate) enum ElicitedResolutionOutcome {
    Committed(CurrentUserActionProjection),
    NotCommitted(CurrentUserActionProjection),
}

fn resolution_from_elicitation(
    pending: &PendingUserAction,
    content: &Value,
) -> Result<UserActionResolutionInput, String> {
    let content = content
        .as_object()
        .ok_or_else(|| "accepted elicitation content must be an object".to_owned())?;
    match &pending.inbox_item.form {
        UserActionInboxForm::Choice {
            choices,
            note_allowed,
            note_max_chars,
        } => {
            let allowed = if *note_allowed {
                &["selected_option_id", "note"][..]
            } else {
                &["selected_option_id"][..]
            };
            reject_unknown_field_names(
                content.keys().map(String::as_str),
                allowed,
                "accepted elicitation content",
            )?;
            let selected = content
                .get("selected_option_id")
                .and_then(Value::as_str)
                .ok_or_else(|| "content.selected_option_id must be a string".to_owned())?;
            let choice = choices
                .iter()
                .find(|choice| choice.choice_id.as_str() == selected)
                .ok_or_else(|| "content.selected_option_id is not a stored choice".to_owned())?;
            let note = match content.get("note") {
                None if *note_allowed => None,
                Some(Value::String(note))
                    if *note_allowed && note.chars().count() <= *note_max_chars as usize =>
                {
                    Some(note.clone())
                }
                Some(Value::String(_)) if *note_allowed => {
                    return Err("content.note exceeds its character limit".to_owned())
                }
                Some(_) if *note_allowed => {
                    return Err("content.note must be a string when supplied".to_owned())
                }
                None => None,
                Some(_) => return Err("this form does not accept a note".to_owned()),
            };
            Ok(UserActionResolutionInput::Choice {
                selected_option_id: choice.choice_id.clone(),
                note: note.into(),
            })
        }
        UserActionInboxForm::EvidenceObservation {
            target_candidates,
            artifact_candidates,
            relevance_options,
            summary_max_chars,
        } => {
            reject_unknown_field_names(
                content.keys().map(String::as_str),
                &[
                    "selected_target",
                    "selected_artifact_ids",
                    "relevance_status",
                    "summary",
                ],
                "accepted elicitation content",
            )?;
            let selected_target = content
                .get("selected_target")
                .and_then(Value::as_str)
                .ok_or_else(|| "content.selected_target must be a string".to_owned())?;
            let presentation = UserActionPresentationPlan::from_form(&pending.inbox_item.form)
                .map_err(|_| "stored evidence form cannot be rendered".to_owned())?;
            let UserActionPresentationForm::EvidenceObservation { targets, .. } =
                &presentation.form
            else {
                return Err("stored evidence form has the wrong variant".to_owned());
            };
            let target_index = targets
                .iter()
                .position(|target| target.selector == selected_target)
                .ok_or_else(|| "content.selected_target is not a stored target".to_owned())?;
            let target = target_candidates[target_index].clone();
            let artifact_values = content
                .get("selected_artifact_ids")
                .and_then(Value::as_array)
                .ok_or_else(|| "content.selected_artifact_ids must be an array".to_owned())?;
            if artifact_values.is_empty() {
                return Err("content.selected_artifact_ids must not be empty".to_owned());
            }
            let mut artifact_ids = Vec::with_capacity(artifact_values.len());
            let mut seen = BTreeSet::new();
            for value in artifact_values {
                let id = value.as_str().ok_or_else(|| {
                    "content.selected_artifact_ids must contain strings".to_owned()
                })?;
                let artifact = artifact_candidates
                    .iter()
                    .find(|artifact| artifact.artifact_id.as_str() == id)
                    .ok_or_else(|| {
                        "content.selected_artifact_ids contains an unknown artifact".to_owned()
                    })?;
                if !seen.insert(id.to_owned()) {
                    return Err(
                        "content.selected_artifact_ids must not contain duplicates".to_owned()
                    );
                }
                artifact_ids.push(artifact.artifact_id.clone());
            }
            let relevance_value = content
                .get("relevance_status")
                .cloned()
                .ok_or_else(|| "content.relevance_status is required".to_owned())?;
            let relevance_status: EvidenceRelevanceStatus = serde_json::from_value(relevance_value)
                .map_err(|_| "content.relevance_status is invalid".to_owned())?;
            if !relevance_options.contains(&relevance_status) {
                return Err("content.relevance_status is not a stored option".to_owned());
            }
            let summary = content
                .get("summary")
                .and_then(Value::as_str)
                .ok_or_else(|| "content.summary must be a string".to_owned())?;
            if summary.trim().is_empty() || summary.chars().count() > *summary_max_chars as usize {
                return Err(
                    "content.summary must be non-empty and within its character limit".to_owned(),
                );
            }
            Ok(UserActionResolutionInput::EvidenceObservation {
                target,
                artifact_ids,
                relevance_status,
                summary: summary.to_owned(),
            })
        }
    }
}

pub(crate) fn resolve_elicited_user_action(
    adapter: &McpAdapter,
    pending: &PendingUserAction,
    resolution: UserActionResolutionInput,
    elicitation_request_id: &str,
) -> Result<ElicitedResolutionOutcome, McpAdapterError> {
    let channel_submission_id = format!("mcp_elicitation:{elicitation_request_id}");
    let request = ResolveUserActionRequest {
        envelope: ToolEnvelope {
            project_id: pending.request.project_id.clone(),
            task_id: Some(pending.request.task_id.clone()).into(),
            request_id: RequestId::new(format!(
                "req_{}",
                sanitize_metadata_component(&channel_submission_id)
            )),
            idempotency_key: Some(IdempotencyKey::new(channel_submission_id.clone())).into(),
            expected_state_version: RequiredNullable::null(),
            dry_run: false,
            locale: Some(DEFAULT_LOCALE.to_owned()).into(),
        },
        user_action_request_id: pending.request.user_action_request_id.clone(),
        channel_submission_id,
        resolution,
    };
    let invocation = InvocationContext::new(
        pending.request.project_id.clone(),
        ActorSource::LocalUser,
        OperationCategory::UserOnly,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
    );
    let response = adapter
        .core
        .resolve_user_action(request, invocation)
        .map_err(McpAdapterError::Core)?;
    let current = current_user_action_projection(adapter, pending)?;
    if response.response_value["base"]["response_kind"].as_str() == Some("result") {
        Ok(ElicitedResolutionOutcome::Committed(current))
    } else {
        Ok(ElicitedResolutionOutcome::NotCommitted(current))
    }
}

fn reject_resolution(pending: &PendingUserAction) -> Option<UserActionResolutionInput> {
    let UserActionRequestBody::Choice(choice) = &pending.request.body else {
        return None;
    };
    let selected = choice
        .options
        .iter()
        .find(|option| option.machine_action == UserActionOptionAction::Reject)?;
    Some(UserActionResolutionInput::Choice {
        selected_option_id: selected.option_id.clone(),
        note: RequiredNullable::null(),
    })
}

pub(crate) struct UserActionFallback {
    texts: Vec<String>,
    kind: DiagnosticFallbackKind,
    deferred_local_web_handoff: Option<DeferredLocalWebHandoff>,
}

pub(crate) fn user_action_fallback(
    adapter: &McpAdapter,
    pending: &PendingUserAction,
    capabilities: &McpUserChannelCapabilities,
) -> Result<UserActionFallback, McpAdapterError> {
    let local_web_available = pending
        .inbox_item
        .preferred_capture_path
        .as_ref()
        .into_iter()
        .chain(pending.inbox_item.fallbacks.iter())
        .any(|path| path.available && path.kind == "local_web_consent");
    if local_web_available && adapter.effective_local_web_consent_available(capabilities) {
        return local_web_consent_fallback(pending);
    }
    Ok(cli_recovery_fallback())
}

pub(crate) fn local_web_consent_fallback(
    pending: &PendingUserAction,
) -> Result<UserActionFallback, McpAdapterError> {
    let form_digest =
        canonical_json_bare_sha256(&pending.inbox_item.form).map_err(McpAdapterError::Json)?;
    Ok(UserActionFallback {
        texts: vec![generic_user_channel_fallback_text()],
        kind: DiagnosticFallbackKind::CliInbox,
        deferred_local_web_handoff: Some(DeferredLocalWebHandoff {
            project_id: pending.request.project_id.clone(),
            user_action_request_id: pending.request.user_action_request_id.clone(),
            form_digest,
        }),
    })
}

pub(crate) fn cli_recovery_fallback() -> UserActionFallback {
    UserActionFallback {
        texts: vec![generic_user_channel_fallback_text()],
        kind: DiagnosticFallbackKind::CliInbox,
        deferred_local_web_handoff: None,
    }
}

fn generic_user_channel_fallback_text() -> String {
    "A pending UserAction requires the user. Use the host's User Channel when available; otherwise open `volicord inbox`. Resume the existing request after the user completes it."
        .to_owned()
}

pub(crate) fn percent_encode_query(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push(hex_digit(byte >> 4));
            encoded.push(hex_digit(byte & 0x0f));
        }
    }
    encoded
}

pub(crate) fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + (value - 10)) as char,
        _ => unreachable!("hex digit input is masked to four bits"),
    }
}

pub(crate) fn agent_facing_user_action_input_allowed(pending: &PendingUserAction) -> bool {
    UserActionPresentationPlan::from_form(&pending.inbox_item.form)
        .and_then(|presentation| {
            presentation.agent_facing_input_safety(
                &pending.inbox_item.question,
                &pending.inbox_item.context_summary,
            )
        })
        .map(UserActionPresentationSafety::allows_agent_facing_input)
        .unwrap_or(false)
}

pub(crate) fn next_server_request_id(prefix: &str, next_server_request_id: &mut u64) -> String {
    let sequence = *next_server_request_id;
    *next_server_request_id = next_server_request_id.saturating_add(1);
    format!("{prefix}_{sequence}")
}

pub(crate) fn concise_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "unserializable JSON value".to_owned())
}

pub(crate) fn is_known_mcp_tool(tool_name: &str) -> bool {
    PUBLIC_METHOD_TOOL_NAMES.contains(&tool_name) || ADAPTER_UTILITY_TOOL_NAMES.contains(&tool_name)
}

pub(crate) fn tool_execution_error_result(
    requested_tool_name: &str,
    error: &McpAdapterError,
) -> Value {
    let structured = match error {
        McpAdapterError::InvalidParams {
            issues, truncated, ..
        } => McpToolErrorResponse {
            code: McpToolErrorCode::InvalidArguments,
            tool_name: requested_tool_name.to_owned(),
            retryable: true,
            reached_core: false,
            committed: false,
            reported_issue_count: issues.len(),
            truncated: *truncated,
            issues: issues.clone(),
        },
        McpAdapterError::ToolExecution { tool_name, message } => {
            let (path, message) = if tool_name == "project routing" {
                (
                    "/project_selector".to_owned(),
                    format!(
                        "{message}. Use volicord.list_projects when project selection is unclear."
                    ),
                )
            } else {
                (
                    String::new(),
                    format!("{tool_name} failed before reaching Core: {message}"),
                )
            };
            McpToolErrorResponse {
                code: McpToolErrorCode::AdapterPreconditionFailed,
                tool_name: requested_tool_name.to_owned(),
                retryable: false,
                reached_core: false,
                committed: false,
                reported_issue_count: 1,
                truncated: false,
                issues: vec![McpToolErrorIssue {
                    path,
                    code: McpToolIssueCode::AdapterPreconditionFailed,
                    message,
                }],
            }
        }
        _ => McpToolErrorResponse {
            code: McpToolErrorCode::AdapterPreconditionFailed,
            tool_name: requested_tool_name.to_owned(),
            retryable: false,
            reached_core: false,
            committed: false,
            reported_issue_count: 1,
            truncated: false,
            issues: vec![McpToolErrorIssue {
                path: String::new(),
                code: McpToolIssueCode::AdapterPreconditionFailed,
                message: "Tool execution failed before reaching Core.".to_owned(),
            }],
        },
    };
    bounded_tool_error_result(structured)
}

fn bounded_tool_error_result(mut structured: McpToolErrorResponse) -> Value {
    let mut truncated = structured.truncated;
    if structured.issues.len() > MAX_VALIDATION_ISSUES {
        structured.issues.truncate(MAX_VALIDATION_ISSUES);
        truncated = true;
    }
    structured.issues = structured
        .issues
        .into_iter()
        .map(|issue| {
            let (issue, issue_truncated) = bound_mcp_tool_error_issue(issue);
            truncated |= issue_truncated;
            issue
        })
        .collect();
    if structured.issues.is_empty() {
        structured.issues.push(McpToolErrorIssue {
            path: String::new(),
            code: McpToolIssueCode::AdapterPreconditionFailed,
            message: "Tool execution failed before reaching Core.".to_owned(),
        });
        truncated = true;
    }

    loop {
        structured.reported_issue_count = structured.issues.len();
        structured.truncated = truncated;
        let result = serialize_tool_error_result(&structured);
        let result_bytes = serde_json::to_vec(&result)
            .expect("MCP tool error result should serialize")
            .len();
        if result_bytes <= MAX_MCP_TOOL_ERROR_RESULT_BYTES {
            return result;
        }
        if structured.issues.len() > 1 {
            structured.issues.pop();
            truncated = true;
            continue;
        }

        // Individual field limits and known tool names make this fallback
        // unreachable in normal operation, but keep the byte contract closed
        // if surrounding JSON overhead changes later.
        structured.issues[0].path.clear();
        structured.issues[0].message = "Validation failed before reaching Core.".to_owned();
        structured.truncated = true;
        let fallback = serialize_tool_error_result(&structured);
        assert!(
            serde_json::to_vec(&fallback)
                .expect("fallback MCP tool error result should serialize")
                .len()
                <= MAX_MCP_TOOL_ERROR_RESULT_BYTES,
            "known-tool MCP error fallback exceeded its response byte limit"
        );
        return fallback;
    }
}

fn serialize_tool_error_result(structured: &McpToolErrorResponse) -> Value {
    let structured_content =
        serde_json::to_value(structured).expect("MCP tool error should serialize");
    let text = serde_json::to_string(&structured_content)
        .expect("MCP tool error compatibility text should serialize");

    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "structuredContent": structured_content,
        "isError": true
    })
}

pub(crate) fn json_rpc_error_for_adapter(id: Value, error: McpAdapterError) -> Value {
    let (code, message) = match error {
        McpAdapterError::UnknownTool(_) | McpAdapterError::InvalidParams { .. } => {
            (-32602, "Invalid params")
        }
        McpAdapterError::Protocol(_)
        | McpAdapterError::Environment(_)
        | McpAdapterError::ToolExecution { .. } => (-32602, "Invalid params"),
        McpAdapterError::Core(_)
        | McpAdapterError::Json(_)
        | McpAdapterError::Io(_)
        | McpAdapterError::Store(_) => (-32603, "Internal error"),
    };
    json_rpc_error(id, code, message, Some(error.to_string()))
}

pub(crate) fn invalid_request(id: Value, data: impl Into<String>) -> JsonRpcFailure {
    JsonRpcFailure {
        id,
        code: -32600,
        message: "Invalid Request",
        data: Some(data.into()),
    }
}

pub(crate) fn invalid_request_response(id: &Value, data: impl Into<String>) -> Value {
    json_rpc_error(id.clone(), -32600, "Invalid Request", Some(data.into()))
}

pub(crate) fn invalid_params_response(id: &Value, data: impl Into<String>) -> Value {
    json_rpc_error(id.clone(), -32602, "Invalid params", Some(data.into()))
}

pub(crate) fn json_rpc_error(id: Value, code: i64, message: &str, data: Option<String>) -> Value {
    let mut error = json!({
        "code": code,
        "message": message
    });
    if let Some(data) = data {
        error["data"] = Value::String(data);
    }

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": error
    })
}

pub(crate) fn write_json_line(
    writer: &mut impl Write,
    value: Value,
) -> Result<(), McpAdapterError> {
    serde_json::to_writer(&mut *writer, &value).map_err(McpAdapterError::Json)?;
    writer.write_all(b"\n").map_err(McpAdapterError::Io)
}

#[cfg(test)]
mod mutation_output_tests {
    use super::*;
    use crate::tests::{
        exact_host_capability_evidence_artifact_sha256, exact_host_capability_input,
        exact_local_web_test_capabilities, publish_exact_host_capability_verification,
    };
    use std::cell::Cell;
    use std::io::{BufReader, Cursor};
    use volicord_store::agent_connections::HOST_KIND_CLAUDE_CODE;
    use volicord_store::evidence_capture::EvidenceCaptureReceiptInsert;
    use volicord_store::host_capabilities::MAX_HOST_CAPABILITY_VERIFICATION_TEXT_BYTES;
    use volicord_store::sqlite::{open_registry_database, registry_db_path};
    use volicord_test_support::core_fixtures::{
        CoreFixture, UpdateScopeFixture, UserActionFixture,
    };
    use volicord_types::{
        AcceptanceCriterionId, BaselineRef, ChangeUnitId, ChangeUnitOperation,
        EvidenceAssuranceLevel, EvidenceCaptureSpec, EvidenceObservationInput, EvidenceProducer,
        EvidenceSourceKind, EvidenceTarget, JudgmentKind, RecordId, StateRecordKind, UtcTimestamp,
        EVIDENCE_CAPTURE_COMMAND_LIMITATION, MAX_OPERATION_RESULT_PAGE_BYTES,
    };

    fn committed_intake_with_receipt(
        prefix: &str,
    ) -> Result<(CoreFixture, PipelineResponse, AuthorityReceipt), Box<dyn Error>> {
        let fixture = CoreFixture::new(prefix)?;
        let core = CoreService::new(fixture.runtime_home_path());
        let invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let committed = core.intake(
            fixture.intake_request(
                "req_mcp_recovery_order",
                "idem_mcp_recovery_order",
                false,
                Some(0),
            ),
            invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .as_ref()
            .expect("committed intake resolves a Task");
        let status = core.status(
            fixture.status_request("req_mcp_recovery_order_status", Some(task_id.as_str())),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::Read,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        let receipt = serde_json::from_value(status.response_value["authority_receipt"].clone())?;
        Ok((fixture, committed, receipt))
    }

    fn committed_record_run_with_capture_producer(
        prefix: &str,
    ) -> Result<
        (
            CoreFixture,
            PipelineResponse,
            PipelineResponse,
            StateRecordRef,
        ),
        Box<dyn Error>,
    > {
        let fixture = CoreFixture::new(prefix)?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workspace = GitWorkspaceContext {
            git_common_dir: fixture
                .product_repo_path()
                .join(".git")
                .to_string_lossy()
                .into_owned(),
            worktree_id: format!("sha256:{}", "1".repeat(64)),
            branch_ref: Some("refs/heads/mcp-producer-recovery".to_owned()),
            head_sha: Some("2".repeat(40)),
            workspace_fingerprint: format!("sha256:{}", "3".repeat(64)),
        };
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
            .with_git_workspace_context(workspace.clone())
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_producer_recovery_intake",
                "idem_mcp_producer_recovery_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("intake resolves a Task");
        let scope = core.update_scope(
            fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_mcp_producer_recovery_scope",
                idempotency_key: "idem_mcp_producer_recovery_scope",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                operation: ChangeUnitOperation::CreateCurrent,
                scope_summary: "Bind an actual evidence producer to compact recovery.",
            }),
            workflow_invocation(),
        )?;
        let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
            .as_str()
            .ok_or("scope should expose the current Change Unit")?;
        let criterion_id = scope.response_value["state"]["acceptance_criteria"][0]
            ["acceptance_criterion_id"]
            .as_str()
            .ok_or("scope should expose the current acceptance criterion")?;
        let target = EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id: AcceptanceCriterionId::new(criterion_id),
        };
        let prepared = core.prepare_evidence_capture(
            PrepareEvidenceCaptureRequest {
                envelope: fixture.envelope(
                    "req_mcp_producer_recovery_prepare",
                    Some("idem_mcp_producer_recovery_prepare"),
                    false,
                    Some(2),
                    Some(task_id.as_str()),
                ),
                task_id: task_id.clone(),
                change_unit_id: ChangeUnitId::new(change_unit_id),
                baseline_ref: BaselineRef::new(
                    volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                ),
                target: target.clone(),
                capture: EvidenceCaptureSpec::VerifiedCommandExecution {
                    command_sha256: "4".repeat(64),
                    command_label: "actual compact producer fixture".to_owned(),
                    expected_exit_code: RequiredNullable::null(),
                },
            },
            workflow_invocation(),
        )?;
        let capture_intent_ref: StateRecordRef =
            serde_json::from_value(prepared.response_value["capture_intent_ref"].clone())?;

        let mut store = fixture.store()?;
        let intent = store
            .evidence_capture_intent_record(capture_intent_ref.record_id.as_str())?
            .expect("committed capture intent should be readable");
        let observed_outcome = json!({
            "exit_code": 0,
            "stdout_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "stdout_size_bytes": 0,
            "stderr_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "stderr_size_bytes": 0
        });
        let result_sha256 = canonical_json_bare_sha256(&observed_outcome)?;
        let source = json!({
            "connection_id": fixture.connection_id(),
            "session_id": null,
            "guard_installation_id": null,
            "guard_event_ids": [],
            "watch_observation_refs": [],
            "host_invocation_id": "host_invocation_mcp_producer_recovery"
        });
        let expected_outcome: Value = serde_json::from_str(&intent.expected_outcome_json)?;
        let safe_receipt = json!({
            "schema_version": "volicord.evidence_capture_receipt.v1",
            "capture_kind": "verified_command_execution",
            "capture_intent_id": capture_intent_ref.record_id,
            "input_sha256": intent.input_sha256,
            "result_sha256": result_sha256,
            "expected_outcome": expected_outcome,
            "observed_outcome": observed_outcome,
            "source": source,
            "complete": true,
            "limitations": [EVIDENCE_CAPTURE_COMMAND_LIMITATION],
            "redaction_state": "redacted",
            "observed_by_actor_source": fixture.actor_source(),
            "observed_at": intent.created_at
        });
        store.fulfill_evidence_capture_source(EvidenceCaptureReceiptInsert {
            evidence_capture_receipt_id: "evidence_capture_receipt_mcp_producer_recovery"
                .to_owned(),
            evidence_capture_intent_id: capture_intent_ref.record_id.as_str().to_owned(),
            staging_handle_id: "staged_capture_receipt_mcp_producer_recovery".to_owned(),
            task_id: intent.task_id.clone(),
            capture_kind: intent.capture_kind.clone(),
            input_sha256: intent.input_sha256.clone(),
            result_sha256: result_sha256.clone(),
            expected_outcome_json: intent.expected_outcome_json.clone(),
            observed_outcome_json: serde_json::to_string(&observed_outcome)?,
            source_refs_json: "[]".to_owned(),
            observed_by_actor_source: fixture.actor_source(),
            observed_at: intent.created_at.clone(),
            limitations_json: serde_json::to_string(&json!([EVIDENCE_CAPTURE_COMMAND_LIMITATION]))?,
            safe_receipt_json: serde_json::to_string(&safe_receipt)?,
            created_at: intent.created_at.clone(),
            staging_expires_at: intent.expires_at.clone(),
            metadata_json: serde_json::to_string(&json!({ "source": source }))?,
        })?;
        drop(store);

        let mut record_request = fixture.record_run_request(
            "req_mcp_producer_recovery_record",
            "idem_mcp_producer_recovery_record",
            false,
            Some(3),
            task_id.as_str(),
            change_unit_id,
        );
        record_request.evidence_observations = vec![EvidenceObservationInput {
            target,
            source_kind: EvidenceSourceKind::ExternalTool,
            assurance_level: EvidenceAssuranceLevel::ExternalToolResult,
            observed_by_actor_source: RequiredNullable::null(),
            tool_name: RequiredNullable::null(),
            tool_invocation_id: RequiredNullable::null(),
            tool_metadata: Map::new(),
            input_refs: vec![capture_intent_ref.clone()],
            source_refs: Vec::new(),
            output_artifact_refs: Vec::new(),
            limitations: Vec::new(),
            observed_at: UtcTimestamp::parse("2000-01-01T00:00:00Z")?,
        }];
        let recorded = core.record_run(record_request, workflow_invocation())?;
        let producer: EvidenceProducer =
            serde_json::from_value(recorded.response_value["evidence_producers"][0].clone())?;
        let producer_id = producer.evidence_producer_id.as_str().to_owned();
        let producer_row = fixture
            .store()?
            .evidence_producer_record(&producer_id)?
            .expect("record_run producer should be immediately readable");
        assert_eq!(
            producer_row.evidence_capture_intent_id,
            capture_intent_ref.record_id.as_str()
        );
        let state_version = recorded.response_value["base"]["state_version"]
            .as_u64()
            .ok_or("record_run should expose its committed state version")?;
        let producer_ref = StateRecordRef {
            record_kind: StateRecordKind::EvidenceProducer,
            record_id: RecordId::new(producer_id),
            project_id: producer.project_id,
            task_id: Some(producer.task_id).into(),
            produced_at_state_version: Some(state_version).into(),
        };

        let refreshed = core.status(
            fixture.status_request("req_mcp_producer_recovery_status", Some(task_id.as_str())),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::Read,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        Ok((fixture, recorded, refreshed, producer_ref))
    }

    fn receipt_with_message_padding(
        receipt: &AuthorityReceipt,
        padding_bytes: usize,
    ) -> AuthorityReceipt {
        let mut value = serde_json::to_value(receipt).expect("receipt should serialize");
        value["close_blockers"][0]["message"] = Value::String("x".repeat(padding_bytes));
        serde_json::from_value(value).expect("padded receipt should remain valid")
    }

    fn recovery_facts() -> ToolDiagnosticFacts {
        ToolDiagnosticFacts {
            core_reached: true,
            core_committed: true,
            effect_kind: Some(EffectKind::CoreCommitted),
            effect_applied: true,
            effect_anchor: Some("authority_event:event_recovery_order".to_owned()),
            ..ToolDiagnosticFacts::default()
        }
    }

    fn recovery_operation_result_ref() -> OperationResultRef {
        OperationResultRef {
            project_id: ProjectId::new("project_mcp_recovery_order"),
            source_method: MethodName::Intake,
            source_idempotency_key: IdempotencyKey::new("idem_mcp_recovery_order"),
            committed_state_version: 1,
            response_sha256: format!("sha256:{}", "a".repeat(64)),
            response_size_bytes: 1_024,
        }
    }

    fn recovery_outcome(
        tool_name: &str,
        requested_detail: MutationDetailLevel,
        authority_receipt: Option<AuthorityReceipt>,
        exact_method_result: Option<Value>,
        compact_method_result: Option<Value>,
    ) -> CanonicalMcpMutationOutcome {
        CanonicalMcpMutationOutcome {
            tool_name: tool_name.to_owned(),
            requested_detail,
            facts: recovery_facts(),
            exact_method_result,
            compact_method_result,
            operation_result_ref: Some(recovery_operation_result_ref()),
            authority_receipt,
            next_actions: Vec::new(),
        }
    }

    fn assert_compact_budget(output: ToolCallOutput) -> Result<(), Box<dyn Error>> {
        assert!(!output.is_error);
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(
            output.structured_content["effect_anchor"],
            "authority_event:event_recovery_order"
        );
        assert_eq!(
            output.structured_content["operation_result_ref"],
            serde_json::to_value(recovery_operation_result_ref())?
        );
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            true
        );
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }

    #[test]
    fn operation_result_page_keeps_escape_heavy_chunk_out_of_bounded_compatibility_text(
    ) -> Result<(), Box<dyn Error>> {
        let chunk_utf8 = "\\\"".repeat(MAX_OPERATION_RESULT_PAGE_BYTES / 2);
        assert_eq!(chunk_utf8.len(), MAX_OPERATION_RESULT_PAGE_BYTES);
        let response_value = json!({
            "base": { "response_kind": "result" },
            "start_offset_bytes": 0,
            "end_offset_bytes": MAX_OPERATION_RESULT_PAGE_BYTES,
            "chunk_utf8": chunk_utf8,
            "complete": false
        });
        let response = PipelineResponse {
            response_json: serde_json::to_string(&response_value)?,
            response_value,
            operation_result_ref: None,
            verified_invocation: None,
            resolved_task_id: None,
            replayed: false,
        };

        let output = ToolCallOutput::from_operation_result_response(&response)?;

        assert!(output.primary_text.len() <= MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES);
        assert!(!output.primary_text.contains(&chunk_utf8));
        assert!(rendered_tool_call_output_size(&output)? <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
        Ok(())
    }

    #[test]
    fn idempotent_mutation_replay_default_summary_returns_refreshed_authority_receipt(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-mutation-replay-summary")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let request = fixture.intake_request(
            "req_mcp_mutation_replay_summary",
            "idem_mcp_mutation_replay_summary",
            false,
            Some(0),
        );
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };

        let committed = core.intake(request.clone(), workflow_invocation())?;
        assert!(!committed.replayed);
        let replayed = core.intake(request, workflow_invocation())?;
        assert!(replayed.replayed);
        let task_id = replayed
            .resolved_task_id
            .clone()
            .expect("replay preserves the resolved Task identity");

        let detail = mutation_detail_for_tool(INTAKE_TOOL_NAME, &json!({}));
        assert_eq!(detail, Some(MutationDetailLevel::Summary));
        let output = ToolCallOutput::from_pipeline_response(&replayed)?;
        let output =
            finalize_mutation_output_with_refresh(INTAKE_TOOL_NAME, detail, output, |context| {
                assert_eq!(context.project_id.as_str(), fixture.project_id());
                assert_eq!(context.task_id, task_id);
                core.status(
                    fixture.status_request(
                        "req_mcp_mutation_replay_summary_refresh",
                        Some(context.task_id.as_str()),
                    ),
                    InvocationContext::new(
                        context.project_id.clone(),
                        ActorSource::agent_connection(fixture.connection_id()),
                        OperationCategory::Read,
                        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
                    ),
                )
                .map_err(McpAdapterError::Core)
            })?;

        assert!(!output.is_error);
        assert!(output.diagnostic_facts.replayed);
        assert!(output.diagnostic_facts.core_reached);
        assert!(!output.diagnostic_facts.core_committed);
        assert_eq!(
            output.structured_content["authority_receipt"]["project_id"],
            fixture.project_id()
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["task_ref"]["record_id"],
            task_id.as_str()
        );
        assert!(output.structured_content["authority_receipt"]["state_version"].is_u64());
        assert_eq!(
            output.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert!(output.structured_content.get("code").is_none());
        assert!(output
            .structured_content
            .get("completion_claim_withheld")
            .is_none());
        Ok(())
    }

    #[test]
    fn full_projection_pairs_exact_method_result_with_newer_fresh_receipt(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-full-fresh-receipt")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_full_fresh_receipt",
                "idem_mcp_full_fresh_receipt",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .as_ref()
            .expect("intake should resolve a Task")
            .clone();
        let original_method_result = intake.response_value.clone();
        core.update_scope(
            fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_mcp_full_fresh_receipt_scope",
                idempotency_key: "idem_mcp_full_fresh_receipt_scope",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                operation: ChangeUnitOperation::KeepCurrent,
                scope_summary: "Advance authority after the original method result.",
            }),
            workflow_invocation(),
        )?;

        let output = finalize_mutation_output_with_refresh(
            INTAKE_TOOL_NAME,
            Some(MutationDetailLevel::Full),
            ToolCallOutput::from_pipeline_response(&intake)?,
            |context| {
                core.status(
                    fixture.status_request(
                        "req_mcp_full_fresh_receipt_status",
                        Some(context.task_id.as_str()),
                    ),
                    InvocationContext::new(
                        context.project_id.clone(),
                        ActorSource::agent_connection(fixture.connection_id()),
                        OperationCategory::Read,
                        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
                    ),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;

        assert!(!output.is_error);
        assert_eq!(
            output.structured_content["method_result"],
            original_method_result
        );
        assert_eq!(
            output.structured_content["method_result"]["base"]["state_version"],
            1
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["state_version"],
            2
        );
        Ok(())
    }

    #[test]
    fn refresh_failure_withholds_success_and_does_not_return_private_error_body() {
        let private_error = "private-refresh-owner-body-must-not-escape";
        let mut output = ToolCallOutput::success(
            json!({
                "base": {
                    "response_kind": "result",
                    "effect_kind": "core_committed"
                }
            })
            .to_string(),
        )
        .expect("tool output");
        output.diagnostic_facts.core_reached = true;
        output.diagnostic_facts.core_committed = true;
        output.diagnostic_facts.effect_kind = Some(EffectKind::CoreCommitted);
        output.diagnostic_facts.effect_applied = true;
        output.diagnostic_facts.effect_anchor =
            Some("authority_event:event_refresh_failure".to_owned());
        output.mutation_refresh_context = Some(MutationRefreshContext {
            project_id: ProjectId::new("project_refresh_failure"),
            task_id: TaskId::new("task_refresh_failure"),
        });

        let output = finalize_mutation_output_with_refresh(
            INTAKE_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |_| Err(McpAdapterError::Environment(private_error.to_owned())),
        )
        .expect("fail-closed output");

        assert!(!output.is_error);
        assert_eq!(output.structured_content["code"], "MCP_UNAVAILABLE");
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["reached_core"], true);
        assert_eq!(output.structured_content["committed"], true);
        assert_eq!(output.structured_content["effect_kind"], "core_committed");
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(
            output.structured_content["effect_anchor"],
            "authority_event:event_refresh_failure"
        );
        assert_eq!(output.structured_content["status_read_required"], true);
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        assert!(output.diagnostic_facts.authoritative_refresh_failure);
        let rendered =
            serde_json::to_string(&tool_call_result_from_output(output)).expect("rendered result");
        assert!(!rendered.contains(private_error));
        assert!(!rendered.contains("response_kind\":\"result"));
    }

    #[test]
    fn refresh_freshness_mismatch_uses_same_non_retryable_failure_boundary(
    ) -> Result<(), Box<dyn Error>> {
        let (fixture, committed, _) =
            committed_intake_with_receipt("mcp-refresh-freshness-mismatch")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let expected_compact =
            compact_mutation_method_result(INTAKE_TOOL_NAME, &committed.response_value)?;
        let mut mismatched_refresh = core.status(
            fixture.status_request(
                "req_mcp_refresh_freshness_mismatch_status",
                Some(task_id.as_str()),
            ),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::Read,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        mismatched_refresh.response_value["authority_receipt"]["state_version"] = json!(999);

        let output = finalize_mutation_output_with_refresh(
            INTAKE_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            ToolCallOutput::from_pipeline_response(&committed)?,
            |_| Ok(mismatched_refresh),
        )?;

        assert!(!output.is_error);
        assert_eq!(output.structured_content["code"], "MCP_UNAVAILABLE");
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["reached_core"], true);
        assert_eq!(output.structured_content["committed"], true);
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(output.structured_content["method_result"], expected_compact);
        assert_eq!(output.structured_content["status_read_required"], true);
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        assert!(output.diagnostic_facts.authoritative_refresh_failure);
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }

    #[test]
    fn post_effect_adapter_failure_refreshes_authority_without_recommending_replay(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-post-effect-adapter-failure")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let committed = core.intake(
            fixture.intake_request(
                "req_mcp_post_effect_adapter_failure",
                "idem_mcp_post_effect_adapter_failure",
                false,
                Some(0),
            ),
            invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let refreshed = core.status(
            fixture.status_request(
                "req_mcp_post_effect_adapter_failure_status",
                Some(task_id.as_str()),
            ),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::Read,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        let expected_compact =
            compact_mutation_method_result(INTAKE_TOOL_NAME, &committed.response_value)?;
        let mut output = ToolCallOutput::from_pipeline_response(&committed)?;
        output.structured_content["adapter_test_padding"] =
            Value::String("x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES));
        let output =
            output.with_post_effect_failure(McpPostEffectFailureCode::McpPostEffectAdapterFailed);
        let output = finalize_mutation_output_with_refresh(
            INTAKE_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |_| Ok(refreshed),
        )?;

        assert!(!output.is_error);
        assert_eq!(
            output.structured_content["code"],
            "MCP_POST_EFFECT_ADAPTER_FAILED"
        );
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["reached_core"], true);
        assert_eq!(output.structured_content["committed"], true);
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(output.structured_content["method_result"], expected_compact);
        assert_eq!(
            output.structured_content["authority_receipt"]["task_ref"]["record_id"],
            task_id.as_str()
        );
        assert_eq!(
            output.structured_content["authoritative_refresh_succeeded"],
            true
        );
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            true
        );
        assert_eq!(output.structured_content["status_read_required"], true);
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        Ok(())
    }

    #[test]
    fn local_web_handoff_budget_accepts_exact_limit_and_rejects_next_byte_without_orphan(
    ) -> Result<(), Box<dyn Error>> {
        for (detail_case, detail, response_budget) in [
            (
                "summary",
                MutationDetailLevel::Summary,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            ),
            (
                "full",
                MutationDetailLevel::Full,
                MAX_MCP_FULL_MUTATION_RESULT_BYTES,
            ),
        ] {
            for (edge_case, extra_byte, expected_handoff) in
                [("exact", 0_usize, true), ("one-over", 1_usize, false)]
            {
                let case = format!("{detail_case}-{edge_case}");
                let fixture = CoreFixture::new_with_host_kind(
                    &format!("mcp-local-web-budget-{case}"),
                    HOST_KIND_CLAUDE_CODE,
                )?;
                let core = CoreService::new(fixture.runtime_home_path());
                let invocation = || {
                    InvocationContext::new(
                        ProjectId::new(fixture.project_id()),
                        ActorSource::agent_connection(fixture.connection_id()),
                        OperationCategory::AgentWorkflow,
                        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
                    )
                };
                let intake = core.intake(
                    fixture.intake_request(
                        &format!("req_mcp_local_web_budget_{case}_intake"),
                        &format!("idem_mcp_local_web_budget_{case}_intake"),
                        false,
                        Some(0),
                    ),
                    invocation(),
                )?;
                let task_id = intake.resolved_task_id.expect("intake resolves a Task");
                let pending = core.request_user_action(
                    fixture.user_action_request(UserActionFixture {
                        request_id: &format!("req_mcp_local_web_budget_{case}_pending"),
                        idempotency_key: &format!("idem_mcp_local_web_budget_{case}_pending"),
                        dry_run: false,
                        expected_state_version: Some(1),
                        task_id: task_id.as_str(),
                        change_unit_id: None,
                        judgment_kind: JudgmentKind::ProductDecision,
                    }),
                    invocation(),
                )?;
                let request_id = pending.response_value["user_action_request_summary"]
                    ["user_action_request_id"]
                    .as_str()
                    .expect("result identifies the pending request");
                let context = McpConnectionContext::resolve(
                    fixture.runtime_home_path(),
                    fixture.connection_id(),
                )?;
                let base_url = "http://127.0.0.1:39000";
                let readiness = LocalWebConsentReadiness::ready_for_test();
                let creator_readiness = readiness.clone();
                let adapter = McpAdapter::new(fixture.runtime_home_path(), context)
                    .with_local_web_consent_readiness(
                        LocalWebConsentContext {
                            base_url: base_url.to_owned(),
                        },
                        readiness,
                    );
                let before: (i64, String) = fixture.conn()?.query_row(
                "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at FROM project_state",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
                let mut output = ToolCallOutput::success("{}".to_owned())?;
                output.primary_text.clear();
                output.deferred_local_web_handoff = Some(DeferredLocalWebHandoff {
                    project_id: ProjectId::new(fixture.project_id()),
                    user_action_request_id: UserActionRequestId::new(request_id),
                    form_digest: format!("sha256:{}", "0".repeat(64)),
                });
                let fixed_length_url = format!(
                    "{base_url}{LOCAL_WEB_CONSENT_PATH}?project={}&token={}",
                    percent_encode_query(fixture.project_id()),
                    "0".repeat(64)
                );
                output.host_meta = Some(local_web_host_meta(
                    &fixed_length_url,
                    "9999-12-31T23:59:59.999999999Z",
                ));
                let fixed_bytes = rendered_tool_call_output_size(&output)?;
                assert!(fixed_bytes <= response_budget);
                output.host_meta = None;
                output.primary_text = "x".repeat(response_budget - fixed_bytes + extra_byte);

                let token_creator_called = Cell::new(false);
                let verification_label = format!("budget_{case}");
                publish_exact_host_capability_verification(&fixture, &verification_label)?;
                let adapter = adapter.with_expected_evidence_artifact_sha256_for_test(
                    exact_host_capability_evidence_artifact_sha256(&verification_label),
                );
                let capabilities = exact_local_web_test_capabilities(&fixture)?;
                let output = materialize_local_web_handoff_with_token_creator(
                    &adapter,
                    Some(detail),
                    &capabilities,
                    output,
                    |runtime_home, input| {
                        token_creator_called.set(true);
                        assert!(
                            creator_readiness.issuance_lease_is_held_for_test(),
                            "token creator must run while the readiness lease is held"
                        );
                        create_user_action_channel_token(runtime_home, input)
                    },
                )?;
                let after: (i64, String) = fixture.conn()?.query_row(
                "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at FROM project_state",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

                assert!(output.deferred_local_web_handoff.is_none());
                assert_eq!(output.host_meta.is_some(), expected_handoff, "{case}");
                assert_eq!(token_creator_called.get(), expected_handoff, "{case}");
                assert_eq!(
                    output.diagnostic_facts.fallback_kind,
                    Some(if expected_handoff {
                        DiagnosticFallbackKind::LocalWebConsent
                    } else {
                        DiagnosticFallbackKind::CliInbox
                    }),
                    "{case}"
                );
                if expected_handoff {
                    assert_eq!(after.0, before.0 + 1, "{case}");
                    assert!(
                        rendered_tool_call_output_size(&output)? <= response_budget,
                        "{case}"
                    );
                } else {
                    assert_eq!(
                        after, before,
                        "one-byte-over fallback must have no token or clock effect"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn degraded_local_web_listener_before_materialization_creates_no_handoff_or_token(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new_with_host_kind(
            "mcp-local-web-degraded-before-materialization",
            HOST_KIND_CLAUDE_CODE,
        )?;
        let core = CoreService::new(fixture.runtime_home_path());
        let session_id = "session_local_web_degraded_before_materialization";
        let invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
            )
            .with_session_id(session_id)
            .with_local_web_consent_available(true)
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_local_web_degraded_intake",
                "idem_mcp_local_web_degraded_intake",
                false,
                Some(0),
            ),
            invocation(),
        )?;
        let task_id = intake.resolved_task_id.expect("intake resolves a Task");
        let pending = core.request_user_action(
            fixture.user_action_request(UserActionFixture {
                request_id: "req_mcp_local_web_degraded_pending",
                idempotency_key: "idem_mcp_local_web_degraded_pending",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                change_unit_id: None,
                judgment_kind: JudgmentKind::ProductDecision,
            }),
            invocation(),
        )?;
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
                .with_invocation_binding_basis(VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING);
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context)
            .with_local_web_consent_readiness(
                LocalWebConsentContext {
                    base_url: "http://127.0.0.1:39000".to_owned(),
                },
                LocalWebConsentReadiness::ready_for_test(),
            );
        let mut input_lines = BufReader::new(Cursor::new(Vec::<u8>::new())).lines();
        let mut wire_output = Vec::new();
        let mut request_sequence = 1;
        publish_exact_host_capability_verification(&fixture, "listener_degraded")?;
        let adapter = adapter.with_expected_evidence_artifact_sha256_for_test(
            exact_host_capability_evidence_artifact_sha256("listener_degraded"),
        );
        let capabilities = exact_local_web_test_capabilities(&fixture)?;
        let output = user_action_tool_output(
            &adapter,
            pending,
            true,
            capabilities.clone(),
            &mut request_sequence,
            &mut input_lines,
            &mut wire_output,
        )?;
        assert!(wire_output.is_empty());
        assert!(output.deferred_local_web_handoff.is_some());
        assert!(output.host_meta.is_none());
        let before: (i64, String) = fixture.conn()?.query_row(
            "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at FROM project_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        adapter
            .local_web_consent_readiness
            .as_ref()
            .expect("fixture listener readiness")
            .mark_unavailable();

        let output = materialize_local_web_handoff(
            &adapter,
            Some(MutationDetailLevel::Summary),
            &capabilities,
            output,
        )?;

        assert!(output.host_meta.is_none());
        assert!(output.deferred_local_web_handoff.is_none());
        assert_eq!(
            output.diagnostic_facts.fallback_kind,
            Some(DiagnosticFallbackKind::CliInbox)
        );
        let result = tool_call_result_from_output(output.clone());
        let model_visible = json!({
            "content": result["content"].clone(),
            "structuredContent": result["structuredContent"].clone(),
        });
        assert!(result.get("_meta").is_none());
        assert!(result["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry["text"].as_str())
            .any(|text| text.contains("`volicord inbox`")));
        let visible_text = serde_json::to_string(&model_visible)?;
        assert!(!visible_text.contains("http://127.0.0.1:39000"));
        assert!(!visible_text.contains("token="));
        let after: (i64, String) = fixture.conn()?.query_row(
            "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at FROM project_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(
            after, before,
            "degraded listener fallback must have no token or clock effect"
        );
        Ok(())
    }

    #[test]
    fn final_materialization_rechecks_current_verification_after_deferred_selection(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new_with_host_kind(
            "mcp-local-web-final-verification-recheck",
            HOST_KIND_CLAUDE_CODE,
        )?;
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context)
            .with_local_web_consent_readiness(
                LocalWebConsentContext {
                    base_url: "http://127.0.0.1:39000".to_owned(),
                },
                LocalWebConsentReadiness::ready_for_test(),
            )
            .with_expected_evidence_artifact_sha256_for_test(
                exact_host_capability_evidence_artifact_sha256("final_recheck_pass"),
            );
        publish_exact_host_capability_verification(&fixture, "final_recheck_pass")?;
        let capabilities = exact_local_web_test_capabilities(&fixture)?;
        assert!(adapter.effective_local_web_consent_available(&capabilities));
        let mut output = ToolCallOutput::success("{}".to_owned())?;
        output.deferred_local_web_handoff = Some(DeferredLocalWebHandoff {
            project_id: ProjectId::new(fixture.project_id()),
            user_action_request_id: UserActionRequestId::new("uar_final_recheck"),
            form_digest: format!("sha256:{}", "0".repeat(64)),
        });
        let before: (i64, String) = fixture.conn()?.query_row(
            "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at FROM project_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let now = chrono::DateTime::<chrono::Utc>::from(SystemTime::now());
        let mut failed = exact_host_capability_input(
            &fixture,
            "final_recheck_failed",
            now,
            now + chrono::Duration::hours(1),
        )?;
        failed.outcome =
            volicord_store::host_capabilities::HOST_CAPABILITY_OUTCOME_FAILED.to_owned();
        volicord_store::host_capabilities::publish_host_capability_verification(
            fixture.runtime_home_path(),
            failed,
        )?;

        let output = materialize_local_web_handoff(
            &adapter,
            Some(MutationDetailLevel::Summary),
            &capabilities,
            output,
        )?;

        assert!(output.host_meta.is_none());
        assert!(output.deferred_local_web_handoff.is_none());
        assert_eq!(
            output.diagnostic_facts.fallback_kind,
            Some(DiagnosticFallbackKind::CliInbox)
        );
        let after: (i64, String) = fixture.conn()?.query_row(
            "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at FROM project_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(after, before);
        Ok(())
    }

    #[test]
    fn overbound_current_host_capability_fails_closed_without_local_web_effects(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new_with_host_kind(
            "mcp-local-web-overbound-current-verification",
            HOST_KIND_CLAUDE_CODE,
        )?;
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
        let verification_label = "overbound_current_verification";
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context)
            .with_local_web_consent_readiness(
                LocalWebConsentContext {
                    base_url: "http://127.0.0.1:39000".to_owned(),
                },
                LocalWebConsentReadiness::ready_for_test(),
            )
            .with_expected_evidence_artifact_sha256_for_test(
                exact_host_capability_evidence_artifact_sha256(verification_label),
            );
        publish_exact_host_capability_verification(&fixture, verification_label)?;
        let capabilities = exact_local_web_test_capabilities(&fixture)?;
        assert!(adapter.effective_local_web_consent_available(&capabilities));

        let output = ToolCallOutput::success("{}".to_owned())?.with_user_action_fallback(
            UserActionFallback {
                texts: vec![generic_user_channel_fallback_text()],
                kind: DiagnosticFallbackKind::CliInbox,
                deferred_local_web_handoff: Some(DeferredLocalWebHandoff {
                    project_id: ProjectId::new(fixture.project_id()),
                    user_action_request_id: UserActionRequestId::new("uar_overbound_current"),
                    form_digest: format!("sha256:{}", "0".repeat(64)),
                }),
            },
        );
        let before: (i64, String) = fixture.conn()?.query_row(
            "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at FROM project_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(before.0, 0);
        let private_marker = "private-overbound-verification-id";
        let corrupt_verification_internal_id = format!(
            "{private_marker}{}",
            "가".repeat(MAX_HOST_CAPABILITY_VERIFICATION_TEXT_BYTES)
        );
        assert!(
            corrupt_verification_internal_id.len() > MAX_HOST_CAPABILITY_VERIFICATION_TEXT_BYTES
        );
        let verification_internal_id = format!("hcv_{verification_label}");
        let registry = open_registry_database(registry_db_path(fixture.runtime_home_path()))?;
        registry.pragma_update(None, "foreign_keys", "OFF")?;
        registry.pragma_update(None, "ignore_check_constraints", "ON")?;
        let corrupted = registry.execute(
            "UPDATE host_capability_verifications
                SET verification_internal_id = ?1
              WHERE verification_internal_id = ?2",
            [
                corrupt_verification_internal_id.as_str(),
                verification_internal_id.as_str(),
            ],
        )?;
        assert_eq!(corrupted, 1);
        let pointer_corrupted = registry.execute(
            "UPDATE host_capability_state
                SET current_verification_internal_id = ?1
              WHERE current_verification_internal_id = ?2",
            [
                corrupt_verification_internal_id.as_str(),
                verification_internal_id.as_str(),
            ],
        )?;
        assert_eq!(pointer_corrupted, 1);
        drop(registry);

        assert!(!adapter.effective_local_web_consent_available(&capabilities));
        let output = materialize_local_web_handoff(
            &adapter,
            Some(MutationDetailLevel::Summary),
            &capabilities,
            output,
        )?;

        assert!(output.host_meta.is_none());
        assert!(output.deferred_local_web_handoff.is_none());
        assert_eq!(
            output.diagnostic_facts.fallback_kind,
            Some(DiagnosticFallbackKind::CliInbox)
        );
        let result = tool_call_result_from_output(output);
        assert!(result.get("_meta").is_none());
        assert!(result["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry["text"].as_str())
            .any(|text| text.contains("`volicord inbox`")));
        let result_text = serde_json::to_string(&result)?;
        assert!(!result_text.contains(private_marker));
        assert!(!result_text.contains("http://127.0.0.1:39000"));
        assert!(!result_text.contains("token="));
        let after: (i64, String) = fixture.conn()?.query_row(
            "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at FROM project_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(
            after, before,
            "corrupt host-capability fallback must have no token or clock effect"
        );
        Ok(())
    }

    #[test]
    fn idempotent_create_replay_never_reissues_a_local_web_handoff() -> Result<(), Box<dyn Error>> {
        let fixture =
            CoreFixture::new_with_host_kind("mcp-local-web-create-replay", HOST_KIND_CLAUDE_CODE)?;
        let core = CoreService::new(fixture.runtime_home_path());
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_local_web_replay_intake",
                "idem_mcp_local_web_replay_intake",
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
        let task_id = intake.resolved_task_id.expect("intake resolves a Task");
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
                .with_invocation_binding_basis(VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING);
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context)
            .with_local_web_consent_readiness(
                LocalWebConsentContext {
                    base_url: "http://127.0.0.1:39000".to_owned(),
                },
                LocalWebConsentReadiness::ready_for_test(),
            );
        let session_id = "session_mcp_local_web_create_replay";
        publish_exact_host_capability_verification(&fixture, "create_replay")?;
        let adapter = adapter.with_expected_evidence_artifact_sha256_for_test(
            exact_host_capability_evidence_artifact_sha256("create_replay"),
        );
        let capabilities = exact_local_web_test_capabilities(&fixture)?;
        adapter.call_tool_for_session_with_user_channel_capabilities(
            STATUS_TOOL_NAME,
            json!({"task_id": task_id.as_str()}),
            Some(session_id),
            capabilities.clone(),
        )?;
        let invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
            )
            .with_session_id(session_id)
            .with_local_web_consent_available(true)
        };
        let request = fixture.user_action_request(UserActionFixture {
            request_id: "req_mcp_local_web_create_replay",
            idempotency_key: "idem_mcp_local_web_create_replay",
            dry_run: false,
            expected_state_version: Some(1),
            task_id: task_id.as_str(),
            change_unit_id: None,
            judgment_kind: JudgmentKind::ProductDecision,
        });
        let created = core.request_user_action(request.clone(), invocation())?;
        assert!(!created.replayed);
        let created_operation_result_ref = created.operation_result_ref.clone();
        let mut lines = io::BufReader::new(io::Cursor::new(Vec::<u8>::new())).lines();
        let mut writer = Vec::new();
        let mut request_sequence = 1;
        let created_output = user_action_tool_output(
            &adapter,
            created,
            true,
            capabilities.clone(),
            &mut request_sequence,
            &mut lines,
            &mut writer,
        )?;
        let created_output = materialize_local_web_handoff(
            &adapter,
            Some(MutationDetailLevel::Summary),
            &capabilities,
            created_output,
        )?;
        assert!(created_output.host_meta.is_some());
        let exact_origin = created_output.structured_content["agent_workflow_result"].clone();
        let after_create_counts = fixture.counts()?;
        let after_create_storage: (i64, String) = fixture.conn()?.query_row(
            "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at FROM project_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(after_create_storage.0, 1);

        let replayed = core.request_user_action(request, invocation())?;
        assert!(replayed.replayed);
        assert_eq!(replayed.operation_result_ref, created_operation_result_ref);
        let mut lines = io::BufReader::new(io::Cursor::new(Vec::<u8>::new())).lines();
        let mut writer = Vec::new();
        let mut request_sequence = 1;
        let replayed_output = user_action_tool_output(
            &adapter,
            replayed,
            true,
            capabilities.clone(),
            &mut request_sequence,
            &mut lines,
            &mut writer,
        )?;
        let replayed_output = materialize_local_web_handoff(
            &adapter,
            Some(MutationDetailLevel::Summary),
            &capabilities,
            replayed_output,
        )?;
        assert!(replayed_output.host_meta.is_none());
        assert!(replayed_output.deferred_local_web_handoff.is_none());
        assert_eq!(
            replayed_output.structured_content["agent_workflow_result_replayed"],
            true
        );
        assert_eq!(
            replayed_output.structured_content["agent_workflow_result"],
            exact_origin
        );
        assert_eq!(fixture.counts()?, after_create_counts);
        let after_replay_storage: (i64, String) = fixture.conn()?.query_row(
            "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at FROM project_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(after_replay_storage, after_create_storage);
        Ok(())
    }

    #[test]
    fn superseded_user_action_suppresses_elicitation_and_preserves_the_origin_effect(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-elicited-record-race")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_elicited_record_race_intake",
                "idem_mcp_elicited_record_race_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("intake resolves a Task");
        let pending_response = core.request_user_action(
            fixture.user_action_request(UserActionFixture {
                request_id: "req_mcp_elicited_record_race_pending",
                idempotency_key: "idem_mcp_elicited_record_race_pending",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                change_unit_id: None,
                judgment_kind: JudgmentKind::ProductDecision,
            }),
            workflow_invocation(),
        )?;
        core.update_scope(
            fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_mcp_elicited_record_race_scope",
                idempotency_key: "idem_mcp_elicited_record_race_scope",
                dry_run: false,
                expected_state_version: Some(2),
                task_id: task_id.as_str(),
                operation: ChangeUnitOperation::KeepCurrent,
                scope_summary: "Advance state while host elicitation remains open.",
            }),
            workflow_invocation(),
        )?;
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context);

        let mut lines = io::BufReader::new(io::Cursor::new(Vec::<u8>::new())).lines();
        let mut elicitation_request = Vec::new();
        let mut request_sequence = 1;
        let output = user_action_tool_output(
            &adapter,
            pending_response,
            true,
            McpUserChannelCapabilities::new(true, false),
            &mut request_sequence,
            &mut lines,
            &mut elicitation_request,
        )?;
        assert_eq!(output.post_effect_failure, None);
        assert_eq!(
            output.structured_content["agent_workflow_result"]["user_action_request_summary"]
                ["status"],
            "pending"
        );
        assert_eq!(output.structured_content["current_status"], "superseded");
        assert!(output.structured_content["user_channel_resolution"].is_null());
        assert_eq!(output.structured_content["derived_refs"], json!([]));
        assert!(elicitation_request.is_empty());
        let output = finalize_mutation_output_with_refresh(
            REQUEST_USER_ACTION_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |context| {
                core.status(
                    fixture.status_request(
                        "req_mcp_elicited_record_race_status",
                        Some(context.task_id.as_str()),
                    ),
                    InvocationContext::new(
                        context.project_id.clone(),
                        ActorSource::agent_connection(fixture.connection_id()),
                        OperationCategory::Read,
                        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
                    ),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;

        assert!(!output.is_error);
        assert!(output.structured_content.get("code").is_none());
        assert_eq!(
            output.structured_content["operation_result_ref"]["source_method"],
            REQUEST_USER_ACTION_TOOL_NAME
        );
        assert_eq!(
            output.structured_content["method_result"]["status"],
            "superseded"
        );
        assert_eq!(
            output.structured_content["method_result"]["agent_workflow_result_replayed"],
            false
        );
        assert_eq!(
            output.structured_content["method_result"]["derived_refs"],
            json!([])
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["state_version"],
            3
        );
        assert!(output
            .structured_content
            .get("completion_claim_withheld")
            .is_none());
        Ok(())
    }

    #[test]
    fn mismatched_safe_summary_routes_to_closed_post_effect_recovery() -> Result<(), Box<dyn Error>>
    {
        let fixture = CoreFixture::new("mcp-noncanonical-pending-post-effect")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_noncanonical_pending_intake",
                "idem_mcp_noncanonical_pending_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let mut pending_response = core.request_user_action(
            fixture.user_action_request(UserActionFixture {
                request_id: "req_mcp_noncanonical_pending_action",
                idempotency_key: "idem_mcp_noncanonical_pending_action",
                dry_run: false,
                expected_state_version: Some(1),
                task_id: task_id.as_str(),
                change_unit_id: None,
                judgment_kind: JudgmentKind::ProductDecision,
            }),
            workflow_invocation(),
        )?;
        let before = fixture.counts()?;
        pending_response.response_value["user_action_request_summary"]["user_action_request_id"] =
            json!("uar_not_in_trusted_projection");
        let context =
            McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
        let adapter = McpAdapter::new(fixture.runtime_home_path(), context);
        let mut lines = io::BufReader::new(io::Cursor::new(Vec::<u8>::new())).lines();
        let mut elicitation_request = Vec::new();
        let mut request_sequence = 1;

        let error = user_action_tool_output(
            &adapter,
            pending_response.clone(),
            true,
            McpUserChannelCapabilities::new(true, false),
            &mut request_sequence,
            &mut lines,
            &mut elicitation_request,
        )
        .expect_err("mismatched public summary must fail before User Channel projection");
        assert!(matches!(error, McpAdapterError::Protocol(_)));
        assert!(elicitation_request.is_empty());
        assert_eq!(request_sequence, 1);
        assert_eq!(fixture.counts()?, before);

        let output = ToolCallOutput::from_pipeline_response(&pending_response)?
            .with_post_effect_failure(McpPostEffectFailureCode::McpPostEffectAdapterFailed);
        let output = finalize_mutation_output_with_refresh(
            REQUEST_USER_ACTION_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |context| {
                core.status(
                    fixture.status_request(
                        "req_mcp_noncanonical_pending_status",
                        Some(context.task_id.as_str()),
                    ),
                    InvocationContext::new(
                        context.project_id.clone(),
                        ActorSource::agent_connection(fixture.connection_id()),
                        OperationCategory::Read,
                        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
                    ),
                )
                .map_err(McpAdapterError::Core)
            },
        )?;
        let recovery: McpMutationPostEffectFailure =
            serde_json::from_value(output.structured_content.clone())?;
        assert_eq!(
            recovery.code,
            McpPostEffectFailureCode::McpPostEffectAdapterFailed
        );
        assert!(!recovery.retryable);
        assert!(recovery.reached_core);
        assert!(recovery.committed);
        assert!(recovery.effect_applied);
        assert!(recovery.response_projection_omitted);
        assert!(recovery.status_read_required);
        assert!(recovery.completion_claim_withheld);
        assert_eq!(
            output.structured_content["method_result"]["user_action_request_summary"]["status"],
            "pending"
        );
        assert!(output
            .structured_content
            .get("agent_workflow_result")
            .is_none());
        assert_eq!(fixture.counts()?, before);
        Ok(())
    }

    #[test]
    fn projection_failure_preserves_effect_facts_and_exact_method_result(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-post-effect-projection-failure")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let committed = core.intake(
            fixture.intake_request(
                "req_mcp_post_effect_projection_failure",
                "idem_mcp_post_effect_projection_failure",
                false,
                Some(0),
            ),
            invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves a Task");
        let refreshed = core.status(
            fixture.status_request(
                "req_mcp_post_effect_projection_failure_status",
                Some(task_id.as_str()),
            ),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::Read,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        let mut output = ToolCallOutput::from_pipeline_response(&committed)?;
        output.structured_content["base"]["effect_kind"] = json!("invalid_projection_fixture");
        let exact_unprojectable_result = output.structured_content.clone();
        let output = finalize_mutation_output_with_refresh(
            INTAKE_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |_| Ok(refreshed),
        )?;

        assert!(!output.is_error);
        assert_eq!(
            output.structured_content["code"],
            "MCP_RESPONSE_PROJECTION_FAILED"
        );
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["effect_kind"], "core_committed");
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(
            output.structured_content["method_result"],
            exact_unprojectable_result
        );
        assert_eq!(
            output.structured_content["authority_receipt"]["task_ref"]["record_id"],
            task_id.as_str()
        );
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            true
        );
        assert_eq!(output.structured_content["completion_claim_withheld"], true);
        Ok(())
    }

    #[test]
    fn staging_refresh_failure_reports_applied_handle_as_non_retryable_recovery(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-staging-refresh-failure")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_staging_refresh_failure",
                "idem_mcp_staging_refresh_failure",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .as_ref()
            .expect("intake should resolve a Task")
            .clone();
        let state_version = intake.response_value["base"]["state_version"]
            .as_u64()
            .expect("intake should report state version");
        let staged = core.stage_artifact(
            fixture.stage_artifact_request(
                "req_mcp_staging_refresh_failure_stage",
                None,
                false,
                Some(state_version),
                task_id.as_str(),
            ),
            workflow_invocation(),
        )?;
        let handle_id = staged.response_value["staged_artifact_handle"]["handle_id"]
            .as_str()
            .expect("stage result should include a handle")
            .to_owned();
        let output = ToolCallOutput::from_pipeline_response(&staged)?;
        assert_eq!(
            output.diagnostic_facts.effect_kind,
            Some(EffectKind::StagingCreated)
        );
        assert!(output.diagnostic_facts.effect_applied);
        assert!(!output.diagnostic_facts.core_committed);
        let effect_anchor = format!("staged_artifact:{handle_id}");
        assert_eq!(
            output.diagnostic_facts.effect_anchor.as_deref(),
            Some(effect_anchor.as_str())
        );

        let output = finalize_mutation_output_with_refresh(
            STAGE_ARTIFACT_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |_| {
                Err(McpAdapterError::Environment(
                    "refresh unavailable".to_owned(),
                ))
            },
        )?;

        assert!(!output.is_error);
        assert_eq!(output.structured_content["retryable"], false);
        assert_eq!(output.structured_content["committed"], false);
        assert_eq!(output.structured_content["effect_kind"], "staging_created");
        assert_eq!(output.structured_content["effect_applied"], true);
        assert_eq!(output.structured_content["effect_anchor"], effect_anchor);
        assert_eq!(
            output.structured_content["method_result"]["staged_artifact_handle"]["handle_id"],
            handle_id
        );
        assert!(output.structured_content["method_result"]["expires_at"].is_string());
        assert_eq!(output.structured_content["status_read_required"], true);
        Ok(())
    }

    #[test]
    fn oversized_valid_projection_preserves_effect_and_refresh_truth_within_each_budget(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-mutation-oversized-fresh-receipt")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let committed = core.intake(
            fixture.intake_request(
                "req_mcp_mutation_oversized_fresh_receipt",
                "idem_mcp_mutation_oversized_fresh_receipt",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = committed
            .resolved_task_id
            .clone()
            .expect("committed intake resolves the Task");
        let mut refreshed = core.status(
            fixture.status_request(
                "req_mcp_mutation_oversized_fresh_receipt_status",
                Some(task_id.as_str()),
            ),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::Read,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        let mut blocker = refreshed.response_value["authority_receipt"]["close_blockers"]
            .as_array()
            .and_then(|blockers| blockers.first())
            .cloned()
            .expect("fresh intake status should expose a close blocker");
        let omitted_marker = "oversized-valid-criterion-blocker-must-not-escape";
        blocker["message"] = Value::String(format!(
            "{omitted_marker}{}",
            "x".repeat(MAX_MCP_FULL_MUTATION_RESULT_BYTES * 2)
        ));
        let oversized_blockers = Value::Array(vec![blocker]);
        refreshed.response_value["authority_receipt"]["close_blockers"] =
            oversized_blockers.clone();
        refreshed.response_value["active_task"]["close_blockers"] = oversized_blockers.clone();
        refreshed.response_value["close_blockers"] = oversized_blockers;
        refreshed.response_json = serde_json::to_string(&refreshed.response_value)?;

        for detail in [
            MutationDetailLevel::Summary,
            MutationDetailLevel::Workflow,
            MutationDetailLevel::Full,
        ] {
            let output = ToolCallOutput::from_pipeline_response(&committed)?;
            let refreshed = refreshed.clone();
            let output = finalize_mutation_output_with_refresh(
                INTAKE_TOOL_NAME,
                Some(detail),
                output,
                |_| Ok(refreshed),
            )?;

            assert!(!output.is_error);
            assert_eq!(
                output.structured_content["code"],
                "MCP_RESPONSE_BUDGET_EXCEEDED"
            );
            assert_eq!(
                output.structured_content["requested_detail"],
                serde_json::to_value(detail)?
            );
            assert_eq!(output.structured_content["retryable"], false);
            assert_eq!(output.structured_content["reached_core"], true);
            assert_eq!(output.structured_content["committed"], true);
            assert_eq!(output.structured_content["effect_kind"], "core_committed");
            assert_eq!(output.structured_content["effect_applied"], true);
            assert!(output.structured_content["effect_anchor"]
                .as_str()
                .is_some_and(|token| token.starts_with("authority_event:")));
            assert!(output.structured_content["operation_result_ref"].is_object());
            assert_eq!(
                output.structured_content["authoritative_refresh_succeeded"],
                true
            );
            assert_eq!(
                output.structured_content["response_projection_omitted"],
                true
            );
            assert_eq!(output.structured_content["status_read_required"], true);
            assert_eq!(output.structured_content["completion_claim_withheld"], true);
            assert_eq!(
                output.structured_content["method_result"]["effect_kind"],
                "core_committed"
            );
            assert!(!output.diagnostic_facts.authoritative_refresh_failure);

            let rendered = serde_json::to_vec(&tool_call_result_from_output(output))?;
            assert!(rendered.len() <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
            assert!(!String::from_utf8(rendered)?.contains(omitted_marker));
        }

        let output = ToolCallOutput::from_pipeline_response(&committed)?
            .with_post_effect_failure(McpPostEffectFailureCode::McpPostEffectAdapterFailed);
        let output = finalize_mutation_output_with_refresh(
            INTAKE_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            output,
            |_| Ok(refreshed),
        )?;
        assert_eq!(
            output.structured_content["code"],
            "MCP_POST_EFFECT_ADAPTER_FAILED"
        );
        assert_eq!(output.structured_content["authority_receipt"], Value::Null);
        assert_eq!(
            output.structured_content["response_projection_omitted"],
            true
        );
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }

    #[test]
    fn post_effect_recovery_preserves_receipt_then_compact_result_then_effect_facts(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, committed, receipt) =
            committed_intake_with_receipt("mcp-post-effect-recovery-order")?;

        let compact = compact_mutation_method_result(INTAKE_TOOL_NAME, &committed.response_value)?;
        let both_outcome = recovery_outcome(
            INTAKE_TOOL_NAME,
            MutationDetailLevel::Summary,
            Some(receipt.clone()),
            Some(committed.response_value.clone()),
            Some(compact),
        );
        let both = mutation_post_effect_failure_output(
            &both_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert!(both.structured_content["authority_receipt"].is_object());
        assert!(both.structured_content["method_result"].is_object());
        assert_compact_budget(both)?;

        let mut oversized_exact_result = committed.response_value.clone();
        oversized_exact_result["adapter_test_padding"] =
            Value::String("x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES));
        let expected_compact =
            compact_mutation_method_result(INTAKE_TOOL_NAME, &oversized_exact_result)?;
        let receipt_and_compact_outcome = recovery_outcome(
            INTAKE_TOOL_NAME,
            MutationDetailLevel::Summary,
            Some(receipt.clone()),
            Some(oversized_exact_result.clone()),
            Some(expected_compact.clone()),
        );
        let receipt_and_compact = mutation_post_effect_failure_output(
            &receipt_and_compact_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert!(receipt_and_compact.structured_content["authority_receipt"].is_object());
        assert_eq!(
            receipt_and_compact.structured_content["method_result"],
            expected_compact
        );
        assert_compact_budget(receipt_and_compact)?;

        let compact_only_outcome = recovery_outcome(
            INTAKE_TOOL_NAME,
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            Some(oversized_exact_result),
            Some(expected_compact),
        );
        let compact_only = mutation_post_effect_failure_output(
            &compact_only_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert_eq!(
            compact_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            compact_only.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert_compact_budget(compact_only)?;

        let mut unprojectable_result = committed.response_value;
        unprojectable_result["base"] = Value::String("invalid".to_owned());
        unprojectable_result["adapter_test_padding"] =
            Value::String("x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES));
        let effect_facts_outcome = recovery_outcome(
            INTAKE_TOOL_NAME,
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            Some(unprojectable_result),
            None,
        );
        let effect_facts_only = mutation_post_effect_failure_output(
            &effect_facts_outcome,
            McpPostEffectFailureCode::McpResponseProjectionFailed,
        )?;
        assert_eq!(
            effect_facts_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            effect_facts_only.structured_content["method_result"],
            Value::Null
        );
        assert_compact_budget(effect_facts_only)?;
        Ok(())
    }

    #[test]
    fn post_effect_recovery_budget_table_uses_canonical_candidate_priority(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, _committed, receipt) =
            committed_intake_with_receipt("mcp-post-effect-recovery-budget-table")?;
        let small_exact = json!({"projection_marker": "exact"});
        let large_exact = json!({
            "projection_marker": "exact",
            "padding": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES),
        });
        let small_compact = json!({"projection_marker": "compact"});
        let large_compact = json!({
            "projection_marker": "compact",
            "padding": "한".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES),
        });
        let oversized_receipt =
            receipt_with_message_padding(&receipt, MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
        let cases = vec![
            (
                "receipt_exact",
                receipt.clone(),
                small_exact.clone(),
                small_compact.clone(),
                true,
                Some("exact"),
            ),
            (
                "receipt_compact",
                receipt.clone(),
                large_exact.clone(),
                small_compact.clone(),
                true,
                Some("compact"),
            ),
            (
                "receipt_only",
                receipt,
                large_exact.clone(),
                large_compact.clone(),
                true,
                None,
            ),
            (
                "compact_only",
                oversized_receipt.clone(),
                large_exact.clone(),
                small_compact,
                false,
                Some("compact"),
            ),
            (
                "effect_facts_only",
                oversized_receipt,
                large_exact,
                large_compact,
                false,
                None,
            ),
        ];

        for (name, receipt, exact, compact, expect_receipt, expected_marker) in cases {
            let outcome = recovery_outcome(
                INTAKE_TOOL_NAME,
                MutationDetailLevel::Summary,
                Some(receipt),
                Some(exact),
                Some(compact),
            );
            let output = mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )?;
            assert_eq!(
                output.structured_content["authority_receipt"].is_object(),
                expect_receipt,
                "{name} receipt preservation"
            );
            assert_eq!(
                output.structured_content["method_result"]["projection_marker"].as_str(),
                expected_marker,
                "{name} method-result preservation"
            );
            if expected_marker.is_none() {
                assert!(
                    output.structured_content["method_result"].is_null(),
                    "{name} must omit the complete method result"
                );
            }
            assert_compact_budget(output)?;
        }
        Ok(())
    }

    #[test]
    fn response_budget_recovery_preserves_receipt_then_compact_result_then_effect_facts(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, _committed, receipt) =
            committed_intake_with_receipt("mcp-response-budget-recovery-order")?;
        let small_result = json!({"effect_kind": "core_committed"});

        let both_outcome = recovery_outcome(
            INTAKE_TOOL_NAME,
            MutationDetailLevel::Full,
            Some(receipt.clone()),
            None,
            Some(small_result.clone()),
        );
        let both = mutation_response_budget_exceeded_output(&both_outcome)?;
        assert!(both.structured_content["authority_receipt"].is_object());
        assert_eq!(
            both.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert_compact_budget(both)?;

        let receipt_only_outcome = recovery_outcome(
            INTAKE_TOOL_NAME,
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(&receipt, 36 * 1024)),
            None,
            Some(json!({
                "effect_kind": "core_committed",
                "padding": "x".repeat(36 * 1024),
            })),
        );
        let receipt_only = mutation_response_budget_exceeded_output(&receipt_only_outcome)?;
        assert!(receipt_only.structured_content["authority_receipt"].is_object());
        assert_eq!(
            receipt_only.structured_content["method_result"],
            Value::Null
        );
        assert_compact_budget(receipt_only)?;

        let compact_only_outcome = recovery_outcome(
            INTAKE_TOOL_NAME,
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            None,
            Some(small_result),
        );
        let compact_only = mutation_response_budget_exceeded_output(&compact_only_outcome)?;
        assert_eq!(
            compact_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            compact_only.structured_content["method_result"]["effect_kind"],
            "core_committed"
        );
        assert_compact_budget(compact_only)?;

        let effect_facts_outcome = recovery_outcome(
            INTAKE_TOOL_NAME,
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            None,
            Some(json!({
                "effect_kind": "core_committed",
                "padding": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES),
            })),
        );
        let effect_facts_only = mutation_response_budget_exceeded_output(&effect_facts_outcome)?;
        assert_eq!(
            effect_facts_only.structured_content["authority_receipt"],
            Value::Null
        );
        assert_eq!(
            effect_facts_only.structured_content["method_result"],
            Value::Null
        );
        assert_compact_budget(effect_facts_only)?;
        Ok(())
    }

    #[test]
    fn record_run_actual_producer_ref_survives_default_compact_and_bounded_recovery(
    ) -> Result<(), Box<dyn Error>> {
        let (_fixture, recorded, refreshed, producer_ref) =
            committed_record_run_with_capture_producer("mcp-record-run-producer-recovery")?;
        let default_detail = MutationDetailLevel::default();
        assert_eq!(default_detail, MutationDetailLevel::Summary);
        let decode_producer_refs =
            |value: Value, label: &str| -> Result<Vec<StateRecordRef>, Box<dyn Error>> {
                if !value.is_array() {
                    return Err(format!("{label} did not preserve producer refs: {value}").into());
                }
                Ok(serde_json::from_value(value)?)
            };

        let normal = finalize_mutation_output_with_refresh(
            RECORD_RUN_TOOL_NAME,
            Some(default_detail),
            ToolCallOutput::from_pipeline_response(&recorded)?,
            |_| Ok(refreshed.clone()),
        )?;
        let default_refs = decode_producer_refs(
            normal.structured_content["method_result"]["evidence_producer_refs"].clone(),
            "default compact finalizer",
        )?;
        assert_eq!(default_refs, vec![producer_ref.clone()]);
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(normal))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );

        let mut oversized_recorded = recorded.clone();
        oversized_recorded.response_value["run_summary"]["summary"] =
            Value::String("x".repeat(MAX_MCP_FULL_MUTATION_RESULT_BYTES));
        oversized_recorded.response_json =
            serde_json::to_string(&oversized_recorded.response_value)?;

        let mut projection_output = ToolCallOutput::from_pipeline_response(&oversized_recorded)?;
        projection_output.post_effect_failure =
            Some(McpPostEffectFailureCode::McpResponseProjectionFailed);
        let projection_recovery = finalize_mutation_output_with_refresh(
            RECORD_RUN_TOOL_NAME,
            Some(default_detail),
            projection_output,
            |_| Ok(refreshed.clone()),
        )?;

        let budget_recovery = finalize_mutation_output_with_refresh(
            RECORD_RUN_TOOL_NAME,
            Some(MutationDetailLevel::Full),
            ToolCallOutput::from_pipeline_response(&oversized_recorded)?,
            |_| Ok(refreshed.clone()),
        )?;

        let recoveries = [
            (projection_recovery, "MCP_RESPONSE_PROJECTION_FAILED"),
            (budget_recovery, "MCP_RESPONSE_BUDGET_EXCEEDED"),
        ];
        for (recovery, expected_code) in recoveries {
            assert_eq!(recovery.structured_content["code"], expected_code);
            assert!(recovery.structured_content["authority_receipt"].is_object());
            let producer_refs = decode_producer_refs(
                recovery.structured_content["method_result"]["evidence_producer_refs"].clone(),
                expected_code,
            )?;
            assert_eq!(producer_refs, vec![producer_ref.clone()]);
            assert_eq!(recovery.structured_content["effect_applied"], true);
            assert_eq!(recovery.structured_content["committed"], true);
            assert_eq!(recovery.structured_content["retryable"], false);
            assert!(
                serde_json::to_vec(&tool_call_result_from_output(recovery))?.len()
                    <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
            );
        }
        Ok(())
    }

    #[test]
    fn user_action_derived_refs_survive_compact_only_recovery_paths() -> Result<(), Box<dyn Error>>
    {
        let (_fixture, _committed, receipt) =
            committed_intake_with_receipt("mcp-user-action-derived-ref-recovery")?;
        let derived_ref = json!({
            "record_kind": "project_continuity_record",
            "record_id": "continuity_user_action_derived",
            "project_id": "project_mcp_recovery_order",
            "task_id": "task_mcp_recovery_order",
            "produced_at_state_version": 3
        });
        let compact = json!({
            "effect": {
                "effect_kind": "core_committed",
                "state_version": 3,
                "events": []
            },
            "agent_workflow_result_replayed": true,
            "user_action_request_summary": {
                "user_action_request_id": "user_action_request_recovery",
                "status": "pending",
                "next_actor": "user"
            },
            "user_action_resolution_ref": {
                "record_kind": "user_action_resolution",
                "record_id": "user_action_resolution_recovery",
                "project_id": "project_mcp_recovery_order",
                "task_id": "task_mcp_recovery_order",
                "produced_at_state_version": 3
            },
            "current_projection_state_version": 4,
            "current_projection_observed_at": "2026-07-13T12:00:00Z",
            "status": "resolved",
            "resolution_summary": {
                "resolution_type": "choice",
                "selected_option_id": "accept",
                "selected_option_label": "Accept",
                "machine_action": "accept",
                "resolution_outcome": "accepted"
            },
            "derived_refs": [derived_ref.clone()]
        });
        let outcome = recovery_outcome(
            REQUEST_USER_ACTION_TOOL_NAME,
            MutationDetailLevel::Summary,
            Some(receipt_with_message_padding(
                &receipt,
                MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            )),
            Some(json!({
                "padding": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES)
            })),
            Some(compact),
        );

        let outputs = [
            mutation_post_effect_failure_output(
                &outcome,
                McpPostEffectFailureCode::McpResponseProjectionFailed,
            )?,
            mutation_response_budget_exceeded_output(&outcome)?,
            authoritative_refresh_failure_output(&outcome)?,
        ];
        for output in outputs {
            assert_eq!(output.structured_content["authority_receipt"], Value::Null);
            assert_eq!(
                output.structured_content["method_result"]["derived_refs"][0],
                derived_ref
            );
            assert_eq!(
                output.structured_content["method_result"]["agent_workflow_result_replayed"],
                true
            );
            assert_eq!(
                output.structured_content["method_result"]["current_projection_state_version"],
                4
            );
            assert_eq!(
                output.structured_content["method_result"]["current_projection_observed_at"],
                "2026-07-13T12:00:00Z"
            );
            assert_eq!(
                output.structured_content["method_result"]["status"],
                "resolved"
            );
            assert!(
                serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                    <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
            );
        }
        Ok(())
    }

    #[test]
    fn oversized_compact_method_result_is_omitted_from_authoritative_refresh_failure(
    ) -> Result<(), Box<dyn Error>> {
        let facts = recovery_facts();
        let oversized_method_result = json!({
            "effect_kind": "core_committed",
            "oversized": "x".repeat(MAX_MCP_COMPACT_MUTATION_RESULT_BYTES * 2),
        });
        let outcome = recovery_outcome(
            INTAKE_TOOL_NAME,
            MutationDetailLevel::Summary,
            None,
            None,
            Some(oversized_method_result),
        );
        let mut outcome = outcome;
        outcome.facts = facts;
        let output = authoritative_refresh_failure_output(&outcome)?;
        assert_eq!(output.structured_content["code"], "MCP_UNAVAILABLE");
        assert_eq!(output.structured_content["method_result"], Value::Null);
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }

    #[test]
    fn oversized_stage_projection_preserves_the_staging_handle_in_bounded_recovery(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("mcp-stage-oversized-fresh-receipt")?;
        let core = CoreService::new(fixture.runtime_home_path());
        let workflow_invocation = || {
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::AgentWorkflow,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            )
        };
        let intake = core.intake(
            fixture.intake_request(
                "req_mcp_stage_oversized_intake",
                "idem_mcp_stage_oversized_intake",
                false,
                Some(0),
            ),
            workflow_invocation(),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("intake resolves a Task");
        let state_version = intake.response_value["base"]["state_version"]
            .as_u64()
            .expect("intake state version");
        let staged = core.stage_artifact(
            fixture.stage_artifact_request(
                "req_mcp_stage_oversized_stage",
                None,
                false,
                Some(state_version),
                task_id.as_str(),
            ),
            workflow_invocation(),
        )?;
        let expected_handle = staged.response_value["staged_artifact_handle"].clone();
        let mut refreshed = core.status(
            fixture.status_request("req_mcp_stage_oversized_status", Some(task_id.as_str())),
            InvocationContext::new(
                ProjectId::new(fixture.project_id()),
                ActorSource::agent_connection(fixture.connection_id()),
                OperationCategory::Read,
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
        )?;
        let mut blocker = refreshed.response_value["authority_receipt"]["close_blockers"]
            .as_array()
            .and_then(|blockers| blockers.first())
            .cloned()
            .expect("status exposes a close blocker");
        blocker["message"] = Value::String("x".repeat(MAX_MCP_FULL_MUTATION_RESULT_BYTES * 2));
        let blockers = Value::Array(vec![blocker]);
        refreshed.response_value["authority_receipt"]["close_blockers"] = blockers.clone();
        refreshed.response_value["active_task"]["close_blockers"] = blockers.clone();
        refreshed.response_value["close_blockers"] = blockers;
        refreshed.response_json = serde_json::to_string(&refreshed.response_value)?;

        let output = finalize_mutation_output_with_refresh(
            STAGE_ARTIFACT_TOOL_NAME,
            Some(MutationDetailLevel::Summary),
            ToolCallOutput::from_pipeline_response(&staged)?,
            |_| Ok(refreshed),
        )?;

        assert_eq!(
            output.structured_content["code"],
            "MCP_RESPONSE_BUDGET_EXCEEDED"
        );
        assert_eq!(
            output.structured_content["method_result"]["effect"]["effect_kind"],
            "staging_created"
        );
        assert_eq!(
            output.structured_content["method_result"]["staged_artifact_handle"],
            expected_handle
        );
        assert!(
            serde_json::to_vec(&tool_call_result_from_output(output))?.len()
                <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES
        );
        Ok(())
    }
}
