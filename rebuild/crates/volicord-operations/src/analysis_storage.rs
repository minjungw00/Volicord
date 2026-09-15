use crate::{Error, LocalOperations};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
use volicord_context::ProjectId;
use volicord_repository_intelligence::{AnalysisSnapshot, AnalysisSnapshotId, InventoryEntry};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AnalysisSectionFootprint {
    pub name: String,
    pub logical_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AnalysisFileFootprint {
    pub analysis_snapshot: AnalysisSnapshotId,
    pub logical_bytes: u64,
    pub physical_bytes: Option<u64>,
    pub entity_count: u64,
    pub relation_count: u64,
    pub bytes_per_graph_item: u64,
    pub sections: Vec<AnalysisSectionFootprint>,
    pub content_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AnalysisReachabilityReference {
    pub analysis_snapshot: AnalysisSnapshotId,
    pub owner: String,
    pub record: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AnalysisStorageFootprint {
    pub project_id: ProjectId,
    pub snapshot_count: u64,
    pub blob_count: u64,
    pub logical_bytes: u64,
    pub physical_bytes: Option<u64>,
    pub unchanged_repeat_delta_bytes: Option<u64>,
    pub reusable_content_overlap_millionths: Option<u64>,
    pub snapshots: Vec<AnalysisFileFootprint>,
    pub reachable_from: Vec<AnalysisReachabilityReference>,
}

impl LocalOperations {
    /// Measures persisted analysis cost and the durable records that name historical analyses.
    /// It is intentionally diagnostic: it does not mutate or collect derived state.
    pub fn analysis_storage_footprint(
        &self,
        project_id: ProjectId,
    ) -> Result<AnalysisStorageFootprint, Error> {
        let mut snapshots = Vec::new();
        let mut decoded = Vec::new();
        for path in self.analysis_paths(project_id)? {
            let analysis = crate::analysis_io::read_analysis(&path)?;
            let manifest = crate::analysis_io::read_manifest(&path)?;
            snapshots.push(measure_file(&path, &analysis)?);
            decoded.push((analysis, manifest, path));
        }
        snapshots.sort_by_key(|value| value.analysis_snapshot);
        decoded.sort_by_key(|value| (value.0.generated_at_unix_micros, value.0.identity));
        let (logical_bytes, physical_bytes, blob_count) =
            allocated_tree(&self.layout().analysis_project_dir(project_id))?;
        let unchanged_repeat_delta_bytes = decoded
            .windows(2)
            .filter(|pair| {
                inventory_overlap(&pair[0].0.inventory.entries, &pair[1].0.inventory.entries)
                    == 1_000_000
            })
            .last()
            .map(|pair| incremental_bytes(&pair[0].1, &pair[1].1, &pair[1].2))
            .transpose()?;
        let reusable_content_overlap_millionths = decoded.windows(2).last().map(|pair| {
            inventory_overlap(&pair[0].0.inventory.entries, &pair[1].0.inventory.entries)
        });
        let identities = decoded
            .iter()
            .map(|value| value.0.identity)
            .collect::<Vec<_>>();
        let reachable_from = self.analysis_reachability(project_id, &identities)?;
        Ok(AnalysisStorageFootprint {
            project_id,
            snapshot_count: snapshots.len() as u64,
            blob_count,
            logical_bytes,
            physical_bytes,
            unchanged_repeat_delta_bytes,
            reusable_content_overlap_millionths,
            snapshots,
            reachable_from,
        })
    }

    fn analysis_reachability(
        &self,
        project_id: ProjectId,
        identities: &[AnalysisSnapshotId],
    ) -> Result<Vec<AnalysisReachabilityReference>, Error> {
        let mut references = Vec::new();
        let candidates = volicord_inquiry::CandidateStore::open(self.layout().candidate_store())
            .map_err(|error| {
                Error::with_source("cannot inspect Candidate analysis references", error)
            })?
            .read_basis(project_id)
            .map_err(|error| {
                Error::with_source("cannot read Candidate analysis references", error)
            })?;
        for candidate in candidates.candidates {
            if let Some(content) = candidate.content {
                collect_named_references(
                    &serde_json::to_value(content)
                        .map_err(|error| Error::with_source("cannot inspect Candidate", error))?,
                    identities,
                    "candidate",
                    &candidate.id.to_string(),
                    &mut references,
                );
            }
        }
        let privacy = volicord_privacy::PrivacyStore::open(self.layout().privacy_store())
            .map_err(|error| {
                Error::with_source("cannot inspect privacy analysis references", error)
            })?
            .inspect_project(project_id)
            .map_err(|error| {
                Error::with_source("cannot read privacy analysis references", error)
            })?;
        for request in privacy.requests {
            if identities.contains(&request.analysis_snapshot) {
                references.push(AnalysisReachabilityReference {
                    analysis_snapshot: request.analysis_snapshot,
                    owner: "provider_request".into(),
                    record: request.id.to_string(),
                });
            }
        }
        for record in privacy.managed_derived {
            if let Some(analysis_snapshot) = record
                .analysis_snapshot
                .filter(|id| identities.contains(id))
            {
                references.push(AnalysisReachabilityReference {
                    analysis_snapshot,
                    owner: "managed_derived".into(),
                    record: record.id.to_string(),
                });
            }
        }
        references.sort_by(|left, right| {
            (&left.owner, &left.record, left.analysis_snapshot).cmp(&(
                &right.owner,
                &right.record,
                right.analysis_snapshot,
            ))
        });
        references.dedup();
        Ok(references)
    }
}

fn collect_named_references(
    value: &serde_json::Value,
    identities: &[AnalysisSnapshotId],
    owner: &str,
    record: &str,
    output: &mut Vec<AnalysisReachabilityReference>,
) {
    match value {
        serde_json::Value::String(value) => {
            if let Some(identity) = identities
                .iter()
                .find(|identity| identity.to_string() == *value)
            {
                output.push(AnalysisReachabilityReference {
                    analysis_snapshot: *identity,
                    owner: owner.into(),
                    record: record.into(),
                });
            }
        }
        serde_json::Value::Array(values) => values
            .iter()
            .for_each(|value| collect_named_references(value, identities, owner, record, output)),
        serde_json::Value::Object(values) => values
            .values()
            .for_each(|value| collect_named_references(value, identities, owner, record, output)),
        _ => {}
    }
}

fn measure_file(path: &Path, analysis: &AnalysisSnapshot) -> Result<AnalysisFileFootprint, Error> {
    let bytes = fs::read(path)
        .map_err(|error| Error::with_source("cannot measure Analysis Snapshot", error))?;
    let manifest = crate::analysis_io::read_manifest(path)?;
    let blobs = crate::analysis_io::blob_dir(path)?;
    let mut paths = vec![path.to_path_buf()];
    paths.extend(
        manifest
            .shape_blobs
            .iter()
            .map(|hash| blobs.join(format!("{hash}.shape"))),
    );
    paths.push(blobs.join(format!("{}.values", manifest.values_blob)));
    if let Some(base) = &manifest.values_base_blob {
        paths.push(blobs.join(format!("{base}.values")));
    }
    let metadata = paths
        .iter()
        .map(fs::metadata)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| Error::with_source("cannot inspect Analysis Snapshot storage", error))?;
    let entity_count = analysis.structural_facts.len() as u64;
    let relation_count = analysis
        .structural_facts
        .iter()
        .map(|fact| fact.relations.len() as u64)
        .sum::<u64>()
        + analysis.semantic_results.len() as u64;
    let graph_items = entity_count + relation_count;
    let sections = vec![
        section("inventory", &analysis.inventory)?,
        section("capabilities", &analysis.capabilities)?,
        section("diagnostics", &analysis.diagnostics)?,
        section("structural_facts", &analysis.structural_facts)?,
        section("semantic_results", &analysis.semantic_results)?,
        section("semantic_annotations", &analysis.semantic_annotations)?,
        section("agent_interpretations", &analysis.agent_interpretations)?,
        section("structural_bases", &analysis.structural_bases)?,
        section("semantic_bases", &analysis.semantic_bases)?,
        section("invalidations", &analysis.invalidations)?,
    ];
    Ok(AnalysisFileFootprint {
        analysis_snapshot: analysis.identity,
        logical_bytes: metadata.iter().map(fs::Metadata::len).sum(),
        physical_bytes: physical_supported()
            .then(|| metadata.iter().filter_map(physical_bytes).sum()),
        entity_count,
        relation_count,
        bytes_per_graph_item: if graph_items == 0 {
            0
        } else {
            manifest.logical_json_bytes / graph_items
        },
        sections,
        content_sha256: format!("{:x}", Sha256::digest(bytes)),
    })
}

fn allocated_tree(root: &Path) -> Result<(u64, Option<u64>, u64), Error> {
    if !root.exists() {
        return Ok((0, physical_supported().then_some(0), 0));
    }
    let mut logical = 0_u64;
    let mut physical = 0_u64;
    let mut blobs = 0_u64;
    for entry in fs::read_dir(root)
        .map_err(|error| Error::with_source("cannot inspect Analysis storage", error))?
    {
        let path = entry
            .map_err(|error| Error::with_source("cannot inspect Analysis storage entry", error))?
            .path();
        if path.is_dir() {
            for blob in fs::read_dir(path)
                .map_err(|error| Error::with_source("cannot inspect Analysis blobs", error))?
            {
                let metadata = blob
                    .map_err(|error| Error::with_source("cannot inspect Analysis blob", error))?
                    .metadata()
                    .map_err(|error| Error::with_source("cannot inspect Analysis blob", error))?;
                logical += metadata.len();
                physical += physical_bytes(&metadata).unwrap_or(0);
                blobs += 1;
            }
        } else {
            let metadata = fs::metadata(path)
                .map_err(|error| Error::with_source("cannot inspect Analysis manifest", error))?;
            logical += metadata.len();
            physical += physical_bytes(&metadata).unwrap_or(0);
        }
    }
    Ok((logical, physical_supported().then_some(physical), blobs))
}

fn incremental_bytes(
    previous: &crate::analysis_io::AnalysisManifest,
    current: &crate::analysis_io::AnalysisManifest,
    path: &Path,
) -> Result<u64, Error> {
    let mut bytes = fs::metadata(path)
        .map_err(|error| Error::with_source("cannot inspect Analysis manifest", error))?
        .len();
    let blobs = crate::analysis_io::blob_dir(path)?;
    for hash in current
        .shape_blobs
        .iter()
        .filter(|hash| !previous.shape_blobs.contains(hash))
    {
        bytes += fs::metadata(blobs.join(format!("{hash}.shape")))
            .map_err(|error| Error::with_source("cannot inspect Analysis shape", error))?
            .len();
    }
    if current.values_blob != previous.values_blob {
        bytes += fs::metadata(blobs.join(format!("{}.values", current.values_blob)))
            .map_err(|error| Error::with_source("cannot inspect Analysis values", error))?
            .len();
    }
    if let Some(base) = &current.values_base_blob {
        if previous.values_blob != *base && previous.values_base_blob.as_ref() != Some(base) {
            bytes += fs::metadata(blobs.join(format!("{base}.values")))
                .map_err(|error| Error::with_source("cannot inspect Analysis values base", error))?
                .len();
        }
    }
    Ok(bytes)
}

fn section(name: &str, value: &impl Serialize) -> Result<AnalysisSectionFootprint, Error> {
    let logical_bytes = serde_json::to_vec(value)
        .map_err(|error| Error::with_source("cannot measure Analysis Snapshot section", error))?
        .len() as u64;
    Ok(AnalysisSectionFootprint {
        name: name.into(),
        logical_bytes,
    })
}

#[cfg(unix)]
fn physical_bytes(metadata: &fs::Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    Some(metadata.blocks().saturating_mul(512))
}

#[cfg(not(unix))]
fn physical_bytes(_metadata: &fs::Metadata) -> Option<u64> {
    None
}

#[cfg(unix)]
fn physical_supported() -> bool {
    true
}

#[cfg(not(unix))]
fn physical_supported() -> bool {
    false
}

fn inventory_overlap(left: &[InventoryEntry], right: &[InventoryEntry]) -> u64 {
    let keyed = |entries: &[InventoryEntry]| {
        entries
            .iter()
            .filter_map(|entry| {
                entry.content_sha256.as_ref().map(|hash| {
                    (
                        entry.area.path.clone(),
                        (hash.clone(), entry.size_bytes.unwrap_or(0)),
                    )
                })
            })
            .collect::<BTreeMap<_, _>>()
    };
    let left = keyed(left);
    let right = keyed(right);
    let denominator = left
        .values()
        .map(|(_, size)| *size)
        .sum::<u64>()
        .max(right.values().map(|(_, size)| *size).sum::<u64>());
    if denominator == 0 {
        return if left == right { 1_000_000 } else { 0 };
    }
    let shared = left
        .iter()
        .filter_map(|(path, value)| (right.get(path) == Some(value)).then_some(value.1))
        .sum::<u64>();
    shared.saturating_mul(1_000_000) / denominator
}
