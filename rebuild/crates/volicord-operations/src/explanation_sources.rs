//! Exact permitted Source retrieval and publication checks. No model dispatch.
use crate::{operations::parse_identity, Error, LocalOperations};
use serde_json::{json, Value};
use volicord_context::{CanonicalReadBasis, ContextItemId, ProjectId, SourceId};
use volicord_projections::{
    explanation_text_hash, ExplanationRealization, ExplanationSourceUse, ExplanationSubject,
};

pub(crate) struct ValidatedExplanationSources {
    pub dependencies: Vec<SourceId>,
    pub expiry: Option<volicord_context::TimestampMicros>,
    pub excerpts: Vec<volicord_projections::ExplanationSourceExcerpt>,
    pub omitted_excerpt_count: usize,
    pub selected: Option<Value>,
}

impl LocalOperations {
    pub fn explanation_source_manifest(
        &self,
        project: ProjectId,
        work: ContextItemId,
        offset: usize,
        expected_fingerprint: Option<String>,
    ) -> Result<Value, Error> {
        let basis = self.canonical_basis(project)?;
        let revision = crate::source_observations::work_revision(&basis, work)?;
        self.canonical_evidence_detail(
            project,
            &crate::EvidenceDetailRequest {
                record_kind: "context_item".into(),
                record_id: work.to_string(),
                revision,
                field: "source_observations".into(),
                offset,
                expected_fingerprint,
                work_item_id: Some(work),
            },
        )
    }

    /// Preparation advertises the existing exact-field reader; it does not
    /// enumerate observations or acquire any additional body authority.
    pub fn explanation_source_discovery(
        &self,
        plan: &volicord_projections::ExplanationPlan,
    ) -> Value {
        match plan.subject {
            ExplanationSubject::WorkName(_) => {
                json!({"scope":"goal_only","body_authority":"not_granted"})
            }
            ExplanationSubject::Work(work) | ExplanationSubject::WorkSummary(work) => {
                let goal = plan.evidence.iter().find(|e| e.key == "goal");
                json!({"scope":"explicit_recorded_associations_only","body_authority":"not_granted",
                    "discovery":{"tool":"canonical_inspect","project_id":plan.project_id,
                        "record_kind":"context_item","record_id":work.to_string(),
                        "work_item_id":work.to_string(),"revision":goal.map(|e|e.revision),
                        "field":"source_observations","offset":0},
                    "encoding":"compact_json_utf8","chunk_byte_limit":2048,
                    "continuation":"Join chunks in next_offset order, then decode JSON. Send the same exact selector and returned fingerprint as expected_fingerprint on each continuation. A changed list or lifecycle rejects continuation; restart at offset 0. Metadata grants no Source body authority."})
            }
            _ => json!({"scope":"recorded_decision_only","body_authority":"not_granted"}),
        }
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
        let inspection = self.privacy_status(basis.project.id)?;
        self.explanation_source_with_inspection(basis, work, source, &inspection)
    }

    fn explanation_source_with_inspection(
        &self,
        basis: &CanonicalReadBasis,
        work: ContextItemId,
        source: SourceId,
        inspection: &volicord_privacy::ProjectPrivacyInspection,
    ) -> Result<Value, Error> {
        let read = self.source_observation_read_with_inspection(basis, work, source, inspection)?;
        let observation = read.body.ok_or_else(|| {
            Error::new(format!(
                "explanation Source is unavailable: {}",
                read.reason
            ))
            .with_reading_failure(
                read.failure
                    .unwrap_or(volicord_projections::ExplanationFailureReason::Unavailable),
            )
        })?;
        let fingerprint = explanation_text_hash(
            &serde_json::to_string(&observation)
                .map_err(|e| Error::with_source("Source encoding failed", e))?,
        );
        let repository_source: SourceId =
            serde_json::from_value(observation["repository_source_id"].clone())
                .map_err(|_| Error::new("Source repository binding is corrupt"))?;
        // Preserve the existing canonical command identity alongside its disposable
        // streams. The label is presentation; fingerprint + Source identifies the
        // invocation, never a retest or assertion-coverage relationship.
        let command_source = basis.sources.iter().find(|s| s.source.id == source)
            .and_then(|s| match &s.source.payload {
                volicord_context::SourcePayload::CommandExecution {
                    command_label, invocation_fingerprint, outcome, capture_fingerprint,
                } => Some(json!({"command_label":command_label,
                    "invocation_fingerprint":invocation_fingerprint,"capture_fingerprint":capture_fingerprint,
                    "exit_code":outcome.exit_code,"termination":format!("{:?}",outcome.termination).to_lowercase(),
                    "recorded_at_unix_micros":s.source.recorded_at.as_unix_micros(),
                    "actor":format!("{:?}",s.source.actor),"observer":format!("{:?}",s.source.observer)})),
                _ => None,
            });
        let citation_basis = json!({"source_id":source.to_string(),"source_revision":observation["source_revision"],
            "observation_fingerprint":fingerprint,"analysis_snapshot_id":observation["analysis_snapshot_id"],
            "repository_snapshot_id":observation["repository_snapshot_id"],"repository_source_id":repository_source.to_string()});
        Ok(
            json!({"observation":observation,"observation_fingerprint":fingerprint,
            "citation_basis":citation_basis,"command_source":command_source,"metadata":read.metadata,"read_only":true,"semantic_approval":"not_established"}),
        )
    }

