use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use volicord_types::IntegrationProfile;

use crate::{
    guard_integration::{
        audit::{CODEX_DISPATCH_WRAPPER, HOOK_WRAPPER_MARKER},
        files::{plan_managed_script_file, GeneratedFilePlan},
        public_host_label, GuardIntegrationError, HookWrapperResolutionStatus,
    },
    host_integration::{
        HostIntegrationFileKind, HostKind, HostLifecyclePhase, MANAGED_WRAPPER_ENV,
        MANAGED_WRAPPER_VALUE, REQUIRED_GUARD_PHASES,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct GuardCommandSpec {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostHookCommand {
    pub(crate) host_kind: HostKind,
    pub(crate) phase: HostLifecyclePhase,
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

impl HostHookCommand {
    pub(crate) fn command_shape_name(&self) -> &'static str {
        match &self.generated_command_shape {
            HostHookCommandShape::ShellCommandString { .. } => "shell_command_string",
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

impl HookRootResolutionBasis {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::GitWorkTree => "git_work_tree",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookCommandPathBasis {
    GitRootRuntime,
}

impl HookCommandPathBasis {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::GitRootRuntime => "git_root_runtime",
        }
    }
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
    guard_commands: &BTreeMap<String, GuardCommandSpec>,
    phases: &[HostLifecyclePhase],
    purpose: HostHookPurpose,
) -> Result<Vec<GeneratedFilePlan>, GuardIntegrationError> {
    phases
        .iter()
        .map(|phase| {
            let guard_command = guard_commands.get(phase.policy_key()).ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "missing generated host-hook command for {}",
                    phase.policy_key()
                ))
            })?;
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
    phase: HostLifecyclePhase,
    purpose: HostHookPurpose,
    guard_command: &GuardCommandSpec,
) -> Result<GeneratedFilePlan, GuardIntegrationError> {
    let relative_path = hook_wrapper_relative_path(host_kind, phase)?;
    let path = repo_root.join(&relative_path);
    let content =
        hook_wrapper_script_content(runtime_home, host_kind, phase, purpose, guard_command);
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
    phases: &[HostLifecyclePhase],
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
            Ok((phase.policy_key().to_owned(), command))
        })
        .collect()
}

pub(crate) fn host_hook_command_spec(
    host_kind: HostKind,
    repo_root: &Path,
    phase: HostLifecyclePhase,
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
    policy_hash: Option<&str>,
) -> BTreeMap<String, GuardCommandSpec> {
    REQUIRED_GUARD_PHASES
        .into_iter()
        .map(|phase| {
            let mut args = vec![
                "_hook".to_owned(),
                phase.command_name().to_owned(),
                "--repo".to_owned(),
                path_text(repo_root),
                "--connection".to_owned(),
                connection_id.to_owned(),
                "--guard-installation".to_owned(),
                guard_installation_id.to_owned(),
                "--host".to_owned(),
                public_host_label(host_kind).to_owned(),
                "--integration-profile".to_owned(),
                profile.as_str().to_owned(),
            ];
            if let Some(policy_hash) = policy_hash {
                args.push("--policy-hash".to_owned());
                args.push(policy_hash.to_owned());
            }
            args.push("--host-output".to_owned());
            args.push("codex".to_owned());
            (
                phase.policy_key().to_owned(),
                GuardCommandSpec {
                    command: path_text(volicord_command),
                    args,
                },
            )
        })
        .collect()
}

pub(crate) fn guard_command_specs_json(
    commands: &BTreeMap<String, GuardCommandSpec>,
) -> Result<Value, GuardIntegrationError> {
    if commands.len() != REQUIRED_GUARD_PHASES.len() {
        return Err(GuardIntegrationError::runtime(
            "Guard command serialization requires the exact Guard phases",
        ));
    }
    let commands = REQUIRED_GUARD_PHASES
        .iter()
        .map(|phase| {
            let spec = commands.get(phase.policy_key()).ok_or_else(|| {
                GuardIntegrationError::runtime(format!(
                    "Guard command serialization requires {}",
                    phase.policy_key()
                ))
            })?;
            Ok((
                phase.policy_key().to_owned(),
                json!({
                    "command": &spec.command,
                    "args": &spec.args,
                }),
            ))
        })
        .collect::<Result<serde_json::Map<_, _>, GuardIntegrationError>>()?;
    Ok(Value::Object(commands))
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
    phase: HostLifecyclePhase,
) -> Result<PathBuf, GuardIntegrationError> {
    let _ = host_kind;
    let base = PathBuf::from(".codex").join("hooks");
    Ok(base.join(format!("volicord-{}.sh", phase.command_name())))
}

fn codex_dispatch_wrapper_relative_path() -> PathBuf {
    PathBuf::from(CODEX_DISPATCH_WRAPPER)
}

pub(crate) fn codex_guard_hook_script(phase: HostLifecyclePhase) -> String {
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
    host_kind: HostKind,
    phase: HostLifecyclePhase,
    purpose: HostHookPurpose,
    guard_command: &GuardCommandSpec,
) -> String {
    let command_line = guard_command_line(guard_command);
    let connection_id = arg_after(&guard_command.args, "--connection").unwrap_or("unknown");
    let guard_installation_id =
        arg_after(&guard_command.args, "--guard-installation").unwrap_or("unknown");
    let policy_hash = arg_after(&guard_command.args, "--policy-hash").unwrap_or("unknown");
    let host_output = arg_after(&guard_command.args, "--host-output").unwrap_or("none");
    let runtime_home = shell_word(&path_text(runtime_home));
    format!(
        "#!/bin/sh\n# {HOOK_WRAPPER_MARKER}\n# host_kind={}\n# phase={}\n# purpose={purpose}\n# connection_id={connection_id}\n# guard_installation_id={guard_installation_id}\n# policy_hash={policy_hash}\n# host_output={host_output}\n# runtime_home_binding=selected_init_runtime_home\nVOLICORD_HOME={runtime_home}\n{MANAGED_WRAPPER_ENV}={MANAGED_WRAPPER_VALUE}\nexport VOLICORD_HOME\nexport {MANAGED_WRAPPER_ENV}\nexec {command_line}\n",
        public_host_label(host_kind),
        phase.policy_key(),
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

fn arg_after<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}
