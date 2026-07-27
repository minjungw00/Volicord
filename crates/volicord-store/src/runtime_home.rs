use std::{
    error::Error,
    ffi::OsString,
    fmt, fs, io,
    path::{Component, Path, PathBuf},
};

use volicord_platform_fs::{
    observe_local_platform_boundary, observe_path_filesystem, LocalPlatformBoundary,
    PathFilesystemKind, PlatformBoundaryError, PlatformDiagnostic, PlatformDiagnosticClass,
    PlatformDiagnosticKind,
};
use volicord_types::platform::PlatformEnvironment;

use crate::CanonicalRuntimeHomePath;

#[cfg(windows)]
use std::path::{Prefix, PrefixComponent};

const VOLICORD_HOME: &str = "VOLICORD_HOME";
const HOME: &str = "HOME";
const USERPROFILE: &str = "USERPROFILE";
const HOMEDRIVE: &str = "HOMEDRIVE";
const HOMEPATH: &str = "HOMEPATH";

/// Errors returned while selecting a Runtime Home path from process inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomeResolutionError {
    EmptyVolicordHome,
    RelativeVolicordHome,
    MissingUserHome,
}

impl fmt::Display for RuntimeHomeResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyVolicordHome => formatter.write_str("VOLICORD_HOME must not be empty"),
            Self::RelativeVolicordHome => {
                formatter.write_str("VOLICORD_HOME must be an absolute path")
            }
            Self::MissingUserHome => formatter
                .write_str("could not determine a default home directory; set VOLICORD_HOME"),
        }
    }
}

impl Error for RuntimeHomeResolutionError {}

/// Component-aware relation between `Volicord Runtime Home` and
/// `Product Repository` filesystem roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProductPathRelation {
    SamePath,
    RuntimeHomeContainsProductRepository,
    ProductRepositoryContainsRuntimeHome,
    Separate,
}

/// Invalid filesystem-boundary condition detected during path validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePathBoundaryViolation {
    SamePath,
    RuntimeHomeContainsProductRepository,
    ProductRepositoryContainsRuntimeHome,
    ProjectHomeOutsideRuntimeHome,
    ProjectHomeOverlapsProductRepository,
}

impl RuntimePathBoundaryViolation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SamePath => "same_path",
            Self::RuntimeHomeContainsProductRepository => {
                "runtime_home_contains_product_repository"
            }
            Self::ProductRepositoryContainsRuntimeHome => {
                "product_repository_contains_runtime_home"
            }
            Self::ProjectHomeOutsideRuntimeHome => "project_home_outside_runtime_home",
            Self::ProjectHomeOverlapsProductRepository => {
                "project_home_overlaps_product_repository"
            }
        }
    }
}

/// Normalized Runtime Home and Product Repository paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProductPathValidation {
    pub runtime_home: PathBuf,
    pub repo_root: PathBuf,
    pub relation: RuntimeProductPathRelation,
}

/// Errors returned while validating Runtime Home and Product Repository
/// filesystem boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimePathBoundaryError {
    InvalidPath {
        role: &'static str,
        path: PathBuf,
        detail: String,
    },
    BoundaryViolation {
        violation: RuntimePathBoundaryViolation,
        runtime_home: PathBuf,
        repo_root: PathBuf,
        project_home: Option<PathBuf>,
    },
    UnsupportedEnvironment {
        diagnostic: PlatformDiagnostic,
    },
    PlatformUnavailable {
        diagnostic: PlatformDiagnostic,
    },
}

impl RuntimePathBoundaryError {
    pub fn violation(&self) -> Option<RuntimePathBoundaryViolation> {
        match self {
            Self::BoundaryViolation { violation, .. } => Some(*violation),
            Self::InvalidPath { .. }
            | Self::UnsupportedEnvironment { .. }
            | Self::PlatformUnavailable { .. } => None,
        }
    }

    /// Returns the typed platform diagnostic without parsing display detail.
    pub const fn platform_diagnostic(&self) -> Option<&PlatformDiagnostic> {
        match self {
            Self::UnsupportedEnvironment { diagnostic, .. }
            | Self::PlatformUnavailable { diagnostic, .. } => Some(diagnostic),
            Self::InvalidPath { .. } | Self::BoundaryViolation { .. } => None,
        }
    }
}

impl fmt::Display for RuntimePathBoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath { role, path, detail } => {
                write!(formatter, "{role} path {} is invalid: {detail}", path.display())
            }
            Self::BoundaryViolation {
                violation,
                runtime_home,
                repo_root,
                project_home,
            } => match violation {
                RuntimePathBoundaryViolation::SamePath => write!(
                    formatter,
                    "Volicord Runtime Home and Product Repository must not be the same path: runtime_home {}, repo_root {}",
                    runtime_home.display(),
                    repo_root.display()
                ),
                RuntimePathBoundaryViolation::RuntimeHomeContainsProductRepository => write!(
                    formatter,
                    "Product Repository must not be inside Volicord Runtime Home: runtime_home {}, repo_root {}",
                    runtime_home.display(),
                    repo_root.display()
                ),
                RuntimePathBoundaryViolation::ProductRepositoryContainsRuntimeHome => write!(
                    formatter,
                    "Volicord Runtime Home must not be inside Product Repository: runtime_home {}, repo_root {}",
                    runtime_home.display(),
                    repo_root.display()
                ),
                RuntimePathBoundaryViolation::ProjectHomeOutsideRuntimeHome => {
                    let project_home = project_home
                        .as_ref()
                        .expect("project-home violation carries project_home");
                    write!(
                        formatter,
                        "project_home must be inside Volicord Runtime Home: runtime_home {}, project_home {}",
                        runtime_home.display(),
                        project_home.display()
                    )
                }
                RuntimePathBoundaryViolation::ProjectHomeOverlapsProductRepository => {
                    let project_home = project_home
                        .as_ref()
                        .expect("project-home violation carries project_home");
                    write!(
                        formatter,
                        "project_home must not overlap Product Repository: repo_root {}, project_home {}",
                        repo_root.display(),
                        project_home.display()
                    )
                }
            },
            Self::UnsupportedEnvironment { diagnostic }
            | Self::PlatformUnavailable { diagnostic } => diagnostic.fmt(formatter),
        }
    }
}

