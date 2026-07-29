use std::collections::BTreeSet;
use volicord_types::ids::{ProjectId, TaskId};
use volicord_types::schema::{GuaranteeDisplay, StateRecordRef, WriteTicketStateSummary};
use volicord_types::values::{
    StateRecordKind, TaskControlLevel, UserActionKind, UserActionRequiredFor, UtcTimestamp,
    WriteTicketInvalidationReason, WriteTicketStatus,
};

use chrono::{DateTime, Utc};
use volicord_store::{
    core_pipeline::{CoreProjectStore, WriteTicketRecord},
    StoreError,
};

use crate::pipeline::{CorePipelineError, CoreResult};
use crate::policy::workflow::{project_workflow_policy, resolve_task_control_authority};
use crate::record_refs::{state_ref, stored_refs_to_state_refs, write_ticket_ref};
use crate::write_ticket::write_ticket_is_idle_expired;
use volicord_user_action_service::{current_sensitive_approval, SensitiveApprovalRequirement};

pub(crate) fn write_ticket_summary_for_record(
    store: Option<&CoreProjectStore>,
    record: &WriteTicketRecord,
    state_version: u64,
    now: Option<DateTime<Utc>>,
    observation_refs: Option<Vec<StateRecordRef>>,
    guarantee_display: Option<GuaranteeDisplay>,
) -> CoreResult<WriteTicketStateSummary> {
    let attempt_scope = &record.attempt_scope;
    let consumed_by_run_ref = record.consumed_by_run_id.as_ref().map(|run_id| {
        state_ref(
            StateRecordKind::Run,
            run_id,
            &ProjectId::new(record.project_id.clone()),
            Some(&TaskId::new(record.task_id.clone())),
            Some(state_version),
        )
    });
    let observation_refs = match (observation_refs, record.consumed_by_run_id.as_ref(), store) {
        (Some(refs), _, _) => refs,
        (None, Some(run_id), Some(store)) => stored_refs_to_state_refs(
            store
                .evidence_observation_refs_for_run(
                    &TaskId::new(record.task_id.clone()),
                    run_id,
                    state_version,
                )
                .map_err(CorePipelineError::from)?,
        ),
        _ => Vec::new(),
    };
    let mut effective_status = effective_write_ticket_status(record, state_version, now)?;
    let mut effective_invalidation_reason =
        effective_write_ticket_invalidation_reason(record, now)?;
    if effective_status == WriteTicketStatus::Active {
        if let (Some(store), Some(now)) = (store, now) {
            if let Some(reason) = write_ticket_projection_invalidation_reason(store, record, now)? {
                effective_status = WriteTicketStatus::Invalidated;
                effective_invalidation_reason = Some(reason);
            }
        }
    }
    Ok(WriteTicketStateSummary {
        status: effective_status,
        write_ticket_ref: Some(write_ticket_ref(record, state_version)),
        basis_state_version: Some(record.basis_state_version),
        validity_basis: Some(record.validity_basis.clone()),
        invalidation_reason: effective_invalidation_reason,
        idle_expires_at: record.idle_expires_at.clone(),
        intended_paths: attempt_scope
            .intended_paths
            .iter()
            .map(|path| path.as_str().to_owned())
            .collect(),
        consumed_by_run_ref,
        observation_refs,
        guarantee_display,
    })
}

pub(crate) fn effective_write_ticket_status(
    record: &WriteTicketRecord,
    _state_version: u64,
    now: Option<DateTime<Utc>>,
) -> CoreResult<WriteTicketStatus> {
    let stored_status = record.status;
    if stored_status != WriteTicketStatus::Active {
        return Ok(stored_status);
    }
    if now
        .map(|now| write_ticket_is_idle_expired(record, now))
        .transpose()
        .map_err(CorePipelineError::from)?
        .unwrap_or(false)
    {
        Ok(WriteTicketStatus::Invalidated)
    } else {
        Ok(WriteTicketStatus::Active)
    }
}

