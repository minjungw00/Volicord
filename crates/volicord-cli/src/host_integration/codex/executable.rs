use std::{
    cell::RefCell,
    collections::BTreeSet,
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use crate::host_integration::process::{CommandInvocation, CommandRunner};
use crate::host_integration::verification::{
    HostConfigurationStatus, HostExecutableStatus, HostGateStatus, Verification,
};
use volicord_types::{
    PlatformEnvironment, PlatformReleaseCoordinate, ProcessBinding, ReleaseTargetTriple,
};

use crate::host_integration::process::detect_platform_boundary;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexExecutableAvailability {
    pub(super) status: HostExecutableStatus,
    pub(super) host_version: Option<String>,
    pub(super) details: String,
    pub(super) diagnostic: Option<String>,
    pub(super) process_binding: Option<ProcessBinding>,
    pub(super) target_triple: Option<ReleaseTargetTriple>,
    pub(super) platform_environment: Option<PlatformEnvironment>,
    pub(super) platform_release_coordinate: Option<PlatformReleaseCoordinate>,
}

impl CodexExecutableAvailability {
    fn available(
        host_version: String,
        details: String,
        process_binding: ProcessBinding,
        target_triple: ReleaseTargetTriple,
        platform_environment: PlatformEnvironment,
        platform_release_coordinate: PlatformReleaseCoordinate,
    ) -> Self {
        Self {
            status: HostExecutableStatus::Available,
            host_version: Some(host_version),
            details,
            diagnostic: None,
            process_binding: Some(process_binding),
            target_triple: Some(target_triple),
            platform_environment: Some(platform_environment),
            platform_release_coordinate: Some(platform_release_coordinate),
        }
    }

    fn unavailable(details: String, diagnostic: impl Into<String>) -> Self {
        Self {
            status: HostExecutableStatus::Unavailable,
            host_version: None,
            details,
            diagnostic: Some(diagnostic.into()),
            process_binding: None,
            target_triple: None,
            platform_environment: None,
            platform_release_coordinate: None,
        }
    }

    pub(super) fn is_available(&self) -> bool {
        self.status == HostExecutableStatus::Available
    }
}

pub(super) fn codex_executable_availability<R: CommandRunner>(
    runner: &RefCell<R>,
    path: Option<&OsString>,
    native_executable: Option<&Path>,
    config_target: &Path,
) -> CodexExecutableAvailability {
    let Some(launcher) = find_executable_in_path("codex", path) else {
        return CodexExecutableAvailability::unavailable(
            format!(
                "Codex executable `codex` was not found on PATH; install Codex or make it available before using this Agent Connection; configuration target: {}",
                config_target.display()
            ),
            "Codex executable `codex` was not found on PATH",
        );
    };
    let platform_boundary = match detect_platform_boundary() {
        Ok(boundary) => boundary,
        Err(error) => {
            return CodexExecutableAvailability::unavailable(
                format!("Codex native executable platform could not be resolved: {error}"),
                error,
            )
        }
    };
    let platform = platform_boundary.environment;
    let target_triple = platform_boundary.target_triple;
    let executable = match native_executable
        .map(Path::to_path_buf)
        .map(Ok)
        .unwrap_or_else(|| resolve_codex_native_executable(&launcher, platform, target_triple))
        .and_then(|candidate| require_native_executable(candidate, platform))
    {
        Ok(executable) => executable,
        Err(error) => {
            return CodexExecutableAvailability::unavailable(
                format!(
                    "Codex native executable could not be resolved from launcher {}; {error}; configuration target: {}",
                    launcher.display(),
                    config_target.display()
                ),
                error,
            )
        }
    };
    let invocation = CommandInvocation {
        program: executable.display().to_string(),
        args: vec!["--version".to_owned()],
        cwd: None,
    };
    match runner.borrow_mut().run(&invocation) {
        Ok(output) if output.success => {
            let Some(version) = canonical_codex_version_output(&output.stdout, &output.stderr) else {
                return CodexExecutableAvailability::unavailable(
                    format!(
                        "Codex executable returned a non-canonical `codex --version` envelope; install or repair Codex before using this Agent Connection; configuration target: {}",
                        config_target.display()
                    ),
                    "Codex executable availability check returned a non-canonical version envelope",
                );
            };
            CodexExecutableAvailability::available(
                version.to_owned(),
                format!(
                    "Codex executable availability check succeeded with `codex --version`; canonical version: {version}; executable: {}; configuration target: {}",
                    executable.display(),
                    config_target.display()
                ),
                output.process_binding,
                output.target_triple,
                output.platform_environment,
                output.platform_release_coordinate,
            )
        }
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

fn resolve_codex_native_executable(
    launcher: &Path,
    platform: PlatformEnvironment,
    target_triple: ReleaseTargetTriple,
) -> Result<PathBuf, String> {
    if native_executable_magic_matches(launcher, platform)? {
        return Ok(launcher.to_path_buf());
    }
    let mut package_roots = BTreeSet::new();
    if let Ok(canonical) = fs::canonicalize(launcher) {
        if canonical.file_name().and_then(|name| name.to_str()) == Some("codex.js")
            && canonical.parent().and_then(Path::parent).is_some()
        {
            package_roots.insert(
                canonical
                    .parent()
                    .and_then(Path::parent)
                    .expect("checked package root")
                    .to_path_buf(),
            );
        }
    }
    let launcher_dir = launcher
        .parent()
        .ok_or_else(|| "native_codex_executable_unresolved: launcher has no parent".to_owned())?;
    for root in [
        launcher_dir.join("node_modules/@openai/codex"),
        launcher_dir.join("lib/node_modules/@openai/codex"),
        launcher_dir.join("../lib/node_modules/@openai/codex"),
    ] {
        if root.exists() {
            package_roots.insert(fs::canonicalize(root).map_err(|error| error.to_string())?);
        }
    }
    let pnpm_global = launcher_dir.join("global");
    if let Ok(entries) = fs::read_dir(pnpm_global) {
        for entry in entries {
            let root = entry
                .map_err(|error| error.to_string())?
                .path()
                .join("node_modules/@openai/codex");
            if root.exists() {
                package_roots.insert(fs::canonicalize(root).map_err(|error| error.to_string())?);
            }
        }
    }
    let (package, target, binary) = native_package_layout(target_triple);
    let mut candidates = BTreeSet::new();
    for root in package_roots {
        let mut package_candidates = vec![
            root.join("vendor").join(target).join("bin").join(binary),
            root.join("node_modules/@openai")
                .join(package)
                .join("vendor")
                .join(target)
                .join("bin")
                .join(binary),
        ];
        if let Some(scope_root) = root.parent() {
            package_candidates.push(
                scope_root
                    .join(package)
                    .join("vendor")
                    .join(target)
                    .join("bin")
                    .join(binary),
            );
        }
        for candidate in package_candidates {
            if candidate.is_file() && native_executable_magic_matches(&candidate, platform)? {
                candidates.insert(
                    fs::canonicalize(candidate)
                        .map_err(|error| format!("native_codex_executable_unresolved: {error}"))?,
                );
            }
        }
    }
    match candidates.len() {
        1 => Ok(candidates.pop_first().expect("one candidate exists")),
        0 => Err("native_codex_executable_unresolved: no native artifact matched the installed Codex package".to_owned()),
        _ => Err("native_codex_executable_ambiguous: multiple installed native artifacts matched the Codex launcher".to_owned()),
    }
}

fn require_native_executable(
    candidate: PathBuf,
    platform: PlatformEnvironment,
) -> Result<PathBuf, String> {
    if !candidate.is_file() || !native_executable_magic_matches(&candidate, platform)? {
        return Err(format!(
            "native_codex_executable_invalid: {} is not a native executable for {}",
            candidate.display(),
            platform.as_str()
        ));
    }
    Ok(candidate)
}

fn native_package_layout(
    target_triple: ReleaseTargetTriple,
) -> (&'static str, &'static str, &'static str) {
    match target_triple {
        ReleaseTargetTriple::X86_64UnknownLinuxGnu => {
            ("codex-linux-x64", "x86_64-unknown-linux-musl", "codex")
        }
        ReleaseTargetTriple::Aarch64UnknownLinuxGnu => {
            ("codex-linux-arm64", "aarch64-unknown-linux-musl", "codex")
        }
        ReleaseTargetTriple::X86_64AppleDarwin => {
            ("codex-darwin-x64", "x86_64-apple-darwin", "codex")
        }
        ReleaseTargetTriple::Aarch64AppleDarwin => {
            ("codex-darwin-arm64", "aarch64-apple-darwin", "codex")
        }
        ReleaseTargetTriple::X86_64PcWindowsMsvc => {
            ("codex-win32-x64", "x86_64-pc-windows-msvc", "codex.exe")
        }
    }
}

fn native_executable_magic_matches(
    path: &Path,
    platform: PlatformEnvironment,
) -> Result<bool, String> {
    let mut file = fs::File::open(path).map_err(|error| {
        format!(
            "native_codex_executable_unreadable for {}: {error}",
            path.display()
        )
    })?;
    let mut magic = [0_u8; 4];
    let count = file.read(&mut magic).map_err(|error| {
        format!(
            "native_codex_executable_unreadable for {}: {error}",
            path.display()
        )
    })?;
    if count < 2 {
        return Ok(false);
    }
    Ok(match platform {
        PlatformEnvironment::Linux | PlatformEnvironment::Wsl2 => magic == *b"\x7fELF",
        PlatformEnvironment::Macos => matches!(
            magic,
            [0xfe, 0xed, 0xfa, 0xce]
                | [0xfe, 0xed, 0xfa, 0xcf]
                | [0xce, 0xfa, 0xed, 0xfe]
                | [0xcf, 0xfa, 0xed, 0xfe]
                | [0xca, 0xfe, 0xba, 0xbe]
                | [0xbe, 0xba, 0xfe, 0xca]
        ),
        PlatformEnvironment::NativeWindows => magic[..2] == *b"MZ",
    })
}

fn canonical_codex_version_output<'a>(stdout: &'a str, stderr: &str) -> Option<&'a str> {
    if !stderr.is_empty() {
        return None;
    }
    let envelope = stdout.strip_suffix('\n')?;
    if envelope.contains(['\n', '\r']) {
        return None;
    }
    let version = envelope.strip_prefix("codex-cli ")?;
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
        || !version
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !version
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
    {
        return None;
    }
    Some(version)
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
    use std::{error::Error, io};

    use volicord_test_support::TempRuntimeHome;

    use super::*;

    #[test]
    fn resolves_pnpm_optional_native_package_next_to_the_main_package() -> Result<(), Box<dyn Error>>
    {
        let fixture = TempRuntimeHome::new("codex-pnpm-native-resolution")?;
        let launcher = fixture.path().join("pnpm/codex");
        fs::create_dir_all(launcher.parent().expect("launcher parent"))?;
        fs::write(&launcher, b"#!/bin/sh\n")?;

        let boundary = detect_platform_boundary().map_err(io::Error::other)?;
        let platform = boundary.environment;
        let (package, target, binary) = native_package_layout(boundary.target_triple);
        let main_package = fixture
            .path()
            .join("pnpm/global/5/node_modules/@openai/codex");
        fs::create_dir_all(&main_package)?;
        let native = main_package
            .parent()
            .expect("scoped package parent")
            .join(package)
            .join("vendor")
            .join(target)
            .join("bin")
            .join(binary);
        fs::create_dir_all(native.parent().expect("native binary parent"))?;
        fs::write(&native, native_magic(platform))?;

        let resolved = resolve_codex_native_executable(&launcher, platform, boundary.target_triple)
            .map_err(io::Error::other)?;
        assert_eq!(resolved, fs::canonicalize(native)?);
        Ok(())
    }

    #[test]
    fn native_package_layout_covers_every_published_target() {
        let layouts = [
            (
                ReleaseTargetTriple::X86_64UnknownLinuxGnu,
                "codex-linux-x64",
            ),
            (
                ReleaseTargetTriple::Aarch64UnknownLinuxGnu,
                "codex-linux-arm64",
            ),
            (
                ReleaseTargetTriple::Aarch64AppleDarwin,
                "codex-darwin-arm64",
            ),
            (ReleaseTargetTriple::X86_64AppleDarwin, "codex-darwin-x64"),
            (ReleaseTargetTriple::X86_64PcWindowsMsvc, "codex-win32-x64"),
        ];
        for (target, expected_package) in layouts {
            assert_eq!(native_package_layout(target).0, expected_package);
        }
    }

    fn native_magic(platform: PlatformEnvironment) -> &'static [u8] {
        match platform {
            PlatformEnvironment::Linux | PlatformEnvironment::Wsl2 => b"\x7fELFtest",
            PlatformEnvironment::Macos => b"\xfe\xed\xfa\xcftest",
            PlatformEnvironment::NativeWindows => b"MZtest",
        }
    }
}
