use std::{
    fs,
    path::{Component, Path},
};

use chrono::Duration;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use volicord_types::{
    canonical_git_object_id, canonical_json_string, is_canonical_sha256_digest, ActorSource,
    ArtifactRef, ChangeUnitEffectContract, CurrentCloseBasis, EvidenceAssuranceLevel,
    EvidenceCoverageItem, EvidenceSourceKind, OperationCategory, PersistedArtifactProducer,
    PersistedArtifactProvenanceMetadata, PersistedCloseSummary, PersistedEvidenceMetadata,
    PersistedEvidenceObservationAuthority, PersistedUserActionRequest,
    PersistedUserActionResolution, ProjectContinuityKind, ProjectContinuityStatus,
    ProjectEnforcementProfile, ProjectEnforcementProfileSource, ProjectEnforcementProfileStatus,
    SourceRef, StateRecordRef, UserActionBasis, UserActionBasisStatus, UserActionChannelKind,
    UserActionKind, UserActionRequestBody, UserActionRequiredFor, UserActionResolutionBody,
    UtcTimestamp, BASELINE_COOPERATIVE_ENFORCEMENT_PROFILE_ID,
    USER_ACTION_EVIDENCE_OBSERVATION_TTL_MINUTES,
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
    if let Some(task_id) = &event.task_id {
        validate_identifier("task_id", task_id)?;
    }
    if event.task_id.is_none()
        && (event.event_kind != "project_workflow_policy_applied" || event.change_unit_id.is_some())
    {
        return Err(StoreError::InvalidInput {
            detail: "only project_workflow_policy_applied may be a project-scoped authority event"
                .to_owned(),
        });
    }
    if event.task_id.is_some() && event.event_kind == "project_workflow_policy_applied" {
        return Err(StoreError::InvalidInput {
            detail: "project_workflow_policy_applied must be project-scoped".to_owned(),
        });
    }
    validate_identifier("event_kind", &event.event_kind)?;
    validate_json_text("authority_events.payload_json", &event.event_payload_json)
}

