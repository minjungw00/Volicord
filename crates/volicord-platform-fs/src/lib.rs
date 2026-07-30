//! Safe platform filesystem primitives used by Volicord's local adapters.

#![deny(unsafe_code)]

use std::{
    fmt, fs, io,
    path::{Component, Path, PathBuf},
};

use serde::Serialize;
use sha2::{Digest, Sha256};
use volicord_types::diagnostics::{
    DiagnosticAction, DiagnosticCode, DiagnosticDomain, DiagnosticError, DiagnosticFactSource,
    DiagnosticFacts, DiagnosticFinding, DiagnosticFindingId, DiagnosticSeverity, DiagnosticSource,
    DiagnosticStage, DiagnosticSubject,
};
use volicord_types::platform::{
    PlatformEnvironment, PINNED_WSL2_DISTRIBUTION_ID, PINNED_WSL2_DISTRIBUTION_VERSION,
};
use volicord_types::release_target::ReleaseTargetTriple;
use volicord_types::values::UtcTimestamp;

mod mutation_lease;
mod product_path;
mod repository_observation;

pub use mutation_lease::{
    canonical_runtime_home_path, CanonicalRuntimeHomePath, RuntimeHomeMutationBusy,
    RuntimeHomeMutationLease, RuntimeHomeMutationLeaseError, RuntimeHomeMutationLeaseMode,
    RuntimeHomeMutationLeaseOutcome, RuntimeHomeMutationLockIdentity, RuntimeHomeMutationPermit,
    RuntimeHomeMutationWaitPolicy,
};
pub use product_path::{ObservedProductPath, ObservedProductPathState, ObservedProductRepository};
pub use repository_observation::{
    ContentIdentity, InvocationObservationPaths, ObservationUnavailable,
    ObservationUnavailableReason, ObserverLimits, ProductPathState, RepositoryDelta,
    RepositoryObservationCoordinate, RepositoryObservationSnapshot, RepositoryObserver,
    RepositoryPathTransition, SemanticObserverContractDigest,
};

#[cfg(windows)]
use std::fs::File;

const MAX_GIT_CONTROL_FILE_BYTES: u64 = 4096;
const MAX_PLATFORM_CONTROL_FILE_BYTES: u64 = 16 * 1024;
const MAX_MOUNTINFO_BYTES: u64 = 4 * 1024 * 1024;
/// Maximum UTF-8 byte length retained for one platform diagnostic detail.
pub const MAX_PLATFORM_DIAGNOSTIC_DETAIL_BYTES: usize = 1_024;
const PLATFORM_DIAGNOSTIC_DETAIL_TRUNCATED_SUFFIX: &str = "...[truncated]";

#[cfg(target_os = "linux")]
const KERNEL_RELEASE_PATH: &str = "/proc/sys/kernel/osrelease";
#[cfg(target_os = "linux")]
const OS_RELEASE_PATH: &str = "/etc/os-release";
#[cfg(target_os = "linux")]
const MOUNTINFO_PATH: &str = "/proc/self/mountinfo";

/// One observed local process-platform boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPlatformBoundary {
    /// Exact Volicord binary target executing in this process.
    pub target_triple: ReleaseTargetTriple,
    /// Exact independent release environment.
    pub environment: PlatformEnvironment,
}

/// Filesystem kind observed for one canonical path or its nearest existing ancestor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathFilesystemKind {
    /// Linux ext-family filesystem used as the WSL2 distribution ext4 boundary.
    LinuxExt4,
    /// A filesystem outside the supported WSL2 ext4 boundary.
    Other,
}

/// Stable routing class for a local platform-boundary diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformDiagnosticClass {
    /// The observed path violates a required platform-owned boundary.
    Rejected,
    /// Required local observation could not be completed.
    Unavailable,
    /// The observed platform or topology is outside the supported platform boundary.
    Unsupported,
}

/// Closed semantic kinds for Volicord-owned platform diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlatformDiagnosticKind {
    /// The operating system is not a supported Volicord platform target.
    UnsupportedOperatingSystem,
    /// The running binary target is not a supported platform target.
    UnsupportedTarget,
    /// The process is running under WSL1 rather than WSL2.
    Wsl1,
    /// The WSL2 distribution identity could not be observed.
    Wsl2DistributionIdentityUnavailable,
    /// The observed WSL2 distribution does not match the supported distribution identity.
    UnsupportedWsl2Distribution,
    /// A required filesystem observation could not be completed.
    FilesystemObservationFailure,
    /// The selected Product Repository root does not exist.
    ProductRepositoryNotFound,
    /// The selected Product Repository root is not a usable directory.
    InvalidProductRepositoryRoot,
    /// An operation requiring an existing Product Repository path could not find it.
    ProductPathNotFound,
    /// A Product Repository path could not be accessed or inspected.
    ProductPathInaccessible,
    /// A Product Repository path resolves outside the canonical repository root.
    ProductPathContainmentFailure,
    /// A path is on a filesystem outside the supported platform boundary.
    UnsupportedFilesystemBoundary,
    /// A required platform observation could not be completed.
    PlatformObservationFailure,
}

impl PlatformDiagnosticKind {
    /// Every owner-defined kind in canonical registry order.
    pub const ALL: [Self; 13] = [
        Self::UnsupportedOperatingSystem,
        Self::UnsupportedTarget,
        Self::Wsl1,
        Self::Wsl2DistributionIdentityUnavailable,
        Self::UnsupportedWsl2Distribution,
        Self::FilesystemObservationFailure,
        Self::ProductRepositoryNotFound,
        Self::InvalidProductRepositoryRoot,
        Self::ProductPathNotFound,
        Self::ProductPathInaccessible,
        Self::ProductPathContainmentFailure,
        Self::UnsupportedFilesystemBoundary,
        Self::PlatformObservationFailure,
    ];

    /// Stable namespaced machine-readable identity for this kind.
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedOperatingSystem => "platform.operating_system.unsupported",
            Self::UnsupportedTarget => "platform.target.unsupported",
            Self::Wsl1 => "platform.wsl1.unsupported",
            Self::Wsl2DistributionIdentityUnavailable => {
                "platform.wsl2.distribution_identity_unavailable"
            }
            Self::UnsupportedWsl2Distribution => "platform.wsl2.distribution_unsupported",
            Self::FilesystemObservationFailure => "platform.filesystem.observation_failed",
            Self::ProductRepositoryNotFound => "platform.product_repository.not_found",
            Self::InvalidProductRepositoryRoot => "platform.product_repository.invalid_root",
            Self::ProductPathNotFound => "platform.product_path.not_found",
            Self::ProductPathInaccessible => "platform.product_path.inaccessible",
            Self::ProductPathContainmentFailure => "platform.product_path.containment_failed",
            Self::UnsupportedFilesystemBoundary => "platform.filesystem.unsupported",
            Self::PlatformObservationFailure => "platform.observation.failed",
        }
    }

    /// Bounded static summary for structured diagnostic facts.
    pub const fn summary(self) -> &'static str {
        match self {
            Self::UnsupportedOperatingSystem => {
                "The operating system is not a supported Volicord platform target"
            }
            Self::UnsupportedTarget => "The running binary target is not supported",
            Self::Wsl1 => "The process is running under WSL1",
            Self::Wsl2DistributionIdentityUnavailable => {
                "The WSL2 distribution identity could not be observed"
            }
            Self::UnsupportedWsl2Distribution => "The observed WSL2 distribution is not supported",
            Self::FilesystemObservationFailure => {
                "The selected path filesystem could not be observed"
            }
            Self::ProductRepositoryNotFound => "The selected Product Repository root was not found",
            Self::InvalidProductRepositoryRoot => "The selected Product Repository root is invalid",
            Self::ProductPathNotFound => "The required Product Repository path was not found",
            Self::ProductPathInaccessible => "The Product Repository path could not be accessed",
            Self::ProductPathContainmentFailure => {
                "The Product Repository path resolves outside the repository boundary"
            }
            Self::UnsupportedFilesystemBoundary => {
                "The selected path is outside the supported filesystem boundary"
            }
            Self::PlatformObservationFailure => "A required local platform observation failed",
        }
    }

    /// Stable broad routing class derived from the semantic kind.
    pub const fn class(self) -> PlatformDiagnosticClass {
        match self {
            Self::ProductPathContainmentFailure => PlatformDiagnosticClass::Rejected,
            Self::UnsupportedOperatingSystem
            | Self::UnsupportedTarget
            | Self::Wsl1
            | Self::UnsupportedWsl2Distribution
            | Self::UnsupportedFilesystemBoundary => PlatformDiagnosticClass::Unsupported,
            Self::Wsl2DistributionIdentityUnavailable
            | Self::FilesystemObservationFailure
            | Self::ProductRepositoryNotFound
            | Self::InvalidProductRepositoryRoot
            | Self::ProductPathNotFound
            | Self::ProductPathInaccessible
            | Self::PlatformObservationFailure => PlatformDiagnosticClass::Unavailable,
        }
    }

    /// Returns whether retrying after an external-state change can succeed.
    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::Wsl2DistributionIdentityUnavailable
                | Self::FilesystemObservationFailure
                | Self::ProductRepositoryNotFound
                | Self::InvalidProductRepositoryRoot
                | Self::ProductPathNotFound
                | Self::ProductPathInaccessible
                | Self::PlatformObservationFailure
        )
    }
}

/// One typed platform diagnostic with bounded human-readable detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformDiagnostic {
    kind: PlatformDiagnosticKind,
    detail: String,
}

