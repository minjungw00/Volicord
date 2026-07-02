use super::*;

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
            Self::UpdateTaskScope(input) => mutation.update_task_scope(input),
            Self::UpdateTaskScopeRevision(input) => mutation.update_task_scope_revision(input),
            Self::UpdateTaskCloseBasis(input) => mutation.update_task_close_basis(input),
            Self::InsertCurrentChangeUnit(input) => {
                mutation.insert_current_change_unit(input, committed_state_version)
            }
            Self::ReplaceCurrentChangeUnit(input) => {
                mutation.replace_current_change_unit(input, committed_state_version)
            }
            Self::MarkActiveWriteChecksStale { task_id } => {
                mutation.mark_active_write_checks_stale(task_id)
            }
            Self::InsertWriteTicket(input) | Self::InsertWriteCheck(input) => {
                mutation.insert_write_check(input, committed_state_version)
            }
            Self::ConsumeWriteCheck(input) => mutation.consume_write_check(input),
            Self::InsertRun(input) => mutation.insert_run(input),
            Self::PromoteStagedArtifact(input) => mutation.promote_staged_artifact(input),
            Self::LinkArtifact(input) => mutation.link_artifact(input),
            Self::UpsertEvidenceSummary(input) => mutation.upsert_evidence_summary(input),
            Self::InsertEvidenceObservation(input) => mutation.insert_evidence_observation(input),
            Self::InsertUserJudgment(input) => mutation.insert_user_judgment(input),
            Self::ResolveUserJudgment(input) => mutation.resolve_user_judgment(input),
            Self::ConsumeLocalWebConsentToken(input) => {
                mutation.consume_local_web_consent_token(input)
            }
            Self::ResolveUnrecordedChange(input) => mutation.resolve_unrecorded_change(input),
            Self::InsertProjectContinuityRecord(input) => {
                mutation.insert_project_continuity_record(input)
            }
            Self::UpdateUserJudgmentBasis(input) => mutation.update_user_judgment_basis(input),
            Self::MarkUserJudgmentBasesStatus(input) => {
                mutation.mark_user_judgment_bases_status(input)
            }
            Self::MarkUserJudgmentsSupersededOrStale(input) => {
                mutation.mark_user_judgments_superseded_or_stale(input)
            }
        }
    }
}

