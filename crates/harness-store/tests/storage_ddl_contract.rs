use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    io::{Error as IoError, ErrorKind},
    path::{Path, PathBuf},
};

use harness_store::{
    migrations::{
        apply_project_state_migrations, apply_registry_migrations, PROJECT_STATE_SCHEMA_VERSION,
        STORAGE_PROFILE,
    },
    sqlite::{enable_foreign_keys, validate_project_state_schema, validate_registry_schema},
};
use rusqlite::{params, Connection, Error as RusqliteError, ErrorCode};
use serde_json::Value;

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

#[derive(Debug, Clone)]
struct StorageDdlSql {
    registry: String,
    project_state: String,
}

#[test]
fn storage_ddl_documents_match_initial_schemas() -> Result<(), Box<dyn Error>> {
    let english = load_storage_ddl_sql("en")?;
    let korean = load_storage_ddl_sql("ko")?;

    let english_registry = build_schema_from_sql(&english.registry)?;
    let korean_registry = build_schema_from_sql(&korean.registry)?;
    let initial_registry = initial_registry_schema()?;

    let english_registry_schema = read_database_schema(&english_registry)?;
    let korean_registry_schema = read_database_schema(&korean_registry)?;
    let initial_registry_schema = read_database_schema(&initial_registry)?;

    assert_schema_eq(
        "English and Korean registry.sqlite DDL",
        &english_registry_schema,
        &korean_registry_schema,
    );
    assert_schema_eq(
        "Storage DDL registry.sqlite and initial registry.sqlite",
        &english_registry_schema,
        &initial_registry_schema,
    );

    let english_project = build_schema_from_sql(&english.project_state)?;
    let korean_project = build_schema_from_sql(&korean.project_state)?;
    let initial_project = initial_project_state_schema()?;

    let english_project_schema = read_database_schema(&english_project)?;
    let korean_project_schema = read_database_schema(&korean_project)?;
    let initial_project_schema = read_database_schema(&initial_project)?;

    assert_schema_eq(
        "English and Korean project state.sqlite DDL",
        &english_project_schema,
        &korean_project_schema,
    );
    assert_schema_eq(
        "Storage DDL project state.sqlite and initial state.sqlite",
        &english_project_schema,
        &initial_project_schema,
    );

    assert_project_contract_behavior("English project state.sqlite DDL", &english_project)?;
    assert_project_contract_behavior("Korean project state.sqlite DDL", &korean_project)?;
    assert_project_contract_behavior("initial project state.sqlite", &initial_project)?;

    Ok(())
}

#[test]
fn schema_comparison_detects_contract_critical_drift() -> Result<(), Box<dyn Error>> {
    let english = load_storage_ddl_sql("en")?;
    let expected_conn = build_schema_from_sql(&english.project_state)?;
    let expected = read_database_schema(&expected_conn)?;

    let without_enforcement_profile = replace_required(
        &english.project_state,
        "  enforcement_profile_json TEXT NOT NULL DEFAULT '{\"profile_id\":\"baseline_cooperative\",\"guarantee_level\":\"cooperative\",\"enabled_mechanisms\":[],\"source\":\"baseline_scope\",\"status\":\"active\"}',\n",
        "",
    );
    assert_schema_differs(
        "removing project_state.enforcement_profile_json",
        &expected,
        &read_database_schema(&build_schema_from_sql(&without_enforcement_profile)?)?,
    );

    let weakened_replay_identity = replace_required(
        &english.project_state,
        "  surface_id TEXT NOT NULL,\n  surface_instance_id TEXT NOT NULL,\n  access_class TEXT NOT NULL,\n",
        "  surface_id TEXT,\n  surface_instance_id TEXT,\n  access_class TEXT,\n",
    );
    assert_schema_differs(
        "weakening tool_invocations replay identity requiredness",
        &expected,
        &read_database_schema(&build_schema_from_sql(&weakened_replay_identity)?)?,
    );

    let weakened_replay_foreign_key = replace_required(
        &english.project_state,
        "  FOREIGN KEY (project_id, surface_id, surface_instance_id)\n    REFERENCES surfaces (project_id, surface_id, surface_instance_id)\n    ON DELETE RESTRICT,\n",
        "  FOREIGN KEY (project_id, surface_id, surface_instance_id)\n    REFERENCES surfaces (project_id, surface_id, surface_instance_id),\n",
    );
    assert_schema_differs(
        "weakening verified replay-context surface foreign key delete action",
        &expected,
        &read_database_schema(&build_schema_from_sql(&weakened_replay_foreign_key)?)?,
    );

    let weakened_resolution_group = replace_required(
        &english.project_state,
        "      status = 'resolved'\n      AND resolution_outcome IS NOT NULL",
        "      status = 'resolved'\n      AND resolution_outcome IS NULL",
    );
    assert_schema_differs(
        "weakening resolved user_judgments resolution completeness",
        &expected,
        &read_database_schema(&build_schema_from_sql(&weakened_resolution_group)?)?,
    );

    Ok(())
}

