use crate::identity::{IdGenerator, SystemIdGenerator};
use crate::model::{
    AgentRecommendation, ApplicabilityScope, Availability, CanonicalInvalidation,
    CanonicalRecordId, CanonicalRecordKind, CanonicalRelation, CanonicalRelationKind, Checkpoint,
    CheckpointDraft, CheckpointKind, CommandOutcome, CommandTermination, ContextItem,
    ContextItemCorrectionDraft, ContextItemDraft, ContextItemRole, CorrectionKind, Decision,
    DecisionChoice, DecisionCorrectionDraft, DecisionLifecycle, DecisionSupersessionDraft,
    ExplicitQuestionResponse, ForgetResult, LocalBinding, OperationResult, Principal,
    PrincipalKind, Project, Question, QuestionAlternative, QuestionDependency, QuestionDraft,
    QuestionReference, QuestionResponseDraft, QuestionResponseResult, QuestionState,
    QuestionTerminalOutcome, ReviewDue, ReviewDueDraft, ReviewDueKind, Source, SourceDraft,
    SourcePayload, SourceRelation, SourceRelationKind, StatementProvenanceRole, Tombstone,
    UserAcceptanceFact, UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource,
    VerificationFact, VerificationState, WorkState,
};
use crate::time::{Clock, SystemClock, TimestampMicros};
use crate::{
    CheckpointId, ContextItemId, DecisionId, Error, ErrorKind, LocalBindingId, OperationId,
    ProjectId, QuestionId, SourceId,
};
use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const SCHEMA_KIND: &str = "volicord-context";
pub const SCHEMA_VERSION: u32 = 9;

const REQUIRED_TABLES: [&str; 29] = [
    "metadata",
    "projects",
    "project_revisions",
    "local_bindings",
    "local_binding_revisions",
    "sources",
    "source_relations",
    "operations",
    "operation_dependencies",
    "questions",
    "question_revisions",
    "question_response_sources",
    "decisions",
    "context_items",
    "context_item_sources",
    "checkpoints",
    "checkpoint_source_relations",
    "checkpoint_decisions",
    "checkpoint_questions",
    "checkpoint_verifications",
    "context_item_revisions",
    "decision_revisions",
    "canonical_relations",
    "review_due",
    "tombstones",
    "managed_bundle_paths",
    "bundle_lineage",
    "merge_events",
    "merge_sanitation",
];

/// One synchronous connection to an explicit Canonical Context store path.
///
/// Mutation methods require `&mut self`, and each begins an immediate SQLite
/// transaction. SQLite therefore serializes writers both within this handle
/// and across handles without an implicit retry under a new operation ID.
pub struct Store {
    pub(crate) connection: Connection,
    pub(crate) ids: Box<dyn IdGenerator>,
    pub(crate) clock: Box<dyn Clock>,
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
            &[CanonicalRecordId::Project(project_id)],
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
            &[CanonicalRecordId::Project(project_id)],
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
            &[CanonicalRecordId::Project(project_id)],
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
            &[CanonicalRecordId::Source(source_id)],
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
            &[
                CanonicalRecordId::Source(from_source_id),
                CanonicalRecordId::Source(to_source_id),
            ],
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

    pub fn create_question(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        draft: QuestionDraft,
    ) -> Result<OperationResult<Question>, Error> {
        validate_question_draft(&draft)?;
        let encoded = EncodedQuestion::from_draft(&draft)?;
        let basis = question_basis(project_id, &draft, &encoded);
        let (connection, ids, clock) = (&mut self.connection, &mut self.ids, &mut self.clock);
        let transaction = begin_write(connection)?;
        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "create_question", &basis)?;
            let question = load_question(
                &transaction,
                project_id,
                QuestionId::from_slice(&operation.result_id)?,
            )?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value: question,
                replayed: true,
            });
        }

        let project = load_project(&transaction, project_id)?;
        ensure_revision(draft.expected_project_revision, project.revision, "Project")?;
        for source_id in draft
            .source_basis
            .iter()
            .chain(draft.recommendation.source_basis.iter())
        {
            ensure_source_project(&transaction, *source_id, project_id)?;
        }
        for dependency in &draft.dependencies {
            let dependency_question =
                load_question(&transaction, project_id, dependency.question_id)?;
            if let Some(required_revision) = dependency.required_revision {
                ensure_question_revision_exists(
                    &transaction,
                    dependency.question_id,
                    required_revision,
                )?;
                if required_revision > dependency_question.revision {
                    return Err(Error::new(
                        ErrorKind::StaleBasis,
                        "Question dependency revision is not current or historical",
                    ));
                }
            }
        }

        let question_id = QuestionId::from_bytes(ids.next_id()?);
        let now = clock.now()?;
        transaction
            .execute(
                "INSERT INTO questions(
                     id, project_id, revision, terminal_outcome, created_at, updated_at
                 ) VALUES (?1, ?2, 1, NULL, ?3, ?3)",
                params![
                    question_id.as_bytes().as_slice(),
                    project_id.as_bytes().as_slice(),
                    now.as_unix_micros(),
                ],
            )
            .map_err(|error| insert_identity_error(error, "Question identity already exists"))?;
        insert_question_revision(&transaction, question_id, project_id, &draft, &encoded, now)?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "create_question",
            &basis,
            "question",
            question_id.as_bytes(),
            1,
            now,
            &[CanonicalRecordId::Question(question_id)],
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: Question {
                id: question_id,
                project_id,
                revision: 1,
                prompt_basis: draft.prompt_basis,
                source_basis: draft.source_basis,
                dependencies: draft.dependencies,
                alternatives: draft.alternatives,
                recommendation: draft.recommendation,
                trade_offs: draft.trade_offs,
                uncertainty: draft.uncertainty,
                material_scope: draft.material_scope,
                state: QuestionState::Open,
                created_at: now,
                updated_at: now,
            },
            replayed: false,
        })
    }

    pub fn get_question(
        &self,
        project_id: ProjectId,
        question_id: QuestionId,
    ) -> Result<Question, Error> {
        load_question(&self.connection, project_id, question_id)
    }

    pub fn record_question_response(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        draft: QuestionResponseDraft,
    ) -> Result<OperationResult<QuestionResponseResult>, Error> {
        validate_response_draft(&draft)?;
        let basis = response_basis(project_id, &draft)?;
        let (connection, ids, clock) = (&mut self.connection, &mut self.ids, &mut self.clock);
        let transaction = begin_write(connection)?;
        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "record_question_response", &basis)?;
            let value = load_question_response(
                &transaction,
                project_id,
                QuestionId::from_slice(&operation.result_id)?,
                operation.result_revision,
            )?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value,
                replayed: true,
            });
        }

        let project = load_project(&transaction, project_id)?;
        ensure_revision(draft.expected_project_revision, project.revision, "Project")?;
        let question = load_question(&transaction, project_id, draft.question_id)?;
        ensure_revision(draft.question_revision, question.revision, "Question")?;
        if question.state != QuestionState::Open {
            return Err(Error::new(
                ErrorKind::DomainConflict,
                "Question is already terminal",
            ));
        }
        let current_alternative_keys: Vec<String> = question
            .alternatives
            .iter()
            .map(|alternative| alternative.key.clone())
            .collect();
        if draft.displayed_alternative_keys != current_alternative_keys
            || draft.displayed_recommendation_key != question.recommendation.alternative_key
        {
            return Err(Error::new(
                ErrorKind::StaleBasis,
                "displayed alternatives or recommendation do not match the Question revision",
            ));
        }
        let (outcome, choice, rationale) = interpret_explicit_response(&question, &draft.response)?;

        let now = clock.now()?;
        let user_turn_source = match &draft.user_turn_source {
            UserTurnSource::Existing(source_id) => {
                let source = load_source(&transaction, *source_id)?;
                ensure_user_turn_source(&source, project_id)?;
                source
            }
            UserTurnSource::Create(source_draft) => {
                ensure_revision(
                    source_draft.expected_project_revision,
                    project.revision,
                    "Project",
                )?;
                validate_source_draft(source_draft)?;
                ensure_user_turn_draft(source_draft)?;
                let source_id = SourceId::from_bytes(ids.next_id()?);
                insert_source(&transaction, source_id, project_id, source_draft, now)?;
                Source {
                    id: source_id,
                    project_id,
                    payload: source_draft.payload.clone(),
                    actor: source_draft.actor.clone(),
                    observer: source_draft.observer.clone(),
                    availability: source_draft.availability,
                    recorded_at: now,
                }
            }
        };
        transaction
            .execute(
                "INSERT INTO question_response_sources(
                     project_id, question_id, question_revision, source_id, recorded_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    project_id.as_bytes().as_slice(),
                    draft.question_id.as_bytes().as_slice(),
                    revision_i64(draft.question_revision)?,
                    user_turn_source.id.as_bytes().as_slice(),
                    now.as_unix_micros(),
                ],
            )
            .map_err(write_error)?;

        let decision = if let Some(choice) = choice {
            let decision_id = DecisionId::from_bytes(ids.next_id()?);
            let (choice_kind, choice_value) = match &choice {
                DecisionChoice::Alternative { alternative_key } => {
                    ("alternative", alternative_key.as_str())
                }
                DecisionChoice::Delegation { delegate_to } => ("delegation", delegate_to.as_str()),
            };
            transaction
                .execute(
                    "INSERT INTO decisions(
                         id, project_id, revision, question_id, question_revision, user_turn_source_id,
                         choice_kind, choice_value, user_rationale, displayed_alternatives,
                         recommendation_key, recommendation_rationale, recommendation_sources,
                         applicability_paths, applicability_components, applicability_work_contexts,
                         assumptions, revisit_triggers, recorded_at
                     ) VALUES (
                         ?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                         ?13, ?14, ?15, ?16, ?17, ?18
                     )",
                    params![
                        decision_id.as_bytes().as_slice(),
                        project_id.as_bytes().as_slice(),
                        draft.question_id.as_bytes().as_slice(),
                        revision_i64(draft.question_revision)?,
                        user_turn_source.id.as_bytes().as_slice(),
                        choice_kind,
                        choice_value,
                        rationale,
                        encode_alternatives(&question.alternatives),
                        question.recommendation.alternative_key,
                        question.recommendation.rationale,
                        encode_source_ids(&question.recommendation.source_basis),
                        encode_strings(&draft.applicability.paths),
                        encode_strings(&draft.applicability.components),
                        encode_strings(&draft.applicability.work_contexts),
                        encode_strings(&draft.assumptions),
                        encode_strings(&draft.revisit_triggers),
                        now.as_unix_micros(),
                    ],
                )
                .map_err(|error| {
                    insert_identity_error(error, "Decision identity already exists")
                })?;
            insert_decision_revision(
                &transaction,
                decision_id,
                project_id,
                1,
                draft.question_id,
                draft.question_revision,
                user_turn_source.id,
                choice_kind,
                choice_value,
                rationale,
                &question.alternatives,
                &question.recommendation,
                &draft.applicability,
                &draft.assumptions,
                &draft.revisit_triggers,
                None,
                None,
                now,
            )?;
            Some(Decision {
                id: decision_id,
                project_id,
                revision: 1,
                question_id: draft.question_id,
                question_revision: draft.question_revision,
                user_turn_source_id: user_turn_source.id,
                choice,
                user_rationale: rationale.map(str::to_owned),
                displayed_alternatives: question.alternatives.clone(),
                displayed_recommendation: question.recommendation.clone(),
                applicability: draft.applicability.clone(),
                assumptions: draft.assumptions.clone(),
                revisit_triggers: draft.revisit_triggers.clone(),
                recorded_at: now,
            })
        } else {
            None
        };
        transaction
            .execute(
                "UPDATE questions SET terminal_outcome = ?3, updated_at = ?4
                 WHERE id = ?1 AND project_id = ?2 AND revision = ?5 AND terminal_outcome IS NULL",
                params![
                    draft.question_id.as_bytes().as_slice(),
                    project_id.as_bytes().as_slice(),
                    outcome.as_str(),
                    now.as_unix_micros(),
                    revision_i64(draft.question_revision)?,
                ],
            )
            .map_err(write_error)
            .and_then(|count| ensure_single_updated(count, "Question changed concurrently"))?;
        let mut operation_dependencies = vec![
            CanonicalRecordId::Question(draft.question_id),
            CanonicalRecordId::Source(user_turn_source.id),
        ];
        if let Some(value) = &decision {
            operation_dependencies.push(CanonicalRecordId::Decision(value.id));
        }
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "record_question_response",
            &basis,
            "question_response",
            draft.question_id.as_bytes(),
            draft.question_revision,
            now,
            &operation_dependencies,
        )?;
        transaction.commit().map_err(commit_error)?;

        let mut terminal_question = question;
        terminal_question.state = QuestionState::Terminal(outcome);
        terminal_question.updated_at = now;
        Ok(OperationResult {
            value: QuestionResponseResult {
                question: terminal_question,
                user_turn_source,
                decision,
            },
            replayed: false,
        })
    }

    pub fn get_decision(
        &self,
        project_id: ProjectId,
        decision_id: DecisionId,
    ) -> Result<Decision, Error> {
        load_decision(&self.connection, project_id, decision_id)
    }

    pub fn record_context_item(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        draft: ContextItemDraft,
    ) -> Result<OperationResult<ContextItem>, Error> {
        validate_context_item_draft(&draft)?;
        let basis = context_item_basis(project_id, &draft);
        let (connection, ids, clock) = (&mut self.connection, &mut self.ids, &mut self.clock);
        let transaction = begin_write(connection)?;
        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "record_context_item", &basis)?;
            let item = load_context_item(
                &transaction,
                project_id,
                ContextItemId::from_slice(&operation.result_id)?,
            )?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value: item,
                replayed: true,
            });
        }
        let project = load_project(&transaction, project_id)?;
        ensure_revision(draft.expected_project_revision, project.revision, "Project")?;
        let sources = load_context_sources(&transaction, project_id, &draft.source_basis)?;
        validate_context_provenance(&draft, &sources)?;
        let item_id = ContextItemId::from_bytes(ids.next_id()?);
        let now = clock.now()?;
        transaction
            .execute(
                "INSERT INTO context_items(
                     id, project_id, revision, role, statement, provenance_role,
                     author_kind, author_identity, applicability_paths,
                     applicability_components, applicability_work_contexts, recorded_at
                 ) VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    item_id.as_bytes().as_slice(),
                    project_id.as_bytes().as_slice(),
                    draft.role.as_str(),
                    draft.statement,
                    draft.provenance_role.as_str(),
                    draft.author.kind.as_str(),
                    draft.author.identity,
                    encode_strings(&draft.applicability.paths),
                    encode_strings(&draft.applicability.components),
                    encode_strings(&draft.applicability.work_contexts),
                    now.as_unix_micros(),
                ],
            )
            .map_err(|error| {
                insert_identity_error(error, "Context Item identity already exists")
            })?;
        for (position, source_id) in draft.source_basis.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO context_item_sources(
                         project_id, context_item_id, source_id, position
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        project_id.as_bytes().as_slice(),
                        item_id.as_bytes().as_slice(),
                        source_id.as_bytes().as_slice(),
                        position_i64(position)?,
                    ],
                )
                .map_err(write_error)?;
        }
        insert_context_item_revision(
            &transaction,
            item_id,
            project_id,
            1,
            &draft,
            None,
            None,
            now,
        )?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "record_context_item",
            &basis,
            "context_item",
            item_id.as_bytes(),
            1,
            now,
            &[CanonicalRecordId::ContextItem(item_id)],
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: ContextItem {
                id: item_id,
                project_id,
                revision: 1,
                role: draft.role,
                statement: draft.statement,
                provenance_role: draft.provenance_role,
                author: draft.author,
                source_basis: draft.source_basis,
                applicability: draft.applicability,
                recorded_at: now,
            },
            replayed: false,
        })
    }

    pub fn get_context_item(
        &self,
        project_id: ProjectId,
        item_id: ContextItemId,
    ) -> Result<ContextItem, Error> {
        load_context_item(&self.connection, project_id, item_id)
    }

    pub fn record_checkpoint(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        draft: CheckpointDraft,
    ) -> Result<OperationResult<Checkpoint>, Error> {
        validate_checkpoint_draft(&draft)?;
        let basis = checkpoint_basis(project_id, &draft);
        let (connection, ids, clock) = (&mut self.connection, &mut self.ids, &mut self.clock);
        let transaction = begin_write(connection)?;
        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "record_checkpoint", &basis)?;
            let checkpoint = load_checkpoint(
                &transaction,
                project_id,
                CheckpointId::from_slice(&operation.result_id)?,
            )?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value: checkpoint,
                replayed: true,
            });
        }
        let project = load_project(&transaction, project_id)?;
        ensure_revision(draft.expected_project_revision, project.revision, "Project")?;
        for source_id in draft
            .source_basis
            .iter()
            .chain(draft.changed_source_basis.iter())
        {
            ensure_source_project(&transaction, *source_id, project_id)?;
        }
        for decision_id in &draft.applied_decisions {
            load_decision(&transaction, project_id, *decision_id)?;
        }
        for question in &draft.open_questions {
            load_question(&transaction, project_id, question.question_id)?;
            ensure_question_revision_exists(&transaction, question.question_id, question.revision)?;
        }
        for verification in &draft.verification {
            if let Some(source_id) = verification.source_id {
                let source = load_source(&transaction, source_id)?;
                if source.project_id != project_id {
                    return Err(Error::new(
                        ErrorKind::WrongProject,
                        "verification Source belongs to a different Project",
                    ));
                }
                if !matches!(source.payload, SourcePayload::CommandExecution { .. }) {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "executed verification requires a command-execution Source",
                    ));
                }
            }
        }
        for source_id in [draft.user_review.source_id, draft.user_acceptance.source_id]
            .into_iter()
            .flatten()
        {
            let source = load_source(&transaction, source_id)?;
            ensure_user_turn_source(&source, project_id)?;
        }

        let checkpoint_id = CheckpointId::from_bytes(ids.next_id()?);
        let now = clock.now()?;
        transaction
            .execute(
                "INSERT INTO checkpoints(
                     id, project_id, revision, checkpoint_kind, goal, work_state, state_change,
                     changed_paths, user_review, user_review_source_id, user_acceptance,
                     user_acceptance_source_id, known_limits, non_goals, next_step,
                     handoff_to, recorded_at
                 ) VALUES (
                     ?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16
                 )",
                params![
                    checkpoint_id.as_bytes().as_slice(),
                    project_id.as_bytes().as_slice(),
                    draft.kind.as_str(),
                    draft.goal,
                    draft.work_state.as_str(),
                    draft.state_change,
                    encode_strings(&draft.changed_paths),
                    draft.user_review.state.as_str(),
                    draft
                        .user_review
                        .source_id
                        .map(|source_id| source_id.as_bytes().to_vec()),
                    draft.user_acceptance.state.as_str(),
                    draft
                        .user_acceptance
                        .source_id
                        .map(|source_id| source_id.as_bytes().to_vec()),
                    encode_strings(&draft.known_limits),
                    encode_strings(&draft.non_goals),
                    draft.next_step,
                    draft.handoff_to,
                    now.as_unix_micros(),
                ],
            )
            .map_err(|error| insert_identity_error(error, "Checkpoint identity already exists"))?;
        insert_checkpoint_sources(
            &transaction,
            project_id,
            checkpoint_id,
            "supported_by",
            &draft.source_basis,
        )?;
        insert_checkpoint_sources(
            &transaction,
            project_id,
            checkpoint_id,
            "changed_basis",
            &draft.changed_source_basis,
        )?;
        for (position, decision_id) in draft.applied_decisions.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO checkpoint_decisions(
                         project_id, checkpoint_id, decision_id, position
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        project_id.as_bytes().as_slice(),
                        checkpoint_id.as_bytes().as_slice(),
                        decision_id.as_bytes().as_slice(),
                        position_i64(position)?,
                    ],
                )
                .map_err(write_error)?;
        }
        for (position, question) in draft.open_questions.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO checkpoint_questions(
                         project_id, checkpoint_id, question_id, question_revision, position
                     ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        project_id.as_bytes().as_slice(),
                        checkpoint_id.as_bytes().as_slice(),
                        question.question_id.as_bytes().as_slice(),
                        revision_i64(question.revision)?,
                        position_i64(position)?,
                    ],
                )
                .map_err(write_error)?;
        }
        for (position, verification) in draft.verification.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO checkpoint_verifications(
                         project_id, checkpoint_id, position, verification_state, source_id, outcome
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        project_id.as_bytes().as_slice(),
                        checkpoint_id.as_bytes().as_slice(),
                        position_i64(position)?,
                        verification.state.as_str(),
                        verification
                            .source_id
                            .map(|source_id| source_id.as_bytes().to_vec()),
                        verification.outcome,
                    ],
                )
                .map_err(write_error)?;
        }
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "record_checkpoint",
            &basis,
            "checkpoint",
            checkpoint_id.as_bytes(),
            1,
            now,
            &[CanonicalRecordId::Checkpoint(checkpoint_id)],
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: Checkpoint {
                id: checkpoint_id,
                project_id,
                revision: 1,
                kind: draft.kind,
                goal: draft.goal,
                work_state: draft.work_state,
                state_change: draft.state_change,
                source_basis: draft.source_basis,
                changed_source_basis: draft.changed_source_basis,
                changed_paths: draft.changed_paths,
                applied_decisions: draft.applied_decisions,
                verification: draft.verification,
                user_review: draft.user_review,
                user_acceptance: draft.user_acceptance,
                known_limits: draft.known_limits,
                non_goals: draft.non_goals,
                open_questions: draft.open_questions,
                next_step: draft.next_step,
                handoff_to: draft.handoff_to,
                recorded_at: now,
            },
            replayed: false,
        })
    }

    pub fn get_checkpoint(
        &self,
        project_id: ProjectId,
        checkpoint_id: CheckpointId,
    ) -> Result<Checkpoint, Error> {
        load_checkpoint(&self.connection, project_id, checkpoint_id)
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_context_item_revision(
    connection: &Connection,
    item_id: ContextItemId,
    project_id: ProjectId,
    revision: u64,
    draft: &ContextItemDraft,
    correction_kind: Option<CorrectionKind>,
    authorization_source_id: Option<SourceId>,
    recorded_at: TimestampMicros,
) -> Result<(), Error> {
    let item = ContextItem {
        id: item_id,
        project_id,
        revision,
        role: draft.role,
        statement: draft.statement.clone(),
        provenance_role: draft.provenance_role,
        author: draft.author.clone(),
        source_basis: draft.source_basis.clone(),
        applicability: draft.applicability.clone(),
        recorded_at,
    };
    insert_context_item_snapshot(
        connection,
        &item,
        correction_kind,
        authorization_source_id,
        recorded_at,
    )
}

fn insert_context_item_snapshot(
    connection: &Connection,
    item: &ContextItem,
    correction_kind: Option<CorrectionKind>,
    authorization_source_id: Option<SourceId>,
    recorded_at: TimestampMicros,
) -> Result<(), Error> {
    connection
        .execute(
            "INSERT INTO context_item_revisions(
             context_item_id, revision, project_id, role, statement, provenance_role,
             author_kind, author_identity, source_basis, applicability_paths,
             applicability_components, applicability_work_contexts, correction_kind,
             authorization_source_id, recorded_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                item.id.as_bytes().as_slice(),
                revision_i64(item.revision)?,
                item.project_id.as_bytes().as_slice(),
                item.role.as_str(),
                item.statement,
                item.provenance_role.as_str(),
                item.author.kind.as_str(),
                item.author.identity,
                encode_source_ids(&item.source_basis),
                encode_strings(&item.applicability.paths),
                encode_strings(&item.applicability.components),
                encode_strings(&item.applicability.work_contexts),
                correction_kind.map(CorrectionKind::as_str),
                authorization_source_id.map(|value| value.as_bytes().to_vec()),
                recorded_at.as_unix_micros(),
            ],
        )
        .map_err(write_error)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_decision_revision(
    connection: &Connection,
    decision_id: DecisionId,
    project_id: ProjectId,
    revision: u64,
    question_id: QuestionId,
    question_revision: u64,
    user_turn_source_id: SourceId,
    choice_kind: &str,
    choice_value: &str,
    user_rationale: Option<&str>,
    displayed_alternatives: &[QuestionAlternative],
    recommendation: &AgentRecommendation,
    applicability: &ApplicabilityScope,
    assumptions: &[String],
    revisit_triggers: &[String],
    correction_kind: Option<CorrectionKind>,
    authorization_source_id: Option<SourceId>,
    recorded_at: TimestampMicros,
) -> Result<(), Error> {
    connection
        .execute(
            "INSERT INTO decision_revisions(
             decision_id, revision, project_id, question_id, question_revision,
             user_turn_source_id, choice_kind, choice_value, user_rationale,
             displayed_alternatives, recommendation_key, recommendation_rationale,
             recommendation_sources, applicability_paths, applicability_components,
             applicability_work_contexts, assumptions, revisit_triggers, correction_kind,
             authorization_source_id, recorded_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                   ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                decision_id.as_bytes().as_slice(),
                revision_i64(revision)?,
                project_id.as_bytes().as_slice(),
                question_id.as_bytes().as_slice(),
                revision_i64(question_revision)?,
                user_turn_source_id.as_bytes().as_slice(),
                choice_kind,
                choice_value,
                user_rationale,
                encode_alternatives(displayed_alternatives),
                recommendation.alternative_key,
                recommendation.rationale,
                encode_source_ids(&recommendation.source_basis),
                encode_strings(&applicability.paths),
                encode_strings(&applicability.components),
                encode_strings(&applicability.work_contexts),
                encode_strings(assumptions),
                encode_strings(revisit_triggers),
                correction_kind.map(CorrectionKind::as_str),
                authorization_source_id.map(|value| value.as_bytes().to_vec()),
                recorded_at.as_unix_micros(),
            ],
        )
        .map_err(write_error)?;
    Ok(())
}