impl ProjectMutation<'_> {
    fn insert_task(&mut self, input: &TaskInsert) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("created_by_actor_source", &input.created_by_actor_source)?;
        validate_identifier("mode", &input.mode)?;
        validate_identifier("lifecycle_phase", &input.lifecycle_phase)?;
        validate_json_text("tasks.shaping_summary_json", &input.shaping_summary_json)?;
        validate_json_text("tasks.bounded_context_json", &input.bounded_context_json)?;
        validate_json_text(
            "tasks.autonomy_boundary_json",
            &input.autonomy_boundary_json,
        )?;
        validate_json_text("tasks.close_summary_json", &input.close_summary_json)?;
        validate_json_text(
            "tasks.completion_policy_json",
            &input.completion_policy_json,
        )?;

        self.tx.execute(
            "INSERT INTO tasks (
                project_id,
                task_id,
                created_by_actor_source,
                mode,
                lifecycle_phase,
                result,
                title,
                summary,
                shaping_summary_json,
                bounded_context_json,
                autonomy_boundary_json,
                close_summary_json,
                completion_policy_json,
                current_change_unit_id,
                created_at,
                updated_at
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
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )",
            params![
                self.project_id,
                input.task_id,
                input.created_by_actor_source,
                input.mode,
                input.lifecycle_phase,
                input.result,
                input.title,
                input.summary,
                input.shaping_summary_json,
                input.bounded_context_json,
                input.autonomy_boundary_json,
                input.close_summary_json,
                input.completion_policy_json,
                input.current_change_unit_id
            ],
        )?;
        Ok(())
    }

    fn set_active_task(&mut self, task_id: &str) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        let changed = self.tx.execute(
            "UPDATE project_state
                SET active_task_id = ?2,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
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
                    closed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND task_id = ?2",
            params![self.project_id, task_id],
        )?;
        Ok(())
    }

    fn close_task(&mut self, input: &TaskCloseUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("lifecycle_phase", &input.lifecycle_phase)?;
        validate_identifier("result", &input.result)?;
        validate_json_text("tasks.close_summary_json", &input.close_summary_json)?;
        validate_identifier("closed_at", &input.closed_at)?;

        let changed = self.tx.execute(
            "UPDATE tasks
                SET lifecycle_phase = ?3,
                    result = ?4,
                    close_summary_json = ?5,
                    closed_at = ?6,
                    updated_at = ?6
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                input.lifecycle_phase,
                input.result,
                input.close_summary_json,
                input.closed_at
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
            validate_json_text("tasks.close_summary_json", value)?;
            self.update_task_text_column(&input.task_id, "close_summary_json", value)?;
        }
        if let Some(value) = &input.completion_policy_json {
            validate_json_text("tasks.completion_policy_json", value)?;
            self.update_task_text_column(&input.task_id, "completion_policy_json", value)?;
        }
        if let Some(value) = &input.lifecycle_phase {
            validate_identifier("lifecycle_phase", value)?;
            self.update_task_text_column(&input.task_id, "lifecycle_phase", value)?;
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

    fn update_task_scope_revision(&mut self, input: &TaskScopeRevisionUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        let scope_revision = u64_to_i64("tasks.scope_revision", input.scope_revision)?;
        let changed = self.tx.execute(
            "UPDATE tasks
                SET scope_revision = ?3,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND task_id = ?2",
            params![self.project_id, input.task_id, scope_revision],
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
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                close_basis_revision,
                input.close_basis_json
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
                    closed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'
                AND is_current = 1",
            params![self.project_id, input.task_id],
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
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
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
                input.lifecycle_json
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
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND task_id = ?2",
            params![self.project_id, task_id, change_unit_id],
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

    fn mark_active_write_checks_stale(&mut self, task_id: &str) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        self.tx.execute(
            "UPDATE write_checks
                SET status = 'stale'
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'",
            params![self.project_id, task_id],
        )?;
        Ok(())
    }

    fn insert_write_check(
        &mut self,
        input: &WriteCheckInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("write_check_id", &input.write_check_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("change_unit_id", &input.change_unit_id)?;
        validate_json_text("write_checks.attempt_scope_json", &input.attempt_scope_json)?;
        validate_identifier("created_by_actor_source", &input.created_by_actor_source)?;
        if let Some(created_by_judgment_id) = &input.created_by_judgment_id {
            validate_identifier("created_by_judgment_id", created_by_judgment_id)?;
        }
        validate_identifier("expires_at", &input.expires_at)?;
        validate_identifier("created_at", &input.created_at)?;
        validate_json_text("write_checks.metadata_json", &input.metadata_json)?;
        let basis_state_version = u64_to_i64("basis_state_version", committed_state_version)?;

        self.tx.execute(
            "INSERT INTO write_checks (
                project_id,
                write_check_id,
                task_id,
                change_unit_id,
                basis_state_version,
                status,
                attempt_scope_json,
                created_by_actor_source,
                created_by_judgment_id,
                expires_at,
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
                NULL,
                NULL,
                NULL,
                ?10,
                ?11
            )",
            params![
                self.project_id,
                input.write_check_id,
                input.task_id,
                input.change_unit_id,
                basis_state_version,
                input.attempt_scope_json,
                input.created_by_actor_source,
                input.created_by_judgment_id,
                input.expires_at,
                input.created_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn consume_write_check(&mut self, input: &WriteCheckConsumption) -> StoreResult<()> {
        validate_identifier("write_check_id", &input.write_check_id)?;
        validate_identifier("run_id", &input.run_id)?;
        let expected_basis = u64_to_i64(
            "write_checks.basis_state_version",
            input.expected_basis_state_version,
        )?;
        let changed = self.tx.execute(
            "UPDATE write_checks
                SET status = 'consumed',
                    consumed_by_run_id = ?3,
                    consumed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND write_check_id = ?2
                AND status = 'active'
                AND basis_state_version = ?4",
            params![
                self.project_id,
                input.write_check_id,
                input.run_id,
                expected_basis
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "active Write Check consumption changed no rows".to_owned(),
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
        if let Some(write_check_id) = &input.write_check_id {
            validate_identifier("write_check_id", write_check_id)?;
        }
        validate_identifier("runs.kind", &input.kind)?;
        validate_identifier("runs.status", &input.status)?;
        validate_json_text("runs.summary_json", &input.summary_json)?;
        validate_json_text("runs.observed_changes_json", &input.observed_changes_json)?;
        validate_json_text("runs.evidence_updates_json", &input.evidence_updates_json)?;
        validate_json_text(
            "runs.write_check_effect_json",
            &input.write_check_effect_json,
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
                write_check_id,
                kind,
                status,
                summary_json,
                observed_changes_json,
                evidence_updates_json,
                write_check_effect_json,
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
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?14
            )",
            params![
                self.project_id,
                input.run_id,
                input.task_id,
                input.change_unit_id,
                scope_revision,
                input.write_check_id,
                input.kind,
                input.status,
                input.summary_json,
                input.observed_changes_json,
                input.evidence_updates_json,
                input.write_check_effect_json,
                input.created_by_actor_source,
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
            || staging.expires_at != input.expected_expires_at
        {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact changed before promotion".to_owned(),
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
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?14
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
                input.metadata_json
            ],
        )?;

        let changed = self.tx.execute(
            "UPDATE artifact_staging
                SET status = 'consumed',
                    consumed_by_run_id = ?3,
                    promoted_artifact_id = ?4,
                    consumed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND handle_id = ?2
                AND status = 'staged'",
            params![
                self.project_id,
                input.handle_id,
                input.run_id,
                input.artifact_id
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
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?7
            )",
            params![
                self.project_id,
                input.artifact_id,
                input.task_id,
                input.owner_record_kind,
                input.owner_record_id,
                input.created_by_run_id,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn upsert_evidence_summary(&mut self, input: &EvidenceSummaryUpsert) -> StoreResult<()> {
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

        self.tx.execute(
            "INSERT INTO evidence_summaries (
                project_id,
                evidence_summary_id,
                task_id,
                change_unit_id,
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
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?9
            )
            ON CONFLICT(project_id, evidence_summary_id) DO UPDATE SET
                task_id = excluded.task_id,
                change_unit_id = excluded.change_unit_id,
                status = excluded.status,
                coverage_json = excluded.coverage_json,
                supporting_refs_json = excluded.supporting_refs_json,
                gap_refs_json = excluded.gap_refs_json,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                metadata_json = excluded.metadata_json",
            params![
                self.project_id,
                input.evidence_summary_id,
                input.task_id,
                input.change_unit_id,
                input.status,
                input.coverage_json,
                input.supporting_refs_json,
                input.gap_refs_json,
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
        validate_identifier("evidence_observations.claim", &input.claim)?;
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
                claim,
                source_kind,
                assurance_level,
                observed_by_actor_source,
                tool_name,
                tool_invocation_id,
                tool_metadata_json,
                input_refs_json,
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
                ?18
            )",
            params![
                self.project_id,
                input.evidence_observation_id,
                input.task_id,
                input.change_unit_id,
                input.run_id,
                input.claim,
                input.source_kind,
                input.assurance_level,
                input.observed_by_actor_source,
                input.tool_name,
                input.tool_invocation_id,
                input.tool_metadata_json,
                input.input_refs_json,
                input.output_artifact_refs_json,
                input.limitations_json,
                input.observed_at,
                input.recorded_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn insert_user_judgment(&mut self, input: &UserJudgmentInsert) -> StoreResult<()> {
        validate_identifier("judgment_id", &input.judgment_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        validate_identifier("judgment_kind", &input.judgment_kind)?;
        validate_user_judgment_request_json("user_judgments.request_json", &input.request_json)?;
        validate_json_text("user_judgments.context_json", &input.context_json)?;
        validate_user_judgment_options_json("user_judgments.options_json", &input.options_json)?;
        validate_json_text(
            "user_judgments.affected_refs_json",
            &input.affected_refs_json,
        )?;
        validate_json_text(
            "user_judgments.artifact_refs_json",
            &input.artifact_refs_json,
        )?;
        validate_json_text(
            "user_judgments.sensitive_action_scope_json",
            &input.sensitive_action_scope_json,
        )?;
        validate_judgment_basis_json("user_judgments.basis_json", &input.basis_json)?;
        validate_identifier(
            "requested_by_actor_source",
            &input.requested_by_actor_source,
        )?;
        validate_identifier("requested_at", &input.requested_at)?;
        validate_json_text("user_judgments.metadata_json", &input.metadata_json)?;

        self.tx.execute(
            "INSERT INTO user_judgments (
                project_id,
                judgment_id,
                task_id,
                change_unit_id,
                judgment_kind,
                status,
                request_json,
                context_json,
                options_json,
                affected_refs_json,
                artifact_refs_json,
                sensitive_action_scope_json,
                basis_json,
                basis_status,
                resolution_outcome,
                resolution_machine_action,
                resolution_json,
                resolution_rationale_json,
                requested_by_actor_source,
                requested_at,
                resolved_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                'pending',
                ?6,
                ?7,
                ?8,
                ?9,
                ?10,
                ?11,
                ?12,
                ?13,
                NULL,
                NULL,
                NULL,
                NULL,
                ?14,
                ?15,
                NULL,
                ?16
            )",
            params![
                self.project_id,
                input.judgment_id,
                input.task_id,
                input.change_unit_id,
                input.judgment_kind,
                input.request_json,
                input.context_json,
                input.options_json,
                input.affected_refs_json,
                input.artifact_refs_json,
                input.sensitive_action_scope_json,
                input.basis_json,
                judgment_basis_status_as_str(input.basis_status),
                input.requested_by_actor_source,
                input.requested_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn resolve_user_judgment(&mut self, input: &UserJudgmentResolutionUpdate) -> StoreResult<()> {
        validate_identifier("judgment_id", &input.judgment_id)?;
        validate_identifier("status", &input.status)?;
        let resolution_outcome = judgment_resolution_outcome_as_str(input.resolution_outcome);
        let resolution_machine_action =
            judgment_machine_action_as_str(input.resolution_machine_action);
        if input.resolution_machine_action.resolution_outcome() != input.resolution_outcome {
            return Err(StoreError::InvalidInput {
                detail: "user_judgments.resolution_machine_action must match resolution_outcome"
                    .to_owned(),
            });
        }
        validate_user_judgment_resolution_json(
            "user_judgments.resolution_json",
            &input.resolution_json,
            input.resolution_machine_action,
            input.resolution_outcome,
        )?;
        validate_judgment_rationale_json(
            "user_judgments.resolution_rationale_json",
            &input.resolution_rationale_json,
        )?;
        if let Some(value) = &input.sensitive_action_scope_json {
            validate_json_text("user_judgments.sensitive_action_scope_json", value)?;
        }
        validate_identifier("resolved_by_actor_source", &input.resolved_by_actor_source)?;
        validate_identifier(
            "resolved_verification_basis",
            &input.resolved_verification_basis,
        )?;
        validate_identifier("resolved_assurance_level", &input.resolved_assurance_level)?;
        validate_identifier("resolved_at", &input.resolved_at)?;

        let changed = self.tx.execute(
            "UPDATE user_judgments
                SET status = ?3,
                    resolution_outcome = ?4,
                    resolution_machine_action = ?5,
                    resolution_json = ?6,
                    resolution_rationale_json = ?7,
                    sensitive_action_scope_json = COALESCE(?8, sensitive_action_scope_json),
                    resolved_by_actor_source = ?9,
                    resolved_verification_basis = ?10,
                    resolved_assurance_level = ?11,
                    resolved_at = ?12
              WHERE project_id = ?1
                AND judgment_id = ?2
                AND status = 'pending'",
            params![
                self.project_id,
                input.judgment_id,
                input.status,
                resolution_outcome,
                resolution_machine_action,
                input.resolution_json,
                input.resolution_rationale_json,
                input.sensitive_action_scope_json,
                input.resolved_by_actor_source,
                input.resolved_verification_basis,
                input.resolved_assurance_level,
                input.resolved_at
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "pending user judgment resolution changed no rows".to_owned(),
            })
        }
    }

    fn consume_local_web_consent_token(
        &mut self,
        input: &LocalWebConsentTokenConsumption,
    ) -> StoreResult<()> {
        validate_identifier("local_web_consent_tokens.token_hash", &input.token_hash)?;
        if input.token_hash.len() != 64
            || input
                .token_hash
                .chars()
                .any(|character| !character.is_ascii_hexdigit())
        {
            return Err(StoreError::InvalidInput {
                detail: "local_web_consent_tokens.token_hash must be 64 hex characters".to_owned(),
            });
        }
        validate_identifier("connection_internal_id", &input.connection_internal_id)?;
        validate_identifier("judgment_id", &input.judgment_id)?;
        validate_identifier("consumed_at", &input.consumed_at)?;
        validate_json_text(
            "local_web_consent_tokens.completion_metadata_json",
            &input.completion_metadata_json,
        )?;

        let changed = self.tx.execute(
            "UPDATE local_web_consent_tokens
                SET status = 'consumed',
                    consumed_at = ?5,
                    completed_at = ?5,
                    completion_metadata_json = ?6
              WHERE project_id = ?1
                AND token_hash = ?2
                AND connection_internal_id = ?3
                AND judgment_id = ?4
                AND status = 'pending'
                AND expires_at > ?5",
            params![
                self.project_id,
                input.token_hash,
                input.connection_internal_id,
                input.judgment_id,
                input.consumed_at,
                input.completion_metadata_json
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::Conflict {
                entity: "local_web_consent_token",
                id: input.token_hash.clone(),
                detail: "token is not pending, is expired, or is not bound to this judgment"
                    .to_owned(),
            })
        }
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

    fn update_user_judgment_basis(&mut self, input: &UserJudgmentBasisUpdate) -> StoreResult<()> {
        validate_identifier("judgment_id", &input.judgment_id)?;
        validate_judgment_basis_json("user_judgments.basis_json", &input.basis_json)?;
        let changed = self.tx.execute(
            "UPDATE user_judgments
                SET basis_json = ?3,
                    basis_status = ?4
              WHERE project_id = ?1
                AND judgment_id = ?2",
            params![
                self.project_id,
                input.judgment_id,
                input.basis_json,
                judgment_basis_status_as_str(input.basis_status)
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "user judgment basis update changed no rows".to_owned(),
            })
        }
    }

    fn mark_user_judgment_bases_status(
        &mut self,
        input: &UserJudgmentBasisStatusMark,
    ) -> StoreResult<()> {
        let status = match input.basis_status {
            JudgmentBasisCompatibilityStatus::Stale
            | JudgmentBasisCompatibilityStatus::Superseded => {
                judgment_basis_status_as_str(input.basis_status)
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: "selected judgment bases may only be marked stale or superseded"
                        .to_owned(),
                })
            }
        };

        for judgment_id in &input.judgment_ids {
            validate_identifier("judgment_id", judgment_id)?;
            let changed = self.tx.execute(
                "UPDATE user_judgments
                    SET basis_status = ?3
                  WHERE project_id = ?1
                    AND judgment_id = ?2",
                params![self.project_id, judgment_id, status],
            )?;
            if changed != 1 {
                return Err(StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: format!(
                        "selected user judgment basis status update changed {changed} rows"
                    ),
                });
            }
        }

        Ok(())
    }

    fn mark_user_judgments_superseded_or_stale(
        &mut self,
        input: &UserJudgmentInvalidation,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        if input.judgment_kinds.is_empty() {
            self.mark_user_judgments_superseded_or_stale_for_kind(&input.task_id, None)?;
        } else {
            for judgment_kind in &input.judgment_kinds {
                validate_identifier("judgment_kind", judgment_kind)?;
                self.mark_user_judgments_superseded_or_stale_for_kind(
                    &input.task_id,
                    Some(judgment_kind),
                )?;
            }
        }
        Ok(())
    }

    fn mark_user_judgments_superseded_or_stale_for_kind(
        &mut self,
        task_id: &str,
        judgment_kind: Option<&str>,
    ) -> StoreResult<()> {
        match judgment_kind {
            Some(judgment_kind) => {
                self.tx.execute(
                    "UPDATE user_judgments
                        SET status = 'superseded',
                            basis_status = 'superseded'
                      WHERE project_id = ?1
                        AND task_id = ?2
                        AND judgment_kind = ?3
                        AND status = 'pending'
                        AND basis_status = 'current'",
                    params![self.project_id, task_id, judgment_kind],
                )?;
                self.tx.execute(
                    "UPDATE user_judgments
                        SET status = 'stale',
                            basis_status = 'stale'
                      WHERE project_id = ?1
                        AND task_id = ?2
                        AND judgment_kind = ?3
                        AND status = 'resolved'
                        AND basis_status = 'current'",
                    params![self.project_id, task_id, judgment_kind],
                )?;
            }
            None => {
                self.tx.execute(
                    "UPDATE user_judgments
                        SET status = 'superseded',
                            basis_status = 'superseded'
                      WHERE project_id = ?1
                        AND task_id = ?2
                        AND status = 'pending'
                        AND basis_status = 'current'",
                    params![self.project_id, task_id],
                )?;
                self.tx.execute(
                    "UPDATE user_judgments
                        SET status = 'stale',
                            basis_status = 'stale'
                      WHERE project_id = ?1
                        AND task_id = ?2
                        AND status = 'resolved'
                        AND basis_status = 'current'",
                    params![self.project_id, task_id],
                )?;
            }
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
                "UPDATE tasks SET shaping_summary_json = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "bounded_context_json" => {
                "UPDATE tasks SET bounded_context_json = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "autonomy_boundary_json" => {
                "UPDATE tasks SET autonomy_boundary_json = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "close_summary_json" => {
                "UPDATE tasks SET close_summary_json = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "completion_policy_json" => {
                "UPDATE tasks SET completion_policy_json = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "lifecycle_phase" => {
                "UPDATE tasks SET lifecycle_phase = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "result" => {
                "UPDATE tasks SET result = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: format!("unsupported Task text column {column}"),
                })
            }
        };
        let changed = self
            .tx
            .execute(sql, params![self.project_id, task_id, value])?;
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
                "UPDATE tasks SET title = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "summary" => {
                "UPDATE tasks SET summary = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: format!("unsupported nullable Task column {column}"),
                })
            }
        };
        let changed = self
            .tx
            .execute(sql, params![self.project_id, task_id, value])?;
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
