//! Current-only, structurally shared Analysis Snapshot persistence.
use crate::Error;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read, Write},
    ops::Range,
    path::{Path, PathBuf},
};
use volicord_context::ProjectId;
use volicord_repository_intelligence::{
    AnalysisMetadata, AnalysisSnapshot, AnalysisSnapshotId, CanonicalProjectRef,
    ANALYSIS_SNAPSHOT_FORMAT_VERSION, ANALYSIS_SNAPSHOT_KIND,
};

const STORAGE_FORMAT: &str = "volicord.normalized_analysis.v2";
const SHAPE_MAGIC: &[u8] = b"VOLICORD-JSON-SHAPE\0";
const VALUES_MAGIC: &[u8] = b"VOLICORD-JSON-VALUES\0";
const VALUES_DELTA_MAGIC: &[u8] = b"VOLICORD-JSON-DELTA2\0";
const BLOB_RAW_MAGIC: &[u8] = b"VOLICORD-BLOB-RAW1\0";
const BLOB_ZSTD_MAGIC: &[u8] = b"VOLICORD-BLOB-ZSTD1\0";
const ANALYSIS_CACHE_MAGIC: &[u8] = b"VOLICORD-ANALYSIS-CACHE1\0";
const ANALYSIS_CACHE_EXTENSION: &str = "snapshot";
const CHUNK_MIN_BYTES: usize = 8 * 1024;
const CHUNK_MAX_BYTES: usize = 32 * 1024;
const CHUNK_WINDOW_BYTES: usize = 63;
const SMALL_COMPLETE_VALUES_BYTES: usize = 8 * 1024 * 1024;

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
    pub metadata_blob: String,
}

