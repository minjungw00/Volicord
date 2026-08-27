use serde_json::{json, Value};
use std::{env, fs, path::PathBuf};
use volicord_context::{
    CanonicalRecordId, Principal, PrincipalKind, ProjectId, SourceId, TimestampMicros,
};
use volicord_inquiry::{
    CandidateCollectionMode, CandidateCollectionScope, CandidateContent, CandidateDraft,
    CandidateId, CandidateKind, CandidateObservationBasis, CandidateOrigin, CandidateRetention,
    CandidateStore, SubmissionOutcome,
};
use volicord_operations::{LocalOperations, RuntimeLayout};
use volicord_privacy::{
    ManagedCanonicalLink, ManagedDerivedDraft, ManagedDerivedId, ManagedDerivedKind,
    ManagedDerivedState, PrivacyStore,
};

const RELATED_CANDIDATE: &str = "V11-LINKED-CANDIDATE-CONTROL";
const UNRELATED_CANDIDATE: &str = "V11-UNRELATED-CANDIDATE-CONTROL";
const RELATED_DERIVED: &str = "V11-LINKED-DERIVED-CONTROL";
const UNRELATED_DERIVED: &str = "V11-UNRELATED-DERIVED-CONTROL";

#[test]
fn seed_and_inspect_v11_forgetting_control() -> Result<(), Box<dyn std::error::Error>> {
    let Ok(action) = env::var("VOLICORD_V11_FIXTURE_ACTION") else {
        return Ok(());
    };
    let runtime = PathBuf::from(env::var("VOLICORD_V11_RUNTIME")?);
    let output = PathBuf::from(env::var("VOLICORD_V11_CONTROL_OUTPUT")?);
    let project = ProjectId::from_bytes(identity(&env::var("VOLICORD_V11_PROJECT")?)?);
    let related_source = SourceId::from_bytes(identity(&env::var("VOLICORD_V11_RELATED_SOURCE")?)?);
    let unrelated_source =
        SourceId::from_bytes(identity(&env::var("VOLICORD_V11_UNRELATED_SOURCE")?)?);
    match action.as_str() {
        "seed" => seed(runtime, output, project, related_source, unrelated_source),
        "inspect" => inspect(
            runtime,
            output,
            project,
            PathBuf::from(env::var("VOLICORD_V11_CONTROL_STATE")?),
        ),
        _ => Err("unsupported V11 fixture-control action".into()),
    }
}

#[test]
fn v11_forgetting_control_detects_product_cleanup_after_restart(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let runtime = temporary.path().join("runtime");
    let state = temporary.path().join("control-state.json");
    let result = temporary.path().join("control-result.json");
    let operations = LocalOperations::new(RuntimeLayout::new(&runtime)?);
    let project = operations
        .initialize_project("V11 forgetting control", None)?
        .project
        .id;
    let related = operations.record_user_source(
        project,
        "v11-fixture".into(),
        "focused".into(),
        "forget the related control".into(),
    )?;
    let unrelated = operations.record_user_source(
        project,
        "v11-fixture".into(),
        "focused".into(),
        "preserve the unrelated control".into(),
    )?;
    let authorization = operations.record_user_source(
        project,
        "v11-fixture".into(),
        "focused".into(),
        "authorize forgetting".into(),
    )?;
    let related = SourceId::from_bytes(identity(&related.identity)?);
    let unrelated = SourceId::from_bytes(identity(&unrelated.identity)?);
    let authorization = SourceId::from_bytes(identity(&authorization.identity)?);
    seed(runtime.clone(), state.clone(), project, related, unrelated)?;
    let forgotten =
        operations.forget_record(project, CanonicalRecordId::Source(related), authorization)?;
    assert!(forgotten.candidate_cleanup_completed);
    assert!(forgotten.managed_derived_cleanup_completed);
    assert!(forgotten.residue_verified);
    drop(operations);

    inspect(runtime, result.clone(), project, state)?;
    let observation: Value = serde_json::from_slice(&fs::read(result)?)?;
    for field in [
        "related_candidate_absent",
        "related_derived_absent",
        "unrelated_candidate_present",
        "unrelated_derived_present",
    ] {
        assert_eq!(observation[field], true, "{field}: {observation}");
    }
    Ok(())
}

