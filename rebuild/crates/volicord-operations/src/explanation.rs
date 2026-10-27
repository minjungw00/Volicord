//! Explicit interactive-host generation lifecycle, owned Derived storage, and
//! read-time freshness. No read can generate, dispatch, or transmit anything.
use crate::operations::{now_micros, parse_identity};
use crate::{Error, LocalOperations};
use volicord_context::{
    CanonicalReadBasis, CheckpointId, ContextItemId, DecisionId, ProjectId, QuestionId, SourceId,
};
use volicord_privacy::{
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedKind, ManagedDerivedState,
    PrivacyStore,
};
use volicord_projections::*;

/// Exact realization/unit/citation selection; offset is relative to that cited
/// range, never an arbitrary file interval. Each read returns at most 4096 bytes.
#[derive(Clone, Debug)]
pub struct ExplanationCitationSelection {
    pub publication_id: String,
    pub realization_fingerprint: String,
    pub unit_index: usize,
    pub citation_index: usize,
    pub offset: usize,
}

fn purpose(subject: ExplanationSubject, language: &str) -> String {
    format!("explanation:{}:{language}", subject.key())
}
impl LocalOperations {
    /// Bounded lossless reading of one exact admitted publication. Every page
    /// revalidates all dependencies, including dependencies outside that page.
    /// Offsets are UTF-8 bytes of compact JSON, not paragraph or file offsets.
    pub fn read_explanation_page(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        offset: usize,
        expected_fingerprint: Option<&str>,
    ) -> Result<serde_json::Value, Error> {
        const CHUNK: usize = 8_192;
        const CONTENT: usize = 1_048_576;
        let _inspection = self.layout().acquire_health_lock()?;
        let (canonical, retained, publication_id) =
            self.permitted_read_publication(project, subject, language)?;
        let sources =
            self.read_retained_sources(&canonical, subject, &retained.realization, None)?;
        let applicability =
            self.retained_applicability(&canonical, &retained, sources.availability);
        let document = compact_publication(&retained, &publication_id);
        let encoded = serde_json::to_string(&document)
            .map_err(|e| Error::with_source("publication encoding failed", e))?;
        if encoded.len() > CONTENT {
            return Err(Error::new(format!("unsupported publication reading size: {} bytes exceeds {CONTENT}; no content returned", encoded.len())));
        }
        let fingerprint = explanation_text_hash(&encoded);
        if (offset > 0 && expected_fingerprint.is_none())
            || expected_fingerprint.is_some_and(|expected| expected != fingerprint)
        {
            return Err(Error::new("publication continuation changed or belongs to another Project, subject, language or realization"));
        }
        if offset >= encoded.len() || !encoded.is_char_boundary(offset) {
            return Err(Error::new("invalid publication UTF-8 offset"));
        }
        let mut end = (offset + CHUNK).min(encoded.len());
        while !encoded.is_char_boundary(end) {
            end -= 1;
        }
        Ok(
            serde_json::json!({"project_id":project.to_string(),"subject":subject,"language":language,
            "publication_id":publication_id,"realization_fingerprint":document["provenance"]["realization_fingerprint"],
            "fingerprint":fingerprint,"encoding":"compact_json_utf8","offset":offset,
            "chunk":&encoded[offset..end],"next_offset":(end < encoded.len()).then_some(end),
            "state":if end == encoded.len() {"complete"} else {"partial"},
            "total_utf8_bytes":encoded.len(),"remaining_utf8_bytes":encoded.len()-end,
            "chunk_byte_limit":CHUNK,"supported_content_byte_limit":CONTENT,"response_byte_limit":65_536,
            "applicability":applicability,"read_only":true}),
        )
    }

    /// Shared publication authority for bounded document and exact citation reads.
    /// The caller holds the inspection lock; Source validation follows selection.
    fn permitted_read_publication(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
    ) -> Result<(CanonicalReadBasis, RetainedExplanation, String), Error> {
        let canonical = self.canonical_basis(project)?;
        let records = self.privacy_status(project)?;
        let (retained, publication) =
            self.permitted_publication_from(&canonical, &records, subject, language)?;
        Ok((canonical, retained, publication))
    }

