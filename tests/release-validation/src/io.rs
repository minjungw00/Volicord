use std::{
    collections::BTreeMap,
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{
    de::{self, DeserializeOwned, MapAccess, SeqAccess, Visitor},
    Deserialize, Serialize,
};
use serde_json::{Number, Value};
use sha2::{Digest, Sha256};

use crate::error::{ValidationError, ValidationResult};

pub const MAX_CANDIDATE_JSON_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_CELL_JSON_BYTES: u64 = 1024 * 1024;
pub const MAX_MANIFEST_JSON_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_AUDIT_JSON_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_EVIDENCE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_VERSION_OUTPUT_BYTES: u64 = 16 * 1024;
const CANDIDATE_VERSION_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateBuildIdentity {
    pub build_id: String,
    pub package_version: String,
    pub git_commit: String,
    pub tree: String,
    pub metadata_source: String,
    pub target: String,
    pub profile: String,
    pub profile_class: String,
    pub profile_exact: String,
    pub opt: String,
    pub debug: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateArtifactInspection {
    pub sha256_before: String,
    pub private_copy_sha256: String,
    pub sha256_after_held: String,
    pub sha256_after_path: Option<String>,
    pub path_identity_stable: bool,
    pub build: CandidateBuildIdentity,
}

#[derive(Debug, Clone)]
pub struct ValidationContext {
    source_checkout: PathBuf,
    target_directory: PathBuf,
    docs_directory: PathBuf,
    runtime_homes: Vec<PathBuf>,
}

impl ValidationContext {
    pub fn new(
        source_checkout: PathBuf,
        target_directory: PathBuf,
        docs_directory: PathBuf,
        runtime_homes: Vec<PathBuf>,
    ) -> ValidationResult<Self> {
        let source_checkout = fs::canonicalize(&source_checkout).map_err(|error| {
            ValidationError::new(format!(
                "cannot canonicalize source checkout {}: {error}",
                source_checkout.display()
            ))
        })?;
        if !source_checkout.is_dir() {
            return Err(ValidationError::new(format!(
                "source checkout is not a directory: {}",
                source_checkout.display()
            )));
        }
        let target_directory = normalize_configured_root(&target_directory, &source_checkout)?;
        let docs_directory = normalize_configured_root(&docs_directory, &source_checkout)?;
        let mut runtime_homes = runtime_homes
            .into_iter()
            .map(|path| normalize_configured_root(&path, &source_checkout))
            .collect::<ValidationResult<Vec<_>>>()?;
        runtime_homes.sort();
        runtime_homes.dedup();
        Ok(Self {
            source_checkout,
            target_directory,
            docs_directory,
            runtime_homes,
        })
    }

    pub fn from_process(current_dir: &Path) -> ValidationResult<Self> {
        let source_checkout = git_toplevel(current_dir)?;
        let target_directory = cargo_target_directory(&source_checkout)?;
        let docs_directory = source_checkout.join("docs");
        let mut runtime_homes = Vec::new();
        if let Some(value) = env::var_os("VOLICORD_HOME").filter(|value| !value.is_empty()) {
            let path = PathBuf::from(value);
            runtime_homes.push(if path.is_absolute() {
                path
            } else {
                current_dir.join(path)
            });
        }
        if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
            if !home.is_empty() {
                let home = PathBuf::from(home);
                let home = if home.is_absolute() {
                    home
                } else {
                    current_dir.join(home)
                };
                runtime_homes.push(home.join(".volicord"));
            }
        }
        Self::new(
            source_checkout,
            target_directory,
            docs_directory,
            runtime_homes,
        )
    }

    pub fn source_checkout(&self) -> &Path {
        &self.source_checkout
    }

    pub fn validate_existing_file(&self, path: &Path) -> ValidationResult<()> {
        validate_absolute_normalized(path)?;
        self.validate_external(path, false)?;
        ensure_no_symlink_components(path)?;
        let canonical = fs::canonicalize(path).map_err(|error| {
            ValidationError::new(format!("cannot canonicalize {}: {error}", path.display()))
        })?;
        if canonical.as_os_str() != path.as_os_str() {
            return Err(ValidationError::new(format!(
                "path is not canonical and symlink-free: {}",
                path.display()
            )));
        }
        Ok(())
    }

    pub fn validate_existing_directory(&self, path: &Path) -> ValidationResult<()> {
        self.validate_existing_file(path)?;
        let metadata = fs::metadata(path)?;
        if !metadata.is_dir() {
            return Err(ValidationError::new(format!(
                "path is not a directory: {}",
                path.display()
            )));
        }
        self.validate_external(path, true)
    }

    pub fn validate_new_output(&self, path: &Path) -> ValidationResult<()> {
        validate_absolute_normalized(path)?;
        self.validate_external(path, false)?;
        let parent = path.parent().ok_or_else(|| {
            ValidationError::new(format!("output has no parent: {}", path.display()))
        })?;
        validate_absolute_normalized(parent)?;
        self.validate_external(parent, true)?;
        ensure_no_symlink_components(parent)?;
        let canonical_parent = fs::canonicalize(parent).map_err(|error| {
            ValidationError::new(format!(
                "cannot canonicalize output parent {}: {error}",
                parent.display()
            ))
        })?;
        if canonical_parent.as_os_str() != parent.as_os_str() {
            return Err(ValidationError::new(format!(
                "output parent is not canonical and symlink-free: {}",
                parent.display()
            )));
        }
        match fs::symlink_metadata(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Ok(_) => Err(ValidationError::new(format!(
                "output already exists: {}",
                path.display()
            ))),
            Err(error) => Err(ValidationError::new(format!(
                "cannot inspect output {}: {error}",
                path.display()
            ))),
        }
    }

    fn validate_external(&self, path: &Path, directory: bool) -> ValidationResult<()> {
        for (label, excluded) in [
            ("source checkout", &self.source_checkout),
            ("Cargo target directory", &self.target_directory),
            ("maintained documentation", &self.docs_directory),
        ] {
            if path.starts_with(excluded) || (directory && excluded.starts_with(path)) {
                return Err(ValidationError::new(format!(
                    "path {} overlaps {label} {}",
                    path.display(),
                    excluded.display()
                )));
            }
        }
        for runtime_home in &self.runtime_homes {
            if path.starts_with(runtime_home) || (directory && runtime_home.starts_with(path)) {
                return Err(ValidationError::new(format!(
                    "path {} overlaps Volicord Runtime Home {}",
                    path.display(),
                    runtime_home.display()
                )));
            }
        }
        for ancestor in path.ancestors() {
            let registry = ancestor.join("registry.sqlite");
            if registry.is_file() {
                return Err(ValidationError::new(format!(
                    "path {} is inside a directory containing a Volicord registry",
                    path.display()
                )));
            }
        }
        Ok(())
    }
}

pub fn read_strict_json<T: DeserializeOwned>(
    context: &ValidationContext,
    path: &Path,
    max_bytes: u64,
) -> ValidationResult<T> {
    let bytes = read_bounded_external_file(context, path, max_bytes)?;
    parse_strict_json(&bytes)
}

pub fn parse_strict_json<T: DeserializeOwned>(bytes: &[u8]) -> ValidationResult<T> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let unique = UniqueJsonValue::deserialize(&mut deserializer)
        .map_err(|error| ValidationError::new(format!("invalid strict JSON: {error}")))?;
    deserializer
        .end()
        .map_err(|error| ValidationError::new(format!("trailing JSON data: {error}")))?;
    serde_json::from_value(unique.0)
        .map_err(|error| ValidationError::new(format!("JSON schema mismatch: {error}")))
}