fn insert_decision_snapshot(
    connection: &Connection,
    decision: &Decision,
    correction_kind: Option<CorrectionKind>,
    authorization_source_id: Option<SourceId>,
    recorded_at: TimestampMicros,
) -> Result<(), Error> {
    let (choice_kind, choice_value) = decision_choice_parts(&decision.choice);
    insert_decision_revision(
        connection,
        decision.id,
        decision.project_id,
        decision.revision,
        decision.question_id,
        decision.question_revision,
        decision.user_turn_source_id,
        choice_kind,
        choice_value,
        decision.user_rationale.as_deref(),
        &decision.displayed_alternatives,
        &decision.displayed_recommendation,
        &decision.applicability,
        &decision.assumptions,
        &decision.revisit_triggers,
        correction_kind,
        authorization_source_id,
        recorded_at,
    )
}

fn decision_choice_parts(choice: &DecisionChoice) -> (&str, &str) {
    match choice {
        DecisionChoice::Alternative { alternative_key } => ("alternative", alternative_key),
        DecisionChoice::Delegation { delegate_to } => ("delegation", delegate_to),
    }
}

fn load_context_item_revision(
    connection: &Connection,
    project_id: ProjectId,
    item_id: ContextItemId,
    revision: u64,
) -> Result<ContextItem, Error> {
    type Row = (
        String,
        String,
        String,
        String,
        String,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        i64,
    );
    let row: Row = connection.query_row(
        "SELECT role, statement, provenance_role, author_kind, author_identity, source_basis,
                applicability_paths, applicability_components, applicability_work_contexts, recorded_at
         FROM context_item_revisions WHERE project_id = ?1 AND context_item_id = ?2 AND revision = ?3",
        params![project_id.as_bytes().as_slice(), item_id.as_bytes().as_slice(), revision_i64(revision)?],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?)),
    ).optional().map_err(read_error)?.ok_or_else(|| Error::new(ErrorKind::NotFound, "Context Item revision was not found"))?;
    Ok(ContextItem {
        id: item_id,
        project_id,
        revision,
        role: ContextItemRole::parse(&row.0).ok_or_else(|| invalid_stored("Context Item role"))?,
        statement: row.1,
        provenance_role: StatementProvenanceRole::parse(&row.2)
            .ok_or_else(|| invalid_stored("Context Item provenance role"))?,
        author: Principal {
            kind: PrincipalKind::parse(&row.3)
                .ok_or_else(|| invalid_stored("Context Item author kind"))?,
            identity: row.4,
        },
        source_basis: decode_source_ids(&row.5)?,
        applicability: ApplicabilityScope {
            paths: decode_strings(&row.6)?,
            components: decode_strings(&row.7)?,
            work_contexts: decode_strings(&row.8)?,
        },
        recorded_at: TimestampMicros::from_unix_micros(row.9),
    })
}

fn load_decision_revision(
    connection: &Connection,
    project_id: ProjectId,
    decision_id: DecisionId,
    revision: u64,
) -> Result<Decision, Error> {
    type Row = (
        Vec<u8>,
        i64,
        Vec<u8>,
        String,
        String,
        Option<String>,
        Vec<u8>,
        Option<String>,
        String,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        i64,
    );
    let row: Row = connection.query_row(
        "SELECT question_id, question_revision, user_turn_source_id, choice_kind, choice_value,
                user_rationale, displayed_alternatives, recommendation_key, recommendation_rationale,
                recommendation_sources, applicability_paths, applicability_components,
                applicability_work_contexts, assumptions, revisit_triggers, recorded_at
         FROM decision_revisions WHERE project_id = ?1 AND decision_id = ?2 AND revision = ?3",
        params![project_id.as_bytes().as_slice(), decision_id.as_bytes().as_slice(), revision_i64(revision)?],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?, row.get(11)?, row.get(12)?, row.get(13)?, row.get(14)?, row.get(15)?)),
    ).optional().map_err(read_error)?.ok_or_else(|| Error::new(ErrorKind::NotFound, "Decision revision was not found"))?;
    let choice = match row.3.as_str() {
        "alternative" => DecisionChoice::Alternative {
            alternative_key: row.4,
        },
        "delegation" => DecisionChoice::Delegation { delegate_to: row.4 },
        _ => return Err(invalid_stored("Decision choice")),
    };
    Ok(Decision {
        id: decision_id,
        project_id,
        revision,
        question_id: QuestionId::from_slice(&row.0)?,
        question_revision: stored_revision(row.1)?,
        user_turn_source_id: SourceId::from_slice(&row.2)?,
        choice,
        user_rationale: row.5,
        displayed_alternatives: decode_alternatives(&row.6)?,
        displayed_recommendation: AgentRecommendation {
            alternative_key: row.7,
            rationale: row.8,
            source_basis: decode_source_ids(&row.9)?,
        },
        applicability: ApplicabilityScope {
            paths: decode_strings(&row.10)?,
            components: decode_strings(&row.11)?,
            work_contexts: decode_strings(&row.12)?,
        },
        assumptions: decode_strings(&row.13)?,
        revisit_triggers: decode_strings(&row.14)?,
        recorded_at: TimestampMicros::from_unix_micros(row.15),
    })
}

fn ensure_meaning_preserving(before: &str, after: &str, kind: CorrectionKind) -> Result<(), Error> {
    if before == after {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "correction must change presentation",
        ));
    }
    let accepted = match kind {
        CorrectionKind::Formatting => compact_alphanumeric(before) == compact_alphanumeric(after),
        CorrectionKind::Typography => edit_distance_with_limit(before, after, 2),
        CorrectionKind::Expression => sorted_words(before) == sorted_words(after),
    };
    if !accepted {
        return Err(Error::new(
            ErrorKind::DomainConflict,
            "correction changes semantic tokens; create a superseding record instead",
        ));
    }
    Ok(())
}

fn compact_alphanumeric(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn sorted_words(value: &str) -> Vec<String> {
    let mut words: Vec<String> = value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect();
    words.sort();
    words
}

fn edit_distance_with_limit(left: &str, right: &str, limit: usize) -> bool {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    if left.len().abs_diff(right.len()) > limit {
        return false;
    }
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (left_index, left_character) in left.iter().enumerate() {
        let mut current = vec![left_index + 1];
        for (right_index, right_character) in right.iter().enumerate() {
            current.push(
                (previous[right_index + 1] + 1)
                    .min(current[right_index] + 1)
                    .min(previous[right_index] + usize::from(left_character != right_character)),
            );
        }
        previous = current;
    }
    previous[right.len()] <= limit
}

fn insert_canonical_relation(
    connection: &Connection,
    project_id: ProjectId,
    from: CanonicalRecordId,
    kind: CanonicalRelationKind,
    to: CanonicalRecordId,
    recorded_at: TimestampMicros,
) -> Result<(), Error> {
    connection.execute(
        "INSERT INTO canonical_relations(project_id, from_kind, from_id, relation_kind, to_kind, to_id, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![project_id.as_bytes().as_slice(), from.kind().as_str(), from.as_bytes().as_slice(),
            kind.as_str(), to.kind().as_str(), to.as_bytes().as_slice(), recorded_at.as_unix_micros()],
    ).map_err(|error| insert_identity_error(error, "canonical relation already exists"))?;
    Ok(())
}

fn load_canonical_relation(
    connection: &Connection,
    project_id: ProjectId,
    from: CanonicalRecordId,
    kind: CanonicalRelationKind,
    to: CanonicalRecordId,
) -> Result<CanonicalRelation, Error> {
    let recorded_at: i64 = connection.query_row(
        "SELECT recorded_at FROM canonical_relations WHERE project_id = ?1 AND from_kind = ?2 AND from_id = ?3 AND relation_kind = ?4 AND to_kind = ?5 AND to_id = ?6",
        params![project_id.as_bytes().as_slice(), from.kind().as_str(), from.as_bytes().as_slice(), kind.as_str(), to.kind().as_str(), to.as_bytes().as_slice()],
        |row| row.get(0),
    ).optional().map_err(read_error)?.ok_or_else(|| Error::new(ErrorKind::NotFound, "canonical relation was not found"))?;
    Ok(CanonicalRelation {
        project_id,
        from,
        kind,
        to,
        recorded_at: TimestampMicros::from_unix_micros(recorded_at),
    })
}