impl Error for RuntimePathBoundaryError {}

/// Injected platform and filesystem facts for one Runtime Home/Product Repository pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProductPlatformFacts {
    pub boundary: LocalPlatformBoundary,
    pub runtime_home_filesystem: PathFilesystemKind,
    pub product_repository_filesystem: PathFilesystemKind,
}

/// Validates the operating-environment topology independently of path probing.
pub fn validate_runtime_product_platform_facts(
    facts: &RuntimeProductPlatformFacts,
) -> Result<(), RuntimePathBoundaryError> {
    if facts.boundary.environment != PlatformEnvironment::Wsl2 {
        return Ok(());
    }
    for (role, filesystem) in [
        ("runtime_home", facts.runtime_home_filesystem),
        ("product_repository", facts.product_repository_filesystem),
    ] {
        if filesystem != PathFilesystemKind::LinuxExt4 {
            return Err(RuntimePathBoundaryError::UnsupportedEnvironment {
                diagnostic: PlatformDiagnostic::new(
                    PlatformDiagnosticKind::UnsupportedFilesystemBoundary,
                    format!("{role} must be on the pinned WSL2 distribution ext4 filesystem"),
                ),
            });
        }
    }
    Ok(())
}

/// Resolves the Volicord Runtime Home path from environment values and a cwd.
///
/// This function performs path selection only. It does not canonicalize the
/// result, create directories, or require the selected path to exist.
pub fn resolve_runtime_home<F>(
    env_var: F,
    current_dir: impl AsRef<Path>,
) -> Result<PathBuf, RuntimeHomeResolutionError>
where
    F: Fn(&str) -> Option<OsString>,
{
    let current_dir = current_dir.as_ref();
    if let Some(value) = env_var(VOLICORD_HOME) {
        if value.is_empty() {
            return Err(RuntimeHomeResolutionError::EmptyVolicordHome);
        }
        let selected = PathBuf::from(value);
        if !selected.is_absolute() {
            return Err(RuntimeHomeResolutionError::RelativeVolicordHome);
        }
        return Ok(selected);
    }

    let home = default_user_home(env_var).ok_or(RuntimeHomeResolutionError::MissingUserHome)?;
    Ok(absolute_path(current_dir, home).join(".volicord"))
}

/// Validates and normalizes the filesystem relationship between Runtime Home
/// and Product Repository.
///
/// The Product Repository must already exist and be a directory. The Runtime
/// Home may be missing; this function canonicalizes its nearest existing
/// ancestor and appends missing path components lexically without creating
/// filesystem state.
pub fn validate_runtime_home_product_repository(
    runtime_home: impl AsRef<Path>,
    repo_root: impl AsRef<Path>,
) -> Result<RuntimeProductPathValidation, RuntimePathBoundaryError> {
    let runtime_home = normalize_maybe_missing_directory("runtime_home", runtime_home.as_ref())?;
    let repo_root = normalize_existing_directory("repo_root", repo_root.as_ref())?;
    validate_runtime_product_platform_paths(&runtime_home, &repo_root)?;
    let relation = runtime_product_path_relation(&runtime_home, &repo_root);
    match relation {
        RuntimeProductPathRelation::Separate => Ok(RuntimeProductPathValidation {
            runtime_home,
            repo_root,
            relation,
        }),
        RuntimeProductPathRelation::SamePath => Err(runtime_product_violation(
            RuntimePathBoundaryViolation::SamePath,
            runtime_home,
            repo_root,
        )),
        RuntimeProductPathRelation::RuntimeHomeContainsProductRepository => {
            Err(runtime_product_violation(
                RuntimePathBoundaryViolation::RuntimeHomeContainsProductRepository,
                runtime_home,
                repo_root,
            ))
        }
        RuntimeProductPathRelation::ProductRepositoryContainsRuntimeHome => {
            Err(runtime_product_violation(
                RuntimePathBoundaryViolation::ProductRepositoryContainsRuntimeHome,
                runtime_home,
                repo_root,
            ))
        }
    }
}

/// Validates a Product Repository against an already-admitted Runtime Home identity.
///
/// The Runtime Home is not selected or canonicalized again. The Product
/// Repository remains subject to the ordinary existing-directory and platform
/// boundary checks.
pub fn validate_runtime_home_product_repository_admitted(
    runtime_home: &CanonicalRuntimeHomePath,
    repo_root: impl AsRef<Path>,
) -> Result<RuntimeProductPathValidation, RuntimePathBoundaryError> {
    let runtime_home = runtime_home.as_path().to_path_buf();
    let repo_root = normalize_existing_directory("repo_root", repo_root.as_ref())?;
    validate_runtime_product_platform_paths(&runtime_home, &repo_root)?;
    let relation = runtime_product_path_relation(&runtime_home, &repo_root);
    match relation {
        RuntimeProductPathRelation::Separate => Ok(RuntimeProductPathValidation {
            runtime_home,
            repo_root,
            relation,
        }),
        RuntimeProductPathRelation::SamePath => Err(runtime_product_violation(
            RuntimePathBoundaryViolation::SamePath,
            runtime_home,
            repo_root,
        )),
        RuntimeProductPathRelation::RuntimeHomeContainsProductRepository => {
            Err(runtime_product_violation(
                RuntimePathBoundaryViolation::RuntimeHomeContainsProductRepository,
                runtime_home,
                repo_root,
            ))
        }
        RuntimeProductPathRelation::ProductRepositoryContainsRuntimeHome => {
            Err(runtime_product_violation(
                RuntimePathBoundaryViolation::ProductRepositoryContainsRuntimeHome,
                runtime_home,
                repo_root,
            ))
        }
    }
}

