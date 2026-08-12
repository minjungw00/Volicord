use crate::store::{
    decision_choice_is_valid, decision_outcome, decode_alternatives, decode_source_ids,
    is_current_host_user_authority, meaning_preserving_correction, CURRENT_HOST_USER_AUTHORITY,
};
use crate::{
    CanonicalRecordId, CheckpointId, ContextItemId, DecisionId, Error, ErrorKind, OperationId,
    OperationResult, ProjectId, QuestionId, SourceId, Store,
};
use rusqlite::types::{Value, ValueRef};
use rusqlite::{params, params_from_iter, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const BUNDLE_KIND: &str = "volicord-context-bundle";
pub const BUNDLE_FORMAT_VERSION: u32 = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleExport {
    pub project_id: ProjectId,
    pub checksum: String,
    pub history_basis: String,
    pub path: PathBuf,
    pub bytes_written: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundleImportStatus {
    Imported,
    AlreadyPresent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleImport {
    pub project_id: ProjectId,
    pub checksum: String,
    pub history_basis: String,
    pub status: BundleImportStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Envelope {
    checksum: String,
    format_version: u32,
    kind: String,
    payload: Payload,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Payload {
    pub(crate) lineage: Lineage,
    pub(crate) project_id: String,
    pub(crate) tables: Vec<PortableTable>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Lineage {
    pub(crate) common_base_basis: String,
    pub(crate) history_basis: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct SemanticState {
    pub(crate) project_id: String,
    pub(crate) tables: Vec<PortableTable>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PortableTable {
    pub(crate) columns: Vec<String>,
    pub(crate) name: String,
    pub(crate) rows: Vec<Vec<PortableValue>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub(crate) enum PortableValue {
    Null,
    Integer(i64),
    Text(String),
    Bytes(String),
}

pub(crate) struct TableSpec {
    pub(crate) name: &'static str,
    pub(crate) columns: &'static [&'static str],
    pub(crate) primary_key: &'static [usize],
    pub(crate) project_column: usize,
    pub(crate) order_by: &'static str,
}

pub(crate) const TABLES: &[TableSpec] = &[
    TableSpec {
        name: "projects",
        columns: &["id", "display_name", "revision", "created_at", "updated_at"],
        primary_key: &[0],
        project_column: 0,
        order_by: "id",
    },
    TableSpec {
        name: "project_revisions",
        columns: &["project_id", "revision", "display_name", "recorded_at"],
        primary_key: &[0, 1],
        project_column: 0,
        order_by: "revision",
    },
    TableSpec {
        name: "sources",
        columns: &[
            "id",
            "project_id",
            "revision",
            "source_kind",
            "locator",
            "snapshot_basis",
            "detail_one",
            "detail_two",
            "exit_code",
            "termination",
            "actor_kind",
            "actor_identity",
            "observer_kind",
            "observer_identity",
            "availability",
            "recorded_at",
        ],
        primary_key: &[0],
        project_column: 1,
        order_by: "id",
    },
    TableSpec {
        name: "source_relations",
        columns: &[
            "project_id",
            "from_source_id",
            "relation_kind",
            "to_source_id",
            "recorded_at",
        ],
        primary_key: &[0, 1, 2, 3],
        project_column: 0,
        order_by: "from_source_id, relation_kind, to_source_id",
    },
    TableSpec {
        name: "questions",
        columns: &[
            "id",
            "project_id",
            "revision",
            "terminal_outcome",
            "created_at",
            "updated_at",
        ],
        primary_key: &[0],
        project_column: 1,
        order_by: "id",
    },
    TableSpec {
        name: "question_revisions",
        columns: &[
            "question_id",
            "revision",
            "project_id",
            "prompt_basis",
            "source_basis",
            "dependencies",
            "alternatives",
            "recommendation_key",
            "recommendation_rationale",
            "recommendation_sources",
            "trade_offs",
            "uncertainty",
            "material_scope",
            "recorded_at",
        ],
        primary_key: &[0, 1],
        project_column: 2,
        order_by: "question_id, revision",
    },
    TableSpec {
        name: "question_response_sources",
        columns: &[
            "project_id",
            "question_id",
            "question_revision",
            "source_id",
            "recorded_at",
        ],
        primary_key: &[0, 1, 2],
        project_column: 0,
        order_by: "question_id, question_revision",
    },
    TableSpec {
        name: "question_decision_history_witnesses",
        columns: &[
            "project_id",
            "question_id",
            "question_revision",
            "root_decision_id",
            "terminal_outcome",
            "response_source_id",
            "response_authority",
            "creation_kind",
            "created_at",
        ],
        primary_key: &[0, 1, 2],
        project_column: 0,
        order_by: "question_id, question_revision",
    },
    TableSpec {
        name: "decisions",
        columns: &[
            "id",
            "project_id",
            "revision",
            "question_id",
            "question_revision",
            "user_turn_source_id",
            "user_authority",
            "choice_kind",
            "choice_value",
            "user_rationale",
            "displayed_alternatives",
            "recommendation_key",
            "recommendation_rationale",
            "recommendation_sources",
            "applicability_paths",
            "applicability_components",
            "applicability_work_contexts",
            "assumptions",
            "revisit_triggers",
            "recorded_at",
        ],
        primary_key: &[0],
        project_column: 1,
        order_by: "id",
    },
    TableSpec {
        name: "decision_revisions",
        columns: &[
            "decision_id",
            "revision",
            "project_id",
            "question_id",
            "question_revision",
            "user_turn_source_id",
            "user_authority",
            "choice_kind",
            "choice_value",
            "user_rationale",
            "displayed_alternatives",
            "recommendation_key",
            "recommendation_rationale",
            "recommendation_sources",
            "applicability_paths",
            "applicability_components",
            "applicability_work_contexts",
            "assumptions",
            "revisit_triggers",
            "correction_kind",
            "authorization_source_id",
            "authorization_authority",
            "recorded_at",
        ],
        primary_key: &[0, 1],
        project_column: 2,
        order_by: "decision_id, revision",
    },
    TableSpec {
        name: "context_items",
        columns: &[
            "id",
            "project_id",
            "revision",
            "role",
            "statement",
            "provenance_role",
            "author_kind",
            "author_identity",
            "applicability_paths",
            "applicability_components",
            "applicability_work_contexts",
            "recorded_at",
        ],
        primary_key: &[0],
        project_column: 1,
        order_by: "id",
    },
    TableSpec {
        name: "context_item_sources",
        columns: &["project_id", "context_item_id", "source_id", "position"],
        primary_key: &[1, 3],
        project_column: 0,
        order_by: "context_item_id, position",
    },
    TableSpec {
        name: "context_item_revisions",
        columns: &[
            "context_item_id",
            "revision",
            "project_id",
            "role",
            "statement",
            "provenance_role",
            "author_kind",
            "author_identity",
            "source_basis",
            "applicability_paths",
            "applicability_components",
            "applicability_work_contexts",
            "correction_kind",
            "authorization_source_id",
            "recorded_at",
        ],
        primary_key: &[0, 1],
        project_column: 2,
        order_by: "context_item_id, revision",
    },
    TableSpec {
        name: "checkpoints",
        columns: &[
            "id",
            "project_id",
            "revision",
            "checkpoint_kind",
            "goal",
            "work_state",
            "state_change",
            "changed_paths",
            "user_review",
            "user_review_source_id",
            "user_acceptance",
            "user_acceptance_source_id",
            "known_limits",
            "non_goals",
            "next_step",
            "handoff_to",
            "recorded_at",
        ],
        primary_key: &[0],
        project_column: 1,
        order_by: "id",
    },
    TableSpec {
        name: "checkpoint_source_relations",
        columns: &[
            "project_id",
            "checkpoint_id",
            "relation_kind",
            "source_id",
            "position",
        ],
        primary_key: &[1, 2, 4],
        project_column: 0,
        order_by: "checkpoint_id, relation_kind, position",
    },
    TableSpec {
        name: "checkpoint_decisions",
        columns: &["project_id", "checkpoint_id", "decision_id", "position"],
        primary_key: &[1, 3],
        project_column: 0,
        order_by: "checkpoint_id, position",
    },
    TableSpec {
        name: "checkpoint_questions",
        columns: &[
            "project_id",
            "checkpoint_id",
            "question_id",
            "question_revision",
            "position",
        ],
        primary_key: &[1, 4],
        project_column: 0,
        order_by: "checkpoint_id, position",
    },
    TableSpec {
        name: "checkpoint_verifications",
        columns: &[
            "project_id",
            "checkpoint_id",
            "position",
            "verification_state",
            "source_id",
            "outcome",
        ],
        primary_key: &[1, 2],
        project_column: 0,
        order_by: "checkpoint_id, position",
    },
    TableSpec {
        name: "canonical_relations",
        columns: &[
            "project_id",
            "from_kind",
            "from_id",
            "relation_kind",
            "to_kind",
            "to_id",
            "recorded_at",
        ],
        primary_key: &[0, 1, 2, 3, 4, 5],
        project_column: 0,
        order_by: "from_kind, from_id, relation_kind, to_kind, to_id",
    },
    TableSpec {
        name: "review_due",
        columns: &[
            "project_id",
            "decision_id",
            "review_kind",
            "explanation",
            "source_basis",
            "marked_at",
        ],
        primary_key: &[0, 1],
        project_column: 0,
        order_by: "decision_id",
    },
    TableSpec {
        name: "tombstones",
        columns: &["project_id", "record_kind", "record_id", "forgotten_at"],
        primary_key: &[0, 1, 2],
        project_column: 0,
        order_by: "record_kind, record_id",
    },
    TableSpec {
        name: "merge_events",
        columns: &[
            "operation_id",
            "project_id",
            "conflict_set_id",
            "conflict_revision",
            "common_base_basis",
            "local_history_basis",
            "incoming_history_basis",
            "result_history_basis",
            "resolution_kind",
            "resolution_source_id",
            "conflict_classes",
            "affected_identities",
            "branch_history_basis",
            "committed_at",
        ],
        primary_key: &[0],
        project_column: 1,
        order_by: "operation_id",
    },
];

impl Store {
    pub fn export_bundle(
        &mut self,
        project_id: ProjectId,
        path: impl AsRef<Path>,
    ) -> Result<BundleExport, Error> {
        let path = validate_final_path(path.as_ref())?;
        let (bytes, checksum, history_basis) = bundle_bytes(self, project_id)?;
        publish_atomic(&path, &bytes)?;
        self.connection.execute(
            "INSERT OR IGNORE INTO bundle_lineage(project_id, common_base_basis) VALUES (?1, ?2)",
            params![project_id.as_bytes().as_slice(), history_basis],
        ).map_err(storage_write)?;
        self.connection.execute(
            "INSERT OR IGNORE INTO managed_bundle_paths(project_id, absolute_path) VALUES (?1, ?2)",
            params![project_id.as_bytes().as_slice(), path_text(&path)?],
        ).map_err(storage_write)?;
        Ok(BundleExport {
            project_id,
            checksum,
            history_basis,
            path,
            bytes_written: u64::try_from(bytes.len()).map_err(|_| {
                Error::new(
                    ErrorKind::StorageUnavailable,
                    "bundle is too large to report",
                )
            })?,
        })
    }

    pub fn import_bundle(
        &mut self,
        operation_id: OperationId,
        path: impl AsRef<Path>,
    ) -> Result<OperationResult<BundleImport>, Error> {
        let path = validate_final_path(path.as_ref())?;
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|error| {
                Error::with_source(
                    ErrorKind::StorageUnavailable,
                    format!("cannot open bundle {}", path.display()),
                    error,
                )
            })?
            .read_to_end(&mut bytes)
            .map_err(|error| {
                Error::with_source(
                    ErrorKind::StorageUnavailable,
                    format!("cannot read bundle {}", path.display()),
                    error,
                )
            })?;
        let validated = validate_bundle(&bytes)?;
        let project_id = validated.project_id;
        let checksum = validated.checksum.clone();
        let history_basis = validated.payload.lineage.history_basis.clone();
        let (connection, clock) = (&mut self.connection, &mut self.clock);
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_write)?;
        if let Some((kind, basis, replay_state, result_id, result_revision)) = transaction.query_row(
            "SELECT operation_kind, input_basis, replay_state, result_id, result_revision FROM operations WHERE operation_id = ?1",
            [operation_id.as_bytes().as_slice()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?, row.get::<_, String>(2)?, row.get::<_, Vec<u8>>(3)?, row.get::<_, i64>(4)?)),
        ).optional().map_err(storage_read)? {
            if replay_state == "forgotten_dependency" {
                return Err(Error::new(ErrorKind::NotFound, "import replay input depended on forgotten canonical content"));
            }
            if kind != "import_bundle" || basis != checksum.as_bytes() || result_id != project_id.as_bytes() || !(result_revision == 0 || result_revision == 1) {
                return Err(Error::new(ErrorKind::DomainConflict, "OperationId was already committed with different bundle input"));
            }
            transaction.commit().map_err(storage_commit)?;
            return Ok(OperationResult {
                value: BundleImport { project_id, checksum, history_basis, status: if result_revision == 0 { BundleImportStatus::Imported } else { BundleImportStatus::AlreadyPresent } },
                replayed: true,
            });
        }
        let project_exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1)",
                [project_id.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .map_err(storage_read)?;
        let mut missing = Vec::new();
        for (spec, table) in TABLES.iter().zip(&validated.payload.tables) {
            for row in &table.rows {
                match compare_row(&transaction, spec, row)? {
                    RowState::Absent => missing.push((spec, row)),
                    RowState::Equal => {}
                    RowState::Different => {
                        return Err(Error::new(
                            ErrorKind::DomainConflict,
                            format!("bundle conflicts with local {} identity", spec.name),
                        ))
                    }
                }
            }
        }
        if project_exists && !missing.is_empty() {
            return Err(Error::new(
                ErrorKind::DomainConflict,
                "bundle would merge divergent canonical state; merge is not implemented",
            ));
        }
        let status = if missing.is_empty() {
            BundleImportStatus::AlreadyPresent
        } else {
            BundleImportStatus::Imported
        };
        for (spec, row) in missing {
            insert_row(&transaction, spec, row)?;
        }
        transaction.execute(
            "INSERT OR IGNORE INTO bundle_lineage(project_id, common_base_basis) VALUES (?1, ?2)",
            params![project_id.as_bytes().as_slice(), history_basis],
        ).map_err(storage_write)?;
        let now = clock.now()?;
        let dependencies = payload_dependencies(&validated.payload)?;
        crate::store::record_operation(
            &transaction,
            operation_id,
            project_id,
            "import_bundle",
            checksum.as_bytes(),
            "bundle_import",
            project_id.as_bytes(),
            if status == BundleImportStatus::Imported {
                0
            } else {
                1
            },
            now,
            &dependencies,
        )?;
        transaction.commit().map_err(storage_commit)?;
        Ok(OperationResult {
            value: BundleImport {
                project_id,
                checksum,
                history_basis,
                status,
            },
            replayed: false,
        })
    }
}

pub(crate) fn payload_dependencies(payload: &Payload) -> Result<Vec<CanonicalRecordId>, Error> {
    let mut dependencies = Vec::new();
    for table in &payload.tables {
        for row in &table.rows {
            let Some(value) = row.first() else {
                continue;
            };
            let bytes = value_bytes(value)?;
            let dependency = match table.name.as_str() {
                "projects" => CanonicalRecordId::Project(ProjectId::from_slice(&bytes)?),
                "sources" => CanonicalRecordId::Source(SourceId::from_slice(&bytes)?),
                "questions" => CanonicalRecordId::Question(QuestionId::from_slice(&bytes)?),
                "decisions" => CanonicalRecordId::Decision(DecisionId::from_slice(&bytes)?),
                "context_items" => {
                    CanonicalRecordId::ContextItem(ContextItemId::from_slice(&bytes)?)
                }
                "checkpoints" => CanonicalRecordId::Checkpoint(CheckpointId::from_slice(&bytes)?),
                _ => continue,
            };
            dependencies.push(dependency);
        }
    }
    Ok(dependencies)
}

pub(crate) fn refresh_managed_bundles(store: &Store, project_id: ProjectId) -> Result<(), Error> {
    let mut statement = store.connection.prepare(
        "SELECT absolute_path FROM managed_bundle_paths WHERE project_id = ?1 ORDER BY absolute_path",
    ).map_err(storage_read)?;
    let rows = statement
        .query_map([project_id.as_bytes().as_slice()], |row| {
            row.get::<_, String>(0)
        })
        .map_err(storage_read)?;
    let mut paths = Vec::new();
    for row in rows {
        paths.push(PathBuf::from(row.map_err(storage_read)?));
    }
    if paths.is_empty() {
        return Ok(());
    }
    let (bytes, _, _) = bundle_bytes(store, project_id)?;
    for path in paths {
        publish_atomic(&path, &bytes)?;
    }
    Ok(())
}

pub(crate) struct ValidatedBundle {
    pub(crate) payload: Payload,
    pub(crate) project_id: ProjectId,
    pub(crate) checksum: String,
}

pub(crate) fn validate_bundle(bytes: &[u8]) -> Result<ValidatedBundle, Error> {
    if !bytes.ends_with(b"\n") || bytes.contains(&b'\r') {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "bundle must be LF-terminated UTF-8 JSON",
        ));
    }
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(|error| {
        Error::with_source(ErrorKind::CorruptState, "bundle JSON is malformed", error)
    })?;
    if envelope.kind != BUNDLE_KIND {
        return Err(Error::new(
            ErrorKind::CorruptState,
            format!("unexpected bundle kind {:?}", envelope.kind),
        ));
    }
    if envelope.format_version != BUNDLE_FORMAT_VERSION {
        return Err(Error::new(
            ErrorKind::UnsupportedVersion,
            format!(
                "portable bundle format version {} is unsupported; current version is {}",
                envelope.format_version, BUNDLE_FORMAT_VERSION
            ),
        ));
    }
    let payload_bytes = serde_json::to_vec(&envelope.payload).map_err(json_error)?;
    let checksum = sha256_hex(&payload_bytes);
    if checksum != envelope.checksum {
        return Err(Error::new(
            ErrorKind::IntegrityFailure,
            "portable bundle checksum does not match its payload",
        ));
    }
    let project_id = parse_project_id(&envelope.payload.project_id)?;
    crate::canonical_state::validate_payload(&envelope.payload, project_id)?;
    let state = SemanticState {
        project_id: envelope.payload.project_id.clone(),
        tables: envelope.payload.tables.clone(),
    };
    let state_bytes = serde_json::to_vec(&state).map_err(json_error)?;
    let common_base = &envelope.payload.lineage.common_base_basis;
    if sha256_hex(&state_bytes) != envelope.payload.lineage.history_basis
        || common_base.len() != 64
        || !common_base.bytes().all(|value| value.is_ascii_hexdigit())
    {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "bundle lineage basis does not match canonical state",
        ));
    }
    Ok(ValidatedBundle {
        payload: envelope.payload,
        project_id,
        checksum,
    })
}

pub(crate) fn bundle_bytes(
    store: &Store,
    project_id: ProjectId,
) -> Result<(Vec<u8>, String, String), Error> {
    let exists: bool = store
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1)",
            [project_id.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .map_err(storage_read)?;
    if !exists {
        return Err(Error::new(ErrorKind::NotFound, "Project was not found"));
    }
    let mut tables = Vec::with_capacity(TABLES.len());
    for spec in TABLES {
        tables.push(export_table(&store.connection, spec, project_id)?);
    }
    let project_text = project_id.to_string();
    let state = SemanticState {
        project_id: project_text.clone(),
        tables: tables.clone(),
    };
    let state_bytes = serde_json::to_vec(&state).map_err(json_error)?;
    let history_basis = sha256_hex(&state_bytes);
    let common_base_basis = store
        .connection
        .query_row(
            "SELECT common_base_basis FROM bundle_lineage WHERE project_id = ?1",
            [project_id.as_bytes().as_slice()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_read)?
        .unwrap_or_else(|| history_basis.clone());
    let payload = Payload {
        project_id: project_text,
        lineage: Lineage {
            history_basis: history_basis.clone(),
            common_base_basis,
        },
        tables,
    };
    let payload_bytes = serde_json::to_vec(&payload).map_err(json_error)?;
    let checksum = sha256_hex(&payload_bytes);
    let envelope = Envelope {
        kind: BUNDLE_KIND.to_owned(),
        format_version: BUNDLE_FORMAT_VERSION,
        checksum: checksum.clone(),
        payload,
    };
    let mut bytes = serde_json::to_vec(&envelope).map_err(json_error)?;
    bytes.push(b'\n');
    validate_bundle(&bytes)?;
    Ok((bytes, checksum, history_basis))
}

pub(crate) fn export_table(
    connection: &rusqlite::Connection,
    spec: &TableSpec,
    project_id: ProjectId,
) -> Result<PortableTable, Error> {
    let columns = spec.columns.join(", ");
    let project_field = spec.columns[spec.project_column];
    let sql = format!(
        "SELECT {columns} FROM {} WHERE {project_field} = ?1 ORDER BY {}",
        spec.name, spec.order_by
    );
    let mut statement = connection.prepare(&sql).map_err(storage_read)?;
    let rows = statement
        .query_map([project_id.as_bytes().as_slice()], |row| {
            let mut values = Vec::with_capacity(spec.columns.len());
            for index in 0..spec.columns.len() {
                values.push(value_from_sql(row.get_ref(index)?));
            }
            Ok(values)
        })
        .map_err(storage_read)?;
    let mut values = Vec::new();
    for row in rows {
        let mut row = row.map_err(storage_read)?;
        if spec.name == "sources" && is_repository_bound_source(&row)? {
            row[14] = PortableValue::Text("unavailable".to_owned());
        }
        values.push(row);
    }
    Ok(PortableTable {
        name: spec.name.to_owned(),
        columns: spec
            .columns
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        rows: values,
    })
}

pub(crate) fn validate_portable_canonical_invariants(
    payload: &Payload,
    project_id: ProjectId,
) -> Result<(), Error> {
    if payload.tables.len() != TABLES.len() {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "bundle table inventory is incomplete",
        ));
    }
    let project_bytes = project_id.as_bytes();
    let mut active = BTreeSet::new();
    let mut tombstones = BTreeSet::new();
    for (spec, table) in TABLES.iter().zip(&payload.tables) {
        if table.name != spec.name
            || table.columns.iter().map(String::as_str).collect::<Vec<_>>() != spec.columns
        {
            return Err(Error::new(
                ErrorKind::CorruptState,
                format!("bundle table contract differs for {}", spec.name),
            ));
        }
        let mut keys = BTreeSet::new();
        for row in &table.rows {
            if row.len() != spec.columns.len() {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    format!("bundle {} row width is invalid", spec.name),
                ));
            }
            if value_bytes(&row[spec.project_column])? != project_bytes {
                return Err(Error::new(
                    ErrorKind::WrongProject,
                    format!("bundle {} row belongs to a different Project", spec.name),
                ));
            }
            let key = spec
                .primary_key
                .iter()
                .map(|index| value_key(&row[*index]))
                .collect::<Vec<_>>()
                .join("|");
            if !keys.insert(key) {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    format!("bundle {} contains duplicate identity", spec.name),
                ));
            }
        }
        if let Some((kind, id_column)) = active_kind(spec.name) {
            for row in &table.rows {
                active.insert((kind.to_owned(), value_key(&row[id_column])));
            }
        }
        if spec.name == "tombstones" {
            for row in &table.rows {
                tombstones.insert((value_text(&row[1])?.to_owned(), value_key(&row[2])));
            }
        }
    }
    let projects = &payload.tables[0].rows;
    if projects.len() != 1 || value_bytes(&projects[0][0])? != project_bytes {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "bundle must contain exactly its declared Project identity",
        ));
    }
    if active.iter().any(|record| tombstones.contains(record)) {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "bundle contains both active canonical content and a tombstone for one record",
        ));
    }
    if tombstones.iter().any(|(kind, _)| {
        !matches!(
            kind.as_str(),
            "source" | "question" | "decision" | "context_item" | "checkpoint"
        )
    }) {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "bundle tombstone kind is not a forgettable canonical record",
        ));
    }
    let present = |kind: &str, identity: &PortableValue| {
        let record = (kind.to_owned(), value_key(identity));
        active.contains(&record) || tombstones.contains(&record)
    };
    let active_record = |kind: &str, identity: &PortableValue| {
        active.contains(&(kind.to_owned(), value_key(identity)))
    };
    for table in &payload.tables {
        for row in &table.rows {
            let owner = match table.name.as_str() {
                "project_revisions" => Some(("project", 0)),
                "source_relations" => Some(("source", 1)),
                "question_revisions" => Some(("question", 0)),
                "question_response_sources" => Some(("question", 1)),
                "decision_revisions" => Some(("decision", 0)),
                "context_item_sources" => Some(("context_item", 1)),
                "context_item_revisions" => Some(("context_item", 0)),
                "checkpoint_source_relations"
                | "checkpoint_decisions"
                | "checkpoint_questions"
                | "checkpoint_verifications" => Some(("checkpoint", 1)),
                "review_due" => Some(("decision", 1)),
                _ => None,
            };
            if let Some((kind, index)) = owner {
                if !active_record(kind, &row[index]) {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        format!(
                            "bundle {} row exists without its active canonical owner",
                            table.name
                        ),
                    ));
                }
            }
            match table.name.as_str() {
                "question_response_sources" if !active_record("source", &row[3]) => {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "Question response references a missing active Source",
                    ));
                }
                "question_decision_history_witnesses" => {
                    if !present("question", &row[1])
                        || !present("decision", &row[3])
                        || !present("source", &row[5])
                    {
                        return Err(Error::new(
                            ErrorKind::CorruptState,
                            "Question Decision history witness references no matching active or tombstoned identity",
                        ));
                    }
                }
                "decisions" => {
                    if !present("question", &row[3]) || !present("source", &row[5]) {
                        return Err(Error::new(
                            ErrorKind::CorruptState,
                            "Decision references neither active nor tombstoned canonical identity",
                        ));
                    }
                }
                "context_item_sources" if !active_record("source", &row[2]) => {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "Context Item link references a missing active Source",
                    ));
                }
                "checkpoint_source_relations" if !active_record("source", &row[3]) => {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "Checkpoint link references a missing active Source",
                    ));
                }
                "checkpoint_decisions" if !active_record("decision", &row[2]) => {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "Checkpoint link references a missing active Decision",
                    ));
                }
                "checkpoint_questions" if !active_record("question", &row[2]) => {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "Checkpoint link references a missing active Question",
                    ));
                }
                "checkpoint_verifications"
                    if !matches!(row[4], PortableValue::Null)
                        && !active_record("source", &row[4]) =>
                {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "Checkpoint verification references a missing active Source",
                    ));
                }
                "merge_events"
                    if !matches!(row[9], PortableValue::Null) && !present("source", &row[9]) =>
                {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "merge provenance references neither active nor tombstoned Source identity",
                    ));
                }
                _ => {}
            }
        }
    }
    let relation_table = payload
        .tables
        .iter()
        .find(|table| table.name == "canonical_relations")
        .ok_or_else(|| Error::new(ErrorKind::CorruptState, "bundle relation table is missing"))?;
    for row in &relation_table.rows {
        for (kind_index, id_index) in [(1, 2), (4, 5)] {
            let target = (
                value_text(&row[kind_index])?.to_owned(),
                value_key(&row[id_index]),
            );
            if !active.contains(&target) && !tombstones.contains(&target) {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "canonical relation references neither an active record nor a tombstone",
                ));
            }
        }
    }
    let source_relations = payload
        .tables
        .iter()
        .find(|table| table.name == "source_relations")
        .ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "bundle Source relation table is missing",
            )
        })?;
    for row in &source_relations.rows {
        for index in [1, 3] {
            let target = ("source".to_owned(), value_key(&row[index]));
            if !active.contains(&target) {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "Source relation references a missing Source",
                ));
            }
        }
    }
    validate_decision_semantics(payload, &tombstones)?;
    Ok(())
}

