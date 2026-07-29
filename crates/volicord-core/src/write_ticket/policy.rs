use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use volicord_store::{core_pipeline::StoredWriteTicket, StoreError};
use volicord_types::ids::{BaselineRef, ChangeUnitId, TaskId};
use volicord_types::product_path::paths_are_authorized;
use volicord_types::schema::{
    ObservedChanges, StateRecordRef, WriteDecisionReason, WriteTicketAttemptScope,
};
use volicord_types::values::{PrepareWriteDecision, UtcTimestamp, WriteDecisionCategory};

pub(crate) fn write_ticket_is_idle_expired(
    record: &StoredWriteTicket,
    now: DateTime<Utc>,
) -> Result<bool, StoreError> {
    let Some(timestamp) = record.idle_expires_at() else {
        return Ok(false);
    };
    Ok(UtcTimestamp::from_datetime(now) >= *timestamp)
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
    scope: &WriteTicketAttemptScope,
    attempt: RunWriteTicketAttempt<'_>,
) -> Option<RunWriteTicketMismatch> {
    if scope.task_id != *attempt.task_id {
        return Some(run_mismatch(
            "task_mismatch",
            "write ticket task is not compatible with the recorded run",
        ));
    }
    if scope.change_unit_id != *attempt.change_unit_id {
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

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::product_path::ProductRelativePath;
    use volicord_types::schema::RequiredNullable;

    fn reason(code: &'static str) -> WriteDecisionReason {
        write_decision_reason(
            WriteDecisionCategory::WriteCompatibility,
            code,
            "decision reason",
            Vec::new(),
        )
    }

    #[test]
    fn prepare_write_decision_uses_the_strongest_semantic_blocker_class() {
        assert_eq!(prepare_write_decision(&[]), PrepareWriteDecision::Allowed);
        assert_eq!(
            prepare_write_decision(&[reason("path_out_of_scope")]),
            PrepareWriteDecision::Blocked
        );
        assert_eq!(
            prepare_write_decision(&[
                reason("path_out_of_scope"),
                reason("sensitive_approval_missing"),
            ]),
            PrepareWriteDecision::ApprovalRequired
        );
        assert_eq!(
            prepare_write_decision(&[
                reason("sensitive_approval_missing"),
                reason("user_action_unresolved"),
            ]),
            PrepareWriteDecision::DecisionRequired
        );
    }

    #[test]
    fn sensitive_categories_are_trimmed_deduplicated_and_sorted() {
        assert_eq!(
            normalized_string_set(&[
                " secrets ".to_owned(),
                "network".to_owned(),
                "secrets".to_owned(),
                " ".to_owned(),
            ]),
            vec!["network".to_owned(), "secrets".to_owned()]
        );
    }

    #[test]
    fn run_compatibility_matrix_reports_the_first_typed_ticket_mismatch() {
        let task_id = TaskId::new("task_current");
        let change_unit_id = ChangeUnitId::new("change_current");
        let baseline_ref = BaselineRef::new("baseline_current");
        let scope = WriteTicketAttemptScope {
            task_id: task_id.clone(),
            change_unit_id: change_unit_id.clone(),
            intended_operation: "edit".to_owned(),
            intended_paths: vec![ProductRelativePath::parse("src").expect("valid product path")],
            product_file_write_intended: true,
            sensitive_categories: vec!["network".to_owned()],
            baseline_ref: Some(baseline_ref.clone()),
        };
        assert_eq!(
            mismatch_reason(
                &scope,
                "task_current",
                "change_current",
                "baseline_current",
                Some("edit"),
                true,
                true,
                &["network"],
                &["src/lib.rs"],
                &["src"],
            ),
            None
        );
        assert_eq!(
            mismatch_reason(
                &scope,
                "task_other",
                "change_current",
                "baseline_current",
                Some("edit"),
                true,
                true,
                &["network"],
                &["src/lib.rs"],
                &["src"],
            ),
            Some("task_mismatch")
        );
        assert_eq!(
            mismatch_reason(
                &scope,
                "task_current",
                "change_other",
                "baseline_current",
                Some("edit"),
                true,
                true,
                &["network"],
                &["src/lib.rs"],
                &["src"],
            ),
            Some("change_unit_mismatch")
        );
        assert_eq!(
            mismatch_reason(
                &scope,
                "task_current",
                "change_current",
                "baseline_current",
                Some("edit"),
                true,
                false,
                &["network"],
                &["src/lib.rs"],
                &["src"],
            ),
            Some("product_write_flag_mismatch")
        );
        assert_eq!(
            mismatch_reason(
                &scope,
                "task_current",
                "change_current",
                "baseline_other",
                Some("edit"),
                true,
                true,
                &["network"],
                &["src/lib.rs"],
                &["src"],
            ),
            Some("baseline_mismatch")
        );
        assert_eq!(
            mismatch_reason(
                &scope,
                "task_current",
                "change_current",
                "baseline_current",
                Some("replace"),
                true,
                true,
                &["network"],
                &["src/lib.rs"],
                &["src"],
            ),
            Some("operation_mismatch")
        );
        assert_eq!(
            mismatch_reason(
                &scope,
                "task_current",
                "change_current",
                "baseline_current",
                Some("edit"),
                true,
                true,
                &["secrets"],
                &["src/lib.rs"],
                &["src"],
            ),
            Some("sensitive_category_mismatch")
        );
        assert_eq!(
            mismatch_reason(
                &scope,
                "task_current",
                "change_current",
                "baseline_current",
                Some("edit"),
                true,
                true,
                &["network"],
                &["docs/outside.md"],
                &["src"],
            ),
            Some("path_mismatch")
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn mismatch_reason(
        scope: &WriteTicketAttemptScope,
        task_id: &str,
        change_unit_id: &str,
        baseline_ref: &str,
        performed_operation: Option<&str>,
        performed_operation_required: bool,
        product_file_write_observed: bool,
        sensitive_categories: &[&str],
        changed_paths: &[&str],
        normalized_scope_paths: &[&str],
    ) -> Option<&'static str> {
        let task_id = TaskId::new(task_id);
        let change_unit_id = ChangeUnitId::new(change_unit_id);
        let baseline_ref = BaselineRef::new(baseline_ref);
        let observed_changes = ObservedChanges {
            changed_paths: changed_paths
                .iter()
                .map(|path| (*path).to_owned())
                .collect(),
            product_file_write_observed,
            sensitive_categories: sensitive_categories
                .iter()
                .map(|category| (*category).to_owned())
                .collect(),
            baseline_ref: RequiredNullable::null(),
        };
        let normalized_scope_paths = normalized_scope_paths
            .iter()
            .map(|path| (*path).to_owned())
            .collect::<Vec<_>>();

        run_write_ticket_mismatch(
            scope,
            RunWriteTicketAttempt {
                task_id: &task_id,
                change_unit_id: &change_unit_id,
                baseline_ref: &baseline_ref,
                performed_operation,
                performed_operation_required,
                observed_changes: &observed_changes,
                normalized_scope_paths: &normalized_scope_paths,
            },
        )
        .map(|mismatch| mismatch.reason)
    }
}
