use std::{
    fs,
    path::{Component, Path},
};

use serde_json::Value;
use sha2::{Digest, Sha256};
use volicord_types::{
    ArtifactRef, ChangeUnitEffectContract, CurrentCloseBasis, EvidenceAssuranceLevel,
    EvidenceCoverageItem, EvidenceSourceKind, JudgmentBasis, JudgmentBasisCompatibilityStatus,
    JudgmentRationale, JudgmentResolutionOutcome, PersistedArtifactProducer,
    PersistedArtifactProvenanceMetadata, PersistedEvidenceMetadata, PersistedJudgmentBasis,
    PersistedUserJudgmentOptions, PersistedUserJudgmentRequest, PersistedUserJudgmentResolution,
    ProjectContinuityKind, ProjectContinuityStatus, ProjectEnforcementProfile,
    ProjectEnforcementProfileSource, ProjectEnforcementProfileStatus, StateRecordRef,
    UserJudgmentOptionAction, UtcTimestamp, BASELINE_COOPERATIVE_ENFORCEMENT_PROFILE_ID,
};

use super::{PendingTaskEvent, VerifiedReplayContext};
use crate::{StoreError, StoreResult};

pub(super) fn validate_project_enforcement_profile(
    profile: &ProjectEnforcementProfile,
    project_id: &str,
) -> StoreResult<()> {
    let unsupported = || {
        StoreError::corrupt_owner_state_value(
            "project_state",
            project_id.to_owned(),
            "enforcement_profile_json",
        )
    };
    if profile.profile_id.trim().is_empty() {
        return Err(unsupported());
    }
    if profile.profile_id != BASELINE_COOPERATIVE_ENFORCEMENT_PROFILE_ID {
        return Err(unsupported());
    }
    if profile.guarantee_level != volicord_types::GuaranteeLevel::Cooperative {
        return Err(unsupported());
    }
    if !profile.enabled_mechanisms.is_empty() {
        return Err(unsupported());
    }
    if profile.source != ProjectEnforcementProfileSource::BaselineScope {
        return Err(unsupported());
    }
    if profile.status != ProjectEnforcementProfileStatus::Active {
        return Err(unsupported());
    }
    Ok(())
}

pub(super) fn validate_pending_event(event: &PendingTaskEvent) -> StoreResult<()> {
    validate_identifier("event_id", &event.event_id)?;
    validate_identifier("task_id", &event.task_id)?;
    validate_identifier("event_kind", &event.event_kind)?;
    validate_json_text("authority_events.payload_json", &event.event_payload_json)
}

pub(super) fn validate_replay_context(context: &VerifiedReplayContext) -> StoreResult<()> {
    validate_identifier("actor_source", &context.actor_source)?;
    validate_identifier("operation_category", &context.operation_category)?;
    if let Some(verification_basis) = &context.verification_basis {
        validate_identifier("verification_basis", verification_basis)?;
    }
    Ok(())
}

pub(super) fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

pub(super) fn validate_project_continuity_kind(
    field: &'static str,
    value: &str,
) -> StoreResult<()> {
    serde_json::from_value::<ProjectContinuityKind>(Value::String(value.to_owned()))
        .map(|_| ())
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("{field} must be a supported project-continuity kind: {error}"),
        })
}

pub(super) fn validate_project_continuity_status(
    field: &'static str,
    value: &str,
) -> StoreResult<()> {
    serde_json::from_value::<ProjectContinuityStatus>(Value::String(value.to_owned()))
        .map(|_| ())
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("{field} must be a supported project-continuity status: {error}"),
        })
}

pub(super) fn validate_nonempty_text(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

pub(super) fn validate_timestamp(field: &'static str, value: &str) -> StoreResult<()> {
    UtcTimestamp::parse(value)
        .map(|_| ())
        .map_err(|_| StoreError::InvalidInput {
            detail: format!("{field} must be a valid RFC 3339 timestamp"),
        })
}

pub(super) fn validate_artifact_sha256(field: &'static str, value: &str) -> StoreResult<()> {
    if is_lowercase_sha256_hex(value) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a lowercase 64-character SHA-256 hex string"),
        })
    }
}

