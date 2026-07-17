use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use volicord_types::{
    CodexReleaseCell, CodexReleaseRunnerArchitecture, CodexReleaseRunnerCoordinate,
    PlatformEnvironment, PINNED_WSL2_DISTRIBUTION_ID, PINNED_WSL2_DISTRIBUTION_VERSION,
};

use crate::{
    error::{ValidationError, ValidationResult},
    io::{sha256_external_file, ValidationContext},
};

use super::{run_bounded_status, GateConfiguration};

const VERSION_PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_WSL_EXECUTABLE_BYTES: u64 = 1024 * 1024 * 1024;

pub(super) fn validate_process_boundary(platform: PlatformEnvironment) -> ValidationResult<()> {
    match platform {
        PlatformEnvironment::Linux => {
            if env::consts::OS != "linux" {
                return Err(wrong_platform(platform));
            }
            let kernel = fs::read_to_string("/proc/sys/kernel/osrelease")?;
            let lower = kernel.to_ascii_lowercase();
            if lower.contains("microsoft")
                || lower.contains("wsl")
                || env::var_os("WSL_DISTRO_NAME").is_some()
                || env::var_os("WSL_INTEROP").is_some()
            {
                return Err(ValidationError::new(
                    "the native Linux cell refuses a WSL process boundary",
                ));
            }
            if linux_container_boundary_observed() {
                return Err(ValidationError::new(
                    "the native Linux cell refuses a container process boundary",
                ));
            }
        }
        PlatformEnvironment::Macos => {
            if env::consts::OS != "macos" {
                return Err(wrong_platform(platform));
            }
        }
        PlatformEnvironment::NativeWindows => {
            if env::consts::OS != "windows"
                || env::var_os("WSL_DISTRO_NAME").is_some()
                || env::var_os("WSL_INTEROP").is_some()
            {
                return Err(wrong_platform(platform));
            }
        }
        PlatformEnvironment::Wsl2 => {
            if env::consts::OS != "windows" {
                return Err(ValidationError::new(
                    "the WSL2 cell gate must run on its native Windows supervisor, not on an Ubuntu or native-Linux runner",
                ));
            }
        }
    }
    Ok(())
}

fn linux_container_boundary_observed() -> bool {
    if Path::new("/.dockerenv").exists()
        || Path::new("/run/.containerenv").exists()
        || Path::new("/run/systemd/container").exists()
        || env::var_os("container").is_some()
    {
        return true;
    }
    [
        "/proc/1/cgroup",
        "/proc/self/cgroup",
        "/proc/self/mountinfo",
    ]
    .into_iter()
    .filter_map(|path| fs::read_to_string(path).ok())
    .map(|text| text.to_ascii_lowercase())
    .any(|text| {
        [
            "docker",
            "containerd",
            "kubepods",
            "libpod",
            "podman",
            "lxc",
        ]
        .into_iter()
        .any(|marker| text.contains(marker))
    })
}

fn wrong_platform(platform: PlatformEnvironment) -> ValidationError {
    ValidationError::new(format!(
        "the current process boundary is not the required {} release cell",
        platform.as_str()
    ))
}

pub(super) fn validate_gate_paths(
    context: &ValidationContext,
    platform: PlatformEnvironment,
    configuration: &GateConfiguration,
) -> ValidationResult<()> {
    context.validate_existing_file(&configuration.scenario_driver)?;
    context.validate_existing_directory(&configuration.evidence_directory)?;
    require_empty_directory(
        &configuration.evidence_directory,
        "release evidence directory",
    )?;

    if platform == PlatformEnvironment::Wsl2 {
        let distribution = configuration
            .wsl2_distribution
            .as_deref()
            .expect("WSL2 configuration has a distribution");
        validate_wsl2_distribution(distribution)?;
        for (label, path, kind) in [
            (
                "Codex executable",
                configuration.codex_path.as_str(),
                WslPathKind::File,
            ),
            (
                "Volicord executable",
                configuration.volicord_path.as_str(),
                WslPathKind::File,
            ),
            (
                "cell work root",
                configuration.work_root.as_str(),
                WslPathKind::Directory,
            ),
        ] {
            validate_wsl2_path(distribution, path, label, kind)?;
        }
        validate_wsl2_runtime_home(
            distribution,
            &configuration.work_root,
            &configuration.runtime_home,
        )?;
        require_empty_wsl2_directory(distribution, &configuration.work_root)?;
    } else {
        let codex = PathBuf::from(&configuration.codex_path);
        let volicord = PathBuf::from(&configuration.volicord_path);
        context.validate_existing_file(&codex)?;
        context.validate_existing_file(&volicord)?;
        let work_root = canonical_existing_directory(&configuration.work_root, "cell work root")?;
        require_outside_repository_roots(context, &work_root, "cell work root")?;
        require_empty_directory(&work_root, "cell work root")?;
        validate_native_runtime_home(&work_root, &configuration.runtime_home)?;
    }
    Ok(())
}

