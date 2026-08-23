use std::{fs, os::unix::fs::PermissionsExt, sync::Mutex};
use tempfile::TempDir;
use volicord_context::{Principal, PrincipalKind, SourceId, TimestampMicros};
use volicord_operations::{
    BackgroundProviderOperationDraft, ConfirmationDecision, ConfirmationRejection,
    GuardedOperationOutcome, GuardedProviderPreparation, GuardedProviderPreparationOutcome,
    LocalOperations, RequestingProvenance, RuntimeLayout,
};
use volicord_privacy::{
    BackgroundSemanticProvider, ProviderAvailability, ProviderDeletionOutcome,
    ProviderDeletionRequest, ProviderExecution, ProviderIdentity, ProviderIntentProvenance,
    ProviderInvocation, ProviderOptInPolicy, ProviderRequestOutcome, ProviderRetentionPolicy,
    SecretFilteringPolicy, SourceExclusionPolicy, TransmissionOutcome,
};

struct Fixture {
    _temporary: TempDir,
    operations: LocalOperations,
    project: volicord_context::ProjectId,
    provider: String,
    model: String,
}

static CONFIG_ENV_LOCK: Mutex<()> = Mutex::new(());

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_provider("fixture-provider", "fixture-model")
    }

    fn new_with_provider(provider: &str, model: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let repository = temporary.path().join("repository");
        fs::create_dir_all(repository.join("src"))?;
        fs::write(
            repository.join("src/lib.rs"),
            "// SECRET=fixture\npub fn answer() -> u32 { 42 }\n",
        )?;
        let operations =
            LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime"))?);
        let project = operations
            .initialize_project("Guarded Provider", Some(&repository))?
            .project
            .id;
        operations.analyze(project, Vec::new())?;
        let source = operations.record_user_source(
            project,
            "codex".into(),
            "privacy-session".into(),
            "enable fixture provider for src/lib.rs".into(),
        )?;
        operations.enable_provider(
            ProviderOptInPolicy {
                project_id: project,
                provider: provider.into(),
                model: model.into(),
                purpose: "background semantic analysis".into(),
                requested_capability: "semantic".into(),
                allowed_source_scopes: vec!["src/lib.rs".into()],
                exclusions: SourceExclusionPolicy {
                    path_prefixes: Vec::new(),
                    file_classes: Vec::new(),
                    basis: "fixture exclusion policy".into(),
                },
                filtering: SecretFilteringPolicy {
                    enabled: true,
                    line_markers: vec!["SECRET".into()],
                    replacement: "[filtered]".into(),
                    known_limits: vec!["marker filtering is incomplete".into()],
                },
                retention: ProviderRetentionPolicy {
                    local_annotation_retained_until: None,
                    local_basis: "until explicit deletion".into(),
                    provider_expectation: "fixture provider policy".into(),
                    provider_known_limits: Vec::new(),
                },
            },
            ProviderIntentProvenance {
                actor: Principal {
                    kind: PrincipalKind::User,
                    identity: "current-host-user".into(),
                },
                host: "codex".into(),
                session: "privacy-session".into(),
                user_turn_source: source_id(&source.identity)?,
                basis: "explicit fixture opt-in".into(),
            },
        )?;
        Ok(Self {
            _temporary: temporary,
            operations,
            project,
            provider: provider.into(),
            model: model.into(),
        })
    }

    fn prepare(&self) -> Result<GuardedProviderPreparation, Box<dyn std::error::Error>> {
        match self.operations.prepare_guarded_provider_operation(
            BackgroundProviderOperationDraft {
                project_id: self.project,
                provider: self.provider.clone(),
                model: self.model.clone(),
                purpose: "background semantic analysis".into(),
                requested_capability: "semantic".into(),
                source_paths: vec!["src/lib.rs".into()],
                expires_at: TimestampMicros::from_unix_micros(9_000_000_000_000_000),
                requesting_provenance: RequestingProvenance {
                    actor: Principal {
                        kind: PrincipalKind::Agent,
                        identity: "codex".into(),
                    },
                    host: Some("codex".into()),
                    session: Some("provider-session".into()),
                    basis: vec!["fixture background request".into()],
                },
            },
        )? {
            GuardedProviderPreparationOutcome::Ready(preparation) => Ok(*preparation),
            GuardedProviderPreparationOutcome::Rejected(record) => {
                Err(format!("unexpected preparation rejection: {:?}", record.outcome).into())
            }
        }
    }

    fn confirm(
        &self,
        preparation: &GuardedProviderPreparation,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.operations.record_confirmation(
            preparation.candidate.confirmation_request_identity,
            preparation.candidate.request_revision,
            &preparation.candidate.effect_fingerprint,
            ConfirmationDecision::Confirmed,
            "codex".into(),
            "provider-session".into(),
            "confirm this exact filtered source transmission".into(),
        )?;
        Ok(())
    }
}

