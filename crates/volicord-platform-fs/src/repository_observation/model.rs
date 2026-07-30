use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    path::PathBuf,
};

use serde::Serialize;
use sha2::{Digest, Sha256};
use volicord_types::product_path::ProductRelativePath;

use super::bounded::ObserverLimits;

pub(crate) const SNAPSHOT_SERIALIZATION_DEPTH: usize = 4;
const OBSERVER_CONTRACT_NAME: &str = "volicord.product-repository-observer";
const OBSERVER_CONTRACT_REVISION: &str = "net-path-state";
const MAX_UNAVAILABLE_DETAIL_BYTES: usize = 512;

/// Content-derived SHA-256 identity for regular-file bytes or a link target.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ContentIdentity(String);

impl ContentIdentity {
    pub(crate) fn from_digest(digest: impl fmt::LowerHex) -> Self {
        Self(format!("sha256:{digest:x}"))
    }

    pub(crate) fn for_bytes(bytes: &[u8]) -> Self {
        Self::from_digest(Sha256::digest(bytes))
    }

    /// Returns the canonical algorithm-qualified identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Effective observable state of one Product Repository path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProductPathState {
    /// No filesystem entry exists at the path.
    Absent,
    /// A regular file with deterministic content and executable-mode identity.
    RegularFile {
        content: ContentIdentity,
        executable: bool,
    },
    /// A symbolic link identified by its exact UTF-8 target bytes.
    SymbolicLink { target: ContentIdentity },
    /// A clean initialized Gitlink identified by the checked-out commit.
    Gitlink { commit_oid: String },
}

/// Exact typed paths supplied for one tool invocation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InvocationObservationPaths {
    target_paths: Vec<ProductRelativePath>,
    changed_path_hints: Vec<ProductRelativePath>,
}

impl InvocationObservationPaths {
    /// Constructs typed exact invocation paths and reviewed changed-path hints.
    pub fn new(
        target_paths: Vec<ProductRelativePath>,
        changed_path_hints: Vec<ProductRelativePath>,
    ) -> Self {
        Self {
            target_paths,
            changed_path_hints,
        }
    }

    /// Exact paths named as invocation targets.
    pub fn target_paths(&self) -> &[ProductRelativePath] {
        &self.target_paths
    }

    /// Exact invocation-scoped paths supplied by a reviewed host contract.
    pub fn changed_path_hints(&self) -> &[ProductRelativePath] {
        &self.changed_path_hints
    }

    pub(crate) fn canonical_set(&self) -> BTreeSet<ProductRelativePath> {
        self.target_paths
            .iter()
            .chain(&self.changed_path_hints)
            .cloned()
            .collect()
    }
}

/// Opaque digest of the observer semantics and all active resource limits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct SemanticObserverContractDigest(String);

impl SemanticObserverContractDigest {
    pub(crate) fn for_limits(limits: &ObserverLimits) -> Self {
        let mut encoder = CanonicalEncoder::new();
        encoder.string(OBSERVER_CONTRACT_NAME);
        encoder.string(OBSERVER_CONTRACT_REVISION);
        encoder.usize(limits.max_git_output_bytes());
        encoder.usize(limits.max_process_input_bytes());
        encoder.usize(limits.max_candidate_paths());
        encoder.u64(limits.max_total_hashed_bytes());
        encoder.u64(limits.max_file_bytes());
        encoder.u128(limits.max_process_duration().as_nanos());
        encoder.usize(limits.max_serialized_bytes());
        encoder.usize(limits.max_serialization_depth());
        encoder.usize(limits.max_stability_attempts());
        Self(ContentIdentity::for_bytes(&encoder.finish()).0)
    }

    /// Returns the canonical algorithm-qualified digest.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable Git and repository coordinates surrounding one snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepositoryObservationCoordinate {
    repository_identity: String,
    git_layout_identity: String,
    worktree_identity: String,
    head_oid: Option<String>,
    tree_oid: Option<String>,
    status_identity: String,
}

