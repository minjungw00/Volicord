use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
};

use rusqlite::{params, Connection, Error as RusqliteError, ErrorCode};
use serde_json::Value;
use volicord_store::{
    migrations::{
        apply_project_state_migrations, apply_registry_migrations, PROJECT_STATE_SCHEMA_VERSION,
        STORAGE_PROFILE,
    },
    sqlite::{enable_foreign_keys, validate_project_state_schema, validate_registry_schema},
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct DatabaseSchema {
    tables: BTreeMap<String, TableSchema>,
    explicit_indexes: BTreeMap<String, IndexSchema>,
    unique_constraints: BTreeSet<UniqueConstraintSchema>,
    triggers: BTreeMap<String, TriggerSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableSchema {
    columns: BTreeMap<String, ColumnSchema>,
    foreign_keys: BTreeSet<ForeignKeySchema>,
    check_constraints: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ColumnSchema {
    declared_type: String,
    not_null: bool,
    default_value: Option<String>,
    primary_key_position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ForeignKeySchema {
    parent_table: String,
    columns: Vec<ForeignKeyColumnSchema>,
    on_update: String,
    on_delete: String,
    match_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ForeignKeyColumnSchema {
    child_column: String,
    parent_column: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexSchema {
    table: String,
    unique: bool,
    columns: Vec<IndexedColumnSchema>,
    partial_predicate: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct UniqueConstraintSchema {
    table: String,
    columns: Vec<IndexedColumnSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct IndexedColumnSchema {
    name: String,
    descending: bool,
    collation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TriggerSchema {
    table: String,
    sql: String,
}

#[test]
fn initial_schemas_satisfy_connection_storage_contract() -> Result<(), Box<dyn Error>> {
    let initial_registry = initial_registry_schema()?;
    let initial_registry_schema = read_database_schema(&initial_registry)?;
    let initial_project = initial_project_state_schema()?;
    let initial_project_schema = read_database_schema(&initial_project)?;

    assert!(initial_registry_schema
        .tables
        .contains_key("agent_connections"));
    assert!(initial_registry_schema
        .tables
        .contains_key("connection_projects"));
    assert!(initial_registry_schema
        .tables
        .contains_key("installation_profile"));
    assert!(initial_registry_schema
        .tables
        .contains_key("project_aliases"));
    assert_columns_include(
        &initial_registry_schema,
        "runtime_home",
        &["runtime_home_path", "registry_db_path"],
    );
    assert_columns_include(
        &initial_registry_schema,
        "installation_profile",
        &[
            "installation_id",
            "volicord_command",
            "volicord_mcp_command",
            "default_connection_mode",
        ],
    );
    assert_columns_include(
        &initial_registry_schema,
        "projects",
        &["project_internal_id", "project_name", "project_alias"],
    );
    assert_columns_include(
        &initial_registry_schema,
        "agent_connections",
        &[
            "connection_internal_id",
            "intent",
            "project_internal_id",
            "last_verification_report_json",
            "last_user_actions_json",
        ],
    );
    assert_columns_include(
        &initial_registry_schema,
        "connection_projects",
        &["connection_internal_id", "project_internal_id"],
    );
    assert_unique_index_columns(
        &initial_registry_schema,
        "idx_projects_repo_root",
        &["repo_root"],
    );
    assert_unique_index_columns(
        &initial_registry_schema,
        "idx_agent_connections_target_project",
        &[
            "host_kind",
            "intent",
            "host_scope",
            "project_internal_id",
            "config_target",
            "server_name",
        ],
    );
    assert_unique_index_columns(
        &initial_registry_schema,
        "idx_agent_connections_target_global",
        &[
            "host_kind",
            "intent",
            "host_scope",
            "config_target",
            "server_name",
        ],
    );

    assert!(initial_project_schema.tables.contains_key("write_checks"));
    assert_columns_include(
        &initial_project_schema,
        "tool_invocations",
        &["actor_source", "operation_category"],
    );

    assert_project_contract_behavior("initial project state.sqlite", &initial_project)?;

    Ok(())
}

#[test]
fn schema_comparison_detects_contract_critical_drift() -> Result<(), Box<dyn Error>> {
    let expected_conn = initial_project_state_schema()?;
    let expected = read_database_schema(&expected_conn)?;

    let missing_actor_source = initial_project_state_schema()?;
    missing_actor_source.execute(
        "CREATE TABLE tool_invocations_drift AS
         SELECT project_id, tool_name, idempotency_key, request_hash,
                basis_state_version, committed_state_version, status,
                operation_category, verification_basis, response_json, created_at
           FROM tool_invocations",
        [],
    )?;
    missing_actor_source.execute("DROP TABLE tool_invocations", [])?;
    missing_actor_source.execute(
        "ALTER TABLE tool_invocations_drift RENAME TO tool_invocations",
        [],
    )?;
    assert_schema_differs(
        "removing tool_invocations.actor_source",
        &expected,
        &read_database_schema(&missing_actor_source)?,
    );

    let weakened_write_check = initial_project_state_schema()?;
    weakened_write_check.execute("DROP INDEX idx_write_checks_consumed_run", [])?;
    assert_schema_differs(
        "removing write check consumed-run uniqueness",
        &expected,
        &read_database_schema(&weakened_write_check)?,
    );

    Ok(())
}

#[test]
fn schema_comparison_ignores_harmless_sql_formatting() -> Result<(), Box<dyn Error>> {
    let sql = "
        CREATE TABLE sample (
          id TEXT PRIMARY KEY,
          value TEXT NOT NULL CHECK (value IN ('a', 'b'))
        );
        CREATE INDEX idx_sample_value ON sample (value);
    ";
    let project = read_database_schema(&build_schema_from_sql(sql)?)?;
    let reformatted_project =
        read_database_schema(&build_schema_from_sql(&harmlessly_reformat_sql(sql))?)?;
    assert_schema_eq(
        "schema with harmless SQL formatting changes",
        &project,
        &reformatted_project,
    );

    Ok(())
}

fn build_schema_from_sql(sql: &str) -> Result<Connection, Box<dyn Error>> {
    let conn = Connection::open_in_memory()?;
    enable_foreign_keys(&conn)?;
    conn.execute_batch(sql)?;
    Ok(conn)
}

fn initial_registry_schema() -> Result<Connection, Box<dyn Error>> {
    let mut conn = Connection::open_in_memory()?;
    enable_foreign_keys(&conn)?;
    apply_registry_migrations(&mut conn)?;
    validate_registry_schema(&conn)?;
    Ok(conn)
}

fn initial_project_state_schema() -> Result<Connection, Box<dyn Error>> {
    let mut conn = Connection::open_in_memory()?;
    enable_foreign_keys(&conn)?;
    apply_project_state_migrations(&mut conn)?;
    validate_project_state_schema(&conn)?;
    Ok(conn)
}

fn assert_columns_include(schema: &DatabaseSchema, table: &str, columns: &[&str]) {
    let table_schema = schema
        .tables
        .get(table)
        .unwrap_or_else(|| panic!("expected table {table}"));
    for column in columns {
        assert!(
            table_schema.columns.contains_key(*column),
            "expected {table}.{column}"
        );
    }
}

fn assert_unique_index_columns(schema: &DatabaseSchema, index: &str, columns: &[&str]) {
    let index_schema = schema
        .explicit_indexes
        .get(index)
        .unwrap_or_else(|| panic!("expected index {index}"));
    assert!(index_schema.unique, "expected {index} to be unique");
    let actual = index_schema
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(actual, columns, "unexpected columns for {index}");
}

fn read_database_schema(conn: &Connection) -> rusqlite::Result<DatabaseSchema> {
    let tables = read_tables(conn)?;
    let (explicit_indexes, unique_constraints) = read_indexes(conn, tables.keys())?;
    let triggers = read_triggers(conn)?;
    Ok(DatabaseSchema {
        tables,
        explicit_indexes,
        unique_constraints,
        triggers,
    })
}

fn read_tables(conn: &Connection) -> rusqlite::Result<BTreeMap<String, TableSchema>> {
    let mut stmt = conn.prepare(
        "SELECT name, sql
           FROM sqlite_master
          WHERE type = 'table'
            AND name NOT LIKE 'sqlite_%'
          ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut tables = BTreeMap::new();
    for row in rows {
        let (name, sql) = row?;
        tables.insert(
            name.clone(),
            TableSchema {
                columns: read_columns(conn, &name)?,
                foreign_keys: read_foreign_keys(conn, &name)?,
                check_constraints: extract_check_constraints(&sql),
            },
        );
    }
    Ok(tables)
}

fn read_columns(
    conn: &Connection,
    table: &str,
) -> rusqlite::Result<BTreeMap<String, ColumnSchema>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", quote_identifier(table)))?;
    let rows = stmt.query_map([], |row| {
        let name = row.get::<_, String>(1)?;
        Ok((
            name,
            ColumnSchema {
                declared_type: row.get::<_, String>(2)?.trim().to_ascii_uppercase(),
                not_null: row.get::<_, i64>(3)? != 0,
                default_value: normalize_default_value(row.get::<_, Option<String>>(4)?),
                primary_key_position: row.get(5)?,
            },
        ))
    })?;

    let mut columns = BTreeMap::new();
    for row in rows {
        let (name, column) = row?;
        columns.insert(name, column);
    }
    Ok(columns)
}

fn read_foreign_keys(
    conn: &Connection,
    table: &str,
) -> rusqlite::Result<BTreeSet<ForeignKeySchema>> {
    #[derive(Debug)]
    struct Row {
        id: i64,
        seq: i64,
        parent_table: String,
        child_column: String,
        parent_column: String,
        on_update: String,
        on_delete: String,
        match_name: String,
    }

    let mut stmt = conn.prepare(&format!(
        "PRAGMA foreign_key_list({})",
        quote_identifier(table)
    ))?;
    let rows = stmt.query_map([], |row| {
        Ok(Row {
            id: row.get(0)?,
            seq: row.get(1)?,
            parent_table: row.get(2)?,
            child_column: row.get(3)?,
            parent_column: row.get(4)?,
            on_update: row.get::<_, String>(5)?.to_ascii_uppercase(),
            on_delete: row.get::<_, String>(6)?.to_ascii_uppercase(),
            match_name: row.get::<_, String>(7)?.to_ascii_uppercase(),
        })
    })?;

    let mut grouped = BTreeMap::<i64, Vec<Row>>::new();
    for row in rows {
        let row = row?;
        grouped.entry(row.id).or_default().push(row);
    }

    let mut foreign_keys = BTreeSet::new();
    for (_, mut rows) in grouped {
        rows.sort_by_key(|row| row.seq);
        let first = rows
            .first()
            .expect("grouped foreign key rows must be non-empty");
        foreign_keys.insert(ForeignKeySchema {
            parent_table: first.parent_table.clone(),
            columns: rows
                .iter()
                .map(|row| ForeignKeyColumnSchema {
                    child_column: row.child_column.clone(),
                    parent_column: row.parent_column.clone(),
                })
                .collect(),
            on_update: first.on_update.clone(),
            on_delete: first.on_delete.clone(),
            match_name: first.match_name.clone(),
        });
    }

    Ok(foreign_keys)
}

fn read_indexes<'a>(
    conn: &Connection,
    tables: impl Iterator<Item = &'a String>,
) -> rusqlite::Result<(
    BTreeMap<String, IndexSchema>,
    BTreeSet<UniqueConstraintSchema>,
)> {
    let mut explicit_indexes = BTreeMap::new();
    let mut unique_constraints = BTreeSet::new();

    for table in tables {
        let mut stmt = conn.prepare(&format!("PRAGMA index_list({})", quote_identifier(table)))?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)? != 0,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)? != 0,
            ))
        })?;

        for row in rows {
            let (name, unique, origin, partial) = row?;
            let columns = read_index_columns(conn, &name)?;
            match origin.as_str() {
                "c" => {
                    explicit_indexes.insert(
                        name.clone(),
                        IndexSchema {
                            table: table.clone(),
                            unique,
                            columns,
                            partial_predicate: if partial {
                                Some(read_partial_index_predicate(conn, &name)?)
                            } else {
                                None
                            },
                        },
                    );
                }
                "u" => {
                    unique_constraints.insert(UniqueConstraintSchema {
                        table: table.clone(),
                        columns,
                    });
                }
                _ => {}
            }
        }
    }

    Ok((explicit_indexes, unique_constraints))
}

fn read_index_columns(
    conn: &Connection,
    index: &str,
) -> rusqlite::Result<Vec<IndexedColumnSchema>> {
    let mut stmt = conn.prepare(&format!("PRAGMA index_xinfo({})", quote_identifier(index)))?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)? != 0,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, i64>(5)? != 0,
        ))
    })?;

    let mut columns = Vec::new();
    for row in rows {
        let (seqno, name, descending, collation, key) = row?;
        if key {
            columns.push((
                seqno,
                IndexedColumnSchema {
                    name: name.unwrap_or_else(|| format!("<expression:{seqno}>")),
                    descending,
                    collation: collation
                        .unwrap_or_else(|| "BINARY".to_owned())
                        .to_ascii_uppercase(),
                },
            ));
        }
    }
    columns.sort_by_key(|(seqno, _)| *seqno);
    Ok(columns.into_iter().map(|(_, column)| column).collect())
}

