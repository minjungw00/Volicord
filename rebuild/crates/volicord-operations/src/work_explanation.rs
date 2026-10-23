//! Explicit host-authored Work production. No model dispatch or persistence here.
use crate::{operations::parse_identity, Error, FileByteRange, LocalOperations};
use serde::{Deserialize, Serialize};
use volicord_context::{ContextItemId, DecisionId, ProjectId, SourceId};
use volicord_projections::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkExplanationDraft {
    #[serde(default)]
    pub diagrams: Vec<volicord_projections::GroundedDiagram>,
    pub plan_fingerprint: String,
    pub language: String,
    pub generator: ExplanationGenerator,
    pub paragraphs: Vec<WorkReaderUnit>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkReaderUnit {
    pub question: ExplanationQuestion,
    pub text: String,
    pub evidence_keys: Vec<String>,
    pub sources: Vec<WorkClaimRange>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkClaimRange {
    pub source_id: String,
    pub field: String,
    pub use_kind: ExplanationSourceUse,
    pub start_byte: usize,
    pub end_byte: usize,
    pub claim: Option<FileByteRange>,
}
impl LocalOperations {
    /// Recorded Decision interpretation uses its actual prepared recommendation
    /// rationale. This grants no Work/repository Source-body authority.
    pub fn compose_decision_explanation(
        &self,
        project: ProjectId,
        decision: DecisionId,
        draft: WorkExplanationDraft,
    ) -> Result<ExplanationRealization, Error> {
        self.compose_subject_explanation(project, ExplanationSubject::Decision(decision), draft)
    }
    /// Bind a newly authored name, synopsis and narrative to current Work evidence.
    /// No unavailable old realization is read, edited or partially revived.
    pub fn compose_work_explanation(
        &self,
        project: ProjectId,
        work: ContextItemId,
        draft: WorkExplanationDraft,
    ) -> Result<ExplanationRealization, Error> {
        self.compose_subject_explanation(project, ExplanationSubject::Work(work), draft)
    }

    pub fn compose_subject_explanation(
        &self,
        project: ProjectId,
        subject: ExplanationSubject,
        draft: WorkExplanationDraft,
    ) -> Result<ExplanationRealization, Error> {
        let canonical = self.canonical_basis(project)?;
        let plan = self.prepare_subject_explanation(&canonical, subject, &draft.language)?;
        if draft.plan_fingerprint != plan.fingerprint {
            return Err(Error::new("stale Work preparation; prepare again"));
        }
        if draft.paragraphs.len() > 32 || draft.paragraphs.iter().any(|p| p.sources.len() > 32) {
            return Err(Error::new("Work document exceeds unit/citation bound"));
        }
        let roles: &[_] = match subject {
            ExplanationSubject::WorkName(_) => &[ExplanationQuestion::Heading],
            ExplanationSubject::WorkSummary(_) => &[ExplanationQuestion::Synopsis],
            _ => &[ExplanationQuestion::Heading, ExplanationQuestion::Synopsis],
        };
        for &role in roles {
            if draft
                .paragraphs
                .iter()
                .filter(|p| p.question == role)
                .count()
                != 1
            {
                return Err(Error::new("Work production requires one authored heading and synopsis; explain the problem, actual approach and supported outcome"));
            }
        }
        let mut paragraphs = Vec::new();
        for unit in draft.paragraphs {
            let mut citations = Vec::new();
            for selected in unit.sources {
                let Some(work) = subject.work() else {
                    return Err(Error::new("Decision composition cannot borrow Work Source authority; use exact recorded Decision evidence"));
                };
                let source = SourceId::from_bytes(parse_identity(&selected.source_id)?);
                let read = self.read_explanation_source(project, work, source)?;
                let detail = &read["observation"]["detail"];
                let (body, locator) = match selected.field.as_str() {
                    "body" if detail["kind"] == "file" => {
                        (detail["body"].as_str(), detail["path"].as_str())
                    }
                    "stdout" | "stderr" if detail["kind"] == "command" => {
                        let original = canonical
                            .sources
                            .iter()
                            .find(|s| s.source.id == source)
                            .ok_or_else(|| Error::new("command Source unavailable"))?;
                        let volicord_context::SourcePayload::CommandExecution {
                            command_label, ..
                        } = &original.source.payload
                        else {
                            return Err(Error::new("command Source kind mismatch"));
                        };
                        (
                            detail[&selected.field]["body"].as_str(),
                            Some(command_label.as_str()),
                        )
                    }
                    _ => return Err(Error::new("Work Source field is not permitted")),
                };
                let body = body.ok_or_else(|| Error::new("Work Source body unavailable"))?;
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
                let binding = &read["citation_basis"];
                let required = |key: &str| {
                    binding[key]
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| Error::new("Source citation binding unavailable"))
                };
                citations.push(ExplanationCitation {
                    source_id: source.to_string(),
                    source_revision: binding["source_revision"]
                        .as_u64()
                        .ok_or_else(|| Error::new("Source revision unavailable"))?,
                    observation_fingerprint: required("observation_fingerprint")?,
                    analysis_snapshot_id: required("analysis_snapshot_id")?,
                    repository_snapshot_id: required("repository_snapshot_id")?,
                    repository_source_id: required("repository_source_id")?,
                    locator: locator
                        .ok_or_else(|| Error::new("Source locator unavailable"))?
                        .into(),
                    start_line: line(start),
                    end_line: line(end - 1),
                    field: selected.field,
                    start_byte: selected.start_byte,
                    end_byte: selected.end_byte,
                    range_sha256: explanation_text_hash(range),
                    claim_start_byte: claim.start_byte,
                    claim_end_byte: claim.end_byte,
                    claim_sha256: explanation_text_hash(claim_text),
                    use_kind: selected.use_kind,
                });
            }
            paragraphs.push(ExplanationParagraph {
                question: unit.question,
                text: unit.text,
                evidence_keys: unit.evidence_keys,
                citations,
            });
        }
        let mut realization = ExplanationRealization {
            format_kind: EXPLANATION_KIND.into(),
            format_version: EXPLANATION_VERSION,
            plan_fingerprint: draft.plan_fingerprint,
            language: draft.language,
            generator: draft.generator,
            paragraphs,
            diagrams: draft.diagrams,
        };
        seal_diagrams(&mut realization).map_err(Error::new)?;
        validate_explanation(&plan, &realization).map_err(Error::new)?;
        self.validate_subject_sources(&canonical, subject, &realization)?;
        Ok(realization)
    }
}
