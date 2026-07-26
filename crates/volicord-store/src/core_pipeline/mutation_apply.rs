use super::artifacts::artifact_staging_record_tx;
use super::user_actions::{
    user_action_request_record, validate_user_action_request_resolution_pair,
};
use super::*;
use crate::evidence_capture::{
    validate_evidence_capture_intent_window, EvidenceCaptureIntentWindowError,
};
use crate::workflow_records::{
    apply_project_workflow_policy_mutation, clear_satisfied_task_policy_reevaluation,
    project_write_authority_fingerprint, ProjectWorkflowPolicyMutationEffect,
};
use volicord_types::schema::WriteTicketValidityBasis;

fn task_control_level_rank(value: &str) -> StoreResult<u8> {
    match value {
        "observe" => Ok(0),
        "light" => Ok(1),
        "tracked" => Ok(2),
        "sensitive" => Ok(3),
        _ => Err(StoreError::InvalidInput {
            detail: "effective_control_level is not supported".to_owned(),
        }),
    }
}

fn acceptance_policy_rank(value: &str) -> StoreResult<u8> {
    match value {
        "not_required" => Ok(0),
        "policy_dependent" => Ok(1),
        "required" => Ok(2),
        _ => Err(StoreError::InvalidInput {
            detail: "acceptance_policy is not supported".to_owned(),
        }),
    }
}

fn validate_write_ticket_invalidation_reason(value: &str) -> StoreResult<()> {
    if matches!(
        value,
        "scope_revision_changed"
            | "change_unit_changed"
            | "baseline_changed"
            | "workspace_changed"
            | "approval_basis_changed"
            | "idle_timeout"
            | "task_closed"
            | "explicit_revoke"
    ) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "write-ticket invalidation reason is not supported".to_owned(),
        })
    }
}

impl CoreStorageMutation {
    /// Applies this storage mutation inside the active Core commit transaction.
    pub fn apply(
        &self,
        mutation: &mut ProjectMutation<'_>,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        match self {
            Self::InsertTask(input) => mutation.insert_task(input),
            Self::SetActiveTask { task_id } => mutation.set_active_task(task_id),
            Self::SupersedeTask { task_id } => mutation.supersede_task(task_id),
            Self::CloseTask(input) => mutation.close_task(input),
            Self::UpdateTaskControlLevel(input) => mutation.update_task_control_level(input),
            Self::UpdateTaskScope(input) => mutation.update_task_scope(input),
            Self::UpdateTaskScopeRevision(input) => mutation.update_task_scope_revision(input),
            Self::UpdateTaskCloseBasis(input) => mutation.update_task_close_basis(input),
            Self::ReplaceAcceptanceCriteria(input) => mutation.replace_acceptance_criteria(input),
            Self::EnsureEvidenceClaim(input) => mutation.ensure_evidence_claim(input),
            Self::InsertCurrentChangeUnit(input) => {
                mutation.insert_current_change_unit(input, committed_state_version)
            }
            Self::ReplaceCurrentChangeUnit(input) => {
                mutation.replace_current_change_unit(input, committed_state_version)
            }
            Self::MarkActiveWriteTicketsStale { task_id } => {
                mutation.mark_active_write_tickets_stale(task_id)
            }
            Self::InvalidateActiveWriteTickets(input) => {
                mutation.invalidate_active_write_tickets(input)
            }
            Self::InvalidateWriteTicket(input) => mutation.invalidate_write_ticket(input),
            Self::InsertWriteTicket(input) => {
                mutation.insert_write_ticket(input, committed_state_version)
            }
            Self::ConsumeWriteTicket(input) => mutation.consume_write_ticket(input),
            Self::InsertRun(input) => mutation.insert_run(input),
            Self::InsertEvidenceCaptureIntent(input) => {
                mutation.insert_evidence_capture_intent(input)
            }
            Self::PromoteStagedArtifact(input) => mutation.promote_staged_artifact(input),
            Self::LinkArtifact(input) => mutation.link_artifact(input),
            Self::UpsertEvidenceSummary(input) => {
                mutation.upsert_evidence_summary(input, committed_state_version)
            }
            Self::InsertEvidenceObservation(input) => mutation.insert_evidence_observation(input),
            Self::InsertEvidenceProducer(input) => mutation.insert_evidence_producer(input),
            Self::InsertUserActionRequest(input) => mutation.insert_user_action_request(input),
            Self::InsertUserActionResolution(input) => {
                mutation.insert_user_action_resolution(input)
            }
            Self::ResolveUnrecordedChange(input) => mutation.resolve_unrecorded_change(input),
            Self::InsertProjectContinuityRecord(input) => {
                mutation.insert_project_continuity_record(input)
            }
            Self::UpdateUserActionBasis(input) => mutation.update_user_action_basis(input),
            Self::MarkUserActionBasesStatus(input) => mutation.mark_user_action_bases_status(input),
            Self::MarkUserActionsSupersededOrStale(input) => {
                mutation.mark_user_actions_superseded_or_stale(input)
            }
            Self::ApplyProjectWorkflowPolicy(input) => mutation
                .apply_project_workflow_policy_with_effect(input)
                .map(|_| ()),
        }
    }
}

