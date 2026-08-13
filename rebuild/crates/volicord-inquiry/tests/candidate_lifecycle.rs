use std::path::Path;
use tempfile::tempdir;
use volicord_context::{
    DeterministicIdGenerator, FixedClock, Principal, PrincipalKind, ProjectId,
    Store as ContextStore, TimestampMicros,
};
use volicord_inquiry::{
    CandidateCleanupKind, CandidateCollectionMode, CandidateCollectionScope, CandidateContent,
    CandidateDisposition, CandidateDraft, CandidateKind, CandidateObservationBasis,
    CandidateOrigin, CandidateRetention, CandidateStore, CollectionOptOutScope, SubmissionOutcome,
};

fn project(value: u8) -> ProjectId {
    ProjectId::from_bytes([value; 16])
}

fn store(path: &Path, ids: &[u8]) -> Result<CandidateStore, volicord_inquiry::Error> {
    CandidateStore::open_with(
        path,
        DeterministicIdGenerator::new(ids.iter().map(|value| [*value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(100)),
    )
}

fn observation(
    project_id: ProjectId,
    mode: CandidateCollectionMode,
    summary: &str,
    retained_until: Option<i64>,
) -> CandidateDraft {
    CandidateDraft {
        project_id,
        kind: CandidateKind::Observation,
        collection_mode: mode,
        origin: CandidateOrigin {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "codex".to_owned(),
            },
            subsystem: "repository-intelligence".to_owned(),
            session: Some("session-a".to_owned()),
            provenance_summary: "bounded file observation".to_owned(),
        },
        collection_scope: CandidateCollectionScope {
            project_id,
            session: Some("session-a".to_owned()),
            source_operation: Some("inventory".to_owned()),
            candidate_kind: CandidateKind::Observation,
        },
        observation_basis: CandidateObservationBasis {
            repository_snapshot: Some("snapshot-a".to_owned()),
            ..CandidateObservationBasis::default()
        },
        observed_at: TimestampMicros::from_unix_micros(50),
        retention: CandidateRetention {
            retained_until: retained_until.map(TimestampMicros::from_unix_micros),
            basis: "project observation retention".to_owned(),
        },
        content: CandidateContent {
            bounded_summary: summary.to_owned(),
            question: None,
        },
    }
}

#[test]
fn scoped_opt_out_preserves_existing_candidates_and_restart_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let path = root.path().join("candidates.sqlite3");
    let project_id = project(1);
    let mut candidates = store(&path, &[2, 3, 4, 5])?;
    let first = match candidates.submit(observation(
        project_id,
        CandidateCollectionMode::Automatic,
        "first bounded observation",
        None,
    ))? {
        SubmissionOutcome::Stored(value) => value,
        SubmissionOutcome::CollectionDisabled { .. } => {
            return Err("collection unexpectedly disabled".into())
        }
    };

    let scope = CollectionOptOutScope {
        project_id,
        session: Some("session-a".to_owned()),
        source_operation: Some("inventory".to_owned()),
        candidate_kind: Some(CandidateKind::Observation),
    };
    candidates.set_collection_opt_out(scope.clone(), true, "user disabled inventory capture")?;
    let blocked = candidates.submit(observation(
        project_id,
        CandidateCollectionMode::Automatic,
        "must not be retained",
        None,
    ))?;
    assert!(matches!(
        blocked,
        SubmissionOutcome::CollectionDisabled { ref matching_scopes }
            if matching_scopes.len() == 1
    ));
    assert_eq!(
        candidates.get(project_id, first.id)?.disposition,
        CandidateDisposition::PendingOrRetained
    );
    assert!(candidates.get(project_id, first.id)?.content.is_some());

    let explicit = match candidates.submit(observation(
        project_id,
        CandidateCollectionMode::ExplicitUserDirected,
        "explicit user-directed observation",
        Some(90),
    ))? {
        SubmissionOutcome::Stored(value) => value,
        SubmissionOutcome::CollectionDisabled { .. } => {
            return Err("explicit work was blocked".into())
        }
    };
    candidates.set_collection_opt_out(scope, false, "user re-enabled inventory capture")?;
    let enabled = match candidates.submit(observation(
        project_id,
        CandidateCollectionMode::Automatic,
        "collection resumed",
        None,
    ))? {
        SubmissionOutcome::Stored(value) => value,
        SubmissionOutcome::CollectionDisabled { .. } => {
            return Err("collection stayed disabled".into())
        }
    };
    assert_eq!(
        enabled
            .opt_out_state_at_collection
            .iter()
            .find(|policy| !policy.opted_out)
            .map(|policy| policy.opted_out),
        Some(false)
    );

    drop(candidates);
    let mut reopened = store(&path, &[])?;
    let basis = reopened.read_basis(project_id)?;
    assert_eq!(basis.candidates.len(), 3);
    assert_eq!(basis.collection_policies.len(), 1);
    assert_eq!(basis.candidates[0].id, first.id);
    let cleaned = reopened.cleanup_expired(project_id)?;
    assert_eq!(cleaned, vec![explicit.id]);
    let expired = reopened.get(project_id, explicit.id)?;
    assert!(expired.content.is_none());
    assert!(matches!(
        expired.disposition,
        CandidateDisposition::ExpiredOrRetentionCleaned {
            kind: CandidateCleanupKind::RetentionExpiry,
            ..
        }
    ));
    assert!(reopened.get(project_id, first.id)?.content.is_some());
    Ok(())
}

