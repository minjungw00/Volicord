use std::{collections::BTreeSet, path::Path};

use chrono::{DateTime, Duration, Utc};
use harness_store::{core_pipeline::WriteAuthorizationRecord, StoreError};
use harness_types::{
    BaselineRef, ChangeUnitId, DryRunSummary, GuaranteeDisplay, GuaranteeLevel,
    JudgmentBasisCompatibilityStatus, JudgmentKind, JudgmentResolutionOutcome, PlannedBlocker,
    PlannedBlockerSourceKind, PlannedEffect, PrepareWriteDecision, SensitiveActionScope,
    StateRecordRef, TaskId, UserJudgmentStatus, UtcTimestamp, WriteDecisionCategory,
    WriteDecisionReason,
};
use serde_json::Value;

use crate::policy::{
    close_readiness::{judgment_has_current_basis, JudgmentAuthority},
    path::{normalize_product_paths, path_is_within, ProductPathError},
};

const WRITE_AUTHORIZATION_LIFETIME_MINUTES: i64 = 15;

pub(crate) fn write_authorization_expires_at(created_at: DateTime<Utc>) -> DateTime<Utc> {
    created_at + Duration::minutes(WRITE_AUTHORIZATION_LIFETIME_MINUTES)
}

pub(crate) fn write_authorization_is_expired(
    record: &WriteAuthorizationRecord,
    now: DateTime<Utc>,
) -> Result<bool, StoreError> {
    Ok(UtcTimestamp::from_datetime(now) >= effective_write_authorization_expiration(record)?)
}

pub(crate) fn effective_write_authorization_expiration(
    record: &WriteAuthorizationRecord,
) -> Result<UtcTimestamp, StoreError> {
    let stored_expires_at = parse_write_authorization_timestamp(record, "expires_at")?;
    let created_at = parse_write_authorization_timestamp(record, "created_at")?;
    Ok(std::cmp::min(
        stored_expires_at,
        UtcTimestamp::from_datetime(write_authorization_expires_at(*created_at.as_datetime())),
    ))
}

fn parse_write_authorization_timestamp(
    record: &WriteAuthorizationRecord,
    logical_column: &'static str,
) -> Result<UtcTimestamp, StoreError> {
    let raw = match logical_column {
        "created_at" => &record.created_at,
        "expires_at" => &record.expires_at,
        _ => {
            return Err(StoreError::corrupt_owner_state_value(
                "write_authorizations",
                record.write_authorization_id.clone(),
                logical_column,
            ));
        }
    };
    UtcTimestamp::parse(raw).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "write_authorizations",
            record.write_authorization_id.clone(),
            logical_column,
        )
    })
}

pub(crate) fn surface_supports_prepare_write(capability_profile: &Value) -> bool {
    if capability_profile
        .get("supported_access_classes")
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values
                .iter()
                .any(|value| value.as_str() == Some("write_authorization"))
        })
    {
        return true;
    }
    if capability_profile
        .get("access_class")
        .and_then(Value::as_str)
        == Some("write_authorization")
    {
        return true;
    }
    if capability_profile
        .get("write_authorization")
        .and_then(Value::as_bool)
        == Some(true)
    {
        return true;
    }
    capability_profile
        .pointer("/capabilities/write_authorization")
        .and_then(Value::as_bool)
        == Some(true)
}

pub(crate) fn prepare_write_decision(reasons: &[WriteDecisionReason]) -> PrepareWriteDecision {
    if reasons.is_empty() {
        PrepareWriteDecision::Allowed
    } else if reasons
        .iter()
        .any(|reason| reason.code == "user_judgment_unresolved")
    {
        PrepareWriteDecision::DecisionRequired
    } else if reasons
        .iter()
        .any(|reason| reason.code == "sensitive_approval_missing")
    {
        PrepareWriteDecision::ApprovalRequired
    } else {
        PrepareWriteDecision::Blocked
    }
}

pub(crate) fn prepare_write_dry_run_summary(
    allowed: bool,
    reasons: &[WriteDecisionReason],
    _write_authorization_ref: Option<StateRecordRef>,
    _guarantee_display: Option<GuaranteeDisplay>,
) -> DryRunSummary {
    DryRunSummary {
        planned_effects: if allowed {
            vec![PlannedEffect {
                target_kind: "write_authorization".to_owned(),
                action: "would_create".to_owned(),
                description: "Prepare write would create one active Write Authorization."
                    .to_owned(),
            }]
        } else {
            Vec::new()
        },
        would_blockers: reasons
            .iter()
            .map(|reason| PlannedBlocker {
                source_kind: PlannedBlockerSourceKind::WriteDecision,
                category: write_decision_category_value(reason.category).to_owned(),
                code: reason.code.clone(),
                message: reason.message.clone(),
                related_refs: reason.related_refs.clone(),
            })
            .collect(),
        would_errors: Vec::new(),
        next_actions: Vec::new(),
        diagnostics: Vec::new(),
    }
}