impl RepositoryObservationCoordinate {
    pub(crate) fn new(
        repository_identity: String,
        git_layout_identity: String,
        worktree_identity: String,
        head_oid: Option<String>,
        tree_oid: Option<String>,
        status_identity: String,
    ) -> Self {
        Self {
            repository_identity,
            git_layout_identity,
            worktree_identity,
            head_oid,
            tree_oid,
            status_identity,
        }
    }

    /// Opaque identity for the canonical Product Repository and Git common directory.
    pub fn repository_identity(&self) -> &str {
        &self.repository_identity
    }

    /// Opaque identity for the complete canonical Git layout.
    pub fn git_layout_identity(&self) -> &str {
        &self.git_layout_identity
    }

    /// Opaque identity for the selected Git worktree.
    pub fn worktree_identity(&self) -> &str {
        &self.worktree_identity
    }

    /// Full HEAD object ID, or `None` for an unborn repository.
    pub fn head_oid(&self) -> Option<&str> {
        self.head_oid.as_deref()
    }

    /// Full HEAD tree object ID, or `None` for an unborn repository.
    pub fn tree_oid(&self) -> Option<&str> {
        self.tree_oid.as_deref()
    }

    /// Opaque digest of the complete porcelain status coordinate.
    pub fn status_identity(&self) -> &str {
        &self.status_identity
    }
}

/// Stable invocation-scoped repository observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryObservationSnapshot {
    pub(crate) repository_root: PathBuf,
    pub(crate) coordinate: RepositoryObservationCoordinate,
    pub(crate) observed_states: BTreeMap<ProductRelativePath, ProductPathState>,
    pub(crate) status_paths: BTreeSet<ProductRelativePath>,
    pub(crate) invocation_paths: BTreeSet<ProductRelativePath>,
    pub(crate) contract_digest: SemanticObserverContractDigest,
    pub(crate) limits: ObserverLimits,
}

impl RepositoryObservationSnapshot {
    /// Stable repository coordinate observed around this snapshot.
    pub fn coordinate(&self) -> &RepositoryObservationCoordinate {
        &self.coordinate
    }

    /// Actual states retained for dirty, untracked, and typed invocation paths.
    pub fn observed_states(&self) -> &BTreeMap<ProductRelativePath, ProductPathState> {
        &self.observed_states
    }

    /// Observer contract and limit digest used to construct this snapshot.
    pub fn contract_digest(&self) -> &SemanticObserverContractDigest {
        &self.contract_digest
    }

    /// Deterministic bounded canonical bytes for this snapshot.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ObservationUnavailable> {
        if self.limits.max_serialization_depth() < SNAPSHOT_SERIALIZATION_DEPTH {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::SerializationDepthLimitExceeded,
                "the configured serialization depth cannot represent a repository snapshot",
            ));
        }
        let mut encoder = CanonicalEncoder::new();
        encoder.string("repository_snapshot");
        encoder.string(self.contract_digest.as_str());
        encode_coordinate(&mut encoder, &self.coordinate);
        encoder.usize(self.observed_states.len());
        for (path, state) in &self.observed_states {
            encoder.string(path.as_str());
            encode_path_state(&mut encoder, state);
        }
        encoder.usize(self.status_paths.len());
        for path in &self.status_paths {
            encoder.string(path.as_str());
        }
        encoder.usize(self.invocation_paths.len());
        for path in &self.invocation_paths {
            encoder.string(path.as_str());
        }
        let bytes = encoder.finish();
        if bytes.len() > self.limits.max_serialized_bytes() {
            return Err(ObservationUnavailable::new(
                ObservationUnavailableReason::SerializationSizeLimitExceeded,
                "the canonical repository snapshot exceeds its serialization limit",
            ));
        }
        Ok(bytes)
    }

    /// Deterministic content identity of [`Self::canonical_bytes`].
    pub fn semantic_digest(&self) -> Result<ContentIdentity, ObservationUnavailable> {
        Ok(ContentIdentity::for_bytes(&self.canonical_bytes()?))
    }
}

