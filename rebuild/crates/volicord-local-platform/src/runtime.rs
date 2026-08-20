use std::{
    fs::{self, File, OpenOptions},
    io,
    path::Path,
};

#[cfg(target_os = "linux")]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

#[derive(Debug)]
pub struct PrivateRuntimeError {
    detail: String,
    source: Option<io::Error>,
}

impl PrivateRuntimeError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            source: None,
        }
    }

    fn with_source(detail: impl Into<String>, source: io::Error) -> Self {
        Self {
            detail: detail.into(),
            source: Some(source),
        }
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl std::fmt::Display for PrivateRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for PrivateRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}

/// Creates or repairs an owner-controlled Linux directory to mode `0700`.
///
/// A symbolic link, a non-directory, or a directory owned by another effective
/// user is rejected before any permission repair is attempted.
pub fn ensure_private_directory(path: &Path) -> Result<(), PrivateRuntimeError> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        return Err(PrivateRuntimeError::new(
            "private Runtime Home access is supported only on Linux",
        ));
    }

    #[cfg(target_os = "linux")]
    {
        match fs::symlink_metadata(path) {
            Ok(metadata) => validate_and_repair(path, &metadata, PrivatePathKind::Directory),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                builder.recursive(true).mode(0o700);
                match builder.create(path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => {
                        return Err(PrivateRuntimeError::with_source(
                            format!("cannot create private directory {}", path.display()),
                            error,
                        ));
                    }
                }
                let metadata = fs::symlink_metadata(path).map_err(|error| {
                    PrivateRuntimeError::with_source(
                        format!("cannot verify private directory {}", path.display()),
                        error,
                    )
                })?;
                validate_and_repair(path, &metadata, PrivatePathKind::Directory)
            }
            Err(error) => Err(PrivateRuntimeError::with_source(
                format!("cannot inspect private directory {}", path.display()),
                error,
            )),
        }
    }
}

/// Creates or repairs an owner-controlled Linux regular file to mode `0600`.
///
/// The file is created without read/write/execute permission for group or
/// other users even when the ambient umask is permissive.
pub fn ensure_private_file(path: &Path) -> Result<(), PrivateRuntimeError> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        return Err(PrivateRuntimeError::new(
            "private Runtime Home access is supported only on Linux",
        ));
    }

    #[cfg(target_os = "linux")]
    {
        match fs::symlink_metadata(path) {
            Ok(metadata) => validate_and_repair(path, &metadata, PrivatePathKind::File),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let mut options = OpenOptions::new();
                options.create_new(true).read(true).write(true).mode(0o600);
                match options.open(path) {
                    Ok(file) => drop(file),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => {
                        return Err(PrivateRuntimeError::with_source(
                            format!("cannot create private file {}", path.display()),
                            error,
                        ));
                    }
                }
                let metadata = fs::symlink_metadata(path).map_err(|error| {
                    PrivateRuntimeError::with_source(
                        format!("cannot verify private file {}", path.display()),
                        error,
                    )
                })?;
                validate_and_repair(path, &metadata, PrivatePathKind::File)
            }
            Err(error) => Err(PrivateRuntimeError::with_source(
                format!("cannot inspect private file {}", path.display()),
                error,
            )),
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
enum PrivatePathKind {
    Directory,
    File,
}

#[cfg(target_os = "linux")]
fn validate_and_repair(
    path: &Path,
    metadata: &fs::Metadata,
    kind: PrivatePathKind,
) -> Result<(), PrivateRuntimeError> {
    if metadata.file_type().is_symlink() {
        return Err(PrivateRuntimeError::new(format!(
            "managed Runtime Home path {} may not be a symbolic link",
            path.display()
        )));
    }
    let correct_kind = match kind {
        PrivatePathKind::Directory => metadata.is_dir(),
        PrivatePathKind::File => metadata.is_file(),
    };
    if !correct_kind {
        return Err(PrivateRuntimeError::new(format!(
            "managed Runtime Home path {} has the wrong file type",
            path.display()
        )));
    }
    let effective_uid = rustix::process::geteuid().as_raw();
    if metadata.uid() != effective_uid {
        return Err(PrivateRuntimeError::new(format!(
            "managed Runtime Home path {} is owned by uid {}, expected effective uid {}",
            path.display(),
            metadata.uid(),
            effective_uid
        )));
    }
    let required_mode = match kind {
        PrivatePathKind::Directory => 0o700,
        PrivatePathKind::File => 0o600,
    };
    if metadata.mode() & 0o7777 != required_mode {
        fs::set_permissions(path, fs::Permissions::from_mode(required_mode)).map_err(|error| {
            PrivateRuntimeError::with_source(
                format!("cannot enforce private permissions on {}", path.display()),
                error,
            )
        })?;
        let repaired = fs::symlink_metadata(path).map_err(|error| {
            PrivateRuntimeError::with_source(
                format!("cannot verify repaired permissions on {}", path.display()),
                error,
            )
        })?;
        if repaired.file_type().is_symlink()
            || repaired.uid() != effective_uid
            || repaired.mode() & 0o7777 != required_mode
        {
            return Err(PrivateRuntimeError::new(format!(
                "managed Runtime Home path {} changed during permission enforcement",
                path.display()
            )));
        }
    }
    Ok(())
}

/// An exclusive advisory lock held by an open file description.
///
/// The lock is released by the kernel when this value is dropped, including
/// after process termination.
#[derive(Debug)]
pub struct MutationLockGuard {
    file: File,
}

impl MutationLockGuard {
    pub fn acquire(path: &Path) -> Result<Self, PrivateRuntimeError> {
        Self::open_and_lock(path, false)?.ok_or_else(|| {
            PrivateRuntimeError::new("blocking mutation lock unexpectedly remained unavailable")
        })
    }

    pub fn try_acquire(path: &Path) -> Result<Option<Self>, PrivateRuntimeError> {
        Self::open_and_lock(path, true)
    }

    #[cfg(target_os = "linux")]
    fn open_and_lock(path: &Path, nonblocking: bool) -> Result<Option<Self>, PrivateRuntimeError> {
        ensure_private_file(path)?;
        use rustix::fs::{flock, open, FlockOperation, Mode, OFlags};
        let descriptor = open(
            path,
            OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(|error| {
            PrivateRuntimeError::with_source(
                format!("cannot open Runtime Home mutation lock {}", path.display()),
                io::Error::from(error),
            )
        })?;
        let file = File::from(descriptor);
        let operation = if nonblocking {
            FlockOperation::NonBlockingLockExclusive
        } else {
            FlockOperation::LockExclusive
        };
        match flock(&file, operation) {
            Ok(()) => Ok(Some(Self { file })),
            Err(error) if nonblocking && error == rustix::io::Errno::WOULDBLOCK => Ok(None),
            Err(error) => Err(PrivateRuntimeError::with_source(
                format!(
                    "cannot acquire Runtime Home mutation lock {}",
                    path.display()
                ),
                io::Error::from(error),
            )),
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn open_and_lock(path: &Path, _nonblocking: bool) -> Result<Option<Self>, PrivateRuntimeError> {
        let _ = path;
        Err(PrivateRuntimeError::new(
            "Runtime Home mutation coordination is supported only on Linux",
        ))
    }

    pub fn file(&self) -> &File {
        &self.file
    }
}
