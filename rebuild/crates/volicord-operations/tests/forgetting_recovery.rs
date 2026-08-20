use rusqlite::{params, Connection};
use std::{error::Error, fs, path::Path};
use tempfile::TempDir;
use volicord_context::{
    Availability, CanonicalRecordId, OperationId, Principal, PrincipalKind, SourceDraft,
    SourcePayload, Store, TimestampMicros,
};
use volicord_inquiry::{
    CandidateCollectionMode, CandidateCollectionScope, CandidateContent, CandidateDraft,
    CandidateKind, CandidateObservationBasis, CandidateOrigin, CandidateRetention, CandidateStore,
    SubmissionOutcome,
};
use volicord_operations::{ForgettingState, HealthIssueKind, LocalOperations, RuntimeLayout};
use volicord_privacy::{
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedKind, ManagedDerivedState,
    PrivacyStore, ProviderDeletionOutcome,
};
use volicord_projections::CandidateContentOmission;

const RELATED_CANDIDATE_SENTINEL: &str = "FORGET-RELATED-CANDIDATE-9af4";
const RELATED_DERIVED_SENTINEL: &str = "FORGET-RELATED-DERIVED-2c81";
const UNRELATED_CANDIDATE_SENTINEL: &str = "KEEP-UNRELATED-CANDIDATE-f185";
const UNRELATED_DERIVED_SENTINEL: &str = "KEEP-UNRELATED-DERIVED-41de";

struct Fixture {
    _root: TempDir,
    operations: LocalOperations,
    project_id: volicord_context::ProjectId,
    target_source: volicord_context::SourceId,
    authorization_source: volicord_context::SourceId,
    related_candidate: volicord_inquiry::CandidateId,
    unrelated_candidate: volicord_inquiry::CandidateId,
    related_derived: volicord_privacy::ManagedDerivedId,
    unrelated_derived: volicord_privacy::ManagedDerivedId,
}

#[test]
fn successful_forgetting_cleans_related_local_content_and_replays_after_restart(
) -> Result<(), Box<dyn Error>> {
    let fixture = fixture()?;
    let runtime = fixture.operations.layout().root().to_path_buf();

    let first = fixture.operations.forget_record(
        fixture.project_id,
        CanonicalRecordId::Source(fixture.target_source),
        fixture.authorization_source,
    )?;
    assert_eq!(first.state, ForgettingState::Completed);
    assert!(first.canonical_committed);
    assert!(first.candidate_cleanup_completed);
    assert!(first.managed_derived_cleanup_completed);
    assert!(first.residue_verified);
    assert!(!first.replayed);
    assert_eq!(
        first.provider_deletion,
        ProviderDeletionOutcome::NotRequested
    );
    assert!(first.diagnostic.is_none());

    assert_local_cleanup(&fixture.operations, &fixture)?;
    assert_sentinel_absent(
        fixture.operations.layout().candidate_store(),
        RELATED_CANDIDATE_SENTINEL,
    )?;
    assert_sentinel_absent(
        fixture.operations.layout().privacy_store(),
        RELATED_DERIVED_SENTINEL,
    )?;

    let restarted = LocalOperations::new(RuntimeLayout::new(runtime)?);
    let replay = restarted.forget_record(
        fixture.project_id,
        CanonicalRecordId::Source(fixture.target_source),
        fixture.authorization_source,
    )?;
    assert_eq!(replay.operation_id, first.operation_id);
    assert_eq!(replay.state, ForgettingState::Completed);
    assert!(replay.replayed);
    assert_local_cleanup(&restarted, &fixture)?;
    Ok(())
}