fn required_table<'a>(payload: &'a Payload, name: &str) -> Result<&'a PortableTable, Error> {
    payload
        .tables
        .iter()
        .find(|table| table.name == name)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                format!("bundle {name} table is missing"),
            )
        })
}

fn value_integer(value: &PortableValue) -> Result<i64, Error> {
    match value {
        PortableValue::Integer(value) => Ok(*value),
        _ => Err(Error::new(
            ErrorKind::CorruptState,
            "bundle value is not an integer",
        )),
    }
}

fn optional_text(value: &PortableValue) -> Result<Option<&str>, Error> {
    match value {
        PortableValue::Null => Ok(None),
        PortableValue::Text(value) => Ok(Some(value)),
        _ => Err(Error::new(
            ErrorKind::CorruptState,
            "bundle optional value is neither null nor text",
        )),
    }
}

fn validate_decision_semantics(
    payload: &Payload,
    tombstones: &BTreeSet<(String, String)>,
) -> Result<(), Error> {
    let sources = required_table(payload, "sources")?
        .rows
        .iter()
        .map(|row| (value_key(&row[0]), row))
        .collect::<BTreeMap<_, _>>();
    let questions = required_table(payload, "questions")?
        .rows
        .iter()
        .map(|row| (value_key(&row[0]), row))
        .collect::<BTreeMap<_, _>>();
    let question_revisions = required_table(payload, "question_revisions")?
        .rows
        .iter()
        .map(|row| Ok(((value_key(&row[0]), value_integer(&row[1])?), row)))
        .collect::<Result<BTreeMap<_, _>, Error>>()?;
    let responses = required_table(payload, "question_response_sources")?
        .rows
        .iter()
        .map(|row| {
            Ok((
                (value_key(&row[1]), value_integer(&row[2])?),
                value_key(&row[3]),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, Error>>()?;
    let witnesses = required_table(payload, "question_decision_history_witnesses")?
        .rows
        .iter()
        .map(|row| Ok(((value_key(&row[1]), value_integer(&row[2])?), row)))
        .collect::<Result<BTreeMap<_, _>, Error>>()?;
    let decisions = required_table(payload, "decisions")?
        .rows
        .iter()
        .map(|row| (value_key(&row[0]), row))
        .collect::<BTreeMap<_, _>>();
    let mut revisions = BTreeMap::<String, BTreeMap<i64, &Vec<PortableValue>>>::new();
    for row in &required_table(payload, "decision_revisions")?.rows {
        revisions
            .entry(value_key(&row[0]))
            .or_default()
            .insert(value_integer(&row[1])?, row);
    }
    let review_due = required_table(payload, "review_due")?
        .rows
        .iter()
        .map(|row| value_key(&row[1]))
        .collect::<BTreeSet<_>>();

    let mut supersedes = BTreeMap::<String, String>::new();
    let mut superseded_by = BTreeMap::<String, String>::new();
    for row in &required_table(payload, "canonical_relations")?.rows {
        if value_text(&row[1])? != "decision"
            || value_text(&row[3])? != "supersedes"
            || value_text(&row[4])? != "decision"
        {
            continue;
        }
        let newer = value_key(&row[2]);
        let older = value_key(&row[5]);
        if newer == older
            || supersedes.insert(newer.clone(), older.clone()).is_some()
            || superseded_by.insert(older, newer).is_some()
        {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "Decision supersession must be directed and non-branching",
            ));
        }
    }
    validate_acyclic_supersession(&supersedes)?;

    for (decision_id, row) in &decisions {
        validate_decision_authority(row, &sources, tombstones)?;
        validate_decision_revision_history(
            decision_id,
            row,
            revisions.get(decision_id),
            &sources,
            tombstones,
        )?;
        validate_decision_question_basis(
            row,
            &questions,
            &question_revisions,
            &review_due,
            tombstones,
            "Decision",
        )?;
        if let Some(history) = revisions.get(decision_id) {
            for revision in history.values() {
                validate_decision_question_basis(
                    revision,
                    &questions,
                    &question_revisions,
                    &review_due,
                    tombstones,
                    "Decision revision",
                )?;
            }
        }
    }

    for (newer, older) in &supersedes {
        if let (Some(newer), Some(older)) = (decisions.get(newer), decisions.get(older)) {
            if newer[3] != older[3] || newer[4] != older[4] {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "one Decision supersession lineage changes Question identity or revision",
                ));
            }
        }
    }

    validate_question_response_roles(
        &sources,
        &questions,
        &question_revisions,
        &responses,
        &witnesses,
        &decisions,
        &supersedes,
        tombstones,
    )
}

