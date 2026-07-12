//! Safe platform filesystem primitives used by Volicord's local adapters.

#![deny(unsafe_code)]

use std::{
    fmt, fs, io,
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};

#[cfg(windows)]
use std::fs::File;

const MAX_GIT_CONTROL_FILE_BYTES: u64 = 4096;

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
    if !matches!(value.len(), 40 | 64) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid_git_data(
            "HEAD does not contain a full Git object id",
        ));
    }
    Ok(value.to_ascii_lowercase())
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
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