    pub(crate) fn permitted_publication_from(
        &self,
        canonical: &CanonicalReadBasis,
        records: &volicord_privacy::ProjectPrivacyInspection,
        subject: ExplanationSubject,
        language: &str,
    ) -> Result<(RetainedExplanation, String), Error> {
        let project = canonical.project.id;
        let target = purpose(subject, language);
        let record = records
            .managed_derived
            .iter()
            .rfind(|r| r.kind == ManagedDerivedKind::CachedSummary && r.purpose == target)
            .ok_or_else(|| {
                Error::new("publication unavailable: missing")
                    .with_reading_failure(ExplanationFailureReason::Missing)
            })?;
        let failure = if records
            .withheld_for_canonical_forgetting
            .contains(&record.id)
        {
            Some(ExplanationFailureReason::Withheld)
        } else if record.state == ManagedDerivedState::Deleted {
            Some(ExplanationFailureReason::Deleted)
        } else if !record.source_computed_explanation_deadline(&records.managed_derived)
            && record
                .retained_until
                .is_some_and(|t| now_micros().is_ok_and(|now| t <= now))
        {
            Some(ExplanationFailureReason::Expired)
        } else if !matches!(
            record.state,
            ManagedDerivedState::Current | ManagedDerivedState::Stale
        ) {
            Some(ExplanationFailureReason::Changed)
        } else {
            None
        };
        if let Some(failure) = failure {
            return Err(
                Error::new("publication is not current or permitted").with_reading_failure(failure)
            );
        }
        let mut retained =
            decode_explanation(record.content.as_deref()).map_err(|(state, reason)| {
                Error::new(reason).with_reading_failure(
                    ExplanationApplicability::unavailable(state)
                        .limitation
                        .unwrap_or(ExplanationFailureReason::Unavailable),
                )
            })?;
        retained.publication_id = Some(record.id.to_string());
        if retained.project_id != project.to_string()
            || retained.subject != subject
            || retained.realization.language != language
        {
            return Err(Error::new("publication identity is corrupt")
                .with_reading_failure(ExplanationFailureReason::Corrupt));
        }
        let corrupt = || {
            Error::new("publication admission metadata is corrupt")
                .with_reading_failure(ExplanationFailureReason::Corrupt)
        };
        let mut required_sources: std::collections::BTreeSet<_> = retained
            .evidence
            .iter()
            .flat_map(|e| e.sources.iter().cloned())
            .collect();
        for citation in retained.realization.citations() {
            required_sources.insert(citation.source_id.clone());
            required_sources.insert(citation.repository_source_id.clone());
        }
        if record.included_sources.iter().any(|source| {
            source.project() != project
                || !record
                    .canonical_links
                    .contains(&ManagedCanonicalLink::Source(source.identity()))
        }) || required_sources.iter().any(|id| {
            !record
                .included_sources
                .iter()
                .any(|source| source.identity().to_string() == *id)
        }) {
            return Err(corrupt());
        }
        for evidence in &retained.evidence {
            if evidence.record_kind == "analysis_snapshot" {
                continue;
            }
            let id = parse_identity(&evidence.identity).map_err(|_| corrupt())?;
            let link = match evidence.record_kind.as_str() {
                "context_item" => ManagedCanonicalLink::ContextItem(ContextItemId::from_bytes(id)),
                "checkpoint" => ManagedCanonicalLink::Checkpoint(CheckpointId::from_bytes(id)),
                "decision" => ManagedCanonicalLink::Decision(DecisionId::from_bytes(id)),
                "question" => ManagedCanonicalLink::Question(QuestionId::from_bytes(id)),
                _ => return Err(corrupt()),
            };
            if !record.canonical_links.contains(&link) {
                return Err(corrupt());
            }
        }
        if matches!(subject, ExplanationSubject::Repository(id) if record.analysis_snapshot != Some(id))
        {
            return Err(corrupt());
        }
        // Explicit body deletion/access withdrawal propagates to the explanation;
        // ordinary expiry removes only evidence bytes. No older-record fallback.
        for source in &record.included_sources {
            let current = canonical
                .sources
                .iter()
                .find(|s| s.source.id == source.identity());
            let admitted_unavailable = current.is_some_and(|s| {
                retained.source_status.iter().any(|status| {
                    status["identity"] == source.identity().to_string()
                        && status["availability"] == format!("{:?}", s.availability)
                })
            });
            // A canonical report may have been admitted with an explicitly
            // unavailable historical witness. Preserve that recorded limitation;
            // loss of a previously available witness remains a blocking change.
            if current.is_none()
                || current.is_some_and(|s| {
                    s.availability != volicord_context::Availability::Available
                        && !admitted_unavailable
                })
            {
                return Err(Error::new("publication Source authority withdrawn")
                    .with_reading_failure(ExplanationFailureReason::Withheld));
            }
        }
        for dependency in &records.managed_derived {
            if dependency.kind == ManagedDerivedKind::SourceObservation
                && dependency.purpose.rsplit(':').next().is_some_and(|id| {
                    record
                        .included_sources
                        .iter()
                        .any(|s| s.identity().to_string() == id)
                })
                && ((dependency.state == ManagedDerivedState::Deleted
                    && !dependency.expired_by_cleanup())
                    || dependency.state == ManagedDerivedState::Invalidated
                    || records
                        .withheld_for_canonical_forgetting
                        .contains(&dependency.id))
            {
                return Err(
                    Error::new("publication evidence explicitly removed or withheld")
                        .with_reading_failure(
                            if dependency.state == ManagedDerivedState::Deleted {
                                ExplanationFailureReason::Deleted
                            } else {
                                ExplanationFailureReason::Withheld
                            },
                        ),
                );
            }
        }
        let plan = self.retained_plan(canonical, &retained)?;
        if !retained_matches(&plan, &retained) {
            return Err(
                Error::new("publication basis changed; reopen the current explanation")
                    .with_reading_failure(ExplanationFailureReason::Changed),
            );
        }
        Ok((retained, record.id.to_string()))
    }