fn canonical_record_id(kind: &str, bytes: &[u8]) -> Result<CanonicalRecordId, Error> {
    match CanonicalRecordKind::parse(kind).ok_or_else(|| invalid_stored("canonical record kind"))? {
        CanonicalRecordKind::Project => {
            Ok(CanonicalRecordId::Project(ProjectId::from_slice(bytes)?))
        }
        CanonicalRecordKind::Source => Ok(CanonicalRecordId::Source(SourceId::from_slice(bytes)?)),
        CanonicalRecordKind::Question => {
            Ok(CanonicalRecordId::Question(QuestionId::from_slice(bytes)?))
        }
        CanonicalRecordKind::Decision => {
            Ok(CanonicalRecordId::Decision(DecisionId::from_slice(bytes)?))
        }
        CanonicalRecordKind::ContextItem => Ok(CanonicalRecordId::ContextItem(
            ContextItemId::from_slice(bytes)?,
        )),
        CanonicalRecordKind::Checkpoint => Ok(CanonicalRecordId::Checkpoint(
            CheckpointId::from_slice(bytes)?,
        )),
    }
}

fn superseded_by(
    connection: &Connection,
    project_id: ProjectId,
    decision_id: DecisionId,
) -> Result<Option<DecisionId>, Error> {
    let bytes: Option<Vec<u8>> = connection.query_row(
        "SELECT from_id FROM canonical_relations WHERE project_id = ?1 AND from_kind = 'decision' AND relation_kind = 'supersedes' AND to_kind = 'decision' AND to_id = ?2 ORDER BY from_id LIMIT 1",
        params![project_id.as_bytes().as_slice(), decision_id.as_bytes().as_slice()], |row| row.get(0),
    ).optional().map_err(read_error)?;
    bytes
        .map(|value| DecisionId::from_slice(&value))
        .transpose()
}

fn load_contradictions(
    connection: &Connection,
    project_id: ProjectId,
    record: CanonicalRecordId,
) -> Result<Vec<CanonicalRecordId>, Error> {
    let mut statement = connection.prepare(
        "SELECT to_kind, to_id FROM canonical_relations WHERE project_id = ?1 AND from_kind = ?2 AND from_id = ?3 AND relation_kind = 'contradicts'
         UNION SELECT from_kind, from_id FROM canonical_relations WHERE project_id = ?1 AND to_kind = ?2 AND to_id = ?3 AND relation_kind = 'contradicts'
         ORDER BY 1, 2",
    ).map_err(read_error)?;
    let rows = statement
        .query_map(
            params![
                project_id.as_bytes().as_slice(),
                record.kind().as_str(),
                record.as_bytes().as_slice()
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
        )
        .map_err(read_error)?;
    let mut values = Vec::new();
    for row in rows {
        let (kind, bytes) = row.map_err(read_error)?;
        values.push(canonical_record_id(&kind, &bytes)?);
    }
    Ok(values)
}

fn load_review_due(
    connection: &Connection,
    project_id: ProjectId,
    decision_id: DecisionId,
) -> Result<ReviewDue, Error> {
    load_review_due_optional(connection, project_id, decision_id)?.ok_or_else(|| {
        Error::new(
            ErrorKind::NotFound,
            "Decision review-due state was not found",
        )
    })
}

fn load_review_due_optional(
    connection: &Connection,
    project_id: ProjectId,
    decision_id: DecisionId,
) -> Result<Option<ReviewDue>, Error> {
    type Row = (String, String, Vec<u8>, i64);
    let row: Option<Row> = connection.query_row(
        "SELECT review_kind, explanation, source_basis, marked_at FROM review_due WHERE project_id = ?1 AND decision_id = ?2",
        params![project_id.as_bytes().as_slice(), decision_id.as_bytes().as_slice()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).optional().map_err(read_error)?;
    row.map(|row| {
        Ok(ReviewDue {
            project_id,
            decision_id,
            kind: ReviewDueKind::parse(&row.0).ok_or_else(|| invalid_stored("review-due kind"))?,
            explanation: row.1,
            source_basis: decode_source_ids(&row.2)?,
            marked_at: TimestampMicros::from_unix_micros(row.3),
        })
    })
    .transpose()
}

fn ensure_record_exists(
    connection: &Connection,
    project_id: ProjectId,
    record: CanonicalRecordId,
) -> Result<(), Error> {
    match record {
        CanonicalRecordId::Project(value) => load_project(connection, value).and_then(|project| {
            if project.id == project_id {
                Ok(())
            } else {
                Err(Error::new(
                    ErrorKind::WrongProject,
                    "Project identity differs from scope",
                ))
            }
        }),
        CanonicalRecordId::Source(value) => ensure_source_project(connection, value, project_id),
        CanonicalRecordId::Question(value) => {
            ensure_record_project(connection, "questions", value.as_bytes(), project_id)?;
            load_question(connection, project_id, value).map(|_| ())
        }
        CanonicalRecordId::Decision(value) => {
            ensure_record_project(connection, "decisions", value.as_bytes(), project_id)?;
            load_decision(connection, project_id, value).map(|_| ())
        }
        CanonicalRecordId::ContextItem(value) => {
            ensure_record_project(connection, "context_items", value.as_bytes(), project_id)?;
            load_context_item(connection, project_id, value).map(|_| ())
        }
        CanonicalRecordId::Checkpoint(value) => {
            ensure_record_project(connection, "checkpoints", value.as_bytes(), project_id)?;
            load_checkpoint(connection, project_id, value).map(|_| ())
        }
    }
}

fn ensure_record_project(
    connection: &Connection,
    table: &str,
    record_id: &[u8; 16],
    expected_project_id: ProjectId,
) -> Result<(), Error> {
    let sql = format!("SELECT project_id FROM {table} WHERE id = ?1");
    let project_bytes: Vec<u8> = connection
        .query_row(&sql, [record_id.as_slice()], |row| row.get(0))
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "canonical record was not found"))?;
    let actual_project_id = ProjectId::from_slice(&project_bytes)?;
    if actual_project_id != expected_project_id {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "canonical record belongs to a different Project",
        ));
    }
    Ok(())
}

fn ensure_record_has_source_basis(
    connection: &Connection,
    project_id: ProjectId,
    record: CanonicalRecordId,
) -> Result<(), Error> {
    let has_basis = match record {
        CanonicalRecordId::ContextItem(value) => !load_context_item(connection, project_id, value)?
            .source_basis
            .is_empty(),
        CanonicalRecordId::Decision(value) => {
            let decision = load_decision(connection, project_id, value)?;
            load_source(connection, decision.user_turn_source_id)?.project_id == project_id
        }
        _ => false,
    };
    if !has_basis {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "contradiction sides must preserve supporting Sources",
        ));
    }
    Ok(())
}

fn load_tombstone(
    connection: &Connection,
    project_id: ProjectId,
    record: CanonicalRecordId,
) -> Result<Tombstone, Error> {
    let forgotten_at: i64 = connection.query_row(
        "SELECT forgotten_at FROM tombstones WHERE project_id = ?1 AND record_kind = ?2 AND record_id = ?3",
        params![project_id.as_bytes().as_slice(), record.kind().as_str(), record.as_bytes().as_slice()], |row| row.get(0),
    ).optional().map_err(read_error)?.ok_or_else(|| Error::new(ErrorKind::NotFound, "tombstone was not found"))?;
    Ok(Tombstone {
        project_id,
        record,
        forgotten_at: TimestampMicros::from_unix_micros(forgotten_at),
    })
}

fn forget_result(tombstone: Tombstone) -> ForgetResult {
    ForgetResult {
        invalidation: CanonicalInvalidation {
            project_id: tombstone.project_id,
            record: tombstone.record,
        },
        tombstone,
    }
}

pub(crate) fn sanitize_forgotten_dependencies(
    connection: &Connection,
    project_id: ProjectId,
    record: CanonicalRecordId,
    forgotten_at: TimestampMicros,
) -> Result<(), Error> {
    connection
        .execute(
            "UPDATE operations
             SET input_basis = X'', replay_state = 'forgotten_dependency'
             WHERE project_id = ?1 AND operation_id IN (
                 SELECT operation_id FROM operation_dependencies
                 WHERE project_id = ?1 AND owner_kind = ?2 AND owner_id = ?3
             )",
            params![
                project_id.as_bytes().as_slice(),
                record.kind().as_str(),
                record.as_bytes().as_slice(),
            ],
        )
        .map_err(write_error)?;
    connection
        .execute(
            "DELETE FROM operation_dependencies
             WHERE operation_id IN (
                 SELECT operation_id FROM operations
                 WHERE project_id = ?1 AND replay_state = 'forgotten_dependency'
             )",
            [project_id.as_bytes().as_slice()],
        )
        .map_err(write_error)?;

    if let CanonicalRecordId::Question(question_id) = record {
        let empty_alternatives = encode_alternatives(&[]);
        let empty_sources = encode_source_ids(&[]);
        connection
            .execute(
                "UPDATE decisions
                 SET displayed_alternatives = ?3, recommendation_key = NULL,
                     recommendation_rationale = '', recommendation_sources = ?4
                 WHERE project_id = ?1 AND question_id = ?2",
                params![
                    project_id.as_bytes().as_slice(),
                    question_id.as_bytes().as_slice(),
                    empty_alternatives,
                    empty_sources,
                ],
            )
            .map_err(write_error)?;
        connection
            .execute(
                "UPDATE decision_revisions
                 SET displayed_alternatives = ?3, recommendation_key = NULL,
                     recommendation_rationale = '', recommendation_sources = ?4
                 WHERE project_id = ?1 AND question_id = ?2",
                params![
                    project_id.as_bytes().as_slice(),
                    question_id.as_bytes().as_slice(),
                    empty_alternatives,
                    empty_sources,
                ],
            )
            .map_err(write_error)?;
        let mut statement = connection
            .prepare(
                "SELECT id FROM decisions WHERE project_id = ?1 AND question_id = ?2 ORDER BY id",
            )
            .map_err(read_error)?;
        let rows = statement
            .query_map(
                params![
                    project_id.as_bytes().as_slice(),
                    question_id.as_bytes().as_slice()
                ],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .map_err(read_error)?;
        let mut decision_ids = Vec::new();
        for row in rows {
            decision_ids.push(DecisionId::from_slice(&row.map_err(read_error)?)?);
        }
        drop(statement);
        for decision_id in decision_ids {
            connection
                .execute(
                    "INSERT INTO review_due(
                         project_id, decision_id, review_kind, explanation, source_basis, marked_at
                     ) VALUES (?1, ?2, 'source_freshness_changed',
                         'Question presentation basis was forgotten', ?3, ?4)
                     ON CONFLICT(project_id, decision_id) DO UPDATE SET
                         review_kind = excluded.review_kind,
                         explanation = excluded.explanation,
                         source_basis = excluded.source_basis,
                         marked_at = excluded.marked_at",
                    params![
                        project_id.as_bytes().as_slice(),
                        decision_id.as_bytes().as_slice(),
                        encode_source_ids(&[]),
                        forgotten_at.as_unix_micros(),
                    ],
                )
                .map_err(write_error)?;
        }
    }
    Ok(())
}

pub(crate) fn sanitize_deleted_content(connection: &Connection) -> Result<(), Error> {
    let (busy, _, _): (i64, i64, i64) = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(write_error)?;
    if busy != 0 {
        return Err(Error::new(
            ErrorKind::RepairRequired,
            "forgotten content committed but WAL truncation is busy",
        ));
    }
    connection
        .execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(write_error)
}

fn refresh_bundles_after_forgetting(store: &Store, project_id: ProjectId) -> Result<(), Error> {
    crate::portable::refresh_managed_bundles(store, project_id).map_err(|error| {
        Error::with_source(
            ErrorKind::RepairRequired,
            "canonical forgetting committed, but a managed portable bundle requires refresh",
            error,
        )
    })
}