impl PlatformDiagnostic {
    /// Constructs a diagnostic and bounds its display detail.
    pub fn new(kind: PlatformDiagnosticKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: bounded_platform_diagnostic_detail(detail.into()),
        }
    }

    /// Returns the typed semantic kind.
    pub const fn kind(&self) -> PlatformDiagnosticKind {
        self.kind
    }

    /// Returns the stable namespaced machine-readable identity.
    pub const fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// Returns the broad routing class.
    pub const fn class(&self) -> PlatformDiagnosticClass {
        self.kind.class()
    }

    /// Returns the bounded human-readable detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for PlatformDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.detail)
    }
}

/// Safe bounded facts for one platform observation finding.
#[derive(Debug, Default, Serialize)]
pub struct PlatformDiagnosticFacts {
    pub target_triple: Option<String>,
    pub platform_environment: Option<String>,
}

impl DiagnosticFactSource for PlatformDiagnosticFacts {}

/// Builds one shared structured finding from a typed platform failure.
pub fn platform_diagnostic_finding(
    diagnostic: &PlatformDiagnostic,
    finding_id: impl Into<String>,
    facts: &PlatformDiagnosticFacts,
    observed_at: UtcTimestamp,
) -> Result<DiagnosticFinding, DiagnosticError> {
    let action = match diagnostic.class() {
        PlatformDiagnosticClass::Rejected => DiagnosticAction::try_new(
            DiagnosticCode::parse("action.platform.select_contained_product_path")?,
            "Select a path contained by the canonical Product Repository",
        )?,
        PlatformDiagnosticClass::Unsupported => DiagnosticAction::try_new(
            DiagnosticCode::parse("action.platform.use_supported_environment")?,
            "Use a supported Volicord platform target and environment",
        )?,
        PlatformDiagnosticClass::Unavailable => DiagnosticAction::try_new(
            DiagnosticCode::parse("action.platform.repair_observation_access")?,
            "Restore access to the required local platform observations",
        )?,
    };
    DiagnosticFinding::try_new(
        DiagnosticFindingId::parse(finding_id)?,
        DiagnosticCode::parse(diagnostic.code())?,
        DiagnosticDomain::parse("platform")?,
        DiagnosticStage::parse("platform_observation")?,
        DiagnosticSeverity::Error,
        DiagnosticSource::parse("platform_filesystem")?,
        DiagnosticSubject::try_new("platform", "local_process")?,
        DiagnosticFacts::project(facts)?,
        observed_at,
    )?
    .with_actions(vec![action])
}

/// Machine-readable local platform-boundary observation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformBoundaryError {
    diagnostic: PlatformDiagnostic,
}

impl PlatformBoundaryError {
    /// Returns the typed semantic diagnostic kind.
    pub const fn kind(&self) -> PlatformDiagnosticKind {
        self.diagnostic.kind()
    }

    /// Returns the stable namespaced machine-readable identity.
    pub const fn code(&self) -> &'static str {
        self.diagnostic.code()
    }

    /// Returns the broad routing class.
    pub const fn class(&self) -> PlatformDiagnosticClass {
        self.diagnostic.class()
    }

    /// Returns bounded implementation-facing detail.
    pub fn detail(&self) -> &str {
        self.diagnostic.detail()
    }

    /// Consumes the error and returns its typed diagnostic.
    pub fn into_diagnostic(self) -> PlatformDiagnostic {
        self.diagnostic
    }
}

impl fmt::Display for PlatformBoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.diagnostic.fmt(formatter)
    }
}

impl std::error::Error for PlatformBoundaryError {}

/// Observes the current native or exact pinned WSL2 process boundary.
#[cfg(target_os = "linux")]
pub fn observe_local_platform_boundary() -> Result<LocalPlatformBoundary, PlatformBoundaryError> {
    let target_triple = current_release_target_triple()?;
    observe_linux_platform_boundary(target_triple, &mut LocalLinuxPlatformObservation)
}

/// Observes the current native or exact pinned WSL2 process boundary.
#[cfg(target_os = "macos")]
pub fn observe_local_platform_boundary() -> Result<LocalPlatformBoundary, PlatformBoundaryError> {
    let target_triple = current_release_target_triple()?;
    Ok(LocalPlatformBoundary {
        target_triple,
        environment: PlatformEnvironment::Macos,
    })
}

/// Observes the current native or exact pinned WSL2 process boundary.
#[cfg(windows)]
pub fn observe_local_platform_boundary() -> Result<LocalPlatformBoundary, PlatformBoundaryError> {
    let target_triple = current_release_target_triple()?;
    Ok(LocalPlatformBoundary {
        target_triple,
        environment: PlatformEnvironment::NativeWindows,
    })
}

/// Observes the current native or exact pinned WSL2 process boundary.
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub fn observe_local_platform_boundary() -> Result<LocalPlatformBoundary, PlatformBoundaryError> {
    Err(unsupported_platform(
        PlatformDiagnosticKind::UnsupportedOperatingSystem,
        "this operating-system target is unsupported; use a supported Volicord platform target",
    ))
}

/// Observes the filesystem containing a path or its nearest existing ancestor.
#[cfg(target_os = "linux")]
pub fn observe_path_filesystem(path: &Path) -> Result<PathFilesystemKind, PlatformBoundaryError> {
    let existing = nearest_existing_path(path)?;
    let canonical_existing = fs::canonicalize(&existing).map_err(|error| {
        unavailable_platform(
            PlatformDiagnosticKind::FilesystemObservationFailure,
            format!("cannot canonicalize {}: {error}", existing.display()),
        )
    })?;
    let stat = rustix::fs::statfs(&canonical_existing).map_err(|error| {
        unavailable_platform(
            PlatformDiagnosticKind::FilesystemObservationFailure,
            format!("cannot inspect filesystem for {}: {error}", path.display()),
        )
    })?;
    if stat.f_type != 0x0000_ef53 {
        return Ok(PathFilesystemKind::Other);
    }
    let mountinfo = read_linux_mountinfo()?;
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

#[cfg(target_os = "linux")]
struct LinuxPlatformFacts<'a> {
    kernel_release: &'a str,
    os_release: Option<&'a str>,
}

#[cfg(target_os = "linux")]
trait LinuxPlatformObservation {
    fn read_kernel_release(&mut self) -> io::Result<String>;

    fn read_wsl2_os_release(&mut self) -> io::Result<String>;
}

#[cfg(target_os = "linux")]
struct LocalLinuxPlatformObservation;

#[cfg(target_os = "linux")]
impl LinuxPlatformObservation for LocalLinuxPlatformObservation {
    fn read_kernel_release(&mut self) -> io::Result<String> {
        read_bounded_platform_text(KERNEL_RELEASE_PATH, MAX_PLATFORM_CONTROL_FILE_BYTES)
    }

    fn read_wsl2_os_release(&mut self) -> io::Result<String> {
        read_bounded_platform_text(OS_RELEASE_PATH, MAX_PLATFORM_CONTROL_FILE_BYTES)
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinuxKernelBoundary {
    Native,
    Wsl1,
    Wsl2,
}

#[cfg(target_os = "linux")]
fn classify_linux_kernel_release(kernel_release: &str) -> LinuxKernelBoundary {
    let kernel = kernel_release.trim().to_ascii_lowercase();
    if !kernel.contains("microsoft") {
        LinuxKernelBoundary::Native
    } else if kernel.contains("wsl2") || kernel.contains("microsoft-standard") {
        LinuxKernelBoundary::Wsl2
    } else {
        LinuxKernelBoundary::Wsl1
    }
}

#[cfg(target_os = "linux")]
fn observe_linux_platform_boundary(
    target_triple: ReleaseTargetTriple,
    observation: &mut impl LinuxPlatformObservation,
) -> Result<LocalPlatformBoundary, PlatformBoundaryError> {
    let kernel_release = observation.read_kernel_release().map_err(|error| {
        unavailable_platform(
            PlatformDiagnosticKind::PlatformObservationFailure,
            format!(
                "the Linux kernel release required to classify the host could not be read from {KERNEL_RELEASE_PATH}: {error}"
            ),
        )
    })?;
    let os_release = match classify_linux_kernel_release(&kernel_release) {
        LinuxKernelBoundary::Wsl2 => Some(observation.read_wsl2_os_release().map_err(|error| {
            unavailable_platform(
                PlatformDiagnosticKind::Wsl2DistributionIdentityUnavailable,
                format!(
                    "the WSL2 distribution identity could not be read from {OS_RELEASE_PATH}: {error}"
                ),
            )
        })?),
        LinuxKernelBoundary::Native | LinuxKernelBoundary::Wsl1 => None,
    };
    classify_linux_platform_boundary(
        target_triple,
        LinuxPlatformFacts {
            kernel_release: &kernel_release,
            os_release: os_release.as_deref(),
        },
    )
}

#[cfg(target_os = "linux")]
fn classify_linux_platform_boundary(
    target_triple: ReleaseTargetTriple,
    facts: LinuxPlatformFacts<'_>,
) -> Result<LocalPlatformBoundary, PlatformBoundaryError> {
    match classify_linux_kernel_release(facts.kernel_release) {
        LinuxKernelBoundary::Native => {
            return Ok(LocalPlatformBoundary {
                target_triple,
                environment: PlatformEnvironment::Linux,
            });
        }
        LinuxKernelBoundary::Wsl1 => {
            return Err(unsupported_platform(
                PlatformDiagnosticKind::Wsl1,
                "the observed Microsoft Linux kernel is not a WSL2 kernel",
            ));
        }
        LinuxKernelBoundary::Wsl2 => {}
    }

    let os_release = facts.os_release.ok_or_else(|| {
        unavailable_platform(
            PlatformDiagnosticKind::Wsl2DistributionIdentityUnavailable,
            "/etc/os-release was not observed inside the WSL2 process",
        )
    })?;
    let (distribution_id, distribution_version) = parse_os_release(os_release)?;
    if distribution_id != PINNED_WSL2_DISTRIBUTION_ID
        || distribution_version != PINNED_WSL2_DISTRIBUTION_VERSION
    {
        return Err(unsupported_platform(
            PlatformDiagnosticKind::UnsupportedWsl2Distribution,
            format!(
                "expected ID={PINNED_WSL2_DISTRIBUTION_ID} and VERSION_ID={PINNED_WSL2_DISTRIBUTION_VERSION}"
            ),
        ));
    }
    if !target_triple.supports_environment(PlatformEnvironment::Wsl2) {
        return Err(unsupported_platform(
            PlatformDiagnosticKind::UnsupportedTarget,
            format!("target {target_triple} cannot run in the supported WSL2 environment"),
        ));
    }
    Ok(LocalPlatformBoundary {
        target_triple,
        environment: PlatformEnvironment::Wsl2,
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
            PlatformDiagnosticKind::UnsupportedTarget,
            "this executable target is not a supported Volicord binary target",
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
                PlatformDiagnosticKind::Wsl2DistributionIdentityUnavailable,
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
                PlatformDiagnosticKind::Wsl2DistributionIdentityUnavailable,
                "/etc/os-release repeats a required coordinate",
            ));
        }
    }
    Ok((
        distribution_id.ok_or_else(|| {
            unavailable_platform(
                PlatformDiagnosticKind::Wsl2DistributionIdentityUnavailable,
                "/etc/os-release is missing ID",
            )
        })?,
        distribution_version.ok_or_else(|| {
            unavailable_platform(
                PlatformDiagnosticKind::Wsl2DistributionIdentityUnavailable,
                "/etc/os-release is missing VERSION_ID",
            )
        })?,
    ))
}

