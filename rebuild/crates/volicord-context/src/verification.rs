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
    let expected = match run.assertion_state {
        AssertionState::NotObserved => VerificationState::Partial,
        AssertionState::Passed => VerificationState::Passed,
        AssertionState::Failed => VerificationState::Failed,
    };
    let hash = |s: &str| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit());
    reject_scoped_basis(
        run,
        work,
        state,
        &[
            ("bounded_declaration", run.declaration.validate().is_ok()),
            ("work_identity", work == Some(run.work_item_id)),
            ("work_revision", run.work_revision != 0),
            ("assertion_verification_state", state == expected),
            ("positive_start", run.started_at_unix_micros > 0),
            (
                "execution_window",
                run.ended_at_unix_micros >= run.started_at_unix_micros,
            ),
            (
                "operation_identity",
                run.operation_id.len() == 32
                    && run.operation_id.bytes().all(|b| b.is_ascii_hexdigit()),
            ),
            (
                "invocation_fingerprint",
                run.invocation_fingerprint
                    .strip_prefix("sha256:")
                    .is_some_and(hash),
            ),
            ("analysis_identity", !run.analysis_snapshot_id.is_empty()),
            (
                "repository_identity",
                !run.repository_snapshot_id.is_empty(),
            ),
            (
                "artifact_count",
                run.artifacts.len() == run.declaration.target_paths.len(),
            ),
            (
                "artifact_order_and_hash",
                run.artifacts
                    .iter()
                    .zip(&run.declaration.target_paths)
                    .all(|(a, p)| a.path == *p && hash(&a.sha256)),
            ),
            (
                "assertion_evidence_presence",
                (run.assertion_state == AssertionState::NotObserved)
                    == run.assertion_evidence.is_none(),
            ),
            (
                "assertion_evidence_range_and_hash",
                run.assertion_evidence.as_ref().is_none_or(|e| {
                    e.start_byte < e.end_byte && e.end_byte <= 8192 && hash(&e.sha256)
                }),
            ),
            (
                "retest_basis",
                run.retest.as_ref().is_none_or(|b| {
                    b.checkpoint_revision != 0
                        && !b.rationale.trim().is_empty()
                        && b.rationale.len() <= 4096
                }),
            ),
        ],
    )
}

/// Content-free first-failure attribution. Untrusted strings become digests;
/// declarations, paths, command labels, assertions and stream bytes never escape.
#[derive(Clone, Debug, Serialize)]
pub struct ScopedCheckFailure {
    pub failed_predicates: Vec<&'static str>,
    pub checkpoint_work_item_id: Option<String>,
    pub run_work_item_id: String,
    pub work_revision: u64,
    pub verification_state: String,
    pub assertion_state: AssertionState,
    pub started_at_unix_micros: i64,
    pub ended_at_unix_micros: i64,
    pub artifact_count: usize,
    pub declared_target_count: usize,
    pub operation_sha256: String,
    pub invocation_sha256: String,
    pub analysis_sha256: String,
    pub repository_sha256: String,
    pub repository_source_id: String,
    pub command_source_id: Option<String>,
    pub command_recorded_at_unix_micros: Option<i64>,
}