fn validate_acyclic_supersession(supersedes: &BTreeMap<String, String>) -> Result<(), Error> {
    for start in supersedes.keys() {
        let mut visited = BTreeSet::new();
        let mut current = start;
        while let Some(next) = supersedes.get(current) {
            if !visited.insert(current.clone()) {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "Decision supersession contains a cycle",
                ));
            }
            current = next;
        }
    }
    Ok(())
}

fn validate_decision_authority(
    row: &[PortableValue],
    sources: &BTreeMap<String, &Vec<PortableValue>>,
    tombstones: &BTreeSet<(String, String)>,
) -> Result<(), Error> {
    if value_text(&row[6])? != CURRENT_HOST_USER_AUTHORITY {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "Decision lacks its current-host user authority witness",
        ));
    }
    validate_authority_source(&row[5], sources, tombstones)
}

fn validate_authority_source(
    source_id: &PortableValue,
    sources: &BTreeMap<String, &Vec<PortableValue>>,
    tombstones: &BTreeSet<(String, String)>,
) -> Result<(), Error> {
    let identity = value_key(source_id);
    if let Some(source) = sources.get(&identity) {
        if !is_current_host_user_authority(value_text(&source[3])?, value_text(&source[10])?) {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "Decision authority Source is not a user-authored current-host turn",
            ));
        }
        return Ok(());
    }
    if tombstones.contains(&("source".to_owned(), identity)) {
        return Ok(());
    }
    Err(Error::new(
        ErrorKind::CorruptState,
        "Decision authority references no active or forgotten Source",
    ))
}

