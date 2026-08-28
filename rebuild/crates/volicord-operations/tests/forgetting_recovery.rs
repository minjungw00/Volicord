use rusqlite::{params, Connection};
use serde_json::Value;
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
use volicord_operations::{
    run_cli, CliExit, ForgettingState, HealthIssueKind, LocalOperations, RuntimeLayout,
};
use volicord_privacy::{
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedKind, ManagedDerivedState,
    PrivacyStore,
};
use volicord_projections::{
    CandidateContentOmission, CandidateDependencyState, ProjectionIssueKind,
};

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

    let mut forget_stdout = Vec::new();
    let mut forget_stderr = Vec::new();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime.to_str().ok_or("runtime path is not UTF-8")?,
                "--project",
                &fixture.project_id.to_string(),
                "--json",
                "advanced",
                "records",
                "forget",
                "source",
                &fixture.target_source.to_string(),
                "--source",
                &fixture.authorization_source.to_string(),
            ],
            &mut forget_stdout,
            &mut forget_stderr,
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&forget_stderr)
    );
    let first: Value = serde_json::from_slice(&forget_stdout)?;
    assert_eq!(first["state"], "completed");
    assert_eq!(first["canonical_committed"], true);
    assert_eq!(first["candidate_cleanup_completed"], true);
    assert_eq!(first["managed_derived_cleanup_completed"], true);
    assert_eq!(first["residue_verified"], true);
    assert_eq!(first["replayed"], false);
    assert_eq!(first["provider_deletion"], "notrequested");
    assert!(first["diagnostic"].is_null());
    let operation_id = OperationId::from_bytes(parse_identity(
        first["forgetting_operation_id"]
            .as_str()
            .ok_or("forgetting operation identity missing")?,
    )?);

    assert_local_cleanup(&fixture.operations, &fixture)?;
    assert_sentinel_absent(
        fixture.operations.layout().candidate_store(),
        RELATED_CANDIDATE_SENTINEL,
    )?;
    assert_sentinel_absent(
        fixture.operations.layout().privacy_store(),
        RELATED_DERIVED_SENTINEL,
    )?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    assert_eq!(
        run_cli(
            [
                "--runtime",
                runtime.to_str().ok_or("runtime path is not UTF-8")?,
                "--project",
                &fixture.project_id.to_string(),
                "--json",
                "doctor",
                "repair",
                "--forgetting",
                &operation_id.to_string(),
            ],
            &mut stdout,
            &mut stderr,
        ),
        CliExit::SUCCESS,
        "{}",
        String::from_utf8_lossy(&stderr)
    );
    let cli: Value = serde_json::from_slice(&stdout)?;
    assert_eq!(cli["state"], "completed");
    assert_eq!(cli["forgetting_operation_id"], operation_id.to_string());

    let restarted = LocalOperations::new(RuntimeLayout::new(runtime)?);
    let replay = restarted.repair_forgetting(fixture.project_id, operation_id)?;
    assert_eq!(replay.operation_id, operation_id);
    assert_eq!(replay.state, ForgettingState::Completed);
    assert!(replay.replayed);
    assert_local_cleanup(&restarted, &fixture)?;
    Ok(())
}

fn parse_identity(value: &str) -> Result<[u8; 16], Box<dyn Error>> {
    if value.len() != 32 {
        return Err("identity must contain 32 hexadecimal characters".into());
    }
    let mut bytes = [0_u8; 16];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = u8::from_str_radix(std::str::from_utf8(chunk)?, 16)?;
    }
    Ok(bytes)
}

#[test]
fn post_canonical_checkpoint_failure_is_repair_required_and_read_barrier_survives_restart(
) -> Result<(), Box<dyn Error>> {
    let fixture = fixture()?;
    let runtime = fixture.operations.layout().root().to_path_buf();
    let precommit_projection = fixture.operations.project_projection(fixture.project_id)?;
    assert_eq!(
        precommit_projection
            .candidate_inspection
            .iter()
            .find(|candidate| candidate.candidate_id == fixture.related_candidate)
            .and_then(|candidate| candidate.bounded_summary.as_deref()),
        Some(RELATED_CANDIDATE_SENTINEL)
    );
    let candidate_path = fixture.operations.layout().candidate_store();
    let blocker = Connection::open(&candidate_path)?;
    blocker.execute_batch("BEGIN DEFERRED")?;
    let precommit_snapshot: String = blocker.query_row(
        "SELECT record_json FROM candidates WHERE id = ?1",
        params![fixture.related_candidate.as_bytes().as_slice()],
        |row| row.get(0),
    )?;
    assert!(precommit_snapshot.contains(RELATED_CANDIDATE_SENTINEL));

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
    let still_precommit_snapshot: String = blocker.query_row(
        "SELECT record_json FROM candidates WHERE id = ?1",
        params![fixture.related_candidate.as_bytes().as_slice()],
        |row| row.get(0),
    )?;
    assert_eq!(still_precommit_snapshot, precommit_snapshot);
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
    assert!(!projected
        .source_catalog
        .iter()
        .any(|source| source.source.id == fixture.target_source));
    assert!(projected
        .source_catalog
        .iter()
        .any(|source| source.source.id == fixture.authorization_source));
    assert_eq!(
        projected.candidate_dependency,
        CandidateDependencyState::RepairRequired
    );
    assert!(projected.issues.iter().any(|issue| {
        issue.kind == ProjectionIssueKind::CandidateRepairRequired
            && issue.affected_scope == "candidate_inspection"
    }));
    assert_eq!(
        projected
            .candidate_inspection
            .iter()
            .find(|candidate| candidate.candidate_id == fixture.related_candidate)
            .and_then(|candidate| candidate.content_omission.clone()),
        Some(CandidateContentOmission::CanonicalForgettingPending)
    );
    assert_eq!(
        projected
            .candidate_inspection
            .iter()
            .find(|candidate| candidate.candidate_id == fixture.unrelated_candidate)
            .and_then(|candidate| candidate.bounded_summary.as_deref()),
        Some(UNRELATED_CANDIDATE_SENTINEL)
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
    let health_issue = health
        .issues
        .iter()
        .find(|issue| issue.kind == HealthIssueKind::RepairRequired)
        .ok_or("forgetting repair health issue missing")?;
    assert!(health_issue.detail.contains(&format!(
        "volicord doctor repair --forgetting {} (or add --project {} when repository resolution is unavailable)",
        partial.operation_id, fixture.project_id
    )));
    assert!(fixture
        .operations
        .repair_forgetting(
            volicord_context::ProjectId::from_bytes([99; 16]),
            partial.operation_id,
        )
        .is_err());

    blocker.execute_batch("ROLLBACK")?;
    drop(blocker);
    let restarted = LocalOperations::new(RuntimeLayout::new(runtime)?);
    let repaired = restarted.repair_forgetting(fixture.project_id, partial.operation_id)?;
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
            engineering_choice_discovery: None,
            materiality_review: None,
            learning_deliberation: None,
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