pub(crate) fn effective_write_ticket_invalidation_reason(
    record: &WriteTicketRecord,
    now: Option<DateTime<Utc>>,
) -> CoreResult<Option<WriteTicketInvalidationReason>> {
    if record.status == WriteTicketStatus::Active
        && now
            .map(|now| write_ticket_is_idle_expired(record, now))
            .transpose()
            .map_err(CorePipelineError::from)?
            .unwrap_or(false)
    {
        return Ok(Some(WriteTicketInvalidationReason::IdleTimeout));
    }
    Ok(record.invalidation_reason)
}

pub(crate) fn write_ticket_projection_invalidation_reason(
    store: &CoreProjectStore,
    record: &WriteTicketRecord,
    now: DateTime<Utc>,
) -> CoreResult<Option<WriteTicketInvalidationReason>> {
    let validity_basis = &record.validity_basis;
    let scope = &record.attempt_scope;
    let task = store
        .task_record(&validity_basis.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            CorePipelineError::Store(StoreError::NotFound {
                entity: "task",
                id: validity_basis.task_id.as_str().to_owned(),
            })
        })?;
    let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;
    if validity_basis.write_authority_fingerprint != workflow_policy.write_authority_fingerprint {
        return Ok(Some(WriteTicketInvalidationReason::ExplicitRevoke));
    }
    let resolved_control =
        resolve_task_control_authority(&task, &workflow_policy).map_err(CorePipelineError::from)?;
    if resolved_control.pending_policy_reevaluation {
        return Ok(Some(WriteTicketInvalidationReason::ExplicitRevoke));
    }
    if validity_basis.approval_basis_refs.is_empty() {
        return Ok((!scope.sensitive_categories.is_empty()
            || resolved_control.effective_control_level == TaskControlLevel::Sensitive)
            .then_some(WriteTicketInvalidationReason::ApprovalBasisChanged));
    }

    let now = UtcTimestamp::from_datetime(now);
    let normalized_scope_paths = scope
        .intended_paths
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect::<Vec<_>>();
    let requirement = SensitiveApprovalRequirement {
        task_id: &validity_basis.task_id,
        change_unit_id: &validity_basis.change_unit_id,
        scope_revision: task.scope_revision,
        operation: &scope.intended_operation,
        normalized_paths: &normalized_scope_paths,
        sensitive_categories: &scope.sensitive_categories,
        baseline_ref: scope.baseline_ref.as_ref(),
        required_for: UserActionRequiredFor::PrepareWrite,
        now: &now,
    };
    let records = store
        .resolved_user_action_records(
            &validity_basis.task_id,
            UserActionKind::SensitiveApproval,
            &now,
        )
        .map_err(CorePipelineError::from)?;
    let mut current_resolution_ids = BTreeSet::new();
    for record in records {
        let authority = volicord_user_action_service::user_action_authority_from_record(&record)?;
        if current_sensitive_approval(&authority, &requirement) {
            if let Some(resolution_id) = authority.user_action_resolution_id {
                current_resolution_ids.insert(resolution_id);
            }
        }
    }
    let approval_basis_is_current = !current_resolution_ids.is_empty()
        && validity_basis.approval_basis_refs.iter().all(|stored| {
            stored.record_kind == StateRecordKind::UserActionResolution
                && current_resolution_ids.contains(stored.record_id.as_str())
        });
    Ok((!approval_basis_is_current).then_some(WriteTicketInvalidationReason::ApprovalBasisChanged))
}

pub(crate) fn write_ticket_is_current_for_projection(
    store: &CoreProjectStore,
    record: &WriteTicketRecord,
    now: DateTime<Utc>,
) -> CoreResult<bool> {
    Ok(write_ticket_projection_invalidation_reason(store, record, now)?.is_none())
}

