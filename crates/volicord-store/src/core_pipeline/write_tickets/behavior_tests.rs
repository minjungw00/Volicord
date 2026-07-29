use std::error::Error;

use rusqlite::params;
use serde_json::json;
use volicord_types::ids::{AgentConnectionId, IdempotencyKey, ProjectId, RequestHash};
use volicord_types::schema::ObservedChanges;
use volicord_types::values::{ActorSource, MethodName, RunKind};
use volicord_types::workflow_policy::ProjectWorkflowPolicy;

use super::{WriteTicketConsumption, WriteTicketMutation};
use crate::core_pipeline::test_support::{
    pending_event_for_task, replay_context, response_json, task_insert,
    StoreFixture as StoreHarness, ACTOR_SOURCE, CONNECTION_ID, PROJECT_ID,
};
use crate::core_pipeline::{
    commit_input, CoreStorageMutation, RunInsert, RunMutation, RunStatus, StoredRunMetadata,
    StoredRunSummary, StoredRunWriteTicketEffect, StoredRunWriteTicketEffectKind, TaskMutation,
};
use crate::StoreError;

#[test]
fn write_ticket_read_rejects_invalid_persisted_expiry() -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_invalid_write_ticket_expiry";
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new(
                "idem_invalid_write_ticket_expiry_setup",
            )),
            &RequestHash::new("sha256:invalid-write-ticket-expiry-setup"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task(
                "invalid_write_ticket_expiry_setup",
                task_id,
            )],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;
    let change_unit_id = "change_unit_invalid_expiry";
    store.conn.execute(
        "INSERT INTO change_units (
                project_id, change_unit_id, task_id, status, is_current,
                basis_state_version, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 'active', 1, 1,
                       '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z')",
        params![PROJECT_ID, change_unit_id, task_id],
    )?;
    store.conn.execute(
        "UPDATE tasks
                SET current_change_unit_id = ?3
              WHERE project_id = ?1
                AND task_id = ?2",
        params![PROJECT_ID, task_id, change_unit_id],
    )?;
    let write_authority_fingerprint =
        crate::workflow_records::project_write_authority_fingerprint(None)?;
    store.conn.execute(
        "INSERT INTO write_tickets (
            project_id, write_ticket_id, task_id, change_unit_id,
            basis_state_version, status, validity_basis_json,
            allowed_path_prefixes_json, denied_path_prefixes_json,
            attempt_scope_json, created_by_actor_source, idle_expires_at,
            created_at, metadata_json
         ) VALUES (?1, 'write_ticket_invalid_expiry', ?2, 'change_unit_invalid_expiry',
                   1, 'active', ?3, '[]', '[]', ?4, ?5, 'tomorrow',
                   '2026-07-13T00:00:00Z', '{}')",
        params![
            PROJECT_ID,
            task_id,
            volicord_types::canonical::canonical_json_string(&json!({
                "task_id": task_id,
                "change_unit_id": change_unit_id,
                "scope_revision": 1,
                "baseline_ref": null,
                "workspace_context_sha256": null,
                "write_authority_fingerprint": write_authority_fingerprint,
                "approval_basis_refs": []
            }))?,
            volicord_types::canonical::canonical_json_string(&json!({
                "task_id": task_id,
                "change_unit_id": change_unit_id,
                "intended_operation": "test",
                "intended_paths": [],
                "product_file_write_intended": false,
                "sensitive_categories": [],
                "baseline_ref": null
            }))?,
            ACTOR_SOURCE,
        ],
    )?;

    let error = store
        .write_ticket_record("write_ticket_invalid_expiry")
        .expect_err("malformed persisted expiry must fail at the Store boundary");
    assert!(matches!(
        error,
        StoreError::CorruptOwnerStateValue {
            table: "write_tickets",
            logical_column: "idle_expires_at",
            ..
        }
    ));
    Ok(())
}