fn read_partial_index_predicate(conn: &Connection, index: &str) -> rusqlite::Result<String> {
    let sql = conn.query_row(
        "SELECT sql
           FROM sqlite_master
          WHERE type = 'index'
            AND name = ?1",
        [index],
        |row| row.get::<_, String>(0),
    )?;
    find_keyword_outside_quotes(&sql, "where")
        .map(|index| normalize_sql_fragment(&sql[index + "where".len()..]))
        .ok_or_else(|| RusqliteError::InvalidQuery)
}

fn read_triggers(conn: &Connection) -> rusqlite::Result<BTreeMap<String, TriggerSchema>> {
    let mut stmt = conn.prepare(
        "SELECT name, tbl_name, sql
           FROM sqlite_master
          WHERE type = 'trigger'
            AND name NOT LIKE 'sqlite_%'
          ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            TriggerSchema {
                table: row.get(1)?,
                sql: normalize_sql_fragment(&row.get::<_, String>(2)?),
            },
        ))
    })?;

    let mut triggers = BTreeMap::new();
    for row in rows {
        let (name, trigger) = row?;
        triggers.insert(name, trigger);
    }
    Ok(triggers)
}

fn normalize_default_value(value: Option<String>) -> Option<String> {
    let value = value?;
    let trimmed = value.trim();
    if let Some(unquoted) = unquote_sql_string(trimmed) {
        if let Ok(json) = serde_json::from_str::<Value>(&unquoted) {
            if json.is_object() || json.is_array() {
                return Some(format!(
                    "json:{}",
                    serde_json::to_string(&json).expect("parsed JSON should serialize")
                ));
            }
        }
        Some(format!("string:{unquoted}"))
    } else {
        Some(format!("expr:{}", normalize_sql_fragment(trimmed)))
    }
}

