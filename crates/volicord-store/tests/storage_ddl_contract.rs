use std::error::Error;

use rusqlite::{params, Connection};
use serde_json::{json, Value};
use volicord_store::{
    operational_sessions::{
        record_mcp_initialize_attempt, record_mcp_initialize_completion,
        record_mcp_initialized_notification, record_mcp_tools_list, McpRuntimeSessionStart,
    },
    schema::{
        current_storage_manifest, current_storage_manifest_json, generated_schema_metadata,
        initialize_project_state_schema, initialize_registry_schema,
    },
    sqlite::{
        enable_foreign_keys, open_project_state_database_read_only, validate_project_state_schema,
        validate_registry_schema,
    },
    StoreError, StoreFailureRoute,
};
use volicord_test_support::{
    core_fixtures::CoreFixture, open_project_fixture_database, TempRuntimeHome,
    TestRuntimeHomeMutation,
};
use volicord_types::canonical::canonical_json_string;
use volicord_types::integration_revision::McpRuntimeSessionSource;
use volicord_types::managed_mcp_client_info::ManagedMcpClientInfo;
use volicord_types::storage_contract::{
    GeneratedRelationKind, StorageDatabaseKind, StorageManifest, STORAGE_CONTRACT_ID,
    STORAGE_ENABLED_CAPABILITIES,
};
use volicord_types::tool_names::AgentToolId;