fn is_lowercase_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn verify_staged_artifact_body(
    project_home: &Path,
    tmp_path: Option<&str>,
    expected_sha256: &str,
    expected_size_bytes: u64,
) -> StoreResult<()> {
    let tmp_path = tmp_path.ok_or_else(|| StoreError::SchemaInvariant {
        database_kind: "project_state",
        detail: "staged artifact body path is missing before promotion".to_owned(),
    })?;
    let relative = Path::new(tmp_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "staged artifact body path is not a safe relative path".to_owned(),
        });
    }

    let bytes = fs::read(project_home.join(relative))?;
    if u64::try_from(bytes.len()).map_err(|_| StoreError::InvalidInput {
        detail: "staged artifact body size does not fit in u64".to_owned(),
    })? != expected_size_bytes
    {
        return Err(StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "staged artifact body size changed before promotion".to_owned(),
        });
    }
    let actual_sha256 = lowercase_sha256_hex(&bytes);
    if actual_sha256 != expected_sha256 {
        return Err(StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "staged artifact body checksum changed before promotion".to_owned(),
        });
    }
    Ok(())
}

fn lowercase_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    lowercase_hex_bytes(&digest)
}

pub(super) fn lowercase_hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

pub(super) fn validate_json_text(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<Value>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be JSON text: {error}"),
    })?;
    Ok(())
}

pub(super) fn validate_effect_contract_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<Option<ChangeUnitEffectContract>>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be ChangeUnitEffectContract JSON or null: {error}"),
        }
    })?;
    Ok(())
}

pub(super) fn validate_current_close_basis_json(
    field: &'static str,
    text: &str,
) -> StoreResult<()> {
    serde_json::from_str::<CurrentCloseBasis>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be CurrentCloseBasis JSON: {error}"),
    })?;
    Ok(())
}

pub(super) fn validate_judgment_basis_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<PersistedJudgmentBasis>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be JudgmentBasis JSON: {error}"),
        }
    })?;
    Ok(())
}

pub(super) fn validate_user_judgment_request_json(
    field: &'static str,
    text: &str,
) -> StoreResult<()> {
    serde_json::from_str::<PersistedUserJudgmentRequest>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted user judgment request JSON: {error}"),
        }
    })?;
    Ok(())
}

pub(super) fn validate_user_judgment_options_json(
    field: &'static str,
    text: &str,
) -> StoreResult<()> {
    let persisted =
        serde_json::from_str::<PersistedUserJudgmentOptions>(text).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("{field} must be persisted user judgment option JSON: {error}"),
            }
        })?;
    for option in &persisted.options {
        if option.resolution_outcome != option.machine_action.resolution_outcome() {
            return Err(StoreError::InvalidInput {
                detail: format!(
                    "{field} current option resolution_outcome must match machine_action"
                ),
            });
        }
    }
    Ok(())
}

pub(super) fn validate_user_judgment_resolution_json(
    field: &'static str,
    text: &str,
    expected_action: UserJudgmentOptionAction,
    expected_outcome: JudgmentResolutionOutcome,
) -> StoreResult<()> {
    let resolution =
        serde_json::from_str::<PersistedUserJudgmentResolution>(text).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("{field} must be persisted user judgment resolution JSON: {error}"),
            }
        })?;
    if resolution.machine_action != expected_action {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "{field} machine_action must match user_judgments.resolution_machine_action"
            ),
        });
    }
    if resolution.resolution_outcome != expected_outcome {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "{field} resolution_outcome must match user_judgments.resolution_outcome"
            ),
        });
    }
    if resolution.machine_action.resolution_outcome() != expected_outcome {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} machine_action must match resolution_outcome"),
        });
    }
    Ok(())
}

pub(super) fn validate_judgment_rationale_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<JudgmentRationale>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be JudgmentRationale JSON: {error}"),
    })?;
    Ok(())
}

pub(super) fn validate_artifact_producer_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<PersistedArtifactProducer>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted artifact producer JSON: {error}"),
        }
    })?;
    Ok(())
}

pub(super) fn validate_artifact_provenance_metadata_json(
    field: &'static str,
    text: &str,
) -> StoreResult<()> {
    serde_json::from_str::<PersistedArtifactProvenanceMetadata>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted artifact provenance metadata JSON: {error}"),
        }
    })?;
    Ok(())
}

pub(super) fn validate_evidence_coverage_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<Vec<EvidenceCoverageItem>>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted evidence coverage JSON: {error}"),
        }
    })?;
    Ok(())
}

pub(super) fn validate_evidence_source_kind(field: &'static str, value: &str) -> StoreResult<()> {
    serde_json::from_value::<EvidenceSourceKind>(Value::String(value.to_owned()))
        .map(|_| ())
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("{field} must be a supported evidence source kind: {error}"),
        })
}

pub(super) fn validate_evidence_assurance_level(
    field: &'static str,
    value: &str,
) -> StoreResult<()> {
    serde_json::from_value::<EvidenceAssuranceLevel>(Value::String(value.to_owned()))
        .map(|_| ())
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("{field} must be a supported evidence assurance level: {error}"),
        })
}