fn unquote_sql_string(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() < 2 || bytes.first() != Some(&b'\'') || bytes.last() != Some(&b'\'') {
        return None;
    }
    Some(value[1..value.len() - 1].replace("''", "'"))
}

fn extract_check_constraints(sql: &str) -> BTreeSet<String> {
    let chars = sql.chars().collect::<Vec<_>>();
    let mut checks = BTreeSet::new();
    let mut index = 0;

    while index < chars.len() {
        if keyword_at(&chars, index, "check") {
            let mut open = index + "check".len();
            while open < chars.len() && chars[open].is_whitespace() {
                open += 1;
            }
            if open < chars.len() && chars[open] == '(' {
                if let Some((expression, end)) = balanced_parenthesized(&chars, open) {
                    checks.insert(normalize_sql_fragment(&expression));
                    index = end;
                    continue;
                }
            }
        }
        index += 1;
    }

    checks
}

fn balanced_parenthesized(chars: &[char], open: usize) -> Option<(String, usize)> {
    let mut depth = 0;
    let mut expression = String::new();
    let mut index = open;
    let mut in_single = false;
    let mut in_double = false;

    while index < chars.len() {
        let ch = chars[index];
        if in_single {
            expression.push(ch);
            if ch == '\'' {
                if chars.get(index + 1) == Some(&'\'') {
                    index += 1;
                    expression.push(chars[index]);
                } else {
                    in_single = false;
                }
            }
        } else if in_double {
            expression.push(ch);
            if ch == '"' {
                if chars.get(index + 1) == Some(&'"') {
                    index += 1;
                    expression.push(chars[index]);
                } else {
                    in_double = false;
                }
            }
        } else {
            match ch {
                '\'' => {
                    in_single = true;
                    expression.push(ch);
                }
                '"' => {
                    in_double = true;
                    expression.push(ch);
                }
                '(' => {
                    if depth > 0 {
                        expression.push(ch);
                    }
                    depth += 1;
                }
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((expression, index + 1));
                    }
                    expression.push(ch);
                }
                _ => expression.push(ch),
            }
        }
        index += 1;
    }

    None
}