fn reject_scoped_basis(
    run: &ScopedCheckRun,
    work: Option<ContextItemId>,
    state: VerificationState,
    predicates: &[(&'static str, bool)],
) -> Result<(), Error> {
    let failed_predicates: Vec<_> = predicates
        .iter()
        .filter(|(_, valid)| !valid)
        .map(|(name, _)| *name)
        .collect();
    if failed_predicates.is_empty() {
        return Ok(());
    }
    use sha2::{Digest, Sha256};
    let digest = |s: &str| format!("{:x}", Sha256::digest(s.as_bytes()));
    Err(Error::scoped_check_failure_error(ScopedCheckFailure {
        failed_predicates,
        checkpoint_work_item_id: work.map(|id| id.to_string()),
        run_work_item_id: run.work_item_id.to_string(),
        work_revision: run.work_revision,
        verification_state: state.as_str().to_owned(),
        assertion_state: run.assertion_state,
        started_at_unix_micros: run.started_at_unix_micros,
        ended_at_unix_micros: run.ended_at_unix_micros,
        artifact_count: run.artifacts.len(),
        declared_target_count: run.declaration.target_paths.len(),
        operation_sha256: digest(&run.operation_id),
        invocation_sha256: digest(&run.invocation_fingerprint),
        analysis_sha256: digest(&run.analysis_snapshot_id),
        repository_sha256: digest(&run.repository_snapshot_id),
        repository_source_id: run.repository_source_id.to_string(),
        command_source_id: None,
        command_recorded_at_unix_micros: None,
    }))
}

pub(crate) fn validate_scoped_command(
    run: &ScopedCheckRun,
    source: &crate::Source,
) -> Result<(), Error> {
    let capture = run.capture_fingerprint()?;
    let (kind, seal, invocation, outcome) = match &source.payload {
        crate::SourcePayload::CommandExecution {
            capture_fingerprint,
            invocation_fingerprint,
            outcome,
            ..
        } => (
            true,
            capture_fingerprint.as_deref() == Some(capture.as_str()),
            *invocation_fingerprint == run.invocation_fingerprint,
            run.assertion_state != AssertionState::Passed
                || (outcome.exit_code == Some(0)
                    && outcome.termination == crate::CommandTermination::Exited),
        ),
        _ => (false, false, false, false),
    };
    let state = match run.assertion_state {
        AssertionState::NotObserved => VerificationState::Partial,
        AssertionState::Passed => VerificationState::Passed,
        AssertionState::Failed => VerificationState::Failed,
    };
    reject_scoped_basis(
        run,
        Some(run.work_item_id),
        state,
        &[
            ("command_source_kind", kind),
            ("command_capture_seal", seal),
            ("command_invocation", invocation),
            ("passed_process_outcome", outcome),
            (
                "command_recording_chronology",
                run.ended_at_unix_micros <= source.recorded_at.as_unix_micros(),
            ),
        ],
    )
    .map_err(|mut error| {
        error.attach_scoped_command_basis(source);
        error
    })
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

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    fn run() -> ScopedCheckRun {
        ScopedCheckRun {
            work_item_id: ContextItemId::from_bytes([1; 16]),
            work_revision: 1,
            declaration: CheckDeclaration {
                key: "SECRET key".into(),
                purpose: "SECRET purpose".into(),
                assertion: "SECRET assertion".into(),
                coverage: None,
                environment: None,
                target_paths: vec!["SECRET path".into()],
            },
            operation_id: "1".repeat(32),
            invocation_fingerprint: format!("sha256:{}", "2".repeat(64)),
            analysis_snapshot_id: "SECRET analysis".into(),
            repository_snapshot_id: "SECRET repository".into(),
            repository_source_id: SourceId::from_bytes([2; 16]),
            started_at_unix_micros: 10,
            ended_at_unix_micros: 20,
            artifacts: vec![CheckArtifact {
                path: "SECRET path".into(),
                sha256: "3".repeat(64),
            }],
            target_unchanged_during_run: true,
            assertion_state: AssertionState::NotObserved,
            assertion_evidence: None,
            retest: None,
        }
    }

    #[test]
    fn every_scoped_predicate_is_attributable_without_sensitive_values(
    ) -> Result<(), Box<dyn std::error::Error>> {
        type Mutation = fn(&mut ScopedCheckRun);
        let cases: &[(&str, Mutation)] = &[
            ("bounded_declaration", |r| r.declaration.key.clear()),
            ("work_revision", |r| r.work_revision = 0),
            ("assertion_verification_state", |r| {
                r.assertion_state = AssertionState::Passed
            }),
            ("positive_start", |r| r.started_at_unix_micros = 0),
            ("execution_window", |r| r.ended_at_unix_micros = 9),
            ("operation_identity", |r| {
                r.operation_id = "SECRET invalid operation".into()
            }),
            ("invocation_fingerprint", |r| {
                r.invocation_fingerprint = "SECRET invocation".into()
            }),
            ("analysis_identity", |r| r.analysis_snapshot_id.clear()),
            ("repository_identity", |r| r.repository_snapshot_id.clear()),
            ("artifact_count", |r| r.artifacts.clear()),
            ("artifact_order_and_hash", |r| {
                r.artifacts[0].path = "SECRET different path".into()
            }),
            ("artifact_order_and_hash", |r| {
                r.artifacts[0].sha256 = "invalid".into()
            }),
            ("assertion_evidence_presence", |r| {
                r.assertion_evidence = Some(AssertionEvidence {
                    stream: AssertionStream::Stdout,
                    start_byte: 0,
                    end_byte: 1,
                    sha256: "4".repeat(64),
                })
            }),
            ("assertion_evidence_range_and_hash", |r| {
                r.assertion_evidence = Some(AssertionEvidence {
                    stream: AssertionStream::Stdout,
                    start_byte: 5,
                    end_byte: 5,
                    sha256: "4".repeat(64),
                })
            }),
            ("retest_basis", |r| {
                r.retest = Some(CheckRetestBasis {
                    checkpoint_id: CheckpointId::from_bytes([3; 16]),
                    checkpoint_revision: 0,
                    verification_index: 0,
                    source_id: SourceId::from_bytes([4; 16]),
                    target_relation: CheckTargetRelation::SameArtifacts,
                    rationale: "SECRET rationale".into(),
                })
            }),
        ];
        let valid = run();
        validate_scoped_check(&valid, Some(valid.work_item_id), VerificationState::Partial)?;
        for (predicate, mutate) in cases {
            let mut invalid = run();
            mutate(&mut invalid);
            let error = validate_scoped_check(
                &invalid,
                Some(invalid.work_item_id),
                VerificationState::Partial,
            )
            .expect_err(predicate);
            assert_eq!(error.kind(), ErrorKind::InvalidInput);
            let detail = error.scoped_check_failure().ok_or("diagnostic missing")?;
            assert!(detail.failed_predicates.contains(predicate), "{detail:?}");
            assert!(!serde_json::to_string(detail)?.contains("SECRET"));
            assert!(!error.to_string().contains("SECRET"));
            assert_eq!(
                detail.started_at_unix_micros,
                invalid.started_at_unix_micros
            );
        }
        let error = validate_scoped_check(&valid, None, VerificationState::Partial)
            .expect_err("Work required");
        assert_eq!(
            error
                .scoped_check_failure()
                .ok_or("diagnostic")?
                .failed_predicates,
            vec!["work_identity"]
        );
        Ok(())
    }

    #[test]
    fn command_seal_outcome_and_chronology_fail_independently(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut run = run();
        run.assertion_state = AssertionState::Passed;
        let mut source = crate::Source {
            id: SourceId::from_bytes([5; 16]),
            project_id: crate::ProjectId::from_bytes([6; 16]),
            payload: crate::SourcePayload::CommandExecution {
                command_label: "SECRET command".into(),
                capture_fingerprint: Some(run.capture_fingerprint()?),
                invocation_fingerprint: run.invocation_fingerprint.clone(),
                outcome: crate::CommandOutcome {
                    exit_code: Some(0),
                    termination: crate::CommandTermination::Exited,
                },
            },
            actor: crate::Principal {
                kind: crate::PrincipalKind::Agent,
                identity: "SECRET actor".into(),
            },
            observer: None,
            availability: crate::Availability::Available,
            recorded_at: crate::TimestampMicros::from_unix_micros(20),
        };
        validate_scoped_command(&run, &source)?;
        source.recorded_at = crate::TimestampMicros::from_unix_micros(19);
        let error = validate_scoped_command(&run, &source).expect_err("chronology");
        let detail = error.scoped_check_failure().ok_or("diagnostic")?;
        assert_eq!(
            detail.failed_predicates,
            vec!["command_recording_chronology"]
        );
        assert_eq!(detail.command_source_id, Some(source.id.to_string()));
        assert_eq!(detail.command_recorded_at_unix_micros, Some(19));
        source.recorded_at = crate::TimestampMicros::from_unix_micros(20);
        if let crate::SourcePayload::CommandExecution {
            capture_fingerprint,
            invocation_fingerprint,
            outcome,
            ..
        } = &mut source.payload
        {
            *capture_fingerprint = None;
            *invocation_fingerprint = "SECRET wrong invocation".into();
            outcome.exit_code = Some(7);
        }
        let error = validate_scoped_command(&run, &source).expect_err("seal and outcome");
        let detail = error.scoped_check_failure().ok_or("diagnostic")?;
        assert_eq!(
            detail.failed_predicates,
            vec![
                "command_capture_seal",
                "command_invocation",
                "passed_process_outcome"
            ]
        );
        assert!(!serde_json::to_string(detail)?.contains("SECRET"));
        Ok(())
    }
}
