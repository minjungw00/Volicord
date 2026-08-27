use std::collections::VecDeque;
use std::error::Error;
use std::fs;
use tempfile::{tempdir, TempDir};
use volicord_context::{
    ApplicabilityScope, Availability, CanonicalReadOptions, ContextItemCorrectionDraft,
    ContextItemDraft, CorrectionKind, DeterministicIdGenerator, FixedClock, OperationId, Principal,
    PrincipalKind, Project, SourceDraft, SourceId, SourcePayload, StatementProvenanceRole, Store,
    TimestampMicros,
};
use volicord_inquiry::{
    CandidateCollectionMode, CandidateCollectionScope, CandidateContent, CandidateDraft,
    CandidateKind, CandidateObservationBasis, CandidateOrigin, CandidateRetention, CandidateStore,
    SubmissionOutcome,
};
use volicord_privacy::{
    AuthorityKind, AuthorityObservation, BackgroundSemanticProvider, BackgroundSemanticRequest,
    BackgroundSource, FilterOutcome, ManagedCanonicalLink, ManagedDeletionScope,
    ManagedDerivedDraft, ManagedDerivedKind, ManagedDerivedState, PreparationOutcome, PrivacyStore,
    ProviderAvailability, ProviderDeletionOutcome, ProviderDeletionRequest, ProviderExecution,
    ProviderGeneratedAnnotation, ProviderIdentity, ProviderIntentProvenance, ProviderInvocation,
    ProviderOptInPolicy, ProviderRequestOutcome, ProviderRetentionPolicy, ScopeOutcome,
    SecretFilteringPolicy, SourceClass, SourceExclusionPolicy, TransmissionOutcome,
};
use volicord_repository_intelligence::{
    AnalysisSnapshotId, CanonicalGrounding, CanonicalSourceRef, RepositorySnapshotId, Uncertainty,
};

const NOW: i64 = 1_800_000_000_000_000;

struct CanonicalFixture {
    _root: TempDir,
    store: Store,
    project: Project,
    source_ids: Vec<SourceId>,
    source_refs: Vec<CanonicalSourceRef>,
    user_source: SourceId,
}

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn analysis(value: u8) -> Result<AnalysisSnapshotId, Box<dyn Error>> {
    AnalysisSnapshotId::from_hex(&format!("{value:02x}").repeat(32))
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error).into())
}

fn repository(value: u8) -> Result<RepositorySnapshotId, Box<dyn Error>> {
    RepositorySnapshotId::from_hex(&format!("{value:02x}").repeat(32))
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error).into())
}

fn canonical_fixture() -> Result<CanonicalFixture, Box<dyn Error>> {
    let root = tempdir()?;
    let ids = (1_u8..=12).map(|value| [value; 16]);
    let mut store = Store::open_with(
        root.path().join("context.sqlite3"),
        DeterministicIdGenerator::new(ids),
        FixedClock::new(TimestampMicros::from_unix_micros(NOW)),
    )?;
    let project = store
        .create_project(operation(90), "Privacy fixture")?
        .value;
    let mut source_ids = Vec::new();
    for (index, locator) in ["src/lib.rs", "src/vendor/generated.rs", "docs/readme.md"]
        .iter()
        .enumerate()
    {
        let source = store
            .record_source(
                operation(91 + index as u8),
                project.id,
                SourceDraft {
                    expected_project_revision: project.revision,
                    payload: SourcePayload::File {
                        locator: (*locator).to_owned(),
                        snapshot: "repository-snapshot-a".to_owned(),
                    },
                    actor: Principal {
                        kind: PrincipalKind::Repository,
                        identity: "fixture-repository".to_owned(),
                    },
                    observer: None,
                    availability: Availability::Available,
                },
            )?
            .value;
        source_ids.push(source.id);
    }
    let user_source = store
        .record_source(
            operation(95),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::CurrentHostUserTurn {
                    host: "codex".to_owned(),
                    session: "privacy-test".to_owned(),
                    turn: "privacy-intent".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "project-owner".to_owned(),
                },
                observer: None,
                availability: Availability::Available,
            },
        )?
        .value
        .id;
    let basis = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    let grounding = CanonicalGrounding::from_read_basis(&basis)?;
    let source_refs = source_ids
        .iter()
        .map(|source| grounding.source_reference(*source))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CanonicalFixture {
        _root: root,
        store,
        project,
        source_ids,
        source_refs,
        user_source,
    })
}