fn normalize_sql_fragment(sql: &str) -> String {
    let mut normalized = String::new();
    let mut chars = sql.chars().peekable();
    let mut pending_space = false;

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            pending_space = true;
            continue;
        }

        if ch == '\'' {
            push_pending_space(&mut normalized, pending_space, true);
            pending_space = false;
            normalized.push(ch);
            while let Some(quoted) = chars.next() {
                normalized.push(quoted);
                if quoted == '\'' {
                    if chars.peek() == Some(&'\'') {
                        normalized.push(chars.next().expect("peeked quote exists"));
                    } else {
                        break;
                    }
                }
            }
            continue;
        }

        if ch == '"' {
            push_pending_space(&mut normalized, pending_space, true);
            pending_space = false;
            normalized.push(ch);
            while let Some(quoted) = chars.next() {
                normalized.push(quoted.to_ascii_lowercase());
                if quoted == '"' {
                    if chars.peek() == Some(&'"') {
                        normalized.push(chars.next().expect("peeked quote exists"));
                    } else {
                        break;
                    }
                }
            }
            continue;
        }

        if is_word_char(ch) {
            push_pending_space(&mut normalized, pending_space, true);
            normalized.push(ch.to_ascii_lowercase());
        } else {
            normalized.push(ch.to_ascii_lowercase());
        }
        pending_space = false;
    }

    normalized.trim().trim_end_matches(';').to_owned()
}