fn validate_decision_revision_history(
    decision_id: &str,
    active: &[PortableValue],
    revisions: Option<&BTreeMap<i64, &Vec<PortableValue>>>,
    sources: &BTreeMap<String, &Vec<PortableValue>>,
    tombstones: &BTreeSet<(String, String)>,
) -> Result<(), Error> {
    let current = value_integer(&active[2])?;
    let revisions = revisions.ok_or_else(|| {
        Error::new(
            ErrorKind::CorruptState,
            "active Decision has no revision history",
        )
    })?;
    if current < 1 || revisions.len() != usize::try_from(current).unwrap_or(usize::MAX) {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "Decision revision history contains a gap",
        ));
    }
    for expected in 1..=current {
        if !revisions.contains_key(&expected) {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "Decision revision history contains a gap",
            ));
        }
    }
    let current_row = revisions.get(&current).ok_or_else(|| {
        Error::new(
            ErrorKind::CorruptState,
            "Decision current revision snapshot is missing",
        )
    })?;
    if active[3..=18] != current_row[3..=18] {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "Decision current row differs from its current revision snapshot",
        ));
    }

    let mut previous: Option<&&Vec<PortableValue>> = None;
    for revision_number in 1..=current {
        let row = revisions.get(&revision_number).ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "Decision revision snapshot is missing",
            )
        })?;
        if value_key(&row[0]) != decision_id {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "Decision revision identity differs from its active Decision",
            ));
        }
        validate_decision_authority(row, sources, tombstones)?;
        if revision_number == 1 {
            if !matches!(row[19], PortableValue::Null)
                || !matches!(row[20], PortableValue::Null)
                || !matches!(row[21], PortableValue::Null)
            {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "initial Decision revision carries correction authorization",
                ));
            }
        } else {
            let kind = value_text(&row[19])?;
            let authorization = match &row[20] {
                PortableValue::Bytes(_) => &row[20],
                _ => {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "Decision correction lacks an authorization Source",
                    ))
                }
            };
            if value_text(&row[21])? != CURRENT_HOST_USER_AUTHORITY {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "Decision correction lacks its user authority witness",
                ));
            }
            validate_authority_source(authorization, sources, tombstones)?;
            let prior = previous.ok_or_else(|| {
                Error::new(
                    ErrorKind::CorruptState,
                    "Decision correction basis is missing",
                )
            })?;
            for index in (3..=8).chain(10..=18) {
                if prior[index] != row[index] {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "Decision correction changes a semantic field",
                    ));
                }
            }
            let before = optional_text(&prior[9])?;
            let after = optional_text(&row[9])?;
            if !matches!((before, after), (Some(before), Some(after)) if meaning_preserving_correction(before, after, kind))
            {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "Decision correction is not a permitted presentation-only change",
                ));
            }
        }
        previous = Some(row);
    }
    Ok(())
}