#[test]
fn generated_metadata_and_manifest_from_both_schema_sources_have_stable_vectors(
) -> Result<(), Box<dyn Error>> {
    let metadata = generated_schema_metadata()?;
    assert_eq!(metadata.tables.len(), 77);
    assert_eq!(metadata.columns.len(), 657);
    assert_eq!(metadata.indexes.len(), 78);
    assert_eq!(metadata.constraints.len(), 51);
    let runtime_home_columns = metadata
        .columns
        .iter()
        .filter(|column| {
            column.database == StorageDatabaseKind::Registry && column.table == "runtime_home"
        })
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        runtime_home_columns,
        [
            "singleton_id",
            "runtime_home_id",
            "publication_id",
            "runtime_home_path",
            "registry_db_path",
            "storage_profile",
            "metadata_json",
            "created_at",
            "updated_at",
        ]
    );
    let agent_connection_columns = metadata
        .columns
        .iter()
        .filter(|column| {
            column.database == StorageDatabaseKind::Registry && column.table == "agent_connections"
        })
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        agent_connection_columns,
        [
            "connection_internal_id",
            "integration_instance_id",
            "host_kind",
            "intent",
            "host_scope",
            "project_internal_id",
            "server_name",
            "config_target",
            "mode",
            "enabled",
            "managed_fingerprint",
            "integration_generation",
            "verification_report_json",
            "metadata_json",
            "created_at",
            "updated_at",
        ]
    );
    let guard_probe_observation_columns = metadata
        .columns
        .iter()
        .filter(|column| {
            column.database == StorageDatabaseKind::Registry
                && column.table == "guard_probe_observations"
        })
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        guard_probe_observation_columns,
        [
            "observation_id",
            "verification_id",
            "guard_event_id",
            "stage",
            "expected_agent_tool_id",
            "expected_host_callable_name",
            "observed_callable_name",
            "hook_event_kind",
            "verification_id_present",
            "verification_id_matches",
            "guard_installation_id",
            "integration_revision",
            "observed_at",
        ]
    );
    let launch_lease_columns = metadata
        .columns
        .iter()
        .filter(|column| {
            column.database == StorageDatabaseKind::Registry
                && column.table == "managed_mcp_launch_leases"
        })
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        launch_lease_columns,
        [
            "launch_lease_id",
            "connection_internal_id",
            "host_kind",
            "expected_integration_revision",
            "expected_launch_fingerprint",
            "issued_at",
            "expires_at",
            "consumed_at",
            "terminal_state",
        ]
    );
    assert!(metadata.indexes.iter().any(|index| {
        index.database == StorageDatabaseKind::Registry
            && index.name == "idx_managed_mcp_launch_leases_cleanup"
    }));
    let host_session_columns = metadata
        .columns
        .iter()
        .filter(|column| {
            column.database == StorageDatabaseKind::ProjectState && column.table == "host_sessions"
        })
        .map(|column| (column.name.as_str(), column.not_null))
        .collect::<Vec<_>>();
    assert_eq!(
        host_session_columns,
        [
            ("project_id", true),
            ("session_id", true),
            ("connection_internal_id", true),
            ("project_integration_revision", true),
            ("host_session_id", true),
            ("first_observed_at", true),
            ("last_observed_at", true),
        ]
    );
    assert!(metadata.indexes.iter().any(|index| {
        index.database == StorageDatabaseKind::ProjectState
            && index.table == "managed_mcp_sessions"
            && index.name == "idx_managed_mcp_sessions_runtime_binding"
            && index.unique
            && index.partial
    }));
    for table in [
        "host_turns",
        "host_tool_invocations",
        "managed_mcp_sessions",
    ] {
        assert!(metadata.tables.iter().any(|relation| {
            relation.database == StorageDatabaseKind::ProjectState
                && relation.name == table
                && relation.relation_kind == GeneratedRelationKind::Table
        }));
    }
    assert!(metadata.indexes.iter().any(|index| {
        index.database == StorageDatabaseKind::Registry
            && index.table == "agent_connections"
            && index.name == "idx_agent_connections_integration_instance"
            && index.unique
            && !index.partial
    }));
    assert!(metadata.tables.iter().any(|relation| {
        relation.database == StorageDatabaseKind::Registry
            && relation.name == "agent_connections_integration_instance_immutable"
            && relation.relation_kind == GeneratedRelationKind::Trigger
    }));
    assert!(metadata.columns.iter().any(|column| {
        column.database == StorageDatabaseKind::Registry
            && column.table == "mcp_runtime_project_session_bindings"
            && column.name == "project_integration_revision"
            && column.not_null
    }));
    let finding_columns = metadata
        .columns
        .iter()
        .filter(|column| {
            column.database == StorageDatabaseKind::Registry
                && column.table == "diagnostic_findings"
        })
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    for required in [
        "finding_id",
        "lifecycle",
        "current_identity_digest",
        "current_subject_identity",
        "diagnostic_scope_kind",
        "diagnostic_scope_identity",
        "current_state_status",
        "resolved_at",
        "code",
        "domain",
        "stage",
        "severity",
        "facts_json",
        "source",
        "connection_internal_id",
        "project_internal_id",
        "runtime_session_id",
        "integration_revision",
        "observed_at",
    ] {
        assert!(finding_columns.contains(&required), "missing {required}");
    }
    assert!(metadata.indexes.iter().any(|index| {
        index.database == StorageDatabaseKind::Registry
            && index.table == "diagnostic_findings"
            && index.name == "idx_diagnostic_findings_current_identity"
            && index.unique
            && index.partial
    }));
    assert!(metadata.indexes.iter().any(|index| {
        index.database == StorageDatabaseKind::Registry
            && index.table == "diagnostic_findings"
            && index.name == "idx_diagnostic_findings_active_current_scope"
            && !index.unique
            && index.partial
    }));
    for trigger in [
        "diagnostic_occurrence_findings_immutable",
        "diagnostic_current_identity_immutable",
    ] {
        assert!(metadata.tables.iter().any(|relation| {
            relation.database == StorageDatabaseKind::Registry
                && relation.name == trigger
                && relation.relation_kind == GeneratedRelationKind::Trigger
        }));
    }
    let runtime_columns = metadata
        .columns
        .iter()
        .filter(|column| {
            column.database == StorageDatabaseKind::Registry
                && column.table == "mcp_runtime_sessions"
        })
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    for required in [
        "attempted_client_name",
        "attempted_client_version",
        "requested_protocol_version",
        "selected_protocol_version",
        "negotiated_protocol_version",
        "initialize_completed_at",
        "tools_list_observed_at",
        "returned_tool_identities_json",
        "required_tools_validated_at",
        "verification_tool_name",
        "verification_tool_observed_at",
        "terminal_finding_id",
    ] {
        assert!(runtime_columns.contains(&required), "missing {required}");
    }
    let verification_columns = metadata
        .columns
        .iter()
        .filter(|column| {
            column.database == StorageDatabaseKind::Registry
                && column.table == "guard_integration_verification_runs"
        })
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    for required in [
        "verification_id",
        "connection_internal_id",
        "project_internal_id",
        "project_id",
        "runtime_session_id",
        "host_session_id",
        "host_turn_id",
        "guard_installation_id",
        "integration_revision",
        "host_contract_profile",
        "hook_definition_digest",
        "policy_digest",
        "expected_probe_tool",
        "observation_policy_kind",
        "observation_deadline_at",
        "allowed_status_reads",
        "status_read_count",
        "created_at",
        "cleanup_after",
        "status",
        "probe_acknowledged_at",
        "completed_at",
        "matched_prompt_event_id",
        "matched_pre_tool_event_id",
        "matched_post_tool_event_id",
        "repair_reason",
        "retry_policy",
        "terminal_finding_code",
        "terminal_finding_summary",
    ] {
        assert!(
            verification_columns.contains(&required),
            "missing integration-verification column {required}"
        );
    }
    assert!(metadata.indexes.iter().any(|index| {
        index.database == StorageDatabaseKind::Registry
            && index.table == "guard_integration_verification_runs"
            && index.name == "idx_guard_integration_verification_coordinate"
            && index.unique
            && !index.partial
    }));
    for trigger in [
        "guard_integration_verification_coordinate_immutable",
        "guard_integration_verification_probe_ack_immutable",
        "guard_integration_verification_terminal_immutable",
    ] {
        assert!(metadata.tables.iter().any(|relation| {
            relation.database == StorageDatabaseKind::Registry
                && relation.name == trigger
                && relation.relation_kind == GeneratedRelationKind::Trigger
        }));
    }
    assert!(metadata.indexes.iter().any(|index| {
        index.database == StorageDatabaseKind::Registry
            && index.table == "guard_integration_verification_runs"
            && index.name == "idx_guard_integration_verification_prompt_attempt"
            && index.unique
            && !index.partial
    }));
    assert!(metadata.tables.iter().any(|relation| {
        relation.database == StorageDatabaseKind::ProjectState
            && relation.name == "host_sessions_project_integration_revision_immutable"
            && relation.relation_kind == GeneratedRelationKind::Trigger
    }));
    for database in [
        StorageDatabaseKind::Registry,
        StorageDatabaseKind::ProjectState,
    ] {
        assert!(
            metadata
                .tables
                .iter()
                .any(|table| table.database == database),
            "{database:?} tables must contribute to generated metadata"
        );
        assert!(
            metadata
                .constraints
                .iter()
                .any(|constraint| constraint.database == database),
            "{database:?} constraints must contribute to generated metadata"
        );
    }
    assert_eq!(
        metadata.canonical_ddl_digest,
        "sha256:5609acd19ec28cbe1427aba1b41634a67379494885f296681b714bdff0cbbbb8"
    );
    assert_eq!(
        metadata.integrity_constraints_digest,
        "sha256:b5da1960b709aaea6e0326c4cec986a108daec15fe8f83fa193d13d0f324741f"
    );
    assert!(metadata.tables.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(metadata.columns.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(metadata.indexes.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(metadata
        .constraints
        .windows(2)
        .all(|pair| pair[0] < pair[1]));
    assert_eq!(
        metadata
            .tables
            .iter()
            .filter(|table| table.relation_kind == GeneratedRelationKind::Table)
            .count(),
        metadata.constraints.len()
    );

    let round_trip = serde_json::from_str(&serde_json::to_string(metadata)?)?;
    assert_eq!(metadata, &round_trip);
    let manifest = current_storage_manifest()?;
    assert_eq!(manifest.contract_id, STORAGE_CONTRACT_ID);
    assert_eq!(
        manifest.enabled_capabilities,
        STORAGE_ENABLED_CAPABILITIES
            .iter()
            .map(|capability| (*capability).to_owned())
            .collect::<Vec<_>>()
    );
    let manifest_json = current_storage_manifest_json()?;
    assert_eq!(manifest_json, canonical_json_string(manifest)?);
    assert_eq!(
        manifest_json,
        concat!(
            "{\"canonical_ddl_digest\":\"sha256:5609acd19ec28cbe1427aba1b41634a67379494885f296681b714bdff0cbbbb8\",",
            "\"contract_id\":\"volicord.sqlite.canonical\",",
            "\"enabled_capabilities\":[\"artifact_storage\",\"authority_event_chain\",",
            "\"exact_operation_result\",\"invocation_repository_observation\",\"managed_codex_connection\",",
            "\"operational_mcp_sessions\",\"project_continuity\",\"shaping_checkpoint_lineage\",",
            "\"shaping_decision_applications\",\"shaping_decision_recovery\",\"shaping_progression\",",
            "\"user_action_cli_resolution\"],",
            "\"integrity_constraints_digest\":\"sha256:b5da1960b709aaea6e0326c4cec986a108daec15fe8f83fa193d13d0f324741f\"}"
        )
    );
    Ok(())
}

#[test]
fn canonical_sql_initialization_and_runtime_validation_share_metadata() -> Result<(), Box<dyn Error>>
{
    let manifest = current_storage_manifest_json()?;
    let mut registry = canonical_registry()?;
    insert_registry_owner(&registry, manifest)?;
    validate_registry_schema(&registry)?;

    let mut project = canonical_project()?;
    insert_project_owner(&project, manifest)?;
    validate_project_state_schema(&project)?;

    initialize_registry_schema(&mut registry)?;
    initialize_project_state_schema(&mut project)?;
    validate_registry_schema(&registry)?;
    validate_project_state_schema(&project)?;
    Ok(())
}

#[test]
fn current_manifest_tampering_is_corrupt() -> Result<(), Box<dyn Error>> {
    let current = current_storage_manifest()?.clone();
    let mut cases = Vec::new();

    let mut digest_tamper = current.clone();
    digest_tamper.canonical_ddl_digest = format!("sha256:{}", "0".repeat(64));
    cases.push(canonical_json_string(&digest_tamper)?);

    let mut constraint_tamper = current.clone();
    constraint_tamper.integrity_constraints_digest = format!("sha256:{}", "1".repeat(64));
    cases.push(canonical_json_string(&constraint_tamper)?);

    let mut missing_capability = current.clone();
    missing_capability.enabled_capabilities.pop();
    cases.push(canonical_json_string(&missing_capability)?);

    let mut reordered = current.clone();
    reordered.enabled_capabilities.swap(0, 1);
    cases.push(serde_json::to_string(&reordered)?);

    let mut unknown_member = serde_json::to_value(&current)?;
    unknown_member
        .as_object_mut()
        .expect("manifest object")
        .insert("unexpected".to_owned(), Value::Bool(true));
    cases.push(canonical_json_string(&unknown_member)?);

    let noncanonical_field_order = serde_json::to_string(&current)?;
    assert_ne!(noncanonical_field_order, current_storage_manifest_json()?);
    cases.push(noncanonical_field_order);

    for (index, persisted) in cases.into_iter().enumerate() {
        let project = canonical_project()?;
        insert_project_owner(&project, &persisted)?;
        let error = validate_project_state_schema(&project)
            .expect_err("current-contract tampering must fail closed");
        assert_corrupt(error, &format!("tamper case {index}"));
    }
    Ok(())
}

#[test]
fn malformed_and_noncurrent_manifests_are_corrupt() -> Result<(), Box<dyn Error>> {
    let current = current_storage_manifest()?;
    let unknown = StorageManifest::new(
        "volicord.sqlite.unknown",
        current.canonical_ddl_digest.clone(),
        current.integrity_constraints_digest.clone(),
        current.enabled_capabilities.clone(),
    )?;
    for (index, persisted) in [
        "{}".to_owned(),
        json!("unsupported_numeric_profile_shape").to_string(),
    ]
    .into_iter()
    .enumerate()
    {
        let registry = canonical_registry()?;
        insert_registry_owner(&registry, &persisted)?;
        let error = validate_registry_schema(&registry)
            .expect_err("malformed persisted manifest must be rejected");
        assert_eq!(
            error.classification().route,
            StoreFailureRoute::PersistedDataCorrupt,
            "corrupt case {index}: {error}"
        );
    }

    let registry = canonical_registry()?;
    insert_registry_owner(&registry, &canonical_json_string(&unknown)?)?;
    let error = validate_registry_schema(&registry)
        .expect_err("well-formed noncurrent storage contract must be rejected");
    assert_eq!(
        error.classification().route,
        StoreFailureRoute::PersistedDataCorrupt,
        "well-formed noncurrent manifest: {error}"
    );
    Ok(())
}

#[test]
fn physical_schema_mismatch_is_corrupt_before_reopen() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("physical-schema-mismatch")?;
    let path = runtime_home.project_state_db_path("project_schema");
    let conn = open_project_fixture_database(&path)?;
    insert_project_owner(&conn, current_storage_manifest_json()?)?;
    validate_project_state_schema(&conn)?;
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN unauthorized_extension TEXT",
        [],
    )?;
    let error = validate_project_state_schema(&conn)
        .expect_err("unexpected physical column must fail closed");
    assert_corrupt(error, "unexpected physical column");
    drop(conn);

    let error = open_project_state_database_read_only(&path)
        .expect_err("read-only reopen must repeat exact-schema rejection");
    assert_corrupt(error, "read-only reopen");
    Ok(())
}

