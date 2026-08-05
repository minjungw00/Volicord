use rusqlite::{params, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use volicord_types::ids::{IdempotencyKey, ProjectId, RequestHash};
use volicord_types::values::MethodName;

use super::mutations::{AggregateMutationResult, MutationContext};
use super::{
    clock::project_current_utc_timestamp_for_conn, project_state::read_project_state_tx,
    replay::tool_invocation_tx, validation::*, CommitMutationInput, CommittedEventRef,
    CommittedMutationFacts, CoreProjectStore, CoreStorageMutation, MutationCommitOutcome,
    PendingTaskEvent, TransitionCommitExpectation, VerifiedReplayContext,
};
use crate::{sqlite::begin_immediate_transaction, StoreError, StoreResult};

impl CoreProjectStore<'_> {
    /// Applies an ordered batch of responsibility-owned mutations in one Core commit.
    pub fn commit_mutation(
        &mut self,
        input: CommitMutationInput,
        mutations: &[CoreStorageMutation],
        build_response_json: impl FnOnce(CommittedMutationFacts) -> StoreResult<String>,
    ) -> StoreResult<MutationCommitOutcome> {
        self.commit_mutation_with_results(input, mutations, build_response_json)
            .map(|(outcome, _)| outcome)
    }

    pub(crate) fn commit_mutation_with_results(
        &mut self,
        input: CommitMutationInput,
        mutations: &[CoreStorageMutation],
        build_response_json: impl FnOnce(CommittedMutationFacts) -> StoreResult<String>,
    ) -> StoreResult<(MutationCommitOutcome, Vec<AggregateMutationResult>)> {
        validate_transition_expectation(&input, mutations)?;
        let mut mutation_results = Vec::with_capacity(mutations.len());
        let outcome = self.commit_with(
            input,
            |context, facts| {
                for mutation in mutations {
                    mutation_results.push(mutation.apply(context, facts)?);
                }
                Ok(())
            },
            build_response_json,
        )?;
        Ok((outcome, mutation_results))
    }

    /// Commits one state-changing Core mutation or returns replay/conflict outcomes.
    ///
    /// The helper performs replay lookup, stale-state checking, project clock
    /// increment, event append, response construction, and replay-row insertion
    /// in one immediate transaction. Any error rolls back the whole attempt.
    pub(crate) fn commit_with(
        &mut self,
        input: CommitMutationInput,
        apply_mutation: impl FnOnce(
            &mut MutationContext<'_>,
            &CommittedMutationFacts,
        ) -> StoreResult<()>,
        build_response_json: impl FnOnce(CommittedMutationFacts) -> StoreResult<String>,
    ) -> StoreResult<MutationCommitOutcome> {
        self.require_mutation_context()?;
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
        let replay_actor_source = replay_context.actor_source.to_canonical_string();
        let replay_operation_category = replay_context.operation_category.as_str();
        let replay_git_workspace_context_json = replay_context
            .git_workspace_context
            .as_ref()
            .map(volicord_types::canonical::canonical_json_string)
            .transpose()
            .map_err(|_| StoreError::InvalidInput {
                detail: "git_workspace_context cannot be serialized canonically".to_owned(),
            })?;
        for event in &input.events {
            validate_pending_event(event)?;
        }

        let explicit_clock_floor = input.clock_floor;
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
                    let persisted_floor = current.updated_at.clone();
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
        let mut mutation = MutationContext::new(
            &self.project.project_id,
            &self.project.project_home,
            &committed_at_text,
            &tx,
        );
        apply_mutation(&mut mutation, &facts)?;
        if let Some(expectation) = input.transition_expectation.as_ref() {
            validate_transition_result_state(&tx, expectation)?;
        }

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
                actor_source: &replay_actor_source,
                operation_category: replay_operation_category,
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
                    replay_actor_source,
                    replay_operation_category,
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
                    replay_actor_source,
                    replay_operation_category,
                    replay_context.verification_basis.as_deref(),
                    replay_git_workspace_context_json.as_deref(),
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
        transition_expectation: None,
        clock_floor: None,
        include_live_storage_time: true,
        events,
    }
}

fn validate_transition_expectation(
    input: &CommitMutationInput,
    mutations: &[CoreStorageMutation],
) -> StoreResult<()> {
    let Some(expectation) = input.transition_expectation.as_ref() else {
        return Ok(());
    };
    if expectation.project_id != input.project_id
        || expectation.action_key.method.as_str() != input.tool_name
        || input.expected_state_version != Some(expectation.basis_state_version)
        || !input
            .events
            .iter()
            .all(|event| event.task_id.as_deref() == Some(expectation.task_id.as_str()))
    {
        return Err(StoreError::InvalidInput {
            detail: "transition expectation contradicts commit authority coordinates".to_owned(),
        });
    }
    if !super::transition_effect_matches_mutations(
        expectation.action_key,
        expectation.effect_class,
        mutations,
    ) {
        return Err(StoreError::InvalidInput {
            detail: "Store aggregate mutations contradict the admitted transition effect"
                .to_owned(),
        });
    }
    Ok(())
}

fn validate_transition_result_state(
    tx: &Transaction<'_>,
    expectation: &TransitionCommitExpectation,
) -> StoreResult<()> {
    let (work_phase, lifecycle_phase, close_basis_present): (String, String, bool) = tx.query_row(
        "SELECT work_phase, lifecycle_phase, close_basis_json IS NOT NULL
               FROM tasks
              WHERE project_id = ?1 AND task_id = ?2",
        params![expectation.project_id, expectation.task_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let terminal = matches!(
        lifecycle_phase.as_str(),
        "completed" | "cancelled" | "superseded"
    );
    let compatible = match expectation.expected_result_state {
        volicord_types::values::WorkflowExpectedResultState::ReevaluateCurrentAuthority => {
            !terminal
        }
        volicord_types::values::WorkflowExpectedResultState::AwaitingUserAction => {
            !terminal && pending_user_action_exists(tx, expectation)?
        }
        volicord_types::values::WorkflowExpectedResultState::Implementation => {
            !terminal && work_phase == "implementation"
        }
        volicord_types::values::WorkflowExpectedResultState::CloseReview => {
            !terminal && close_basis_present
        }
        volicord_types::values::WorkflowExpectedResultState::Terminal => terminal,
    };
    if !compatible {
        return Err(StoreError::SchemaInvariant {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            detail: "post-mutation Task state contradicts admitted transition result family"
                .to_owned(),
        });
    }
    Ok(())
}

fn pending_user_action_exists(
    tx: &Transaction<'_>,
    expectation: &TransitionCommitExpectation,
) -> StoreResult<bool> {
    tx.query_row(
        "SELECT EXISTS(
             SELECT 1
               FROM user_action_requests request
              WHERE request.project_id = ?1
                AND request.task_id = ?2
                AND request.basis_status = 'current'
                AND NOT EXISTS (
                    SELECT 1 FROM user_action_resolutions resolution
                     WHERE resolution.project_id = request.project_id
                       AND resolution.user_action_request_id = request.user_action_request_id
                )
         )",
        params![expectation.project_id, expectation.task_id],
        |row| row.get(0),
    )
    .map_err(StoreError::from)
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

#[cfg(test)]
mod behavior_tests;
