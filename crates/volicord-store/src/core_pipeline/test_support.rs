use std::{
    error::Error,
    path::{Path, PathBuf},
};

use serde_json::json;
use volicord_test_support::TempRuntimeHome;
use volicord_types::{
    ids::{AgentConnectionId, ProjectId},
    schema::JsonObject,
    values::{
        AcceptancePolicy, ActorSource, OperationCategory, PersistedCloseSummary,
        RequestedControlLevel, TaskControlLevel, TaskLifecyclePhase, TaskMode, WorkPhase,
    },
};

use super::{
    CommittedMutationFacts, CoreProjectStore, PendingTaskEvent, TaskAutonomyBoundary, TaskInsert,
    TaskShapingFacts, VerifiedReplayContext,
};
use crate::bootstrap::{
    initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
};
use crate::mutation::TestRuntimeHomeAdmission;
use crate::StoreResult;

pub(super) const PROJECT_ID: &str = "project_store";
pub(super) const CONNECTION_ID: &str = "conn_store";
pub(super) const ACTOR_SOURCE: &str = "agent_connection:conn_store";

pub(super) struct StoreFixture {
    _runtime_home: TempRuntimeHome,
    mutation: TestRuntimeHomeAdmission,
    runtime_home_path: PathBuf,
}

impl StoreFixture {
    pub(super) fn new() -> Result<Self, Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-aggregate")?;
        let setup = TestRuntimeHomeAdmission::exclusive(runtime_home.path())?;
        let setup_context = setup.context()?;
        initialize_runtime_home(&setup_context, "runtime_home_store", "{}")?;
        register_project(
            &setup_context,
            ProjectRegistration {
                project_id: PROJECT_ID.to_owned(),
                repo_root: runtime_home.create_product_repo("repo")?,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        drop(setup_context);
        drop(setup);
        let mutation = TestRuntimeHomeAdmission::shared(runtime_home.path())?;

        Ok(Self {
            runtime_home_path: runtime_home.path().to_path_buf(),
            mutation,
            _runtime_home: runtime_home,
        })
    }

    pub(super) fn store(&self) -> StoreResult<CoreProjectStore<'_>> {
        CoreProjectStore::open_for_mutation(&self.mutation.context()?, &ProjectId::new(PROJECT_ID))
    }

    pub(super) fn state_database_path(&self) -> PathBuf {
        state_database_path(&self.runtime_home_path)
    }
}

fn state_database_path(runtime_home_path: &Path) -> PathBuf {
    runtime_home_path
        .join("projects")
        .join(PROJECT_ID)
        .join("state.sqlite")
}

pub(super) fn replay_context(
    connection_id: &str,
    operation_category: &str,
) -> VerifiedReplayContext {
    let operation_category = match operation_category {
        "read" | "read_only" => OperationCategory::Read,
        "agent_workflow" => OperationCategory::AgentWorkflow,
        "user_only" => OperationCategory::UserOnly,
        "admin_local" => OperationCategory::AdminLocal,
        "local_recovery" => OperationCategory::LocalRecovery,
        value => panic!("unsupported test operation category: {value}"),
    };
    VerifiedReplayContext {
        actor_source: ActorSource::AgentConnection(AgentConnectionId::new(connection_id)),
        operation_category,
        verification_basis: Some("store_test_registration".to_owned()),
        git_workspace_context: None,
    }
}

pub(super) fn local_user_replay_context() -> VerifiedReplayContext {
    VerifiedReplayContext {
        actor_source: ActorSource::LocalUser,
        operation_category: OperationCategory::UserOnly,
        verification_basis: Some("store_test_user_channel".to_owned()),
        git_workspace_context: None,
    }
}

pub(super) fn pending_event(marker: &str) -> PendingTaskEvent {
    pending_event_for_task(marker, &format!("task_{marker}"))
}

pub(super) fn pending_event_for_task(marker: &str, task_id: &str) -> PendingTaskEvent {
    PendingTaskEvent {
        event_id: format!("evt_{marker}"),
        task_id: Some(task_id.to_owned()),
        change_unit_id: None,
        event_kind: "store_test_event".to_owned(),
        event_payload_json: "{}".to_owned(),
    }
}

pub(super) fn task_insert(task_id: &str) -> TaskInsert {
    TaskInsert {
        task_id: task_id.to_owned(),
        created_by_actor_source: ActorSource::AgentConnection(AgentConnectionId::new(
            CONNECTION_ID,
        )),
        mode: TaskMode::Work,
        requested_control_level: RequestedControlLevel::Tracked,
        effective_control_level: TaskControlLevel::Tracked,
        control_level_reason: "Store test control.".to_owned(),
        work_phase: WorkPhase::Shaping,
        acceptance_policy: AcceptancePolicy::Required,
        acceptance_policy_reason: "Store test policy.".to_owned(),
        predecessor_task_id: None,
        lineage_relation: None,
        lineage_reason: None,
        carry_forward: Vec::new(),
        lifecycle_phase: TaskLifecyclePhase::Shaping,
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
        close_summary: PersistedCloseSummary::default(),
        current_change_unit_id: None,
    }
}

pub(super) fn response_json(facts: CommittedMutationFacts) -> StoreResult<String> {
    Ok(json!({
        "base": {
            "state_version": facts.committed_state_version
        },
        "stored_response": "must_not_leak_on_mismatch"
    })
    .to_string())
}