pub(super) fn validate_replay_context(context: &VerifiedReplayContext) -> StoreResult<()> {
    validate_canonical_replay_identity(
        &context.actor_source,
        &context.operation_category,
        context.verification_basis.as_deref(),
        context.git_workspace_context_json.as_deref(),
    )
    .map_err(|failure| StoreError::InvalidInput {
        detail: format!("{} {}", failure.input_field, failure.detail),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReplayContextFieldKind {
    Value,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ReplayContextValidationFailure {
    pub(super) input_field: &'static str,
    pub(super) logical_column: &'static str,
    pub(super) detail: &'static str,
    pub(super) field_kind: ReplayContextFieldKind,
}

pub(super) fn validate_canonical_replay_identity(
    actor_source: &str,
    operation_category: &str,
    verification_basis: Option<&str>,
    git_workspace_context_json: Option<&str>,
) -> Result<(), ReplayContextValidationFailure> {
    let actor_source_is_canonical = actor_source
        .parse::<ActorSource>()
        .is_ok_and(|parsed| parsed.to_canonical_string() == actor_source);
    if !actor_source_is_canonical {
        return Err(ReplayContextValidationFailure {
            input_field: "actor_source",
            logical_column: "actor_source",
            detail: "must be a canonical ActorSource value",
            field_kind: ReplayContextFieldKind::Value,
        });
    }

    let operation_category_is_canonical =
        serde_json::from_value::<OperationCategory>(Value::String(operation_category.to_owned()))
            .is_ok_and(|parsed| parsed.as_str() == operation_category);
    if !operation_category_is_canonical {
        return Err(ReplayContextValidationFailure {
            input_field: "operation_category",
            logical_column: "operation_category",
            detail: "must be a canonical supported OperationCategory value",
            field_kind: ReplayContextFieldKind::Value,
        });
    }

    if verification_basis.is_some_and(|basis| basis.trim().is_empty()) {
        return Err(ReplayContextValidationFailure {
            input_field: "verification_basis",
            logical_column: "verification_basis",
            detail: "must not be empty",
            field_kind: ReplayContextFieldKind::Value,
        });
    }

    if let Some(context) = git_workspace_context_json {
        parse_canonical_git_workspace_context(context).map_err(|detail| {
            ReplayContextValidationFailure {
                input_field: "tool_invocations.git_workspace_context_json",
                logical_column: "git_workspace_context_json",
                detail,
                field_kind: ReplayContextFieldKind::Json,
            }
        })?;
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedGitWorkspaceReplayContext {
    git_common_dir: String,
    worktree_id: String,
    branch_ref: Option<String>,
    head_sha: Option<String>,
    workspace_fingerprint: String,
}

fn parse_canonical_git_workspace_context(
    text: &str,
) -> Result<PersistedGitWorkspaceReplayContext, &'static str> {
    let context = serde_json::from_str::<PersistedGitWorkspaceReplayContext>(text)
        .map_err(|_| "must have the strict Git workspace-context JSON shape")?;
    if context.git_common_dir.trim().is_empty() || !Path::new(&context.git_common_dir).is_absolute()
    {
        return Err("must contain an absolute non-empty git_common_dir");
    }
    if !is_canonical_sha256_digest(&context.worktree_id) {
        return Err("must contain a valid worktree_id");
    }
    if context.branch_ref.as_ref().is_some_and(|reference| {
        !reference.starts_with("refs/")
            || reference.contains(['\0', '\n', '\r'])
            || reference.trim() != reference
    }) {
        return Err("must contain a valid branch_ref or null");
    }
    if context
        .head_sha
        .as_ref()
        .is_some_and(|sha| match canonical_git_object_id(sha) {
            Ok(canonical) => canonical != *sha,
            Err(_) => true,
        })
    {
        return Err("must contain a canonical lowercase head_sha or null");
    }
    if !is_canonical_sha256_digest(&context.workspace_fingerprint) {
        return Err("must contain a valid workspace_fingerprint");
    }
    if canonical_json_string(&context).map_err(|_| "must be serializable")? != text {
        return Err("must use canonical JSON serialization");
    }
    Ok(context)
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
        .and_then(|timestamp| {
            timestamp
                .ensure_canonical_rfc3339_representable()
                .map_err(|_| volicord_types::UtcTimestampParseError)
        })
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

pub(super) fn validate_persisted_close_summary_json(
    field: &'static str,
    text: &str,
) -> StoreResult<()> {
    serde_json::from_str::<PersistedCloseSummary>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be canonical persisted close-summary JSON: {error}"),
        }
    })?;
    Ok(())
}

pub(super) fn validate_user_action_basis_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<UserActionBasis>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be UserActionBasis JSON: {error}"),
    })?;
    Ok(())
}

pub(super) fn user_action_basis_json_with_status(
    text: &str,
    status: UserActionBasisStatus,
) -> StoreResult<String> {
    let mut basis = serde_json::from_str::<UserActionBasis>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("user-action basis must be UserActionBasis JSON: {error}"),
        }
    })?;
    match &mut basis {
        UserActionBasis::Choice(choice) => choice.coordinates.compatibility_status = status,
        UserActionBasis::EvidenceObservation(observation) => {
            observation.coordinates.compatibility_status = status;
        }
    }
    canonical_json_string(&basis).map_err(|error| StoreError::InvalidInput {
        detail: format!("user-action basis must be canonically serializable: {error}"),
    })
}

pub(super) fn validate_persisted_user_action_request_json(
    field: &'static str,
    text: &str,
) -> StoreResult<()> {
    let request = serde_json::from_str::<PersistedUserActionRequest>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted user-action request JSON: {error}"),
        }
    })?;
    request
        .body
        .validate_bounds()
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("{field} violates the persisted user-action bounds: {error}"),
        })?;
    validate_user_action_required_for(&request.required_for, field)?;
    Ok(())
}

