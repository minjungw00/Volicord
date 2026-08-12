use crate::identity::{IdGenerator, SystemIdGenerator};
use crate::model::{
    Availability, CommandOutcome, CommandTermination, LocalBinding, OperationResult, Principal,
    PrincipalKind, Project, Source, SourceDraft, SourcePayload, SourceRelation, SourceRelationKind,
};
use crate::time::{Clock, SystemClock, TimestampMicros};
use crate::{Error, ErrorKind, LocalBindingId, OperationId, ProjectId, SourceId};
use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const SCHEMA_KIND: &str = "volicord-context";
pub const SCHEMA_VERSION: u32 = 1;

const REQUIRED_TABLES: [&str; 8] = [
    "metadata",
    "projects",
    "project_revisions",
    "local_bindings",
    "local_binding_revisions",
    "sources",
    "source_relations",
    "operations",
];

/// One synchronous connection to an explicit Canonical Context store path.
///
/// Mutation methods require `&mut self`, and each begins an immediate SQLite
/// transaction. SQLite therefore serializes writers both within this handle
/// and across handles without an implicit retry under a new operation ID.
pub struct Store {
    connection: Connection,
    ids: Box<dyn IdGenerator>,
    clock: Box<dyn Clock>,
    path: PathBuf,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::open_with(path, SystemIdGenerator, SystemClock)
    }

    pub fn open_with(
        path: impl AsRef<Path>,
        ids: impl IdGenerator + 'static,
        clock: impl Clock + 'static,
    ) -> Result<Self, Error> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "store path must be explicitly supplied",
            ));
        }

        let existed = path.try_exists().map_err(|error| {
            Error::with_source(
                ErrorKind::StorageUnavailable,
                format!("cannot inspect store path {}", path.display()),
                error,
            )
        })?;
        let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
        if !existed {
            flags |= OpenFlags::SQLITE_OPEN_CREATE;
        }

        let connection = Connection::open_with_flags(path, flags).map_err(|error| {
            map_open_error(error, format!("cannot open store {}", path.display()))
        })?;
        connection.busy_timeout(Duration::ZERO).map_err(|error| {
            Error::with_source(
                ErrorKind::StorageUnavailable,
                "cannot configure canonical writer timeout",
                error,
            )
        })?;

        if existed {
            validate_existing_schema(&connection)?;
            configure_and_verify_durability(&connection)?;
        } else {
            configure_and_verify_durability(&connection)?;
            initialize_schema(&connection)?;
        }

        Ok(Self {
            connection,
            ids: Box::new(ids),
            clock: Box::new(clock),
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn create_project(
        &mut self,
        operation_id: OperationId,
        display_name: impl Into<String>,
    ) -> Result<OperationResult<Project>, Error> {
        let display_name = display_name.into();
        validate_nonempty("project display name", &display_name)?;
        let basis = Basis::new("create_project").string(&display_name).finish();
        let (connection, ids, clock) = (&mut self.connection, &mut self.ids, &mut self.clock);
        let transaction = begin_write(connection)?;

        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "create_project", &basis)?;
            let project = load_project_revision(
                &transaction,
                ProjectId::from_slice(&operation.result_id)?,
                operation.result_revision,
            )?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value: project,
                replayed: true,
            });
        }

        let project_id = ProjectId::from_bytes(ids.next_id()?);
        let now = clock.now()?;
        let inserted = transaction
            .execute(
                "INSERT INTO projects(id, display_name, revision, created_at, updated_at)
                 VALUES (?1, ?2, 1, ?3, ?3)",
                params![
                    project_id.as_bytes().as_slice(),
                    display_name,
                    now.as_unix_micros()
                ],
            )
            .map_err(|error| insert_identity_error(error, "Project identity already exists"))?;
        if inserted != 1 {
            return Err(Error::new(
                ErrorKind::TransactionFailure,
                "Project insertion affected an unexpected row count",
            ));
        }
        transaction
            .execute(
                "INSERT INTO project_revisions(project_id, revision, display_name, recorded_at)
                 VALUES (?1, 1, ?2, ?3)",
                params![
                    project_id.as_bytes().as_slice(),
                    display_name,
                    now.as_unix_micros()
                ],
            )
            .map_err(write_error)?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "create_project",
            &basis,
            "project",
            project_id.as_bytes(),
            1,
            now,
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: Project {
                id: project_id,
                display_name,
                revision: 1,
                created_at: now,
                updated_at: now,
            },
            replayed: false,
        })
    }

    pub fn get_project(&self, project_id: ProjectId) -> Result<Project, Error> {
        load_project(&self.connection, project_id)
    }

    pub fn rename_project(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        expected_revision: u64,
        display_name: impl Into<String>,
    ) -> Result<OperationResult<Project>, Error> {
        let display_name = display_name.into();
        validate_nonempty("project display name", &display_name)?;
        let basis = Basis::new("rename_project")
            .bytes(project_id.as_bytes())
            .number(expected_revision)
            .string(&display_name)
            .finish();
        let (connection, clock) = (&mut self.connection, &mut self.clock);
        let transaction = begin_write(connection)?;

        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "rename_project", &basis)?;
            let project =
                load_project_revision(&transaction, project_id, operation.result_revision)?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value: project,
                replayed: true,
            });
        }

        let current = load_project(&transaction, project_id)?;
        ensure_revision(expected_revision, current.revision, "Project")?;
        let revision = current.revision.checked_add(1).ok_or_else(|| {
            Error::new(ErrorKind::RepairRequired, "Project revision is exhausted")
        })?;
        let now = clock.now()?;
        transaction
            .execute(
                "UPDATE projects SET display_name = ?2, revision = ?3, updated_at = ?4
                 WHERE id = ?1 AND revision = ?5",
                params![
                    project_id.as_bytes().as_slice(),
                    display_name,
                    revision_i64(revision)?,
                    now.as_unix_micros(),
                    revision_i64(expected_revision)?,
                ],
            )
            .map_err(write_error)
            .and_then(|count| ensure_single_updated(count, "Project changed concurrently"))?;
        transaction
            .execute(
                "INSERT INTO project_revisions(project_id, revision, display_name, recorded_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    project_id.as_bytes().as_slice(),
                    revision_i64(revision)?,
                    display_name,
                    now.as_unix_micros(),
                ],
            )
            .map_err(write_error)?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "rename_project",
            &basis,
            "project",
            project_id.as_bytes(),
            revision,
            now,
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: Project {
                id: project_id,
                display_name,
                revision,
                created_at: current.created_at,
                updated_at: now,
            },
            replayed: false,
        })
    }

    pub fn bind_clone(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        expected_binding_revision: Option<u64>,
        absolute_path: impl Into<PathBuf>,
        availability: Availability,
    ) -> Result<OperationResult<LocalBinding>, Error> {
        let absolute_path = absolute_path.into();
        if !absolute_path.is_absolute() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "local clone binding path must be absolute",
            ));
        }
        let path_text = absolute_path.to_str().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "local clone binding path must be valid UTF-8",
            )
        })?;
        let basis = Basis::new("bind_clone")
            .bytes(project_id.as_bytes())
            .optional_number(expected_binding_revision)
            .string(path_text)
            .string(availability.as_str())
            .finish();
        let (connection, ids, clock) = (&mut self.connection, &mut self.ids, &mut self.clock);
        let transaction = begin_write(connection)?;

        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "bind_clone", &basis)?;
            let binding = load_binding_revision(
                &transaction,
                LocalBindingId::from_slice(&operation.result_id)?,
                operation.result_revision,
            )?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value: binding,
                replayed: true,
            });
        }

        load_project(&transaction, project_id)?;
        if let Some(owner) = binding_path_owner(&transaction, path_text)? {
            if owner != project_id {
                return Err(Error::new(
                    ErrorKind::WrongProject,
                    "local clone path is already bound to a different Project",
                ));
            }
        }

        let existing = load_binding_optional(&transaction, project_id)?;
        let now = clock.now()?;
        let (binding_id, revision) = match (existing, expected_binding_revision) {
            (None, None) => {
                let binding_id = LocalBindingId::from_bytes(ids.next_id()?);
                transaction
                    .execute(
                        "INSERT INTO local_bindings(
                             id, project_id, absolute_path, availability, revision, bound_at
                         ) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                        params![
                            binding_id.as_bytes().as_slice(),
                            project_id.as_bytes().as_slice(),
                            path_text,
                            availability.as_str(),
                            now.as_unix_micros(),
                        ],
                    )
                    .map_err(write_error)?;
                (binding_id, 1)
            }
            (Some(_), None) => {
                return Err(Error::new(
                    ErrorKind::AlreadyExists,
                    "Project already has a local clone binding; use its revision to rebind",
                ));
            }
            (None, Some(_)) => {
                return Err(Error::new(
                    ErrorKind::StaleBasis,
                    "local clone binding does not exist at the supplied revision",
                ));
            }
            (Some(current), Some(expected)) => {
                ensure_revision(expected, current.revision, "local clone binding")?;
                let revision = current.revision.checked_add(1).ok_or_else(|| {
                    Error::new(
                        ErrorKind::RepairRequired,
                        "local clone binding revision is exhausted",
                    )
                })?;
                transaction
                    .execute(
                        "UPDATE local_bindings
                         SET absolute_path = ?2, availability = ?3, revision = ?4, bound_at = ?5
                         WHERE id = ?1 AND revision = ?6",
                        params![
                            current.id.as_bytes().as_slice(),
                            path_text,
                            availability.as_str(),
                            revision_i64(revision)?,
                            now.as_unix_micros(),
                            revision_i64(expected)?,
                        ],
                    )
                    .map_err(write_error)
                    .and_then(|count| {
                        ensure_single_updated(count, "local clone binding changed concurrently")
                    })?;
                (current.id, revision)
            }
        };
        transaction
            .execute(
                "INSERT INTO local_binding_revisions(
                     binding_id, revision, project_id, absolute_path, availability, recorded_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    binding_id.as_bytes().as_slice(),
                    revision_i64(revision)?,
                    project_id.as_bytes().as_slice(),
                    path_text,
                    availability.as_str(),
                    now.as_unix_micros(),
                ],
            )
            .map_err(write_error)?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "bind_clone",
            &basis,
            "local_binding",
            binding_id.as_bytes(),
            revision,
            now,
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: LocalBinding {
                id: binding_id,
                project_id,
                absolute_path,
                availability,
                revision,
                bound_at: now,
            },
            replayed: false,
        })
    }

    pub fn get_local_binding(&self, project_id: ProjectId) -> Result<LocalBinding, Error> {
        load_binding_optional(&self.connection, project_id)?
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "local clone binding was not found"))
    }

    pub fn record_source(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        draft: SourceDraft,
    ) -> Result<OperationResult<Source>, Error> {
        validate_source_draft(&draft)?;
        let encoded = EncodedSource::from_payload(&draft.payload);
        let basis = source_basis(project_id, &draft, &encoded);
        let (connection, ids, clock) = (&mut self.connection, &mut self.ids, &mut self.clock);
        let transaction = begin_write(connection)?;

        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "record_source", &basis)?;
            let source = load_source(&transaction, SourceId::from_slice(&operation.result_id)?)?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value: source,
                replayed: true,
            });
        }

        let project = load_project(&transaction, project_id)?;
        ensure_revision(draft.expected_project_revision, project.revision, "Project")?;
        let source_id = SourceId::from_bytes(ids.next_id()?);
        let now = clock.now()?;
        transaction
            .execute(
                "INSERT INTO sources(
                     id, project_id, revision, source_kind, locator, snapshot_basis,
                     detail_one, detail_two, exit_code, termination, actor_kind,
                     actor_identity, observer_kind, observer_identity, availability, recorded_at
                 ) VALUES (
                     ?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15
                 )",
                params![
                    source_id.as_bytes().as_slice(),
                    project_id.as_bytes().as_slice(),
                    encoded.kind,
                    encoded.locator,
                    encoded.snapshot_basis,
                    encoded.detail_one,
                    encoded.detail_two,
                    encoded.exit_code,
                    encoded.termination,
                    draft.actor.kind.as_str(),
                    draft.actor.identity,
                    draft.observer.as_ref().map(|value| value.kind.as_str()),
                    draft.observer.as_ref().map(|value| value.identity.as_str()),
                    draft.availability.as_str(),
                    now.as_unix_micros(),
                ],
            )
            .map_err(|error| insert_identity_error(error, "Source identity already exists"))?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "record_source",
            &basis,
            "source",
            source_id.as_bytes(),
            1,
            now,
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: Source {
                id: source_id,
                project_id,
                payload: draft.payload,
                actor: draft.actor,
                observer: draft.observer,
                availability: draft.availability,
                recorded_at: now,
            },
            replayed: false,
        })
    }

    pub fn get_source(&self, project_id: ProjectId, source_id: SourceId) -> Result<Source, Error> {
        let source = load_source(&self.connection, source_id)?;
        if source.project_id != project_id {
            return Err(Error::new(
                ErrorKind::WrongProject,
                "Source belongs to a different Project",
            ));
        }
        Ok(source)
    }

    pub fn relate_sources(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        expected_project_revision: u64,
        from_source_id: SourceId,
        kind: SourceRelationKind,
        to_source_id: SourceId,
    ) -> Result<OperationResult<SourceRelation>, Error> {
        let basis = Basis::new("relate_sources")
            .bytes(project_id.as_bytes())
            .number(expected_project_revision)
            .bytes(from_source_id.as_bytes())
            .string(kind.as_str())
            .bytes(to_source_id.as_bytes())
            .finish();
        let (connection, clock) = (&mut self.connection, &mut self.clock);
        let transaction = begin_write(connection)?;
        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "relate_sources", &basis)?;
            let relation =
                load_relation(&transaction, project_id, from_source_id, kind, to_source_id)?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value: relation,
                replayed: true,
            });
        }

        let project = load_project(&transaction, project_id)?;
        ensure_revision(expected_project_revision, project.revision, "Project")?;
        ensure_source_project(&transaction, from_source_id, project_id)?;
        ensure_source_project(&transaction, to_source_id, project_id)?;
        if relation_exists(&transaction, project_id, from_source_id, kind, to_source_id)? {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                "Source relation already exists",
            ));
        }
        let now = clock.now()?;
        transaction
            .execute(
                "INSERT INTO source_relations(
                     project_id, from_source_id, relation_kind, to_source_id, recorded_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    project_id.as_bytes().as_slice(),
                    from_source_id.as_bytes().as_slice(),
                    kind.as_str(),
                    to_source_id.as_bytes().as_slice(),
                    now.as_unix_micros(),
                ],
            )
            .map_err(write_error)?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "relate_sources",
            &basis,
            "source_relation",
            from_source_id.as_bytes(),
            0,
            now,
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: SourceRelation {
                project_id,
                from_source_id,
                kind,
                to_source_id,
                recorded_at: now,
            },
            replayed: false,
        })
    }

    pub fn get_source_relation(
        &self,
        project_id: ProjectId,
        from_source_id: SourceId,
        kind: SourceRelationKind,
        to_source_id: SourceId,
    ) -> Result<SourceRelation, Error> {
        load_relation(
            &self.connection,
            project_id,
            from_source_id,
            kind,
            to_source_id,
        )
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), Error> {
    let transaction = connection.unchecked_transaction().map_err(write_error)?;
    transaction
        .execute_batch(
            "CREATE TABLE metadata(
                 key TEXT PRIMARY KEY NOT NULL,
                 value TEXT NOT NULL
             ) WITHOUT ROWID;
             CREATE TABLE projects(
                 id BLOB PRIMARY KEY NOT NULL CHECK(length(id) = 16),
                 display_name TEXT NOT NULL CHECK(length(display_name) > 0),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL
             );
             CREATE TABLE project_revisions(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 display_name TEXT NOT NULL CHECK(length(display_name) > 0),
                 recorded_at INTEGER NOT NULL,
                 PRIMARY KEY(project_id, revision),
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE local_bindings(
                 id BLOB PRIMARY KEY NOT NULL CHECK(length(id) = 16),
                 project_id BLOB UNIQUE NOT NULL CHECK(length(project_id) = 16),
                 absolute_path TEXT UNIQUE NOT NULL CHECK(length(absolute_path) > 0),
                 availability TEXT NOT NULL CHECK(availability IN ('available','unavailable','stale','unknown')),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 bound_at INTEGER NOT NULL,
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             );
             CREATE TABLE local_binding_revisions(
                 binding_id BLOB NOT NULL CHECK(length(binding_id) = 16),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 absolute_path TEXT NOT NULL CHECK(length(absolute_path) > 0),
                 availability TEXT NOT NULL CHECK(availability IN ('available','unavailable','stale','unknown')),
                 recorded_at INTEGER NOT NULL,
                 PRIMARY KEY(binding_id, revision),
                 FOREIGN KEY(binding_id) REFERENCES local_bindings(id) ON DELETE RESTRICT,
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE sources(
                 id BLOB PRIMARY KEY NOT NULL CHECK(length(id) = 16),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 revision INTEGER NOT NULL CHECK(revision = 1),
                 source_kind TEXT NOT NULL,
                 locator TEXT,
                 snapshot_basis TEXT,
                 detail_one TEXT,
                 detail_two TEXT,
                 exit_code INTEGER,
                 termination TEXT,
                 actor_kind TEXT NOT NULL,
                 actor_identity TEXT NOT NULL CHECK(length(actor_identity) > 0),
                 observer_kind TEXT,
                 observer_identity TEXT,
                 availability TEXT NOT NULL CHECK(availability IN ('available','unavailable','stale','unknown')),
                 recorded_at INTEGER NOT NULL,
                 UNIQUE(project_id, id),
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             );
             CREATE TABLE source_relations(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 from_source_id BLOB NOT NULL CHECK(length(from_source_id) = 16),
                 relation_kind TEXT NOT NULL CHECK(relation_kind IN ('derived_from','supported_by')),
                 to_source_id BLOB NOT NULL CHECK(length(to_source_id) = 16),
                 recorded_at INTEGER NOT NULL,
                 PRIMARY KEY(project_id, from_source_id, relation_kind, to_source_id),
                 FOREIGN KEY(project_id, from_source_id) REFERENCES sources(project_id, id) ON DELETE RESTRICT,
                 FOREIGN KEY(project_id, to_source_id) REFERENCES sources(project_id, id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE operations(
                 operation_id BLOB PRIMARY KEY NOT NULL CHECK(length(operation_id) = 16),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 operation_kind TEXT NOT NULL,
                 input_basis BLOB NOT NULL,
                 outcome TEXT NOT NULL CHECK(outcome = 'committed'),
                 result_kind TEXT NOT NULL,
                 result_id BLOB NOT NULL CHECK(length(result_id) = 16),
                 result_revision INTEGER NOT NULL CHECK(result_revision >= 0),
                 committed_at INTEGER NOT NULL,
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             ) WITHOUT ROWID;",
        )
        .map_err(write_error)?;
    transaction
        .execute(
            "INSERT INTO metadata(key, value) VALUES ('schema_kind', ?1), ('schema_version', ?2)",
            params![SCHEMA_KIND, SCHEMA_VERSION.to_string()],
        )
        .map_err(write_error)?;
    transaction.commit().map_err(commit_error)
}

fn validate_existing_schema(connection: &Connection) -> Result<(), Error> {
    let integrity: String = connection
        .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
        .map_err(read_corrupt_error)?;
    if integrity != "ok" {
        return Err(Error::new(
            ErrorKind::CorruptState,
            format!("SQLite integrity check failed: {integrity}"),
        ));
    }

    let mut statement = connection
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table'")
        .map_err(read_corrupt_error)?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(read_corrupt_error)?;
    let mut tables = BTreeSet::new();
    for name in names {
        tables.insert(name.map_err(read_corrupt_error)?);
    }
    for required in REQUIRED_TABLES {
        if !tables.contains(required) {
            return Err(Error::new(
                ErrorKind::CorruptState,
                format!("canonical store is missing required table {required}"),
            ));
        }
    }

    let kind = metadata_value(connection, "schema_kind")?;
    if kind != SCHEMA_KIND {
        return Err(Error::new(
            ErrorKind::CorruptState,
            format!("unexpected canonical schema kind {kind:?}"),
        ));
    }
    let version_text = metadata_value(connection, "schema_version")?;
    let version = version_text.parse::<u32>().map_err(|_| {
        Error::new(
            ErrorKind::CorruptState,
            "canonical schema version is malformed",
        )
    })?;
    if version != SCHEMA_VERSION {
        return Err(Error::new(
            ErrorKind::UnsupportedVersion,
            format!(
                "canonical schema version {version} is unsupported; current version is {SCHEMA_VERSION}"
            ),
        ));
    }

    let foreign_key_violation: Option<i64> = connection
        .query_row(
            "SELECT 1 FROM pragma_foreign_key_check LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(read_corrupt_error)?;
    if foreign_key_violation.is_some() {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "canonical store contains a foreign-key violation",
        ));
    }
    Ok(())
}

fn configure_and_verify_durability(connection: &Connection) -> Result<(), Error> {
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA synchronous = FULL;
             PRAGMA secure_delete = ON;",
        )
        .map_err(|error| {
            Error::with_source(
                ErrorKind::StorageUnavailable,
                "cannot apply SQLite durability profile",
                error,
            )
        })?;
    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .map_err(|error| {
            Error::with_source(
                ErrorKind::StorageUnavailable,
                "cannot enable SQLite WAL mode",
                error,
            )
        })?;
    let foreign_keys: i64 = pragma_integer(connection, "foreign_keys")?;
    let synchronous: i64 = pragma_integer(connection, "synchronous")?;
    let secure_delete: i64 = pragma_integer(connection, "secure_delete")?;
    if !journal_mode.eq_ignore_ascii_case("wal")
        || foreign_keys != 1
        || synchronous != 2
        || secure_delete != 1
    {
        return Err(Error::new(
            ErrorKind::StorageUnavailable,
            format!(
                "SQLite durability profile verification failed: journal_mode={journal_mode}, foreign_keys={foreign_keys}, synchronous={synchronous}, secure_delete={secure_delete}"
            ),
        ));
    }
    Ok(())
}

fn pragma_integer(connection: &Connection, name: &str) -> Result<i64, Error> {
    connection
        .query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
        .map_err(|error| {
            Error::with_source(
                ErrorKind::StorageUnavailable,
                format!("cannot verify SQLite PRAGMA {name}"),
                error,
            )
        })
}

fn metadata_value(connection: &Connection, key: &str) -> Result<String, Error> {
    connection
        .query_row("SELECT value FROM metadata WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()
        .map_err(read_corrupt_error)?
        .ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                format!("canonical store is missing metadata key {key}"),
            )
        })
}

