use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde_json::Value;
use volicord_types::{
    AgentConnectionId, GuardCommand, GuardCommandAbsolutePath, GuardCommandInvocation,
    GuardCommandInvocationSet, GuardCommandProjection, GuardCommandSet, GuardHookPhase,
    GuardInstallationId, IntegrationProfile, PolicyHash,
};

use crate::{
    guard_integration::{
        audit::{CODEX_DISPATCH_WRAPPER, HOOK_WRAPPER_MARKER},
        files::{plan_managed_script_file, GeneratedFilePlan},
        public_host_label, GuardIntegrationError, HookWrapperResolutionStatus,
    },
    host_integration::{
        HostIntegrationFileKind, HostKind, MANAGED_WRAPPER_ENV, MANAGED_WRAPPER_VALUE,
    },
};

pub(crate) type GuardCommandSpec = GuardCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostHookCommand {
    pub(crate) host_kind: HostKind,
    pub(crate) phase: GuardHookPhase,
    pub(crate) purpose: HostHookPurpose,
    pub(crate) generated_command_shape: HostHookCommandShape,
    pub(crate) expected_wrapper_path: PathBuf,
    pub(crate) expected_phase_wrapper_path: PathBuf,
    pub(crate) root_resolution_basis: HookRootResolutionBasis,
    pub(crate) hook_command_path_basis: HookCommandPathBasis,
    pub(crate) cwd_independent: bool,
    pub(crate) subdirectory_safe: bool,
    pub(crate) wrapper_resolution_status: HookWrapperResolutionStatus,
    pub(crate) verification: HostHookCommandVerification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostHookPurpose {
    Guard,
}

impl HostHookPurpose {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Guard => "guard",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HostHookCommandShape {
    ShellCommandString {
        command_text: String,
        argv: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookRootResolutionBasis {
    GitWorkTree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookCommandPathBasis {
    GitRootRuntime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostHookCommandVerification {
    pub(crate) basis_verified_by: String,
    pub(crate) host_contract_source: String,
}

pub(crate) fn plan_hook_wrapper_files(
    repo_root: &Path,
    runtime_home: &Path,
    host_kind: HostKind,
    guard_commands: &GuardCommandSet,
    phases: &[GuardHookPhase],
    purpose: HostHookPurpose,
) -> Result<Vec<GeneratedFilePlan>, GuardIntegrationError> {
    phases
        .iter()
        .map(|phase| {
            let guard_command = guard_commands.get(*phase);
            plan_hook_wrapper_file(
                repo_root,
                runtime_home,
                host_kind,
                *phase,
                purpose,
                guard_command,
            )
        })
        .collect()
}

pub(crate) fn plan_hook_wrapper_file(
    repo_root: &Path,
    runtime_home: &Path,
    host_kind: HostKind,
    phase: GuardHookPhase,
    purpose: HostHookPurpose,
    guard_command: &GuardCommandSpec,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let invocation =
        GuardCommandInvocation::from_runtime_command(guard_command).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "generated Guard runtime command is malformed: {error}"
            ))
        })?;
    if invocation.phase != phase || invocation.host_kind != host_kind {
        return Err(GuardIntegrationError::runtime(
            "generated Guard runtime command does not match its wrapper phase and host",
        ));
    }
    let relative_path = hook_wrapper_relative_path(host_kind, phase)?;
    let path = repo_root.join(&relative_path);
    let content = hook_wrapper_script_content(runtime_home, purpose, &invocation);
    plan_managed_script_file(
        repo_root,
        &path,
        &content,
        HostIntegrationFileKind::HostHookWrapper,
    )
}

pub(crate) fn plan_codex_dispatch_wrapper_file(
    repo_root: &Path,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let path = repo_root.join(codex_dispatch_wrapper_relative_path());
    let content = codex_dispatch_wrapper_script_content();
    plan_managed_script_file(
        repo_root,
        &path,
        &content,
        HostIntegrationFileKind::HostHookDispatch,
    )
}

pub(crate) fn host_hook_command_specs(
    host_kind: HostKind,
    repo_root: &Path,
    phases: &[GuardHookPhase],
    purpose: HostHookPurpose,
) -> Result<BTreeMap<String, HostHookCommand>, GuardIntegrationError> {
    if host_kind == HostKind::Codex && !codex_hook_root_available(repo_root)? {
        return Err(GuardIntegrationError::runtime(
            hook_root_unsupported_message(host_kind, repo_root, purpose),
        ));
    }
    phases
        .iter()
        .copied()
        .map(|phase| {
            let command = host_hook_command_spec(host_kind, repo_root, phase, purpose)?;
            Ok((phase.as_str().to_owned(), command))
        })
        .collect()
}

pub(crate) fn host_hook_command_spec(
    host_kind: HostKind,
    repo_root: &Path,
    phase: GuardHookPhase,
    purpose: HostHookPurpose,
) -> Result<HostHookCommand, GuardIntegrationError> {
    let relative_path = hook_wrapper_relative_path(host_kind, phase)?;
    match host_kind {
        HostKind::Codex => {
            let dispatch_relative = codex_dispatch_wrapper_relative_path();
            let expected_phase_wrapper_path = repo_root.join(&relative_path);
            let expected_wrapper_path = repo_root.join(&dispatch_relative);
            let script = codex_guard_hook_script(phase);
            Ok(HostHookCommand {
                host_kind,
                phase,
                purpose,
                generated_command_shape: HostHookCommandShape::ShellCommandString {
                    command_text: format!("sh -c {}", shell_word(&script)),
                    argv: vec!["sh".to_owned(), "-c".to_owned(), script],
                },
                expected_wrapper_path,
                expected_phase_wrapper_path,
                root_resolution_basis: HookRootResolutionBasis::GitWorkTree,
                hook_command_path_basis: HookCommandPathBasis::GitRootRuntime,
                cwd_independent: true,
                subdirectory_safe: true,
                wrapper_resolution_status: HookWrapperResolutionStatus::Ok,
                verification: HostHookCommandVerification {
                    basis_verified_by: "repo_root_git_marker".to_owned(),
                    host_contract_source: "codex_hook_command_string".to_owned(),
                },
            })
        }
    }
}

pub(crate) fn guard_command_specs(
    volicord_command: &Path,
    repo_root: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    host_kind: HostKind,
    profile: IntegrationProfile,
    policy_hash: Option<&PolicyHash>,
) -> Result<GuardCommandSet, GuardIntegrationError> {
    let invocations = GuardCommandInvocationSet::new(
        GuardCommandAbsolutePath::from_path(volicord_command).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "generated Guard executable path is invalid: {error}"
            ))
        })?,
        GuardCommandAbsolutePath::from_path(repo_root).map_err(|error| {
            GuardIntegrationError::runtime(format!(
                "generated Guard repository path is invalid: {error}"
            ))
        })?,
        AgentConnectionId::new(connection_id),
        GuardInstallationId::new(guard_installation_id),
        host_kind,
        profile,
        policy_hash.cloned(),
        HostKind::Codex,
    )
    .map_err(|error| {
        GuardIntegrationError::runtime(format!("generated Guard command is invalid: {error}"))
    })?;
    let projection = if policy_hash.is_some() {
        GuardCommandProjection::Runtime
    } else {
        GuardCommandProjection::Policy
    };
    invocations.to_commands(projection).map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to serialize generated Guard command: {error}"
        ))
    })
}