pub fn read_bounded_external_file(
    context: &ValidationContext,
    path: &Path,
    max_bytes: u64,
) -> ValidationResult<Vec<u8>> {
    context.validate_existing_file(path)?;
    let (mut file, metadata) = open_regular_file(path)?;
    if metadata.len() > max_bytes {
        return Err(ValidationError::new(format!(
            "file exceeds {max_bytes} byte bound: {}",
            path.display()
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::by_ref(&mut file)
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(ValidationError::new(format!(
            "file grew beyond {max_bytes} byte bound: {}",
            path.display()
        )));
    }
    Ok(bytes)
}

pub fn sha256_external_file(
    context: &ValidationContext,
    path: &Path,
    max_bytes: Option<u64>,
) -> ValidationResult<String> {
    context.validate_existing_file(path)?;
    let (mut file, metadata) = open_regular_file(path)?;
    if max_bytes.is_some_and(|bound| metadata.len() > bound) {
        return Err(ValidationError::new(format!(
            "file exceeds {} byte bound: {}",
            max_bytes.expect("bound was checked"),
            path.display()
        )));
    }
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or_else(|| ValidationError::new("file length overflow while hashing"))?;
        if max_bytes.is_some_and(|bound| total > bound) {
            return Err(ValidationError::new(format!(
                "file grew beyond {} byte bound: {}",
                max_bytes.expect("bound was checked"),
                path.display()
            )));
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(hasher.finalize().as_slice()))
}

pub fn inspect_candidate_artifact(
    context: &ValidationContext,
    candidate_path: &Path,
    expected_sha256: &str,
) -> ValidationResult<CandidateArtifactInspection> {
    context.validate_existing_file(candidate_path)?;
    let (mut held_candidate, initial_metadata) = open_regular_file(candidate_path)?;
    let sha256_before = sha256_file_handle(&mut held_candidate)?;
    if sha256_before != expected_sha256 {
        return Err(ValidationError::new(format!(
            "candidate digest differs from the descriptor before execution: {}",
            candidate_path.display()
        )));
    }

    held_candidate.seek(SeekFrom::Start(0))?;
    let private_directory = tempfile::Builder::new()
        .prefix("volicord-release-candidate-")
        .tempdir()
        .map_err(|error| {
            ValidationError::new(format!(
                "cannot create private candidate directory: {error}"
            ))
        })?;
    let private_candidate_path = private_directory.path().join("candidate");
    let mut private_candidate = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&private_candidate_path)
        .map_err(|error| {
            ValidationError::new(format!("cannot create private candidate copy: {error}"))
        })?;
    let private_copy_sha256 = copy_and_hash(&mut held_candidate, &mut private_candidate)?;
    if private_copy_sha256 != expected_sha256 {
        return Err(ValidationError::new(
            "candidate changed while copying from the held file handle",
        ));
    }
    private_candidate.sync_all()?;
    make_private_copy_executable(&private_candidate)?;
    drop(private_candidate);

    let mut command = Command::new(&private_candidate_path);
    command
        .arg("--version")
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = spawn_private_candidate(&mut command, candidate_path)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ValidationError::new("candidate version stdout is unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ValidationError::new("candidate version stderr is unavailable"))?;
    let stdout_reader = thread::spawn(move || read_pipe_bounded(stdout, MAX_VERSION_OUTPUT_BYTES));
    let stderr_reader = thread::spawn(move || read_pipe_bounded(stderr, MAX_VERSION_OUTPUT_BYTES));
    let deadline = Instant::now() + CANDIDATE_VERSION_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ValidationError::new(
                "candidate --version exceeded the 10 second bound",
            ));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| ValidationError::new("candidate stdout reader panicked"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| ValidationError::new("candidate stderr reader panicked"))??;
    if !status.success() {
        return Err(ValidationError::new(format!(
            "candidate --version failed with status {status}"
        )));
    }
    if !stderr.is_empty() {
        return Err(ValidationError::new(
            "candidate --version must not write stderr",
        ));
    }
    let stdout = std::str::from_utf8(&stdout)
        .map_err(|_| ValidationError::new("candidate --version output is not UTF-8"))?;
    let build = parse_candidate_version(stdout)?;
    let sha256_after_held = sha256_file_handle(&mut held_candidate)?;
    let (sha256_after_path, path_identity_stable) =
        inspect_final_candidate_path(context, candidate_path, &initial_metadata);
    Ok(CandidateArtifactInspection {
        sha256_before,
        private_copy_sha256,
        sha256_after_held,
        sha256_after_path,
        path_identity_stable,
        build,
    })
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_digest(hasher.finalize().as_slice())
}

pub fn write_json_create_new<T: Serialize>(
    context: &ValidationContext,
    path: &Path,
    value: &T,
    max_bytes: u64,
) -> ValidationResult<()> {
    context.validate_new_output(path)?;
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > max_bytes {
        return Err(ValidationError::new(format!(
            "serialized output exceeds {max_bytes} byte bound"
        )));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            ValidationError::new(format!(
                "cannot create new output {}: {error}",
                path.display()
            ))
        })?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn git_head(source_checkout: &Path) -> ValidationResult<String> {
    run_git_text(source_checkout, &["rev-parse", "HEAD"], 256)
}

pub fn git_is_clean(source_checkout: &Path) -> ValidationResult<bool> {
    Ok(run_git_text(
        source_checkout,
        &["status", "--porcelain=v1", "--untracked-files=all"],
        1024 * 1024,
    )?
    .is_empty())
}

pub fn git_archive_sha256(
    source_checkout: &Path,
    source_revision: &str,
) -> ValidationResult<String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(source_checkout)
        .args(["archive", "--format=tar", source_revision])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| ValidationError::new(format!("cannot start git archive: {error}")))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| ValidationError::new("git archive stdout is unavailable"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = stdout.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(ValidationError::new(format!(
            "git archive failed with status {status}"
        )));
    }
    Ok(hex_digest(hasher.finalize().as_slice()))
}

