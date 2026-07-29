use crate::pipeline::VerifiedInvocationContext;
use volicord_store::core_pipeline::{
    ChangeUnitInsert, ChangeUnitRecord, ChangeUnitStatus, StoredChangeUnitLifecycle,
    StoredChangeUnitScopeSummary, StoredChangeUnitWriteBasis,
};
use volicord_types::ids::{ChangeUnitId, ProjectId, TaskId};
use volicord_types::methods::UpdateScopeRequest;

pub(crate) struct ChangeUnitPlan {
    pub(crate) insert: ChangeUnitInsert,
    pub(crate) projected_record: ChangeUnitRecord,
}

pub(crate) fn plan_current_change_unit(
    request: &UpdateScopeRequest,
    change_unit_id: &ChangeUnitId,
    verified_invocation: &VerifiedInvocationContext,
    planned_state_version: u64,
) -> ChangeUnitPlan {
    let insert = ChangeUnitInsert {
        change_unit_id: change_unit_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        scope_summary: StoredChangeUnitScopeSummary {
            scope_summary: Some(
                request
                    .change_unit
                    .scope_summary()
                    .map(str::to_owned)
                    .or_else(|| request.scope_boundary.as_ref().cloned())
                    .unwrap_or_else(|| "Current Change Unit".to_owned()),
            ),
            affected_areas: request.change_unit.affected_areas(),
            constraints: request.change_unit.constraints(),
        },
        bounded_paths: request.change_unit.affected_paths(),
        write_basis: StoredChangeUnitWriteBasis {
            baseline_ref: request.baseline_ref.clone().into_option(),
            git_workspace_context: verified_invocation.git_workspace_context.as_ref().map(
                |context| volicord_store::core_pipeline::StoredGitWorkspaceContext {
                    git_common_dir: context.git_common_dir.clone(),
                    worktree_id: context.worktree_id.clone(),
                    branch_ref: context.branch_ref.clone(),
                    head_sha: context.head_sha.clone(),
                    workspace_fingerprint: context.workspace_fingerprint.clone(),
                },
            ),
        },
        effect_contract: request.change_unit.effect_contract.clone(),
        lifecycle: StoredChangeUnitLifecycle {
            recovery_required: false,
        },
    };
    let projected_record = projected_record(
        &request.envelope.project_id,
        &request.task_id,
        &insert,
        planned_state_version,
    );
    ChangeUnitPlan {
        insert,
        projected_record,
    }
}

fn projected_record(
    project_id: &ProjectId,
    task_id: &TaskId,
    insert: &ChangeUnitInsert,
    planned_state_version: u64,
) -> ChangeUnitRecord {
    ChangeUnitRecord {
        project_id: project_id.as_str().to_owned(),
        change_unit_id: insert.change_unit_id.clone(),
        task_id: task_id.as_str().to_owned(),
        status: ChangeUnitStatus::Active,
        is_current: true,
        basis_state_version: planned_state_version,
        scope_summary: insert.scope_summary.clone(),
        bounded_paths: insert.bounded_paths.clone(),
        write_basis: insert.write_basis.clone(),
        effect_contract: insert.effect_contract.clone(),
        lifecycle: insert.lifecycle.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::VerifiedInvocationContext;
    use serde_json::{json, Map};
    use volicord_types::ids::{IdempotencyKey, RequestId};
    use volicord_types::methods::ChangeUnitUpdate;
    use volicord_types::schema::{DryRunIntent, RequiredNullable, ToolEnvelope};
    use volicord_types::values::{ActorSource, ChangeUnitOperation, OperationCategory};

    #[test]
    fn change_unit_owner_builds_typed_insert_and_projected_record() {
        let project_id = ProjectId::new("project_change_unit_plan");
        let task_id = TaskId::new("task_change_unit_plan");
        let request = UpdateScopeRequest {
            envelope: ToolEnvelope {
                project_id: project_id.clone(),
                task_id: RequiredNullable::some(task_id.clone()),
                request_id: RequestId::new("request_change_unit_plan"),
                idempotency_key: RequiredNullable::some(IdempotencyKey::new(
                    "idempotency_change_unit_plan",
                )),
                expected_state_version: RequiredNullable::some(4),
                dry_run: DryRunIntent::NotRequested,
                locale: RequiredNullable::null(),
            },
            task_id: task_id.clone(),
            goal_summary: RequiredNullable::null(),
            scope_update: RequiredNullable::null(),
            scope_boundary: RequiredNullable::some("fallback scope".to_owned()),
            non_goals: RequiredNullable::null(),
            acceptance_criteria: RequiredNullable::null(),
            autonomy_boundary: RequiredNullable::null(),
            baseline_ref: RequiredNullable::null(),
            change_unit: ChangeUnitUpdate {
                operation: ChangeUnitOperation::CreateCurrent,
                effect_contract: None,
                fields: Map::from_iter([
                    ("scope_summary".to_owned(), json!("typed scope")),
                    ("affected_areas".to_owned(), json!(["core", "store"])),
                    ("affected_paths".to_owned(), json!(["crates/volicord-core"])),
                    ("constraints".to_owned(), json!(["preserve API"])),
                ]),
            },
            related_scope_decision_refs: Vec::new(),
        };
        let invocation = VerifiedInvocationContext {
            project_id,
            actor_source: ActorSource::System,
            operation_category: OperationCategory::AgentWorkflow,
            verification_basis: "test".to_owned(),
            assurance_level: "test".to_owned(),
            session_id: None,
            git_workspace_context: None,
        };

        let plan = plan_current_change_unit(
            &request,
            &ChangeUnitId::new("change_unit_planned"),
            &invocation,
            5,
        );

        assert_eq!(
            plan.insert.scope_summary.scope_summary.as_deref(),
            Some("typed scope")
        );
        assert_eq!(plan.insert.bounded_paths, ["crates/volicord-core"]);
        assert_eq!(plan.projected_record.basis_state_version, 5);
        assert_eq!(
            plan.projected_record.change_unit_id,
            plan.insert.change_unit_id
        );
    }
}