pub(crate) fn guard_command_specs_json(
    commands: &GuardCommandSet,
) -> Result<Value, GuardIntegrationError> {
    serde_json::to_value(commands)
        .map_err(|error| GuardIntegrationError::runtime(error.to_string()))
}

pub(crate) fn guard_command_line(spec: &GuardCommandSpec) -> String {
    let mut words = Vec::with_capacity(spec.args.len() + 1);
    words.push(shell_word(&spec.command));
    words.extend(spec.args.iter().map(|arg| shell_word(arg)));
    words.join(" ")
}

pub(crate) fn shell_word(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '='))
    {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(crate) fn guard_hook_root_unsupported_message(host_kind: HostKind, repo_root: &Path) -> String {
    format!(
        "GUARD_HOOK_ROOT_UNSUPPORTED: {} record init requires the selected adapter's host-hook configuration to resolve a Git work tree root, but no Git repository root was found from {}.",
        public_host_label(host_kind),
        repo_root.display()
    )
}

fn hook_root_unsupported_message(
    host_kind: HostKind,
    repo_root: &Path,
    purpose: HostHookPurpose,
) -> String {
    let _ = purpose;
    guard_hook_root_unsupported_message(host_kind, repo_root)
}

fn hook_wrapper_relative_path(
    host_kind: HostKind,
    phase: GuardHookPhase,
) -> Result<PathBuf, GuardIntegrationError> {
    let _ = host_kind;
    let base = PathBuf::from(".codex").join("hooks");
    Ok(base.join(format!("volicord-{}.sh", phase.command_name())))
}

fn codex_dispatch_wrapper_relative_path() -> PathBuf {
    PathBuf::from(CODEX_DISPATCH_WRAPPER)
}

pub(crate) fn codex_guard_hook_script(phase: GuardHookPhase) -> String {
    let dispatch_relative_text = path_text(&codex_dispatch_wrapper_relative_path());
    format!(
        "root=$(git rev-parse --show-toplevel) || exit $?; exec \"$root/{dispatch_relative_text}\" {}",
        phase.command_name()
    )
}

pub(crate) fn codex_hook_root_available(repo_root: &Path) -> Result<bool, GuardIntegrationError> {
    repo_root.join(".git").try_exists().map_err(|error| {
        GuardIntegrationError::runtime(format!(
            "failed to inspect Git repository marker {}: {error}",
            repo_root.join(".git").display()
        ))
    })
}

fn hook_wrapper_script_content(
    runtime_home: &Path,
    purpose: HostHookPurpose,
    invocation: &GuardCommandInvocation,
) -> String {
    let guard_command = invocation
        .to_runtime_command()
        .expect("a parsed runtime invocation retains its canonical policy hash");
    let command_line = guard_command_line(&guard_command);
    let connection_id = invocation.connection_id.as_str();
    let guard_installation_id = invocation.guard_installation_id.as_str();
    let policy_hash = invocation
        .policy_hash
        .as_ref()
        .expect("a parsed runtime invocation retains its canonical policy hash")
        .as_str();
    let host_output = invocation.host_output.as_str();
    let runtime_home = shell_word(&path_text(runtime_home));
    format!(
        "#!/bin/sh\n# {HOOK_WRAPPER_MARKER}\n# host_kind={}\n# phase={}\n# purpose={purpose}\n# connection_id={connection_id}\n# guard_installation_id={guard_installation_id}\n# policy_hash={policy_hash}\n# host_output={host_output}\n# runtime_home_binding=selected_init_runtime_home\nVOLICORD_HOME={runtime_home}\n{MANAGED_WRAPPER_ENV}={MANAGED_WRAPPER_VALUE}\nexport VOLICORD_HOME\nexport {MANAGED_WRAPPER_ENV}\nexec {command_line}\n",
        public_host_label(invocation.host_kind),
        invocation.phase.as_str(),
        purpose = purpose.as_str(),
    )
}

fn codex_dispatch_wrapper_script_content() -> String {
    format!(
        concat!(
            "#!/bin/sh\n",
            "# {}\n",
            "# host_kind=codex\n",
            "# phase=dispatch\n",
            "# script_role=codex_dispatch\n",
            "if [ \"$#\" -ne 1 ]; then\n",
            "    printf '%s\\n' 'volicord dispatch: expected one host-hook phase argument' >&2\n",
            "    exit 64\n",
            "fi\n",
            "phase=$1\n",
            "case \"$phase\" in\n",
            "    pre-tool|post-tool|prompt-capture) ;;\n",
            "    *)\n",
            "        printf '%s\\n' \"volicord dispatch: unsupported host-hook phase: $phase\" >&2\n",
            "        exit 64\n",
            "        ;;\n",
            "esac\n",
            "root=$(git rev-parse --show-toplevel 2>/dev/null) || {{\n",
            "    printf '%s\\n' 'volicord dispatch: failed to resolve Git work-tree root' >&2\n",
            "    exit 70\n",
            "}}\n",
            "case \"$root\" in\n",
            "    /*) ;;\n",
            "    *)\n",
            "        printf '%s\\n' 'volicord dispatch: resolved Git work-tree root is not absolute' >&2\n",
            "        exit 70\n",
            "        ;;\n",
            "esac\n",
            "wrapper=\"$root/.codex/hooks/volicord-$phase.sh\"\n",
            "if [ ! -f \"$wrapper\" ]; then\n",
            "    printf '%s\\n' \"volicord dispatch: missing phase wrapper: $wrapper\" >&2\n",
            "    exit 70\n",
            "fi\n",
            "if [ ! -x \"$wrapper\" ]; then\n",
            "    printf '%s\\n' \"volicord dispatch: phase wrapper is not executable: $wrapper\" >&2\n",
            "    exit 70\n",
            "fi\n",
            "exec \"$wrapper\"\n",
        ),
        HOOK_WRAPPER_MARKER
    )
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}
