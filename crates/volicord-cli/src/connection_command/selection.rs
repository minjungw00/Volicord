use std::{
    fs,
    path::{Path, PathBuf},
};

use volicord_store::{
    agent_connections::{
        list_agent_connections, list_agent_connections_for_diagnostics,
        list_agent_connections_read_only, list_connection_projects,
        list_connection_projects_for_diagnostics, AgentConnectionRecord, ConnectionProjectRecord,
    },
    bootstrap::project_record_by_repo_root,
};

use crate::guard_integration::hooks::shell_word;
use crate::host_integration::{
    codex::CodexAdapter, ConnectionIntent, HostAdapter, HostKind, HostScope,
};

use super::{
    args::{absolute_path, ParsedConnectionOptions},
    codex_environment, connection_intent_from_flags, intent_flag_suffix,
    output::display_project_roots,
    path_text, public_host_label, public_host_name_text, public_mode_text, ConnectionCommandError,
    ConnectionProcess,
};

#[derive(Debug, Clone)]
pub(super) struct ConnectionSelector {
    host_kind: HostKind,
    intent: Option<ConnectionIntent>,
    host_scope: Option<HostScope>,
    repo_root: PathBuf,
}

impl ConnectionSelector {
    pub(super) fn repo_root(&self) -> &Path {
        &self.repo_root
    }
}

pub(super) fn host_scope_for_intent(
    _host_kind: HostKind,
    intent: ConnectionIntent,
) -> Result<HostScope, ConnectionCommandError> {
    match intent {
        ConnectionIntent::Personal => Ok(HostScope::User),
        ConnectionIntent::Shared => Ok(HostScope::Project),
    }
}

pub(super) fn resolve_connection_host(
    explicit: Option<HostKind>,
    process: &impl ConnectionProcess,
) -> Result<HostKind, ConnectionCommandError> {
    if let Some(host_kind) = explicit {
        return Ok(host_kind);
    }
    let mut available = Vec::new();
    if let Ok(detection) = CodexAdapter::new(codex_environment(process)).detect() {
        if detection.available {
            available.push(detection.host_kind);
        }
    }
    available.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    available.dedup();
    match available.as_slice() {
        [host_kind] => Ok(*host_kind),
        [] => Err(ConnectionCommandError::usage(
            "HOST_NOT_DETECTED: Codex could not be identified; pass `codex` after installing it",
        )),
        _ => Err(ConnectionCommandError::usage(
            "HOST_AMBIGUOUS: multiple Codex host detections were returned",
        )),
    }
}

pub(super) fn connection_selector(
    parsed: &ParsedConnectionOptions,
    current_dir: &Path,
    process: &impl ConnectionProcess,
) -> Result<ConnectionSelector, ConnectionCommandError> {
    let host_kind = resolve_connection_host(parsed.host_kind, process)?;
    let intent = if parsed.shared {
        Some(connection_intent_from_flags(parsed)?)
    } else {
        None
    };
    let host_scope = intent
        .map(|intent| host_scope_for_intent(host_kind, intent))
        .transpose()?;
    let repo_root = resolve_connection_repo_root(current_dir, parsed.repo.as_deref())?;
    Ok(ConnectionSelector {
        host_kind,
        intent,
        host_scope,
        repo_root,
    })
}

pub(super) fn resolve_connection_repo_root(
    current_dir: &Path,
    selected_path: Option<&Path>,
) -> Result<PathBuf, ConnectionCommandError> {
    let selected = selected_path.unwrap_or(current_dir);
    let absolute = absolute_path(current_dir, selected.to_path_buf());
    let canonical = fs::canonicalize(&absolute).map_err(|error| {
        ConnectionCommandError::runtime(format!(
            "repository path is not accessible: {} ({error})",
            absolute.display()
        ))
    })?;
    let metadata = fs::metadata(&canonical).map_err(|error| {
        ConnectionCommandError::runtime(format!(
            "repository path is not accessible: {} ({error})",
            canonical.display()
        ))
    })?;
    let mut cursor = if metadata.is_file() {
        canonical
            .parent()
            .ok_or_else(|| {
                ConnectionCommandError::runtime(format!(
                    "repository path has no parent directory: {}",
                    canonical.display()
                ))
            })?
            .to_path_buf()
    } else {
        canonical
    };

    loop {
        let git_path = cursor.join(".git");
        match git_path.try_exists() {
            Ok(true) => return Ok(cursor),
            Ok(false) => {}
            Err(error) => {
                return Err(ConnectionCommandError::runtime(format!(
                    "failed to inspect Git repository marker {}: {error}",
                    git_path.display()
                )));
            }
        }
        if !cursor.pop() {
            break;
        }
    }

    Err(ConnectionCommandError::runtime(format!(
        "no Git repository root found from {}; run `volicord project use PATH` from inside a Git repository or pass --repo PATH",
        absolute.display()
    )))
}

