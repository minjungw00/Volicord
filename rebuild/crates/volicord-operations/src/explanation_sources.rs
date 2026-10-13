//! Exact permitted Source retrieval and publication checks. No model dispatch.
use crate::{operations::parse_identity, Error, LocalOperations};
use serde_json::{json, Value};
use volicord_context::{CanonicalReadBasis, ContextItemId, ProjectId, SourceId};
use volicord_projections::{explanation_text_hash, ExplanationRealization, ExplanationSourceUse};

impl LocalOperations {
    pub fn explanation_source_manifest(
        &self,
        project: ProjectId,
        work: ContextItemId,
    ) -> Result<Value, Error> {
        self.work_source_manifest(&self.canonical_basis(project)?, work)
    }

    /// Explicit read of one already retained and permitted Source; no capture,
    /// filesystem fallback, retention renewal, generation or canonical mutation.
    pub fn read_explanation_source(
        &self,
        project: ProjectId,
        work: ContextItemId,
        source: SourceId,
    ) -> Result<Value, Error> {
        self.explanation_source(&self.canonical_basis(project)?, work, source)
    }

    fn explanation_source(
        &self,
        basis: &CanonicalReadBasis,
        work: ContextItemId,
        source: SourceId,
    ) -> Result<Value, Error> {
        let read = self.source_observation_read(basis, work, source)?;
        let observation = read.body.ok_or_else(|| {
            Error::new(format!(
                "explanation Source is unavailable: {}",
                read.reason
            ))
        })?;
        let fingerprint = explanation_text_hash(
            &serde_json::to_string(&observation)
                .map_err(|e| Error::with_source("Source encoding failed", e))?,
        );
        let repository_source: SourceId =
            serde_json::from_value(observation["repository_source_id"].clone())
                .map_err(|_| Error::new("Source repository binding is corrupt"))?;
        let citation_basis = json!({"source_id":source.to_string(),"source_revision":observation["source_revision"],
            "observation_fingerprint":fingerprint,"analysis_snapshot_id":observation["analysis_snapshot_id"],
            "repository_snapshot_id":observation["repository_snapshot_id"],"repository_source_id":repository_source.to_string()});
        Ok(
            json!({"observation":observation,"observation_fingerprint":fingerprint,
            "citation_basis":citation_basis,"metadata":read.metadata,"read_only":true,"semantic_approval":"not_established"}),
        )
    }