struct FixtureProvider {
    execution: ProviderExecution,
    calls: usize,
    invocations: Vec<ProviderInvocation>,
}

impl BackgroundSemanticProvider for FixtureProvider {
    fn identity(&self) -> ProviderIdentity {
        ProviderIdentity {
            provider: "fixture-provider".into(),
            model: "fixture-model".into(),
        }
    }

    fn availability(&self) -> ProviderAvailability {
        ProviderAvailability::Available
    }

    fn invoke(&mut self, request: ProviderInvocation) -> ProviderExecution {
        self.calls += 1;
        self.invocations.push(request);
        self.execution.clone()
    }

    fn delete(&mut self, _request: ProviderDeletionRequest) -> ProviderDeletionOutcome {
        ProviderDeletionOutcome::NotRequested
    }
}

#[test]
fn local_operations_preserve_no_dispatch_exact_confirmation_and_single_use(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let mut preparation = fixture.prepare()?;
    assert!(preparation.candidate.scope.contains(&format!(
        "provider_request:{}",
        preparation.provider_request.id
    )));
    let mut provider = FixtureProvider {
        execution: ProviderExecution::Completed {
            annotations: Vec::new(),
            diagnostic: None,
        },
        calls: 0,
        invocations: Vec::new(),
    };
    let revision = preparation.candidate.request_revision;
    let fingerprint = preparation.candidate.effect_fingerprint.clone();

    let missing = fixture.operations.dispatch_guarded_provider(
        &mut preparation,
        revision,
        &fingerprint,
        &mut provider,
    )?;
    assert!(matches!(
        missing.outcome,
        GuardedOperationOutcome::NotDispatched {
            rejection: Some(ConfirmationRejection::Missing),
            confirmation_consumed: false,
            ..
        }
    ));
    assert_eq!(provider.calls, 0);
    fixture.confirm(&preparation)?;

    let completed = fixture.operations.dispatch_guarded_provider(
        &mut preparation,
        revision,
        &fingerprint,
        &mut provider,
    )?;
    assert!(matches!(
        completed.outcome,
        GuardedOperationOutcome::DispatchedAndCompleted { .. }
    ));
    assert_eq!(provider.calls, 1);
    assert_eq!(provider.invocations[0].sources.len(), 1);
    assert!(!provider.invocations[0].sources[0]
        .filtered_body
        .contains("SECRET"));

    let inspected = fixture.operations.inspect_guarded_provider_operation(
        fixture.project,
        completed.operation_identity,
        preparation.provider_request.id,
    )?;
    assert_eq!(inspected.operation, completed);
    assert_eq!(
        inspected.provider_request.outcome,
        ProviderRequestOutcome::Completed
    );
    assert!(inspected
        .provider_request
        .manifest
        .iter()
        .any(|entry| entry.transmission_outcome == TransmissionOutcome::Transmitted));

    let reused = fixture.operations.dispatch_guarded_provider(
        &mut preparation,
        revision,
        &fingerprint,
        &mut provider,
    )?;
    assert!(matches!(
        reused.outcome,
        GuardedOperationOutcome::NotDispatched {
            rejection: Some(ConfirmationRejection::Reused),
            ..
        }
    ));
    assert_eq!(provider.calls, 1);
    Ok(())
}

