use std::{error::Error as StdError, fmt};

/// Analysis publication and its diagnostic receipt have separate outcomes.
/// Only closed identities/states cross CLI/MCP; the storage chain stays private.
#[derive(Debug)]
pub struct AnalysisReceiptFailure {
    pub project_id: volicord_context::ProjectId,
    pub operation_id: volicord_context::OperationId,
    pub primary_failed: bool,
    pub published_analysis_snapshot: Option<volicord_repository_intelligence::AnalysisSnapshotId>,
    pub published_operation_state: Option<crate::OperationState>,
    recording_error: Box<Error>,
}
impl AnalysisReceiptFailure {
    pub fn recording_error(&self) -> &Error {
        &self.recording_error
    }
    pub(crate) fn new(
        project_id: volicord_context::ProjectId,
        operation_id: volicord_context::OperationId,
        primary_failed: bool,
        published_analysis_snapshot: Option<volicord_repository_intelligence::AnalysisSnapshotId>,
        published_operation_state: Option<crate::OperationState>,
        recording_error: Error,
    ) -> Self {
        Self {
            project_id,
            operation_id,
            primary_failed,
            published_analysis_snapshot,
            published_operation_state,
            recording_error: Box::new(recording_error),
        }
    }
}

#[derive(Debug)]
pub struct Error {
    message: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
    checkpoint_scope_violation: Option<Box<crate::CheckpointScopeViolation>>,
    reading_failure: Option<volicord_projections::ExplanationFailureReason>,
    publication_readable: bool,
    analysis_receipt_failure: Option<Box<AnalysisReceiptFailure>>,
}

impl Error {
    /// Only for decoding the caller's explicit explanation input. Do not expose
    /// storage/provider error chains through this authoring diagnostic.
    pub fn explanation_input(context: &str, source: serde_json::Error) -> Self {
        let diagnostic = source
            .to_string()
            .escape_debug()
            .take(512)
            .collect::<String>();
        Self::with_source(format!("{context}: {diagnostic}"), source)
    }

    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
            checkpoint_scope_violation: None,
            reading_failure: None,
            publication_readable: false,
            analysis_receipt_failure: None,
        }
    }

    pub fn with_source(
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(source)),
            checkpoint_scope_violation: None,
            reading_failure: None,
            publication_readable: false,
            analysis_receipt_failure: None,
        }
    }

    /// Preserve only the bounded canonical diagnostic, never storage source details.
    pub fn canonical_cause(&self) -> Option<&volicord_context::Error> {
        let mut source = self.source();
        for _ in 0..8 {
            let current = source?;
            if let Some(canonical) = current.downcast_ref::<volicord_context::Error>() {
                return Some(canonical);
            }
            source = current.source();
        }
        None
    }

    pub fn inquiry_cause(&self) -> Option<&volicord_inquiry::Error> {
        let mut source = self.source();
        for _ in 0..8 {
            let current = source?;
            if let Some(inquiry) = current.downcast_ref::<volicord_inquiry::Error>() {
                return Some(inquiry);
            }
            source = current.source();
        }
        None
    }

    pub fn work_selection_cause(&self) -> Option<&volicord_projections::WorkSelectionError> {
        self.source
            .as_deref()?
            .downcast_ref::<volicord_projections::WorkSelectionError>()
    }

    pub(crate) fn with_analysis_receipt_failure(mut self, failure: AnalysisReceiptFailure) -> Self {
        self.analysis_receipt_failure = Some(Box::new(failure));
        self
    }
    pub fn analysis_receipt_failure(&self) -> Option<&AnalysisReceiptFailure> {
        self.analysis_receipt_failure.as_deref()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn reading_failure(&self) -> Option<volicord_projections::ExplanationFailureReason> {
        self.reading_failure.or_else(|| {
            self.source
                .as_deref()
                .and_then(|s| s.downcast_ref::<Self>())
                .and_then(Self::reading_failure)
        })
    }

    pub(crate) fn with_reading_failure(
        mut self,
        reason: volicord_projections::ExplanationFailureReason,
    ) -> Self {
        self.reading_failure = Some(reason);
        self
    }

    pub fn publication_readable(&self) -> bool {
        self.publication_readable
    }

    pub(crate) fn with_readable_publication(mut self) -> Self {
        self.publication_readable = true;
        self
    }

    pub fn checkpoint_scope_violation(&self) -> Option<&crate::CheckpointScopeViolation> {
        self.checkpoint_scope_violation.as_deref()
    }

    pub(crate) fn with_checkpoint_scope_violation(
        message: impl Into<String>,
        violation: crate::CheckpointScopeViolation,
    ) -> Self {
        Self {
            message: message.into(),
            source: None,
            checkpoint_scope_violation: Some(Box::new(violation)),
            reading_failure: None,
            publication_readable: false,
            analysis_receipt_failure: None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn StdError + 'static))
    }
}
