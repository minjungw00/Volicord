mod build_support;

use std::{
    collections::BTreeSet,
    env,
    ffi::{OsStr, OsString},
    path::{Component, Path, PathBuf},
    process::{Command, Output},
};

use build_support::{normalized_git_commit, parse_explicit_git_metadata, parse_explicit_profile};

const UNKNOWN: &str = "unknown";
const GIT_REPOSITORY_ENV: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CEILING_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_CONFIG_COUNT",
    "GIT_CONFIG_PARAMETERS",
    "GIT_DIR",
    "GIT_DISCOVERY_ACROSS_FILESYSTEM",
    "GIT_INDEX_FILE",
    "GIT_NAMESPACE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_PREFIX",
    "GIT_QUARANTINE_PATH",
    "GIT_SHALLOW_FILE",
    "GIT_SUPER_PREFIX",
    "GIT_WORK_TREE",
];

fn main() {
    if let Err(error) = run() {
        panic!("invalid Volicord build identity configuration: {error}");
    }
}

fn run() -> Result<(), String> {
    println!("cargo:rerun-if-env-changed=VOLICORD_BUILD_GIT_COMMIT");
    println!("cargo:rerun-if-env-changed=VOLICORD_BUILD_GIT_DIRTY");
    println!("cargo:rerun-if-env-changed=VOLICORD_BUILD_PROFILE");

    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "CARGO_MANIFEST_DIR is not set".to_owned())?;
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "volicord-mcp is not inside the workspace crates directory".to_owned())?;
    emit_base_rerun_paths(workspace_root);

    let explicit_git = parse_explicit_git_metadata(
        env::var_os("VOLICORD_BUILD_GIT_COMMIT"),
        env::var_os("VOLICORD_BUILD_GIT_DIRTY"),
    )?;
    let (git_commit, git_dirty, metadata_source) = match explicit_git {
        Some(metadata) => (metadata.commit, metadata.dirty.to_string(), "environment"),
        None => match repository_metadata(workspace_root) {
            Some((commit, dirty)) => {
                emit_repository_rerun_paths(workspace_root);
                (commit, dirty.to_string(), "repository")
            }
            None => (UNKNOWN.to_owned(), UNKNOWN.to_owned(), UNKNOWN),
        },
    };

    let explicit_profile = parse_explicit_profile(env::var_os("VOLICORD_BUILD_PROFILE"))?;
    let profile_class = cargo_ascii_value("PROFILE", &["debug", "release"]);
    let build_profile = explicit_profile.unwrap_or_else(|| UNKNOWN.to_owned());
    let opt_level = cargo_ascii_value("OPT_LEVEL", &["0", "1", "2", "3", "s", "z"]);
    let debug = cargo_ascii_value("DEBUG", &["true", "false"]);
    let target = cargo_nonempty_ascii_value("TARGET");

    println!("cargo:rustc-env=VOLICORD_BUILD_GIT_COMMIT={git_commit}");
    println!("cargo:rustc-env=VOLICORD_BUILD_GIT_DIRTY={git_dirty}");
    println!("cargo:rustc-env=VOLICORD_BUILD_METADATA_SOURCE={metadata_source}");
    println!("cargo:rustc-env=VOLICORD_BUILD_TARGET={target}");
    println!("cargo:rustc-env=VOLICORD_BUILD_PROFILE={build_profile}");
    println!("cargo:rustc-env=VOLICORD_BUILD_PROFILE_CLASS={profile_class}");
    println!("cargo:rustc-env=VOLICORD_BUILD_OPT_LEVEL={opt_level}");
    println!("cargo:rustc-env=VOLICORD_BUILD_DEBUG={debug}");
    Ok(())
}

fn emit_base_rerun_paths(workspace_root: &Path) {
    for path in [
        workspace_root.join("Cargo.toml"),
        workspace_root.join("Cargo.lock"),
        workspace_root.join("crates"),
    ] {
        emit_rerun_path(&path);
    }
}

fn repository_metadata(workspace_root: &Path) -> Option<(String, bool)> {
    if !is_exact_worktree_root(workspace_root) {
        return None;
    }
    let commit = git_text_output(workspace_root, ["rev-parse", "HEAD"])
        .and_then(|value| normalized_git_commit(&value))?;
    let status = git_output(
        workspace_root,
        ["status", "--porcelain=v1", "--untracked-files=normal"],
    )?;
    status
        .status
        .success()
        .then_some((commit, !status.stdout.is_empty()))
}