pub(super) fn connection_for_host_target(
    runtime_home: &Path,
    host_kind: HostKind,
    intent: ConnectionIntent,
    host_scope: HostScope,
    config_target: &str,
    server_name: &str,
) -> Result<Option<AgentConnectionRecord>, ConnectionCommandError> {
    let matches = list_agent_connections_read_only(runtime_home)?
        .into_iter()
        .filter(|connection| {
            connection.host_kind == host_kind.as_str()
                && connection.intent == intent.as_str()
                && connection.host_scope == host_scope.as_str()
                && connection.config_target == config_target
                && connection.server_name == server_name
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [connection] => Ok(Some(connection.clone())),
        connections => Err(ConnectionCommandError::runtime(ambiguous_target_message(
            connections,
        ))),
    }
}

pub(super) fn select_connection(
    runtime_home: &Path,
    selector: &ConnectionSelector,
) -> Result<(AgentConnectionRecord, Vec<ConnectionProjectRecord>), ConnectionCommandError> {
    select_connection_with_diagnostic_reads(runtime_home, selector, false)
}

pub(super) fn select_connection_for_diagnostics(
    runtime_home: &Path,
    selector: &ConnectionSelector,
) -> Result<(AgentConnectionRecord, Vec<ConnectionProjectRecord>), ConnectionCommandError> {
    select_connection_with_diagnostic_reads(runtime_home, selector, true)
}

fn select_connection_with_diagnostic_reads(
    runtime_home: &Path,
    selector: &ConnectionSelector,
    diagnostic_reads: bool,
) -> Result<(AgentConnectionRecord, Vec<ConnectionProjectRecord>), ConnectionCommandError> {
    if project_record_by_repo_root(runtime_home, &selector.repo_root)?.is_none() {
        return Err(ConnectionCommandError::runtime(format!(
            "PROJECT_NOT_REGISTERED: repository {} is not registered in Runtime Home {}; run `{}` first",
            selector.repo_root.display(),
            runtime_home.display(),
            selector_repair_command(selector, runtime_home)
        )));
    }
    let mut matches = Vec::new();
    let mut same_host_connections = Vec::new();
    let connections = if diagnostic_reads {
        list_agent_connections_for_diagnostics(runtime_home)?
    } else {
        list_agent_connections(runtime_home)?
    };
    for connection in connections {
        if connection.host_kind != selector.host_kind.as_str() {
            continue;
        }
        if selector
            .intent
            .is_some_and(|intent| connection.intent != intent.as_str())
        {
            continue;
        }
        if selector
            .host_scope
            .is_some_and(|scope| connection.host_scope != scope.as_str())
        {
            continue;
        }
        let projects = if diagnostic_reads {
            list_connection_projects_for_diagnostics(
                runtime_home,
                &connection.connection_internal_id,
            )?
        } else {
            list_connection_projects(runtime_home, &connection.connection_internal_id)?
        };
        same_host_connections.push((connection.clone(), projects.clone()));
        if projects
            .iter()
            .any(|project| project.project.repo_root == selector.repo_root)
        {
            matches.push((connection, projects));
        }
    }
    match matches.len() {
        0 if same_host_connections.is_empty() => Err(ConnectionCommandError::runtime(format!(
            "CONNECTION_NOT_FOUND: no Agent Connection in Runtime Home {} matches host {}, intent {}, and repository {}; run `{}`",
            runtime_home.display(),
            public_host_label(selector.host_kind),
            selector_intent_text(selector),
            selector.repo_root.display(),
            selector_repair_command(selector, runtime_home)
        ))),
        0 => Err(ConnectionCommandError::runtime(format!(
            "CONNECTION_ALLOWLIST_MISMATCH: repository {} is not in the selected Agent Connection project allowlist in Runtime Home {}; run `{}`",
            selector.repo_root.display(),
            runtime_home.display(),
            selector_repair_command(selector, runtime_home)
        ))),
        1 => Ok(matches.remove(0)),
        _ => Err(ConnectionCommandError::runtime(ambiguous_selector_message(
            selector, &matches,
        ))),
    }
}

pub(super) fn selected_connection_project<'a>(
    projects: &'a [ConnectionProjectRecord],
    repo_root: &Path,
) -> Result<&'a ConnectionProjectRecord, ConnectionCommandError> {
    projects
        .iter()
        .find(|project| project.project.repo_root == repo_root)
        .ok_or_else(|| ConnectionCommandError::runtime("selected repository is not connected"))
}

fn selector_intent_text(selector: &ConnectionSelector) -> &'static str {
    selector
        .intent
        .map(|intent| intent.as_str())
        .unwrap_or("any")
}

fn selector_repair_command(selector: &ConnectionSelector, runtime_home: &Path) -> String {
    let runtime_home = shell_word(&path_text(runtime_home));
    let repo_root = shell_word(&path_text(&selector.repo_root));
    match selector.intent {
        Some(intent @ ConnectionIntent::Personal) => format!(
            "volicord connection add {}{} --repo {} --home {}",
            public_host_label(selector.host_kind),
            intent_flag_suffix(intent),
            repo_root,
            runtime_home
        ),
        Some(ConnectionIntent::Shared) => format!(
            "volicord init --host {} --shared --repo {} --home {}",
            public_host_label(selector.host_kind),
            repo_root,
            runtime_home
        ),
        None => format!(
            "volicord init --host {} --repo {} --home {}",
            public_host_label(selector.host_kind),
            repo_root,
            runtime_home
        ),
    }
}

fn ambiguous_target_message(connections: &[AgentConnectionRecord]) -> String {
    let mut message = String::from("host target matches multiple Agent Connections; choices:\n");
    for connection in connections {
        message.push_str(&format!(
            "- host: {}; intent: {}; target: {}; mode: {}\n",
            public_host_name_text(&connection.host_kind),
            connection.intent,
            connection.config_target,
            public_mode_text(&connection.mode)
        ));
    }
    message
}

fn ambiguous_selector_message(
    selector: &ConnectionSelector,
    matches: &[(AgentConnectionRecord, Vec<ConnectionProjectRecord>)],
) -> String {
    let mut message = format!(
        "connection selector is ambiguous for host {}, intent {}, repository {}; choices:\n",
        public_host_label(selector.host_kind),
        selector_intent_text(selector),
        selector.repo_root.display()
    );
    for (connection, projects) in matches {
        message.push_str(&format!(
            "- target: {}; mode: {}; connected_repositories: {}\n",
            connection.config_target,
            public_mode_text(&connection.mode),
            display_project_roots(projects)
        ));
    }
    message.push_str("Use a more specific repository path or remove the duplicate connection.\n");
    message
}