#[test]
fn schema_comparison_ignores_harmless_sql_formatting() -> Result<(), Box<dyn Error>> {
    let english = load_storage_ddl_sql("en")?;

    let registry = read_database_schema(&build_schema_from_sql(&english.registry)?)?;
    let reformatted_registry = read_database_schema(&build_schema_from_sql(
        &harmlessly_reformat_sql(&english.registry),
    )?)?;
    assert_schema_eq(
        "registry.sqlite DDL with harmless SQL formatting changes",
        &registry,
        &reformatted_registry,
    );

    let project = read_database_schema(&build_schema_from_sql(&english.project_state)?)?;
    let reformatted_project = read_database_schema(&build_schema_from_sql(
        &harmlessly_reformat_sql(&english.project_state),
    )?)?;
    assert_schema_eq(
        "project state.sqlite DDL with harmless SQL formatting changes",
        &project,
        &reformatted_project,
    );

    Ok(())
}

fn load_storage_ddl_sql(language: &str) -> Result<StorageDdlSql, Box<dyn Error>> {
    let path = repo_root()
        .join("docs")
        .join(language)
        .join("reference")
        .join("storage-ddl.md");
    let markdown = fs::read_to_string(&path)?;
    Ok(StorageDdlSql {
        registry: extract_database_sql(&markdown, "registry.sqlite", 1, &path)?,
        project_state: extract_database_sql(&markdown, "state.sqlite", 2, &path)?,
    })
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("harness-store must live under crates/")
        .to_path_buf()
}

fn extract_database_sql(
    markdown: &str,
    heading_token: &str,
    expected_sql_blocks: usize,
    path: &Path,
) -> Result<String, IoError> {
    let lines = markdown.lines().collect::<Vec<_>>();
    let section_start = lines
        .iter()
        .position(|line| line.starts_with("## ") && line.contains(heading_token))
        .ok_or_else(|| {
            invalid_data(format!(
                "{} does not contain a database section for {heading_token}",
                path.display()
            ))
        })?;
    let section_end = lines
        .iter()
        .enumerate()
        .skip(section_start + 1)
        .find_map(|(index, line)| line.starts_with("## ").then_some(index))
        .unwrap_or(lines.len());
    let section = lines[section_start + 1..section_end].join("\n");
    let blocks = sql_fence_blocks(&section);
    if blocks.len() != expected_sql_blocks {
        return Err(invalid_data(format!(
            "{} section {heading_token} has {} SQL blocks, expected {expected_sql_blocks}",
            path.display(),
            blocks.len()
        )));
    }
    Ok(blocks.join("\n\n"))
}

