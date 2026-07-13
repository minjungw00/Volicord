use super::*;

const MAX_CAPTURE_LABEL_BYTES: usize = 256;
const MAX_CAPTURE_TOOL_NAME_BYTES: usize = 256;

impl CoreService {
    /// Executes `volicord.prepare_evidence_capture` through the shared Core mutation pipeline.
    pub fn prepare_evidence_capture(
        &self,
        request: PrepareEvidenceCaptureRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = request.envelope.task_id.as_ref() {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match PrepareEvidenceCaptureRequest.task_id",
                );
            }
        }
        let prepared = match prepare_or_response(
            self,
            MethodName::PrepareEvidenceCapture,
            request.envelope.clone(),
            request_json,
            invocation,
            mutation_method_policy(
                request.operation_category(),
                TaskRequirement::Exact(request.task_id.clone()),
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_prepare_evidence_capture(
            self,
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
            &prepared.context.verified_invocation,
            &prepared.operation_now,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                return plan_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    error,
                )
            }
        };

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: dry_run_summary(
                        "evidence_capture_intent",
                        "would_create",
                        "Evidence capture preparation would create one expiring intent and no receipt or producer.",
                        Vec::new(),
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "evidence_capture_prepared".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }
}

fn plan_prepare_evidence_capture(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    mut request: PrepareEvidenceCaptureRequest,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<MethodPlan, PlanError> {
    let connection_id = verified_invocation
        .actor_source
        .agent_connection_id()
        .cloned()
        .ok_or_else(|| {
            PlanError::Response(Box::new(
                rejected_pipeline_response(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    vec![tool_error(
                        ErrorCode::InvocationContextMismatch,
                        "evidence capture intent creation requires a verified Agent Connection",
                        false,
                        None,
                    )],
                )
                .expect("fixed evidence-capture rejection should serialize"),
            ))
        })?;
    let workspace = verified_invocation
        .git_workspace_context
        .as_ref()
        .ok_or_else(|| {
            PlanError::Response(Box::new(
                rejected_pipeline_response(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    vec![tool_error(
                        ErrorCode::InvocationContextMismatch,
                        "evidence capture intent creation requires verified Git workspace context",
                        false,
                        None,
                    )],
                )
                .expect("fixed evidence-capture rejection should serialize"),
            ))
        })?;

    let task = store
        .task_record(&request.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    if task.current_change_unit_id.as_deref() != Some(request.change_unit_id.as_str()) {
        return Err(PlanError::Response(Box::new(
            no_active_change_unit_response(
                &request.envelope,
                Some(project_state.state_version),
                "change_unit_id must identify the current Change Unit",
            ),
        )));
    }
    let change_unit = store
        .change_unit_record(&request.task_id, request.change_unit_id.as_str())
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_change_unit_response(
                &request.envelope,
                Some(project_state.state_version),
                "change_unit_id does not identify a Change Unit for this Task",
            )))
        })?;
    if change_unit.status != "active" || !change_unit.is_current {
        return Err(PlanError::Response(Box::new(
            no_active_change_unit_response(
                &request.envelope,
                Some(project_state.state_version),
                "evidence capture preparation requires the current active Change Unit",
            ),
        )));
    }
    if !baseline_matches(&change_unit, &task, &request.baseline_ref)? {
        return Err(PlanError::Response(Box::new(baseline_stale_response(
            &request.envelope,
            Some(project_state.state_version),
            &request.baseline_ref,
        ))));
    }
    if !workspace_context_matches(&change_unit, verified_invocation)? {
        return Err(PlanError::Response(Box::new(workspace_stale_response(
            &request.envelope,
            Some(project_state.state_version),
        ))));
    }

    if let EvidenceTarget::SupplementalClaim { statement, .. } = &mut request.target {
        *statement = normalize_display_text(statement);
    }
    validate_capture_target(store, project_state, &request)?;
    let normalized = normalize_capture_spec(project_state, &request)?;
    request.capture = normalized.capture;
    if !matches!(
        request.capture,
        EvidenceCaptureSpec::VerifiedCommandExecution { .. }
    ) && verified_invocation.session_id.is_none()
    {
        return Err(PlanError::Response(Box::new(
            rejected_pipeline_response(
                request.envelope.dry_run,
                Some(project_state.state_version),
                vec![tool_error(
                    ErrorCode::InvocationContextMismatch,
                    "tool and registered-connection evidence capture intents require an exact verified Agent Session",
                    false,
                    None,
                )],
            )
            .expect("fixed evidence-capture session rejection should serialize"),
        )));
    }

    let capture_intent_id =
        allocate_evidence_capture_intent_id(service, store).map_err(PlanError::Core)?;
    let created_at = operation_now.clone();
    let expires_at = checked_derived_expiration(
        &created_at,
        Duration::minutes(EVIDENCE_CAPTURE_INTENT_TTL_MINUTES),
        request.envelope.dry_run,
        Some(project_state.state_version),
        "expires_at",
    )?;
    let planned_state_version = project_state.state_version + 1;
    let capture_intent_ref = state_ref(
        StateRecordKind::EvidenceCaptureIntent,
        capture_intent_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let workspace_context = object_from_value(serde_json::to_value(workspace)?)?;
    let capture_intent = EvidenceCaptureIntent {
        capture_intent_id: capture_intent_id.clone(),
        project_id: request.envelope.project_id.clone(),
        task_id: request.task_id.clone(),
        change_unit_id: request.change_unit_id.clone(),
        scope_revision: task.scope_revision,
        baseline_ref: request.baseline_ref.clone(),
        target: request.target.clone(),
        capture: request.capture.clone(),
        input_sha256: normalized.input_sha256.clone(),
        expected_outcome: normalized.expected_outcome.clone(),
        requested_by_actor_source: verified_invocation.actor_source.clone(),
        workspace_context: workspace_context.clone(),
        created_at: created_at.clone(),
        expires_at: expires_at.clone(),
    };
    let result = PrepareEvidenceCaptureResult {
        base: placeholder_base(),
        capture_intent_ref: capture_intent_ref.clone(),
        capture_intent: capture_intent.clone(),
        expires_at: expires_at.clone(),
    };
    let capture_kind = storage_value(normalized.producer_kind)?;
    let storage_mutations = vec![CoreStorageMutation::InsertEvidenceCaptureIntent(
        EvidenceCaptureIntentInsert {
            evidence_capture_intent_id: capture_intent_id.as_str().to_owned(),
            task_id: request.task_id.as_str().to_owned(),
            change_unit_id: request.change_unit_id.as_str().to_owned(),
            scope_revision: task.scope_revision,
            baseline_ref: request.baseline_ref.as_str().to_owned(),
            target_json: serde_json::to_string(&request.target)?,
            capture_kind: capture_kind.clone(),
            capture_spec_json: serde_json::to_string(&request.capture)?,
            input_sha256: normalized.input_sha256.clone(),
            expected_outcome_json: serde_json::to_string(&normalized.expected_outcome)?,
            requested_by_actor_source: verified_invocation.actor_source.to_canonical_string(),
            requesting_connection_internal_id: connection_id.as_str().to_owned(),
            session_context_json: serde_json::to_string(&json!({
                "session_id": verified_invocation.session_id
            }))?,
            workspace_context_json: serde_json::to_string(&workspace_context)?,
            created_at: created_at.to_canonical_string(),
            expires_at: expires_at.to_canonical_string(),
            metadata_json: serde_json::to_string(&json!({
                "verification_basis": verified_invocation.verification_basis
            }))?,
        },
    )];
    let event_payload = object_from_value(json!({
        "capture_intent_ref": capture_intent_ref,
        "capture_kind": capture_kind,
        "task_id": request.task_id,
        "change_unit_id": request.change_unit_id,
        "scope_revision": task.scope_revision,
        "baseline_ref": request.baseline_ref,
        "target": request.target,
        "input_sha256": normalized.input_sha256,
        "expires_at": expires_at
    }))?;

    Ok(MethodPlan {
        task_id: capture_intent.task_id,
        change_unit_id: Some(capture_intent.change_unit_id),
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions: Vec::new(),
    })
}