fn privacy_store(root: &TempDir) -> Result<PrivacyStore, volicord_privacy::Error> {
    PrivacyStore::open_with(
        root.path().join("privacy.sqlite3"),
        DeterministicIdGenerator::new((20_u8..=90).map(|value| [value; 16])),
        FixedClock::new(TimestampMicros::from_unix_micros(NOW + 100)),
    )
}

fn intent(source: SourceId, basis: &str) -> ProviderIntentProvenance {
    ProviderIntentProvenance {
        actor: Principal {
            kind: PrincipalKind::User,
            identity: "project-owner".to_owned(),
        },
        host: "codex".to_owned(),
        session: "privacy-test".to_owned(),
        user_turn_source: source,
        basis: basis.to_owned(),
    }
}

fn policy(project_id: volicord_context::ProjectId) -> ProviderOptInPolicy {
    ProviderOptInPolicy {
        project_id,
        provider: "deterministic-test-provider".to_owned(),
        model: "fixture-model".to_owned(),
        purpose: "repository annotation".to_owned(),
        requested_capability: "semantic_annotation".to_owned(),
        allowed_source_scopes: vec!["src".to_owned()],
        exclusions: SourceExclusionPolicy {
            path_prefixes: vec!["src/vendor".to_owned()],
            file_classes: vec![SourceClass::Vendor, SourceClass::Binary],
            basis: "vendor and binary content excluded".to_owned(),
        },
        filtering: SecretFilteringPolicy {
            enabled: true,
            line_markers: vec!["TOKEN=".to_owned()],
            replacement: "[secret-like line filtered]".to_owned(),
            known_limits: vec![
                "marker filtering cannot guarantee that every secret is detected".to_owned(),
            ],
        },
        retention: ProviderRetentionPolicy {
            local_annotation_retained_until: None,
            local_basis: "until explicit deletion or invalidation".to_owned(),
            provider_expectation: "test adapter reports deletion outcome".to_owned(),
            provider_known_limits: vec![
                "provider-side deletion is observed separately from local deletion".to_owned(),
            ],
        },
    }
}

fn request(
    fixture: &CanonicalFixture,
    bodies: [&str; 3],
) -> Result<BackgroundSemanticRequest, Box<dyn Error>> {
    Ok(BackgroundSemanticRequest {
        project_id: fixture.project.id,
        repository_snapshot: repository(7)?,
        analysis_snapshot: analysis(8)?,
        provider: "deterministic-test-provider".to_owned(),
        model: "fixture-model".to_owned(),
        purpose: "repository annotation".to_owned(),
        requested_capability: "semantic_annotation".to_owned(),
        requested_source_scopes: vec!["src".to_owned()],
        sources: vec![
            BackgroundSource {
                source: fixture.source_refs[0].clone(),
                locator: "src/lib.rs".to_owned(),
                class: SourceClass::Source,
                body: bodies[0].to_owned(),
            },
            BackgroundSource {
                source: fixture.source_refs[1].clone(),
                locator: "src/vendor/generated.rs".to_owned(),
                class: SourceClass::Vendor,
                body: bodies[1].to_owned(),
            },
            BackgroundSource {
                source: fixture.source_refs[2].clone(),
                locator: "docs/readme.md".to_owned(),
                class: SourceClass::Document,
                body: bodies[2].to_owned(),
            },
        ],
    })
}

struct TestProvider {
    identity: ProviderIdentity,
    availability: ProviderAvailability,
    executions: VecDeque<ProviderExecution>,
    deletion: ProviderDeletionOutcome,
    invocations: Vec<ProviderInvocation>,
    deletions: Vec<ProviderDeletionRequest>,
}

