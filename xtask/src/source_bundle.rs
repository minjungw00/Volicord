use crate::repository;
use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const LOCAL_FILE_HEADER_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_DIRECTORY_HEADER_SIGNATURE: u32 = 0x0201_4b50;
const END_OF_CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0605_4b50;
const ZIP_VERSION: u16 = 20;
const ZIP_VERSION_MADE_BY_UNIX: u16 = (3 << 8) | ZIP_VERSION;
const UTF8_FILE_NAME_FLAG: u16 = 1 << 11;
const STORED_COMPRESSION: u16 = 0;
const NORMALIZED_DOS_TIME: u16 = 0;
const NORMALIZED_DOS_DATE: u16 = 1 | (1 << 5);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceBundleReport {
    commit: String,
    tree: String,
    entry_count: usize,
    byte_len: u64,
}

impl SourceBundleReport {
    pub fn commit(&self) -> &str {
        &self.commit
    }

    pub fn tree(&self) -> &str {
        &self.tree
    }

    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    pub fn byte_len(&self) -> u64 {
        self.byte_len
    }
}

#[derive(Clone, Debug)]
struct SelectedTree {
    commit: String,
    tree: String,
    entries: Vec<TreeEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TreeEntryKind {
    Directory,
    Regular,
    Symlink,
}

#[derive(Clone, Debug)]
struct TreeEntry {
    archive_path: String,
    git_mode: u32,
    object_id: Option<String>,
    kind: TreeEntryKind,
}

impl TreeEntry {
    fn archive_mode(&self) -> u32 {
        match self.kind {
            TreeEntryKind::Directory => 0o040755,
            TreeEntryKind::Regular => self.git_mode,
            TreeEntryKind::Symlink => 0o120777,
        }
    }
}

#[derive(Clone, Debug)]
struct ZipEntryData {
    path: String,
    mode: u32,
    data: Vec<u8>,
}

#[derive(Clone, Debug)]
struct CentralDirectoryRecord {
    path: String,
    mode: u32,
    crc32: u32,
    size: u32,
    local_header_offset: u32,
}

#[derive(Clone, Debug)]
struct ParsedZipEntry {
    path: String,
    mode: u32,
    data_start: usize,
    data_end: usize,
}

pub fn create_source_bundle(
    root: &Path,
    output: &Path,
    commit: Option<&str>,
) -> Result<SourceBundleReport> {
    let root = repository::normalize_existing_root(root)?;
    if commit.is_none() {
        ensure_default_head_is_current(&root)?;
    }
    let selected = load_selected_tree(&root, commit.unwrap_or("HEAD"))?;
    let output = resolve_root_relative_path(&root, output);
    ensure_output_is_not_tracked(&root, &output, &selected.entries)?;
    ensure!(
        fs::symlink_metadata(&output).is_err(),
        "refusing to replace existing source bundle output {}",
        output.display()
    );
    let parent = output
        .parent()
        .context("source bundle output has no parent")?;
    ensure!(
        parent.is_dir(),
        "source bundle output parent is not a directory: {}",
        parent.display()
    );

    let archive = build_zip_from_tree(&root, &selected)?;
    validate_zip_against_tree(&root, &selected, &archive)?;

    let mut temporary = tempfile::NamedTempFile::new_in(parent).with_context(|| {
        format!(
            "failed to create temporary source bundle beside {}",
            output.display()
        )
    })?;
    temporary
        .write_all(&archive)
        .context("failed to write temporary source bundle")?;
    temporary
        .as_file_mut()
        .sync_all()
        .context("failed to synchronize temporary source bundle")?;
    temporary.persist_noclobber(&output).map_err(|error| {
        anyhow::anyhow!(
            "failed to publish source bundle {}: {}",
            output.display(),
            error.error
        )
    })?;

    Ok(SourceBundleReport {
        commit: selected.commit,
        tree: selected.tree,
        entry_count: selected.entries.len(),
        byte_len: archive.len() as u64,
    })
}

pub fn validate_source_bundle(
    root: &Path,
    input: &Path,
    commit: Option<&str>,
) -> Result<SourceBundleReport> {
    let root = repository::normalize_existing_root(root)?;
    let selected = load_selected_tree(&root, commit.unwrap_or("HEAD"))?;
    let input = resolve_root_relative_path(&root, input);
    let archive = fs::read(&input)
        .with_context(|| format!("failed to read source bundle {}", input.display()))?;
    validate_zip_against_tree(&root, &selected, &archive)?;

    Ok(SourceBundleReport {
        commit: selected.commit,
        tree: selected.tree,
        entry_count: selected.entries.len(),
        byte_len: archive.len() as u64,
    })
}

fn ensure_default_head_is_current(root: &Path) -> Result<()> {
    let status = git_output(
        root,
        &[
            "status",
            "--porcelain",
            "-z",
            "--untracked-files=no",
            "--ignore-submodules=none",
        ],
    )?;
    ensure!(
        status.is_empty(),
        "tracked index or working-tree changes are present; commit them or select an explicit commit with --commit"
    );
    Ok(())
}

fn load_selected_tree(root: &Path, revision: &str) -> Result<SelectedTree> {
    ensure!(!revision.is_empty(), "source bundle commit cannot be empty");
    let commit_expression = format!("{revision}^{{commit}}");
    let commit = git_text(
        root,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &commit_expression,
        ],
    )
    .with_context(|| format!("failed to resolve commit {revision:?}"))?;
    let tree_expression = format!("{commit}^{{tree}}");
    let tree = git_text(
        root,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &tree_expression,
        ],
    )
    .with_context(|| format!("failed to resolve tree for commit {commit}"))?;
    let output = git_output(root, &["ls-tree", "-r", "-t", "-z", "--full-tree", &tree])
        .with_context(|| format!("failed to inspect Git tree {tree}"))?;
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();

    for raw_record in output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let separator = raw_record
            .iter()
            .position(|byte| *byte == b'\t')
            .context("Git tree entry is missing its path separator")?;
        let metadata = std::str::from_utf8(&raw_record[..separator])
            .context("Git tree metadata is not UTF-8")?;
        let path = std::str::from_utf8(&raw_record[separator + 1..])
            .context("source bundle paths must be UTF-8")?;
        let mut fields = metadata.split_ascii_whitespace();
        let mode_text = fields
            .next()
            .context("Git tree entry is missing its mode")?;
        let object_type = fields
            .next()
            .context("Git tree entry is missing its object type")?;
        let object_id = fields
            .next()
            .context("Git tree entry is missing its object ID")?;
        ensure!(
            fields.next().is_none(),
            "Git tree entry has unexpected metadata fields"
        );
        let git_mode = u32::from_str_radix(mode_text, 8)
            .with_context(|| format!("invalid Git mode {mode_text:?} for {path:?}"))?;
        let (kind, archive_path, object_id) = match (mode_text, object_type) {
            ("040000", "tree") => (TreeEntryKind::Directory, format!("{path}/"), None),
            ("100644" | "100755", "blob") => (
                TreeEntryKind::Regular,
                path.to_string(),
                Some(object_id.to_string()),
            ),
            ("120000", "blob") => (
                TreeEntryKind::Symlink,
                path.to_string(),
                Some(object_id.to_string()),
            ),
            _ => bail!("unsupported Git tree entry {path:?}: mode {mode_text}, type {object_type}"),
        };
        validate_archive_path(&archive_path)?;
        ensure!(
            seen.insert(archive_path.clone()),
            "Git tree produced duplicate archive path {archive_path:?}"
        );
        entries.push(TreeEntry {
            archive_path,
            git_mode,
            object_id,
            kind,
        });
    }

    entries.sort_by(|left, right| {
        left.archive_path
            .as_bytes()
            .cmp(right.archive_path.as_bytes())
    });

    Ok(SelectedTree {
        commit,
        tree,
        entries,
    })
}

