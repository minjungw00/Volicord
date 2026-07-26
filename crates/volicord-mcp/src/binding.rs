//! Runtime Home, repository, Connection, and managed-session binding.
//!
//! This module owns selection of the local routing identity. It does not parse
//! JSON-RPC envelopes, frame stdio, or invoke Core methods.

use crate::adapter::{ManagedAgentSessionBinding, McpAdapter};
use crate::errors::{McpAdapterError, McpHostError};
use crate::routing::{
    validate_mcp_project_allowlist, McpConnectionContext, McpConnectionStartupInspection,
    McpPreflightReport, RepositoryDiscoveryResolution,
};
use crate::util::{current_dir_environment_error, process_env_var};
use crate::VOLICORD_HOME_ENV;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use volicord_host_contract::{CodexMcpCorrelation, CodexMcpTurnMetadata, HostContractErrorCode};
use volicord_store::agent_connections::agent_connection_record_read_only;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_store::runtime_home::{
    resolve_runtime_home as resolve_shared_runtime_home, RuntimeHomeResolutionError,
};
use volicord_types::ids::ProjectId;
use volicord_types::integration_revision::McpRuntimeSessionSource;
use volicord_types::values::HostKind;

const CODEX_THREAD_BINDING_DOMAIN: &[u8] = b"volicord.codex-mcp-thread-binding\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CodexManagedBinding {
    NotApplicable,
    Pending,
    Bound {
        thread_digest: [u8; 32],
        correlation: CodexMcpCorrelation,
    },
}

impl CodexManagedBinding {
    pub(crate) const fn for_session_source(source: McpRuntimeSessionSource) -> Self {
        if matches!(source, McpRuntimeSessionSource::ManagedHost) {
            Self::Pending
        } else {
            Self::NotApplicable
        }
    }

    pub(crate) const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    pub(crate) const fn is_bound(&self) -> bool {
        matches!(self, Self::Bound { .. })
    }

    pub(crate) fn correlation(&self) -> Option<&CodexMcpCorrelation> {
        match self {
            Self::Bound { correlation, .. } => Some(correlation),
            Self::NotApplicable | Self::Pending => None,
        }
    }
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

/// Resolves the explicit Runtime Home required by repository discovery.
pub fn resolve_repository_discovery_runtime_home<F>(
    env_var: F,
    current_dir: &Path,
) -> Result<PathBuf, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = env_var(VOLICORD_HOME_ENV).ok_or_else(|| {
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
        |name| (name == VOLICORD_HOME_ENV).then(|| runtime_home.clone()),
        current_dir,
    )
}

/// Runs MCP startup validation from process environment.
pub fn run_preflight_check_from_env(
    connection_id: &str,
    project_id: Option<&str>,
) -> Result<McpPreflightReport, McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    preflight_check(process_env_var, &current_dir, connection_id, project_id)
}

/// Runs MCP startup validation from injected process inputs.
pub fn preflight_check<F>(
    env_var: F,
    current_dir: &Path,
    connection_id: &str,
    project_id: Option<&str>,
) -> Result<McpPreflightReport, McpAdapterError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let runtime_home = resolve_runtime_home(&env_var, current_dir)?;
    let detail_project_id = project_id.map(ProjectId::new);
    let inspection =
        McpConnectionStartupInspection::resolve(&runtime_home, connection_id, detail_project_id)?;
    Ok(inspection.preflight_report())
}

pub(crate) fn manual_adapter_from_process(
    connection_id: &str,
    project_id: Option<&str>,
) -> Result<McpAdapter, McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    let runtime_home = resolve_runtime_home(process_env_var, &current_dir)?;
    let project_allowlist = project_id
        .map(ProjectId::new)
        .into_iter()
        .collect::<Vec<_>>();
    validate_mcp_project_allowlist(&runtime_home, connection_id, &project_allowlist)?;
    let context = McpConnectionContext::resolve(&runtime_home, connection_id)?
        .with_project_allowlist(project_allowlist);
    Ok(McpAdapter::new(runtime_home, context))
}

pub(crate) fn repository_discovery_adapter_from_process(
    host: HostKind,
) -> Result<McpAdapter, McpAdapterError> {
    let current_dir = std::env::current_dir().map_err(current_dir_environment_error)?;
    let runtime_home = resolve_repository_discovery_runtime_home(process_env_var, &current_dir)?;
    let resolution = RepositoryDiscoveryResolution::resolve(&runtime_home, &current_dir, host)?;
    Ok(McpAdapter::new(runtime_home, resolution.context))
}

pub(crate) fn managed_agent_session_binding(
    binding: &CodexManagedBinding,
    runtime_session_id: &str,
) -> Option<ManagedAgentSessionBinding> {
    binding
        .correlation()
        .map(|correlation| ManagedAgentSessionBinding {
            runtime_session_id: runtime_session_id.to_owned(),
            correlation: correlation.clone(),
        })
}

pub(crate) fn bind_codex_managed_tool_call(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    active_binding: &mut CodexManagedBinding,
    params: &Map<String, Value>,
) -> Result<bool, McpHostError> {
    if matches!(active_binding, CodexManagedBinding::NotApplicable) {
        return Ok(false);
    }
    let binding =
        codex_managed_call_binding(params, adapter.context.connection_internal_id.as_str())?;
    match active_binding {
        CodexManagedBinding::Pending => {
            let candidate = CodexManagedBinding::Bound {
                thread_digest: binding.thread_digest,
                correlation: binding.correlation,
            };
            let previous = std::mem::replace(active_binding, candidate);
            if validate_managed_stdio_session_ownership_admitted(context, adapter, active_binding)
                .is_err()
            {
                *active_binding = previous;
                return Err(McpHostError::RegisteredSessionCorrelationMismatch);
            }
            Ok(true)
        }
        CodexManagedBinding::Bound {
            thread_digest,
            correlation,
        } if correlation.session_id == binding.correlation.session_id
            && *thread_digest == binding.thread_digest =>
        {
            correlation.turn_id = binding.correlation.turn_id;
            Ok(false)
        }
        CodexManagedBinding::Bound { .. } => {
            Err(McpHostError::RegisteredSessionCorrelationMismatch)
        }
        CodexManagedBinding::NotApplicable => Ok(false),
    }
}