#[cfg(target_os = "linux")]
fn read_bounded_platform_text(path: impl AsRef<Path>, max_bytes: u64) -> io::Result<String> {
    use std::io::Read as _;

    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("cannot inspect {}: {error}", path.display()),
        )
    })?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not a bounded regular control file", path.display()),
        ));
    }
    let file = fs::File::open(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("cannot open {}: {error}", path.display()),
        )
    })?;
    let mut bytes = Vec::new();
    file.take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("cannot read {}: {error}", path.display()),
            )
        })?;
    if bytes.len() as u64 > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} exceeds the bounded control-file size", path.display()),
        ));
    }
    if bytes.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} contains NUL", path.display()),
        ));
    }
    String::from_utf8(bytes).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not valid UTF-8: {error}", path.display()),
        )
    })
}

#[cfg(target_os = "linux")]
fn read_linux_mountinfo() -> Result<String, PlatformBoundaryError> {
    read_bounded_platform_text(MOUNTINFO_PATH, MAX_MOUNTINFO_BYTES).map_err(|error| {
        unavailable_platform(
            PlatformDiagnosticKind::FilesystemObservationFailure,
            error.to_string(),
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
                    PlatformDiagnosticKind::FilesystemObservationFailure,
                    "/proc/self/mountinfo contains a malformed entry",
                )
            })?;
        if fields.len() < 7 || separator < 6 || separator + 2 >= fields.len() {
            return Err(unavailable_platform(
                PlatformDiagnosticKind::FilesystemObservationFailure,
                "/proc/self/mountinfo contains an incomplete entry",
            ));
        }
        let mount_id = fields[0].parse::<u64>().map_err(|_| {
            unavailable_platform(
                PlatformDiagnosticKind::FilesystemObservationFailure,
                "/proc/self/mountinfo contains a non-decimal mount identifier",
            )
        })?;
        let mount_point = decode_mountinfo_path(fields[4])?;
        if !mount_point.is_absolute() {
            return Err(unavailable_platform(
                PlatformDiagnosticKind::FilesystemObservationFailure,
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
                PlatformDiagnosticKind::FilesystemObservationFailure,
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
        PlatformDiagnosticKind::FilesystemObservationFailure,
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
                        PlatformDiagnosticKind::FilesystemObservationFailure,
                        format!("{} has no observable existing ancestor", path.display()),
                    ));
                }
            }
            Err(error) => {
                return Err(unavailable_platform(
                    PlatformDiagnosticKind::FilesystemObservationFailure,
                    format!("cannot inspect {}: {error}", candidate.display()),
                ));
            }
        }
    }
}

fn unavailable_platform(
    kind: PlatformDiagnosticKind,
    detail: impl Into<String>,
) -> PlatformBoundaryError {
    debug_assert_eq!(kind.class(), PlatformDiagnosticClass::Unavailable);
    PlatformBoundaryError {
        diagnostic: PlatformDiagnostic::new(kind, detail),
    }
}

fn unsupported_platform(
    kind: PlatformDiagnosticKind,
    detail: impl Into<String>,
) -> PlatformBoundaryError {
    debug_assert_eq!(kind.class(), PlatformDiagnosticClass::Unsupported);
    PlatformBoundaryError {
        diagnostic: PlatformDiagnostic::new(kind, detail),
    }
}

fn bounded_platform_diagnostic_detail(mut detail: String) -> String {
    if detail.len() <= MAX_PLATFORM_DIAGNOSTIC_DETAIL_BYTES {
        return detail;
    }
    let suffix_bytes = PLATFORM_DIAGNOSTIC_DETAIL_TRUNCATED_SUFFIX.len();
    let mut end = MAX_PLATFORM_DIAGNOSTIC_DETAIL_BYTES.saturating_sub(suffix_bytes);
    while !detail.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    detail.truncate(end);
    detail.push_str(PLATFORM_DIAGNOSTIC_DETAIL_TRUNCATED_SUFFIX);
    detail
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
    volicord_types::canonical::canonical_git_object_id(value)
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

/// Namespace effect observed for one exact recursive directory removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryTreeRemovalEffect {
    /// The primitive knows that it did not remove any part of the target.
    NotRemoved,
    /// The target was observed absent after recursive removal completed.
    Removed,
    /// Some removal may have occurred, or the resulting effect could not be
    /// observed.
    PartiallyRemovedOrUnknown,
}

impl DirectoryTreeRemovalEffect {
    /// Stable machine-readable value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotRemoved => "not_removed",
            Self::Removed => "removed",
            Self::PartiallyRemovedOrUnknown => "partially_removed_or_unknown",
        }
    }
}

/// Durability observation for the removed directory's parent namespace entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryEntryDurability {
    /// The platform synchronized the parent directory after removal.
    ParentSynchronized,
    /// Parent-directory synchronization was required but did not complete.
    ParentSynchronizationFailed,
    /// The current platform contract does not expose parent-directory
    /// synchronization.
    NotApplicable,
}

impl DirectoryEntryDurability {
    /// Stable machine-readable value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ParentSynchronized => "parent_synchronized",
            Self::ParentSynchronizationFailed => "parent_synchronization_failed",
            Self::NotApplicable => "not_applicable",
        }
    }
}

/// Namespace effect of a failed atomic no-replace file publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoReplaceFilePublicationEffect {
    /// The source and destination retained their original names.
    NamesUnchanged,
    /// The source was published at the destination name.
    Published,
    /// The namespace effect could not be established.
    Unknown,
}

/// Operation phase for an atomic no-replace file publication failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoReplaceFilePublicationPhase {
    /// The source, destination, or shared parent did not satisfy the primitive.
    Validation,
    /// The platform no-replace namespace operation failed.
    NamespacePublication,
    /// Publication succeeded but parent-directory synchronization failed.
    ParentDirectorySynchronization,
}

/// Successful atomic no-replace publication result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoReplaceFilePublicationOutcome {
    /// This invocation published the source at the destination.
    Published {
        /// Durability established for the destination's parent entry.
        durability: DirectoryEntryDurability,
    },
    /// The destination already existed and was not replaced.
    DestinationExists,
}

/// Failed atomic no-replace file publication with its namespace effect.
#[derive(Debug)]
pub struct NoReplaceFilePublicationError {
    pub phase: NoReplaceFilePublicationPhase,
    pub effect: NoReplaceFilePublicationEffect,
    pub durability: DirectoryEntryDurability,
    source: io::Error,
}

impl NoReplaceFilePublicationError {
    fn new(
        phase: NoReplaceFilePublicationPhase,
        effect: NoReplaceFilePublicationEffect,
        durability: DirectoryEntryDurability,
        source: io::Error,
    ) -> Self {
        Self {
            phase,
            effect,
            durability,
            source,
        }
    }

    /// Underlying I/O failure.
    pub fn io_error(&self) -> &io::Error {
        &self.source
    }
}

impl fmt::Display for NoReplaceFilePublicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "no-replace file publication failed during {:?} (effect: {:?}, durability: {}): {}",
            self.phase,
            self.effect,
            self.durability.as_str(),
            self.source
        )
    }
}

impl std::error::Error for NoReplaceFilePublicationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Operation that failed during exact directory-tree removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryTreeRemovalPhase {
    TargetInspection,
    RecursiveRemoval,
    PostRemovalInspection,
    ParentDirectorySynchronization,
}

