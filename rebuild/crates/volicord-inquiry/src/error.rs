use std::error::Error as StdError;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    InvalidInput,
    NotFound,
    WrongProject,
    CollectionDisabled,
    DomainConflict,
    StaleBasis,
    UnsupportedVersion,
    CorruptState,
    StorageUnavailable,
    TransactionFailure,
    CanonicalFailure,
}

/// Typed authoring location. Dimension fields remain domain-owned; adapters translate them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoringLocation {
    pub field_path: String,
    pub choice_id: Option<String>,
}

#[derive(Debug)]
pub struct Error {
    authoring_location: Option<AuthoringLocation>,
    kind: ErrorKind,
    message: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            authoring_location: None,
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
            authoring_location: None,
            kind,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub(crate) fn at_authoring_field(
        mut self,
        field_path: impl Into<String>,
        choice_id: Option<&str>,
    ) -> Self {
        if self.authoring_location.is_none() {
            self.authoring_location = Some(AuthoringLocation {
                field_path: field_path.into(),
                choice_id: choice_id.map(ToOwned::to_owned),
            });
        }
        self
    }

    pub fn authoring_location(&self) -> Option<&AuthoringLocation> {
        self.authoring_location.as_ref()
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