pub(super) fn validate_persisted_user_action_resolution_json(
    field: &'static str,
    text: &str,
) -> StoreResult<()> {
    let resolution =
        serde_json::from_str::<PersistedUserActionResolution>(text).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("{field} must be persisted user-action resolution JSON: {error}"),
            }
        })?;
    resolution
        .validate()
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("{field} violates the persisted user-action bounds: {error}"),
        })?;
    Ok(())
}

pub(super) fn validate_user_action_required_for_json(
    field: &'static str,
    text: &str,
) -> StoreResult<()> {
    let required_for =
        serde_json::from_str::<Vec<UserActionRequiredFor>>(text).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("{field} must be user-action required-for JSON: {error}"),
            }
        })?;
    validate_user_action_required_for(&required_for, field)
}

fn validate_user_action_required_for(
    required_for: &[UserActionRequiredFor],
    field: &'static str,
) -> StoreResult<()> {
    if required_for.is_empty()
        || required_for
            .iter()
            .enumerate()
            .any(|(index, value)| required_for[..index].contains(value))
    {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must be nonempty and contain unique values"),
        });
    }
    Ok(())
}

pub(super) struct UserActionRequestColumnFacts<'a> {
    pub task_id: &'a str,
    pub change_unit_id: Option<&'a str>,
    pub request_json: &'a str,
    pub basis_json: &'a str,
    pub required_for_json: &'a str,
    pub requested_at: &'a str,
    pub expires_at: Option<&'a str>,
    pub action_kind: UserActionKind,
    pub basis_status: UserActionBasisStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UserActionTimestampOrderFailure {
    ExpiryNotAfterRequest,
    ResolutionBeforeRequest,
    ResolutionAtOrAfterExpiry,
}

pub(super) fn validate_user_action_timestamp_order(
    requested_at: &UtcTimestamp,
    expires_at: Option<&UtcTimestamp>,
    resolved_at: Option<&UtcTimestamp>,
) -> Result<(), UserActionTimestampOrderFailure> {
    if expires_at.is_some_and(|expires_at| expires_at <= requested_at) {
        return Err(UserActionTimestampOrderFailure::ExpiryNotAfterRequest);
    }
    if let Some(resolved_at) = resolved_at {
        if resolved_at < requested_at {
            return Err(UserActionTimestampOrderFailure::ResolutionBeforeRequest);
        }
        if expires_at.is_some_and(|expires_at| resolved_at >= expires_at) {
            return Err(UserActionTimestampOrderFailure::ResolutionAtOrAfterExpiry);
        }
    }
    Ok(())
}

pub(super) fn validate_user_action_request_column_agreement(
    facts: UserActionRequestColumnFacts<'_>,
) -> StoreResult<()> {
    validate_user_action_request_column_agreement_inner(facts).map_err(|failure| {
        StoreError::InvalidInput {
            detail: failure.detail,
        }
    })
}

pub(super) fn validate_stored_user_action_request_column_agreement(
    record_ref: &str,
    facts: UserActionRequestColumnFacts<'_>,
) -> StoreResult<()> {
    validate_user_action_request_column_agreement_inner(facts).map_err(|failure| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            record_ref,
            failure.logical_column,
        )
    })
}

struct UserActionRequestColumnFailure {
    logical_column: &'static str,
    detail: String,
}

impl UserActionRequestColumnFailure {
    fn new(logical_column: &'static str, detail: impl Into<String>) -> Self {
        Self {
            logical_column,
            detail: detail.into(),
        }
    }
}

