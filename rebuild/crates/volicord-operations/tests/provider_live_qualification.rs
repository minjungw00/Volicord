use serde_json::json;
use std::{env, fs, path::Path};
use volicord_context::{Principal, PrincipalKind, SourceId, TimestampMicros};
use volicord_operations::{
    BackgroundProviderOperationDraft, ConfirmationDecision, GuardedOperationOutcome,
    GuardedProviderPreparation, GuardedProviderPreparationOutcome, LocalOperations,
    RequestingProvenance, RuntimeLayout,
};
use volicord_privacy::{
    ManagedDerivedKind, ProviderIntentProvenance, ProviderOptInPolicy, ProviderRequestOutcome,
    ProviderRetentionPolicy, SecretFilteringPolicy, SourceExclusionPolicy, TransmissionOutcome,
};

const AUTHORIZATION_ENV: &str = "VOLICORD_PROVIDER_QUALIFICATION_AUTHORIZATION";
const MODEL_ENV: &str = "VOLICORD_PROVIDER_QUALIFICATION_MODEL";
const HEAD_ENV: &str = "VOLICORD_PROVIDER_QUALIFICATION_PRODUCTION_HEAD";
const AUTHORIZATION_ASSERTION: &str = "openai-codex-background-semantic-bounded-rust-v1";
const PROVIDER: &str = "openai-codex";
const PURPOSE: &str = "qualify the bounded background semantic provider fixture";
const CAPABILITY: &str = "semantic_annotation";
const LOCATOR: &str = "src/lib.rs";

fn authorization_is_exact(value: Option<&str>) -> bool {
    value == Some(AUTHORIZATION_ASSERTION)
}

#[test]
fn qualification_authorization_is_exact_and_distinct_from_v11() {
    assert!(!authorization_is_exact(None));
    assert!(!authorization_is_exact(Some("")));
    assert!(!authorization_is_exact(Some(
        "v11-openai-codex-project-health-three-targets"
    )));
    assert!(authorization_is_exact(Some(AUTHORIZATION_ASSERTION)));
}