#[test]
fn write_ticket_consumption_revalidates_policy_authority_inside_transaction(
) -> Result<(), Box<dyn Error>> {
    let harness = StoreHarness::new()?;
    let mut store = harness.store()?;
    let task_id = "task_ticket_policy_transaction";
    let write_ticket_id = "ticket_policy_transaction";
    let run_id = "run_policy_transaction";
    store.commit_with(
        commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_ticket_policy_transaction_setup")),
            &RequestHash::new("sha256:ticket-policy-transaction-setup"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task(
                "ticket_policy_transaction_setup",
                task_id,
            )],
        ),
        |mutation, facts| {
            CoreStorageMutation::Task(TaskMutation::insert(task_insert(task_id)))
                .apply(mutation, facts)
                .map(|_| ())
        },
        response_json,
    )?;

    let change_unit_id = "change_unit_ticket_policy_transaction";
    store.conn.execute(
        "INSERT INTO change_units (
                project_id, change_unit_id, task_id, status, is_current,
                basis_state_version, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 'active', 1, 1,
                       '2026-07-17T00:00:00Z', '2026-07-17T00:00:00Z')",
        params![PROJECT_ID, change_unit_id, task_id],
    )?;
    store.conn.execute(
        "UPDATE tasks
                SET current_change_unit_id = ?3
              WHERE project_id = ?1
                AND task_id = ?2",
        params![PROJECT_ID, task_id, change_unit_id],
    )?;

    let issued_fingerprint = crate::workflow_records::project_write_authority_fingerprint(None)?;
    let validity_basis_json = volicord_types::canonical::canonical_json_string(&json!({
        "task_id": task_id,
        "change_unit_id": change_unit_id,
        "scope_revision": 1,
        "baseline_ref": null,
        "workspace_context_sha256": null,
        "write_authority_fingerprint": issued_fingerprint,
        "approval_basis_refs": []
    }))?;
    let attempt_scope_json = volicord_types::canonical::canonical_json_string(&json!({
        "task_id": task_id,
        "change_unit_id": change_unit_id,
        "intended_operation": "test",
        "intended_paths": ["src/export.rs"],
        "product_file_write_intended": true,
        "sensitive_categories": [],
        "baseline_ref": null
    }))?;
    store.conn.execute(
        "INSERT INTO write_tickets (
                project_id, write_ticket_id, task_id, change_unit_id,
                basis_state_version, status, validity_basis_json,
                allowed_path_prefixes_json, denied_path_prefixes_json,
                attempt_scope_json, created_by_actor_source, created_at,
                metadata_json
             ) VALUES (?1, ?2, ?3, ?4, 1, 'active', ?5,
                       '[\"src/export.rs\"]', '[]', ?6, ?7,
                       '2026-07-17T00:00:00Z', '{}')",
        params![
            PROJECT_ID,
            write_ticket_id,
            task_id,
            change_unit_id,
            validity_basis_json,
            attempt_scope_json,
            ACTOR_SOURCE
        ],
    )?;
    let tightened_policy = json!({
        "schema": volicord_types::schema::WORKFLOW_POLICY_CONTRACT_ID,
        "managed_by": "volicord",
        "storage_scope": "local_overlay",
        "connection_intent": "shared",
        "host": "codex",
        "repo_root": "/tmp/write-ticket-policy-test",
        "connection_id": "connection_write_ticket_policy_test",
        "guard_installation_id": "guard_write_ticket_policy_test",
        "selected_profile": "record",
        "mcp": {
            "command": "volicord-mcp",
            "args": [],
            "env": {}
        },
        "host_hook": {
            "enabled": true,
            "commands": {
                "pre_tool": {"command": "volicord", "args": ["guard", "pre-tool"]},
                "post_tool": {"command": "volicord", "args": ["guard", "post-tool"]},
                "prompt_capture": {
                    "command": "volicord",
                    "args": ["guard", "prompt-capture"]
                }
            }
        },
        "workflow": {
            "default_direct_control": "tracked",
            "default_work_control": "tracked",
            "light": {
                "enabled": false,
                "max_intended_paths": 3,
                "allowed_path_patterns": [],
                "denied_path_patterns": ["src/**"],
                "final_acceptance": "policy_dependent"
            },
            "write_ticket": {
                "idle_timeout_minutes": null
            }
        }
    });
    let policy_json = volicord_types::canonical::canonical_json_string(&tightened_policy)?;
    let policy_fingerprint =
        volicord_types::canonical::canonical_json_sha256(&tightened_policy)?.into_inner();
    let typed_policy = serde_json::from_value::<ProjectWorkflowPolicy>(tightened_policy.clone())?;
    let current_fingerprint =
        crate::workflow_records::project_write_authority_fingerprint(Some(&typed_policy))?;
    assert_ne!(issued_fingerprint, current_fingerprint);
    store.conn.execute(
        "INSERT INTO project_workflow_policies (
                project_id, policy_schema, policy_version, policy_json,
                policy_fingerprint, source, applied_at, created_at
             ) VALUES (?1, ?2, 1, ?3, ?4, 'project_database',
                       '2026-07-17T00:00:00Z', '2026-07-17T00:00:00Z')",
        params![
            PROJECT_ID,
            volicord_types::schema::WORKFLOW_POLICY_CONTRACT_ID,
            policy_json,
            policy_fingerprint
        ],
    )?;
    let before_state = store.project_state()?;
    let before_effects = store.effect_counts()?;

    let error = store
        .commit_with(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RecordRun,
                Some(&IdempotencyKey::new(
                    "idem_ticket_policy_transaction_consume",
                )),
                &RequestHash::new("sha256:ticket-policy-transaction-consume"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(1),
                vec![pending_event_for_task(
                    "ticket_policy_transaction_consume",
                    task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::Run(RunMutation::Insert(RunInsert {
                    run_id: run_id.to_owned(),
                    task_id: task_id.to_owned(),
                    change_unit_id: None,
                    scope_revision: 0,
                    write_ticket_id: Some(write_ticket_id.to_owned()),
                    kind: RunKind::Implementation,
                    status: RunStatus::Recorded,
                    summary: StoredRunSummary {
                        summary: String::new(),
                    },
                    observed_changes: ObservedChanges {
                        changed_paths: Vec::new(),
                        product_file_write_observed: false,
                        sensitive_categories: Vec::new(),
                        baseline_ref: None.into(),
                    },
                    evidence_updates: Vec::new(),
                    write_ticket_effect: StoredRunWriteTicketEffect {
                        write_ticket_id: Some(write_ticket_id.into()),
                        effect: StoredRunWriteTicketEffectKind::Consumed,
                    },
                    created_by_actor_source: ActorSource::AgentConnection(AgentConnectionId::new(
                        CONNECTION_ID,
                    )),
                    metadata: StoredRunMetadata {
                        verification_basis: "store_test_boundary".to_owned(),
                    },
                }))
                .apply(mutation, facts)
                .map(|_| ())?;
                CoreStorageMutation::WriteTicket(WriteTicketMutation::Consume(
                    WriteTicketConsumption {
                        write_ticket_id: write_ticket_id.to_owned(),
                        run_id: run_id.to_owned(),
                        expected_basis_state_version: 1,
                        expected_write_authority_fingerprint: issued_fingerprint.clone(),
                    },
                ))
                .apply(mutation, facts)
                .map(|_| ())
            },
            response_json,
        )
        .expect_err("changed policy authority must reject ticket consumption");

    assert!(matches!(
        error,
        StoreError::Conflict {
            entity: "write_ticket",
            ..
        }
    ));
    let (status, consumed_by_run_id): (String, Option<String>) = store.conn.query_row(
        "SELECT status, consumed_by_run_id
               FROM write_tickets
              WHERE project_id = ?1
                AND write_ticket_id = ?2",
        params![PROJECT_ID, write_ticket_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(status, "active");
    assert_eq!(consumed_by_run_id, None);
    let run_count: i64 = store.conn.query_row(
        "SELECT COUNT(*)
               FROM runs
              WHERE project_id = ?1
                AND run_id = ?2",
        params![PROJECT_ID, run_id],
        |row| row.get(0),
    )?;
    assert_eq!(run_count, 0);
    assert_eq!(store.project_state()?, before_state);
    assert_eq!(store.effect_counts()?, before_effects);
    Ok(())
}