fn ensure_output_is_not_tracked(root: &Path, output: &Path, entries: &[TreeEntry]) -> Result<()> {
    let Ok(relative) = output.strip_prefix(root) else {
        return Ok(());
    };
    let relative = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    ensure!(
        !entries
            .iter()
            .any(|entry| entry.archive_path.trim_end_matches('/') == relative),
        "source bundle output path is part of the selected Git tree: {relative}"
    );
    Ok(())
}

fn resolve_root_relative_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        repository::normalize_path(path)
    } else {
        repository::normalize_path(&root.join(path))
    }
}

fn build_zip_from_tree(root: &Path, selected: &SelectedTree) -> Result<Vec<u8>> {
    let mut blob_reader = GitBlobReader::spawn(root)?;
    let mut zip_entries = Vec::with_capacity(selected.entries.len());
    for entry in &selected.entries {
        let data = match entry.object_id.as_deref() {
            Some(object_id) => blob_reader
                .read_blob(object_id)
                .with_context(|| format!("failed to read Git blob for {:?}", entry.archive_path))?,
            None => Vec::new(),
        };
        zip_entries.push(ZipEntryData {
            path: entry.archive_path.clone(),
            mode: entry.archive_mode(),
            data,
        });
    }
    blob_reader.finish()?;
    encode_zip(&zip_entries)
}

