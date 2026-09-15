//! Current-only, structurally shared Analysis Snapshot persistence.
use crate::Error;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};
use volicord_context::ProjectId;
use volicord_repository_intelligence::{
    AnalysisMetadata, AnalysisSnapshot, AnalysisSnapshotId, CanonicalProjectRef,
    ANALYSIS_SNAPSHOT_FORMAT_VERSION, ANALYSIS_SNAPSHOT_KIND,
};

const STORAGE_FORMAT: &str = "volicord.normalized_analysis";
const SHAPE_MAGIC: &[u8] = b"VOLICORD-JSON-SHAPE\0";
const VALUES_MAGIC: &[u8] = b"VOLICORD-JSON-VALUES\0";
const VALUES_DELTA_MAGIC: &[u8] = b"VOLICORD-JSON-DELTA\0";

#[derive(Clone, Deserialize)]
pub(crate) struct AnalysisHeader {
    format_kind: String,
    format_version: u32,
    pub identity: AnalysisSnapshotId,
    project: CanonicalProjectRef,
    pub generated_at_unix_micros: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AnalysisManifest {
    format_kind: String,
    format_version: u32,
    storage_format: String,
    pub identity: AnalysisSnapshotId,
    project: CanonicalProjectRef,
    pub generated_at_unix_micros: i64,
    pub logical_json_bytes: u64,
    pub shape_blobs: Vec<String>,
    pub values_blob: String,
    pub values_base_blob: Option<String>,
    pub scalar_count: u64,
    pub inventory_entry_count: u64,
    pub entity_count: u64,
    pub relation_count: u64,
    pub metadata: AnalysisMetadata,
}

pub(crate) struct EncodedAnalysis {
    pub manifest: Vec<u8>,
    pub shapes: Vec<(String, Vec<u8>)>,
    pub values: Vec<u8>,
    pub values_hash: String,
}

pub(crate) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let file = File::open(path)
        .map_err(|error| Error::with_source("cannot open Analysis Snapshot", error))?;
    serde_json::from_reader(BufReader::with_capacity(64 * 1024, file)).map_err(|error| {
        Error::with_source(
            format!(
                "Analysis Snapshot {} is unsupported or corrupt",
                path.display()
            ),
            error,
        )
    })
}

pub(crate) fn encode_analysis(
    analysis: &AnalysisSnapshot,
    base_values: Option<(String, Vec<u8>)>,
) -> Result<EncodedAnalysis, Error> {
    let mut encoder = NormalizingWriter::default();
    serde_json::to_writer(&mut encoder, analysis)
        .map_err(|error| Error::with_source("cannot normalize Analysis Snapshot", error))?;
    let normalized = encoder.finish()?;
    let shapes = content_defined_chunks(&normalized.shape)
        .into_iter()
        .map(|chunk| (digest(chunk), chunk.to_vec()))
        .collect::<Vec<_>>();
    let (values, values_base_blob) = if let Some((base_hash, base)) = base_values {
        let delta = encode_delta(&base, &normalized.values);
        if delta.len() < normalized.values.len() {
            (delta, Some(base_hash))
        } else {
            (normalized.values, None)
        }
    } else {
        (normalized.values, None)
    };
    let values_hash = digest(&values);
    let manifest = AnalysisManifest {
        format_kind: ANALYSIS_SNAPSHOT_KIND.into(),
        format_version: ANALYSIS_SNAPSHOT_FORMAT_VERSION,
        storage_format: STORAGE_FORMAT.into(),
        identity: analysis.identity,
        project: analysis.project,
        generated_at_unix_micros: analysis.generated_at_unix_micros,
        logical_json_bytes: normalized.logical_bytes,
        shape_blobs: shapes.iter().map(|(hash, _)| hash.clone()).collect(),
        values_blob: values_hash.clone(),
        values_base_blob,
        scalar_count: normalized.scalar_count,
        inventory_entry_count: analysis.inventory.entries.len() as u64,
        entity_count: analysis.structural_facts.len() as u64,
        relation_count: analysis
            .structural_facts
            .iter()
            .map(|fact| fact.relations.len() as u64)
            .sum::<u64>()
            + analysis.semantic_results.len() as u64,
        metadata: AnalysisMetadata::from(analysis),
    };
    Ok(EncodedAnalysis {
        manifest: serde_json::to_vec(&manifest)
            .map_err(|error| Error::with_source("cannot encode Analysis manifest", error))?,
        shapes,
        values,
        values_hash,
    })
}