    pub(crate) fn validate_read_publication(
        &self,
        project: ProjectId,
        fingerprint: &str,
        explanations: &[ExplanationProvenance],
    ) -> Result<(), Error> {
        let canonical = self.canonical_basis(project)?;
        if canonical_read_fingerprint(&canonical) != fingerprint {
            return Err(Error::new(
                "read basis changed; render again before publication",
            ));
        }
        if explanations.is_empty() {
            return Ok(());
        }
        for provenance in explanations {
            let (_, retained, _) =
                self.permitted_read_publication(project, provenance.subject, &provenance.language)?;
            let mut expected = ExplanationProvenance::from(&retained);
            if retained.realization.citations().next().is_some()
                && provenance.source_read_fingerprint.is_none()
            {
                return Err(Error::new(
                    "missing evidence read basis; render again before publication",
                ));
            }
            if provenance.source_read_fingerprint.is_some() {
                let sources = self.read_retained_sources(
                    &canonical,
                    provenance.subject,
                    &retained.realization,
                    None,
                )?;
                expected.source_read_fingerprint =
                    Some(source_availability_fingerprint(&sources.availability));
            }
            if expected != *provenance {
                return Err(Error::new(
                    "explanation changed; render again before publication",
                ));
            }
        }
        Ok(())
    }