fn validate_decision_question_basis(
    row: &[PortableValue],
    questions: &BTreeMap<String, &Vec<PortableValue>>,
    question_revisions: &BTreeMap<(String, i64), &Vec<PortableValue>>,
    review_due: &BTreeSet<String>,
    tombstones: &BTreeSet<(String, String)>,
    owner: &str,
) -> Result<(), Error> {
    let question_id = value_key(&row[3]);
    let question_revision = value_integer(&row[4])?;
    if questions.contains_key(&question_id) {
        let revision = question_revisions
            .get(&(question_id, question_revision))
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::CorruptState,
                    format!("{owner} references a nonexistent Question revision"),
                )
            })?;
        let displayed = decode_alternatives(&value_bytes(&row[10])?)?;
        let alternatives = decode_alternatives(&value_bytes(&revision[6])?)?;
        let displayed_sources = decode_source_ids(&value_bytes(&row[13])?)?;
        let recommendation_sources = decode_source_ids(&value_bytes(&revision[9])?)?;
        if displayed != alternatives
            || optional_text(&row[11])? != optional_text(&revision[7])?
            || value_text(&row[12])? != value_text(&revision[8])?
            || displayed_sources != recommendation_sources
        {
            return Err(Error::new(
                ErrorKind::CorruptState,
                format!("{owner} presentation differs from its exact Question revision"),
            ));
        }
        if !decision_choice_is_valid(value_text(&row[7])?, value_text(&row[8])?, &alternatives) {
            return Err(Error::new(
                ErrorKind::CorruptState,
                format!("{owner} choice is invalid for its Question revision"),
            ));
        }
    } else if tombstones.contains(&("question".to_owned(), question_id)) {
        validate_sanitized_question_presentation(row, owner)?;
        if owner == "Decision" && !review_due.contains(&value_key(&row[0])) {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "active Decision referencing a forgotten Question is missing review_due",
            ));
        }
        if !matches!(value_text(&row[7])?, "alternative" | "delegation")
            || value_text(&row[8])?.is_empty()
        {
            return Err(Error::new(
                ErrorKind::CorruptState,
                format!("{owner} retains an invalid independently owned choice"),
            ));
        }
    } else {
        return Err(Error::new(
            ErrorKind::CorruptState,
            format!("{owner} references no active or forgotten Question"),
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_question_response_roles(
    sources: &BTreeMap<String, &Vec<PortableValue>>,
    questions: &BTreeMap<String, &Vec<PortableValue>>,
    question_revisions: &BTreeMap<(String, i64), &Vec<PortableValue>>,
    responses: &BTreeMap<(String, i64), String>,
    witnesses: &BTreeMap<(String, i64), &Vec<PortableValue>>,
    decisions: &BTreeMap<String, &Vec<PortableValue>>,
    supersedes: &BTreeMap<String, String>,
    tombstones: &BTreeSet<(String, String)>,
) -> Result<(), Error> {
    let mut decisions_by_question = BTreeMap::<(String, i64), Vec<&Vec<PortableValue>>>::new();
    for row in decisions.values() {
        let key = (value_key(&row[3]), value_integer(&row[4])?);
        decisions_by_question.entry(key).or_default().push(row);
    }

    for ((question_id, revision), source_id) in responses {
        let question = questions.get(question_id).ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "Question response link has no active Question",
            )
        })?;
        if value_integer(&question[2])? != *revision
            || !question_revisions.contains_key(&(question_id.clone(), *revision))
            || matches!(question[3], PortableValue::Null)
        {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "Question response link does not name the active terminal revision",
            ));
        }
        let source = sources.get(source_id).ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "Question response link has no active Source",
            )
        })?;
        if !is_current_host_user_authority(value_text(&source[3])?, value_text(&source[10])?) {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "Question response link is not a user-authored current-host turn",
            ));
        }
    }

    validate_question_decision_history_witnesses(
        sources,
        questions,
        question_revisions,
        witnesses,
        decisions,
        &decisions_by_question,
        supersedes,
        tombstones,
    )?;

    for (question_id, question) in questions {
        let revision = value_integer(&question[2])?;
        let key = (question_id.clone(), revision);
        let response = responses.get(&key);
        let witness = witnesses.get(&key).copied();
        let linked_decisions = decisions_by_question
            .get(&key)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let outcome = optional_text(&question[3])?;
        match outcome {
            None => {
                if response.is_some() || witness.is_some() || !linked_decisions.is_empty() {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "open Question carries terminal response history",
                    ));
                }
            }
            Some(outcome @ ("answered" | "delegated")) => {
                let witness = witness.ok_or_else(|| {
                    Error::new(
                        ErrorKind::CorruptState,
                        "answered or delegated Question has no Question-specific Decision history witness",
                    )
                })?;
                if value_text(&witness[4])? != outcome {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "Question Decision history witness conflicts with the terminal outcome",
                    ));
                }
                let witness_source = value_key(&witness[5]);
                if sources.contains_key(&witness_source) {
                    if response != Some(&witness_source) {
                        return Err(Error::new(
                            ErrorKind::CorruptState,
                            "Question response link does not match its Decision history witness",
                        ));
                    }
                } else if response.is_some() {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "forgotten response Source still has an active Question response link",
                    ));
                }
            }
            Some(_) => {
                if witness.is_some() || !linked_decisions.is_empty() {
                    return Err(Error::new(
                        ErrorKind::CorruptState,
                        "non-Decision terminal outcome carries Decision response history",
                    ));
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_question_decision_history_witnesses(
    sources: &BTreeMap<String, &Vec<PortableValue>>,
    questions: &BTreeMap<String, &Vec<PortableValue>>,
    question_revisions: &BTreeMap<(String, i64), &Vec<PortableValue>>,
    witnesses: &BTreeMap<(String, i64), &Vec<PortableValue>>,
    decisions: &BTreeMap<String, &Vec<PortableValue>>,
    decisions_by_question: &BTreeMap<(String, i64), Vec<&Vec<PortableValue>>>,
    supersedes: &BTreeMap<String, String>,
    tombstones: &BTreeSet<(String, String)>,
) -> Result<(), Error> {
    let mut witnessed_roots = BTreeSet::new();
    for (key, witness) in witnesses {
        if let Some(question) = questions.get(&key.0) {
            if value_integer(&question[2])? != key.1
                || !question_revisions.contains_key(key)
                || !matches!(optional_text(&question[3])?, Some("answered" | "delegated"))
            {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "Question Decision history witness does not name an active Decision-terminal Question revision",
                ));
            }
        }
        let root_id = value_key(&witness[3]);
        if !witnessed_roots.insert(root_id.clone()) {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "one root Decision has duplicate Question history witnesses",
            ));
        }
        let outcome = value_text(&witness[4])?;
        if !matches!(outcome, "answered" | "delegated")
            || value_text(&witness[6])? != CURRENT_HOST_USER_AUTHORITY
            || value_text(&witness[7])? != "explicit_question_response"
        {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "Question Decision history witness has invalid content-free provenance",
            ));
        }
        validate_authority_source(&witness[5], sources, tombstones)?;
        if supersedes.contains_key(&root_id) {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "Question Decision history witness does not identify the original root Decision",
            ));
        }

        if let Some(root) = decisions.get(&root_id) {
            if value_key(&root[3]) != key.0
                || value_integer(&root[4])? != key.1
                || value_key(&root[5]) != value_key(&witness[5])
                || decision_outcome(value_text(&root[7])?) != Some(outcome)
            {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "active root Decision does not match its Question history witness",
                ));
            }
        } else if !tombstones.contains(&("decision".to_owned(), root_id.clone())) {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "Question Decision history witness has no exact active or forgotten root Decision",
            ));
        }

        for decision in decisions_by_question
            .get(key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            if !decision_lineage_reaches_root(&value_key(&decision[0]), &root_id, supersedes) {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "Question Decision lineage is not rooted in its response-history witness",
                ));
            }
        }
    }
    Ok(())
}