fn cargo_target_directory(source_checkout: &Path) -> ValidationResult<PathBuf> {
    if let Some(value) = env::var_os("CARGO_TARGET_DIR").filter(|value| !value.is_empty()) {
        let path = PathBuf::from(value);
        return Ok(if path.is_absolute() {
            path
        } else {
            source_checkout.join(path)
        });
    }
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--no-deps")
        .arg("--format-version=1")
        .current_dir(source_checkout)
        .output()
        .map_err(|error| ValidationError::new(format!("cannot run cargo metadata: {error}")))?;
    if !output.status.success() {
        return Err(ValidationError::new(format!(
            "cargo metadata failed with status {}",
            output.status
        )));
    }
    let value: Value = serde_json::from_slice(&output.stdout)?;
    let path = value["target_directory"]
        .as_str()
        .ok_or_else(|| ValidationError::new("cargo metadata omitted target_directory"))?;
    Ok(PathBuf::from(path))
}

fn git_toplevel(current_dir: &Path) -> ValidationResult<PathBuf> {
    let root = run_git_text(current_dir, &["rev-parse", "--show-toplevel"], 16 * 1024)?;
    let path = PathBuf::from(root);
    fs::canonicalize(&path).map_err(|error| {
        ValidationError::new(format!(
            "cannot canonicalize source checkout {}: {error}",
            path.display()
        ))
    })
}

