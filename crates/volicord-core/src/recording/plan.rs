use crate::acceptance_facts::active_acceptance_criteria;
use crate::close_readiness::{
    build_record_run_close_basis, RecordRunCloseBasisContext, RecordRunCloseBasisError,
};
use crate::evidence_facts::load_current_evidence_summary_facts;
use crate::evidence_projection::evidence_summary_for_display;
use crate::identity::{allocate_evidence_summary_id, allocate_run_id};
use crate::json_object::object_from_value;
use crate::pipeline::{CoreService, VerifiedInvocationContext};
use crate::policy::close_readiness_evidence::{
    evidence_summary_with_required_criteria, project_close_evidence_summary,
    required_acceptance_criterion_ids,
};
use crate::record_refs::{change_unit_ref, state_ref, state_ref_from_stored};
use crate::task_facts::active_blocker_refs;
use crate::task_policy::{plan_user_action_lifecycle_transition, TaskLifecycleFacts};
use crate::write_ticket::{admit_record_run, RecordRunWriteAdmission, WriteTicketAdmissionError};
use serde_json::json;
use volicord_store::core_pipeline::{
    ArtifactLinkInsert, ArtifactMutation, CoreProjectStore, EvidenceMutation,
    EvidenceSummaryUpsert, ProjectStateHeader, RunInsert, RunMutation, RunStatus,
    StoredRunMetadata, StoredRunSummary, StoredRunWriteTicketEffect,
    StoredRunWriteTicketEffectKind, TaskCloseBasisUpdate, TaskControlLevelUpdate, TaskMutation,
    UserActionInvalidation, UserActionMutation, WriteTicketConsumption, WriteTicketMutation,
};
use volicord_types::schema::{PersistedEvidenceMetadata, StateRecordRef, StateSummary};
use volicord_types::values::{
    AcceptancePolicy, StateRecordKind, TaskControlLevel, UserActionKind, UserActionRequiredFor,
    UtcTimestamp,
};
use volicord_user_action_service::{
    pending_user_action_authorities, pending_user_action_refs_for_operation,
    projected_user_action_lifecycle_phase, SensitiveApprovalRequirement, UserActionOperation,
    UserActionOperationContext,
};

use crate::recording::{
    recording_store_error, recording_user_action_error, recording_validation_error,
    RecordRunEffect, RecordRunInput, RecordRunOperationPlan, RecordRunResultFacts, RecordingError,
    RecordingRejection,
};

use super::{
    artifact::plan_record_run_artifacts,
    authority::plan_record_run_capture_authorities,
    context::{acquire_record_run_facts, normalize_record_run_request},
    evidence::{
        build_record_run_evidence_summary, observation_refs_by_target,
        plan_record_run_evidence_targets, plan_record_run_observations,
        RecordRunObservationContext,
    },
    model::{
        RecordRunArtifactContext, RecordRunEvidenceTargetPlan, RecordRunFacts, RecordRunMutation,
        RecordRunMutationAssembly, RecordRunMutationPlan, RecordRunNormalizedRequest,
        RecordRunPlannedMutations, RecordRunPolicyDecision, RecordRunRawRequest,
    },
    state::acquire_record_run_state,
};