    pub fn prepare_explanation(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
    ) -> Result<ExplanationPlan, Error> {
        let canonical = self.canonical_basis(project)?;
        self.prepare_subject_explanation(&canonical, subject, language)
    }
    pub(crate) fn prepare_subject_explanation(
        &self,
        canonical: &CanonicalReadBasis,
        subject: ExplanationSubject,
        language: &str,
    ) -> Result<ExplanationPlan, Error> {
        match subject {
            ExplanationSubject::Repository(id) => {
                self.repository_explanation_plan(canonical, id, language)
            }
            _ => prepare_explanation(canonical, subject, language).map_err(Error::new),
        }
    }
    pub fn record_explanation(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        realization: ExplanationRealization,
    ) -> Result<RetainedExplanation, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        let canonical = self.canonical_basis(project)?;
        let plan = self.prepare_subject_explanation(&canonical, subject, language)?;
        validate_explanation(&plan, &realization).map_err(Error::new)?;
        let source_dependencies = match subject {
            ExplanationSubject::Work(_)
            | ExplanationSubject::WorkName(_)
            | ExplanationSubject::WorkSummary(_)
            | ExplanationSubject::Repository(_) => {
                let validated = self.validate_subject_sources(&canonical, subject, &realization)?;
                validated.dependencies
            }
            ExplanationSubject::Decision(_) => Vec::new(),
        };
        let grounding =
            volicord_repository_intelligence::CanonicalGrounding::from_read_basis(&canonical)
                .map_err(|e| Error::with_source("cannot ground explanation", e))?;
        let mut sources = plan
            .evidence
            .iter()
            .flat_map(|e| e.sources.iter())
            .collect::<std::collections::BTreeSet<_>>();
        let source_dependency_text: Vec<_> = source_dependencies
            .iter()
            .map(ToString::to_string)
            .collect();
        sources.extend(source_dependency_text.iter());
        let included_sources = sources
            .iter()
            .map(|id| {
                grounding
                    .source_reference(SourceId::from_bytes(parse_identity(id)?))
                    .map_err(|e| Error::with_source("cannot ground explanation Source", e))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let mut canonical_links = Vec::new();
        for e in &plan.evidence {
            if e.record_kind == "analysis_snapshot" {
                continue;
            }
            let bytes = parse_identity(&e.identity)?;
            canonical_links.push(match e.record_kind.as_str() {
                "context_item" => {
                    ManagedCanonicalLink::ContextItem(ContextItemId::from_bytes(bytes))
                }
                "checkpoint" => ManagedCanonicalLink::Checkpoint(CheckpointId::from_bytes(bytes)),
                "decision" => ManagedCanonicalLink::Decision(DecisionId::from_bytes(bytes)),
                "question" => ManagedCanonicalLink::Question(QuestionId::from_bytes(bytes)),
                _ => return Err(Error::new("unsupported explanation evidence kind")),
            });
        }
        canonical_links.extend(
            sources
                .iter()
                .map(|id| {
                    parse_identity(id)
                        .map(|bytes| ManagedCanonicalLink::Source(SourceId::from_bytes(bytes)))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        let mut retained = retain_explanation(&plan, realization, now_micros()?.as_unix_micros());
        // Revalidate actual serialized bytes before opening or writing the store.
        let content = encode_retained_explanation(&retained).map_err(Error::new)?;
        let mut privacy = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|e| Error::with_source("cannot open explanation storage", e))?;
        let record = privacy.record_managed_derived(ManagedDerivedDraft { project_id:project, kind:ManagedDerivedKind::CachedSummary,
            provider:None, model:retained.realization.generator.model.clone(), purpose:purpose(subject, language), analysis_snapshot:match subject { ExplanationSubject::Repository(id) => Some(id), _ => None },
            included_sources, canonical_links, content,
            uncertainty:None, retained_until:None,
            retention_basis:"Explicit active-host explanation recording; local disposable Derived content; no provider invocation".into() })
            .map_err(|e| {
                if e.kind() == volicord_privacy::ErrorKind::InvalidInput {
                    Error::new(format!("cannot retain explanation: {e}"))
                } else {
                    Error::with_source("explanation storage unavailable; inspect local storage and retry recording; no success receipt was issued",e)
                }
            })?;
        retained.publication_id = Some(record.id.to_string());
        Ok(retained)
    }
    pub fn delete_explanations(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
    ) -> Result<usize, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        let mut privacy = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|e| Error::with_source("cannot open explanation storage", e))?;
        let ids = privacy
            .inspect_project(project)
            .map_err(|e| Error::with_source("cannot inspect explanations", e))?
            .managed_derived
            .into_iter()
            .filter(|r| {
                r.kind == ManagedDerivedKind::CachedSummary
                    && r.purpose
                        .starts_with(&format!("explanation:{}:", subject.key()))
            })
            .map(|r| r.id)
            .collect::<Vec<_>>();
        privacy
            .delete_managed_ids(project, &ids, "Explicit explanation deletion")
            .map_err(|e| {
                Error::with_source(
                    "explanation cleanup incomplete; repeat explicit deletion",
                    e,
                )
            })?;
        Ok(ids.len())
    }
    pub fn read_repository_explanation(
        &self,
        project: ProjectId,
        language: &str,
        locale: FixedLocale,
    ) -> Result<volicord_projections::QuestionAnswers, Error> {
        let (projection, _) = self.project_projection_read_profiled(
            project,
            WorkSelector::Repository,
            ProjectionDetail::default(),
            ProjectionReadRequirements {
                code: false,
                inspection: false,
            },
        )?;
        Ok(explanation_answers(
            &projection.repository_explanations,
            language,
            locale,
        ))
    }
    /// Inspect one exact citation in the latest current permitted realization.
    /// This is not a Source-ID or path reader. All dependencies are revalidated
    /// under the existing inspection lock; no observation or retention write occurs.
    pub fn read_explanation_citation(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        selection: &ExplanationCitationSelection,
    ) -> Result<serde_json::Value, Error> {
        let _inspection = self.layout().acquire_health_lock()?;
        let (canonical, retained, publication_id) =
            self.permitted_read_publication(project, subject, language)?;
        if selection.publication_id != publication_id {
            return Err(Error::new(
                "selected publication was replaced; reopen the current explanation",
            )
            .with_reading_failure(ExplanationFailureReason::Changed));
        }
        let provenance = ExplanationProvenance::from(&retained);
        if provenance.realization_fingerprint != selection.realization_fingerprint {
            return Err(Error::new(
                "selected citation realization changed; reopen the current explanation",
            )
            .with_reading_failure(ExplanationFailureReason::Changed));
        }
        let unit = retained
            .realization
            .generated_units()
            .nth(selection.unit_index)
            .ok_or_else(|| Error::new("selected citation unit unavailable"))?;
        let citation = unit
            .citations
            .get(selection.citation_index)
            .ok_or_else(|| Error::new("selected citation unavailable"))?;
        let validated = self.read_retained_sources(
            &canonical,
            subject,
            &retained.realization,
            Some((citation, selection.offset)),
        )?;
        let applicability =
            self.retained_applicability(&canonical, &retained, validated.availability.clone());
        if let Some(failure) = validated
            .availability
            .iter()
            .find(|a| &a.citation == citation)
            .and_then(|a| a.limitation)
        {
            return Err(Error::new("selected evidence range is unavailable")
                .with_reading_failure(failure)
                .with_readable_publication());
        }
        let chunk = validated
            .selected
            .ok_or_else(|| Error::new("selected citation bytes unavailable"))?;
        let claim = unit
            .text
            .get(citation.claim_start_byte..citation.claim_end_byte)
            .ok_or_else(|| Error::new("selected citation claim unavailable"))?;
        let result = serde_json::json!({"publication_id":publication_id,"project_id":project.to_string(),"subject":subject,
            "language":language,"realization_fingerprint":selection.realization_fingerprint,
            "unit_index":selection.unit_index,"citation_index":selection.citation_index,
            "claim":claim,"chunk":chunk,"applicability":applicability,"read_only":true,"semantic_approval":"not_established",
            "text_byte_limit":EXPLANATION_SOURCE_RANGE_BYTE_LIMIT,"response_byte_limit":32_768});
        let bytes = serde_json::to_vec(&result)
            .map_err(|e| Error::with_source("citation encoding failed", e))?;
        if bytes.len() > 32_768 {
            return Err(Error::new("selected citation response exceeds 32768 bytes"));
        }
        Ok(result)
    }

    pub(crate) fn attach_explanations(
        &self,
        canonical: &CanonicalReadBasis,
        projection: &mut ProjectProjection,
    ) -> usize {
        let records = self.privacy_status(canonical.project.id);
        let mut latest = std::collections::BTreeMap::new();
        if let Ok(records) = &records {
            for record in records.managed_derived.iter().filter(|r| {
                r.kind == ManagedDerivedKind::CachedSummary && r.purpose.starts_with("explanation:")
            }) {
                // Privacy preserves local insertion order. UTC ties/rollback and
                // random IDs cannot revive an older output or deleted content.
                latest.insert(record.purpose.as_str(), record);
            }
        }
        let mut attached = std::collections::BTreeMap::<String, Vec<ExplanationReading>>::new();
        let mut preparations = 0;
        let mut attach = |subject: ExplanationSubject| -> Vec<ExplanationReading> {
            let key = subject.key();
            if let Some(readings) = attached.get(&key) {
                return readings.clone();
            }
            let records = match &records {
                Ok(records) => records,
                Err(e) => {
                    return vec![ExplanationReading {
                        language: "*".into(),
                        state: ExplanationState::Unavailable,
                        content: None,
                        diagnostic: Some(format!("explanation storage unavailable: {e}")),
                        applicability: ExplanationApplicability::unavailable(
                            ExplanationState::Unavailable,
                        ),
                        source_excerpts: Vec::new(),
                        omitted_source_excerpt_count: 0,
                    }]
                }
            };
            let prefix = format!("explanation:{key}:");
            let readings = latest
                .range(prefix.as_str()..)
                .take_while(|(purpose, _)| purpose.starts_with(&prefix))
                .map(|(_, record)| *record)
                .map(|record| {
                    let language = record.purpose[prefix.len()..].to_owned();
                    preparations += 1;
                    let (
                        state,
                        content,
                        diagnostic,
                        applicability,
                        source_excerpts,
                        omitted_source_excerpt_count,
                    ) = match self
                        .permitted_publication_from(canonical, records, subject, &language)
                    {
                        Ok((retained, _)) => match self.read_retained_sources(
                            canonical,
                            subject,
                            &retained.realization,
                            None,
                        ) {
                            Ok(sources) => {
                                let applicability = self.retained_applicability(
                                    canonical,
                                    &retained,
                                    sources.availability,
                                );
                                (
                                    ExplanationState::Current,
                                    Some(retained),
                                    None,
                                    applicability,
                                    sources.excerpts,
                                    sources.omitted_excerpt_count,
                                )
                            }
                            Err(e) => (
                                ExplanationState::Unavailable,
                                None,
                                Some(e.to_string()),
                                ExplanationApplicability::unavailable(
                                    ExplanationState::Unavailable,
                                ),
                                Vec::new(),
                                0,
                            ),
                        },
                        Err(e) => {
                            let state = match e.reading_failure() {
                                Some(ExplanationFailureReason::Deleted) => {
                                    ExplanationState::Deleted
                                }
                                Some(ExplanationFailureReason::Withheld) => {
                                    ExplanationState::Withheld
                                }
                                Some(ExplanationFailureReason::Corrupt) => {
                                    ExplanationState::Corrupt
                                }
                                Some(ExplanationFailureReason::Unsupported) => {
                                    ExplanationState::Unsupported
                                }
                                Some(
                                    ExplanationFailureReason::Changed
                                    | ExplanationFailureReason::Expired,
                                ) => ExplanationState::Stale,
                                _ => ExplanationState::Unavailable,
                            };
                            let mut applicability = ExplanationApplicability::unavailable(state);
                            applicability.limitation = e.reading_failure();
                            (
                                state,
                                None,
                                Some(e.to_string()),
                                applicability,
                                Vec::new(),
                                0,
                            )
                        }
                    };
                    ExplanationReading {
                        language,
                        state,
                        content,
                        diagnostic,
                        applicability,
                        source_excerpts,
                        omitted_source_excerpt_count,
                    }
                })
                .collect::<Vec<_>>();
            attached.insert(key, readings.clone());
            readings
        };
        let mut repository_subjects = std::collections::BTreeMap::new();
        if let Ok(inspection) = &records {
            for record in &inspection.managed_derived {
                if record.kind != ManagedDerivedKind::CachedSummary {
                    continue;
                }
                if let Some(rest) = record.purpose.strip_prefix("explanation:repository:") {
                    if let Some((id, language)) = rest.split_once(':') {
                        if let Ok(id) =
                            volicord_repository_intelligence::AnalysisSnapshotId::from_hex(id)
                        {
                            repository_subjects
                                .insert(language, ExplanationSubject::Repository(id));
                        }
                    }
                }
            }
        }
        for (language, subject) in repository_subjects {
            projection.repository_explanations.extend(
                attach(subject)
                    .into_iter()
                    .filter(|r| r.language == language),
            );
        }
        if let Err(error) = &records {
            projection.repository_explanations = vec![ExplanationReading {
                language: "*".into(),
                state: ExplanationState::Unavailable,
                content: None,
                diagnostic: Some(format!("explanation storage unavailable: {error}")),
                applicability: ExplanationApplicability::unavailable(ExplanationState::Unavailable),
                source_excerpts: Vec::new(),
                omitted_source_excerpt_count: 0,
            }];
        }
        for work in projection
            .selected_work
            .iter_mut()
            .chain(&mut projection.work_history)
            .chain(&mut projection.work_overview.current.items)
            .chain(&mut projection.work_overview.completed.items)
            .chain(&mut projection.work_overview.remaining.items)
            .chain(&mut projection.work_overview.next_steps.items)
            .chain(projection.resume.selected_work.iter_mut())
        {
            work.reading.explanations = attach(ExplanationSubject::Work(work.work_item_id));
            work.reading.name_explanations =
                attach(ExplanationSubject::WorkName(work.work_item_id));
            work.reading.summary_explanations =
                attach(ExplanationSubject::WorkSummary(work.work_item_id));
        }
        for decision in projection
            .decision_catalog
            .iter_mut()
            .chain(&mut projection.selected_work_decisions)
            .chain(projection.selected_decision.iter_mut())
        {
            decision.decision.explanations =
                attach(ExplanationSubject::Decision(decision.decision.decision_id));
        }
        for decision in &mut projection.resume.decisions {
            decision.explanations = attach(ExplanationSubject::Decision(decision.decision_id));
        }
        preparations
    }
}
impl LocalOperations {
    /// Reconstruct the admitted metadata using the stored repository basis,
    /// never a current file or a replacement Source body. Canonical correction
    /// remains a separate authority boundary.
    fn retained_plan(
        &self,
        canonical: &CanonicalReadBasis,
        retained: &RetainedExplanation,
    ) -> Result<ExplanationPlan, Error> {
        if let ExplanationSubject::Repository(id) = retained.subject {
            let evidence = retained
                .evidence
                .iter()
                .find(|e| e.key == "repository_basis")
                .ok_or_else(|| {
                    Error::new("publication repository basis missing")
                        .with_reading_failure(ExplanationFailureReason::Corrupt)
                })?;
            let source = evidence
                .sources
                .first()
                .ok_or_else(|| Error::new("publication repository Source missing"))?;
            volicord_projections::prepare_repository_explanation(
                canonical,
                id,
                SourceId::from_bytes(parse_identity(source)?),
                evidence.content.clone(),
                &retained.realization.language,
            )
            .map_err(Error::new)
        } else {
            self.prepare_subject_explanation(
                canonical,
                retained.subject,
                &retained.realization.language,
            )
        }
    }