impl DirectoryTreeRemovalPhase {
    /// Stable machine-readable value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TargetInspection => "target_inspection",
            Self::RecursiveRemoval => "recursive_removal",
            Self::PostRemovalInspection => "post_removal_inspection",
            Self::ParentDirectorySynchronization => "parent_directory_synchronization",
        }
    }
}

/// Last exact-path observation made by the removal primitive.
///
/// This is an observation at one instant. `Absent` does not assert that another
/// process cannot recreate the path after the observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryTreeTargetState {
    Present,
    Absent,
    Unknown,
}

impl DirectoryTreeTargetState {
    /// Stable machine-readable value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Absent => "absent",
            Self::Unknown => "unknown",
        }
    }
}

/// Successful exact directory-tree removal facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectoryTreeRemovalOutcome {
    pub effect: DirectoryTreeRemovalEffect,
    pub durability: DirectoryEntryDurability,
    pub target_state: DirectoryTreeTargetState,
}

/// Failed exact directory-tree removal facts and their underlying I/O errors.
#[derive(Debug)]
pub struct DirectoryTreeRemovalError {
    pub phase: DirectoryTreeRemovalPhase,
    pub effect: DirectoryTreeRemovalEffect,
    pub durability: DirectoryEntryDurability,
    pub target_state: DirectoryTreeTargetState,
    source: io::Error,
    preceding_error: Option<io::Error>,
}

impl DirectoryTreeRemovalError {
    fn new(
        phase: DirectoryTreeRemovalPhase,
        effect: DirectoryTreeRemovalEffect,
        durability: DirectoryEntryDurability,
        target_state: DirectoryTreeTargetState,
        source: io::Error,
    ) -> Self {
        Self {
            phase,
            effect,
            durability,
            target_state,
            source,
            preceding_error: None,
        }
    }

    fn after_error(
        phase: DirectoryTreeRemovalPhase,
        effect: DirectoryTreeRemovalEffect,
        durability: DirectoryEntryDurability,
        target_state: DirectoryTreeTargetState,
        source: io::Error,
        preceding_error: io::Error,
    ) -> Self {
        Self {
            phase,
            effect,
            durability,
            target_state,
            source,
            preceding_error: Some(preceding_error),
        }
    }

    /// I/O error from the operation identified by `phase`.
    pub fn io_error(&self) -> &io::Error {
        &self.source
    }

    /// Earlier recursive-removal error retained when post-removal inspection
    /// also failed.
    pub fn preceding_io_error(&self) -> Option<&io::Error> {
        self.preceding_error.as_ref()
    }
}

impl fmt::Display for DirectoryTreeRemovalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "directory-tree removal failed during {} (effect: {}, target: {}, durability: {}): {}",
            self.phase.as_str(),
            self.effect.as_str(),
            self.target_state.as_str(),
            self.durability.as_str(),
            self.source
        )?;
        if let Some(preceding) = &self.preceding_error {
            write!(
                formatter,
                "; preceding recursive-removal error: {preceding}"
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for DirectoryTreeRemovalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Removes one exact ordinary directory tree and synchronizes its parent where
/// the platform exposes directory synchronization.
///
/// Callers must establish product ownership immediately before invoking this
/// primitive. This function rejects symlinks and non-directory namespace
/// entries; it does not infer product ownership from a path. Its exact-path
/// observations describe the completed operation only and do not prevent a
/// later path recreation.
pub fn remove_owned_directory_tree(
    path: &Path,
) -> Result<DirectoryTreeRemovalOutcome, DirectoryTreeRemovalError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| {
        let target_state = if matches!(
            source.kind(),
            io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
        ) {
            DirectoryTreeTargetState::Absent
        } else {
            DirectoryTreeTargetState::Unknown
        };
        DirectoryTreeRemovalError::new(
            DirectoryTreeRemovalPhase::TargetInspection,
            DirectoryTreeRemovalEffect::NotRemoved,
            DirectoryEntryDurability::NotApplicable,
            target_state,
            source,
        )
    })?;
    if !metadata.file_type().is_dir() {
        return Err(DirectoryTreeRemovalError::new(
            DirectoryTreeRemovalPhase::TargetInspection,
            DirectoryTreeRemovalEffect::NotRemoved,
            DirectoryEntryDurability::NotApplicable,
            DirectoryTreeTargetState::Present,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "owned directory removal requires an ordinary directory",
            ),
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        DirectoryTreeRemovalError::new(
            DirectoryTreeRemovalPhase::TargetInspection,
            DirectoryTreeRemovalEffect::NotRemoved,
            DirectoryEntryDurability::NotApplicable,
            DirectoryTreeTargetState::Present,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "owned directory removal requires a parent directory",
            ),
        )
    })?;

    #[cfg(any(test, feature = "test-support"))]
    let fault = directory_tree_removal_test_support::take_next_fault();

    #[cfg(any(test, feature = "test-support"))]
    if matches!(
        fault,
        Some(
            directory_tree_removal_test_support::DirectoryTreeRemovalFault::BeforeRecursiveRemoval
        )
    ) {
        return Err(DirectoryTreeRemovalError::new(
            DirectoryTreeRemovalPhase::RecursiveRemoval,
            DirectoryTreeRemovalEffect::NotRemoved,
            DirectoryEntryDurability::NotApplicable,
            DirectoryTreeTargetState::Present,
            injected_removal_error("before recursive removal"),
        ));
    }

    #[cfg(any(test, feature = "test-support"))]
    if matches!(
        fault,
        Some(
            directory_tree_removal_test_support::DirectoryTreeRemovalFault::PostRemovalInspectionFailure
        )
    ) {
        return Err(DirectoryTreeRemovalError::after_error(
            DirectoryTreeRemovalPhase::PostRemovalInspection,
            DirectoryTreeRemovalEffect::PartiallyRemovedOrUnknown,
            DirectoryEntryDurability::NotApplicable,
            DirectoryTreeTargetState::Unknown,
            injected_removal_error("during post-removal inspection"),
            injected_removal_error("during recursive removal"),
        ));
    }

    #[cfg(any(test, feature = "test-support"))]
    if matches!(
        fault,
        Some(
            directory_tree_removal_test_support::DirectoryTreeRemovalFault::RecursiveRemovalAfterPartialEffect
        )
    ) {
        remove_one_test_entry(path).map_err(|source| {
            DirectoryTreeRemovalError::new(
                DirectoryTreeRemovalPhase::RecursiveRemoval,
                DirectoryTreeRemovalEffect::PartiallyRemovedOrUnknown,
                DirectoryEntryDurability::NotApplicable,
                DirectoryTreeTargetState::Present,
                source,
            )
        })?;
        return classify_recursive_removal_error(
            path,
            injected_removal_error("after partial recursive removal"),
        );
    }

    if let Err(source) = fs::remove_dir_all(path) {
        return classify_recursive_removal_error(path, source);
    }

    #[cfg(any(test, feature = "test-support"))]
    if matches!(
        fault,
        Some(
            directory_tree_removal_test_support::DirectoryTreeRemovalFault::AfterRecursiveRemovalBeforeParentSync
        )
    ) {
        return Err(DirectoryTreeRemovalError::new(
            DirectoryTreeRemovalPhase::RecursiveRemoval,
            DirectoryTreeRemovalEffect::Removed,
            failed_or_unavailable_parent_durability(),
            DirectoryTreeTargetState::Absent,
            injected_removal_error("after recursive removal before parent synchronization"),
        ));
    }

    #[cfg(all(unix, any(test, feature = "test-support")))]
    if matches!(
        fault,
        Some(
            directory_tree_removal_test_support::DirectoryTreeRemovalFault::ParentDirectorySyncFailure
        )
    ) {
        return Err(DirectoryTreeRemovalError::new(
            DirectoryTreeRemovalPhase::ParentDirectorySynchronization,
            DirectoryTreeRemovalEffect::Removed,
            failed_or_unavailable_parent_durability(),
            DirectoryTreeTargetState::Absent,
            injected_removal_error("during parent-directory synchronization"),
        ));
    }

    sync_directory_entry_parent(parent).map_err(|source| {
        DirectoryTreeRemovalError::new(
            DirectoryTreeRemovalPhase::ParentDirectorySynchronization,
            DirectoryTreeRemovalEffect::Removed,
            DirectoryEntryDurability::ParentSynchronizationFailed,
            DirectoryTreeTargetState::Absent,
            source,
        )
    })?;
    Ok(DirectoryTreeRemovalOutcome {
        effect: DirectoryTreeRemovalEffect::Removed,
        durability: supported_parent_durability(),
        target_state: DirectoryTreeTargetState::Absent,
    })
}

fn classify_recursive_removal_error(
    path: &Path,
    removal_error: io::Error,
) -> Result<DirectoryTreeRemovalOutcome, DirectoryTreeRemovalError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(DirectoryTreeRemovalError::new(
            DirectoryTreeRemovalPhase::RecursiveRemoval,
            DirectoryTreeRemovalEffect::PartiallyRemovedOrUnknown,
            DirectoryEntryDurability::NotApplicable,
            DirectoryTreeTargetState::Present,
            removal_error,
        )),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Err(DirectoryTreeRemovalError::new(
                DirectoryTreeRemovalPhase::RecursiveRemoval,
                DirectoryTreeRemovalEffect::Removed,
                failed_or_unavailable_parent_durability(),
                DirectoryTreeTargetState::Absent,
                removal_error,
            ))
        }
        Err(inspection_error) => Err(DirectoryTreeRemovalError::after_error(
            DirectoryTreeRemovalPhase::PostRemovalInspection,
            DirectoryTreeRemovalEffect::PartiallyRemovedOrUnknown,
            DirectoryEntryDurability::NotApplicable,
            DirectoryTreeTargetState::Unknown,
            inspection_error,
            removal_error,
        )),
    }
}