pub(crate) fn plan_record_run(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: RecordRunInput,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<RecordRunOperationPlan, RecordingError> {
    let raw = RecordRunRawRequest::new(request, operation_now);
    let normalized = normalize_record_run_request(store, project_state, raw)?;
    let facts = acquire_record_run_facts(store, normalized, verified_invocation)?;
    let evidence_target_plan =
        plan_record_run_evidence_targets(store, &facts.normalized.raw.request)?;
    let policy =
        decide_record_run_policy(service, store, project_state, verified_invocation, facts)?;
    let mutations = plan_record_run_mutations(
        service,
        store,
        project_state,
        verified_invocation,
        policy,
        evidence_target_plan,
    )?;
    let state = acquire_record_run_state(store, project_state, verified_invocation, &mutations)?;
    Ok(record_run_operation_plan(mutations, state))
}

fn record_run_operation_plan(
    planned: RecordRunPlannedMutations,
    state: StateSummary,
) -> RecordRunOperationPlan {
    let RecordRunPlannedMutations {
        request,
        run_ref,
        normalized_observed_changes,
        registered_artifacts,
        evidence_observations,
        evidence_producers,
        recorded_evidence_summary,
        current_close_basis,
        blocker_refs,
        mutation_plan,
        event_payload,
        ..
    } = planned;
    RecordRunOperationPlan {
        effect: RecordRunEffect {
            task_id: request.task_id,
            change_unit_id: request.change_unit_id,
            mutation_plan,
            event_payload,
        },
        result_facts: RecordRunResultFacts {
            run_ref,
            kind: request.kind,
            summary: request.summary,
            observed_changes: normalized_observed_changes,
            registered_artifacts,
            evidence_summary: recorded_evidence_summary,
            evidence_observations,
            evidence_producers,
            current_close_basis,
            blocker_refs,
            state,
        },
    }
}

pub(super) fn decide_record_run_policy(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    facts: RecordRunFacts,
) -> Result<RecordRunPolicyDecision, RecordingError> {
    let RecordRunFacts {
        normalized,
        task,
        change_unit,
        workflow_policy,
        resolved_control,
    } = facts;
    let request = &normalized.raw.request;
    let planned_state_version = normalized.planned_state_version;
    let plan_now = &normalized.raw.plan_now;
    let normalized_changed_paths = &normalized.normalized_changed_paths;
    let normalized_observed_changes = &normalized.normalized_observed_changes;
    let task_control = resolved_control.effective_control_level;
    let write_ticket_required = request.observed_changes.product_file_write_observed
        || task_control == TaskControlLevel::Sensitive;
    let write_ticket_scope = if write_ticket_required {
        let Some(write_ticket_id) = request.write_ticket_id.as_ref() else {
            return Err(RecordingError::Rejected(
                RecordingRejection::WriteTicketRequired,
            ));
        };
        if resolved_control.pending_policy_reevaluation {
            return Err(write_ticket_invalid(
                "approval_basis_changed",
                "a pending project-policy control reevaluation requires prepare_write before this Run can consume a ticket",
            ));
        }
        let record = store
            .write_ticket_record(write_ticket_id.as_str())
            .map_err(recording_store_error)?
            .ok_or_else(|| {
                write_ticket_invalid(
                    "missing",
                    "write_ticket_id does not identify a write ticket",
                )
            })?;
        let scope = admit_record_run(
            &record,
            RecordRunWriteAdmission {
                store,
                project_id: &request.project_id,
                task_id: &request.task_id,
                change_unit_id: &request.change_unit_id,
                baseline_ref: &request.baseline_ref,
                performed_operation: request.performed_operation.as_deref(),
                task: &task,
                change_unit: &change_unit,
                verified_invocation,
                observed_changes: normalized_observed_changes,
                write_authority_fingerprint: &workflow_policy.write_authority_fingerprint,
                now: *plan_now.as_datetime(),
            },
        )
        .map_err(record_run_write_admission_error)?;
        Some((record, scope))
    } else {
        if request.write_ticket_id.is_some() {
            return Err(write_ticket_invalid(
                "incompatible",
                "write_ticket_id is only consumed for an observed product-file write or an effective sensitive Task",
            ));
        }
        None
    };

    let operation_refs = vec![
        state_ref(
            StateRecordKind::Task,
            request.task_id.as_str(),
            &request.project_id,
            Some(&request.task_id),
            Some(project_state.state_version),
        ),
        change_unit_ref(
            &request.project_id,
            &request.task_id,
            &change_unit,
            project_state.state_version,
        ),
    ];
    let sensitive_approval = write_ticket_scope
        .as_ref()
        .filter(|_| {
            task_control == TaskControlLevel::Sensitive
                || !normalized_observed_changes.sensitive_categories.is_empty()
        })
        .map(|(_, scope)| SensitiveApprovalRequirement {
            task_id: &request.task_id,
            change_unit_id: &request.change_unit_id,
            scope_revision: task.scope_revision,
            operation: &scope.intended_operation,
            normalized_paths: normalized_changed_paths,
            sensitive_categories: &normalized_observed_changes.sensitive_categories,
            baseline_ref: Some(&request.baseline_ref),
            required_for: UserActionRequiredFor::RecordRun,
            now: plan_now,
        });
    let operation_context = UserActionOperationContext {
        operation: UserActionOperation::RecordRun,
        task_id: &request.task_id,
        change_unit_id: Some(&request.change_unit_id),
        scope_revision: task.scope_revision,
        close_basis: None,
        operation_refs: &operation_refs,
        sensitive_approval: sensitive_approval.as_ref(),
    };
    if !pending_user_action_refs_for_operation(
        store,
        &request.project_id,
        project_state.state_version,
        plan_now,
        &operation_context,
    )
    .map_err(recording_user_action_error)?
    .is_empty()
    {
        return Err(RecordingError::Rejected(
            RecordingRejection::DecisionRejected {
                message:
                    "a current pending user action must be resolved before this Run can be recorded",
            },
        ));
    }

    let run_id = match request.run_id.clone() {
        Some(run_id) => run_id,
        None => {
            allocate_run_id(service.durable_id_generator(), store).map_err(RecordingError::Core)?
        }
    };
    if request.run_id.is_some()
        && store
            .run_id_exists(run_id.as_str())
            .map_err(recording_store_error)?
    {
        return recording_validation_error("run_id", "run_id already identifies an existing Run");
    }
    let run_ref = state_ref(
        StateRecordKind::Run,
        run_id.as_str(),
        &request.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );

    Ok(RecordRunPolicyDecision {
        facts: RecordRunFacts {
            normalized,
            task,
            change_unit,
            workflow_policy,
            resolved_control,
        },
        write_ticket_scope,
        run_id,
        run_ref,
    })
}

fn record_run_write_admission_error(error: WriteTicketAdmissionError) -> RecordingError {
    match error {
        WriteTicketAdmissionError::Core(error) => RecordingError::Core(error),
        WriteTicketAdmissionError::UserAction(error) => RecordingError::UserAction(error),
        WriteTicketAdmissionError::Invalid { reason, message } => {
            write_ticket_invalid(reason, message)
        }
    }
}

fn record_run_close_basis_error(error: RecordRunCloseBasisError) -> RecordingError {
    match error {
        RecordRunCloseBasisError::Core(error) => RecordingError::Core(error),
        RecordRunCloseBasisError::Validation { field, message } => {
            RecordingError::Rejected(RecordingRejection::Validation { field, message })
        }
    }
}

fn write_ticket_invalid(reason: &'static str, message: &'static str) -> RecordingError {
    RecordingError::Rejected(RecordingRejection::WriteTicketInvalid { reason, message })
}

pub(super) fn plan_record_run_mutations(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    policy: RecordRunPolicyDecision,
    evidence_target_plan: RecordRunEvidenceTargetPlan,
) -> Result<RecordRunPlannedMutations, RecordingError> {
    let RecordRunPolicyDecision {
        facts,
        write_ticket_scope,
        run_id,
        run_ref,
    } = policy;
    let RecordRunFacts {
        normalized,
        task,
        change_unit,
        workflow_policy,
        resolved_control: _,
    } = facts;
    let RecordRunEvidenceTargetPlan {
        claim_mutations: evidence_claim_mutations,
    } = evidence_target_plan;
    let request = &normalized.raw.request;
    let plan_now = &normalized.raw.plan_now;
    let planned_state_version = normalized.planned_state_version;
    let normalized_observed_changes = &normalized.normalized_observed_changes;

    let artifact_context = RecordRunArtifactContext {
        store,
        project_state,
        request,
        verified_invocation,
        run_id: &run_id,
        run_ref: &run_ref,
        now: plan_now,
    };
    let mut artifact_plans = plan_record_run_artifacts(service, artifact_context)?;
    let capture_artifact_context = RecordRunArtifactContext {
        store,
        project_state,
        request,
        verified_invocation,
        run_id: &run_id,
        run_ref: &run_ref,
        now: plan_now,
    };
    let (capture_artifact_plans, capture_authorities) = plan_record_run_capture_authorities(
        service,
        &capture_artifact_context,
        task.scope_revision,
    )?;
    artifact_plans.extend(capture_artifact_plans);
    let registered_artifacts = artifact_plans
        .iter()
        .map(|plan| plan.artifact_ref.clone())
        .collect::<Vec<_>>();
    let observation_context = RecordRunObservationContext {
        service,
        store,
        project_state,
        request,
        verified_invocation,
        run_id: &run_id,
        run_ref: &run_ref,
        registered_artifacts: &registered_artifacts,
        artifact_plans: &artifact_plans,
        capture_authorities: &capture_authorities,
        current_scope_revision: task.scope_revision,
        planned_state_version,
        now: plan_now,
    };
    let observation_plans = plan_record_run_observations(&observation_context)?;
    let evidence_observations = observation_plans
        .iter()
        .map(|plan| plan.observation.clone())
        .collect::<Vec<_>>();
    let evidence_producers = observation_plans
        .iter()
        .filter_map(|plan| plan.producer.clone())
        .collect::<Vec<_>>();
    let observation_refs_by_target = observation_refs_by_target(&observation_plans);

    let acceptance_criteria = active_acceptance_criteria(store, &request.task_id)?;
    let mut recorded_evidence_summary = build_record_run_evidence_summary(
        &observation_context,
        request,
        &run_ref,
        &registered_artifacts,
        &artifact_plans,
        &observation_refs_by_target,
    )?;
    let evidence_summary_id = if recorded_evidence_summary.is_some() {
        Some(
            allocate_evidence_summary_id(service.durable_id_generator(), store)
                .map_err(RecordingError::Core)?,
        )
    } else {
        None
    };
    let evidence_summary_ref = evidence_summary_id.as_ref().map(|id| {
        state_ref(
            StateRecordKind::EvidenceSummary,
            id,
            &request.project_id,
            Some(&request.task_id),
            Some(planned_state_version),
        )
    });
    let close_basis_revision = task.close_basis_revision + 1;
    let close_basis_context = RecordRunCloseBasisContext {
        service,
        store,
        request,
        task: &task,
        run_ref: &run_ref,
        write_ticket_scope: write_ticket_scope.as_ref(),
        evidence_summary_ref: evidence_summary_ref.clone(),
        registered_artifacts: &registered_artifacts,
        close_basis_revision,
        snapshot_state_version: planned_state_version,
        now: plan_now,
    };
    let current_close_basis =
        build_record_run_close_basis(close_basis_context).map_err(record_run_close_basis_error)?;
    recorded_evidence_summary = recorded_evidence_summary
        .map(|summary| evidence_summary_for_display(summary, current_close_basis.as_ref()));
    let projected_close_evidence_summary = evidence_summary_with_required_criteria(
        recorded_evidence_summary.clone(),
        &acceptance_criteria,
    );
    let projected_state_evidence_summary = match recorded_evidence_summary.as_ref() {
        Some(_) => projected_close_evidence_summary.clone(),
        None => {
            let evidence_facts = load_current_evidence_summary_facts(
                store,
                &task,
                &request.project_id,
                &request.task_id,
                planned_state_version,
            )?;
            let required = required_acceptance_criterion_ids(&acceptance_criteria);
            project_close_evidence_summary(evidence_facts, &required)
                .map(|summary| evidence_summary_for_display(summary, current_close_basis.as_ref()))
        }
    };
    let close_basis = current_close_basis.clone();
    let blocker_refs = active_blocker_refs(store, &request.task_id, planned_state_version)?;
    let pending_user_action_refs = pending_refs_after_record_run_invalidation(
        store,
        request,
        planned_state_version,
        plan_now,
    )?;
    let pending_authorities = pending_user_action_authorities(store, &request.task_id, plan_now)
        .map_err(recording_user_action_error)?
        .into_iter()
        .filter(|authority| {
            !matches!(
                authority.action_kind,
                UserActionKind::FinalAcceptance | UserActionKind::ResidualRiskAcceptance
            )
        })
        .collect::<Vec<_>>();
    let lifecycle_phase = projected_user_action_lifecycle_phase(
        project_state,
        &task,
        Some(&change_unit),
        &pending_authorities,
    );
    let mut projected_task = task.clone();
    projected_task.close_basis_revision = close_basis_revision;
    let sensitive_category_acceptance_update = if normalized_observed_changes
        .sensitive_categories
        .is_empty()
        || projected_task.acceptance_policy == AcceptancePolicy::Required
    {
        None
    } else {
        let acceptance_reason = "A recorded sensitive-category signal requires final acceptance without establishing sensitive-action approval authority.".to_owned();
        projected_task.acceptance_policy = AcceptancePolicy::Required;
        projected_task.acceptance_policy_reason = acceptance_reason.clone();
        Some(TaskMutation::UpdateControlLevel(TaskControlLevelUpdate {
            task_id: projected_task.task_id.clone(),
            effective_control_level: projected_task.effective_control_level,
            control_level_reason: projected_task.control_level_reason.clone(),
            acceptance_policy: Some(AcceptancePolicy::Required),
            acceptance_policy_reason: Some(acceptance_reason),
        }))
    };
    if let Some(lifecycle_phase) = lifecycle_phase {
        projected_task.lifecycle_phase = lifecycle_phase;
    }
    let observation_refs = observation_plans
        .iter()
        .map(|plan| plan.observation_ref.clone())
        .collect::<Vec<_>>();

    let mutation_plan = assemble_record_run_mutation_plan(RecordRunMutationAssembly {
        request,
        task: &task,
        workflow_policy: &workflow_policy,
        write_ticket_scope: write_ticket_scope.as_ref(),
        run_id: &run_id,
        normalized_observed_changes,
        close_basis_revision,
        close_basis,
        lifecycle_phase,
        sensitive_category_acceptance_update,
        evidence_claim_mutations,
        artifact_plans: &artifact_plans,
        observation_plans: &observation_plans,
        recorded_evidence_summary: recorded_evidence_summary.as_ref(),
        evidence_summary_id: evidence_summary_id.as_ref(),
        registered_artifacts: &registered_artifacts,
        verified_invocation,
    })?;
    let residual_risk_ids = current_close_basis
        .as_ref()
        .map(|basis| {
            basis
                .residual_risks
                .iter()
                .map(|risk| risk.risk_id.as_str().to_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let event_payload = object_from_value(json!({
        "task_id": request.task_id,
        "change_unit_id": request.change_unit_id,
        "run_id": run_id,
        "source_run_ref": run_ref,
        "scope_revision": task.scope_revision,
        "close_basis_revision": close_basis_revision,
        "residual_risk_ids": residual_risk_ids,
        "kind": request.kind,
        "product_file_write_observed": normalized_observed_changes.product_file_write_observed,
        "write_ticket_id": write_ticket_scope
            .as_ref()
            .map(|(record, _scope)| record.write_ticket_id().to_owned()),
        "artifact_ids": registered_artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.as_str().to_owned())
            .collect::<Vec<_>>(),
        "evidence_observation_ids": evidence_observations
            .iter()
            .map(|observation| observation.observation_id.as_str().to_owned())
            .collect::<Vec<_>>()
    }))?;

    let RecordRunNormalizedRequest {
        raw,
        normalized_observed_changes,
        ..
    } = normalized;
    Ok(RecordRunPlannedMutations {
        request: raw.request,
        plan_now: raw.plan_now,
        planned_state_version,
        change_unit,
        write_ticket_scope,
        run_id,
        run_ref,
        normalized_observed_changes,
        registered_artifacts,
        evidence_observations,
        observation_refs,
        evidence_producers,
        acceptance_criteria,
        recorded_evidence_summary,
        projected_close_evidence_summary,
        projected_state_evidence_summary,
        current_close_basis,
        blocker_refs,
        pending_user_action_refs,
        pending_authorities,
        projected_task,
        mutation_plan,
        event_payload,
    })
}

pub(super) fn assemble_record_run_mutation_plan(
    assembly: RecordRunMutationAssembly<'_>,
) -> Result<RecordRunMutationPlan, RecordingError> {
    let RecordRunMutationAssembly {
        request,
        task,
        workflow_policy,
        write_ticket_scope,
        run_id,
        normalized_observed_changes,
        close_basis_revision,
        close_basis,
        lifecycle_phase,
        sensitive_category_acceptance_update,
        evidence_claim_mutations,
        artifact_plans,
        observation_plans,
        recorded_evidence_summary,
        evidence_summary_id,
        registered_artifacts,
        verified_invocation,
    } = assembly;
    let mut steps = vec![RecordRunMutation::Run(RunMutation::Insert(RunInsert {
        run_id: run_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        change_unit_id: Some(request.change_unit_id.as_str().to_owned()),
        scope_revision: task.scope_revision,
        write_ticket_id: request
            .write_ticket_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        kind: request.kind,
        status: RunStatus::Recorded,
        summary: StoredRunSummary {
            summary: request.summary.clone(),
        },
        observed_changes: normalized_observed_changes.clone(),
        evidence_updates: request.evidence_updates.clone(),
        write_ticket_effect: StoredRunWriteTicketEffect {
            write_ticket_id: request.write_ticket_id.as_ref().cloned(),
            effect: if write_ticket_scope.is_some() {
                StoredRunWriteTicketEffectKind::Consumed
            } else {
                StoredRunWriteTicketEffectKind::None
            },
        },
        created_by_actor_source: verified_invocation.actor_source.clone(),
        metadata: StoredRunMetadata {
            verification_basis: verified_invocation.verification_basis.clone(),
        },
    }))];
    if let Some(acceptance_update) = sensitive_category_acceptance_update {
        steps.push(RecordRunMutation::Task(acceptance_update));
    }
    steps.push(RecordRunMutation::Task(TaskMutation::UpdateCloseBasis(
        TaskCloseBasisUpdate {
            task_id: request.task_id.as_str().to_owned(),
            close_basis_revision,
            close_basis,
        },
    )));
    steps.push(RecordRunMutation::UserAction(
        UserActionMutation::MarkSupersededOrStale(UserActionInvalidation {
            task_id: request.task_id.as_str().to_owned(),
            action_kinds: vec![
                UserActionKind::FinalAcceptance,
                UserActionKind::ResidualRiskAcceptance,
            ],
        }),
    ));
    if let Some(lifecycle_phase) = lifecycle_phase {
        if let Some(transition) =
            plan_user_action_lifecycle_transition(TaskLifecycleFacts::from(task), lifecycle_phase)?
        {
            steps.push(RecordRunMutation::Task(transition.task_mutation()));
        }
    }
    if let Some((record, _scope)) = write_ticket_scope {
        steps.push(RecordRunMutation::WriteTicket(
            WriteTicketMutation::Consume(WriteTicketConsumption {
                write_ticket_id: record.write_ticket_id().to_owned(),
                run_id: run_id.as_str().to_owned(),
                expected_basis_state_version: record.basis_state_version(),
                expected_write_authority_fingerprint: workflow_policy
                    .write_authority_fingerprint
                    .clone(),
            }),
        ));
    }
    steps.extend(
        evidence_claim_mutations
            .into_iter()
            .map(Box::new)
            .map(RecordRunMutation::Evidence),
    );
    for plan in artifact_plans {
        if let Some(mutation) = &plan.source_mutation {
            steps.push(RecordRunMutation::Artifact(mutation.clone()));
        }
        steps.push(RecordRunMutation::Artifact(plan.run_link.clone()));
    }
    for plan in observation_plans {
        steps.push(RecordRunMutation::Evidence(Box::new(plan.mutation.clone())));
        for artifact_ref in &plan.observation.output_artifact_refs {
            steps.push(RecordRunMutation::Artifact(ArtifactMutation::Link(
                ArtifactLinkInsert {
                    artifact_id: artifact_ref.artifact_id.as_str().to_owned(),
                    task_id: request.task_id.as_str().to_owned(),
                    owner_record_kind: StateRecordKind::EvidenceObservation,
                    owner_record_id: plan.observation.observation_id.as_str().to_owned(),
                    created_by_run_id: run_id.as_str().to_owned(),
                    metadata: object_from_value(json!({
                        "relation": "evidence_observation_output"
                    }))?,
                },
            )));
        }
        if let Some(producer_mutation) = &plan.producer_mutation {
            steps.push(RecordRunMutation::Evidence(Box::new(
                producer_mutation.clone(),
            )));
        }
        if let Some(producer) = &plan.producer {
            for artifact_ref in &producer.receipt_artifact_refs {
                steps.push(RecordRunMutation::Artifact(ArtifactMutation::Link(
                    ArtifactLinkInsert {
                        artifact_id: artifact_ref.artifact_id.as_str().to_owned(),
                        task_id: request.task_id.as_str().to_owned(),
                        owner_record_kind: StateRecordKind::EvidenceProducer,
                        owner_record_id: producer.evidence_producer_id.as_str().to_owned(),
                        created_by_run_id: run_id.as_str().to_owned(),
                        metadata: object_from_value(json!({
                            "relation": "evidence_capture_receipt"
                        }))?,
                    },
                )));
            }
        }
    }
    if let (Some(evidence_summary), Some(evidence_summary_id)) =
        (recorded_evidence_summary, evidence_summary_id)
    {
        steps.push(RecordRunMutation::Evidence(Box::new(
            EvidenceMutation::UpsertSummary(EvidenceSummaryUpsert {
                evidence_summary_id: evidence_summary_id.clone(),
                task_id: request.task_id.as_str().to_owned(),
                change_unit_id: Some(request.change_unit_id.as_str().to_owned()),
                status: evidence_summary.status,
                coverage: evidence_summary.coverage_items.clone(),
                supporting_refs: evidence_summary
                    .coverage_items
                    .iter()
                    .flat_map(|item| item.supporting_run_refs.clone())
                    .collect(),
                gap_refs: evidence_summary
                    .coverage_items
                    .iter()
                    .flat_map(|item| item.gap_refs.clone())
                    .collect(),
                metadata: PersistedEvidenceMetadata {
                    updated_by_run_id: run_id.clone(),
                },
            }),
        )));
        for artifact_ref in registered_artifacts {
            steps.push(RecordRunMutation::Artifact(ArtifactMutation::Link(
                ArtifactLinkInsert {
                    artifact_id: artifact_ref.artifact_id.as_str().to_owned(),
                    task_id: request.task_id.as_str().to_owned(),
                    owner_record_kind: StateRecordKind::EvidenceSummary,
                    owner_record_id: evidence_summary_id.clone(),
                    created_by_run_id: run_id.as_str().to_owned(),
                    metadata: object_from_value(json!({
                        "relation": "evidence_support"
                    }))?,
                },
            )));
        }
    }
    Ok(RecordRunMutationPlan { steps })
}

pub(super) fn pending_refs_after_record_run_invalidation(
    store: &CoreProjectStore,
    request: &RecordRunInput,
    planned_state_version: u64,
    now: &UtcTimestamp,
) -> Result<Vec<StateRecordRef>, RecordingError> {
    let invalidated_kinds = [
        UserActionKind::FinalAcceptance,
        UserActionKind::ResidualRiskAcceptance,
    ];
    let mut refs = Vec::new();
    for record_ref in store
        .pending_user_action_refs(&request.task_id, planned_state_version, now)
        .map_err(recording_store_error)?
    {
        let record = store
            .user_action_record(&record_ref.record_id, now)
            .map_err(recording_store_error)?;
        if record
            .as_ref()
            .is_some_and(|record| invalidated_kinds.contains(&record.request().action_kind()))
        {
            continue;
        }
        refs.push(state_ref_from_stored(record_ref));
    }
    Ok(refs)
}
