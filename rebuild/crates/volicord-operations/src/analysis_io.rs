//! Current-only, structurally shared Analysis Snapshot persistence.
mod normalized;
mod symbols;
use crate::Error;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
};
use volicord_context::ProjectId;
use volicord_repository_intelligence::{
    AnalysisMetadata, AnalysisSnapshot, AnalysisSnapshotId, CanonicalProjectRef,
    RepositoryChangeBasis, ANALYSIS_SNAPSHOT_FORMAT_VERSION, ANALYSIS_SNAPSHOT_KIND,
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
const SHAPE_COMPRESSION_LEVEL: i32 = 9;

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
    dictionary: Option<symbols::Dictionary>,
    shapes: HashMap<String, (String, Vec<u8>)>,
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
    mut base_values: Option<PackedAnalysisValues>,
) -> Result<EncodedAnalysis, Error> {
    #[cfg(test)]
    trace_analysis_memory("encode_start");
    // First-seen order belongs to this complete snapshot. Preloading an older
    // symbol dictionary shifts fresh observation identities to its tail and
    // changes otherwise identical reference runs throughout a large graph.
    // The existing byte-delta codec still reuses the retained complete base.
    let mut encoder = NormalizingWriter::default();
    #[cfg(test)]
    trace_analysis_memory("base_symbols");
    serde_json::to_writer(&mut encoder, analysis)
        .map_err(|error| Error::with_source("cannot normalize Analysis Snapshot", error))?;
    #[cfg(test)]
    trace_analysis_memory("normalized_writer");
    let normalized = encoder.finish_parts()?;
    #[cfg(test)]
    trace_analysis_memory("normalized_parts_finished");
    let reusable_shapes = base_values.as_ref().map(|base| &base.shapes);
    let shapes = content_defined_chunks(&normalized.shape)
        .into_iter()
        .map(|chunk| pack_shape(chunk, reusable_shapes))
        .collect::<Result<Vec<_>, Error>>()?;
    // Only packed chunks are used from here. Do not retain the full shape
    // allocation while expanding the base and building the value delta.
    drop(normalized.shape);
    #[cfg(test)]
    trace_analysis_memory("shapes_packed");
    let pending = match base_values.as_mut().and_then(|base| base.dictionary.take()) {
        Some(dictionary) => dictionary.apply(normalized.values)?,
        None => normalized.values,
    };
    let uncompressed_values = pending.finish()?;
    #[cfg(test)]
    trace_analysis_memory("values_expanded_after_shape_release");
    let (values, values_base_blob) = pack_analysis_values(&uncompressed_values, base_values)?;
    drop(uncompressed_values);
    let values_hash = digest(&values);
    let metadata = pack_metadata(
        &serde_json::to_vec(&AnalysisMetadata::from(analysis))
            .map_err(|error| Error::with_source("cannot encode Analysis metadata", error))?,
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

fn pack_shape(
    chunk: &[u8],
    reusable: Option<&HashMap<String, (String, Vec<u8>)>>,
) -> Result<(String, Vec<u8>), Error> {
    if let Some((hash, packed)) = reusable.and_then(|shapes| shapes.get(&digest(chunk))) {
        // This is a request-local physical shortcut, not graph reuse. Recheck
        // packed identity and every current raw byte before publication.
        if digest(packed) == *hash && unpack_blob(packed).is_ok_and(|raw| raw == chunk) {
            return Ok((hash.clone(), packed.clone()));
        }
    }
    // Stronger matching remains bounded to at most 32 KiB shape chunks.
    // Keep the hundreds-of-megabytes value/delta streams at level 1.
    let packed = pack_blob_at_level(chunk, SHAPE_COMPRESSION_LEVEL)?;
    Ok((digest(&packed), packed))
}

fn pack_analysis_values(
    current: &[u8],
    base_values: Option<PackedAnalysisValues>,
) -> Result<(Vec<u8>, Option<String>), Error> {
    Ok(if let Some(base) = base_values {
        // Normalization only needs the base symbols. Keep the full reference
        // stream packed until delta generation, after encoder scratch is gone.
        let base_hash = base.hash;
        // Match bounded decoded chunks against the current stream. A complete
        // decoded base need not coexist with both graphs and current values.
        let delta = pack_delta_from_packed_base(&base.packed, current)?;
        drop(base.packed);
        #[cfg(test)]
        trace_analysis_memory("delta_packed");
        // The full decoded base is scratch for the delta. Release it before
        // allocating the competing complete representation.
        let complete_values = pack_blob(current)?;
        // The base is already retained. Publish the smaller incremental
        // representation; requiring a 50% saving needlessly retains another
        // complete base even when a delta saves tens of megabytes.
        if delta.len() < complete_values.len() {
            (delta, Some(base_hash))
        } else {
            (complete_values, None)
        }
    } else {
        (pack_blob(current)?, None)
    })
}

#[cfg(test)]
fn trace_analysis_memory(stage: &str) {
    if std::env::var_os("VOLICORD_ANALYSIS_RESOURCE_MANIFEST").is_some() {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        let elapsed = START.get_or_init(std::time::Instant::now).elapsed();
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            let values = status
                .lines()
                .filter(|line| line.starts_with("VmRSS:") || line.starts_with("VmHWM:"))
                .collect::<Vec<_>>()
                .join(" ");
            eprintln!(
                "analysis_resource stage={stage} elapsed_ms={:.3} {values}",
                elapsed.as_secs_f64() * 1000.0
            );
        }
    }
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
    let snapshot: AnalysisSnapshot = read_projection_payload(path, &manifest)?;
    if snapshot.identity != manifest.identity
        || snapshot.project != manifest.project
        || snapshot.generated_at_unix_micros != manifest.generated_at_unix_micros
    {
        return Err(Error::new("Analysis manifest and payload bindings differ"));
    }
    Ok(snapshot)
}

/// Read the exact change-attribution fields, retaining blob integrity checks but
/// without certifying or materializing graph content.
pub(crate) fn read_repository_change_basis(path: &Path) -> Result<RepositoryChangeBasis, Error> {
    let manifest = read_manifest(path)?;
    verify_manifest_blobs(path, &manifest)?;
    let matches = |value: &RepositoryChangeBasis| {
        value.identity == manifest.identity
            && value.project == manifest.project
            && value.generated_at_unix_micros == manifest.generated_at_unix_micros
    };
    if let Some(value) = read_cached_projection(path, &manifest, matches) {
        return Ok(value);
    }
    let value = read_projection_payload(path, &manifest)?;
    if !matches(&value) {
        return Err(Error::new(
            "Analysis manifest and change basis bindings differ",
        ));
    }
    Ok(value)
}

fn read_projection_payload<T: serde::de::DeserializeOwned>(
    path: &Path,
    manifest: &AnalysisManifest,
) -> Result<T, Error> {
    #[cfg(test)]
    trace_analysis_memory("read_start");
    read_metadata(path, manifest)?;
    let blobs = blob_dir(path)?;
    if manifest.shape_blobs.is_empty() {
        return Err(Error::new("Analysis manifest has no shape blobs"));
    }
    let mut shape = Vec::new();
    for hash in &manifest.shape_blobs {
        let packed = read_verified_blob(&blobs.join(format!("{hash}.shape")), hash)?;
        shape.extend_from_slice(&unpack_blob(&packed)?);
    }
    #[cfg(test)]
    trace_analysis_memory("shape_unpacked");
    // Apply the delta directly from its verified envelope. A large literal
    // must not coexist as another complete decoded buffer with base/output.
    let values = {
        let packed_values = read_verified_blob(
            &blobs.join(format!("{}.values", manifest.values_blob)),
            &manifest.values_blob,
        )?;
        if let Some(base_hash) = &manifest.values_base_blob {
            let packed_base =
                read_verified_blob(&blobs.join(format!("{base_hash}.values")), base_hash)?;
            let base = unpack_blob(&packed_base)?;
            drop(packed_base);
            apply_packed_delta(&base, &packed_values)?
        } else {
            unpack_blob(&packed_values)?
        }
    };
    let reader = DenormalizingReader::new(shape, values, manifest.scalar_count)?;
    #[cfg(test)]
    trace_analysis_memory("values_reconstructed");
    let snapshot: T = normalized::decode(reader, manifest.logical_json_bytes)?;
    #[cfg(test)]
    trace_analysis_memory("read_decoded");
    Ok(snapshot)
}

// Independent JSON reconstruction/decoding oracles for the direct normalized
// decoder. Production has one reader and never materializes the expanded JSON.
#[cfg(test)]
fn decode_normalized_json_streamed<T: serde::de::DeserializeOwned>(
    reader: impl Read,
    logical_bytes: u64,
) -> Result<T, Error> {
    let limit = logical_bytes
        .checked_add(1)
        .ok_or_else(|| Error::new("Analysis JSON length is unsupported"))?;
    let mut input = reader.take(limit);
    let value = serde_json::from_reader(BufReader::with_capacity(1024 * 1024, &mut input))
        .map_err(|error| Error::with_source("normalized Analysis Snapshot is corrupt", error))?;
    // from_reader validates EOF, including trailing JSON. The sentinel also
    // rejects a valid value with too few bytes or extra whitespace beyond the
    // exact declared length, without reading an unbounded malformed payload.
    if input.limit() != 1 {
        return Err(Error::new("Analysis JSON length differs from manifest"));
    }
    Ok(value)
}

#[cfg(test)]
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
    let mut encoder = zstd::stream::write::Encoder::new(output, 3)
        .map_err(|error| Error::with_source("cannot create Analysis read cache encoder", error))?;
    // A large graph repeats facts across sections farther apart than the
    // default window. Level 3 also reduces repeated reference/field overhead
    // within that same bounded window; retain the complete MessagePack stream.
    encoder
        .window_log(23)
        .and_then(|_| encoder.long_distance_matching(true))
        .map_err(|error| {
            Error::with_source("cannot configure Analysis read cache encoder", error)
        })?;
    write_analysis_cache_payload(&mut encoder, value)?;
    encoder
        .finish()
        .map_err(|error| Error::with_source("cannot finish Analysis read cache", error))?;
    Ok(())
}

