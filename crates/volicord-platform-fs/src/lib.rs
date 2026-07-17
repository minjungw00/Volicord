//! Safe platform filesystem primitives used by Volicord's local adapters.

#![deny(unsafe_code)]

use std::{
    fmt, fs, io,
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};
use volicord_types::{
    validate_canonical_platform_path, PlatformEnvironment, PlatformReleaseCoordinate,
    ProcessBinding, ReleaseTargetTriple, PINNED_WSL2_DISTRIBUTION_ID,
    PINNED_WSL2_DISTRIBUTION_NAME, PINNED_WSL2_DISTRIBUTION_VERSION,
};

#[cfg(windows)]
use std::fs::File;

const MAX_GIT_CONTROL_FILE_BYTES: u64 = 4096;
const MAX_PLATFORM_CONTROL_FILE_BYTES: u64 = 16 * 1024;
const MAX_MOUNTINFO_BYTES: u64 = 4 * 1024 * 1024;

/// One observed local process-platform boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPlatformBoundary {
    /// Exact Volicord binary target executing in this process.
    pub target_triple: ReleaseTargetTriple,
    /// Exact independent release environment.
    pub environment: PlatformEnvironment,
    /// Exact native or pinned WSL2 release coordinate.
    pub release_coordinate: PlatformReleaseCoordinate,
}

/// Machine-readable failure while observing the live MCP parent process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessBindingObservationError {
    reason: &'static str,
    detail: String,
}

impl ProcessBindingObservationError {
    /// Returns the stable failure reason.
    pub const fn reason(&self) -> &'static str {
        self.reason
    }

    /// Returns bounded implementation-facing detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for ProcessBindingObservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.reason, self.detail)
    }
}

impl std::error::Error for ProcessBindingObservationError {}

/// Filesystem kind observed for one canonical path or its nearest existing ancestor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathFilesystemKind {
    /// Linux ext-family filesystem used as the WSL2 distribution ext4 boundary.
    LinuxExt4,
    /// A filesystem outside the supported WSL2 ext4 boundary.
    Other,
}

/// Stable class for a local platform-boundary observation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformBoundaryErrorKind {
    /// Required local observation could not be completed.
    Unavailable,
    /// The observed platform or topology is outside the first-release contract.
    Unsupported,
}

/// Machine-readable local platform-boundary observation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformBoundaryError {
    kind: PlatformBoundaryErrorKind,
    reason: &'static str,
    detail: String,
}

impl PlatformBoundaryError {
    /// Returns the stable failure class.
    pub const fn kind(&self) -> PlatformBoundaryErrorKind {
        self.kind
    }

    /// Returns the stable machine-readable reason.
    pub const fn reason(&self) -> &'static str {
        self.reason
    }

    /// Returns bounded implementation-facing detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for PlatformBoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.reason, self.detail)
    }
}

impl std::error::Error for PlatformBoundaryError {}

/// Observes the current native or exact pinned WSL2 process boundary.
#[cfg(target_os = "linux")]
pub fn observe_local_platform_boundary() -> Result<LocalPlatformBoundary, PlatformBoundaryError> {
    let target_triple = current_release_target_triple()?;
    let kernel_release = read_platform_text("/proc/sys/kernel/osrelease")?;
    let wsl_distribution_name = std::env::var("WSL_DISTRO_NAME").ok();
    classify_linux_platform_boundary(
        target_triple,
        LinuxPlatformFacts {
            kernel_release: &kernel_release,
            wsl_distribution_name: wsl_distribution_name.as_deref(),
            os_release: if kernel_release.to_ascii_lowercase().contains("microsoft") {
                Some(read_platform_text("/etc/os-release")?)
            } else {
                None
            },
        },
    )
}

/// Observes the current native or exact pinned WSL2 process boundary.
#[cfg(target_os = "macos")]
pub fn observe_local_platform_boundary() -> Result<LocalPlatformBoundary, PlatformBoundaryError> {
    let target_triple = current_release_target_triple()?;
    Ok(LocalPlatformBoundary {
        target_triple,
        environment: PlatformEnvironment::Macos,
        release_coordinate: PlatformReleaseCoordinate::Native,
    })
}

/// Observes the current native or exact pinned WSL2 process boundary.
#[cfg(windows)]
pub fn observe_local_platform_boundary() -> Result<LocalPlatformBoundary, PlatformBoundaryError> {
    let target_triple = current_release_target_triple()?;
    Ok(LocalPlatformBoundary {
        target_triple,
        environment: PlatformEnvironment::NativeWindows,
        release_coordinate: PlatformReleaseCoordinate::Native,
    })
}

/// Observes the current native or exact pinned WSL2 process boundary.
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub fn observe_local_platform_boundary() -> Result<LocalPlatformBoundary, PlatformBoundaryError> {
    Err(unsupported_platform(
        "unsupported_platform_environment",
        "this operating-system target has no first-release platform cell",
    ))
}

/// Observes the filesystem containing a path or its nearest existing ancestor.
#[cfg(target_os = "linux")]
pub fn observe_path_filesystem(path: &Path) -> Result<PathFilesystemKind, PlatformBoundaryError> {
    let existing = nearest_existing_path(path)?;
    let canonical_existing = fs::canonicalize(&existing).map_err(|error| {
        unavailable_platform(
            "platform_filesystem_unavailable",
            format!("cannot canonicalize {}: {error}", existing.display()),
        )
    })?;
    let stat = rustix::fs::statfs(&canonical_existing).map_err(|error| {
        unavailable_platform(
            "platform_filesystem_unavailable",
            format!("cannot inspect filesystem for {}: {error}", path.display()),
        )
    })?;
    if stat.f_type != 0x0000_ef53 {
        return Ok(PathFilesystemKind::Other);
    }
    let mountinfo = read_bounded_platform_text(
        "/proc/self/mountinfo",
        MAX_MOUNTINFO_BYTES,
        "platform_filesystem_unavailable",
    )?;
    Ok(
        if filesystem_type_for_path(&canonical_existing, &mountinfo)? == "ext4" {
            PathFilesystemKind::LinuxExt4
        } else {
            PathFilesystemKind::Other
        },
    )
}

/// Observes the filesystem containing a path or its nearest existing ancestor.
#[cfg(not(target_os = "linux"))]
pub fn observe_path_filesystem(_path: &Path) -> Result<PathFilesystemKind, PlatformBoundaryError> {
    Ok(PathFilesystemKind::Other)
}

