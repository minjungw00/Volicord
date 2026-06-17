#![forbid(unsafe_code)]

//! Shared implementation-test helpers.
//!
//! Helpers in this crate should use disposable locations, such as `/tmp`, for
//! future runtime homes and fixture output.

use std::path::{Path, PathBuf};

use harness_types::TypeBoundary;
use tempfile::{Builder, TempDir};

pub mod fixtures {
    /// Placement marker for future shared fixtures.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct FixtureBoundary;
}

pub mod golden {
    /// Placement marker for future golden-output helpers.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct GoldenBoundary;
}

/// Returns a candidate disposable runtime-home path without creating it.
pub fn disposable_runtime_home(name: &str) -> PathBuf {
    std::env::temp_dir().join("harness-test-runtime").join(name)
}

/// Automatically cleaned disposable Runtime Home for implementation tests.
#[derive(Debug)]
pub struct TempRuntimeHome {
    dir: TempDir,
}

impl TempRuntimeHome {
    /// Creates a new empty Runtime Home under the system temporary directory.
    pub fn new(prefix: &str) -> std::io::Result<Self> {
        let dir = Builder::new()
            .prefix(&format!("harness-runtime-{prefix}-"))
            .tempdir()?;
        Ok(Self { dir })
    }

    /// Returns the Runtime Home directory path.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Returns the `registry.sqlite` path under this Runtime Home.
    pub fn registry_db_path(&self) -> PathBuf {
        self.path().join("registry.sqlite")
    }

    /// Returns the project home path under this Runtime Home.
    pub fn project_home_path(&self, project_id: &str) -> PathBuf {
        self.path().join("projects").join(project_id)
    }

    /// Returns the project-local `state.sqlite` path under this Runtime Home.
    pub fn project_state_db_path(&self, project_id: &str) -> PathBuf {
        self.project_home_path(project_id).join("state.sqlite")
    }

    /// Returns the transient artifact staging path under this Runtime Home.
    pub fn artifacts_tmp_path(&self, project_id: &str) -> PathBuf {
        self.project_home_path(project_id)
            .join("artifacts")
            .join("tmp")
    }
}

/// Identifies the shared type boundary used by test helpers.
pub const fn shared_type_boundary() -> TypeBoundary {
    TypeBoundary::Domain
}

#[cfg(test)]
mod tests {
    use super::{disposable_runtime_home, shared_type_boundary, TempRuntimeHome};
    use harness_types::TypeBoundary;

    #[test]
    fn disposable_runtime_home_stays_under_system_temp() {
        let path = disposable_runtime_home("workspace-skeleton");
        assert!(path.is_absolute());
        assert!(path.ends_with("harness-test-runtime/workspace-skeleton"));
    }

    #[test]
    fn test_support_uses_domain_type_boundary() {
        assert_eq!(shared_type_boundary(), TypeBoundary::Domain);
    }

    #[test]
    fn temp_runtime_home_uses_disposable_directory() {
        let runtime_home = TempRuntimeHome::new("helpers").expect("tempdir should be created");
        assert!(runtime_home.path().is_absolute());
        assert!(runtime_home.path().exists());
        assert!(runtime_home.registry_db_path().ends_with("registry.sqlite"));
        assert!(runtime_home
            .project_state_db_path("PRJ-helpers")
            .ends_with("projects/PRJ-helpers/state.sqlite"));
        assert!(runtime_home
            .artifacts_tmp_path("PRJ-helpers")
            .ends_with("projects/PRJ-helpers/artifacts/tmp"));
    }
}
