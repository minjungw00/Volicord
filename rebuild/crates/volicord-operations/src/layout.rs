use crate::Error;
use std::{
    env,
    path::{Path, PathBuf},
};
use volicord_context::ProjectId;

const RUNTIME_ENV: &str = "VOLICORD_RUNTIME_DIR";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeLayout {
    root: PathBuf,
}

impl RuntimeLayout {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, Error> {
        let root = root.into();
        if root.as_os_str().is_empty() {
            return Err(Error::new("runtime root must be explicitly supplied"));
        }
        if !root.is_absolute() {
            return Err(Error::new("runtime root must be absolute"));
        }
        Ok(Self { root })
    }

    /// Selects only the current product runtime. No legacy runtime variable
    /// or schema is inspected.
    pub fn from_environment() -> Result<Self, Error> {
        if let Some(root) = env::var_os(RUNTIME_ENV) {
            return Self::new(PathBuf::from(root));
        }
        if let Some(root) = env::var_os("XDG_DATA_HOME") {
            return Self::new(PathBuf::from(root).join("volicord"));
        }
        let home = env::var_os("HOME")
            .ok_or_else(|| Error::new("set VOLICORD_RUNTIME_DIR or XDG_DATA_HOME"))?;
        Self::new(PathBuf::from(home).join(".local/share/volicord"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn canonical_store(&self) -> PathBuf {
        self.root.join("canonical.sqlite3")
    }
    pub fn candidate_store(&self) -> PathBuf {
        self.root.join("candidates.sqlite3")
    }
    pub fn privacy_store(&self) -> PathBuf {
        self.root.join("privacy.sqlite3")
    }
    pub fn guarded_store(&self) -> PathBuf {
        self.root.join("guarded.sqlite3")
    }
    pub fn derived_dir(&self) -> PathBuf {
        self.root.join("derived")
    }
    pub fn analysis_dir(&self) -> PathBuf {
        self.derived_dir().join("analysis")
    }
    pub fn analysis_project_dir(&self, project_id: ProjectId) -> PathBuf {
        self.analysis_dir().join(project_id.to_string())
    }
    pub fn artifacts_dir(&self) -> PathBuf {
        self.root.join("operations")
    }
}
