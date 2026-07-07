use std::{
    cell::RefCell,
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::host_integration::claude_code::{CommandInvocation, CommandRunner};
use crate::host_integration::verification::{
    HostConfigurationStatus, HostExecutableStatus, HostGateStatus, Verification,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexExecutableAvailability {
    pub(super) status: HostExecutableStatus,
    pub(super) details: String,
    pub(super) diagnostic: Option<String>,
}

impl CodexExecutableAvailability {
    fn available(details: String) -> Self {
        Self {
            status: HostExecutableStatus::Available,
            details,
            diagnostic: None,
        }
    }

    fn unavailable(details: String, diagnostic: impl Into<String>) -> Self {
        Self {
            status: HostExecutableStatus::Unavailable,
            details,
            diagnostic: Some(diagnostic.into()),
        }
    }

    pub(super) fn is_available(&self) -> bool {
        self.status == HostExecutableStatus::Available
    }
}

pub(super) fn codex_executable_availability<R: CommandRunner>(
    runner: &RefCell<R>,
    path: Option<&OsString>,
    config_target: &Path,
) -> CodexExecutableAvailability {
    let Some(executable) = find_executable_in_path("codex", path) else {
        return CodexExecutableAvailability::unavailable(
            format!(
                "Codex executable `codex` was not found on PATH; install Codex or make it available before using this Agent Connection; configuration target: {}",
                config_target.display()
            ),
            "Codex executable `codex` was not found on PATH",
        );
    };
    let invocation = CommandInvocation {
        program: executable.display().to_string(),
        args: vec!["--version".to_owned()],
        cwd: None,
    };
    match runner.borrow_mut().run(&invocation) {
        Ok(output) if output.success => CodexExecutableAvailability::available(format!(
            "Codex executable availability check succeeded with `codex --version`; executable: {}; configuration target: {}",
            executable.display(),
            config_target.display()
        )),
        Ok(output) => CodexExecutableAvailability::unavailable(
            format!(
                "Codex executable failed its availability check `codex --version` with status {}; install or repair Codex before using this Agent Connection; configuration target: {}",
                status_text(output.status_code),
                config_target.display()
            ),
            format!(
                "Codex executable availability check failed with status {}",
                status_text(output.status_code)
            ),
        ),
        Err(error) => CodexExecutableAvailability::unavailable(
            format!(
                "Codex executable could not be launched for availability check `codex --version`: {error}; install Codex or make it executable before using this Agent Connection; configuration target: {}",
                config_target.display()
            ),
            format!("Codex executable availability check could not launch: {error}"),
        ),
    }
}

fn status_text(status_code: Option<i32>) -> String {
    status_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "without exit status".to_owned())
}

pub(super) fn verification_from_executable_unavailable(
    executable: CodexExecutableAvailability,
) -> Verification {
    let mut verification = Verification::action_required(executable.details)
        .with_host_executable(HostExecutableStatus::Unavailable)
        .with_host_gate(HostGateStatus::ActionRequired)
        .with_host_configuration(HostConfigurationStatus::Discovered)
        .with_mcp_handshake_allowed(false);
    if let Some(diagnostic) = executable.diagnostic {
        verification = verification.with_diagnostic(diagnostic);
    }
    verification
}

fn find_executable_in_path(program: &str, path: Option<&OsString>) -> Option<PathBuf> {
    let path = path.cloned().or_else(|| std::env::var_os("PATH"))?;
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
