use std::fs;
use tempfile::TempDir;
use volicord_context::{Principal, PrincipalKind, SourceId, TimestampMicros};
use volicord_operations::{
    BackgroundProviderDispatcher, ConfirmationDecision, ConfirmationRejection,
    ConfirmationResponse, DispatchExpectation, DispatchObservation, GuardedEffectCandidate,
    GuardedEffectCategory, GuardedEffectDispatcher, GuardedEffectDraft, GuardedOperationId,
    GuardedOperationOutcome, GuardedRisk, GuardedStore, LocalOperations, RequestingProvenance,
    RuntimeLayout,
};
use volicord_privacy::{
    BackgroundSemanticProvider, BackgroundSemanticRequest, BackgroundSource, PreparationOutcome,
    PrivacyStore, ProviderAvailability, ProviderDeletionOutcome, ProviderDeletionRequest,
    ProviderExecution, ProviderIdentity, ProviderIntentProvenance, ProviderInvocation,
    ProviderOptInPolicy, ProviderRetentionPolicy, SecretFilteringPolicy, SourceClass,
    SourceExclusionPolicy,
};

struct FakeDispatcher {
    calls: usize,
    observation: DispatchObservation,
    operation_ids: Vec<GuardedOperationId>,
}

impl FakeDispatcher {
    fn new(observation: DispatchObservation) -> Self {
        Self {
            calls: 0,
            observation,
            operation_ids: Vec::new(),
        }
    }
}

impl GuardedEffectDispatcher for FakeDispatcher {
    fn dispatch(
        &mut self,
        operation_id: GuardedOperationId,
        _effect: &GuardedEffectCandidate,
    ) -> DispatchObservation {
        self.calls += 1;
        self.operation_ids.push(operation_id);
        self.observation.clone()
    }
}

fn fixture(
) -> Result<(TempDir, LocalOperations, volicord_context::ProjectId), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let repository = temporary.path().join("repository");
    fs::create_dir(&repository)?;
    let operations = LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime"))?);
    let project = operations
        .initialize_project("Guarded Fixture", Some(&repository))?
        .project
        .id;
    Ok((temporary, operations, project))
}

fn future() -> TimestampMicros {
    TimestampMicros::from_unix_micros(9_000_000_000_000_000)
}

fn draft(project_id: volicord_context::ProjectId, suffix: &str) -> GuardedEffectDraft {
    GuardedEffectDraft {
        project_id,
        exact_action: format!("publish-{suffix}"),
        target: format!("registry.example/{suffix}"),
        expected_effect: format!("publish release {suffix}"),
        risk: GuardedRisk {
            category: GuardedEffectCategory::ExternalDeploymentOrPublicPublication,
            concrete_consequence: "external users can observe the release".into(),
        },
        scope: vec![format!("artifact:{suffix}")],
        expires_at: future(),
        requesting_provenance: RequestingProvenance {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "test-agent".into(),
            },
            host: Some("test-host".into()),
            session: Some("test-session".into()),
            basis: vec!["test request".into()],
        },
    }
}

fn assert_rejection(outcome: &GuardedOperationOutcome, expected: ConfirmationRejection) {
    assert!(
        matches!(outcome, GuardedOperationOutcome::NotDispatched { rejection: Some(actual), .. } if *actual == expected)
    );
}

#[test]
fn missing_and_mismatched_confirmation_never_dispatch() -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, project) = fixture()?;
    let candidate = operations.create_guarded_request(draft(project, "one"))?;
    let mut dispatcher =
        FakeDispatcher::new(DispatchObservation::DispatchedAndCompleted { diagnostic: None });
    let missing = operations.dispatch_guarded(
        candidate.confirmation_request_identity,
        candidate.request_revision,
        &DispatchExpectation::from(&candidate),
        &mut dispatcher,
    )?;
    assert_rejection(&missing.outcome, ConfirmationRejection::Missing);
    assert_eq!(dispatcher.calls, 0);
    let mut wrong = DispatchExpectation::from(&candidate);
    wrong.target = "registry.example/different".into();
    let mismatched = operations.dispatch_guarded(
        candidate.confirmation_request_identity,
        candidate.request_revision,
        &wrong,
        &mut dispatcher,
    )?;
    assert_rejection(&mismatched.outcome, ConfirmationRejection::Mismatched);
    assert_eq!(dispatcher.calls, 0);
    Ok(())
}