    pub(crate) fn validate_subject_sources(
        &self,
        basis: &CanonicalReadBasis,
        subject: ExplanationSubject,
        response: &ExplanationRealization,
    ) -> Result<ValidatedExplanationSources, Error> {
        self.validate_subject_sources_selected(basis, subject, response, None)
    }

    pub(crate) fn validate_subject_sources_selected(
        &self,
        basis: &CanonicalReadBasis,
        subject: ExplanationSubject,
        response: &ExplanationRealization,
        selection: Option<(&volicord_projections::ExplanationCitation, usize)>,
    ) -> Result<ValidatedExplanationSources, Error> {
        self.validate_subject_sources_in_read(basis, subject, response, selection, None)
    }

    pub(crate) fn validate_subject_sources_in_read(
        &self,
        basis: &CanonicalReadBasis,
        subject: ExplanationSubject,
        response: &ExplanationRealization,
        selection: Option<(&volicord_projections::ExplanationCitation, usize)>,
        repository: Option<&crate::repository_explanation::RepositoryExplanationReadBasis>,
    ) -> Result<ValidatedExplanationSources, Error> {
        if let Some(repository) = repository {
            if subject != ExplanationSubject::Repository(repository.analysis.identity)
                || repository.analysis.project.identity() != basis.project.id
                || repository.repository.identity != repository.analysis.repository_snapshot
            {
                return Err(Error::new("foreign repository Source validation basis"));
            }
        }
        let mut selected = None;
        // All entries belong to this validation call. Nothing survives a page,
        // citation request, failure, deletion or forgetting operation. In
        // particular a previous successful read cannot bypass fresh inspection.
        let inspection = response
            .citations()
            .next()
            .map(|_| self.privacy_status(basis.project.id))
            .transpose()?;
        let repository_metadata = match subject {
            ExplanationSubject::Repository(id) if inspection.is_some() && repository.is_none() => {
                Some(self.repository_source_metadata(basis.project.id, id)?)
            }
            _ => None,
        };
        let repository_metadata = repository
            .map(|r| &r.analysis)
            .or(repository_metadata.as_ref());
        let read_source = |source| match subject {
            ExplanationSubject::Work(work)
            | ExplanationSubject::WorkName(work)
            | ExplanationSubject::WorkSummary(work) => self.explanation_source_with_inspection(
                basis,
                work,
                source,
                inspection
                    .as_ref()
                    .ok_or_else(|| Error::new("Source inspection unavailable"))?,
            ),
            ExplanationSubject::Repository(_) => self
                .read_repository_explanation_source_with_basis(
                    basis,
                    repository_metadata
                        .ok_or_else(|| Error::new("repository Source metadata unavailable"))?,
                    inspection
                        .as_ref()
                        .ok_or_else(|| Error::new("Source inspection unavailable"))?,
                    source,
                ),
            ExplanationSubject::Decision(_) => {
                Err(Error::new("Decision cannot read scoped Source bodies"))
            }
        };
        let mut readings = std::collections::BTreeMap::new();
        let mut dependencies = std::collections::BTreeSet::new();
        let mut expiry = None;
        let mut current_snapshots = std::collections::BTreeMap::new();
        if let Some(repository) = repository {
            current_snapshots.insert(
                repository.analysis.identity,
                (
                    repository.repository.identity.to_string(),
                    repository.analysis.repository_source.identity().to_string(),
                ),
            );
        }
        let mut excerpts = Vec::new();
        let mut excerpt_keys = std::collections::BTreeSet::new();
        for citation in response.citations() {
            if let ExplanationSubject::Repository(analysis) = subject {
                if citation.analysis_snapshot_id != analysis.to_string()
                    || citation.use_kind != ExplanationSourceUse::CurrentCode
                {
                    return Err(Error::new("repository explanation requires current file Sources from its exact Analysis Snapshot"));
                }
            }
            let source = SourceId::from_bytes(parse_identity(&citation.source_id)?);
            if let std::collections::btree_map::Entry::Vacant(entry) = readings.entry(source) {
                entry.insert(read_source(source)?);
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
            let file_region = (detail["kind"] == "file"
                && detail["representation"] == "bounded_excerpt")
                .then_some(&detail["region"]);
            let byte_base = file_region
                .and_then(|r| r["start_byte"].as_u64())
                .unwrap_or(0) as usize;
            let line_base = file_region
                .and_then(|r| r["start_line"].as_u64())
                .unwrap_or(1) as usize;
            let start = citation
                .start_byte
                .checked_sub(byte_base)
                .ok_or_else(|| Error::new("citation precedes observed excerpt"))?;
            let end = citation
                .end_byte
                .checked_sub(byte_base)
                .ok_or_else(|| Error::new("citation precedes observed excerpt"))?;
            let range = body
                .get(start..end)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| Error::new("invalid explanation Source range"))?;
            let line = |offset: usize| {
                body.as_bytes()[..offset]
                    .iter()
                    .filter(|b| **b == b'\n')
                    .count()
                    + line_base
            };
            if locator != Some(citation.locator.as_str())
                || explanation_text_hash(range) != citation.range_sha256
                || line(start) != citation.start_line
                || line(end - 1) != citation.end_line
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
                        .open_canonical_read_only()?
                        .get_local_binding(basis.project.id)
                        .map_err(|e| {
                            Error::with_source("explanation repository binding unavailable", e)
                        })?;
                    let root =
                        volicord_local_platform::RepositoryRoot::open(&binding.absolute_path)
                            .map_err(|e| {
                                Error::with_source("explanation repository unavailable", e)
                            .with_reading_failure(
                            volicord_projections::ExplanationFailureReason::ComparisonUnavailable,
                        )
                            })?;
                    if root.canonical_path() != binding.absolute_path {
                        return Err(Error::new("explanation repository root was redirected; explicit binding is required"));
                    }
                    let current_hash = if file_region.is_some() {
                        root.read_utf8_excerpt(
                            std::path::Path::new(&citation.locator),
                            byte_base,
                            byte_base + body.len(),
                        )
                        .map_err(|e| {
                            Error::with_source("explanation current excerpt unavailable", e).with_reading_failure(volicord_projections::ExplanationFailureReason::ComparisonUnavailable)
                        })?
                        .sha256
                    } else {
                        use sha2::Digest;
                        let current = root
                            .read_bounded_regular_file(
                                std::path::Path::new(&citation.locator),
                                16_384,
                            )
                            .map_err(|e| {
                                Error::with_source("explanation current Source unavailable", e).with_reading_failure(volicord_projections::ExplanationFailureReason::ComparisonUnavailable)
                            })?;
                        format!("{:x}", sha2::Sha256::digest(&current))
                    };
                    if detail["sha256"] != current_hash {
                        return Err(Error::new(
                            "explanation Source bytes changed; observe and generate again",
                        )
                        .with_reading_failure(
                            volicord_projections::ExplanationFailureReason::Changed,
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
                        // Repository articles claim inspected Source meaning, not
                        // graph completeness. Their authority is verified metadata,
                        // included hashes and a fresh repository comparison.
                        let (repository_source, repository_snapshot, entries) =
                            if matches!(subject, ExplanationSubject::Repository(_)) {
                                let metadata = repository_metadata.ok_or_else(|| {
                                    Error::new("repository Source metadata unavailable")
                                })?;
                                (
                                    metadata.repository_source.clone(),
                                    metadata.repository_snapshot,
                                    metadata.inventory.entries.clone(),
                                )
                            } else {
                                let analysis =
                                    self.load_analysis_snapshot(basis.project.id, analysis_id)?;
                                (
                                    analysis.repository_source,
                                    analysis.repository_snapshot,
                                    analysis.inventory.entries,
                                )
                            };
                        if repository_source.identity().to_string() != citation.repository_source_id
                            || repository_snapshot.to_string() != citation.repository_snapshot_id
                        {
                            return Err(Error::new(
                                "explanation repository snapshot binding changed",
                            )
                            .with_reading_failure(
                                volicord_projections::ExplanationFailureReason::Changed,
                            ));
                        }
                        let comparison = self.observe_projection_repository(basis,repository_source.identity(),&entries)
                            .map_err(|e|e.with_reading_failure(volicord_projections::ExplanationFailureReason::ComparisonUnavailable))?;
                        if comparison.identity != repository_snapshot {
                            return Err(Error::new("explanation repository changed; analyze, observe and generate again")
                                .with_reading_failure(volicord_projections::ExplanationFailureReason::Changed));
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
                        let earlier = read_source(before)?;
                        let old = &earlier["observation"];
                        let comparison = if old["detail"]["sha256"] == detail["sha256"] {
                            "no_change"
                        } else {
                            "changed"
                        };
                        if old["detail"]["path"] != detail["path"]
                            || !crate::source_observations::compatible_file_regions(
                                &old["detail"],
                                detail,
                            )
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
            if !observation["integrity_source_id"].is_null() {
                let integrity: SourceId =
                    serde_json::from_value(observation["integrity_source_id"].clone()).map_err(
                        |_| Error::new("command observation integrity Source binding unavailable"),
                    )?;
                dependencies.insert(integrity);
            }
            dependencies.insert(source);
            dependencies.insert(SourceId::from_bytes(parse_identity(
                &citation.repository_source_id,
            )?));
            bound_expiry(&mut expiry, read)?;
            if let Some((target, offset)) = selection.filter(|(target, _)| *target == citation) {
                let suffix = range
                    .get(offset..)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| Error::new("invalid selected citation UTF-8 offset"))?;
                let mut size = suffix
                    .len()
                    .min(volicord_projections::EXPLANATION_SOURCE_RANGE_BYTE_LIMIT);
                while !suffix.is_char_boundary(size) {
                    size -= 1;
                }
                selected = Some(json!({
                    "citation":target, "text":&suffix[..size],
                    "displayed_start_byte":citation.start_byte + offset,
                    "displayed_end_byte":citation.start_byte + offset + size,
                    "displayed_start_line":line(start + offset),
                    "displayed_end_line":line(start + offset + size - 1),
                    "omitted_before_utf8_bytes":offset,
                    "omitted_after_utf8_bytes":range.len() - offset - size,
                    "next_offset":(offset + size < range.len()).then_some(offset + size),
                    "source_representation":detail["representation"],
                    "observation_region":file_region,
                    "comparison":detail["comparison"],
                    "before_source_id":detail["before_source_id"],
                    "observed_at_unix_micros":observation["observed_at_unix_micros"],
                    "work_revision":observation["work_revision"],
                    "execution":(detail["kind"] == "command").then(|| json!({
                        "exit_code":detail["exit_code"],"termination":detail["termination"],
                        "stream":citation.field,"representation":detail[&citation.field]["representation"],
                        "observed_bytes":detail[&citation.field]["observed_bytes"],
                        "omitted_bytes":detail[&citation.field]["omitted_bytes"]}))
                }));
            }
            // Materialize only exact cited bytes already admitted by this reader.
            // Repeated claims may cite the same range; retain one navigation target.
            let key = (
                citation.source_id.clone(),
                citation.field.clone(),
                citation.start_byte,
                citation.end_byte,
                format!("{:?}", citation.use_kind),
            );
            if selection.is_none() && excerpt_keys.insert(key) {
                let mut bytes = range
                    .len()
                    .min(volicord_projections::EXPLANATION_SOURCE_WINDOW_BYTE_LIMIT);
                while !range.is_char_boundary(bytes) {
                    bytes -= 1;
                }
                if bytes > 0 {
                    let end = citation.start_byte + bytes;
                    excerpts.push(volicord_projections::ExplanationSourceExcerpt {
                        citation: citation.clone(),
                        work_revision: match subject {
                            ExplanationSubject::Work(_) | ExplanationSubject::WorkName(_) | ExplanationSubject::WorkSummary(_) => Some(observation["work_revision"].as_u64().ok_or_else(||Error::new("Source Work revision unavailable"))?),
                            _ => None,
                        },
                        observed_at_unix_micros: observation["observed_at_unix_micros"].as_i64()
                            .ok_or_else(|| Error::new("Source observation time unavailable"))?,
                        text: range[..bytes].into(), displayed_end_byte: end,
                        displayed_end_line: line(end - byte_base - 1), omitted_utf8_bytes: range.len() - bytes,
                        comparison: detail["comparison"].as_str().map(str::to_owned),
                        before_source_id: detail["before_source_id"].as_str().map(str::to_owned),
                        source_representation: detail["representation"].as_str().map(str::to_owned),
                        observation_region: file_region.cloned(),
                        execution: (detail["kind"] == "command").then(|| json!({
                            "exit_code":detail["exit_code"], "termination":detail["termination"],
                            "stream":citation.field, "representation":detail[&citation.field]["representation"],
                            "observed_bytes":detail[&citation.field]["observed_bytes"],
                            "omitted_bytes":detail[&citation.field]["omitted_bytes"]})),
                    });
                }
            }
        }
        for before in response
            .citations()
            .filter(|c| c.use_kind == ExplanationSourceUse::BeforeCode)
        {
            let paired = response
                .citations()
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
        let omitted_excerpt_count = bound_excerpts(&mut excerpts, response);
        Ok(ValidatedExplanationSources {
            dependencies: dependencies.into_iter().collect(),
            expiry,
            excerpts,
            omitted_excerpt_count,
            selected,
        })
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

/// Select complete claim windows, independently of paragraph role. A bundle
/// contains every cited range for one unit, in authored reading order. Small
/// supporting windows cannot displace an earlier complete mechanism. Then include
/// any complete supporting ranges that still fit.
/// No equal-prefix clipping can hide a function's material return/cleanup path.
/// Selection is presentation scope, never an assessment of claim importance.
fn bound_excerpts(
    excerpts: &mut Vec<volicord_projections::ExplanationSourceExcerpt>,
    response: &ExplanationRealization,
) -> usize {
    let key = |c: &volicord_projections::ExplanationCitation| {
        (
            c.source_id.clone(),
            c.field.clone(),
            c.start_byte,
            c.end_byte,
            format!("{:?}", c.use_kind),
        )
    };
    let sizes: std::collections::BTreeMap<_, _> = excerpts
        .iter()
        .filter(|e| e.omitted_utf8_bytes == 0)
        .map(|e| (key(&e.citation), e.text.len()))
        .collect();
    let bundles: Vec<_> = response
        .generated_units()
        .filter(|u| !u.citations.is_empty())
        .filter_map(|u| {
            let keys: std::collections::BTreeSet<_> = u.citations.iter().map(key).collect();
            let size = keys
                .iter()
                .try_fold(0usize, |sum, k| sizes.get(k).map(|size| sum + size))?;
            Some((size, keys))
        })
        .collect();
    let mut selected = std::collections::BTreeSet::new();
    // Preserve complete bundles admitted by the established 32 KiB selection.
    // Additional capacity fills complete supporting windows afterwards: admitting
    // a newly fitting early inventory bundle must not displace a later mechanism
    // which was already complete. This is allocation stability, not importance.
    const BUNDLE_CAPACITY: usize = 32_768;
    let capacity = volicord_projections::EXPLANATION_SOURCE_EXCERPT_BYTE_LIMIT;
    let mut remaining = capacity.min(BUNDLE_CAPACITY);
    for (_, keys) in bundles {
        let added: usize = keys
            .iter()
            .filter(|k| !selected.contains(*k))
            .map(|k| sizes[k])
            .sum();
        if added <= remaining {
            remaining -= added;
            selected.extend(keys);
        }
    }
    remaining += capacity.saturating_sub(BUNDLE_CAPACITY);
    let mut supporting: Vec<_> = sizes.iter().collect();
    supporting.sort_by_key(|(k, size)| (**size, (*k).clone()));
    for (k, size) in supporting {
        if !selected.contains(k) && *size <= remaining {
            selected.insert(k.clone());
            remaining -= size;
        }
    }
    let before = excerpts.len();
    excerpts.retain(|e| selected.contains(&key(&e.citation)));
    before - excerpts.len()
}
