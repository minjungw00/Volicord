//! Explicit repository-scoped evidence. No Work lookup, ambient file fallback,
//! model dispatch, second truth store or read-time retention renewal.
use crate::{
    operations::{new_operation_id, now_micros},
    source_observations::{file_snapshot_binding, retention_until},
    Error, FileByteRange, LocalOperations,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use volicord_context::{
    Availability, CanonicalReadBasis, Principal, PrincipalKind, ProjectId, SourceDraft, SourceId,
    SourcePayload,
};
use volicord_privacy::{
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedKind, PrivacyStore,
};
use volicord_projections::{explanation_text_hash, ExplanationPlan};
use volicord_repository_intelligence::{AnalysisMetadata, AnalysisSnapshotId, CanonicalGrounding};

const KIND: &str = "volicord_repository_source_observation";
const VERSION: u32 = 2;

#[derive(Clone, Debug)]
pub struct RepositoryFileObservationRequest {
    pub project_id: ProjectId,
    pub analysis_snapshot_id: AnalysisSnapshotId,
    pub path: String,
    pub byte_range: Option<FileByteRange>,
    pub retain_seconds: u64,
}
/// Ephemeral host-authored input. Product derives mechanical citation metadata;
/// this is not a second retained format or a semantic approval.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryExplanationDraft {
    #[serde(default)]
    pub diagrams: Vec<volicord_projections::GroundedDiagram>,
    pub plan_fingerprint: String,
    pub language: String,
    pub generator: volicord_projections::ExplanationGenerator,
    pub paragraphs: Vec<RepositoryReaderUnit>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryReaderUnit {
    pub question: volicord_projections::ExplanationQuestion,
    pub text: String,
    pub evidence_keys: Vec<String>,
    pub sources: Vec<RepositoryClaimRange>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryClaimRange {
    #[serde(with = "source_identity")]
    pub source_id: SourceId,
    pub start_byte: usize,
    pub end_byte: usize,
    /// None binds the complete authored unit. Explicit subclaims are UTF-8 bytes.
    pub claim: Option<FileByteRange>,
}
mod source_identity {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        id: &SourceId,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(id)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<SourceId, D::Error> {
        let text = String::deserialize(deserializer)?;
        crate::operations::parse_identity(&text)
            .map(SourceId::from_bytes)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RepositoryObservation {
    format_kind: String,
    format_version: u32,
    project_id: ProjectId,
    analysis_snapshot_id: AnalysisSnapshotId,
    repository_snapshot_id: String,
    repository_source_id: SourceId,
    binding_id: String,
    binding_revision: u64,
    source_id: SourceId,
    source_revision: u64,
    observed_at_unix_micros: i64,
    retained_until_unix_micros: i64,
    detail: Value,
}
impl RepositoryObservation {
    fn identity_basis(&self) -> Value {
        json!([
            self.project_id,
            self.analysis_snapshot_id,
            self.repository_snapshot_id,
            self.repository_source_id,
            self.binding_id,
            self.binding_revision,
            self.observed_at_unix_micros,
            self.retained_until_unix_micros
        ])
    }
}
fn repository_file_binding(observation: &RepositoryObservation) -> Result<String, Error> {
    let file = file_snapshot_binding(&observation.detail, &observation.identity_basis())?;
    Ok(format!(
        "{file};repository-basis:{}",
        explanation_text_hash(
            &json!([observation.identity_basis(), observation.detail]).to_string()
        )
    ))
}
fn purpose(analysis: AnalysisSnapshotId, source: SourceId) -> String {
    format!("repository-source:{analysis}:{source}")
}

/// Commit to the complete ordered area set without copying every path into each
/// capability and retained reader document. Exact sets remain in analysis evidence.
fn area_summary(areas: &[volicord_repository_intelligence::AreaId]) -> Value {
    use volicord_repository_intelligence::AreaKind;
    if areas.is_empty() {
        return json!({"area_count":0,"file_count":0});
    }
    let files = areas
        .iter()
        .filter(|a| a.kind == AreaKind::File)
        .collect::<Vec<_>>();
    let mut common: Option<Vec<&str>> = None;
    for file in &files {
        let mut parent = file.path.split('/').collect::<Vec<_>>();
        parent.pop();
        match &mut common {
            None => common = Some(parent),
            Some(prefix) => {
                let count = prefix
                    .iter()
                    .zip(parent)
                    .take_while(|(a, b)| **a == *b)
                    .count();
                prefix.truncate(count);
            }
        }
    }
    let common = common.unwrap_or_default().join("/");
    json!({"area_count":areas.len(),"file_count":files.len(),
        "common_file_area":if common.is_empty() {"."} else {&common},
        "sha256":explanation_text_hash(&json!(areas).to_string())})
}
/// Fresh request-owned comparison, never persisted or accepted from a host.
/// Constructed only after complete included/uncited repository comparison.
pub(crate) struct RepositoryExplanationReadBasis {
    pub(crate) analysis: AnalysisMetadata,
    pub(crate) repository: volicord_repository_intelligence::RepositorySnapshot,
}

impl LocalOperations {
    pub fn compose_repository_explanation(
        &self,
        project: ProjectId,
        analysis: AnalysisSnapshotId,
        draft: RepositoryExplanationDraft,
    ) -> Result<volicord_projections::ExplanationRealization, Error> {
        use volicord_projections::*;
        let plan = self.prepare_explanation(
            project,
            ExplanationSubject::Repository(analysis),
            &draft.language,
        )?;
        if draft.plan_fingerprint != plan.fingerprint {
            return Err(Error::new("stale repository preparation; prepare again"));
        }
        if draft.paragraphs.len() > 32 || draft.paragraphs.iter().any(|p| p.sources.len() > 32) {
            return Err(Error::new(
                "repository document exceeds unit/citation bound",
            ));
        }
        let mut paragraphs = Vec::new();
        for unit in draft.paragraphs {
            let mut citations = Vec::new();
            for selected in unit.sources {
                let read =
                    self.read_repository_explanation_source(project, analysis, selected.source_id)?;
                let detail = &read["observation"]["detail"];
                let body = detail["body"]
                    .as_str()
                    .ok_or_else(|| Error::new("Source body unavailable"))?;
                let base = detail["region"]["start_byte"].as_u64().unwrap_or(0) as usize;
                let line_base = detail["region"]["start_line"].as_u64().unwrap_or(1) as usize;
                let start = selected
                    .start_byte
                    .checked_sub(base)
                    .ok_or_else(|| Error::new("Source range precedes observation"))?;
                let end = selected
                    .end_byte
                    .checked_sub(base)
                    .ok_or_else(|| Error::new("Source range precedes observation"))?;
                let range = body
                    .get(start..end)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| Error::new("invalid Source UTF-8 range"))?;
                let claim = selected.claim.unwrap_or(FileByteRange {
                    start_byte: 0,
                    end_byte: unit.text.len(),
                });
                let claim_text = unit
                    .text
                    .get(claim.start_byte..claim.end_byte)
                    .filter(|s| !s.trim().is_empty())
                    .ok_or_else(|| Error::new("invalid claim UTF-8 range"))?;
                let line = |offset| {
                    line_base
                        + body.as_bytes()[..offset]
                            .iter()
                            .filter(|b| **b == b'\n')
                            .count()
                };
                citations.push(ExplanationCitation {
                    source_id: selected.source_id.to_string(),
                    source_revision: 1,
                    observation_fingerprint: read["observation_fingerprint"]
                        .as_str()
                        .ok_or_else(|| Error::new("Source fingerprint unavailable"))?
                        .into(),
                    analysis_snapshot_id: analysis.to_string(),
                    repository_snapshot_id: read["observation"]["repository_snapshot_id"]
                        .as_str()
                        .ok_or_else(|| Error::new("Repository Snapshot unavailable"))?
                        .into(),
                    repository_source_id: read["citation_basis"]["repository_source_id"]
                        .as_str()
                        .ok_or_else(|| Error::new("repository Source unavailable"))?
                        .into(),
                    locator: detail["path"]
                        .as_str()
                        .ok_or_else(|| Error::new("Source locator unavailable"))?
                        .into(),
                    start_byte: selected.start_byte,
                    end_byte: selected.end_byte,
                    start_line: line(start),
                    end_line: line(end - 1),
                    range_sha256: explanation_text_hash(range),
                    claim_start_byte: claim.start_byte,
                    claim_end_byte: claim.end_byte,
                    claim_sha256: explanation_text_hash(claim_text),
                    field: "body".into(),
                    use_kind: ExplanationSourceUse::CurrentCode,
                });
            }
            paragraphs.push(ExplanationParagraph {
                question: unit.question,
                text: unit.text,
                evidence_keys: unit.evidence_keys,
                citations,
            });
        }
        let mut response = ExplanationRealization {
            format_kind: EXPLANATION_KIND.into(),
            format_version: EXPLANATION_VERSION,
            plan_fingerprint: draft.plan_fingerprint,
            language: draft.language,
            generator: draft.generator,
            paragraphs,
            diagrams: draft.diagrams,
        };
        seal_diagrams(&mut response).map_err(Error::new)?;
        validate_explanation(&plan, &response).map_err(Error::new)?;
        self.validate_subject_sources(
            &self.canonical_basis(project)?,
            ExplanationSubject::Repository(analysis),
            &response,
        )?;
        Ok(response)
    }
    /// Explicit exact-snapshot metadata inspection. No Source bodies or current
    /// filesystem fallback, generation, retention renewal or authority upgrade.
    pub fn repository_explanation_evidence(
        &self,
        project: ProjectId,
        id: AnalysisSnapshotId,
    ) -> Result<Value, Error> {
        self.canonical_basis(project)?;
        let analysis = self.load_analysis_snapshot(project, id)?;
        Ok(
            json!({"analysis_metadata":volicord_repository_intelligence::AnalysisMetadata::from(&analysis),
            "diagnostics":analysis.diagnostics,"repository_worktree":analysis.repository_worktree,"read_only":true,"evidence_use":"historical_stored_analysis_not_complete_semantic_understanding"}),
        )
    }
    pub(crate) fn repository_explanation_plan(
        &self,
        canonical: &CanonicalReadBasis,
        id: AnalysisSnapshotId,
        language: &str,
    ) -> Result<ExplanationPlan, Error> {
        let basis = self.repository_explanation_read_basis(canonical, id)?;
        self.repository_explanation_plan_from_read(canonical, &basis, language)
    }
    pub(crate) fn repository_explanation_read_basis(
        &self,
        canonical: &CanonicalReadBasis,
        id: AnalysisSnapshotId,
    ) -> Result<RepositoryExplanationReadBasis, Error> {
        let analysis = self.repository_source_metadata(canonical.project.id, id)?;
        let repository = self.current_explanation_repository(canonical, &analysis)?;
        Ok(RepositoryExplanationReadBasis {
            analysis,
            repository,
        })
    }
    pub(crate) fn repository_explanation_plan_from_read(
        &self,
        canonical: &CanonicalReadBasis,
        basis: &RepositoryExplanationReadBasis,
        language: &str,
    ) -> Result<ExplanationPlan, Error> {
        let analysis = &basis.analysis;
        let id = analysis.identity;
        if analysis.project.identity() != canonical.project.id
            || basis.repository.identity != analysis.repository_snapshot
        {
            return Err(Error::new("foreign repository explanation read basis"));
        }
        let mut repository = serde_json::to_value(&basis.repository)
            .map_err(|e| Error::with_source("cannot encode repository basis", e))?;
        // Read-time observation time is not historical evidence and must not
        // change the retained basis on every GET.
        if let Some(object) = repository.as_object_mut() {
            object.remove("observed_at_unix_micros");
            for key in ["included_areas", "excluded_areas", "unavailable_areas"] {
                let areas: Vec<volicord_repository_intelligence::AreaId> =
                    serde_json::from_value(object[key].clone())
                        .map_err(|e| Error::with_source("cannot decode repository scope", e))?;
                object.insert(key.into(), area_summary(&areas));
            }
        }
        let binding = self
            .open_canonical_read_only()?
            .get_local_binding(canonical.project.id)
            .map_err(|e| Error::with_source("repository binding unavailable", e))?;
        let mut classifications = std::collections::BTreeMap::<String, usize>::new();
        for entry in &analysis.inventory.entries {
            for class in &entry.classifications {
                let value = serde_json::to_value(class)
                    .map_err(|e| Error::with_source("cannot encode inventory classification", e))?;
                let key = value
                    .as_str()
                    .ok_or_else(|| Error::new("invalid inventory classification"))?;
                *classifications.entry(key.into()).or_default() += 1;
            }
        }
        let scope = json!({"entry_count":analysis.inventory.entries.len(),
            "classification_counts":classifications,
            "sha256":explanation_text_hash(&json!(analysis.inventory).to_string()),
            "entries":"not_embedded; complete inventory in analysis evidence"});
        let capabilities: Vec<_> = analysis.capabilities.iter().map(|report| {
            let coverage = &report.coverage;
            // Repeated diagnostic IDs can be megabytes. Preserve their exact
            // count/hash and expose the full immutable metadata explicitly.
            json!({"language":report.language,"area":report.area,"capability":report.capability,"state":report.state,
                "reason":report.reason,"usable_remainder":report.usable_remainder,"user_visible_consequence":report.user_visible_consequence,
                "coverage":{"included":area_summary(&coverage.included),"excluded":area_summary(&coverage.excluded),"unsupported":area_summary(&coverage.unsupported),
                    "unavailable":area_summary(&coverage.unavailable),"failed":area_summary(&coverage.failed),"stale":area_summary(&coverage.stale),
                    "covered_file_count":coverage.covered_file_count,"covered_entity_count":coverage.covered_entity_count,"covered_relation_count":coverage.covered_relation_count},
                "diagnostic_count":report.diagnostics.len(),"diagnostics_sha256":explanation_text_hash(&json!(report.diagnostics).to_string()),
                "diagnostic_detail":"repository explain evidence --analysis exact_analysis_snapshot_id",
                "adapter":report.adapter,"analyzer":report.analyzer,"provenance_class":report.provenance_class,"freshness":report.freshness,
                "uncertainty_level":report.uncertainty.level,"uncertainty_reason_count":report.uncertainty.reasons.len(),
                "uncertainty_reasons_sha256":explanation_text_hash(&json!(report.uncertainty.reasons).to_string())})
        }).collect();
        volicord_projections::prepare_repository_explanation(canonical, id,
            analysis.repository_source.identity(), json!({"repository":repository,
                "analysis_snapshot_id":id, "analysis_format_version":analysis.format_version,
                "binding_id":binding.id.to_string(), "binding_revision":binding.revision,
                "inventory_scope":scope,
                "scope_detail":{"project_id":canonical.project.id.to_string(),"analysis_snapshot_id":id,
                    "command":format!("repository explain evidence --analysis {id}"),
                    "omitted_from_preparation":"inventory entries, coverage path sets and historical worktree detail; counts and hashes bind complete sets, not a selected sample",
                    "validation":"verified analysis metadata and current repository; graph integrity and complete semantic understanding not established",
                    "investigated_source_scope":"only explicitly observed/read Sources; inventory and capability do not establish authored topic coverage"},
                "capabilities":capabilities, "freshness":analysis.freshness,
                "generated_at_unix_micros":analysis.generated_at_unix_micros,
                "semantic_completeness":"not_established", "purpose_role":"inferred_repository_summary_is_not_canonical_project_purpose"}), language).map_err(Error::new)
    }
    pub(crate) fn current_explanation_repository(
        &self,
        canonical: &CanonicalReadBasis,
        analysis: &AnalysisMetadata,
    ) -> Result<volicord_repository_intelligence::RepositorySnapshot, Error> {
        let binding = self
            .open_canonical_read_only()?
            .get_local_binding(canonical.project.id)
            .map_err(|e| Error::with_source("repository binding unavailable", e))?;
        let root = volicord_local_platform::RepositoryRoot::open(&binding.absolute_path)
            .map_err(|e| Error::with_source("repository unavailable", e))?;
        if binding.availability != Availability::Available
            || root.canonical_path() != binding.absolute_path
        {
            return Err(Error::new("repository binding unavailable or redirected"));
        }
        let repository = self.observe_projection_repository(
            canonical,
            analysis.repository_source.identity(),
            &analysis.inventory.entries,
        )?;
        if repository.identity != analysis.repository_snapshot {
            return Err(Error::new(
                "repository differs from the selected Analysis Snapshot; analyze explicitly again",
            )
            .with_reading_failure(volicord_projections::ExplanationFailureReason::Changed));
        }
        Ok(repository)
    }
    pub fn observe_repository_file(
        &self,
        request: RepositoryFileObservationRequest,
    ) -> Result<Value, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        retention_until(now_micros()?, request.retain_seconds)?;
        let canonical = self.canonical_basis(request.project_id)?;
        let analysis =
            self.repository_source_metadata(request.project_id, request.analysis_snapshot_id)?;
        self.current_explanation_repository(&canonical, &analysis)?;
        let binding = self
            .open_canonical_read_only()?
            .get_local_binding(request.project_id)
            .map_err(|e| Error::with_source("repository binding unavailable", e))?;
        let detail = self.capture_inventory_file(
            &canonical,
            &analysis.inventory.entries,
            &request.path,
            request.byte_range,
        )?;
        let now = now_micros()?;
        let until = retention_until(now, request.retain_seconds)?;
        let mut observation = RepositoryObservation {
            format_kind: KIND.into(),
            format_version: VERSION,
            project_id: request.project_id,
            analysis_snapshot_id: analysis.identity,
            repository_snapshot_id: analysis.repository_snapshot.to_string(),
            repository_source_id: analysis.repository_source.identity(),
            binding_id: binding.id.to_string(),
            binding_revision: binding.revision,
            source_id: SourceId::from_bytes([0; 16]),
            source_revision: 1,
            observed_at_unix_micros: now.as_unix_micros(),
            retained_until_unix_micros: until.as_unix_micros(),
            detail,
        };
        // Open/check both stores before publishing the canonical Source identity.
        let mut privacy = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|e| Error::with_source("Source storage unavailable", e))?;
        let source = self
            .open_canonical()?
            .record_source(
                new_operation_id()?,
                request.project_id,
                SourceDraft {
                    expected_project_revision: canonical.project.revision,
                    payload: SourcePayload::File {
                        locator: request.path,
                        snapshot: repository_file_binding(&observation)?,
                    },
                    actor: Principal {
                        kind: PrincipalKind::Repository,
                        identity: "volicord-local-source-observer".into(),
                    },
                    observer: Some(Principal {
                        kind: PrincipalKind::Agent,
                        identity: "volicord-local-operations".into(),
                    }),
                    availability: Availability::Available,
                },
            )
            .map_err(|e| Error::with_source("cannot record repository Source", e))?
            .value;
        observation.source_id = source.id;
        let current = self.canonical_basis(request.project_id)?;
        let grounding = CanonicalGrounding::from_read_basis(&current)
            .map_err(|e| Error::with_source("cannot ground Source", e))?;
        let ids = [source.id, analysis.repository_source.identity()];
        let included_sources = ids
            .iter()
            .map(|id| {
                grounding
                    .source_reference(*id)
                    .map_err(|e| Error::with_source("Source unavailable", e))
            })
            .collect::<Result<Vec<_>, _>>()?;
        privacy
            .record_managed_derived(ManagedDerivedDraft {
                project_id: request.project_id,
                kind: ManagedDerivedKind::SourceObservation,
                provider: None,
                model: None,
                purpose: purpose(analysis.identity, source.id),
                analysis_snapshot: Some(analysis.identity),
                included_sources,
                canonical_links: vec![
                    ManagedCanonicalLink::Project(request.project_id),
                    ManagedCanonicalLink::Source(source.id),
                    ManagedCanonicalLink::Source(analysis.repository_source.identity()),
                ],
                content: serde_json::to_string(&observation)
                    .map_err(|e| Error::with_source("cannot encode observation", e))?,
                uncertainty: None,
                retained_until: Some(until),
                retention_basis:
                    "Explicit bounded repository Source observation; no background transmission"
                        .into(),
            })
            .map_err(|e| {
                Error::new(format!(
                    "Source {} recorded but body retention failed: {e}; body unavailable",
                    source.id
                ))
            })?;
        Ok(
            json!({"project_id":request.project_id.to_string(),"analysis_snapshot_id":analysis.identity,"source_id":source.id.to_string(),"source_revision":1,
            "retained_until_unix_micros":until.as_unix_micros()}),
        )
    }
    pub(crate) fn repository_source_metadata(
        &self,
        project: ProjectId,
        id: AnalysisSnapshotId,
    ) -> Result<volicord_repository_intelligence::AnalysisMetadata, Error> {
        let path = self
            .layout()
            .analysis_project_dir(project)
            .join(format!("{id}.json"));
        let manifest = crate::analysis_io::read_manifest(&path)?;
        let metadata = crate::analysis_io::read_metadata(&path, &manifest)?;
        if metadata.project.identity() != project || metadata.identity != id {
            return Err(Error::new(
                "repository Source Analysis metadata belongs to another Project or snapshot",
            ));
        }
        Ok(metadata)
    }
    /// Historical retained bytes only. Current repository equality is a separate
    /// explanation admission/publication check, never a fallback body source.
    pub fn read_repository_explanation_source(
        &self,
        project: ProjectId,
        analysis_id: AnalysisSnapshotId,
        source: SourceId,
    ) -> Result<Value, Error> {
        let canonical = self.canonical_basis(project)?;
        let analysis = self.repository_source_metadata(project, analysis_id)?;
        let inspection = self.privacy_status(project)?;
        self.read_repository_explanation_source_with_basis(
            &canonical,
            &analysis,
            &inspection,
            source,
        )
    }

    /// Request-owned inputs may be shared across exact Source reads. They are
    /// observations, never authority for another request; every body still passes
    /// its own association, lifetime, canonical binding and integrity checks.
    pub(crate) fn read_repository_explanation_source_with_basis(
        &self,
        canonical: &CanonicalReadBasis,
        analysis: &AnalysisMetadata,
        inspection: &volicord_privacy::ProjectPrivacyInspection,
        source: SourceId,
    ) -> Result<Value, Error> {
        let project = canonical.project.id;
        let analysis_id = analysis.identity;
        if inspection.project_id != project || analysis.project.identity() != project {
            return Err(Error::new(
                "repository Source read bases belong to another Project",
            ));
        }
        let record = inspection.managed_derived.iter().find(|r| r.kind == ManagedDerivedKind::SourceObservation && r.purpose == purpose(analysis_id,source))
            .ok_or_else(|| Error::new("Source has no explicit association with this Project/repository/Analysis Snapshot"))?;
        let now = now_micros()?;
        if let Some(failure) = crate::source_observations::observation_lifetime_failure(
            record,
            inspection
                .withheld_for_canonical_forgetting
                .contains(&record.id),
            now,
        ) {
            return Err(Error::new(
                "repository Source copy deleted, expired, invalidated or withheld",
            )
            .with_reading_failure(failure));
        }
        let stored = PrivacyStore::open(self.layout().privacy_store())
            .and_then(|s| s.get_derived(project, record.id))
            .map_err(|e| Error::with_source("Source copy unavailable", e))?;
        let content = stored
            .content
            .as_deref()
            .ok_or_else(|| Error::new("Source copy unavailable"))?;
        if content.len() > volicord_privacy::MANAGED_DERIVED_CONTENT_BYTE_LIMIT {
            return Err(Error::new("Source copy exceeds content bound"));
        }
        let header: Value = serde_json::from_str(content).map_err(|_| {
            Error::new("corrupt repository Source copy")
                .with_reading_failure(volicord_projections::ExplanationFailureReason::Corrupt)
        })?;
        if header["format_kind"] != KIND || header["format_version"] != VERSION {
            return Err(Error::new(
                "unsupported repository Source observation; explicitly observe again",
            )
            .with_reading_failure(volicord_projections::ExplanationFailureReason::Unsupported));
        }
        let observation: RepositoryObservation = serde_json::from_value(header).map_err(|_| {
            Error::new("corrupt repository Source copy shape")
                .with_reading_failure(volicord_projections::ExplanationFailureReason::Corrupt)
        })?;
        let binding = self
            .open_canonical_read_only()?
            .get_local_binding(project)
            .map_err(|e| Error::with_source("repository binding unavailable", e))?;
        if observation.project_id != project
            || observation.analysis_snapshot_id != analysis_id
            || observation.source_id != source
            || observation.source_revision != 1
            || binding.availability != Availability::Available
            || record.retained_until.map(|t| t.as_unix_micros())
                != Some(observation.retained_until_unix_micros)
            || observation.repository_snapshot_id != analysis.repository_snapshot.to_string()
            || observation.repository_source_id != analysis.repository_source.identity()
            || record.analysis_snapshot != Some(analysis_id)
            || observation.binding_id != binding.id.to_string()
            || observation.binding_revision != binding.revision
            || observation.observed_at_unix_micros <= 0
            || observation.observed_at_unix_micros > stored.created_at.as_unix_micros()
        {
            return Err(Error::new("repository Source identity/binding mismatch"));
        }
        for id in [source, observation.repository_source_id] {
            if !canonical
                .sources
                .iter()
                .any(|s| s.source.id == id && s.availability == Availability::Available)
                || !record.included_sources.iter().any(|s| s.identity() == id)
            {
                return Err(Error::new("repository Source unavailable or withheld"));
            }
        }
        let original = canonical
            .sources
            .iter()
            .find(|s| s.source.id == source)
            .ok_or_else(|| Error::new("Source unavailable"))?;
        let revision = canonical
            .revisions
            .iter()
            .find(|r| {
                r.record_kind == volicord_context::CanonicalRecordKind::Source
                    && r.record_identity == source.to_string()
            })
            .and_then(|r| r.revisions.last().copied());
        if revision != Some(observation.source_revision)
            || observation.observed_at_unix_micros > original.source.recorded_at.as_unix_micros()
            || observation.detail["kind"] != "file"
        {
            return Err(Error::new(
                "repository Source kind, revision or chronology mismatch",
            ));
        }
        let locator = observation.detail["path"]
            .as_str()
            .ok_or_else(|| Error::new("corrupt Source locator"))?;
        let hash = observation.detail["sha256"]
            .as_str()
            .ok_or_else(|| Error::new("corrupt Source hash"))?;
        if !analysis.inventory.entries.iter().any(|e| {
            e.area.path == locator
                && e.classifications
                    .contains(&volicord_repository_intelligence::InventoryClassification::Included)
                && e.content_sha256.as_deref() == Some(hash)
        }) {
            return Err(Error::new(
                "repository Source is outside included hashed inventory",
            ));
        }
        let snapshot = repository_file_binding(&observation)?;
        if !matches!(&original.source.payload, SourcePayload::File{locator:actual,snapshot:bound} if actual==locator && bound==&snapshot)
        {
            return Err(Error::new(
                "repository Source content/hash/coordinates mismatch",
            ));
        }
        let observation = serde_json::to_value(observation)
            .map_err(|e| Error::with_source("cannot encode Source", e))?;
        let fingerprint = explanation_text_hash(
            &serde_json::to_string(&observation)
                .map_err(|e| Error::with_source("cannot encode Source", e))?,
        );
        Ok(
            json!({"observation":observation,"observation_fingerprint":fingerprint,
            "citation_basis":{"source_id":source.to_string(),"source_revision":1,"observation_fingerprint":fingerprint,
                "analysis_snapshot_id":analysis_id,"repository_snapshot_id":analysis.repository_snapshot,"repository_source_id":analysis.repository_source.identity().to_string()},
            "metadata":{"retained_until_unix_micros":record.retained_until.map(|t|t.as_unix_micros())},"scope_validation":"verified_analysis_metadata_inventory_not_graph_completeness","read_only":true}),
        )
    }
    pub fn delete_repository_source_observation(
        &self,
        project: ProjectId,
        analysis: AnalysisSnapshotId,
        source: SourceId,
    ) -> Result<usize, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        self.canonical_basis(project)?;
        let mut store = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|e| Error::with_source("Source storage unavailable", e))?;
        let ids: Vec<_> = store
            .inspect_project(project)
            .map_err(|e| Error::with_source("Source inspection failed", e))?
            .managed_derived
            .iter()
            .filter(|r| {
                r.kind == ManagedDerivedKind::SourceObservation
                    && r.purpose == purpose(analysis, source)
            })
            .map(|r| r.id)
            .collect();
        if ids.is_empty() {
            return Err(Error::new("Source has no exact repository association"));
        }
        store
            .delete_managed_ids(project, &ids, "Explicit repository Source copy deletion")
            .map_err(|e| Error::with_source("Source deletion incomplete; retry", e))?;
        Ok(ids.len())
    }
}