fn begin_write(connection: &mut Connection) -> Result<Transaction<'_>, Error> {
    connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(write_error)
}

struct StoredOperation {
    kind: String,
    input_basis: Vec<u8>,
    result_kind: String,
    result_id: Vec<u8>,
    result_revision: u64,
}

fn load_operation(
    connection: &Connection,
    operation_id: OperationId,
) -> Result<Option<StoredOperation>, Error> {
    let row = connection
        .query_row(
            "SELECT operation_kind, input_basis, outcome, result_kind, result_id, result_revision
             FROM operations WHERE operation_id = ?1",
            [operation_id.as_bytes().as_slice()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()
        .map_err(read_error)?;
    row.map(
        |(kind, input_basis, outcome, result_kind, result_id, result_revision)| {
            if outcome != "committed" || result_revision < 0 {
                return Err(Error::new(
                    ErrorKind::RepairRequired,
                    "stored operation outcome is invalid",
                ));
            }
            Ok(StoredOperation {
                kind,
                input_basis,
                result_kind,
                result_id,
                result_revision: result_revision as u64,
            })
        },
    )
    .transpose()
}

fn ensure_replay_input(
    operation: &StoredOperation,
    expected_kind: &str,
    expected_basis: &[u8],
) -> Result<(), Error> {
    if operation.kind != expected_kind || operation.input_basis != expected_basis {
        return Err(Error::new(
            ErrorKind::DomainConflict,
            "OperationId was already committed with different input",
        ));
    }
    let expected_result_kind = match expected_kind {
        "create_project" | "rename_project" => "project",
        "bind_clone" => "local_binding",
        "record_source" => "source",
        "relate_sources" => "source_relation",
        _ => {
            return Err(Error::new(
                ErrorKind::RepairRequired,
                "stored operation kind is not recognized",
            ));
        }
    };
    if operation.result_kind != expected_result_kind {
        return Err(Error::new(
            ErrorKind::RepairRequired,
            "stored operation result kind does not match its command",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record_operation(
    transaction: &Transaction<'_>,
    operation_id: OperationId,
    project_id: ProjectId,
    operation_kind: &str,
    input_basis: &[u8],
    result_kind: &str,
    result_id: &[u8; 16],
    result_revision: u64,
    committed_at: TimestampMicros,
) -> Result<(), Error> {
    transaction
        .execute(
            "INSERT INTO operations(
                 operation_id, project_id, operation_kind, input_basis, outcome,
                 result_kind, result_id, result_revision, committed_at
             ) VALUES (?1, ?2, ?3, ?4, 'committed', ?5, ?6, ?7, ?8)",
            params![
                operation_id.as_bytes().as_slice(),
                project_id.as_bytes().as_slice(),
                operation_kind,
                input_basis,
                result_kind,
                result_id.as_slice(),
                revision_i64(result_revision)?,
                committed_at.as_unix_micros(),
            ],
        )
        .map_err(write_error)?;
    Ok(())
}

fn load_project(connection: &Connection, project_id: ProjectId) -> Result<Project, Error> {
    let row = connection
        .query_row(
            "SELECT display_name, revision, created_at, updated_at FROM projects WHERE id = ?1",
            [project_id.as_bytes().as_slice()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Project was not found"))?;
    Ok(Project {
        id: project_id,
        display_name: row.0,
        revision: stored_revision(row.1)?,
        created_at: TimestampMicros::from_unix_micros(row.2),
        updated_at: TimestampMicros::from_unix_micros(row.3),
    })
}

fn load_project_revision(
    connection: &Connection,
    project_id: ProjectId,
    revision: u64,
) -> Result<Project, Error> {
    let current = load_project(connection, project_id)?;
    let row = connection
        .query_row(
            "SELECT display_name, recorded_at FROM project_revisions
             WHERE project_id = ?1 AND revision = ?2",
            params![project_id.as_bytes().as_slice(), revision_i64(revision)?],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| {
            Error::new(
                ErrorKind::RepairRequired,
                "committed Project operation has no immutable result revision",
            )
        })?;
    Ok(Project {
        id: project_id,
        display_name: row.0,
        revision,
        created_at: current.created_at,
        updated_at: TimestampMicros::from_unix_micros(row.1),
    })
}

fn binding_path_owner(
    connection: &Connection,
    absolute_path: &str,
) -> Result<Option<ProjectId>, Error> {
    let bytes: Option<Vec<u8>> = connection
        .query_row(
            "SELECT project_id FROM local_bindings WHERE absolute_path = ?1",
            [absolute_path],
            |row| row.get(0),
        )
        .optional()
        .map_err(read_error)?;
    bytes.map(|value| ProjectId::from_slice(&value)).transpose()
}

fn load_binding_optional(
    connection: &Connection,
    project_id: ProjectId,
) -> Result<Option<LocalBinding>, Error> {
    let row = connection
        .query_row(
            "SELECT id, absolute_path, availability, revision, bound_at
             FROM local_bindings WHERE project_id = ?1",
            [project_id.as_bytes().as_slice()],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
        .map_err(read_error)?;
    row.map(|row| binding_from_row(project_id, row)).transpose()
}

fn load_binding_revision(
    connection: &Connection,
    binding_id: LocalBindingId,
    revision: u64,
) -> Result<LocalBinding, Error> {
    let row = connection
        .query_row(
            "SELECT project_id, absolute_path, availability, recorded_at
             FROM local_binding_revisions WHERE binding_id = ?1 AND revision = ?2",
            params![binding_id.as_bytes().as_slice(), revision_i64(revision)?],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| {
            Error::new(
                ErrorKind::RepairRequired,
                "committed binding operation has no immutable result revision",
            )
        })?;
    Ok(LocalBinding {
        id: binding_id,
        project_id: ProjectId::from_slice(&row.0)?,
        absolute_path: PathBuf::from(row.1),
        availability: parse_availability(&row.2)?,
        revision,
        bound_at: TimestampMicros::from_unix_micros(row.3),
    })
}

type BindingRow = (Vec<u8>, String, String, i64, i64);

fn binding_from_row(project_id: ProjectId, row: BindingRow) -> Result<LocalBinding, Error> {
    Ok(LocalBinding {
        id: LocalBindingId::from_slice(&row.0)?,
        project_id,
        absolute_path: PathBuf::from(row.1),
        availability: parse_availability(&row.2)?,
        revision: stored_revision(row.3)?,
        bound_at: TimestampMicros::from_unix_micros(row.4),
    })
}

type SourceRow = (
    Vec<u8>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<String>,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    i64,
);

fn load_source(connection: &Connection, source_id: SourceId) -> Result<Source, Error> {
    let row: SourceRow = connection
        .query_row(
            "SELECT project_id, source_kind, locator, snapshot_basis, detail_one, detail_two,
                    exit_code, termination, actor_kind, actor_identity, observer_kind,
                    observer_identity, availability, recorded_at
             FROM sources WHERE id = ?1",
            [source_id.as_bytes().as_slice()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                ))
            },
        )
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Source was not found"))?;
    source_from_row(source_id, row)
}

fn source_from_row(source_id: SourceId, row: SourceRow) -> Result<Source, Error> {
    let observer = match (row.10, row.11) {
        (None, None) => None,
        (Some(kind), Some(identity)) => Some(Principal {
            kind: parse_principal_kind(&kind)?,
            identity,
        }),
        _ => {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "stored Source observer provenance is incomplete",
            ));
        }
    };
    Ok(Source {
        id: source_id,
        project_id: ProjectId::from_slice(&row.0)?,
        payload: decode_payload(&row.1, row.2, row.3, row.4, row.5, row.6, row.7)?,
        actor: Principal {
            kind: parse_principal_kind(&row.8)?,
            identity: row.9,
        },
        observer,
        availability: parse_availability(&row.12)?,
        recorded_at: TimestampMicros::from_unix_micros(row.13),
    })
}

fn ensure_source_project(
    connection: &Connection,
    source_id: SourceId,
    project_id: ProjectId,
) -> Result<(), Error> {
    let source = load_source(connection, source_id)?;
    if source.project_id != project_id {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "cross-Project Source relation is not allowed",
        ));
    }
    Ok(())
}

fn relation_exists(
    connection: &Connection,
    project_id: ProjectId,
    from_source_id: SourceId,
    kind: SourceRelationKind,
    to_source_id: SourceId,
) -> Result<bool, Error> {
    connection
        .query_row(
            "SELECT 1 FROM source_relations
             WHERE project_id = ?1 AND from_source_id = ?2
               AND relation_kind = ?3 AND to_source_id = ?4",
            params![
                project_id.as_bytes().as_slice(),
                from_source_id.as_bytes().as_slice(),
                kind.as_str(),
                to_source_id.as_bytes().as_slice(),
            ],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(read_error)
}

fn load_relation(
    connection: &Connection,
    project_id: ProjectId,
    from_source_id: SourceId,
    kind: SourceRelationKind,
    to_source_id: SourceId,
) -> Result<SourceRelation, Error> {
    let row: Option<(String, i64)> = connection
        .query_row(
            "SELECT relation_kind, recorded_at FROM source_relations
             WHERE project_id = ?1 AND from_source_id = ?2
               AND relation_kind = ?3 AND to_source_id = ?4",
            params![
                project_id.as_bytes().as_slice(),
                from_source_id.as_bytes().as_slice(),
                kind.as_str(),
                to_source_id.as_bytes().as_slice(),
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(read_error)?;
    let (stored_kind, recorded_at) =
        row.ok_or_else(|| Error::new(ErrorKind::NotFound, "Source relation was not found"))?;
    Ok(SourceRelation {
        project_id,
        from_source_id,
        kind: SourceRelationKind::parse(&stored_kind).ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "stored Source relation kind is invalid",
            )
        })?,
        to_source_id,
        recorded_at: TimestampMicros::from_unix_micros(recorded_at),
    })
}

struct EncodedSource<'a> {
    kind: &'static str,
    locator: Option<&'a str>,
    snapshot_basis: Option<&'a str>,
    detail_one: Option<&'a str>,
    detail_two: Option<&'a str>,
    exit_code: Option<i32>,
    termination: Option<&'static str>,
}

impl<'a> EncodedSource<'a> {
    fn from_payload(payload: &'a SourcePayload) -> Self {
        let mut value = Self {
            kind: payload.kind(),
            locator: None,
            snapshot_basis: None,
            detail_one: None,
            detail_two: None,
            exit_code: None,
            termination: None,
        };
        match payload {
            SourcePayload::RepositorySnapshot { revision } => {
                value.snapshot_basis = Some(revision);
            }
            SourcePayload::RepositoryCommit { commit } => {
                value.snapshot_basis = Some(commit);
            }
            SourcePayload::File { locator, snapshot }
            | SourcePayload::Symbol { locator, snapshot }
            | SourcePayload::AdoptedArtifact {
                locator,
                revision: snapshot,
            } => {
                value.locator = Some(locator);
                value.snapshot_basis = Some(snapshot);
            }
            SourcePayload::CommandExecution {
                command_label,
                outcome,
            } => {
                value.locator = Some(command_label);
                value.exit_code = outcome.exit_code;
                value.termination = Some(outcome.termination.as_str());
            }
            SourcePayload::CurrentHostUserTurn {
                host,
                session,
                turn,
            } => {
                value.locator = Some(turn);
                value.detail_one = Some(host);
                value.detail_two = Some(session);
            }
            SourcePayload::Url { url } => value.locator = Some(url),
        }
        value
    }
}

#[allow(clippy::too_many_arguments)]
fn decode_payload(
    kind: &str,
    locator: Option<String>,
    snapshot_basis: Option<String>,
    detail_one: Option<String>,
    detail_two: Option<String>,
    exit_code: Option<i32>,
    termination: Option<String>,
) -> Result<SourcePayload, Error> {
    let missing = || {
        Error::new(
            ErrorKind::CorruptState,
            format!("stored {kind} Source payload is incomplete"),
        )
    };
    match kind {
        "repository_snapshot" => Ok(SourcePayload::RepositorySnapshot {
            revision: snapshot_basis.ok_or_else(missing)?,
        }),
        "repository_commit" => Ok(SourcePayload::RepositoryCommit {
            commit: snapshot_basis.ok_or_else(missing)?,
        }),
        "file" => Ok(SourcePayload::File {
            locator: locator.ok_or_else(missing)?,
            snapshot: snapshot_basis.ok_or_else(missing)?,
        }),
        "symbol" => Ok(SourcePayload::Symbol {
            locator: locator.ok_or_else(missing)?,
            snapshot: snapshot_basis.ok_or_else(missing)?,
        }),
        "command_execution" => Ok(SourcePayload::CommandExecution {
            command_label: locator.ok_or_else(missing)?,
            outcome: CommandOutcome {
                exit_code,
                termination: CommandTermination::parse(&termination.ok_or_else(missing)?)
                    .ok_or_else(missing)?,
            },
        }),
        "current_host_user_turn" => Ok(SourcePayload::CurrentHostUserTurn {
            host: detail_one.ok_or_else(missing)?,
            session: detail_two.ok_or_else(missing)?,
            turn: locator.ok_or_else(missing)?,
        }),
        "url" => Ok(SourcePayload::Url {
            url: locator.ok_or_else(missing)?,
        }),
        "adopted_artifact" => Ok(SourcePayload::AdoptedArtifact {
            locator: locator.ok_or_else(missing)?,
            revision: snapshot_basis.ok_or_else(missing)?,
        }),
        _ => Err(Error::new(
            ErrorKind::CorruptState,
            format!("stored Source kind {kind:?} is invalid"),
        )),
    }
}

fn validate_source_draft(draft: &SourceDraft) -> Result<(), Error> {
    validate_nonempty("Source actor identity", &draft.actor.identity)?;
    if let Some(observer) = &draft.observer {
        validate_nonempty("Source observer identity", &observer.identity)?;
    }
    let encoded = EncodedSource::from_payload(&draft.payload);
    for (label, value) in [
        ("Source locator", encoded.locator),
        ("Source snapshot basis", encoded.snapshot_basis),
        ("Source host", encoded.detail_one),
        ("Source session", encoded.detail_two),
    ] {
        if let Some(value) = value {
            validate_nonempty(label, value)?;
        }
    }
    Ok(())
}

fn source_basis(
    project_id: ProjectId,
    draft: &SourceDraft,
    encoded: &EncodedSource<'_>,
) -> Vec<u8> {
    let mut basis = Basis::new("record_source")
        .bytes(project_id.as_bytes())
        .number(draft.expected_project_revision)
        .string(encoded.kind)
        .optional_string(encoded.locator)
        .optional_string(encoded.snapshot_basis)
        .optional_string(encoded.detail_one)
        .optional_string(encoded.detail_two)
        .optional_i32(encoded.exit_code)
        .optional_string(encoded.termination)
        .string(draft.actor.kind.as_str())
        .string(&draft.actor.identity);
    if let Some(observer) = &draft.observer {
        basis = basis
            .string("observer")
            .string(observer.kind.as_str())
            .string(&observer.identity);
    } else {
        basis = basis.string("no_observer");
    }
    basis.string(draft.availability.as_str()).finish()
}

struct Basis(Vec<u8>);

impl Basis {
    fn new(kind: &str) -> Self {
        Self(Vec::new()).string(kind)
    }

    fn bytes(mut self, value: &[u8]) -> Self {
        self.0
            .extend_from_slice(&(value.len() as u64).to_be_bytes());
        self.0.extend_from_slice(value);
        self
    }

    fn string(self, value: &str) -> Self {
        self.bytes(value.as_bytes())
    }

    fn number(self, value: u64) -> Self {
        self.bytes(&value.to_be_bytes())
    }

    fn optional_number(self, value: Option<u64>) -> Self {
        match value {
            Some(value) => self.string("some").number(value),
            None => self.string("none"),
        }
    }

    fn optional_i32(self, value: Option<i32>) -> Self {
        match value {
            Some(value) => self.string("some").bytes(&value.to_be_bytes()),
            None => self.string("none"),
        }
    }

    fn optional_string(self, value: Option<&str>) -> Self {
        match value {
            Some(value) => self.string("some").string(value),
            None => self.string("none"),
        }
    }

    fn finish(self) -> Vec<u8> {
        self.0
    }
}

fn parse_availability(value: &str) -> Result<Availability, Error> {
    Availability::parse(value).ok_or_else(|| {
        Error::new(
            ErrorKind::CorruptState,
            format!("stored availability {value:?} is invalid"),
        )
    })
}

fn parse_principal_kind(value: &str) -> Result<PrincipalKind, Error> {
    PrincipalKind::parse(value).ok_or_else(|| {
        Error::new(
            ErrorKind::CorruptState,
            format!("stored principal kind {value:?} is invalid"),
        )
    })
}

fn validate_nonempty(label: &str, value: &str) -> Result<(), Error> {
    if value.trim().is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must not be empty"),
        ));
    }
    Ok(())
}