impl TestProvider {
    fn new(executions: impl IntoIterator<Item = ProviderExecution>) -> Self {
        Self {
            identity: ProviderIdentity {
                provider: "deterministic-test-provider".to_owned(),
                model: "fixture-model".to_owned(),
            },
            availability: ProviderAvailability::Available,
            executions: executions.into_iter().collect(),
            deletion: ProviderDeletionOutcome::Succeeded { diagnostic: None },
            invocations: Vec::new(),
            deletions: Vec::new(),
        }
    }
}

impl BackgroundSemanticProvider for TestProvider {
    fn identity(&self) -> ProviderIdentity {
        self.identity.clone()
    }

    fn availability(&self) -> ProviderAvailability {
        self.availability.clone()
    }

    fn invoke(&mut self, request: ProviderInvocation) -> ProviderExecution {
        self.invocations.push(request);
        self.executions
            .pop_front()
            .unwrap_or_else(|| ProviderExecution::Failed {
                diagnostic: "deterministic provider has no configured result".to_owned(),
            })
    }

    fn delete(&mut self, request: ProviderDeletionRequest) -> ProviderDeletionOutcome {
        self.deletions.push(request);
        self.deletion.clone()
    }
}

#[test]
fn interactive_and_local_authority_never_authorize_background_invocation(
) -> Result<(), Box<dyn Error>> {
    let fixture = canonical_fixture()?;
    let privacy_root = tempdir()?;
    let mut privacy = privacy_store(&privacy_root)?;
    let initial = privacy.inspect_project(fixture.project.id)?;
    assert_eq!(
        initial.configuration_state,
        volicord_privacy::ProviderConfigurationState::NeverEnabled
    );
    privacy.record_authority_observation(AuthorityObservation {
        project_id: fixture.project.id,
        kind: AuthorityKind::InteractiveCurrentHostAccess,
        host: Some("codex".to_owned()),
        session: Some("privacy-test".to_owned()),
        request_or_operation: "explain src/lib.rs".to_owned(),
        source_basis: vec![fixture.source_ids[0]],
        purpose: "interactive explanation".to_owned(),
        observed_at: TimestampMicros::from_unix_micros(0),
    })?;
    let rejected = privacy.prepare_background_request(request(
        &fixture,
        ["fn local() {}", "vendor", "documentation"],
    )?)?;
    let record = match rejected {
        PreparationOutcome::Rejected(record) => record,
        PreparationOutcome::Ready(_) => return Err("request ran before opt-in".into()),
    };
    assert_eq!(record.outcome, ProviderRequestOutcome::NotAuthorized);
    assert!(record
        .manifest
        .iter()
        .all(|entry| entry.transmission_outcome == TransmissionOutcome::NotTransmitted));
    let inspection = privacy.inspect_project(fixture.project.id)?;
    assert_eq!(inspection.authority_observations.len(), 1);
    assert!(inspection.current_opt_in.is_none());
    Ok(())
}

