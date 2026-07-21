use std::error::Error;

use rusqlite::{params, Connection};
use serde_json::{json, Value};
use volicord_store::{
    schema::{
        current_storage_manifest, current_storage_manifest_json, generated_schema_metadata,
        initialize_project_state_schema, initialize_registry_schema,
    },
    sqlite::{
        enable_foreign_keys, open_project_state_database, open_project_state_database_read_only,
        validate_project_state_schema, validate_registry_schema,
    },
    StoreError, StoreFailureRoute,
};
use volicord_test_support::TempRuntimeHome;
use volicord_types::{
    canonical_json_string, GeneratedRelationKind, StorageDatabaseKind, StorageManifest,
    STORAGE_CONTRACT_ID, STORAGE_ENABLED_CAPABILITIES,
};

#[test]
fn generated_metadata_and_manifest_from_both_schema_sources_have_stable_vectors(
) -> Result<(), Box<dyn Error>> {
    let metadata = generated_schema_metadata()?;
    assert_eq!(metadata.tables.len(), 39);
    assert_eq!(metadata.columns.len(), 484);
    assert_eq!(metadata.indexes.len(), 65);
    assert_eq!(metadata.constraints.len(), 37);
    let agent_connection_columns = metadata
        .columns
        .iter()
        .filter(|column| {
            column.database == StorageDatabaseKind::Registry && column.table == "agent_connections"
        })
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert!(agent_connection_columns.contains(&"verification_report_json"));
    assert!(agent_connection_columns.contains(&"integration_generation"));
    assert!(agent_connection_columns.contains(&"integration_instance_id"));
    assert!(!agent_connection_columns.contains(&"last_verification_status"));
    assert!(!agent_connection_columns.contains(&"last_user_actions_json"));
    let agent_session_columns = metadata
        .columns
        .iter()
        .filter(|column| {
            column.database == StorageDatabaseKind::ProjectState && column.table == "agent_sessions"
        })
        .collect::<Vec<_>>();
    assert!(agent_session_columns
        .iter()
        .any(|column| column.name == "runtime_session_id" && !column.not_null));
    assert!(agent_session_columns
        .iter()
        .any(|column| column.name == "first_observed_at" && column.not_null));
    assert!(!agent_session_columns
        .iter()
        .any(|column| column.name == "started_at"));
    assert!(metadata.indexes.iter().any(|index| {
        index.database == StorageDatabaseKind::ProjectState
            && index.table == "agent_sessions"
            && index.name == "idx_agent_sessions_runtime_binding"
            && index.unique
            && index.partial
    }));
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
    assert!(metadata.tables.iter().any(|relation| {
        relation.database == StorageDatabaseKind::ProjectState
            && relation.name == "agent_sessions_project_integration_revision_immutable"
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
        "sha256:265f31187bf96e6869817449e37bd54622c5bd2e364362fab351fffc8280f9cb"
    );
    assert_eq!(
        metadata.integrity_constraints_digest,
        "sha256:4fb752c4487ea46b186ebc655261231a49e412e480f974966ce24dd6bd0e4c37"
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
            "{\"canonical_ddl_digest\":\"sha256:265f31187bf96e6869817449e37bd54622c5bd2e364362fab351fffc8280f9cb\",",
            "\"contract_id\":\"volicord.sqlite.canonical\",",
            "\"enabled_capabilities\":[\"artifact_storage\",\"authority_event_chain\",",
            "\"exact_operation_result\",\"guard_reconciliation\",\"managed_codex_connection\",",
            "\"operational_mcp_sessions\",\"project_continuity\",\"user_action_cli_resolution\"],",
            "\"integrity_constraints_digest\":\"sha256:4fb752c4487ea46b186ebc655261231a49e412e480f974966ce24dd6bd0e4c37\"}"
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
fn preceding_connection_bound_session_schema_manifest_requires_reinitialization(
) -> Result<(), Box<dyn Error>> {
    let preceding = StorageManifest::new(
        STORAGE_CONTRACT_ID,
        "sha256:28efe2a0d3a544481181185b4d73fb465e1bcf4158237c73217a25a1db963e4b",
        "sha256:1549e654b8de2b08ef6327a8f3d01c68f47f83bcad4bf39d3a3d3e5c416abdd9",
        STORAGE_ENABLED_CAPABILITIES
            .iter()
            .map(|capability| (*capability).to_owned())
            .collect(),
    )?;
    let registry = canonical_registry()?;
    let persisted = canonical_json_string(&preceding)?;
    insert_registry_owner(&registry, &persisted)?;

    let error = validate_registry_schema(&registry)
        .expect_err("the preceding connection-bound session schema must not be silently upgraded");
    assert!(
        matches!(
            error,
            StoreError::UnsupportedStorageProfile {
                database_kind: "registry",
                ref actual_storage_profile,
                ..
            } if actual_storage_profile.as_str() == persisted
        ),
        "{error:?}"
    );
    let still_persisted: String = registry.query_row(
        "SELECT storage_profile FROM runtime_home WHERE singleton_id = 1",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(still_persisted, persisted);
    Ok(())
}

#[test]
fn physical_schema_mismatch_is_corrupt_before_reopen() -> Result<(), Box<dyn Error>> {
    let runtime_home = TempRuntimeHome::new("physical-schema-mismatch")?;
    let path = runtime_home.project_state_db_path("project_schema");
    let conn = open_project_state_database(&path)?;
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
    let conn = open_project_state_database(&path)?;
    insert_project_owner(
        &conn,
        json!("unsupported_numeric_profile_shape")
            .to_string()
            .as_str(),
    )?;
    drop(conn);

    let error = open_project_state_database(&path)
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
            singleton_id, runtime_home_id, runtime_home_path, registry_db_path,
            storage_profile, created_at, updated_at
         ) VALUES (1, 'runtime_a', '/runtime-a', '/runtime-a/registry.sqlite', ?1, 't0', 't0')",
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

fn assert_corrupt(error: StoreError, label: &str) {
    assert_eq!(
        error.classification().route,
        StoreFailureRoute::PersistedDataCorrupt,
        "{label}: {error}"
    );
}