fn encode_zip(entries: &[ZipEntryData]) -> Result<Vec<u8>> {
    ensure!(
        entries.len() <= u16::MAX as usize,
        "source bundle has too many ZIP entries"
    );
    let mut archive = Vec::new();
    let mut central_records = Vec::with_capacity(entries.len());

    for entry in entries {
        let name = entry.path.as_bytes();
        ensure!(
            name.len() <= u16::MAX as usize,
            "ZIP path is too long: {:?}",
            entry.path
        );
        let size =
            u32::try_from(entry.data.len()).context("source bundle entry exceeds ZIP32 size")?;
        let local_header_offset =
            u32::try_from(archive.len()).context("source bundle exceeds ZIP32 offset limit")?;
        let crc32 = crc32(&entry.data);

        push_u32(&mut archive, LOCAL_FILE_HEADER_SIGNATURE);
        push_u16(&mut archive, ZIP_VERSION);
        push_u16(&mut archive, UTF8_FILE_NAME_FLAG);
        push_u16(&mut archive, STORED_COMPRESSION);
        push_u16(&mut archive, NORMALIZED_DOS_TIME);
        push_u16(&mut archive, NORMALIZED_DOS_DATE);
        push_u32(&mut archive, crc32);
        push_u32(&mut archive, size);
        push_u32(&mut archive, size);
        push_u16(&mut archive, name.len() as u16);
        push_u16(&mut archive, 0);
        archive.extend_from_slice(name);
        archive.extend_from_slice(&entry.data);

        central_records.push(CentralDirectoryRecord {
            path: entry.path.clone(),
            mode: entry.mode,
            crc32,
            size,
            local_header_offset,
        });
    }

    let central_directory_offset =
        u32::try_from(archive.len()).context("source bundle exceeds ZIP32 offset limit")?;
    for record in &central_records {
        let name = record.path.as_bytes();
        push_u32(&mut archive, CENTRAL_DIRECTORY_HEADER_SIGNATURE);
        push_u16(&mut archive, ZIP_VERSION_MADE_BY_UNIX);
        push_u16(&mut archive, ZIP_VERSION);
        push_u16(&mut archive, UTF8_FILE_NAME_FLAG);
        push_u16(&mut archive, STORED_COMPRESSION);
        push_u16(&mut archive, NORMALIZED_DOS_TIME);
        push_u16(&mut archive, NORMALIZED_DOS_DATE);
        push_u32(&mut archive, record.crc32);
        push_u32(&mut archive, record.size);
        push_u32(&mut archive, record.size);
        push_u16(&mut archive, name.len() as u16);
        push_u16(&mut archive, 0);
        push_u16(&mut archive, 0);
        push_u16(&mut archive, 0);
        push_u16(&mut archive, 0);
        let dos_attribute = if record.path.ends_with('/') { 0x10 } else { 0 };
        push_u32(&mut archive, (record.mode << 16) | dos_attribute);
        push_u32(&mut archive, record.local_header_offset);
        archive.extend_from_slice(name);
    }
    let central_directory_size = u32::try_from(archive.len())
        .context("source bundle exceeds ZIP32 size limit")?
        .checked_sub(central_directory_offset)
        .context("invalid central directory size")?;

    push_u32(&mut archive, END_OF_CENTRAL_DIRECTORY_SIGNATURE);
    push_u16(&mut archive, 0);
    push_u16(&mut archive, 0);
    push_u16(&mut archive, entries.len() as u16);
    push_u16(&mut archive, entries.len() as u16);
    push_u32(&mut archive, central_directory_size);
    push_u32(&mut archive, central_directory_offset);
    push_u16(&mut archive, 0);

    Ok(archive)
}