#[test]
#[ignore = "requires exact source-transmission authorization and an authenticated Codex CLI"]
fn live_provider_qualification() -> Result<(), Box<dyn std::error::Error>> {
    let authorization = env::var(AUTHORIZATION_ENV).ok();
    if !authorization_is_exact(authorization.as_deref()) {
        return Err(format!(
            "authorization_blocked: set {AUTHORIZATION_ENV} to the exact maintained assertion"
        )
        .into());
    }
    let model = env::var(MODEL_ENV)
        .map_err(|_| format!("authorization admitted, but {MODEL_ENV} is missing"))?;
    if model.trim().is_empty() {
        return Err(format!("authorization admitted, but {MODEL_ENV} is empty").into());
    }
    let production_head = env::var(HEAD_ENV)
        .map_err(|_| format!("authorization admitted, but {HEAD_ENV} is missing"))?;

    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../validation/privacy/background-provider-qualification/fixtures/bounded-rust");
    let source_body = fs::read(fixture_root.join(LOCATOR))?;
    if source_body.len() > 4 * 1024 {
        return Err("bounded qualification Source exceeds 4096 bytes".into());
    }
    let extra_files = fixture_root
        .join("src")
        .read_dir()?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .count();
    if extra_files != 1 {
        return Err("bounded qualification fixture must contain exactly one Source".into());
    }

    let temporary = tempfile::tempdir()?;
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src"))?;
    fs::write(repository.join(LOCATOR), &source_body)?;
    let operations = LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime"))?);
    let project = operations
        .initialize_project("Background Provider Qualification", Some(&repository))?
        .project
        .id;
    operations.analyze(project, Vec::new())?;
    let intent_source = operations.record_user_source(
        project,
        "codex".into(),
        "background-provider-qualification".into(),
        format!("opt in to {PROVIDER}/{model} for {PURPOSE}; allowed Source is exactly {LOCATOR}"),
    )?;
    operations.enable_provider(
        ProviderOptInPolicy {
            project_id: project,
            provider: PROVIDER.into(),
            model: model.clone(),
            purpose: PURPOSE.into(),
            requested_capability: CAPABILITY.into(),
            allowed_source_scopes: vec![LOCATOR.into()],
            exclusions: SourceExclusionPolicy {
                path_prefixes: vec![".git".into(), "target".into()],
                file_classes: Vec::new(),
                basis: "qualification transmits only the maintained one-file fixture".into(),
            },
            filtering: SecretFilteringPolicy {
                enabled: true,
                line_markers: vec!["QUALIFICATION_SECRET".into()],
                replacement: "[filtered]".into(),
                known_limits: vec![
                    "marker filtering does not establish that arbitrary content is non-sensitive"
                        .into(),
                ],
            },
            retention: ProviderRetentionPolicy {
                local_annotation_retained_until: None,
                local_basis: "retained locally until explicit managed deletion".into(),
                provider_expectation: "authenticated Codex service handling applies".into(),
                provider_known_limits: vec![
                    "this adapter cannot request or prove provider-side deletion".into(),
                ],
            },
        },
        ProviderIntentProvenance {
            actor: Principal {
                kind: PrincipalKind::User,
                identity: "current-host-user".into(),
            },
            host: "codex".into(),
            session: "background-provider-qualification".into(),
            user_turn_source: source_id(&intent_source.identity)?,
            basis: format!(
                "exact external transmission assertion {AUTHORIZATION_ASSERTION} supplied by caller"
            ),
        },
    )?;

    let mut preparation = prepare(&operations, project, &model)?;
    confirm(&operations, &preparation)?;
    let revision = preparation.candidate.request_revision;
    let fingerprint = preparation.candidate.effect_fingerprint.clone();
    let successful = operations.dispatch_guarded_provider_with_configured_adapter(
        &mut preparation,
        revision,
        &fingerprint,
    )?;
    if !matches!(
        successful.outcome,
        GuardedOperationOutcome::DispatchedAndCompleted { .. }
    ) {
        return Err(format!(
            "live provider did not complete successfully: {:?}",
            successful.outcome
        )
        .into());
    }
    let successful_inspection = operations.inspect_guarded_provider_operation(
        project,
        successful.operation_identity,
        preparation.provider_request.id,
    )?;
    if successful_inspection.provider_request.outcome != ProviderRequestOutcome::Completed {
        return Err(format!(
            "live provider request outcome was {:?}",
            successful_inspection.provider_request.outcome
        )
        .into());
    }
    let transmitted = successful_inspection
        .provider_request
        .manifest
        .iter()
        .filter(|entry| entry.transmission_outcome == TransmissionOutcome::Transmitted)
        .collect::<Vec<_>>();
    if transmitted.len() != 1 || transmitted[0].locator != LOCATOR {
        return Err("live provider transmission escaped the exact one-Source scope".into());
    }
    let privacy = operations.privacy_status(project)?;
    let annotations = privacy
        .managed_derived
        .iter()
        .filter(|record| record.kind == ManagedDerivedKind::SemanticAnnotation)
        .collect::<Vec<_>>();
    if annotations.is_empty()
        || annotations.iter().any(|record| {
            record.provider.as_deref() != Some(PROVIDER)
                || record.model.as_deref() != Some(model.as_str())
                || record.analysis_snapshot
                    != Some(successful_inspection.provider_request.analysis_snapshot)
                || record.included_sources.len() != 1
                || record.content.as_deref().is_none_or(str::is_empty)
        })
    {
        return Err("live semantic annotation lost provider, model, snapshot, Source, or content provenance".into());
    }
    let privacy_bytes = fs::read(operations.layout().privacy_store())?;
    if String::from_utf8_lossy(&privacy_bytes).contains("pub struct PulseWindow") {
        return Err("raw qualification Source was retained in the privacy store".into());
    }
    if operations
        .layout()
        .artifacts_dir()
        .read_dir()?
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with("provider-"))
    {
        return Err("raw provider operation artifacts survived live dispatch".into());
    }

    let prior_executable = env::var_os(volicord_operations::CODEX_EXECUTABLE_ENV);
    let missing_executable = temporary.path().join("missing-codex-for-degradation");
    env::set_var(
        volicord_operations::CODEX_EXECUTABLE_ENV,
        &missing_executable,
    );
    let degraded_result = run_degraded_request(&operations, project, &model);
    restore_environment(volicord_operations::CODEX_EXECUTABLE_ENV, prior_executable);
    let (degraded, degraded_inspection) = degraded_result?;
    if !matches!(
        degraded.outcome,
        GuardedOperationOutcome::NotDispatched {
            rejection: None,
            confirmation_consumed: true,
            ..
        }
    ) || degraded_inspection.provider_request.outcome
        != ProviderRequestOutcome::ProviderUnavailable
        || degraded_inspection
            .provider_request
            .manifest
            .iter()
            .any(|entry| entry.transmission_outcome != TransmissionOutcome::NotTransmitted)
    {
        return Err(
            "configured-provider unavailability was not recorded as non-transmission".into(),
        );
    }
    let local = operations.record_user_source(
        project,
        "codex".into(),
        "background-provider-qualification-local-continuity".into(),
        "continue local canonical work after provider unavailability".into(),
    )?;
    let local_continuity =
        local.record_kind == "source" && !operations.canonical_basis(project)?.sources.is_empty();
    if !local_continuity {
        return Err("local canonical work did not continue after provider unavailability".into());
    }

    let evidence = json!({
        "schema_version": 1,
        "qualification_id": "background-provider-openai-codex-v1",
        "production_head": production_head,
        "authorization": {
            "assertion_id": AUTHORIZATION_ASSERTION,
            "distinct_from_v11": true,
            "supplied_by": "caller"
        },
        "provider": {
            "identity": PROVIDER,
            "model": model,
            "transport": "authenticated installed Codex CLI",
            "provider_side_deletion": "unsupported_by_adapter"
        },
        "fixture": {
            "id": "background-provider-bounded-rust-v1",
            "source_count": 1,
            "source_locator": LOCATOR,
            "original_bytes": transmitted[0].original_bytes,
            "transmitted_bytes": transmitted[0].transmitted_bytes
        },
        "success": {
            "guarded_outcome": "dispatched_and_completed",
            "provider_request_outcome": "completed",
            "transmission_outcome": "transmitted",
            "repository_snapshot": successful_inspection.provider_request.repository_snapshot.to_string(),
            "analysis_snapshot": successful_inspection.provider_request.analysis_snapshot.to_string(),
            "semantic_annotation_count": annotations.len(),
            "annotation_provenance_complete": true
        },
        "degradation": {
            "trigger": "configured executable unavailable",
            "guarded_confirmation_consumed": true,
            "provider_request_outcome": "provider_unavailable",
            "transmission_outcome": "not_transmitted",
            "local_canonical_continuity": true
        },
        "retained_evidence": {
            "source_body": false,
            "provider_response_body": false,
            "credential": false
        }
    });
    println!(
        "VOLICORD_PROVIDER_QUALIFICATION_EVIDENCE={}",
        serde_json::to_string(&evidence)?
    );
    Ok(())
}