fn push_pending_space(normalized: &mut String, pending_space: bool, next_word_like: bool) {
    if pending_space
        && normalized
            .chars()
            .last()
            .is_some_and(|previous| next_word_like && is_word_char(previous))
    {
        normalized.push(' ');
    }
}

fn find_keyword_outside_quotes(sql: &str, keyword: &str) -> Option<usize> {
    let chars = sql.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut in_single = false;
    let mut in_double = false;

    while index < chars.len() {
        let ch = chars[index];
        if in_single {
            if ch == '\'' {
                if chars.get(index + 1) == Some(&'\'') {
                    index += 1;
                } else {
                    in_single = false;
                }
            }
        } else if in_double {
            if ch == '"' {
                if chars.get(index + 1) == Some(&'"') {
                    index += 1;
                } else {
                    in_double = false;
                }
            }
        } else if ch == '\'' {
            in_single = true;
        } else if ch == '"' {
            in_double = true;
        } else if keyword_at(&chars, index, keyword) {
            return Some(chars[..index].iter().map(|ch| ch.len_utf8()).sum());
        }
        index += 1;
    }

    None
}

fn keyword_at(chars: &[char], index: usize, keyword: &str) -> bool {
    let keyword_chars = keyword.chars().collect::<Vec<_>>();
    if index + keyword_chars.len() > chars.len() {
        return false;
    }
    if index > 0 && is_word_char(chars[index - 1]) {
        return false;
    }
    if index + keyword_chars.len() < chars.len() && is_word_char(chars[index + keyword_chars.len()])
    {
        return false;
    }
    chars[index..index + keyword_chars.len()]
        .iter()
        .zip(keyword_chars.iter())
        .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')
}

fn assert_project_contract_behavior(label: &str, conn: &Connection) -> Result<(), Box<dyn Error>> {
    insert_minimal_project_graph(conn)?;
    assert_user_judgments_status_is_closed(label, conn);
    assert_resolution_outcome_is_closed(label, conn);
    assert_resolution_machine_action_is_closed(label, conn);
    assert_user_judgments_require_basis(label, conn);
    assert_resolved_user_judgments_require_complete_resolution(label, conn);
    assert_project_continuity_value_sets_are_closed(label, conn);
    assert_write_check_status_is_closed(label, conn);
    assert_evidence_observation_value_sets_are_closed(label, conn);
    assert_tool_invocation_requires_identity(label, conn);
    assert_one_active_current_change_unit(label, conn);
    assert_artifacts_integrity_status_is_closed(label, conn);
    assert_verified_artifacts_require_integrity_facts(label, conn);
    assert_artifacts_body_path_shape(label, conn);
    Ok(())
}

fn insert_minimal_project_graph(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO project_state (
            project_id,
            storage_profile,
            schema_version,
            created_at,
            updated_at
        )
        VALUES (?1, ?2, ?3, 't0', 't0')",
        params!["project_a", STORAGE_PROFILE, PROJECT_STATE_SCHEMA_VERSION],
    )?;
    conn.execute(
        "INSERT INTO tasks (
            project_id,
            task_id,
            created_by_actor_source,
            mode,
            lifecycle_phase,
            created_at,
            updated_at
        )
        VALUES (
            'project_a',
            'task_a',
            'agent_connection:conn_main',
            'work',
            'shaping',
            't0',
            't0'
        )",
        [],
    )?;
    Ok(())
}

fn assert_user_judgments_status_is_closed(label: &str, conn: &Connection) {
    let error = conn
        .execute(
            "INSERT INTO user_judgments (
                project_id,
                judgment_id,
                task_id,
                judgment_kind,
                status,
                basis_json,
                requested_by_actor_source,
                requested_at
            )
            VALUES (
                'project_a',
                'judgment_bad_status',
                'task_a',
                'approval',
                'accepted',
                '{\"task_id\":\"task_a\",\"change_unit_id\":null,\"scope_revision\":0,\"close_basis_revision\":null,\"baseline_ref\":null,\"result_refs\":[],\"residual_risk_ids\":[],\"sensitive_action_scope\":null,\"created_at_state_version\":0,\"compatibility_status\":\"current\"}',
                'agent_connection:conn_main',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, error);
}

