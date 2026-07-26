use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use volicord_platform_fs::{
    observe_local_platform_boundary, observe_path_filesystem, LocalPlatformBoundary,
    PathFilesystemKind,
};
use volicord_types::platform::{validate_canonical_platform_path, PlatformEnvironment};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub executable_path: String,
}

pub trait CommandRunner {
    fn run(&mut self, invocation: &CommandInvocation) -> Result<CommandOutput, String>;
}

#[derive(Debug, Default, Clone)]
pub struct ProductionCommandRunner;

impl CommandRunner for ProductionCommandRunner {
    fn run(&mut self, invocation: &CommandInvocation) -> Result<CommandOutput, String> {
        let platform = detect_platform_boundary()?.environment;
        let executable_path =
            canonical_existing_platform_path(Path::new(&invocation.program), platform)?;
        let mut command = Command::new(Path::new(&invocation.program));
        command.args(&invocation.args);
        if let Some(cwd) = &invocation.cwd {
            command.current_dir(cwd);
        }
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let output = command.output().map_err(|error| {
            format!(
                "failed to run {} {}: {error}",
                invocation.program,
                invocation.args.join(" ")
            )
        })?;
        Ok(CommandOutput {
            success: output.status.success(),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            executable_path,
        })
    }
}

pub(crate) fn detect_platform_boundary() -> Result<LocalPlatformBoundary, String> {
    let boundary = observe_local_platform_boundary().map_err(|error| error.to_string())?;
    if boundary.environment == PlatformEnvironment::Wsl2 {
        let current_exe = std::env::current_exe()
            .map_err(|error| format!("volicord_executable_unavailable: {error}"))?;
        require_supported_path_filesystem(&current_exe, boundary.environment)?;
    }
    Ok(boundary)
}

pub(crate) fn canonical_existing_platform_path(
    path: &Path,
    platform: PlatformEnvironment,
) -> Result<String, String> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        format!(
            "canonical_platform_path_unavailable for {}: {error}",
            path.display()
        )
    })?;
    let text = platform_path_text(&canonical, platform)?;
    require_supported_path_filesystem(&canonical, platform)?;
    validate_canonical_platform_path(platform, &text).map_err(|error| error.reason().to_owned())?;
    Ok(text)
}

fn require_supported_path_filesystem(
    path: &Path,
    platform: PlatformEnvironment,
) -> Result<(), String> {
    if platform != PlatformEnvironment::Wsl2 {
        return Ok(());
    }
    let filesystem = observe_path_filesystem(path).map_err(|error| error.to_string())?;
    validate_path_filesystem_fact(platform, filesystem).map_err(|reason| {
        format!(
            "{reason}: {} must be on the WSL2 distribution ext4 filesystem",
            path.display()
        )
    })
}

fn validate_path_filesystem_fact(
    platform: PlatformEnvironment,
    filesystem: PathFilesystemKind,
) -> Result<(), &'static str> {
    if platform == PlatformEnvironment::Wsl2 && filesystem != PathFilesystemKind::LinuxExt4 {
        Err("unsupported_wsl2_filesystem")
    } else {
        Ok(())
    }
}

fn platform_path_text(path: &Path, platform: PlatformEnvironment) -> Result<String, String> {
    let text = path
        .to_str()
        .ok_or_else(|| "canonical_platform_path_not_utf8".to_owned())?;
    if platform != PlatformEnvironment::NativeWindows {
        return Ok(text.to_owned());
    }
    let text = text
        .strip_prefix(r"\\?\")
        .ok_or_else(|| "canonical_native_windows_path_missing_extended_prefix".to_owned())?;
    if text.starts_with("UNC\\") {
        return Err("canonical_native_windows_unc_path_unsupported".to_owned());
    }
    let mut normalized = text.replace('\\', "/");
    if let Some(first) = normalized.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injected_filesystem_facts_reject_non_ext4_only_for_wsl2() {
        assert_eq!(
            validate_path_filesystem_fact(PlatformEnvironment::Wsl2, PathFilesystemKind::Other),
            Err("unsupported_wsl2_filesystem")
        );
        validate_path_filesystem_fact(PlatformEnvironment::Wsl2, PathFilesystemKind::LinuxExt4)
            .expect("WSL2 ext4 should be supported");
        for platform in [
            PlatformEnvironment::Linux,
            PlatformEnvironment::Macos,
            PlatformEnvironment::NativeWindows,
        ] {
            validate_path_filesystem_fact(platform, PathFilesystemKind::Other)
                .expect("native platform filesystem policy remains platform-native");
        }
    }
}