#[test]
fn missing_index_and_unexpected_trigger_are_corrupt() -> Result<(), Box<dyn Error>> {
    let manifest = current_storage_manifest_json()?;
    let missing_index = canonical_project()?;
    insert_project_owner(&missing_index, manifest)?;
    missing_index.execute("DROP INDEX idx_tasks_lifecycle", [])?;
    assert_corrupt(
        validate_project_state_schema(&missing_index)
            .expect_err("missing canonical index must be rejected"),
        "missing index",
    );

    let unexpected_trigger = canonical_project()?;
    insert_project_owner(&unexpected_trigger, manifest)?;
    unexpected_trigger.execute_batch(
        "CREATE TRIGGER unauthorized_task_trigger
         AFTER INSERT ON tasks BEGIN SELECT 1; END",
    )?;
    assert_corrupt(
        validate_project_state_schema(&unexpected_trigger)
            .expect_err("unexpected trigger must be rejected"),
        "unexpected trigger",
    );
    Ok(())
}

#[test]
fn runtime_and_project_carriers_persist_the_same_complete_manifest() -> Result<(), Box<dyn Error>> {
    let manifest = current_storage_manifest_json()?;
    let registry = canonical_registry()?;
    insert_registry_owner(&registry, manifest)?;
    let project = canonical_project()?;
    insert_project_owner(&project, manifest)?;

    let runtime_profile: String = registry.query_row(
        "SELECT storage_profile FROM runtime_home WHERE singleton_id = 1",
        [],
        |row| row.get(0),
    )?;
    let project_profile: String = project.query_row(
        "SELECT storage_profile FROM project_state WHERE project_id = 'project_a'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(runtime_profile, project_profile);
    assert_eq!(runtime_profile, manifest);
    validate_registry_schema(&registry)?;
    validate_project_state_schema(&project)?;
    Ok(())
}

#[test]
fn ordinary_open_rejects_a_tampered_manifest_before_returning_a_write_handle(
) -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("manifest-before-write")?;
    let path = runtime_home.project_state_db_path("project_manifest");
    let conn = open_project_fixture_database(&path)?;
    insert_project_owner(
        &conn,
        json!("unsupported_numeric_profile_shape")
            .to_string()
            .as_str(),
    )?;
    drop(conn);

    let error = open_project_fixture_database(&path)
        .expect_err("ordinary open must reject before exposing a writable connection");
    assert_eq!(
        error.classification().route,
        StoreFailureRoute::PersistedDataCorrupt
    );
    Ok(())
}