struct NormalizedCaptureSpec {
    capture: EvidenceCaptureSpec,
    producer_kind: EvidenceProducerKind,
    input_sha256: String,
    expected_outcome: JsonObject,
}

fn normalize_capture_spec(
    project_state: &ProjectStateHeader,
    request: &PrepareEvidenceCaptureRequest,
) -> Result<NormalizedCaptureSpec, PlanError> {
    match &request.capture {
        EvidenceCaptureSpec::VerifiedCommandExecution {
            command_sha256,
            command_label,
            expected_exit_code,
        } => {
            if !artifact_sha256_is_lowercase_hex(command_sha256) {
                return invalid_capture_spec(
                    request,
                    project_state,
                    "capture.command_sha256",
                    "command_sha256 must be a lowercase 64-character SHA-256",
                );
            }
            let command_label = normalize_display_text(command_label);
            if command_label.is_empty() || command_label.len() > MAX_CAPTURE_LABEL_BYTES {
                return invalid_capture_spec(
                    request,
                    project_state,
                    "capture.command_label",
                    "command_label must be non-empty and at most 256 UTF-8 bytes",
                );
            }
            let expected_exit_code = expected_exit_code.clone().into_option().unwrap_or(0);
            let capture = EvidenceCaptureSpec::VerifiedCommandExecution {
                command_sha256: command_sha256.clone(),
                command_label,
                expected_exit_code: Some(expected_exit_code).into(),
            };
            Ok(NormalizedCaptureSpec {
                expected_outcome: evidence_capture_expected_outcome(&capture),
                capture,
                producer_kind: EvidenceProducerKind::VerifiedCommandExecution,
                input_sha256: command_sha256.clone(),
            })
        }
        EvidenceCaptureSpec::VerifiedToolInvocation {
            tool_name,
            tool_input_sha256,
            expected_success,
        } => {
            if !artifact_sha256_is_lowercase_hex(tool_input_sha256) {
                return invalid_capture_spec(
                    request,
                    project_state,
                    "capture.tool_input_sha256",
                    "tool_input_sha256 must be a lowercase 64-character SHA-256",
                );
            }
            let tool_name = tool_name.trim().to_owned();
            if tool_name.is_empty() || tool_name.len() > MAX_CAPTURE_TOOL_NAME_BYTES {
                return invalid_capture_spec(
                    request,
                    project_state,
                    "capture.tool_name",
                    "tool_name must be non-empty and at most 256 UTF-8 bytes",
                );
            }
            let expected_success = expected_success.clone().into_option().unwrap_or(true);
            let capture = EvidenceCaptureSpec::VerifiedToolInvocation {
                tool_name,
                tool_input_sha256: tool_input_sha256.clone(),
                expected_success: Some(expected_success).into(),
            };
            Ok(NormalizedCaptureSpec {
                expected_outcome: evidence_capture_expected_outcome(&capture),
                capture,
                producer_kind: EvidenceProducerKind::VerifiedToolInvocation,
                input_sha256: tool_input_sha256.clone(),
            })
        }
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_selector,
            expected_complete,
        } => {
            let expected_complete = expected_complete.clone().into_option().unwrap_or(true);
            let capture = EvidenceCaptureSpec::RegisteredConnectionObservation {
                source_selector: *source_selector,
                expected_complete: Some(expected_complete).into(),
            };
            let input_sha256 = evidence_capture_input_sha256(&capture)?;
            Ok(NormalizedCaptureSpec {
                expected_outcome: evidence_capture_expected_outcome(&capture),
                capture,
                producer_kind: EvidenceProducerKind::RegisteredConnectionObservation,
                input_sha256,
            })
        }
    }
}