/// Observes the exact live parent process of the current managed stdio MCP process.
pub fn observe_parent_process_binding() -> Result<ProcessBinding, ProcessBindingObservationError> {
    let platform = observe_local_platform_boundary()
        .map_err(|error| process_observation_error(error.reason(), error.detail()))?;
    let parent_id = parent_process_id()?;
    process_binding_for_id(parent_id, platform.environment)
}

#[cfg(target_os = "linux")]
fn parent_process_id() -> Result<u64, ProcessBindingObservationError> {
    let stat = read_process_text(Path::new("/proc/self/stat"), "parent_process_unavailable")?;
    linux_stat_field(&stat, 1, "parent_process_unavailable")?
        .parse::<u64>()
        .map_err(|error| process_observation_error("parent_process_unavailable", error))
        .and_then(require_process_id)
}

#[cfg(target_os = "linux")]
fn process_binding_for_id(
    process_id: u64,
    platform: PlatformEnvironment,
) -> Result<ProcessBinding, ProcessBindingObservationError> {
    let stat_path = PathBuf::from(format!("/proc/{process_id}/stat"));
    let stat = read_process_text(&stat_path, "process_start_token_unavailable")?;
    let start_time = linux_stat_field(&stat, 19, "process_start_token_unavailable")?;
    if !start_time.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(process_observation_error(
            "process_start_token_invalid",
            "Linux process start time is not decimal",
        ));
    }
    let executable = fs::canonicalize(format!("/proc/{process_id}/exe"))
        .map_err(|error| process_observation_error("process_executable_unavailable", error))?;
    let executable_path = canonical_process_path(&executable, platform)?;
    let executable_digest = sha256_regular_file(&executable)?;
    let boot_id = read_process_text(
        Path::new("/proc/sys/kernel/random/boot_id"),
        "platform_instance_token_unavailable",
    )?;
    let boot_id = boot_id.trim();
    if boot_id.is_empty() || boot_id.chars().any(char::is_control) {
        return Err(process_observation_error(
            "platform_instance_token_invalid",
            "Linux boot ID is empty or contains control characters",
        ));
    }
    Ok(ProcessBinding {
        process_id,
        process_start_token: format!("linux-proc-start:{start_time}"),
        platform_instance_token: format!("linux-boot-id:{boot_id}"),
        executable_path,
        executable_digest,
    })
}

#[cfg(target_os = "macos")]
fn parent_process_id() -> Result<u64, ProcessBindingObservationError> {
    command_one_line(
        "/bin/ps",
        &["-o", "ppid=", "-p", &std::process::id().to_string()],
        "parent_process_unavailable",
    )?
    .trim()
    .parse::<u64>()
    .map_err(|error| process_observation_error("parent_process_unavailable", error))
    .and_then(require_process_id)
}

#[cfg(target_os = "macos")]
fn process_binding_for_id(
    process_id: u64,
    platform: PlatformEnvironment,
) -> Result<ProcessBinding, ProcessBindingObservationError> {
    let pid = process_id.to_string();
    let executable_text = command_one_line(
        "/bin/ps",
        &["-o", "comm=", "-p", &pid],
        "process_executable_unavailable",
    )?;
    let executable = fs::canonicalize(executable_text.trim())
        .map_err(|error| process_observation_error("process_executable_unavailable", error))?;
    let executable_path = canonical_process_path(&executable, platform)?;
    let start = command_one_line(
        "/bin/ps",
        &["-o", "lstart=", "-p", &pid],
        "process_start_token_unavailable",
    )?;
    let boot = command_one_line(
        "/usr/sbin/sysctl",
        &["-n", "kern.boottime"],
        "platform_instance_token_unavailable",
    )?;
    Ok(ProcessBinding {
        process_id,
        process_start_token: bounded_process_token("macos-process-start", &start)?,
        platform_instance_token: bounded_process_token("macos-boot-time", &boot)?,
        executable_path,
        executable_digest: sha256_regular_file(&executable)?,
    })
}

#[cfg(windows)]
fn parent_process_id() -> Result<u64, ProcessBindingObservationError> {
    let script = format!(
        "$p=Get-CimInstance Win32_Process -Filter \"ProcessId={}\"; [Console]::Out.Write($p.ParentProcessId)",
        std::process::id()
    );
    command_one_line(
        "powershell.exe",
        &["-NoProfile", "-NonInteractive", "-Command", &script],
        "parent_process_unavailable",
    )?
    .trim()
    .parse::<u64>()
    .map_err(|error| process_observation_error("parent_process_unavailable", error))
    .and_then(require_process_id)
}

#[cfg(windows)]
fn process_binding_for_id(
    process_id: u64,
    platform: PlatformEnvironment,
) -> Result<ProcessBinding, ProcessBindingObservationError> {
    let script = format!(
        "$p=Get-CimInstance Win32_Process -Filter \"ProcessId={process_id}\"; [Console]::Out.WriteLine($p.ExecutablePath); [Console]::Out.Write($p.CreationDate.ToUniversalTime().ToString('O'))"
    );
    let output = command_output(
        "powershell.exe",
        &["-NoProfile", "-NonInteractive", "-Command", &script],
        "process_executable_unavailable",
    )?;
    let mut lines = output.lines();
    let executable_text = lines.next().unwrap_or_default();
    let creation = lines.next().unwrap_or_default();
    if lines.next().is_some() || executable_text.is_empty() || creation.is_empty() {
        return Err(process_observation_error(
            "process_executable_unavailable",
            "Windows process observation returned an invalid envelope",
        ));
    }
    let executable = fs::canonicalize(executable_text)
        .map_err(|error| process_observation_error("process_executable_unavailable", error))?;
    let executable_path = canonical_process_path(&executable, platform)?;
    let boot = command_one_line(
        "powershell.exe",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::Out.Write((Get-CimInstance Win32_OperatingSystem).LastBootUpTime.ToUniversalTime().ToString('O'))",
        ],
        "platform_instance_token_unavailable",
    )?;
    Ok(ProcessBinding {
        process_id,
        process_start_token: bounded_process_token("windows-process-start", creation)?,
        platform_instance_token: bounded_process_token("windows-boot-time", &boot)?,
        executable_path,
        executable_digest: sha256_regular_file(&executable)?,
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn parent_process_id() -> Result<u64, ProcessBindingObservationError> {
    Err(process_observation_error(
        "unsupported_platform_environment",
        "parent-process observation is unavailable on this target",
    ))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn process_binding_for_id(
    _process_id: u64,
    _platform: PlatformEnvironment,
) -> Result<ProcessBinding, ProcessBindingObservationError> {
    Err(process_observation_error(
        "unsupported_platform_environment",
        "process-binding observation is unavailable on this target",
    ))
}

#[cfg(target_os = "linux")]
fn linux_stat_field<'a>(
    stat: &'a str,
    index: usize,
    reason: &'static str,
) -> Result<&'a str, ProcessBindingObservationError> {
    let closing = stat
        .rfind(')')
        .ok_or_else(|| process_observation_error(reason, "Linux process stat is malformed"))?;
    stat[closing + 1..]
        .split_whitespace()
        .nth(index)
        .ok_or_else(|| process_observation_error(reason, "Linux process stat is incomplete"))
}

#[cfg(target_os = "linux")]
fn read_process_text(
    path: &Path,
    reason: &'static str,
) -> Result<String, ProcessBindingObservationError> {
    fs::read_to_string(path).map_err(|error| process_observation_error(reason, error))
}

#[cfg(any(target_os = "macos", windows))]
fn command_one_line(
    program: &str,
    args: &[&str],
    reason: &'static str,
) -> Result<String, ProcessBindingObservationError> {
    let output = command_output(program, args, reason)?;
    let value = output.trim_end_matches(['\r', '\n']);
    if value.is_empty() || value.contains(['\r', '\n']) {
        return Err(process_observation_error(
            reason,
            "process observation command did not return exactly one line",
        ));
    }
    Ok(value.to_owned())
}

#[cfg(any(target_os = "macos", windows))]
fn command_output(
    program: &str,
    args: &[&str],
    reason: &'static str,
) -> Result<String, ProcessBindingObservationError> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|error| process_observation_error(reason, error))?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(process_observation_error(
            reason,
            format!("process observation command failed with {}", output.status),
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| process_observation_error(reason, error))
}

