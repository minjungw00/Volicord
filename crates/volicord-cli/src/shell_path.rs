use std::{
    env,
    ffi::{OsStr, OsString},
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const PATH_ENV: &str = "PATH";
static WRITE_PROBE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SetupLinkDirCandidate {
    ExistingVerifiedWritable(PathBuf),
    MissingCreatableUserBin(PathBuf),
    ExistingNotWritable(PathBuf),
    Unavailable(PathBuf),
}

impl SetupLinkDirCandidate {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::ExistingVerifiedWritable(path)
            | Self::MissingCreatableUserBin(path)
            | Self::ExistingNotWritable(path)
            | Self::Unavailable(path) => path,
        }
    }

    pub(crate) fn is_usable(&self) -> bool {
        matches!(
            self,
            Self::ExistingVerifiedWritable(_) | Self::MissingCreatableUserBin(_)
        )
    }

    pub(crate) fn requires_creation(&self) -> bool {
        matches!(self, Self::MissingCreatableUserBin(_))
    }
}

pub(crate) fn detect_command_on_path(
    command_name: &str,
    path_env: Option<&OsStr>,
) -> Option<PathBuf> {
    path_env
        .map(env::split_paths)?
        .map(|dir| dir.join(command_name))
        .find(|candidate| is_executable_file(candidate))
        .map(|candidate| fs::canonicalize(&candidate).unwrap_or(candidate))
}

pub(crate) fn path_directory_is_on_path(path_env: Option<&OsStr>, dir: &Path) -> bool {
    path_env
        .map(env::split_paths)
        .into_iter()
        .flatten()
        .any(|candidate| paths_equivalent(&candidate, dir))
}

pub(crate) fn path_directory_is_verified_writable(path: &Path) -> bool {
    verify_directory_writable(path).is_ok()
}

pub(crate) fn verify_directory_writable(path: &Path) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a directory", path.display()),
        ));
    }

    let mut last_collision = None;
    for _ in 0..16 {
        let probe_path = path.join(unique_write_probe_name());
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe_path)
        {
            Ok(mut file) => {
                let write_result = file
                    .write_all(b"volicord PATH write probe\n")
                    .and_then(|()| file.flush());
                drop(file);
                let cleanup_result = fs::remove_file(&probe_path);
                return match (write_result, cleanup_result) {
                    (Ok(()), Ok(())) => Ok(()),
                    (Err(error), _) => {
                        let _ = fs::remove_file(&probe_path);
                        Err(error)
                    }
                    (Ok(()), Err(error)) => Err(io::Error::new(
                        error.kind(),
                        format!(
                            "created probe file but could not remove {}: {error}",
                            probe_path.display()
                        ),
                    )),
                };
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                last_collision = Some(error);
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_collision.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "could not create a unique Volicord setup probe file in {}",
                path.display()
            ),
        )
    }))
}

fn unique_write_probe_name() -> String {
    let counter = WRITE_PROBE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!(
        ".volicord-setup-write-probe-{}-{nanos}-{counter}.tmp",
        std::process::id()
    )
}

pub(crate) fn candidate_user_bin_dirs<F>(env_var: &F) -> Vec<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    let mut dirs = Vec::new();
    if let Some(home) = env_var("HOME").filter(|value| !value.is_empty()) {
        let home = PathBuf::from(home);
        dirs.push(home.join(".local").join("bin"));
        dirs.push(home.join("bin"));
    }
    #[cfg(windows)]
    if let Some(local_app_data) = env_var("LOCALAPPDATA").filter(|value| !value.is_empty()) {
        dirs.push(PathBuf::from(local_app_data).join("Volicord").join("bin"));
    }
    dirs
}

pub(crate) fn setup_link_dir_candidates<F>(env_var: &F) -> Vec<SetupLinkDirCandidate>
where
    F: Fn(&str) -> Option<OsString>,
{
    let mut candidates = Vec::new();
    if let Some(path_env) = env_var(PATH_ENV) {
        for dir in env::split_paths(&path_env) {
            if let Some(candidate) = classify_path_candidate(&dir) {
                push_unique_candidate(&mut candidates, candidate);
            }
        }
    }
    let home = env_var("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    for dir in candidate_user_bin_dirs(env_var) {
        push_unique_candidate(
            &mut candidates,
            classify_user_bin_candidate(&dir, home.as_deref()),
        );
    }
    candidates
}

