use std::{
    fs, io,
    path::{Path, PathBuf},
};

use volicord_types::product_path::ProductRelativePath;

use crate::{PlatformBoundaryError, PlatformDiagnostic, PlatformDiagnosticKind};

/// Canonical, live observation of one Product Repository root.
///
/// The canonical root remains private so callers cannot use it to reconstruct
/// repository-containment checks around an unrelated raw path.
pub struct ObservedProductRepository {
    canonical_root: PathBuf,
}

impl std::fmt::Debug for ObservedProductRepository {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ObservedProductRepository")
            .finish_non_exhaustive()
    }
}

impl ObservedProductRepository {
    /// Canonicalizes and validates one live Product Repository root.
    pub fn observe(repository_root: &Path) -> Result<Self, PlatformBoundaryError> {
        let canonical_root = fs::canonicalize(repository_root).map_err(repository_root_error)?;
        let metadata = fs::metadata(&canonical_root).map_err(repository_root_error)?;
        if !metadata.is_dir() {
            return Err(product_path_error(
                PlatformDiagnosticKind::InvalidProductRepositoryRoot,
                "the Product Repository root is not a directory",
            ));
        }
        Ok(Self { canonical_root })
    }
    /// Observes containment for a path that may not exist yet.
    pub fn observe_path(
        &self,
        relative_path: ProductRelativePath,
    ) -> Result<ObservedProductPath, PlatformBoundaryError> {
        self.observe_path_with_requirement(relative_path, ExistingPathRequirement::Optional)
    }

    /// Observes containment for a path that must already exist.
    pub fn observe_existing_path(
        &self,
        relative_path: ProductRelativePath,
    ) -> Result<ObservedProductPath, PlatformBoundaryError> {
        self.observe_path_with_requirement(relative_path, ExistingPathRequirement::Required)
    }

    fn observe_path_with_requirement(
        &self,
        relative_path: ProductRelativePath,
        requirement: ExistingPathRequirement,
    ) -> Result<ObservedProductPath, PlatformBoundaryError> {
        let candidate = self.canonical_root.join(relative_path.as_str());
        let (existing, state) =
            nearest_existing_candidate(&candidate, &self.canonical_root, requirement)?;
        let canonical_existing = fs::canonicalize(&existing).map_err(product_candidate_error)?;
        if !canonical_existing.starts_with(&self.canonical_root) {
            return Err(product_path_error(
                PlatformDiagnosticKind::ProductPathContainmentFailure,
                "the Product Repository path resolves outside the canonical repository root",
            ));
        }
        Ok(ObservedProductPath {
            relative_path,
            state,
            _canonical_repository_root: self.canonical_root.clone(),
            _canonical_existing_ancestor: canonical_existing,
        })
    }
}

/// Whether the observed semantic path itself currently exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedProductPathState {
    Existing,
    Missing,
}

/// One platform-observed Product Repository path.
///
/// All filesystem identities are private. Callers may consume only the
/// platform-neutral path and its observed existence state.
pub struct ObservedProductPath {
    relative_path: ProductRelativePath,
    state: ObservedProductPathState,
    _canonical_repository_root: PathBuf,
    _canonical_existing_ancestor: PathBuf,
}

impl ObservedProductPath {
    /// Returns the platform-neutral semantic path that was observed.
    pub fn relative_path(&self) -> &ProductRelativePath {
        &self.relative_path
    }

    /// Returns whether the semantic path itself existed during observation.
    pub const fn state(&self) -> ObservedProductPathState {
        self.state
    }

    /// Consumes the observation and returns its platform-neutral path.
    pub fn into_relative_path(self) -> ProductRelativePath {
        self.relative_path
    }
}

impl std::fmt::Debug for ObservedProductPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ObservedProductPath")
            .field("relative_path", &self.relative_path)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExistingPathRequirement {
    Optional,
    Required,
}

fn nearest_existing_candidate(
    candidate: &Path,
    repository_root: &Path,
    requirement: ExistingPathRequirement,
) -> Result<(PathBuf, ObservedProductPathState), PlatformBoundaryError> {
    let mut existing = candidate.to_path_buf();
    let mut candidate_missing = false;
    loop {
        match fs::symlink_metadata(&existing) {
            Ok(_) => {
                let state = if candidate_missing {
                    ObservedProductPathState::Missing
                } else {
                    ObservedProductPathState::Existing
                };
                return Ok((existing, state));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if requirement == ExistingPathRequirement::Required && !candidate_missing {
                    return Err(product_path_error(
                        PlatformDiagnosticKind::ProductPathNotFound,
                        "the required Product Repository path was not found",
                    ));
                }
                candidate_missing = true;
                if existing == repository_root || !existing.pop() {
                    return Err(product_path_error(
                        PlatformDiagnosticKind::ProductRepositoryNotFound,
                        "the Product Repository root disappeared during path observation",
                    ));
                }
            }
            Err(error) => return Err(product_candidate_error(error)),
        }
    }
}