#[cfg(any(target_os = "macos", windows))]
fn bounded_process_token(
    prefix: &str,
    value: &str,
) -> Result<String, ProcessBindingObservationError> {
    let value = value.trim();
    let token = format!("{prefix}:{value}");
    if value.is_empty() || token.len() > 256 || token.chars().any(char::is_control) {
        return Err(process_observation_error(
            "process_token_invalid",
            "process token is empty, oversized, or contains control characters",
        ));
    }
    Ok(token)
}

fn require_process_id(process_id: u64) -> Result<u64, ProcessBindingObservationError> {
    if process_id == 0 {
        Err(process_observation_error(
            "parent_process_unavailable",
            "the observed parent process ID is zero",
        ))
    } else {
        Ok(process_id)
    }
}

fn canonical_process_path(
    path: &Path,
    platform: PlatformEnvironment,
) -> Result<String, ProcessBindingObservationError> {
    #[cfg(windows)]
    let text = {
        let raw = path.to_str().ok_or_else(|| {
            process_observation_error("process_executable_path_invalid", "path is not UTF-8")
        })?;
        let raw = raw.strip_prefix(r"\\?\").unwrap_or(raw);
        let mut normalized = raw.replace('\\', "/");
        if let Some(first) = normalized.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        normalized
    };
    #[cfg(not(windows))]
    let text = path
        .to_str()
        .ok_or_else(|| {
            process_observation_error("process_executable_path_invalid", "path is not UTF-8")
        })?
        .to_owned();
    validate_canonical_platform_path(platform, &text).map_err(|error| {
        process_observation_error(error.reason(), "process executable path is not canonical")
    })?;
    Ok(text)
}

fn sha256_regular_file(path: &Path) -> Result<String, ProcessBindingObservationError> {
    use std::io::Read;

    let metadata = fs::symlink_metadata(path)
        .map_err(|error| process_observation_error("process_executable_unavailable", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(process_observation_error(
            "process_executable_unavailable",
            "process executable is not a regular non-symlink file",
        ));
    }
    let mut file = fs::File::open(path)
        .map_err(|error| process_observation_error("process_executable_unavailable", error))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| process_observation_error("process_executable_unavailable", error))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn process_observation_error(
    reason: &'static str,
    detail: impl fmt::Display,
) -> ProcessBindingObservationError {
    ProcessBindingObservationError {
        reason,
        detail: detail.to_string(),
    }
}

#[cfg(target_os = "linux")]
struct LinuxPlatformFacts<'a> {
    kernel_release: &'a str,
    wsl_distribution_name: Option<&'a str>,
    os_release: Option<String>,
}

#[cfg(target_os = "linux")]
fn classify_linux_platform_boundary(
    target_triple: ReleaseTargetTriple,
    facts: LinuxPlatformFacts<'_>,
) -> Result<LocalPlatformBoundary, PlatformBoundaryError> {
    let kernel = facts.kernel_release.trim().to_ascii_lowercase();
    if !kernel.contains("microsoft") {
        if facts.wsl_distribution_name.is_some() {
            return Err(unsupported_platform(
                "unsupported_wsl_cross_topology",
                "WSL_DISTRO_NAME is present outside an observable WSL kernel boundary",
            ));
        }
        return Ok(LocalPlatformBoundary {
            target_triple,
            environment: PlatformEnvironment::Linux,
            release_coordinate: PlatformReleaseCoordinate::Native,
        });
    }
    if !(kernel.contains("wsl2") || kernel.contains("microsoft-standard")) {
        return Err(unsupported_platform(
            "unsupported_wsl1",
            "the observed Microsoft Linux kernel is not a WSL2 kernel",
        ));
    }

    let distribution_name = facts.wsl_distribution_name.ok_or_else(|| {
        unavailable_platform(
            "wsl2_distribution_unavailable",
            "WSL_DISTRO_NAME is absent inside the WSL2 process",
        )
    })?;
    let os_release = facts.os_release.ok_or_else(|| {
        unavailable_platform(
            "wsl2_distribution_unavailable",
            "/etc/os-release was not observed inside the WSL2 process",
        )
    })?;
    let (distribution_id, distribution_version) = parse_os_release(&os_release)?;
    if distribution_name != PINNED_WSL2_DISTRIBUTION_NAME
        || distribution_id != PINNED_WSL2_DISTRIBUTION_ID
        || distribution_version != PINNED_WSL2_DISTRIBUTION_VERSION
    {
        return Err(unsupported_platform(
            "unsupported_wsl2_distribution",
            format!(
                "expected {PINNED_WSL2_DISTRIBUTION_NAME} with ID={PINNED_WSL2_DISTRIBUTION_ID} and VERSION_ID={PINNED_WSL2_DISTRIBUTION_VERSION}"
            ),
        ));
    }
    if !target_triple.supports_environment(PlatformEnvironment::Wsl2) {
        return Err(unsupported_platform(
            "unsupported_wsl2_target",
            format!("target {target_triple} has no WSL2 release cell"),
        ));
    }
    Ok(LocalPlatformBoundary {
        target_triple,
        environment: PlatformEnvironment::Wsl2,
        release_coordinate: PlatformReleaseCoordinate::first_release_wsl2(),
    })
}

fn current_release_target_triple() -> Result<ReleaseTargetTriple, PlatformBoundaryError> {
    let target = if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        Some(ReleaseTargetTriple::X86_64UnknownLinuxGnu)
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "aarch64",
        target_env = "gnu"
    )) {
        Some(ReleaseTargetTriple::Aarch64UnknownLinuxGnu)
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some(ReleaseTargetTriple::Aarch64AppleDarwin)
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some(ReleaseTargetTriple::X86_64AppleDarwin)
    } else if cfg!(all(windows, target_arch = "x86_64", target_env = "msvc")) {
        Some(ReleaseTargetTriple::X86_64PcWindowsMsvc)
    } else {
        None
    };
    target.ok_or_else(|| {
        unsupported_platform(
            "unsupported_release_target",
            "this executable target is not a published Volicord binary target",
        )
    })
}