fn run_git_text(root: &Path, args: &[&str], max_bytes: usize) -> ValidationResult<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| ValidationError::new(format!("cannot run git: {error}")))?;
    if !output.status.success() {
        return Err(ValidationError::new(format!(
            "git command failed with status {}",
            output.status
        )));
    }
    if output.stdout.len() > max_bytes {
        return Err(ValidationError::new("git command output exceeds bound"));
    }
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|_| ValidationError::new("git command output is not UTF-8"))?;
    Ok(text.trim_end_matches(['\r', '\n']).to_owned())
}

fn validate_absolute_normalized(path: &Path) -> ValidationResult<()> {
    if !path.is_absolute() {
        return Err(ValidationError::new(format!(
            "path must be absolute: {}",
            path.display()
        )));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(ValidationError::new(format!(
            "path must not contain dot components: {}",
            path.display()
        )));
    }
    let normalized = path.components().collect::<PathBuf>();
    if normalized.as_os_str() != path.as_os_str() {
        return Err(ValidationError::new(format!(
            "path must be lexically normalized: {}",
            path.display()
        )));
    }
    Ok(())
}

fn normalize_configured_root(path: &Path, relative_base: &Path) -> ValidationResult<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        relative_base.join(path)
    };
    let resolved = canonicalize_existing_prefix(&absolute)?;
    lexical_normalize_absolute(&resolved)
}

fn lexical_normalize_absolute(path: &Path) -> ValidationResult<PathBuf> {
    if !path.is_absolute() {
        return Err(ValidationError::new(format!(
            "configured exclusion root must resolve to an absolute path: {}",
            path.display()
        )));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
        }
    }
    if !normalized.is_absolute() {
        return Err(ValidationError::new(format!(
            "configured exclusion root did not remain absolute: {}",
            path.display()
        )));
    }
    Ok(normalized)
}