fn decision_lineage_reaches_root(
    decision_id: &str,
    root_id: &str,
    supersedes: &BTreeMap<String, String>,
) -> bool {
    let mut current = decision_id;
    loop {
        if current == root_id {
            return true;
        }
        let Some(previous) = supersedes.get(current) else {
            return false;
        };
        current = previous;
    }
}

fn validate_sanitized_question_presentation(
    row: &[PortableValue],
    owner: &str,
) -> Result<(), Error> {
    let alternatives = decode_alternatives(&value_bytes(&row[10])?)?;
    let recommendation_sources = decode_source_ids(&value_bytes(&row[13])?)?;
    let recommendation_key_is_absent = matches!(row[11], PortableValue::Null);
    let recommendation_rationale_is_empty = value_text(&row[12])?.is_empty();
    if !alternatives.is_empty()
        || !recommendation_key_is_absent
        || !recommendation_rationale_is_empty
        || !recommendation_sources.is_empty()
    {
        return Err(Error::new(
            ErrorKind::CorruptState,
            format!("{owner} retains presentation owned by a forgotten Question"),
        ));
    }
    Ok(())
}

fn is_repository_bound_source(row: &[PortableValue]) -> Result<bool, Error> {
    let kind = value_text(&row[3])?;
    Ok(matches!(
        kind,
        "repository_snapshot" | "repository_commit" | "file" | "symbol" | "adopted_artifact"
    ))
}