#[cfg(target_os = "linux")]
fn parse_os_release(document: &str) -> Result<(String, String), PlatformBoundaryError> {
    let mut distribution_id = None;
    let mut distribution_version = None;
    for raw_line in document.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, raw_value)) = line.split_once('=') else {
            return Err(unavailable_platform(
                "wsl2_distribution_unavailable",
                "/etc/os-release contains a malformed entry",
            ));
        };
        let value = raw_value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(raw_value);
        let target = match name {
            "ID" => &mut distribution_id,
            "VERSION_ID" => &mut distribution_version,
            _ => continue,
        };
        if target.replace(value.to_owned()).is_some() {
            return Err(unavailable_platform(
                "wsl2_distribution_unavailable",
                "/etc/os-release repeats a required coordinate",
            ));
        }
    }
    Ok((
        distribution_id.ok_or_else(|| {
            unavailable_platform(
                "wsl2_distribution_unavailable",
                "/etc/os-release is missing ID",
            )
        })?,
        distribution_version.ok_or_else(|| {
            unavailable_platform(
                "wsl2_distribution_unavailable",
                "/etc/os-release is missing VERSION_ID",
            )
        })?,
    ))
}

#[cfg(target_os = "linux")]
fn read_platform_text(path: impl AsRef<Path>) -> Result<String, PlatformBoundaryError> {
    read_bounded_platform_text(
        path,
        MAX_PLATFORM_CONTROL_FILE_BYTES,
        "platform_environment_unavailable",
    )
}

#[cfg(target_os = "linux")]
fn read_bounded_platform_text(
    path: impl AsRef<Path>,
    max_bytes: u64,
    reason: &'static str,
) -> Result<String, PlatformBoundaryError> {
    use std::io::Read as _;

    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|error| {
        unavailable_platform(
            reason,
            format!("cannot inspect {}: {error}", path.display()),
        )
    })?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return Err(unavailable_platform(
            reason,
            format!("{} is not a bounded regular control file", path.display()),
        ));
    }
    let file = fs::File::open(path).map_err(|error| {
        unavailable_platform(reason, format!("cannot open {}: {error}", path.display()))
    })?;
    let mut bytes = Vec::new();
    file.take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            unavailable_platform(reason, format!("cannot read {}: {error}", path.display()))
        })?;
    if bytes.len() as u64 > max_bytes {
        return Err(unavailable_platform(
            reason,
            format!("{} exceeds the bounded control-file size", path.display()),
        ));
    }
    if bytes.contains(&0) {
        return Err(unavailable_platform(
            reason,
            format!("{} contains NUL", path.display()),
        ));
    }
    String::from_utf8(bytes).map_err(|error| {
        unavailable_platform(
            reason,
            format!("{} is not valid UTF-8: {error}", path.display()),
        )
    })
}

#[cfg(target_os = "linux")]
fn filesystem_type_for_path(path: &Path, mountinfo: &str) -> Result<String, PlatformBoundaryError> {
    let mut selected: Option<(usize, u64, String)> = None;
    for line in mountinfo.lines() {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        let separator = fields
            .iter()
            .position(|field| *field == "-")
            .ok_or_else(|| {
                unavailable_platform(
                    "platform_filesystem_unavailable",
                    "/proc/self/mountinfo contains a malformed entry",
                )
            })?;
        if fields.len() < 7 || separator < 6 || separator + 2 >= fields.len() {
            return Err(unavailable_platform(
                "platform_filesystem_unavailable",
                "/proc/self/mountinfo contains an incomplete entry",
            ));
        }
        let mount_id = fields[0].parse::<u64>().map_err(|_| {
            unavailable_platform(
                "platform_filesystem_unavailable",
                "/proc/self/mountinfo contains a non-decimal mount identifier",
            )
        })?;
        let mount_point = decode_mountinfo_path(fields[4])?;
        if !mount_point.is_absolute() {
            return Err(unavailable_platform(
                "platform_filesystem_unavailable",
                "/proc/self/mountinfo contains a non-absolute mount point",
            ));
        }
        if path.starts_with(&mount_point) {
            let depth = mount_point.components().count();
            let key = (depth, mount_id);
            if selected
                .as_ref()
                .is_none_or(|(selected_depth, selected_id, _)| {
                    key > (*selected_depth, *selected_id)
                })
            {
                selected = Some((depth, mount_id, fields[separator + 1].to_owned()));
            }
        }
    }
    selected
        .map(|(_, _, filesystem_type)| filesystem_type)
        .ok_or_else(|| {
            unavailable_platform(
                "platform_filesystem_unavailable",
                format!("/proc/self/mountinfo has no mount for {}", path.display()),
            )
        })
}

#[cfg(target_os = "linux")]
fn decode_mountinfo_path(encoded: &str) -> Result<PathBuf, PlatformBoundaryError> {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        let Some(escape) = bytes.get(index + 1..index + 4) else {
            return Err(malformed_mountinfo_escape());
        };
        decoded.push(match escape {
            b"040" => b' ',
            b"011" => b'\t',
            b"012" => b'\n',
            b"134" => b'\\',
            _ => return Err(malformed_mountinfo_escape()),
        });
        index += 4;
    }
    let decoded = String::from_utf8(decoded).map_err(|_| malformed_mountinfo_escape())?;
    Ok(PathBuf::from(decoded))
}

#[cfg(target_os = "linux")]
fn malformed_mountinfo_escape() -> PlatformBoundaryError {
    unavailable_platform(
        "platform_filesystem_unavailable",
        "/proc/self/mountinfo contains an invalid mount-point escape",
    )
}