/// Classifies a normalized Runtime Home and Product Repository pair.
pub fn runtime_product_path_relation(
    runtime_home: &Path,
    repo_root: &Path,
) -> RuntimeProductPathRelation {
    if paths_equal_for_boundary(runtime_home, repo_root) {
        RuntimeProductPathRelation::SamePath
    } else if path_starts_with_for_boundary(repo_root, runtime_home) {
        RuntimeProductPathRelation::RuntimeHomeContainsProductRepository
    } else if path_starts_with_for_boundary(runtime_home, repo_root) {
        RuntimeProductPathRelation::ProductRepositoryContainsRuntimeHome
    } else {
        RuntimeProductPathRelation::Separate
    }
}

/// Validates a project-home path using the same Runtime Home/Product
/// Repository boundary inputs.
pub fn validate_project_home_boundary(
    runtime_home: impl AsRef<Path>,
    repo_root: impl AsRef<Path>,
    project_home: impl AsRef<Path>,
) -> Result<PathBuf, RuntimePathBoundaryError> {
    let RuntimeProductPathValidation {
        runtime_home,
        repo_root,
        ..
    } = validate_runtime_home_product_repository(runtime_home, repo_root)?;
    let project_home = normalize_maybe_missing_directory("project_home", project_home.as_ref())?;
    validate_platform_path(&project_home, "project_home")?;

    if paths_overlap(&project_home, &repo_root) {
        return Err(RuntimePathBoundaryError::BoundaryViolation {
            violation: RuntimePathBoundaryViolation::ProjectHomeOverlapsProductRepository,
            runtime_home,
            repo_root,
            project_home: Some(project_home),
        });
    }

    if !path_starts_with_for_boundary(&project_home, &runtime_home) {
        return Err(RuntimePathBoundaryError::BoundaryViolation {
            violation: RuntimePathBoundaryViolation::ProjectHomeOutsideRuntimeHome,
            runtime_home,
            repo_root,
            project_home: Some(project_home),
        });
    }

    Ok(project_home)
}

/// Validates a project-home path against an admitted Runtime Home identity.
pub fn validate_project_home_boundary_admitted(
    runtime_home: &CanonicalRuntimeHomePath,
    repo_root: impl AsRef<Path>,
    project_home: impl AsRef<Path>,
) -> Result<PathBuf, RuntimePathBoundaryError> {
    let RuntimeProductPathValidation {
        runtime_home,
        repo_root,
        ..
    } = validate_runtime_home_product_repository_admitted(runtime_home, repo_root)?;
    let project_home = normalize_maybe_missing_directory("project_home", project_home.as_ref())?;
    validate_platform_path(&project_home, "project_home")?;

    if paths_overlap(&project_home, &repo_root) {
        return Err(RuntimePathBoundaryError::BoundaryViolation {
            violation: RuntimePathBoundaryViolation::ProjectHomeOverlapsProductRepository,
            runtime_home,
            repo_root,
            project_home: Some(project_home),
        });
    }

    if !path_starts_with_for_boundary(&project_home, &runtime_home) {
        return Err(RuntimePathBoundaryError::BoundaryViolation {
            violation: RuntimePathBoundaryViolation::ProjectHomeOutsideRuntimeHome,
            runtime_home,
            repo_root,
            project_home: Some(project_home),
        });
    }

    Ok(project_home)
}

fn validate_runtime_product_platform_paths(
    runtime_home: &Path,
    repo_root: &Path,
) -> Result<(), RuntimePathBoundaryError> {
    let boundary = observe_local_platform_boundary().map_err(runtime_platform_error)?;
    let (runtime_home_filesystem, product_repository_filesystem) =
        if boundary.environment == PlatformEnvironment::Wsl2 {
            (
                observe_path_filesystem(runtime_home).map_err(runtime_platform_error)?,
                observe_path_filesystem(repo_root).map_err(runtime_platform_error)?,
            )
        } else {
            (PathFilesystemKind::Other, PathFilesystemKind::Other)
        };
    validate_runtime_product_platform_facts(&RuntimeProductPlatformFacts {
        boundary,
        runtime_home_filesystem,
        product_repository_filesystem,
    })
}

fn validate_platform_path(path: &Path, role: &'static str) -> Result<(), RuntimePathBoundaryError> {
    let boundary = observe_local_platform_boundary().map_err(runtime_platform_error)?;
    if boundary.environment != PlatformEnvironment::Wsl2 {
        return Ok(());
    }
    let filesystem = observe_path_filesystem(path).map_err(runtime_platform_error)?;
    if filesystem == PathFilesystemKind::LinuxExt4 {
        Ok(())
    } else {
        Err(RuntimePathBoundaryError::UnsupportedEnvironment {
            diagnostic: PlatformDiagnostic::new(
                PlatformDiagnosticKind::UnsupportedFilesystemBoundary,
                format!("{role} must be on the pinned WSL2 distribution ext4 filesystem"),
            ),
        })
    }
}

fn runtime_platform_error(error: PlatformBoundaryError) -> RuntimePathBoundaryError {
    let diagnostic = error.into_diagnostic();
    match diagnostic.class() {
        PlatformDiagnosticClass::Rejected => {
            RuntimePathBoundaryError::PlatformUnavailable { diagnostic }
        }
        PlatformDiagnosticClass::Unsupported => {
            RuntimePathBoundaryError::UnsupportedEnvironment { diagnostic }
        }
        PlatformDiagnosticClass::Unavailable => {
            RuntimePathBoundaryError::PlatformUnavailable { diagnostic }
        }
    }
}

pub(crate) fn normalize_lexical_path(
    role: &'static str,
    path: &Path,
) -> Result<PathBuf, RuntimePathBoundaryError> {
    make_absolute_without_parent_traversal(role, path)
}

