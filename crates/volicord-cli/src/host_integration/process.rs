use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use sha2::{Digest, Sha256};
use volicord_platform_fs::{
    observe_local_platform_boundary, observe_path_filesystem, LocalPlatformBoundary,
    PathFilesystemKind,
};
use volicord_types::{
    validate_canonical_platform_path, PlatformEnvironment, PlatformReleaseCoordinate,
    ProcessBinding,
};

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
    pub process_binding: ProcessBinding,
    pub platform_environment: PlatformEnvironment,
    pub platform_release_coordinate: PlatformReleaseCoordinate,
}

pub trait CommandRunner {
    fn run(&mut self, invocation: &CommandInvocation) -> Result<CommandOutput, String>;
}

#[derive(Debug, Default, Clone)]
pub struct ProductionCommandRunner;

impl CommandRunner for ProductionCommandRunner {
    fn run(&mut self, invocation: &CommandInvocation) -> Result<CommandOutput, String> {
        let platform_boundary = detect_platform_boundary()?;
        let platform_environment = platform_boundary.environment;
        let executable_path =
            canonical_existing_platform_path(Path::new(&invocation.program), platform_environment)?;
        let executable_digest = sha256_file(Path::new(&invocation.program))?;
        let mut command = Command::new(Path::new(&invocation.program));
        command.args(&invocation.args);
        if let Some(cwd) = &invocation.cwd {
            command.current_dir(cwd);
        }
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|error| {
            format!(
                "failed to run {} {}: {error}",
                invocation.program,
                invocation.args.join(" ")
            )
        })?;
        let process_id = u64::from(child.id());
        let process_start_token = match process_start_token(child.id()) {
            Ok(token) => token,
            Err(error) => {
                terminate_failed_observation(&mut child);
                return Err(error);
            }
        };
        let platform_instance_token = match platform_instance_token() {
            Ok(token) => token,
            Err(error) => {
                terminate_failed_observation(&mut child);
                return Err(error);
            }
        };
        let output = child.wait_with_output().map_err(|error| {
            format!(
                "failed to collect {} {} output: {error}",
                invocation.program,
                invocation.args.join(" ")
            )
        })?;
        Ok(CommandOutput {
            success: output.status.success(),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            process_binding: ProcessBinding {
                process_id,
                process_start_token,
                platform_instance_token,
                executable_path,
                executable_digest,
            },
            platform_environment,
            platform_release_coordinate: platform_boundary.release_coordinate,
        })
    }
}

fn terminate_failed_observation(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub(crate) fn detect_platform_environment() -> Result<PlatformEnvironment, String> {
    detect_platform_boundary().map(|boundary| boundary.environment)
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

pub(crate) fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "executable_digest_unavailable for {}: {error}",
            path.display()
        )
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(target_os = "linux")]
fn process_start_token(process_id: u32) -> Result<String, String> {
    let stat = fs::read_to_string(format!("/proc/{process_id}/stat"))
        .map_err(|error| format!("process_start_token_unavailable: {error}"))?;
    let closing = stat
        .rfind(')')
        .ok_or_else(|| "process_start_token_invalid".to_owned())?;
    let start_time = stat[closing + 1..]
        .split_whitespace()
        .nth(19)
        .ok_or_else(|| "process_start_token_invalid".to_owned())?;
    if !start_time.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("process_start_token_invalid".to_owned());
    }
    Ok(format!("linux-proc-start:{start_time}"))
}

#[cfg(target_os = "linux")]
fn platform_instance_token() -> Result<String, String> {
    let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map_err(|error| format!("platform_instance_token_unavailable: {error}"))?;
    let boot_id = boot_id.trim();
    if boot_id.is_empty() || boot_id.chars().any(char::is_control) {
        return Err("platform_instance_token_invalid".to_owned());
    }
    Ok(format!("linux-boot-id:{boot_id}"))
}

#[cfg(target_os = "macos")]
fn process_start_token(process_id: u32) -> Result<String, String> {
    let output = Command::new("/bin/ps")
        .args(["-o", "lstart=", "-p", &process_id.to_string()])
        .output()
        .map_err(|error| format!("process_start_token_unavailable: {error}"))?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err("process_start_token_unavailable".to_owned());
    }
    bounded_token("macos-process-start", &output.stdout)
}

#[cfg(target_os = "macos")]
fn platform_instance_token() -> Result<String, String> {
    let output = Command::new("/usr/sbin/sysctl")
        .args(["-n", "kern.boottime"])
        .output()
        .map_err(|error| format!("platform_instance_token_unavailable: {error}"))?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err("platform_instance_token_unavailable".to_owned());
    }
    bounded_token("macos-boot-time", &output.stdout)
}

#[cfg(any(target_os = "macos", windows))]
fn bounded_token(prefix: &str, bytes: &[u8]) -> Result<String, String> {
    let value = std::str::from_utf8(bytes)
        .map_err(|_| "process_token_not_utf8".to_owned())?
        .trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err("process_token_invalid".to_owned());
    }
    Ok(format!("{prefix}:{value}"))
}

#[cfg(windows)]
fn process_start_token(process_id: u32) -> Result<String, String> {
    let script = format!(
        "$p = Get-Process -Id {process_id} -ErrorAction Stop; [Console]::Out.Write($p.StartTime.ToUniversalTime().Ticks)"
    );
    let output = windows_powershell_output(&script, "process_start_token_unavailable")?;
    bounded_token("windows-process-filetime", &output)
}

#[cfg(windows)]
fn platform_instance_token() -> Result<String, String> {
    let script = "$os = Get-CimInstance Win32_OperatingSystem -ErrorAction Stop; [Console]::Out.Write($os.LastBootUpTime.ToUniversalTime().Ticks)";
    let output = windows_powershell_output(script, "platform_instance_token_unavailable")?;
    bounded_token("windows-boot-filetime", &output)
}

#[cfg(windows)]
fn windows_powershell_output(script: &str, reason: &str) -> Result<Vec<u8>, String> {
    let system_root = std::env::var_os("SystemRoot")
        .ok_or_else(|| format!("{reason}: SystemRoot is unavailable"))?;
    let powershell = Path::new(&system_root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let output = Command::new(powershell)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .output()
        .map_err(|error| format!("{reason}: {error}"))?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(reason.to_owned());
    }
    Ok(output.stdout)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn process_start_token(_process_id: u32) -> Result<String, String> {
    Err("unsupported_platform_environment".to_owned())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform_instance_token() -> Result<String, String> {
    Err("unsupported_platform_environment".to_owned())
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
