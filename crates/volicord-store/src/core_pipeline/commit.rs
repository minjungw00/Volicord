use rusqlite::{params, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use volicord_types::{IdempotencyKey, MethodName, ProjectId, RequestHash, UtcTimestamp};

use super::{
    project_current_utc_timestamp_for_conn, read_project_state_tx, replay::tool_invocation_tx,
    validation::*, CommitMutationInput, CommittedEventRef, CommittedMutationFacts,
    CoreProjectStore, MutationCommitOutcome, PendingTaskEvent, ProjectMutation,
    VerifiedReplayContext,
};
use crate::{sqlite::begin_immediate_transaction, StoreError, StoreResult};

impl CoreProjectStore {
    /// Commits one state-changing Core mutation or returns replay/conflict outcomes.
    ///
    /// The helper performs replay lookup, stale-state checking, project clock
    /// increment, event append, response construction, and replay-row insertion
    /// in one immediate transaction. Any error rolls back the whole attempt.
    pub fn commit_mutation(
        &mut self,
        input: CommitMutationInput,
        apply_mutation: impl FnOnce(
            &mut ProjectMutation<'_>,
            &CommittedMutationFacts,
        ) -> StoreResult<()>,
        build_response_json: impl FnOnce(CommittedMutationFacts) -> StoreResult<String>,
    ) -> StoreResult<MutationCommitOutcome> {
        if input.project_id != self.project.project_id {
            return Err(StoreError::InvalidInput {
                detail: "commit project_id must match the opened project".to_owned(),
            });
        }
        if input.events.is_empty() {
            return Err(StoreError::InvalidInput {
                detail: "committed Core mutations must append at least one authority event"
                    .to_owned(),
            });
        }
        validate_identifier("tool_name", &input.tool_name)?;
        validate_identifier("request_hash", &input.request_hash)?;
        let replay_context =
            input
                .replay_context
                .as_ref()
                .ok_or_else(|| StoreError::InvalidInput {
                    detail: "committed mutations require verified invocation context".to_owned(),
                })?;
        validate_replay_context(replay_context)?;
        for event in &input.events {
            validate_pending_event(event)?;
        }

        let explicit_clock_floor = input
            .clock_floor
            .as_deref()
            .map(UtcTimestamp::parse)
            .transpose()
            .map_err(|_| StoreError::InvalidInput {
                detail: "commit clock_floor must be a valid RFC 3339 UTC timestamp".to_owned(),
            })?;
        if explicit_clock_floor
            .as_ref()
            .is_some_and(|timestamp| timestamp.ensure_canonical_rfc3339_representable().is_err())
        {
            return Err(StoreError::InvalidInput {
                detail:
                    "commit clock_floor must have a canonical four-digit RFC 3339 representation"
                        .to_owned(),
            });
        }
        let remembered_clock_floor = self.last_clock_sample.borrow().clone();
        if remembered_clock_floor
            .as_ref()
            .is_some_and(|timestamp| timestamp.ensure_canonical_rfc3339_representable().is_err())
        {
            return Err(StoreError::SchemaInvariant {
                database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
                detail: "Core Store handle clock sample is outside the canonical RFC 3339 range"
                    .to_owned(),
            });
        }
        let local_clock_floor = [remembered_clock_floor, explicit_clock_floor]
            .into_iter()
            .flatten()
            .max();
        let tx = begin_immediate_transaction(&mut self.conn)?;
        let current = read_project_state_tx(&tx, &self.project.project_id)?;

        if let Some(idempotency_key) = &input.idempotency_key {
            validate_identifier("idempotency_key", idempotency_key)?;
            if let Some(record) = tool_invocation_tx(
                &tx,
                &self.project.project_id,
                &input.tool_name,
                idempotency_key,
            )? {
                tx.rollback()?;
                let replay_context =
                    input
                        .replay_context
                        .as_ref()
                        .ok_or_else(|| StoreError::InvalidInput {
                            detail: "idempotent commits require verified replay context".to_owned(),
                        })?;
                if !record.matches_verified_replay_context(replay_context) {
                    return Ok(MutationCommitOutcome::ReplayContextMismatch {
                        current_state_version: current.state_version,
                        idempotency_key: idempotency_key.clone(),
                    });
                }
                if record.request_hash == input.request_hash {
                    return Ok(MutationCommitOutcome::Replayed {
                        response_json: record.response_json,
                        basis_state_version: record.basis_state_version,
                        committed_state_version: record.committed_state_version,
                    });
                }

                return Ok(MutationCommitOutcome::IdempotencyConflict {
                    current_state_version: current.state_version,
                    idempotency_key: idempotency_key.clone(),
                    stored_request_hash: record.request_hash,
                    attempted_request_hash: input.request_hash,
                });
            }
        }

        if let Some(expected_state_version) = input.expected_state_version {
            if expected_state_version != current.state_version {
                tx.rollback()?;
                return Ok(MutationCommitOutcome::StaleExpectedState {
                    current_state_version: current.state_version,
                    expected_state_version,
                });
            }
        }

        let committed_state_version =
            current
                .state_version
                .checked_add(1)
                .ok_or_else(|| StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: "project_state.state_version overflow".to_owned(),
                })?;
        let current_state_i64 = u64_to_i64("basis_state_version", current.state_version)?;
        let committed_state_i64 = u64_to_i64("committed_state_version", committed_state_version)?;
        let committed_at = if input.include_live_storage_time {
            project_current_utc_timestamp_for_conn(
                &tx,
                &self.project.project_id,
                local_clock_floor.as_ref(),
            )?
        } else {
            // An injected Core clock replaces the SQLite live candidate, but it
            // remains bounded by the transaction's persisted project floor.
            match local_clock_floor {
                Some(local_clock_floor) => {
                    let persisted_floor =
                        UtcTimestamp::parse(&current.updated_at).map_err(|_| {
                            StoreError::corrupt_owner_state_value(
                                "project_state",
                                &self.project.project_id,
                                "updated_at",
                            )
                        })?;
                    std::cmp::max(persisted_floor, local_clock_floor)
                }
                None => {
                    return Err(StoreError::InvalidInput {
                        detail: "commits without live storage time require an explicit clock floor"
                            .to_owned(),
                    })
                }
            }
        };
        let committed_at_text = committed_at.to_string();

        let changed = tx.execute(
            "UPDATE project_state
                SET state_version = ?3,
                    updated_at = ?4
              WHERE project_id = ?1
                AND state_version = ?2",
            params![
                self.project.project_id,
                current_state_i64,
                committed_state_i64,
                committed_at_text
            ],
        )?;
        if changed != 1 {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "project_state state_version update changed no rows".to_owned(),
            });
        }

        let committed_events = input
            .events
            .iter()
            .map(|event| CommittedEventRef {
                event_id: event.event_id.clone(),
                event_kind: event.event_kind.clone(),
            })
            .collect::<Vec<_>>();
        let facts = CommittedMutationFacts {
            basis_state_version: current.state_version,
            committed_state_version,
            events: committed_events.clone(),
        };
        let mut mutation = ProjectMutation {
            project_id: &self.project.project_id,
            project_home: &self.project.project_home,
            committed_at: &committed_at_text,
            tx: &tx,
        };
        apply_mutation(&mut mutation, &facts)?;

        let first_event_seq = next_event_seq(&tx, &self.project.project_id)?;
        let mut previous_event_hash = previous_event_hash_tx(&tx, &self.project.project_id)?;
        for (index, event) in input.events.iter().enumerate() {
            let event_seq = first_event_seq
                + i64::try_from(index).map_err(|_| StoreError::InvalidInput {
                    detail: "event index does not fit in SQLite integer".to_owned(),
                })?;
            let created_at = committed_at_text.as_str();
            let event_hash = authority_event_hash(AuthorityEventHashInput {
                project_id: &self.project.project_id,
                event_seq,
                event_id: &event.event_id,
                state_version: committed_state_i64,
                event_type: &event.event_kind,
                actor_source: replay_context.actor_source.as_str(),
                operation_category: replay_context.operation_category.as_str(),
                task_id: event.task_id.as_deref(),
                change_unit_id: event.change_unit_id.as_deref(),
                payload_json: &event.event_payload_json,
                request_hash: &input.request_hash,
                previous_event_hash: previous_event_hash.as_deref(),
                created_at,
            });
            tx.execute(
                "INSERT INTO authority_events (
                    project_id,
                    event_seq,
                    event_id,
                    state_version,
                    event_type,
                    actor_source,
                    operation_category,
                    task_id,
                    change_unit_id,
                    payload_json,
                    request_hash,
                    previous_event_hash,
                    event_hash,
                    created_at
                )
                VALUES (
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    ?5,
                    ?6,
                    ?7,
                    ?8,
                    ?9,
                    ?10,
                    ?11,
                    ?12,
                    ?13,
                    ?14
                )",
                params![
                    self.project.project_id,
                    event_seq,
                    event.event_id,
                    committed_state_i64,
                    event.event_kind,
                    replay_context.actor_source.as_str(),
                    replay_context.operation_category.as_str(),
                    event.task_id,
                    event.change_unit_id,
                    event.event_payload_json,
                    input.request_hash,
                    previous_event_hash.as_deref(),
                    event_hash,
                    created_at
                ],
            )?;
            previous_event_hash = Some(event_hash);
        }

        let response_json = build_response_json(facts)?;
        validate_json_text("tool_invocations.response_json", &response_json)?;

        if let Some(idempotency_key) = &input.idempotency_key {
            tx.execute(
                "INSERT INTO tool_invocations (
                    project_id,
                    tool_name,
                    idempotency_key,
                    request_hash,
                    basis_state_version,
                    committed_state_version,
                    actor_source,
                    operation_category,
                    verification_basis,
                    git_workspace_context_json,
                    response_json,
                    created_at
                )
                VALUES (
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    ?5,
                    ?6,
                    ?7,
                    ?8,
                    ?9,
                    ?10,
                    ?11,
                    ?12
                )",
                params![
                    self.project.project_id,
                    input.tool_name,
                    idempotency_key,
                    input.request_hash,
                    current_state_i64,
                    committed_state_i64,
                    replay_context.actor_source.as_str(),
                    replay_context.operation_category.as_str(),
                    replay_context.verification_basis.as_deref(),
                    replay_context.git_workspace_context_json.as_deref(),
                    response_json,
                    committed_at_text
                ],
            )?;
        }

        tx.commit()?;
        *self.last_clock_sample.borrow_mut() = Some(committed_at);
        Ok(MutationCommitOutcome::Committed {
            response_json,
            basis_state_version: current.state_version,
            committed_state_version,
            events: committed_events,
        })
    }
}