#[test]
fn dismissal_and_explicit_deletion_are_candidate_local() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let project_id = project(9);
    let mut candidates = store(&root.path().join("candidates.sqlite3"), &[1, 2])?;
    let first = match candidates.submit(observation(
        project_id,
        CandidateCollectionMode::ExplicitUserDirected,
        "dismiss me",
        None,
    ))? {
        SubmissionOutcome::Stored(value) => value,
        _ => return Err("explicit Candidate was blocked".into()),
    };
    let second = match candidates.submit(observation(
        project_id,
        CandidateCollectionMode::ExplicitUserDirected,
        "delete me",
        None,
    ))? {
        SubmissionOutcome::Stored(value) => value,
        _ => return Err("explicit Candidate was blocked".into()),
    };
    let dismissed = candidates.dismiss(project_id, first.id, "not useful")?;
    assert!(matches!(
        dismissed.disposition,
        CandidateDisposition::Dismissed { .. }
    ));
    assert!(dismissed.content.is_some());
    let deleted = candidates.delete_candidate(project_id, second.id, "privacy request")?;
    assert!(deleted.content.is_none());
    assert!(matches!(
        deleted.disposition,
        CandidateDisposition::ExpiredOrRetentionCleaned {
            kind: CandidateCleanupKind::ExplicitDeletion,
            ..
        }
    ));
    let error = candidates
        .submit(observation(
            project_id,
            CandidateCollectionMode::Automatic,
            &"x".repeat(4_097),
            None,
        ))
        .err()
        .ok_or("unbounded Candidate content was retained")?;
    assert_eq!(error.kind(), volicord_inquiry::ErrorKind::InvalidInput);
    Ok(())
}

#[test]
fn candidate_and_canonical_stores_reject_a_shared_database_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let canonical_path = root.path().join("canonical.sqlite3");
    let _canonical = ContextStore::open(&canonical_path)?;
    assert_eq!(
        CandidateStore::open(&canonical_path)
            .err()
            .ok_or("Candidate store admitted a canonical database")?
            .kind(),
        volicord_inquiry::ErrorKind::UnsupportedVersion
    );

    let candidate_path = root.path().join("candidates.sqlite3");
    let _candidates = CandidateStore::open(&candidate_path)?;
    assert!(ContextStore::open(&candidate_path).is_err());
    Ok(())
}