fn validate_user_action_request_column_agreement_inner(
    facts: UserActionRequestColumnFacts<'_>,
) -> Result<(), UserActionRequestColumnFailure> {
    let request = serde_json::from_str::<PersistedUserActionRequest>(facts.request_json)
        .map_err(|error| {
            UserActionRequestColumnFailure::new(
                "request_json",
                format!(
                    "user_action_requests.request_json must be persisted user-action request JSON: {error}"
                ),
            )
        })?;
    let basis = serde_json::from_str::<UserActionBasis>(facts.basis_json).map_err(|error| {
        UserActionRequestColumnFailure::new(
            "basis_json",
            format!("user_action_requests.basis_json must be UserActionBasis JSON: {error}"),
        )
    })?;
    let required_for = serde_json::from_str::<Vec<UserActionRequiredFor>>(facts.required_for_json)
        .map_err(|error| {
            UserActionRequestColumnFailure::new(
                "required_for_json",
                format!(
                "user_action_requests.required_for_json must be user-action required-for JSON: {error}"
                ),
            )
        })?;
    let requested_at = UtcTimestamp::parse(facts.requested_at).map_err(|_| {
        UserActionRequestColumnFailure::new(
            "requested_at",
            "user_action_requests.requested_at must be a valid RFC 3339 timestamp",
        )
    })?;
    let expires_at = facts
        .expires_at
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| {
            UserActionRequestColumnFailure::new(
                "expires_at",
                "user_action_requests.expires_at must be a valid RFC 3339 timestamp",
            )
        })?;
    request
        .body
        .validate_bounds()
        .map_err(|error| {
            UserActionRequestColumnFailure::new(
                "request_json",
                format!(
                "user_action_requests.request_json violates the persisted user-action bounds: {error}"
                ),
            )
        })?;
    validate_user_action_required_for(&request.required_for, "user_action_requests.request_json")
        .map_err(|_| {
            UserActionRequestColumnFailure::new(
                "request_json",
                "user_action_requests.request_json required_for must be nonempty and contain unique values",
            )
        })?;
    validate_user_action_required_for(&required_for, "user_action_requests.required_for_json")
        .map_err(|_| {
            UserActionRequestColumnFailure::new(
                "required_for_json",
                "user_action_requests.required_for_json must be nonempty and contain unique values",
            )
        })?;
    let matching_family = match (&request.body, &basis, facts.action_kind) {
        (UserActionRequestBody::Choice(body), UserActionBasis::Choice(basis), kind)
            if kind != UserActionKind::EvidenceObservation =>
        {
            body.sensitive_action_scope == basis.sensitive_action_scope
        }
        (
            UserActionRequestBody::EvidenceObservation(body),
            UserActionBasis::EvidenceObservation(basis),
            UserActionKind::EvidenceObservation,
        ) => {
            body.target_candidates == basis.target_candidates
                && body.artifact_candidates == basis.artifact_candidates
        }
        _ => false,
    };
    let coordinates = basis.coordinates();
    if request.body.action_kind() != facts.action_kind {
        return Err(UserActionRequestColumnFailure::new(
            "request_json",
            "user-action request JSON must agree with its action kind",
        ));
    }
    if basis.compatibility_status() != facts.basis_status
        || coordinates.task_id.as_str() != facts.task_id
        || coordinates.change_unit_id.as_ref().map(|id| id.as_str()) != facts.change_unit_id
        || !matching_family
    {
        return Err(UserActionRequestColumnFailure::new(
            "basis_json",
            "user-action basis JSON must agree with its request and scalar storage columns",
        ));
    }
    if request.required_for != required_for {
        return Err(UserActionRequestColumnFailure::new(
            "required_for_json",
            "user-action required-for JSON must agree with the canonical request",
        ));
    }
    if !request
        .required_for
        .iter()
        .copied()
        .all(|target| facts.action_kind.is_compatible_with_required_for(target))
    {
        return Err(UserActionRequestColumnFailure::new(
            "request_json",
            "user_action_requests.request_json required_for contains an operation incompatible with its action kind",
        ));
    }
    if request.expires_at.as_ref() != expires_at.as_ref() {
        return Err(UserActionRequestColumnFailure::new(
            "request_json",
            "user-action request JSON must agree with the stored expiry",
        ));
    }
    if facts.action_kind == UserActionKind::EvidenceObservation {
        let expected_expires_at = requested_at
            .checked_add(Duration::minutes(
                USER_ACTION_EVIDENCE_OBSERVATION_TTL_MINUTES,
            ))
            .map_err(|_| {
                UserActionRequestColumnFailure::new(
                    "expires_at",
                    "evidence-observation user_action_requests.expires_at must be exactly 15 minutes after user_action_requests.requested_at",
                )
            })?;
        if expires_at.as_ref() != Some(&expected_expires_at) {
            return Err(UserActionRequestColumnFailure::new(
                "expires_at",
                "evidence-observation user_action_requests.expires_at must be exactly 15 minutes after user_action_requests.requested_at",
            ));
        }
    } else if validate_user_action_timestamp_order(&requested_at, expires_at.as_ref(), None)
        .is_err()
    {
        return Err(UserActionRequestColumnFailure::new(
            "expires_at",
            "user_action_requests.expires_at must be later than user_action_requests.requested_at",
        ));
    }
    Ok(())
}