#[cfg(target_os = "linux")]
fn nearest_existing_path(path: &Path) -> Result<PathBuf, PlatformBoundaryError> {
    let mut candidate = path.to_path_buf();
    loop {
        match fs::metadata(&candidate) {
            Ok(_) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if !candidate.pop() {
                    return Err(unavailable_platform(
                        "platform_filesystem_unavailable",
                        format!("{} has no observable existing ancestor", path.display()),
                    ));
                }
            }
            Err(error) => {
                return Err(unavailable_platform(
                    "platform_filesystem_unavailable",
                    format!("cannot inspect {}: {error}", candidate.display()),
                ));
            }
        }
    }
}

fn unavailable_platform(reason: &'static str, detail: impl Into<String>) -> PlatformBoundaryError {
    PlatformBoundaryError {
        kind: PlatformBoundaryErrorKind::Unavailable,
        reason,
        detail: detail.into(),
    }
}

fn unsupported_platform(reason: &'static str, detail: impl Into<String>) -> PlatformBoundaryError {
    PlatformBoundaryError {
        kind: PlatformBoundaryErrorKind::Unsupported,
        reason,
        detail: detail.into(),
    }
}

/// Resolved Git metadata roots for one Product Repository worktree.
///
/// `git_dir` identifies the selected worktree while `common_dir` identifies
/// metadata shared by linked sibling worktrees. Callers retain ownership of
/// any policy that uses these paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeLayout {
    /// Canonical Product Repository worktree root.
    pub repository_root: PathBuf,
    /// Canonical Git directory for this exact worktree.
    pub git_dir: PathBuf,
    /// Canonical Git common directory shared by linked worktrees.
    pub common_dir: PathBuf,
    /// Whether `git_dir` declares a distinct `commondir`.
    pub is_linked_worktree: bool,
}

/// Read-only Git workspace coordinate captured for one worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorkspaceSnapshot {
    /// Resolved worktree/common-directory layout.
    pub layout: GitWorktreeLayout,
    /// Stable local identity derived from the exact worktree Git directory.
    pub worktree_id: String,
    /// Symbolic HEAD ref when HEAD is symbolic, including its `refs/...` name.
    pub branch_ref: Option<String>,
    /// Current full Git object id, or `None` for an unborn symbolic HEAD.
    pub head_sha: Option<String>,
    /// Opaque coordinate digest over common dir, worktree id, branch, and HEAD.
    pub workspace_fingerprint: String,
}

/// Resolves a normal Git repository or linked worktree without invoking Git.
///
/// A missing `.git` marker returns `Ok(None)`. Unsafe, malformed, oversized,
/// or non-canonical control paths fail closed with `InvalidData`.
pub fn resolve_git_worktree_layout(
    repository_root: &Path,
) -> io::Result<Option<GitWorktreeLayout>> {
    let repository_root = fs::canonicalize(repository_root)?;
    let marker = repository_root.join(".git");
    let marker_metadata = match fs::symlink_metadata(&marker) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if marker_metadata.file_type().is_symlink() {
        return Err(invalid_git_data("the .git marker is a symbolic link"));
    }

    let git_dir = if marker_metadata.is_dir() {
        canonical_safe_directory(&marker)?
    } else if marker_metadata.is_file() {
        let marker_text = read_one_line_control_file(&marker)?;
        let value = marker_text
            .strip_prefix("gitdir: ")
            .ok_or_else(|| invalid_git_data("the .git file must contain one gitdir declaration"))?;
        let resolved = resolve_control_path(&repository_root, value)?;
        canonical_safe_directory(&resolved)?
    } else {
        return Err(invalid_git_data(
            "the .git marker is neither a directory nor a regular gitdir file",
        ));
    };

    let commondir_control = git_dir.join("commondir");
    let (common_dir, is_linked_worktree) = match fs::symlink_metadata(&commondir_control) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(invalid_git_data(
                    "the commondir control path is not a regular file",
                ));
            }
            let value = read_one_line_control_file(&commondir_control)?;
            let resolved = resolve_control_path(&git_dir, &value)?;
            (canonical_safe_directory(&resolved)?, true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => (git_dir.clone(), false),
        Err(error) => return Err(error),
    };

    Ok(Some(GitWorktreeLayout {
        repository_root,
        git_dir,
        common_dir,
        is_linked_worktree,
    }))
}

/// Captures the current branch/HEAD coordinate for a resolved Git worktree.
pub fn capture_git_workspace_snapshot(
    repository_root: &Path,
) -> io::Result<Option<GitWorkspaceSnapshot>> {
    let Some(layout) = resolve_git_worktree_layout(repository_root)? else {
        return Ok(None);
    };
    ensure_supported_git_reference_storage(&layout)?;
    let head_text = read_one_line_control_file(&layout.git_dir.join("HEAD"))?;
    let (branch_ref, head_sha) = if let Some(reference) = head_text.strip_prefix("ref: ") {
        validate_git_reference(reference)?;
        let resolved = resolve_reference_oid(&layout, reference)?;
        (Some(reference.to_owned()), resolved)
    } else {
        (None, Some(normalize_git_oid(&head_text)?))
    };

    let worktree_id = digest_fields(&[path_text(&layout.git_dir)?.as_str()]);
    let common_dir = path_text(&layout.common_dir)?;
    let branch_coordinate = branch_ref.as_deref().unwrap_or("detached");
    let head_coordinate = head_sha.as_deref().unwrap_or("unborn");
    let workspace_fingerprint = digest_fields(&[
        common_dir.as_str(),
        worktree_id.as_str(),
        branch_coordinate,
        head_coordinate,
    ]);
    Ok(Some(GitWorkspaceSnapshot {
        layout,
        worktree_id,
        branch_ref,
        head_sha,
        workspace_fingerprint,
    }))
}

fn ensure_supported_git_reference_storage(layout: &GitWorktreeLayout) -> io::Result<()> {
    let config_path = layout.common_dir.join("config");
    let bytes = match read_bounded_regular_file(&config_path, 1024 * 1024) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if bytes.contains(&0) {
        return Err(invalid_git_data("Git config contains NUL"));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| invalid_git_data("Git config is not valid UTF-8"))?;
    let mut section = String::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        if line.starts_with('[') {
            let Some(end) = line.find(']') else {
                return Err(invalid_git_data("Git config contains a malformed section"));
            };
            section = line[1..end]
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            continue;
        }
        if section != "extensions" {
            continue;
        }
        let (key, raw_value) = match line.split_once('=') {
            Some((key, value)) => (key.trim(), value.trim()),
            None => {
                let mut fields = line.splitn(2, char::is_whitespace);
                (
                    fields.next().unwrap_or_default(),
                    fields.next().unwrap_or_default().trim(),
                )
            }
        };
        if !key.eq_ignore_ascii_case("refstorage") {
            continue;
        }
        let value = raw_value
            .split(['#', ';'])
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches('"');
        if !value.eq_ignore_ascii_case("files") {
            return Err(invalid_git_data(
                "unsupported Git reference storage; only the files backend is supported",
            ));
        }
    }
    Ok(())
}

