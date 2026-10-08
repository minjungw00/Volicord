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
    rc::Rc,
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

pub(crate) struct PackedAnalysisValues {
    hash: String,
    packed: Vec<u8>,
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
    base_values: Option<PackedAnalysisValues>,
) -> Result<EncodedAnalysis, Error> {
    let mut encoder = match &base_values {
        Some(base) => NormalizingWriter::with_base_symbols(&unpack_blob(&base.packed)?)?,
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
    let (values, values_base_blob) = if let Some(base) = base_values {
        // Normalization only needs the base symbols. Keep the full reference
        // stream packed until delta generation, after encoder scratch is gone.
        let unpacked = unpack_blob(&base.packed)?;
        let base_hash = base.hash;
        drop(base.packed);
        let delta = pack_delta(&unpacked, &normalized.values)?;
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
    let snapshot: AnalysisSnapshot = decode_normalized_json(reader, manifest.logical_json_bytes)?;
    if snapshot.identity != manifest.identity
        || snapshot.project != manifest.project
        || snapshot.generated_at_unix_micros != manifest.generated_at_unix_micros
    {
        return Err(Error::new("Analysis manifest and payload bindings differ"));
    }
    Ok(snapshot)
}

fn decode_normalized_json<T: serde::de::DeserializeOwned>(
    reader: impl Read,
    logical_bytes: u64,
) -> Result<T, Error> {
    let length = usize::try_from(logical_bytes)
        .map_err(|error| Error::with_source("Analysis JSON length is unsupported", error))?;
    let mut json = Vec::new();
    json.try_reserve_exact(length)
        .map_err(|error| Error::with_source("cannot allocate Analysis JSON read buffer", error))?;
    // Release shape/value buffers before allocating the typed graph. Slice decoding
    // scans strings in bulk; even a BufReader leaves serde_json's generic reader
    // parser visiting every byte individually in gigabyte-sized durable inspections.
    {
        let mut input = reader.take(logical_bytes.saturating_add(1));
        input.read_to_end(&mut json).map_err(|error| {
            Error::with_source("cannot reconstruct normalized Analysis JSON", error)
        })?;
    }
    if json.len() != length {
        return Err(Error::new("Analysis JSON length differs from manifest"));
    }
    serde_json::from_slice(&json)
        .map_err(|error| Error::with_source("normalized Analysis Snapshot is corrupt", error))
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
    // A large graph repeats facts across sections farther apart than the
    // default window. Reuse them without raising the compression level.
    encoder
        .window_log(26)
        .and_then(|_| encoder.long_distance_matching(true))
        .map_err(|error| {
            Error::with_source("cannot configure Analysis read cache encoder", error)
        })?;
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
) -> Result<Option<PackedAnalysisValues>, Error> {
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
        if unpack_blob(&bytes).is_err() {
            continue;
        }
        return Ok(Some(PackedAnalysisValues {
            hash: values_hash,
            packed: bytes,
        }));
    }
    Ok(None)
}

fn pack_blob(input: &[u8]) -> Result<Vec<u8>, Error> {
    pack_blob_at_level(input, 1)
}

fn pack_blob_at_level(input: &[u8], level: i32) -> Result<Vec<u8>, Error> {
    let compressed = zstd::stream::encode_all(input, level)
        .map_err(|error| Error::with_source("cannot compress Analysis blob", error))?;
    if BLOB_ZSTD_MAGIC.len() + 8 + compressed.len() < BLOB_RAW_MAGIC.len() + input.len() {
        let mut packed = Vec::with_capacity(BLOB_ZSTD_MAGIC.len() + 8 + compressed.len());
        packed.extend_from_slice(BLOB_ZSTD_MAGIC);
        put_u64(&mut packed, input.len() as u64);
        packed.extend_from_slice(&compressed);
        return Ok(packed);
    }
    // Large value streams normally compress. Do not allocate their full raw
    // copy merely to compare lengths; release the rejected representation first.
    drop(compressed);
    let mut raw = Vec::with_capacity(BLOB_RAW_MAGIC.len() + input.len());
    raw.extend_from_slice(BLOB_RAW_MAGIC);
    raw.extend_from_slice(input);
    Ok(raw)
}

fn unpack_blob(input: &[u8]) -> Result<Vec<u8>, Error> {
    if let Some(raw) = input.strip_prefix(BLOB_RAW_MAGIC) {
        return Ok(raw.to_vec());
    }
    let mut at = expect_magic(input, BLOB_ZSTD_MAGIC)?;
    let expected = usize::try_from(take_u64(input, &mut at)?)
        .map_err(|_| Error::new("Analysis packed blob length is unsupported"))?;
    let mut decoder = zstd::stream::read::Decoder::new(&input[at..])
        .map_err(|error| Error::with_source("cannot decompress Analysis blob", error))?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(expected)
        .map_err(|error| Error::with_source("cannot allocate unpacked Analysis blob", error))?;
    // The verified envelope states the decoded length. Avoid geometric growth
    // of a hundreds-of-megabytes base, while still consuming and validating EOF.
    decoder
        .by_ref()
        .take(expected as u64)
        .read_to_end(&mut output)
        .map_err(|error| Error::with_source("cannot decompress Analysis blob", error))?;
    let trailing = decoder
        .read(&mut [0_u8; 1])
        .map_err(|error| Error::with_source("cannot decompress Analysis blob", error))?;
    if output.len() != expected || trailing != 0 {
        return Err(Error::new("Analysis packed blob length is corrupt"));
    }
    Ok(output)
}

fn delta_operations(base: &[u8], current: &[u8]) -> Vec<DeltaOperation> {
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
                Some(DeltaOperation::Literal(value)) => value.end = range.end,
                _ => operations.push(DeltaOperation::Literal(range)),
            }
        }
    }
    operations
}

