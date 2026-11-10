//! Explicit host production intent, in the existing disposable managed store.
//! Status is a read; only a host invocation starts/retries production.
use crate::{operations::parse_identity, Error, LocalOperations};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use volicord_context::{CanonicalReadBasis, ContextItemId, DecisionId, ProjectId};
use volicord_privacy::{
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedKind, ManagedDerivedState,
    PrivacyStore,
};
use volicord_projections::{ExplanationFailureReason, ExplanationPlan, ExplanationSubject};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationProductionReason {
    Initiation,
    Progress,
    Boundary,
    Refinement,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationProductionFailure {
    HostUnavailable,
    SourceUnavailable,
    AuthoringFailed,
    Interrupted,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    format_kind: String,
    format_version: u32,
    project_id: String,
    subject: ExplanationSubject,
    language: String,
    plan_fingerprint: String,
    reason: ExplanationProductionReason,
    previous_publication_id: Option<String>,
    failure: Option<ExplanationProductionFailure>,
}

fn purpose(subject: ExplanationSubject, language: &str) -> String {
    format!("explanation-production:{}:{language}", subject.key())
}

impl LocalOperations {
    fn production_intent(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
    ) -> Result<Option<(String, Intent)>, Error> {
        let records = self.explanation_dependencies(project)?;
        let target = purpose(subject, language);
        let Some(record) = records
            .managed_derived
            .iter()
            .rfind(|r| r.kind == ManagedDerivedKind::CachedSummary && r.purpose == target)
        else {
            return Ok(None);
        };
        if records
            .withheld_for_canonical_forgetting
            .contains(&record.id)
            || record.state != ManagedDerivedState::Current
        {
            return Ok(None);
        }
        let store = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|e| Error::with_source("production storage unavailable", e))?;
        let full = store
            .get_derived(project, record.id)
            .map_err(|e| Error::with_source("production intent unavailable", e))?;
        let mut metadata = full.clone();
        metadata.content = None;
        if metadata != *record {
            return Err(Error::new(
                "production intent changed during selection; retry status",
            ));
        }
        let intent: Intent = serde_json::from_str(full.content.as_deref().ok_or_else(|| Error::new("production intent withheld"))?)
            .map_err(|_| Error::new("production intent corrupt; preserve it and explicitly produce from current evidence").with_reading_failure(ExplanationFailureReason::Corrupt))?;
        if intent.format_kind != "volicord_explanation_production"
            || intent.format_version != 1
            || intent.project_id != project.to_string()
            || intent.subject != subject
            || intent.language != language
        {
            return Err(Error::new("unsupported or foreign production intent; explicitly produce from current evidence").with_reading_failure(ExplanationFailureReason::Unsupported));
        }
        Ok(Some((record.id.to_string(), intent)))
    }

    fn publication_for_production(
        &self,
        canonical: &CanonicalReadBasis,
        subject: ExplanationSubject,
        language: &str,
        plan: &ExplanationPlan,
    ) -> Result<(Option<String>, bool), Error> {
        let records = self.explanation_dependencies(canonical.project.id)?;
        match self.permitted_publication_from(canonical, &records, subject, language) {
            Ok((publication, id)) => Ok((
                Some(id),
                publication.realization.plan_fingerprint == plan.fingerprint,
            )),
            Err(e) if e.reading_failure().is_some() => Ok((
                records
                    .managed_derived
                    .iter()
                    .rfind(|r| {
                        r.kind == ManagedDerivedKind::CachedSummary
                            && r.purpose == format!("explanation:{}:{language}", subject.key())
                    })
                    .map(|r| r.id.to_string()),
                false,
            )),
            Err(e) => Err(e),
        }
    }

    /// Complete exact-scope production state, independent of prose readability.
    /// No Source collection, model invocation, persistence or lifetime renewal.
    pub fn explanation_production_status(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
    ) -> Result<Value, Error> {
        let canonical = self.canonical_basis(project)?;
        let plan = self.prepare_subject_explanation(&canonical, subject, language)?;
        let (publication_id, current) =
            self.publication_for_production(&canonical, subject, language, &plan)?;
        let intent = self.production_intent(project, subject, language)?;
        let state = match &intent {
            Some((_, i)) if i.plan_fingerprint != plan.fingerprint => "basis_changed",
            Some((_, i)) if current && publication_id != i.previous_publication_id => "complete",
            Some((_, i))
                if !current
                    && publication_id.is_some()
                    && publication_id != i.previous_publication_id =>
            {
                "publication_unavailable"
            }
            Some((_, i)) if i.failure.is_some() => "failed",
            Some(_) => "awaiting_host_interpretation",
            None if current => "current",
            None => "needed",
        };
        let assessment = intent
            .as_ref()
            // An assessment belongs to its exact production preparation. A
            // changed basis needs a new assessment, not validation of the old
            // assessment against a plan it never claimed to evaluate.
            .filter(|(_, intent)| intent.plan_fingerprint == plan.fingerprint)
            .map(|(id, _)| {
                self.explanation_diagram_assessment(
                    project,
                    subject,
                    language,
                    id,
                    &plan.fingerprint,
                )
            })
            .transpose()?
            .flatten();
        let article = matches!(
            subject,
            ExplanationSubject::Work(_)
                | ExplanationSubject::Repository(_)
                | ExplanationSubject::Decision(_)
        );
        let content_complete = matches!(state, "complete" | "current")
            && (!article
                || assessment.as_ref().is_some_and(|a| {
                    a.outcome != crate::DiagramAssessmentOutcome::InsufficientEvidence
                }));
        Ok(
            json!({"diagram_assessment":assessment,"content_complete":content_complete,"project_id":project.to_string(),"subject":subject,"language":language,
            "state":state,"plan_fingerprint":plan.fingerprint,"publication_id":publication_id,
            "production_id":intent.as_ref().map(|(id,_)|id),"intent":intent.as_ref().map(|(_,i)|i),
            "read_only":true,"generation_required":!matches!(state,"complete"|"current"),
            "recovery":"Explicitly produce this exact scope/language from current evidence; restart requires fresh request-local Source reads. Ordinary Work and independently permitted publications remain usable."}),
        )
    }

    /// Authorized host invocation. Unchanged successful bases cause no write.
    /// Refinement is the explicit exception and always requests new authorship.
    pub fn begin_explanation_production(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        reason: ExplanationProductionReason,
    ) -> Result<Value, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        let canonical = self.canonical_basis(project)?;
        let plan = self.prepare_subject_explanation(&canonical, subject, language)?;
        let status = match self.explanation_production_status(project, subject, language) {
            Ok(status) => status,
            Err(error)
                if matches!(
                    error.reading_failure(),
                    Some(ExplanationFailureReason::Corrupt | ExplanationFailureReason::Unsupported)
                ) =>
            {
                json!({"generation_required":true})
            }
            Err(error) => return Err(error),
        };
        if reason != ExplanationProductionReason::Refinement
            && status["generation_required"] == false
        {
            return Ok(
                json!({"production":status,"generation_status":"skipped_unchanged_basis","retention_renewed":false}),
            );
        }
        let (previous_publication_id, _) =
            self.publication_for_production(&canonical, subject, language, &plan)?;
        let intent = Intent {
            format_kind: "volicord_explanation_production".into(),
            format_version: 1,
            project_id: project.to_string(),
            subject,
            language: language.into(),
            plan_fingerprint: plan.fingerprint.clone(),
            reason,
            previous_publication_id,
            failure: None,
        };
        let id = self.store_production_intent(&plan, &intent)?;
        Ok(
            json!({"production_id":id,"plan":plan,"generation_status":"awaiting_host_interpretation","retention_renewed":false}),
        )
    }

    fn store_production_intent(
        &self,
        plan: &ExplanationPlan,
        intent: &Intent,
    ) -> Result<String, Error> {
        self.store_production_metadata(
            plan,
            purpose(intent.subject, &intent.language),
            serde_json::to_string(intent)
                .map_err(|e| Error::with_source("production encoding failed", e))?,
            &[],
        )
    }

    pub(crate) fn store_production_metadata(
        &self,
        plan: &ExplanationPlan,
        purpose: String,
        content: String,
        additional_sources: &[volicord_context::SourceId],
    ) -> Result<String, Error> {
        let mut links = Vec::new();
        for evidence in &plan.evidence {
            let bytes = || parse_identity(&evidence.identity);
            let link = match evidence.record_kind.as_str() {
                "context_item" => Some(ManagedCanonicalLink::ContextItem(
                    ContextItemId::from_bytes(bytes()?),
                )),
                "decision" => Some(ManagedCanonicalLink::Decision(DecisionId::from_bytes(
                    bytes()?,
                ))),
                "checkpoint" => Some(ManagedCanonicalLink::Checkpoint(
                    volicord_context::CheckpointId::from_bytes(bytes()?),
                )),
                "question" => Some(ManagedCanonicalLink::Question(
                    volicord_context::QuestionId::from_bytes(bytes()?),
                )),
                _ => None,
            };
            links.extend(link);
            for source in &evidence.sources {
                links.push(ManagedCanonicalLink::Source(
                    volicord_context::SourceId::from_bytes(parse_identity(source)?),
                ));
            }
        }
        links.extend(
            additional_sources
                .iter()
                .copied()
                .map(ManagedCanonicalLink::Source),
        );
        let project = ProjectId::from_bytes(parse_identity(&plan.project_id)?);
        let mut store = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|e| Error::with_source("production storage unavailable", e))?;
        let record = store.record_managed_derived(ManagedDerivedDraft {
            project_id:project, kind:ManagedDerivedKind::CachedSummary, provider:None, model:None,
            purpose, analysis_snapshot:None, included_sources:vec![], canonical_links:links,
            content, uncertainty:None, retained_until:None,
            retention_basis:"Explicit host production intent and recovery metadata; no prose, Source bodies, background dispatch or provider authority".into(),
        }).map_err(|e|Error::with_source("production intent was not retained",e))?;
        Ok(record.id.to_string())
    }

    pub fn validate_explanation_production(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        production_id: &str,
        fingerprint: &str,
    ) -> Result<(), Error> {
        let Some((id, intent)) = self.production_intent(project, subject, language)? else {
            return Err(Error::new(
                "production intent unavailable; explicitly produce again",
            ));
        };
        if id != production_id || intent.plan_fingerprint != fingerprint || intent.failure.is_some()
        {
            return Err(Error::new("production superseded, failed or belongs to another preparation; inspect status and explicitly produce again"));
        }
        if self.explanation_production_status(project, subject, language)?["state"] == "complete" {
            return Err(Error::new("production already completed; read its publication or explicitly request refinement before another recording"));
        }
        Ok(())
    }

    pub fn fail_explanation_production(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        production_id: &str,
        failure: ExplanationProductionFailure,
    ) -> Result<Value, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        let Some((id, mut intent)) = self.production_intent(project, subject, language)? else {
            return Err(Error::new("production intent unavailable"));
        };
        if id != production_id {
            return Err(Error::new("production superseded; inspect status"));
        }
        let plan = self.prepare_explanation(project, subject, language)?;
        if plan.fingerprint != intent.plan_fingerprint {
            return Err(Error::new(
                "production basis changed; explicitly produce again",
            ));
        }
        self.validate_explanation_production(
            project,
            subject,
            language,
            production_id,
            &plan.fingerprint,
        )?;
        intent.failure = Some(failure);
        self.store_production_intent(&plan, &intent)?;
        let mut status = self.explanation_production_status(project, subject, language)?;
        status["read_only"] = json!(false);
        status["metadata_written"] = json!(true);
        Ok(status)
    }
}