#[test]
fn opt_in_filters_truthful_manifest_and_revoke_blocks_prepared_dispatch(
) -> Result<(), Box<dyn Error>> {
    let fixture = canonical_fixture()?;
    let privacy_root = tempdir()?;
    let mut privacy = privacy_store(&privacy_root)?;
    let canonical = fixture
        .store
        .read_canonical_basis(fixture.project.id, CanonicalReadOptions::default())?;
    privacy.enable(
        &canonical,
        policy(fixture.project.id),
        intent(fixture.user_source, "explicit opt-in"),
    )?;
    let mut overbroad = request(&fixture, ["fn broad() {}", "vendor", "docs"])?;
    overbroad.requested_source_scopes.push("docs".to_owned());
    let scope_rejection = match privacy.prepare_background_request(overbroad)? {
        PreparationOutcome::Rejected(record) => record,
        PreparationOutcome::Ready(_) => {
            return Err("scope expansion was silently authorized".into())
        }
    };
    assert_eq!(
        scope_rejection.outcome,
        ProviderRequestOutcome::NotAuthorized
    );
    let annotation_source = fixture.source_ids[0];
    let mut provider = TestProvider::new([ProviderExecution::Completed {
        annotations: vec![ProviderGeneratedAnnotation {
            included_sources: vec![annotation_source],
            text: "bounded semantic annotation".to_owned(),
            uncertainty: Uncertainty::none(),
        }],
        diagnostic: None,
    }]);
    let prepared = match privacy.prepare_background_request(request(
        &fixture,
        [
            "pub fn useful() {}\nTOKEN=must-not-leave\npub fn retained() {}\n",
            "TOKEN=excluded-vendor",
            "outside requested scope",
        ],
    )?)? {
        PreparationOutcome::Ready(value) => value,
        PreparationOutcome::Rejected(record) => {
            return Err(format!("authorized request rejected: {:?}", record.diagnostic).into())
        }
    };
    let completed = privacy.dispatch_background(prepared, &mut provider)?;
    assert_eq!(completed.outcome, ProviderRequestOutcome::Completed);
    assert_eq!(provider.invocations.len(), 1);
    assert_eq!(provider.invocations[0].sources.len(), 1);
    assert!(!provider.invocations[0].sources[0]
        .filtered_body
        .contains("must-not-leave"));
    assert!(provider.invocations[0].sources[0]
        .filtered_body
        .contains("secret-like line filtered"));
    assert!(completed.manifest.iter().any(|entry| {
        entry.locator == "src/lib.rs"
            && entry.filter_outcome == FilterOutcome::Filtered
            && entry.transmission_outcome == TransmissionOutcome::Transmitted
    }));
    assert!(completed.manifest.iter().any(|entry| {
        entry.locator == "src/vendor/generated.rs"
            && entry.scope_outcome == ScopeOutcome::Excluded
            && entry.transmission_outcome == TransmissionOutcome::NotTransmitted
    }));
    assert!(completed.manifest.iter().any(|entry| {
        entry.locator == "docs/readme.md"
            && entry.scope_outcome == ScopeOutcome::OutsideRequestedScope
    }));

    let stale_preparation = match privacy
        .prepare_background_request(request(&fixture, ["fn later() {}", "vendor", "docs"])?)?
    {
        PreparationOutcome::Ready(value) => value,
        PreparationOutcome::Rejected(_) => return Err("second request was not prepared".into()),
    };
    privacy.revoke(
        &canonical,
        fixture.project.id,
        intent(fixture.user_source, "revoke background processing"),
    )?;
    let revoked = privacy.dispatch_background(stale_preparation, &mut provider)?;
    assert_eq!(revoked.outcome, ProviderRequestOutcome::NotAuthorized);
    assert_eq!(provider.invocations.len(), 1);

    drop(privacy);
    let reopened = PrivacyStore::open(privacy_root.path().join("privacy.sqlite3"))?;
    let inspection = reopened.inspect_project(fixture.project.id)?;
    assert_eq!(
        inspection.configuration_state,
        volicord_privacy::ProviderConfigurationState::Revoked
    );
    assert_eq!(inspection.requests.len(), 3);
    assert_eq!(inspection.managed_derived.len(), 1);
    assert!(inspection.managed_derived[0]
        .semantic_annotation()
        .is_some());
    Ok(())
}