fn ensure_revision(expected: u64, actual: u64, entity: &str) -> Result<(), Error> {
    if expected != actual {
        return Err(Error::new(
            ErrorKind::StaleBasis,
            format!(
                "{entity} basis is stale: expected revision {expected}, current revision {actual}"
            ),
        ));
    }
    Ok(())
}

fn ensure_single_updated(count: usize, message: &str) -> Result<(), Error> {
    if count != 1 {
        return Err(Error::new(ErrorKind::StaleBasis, message));
    }
    Ok(())
}

fn revision_i64(revision: u64) -> Result<i64, Error> {
    i64::try_from(revision).map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            "revision is outside the supported range",
        )
    })
}

fn stored_revision(revision: i64) -> Result<u64, Error> {
    u64::try_from(revision).map_err(|_| {
        Error::new(
            ErrorKind::CorruptState,
            "stored revision is outside the supported range",
        )
    })
}

fn map_open_error(error: rusqlite::Error, message: String) -> Error {
    let kind = match sqlite_code(&error) {
        Some(rusqlite::ErrorCode::DatabaseCorrupt) | Some(rusqlite::ErrorCode::NotADatabase) => {
            ErrorKind::CorruptState
        }
        _ => ErrorKind::StorageUnavailable,
    };
    Error::with_source(kind, message, error)
}

