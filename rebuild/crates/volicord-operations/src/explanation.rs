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

fn purpose(subject: ExplanationSubject, language: &str) -> String {
    format!("explanation:{}:{language}", subject.key())
}
impl LocalOperations {
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
        let records = self.privacy_status(project)?;
        for provenance in explanations {
            let target = purpose(provenance.subject, &provenance.language);
            let record = records
                .managed_derived
                .iter()
                .filter(|r| r.kind == ManagedDerivedKind::CachedSummary && r.purpose == target)
                .max_by_key(|r| (r.created_at, r.id))
                .ok_or_else(|| {
                    Error::new("explanation deleted; render again before publication")
                })?;
            let retained = decode_explanation(record.content.as_deref()).map_err(|_| {
                Error::new("explanation unavailable; render again before publication")
            })?;
            let plan = prepare_explanation(&canonical, provenance.subject, &provenance.language)
                .map_err(Error::new)?;
            if record.state != ManagedDerivedState::Current
                || ExplanationProvenance::from(&retained) != *provenance
                || !retained_matches(&plan, &retained)
            {
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
        prepare_explanation(&canonical, subject, language).map_err(Error::new)
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
        let plan = prepare_explanation(&canonical, subject, language).map_err(Error::new)?;
        validate_explanation(&plan, &realization).map_err(Error::new)?;
        let grounding =
            volicord_repository_intelligence::CanonicalGrounding::from_read_basis(&canonical)
                .map_err(|e| Error::with_source("cannot ground explanation", e))?;
        let sources = plan
            .evidence
            .iter()
            .flat_map(|e| e.sources.iter())
            .collect::<std::collections::BTreeSet<_>>();
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
        let retained = retain_explanation(&plan, realization, now_micros()?.as_unix_micros());
        // Revalidate actual serialized bytes before opening or writing the store.
        let content = encode_retained_explanation(&retained).map_err(Error::new)?;
        let mut privacy = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|e| Error::with_source("cannot open explanation storage", e))?;
        privacy.record_managed_derived(ManagedDerivedDraft { project_id:project, kind:ManagedDerivedKind::CachedSummary,
            provider:None, model:retained.realization.generator.model.clone(), purpose:purpose(subject, language), analysis_snapshot:None,
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
                if latest.get(record.purpose.as_str()).is_none_or(
                    |old: &&volicord_privacy::ManagedDerivedRecord| {
                        (old.created_at, old.id) < (record.created_at, record.id)
                    },
                ) {
                    latest.insert(record.purpose.as_str(), record);
                }
            }
        }
        let mut attached = std::collections::BTreeMap::<String, Vec<ExplanationReading>>::new();
        let mut preparations = 0;
        let mut attach = |subject: ExplanationSubject| -> Vec<ExplanationReading> {
            let key = subject.key();
            if let Some(readings) = attached.get(&key) {
                return readings.clone();
            }
            if let Err(e) = &records {
                return vec![ExplanationReading {
                    language: "*".into(),
                    state: ExplanationState::Unavailable,
                    content: None,
                    diagnostic: Some(format!("explanation storage unavailable: {e}")),
                }];
            }
            let prefix = format!("explanation:{key}:");
            let readings = latest
                .range(prefix.as_str()..)
                .take_while(|(purpose, _)| purpose.starts_with(&prefix))
                .map(|(_, record)| *record)
                .filter(|record| record.state != ManagedDerivedState::Deleted)
                .map(|record| {
                    let language = record.purpose[prefix.len()..].to_owned();
                    let (state, content, diagnostic) =
                        match decode_explanation(record.content.as_deref()) {
                            Err((state, diagnostic)) => (state, None, Some(diagnostic)),
                            Ok(retained) => {
                                preparations += 1;
                                match prepare_explanation(canonical, subject, &language) {
                            Err(e) => (ExplanationState::Unavailable, None, Some(e)),
                            Ok(plan)
                                if record.state == ManagedDerivedState::Current
                                    && retained_matches(&plan, &retained) =>
                            {
                                (ExplanationState::Current, Some(retained), None)
                            }
                            Ok(_) => (
                                ExplanationState::Stale,
                                None,
                                Some(
                                    "evidence changed or invalidated; prepare and generate again"
                                        .into(),
                                ),
                            ),
                        }
                            }
                        };
                    ExplanationReading {
                        language,
                        state,
                        content,
                        diagnostic,
                    }
                })
                .collect::<Vec<_>>();
            attached.insert(key, readings.clone());
            readings
        };
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
    serde_json::from_value(envelope).map_err(|_| {
        (
            ExplanationState::Corrupt,
            "stored explanation schema is invalid; explicitly delete and regenerate".into(),
        )
    })
}

fn retained_matches(plan: &ExplanationPlan, retained: &RetainedExplanation) -> bool {
    *retained
        == retain_explanation(
            plan,
            retained.realization.clone(),
            retained.generated_at_unix_micros,
        )
        && validate_explanation(plan, &retained.realization).is_ok()
}

#[cfg(test)]
mod size_tests {
    use super::*;
    #[test]
    fn retained_decoder_uses_the_content_bound_including_formatting() -> Result<(), String> {
        let retained = RetainedExplanation {
            realization: ExplanationRealization {
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
        };
        for target in [147455, 147456, 147457] {
            let mut content = encode_retained_explanation(&retained)?;
            content.push_str(&" ".repeat(target - content.len()));
            let result = decode_explanation(Some(&content));
            if target <= 147456 {
                assert_eq!(result.map_err(|(_, e)| e)?, retained);
            } else {
                let (state, error) = result.err().ok_or("oversize accepted")?;
                assert_eq!(state, ExplanationState::Corrupt);
                assert!(error.contains("147457") && error.contains("147456"));
            }
        }
        let mut malformed = serde_json::to_value(&retained).map_err(|e| e.to_string())?;
        malformed["subject"]["kind"] = serde_json::json!("private-source-sentinel");
        let error = decode_explanation(Some(&malformed.to_string()))
            .err()
            .ok_or("invalid schema accepted")?;
        assert_eq!(error.0, ExplanationState::Corrupt);
        assert!(!error.1.contains("private-source-sentinel"));
        // The writer measures complete compact JSON, including UTF-8 and escaping.
        for target in [147455, 147456, 147457] {
            let mut e = retained.clone();
            e.realization.generator.session = "한\"\\\n".repeat(20);
            let base = serde_json::to_vec(&e).map_err(|e| e.to_string())?.len();
            e.realization
                .generator
                .session
                .push_str(&"x".repeat(target - base));
            let result = encode_retained_explanation(&e);
            if target <= 147456 {
                assert_eq!(result?.len(), target);
            } else {
                assert!(result.is_err());
            }
        }
        Ok(())
    }
}