fn assert_resolution_outcome_is_closed(label: &str, conn: &Connection) {
    let error = conn
        .execute(
            "INSERT INTO user_judgments (
                project_id,
                judgment_id,
                task_id,
                judgment_kind,
                status,
                basis_json,
                resolution_outcome,
                resolution_machine_action,
                resolution_json,
                resolution_rationale_json,
                requested_by_actor_source,
                resolved_by_actor_source,
                resolved_verification_basis,
                resolved_assurance_level,
                resolved_at,
                requested_at
            )
            VALUES (
                'project_a',
                'judgment_bad_outcome',
                'task_a',
                'approval',
                'resolved',
                '{\"task_id\":\"task_a\",\"change_unit_id\":null,\"scope_revision\":0,\"close_basis_revision\":null,\"baseline_ref\":null,\"result_refs\":[],\"residual_risk_ids\":[],\"sensitive_action_scope\":null,\"created_at_state_version\":0,\"compatibility_status\":\"current\"}',
                'blocked',
                'accept',
                '{\"selected_option_id\":\"accept\",\"machine_action\":\"accept\",\"resolution_outcome\":\"blocked\",\"answer\":{\"product_decision\":{\"judgment\":{\"decision\":\"accepted\"}},\"technical_decision\":null,\"scope_decision\":null,\"sensitive_action_scope\":null,\"final_acceptance\":null,\"residual_risk_acceptance\":null,\"cancellation\":null},\"note\":null,\"accepted_risks\":[],\"resolved_by_actor_source\":\"local_user\"}',
                '{\"summary\":\"test rationale\",\"selected_reason\":\"test reason\",\"considered_alternatives\":[],\"rejected_alternatives\":[],\"assumptions\":[],\"tradeoffs\":[\"test tradeoff\"],\"uncertainties\":[],\"review_triggers\":[\"test trigger\"],\"related_refs\":[],\"artifact_refs\":[]}',
                'agent_connection:conn_main',
                'local_user',
                'fixture',
                'cooperative',
                't1',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, error);
}

fn assert_resolution_machine_action_is_closed(label: &str, conn: &Connection) {
    let error = conn
        .execute(
            "INSERT INTO user_judgments (
                project_id,
                judgment_id,
                task_id,
                judgment_kind,
                status,
                basis_json,
                resolution_outcome,
                resolution_machine_action,
                resolution_json,
                resolution_rationale_json,
                requested_by_actor_source,
                resolved_by_actor_source,
                resolved_verification_basis,
                resolved_assurance_level,
                resolved_at,
                requested_at
            )
            VALUES (
                'project_a',
                'judgment_bad_action',
                'task_a',
                'approval',
                'resolved',
                '{\"task_id\":\"task_a\",\"change_unit_id\":null,\"scope_revision\":0,\"close_basis_revision\":null,\"baseline_ref\":null,\"result_refs\":[],\"residual_risk_ids\":[],\"sensitive_action_scope\":null,\"created_at_state_version\":0,\"compatibility_status\":\"current\"}',
                'accepted',
                'approve',
                '{\"selected_option_id\":\"accept\",\"machine_action\":\"accept\",\"resolution_outcome\":\"accepted\",\"answer\":{\"product_decision\":{\"judgment\":{\"decision\":\"accepted\"}},\"technical_decision\":null,\"scope_decision\":null,\"sensitive_action_scope\":null,\"final_acceptance\":null,\"residual_risk_acceptance\":null,\"cancellation\":null},\"note\":null,\"accepted_risks\":[],\"resolved_by_actor_source\":\"local_user\"}',
                '{\"summary\":\"test rationale\",\"selected_reason\":\"test reason\",\"considered_alternatives\":[],\"rejected_alternatives\":[],\"assumptions\":[],\"tradeoffs\":[\"test tradeoff\"],\"uncertainties\":[],\"review_triggers\":[\"test trigger\"],\"related_refs\":[],\"artifact_refs\":[]}',
                'agent_connection:conn_main',
                'local_user',
                'fixture',
                'cooperative',
                't1',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, error);
}

fn assert_user_judgments_require_basis(label: &str, conn: &Connection) {
    let error = conn
        .execute(
            "INSERT INTO user_judgments (
                project_id,
                judgment_id,
                task_id,
                judgment_kind,
                status,
                requested_by_actor_source,
                requested_at
            )
            VALUES (
                'project_a',
                'judgment_missing_basis',
                'task_a',
                'approval',
                'pending',
                'agent_connection:conn_main',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, error);
}