#[test]
fn canonical_constraints_remain_executable() -> Result<(), Box<dyn Error>> {
    let project = canonical_project()?;
    insert_project_owner(&project, current_storage_manifest_json()?)?;
    let invalid_control = project.execute(
        "INSERT INTO tasks (
            project_id, task_id, created_by_actor_source, mode,
            requested_control_level, effective_control_level, control_level_reason,
            work_phase, acceptance_policy, acceptance_policy_reason, carry_forward_json,
            lifecycle_phase, created_at, updated_at
         ) VALUES (
            'project_a', 'task_invalid', 'agent_connection:conn_main', 'work',
            'unbounded', 'tracked', 'fixture', 'shaping', 'required', 'fixture', '[]',
            'shaping', 't0', 't0'
         )",
        [],
    );
    assert!(invalid_control.is_err());

    insert_task(&project)?;
    project.execute(
        "INSERT INTO change_units (
            project_id, change_unit_id, task_id, status, is_current,
            basis_state_version, created_at, updated_at
         ) VALUES ('project_a', 'cu_a', 'task_a', 'active', 1, 1, 't0', 't0')",
        [],
    )?;
    let duplicate = project.execute(
        "INSERT INTO change_units (
            project_id, change_unit_id, task_id, status, is_current,
            basis_state_version, created_at, updated_at
         ) VALUES ('project_a', 'cu_b', 'task_a', 'active', 1, 1, 't0', 't0')",
        [],
    );
    assert!(duplicate.is_err());
    Ok(())
}