fn write_analysis_cache_payload(
    output: &mut impl Write,
    value: &impl Serialize,
) -> Result<(), Error> {
    // MessagePack emits small field/header writes. Batch those calls before
    // compression, with a fixed request-local buffer and explicit flush errors.
    let mut buffered = BufWriter::with_capacity(64 * 1024, output);
    rmp_serde::encode::write_named(&mut buffered, value)
        .map_err(|error| Error::with_source("cannot stream Analysis read cache", error))?;
    // Drain the buffer without flushing the underlying compressor into an extra
    // frame block. Encoder::finish below retains the existing wire bytes.
    buffered
        .into_inner()
        .map(|_| ())
        .map_err(|error| Error::with_source("cannot flush Analysis read cache", error.into_error()))
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
    read_cached_projection(path, manifest, |snapshot: &AnalysisSnapshot| {
        snapshot.identity == manifest.identity
            && snapshot.project == manifest.project
            && snapshot.generated_at_unix_micros == manifest.generated_at_unix_micros
    })
}

fn read_cached_projection<T: serde::de::DeserializeOwned>(
    path: &Path,
    manifest: &AnalysisManifest,
    matches: impl Fn(&T) -> bool,
) -> Option<T> {
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
        let snapshot: T = read_analysis_cache_value(File::open(&candidate).ok()?).ok()?;
        if matches(&snapshot) {
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
    let mut complete_bases = Vec::<(i64, String, String, PathBuf)>::new();
    let mut shape_bases = Vec::<(i64, String, PathBuf)>::new();
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
        shape_bases.push((
            manifest.generated_at_unix_micros,
            manifest.identity.to_string(),
            path.clone(),
        ));
        if manifest.values_base_blob.is_none() {
            complete_bases.push((
                manifest.generated_at_unix_micros,
                manifest.identity.to_string(),
                manifest.values_blob,
                path,
            ));
        }
    }
    complete_bases.sort_unstable_by(|left, right| right.cmp(left));
    shape_bases.sort_unstable_by(|left, right| right.cmp(left));
    let blobs = project_directory.join("blobs");
    for (_, _, values_hash, path) in complete_bases {
        let Ok(bytes) =
            read_verified_blob(&blobs.join(format!("{values_hash}.values")), &values_hash)
        else {
            continue;
        };
        let Ok(values) = unpack_blob(&bytes) else {
            continue;
        };
        // This is an optional physical plan, never graph authority. A damaged
        // shape prevents slot reuse, while the verified value base can still
        // supply exact byte matches for the complete current representation.
        let dictionary = (|| {
            let manifest = read_manifest(&path)?;
            let mut shape = Vec::new();
            for hash in &manifest.shape_blobs {
                let packed = read_verified_blob(&blobs.join(format!("{hash}.shape")), hash)?;
                shape.extend_from_slice(&unpack_blob(&packed)?);
            }
            symbols::Dictionary::from_reader(DenormalizingReader::new(
                shape,
                values,
                manifest.scalar_count,
            )?)
        })()
        .ok();
        // Shape chunks do not depend on the scalar dictionary or values delta.
        // The latest verified layout gives locality after inventory changes;
        // the complete values base can legitimately be an older observation.
        let shapes = shape_bases
            .first()
            .and_then(|(_, _, path)| {
                let manifest = read_manifest(path).ok()?;
                let mut shapes = HashMap::new();
                let mut verified = HashSet::new();
                for hash in manifest.shape_blobs {
                    if !verified.insert(hash.clone()) {
                        continue;
                    }
                    let packed =
                        read_verified_blob(&blobs.join(format!("{hash}.shape")), &hash).ok()?;
                    let raw = unpack_blob(&packed).ok()?;
                    shapes.insert(digest(&raw), (hash, packed));
                }
                Some(shapes)
            })
            .unwrap_or_default();
        return Ok(Some(PackedAnalysisValues {
            hash: values_hash,
            packed: bytes,
            dictionary,
            shapes,
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
    Ok(wrap_blob(input, compressed))
}

fn pack_metadata(input: &[u8]) -> Result<Vec<u8>, Error> {
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 3)
        .map_err(|error| Error::with_source("cannot create Analysis metadata encoder", error))?;
    // Complete inventory/capability sections repeat farther apart than the
    // default small window. Bound matching to 8 MiB without level-19 CPU cost.
    encoder
        .window_log(23)
        .and_then(|_| encoder.long_distance_matching(true))
        .map_err(|error| Error::with_source("cannot configure Analysis metadata encoder", error))?;
    encoder
        .write_all(input)
        .map_err(|error| Error::with_source("cannot encode Analysis metadata", error))?;
    let compressed = encoder
        .finish()
        .map_err(|error| Error::with_source("cannot finish Analysis metadata", error))?;
    Ok(wrap_blob(input, compressed))
}

fn wrap_blob(input: &[u8], compressed: Vec<u8>) -> Vec<u8> {
    if BLOB_ZSTD_MAGIC.len() + 8 + compressed.len() < BLOB_RAW_MAGIC.len() + input.len() {
        let mut packed = Vec::with_capacity(BLOB_ZSTD_MAGIC.len() + 8 + compressed.len());
        packed.extend_from_slice(BLOB_ZSTD_MAGIC);
        put_u64(&mut packed, input.len() as u64);
        packed.extend_from_slice(&compressed);
        return packed;
    }
    // Large value streams normally compress. Do not allocate their full raw
    // copy merely to compare lengths; release the rejected representation first.
    drop(compressed);
    let mut raw = Vec::with_capacity(BLOB_RAW_MAGIC.len() + input.len());
    raw.extend_from_slice(BLOB_RAW_MAGIC);
    raw.extend_from_slice(input);
    raw
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

#[cfg(test)]
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

/// Visit the same globally rolling chunks as `content_defined_ranges`, without
/// retaining the decoded base. Consume EOF and enforce its exact packed length.
fn visit_base_chunks(
    mut input: impl Read,
    expected: usize,
    mut visit: impl FnMut(usize, &[u8]),
) -> Result<(), Error> {
    let mut buffer = [0_u8; 64 * 1024];
    let mut window = [0_u8; CHUNK_WINDOW_BYTES];
    let mut chunk = Vec::with_capacity(CHUNK_MAX_BYTES);
    let mut rolling = 0_u64;
    let mut total = 0_usize;
    loop {
        let limit = expected
            .saturating_sub(total)
            .saturating_add(1)
            .min(buffer.len());
        let count = input
            .read(&mut buffer[..limit])
            .map_err(|error| Error::with_source("cannot decompress Analysis base", error))?;
        if count == 0 {
            break;
        }
        if count > expected.saturating_sub(total) {
            return Err(Error::new("Analysis packed blob length is corrupt"));
        }
        for &byte in &buffer[..count] {
            let slot = total % CHUNK_WINDOW_BYTES;
            rolling = rolling.rotate_left(1) ^ chunk_byte_hash(byte);
            if total >= CHUNK_WINDOW_BYTES {
                rolling ^= chunk_byte_hash(window[slot]).rotate_left(CHUNK_WINDOW_BYTES as u32);
            }
            window[slot] = byte;
            chunk.push(byte);
            total += 1;
            if chunk.len() >= CHUNK_MIN_BYTES
                && (rolling & 0x1fff == 0 || chunk.len() >= CHUNK_MAX_BYTES)
            {
                visit(total - chunk.len(), &chunk);
                chunk.clear();
            }
        }
    }
    if total != expected {
        return Err(Error::new("Analysis packed blob length is corrupt"));
    }
    if !chunk.is_empty() {
        visit(total - chunk.len(), &chunk);
    }
    Ok(())
}

fn pack_delta_from_packed_base(packed: &[u8], current: &[u8]) -> Result<Vec<u8>, Error> {
    let ranges = content_defined_ranges(current);
    let mut candidates = HashMap::<[u8; 32], Vec<usize>>::new();
    for (index, range) in ranges.iter().enumerate() {
        candidates
            .entry(Sha256::digest(&current[range.clone()]).into())
            .or_default()
            .push(index);
    }
    let mut matches = vec![None; ranges.len()];
    let mut seen = HashSet::<[u8; 32]>::new();
    let mut visit = |offset, bytes: &[u8]| {
        let hash = Sha256::digest(bytes).into();
        // Preserve first-base-chunk selection, including exact byte checking
        // after the hash lookup. Duplicate chunks never change the delta.
        if seen.insert(hash) {
            if let Some(indices) = candidates.get(&hash) {
                for &index in indices {
                    if current[ranges[index].clone()] == *bytes {
                        matches[index] = Some(offset);
                    }
                }
            }
        }
    };
    if let Some(raw) = packed.strip_prefix(BLOB_RAW_MAGIC) {
        visit_base_chunks(raw, raw.len(), &mut visit)?;
    } else {
        let mut at = expect_magic(packed, BLOB_ZSTD_MAGIC)?;
        let expected = usize::try_from(take_u64(packed, &mut at)?)
            .map_err(|_| Error::new("Analysis packed blob length is unsupported"))?;
        let decoder = zstd::stream::read::Decoder::new(&packed[at..])
            .map_err(|error| Error::with_source("cannot decompress Analysis base", error))?;
        visit_base_chunks(decoder, expected, &mut visit)?;
    }
    let mut operations = Vec::new();
    for (range, base_offset) in ranges.into_iter().zip(matches) {
        if let Some(base_offset) = base_offset {
            match operations.last_mut() {
                Some(DeltaOperation::Copy { offset, length })
                    if *offset + *length == base_offset =>
                {
                    *length += range.len();
                }
                _ => operations.push(DeltaOperation::Copy {
                    offset: base_offset,
                    length: range.len(),
                }),
            }
        } else {
            match operations.last_mut() {
                Some(DeltaOperation::Literal(value)) => value.end = range.end,
                _ => operations.push(DeltaOperation::Literal(range)),
            }
        }
    }
    // Lookup tables are scratch too; compression only owns the compact plan.
    drop((candidates, seen));
    pack_delta_operations(current, &operations)
}

#[cfg(test)]
fn pack_delta(base: &[u8], current: &[u8]) -> Result<Vec<u8>, Error> {
    let operations = delta_operations(base, current);
    pack_delta_operations(current, &operations)
}

fn pack_delta_operations(current: &[u8], operations: &[DeltaOperation]) -> Result<Vec<u8>, Error> {
    let length = delta_length(operations);
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 1)
        .map_err(|error| Error::with_source("cannot compress Analysis delta", error))?;
    // Emit directly into compression instead of allocating the entire decoded
    // delta alongside the baseline, current graph and both value streams.
    write_delta(&mut encoder, current, operations)
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
    write_delta(&mut output, current, operations)
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

fn read_delta_bytes(
    input: &mut impl Read,
    remaining: &mut usize,
    output: &mut [u8],
) -> Result<(), Error> {
    if output.len() > *remaining {
        return Err(Error::new("Analysis values delta is truncated"));
    }
    input
        .read_exact(output)
        .map_err(|error| Error::with_source("cannot read Analysis delta", error))?;
    *remaining -= output.len();
    Ok(())
}
fn read_delta_u64(input: &mut impl Read, remaining: &mut usize) -> Result<u64, Error> {
    let mut bytes = [0; 8];
    read_delta_bytes(input, remaining, &mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}
fn apply_delta_streamed(
    base: &[u8],
    mut input: impl Read,
    mut remaining: usize,
) -> Result<Vec<u8>, Error> {
    let mut magic = [0; VALUES_DELTA_MAGIC.len()];
    read_delta_bytes(&mut input, &mut remaining, &mut magic)?;
    if magic != VALUES_DELTA_MAGIC {
        return Err(Error::new("Analysis values delta format is corrupt"));
    }
    let length = usize::try_from(read_delta_u64(&mut input, &mut remaining)?)
        .map_err(|_| Error::new("Analysis values delta length is unsupported"))?;
    let operations = read_delta_u64(&mut input, &mut remaining)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(|error| Error::with_source("cannot allocate Analysis values", error))?;
    let mut buffer = [0; 64 * 1024];
    for _ in 0..operations {
        let mut kind = [0];
        read_delta_bytes(&mut input, &mut remaining, &mut kind)?;
        if kind[0] == 0 {
            let offset = usize::try_from(read_delta_u64(&mut input, &mut remaining)?)
                .map_err(|_| Error::new("Analysis values delta offset is unsupported"))?;
            let count = usize::try_from(read_delta_u64(&mut input, &mut remaining)?)
                .map_err(|_| Error::new("Analysis values delta length is unsupported"))?;
            let end = offset
                .checked_add(count)
                .filter(|end| *end <= base.len())
                .ok_or_else(|| Error::new("Analysis values delta base range is corrupt"))?;
            if count > length.saturating_sub(output.len()) {
                return Err(Error::new("Analysis values delta output length is corrupt"));
            }
            output.extend_from_slice(&base[offset..end]);
        } else if kind[0] == 1 {
            let count = usize::try_from(read_delta_u64(&mut input, &mut remaining)?)
                .map_err(|_| Error::new("Analysis values delta length is unsupported"))?;
            if count > remaining || count > length.saturating_sub(output.len()) {
                return Err(Error::new(
                    "Analysis values delta literal length is corrupt",
                ));
            }
            let mut left = count;
            while left > 0 {
                let count = left.min(buffer.len());
                read_delta_bytes(&mut input, &mut remaining, &mut buffer[..count])?;
                output.extend_from_slice(&buffer[..count]);
                left -= count;
            }
        } else {
            return Err(Error::new("Analysis values delta operation is corrupt"));
        }
    }
    if remaining != 0 || output.len() != length {
        return Err(Error::new("Analysis values delta has trailing data"));
    }
    if input
        .read(&mut [0; 1])
        .map_err(|error| Error::with_source("cannot finish Analysis delta", error))?
        != 0
    {
        return Err(Error::new("Analysis packed blob length is corrupt"));
    }
    Ok(output)
}
fn apply_packed_delta(base: &[u8], packed: &[u8]) -> Result<Vec<u8>, Error> {
    if let Some(raw) = packed.strip_prefix(BLOB_RAW_MAGIC) {
        return apply_delta_streamed(base, raw, raw.len());
    }
    let mut at = expect_magic(packed, BLOB_ZSTD_MAGIC)?;
    let expected = usize::try_from(take_u64(packed, &mut at)?)
        .map_err(|_| Error::new("Analysis packed blob length is unsupported"))?;
    let decoder = zstd::stream::read::Decoder::new(&packed[at..])
        .map_err(|error| Error::with_source("cannot decompress Analysis delta", error))?;
    apply_delta_streamed(base, decoder, expected)
}

#[cfg(test)]
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

// Chunking visits hundreds of megabytes twice, including the outgoing byte of
// each rolling window. The byte alphabet is fixed; preserve the exact mixing
// values without repeating their arithmetic for every visit.
const CHUNK_BYTE_HASHES: [u64; 256] = {
    let mut hashes = [0; 256];
    let mut index = 0;
    while index < hashes.len() {
        let mut value = (index as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        hashes[index] = value ^ (value >> 31);
        index += 1;
    }
    hashes
};

fn chunk_byte_hash(byte: u8) -> u64 {
    CHUNK_BYTE_HASHES[usize::from(byte)]
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
    classes: Vec<symbols::ScalarClass>,
    positions: Vec<Option<symbols::Position>>,
    tracker: symbols::Tracker,
    references: Option<BufWriter<zstd::stream::write::Encoder<'static, Vec<u8>>>>,
    scalar_count: u64,
    token: Vec<u8>,
    in_string: bool,
    escaped: bool,
    logical_bytes: u64,
}

impl Write for NormalizingWriter {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        self.logical_bytes = self.logical_bytes.saturating_add(input.len() as u64);
        // The serializer supplies complete plain fragments separately from
        // quotes/escapes. In particular, formatted identities arrive in one
        // bounded span. Keep the scalar bytes and first-seen order unchanged.
        if self.in_string && !self.escaped && !input.contains(&b'"') && !input.contains(&b'\\') {
            self.token.extend_from_slice(input);
            return Ok(input.len());
        }
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
                b'{' | b'}' | b'[' | b']' | b',' | b':' => {
                    self.tracker.punctuation(byte);
                    self.shape.push(byte);
                }
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
    // Fixture encoder for retained current-format symbol ordering. Production
    // emits only first-seen dictionaries; the one decoder still reads original
    // current-format streams without rewriting their symbols or references.
    #[cfg(test)]
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
            positions: vec![None; symbols.len()],
            classes: symbols
                .iter()
                .map(|symbol| symbols::ScalarClass::new(symbol))
                .collect(),
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
            self.positions.push(None);
            self.classes.push(symbols::ScalarClass::new(&self.token));
            index
        };
        let class = self
            .classes
            .get(index as usize)
            .ok_or_else(|| std::io::Error::other("Analysis symbol class is missing"))?;
        let position = self.tracker.scalar(*class);
        let saved = self
            .positions
            .get_mut(index as usize)
            .ok_or_else(|| std::io::Error::other("Analysis symbol position is missing"))?;
        if saved.is_none() {
            *saved = Some(position);
        }
        // Repeated field names and values dominate large graphs. Keep the
        // scratch capacity instead of reallocating it for every scalar; interned
        // symbols own independent bytes and retain the same first-seen indices.
        self.token.clear();
        self.shape.push(b'$');
        if self.references.is_none() {
            self.references = Some(BufWriter::with_capacity(
                64 * 1024,
                zstd::stream::write::Encoder::new(Vec::new(), 1)?,
            ));
        }
        if let Some(references) = &mut self.references {
            references.write_all(&index.to_le_bytes())?;
        }
        self.scalar_count = self
            .scalar_count
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("Analysis scalar count overflow"))?;
        Ok(())
    }

    fn finish_parts(mut self) -> Result<NormalizedParts, Error> {
        if self.in_string {
            return Err(Error::new("Analysis JSON ended inside a string"));
        }
        if !self.token.is_empty() {
            self.finish_scalar()
                .map_err(|error| Error::with_source("cannot finish Analysis scalar", error))?;
        }
        // References remain compressed while symbols are interned. Serialize and
        // release the symbol table before expanding references into the final
        // unchanged value stream; both graphs remain available throughout.
        drop(self.symbol_index);
        drop(self.classes);
        let scalar_count = self.scalar_count;
        let packed_references = self
            .references
            .map(|references| {
                references
                    .into_inner()
                    .map_err(|error| {
                        Error::with_source("cannot flush Analysis references", error.into_error())
                    })?
                    .finish()
                    .map_err(|error| Error::with_source("cannot finish Analysis references", error))
            })
            .transpose()?;
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
        Ok(NormalizedParts {
            shape,
            values: PendingValues {
                symbols: self.symbols,
                positions: self.positions,
                packed_references,
                scalar_count,
            },
            logical_bytes: self.logical_bytes,
            scalar_count,
        })
    }

    #[cfg(test)]
    fn finish(self) -> Result<Normalized, Error> {
        let parts = self.finish_parts()?;
        Ok(Normalized {
            shape: parts.shape,
            values: parts.values.finish()?,
            logical_bytes: parts.logical_bytes,
            scalar_count: parts.scalar_count,
        })
    }
}

struct NormalizedParts {
    shape: Vec<u8>,
    values: PendingValues,
    logical_bytes: u64,
    scalar_count: u64,
}

struct PendingValues {
    symbols: Vec<Rc<[u8]>>,
    positions: Vec<Option<symbols::Position>>,
    packed_references: Option<Vec<u8>>,
    scalar_count: u64,
}

impl PendingValues {
    fn finish(self) -> Result<Vec<u8>, Error> {
        let scalar_count = self.scalar_count;
        let prefix_length = self
            .symbols
            .iter()
            .try_fold(VALUES_MAGIC.len() + 16, |length, symbol| {
                length.checked_add(8)?.checked_add(symbol.len())
            })
            .ok_or_else(|| Error::new("Analysis value stream length is unsupported"))?;
        let reference_length = usize::try_from(scalar_count)
            .ok()
            .and_then(|count| count.checked_mul(4))
            .ok_or_else(|| Error::new("Analysis reference length is unsupported"))?;
        let value_length = prefix_length
            .checked_add(reference_length)
            .ok_or_else(|| Error::new("Analysis value stream length is unsupported"))?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(value_length)
            .map_err(|error| Error::with_source("cannot allocate Analysis value stream", error))?;
        (|| -> std::io::Result<()> {
            values.write_all(VALUES_MAGIC)?;
            values.write_all(&(self.symbols.len() as u64).to_le_bytes())?;
            for symbol in self.symbols {
                values.write_all(&(symbol.len() as u64).to_le_bytes())?;
                values.write_all(&symbol)?;
            }
            values.write_all(&scalar_count.to_le_bytes())
        })()
        .map_err(|error| Error::with_source("cannot write Analysis value prefix", error))?;
        if values.len() != prefix_length {
            return Err(Error::new("Analysis value prefix length differs"));
        }
        if let Some(packed) = self.packed_references {
            let decoder = zstd::stream::read::Decoder::new(packed.as_slice())
                .map_err(|error| Error::with_source("cannot open Analysis references", error))?;
            let limit = (reference_length as u64)
                .checked_add(1)
                .ok_or_else(|| Error::new("Analysis reference length is unsupported"))?;
            let copied = std::io::copy(&mut decoder.take(limit), &mut values)
                .map_err(|error| Error::with_source("cannot restore Analysis references", error))?;
            if copied != reference_length as u64 {
                return Err(Error::new("Analysis reference length differs"));
            }
        }
        if values.len() != value_length {
            return Err(Error::new("Analysis value stream length differs"));
        }
        Ok(values)
    }
}

#[cfg(test)]
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

#[cfg(test)]
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
    fn shape_reuse_requires_exact_current_bytes_and_packed_identity() -> Result<(), Error> {
        let chunk = br#"[{"field":$},{"other":[$,$]}]"#.repeat(512);
        // A raw envelope is valid in the same current representation. Its
        // different bytes make actual reuse observable without a timing oracle.
        let retained = [BLOB_RAW_MAGIC, chunk.as_slice()].concat();
        let raw_hash = digest(&chunk);
        let mut shapes = HashMap::from([(raw_hash.clone(), (digest(&retained), retained.clone()))]);
        assert_eq!(pack_shape(&chunk, Some(&shapes))?.1, retained);
        let fresh = pack_shape(&chunk, None)?;
        assert_ne!(fresh.1, retained);
        assert_eq!(unpack_blob(&fresh.1)?, chunk);
        for invalid in [
            ("wrong packed identity".into(), retained),
            {
                let wrong = pack_blob(b"different current shape")?;
                (digest(&wrong), wrong)
            },
            {
                let corrupt = b"corrupt packed envelope".to_vec();
                (digest(&corrupt), corrupt)
            },
        ] {
            shapes.insert(raw_hash.clone(), invalid);
            assert_eq!(pack_shape(&chunk, Some(&shapes))?, fresh);
        }
        assert_eq!(
            pack_shape(b"new shape", Some(&shapes))?,
            pack_shape(b"new shape", None)?
        );
        Ok(())
    }

    #[test]
    fn shape_reuse_selects_latest_layout_independent_of_complete_values_base(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let repository = home.path().join("repository");
        std::fs::create_dir(&repository)?;
        std::fs::write(repository.join("lib.rs"), "pub fn value() -> u8 { 7 }\n")?;
        let operations =
            crate::LocalOperations::new(crate::RuntimeLayout::new(home.path().join("runtime"))?);
        let project = operations
            .initialize_project("Shape reuse", Some(&repository))?
            .project
            .id;
        let first = operations
            .analyze(project, Vec::new())?
            .value
            .ok_or("first")?;
        std::fs::write(repository.join("extra.py"), "def extra():\n    return 8\n")?;
        let latest = operations
            .analyze(project, Vec::new())?
            .value
            .ok_or("latest")?;
        let first_manifest = read_manifest(&first.stored_at)?;
        let mut latest_manifest = read_manifest(&latest.stored_at)?;
        assert_ne!(first_manifest.shape_blobs, latest_manifest.shape_blobs);
        // Select only the first snapshot as a complete value base. This manifest
        // control changes no payload or production schema, and is restored before
        // independently checking both complete historical graphs below.
        let original_latest = std::fs::read(&latest.stored_at)?;
        latest_manifest.values_base_blob = Some(first_manifest.values_blob.clone());
        std::fs::write(&latest.stored_at, serde_json::to_vec(&latest_manifest)?)?;
        let directory = latest.stored_at.parent().ok_or("parent")?;
        let base = reusable_base_values(directory)?.ok_or("base")?;
        assert_eq!(base.hash, first_manifest.values_blob);
        let expected = latest_manifest
            .shape_blobs
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        assert_eq!(
            base.shapes
                .values()
                .map(|(hash, _)| hash.clone())
                .collect::<HashSet<_>>(),
            expected
        );
        for (raw_hash, (packed_hash, packed)) in &base.shapes {
            assert_eq!(digest(packed), *packed_hash);
            assert_eq!(digest(&unpack_blob(packed)?), *raw_hash);
        }
        let damaged_path =
            blob_dir(&latest.stored_at)?.join(format!("{}.shape", latest_manifest.shape_blobs[0]));
        let original_shape = std::fs::read(&damaged_path)?;
        std::fs::write(&damaged_path, b"damaged optional shape")?;
        assert!(reusable_base_values(directory)?
            .ok_or("base")?
            .shapes
            .is_empty());
        std::fs::write(damaged_path, original_shape)?;
        std::fs::write(&latest.stored_at, original_latest)?;
        assert_eq!(read_analysis_durable(&first.stored_at)?, first.analysis);
        assert_eq!(read_analysis_durable(&latest.stored_at)?, latest.analysis);
        Ok(())
    }

    #[test]
    fn compressed_reference_scratch_preserves_wire_bytes() -> Result<(), Error> {
        for (input, symbols, references) in [
            (b"[]".as_slice(), Vec::new(), Vec::new()),
            (
                br#"["a","b","a"]"#.as_slice(),
                vec![br#""a""#.as_slice(), br#""b""#.as_slice()],
                vec![0_u32, 1, 0],
            ),
        ] {
            let mut writer = NormalizingWriter::default();
            writer
                .write_all(input)
                .map_err(|error| Error::with_source("cannot normalize fixture", error))?;
            let normalized = writer.finish()?;
            let mut expected = VALUES_MAGIC.to_vec();
            put_u64(&mut expected, symbols.len() as u64);
            for symbol in symbols {
                put_u64(&mut expected, symbol.len() as u64);
                expected.extend_from_slice(symbol);
            }
            put_u64(&mut expected, references.len() as u64);
            for reference in references {
                expected.extend_from_slice(&reference.to_le_bytes());
            }
            assert_eq!(normalized.values, expected);
        }
        Ok(())
    }

    #[test]
    fn compressed_references_cross_buffer_boundaries_with_exact_seeded_indices(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let count = 65_537usize;
        let tokens = ["\"한글\\n🦀\"".as_bytes(), b"42", b"null"];
        let mut input = vec![b'['];
        for index in 0..count {
            if index != 0 {
                input.push(b',');
            }
            input.extend_from_slice(tokens[index % 3]);
        }
        input.push(b']');
        let mut seed = NormalizingWriter::default();
        seed.write_all(br#"["base-only",42]"#)?;
        let base = seed.finish()?.values;
        for seeded in [false, true] {
            let (symbols, indices) = if seeded {
                (
                    vec![br#""base-only""#.as_slice(), b"42", tokens[0], b"null"],
                    [2u32, 1, 3],
                )
            } else {
                (tokens.to_vec(), [0u32, 1, 2])
            };
            let mut expected = VALUES_MAGIC.to_vec();
            put_u64(&mut expected, symbols.len() as u64);
            for symbol in symbols {
                put_u64(&mut expected, symbol.len() as u64);
                expected.extend_from_slice(symbol);
            }
            put_u64(&mut expected, count as u64);
            for index in 0..count {
                expected.extend_from_slice(&indices[index % 3].to_le_bytes());
            }
            for chunk_size in [1, 7, 65_536] {
                let mut writer = if seeded {
                    NormalizingWriter::with_base_symbols(&base)?
                } else {
                    NormalizingWriter::default()
                };
                for chunk in input.chunks(chunk_size) {
                    writer.write_all(chunk)?;
                }
                let normalized = writer.finish()?;
                assert_eq!(normalized.values, expected);
                assert_eq!(normalized.scalar_count, count as u64);
                assert_eq!(normalized.logical_bytes, input.len() as u64);
                let actual: serde_json::Value = decode_normalized_json_streamed(
                    DenormalizingReader::new(normalized.shape, normalized.values, count as u64)?,
                    input.len() as u64,
                )?;
                assert_eq!(actual, serde_json::from_slice::<serde_json::Value>(&input)?);
            }
        }
        Ok(())
    }

    #[test]
    fn large_profitable_delta_does_not_require_another_complete_base() -> Result<(), Error> {
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut random = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state.to_le_bytes()
        };
        let base = (0..12 * 1024 * 1024 / 8)
            .flat_map(|_| random())
            .collect::<Vec<_>>();
        let mut current = base.clone();
        for chunk in current[..9 * 1024 * 1024].chunks_exact_mut(8) {
            chunk.copy_from_slice(&random());
        }
        let complete = pack_blob(&current)?;
        let packed = pack_blob(&base)?;
        let hash = digest(&packed);
        let (selected, selected_base) = pack_analysis_values(
            &current,
            Some(PackedAnalysisValues {
                hash: hash.clone(),
                packed,
                dictionary: None,
                shapes: HashMap::new(),
            }),
        )?;
        assert!(complete.len() > 8 * 1024 * 1024);
        assert!(selected.len() > complete.len() / 2);
        assert!(selected.len() < complete.len());
        assert_eq!(selected_base.as_deref(), Some(hash.as_str()));
        assert_eq!(apply_delta(&base, &unpack_blob(&selected)?)?, current);
        let (standalone, selected_base) = pack_analysis_values(&current, None)?;
        assert!(selected_base.is_none());
        assert_eq!(unpack_blob(&standalone)?, current);
        let replacement = vec![0; current.len()];
        let (standalone, selected_base) = pack_analysis_values(
            &replacement,
            Some(PackedAnalysisValues {
                hash,
                packed: pack_blob(&base)?,
                dictionary: None,
                shapes: HashMap::new(),
            }),
        )?;
        assert!(selected_base.is_none());
        assert_eq!(unpack_blob(&standalone)?, replacement);
        Ok(())
    }

    #[test]
    fn chunk_byte_hashes_preserve_existing_mapping() {
        let bytes = (0_u8..=u8::MAX)
            .flat_map(|byte| chunk_byte_hash(byte).to_le_bytes())
            .collect::<Vec<_>>();
        // Captured independently from the original arithmetic for all 256 bytes.
        assert_eq!(
            digest(&bytes),
            "33ba31e3638819c4c7eb8c80c94bd1b864ae49b0df709e50c2a8fc0255753b73"
        );
    }

    /// With no override this uses actual small polyglot Product output. A retained
    /// large manifest can be supplied for the same read/use/write path without
    /// rebuilding a different repository or changing its Source/snapshot identity.
    #[test]
    fn resource_pressure_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        fn fingerprint(value: &impl Serialize) -> Result<String, serde_json::Error> {
            struct HashWriter(Sha256);
            impl Write for HashWriter {
                fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                    self.0.update(bytes);
                    Ok(bytes.len())
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    Ok(())
                }
            }
            let mut output = HashWriter(Sha256::new());
            serde_json::to_writer(&mut output, value)?;
            Ok(format!("{:x}", output.0.finalize()))
        }
        fn memory(stage: &str) {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                let values = status
                    .lines()
                    .filter(|line| line.starts_with("VmRSS:") || line.starts_with("VmHWM:"))
                    .collect::<Vec<_>>()
                    .join(" ");
                eprintln!("analysis_resource stage={stage} {values}");
            }
        }
        let temporary = tempfile::tempdir()?;
        let input = match std::env::var_os("VOLICORD_ANALYSIS_RESOURCE_MANIFEST") {
            Some(path) => PathBuf::from(path),
            None => {
                let repository = temporary.path().join("repository");
                std::fs::create_dir_all(&repository)?;
                for index in 0..16 {
                    for (extension, body) in [
                        ("py", format!("def value_{index}():\n    return {index}\n")),
                        (
                            "rs",
                            format!("pub fn value_{index}() -> i32 {{ {index} }}\n"),
                        ),
                        (
                            "ts",
                            format!(
                                "export function value_{index}(): number {{ return {index}; }}\n"
                            ),
                        ),
                    ] {
                        std::fs::write(
                            repository.join(format!("module_{index}.{extension}")),
                            body,
                        )?;
                    }
                }
                let operations = crate::LocalOperations::new(crate::RuntimeLayout::new(
                    temporary.path().join("runtime"),
                )?);
                let project = operations
                    .initialize_project("Resource roundtrip", Some(&repository))?
                    .project
                    .id;
                operations
                    .analyze(project, Vec::new())?
                    .value
                    .ok_or("analysis")?
                    .stored_at
            }
        };
        let input_bytes = std::fs::read(&input)?;
        let baseline = read_analysis(&input)?;
        let current = read_analysis(&input)?;
        let expected = fingerprint(&baseline)?;
        assert_eq!(fingerprint(&current)?, expected);
        memory("baseline_and_current_loaded");
        let base = reusable_base_values(input.parent().ok_or("input directory")?)?;
        let encoded = encode_analysis(&current, base)?;
        memory("encoded_with_baseline_retained");
        // Metadata is consumed separately from the full graph. Check its entire
        // restored JSON, including coverage/provenance, as well as the graph
        // roundtrips below; a smaller but incomplete side payload is invalid.
        let metadata_json = serde_json::to_vec(&AnalysisMetadata::from(&current))?;
        assert_eq!(unpack_blob(&encoded.metadata)?, metadata_json);
        eprintln!(
            "analysis_resource metadata_json={} metadata_packed={}",
            metadata_json.len(),
            encoded.metadata.len()
        );
        drop(metadata_json);
        // Fingerprint every produced byte, so before/after runs on the same exact
        // retained input compare complete output, not merely counts or identity.
        eprintln!(
            "analysis_resource input={} manifest={} values={} metadata={} shape_count={} values_bytes={} shape_bytes={}",
            digest(&input_bytes),
            digest(&encoded.manifest),
            encoded.values_hash,
            encoded.metadata_hash,
            encoded.shapes.len(),
            encoded.values.len(),
            encoded.shapes.iter().map(|(_, bytes)| bytes.len()).sum::<usize>()
        );
        let output = temporary.path().join("roundtrip");
        std::fs::create_dir_all(output.join("blobs"))?;
        if std::env::var_os("VOLICORD_ANALYSIS_RESOURCE_PUBLISHED_CACHE").is_some() {
            let cache = output.join("cache");
            std::fs::create_dir(&cache)?;
            let temporary_cache = cache.join("pending");
            write_analysis_cache(&mut File::create(&temporary_cache)?, &current)?;
            let hash = digest_file(&temporary_cache)?;
            std::fs::rename(
                temporary_cache,
                analysis_cache_path(&output, current.identity, &hash),
            )?;
        }
        drop((baseline, current));
        let manifest: AnalysisManifest = serde_json::from_slice(&encoded.manifest)?;
        if let Some(base) = &manifest.values_base_blob {
            let name = format!("{base}.values");
            std::fs::copy(
                blob_dir(&input)?.join(&name),
                output.join("blobs").join(&name),
            )?;
        }
        for (hash, bytes) in &encoded.shapes {
            std::fs::write(output.join("blobs").join(format!("{hash}.shape")), bytes)?;
        }
        std::fs::write(
            output
                .join("blobs")
                .join(format!("{}.values", encoded.values_hash)),
            &encoded.values,
        )?;
        std::fs::write(
            output
                .join("blobs")
                .join(format!("{}.metadata", encoded.metadata_hash)),
            &encoded.metadata,
        )?;
        let path = output.join(format!("{}.json", manifest.identity));
        std::fs::write(&path, &encoded.manifest)?;
        drop(encoded);
        if std::env::var_os("VOLICORD_ANALYSIS_RESOURCE_PAIRED_READER").is_some() {
            // Normal Work consumers retain a baseline while reading the current
            // graph. By default the copy has no read cache; the explicit cache
            // mode instead exercises the current publication codec. Neither
            // mode changes the input or graph scope.
            let baseline = read_analysis(&input)?;
            for _ in 0..3 {
                assert_eq!(fingerprint(&read_analysis(&path)?)?, expected);
            }
            assert_eq!(fingerprint(&baseline)?, expected);
            memory("retained_baseline_and_normal_reads");
        } else {
            for _ in 0..3 {
                // No read cache: each call independently reconstructs the complete
                // durable graph, just as a fresh process with absent Derived caches.
                assert_eq!(fingerprint(&read_analysis_durable(&path)?)?, expected);
            }
            memory("repeated_durable_reads");
        }
        let corrupt = output
            .join("blobs")
            .join(format!("{}.values", manifest.values_blob));
        std::fs::write(&corrupt, b"corrupt")?;
        assert!(read_analysis(&path).is_err());
        assert!(read_analysis_durable(&path).is_err());
        assert_eq!(
            std::fs::read(&input)?,
            input_bytes,
            "retained input must remain immutable"
        );
        Ok(())
    }

    #[test]
    fn change_basis_is_equal_with_and_without_cache_and_refuses_damaged_graph_blobs(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use volicord_repository_intelligence::RepositoryChangeView;
        let home = tempfile::tempdir()?;
        let repository = home.path().join("repository");
        std::fs::create_dir(&repository)?;
        std::fs::write(repository.join("lib.rs"), "pub fn value() -> u8 { 7 }\n")?;
        std::fs::write(repository.join("value.py"), "def value():\n    return 7\n")?;
        let operations =
            crate::LocalOperations::new(crate::RuntimeLayout::new(home.path().join("runtime"))?);
        let project = operations
            .initialize_project("Change basis", Some(&repository))?
            .project
            .id;
        let outcome = operations
            .analyze(project, Vec::new())?
            .value
            .ok_or("analysis")?;
        let expected = outcome.analysis.change_basis().into_owned();
        let original_manifest = std::fs::read(&outcome.stored_at)?;
        let original_canonical = operations.canonical_basis(project)?;
        assert_eq!(read_repository_change_basis(&outcome.stored_at)?, expected);
        std::fs::remove_dir_all(outcome.stored_at.parent().ok_or("parent")?.join("cache"))?;
        assert_eq!(read_repository_change_basis(&outcome.stored_at)?, expected);
        assert_eq!(read_analysis_durable(&outcome.stored_at)?, outcome.analysis);
        assert_eq!(std::fs::read(&outcome.stored_at)?, original_manifest);
        assert_eq!(operations.canonical_basis(project)?, original_canonical);
        let manifest = read_manifest(&outcome.stored_at)?;
        std::fs::write(
            blob_dir(&outcome.stored_at)?.join(format!("{}.values", manifest.values_blob)),
            b"damaged",
        )?;
        assert!(read_repository_change_basis(&outcome.stored_at).is_err());
        assert!(read_analysis(&outcome.stored_at).is_err());
        assert_eq!(operations.canonical_basis(project)?, original_canonical);
        Ok(())
    }

    #[test]
    fn packed_blobs_preserve_both_formats_and_reject_corrupt_lengths() -> Result<(), Error> {
        // All production blob writers share the same envelope/decoder. Exercise
        // shape and metadata settings as well as the large value-stream codec.
        for level in [1, SHAPE_COMPRESSION_LEVEL, 19] {
            for input in [b"".as_slice(), b"x"] {
                let packed = pack_blob_at_level(input, level)?;
                let mut expected = BLOB_RAW_MAGIC.to_vec();
                expected.extend_from_slice(input);
                assert_eq!(packed, expected);
                assert_eq!(unpack_blob(&packed)?, input);
            }
            let input = vec![b'x'; 64 * 1024];
            let packed = pack_blob_at_level(&input, level)?;
            assert!(packed.starts_with(BLOB_ZSTD_MAGIC));
            assert_eq!(unpack_blob(&packed)?, input);
            assert_eq!(pack_blob_at_level(&input, level)?, packed);
            let mut wrong_length = packed.clone();
            wrong_length[BLOB_ZSTD_MAGIC.len()..BLOB_ZSTD_MAGIC.len() + 8]
                .copy_from_slice(&(input.len() as u64 + 1).to_le_bytes());
            assert!(unpack_blob(&wrong_length).is_err());
            let mut short_length = packed.clone();
            short_length[BLOB_ZSTD_MAGIC.len()..BLOB_ZSTD_MAGIC.len() + 8]
                .copy_from_slice(&(input.len() as u64 - 1).to_le_bytes());
            assert!(unpack_blob(&short_length).is_err());
            assert!(unpack_blob(&packed[..packed.len() - 1]).is_err());
        }
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
    fn analysis_identity_keeps_messagepack_cache_wire_bytes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let text = "a1".repeat(32);
        let identity = AnalysisSnapshotId::from_hex(&text.to_uppercase())?;
        let mut expected = vec![0xd9, 64];
        expected.extend_from_slice(text.as_bytes());
        assert_eq!(rmp_serde::to_vec(&identity)?, expected);
        assert_eq!(
            rmp_serde::from_slice::<AnalysisSnapshotId>(&expected)?,
            identity
        );
        let mut cache = Vec::new();
        write_analysis_cache_value(&mut cache, &identity)?;
        assert_eq!(
            read_analysis_cache_value::<AnalysisSnapshotId>(cache.as_slice())?,
            identity
        );
        Ok(())
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
        // Cross quotes, escapes and UTF-8 boundaries independently of serializer
        // fragments, including single bytes and large plain spans.
        for size in [1, 2, 3, 7, 31, 64, 257, original.len()] {
            let mut writer = NormalizingWriter::default();
            for chunk in original.chunks(size) {
                writer.write_all(chunk).map_err(|error| {
                    Error::with_source("cannot normalize scalar reuse fixture", error)
                })?;
            }
            let normalized = writer.finish()?;
            let mut reader = DenormalizingReader::new(
                normalized.shape,
                normalized.values,
                normalized.scalar_count,
            )?;
            let mut decoded = Vec::new();
            reader.read_to_end(&mut decoded).map_err(|error| {
                Error::with_source("cannot reconstruct scalar reuse fixture", error)
            })?;
            assert_eq!(decoded, original);
        }
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
            assert!(
                decode_normalized_json_streamed::<Vec<String>>(encoded.as_slice(), length).is_err()
            );
        }
        for corrupt in [br#"["a",2]"#.as_slice(), br#"["a"]true"#.as_slice()] {
            assert!(decode_normalized_json::<Vec<String>>(corrupt, corrupt.len() as u64).is_err());
            assert!(
                decode_normalized_json_streamed::<Vec<String>>(corrupt, corrupt.len() as u64)
                    .is_err()
            );
        }
        assert!(decode_normalized_json_streamed::<Vec<String>>(b"[] ".as_slice(), 2).is_err());
        assert!(
            decode_normalized_json_streamed::<Vec<String>>(encoded.as_slice(), u64::MAX).is_err()
        );
        Ok(())
    }

    #[test]
    fn normalized_stream_delivers_values_before_reading_the_whole_payload(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::cell::Cell;
        thread_local! {
            static PROGRESS: Cell<(usize, Option<usize>)> = const { Cell::new((0, None)) };
        }
        struct ObservedRead<'a>(&'a [u8]);
        impl Read for ObservedRead<'_> {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                let count = self.0.read(output)?;
                PROGRESS.with(|p| {
                    let (n, first) = p.get();
                    p.set((n + count, first));
                });
                Ok(count)
            }
        }
        #[derive(Debug, PartialEq)]
        struct Item(String);
        impl<'de> Deserialize<'de> for Item {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                PROGRESS.with(|p| {
                    let (n, first) = p.get();
                    p.set((n, first.or(Some(n))));
                });
                Ok(Self(value))
            }
        }
        let values = vec!["first".to_owned(), "tail 한글".repeat(512 * 1024)];
        let encoded = serde_json::to_vec(&values)?;
        PROGRESS.with(|p| p.set((0, None)));
        let decoded: Vec<Item> =
            decode_normalized_json_streamed(ObservedRead(&encoded), encoded.len() as u64)?;
        assert_eq!(decoded, values.into_iter().map(Item).collect::<Vec<_>>());
        let (total, first) = PROGRESS.with(Cell::get);
        assert_eq!(total, encoded.len());
        assert!(
            first.is_some_and(|n| n > 0 && n < total),
            "typed values must be delivered before the entire payload is buffered"
        );
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
    fn streamed_delta_reads_match_bulk_bytes_across_short_reads() -> Result<(), Error> {
        struct ShortReads<'a>(&'a [u8], usize);
        impl Read for ShortReads<'_> {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                let count = output.len().min(self.1);
                self.0.read(&mut output[..count])
            }
        }
        let mut state = 37_u64;
        let noisy = (0..100_000)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (state >> 56) as u8
            })
            .collect::<Vec<_>>();
        let mut compressed_seen = false;
        let mut raw_seen = false;
        for base in [Vec::new(), noisy, vec![b'x'; 100_000]] {
            let mut changed = base.clone();
            changed.splice(
                17.min(changed.len())..17.min(changed.len()),
                "한글-change".bytes(),
            );
            for current in [Vec::new(), base.clone(), changed, base.repeat(2)] {
                let delta = encode_delta(&base, &current)?;
                assert_eq!(apply_delta(&base, &delta)?, current);
                let packed = pack_blob(&delta)?;
                raw_seen |= packed.starts_with(BLOB_RAW_MAGIC);
                compressed_seen |= packed.starts_with(BLOB_ZSTD_MAGIC);
                assert_eq!(apply_packed_delta(&base, &packed)?, current);
                let mut raw = BLOB_RAW_MAGIC.to_vec();
                raw.extend_from_slice(&delta);
                assert_eq!(apply_packed_delta(&base, &raw)?, current);
                for size in [1, 7, 63, 65_536] {
                    assert_eq!(
                        apply_delta_streamed(&base, ShortReads(&delta, size), delta.len())?,
                        current
                    );
                }
            }
        }
        assert!(raw_seen && compressed_seen);
        Ok(())
    }

    #[test]
    fn streamed_delta_rejects_corrupt_lengths_ranges_eof_and_io() -> Result<(), Error> {
        let current = vec![b'z'; 150_000];
        let delta = encode_delta(&[], &current)?;
        let packed = pack_blob(&delta)?;
        assert!(packed.starts_with(BLOB_ZSTD_MAGIC));
        for length in [delta.len() - 1, delta.len() + 1] {
            let mut invalid = packed.clone();
            invalid[BLOB_ZSTD_MAGIC.len()..BLOB_ZSTD_MAGIC.len() + 8]
                .copy_from_slice(&(length as u64).to_le_bytes());
            assert!(apply_packed_delta(&[], &invalid).is_err());
            assert!(apply_delta_streamed(&[], &delta[..], length).is_err());
        }
        for length in [packed.len() / 2, packed.len() - 1] {
            assert!(apply_packed_delta(&[], &packed[..length]).is_err());
        }
        let mut invalid = Vec::new();
        let mut trailing = delta.clone();
        trailing.push(0);
        invalid.push(trailing);
        invalid.push(delta[..delta.len() - 1].to_vec());
        let mut kind = delta.clone();
        kind[VALUES_DELTA_MAGIC.len() + 16] = 2;
        invalid.push(kind);
        let mut literal = delta.clone();
        literal[VALUES_DELTA_MAGIC.len() + 17..VALUES_DELTA_MAGIC.len() + 25]
            .copy_from_slice(&u64::MAX.to_le_bytes());
        invalid.push(literal);
        for length in [current.len() - 1, current.len() + 1] {
            let mut output_length = delta.clone();
            output_length[VALUES_DELTA_MAGIC.len()..VALUES_DELTA_MAGIC.len() + 8]
                .copy_from_slice(&(length as u64).to_le_bytes());
            invalid.push(output_length);
        }
        let mut copy = VALUES_DELTA_MAGIC.to_vec();
        for n in [1, 1] {
            put_u64(&mut copy, n);
        }
        copy.push(0);
        put_u64(&mut copy, u64::MAX);
        put_u64(&mut copy, 1);
        invalid.push(copy);
        for bytes in invalid {
            assert!(apply_packed_delta(&[], &pack_blob(&bytes)?).is_err());
        }
        assert!(apply_packed_delta(&[], b"wrong envelope").is_err());
        struct FailedRead;
        impl Read for FailedRead {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("injected delta read failure"))
            }
        }
        assert!(apply_delta_streamed(&[], FailedRead, delta.len()).is_err());
        Ok(())
    }

    #[test]
    fn packed_base_delta_preserves_exact_bulk_bytes_and_rejects_invalid_base() -> Result<(), Error>
    {
        let mut state = 17_u64;
        let noisy = (0..300_000)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (state >> 56) as u8
            })
            .collect::<Vec<_>>();
        for base in [Vec::new(), b"short".to_vec(), noisy, vec![b'x'; 300_000]] {
            let mut changed = base.clone();
            changed.splice(
                changed.len() / 2..changed.len() / 2,
                "inserted-한글".as_bytes().iter().copied(),
            );
            for current in [Vec::new(), base.clone(), changed, base.repeat(2)] {
                let expected = pack_delta(&base, &current)?;
                let mut raw = BLOB_RAW_MAGIC.to_vec();
                raw.extend_from_slice(&base);
                for packed in [raw, pack_blob(&base)?] {
                    assert_eq!(pack_delta_from_packed_base(&packed, &current)?, expected);
                    assert_eq!(apply_delta(&base, &unpack_blob(&expected)?)?, current);
                }
            }
        }
        let packed = pack_blob(&vec![b'x'; 300_000])?;
        assert!(packed.starts_with(BLOB_ZSTD_MAGIC));
        for length in [299_999_u64, 300_001] {
            let mut invalid = packed.clone();
            invalid[BLOB_ZSTD_MAGIC.len()..BLOB_ZSTD_MAGIC.len() + 8]
                .copy_from_slice(&length.to_le_bytes());
            // Empty current must still consume/validate every base byte.
            assert!(pack_delta_from_packed_base(&invalid, &[]).is_err());
        }
        assert!(pack_delta_from_packed_base(&packed[..packed.len() - 1], &[]).is_err());
        assert!(pack_delta_from_packed_base(b"wrong format", &[]).is_err());
        Ok(())
    }

    #[test]
    fn base_chunk_stream_preserves_global_window_across_short_reads() -> Result<(), Error> {
        struct ShortReads<'a>(&'a [u8], usize);
        impl Read for ShortReads<'_> {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                let count = output.len().min(self.1);
                self.0.read(&mut output[..count])
            }
        }
        let mut state = 51_u64;
        let base = (0..400_000)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (state >> 56) as u8
            })
            .collect::<Vec<_>>();
        let expected = content_defined_ranges(&base);
        for size in [1, 7, 63, 8192, 65536] {
            let mut ranges = Vec::new();
            visit_base_chunks(ShortReads(&base, size), base.len(), |at, bytes| {
                assert!(bytes.len() <= CHUNK_MAX_BYTES);
                assert_eq!(bytes, &base[at..at + bytes.len()]);
                ranges.push(at..at + bytes.len());
            })?;
            assert_eq!(ranges, expected);
        }
        Ok(())
    }

    #[test]
    fn cache_payload_batches_writes_preserves_bytes_and_propagates_flush_failure(
    ) -> Result<(), Error> {
        struct Writes {
            bytes: Vec<u8>,
            calls: usize,
            fail: bool,
        }
        impl Write for Writes {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.calls += 1;
                if self.fail {
                    return Err(std::io::Error::other("injected sink failure"));
                }
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let value = (0..40_000)
            .map(|index| json!({"name": format!("한글-{index}"), "state": true}))
            .collect::<Vec<_>>();
        let mut direct = Writes {
            bytes: Vec::new(),
            calls: 0,
            fail: false,
        };
        rmp_serde::encode::write_named(&mut direct, &value)
            .map_err(|error| Error::with_source("cannot encode test reference", error))?;
        let mut buffered = Writes {
            bytes: Vec::new(),
            calls: 0,
            fail: false,
        };
        write_analysis_cache_payload(&mut buffered, &value)?;
        assert_eq!(buffered.bytes, direct.bytes);
        assert!(direct.calls > 100_000);
        assert!(buffered.calls <= direct.bytes.len() / (64 * 1024) + 1);
        let mut actual = Vec::new();
        write_analysis_cache_value(&mut actual, &value)?;
        // Pin current compressed bytes against an unbuffered reference. Both
        // ordinary zstd levels remain the same complete, current cache format.
        for level in [1, 3] {
            let mut reference = ANALYSIS_CACHE_MAGIC.to_vec();
            let mut encoder = zstd::stream::write::Encoder::new(&mut reference, level)
                .map_err(|error| Error::with_source("cannot create reference encoder", error))?;
            encoder
                .window_log(23)
                .and_then(|_| encoder.long_distance_matching(true))
                .map_err(|error| Error::with_source("cannot configure reference encoder", error))?;
            rmp_serde::encode::write_named(&mut encoder, &value)
                .map_err(|error| Error::with_source("cannot write reference cache", error))?;
            encoder
                .finish()
                .map_err(|error| Error::with_source("cannot finish reference cache", error))?;
            assert_eq!(
                read_analysis_cache_value::<serde_json::Value>(reference.as_slice())?,
                json!(value)
            );
            if level == 3 {
                assert_eq!(actual, reference);
            }
        }
        let mut failing = Writes {
            bytes: Vec::new(),
            calls: 0,
            fail: true,
        };
        assert!(write_analysis_cache_payload(&mut failing, &"small final buffer").is_err());
        assert!(failing.calls > 0);
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
        // Every prior current-format compression level uses the sole decoder;
        // complete metadata and history are never migrated or relabelled.
        let metadata_bytes = serde_json::to_vec(&metadata)?;
        assert_eq!(
            unpack_blob(&pack_metadata(&metadata_bytes)?)?,
            metadata_bytes
        );
        for level in [1, 19] {
            assert_eq!(
                unpack_blob(&pack_blob_at_level(&metadata_bytes, level)?)?,
                metadata_bytes
            );
        }
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