fn canonicalize_existing_prefix(path: &Path) -> ValidationResult<PathBuf> {
    let mut existing = path.to_path_buf();
    let mut suffix = Vec::<OsString>::new();
    loop {
        match fs::symlink_metadata(&existing) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = existing.file_name().ok_or_else(|| {
                    ValidationError::new(format!(
                        "cannot find an existing prefix for configured exclusion root {}",
                        path.display()
                    ))
                })?;
                suffix.push(name.to_os_string());
                existing = existing
                    .parent()
                    .ok_or_else(|| {
                        ValidationError::new(format!(
                            "configured exclusion root has no parent: {}",
                            path.display()
                        ))
                    })?
                    .to_path_buf();
            }
            Err(error) => {
                return Err(ValidationError::new(format!(
                    "cannot inspect configured exclusion root {}: {error}",
                    path.display()
                )))
            }
        }
    }
    let mut canonical = fs::canonicalize(&existing).map_err(|error| {
        ValidationError::new(format!(
            "cannot canonicalize configured exclusion prefix {}: {error}",
            existing.display()
        ))
    })?;
    for component in suffix.into_iter().rev() {
        canonical.push(component);
    }
    Ok(canonical)
}

fn ensure_no_symlink_components(path: &Path) -> ValidationResult<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current).map_err(|error| {
            ValidationError::new(format!(
                "cannot inspect path component {}: {error}",
                current.display()
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(ValidationError::new(format!(
                "symbolic links are not allowed in release-evidence paths: {}",
                current.display()
            )));
        }
    }
    Ok(())
}

fn open_regular_file(path: &Path) -> ValidationResult<(File, fs::Metadata)> {
    let before = fs::symlink_metadata(path)?;
    if before.file_type().is_symlink() || !before.is_file() {
        return Err(ValidationError::new(format!(
            "input is not a regular non-symlink file: {}",
            path.display()
        )));
    }
    let file = File::open(path)?;
    let after = file.metadata()?;
    if !after.is_file() || !same_file(&before, &after) {
        return Err(ValidationError::new(format!(
            "input changed while opening: {}",
            path.display()
        )));
    }
    Ok((file, after))
}

fn sha256_file_handle(file: &mut File) -> ValidationResult<String> {
    file.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(hasher.finalize().as_slice()))
}

fn copy_and_hash(source: &mut File, destination: &mut File) -> ValidationResult<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        destination.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
    }
    destination.flush()?;
    Ok(hex_digest(hasher.finalize().as_slice()))
}

#[cfg(unix)]
fn make_private_copy_executable(file: &File) -> ValidationResult<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = file.metadata()?.permissions();
    permissions.set_mode(0o500);
    file.set_permissions(permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_private_copy_executable(_: &File) -> ValidationResult<()> {
    Ok(())
}

fn inspect_final_candidate_path(
    context: &ValidationContext,
    candidate_path: &Path,
    initial_metadata: &fs::Metadata,
) -> (Option<String>, bool) {
    let inspection = (|| -> ValidationResult<(String, bool)> {
        context.validate_existing_file(candidate_path)?;
        let (mut final_file, final_metadata) = open_regular_file(candidate_path)?;
        let digest = sha256_file_handle(&mut final_file)?;
        Ok((digest, same_file(initial_metadata, &final_metadata)))
    })();
    match inspection {
        Ok((digest, identity_stable)) => (Some(digest), identity_stable),
        Err(_) => (None, false),
    }
}

fn read_pipe_bounded(mut pipe: impl Read, max_bytes: u64) -> ValidationResult<Vec<u8>> {
    let mut bytes = Vec::new();
    pipe.by_ref().take(max_bytes + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(ValidationError::new(
            "candidate --version output exceeds bound",
        ));
    }
    Ok(bytes)
}

fn spawn_private_candidate(
    command: &mut Command,
    descriptor_path: &Path,
) -> ValidationResult<std::process::Child> {
    for attempt in 0..20 {
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(error) if is_text_file_busy(&error) && attempt < 19 => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => {
                return Err(ValidationError::new(format!(
                    "cannot execute the private candidate copy for {} --version: {error}",
                    descriptor_path.display(),
                )))
            }
        }
    }
    unreachable!("the bounded candidate spawn loop always returns")
}

