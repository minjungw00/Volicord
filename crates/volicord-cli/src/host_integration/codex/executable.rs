use std::{cell::RefCell, ffi::OsString, path::PathBuf};

use crate::host_integration::process::{CommandInvocation, CommandRunner};
use crate::host_integration::verification::HostExecutableStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexExecutableAvailability {
    pub(super) status: HostExecutableStatus,
    pub(super) executable_path: Option<String>,
    pub(super) host_version: Option<String>,
    pub(super) code: String,
    pub(super) details: String,
}

impl CodexExecutableAvailability {
    fn available(executable_path: String, host_version: String, details: String) -> Self {
        Self {
            status: HostExecutableStatus::Available,
            executable_path: Some(executable_path),
            host_version: Some(host_version),
            code: "host_executable_available".to_owned(),
            details,
        }
    }

    fn unavailable(code: &str, executable_path: Option<String>, details: String) -> Self {
        Self {
            status: HostExecutableStatus::Unavailable,
            executable_path,
            host_version: None,
            code: code.to_owned(),
            details,
        }
    }

    pub(super) fn is_available(&self) -> bool {
        self.status == HostExecutableStatus::Available
    }
}

pub(super) fn codex_executable_availability<R: CommandRunner>(
    runner: &RefCell<R>,
    path: Option<&OsString>,
) -> CodexExecutableAvailability {
    let Some(launcher) = find_executable_in_path("codex", path) else {
        return CodexExecutableAvailability::unavailable(
            "host_executable_not_found",
            None,
            "Codex executable `codex` was not found on PATH".to_owned(),
        );
    };
    let invocation = CommandInvocation {
        program: launcher.display().to_string(),
        args: vec!["--version".to_owned()],
        cwd: None,
    };
    match runner.borrow_mut().run(&invocation) {
        Ok(output) if output.success => {
            let Some(version) = bounded_codex_version_output(&output.stdout, &output.stderr) else {
                return CodexExecutableAvailability::unavailable(
                    "host_executable_version_invalid",
                    Some(output.executable_path.clone()),
                    format!(
                        "Codex executable {} returned an invalid `codex --version` response",
                        output.executable_path
                    ),
                );
            };
            CodexExecutableAvailability::available(
                output.executable_path.clone(),
                version,
                format!(
                    "Codex executable discovery and `codex --version` succeeded; executable: {}; version: {}",
                    output.executable_path,
                    bounded_codex_version_output(&output.stdout, &output.stderr)
                        .expect("version was validated")
                ),
            )
        }
        Ok(output) => CodexExecutableAvailability::unavailable(
            "host_executable_probe_failed",
            Some(output.executable_path.clone()),
            format!(
                "Codex executable {} failed `codex --version` with status {}",
                output.executable_path,
                status_text(output.status_code)
            ),
        ),
        Err(error) => CodexExecutableAvailability::unavailable(
            "host_executable_probe_failed",
            Some(launcher.display().to_string()),
            format!("Codex executable could not run `codex --version`: {error}"),
        ),
    }
}

fn bounded_codex_version_output(stdout: &str, stderr: &str) -> Option<String> {
    let output = if stdout.trim().is_empty() {
        stderr
    } else {
        stdout
    };
    let envelope = output.trim_end_matches(['\n', '\r']);
    if envelope.is_empty()
        || envelope.len() > 256
        || envelope.contains(['\n', '\r'])
        || envelope.chars().any(char::is_control)
    {
        return None;
    }
    let version = envelope.strip_prefix("codex-cli ").unwrap_or(envelope);
    (!version.is_empty()).then(|| version.to_owned())
}

fn status_text(status_code: Option<i32>) -> String {
    status_code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "without exit status".to_owned())
}

fn find_executable_in_path(program: &str, path: Option<&OsString>) -> Option<PathBuf> {
    let path = path.cloned().or_else(|| std::env::var_os("PATH"))?;
    for directory in std::env::split_paths(&path) {
        #[cfg(windows)]
        let candidates = [
            directory.join(program),
            directory.join(format!("{program}.exe")),
            directory.join(format!("{program}.cmd")),
            directory.join(format!("{program}.bat")),
        ];
        #[cfg(not(windows))]
        let candidates = [directory.join(program)];

        for candidate in candidates {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::bounded_codex_version_output;

    #[test]
    fn accepts_arbitrary_bounded_codex_version_text() {
        assert_eq!(
            bounded_codex_version_output("codex-cli 999.0-future+build\n", ""),
            Some("999.0-future+build".to_owned())
        );
        assert_eq!(
            bounded_codex_version_output("Codex preview channel 42\n", ""),
            Some("Codex preview channel 42".to_owned())
        );
    }

    #[test]
    fn accepts_successful_stderr_output_but_rejects_multiline_envelopes() {
        assert_eq!(
            bounded_codex_version_output("codex-cli 1\nextra\n", ""),
            None
        );
        assert_eq!(
            bounded_codex_version_output("codex-cli 1\n", "warning"),
            Some("1".to_owned())
        );
        assert_eq!(
            bounded_codex_version_output("", "codex-cli future\n"),
            Some("future".to_owned())
        );
    }
}