impl Store {
    pub fn correct_context_item(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        item_id: ContextItemId,
        draft: ContextItemCorrectionDraft,
    ) -> Result<OperationResult<ContextItem>, Error> {
        validate_nonempty(
            "corrected Context Item statement",
            &draft.corrected_statement,
        )?;
        let basis = Basis::new("correct_context_item")
            .bytes(project_id.as_bytes())
            .bytes(item_id.as_bytes())
            .number(draft.expected_revision)
            .string(&draft.corrected_statement)
            .string(draft.kind.as_str())
            .bytes(draft.user_authorization_source_id.as_bytes())
            .finish();
        let (connection, clock) = (&mut self.connection, &mut self.clock);
        let transaction = begin_write(connection)?;
        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "correct_context_item", &basis)?;
            let value = load_context_item_revision(
                &transaction,
                project_id,
                item_id,
                operation.result_revision,
            )?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value,
                replayed: true,
            });
        }
        let current = load_context_item(&transaction, project_id, item_id)?;
        ensure_revision(draft.expected_revision, current.revision, "Context Item")?;
        ensure_user_turn_source(
            &load_source(&transaction, draft.user_authorization_source_id)?,
            project_id,
        )?;
        ensure_meaning_preserving(&current.statement, &draft.corrected_statement, draft.kind)?;
        let revision = current.revision.checked_add(1).ok_or_else(|| {
            Error::new(
                ErrorKind::RepairRequired,
                "Context Item revision is exhausted",
            )
        })?;
        let now = clock.now()?;
        transaction
            .execute(
                "UPDATE context_items SET statement = ?2, revision = ?3
                 WHERE id = ?1 AND project_id = ?4 AND revision = ?5",
                params![
                    item_id.as_bytes().as_slice(),
                    draft.corrected_statement,
                    revision_i64(revision)?,
                    project_id.as_bytes().as_slice(),
                    revision_i64(draft.expected_revision)?,
                ],
            )
            .map_err(write_error)
            .and_then(|count| ensure_single_updated(count, "Context Item changed concurrently"))?;
        let corrected = ContextItem {
            revision,
            statement: draft.corrected_statement,
            ..current
        };
        insert_context_item_snapshot(
            &transaction,
            &corrected,
            Some(draft.kind),
            Some(draft.user_authorization_source_id),
            now,
        )?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "correct_context_item",
            &basis,
            "context_item",
            item_id.as_bytes(),
            revision,
            now,
            &[
                CanonicalRecordId::ContextItem(item_id),
                CanonicalRecordId::Source(draft.user_authorization_source_id),
            ],
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: corrected,
            replayed: false,
        })
    }

    pub fn correct_decision(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        decision_id: DecisionId,
        draft: DecisionCorrectionDraft,
    ) -> Result<OperationResult<Decision>, Error> {
        validate_optional_nonempty(
            "corrected user rationale",
            draft.corrected_user_rationale.as_deref(),
        )?;
        let basis = Basis::new("correct_decision")
            .bytes(project_id.as_bytes())
            .bytes(decision_id.as_bytes())
            .number(draft.expected_revision)
            .optional_string(draft.corrected_user_rationale.as_deref())
            .string(draft.kind.as_str())
            .bytes(draft.user_authorization_source_id.as_bytes())
            .finish();
        let (connection, clock) = (&mut self.connection, &mut self.clock);
        let transaction = begin_write(connection)?;
        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "correct_decision", &basis)?;
            let value = load_decision_revision(
                &transaction,
                project_id,
                decision_id,
                operation.result_revision,
            )?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value,
                replayed: true,
            });
        }
        let current = load_decision(&transaction, project_id, decision_id)?;
        ensure_revision(draft.expected_revision, current.revision, "Decision")?;
        ensure_user_turn_source(
            &load_source(&transaction, draft.user_authorization_source_id)?,
            project_id,
        )?;
        match (&current.user_rationale, &draft.corrected_user_rationale) {
            (Some(before), Some(after)) => ensure_meaning_preserving(before, after, draft.kind)?,
            (None, None) => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Decision correction must change the non-semantic rationale presentation",
                ));
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::DomainConflict,
                    "adding or removing Decision rationale is semantic; supersede the Decision",
                ));
            }
        }
        let revision = current.revision.checked_add(1).ok_or_else(|| {
            Error::new(ErrorKind::RepairRequired, "Decision revision is exhausted")
        })?;
        let now = clock.now()?;
        transaction
            .execute(
                "UPDATE decisions SET user_rationale = ?2, revision = ?3
                 WHERE id = ?1 AND project_id = ?4 AND revision = ?5",
                params![
                    decision_id.as_bytes().as_slice(),
                    draft.corrected_user_rationale,
                    revision_i64(revision)?,
                    project_id.as_bytes().as_slice(),
                    revision_i64(draft.expected_revision)?,
                ],
            )
            .map_err(write_error)
            .and_then(|count| ensure_single_updated(count, "Decision changed concurrently"))?;
        let corrected = Decision {
            revision,
            user_rationale: draft.corrected_user_rationale,
            ..current
        };
        insert_decision_snapshot(
            &transaction,
            &corrected,
            Some(draft.kind),
            Some(draft.user_authorization_source_id),
            now,
        )?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "correct_decision",
            &basis,
            "decision",
            decision_id.as_bytes(),
            revision,
            now,
            &[
                CanonicalRecordId::Decision(decision_id),
                CanonicalRecordId::Source(draft.user_authorization_source_id),
            ],
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: corrected,
            replayed: false,
        })
    }

    pub fn supersede_decision(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        draft: DecisionSupersessionDraft,
    ) -> Result<OperationResult<Decision>, Error> {
        validate_string_list("Decision assumptions", &draft.assumptions)?;
        validate_string_list("Decision revisit triggers", &draft.revisit_triggers)?;
        validate_portable_paths("Decision applicability path", &draft.applicability.paths)?;
        validate_optional_nonempty("user rationale", draft.user_rationale.as_deref())?;
        let choice_basis = match &draft.choice {
            DecisionChoice::Alternative { alternative_key } => {
                validate_nonempty("Decision alternative", alternative_key)?;
                format!("alternative:{alternative_key}")
            }
            DecisionChoice::Delegation { delegate_to } => {
                validate_nonempty("Decision delegate", delegate_to)?;
                format!("delegation:{delegate_to}")
            }
        };
        let source_basis = match &draft.user_turn_source {
            UserTurnSource::Existing(value) => {
                Basis::new("existing").bytes(value.as_bytes()).finish()
            }
            UserTurnSource::Create(value) => {
                let encoded = EncodedSource::from_payload(&value.payload);
                source_basis(project_id, value, &encoded)
            }
        };
        let basis = Basis::new("supersede_decision")
            .bytes(project_id.as_bytes())
            .number(draft.expected_project_revision)
            .bytes(draft.previous_decision_id.as_bytes())
            .bytes(&source_basis)
            .string(&choice_basis)
            .optional_string(draft.user_rationale.as_deref())
            .bytes(&encode_strings(&draft.applicability.paths))
            .bytes(&encode_strings(&draft.applicability.components))
            .bytes(&encode_strings(&draft.applicability.work_contexts))
            .bytes(&encode_strings(&draft.assumptions))
            .bytes(&encode_strings(&draft.revisit_triggers))
            .finish();
        let (connection, ids, clock) = (&mut self.connection, &mut self.ids, &mut self.clock);
        let transaction = begin_write(connection)?;
        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "supersede_decision", &basis)?;
            let value = load_decision(
                &transaction,
                project_id,
                DecisionId::from_slice(&operation.result_id)?,
            )?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value,
                replayed: true,
            });
        }
        let project = load_project(&transaction, project_id)?;
        ensure_revision(draft.expected_project_revision, project.revision, "Project")?;
        let previous = load_decision(&transaction, project_id, draft.previous_decision_id)?;
        if superseded_by(&transaction, project_id, draft.previous_decision_id)?.is_some() {
            return Err(Error::new(
                ErrorKind::DomainConflict,
                "only the current Decision can be superseded",
            ));
        }
        if let DecisionChoice::Alternative { alternative_key } = &draft.choice {
            if !previous
                .displayed_alternatives
                .iter()
                .any(|alternative| alternative.key == *alternative_key)
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "superseding Decision alternative was not displayed for the Question",
                ));
            }
        }
        let now = clock.now()?;
        let user_turn_source = match &draft.user_turn_source {
            UserTurnSource::Existing(source_id) => {
                let source = load_source(&transaction, *source_id)?;
                ensure_user_turn_source(&source, project_id)?;
                source
            }
            UserTurnSource::Create(source_draft) => {
                ensure_revision(
                    source_draft.expected_project_revision,
                    project.revision,
                    "Project",
                )?;
                validate_source_draft(source_draft)?;
                ensure_user_turn_draft(source_draft)?;
                let source_id = SourceId::from_bytes(ids.next_id()?);
                insert_source(&transaction, source_id, project_id, source_draft, now)?;
                Source {
                    id: source_id,
                    project_id,
                    payload: source_draft.payload.clone(),
                    actor: source_draft.actor.clone(),
                    observer: source_draft.observer.clone(),
                    availability: source_draft.availability,
                    recorded_at: now,
                }
            }
        };
        let decision_id = DecisionId::from_bytes(ids.next_id()?);
        let (choice_kind, choice_value) = decision_choice_parts(&draft.choice);
        transaction
            .execute(
                "INSERT INTO decisions(
                     id, project_id, revision, question_id, question_revision, user_turn_source_id,
                     choice_kind, choice_value, user_rationale, displayed_alternatives,
                     recommendation_key, recommendation_rationale, recommendation_sources,
                     applicability_paths, applicability_components, applicability_work_contexts,
                     assumptions, revisit_triggers, recorded_at
                 ) VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                           ?13, ?14, ?15, ?16, ?17, ?18)",
                params![
                    decision_id.as_bytes().as_slice(),
                    project_id.as_bytes().as_slice(),
                    previous.question_id.as_bytes().as_slice(),
                    revision_i64(previous.question_revision)?,
                    user_turn_source.id.as_bytes().as_slice(),
                    choice_kind,
                    choice_value,
                    draft.user_rationale,
                    encode_alternatives(&previous.displayed_alternatives),
                    previous.displayed_recommendation.alternative_key,
                    previous.displayed_recommendation.rationale,
                    encode_source_ids(&previous.displayed_recommendation.source_basis),
                    encode_strings(&draft.applicability.paths),
                    encode_strings(&draft.applicability.components),
                    encode_strings(&draft.applicability.work_contexts),
                    encode_strings(&draft.assumptions),
                    encode_strings(&draft.revisit_triggers),
                    now.as_unix_micros(),
                ],
            )
            .map_err(|error| insert_identity_error(error, "Decision identity already exists"))?;
        let decision = Decision {
            id: decision_id,
            project_id,
            revision: 1,
            question_id: previous.question_id,
            question_revision: previous.question_revision,
            user_turn_source_id: user_turn_source.id,
            choice: draft.choice,
            user_rationale: draft.user_rationale,
            displayed_alternatives: previous.displayed_alternatives,
            displayed_recommendation: previous.displayed_recommendation,
            applicability: draft.applicability,
            assumptions: draft.assumptions,
            revisit_triggers: draft.revisit_triggers,
            recorded_at: now,
        };
        insert_decision_snapshot(&transaction, &decision, None, None, now)?;
        insert_canonical_relation(
            &transaction,
            project_id,
            CanonicalRecordId::Decision(decision_id),
            CanonicalRelationKind::Supersedes,
            CanonicalRecordId::Decision(draft.previous_decision_id),
            now,
        )?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "supersede_decision",
            &basis,
            "decision",
            decision_id.as_bytes(),
            1,
            now,
            &[
                CanonicalRecordId::Decision(decision_id),
                CanonicalRecordId::Decision(draft.previous_decision_id),
                CanonicalRecordId::Source(user_turn_source.id),
                CanonicalRecordId::Question(previous.question_id),
            ],
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: decision,
            replayed: false,
        })
    }

    pub fn record_contradiction(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        left: CanonicalRecordId,
        right: CanonicalRecordId,
    ) -> Result<OperationResult<CanonicalRelation>, Error> {
        if left == right {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "a record cannot contradict itself",
            ));
        }
        let basis = Basis::new("record_contradiction")
            .bytes(project_id.as_bytes())
            .string(left.kind().as_str())
            .bytes(&left.as_bytes())
            .string(right.kind().as_str())
            .bytes(&right.as_bytes())
            .finish();
        let (connection, clock) = (&mut self.connection, &mut self.clock);
        let transaction = begin_write(connection)?;
        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "record_contradiction", &basis)?;
            let value = load_canonical_relation(
                &transaction,
                project_id,
                left,
                CanonicalRelationKind::Contradicts,
                right,
            )?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value,
                replayed: true,
            });
        }
        ensure_record_exists(&transaction, project_id, left)?;
        ensure_record_exists(&transaction, project_id, right)?;
        ensure_record_has_source_basis(&transaction, project_id, left)?;
        ensure_record_has_source_basis(&transaction, project_id, right)?;
        let now = clock.now()?;
        insert_canonical_relation(
            &transaction,
            project_id,
            left,
            CanonicalRelationKind::Contradicts,
            right,
            now,
        )?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "record_contradiction",
            &basis,
            "canonical_relation",
            &left.as_bytes(),
            0,
            now,
            &[left, right],
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: CanonicalRelation {
                project_id,
                from: left,
                kind: CanonicalRelationKind::Contradicts,
                to: right,
                recorded_at: now,
            },
            replayed: false,
        })
    }

    pub fn mark_decision_review_due(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        decision_id: DecisionId,
        draft: ReviewDueDraft,
    ) -> Result<OperationResult<ReviewDue>, Error> {
        validate_nonempty("review-due explanation", &draft.explanation)?;
        ensure_unique_ids("review-due Source", &draft.source_basis)?;
        let basis = Basis::new("mark_decision_review_due")
            .bytes(project_id.as_bytes())
            .bytes(decision_id.as_bytes())
            .string(draft.kind.as_str())
            .string(&draft.explanation)
            .bytes(&encode_source_ids(&draft.source_basis))
            .finish();
        let (connection, clock) = (&mut self.connection, &mut self.clock);
        let transaction = begin_write(connection)?;
        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "mark_decision_review_due", &basis)?;
            let value = load_review_due(&transaction, project_id, decision_id)?;
            transaction.commit().map_err(commit_error)?;
            return Ok(OperationResult {
                value,
                replayed: true,
            });
        }
        load_decision(&transaction, project_id, decision_id)?;
        for source_id in &draft.source_basis {
            ensure_source_project(&transaction, *source_id, project_id)?;
        }
        let now = clock.now()?;
        transaction.execute(
            "INSERT INTO review_due(project_id, decision_id, review_kind, explanation, source_basis, marked_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![project_id.as_bytes().as_slice(), decision_id.as_bytes().as_slice(),
                draft.kind.as_str(), draft.explanation, encode_source_ids(&draft.source_basis), now.as_unix_micros()],
        ).map_err(|error| insert_identity_error(error, "Decision is already marked review due"))?;
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "mark_decision_review_due",
            &basis,
            "review_due",
            decision_id.as_bytes(),
            0,
            now,
            &[CanonicalRecordId::Decision(decision_id)],
        )?;
        transaction.commit().map_err(commit_error)?;
        Ok(OperationResult {
            value: ReviewDue {
                project_id,
                decision_id,
                kind: draft.kind,
                explanation: draft.explanation,
                source_basis: draft.source_basis,
                marked_at: now,
            },
            replayed: false,
        })
    }

    pub fn get_decision_history(
        &self,
        project_id: ProjectId,
        question_id: QuestionId,
    ) -> Result<Vec<Decision>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT id FROM decisions WHERE project_id = ?1 AND question_id = ?2 ORDER BY recorded_at, id",
        ).map_err(read_error)?;
        let rows = statement
            .query_map(
                params![
                    project_id.as_bytes().as_slice(),
                    question_id.as_bytes().as_slice()
                ],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .map_err(read_error)?;
        let mut values = Vec::new();
        for row in rows {
            values.push(load_decision(
                &self.connection,
                project_id,
                DecisionId::from_slice(&row.map_err(read_error)?)?,
            )?);
        }
        Ok(values)
    }

    pub fn get_current_decision(
        &self,
        project_id: ProjectId,
        question_id: QuestionId,
    ) -> Result<DecisionLifecycle, Error> {
        let history = self.get_decision_history(project_id, question_id)?;
        let mut current = Vec::new();
        for decision in history {
            if superseded_by(&self.connection, project_id, decision.id)?.is_none() {
                current.push(decision);
            }
        }
        if current.len() != 1 {
            return Err(Error::new(
                ErrorKind::DomainConflict,
                "Question does not have exactly one deterministic current Decision",
            ));
        }
        self.get_decision_lifecycle(project_id, current.remove(0).id)
    }

    pub fn get_decision_lifecycle(
        &self,
        project_id: ProjectId,
        decision_id: DecisionId,
    ) -> Result<DecisionLifecycle, Error> {
        let decision = load_decision(&self.connection, project_id, decision_id)?;
        let superseded_by = superseded_by(&self.connection, project_id, decision_id)?;
        let contradictions = load_contradictions(
            &self.connection,
            project_id,
            CanonicalRecordId::Decision(decision_id),
        )?;
        let review_due = load_review_due_optional(&self.connection, project_id, decision_id)?;
        Ok(DecisionLifecycle {
            decision,
            superseded_by,
            contradictions,
            review_due,
        })
    }

    pub fn get_canonical_relation(
        &self,
        project_id: ProjectId,
        from: CanonicalRecordId,
        kind: CanonicalRelationKind,
        to: CanonicalRecordId,
    ) -> Result<CanonicalRelation, Error> {
        load_canonical_relation(&self.connection, project_id, from, kind, to)
    }

    pub fn get_tombstone(
        &self,
        project_id: ProjectId,
        record: CanonicalRecordId,
    ) -> Result<Tombstone, Error> {
        load_tombstone(&self.connection, project_id, record)
    }

    pub fn get_context_item_history(
        &self,
        project_id: ProjectId,
        item_id: ContextItemId,
    ) -> Result<Vec<ContextItem>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT revision FROM context_item_revisions WHERE project_id = ?1 AND context_item_id = ?2 ORDER BY revision",
        ).map_err(read_error)?;
        let rows = statement
            .query_map(
                params![
                    project_id.as_bytes().as_slice(),
                    item_id.as_bytes().as_slice()
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(read_error)?;
        let mut values = Vec::new();
        for row in rows {
            values.push(load_context_item_revision(
                &self.connection,
                project_id,
                item_id,
                stored_revision(row.map_err(read_error)?)?,
            )?);
        }
        Ok(values)
    }

    pub fn forget_context_item(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        item_id: ContextItemId,
        user_authorization_source_id: SourceId,
    ) -> Result<OperationResult<ForgetResult>, Error> {
        self.forget_record(
            operation_id,
            project_id,
            CanonicalRecordId::ContextItem(item_id),
            user_authorization_source_id,
        )
    }

    pub fn forget_source(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        source_id: SourceId,
        user_authorization_source_id: SourceId,
    ) -> Result<OperationResult<ForgetResult>, Error> {
        self.forget_record(
            operation_id,
            project_id,
            CanonicalRecordId::Source(source_id),
            user_authorization_source_id,
        )
    }

    pub fn forget_question(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        question_id: QuestionId,
        user_authorization_source_id: SourceId,
    ) -> Result<OperationResult<ForgetResult>, Error> {
        self.forget_record(
            operation_id,
            project_id,
            CanonicalRecordId::Question(question_id),
            user_authorization_source_id,
        )
    }

    pub fn forget_decision(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        decision_id: DecisionId,
        user_authorization_source_id: SourceId,
    ) -> Result<OperationResult<ForgetResult>, Error> {
        self.forget_record(
            operation_id,
            project_id,
            CanonicalRecordId::Decision(decision_id),
            user_authorization_source_id,
        )
    }

    pub fn forget_checkpoint(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        checkpoint_id: CheckpointId,
        user_authorization_source_id: SourceId,
    ) -> Result<OperationResult<ForgetResult>, Error> {
        self.forget_record(
            operation_id,
            project_id,
            CanonicalRecordId::Checkpoint(checkpoint_id),
            user_authorization_source_id,
        )
    }

    fn forget_record(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        record: CanonicalRecordId,
        user_authorization_source_id: SourceId,
    ) -> Result<OperationResult<ForgetResult>, Error> {
        let basis = Basis::new("forget_canonical_record")
            .bytes(project_id.as_bytes())
            .string(record.kind().as_str())
            .bytes(&record.as_bytes())
            .bytes(user_authorization_source_id.as_bytes())
            .finish();
        let (connection, clock) = (&mut self.connection, &mut self.clock);
        let transaction = begin_write(connection)?;
        if let Some(operation) = load_operation(&transaction, operation_id)? {
            ensure_replay_input(&operation, "forget_canonical_record", &basis)?;
            let tombstone = load_tombstone(&transaction, project_id, record)?;
            transaction.commit().map_err(commit_error)?;
            sanitize_deleted_content(connection)?;
            refresh_bundles_after_forgetting(self, project_id)?;
            return Ok(OperationResult {
                value: forget_result(tombstone),
                replayed: true,
            });
        }
        ensure_record_exists(&transaction, project_id, record)?;
        ensure_user_turn_source(
            &load_source(&transaction, user_authorization_source_id)?,
            project_id,
        )?;
        let now = clock.now()?;
        transaction.execute(
            "INSERT INTO tombstones(project_id, record_kind, record_id, forgotten_at) VALUES (?1, ?2, ?3, ?4)",
            params![project_id.as_bytes().as_slice(), record.kind().as_str(), record.as_bytes().as_slice(), now.as_unix_micros()],
        ).map_err(write_error)?;
        sanitize_forgotten_dependencies(&transaction, project_id, record, now)?;
        match record {
            CanonicalRecordId::Source(source_id) => {
                transaction.execute(
                    "DELETE FROM source_relations WHERE project_id = ?1 AND (from_source_id = ?2 OR to_source_id = ?2)",
                    params![project_id.as_bytes().as_slice(), source_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction.execute(
                    "DELETE FROM question_response_sources WHERE project_id = ?1 AND source_id = ?2",
                    params![project_id.as_bytes().as_slice(), source_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction
                    .execute(
                        "DELETE FROM context_item_sources WHERE project_id = ?1 AND source_id = ?2",
                        params![
                            project_id.as_bytes().as_slice(),
                            source_id.as_bytes().as_slice()
                        ],
                    )
                    .map_err(write_error)?;
                transaction.execute(
                    "DELETE FROM checkpoint_source_relations WHERE project_id = ?1 AND source_id = ?2",
                    params![project_id.as_bytes().as_slice(), source_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction.execute(
                    "UPDATE checkpoint_verifications SET source_id = NULL WHERE project_id = ?1 AND source_id = ?2",
                    params![project_id.as_bytes().as_slice(), source_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction.execute(
                    "UPDATE checkpoints SET user_review_source_id = NULL WHERE project_id = ?1 AND user_review_source_id = ?2",
                    params![project_id.as_bytes().as_slice(), source_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction.execute(
                    "UPDATE checkpoints SET user_acceptance_source_id = NULL WHERE project_id = ?1 AND user_acceptance_source_id = ?2",
                    params![project_id.as_bytes().as_slice(), source_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction
                    .execute(
                        "DELETE FROM sources WHERE project_id = ?1 AND id = ?2",
                        params![
                            project_id.as_bytes().as_slice(),
                            source_id.as_bytes().as_slice()
                        ],
                    )
                    .map_err(write_error)
                    .and_then(|count| {
                        ensure_single_updated(count, "Source changed concurrently")
                    })?;
            }
            CanonicalRecordId::Question(question_id) => {
                remove_question_dependencies(&transaction, project_id, question_id)?;
                transaction.execute(
                    "DELETE FROM checkpoint_questions WHERE project_id = ?1 AND question_id = ?2",
                    params![project_id.as_bytes().as_slice(), question_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction.execute(
                    "DELETE FROM question_response_sources WHERE project_id = ?1 AND question_id = ?2",
                    params![project_id.as_bytes().as_slice(), question_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction
                    .execute(
                        "DELETE FROM question_revisions WHERE project_id = ?1 AND question_id = ?2",
                        params![
                            project_id.as_bytes().as_slice(),
                            question_id.as_bytes().as_slice()
                        ],
                    )
                    .map_err(write_error)?;
                transaction
                    .execute(
                        "DELETE FROM questions WHERE project_id = ?1 AND id = ?2",
                        params![
                            project_id.as_bytes().as_slice(),
                            question_id.as_bytes().as_slice()
                        ],
                    )
                    .map_err(write_error)
                    .and_then(|count| {
                        ensure_single_updated(count, "Question changed concurrently")
                    })?;
            }
            CanonicalRecordId::ContextItem(item_id) => {
                transaction.execute("DELETE FROM context_item_sources WHERE project_id = ?1 AND context_item_id = ?2", params![project_id.as_bytes().as_slice(), item_id.as_bytes().as_slice()]).map_err(write_error)?;
                transaction.execute("DELETE FROM context_item_revisions WHERE project_id = ?1 AND context_item_id = ?2", params![project_id.as_bytes().as_slice(), item_id.as_bytes().as_slice()]).map_err(write_error)?;
                transaction
                    .execute(
                        "DELETE FROM context_items WHERE project_id = ?1 AND id = ?2",
                        params![
                            project_id.as_bytes().as_slice(),
                            item_id.as_bytes().as_slice()
                        ],
                    )
                    .map_err(write_error)
                    .and_then(|count| {
                        ensure_single_updated(count, "Context Item changed concurrently")
                    })?;
            }
            CanonicalRecordId::Decision(decision_id) => {
                transaction.execute("DELETE FROM checkpoint_decisions WHERE project_id = ?1 AND decision_id = ?2", params![project_id.as_bytes().as_slice(), decision_id.as_bytes().as_slice()]).map_err(write_error)?;
                transaction
                    .execute(
                        "DELETE FROM review_due WHERE project_id = ?1 AND decision_id = ?2",
                        params![
                            project_id.as_bytes().as_slice(),
                            decision_id.as_bytes().as_slice()
                        ],
                    )
                    .map_err(write_error)?;
                transaction
                    .execute(
                        "DELETE FROM decision_revisions WHERE project_id = ?1 AND decision_id = ?2",
                        params![
                            project_id.as_bytes().as_slice(),
                            decision_id.as_bytes().as_slice()
                        ],
                    )
                    .map_err(write_error)?;
                transaction
                    .execute(
                        "DELETE FROM decisions WHERE project_id = ?1 AND id = ?2",
                        params![
                            project_id.as_bytes().as_slice(),
                            decision_id.as_bytes().as_slice()
                        ],
                    )
                    .map_err(write_error)
                    .and_then(|count| {
                        ensure_single_updated(count, "Decision changed concurrently")
                    })?;
            }
            CanonicalRecordId::Checkpoint(checkpoint_id) => {
                transaction.execute(
                    "DELETE FROM checkpoint_verifications WHERE project_id = ?1 AND checkpoint_id = ?2",
                    params![project_id.as_bytes().as_slice(), checkpoint_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction.execute(
                    "DELETE FROM checkpoint_questions WHERE project_id = ?1 AND checkpoint_id = ?2",
                    params![project_id.as_bytes().as_slice(), checkpoint_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction.execute(
                    "DELETE FROM checkpoint_decisions WHERE project_id = ?1 AND checkpoint_id = ?2",
                    params![project_id.as_bytes().as_slice(), checkpoint_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction.execute(
                    "DELETE FROM checkpoint_source_relations WHERE project_id = ?1 AND checkpoint_id = ?2",
                    params![project_id.as_bytes().as_slice(), checkpoint_id.as_bytes().as_slice()],
                ).map_err(write_error)?;
                transaction
                    .execute(
                        "DELETE FROM checkpoints WHERE project_id = ?1 AND id = ?2",
                        params![
                            project_id.as_bytes().as_slice(),
                            checkpoint_id.as_bytes().as_slice()
                        ],
                    )
                    .map_err(write_error)
                    .and_then(|count| {
                        ensure_single_updated(count, "Checkpoint changed concurrently")
                    })?;
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "forgetting this canonical record kind is not implemented",
                ))
            }
        }
        record_operation(
            &transaction,
            operation_id,
            project_id,
            "forget_canonical_record",
            &basis,
            "tombstone",
            &record.as_bytes(),
            0,
            now,
            &[CanonicalRecordId::Source(user_authorization_source_id)],
        )?;
        transaction.commit().map_err(commit_error)?;
        sanitize_deleted_content(connection)?;
        refresh_bundles_after_forgetting(self, project_id)?;
        let tombstone = Tombstone {
            project_id,
            record,
            forgotten_at: now,
        };
        Ok(OperationResult {
            value: forget_result(tombstone),
            replayed: false,
        })
    }
}

fn validate_context_item_draft(draft: &ContextItemDraft) -> Result<(), Error> {
    validate_nonempty("Context Item statement", &draft.statement)?;
    validate_nonempty("Context Item author identity", &draft.author.identity)?;
    if draft.source_basis.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Context Item requires an explicit Source basis",
        ));
    }
    ensure_unique_ids("Context Item Source basis", &draft.source_basis)?;
    validate_string_list(
        "Context Item applicability path",
        &draft.applicability.paths,
    )?;
    validate_portable_paths(
        "Context Item applicability path",
        &draft.applicability.paths,
    )?;
    validate_string_list(
        "Context Item applicability component",
        &draft.applicability.components,
    )?;
    validate_string_list(
        "Context Item applicability work context",
        &draft.applicability.work_contexts,
    )?;
    Ok(())
}

fn load_context_sources(
    connection: &Connection,
    project_id: ProjectId,
    source_ids: &[SourceId],
) -> Result<Vec<Source>, Error> {
    let mut sources = Vec::with_capacity(source_ids.len());
    for source_id in source_ids {
        let source = load_source(connection, *source_id)?;
        if source.project_id != project_id {
            return Err(Error::new(
                ErrorKind::WrongProject,
                "Context Item Source belongs to a different Project",
            ));
        }
        sources.push(source);
    }
    Ok(sources)
}

fn validate_context_provenance(draft: &ContextItemDraft, sources: &[Source]) -> Result<(), Error> {
    let has_user_turn = sources.iter().any(|source| {
        source.actor.kind == PrincipalKind::User
            && matches!(source.payload, SourcePayload::CurrentHostUserTurn { .. })
    });
    let has_observation = sources.iter().any(|source| {
        matches!(
            source.actor.kind,
            PrincipalKind::Repository | PrincipalKind::Command
        ) || matches!(
            source.payload,
            SourcePayload::RepositorySnapshot { .. }
                | SourcePayload::RepositoryCommit { .. }
                | SourcePayload::File { .. }
                | SourcePayload::Symbol { .. }
                | SourcePayload::CommandExecution { .. }
        )
    });
    let has_generated = sources.iter().any(|source| {
        matches!(
            source.actor.kind,
            PrincipalKind::Agent | PrincipalKind::Provider | PrincipalKind::Generator
        )
    });
    match draft.provenance_role {
        StatementProvenanceRole::UserStatement => {
            if draft.author.kind != PrincipalKind::User || !has_user_turn {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "user-authored Context Item requires user provenance and a current-host user-turn Source",
                ));
            }
        }
        StatementProvenanceRole::Observed => {
            if !has_observation
                || matches!(
                    draft.author.kind,
                    PrincipalKind::Provider | PrincipalKind::Generator
                )
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "observed Context Item requires repository or command observation provenance",
                ));
            }
        }
        StatementProvenanceRole::AgentStatement => {
            if draft.author.kind != PrincipalKind::Agent {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "agent-authored Context Item requires an agent author",
                ));
            }
        }
        StatementProvenanceRole::GeneratedInterpretation => {
            if !has_generated
                || !matches!(
                    draft.author.kind,
                    PrincipalKind::Agent | PrincipalKind::Provider | PrincipalKind::Generator
                )
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "generated interpretation requires agent, provider, or generator provenance",
                ));
            }
        }
    }
    if draft.role == ContextItemRole::Fact
        && draft.provenance_role != StatementProvenanceRole::Observed
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "only observed provenance may be recorded with the fact role",
        ));
    }
    if draft.role == ContextItemRole::Preference
        && (draft.provenance_role != StatementProvenanceRole::UserStatement || !has_user_turn)
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "explicit preference requires a current-host user-turn Source",
        ));
    }
    Ok(())
}

fn context_item_basis(project_id: ProjectId, draft: &ContextItemDraft) -> Vec<u8> {
    Basis::new("record_context_item")
        .bytes(project_id.as_bytes())
        .number(draft.expected_project_revision)
        .string(draft.role.as_str())
        .string(&draft.statement)
        .string(draft.provenance_role.as_str())
        .string(draft.author.kind.as_str())
        .string(&draft.author.identity)
        .bytes(&encode_source_ids(&draft.source_basis))
        .bytes(&encode_strings(&draft.applicability.paths))
        .bytes(&encode_strings(&draft.applicability.components))
        .bytes(&encode_strings(&draft.applicability.work_contexts))
        .finish()
}

fn load_context_item(
    connection: &Connection,
    project_id: ProjectId,
    item_id: ContextItemId,
) -> Result<ContextItem, Error> {
    type ContextItemRow = (
        Vec<u8>,
        i64,
        String,
        String,
        String,
        String,
        String,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        i64,
    );
    let row: ContextItemRow = connection
        .query_row(
            "SELECT project_id, revision, role, statement, provenance_role, author_kind,
                    author_identity, applicability_paths, applicability_components,
                    applicability_work_contexts, recorded_at
             FROM context_items WHERE id = ?1",
            [item_id.as_bytes().as_slice()],
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
                ))
            },
        )
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Context Item was not found"))?;
    let owner = ProjectId::from_slice(&row.0)?;
    if owner != project_id {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "Context Item belongs to a different Project",
        ));
    }
    let mut statement = connection
        .prepare(
            "SELECT source_id FROM context_item_sources
             WHERE context_item_id = ?1 ORDER BY position",
        )
        .map_err(read_error)?;
    let rows = statement
        .query_map([item_id.as_bytes().as_slice()], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .map_err(read_error)?;
    let mut source_basis = Vec::new();
    for source_id in rows {
        source_basis.push(SourceId::from_slice(&source_id.map_err(read_error)?)?);
    }
    Ok(ContextItem {
        id: item_id,
        project_id,
        revision: stored_revision(row.1)?,
        role: ContextItemRole::parse(&row.2).ok_or_else(|| invalid_stored("Context Item role"))?,
        statement: row.3,
        provenance_role: StatementProvenanceRole::parse(&row.4)
            .ok_or_else(|| invalid_stored("Context Item provenance role"))?,
        author: Principal {
            kind: parse_principal_kind(&row.5)?,
            identity: row.6,
        },
        source_basis,
        applicability: ApplicabilityScope {
            paths: decode_strings(&row.7)?,
            components: decode_strings(&row.8)?,
            work_contexts: decode_strings(&row.9)?,
        },
        recorded_at: TimestampMicros::from_unix_micros(row.10),
    })
}

