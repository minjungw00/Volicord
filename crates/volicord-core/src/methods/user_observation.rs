use super::*;

impl CoreService {
    /// Executes the User Channel-only `volicord.record_user_observation` transition.
    pub fn record_user_observation(
        &self,
        request: RecordUserObservationRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if request.envelope.task_id.as_ref() != Some(&request.task_id) {
            return validation_rejected(
                request.envelope.dry_run,
                None,
                "task_id",
                "envelope.task_id must match RecordUserObservationRequest.task_id",
            );
        }
        let prepared = match prepare_or_response(
            self,
            MethodName::RecordUserObservation,
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
        let plan = match plan_record_user_observation(
            self,
            &prepared.store,
            &prepared.context.project_state,
            &prepared.context.verified_invocation,
            request.clone(),
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
                        "user_evidence_observation",
                        "record",
                        "Request would record one target-bound User Channel evidence observation.",
                        Vec::new(),
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "user_evidence_observation_recorded".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }
}

fn plan_record_user_observation(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    mut request: RecordUserObservationRequest,
) -> Result<MethodPlan, PlanError> {
    if verified_invocation.actor_source != ActorSource::LocalUser {
        return user_observation_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "actor_source",
            "record_user_observation requires a verified local User Channel",
        );
    }
    request.summary = normalize_display_text(&request.summary);
    if request.summary.is_empty() {
        return user_observation_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "summary",
            "summary must not be empty",
        );
    }
    if !matches!(
        request.relevance_status,
        EvidenceRelevanceStatus::Supported | EvidenceRelevanceStatus::Contradicted
    ) {
        return user_observation_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "relevance_status",
            "User Channel relevance must be supported or contradicted",
        );
    }
    if request.observed_at > utc_timestamp(service.now()) {
        return user_observation_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_at",
            "observed_at must not be in the future",
        );
    }

    let task = store
        .task_record(&request.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "task_id does not identify an existing Task",
            )))
        })?;
    if task.current_change_unit_id.as_deref() != Some(request.change_unit_id.as_str()) {
        return user_observation_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "change_unit_id",
            "change_unit_id must identify the Task's current Change Unit",
        );
    }
    let change_unit = store
        .change_unit_record(&request.task_id, request.change_unit_id.as_str())
        .map_err(CorePipelineError::from)?;
    if change_unit
        .as_ref()
        .is_none_or(|record| record.status != "active")
    {
        return user_observation_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "change_unit_id",
            "change_unit_id must identify an active Change Unit",
        );
    }
    let Some(baseline_ref) = StoredScope::from_task(&task)?
        .baseline_ref
        .map(BaselineRef::new)
    else {
        return user_observation_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "baseline_ref",
            "a current Task baseline is required before recording user evidence",
        );
    };

    validate_user_observation_target(store, project_state, &request)?;
    let output_artifact_refs =
        canonical_user_observation_artifacts(store, project_state, &request)?;
    let observation_id =
        allocate_evidence_observation_id(service, store).map_err(PlanError::Core)?;
    let planned_state_version = project_state.state_version + 1;
    let observation_ref = state_ref(
        StateRecordKind::UserEvidenceObservation,
        observation_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let recorded_at = utc_timestamp(service.now());
    let observation = UserEvidenceObservation {
        observation_id: observation_id.clone(),
        project_id: request.envelope.project_id.clone(),
        task_id: request.task_id.clone(),
        change_unit_id: request.change_unit_id.clone(),
        scope_revision: task.scope_revision,
        baseline_ref: baseline_ref.clone(),
        target: request.target.clone(),
        relevance_status: request.relevance_status,
        output_artifact_refs: output_artifact_refs.clone(),
        summary: request.summary.clone(),
        observed_by_actor_source: ActorSource::LocalUser,
        verification_basis: verified_invocation.verification_basis.clone(),
        observed_at: request.observed_at.clone(),
        recorded_at: recorded_at.clone(),
    };
    let result = RecordUserObservationResult {
        base: placeholder_base(),
        user_observation_ref: observation_ref.clone(),
        user_observation: observation,
    };
    let mutation =
        CoreStorageMutation::InsertUserEvidenceObservation(UserEvidenceObservationInsert {
            user_evidence_observation_id: observation_id.as_str().to_owned(),
            task_id: request.task_id.as_str().to_owned(),
            change_unit_id: request.change_unit_id.as_str().to_owned(),
            scope_revision: task.scope_revision,
            baseline_ref: baseline_ref.as_str().to_owned(),
            acceptance_criterion_id: match &request.target {
                EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id,
                } => Some(acceptance_criterion_id.as_str().to_owned()),
                EvidenceTarget::SupplementalClaim { .. } => None,
            },
            evidence_claim_id: match &request.target {
                EvidenceTarget::SupplementalClaim {
                    evidence_claim_id, ..
                } => Some(evidence_claim_id.as_str().to_owned()),
                EvidenceTarget::AcceptanceCriterion { .. } => None,
            },
            relevance_status: storage_value(request.relevance_status)?,
            output_artifact_refs_json: serde_json::to_string(&output_artifact_refs)?,
            summary: request.summary,
            observed_by_actor_source: ActorSource::LocalUser.to_canonical_string(),
            verification_basis: verified_invocation.verification_basis.clone(),
            observed_at: request.observed_at.to_canonical_string(),
            recorded_at: recorded_at.to_canonical_string(),
        });
    Ok(MethodPlan {
        task_id: request.task_id,
        change_unit_id: Some(request.change_unit_id),
        storage_mutations: vec![mutation],
        event_payload: object_from_value(json!({
            "user_evidence_observation_id": observation_id,
            "target": request.target,
            "relevance_status": request.relevance_status,
            "output_artifact_refs": output_artifact_refs,
            "verification_basis": verified_invocation.verification_basis,
        }))?,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions: Vec::new(),
    })
}

