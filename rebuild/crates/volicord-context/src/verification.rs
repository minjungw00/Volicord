//! Explicit check semantics retained inside the original Checkpoint, never inferred
//! from command labels or process exit. Raw invocation/output remains disposable.
use crate::{CheckpointId, ContextItemId, Error, ErrorKind, SourceId, VerificationState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub struct CheckDeclaration {
    pub key: String,
    pub purpose: String,
    pub assertion: String,
    /// Declared bounded coverage, not measured test coverage or whole-tree proof.
    pub coverage: Option<String>,
    /// Explicit non-secret operator description, never an environment dump.
    pub environment: Option<String>,
    pub target_paths: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub struct CheckArtifact {
    pub path: String,
    pub sha256: String,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AssertionState {
    NotObserved,
    Passed,
    Failed,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AssertionStream {
    Stdout,
    Stderr,
}
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub struct AssertionEvidence {
    pub stream: AssertionStream,
    pub start_byte: usize,
    pub end_byte: usize,
    pub sha256: String,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CheckTargetRelation {
    SameArtifacts,
    ChangedArtifacts,
}
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub struct CheckRetestBasis {
    #[serde(with = "check_checkpoint_id")]
    pub checkpoint_id: CheckpointId,
    pub checkpoint_revision: u64,
    pub verification_index: usize,
    #[serde(with = "check_source_id")]
    pub source_id: SourceId,
    pub target_relation: CheckTargetRelation,
    pub rationale: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub struct ScopedCheckRun {
    #[serde(with = "check_work_id")]
    pub work_item_id: ContextItemId,
    pub work_revision: u64,
    pub declaration: CheckDeclaration,
    pub operation_id: String,
    pub invocation_fingerprint: String,
    pub analysis_snapshot_id: String,
    pub repository_snapshot_id: String,
    #[serde(with = "check_source_id")]
    pub repository_source_id: SourceId,
    pub started_at_unix_micros: i64,
    pub ended_at_unix_micros: i64,
    pub artifacts: Vec<CheckArtifact>,
    /// End-point samples agree; not an atomic snapshot or intervening-write proof.
    pub target_unchanged_during_run: bool,
    pub assertion_state: AssertionState,
    pub assertion_evidence: Option<AssertionEvidence>,
    pub retest: Option<CheckRetestBasis>,
}
impl CheckDeclaration {
    pub fn validate(&self) -> Result<(), Error> {
        if [&self.key, &self.purpose, &self.assertion]
            .iter()
            .any(|v| v.trim().is_empty() || v.len() > 4096)
            || [&self.coverage, &self.environment]
                .into_iter()
                .flatten()
                .any(|v| v.trim().is_empty() || v.len() > 4096)
            || self.target_paths.is_empty()
            || self.target_paths.len() > 16
            || self
                .target_paths
                .iter()
                .any(|p| p.is_empty() || p.len() > 4096)
            || self
                .target_paths
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != self.target_paths.len()
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "invalid bounded check declaration",
            ));
        }
        Ok(())
    }
}
pub(crate) fn validate_scoped_check(
    run: &ScopedCheckRun,
    work: Option<ContextItemId>,
    state: VerificationState,
) -> Result<(), Error> {
    run.declaration.validate()?;
    let expected = match run.assertion_state {
        AssertionState::NotObserved => VerificationState::Partial,
        AssertionState::Passed => VerificationState::Passed,
        AssertionState::Failed => VerificationState::Failed,
    };
    let hash = |s: &str| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit());
    if work != Some(run.work_item_id)
        || run.work_revision == 0
        || state != expected
        || run.started_at_unix_micros <= 0
        || run.ended_at_unix_micros < run.started_at_unix_micros
        || run.operation_id.len() != 32
        || !run.operation_id.bytes().all(|b| b.is_ascii_hexdigit())
        || !run
            .invocation_fingerprint
            .strip_prefix("sha256:")
            .is_some_and(hash)
        || run.analysis_snapshot_id.is_empty()
        || run.repository_snapshot_id.is_empty()
        || run.artifacts.len() != run.declaration.target_paths.len()
        || run
            .artifacts
            .iter()
            .zip(&run.declaration.target_paths)
            .any(|(a, p)| a.path != *p || !hash(&a.sha256))
        || (run.assertion_state == AssertionState::NotObserved) != run.assertion_evidence.is_none()
        || run
            .assertion_evidence
            .as_ref()
            .is_some_and(|e| e.start_byte >= e.end_byte || e.end_byte > 8192 || !hash(&e.sha256))
        || run.retest.as_ref().is_some_and(|b| {
            b.checkpoint_revision == 0 || b.rationale.trim().is_empty() || b.rationale.len() > 4096
        })
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "inconsistent scoped check/run/Work/assertion basis",
        ));
    }
    Ok(())
}

pub(crate) fn validate_scoped_command(
    run: &ScopedCheckRun,
    source: &crate::Source,
) -> Result<(), Error> {
    if !matches!(&source.payload, crate::SourcePayload::CommandExecution { capture_fingerprint, invocation_fingerprint, outcome, .. }
        if capture_fingerprint.as_deref() == Some(run.capture_fingerprint()?.as_str())
            && *invocation_fingerprint == run.invocation_fingerprint
            && (run.assertion_state != AssertionState::Passed || (outcome.exit_code == Some(0) && outcome.termination == crate::CommandTermination::Exited)))
        || run.ended_at_unix_micros > source.recorded_at.as_unix_micros()
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "scoped run disagrees with Command Source",
        ));
    }
    Ok(())
}

// New verification input/read fields use the public hexadecimal identity spelling.
macro_rules! string_identity {
    ($module:ident, $id:ident) => {
        pub mod $module {
            use super::*;
            pub fn serialize<S: serde::Serializer>(id: &$id, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&id.to_string())
            }
            pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<$id, D::Error> {
                let text = String::deserialize(d)?;
                if text.len() != 32 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(serde::de::Error::custom(
                        "expected hexadecimal 128-bit identity",
                    ));
                }
                let mut bytes = [0u8; 16];
                for (i, byte) in bytes.iter_mut().enumerate() {
                    *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
                        .map_err(serde::de::Error::custom)?;
                }
                Ok($id::from_bytes(bytes))
            }
        }
    };
}
string_identity!(check_work_id, ContextItemId);
string_identity!(check_source_id, SourceId);
string_identity!(check_checkpoint_id, CheckpointId);

impl ScopedCheckRun {
    /// The Source commits the observation before any assertion interpretation or
    /// retest link. Those later reports cannot change captured execution identity.
    pub fn capture_fingerprint(&self) -> Result<String, Error> {
        use sha2::{Digest, Sha256};
        let mut basis = self.clone();
        basis.assertion_state = AssertionState::NotObserved;
        basis.assertion_evidence = None;
        basis.retest = None;
        let bytes =
            serde_json::to_vec(&("volicord-scoped-command-capture", basis)).map_err(|e| {
                Error::with_source(
                    ErrorKind::InvalidInput,
                    "cannot fingerprint captured check basis",
                    e,
                )
            })?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}