fn validate_checkpoint_draft(draft: &CheckpointDraft) -> Result<(), Error> {
    validate_nonempty("Checkpoint goal", &draft.goal)?;
    validate_nonempty("Checkpoint next step", &draft.next_step)?;
    validate_optional_nonempty("Checkpoint state change", draft.state_change.as_deref())?;
    validate_optional_nonempty("Checkpoint handoff target", draft.handoff_to.as_deref())?;
    if draft.source_basis.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Checkpoint requires an explicit supporting Source basis",
        ));
    }
    ensure_unique_ids("Checkpoint Source basis", &draft.source_basis)?;
    ensure_unique_ids(
        "Checkpoint changed Source basis",
        &draft.changed_source_basis,
    )?;
    ensure_unique_ids("Checkpoint applied Decisions", &draft.applied_decisions)?;
    validate_string_list("Checkpoint changed path", &draft.changed_paths)?;
    validate_portable_paths("Checkpoint changed path", &draft.changed_paths)?;
    validate_string_list("Checkpoint known limit", &draft.known_limits)?;
    validate_string_list("Checkpoint non-goal", &draft.non_goals)?;
    for verification in &draft.verification {
        match verification.state {
            VerificationState::NotRun => {
                if verification.source_id.is_some() || verification.outcome.is_some() {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "not-run verification cannot claim a Source or outcome",
                    ));
                }
            }
            VerificationState::Partial | VerificationState::Passed | VerificationState::Failed => {
                if verification.source_id.is_none() {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "executed verification requires an explicit Source",
                    ));
                }
                validate_nonempty(
                    "verification outcome",
                    verification.outcome.as_deref().unwrap_or_default(),
                )?;
            }
        }
    }
    match draft.user_review.state {
        UserReviewState::NotRequested | UserReviewState::Pending => {
            if draft.user_review.source_id.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "unobserved user review state cannot claim a user Source",
                ));
            }
        }
        UserReviewState::Reviewed => {
            if draft.user_review.source_id.is_none() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "reviewed state requires an explicit current-host user-turn Source",
                ));
            }
        }
    }
    match draft.user_acceptance.state {
        UserAcceptanceState::NotRequested | UserAcceptanceState::Pending => {
            if draft.user_acceptance.source_id.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "unobserved user acceptance state cannot claim a user Source",
                ));
            }
        }
        UserAcceptanceState::Accepted | UserAcceptanceState::Rejected => {
            if draft.user_acceptance.source_id.is_none() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "accepted or rejected state requires an explicit current-host user-turn Source",
                ));
            }
        }
    }
    let completion_basis = draft.state_change.is_some()
        || !draft.changed_source_basis.is_empty()
        || !draft.changed_paths.is_empty()
        || !draft.applied_decisions.is_empty()
        || draft
            .verification
            .iter()
            .any(|fact| fact.state != VerificationState::NotRun)
        || !draft.known_limits.is_empty();
    match draft.kind {
        CheckpointKind::Completion => {
            if draft.work_state != WorkState::Completed || !completion_basis {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "completion Checkpoint requires completed work and an explicit meaningful basis",
                ));
            }
            if draft.handoff_to.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "completion Checkpoint cannot claim a handoff target",
                ));
            }
        }
        CheckpointKind::Pause => {
            if draft.work_state != WorkState::Paused || draft.handoff_to.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "pause Checkpoint requires paused work and no handoff target",
                ));
            }
        }
        CheckpointKind::Handoff => {
            if draft.handoff_to.is_none() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "handoff Checkpoint requires an explicit handoff target",
                ));
            }
        }
    }
    Ok(())
}

