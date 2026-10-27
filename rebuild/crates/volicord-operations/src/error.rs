use std::{error::Error as StdError, fmt};

#[derive(Debug)]
pub struct Error {
    message: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
    checkpoint_scope_violation: Option<Box<crate::CheckpointScopeViolation>>,
    reading_failure: Option<volicord_projections::ExplanationFailureReason>,
    publication_readable: bool,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
            checkpoint_scope_violation: None,
            reading_failure: None,
            publication_readable: false,
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