#[test]
fn provider_degradation_preserves_local_state_and_truthful_transmission(
) -> Result<(), Box<dyn Error>> {
    let mut fixture = canonical_fixture()?;
    let privacy_root = tempdir()?;
    let mut privacy = privacy_store(&privacy_root)?;
    let canonical = fixture
        .store
        .read_canonical_basis(fixture.project.id, CanonicalReadOptions::default())?;
    privacy.enable(
        &canonical,
        policy(fixture.project.id),
        intent(fixture.user_source, "explicit opt-in"),
    )?;
    let canonical_before = fixture
        .store
        .read_canonical_basis(fixture.project.id, CanonicalReadOptions::default())?;
    privacy.disable(
        &canonical,
        fixture.project.id,
        intent(fixture.user_source, "disable background processing"),
    )?;
    let disabled = match privacy
        .prepare_background_request(request(&fixture, ["disabled body", "vendor", "docs"])?)?
    {
        PreparationOutcome::Rejected(record) => record,
        PreparationOutcome::Ready(_) => return Err("disabled provider prepared a dispatch".into()),
    };
    assert_eq!(disabled.outcome, ProviderRequestOutcome::NotAuthorized);
    privacy.enable(
        &canonical,
        policy(fixture.project.id),
        intent(
            fixture.user_source,
            "explicitly re-enable background processing",
        ),
    )?;

    let mut mismatched = TestProvider::new([]);
    mismatched.identity.provider = "different-provider".to_owned();
    let prepared = match privacy
        .prepare_background_request(request(&fixture, ["fn local() {}", "vendor", "docs"])?)?
    {
        PreparationOutcome::Ready(value) => value,
        PreparationOutcome::Rejected(_) => return Err("mismatch request not prepared".into()),
    };
    let mismatched_record = privacy.dispatch_background(prepared, &mut mismatched)?;
    assert_eq!(
        mismatched_record.outcome,
        ProviderRequestOutcome::ProviderUnavailable
    );
    assert!(mismatched.invocations.is_empty());
    assert!(mismatched_record
        .manifest
        .iter()
        .all(|entry| entry.transmission_outcome == TransmissionOutcome::NotTransmitted));

    let mut unavailable = TestProvider::new([]);
    unavailable.availability = ProviderAvailability::Unavailable {
        diagnostic: "provider configuration absent".to_owned(),
    };
    let prepared = match privacy
        .prepare_background_request(request(&fixture, ["fn local() {}", "vendor", "docs"])?)?
    {
        PreparationOutcome::Ready(value) => value,
        PreparationOutcome::Rejected(_) => return Err("unavailable request not prepared".into()),
    };
    let unavailable_record = privacy.dispatch_background(prepared, &mut unavailable)?;
    assert_eq!(
        unavailable_record.outcome,
        ProviderRequestOutcome::ProviderUnavailable
    );
    assert!(unavailable_record
        .manifest
        .iter()
        .all(|entry| entry.transmission_outcome == TransmissionOutcome::NotTransmitted));
    assert!(unavailable.invocations.is_empty());

    let mut provider = TestProvider::new([
        ProviderExecution::Failed {
            diagnostic: "provider rejected the request".to_owned(),
        },
        ProviderExecution::TimedOut {
            diagnostic: "provider process timed out after receiving the request".to_owned(),
        },
        ProviderExecution::Cancelled {
            diagnostic: "provider process was cancelled after receiving the request".to_owned(),
        },
        ProviderExecution::Partial {
            annotations: vec![ProviderGeneratedAnnotation {
                included_sources: vec![fixture.source_ids[0]],
                text: "partial annotation".to_owned(),
                uncertainty: Uncertainty::none(),
            }],
            diagnostic: "one requested result was omitted".to_owned(),
        },
        ProviderExecution::Stale {
            annotations: vec![ProviderGeneratedAnnotation {
                included_sources: vec![fixture.source_ids[0]],
                text: "historical annotation".to_owned(),
                uncertainty: Uncertainty::none(),
            }],
            diagnostic: "provider result used an older snapshot".to_owned(),
        },
    ]);
    let mut outcomes = Vec::new();
    for body in [
        "failed body",
        "timed out body",
        "cancelled body",
        "partial body",
        "stale body",
    ] {
        let prepared = match privacy
            .prepare_background_request(request(&fixture, [body, "vendor", "docs"])?)?
        {
            PreparationOutcome::Ready(value) => value,
            PreparationOutcome::Rejected(_) => return Err("degradation request rejected".into()),
        };
        outcomes.push(
            privacy
                .dispatch_background(prepared, &mut provider)?
                .outcome,
        );
    }
    assert_eq!(
        outcomes,
        vec![
            ProviderRequestOutcome::ProviderFailed,
            ProviderRequestOutcome::ProviderTimedOut,
            ProviderRequestOutcome::ProviderCancelled,
            ProviderRequestOutcome::Partial,
            ProviderRequestOutcome::Stale,
        ]
    );
    assert_eq!(provider.invocations.len(), 5);
    let invalidated = privacy.invalidate_snapshot(fixture.project.id, analysis(12)?)?;
    assert_eq!(invalidated.len(), 1);
    let canonical_after = fixture
        .store
        .read_canonical_basis(fixture.project.id, CanonicalReadOptions::default())?;
    assert_eq!(canonical_before, canonical_after);

    let context_item = fixture
        .store
        .record_context_item(
            operation(110),
            fixture.project.id,
            ContextItemDraft {
                expected_project_revision: fixture.project.revision,
                role: volicord_context::ContextItemRole::Constraint,
                statement: "User correction remains authoritative".to_owned(),
                provenance_role: StatementProvenanceRole::UserStatement,
                author: Principal {
                    kind: PrincipalKind::User,
                    identity: "project-owner".to_owned(),
                },
                source_basis: vec![fixture.user_source],
                applicability: ApplicabilityScope::default(),
            },
        )?
        .value;
    let context_item = fixture
        .store
        .correct_context_item(
            operation(111),
            fixture.project.id,
            context_item.id,
            ContextItemCorrectionDraft {
                expected_revision: context_item.revision,
                corrected_statement: "User  correction remains authoritative".to_owned(),
                kind: CorrectionKind::Formatting,
                user_authorization_source_id: fixture.user_source,
            },
        )?
        .value;
    let corrected_basis = fixture
        .store
        .read_canonical_basis(fixture.project.id, CanonicalReadOptions::default())?;
    let correction_link = ManagedCanonicalLink::ContextItem(context_item.id);
    privacy.record_managed_derived(ManagedDerivedDraft {
        project_id: fixture.project.id,
        kind: ManagedDerivedKind::CachedSummary,
        provider: Some("deterministic-test-provider".to_owned()),
        model: Some("fixture-model".to_owned()),
        purpose: "reanalysis contradiction".to_owned(),
        analysis_snapshot: Some(analysis(9)?),
        included_sources: vec![fixture.source_refs[0].clone()],
        canonical_links: vec![correction_link],
        content: "provider disagrees with the user correction".to_owned(),
        uncertainty: Some(Uncertainty::none()),
        retained_until: None,
        retention_basis: "managed cache".to_owned(),
    })?;
    assert_eq!(
        corrected_basis,
        fixture
            .store
            .read_canonical_basis(fixture.project.id, CanonicalReadOptions::default())?
    );
    Ok(())
}