fn validate_zip_against_tree(root: &Path, selected: &SelectedTree, archive: &[u8]) -> Result<()> {
    let parsed = parse_zip(archive)?;
    ensure!(
        parsed.len() == selected.entries.len(),
        "source bundle entry count {} does not match Git tree entry count {}",
        parsed.len(),
        selected.entries.len()
    );

    let mut blob_reader = GitBlobReader::spawn(root)?;
    for (archive_entry, tree_entry) in parsed.iter().zip(&selected.entries) {
        ensure!(
            archive_entry.path == tree_entry.archive_path,
            "source bundle path {:?} does not match Git tree path {:?}",
            archive_entry.path,
            tree_entry.archive_path
        );
        ensure!(
            archive_entry.mode == tree_entry.archive_mode(),
            "source bundle mode {:06o} for {:?} does not match expected mode {:06o}",
            archive_entry.mode,
            archive_entry.path,
            tree_entry.archive_mode()
        );
        let archive_data = &archive[archive_entry.data_start..archive_entry.data_end];
        match tree_entry.object_id.as_deref() {
            Some(object_id) => {
                let blob = blob_reader.read_blob(object_id).with_context(|| {
                    format!("failed to read Git blob for {:?}", tree_entry.archive_path)
                })?;
                ensure!(
                    archive_data == blob,
                    "source bundle content for {:?} does not match Git blob {object_id}",
                    tree_entry.archive_path
                );
            }
            None => ensure!(
                archive_data.is_empty(),
                "source bundle directory {:?} contains data",
                tree_entry.archive_path
            ),
        }
    }
    blob_reader.finish()
}

