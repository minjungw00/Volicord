use harness_types::{
    DryRunSummary, GuaranteeDisplay, GuaranteeLevel, PlannedBlocker, PlannedBlockerSourceKind,
    PlannedEffect, PrepareWriteDecision, StateRecordRef, WriteDecisionCategory,
    WriteDecisionReason,
};
use serde_json::Value;

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