fn checkpoint_basis(project_id: ProjectId, draft: &CheckpointDraft) -> Vec<u8> {
    let mut basis = Basis::new("record_checkpoint")
        .bytes(project_id.as_bytes())
        .number(draft.expected_project_revision)
        .string(draft.kind.as_str())
        .string(&draft.goal)
        .string(draft.work_state.as_str())
        .optional_string(draft.state_change.as_deref())
        .bytes(&encode_source_ids(&draft.source_basis))
        .bytes(&encode_source_ids(&draft.changed_source_basis))
        .bytes(&encode_strings(&draft.changed_paths));
    for decision_id in &draft.applied_decisions {
        basis = basis.bytes(decision_id.as_bytes());
    }
    basis = basis.string("decisions_end");
    for verification in &draft.verification {
        basis = basis.string(verification.state.as_str());
        basis = match verification.source_id {
            Some(source_id) => basis.string("some").bytes(source_id.as_bytes()),
            None => basis.string("none"),
        };
        basis = basis.optional_string(verification.outcome.as_deref());
    }
    basis = basis
        .string("verification_end")
        .string(draft.user_review.state.as_str());
    basis = match draft.user_review.source_id {
        Some(source_id) => basis.string("some").bytes(source_id.as_bytes()),
        None => basis.string("none"),
    };
    basis = basis.string(draft.user_acceptance.state.as_str());
    basis = match draft.user_acceptance.source_id {
        Some(source_id) => basis.string("some").bytes(source_id.as_bytes()),
        None => basis.string("none"),
    };
    basis = basis
        .bytes(&encode_strings(&draft.known_limits))
        .bytes(&encode_strings(&draft.non_goals));
    for question in &draft.open_questions {
        basis = basis
            .bytes(question.question_id.as_bytes())
            .number(question.revision);
    }
    basis
        .string("questions_end")
        .string(&draft.next_step)
        .optional_string(draft.handoff_to.as_deref())
        .finish()
}

fn insert_checkpoint_sources(
    transaction: &Transaction<'_>,
    project_id: ProjectId,
    checkpoint_id: CheckpointId,
    relation_kind: &str,
    source_ids: &[SourceId],
) -> Result<(), Error> {
    for (position, source_id) in source_ids.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO checkpoint_source_relations(
                     project_id, checkpoint_id, relation_kind, source_id, position
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    project_id.as_bytes().as_slice(),
                    checkpoint_id.as_bytes().as_slice(),
                    relation_kind,
                    source_id.as_bytes().as_slice(),
                    position_i64(position)?,
                ],
            )
            .map_err(write_error)?;
    }
    Ok(())
}

fn load_checkpoint(
    connection: &Connection,
    project_id: ProjectId,
    checkpoint_id: CheckpointId,
) -> Result<Checkpoint, Error> {
    type CheckpointRow = (
        Vec<u8>,
        i64,
        String,
        String,
        String,
        Option<String>,
        Vec<u8>,
        String,
        Option<Vec<u8>>,
        String,
        Option<Vec<u8>>,
        Vec<u8>,
        Vec<u8>,
        String,
        Option<String>,
        i64,
    );
    let row: CheckpointRow = connection
        .query_row(
            "SELECT project_id, revision, checkpoint_kind, goal, work_state, state_change,
                    changed_paths, user_review, user_review_source_id, user_acceptance,
                    user_acceptance_source_id, known_limits, non_goals, next_step,
                    handoff_to, recorded_at
             FROM checkpoints WHERE id = ?1",
            [checkpoint_id.as_bytes().as_slice()],
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
                    row.get(14)?,
                    row.get(15)?,
                ))
            },
        )
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Checkpoint was not found"))?;
    let owner = ProjectId::from_slice(&row.0)?;
    if owner != project_id {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "Checkpoint belongs to a different Project",
        ));
    }
    let source_basis = load_checkpoint_source_ids(connection, checkpoint_id, "supported_by")?;
    let changed_source_basis =
        load_checkpoint_source_ids(connection, checkpoint_id, "changed_basis")?;
    let applied_decisions = load_checkpoint_decision_ids(connection, checkpoint_id)?;
    let open_questions = load_checkpoint_question_refs(connection, checkpoint_id)?;
    let verification = load_checkpoint_verification(connection, checkpoint_id)?;
    Ok(Checkpoint {
        id: checkpoint_id,
        project_id,
        revision: stored_revision(row.1)?,
        kind: CheckpointKind::parse(&row.2).ok_or_else(|| invalid_stored("Checkpoint kind"))?,
        goal: row.3,
        work_state: WorkState::parse(&row.4)
            .ok_or_else(|| invalid_stored("Checkpoint work state"))?,
        state_change: row.5,
        source_basis,
        changed_source_basis,
        changed_paths: decode_strings(&row.6)?,
        applied_decisions,
        verification,
        user_review: UserReviewFact {
            state: UserReviewState::parse(&row.7)
                .ok_or_else(|| invalid_stored("Checkpoint user review state"))?,
            source_id: row.8.as_deref().map(SourceId::from_slice).transpose()?,
        },
        user_acceptance: UserAcceptanceFact {
            state: UserAcceptanceState::parse(&row.9)
                .ok_or_else(|| invalid_stored("Checkpoint user acceptance state"))?,
            source_id: row.10.as_deref().map(SourceId::from_slice).transpose()?,
        },
        known_limits: decode_strings(&row.11)?,
        non_goals: decode_strings(&row.12)?,
        open_questions,
        next_step: row.13,
        handoff_to: row.14,
        recorded_at: TimestampMicros::from_unix_micros(row.15),
    })
}

fn load_checkpoint_source_ids(
    connection: &Connection,
    checkpoint_id: CheckpointId,
    relation_kind: &str,
) -> Result<Vec<SourceId>, Error> {
    let mut statement = connection
        .prepare(
            "SELECT source_id FROM checkpoint_source_relations
             WHERE checkpoint_id = ?1 AND relation_kind = ?2 ORDER BY position",
        )
        .map_err(read_error)?;
    let rows = statement
        .query_map(
            params![checkpoint_id.as_bytes().as_slice(), relation_kind],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .map_err(read_error)?;
    let mut values = Vec::new();
    for value in rows {
        values.push(SourceId::from_slice(&value.map_err(read_error)?)?);
    }
    Ok(values)
}

fn load_checkpoint_decision_ids(
    connection: &Connection,
    checkpoint_id: CheckpointId,
) -> Result<Vec<DecisionId>, Error> {
    let mut statement = connection
        .prepare(
            "SELECT decision_id FROM checkpoint_decisions
             WHERE checkpoint_id = ?1 ORDER BY position",
        )
        .map_err(read_error)?;
    let rows = statement
        .query_map([checkpoint_id.as_bytes().as_slice()], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .map_err(read_error)?;
    let mut values = Vec::new();
    for value in rows {
        values.push(DecisionId::from_slice(&value.map_err(read_error)?)?);
    }
    Ok(values)
}

fn load_checkpoint_question_refs(
    connection: &Connection,
    checkpoint_id: CheckpointId,
) -> Result<Vec<QuestionReference>, Error> {
    let mut statement = connection
        .prepare(
            "SELECT question_id, question_revision FROM checkpoint_questions
             WHERE checkpoint_id = ?1 ORDER BY position",
        )
        .map_err(read_error)?;
    let rows = statement
        .query_map([checkpoint_id.as_bytes().as_slice()], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(read_error)?;
    let mut values = Vec::new();
    for value in rows {
        let (question_id, revision) = value.map_err(read_error)?;
        values.push(QuestionReference {
            question_id: QuestionId::from_slice(&question_id)?,
            revision: stored_revision(revision)?,
        });
    }
    Ok(values)
}

fn load_checkpoint_verification(
    connection: &Connection,
    checkpoint_id: CheckpointId,
) -> Result<Vec<VerificationFact>, Error> {
    let mut statement = connection
        .prepare(
            "SELECT verification_state, source_id, outcome FROM checkpoint_verifications
             WHERE checkpoint_id = ?1 ORDER BY position",
        )
        .map_err(read_error)?;
    let rows = statement
        .query_map([checkpoint_id.as_bytes().as_slice()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<Vec<u8>>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(read_error)?;
    let mut values = Vec::new();
    for value in rows {
        let (state, source_id, outcome) = value.map_err(read_error)?;
        values.push(VerificationFact {
            state: VerificationState::parse(&state)
                .ok_or_else(|| invalid_stored("Checkpoint verification state"))?,
            source_id: source_id
                .map(|value| SourceId::from_slice(&value))
                .transpose()?,
            outcome,
        });
    }
    Ok(values)
}

fn ensure_unique_ids<T: Copy + Ord>(label: &str, values: &[T]) -> Result<(), Error> {
    let mut unique = BTreeSet::new();
    if values.iter().any(|value| !unique.insert(*value)) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must not contain duplicates"),
        ));
    }
    Ok(())
}

fn position_i64(position: usize) -> Result<i64, Error> {
    i64::try_from(position).map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            "ordered canonical relation position is outside the supported range",
        )
    })
}

fn invalid_stored(label: &str) -> Error {
    Error::new(
        ErrorKind::CorruptState,
        format!("stored {label} is invalid"),
    )
}

struct EncodedQuestion {
    source_basis: Vec<u8>,
    dependencies: Vec<u8>,
    alternatives: Vec<u8>,
    recommendation_sources: Vec<u8>,
    trade_offs: Vec<u8>,
    uncertainty: Vec<u8>,
    material_scope: Vec<u8>,
}

impl EncodedQuestion {
    fn from_draft(draft: &QuestionDraft) -> Result<Self, Error> {
        Ok(Self {
            source_basis: encode_source_ids(&draft.source_basis),
            dependencies: encode_dependencies(&draft.dependencies),
            alternatives: encode_alternatives(&draft.alternatives),
            recommendation_sources: encode_source_ids(&draft.recommendation.source_basis),
            trade_offs: encode_strings(&draft.trade_offs),
            uncertainty: encode_strings(&draft.uncertainty),
            material_scope: encode_strings(&draft.material_scope),
        })
    }
}

fn validate_question_draft(draft: &QuestionDraft) -> Result<(), Error> {
    validate_nonempty("Question prompt basis", &draft.prompt_basis)?;
    validate_nonempty(
        "Question recommendation rationale",
        &draft.recommendation.rationale,
    )?;
    if draft.source_basis.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question requires a Source basis",
        ));
    }
    if draft.recommendation.source_basis.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question recommendation requires a Source basis",
        ));
    }
    if draft.alternatives.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question requires at least one explicit alternative",
        ));
    }
    if draft.material_scope.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question requires a material scope",
        ));
    }
    let mut keys = BTreeSet::new();
    for alternative in &draft.alternatives {
        validate_nonempty("Question alternative key", &alternative.key)?;
        validate_nonempty("Question alternative label", &alternative.label)?;
        validate_nonempty("Question alternative consequence", &alternative.consequence)?;
        if !keys.insert(alternative.key.as_str()) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Question alternative keys must be unique",
            ));
        }
    }
    if let Some(key) = &draft.recommendation.alternative_key {
        if !keys.contains(key.as_str()) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Question recommendation must name a displayed alternative",
            ));
        }
    }
    validate_string_list("Question trade-off", &draft.trade_offs)?;
    validate_string_list("Question uncertainty", &draft.uncertainty)?;
    validate_string_list("Question material scope", &draft.material_scope)?;
    let mut dependencies = BTreeSet::new();
    for dependency in &draft.dependencies {
        if !dependencies.insert((dependency.question_id, dependency.required_revision)) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Question dependencies must be unique",
            ));
        }
    }
    Ok(())
}

fn validate_response_draft(draft: &QuestionResponseDraft) -> Result<(), Error> {
    validate_string_list(
        "displayed Question alternative key",
        &draft.displayed_alternative_keys,
    )?;
    validate_string_list("Decision applicability path", &draft.applicability.paths)?;
    validate_portable_paths("Decision applicability path", &draft.applicability.paths)?;
    validate_string_list(
        "Decision applicability component",
        &draft.applicability.components,
    )?;
    validate_string_list(
        "Decision applicability work context",
        &draft.applicability.work_contexts,
    )?;
    validate_string_list("Decision assumption", &draft.assumptions)?;
    validate_string_list("Decision revisit trigger", &draft.revisit_triggers)?;
    match &draft.response {
        ExplicitQuestionResponse::Choice {
            alternative_key,
            user_rationale,
        } => {
            validate_nonempty("explicit alternative key", alternative_key)?;
            validate_optional_nonempty("user rationale", user_rationale.as_deref())?;
        }
        ExplicitQuestionResponse::Delegation {
            delegate_to,
            user_rationale,
        } => {
            validate_nonempty("delegation target", delegate_to)?;
            validate_optional_nonempty("user rationale", user_rationale.as_deref())?;
        }
        ExplicitQuestionResponse::Terminal { outcome } => {
            if matches!(
                outcome,
                QuestionTerminalOutcome::Answered | QuestionTerminalOutcome::Delegated
            ) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "answered and delegated outcomes require an explicit choice or delegation",
                ));
            }
        }
    }
    Ok(())
}

fn validate_optional_nonempty(label: &str, value: Option<&str>) -> Result<(), Error> {
    if let Some(value) = value {
        validate_nonempty(label, value)?;
    }
    Ok(())
}

fn validate_string_list(label: &str, values: &[String]) -> Result<(), Error> {
    for value in values {
        validate_nonempty(label, value)?;
    }
    Ok(())
}

fn question_basis(
    project_id: ProjectId,
    draft: &QuestionDraft,
    encoded: &EncodedQuestion,
) -> Vec<u8> {
    Basis::new("create_question")
        .bytes(project_id.as_bytes())
        .number(draft.expected_project_revision)
        .string(&draft.prompt_basis)
        .bytes(&encoded.source_basis)
        .bytes(&encoded.dependencies)
        .bytes(&encoded.alternatives)
        .optional_string(draft.recommendation.alternative_key.as_deref())
        .string(&draft.recommendation.rationale)
        .bytes(&encoded.recommendation_sources)
        .bytes(&encoded.trade_offs)
        .bytes(&encoded.uncertainty)
        .bytes(&encoded.material_scope)
        .finish()
}

fn response_basis(project_id: ProjectId, draft: &QuestionResponseDraft) -> Result<Vec<u8>, Error> {
    let mut basis = Basis::new("record_question_response")
        .bytes(project_id.as_bytes())
        .number(draft.expected_project_revision)
        .bytes(draft.question_id.as_bytes())
        .number(draft.question_revision);
    basis = match &draft.user_turn_source {
        UserTurnSource::Existing(source_id) => basis.string("existing").bytes(source_id.as_bytes()),
        UserTurnSource::Create(source_draft) => {
            validate_source_draft(source_draft)?;
            let encoded = EncodedSource::from_payload(&source_draft.payload);
            basis
                .string("create")
                .bytes(&source_basis(project_id, source_draft, &encoded))
        }
    };
    basis = basis
        .bytes(&encode_strings(&draft.displayed_alternative_keys))
        .optional_string(draft.displayed_recommendation_key.as_deref());
    basis = match &draft.response {
        ExplicitQuestionResponse::Choice {
            alternative_key,
            user_rationale,
        } => basis
            .string("choice")
            .string(alternative_key)
            .optional_string(user_rationale.as_deref()),
        ExplicitQuestionResponse::Delegation {
            delegate_to,
            user_rationale,
        } => basis
            .string("delegation")
            .string(delegate_to)
            .optional_string(user_rationale.as_deref()),
        ExplicitQuestionResponse::Terminal { outcome } => {
            basis.string("terminal").string(outcome.as_str())
        }
    };
    Ok(basis
        .bytes(&encode_strings(&draft.applicability.paths))
        .bytes(&encode_strings(&draft.applicability.components))
        .bytes(&encode_strings(&draft.applicability.work_contexts))
        .bytes(&encode_strings(&draft.assumptions))
        .bytes(&encode_strings(&draft.revisit_triggers))
        .finish())
}

fn insert_question_revision(
    transaction: &Transaction<'_>,
    question_id: QuestionId,
    project_id: ProjectId,
    draft: &QuestionDraft,
    encoded: &EncodedQuestion,
    now: TimestampMicros,
) -> Result<(), Error> {
    transaction
        .execute(
            "INSERT INTO question_revisions(
                 question_id, revision, project_id, prompt_basis, source_basis, dependencies,
                 alternatives, recommendation_key, recommendation_rationale,
                 recommendation_sources, trade_offs, uncertainty, material_scope, recorded_at
             ) VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                question_id.as_bytes().as_slice(),
                project_id.as_bytes().as_slice(),
                draft.prompt_basis,
                encoded.source_basis,
                encoded.dependencies,
                encoded.alternatives,
                draft.recommendation.alternative_key,
                draft.recommendation.rationale,
                encoded.recommendation_sources,
                encoded.trade_offs,
                encoded.uncertainty,
                encoded.material_scope,
                now.as_unix_micros(),
            ],
        )
        .map_err(write_error)?;
    Ok(())
}