fn canonical_existing_directory(value: &str, label: &str) -> ValidationResult<PathBuf> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(ValidationError::new(format!("{label} must be absolute")));
    }
    let canonical = fs::canonicalize(&path).map_err(|error| {
        ValidationError::new(format!(
            "cannot canonicalize {label} {}: {error}",
            path.display()
        ))
    })?;
    if canonical != path || !fs::metadata(&canonical)?.is_dir() {
        return Err(ValidationError::new(format!(
            "{label} must be an existing canonical symlink-free directory: {}",
            path.display()
        )));
    }
    Ok(canonical)
}

fn require_outside_repository_roots(
    context: &ValidationContext,
    path: &Path,
    label: &str,
) -> ValidationResult<()> {
    if path.starts_with(context.source_checkout())
        || path.starts_with(context.target_directory())
        || context.source_checkout().starts_with(path)
        || context.target_directory().starts_with(path)
    {
        return Err(ValidationError::new(format!(
            "{label} must not overlap the source checkout or Cargo target directory"
        )));
    }
    Ok(())
}

fn require_empty_directory(path: &Path, label: &str) -> ValidationResult<()> {
    if fs::read_dir(path)?.next().is_some() {
        return Err(ValidationError::new(format!(
            "{label} must be empty before a qualifying attempt: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_native_runtime_home(work_root: &Path, runtime_home: &str) -> ValidationResult<()> {
    let runtime_home = PathBuf::from(runtime_home);
    if !runtime_home.is_absolute()
        || !runtime_home.starts_with(work_root)
        || runtime_home == work_root
    {
        return Err(ValidationError::new(
            "VOLICORD_HOME must name an absent child of the cell work root",
        ));
    }
    if runtime_home.exists() {
        return Err(ValidationError::new(
            "VOLICORD_HOME must be absent before the runtime_home_creation scenario",
        ));
    }
    Ok(())
}

pub(super) fn validate_actual_runner_coordinate(
    cell: &CodexReleaseCell,
    platform: PlatformEnvironment,
    configuration: &GateConfiguration,
) -> ValidationResult<()> {
    let actual = collect_runner_coordinate(platform, configuration)?;
    let expected = &cell.validation_evidence.runner;
    if expected.runner_id != actual.runner_id
        || expected.target_triple != actual.target_triple
        || expected.architecture != actual.architecture
        || expected.os_release != actual.os_release
        || expected.environment_image != actual.environment_image
    {
        return Err(ValidationError::new(format!(
            "actual runner coordinate does not exactly match the checked-in {} cell",
            cell.platform.as_str()
        )));
    }
    Ok(())
}

pub(super) fn collect_runner_coordinate(
    platform: PlatformEnvironment,
    configuration: &GateConfiguration,
) -> ValidationResult<CodexReleaseRunnerCoordinate> {
    let (architecture, target_triple, os_release) = if platform == PlatformEnvironment::Wsl2 {
        let distribution = configuration
            .wsl2_distribution
            .as_deref()
            .expect("WSL2 configuration has a distribution");
        let architecture = parse_architecture(&wsl_text(distribution, "uname", &["-m"], 256)?)?;
        let target = linux_target_triple(architecture).to_owned();
        let release = wsl_text(distribution, "cat", &["/proc/sys/kernel/osrelease"], 4096)?;
        (architecture, target, release)
    } else {
        let architecture = parse_architecture(env::consts::ARCH)?;
        let target = native_target_triple(platform, architecture)?.to_owned();
        let release = native_os_release(platform)?;
        (architecture, target, release)
    };
    validate_release_architecture(platform, architecture)?;
    Ok(CodexReleaseRunnerCoordinate {
        runner_id: configuration.runner_id.clone(),
        target_triple,
        architecture,
        os_release,
        environment_image: configuration.environment_image.clone(),
    })
}

fn validate_release_architecture(
    platform: PlatformEnvironment,
    architecture: CodexReleaseRunnerArchitecture,
) -> ValidationResult<()> {
    let expected = match platform {
        PlatformEnvironment::Macos => CodexReleaseRunnerArchitecture::Aarch64,
        PlatformEnvironment::Linux
        | PlatformEnvironment::NativeWindows
        | PlatformEnvironment::Wsl2 => CodexReleaseRunnerArchitecture::X86_64,
    };
    if architecture != expected {
        return Err(ValidationError::new(format!(
            "the {} release cell requires {} architecture, not {}",
            platform.as_str(),
            expected.as_str(),
            architecture.as_str()
        )));
    }
    Ok(())
}

fn parse_architecture(value: &str) -> ValidationResult<CodexReleaseRunnerArchitecture> {
    match value.trim() {
        "x86_64" => Ok(CodexReleaseRunnerArchitecture::X86_64),
        "aarch64" | "arm64" => Ok(CodexReleaseRunnerArchitecture::Aarch64),
        other => Err(ValidationError::new(format!(
            "unsupported release-cell architecture {other}"
        ))),
    }
}

fn linux_target_triple(architecture: CodexReleaseRunnerArchitecture) -> &'static str {
    match architecture {
        CodexReleaseRunnerArchitecture::X86_64 => "x86_64-unknown-linux-gnu",
        CodexReleaseRunnerArchitecture::Aarch64 => "aarch64-unknown-linux-gnu",
    }
}

fn native_target_triple(
    platform: PlatformEnvironment,
    architecture: CodexReleaseRunnerArchitecture,
) -> ValidationResult<&'static str> {
    match (platform, architecture) {
        (PlatformEnvironment::Linux, architecture) => Ok(linux_target_triple(architecture)),
        (PlatformEnvironment::Macos, CodexReleaseRunnerArchitecture::X86_64) => {
            Ok("x86_64-apple-darwin")
        }
        (PlatformEnvironment::Macos, CodexReleaseRunnerArchitecture::Aarch64) => {
            Ok("aarch64-apple-darwin")
        }
        (PlatformEnvironment::NativeWindows, CodexReleaseRunnerArchitecture::X86_64) => {
            Ok("x86_64-pc-windows-msvc")
        }
        (PlatformEnvironment::NativeWindows, CodexReleaseRunnerArchitecture::Aarch64) => {
            Ok("aarch64-pc-windows-msvc")
        }
        (PlatformEnvironment::Wsl2, _) => Err(ValidationError::new(
            "WSL2 target triples are collected through the Windows supervisor",
        )),
    }
}

fn native_os_release(platform: PlatformEnvironment) -> ValidationResult<String> {
    match platform {
        PlatformEnvironment::Linux => bounded_text(
            fs::read("/proc/sys/kernel/osrelease")?,
            "Linux kernel release",
        ),
        PlatformEnvironment::Macos => command_text("sw_vers", &["-productVersion"], 4096),
        PlatformEnvironment::NativeWindows => command_text("cmd.exe", &["/D", "/C", "ver"], 4096),
        PlatformEnvironment::Wsl2 => Err(ValidationError::new(
            "WSL2 OS release must be read through its Windows supervisor",
        )),
    }
}

pub(super) fn hash_artifact(
    context: &ValidationContext,
    platform: PlatformEnvironment,
    artifact_path: &str,
    configuration: &GateConfiguration,
) -> ValidationResult<String> {
    if platform == PlatformEnvironment::Wsl2 {
        hash_wsl2_file(
            configuration
                .wsl2_distribution
                .as_deref()
                .expect("WSL2 configuration has a distribution"),
            artifact_path,
        )
    } else {
        sha256_external_file(context, Path::new(artifact_path), None)
    }
}

pub(super) fn probe_executable(
    platform: PlatformEnvironment,
    artifact_path: &str,
    configuration: &GateConfiguration,
) -> ValidationResult<()> {
    let mut command = if platform == PlatformEnvironment::Wsl2 {
        let mut command = Command::new("wsl.exe");
        command
            .args(["--distribution"])
            .arg(
                configuration
                    .wsl2_distribution
                    .as_deref()
                    .expect("WSL2 configuration has a distribution"),
            )
            .args(["--exec"])
            .arg(artifact_path);
        command
    } else {
        Command::new(artifact_path)
    };
    command.arg("--version");
    run_bounded_status(
        &mut command,
        VERSION_PROBE_TIMEOUT,
        "executable --version probe",
    )
}

#[derive(Debug, Clone, Copy)]
enum WslPathKind {
    File,
    Directory,
}

fn validate_wsl2_distribution(distribution: &str) -> ValidationResult<()> {
    let kernel = wsl_text(distribution, "cat", &["/proc/sys/kernel/osrelease"], 4096)?;
    let lower = kernel.to_ascii_lowercase();
    if !(lower.contains("microsoft-standard-wsl2") || lower.contains("wsl2")) {
        return Err(ValidationError::new(
            "selected distribution is not an observable WSL2 kernel boundary",
        ));
    }
    let observed_distribution = wsl_text(distribution, "printenv", &["WSL_DISTRO_NAME"], 4096)?;
    if observed_distribution != distribution {
        return Err(ValidationError::new(
            "selected WSL2 distribution name differs inside the distribution",
        ));
    }
    let os_release = wsl_document(distribution, "cat", &["/etc/os-release"], 16 * 1024)?;
    let id = os_release_value(&os_release, "ID");
    let version = os_release_value(&os_release, "VERSION_ID");
    if id.as_deref() != Some(PINNED_WSL2_DISTRIBUTION_ID)
        || version.as_deref() != Some(PINNED_WSL2_DISTRIBUTION_VERSION)
    {
        return Err(ValidationError::new(format!(
            "the first-release WSL2 cell requires Ubuntu {PINNED_WSL2_DISTRIBUTION_VERSION}"
        )));
    }
    Ok(())
}

fn os_release_value(document: &str, key: &str) -> Option<String> {
    document.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name == key).then(|| value.trim_matches('"').to_owned())
    })
}