#[cfg(unix)]
const fn supported_parent_durability() -> DirectoryEntryDurability {
    DirectoryEntryDurability::ParentSynchronized
}

#[cfg(not(unix))]
const fn supported_parent_durability() -> DirectoryEntryDurability {
    DirectoryEntryDurability::NotApplicable
}

#[cfg(unix)]
const fn failed_or_unavailable_parent_durability() -> DirectoryEntryDurability {
    DirectoryEntryDurability::ParentSynchronizationFailed
}

#[cfg(not(unix))]
const fn failed_or_unavailable_parent_durability() -> DirectoryEntryDurability {
    DirectoryEntryDurability::NotApplicable
}

#[cfg(unix)]
fn sync_directory_entry_parent(parent: &Path) -> io::Result<()> {
    fs::File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory_entry_parent(_parent: &Path) -> io::Result<()> {
    Ok(())
}

/// Atomically publishes one ordinary file without replacing an existing
/// destination and synchronizes the shared parent where supported.
///
/// The source and destination must be distinct names in the same parent
/// directory. Keeping both names in one directory establishes the
/// same-filesystem boundary required by the platform rename primitive.
pub fn publish_file_no_replace(
    source: &Path,
    destination: &Path,
) -> Result<NoReplaceFilePublicationOutcome, NoReplaceFilePublicationError> {
    let source_parent = source.parent().ok_or_else(|| {
        NoReplaceFilePublicationError::new(
            NoReplaceFilePublicationPhase::Validation,
            NoReplaceFilePublicationEffect::NamesUnchanged,
            DirectoryEntryDurability::NotApplicable,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "no-replace file publication requires a source parent",
            ),
        )
    })?;
    let destination_parent = destination.parent().ok_or_else(|| {
        NoReplaceFilePublicationError::new(
            NoReplaceFilePublicationPhase::Validation,
            NoReplaceFilePublicationEffect::NamesUnchanged,
            DirectoryEntryDurability::NotApplicable,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "no-replace file publication requires a destination parent",
            ),
        )
    })?;
    if source_parent != destination_parent || source == destination {
        return Err(NoReplaceFilePublicationError::new(
            NoReplaceFilePublicationPhase::Validation,
            NoReplaceFilePublicationEffect::NamesUnchanged,
            DirectoryEntryDurability::NotApplicable,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "no-replace file publication requires distinct names in one parent directory",
            ),
        ));
    }

    let source_metadata = fs::symlink_metadata(source).map_err(|source| {
        NoReplaceFilePublicationError::new(
            NoReplaceFilePublicationPhase::Validation,
            NoReplaceFilePublicationEffect::NamesUnchanged,
            DirectoryEntryDurability::NotApplicable,
            source,
        )
    })?;
    if !source_metadata.file_type().is_file() {
        return Err(NoReplaceFilePublicationError::new(
            NoReplaceFilePublicationPhase::Validation,
            NoReplaceFilePublicationEffect::NamesUnchanged,
            DirectoryEntryDurability::NotApplicable,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "no-replace file publication requires an ordinary source file",
            ),
        ));
    }
    match fs::symlink_metadata(destination) {
        Ok(_) => return Ok(NoReplaceFilePublicationOutcome::DestinationExists),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(NoReplaceFilePublicationError::new(
                NoReplaceFilePublicationPhase::Validation,
                NoReplaceFilePublicationEffect::NamesUnchanged,
                DirectoryEntryDurability::NotApplicable,
                source,
            ));
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    let fault = no_replace_file_publication_test_support::take_next_fault();

    #[cfg(any(test, feature = "test-support"))]
    if matches!(
        fault,
        Some(
            no_replace_file_publication_test_support::NoReplaceFilePublicationFault::BeforeNamespacePublication
        )
    ) {
        return Err(NoReplaceFilePublicationError::new(
            NoReplaceFilePublicationPhase::NamespacePublication,
            NoReplaceFilePublicationEffect::NamesUnchanged,
            DirectoryEntryDurability::NotApplicable,
            injected_file_publication_error("before namespace publication"),
        ));
    }

    #[cfg(any(test, feature = "test-support"))]
    if matches!(
        fault,
        Some(
            no_replace_file_publication_test_support::NoReplaceFilePublicationFault::NamespaceEffectUnknown
        )
    ) {
        return Err(NoReplaceFilePublicationError::new(
            NoReplaceFilePublicationPhase::NamespacePublication,
            NoReplaceFilePublicationEffect::Unknown,
            DirectoryEntryDurability::NotApplicable,
            injected_file_publication_error("with an unknown namespace effect"),
        ));
    }

    match move_path_no_replace(source, destination) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Ok(NoReplaceFilePublicationOutcome::DestinationExists);
        }
        Err(source) => {
            return Err(NoReplaceFilePublicationError::new(
                NoReplaceFilePublicationPhase::NamespacePublication,
                NoReplaceFilePublicationEffect::NamesUnchanged,
                DirectoryEntryDurability::NotApplicable,
                source,
            ));
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    if matches!(
        fault,
        Some(
            no_replace_file_publication_test_support::NoReplaceFilePublicationFault::ParentDirectorySynchronizationFailure
        )
    ) {
        return Err(NoReplaceFilePublicationError::new(
            NoReplaceFilePublicationPhase::ParentDirectorySynchronization,
            NoReplaceFilePublicationEffect::Published,
            failed_or_unavailable_parent_durability(),
            injected_file_publication_error("during parent-directory synchronization"),
        ));
    }

    sync_directory_entry_parent(destination_parent).map_err(|source| {
        NoReplaceFilePublicationError::new(
            NoReplaceFilePublicationPhase::ParentDirectorySynchronization,
            NoReplaceFilePublicationEffect::Published,
            failed_or_unavailable_parent_durability(),
            source,
        )
    })?;
    Ok(NoReplaceFilePublicationOutcome::Published {
        durability: supported_parent_durability(),
    })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn move_path_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use rustix::fs::{renameat_with, RenameFlags, CWD};

    renameat_with(CWD, source, CWD, destination, RenameFlags::NOREPLACE).map_err(io::Error::from)
}

#[cfg(windows)]
fn move_path_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    move_file_no_replace(source, destination)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn move_path_no_replace(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic no-replace file publication is unsupported on this platform",
    ))
}

#[cfg(any(test, feature = "test-support"))]
fn injected_file_publication_error(point: &'static str) -> io::Error {
    io::Error::other(format!(
        "injected no-replace file publication failure {point}"
    ))
}

/// Repository-owned fault support for no-replace file-publication tests.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod no_replace_file_publication_test_support {
    use std::cell::Cell;

    /// One failure injected into the next publication on the current thread.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum NoReplaceFilePublicationFault {
        BeforeNamespacePublication,
        NamespaceEffectUnknown,
        ParentDirectorySynchronizationFailure,
    }

    thread_local! {
        static NEXT_FAULT: Cell<Option<NoReplaceFilePublicationFault>> = const { Cell::new(None) };
    }

    /// Arms one current-thread failure for the next publication primitive.
    pub fn fail_next_no_replace_file_publication(fault: NoReplaceFilePublicationFault) {
        NEXT_FAULT.with(|next| {
            assert!(
                next.replace(Some(fault)).is_none(),
                "a no-replace file-publication fault is already armed on this test thread"
            );
        });
    }

    pub(super) fn take_next_fault() -> Option<NoReplaceFilePublicationFault> {
        NEXT_FAULT.with(Cell::take)
    }
}

#[cfg(any(test, feature = "test-support"))]
fn injected_removal_error(point: &'static str) -> io::Error {
    io::Error::other(format!("injected directory-tree removal failure {point}"))
}

#[cfg(any(test, feature = "test-support"))]
fn remove_one_test_entry(path: &Path) -> io::Result<()> {
    let entry = fs::read_dir(path)?
        .next()
        .transpose()?
        .ok_or_else(|| io::Error::other("partial-removal fixture has no child entry"))?;
    let metadata = fs::symlink_metadata(entry.path())?;
    if metadata.file_type().is_dir() {
        fs::remove_dir_all(entry.path())
    } else {
        fs::remove_file(entry.path())
    }
}