fn active_kind(table: &str) -> Option<(&'static str, usize)> {
    match table {
        "projects" => Some(("project", 0)),
        "sources" => Some(("source", 0)),
        "questions" => Some(("question", 0)),
        "decisions" => Some(("decision", 0)),
        "context_items" => Some(("context_item", 0)),
        "checkpoints" => Some(("checkpoint", 0)),
        _ => None,
    }
}

enum RowState {
    Absent,
    Equal,
    Different,
}

fn compare_row(
    transaction: &rusqlite::Transaction<'_>,
    spec: &TableSpec,
    row: &[PortableValue],
) -> Result<RowState, Error> {
    let where_clause = spec
        .primary_key
        .iter()
        .enumerate()
        .map(|(position, index)| format!("{} = ?{}", spec.columns[*index], position + 1))
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = format!(
        "SELECT {} FROM {} WHERE {where_clause}",
        spec.columns.join(", "),
        spec.name
    );
    let key_values: Vec<Value> = spec
        .primary_key
        .iter()
        .map(|index| value_to_sql(&row[*index]))
        .collect::<Result<_, _>>()?;
    let existing: Option<Vec<PortableValue>> = transaction
        .query_row(&sql, params_from_iter(key_values), |sql_row| {
            let mut values = Vec::with_capacity(spec.columns.len());
            for index in 0..spec.columns.len() {
                values.push(value_from_sql(sql_row.get_ref(index)?));
            }
            Ok(values)
        })
        .optional()
        .map_err(storage_read)?;
    Ok(match existing {
        None => RowState::Absent,
        Some(existing) if existing == row => RowState::Equal,
        Some(_) => RowState::Different,
    })
}

