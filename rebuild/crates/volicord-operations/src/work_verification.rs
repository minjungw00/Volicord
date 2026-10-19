//! Enrich canonical verification with explicitly retained same-Work process
//! metadata. Independent of generated prose and citation/excerpt selection.
use crate::{Error, LocalOperations};
use std::collections::BTreeMap;
use volicord_context::{CanonicalReadBasis, ContextItemId, ProjectId, SourceId};
use volicord_projections::*;

impl LocalOperations {
    pub fn read_work_verification(
        &self,
        project: ProjectId,
        work: ContextItemId,
    ) -> Result<WorkVerificationReport, Error> {
        let projection =
            self.project_projection_selected(project, WorkSelector::ExactWork(work))?;
        Ok(projection
            .selected_work
            .ok_or_else(|| Error::new("Work unavailable"))?
            .reading
            .verification_report)
    }

    pub(crate) fn attach_work_verification(
        &self,
        canonical: &CanonicalReadBasis,
        projection: &mut ProjectProjection,
    ) {
        let records = self.privacy_status(canonical.project.id);
        let mut associations: BTreeMap<ContextItemId, Vec<SourceId>> = BTreeMap::new();
        if let Ok(records) = &records {
            for record in &records.managed_derived {
                if record.kind != volicord_privacy::ManagedDerivedKind::SourceObservation {
                    continue;
                }
                if let Some((work, source)) = record
                    .purpose
                    .strip_prefix("work-source:")
                    .and_then(|s| s.split_once(':'))
                {
                    if let (Ok(work), Ok(source)) = (
                        crate::operations::parse_identity(work),
                        crate::operations::parse_identity(source),
                    ) {
                        associations
                            .entry(ContextItemId::from_bytes(work))
                            .or_default()
                            .push(SourceId::from_bytes(source));
                    }
                }
            }
        }
        let mut cache = BTreeMap::new();
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
            let report = cache.entry(work.work_item_id).or_insert_with(|| {
                let mut report = work.reading.verification_report.clone();
                let source_ids = associations
                    .get(&work.work_item_id)
                    .cloned()
                    .unwrap_or_default();
                for id in &source_ids {
                    if !report
                        .executions
                        .iter()
                        .any(|e| e.source_id == id.to_string())
                    {
                        if let Some(execution) =
                            execution_reading(canonical, &id.to_string(), &report.checks)
                        {
                            report.executions.push(execution);
                        }
                    }
                }
                for execution in &mut report.executions {
                    let source = SourceId::from_bytes(
                        match crate::operations::parse_identity(&execution.source_id) {
                            Ok(id) => id,
                            Err(_) => continue,
                        },
                    );
                    if records.is_err() {
                        execution.body_state = CommandBodyState::Unavailable;
                        execution.observation = EvidenceDetail::Unavailable;
                        continue;
                    }
                    if !source_ids.contains(&source) {
                        continue;
                    }
                    match self.source_observation_read(canonical, work.work_item_id, source) {
                        Ok(read) => {
                            execution.body_state = match read.reason {
                                "available" => CommandBodyState::Available,
                                "source_body_deleted" => CommandBodyState::Deleted,
                                "source_body_expired" => CommandBodyState::Expired,
                                "work_revision_changed" => CommandBodyState::Changed,
                                "source_body_invalidated" => CommandBodyState::Invalidated,
                                _ => CommandBodyState::Unavailable,
                            };
                            execution.observation = read
                                .body
                                .as_ref()
                                .and_then(|body| scoped_command(body, &read.metadata))
                                .map_or(EvidenceDetail::Unavailable, EvidenceDetail::Observed);
                            if matches!(execution.observation, EvidenceDetail::Observed(_)) {
                                execution.observation_fingerprint = observation_fingerprint(&read);
                            }
                            if execution.observation_fingerprint.is_none() {
                                execution.observation = EvidenceDetail::Unavailable;
                                if execution.body_state == CommandBodyState::Available {
                                    execution.body_state = CommandBodyState::Unavailable;
                                }
                            }
                        }
                        Err(_) => {
                            execution.body_state = CommandBodyState::Unavailable;
                            execution.observation = EvidenceDetail::Unavailable;
                        }
                    }
                }
                report.executions.sort_by(|a, b| {
                    (a.recorded_at_unix_micros, &a.source_id)
                        .cmp(&(b.recorded_at_unix_micros, &b.source_id))
                });
                report
            });
            work.reading.verification_report = report.clone();
        }
    }

    pub(crate) fn validate_work_verification_publication(
        &self,
        project: ProjectId,
        proofs: &[WorkVerificationBasis],
    ) -> Result<(), Error> {
        let canonical = self.canonical_basis(project)?;
        for proof in proofs {
            let source = SourceId::from_bytes(crate::operations::parse_identity(&proof.source_id)?);
            let read = self.source_observation_read(&canonical, proof.work_item_id, source)?;
            if read.body.is_none()
                || observation_fingerprint(&read).as_deref()
                    != Some(proof.observation_fingerprint.as_str())
            {
                return Err(Error::new("verification Source changed or became unavailable; render again before publication"));
            }
        }
        Ok(())
    }
}

fn observation_fingerprint(read: &crate::source_observations::ObservationRead) -> Option<String> {
    serde_json::to_string(&serde_json::json!({"observation":read.body,"metadata":read.metadata}))
        .ok()
        .map(|text| explanation_text_hash(&text))
}

fn scoped_command(
    body: &serde_json::Value,
    metadata: &serde_json::Value,
) -> Option<ScopedCommandReading> {
    let detail = &body["detail"];
    if detail["kind"] != "command"
        || detail["repository_state"] != "selected_analysis_only_not_rechecked"
    {
        return None;
    }
    let stream = |field: &str| {
        Some(StreamCapture {
            observed_bytes: detail[field]["observed_bytes"].as_u64()?,
            omitted_bytes: detail[field]["omitted_bytes"].as_u64(),
            representation: match detail[field]["representation"].as_str()? {
                "whole_stream" => StreamRepresentation::WholeStream,
                "bounded_excerpt" => StreamRepresentation::BoundedExcerpt,
                _ => StreamRepresentation::Unavailable,
            },
            capture_completeness: match detail[field]["capture_completeness"].as_str() {
                Some("complete") => CaptureCompleteness::Complete,
                Some("incomplete") => CaptureCompleteness::Incomplete,
                _ => CaptureCompleteness::Unobserved,
            },
        })
    };
    Some(ScopedCommandReading {
        operation_id: detail["operation_id"].as_str()?.into(),
        work_revision: body["work_revision"].as_u64()?,
        analysis_snapshot_id: body["analysis_snapshot_id"].as_str()?.into(),
        repository_snapshot_id: body["repository_snapshot_id"].as_str()?.into(),
        repository_source_id: serde_json::from_value::<SourceId>(
            body["repository_source_id"].clone(),
        )
        .ok()?
        .to_string(),
        target_state: ExecutionTargetState::SelectedAnalysisOnlyNotRechecked,
        started_at_unix_micros: detail["started_at_unix_micros"].as_i64()?,
        ended_at_unix_micros: detail["ended_at_unix_micros"].as_i64()?,
        retained_until_unix_micros: metadata["retained_until_unix_micros"].as_i64()?,
        signal_number: detail["signal_number"].as_i64(),
        timeout_detected: detail["timeout_detected"].as_bool()?,
        cancellation_requested: detail["cancellation_requested"].as_bool()?,
        stdout: stream("stdout")?,
        stderr: stream("stderr")?,
    })
}