#[test]
fn exact_cli_confirmation_is_source_linked_single_use_and_operation_linked(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, project) = fixture()?;
    let candidate = operations.create_guarded_request(draft(project, "two"))?;
    let response = operations.record_confirmation(
        candidate.confirmation_request_identity,
        candidate.request_revision,
        &candidate.effect_fingerprint,
        ConfirmationDecision::Confirmed,
        "codex".into(),
        "session-1".into(),
        "I confirm this exact publication".into(),
    )?;
    let canonical = operations.canonical_basis(project)?;
    assert!(canonical
        .sources
        .iter()
        .any(|source| source.source.id == response.user_response_source_id));
    let mut dispatcher = FakeDispatcher::new(DispatchObservation::DispatchedAndCompleted {
        diagnostic: Some("published".into()),
    });
    let completed = operations.dispatch_guarded(
        candidate.confirmation_request_identity,
        candidate.request_revision,
        &DispatchExpectation::from(&candidate),
        &mut dispatcher,
    )?;
    assert!(matches!(
        completed.outcome,
        GuardedOperationOutcome::DispatchedAndCompleted { .. }
    ));
    assert_eq!(dispatcher.calls, 1);
    assert_eq!(dispatcher.operation_ids, vec![completed.operation_identity]);
    assert_eq!(
        completed.user_response_source_id,
        Some(response.user_response_source_id)
    );
    let reused = operations.dispatch_guarded(
        candidate.confirmation_request_identity,
        candidate.request_revision,
        &DispatchExpectation::from(&candidate),
        &mut dispatcher,
    )?;
    assert_rejection(&reused.outcome, ConfirmationRejection::Reused);
    assert_eq!(dispatcher.calls, 1);
    Ok(())
}

#[test]
fn denied_stale_expired_and_invalid_source_responses_never_dispatch(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, project) = fixture()?;
    let denied_candidate = operations.create_guarded_request(draft(project, "denied"))?;
    operations.record_confirmation(
        denied_candidate.confirmation_request_identity,
        denied_candidate.request_revision,
        &denied_candidate.effect_fingerprint,
        ConfirmationDecision::Denied,
        "cli".into(),
        "denial-session".into(),
        "deny".into(),
    )?;
    let mut dispatcher =
        FakeDispatcher::new(DispatchObservation::DispatchedAndCompleted { diagnostic: None });
    let denied = operations.dispatch_guarded(
        denied_candidate.confirmation_request_identity,
        denied_candidate.request_revision,
        &DispatchExpectation::from(&denied_candidate),
        &mut dispatcher,
    )?;
    assert_rejection(&denied.outcome, ConfirmationRejection::Denied);

    let stale_candidate = operations.create_guarded_request(draft(project, "stale"))?;
    let revised = operations.revise_guarded_request(
        stale_candidate.confirmation_request_identity,
        1,
        draft(project, "revised"),
    )?;
    assert_eq!(revised.request_revision, 2);
    let stale = operations.dispatch_guarded(
        stale_candidate.confirmation_request_identity,
        1,
        &DispatchExpectation::from(&stale_candidate),
        &mut dispatcher,
    )?;
    assert_rejection(&stale.outcome, ConfirmationRejection::Stale);

    let mut guarded = GuardedStore::open(operations.layout().guarded_store())?;
    let expired_candidate = guarded.create_request(
        GuardedEffectDraft {
            expires_at: TimestampMicros::from_unix_micros(20),
            ..draft(project, "expired")
        },
        TimestampMicros::from_unix_micros(10),
    )?;
    let expired = guarded.dispatch(
        expired_candidate.confirmation_request_identity,
        1,
        &DispatchExpectation::from(&expired_candidate),
        &operations.canonical_basis(project)?,
        TimestampMicros::from_unix_micros(20),
        &mut dispatcher,
    )?;
    assert_rejection(&expired.outcome, ConfirmationRejection::Expired);

    let invalid_candidate = guarded.create_request(
        draft(project, "invalid-source"),
        TimestampMicros::from_unix_micros(10),
    )?;
    guarded.record_response(ConfirmationResponse::exact_for(
        &invalid_candidate,
        ConfirmationDecision::Confirmed,
        SourceId::from_bytes([99; 16]),
        TimestampMicros::from_unix_micros(11),
    )?)?;
    let invalid = guarded.dispatch(
        invalid_candidate.confirmation_request_identity,
        1,
        &DispatchExpectation::from(&invalid_candidate),
        &operations.canonical_basis(project)?,
        TimestampMicros::from_unix_micros(12),
        &mut dispatcher,
    )?;
    assert_rejection(&invalid.outcome, ConfirmationRejection::InvalidUserSource);
    assert_eq!(dispatcher.calls, 0);
    Ok(())
}