/// Repository-owned fault support for directory-removal contract tests.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod directory_tree_removal_test_support {
    use std::cell::Cell;

    /// One failure injected into the next removal on the current test thread.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DirectoryTreeRemovalFault {
        BeforeRecursiveRemoval,
        RecursiveRemovalAfterPartialEffect,
        AfterRecursiveRemovalBeforeParentSync,
        ParentDirectorySyncFailure,
        PostRemovalInspectionFailure,
    }

    thread_local! {
        static NEXT_FAULT: Cell<Option<DirectoryTreeRemovalFault>> = const { Cell::new(None) };
    }

    /// Arms one current-thread failure for the next removal primitive call.
    pub fn fail_next_directory_tree_removal(fault: DirectoryTreeRemovalFault) {
        NEXT_FAULT.with(|next| {
            assert!(
                next.replace(Some(fault)).is_none(),
                "a directory-tree removal fault is already armed on this test thread"
            );
        });
    }

    pub(super) fn take_next_fault() -> Option<DirectoryTreeRemovalFault> {
        NEXT_FAULT.with(Cell::take)
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
        collections::BTreeSet,
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc, Barrier,
        },
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn no_replace_file_publication_preserves_an_existing_destination() -> io::Result<()> {
        let directory = TestDirectory::new("no-replace-file-publication")?;
        let first_source = directory.path().join("first.staging");
        let second_source = directory.path().join("second.staging");
        let destination = directory.path().join("diagnostics.sqlite");
        fs::write(&first_source, b"first complete database")?;
        fs::write(&second_source, b"second complete database")?;

        let first = publish_file_no_replace(&first_source, &destination)
            .map_err(|error| io::Error::other(error.to_string()))?;
        assert_eq!(
            first,
            NoReplaceFilePublicationOutcome::Published {
                durability: supported_parent_durability(),
            }
        );
        assert!(!first_source.exists());
        assert_eq!(fs::read(&destination)?, b"first complete database");

        let second = publish_file_no_replace(&second_source, &destination)
            .map_err(|error| io::Error::other(error.to_string()))?;
        assert_eq!(second, NoReplaceFilePublicationOutcome::DestinationExists);
        assert_eq!(fs::read(&destination)?, b"first complete database");
        assert_eq!(fs::read(&second_source)?, b"second complete database");
        Ok(())
    }

    #[test]
    fn concurrent_no_replace_file_publishers_select_exactly_one_winner() -> io::Result<()> {
        let directory = TestDirectory::new("concurrent-no-replace-file-publication")?;
        let destination = directory.path().join("diagnostics.sqlite");
        let barrier = Arc::new(Barrier::new(3));
        let creators = [
            ("first.staging", b"first".as_slice()),
            ("second.staging", b"second".as_slice()),
        ]
        .map(|(name, bytes)| {
            let source = directory.path().join(name);
            fs::write(&source, bytes).expect("staging source");
            let destination = destination.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let outcome = publish_file_no_replace(&source, &destination)
                    .map_err(|error| error.to_string());
                (source, outcome)
            })
        });

        barrier.wait();
        let results = creators.map(|creator| creator.join().expect("publisher thread"));
        let published = results
            .iter()
            .filter(|(_, outcome)| {
                matches!(
                    outcome,
                    Ok(NoReplaceFilePublicationOutcome::Published { .. })
                )
            })
            .count();
        let destination_exists = results
            .iter()
            .filter(|(_, outcome)| {
                matches!(
                    outcome,
                    Ok(NoReplaceFilePublicationOutcome::DestinationExists)
                )
            })
            .count();
        assert_eq!(published, 1);
        assert_eq!(destination_exists, 1);
        let published_bytes = fs::read(&destination)?;
        assert!(published_bytes == b"first" || published_bytes == b"second");
        assert_eq!(
            results.iter().filter(|(source, _)| source.exists()).count(),
            1,
            "only the losing source name remains for caller-owned cleanup"
        );
        Ok(())
    }

    #[test]
    fn no_replace_file_publication_rejects_cross_parent_sources_without_effect() -> io::Result<()> {
        let directory = TestDirectory::new("no-replace-file-cross-parent")?;
        let other_parent = directory.path().join("other");
        fs::create_dir(&other_parent)?;
        let source = directory.path().join("source.staging");
        let destination = other_parent.join("diagnostics.sqlite");
        fs::write(&source, b"complete database")?;

        let error = publish_file_no_replace(&source, &destination)
            .expect_err("cross-parent publication must be rejected");
        assert_eq!(error.phase, NoReplaceFilePublicationPhase::Validation);
        assert_eq!(error.effect, NoReplaceFilePublicationEffect::NamesUnchanged);
        assert_eq!(fs::read(&source)?, b"complete database");
        assert!(!destination.exists());
        Ok(())
    }

    #[test]
    fn file_publication_faults_preserve_typed_namespace_effects() -> io::Result<()> {
        let directory = TestDirectory::new("no-replace-file-effects")?;
        for (fault, expected_effect) in [
            (
                no_replace_file_publication_test_support::NoReplaceFilePublicationFault::BeforeNamespacePublication,
                NoReplaceFilePublicationEffect::NamesUnchanged,
            ),
            (
                no_replace_file_publication_test_support::NoReplaceFilePublicationFault::NamespaceEffectUnknown,
                NoReplaceFilePublicationEffect::Unknown,
            ),
        ] {
            let source = directory.path().join(format!("source-{expected_effect:?}"));
            let destination = directory
                .path()
                .join(format!("destination-{expected_effect:?}"));
            fs::write(&source, b"complete database")?;
            no_replace_file_publication_test_support::fail_next_no_replace_file_publication(fault);

            let error =
                publish_file_no_replace(&source, &destination).expect_err("fault must fail");
            assert_eq!(error.effect, expected_effect);
            assert_eq!(
                error.phase,
                NoReplaceFilePublicationPhase::NamespacePublication
            );
            assert_eq!(fs::read(&source)?, b"complete database");
            assert!(!destination.exists());
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn parent_sync_failure_preserves_successful_file_publication() -> io::Result<()> {
        let directory = TestDirectory::new("no-replace-file-parent-sync")?;
        let source = directory.path().join("source.staging");
        let destination = directory.path().join("diagnostics.sqlite");
        fs::write(&source, b"complete database")?;
        no_replace_file_publication_test_support::fail_next_no_replace_file_publication(
            no_replace_file_publication_test_support::NoReplaceFilePublicationFault::ParentDirectorySynchronizationFailure,
        );

        let error = publish_file_no_replace(&source, &destination)
            .expect_err("parent synchronization must fail");
        assert_eq!(
            error.phase,
            NoReplaceFilePublicationPhase::ParentDirectorySynchronization
        );
        assert_eq!(error.effect, NoReplaceFilePublicationEffect::Published);
        assert_eq!(
            error.durability,
            DirectoryEntryDurability::ParentSynchronizationFailed
        );
        assert!(!source.exists());
        assert_eq!(fs::read(destination)?, b"complete database");
        Ok(())
    }

    #[test]
    fn owned_directory_removal_rejects_non_directories_and_removes_exact_tree() -> io::Result<()> {
        let directory = TestDirectory::new("owned-directory-removal")?;
        let owned = directory.path().join("owned");
        fs::create_dir(&owned)?;
        fs::write(owned.join("record"), b"owned")?;

        let outcome = remove_owned_directory_tree(&owned)
            .map_err(|error| io::Error::other(error.to_string()))?;
        assert_eq!(outcome.effect, DirectoryTreeRemovalEffect::Removed);
        assert_eq!(outcome.target_state, DirectoryTreeTargetState::Absent);
        #[cfg(unix)]
        assert_eq!(
            outcome.durability,
            DirectoryEntryDurability::ParentSynchronized
        );
        #[cfg(not(unix))]
        assert_eq!(outcome.durability, DirectoryEntryDurability::NotApplicable);
        assert!(!owned.exists());

        let file = directory.path().join("ordinary-file");
        fs::write(&file, b"preserve")?;
        let error =
            remove_owned_directory_tree(&file).expect_err("ordinary file must not be removed");
        assert_eq!(error.phase, DirectoryTreeRemovalPhase::TargetInspection);
        assert_eq!(error.effect, DirectoryTreeRemovalEffect::NotRemoved);
        assert_eq!(error.target_state, DirectoryTreeTargetState::Present);
        assert_eq!(error.io_error().kind(), io::ErrorKind::InvalidInput);
        assert_eq!(fs::read(file)?, b"preserve");
        Ok(())
    }

    #[test]
    fn removal_fault_before_recursive_effect_keeps_the_target() -> io::Result<()> {
        let directory = TestDirectory::new("removal-before-recursive")?;
        let owned = directory.path().join("owned");
        fs::create_dir(&owned)?;
        fs::write(owned.join("record"), b"owned")?;
        directory_tree_removal_test_support::fail_next_directory_tree_removal(
            directory_tree_removal_test_support::DirectoryTreeRemovalFault::BeforeRecursiveRemoval,
        );

        let error = remove_owned_directory_tree(&owned).expect_err("injected removal must fail");

        assert_eq!(error.phase, DirectoryTreeRemovalPhase::RecursiveRemoval);
        assert_eq!(error.effect, DirectoryTreeRemovalEffect::NotRemoved);
        assert_eq!(error.target_state, DirectoryTreeTargetState::Present);
        assert!(owned.is_dir());
        assert_eq!(fs::read(owned.join("record"))?, b"owned");
        Ok(())
    }

    #[test]
    fn recursive_failure_after_partial_effect_is_observed_without_retry() -> io::Result<()> {
        let directory = TestDirectory::new("removal-partial-effect")?;
        let owned = directory.path().join("owned");
        fs::create_dir(&owned)?;
        fs::write(owned.join("first"), b"first")?;
        fs::write(owned.join("second"), b"second")?;
        directory_tree_removal_test_support::fail_next_directory_tree_removal(
            directory_tree_removal_test_support::DirectoryTreeRemovalFault::RecursiveRemovalAfterPartialEffect,
        );

        let error =
            remove_owned_directory_tree(&owned).expect_err("injected partial removal must fail");

        assert_eq!(error.phase, DirectoryTreeRemovalPhase::RecursiveRemoval);
        assert_eq!(
            error.effect,
            DirectoryTreeRemovalEffect::PartiallyRemovedOrUnknown
        );
        assert_eq!(error.target_state, DirectoryTreeTargetState::Present);
        assert!(owned.is_dir());
        assert_eq!(fs::read_dir(&owned)?.count(), 1);
        Ok(())
    }

    #[test]
    fn failed_post_removal_inspection_retains_both_io_errors() -> io::Result<()> {
        let directory = TestDirectory::new("removal-post-inspection")?;
        let owned = directory.path().join("owned");
        fs::create_dir(&owned)?;
        directory_tree_removal_test_support::fail_next_directory_tree_removal(
            directory_tree_removal_test_support::DirectoryTreeRemovalFault::PostRemovalInspectionFailure,
        );

        let error =
            remove_owned_directory_tree(&owned).expect_err("post-removal inspection must fail");

        assert_eq!(
            error.phase,
            DirectoryTreeRemovalPhase::PostRemovalInspection
        );
        assert_eq!(
            error.effect,
            DirectoryTreeRemovalEffect::PartiallyRemovedOrUnknown
        );
        assert_eq!(error.target_state, DirectoryTreeTargetState::Unknown);
        assert!(error
            .io_error()
            .to_string()
            .contains("post-removal inspection"));
        assert!(error
            .preceding_io_error()
            .is_some_and(|source| source.to_string().contains("recursive removal")));
        assert!(owned.is_dir());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn parent_sync_failure_preserves_known_removal_effect() -> io::Result<()> {
        let directory = TestDirectory::new("removal-parent-sync")?;
        let owned = directory.path().join("owned");
        fs::create_dir(&owned)?;
        fs::write(owned.join("record"), b"owned")?;
        directory_tree_removal_test_support::fail_next_directory_tree_removal(
            directory_tree_removal_test_support::DirectoryTreeRemovalFault::ParentDirectorySyncFailure,
        );

        let error =
            remove_owned_directory_tree(&owned).expect_err("parent synchronization must fail");

        assert_eq!(
            error.phase,
            DirectoryTreeRemovalPhase::ParentDirectorySynchronization
        );
        assert_eq!(error.effect, DirectoryTreeRemovalEffect::Removed);
        assert_eq!(error.target_state, DirectoryTreeTargetState::Absent);
        assert_eq!(
            error.durability,
            DirectoryEntryDurability::ParentSynchronizationFailed
        );
        assert!(!owned.exists());
        Ok(())
    }

    #[test]
    fn post_recursive_pre_sync_fault_preserves_known_removal_effect() -> io::Result<()> {
        let directory = TestDirectory::new("removal-after-recursive")?;
        let owned = directory.path().join("owned");
        fs::create_dir(&owned)?;
        fs::write(owned.join("record"), b"owned")?;
        directory_tree_removal_test_support::fail_next_directory_tree_removal(
            directory_tree_removal_test_support::DirectoryTreeRemovalFault::AfterRecursiveRemovalBeforeParentSync,
        );

        let error =
            remove_owned_directory_tree(&owned).expect_err("post-recursive fault must fail");

        assert_eq!(error.phase, DirectoryTreeRemovalPhase::RecursiveRemoval);
        assert_eq!(error.effect, DirectoryTreeRemovalEffect::Removed);
        assert_eq!(error.target_state, DirectoryTreeTargetState::Absent);
        assert!(!owned.exists());
        Ok(())
    }

    static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn platform_diagnostic_registry_has_valid_unique_namespaced_codes() {
        let codes = PlatformDiagnosticKind::ALL
            .into_iter()
            .map(PlatformDiagnosticKind::code)
            .collect::<Vec<_>>();
        let unique = codes.iter().copied().collect::<BTreeSet<_>>();

        assert_eq!(codes.len(), PlatformDiagnosticKind::ALL.len());
        assert_eq!(unique.len(), codes.len());
        for code in codes {
            assert!(code.starts_with("platform."));
            DiagnosticCode::parse(code).expect("platform registry code");
        }
    }

    #[test]
    fn platform_diagnostic_kinds_map_to_the_canonical_codes() {
        let expected = [
            (
                PlatformDiagnosticKind::UnsupportedOperatingSystem,
                "platform.operating_system.unsupported",
            ),
            (
                PlatformDiagnosticKind::UnsupportedTarget,
                "platform.target.unsupported",
            ),
            (PlatformDiagnosticKind::Wsl1, "platform.wsl1.unsupported"),
            (
                PlatformDiagnosticKind::Wsl2DistributionIdentityUnavailable,
                "platform.wsl2.distribution_identity_unavailable",
            ),
            (
                PlatformDiagnosticKind::UnsupportedWsl2Distribution,
                "platform.wsl2.distribution_unsupported",
            ),
            (
                PlatformDiagnosticKind::FilesystemObservationFailure,
                "platform.filesystem.observation_failed",
            ),
            (
                PlatformDiagnosticKind::ProductRepositoryNotFound,
                "platform.product_repository.not_found",
            ),
            (
                PlatformDiagnosticKind::InvalidProductRepositoryRoot,
                "platform.product_repository.invalid_root",
            ),
            (
                PlatformDiagnosticKind::ProductPathNotFound,
                "platform.product_path.not_found",
            ),
            (
                PlatformDiagnosticKind::ProductPathInaccessible,
                "platform.product_path.inaccessible",
            ),
            (
                PlatformDiagnosticKind::ProductPathContainmentFailure,
                "platform.product_path.containment_failed",
            ),
            (
                PlatformDiagnosticKind::UnsupportedFilesystemBoundary,
                "platform.filesystem.unsupported",
            ),
            (
                PlatformDiagnosticKind::PlatformObservationFailure,
                "platform.observation.failed",
            ),
        ];

        assert_eq!(PlatformDiagnosticKind::ALL, expected.map(|(kind, _)| kind));
        for (kind, code) in expected {
            assert_eq!(kind.code(), code);
        }
    }

    #[test]
    fn platform_finding_uses_the_kind_identity_and_current_fact_fields() {
        let diagnostic = PlatformDiagnostic::new(
            PlatformDiagnosticKind::UnsupportedOperatingSystem,
            "display prose does not classify this error",
        );
        let finding = platform_diagnostic_finding(
            &diagnostic,
            "finding.platform.unsupported_fixture",
            &PlatformDiagnosticFacts {
                target_triple: Some("x86_64-unknown-linux-gnu".to_owned()),
                platform_environment: Some("linux".to_owned()),
            },
            UtcTimestamp::parse("2026-07-22T00:00:00Z").expect("timestamp"),
        )
        .expect("platform finding");
        assert_eq!(
            finding.code().as_str(),
            "platform.operating_system.unsupported"
        );
        assert_eq!(
            finding.actions()[0].code().as_str(),
            "action.platform.use_supported_environment"
        );
        assert_eq!(
            finding
                .facts()
                .data()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["platform_environment", "target_triple"]
        );
    }

    #[test]
    fn platform_action_selection_uses_the_typed_diagnostic_class() {
        let unavailable = PlatformDiagnostic::new(
            PlatformDiagnosticKind::FilesystemObservationFailure,
            "filesystem observation failed",
        );
        let finding = platform_diagnostic_finding(
            &unavailable,
            "finding.platform.observation_fixture",
            &PlatformDiagnosticFacts::default(),
            UtcTimestamp::parse("2026-07-22T00:00:00Z").expect("timestamp"),
        )
        .expect("platform finding");

        assert_eq!(
            finding.actions()[0].code().as_str(),
            "action.platform.repair_observation_access"
        );
    }

    #[test]
    fn platform_display_uses_canonical_code_and_bounded_detail() {
        let error = unavailable_platform(
            PlatformDiagnosticKind::PlatformObservationFailure,
            "가".repeat(MAX_PLATFORM_DIAGNOSTIC_DETAIL_BYTES),
        );

        assert_eq!(
            error.kind(),
            PlatformDiagnosticKind::PlatformObservationFailure
        );
        assert_eq!(error.code(), "platform.observation.failed");
        assert!(error.detail().len() <= MAX_PLATFORM_DIAGNOSTIC_DETAIL_BYTES);
        assert!(error
            .detail()
            .ends_with(PLATFORM_DIAGNOSTIC_DETAIL_TRUNCATED_SUFFIX));
        assert_eq!(
            error.to_string(),
            format!("{}: {}", error.code(), error.detail())
        );
    }

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

    #[cfg(target_os = "linux")]
    enum TestPlatformRead {
        Text(&'static str),
        Failure(&'static str),
    }

    #[cfg(target_os = "linux")]
    impl TestPlatformRead {
        fn result(&self) -> io::Result<String> {
            match self {
                Self::Text(value) => Ok((*value).to_owned()),
                Self::Failure(detail) => Err(io::Error::other(*detail)),
            }
        }
    }

    #[cfg(target_os = "linux")]
    struct TestLinuxPlatformObservation {
        kernel_release: TestPlatformRead,
        os_release: TestPlatformRead,
        kernel_release_reads: usize,
        os_release_reads: usize,
    }

    #[cfg(target_os = "linux")]
    impl TestLinuxPlatformObservation {
        fn new(kernel_release: TestPlatformRead, os_release: TestPlatformRead) -> Self {
            Self {
                kernel_release,
                os_release,
                kernel_release_reads: 0,
                os_release_reads: 0,
            }
        }
    }

    #[cfg(target_os = "linux")]
    impl LinuxPlatformObservation for TestLinuxPlatformObservation {
        fn read_kernel_release(&mut self) -> io::Result<String> {
            self.kernel_release_reads += 1;
            self.kernel_release.result()
        }

        fn read_wsl2_os_release(&mut self) -> io::Result<String> {
            self.os_release_reads += 1;
            self.os_release.result()
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
    fn kernel_read_failure_reports_canonical_platform_observation_code() {
        let mut observation = TestLinuxPlatformObservation::new(
            TestPlatformRead::Failure("injected kernel read failure"),
            TestPlatformRead::Failure("os-release must not be read"),
        );
        let error = observe_linux_platform_boundary(
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            &mut observation,
        )
        .expect_err("an unavailable kernel observation must fail closed");
        assert_eq!(error.class(), PlatformDiagnosticClass::Unavailable);
        assert_eq!(
            error.kind(),
            PlatformDiagnosticKind::PlatformObservationFailure
        );
        assert_eq!(error.code(), "platform.observation.failed");
        assert_eq!(
            error.detail(),
            "the Linux kernel release required to classify the host could not be read from /proc/sys/kernel/osrelease: injected kernel read failure"
        );
        assert_eq!(observation.kernel_release_reads, 1);
        assert_eq!(observation.os_release_reads, 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn native_linux_observation_does_not_read_os_release() {
        let mut observation = TestLinuxPlatformObservation::new(
            TestPlatformRead::Text("6.8.0-generic"),
            TestPlatformRead::Failure("os-release must not be read"),
        );
        let boundary = observe_linux_platform_boundary(
            ReleaseTargetTriple::Aarch64UnknownLinuxGnu,
            &mut observation,
        )
        .expect("a non-Microsoft kernel should be native Linux");
        assert_eq!(boundary.environment, PlatformEnvironment::Linux);
        assert_eq!(
            boundary.target_triple,
            ReleaseTargetTriple::Aarch64UnknownLinuxGnu
        );
        assert_eq!(observation.kernel_release_reads, 1);
        assert_eq!(observation.os_release_reads, 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wsl2_os_release_read_failure_reports_distribution_unavailable() {
        let mut observation = TestLinuxPlatformObservation::new(
            TestPlatformRead::Text("6.6.87.2-microsoft-standard-WSL2"),
            TestPlatformRead::Failure("injected os-release read failure"),
        );
        let error = observe_linux_platform_boundary(
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            &mut observation,
        )
        .expect_err("an unavailable WSL2 distribution observation must fail closed");
        assert_eq!(error.class(), PlatformDiagnosticClass::Unavailable);
        assert_eq!(
            error.kind(),
            PlatformDiagnosticKind::Wsl2DistributionIdentityUnavailable
        );
        assert_eq!(
            error.code(),
            "platform.wsl2.distribution_identity_unavailable"
        );
        assert_eq!(
            error.detail(),
            "the WSL2 distribution identity could not be read from /etc/os-release: injected os-release read failure"
        );
        assert_eq!(observation.kernel_release_reads, 1);
        assert_eq!(observation.os_release_reads, 1);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wsl2_malformed_os_release_reports_distribution_unavailable() {
        let mut observation = TestLinuxPlatformObservation::new(
            TestPlatformRead::Text("6.6.87.2-microsoft-standard-WSL2"),
            TestPlatformRead::Text("ID=ubuntu\nmalformed\nVERSION_ID=24.04\n"),
        );
        let error = observe_linux_platform_boundary(
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            &mut observation,
        )
        .expect_err("a malformed WSL2 distribution observation must fail closed");
        assert_eq!(error.class(), PlatformDiagnosticClass::Unavailable);
        assert_eq!(
            error.kind(),
            PlatformDiagnosticKind::Wsl2DistributionIdentityUnavailable
        );
        assert_eq!(
            error.code(),
            "platform.wsl2.distribution_identity_unavailable"
        );
        assert_eq!(error.detail(), "/etc/os-release contains a malformed entry");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn microsoft_kernel_without_wsl2_boundary_is_rejected_as_wsl1() {
        let mut observation = TestLinuxPlatformObservation::new(
            TestPlatformRead::Text("4.4.0-19041-Microsoft"),
            TestPlatformRead::Failure("os-release must not be read"),
        );
        let error = observe_linux_platform_boundary(
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            &mut observation,
        )
        .expect_err("WSL1 must be unsupported");
        assert_eq!(error.class(), PlatformDiagnosticClass::Unsupported);
        assert_eq!(error.kind(), PlatformDiagnosticKind::Wsl1);
        assert_eq!(error.code(), "platform.wsl1.unsupported");
        assert_eq!(
            error.detail(),
            "the observed Microsoft Linux kernel is not a WSL2 kernel"
        );
        assert_eq!(observation.kernel_release_reads, 1);
        assert_eq!(observation.os_release_reads, 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wsl2_observation_rejects_unsupported_distribution_identity() {
        let mut observation = TestLinuxPlatformObservation::new(
            TestPlatformRead::Text("6.6.87.2-microsoft-standard-WSL2"),
            TestPlatformRead::Text("ID=debian\nVERSION_ID=24.04\n"),
        );
        let error = observe_linux_platform_boundary(
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            &mut observation,
        )
        .expect_err("an unsupported distribution ID must fail closed");
        assert_eq!(error.class(), PlatformDiagnosticClass::Unsupported);
        assert_eq!(
            error.kind(),
            PlatformDiagnosticKind::UnsupportedWsl2Distribution
        );
        assert_eq!(error.code(), "platform.wsl2.distribution_unsupported");
        assert_eq!(error.detail(), "expected ID=ubuntu and VERSION_ID=24.04");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wsl2_rejects_unsupported_distribution_version() {
        let error = classify_linux_platform_boundary(
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            LinuxPlatformFacts {
                kernel_release: "6.6.87.2-microsoft-standard-WSL2",
                os_release: Some("ID=ubuntu\nVERSION_ID=24.10\n"),
            },
        )
        .expect_err("an unsupported distribution version must fail closed");
        assert_eq!(error.class(), PlatformDiagnosticClass::Unsupported);
        assert_eq!(
            error.kind(),
            PlatformDiagnosticKind::UnsupportedWsl2Distribution
        );
        assert_eq!(error.code(), "platform.wsl2.distribution_unsupported");
        assert_eq!(error.detail(), "expected ID=ubuntu and VERSION_ID=24.04");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wsl2_missing_os_release_fields_report_distribution_unavailable() {
        for (os_release, detail) in [
            (
                None,
                "/etc/os-release was not observed inside the WSL2 process",
            ),
            (Some("VERSION_ID=24.04\n"), "/etc/os-release is missing ID"),
            (Some("ID=ubuntu\n"), "/etc/os-release is missing VERSION_ID"),
        ] {
            let error = classify_linux_platform_boundary(
                ReleaseTargetTriple::X86_64UnknownLinuxGnu,
                LinuxPlatformFacts {
                    kernel_release: "6.6.87.2-microsoft-standard-WSL2",
                    os_release,
                },
            )
            .expect_err("missing os-release data must fail closed");
            assert_eq!(error.class(), PlatformDiagnosticClass::Unavailable);
            assert_eq!(
                error.kind(),
                PlatformDiagnosticKind::Wsl2DistributionIdentityUnavailable
            );
            assert_eq!(
                error.code(),
                "platform.wsl2.distribution_identity_unavailable"
            );
            assert_eq!(error.detail(), detail);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn supported_ubuntu_24_04_wsl2_observation_is_accepted() {
        let mut observation = TestLinuxPlatformObservation::new(
            TestPlatformRead::Text("6.6.87.2-microsoft-standard-WSL2"),
            TestPlatformRead::Text("ID=ubuntu\nVERSION_ID=\"24.04\"\n"),
        );
        let boundary = observe_linux_platform_boundary(
            ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            &mut observation,
        )
        .expect("the supported WSL2 distribution must be accepted");
        assert_eq!(boundary.environment, PlatformEnvironment::Wsl2);
        assert_eq!(
            boundary.target_triple,
            ReleaseTargetTriple::X86_64UnknownLinuxGnu
        );
        assert_eq!(observation.kernel_release_reads, 1);
        assert_eq!(observation.os_release_reads, 1);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wsl2_incompatible_target_uses_canonical_code() {
        let error = classify_linux_platform_boundary(
            ReleaseTargetTriple::Aarch64UnknownLinuxGnu,
            LinuxPlatformFacts {
                kernel_release: "6.6.87.2-microsoft-standard-WSL2",
                os_release: Some("ID=ubuntu\nVERSION_ID=24.04\n"),
            },
        )
        .expect_err("Linux AArch64 must not satisfy the x86-64 WSL2 environment");
        assert_eq!(error.class(), PlatformDiagnosticClass::Unsupported);
        assert_eq!(error.kind(), PlatformDiagnosticKind::UnsupportedTarget);
        assert_eq!(error.code(), "platform.target.unsupported");
        assert_eq!(
            error.detail(),
            "target aarch64-unknown-linux-gnu cannot run in the supported WSL2 environment"
        );
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
    fn directory_removal_reports_parent_sync_as_not_applicable() {
        let directory = TestDirectory::new("directory-removal-durability");
        let target = directory.join("owned");
        fs::create_dir(&target).expect("owned directory should be created");

        let outcome =
            remove_owned_directory_tree(&target).expect("owned directory should be removed");

        assert_eq!(outcome.effect, DirectoryTreeRemovalEffect::Removed);
        assert_eq!(outcome.durability, DirectoryEntryDurability::NotApplicable);
        assert_eq!(outcome.target_state, DirectoryTreeTargetState::Absent);
        assert!(!target.exists());
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