pub(super) fn validate_state_refs_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<Vec<StateRecordRef>>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted StateRecordRef array JSON: {error}"),
        }
    })?;
    Ok(())
}

pub(super) fn validate_artifact_refs_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<Vec<ArtifactRef>>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be persisted ArtifactRef array JSON: {error}"),
    })?;
    Ok(())
}

pub(super) fn validate_string_list_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<Vec<String>>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be a JSON string array: {error}"),
    })?;
    Ok(())
}

pub(super) fn validate_evidence_metadata_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<PersistedEvidenceMetadata>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted evidence metadata JSON: {error}"),
        }
    })?;
    Ok(())
}

pub(super) fn validate_evidence_observation_tool_metadata_json(
    field: &'static str,
    text: &str,
) -> StoreResult<()> {
    validate_evidence_observation_metadata_json(field, text)
}

pub(super) fn validate_evidence_observation_metadata_json(
    field: &'static str,
    text: &str,
) -> StoreResult<()> {
    serde_json::from_str::<serde_json::Map<String, Value>>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be a JSON object: {error}"),
        }
    })?;
    Ok(())
}

pub(super) fn decode_owner_json_text<T>(
    table: &'static str,
    record_ref: impl Into<String>,
    logical_column: &'static str,
    text: &str,
) -> StoreResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let record_ref = record_ref.into();
    serde_json::from_str(text)
        .map_err(|_| StoreError::corrupt_owner_state_json(table, record_ref, logical_column))
}

pub(super) fn decode_current_close_basis_column(
    record_ref: &str,
    text: Option<&str>,
) -> StoreResult<Option<CurrentCloseBasis>> {
    text.map(|value| decode_current_close_basis_text(record_ref, value))
        .transpose()
}

fn decode_current_close_basis_text(record_ref: &str, text: &str) -> StoreResult<CurrentCloseBasis> {
    serde_json::from_str::<CurrentCloseBasis>(text)
        .map_err(|_| StoreError::corrupt_owner_state_json("tasks", record_ref, "close_basis_json"))
}

pub(super) fn decode_judgment_basis_column(
    record_ref: &str,
    text: &str,
) -> StoreResult<JudgmentBasis> {
    decode_owner_json_text::<PersistedJudgmentBasis>(
        "user_judgments",
        record_ref,
        "basis_json",
        text,
    )
}

pub(super) fn judgment_basis_status_as_str(
    status: JudgmentBasisCompatibilityStatus,
) -> &'static str {
    match status {
        JudgmentBasisCompatibilityStatus::Current => "current",
        JudgmentBasisCompatibilityStatus::Stale => "stale",
        JudgmentBasisCompatibilityStatus::Superseded => "superseded",
    }
}

pub(super) fn judgment_resolution_outcome_as_str(
    outcome: JudgmentResolutionOutcome,
) -> &'static str {
    match outcome {
        JudgmentResolutionOutcome::Accepted => "accepted",
        JudgmentResolutionOutcome::Rejected => "rejected",
        JudgmentResolutionOutcome::Deferred => "deferred",
    }
}

pub(super) fn judgment_machine_action_as_str(action: UserJudgmentOptionAction) -> &'static str {
    match action {
        UserJudgmentOptionAction::Accept => "accept",
        UserJudgmentOptionAction::Reject => "reject",
        UserJudgmentOptionAction::Defer => "defer",
    }
}

pub(super) fn parse_judgment_basis_status(
    record_ref: &str,
    logical_column: &'static str,
    value: &str,
) -> StoreResult<JudgmentBasisCompatibilityStatus> {
    match value {
        "current" => Ok(JudgmentBasisCompatibilityStatus::Current),
        "stale" => Ok(JudgmentBasisCompatibilityStatus::Stale),
        "superseded" => Ok(JudgmentBasisCompatibilityStatus::Superseded),
        _ => Err(StoreError::corrupt_owner_state_value(
            "user_judgments",
            record_ref,
            logical_column,
        )),
    }
}

pub(super) fn nonnegative_i64_to_u64(
    field: &'static str,
    value: i64,
) -> Result<u64, rusqlite::Error> {
    u64::try_from(value).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            format!("{field} must be nonnegative").into(),
        )
    })
}

pub(super) fn u64_to_i64(field: &'static str, value: u64) -> StoreResult<i64> {
    i64::try_from(value).map_err(|_| StoreError::InvalidInput {
        detail: format!("{field} does not fit in SQLite integer"),
    })
}