fn assert_resolved_user_judgments_require_complete_resolution(label: &str, conn: &Connection) {
    let error = conn
        .execute(
            "INSERT INTO user_judgments (
                project_id,
                judgment_id,
                task_id,
                judgment_kind,
                status,
                basis_json,
                requested_by_actor_source,
                requested_at
            )
            VALUES (
                'project_a',
                'judgment_incomplete_resolution',
                'task_a',
                'approval',
                'resolved',
                '{\"task_id\":\"task_a\",\"change_unit_id\":null,\"scope_revision\":0,\"close_basis_revision\":null,\"baseline_ref\":null,\"result_refs\":[],\"residual_risk_ids\":[],\"sensitive_action_scope\":null,\"created_at_state_version\":0,\"compatibility_status\":\"current\"}',
                'agent_connection:conn_main',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, error);
}

fn assert_project_continuity_value_sets_are_closed(label: &str, conn: &Connection) {
    let bad_kind = conn
        .execute(
            "INSERT INTO project_continuity_records (
                project_id,
                continuity_record_id,
                source_task_id,
                kind,
                title,
                summary,
                status,
                created_at,
                updated_at
            )
            VALUES (
                'project_a',
                'continuity_bad_kind',
                'task_a',
                'authority',
                'Bad continuity kind',
                'Continuity records must stay inside the documented kind set.',
                'active',
                't1',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, bad_kind);

    let bad_status = conn
        .execute(
            "INSERT INTO project_continuity_records (
                project_id,
                continuity_record_id,
                source_task_id,
                kind,
                title,
                summary,
                status,
                created_at,
                updated_at
            )
            VALUES (
                'project_a',
                'continuity_bad_status',
                'task_a',
                'decision',
                'Bad continuity status',
                'Continuity status values must stay closed.',
                'current_authority',
                't1',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, bad_status);
}

fn assert_write_check_status_is_closed(label: &str, conn: &Connection) {
    let error = conn
        .execute(
            "INSERT INTO write_checks (
                project_id,
                write_check_id,
                task_id,
                basis_state_version,
                status,
                created_by_actor_source,
                expires_at,
                created_at
            )
            VALUES (
                'project_a',
                'write_check_bad_status',
                'task_a',
                1,
                'accepted',
                'agent_connection:conn_main',
                't2',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, error);
}

fn assert_evidence_observation_value_sets_are_closed(label: &str, conn: &Connection) {
    let bad_source = conn
        .execute(
            "INSERT INTO evidence_observations (
                project_id,
                evidence_observation_id,
                task_id,
                claim,
                source_kind,
                assurance_level,
                observed_at,
                recorded_at
            )
            VALUES (
                'project_a',
                'evidence_observation_bad_source',
                'task_a',
                'Close claim supported.',
                'final_acceptance',
                'external_tool_result',
                't1',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, bad_source);

    let bad_assurance = conn
        .execute(
            "INSERT INTO evidence_observations (
                project_id,
                evidence_observation_id,
                task_id,
                claim,
                source_kind,
                assurance_level,
                observed_at,
                recorded_at
            )
            VALUES (
                'project_a',
                'evidence_observation_bad_assurance',
                'task_a',
                'Close claim supported.',
                'external_tool',
                'accepted',
                't1',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, bad_assurance);
}

fn assert_tool_invocation_requires_identity(label: &str, conn: &Connection) {
    let error = conn
        .execute(
            "INSERT INTO tool_invocations (
                project_id,
                tool_name,
                idempotency_key,
                request_hash,
                basis_state_version,
                committed_state_version,
                response_json,
                created_at
            )
            VALUES (
                'project_a',
                'volicord.intake',
                'idem_verified_missing_context',
                'sha256:second',
                0,
                2,
                '{}',
                't2'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, error);
}

fn assert_one_active_current_change_unit(label: &str, conn: &Connection) {
    conn.execute(
        "INSERT INTO change_units (
            project_id,
            change_unit_id,
            task_id,
            status,
            is_current,
            created_at,
            updated_at
        )
        VALUES ('project_a', 'cu_current_1', 'task_a', 'active', 1, 't1', 't1')",
        [],
    )
    .expect("first current Change Unit should insert");

    let error = conn
        .execute(
            "INSERT INTO change_units (
                project_id,
                change_unit_id,
                task_id,
                status,
                is_current,
                created_at,
                updated_at
            )
            VALUES ('project_a', 'cu_current_2', 'task_a', 'active', 1, 't2', 't2')",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, error);
}