pub(super) fn validate_user_action_resolution_column_agreement(
    resolution_json: &str,
    action_kind: UserActionKind,
    _user_action_resolution_id: &str,
) -> StoreResult<()> {
    let resolution = serde_json::from_str::<UserActionResolutionBody>(resolution_json).map_err(
        |error| StoreError::InvalidInput {
            detail: format!(
                "user_action_resolutions.resolution_json must be persisted user-action resolution JSON: {error}"
            ),
        },
    )?;
    resolution
        .validate()
        .map_err(|error| StoreError::InvalidInput {
            detail: format!(
                "user_action_resolutions.resolution_json violates the persisted user-action bounds: {error}"
            ),
        })?;
    let agrees = match resolution {
        UserActionResolutionBody::Choice { .. } => {
            action_kind != UserActionKind::EvidenceObservation
        }
        UserActionResolutionBody::EvidenceObservation { .. } => {
            action_kind == UserActionKind::EvidenceObservation
        }
    };
    if agrees {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "user-action resolution JSON must agree with its action kind".to_owned(),
        })
    }
}

pub(super) fn validate_user_action_resolution_provenance(
    channel_kind: UserActionChannelKind,
    resolved_by_actor_source: &str,
    resolved_verification_basis: &str,
    resolved_assurance_level: &str,
) -> StoreResult<()> {
    if resolved_by_actor_source != "local_user"
        || resolved_verification_basis != channel_kind.verification_basis()
        || resolved_assurance_level.trim().is_empty()
    {
        return Err(StoreError::InvalidInput {
            detail: "user-action resolution provenance must match its verified channel".to_owned(),
        });
    }
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

pub(super) fn validate_source_refs_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<Vec<SourceRef>>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be persisted SourceRef array JSON: {error}"),
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
    let value: Value = serde_json::from_str(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be valid JSON: {error}"),
    })?;
    if value.is_object() {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a JSON object"),
        })
    }
}

pub(super) fn validate_evidence_observation_metadata_json(
    field: &'static str,
    text: &str,
) -> StoreResult<()> {
    serde_json::from_str::<PersistedEvidenceObservationAuthority>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!(
                "{field} must be persisted evidence observation authority JSON: {error}"
            ),
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

pub(super) fn user_action_basis_status_as_str(status: UserActionBasisStatus) -> &'static str {
    match status {
        UserActionBasisStatus::Current => "current",
        UserActionBasisStatus::Stale => "stale",
        UserActionBasisStatus::Superseded => "superseded",
    }
}

pub(super) fn user_action_kind_as_str(kind: UserActionKind) -> &'static str {
    match kind {
        UserActionKind::ProductDecision => "product_decision",
        UserActionKind::TechnicalDecision => "technical_decision",
        UserActionKind::ScopeDecision => "scope_decision",
        UserActionKind::SensitiveApproval => "sensitive_approval",
        UserActionKind::FinalAcceptance => "final_acceptance",
        UserActionKind::ResidualRiskAcceptance => "residual_risk_acceptance",
        UserActionKind::Cancellation => "cancellation",
        UserActionKind::EvidenceObservation => "evidence_observation",
    }
}

pub(super) fn user_action_channel_kind_as_str(kind: UserActionChannelKind) -> &'static str {
    match kind {
        UserActionChannelKind::Cli => "cli",
    }
}