#[cfg(not(windows))]
fn default_user_home<F>(env_var: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    non_empty_env(&env_var, HOME)
        .map(PathBuf::from)
        .or_else(|| non_empty_env(&env_var, USERPROFILE).map(PathBuf::from))
        .or_else(|| {
            let drive = non_empty_env(&env_var, HOMEDRIVE)?;
            let path = non_empty_env(&env_var, HOMEPATH)?;
            let mut home = PathBuf::from(drive);
            home.push(path);
            Some(home)
        })
}

#[cfg(windows)]
fn default_user_home<F>(env_var: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    non_empty_env(&env_var, USERPROFILE)
        .map(PathBuf::from)
        .or_else(|| {
            let drive = non_empty_env(&env_var, HOMEDRIVE)?;
            let path = non_empty_env(&env_var, HOMEPATH)?;
            let mut home = PathBuf::from(drive);
            home.push(path);
            Some(home)
        })
        .or_else(|| {
            let home = non_empty_env(&env_var, HOME).map(PathBuf::from)?;
            if looks_like_wsl_mount_path(&home) {
                None
            } else {
                Some(home)
            }
        })
}

fn non_empty_env<F>(env_var: &F, name: &str) -> Option<OsString>
where
    F: Fn(&str) -> Option<OsString>,
{
    env_var(name).filter(|value| !value.is_empty())
}

fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

fn normalize_existing_directory(
    role: &'static str,
    path: &Path,
) -> Result<PathBuf, RuntimePathBoundaryError> {
    let current_dir;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_dir = std::env::current_dir()
            .map_err(|error| invalid_path(role, path, format!("failed to read cwd: {error}")))?;
        current_dir.join(path)
    };
    validate_supported_platform_path(role, path, &absolute)?;
    let canonical = fs::canonicalize(&absolute).map_err(|error| {
        invalid_path(
            role,
            path,
            format!("directory does not exist or is not accessible: {error}"),
        )
    })?;
    validate_supported_platform_path(role, path, &canonical)?;
    match fs::metadata(&canonical) {
        Ok(metadata) if metadata.is_dir() => Ok(canonical),
        Ok(_) => Err(invalid_path(
            role,
            path,
            format!("existing path is not a directory: {}", canonical.display()),
        )),
        Err(error) => Err(invalid_path(
            role,
            path,
            format!("failed to inspect {}: {error}", canonical.display()),
        )),
    }
}

fn normalize_maybe_missing_directory(
    role: &'static str,
    path: &Path,
) -> Result<PathBuf, RuntimePathBoundaryError> {
    let absolute = make_absolute_without_parent_traversal(role, path)?;
    let (ancestor, mut unresolved) = nearest_existing_directory_ancestor(role, path, &absolute)?;
    let mut normalized = fs::canonicalize(&ancestor).map_err(|error| {
        invalid_path(
            role,
            path,
            format!(
                "failed to canonicalize existing ancestor {}: {error}",
                ancestor.display()
            ),
        )
    })?;
    validate_supported_platform_path(role, path, &normalized)?;
    unresolved.reverse();
    for component in unresolved {
        normalized.push(component);
    }
    Ok(normalized)
}

fn make_absolute_without_parent_traversal(
    role: &'static str,
    path: &Path,
) -> Result<PathBuf, RuntimePathBoundaryError> {
    let current_dir;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_dir = std::env::current_dir()
            .map_err(|error| invalid_path(role, path, format!("failed to read cwd: {error}")))?;
        current_dir.join(path)
    };
    let normalized = normalize_lexical_components(role, path, &absolute)?;
    validate_supported_platform_path(role, path, &normalized)?;
    Ok(normalized)
}

fn normalize_lexical_components(
    role: &'static str,
    original: &Path,
    absolute: &Path,
) -> Result<PathBuf, RuntimePathBoundaryError> {
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(invalid_path(
                    role,
                    original,
                    "parent traversal is not valid for this path role",
                ));
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        Err(invalid_path(role, original, "path must not be empty"))
    } else {
        Ok(normalized)
    }
}

fn nearest_existing_directory_ancestor(
    role: &'static str,
    original: &Path,
    absolute: &Path,
) -> Result<(PathBuf, Vec<OsString>), RuntimePathBoundaryError> {
    let mut candidate = absolute.to_path_buf();
    let mut unresolved = Vec::new();

    loop {
        match fs::metadata(&candidate) {
            Ok(metadata) if metadata.is_dir() => return Ok((candidate, unresolved)),
            Ok(_) => {
                return Err(invalid_path(
                    role,
                    original,
                    format!("existing path is not a directory: {}", candidate.display()),
                ));
            }
            Err(error) if missing_path_error(&error) => {
                let Some(name) = candidate.file_name().map(OsString::from) else {
                    return Err(invalid_path(
                        role,
                        original,
                        "path has no existing directory ancestor",
                    ));
                };
                unresolved.push(name);
                let Some(parent) = candidate.parent() else {
                    return Err(invalid_path(
                        role,
                        original,
                        "path has no existing directory ancestor",
                    ));
                };
                candidate = parent.to_path_buf();
            }
            Err(error) => {
                return Err(invalid_path(
                    role,
                    original,
                    format!("failed to inspect {}: {error}", candidate.display()),
                ));
            }
        }
    }
}

fn missing_path_error(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    )
}

#[cfg(not(windows))]
fn validate_supported_platform_path(
    _role: &'static str,
    _original: &Path,
    _path: &Path,
) -> Result<(), RuntimePathBoundaryError> {
    Ok(())
}