fn validate_wsl2_path(
    distribution: &str,
    path: &str,
    label: &str,
    kind: WslPathKind,
) -> ValidationResult<()> {
    validate_linux_absolute_path(path, label)?;
    let canonical = wsl_text(distribution, "readlink", &["-f", "--", path], 16 * 1024)?;
    if canonical != path {
        return Err(ValidationError::new(format!(
            "{label} must be canonical and symlink-free inside WSL2"
        )));
    }
    let file_type = wsl_text(distribution, "stat", &["-c", "%F", "--", path], 4096)?;
    let expected_type = match kind {
        WslPathKind::File => "regular file",
        WslPathKind::Directory => "directory",
    };
    if file_type != expected_type {
        return Err(ValidationError::new(format!(
            "{label} is not an exact {expected_type} inside WSL2"
        )));
    }
    let filesystem = wsl_text(distribution, "stat", &["-f", "-c", "%T", "--", path], 4096)?;
    if filesystem != "ext2/ext3" && filesystem != "ext2/ext3/ext4" && filesystem != "ext4" {
        return Err(ValidationError::new(format!(
            "{label} must be on the selected distribution's ext4 filesystem, found {filesystem}"
        )));
    }
    Ok(())
}

fn validate_wsl2_runtime_home(
    distribution: &str,
    work_root: &str,
    runtime_home: &str,
) -> ValidationResult<()> {
    validate_linux_absolute_path(runtime_home, "Volicord Runtime Home")?;
    let prefix = format!("{}/", work_root.trim_end_matches('/'));
    if !runtime_home.starts_with(&prefix) {
        return Err(ValidationError::new(
            "VOLICORD_HOME must name an absent child of the WSL2 cell work root",
        ));
    }
    let status = Command::new("wsl.exe")
        .args(["--distribution", distribution, "--exec", "stat", "--"])
        .arg(runtime_home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        return Err(ValidationError::new(
            "VOLICORD_HOME must be absent before the runtime_home_creation scenario",
        ));
    }
    Ok(())
}

