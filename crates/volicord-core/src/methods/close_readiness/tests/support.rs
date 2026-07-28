use super::facts::{facts_from_projection, CloseReadinessFacts};
use super::service::CloseReadinessRequest;
use volicord_store::core_pipeline::{
    ProjectStateHeader, TaskAutonomyBoundary, TaskRecord, TaskShapingFacts,
};
use volicord_store::guards::UnrecordedChangeRecord;
use volicord_types::ids::{ProjectId, RequestId, TaskId};
use volicord_types::schema::{JsonObject, ToolEnvelope};
use volicord_types::values::{
    AcceptancePolicy, PersistedCloseSummary, RequestedControlLevel, TaskControlLevel,
    TaskLifecyclePhase, TaskMode, UtcTimestamp, WorkPhase,
};

pub(super) fn facts() -> CloseReadinessFacts {
    facts_from_projection(
        task(),
        None,
        None,
        Vec::new(),
        Vec::new(),
        None,
        UtcTimestamp::parse("2026-07-27T00:00:00Z").expect("valid test timestamp"),
    )
}

pub(super) fn task() -> TaskRecord {
    TaskRecord {
        project_id: "project_close_readiness".to_owned(),
        task_id: "task_close_readiness".to_owned(),
        mode: TaskMode::Work,
        requested_control_level: RequestedControlLevel::Light,
        effective_control_level: TaskControlLevel::Light,
        control_level_reason: "test".to_owned(),
        work_phase: WorkPhase::Implementation,
        acceptance_policy: AcceptancePolicy::PolicyDependent,
        acceptance_policy_reason: "test".to_owned(),
        predecessor_task_id: None,
        lineage_relation: None,
        lineage_reason: None,
        carry_forward: Vec::new(),
        lifecycle_phase: TaskLifecyclePhase::Executing,
        result: None,
        title: None,
        summary: None,
        shaping: TaskShapingFacts {
            goal_summary: None,
            scope_summary: None,
            non_goals: Vec::new(),
            baseline_ref: None,
            autonomy_boundary: None,
            initial_context_refs: Vec::new(),
            initial_source_refs: Vec::new(),
        },
        bounded_context: JsonObject::new(),
        autonomy_boundary: TaskAutonomyBoundary {
            autonomy_boundary: None,
        },
        scope_revision: 1,
        close_basis_revision: 0,
        close_basis: None,
        close_summary: PersistedCloseSummary::default(),
        current_change_unit_id: None,
        closed_at: None,
        metadata: JsonObject::new(),
    }
}

pub(super) fn request() -> CloseReadinessRequest {
    let task_id = TaskId::new("task_close_readiness");
    CloseReadinessRequest::check(
        ToolEnvelope {
            project_id: ProjectId::new("project_close_readiness"),
            task_id: Some(task_id.clone()).into(),
            request_id: RequestId::new("request_close_readiness"),
            idempotency_key: None.into(),
            expected_state_version: None.into(),
            dry_run: volicord_types::schema::DryRunIntent::NotRequested,
            locale: None.into(),
        },
        task_id,
    )
}

pub(super) fn project_state() -> ProjectStateHeader {
    ProjectStateHeader {
        project_id: "project_close_readiness".to_owned(),
        state_version: 7,
        active_task_id: Some("task_close_readiness".to_owned()),
        updated_at: "2026-07-27T00:00:00Z".to_owned(),
    }
}

pub(super) fn unresolved_change() -> UnrecordedChangeRecord {
    UnrecordedChangeRecord {
        project_id: "project_close_readiness".to_owned(),
        unrecorded_change_id: "unrecorded_close_readiness".to_owned(),
        session_id: None,
        correlation: None,
        connection_internal_id: "connection_close_readiness".to_owned(),
        task_id: Some("task_close_readiness".to_owned()),
        status: "unresolved".to_owned(),
        confidence: "confirmed".to_owned(),
        summary: "unrecorded test change".to_owned(),
        observed_paths_json: r#"["src/lib.rs"]"#.to_owned(),
        detection_json: "{}".to_owned(),
        resolution_json: None,
        detected_at: "2026-07-27T00:00:00Z".to_owned(),
        resolved_at: None,
        resolved_by_actor_source: None,
        metadata_json: "{}".to_owned(),
    }
}