pub(crate) struct EncodedAnalysis {
    pub manifest: Vec<u8>,
    pub shapes: Vec<(String, Vec<u8>)>,
    pub values: Vec<u8>,
    pub values_hash: String,
    pub metadata: Vec<u8>,
    pub metadata_hash: String,
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
    let mut encoder = match &base_values {
        Some((_, base)) => NormalizingWriter::with_base_symbols(base)?,
        None => NormalizingWriter::default(),
    };
    serde_json::to_writer(&mut encoder, analysis)
        .map_err(|error| Error::with_source("cannot normalize Analysis Snapshot", error))?;
    let normalized = encoder.finish()?;
    let shapes = content_defined_chunks(&normalized.shape)
        .into_iter()
        .map(|chunk| {
            let packed = pack_blob(chunk)?;
            Ok((digest(&packed), packed))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let complete_values = pack_blob(&normalized.values)?;
    let (values, values_base_blob) = if let Some((base_hash, base)) = base_values {
        let delta = pack_blob(&encode_delta(&base, &normalized.values))?;
        if delta.len() < complete_values.len()
            && (complete_values.len() <= SMALL_COMPLETE_VALUES_BYTES
                || delta.len() < complete_values.len() / 2)
        {
            (delta, Some(base_hash))
        } else {
            (complete_values, None)
        }
    } else {
        (complete_values, None)
    };
    let values_hash = digest(&values);
    let metadata = pack_blob_at_level(
        &serde_json::to_vec(&AnalysisMetadata::from(analysis))
            .map_err(|error| Error::with_source("cannot encode Analysis metadata", error))?,
        9,
    )?;
    let metadata_hash = digest(&metadata);
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
        metadata_blob: metadata_hash.clone(),
    };
    Ok(EncodedAnalysis {
        manifest: serde_json::to_vec(&manifest)
            .map_err(|error| Error::with_source("cannot encode Analysis manifest", error))?,
        shapes,
        values,
        values_hash,
        metadata,
        metadata_hash,
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

pub(crate) fn read_metadata(
    path: &Path,
    manifest: &AnalysisManifest,
) -> Result<AnalysisMetadata, Error> {
    let blobs = blob_dir(path)?;
    let packed = read_verified_blob(
        &blobs.join(format!("{}.metadata", manifest.metadata_blob)),
        &manifest.metadata_blob,
    )?;
    let metadata: AnalysisMetadata = serde_json::from_slice(&unpack_blob(&packed)?)
        .map_err(|error| Error::with_source("Analysis metadata is corrupt", error))?;
    if metadata.identity != manifest.identity
        || metadata.project != manifest.project
        || metadata.generated_at_unix_micros != manifest.generated_at_unix_micros
    {
        return Err(Error::new("Analysis manifest and metadata bindings differ"));
    }
    Ok(metadata)
}

pub(crate) fn read_analysis(path: &Path) -> Result<AnalysisSnapshot, Error> {
    let manifest = read_manifest(path)?;
    verify_manifest_blobs(path, &manifest)?;
    if let Some(snapshot) = read_cached_analysis(path, &manifest) {
        return Ok(snapshot);
    }
    read_analysis_from_manifest(path, manifest)
}

pub(crate) fn read_analysis_durable(path: &Path) -> Result<AnalysisSnapshot, Error> {
    let manifest = read_manifest(path)?;
    read_analysis_from_manifest(path, manifest)
}

fn read_analysis_from_manifest(
    path: &Path,
    manifest: AnalysisManifest,
) -> Result<AnalysisSnapshot, Error> {
    read_metadata(path, &manifest)?;
    let blobs = blob_dir(path)?;
    if manifest.shape_blobs.is_empty() {
        return Err(Error::new("Analysis manifest has no shape blobs"));
    }
    let mut shape = Vec::new();
    for hash in &manifest.shape_blobs {
        let packed = read_verified_blob(&blobs.join(format!("{hash}.shape")), hash)?;
        shape.extend_from_slice(&unpack_blob(&packed)?);
    }
    let packed_values = read_verified_blob(
        &blobs.join(format!("{}.values", manifest.values_blob)),
        &manifest.values_blob,
    )?;
    let encoded_values = unpack_blob(&packed_values)?;
    let values = if let Some(base_hash) = &manifest.values_base_blob {
        let packed_base =
            read_verified_blob(&blobs.join(format!("{base_hash}.values")), base_hash)?;
        let base = unpack_blob(&packed_base)?;
        apply_delta(&base, &encoded_values)?
    } else {
        encoded_values
    };
    let reader = DenormalizingReader::new(shape, values, manifest.scalar_count)?;
    // serde_json's generic reader path may request very small reads while it scans
    // tokens. Buffer the reconstructed stream so a large normalized graph does not
    // repeatedly cross the denormalizer boundary one byte at a time.
    let reader = BufReader::with_capacity(1024 * 1024, reader);
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

pub(crate) fn write_analysis_cache(
    output: &mut impl Write,
    analysis: &AnalysisSnapshot,
) -> Result<(), Error> {
    write_analysis_cache_value(output, analysis)
}

fn write_analysis_cache_value(
    output: &mut impl Write,
    value: &impl Serialize,
) -> Result<(), Error> {
    output
        .write_all(ANALYSIS_CACHE_MAGIC)
        .map_err(|error| Error::with_source("cannot write Analysis read cache header", error))?;
    let mut encoder = zstd::stream::write::Encoder::new(output, 1)
        .map_err(|error| Error::with_source("cannot create Analysis read cache encoder", error))?;
    rmp_serde::encode::write_named(&mut encoder, value)
        .map_err(|error| Error::with_source("cannot stream Analysis read cache", error))?;
    encoder
        .finish()
        .map_err(|error| Error::with_source("cannot finish Analysis read cache", error))?;
    Ok(())
}

pub(crate) fn analysis_cache_path(
    project_directory: &Path,
    identity: AnalysisSnapshotId,
    hash: &str,
) -> PathBuf {
    project_directory
        .join("cache")
        .join(format!("{identity}-{hash}.{ANALYSIS_CACHE_EXTENSION}"))
}

fn verify_manifest_blobs(path: &Path, manifest: &AnalysisManifest) -> Result<(), Error> {
    read_metadata(path, manifest)?;
    let blobs = blob_dir(path)?;
    for hash in &manifest.shape_blobs {
        read_verified_blob(&blobs.join(format!("{hash}.shape")), hash)?;
    }
    read_verified_blob(
        &blobs.join(format!("{}.values", manifest.values_blob)),
        &manifest.values_blob,
    )?;
    if let Some(base) = &manifest.values_base_blob {
        read_verified_blob(&blobs.join(format!("{base}.values")), base)?;
    }
    Ok(())
}

fn read_cached_analysis(path: &Path, manifest: &AnalysisManifest) -> Option<AnalysisSnapshot> {
    let project_directory = path.parent()?;
    let cache_directory = project_directory.join("cache");
    let prefix = format!("{}-", manifest.identity);
    let mut candidates = std::fs::read_dir(cache_directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| {
            candidate.extension().and_then(|value| value.to_str()) == Some(ANALYSIS_CACHE_EXTENSION)
                && candidate
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.starts_with(&prefix))
        })
        .collect::<Vec<_>>();
    candidates.sort();
    for candidate in candidates {
        let stem = candidate.file_stem()?.to_str()?;
        let expected = stem.strip_prefix(&prefix)?;
        if digest_file(&candidate).ok()?.as_str() != expected {
            continue;
        }
        let snapshot: AnalysisSnapshot =
            read_analysis_cache_value(File::open(&candidate).ok()?).ok()?;
        if snapshot.identity == manifest.identity
            && snapshot.project == manifest.project
            && snapshot.generated_at_unix_micros == manifest.generated_at_unix_micros
        {
            return Some(snapshot);
        }
    }
    None
}

fn read_analysis_cache_value<T: serde::de::DeserializeOwned>(input: impl Read) -> Result<T, Error> {
    let mut reader = BufReader::with_capacity(1024 * 1024, input);
    let mut magic = [0; ANALYSIS_CACHE_MAGIC.len()];
    reader
        .read_exact(&mut magic)
        .map_err(|error| Error::with_source("cannot read Analysis cache header", error))?;
    if magic != ANALYSIS_CACHE_MAGIC {
        return Err(Error::new("Analysis read cache format is corrupt"));
    }
    let decoder = zstd::stream::read::Decoder::new(reader)
        .map_err(|error| Error::with_source("cannot open Analysis read cache", error))?;
    decode_analysis_cache_value(decoder)
}

fn decode_analysis_cache_value<T: serde::de::DeserializeOwned>(
    input: impl Read,
) -> Result<T, Error> {
    // MessagePack requests small headers and scalar reads. Buffer the decoded
    // stream as well as the compressed input, keeping memory bounded while
    // avoiding a decompressor call for every field of a large graph.
    rmp_serde::from_read(BufReader::with_capacity(1024 * 1024, input))
        .map_err(|error| Error::with_source("cannot decode Analysis read cache", error))
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

pub(crate) fn digest_file(path: &Path) -> Result<String, Error> {
    let file = File::open(path)
        .map_err(|error| Error::with_source("cannot open Analysis cache for hashing", error))?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| Error::with_source("cannot hash Analysis cache", error))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn reusable_base_values(
    project_directory: &Path,
) -> Result<Option<(String, Vec<u8>)>, Error> {
    if !project_directory.exists() {
        return Ok(None);
    }
    let mut complete_bases = Vec::<(i64, String, String)>::new();
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
            complete_bases.push((
                manifest.generated_at_unix_micros,
                manifest.identity.to_string(),
                manifest.values_blob,
            ));
        }
    }
    complete_bases.sort_unstable_by(|left, right| right.cmp(left));
    let blobs = project_directory.join("blobs");
    for (_, _, values_hash) in complete_bases {
        let Ok(bytes) =
            read_verified_blob(&blobs.join(format!("{values_hash}.values")), &values_hash)
        else {
            continue;
        };
        let Ok(values) = unpack_blob(&bytes) else {
            continue;
        };
        return Ok(Some((values_hash, values)));
    }
    Ok(None)
}

fn pack_blob(input: &[u8]) -> Result<Vec<u8>, Error> {
    pack_blob_at_level(input, 1)
}

fn pack_blob_at_level(input: &[u8], level: i32) -> Result<Vec<u8>, Error> {
    let compressed = zstd::stream::encode_all(input, level)
        .map_err(|error| Error::with_source("cannot compress Analysis blob", error))?;
    let mut packed = Vec::with_capacity(BLOB_ZSTD_MAGIC.len() + 8 + compressed.len());
    packed.extend_from_slice(BLOB_ZSTD_MAGIC);
    put_u64(&mut packed, input.len() as u64);
    packed.extend_from_slice(&compressed);
    let mut raw = Vec::with_capacity(BLOB_RAW_MAGIC.len() + input.len());
    raw.extend_from_slice(BLOB_RAW_MAGIC);
    raw.extend_from_slice(input);
    if packed.len() < raw.len() {
        Ok(packed)
    } else {
        Ok(raw)
    }
}

fn unpack_blob(input: &[u8]) -> Result<Vec<u8>, Error> {
    if let Some(raw) = input.strip_prefix(BLOB_RAW_MAGIC) {
        return Ok(raw.to_vec());
    }
    let mut at = expect_magic(input, BLOB_ZSTD_MAGIC)?;
    let expected = usize::try_from(take_u64(input, &mut at)?)
        .map_err(|_| Error::new("Analysis packed blob length is unsupported"))?;
    let output = zstd::stream::decode_all(&input[at..])
        .map_err(|error| Error::with_source("cannot decompress Analysis blob", error))?;
    if output.len() != expected {
        return Err(Error::new("Analysis packed blob length is corrupt"));
    }
    Ok(output)
}

fn encode_delta(base: &[u8], current: &[u8]) -> Vec<u8> {
    let mut base_chunks = HashMap::<[u8; 32], Range<usize>>::new();
    for range in content_defined_ranges(base) {
        let hash = Sha256::digest(&base[range.clone()]).into();
        base_chunks.entry(hash).or_insert(range);
    }
    let mut operations = Vec::<DeltaOperation>::new();
    for range in content_defined_ranges(current) {
        let bytes = &current[range.clone()];
        let hash: [u8; 32] = Sha256::digest(bytes).into();
        if let Some(base_range) = base_chunks
            .get(&hash)
            .filter(|candidate| base[candidate.start..candidate.end] == *bytes)
        {
            match operations.last_mut() {
                Some(DeltaOperation::Copy { offset, length })
                    if *offset + *length == base_range.start =>
                {
                    *length += base_range.len();
                }
                _ => operations.push(DeltaOperation::Copy {
                    offset: base_range.start,
                    length: base_range.len(),
                }),
            }
        } else {
            match operations.last_mut() {
                Some(DeltaOperation::Literal(value)) => value.extend_from_slice(bytes),
                _ => operations.push(DeltaOperation::Literal(bytes.to_vec())),
            }
        }
    }
    let mut output = VALUES_DELTA_MAGIC.to_vec();
    put_u64(&mut output, current.len() as u64);
    put_u64(&mut output, operations.len() as u64);
    for operation in operations {
        match operation {
            DeltaOperation::Copy { offset, length } => {
                output.push(0);
                put_u64(&mut output, offset as u64);
                put_u64(&mut output, length as u64);
            }
            DeltaOperation::Literal(bytes) => {
                output.push(1);
                put_u64(&mut output, bytes.len() as u64);
                output.extend_from_slice(&bytes);
            }
        }
    }
    output
}

fn apply_delta(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, Error> {
    let mut at = expect_magic(delta, VALUES_DELTA_MAGIC)?;
    let length = take_u64(delta, &mut at)? as usize;
    let operation_count = take_u64(delta, &mut at)?;
    let mut output = Vec::with_capacity(length);
    for _ in 0..operation_count {
        let kind = *delta
            .get(at)
            .ok_or_else(|| Error::new("Analysis values delta operation is truncated"))?;
        at += 1;
        if kind == 0 {
            let offset = take_u64(delta, &mut at)? as usize;
            let count = take_u64(delta, &mut at)? as usize;
            let end = offset
                .checked_add(count)
                .filter(|end| *end <= base.len())
                .ok_or_else(|| Error::new("Analysis values delta base range is corrupt"))?;
            output.extend_from_slice(&base[offset..end]);
            continue;
        }
        if kind != 1 {
            return Err(Error::new("Analysis values delta operation is corrupt"));
        }
        let count = take_u64(delta, &mut at)? as usize;
        let source_end = at
            .checked_add(count)
            .filter(|end| *end <= delta.len())
            .ok_or_else(|| Error::new("Analysis values delta is truncated"))?;
        output.extend_from_slice(&delta[at..source_end]);
        at = source_end;
    }
    if at != delta.len() || output.len() != length {
        return Err(Error::new("Analysis values delta has trailing data"));
    }
    Ok(output)
}

fn content_defined_chunks(bytes: &[u8]) -> Vec<&[u8]> {
    content_defined_ranges(bytes)
        .into_iter()
        .map(|range| &bytes[range])
        .collect()
}

fn content_defined_ranges(bytes: &[u8]) -> Vec<Range<usize>> {
    let mut output = Vec::new();
    let mut start = 0;
    let mut rolling = 0_u64;
    for (index, byte) in bytes.iter().copied().enumerate() {
        rolling = rolling.rotate_left(1) ^ chunk_byte_hash(byte);
        if index >= CHUNK_WINDOW_BYTES {
            rolling ^= chunk_byte_hash(bytes[index - CHUNK_WINDOW_BYTES])
                .rotate_left(CHUNK_WINDOW_BYTES as u32);
        }
        let length = index + 1 - start;
        let boundary = rolling & 0x1fff == 0;
        if length >= CHUNK_MIN_BYTES && (boundary || length >= CHUNK_MAX_BYTES) {
            output.push(start..index + 1);
            start = index + 1;
        }
    }
    if start < bytes.len() {
        output.push(start..bytes.len());
    }
    output
}

fn chunk_byte_hash(byte: u8) -> u64 {
    let mut value = u64::from(byte).wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

enum DeltaOperation {
    Copy { offset: usize, length: usize },
    Literal(Vec<u8>),
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
    fn with_base_symbols(values: &[u8]) -> Result<Self, Error> {
        let mut at = expect_magic(values, VALUES_MAGIC)?;
        let symbol_count = take_u64(values, &mut at)?;
        let mut symbols = Vec::with_capacity(symbol_count as usize);
        let mut symbol_index = HashMap::with_capacity(symbol_count as usize);
        for index in 0..symbol_count {
            let length = take_u64(values, &mut at)? as usize;
            let end = at
                .checked_add(length)
                .filter(|end| *end <= values.len())
                .ok_or_else(|| Error::new("Analysis base symbol is truncated"))?;
            let symbol = values[at..end].to_vec();
            if symbol_index.insert(symbol.clone(), index as u32).is_some() {
                return Err(Error::new("Analysis base contains duplicate symbols"));
            }
            symbols.push(symbol);
            at = end;
        }
        let reference_count = take_u64(values, &mut at)? as usize;
        let reference_bytes = reference_count
            .checked_mul(std::mem::size_of::<u32>())
            .and_then(|count| at.checked_add(count))
            .filter(|end| *end == values.len())
            .ok_or_else(|| Error::new("Analysis base references are corrupt"))?;
        debug_assert_eq!(reference_bytes, values.len());
        Ok(Self {
            symbols,
            symbol_index,
            ..Self::default()
        })
    }

    fn finish_scalar(&mut self) -> std::io::Result<()> {
        let index = if let Some(index) = self.symbol_index.get(&self.token) {
            *index
        } else {
            let index = u32::try_from(self.symbols.len()).map_err(std::io::Error::other)?;
            self.symbol_index.insert(self.token.clone(), index);
            self.symbols.push(self.token.clone());
            index
        };
        // Repeated field names and values dominate large graphs. Keep the
        // scratch capacity instead of reallocating it for every scalar; interned
        // symbols own independent bytes and retain the same first-seen indices.
        self.token.clear();
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::ser::SerializeSeq;
    use serde_json::json;
    use std::{cell::Cell, rc::Rc};

    struct StreamingValue {
        count: u32,
        serialization_active: Rc<Cell<bool>>,
    }

    impl Serialize for StreamingValue {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.serialization_active.set(true);
            let mut sequence = serializer.serialize_seq(Some(self.count as usize))?;
            for index in 0..self.count {
                sequence.serialize_element(&format!(
                    "repository-entity-{index:08}-{}",
                    index.rotate_left(7)
                ))?;
            }
            let result = sequence.end();
            self.serialization_active.set(false);
            result
        }
    }

    struct ChunkObserver {
        bytes: Vec<u8>,
        largest_write: usize,
        serialization_active: Rc<Cell<bool>>,
        wrote_during_serialization: bool,
    }

    impl Write for ChunkObserver {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.largest_write = self.largest_write.max(bytes.len());
            self.wrote_during_serialization |= self.serialization_active.get();
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn analysis_cache_round_trips_through_bounded_stream_writes() -> Result<(), Error> {
        let serialization_active = Rc::new(Cell::new(false));
        let value = StreamingValue {
            count: 120_000,
            serialization_active: Rc::clone(&serialization_active),
        };
        let logical_bytes = value.count as usize * "repository-entity-00000000-00000000".len();
        let mut output = ChunkObserver {
            bytes: Vec::new(),
            largest_write: 0,
            serialization_active,
            wrote_during_serialization: false,
        };
        write_analysis_cache_value(&mut output, &value)?;
        assert!(output.bytes.len() > 256 * 1024);
        assert!(
            output.wrote_during_serialization,
            "cache publication must write while serializing instead of retaining a complete encoded snapshot"
        );
        assert!(
            output.largest_write < output.bytes.len(),
            "cache publication must not hand one complete encoded snapshot to its writer"
        );
        assert!(
            output.largest_write * 8 < logical_bytes,
            "cache writes must stay bounded relative to the logical snapshot"
        );
        let decoded: Vec<String> = read_analysis_cache_value(output.bytes.as_slice())?;
        assert_eq!(decoded.len(), value.count as usize);
        assert_eq!(
            decoded.first().map(String::as_str),
            Some("repository-entity-00000000-0")
        );
        assert_eq!(
            decoded.last().map(String::as_str),
            Some("repository-entity-00119999-15359872")
        );

        let legacy_value = vec!["legacy cache payload"; 32];
        let legacy = pack_blob(&rmp_serde::to_vec_named(&legacy_value).map_err(|error| {
            Error::with_source("cannot create legacy cache test value", error)
        })?)?;
        assert!(read_analysis_cache_value::<Vec<String>>(legacy.as_slice()).is_err());
        Ok(())
    }

    #[test]
    fn analysis_cache_batches_decoded_reads_and_rejects_truncation() -> Result<(), Error> {
        struct ObservedRead<'a> {
            remaining: &'a [u8],
            calls: Rc<Cell<usize>>,
            largest_read: Rc<Cell<usize>>,
        }

        impl Read for ObservedRead<'_> {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                self.calls.set(self.calls.get() + 1);
                self.largest_read
                    .set(self.largest_read.get().max(output.len()));
                self.remaining.read(output)
            }
        }

        let values = (0..120_000)
            .map(|index| format!("repository-entity-{index:08}"))
            .collect::<Vec<_>>();
        let encoded = rmp_serde::to_vec_named(&values)
            .map_err(|error| Error::with_source("cannot encode cache read fixture", error))?;
        let calls = Rc::new(Cell::new(0));
        let largest_read = Rc::new(Cell::new(0));
        let decoded: Vec<String> = decode_analysis_cache_value(ObservedRead {
            remaining: &encoded,
            calls: Rc::clone(&calls),
            largest_read: Rc::clone(&largest_read),
        })?;
        assert_eq!(decoded, values);
        assert!(calls.get() <= encoded.len() / (1024 * 1024) + 2);
        assert_eq!(largest_read.get(), 1024 * 1024);
        assert!(decode_analysis_cache_value::<Vec<String>>(&encoded[..encoded.len() - 1]).is_err());

        let mut compressed = Vec::new();
        write_analysis_cache_value(&mut compressed, &values)?;
        assert!(
            read_analysis_cache_value::<Vec<String>>(&compressed[..compressed.len() / 2]).is_err()
        );
        assert!(read_analysis_cache_value::<Vec<u64>>(compressed.as_slice()).is_err());
        Ok(())
    }

    #[test]
    fn normalized_scalar_reuse_preserves_exact_json_and_symbol_indices() -> Result<(), Error> {
        let mut scalar_writer = NormalizingWriter::default();
        scalar_writer
            .write_all(br#"["repeat","unique","repeat",true,false,true,null,null,-12,-12]"#)
            .map_err(|error| Error::with_source("cannot normalize scalar index fixture", error))?;
        let mut expected_values = VALUES_MAGIC.to_vec();
        put_u64(&mut expected_values, 6);
        for symbol in [
            b"\"repeat\"".as_slice(),
            b"\"unique\"",
            b"true",
            b"false",
            b"null",
            b"-12",
        ] {
            put_u64(&mut expected_values, symbol.len() as u64);
            expected_values.extend_from_slice(symbol);
        }
        put_u64(&mut expected_values, 10);
        for index in [0_u32, 1, 0, 2, 3, 2, 4, 4, 5, 5] {
            expected_values.extend_from_slice(&index.to_le_bytes());
        }
        assert_eq!(scalar_writer.finish()?.values, expected_values);

        let value = json!({
            "escaped": ["quote\"slash\\line\n", "한글 λ", "quote\"slash\\line\n"],
            "numbers": [0, -12, 1.25, 0, -12],
            "nested": [{"state": true, "value": null}, {"state": false, "value": null}],
            "long": ["x".repeat(64 * 1024), "short", "x".repeat(64 * 1024)]
        });
        let original = serde_json::to_vec(&value)
            .map_err(|error| Error::with_source("cannot encode scalar reuse fixture", error))?;
        let mut writer = NormalizingWriter::default();
        // Cross scalar and UTF-8 boundaries independently of serializer writes.
        for chunk in original.chunks(7) {
            writer.write_all(chunk).map_err(|error| {
                Error::with_source("cannot normalize scalar reuse fixture", error)
            })?;
        }
        let normalized = writer.finish()?;
        let mut reader =
            DenormalizingReader::new(normalized.shape, normalized.values, normalized.scalar_count)?;
        let mut decoded = Vec::new();
        reader.read_to_end(&mut decoded).map_err(|error| {
            Error::with_source("cannot reconstruct scalar reuse fixture", error)
        })?;
        assert_eq!(decoded, original);
        Ok(())
    }

    #[test]
    fn content_delta_resynchronizes_after_insertions() -> Result<(), Error> {
        let mut state = 0x1234_5678_9abc_def0_u64;
        let base = (0..240_000)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                b"{[$,:]}"[((state >> 61) % 7) as usize]
            })
            .collect::<Vec<_>>();
        let mut current = base.clone();
        current.splice(71_111..71_111, b"[$,$,$,$]".iter().copied());
        current.splice(181_000..181_021, b"{[$]}".iter().copied());

        let delta = encode_delta(&base, &current);
        assert_eq!(apply_delta(&base, &delta)?, current);
        assert!(
            delta.len() * 3 < current.len(),
            "shifted content should mostly reuse base chunks: delta={}, current={}",
            delta.len(),
            current.len()
        );
        Ok(())
    }

    #[test]
    fn compressed_metadata_preserves_snapshot_and_rejects_corruption(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let repository = temporary.path().join("repository");
        std::fs::create_dir_all(&repository)?;
        std::fs::write(
            repository.join("example.py"),
            "def answer():\n    return 42\n",
        )?;
        let operations = crate::LocalOperations::new(crate::RuntimeLayout::new(
            temporary.path().join("runtime"),
        )?);
        let project = operations
            .initialize_project("Metadata roundtrip", Some(&repository))?
            .project
            .id;
        let outcome = operations
            .analyze(project, Vec::new())?
            .value
            .ok_or("analysis")?;
        let manifest = read_manifest(&outcome.stored_at)?;
        let metadata = read_metadata(&outcome.stored_at, &manifest)?;
        assert_eq!(
            metadata,
            volicord_repository_intelligence::AnalysisMetadata::from(&outcome.analysis)
        );
        let durable = read_analysis_durable(&outcome.stored_at)?;
        assert_eq!(
            serde_json::to_value(&durable)?,
            serde_json::to_value(&outcome.analysis)?,
            "durable metadata relocation must preserve the complete semantic graph"
        );
        let metadata_path =
            blob_dir(&outcome.stored_at)?.join(format!("{}.metadata", manifest.metadata_blob));
        std::fs::write(metadata_path, b"corrupt metadata")?;
        assert!(read_analysis(&outcome.stored_at).is_err());
        assert!(read_analysis_durable(&outcome.stored_at).is_err());
        Ok(())
    }

    #[test]
    fn seeded_symbols_keep_shifted_reference_stream_reusable() -> Result<(), Error> {
        let base_value = (0..20_000)
            .map(|index| json!({"name": format!("item-{index}"), "state": "stable"}))
            .collect::<Vec<_>>();
        let mut base_writer = NormalizingWriter::default();
        serde_json::to_writer(&mut base_writer, &base_value)
            .map_err(|error| Error::with_source("cannot encode base test value", error))?;
        let base = base_writer.finish()?.values;

        let mut current_value = base_value;
        current_value.insert(7_777, json!({"name": "inserted", "state": "changed"}));
        let mut current_writer = NormalizingWriter::with_base_symbols(&base)?;
        serde_json::to_writer(&mut current_writer, &current_value)
            .map_err(|error| Error::with_source("cannot encode current test value", error))?;
        let current = current_writer.finish()?.values;

        let delta = encode_delta(&base, &current);
        assert_eq!(apply_delta(&base, &delta)?, current);
        assert!(
            delta.len() * 4 < current.len(),
            "stable symbol indexes should keep reference insertion deltas bounded"
        );
        Ok(())
    }
}