pub(crate) fn validate_managed_stdio_session_ownership(
    adapter: &McpAdapter,
    binding: &CodexManagedBinding,
) -> Result<(), McpAdapterError> {
    if !binding.is_bound() {
        return Ok(());
    }
    let _correlation = binding.correlation().ok_or_else(|| {
        McpAdapterError::Environment(
            "managed_host_session_correlation_invalid: active managed stdio binding has no host-native session correlation"
                .to_owned(),
        )
    })?;
    let _connection = agent_connection_record_read_only(
        &adapter.runtime_home,
        adapter.context.connection_internal_id.as_str(),
    )
    .map_err(McpAdapterError::Store)?
    .ok_or_else(|| {
        McpAdapterError::Environment(
            "managed_stdio_session_ownership_unavailable: managed stdio connection is unavailable"
                .to_owned(),
        )
    })?;
    Ok(())
}

pub(crate) fn validate_managed_stdio_session_ownership_admitted(
    context: &RuntimeHomeMutationContext<'_>,
    adapter: &McpAdapter,
    binding: &CodexManagedBinding,
) -> Result<(), McpAdapterError> {
    if !binding.is_bound() {
        return Ok(());
    }
    let _correlation = binding.correlation().ok_or_else(|| {
        McpAdapterError::Environment(
            "managed_host_session_correlation_invalid: active managed stdio binding has no host-native session correlation"
                .to_owned(),
        )
    })?;
    let _connection = agent_connection_record_read_only(
        adapter.admitted_runtime_home(context)?,
        adapter.context.connection_internal_id.as_str(),
    )
    .map_err(McpAdapterError::Store)?
    .ok_or_else(|| {
        McpAdapterError::Environment(
            "managed_stdio_session_ownership_unavailable: managed stdio connection is unavailable"
                .to_owned(),
        )
    })?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexManagedCallBinding {
    thread_digest: [u8; 32],
    correlation: CodexMcpCorrelation,
}

fn codex_managed_call_binding(
    params: &Map<String, Value>,
    connection_internal_id: &str,
) -> Result<CodexManagedCallBinding, McpHostError> {
    let correlation = CodexMcpTurnMetadata
        .parse_tools_call_params(params)
        .map_err(|error| match error.code() {
            HostContractErrorCode::InconsistentCorrelation => {
                McpHostError::SessionThreadTurnInconsistent
            }
            _ => McpHostError::MalformedNativeMetadata,
        })?;
    let thread_digest = codex_thread_correlation_digest(
        connection_internal_id,
        correlation.session_id.as_str(),
        correlation.thread_id.as_str(),
    );
    Ok(CodexManagedCallBinding {
        thread_digest,
        correlation,
    })
}

fn codex_thread_correlation_digest(
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
mod tests {
    use super::*;
    use serde_json::json;

    const CODEX_TURN_METADATA_KEY: &str = "x-codex-turn-metadata";

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
        let replay = codex_managed_call_binding(&valid_params(), "connection.alpha")
            .expect("same metadata should replay");
        assert_eq!(first, replay);

        let mut next_turn = valid_params();
        next_turn["_meta"][CODEX_TURN_METADATA_KEY]["turn_id"] = json!("turn.beta");
        let next = codex_managed_call_binding(&next_turn, "connection.alpha")
            .expect("turn changes do not change the session/thread binding");
        assert_eq!(next.correlation.session_id, first.correlation.session_id);
        assert_eq!(next.thread_digest, first.thread_digest);
        assert_eq!(next.correlation.thread_id, first.correlation.thread_id);
        assert_eq!(next.correlation.turn_id.as_str(), "turn.beta");
    }

    #[test]
    fn malformed_codex_call_metadata_is_rejected() {
        for params in [
            json!({"name":"volicord.status","arguments":{}}),
            json!({"name":"volicord.status","arguments":{},"_meta":null}),
            json!({
                "name":"volicord.status",
                "arguments":{},
                "_meta":{
                    "threadId":"thread.one",
                    CODEX_TURN_METADATA_KEY:{
                        "session_id":"session.one",
                        "thread_id":"thread.two",
                        "turn_id":"turn.one"
                    }
                }
            }),
        ] {
            assert!(codex_managed_call_binding(
                params.as_object().expect("fixture object"),
                "connection.alpha"
            )
            .is_err());
        }
    }

    #[test]
    fn repository_discovery_requires_an_explicit_absolute_runtime_home() {
        let missing = resolve_repository_discovery_runtime_home(|_| None, Path::new("/repo"))
            .expect_err("missing Runtime Home");
        assert!(missing.to_string().contains("requires VOLICORD_HOME"));

        let relative = resolve_repository_discovery_runtime_home(
            |name| (name == VOLICORD_HOME_ENV).then(|| OsString::from("runtime")),
            Path::new("/repo"),
        )
        .expect_err("relative Runtime Home");
        assert!(relative.to_string().contains("absolute VOLICORD_HOME"));
    }
}