fn delta_length(operations: &[DeltaOperation]) -> usize {
    VALUES_DELTA_MAGIC.len()
        + 16
        + operations
            .iter()
            .map(|operation| match operation {
                DeltaOperation::Copy { .. } => 17,
                DeltaOperation::Literal(range) => 9 + range.len(),
            })
            .sum::<usize>()
}

fn write_delta(
    output: &mut impl Write,
    current: &[u8],
    operations: &[DeltaOperation],
) -> std::io::Result<()> {
    output.write_all(VALUES_DELTA_MAGIC)?;
    output.write_all(&(current.len() as u64).to_le_bytes())?;
    output.write_all(&(operations.len() as u64).to_le_bytes())?;
    for operation in operations {
        match operation {
            DeltaOperation::Copy { offset, length } => {
                output.write_all(&[0])?;
                output.write_all(&(*offset as u64).to_le_bytes())?;
                output.write_all(&(*length as u64).to_le_bytes())?;
            }
            DeltaOperation::Literal(range) => {
                output.write_all(&[1])?;
                output.write_all(&(range.len() as u64).to_le_bytes())?;
                output.write_all(&current[range.clone()])?;
            }
        }
    }
    Ok(())
}

fn pack_delta(base: &[u8], current: &[u8]) -> Result<Vec<u8>, Error> {
    let operations = delta_operations(base, current);
    let length = delta_length(&operations);
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 1)
        .map_err(|error| Error::with_source("cannot compress Analysis delta", error))?;
    // Emit directly into compression instead of allocating the entire decoded
    // delta alongside the baseline, current graph and both value streams.
    write_delta(&mut encoder, current, &operations)
        .map_err(|error| Error::with_source("cannot encode Analysis delta", error))?;
    let compressed = encoder
        .finish()
        .map_err(|error| Error::with_source("cannot finish Analysis delta compression", error))?;
    if BLOB_ZSTD_MAGIC.len() + 8 + compressed.len() < BLOB_RAW_MAGIC.len() + length {
        let mut output = Vec::with_capacity(BLOB_ZSTD_MAGIC.len() + 8 + compressed.len());
        output.extend_from_slice(BLOB_ZSTD_MAGIC);
        put_u64(&mut output, length as u64);
        output.extend_from_slice(&compressed);
        return Ok(output);
    }
    drop(compressed);
    let mut output = Vec::with_capacity(BLOB_RAW_MAGIC.len() + length);
    output.extend_from_slice(BLOB_RAW_MAGIC);
    write_delta(&mut output, current, &operations)
        .map_err(|error| Error::with_source("cannot encode raw Analysis delta", error))?;
    Ok(output)
}

