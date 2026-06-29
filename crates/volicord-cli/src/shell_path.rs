use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
};

pub(crate) const PATH_ENV: &str = "PATH";

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

pub(crate) fn path_directory_is_writable(path: &Path) -> bool {
    let probe = if path.exists() {
        path
    } else {
        match path.parent() {
            Some(parent) => parent,
            None => return false,
        }
    };
    fs::metadata(probe)
        .map(|metadata| metadata.is_dir() && !metadata.permissions().readonly())
        .unwrap_or(false)
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

pub(crate) fn candidate_setup_link_dirs<F>(env_var: &F) -> Vec<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    let mut dirs = Vec::new();
    if let Some(path_env) = env_var(PATH_ENV) {
        for dir in env::split_paths(&path_env) {
            if path_directory_is_writable(&dir) {
                push_unique_path(&mut dirs, dir);
            }
        }
    }
    for dir in candidate_user_bin_dirs(env_var) {
        push_unique_path(&mut dirs, dir);
    }
    dirs
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths
        .iter()
        .any(|existing| paths_equivalent(existing, &path))
    {
        paths.push(path);
    }
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
    format!("volicord-mcp{}", env::consts::EXE_SUFFIX)
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs, io::Write, path::Path};

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
        fs::create_dir_all(&path_dir)?;
        fs::create_dir_all(&home)?;
        let env = BTreeMap::from([
            (PATH_ENV.to_owned(), env::join_paths([path_dir.as_path()])?),
            ("HOME".to_owned(), home.clone().into_os_string()),
        ]);

        let dirs = candidate_setup_link_dirs(&|name| env.get(name).cloned());

        assert_eq!(dirs.first(), Some(&path_dir));
        assert!(dirs.contains(&home.join(".local").join("bin")));
        Ok(())
    }

    #[test]
    fn candidate_setup_link_dirs_uses_user_bin_when_path_has_no_writable_directory(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = TempRuntimeHome::new("shell-path-user-bin")?;
        let path_file = fixture.path().join("path-file");
        let home = fixture.path().join("home");
        fs::write(&path_file, "not a directory")?;
        fs::create_dir_all(&home)?;
        let env = BTreeMap::from([
            (PATH_ENV.to_owned(), env::join_paths([path_file.as_path()])?),
            ("HOME".to_owned(), home.clone().into_os_string()),
        ]);

        let dirs = candidate_setup_link_dirs(&|name| env.get(name).cloned());

        assert_eq!(dirs.first(), Some(&home.join(".local").join("bin")));
        assert!(dirs.contains(&home.join("bin")));
        assert!(!dirs.contains(&path_file));
        Ok(())
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
}