fn validate_linux_absolute_path(path: &str, label: &str) -> ValidationResult<()> {
    if !path.starts_with('/')
        || path == "/"
        || path.contains("//")
        || path
            .split('/')
            .any(|component| component == "." || component == "..")
        || path.starts_with("/mnt/")
        || path == "/mnt"
        || path.chars().any(char::is_control)
    {
        return Err(ValidationError::new(format!(
            "{label} must be an absolute normalized non-DrvFS Linux path"
        )));
    }
    Ok(())
}

fn require_empty_wsl2_directory(distribution: &str, path: &str) -> ValidationResult<()> {
    let output = wsl_text(
        distribution,
        "find",
        &[path, "-mindepth", "1", "-maxdepth", "1", "-print", "-quit"],
        16 * 1024,
    )?;
    if !output.is_empty() {
        return Err(ValidationError::new(
            "WSL2 cell work root must be empty before a qualifying attempt",
        ));
    }
    Ok(())
}

fn hash_wsl2_file(distribution: &str, path: &str) -> ValidationResult<String> {
    validate_wsl2_path(distribution, path, "release executable", WslPathKind::File)?;
    let mut child = Command::new("wsl.exe")
        .args(["--distribution", distribution, "--exec", "cat", "--"])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| ValidationError::new(format!("cannot read WSL2 executable: {error}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ValidationError::new("WSL2 executable reader has no stdout"))?;
    let mut reader = std::io::BufReader::new(stdout);
    let mut hasher = sha2::Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    use sha2::Digest as _;
    use std::io::Read as _;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > MAX_WSL_EXECUTABLE_BYTES {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ValidationError::new(
                "WSL2 executable exceeds the one-GiB release bound",
            ));
        }
        hasher.update(&buffer[..read]);
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(ValidationError::new(format!(
            "cannot read WSL2 executable bytes: {status}"
        )));
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn wsl_text(
    distribution: &str,
    program: &str,
    arguments: &[&str],
    max_bytes: usize,
) -> ValidationResult<String> {
    let mut command = Command::new("wsl.exe");
    command
        .args(["--distribution", distribution, "--exec", program])
        .args(arguments);
    command_text_from(&mut command, max_bytes, "WSL2 probe")
}

fn wsl_document(
    distribution: &str,
    program: &str,
    arguments: &[&str],
    max_bytes: usize,
) -> ValidationResult<String> {
    let mut command = Command::new("wsl.exe");
    let output = command
        .args(["--distribution", distribution, "--exec", program])
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| ValidationError::new(format!("cannot run WSL2 probe: {error}")))?;
    if !output.status.success() {
        return Err(ValidationError::new(format!(
            "WSL2 probe failed with status {}",
            output.status
        )));
    }
    if output.stdout.len() > max_bytes {
        return Err(ValidationError::new(
            "WSL2 probe output exceeds its byte bound",
        ));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|_| ValidationError::new("WSL2 probe output is not UTF-8"))?;
    if text.is_empty()
        || text
            .chars()
            .any(|character| character.is_control() && character != '\n' && character != '\r')
    {
        return Err(ValidationError::new(
            "WSL2 probe document contains unsupported control characters",
        ));
    }
    Ok(text)
}

fn command_text(program: &str, arguments: &[&str], max_bytes: usize) -> ValidationResult<String> {
    let mut command = Command::new(program);
    command.args(arguments);
    command_text_from(&mut command, max_bytes, program)
}

fn command_text_from(
    command: &mut Command,
    max_bytes: usize,
    label: &str,
) -> ValidationResult<String> {
    let output = command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| ValidationError::new(format!("cannot run {label}: {error}")))?;
    if !output.status.success() {
        return Err(ValidationError::new(format!(
            "{label} failed with status {}",
            output.status
        )));
    }
    if output.stdout.len() > max_bytes {
        return Err(ValidationError::new(format!(
            "{label} output exceeds its byte bound"
        )));
    }
    bounded_text(output.stdout, label)
}