fn parse_zip(archive: &[u8]) -> Result<Vec<ParsedZipEntry>> {
    ensure!(
        archive.len() >= 22,
        "source bundle is too short to contain a ZIP end record"
    );
    let end_offset = archive.len() - 22;
    ensure!(
        read_u32(archive, end_offset)? == END_OF_CENTRAL_DIRECTORY_SIGNATURE,
        "source bundle must end with one canonical ZIP end record"
    );
    ensure!(
        read_u16(archive, end_offset + 4)? == 0 && read_u16(archive, end_offset + 6)? == 0,
        "multi-disk ZIP source bundles are not supported"
    );
    let entry_count_on_disk = read_u16(archive, end_offset + 8)?;
    let entry_count = read_u16(archive, end_offset + 10)?;
    ensure!(
        entry_count_on_disk == entry_count,
        "ZIP entry counts disagree across disks"
    );
    let central_size = read_u32(archive, end_offset + 12)? as usize;
    let central_offset = read_u32(archive, end_offset + 16)? as usize;
    ensure!(
        read_u16(archive, end_offset + 20)? == 0,
        "source bundle ZIP comments are not canonical"
    );
    ensure!(
        central_offset
            .checked_add(central_size)
            .is_some_and(|end| end == end_offset),
        "ZIP central directory bounds are invalid"
    );

    let mut entries = Vec::with_capacity(entry_count as usize);
    let mut central_cursor = central_offset;
    let mut local_cursor = 0usize;
    let mut seen = BTreeSet::new();
    for _ in 0..entry_count {
        ensure!(
            read_u32(archive, central_cursor)? == CENTRAL_DIRECTORY_HEADER_SIGNATURE,
            "ZIP central directory entry has an invalid signature"
        );
        ensure!(
            read_u16(archive, central_cursor + 4)? == ZIP_VERSION_MADE_BY_UNIX,
            "ZIP entry was not created with canonical Unix metadata"
        );
        ensure!(
            read_u16(archive, central_cursor + 6)? == ZIP_VERSION,
            "ZIP entry requires an unsupported version"
        );
        validate_common_zip_fields(archive, central_cursor + 8)?;
        let expected_crc = read_u32(archive, central_cursor + 16)?;
        let compressed_size = read_u32(archive, central_cursor + 20)? as usize;
        let uncompressed_size = read_u32(archive, central_cursor + 24)? as usize;
        ensure!(
            compressed_size == uncompressed_size,
            "stored ZIP entry has unequal compressed and uncompressed sizes"
        );
        let name_len = read_u16(archive, central_cursor + 28)? as usize;
        let extra_len = read_u16(archive, central_cursor + 30)? as usize;
        let comment_len = read_u16(archive, central_cursor + 32)? as usize;
        ensure!(
            extra_len == 0 && comment_len == 0,
            "source bundle ZIP entries must not contain extra fields or comments"
        );
        ensure!(
            read_u16(archive, central_cursor + 34)? == 0
                && read_u16(archive, central_cursor + 36)? == 0,
            "source bundle ZIP entry has noncanonical disk or internal attributes"
        );
        let external_attributes = read_u32(archive, central_cursor + 38)?;
        let mode = external_attributes >> 16;
        let local_offset = read_u32(archive, central_cursor + 42)? as usize;
        ensure!(
            local_offset == local_cursor,
            "ZIP local entries are not in canonical contiguous order"
        );
        let name_start = checked_add(central_cursor, 46, archive.len())?;
        let name_end = checked_add(name_start, name_len, archive.len())?;
        ensure!(
            name_end <= central_offset + central_size,
            "ZIP central entry name exceeds the central directory"
        );
        let path = std::str::from_utf8(&archive[name_start..name_end])
            .context("source bundle ZIP path is not UTF-8")?
            .to_string();
        validate_archive_path(&path)?;
        ensure!(
            seen.insert(path.clone()),
            "source bundle contains duplicate path {path:?}"
        );
        validate_archive_mode(&path, mode, external_attributes)?;

        ensure!(
            read_u32(archive, local_offset)? == LOCAL_FILE_HEADER_SIGNATURE,
            "ZIP local entry has an invalid signature"
        );
        ensure!(
            read_u16(archive, local_offset + 4)? == ZIP_VERSION,
            "ZIP local entry requires an unsupported version"
        );
        validate_common_zip_fields(archive, local_offset + 6)?;
        ensure!(
            read_u32(archive, local_offset + 14)? == expected_crc,
            "ZIP local and central CRC values disagree"
        );
        ensure!(
            read_u32(archive, local_offset + 18)? as usize == compressed_size
                && read_u32(archive, local_offset + 22)? as usize == uncompressed_size,
            "ZIP local and central sizes disagree"
        );
        let local_name_len = read_u16(archive, local_offset + 26)? as usize;
        let local_extra_len = read_u16(archive, local_offset + 28)? as usize;
        ensure!(
            local_extra_len == 0,
            "source bundle ZIP local entries must not contain extra fields"
        );
        let local_name_start = checked_add(local_offset, 30, archive.len())?;
        let local_name_end = checked_add(local_name_start, local_name_len, archive.len())?;
        ensure!(
            archive.get(local_name_start..local_name_end) == Some(path.as_bytes()),
            "ZIP local and central paths disagree for {path:?}"
        );
        let data_start = local_name_end;
        let data_end = checked_add(data_start, compressed_size, archive.len())?;
        ensure!(
            data_end <= central_offset,
            "ZIP entry data overlaps the central directory"
        );
        ensure!(
            crc32(&archive[data_start..data_end]) == expected_crc,
            "ZIP entry CRC does not match content for {path:?}"
        );
        if path.ends_with('/') {
            ensure!(
                data_start == data_end,
                "ZIP directory entry contains data for {path:?}"
            );
        }

        entries.push(ParsedZipEntry {
            path,
            mode,
            data_start,
            data_end,
        });
        local_cursor = data_end;
        central_cursor = name_end;
    }
    ensure!(
        central_cursor == central_offset + central_size,
        "ZIP central directory contains trailing or missing data"
    );
    ensure!(
        local_cursor == central_offset,
        "ZIP local entry region contains trailing or missing data"
    );
    Ok(entries)
}