#[test]
fn configured_adapter_unavailability_is_truthful_and_local_work_continues(
) -> Result<(), Box<dyn std::error::Error>> {
    let _environment = CONFIG_ENV_LOCK
        .lock()
        .map_err(|_| "environment lock poisoned")?;
    std::env::remove_var(volicord_operations::CODEX_EXECUTABLE_ENV);
    let fixture = Fixture::new()?;
    let mut preparation = fixture.prepare()?;
    fixture.confirm(&preparation)?;
    let revision = preparation.candidate.request_revision;
    let fingerprint = preparation.candidate.effect_fingerprint.clone();
    let result = fixture
        .operations
        .dispatch_guarded_provider_with_configured_adapter(
            &mut preparation,
            revision,
            &fingerprint,
        )?;
    assert!(matches!(
        result.outcome,
        GuardedOperationOutcome::NotDispatched {
            rejection: None,
            confirmation_consumed: true,
            ..
        }
    ));
    let inspected = fixture.operations.inspect_guarded_provider_operation(
        fixture.project,
        result.operation_identity,
        preparation.provider_request.id,
    )?;
    assert_eq!(
        inspected.provider_request.outcome,
        ProviderRequestOutcome::ProviderUnavailable
    );
    assert!(inspected
        .provider_request
        .manifest
        .iter()
        .all(|entry| entry.transmission_outcome == TransmissionOutcome::NotTransmitted));

    let local = fixture.operations.record_user_source(
        fixture.project,
        "codex".into(),
        "local-continuity".into(),
        "continue local canonical work".into(),
    )?;
    assert_eq!(local.record_kind, "source");
    assert!(!fixture
        .operations
        .canonical_basis(fixture.project)?
        .sources
        .is_empty());
    Ok(())
}

#[test]
fn configured_codex_adapter_completes_the_guarded_production_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let _environment = CONFIG_ENV_LOCK
        .lock()
        .map_err(|_| "environment lock poisoned")?;
    let fixture = Fixture::new_with_provider("openai-codex", "fixture-model")?;
    let executable = fixture._temporary.path().join("codex-fixture");
    fs::write(
        &executable,
        r#"#!/bin/sh
set -eu
if [ "${1:-}" = login ]; then exit 0; fi
output=''
arguments="$*"
while [ "$#" -gt 0 ]; do
  if [ "$1" = '--output-last-message' ]; then
    shift
    output="$1"
  fi
  shift
done
payload=$(sed -n '1,$p')
case "$payload" in
  *'pub fn answer()'*) ;;
  *) exit 41 ;;
esac
case "$payload" in
  *'SECRET=fixture'*) exit 42 ;;
  *) ;;
esac
case "$arguments" in
  *'pub fn answer()'*) exit 43 ;;
  *) ;;
esac
printf '%s' '{"outcome":"completed","diagnostic":"","annotations":[{"included_source_locators":["src/lib.rs"],"text":"The bounded fixture exposes answer.","uncertainty":"low","uncertainty_reasons":["fixture transport"]}]}' > "$output"
printf '%s\n' '{"type":"turn.completed"}'
"#,
    )?;
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))?;
    std::env::set_var(volicord_operations::CODEX_EXECUTABLE_ENV, &executable);

    let result = (|| -> Result<_, Box<dyn std::error::Error>> {
        let mut preparation = fixture.prepare()?;
        fixture.confirm(&preparation)?;
        let revision = preparation.candidate.request_revision;
        let fingerprint = preparation.candidate.effect_fingerprint.clone();
        let operation = fixture
            .operations
            .dispatch_guarded_provider_with_configured_adapter(
                &mut preparation,
                revision,
                &fingerprint,
            )?;
        let inspection = fixture.operations.inspect_guarded_provider_operation(
            fixture.project,
            operation.operation_identity,
            preparation.provider_request.id,
        )?;
        Ok((operation, inspection))
    })();
    std::env::remove_var(volicord_operations::CODEX_EXECUTABLE_ENV);
    let (operation, inspection) = result?;

    assert!(matches!(
        operation.outcome,
        GuardedOperationOutcome::DispatchedAndCompleted { .. }
    ));
    assert_eq!(
        inspection.provider_request.outcome,
        ProviderRequestOutcome::Completed
    );
    assert!(inspection
        .provider_request
        .manifest
        .iter()
        .any(|entry| entry.transmission_outcome == TransmissionOutcome::Transmitted));
    let runtime_bytes = fs::read(fixture.operations.layout().privacy_store())?;
    assert!(!String::from_utf8_lossy(&runtime_bytes).contains("SECRET=fixture"));
    assert!(!fixture
        .operations
        .layout()
        .artifacts_dir()
        .read_dir()?
        .any(|entry| entry
            .ok()
            .is_some_and(|entry| entry.file_name().to_string_lossy().starts_with("provider-"))));
    Ok(())
}