fn validate_user_observation_target(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordUserObservationRequest,
) -> Result<(), PlanError> {
    let current = match &request.target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => store
            .acceptance_criterion_record(acceptance_criterion_id.as_str())
            .map_err(CorePipelineError::from)?
            .is_some_and(|record| {
                record.task_id == request.task_id.as_str() && record.status == "active"
            }),
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id,
            statement,
        } => store
            .evidence_claim_record(&request.task_id, evidence_claim_id.as_str())
            .map_err(CorePipelineError::from)?
            .is_some_and(|record| record.statement == normalize_display_text(statement)),
    };
    if current {
        Ok(())
    } else {
        user_observation_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "target",
            "target must identify a current acceptance criterion or existing supplemental claim",
        )
    }
}

fn canonical_user_observation_artifacts(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordUserObservationRequest,
) -> Result<Vec<ArtifactRef>, PlanError> {
    if request.artifact_ids.is_empty() {
        return user_observation_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "output_artifact_refs",
            "a user evidence observation must bind at least one artifact",
        );
    }
    let mut canonical = BTreeMap::new();
    for artifact_id in &request.artifact_ids {
        let record = store
            .artifact_record(artifact_id.as_str())
            .map_err(CorePipelineError::from)?;
        let owner_link = store
            .artifact_has_task_owner_link(artifact_id.as_str(), request.task_id.as_str())
            .map_err(CorePipelineError::from)?;
        let Some(record) = record else {
            return user_observation_validation_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "output_artifact_refs",
                "artifact refs must identify current persistent Task artifacts",
            );
        };
        if record.project_id != request.envelope.project_id.as_str()
            || record.task_id != request.task_id.as_str()
            || !owner_link
            || !persistent_artifact_is_verified_current(store, &record)?
        {
            return user_observation_validation_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "output_artifact_refs",
                "artifact refs must identify verified current artifacts owned by this Task",
            );
        }
        let canonical_ref = artifact_ref_from_verified_record(
            store,
            &record,
            None,
            Some(project_state.state_version),
        )?;
        canonical.insert(canonical_ref.artifact_id.as_str().to_owned(), canonical_ref);
    }
    Ok(canonical.into_values().collect())
}

fn user_observation_validation_error<T>(
    dry_run: bool,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    match validation_plan_error(dry_run, state_version, field, message) {
        Err(error) => Err(error),
        Ok(()) => unreachable!("validation_plan_error always returns Err"),
    }
}