#[test]
fn mismatched_response_payload_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, project) = fixture()?;
    let source =
        operations.record_user_source(project, "cli".into(), "session".into(), "confirm".into())?;
    let source_id = SourceId::from_bytes(parse_hex(&source.identity)?);
    let mut guarded = GuardedStore::open(operations.layout().guarded_store())?;
    let candidate = guarded.create_request(
        draft(project, "response-mismatch"),
        TimestampMicros::from_unix_micros(10),
    )?;
    let mut response = ConfirmationResponse::exact_for(
        &candidate,
        ConfirmationDecision::Confirmed,
        source_id,
        TimestampMicros::from_unix_micros(11),
    )?;
    response.expected_effect = "different effect".into();
    guarded.record_response(response)?;
    let mut dispatcher =
        FakeDispatcher::new(DispatchObservation::DispatchedAndCompleted { diagnostic: None });
    let result = guarded.dispatch(
        candidate.confirmation_request_identity,
        1,
        &DispatchExpectation::from(&candidate),
        &operations.canonical_basis(project)?,
        TimestampMicros::from_unix_micros(12),
        &mut dispatcher,
    )?;
    assert_rejection(&result.outcome, ConfirmationRejection::Mismatched);
    assert_eq!(dispatcher.calls, 0);
    Ok(())
}

#[test]
fn indeterminate_execution_is_durable_and_never_silently_retried(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, project) = fixture()?;
    let candidate = operations.create_guarded_request(draft(project, "indeterminate"))?;
    operations.record_confirmation(
        candidate.confirmation_request_identity,
        1,
        &candidate.effect_fingerprint,
        ConfirmationDecision::Confirmed,
        "cli".into(),
        "session".into(),
        "confirm".into(),
    )?;
    let mut dispatcher = FakeDispatcher::new(DispatchObservation::ExecutionOutcomeIndeterminate {
        diagnostic: "connection lost after dispatch".into(),
    });
    let result = operations.dispatch_guarded(
        candidate.confirmation_request_identity,
        1,
        &DispatchExpectation::from(&candidate),
        &mut dispatcher,
    )?;
    assert!(matches!(
        result.outcome,
        GuardedOperationOutcome::ExecutionOutcomeIndeterminate { .. }
    ));
    assert_eq!(dispatcher.calls, 1);
    let reopened = GuardedStore::open(operations.layout().guarded_store())?;
    assert_eq!(
        reopened.operation(result.operation_identity)?.outcome,
        result.outcome
    );
    let retry = operations.dispatch_guarded(
        candidate.confirmation_request_identity,
        1,
        &DispatchExpectation::from(&candidate),
        &mut dispatcher,
    )?;
    assert_rejection(&retry.outcome, ConfirmationRejection::Reused);
    assert_eq!(dispatcher.calls, 1);
    Ok(())
}

#[test]
fn confirmed_but_not_dispatched_is_consumed_and_not_reused(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, project) = fixture()?;
    let candidate = operations.create_guarded_request(draft(project, "dispatch-failed"))?;
    operations.record_confirmation(
        candidate.confirmation_request_identity,
        1,
        &candidate.effect_fingerprint,
        ConfirmationDecision::Confirmed,
        "cli".into(),
        "session".into(),
        "confirm".into(),
    )?;
    let mut dispatcher = FakeDispatcher::new(DispatchObservation::NotDispatched {
        diagnostic: "adapter unavailable".into(),
    });
    let result = operations.dispatch_guarded(
        candidate.confirmation_request_identity,
        1,
        &DispatchExpectation::from(&candidate),
        &mut dispatcher,
    )?;
    assert!(matches!(
        result.outcome,
        GuardedOperationOutcome::NotDispatched {
            rejection: None,
            confirmation_consumed: true,
            ..
        }
    ));
    let retry = operations.dispatch_guarded(
        candidate.confirmation_request_identity,
        1,
        &DispatchExpectation::from(&candidate),
        &mut dispatcher,
    )?;
    assert_rejection(&retry.outcome, ConfirmationRejection::Reused);
    assert_eq!(dispatcher.calls, 1);
    Ok(())
}

struct FakeProvider {
    calls: usize,
}
impl BackgroundSemanticProvider for FakeProvider {
    fn identity(&self) -> ProviderIdentity {
        ProviderIdentity {
            provider: "fixture-provider".into(),
            model: "fixture-model".into(),
        }
    }
    fn availability(&self) -> ProviderAvailability {
        ProviderAvailability::Available
    }
    fn invoke(&mut self, _request: ProviderInvocation) -> ProviderExecution {
        self.calls += 1;
        ProviderExecution::Completed {
            annotations: Vec::new(),
        }
    }
    fn delete(&mut self, _request: ProviderDeletionRequest) -> ProviderDeletionOutcome {
        ProviderDeletionOutcome::NotRequested
    }
}