/// Builds a commit input from typed public identifiers.
pub fn commit_input(
    project_id: &ProjectId,
    method_name: MethodName,
    idempotency_key: Option<&IdempotencyKey>,
    request_hash: &RequestHash,
    replay_context: Option<VerifiedReplayContext>,
    expected_state_version: Option<u64>,
    events: Vec<PendingTaskEvent>,
) -> CommitMutationInput {
    CommitMutationInput {
        project_id: project_id.as_str().to_owned(),
        tool_name: method_name.as_str().to_owned(),
        idempotency_key: idempotency_key.map(|key| key.as_str().to_owned()),
        request_hash: request_hash.as_str().to_owned(),
        replay_context,
        expected_state_version,
        clock_floor: None,
        include_live_storage_time: true,
        events,
    }
}

fn next_event_seq(tx: &Transaction<'_>, project_id: &str) -> StoreResult<i64> {
    let last_seq: i64 = tx.query_row(
        "SELECT COALESCE(MAX(event_seq), 0)
           FROM authority_events
          WHERE project_id = ?1",
        params![project_id],
        |row| row.get(0),
    )?;
    last_seq
        .checked_add(1)
        .ok_or_else(|| StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "authority_events.event_seq overflow".to_owned(),
        })
}

fn previous_event_hash_tx(tx: &Transaction<'_>, project_id: &str) -> StoreResult<Option<String>> {
    tx.query_row(
        "SELECT event_hash
           FROM authority_events
          WHERE project_id = ?1
          ORDER BY event_seq DESC
          LIMIT 1",
        params![project_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(StoreError::from)
}

struct AuthorityEventHashInput<'a> {
    project_id: &'a str,
    event_seq: i64,
    event_id: &'a str,
    state_version: i64,
    event_type: &'a str,
    actor_source: &'a str,
    operation_category: &'a str,
    task_id: Option<&'a str>,
    change_unit_id: Option<&'a str>,
    payload_json: &'a str,
    request_hash: &'a str,
    previous_event_hash: Option<&'a str>,
    created_at: &'a str,
}

fn authority_event_hash(input: AuthorityEventHashInput<'_>) -> String {
    let mut hasher = Sha256::new();

    fn update_field(hasher: &mut Sha256, field: &str) {
        hasher.update(field.len().to_string().as_bytes());
        hasher.update(b":");
        hasher.update(field.as_bytes());
        hasher.update(b"\n");
    }

    update_field(&mut hasher, input.project_id);
    update_field(&mut hasher, &input.event_seq.to_string());
    update_field(&mut hasher, input.event_id);
    update_field(&mut hasher, &input.state_version.to_string());
    update_field(&mut hasher, input.event_type);
    update_field(&mut hasher, input.actor_source);
    update_field(&mut hasher, input.operation_category);
    update_field(&mut hasher, input.task_id.unwrap_or(""));
    update_field(&mut hasher, input.change_unit_id.unwrap_or(""));
    update_field(&mut hasher, input.payload_json);
    update_field(&mut hasher, input.request_hash);
    update_field(&mut hasher, input.previous_event_hash.unwrap_or(""));
    update_field(&mut hasher, input.created_at);

    format!("sha256:{}", lowercase_hex_bytes(&hasher.finalize()))
}
