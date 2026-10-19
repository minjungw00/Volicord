//! Source-bound Checkpoint admission. Explicit reported assertions are validated
//! against retained stream coordinates; process exit never manufactures a pass.
use crate::operations::new_operation_id;
use crate::{Error, LocalOperations};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use volicord_context::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkCheckRecording {
    #[serde(with = "volicord_context::check_work_id")]
    pub work_item_id: ContextItemId,
    #[serde(with = "volicord_context::check_source_id")]
    pub source_id: SourceId,
    pub assertion_state: AssertionState,
    pub assertion_evidence: Option<AssertionEvidence>,
    pub retest: Option<CheckRetestBasis>,
    pub outcome: String,
    pub next_step: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct WorkCheckReceipt {
    #[serde(with = "volicord_context::check_checkpoint_id")]
    pub checkpoint_id: CheckpointId,
    pub checkpoint_revision: u64,
    #[serde(with = "volicord_context::check_source_id")]
    pub source_id: SourceId,
    #[serde(with = "volicord_context::check_work_id")]
    pub work_item_id: ContextItemId,
}
impl LocalOperations {
    pub fn record_work_check(
        &self,
        project: ProjectId,
        request: WorkCheckRecording,
    ) -> Result<WorkCheckReceipt, Error> {
        let _mutation = self.layout().acquire_mutation_lock()?;
        let basis = self.canonical_basis(project)?;
        let work = basis
            .context_items
            .iter()
            .find(|w| w.id == request.work_item_id && w.role == ContextItemRole::Goal)
            .ok_or_else(|| Error::new("Work unavailable in Project"))?;
        let observation = self
            .retained_source_observation(&basis, work.id, request.source_id)?
            .ok_or_else(|| {
                Error::new("exact command observation unavailable; cannot attach check")
            })?;
        let detail = &observation.detail;
        if detail["kind"] != "command"
            || detail["check_capture"]["authority"] != "explicit_local_capture"
        {
            return Err(Error::new("Source has no explicit local check declaration"));
        }
        let decode = |name: &str| {
            detail[name]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| Error::new(format!("missing command {name}")))
        };
        let source = basis
            .sources
            .iter()
            .find(|s| s.source.id == request.source_id && s.availability == Availability::Available)
            .ok_or_else(|| Error::new("Command Source unavailable"))?;
        let SourcePayload::CommandExecution {
            invocation_fingerprint,
            outcome,
            ..
        } = &source.source.payload
        else {
            return Err(Error::new("not a Command Source"));
        };
        if request.assertion_state == AssertionState::Passed
            && (outcome.exit_code != Some(0)
                || outcome.termination != CommandTermination::Exited
                || detail["timeout_detected"] != false
                || detail["cancellation_requested"] != false)
        {
            return Err(Error::new(
                "passed assertion contradicts process termination",
            ));
        }
        if let Some(evidence) = &request.assertion_evidence {
            let stream = match evidence.stream {
                AssertionStream::Stdout => "stdout",
                AssertionStream::Stderr => "stderr",
            };
            let body = detail[stream]["body"]
                .as_str()
                .ok_or_else(|| Error::new("assertion stream unavailable"))?;
            let slice = body
                .get(evidence.start_byte..evidence.end_byte)
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| Error::new("invalid assertion evidence coordinates"))?;
            if format!("{:x}", Sha256::digest(slice.as_bytes())) != evidence.sha256 {
                return Err(Error::new("assertion evidence hash mismatch"));
            }
        }
        let body = serde_json::to_value(&observation)
            .map_err(|e| Error::with_source("cannot inspect observation", e))?;
        let run = ScopedCheckRun {
            work_item_id: work.id,
            work_revision: work.revision,
            declaration: serde_json::from_value(detail["check_capture"]["declaration"].clone())
                .map_err(|e| Error::with_source("check declaration unavailable", e))?,
            artifacts: serde_json::from_value(detail["check_capture"]["artifacts"].clone())
                .map_err(|e| Error::with_source("target artifacts unavailable", e))?,
            operation_id: decode("operation_id")?,
            invocation_fingerprint: invocation_fingerprint.clone(),
            analysis_snapshot_id: body["analysis_snapshot_id"]
                .as_str()
                .ok_or_else(|| Error::new("analysis missing"))?
                .into(),
            repository_snapshot_id: body["repository_snapshot_id"]
                .as_str()
                .ok_or_else(|| Error::new("repository snapshot missing"))?
                .into(),
            repository_source_id: serde_json::from_value(body["repository_source_id"].clone())
                .map_err(|e| Error::with_source("repository Source missing", e))?,
            started_at_unix_micros: detail["started_at_unix_micros"]
                .as_i64()
                .ok_or_else(|| Error::new("start missing"))?,
            ended_at_unix_micros: detail["ended_at_unix_micros"]
                .as_i64()
                .ok_or_else(|| Error::new("end missing"))?,
            target_unchanged_during_run: detail["check_capture"]["target_unchanged_during_run"]
                .as_bool()
                .ok_or_else(|| Error::new("target stability missing"))?,
            assertion_state: request.assertion_state,
            assertion_evidence: request.assertion_evidence,
            retest: request.retest,
        };
        // The relation is recorded even when its stated comparison cannot be
        // supported. Readers expose non-comparability, never repair causality.
        if let Some(previous) = &run.retest {
            let checkpoint = basis
                .checkpoint_history
                .iter()
                .find(|c| {
                    c.id == previous.checkpoint_id
                        && c.revision == previous.checkpoint_revision
                        && c.work_item_id == Some(work.id)
                })
                .ok_or_else(|| {
                    Error::new("retest requires an exact same-Work Checkpoint revision")
                })?;
            let prior = checkpoint
                .verification
                .get(previous.verification_index)
                .filter(|v| v.source_id == Some(previous.source_id))
                .and_then(|v| v.scoped_run.as_ref())
                .ok_or_else(|| {
                    Error::new("retest must name a previously recorded scoped execution")
                })?;
            if prior.ended_at_unix_micros >= run.started_at_unix_micros
                || previous.source_id == request.source_id
            {
                return Err(Error::new(
                    "retest must follow a different recorded execution",
                ));
            }
        }
        // No statement of current whole-tree coverage or acceptance is made.
        let state = match run.assertion_state {
            AssertionState::NotObserved => VerificationState::Partial,
            AssertionState::Passed => VerificationState::Passed,
            AssertionState::Failed => VerificationState::Failed,
        };
        let mut store = self.open_canonical()?;
        let checkpoint = store.record_checkpoint(new_operation_id()?, project, CheckpointDraft {
            expected_project_revision: basis.project.revision, work_item_id: Some(work.id), kind: CheckpointKind::Pause,
            goal: work.statement.clone(), work_state: WorkState::Paused, state_change: None,
            source_basis: vec![request.source_id,run.repository_source_id], changed_source_basis: vec![], changed_paths: vec![], applied_decisions: vec![],
            verification: vec![VerificationFact {state,source_id:Some(request.source_id),outcome:Some(request.outcome),scoped_run:Some(run)}],
            user_review: UserReviewFact {state:UserReviewState::NotRequested,source_id:None},
            user_acceptance: UserAcceptanceFact {state:UserAcceptanceState::NotRequested,source_id:None},
            known_limits: vec!["Explicit local capture; assertions and coverage are operator reports grounded to stream coordinates. Artifact end-point samples are not an atomic snapshot or proof of whole-tree coverage. Trusted-host capture and human acceptance are unobserved.".into()],
            non_goals: vec![],open_questions: vec![],next_step:request.next_step,handoff_to:None,
        }).map_err(|e| Error::with_source("cannot record scoped check Checkpoint",e))?.value;
        Ok(WorkCheckReceipt {
            checkpoint_id: checkpoint.id,
            checkpoint_revision: checkpoint.revision,
            source_id: request.source_id,
            work_item_id: work.id,
        })
    }
}
