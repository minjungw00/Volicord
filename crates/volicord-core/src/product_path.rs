use std::{error::Error, fmt, path::Path};

use volicord_platform_fs::{
    ObservedProductRepository, PlatformBoundaryError, PlatformDiagnosticClass,
};
use volicord_types::product_path::{parse_product_paths, ProductPathError};

/// A Product Repository path failure separated by semantic and platform owner.
#[derive(Debug)]
pub(crate) enum ProductPathValidationError {
    Lexical(ProductPathError),
    Platform(PlatformBoundaryError),
}

impl ProductPathValidationError {
    pub(crate) const fn platform_class(&self) -> Option<PlatformDiagnosticClass> {
        match self {
            Self::Lexical(_) => None,
            Self::Platform(error) => Some(error.class()),
        }
    }
}

impl fmt::Display for ProductPathValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lexical(error) => write!(formatter, "invalid Product Repository path: {error}"),
            Self::Platform(error) => error.fmt(formatter),
        }
    }
}

impl Error for ProductPathValidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Lexical(error) => Some(error),
            Self::Platform(error) => Some(error),
        }
    }
}

/// Parses paths through the shared semantic owner, observes them through the
/// platform owner, and returns only the verified semantic identities.
pub(crate) fn observe_product_paths(
    repository_root: &Path,
    raw_paths: &[String],
) -> Result<Vec<String>, ProductPathValidationError> {
    let paths = parse_product_paths(raw_paths).map_err(ProductPathValidationError::Lexical)?;
    let repository = ObservedProductRepository::observe(repository_root)
        .map_err(ProductPathValidationError::Platform)?;
    paths
        .into_iter()
        .map(|path| {
            repository
                .observe_path(path)
                .map(|observed| observed.into_relative_path().into_string())
                .map_err(ProductPathValidationError::Platform)
        })
        .collect()
}