fn load_question(
    connection: &Connection,
    project_id: ProjectId,
    question_id: QuestionId,
) -> Result<Question, Error> {
    let current = connection
        .query_row(
            "SELECT project_id, revision, terminal_outcome, created_at, updated_at
             FROM questions WHERE id = ?1",
            [question_id.as_bytes().as_slice()],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Question was not found"))?;
    let owner = ProjectId::from_slice(&current.0)?;
    if owner != project_id {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "Question belongs to a different Project",
        ));
    }
    let revision = stored_revision(current.1)?;
    let row = connection
        .query_row(
            "SELECT prompt_basis, source_basis, dependencies, alternatives,
                    recommendation_key, recommendation_rationale, recommendation_sources,
                    trade_offs, uncertainty, material_scope
             FROM question_revisions WHERE question_id = ?1 AND revision = ?2",
            params![question_id.as_bytes().as_slice(), revision_i64(revision)?],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Vec<u8>>(6)?,
                    row.get::<_, Vec<u8>>(7)?,
                    row.get::<_, Vec<u8>>(8)?,
                    row.get::<_, Vec<u8>>(9)?,
                ))
            },
        )
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| {
            Error::new(
                ErrorKind::RepairRequired,
                "Question current revision is missing",
            )
        })?;
    let state = match current.2 {
        None => QuestionState::Open,
        Some(value) => {
            QuestionState::Terminal(QuestionTerminalOutcome::parse(&value).ok_or_else(|| {
                Error::new(
                    ErrorKind::CorruptState,
                    "stored Question terminal outcome is invalid",
                )
            })?)
        }
    };
    Ok(Question {
        id: question_id,
        project_id,
        revision,
        prompt_basis: row.0,
        source_basis: decode_source_ids(&row.1)?,
        dependencies: decode_dependencies(&row.2)?,
        alternatives: decode_alternatives(&row.3)?,
        recommendation: AgentRecommendation {
            alternative_key: row.4,
            rationale: row.5,
            source_basis: decode_source_ids(&row.6)?,
        },
        trade_offs: decode_strings(&row.7)?,
        uncertainty: decode_strings(&row.8)?,
        material_scope: decode_strings(&row.9)?,
        state,
        created_at: TimestampMicros::from_unix_micros(current.3),
        updated_at: TimestampMicros::from_unix_micros(current.4),
    })
}

fn ensure_question_revision_exists(
    connection: &Connection,
    question_id: QuestionId,
    revision: u64,
) -> Result<(), Error> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM question_revisions WHERE question_id = ?1 AND revision = ?2",
            params![question_id.as_bytes().as_slice(), revision_i64(revision)?],
            |_| Ok(()),
        )
        .optional()
        .map_err(read_error)?
        .is_some();
    if !exists {
        return Err(Error::new(
            ErrorKind::StaleBasis,
            "Question revision does not exist",
        ));
    }
    Ok(())
}

fn interpret_explicit_response<'a>(
    question: &Question,
    response: &'a ExplicitQuestionResponse,
) -> Result<
    (
        QuestionTerminalOutcome,
        Option<DecisionChoice>,
        Option<&'a str>,
    ),
    Error,
> {
    match response {
        ExplicitQuestionResponse::Choice {
            alternative_key,
            user_rationale,
        } => {
            if !question
                .alternatives
                .iter()
                .any(|alternative| alternative.key == *alternative_key)
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "explicit choice does not name one displayed alternative",
                ));
            }
            Ok((
                QuestionTerminalOutcome::Answered,
                Some(DecisionChoice::Alternative {
                    alternative_key: alternative_key.clone(),
                }),
                user_rationale.as_deref(),
            ))
        }
        ExplicitQuestionResponse::Delegation {
            delegate_to,
            user_rationale,
        } => Ok((
            QuestionTerminalOutcome::Delegated,
            Some(DecisionChoice::Delegation {
                delegate_to: delegate_to.clone(),
            }),
            user_rationale.as_deref(),
        )),
        ExplicitQuestionResponse::Terminal { outcome } => Ok((*outcome, None, None)),
    }
}

fn ensure_user_turn_draft(draft: &SourceDraft) -> Result<(), Error> {
    if draft.actor.kind != PrincipalKind::User
        || !matches!(draft.payload, SourcePayload::CurrentHostUserTurn { .. })
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question response requires a current-host user-turn Source authored by the user",
        ));
    }
    Ok(())
}

fn ensure_user_turn_source(source: &Source, project_id: ProjectId) -> Result<(), Error> {
    if source.project_id != project_id {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "Question response Source belongs to a different Project",
        ));
    }
    if source.actor.kind != PrincipalKind::User
        || !matches!(source.payload, SourcePayload::CurrentHostUserTurn { .. })
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question response requires a current-host user-turn Source authored by the user",
        ));
    }
    Ok(())
}

fn insert_source(
    transaction: &Transaction<'_>,
    source_id: SourceId,
    project_id: ProjectId,
    draft: &SourceDraft,
    now: TimestampMicros,
) -> Result<(), Error> {
    let encoded = EncodedSource::from_payload(&draft.payload);
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
    Ok(())
}

fn load_question_response(
    connection: &Connection,
    project_id: ProjectId,
    question_id: QuestionId,
    question_revision: u64,
) -> Result<QuestionResponseResult, Error> {
    let source_bytes: Vec<u8> = connection
        .query_row(
            "SELECT source_id FROM question_response_sources
             WHERE project_id = ?1 AND question_id = ?2 AND question_revision = ?3",
            params![
                project_id.as_bytes().as_slice(),
                question_id.as_bytes().as_slice(),
                revision_i64(question_revision)?,
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| {
            Error::new(
                ErrorKind::RepairRequired,
                "committed Question response has no Source linkage",
            )
        })?;
    let user_turn_source = load_source(connection, SourceId::from_slice(&source_bytes)?)?;
    let decision_id: Option<Vec<u8>> = connection
        .query_row(
            "SELECT id FROM decisions WHERE question_id = ?1 AND question_revision = ?2
             ORDER BY recorded_at, id LIMIT 1",
            params![
                question_id.as_bytes().as_slice(),
                revision_i64(question_revision)?
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(read_error)?;
    let decision = decision_id
        .map(|bytes| load_decision(connection, project_id, DecisionId::from_slice(&bytes)?))
        .transpose()?;
    Ok(QuestionResponseResult {
        question: load_question(connection, project_id, question_id)?,
        user_turn_source,
        decision,
    })
}

fn load_decision(
    connection: &Connection,
    project_id: ProjectId,
    decision_id: DecisionId,
) -> Result<Decision, Error> {
    type DecisionRow = (
        Vec<u8>,
        i64,
        Vec<u8>,
        i64,
        Vec<u8>,
        String,
        String,
        Option<String>,
        Vec<u8>,
        Option<String>,
        String,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        i64,
    );
    let row: DecisionRow = connection
        .query_row(
            "SELECT project_id, revision, question_id, question_revision, user_turn_source_id,
                    choice_kind, choice_value, user_rationale, displayed_alternatives,
                    recommendation_key, recommendation_rationale, recommendation_sources,
                    applicability_paths, applicability_components, applicability_work_contexts,
                    assumptions, revisit_triggers, recorded_at
             FROM decisions WHERE id = ?1",
            [decision_id.as_bytes().as_slice()],
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
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                ))
            },
        )
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Decision was not found"))?;
    let owner = ProjectId::from_slice(&row.0)?;
    if owner != project_id {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "Decision belongs to a different Project",
        ));
    }
    let choice = match row.5.as_str() {
        "alternative" => DecisionChoice::Alternative {
            alternative_key: row.6,
        },
        "delegation" => DecisionChoice::Delegation { delegate_to: row.6 },
        _ => {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "stored Decision choice kind is invalid",
            ));
        }
    };
    Ok(Decision {
        id: decision_id,
        project_id,
        revision: stored_revision(row.1)?,
        question_id: QuestionId::from_slice(&row.2)?,
        question_revision: stored_revision(row.3)?,
        user_turn_source_id: SourceId::from_slice(&row.4)?,
        choice,
        user_rationale: row.7,
        displayed_alternatives: decode_alternatives(&row.8)?,
        displayed_recommendation: AgentRecommendation {
            alternative_key: row.9,
            rationale: row.10,
            source_basis: decode_source_ids(&row.11)?,
        },
        applicability: ApplicabilityScope {
            paths: decode_strings(&row.12)?,
            components: decode_strings(&row.13)?,
            work_contexts: decode_strings(&row.14)?,
        },
        assumptions: decode_strings(&row.15)?,
        revisit_triggers: decode_strings(&row.16)?,
        recorded_at: TimestampMicros::from_unix_micros(row.17),
    })
}

fn encode_strings(values: &[String]) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u64(&mut bytes, values.len() as u64);
    for value in values {
        push_bytes(&mut bytes, value.as_bytes());
    }
    bytes
}

fn decode_strings(bytes: &[u8]) -> Result<Vec<String>, Error> {
    let mut cursor = BlobCursor::new(bytes);
    let count = cursor.count()?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let value = std::str::from_utf8(cursor.bytes()?).map_err(|_| malformed_blob())?;
        values.push(value.to_owned());
    }
    cursor.finish()?;
    Ok(values)
}

fn encode_source_ids(values: &[SourceId]) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u64(&mut bytes, values.len() as u64);
    for value in values {
        bytes.extend_from_slice(value.as_bytes());
    }
    bytes
}

pub(crate) fn decode_source_ids(bytes: &[u8]) -> Result<Vec<SourceId>, Error> {
    let mut cursor = BlobCursor::new(bytes);
    let count = cursor.count()?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(SourceId::from_slice(cursor.fixed(16)?)?);
    }
    cursor.finish()?;
    Ok(values)
}

fn encode_dependencies(values: &[QuestionDependency]) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u64(&mut bytes, values.len() as u64);
    for value in values {
        bytes.extend_from_slice(value.question_id.as_bytes());
        match value.required_revision {
            Some(revision) => {
                bytes.push(1);
                push_u64(&mut bytes, revision);
            }
            None => bytes.push(0),
        }
    }
    bytes
}

fn decode_dependencies(bytes: &[u8]) -> Result<Vec<QuestionDependency>, Error> {
    let mut cursor = BlobCursor::new(bytes);
    let count = cursor.count()?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let question_id = QuestionId::from_slice(cursor.fixed(16)?)?;
        let required_revision = match cursor.byte()? {
            0 => None,
            1 => Some(cursor.u64()?),
            _ => return Err(malformed_blob()),
        };
        values.push(QuestionDependency {
            question_id,
            required_revision,
        });
    }
    cursor.finish()?;
    Ok(values)
}

fn remove_question_dependencies(
    transaction: &Transaction<'_>,
    project_id: ProjectId,
    forgotten_question_id: QuestionId,
) -> Result<(), Error> {
    let mut statement = transaction
        .prepare(
            "SELECT question_id, revision, dependencies FROM question_revisions
             WHERE project_id = ?1 ORDER BY question_id, revision",
        )
        .map_err(read_error)?;
    let rows = statement
        .query_map([project_id.as_bytes().as_slice()], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(read_error)?;
    let mut updates = Vec::new();
    for row in rows {
        let (question_bytes, revision, encoded) = row.map_err(read_error)?;
        let question_id = QuestionId::from_slice(&question_bytes)?;
        if question_id == forgotten_question_id {
            continue;
        }
        let mut dependencies = decode_dependencies(&encoded)?;
        let before = dependencies.len();
        dependencies.retain(|dependency| dependency.question_id != forgotten_question_id);
        if dependencies.len() != before {
            updates.push((question_id, revision, encode_dependencies(&dependencies)));
        }
    }
    drop(statement);
    for (question_id, revision, dependencies) in updates {
        transaction
            .execute(
                "UPDATE question_revisions SET dependencies = ?4
                 WHERE project_id = ?1 AND question_id = ?2 AND revision = ?3",
                params![
                    project_id.as_bytes().as_slice(),
                    question_id.as_bytes().as_slice(),
                    revision,
                    dependencies,
                ],
            )
            .map_err(write_error)?;
    }
    Ok(())
}

fn encode_alternatives(values: &[QuestionAlternative]) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u64(&mut bytes, values.len() as u64);
    for value in values {
        push_bytes(&mut bytes, value.key.as_bytes());
        push_bytes(&mut bytes, value.label.as_bytes());
        push_bytes(&mut bytes, value.consequence.as_bytes());
    }
    bytes
}

pub(crate) fn decode_alternatives(bytes: &[u8]) -> Result<Vec<QuestionAlternative>, Error> {
    let mut cursor = BlobCursor::new(bytes);
    let count = cursor.count()?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let key = std::str::from_utf8(cursor.bytes()?)
            .map_err(|_| malformed_blob())?
            .to_owned();
        let label = std::str::from_utf8(cursor.bytes()?)
            .map_err(|_| malformed_blob())?
            .to_owned();
        let consequence = std::str::from_utf8(cursor.bytes()?)
            .map_err(|_| malformed_blob())?
            .to_owned();
        values.push(QuestionAlternative {
            key,
            label,
            consequence,
        });
    }
    cursor.finish()?;
    Ok(values)
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    push_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value);
}

struct BlobCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> BlobCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn count(&mut self) -> Result<usize, Error> {
        usize::try_from(self.u64()?).map_err(|_| malformed_blob())
    }

    fn u64(&mut self) -> Result<u64, Error> {
        let bytes: [u8; 8] = self.fixed(8)?.try_into().map_err(|_| malformed_blob())?;
        Ok(u64::from_be_bytes(bytes))
    }

    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.fixed(1)?[0])
    }

    fn bytes(&mut self) -> Result<&'a [u8], Error> {
        let length = self.count()?;
        self.fixed(length)
    }

    fn fixed(&mut self, length: usize) -> Result<&'a [u8], Error> {
        let end = self.offset.checked_add(length).ok_or_else(malformed_blob)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(malformed_blob)?;
        self.offset = end;
        Ok(value)
    }

    fn finish(self) -> Result<(), Error> {
        if self.offset != self.bytes.len() {
            return Err(malformed_blob());
        }
        Ok(())
    }
}