#[test]
fn managed_retention_expiry_is_local_scoped_and_does_not_infer_provider_deletion(
) -> Result<(), Box<dyn Error>> {
    let fixture = canonical_fixture()?;
    let privacy_root = tempdir()?;
    let mut privacy = privacy_store(&privacy_root)?;
    let expired = privacy.record_managed_derived(ManagedDerivedDraft {
        project_id: fixture.project.id,
        kind: ManagedDerivedKind::Embedding,
        provider: Some("deterministic-test-provider".to_owned()),
        model: Some("fixture-model".to_owned()),
        purpose: "expired embedding".to_owned(),
        analysis_snapshot: Some(analysis(2)?),
        included_sources: vec![fixture.source_refs[0].clone()],
        canonical_links: vec![ManagedCanonicalLink::Source(fixture.source_ids[0])],
        content: "expired vector payload".to_owned(),
        uncertainty: None,
        retained_until: Some(TimestampMicros::from_unix_micros(NOW)),
        retention_basis: "short local cache retention".to_owned(),
    })?;
    let retained = privacy.record_managed_derived(ManagedDerivedDraft {
        project_id: fixture.project.id,
        kind: ManagedDerivedKind::Embedding,
        provider: None,
        model: None,
        purpose: "retained embedding".to_owned(),
        analysis_snapshot: Some(analysis(2)?),
        included_sources: vec![fixture.source_refs[2].clone()],
        canonical_links: vec![ManagedCanonicalLink::Source(fixture.source_ids[2])],
        content: "retained vector payload".to_owned(),
        uncertainty: None,
        retained_until: Some(TimestampMicros::from_unix_micros(NOW + 10_000)),
        retention_basis: "long local cache retention".to_owned(),
    })?;
    assert_eq!(
        privacy.cleanup_expired(fixture.project.id)?,
        vec![expired.id]
    );
    let expired = privacy.get_derived(fixture.project.id, expired.id)?;
    assert_eq!(expired.state, ManagedDerivedState::Deleted);
    assert!(expired.local_deletion.is_some());
    assert_eq!(
        expired.provider_deletion,
        ProviderDeletionOutcome::NotRequested
    );
    assert_eq!(
        privacy.get_derived(fixture.project.id, retained.id)?.state,
        ManagedDerivedState::Current
    );
    Ok(())
}