pub(crate) fn read_manifest(path: &Path) -> Result<AnalysisManifest, Error> {
    let manifest: AnalysisManifest = read_json(path)?;
    if manifest.format_kind != ANALYSIS_SNAPSHOT_KIND
        || manifest.format_version != ANALYSIS_SNAPSHOT_FORMAT_VERSION
        || manifest.storage_format != STORAGE_FORMAT
    {
        return Err(Error::new(
            "Analysis manifest uses an unsupported current storage format",
        ));
    }
    Ok(manifest)
}

pub(crate) fn read_analysis(path: &Path) -> Result<AnalysisSnapshot, Error> {
    let manifest = read_manifest(path)?;
    let blobs = blob_dir(path)?;
    if manifest.shape_blobs.is_empty() {
        return Err(Error::new("Analysis manifest has no shape blobs"));
    }
    let mut shape = Vec::new();
    for hash in &manifest.shape_blobs {
        shape.extend_from_slice(&read_verified_blob(
            &blobs.join(format!("{hash}.shape")),
            hash,
        )?);
    }
    let encoded_values = read_verified_blob(
        &blobs.join(format!("{}.values", manifest.values_blob)),
        &manifest.values_blob,
    )?;
    let values = if let Some(base_hash) = &manifest.values_base_blob {
        let base = read_verified_blob(&blobs.join(format!("{base_hash}.values")), base_hash)?;
        apply_delta(&base, &encoded_values)?
    } else {
        encoded_values
    };
    let reader = DenormalizingReader::new(shape, values, manifest.scalar_count)?;
    let snapshot: AnalysisSnapshot = serde_json::from_reader(reader)
        .map_err(|error| Error::with_source("normalized Analysis Snapshot is corrupt", error))?;
    if snapshot.identity != manifest.identity
        || snapshot.project != manifest.project
        || snapshot.generated_at_unix_micros != manifest.generated_at_unix_micros
    {
        return Err(Error::new("Analysis manifest and payload bindings differ"));
    }
    Ok(snapshot)
}

pub(crate) fn blob_dir(manifest: &Path) -> Result<PathBuf, Error> {
    Ok(manifest
        .parent()
        .ok_or_else(|| Error::new("Analysis manifest has no parent"))?
        .join("blobs"))
}

