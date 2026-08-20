use crate::Error;
use std::{
    env,
    path::{Path, PathBuf},
};
use volicord_context::ProjectId;
use volicord_local_platform::{ensure_private_directory, ensure_private_file, MutationLockGuard};

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
    pub fn forgetting_store(&self) -> PathBuf {
        self.root.join("forgetting.sqlite3")
    }
    pub fn mutation_lock(&self) -> PathBuf {
        self.root.join("mutation.lock")
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

    pub(crate) fn prepare_private_paths(&self) -> Result<(), Error> {
        ensure_private_directory(&self.root)
            .map_err(|error| Error::with_source("Runtime Home is not private", error))?;
        for directory in [
            self.derived_dir(),
            self.analysis_dir(),
            self.artifacts_dir(),
        ] {
            ensure_private_directory(&directory).map_err(|error| {
                Error::with_source("managed Runtime Home directory is not private", error)
            })?;
        }
        for path in [
            self.canonical_store(),
            self.candidate_store(),
            self.privacy_store(),
            self.guarded_store(),
            self.forgetting_store(),
        ] {
            if path.try_exists().map_err(|error| {
                Error::with_source("cannot inspect managed Runtime Home file", error)
            })? {
                ensure_private_file(&path).map_err(|error| {
                    Error::with_source("managed Runtime Home file is not private", error)
                })?;
            }
        }
        ensure_private_file(&self.mutation_lock())
            .map_err(|error| Error::with_source("mutation lock is not private", error))?;
        Ok(())
    }

    pub(crate) fn enforce_private_store_files(&self) -> Result<(), Error> {
        for path in [
            self.canonical_store(),
            self.candidate_store(),
            self.privacy_store(),
            self.guarded_store(),
            self.forgetting_store(),
        ] {
            ensure_private_file(&path).map_err(|error| {
                Error::with_source("managed Runtime Home store is not private", error)
            })?;
            for suffix in ["-wal", "-shm", "-journal"] {
                let mut sidecar = path.as_os_str().to_os_string();
                sidecar.push(suffix);
                let sidecar = PathBuf::from(sidecar);
                if sidecar.try_exists().map_err(|error| {
                    Error::with_source("cannot inspect managed SQLite sidecar", error)
                })? {
                    ensure_private_file(&sidecar).map_err(|error| {
                        Error::with_source("managed SQLite sidecar is not private", error)
                    })?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn acquire_mutation_lock(&self) -> Result<MutationLockGuard, Error> {
        self.prepare_private_paths()?;
        MutationLockGuard::acquire(&self.mutation_lock())
            .map_err(|error| Error::with_source("cannot coordinate Runtime Home mutation", error))
    }
}