fn classify_path_candidate(path: &Path) -> Option<SetupLinkDirCandidate> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => {
            if path_directory_is_verified_writable(path) {
                Some(SetupLinkDirCandidate::ExistingVerifiedWritable(
                    path.to_path_buf(),
                ))
            } else {
                Some(SetupLinkDirCandidate::ExistingNotWritable(
                    path.to_path_buf(),
                ))
            }
        }
        Ok(_) => Some(SetupLinkDirCandidate::Unavailable(path.to_path_buf())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(_) => Some(SetupLinkDirCandidate::Unavailable(path.to_path_buf())),
    }
}

fn classify_user_bin_candidate(path: &Path, home: Option<&Path>) -> SetupLinkDirCandidate {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => {
            if path_directory_is_verified_writable(path) {
                SetupLinkDirCandidate::ExistingVerifiedWritable(path.to_path_buf())
            } else {
                SetupLinkDirCandidate::ExistingNotWritable(path.to_path_buf())
            }
        }
        Ok(_) => SetupLinkDirCandidate::Unavailable(path.to_path_buf()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if home.is_some_and(|home| missing_user_bin_is_safely_creatable(home, path)) {
                SetupLinkDirCandidate::MissingCreatableUserBin(path.to_path_buf())
            } else {
                SetupLinkDirCandidate::Unavailable(path.to_path_buf())
            }
        }
        Err(_) => SetupLinkDirCandidate::Unavailable(path.to_path_buf()),
    }
}

fn missing_user_bin_is_safely_creatable(home: &Path, path: &Path) -> bool {
    if !home.is_absolute() || !path.is_absolute() {
        return false;
    }
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        _ => return false,
    }
    match fs::metadata(home) {
        Ok(metadata) if metadata.is_dir() => {}
        _ => return false,
    }

    let Ok(relative) = path.strip_prefix(home) else {
        return false;
    };
    if relative == Path::new("bin") {
        return verify_directory_writable(home).is_ok();
    }
    if relative != Path::new(".local").join("bin") {
        return false;
    }

    let local_dir = home.join(".local");
    match fs::symlink_metadata(&local_dir) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            verify_directory_writable(&local_dir).is_ok()
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            verify_directory_writable(home).is_ok()
        }
        _ => false,
    }
}

fn push_unique_candidate(
    candidates: &mut Vec<SetupLinkDirCandidate>,
    candidate: SetupLinkDirCandidate,
) {
    if !candidates
        .iter()
        .any(|existing| paths_equivalent_or_equal(existing.path(), candidate.path()))
    {
        candidates.push(candidate);
    }
}

fn paths_equivalent_or_equal(left: &Path, right: &Path) -> bool {
    left == right || paths_equivalent(left, right)
}

pub(crate) fn paths_equivalent(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

pub(crate) fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    is_executable_metadata(&metadata)
}