    fn retained_applicability(
        &self,
        canonical: &CanonicalReadBasis,
        retained: &RetainedExplanation,
        source_ranges: Vec<SourceRangeAvailability>,
    ) -> ExplanationApplicability {
        let mut basis = ExplanationApplicability::from_retained(retained);
        basis.publication = PublicationReadability::Readable;
        basis.canonical_basis_current = Some(true);
        basis.source_ranges = source_ranges;
        let ids: std::collections::BTreeSet<_> = retained
            .realization
            .citations()
            .filter(|c| c.use_kind == ExplanationSourceUse::CurrentCode)
            .map(|c| c.analysis_snapshot_id.as_str())
            .collect();
        for id in ids {
            basis.comparison_scope = Some(RepositoryComparisonScope::IncludedInventory);
            basis.checked_at_unix_micros = now_micros().ok().map(|t| t.as_unix_micros());
            let comparison = crate::AnalysisSnapshotId::from_hex(id)
                .map_err(Error::new)
                .and_then(|id| self.repository_source_metadata(canonical.project.id, id))
                .and_then(|metadata| self.current_explanation_repository(canonical, &metadata));
            match comparison {
                Ok(_)
                    if basis.repository_comparison != RepositoryComparisonState::Changed
                        && basis.repository_comparison
                            != RepositoryComparisonState::Unavailable =>
                {
                    basis.repository_comparison = RepositoryComparisonState::VerifiedAtRead;
                }
                Err(e) if e.reading_failure() == Some(ExplanationFailureReason::Changed) => {
                    basis.repository_comparison = RepositoryComparisonState::Changed;
                    basis.limitation = Some(ExplanationFailureReason::Changed);
                }
                Err(_) if basis.repository_comparison != RepositoryComparisonState::Changed => {
                    basis.repository_comparison = RepositoryComparisonState::Unavailable;
                    basis.limitation = Some(ExplanationFailureReason::ComparisonUnavailable);
                }
                _ => {}
            }
        }
        basis
    }
}

fn decode_explanation(
    content: Option<&str>,
) -> Result<RetainedExplanation, (ExplanationState, String)> {
    let content = content.ok_or((
        ExplanationState::Unavailable,
        "explanation content withheld or unavailable".into(),
    ))?;
    if content.len() > EXPLANATION_RETAINED_BYTE_LIMIT {
        return Err((
            ExplanationState::Corrupt,
            format!("stored explanation envelope is {} bytes; limit {EXPLANATION_RETAINED_BYTE_LIMIT}; explicitly delete and regenerate from current preparation",content.len()),
        ));
    }
    let envelope: serde_json::Value = serde_json::from_str(content).map_err(|_| {
        (
            ExplanationState::Corrupt,
            "stored explanation JSON is invalid; explicitly delete and regenerate".into(),
        )
    })?;
    if envelope["realization"]["format_kind"] != EXPLANATION_KIND
        || envelope["realization"]["format_version"] != EXPLANATION_VERSION
    {
        return Err((
            ExplanationState::Unsupported,
            "unsupported explanation format; explicitly delete and regenerate".into(),
        ));
    }
    let retained: RetainedExplanation = serde_json::from_value(envelope).map_err(|_| {
        (
            ExplanationState::Corrupt,
            "stored explanation schema is invalid; explicitly delete and regenerate".into(),
        )
    })?;
    validate_diagrams(retained.subject, &retained.realization).map_err(|_| {
        (
            ExplanationState::Corrupt,
            "stored diagram meaning or binding is invalid; explicitly delete and regenerate".into(),
        )
    })?;
    Ok(retained)
}

fn retained_matches(plan: &ExplanationPlan, retained: &RetainedExplanation) -> bool {
    let mut expected = retain_explanation(
        plan,
        retained.realization.clone(),
        retained.generated_at_unix_micros,
    );
    // Store identity is assigned separately after decode/record, never retained
    // content or part of preparation equality. Citation selection checks it.
    expected.publication_id = retained.publication_id.clone();
    *retained == expected && validate_explanation(plan, &retained.realization).is_ok()
}

#[cfg(test)]
mod size_tests {
    use super::*;
    #[test]
    fn retained_decoder_uses_the_content_bound_including_formatting() -> Result<(), String> {
        let retained = RetainedExplanation {
            publication_id: None,
            realization: ExplanationRealization {
                diagrams: Vec::new(),
                format_kind: EXPLANATION_KIND.into(),
                format_version: EXPLANATION_VERSION,
                plan_fingerprint: "sha256:".to_owned() + &"a".repeat(64),
                language: "ko".into(),
                generator: ExplanationGenerator {
                    host: "authored-control".into(),
                    session: "test".into(),
                    agent: None,
                    model: None,
                },
                paragraphs: Vec::new(),
            },
            project_id: "01".repeat(16),
            subject: ExplanationSubject::Work(ContextItemId::from_bytes([1; 16])),
            question: "work_outcome".into(),
            evidence: Vec::new(),
            source_status: Vec::new(),
            conflicts: Vec::new(),
            generated_at_unix_micros: i64::MAX,
            generator_identity_status: "self_reported_not_independently_verified".into(),
            semantic_approval_status: "not_established".into(),
        };
        for target in [163839, 163840, 163841] {
            let mut content = encode_retained_explanation(&retained)?;
            content.push_str(&" ".repeat(target - content.len()));
            let result = decode_explanation(Some(&content));
            if target <= 163840 {
                assert_eq!(result.map_err(|(_, e)| e)?, retained);
            } else {
                let (state, error) = result.err().ok_or("oversize accepted")?;
                assert_eq!(state, ExplanationState::Corrupt);
                assert!(error.contains("163841") && error.contains("163840"));
            }
        }
        let mut malformed = serde_json::to_value(&retained).map_err(|e| e.to_string())?;
        malformed["subject"]["kind"] = serde_json::json!("private-source-sentinel");
        let error = decode_explanation(Some(&malformed.to_string()))
            .err()
            .ok_or("invalid schema accepted")?;
        assert_eq!(error.0, ExplanationState::Corrupt);
        assert!(!error.1.contains("private-source-sentinel"));
        for version in [EXPLANATION_VERSION - 1, EXPLANATION_VERSION + 1] {
            let mut other = retained.clone();
            other.realization.format_version = version;
            assert_eq!(
                decode_explanation(Some(
                    &serde_json::to_string(&other).map_err(|e| e.to_string())?
                ))
                .err()
                .ok_or("non-current accepted")?
                .0,
                ExplanationState::Unsupported
            );
        }
        let mut corrupt = retained.clone();
        corrupt.realization.diagrams.push(GroundedDiagram {
            id: "invalid".into(),
            title: DiagramLabel {
                unit: 0,
                start_byte: 0,
                end_byte: 1,
            },
            caption_unit: 0,
            semantic_fingerprint: String::new(),
            meaning: DiagramMeaning::Concept {
                nodes: vec![],
                relations: vec![],
            },
        });
        assert_eq!(
            decode_explanation(Some(
                &serde_json::to_string(&corrupt).map_err(|e| e.to_string())?
            ))
            .err()
            .ok_or("invalid diagram accepted")?
            .0,
            ExplanationState::Corrupt
        );
        // The writer measures complete compact JSON, including UTF-8 and escaping.
        for target in [163839, 163840, 163841] {
            let mut e = retained.clone();
            e.realization.generator.session = "한\"\\\n".repeat(20);
            let base = serde_json::to_vec(&e).map_err(|e| e.to_string())?.len();
            e.realization
                .generator
                .session
                .push_str(&"x".repeat(target - base));
            let result = encode_retained_explanation(&e);
            if target <= 163840 {
                assert_eq!(result?.len(), target);
            } else {
                assert!(result.is_err());
            }
        }
        Ok(())
    }
}
