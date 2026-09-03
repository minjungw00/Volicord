use std::path::Path;
use tempfile::tempdir;
use volicord_context::{
    CanonicalReadOptions, DeterministicIdGenerator, FixedClock, OperationId, Principal,
    PrincipalKind, ProjectId, Store as ContextStore, TimestampMicros,
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
            engineering_choice_discovery: None,
            materiality_review: None,
            learning_deliberation: None,
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
    assert_eq!(
        expired.disposition,
        CandidateDisposition::ExpiredOrRetentionCleaned
    );
    assert_eq!(
        expired.cleanup.as_ref().map(|cleanup| cleanup.kind),
        Some(CandidateCleanupKind::RetentionExpiry)
    );
    assert!(reopened.get(project_id, first.id)?.content.is_some());
    Ok(())
}

#[test]
fn dismissal_and_explicit_deletion_are_candidate_local() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let project_id = project(9);
    let other_project = project(8);
    let canonical_path = root.path().join("canonical.sqlite3");
    let candidate_path = root.path().join("candidates.sqlite3");
    let mut canonical = ContextStore::open_with(
        &canonical_path,
        DeterministicIdGenerator::new([[9; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(100)),
    )?;
    let canonical_project = canonical
        .create_project(
            OperationId::from_bytes([7; 16]),
            "Candidate cleanup isolation",
        )?
        .value;
    assert_eq!(canonical_project.id, project_id);
    let mut candidates = store(&candidate_path, &[1, 2, 3, 4, 5, 6])?;
    assert!(canonical
        .read_canonical_basis(project_id, CanonicalReadOptions::default())?
        .active_questions
        .is_empty());
    let first = match candidates.submit(observation(
        project_id,
        CandidateCollectionMode::ExplicitUserDirected,
        "dismiss and expire me",
        Some(90),
    ))? {
        SubmissionOutcome::Stored(value) => value,
        _ => return Err("explicit Candidate was blocked".into()),
    };
    let second = match candidates.submit(observation(
        project_id,
        CandidateCollectionMode::ExplicitUserDirected,
        "dismiss and delete me",
        None,
    ))? {
        SubmissionOutcome::Stored(value) => value,
        _ => return Err("explicit Candidate was blocked".into()),
    };
    let pending_deleted = match candidates.submit(observation(
        project_id,
        CandidateCollectionMode::ExplicitUserDirected,
        "delete pending candidate",
        None,
    ))? {
        SubmissionOutcome::Stored(value) => value,
        _ => return Err("explicit Candidate was blocked".into()),
    };
    let no_expiry = match candidates.submit(observation(
        project_id,
        CandidateCollectionMode::ExplicitUserDirected,
        "retain without deadline",
        None,
    ))? {
        SubmissionOutcome::Stored(value) => value,
        _ => return Err("explicit Candidate was blocked".into()),
    };
    let future_expiry = match candidates.submit(observation(
        project_id,
        CandidateCollectionMode::ExplicitUserDirected,
        "retain until later",
        Some(200),
    ))? {
        SubmissionOutcome::Stored(value) => value,
        _ => return Err("explicit Candidate was blocked".into()),
    };
    let other_expired = match candidates.submit(observation(
        other_project,
        CandidateCollectionMode::ExplicitUserDirected,
        "other project expired candidate",
        Some(90),
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
    candidates.dismiss(project_id, second.id, "not applicable")?;
    let deleted = candidates.delete_candidate(project_id, second.id, "privacy request")?;
    assert!(deleted.content.is_none());
    assert!(matches!(
        deleted.disposition,
        CandidateDisposition::Dismissed { ref reason, .. } if reason == "not applicable"
    ));
    assert_eq!(
        deleted.cleanup.as_ref().map(|cleanup| cleanup.kind),
        Some(CandidateCleanupKind::ExplicitDeletion)
    );
    let deleted_revision = deleted.revision;
    let repeated = candidates.delete_candidate(project_id, second.id, "repeated request")?;
    assert_eq!(repeated.revision, deleted_revision);
    assert_eq!(repeated.cleanup, deleted.cleanup);

    let pending_deleted =
        candidates.delete_candidate(project_id, pending_deleted.id, "pending privacy request")?;
    assert_eq!(
        pending_deleted.disposition,
        CandidateDisposition::ExpiredOrRetentionCleaned
    );
    assert_eq!(
        pending_deleted.cleanup.as_ref().map(|cleanup| cleanup.kind),
        Some(CandidateCleanupKind::ExplicitDeletion)
    );

    let cleaned = candidates.cleanup_expired(project_id)?;
    assert_eq!(cleaned, vec![first.id]);
    let expired_dismissed = candidates.get(project_id, first.id)?;
    assert!(matches!(
        expired_dismissed.disposition,
        CandidateDisposition::Dismissed { ref reason, .. } if reason == "not useful"
    ));
    assert_eq!(
        expired_dismissed
            .cleanup
            .as_ref()
            .map(|cleanup| cleanup.kind),
        Some(CandidateCleanupKind::RetentionExpiry)
    );
    assert!(expired_dismissed.content.is_none());
    assert!(candidates.get(project_id, no_expiry.id)?.content.is_some());
    assert!(candidates
        .get(project_id, future_expiry.id)?
        .content
        .is_some());
    assert!(candidates
        .get(other_project, other_expired.id)?
        .content
        .is_some());
    assert!(candidates.cleanup_expired(project_id)?.is_empty());
    assert_eq!(
        candidates.cleanup_expired(other_project)?,
        vec![other_expired.id]
    );
    assert!(canonical
        .read_canonical_basis(project_id, CanonicalReadOptions::default())?
        .active_questions
        .is_empty());
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

#[test]
fn candidate_store_accepts_only_the_current_material_boundary_format(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let current = root.path().join("current.sqlite3");
    drop(CandidateStore::open(&current)?);
    drop(CandidateStore::open(&current)?);

    let non_current = root.path().join("non-current.sqlite3");
    drop(CandidateStore::open(&non_current)?);
    rusqlite::Connection::open(&non_current)?.execute(
        "UPDATE metadata SET value = '12' WHERE key = 'schema_version'",
        [],
    )?;
    let error = CandidateStore::open(&non_current)
        .err()
        .ok_or("non-current Candidate format was admitted")?;
    assert_eq!(
        error.kind(),
        volicord_inquiry::ErrorKind::UnsupportedVersion
    );
    assert!(error.to_string().contains("current version is 14"));
    Ok(())
}