fn malformed_blob() -> Error {
    Error::new(
        ErrorKind::CorruptState,
        "stored canonical structured value is malformed",
    )
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
             CREATE TABLE questions(
                 id BLOB PRIMARY KEY NOT NULL CHECK(length(id) = 16),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 terminal_outcome TEXT CHECK(terminal_outcome IS NULL OR terminal_outcome IN (
                     'answered','delegated','resolved_by_research','requires_prototype',
                     'deferred','out_of_scope','superseded'
                 )),
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL,
                 UNIQUE(project_id, id),
                 UNIQUE(id, revision),
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             );
             CREATE TABLE question_revisions(
                 question_id BLOB NOT NULL CHECK(length(question_id) = 16),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 prompt_basis TEXT NOT NULL CHECK(length(prompt_basis) > 0),
                 source_basis BLOB NOT NULL,
                 dependencies BLOB NOT NULL,
                 alternatives BLOB NOT NULL,
                 recommendation_key TEXT,
                 recommendation_rationale TEXT NOT NULL CHECK(length(recommendation_rationale) > 0),
                 recommendation_sources BLOB NOT NULL,
                 trade_offs BLOB NOT NULL,
                 uncertainty BLOB NOT NULL,
                 material_scope BLOB NOT NULL,
                 recorded_at INTEGER NOT NULL,
                 PRIMARY KEY(question_id, revision),
                 FOREIGN KEY(project_id, question_id) REFERENCES questions(project_id, id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE question_response_sources(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 question_id BLOB NOT NULL CHECK(length(question_id) = 16),
                 question_revision INTEGER NOT NULL CHECK(question_revision >= 1),
                 source_id BLOB NOT NULL CHECK(length(source_id) = 16),
                 recorded_at INTEGER NOT NULL,
                 PRIMARY KEY(project_id, question_id, question_revision),
                 FOREIGN KEY(project_id, question_id) REFERENCES questions(project_id, id) ON DELETE RESTRICT,
                 FOREIGN KEY(question_id, question_revision) REFERENCES question_revisions(question_id, revision) ON DELETE RESTRICT,
                 FOREIGN KEY(project_id, source_id) REFERENCES sources(project_id, id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE decisions(
                 id BLOB PRIMARY KEY NOT NULL CHECK(length(id) = 16),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 question_id BLOB NOT NULL CHECK(length(question_id) = 16),
                 question_revision INTEGER NOT NULL CHECK(question_revision >= 1),
                 user_turn_source_id BLOB NOT NULL CHECK(length(user_turn_source_id) = 16),
                 choice_kind TEXT NOT NULL CHECK(choice_kind IN ('alternative','delegation')),
                 choice_value TEXT NOT NULL CHECK(length(choice_value) > 0),
                 user_rationale TEXT,
                 displayed_alternatives BLOB NOT NULL,
                 recommendation_key TEXT,
                 recommendation_rationale TEXT NOT NULL,
                 recommendation_sources BLOB NOT NULL,
                 applicability_paths BLOB NOT NULL,
                 applicability_components BLOB NOT NULL,
                 applicability_work_contexts BLOB NOT NULL,
                 assumptions BLOB NOT NULL,
                 revisit_triggers BLOB NOT NULL,
                 recorded_at INTEGER NOT NULL,
                 UNIQUE(project_id, id),
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             );
             CREATE TABLE context_items(
                 id BLOB PRIMARY KEY NOT NULL CHECK(length(id) = 16),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 role TEXT NOT NULL CHECK(role IN (
                     'goal','fact','assumption','constraint','preference','risk','learning','known_limit'
                 )),
                 statement TEXT NOT NULL CHECK(length(statement) > 0),
                 provenance_role TEXT NOT NULL CHECK(provenance_role IN (
                     'user_statement','observed','agent_statement','generated_interpretation'
                 )),
                 author_kind TEXT NOT NULL,
                 author_identity TEXT NOT NULL CHECK(length(author_identity) > 0),
                 applicability_paths BLOB NOT NULL,
                 applicability_components BLOB NOT NULL,
                 applicability_work_contexts BLOB NOT NULL,
                 recorded_at INTEGER NOT NULL,
                 UNIQUE(project_id, id),
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             );
             CREATE TABLE context_item_sources(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 context_item_id BLOB NOT NULL CHECK(length(context_item_id) = 16),
                 source_id BLOB NOT NULL CHECK(length(source_id) = 16),
                 position INTEGER NOT NULL CHECK(position >= 0),
                 PRIMARY KEY(context_item_id, position),
                 UNIQUE(context_item_id, source_id),
                 FOREIGN KEY(project_id, context_item_id) REFERENCES context_items(project_id, id) ON DELETE RESTRICT,
                 FOREIGN KEY(project_id, source_id) REFERENCES sources(project_id, id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE context_item_revisions(
                 context_item_id BLOB NOT NULL CHECK(length(context_item_id) = 16),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 role TEXT NOT NULL,
                 statement TEXT NOT NULL CHECK(length(statement) > 0),
                 provenance_role TEXT NOT NULL,
                 author_kind TEXT NOT NULL,
                 author_identity TEXT NOT NULL,
                 source_basis BLOB NOT NULL,
                 applicability_paths BLOB NOT NULL,
                 applicability_components BLOB NOT NULL,
                 applicability_work_contexts BLOB NOT NULL,
                 correction_kind TEXT,
                 authorization_source_id BLOB,
                 recorded_at INTEGER NOT NULL,
                 PRIMARY KEY(context_item_id, revision)
             ) WITHOUT ROWID;
             CREATE TABLE decision_revisions(
                 decision_id BLOB NOT NULL CHECK(length(decision_id) = 16),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 question_id BLOB NOT NULL CHECK(length(question_id) = 16),
                 question_revision INTEGER NOT NULL,
                 user_turn_source_id BLOB NOT NULL CHECK(length(user_turn_source_id) = 16),
                 choice_kind TEXT NOT NULL,
                 choice_value TEXT NOT NULL,
                 user_rationale TEXT,
                 displayed_alternatives BLOB NOT NULL,
                 recommendation_key TEXT,
                 recommendation_rationale TEXT NOT NULL,
                 recommendation_sources BLOB NOT NULL,
                 applicability_paths BLOB NOT NULL,
                 applicability_components BLOB NOT NULL,
                 applicability_work_contexts BLOB NOT NULL,
                 assumptions BLOB NOT NULL,
                 revisit_triggers BLOB NOT NULL,
                 correction_kind TEXT,
                 authorization_source_id BLOB,
                 recorded_at INTEGER NOT NULL,
                 PRIMARY KEY(decision_id, revision)
             ) WITHOUT ROWID;
             CREATE TABLE canonical_relations(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 from_kind TEXT NOT NULL,
                 from_id BLOB NOT NULL CHECK(length(from_id) = 16),
                 relation_kind TEXT NOT NULL CHECK(relation_kind IN ('supersedes','contradicts')),
                 to_kind TEXT NOT NULL,
                 to_id BLOB NOT NULL CHECK(length(to_id) = 16),
                 recorded_at INTEGER NOT NULL,
                 PRIMARY KEY(project_id, from_kind, from_id, relation_kind, to_kind, to_id)
             ) WITHOUT ROWID;
             CREATE TABLE review_due(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 decision_id BLOB NOT NULL CHECK(length(decision_id) = 16),
                 review_kind TEXT NOT NULL,
                 explanation TEXT NOT NULL CHECK(length(explanation) > 0),
                 source_basis BLOB NOT NULL,
                 marked_at INTEGER NOT NULL,
                 PRIMARY KEY(project_id, decision_id)
             ) WITHOUT ROWID;
             CREATE TABLE tombstones(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 record_kind TEXT NOT NULL,
                 record_id BLOB NOT NULL CHECK(length(record_id) = 16),
                 forgotten_at INTEGER NOT NULL,
                 PRIMARY KEY(project_id, record_kind, record_id)
             ) WITHOUT ROWID;
             CREATE TABLE managed_bundle_paths(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 absolute_path TEXT NOT NULL CHECK(length(absolute_path) > 0),
                 PRIMARY KEY(project_id, absolute_path),
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE bundle_lineage(
                 project_id BLOB PRIMARY KEY NOT NULL CHECK(length(project_id) = 16),
                 common_base_basis TEXT NOT NULL CHECK(length(common_base_basis) = 64),
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
             ) WITHOUT ROWID;
             CREATE TABLE merge_events(
                 operation_id BLOB PRIMARY KEY NOT NULL CHECK(length(operation_id) = 16),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 conflict_set_id TEXT NOT NULL CHECK(length(conflict_set_id) = 64),
                 conflict_revision INTEGER NOT NULL CHECK(conflict_revision >= 1),
                 common_base_basis TEXT,
                 local_history_basis TEXT NOT NULL CHECK(length(local_history_basis) = 64),
                 incoming_history_basis TEXT NOT NULL CHECK(length(incoming_history_basis) = 64),
                 result_history_basis TEXT NOT NULL CHECK(length(result_history_basis) = 64),
                 resolution_kind TEXT NOT NULL CHECK(resolution_kind IN (
                     'automatic','already_present','unresolved','choose_local','choose_incoming','explicit_merged','context_branch'
                 )),
                 resolution_source_id BLOB CHECK(resolution_source_id IS NULL OR length(resolution_source_id) = 16),
                 conflict_classes BLOB NOT NULL,
                 affected_identities BLOB NOT NULL,
                 branch_history_basis TEXT,
                 committed_at INTEGER NOT NULL,
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE merge_sanitation(
                 operation_id BLOB PRIMARY KEY NOT NULL CHECK(length(operation_id) = 16),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 sanitation_state TEXT NOT NULL CHECK(sanitation_state IN ('pending','complete')),
                 updated_at INTEGER NOT NULL,
                 FOREIGN KEY(operation_id) REFERENCES operations(operation_id) ON DELETE CASCADE,
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE checkpoints(
                 id BLOB PRIMARY KEY NOT NULL CHECK(length(id) = 16),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 revision INTEGER NOT NULL CHECK(revision = 1),
                 checkpoint_kind TEXT NOT NULL CHECK(checkpoint_kind IN ('completion','pause','handoff')),
                 goal TEXT NOT NULL CHECK(length(goal) > 0),
                 work_state TEXT NOT NULL CHECK(work_state IN (
                     'in_progress','paused','completed','abandoned','superseded'
                 )),
                 state_change TEXT,
                 changed_paths BLOB NOT NULL,
                 user_review TEXT NOT NULL CHECK(user_review IN ('not_requested','pending','reviewed')),
                 user_review_source_id BLOB CHECK(user_review_source_id IS NULL OR length(user_review_source_id) = 16),
                 user_acceptance TEXT NOT NULL CHECK(user_acceptance IN (
                     'not_requested','pending','accepted','rejected'
                 )),
                 user_acceptance_source_id BLOB CHECK(user_acceptance_source_id IS NULL OR length(user_acceptance_source_id) = 16),
                 known_limits BLOB NOT NULL,
                 non_goals BLOB NOT NULL,
                 next_step TEXT NOT NULL CHECK(length(next_step) > 0),
                 handoff_to TEXT,
                 recorded_at INTEGER NOT NULL,
                 UNIQUE(project_id, id),
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT,
                 FOREIGN KEY(project_id, user_review_source_id) REFERENCES sources(project_id, id) ON DELETE RESTRICT,
                 FOREIGN KEY(project_id, user_acceptance_source_id) REFERENCES sources(project_id, id) ON DELETE RESTRICT
             );
             CREATE TABLE checkpoint_source_relations(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 checkpoint_id BLOB NOT NULL CHECK(length(checkpoint_id) = 16),
                 relation_kind TEXT NOT NULL CHECK(relation_kind IN ('supported_by','changed_basis')),
                 source_id BLOB NOT NULL CHECK(length(source_id) = 16),
                 position INTEGER NOT NULL CHECK(position >= 0),
                 PRIMARY KEY(checkpoint_id, relation_kind, position),
                 UNIQUE(checkpoint_id, relation_kind, source_id),
                 FOREIGN KEY(project_id, checkpoint_id) REFERENCES checkpoints(project_id, id) ON DELETE RESTRICT,
                 FOREIGN KEY(project_id, source_id) REFERENCES sources(project_id, id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE checkpoint_decisions(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 checkpoint_id BLOB NOT NULL CHECK(length(checkpoint_id) = 16),
                 decision_id BLOB NOT NULL CHECK(length(decision_id) = 16),
                 position INTEGER NOT NULL CHECK(position >= 0),
                 PRIMARY KEY(checkpoint_id, position),
                 UNIQUE(checkpoint_id, decision_id),
                 FOREIGN KEY(project_id, checkpoint_id) REFERENCES checkpoints(project_id, id) ON DELETE RESTRICT,
                 FOREIGN KEY(project_id, decision_id) REFERENCES decisions(project_id, id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE checkpoint_questions(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 checkpoint_id BLOB NOT NULL CHECK(length(checkpoint_id) = 16),
                 question_id BLOB NOT NULL CHECK(length(question_id) = 16),
                 question_revision INTEGER NOT NULL CHECK(question_revision >= 1),
                 position INTEGER NOT NULL CHECK(position >= 0),
                 PRIMARY KEY(checkpoint_id, position),
                 UNIQUE(checkpoint_id, question_id, question_revision),
                 FOREIGN KEY(project_id, checkpoint_id) REFERENCES checkpoints(project_id, id) ON DELETE RESTRICT,
                 FOREIGN KEY(project_id, question_id) REFERENCES questions(project_id, id) ON DELETE RESTRICT,
                 FOREIGN KEY(question_id, question_revision) REFERENCES question_revisions(question_id, revision) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE checkpoint_verifications(
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 checkpoint_id BLOB NOT NULL CHECK(length(checkpoint_id) = 16),
                 position INTEGER NOT NULL CHECK(position >= 0),
                 verification_state TEXT NOT NULL CHECK(verification_state IN (
                     'not_run','partial','passed','failed'
                 )),
                 source_id BLOB CHECK(source_id IS NULL OR length(source_id) = 16),
                 outcome TEXT,
                 PRIMARY KEY(checkpoint_id, position),
                 FOREIGN KEY(project_id, checkpoint_id) REFERENCES checkpoints(project_id, id) ON DELETE RESTRICT,
                 FOREIGN KEY(project_id, source_id) REFERENCES sources(project_id, id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE operations(
                 operation_id BLOB PRIMARY KEY NOT NULL CHECK(length(operation_id) = 16),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 operation_kind TEXT NOT NULL,
                 input_basis BLOB NOT NULL,
                 replay_state TEXT NOT NULL CHECK(replay_state IN ('available','forgotten_dependency')),
                 outcome TEXT NOT NULL CHECK(outcome = 'committed'),
                 result_kind TEXT NOT NULL,
                 result_id BLOB NOT NULL CHECK(length(result_id) = 16),
                 result_revision INTEGER NOT NULL CHECK(result_revision >= 0),
                 committed_at INTEGER NOT NULL,
                 FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
             ) WITHOUT ROWID;
             CREATE TABLE operation_dependencies(
                 operation_id BLOB NOT NULL CHECK(length(operation_id) = 16),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 owner_kind TEXT NOT NULL CHECK(owner_kind IN (
                     'project','source','question','decision','context_item','checkpoint'
                 )),
                 owner_id BLOB NOT NULL CHECK(length(owner_id) = 16),
                 PRIMARY KEY(operation_id, owner_kind, owner_id),
                 FOREIGN KEY(operation_id) REFERENCES operations(operation_id) ON DELETE CASCADE,
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
    replay_state: String,
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
            "SELECT operation_kind, input_basis, replay_state, outcome, result_kind, result_id, result_revision
             FROM operations WHERE operation_id = ?1",
            [operation_id.as_bytes().as_slice()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            },
        )
        .optional()
        .map_err(read_error)?;
    row.map(
        |(kind, input_basis, replay_state, outcome, result_kind, result_id, result_revision)| {
            if outcome != "committed" || result_revision < 0 {
                return Err(Error::new(
                    ErrorKind::RepairRequired,
                    "stored operation outcome is invalid",
                ));
            }
            Ok(StoredOperation {
                kind,
                input_basis,
                replay_state,
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
    if operation.replay_state == "forgotten_dependency" {
        return Err(Error::new(
            ErrorKind::NotFound,
            "operation replay input depended on forgotten canonical content",
        ));
    }
    if operation.replay_state != "available" {
        return Err(Error::new(
            ErrorKind::RepairRequired,
            "stored operation replay state is invalid",
        ));
    }
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
        "create_question" => "question",
        "record_question_response" => "question_response",
        "record_context_item" => "context_item",
        "record_checkpoint" => "checkpoint",
        "correct_context_item" => "context_item",
        "correct_decision" | "supersede_decision" => "decision",
        "record_contradiction" => "canonical_relation",
        "mark_decision_review_due" => "review_due",
        "forget_canonical_record" => "tombstone",
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
pub(crate) fn record_operation(
    transaction: &Transaction<'_>,
    operation_id: OperationId,
    project_id: ProjectId,
    operation_kind: &str,
    input_basis: &[u8],
    result_kind: &str,
    result_id: &[u8; 16],
    result_revision: u64,
    committed_at: TimestampMicros,
    dependencies: &[CanonicalRecordId],
) -> Result<(), Error> {
    if dependencies.is_empty() {
        return Err(Error::new(
            ErrorKind::RepairRequired,
            "content-bearing operation omitted canonical dependency registration",
        ));
    }
    transaction
        .execute(
            "INSERT INTO operations(
                 operation_id, project_id, operation_kind, input_basis, replay_state, outcome,
                 result_kind, result_id, result_revision, committed_at
             ) VALUES (?1, ?2, ?3, ?4, 'available', 'committed', ?5, ?6, ?7, ?8)",
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
    let mut unique = BTreeSet::new();
    for dependency in dependencies {
        let key = (dependency.kind().as_str(), dependency.as_bytes());
        if !unique.insert(key) {
            continue;
        }
        transaction
            .execute(
                "INSERT INTO operation_dependencies(
                     operation_id, project_id, owner_kind, owner_id
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    operation_id.as_bytes().as_slice(),
                    project_id.as_bytes().as_slice(),
                    dependency.kind().as_str(),
                    dependency.as_bytes().as_slice(),
                ],
            )
            .map_err(write_error)?;
    }
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
    let portable_locator = match &draft.payload {
        SourcePayload::File { locator, .. }
        | SourcePayload::Symbol { locator, .. }
        | SourcePayload::AdoptedArtifact { locator, .. } => Some(locator.as_str()),
        _ => None,
    };
    if portable_locator.is_some_and(|locator| Path::new(locator).is_absolute()) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "portable Source locator must not be a local absolute path",
        ));
    }
    Ok(())
}

fn validate_portable_paths(label: &str, values: &[String]) -> Result<(), Error> {
    if values.iter().any(|value| Path::new(value).is_absolute()) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must not contain a local absolute path"),
        ));
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
    match &error {
        rusqlite::Error::SqliteFailure(inner, _) if matches!(inner.extended_code, 1555 | 2067) => {
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
    use super::{begin_write, record_operation, Store};
    use crate::{ErrorKind, OperationId, TimestampMicros};

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

    #[test]
    fn content_bearing_operation_cannot_omit_dependency_registration(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let mut store = Store::open(root.path().join("context.sqlite3"))?;
        let project = store
            .create_project(OperationId::from_bytes([1; 16]), "Dependency guard")?
            .value;
        let transaction = begin_write(&mut store.connection)?;
        let error = record_operation(
            &transaction,
            OperationId::from_bytes([2; 16]),
            project.id,
            "new_content_operation",
            b"content-bearing-basis",
            "project",
            project.id.as_bytes(),
            1,
            TimestampMicros::from_unix_micros(1),
            &[],
        )
        .err()
        .ok_or("operation without dependencies was accepted")?;
        assert_eq!(error.kind(), ErrorKind::RepairRequired);
        let count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM operations WHERE operation_id = ?1",
            [OperationId::from_bytes([2; 16]).as_bytes().as_slice()],
            |row| row.get(0),
        )?;
        assert_eq!(count, 0);
        transaction.rollback()?;
        Ok(())
    }
}