#[test]
fn shaping_predecessor_and_live_authority_constraints_are_executable() -> Result<(), Box<dyn Error>>
{
    let lineage = canonical_project()?;
    insert_project_owner(&lineage, current_storage_manifest_json()?)?;
    insert_task(&lineage)?;
    insert_shaping_checkpoint(
        &lineage,
        "checkpoint_initial",
        None,
        "task_a",
        "ready",
        "t0",
        None,
    )?;

    let predecessor_not_superseded = insert_shaping_checkpoint(
        &lineage,
        "checkpoint_early_successor",
        Some("checkpoint_initial"),
        "task_a",
        "ready",
        "t1",
        None,
    );
    assert!(predecessor_not_superseded.is_err());
    lineage.execute(
        "UPDATE shaping_checkpoints
            SET readiness = 'superseded', superseded_at = 't1'
          WHERE project_id = 'project_a' AND shaping_checkpoint_id = 'checkpoint_initial'",
        [],
    )?;
    let mismatched_timestamp = insert_shaping_checkpoint(
        &lineage,
        "checkpoint_mismatched_timestamp",
        Some("checkpoint_initial"),
        "task_a",
        "ready",
        "t2",
        None,
    );
    assert!(mismatched_timestamp.is_err());
    insert_shaping_checkpoint(
        &lineage,
        "checkpoint_successor",
        Some("checkpoint_initial"),
        "task_a",
        "ready",
        "t1",
        None,
    )?;
    assert!(lineage
        .execute(
            "UPDATE shaping_checkpoints
                SET predecessor_shaping_checkpoint_id = NULL
              WHERE project_id = 'project_a'
                AND shaping_checkpoint_id = 'checkpoint_successor'",
            [],
        )
        .is_err());

    let mut detached = canonical_project()?;
    insert_project_owner(&detached, current_storage_manifest_json()?)?;
    insert_task(&detached)?;
    let transaction = detached.transaction()?;
    transaction.execute(
        "INSERT INTO user_action_requests (
            project_id, user_action_request_id, task_id, action_kind,
            request_json, basis_json, basis_status, required_for_json,
            requested_by_actor_source, source_method, source_idempotency_key,
            requested_at, metadata_json
         ) VALUES (
            'project_a', 'request_live', 'task_a', 'product_decision',
            '{}', '{}', 'current', '[\"advance_task\"]',
            'agent_connection:conn_main', 'volicord.record_shaping', 'idem_live',
            't0', '{}'
         )",
        [],
    )?;
    insert_shaping_checkpoint(
        &transaction,
        "checkpoint_live",
        None,
        "task_a",
        "blocked",
        "t0",
        None,
    )?;
    transaction.execute(
        "INSERT INTO shaping_checkpoint_gaps (
            project_id, shaping_checkpoint_id, shaping_gap_id, task_id,
            gap_kind, summary, affected_refs_json, status,
            user_action_request_id, user_action_kind
         ) VALUES (
            'project_a', 'checkpoint_live', 'gap_live', 'task_a',
            'user_product_decision_required', 'Decision required.', '[]', 'current',
            'request_live', 'product_decision'
         )",
        [],
    )?;
    transaction.execute(
        "INSERT INTO shaping_checkpoint_user_actions (
            project_id, shaping_checkpoint_id, shaping_gap_id, task_id,
            user_action_request_id, action_kind, linked_at
         ) VALUES (
            'project_a', 'checkpoint_live', 'gap_live', 'task_a',
            'request_live', 'product_decision', 't0'
         )",
        [],
    )?;
    transaction.commit()?;
    assert!(detached
        .execute(
            "UPDATE shaping_checkpoints
                SET readiness = 'superseded', superseded_at = 't1'
              WHERE project_id = 'project_a'
                AND shaping_checkpoint_id = 'checkpoint_live'",
            [],
        )
        .is_err());
    detached.execute(
        "UPDATE user_action_requests
            SET basis_status = 'superseded'
          WHERE project_id = 'project_a'
            AND user_action_request_id = 'request_live'",
        [],
    )?;
    detached.execute(
        "UPDATE shaping_checkpoints
            SET readiness = 'superseded', superseded_at = 't1'
          WHERE project_id = 'project_a'
            AND shaping_checkpoint_id = 'checkpoint_live'",
        [],
    )?;
    Ok(())
}