fn read_error(error: rusqlite::Error) -> Error {
    let kind = match sqlite_code(&error) {
        Some(rusqlite::ErrorCode::DatabaseCorrupt) | Some(rusqlite::ErrorCode::NotADatabase) => {
            ErrorKind::CorruptState
        }
        _ => ErrorKind::TransactionFailure,
    };
    Error::with_source(kind, "cannot read canonical state", error)
}

fn read_corrupt_error(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::CorruptState,
        "canonical store schema or content is malformed",
        error,
    )
}

fn write_error(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::TransactionFailure,
        "canonical transaction failed",
        error,
    )
}

fn commit_error(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::IndeterminateOutcome,
        "canonical commit outcome could not be confirmed",
        error,
    )
}

fn insert_identity_error(error: rusqlite::Error, message: &str) -> Error {
    match sqlite_code(&error) {
        Some(rusqlite::ErrorCode::ConstraintViolation) => {
            Error::with_source(ErrorKind::AlreadyExists, message, error)
        }
        _ => write_error(error),
    }
}

fn sqlite_code(error: &rusqlite::Error) -> Option<rusqlite::ErrorCode> {
    match error {
        rusqlite::Error::SqliteFailure(inner, _) => Some(inner.code),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::Store;

    #[test]
    fn every_store_connection_has_the_verified_durability_profile(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path().join("context.sqlite3"))?;
        let foreign_keys: i64 = store
            .connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
        let journal: String = store
            .connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
        let synchronous: i64 = store
            .connection
            .query_row("PRAGMA synchronous", [], |row| row.get(0))?;
        let secure_delete: i64 = store
            .connection
            .query_row("PRAGMA secure_delete", [], |row| row.get(0))?;
        assert_eq!(foreign_keys, 1);
        assert!(journal.eq_ignore_ascii_case("wal"));
        assert_eq!(synchronous, 2);
        assert_eq!(secure_delete, 1);
        Ok(())
    }
}
