use std::{
    error::Error,
    ffi::OsString,
    fmt, fs, io,
    path::{Component, Path, PathBuf},
};

const HARNESS_HOME: &str = "HARNESS_HOME";
const HOME: &str = "HOME";
const USERPROFILE: &str = "USERPROFILE";
const HOMEDRIVE: &str = "HOMEDRIVE";
const HOMEPATH: &str = "HOMEPATH";

/// Errors returned while selecting a Runtime Home path from process inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomeResolutionError {
    EmptyHarnessHome,
    MissingUserHome,
}

impl fmt::Display for RuntimeHomeResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyHarnessHome => formatter.write_str("HARNESS_HOME must not be empty"),
            Self::MissingUserHome => formatter
                .write_str("could not determine a default home directory; set HARNESS_HOME"),
        }
    }
}

impl Error for RuntimeHomeResolutionError {}

/// Component-aware relation between `Harness Runtime Home` and
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
}

impl RuntimePathBoundaryError {
    pub fn violation(&self) -> Option<RuntimePathBoundaryViolation> {
        match self {
            Self::BoundaryViolation { violation, .. } => Some(*violation),
            Self::InvalidPath { .. } => None,
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
                    "Harness Runtime Home and Product Repository must not be the same path: runtime_home {}, repo_root {}",
                    runtime_home.display(),
                    repo_root.display()
                ),
                RuntimePathBoundaryViolation::RuntimeHomeContainsProductRepository => write!(
                    formatter,
                    "Product Repository must not be inside Harness Runtime Home: runtime_home {}, repo_root {}",
                    runtime_home.display(),
                    repo_root.display()
                ),
                RuntimePathBoundaryViolation::ProductRepositoryContainsRuntimeHome => write!(
                    formatter,
                    "Harness Runtime Home must not be inside Product Repository: runtime_home {}, repo_root {}",
                    runtime_home.display(),
                    repo_root.display()
                ),
                RuntimePathBoundaryViolation::ProjectHomeOutsideRuntimeHome => {
                    let project_home = project_home
                        .as_ref()
                        .expect("project-home violation carries project_home");
                    write!(
                        formatter,
                        "project_home must be inside Harness Runtime Home: runtime_home {}, project_home {}",
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
        }
    }
}

impl Error for RuntimePathBoundaryError {}

/// Resolves the Harness Runtime Home path from environment values and a cwd.
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
    if let Some(value) = env_var(HARNESS_HOME) {
        if value.is_empty() {
            return Err(RuntimeHomeResolutionError::EmptyHarnessHome);
        }
        return Ok(absolute_path(current_dir, PathBuf::from(value)));
    }