#[test]
fn post_canonical_checkpoint_failure_is_repair_required_and_read_barrier_survives_restart(
) -> Result<(), Box<dyn Error>> {
    let fixture = fixture()?;
    let runtime = fixture.operations.layout().root().to_path_buf();
    let candidate_path = fixture.operations.layout().candidate_store();
    let blocker = Connection::open(&candidate_path)?;
    blocker.execute_batch("BEGIN DEFERRED")?;
    let _: String = blocker.query_row(
        "SELECT record_json FROM candidates WHERE id = ?1",
        params![fixture.related_candidate.as_bytes().as_slice()],
        |row| row.get(0),
    )?;

    let partial = fixture.operations.forget_record(
        fixture.project_id,
        CanonicalRecordId::Source(fixture.target_source),
        fixture.authorization_source,
    )?;
    assert_eq!(partial.state, ForgettingState::RepairRequired);
    assert!(partial.canonical_committed);
    assert!(partial.candidate_cleanup_completed);
    assert!(!partial.managed_derived_cleanup_completed);
    assert!(!partial.residue_verified);
    assert!(
        partial
            .diagnostic
            .as_deref()
            .is_some_and(|value| value.contains("cleanup") || value.contains("WAL")),
        "{:?}",
        partial.diagnostic
    );

    Store::open(fixture.operations.layout().canonical_store())?.get_tombstone(
        fixture.project_id,
        CanonicalRecordId::Source(fixture.target_source),
    )?;
    let candidate_basis = fixture.operations.candidate_basis(fixture.project_id)?;
    assert!(candidate_basis
        .withheld_for_canonical_forgetting
        .contains(&fixture.related_candidate));
    assert!(candidate_basis
        .candidates
        .iter()
        .find(|candidate| candidate.id == fixture.related_candidate)
        .and_then(|candidate| candidate.content.as_ref())
        .is_none());
    let projected = fixture.operations.project_projection(fixture.project_id)?;
    assert_eq!(
        projected
            .candidate_inspection
            .iter()
            .find(|candidate| candidate.candidate_id == fixture.related_candidate)
            .and_then(|candidate| candidate.content_omission.clone()),
        Some(CandidateContentOmission::CanonicalForgettingPending)
    );
    let privacy = fixture.operations.privacy_status(fixture.project_id)?;
    assert!(privacy
        .withheld_for_canonical_forgetting
        .contains(&fixture.related_derived));
    let withheld = privacy
        .managed_derived
        .iter()
        .find(|record| record.id == fixture.related_derived)
        .ok_or("related managed Derived record missing")?;
    assert_eq!(withheld.state, ManagedDerivedState::Invalidated);
    assert!(withheld.content.is_none());
    assert!(fixture
        .operations
        .promote_question_candidate(fixture.project_id, fixture.related_candidate)
        .is_err());
    let health = fixture.operations.health(Some(fixture.project_id));
    assert!(health
        .issues
        .iter()
        .any(|issue| issue.kind == HealthIssueKind::RepairRequired));

    blocker.execute_batch("ROLLBACK")?;
    drop(blocker);
    let restarted = LocalOperations::new(RuntimeLayout::new(runtime)?);
    let repaired = restarted.forget_record(
        fixture.project_id,
        CanonicalRecordId::Source(fixture.target_source),
        fixture.authorization_source,
    )?;
    assert_eq!(repaired.operation_id, partial.operation_id);
    assert_eq!(repaired.state, ForgettingState::Completed);
    assert!(repaired.replayed);
    assert_local_cleanup(&restarted, &fixture)?;
    assert!(!restarted
        .health(Some(fixture.project_id))
        .issues
        .iter()
        .any(|issue| issue.kind == HealthIssueKind::RepairRequired));
    Ok(())
}

fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let operations = LocalOperations::new(RuntimeLayout::new(root.path().join("runtime"))?);
    let project = operations
        .initialize_project("Forgetting recovery", None)?
        .project;
    let mut canonical = Store::open(operations.layout().canonical_store())?;
    let target_source = canonical
        .record_source(
            OperationId::from_bytes([81; 16]),
            project.id,
            source_draft(project.revision, "target source"),
        )?
        .value
        .id;
    let authorization_source = canonical
        .record_source(
            OperationId::from_bytes([82; 16]),
            project.id,
            source_draft(project.revision, "forget authorization"),
        )?
        .value
        .id;
    drop(canonical);

    let related_candidate = stored_candidate(
        &operations,
        candidate_draft(project.id, target_source, RELATED_CANDIDATE_SENTINEL),
    )?;
    let unrelated_candidate = stored_candidate(
        &operations,
        candidate_draft(
            project.id,
            authorization_source,
            UNRELATED_CANDIDATE_SENTINEL,
        ),
    )?;
    let mut privacy = PrivacyStore::open(operations.layout().privacy_store())?;
    let related_derived = privacy
        .record_managed_derived(derived_draft(
            project.id,
            target_source,
            RELATED_DERIVED_SENTINEL,
        ))?
        .id;
    let unrelated_derived = privacy
        .record_managed_derived(derived_draft(
            project.id,
            authorization_source,
            UNRELATED_DERIVED_SENTINEL,
        ))?
        .id;
    drop(privacy);
    Ok(Fixture {
        _root: root,
        operations,
        project_id: project.id,
        target_source,
        authorization_source,
        related_candidate,
        unrelated_candidate,
        related_derived,
        unrelated_derived,
    })
}

