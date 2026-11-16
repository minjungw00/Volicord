//! Explicit host reader review. Bindings never establish semantic or Human approval.
use crate::{Error, LocalOperations};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use volicord_context::ProjectId;
use volicord_projections::{
    explanation_text_hash, reader_review_obligations, validate_explanation,
    ExplanationFailureReason, ExplanationRealization, ExplanationSubject,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderReviewOutcome {
    Addressed,
    NotApplicable,
    InsufficientEvidence,
    NeedsRevision,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReaderReviewCheck {
    pub obligation: String,
    pub outcome: ReaderReviewOutcome,
    pub rationale: String,
    pub unit_indices: Vec<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplanationReaderReview {
    pub realization: ExplanationRealization,
    pub checks: Vec<ReaderReviewCheck>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReaderReviewRecord {
    format_kind: String,
    format_version: u32,
    project_id: String,
    subject: ExplanationSubject,
    language: String,
    production_id: String,
    plan_fingerprint: String,
    pub(crate) realization_fingerprint: String,
    pub(crate) checks: Vec<ReaderReviewCheck>,
}

pub(crate) fn realization_fingerprint(
    realization: &ExplanationRealization,
) -> Result<String, Error> {
    serde_json::to_string(realization)
        .map(|value| explanation_text_hash(&value))
        .map_err(|e| Error::with_source("reader review realization encoding failed", e))
}

fn validate_checks(
    subject: ExplanationSubject,
    checks: &[ReaderReviewCheck],
    units: Option<usize>,
) -> Result<(), Error> {
    let expected = reader_review_obligations(subject);
    if expected.is_empty() || checks.len() != expected.len() {
        return Err(Error::new(
            "reader review requires every article obligation exactly once; it is not a card review",
        ));
    }
    let mut seen = BTreeSet::new();
    for check in checks {
        if !expected.contains(&check.obligation.as_str()) || !seen.insert(check.obligation.as_str())
        {
            return Err(Error::new("unknown or duplicate reader review obligation"));
        }
        if check.outcome == ReaderReviewOutcome::NotApplicable
            && matches!(
                check.obligation.as_str(),
                "grounding_and_limits" | "readable_structure" | "language_review"
            )
        {
            return Err(Error::new("grounding/limits, readable structure and language review always apply; missing evidence or inability to review stays incomplete"));
        }
        if check.rationale.trim().is_empty()
            || check.rationale.len() > 1536
            || check.rationale.contains('\0')
        {
            return Err(Error::new("reader review needs a concrete rationale within 1536 UTF-8 bytes; missing evidence is insufficient, not non-applicability"));
        }
        if check.unit_indices.len() > 128
            || check.unit_indices.iter().collect::<BTreeSet<_>>().len() != check.unit_indices.len()
            || units.is_some_and(|count| check.unit_indices.iter().any(|i| *i >= count))
            || (check.outcome == ReaderReviewOutcome::Addressed && check.unit_indices.is_empty())
        {
            return Err(Error::new("reader review unit references must be exact, unique and in bounds; addressed needs actual units"));
        }
        if check.obligation == "language_review"
            && check.outcome == ReaderReviewOutcome::Addressed
            && units.is_some_and(|count| {
                check.unit_indices.iter().copied().collect::<BTreeSet<_>>() != (0..count).collect()
            })
        {
            return Err(Error::new("positive language review must cover every exact unit, including unchanged text; include diagram wording in the same whole-realization review"));
        }
    }
    Ok(())
}

impl ReaderReviewRecord {
    pub(crate) fn complete(&self) -> bool {
        self.checks.iter().all(|check| {
            matches!(
                check.outcome,
                ReaderReviewOutcome::Addressed | ReaderReviewOutcome::NotApplicable
            )
        })
    }
}

impl LocalOperations {
    /// Review an admitted draft before normal production recording, not a model call.
    pub fn assess_explanation_reader(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        production_id: &str,
        review: ExplanationReaderReview,
    ) -> Result<Value, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        let plan = self.prepare_explanation(project, subject, language)?;
        self.validate_explanation_production(
            project,
            subject,
            language,
            production_id,
            &plan.fingerprint,
        )?;
        validate_explanation(&plan, &review.realization).map_err(Error::new)?;
        validate_checks(
            subject,
            &review.checks,
            Some(review.realization.paragraphs.len()),
        )?;
        let canonical = self.canonical_basis(project)?;
        let sources = if matches!(subject, ExplanationSubject::Decision(_)) {
            vec![]
        } else {
            self.validate_subject_sources(&canonical, subject, &review.realization)?
                .dependencies
        };
        let record = ReaderReviewRecord {
            format_kind: "volicord_explanation_reader_review".into(),
            format_version: 1,
            project_id: project.to_string(),
            subject,
            language: language.into(),
            production_id: production_id.into(),
            plan_fingerprint: plan.fingerprint.clone(),
            realization_fingerprint: realization_fingerprint(&review.realization)?,
            checks: review.checks,
        };
        let encoded = serde_json::to_string(&record)
            .map_err(|e| Error::with_source("reader review encoding failed", e))?;
        if encoded.len() > 16_384 {
            return Err(Error::new(format!("reader review metadata needs {} bytes; bound is 16384, preserve the obligation and report pressure", encoded.len())));
        }
        self.store_production_metadata(
            &plan,
            format!("explanation-reader-review:{production_id}"),
            encoded,
            &sources,
        )?;
        let mut status = self.explanation_production_status(project, subject, language)?;
        status["read_only"] = json!(false);
        status["metadata_written"] = json!(true);
        Ok(status)
    }

    pub(crate) fn explanation_reader_review(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        production_id: &str,
        fingerprint: &str,
    ) -> Result<Option<ReaderReviewRecord>, Error> {
        let Some(content) = self.read_production_metadata(
            project,
            &format!("explanation-reader-review:{production_id}"),
        )?
        else {
            return Ok(None);
        };
        let value: Value = serde_json::from_str(&content).map_err(|_| {
            Error::new("reader review metadata corrupt; explicitly produce a fresh refinement, then review its exact realization")
                .with_reading_failure(ExplanationFailureReason::Corrupt)
        })?;
        if value["format_kind"] != "volicord_explanation_reader_review"
            || value["format_version"] != 1
        {
            return Err(
                Error::new("reader review metadata unsupported; explicitly produce a fresh refinement, then review its exact realization")
                    .with_reading_failure(ExplanationFailureReason::Unsupported),
            );
        }
        let record: ReaderReviewRecord = serde_json::from_value(value).map_err(|_| {
            Error::new("reader review metadata malformed; explicitly produce a fresh refinement, then review its exact realization")
                .with_reading_failure(ExplanationFailureReason::Corrupt)
        })?;
        if record.project_id != project.to_string()
            || record.subject != subject
            || record.language != language
            || record.production_id != production_id
            || record.plan_fingerprint != fingerprint
        {
            return Err(Error::new(
                "reader review belongs to another Project, subject, language or preparation",
            ));
        }
        validate_checks(subject, &record.checks, None)?;
        Ok(Some(record))
    }

    pub(crate) fn validate_production_reader_review(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        language: &str,
        production_id: &str,
        realization: &ExplanationRealization,
    ) -> Result<(), Error> {
        if reader_review_obligations(subject).is_empty() {
            return Ok(());
        }
        let record = self.explanation_reader_review(project, subject, language, production_id, &realization.plan_fingerprint)?
            .ok_or_else(|| Error::new("article production needs an explicit exact-realization host reader review before record; inspect reader_review_obligations, review the actual language and use production-review/action=review"))?;
        if record.realization_fingerprint != realization_fingerprint(realization)? {
            return Err(Error::new("realization changed after reader review; reread all changed units, claims and diagram roles and explicitly review the exact realization again"));
        }
        validate_checks(subject, &record.checks, Some(realization.paragraphs.len()))
    }
}