#[cfg(unix)]
fn is_text_file_busy(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(26)
}

#[cfg(not(unix))]
fn is_text_file_busy(_: &std::io::Error) -> bool {
    false
}

fn parse_candidate_version(output: &str) -> ValidationResult<CandidateBuildIdentity> {
    let line = output
        .strip_suffix('\n')
        .ok_or_else(|| ValidationError::new("candidate --version must end with one LF"))?;
    if line.contains(['\r', '\n']) {
        return Err(ValidationError::new(
            "candidate --version must contain exactly one line",
        ));
    }
    let body = line
        .strip_prefix("volicord ")
        .ok_or_else(|| ValidationError::new("candidate version prefix mismatch"))?;
    let (package_version, build_id) = body
        .split_once(" (build_id=")
        .ok_or_else(|| ValidationError::new("candidate version build_id wrapper mismatch"))?;
    let build_id = build_id
        .strip_suffix(')')
        .ok_or_else(|| ValidationError::new("candidate version closing wrapper mismatch"))?;
    validate_version_component("package_version", package_version)?;
    let mut components = build_id.split(';');
    let embedded_package = components
        .next()
        .ok_or_else(|| ValidationError::new("build_id package version is missing"))?;
    if embedded_package != package_version {
        return Err(ValidationError::new(
            "outer and build_id package versions differ",
        ));
    }
    let names = [
        "git",
        "tree",
        "metadata_source",
        "target",
        "profile",
        "profile_class",
        "profile_exact",
        "opt",
        "debug",
    ];
    let mut values = BTreeMap::new();
    for name in names {
        let component = components
            .next()
            .ok_or_else(|| ValidationError::new(format!("build_id {name} is missing")))?;
        let (actual_name, value) = component
            .split_once('=')
            .ok_or_else(|| ValidationError::new(format!("build_id {name} is malformed")))?;
        if actual_name != name {
            return Err(ValidationError::new(format!(
                "build_id expected {name}, found {actual_name}"
            )));
        }
        validate_version_component(name, value)?;
        values.insert(name, value.to_owned());
    }
    if components.next().is_some() {
        return Err(ValidationError::new(
            "build_id contains an additional component",
        ));
    }
    Ok(CandidateBuildIdentity {
        build_id: build_id.to_owned(),
        package_version: package_version.to_owned(),
        git_commit: values.remove("git").expect("validated component"),
        tree: values.remove("tree").expect("validated component"),
        metadata_source: values
            .remove("metadata_source")
            .expect("validated component"),
        target: values.remove("target").expect("validated component"),
        profile: values.remove("profile").expect("validated component"),
        profile_class: values.remove("profile_class").expect("validated component"),
        profile_exact: values.remove("profile_exact").expect("validated component"),
        opt: values.remove("opt").expect("validated component"),
        debug: values.remove("debug").expect("validated component"),
    })
}

fn validate_version_component(field: &str, value: &str) -> ValidationResult<()> {
    if value.is_empty()
        || value.len() > 512
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b';' | b'(' | b')'))
    {
        return Err(ValidationError::new(format!(
            "candidate build {field} is empty, oversized, or malformed"
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.len() == right.len()
        && left.file_type().is_file() == right.file_type().is_file()
        && left.modified().ok() == right.modified().ok()
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

struct UniqueJsonValue(Value);

impl<'de> Deserialize<'de> for UniqueJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueJsonVisitor).map(Self)
    }
}

struct UniqueJsonVisitor;

impl<'de> Visitor<'de> for UniqueJsonVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value without duplicate object members")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        UniqueJsonValue::deserialize(deserializer).map(|value| value.0)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<UniqueJsonValue>()? {
            values.push(value.0);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object member: {key}"
                )));
            }
            let value = map.next_value::<UniqueJsonValue>()?;
            values.insert(key, value.0);
        }
        Ok(Value::Object(values.into_iter().collect()))
    }
}