#[cfg(windows)]
fn validate_supported_platform_path(
    role: &'static str,
    original: &Path,
    path: &Path,
) -> Result<(), RuntimePathBoundaryError> {
    if looks_like_wsl_mount_path(original) {
        return Err(invalid_path(
            role,
            original,
            "WSL-style /mnt/<drive> paths are ambiguous in native Windows; run Volicord inside WSL2 or use a native drive-letter path such as C:\\Users\\you\\repo",
        ));
    }

    for component in path.components() {
        let Component::Prefix(prefix) = component else {
            continue;
        };
        return match prefix.kind() {
            Prefix::Disk(_) | Prefix::VerbatimDisk(_) => Ok(()),
            Prefix::UNC(server, _) | Prefix::VerbatimUNC(server, _)
                if windows_os_str_eq_ignore_ascii_case(server, "wsl$")
                    || windows_os_str_eq_ignore_ascii_case(server, "wsl.localhost") =>
            {
                Err(invalid_path(
                    role,
                    original,
                    "WSL UNC paths are not valid native Windows Runtime Home or Product Repository paths; run Volicord inside WSL2 or use a native drive-letter path",
                ))
            }
            Prefix::UNC(_, _) | Prefix::VerbatimUNC(_, _) => Err(invalid_path(
                role,
                original,
                "UNC paths are not supported for Runtime Home or Product Repository boundaries; use a local drive-letter path",
            )),
            _ => Err(invalid_path(
                role,
                original,
                "unsupported Windows path prefix; use a local drive-letter path",
            )),
        };
    }

    Err(invalid_path(
        role,
        original,
        "native Windows paths must resolve to a local drive-letter path; root-relative and WSL-style paths are not supported",
    ))
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    paths_equal_for_boundary(left, right)
        || path_starts_with_for_boundary(left, right)
        || path_starts_with_for_boundary(right, left)
}

pub(crate) fn paths_equal_for_boundary(left: &Path, right: &Path) -> bool {
    path_starts_with_for_boundary(left, right) && path_starts_with_for_boundary(right, left)
}

pub(crate) fn path_starts_with_for_boundary(path: &Path, base: &Path) -> bool {
    #[cfg(not(windows))]
    {
        path.starts_with(base)
    }
    #[cfg(windows)]
    {
        let mut components = path.components();
        for base_component in base.components() {
            let Some(component) = components.next() else {
                return false;
            };
            if !windows_components_equal(component, base_component) {
                return false;
            }
        }
        true
    }
}

#[cfg(windows)]
fn windows_components_equal(left: Component<'_>, right: Component<'_>) -> bool {
    match (left, right) {
        (Component::Prefix(left), Component::Prefix(right)) => windows_prefixes_equal(left, right),
        (Component::RootDir, Component::RootDir) => true,
        (Component::CurDir, Component::CurDir) => true,
        (Component::ParentDir, Component::ParentDir) => true,
        (Component::Normal(left), Component::Normal(right)) => {
            windows_os_str_eq_ignore_ascii_case(left, right)
        }
        _ => false,
    }
}

#[cfg(windows)]
fn windows_prefixes_equal(left: PrefixComponent<'_>, right: PrefixComponent<'_>) -> bool {
    match (left.kind(), right.kind()) {
        (Prefix::Disk(left), Prefix::Disk(right))
        | (Prefix::Disk(left), Prefix::VerbatimDisk(right))
        | (Prefix::VerbatimDisk(left), Prefix::Disk(right))
        | (Prefix::VerbatimDisk(left), Prefix::VerbatimDisk(right)) => {
            left.eq_ignore_ascii_case(&right)
        }
        _ => windows_os_str_eq_ignore_ascii_case(left.as_os_str(), right.as_os_str()),
    }
}

#[cfg(windows)]
fn windows_os_str_eq_ignore_ascii_case(
    left: &std::ffi::OsStr,
    right: impl AsRef<std::ffi::OsStr>,
) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.as_ref().to_string_lossy())
}

#[cfg(windows)]
fn looks_like_wsl_mount_path(path: &Path) -> bool {
    let text = path.as_os_str().to_string_lossy().replace('\\', "/");
    let trimmed = text.trim_start_matches('/');
    let mut parts = trimmed.split('/');
    matches!(parts.next(), Some("mnt"))
        && parts
            .next()
            .is_some_and(|drive| drive.len() == 1 && drive.as_bytes()[0].is_ascii_alphabetic())
}

fn runtime_product_violation(
    violation: RuntimePathBoundaryViolation,
    runtime_home: PathBuf,
    repo_root: PathBuf,
) -> RuntimePathBoundaryError {
    RuntimePathBoundaryError::BoundaryViolation {
        violation,
        runtime_home,
        repo_root,
        project_home: None,
    }
}