/// One exact before/after transition for a Product Repository path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepositoryPathTransition {
    path: ProductRelativePath,
    before: ProductPathState,
    after: ProductPathState,
}

impl RepositoryPathTransition {
    pub(crate) fn new(
        path: ProductRelativePath,
        before: ProductPathState,
        after: ProductPathState,
    ) -> Self {
        Self {
            path,
            before,
            after,
        }
    }

    /// Product Repository-relative path that transitioned.
    pub fn path(&self) -> &ProductRelativePath {
        &self.path
    }

    /// Effective path state before the invocation.
    pub fn before(&self) -> &ProductPathState {
        &self.before
    }

    /// Effective path state after the invocation.
    pub fn after(&self) -> &ProductPathState {
        &self.after
    }
}

/// Deterministically ordered net Product Repository delta.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RepositoryDelta {
    transitions: Vec<RepositoryPathTransition>,
}

impl RepositoryDelta {
    pub(crate) fn new(transitions: Vec<RepositoryPathTransition>) -> Self {
        Self { transitions }
    }

    /// Sorted path transitions in canonical bytewise path order.
    pub fn transitions(&self) -> &[RepositoryPathTransition] {
        &self.transitions
    }

    /// Returns whether the effective Product Repository state is unchanged.
    pub fn is_empty(&self) -> bool {
        self.transitions.is_empty()
    }

    /// Deterministic canonical bytes for this delta.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut encoder = CanonicalEncoder::new();
        encoder.string("repository_delta");
        encoder.usize(self.transitions.len());
        for transition in &self.transitions {
            encoder.string(transition.path.as_str());
            encode_path_state(&mut encoder, &transition.before);
            encode_path_state(&mut encoder, &transition.after);
        }
        encoder.finish()
    }

    /// Deterministic content identity for this delta.
    pub fn digest(&self) -> ContentIdentity {
        ContentIdentity::for_bytes(&self.canonical_bytes())
    }
}

/// Closed reason exact repository observation could not be completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationUnavailableReason {
    InvalidObserverLimits,
    InvalidRepositoryRoot,
    NotGitRepository,
    GitLayoutUnavailable,
    GitCommandUnavailable,
    GitCommandFailed,
    ProcessTimeout,
    GitOutputLimitExceeded,
    ProcessInputLimitExceeded,
    CandidatePathLimitExceeded,
    TotalHashBytesLimitExceeded,
    FileSizeLimitExceeded,
    SerializationDepthLimitExceeded,
    SerializationSizeLimitExceeded,
    InvalidRelativePath,
    NonUtf8Path,
    PathOutsideRepository,
    InaccessiblePath,
    UnsupportedPathState,
    UnstableRepository,
    RepositoryIdentityChanged,
    ObserverContractMismatch,
    GitObjectUnavailable,
}

impl ObservationUnavailableReason {
    /// Stable implementation-facing reason.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidObserverLimits => "invalid_observer_limits",
            Self::InvalidRepositoryRoot => "invalid_repository_root",
            Self::NotGitRepository => "not_git_repository",
            Self::GitLayoutUnavailable => "git_layout_unavailable",
            Self::GitCommandUnavailable => "git_command_unavailable",
            Self::GitCommandFailed => "git_command_failed",
            Self::ProcessTimeout => "process_timeout",
            Self::GitOutputLimitExceeded => "git_output_limit_exceeded",
            Self::ProcessInputLimitExceeded => "process_input_limit_exceeded",
            Self::CandidatePathLimitExceeded => "candidate_path_limit_exceeded",
            Self::TotalHashBytesLimitExceeded => "total_hash_bytes_limit_exceeded",
            Self::FileSizeLimitExceeded => "file_size_limit_exceeded",
            Self::SerializationDepthLimitExceeded => "serialization_depth_limit_exceeded",
            Self::SerializationSizeLimitExceeded => "serialization_size_limit_exceeded",
            Self::InvalidRelativePath => "invalid_relative_path",
            Self::NonUtf8Path => "non_utf8_path",
            Self::PathOutsideRepository => "path_outside_repository",
            Self::InaccessiblePath => "inaccessible_path",
            Self::UnsupportedPathState => "unsupported_path_state",
            Self::UnstableRepository => "unstable_repository",
            Self::RepositoryIdentityChanged => "repository_identity_changed",
            Self::ObserverContractMismatch => "observer_contract_mismatch",
            Self::GitObjectUnavailable => "git_object_unavailable",
        }
    }
}