fn candidate(
    project_id: volicord_context::ProjectId,
    source_id: SourceId,
    summary: &str,
) -> CandidateDraft {
    CandidateDraft {
        project_id,
        kind: CandidateKind::Observation,
        collection_mode: CandidateCollectionMode::Automatic,
        origin: CandidateOrigin {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "repository-observer".to_owned(),
            },
            subsystem: "repository-intelligence".to_owned(),
            session: Some("privacy-test".to_owned()),
            provenance_summary: "bounded source observation".to_owned(),
        },
        collection_scope: CandidateCollectionScope {
            project_id,
            session: Some("privacy-test".to_owned()),
            source_operation: Some("inventory".to_owned()),
            candidate_kind: CandidateKind::Observation,
        },
        observation_basis: CandidateObservationBasis {
            source_basis: vec![source_id],
            repository_snapshot: Some("repository-snapshot-a".to_owned()),
            ..CandidateObservationBasis::default()
        },
        observed_at: TimestampMicros::from_unix_micros(NOW),
        retention: CandidateRetention {
            retained_until: None,
            basis: "Project observation retention".to_owned(),
        },
        content: CandidateContent {
            bounded_summary: summary.to_owned(),
            question: None,
            materiality_review: None,
        },
    }
}

#[test]
fn canonical_forgetting_cleans_only_related_candidate_and_derived_content(
) -> Result<(), Box<dyn Error>> {
    let mut fixture = canonical_fixture()?;
    let privacy_root = tempdir()?;
    let candidate_root = tempdir()?;
    let mut privacy = privacy_store(&privacy_root)?;
    let mut candidates = CandidateStore::open_with(
        candidate_root.path().join("candidates.sqlite3"),
        DeterministicIdGenerator::new([[70; 16], [71; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(NOW + 100)),
    )?;
    let related = match candidates.submit(candidate(
        fixture.project.id,
        fixture.source_ids[0],
        "related Candidate",
    ))? {
        SubmissionOutcome::Stored(value) => value,
        SubmissionOutcome::CollectionDisabled { .. } => return Err("collection disabled".into()),
    };
    let unrelated = match candidates.submit(candidate(
        fixture.project.id,
        fixture.source_ids[2],
        "unrelated Candidate",
    ))? {
        SubmissionOutcome::Stored(value) => value,
        SubmissionOutcome::CollectionDisabled { .. } => return Err("collection disabled".into()),
    };
    let related_derived = privacy.record_managed_derived(ManagedDerivedDraft {
        project_id: fixture.project.id,
        kind: ManagedDerivedKind::CachedSummary,
        provider: None,
        model: None,
        purpose: "related cache".to_owned(),
        analysis_snapshot: Some(analysis(1)?),
        included_sources: vec![fixture.source_refs[0].clone()],
        canonical_links: vec![ManagedCanonicalLink::Source(fixture.source_ids[0])],
        content: "related managed content".to_owned(),
        uncertainty: None,
        retained_until: None,
        retention_basis: "rebuildable".to_owned(),
    })?;
    let unrelated_derived = privacy.record_managed_derived(ManagedDerivedDraft {
        project_id: fixture.project.id,
        kind: ManagedDerivedKind::CachedSummary,
        provider: None,
        model: None,
        purpose: "unrelated cache".to_owned(),
        analysis_snapshot: Some(analysis(1)?),
        included_sources: vec![fixture.source_refs[2].clone()],
        canonical_links: vec![ManagedCanonicalLink::Source(fixture.source_ids[2])],
        content: "unrelated managed content".to_owned(),
        uncertainty: None,
        retained_until: None,
        retention_basis: "rebuildable".to_owned(),
    })?;
    let forgotten = fixture.store.forget_source(
        operation(120),
        fixture.project.id,
        fixture.source_ids[0],
        fixture.user_source,
    )?;
    let cleanup = privacy.apply_canonical_forgetting(
        &mut candidates,
        &forgotten.value.invalidation,
        "canonical forgetting propagation",
    )?;
    assert_eq!(cleanup.candidate_ids, vec![related.id]);
    assert_eq!(cleanup.derived_ids, vec![related_derived.id]);
    assert!(candidates
        .get(fixture.project.id, related.id)?
        .content
        .is_none());
    assert!(candidates
        .get(fixture.project.id, unrelated.id)?
        .content
        .is_some());
    assert_eq!(
        privacy
            .get_derived(fixture.project.id, related_derived.id)?
            .state,
        ManagedDerivedState::Deleted
    );
    assert_eq!(
        privacy
            .get_derived(fixture.project.id, unrelated_derived.id)?
            .state,
        ManagedDerivedState::Current
    );
    assert!(fixture
        .store
        .read_canonical_basis(fixture.project.id, CanonicalReadOptions::default())?
        .sources
        .iter()
        .any(|source| source.source.id == fixture.source_ids[2]));
    Ok(())
}

#[test]
fn local_and_provider_deletion_truth_are_separate_and_raw_bodies_are_not_portable(
) -> Result<(), Box<dyn Error>> {
    let mut fixture = canonical_fixture()?;
    let privacy_root = tempdir()?;
    let mut privacy = privacy_store(&privacy_root)?;
    let canonical = fixture
        .store
        .read_canonical_basis(fixture.project.id, CanonicalReadOptions::default())?;
    privacy.enable(
        &canonical,
        policy(fixture.project.id),
        intent(fixture.user_source, "explicit opt-in"),
    )?;
    let raw_marker = "RAW-REPOSITORY-BODY-7f9a";
    let mut provider = TestProvider::new([ProviderExecution::Completed {
        annotations: vec![ProviderGeneratedAnnotation {
            included_sources: vec![fixture.source_ids[0]],
            text: "annotation without raw body".to_owned(),
            uncertainty: Uncertainty::none(),
        }],
        diagnostic: None,
    }]);
    let prepared = match privacy
        .prepare_background_request(request(&fixture, [raw_marker, "vendor", "docs"])?)?
    {
        PreparationOutcome::Ready(value) => value,
        PreparationOutcome::Rejected(_) => return Err("authorized request rejected".into()),
    };
    privacy.dispatch_background(prepared, &mut provider)?;
    let annotation = privacy.inspect_project(fixture.project.id)?.managed_derived[0].clone();
    provider.deletion = ProviderDeletionOutcome::Failed {
        diagnostic: "provider deletion endpoint unavailable".to_owned(),
    };
    let deletion = privacy.delete_managed(
        &ManagedDeletionScope {
            project_id: fixture.project.id,
            kinds: vec![ManagedDerivedKind::SemanticAnnotation],
            provider: Some("deterministic-test-provider".to_owned()),
            source_ids: vec![fixture.source_ids[0]],
        },
        "user requested annotation deletion",
        Some(&mut provider),
    )?;
    assert_eq!(deletion.locally_deleted, vec![annotation.id]);
    assert!(matches!(
        deletion.provider_outcome,
        ProviderDeletionOutcome::Failed { .. }
    ));
    let deleted = privacy.get_derived(fixture.project.id, annotation.id)?;
    assert_eq!(deleted.state, ManagedDerivedState::Deleted);
    assert!(deleted.content.is_none());
    assert!(deleted.local_deletion.is_some());
    assert!(matches!(
        deleted.provider_deletion,
        ProviderDeletionOutcome::Failed { .. }
    ));

    let bundle = privacy_root.path().join("context.bundle.json");
    fixture.store.export_bundle(fixture.project.id, &bundle)?;
    assert!(!fs::read_to_string(&bundle)?.contains(raw_marker));
    drop(privacy);
    assert!(
        !String::from_utf8_lossy(&fs::read(privacy_root.path().join("privacy.sqlite3"))?)
            .contains(raw_marker)
    );
    Ok(())
}
