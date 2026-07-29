use std::{collections::BTreeSet, error::Error, fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{de, Deserialize, Deserializer, Serialize};

/// A normalized, platform-neutral path relative to one Product Repository.
///
/// This value establishes lexical validity only. It does not establish that a
/// path exists or remains contained by a repository after filesystem links are
/// resolved.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct ProductRelativePath(String);

impl ProductRelativePath {
    /// Parses one repository-relative slash-separated UTF-8 path.
    pub fn parse(value: impl Into<String>) -> Result<Self, ProductPathError> {
        let value = value.into();
        validate_product_relative_path(&value)?;
        Ok(Self(value))
    }

    /// Returns the normalized repository-relative text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the value and returns its normalized repository-relative text.
    pub fn into_string(self) -> String {
        self.0
    }

    /// Returns whether this path is equal to or nested beneath `scope`.
    pub fn is_within(&self, scope: &Self) -> bool {
        self == scope
            || self
                .as_str()
                .strip_prefix(scope.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
    }
}

/// Immutable allowed and denied path scope for one Write Ticket.
///
/// Every path is already a canonical [`ProductRelativePath`]. Construction
/// additionally guarantees set uniqueness and allowed/denied disjointness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketPathScope {
    allowed: Vec<ProductRelativePath>,
    denied: Vec<ProductRelativePath>,
}

impl WriteTicketPathScope {
    /// Constructs one invariant-bearing Write Ticket path scope.
    pub fn new(
        allowed: Vec<ProductRelativePath>,
        denied: Vec<ProductRelativePath>,
    ) -> Result<Self, WriteTicketPathScopeError> {
        let allowed_set = allowed.iter().collect::<BTreeSet<_>>();
        if allowed_set.len() != allowed.len() {
            return Err(WriteTicketPathScopeError::DuplicateAllowedPath);
        }
        let denied_set = denied.iter().collect::<BTreeSet<_>>();
        if denied_set.len() != denied.len() {
            return Err(WriteTicketPathScopeError::DuplicateDeniedPath);
        }
        if allowed.iter().any(|allowed_path| {
            denied.iter().any(|denied_path| {
                allowed_path.is_within(denied_path) || denied_path.is_within(allowed_path)
            })
        }) {
            return Err(WriteTicketPathScopeError::AllowedDeniedOverlap);
        }
        Ok(Self { allowed, denied })
    }

    /// Returns the canonical paths allowed by this scope.
    pub fn allowed(&self) -> &[ProductRelativePath] {
        &self.allowed
    }

    /// Returns the canonical paths denied by this scope.
    pub fn denied(&self) -> &[ProductRelativePath] {
        &self.denied
    }
}

/// Construction failures for an immutable Write Ticket path scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteTicketPathScopeError {
    DuplicateAllowedPath,
    DuplicateDeniedPath,
    AllowedDeniedOverlap,
}

impl fmt::Display for WriteTicketPathScopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::DuplicateAllowedPath => "duplicate allowed Write Ticket path",
            Self::DuplicateDeniedPath => "duplicate denied Write Ticket path",
            Self::AllowedDeniedOverlap => "overlapping allowed and denied Write Ticket paths",
        })
    }
}

impl Error for WriteTicketPathScopeError {}

impl fmt::Display for ProductRelativePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ProductRelativePath {
    type Err = ProductPathError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl<'de> Deserialize<'de> for ProductRelativePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(de::Error::custom)
    }
}

/// Lexical validation failures for a Product Repository relative path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPathError {
    Empty,
    Absolute,
    PlatformPrefix,
    Backslash,
    EmptyComponent,
    CurrentDirectory,
    ParentTraversal,
    Nul,
}

impl ProductPathError {
    /// Returns the stable implementation-facing reason.
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Empty => "product_path_empty",
            Self::Absolute => "product_path_absolute",
            Self::PlatformPrefix => "product_path_platform_prefix",
            Self::Backslash => "product_path_backslash",
            Self::EmptyComponent => "product_path_empty_component",
            Self::CurrentDirectory => "product_path_current_directory_component",
            Self::ParentTraversal => "product_path_parent_traversal",
            Self::Nul => "product_path_nul",
        }
    }
}

impl fmt::Display for ProductPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}

impl Error for ProductPathError {}

/// Parses a collection of platform-neutral Product Repository relative paths.
pub fn parse_product_paths(
    raw_paths: &[String],
) -> Result<Vec<ProductRelativePath>, ProductPathError> {
    raw_paths
        .iter()
        .map(|path| ProductRelativePath::parse(path.clone()))
        .collect()
}