#[test]
fn diagnostic_lifecycle_columns_enforce_fresh_schema_invariants() -> Result<(), Box<dyn Error>> {
    let registry = canonical_registry()?;
    let insert = |id: &str,
                  lifecycle: &str,
                  digest: Option<&str>,
                  subject_identity: Option<&str>,
                  scope_kind: Option<&str>,
                  scope_identity: Option<&str>,
                  status: Option<&str>,
                  resolved_at: Option<&str>,
                  runtime_session_id: Option<&str>| {
        registry.execute(
            "INSERT INTO diagnostic_findings (
                finding_id, lifecycle, current_identity_digest, current_subject_identity,
                diagnostic_scope_kind, diagnostic_scope_identity,
                current_state_status, resolved_at,
                code, domain, stage, severity, source,
                subject_json, facts_json, actions_json,
                runtime_session_id, observed_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                'store.lifecycle_test', 'store', 'test', 'error', 'store_test',
                '{\"kind\":\"test_case\",\"reference\":\"subject\"}', '{}', '[]',
                ?9, '2026-07-22T00:00:00Z'
             )",
            params![
                id,
                lifecycle,
                digest,
                subject_identity,
                scope_kind,
                scope_identity,
                status,
                resolved_at,
                runtime_session_id,
            ],
        )
    };

    let occurrence_id = "finding.occurrence_00000000-0000-4000-8000-000000000001";
    assert_eq!(
        insert(
            occurrence_id,
            "occurrence",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )?,
        1
    );
    assert!(registry
        .execute(
            "UPDATE diagnostic_findings SET facts_json = '{\"changed\":true}' WHERE finding_id = ?1",
            [occurrence_id],
        )
        .is_err());
    assert!(registry
        .execute(
            "UPDATE diagnostic_findings SET lifecycle = 'current_state' WHERE finding_id = ?1",
            [occurrence_id],
        )
        .is_err());
    assert!(insert(
        "finding.occurrence_00000000-0000-4000-8000-000000000003",
        "retired_lifecycle",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .is_err());
    assert!(insert(
        "finding.occurrence_00000000-0000-4000-8000-000000000002",
        "occurrence",
        Some(&"a".repeat(64)),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .is_err());
    assert!(insert(
        "finding.occurrence_00000000-0000-4000-8000-000000000004",
        "occurrence",
        None,
        None,
        None,
        None,
        Some("active"),
        None,
        None,
    )
    .is_err());

    let subject_identity = format!("sha256:{}", "1".repeat(64));
    assert!(insert(
        "finding.occurrence_00000000-0000-4000-8000-000000000005",
        "occurrence",
        None,
        Some(&subject_identity),
        None,
        None,
        None,
        None,
        None,
    )
    .is_err());

    let digest = "b".repeat(64);
    let current_id = format!("finding.current.sha256:{digest}");
    assert_eq!(
        insert(
            &current_id,
            "current_state",
            Some(&digest),
            Some(&subject_identity),
            Some("connection"),
            Some("opaque connection identity"),
            Some("active"),
            None,
            None,
        )?,
        1
    );
    assert_eq!(
        registry.execute(
            "UPDATE diagnostic_findings SET subject_json = '{\"kind\":\"test_case\",\"reference\":\"other\"}' WHERE finding_id = ?1",
            [&current_id],
        )?,
        1
    );
    assert!(registry
        .execute(
            "UPDATE diagnostic_findings SET current_subject_identity = ?2 WHERE finding_id = ?1",
            params![&current_id, format!("sha256:{}", "2".repeat(64))],
        )
        .is_err());
    assert!(insert(
        &format!("finding.current.sha256:{}", "c".repeat(64)),
        "current_state",
        Some(&"c".repeat(64)),
        Some(&subject_identity),
        Some("connection"),
        Some("opaque connection identity"),
        Some("resolved"),
        None,
        None,
    )
    .is_err());
    assert!(insert(
        &format!("finding.current.sha256:{}", "e".repeat(64)),
        "current_state",
        None,
        Some(&subject_identity),
        Some("connection"),
        Some("opaque connection identity"),
        Some("active"),
        None,
        None,
    )
    .is_err());
    assert!(insert(
        &format!("finding.current.sha256:{}", "9".repeat(64)),
        "current_state",
        Some(&"9".repeat(64)),
        None,
        Some("connection"),
        Some("opaque connection identity"),
        Some("active"),
        None,
        None,
    )
    .is_err());
    assert!(insert(
        &format!("finding.current.sha256:{}", "8".repeat(64)),
        "current_state",
        Some(&"8".repeat(64)),
        Some("sha256:invalid"),
        Some("connection"),
        Some("opaque connection identity"),
        Some("active"),
        None,
        None,
    )
    .is_err());
    assert!(insert(
        &format!("finding.current.sha256:{}", "f".repeat(64)),
        "current_state",
        Some(&"a".repeat(64)),
        Some(&subject_identity),
        Some("connection"),
        Some("opaque connection identity"),
        Some("active"),
        None,
        None,
    )
    .is_err());
    assert!(insert(
        &format!("finding.current.sha256:{}", "e".repeat(64)),
        "current_state",
        Some(&"e".repeat(64)),
        Some(&subject_identity),
        Some("connection"),
        Some("opaque connection identity"),
        Some("active"),
        Some("2026-07-22T00:00:01Z"),
        None,
    )
    .is_err());
    assert!(insert(
        &format!("finding.current.sha256:{}", "f".repeat(64)),
        "current_state",
        Some(&"f".repeat(64)),
        Some(&subject_identity),
        Some("connection"),
        Some(""),
        Some("active"),
        None,
        None,
    )
    .is_err());
    assert_eq!(
        insert(
            &format!("finding.current.sha256:{}", "0".repeat(64)),
            "current_state",
            Some(&"0".repeat(64)),
            Some(&subject_identity),
            Some("project"),
            Some("opaque project identity"),
            Some("resolved"),
            Some("2026-07-22T00:00:01Z"),
            None,
        )?,
        1
    );
    assert!(insert(
        &format!("finding.current.sha256:{}", "d".repeat(64)),
        "current_state",
        Some(&"d".repeat(64)),
        Some(&subject_identity),
        Some("connection"),
        Some("opaque connection identity"),
        Some("active"),
        None,
        Some("runtime_not_allowed"),
    )
    .is_err());
    Ok(())
}

#[test]
fn runtime_verification_tool_columns_enforce_pair_name_and_milestone_order(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("storage-runtime-verification-tool-constraints")?;
    let admission = TestRuntimeHomeMutation::acquire(fixture.runtime_home_path())?;
    let context = admission.context()?;
    let runtime = volicord_test_support::start_test_mcp_runtime_session(
        fixture.runtime_home_path(),
        McpRuntimeSessionStart {
            connection_internal_id: fixture.connection_id().to_owned(),
            session_source: McpRuntimeSessionSource::ManagedHost,
            observed_host_executable_version: Some("fixture-host".to_owned()),
            process_id: 71,
            process_started_at: "2026-07-22T00:00:00Z".to_owned(),
        },
    )?;
    let client = ManagedMcpClientInfo::new("fixture-client", "1.0")?;
    record_mcp_initialize_attempt(
        &context,
        &runtime.runtime_session_id,
        &client,
        "2025-11-25",
        "2026-07-22T00:00:01Z",
    )?;
    record_mcp_initialize_completion(
        &context,
        &runtime.runtime_session_id,
        "2025-11-25",
        "2026-07-22T00:00:01Z",
    )?;
    record_mcp_initialized_notification(
        &context,
        &runtime.runtime_session_id,
        "2025-11-25",
        "2026-07-22T00:00:02Z",
    )?;
    record_mcp_tools_list(
        &context,
        &runtime.runtime_session_id,
        &[AgentToolId::LIST_PROJECTS.wire_name().to_owned()],
        true,
        "2026-07-22T00:00:03Z",
    )?;

    let registry = Connection::open(fixture.runtime_home_path().join("registry.sqlite"))?;
    let update = |name: Option<&str>, observed_at: Option<&str>| {
        registry.execute(
            "UPDATE mcp_runtime_sessions
                SET verification_tool_name = ?2, verification_tool_observed_at = ?3
              WHERE runtime_session_id = ?1",
            params![runtime.runtime_session_id, name, observed_at],
        )
    };
    assert!(update(Some(AgentToolId::LIST_PROJECTS.wire_name()), None).is_err());
    assert!(update(None, Some("2026-07-22T00:00:03Z")).is_err());
    assert!(update(Some("volicord/list projects"), Some("2026-07-22T00:00:03Z")).is_err());
    assert!(update(
        Some("volicord.list_projects\0ignored"),
        Some("2026-07-22T00:00:03Z")
    )
    .is_err());
    assert!(update(
        Some(AgentToolId::LIST_PROJECTS.wire_name()),
        Some("2026-07-22T00:00:01Z")
    )
    .is_err());
    assert_eq!(
        update(
            Some(AgentToolId::LIST_PROJECTS.wire_name()),
            Some("2026-07-22T00:00:04Z")
        )?,
        1
    );
    Ok(())
}

fn canonical_registry() -> Result<Connection, Box<dyn Error>> {
    let mut conn = Connection::open_in_memory()?;
    enable_foreign_keys(&conn)?;
    initialize_registry_schema(&mut conn)?;
    Ok(conn)
}

fn canonical_project() -> Result<Connection, Box<dyn Error>> {
    let mut conn = Connection::open_in_memory()?;
    enable_foreign_keys(&conn)?;
    initialize_project_state_schema(&mut conn)?;
    Ok(conn)
}

fn insert_registry_owner(conn: &Connection, manifest: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO runtime_home (
            singleton_id, runtime_home_id, publication_id, runtime_home_path, registry_db_path,
            storage_profile, created_at, updated_at
         ) VALUES (
            1,
            'runtime_a',
            'runtime_home_publication_00112233-4455-4abb-8cdd-eeff10203040',
            '/runtime-a',
            '/runtime-a/registry.sqlite',
            ?1,
            't0',
            't0'
         )",
        [manifest],
    )?;
    Ok(())
}