fn resolve_reference_oid(
    layout: &GitWorktreeLayout,
    reference: &str,
) -> io::Result<Option<String>> {
    for root in [&layout.git_dir, &layout.common_dir] {
        let path = root.join(reference);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(invalid_git_data("a Git reference is not a regular file"));
                }
                return read_one_line_control_file(&path)
                    .and_then(|value| normalize_git_oid(&value))
                    .map(Some);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }

    let packed_refs = layout.common_dir.join("packed-refs");
    let bytes = match read_bounded_regular_file(&packed_refs, 1024 * 1024) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| invalid_git_data("packed-refs is not valid UTF-8"))?;
    for line in text.lines() {
        if line.is_empty() || line.starts_with(['#', '^']) {
            continue;
        }
        let Some((oid, name)) = line.split_once(' ') else {
            return Err(invalid_git_data("packed-refs contains a malformed entry"));
        };
        if name == reference {
            return normalize_git_oid(oid).map(Some);
        }
    }
    Ok(None)
}

fn read_one_line_control_file(path: &Path) -> io::Result<String> {
    let bytes = read_bounded_regular_file(path, MAX_GIT_CONTROL_FILE_BYTES)?;
    if bytes.contains(&0) {
        return Err(invalid_git_data("a Git control file contains NUL"));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| invalid_git_data("a Git control file is not valid UTF-8"))?;
    let value = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);
    if value.is_empty() || value.contains(['\n', '\r']) {
        return Err(invalid_git_data(
            "a Git control file must contain one non-empty line",
        ));
    }
    Ok(value.to_owned())
}

fn read_bounded_regular_file(path: &Path, max_bytes: u64) -> io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(invalid_git_data(
            "a Git metadata path is not a regular file",
        ));
    }
    if metadata.len() > max_bytes {
        return Err(invalid_git_data("a Git metadata file is too large"));
    }
    fs::read(path)
}

fn resolve_control_path(base: &Path, value: &str) -> io::Result<PathBuf> {
    let value = Path::new(value);
    let joined = if value.is_absolute() {
        value.to_path_buf()
    } else {
        base.join(value)
    };
    normalize_absolute_path(&joined)
}

fn normalize_absolute_path(path: &Path) -> io::Result<PathBuf> {
    if !path.is_absolute() {
        return Err(invalid_git_data(
            "a resolved Git metadata path is not absolute",
        ));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(invalid_git_data(
                        "a resolved Git metadata path escapes its filesystem root",
                    ));
                }
            }
            Component::Normal(component) => normalized.push(component),
        }
    }
    Ok(normalized)
}

fn canonical_safe_directory(path: &Path) -> io::Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(invalid_git_data(
            "a resolved Git metadata path is not a regular directory",
        ));
    }
    let canonical = fs::canonicalize(path)?;
    if canonical != path {
        return Err(invalid_git_data(
            "a resolved Git metadata path traverses a symbolic link or non-canonical component",
        ));
    }
    Ok(canonical)
}

fn validate_git_reference(reference: &str) -> io::Result<()> {
    if !reference.starts_with("refs/")
        || reference.len() > 1024
        || reference.contains(['\0', '\n', '\r', '\\'])
        || reference.split('/').any(|component| {
            component.is_empty()
                || component == "."
                || component == ".."
                || component.starts_with('.')
        })
    {
        return Err(invalid_git_data(
            "HEAD contains an unsafe symbolic reference",
        ));
    }
    Ok(())
}

fn normalize_git_oid(value: &str) -> io::Result<String> {
    volicord_types::canonical_git_object_id(value)
        .map_err(|_| invalid_git_data("HEAD does not contain a full Git object id"))
}

fn path_text(path: &Path) -> io::Result<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid_git_data("a Git metadata path is not valid UTF-8"))
}

fn digest_fields(fields: &[&str]) -> String {
    let mut digest = Sha256::new();
    for field in fields {
        digest.update((field.len() as u64).to_be_bytes());
        digest.update(field.as_bytes());
    }
    format!("sha256:{:x}", digest.finalize())
}

fn invalid_git_data(detail: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, detail)
}

/// The documented namespace effect when `ReplaceFileW` reports a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaceFailureEffect {
    /// The replaced and replacement files retained their original names.
    NamesUnchanged,
    /// The replaced file moved to the requested backup name while the
    /// replacement retained its original name.
    ReplacedMovedToBackup,
}

/// A failed Windows replacement together with its documented namespace effect.
#[derive(Debug)]
pub struct ReplaceFileError {
    source: io::Error,
    effect: ReplaceFailureEffect,
}

impl ReplaceFileError {
    /// Returns the documented namespace effect that callers must revalidate.
    pub fn effect(&self) -> ReplaceFailureEffect {
        self.effect
    }

    /// Returns the underlying Windows error.
    pub fn io_error(&self) -> &io::Error {
        &self.source
    }
}

impl fmt::Display for ReplaceFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.source)
    }
}

impl std::error::Error for ReplaceFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[cfg(windows)]
/// Opens a regular file so that replacement may rename it while new write
/// handles and writable mappings are rejected until the returned handle is
/// dropped.
///
/// The caller must still compare the opened file's identity and contents with
/// its expected snapshot. This handle pins an object; it does not pin the path
/// name that currently refers to that object.
pub fn open_file_for_replace(path: &Path) -> io::Result<File> {
    use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ,
    };

    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(windows)]
#[allow(unsafe_code)]
/// Replaces `replaced` with `replacement` and moves the prior file to `backup`.
///
/// An error carries the namespace effect documented for the returned Windows
/// error code. Callers still need to inspect every participating path before
/// deciding whether cleanup or recovery is safe.
pub fn replace_file_with_backup(
    replaced: &Path,
    replacement: &Path,
    backup: &Path,
) -> Result<(), ReplaceFileError> {
    use std::ptr;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    let replaced = wide_path(replaced);
    let replacement = wide_path(replacement);
    let backup = wide_path(backup);
    // SAFETY: each pointer references a live, NUL-terminated UTF-16 buffer for
    // the duration of the call. The reserved pointers are required to be null.
    let success = unsafe {
        ReplaceFileW(
            replaced.as_ptr(),
            replacement.as_ptr(),
            backup.as_ptr(),
            0,
            ptr::null(),
            ptr::null(),
        )
    };
    if success != 0 {
        return Ok(());
    }

    let source = io::Error::last_os_error();
    Err(ReplaceFileError {
        effect: replace_failure_effect(source.raw_os_error()),
        source,
    })
}