pub(crate) fn selected_write_ticket_for_projection(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
    now: DateTime<Utc>,
) -> CoreResult<Option<WriteTicketRecord>> {
    let records = store
        .write_tickets_for_task(task_id)
        .map_err(CorePipelineError::from)?;
    let mut selected = None;
    let mut selected_priority = u8::MAX;
    for record in records {
        let mut status = effective_write_ticket_status(&record, state_version, Some(now))?;
        if status == WriteTicketStatus::Active
            && !write_ticket_is_current_for_projection(store, &record, now)?
        {
            status = WriteTicketStatus::Invalidated;
        }
        let priority = match status {
            WriteTicketStatus::Active => 0,
            WriteTicketStatus::Invalidated => 1,
            WriteTicketStatus::Consumed => 2,
            WriteTicketStatus::Revoked => 3,
        };
        if priority < selected_priority {
            selected_priority = priority;
            selected = Some(record);
        }
    }
    Ok(selected)
}

pub(crate) fn projected_write_ticket_summary(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
    now: DateTime<Utc>,
    guarantee_display: Option<GuaranteeDisplay>,
) -> CoreResult<Option<WriteTicketStateSummary>> {
    selected_write_ticket_for_projection(store, task_id, state_version, now)?
        .as_ref()
        .map(|record| {
            write_ticket_summary_for_record(
                Some(store),
                record,
                state_version,
                Some(now),
                None,
                guarantee_display,
            )
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::{
        ids::{BaselineRef, ChangeUnitId},
        product_path::ProductRelativePath,
        schema::{WriteTicketAttemptScope, WriteTicketValidityBasis},
    };

    #[test]
    fn idle_expiry_projects_status_and_reason_at_the_exact_boundary() {
        let idle_expires_at =
            UtcTimestamp::parse("2026-07-29T00:15:00Z").expect("valid expiration");
        let record = ticket_record(idle_expires_at.clone());
        let before = UtcTimestamp::parse("2026-07-29T00:14:59Z")
            .expect("valid timestamp")
            .into_datetime();
        let at_boundary = idle_expires_at.into_datetime();

        assert_eq!(
            effective_write_ticket_status(&record, 7, Some(before)).expect("valid projection"),
            WriteTicketStatus::Active
        );
        assert_eq!(
            effective_write_ticket_status(&record, 7, Some(at_boundary)).expect("valid projection"),
            WriteTicketStatus::Invalidated
        );
        assert_eq!(
            effective_write_ticket_invalidation_reason(&record, Some(at_boundary))
                .expect("valid projection"),
            Some(WriteTicketInvalidationReason::IdleTimeout)
        );
    }

    fn ticket_record(idle_expires_at: UtcTimestamp) -> WriteTicketRecord {
        let task_id = TaskId::new("task_current");
        let change_unit_id = ChangeUnitId::new("change_current");
        let baseline_ref = BaselineRef::new("baseline_current");
        let attempt_scope = WriteTicketAttemptScope {
            task_id: task_id.clone(),
            change_unit_id: change_unit_id.clone(),
            intended_operation: "edit".to_owned(),
            intended_paths: vec![ProductRelativePath::parse("src").expect("valid product path")],
            product_file_write_intended: true,
            sensitive_categories: Vec::new(),
            baseline_ref: Some(baseline_ref.clone()),
        };
        WriteTicketRecord {
            project_id: "project_current".to_owned(),
            write_ticket_id: "write_ticket_current".to_owned(),
            task_id: task_id.as_str().to_owned(),
            change_unit_id: change_unit_id.as_str().to_owned(),
            basis_state_version: 7,
            status: WriteTicketStatus::Active,
            validity_basis: WriteTicketValidityBasis {
                task_id,
                change_unit_id,
                scope_revision: 3,
                baseline_ref: Some(baseline_ref),
                workspace_context_sha256: None,
                write_authority_fingerprint: "authority_current".to_owned(),
                approval_basis_refs: Vec::new(),
            },
            allowed_path_prefixes: attempt_scope.intended_paths.clone(),
            denied_path_prefixes: Vec::new(),
            attempt_scope,
            created_by_actor_source: volicord_types::values::ActorSource::System,
            created_by_user_action_resolution_id: None,
            idle_expires_at: Some(idle_expires_at),
            invalidation_reason: None,
            consumed_by_run_id: None,
            consumed_at: None,
            revoked_at: None,
            created_at: UtcTimestamp::parse("2026-07-29T00:00:00Z")
                .expect("valid creation timestamp"),
            metadata: serde_json::Map::new(),
        }
    }
}