fn assert_artifacts_integrity_status_is_closed(label: &str, conn: &Connection) {
    let error = conn
        .execute(
            "INSERT INTO artifacts (
                project_id,
                artifact_id,
                task_id,
                uri,
                integrity_status,
                redaction_state,
                status,
                created_at,
                updated_at
            )
            VALUES (
                'project_a',
                'artifact_bad_integrity_status',
                'task_a',
                'volicord-artifact://project_a/artifact_bad_integrity_status',
                'legacy_unknown',
                'none',
                'unavailable',
                't1',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, error);
}

fn assert_verified_artifacts_require_integrity_facts(label: &str, conn: &Connection) {
    let error = conn
        .execute(
            "INSERT INTO artifacts (
                project_id,
                artifact_id,
                task_id,
                uri,
                integrity_status,
                redaction_state,
                status,
                created_at,
                updated_at
            )
            VALUES (
                'project_a',
                'artifact_verified_missing_facts',
                'task_a',
                'volicord-artifact://project_a/artifact_verified_missing_facts',
                'verified',
                'none',
                'available',
                't1',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, error);
}

fn assert_artifacts_body_path_shape(label: &str, conn: &Connection) {
    conn.execute(
        "INSERT INTO artifacts (
            project_id,
            artifact_id,
            task_id,
            uri,
            body_path,
            sha256,
            size_bytes,
            content_type,
            integrity_status,
            redaction_state,
            status,
            created_at,
            updated_at
        )
        VALUES (
            'project_a',
            'artifact_canonical_body_path',
            'task_a',
            'volicord-artifact://project_a/artifact_canonical_body_path',
            'tmp/canonical.txt',
            'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
            0,
            'text/plain',
            'verified',
            'none',
            'available',
            't1',
            't1'
        )",
        [],
    )
    .expect("canonical artifact-store-relative body_path should insert");

    for (artifact_id, body_path) in [
        ("artifact_empty_body_path", ""),
        ("artifact_absolute_body_path", "/tmp/absolute.txt"),
        ("artifact_parent_body_path", "../tmp/parent.txt"),
        ("artifact_nested_parent_body_path", "tmp/../parent.txt"),
        ("artifact_terminal_parent_body_path", "tmp/parent/.."),
        ("artifact_drive_prefix_body_path", "C:tmp/drive.txt"),
        ("artifact_backslash_body_path", r"tmp\backslash.txt"),
        ("artifact_project_home_dir_body_path", "artifacts"),
        (
            "artifact_project_home_body_path",
            "artifacts/tmp/obsolete.txt",
        ),
    ] {
        let error = conn
            .execute(
                "INSERT INTO artifacts (
                    project_id,
                    artifact_id,
                    task_id,
                    uri,
                    body_path,
                    sha256,
                    size_bytes,
                    content_type,
                    integrity_status,
                    redaction_state,
                    status,
                    created_at,
                    updated_at
                )
                VALUES (
                    'project_a',
                    ?1,
                    'task_a',
                    'volicord-artifact://project_a/bad_body_path',
                    ?2,
                    'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
                    0,
                    'text/plain',
                    'verified',
                    'none',
                    'available',
                    't1',
                    't1'
                )",
                params![artifact_id, body_path],
            )
            .unwrap_err();
        assert_constraint_error(label, error);
    }
}

fn assert_constraint_error(label: &str, error: RusqliteError) {
    match error {
        RusqliteError::SqliteFailure(failure, _) => {
            assert_eq!(
                failure.code,
                ErrorCode::ConstraintViolation,
                "{label}: expected SQLite constraint failure"
            );
        }
        other => panic!("{label}: expected SQLite constraint failure, got {other:?}"),
    }
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn assert_schema_eq(label: &str, expected: &DatabaseSchema, actual: &DatabaseSchema) {
    assert_eq!(expected, actual, "{label} schemas differ");
}

fn assert_schema_differs(label: &str, expected: &DatabaseSchema, actual: &DatabaseSchema) {
    assert_ne!(
        expected, actual,
        "{label} should change the normalized schema comparison"
    );
}

fn harmlessly_reformat_sql(sql: &str) -> String {
    sql.replace("CREATE UNIQUE INDEX", "create\n  unique\n  index")
        .replace("CREATE INDEX", "create\n  index")
        .replace("CREATE TABLE", "create\n  table")
        .replace("CREATE TRIGGER", "create\n  trigger")
        .replace("FOREIGN KEY", "foreign\n  key")
        .replace("CHECK", "check")
        .replace(" DEFAULT ", "\n  default\n  ")
}