fn is_exact_worktree_root(workspace_root: &Path) -> bool {
    let Some(top_level) = git_path_output(workspace_root, ["rev-parse", "--show-toplevel"]) else {
        return false;
    };
    match (workspace_root.canonicalize(), top_level.canonicalize()) {
        (Ok(workspace_root), Ok(top_level)) => workspace_root == top_level,
        _ => false,
    }
}

fn emit_repository_rerun_paths(workspace_root: &Path) {
    let mut paths = BTreeSet::new();
    for git_path in ["HEAD", "index", "packed-refs"] {
        if let Some(path) = resolved_git_path(workspace_root, OsStr::new(git_path)) {
            paths.insert(path);
        }
    }

    if let Some(path) = git_os_output(workspace_root, ["symbolic-ref", "-q", "HEAD"])
        .and_then(|symbolic_ref| resolved_git_path(workspace_root, &symbolic_ref))
    {
        paths.insert(path);
    }

    if let Some(output) = git_output(
        workspace_root,
        [
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )
    .filter(|output| output.status.success())
    {
        for bytes in output.stdout.split(|byte| *byte == 0) {
            if bytes.is_empty() {
                continue;
            }
            let relative = path_from_git_bytes(bytes);
            if is_safe_repository_relative_path(&relative) {
                paths.insert(workspace_root.join(relative));
            }
        }
    }

    for path in paths {
        emit_rerun_path(&path);
    }
}

fn resolved_git_path(workspace_root: &Path, git_path: &OsStr) -> Option<PathBuf> {
    let args = [
        OsString::from("rev-parse"),
        OsString::from("--git-path"),
        git_path.to_owned(),
    ];
    let path = git_path_output(workspace_root, args)?;
    Some(if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
    })
}

fn is_safe_repository_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn emit_rerun_path(path: &Path) {
    let display = path.to_string_lossy();
    if !display.contains(['\n', '\r']) {
        println!("cargo:rerun-if-changed={display}");
    }
}

fn git_text_output<I, S>(workspace_root: &Path, args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = git_output(workspace_root, args)?;
    if !output.status.success() {
        return None;
    }
    let bytes = trim_git_line_end(&output.stdout);
    String::from_utf8(bytes.to_vec()).ok()
}

fn git_os_output<I, S>(workspace_root: &Path, args: I) -> Option<OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = git_output(workspace_root, args)?;
    output
        .status
        .success()
        .then(|| os_string_from_git_bytes(trim_git_line_end(&output.stdout)))
}

fn git_path_output<I, S>(workspace_root: &Path, args: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    git_os_output(workspace_root, args).map(PathBuf::from)
}

fn git_output<I, S>(workspace_root: &Path, args: I) -> Option<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("git");
    command.arg("-C").arg(workspace_root).args(args);
    for name in GIT_REPOSITORY_ENV {
        command.env_remove(name);
    }
    for (name, _) in env::vars_os() {
        let name_text = name.to_string_lossy();
        if name_text.starts_with("GIT_CONFIG_KEY_") || name_text.starts_with("GIT_CONFIG_VALUE_") {
            command.env_remove(name);
        }
    }
    command.env("GIT_OPTIONAL_LOCKS", "0");
    command.output().ok()
}

fn trim_git_line_end(mut bytes: &[u8]) -> &[u8] {
    while matches!(bytes.last(), Some(b'\n' | b'\r')) {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}

#[cfg(unix)]
fn os_string_from_git_bytes(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;

    OsString::from_vec(bytes.to_vec())
}

#[cfg(not(unix))]
fn os_string_from_git_bytes(bytes: &[u8]) -> OsString {
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}

fn path_from_git_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(os_string_from_git_bytes(bytes))
}

fn cargo_ascii_value(name: &str, allowed: &[&str]) -> String {
    env::var_os(name)
        .and_then(|value| value.into_string().ok())
        .filter(|value| allowed.contains(&value.as_str()))
        .unwrap_or_else(|| UNKNOWN.to_owned())
}

fn cargo_nonempty_ascii_value(name: &str) -> String {
    env::var_os(name)
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.is_empty() && value.is_ascii())
        .unwrap_or_else(|| UNKNOWN.to_owned())
}