#[test]
fn provider_execution_failure_remains_dispatched_and_failed(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let mut preparation = fixture.prepare()?;
    fixture.confirm(&preparation)?;
    let mut provider = FixtureProvider {
        execution: ProviderExecution::Failed {
            diagnostic: "fixture provider rejected the request".into(),
        },
        calls: 0,
        invocations: Vec::new(),
    };
    let revision = preparation.candidate.request_revision;
    let fingerprint = preparation.candidate.effect_fingerprint.clone();
    let result = fixture.operations.dispatch_guarded_provider(
        &mut preparation,
        revision,
        &fingerprint,
        &mut provider,
    )?;
    assert!(matches!(
        result.outcome,
        GuardedOperationOutcome::DispatchedAndFailed { .. }
    ));
    let inspected = fixture.operations.inspect_guarded_provider_operation(
        fixture.project,
        result.operation_identity,
        preparation.provider_request.id,
    )?;
    assert_eq!(
        inspected.provider_request.outcome,
        ProviderRequestOutcome::ProviderFailed
    );
    assert_eq!(provider.calls, 1);
    Ok(())
}

#[test]
fn revoked_privacy_authority_is_rechecked_after_guarded_confirmation(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let mut preparation = fixture.prepare()?;
    fixture.confirm(&preparation)?;
    let revoke_source = fixture.operations.record_user_source(
        fixture.project,
        "codex".into(),
        "privacy-revoke".into(),
        "revoke provider before dispatch".into(),
    )?;
    fixture.operations.revoke_provider(
        fixture.project,
        ProviderIntentProvenance {
            actor: Principal {
                kind: PrincipalKind::User,
                identity: "current-host-user".into(),
            },
            host: "codex".into(),
            session: "privacy-revoke".into(),
            user_turn_source: source_id(&revoke_source.identity)?,
            basis: "explicit revoke before Guarded dispatch".into(),
        },
    )?;
    let mut provider = FixtureProvider {
        execution: ProviderExecution::Completed {
            annotations: Vec::new(),
            diagnostic: None,
        },
        calls: 0,
        invocations: Vec::new(),
    };
    let revision = preparation.candidate.request_revision;
    let fingerprint = preparation.candidate.effect_fingerprint.clone();
    let result = fixture.operations.dispatch_guarded_provider(
        &mut preparation,
        revision,
        &fingerprint,
        &mut provider,
    )?;
    assert!(matches!(
        result.outcome,
        GuardedOperationOutcome::NotDispatched {
            rejection: None,
            confirmation_consumed: true,
            ..
        }
    ));
    assert_eq!(provider.calls, 0);
    let inspected = fixture.operations.inspect_guarded_provider_operation(
        fixture.project,
        result.operation_identity,
        preparation.provider_request.id,
    )?;
    assert_eq!(
        inspected.provider_request.outcome,
        ProviderRequestOutcome::NotAuthorized
    );
    assert!(inspected
        .provider_request
        .manifest
        .iter()
        .all(|entry| entry.transmission_outcome == TransmissionOutcome::NotTransmitted));
    Ok(())
}

fn source_id(value: &str) -> Result<SourceId, Box<dyn std::error::Error>> {
    if value.len() != 32 {
        return Err("invalid Source identity length".into());
    }
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
    }
    Ok(SourceId::from_bytes(bytes))
}