fn validate_common_zip_fields(archive: &[u8], offset: usize) -> Result<()> {
    ensure!(
        read_u16(archive, offset)? == UTF8_FILE_NAME_FLAG,
        "ZIP entry flags are not canonical UTF-8-only flags"
    );
    ensure!(
        read_u16(archive, offset + 2)? == STORED_COMPRESSION,
        "source bundle ZIP entries must use deterministic stored compression"
    );
    ensure!(
        read_u16(archive, offset + 4)? == NORMALIZED_DOS_TIME
            && read_u16(archive, offset + 6)? == NORMALIZED_DOS_DATE,
        "source bundle ZIP timestamps are not normalized"
    );
    Ok(())
}

fn validate_archive_mode(path: &str, mode: u32, external_attributes: u32) -> Result<()> {
    let is_directory = path.ends_with('/');
    let expected_dos_attribute = if is_directory { 0x10 } else { 0 };
    ensure!(
        external_attributes == (mode << 16) | expected_dos_attribute,
        "ZIP external attributes are not canonical for {path:?}"
    );
    match (mode & 0o170000, is_directory) {
        (0o040000, true) => ensure!(
            mode == 0o040755,
            "ZIP directory mode must be 040755 for {path:?}"
        ),
        (0o100000, false) => ensure!(
            matches!(mode, 0o100644 | 0o100755),
            "ZIP regular-file mode must be 100644 or 100755 for {path:?}"
        ),
        (0o120000, false) => ensure!(
            mode == 0o120777,
            "ZIP symbolic-link mode must be 120777 for {path:?}"
        ),
        _ => bail!("ZIP file type and path suffix disagree for {path:?}"),
    }
    Ok(())
}

fn validate_archive_path(path: &str) -> Result<()> {
    ensure!(!path.is_empty(), "source bundle path cannot be empty");
    ensure!(
        !path.starts_with('/') && !path.starts_with('\\'),
        "source bundle path must be relative: {path:?}"
    );
    ensure!(
        !path.contains('\\'),
        "source bundle path must use forward slashes: {path:?}"
    );
    ensure!(
        !path.contains('\0'),
        "source bundle path must not contain NUL: {path:?}"
    );
    let body = path.strip_suffix('/').unwrap_or(path);
    ensure!(
        !body.is_empty(),
        "source bundle root directory entry is not allowed"
    );
    let components = body.split('/').collect::<Vec<_>>();
    ensure!(
        !components
            .first()
            .is_some_and(|component| is_windows_drive_component(component)),
        "source bundle path must not use a drive prefix: {path:?}"
    );
    for component in components {
        ensure!(
            !component.is_empty(),
            "source bundle path has an empty component: {path:?}"
        );
        ensure!(
            !matches!(component, "." | ".."),
            "source bundle path has a traversal component: {path:?}"
        );
        ensure!(
            !component.eq_ignore_ascii_case(".git"),
            "source bundle path contains Git metadata: {path:?}"
        );
    }
    Ok(())
}