fn prepare(
    operations: &LocalOperations,
    project: volicord_context::ProjectId,
    model: &str,
) -> Result<GuardedProviderPreparation, Box<dyn std::error::Error>> {
    match operations.prepare_guarded_provider_operation(BackgroundProviderOperationDraft {
        project_id: project,
        provider: PROVIDER.into(),
        model: model.into(),
        purpose: PURPOSE.into(),
        requested_capability: CAPABILITY.into(),
        source_paths: vec![LOCATOR.into()],
        expires_at: TimestampMicros::from_unix_micros(9_000_000_000_000_000),
        requesting_provenance: RequestingProvenance {
            actor: Principal {
                kind: PrincipalKind::Agent,
                identity: "codex".into(),
            },
            host: Some("codex".into()),
            session: Some("background-provider-qualification".into()),
            basis: vec!["bounded live qualification requested by operator".into()],
        },
    })? {
        GuardedProviderPreparationOutcome::Ready(preparation) => Ok(*preparation),
        GuardedProviderPreparationOutcome::Rejected(record) => {
            Err(format!("provider preparation rejected with {:?}", record.outcome).into())
        }
    }
}

fn confirm(
    operations: &LocalOperations,
    preparation: &GuardedProviderPreparation,
) -> Result<(), Box<dyn std::error::Error>> {
    operations.record_confirmation(
        preparation.candidate.confirmation_request_identity,
        preparation.candidate.request_revision,
        &preparation.candidate.effect_fingerprint,
        ConfirmationDecision::Confirmed,
        "codex".into(),
        "background-provider-qualification".into(),
        format!(
            "confirm exactly one filtered {LOCATOR} transmission to {PROVIDER} under {AUTHORIZATION_ASSERTION}"
        ),
    )?;
    Ok(())
}

fn run_degraded_request(
    operations: &LocalOperations,
    project: volicord_context::ProjectId,
    model: &str,
) -> Result<
    (
        volicord_operations::GuardedOperationResult,
        volicord_operations::GuardedProviderInspection,
    ),
    Box<dyn std::error::Error>,
> {
    let mut preparation = prepare(operations, project, model)?;
    confirm(operations, &preparation)?;
    let revision = preparation.candidate.request_revision;
    let fingerprint = preparation.candidate.effect_fingerprint.clone();
    let result = operations.dispatch_guarded_provider_with_configured_adapter(
        &mut preparation,
        revision,
        &fingerprint,
    )?;
    let inspection = operations.inspect_guarded_provider_operation(
        project,
        result.operation_identity,
        preparation.provider_request.id,
    )?;
    Ok((result, inspection))
}

fn restore_environment(name: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => env::set_var(name, value),
        None => env::remove_var(name),
    }
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
