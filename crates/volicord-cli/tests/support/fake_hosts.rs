use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use super::binary_fixture::volicord_bin;

#[cfg(unix)]
pub(crate) fn path_env(path_dirs: &[&Path]) -> String {
    std::env::join_paths(path_dirs)
        .expect("test PATH should be valid")
        .to_string_lossy()
        .into_owned()
}

#[cfg(unix)]
pub(crate) fn hook_execution_path_env(fake_bin_dir: &Path) -> Result<String, Box<dyn Error>> {
    let volicord_dir = Path::new(volicord_bin())
        .parent()
        .ok_or("volicord test binary path should have a parent")?;
    path_env_with_existing(&[volicord_dir, fake_bin_dir])
}

#[cfg(unix)]
pub(crate) fn path_env_with_existing(path_dirs: &[&Path]) -> Result<String, Box<dyn Error>> {
    let mut paths = path_dirs
        .iter()
        .map(|path| (*path).to_path_buf())
        .collect::<Vec<_>>();
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    Ok(std::env::join_paths(paths)?.to_string_lossy().into_owned())
}

#[cfg(unix)]
pub(crate) fn write_fake_codex(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    write_fake_codex_with_version(dir, "1.2.3-test")
}

pub(crate) fn write_fake_codex_with_version(
    dir: &Path,
    version: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join("codex");
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'codex-cli {version}\\n'; exit 0; fi\nprintf 'unexpected codex invocation\\n' >&2\nexit 2\n"
        ),
    )?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
pub(crate) fn write_fake_claude_code(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join("claude");
    let state_path = path.with_extension("state");
    let state_text = state_path.display().to_string().replace('\'', "'\\''");
    let mut script = format!("#!/bin/sh\nstate='{state_text}'\n");
    script.push_str(
        "if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"get\" ]; then\n\
         if [ -f \"$state\" ]; then cat \"$state\"; exit 0; fi\n\
         printf 'Server not found\\n' >&2\n\
         exit 1\n\
         fi\n\
         if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"add\" ]; then\n\
         shift 2\n\
         scope=\"\"\n\
         env_line=\"\"\n\
         command=\"\"\n\
         args=\"\"\n\
         while [ \"$#\" -gt 0 ]; do\n\
         case \"$1\" in\n\
         --env) env_line=\"$2\"; shift 2 ;;\n\
         --transport) shift 2 ;;\n\
         --scope) scope=\"$2\"; shift 2 ;;\n\
         --) shift; command=\"$1\"; shift; args=\"$*\"; break ;;\n\
         *) shift ;;\n\
         esac\n\
         done\n\
         {\n\
         printf 'Status: Connected\\n'\n\
         printf 'Scope: %s\\n' \"$scope\"\n\
         printf 'Command: %s\\n' \"$command\"\n\
         printf 'Args: %s\\n' \"$args\"\n\
         if [ -n \"$env_line\" ]; then printf 'Environment:\\n  %s\\n' \"$env_line\"; fi\n\
         } > \"$state\"\n\
         exit 0\n\
         fi\n\
         if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"remove\" ]; then\n\
         /bin/rm -f \"$state\"\n\
         exit 0\n\
         fi\n\
         printf 'unexpected claude invocation\\n' >&2\n\
         exit 2\n",
    );
    fs::write(&path, script)?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
pub(crate) fn shell_single_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

#[cfg(unix)]
pub(crate) fn make_executable(path: &Path) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(unix)]
pub(crate) fn is_executable(path: &Path) -> Result<bool, Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    Ok(fs::metadata(path)?.permissions().mode() & 0o111 != 0)
}
