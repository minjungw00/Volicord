//! Authored diagram applicability is separate from omitted or failed authorship.
use crate::{Error, LocalOperations};
use serde::{Deserialize, Serialize};
use volicord_context::ProjectId;
use volicord_privacy::{ManagedDerivedKind, ManagedDerivedState, PrivacyStore};
use volicord_projections::{ExplanationRealization, ExplanationSubject};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramAssessmentOutcome {
    Authored,
    NotNeeded,
    InsufficientEvidence,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplanationDiagramAssessment {
    pub outcome: DiagramAssessmentOutcome,
    pub rationale: String,
    #[serde(default)]
    pub source_ids: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Assessment {
    format_kind: String,
    format_version: u32,
    project_id: String,
    subject: ExplanationSubject,
    language: String,
    production_id: String,
    plan_fingerprint: String,
    assessment: ExplanationDiagramAssessment,
}
fn validate(assessment: &ExplanationDiagramAssessment) -> Result<(), Error> {
    if assessment.rationale.trim().is_empty()
        || assessment.rationale.len() > 2048
        || assessment.rationale.contains('\0')
    {
        return Err(Error::new("diagram assessment needs a nonblank rationale within 2048 UTF-8 bytes: reader question, relationships/order, and evidence sufficiency"));
    }
    Ok(())
}

impl LocalOperations {
    /// Explicit host assessment, not a semantic oracle or Source permission.
    pub fn assess_explanation_diagrams(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        production_id: &str,
        assessment: ExplanationDiagramAssessment,
    ) -> Result<serde_json::Value, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        if matches!(
            subject,
            ExplanationSubject::WorkName(_) | ExplanationSubject::WorkSummary(_)
        ) {
            return Err(Error::new(
                "diagram assessment belongs to an article, not a Work card",
            ));
        }
        validate(&assessment)?;
        if assessment.source_ids.len() > 128 {
            return Err(Error::new(
                "assessment Source count exceeds request retrieval bound",
            ));
        }
        let mut sources = Vec::new();
        for source in &assessment.source_ids {
            let id =
                volicord_context::SourceId::from_bytes(crate::operations::parse_identity(source)?);
            match subject {
                ExplanationSubject::Repository(analysis) => {
                    self.read_repository_explanation_source(project, analysis, id)?;
                }
                subject if subject.work().is_some() => {
                    self.read_explanation_source(
                        project,
                        subject.work().ok_or_else(|| Error::new("Work required"))?,
                        id,
                    )?;
                }
                _ => {
                    return Err(Error::new(
                        "Decision assessment has no Work/Repository Source-body authority",
                    ))
                }
            }
            sources.push(id);
        }
        let plan = self.prepare_explanation(project, subject, language)?;
        self.validate_explanation_production(
            project,
            subject,
            language,
            production_id,
            &plan.fingerprint,
        )?;
        let record = Assessment {
            format_kind: "volicord_explanation_diagram_assessment".into(),
            format_version: 1,
            project_id: project.to_string(),
            subject,
            language: language.into(),
            production_id: production_id.into(),
            plan_fingerprint: plan.fingerprint.clone(),
            assessment,
        };
        self.store_production_metadata(
            &plan,
            format!("explanation-assessment:{production_id}"),
            serde_json::to_string(&record)
                .map_err(|e| Error::with_source("assessment encoding failed", e))?,
            &sources,
        )?;
        self.explanation_production_status(project, subject, language)
    }

    pub(crate) fn explanation_diagram_assessment(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        production_id: &str,
        fingerprint: &str,
    ) -> Result<Option<ExplanationDiagramAssessment>, Error> {
        let records = self.explanation_dependencies(project)?;
        let purpose = format!("explanation-assessment:{production_id}");
        let Some(record) = records
            .managed_derived
            .iter()
            .rfind(|r| r.kind == ManagedDerivedKind::CachedSummary && r.purpose == purpose)
        else {
            return Ok(None);
        };
        if record.state != ManagedDerivedState::Current
            || records
                .withheld_for_canonical_forgetting
                .contains(&record.id)
        {
            return Ok(None);
        }
        let store = PrivacyStore::open(self.layout().privacy_store())
            .map_err(|e| Error::with_source("assessment storage unavailable", e))?;
        let full = store
            .get_derived(project, record.id)
            .map_err(|e| Error::with_source("assessment unavailable", e))?;
        let mut metadata = full.clone();
        metadata.content = None;
        if metadata != *record {
            return Err(Error::new(
                "assessment changed during selection; retry status",
            ));
        }
        let value: Assessment = serde_json::from_str(
            full.content
                .as_deref()
                .ok_or_else(|| Error::new("assessment withheld"))?,
        )
        .map_err(|_| {
            Error::new("assessment corrupt; explicitly assess again")
                .with_reading_failure(volicord_projections::ExplanationFailureReason::Corrupt)
        })?;
        if value.format_kind != "volicord_explanation_diagram_assessment"
            || value.format_version != 1
            || value.project_id != project.to_string()
            || value.subject != subject
            || value.language != language
            || value.production_id != production_id
            || value.plan_fingerprint != fingerprint
        {
            return Err(Error::new(
                "assessment unsupported or belongs to another production; explicitly assess again",
            ));
        }
        validate(&value.assessment)?;
        Ok(Some(value.assessment))
    }

    pub(crate) fn validate_production_diagrams(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        production_id: &str,
        realization: &ExplanationRealization,
    ) -> Result<(), Error> {
        if matches!(
            subject,
            ExplanationSubject::WorkName(_) | ExplanationSubject::WorkSummary(_)
        ) {
            return Ok(());
        }
        let assessment = self.explanation_diagram_assessment(project,subject,language,production_id,&realization.plan_fingerprint)?.ok_or_else(||Error::new("article production needs an explicit diagram assessment: assess authored, not_needed or insufficient_evidence with the reader question and evidence rationale"))?;
        match assessment.outcome {
            DiagramAssessmentOutcome::Authored if realization.diagrams.is_empty() => Err(Error::new("assessment claims authored diagrams but the realization has none; author grounded diagrams or correct the assessment")),
            DiagramAssessmentOutcome::NotNeeded | DiagramAssessmentOutcome::InsufficientEvidence if !realization.diagrams.is_empty() => Err(Error::new("diagram assessment disagrees with authored diagrams; assess the actual outcome")),
            _ => Ok(()),
        }
    }
}