impl ProjectMutation<'_> {
    pub(crate) fn apply_project_workflow_policy_with_effect(
        &mut self,
        input: &ProjectWorkflowPolicyMutation,
    ) -> StoreResult<ProjectWorkflowPolicyMutationEffect> {
        apply_project_workflow_policy_mutation(self.tx, self.project_id, self.committed_at, input)
    }

    fn insert_task(&mut self, input: &TaskInsert) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("created_by_actor_source", &input.created_by_actor_source)?;
        validate_identifier("mode", &input.mode)?;
        if !matches!(
            input.requested_control_level.as_str(),
            "auto" | "observe" | "light" | "tracked" | "sensitive"
        ) {
            return Err(StoreError::InvalidInput {
                detail: "requested_control_level is not supported".to_owned(),
            });
        }
        if !matches!(
            input.effective_control_level.as_str(),
            "observe" | "light" | "tracked" | "sensitive"
        ) {
            return Err(StoreError::InvalidInput {
                detail: "effective_control_level is not supported".to_owned(),
            });
        }
        if input.control_level_reason.trim().is_empty() {
            return Err(StoreError::InvalidInput {
                detail: "control_level_reason must not be empty".to_owned(),
            });
        }
        validate_identifier("work_phase", &input.work_phase)?;
        validate_identifier("acceptance_policy", &input.acceptance_policy)?;
        if input.acceptance_policy_reason.trim().is_empty() {
            return Err(StoreError::schema_invariant(
                "project_state",
                "Task acceptance policy reason must not be empty",
            ));
        }
        validate_json_text("tasks.carry_forward_json", &input.carry_forward_json)?;
        validate_identifier("lifecycle_phase", &input.lifecycle_phase)?;
        validate_json_text("tasks.shaping_summary_json", &input.shaping_summary_json)?;
        validate_json_text("tasks.bounded_context_json", &input.bounded_context_json)?;
        validate_json_text(
            "tasks.autonomy_boundary_json",
            &input.autonomy_boundary_json,
        )?;
        validate_persisted_close_summary_json(
            "tasks.close_summary_json",
            &input.close_summary_json,
        )?;
        self.tx.execute(
            "INSERT INTO tasks (
                project_id,
                task_id,
                created_by_actor_source,
                mode,
                requested_control_level,
                effective_control_level,
                control_level_reason,
                work_phase,
                acceptance_policy,
                acceptance_policy_reason,
                predecessor_task_id,
                lineage_relation,
                lineage_reason,
                carry_forward_json,
                lifecycle_phase,
                result,
                title,
                summary,
                shaping_summary_json,
                bounded_context_json,
                autonomy_boundary_json,
                close_summary_json,
                current_change_unit_id,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23,
                ?24,
                ?24
            )",
            params![
                self.project_id,
                input.task_id,
                input.created_by_actor_source,
                input.mode,
                input.requested_control_level,
                input.effective_control_level,
                input.control_level_reason,
                input.work_phase,
                input.acceptance_policy,
                input.acceptance_policy_reason,
                input.predecessor_task_id,
                input.lineage_relation,
                input.lineage_reason,
                input.carry_forward_json,
                input.lifecycle_phase,
                input.result,
                input.title,
                input.summary,
                input.shaping_summary_json,
                input.bounded_context_json,
                input.autonomy_boundary_json,
                input.close_summary_json,
                input.current_change_unit_id,
                self.committed_at
            ],
        )?;
        Ok(())
    }

    fn update_task_control_level(&mut self, input: &TaskControlLevelUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        let requested_rank = task_control_level_rank(&input.effective_control_level)?;
        if input.control_level_reason.trim().is_empty() {
            return Err(StoreError::InvalidInput {
                detail: "control_level_reason must not be empty".to_owned(),
            });
        }
        let acceptance_update = match (
            input.acceptance_policy.as_deref(),
            input.acceptance_policy_reason.as_deref(),
        ) {
            (None, None) => None,
            (Some(policy), Some(reason)) if !reason.trim().is_empty() => {
                Some((policy, reason, acceptance_policy_rank(policy)?))
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: "acceptance_policy and acceptance_policy_reason must be supplied together with a non-empty reason".to_owned(),
                })
            }
        };
        let (current_level, current_acceptance_policy, metadata_json) = self
            .tx
            .query_row(
                "SELECT effective_control_level, acceptance_policy, metadata_json
                   FROM tasks
                  WHERE project_id = ?1
                    AND task_id = ?2",
                params![self.project_id, input.task_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound {
                entity: "task",
                id: input.task_id.clone(),
            })?;
        if requested_rank < task_control_level_rank(&current_level)? {
            return Err(StoreError::Conflict {
                entity: "task",
                id: input.task_id.clone(),
                detail: "effective Task control level cannot decrease".to_owned(),
            });
        }
        if let Some((_, _, requested_acceptance_rank)) = &acceptance_update {
            if *requested_acceptance_rank < acceptance_policy_rank(&current_acceptance_policy)? {
                return Err(StoreError::Conflict {
                    entity: "task",
                    id: input.task_id.clone(),
                    detail: "Task acceptance policy cannot decrease".to_owned(),
                });
            }
        }
        let (acceptance_policy, acceptance_policy_reason) = acceptance_update
            .map(|(policy, reason, _)| (Some(policy), Some(reason)))
            .unwrap_or((None, None));
        let metadata_json = clear_satisfied_task_policy_reevaluation(
            &metadata_json,
            &input.task_id,
            &input.effective_control_level,
            acceptance_policy.unwrap_or(&current_acceptance_policy),
        )?;
        self.tx.execute(
            "UPDATE tasks
                SET effective_control_level = ?3,
                    control_level_reason = ?4,
                    acceptance_policy = COALESCE(?5, acceptance_policy),
                    acceptance_policy_reason = COALESCE(?6, acceptance_policy_reason),
                    metadata_json = ?7,
                    updated_at = ?8
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                input.effective_control_level,
                input.control_level_reason,
                acceptance_policy,
                acceptance_policy_reason,
                metadata_json,
                self.committed_at
            ],
        )?;
        Ok(())
    }

    fn set_active_task(&mut self, task_id: &str) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        let changed = self.tx.execute(
            "UPDATE project_state
                SET active_task_id = ?2
              WHERE project_id = ?1",
            params![self.project_id, task_id],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "active Task update changed no rows".to_owned(),
            })
        }
    }

    fn supersede_task(&mut self, task_id: &str) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        self.tx.execute(
            "UPDATE tasks
                SET lifecycle_phase = 'superseded',
                    result = 'superseded',
                    close_summary_json = '{\"close_reason\":\"superseded\"}',
                    closed_at = ?3,
                    updated_at = ?3
              WHERE project_id = ?1
                AND task_id = ?2",
            params![self.project_id, task_id, self.committed_at],
        )?;
        Ok(())
    }

    fn close_task(&mut self, input: &TaskCloseUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("lifecycle_phase", &input.lifecycle_phase)?;
        validate_identifier("result", &input.result)?;
        validate_persisted_close_summary_json(
            "tasks.close_summary_json",
            &input.close_summary_json,
        )?;
        validate_timestamp("tasks.closed_at", &input.closed_at)?;

        let changed = self.tx.execute(
            "UPDATE tasks
                SET lifecycle_phase = ?3,
                    result = ?4,
                    close_summary_json = ?5,
                    closed_at = ?6,
                    updated_at = ?7
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                input.lifecycle_phase,
                input.result,
                input.close_summary_json,
                input.closed_at,
                self.committed_at
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "Task close transition changed no rows".to_owned(),
            })
        }
    }

    fn update_task_scope(&mut self, input: &TaskScopeUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        if let Some(value) = &input.shaping_summary_json {
            validate_json_text("tasks.shaping_summary_json", value)?;
            self.update_task_text_column(&input.task_id, "shaping_summary_json", value)?;
        }
        if let Some(value) = &input.bounded_context_json {
            validate_json_text("tasks.bounded_context_json", value)?;
            self.update_task_text_column(&input.task_id, "bounded_context_json", value)?;
        }
        if let Some(value) = &input.autonomy_boundary_json {
            validate_json_text("tasks.autonomy_boundary_json", value)?;
            self.update_task_text_column(&input.task_id, "autonomy_boundary_json", value)?;
        }
        if let Some(value) = &input.close_summary_json {
            validate_persisted_close_summary_json("tasks.close_summary_json", value)?;
            self.update_task_text_column(&input.task_id, "close_summary_json", value)?;
        }
        if let Some(value) = &input.lifecycle_phase {
            validate_identifier("lifecycle_phase", value)?;
            self.update_task_text_column(&input.task_id, "lifecycle_phase", value)?;
        }
        if let Some(value) = &input.work_phase {
            validate_identifier("work_phase", value)?;
            self.update_task_text_column(&input.task_id, "work_phase", value)?;
        }
        if let Some(value) = &input.result {
            validate_identifier("result", value)?;
            self.update_task_text_column(&input.task_id, "result", value)?;
        }
        if let Some(value) = &input.title {
            self.update_task_nullable_text_column(&input.task_id, "title", Some(value))?;
        }
        if let Some(value) = &input.summary {
            self.update_task_nullable_text_column(&input.task_id, "summary", Some(value))?;
        }
        Ok(())
    }

    fn replace_acceptance_criteria(
        &mut self,
        input: &AcceptanceCriteriaReplace,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        let mut ids = Vec::with_capacity(input.criteria.len());
        for criterion in &input.criteria {
            validate_identifier(
                "acceptance_criterion_id",
                &criterion.acceptance_criterion_id,
            )?;
            validate_identifier(
                "acceptance_criteria.evidence_requirement",
                &criterion.evidence_requirement,
            )?;
            if criterion.statement.trim().is_empty() {
                return Err(StoreError::schema_invariant(
                    "project_state",
                    "acceptance criterion statement must not be empty",
                ));
            }
            ids.push(criterion.acceptance_criterion_id.clone());
            self.tx.execute(
                "INSERT INTO acceptance_criteria (
                    project_id,
                    acceptance_criterion_id,
                    task_id,
                    statement,
                    evidence_requirement,
                    position,
                    status,
                    created_at,
                    updated_at,
                    retired_at
                )
                VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, 'active',
                    ?7,
                    ?7,
                    NULL
                )
                ON CONFLICT(project_id, acceptance_criterion_id) DO UPDATE SET
                    statement = excluded.statement,
                    evidence_requirement = excluded.evidence_requirement,
                    position = excluded.position,
                    updated_at = excluded.updated_at
                WHERE acceptance_criteria.task_id = excluded.task_id
                  AND acceptance_criteria.status = 'active'",
                params![
                    self.project_id,
                    criterion.acceptance_criterion_id,
                    input.task_id,
                    criterion.statement,
                    criterion.evidence_requirement,
                    i64::try_from(criterion.position).map_err(|_| StoreError::schema_invariant(
                        "project_state",
                        "acceptance criterion position exceeds SQLite INTEGER range",
                    ))?,
                    self.committed_at,
                ],
            )?;
        }

        if ids.is_empty() {
            self.tx.execute(
                "UPDATE acceptance_criteria
                    SET status = 'retired',
                        retired_at = ?3,
                        updated_at = ?3
                  WHERE project_id = ?1
                    AND task_id = ?2
                    AND status = 'active'",
                params![self.project_id, input.task_id, self.committed_at],
            )?;
        } else {
            let placeholders = (0..ids.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!(
                "UPDATE acceptance_criteria
                    SET status = 'retired',
                        retired_at = ?,
                        updated_at = ?
                  WHERE project_id = ?
                    AND task_id = ?
                    AND status = 'active'
                    AND acceptance_criterion_id NOT IN ({placeholders})"
            );
            let mut values: Vec<&dyn rusqlite::ToSql> = vec![
                &self.committed_at,
                &self.committed_at,
                &self.project_id,
                &input.task_id,
            ];
            values.extend(ids.iter().map(|id| id as &dyn rusqlite::ToSql));
            self.tx.execute(&sql, values.as_slice())?;
        }
        Ok(())
    }

    fn ensure_evidence_claim(&mut self, input: &EvidenceClaimInsert) -> StoreResult<()> {
        validate_identifier("evidence_claim_id", &input.evidence_claim_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if input.statement.trim().is_empty() {
            return Err(StoreError::schema_invariant(
                "project_state",
                "supplemental evidence claim statement must not be empty",
            ));
        }
        self.tx.execute(
            "INSERT OR IGNORE INTO evidence_claims (
                project_id, evidence_claim_id, task_id, statement, created_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5
            )",
            params![
                self.project_id,
                input.evidence_claim_id,
                input.task_id,
                input.statement,
                self.committed_at,
            ],
        )?;
        Ok(())
    }

    fn update_task_scope_revision(&mut self, input: &TaskScopeRevisionUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        let scope_revision = u64_to_i64("tasks.scope_revision", input.scope_revision)?;
        let changed = self.tx.execute(
            "UPDATE tasks
                SET scope_revision = ?3,
                    updated_at = ?4
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                scope_revision,
                self.committed_at
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "Task scope revision update changed no rows".to_owned(),
            })
        }
    }

    fn update_task_close_basis(&mut self, input: &TaskCloseBasisUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        if let Some(value) = &input.close_basis_json {
            validate_current_close_basis_json("tasks.close_basis_json", value)?;
        }
        let close_basis_revision =
            u64_to_i64("tasks.close_basis_revision", input.close_basis_revision)?;
        let changed = self.tx.execute(
            "UPDATE tasks
                SET close_basis_revision = ?3,
                    close_basis_json = ?4,
                    updated_at = ?5
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                close_basis_revision,
                input.close_basis_json,
                self.committed_at
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "Task close-basis update changed no rows".to_owned(),
            })
        }
    }

    fn insert_current_change_unit(
        &mut self,
        input: &ChangeUnitInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        self.insert_change_unit(input, committed_state_version)?;
        self.set_task_current_change_unit(&input.task_id, Some(&input.change_unit_id))
    }

    fn replace_current_change_unit(
        &mut self,
        input: &ChangeUnitInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        self.tx.execute(
            "UPDATE change_units
                SET status = 'replaced',
                    is_current = 0,
                    closed_at = ?3,
                    updated_at = ?3
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'
                AND is_current = 1",
            params![self.project_id, input.task_id, self.committed_at],
        )?;
        self.insert_current_change_unit(input, committed_state_version)
    }

    fn insert_change_unit(
        &mut self,
        input: &ChangeUnitInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("change_unit_id", &input.change_unit_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_json_text("change_units.scope_summary_json", &input.scope_summary_json)?;
        validate_json_text("change_units.bounded_paths_json", &input.bounded_paths_json)?;
        validate_json_text("change_units.write_basis_json", &input.write_basis_json)?;
        validate_effect_contract_json(
            "change_units.effect_contract_json",
            &input.effect_contract_json,
        )?;
        validate_json_text("change_units.lifecycle_json", &input.lifecycle_json)?;
        let basis_state_version = u64_to_i64("basis_state_version", committed_state_version)?;

        self.tx.execute(
            "INSERT INTO change_units (
                project_id,
                change_unit_id,
                task_id,
                status,
                is_current,
                basis_state_version,
                scope_summary_json,
                bounded_paths_json,
                write_basis_json,
                effect_contract_json,
                lifecycle_json,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                'active',
                1,
                ?4,
                ?5,
                ?6,
                ?7,
                ?8,
                ?9,
                ?10,
                ?10
            )",
            params![
                self.project_id,
                input.change_unit_id,
                input.task_id,
                basis_state_version,
                input.scope_summary_json,
                input.bounded_paths_json,
                input.write_basis_json,
                input.effect_contract_json,
                input.lifecycle_json,
                self.committed_at
            ],
        )?;
        Ok(())
    }

    fn set_task_current_change_unit(
        &mut self,
        task_id: &str,
        change_unit_id: Option<&str>,
    ) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        let changed = self.tx.execute(
            "UPDATE tasks
                SET current_change_unit_id = ?3,
                    lifecycle_phase = CASE
                        WHEN ?3 IS NULL THEN lifecycle_phase
                        ELSE 'ready'
                    END,
                    updated_at = ?4
              WHERE project_id = ?1
                AND task_id = ?2",
            params![self.project_id, task_id, change_unit_id, self.committed_at],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "Task current Change Unit update changed no rows".to_owned(),
            })
        }
    }

    fn mark_active_write_tickets_stale(&mut self, task_id: &str) -> StoreResult<()> {
        self.invalidate_active_write_tickets(&WriteTicketInvalidation {
            task_id: task_id.to_owned(),
            invalidation_reason: "scope_revision_changed".to_owned(),
        })
    }

    fn invalidate_active_write_tickets(
        &mut self,
        input: &WriteTicketInvalidation,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_write_ticket_invalidation_reason(&input.invalidation_reason)?;
        self.tx.execute(
            "UPDATE write_tickets
                SET status = 'invalidated',
                    invalidation_reason = ?3
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'",
            params![self.project_id, input.task_id, input.invalidation_reason],
        )?;
        Ok(())
    }

    fn invalidate_write_ticket(&mut self, input: &WriteTicketByIdInvalidation) -> StoreResult<()> {
        validate_identifier("write_ticket_id", &input.write_ticket_id)?;
        validate_write_ticket_invalidation_reason(&input.invalidation_reason)?;
        let changed = self.tx.execute(
            "UPDATE write_tickets
                SET status = 'invalidated',
                    invalidation_reason = ?3
              WHERE project_id = ?1
                AND write_ticket_id = ?2
                AND status = 'active'",
            params![
                self.project_id,
                input.write_ticket_id,
                input.invalidation_reason
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "identified active write ticket invalidation changed no rows".to_owned(),
            })
        }
    }

    fn insert_write_ticket(
        &mut self,
        input: &WriteTicketInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("write_ticket_id", &input.write_ticket_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("change_unit_id", &input.change_unit_id)?;
        validate_json_text(
            "write_tickets.validity_basis_json",
            &input.validity_basis_json,
        )?;
        validate_json_text(
            "write_tickets.allowed_path_prefixes_json",
            &input.allowed_path_prefixes_json,
        )?;
        validate_json_text(
            "write_tickets.denied_path_prefixes_json",
            &input.denied_path_prefixes_json,
        )?;
        validate_json_text(
            "write_tickets.attempt_scope_json",
            &input.attempt_scope_json,
        )?;
        validate_identifier("created_by_actor_source", &input.created_by_actor_source)?;
        if let Some(resolution_id) = &input.created_by_user_action_resolution_id {
            validate_identifier("created_by_user_action_resolution_id", resolution_id)?;
        }
        if let Some(idle_expires_at) = &input.idle_expires_at {
            validate_timestamp("write_tickets.idle_expires_at", idle_expires_at)?;
        }
        validate_timestamp("write_tickets.created_at", &input.created_at)?;
        validate_json_text("write_tickets.metadata_json", &input.metadata_json)?;
        let basis_state_version = u64_to_i64("basis_state_version", committed_state_version)?;

        self.tx.execute(
            "INSERT INTO write_tickets (
                project_id,
                write_ticket_id,
                task_id,
                change_unit_id,
                basis_state_version,
                status,
                validity_basis_json,
                allowed_path_prefixes_json,
                denied_path_prefixes_json,
                attempt_scope_json,
                created_by_actor_source,
                created_by_user_action_resolution_id,
                idle_expires_at,
                invalidation_reason,
                consumed_by_run_id,
                consumed_at,
                revoked_at,
                created_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                'active',
                ?6,
                ?7,
                ?8,
                ?9,
                ?10,
                ?11,
                ?12,
                NULL,
                NULL,
                NULL,
                NULL,
                ?13,
                ?14
            )",
            params![
                self.project_id,
                input.write_ticket_id,
                input.task_id,
                input.change_unit_id,
                basis_state_version,
                input.validity_basis_json,
                input.allowed_path_prefixes_json,
                input.denied_path_prefixes_json,
                input.attempt_scope_json,
                input.created_by_actor_source,
                input.created_by_user_action_resolution_id,
                input.idle_expires_at,
                input.created_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn consume_write_ticket(&mut self, input: &WriteTicketConsumption) -> StoreResult<()> {
        validate_identifier("write_ticket_id", &input.write_ticket_id)?;
        validate_identifier("run_id", &input.run_id)?;
        let (basis_state_version, status, validity_basis_json) = self
            .tx
            .query_row(
                "SELECT basis_state_version, status, validity_basis_json
                   FROM write_tickets
                  WHERE project_id = ?1
                    AND write_ticket_id = ?2",
                params![self.project_id, input.write_ticket_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound {
                entity: "write_ticket",
                id: input.write_ticket_id.clone(),
            })?;
        let basis_state_version =
            nonnegative_i64_to_u64("write_tickets.basis_state_version", basis_state_version)?;
        let validity_basis: WriteTicketValidityBasis = serde_json::from_str(&validity_basis_json)
            .map_err(|_| {
            StoreError::corrupt_owner_state_json(
                "write_tickets",
                &input.write_ticket_id,
                "validity_basis_json",
            )
        })?;
        let policy_json = self
            .tx
            .query_row(
                "SELECT policy_json
                   FROM project_workflow_policies
                  WHERE project_id = ?1",
                [self.project_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let current_write_authority_fingerprint =
            project_write_authority_fingerprint(policy_json.as_deref())?;
        if status != "active"
            || basis_state_version != input.expected_basis_state_version
            || validity_basis.write_authority_fingerprint
                != input.expected_write_authority_fingerprint
            || current_write_authority_fingerprint != input.expected_write_authority_fingerprint
        {
            return Err(StoreError::Conflict {
                entity: "write_ticket",
                id: input.write_ticket_id.clone(),
                detail: "write ticket authority changed before consumption".to_owned(),
            });
        }
        let expected_basis_state_version = u64_to_i64(
            "write_tickets.basis_state_version",
            input.expected_basis_state_version,
        )?;
        let changed = self.tx.execute(
            "UPDATE write_tickets
                SET status = 'consumed',
                    consumed_by_run_id = ?3,
                    consumed_at = ?4
              WHERE project_id = ?1
                AND write_ticket_id = ?2
                AND status = 'active'
                AND basis_state_version = ?5",
            params![
                self.project_id,
                input.write_ticket_id,
                input.run_id,
                self.committed_at,
                expected_basis_state_version,
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "active Write Ticket consumption changed no rows".to_owned(),
            })
        }
    }

    fn insert_run(&mut self, input: &RunInsert) -> StoreResult<()> {
        validate_identifier("run_id", &input.run_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        let scope_revision = u64_to_i64("runs.scope_revision", input.scope_revision)?;
        if let Some(write_ticket_id) = &input.write_ticket_id {
            validate_identifier("write_ticket_id", write_ticket_id)?;
        }
        validate_identifier("runs.kind", &input.kind)?;
        validate_identifier("runs.status", &input.status)?;
        validate_json_text("runs.summary_json", &input.summary_json)?;
        validate_json_text("runs.observed_changes_json", &input.observed_changes_json)?;
        validate_json_text("runs.evidence_updates_json", &input.evidence_updates_json)?;
        validate_json_text(
            "runs.write_ticket_effect_json",
            &input.write_ticket_effect_json,
        )?;
        validate_identifier("created_by_actor_source", &input.created_by_actor_source)?;
        validate_json_text("runs.metadata_json", &input.metadata_json)?;

        self.tx.execute(
            "INSERT INTO runs (
                project_id,
                run_id,
                task_id,
                change_unit_id,
                scope_revision,
                write_ticket_id,
                kind,
                status,
                summary_json,
                observed_changes_json,
                evidence_updates_json,
                write_ticket_effect_json,
                created_by_actor_source,
                started_at,
                completed_at,
                created_at,
                metadata_json
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
                ?14,
                ?14,
                ?14,
                ?15
            )",
            params![
                self.project_id,
                input.run_id,
                input.task_id,
                input.change_unit_id,
                scope_revision,
                input.write_ticket_id,
                input.kind,
                input.status,
                input.summary_json,
                input.observed_changes_json,
                input.evidence_updates_json,
                input.write_ticket_effect_json,
                input.created_by_actor_source,
                self.committed_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn insert_evidence_capture_intent(
        &mut self,
        input: &EvidenceCaptureIntentInsert,
    ) -> StoreResult<()> {
        validate_identifier(
            "evidence_capture_intent_id",
            &input.evidence_capture_intent_id,
        )?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("change_unit_id", &input.change_unit_id)?;
        validate_identifier("baseline_ref", &input.baseline_ref)?;
        validate_evidence_capture_kind("capture_kind", &input.capture_kind)?;
        validate_artifact_sha256("input_sha256", &input.input_sha256)?;
        validate_identifier(
            "requested_by_actor_source",
            &input.requested_by_actor_source,
        )?;
        validate_identifier(
            "requesting_connection_internal_id",
            &input.requesting_connection_internal_id,
        )?;
        for (field, value) in [
            ("target_json", input.target_json.as_str()),
            ("capture_spec_json", input.capture_spec_json.as_str()),
            (
                "expected_outcome_json",
                input.expected_outcome_json.as_str(),
            ),
            ("session_context_json", input.session_context_json.as_str()),
            (
                "workspace_context_json",
                input.workspace_context_json.as_str(),
            ),
            ("metadata_json", input.metadata_json.as_str()),
        ] {
            validate_json_text(field, value)?;
        }
        validate_evidence_capture_intent_window(&input.created_at, &input.expires_at).map_err(
            |field| {
                StoreError::schema_invariant(
                    "project_state",
                    match field {
                        EvidenceCaptureIntentWindowError::CreatedAt => {
                            "invalid capture-intent created_at"
                        }
                        EvidenceCaptureIntentWindowError::ExpiresAt => {
                            "capture-intent expires_at must be exactly 15 minutes after created_at"
                        }
                    },
                )
            },
        )?;

        self.tx.execute(
            "INSERT INTO evidence_capture_intents (
                project_id,
                evidence_capture_intent_id,
                task_id,
                change_unit_id,
                scope_revision,
                baseline_ref,
                target_json,
                capture_kind,
                capture_spec_json,
                input_sha256,
                expected_outcome_json,
                requested_by_actor_source,
                requesting_connection_internal_id,
                session_context_json,
                workspace_context_json,
                created_at,
                expires_at,
                metadata_json
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18
            )",
            params![
                self.project_id,
                input.evidence_capture_intent_id,
                input.task_id,
                input.change_unit_id,
                u64_to_i64(
                    "evidence_capture_intents.scope_revision",
                    input.scope_revision
                )?,
                input.baseline_ref,
                input.target_json,
                input.capture_kind,
                input.capture_spec_json,
                input.input_sha256,
                input.expected_outcome_json,
                input.requested_by_actor_source,
                input.requesting_connection_internal_id,
                input.session_context_json,
                input.workspace_context_json,
                input.created_at,
                input.expires_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn promote_staged_artifact(&mut self, input: &ArtifactPromotion) -> StoreResult<()> {
        validate_identifier("artifact_staging.handle_id", &input.handle_id)?;
        validate_identifier("artifact_id", &input.artifact_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("run_id", &input.run_id)?;
        validate_identifier(
            "expected_created_by_actor_source",
            &input.expected_created_by_actor_source,
        )?;
        validate_artifact_sha256("expected_sha256", &input.expected_sha256)?;
        validate_identifier("expected_redaction_state", &input.expected_redaction_state)?;
        validate_timestamp("expected_created_at", &input.expected_created_at)?;
        validate_timestamp("expected_expires_at", &input.expected_expires_at)?;
        validate_identifier("artifacts.uri", &input.uri)?;
        validate_json_text("artifacts.retention_json", &input.retention_json)?;
        validate_artifact_producer_json("artifacts.producer_json", &input.producer_json)?;
        validate_artifact_provenance_metadata_json(
            "artifacts.metadata_json",
            &input.metadata_json,
        )?;

        let staging = artifact_staging_record_tx(self.tx, self.project_id, &input.handle_id)?
            .ok_or_else(|| StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact disappeared before promotion".to_owned(),
            })?;
        if staging.task_id != input.task_id
            || staging.created_by_actor_source != input.expected_created_by_actor_source
            || staging.status != "staged"
            || staging.sha256.as_deref() != Some(input.expected_sha256.as_str())
            || staging.size_bytes != Some(input.expected_size_bytes)
            || staging.redaction_state != input.expected_redaction_state
            || staging.created_at != input.expected_created_at
            || staging.expires_at != input.expected_expires_at
        {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact changed before promotion".to_owned(),
            });
        }
        let created_at = UtcTimestamp::parse(&staging.created_at).map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "artifact_staging",
                &staging.handle_id,
                "created_at",
            )
        })?;
        let expires_at = UtcTimestamp::parse(&staging.expires_at).map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "artifact_staging",
                &staging.handle_id,
                "expires_at",
            )
        })?;
        let committed_at = UtcTimestamp::parse(self.committed_at).map_err(|_| {
            StoreError::corrupt_owner_state_value("project_state", self.project_id, "updated_at")
        })?;
        if committed_at < created_at || committed_at >= expires_at {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact is outside its exact eligibility window".to_owned(),
            });
        }
        let staging_tmp_path =
            staging
                .tmp_path
                .as_deref()
                .ok_or_else(|| StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: "staged artifact body path is missing before promotion".to_owned(),
                })?;
        let body_path = persistent_body_path_from_staging_tmp_path(staging_tmp_path)?;
        verify_staged_artifact_body(
            self.project_home,
            Some(staging_tmp_path),
            &input.expected_sha256,
            input.expected_size_bytes,
        )?;

        let size_bytes = u64_to_i64("artifacts.size_bytes", input.expected_size_bytes)?;
        self.tx.execute(
            "INSERT INTO artifacts (
                project_id,
                artifact_id,
                task_id,
                producer_run_id,
                source_staging_handle_id,
                uri,
                body_path,
                sha256,
                size_bytes,
                content_type,
                integrity_status,
                redaction_state,
                status,
                retention_json,
                producer_json,
                created_at,
                updated_at,
                metadata_json
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
                'verified',
                ?11,
                'available',
                ?12,
                ?13,
                ?14,
                ?14,
                ?15
            )",
            params![
                self.project_id,
                input.artifact_id,
                input.task_id,
                input.run_id,
                input.handle_id,
                input.uri,
                body_path,
                input.expected_sha256,
                size_bytes,
                staging.content_type,
                input.expected_redaction_state,
                input.retention_json,
                input.producer_json,
                self.committed_at,
                input.metadata_json
            ],
        )?;

        let changed = self.tx.execute(
            "UPDATE artifact_staging
                SET status = 'consumed',
                    consumed_by_run_id = ?3,
                    promoted_artifact_id = ?4,
                    consumed_at = ?5
              WHERE project_id = ?1
                AND handle_id = ?2
                AND status = 'staged'",
            params![
                self.project_id,
                input.handle_id,
                input.run_id,
                input.artifact_id,
                self.committed_at
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact consumption changed no rows".to_owned(),
            })
        }
    }

    fn link_artifact(&mut self, input: &ArtifactLinkInsert) -> StoreResult<()> {
        validate_identifier("artifact_id", &input.artifact_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("owner_record_kind", &input.owner_record_kind)?;
        validate_identifier("owner_record_id", &input.owner_record_id)?;
        validate_identifier("created_by_run_id", &input.created_by_run_id)?;
        validate_json_text("artifact_links.metadata_json", &input.metadata_json)?;

        self.tx.execute(
            "INSERT OR IGNORE INTO artifact_links (
                project_id,
                artifact_id,
                task_id,
                owner_record_kind,
                owner_record_id,
                created_by_run_id,
                created_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                ?7,
                ?8
            )",
            params![
                self.project_id,
                input.artifact_id,
                input.task_id,
                input.owner_record_kind,
                input.owner_record_id,
                input.created_by_run_id,
                self.committed_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn upsert_evidence_summary(
        &mut self,
        input: &EvidenceSummaryUpsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("evidence_summary_id", &input.evidence_summary_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        validate_identifier("evidence_summaries.status", &input.status)?;
        validate_evidence_coverage_json("evidence_summaries.coverage_json", &input.coverage_json)?;
        validate_state_refs_json(
            "evidence_summaries.supporting_refs_json",
            &input.supporting_refs_json,
        )?;
        validate_state_refs_json("evidence_summaries.gap_refs_json", &input.gap_refs_json)?;
        validate_evidence_metadata_json("evidence_summaries.metadata_json", &input.metadata_json)?;
        let produced_at_state_version = u64_to_i64(
            "evidence_summaries.produced_at_state_version",
            committed_state_version,
        )?;

        self.tx.execute(
            "INSERT INTO evidence_summaries (
                project_id,
                evidence_summary_id,
                task_id,
                change_unit_id,
                produced_at_state_version,
                status,
                coverage_json,
                supporting_refs_json,
                gap_refs_json,
                created_at,
                updated_at,
                metadata_json
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
                ?10,
                ?11
            )
            ON CONFLICT(project_id, evidence_summary_id) DO UPDATE SET
                task_id = excluded.task_id,
                change_unit_id = excluded.change_unit_id,
                produced_at_state_version = excluded.produced_at_state_version,
                status = excluded.status,
                coverage_json = excluded.coverage_json,
                supporting_refs_json = excluded.supporting_refs_json,
                gap_refs_json = excluded.gap_refs_json,
                updated_at = excluded.updated_at,
                metadata_json = excluded.metadata_json",
            params![
                self.project_id,
                input.evidence_summary_id,
                input.task_id,
                input.change_unit_id,
                produced_at_state_version,
                input.status,
                input.coverage_json,
                input.supporting_refs_json,
                input.gap_refs_json,
                self.committed_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn insert_evidence_observation(
        &mut self,
        input: &EvidenceObservationInsert,
    ) -> StoreResult<()> {
        validate_identifier("evidence_observation_id", &input.evidence_observation_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        if let Some(run_id) = &input.run_id {
            validate_identifier("run_id", run_id)?;
        }
        if input.acceptance_criterion_id.is_some() == input.evidence_claim_id.is_some() {
            return Err(StoreError::schema_invariant(
                "project_state",
                "evidence observation must select exactly one target identity",
            ));
        }
        validate_evidence_source_kind("evidence_observations.source_kind", &input.source_kind)?;
        validate_evidence_assurance_level(
            "evidence_observations.assurance_level",
            &input.assurance_level,
        )?;
        if let Some(actor_source) = &input.observed_by_actor_source {
            validate_identifier("observed_by_actor_source", actor_source)?;
        }
        if let Some(tool_name) = &input.tool_name {
            validate_identifier("tool_name", tool_name)?;
        }
        if let Some(tool_invocation_id) = &input.tool_invocation_id {
            validate_identifier("tool_invocation_id", tool_invocation_id)?;
        }
        validate_evidence_observation_tool_metadata_json(
            "evidence_observations.tool_metadata_json",
            &input.tool_metadata_json,
        )?;
        validate_state_refs_json(
            "evidence_observations.input_refs_json",
            &input.input_refs_json,
        )?;
        validate_source_refs_json(
            "evidence_observations.source_refs_json",
            &input.source_refs_json,
        )?;
        validate_artifact_refs_json(
            "evidence_observations.output_artifact_refs_json",
            &input.output_artifact_refs_json,
        )?;
        validate_string_list_json(
            "evidence_observations.limitations_json",
            &input.limitations_json,
        )?;
        validate_timestamp("observed_at", &input.observed_at)?;
        validate_timestamp("recorded_at", &input.recorded_at)?;
        validate_evidence_observation_metadata_json(
            "evidence_observations.metadata_json",
            &input.metadata_json,
        )?;

        self.tx.execute(
            "INSERT INTO evidence_observations (
                project_id,
                evidence_observation_id,
                task_id,
                change_unit_id,
                run_id,
                acceptance_criterion_id,
                evidence_claim_id,
                source_kind,
                assurance_level,
                observed_by_actor_source,
                tool_name,
                tool_invocation_id,
                tool_metadata_json,
                input_refs_json,
                source_refs_json,
                output_artifact_refs_json,
                limitations_json,
                observed_at,
                recorded_at,
                metadata_json
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
                ?14,
                ?15,
                ?16,
                ?17,
                ?18,
                ?19,
                ?20
            )",
            params![
                self.project_id,
                input.evidence_observation_id,
                input.task_id,
                input.change_unit_id,
                input.run_id,
                input.acceptance_criterion_id,
                input.evidence_claim_id,
                input.source_kind,
                input.assurance_level,
                input.observed_by_actor_source,
                input.tool_name,
                input.tool_invocation_id,
                input.tool_metadata_json,
                input.input_refs_json,
                input.source_refs_json,
                input.output_artifact_refs_json,
                input.limitations_json,
                input.observed_at,
                input.recorded_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn insert_evidence_producer(&mut self, input: &EvidenceProducerInsert) -> StoreResult<()> {
        for (field, value) in [
            ("evidence_producer_id", input.evidence_producer_id.as_str()),
            (
                "evidence_capture_intent_id",
                input.evidence_capture_intent_id.as_str(),
            ),
            (
                "evidence_capture_receipt_id",
                input.evidence_capture_receipt_id.as_str(),
            ),
            (
                "evidence_observation_id",
                input.evidence_observation_id.as_str(),
            ),
            ("artifact_id", input.artifact_id.as_str()),
            ("run_id", input.run_id.as_str()),
            ("task_id", input.task_id.as_str()),
            ("change_unit_id", input.change_unit_id.as_str()),
            ("baseline_ref", input.baseline_ref.as_str()),
        ] {
            validate_identifier(field, value)?;
        }
        validate_evidence_capture_kind("producer_kind", &input.producer_kind)?;
        validate_json_text(
            "evidence_producers.canonical_producer_json",
            &input.canonical_producer_json,
        )?;
        validate_timestamp("created_at", &input.created_at)?;
        validate_json_text("evidence_producers.metadata_json", &input.metadata_json)?;

        self.tx.execute(
            "INSERT INTO evidence_producers (
                project_id,
                evidence_producer_id,
                evidence_capture_intent_id,
                evidence_capture_receipt_id,
                evidence_observation_id,
                artifact_id,
                run_id,
                task_id,
                change_unit_id,
                scope_revision,
                baseline_ref,
                producer_kind,
                canonical_producer_json,
                created_at,
                metadata_json
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                ?9, ?10, ?11, ?12, ?13, ?14, ?15
            )",
            params![
                self.project_id,
                input.evidence_producer_id,
                input.evidence_capture_intent_id,
                input.evidence_capture_receipt_id,
                input.evidence_observation_id,
                input.artifact_id,
                input.run_id,
                input.task_id,
                input.change_unit_id,
                u64_to_i64("evidence_producers.scope_revision", input.scope_revision)?,
                input.baseline_ref,
                input.producer_kind,
                input.canonical_producer_json,
                input.created_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn insert_user_action_request(&mut self, input: &UserActionRequestInsert) -> StoreResult<()> {
        validate_identifier("user_action_request_id", &input.user_action_request_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        validate_persisted_user_action_request_json(
            "user_action_requests.request_json",
            &input.request_json,
        )?;
        validate_user_action_basis_json("user_action_requests.basis_json", &input.basis_json)?;
        validate_user_action_required_for_json(
            "user_action_requests.required_for_json",
            &input.required_for_json,
        )?;
        validate_identifier(
            "requested_by_actor_source",
            &input.requested_by_actor_source,
        )?;
        validate_timestamp("requested_at", &input.requested_at)?;
        if let Some(expires_at) = &input.expires_at {
            validate_timestamp("expires_at", expires_at)?;
        }
        validate_json_text("user_action_requests.metadata_json", &input.metadata_json)?;
        if input.source_method != MethodName::RequestUserAction.as_str()
            && input.source_method != MethodName::ReconcileChanges.as_str()
        {
            return Err(StoreError::InvalidInput {
                detail: "user-action request source_method is not an allowed creator".to_owned(),
            });
        }
        validate_identifier(
            "user_action_requests.source_idempotency_key",
            &input.source_idempotency_key,
        )?;
        validate_user_action_request_column_agreement(UserActionRequestColumnFacts {
            task_id: &input.task_id,
            change_unit_id: input.change_unit_id.as_deref(),
            request_json: &input.request_json,
            basis_json: &input.basis_json,
            required_for_json: &input.required_for_json,
            requested_at: &input.requested_at,
            expires_at: input.expires_at.as_deref(),
            action_kind: input.action_kind,
            basis_status: input.basis_status,
        })?;

        self.tx.execute(
            "INSERT INTO user_action_requests (
                project_id,
                user_action_request_id,
                task_id,
                change_unit_id,
                action_kind,
                request_json,
                basis_json,
                basis_status,
                required_for_json,
                requested_by_actor_source,
                source_method,
                source_idempotency_key,
                requested_at,
                expires_at,
                metadata_json
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15
            )",
            params![
                self.project_id,
                input.user_action_request_id,
                input.task_id,
                input.change_unit_id,
                user_action_kind_as_str(input.action_kind),
                input.request_json,
                input.basis_json,
                user_action_basis_status_as_str(input.basis_status),
                input.required_for_json,
                input.requested_by_actor_source,
                input.source_method,
                input.source_idempotency_key,
                input.requested_at,
                input.expires_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn insert_user_action_resolution(
        &mut self,
        input: &UserActionResolutionInsert,
    ) -> StoreResult<()> {
        validate_identifier(
            "user_action_resolution_id",
            &input.user_action_resolution_id,
        )?;
        validate_identifier("user_action_request_id", &input.user_action_request_id)?;
        validate_channel_submission_id(&input.channel_submission_id).map_err(|error| {
            StoreError::InvalidInput {
                detail: error.to_string(),
            }
        })?;
        validate_persisted_user_action_resolution_json(
            "user_action_resolutions.resolution_json",
            &input.resolution_json,
        )?;
        if input.resolved_by_actor_source != "local_user" {
            return Err(StoreError::InvalidInput {
                detail: "user-action resolution actor must be local_user".to_owned(),
            });
        }
        validate_identifier(
            "resolved_verification_basis",
            &input.resolved_verification_basis,
        )?;
        validate_identifier("resolved_assurance_level", &input.resolved_assurance_level)?;
        validate_user_action_resolution_provenance(
            input.channel_kind,
            &input.resolved_by_actor_source,
            &input.resolved_verification_basis,
            &input.resolved_assurance_level,
        )?;
        validate_timestamp("resolved_at", &input.resolved_at)?;
        validate_user_action_resolution_column_agreement(
            &input.resolution_json,
            input.action_kind,
            &input.user_action_resolution_id,
        )?;
        if let Some(request) =
            user_action_request_record(self.tx, self.project_id, &input.user_action_request_id)?
        {
            validate_user_action_resolution_timestamp_order_for_insert(
                &request,
                &input.resolved_at,
            )?;
            let candidate = UserActionResolutionRecord {
                project_id: self.project_id.to_owned(),
                user_action_resolution_id: input.user_action_resolution_id.clone(),
                user_action_request_id: input.user_action_request_id.clone(),
                action_kind: input.action_kind,
                channel_kind: input.channel_kind,
                channel_submission_id: input.channel_submission_id.clone(),
                resolution_json: input.resolution_json.clone(),
                resolved_by_actor_source: input.resolved_by_actor_source.clone(),
                resolved_verification_basis: input.resolved_verification_basis.clone(),
                resolved_assurance_level: input.resolved_assurance_level.clone(),
                resolved_at: input.resolved_at.clone(),
            };
            validate_user_action_request_resolution_pair(&request, &candidate).map_err(|_| {
                StoreError::InvalidInput {
                    detail:
                        "user-action resolution must exactly preserve its stored request authority"
                            .to_owned(),
                }
            })?;
        }

        self.tx.execute(
            "INSERT INTO user_action_resolutions (
                project_id,
                user_action_resolution_id,
                user_action_request_id,
                action_kind,
                channel_kind,
                channel_submission_id,
                resolution_json,
                resolved_by_actor_source,
                resolved_verification_basis,
                resolved_assurance_level,
                resolved_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                self.project_id,
                input.user_action_resolution_id,
                input.user_action_request_id,
                user_action_kind_as_str(input.action_kind),
                user_action_channel_kind_as_str(input.channel_kind),
                input.channel_submission_id,
                input.resolution_json,
                input.resolved_by_actor_source,
                input.resolved_verification_basis,
                input.resolved_assurance_level,
                input.resolved_at
            ],
        )?;
        Ok(())
    }

    fn resolve_unrecorded_change(
        &mut self,
        input: &UnrecordedChangeResolutionUpdate,
    ) -> StoreResult<()> {
        validate_identifier("unrecorded_change_id", &input.unrecorded_change_id)?;
        validate_json_text("unrecorded_changes.resolution_json", &input.resolution_json)?;
        validate_timestamp("resolved_at", &input.resolved_at)?;
        validate_identifier("resolved_by_actor_source", &input.resolved_by_actor_source)?;

        let changed = self.tx.execute(
            "UPDATE unrecorded_changes
                SET status = 'resolved',
                    resolution_json = ?3,
                    resolved_at = ?4,
                    resolved_by_actor_source = ?5
              WHERE project_id = ?1
                AND unrecorded_change_id = ?2
                AND status = 'unresolved'",
            params![
                self.project_id,
                input.unrecorded_change_id,
                input.resolution_json,
                input.resolved_at,
                input.resolved_by_actor_source,
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "unresolved unrecorded-change resolution changed no rows".to_owned(),
            })
        }
    }

    fn insert_project_continuity_record(
        &mut self,
        input: &ProjectContinuityRecordInsert,
    ) -> StoreResult<()> {
        validate_identifier("continuity_record_id", &input.continuity_record_id)?;
        validate_identifier("source_task_id", &input.source_task_id)?;
        if let Some(source_change_unit_id) = &input.source_change_unit_id {
            validate_identifier("source_change_unit_id", source_change_unit_id)?;
        }
        validate_project_continuity_kind("project_continuity_records.kind", &input.kind)?;
        validate_nonempty_text("project_continuity_records.title", &input.title)?;
        validate_nonempty_text("project_continuity_records.summary", &input.summary)?;
        if let Some(rationale) = &input.rationale {
            validate_nonempty_text("project_continuity_records.rationale", rationale)?;
        }
        validate_string_list_json(
            "project_continuity_records.applies_to_paths_json",
            &input.applies_to_paths_json,
        )?;
        validate_state_refs_json(
            "project_continuity_records.applies_to_refs_json",
            &input.applies_to_refs_json,
        )?;
        validate_state_refs_json(
            "project_continuity_records.source_refs_json",
            &input.source_refs_json,
        )?;
        validate_artifact_refs_json(
            "project_continuity_records.artifact_refs_json",
            &input.artifact_refs_json,
        )?;
        validate_project_continuity_status("project_continuity_records.status", &input.status)?;
        validate_state_refs_json(
            "project_continuity_records.supersedes_refs_json",
            &input.supersedes_refs_json,
        )?;
        validate_string_list_json(
            "project_continuity_records.review_triggers_json",
            &input.review_triggers_json,
        )?;
        validate_timestamp("project_continuity_records.created_at", &input.created_at)?;
        validate_timestamp("project_continuity_records.updated_at", &input.updated_at)?;
        validate_json_text(
            "project_continuity_records.metadata_json",
            &input.metadata_json,
        )?;

        self.tx.execute(
            "INSERT INTO project_continuity_records (
                project_id,
                continuity_record_id,
                source_task_id,
                source_change_unit_id,
                kind,
                title,
                summary,
                rationale,
                applies_to_paths_json,
                applies_to_refs_json,
                source_refs_json,
                artifact_refs_json,
                status,
                supersedes_refs_json,
                review_triggers_json,
                created_at,
                updated_at,
                metadata_json
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
                ?14,
                ?15,
                ?16,
                ?17,
                ?18
            )",
            params![
                self.project_id,
                input.continuity_record_id,
                input.source_task_id,
                input.source_change_unit_id,
                input.kind,
                input.title,
                input.summary,
                input.rationale,
                input.applies_to_paths_json,
                input.applies_to_refs_json,
                input.source_refs_json,
                input.artifact_refs_json,
                input.status,
                input.supersedes_refs_json,
                input.review_triggers_json,
                input.created_at,
                input.updated_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn update_user_action_basis(&mut self, input: &UserActionBasisUpdate) -> StoreResult<()> {
        validate_identifier("user_action_request_id", &input.user_action_request_id)?;
        validate_user_action_basis_json("user_action_requests.basis_json", &input.basis_json)?;
        let basis_json = user_action_basis_json_with_status(&input.basis_json, input.basis_status)?;
        let changed = self.tx.execute(
            "UPDATE user_action_requests
                SET basis_json = ?3,
                    basis_status = ?4
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![
                self.project_id,
                input.user_action_request_id,
                basis_json,
                user_action_basis_status_as_str(input.basis_status)
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "user-action basis update changed no rows".to_owned(),
            })
        }
    }

    fn mark_user_action_bases_status(
        &mut self,
        input: &UserActionBasisStatusMark,
    ) -> StoreResult<()> {
        let status = match input.basis_status {
            UserActionBasisStatus::Stale | UserActionBasisStatus::Superseded => {
                user_action_basis_status_as_str(input.basis_status)
            }
            UserActionBasisStatus::Current => {
                return Err(StoreError::InvalidInput {
                    detail: "selected user-action bases may only be marked stale or superseded"
                        .to_owned(),
                })
            }
        };

        for request_id in &input.user_action_request_ids {
            validate_identifier("user_action_request_id", request_id)?;
            let basis_json = self
                .tx
                .query_row(
                    "SELECT basis_json FROM user_action_requests
                      WHERE project_id = ?1 AND user_action_request_id = ?2",
                    params![self.project_id, request_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            let Some(basis_json) = basis_json else {
                return Err(StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: "selected user-action basis request does not exist".to_owned(),
                });
            };
            let basis_json = user_action_basis_json_with_status(&basis_json, input.basis_status)?;
            let changed = self.tx.execute(
                "UPDATE user_action_requests
                    SET basis_status = ?3,
                        basis_json = ?4
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                params![self.project_id, request_id, status, basis_json],
            )?;
            if changed != 1 {
                return Err(StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: format!(
                        "selected user-action basis status update changed {changed} rows"
                    ),
                });
            }
        }

        Ok(())
    }

    fn mark_user_actions_superseded_or_stale(
        &mut self,
        input: &UserActionInvalidation,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        if input.action_kinds.is_empty() {
            self.mark_user_actions_superseded_or_stale_for_kind(&input.task_id, None)?;
        } else {
            for action_kind in &input.action_kinds {
                self.mark_user_actions_superseded_or_stale_for_kind(
                    &input.task_id,
                    Some(*action_kind),
                )?;
            }
        }
        Ok(())
    }

    fn mark_user_actions_superseded_or_stale_for_kind(
        &mut self,
        task_id: &str,
        action_kind: Option<UserActionKind>,
    ) -> StoreResult<()> {
        let sql = if action_kind.is_some() {
            "SELECT
                a.user_action_request_id,
                a.basis_json,
                EXISTS (
                  SELECT 1 FROM user_action_resolutions AS r
                   WHERE r.project_id = a.project_id
                     AND r.user_action_request_id = a.user_action_request_id
                )
               FROM user_action_requests AS a
              WHERE a.project_id = ?1
                AND a.task_id = ?2
                AND a.action_kind = ?3
                AND a.basis_status = 'current'"
        } else {
            "SELECT
                a.user_action_request_id,
                a.basis_json,
                EXISTS (
                  SELECT 1 FROM user_action_resolutions AS r
                   WHERE r.project_id = a.project_id
                     AND r.user_action_request_id = a.user_action_request_id
                )
               FROM user_action_requests AS a
              WHERE a.project_id = ?1
                AND a.task_id = ?2
                AND (?3 IS NULL OR a.action_kind = ?3)
                AND a.basis_status = 'current'"
        };
        let kind = action_kind.map(user_action_kind_as_str);
        let rows = {
            let mut stmt = self.tx.prepare(sql)?;
            let mapped = stmt.query_map(params![self.project_id, task_id, kind], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            })?;
            mapped.collect::<Result<Vec<_>, _>>()?
        };
        for (request_id, basis_json, has_resolution) in rows {
            let status = if has_resolution {
                UserActionBasisStatus::Stale
            } else {
                UserActionBasisStatus::Superseded
            };
            let basis_json = user_action_basis_json_with_status(&basis_json, status)?;
            self.tx.execute(
                "UPDATE user_action_requests
                    SET basis_status = ?3,
                        basis_json = ?4
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2",
                params![
                    self.project_id,
                    request_id,
                    user_action_basis_status_as_str(status),
                    basis_json
                ],
            )?;
        }
        Ok(())
    }

    fn update_task_text_column(
        &mut self,
        task_id: &str,
        column: &'static str,
        value: &str,
    ) -> StoreResult<()> {
        let sql = match column {
            "shaping_summary_json" => {
                "UPDATE tasks SET shaping_summary_json = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "bounded_context_json" => {
                "UPDATE tasks SET bounded_context_json = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "autonomy_boundary_json" => {
                "UPDATE tasks SET autonomy_boundary_json = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "close_summary_json" => {
                "UPDATE tasks SET close_summary_json = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "lifecycle_phase" => {
                "UPDATE tasks SET lifecycle_phase = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "work_phase" => {
                "UPDATE tasks SET work_phase = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "result" => {
                "UPDATE tasks SET result = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: format!("unsupported Task text column {column}"),
                })
            }
        };
        let changed = self.tx.execute(
            sql,
            params![self.project_id, task_id, value, self.committed_at],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: format!("Task column {column} update changed no rows"),
            })
        }
    }

    fn update_task_nullable_text_column(
        &mut self,
        task_id: &str,
        column: &'static str,
        value: Option<&str>,
    ) -> StoreResult<()> {
        let sql = match column {
            "title" => {
                "UPDATE tasks SET title = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "summary" => {
                "UPDATE tasks SET summary = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: format!("unsupported nullable Task column {column}"),
                })
            }
        };
        let changed = self.tx.execute(
            sql,
            params![self.project_id, task_id, value, self.committed_at],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: format!("Task column {column} update changed no rows"),
            })
        }
    }
}

fn validate_user_action_resolution_timestamp_order_for_insert(
    request: &UserActionRequestRecord,
    resolved_at: &str,
) -> StoreResult<()> {
    let requested_at = UtcTimestamp::parse(&request.requested_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            &request.user_action_request_id,
            "requested_at",
        )
    })?;
    let expires_at = request
        .expires_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "user_action_requests",
                &request.user_action_request_id,
                "expires_at",
            )
        })?;
    let resolved_at = UtcTimestamp::parse(resolved_at).map_err(|_| StoreError::InvalidInput {
        detail: "user_action_resolutions.resolved_at must be a valid RFC 3339 timestamp".to_owned(),
    })?;
    match validate_user_action_timestamp_order(
        &requested_at,
        expires_at.as_ref(),
        Some(&resolved_at),
    ) {
        Ok(()) => Ok(()),
        Err(UserActionTimestampOrderFailure::ExpiryNotAfterRequest) => {
            Err(StoreError::corrupt_owner_state_value(
                "user_action_requests",
                &request.user_action_request_id,
                "expires_at",
            ))
        }
        Err(UserActionTimestampOrderFailure::ResolutionBeforeRequest) => {
            Err(StoreError::InvalidInput {
                detail: "user_action_resolutions.resolved_at must be at or after user_action_requests.requested_at".to_owned(),
            })
        }
        Err(UserActionTimestampOrderFailure::ResolutionAtOrAfterExpiry) => {
            Err(StoreError::InvalidInput {
                detail: "user_action_resolutions.resolved_at must be before user_action_requests.expires_at".to_owned(),
            })
        }
    }
}

fn validate_evidence_capture_kind(field: &'static str, value: &str) -> StoreResult<()> {
    if matches!(
        value,
        "verified_command_execution"
            | "verified_tool_invocation"
            | "registered_connection_observation"
    ) {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            "project_state",
            format!("{field} is outside the evidence-capture value set"),
        ))
    }
}