fn is_windows_drive_component(component: &str) -> bool {
    let bytes = component.as_bytes();
    bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

struct GitBlobReader {
    child: Child,
    input: Option<BufWriter<ChildStdin>>,
    output: BufReader<ChildStdout>,
}

impl GitBlobReader {
    fn spawn(root: &Path) -> Result<Self> {
        let mut child = Command::new("git")
            .current_dir(root)
            .args(["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("failed to start git cat-file --batch")?;
        let input = child
            .stdin
            .take()
            .context("git cat-file stdin is unavailable")?;
        let output = child
            .stdout
            .take()
            .context("git cat-file stdout is unavailable")?;
        Ok(Self {
            child,
            input: Some(BufWriter::new(input)),
            output: BufReader::new(output),
        })
    }

    fn read_blob(&mut self, object_id: &str) -> Result<Vec<u8>> {
        let input = self
            .input
            .as_mut()
            .context("git cat-file input is closed")?;
        writeln!(input, "{object_id}").context("failed to request Git blob")?;
        input.flush().context("failed to flush Git blob request")?;

        let mut header = String::new();
        self.output
            .read_line(&mut header)
            .context("failed to read Git blob header")?;
        ensure!(!header.is_empty(), "git cat-file ended before blob header");
        let mut fields = header.split_ascii_whitespace();
        let returned_id = fields
            .next()
            .context("Git blob header is missing object ID")?;
        let object_type = fields.next().context("Git blob header is missing type")?;
        let size_text = fields.next().context("Git blob header is missing size")?;
        ensure!(
            fields.next().is_none(),
            "Git blob header has unexpected fields"
        );
        ensure!(
            returned_id == object_id,
            "git cat-file returned {returned_id} for requested object {object_id}"
        );
        ensure!(
            object_type == "blob",
            "Git object {object_id} is {object_type}, not a blob"
        );
        let size = size_text
            .parse::<usize>()
            .with_context(|| format!("invalid Git blob size {size_text:?}"))?;
        let mut data = vec![0; size];
        self.output
            .read_exact(&mut data)
            .context("failed to read Git blob content")?;
        let mut newline = [0u8; 1];
        self.output
            .read_exact(&mut newline)
            .context("failed to read Git blob delimiter")?;
        ensure!(
            newline == *b"\n",
            "Git blob response has an invalid delimiter"
        );
        Ok(data)
    }

    fn finish(mut self) -> Result<()> {
        self.input.take();
        let status = self
            .child
            .wait()
            .context("failed to wait for git cat-file")?;
        ensure!(
            status.success(),
            "git cat-file --batch failed with {status}"
        );
        Ok(())
    }
}

fn git_text(root: &Path, args: &[&str]) -> Result<String> {
    let output = git_output(root, args)?;
    let text = String::from_utf8(output).context("Git output is not UTF-8")?;
    let text = text.trim_end_matches(['\r', '\n']);
    ensure!(!text.is_empty(), "Git command returned an empty value");
    Ok(text.to_string())
}

fn git_output(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;
    ensure!(
        output.status.success(),
        "git {} failed with {}: {}",
        args.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(output.stdout)
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn read_u16(input: &[u8], offset: usize) -> Result<u16> {
    let end = checked_add(offset, 2, input.len())?;
    Ok(u16::from_le_bytes(
        input[offset..end]
            .try_into()
            .expect("two-byte slice checked above"),
    ))
}

fn read_u32(input: &[u8], offset: usize) -> Result<u32> {
    let end = checked_add(offset, 4, input.len())?;
    Ok(u32::from_le_bytes(
        input[offset..end]
            .try_into()
            .expect("four-byte slice checked above"),
    ))
}

fn checked_add(offset: usize, length: usize, bound: usize) -> Result<usize> {
    let end = offset
        .checked_add(length)
        .context("ZIP offset overflowed")?;
    ensure!(end <= bound, "ZIP structure exceeds archive bounds");
    Ok(end)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> ZipEntryData {
        ZipEntryData {
            path: path.to_string(),
            mode: 0o100644,
            data: b"fixture".to_vec(),
        }
    }

    #[test]
    fn rejects_duplicate_zip_paths() {
        let archive = encode_zip(&[entry("same.txt"), entry("same.txt")]).expect("fixture ZIP");
        let error = parse_zip(&archive).expect_err("duplicate paths must fail");
        assert!(error.to_string().contains("duplicate path"));
    }

    #[test]
    fn rejects_absolute_zip_paths() {
        let archive = encode_zip(&[entry("/absolute")]).expect("fixture ZIP");
        let error = parse_zip(&archive).expect_err("absolute paths must fail");
        assert!(error.to_string().contains("must be relative"));
    }

    #[test]
    fn rejects_parent_traversal_zip_paths() {
        let archive = encode_zip(&[entry("../outside")]).expect("fixture ZIP");
        let error = parse_zip(&archive).expect_err("traversal paths must fail");
        assert!(error.to_string().contains("traversal component"));
    }

    #[test]
    fn canonical_zip_encoding_is_stable() {
        let entries = [
            ZipEntryData {
                path: "dir/".to_string(),
                mode: 0o040755,
                data: Vec::new(),
            },
            entry("dir/file.txt"),
        ];
        assert_eq!(
            encode_zip(&entries).expect("first ZIP"),
            encode_zip(&entries).expect("second ZIP")
        );
    }
}