fn bounded_text(bytes: Vec<u8>, label: &str) -> ValidationResult<String> {
    let text = String::from_utf8(bytes)
        .map_err(|_| ValidationError::new(format!("{label} output is not UTF-8")))?;
    let trimmed = text.trim_matches(['\r', '\n']).to_owned();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(ValidationError::new(format!(
            "{label} output must be nonempty and control-free"
        )));
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_paths_reject_drvfs_and_lexical_aliases() {
        assert!(validate_linux_absolute_path("/home/release/codex", "artifact").is_ok());
        for path in ["relative", "/mnt/c/codex", "/home/../codex", "/home//codex"] {
            assert!(validate_linux_absolute_path(path, "artifact").is_err());
        }
    }

    #[test]
    fn target_triples_are_platform_and_architecture_exact() {
        assert_eq!(
            native_target_triple(
                PlatformEnvironment::NativeWindows,
                CodexReleaseRunnerArchitecture::X86_64
            )
            .unwrap(),
            "x86_64-pc-windows-msvc"
        );
        assert_eq!(
            native_target_triple(
                PlatformEnvironment::Macos,
                CodexReleaseRunnerArchitecture::Aarch64
            )
            .unwrap(),
            "aarch64-apple-darwin"
        );
        assert!(native_target_triple(
            PlatformEnvironment::Wsl2,
            CodexReleaseRunnerArchitecture::X86_64
        )
        .is_err());

        assert!(validate_release_architecture(
            PlatformEnvironment::Macos,
            CodexReleaseRunnerArchitecture::Aarch64
        )
        .is_ok());
        assert!(validate_release_architecture(
            PlatformEnvironment::Macos,
            CodexReleaseRunnerArchitecture::X86_64
        )
        .is_err());
    }
}