fn insert_project_owner(conn: &Connection, manifest: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO project_state (project_id, storage_profile, created_at, updated_at)
         VALUES ('project_a', ?1, 't0', 't0')",
        [manifest],
    )?;
    Ok(())
}

fn insert_task(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO tasks (
            project_id, task_id, created_by_actor_source, mode,
            requested_control_level, effective_control_level, control_level_reason,
            work_phase, acceptance_policy, acceptance_policy_reason, carry_forward_json,
            lifecycle_phase, created_at, updated_at
         ) VALUES (
            'project_a', 'task_a', 'agent_connection:conn_main', 'work',
            'tracked', 'tracked', 'fixture', 'shaping', 'required', 'fixture', '[]',
            'shaping', 't0', 't0'
         )",
        params![],
    )?;
    Ok(())
}

fn insert_shaping_checkpoint(
    conn: &Connection,
    checkpoint_id: &str,
    predecessor_id: Option<&str>,
    task_id: &str,
    readiness: &str,
    created_at: &str,
    superseded_at: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO shaping_checkpoints (
            project_id, shaping_checkpoint_id, predecessor_shaping_checkpoint_id,
            task_id, scope_revision, baseline_ref, summary,
            implementation_boundary, readiness, source_refs_json,
            evidence_refs_json, created_at, superseded_at
         ) VALUES (
            'project_a', ?1, ?2, ?3, 0, 'baseline_test',
            'Current shaping authority.', 'Exact current boundary.', ?4,
            '[]', '[]', ?5, ?6
         )",
        params![
            checkpoint_id,
            predecessor_id,
            task_id,
            readiness,
            created_at,
            superseded_at
        ],
    )?;
    Ok(())
}

fn assert_corrupt(error: StoreError, label: &str) {
    assert_eq!(
        error.classification().route,
        StoreFailureRoute::PersistedDataCorrupt,
        "{label}: {error}"
    );
}
