//! Local operation receipts, separate from canonical context and analysis graphs.
use crate::{AnalysisOutcome, Error, LocalOperations, LongOperationResult};
use serde::{Deserialize, Serialize};
use std::{fs, io::Read, path::PathBuf};
use volicord_context::{OperationId, ProjectId};
use volicord_projections::{AnalysisAttemptReading, RepositoryAnalysisReading};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AttemptReceipt {
    format_kind: String,
    format_version: u32,
    project_id: String,
    attempt: AnalysisAttemptReading,
}

impl LocalOperations {
    fn attempt_dir(&self, project: ProjectId) -> PathBuf {
        self.layout()
            .artifacts_dir()
            .join("analysis-attempts")
            .join(project.to_string())
    }

    pub(crate) fn record_analysis_attempt(
        &self,
        project: ProjectId,
        operation: OperationId,
        result: &Result<LongOperationResult<AnalysisOutcome>, Error>,
    ) -> Result<(), Error> {
        let completed = crate::operations::now_micros()?.as_unix_micros();
        let receipt = AttemptReceipt {
            format_kind: "volicord_analysis_attempt".into(),
            format_version: 1,
            project_id: project.to_string(),
            attempt: AnalysisAttemptReading {
                operation_id: operation.to_string(),
                completed_at_unix_micros: completed,
                failed: result.is_err(),
                diagnostic: result
                    .as_ref()
                    .err()
                    .map(|e| e.message().chars().take(1024).collect()),
            },
        };
        self.layout().prepare_private_paths()?;
        let dir = self.attempt_dir(project);
        volicord_local_platform::ensure_private_directory(&dir)
            .map_err(|e| Error::with_source("cannot prepare analysis attempt receipts", e))?;
        let bytes = serde_json::to_vec(&receipt)
            .map_err(|e| Error::with_source("cannot encode analysis attempt receipt", e))?;
        crate::operations::publish_bytes_no_replace(
            &dir.join(format!("{operation}.json")),
            &bytes,
        )?;
        Ok(())
    }

    pub(crate) fn attach_analysis_attempt(
        &self,
        project: ProjectId,
        status: &mut RepositoryAnalysisReading,
    ) {
        match self.latest_analysis_attempt(project) {
            Ok(Some(attempt)) => status.observe_attempt(attempt),
            Ok(None) => {}
            Err(error) => status.latest_attempt_error = Some(error.message().into()),
        }
    }

    fn latest_analysis_attempt(
        &self,
        project: ProjectId,
    ) -> Result<Option<AnalysisAttemptReading>, Error> {
        let dir = match fs::read_dir(self.attempt_dir(project)) {
            Ok(dir) => dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(Error::with_source(
                    "analysis attempt history unavailable",
                    error,
                ))
            }
        };
        let mut latest: Option<AnalysisAttemptReading> = None;
        for entry in dir {
            let entry =
                entry.map_err(|e| Error::with_source("analysis attempt history unreadable", e))?;
            if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let metadata = entry
                .metadata()
                .map_err(|e| Error::with_source("analysis attempt receipt unreadable", e))?;
            if !metadata.is_file() || metadata.len() > 8192 {
                return Err(Error::new(
                    "analysis attempt receipt is invalid or oversized",
                ));
            }
            let mut bytes = Vec::new();
            fs::File::open(entry.path())
                .and_then(|file| file.take(8193).read_to_end(&mut bytes))
                .map_err(|e| Error::with_source("analysis attempt receipt unreadable", e))?;
            if bytes.len() > 8192 {
                return Err(Error::new("analysis attempt receipt is oversized"));
            }
            let value: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| Error::with_source("analysis attempt receipt corrupt", e))?;
            if value["format_kind"] != "volicord_analysis_attempt" || value["format_version"] != 1 {
                return Err(Error::new(
                    "analysis attempt receipt format unsupported; latest attempt unknown",
                ));
            }
            let receipt: AttemptReceipt = serde_json::from_value(value)
                .map_err(|e| Error::with_source("analysis attempt receipt corrupt", e))?;
            if receipt.project_id != project.to_string()
                || entry.path().file_stem().and_then(|s| s.to_str())
                    != Some(receipt.attempt.operation_id.as_str())
            {
                return Err(Error::new("analysis attempt receipt identity mismatch"));
            }
            if latest.as_ref().is_none_or(|a| {
                (a.completed_at_unix_micros, &a.operation_id)
                    < (
                        receipt.attempt.completed_at_unix_micros,
                        &receipt.attempt.operation_id,
                    )
            }) {
                latest = Some(receipt.attempt);
            }
        }
        Ok(latest)
    }
}
