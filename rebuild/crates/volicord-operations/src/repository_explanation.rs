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
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedKind, ManagedDerivedState,
    PrivacyStore,
};
use volicord_projections::{explanation_text_hash, ExplanationPlan};
use volicord_repository_intelligence::{AnalysisSnapshot, AnalysisSnapshotId, CanonicalGrounding};

const KIND: &str = "volicord_repository_source_observation";
const VERSION: u32 = 1;

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
    pub source_id: SourceId,
    pub start_byte: usize,
    pub end_byte: usize,
    /// None binds the complete authored unit. Explicit subclaims are UTF-8 bytes.
    pub claim: Option<FileByteRange>,
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
            self.observed_at_unix_micros
        ])
    }
}
fn repository_file_binding(observation: &RepositoryObservation) -> Result<String, Error> {
    let file = file_snapshot_binding(&observation.detail, &observation.identity_basis())?;
    Ok(format!(
        "{file};repository-basis:{}",
        explanation_text_hash(&observation.identity_basis().to_string())
    ))
}
fn purpose(analysis: AnalysisSnapshotId, source: SourceId) -> String {
    format!("repository-source:{analysis}:{source}")
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
        let response = ExplanationRealization {
            format_kind: EXPLANATION_KIND.into(),
            format_version: EXPLANATION_VERSION,
            plan_fingerprint: draft.plan_fingerprint,
            language: draft.language,
            generator: draft.generator,
            paragraphs,
        };
        validate_explanation(&plan, &response).map_err(Error::new)?;
        self.validate_subject_sources(
            &self.canonical_basis(project)?,
            ExplanationSubject::Repository(analysis),
            &response,
        )?;
        Ok(response)
    }
    pub(crate) fn repository_explanation_plan(
        &self,
        canonical: &CanonicalReadBasis,
        id: AnalysisSnapshotId,
        language: &str,
    ) -> Result<ExplanationPlan, Error> {
        let analysis = self.load_analysis_snapshot(canonical.project.id, id)?;
        let repository = self.current_explanation_repository(canonical, &analysis)?;
        let mut repository = serde_json::to_value(repository)
            .map_err(|e| Error::with_source("cannot encode repository basis", e))?;
        // Read-time observation time is not historical evidence and must not
        // change the retained basis on every GET.
        if let Some(object) = repository.as_object_mut() {
            object.remove("observed_at_unix_micros");
        }
        let binding = self
            .open_canonical()?
            .get_local_binding(canonical.project.id)
            .map_err(|e| Error::with_source("repository binding unavailable", e))?;
        // Complete inspectable inventory scope, without graph/body duplication.
        let scope: Vec<_> = analysis
            .inventory
            .entries
            .iter()
            .map(|e| json!([e.area.path, e.classifications]))
            .collect();
        volicord_projections::prepare_repository_explanation(canonical, id,
            analysis.repository_source.identity(), json!({"repository":repository,
                "analysis_snapshot_id":id, "analysis_format_version":analysis.format_version,
                "binding_id":binding.id.to_string(), "binding_revision":binding.revision,
                "observed_worktree":analysis.repository_worktree, "inventory_scope":scope,
                "capabilities":analysis.capabilities, "freshness":analysis.freshness,
                "generated_at_unix_micros":analysis.generated_at_unix_micros,
                "semantic_completeness":"not_established", "purpose_role":"inferred_repository_summary_is_not_canonical_project_purpose"}), language).map_err(Error::new)
    }
    pub(crate) fn current_explanation_repository(
        &self,
        canonical: &CanonicalReadBasis,
        analysis: &AnalysisSnapshot,
    ) -> Result<volicord_repository_intelligence::RepositorySnapshot, Error> {
        let binding = self
            .open_canonical()?
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
            ));
        }
        Ok(repository)
    }
    pub fn observe_repository_file(
        &self,
        request: RepositoryFileObservationRequest,
    ) -> Result<Value, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        let now = now_micros()?;
        let until = retention_until(now, request.retain_seconds)?;
        let canonical = self.canonical_basis(request.project_id)?;
        let analysis =
            self.load_analysis_snapshot(request.project_id, request.analysis_snapshot_id)?;
        self.current_explanation_repository(&canonical, &analysis)?;
        let binding = self
            .open_canonical()?
            .get_local_binding(request.project_id)
            .map_err(|e| Error::with_source("repository binding unavailable", e))?;
        let detail =
            self.capture_included_file(&canonical, &analysis, &request.path, request.byte_range)?;
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
            json!({"project_id":request.project_id,"analysis_snapshot_id":analysis.identity,"source_id":source.id,"source_revision":1,
            "retained_until_unix_micros":until.as_unix_micros()}),
        )
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
        let analysis = self.load_analysis_snapshot(project, analysis_id)?;
        let inspection = self.privacy_status(project)?;
        let record = inspection.managed_derived.iter().find(|r| r.kind == ManagedDerivedKind::SourceObservation && r.purpose == purpose(analysis_id,source))
            .ok_or_else(|| Error::new("Source has no explicit association with this Project/repository/Analysis Snapshot"))?;
        let now = now_micros()?;
        if !matches!(
            record.state,
            ManagedDerivedState::Current | ManagedDerivedState::Stale
        ) || record.retained_until.is_none_or(|t| t <= now)
            || inspection
                .withheld_for_canonical_forgetting
                .contains(&record.id)
        {
            return Err(Error::new(
                "repository Source copy deleted, expired, invalidated or withheld",
            ));
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
        let header: Value = serde_json::from_str(content)
            .map_err(|_| Error::new("corrupt repository Source copy"))?;
        if header["format_kind"] != KIND || header["format_version"] != VERSION {
            return Err(Error::new(
                "unsupported repository Source observation; explicitly observe again",
            ));
        }
        let observation: RepositoryObservation = serde_json::from_value(header)
            .map_err(|_| Error::new("corrupt repository Source copy shape"))?;
        let binding = self
            .open_canonical()?
            .get_local_binding(project)
            .map_err(|e| Error::with_source("repository binding unavailable", e))?;
        if observation.project_id != project
            || observation.analysis_snapshot_id != analysis_id
            || observation.source_id != source
            || observation.source_revision != 1
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
            "metadata":{"retained_until_unix_micros":record.retained_until.map(|t|t.as_unix_micros())},"read_only":true}),
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