    let home = default_user_home(env_var).ok_or(RuntimeHomeResolutionError::MissingUserHome)?;
    Ok(absolute_path(current_dir, home).join(".harness"))
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
    if runtime_home == repo_root {
        RuntimeProductPathRelation::SamePath
    } else if repo_root.starts_with(runtime_home) {
        RuntimeProductPathRelation::RuntimeHomeContainsProductRepository
    } else if runtime_home.starts_with(repo_root) {
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

    if paths_overlap(&project_home, &repo_root) {
        return Err(RuntimePathBoundaryError::BoundaryViolation {
            violation: RuntimePathBoundaryViolation::ProjectHomeOverlapsProductRepository,
            runtime_home,
            repo_root,
            project_home: Some(project_home),
        });
    }

    if !project_home.starts_with(&runtime_home) {
        return Err(RuntimePathBoundaryError::BoundaryViolation {
            violation: RuntimePathBoundaryViolation::ProjectHomeOutsideRuntimeHome,
            runtime_home,
            repo_root,
            project_home: Some(project_home),
        });
    }

    Ok(project_home)
}

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
    let canonical = fs::canonicalize(&absolute).map_err(|error| {
        invalid_path(
            role,
            path,
            format!("directory does not exist or is not accessible: {error}"),
        )
    })?;
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
    normalize_lexical_components(role, path, &absolute)
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

fn paths_overlap(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
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
    use std::{
        error::Error,
        ffi::OsString,
        fs,
        path::{Path, PathBuf},
    };

    use harness_test_support::TempRuntimeHome;

    use super::{
        resolve_runtime_home, validate_runtime_home_product_repository, RuntimeHomeResolutionError,
        RuntimePathBoundaryViolation, RuntimeProductPathRelation,
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
    fn absolute_harness_home_is_used_as_supplied() {
        let path = cwd().join("runtime-home-absolute");

        let resolved = resolve(&[("HARNESS_HOME", path.clone().into_os_string())])
            .expect("absolute HARNESS_HOME should resolve");

        assert_eq!(resolved, path);
    }

    #[test]
    fn relative_harness_home_is_resolved_against_current_dir() {
        let resolved = resolve(&[("HARNESS_HOME", OsString::from("runtime-home-relative"))])
            .expect("relative HARNESS_HOME should resolve");

        assert_eq!(resolved, cwd().join("runtime-home-relative"));
    }

    #[test]
    fn empty_harness_home_is_an_error() {
        let error = resolve(&[("HARNESS_HOME", OsString::new())])
            .expect_err("empty HARNESS_HOME should fail");

        assert_eq!(error, RuntimeHomeResolutionError::EmptyHarnessHome);
        assert!(error.to_string().contains("HARNESS_HOME"));
    }

    #[test]
    fn home_fallback_appends_harness() {
        let home = cwd().join("home-fallback");

        let resolved =
            resolve(&[("HOME", home.clone().into_os_string())]).expect("HOME should resolve");

        assert_eq!(resolved, home.join(".harness"));
    }

    #[test]
    fn userprofile_fallback_is_used_after_missing_home() {
        let home = cwd().join("userprofile-fallback");

        let resolved = resolve(&[("USERPROFILE", home.clone().into_os_string())])
            .expect("USERPROFILE should resolve");

        assert_eq!(resolved, home.join(".harness"));
    }

    #[test]
    fn homedrive_and_homepath_fallback_are_combined() {
        let drive = cwd().join("drive-fallback");

        let resolved = resolve(&[
            ("HOMEDRIVE", drive.clone().into_os_string()),
            ("HOMEPATH", OsString::from("homepath")),
        ])
        .expect("HOMEDRIVE and HOMEPATH should resolve");

        assert_eq!(resolved, drive.join("homepath").join(".harness"));
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

        assert_eq!(resolved, userprofile.join(".harness"));
    }

    #[test]
    fn relative_fallback_home_is_made_absolute() {
        let resolved = resolve(&[("HOME", OsString::from("relative-home"))])
            .expect("relative HOME should resolve");

        assert_eq!(resolved, cwd().join("relative-home").join(".harness"));
        assert!(resolved.is_absolute());
    }

    #[test]
    fn no_available_home_source_is_an_error() {
        let error = resolve(&[]).expect_err("missing home sources should fail");

        assert_eq!(error, RuntimeHomeResolutionError::MissingUserHome);
        assert!(error.to_string().contains("set HARNESS_HOME"));
    }

    #[test]
    fn selected_runtime_home_is_not_canonicalized_or_required_to_exist() {
        let resolved = resolve(&[(
            "HARNESS_HOME",
            OsString::from("missing-runtime-home/../still-missing"),
        )])
        .expect("nonexistent relative HARNESS_HOME should resolve");

        assert_eq!(
            resolved,
            cwd().join(Path::new("missing-runtime-home/../still-missing"))
        );
    }

    #[test]
    fn runtime_product_validation_accepts_separate_sibling_paths() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-siblings")?;
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

    #[test]
    fn runtime_product_validation_rejects_same_path() -> Result<(), Box<dyn Error>> {
        let fixture = TempRuntimeHome::new("boundary-same")?;

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
        let runtime_home = repo_root.join(".harness");

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
        let runtime_home = repo_link.join(".harness");

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

        let path = PathBuf::from(OsString::from_vec(b"/tmp/harness-\xFF-home".to_vec()));

        let resolved = resolve(&[("HARNESS_HOME", path.clone().into_os_string())])
            .expect("non-UTF-8 HARNESS_HOME should resolve");

        assert_eq!(resolved, path);
    }
}