#[cfg(test)]
fn encode_delta(base: &[u8], current: &[u8]) -> Result<Vec<u8>, Error> {
    let operations = delta_operations(base, current);
    let mut output = Vec::with_capacity(delta_length(&operations));
    write_delta(&mut output, current, &operations)
        .map_err(|error| Error::with_source("cannot encode test Analysis delta", error))?;
    Ok(output)
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
    Literal(Range<usize>),
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
    symbols: Vec<Rc<[u8]>>,
    symbol_index: HashMap<Rc<[u8]>, u32>,
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
            let symbol = Rc::<[u8]>::from(&values[at..end]);
            if symbol_index
                .insert(Rc::clone(&symbol), index as u32)
                .is_some()
            {
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
        let index = if let Some(index) = self.symbol_index.get(self.token.as_slice()) {
            *index
        } else {
            let index = u32::try_from(self.symbols.len()).map_err(std::io::Error::other)?;
            let symbol = Rc::<[u8]>::from(self.token.as_slice());
            self.symbol_index.insert(Rc::clone(&symbol), index);
            self.symbols.push(symbol);
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
        // Finish in the existing shape allocation and reserve the exact value
        // stream length. Holding copied shapes or a geometrically grown values
        // buffer alongside baseline/current graphs raises long-lived MCP peaks.
        drop(self.symbol_index);
        let scalar_count = self.references.len() as u64;
        let shape_length = self.shape.len();
        let header_length = SHAPE_MAGIC.len() + 8;
        let mut shape = self.shape;
        shape
            .try_reserve_exact(header_length)
            .map_err(|error| Error::with_source("cannot allocate Analysis shape header", error))?;
        shape.resize(shape_length + header_length, 0);
        shape.copy_within(..shape_length, header_length);
        shape[..SHAPE_MAGIC.len()].copy_from_slice(SHAPE_MAGIC);
        shape[SHAPE_MAGIC.len()..header_length]
            .copy_from_slice(&(shape_length as u64).to_le_bytes());
        let value_length = self
            .symbols
            .iter()
            .try_fold(VALUES_MAGIC.len() + 16, |length, symbol| {
                length.checked_add(8)?.checked_add(symbol.len())
            })
            .and_then(|length| length.checked_add(self.references.len().checked_mul(4)?))
            .ok_or_else(|| Error::new("Analysis value stream length is unsupported"))?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(value_length)
            .map_err(|error| Error::with_source("cannot allocate Analysis value stream", error))?;
        values.extend_from_slice(VALUES_MAGIC);
        put_u64(&mut values, self.symbols.len() as u64);
        for symbol in self.symbols {
            put_u64(&mut values, symbol.len() as u64);
            values.extend_from_slice(&symbol);
        }
        put_u64(&mut values, self.references.len() as u64);
        for reference in self.references {
            values.extend_from_slice(&reference.to_le_bytes());
        }
        Ok(Normalized {
            shape,
            values,
            logical_bytes: self.logical_bytes,
            scalar_count,
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
    // Keep symbols and references in their verified value stream. Large graphs
    // reuse these bytes millions of times; copying them is unnecessary.
    values: Vec<u8>,
    symbols: Vec<Range<usize>>,
    shape_at: usize,
    reference_at: usize,
    pending: Option<Range<usize>>,
}

impl DenormalizingReader {
    fn new(shape: Vec<u8>, values: Vec<u8>, expected_scalars: u64) -> Result<Self, Error> {
        let mut shape_at = expect_magic(&shape, SHAPE_MAGIC)?;
        let shape_len = take_u64(&shape, &mut shape_at)? as usize;
        let shape_end = shape_at
            .checked_add(shape_len)
            .filter(|end| *end == shape.len())
            .ok_or_else(|| Error::new("Analysis shape length is corrupt"))?;
        debug_assert_eq!(shape_end, shape.len());
        let mut at = expect_magic(&values, VALUES_MAGIC)?;
        let symbol_count = take_u64(&values, &mut at)? as usize;
        let mut symbols = Vec::with_capacity(symbol_count);
        for _ in 0..symbol_count {
            let length = take_u64(&values, &mut at)? as usize;
            let end = at
                .checked_add(length)
                .filter(|end| *end <= values.len())
                .ok_or_else(|| Error::new("Analysis symbol length is corrupt"))?;
            symbols.push(at..end);
            at = end;
        }
        let reference_count = take_u64(&values, &mut at)?;
        if reference_count != expected_scalars {
            return Err(Error::new("Analysis scalar count differs from manifest"));
        }
        let reference_bytes = usize::try_from(reference_count)
            .ok()
            .and_then(|count| count.checked_mul(4))
            .and_then(|bytes| at.checked_add(bytes))
            .ok_or_else(|| Error::new("Analysis scalar reference is truncated"))?;
        if reference_bytes > values.len() {
            return Err(Error::new("Analysis scalar reference is truncated"));
        }
        if reference_bytes != values.len()
            || values[at..].chunks_exact(4).any(|bytes| {
                u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize
                    >= symbols.len()
            })
        {
            return Err(Error::new(
                "Analysis values contain trailing or invalid references",
            ));
        }
        Ok(Self {
            shape,
            values,
            symbols,
            shape_at,
            reference_at: at,
            pending: None,
        })
    }
}

impl Read for DenormalizingReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let mut written = 0;
        while written < output.len() {
            if let Some(pending) = &mut self.pending {
                let count = (output.len() - written).min(pending.len());
                output[written..written + count]
                    .copy_from_slice(&self.values[pending.start..pending.start + count]);
                pending.start += count;
                written += count;
                if pending.start == pending.end {
                    self.pending = None;
                }
                continue;
            }
            let Some(&token) = self.shape.get(self.shape_at) else {
                break;
            };
            self.shape_at += 1;
            if token == b'$' {
                let bytes = self.values[self.reference_at..]
                    .get(..4)
                    .ok_or_else(|| std::io::Error::other("missing Analysis scalar reference"))?;
                let index = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
                self.reference_at += 4;
                let symbol = self
                    .symbols
                    .get(index)
                    .ok_or_else(|| std::io::Error::other("invalid Analysis scalar reference"))?;
                let count = (output.len() - written).min(symbol.len());
                output[written..written + count]
                    .copy_from_slice(&self.values[symbol.start..symbol.start + count]);
                written += count;
                if count < symbol.len() {
                    self.pending = Some(symbol.start + count..symbol.end);
                }
            } else {
                output[written] = token;
                written += 1;
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

    #[test]
    fn packed_blobs_preserve_both_formats_and_reject_corrupt_lengths() -> Result<(), Error> {
        for input in [b"".as_slice(), b"x"] {
            let packed = pack_blob(input)?;
            let mut expected = BLOB_RAW_MAGIC.to_vec();
            expected.extend_from_slice(input);
            assert_eq!(packed, expected);
            assert_eq!(unpack_blob(&packed)?, input);
        }
        let input = vec![b'x'; 64 * 1024];
        let packed = pack_blob(&input)?;
        assert!(packed.starts_with(BLOB_ZSTD_MAGIC));
        assert_eq!(unpack_blob(&packed)?, input);
        assert_eq!(pack_blob(&input)?, packed);
        let mut wrong_length = packed.clone();
        wrong_length[BLOB_ZSTD_MAGIC.len()..BLOB_ZSTD_MAGIC.len() + 8]
            .copy_from_slice(&(input.len() as u64 + 1).to_le_bytes());
        assert!(unpack_blob(&wrong_length).is_err());
        let mut short_length = packed.clone();
        short_length[BLOB_ZSTD_MAGIC.len()..BLOB_ZSTD_MAGIC.len() + 8]
            .copy_from_slice(&(input.len() as u64 - 1).to_le_bytes());
        assert!(unpack_blob(&short_length).is_err());
        assert!(unpack_blob(&packed[..packed.len() - 1]).is_err());
        Ok(())
    }

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
    fn analysis_cache_reuses_distant_repeated_sections_without_losing_values() -> Result<(), Error>
    {
        let mut state = 1_u64;
        let block = (0..150_000)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                state
            })
            .collect::<Vec<_>>();
        let value = block.repeat(3);
        let original = rmp_serde::to_vec_named(&value)
            .map_err(|error| Error::with_source("cannot encode distant cache fixture", error))?;
        let default = zstd::stream::encode_all(original.as_slice(), 1)
            .map_err(|error| Error::with_source("cannot compress default cache fixture", error))?;
        let mut packed = Vec::new();
        write_analysis_cache_value(&mut packed, &value)?;
        assert!(
            packed.len() * 2 < default.len(),
            "distant sections were stored repeatedly"
        );
        let decoded: Vec<u64> = read_analysis_cache_value(packed.as_slice())?;
        assert_eq!(decoded, value);
        Ok(())
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
        let normalized = scalar_writer.finish()?;
        let shape_tokens = b"[$,$,$,$,$,$,$,$,$,$]";
        let mut expected_shape = SHAPE_MAGIC.to_vec();
        put_u64(&mut expected_shape, shape_tokens.len() as u64);
        expected_shape.extend_from_slice(shape_tokens);
        assert_eq!(normalized.shape, expected_shape);
        assert_eq!(normalized.values, expected_values);

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
    fn normalized_reader_preserves_chunked_large_symbol_table(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let value = (0..120_000)
            .map(|index| {
                json!({"name": format!("repository-entity-{index:08}"),
                    "repeat": "source-grounded", "unicode": "한글 λ",
                    "escaped": "quote\"slash\\line\n", "number": index, "value": null})
            })
            .collect::<Vec<_>>();
        let encoded = serde_json::to_vec(&value)
            .map_err(|error| Error::with_source("cannot encode normalized read fixture", error))?;
        let mut writer = NormalizingWriter::default();
        writer
            .write_all(&encoded)
            .map_err(|error| Error::with_source("cannot normalize read fixture", error))?;
        let normalized = writer.finish()?;
        let started = std::time::Instant::now();
        let reader =
            DenormalizingReader::new(normalized.shape, normalized.values, normalized.scalar_count)?;
        let decoded: Vec<serde_json::Value> = decode_normalized_json(reader, encoded.len() as u64)?;
        println!(
            "normalized large-symbol decode ms: {}",
            started.elapsed().as_millis()
        );
        assert_eq!(decoded, value);

        let encoded = r#"["a","long-한글","a",true,false,null,-12]"#.as_bytes();
        for chunk_size in [1, 2, 7, 31, 1024] {
            let mut writer = NormalizingWriter::default();
            writer
                .write_all(encoded)
                .map_err(|error| Error::with_source("cannot normalize chunk fixture", error))?;
            let normalized = writer.finish()?;
            let mut reader = DenormalizingReader::new(
                normalized.shape,
                normalized.values,
                normalized.scalar_count,
            )?;
            let mut decoded = Vec::new();
            let mut buffer = vec![0; chunk_size];
            assert_eq!(reader.read(&mut [])?, 0);
            loop {
                let count = reader.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                decoded.extend_from_slice(&buffer[..count]);
            }
            assert_eq!(decoded, encoded);
        }
        Ok(())
    }

    #[test]
    fn normalized_json_decoder_rejects_length_schema_and_trailing_corruption() -> Result<(), Error>
    {
        let encoded = br#"["a","b"]"#;
        for length in [encoded.len() as u64 - 1, encoded.len() as u64 + 1] {
            assert!(decode_normalized_json::<Vec<String>>(encoded.as_slice(), length).is_err());
        }
        for corrupt in [br#"["a",2]"#.as_slice(), br#"["a"]true"#.as_slice()] {
            assert!(decode_normalized_json::<Vec<String>>(corrupt, corrupt.len() as u64).is_err());
        }
        Ok(())
    }

    #[test]
    fn normalized_reader_rejects_corrupt_scalar_references() -> Result<(), Error> {
        let mut writer = NormalizingWriter::default();
        writer
            .write_all(br#"["a","b","a"]"#)
            .map_err(|error| Error::with_source("cannot normalize reference fixture", error))?;
        let normalized = writer.finish()?;
        let reader =
            |values, count| DenormalizingReader::new(normalized.shape.clone(), values, count);
        assert!(reader(normalized.values.clone(), normalized.scalar_count + 1).is_err());
        assert!(reader(
            normalized.values[..normalized.values.len() - 1].to_vec(),
            normalized.scalar_count
        )
        .is_err());
        let mut trailing = normalized.values.clone();
        trailing.push(0);
        assert!(reader(trailing, normalized.scalar_count).is_err());
        let mut invalid_index = normalized.values.clone();
        let last_reference = invalid_index.len() - 4;
        invalid_index[last_reference..].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(reader(invalid_index, normalized.scalar_count).is_err());

        let mut extra_scalar_shape = normalized.shape.clone();
        extra_scalar_shape.push(b'$');
        let length = extra_scalar_shape.len() - SHAPE_MAGIC.len() - 8;
        extra_scalar_shape[SHAPE_MAGIC.len()..SHAPE_MAGIC.len() + 8]
            .copy_from_slice(&(length as u64).to_le_bytes());
        let mut missing_reference = DenormalizingReader::new(
            extra_scalar_shape,
            normalized.values,
            normalized.scalar_count,
        )?;
        assert!(missing_reference.read_to_end(&mut Vec::new()).is_err());
        Ok(())
    }

    #[test]
    fn streamed_delta_matches_retained_raw_and_compressed_bytes() -> Result<(), Error> {
        let mut state = 1_u64;
        let noisy = (0..64 * 1024)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (state >> 56) as u8
            })
            .collect::<Vec<_>>();
        let mut raw_seen = false;
        let mut compressed_seen = false;
        for current in [Vec::new(), noisy, vec![b'x'; 128 * 1024]] {
            let plain = encode_delta(&[], &current)?;
            let packed = pack_delta(&[], &current)?;
            assert_eq!(packed, pack_blob(&plain)?);
            assert_eq!(unpack_blob(&packed)?, plain);
            assert_eq!(apply_delta(&[], &plain)?, current);
            raw_seen |= packed.starts_with(BLOB_RAW_MAGIC);
            compressed_seen |= packed.starts_with(BLOB_ZSTD_MAGIC);
        }
        assert!(raw_seen && compressed_seen);
        let base = vec![b's'; CHUNK_MAX_BYTES * 4];
        let mut current = base.clone();
        current.splice(17_777..17_777, b"changed-source".iter().copied());
        assert_eq!(
            pack_delta(&base, &current)?,
            pack_blob(&encode_delta(&base, &current)?)?
        );
        Ok(())
    }

    #[test]
    fn delta_literals_coalesce_without_changing_wire_bytes() -> Result<(), Error> {
        let current = vec![b'x'; CHUNK_MAX_BYTES * 3 + 17];
        let delta = encode_delta(&[], &current)?;
        let mut expected = VALUES_DELTA_MAGIC.to_vec();
        put_u64(&mut expected, current.len() as u64);
        put_u64(&mut expected, 1);
        expected.push(1);
        put_u64(&mut expected, current.len() as u64);
        expected.extend_from_slice(&current);
        assert_eq!(delta, expected);
        assert_eq!(apply_delta(&[], &delta)?, current);
        assert!(apply_delta(&[], &delta[..delta.len() - 1]).is_err());
        let mut invalid_kind = delta;
        invalid_kind[VALUES_DELTA_MAGIC.len() + 16] = 2;
        assert!(apply_delta(&[], &invalid_kind).is_err());
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

        let delta = encode_delta(&base, &current)?;
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
        for length in [
            manifest.logical_json_bytes - 1,
            manifest.logical_json_bytes + 1,
        ] {
            let mut changed = manifest.clone();
            changed.logical_json_bytes = length;
            std::fs::write(&outcome.stored_at, serde_json::to_vec(&changed)?)?;
            assert!(read_analysis_durable(&outcome.stored_at).is_err());
        }
        std::fs::write(&outcome.stored_at, serde_json::to_vec(&manifest)?)?;
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

        let delta = encode_delta(&base, &current)?;
        assert_eq!(apply_delta(&base, &delta)?, current);
        assert!(
            delta.len() * 4 < current.len(),
            "stable symbol indexes should keep reference insertion deltas bounded"
        );
        Ok(())
    }
}