#[cfg(windows)]
#[allow(unsafe_code)]
/// Moves a file without replacing an existing destination entry.
pub fn move_file_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};

    let source = wide_path(source);
    let destination = wide_path(destination);
    // SAFETY: both pointers reference live, NUL-terminated UTF-16 buffers for
    // the duration of the call.
    let success = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if success == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(any(windows, test))]
fn replace_failure_effect(raw_os_error: Option<i32>) -> ReplaceFailureEffect {
    // ERROR_UNABLE_TO_MOVE_REPLACEMENT_2. With a backup path supplied,
    // ReplaceFileW documents that the replaced file moved to that backup while
    // the replacement remains at its original name.
    if raw_os_error == Some(1177) {
        ReplaceFailureEffect::ReplacedMovedToBackup
    } else {
        ReplaceFailureEffect::NamesUnchanged
    }
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> io::Result<Self> {
            let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(io::Error::other)?
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "volicord-platform-fs-{label}-{}-{sequence}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path)?;
            Ok(Self(fs::canonicalize(path)?))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn replace_failure_effect_identifies_partial_namespace_move() {
        assert_eq!(
            replace_failure_effect(Some(1177)),
            ReplaceFailureEffect::ReplacedMovedToBackup
        );
        for code in [1175, 1176, 87] {
            assert_eq!(
                replace_failure_effect(Some(code)),
                ReplaceFailureEffect::NamesUnchanged
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn injected_platform_facts_accept_only_the_pinned_wsl2_coordinate() {
        let exact = classify_linux_platform_boundary(
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            LinuxPlatformFacts {
                kernel_release: "6.6.87.2-microsoft-standard-WSL2",
                wsl_distribution_name: Some("Ubuntu-24.04"),
                os_release: Some("ID=ubuntu\nVERSION_ID=\"24.04\"\n".to_owned()),
            },
        )
        .expect("exact WSL2 facts should be supported");
        assert_eq!(exact.environment, PlatformEnvironment::Wsl2);
        assert_eq!(
            exact.release_coordinate,
            PlatformReleaseCoordinate::first_release_wsl2()
        );

        for (name, id, version) in [
            ("Ubuntu-22.04", "ubuntu", "22.04"),
            ("Debian", "debian", "12"),
            ("Ubuntu-24.04", "ubuntu", "24.10"),
        ] {
            let error = classify_linux_platform_boundary(
                ReleaseTargetTriple::X86_64UnknownLinuxGnu,
                LinuxPlatformFacts {
                    kernel_release: "6.6.87.2-microsoft-standard-WSL2",
                    wsl_distribution_name: Some(name),
                    os_release: Some(format!("ID={id}\nVERSION_ID={version}\n")),
                },
            )
            .expect_err("neighboring distribution facts must fail closed");
            assert_eq!(error.kind(), PlatformBoundaryErrorKind::Unsupported);
            assert_eq!(error.reason(), "unsupported_wsl2_distribution");
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn injected_platform_facts_reject_wsl1_and_cross_topology() {
        let wsl1 = classify_linux_platform_boundary(
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            LinuxPlatformFacts {
                kernel_release: "4.4.0-19041-Microsoft",
                wsl_distribution_name: Some("Ubuntu-24.04"),
                os_release: Some("ID=ubuntu\nVERSION_ID=24.04\n".to_owned()),
            },
        )
        .expect_err("WSL1 must be unsupported");
        assert_eq!(wsl1.reason(), "unsupported_wsl1");

        let cross = classify_linux_platform_boundary(
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            LinuxPlatformFacts {
                kernel_release: "6.8.0-generic",
                wsl_distribution_name: Some("Ubuntu-24.04"),
                os_release: None,
            },
        )
        .expect_err("WSL environment values outside WSL must fail closed");
        assert_eq!(cross.reason(), "unsupported_wsl_cross_topology");

        let native = classify_linux_platform_boundary(
            ReleaseTargetTriple::Aarch64UnknownLinuxGnu,
            LinuxPlatformFacts {
                kernel_release: "6.8.0-generic",
                wsl_distribution_name: None,
                os_release: None,
            },
        )
        .expect("native Linux should remain supported");
        assert_eq!(native.environment, PlatformEnvironment::Linux);
        assert_eq!(
            native.target_triple,
            ReleaseTargetTriple::Aarch64UnknownLinuxGnu
        );
        assert_eq!(native.release_coordinate, PlatformReleaseCoordinate::Native);

        let arm_wsl2 = classify_linux_platform_boundary(
            ReleaseTargetTriple::Aarch64UnknownLinuxGnu,
            LinuxPlatformFacts {
                kernel_release: "6.6.87.2-microsoft-standard-WSL2",
                wsl_distribution_name: Some("Ubuntu-24.04"),
                os_release: Some("ID=ubuntu\nVERSION_ID=24.04\n".to_owned()),
            },
        )
        .expect_err("Linux AArch64 must not satisfy the x86-64 WSL2 cell");
        assert_eq!(arm_wsl2.reason(), "unsupported_wsl2_target");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn mountinfo_selection_requires_the_exact_ext4_filesystem_type() {
        let mountinfo = concat!(
            "21 1 8:1 / / rw,relatime - ext4 /dev/root rw\n",
            "34 21 0:42 / /home rw,relatime - 9p drvfs rw\n",
            "35 21 8:2 / /srv\\040data rw,relatime - ext3 /dev/loop0 rw\n",
        );
        assert_eq!(
            filesystem_type_for_path(Path::new("/opt/project"), mountinfo).unwrap(),
            "ext4"
        );
        assert_eq!(
            filesystem_type_for_path(Path::new("/home/project"), mountinfo).unwrap(),
            "9p"
        );
        assert_eq!(
            filesystem_type_for_path(Path::new("/srv data/project"), mountinfo).unwrap(),
            "ext3"
        );
    }

    #[test]
    fn resolves_normal_and_linked_worktrees_with_one_layout_model() -> io::Result<()> {
        let normal = TestDirectory::new("normal-layout")?;
        fs::create_dir(normal.path().join(".git"))?;
        let normal_layout = resolve_git_worktree_layout(normal.path())?
            .expect("normal repository should have a Git layout");
        assert_eq!(normal_layout.repository_root, normal.path());
        assert_eq!(normal_layout.git_dir, normal.path().join(".git"));
        assert_eq!(normal_layout.common_dir, normal.path().join(".git"));
        assert!(!normal_layout.is_linked_worktree);

        let linked = TestDirectory::new("linked-layout")?;
        let repository_root = linked.path().join("repo");
        let common_dir = linked.path().join("main/.git");
        let git_dir = common_dir.join("worktrees/repo");
        fs::create_dir_all(&repository_root)?;
        fs::create_dir_all(&git_dir)?;
        fs::write(
            repository_root.join(".git"),
            format!("gitdir: {}\n", git_dir.display()),
        )?;
        fs::write(git_dir.join("commondir"), "../..\n")?;

        let linked_layout = resolve_git_worktree_layout(&repository_root)?
            .expect("linked repository should have a Git layout");
        assert_eq!(linked_layout.repository_root, repository_root);
        assert_eq!(linked_layout.git_dir, git_dir);
        assert_eq!(linked_layout.common_dir, common_dir);
        assert!(linked_layout.is_linked_worktree);
        Ok(())
    }

    #[test]
    fn captures_symbolic_and_detached_workspace_coordinates() -> io::Result<()> {
        let repository = TestDirectory::new("workspace-snapshot")?;
        let git_dir = repository.path().join(".git");
        fs::create_dir_all(git_dir.join("refs/heads"))?;
        let first = "0123456789abcdef0123456789abcdef01234567";
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n")?;
        fs::write(git_dir.join("refs/heads/main"), format!("{first}\n"))?;

        let symbolic = capture_git_workspace_snapshot(repository.path())?
            .expect("symbolic workspace should be captured");
        assert_eq!(symbolic.branch_ref.as_deref(), Some("refs/heads/main"));
        assert_eq!(symbolic.head_sha.as_deref(), Some(first));
        assert!(symbolic.worktree_id.starts_with("sha256:"));
        assert!(symbolic.workspace_fingerprint.starts_with("sha256:"));

        let second = "89abcdef0123456789abcdef0123456789abcdef";
        fs::write(git_dir.join("HEAD"), format!("{second}\n"))?;
        let detached = capture_git_workspace_snapshot(repository.path())?
            .expect("detached workspace should be captured");
        assert_eq!(detached.branch_ref, None);
        assert_eq!(detached.head_sha.as_deref(), Some(second));
        assert_ne!(
            detached.workspace_fingerprint,
            symbolic.workspace_fingerprint
        );
        Ok(())
    }

    #[test]
    fn captures_unborn_and_packed_reference_states() -> io::Result<()> {
        let repository = TestDirectory::new("packed-workspace")?;
        let git_dir = repository.path().join(".git");
        fs::create_dir(&git_dir)?;
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/topic\n")?;

        let unborn = capture_git_workspace_snapshot(repository.path())?
            .expect("unborn workspace should be captured");
        assert_eq!(unborn.branch_ref.as_deref(), Some("refs/heads/topic"));
        assert_eq!(unborn.head_sha, None);

        let oid = "abcdef0123456789abcdef0123456789abcdef01";
        fs::write(
            git_dir.join("packed-refs"),
            format!("# pack-refs with: peeled\n{oid} refs/heads/topic\n"),
        )?;
        let packed = capture_git_workspace_snapshot(repository.path())?
            .expect("packed workspace should be captured");
        assert_eq!(packed.head_sha.as_deref(), Some(oid));
        assert_ne!(packed.workspace_fingerprint, unborn.workspace_fingerprint);
        Ok(())
    }

    #[test]
    fn rejects_unsupported_reftable_reference_storage() -> io::Result<()> {
        let repository = TestDirectory::new("reftable-workspace")?;
        let git_dir = repository.path().join(".git");
        fs::create_dir(&git_dir)?;
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n")?;
        fs::write(
            git_dir.join("config"),
            "[core]\n\trepositoryformatversion = 1\n[extensions]\n\trefStorage = reftable\n",
        )?;

        let error = capture_git_workspace_snapshot(repository.path())
            .expect_err("reftable must fail closed before an unborn snapshot is returned");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("only the files backend"));
        Ok(())
    }

    #[test]
    fn rejects_unsafe_git_control_paths() -> io::Result<()> {
        let repository = TestDirectory::new("unsafe-layout")?;
        fs::write(repository.path().join(".git"), "gitdir: /missing\nextra\n")?;
        let error = resolve_git_worktree_layout(repository.path())
            .expect_err("multi-line gitdir control must fail closed");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use std::{
        fs::{self, OpenOptions},
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "volicord-platform-fs-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("test directory should be created");
            Self(path)
        }

        fn join(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn replace_guard_rejects_new_write_handles() {
        let directory = TestDirectory::new("write-sharing");
        let target = directory.join("target.txt");
        fs::write(&target, b"old").expect("target should be written");

        let guard = open_file_for_replace(&target).expect("target should be guarded");
        let error = OpenOptions::new()
            .write(true)
            .open(&target)
            .expect_err("a new write handle should conflict with the guard");
        assert_eq!(error.raw_os_error(), Some(32));

        drop(guard);
        OpenOptions::new()
            .write(true)
            .open(&target)
            .expect("the target should be writable after the guard is dropped");
    }

    #[test]
    fn replacement_accepts_reserved_backup_and_preserved_predecessor() {
        let directory = TestDirectory::new("reserved-backup");
        let target = directory.join("target.txt");
        let replacement = directory.join("replacement.txt");
        let backup = directory.join("backup.txt");
        let predecessor = directory.join("predecessor.txt");
        fs::write(&target, b"old").expect("target should be written");
        fs::write(&replacement, b"new").expect("replacement should be written");
        fs::write(&backup, b"").expect("backup sentinel should be reserved");
        fs::hard_link(&target, &predecessor).expect("predecessor should be preserved");

        let guard = open_file_for_replace(&target).expect("target should be guarded");
        replace_file_with_backup(&target, &replacement, &backup)
            .expect("replacement should succeed");

        assert_eq!(
            fs::read(&target).expect("target should be readable"),
            b"new"
        );
        assert_eq!(
            fs::read(&backup).expect("backup should be readable"),
            b"old"
        );
        assert_eq!(
            fs::read(&predecessor).expect("predecessor should be readable"),
            b"old"
        );
        assert!(!replacement.exists());
        drop(guard);
    }
}
