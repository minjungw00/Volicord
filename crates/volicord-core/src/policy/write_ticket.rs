use std::{collections::BTreeSet, path::Path};

use chrono::{DateTime, Utc};
use volicord_store::{core_pipeline::WriteTicketRecord, StoreError};
use volicord_types::ids::{BaselineRef, ChangeUnitId, TaskId};
use volicord_types::schema::{
    DryRunSummary, GuaranteeDisplay, ObservedChanges, PlannedBlocker, PlannedEffect,
    SensitiveActionScope, StateRecordRef, WriteDecisionReason, WriteTicketAttemptScope,
};
use volicord_types::values::{
    PlannedBlockerSourceKind, PrepareWriteDecision, UserActionKind, UserActionRequiredFor,
    UtcTimestamp, WriteDecisionCategory,
};

use crate::policy::{
    close_readiness::{accepted_current_user_authority, UserActionAuthority},
    path::{normalize_product_paths, path_is_within, paths_are_authorized, ProductPathError},
};

pub(crate) fn write_ticket_is_idle_expired(
    record: &WriteTicketRecord,
    now: DateTime<Utc>,
) -> Result<bool, StoreError> {
    let Some(raw) = record.idle_expires_at.as_ref() else {
        return Ok(false);
    };
    let corrupt = || {
        StoreError::corrupt_owner_state_value(
            "write_tickets",
            record.write_ticket_id.clone(),
            "idle_expires_at",
        )
    };
    let timestamp = UtcTimestamp::parse(raw).map_err(|_| corrupt())?;
    timestamp
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| corrupt())?;
    Ok(UtcTimestamp::from_datetime(now) >= timestamp)
}

pub(crate) fn prepare_write_decision(reasons: &[WriteDecisionReason]) -> PrepareWriteDecision {
    if reasons.is_empty() {
        PrepareWriteDecision::Allowed
    } else if reasons
        .iter()
        .any(|reason| reason.code == "user_action_unresolved")
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
    _write_ticket_ref: Option<StateRecordRef>,
    _guarantee_display: Option<GuaranteeDisplay>,
) -> DryRunSummary {
    DryRunSummary {
        planned_effects: if allowed {
            vec![PlannedEffect {
                target_kind: "write_ticket".to_owned(),
                action: "would_issue".to_owned(),
                description: "Prepare write would issue one open write ticket.".to_owned(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RunWriteTicketMismatch {
    pub(crate) reason: &'static str,
    pub(crate) message: &'static str,
}

pub(crate) struct RunWriteTicketAttempt<'a> {
    pub(crate) task_id: &'a TaskId,
    pub(crate) change_unit_id: &'a ChangeUnitId,
    pub(crate) baseline_ref: &'a BaselineRef,
    pub(crate) performed_operation: Option<&'a str>,
    pub(crate) performed_operation_required: bool,
    pub(crate) observed_changes: &'a ObservedChanges,
    pub(crate) normalized_scope_paths: &'a [String],
}

pub(crate) fn run_write_ticket_mismatch(
    record: &WriteTicketRecord,
    scope: &WriteTicketAttemptScope,
    attempt: RunWriteTicketAttempt<'_>,
) -> Option<RunWriteTicketMismatch> {
    if record.task_id != attempt.task_id.as_str() || scope.task_id != *attempt.task_id {
        return Some(run_mismatch(
            "task_mismatch",
            "write ticket task is not compatible with the recorded run",
        ));
    }
    if record.change_unit_id != attempt.change_unit_id.as_str()
        || scope.change_unit_id != *attempt.change_unit_id
    {
        return Some(run_mismatch(
            "change_unit_mismatch",
            "write ticket Change Unit is not compatible with the recorded run",
        ));
    }
    if scope.product_file_write_intended != attempt.observed_changes.product_file_write_observed {
        return Some(run_mismatch(
            "product_write_flag_mismatch",
            "write ticket product-file intent is not compatible with the recorded run",
        ));
    }
    if scope.baseline_ref.as_ref() != Some(attempt.baseline_ref) {
        return Some(run_mismatch(
            "baseline_mismatch",
            "write ticket baseline is not compatible with the recorded run",
        ));
    }
    if attempt
        .performed_operation
        .is_some_and(|operation| operation != scope.intended_operation.as_str())
        || (attempt.performed_operation_required && attempt.performed_operation.is_none())
    {
        return Some(run_mismatch(
            "operation_mismatch",
            "performed operation does not exactly match the write ticket operation",
        ));
    }
    if category_set(&normalized_string_set(&scope.sensitive_categories))
        != category_set(&attempt.observed_changes.sensitive_categories)
    {
        return Some(run_mismatch(
            "sensitive_category_mismatch",
            "write ticket sensitive categories are not compatible with the recorded run",
        ));
    }
    if attempt.observed_changes.product_file_write_observed
        && !paths_are_authorized(
            &attempt.observed_changes.changed_paths,
            attempt.normalized_scope_paths,
        )
    {
        return Some(run_mismatch(
            "path_mismatch",
            "write ticket paths are not compatible with the recorded run",
        ));
    }
    None
}

fn run_mismatch(reason: &'static str, message: &'static str) -> RunWriteTicketMismatch {
    RunWriteTicketMismatch { reason, message }
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
        WriteDecisionCategory::Workspace => "workspace",
        WriteDecisionCategory::UserAction => "user_action",
        WriteDecisionCategory::SensitiveApproval => "sensitive_approval",
        WriteDecisionCategory::WriteCompatibility => "write_compatibility",
        WriteDecisionCategory::Baseline => "baseline",
        WriteDecisionCategory::EffectContract => "effect_contract",
        WriteDecisionCategory::ConnectionCapability => "connection_capability",
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
    pub(crate) required_for: UserActionRequiredFor,
    pub(crate) now: &'a UtcTimestamp,
    pub(crate) repo_root: &'a Path,
}

pub(crate) fn current_sensitive_approval(
    judgment: &UserActionAuthority,
    requirement: &SensitiveApprovalRequirement<'_>,
) -> bool {
    if !accepted_current_user_authority(judgment, UserActionKind::SensitiveApproval) {
        return false;
    }
    if !judgment.required_for.contains(&requirement.required_for) {
        return false;
    }
    let Some(basis) = judgment.basis.as_ref() else {
        return false;
    };
    let coordinates = basis.coordinates();
    if coordinates.task_id != *requirement.task_id
        || coordinates.change_unit_id.as_ref() != Some(requirement.change_unit_id)
        || coordinates.scope_revision != requirement.scope_revision
        || coordinates.baseline_ref.as_ref() != requirement.baseline_ref
    {
        return false;
    }
    let Some(scope) = basis.sensitive_action_scope() else {
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