fn invalid_capture_spec<T>(
    request: &PrepareEvidenceCaptureRequest,
    project_state: &ProjectStateHeader,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    validation_plan_error(
        request.envelope.dry_run,
        Some(project_state.state_version),
        field,
        message,
    )?;
    unreachable!("validation_plan_error always returns Err")
}

fn validate_capture_target(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &PrepareEvidenceCaptureRequest,
) -> Result<(), PlanError> {
    match &request.target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => {
            let record = store
                .acceptance_criterion_record(acceptance_criterion_id.as_str())
                .map_err(CorePipelineError::from)?;
            if record.as_ref().is_none_or(|record| {
                record.task_id != request.task_id.as_str() || record.status != "active"
            }) {
                return validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "target",
                    "acceptance criterion target must be current for this Task",
                );
            }
        }
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id,
            statement,
        } => {
            if evidence_claim_id.as_str().trim().is_empty() || statement.trim().is_empty() {
                return validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "target",
                    "supplemental claim target requires a non-empty ID and statement",
                );
            }
            if let Some(record) = store
                .evidence_claim_record(&request.task_id, evidence_claim_id.as_str())
                .map_err(CorePipelineError::from)?
            {
                if record.statement != *statement {
                    return validation_plan_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "target.statement",
                        "supplemental claim statement is immutable within a Task",
                    );
                }
            }
        }
    }
    Ok(())
}
