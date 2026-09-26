use std::error::Error as StdError;
use std::fmt;

/// Stable categories callers can use without parsing storage diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    InvalidInput,
    NotFound,
    AlreadyExists,
    WrongProject,
    StaleBasis,
    DomainConflict,
    UnsupportedVersion,
    CorruptState,
    IntegrityFailure,
    StorageUnavailable,
    TransactionFailure,
    IndeterminateOutcome,
    RepairRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointDecisionWorkMismatch {
    pub checkpoint_work_item_id: Option<crate::ContextItemId>,
    pub decision_id: crate::DecisionId,
    pub decision_work_item_id: crate::ContextItemId,
}

/// A typed kernel failure with a bounded diagnostic.
#[derive(Debug)]
pub struct Error {
    checkpoint_decision_work_mismatch: Option<CheckpointDecisionWorkMismatch>,
    kind: ErrorKind,
    message: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            checkpoint_decision_work_mismatch: None,
            kind,
            message: message.into(),
            source: None,
        }
    }

    pub(crate) fn with_source(
        kind: ErrorKind,
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self {
            checkpoint_decision_work_mismatch: None,
            kind,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub(crate) fn checkpoint_work_mismatch(diagnostic: CheckpointDecisionWorkMismatch) -> Self {
        let mut error = Self::new(
            ErrorKind::InvalidInput,
            "Checkpoint cannot apply a Decision scoped to a different Work Item",
        );
        error.checkpoint_decision_work_mismatch = Some(diagnostic);
        error
    }

    pub fn checkpoint_decision_work_mismatch(&self) -> Option<&CheckpointDecisionWorkMismatch> {
        self.checkpoint_decision_work_mismatch.as_ref()
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn StdError + 'static))
    }
}

/// Shared canonical precondition; Store always rechecks inside its transaction.
pub fn validate_checkpoint_decision_work_scope(
    checkpoint_work_item_id: Option<crate::ContextItemId>,
    decision: &crate::Decision,
) -> Result<(), Error> {
    if let crate::DecisionWorkScope::WorkItem(decision_work_item_id) = decision.work_scope {
        if checkpoint_work_item_id != Some(decision_work_item_id) {
            return Err(Error::checkpoint_work_mismatch(
                CheckpointDecisionWorkMismatch {
                    checkpoint_work_item_id,
                    decision_id: decision.id,
                    decision_work_item_id,
                },
            ));
        }
    }
    Ok(())
}