fn invalid_path(
    role: &'static str,
    path: &Path,
    detail: impl Into<String>,
) -> RuntimePathBoundaryError {
    RuntimePathBoundaryError::InvalidPath {
        role,
        path: path.to_path_buf(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, ffi::OsString, fs, path::PathBuf};

    use volicord_test_support::TempRuntimeHome;
    use volicord_types::platform::PlatformEnvironment;
    use volicord_types::release_target::ReleaseTargetTriple;

    use super::{
        resolve_runtime_home, validate_runtime_home_product_repository,
        validate_runtime_product_platform_facts, LocalPlatformBoundary, PathFilesystemKind,
        PlatformDiagnosticKind, RuntimeHomeResolutionError, RuntimePathBoundaryViolation,
        RuntimeProductPathRelation, RuntimeProductPlatformFacts,
    };

    fn cwd() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn resolve(entries: &[(&str, OsString)]) -> Result<PathBuf, RuntimeHomeResolutionError> {
        resolve_runtime_home(
            |name| {
                entries
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| value.clone())
            },
            cwd(),
        )
    }

    #[test]
    fn injected_wsl2_path_facts_require_ext4_for_both_product_roots() {
        let supported = RuntimeProductPlatformFacts {
            boundary: LocalPlatformBoundary {
                target_triple: ReleaseTargetTriple::X86_64UnknownLinuxGnu,
                environment: PlatformEnvironment::Wsl2,
            },
            runtime_home_filesystem: PathFilesystemKind::LinuxExt4,
            product_repository_filesystem: PathFilesystemKind::LinuxExt4,
        };
        validate_runtime_product_platform_facts(&supported)
            .expect("one-distribution WSL2 ext4 topology should be valid");

        for mutate in [
            |facts: &mut RuntimeProductPlatformFacts| {
                facts.runtime_home_filesystem = PathFilesystemKind::Other;
            },
            |facts: &mut RuntimeProductPlatformFacts| {
                facts.product_repository_filesystem = PathFilesystemKind::Other;
            },
        ] {
            let mut facts = supported.clone();
            mutate(&mut facts);
            let error = validate_runtime_product_platform_facts(&facts)
                .expect_err("a WSL2 cross-filesystem topology must fail closed");
            let diagnostic = error
                .platform_diagnostic()
                .expect("the topology failure must retain its typed platform diagnostic");
            assert_eq!(
                diagnostic.kind(),
                PlatformDiagnosticKind::UnsupportedFilesystemBoundary
            );
            assert_eq!(diagnostic.code(), "platform.filesystem.unsupported");
            assert_eq!(
                error.to_string(),
                format!("{}: {}", diagnostic.code(), diagnostic.detail())
            );
        }
    }

    #[test]
    fn injected_native_platform_facts_keep_native_filesystem_behavior() {
        for (environment, target_triple) in [
            (
                PlatformEnvironment::Linux,
                ReleaseTargetTriple::X86_64UnknownLinuxGnu,
            ),
            (
                PlatformEnvironment::Macos,
                ReleaseTargetTriple::Aarch64AppleDarwin,
            ),
            (
                PlatformEnvironment::NativeWindows,
                ReleaseTargetTriple::X86_64PcWindowsMsvc,
            ),
        ] {
            validate_runtime_product_platform_facts(&RuntimeProductPlatformFacts {
                boundary: LocalPlatformBoundary {
                    target_triple,
                    environment,
                },
                runtime_home_filesystem: PathFilesystemKind::Other,
                product_repository_filesystem: PathFilesystemKind::Other,
            })
            .expect("native platform path policy should remain native");
        }
    }

    #[test]
    fn absolute_volicord_home_is_used_as_supplied() {
        let path = cwd().join("runtime-home-absolute");

        let resolved = resolve(&[("VOLICORD_HOME", path.clone().into_os_string())])
            .expect("absolute VOLICORD_HOME should resolve");

        assert_eq!(resolved, path);
    }

    #[test]
    fn relative_volicord_home_is_rejected() {
        let error = resolve(&[("VOLICORD_HOME", OsString::from("runtime-home-relative"))])
            .expect_err("relative VOLICORD_HOME should fail closed");

        assert_eq!(error, RuntimeHomeResolutionError::RelativeVolicordHome);
    }

    #[test]
    fn empty_volicord_home_is_an_error() {
        let error = resolve(&[("VOLICORD_HOME", OsString::new())])
            .expect_err("empty VOLICORD_HOME should fail");

        assert_eq!(error, RuntimeHomeResolutionError::EmptyVolicordHome);
        assert!(error.to_string().contains("VOLICORD_HOME"));
    }

    #[test]
    fn home_fallback_appends_volicord() {
        let home = cwd().join("home-fallback");

        let resolved =
            resolve(&[("HOME", home.clone().into_os_string())]).expect("HOME should resolve");

        assert_eq!(resolved, home.join(".volicord"));
    }

    #[test]
    fn userprofile_fallback_is_used_after_missing_home() {
        let home = cwd().join("userprofile-fallback");

        let resolved = resolve(&[("USERPROFILE", home.clone().into_os_string())])
            .expect("USERPROFILE should resolve");

        assert_eq!(resolved, home.join(".volicord"));
    }

    #[test]
    fn homedrive_and_homepath_fallback_are_combined() {
        let drive = cwd().join("drive-fallback");

        let resolved = resolve(&[
            ("HOMEDRIVE", drive.clone().into_os_string()),
            ("HOMEPATH", OsString::from("homepath")),
        ])
        .expect("HOMEDRIVE and HOMEPATH should resolve");

        assert_eq!(resolved, drive.join("homepath").join(".volicord"));
    }

    #[test]
    fn empty_fallback_values_are_skipped() {
        let userprofile = cwd().join("fallback-after-empty-home");

        let resolved = resolve(&[
            ("HOME", OsString::new()),
            ("USERPROFILE", userprofile.clone().into_os_string()),
            ("HOMEDRIVE", cwd().join("unused-drive").into_os_string()),
            ("HOMEPATH", OsString::from("unused-path")),
        ])
        .expect("non-empty USERPROFILE should resolve after empty HOME");

        assert_eq!(resolved, userprofile.join(".volicord"));
    }

    #[cfg(windows)]
    #[test]
    fn native_windows_home_sources_prefer_userprofile_over_posix_home() {
        let userprofile = PathBuf::from(r"C:\Users\volicord");

        let resolved = resolve(&[
            ("HOME", OsString::from(r"/mnt/c/Users/volicord")),
            ("USERPROFILE", userprofile.clone().into_os_string()),
        ])
        .expect("USERPROFILE should be preferred on native Windows");

        assert_eq!(resolved, userprofile.join(".volicord"));
    }

    #[test]
    fn relative_fallback_home_is_made_absolute() {
        let resolved = resolve(&[("HOME", OsString::from("relative-home"))])
            .expect("relative HOME should resolve");

        assert_eq!(resolved, cwd().join("relative-home").join(".volicord"));
        assert!(resolved.is_absolute());
    }

    #[test]
    fn no_available_home_source_is_an_error() {
        let error = resolve(&[]).expect_err("missing home sources should fail");

        assert_eq!(error, RuntimeHomeResolutionError::MissingUserHome);
        assert!(error.to_string().contains("set VOLICORD_HOME"));
    }

    #[test]
    fn selected_runtime_home_is_not_canonicalized_or_required_to_exist() {
        let selected = cwd().join("missing-runtime-home/../still-missing");
        let resolved = resolve(&[("VOLICORD_HOME", selected.clone().into_os_string())])
            .expect("nonexistent absolute VOLICORD_HOME should resolve");

        assert_eq!(resolved, selected);
    }

    #[test]
    fn runtime_product_validation_accepts_separate_sibling_paths() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-siblings")?;
        fs::create_dir_all(fixture.path())?;
        let repo_root = fixture.create_product_repo("repo")?;

        let validation = validate_runtime_home_product_repository(fixture.path(), &repo_root)?;

        assert_eq!(validation.relation, RuntimeProductPathRelation::Separate);
        assert_eq!(validation.runtime_home, fs::canonicalize(fixture.path())?);
        assert_eq!(validation.repo_root, fs::canonicalize(repo_root)?);
        Ok(())
    }

    #[test]
    fn runtime_product_validation_uses_components_not_text_prefix() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-text-prefix")?;
        let parent = fixture.path().parent().expect("runtime home has parent");
        let runtime_home = parent.join("repo");
        let repo_root = parent.join("repository");
        fs::create_dir_all(&repo_root)?;

        let validation = validate_runtime_home_product_repository(&runtime_home, &repo_root)?;

        assert_eq!(validation.relation, RuntimeProductPathRelation::Separate);
        assert!(validation.runtime_home.ends_with("repo"));
        assert!(validation.repo_root.ends_with("repository"));
        Ok(())
    }

    #[test]
    fn runtime_product_validation_normalizes_dot_components() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-dot")?;
        fs::create_dir_all(fixture.path())?;
        let repo_root = fixture.create_product_repo("repo")?;
        let runtime_with_dot = fixture.path().join(".");
        let repo_with_dot = repo_root.join(".");

        let validation =
            validate_runtime_home_product_repository(&runtime_with_dot, &repo_with_dot)?;

        assert_eq!(validation.relation, RuntimeProductPathRelation::Separate);
        assert_eq!(validation.runtime_home, fs::canonicalize(fixture.path())?);
        assert_eq!(validation.repo_root, fs::canonicalize(repo_root)?);
        Ok(())
    }

    #[test]
    fn runtime_product_validation_allows_missing_runtime_home_under_existing_ancestor(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-missing-runtime")?;
        let parent = fixture.path().parent().expect("runtime home has parent");
        let runtime_home = parent.join("missing").join("runtime-home");
        let repo_root = fixture.create_product_repo("repo")?;

        let validation = validate_runtime_home_product_repository(&runtime_home, &repo_root)?;

        assert_eq!(validation.relation, RuntimeProductPathRelation::Separate);
        assert!(validation.runtime_home.ends_with("missing/runtime-home"));
        assert!(!runtime_home.exists());
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn runtime_product_validation_accepts_native_drive_letter_paths() -> Result<(), Box<dyn Error>>
    {
        use std::path::Prefix;

        fn has_drive_prefix(path: &Path) -> bool {
            path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::Prefix(prefix)
                        if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
                )
            })
        }

        let fixture = TempRuntimeHome::new("boundary-windows-drive")?;
        let repo_root = fixture.create_product_repo("repo")?;

        let validation = validate_runtime_home_product_repository(fixture.path(), &repo_root)?;

        assert_eq!(validation.relation, RuntimeProductPathRelation::Separate);
        assert!(has_drive_prefix(&validation.runtime_home));
        assert!(has_drive_prefix(&validation.repo_root));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn runtime_product_relation_is_case_insensitive_for_native_windows_drive_paths() {
        let repo_root = Path::new(r"C:\Users\Example\Product");
        let runtime_home = Path::new(r"c:\users\example\product\.volicord");

        assert_eq!(
            super::runtime_product_path_relation(runtime_home, repo_root),
            RuntimeProductPathRelation::ProductRepositoryContainsRuntimeHome
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_prefix_comparison_preserves_drive_unc_and_wsl_namespaces() {
        fn first_component(path: &Path) -> std::path::Component<'_> {
            path.components().next().expect("path has a prefix")
        }

        assert!(super::windows_components_equal(
            first_component(Path::new(r"C:\Product")),
            first_component(Path::new(r"\\?\c:\product")),
        ));
        assert!(super::windows_components_equal(
            first_component(Path::new(r"\\Server\Share\Product")),
            first_component(Path::new(r"\\server\share\product")),
        ));
        assert!(super::windows_components_equal(
            first_component(Path::new(r"\\wsl$\Ubuntu\home")),
            first_component(Path::new(r"\\WSL$\ubuntu\home")),
        ));
        assert!(!super::windows_components_equal(
            first_component(Path::new(r"\\server\share\product")),
            first_component(Path::new(r"\\wsl$\Ubuntu\home")),
        ));
    }

    #[cfg(windows)]
    #[test]
    fn runtime_product_validation_rejects_unc_runtime_home() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-windows-unc")?;
        let repo_root = fixture.create_product_repo("repo")?;

        let error = validate_runtime_home_product_repository(
            Path::new(r"\\server\share\Volicord"),
            &repo_root,
        )
        .expect_err("UNC runtime homes should be rejected before probing the share");

        assert!(error.to_string().contains("UNC paths are not supported"));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn runtime_product_validation_rejects_wsl_unc_paths() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-windows-wsl-unc")?;
        let repo_root = fixture.create_product_repo("repo")?;

        let error = validate_runtime_home_product_repository(
            Path::new(r"\\wsl$\Ubuntu\home\user\.volicord"),
            &repo_root,
        )
        .expect_err("WSL UNC runtime homes should be rejected");

        assert!(error.to_string().contains("WSL UNC paths"));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn runtime_product_validation_rejects_wsl_mount_syntax() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-windows-wsl-mount")?;
        let repo_root = fixture.create_product_repo("repo")?;

        let error = validate_runtime_home_product_repository(
            Path::new(r"/mnt/c/Users/me/.volicord"),
            &repo_root,
        )
        .expect_err("WSL mount syntax should be rejected in native Windows");

        assert!(error.to_string().contains("WSL-style"));
        Ok(())
    }

    #[test]
    fn runtime_product_validation_rejects_same_path() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-same")?;
        fs::create_dir_all(fixture.path())?;

        let error = validate_runtime_home_product_repository(fixture.path(), fixture.path())
            .expect_err("same path should be rejected");

        assert_eq!(
            error.violation(),
            Some(RuntimePathBoundaryViolation::SamePath)
        );
        assert!(error.to_string().contains("same path"));
        Ok(())
    }

    #[test]
    fn runtime_product_validation_rejects_repository_under_runtime_home(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-runtime-contains")?;
        let repo_root = fixture.path().join("repo");
        fs::create_dir_all(&repo_root)?;

        let error = validate_runtime_home_product_repository(fixture.path(), &repo_root)
            .expect_err("repository under runtime should be rejected");

        assert_eq!(
            error.violation(),
            Some(RuntimePathBoundaryViolation::RuntimeHomeContainsProductRepository)
        );
        Ok(())
    }

    #[test]
    fn runtime_product_validation_rejects_runtime_home_under_repository(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-product-contains")?;
        let repo_root = fixture.create_product_repo("repo")?;
        let runtime_home = repo_root.join(".volicord");

        let error = validate_runtime_home_product_repository(&runtime_home, &repo_root)
            .expect_err("runtime under repository should be rejected");

        assert_eq!(
            error.violation(),
            Some(RuntimePathBoundaryViolation::ProductRepositoryContainsRuntimeHome)
        );
        assert!(!runtime_home.exists());
        Ok(())
    }

    #[test]
    fn runtime_product_validation_rejects_parent_traversal_in_runtime_home(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-parent")?;
        let repo_root = fixture.create_product_repo("repo")?;
        let runtime_home = fixture.path().join("child").join("..").join("runtime");

        let error = validate_runtime_home_product_repository(&runtime_home, &repo_root)
            .expect_err("runtime parent traversal should be rejected");

        assert!(error.to_string().contains("parent traversal"));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn runtime_product_validation_resolves_windows_directory_symlink_aliases_when_available(
    ) -> Result<(), Box<dyn Error>> {
        use std::os::windows::fs::symlink_dir;

        let fixture = TempRuntimeHome::new("boundary-windows-symlink-same")?;
        let repo_root = fixture.create_product_repo("repo")?;
        let runtime_link = fixture
            .path()
            .parent()
            .expect("runtime home has parent")
            .join("runtime-link");

        if let Err(error) = symlink_dir(&repo_root, &runtime_link) {
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                eprintln!(
                    "skipping Windows directory symlink boundary check because symlink creation is not permitted"
                );
                return Ok(());
            }
            return Err(Box::new(error));
        }

        let error = validate_runtime_home_product_repository(&runtime_link, &repo_root)
            .expect_err("directory symlink alias should be rejected as same path");

        assert_eq!(
            error.violation(),
            Some(RuntimePathBoundaryViolation::SamePath)
        );
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn runtime_product_validation_resolves_windows_directory_junction_aliases_when_available(
    ) -> Result<(), Box<dyn Error>> {
        use std::process::Command;

        let fixture = TempRuntimeHome::new("boundary-windows-junction-same")?;
        let repo_root = fixture.create_product_repo("repo")?;
        let runtime_junction = fixture
            .path()
            .parent()
            .expect("runtime home has parent")
            .join("runtime-junction");

        let output = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&runtime_junction)
            .arg(&repo_root)
            .output()?;
        if !output.status.success() {
            eprintln!(
                "skipping Windows directory junction boundary check because junction creation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return Ok(());
        }

        let error = validate_runtime_home_product_repository(&runtime_junction, &repo_root)
            .expect_err("directory junction alias should be rejected as same path");

        assert_eq!(
            error.violation(),
            Some(RuntimePathBoundaryViolation::SamePath)
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn runtime_product_validation_resolves_symlink_aliases() -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::symlink;

        let fixture = TempRuntimeHome::new("boundary-symlink-same")?;
        let repo_root = fixture.create_product_repo("repo")?;
        let runtime_link = fixture
            .path()
            .parent()
            .expect("runtime home has parent")
            .join("runtime-link");
        symlink(&repo_root, &runtime_link)?;

        let error = validate_runtime_home_product_repository(&runtime_link, &repo_root)
            .expect_err("symlink alias should be rejected as same path");

        assert_eq!(
            error.violation(),
            Some(RuntimePathBoundaryViolation::SamePath)
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn runtime_product_validation_resolves_symlink_ancestor_for_missing_runtime(
    ) -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::symlink;

        let fixture = TempRuntimeHome::new("boundary-symlink-ancestor")?;
        let repo_root = fixture.create_product_repo("repo")?;
        let repo_link = fixture
            .path()
            .parent()
            .expect("runtime home has parent")
            .join("repo-link");
        symlink(&repo_root, &repo_link)?;
        let runtime_home = repo_link.join(".volicord");

        let error = validate_runtime_home_product_repository(&runtime_home, &repo_root)
            .expect_err("missing runtime under symlinked repo should be rejected");

        assert_eq!(
            error.violation(),
            Some(RuntimePathBoundaryViolation::ProductRepositoryContainsRuntimeHome)
        );
        assert!(!runtime_home.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_values_are_supported_on_unix() {
        use std::os::unix::ffi::OsStringExt;

        let path = PathBuf::from(OsString::from_vec(b"/tmp/volicord-\xFF-home".to_vec()));

        let resolved = resolve(&[("VOLICORD_HOME", path.clone().into_os_string())])
            .expect("non-UTF-8 VOLICORD_HOME should resolve");

        assert_eq!(resolved, path);
    }
}