pub(super) fn parse_user_action_basis_status(
    record_ref: &str,
    logical_column: &'static str,
    value: &str,
) -> StoreResult<UserActionBasisStatus> {
    match value {
        "current" => Ok(UserActionBasisStatus::Current),
        "stale" => Ok(UserActionBasisStatus::Stale),
        "superseded" => Ok(UserActionBasisStatus::Superseded),
        _ => Err(StoreError::corrupt_owner_state_value(
            "user_action_requests",
            record_ref,
            logical_column,
        )),
    }
}

pub(super) fn parse_user_action_kind(
    record_ref: &str,
    logical_column: &'static str,
    value: &str,
) -> StoreResult<UserActionKind> {
    serde_json::from_value(Value::String(value.to_owned())).map_err(|_| {
        StoreError::corrupt_owner_state_value("user_action_requests", record_ref, logical_column)
    })
}

pub(super) fn parse_user_action_channel_kind(
    record_ref: &str,
    logical_column: &'static str,
    value: &str,
) -> StoreResult<UserActionChannelKind> {
    serde_json::from_value(Value::String(value.to_owned())).map_err(|_| {
        StoreError::corrupt_owner_state_value("user_action_resolutions", record_ref, logical_column)
    })
}

pub(super) fn validate_stored_timestamp(field: &'static str, value: &str) -> StoreResult<()> {
    UtcTimestamp::parse(value)
        .and_then(|timestamp| {
            timestamp
                .ensure_canonical_rfc3339_representable()
                .map_err(|_| volicord_types::UtcTimestampParseError)
        })
        .map_err(|_| StoreError::CorruptStoredValue {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            field,
        })
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

#[cfg(test)]
mod source_ref_tests {
    use super::*;

    #[test]
    fn persisted_source_refs_require_a_strict_tagged_shape() {
        assert!(validate_source_refs_json(
            "source_refs_json",
            r#"[{"source_kind":"user_context","source":{"context_id":"message_1"}}]"#,
        )
        .is_ok());
        assert!(validate_source_refs_json(
            "source_refs_json",
            r#"[{"source_kind":"user_context","source":{"context_id":"message_1","uri":"https://example.invalid"}}]"#,
        )
        .is_err());
    }

    #[test]
    fn persisted_workspace_context_requires_canonical_git_object_id_spelling() {
        let canonical = canonical_json_string(&serde_json::json!({
            "git_common_dir": "/tmp/volicord-git-object-id/.git",
            "worktree_id": format!("sha256:{}", "1".repeat(64)),
            "branch_ref": "refs/heads/test",
            "head_sha": "a".repeat(40),
            "workspace_fingerprint": format!("sha256:{}", "2".repeat(64)),
        }))
        .expect("fixture must serialize");
        assert!(parse_canonical_git_workspace_context(&canonical).is_ok());

        let uppercase = canonical_json_string(&serde_json::json!({
            "git_common_dir": "/tmp/volicord-git-object-id/.git",
            "worktree_id": format!("sha256:{}", "1".repeat(64)),
            "branch_ref": "refs/heads/test",
            "head_sha": "A".repeat(40),
            "workspace_fingerprint": format!("sha256:{}", "2".repeat(64)),
        }))
        .expect("fixture must serialize");
        assert!(parse_canonical_git_workspace_context(&uppercase).is_err());

        for (worktree_id, workspace_fingerprint) in [
            (
                format!("sha256:{}", "A".repeat(64)),
                format!("sha256:{}", "2".repeat(64)),
            ),
            (
                format!("sha256:{}", "1".repeat(64)),
                format!("sha256:{}", "F".repeat(64)),
            ),
        ] {
            let uppercase_digest = canonical_json_string(&serde_json::json!({
                "git_common_dir": "/tmp/volicord-git-object-id/.git",
                "worktree_id": worktree_id,
                "branch_ref": "refs/heads/test",
                "head_sha": "a".repeat(40),
                "workspace_fingerprint": workspace_fingerprint,
            }))
            .expect("fixture must serialize");
            assert!(parse_canonical_git_workspace_context(&uppercase_digest).is_err());
        }
    }
}
