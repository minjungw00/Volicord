use std::{error::Error as StdError, fmt};

#[derive(Debug)]
pub struct Error {
    message: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
    checkpoint_scope_violation: Option<crate::CheckpointScopeViolation>,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
            checkpoint_scope_violation: None,
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
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn checkpoint_scope_violation(&self) -> Option<&crate::CheckpointScopeViolation> {
        self.checkpoint_scope_violation.as_ref()
    }

    pub(crate) fn with_checkpoint_scope_violation(
        message: impl Into<String>,
        violation: crate::CheckpointScopeViolation,
    ) -> Self {
        Self {
            message: message.into(),
            source: None,
            checkpoint_scope_violation: Some(violation),
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
