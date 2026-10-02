//! Explicit interactive-host generation lifecycle, owned Derived storage, and
//! read-time freshness. No read can generate, dispatch, or transmit anything.
use crate::operations::{now_micros, parse_identity};
use crate::{Error, LocalOperations};
use serde_json::Value;
use volicord_context::{CanonicalReadBasis, CheckpointId, ContextItemId, ProjectId, SourceId};
use volicord_privacy::{
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedKind, ManagedDerivedState,
    PrivacyStore,
};
use volicord_projections::*;

fn purpose(work: ContextItemId, language: &str) -> String {
    format!("work_outcome:{work}:{language}")
}
impl LocalOperations {
    pub fn prepare_work_explanation(
        &self,
        project: ProjectId,
        work: ContextItemId,
        language: &str,
    ) -> Result<WorkExplanationPlan, Error> {
        let canonical = self.canonical_basis(project)?;
        WorkSelector::ExactWork(work)
            .resolve(&canonical)
            .map_err(|e| Error::with_source("Work selection failed", e))?;
        prepare_work_explanation(&canonical, work, language).map_err(Error::new)
    }
    pub fn record_work_explanation(
        &self,
        project: ProjectId,
        work: ContextItemId,
        language: &str,
        realization: WorkExplanationRealization,
    ) -> Result<RetainedWorkExplanation, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        let canonical = self.canonical_basis(project)?;
        let plan = prepare_work_explanation(&canonical, work, language).map_err(Error::new)?;
        validate_work_explanation(&plan, &realization).map_err(Error::new)?;
        let grounding =
            volicord_repository_intelligence::CanonicalGrounding::from_read_basis(&canonical)
                .map_err(|e| Error::with_source("cannot ground Work explanation", e))?;
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
        let mut canonical_links = vec![ManagedCanonicalLink::ContextItem(work)];
        for e in &plan.evidence {
            let bytes = parse_identity(&e.identity)?;
            canonical_links.push(match e.record_kind.as_str() {
                "context_item" => {
                    ManagedCanonicalLink::ContextItem(ContextItemId::from_bytes(bytes))
                }
                "checkpoint" => ManagedCanonicalLink::Checkpoint(CheckpointId::from_bytes(bytes)),
                _ => return Err(Error::new("unsupported Work explanation evidence kind")),
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
        let mut evidence = plan.evidence;
        for e in &mut evidence {
            e.content = Value::Null;
        }
        let mut source_status = plan.source_status;
        for status in &mut source_status {
            if let Some(object) = status.as_object_mut() {
                object.remove("observation");
            }
        }
        let retained = RetainedWorkExplanation {
            realization,
            project_id: project.to_string(),
            work_item_id: work.to_string(),
            question: plan.question,
            evidence,
            source_status,
            conflicts: plan.conflicts,
            generated_at_unix_micros: now_micros()?.as_unix_micros(),
            generator_identity_status: "self_reported_not_independently_verified".into(),
        };
        let mut privacy = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|e| Error::with_source("cannot open explanation storage", e))?;
        privacy.record_managed_derived(ManagedDerivedDraft { project_id:project, kind:ManagedDerivedKind::CachedSummary,
            provider:None, model:retained.realization.generator.model.clone(), purpose:purpose(work, language), analysis_snapshot:None,
            included_sources, canonical_links, content:serde_json::to_string(&retained).map_err(|e|Error::with_source("cannot encode Work explanation",e))?,
            uncertainty:None, retained_until:None,
            retention_basis:"Explicit active-host Work explanation recording; local disposable Derived content; no provider invocation".into() })
            .map_err(|e|Error::with_source("cannot retain Work explanation",e))?;
        Ok(retained)
    }
    pub fn delete_work_explanations(
        &self,
        project: ProjectId,
        work: ContextItemId,
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
                    && r.purpose.starts_with(&format!("work_outcome:{work}:"))
            })
            .map(|r| r.id)
            .collect::<Vec<_>>();
        privacy
            .delete_managed_ids(project, &ids, "Explicit Work explanation deletion")
            .map_err(|e| {
                Error::with_source(
                    "explanation cleanup incomplete; repeat explicit deletion",
                    e,
                )
            })?;
        Ok(ids.len())
    }
    pub(crate) fn attach_work_explanations(
        &self,
        canonical: &CanonicalReadBasis,
        projection: &mut ProjectProjection,
    ) {
        let records = self.privacy_status(canonical.project.id);
        let attach = |work: &mut UnderstandingWork| match &records {
            Err(e) => work.reading.explanations.push(WorkExplanationReading {
                language: "*".into(),
                state: WorkExplanationState::Unavailable,
                content: None,
                diagnostic: Some(format!("explanation storage unavailable: {e}")),
            }),
            Ok(records) => {
                let prefix = format!("work_outcome:{}:", work.work_item_id);
                let mut latest = std::collections::BTreeMap::new();
                for r in records.managed_derived.iter().filter(|r| {
                    r.kind == ManagedDerivedKind::CachedSummary && r.purpose.starts_with(&prefix)
                }) {
                    if latest.get(&r.purpose).is_none_or(
                        |old: &&volicord_privacy::ManagedDerivedRecord| {
                            (old.created_at, old.id) < (r.created_at, r.id)
                        },
                    ) {
                        latest.insert(&r.purpose, r);
                    }
                }
                for record in latest.into_values() {
                    if record.state == ManagedDerivedState::Deleted {
                        continue;
                    }
                    let language = record.purpose[prefix.len()..].to_owned();
                    let decoded = decode_work_explanation(record.content.as_deref());
                    let (state, content, diagnostic) = match decoded {
                        Err((state, diagnostic)) => (state, None, Some(diagnostic)),
                        Ok(retained) => {
                            let plan =
                                prepare_work_explanation(canonical, work.work_item_id, &language);
                            let current = plan.as_ref().is_ok_and(|p| {
                                p.fingerprint == retained.realization.plan_fingerprint
                                    && retained.project_id == p.project_id
                                    && retained.work_item_id == p.work_item_id
                                    && validate_work_explanation(p, &retained.realization).is_ok()
                            });
                            if let Err(error) = plan {
                                (WorkExplanationState::Unavailable, None, Some(error))
                            } else if current && record.state == ManagedDerivedState::Current {
                                (WorkExplanationState::Current, Some(retained), None)
                            } else {
                                (WorkExplanationState::Stale,None,Some("evidence changed or invalidated; prepare and generate again".into()))
                            }
                        }
                    };
                    work.reading.explanations.push(WorkExplanationReading {
                        language,
                        state,
                        content,
                        diagnostic,
                    });
                }
            }
        };
        for w in projection
            .selected_work
            .iter_mut()
            .chain(&mut projection.work_history)
            .chain(&mut projection.work_overview.current.items)
            .chain(&mut projection.work_overview.completed.items)
            .chain(&mut projection.work_overview.remaining.items)
            .chain(&mut projection.work_overview.next_steps.items)
        {
            attach(w);
        }
    }
}
fn decode_work_explanation(
    content: Option<&str>,
) -> Result<RetainedWorkExplanation, (WorkExplanationState, String)> {
    let content = content.ok_or((
        WorkExplanationState::Unavailable,
        "explanation content withheld or unavailable".into(),
    ))?;
    if content.len() > WORK_EXPLANATION_PLAN_BYTE_LIMIT {
        return Err((
            WorkExplanationState::Corrupt,
            "stored explanation exceeds budget".into(),
        ));
    }
    let envelope: Value = serde_json::from_str(content)
        .map_err(|e| (WorkExplanationState::Corrupt, e.to_string()))?;
    if envelope["realization"]["format_kind"] != WORK_EXPLANATION_KIND
        || envelope["realization"]["format_version"] != WORK_EXPLANATION_VERSION
    {
        return Err((
            WorkExplanationState::Unsupported,
            "unsupported Work explanation format; explicitly delete and regenerate".into(),
        ));
    }
    serde_json::from_value(envelope).map_err(|e| (WorkExplanationState::Corrupt, e.to_string()))
}