#[cfg(unix)]
fn is_executable_metadata(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable_metadata(_metadata: &fs::Metadata) -> bool {
    true
}

pub(crate) fn volicord_binary_name() -> String {
    format!("volicord{}", env::consts::EXE_SUFFIX)
}

pub(crate) fn mcp_binary_name() -> String {
    volicord_binary_name()
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        io::Write,
        path::{Path, PathBuf},
    };

    use volicord_test_support::TempRuntimeHome;

    use super::*;

    #[test]
    fn detect_command_on_path_finds_executable_command() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("shell-path-detect-command")?;
        let path_dir = fixture.path().join("path-bin");
        let command = write_executable(&path_dir, &volicord_binary_name())?;
        let path_env = env::join_paths([path_dir.as_path()])?;

        let detected = detect_command_on_path(&volicord_binary_name(), Some(&path_env))
            .expect("command should be found on PATH");

        assert_eq!(detected, fs::canonicalize(command)?);
        Ok(())
    }

    #[test]
    fn candidate_setup_link_dirs_prefers_writable_path_dirs(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("shell-path-candidates")?;
        let path_dir = fixture.path().join("path-bin");
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        fs::create_dir_all(&path_dir)?;
        fs::create_dir_all(&local_bin)?;
        let env = BTreeMap::from([
            (PATH_ENV.to_owned(), env::join_paths([path_dir.as_path()])?),
            ("HOME".to_owned(), home.clone().into_os_string()),
        ]);

        let candidates = setup_link_dir_candidates(&|name| env.get(name).cloned());
        let dirs = usable_candidate_dirs(&candidates);

        assert_eq!(dirs.first(), Some(&path_dir));
        assert!(dirs.contains(&local_bin));
        assert_eq!(fs::read_dir(&path_dir)?.count(), 0);
        assert_eq!(fs::read_dir(&local_bin)?.count(), 0);
        Ok(())
    }

    #[test]
    fn candidate_setup_link_dirs_uses_user_bin_when_path_has_no_writable_directory(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("shell-path-user-bin")?;
        let path_file = fixture.path().join("path-file");
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        let home_bin = home.join("bin");
        fs::write(&path_file, "not a directory")?;
        fs::create_dir_all(&local_bin)?;
        fs::create_dir_all(&home_bin)?;
        let env = BTreeMap::from([
            (PATH_ENV.to_owned(), env::join_paths([path_file.as_path()])?),
            ("HOME".to_owned(), home.clone().into_os_string()),
        ]);

        let candidates = setup_link_dir_candidates(&|name| env.get(name).cloned());
        let dirs = usable_candidate_dirs(&candidates);

        assert_eq!(dirs.first(), Some(&local_bin));
        assert!(dirs.contains(&home_bin));
        assert!(!dirs.contains(&path_file));
        assert_eq!(fs::read_dir(&local_bin)?.count(), 0);
        assert_eq!(fs::read_dir(&home_bin)?.count(), 0);
        Ok(())
    }

    #[test]
    fn candidate_setup_link_dirs_offers_missing_creatable_user_bin_dirs(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("shell-path-missing-user-bin")?;
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        let env = BTreeMap::from([("HOME".to_owned(), home.clone().into_os_string())]);

        let candidates = setup_link_dir_candidates(&|name| env.get(name).cloned());
        let dirs = usable_candidate_dirs(&candidates);

        assert_eq!(
            dirs,
            vec![home.join(".local").join("bin"), home.join("bin")]
        );
        assert!(
            candidates.contains(&SetupLinkDirCandidate::MissingCreatableUserBin(
                home.join(".local").join("bin")
            ))
        );
        assert!(
            candidates.contains(&SetupLinkDirCandidate::MissingCreatableUserBin(
                home.join("bin")
            ))
        );
        assert!(!home.join(".local").exists());
        assert!(!home.join("bin").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn candidate_setup_link_dirs_does_not_offer_missing_user_bins_when_home_is_unwritable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let fixture = TempRuntimeHome::new("shell-path-unwritable-home-user-bin")?;
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        let home_bin = home.join("bin");
        fs::create_dir_all(&home)?;
        let mut permissions = fs::metadata(&home)?.permissions();
        permissions.set_mode(0o555);
        fs::set_permissions(&home, permissions)?;

        if path_directory_is_verified_writable(&home) {
            restore_writable_dir(&home)?;
            return Ok(());
        }

        let env = BTreeMap::from([("HOME".to_owned(), home.clone().into_os_string())]);
        let candidates = setup_link_dir_candidates(&|name| env.get(name).cloned());
        let dirs = usable_candidate_dirs(&candidates);

        restore_writable_dir(&home)?;
        assert!(dirs.is_empty());
        assert!(candidates.contains(&SetupLinkDirCandidate::Unavailable(local_bin.clone())));
        assert!(candidates.contains(&SetupLinkDirCandidate::Unavailable(home_bin.clone())));
        assert!(!local_bin.exists());
        assert!(!home_bin.exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn candidate_setup_link_dirs_does_not_offer_missing_local_bin_when_parent_is_unwritable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let fixture = TempRuntimeHome::new("shell-path-unwritable-local-user-bin")?;
        let home = fixture.path().join("home");
        let local = home.join(".local");
        let local_bin = local.join("bin");
        let home_bin = home.join("bin");
        fs::create_dir_all(&local)?;
        let mut permissions = fs::metadata(&local)?.permissions();
        permissions.set_mode(0o555);
        fs::set_permissions(&local, permissions)?;

        if path_directory_is_verified_writable(&local) {
            restore_writable_dir(&local)?;
            return Ok(());
        }

        let env = BTreeMap::from([("HOME".to_owned(), home.clone().into_os_string())]);
        let candidates = setup_link_dir_candidates(&|name| env.get(name).cloned());
        let dirs = usable_candidate_dirs(&candidates);

        restore_writable_dir(&local)?;
        assert!(!dirs.contains(&local_bin));
        assert!(dirs.contains(&home_bin));
        assert!(candidates.contains(&SetupLinkDirCandidate::Unavailable(local_bin.clone())));
        assert!(
            candidates.contains(&SetupLinkDirCandidate::MissingCreatableUserBin(
                home_bin.clone()
            ))
        );
        assert!(!local_bin.exists());
        assert!(!home_bin.exists());
        Ok(())
    }

    #[test]
    fn candidate_setup_link_dirs_does_not_offer_arbitrary_missing_path_dirs(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("shell-path-missing-path-dir")?;
        let home = fixture.path().join("home");
        let missing_path_dir = fixture.path().join("missing-path-bin");
        fs::create_dir_all(&home)?;
        let env = BTreeMap::from([
            (
                PATH_ENV.to_owned(),
                env::join_paths([missing_path_dir.as_path()])?,
            ),
            ("HOME".to_owned(), home.clone().into_os_string()),
        ]);

        let candidates = setup_link_dir_candidates(&|name| env.get(name).cloned());
        let dirs = usable_candidate_dirs(&candidates);

        assert!(!dirs.contains(&missing_path_dir));
        assert!(!candidates
            .iter()
            .any(|candidate| candidate.path() == missing_path_dir.as_path()));
        assert!(dirs.contains(&home.join(".local").join("bin")));
        assert!(!missing_path_dir.exists());
        Ok(())
    }

    #[test]
    fn candidate_setup_link_dirs_marks_user_bin_with_unsafe_parent_unavailable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("shell-path-unsafe-user-bin")?;
        let home = fixture.path().join("home");
        fs::create_dir_all(&home)?;
        fs::write(home.join(".local"), "not a directory")?;
        let env = BTreeMap::from([("HOME".to_owned(), home.clone().into_os_string())]);

        let candidates = setup_link_dir_candidates(&|name| env.get(name).cloned());
        let dirs = usable_candidate_dirs(&candidates);

        assert!(!dirs.contains(&home.join(".local").join("bin")));
        assert!(dirs.contains(&home.join("bin")));
        assert!(candidates.contains(&SetupLinkDirCandidate::Unavailable(
            home.join(".local").join("bin")
        )));
        assert!(!home.join("bin").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn candidate_setup_link_dirs_marks_broken_user_bin_symlink_unavailable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::symlink;

        let fixture = TempRuntimeHome::new("shell-path-broken-user-bin-symlink")?;
        let home = fixture.path().join("home");
        let local = home.join(".local");
        let local_bin = local.join("bin");
        fs::create_dir_all(&local)?;
        symlink(home.join("missing-target"), &local_bin)?;
        let env = BTreeMap::from([("HOME".to_owned(), home.clone().into_os_string())]);

        let candidates = setup_link_dir_candidates(&|name| env.get(name).cloned());
        let dirs = usable_candidate_dirs(&candidates);

        assert!(!dirs.contains(&local_bin));
        assert!(candidates.contains(&SetupLinkDirCandidate::Unavailable(local_bin)));
        Ok(())
    }

    #[test]
    fn writable_directory_probe_cleans_up_probe_file() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("shell-path-write-probe")?;
        let path_dir = fixture.path().join("path-bin");
        fs::create_dir_all(&path_dir)?;

        verify_directory_writable(&path_dir)?;

        assert_eq!(fs::read_dir(&path_dir)?.count(), 0);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn candidate_setup_link_dirs_skips_path_dir_when_write_probe_fails(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let fixture = TempRuntimeHome::new("shell-path-unwritable-path-dir")?;
        let path_dir = fixture.path().join("path-bin");
        let home = fixture.path().join("home");
        let local_bin = home.join(".local").join("bin");
        fs::create_dir_all(&path_dir)?;
        fs::create_dir_all(&local_bin)?;
        let mut permissions = fs::metadata(&path_dir)?.permissions();
        permissions.set_mode(0o555);
        fs::set_permissions(&path_dir, permissions)?;

        if path_directory_is_verified_writable(&path_dir) {
            restore_writable_dir(&path_dir)?;
            return Ok(());
        }

        let env = BTreeMap::from([
            (PATH_ENV.to_owned(), env::join_paths([path_dir.as_path()])?),
            ("HOME".to_owned(), home.clone().into_os_string()),
        ]);
        let candidates = setup_link_dir_candidates(&|name| env.get(name).cloned());
        let dirs = usable_candidate_dirs(&candidates);

        restore_writable_dir(&path_dir)?;
        assert_eq!(dirs.first(), Some(&local_bin));
        assert!(!dirs.contains(&path_dir));
        Ok(())
    }

    fn usable_candidate_dirs(candidates: &[SetupLinkDirCandidate]) -> Vec<PathBuf> {
        candidates
            .iter()
            .filter(|candidate| candidate.is_usable())
            .map(|candidate| candidate.path().to_path_buf())
            .collect()
    }

    fn write_executable(dir: &Path, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        fs::create_dir_all(dir)?;
        let path = dir.join(name);
        let mut file = fs::File::create(&path)?;
        writeln!(file, "#!/bin/sh")?;
        make_executable(&path)?;
        Ok(path)
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
        Ok(())
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    #[cfg(unix)]
    fn restore_writable_dir(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
        Ok(())
    }
}