    pub(crate) fn validate_explanation_sources(
        &self,
        basis: &CanonicalReadBasis,
        work: ContextItemId,
        response: &ExplanationRealization,
    ) -> Result<(Vec<SourceId>, Option<volicord_context::TimestampMicros>), Error> {
        let mut readings = std::collections::BTreeMap::new();
        let mut dependencies = std::collections::BTreeSet::new();
        let mut expiry = None;
        let mut current_snapshots = std::collections::BTreeMap::new();
        for citation in response.paragraphs.iter().flat_map(|p| &p.citations) {
            let source = SourceId::from_bytes(parse_identity(&citation.source_id)?);
            if let std::collections::btree_map::Entry::Vacant(entry) = readings.entry(source) {
                entry.insert(self.explanation_source(basis, work, source)?);
            }
            let read = &readings[&source];
            let observation = &read["observation"];
            let detail = &observation["detail"];
            if read["observation_fingerprint"] != citation.observation_fingerprint
                || observation["source_revision"] != citation.source_revision
                || observation["analysis_snapshot_id"] != citation.analysis_snapshot_id
                || observation["repository_snapshot_id"] != citation.repository_snapshot_id
                || read["citation_basis"]["repository_source_id"] != citation.repository_source_id
            {
                return Err(Error::new(
                    "explanation Source identity, revision or retrieval binding changed",
                ));
            }
            let (body, locator) = match citation.field.as_str() {
                "body" if detail["kind"] == "file" => {
                    (detail["body"].as_str(), detail["path"].as_str())
                }
                "stdout" | "stderr" if detail["kind"] == "command" => {
                    let canonical = basis
                        .sources
                        .iter()
                        .find(|s| s.source.id == source)
                        .ok_or_else(|| Error::new("command Source unavailable"))?;
                    let volicord_context::SourcePayload::CommandExecution { command_label, .. } =
                        &canonical.source.payload
                    else {
                        return Err(Error::new("command Source kind mismatch"));
                    };
                    (
                        detail[&citation.field]["body"].as_str(),
                        Some(command_label.as_str()),
                    )
                }
                _ => return Err(Error::new("explanation Source field is not permitted")),
            };
            let body = body.ok_or_else(|| Error::new("explanation Source bytes unavailable"))?;
            let range = body
                .get(citation.start_byte..citation.end_byte)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| Error::new("invalid explanation Source range"))?;
            let line = |offset: usize| {
                body.as_bytes()[..offset]
                    .iter()
                    .copied()
                    .filter(|b| *b == b'\n')
                    .count()
                    + 1
            };
            if locator != Some(citation.locator.as_str())
                || explanation_text_hash(range) != citation.range_sha256
                || line(citation.start_byte) != citation.start_line
                || line(citation.end_byte - 1) != citation.end_line
            {
                return Err(Error::new(
                    "explanation Source locator, range or hash mismatch",
                ));
            }
            match citation.use_kind {
                ExplanationSourceUse::CurrentCode => {
                    if detail["kind"] != "file" {
                        return Err(Error::new("current code requires a file observation"));
                    }
                    let binding = self
                        .open_canonical()?
                        .get_local_binding(basis.project.id)
                        .map_err(|e| {
                            Error::with_source("explanation repository binding unavailable", e)
                        })?;
                    let root =
                        volicord_local_platform::RepositoryRoot::open(&binding.absolute_path)
                            .map_err(|e| {
                                Error::with_source("explanation repository unavailable", e)
                            })?;
                    if root.canonical_path() != binding.absolute_path {
                        return Err(Error::new("explanation repository root was redirected; explicit binding is required"));
                    }
                    let current = root
                        .read_bounded_regular_file(std::path::Path::new(&citation.locator), 16_384)
                        .map_err(|e| {
                            Error::with_source(
                                "explanation current Source read denied or unavailable",
                                e,
                            )
                        })?;
                    if current != body.as_bytes() {
                        return Err(Error::new(
                            "explanation Source bytes changed; observe and generate again",
                        ));
                    }
                    let analysis_id =
                        crate::AnalysisSnapshotId::from_hex(&citation.analysis_snapshot_id)
                            .map_err(Error::new)?;
                    if let Some((repository, repository_source)) =
                        current_snapshots.get(&analysis_id)
                    {
                        if repository != &citation.repository_snapshot_id
                            || repository_source != &citation.repository_source_id
                        {
                            return Err(Error::new(
                                "explanation Analysis/Repository/Source binding mismatch",
                            ));
                        }
                    } else {
                        let analysis =
                            self.load_analysis_snapshot(basis.project.id, analysis_id)?;
                        if analysis.repository_source.identity().to_string()
                            != citation.repository_source_id
                            || analysis.repository_snapshot.to_string()
                                != citation.repository_snapshot_id
                            || self
                                .observe_projection_repository(
                                    basis,
                                    analysis.repository_source.identity(),
                                    &analysis.inventory.entries,
                                )?
                                .identity
                                != analysis.repository_snapshot
                        {
                            return Err(Error::new("explanation repository changed or is unavailable; analyze, observe and generate again"));
                        }
                        current_snapshots.insert(
                            analysis_id,
                            (
                                citation.repository_snapshot_id.clone(),
                                citation.repository_source_id.clone(),
                            ),
                        );
                    }
                    // A corrupt comparison cannot supply even an otherwise valid current range.
                    if let Some(before) = detail["before_source_id"].as_str() {
                        let before = SourceId::from_bytes(parse_identity(before)?);
                        let earlier = self.explanation_source(basis, work, before)?;
                        let old = &earlier["observation"];
                        let comparison = if old["detail"]["sha256"] == detail["sha256"] {
                            "no_change"
                        } else {
                            "changed"
                        };
                        if old["detail"]["path"] != detail["path"]
                            || old["observed_at_unix_micros"].as_i64()
                                >= observation["observed_at_unix_micros"].as_i64()
                            || detail["comparison"] != comparison
                        {
                            return Err(Error::new("invalid before/after explanation evidence"));
                        }
                        dependencies.insert(before);
                        dependencies.insert(SourceId::from_bytes(parse_identity(
                            earlier["citation_basis"]["repository_source_id"]
                                .as_str()
                                .ok_or_else(|| {
                                    Error::new("before-state repository Source unavailable")
                                })?,
                        )?));
                        bound_expiry(&mut expiry, &earlier)?;
                    } else if detail["comparison"] != "missing_before_state" {
                        return Err(Error::new("missing explicit before-state"));
                    }
                }
                ExplanationSourceUse::BeforeCode => {
                    if detail["kind"] != "file" {
                        return Err(Error::new("before code requires a file observation"));
                    }
                }
                ExplanationSourceUse::ObservedExecution => {
                    if detail["kind"] != "command" {
                        return Err(Error::new(
                            "execution evidence requires an observed command Source",
                        ));
                    }
                }
            }
            dependencies.insert(source);
            dependencies.insert(SourceId::from_bytes(parse_identity(
                &citation.repository_source_id,
            )?));
            bound_expiry(&mut expiry, read)?;
        }
        for before in response
            .paragraphs
            .iter()
            .flat_map(|p| &p.citations)
            .filter(|c| c.use_kind == ExplanationSourceUse::BeforeCode)
        {
            let paired = response
                .paragraphs
                .iter()
                .flat_map(|p| &p.citations)
                .filter(|c| c.use_kind == ExplanationSourceUse::CurrentCode)
                .any(|after| {
                    parse_identity(&after.source_id)
                        .ok()
                        .and_then(|id| readings.get(&SourceId::from_bytes(id)))
                        .is_some_and(|r| {
                            r["observation"]["detail"]["before_source_id"] == before.source_id
                        })
                });
            if !paired {
                return Err(Error::new(
                    "before-state citation requires its explicit observed after-state",
                ));
            }
        }
        Ok((dependencies.into_iter().collect(), expiry))
    }
}

fn bound_expiry(
    expiry: &mut Option<volicord_context::TimestampMicros>,
    read: &Value,
) -> Result<(), Error> {
    let until = read["metadata"]["retained_until_unix_micros"]
        .as_i64()
        .ok_or_else(|| Error::new("Source expiry unavailable"))?;
    let until = volicord_context::TimestampMicros::from_unix_micros(until);
    *expiry = Some(expiry.map_or(until, |old| old.min(until)));
    Ok(())
}