#[test]
fn background_source_transmission_uses_guarded_dispatch_after_privacy_opt_in(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temporary, operations, project) = fixture()?;
    let repository = operations.canonical_basis(project)?.project;
    let user_source = operations.record_user_source(
        project,
        "cli".into(),
        "privacy-session".into(),
        "enable provider".into(),
    )?;
    let user_source_id = SourceId::from_bytes(parse_hex(&user_source.identity)?);
    operations.enable_provider(
        ProviderOptInPolicy {
            project_id: project,
            provider: "fixture-provider".into(),
            model: "fixture-model".into(),
            purpose: "semantic analysis".into(),
            requested_capability: "semantic".into(),
            allowed_source_scopes: vec!["src/lib.rs".into()],
            exclusions: SourceExclusionPolicy {
                path_prefixes: Vec::new(),
                file_classes: Vec::new(),
                basis: "fixture".into(),
            },
            filtering: SecretFilteringPolicy {
                enabled: true,
                line_markers: vec!["SECRET".into()],
                replacement: "[filtered]".into(),
                known_limits: vec!["marker filtering is incomplete".into()],
            },
            retention: ProviderRetentionPolicy {
                local_annotation_retained_until: None,
                local_basis: "fixture".into(),
                provider_expectation: "fixture".into(),
                provider_known_limits: Vec::new(),
            },
        },
        ProviderIntentProvenance {
            actor: Principal {
                kind: PrincipalKind::User,
                identity: "current-host-user".into(),
            },
            host: "cli".into(),
            session: "privacy-session".into(),
            user_turn_source: user_source_id,
            basis: "explicit opt-in".into(),
        },
    )?;
    let analysis = operations
        .analyze(project, Vec::new())?
        .value
        .ok_or("missing analysis")?;
    let mut privacy = PrivacyStore::open(operations.layout().privacy_store())?;
    let prepared = match privacy.prepare_background_request(BackgroundSemanticRequest {
        project_id: project,
        repository_snapshot: analysis.repository.identity,
        analysis_snapshot: analysis.analysis.identity,
        provider: "fixture-provider".into(),
        model: "fixture-model".into(),
        purpose: "semantic analysis".into(),
        requested_capability: "semantic".into(),
        requested_source_scopes: vec!["src/lib.rs".into()],
        sources: vec![BackgroundSource {
            source: analysis.analysis.repository_source.clone(),
            locator: "src/lib.rs".into(),
            class: SourceClass::Source,
            body: "pub fn answer() -> u32 { 42 }".into(),
        }],
    })? {
        PreparationOutcome::Ready(prepared) => prepared,
        PreparationOutcome::Rejected(record) => {
            return Err(format!("provider request rejected: {:?}", record.outcome).into())
        }
    };
    let candidate = operations.create_guarded_request(GuardedEffectDraft {
        project_id: repository.id,
        exact_action: "transmit-source-for-semantic-analysis".into(),
        target: "fixture-provider/fixture-model".into(),
        expected_effect: "send filtered src/lib.rs to the configured provider".into(),
        risk: GuardedRisk {
            category: GuardedEffectCategory::PersonalDataOrSourceCodeExternalTransmission,
            concrete_consequence: "source leaves the local environment".into(),
        },
        scope: vec!["src/lib.rs".into()],
        expires_at: future(),
        requesting_provenance: RequestingProvenance {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "test-agent".into(),
            },
            host: Some("cli".into()),
            session: Some("provider-session".into()),
            basis: vec!["prepared provider request".into()],
        },
    })?;
    operations.record_confirmation(
        candidate.confirmation_request_identity,
        1,
        &candidate.effect_fingerprint,
        ConfirmationDecision::Confirmed,
        "cli".into(),
        "provider-session".into(),
        "confirm exact source transmission".into(),
    )?;
    let mut provider = FakeProvider { calls: 0 };
    let mut prepared = Some(prepared);
    let mut dispatcher =
        BackgroundProviderDispatcher::new(&mut privacy, &mut prepared, &candidate, &mut provider);
    let result = operations.dispatch_guarded(
        candidate.confirmation_request_identity,
        1,
        &DispatchExpectation::from(&candidate),
        &mut dispatcher,
    )?;
    drop(dispatcher);
    assert!(matches!(
        result.outcome,
        GuardedOperationOutcome::DispatchedAndCompleted { .. }
    ));
    assert_eq!(provider.calls, 1);
    Ok(())
}

fn parse_hex(value: &str) -> Result<[u8; 16], Box<dyn std::error::Error>> {
    if value.len() != 32 {
        return Err("invalid identity length".into());
    }
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
    }
    Ok(bytes)
}
