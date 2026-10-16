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
}

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

    pub(crate) fn validate_subject_sources(
        &self,
        basis: &CanonicalReadBasis,
        subject: ExplanationSubject,
        response: &ExplanationRealization,
    ) -> Result<ValidatedExplanationSources, Error> {
        let read_source = |source| match subject {
            ExplanationSubject::Work(work) => self.explanation_source(basis, work, source),
            ExplanationSubject::Repository(analysis) => {
                self.read_repository_explanation_source(basis.project.id, analysis, source)
            }
            ExplanationSubject::Decision(_) => {
                Err(Error::new("Decision cannot read scoped Source bodies"))
            }
        };
        let mut readings = std::collections::BTreeMap::new();
        let mut dependencies = std::collections::BTreeSet::new();
        let mut expiry = None;
        let mut current_snapshots = std::collections::BTreeMap::new();
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
                    let current_hash = if file_region.is_some() {
                        root.read_utf8_excerpt(
                            std::path::Path::new(&citation.locator),
                            byte_base,
                            byte_base + body.len(),
                        )
                        .map_err(|e| {
                            Error::with_source("explanation current excerpt unavailable", e)
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
                                Error::with_source("explanation current Source unavailable", e)
                            })?;
                        format!("{:x}", sha2::Sha256::digest(&current))
                    };
                    if detail["sha256"] != current_hash {
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
            dependencies.insert(source);
            dependencies.insert(SourceId::from_bytes(parse_identity(
                &citation.repository_source_id,
            )?));
            bound_expiry(&mut expiry, read)?;
            // Materialize only exact cited bytes already admitted by this reader.
            // Repeated claims may cite the same range; retain one navigation target.
            let key = (
                citation.source_id.clone(),
                citation.field.clone(),
                citation.start_byte,
                citation.end_byte,
                format!("{:?}", citation.use_kind),
            );
            if excerpt_keys.insert(key) {
                let mut bytes = range
                    .len()
                    .min(volicord_projections::EXPLANATION_SOURCE_RANGE_BYTE_LIMIT);
                while !range.is_char_boundary(bytes) {
                    bytes -= 1;
                }
                if bytes > 0 {
                    let end = citation.start_byte + bytes;
                    excerpts.push(volicord_projections::ExplanationSourceExcerpt {
                        citation: citation.clone(),
                        work_revision: match subject {
                            ExplanationSubject::Work(_) => Some(observation["work_revision"].as_u64().ok_or_else(||Error::new("Source Work revision unavailable"))?),
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
        let omitted_excerpt_count = bound_excerpts(&mut excerpts);
        Ok(ValidatedExplanationSources {
            dependencies: dependencies.into_iter().collect(),
            expiry,
            excerpts,
            omitted_excerpt_count,
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

/// Max-min character allocation over unique verified ranges. Stable identity order
/// makes reordered citations and repeated claims unable to consume another range's
/// share. Short ranges release unused capacity to the rest; no byte limit changes.
fn bound_excerpts(excerpts: &mut Vec<volicord_projections::ExplanationSourceExcerpt>) -> usize {
    let mut order: Vec<_> = (0..excerpts.len()).collect();
    order.sort_by_key(|&i| {
        let c = &excerpts[i].citation;
        (
            c.source_id.clone(),
            c.field.clone(),
            c.start_byte,
            c.end_byte,
            format!("{:?}", c.use_kind),
        )
    });
    let mut ends = vec![0; excerpts.len()];
    let mut remaining = volicord_projections::EXPLANATION_SOURCE_EXCERPT_BYTE_LIMIT;
    loop {
        let mut advanced = false;
        for &i in &order {
            if let Some(ch) = excerpts[i].text[ends[i]..].chars().next() {
                if ch.len_utf8() <= remaining {
                    ends[i] += ch.len_utf8();
                    remaining -= ch.len_utf8();
                    advanced = true;
                }
            }
        }
        if !advanced {
            break;
        }
    }
    for (e, end) in excerpts.iter_mut().zip(ends) {
        e.omitted_utf8_bytes += e.text.len() - end;
        e.text.truncate(end);
        e.displayed_end_byte = e.citation.start_byte + end;
        e.displayed_end_line = e.citation.start_line
            + e.text
                .as_bytes()
                .get(..end.saturating_sub(1))
                .unwrap_or_default()
                .iter()
                .filter(|b| **b == b'\n')
                .count();
    }
    let omitted = excerpts.iter().filter(|e| e.text.is_empty()).count();
    excerpts.retain(|e| !e.text.is_empty());
    omitted
}