fn source_draft(expected_project_revision: u64, turn: &str) -> SourceDraft {
    SourceDraft {
        expected_project_revision,
        payload: SourcePayload::CurrentHostUserTurn {
            host: "test".into(),
            session: "forgetting".into(),
            turn: turn.into(),
        },
        actor: Principal {
            kind: PrincipalKind::User,
            identity: "test-user".into(),
        },
        observer: None,
        availability: Availability::Available,
    }
}

fn candidate_draft(
    project_id: volicord_context::ProjectId,
    source_id: volicord_context::SourceId,
    content: &str,
) -> CandidateDraft {
    CandidateDraft {
        project_id,
        kind: CandidateKind::Observation,
        collection_mode: CandidateCollectionMode::Automatic,
        origin: CandidateOrigin {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "test-agent".into(),
            },
            subsystem: "forgetting-test".into(),
            session: Some("forgetting".into()),
            provenance_summary: "forgetting recovery fixture".into(),
        },
        collection_scope: CandidateCollectionScope {
            project_id,
            session: Some("forgetting".into()),
            source_operation: Some("fixture".into()),
            candidate_kind: CandidateKind::Observation,
        },
        observation_basis: CandidateObservationBasis {
            source_basis: vec![source_id],
            ..CandidateObservationBasis::default()
        },
        observed_at: TimestampMicros::from_unix_micros(1),
        retention: CandidateRetention {
            retained_until: None,
            basis: "test retention".into(),
        },
        content: CandidateContent {
            bounded_summary: content.into(),
            question: None,
        },
    }
}

fn stored_candidate(
    operations: &LocalOperations,
    draft: CandidateDraft,
) -> Result<volicord_inquiry::CandidateId, Box<dyn Error>> {
    match operations.submit_candidate(draft)? {
        SubmissionOutcome::Stored(candidate) => Ok(candidate.id),
        SubmissionOutcome::CollectionDisabled { .. } => Err("Candidate collection disabled".into()),
    }
}

fn derived_draft(
    project_id: volicord_context::ProjectId,
    source_id: volicord_context::SourceId,
    content: &str,
) -> ManagedDerivedDraft {
    ManagedDerivedDraft {
        project_id,
        kind: ManagedDerivedKind::CachedSummary,
        provider: None,
        model: None,
        purpose: "forgetting recovery fixture".into(),
        analysis_snapshot: None,
        included_sources: Vec::new(),
        canonical_links: vec![ManagedCanonicalLink::Source(source_id)],
        content: content.into(),
        uncertainty: None,
        retained_until: None,
        retention_basis: "rebuildable test content".into(),
    }
}

fn assert_local_cleanup(
    operations: &LocalOperations,
    fixture: &Fixture,
) -> Result<(), Box<dyn Error>> {
    let candidates = CandidateStore::open(operations.layout().candidate_store())?;
    assert!(candidates
        .get(fixture.project_id, fixture.related_candidate)?
        .content
        .is_none());
    assert_eq!(
        candidates
            .get(fixture.project_id, fixture.unrelated_candidate)?
            .content
            .as_ref()
            .map(|content| content.bounded_summary.as_str()),
        Some(UNRELATED_CANDIDATE_SENTINEL)
    );
    drop(candidates);
    let privacy = PrivacyStore::open(operations.layout().privacy_store())?;
    let related = privacy.get_derived(fixture.project_id, fixture.related_derived)?;
    assert_eq!(related.state, ManagedDerivedState::Deleted);
    assert!(related.content.is_none());
    let unrelated = privacy.get_derived(fixture.project_id, fixture.unrelated_derived)?;
    assert_eq!(unrelated.state, ManagedDerivedState::Current);
    assert_eq!(
        unrelated.content.as_deref(),
        Some(UNRELATED_DERIVED_SENTINEL)
    );
    Ok(())
}

fn assert_sentinel_absent(path: impl AsRef<Path>, sentinel: &str) -> Result<(), Box<dyn Error>> {
    let path = path.as_ref();
    for candidate in [
        path.to_path_buf(),
        path.with_extension("sqlite3-wal"),
        path.with_extension("sqlite3-shm"),
    ] {
        if candidate.exists() {
            let bytes = fs::read(&candidate)?;
            assert!(
                !bytes
                    .windows(sentinel.len())
                    .any(|window| window == sentinel.as_bytes()),
                "forgotten sentinel remains in {}",
                candidate.display()
            );
        }
    }
    Ok(())
}