fn seed(
    runtime: PathBuf,
    output: PathBuf,
    project: ProjectId,
    related_source: SourceId,
    unrelated_source: SourceId,
) -> Result<(), Box<dyn std::error::Error>> {
    let operations = LocalOperations::new(RuntimeLayout::new(runtime)?);
    let related_candidate = candidate(&operations, project, related_source, RELATED_CANDIDATE)?;
    let unrelated_candidate =
        candidate(&operations, project, unrelated_source, UNRELATED_CANDIDATE)?;
    let mut privacy = PrivacyStore::open(operations.layout().privacy_store())?;
    let related_derived = privacy
        .record_managed_derived(derived(project, related_source, RELATED_DERIVED))?
        .id;
    let unrelated_derived = privacy
        .record_managed_derived(derived(project, unrelated_source, UNRELATED_DERIVED))?
        .id;
    fs::write(
        output,
        serde_json::to_vec_pretty(&json!({
            "related_candidate": related_candidate.to_string(),
            "unrelated_candidate": unrelated_candidate.to_string(),
            "related_derived": related_derived.to_string(),
            "unrelated_derived": unrelated_derived.to_string()
        }))?,
    )?;
    Ok(())
}

fn inspect(
    runtime: PathBuf,
    output: PathBuf,
    project: ProjectId,
    state_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let state: Value = serde_json::from_slice(&fs::read(state_path)?)?;
    let layout = RuntimeLayout::new(runtime)?;
    let candidates = CandidateStore::open(layout.candidate_store())?;
    let related_candidate = candidates.get(
        project,
        CandidateId::from_bytes(identity(required(&state, "related_candidate")?)?),
    )?;
    let unrelated_candidate = candidates.get(
        project,
        CandidateId::from_bytes(identity(required(&state, "unrelated_candidate")?)?),
    )?;
    let privacy = PrivacyStore::open(layout.privacy_store())?;
    let related_derived = privacy.get_derived(
        project,
        ManagedDerivedId::from_bytes(identity(required(&state, "related_derived")?)?),
    )?;
    let unrelated_derived = privacy.get_derived(
        project,
        ManagedDerivedId::from_bytes(identity(required(&state, "unrelated_derived")?)?),
    )?;
    fs::write(
        output,
        serde_json::to_vec_pretty(&json!({
            "related_candidate_absent": related_candidate.content.is_none(),
            "unrelated_candidate_present": unrelated_candidate.content.as_ref().is_some_and(|content| content.bounded_summary == UNRELATED_CANDIDATE),
            "related_derived_absent": related_derived.state == ManagedDerivedState::Deleted && related_derived.content.is_none(),
            "unrelated_derived_present": unrelated_derived.state == ManagedDerivedState::Current && unrelated_derived.content.as_deref() == Some(UNRELATED_DERIVED)
        }))?,
    )?;
    Ok(())
}

fn candidate(
    operations: &LocalOperations,
    project_id: ProjectId,
    source_id: SourceId,
    summary: &str,
) -> Result<CandidateId, Box<dyn std::error::Error>> {
    match operations.submit_candidate(CandidateDraft {
        project_id,
        kind: CandidateKind::Observation,
        collection_mode: CandidateCollectionMode::Automatic,
        origin: CandidateOrigin {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "v11-fixture-control".into(),
            },
            subsystem: "v11-fixture-control".into(),
            session: Some("v11".into()),
            provenance_summary: "bounded V11 forgetting control".into(),
        },
        collection_scope: CandidateCollectionScope {
            project_id,
            session: Some("v11".into()),
            source_operation: Some("fixture-control".into()),
            candidate_kind: CandidateKind::Observation,
        },
        observation_basis: CandidateObservationBasis {
            source_basis: vec![source_id],
            ..CandidateObservationBasis::default()
        },
        observed_at: TimestampMicros::from_unix_micros(1),
        retention: CandidateRetention {
            retained_until: None,
            basis: "retain until V11 forgetting".into(),
        },
        content: CandidateContent {
            bounded_summary: summary.into(),
            question: None,
            materiality_review: None,
        },
    })? {
        SubmissionOutcome::Stored(candidate) => Ok(candidate.id),
        SubmissionOutcome::CollectionDisabled { .. } => Err("Candidate collection disabled".into()),
    }
}

fn derived(project_id: ProjectId, source_id: SourceId, content: &str) -> ManagedDerivedDraft {
    ManagedDerivedDraft {
        project_id,
        kind: ManagedDerivedKind::CachedSummary,
        provider: None,
        model: None,
        purpose: "bounded V11 forgetting control".into(),
        analysis_snapshot: None,
        included_sources: Vec::new(),
        canonical_links: vec![ManagedCanonicalLink::Source(source_id)],
        content: content.into(),
        uncertainty: None,
        retained_until: None,
        retention_basis: "rebuildable V11 fixture".into(),
    }
}

fn identity(value: &str) -> Result<[u8; 16], Box<dyn std::error::Error>> {
    if value.len() != 32 {
        return Err("identity must contain 32 hexadecimal characters".into());
    }
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
    }
    Ok(bytes)
}

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing V11 fixture identity {key}").into())
}