/// Typed unavailable outcome with bounded diagnostic detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationUnavailable {
    reason: ObservationUnavailableReason,
    detail: String,
}

impl ObservationUnavailable {
    pub(crate) fn new(reason: ObservationUnavailableReason, detail: impl Into<String>) -> Self {
        Self {
            reason,
            detail: bound_detail(detail.into()),
        }
    }

    /// Closed semantic reason for the unavailable observation.
    pub const fn reason(&self) -> ObservationUnavailableReason {
        self.reason
    }

    /// Bounded implementation detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for ObservationUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.reason.as_str(), self.detail)
    }
}

impl Error for ObservationUnavailable {}

pub(crate) fn ensure_candidate_limit(
    count: usize,
    limits: &ObserverLimits,
) -> Result<(), ObservationUnavailable> {
    if count > limits.max_candidate_paths() {
        return Err(ObservationUnavailable::new(
            ObservationUnavailableReason::CandidatePathLimitExceeded,
            "the semantic candidate path union exceeds its configured limit",
        ));
    }
    Ok(())
}

pub(crate) fn hash_fields(fields: &[&str]) -> String {
    let mut encoder = CanonicalEncoder::new();
    for field in fields {
        encoder.string(field);
    }
    ContentIdentity::for_bytes(&encoder.finish()).0
}

fn encode_coordinate(encoder: &mut CanonicalEncoder, coordinate: &RepositoryObservationCoordinate) {
    encoder.string(&coordinate.repository_identity);
    encoder.string(&coordinate.git_layout_identity);
    encoder.string(&coordinate.worktree_identity);
    encoder.optional_string(coordinate.head_oid.as_deref());
    encoder.optional_string(coordinate.tree_oid.as_deref());
    encoder.string(&coordinate.status_identity);
}

fn encode_path_state(encoder: &mut CanonicalEncoder, state: &ProductPathState) {
    match state {
        ProductPathState::Absent => encoder.u8(0),
        ProductPathState::RegularFile {
            content,
            executable,
        } => {
            encoder.u8(1);
            encoder.string(content.as_str());
            encoder.u8(u8::from(*executable));
        }
        ProductPathState::SymbolicLink { target } => {
            encoder.u8(2);
            encoder.string(target.as_str());
        }
        ProductPathState::Gitlink { commit_oid } => {
            encoder.u8(3);
            encoder.string(commit_oid);
        }
    }
}

fn bound_detail(mut detail: String) -> String {
    if detail.len() <= MAX_UNAVAILABLE_DETAIL_BYTES {
        return detail;
    }
    let suffix = "...[truncated]";
    let mut end = MAX_UNAVAILABLE_DETAIL_BYTES.saturating_sub(suffix.len());
    while !detail.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    detail.truncate(end);
    detail.push_str(suffix);
    detail
}

pub(crate) struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    pub(crate) fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn usize(&mut self, value: usize) {
        self.u64(value as u64);
    }

    pub(crate) fn string(&mut self, value: &str) {
        self.usize(value.len());
        self.bytes.extend_from_slice(value.as_bytes());
    }

    pub(crate) fn optional_string(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.u8(1);
                self.string(value);
            }
            None => self.u8(0),
        }
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}
