use std::{
    error::Error,
    path::{Path, PathBuf},
};

use serde_json::json;
use volicord_test_support::TempRuntimeHome;
use volicord_types::ids::ProjectId;

use super::{
    CommittedMutationFacts, CoreProjectStore, PendingTaskEvent, TaskInsert, VerifiedReplayContext,
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
    VerifiedReplayContext {
        actor_source: format!("agent_connection:{connection_id}"),
        operation_category: operation_category.to_owned(),
        verification_basis: Some("store_test_registration".to_owned()),
        git_workspace_context_json: None,
    }
}

pub(super) fn local_user_replay_context() -> VerifiedReplayContext {
    VerifiedReplayContext {
        actor_source: "local_user".to_owned(),
        operation_category: "user_only".to_owned(),
        verification_basis: Some("store_test_user_channel".to_owned()),
        git_workspace_context_json: None,
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
        created_by_actor_source: ACTOR_SOURCE.to_owned(),
        mode: "work".to_owned(),
        requested_control_level: "tracked".to_owned(),
        effective_control_level: "tracked".to_owned(),
        control_level_reason: "Store test control.".to_owned(),
        work_phase: "shaping".to_owned(),
        acceptance_policy: "required".to_owned(),
        acceptance_policy_reason: "Store test policy.".to_owned(),
        predecessor_task_id: None,
        lineage_relation: None,
        lineage_reason: None,
        carry_forward_json: "[]".to_owned(),
        lifecycle_phase: "shaping".to_owned(),
        result: None,
        title: None,
        summary: None,
        shaping_summary_json: "{}".to_owned(),
        bounded_context_json: "[]".to_owned(),
        autonomy_boundary_json: "{}".to_owned(),
        close_summary_json: "{\"close_reason\":\"none\"}".to_owned(),
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
