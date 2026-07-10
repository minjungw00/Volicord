//! Safe platform filesystem primitives used by Volicord's local adapters.

#![deny(unsafe_code)]

use std::{fmt, io};

#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
use std::fs::File;

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
    use super::*;

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