fn sql_fence_blocks(section: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut in_sql = false;

    for line in section.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            if in_sql {
                blocks.push(current.join("\n"));
                current.clear();
                in_sql = false;
            } else {
                let info = trimmed.trim_start_matches("```").trim();
                in_sql = info.eq_ignore_ascii_case("sql");
            }
            continue;
        }

        if in_sql {
            current.push(line);
        }
    }

    blocks
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
    assert_resolution_machine_action_is_closed(label, conn);
    assert_resolved_surface_foreign_key_is_enforced(label, conn);
    assert_user_judgments_require_basis(label, conn);
    assert_resolved_user_judgments_require_complete_resolution(label, conn);
    assert_tool_invocation_requires_identity(label, conn);
    assert_one_active_current_change_unit(label, conn);
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
        "INSERT INTO surfaces (
            project_id,
            surface_id,
            surface_instance_id,
            surface_kind,
            registered_at
        )
        VALUES ('project_a', 'surface_main', 'surface_instance_1', 'cli', 't0')",
        [],
    )?;
    conn.execute(
        "INSERT INTO tasks (
            project_id,
            task_id,
            created_by_surface_id,
            created_by_surface_instance_id,
            mode,
            lifecycle_phase,
            created_at,
            updated_at
        )
        VALUES (
            'project_a',
            'task_a',
            'surface_main',
            'surface_instance_1',
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
                requested_by_surface_id,
                requested_by_surface_instance_id,
                requested_at
            )
            VALUES (
                'project_a',
                'judgment_bad_status',
                'task_a',
                'approval',
                'accepted',
                '{\"task_id\":\"task_a\",\"change_unit_id\":null,\"scope_revision\":0,\"close_basis_revision\":null,\"baseline_ref\":null,\"result_refs\":[],\"residual_risk_ids\":[],\"sensitive_action_scope\":null,\"created_at_state_version\":0,\"compatibility_status\":\"current\"}',
                'surface_main',
                'surface_instance_1',
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
                requested_by_surface_id,
                requested_by_surface_instance_id,
                resolved_by_actor_kind,
                resolved_actor_role,
                resolved_by_surface_id,
                resolved_by_surface_instance_id,
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
                '{\"selected_option_id\":\"accept\",\"machine_action\":\"accept\",\"resolution_outcome\":\"accepted\",\"answer\":{\"product_decision\":{\"judgment\":{\"decision\":\"accepted\"}},\"technical_decision\":null,\"scope_decision\":null,\"sensitive_action_scope\":null,\"final_acceptance\":null,\"residual_risk_acceptance\":null,\"cancellation\":null},\"note\":null,\"accepted_risks\":[],\"resolved_by_actor_kind\":\"user\"}',
                'surface_main',
                'surface_instance_1',
                'user',
                'user_interaction',
                'surface_main',
                'surface_instance_1',
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

fn assert_resolved_surface_foreign_key_is_enforced(label: &str, conn: &Connection) {
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
                requested_by_surface_id,
                requested_by_surface_instance_id,
                resolved_by_actor_kind,
                resolved_actor_role,
                resolved_by_surface_id,
                resolved_by_surface_instance_id,
                resolved_verification_basis,
                resolved_assurance_level,
                resolved_at,
                requested_at
            )
            VALUES (
                'project_a',
                'judgment_bad_resolved_surface',
                'task_a',
                'approval',
                'resolved',
                '{\"task_id\":\"task_a\",\"change_unit_id\":null,\"scope_revision\":0,\"close_basis_revision\":null,\"baseline_ref\":null,\"result_refs\":[],\"residual_risk_ids\":[],\"sensitive_action_scope\":null,\"created_at_state_version\":0,\"compatibility_status\":\"current\"}',
                'accepted',
                'accept',
                '{\"selected_option_id\":\"accept\",\"machine_action\":\"accept\",\"resolution_outcome\":\"accepted\",\"answer\":{\"product_decision\":{\"judgment\":{\"decision\":\"accepted\"}},\"technical_decision\":null,\"scope_decision\":null,\"sensitive_action_scope\":null,\"final_acceptance\":null,\"residual_risk_acceptance\":null,\"cancellation\":null},\"note\":null,\"accepted_risks\":[],\"resolved_by_actor_kind\":\"user\"}',
                'surface_main',
                'surface_instance_1',
                'user',
                'user_interaction',
                'missing_surface',
                'missing_surface_instance',
                'fixture',
                'cooperative',
                't1',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_foreign_key_error(label, error);
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
                requested_by_surface_id,
                requested_by_surface_instance_id,
                requested_at
            )
            VALUES (
                'project_a',
                'judgment_missing_basis',
                'task_a',
                'approval',
                'pending',
                'surface_main',
                'surface_instance_1',
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
                requested_by_surface_id,
                requested_by_surface_instance_id,
                requested_at
            )
            VALUES (
                'project_a',
                'judgment_incomplete_resolution',
                'task_a',
                'approval',
                'resolved',
                '{\"task_id\":\"task_a\",\"change_unit_id\":null,\"scope_revision\":0,\"close_basis_revision\":null,\"baseline_ref\":null,\"result_refs\":[],\"residual_risk_ids\":[],\"sensitive_action_scope\":null,\"created_at_state_version\":0,\"compatibility_status\":\"current\"}',
                'surface_main',
                'surface_instance_1',
                't1'
            )",
            [],
        )
        .unwrap_err();
    assert_constraint_error(label, error);
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
                'harness.intake',
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

fn assert_foreign_key_error(label: &str, error: RusqliteError) {
    match error {
        RusqliteError::SqliteFailure(failure, _) => {
            assert_eq!(
                failure.code,
                ErrorCode::ConstraintViolation,
                "{label}: expected SQLite foreign-key failure"
            );
            assert_eq!(
                failure.extended_code,
                rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY,
                "{label}: expected foreign-key extended code"
            );
        }
        other => panic!("{label}: expected SQLite foreign-key failure, got {other:?}"),
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

fn replace_required(input: &str, needle: &str, replacement: &str) -> String {
    let count = input.matches(needle).count();
    assert_eq!(count, 1, "expected exactly one occurrence of {needle:?}");
    input.replacen(needle, replacement, 1)
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

fn invalid_data(message: String) -> IoError {
    IoError::new(ErrorKind::InvalidData, message)
}
