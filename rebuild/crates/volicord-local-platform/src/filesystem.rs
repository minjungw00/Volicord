use std::{
    collections::HashSet,
    ffi::OsStr,
    fs::{self, File},
    io::{self, Read},
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const CONTROL_FILE_LIMIT: u64 = 4096;
const GIT_CONFIG_LIMIT: u64 = 256 * 1024;
const REPOSITORY_NAME_HINT_LIMIT: usize = 255;
const LOCAL_ORIGIN_LINEAGE_HOP_LIMIT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPathError(String);

impl RepositoryPathError {
    pub fn detail(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RepositoryPathError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RepositoryPathError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryPathState {
    Existing,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRepositoryPath {
    relative: PathBuf,
    state: RepositoryPathState,
    traversed_symlink: bool,
}

impl ResolvedRepositoryPath {
    pub fn relative(&self) -> &Path {
        &self.relative
    }

    pub const fn state(&self) -> RepositoryPathState {
        self.state
    }

    pub const fn traversed_symlink(&self) -> bool {
        self.traversed_symlink
    }
}

#[derive(Debug, Clone)]
pub struct RepositoryRoot {
    canonical: PathBuf,
}

impl RepositoryRoot {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, RepositoryPathError> {
        let canonical = fs::canonicalize(path.as_ref()).map_err(|error| {
            RepositoryPathError(format!("cannot canonicalize repository root: {error}"))
        })?;
        if !fs::metadata(&canonical).map_err(path_error)?.is_dir() {
            return Err(RepositoryPathError(
                "repository root is not a directory".to_owned(),
            ));
        }
        Ok(Self { canonical })
    }

    pub fn canonical_path(&self) -> &Path {
        &self.canonical
    }

    pub fn resolve(
        &self,
        relative: impl AsRef<Path>,
    ) -> Result<ResolvedRepositoryPath, RepositoryPathError> {
        let relative = normalize_relative(relative.as_ref())?;
        let candidate = self.canonical.join(&relative);
        let mut current = self.canonical.clone();
        let mut traversed_symlink = false;
        for component in relative.components() {
            if let Component::Normal(value) = component {
                current.push(value);
                match fs::symlink_metadata(&current) {
                    Ok(metadata) if metadata.file_type().is_symlink() => traversed_symlink = true,
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => break,
                    Err(error) => return Err(path_error(error)),
                }
            }
        }
        let (existing, state) = nearest_existing(&candidate, &self.canonical)?;
        let resolved = fs::canonicalize(existing).map_err(path_error)?;
        if !resolved.starts_with(&self.canonical) {
            return Err(RepositoryPathError(
                "repository path resolves outside its canonical root".to_owned(),
            ));
        }
        Ok(ResolvedRepositoryPath {
            relative,
            state,
            traversed_symlink,
        })
    }
}

fn normalize_relative(path: &Path) -> Result<PathBuf, RepositoryPathError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(RepositoryPathError(
            "repository path must be a non-empty relative path".to_owned(),
        ));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => normalized.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(RepositoryPathError(
                    "repository path may not contain a parent or root component".to_owned(),
                ));
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(RepositoryPathError(
            "repository path resolves to an empty path".to_owned(),
        ));
    }
    Ok(normalized)
}

fn nearest_existing(
    candidate: &Path,
    root: &Path,
) -> Result<(PathBuf, RepositoryPathState), RepositoryPathError> {
    let mut current = candidate.to_owned();
    let mut missing = false;
    loop {
        match fs::symlink_metadata(&current) {
            Ok(_) => {
                return Ok((
                    current,
                    if missing {
                        RepositoryPathState::Missing
                    } else {
                        RepositoryPathState::Existing
                    },
                ))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                missing = true;
                if current == root || !current.pop() {
                    return Err(RepositoryPathError(
                        "repository root disappeared during observation".to_owned(),
                    ));
                }
            }
            Err(error) => return Err(path_error(error)),
        }
    }
}

fn path_error(error: io::Error) -> RepositoryPathError {
    RepositoryPathError(format!("repository path observation failed: {error}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFingerprint(String);

impl SourceFingerprint {
    pub fn observe(path: impl AsRef<Path>) -> Result<Self, SourceFingerprintError> {
        fingerprint(path.as_ref())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub struct SourceFingerprintError(String);

impl std::fmt::Display for SourceFingerprintError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SourceFingerprintError {}

fn fingerprint(path: &Path) -> Result<SourceFingerprint, SourceFingerprintError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(SourceFingerprint(hash_fields(&[b"absent"])))
        }
        Err(error) => {
            return Err(SourceFingerprintError(format!(
                "cannot inspect source: {error}"
            )))
        }
    };
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(path).map_err(|error| {
            SourceFingerprintError(format!("cannot read symlink target: {error}"))
        })?;
        return Ok(SourceFingerprint(hash_fields(&[
            b"symlink",
            os_bytes(target.as_os_str()),
        ])));
    }
    if metadata.is_file() {
        let bytes = fs::read(path)
            .map_err(|error| SourceFingerprintError(format!("cannot read source file: {error}")))?;
        #[cfg(unix)]
        let mode = metadata.permissions().mode().to_be_bytes();
        #[cfg(not(unix))]
        let mode = [0_u8; 4];
        return Ok(SourceFingerprint(hash_fields(&[b"regular", &mode, &bytes])));
    }
    Err(SourceFingerprintError(
        "source is neither a regular file nor a symbolic link".to_owned(),
    ))
}

#[cfg(unix)]
fn os_bytes(value: &OsStr) -> &[u8] {
    value.as_bytes()
}

#[cfg(not(unix))]
fn os_bytes(value: &OsStr) -> &[u8] {
    value.to_string_lossy().as_bytes()
}

fn hash_fields(fields: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    for field in fields {
        digest.update((field.len() as u64).to_be_bytes());
        digest.update(field);
    }
    format!("sha256:{:x}", digest.finalize())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeLayout {
    repository_root: PathBuf,
    git_dir: PathBuf,
    common_dir: PathBuf,
    linked_worktree: bool,
}

impl GitWorktreeLayout {
    pub fn resolve(repository_root: impl AsRef<Path>) -> Result<Option<Self>, RepositoryPathError> {
        resolve_git_layout(repository_root.as_ref())
    }

    pub fn coordinate(&self) -> LocalRepositoryCoordinate {
        LocalRepositoryCoordinate {
            clone_identity: hash_fields(&[b"local_clone", os_bytes(self.common_dir.as_os_str())]),
            worktree_identity: hash_fields(&[
                b"local_worktree",
                os_bytes(self.git_dir.as_os_str()),
            ]),
        }
    }

    pub fn repository_root(&self) -> &Path {
        &self.repository_root
    }
    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }
    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }
    pub const fn is_linked_worktree(&self) -> bool {
        self.linked_worktree
    }

    /// Returns a bounded display-name hint from the local `origin` lineage when
    /// it is present and unambiguous. This is not a Project or clone identity.
    pub fn repository_name_hint(&self) -> Option<String> {
        let origin = self.origin_url()?;
        let immediate_hint = repository_slug(&origin);
        let mut current_origin = origin;
        let mut visited = HashSet::from([self.common_dir.clone()]);

        for _ in 0..LOCAL_ORIGIN_LINEAGE_HOP_LIMIT {
            let Some(local_repository) = local_origin_repository_path(&current_origin) else {
                return repository_slug(&current_origin).or(immediate_hint);
            };
            let Some(layout) = Self::resolve(local_repository).ok().flatten() else {
                return immediate_hint;
            };
            if !visited.insert(layout.common_dir.clone()) {
                return immediate_hint;
            }
            let Some(next_origin) = layout.origin_url() else {
                return immediate_hint;
            };
            if local_origin_repository_path(&next_origin).is_none() {
                return repository_slug(&next_origin).or(immediate_hint);
            }
            current_origin = next_origin;
        }

        immediate_hint
    }

    fn origin_url(&self) -> Option<String> {
        let config =
            read_bounded_regular_file(&self.common_dir.join("config"), GIT_CONFIG_LIMIT).ok()?;
        let config = std::str::from_utf8(&config).ok()?;
        unambiguous_origin_url(config).map(str::to_owned)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRepositoryCoordinate {
    clone_identity: String,
    worktree_identity: String,
}

impl LocalRepositoryCoordinate {
    pub fn clone_identity(&self) -> &str {
        &self.clone_identity
    }
    pub fn worktree_identity(&self) -> &str {
        &self.worktree_identity
    }
}

fn resolve_git_layout(root: &Path) -> Result<Option<GitWorktreeLayout>, RepositoryPathError> {
    let repository_root = fs::canonicalize(root).map_err(path_error)?;
    let marker = repository_root.join(".git");
    let metadata = match fs::symlink_metadata(&marker) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(path_error(error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(RepositoryPathError(
            ".git marker may not be a symbolic link".to_owned(),
        ));
    }
    let git_dir = if metadata.is_dir() {
        fs::canonicalize(&marker).map_err(path_error)?
    } else if metadata.is_file() {
        let line = read_control_file(&marker)?;
        let value = line.strip_prefix("gitdir: ").ok_or_else(|| {
            RepositoryPathError(".git file must contain one gitdir declaration".to_owned())
        })?;
        fs::canonicalize(repository_root.join(value)).map_err(path_error)?
    } else {
        return Err(RepositoryPathError(
            ".git marker is not a file or directory".to_owned(),
        ));
    };
    let common_control = git_dir.join("commondir");
    let (common_dir, linked_worktree) = match fs::symlink_metadata(&common_control) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(RepositoryPathError(
                "commondir control must be a regular file".to_owned(),
            ));
        }
        Ok(_) => {
            let line = read_control_file(&common_control)?;
            (
                fs::canonicalize(git_dir.join(line)).map_err(path_error)?,
                true,
            )
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => (git_dir.clone(), false),
        Err(error) => return Err(path_error(error)),
    };
    Ok(Some(GitWorktreeLayout {
        repository_root,
        git_dir,
        common_dir,
        linked_worktree,
    }))
}

fn read_control_file(path: &Path) -> Result<String, RepositoryPathError> {
    let metadata = fs::symlink_metadata(path).map_err(path_error)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > CONTROL_FILE_LIMIT
    {
        return Err(RepositoryPathError(
            "Git control path must be a bounded regular file".to_owned(),
        ));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(path_error)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| RepositoryPathError("Git control file is not UTF-8".to_owned()))?;
    let line = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);
    if line.is_empty() || line.contains(['\n', '\r', '\0']) {
        return Err(RepositoryPathError(
            "Git control file must contain one non-empty line".to_owned(),
        ));
    }
    Ok(line.to_owned())
}

fn read_bounded_regular_file(path: &Path, limit: u64) -> Result<Vec<u8>, RepositoryPathError> {
    let metadata = fs::symlink_metadata(path).map_err(path_error)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > limit {
        return Err(RepositoryPathError(
            "Git metadata path must be a bounded regular file".to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(path_error)?;
    Ok(bytes)
}

fn unambiguous_origin_url(config: &str) -> Option<&str> {
    let mut in_origin = false;
    let mut origin = None;
    for raw_line in config.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        if line.starts_with('[') {
            let section = line.strip_prefix('[')?.strip_suffix(']')?.trim();
            let mut parts = section.splitn(2, char::is_whitespace);
            let kind = parts.next().unwrap_or_default();
            let subsection = parts.next();
            in_origin = kind.eq_ignore_ascii_case("remote")
                && subsection
                    .map(str::trim)
                    .unwrap_or_default()
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
                    == Some("origin");
            continue;
        }
        if !in_origin {
            continue;
        }
        let (key, value) = line.split_once('=')?;
        if !key.trim().eq_ignore_ascii_case("url") {
            continue;
        }
        let value = value.trim();
        let value = if let Some(quoted) = value.strip_prefix('"') {
            quoted.strip_suffix('"')?
        } else {
            value
        };
        if value.is_empty() || origin.replace(value).is_some() {
            return None;
        }
    }
    origin
}

fn repository_slug(origin: &str) -> Option<String> {
    let origin = origin.trim();
    if origin.is_empty()
        || origin.len() > GIT_CONFIG_LIMIT as usize
        || origin.contains(['\0', '\n', '\r', '?', '#', '\\'])
    {
        return None;
    }
    let origin = origin.trim_end_matches('/');
    let candidate = origin.rsplit(['/', ':']).next()?;
    let candidate = candidate.strip_suffix(".git").unwrap_or(candidate);
    if candidate.is_empty()
        || candidate == "."
        || candidate == ".."
        || candidate.len() > REPOSITORY_NAME_HINT_LIMIT
        || !candidate
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return None;
    }
    Some(candidate.to_owned())
}

fn local_origin_repository_path(origin: &str) -> Option<PathBuf> {
    if origin.contains(['\0', '\n', '\r', '?', '#', '\\']) {
        return None;
    }
    if let Some(path) = origin.strip_prefix("file://") {
        if !path.starts_with('/') || path.contains('%') {
            return None;
        }
        return Some(PathBuf::from(path));
    }
    let path = Path::new(origin);
    path.is_absolute().then(|| path.to_path_buf())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyObservation {
    dirty: bool,
    fingerprint: String,
    dirty_paths: Vec<String>,
}

impl DirtyObservation {
    pub fn from_porcelain_v2(bytes: &[u8]) -> Result<Self, RepositoryPathError> {
        let mut records = bytes.split(|byte| *byte == 0).peekable();
        let mut dirty_paths = Vec::new();
        while let Some(record) = records.next() {
            if record.is_empty() {
                if records.peek().is_some() {
                    return Err(RepositoryPathError(
                        "Git porcelain v2 contains an empty record".to_owned(),
                    ));
                }
                break;
            }
            match record.first().copied() {
                Some(b'1') => {
                    dirty_paths.push(porcelain_path_field(record, 8)?);
                }
                Some(b'2') => {
                    let destination = porcelain_path_field(record, 9)?;
                    let score = porcelain_field(record, 8, 9)?;
                    let origin = records.next().ok_or_else(|| {
                        RepositoryPathError(
                            "Git porcelain v2 rename/copy record has no origin path".to_owned(),
                        )
                    })?;
                    let origin = normalize_porcelain_path(origin)?;
                    dirty_paths.push(destination);
                    if score.first() == Some(&b'R') {
                        dirty_paths.push(origin);
                    }
                }
                Some(b'u') => {
                    dirty_paths.push(porcelain_path_field(record, 10)?);
                }
                Some(b'?') if record.get(1) == Some(&b' ') => {
                    dirty_paths.push(normalize_porcelain_path(&record[2..])?);
                }
                Some(b'!') if record.get(1) == Some(&b' ') => {}
                Some(b'#') if record.get(1) == Some(&b' ') => {}
                _ => {
                    return Err(RepositoryPathError(
                        "Git porcelain v2 contains an unsupported record".to_owned(),
                    ));
                }
            }
        }
        dirty_paths.sort();
        dirty_paths.dedup();
        Ok(Self {
            dirty: !bytes.is_empty(),
            fingerprint: hash_fields(&[b"git_porcelain_v2", bytes]),
            dirty_paths,
        })
    }

    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn dirty_paths(&self) -> &[String] {
        &self.dirty_paths
    }

    pub fn source_fingerprint(&self, head_oid: Option<&str>) -> SourceFingerprint {
        SourceFingerprint(hash_fields(&[
            b"repository_source",
            head_oid.unwrap_or("unborn").as_bytes(),
            self.fingerprint.as_bytes(),
        ]))
    }
}

fn porcelain_field(
    record: &[u8],
    field_index: usize,
    field_count: usize,
) -> Result<&[u8], RepositoryPathError> {
    record
        .splitn(field_count + 1, |byte| *byte == b' ')
        .nth(field_index)
        .filter(|field| !field.is_empty())
        .ok_or_else(|| RepositoryPathError("Git porcelain v2 record is malformed".to_owned()))
}

fn porcelain_path_field(
    record: &[u8],
    path_field_index: usize,
) -> Result<String, RepositoryPathError> {
    let path = record
        .splitn(path_field_index + 1, |byte| *byte == b' ')
        .nth(path_field_index)
        .ok_or_else(|| RepositoryPathError("Git porcelain v2 path is missing".to_owned()))?;
    normalize_porcelain_path(path)
}

fn normalize_porcelain_path(path: &[u8]) -> Result<String, RepositoryPathError> {
    let path = std::str::from_utf8(path)
        .map_err(|_| RepositoryPathError("Git dirty path is not portable UTF-8".to_owned()))?;
    if path.is_empty()
        || path.starts_with('/')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(RepositoryPathError(
            "Git dirty path escapes or does not identify a repository-relative path".to_owned(),
        ));
    }
    Ok(path.to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryEntryDurability {
    ParentSynchronized,
    ParentSynchronizationFailed,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoReplacePublicationEffect {
    NamesUnchanged,
    Published,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoReplacePublicationPhase {
    Validation,
    NamespacePublication,
    ParentDirectorySynchronization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoReplacePublicationOutcome {
    Published {
        durability: DirectoryEntryDurability,
    },
    DestinationExists,
}

#[derive(Debug)]
pub struct NoReplacePublicationError {
    phase: NoReplacePublicationPhase,
    effect: NoReplacePublicationEffect,
    durability: DirectoryEntryDurability,
    source: io::Error,
}

impl NoReplacePublicationError {
    pub const fn phase(&self) -> NoReplacePublicationPhase {
        self.phase
    }
    pub const fn effect(&self) -> NoReplacePublicationEffect {
        self.effect
    }
    pub const fn durability(&self) -> DirectoryEntryDurability {
        self.durability
    }
    pub fn io_error(&self) -> &io::Error {
        &self.source
    }
}

impl std::fmt::Display for NoReplacePublicationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "atomic no-replace publication failed during {:?} with {:?}: {}",
            self.phase, self.effect, self.source
        )
    }
}

impl std::error::Error for NoReplacePublicationError {}

fn publication_error(
    phase: NoReplacePublicationPhase,
    effect: NoReplacePublicationEffect,
    durability: DirectoryEntryDurability,
    source: io::Error,
) -> NoReplacePublicationError {
    NoReplacePublicationError {
        phase,
        effect,
        durability,
        source,
    }
}

pub fn publish_file_no_replace(
    source: &Path,
    destination: &Path,
) -> Result<NoReplacePublicationOutcome, NoReplacePublicationError> {
    let source_parent = source.parent().ok_or_else(|| {
        publication_error(
            NoReplacePublicationPhase::Validation,
            NoReplacePublicationEffect::NamesUnchanged,
            DirectoryEntryDurability::NotApplicable,
            io::Error::new(io::ErrorKind::InvalidInput, "source has no parent"),
        )
    })?;
    let destination_parent = destination.parent().ok_or_else(|| {
        publication_error(
            NoReplacePublicationPhase::Validation,
            NoReplacePublicationEffect::NamesUnchanged,
            DirectoryEntryDurability::NotApplicable,
            io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent"),
        )
    })?;
    if source_parent != destination_parent || source == destination {
        return Err(publication_error(
            NoReplacePublicationPhase::Validation,
            NoReplacePublicationEffect::NamesUnchanged,
            DirectoryEntryDurability::NotApplicable,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "source and destination must be distinct names in one parent",
            ),
        ));
    }
    let metadata = fs::symlink_metadata(source).map_err(|error| {
        publication_error(
            NoReplacePublicationPhase::Validation,
            NoReplacePublicationEffect::NamesUnchanged,
            DirectoryEntryDurability::NotApplicable,
            error,
        )
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(publication_error(
            NoReplacePublicationPhase::Validation,
            NoReplacePublicationEffect::NamesUnchanged,
            DirectoryEntryDurability::NotApplicable,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "source must be an ordinary file",
            ),
        ));
    }
    match fs::symlink_metadata(destination) {
        Ok(_) => return Ok(NoReplacePublicationOutcome::DestinationExists),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(publication_error(
                NoReplacePublicationPhase::Validation,
                NoReplacePublicationEffect::NamesUnchanged,
                DirectoryEntryDurability::NotApplicable,
                error,
            ))
        }
    }
    match move_no_replace(source, destination) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Ok(NoReplacePublicationOutcome::DestinationExists);
        }
        Err(error) => {
            return Err(publication_error(
                NoReplacePublicationPhase::NamespacePublication,
                NoReplacePublicationEffect::NamesUnchanged,
                DirectoryEntryDurability::NotApplicable,
                error,
            ));
        }
    }
    sync_parent(destination_parent).map_err(|error| {
        publication_error(
            NoReplacePublicationPhase::ParentDirectorySynchronization,
            NoReplacePublicationEffect::Published,
            DirectoryEntryDurability::ParentSynchronizationFailed,
            error,
        )
    })?;
    Ok(NoReplacePublicationOutcome::Published {
        durability: DirectoryEntryDurability::ParentSynchronized,
    })
}

#[cfg(target_os = "linux")]
fn move_no_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use rustix::fs::{renameat_with, RenameFlags, CWD};
    renameat_with(CWD, source, CWD, destination, RenameFlags::NOREPLACE).map_err(io::Error::from)
}

#[cfg(not(target_os = "linux"))]
fn move_no_replace(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic no-replace publication is supported only on Linux",
    ))
}

fn sync_parent(parent: &Path) -> io::Result<()> {
    #[cfg(test)]
    if FAIL_PARENT_SYNC.swap(false, std::sync::atomic::Ordering::AcqRel) {
        return Err(io::Error::other(
            "injected parent-directory synchronization failure",
        ));
    }
    File::open(parent)?.sync_all()
}

#[cfg(test)]
static FAIL_PARENT_SYNC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_sync_failure_reports_published_namespace_effect() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("staging");
        let destination = temporary.path().join("published");
        fs::write(&source, b"complete").expect("staging file");
        FAIL_PARENT_SYNC.store(true, std::sync::atomic::Ordering::Release);
        let error = publish_file_no_replace(&source, &destination)
            .expect_err("parent synchronization must fail");
        assert_eq!(
            error.phase(),
            NoReplacePublicationPhase::ParentDirectorySynchronization
        );
        assert_eq!(error.effect(), NoReplacePublicationEffect::Published);
        assert_eq!(
            error.durability(),
            DirectoryEntryDurability::ParentSynchronizationFailed
        );
        assert!(!source.exists());
        assert_eq!(fs::read(destination).expect("published bytes"), b"complete");
    }
}
