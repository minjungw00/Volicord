use std::error::Error;
use tempfile::tempdir;
use volicord_context::{
    Availability, CanonicalReadOptions, DeterministicIdGenerator, FixedClock, OperationId,
    Principal, PrincipalKind, ProjectId, SourceDraft, SourceId, SourcePayload, Store,
    TimestampMicros,
};
use volicord_repository_intelligence::CanonicalGrounding;

#[allow(dead_code)]
pub struct TestCanonical {
    pub grounding: CanonicalGrounding,
    pub source_id: SourceId,
}

pub fn repository_grounding(
    project_byte: u8,
    source_byte: u8,
) -> Result<TestCanonical, Box<dyn Error>> {
    let runtime = tempdir()?;
    let project_id = ProjectId::from_bytes([project_byte; 16]);
    let source_id = SourceId::from_bytes([source_byte; 16]);
    let mut store = Store::open_with(
        runtime.path().join("context.sqlite3"),
        DeterministicIdGenerator::new([*project_id.as_bytes(), *source_id.as_bytes()]),
        FixedClock::new(TimestampMicros::from_unix_micros(1_725_000_000_000_000)),
    )?;
    let project = store
        .create_project(OperationId::from_bytes([0xf1; 16]), "Repository fixture")?
        .value;
    let source = store
        .record_source(
            OperationId::from_bytes([0xf2; 16]),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "fixture-repository-snapshot".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "repository-fixture".to_owned(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value;
    let basis = store.read_canonical_basis(
        project.id,
        CanonicalReadOptions {
            include_checkpoint_history: true,
        },
    )?;
    Ok(TestCanonical {
        grounding: CanonicalGrounding::from_read_basis(&basis)?,
        source_id: source.id,
    })
}