pub(crate) fn insert_row(
    transaction: &rusqlite::Transaction<'_>,
    spec: &TableSpec,
    row: &[PortableValue],
) -> Result<(), Error> {
    let placeholders = (1..=spec.columns.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO {}({}) VALUES ({placeholders})",
        spec.name,
        spec.columns.join(", ")
    );
    let values: Vec<Value> = row.iter().map(value_to_sql).collect::<Result<_, _>>()?;
    transaction
        .execute(&sql, params_from_iter(values))
        .map_err(storage_write)?;
    Ok(())
}

fn value_from_sql(value: ValueRef<'_>) -> PortableValue {
    match value {
        ValueRef::Null => PortableValue::Null,
        ValueRef::Integer(value) => PortableValue::Integer(value),
        ValueRef::Real(value) => PortableValue::Text(value.to_string()),
        ValueRef::Text(value) => PortableValue::Text(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => PortableValue::Bytes(hex_encode(value)),
    }
}

pub(crate) fn value_to_sql(value: &PortableValue) -> Result<Value, Error> {
    Ok(match value {
        PortableValue::Null => Value::Null,
        PortableValue::Integer(value) => Value::Integer(*value),
        PortableValue::Text(value) => Value::Text(value.clone()),
        PortableValue::Bytes(value) => Value::Blob(hex_decode(value)?),
    })
}

pub(crate) fn value_bytes(value: &PortableValue) -> Result<Vec<u8>, Error> {
    match value {
        PortableValue::Bytes(value) => hex_decode(value),
        _ => Err(Error::new(
            ErrorKind::CorruptState,
            "bundle identity is not bytes",
        )),
    }
}

pub(crate) fn value_text(value: &PortableValue) -> Result<&str, Error> {
    match value {
        PortableValue::Text(value) => Ok(value),
        _ => Err(Error::new(
            ErrorKind::CorruptState,
            "bundle value is not text",
        )),
    }
}

pub(crate) fn value_key(value: &PortableValue) -> String {
    match value {
        PortableValue::Null => "n".to_owned(),
        PortableValue::Integer(value) => format!("i:{value}"),
        PortableValue::Text(value) => format!("t:{value}"),
        PortableValue::Bytes(value) => format!("b:{value}"),
    }
}

fn validate_final_path(path: &Path) -> Result<PathBuf, Error> {
    if path.as_os_str().is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "bundle path must be explicitly supplied",
        ));
    }
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".volicord-context.tmp"))
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "temporary bundle candidates are not authoritative import/export paths",
        ));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                Error::with_source(
                    ErrorKind::StorageUnavailable,
                    "cannot resolve bundle path",
                    error,
                )
            })?
            .join(path)
    };
    Ok(absolute)
}

fn publish_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let parent = path.parent().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "bundle path has no containing directory",
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "bundle file name must be UTF-8"))?;
    let temporary = parent.join(format!(".{file_name}.volicord-context.tmp"));
    if temporary.try_exists().map_err(|error| {
        Error::with_source(
            ErrorKind::StorageUnavailable,
            "cannot inspect bundle temporary candidate",
            error,
        )
    })? {
        if temporary.is_file() {
            fs::remove_file(&temporary).map_err(|error| {
                Error::with_source(
                    ErrorKind::StorageUnavailable,
                    "cannot clean orphan bundle temporary candidate",
                    error,
                )
            })?;
        } else {
            return Err(Error::new(
                ErrorKind::StorageUnavailable,
                "bundle temporary candidate is not a removable file",
            ));
        }
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| {
            Error::with_source(
                ErrorKind::StorageUnavailable,
                format!(
                    "cannot create bundle temporary file {}",
                    temporary.display()
                ),
                error,
            )
        })?;
    file.write_all(bytes).map_err(|error| {
        Error::with_source(
            ErrorKind::StorageUnavailable,
            "cannot write bundle temporary file",
            error,
        )
    })?;
    file.sync_all().map_err(|error| {
        Error::with_source(
            ErrorKind::StorageUnavailable,
            "cannot sync bundle temporary file",
            error,
        )
    })?;
    drop(file);
    fs::rename(&temporary, path).map_err(|error| {
        Error::with_source(
            ErrorKind::StorageUnavailable,
            format!("cannot atomically publish bundle {}", path.display()),
            error,
        )
    })?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            Error::with_source(
                ErrorKind::StorageUnavailable,
                format!("cannot sync bundle directory {}", parent.display()),
                error,
            )
        })?;
    Ok(())
}

fn path_text(path: &Path) -> Result<&str, Error> {
    path.to_str()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "managed bundle path must be UTF-8"))
}

fn parse_project_id(value: &str) -> Result<ProjectId, Error> {
    let bytes = hex_decode(value)?;
    ProjectId::from_slice(&bytes).map_err(|_| {
        Error::new(
            ErrorKind::CorruptState,
            "bundle Project identity is invalid",
        )
    })
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn hex_decode(value: &str) -> Result<Vec<u8>, Error> {
    if value.len() % 2 != 0 {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "bundle hexadecimal value has odd length",
        ));
    }
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn hex_digit(value: u8) -> Result<u8, Error> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(Error::new(
            ErrorKind::CorruptState,
            "bundle hexadecimal value is malformed",
        )),
    }
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    hex_encode(&sha256(bytes))
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state = [
        0x6a09e667_u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in padded.chunks_exact(64) {
        let mut words = [0_u32; 64];
        for (index, word) in chunk.chunks_exact(4).enumerate() {
            words[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (target, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *target = target.wrapping_add(value);
        }
    }
    let mut output = [0_u8; 32];
    for (chunk, value) in output.chunks_exact_mut(4).zip(state) {
        chunk.copy_from_slice(&value.to_be_bytes());
    }
    output
}

fn json_error(error: serde_json::Error) -> Error {
    Error::with_source(
        ErrorKind::CorruptState,
        "cannot serialize canonical bundle JSON",
        error,
    )
}
fn storage_read(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::CorruptState,
        "cannot read canonical state for portable bundle",
        error,
    )
}
fn storage_write(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::TransactionFailure,
        "portable bundle transaction failed",
        error,
    )
}
fn storage_commit(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::IndeterminateOutcome,
        "portable bundle commit outcome is indeterminate",
        error,
    )
}

#[cfg(test)]
mod tests {
    use super::{sha256_hex, BUNDLE_FORMAT_VERSION, BUNDLE_KIND};

    #[test]
    fn checksum_matches_standard_sha256_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(BUNDLE_KIND, "volicord-context-bundle");
        assert_eq!(BUNDLE_FORMAT_VERSION, 4);
    }
}
