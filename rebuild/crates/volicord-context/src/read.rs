use crate::{
    Availability, CanonicalRecordKind, Checkpoint, ContextItem, DecisionLifecycle, Error,
    ErrorKind, Project, ProjectId, Question, QuestionState, Source, SourceId, SourcePayload, Store,
    TimestampMicros,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CanonicalReadOptions {
    pub include_checkpoint_history: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceFreshness {
    Current,
    Stale,
    Unavailable,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceReadBasis {
    pub source: Source,
    pub snapshot_basis: Option<String>,
    pub availability: Availability,
    pub freshness: SourceFreshness,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalRevisionBasis {
    pub record_kind: CanonicalRecordKind,
    pub record_identity: String,
    pub revisions: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadRelationBasis {
    pub from_kind: String,
    pub from_identity: String,
    pub relation_kind: String,
    pub to_kind: String,
    pub to_identity: String,
    pub recorded_at: TimestampMicros,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForgottenRecordBasis {
    pub record_kind: CanonicalRecordKind,
    pub record_identity: String,
    pub forgotten_at: TimestampMicros,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForgottenCheckpointSourceBasis {
    pub checkpoint_identity: String,
    pub source_identity: String,
    pub semantic_use: String,
    pub position: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeReadBasis {
    pub operation_identity: String,
    pub conflict_set_identity: String,
    pub conflict_revision: u64,
    pub common_base_basis: Option<String>,
    pub local_history_basis: String,
    pub incoming_history_basis: String,
    pub result_history_basis: String,
    pub resolution_kind: String,
    pub resolution_source_identity: Option<String>,
    pub conflict_classes: Vec<String>,
    pub affected_identities: Vec<String>,
    pub unresolved: bool,
    pub branch_history_basis: Option<String>,
    pub committed_at: TimestampMicros,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalReadBasis {
    pub project: Project,
    pub active_questions: Vec<Question>,
    pub terminal_question_history: Vec<Question>,
    pub active_decisions: Vec<DecisionLifecycle>,
    pub superseded_decisions: Vec<DecisionLifecycle>,
    pub context_items: Vec<ContextItem>,
    pub latest_checkpoint: Option<Checkpoint>,
    pub checkpoint_history: Vec<Checkpoint>,
    pub sources: Vec<SourceReadBasis>,
    pub revisions: Vec<CanonicalRevisionBasis>,
    pub relations: Vec<ReadRelationBasis>,
    pub forgotten: Vec<ForgottenRecordBasis>,
    pub forgotten_checkpoint_sources: Vec<ForgottenCheckpointSourceBasis>,
    pub bundle_merges: Vec<MergeReadBasis>,
    pub stable_ordering_identity: Vec<String>,
}

impl Store {
    /// Returns the provider- and analyzer-independent authoritative input for
    /// later Recall projections. All lists use explicit byte-identity or
    /// `(recorded_at, identity)` ordering; local paths and access observations
    /// are neither returned nor used as tie-breakers.
    pub fn read_canonical_basis(
        &self,
        project_id: ProjectId,
        options: CanonicalReadOptions,
    ) -> Result<CanonicalReadBasis, Error> {
        let project = self.get_project(project_id)?;
        let mut active_questions = Vec::new();
        let mut terminal_question_history = Vec::new();
        for id in read_ids(&self.connection, "questions", project_id)? {
            let question = self.get_question(project_id, crate::QuestionId::from_slice(&id)?)?;
            match question.state {
                QuestionState::Open => active_questions.push(question),
                QuestionState::Terminal(_) => terminal_question_history.push(question),
            }
        }

        let mut active_decisions = Vec::new();
        let mut superseded_decisions = Vec::new();
        for id in read_ids(&self.connection, "decisions", project_id)? {
            let lifecycle =
                self.get_decision_lifecycle(project_id, crate::DecisionId::from_slice(&id)?)?;
            if lifecycle.superseded_by.is_some() {
                superseded_decisions.push(lifecycle);
            } else {
                active_decisions.push(lifecycle);
            }
        }

        let mut context_items = Vec::new();
        for id in read_ids(&self.connection, "context_items", project_id)? {
            context_items
                .push(self.get_context_item(project_id, crate::ContextItemId::from_slice(&id)?)?);
        }

        let mut checkpoints = Vec::new();
        let mut statement = self
            .connection
            .prepare("SELECT id FROM checkpoints WHERE project_id = ?1 ORDER BY recorded_at, id")
            .map_err(read_error)?;
        let rows = statement
            .query_map([project_id.as_bytes().as_slice()], |row| {
                row.get::<_, Vec<u8>>(0)
            })
            .map_err(read_error)?;
        for row in rows {
            checkpoints.push(self.get_checkpoint(
                project_id,
                crate::CheckpointId::from_slice(&row.map_err(read_error)?)?,
            )?);
        }
        let checkpoint_ordering_identity = checkpoints
            .iter()
            .map(|value| {
                format!(
                    "checkpoint:{}:{}",
                    value.recorded_at.as_unix_micros(),
                    value.id
                )
            })
            .collect::<Vec<_>>();
        let latest_checkpoint = checkpoints.last().cloned();
        let checkpoint_history = if options.include_checkpoint_history {
            checkpoints
        } else {
            Vec::new()
        };

        let mut sources = Vec::new();
        for id in read_ids(&self.connection, "sources", project_id)? {
            let source = self.get_source(project_id, SourceId::from_slice(&id)?)?;
            sources.push(source_read_basis(source));
        }
        let revisions = read_revisions(&self.connection, project_id)?;
        let relations = read_relations(&self.connection, project_id)?;
        let forgotten = read_tombstones(&self.connection, project_id)?;
        let forgotten_checkpoint_sources =
            read_forgotten_checkpoint_sources(&self.connection, project_id)?;
        let bundle_merges = read_merges(&self.connection, project_id)?;

        let mut stable_ordering_identity = Vec::new();
        stable_ordering_identity.push(format!("project:{}", project.id));
        stable_ordering_identity.extend(
            sources
                .iter()
                .map(|value| format!("source:{}", value.source.id)),
        );
        stable_ordering_identity.extend(
            active_questions
                .iter()
                .map(|value| format!("active_question:{}", value.id)),
        );
        stable_ordering_identity.extend(
            terminal_question_history
                .iter()
                .map(|value| format!("terminal_question:{}", value.id)),
        );
        stable_ordering_identity.extend(
            active_decisions
                .iter()
                .map(|value| format!("active_decision:{}", value.decision.id)),
        );
        stable_ordering_identity.extend(
            superseded_decisions
                .iter()
                .map(|value| format!("superseded_decision:{}", value.decision.id)),
        );
        stable_ordering_identity.extend(
            context_items
                .iter()
                .map(|value| format!("context_item:{}", value.id)),
        );
        stable_ordering_identity.extend(checkpoint_ordering_identity);
        stable_ordering_identity.extend(forgotten.iter().map(|value| {
            format!(
                "forgotten:{}:{}",
                value.record_kind.as_str(),
                value.record_identity
            )
        }));
        stable_ordering_identity.extend(forgotten_checkpoint_sources.iter().map(|value| {
            format!(
                "forgotten_checkpoint_source:{}:{}:{}:{}",
                value.checkpoint_identity,
                value.semantic_use,
                value.position,
                value.source_identity
            )
        }));
        stable_ordering_identity.extend(bundle_merges.iter().map(|value| {
            format!(
                "merge:{}:{}",
                value.conflict_set_identity, value.conflict_revision
            )
        }));

        Ok(CanonicalReadBasis {
            project,
            active_questions,
            terminal_question_history,
            active_decisions,
            superseded_decisions,
            context_items,
            latest_checkpoint,
            checkpoint_history,
            sources,
            revisions,
            relations,
            forgotten,
            forgotten_checkpoint_sources,
            bundle_merges,
            stable_ordering_identity,
        })
    }
}

fn read_forgotten_checkpoint_sources(
    connection: &rusqlite::Connection,
    project_id: ProjectId,
) -> Result<Vec<ForgottenCheckpointSourceBasis>, Error> {
    let mut statement = connection
        .prepare(
            "SELECT checkpoint_id, source_id, semantic_use, position
         FROM checkpoint_forgotten_source_witnesses
         WHERE project_id = ?1
         ORDER BY checkpoint_id, semantic_use, position",
        )
        .map_err(read_error)?;
    let rows = statement
        .query_map([project_id.as_bytes().as_slice()], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(read_error)?;
    let mut values = Vec::new();
    for row in rows {
        let (checkpoint_id, source_id, semantic_use, position) = row.map_err(read_error)?;
        values.push(ForgottenCheckpointSourceBasis {
            checkpoint_identity: hex(&checkpoint_id),
            source_identity: hex(&source_id),
            semantic_use,
            position: u64::try_from(position).map_err(|_| {
                Error::new(
                    ErrorKind::CorruptState,
                    "forgotten Checkpoint Source position is invalid",
                )
            })?,
        });
    }
    Ok(values)
}

fn source_read_basis(source: Source) -> SourceReadBasis {
    let snapshot_basis = match &source.payload {
        SourcePayload::RepositorySnapshot { revision } => Some(revision.clone()),
        SourcePayload::RepositoryCommit { commit } => Some(commit.clone()),
        SourcePayload::File { snapshot, .. } | SourcePayload::Symbol { snapshot, .. } => {
            Some(snapshot.clone())
        }
        SourcePayload::AdoptedArtifact { revision, .. } => Some(revision.clone()),
        SourcePayload::CommandExecution { .. }
        | SourcePayload::CurrentHostUserTurn { .. }
        | SourcePayload::Url { .. } => None,
    };
    let availability = source.availability;
    let freshness = match availability {
        Availability::Available => SourceFreshness::Current,
        Availability::Unavailable => SourceFreshness::Unavailable,
        Availability::Stale => SourceFreshness::Stale,
        Availability::Unknown => SourceFreshness::Unknown,
    };
    SourceReadBasis {
        source,
        snapshot_basis,
        availability,
        freshness,
    }
}

fn read_ids(
    connection: &rusqlite::Connection,
    table: &str,
    project_id: ProjectId,
) -> Result<Vec<Vec<u8>>, Error> {
    let sql = format!("SELECT id FROM {table} WHERE project_id = ?1 ORDER BY id");
    let mut statement = connection.prepare(&sql).map_err(read_error)?;
    let rows = statement
        .query_map([project_id.as_bytes().as_slice()], |row| row.get(0))
        .map_err(read_error)?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(read_error)?);
    }
    Ok(values)
}

fn read_revisions(
    connection: &rusqlite::Connection,
    project_id: ProjectId,
) -> Result<Vec<CanonicalRevisionBasis>, Error> {
    let specs = [
        (
            CanonicalRecordKind::Project,
            "project_revisions",
            "project_id",
        ),
        (CanonicalRecordKind::Source, "sources", "id"),
        (
            CanonicalRecordKind::Question,
            "question_revisions",
            "question_id",
        ),
        (
            CanonicalRecordKind::Decision,
            "decision_revisions",
            "decision_id",
        ),
        (
            CanonicalRecordKind::ContextItem,
            "context_item_revisions",
            "context_item_id",
        ),
        (CanonicalRecordKind::Checkpoint, "checkpoints", "id"),
    ];
    let mut values = Vec::new();
    for (kind, table, id_column) in specs {
        let sql = format!(
            "SELECT {id_column}, revision FROM {table} WHERE project_id = ?1 ORDER BY {id_column}, revision"
        );
        let mut statement = connection.prepare(&sql).map_err(read_error)?;
        let rows = statement
            .query_map([project_id.as_bytes().as_slice()], |row| {
                Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(read_error)?;
        let mut grouped = BTreeMap::<String, Vec<u64>>::new();
        for row in rows {
            let (id, revision) = row.map_err(read_error)?;
            let revision = u64::try_from(revision)
                .map_err(|_| Error::new(ErrorKind::CorruptState, "revision is invalid"))?;
            grouped.entry(hex(&id)).or_default().push(revision);
        }
        values.extend(grouped.into_iter().map(|(record_identity, revisions)| {
            CanonicalRevisionBasis {
                record_kind: kind,
                record_identity,
                revisions,
            }
        }));
    }
    values.sort_by(|left, right| {
        (left.record_kind, &left.record_identity).cmp(&(right.record_kind, &right.record_identity))
    });
    Ok(values)
}

fn read_relations(
    connection: &rusqlite::Connection,
    project_id: ProjectId,
) -> Result<Vec<ReadRelationBasis>, Error> {
    let mut statement = connection
        .prepare(
            "SELECT from_kind, from_id, relation_kind, to_kind, to_id, recorded_at
         FROM canonical_relations WHERE project_id = ?1
         ORDER BY from_kind, from_id, relation_kind, to_kind, to_id",
        )
        .map_err(read_error)?;
    let rows = statement
        .query_map([project_id.as_bytes().as_slice()], |row| {
            Ok(ReadRelationBasis {
                from_kind: row.get(0)?,
                from_identity: hex(&row.get::<_, Vec<u8>>(1)?),
                relation_kind: row.get(2)?,
                to_kind: row.get(3)?,
                to_identity: hex(&row.get::<_, Vec<u8>>(4)?),
                recorded_at: TimestampMicros::from_unix_micros(row.get(5)?),
            })
        })
        .map_err(read_error)?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(read_error)?);
    }
    Ok(values)
}

fn read_tombstones(
    connection: &rusqlite::Connection,
    project_id: ProjectId,
) -> Result<Vec<ForgottenRecordBasis>, Error> {
    let mut statement = connection.prepare(
        "SELECT record_kind, record_id, forgotten_at FROM tombstones WHERE project_id = ?1 ORDER BY record_kind, record_id",
    ).map_err(read_error)?;
    let rows = statement
        .query_map([project_id.as_bytes().as_slice()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(read_error)?;
    let mut values = Vec::new();
    for row in rows {
        let (kind, id, forgotten_at) = row.map_err(read_error)?;
        values.push(ForgottenRecordBasis {
            record_kind: CanonicalRecordKind::parse(&kind)
                .ok_or_else(|| Error::new(ErrorKind::CorruptState, "tombstone kind is invalid"))?,
            record_identity: hex(&id),
            forgotten_at: TimestampMicros::from_unix_micros(forgotten_at),
        });
    }
    Ok(values)
}

fn read_merges(
    connection: &rusqlite::Connection,
    project_id: ProjectId,
) -> Result<Vec<MergeReadBasis>, Error> {
    let mut statement = connection.prepare(
        "SELECT operation_id, conflict_set_id, conflict_revision, common_base_basis, local_history_basis,
                incoming_history_basis, result_history_basis, resolution_kind, resolution_source_id,
                conflict_classes, affected_identities, branch_history_basis, committed_at
         FROM merge_events WHERE project_id = ?1 ORDER BY conflict_set_id, conflict_revision, operation_id",
    ).map_err(read_error)?;
    let rows = statement
        .query_map([project_id.as_bytes().as_slice()], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<Vec<u8>>>(8)?,
                row.get::<_, Vec<u8>>(9)?,
                row.get::<_, Vec<u8>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, i64>(12)?,
            ))
        })
        .map_err(read_error)?;
    let mut values = Vec::new();
    for row in rows {
        let row = row.map_err(read_error)?;
        values.push(MergeReadBasis {
            operation_identity: hex(&row.0),
            conflict_set_identity: row.1,
            conflict_revision: u64::try_from(row.2)
                .map_err(|_| Error::new(ErrorKind::CorruptState, "merge revision is invalid"))?,
            common_base_basis: row.3,
            local_history_basis: row.4,
            incoming_history_basis: row.5,
            result_history_basis: row.6,
            unresolved: row.7 == "unresolved",
            resolution_kind: row.7,
            resolution_source_identity: row.8.map(|value| hex(&value)),
            conflict_classes: decode_strings(&row.9)?,
            affected_identities: decode_strings(&row.10)?,
            branch_history_basis: row.11,
            committed_at: TimestampMicros::from_unix_micros(row.12),
        });
    }
    Ok(values)
}

fn decode_strings(bytes: &[u8]) -> Result<Vec<String>, Error> {
    serde_json::from_slice(bytes).map_err(|error| {
        Error::with_source(
            ErrorKind::CorruptState,
            "stored read basis list is malformed",
            error,
        )
    })
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn read_error(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::CorruptState,
        "cannot read deterministic canonical basis",
        error,
    )
}