fn read_verified_blob(path: &Path, expected: &str) -> Result<Vec<u8>, Error> {
    let bytes = std::fs::read(path)
        .map_err(|error| Error::with_source("cannot read Analysis blob", error))?;
    if digest(&bytes) != expected {
        return Err(Error::new("Analysis blob content identity is corrupt"));
    }
    Ok(bytes)
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn reusable_base_values(
    project_directory: &Path,
) -> Result<Option<(String, Vec<u8>)>, Error> {
    if !project_directory.exists() {
        return Ok(None);
    }
    for entry in std::fs::read_dir(project_directory)
        .map_err(|error| Error::with_source("cannot inspect Analysis values bases", error))?
    {
        let path = entry
            .map_err(|error| Error::with_source("cannot inspect Analysis values base", error))?
            .path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let manifest = match read_manifest(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if manifest.values_base_blob.is_none() {
            let blobs = blob_dir(&path)?;
            let Ok(bytes) = read_verified_blob(
                &blobs.join(format!("{}.values", manifest.values_blob)),
                &manifest.values_blob,
            ) else {
                continue;
            };
            return Ok(Some((manifest.values_blob, bytes)));
        }
    }
    Ok(None)
}

fn encode_delta(base: &[u8], current: &[u8]) -> Vec<u8> {
    let mut ranges = Vec::<(usize, Vec<u8>)>::new();
    let mut at = 0;
    while at < current.len() {
        if base.get(at) == current.get(at) {
            at += 1;
            continue;
        }
        let start = at;
        let mut equal_run = 0_usize;
        while at < current.len() {
            if base.get(at) == current.get(at) {
                equal_run += 1;
                if equal_run >= 64 {
                    at = at + 1 - equal_run;
                    break;
                }
            } else {
                equal_run = 0;
            }
            at += 1;
        }
        ranges.push((start, current[start..at].to_vec()));
    }
    let mut output = VALUES_DELTA_MAGIC.to_vec();
    put_u64(&mut output, current.len() as u64);
    put_u64(&mut output, ranges.len() as u64);
    for (offset, bytes) in ranges {
        put_u64(&mut output, offset as u64);
        put_u64(&mut output, bytes.len() as u64);
        output.extend_from_slice(&bytes);
    }
    output
}

fn apply_delta(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, Error> {
    let mut at = expect_magic(delta, VALUES_DELTA_MAGIC)?;
    let length = take_u64(delta, &mut at)? as usize;
    let range_count = take_u64(delta, &mut at)?;
    let mut output = base[..base.len().min(length)].to_vec();
    output.resize(length, 0);
    for _ in 0..range_count {
        let offset = take_u64(delta, &mut at)? as usize;
        let count = take_u64(delta, &mut at)? as usize;
        let end = offset
            .checked_add(count)
            .filter(|end| *end <= output.len())
            .ok_or_else(|| Error::new("Analysis values delta range is corrupt"))?;
        let source_end = at
            .checked_add(count)
            .filter(|end| *end <= delta.len())
            .ok_or_else(|| Error::new("Analysis values delta is truncated"))?;
        output[offset..end].copy_from_slice(&delta[at..source_end]);
        at = source_end;
    }
    if at != delta.len() {
        return Err(Error::new("Analysis values delta has trailing data"));
    }
    Ok(output)
}

fn content_defined_chunks(bytes: &[u8]) -> Vec<&[u8]> {
    let mut output = Vec::new();
    let mut start = 0;
    for index in 0..bytes.len() {
        let length = index + 1 - start;
        let boundary = if index >= 31 {
            bytes[index - 31..=index]
                .iter()
                .fold(0xcbf2_9ce4_8422_2325, |hash, value| {
                    (hash ^ u64::from(*value)).wrapping_mul(0x100_0000_01b3)
                })
                & 0x1fff
                == 0
        } else {
            false
        };
        if length >= 8 * 1024 && (boundary || length >= 32 * 1024) {
            output.push(&bytes[start..=index]);
            start = index + 1;
        }
    }
    if start < bytes.len() {
        output.push(&bytes[start..]);
    }
    output
}

impl AnalysisHeader {
    pub fn read(path: &Path, project_id: ProjectId) -> Result<Self, Error> {
        let header: Self = read_json(path)?;
        if header.format_kind != ANALYSIS_SNAPSHOT_KIND
            || header.format_version != ANALYSIS_SNAPSHOT_FORMAT_VERSION
            || header.project.identity() != project_id
            || path.file_stem().and_then(|name| name.to_str()) != Some(&header.identity.to_string())
        {
            return Err(Error::new(
                "Analysis Snapshot header has incompatible format, identity, or Project binding",
            ));
        }
        Ok(header)
    }

    pub fn matches(&self, snapshot: &AnalysisSnapshot) -> bool {
        self.identity == snapshot.identity
            && self.project == snapshot.project
            && self.generated_at_unix_micros == snapshot.generated_at_unix_micros
    }
}

#[derive(Default)]
struct NormalizingWriter {
    shape: Vec<u8>,
    symbols: Vec<Vec<u8>>,
    symbol_index: HashMap<Vec<u8>, u32>,
    references: Vec<u32>,
    token: Vec<u8>,
    in_string: bool,
    escaped: bool,
    logical_bytes: u64,
}

impl Write for NormalizingWriter {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        self.logical_bytes = self.logical_bytes.saturating_add(input.len() as u64);
        for &byte in input {
            if self.in_string {
                self.token.push(byte);
                if self.escaped {
                    self.escaped = false;
                } else if byte == b'\\' {
                    self.escaped = true;
                } else if byte == b'"' {
                    self.in_string = false;
                    self.finish_scalar()?;
                }
                continue;
            }
            if !self.token.is_empty() {
                if matches!(byte, b',' | b']' | b'}' | b':') {
                    self.finish_scalar()?;
                } else if byte.is_ascii_whitespace() {
                    self.finish_scalar()?;
                    continue;
                } else {
                    self.token.push(byte);
                    continue;
                }
            }
            match byte {
                b'"' => {
                    self.in_string = true;
                    self.token.push(byte);
                }
                b'{' | b'}' | b'[' | b']' | b',' | b':' => self.shape.push(byte),
                value if value.is_ascii_whitespace() => {}
                value => self.token.push(value),
            }
        }
        Ok(input.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl NormalizingWriter {
    fn finish_scalar(&mut self) -> std::io::Result<()> {
        let value = std::mem::take(&mut self.token);
        let index = if let Some(index) = self.symbol_index.get(&value) {
            *index
        } else {
            let index = u32::try_from(self.symbols.len()).map_err(std::io::Error::other)?;
            self.symbol_index.insert(value.clone(), index);
            self.symbols.push(value);
            index
        };
        self.shape.push(b'$');
        self.references.push(index);
        Ok(())
    }

    fn finish(mut self) -> Result<Normalized, Error> {
        if self.in_string {
            return Err(Error::new("Analysis JSON ended inside a string"));
        }
        if !self.token.is_empty() {
            self.finish_scalar()
                .map_err(|error| Error::with_source("cannot finish Analysis scalar", error))?;
        }
        let mut shape = SHAPE_MAGIC.to_vec();
        put_u64(&mut shape, self.shape.len() as u64);
        shape.extend_from_slice(&self.shape);
        let mut values = VALUES_MAGIC.to_vec();
        put_u64(&mut values, self.symbols.len() as u64);
        for symbol in self.symbols {
            put_u64(&mut values, symbol.len() as u64);
            values.extend_from_slice(&symbol);
        }
        put_u64(&mut values, self.references.len() as u64);
        for reference in &self.references {
            values.extend_from_slice(&reference.to_le_bytes());
        }
        Ok(Normalized {
            shape,
            values,
            logical_bytes: self.logical_bytes,
            scalar_count: self.references.len() as u64,
        })
    }
}

struct Normalized {
    shape: Vec<u8>,
    values: Vec<u8>,
    logical_bytes: u64,
    scalar_count: u64,
}

struct DenormalizingReader {
    shape: Vec<u8>,
    symbols: Vec<Vec<u8>>,
    references: Vec<u32>,
    shape_at: usize,
    reference_at: usize,
    pending: Vec<u8>,
    pending_at: usize,
}

impl DenormalizingReader {
    fn new(shape: Vec<u8>, values: Vec<u8>, expected_scalars: u64) -> Result<Self, Error> {
        let mut shape_at = expect_magic(&shape, SHAPE_MAGIC)?;
        let shape_len = take_u64(&shape, &mut shape_at)? as usize;
        let shape_end = shape_at
            .checked_add(shape_len)
            .filter(|end| *end == shape.len())
            .ok_or_else(|| Error::new("Analysis shape length is corrupt"))?;
        let shape = shape[shape_at..shape_end].to_vec();
        let mut at = expect_magic(&values, VALUES_MAGIC)?;
        let symbol_count = take_u64(&values, &mut at)? as usize;
        let mut symbols = Vec::with_capacity(symbol_count);
        for _ in 0..symbol_count {
            let length = take_u64(&values, &mut at)? as usize;
            let end = at
                .checked_add(length)
                .filter(|end| *end <= values.len())
                .ok_or_else(|| Error::new("Analysis symbol length is corrupt"))?;
            symbols.push(values[at..end].to_vec());
            at = end;
        }
        let reference_count = take_u64(&values, &mut at)?;
        if reference_count != expected_scalars {
            return Err(Error::new("Analysis scalar count differs from manifest"));
        }
        let mut references = Vec::with_capacity(reference_count as usize);
        for _ in 0..reference_count {
            let end = at
                .checked_add(4)
                .filter(|end| *end <= values.len())
                .ok_or_else(|| Error::new("Analysis scalar reference is truncated"))?;
            references.push(u32::from_le_bytes(
                values[at..end].try_into().expect("four bytes"),
            ));
            at = end;
        }
        if at != values.len()
            || references
                .iter()
                .any(|index| *index as usize >= symbols.len())
        {
            return Err(Error::new(
                "Analysis values contain trailing or invalid references",
            ));
        }
        Ok(Self {
            shape,
            symbols,
            references,
            shape_at: 0,
            reference_at: 0,
            pending: Vec::new(),
            pending_at: 0,
        })
    }
}

impl Read for DenormalizingReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let mut written = 0;
        while written < output.len() {
            if self.pending_at < self.pending.len() {
                let count = (output.len() - written).min(self.pending.len() - self.pending_at);
                output[written..written + count]
                    .copy_from_slice(&self.pending[self.pending_at..self.pending_at + count]);
                self.pending_at += count;
                written += count;
                continue;
            }
            self.pending.clear();
            self.pending_at = 0;
            let Some(&token) = self.shape.get(self.shape_at) else {
                break;
            };
            self.shape_at += 1;
            if token == b'$' {
                let index = *self
                    .references
                    .get(self.reference_at)
                    .ok_or_else(|| std::io::Error::other("missing Analysis scalar reference"))?
                    as usize;
                self.reference_at += 1;
                self.pending.extend_from_slice(
                    self.symbols.get(index).ok_or_else(|| {
                        std::io::Error::other("invalid Analysis scalar reference")
                    })?,
                );
            } else {
                self.pending.push(token);
            }
        }
        Ok(written)
    }
}

fn put_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}
fn expect_magic(bytes: &[u8], magic: &[u8]) -> Result<usize, Error> {
    bytes
        .starts_with(magic)
        .then_some(magic.len())
        .ok_or_else(|| Error::new("Analysis blob format is corrupt"))
}
fn take_u64(bytes: &[u8], at: &mut usize) -> Result<u64, Error> {
    let end = at
        .checked_add(8)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| Error::new("Analysis blob integer is truncated"))?;
    let value = u64::from_le_bytes(bytes[*at..end].try_into().expect("eight bytes"));
    *at = end;
    Ok(value)
}