fn repository_root_error(error: io::Error) -> PlatformBoundaryError {
    let kind = match error.kind() {
        io::ErrorKind::NotFound => PlatformDiagnosticKind::ProductRepositoryNotFound,
        io::ErrorKind::PermissionDenied => PlatformDiagnosticKind::ProductPathInaccessible,
        io::ErrorKind::NotADirectory | io::ErrorKind::InvalidInput => {
            PlatformDiagnosticKind::InvalidProductRepositoryRoot
        }
        _ => PlatformDiagnosticKind::FilesystemObservationFailure,
    };
    product_path_error(
        kind,
        format!("Product Repository root observation failed: {error}"),
    )
}

fn product_candidate_error(error: io::Error) -> PlatformBoundaryError {
    let kind = match error.kind() {
        io::ErrorKind::NotFound => PlatformDiagnosticKind::ProductPathNotFound,
        io::ErrorKind::PermissionDenied | io::ErrorKind::NotADirectory => {
            PlatformDiagnosticKind::ProductPathInaccessible
        }
        _ => PlatformDiagnosticKind::FilesystemObservationFailure,
    };
    product_path_error(
        kind,
        format!("Product Repository path observation failed: {error}"),
    )
}

fn product_path_error(
    kind: PlatformDiagnosticKind,
    detail: impl Into<String>,
) -> PlatformBoundaryError {
    PlatformBoundaryError {
        diagnostic: PlatformDiagnostic::new(kind, detail),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn relative(value: &str) -> ProductRelativePath {
        ProductRelativePath::parse(value).expect("relative path")
    }

    #[test]
    fn canonical_repository_observes_existing_and_missing_candidates() -> io::Result<()> {
        let fixture = tempdir()?;
        let repository_root = fixture.path().join("repository");
        fs::create_dir(&repository_root)?;
        fs::create_dir(repository_root.join("src"))?;
        fs::write(repository_root.join("src/lib.rs"), b"fixture")?;
        let repository = ObservedProductRepository::observe(&repository_root).expect("repository");

        let existing = repository
            .observe_existing_path(relative("src/lib.rs"))
            .expect("existing path");
        assert_eq!(existing.state(), ObservedProductPathState::Existing);
        assert_eq!(existing.relative_path().as_str(), "src/lib.rs");

        let missing = repository
            .observe_path(relative("src/generated/module.rs"))
            .expect("missing path with existing ancestor");
        assert_eq!(missing.state(), ObservedProductPathState::Missing);
        assert_eq!(
            missing.into_relative_path().as_str(),
            "src/generated/module.rs"
        );
        Ok(())
    }

    #[test]
    fn required_existing_candidate_reports_not_found() -> io::Result<()> {
        let fixture = tempdir()?;
        let repository_root = fixture.path().join("repository");
        fs::create_dir(&repository_root)?;
        let repository = ObservedProductRepository::observe(&repository_root).expect("repository");

        let error = repository
            .observe_existing_path(relative("missing.rs"))
            .expect_err("missing existing path");
        assert_eq!(error.kind(), PlatformDiagnosticKind::ProductPathNotFound);
        assert_eq!(error.code(), "platform.product_path.not_found");
        Ok(())
    }

    #[test]
    fn missing_and_invalid_repository_roots_have_distinct_diagnostics() -> io::Result<()> {
        let fixture = tempdir()?;
        let missing = fixture.path().join("missing");
        let missing_error = ObservedProductRepository::observe(&missing).expect_err("missing root");
        assert_eq!(
            missing_error.kind(),
            PlatformDiagnosticKind::ProductRepositoryNotFound
        );

        let file_root = fixture.path().join("file-root");
        fs::write(&file_root, b"not a directory")?;
        let invalid_error = ObservedProductRepository::observe(&file_root).expect_err("file root");
        assert_eq!(
            invalid_error.kind(),
            PlatformDiagnosticKind::InvalidProductRepositoryRoot
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_a_containment_diagnostic() -> io::Result<()> {
        use std::os::unix::fs::symlink;

        let fixture = tempdir()?;
        let repository_root = fixture.path().join("repository");
        let outside = fixture.path().join("outside");
        fs::create_dir(&repository_root)?;
        fs::create_dir(&outside)?;
        symlink(&outside, repository_root.join("escape"))?;
        let repository = ObservedProductRepository::observe(&repository_root).expect("repository");

        let error = repository
            .observe_path(relative("escape/generated.rs"))
            .expect_err("symlink escape");
        assert_eq!(
            error.kind(),
            PlatformDiagnosticKind::ProductPathContainmentFailure
        );
        assert_eq!(error.code(), "platform.product_path.containment_failed");
        assert_eq!(error.class(), crate::PlatformDiagnosticClass::Rejected);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn inaccessible_ancestor_is_an_access_diagnostic() -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let fixture = tempdir()?;
        let repository_root = fixture.path().join("repository");
        let inaccessible = repository_root.join("inaccessible");
        fs::create_dir(&repository_root)?;
        fs::create_dir(&inaccessible)?;
        let repository = ObservedProductRepository::observe(&repository_root).expect("repository");

        fs::set_permissions(&inaccessible, fs::Permissions::from_mode(0o000))?;
        let result = repository.observe_path(relative("inaccessible/generated.rs"));
        fs::set_permissions(&inaccessible, fs::Permissions::from_mode(0o700))?;

        let error = result.expect_err("inaccessible ancestor");
        assert_eq!(
            error.kind(),
            PlatformDiagnosticKind::ProductPathInaccessible
        );
        assert_eq!(error.code(), "platform.product_path.inaccessible");
        Ok(())
    }
}