/// Returns whether two lexically valid Product Repository paths have a
/// containment relationship.
pub fn path_is_within(path: &str, scope: &str) -> bool {
    let (Ok(path), Ok(scope)) = (
        ProductRelativePath::parse(path),
        ProductRelativePath::parse(scope),
    ) else {
        return false;
    };
    path.is_within(&scope)
}

/// Returns whether every observed path is covered by at least one authorized
/// path.
pub fn paths_are_authorized(observed_paths: &[String], authorized_paths: &[String]) -> bool {
    !observed_paths.is_empty()
        && !authorized_paths.is_empty()
        && observed_paths.iter().all(|path| {
            authorized_paths
                .iter()
                .any(|authorized| path_is_within(path, authorized))
        })
}

fn validate_product_relative_path(value: &str) -> Result<(), ProductPathError> {
    if value.is_empty() || value.trim().is_empty() {
        return Err(ProductPathError::Empty);
    }
    if value.as_bytes().contains(&0) {
        return Err(ProductPathError::Nul);
    }
    if value.contains('\\') {
        return Err(ProductPathError::Backslash);
    }
    if value.starts_with('/') {
        return Err(ProductPathError::Absolute);
    }
    if has_windows_drive_prefix(value) {
        return Err(ProductPathError::PlatformPrefix);
    }

    for component in value.split('/') {
        match component {
            "" => return Err(ProductPathError::EmptyComponent),
            "." => return Err(ProductPathError::CurrentDirectory),
            ".." => return Err(ProductPathError::ParentTraversal),
            _ => {}
        }
    }
    Ok(())
}

fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normalized_relative_paths_without_filesystem_access() {
        let path = ProductRelativePath::parse("src/domain/model.rs").expect("relative path");
        assert_eq!(path.as_str(), "src/domain/model.rs");
    }

    #[test]
    fn rejects_non_relative_or_non_normalized_lexical_forms() {
        for (value, expected) in [
            ("", ProductPathError::Empty),
            (" ", ProductPathError::Empty),
            ("/src/lib.rs", ProductPathError::Absolute),
            ("C:/src/lib.rs", ProductPathError::PlatformPrefix),
            ("src\\lib.rs", ProductPathError::Backslash),
            ("src//lib.rs", ProductPathError::EmptyComponent),
            ("src/./lib.rs", ProductPathError::CurrentDirectory),
            ("src/../lib.rs", ProductPathError::ParentTraversal),
            ("src/\0/lib.rs", ProductPathError::Nul),
        ] {
            assert_eq!(
                ProductRelativePath::parse(value),
                Err(expected),
                "{value:?}"
            );
        }
    }

    #[test]
    fn compares_only_valid_normalized_components() {
        let path = ProductRelativePath::parse("src/domain/model.rs").expect("path");
        let scope = ProductRelativePath::parse("src/domain").expect("scope");
        let sibling = ProductRelativePath::parse("src/domains").expect("sibling");

        assert!(path.is_within(&scope));
        assert!(!path.is_within(&sibling));
        assert!(!path_is_within("src/./domain/model.rs", "src/domain"));
    }

    #[test]
    fn write_ticket_path_scope_preserves_unique_disjoint_typed_paths() {
        let allowed = ProductRelativePath::parse("src").expect("allowed path");
        let denied = ProductRelativePath::parse("tests").expect("denied path");
        let scope = WriteTicketPathScope::new(vec![allowed.clone()], vec![denied.clone()])
            .expect("disjoint scope");

        assert_eq!(scope.allowed(), &[allowed]);
        assert_eq!(scope.denied(), &[denied]);
    }

    #[test]
    fn write_ticket_path_scope_rejects_duplicates_and_containment_overlap() {
        let src = ProductRelativePath::parse("src").expect("source path");
        let nested = ProductRelativePath::parse("src/private").expect("nested path");

        assert_eq!(
            WriteTicketPathScope::new(vec![src.clone(), src.clone()], Vec::new()),
            Err(WriteTicketPathScopeError::DuplicateAllowedPath)
        );
        assert_eq!(
            WriteTicketPathScope::new(Vec::new(), vec![src.clone(), src.clone()]),
            Err(WriteTicketPathScopeError::DuplicateDeniedPath)
        );
        assert_eq!(
            WriteTicketPathScope::new(vec![src], vec![nested]),
            Err(WriteTicketPathScopeError::AllowedDeniedOverlap)
        );
    }
}