pub(crate) fn write_decision_reason(
    category: WriteDecisionCategory,
    code: &'static str,
    message: &'static str,
    related_refs: Vec<StateRecordRef>,
) -> WriteDecisionReason {
    WriteDecisionReason {
        category,
        code: code.to_owned(),
        message: message.to_owned(),
        related_refs,
    }
}

fn write_decision_category_value(category: WriteDecisionCategory) -> &'static str {
    match category {
        WriteDecisionCategory::Scope => "scope",
        WriteDecisionCategory::UserJudgment => "user_judgment",
        WriteDecisionCategory::SensitiveApproval => "sensitive_approval",
        WriteDecisionCategory::WriteCompatibility => "write_compatibility",
        WriteDecisionCategory::Baseline => "baseline",
        WriteDecisionCategory::SurfaceCapability => "surface_capability",
    }
}

pub(crate) fn write_authorization_guarantee() -> GuaranteeDisplay {
    GuaranteeDisplay {
        level: GuaranteeLevel::Cooperative,
        basis: "Write Authorization is a Harness compatibility record, not OS permission."
            .to_owned(),
        capability_refs: Vec::new(),
    }
}

pub(crate) struct SensitiveApprovalRequirement<'a> {
    pub(crate) task_id: &'a TaskId,
    pub(crate) change_unit_id: &'a ChangeUnitId,
    pub(crate) scope_revision: u64,
    pub(crate) operation: &'a str,
    pub(crate) normalized_paths: &'a [String],
    pub(crate) sensitive_categories: &'a [String],
    pub(crate) baseline_ref: Option<&'a BaselineRef>,
    pub(crate) now: &'a UtcTimestamp,
    pub(crate) repo_root: &'a Path,
}

pub(crate) fn current_sensitive_approval(
    judgment: &JudgmentAuthority,
    requirement: &SensitiveApprovalRequirement<'_>,
) -> bool {
    if !judgment_has_current_basis(judgment)
        || judgment.status != UserJudgmentStatus::Resolved
        || judgment.resolution_outcome != Some(JudgmentResolutionOutcome::Accepted)
        || judgment.judgment_kind != JudgmentKind::SensitiveApproval
    {
        return false;
    }
    let Some(basis) = judgment.basis.as_ref() else {
        return false;
    };
    if basis.compatibility_status != JudgmentBasisCompatibilityStatus::Current
        || basis.task_id != *requirement.task_id
        || basis.change_unit_id.as_ref() != Some(requirement.change_unit_id)
        || basis.scope_revision != requirement.scope_revision
        || basis.baseline_ref.as_ref() != requirement.baseline_ref
    {
        return false;
    }
    let Some(scope) = basis.sensitive_action_scope.as_ref() else {
        return false;
    };
    sensitive_action_scope_matches_requirement(scope, requirement)
}

pub(crate) fn sensitive_action_scope_matches_requirement(
    scope: &SensitiveActionScope,
    requirement: &SensitiveApprovalRequirement<'_>,
) -> bool {
    if scope
        .expires_at
        .as_ref()
        .is_some_and(|expires_at| requirement.now >= expires_at)
    {
        return false;
    }
    if scope.action_kind != normalize_sensitive_text(requirement.operation) {
        return false;
    }
    if !category_set(requirement.sensitive_categories)
        .is_subset(&category_set(&scope.sensitive_categories))
    {
        return false;
    }
    let Ok(approved_paths) = normalize_product_paths(requirement.repo_root, &scope.intended_paths)
    else {
        return false;
    };
    requirement.normalized_paths.iter().all(|path| {
        approved_paths
            .iter()
            .any(|approved| path_is_within(path, approved))
    })
}

pub(crate) fn normalize_sensitive_action_scope(
    repo_root: &Path,
    scope: &SensitiveActionScope,
) -> Result<SensitiveActionScope, ProductPathError> {
    Ok(SensitiveActionScope {
        action_kind: normalize_sensitive_text(&scope.action_kind),
        description: normalize_sensitive_text(&scope.description),
        intended_paths: normalize_product_paths(repo_root, &scope.intended_paths)?
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        sensitive_categories: normalized_string_set(&scope.sensitive_categories),
        command_or_tool_summary: scope
            .command_or_tool_summary
            .as_ref()
            .map(|value| normalize_sensitive_text(value))
            .filter(|value| !value.is_empty())
            .into(),
        network_or_host_summary: scope
            .network_or_host_summary
            .as_ref()
            .map(|value| normalize_sensitive_text(value))
            .filter(|value| !value.is_empty())
            .into(),
        secret_or_credential_summary: scope
            .secret_or_credential_summary
            .as_ref()
            .map(|value| normalize_sensitive_text(value))
            .filter(|value| !value.is_empty())
            .into(),
        capability_claim: normalize_sensitive_text(&scope.capability_claim),
        expires_at: scope.expires_at.clone(),
    })
}

pub(crate) fn normalized_string_set(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| normalize_sensitive_text(value))
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn category_set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn normalize_sensitive_text(value: &str) -> String {
    value.trim().to_owned()
}
