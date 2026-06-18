use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path},
};

use harness_store::{
    artifacts::{ArtifactStagingInsert, StagedPayloadKind},
    core_pipeline::{
        ArtifactLinkInsert, ArtifactPromotion, ChangeUnitInsert, ChangeUnitRecord,
        CoreProjectStore, CoreStorageMutation, EvidenceSummaryRecord, EvidenceSummaryUpsert,
        ProjectStateHeader, RunInsert, StoredArtifactRecord, StoredArtifactStagingRecord,
        StoredRecordRef, TaskCloseUpdate, TaskInsert, TaskRecord, TaskScopeUpdate,
        UserJudgmentInsert, UserJudgmentRecord, UserJudgmentResolutionUpdate,
        WriteAuthorizationConsumption, WriteAuthorizationInsert, WriteAuthorizationRecord,
    },
    StoreError,
};
use harness_types::{
    AccessClass, ArtifactAvailability, ArtifactId, ArtifactInput, ArtifactInputSourceKind,
    ArtifactRef, AuthorizationEffect, AuthorizedAttemptScope, BaselineRef, ChangeUnitId,
    ChangeUnitOperation, CloseIntent, CloseReadinessBlocker, CloseReadinessBlockerCategory,
    CloseReason, CloseState, CloseTaskRequest, CloseTaskResult, CompletionPolicy, DryRunSummary,
    DurableIdKind, EffectKind, ErrorCode, EvidenceCoverageItem, EvidenceCoverageState,
    EvidenceStatus, EvidenceSummary, GuaranteeDisplay, GuaranteeLevel, JsonObject, JudgmentKind,
    MethodName, NextActionKind, NextActionSummary, ObservedChanges, PlannedBlocker,
    PlannedBlockerSourceKind, PlannedEffect, PrepareWriteDecision, PrepareWriteRequest,
    PrepareWriteResult, ProjectId, RecordId, RecordRunRequest, RecordRunResult,
    RecordUserJudgmentPayload, RecordUserJudgmentRequest, RedactionState, RequestedMode,
    ResumePolicy, RunId, RunSummary, SensitiveActionScope, StageArtifactRequest,
    StageArtifactResult, StagedArtifactHandle, StagedArtifactHandleId, StateRecordKind,
    StateRecordRef, StatusCloseState, StatusInclude, StatusRequest, StorageRef, SurfaceId,
    SurfaceInstanceId, TaskId, TaskLifecyclePhase, TaskLifecycleState, TaskMode, TaskResult,
    ToolEnvelope, ToolResultBase, UpdateScopeRequest, UserJudgment, UserJudgmentContext,
    UserJudgmentOption, UserJudgmentResolution, UserJudgmentStatus, WriteAuthoritySummary,
    WriteAuthorizationId, WriteAuthorizationStatus, WriteAuthorizationSummary,
    WriteDecisionCategory, WriteDecisionReason,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::pipeline::{
    dry_run_response, method_result_base, rejected_response, tool_error, CorePipelineError,
    CoreResult, CoreService, FreshnessPolicy, InvocationContext, MethodEffectPolicy, MethodPolicy,
    OwnerPipelineBranch, PipelinePreflightOutcome, PipelinePreflightRequest, PipelineResponse,
    PreparedRequest, ReplayPolicy, TaskRequirement, VerifiedSurfaceContext,
};

impl CoreService {
    /// Executes `harness.status` as a read-only Core result.
    pub fn status(
        &self,
        request: StatusRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        let prepared = match prepare_or_response(
            self,
            MethodName::Status,
            request.envelope.clone(),
            request_json,
            invocation,
            MethodPolicy::exact(
                AccessClass::ReadStatus,
                TaskRequirement::Optional,
                ReplayPolicy::None,
                FreshnessPolicy::None,
                MethodEffectPolicy::ReadOnly,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };

        let task = status_task(
            &prepared.store,
            &prepared.context.project_state,
            request.envelope.task_id.as_ref(),
        )?;
        let result_fields = status_result_fields(
            &prepared.store,
            &request.envelope.project_id,
            prepared.context.project_state.state_version,
            task.as_ref(),
            &request.include,
        )?;

        self.execute_prepared_request(prepared, OwnerPipelineBranch::ReadOnly { result_fields })
    }

    /// Executes `harness.intake` through the shared Core mutation pipeline.
    pub fn intake(
        &self,
        request: harness_types::IntakeRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        let policy = mutation_method_policy(
            AccessClass::CoreMutation,
            TaskRequirement::None,
            request.envelope.dry_run,
        );
        let prepared = match prepare_or_response(
            self,
            MethodName::Intake,
            request.envelope.clone(),
            request_json,
            invocation,
            policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let store = &prepared.store;
        let project_state = &prepared.context.project_state;
        if request.resume_policy == ResumePolicy::RejectIfActive
            && project_state.active_task_id.is_some()
        {
            return validation_rejected(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "resume_policy",
                "resume_policy=reject_if_active cannot proceed while a Task is active",
            );
        }

        let plan = plan_intake(
            self,
            store,
            project_state,
            request.clone(),
            &prepared.context.verified_surface,
        )?;

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: dry_run_summary(
                        "task",
                        "commit",
                        "Intake would select or create a Task.",
                        plan.next_actions,
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "task_intake".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: None,
                storage_mutations: plan.storage_mutations,
            },
        )
    }

    /// Executes `harness.update_scope` through the shared Core mutation pipeline.
    pub fn update_scope(
        &self,
        request: UpdateScopeRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = &request.envelope.task_id {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match UpdateScopeRequest.task_id",
                );
            }
        }
        let policy = mutation_method_policy(
            AccessClass::CoreMutation,
            TaskRequirement::Exact(request.task_id.clone()),
            request.envelope.dry_run,
        );
        let prepared = match prepare_or_response(
            self,
            MethodName::UpdateScope,
            request.envelope.clone(),
            request_json,
            invocation,
            policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_update_scope(
            self,
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
        ) {
            Ok(plan) => plan,
            Err(PlanError::Response(response)) => return Ok(*response),
            Err(PlanError::Core(error)) => return Err(error),
        };

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: dry_run_summary(
                        "scope",
                        "commit",
                        "Scope update would update current Task scope and Change Unit state.",
                        plan.next_actions,
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "scope_updated".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }

    /// Executes `harness.prepare_write` through the shared Core mutation pipeline.
    pub fn prepare_write(
        &self,
        request: PrepareWriteRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = &request.envelope.task_id {
            if request
                .task_id
                .as_ref()
                .is_some_and(|task_id| task_id != envelope_task_id)
            {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match PrepareWriteRequest.task_id",
                );
            }
        }
        let policy = prepare_write_policy(&request);
        let prepared = match prepare_or_response(
            self,
            MethodName::PrepareWrite,
            request.envelope.clone(),
            request_json,
            invocation,
            policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_prepare_write(
            self,
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
            &prepared.context.verified_surface,
        ) {
            Ok(plan) => plan,
            Err(PlanError::Response(response)) => return Ok(*response),
            Err(PlanError::Core(error)) => return Err(error),
        };

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: plan.dry_run_summary,
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: plan.event_kind,
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }

    /// Executes `harness.stage_artifact` as storage-owned transient staging.
    pub fn stage_artifact(
        &self,
        request: StageArtifactRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = &request.envelope.task_id {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match StageArtifactRequest.task_id",
                );
            }
        }

        let policy = MethodPolicy::exact(
            AccessClass::ArtifactRegistration,
            TaskRequirement::Exact(request.task_id.clone()),
            ReplayPolicy::None,
            FreshnessPolicy::IfPresent,
            if request.envelope.dry_run {
                MethodEffectPolicy::DryRunPreview
            } else {
                MethodEffectPolicy::Staging
            },
        );
        let mut prepared = match prepare_or_response(
            self,
            MethodName::StageArtifact,
            request.envelope.clone(),
            request_json,
            invocation,
            policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let project_state = prepared.context.project_state.clone();
        let verified_surface = prepared.context.verified_surface.clone();

        let stage_input = match validate_stage_artifact_input(&request) {
            Ok(input) => input,
            Err(errors) => {
                return rejected_pipeline_response(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    errors,
                );
            }
        };
        if !surface_supports_artifact_staging(&verified_surface.capability_profile) {
            return rejected_pipeline_response(
                request.envelope.dry_run,
                Some(project_state.state_version),
                vec![capability_error(
                    "surface lacks manual artifact attachment support",
                    Some(json!({
                        "required_capability": "manual_artifact_attachment_supported"
                    })),
                )],
            );
        }

        if request.envelope.dry_run {
            let response = dry_run_response(
                Some(project_state.state_version),
                dry_run_summary(
                    "artifact_staging",
                    "would_stage",
                    "Stage artifact would create one transient staged handle.",
                    Vec::new(),
                ),
            );
            let response_value = serde_json::to_value(response)?;
            let response_json = serde_json::to_string(&response_value)?;
            return Ok(PipelineResponse {
                response_json,
                response_value,
                verified_surface: Some(verified_surface),
                resolved_task_id: Some(request.task_id),
                replayed: false,
            });
        }

        let handle_id = allocate_staged_artifact_handle_id(self, &prepared.store)?;
        let staging_record = prepared
            .store
            .create_artifact_staging(ArtifactStagingInsert {
                handle_id: handle_id.into_inner(),
                task_id: request.task_id.as_str().to_owned(),
                created_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
                created_by_surface_instance_id: verified_surface
                    .surface_instance_id
                    .as_str()
                    .to_owned(),
                display_name: request.display_name,
                content_type: request.content_type,
                sha256: stage_input.sha256.clone(),
                size_bytes: stage_input.size_bytes,
                redaction_state: redaction_state_value(request.redaction_state).to_owned(),
                relation_hint: request.relation_hint,
                payload_kind: stage_input.payload_kind,
                safe_bytes_or_notice: stage_input.safe_bytes,
            })?;

        let resolved_task_id = TaskId::new(staging_record.task_id.clone());
        let handle = StagedArtifactHandle {
            handle_id: StagedArtifactHandleId::new(staging_record.handle_id),
            project_id: request.envelope.project_id.clone(),
            task_id: resolved_task_id.clone(),
            created_by_surface_id: SurfaceId::new(staging_record.created_by_surface_id),
            created_by_surface_instance_id: SurfaceInstanceId::new(
                staging_record.created_by_surface_instance_id,
            ),
            content_type: staging_record.content_type,
            sha256: staging_record.sha256,
            size_bytes: staging_record.size_bytes,
            redaction_state: request.redaction_state,
            expires_at: staging_record.expires_at.clone(),
            consumed: false,
        };
        let result = StageArtifactResult {
            base: method_result_base(
                EffectKind::StagingCreated,
                false,
                Some(project_state.state_version),
                Vec::new(),
            ),
            staged_artifact_handle: handle,
            expires_at: staging_record.expires_at,
        };
        let response_value = serde_json::to_value(result)?;
        let response_json = serde_json::to_string(&response_value)?;
        Ok(PipelineResponse {
            response_json,
            response_value,
            verified_surface: Some(verified_surface),
            resolved_task_id: Some(resolved_task_id),
            replayed: false,
        })
    }

    /// Executes `harness.record_run` through the shared Core mutation pipeline.
    pub fn record_run(
        &self,
        request: RecordRunRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = &request.envelope.task_id {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match RecordRunRequest.task_id",
                );
            }
        }
        let prepared = match prepare_or_response(
            self,
            MethodName::RecordRun,
            request.envelope.clone(),
            request_json,
            invocation,
            mutation_method_policy(
                AccessClass::RunRecording,
                TaskRequirement::Exact(request.task_id.clone()),
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_record_run(
            self,
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
            &prepared.context.verified_surface,
        ) {
            Ok(plan) => plan,
            Err(PlanError::Response(response)) => return Ok(*response),
            Err(PlanError::Core(error)) => return Err(error),
        };

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: dry_run_summary(
                        "run",
                        "would_record",
                        "Record run would create one Run and any compatible evidence or artifact links.",
                        Vec::new(),
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "run_recorded".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }

    /// Executes `harness.request_user_judgment` through the shared Core mutation pipeline.
    pub fn request_user_judgment(
        &self,
        request: harness_types::RequestUserJudgmentRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = &request.envelope.task_id {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match RequestUserJudgmentRequest.task_id",
                );
            }
        }
        let prepared = match prepare_or_response(
            self,
            MethodName::RequestUserJudgment,
            request.envelope.clone(),
            request_json,
            invocation,
            mutation_method_policy(
                AccessClass::CoreMutation,
                TaskRequirement::Exact(request.task_id.clone()),
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_request_user_judgment(
            self,
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
            &prepared.context.verified_surface,
        ) {
            Ok(plan) => plan,
            Err(PlanError::Response(response)) => return Ok(*response),
            Err(PlanError::Core(error)) => return Err(error),
        };

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: dry_run_summary(
                        "user_judgment",
                        "create_pending",
                        "Request would create one pending user-owned judgment.",
                        plan.next_actions,
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "user_judgment_requested".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }

    /// Executes `harness.record_user_judgment` through the shared Core mutation pipeline.
    pub fn record_user_judgment(
        &self,
        request: RecordUserJudgmentRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        let task_requirement = request
            .envelope
            .task_id
            .clone()
            .map(TaskRequirement::Exact)
            .unwrap_or(TaskRequirement::None);
        let prepared = match prepare_or_response(
            self,
            MethodName::RecordUserJudgment,
            request.envelope.clone(),
            request_json,
            invocation,
            mutation_method_policy(
                AccessClass::CoreMutation,
                task_requirement,
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_record_user_judgment(
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
        ) {
            Ok(plan) => plan,
            Err(PlanError::Response(response)) => return Ok(*response),
            Err(PlanError::Core(error)) => return Err(error),
        };

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: dry_run_summary(
                        "user_judgment",
                        "resolve_pending",
                        "Request would record the user's answer for one pending judgment.",
                        plan.next_actions,
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "user_judgment_recorded".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }

    /// Executes `harness.close_task` through close-readiness and terminal transition rules.
    pub fn close_task(
        &self,
        request: CloseTaskRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = &request.envelope.task_id {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match CloseTaskRequest.task_id",
                );
            }
        } else {
            return validation_rejected(
                request.envelope.dry_run,
                None,
                "envelope.task_id",
                "close_task requires envelope.task_id to identify the Task being closed",
            );
        }
        if let Some(response) = validate_close_intent_fields(&request)? {
            return Ok(response);
        }
        let close_policy = close_task_policy(&request);
        let prepared = match prepare_or_response(
            self,
            MethodName::CloseTask,
            request.envelope.clone(),
            request_json,
            invocation,
            close_policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        if request.intent != CloseIntent::Check {
            if let Some(response) = reject_stale_close_write_authorization(
                &prepared.store,
                &prepared.context.project_state,
                &request,
            )? {
                return Ok(response);
            }
        }

        if request.intent == CloseIntent::Check {
            let plan = match plan_close_task(
                &prepared.store,
                &prepared.context.project_state,
                request.clone(),
            ) {
                Ok(plan) => plan,
                Err(PlanError::Response(response)) => return Ok(*response),
                Err(PlanError::Core(error)) => return Err(error),
            };
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::ReadOnly {
                    result_fields: plan.result_fields,
                },
            );
        }

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: close_task_dry_run_summary(request.intent),
                },
            );
        }

        let plan = match plan_close_task(
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
        ) {
            Ok(plan) => plan,
            Err(PlanError::Response(response)) => return Ok(*response),
            Err(PlanError::Core(error)) => return Err(error),
        };

        if !plan.blockers.is_empty() {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::NoEffectResult {
                    result_fields: plan.result_fields,
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: plan.event_kind,
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }
}

struct MethodPlan {
    task_id: TaskId,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    event_payload: JsonObject,
    result_fields: JsonObject,
    next_actions: Vec<NextActionSummary>,
}

struct PrepareWritePlan {
    task_id: TaskId,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
    result_fields: JsonObject,
    dry_run_summary: DryRunSummary,
}

struct CloseTaskPlan {
    task_id: TaskId,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
    result_fields: JsonObject,
    blockers: Vec<CloseReadinessBlocker>,
}

struct CloseTaskContext {
    task: TaskRecord,
    current_change_unit: Option<ChangeUnitRecord>,
    pending_user_judgment_refs: Vec<StateRecordRef>,
    blocker_refs: Vec<StateRecordRef>,
    evidence_summary: Option<EvidenceSummary>,
    artifact_refs: Vec<ArtifactRef>,
}

struct ValidatedStageArtifactInput {
    safe_bytes: Vec<u8>,
    sha256: String,
    size_bytes: u64,
    payload_kind: StagedPayloadKind,
}

enum PlanError {
    Core(CorePipelineError),
    Response(Box<PipelineResponse>),
}

impl From<CorePipelineError> for PlanError {
    fn from(error: CorePipelineError) -> Self {
        Self::Core(error)
    }
}

impl From<serde_json::Error> for PlanError {
    fn from(error: serde_json::Error) -> Self {
        Self::Core(CorePipelineError::from(error))
    }
}

fn allocate_task_id(service: &CoreService, store: &CoreProjectStore) -> CoreResult<TaskId> {
    service
        .allocate_generated_id(DurableIdKind::Task, |candidate| {
            store
                .task_exists(&TaskId::new(candidate))
                .map_err(CorePipelineError::from)
        })
        .map(TaskId::new)
}

fn allocate_change_unit_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<ChangeUnitId> {
    service
        .allocate_generated_id(DurableIdKind::ChangeUnit, |candidate| {
            store
                .change_unit_id_exists(candidate)
                .map_err(CorePipelineError::from)
        })
        .map(ChangeUnitId::new)
}

fn allocate_user_judgment_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<harness_types::UserJudgmentId> {
    service
        .allocate_generated_id(DurableIdKind::UserJudgment, |candidate| {
            store
                .user_judgment_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(harness_types::UserJudgmentId::new)
}

fn allocate_write_authorization_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<WriteAuthorizationId> {
    service
        .allocate_generated_id(DurableIdKind::WriteAuthorization, |candidate| {
            store
                .write_authorization_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(WriteAuthorizationId::new)
}

fn allocate_run_id(service: &CoreService, store: &CoreProjectStore) -> CoreResult<RunId> {
    service
        .allocate_generated_id(DurableIdKind::Run, |candidate| {
            store
                .run_id_exists(candidate)
                .map_err(CorePipelineError::from)
        })
        .map(RunId::new)
}

fn allocate_staged_artifact_handle_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<StagedArtifactHandleId> {
    service
        .allocate_generated_id(DurableIdKind::StagedArtifact, |candidate| {
            store
                .artifact_staging_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(StagedArtifactHandleId::new)
}

fn allocate_artifact_id(service: &CoreService, store: &CoreProjectStore) -> CoreResult<ArtifactId> {
    service
        .allocate_generated_id(DurableIdKind::Artifact, |candidate| {
            store
                .artifact_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(ArtifactId::new)
}

fn allocate_evidence_summary_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<String> {
    service.allocate_generated_id(DurableIdKind::Evidence, |candidate| {
        store
            .evidence_summary_exists(candidate)
            .map_err(CorePipelineError::from)
    })
}

fn prepare_or_response(
    service: &CoreService,
    method_name: MethodName,
    envelope: ToolEnvelope,
    request_json: Value,
    invocation: InvocationContext,
    policy: MethodPolicy,
) -> CoreResult<Result<PreparedRequest, PipelineResponse>> {
    match service.prepare_request(PipelinePreflightRequest {
        method_name,
        envelope,
        request_json,
        invocation,
        policy,
    })? {
        PipelinePreflightOutcome::Prepared(prepared) => Ok(Ok(*prepared)),
        PipelinePreflightOutcome::Response(response) => Ok(Err(*response)),
    }
}

fn mutation_method_policy(
    access_class: AccessClass,
    task: TaskRequirement,
    dry_run: bool,
) -> MethodPolicy {
    if dry_run {
        MethodPolicy::exact(
            access_class,
            task,
            ReplayPolicy::None,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::DryRunPreview,
        )
    } else {
        MethodPolicy::exact(
            access_class,
            task,
            ReplayPolicy::Committed,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::CoreMutation,
        )
    }
}

fn prepare_write_policy(request: &PrepareWriteRequest) -> MethodPolicy {
    let task = request
        .task_id
        .clone()
        .or_else(|| request.envelope.task_id.clone())
        .map(TaskRequirement::Exact)
        .unwrap_or(TaskRequirement::Required);

    if request.envelope.dry_run {
        MethodPolicy::verified_grant_only(
            task,
            ReplayPolicy::None,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::DryRunPreview,
        )
    } else {
        MethodPolicy::verified_grant_only(
            task,
            ReplayPolicy::Committed,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::CoreMutation,
        )
    }
}

fn close_task_policy(request: &CloseTaskRequest) -> MethodPolicy {
    let task = TaskRequirement::Exact(request.task_id.clone());
    if request.intent == CloseIntent::Check {
        MethodPolicy::exact(
            AccessClass::ReadStatus,
            task,
            ReplayPolicy::None,
            FreshnessPolicy::None,
            MethodEffectPolicy::ReadOnly,
        )
    } else {
        mutation_method_policy(AccessClass::CoreMutation, task, request.envelope.dry_run)
    }
}

const MAX_STAGED_BODY_BYTES: usize = 10 * 1024 * 1024;

fn validate_stage_artifact_input(
    request: &StageArtifactRequest,
) -> Result<ValidatedStageArtifactInput, Vec<harness_types::ToolError>> {
    let mut errors = Vec::new();
    validate_stage_envelope(&request.envelope, &mut errors);
    validate_stage_text_field("task_id", request.task_id.as_str(), &mut errors);
    validate_stage_text_field("display_name", &request.display_name, &mut errors);
    validate_stage_text_field("content_type", &request.content_type, &mut errors);

    let safe_bytes = request.safe_bytes_or_notice.as_bytes().to_vec();
    if safe_bytes.is_empty() {
        errors.push(stage_validation_error(
            "safe_bytes_or_notice",
            "safe_bytes_or_notice must not be empty",
        ));
    }
    if safe_bytes.len() > MAX_STAGED_BODY_BYTES {
        errors.push(stage_validation_error(
            "safe_bytes_or_notice",
            "safe_bytes_or_notice exceeds the 10 MiB staging limit",
        ));
    }
    if contains_obvious_raw_secret(&request.safe_bytes_or_notice) {
        errors.push(stage_validation_error(
            "safe_bytes_or_notice",
            "raw secret-like content must be omitted or replaced with a safe notice",
        ));
    }

    let media_type = normalized_media_type(&request.content_type);
    let textual_media_type = media_type
        .as_deref()
        .is_some_and(is_safe_textual_media_type);
    let payload_kind = if matches!(
        request.redaction_state,
        RedactionState::SecretOmitted | RedactionState::Blocked
    ) {
        StagedPayloadKind::SafeNotice
    } else if textual_media_type {
        StagedPayloadKind::SafeTextBody
    } else {
        StagedPayloadKind::SafeNotice
    };
    if media_type.is_none() {
        errors.push(stage_validation_error(
            "content_type",
            "content_type must be a valid media type",
        ));
    }
    if !textual_media_type
        && !matches!(
            request.redaction_state,
            RedactionState::SecretOmitted | RedactionState::Blocked
        )
    {
        errors.push(stage_validation_error(
            "content_type",
            "binary or unsupported content types must be represented by a safe notice",
        ));
    }

    let size_bytes = safe_bytes.len() as u64;
    if let Some(expected_size_bytes) = request.expected_size_bytes {
        if expected_size_bytes != size_bytes {
            errors.push(stage_validation_error(
                "expected_size_bytes",
                "expected_size_bytes does not match safe_bytes_or_notice byte length",
            ));
        }
    }
    let sha256 = sha256_string(&safe_bytes);
    if let Some(expected_sha256) = &request.expected_sha256 {
        if expected_sha256.trim().is_empty() {
            errors.push(stage_validation_error(
                "expected_sha256",
                "expected_sha256 must not be empty when present",
            ));
        } else if expected_sha256 != &sha256 {
            errors.push(stage_validation_error(
                "expected_sha256",
                "expected_sha256 does not match safe_bytes_or_notice",
            ));
        }
    }

    if errors.is_empty() {
        Ok(ValidatedStageArtifactInput {
            safe_bytes,
            sha256,
            size_bytes,
            payload_kind,
        })
    } else {
        Err(errors)
    }
}

fn validate_stage_envelope(envelope: &ToolEnvelope, errors: &mut Vec<harness_types::ToolError>) {
    validate_stage_text_field("project_id", envelope.project_id.as_str(), errors);
    if let Some(task_id) = &envelope.task_id {
        validate_stage_text_field("envelope.task_id", task_id.as_str(), errors);
    }
    validate_stage_text_field("surface_id", envelope.surface_id.as_str(), errors);
    validate_stage_text_field("request_id", envelope.request_id.as_str(), errors);
    if let Some(idempotency_key) = &envelope.idempotency_key {
        validate_stage_text_field("idempotency_key", idempotency_key.as_str(), errors);
    }
}

fn validate_stage_text_field(
    field: &'static str,
    value: &str,
    errors: &mut Vec<harness_types::ToolError>,
) {
    if value.trim().is_empty() {
        errors.push(stage_validation_error(field, "field must not be empty"));
    } else if value.chars().any(char::is_control) {
        errors.push(stage_validation_error(
            field,
            "field must not contain control characters",
        ));
    }
}

fn normalized_media_type(content_type: &str) -> Option<String> {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let (top, subtype) = media_type.split_once('/')?;
    if top.is_empty()
        || subtype.is_empty()
        || media_type.chars().any(char::is_whitespace)
        || media_type.chars().any(char::is_control)
    {
        None
    } else {
        Some(media_type)
    }
}

fn is_safe_textual_media_type(media_type: &str) -> bool {
    if media_type.starts_with("text/") {
        return true;
    }
    matches!(
        media_type,
        "application/json"
            | "application/xml"
            | "application/markdown"
            | "application/x-ndjson"
            | "application/yaml"
            | "application/x-yaml"
            | "application/toml"
            | "application/javascript"
            | "application/ecmascript"
    ) || media_type.ends_with("+json")
        || media_type.ends_with("+xml")
}

fn contains_obvious_raw_secret(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "password=",
        "passwd=",
        "secret=",
        "token=",
        "api_key=",
        "apikey=",
        "aws_secret_access_key",
        "authorization: bearer ",
        "-----begin private key-----",
        "-----begin rsa private key-----",
        "-----begin openssh private key-----",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn sha256_string(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{}", lowercase_hex(&digest))
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn stage_validation_error(field: &'static str, message: &'static str) -> harness_types::ToolError {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    tool_error(ErrorCode::ValidationFailed, message, false, Some(details))
}

fn capability_error(message: &'static str, details: Option<Value>) -> harness_types::ToolError {
    let details = details.and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    tool_error(ErrorCode::CapabilityInsufficient, message, false, details)
}

fn surface_supports_artifact_staging(capability_profile: &Value) -> bool {
    surface_declares_artifact_registration(capability_profile)
        && capability_profile
            .get("manual_artifact_attachment_supported")
            .and_then(Value::as_bool)
            .or_else(|| {
                capability_profile
                    .pointer("/capabilities/manual_artifact_attachment_supported")
                    .and_then(Value::as_bool)
            })
            == Some(true)
}

fn surface_declares_artifact_registration(capability_profile: &Value) -> bool {
    if capability_profile
        .get("supported_access_classes")
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values
                .iter()
                .any(|value| value.as_str() == Some("artifact_registration"))
        })
    {
        return true;
    }
    if capability_profile
        .get("access_class")
        .and_then(Value::as_str)
        == Some("artifact_registration")
    {
        return true;
    }
    capability_profile
        .pointer("/capabilities/artifact_registration")
        .and_then(Value::as_bool)
        == Some(true)
}

fn redaction_state_value(redaction_state: RedactionState) -> &'static str {
    match redaction_state {
        RedactionState::None => "none",
        RedactionState::Redacted => "redacted",
        RedactionState::SecretOmitted => "secret_omitted",
        RedactionState::Blocked => "blocked",
    }
}

fn validate_close_intent_fields(
    request: &CloseTaskRequest,
) -> CoreResult<Option<PipelineResponse>> {
    let invalid = |field, message| {
        validation_rejected(request.envelope.dry_run, None, field, message).map(Some)
    };
    match request.intent {
        CloseIntent::Check => {
            if request.close_reason.is_some() {
                return invalid("close_reason", "intent=check must not include close_reason");
            }
            if request.superseding_task_id.is_some() {
                return invalid(
                    "superseding_task_id",
                    "intent=check must not include superseding_task_id",
                );
            }
        }
        CloseIntent::Complete => {
            if !matches!(
                request.close_reason,
                Some(CloseReason::CompletedSelfChecked | CloseReason::CompletedWithRiskAccepted)
            ) {
                return invalid(
                    "close_reason",
                    "intent=complete requires a completion close_reason",
                );
            }
            if request.superseding_task_id.is_some() {
                return invalid(
                    "superseding_task_id",
                    "intent=complete must not include superseding_task_id",
                );
            }
        }
        CloseIntent::Cancel => {
            if request.close_reason != Some(CloseReason::Cancelled) {
                return invalid(
                    "close_reason",
                    "intent=cancel requires close_reason=cancelled",
                );
            }
            if request.superseding_task_id.is_some() {
                return invalid(
                    "superseding_task_id",
                    "intent=cancel must not include superseding_task_id",
                );
            }
        }
        CloseIntent::Supersede => {
            if request.close_reason != Some(CloseReason::Superseded) {
                return invalid(
                    "close_reason",
                    "intent=supersede requires close_reason=superseded",
                );
            }
            let Some(superseding_task_id) = &request.superseding_task_id else {
                return invalid(
                    "superseding_task_id",
                    "intent=supersede requires superseding_task_id",
                );
            };
            if superseding_task_id == &request.task_id {
                return invalid(
                    "superseding_task_id",
                    "superseding_task_id must identify a different Task",
                );
            }
        }
    }
    Ok(None)
}

fn reject_stale_close_write_authorization(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
) -> CoreResult<Option<PipelineResponse>> {
    let active_write_authorizations = store
        .active_write_authorizations(&request.task_id)
        .map_err(CorePipelineError::from)?;
    Ok(active_write_authorizations
        .iter()
        .find(|record| record.basis_state_version != project_state.state_version)
        .map(|record| {
            stale_write_authorization_basis_response(
                &request.envelope,
                record,
                project_state.state_version,
            )
        }))
}

fn close_task_dry_run_summary(intent: CloseIntent) -> DryRunSummary {
    let (action, description) = match intent {
        CloseIntent::Check => (
            "would_check",
            "Close readiness check would read the current Task state.",
        ),
        CloseIntent::Complete => (
            "would_complete",
            "Close task would attempt the complete terminal transition.",
        ),
        CloseIntent::Cancel => (
            "would_cancel",
            "Close task would attempt the cancel terminal transition.",
        ),
        CloseIntent::Supersede => (
            "would_supersede",
            "Close task would attempt the supersede terminal transition.",
        ),
    };
    dry_run_summary("task", action, description, Vec::new())
}

fn status_task(
    store: &CoreProjectStore,
    _project_state: &ProjectStateHeader,
    envelope_task_id: Option<&TaskId>,
) -> CoreResult<Option<TaskRecord>> {
    match envelope_task_id {
        Some(task_id) => store.task_record(task_id).map_err(CorePipelineError::from),
        None => store.active_task_record().map_err(CorePipelineError::from),
    }
}

fn plan_close_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: CloseTaskRequest,
) -> Result<CloseTaskPlan, PlanError> {
    let context = load_close_task_context(store, project_state, &request)?;
    let mut blockers = terminal_close_blockers(store, project_state, &request, &context)?;
    if matches!(request.intent, CloseIntent::Check | CloseIntent::Complete) {
        blockers.extend(completion_close_blockers(
            store,
            project_state,
            &request,
            &context,
        )?);
    }

    let committed_terminal = request.intent != CloseIntent::Check && blockers.is_empty();
    let response_state_version = if committed_terminal {
        project_state.state_version + 1
    } else {
        project_state.state_version
    };
    let close_state = match request.intent {
        CloseIntent::Check => {
            if blockers.is_empty() {
                CloseState::Ready
            } else {
                CloseState::Blocked
            }
        }
        CloseIntent::Complete => {
            if blockers.is_empty() {
                CloseState::Closed
            } else {
                CloseState::Blocked
            }
        }
        CloseIntent::Cancel => {
            if blockers.is_empty() {
                CloseState::Cancelled
            } else {
                CloseState::Blocked
            }
        }
        CloseIntent::Supersede => {
            if blockers.is_empty() {
                CloseState::Superseded
            } else {
                CloseState::Blocked
            }
        }
    };

    let mut synthetic_task = context.task.clone();
    let mut storage_mutations = Vec::new();
    let mut event_kind = String::new();
    let mut event_payload = Map::new();
    let closed_at = if committed_terminal {
        Some(store.current_timestamp().map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?)
    } else {
        None
    };

    if let Some(closed_at) = &closed_at {
        let terminal = close_terminal_storage(request.intent);
        let close_summary_json = terminal_close_summary_json(&context.task, &request, closed_at)?;
        synthetic_task.lifecycle_phase = terminal.lifecycle_phase.to_owned();
        synthetic_task.result = Some(terminal.result.to_owned());
        synthetic_task.close_summary_json = close_summary_json.clone();
        synthetic_task.closed_at = Some(closed_at.clone());
        storage_mutations.push(CoreStorageMutation::CloseTask(TaskCloseUpdate {
            task_id: request.task_id.as_str().to_owned(),
            lifecycle_phase: terminal.lifecycle_phase.to_owned(),
            result: terminal.result.to_owned(),
            close_summary_json,
            closed_at: closed_at.clone(),
        }));
        if request.intent == CloseIntent::Supersede {
            if let Some(superseding_task_id) = &request.superseding_task_id {
                storage_mutations.push(CoreStorageMutation::SetActiveTask {
                    task_id: superseding_task_id.as_str().to_owned(),
                });
            }
        }
        event_kind = terminal.event_kind.to_owned();
        event_payload = object_from_value(json!({
            "task_id": request.task_id,
            "intent": request.intent,
            "close_reason": request.close_reason,
            "superseding_task_id": request.superseding_task_id,
            "user_note": request.user_note,
            "closed_at": closed_at
        }))?;
    }

    let mut state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: response_state_version,
        task: &synthetic_task,
        current_change_unit: context.current_change_unit.as_ref(),
        pending_user_judgment_refs: context.pending_user_judgment_refs.clone(),
        blocker_refs: context.blocker_refs.clone(),
        active_write_authorization: None,
        options: SummaryOptions::mutation(),
    })?;
    state.evidence_summary = context.evidence_summary.clone();
    state.close_state = Some(close_state);
    state.close_blockers = blockers.clone();

    let result = CloseTaskResult {
        base: placeholder_base(),
        close_state,
        state,
        blockers: blockers.clone(),
        evidence_summary: context.evidence_summary.clone(),
        artifact_refs: context.artifact_refs.clone(),
    };

    Ok(CloseTaskPlan {
        task_id: request.task_id,
        change_unit_id: context
            .current_change_unit
            .as_ref()
            .map(|record| ChangeUnitId::new(record.change_unit_id.clone())),
        storage_mutations,
        event_kind,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        blockers,
    })
}

struct CloseTerminalStorage {
    lifecycle_phase: &'static str,
    result: &'static str,
    event_kind: &'static str,
}

fn close_terminal_storage(intent: CloseIntent) -> CloseTerminalStorage {
    match intent {
        CloseIntent::Complete => CloseTerminalStorage {
            lifecycle_phase: "completed",
            result: "completed",
            event_kind: "task_completed",
        },
        CloseIntent::Cancel => CloseTerminalStorage {
            lifecycle_phase: "cancelled",
            result: "cancelled",
            event_kind: "task_cancelled",
        },
        CloseIntent::Supersede => CloseTerminalStorage {
            lifecycle_phase: "superseded",
            result: "superseded",
            event_kind: "task_superseded",
        },
        CloseIntent::Check => CloseTerminalStorage {
            lifecycle_phase: "ready",
            result: "none",
            event_kind: "task_close_checked",
        },
    }
}

fn terminal_close_summary_json(
    task: &TaskRecord,
    request: &CloseTaskRequest,
    closed_at: &str,
) -> CoreResult<String> {
    let mut close_summary = parse_json_object(&task.close_summary_json);
    close_summary.insert(
        "close_reason".to_owned(),
        serde_json::to_value(
            request
                .close_reason
                .expect("validated terminal close_reason is present"),
        )?,
    );
    close_summary.insert("closed_at".to_owned(), Value::String(closed_at.to_owned()));
    close_summary.insert("intent".to_owned(), serde_json::to_value(request.intent)?);
    close_summary.insert(
        "user_note".to_owned(),
        request
            .user_note
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    close_summary.insert(
        "superseding_task_id".to_owned(),
        request
            .superseding_task_id
            .as_ref()
            .map(|id| Value::String(id.as_str().to_owned()))
            .unwrap_or(Value::Null),
    );
    serde_json::to_string(&Value::Object(close_summary)).map_err(CorePipelineError::from)
}

fn load_close_task_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
) -> Result<CloseTaskContext, PlanError> {
    let task = store
        .task_record(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let current_change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let pending_user_judgment_refs = store
        .pending_user_judgment_refs(&request.task_id, project_state.state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let blocker_refs = store
        .active_blocker_refs(&request.task_id, project_state.state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let evidence_record = store
        .latest_evidence_summary(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let evidence_summary = close_evidence_summary(
        evidence_record.as_ref(),
        &task,
        &request.envelope.project_id,
        &request.task_id,
        project_state.state_version,
    )?;
    let artifact_refs = evidence_summary
        .as_ref()
        .map(|summary| summary.artifact_refs.clone())
        .unwrap_or_default();

    Ok(CloseTaskContext {
        task,
        current_change_unit,
        pending_user_judgment_refs,
        blocker_refs,
        evidence_summary,
        artifact_refs,
    })
}

fn terminal_close_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);
    if is_terminal_lifecycle(&context.task.lifecycle_phase)
        || project_state
            .active_task_id
            .as_deref()
            .is_some_and(|active_task_id| active_task_id != request.task_id.as_str())
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Task,
            "task_not_closeable",
            "The addressed Task is not the current non-terminal Task.",
            vec![task_ref.clone()],
            vec![close_next_action(
                "Review the current Task before closing.",
                vec![task_ref.clone()],
            )],
        ));
    }

    if request.intent == CloseIntent::Supersede {
        let superseding_ref = request.superseding_task_id.as_ref().map(|task_id| {
            state_ref(
                StateRecordKind::Task,
                task_id.as_str(),
                &request.envelope.project_id,
                Some(task_id),
                Some(project_state.state_version),
            )
        });
        let replacement = request
            .superseding_task_id
            .as_ref()
            .map(|task_id| {
                store.task_record(task_id).map_err(|error| {
                    PlanError::Response(Box::new(store_error_response(
                        &request.envelope,
                        project_state,
                        error,
                    )))
                })
            })
            .transpose()?
            .flatten();
        if replacement
            .as_ref()
            .map(|task| is_terminal_lifecycle(&task.lifecycle_phase))
            .unwrap_or(true)
        {
            blockers.push(close_blocker(
                CloseReadinessBlockerCategory::Task,
                "task_not_closeable",
                "superseding_task_id must identify a non-terminal Task in this project.",
                superseding_ref.into_iter().collect(),
                Vec::new(),
            ));
        }
    }

    if recovery_required(context) {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Recovery,
            "recovery_required",
            "A recovery constraint or active blocker must be resolved before this terminal transition.",
            context.blocker_refs.clone(),
            vec![close_next_action(
                "Resolve recovery blockers before closing the Task.",
                context.blocker_refs.clone(),
            )],
        ));
    }

    Ok(blockers)
}

fn completion_close_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    context: &CloseTaskContext,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);
    let change_unit_ref = context.current_change_unit.as_ref().map(|record| {
        change_unit_ref(
            &request.envelope.project_id,
            &request.task_id,
            record,
            project_state.state_version,
        )
    });

    if context
        .current_change_unit
        .as_ref()
        .map(|record| record.status != "active" || !record.is_current)
        .unwrap_or(true)
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Scope,
            "missing_active_change_unit",
            "Completion requires a current active Change Unit.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::UpdateScope,
                owner_method: Some(MethodName::UpdateScope),
                label: "Create or restore the current active Change Unit.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    if !context.pending_user_judgment_refs.is_empty() {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::UserJudgment,
            "pending_user_judgment",
            "A user-owned judgment required before close is still pending.",
            context.pending_user_judgment_refs.clone(),
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordUserJudgment,
                owner_method: Some(MethodName::RecordUserJudgment),
                label: "Resolve pending user-owned judgments required for close.".to_owned(),
                blocking_question: None,
                required_refs: context.pending_user_judgment_refs.clone(),
            }],
        ));
    }

    if sensitive_approval_required(context)
        && !has_resolved_judgment(store, project_state, request, "sensitive_approval")?
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::SensitiveApproval,
            "missing_sensitive_approval",
            "A documented sensitive-action approval required for close is missing.",
            change_unit_ref.clone().into_iter().collect(),
            vec![NextActionSummary {
                action_kind: NextActionKind::RequestUserJudgment,
                owner_method: Some(MethodName::RequestUserJudgment),
                label: "Request the user-owned sensitive-action approval.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    for record in store
        .active_write_authorizations(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .iter()
        .filter(|record| record.basis_state_version != project_state.state_version)
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::WriteCompatibility,
            "write_authorization_stale",
            "An active Write Authorization is stale against the current state version.",
            vec![write_authorization_ref(record, project_state.state_version)],
            vec![NextActionSummary {
                action_kind: NextActionKind::PrepareWrite,
                owner_method: Some(MethodName::PrepareWrite),
                label: "Refresh write compatibility before completing the Task.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    if baseline_stale_for_close(context) {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Baseline,
            "baseline_stale",
            "The close basis marks the baseline as stale.",
            change_unit_ref.clone().into_iter().collect(),
            vec![NextActionSummary {
                action_kind: NextActionKind::UpdateScope,
                owner_method: Some(MethodName::UpdateScope),
                label: "Refresh the current scope or close basis before completing the Task."
                    .to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    let unsupported_items = unsupported_close_evidence_items(context.evidence_summary.as_ref());
    if !unsupported_items.is_empty() {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Evidence,
            "evidence_claim_unsupported",
            "One or more required close evidence claims are unsupported.",
            unsupported_items
                .iter()
                .flat_map(|item| item.gap_refs.clone())
                .collect(),
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                label: "Record evidence that supports the required close claims.".to_owned(),
                blocking_question: None,
                required_refs: change_unit_ref.clone().into_iter().collect(),
            }],
        ));
    }

    let unavailable_artifacts = unavailable_close_artifact_refs(
        store,
        project_state,
        request,
        context.evidence_summary.as_ref(),
    )?;
    if !unavailable_artifacts.is_empty() {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ArtifactAvailability,
            "artifact_unavailable",
            "A required close artifact is missing, unavailable, or incompatible with storage.",
            unavailable_artifacts,
            vec![NextActionSummary {
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                label: "Record or repair the artifact supporting close evidence.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    if !has_resolved_judgment_with_answer(
        store,
        project_state,
        request,
        "final_acceptance",
        |resolution| resolution.answer.final_acceptance.is_some(),
    )? {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::FinalAcceptance,
            "missing_final_acceptance",
            "Final acceptance is required before completing the Task.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::RequestUserJudgment,
                owner_method: Some(MethodName::RequestUserJudgment),
                label: "Request final acceptance from the user.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    let residual_risk = residual_risk_state(context);
    if residual_risk.known && !residual_risk.visible {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ResidualRiskVisibility,
            "residual_risk_not_visible",
            "Residual risk exists but is not visible in the close basis.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::RequestUserJudgment,
                owner_method: Some(MethodName::RequestUserJudgment),
                label: "Make residual risk visible before requesting acceptance.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }
    if residual_risk.known
        && residual_risk.visible
        && !has_residual_risk_acceptance(store, project_state, request)?
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ResidualRiskAcceptance,
            "missing_residual_risk_acceptance",
            "Visible residual risk requires distinct residual-risk acceptance.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                action_kind: NextActionKind::RequestUserJudgment,
                owner_method: Some(MethodName::RequestUserJudgment),
                label: "Request residual-risk acceptance from the user.".to_owned(),
                blocking_question: None,
                required_refs: vec![task_ref],
            }],
        ));
    }

    Ok(blockers)
}

fn close_evidence_summary(
    record: Option<&EvidenceSummaryRecord>,
    task: &TaskRecord,
    project_id: &ProjectId,
    task_id: &TaskId,
    state_version: u64,
) -> CoreResult<Option<EvidenceSummary>> {
    let policy = task_completion_policy(task);
    let mut required_claims = sorted_unique(policy.required_claims);
    if policy.evidence_required && required_claims.is_empty() {
        required_claims.push("completion_evidence".to_owned());
    }
    let required_set = required_claims.iter().cloned().collect::<BTreeSet<_>>();
    let mut coverage_items = record
        .map(|record| {
            parse_json_text::<Vec<EvidenceCoverageItem>>(
                "evidence_summaries.coverage_json",
                &record.coverage_json,
            )
        })
        .transpose()?
        .unwrap_or_default();
    for item in &mut coverage_items {
        if required_set.contains(&item.claim) {
            item.required_for_close = true;
        }
    }
    for claim in &required_set {
        if !coverage_items.iter().any(|item| item.claim == *claim) {
            coverage_items.push(EvidenceCoverageItem {
                claim: claim.clone(),
                required_for_close: true,
                coverage_state: EvidenceCoverageState::Unsupported,
                supporting_refs: Vec::new(),
                supporting_artifact_refs: Vec::new(),
                gap_refs: Vec::new(),
            });
        }
    }
    if coverage_items.is_empty() && !policy.evidence_required {
        return Ok(None);
    }
    let artifact_refs = unique_artifact_refs(
        coverage_items
            .iter()
            .flat_map(|item| item.supporting_artifact_refs.clone())
            .collect(),
    );
    let status = if coverage_items.is_empty() {
        record
            .map(|record| parse_storage_value("evidence_summaries.status", &record.status))
            .transpose()?
            .unwrap_or(EvidenceStatus::Unknown)
    } else {
        evidence_status_for_items(&coverage_items)
    };
    let updated_by_run_ref = record.and_then(|record| {
        string_member(
            &parse_json_object(&record.metadata_json),
            "updated_by_run_id",
        )
        .map(|run_id| {
            state_ref(
                StateRecordKind::Run,
                &run_id,
                project_id,
                Some(task_id),
                Some(state_version),
            )
        })
    });

    Ok(Some(EvidenceSummary {
        status,
        completion_policy: CompletionPolicy {
            evidence_required: policy.evidence_required || !required_claims.is_empty(),
            required_claims,
        },
        coverage_items,
        artifact_refs,
        updated_by_run_ref,
    }))
}

fn task_completion_policy(task: &TaskRecord) -> CompletionPolicy {
    let object = parse_json_object(&task.completion_policy_json);
    let required_claims = string_array_member(&object, "required_claims");
    CompletionPolicy {
        evidence_required: object
            .get("evidence_required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || !required_claims.is_empty(),
        required_claims,
    }
}

fn unsupported_close_evidence_items(
    evidence_summary: Option<&EvidenceSummary>,
) -> Vec<&EvidenceCoverageItem> {
    evidence_summary
        .map(|summary| {
            summary
                .coverage_items
                .iter()
                .filter(|item| {
                    item.required_for_close
                        && !matches!(
                            item.coverage_state,
                            EvidenceCoverageState::Supported | EvidenceCoverageState::NotApplicable
                        )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn unavailable_close_artifact_refs(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    evidence_summary: Option<&EvidenceSummary>,
) -> Result<Vec<StateRecordRef>, PlanError> {
    let mut seen = BTreeSet::new();
    let mut unavailable = Vec::new();
    let Some(evidence_summary) = evidence_summary else {
        return Ok(unavailable);
    };
    for artifact_ref in evidence_summary
        .coverage_items
        .iter()
        .filter(|item| item.required_for_close)
        .flat_map(|item| item.supporting_artifact_refs.iter())
    {
        if !seen.insert(artifact_ref.artifact_id.as_str().to_owned()) {
            continue;
        }
        let state_ref = state_ref(
            StateRecordKind::Artifact,
            artifact_ref.artifact_id.as_str(),
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(project_state.state_version),
        );
        if artifact_ref.availability != ArtifactAvailability::Available {
            unavailable.push(state_ref);
            continue;
        }
        let stored = store
            .artifact_record(artifact_ref.artifact_id.as_str())
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?;
        let Some(stored) = stored else {
            unavailable.push(state_ref);
            continue;
        };
        let owner_link_exists = store
            .artifact_has_task_owner_link(
                artifact_ref.artifact_id.as_str(),
                request.task_id.as_str(),
            )
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?;
        if stored.project_id != request.envelope.project_id.as_str()
            || stored.task_id != request.task_id.as_str()
            || stored.status != "available"
            || stored.sha256.as_deref() != Some(artifact_ref.sha256.as_str())
            || stored.size_bytes != Some(artifact_ref.size_bytes)
            || stored.redaction_state != redaction_state_value(artifact_ref.redaction_state)
            || !owner_link_exists
        {
            unavailable.push(state_ref);
        }
    }
    Ok(unavailable)
}

fn has_resolved_judgment(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    judgment_kind: &str,
) -> Result<bool, PlanError> {
    has_resolved_judgment_with_answer(store, project_state, request, judgment_kind, |_| true)
}

fn has_resolved_judgment_with_answer<F>(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
    judgment_kind: &str,
    predicate: F,
) -> Result<bool, PlanError>
where
    F: Fn(&UserJudgmentResolution) -> bool,
{
    let records = store
        .resolved_user_judgment_records(&request.task_id, judgment_kind)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    for record in records {
        let Some(resolution_json) = &record.resolution_json else {
            continue;
        };
        let resolution: UserJudgmentResolution =
            parse_json_text("user_judgments.resolution_json", resolution_json)?;
        if predicate(&resolution) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn has_residual_risk_acceptance(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskRequest,
) -> Result<bool, PlanError> {
    has_resolved_judgment_with_answer(
        store,
        project_state,
        request,
        "residual_risk_acceptance",
        |resolution| {
            resolution.answer.residual_risk_acceptance.is_some()
                || resolution
                    .accepted_risks
                    .iter()
                    .any(|risk| risk.accepted_for_close)
        },
    )
}

fn sensitive_approval_required(context: &CloseTaskContext) -> bool {
    let close_summary = parse_json_object(&context.task.close_summary_json);
    if !string_array_member(&close_summary, "required_sensitive_categories").is_empty()
        || !string_array_member(&close_summary, "sensitive_categories").is_empty()
    {
        return true;
    }
    context.current_change_unit.as_ref().is_some_and(|record| {
        let close_basis = parse_json_object(&record.close_basis_json);
        !string_array_member(&close_basis, "required_sensitive_categories").is_empty()
            || !string_array_member(&close_basis, "sensitive_categories").is_empty()
    })
}

fn baseline_stale_for_close(context: &CloseTaskContext) -> bool {
    let close_summary = parse_json_object(&context.task.close_summary_json);
    if bool_member(&close_summary, "baseline_stale")
        || string_member(&close_summary, "baseline_status").as_deref() == Some("stale")
    {
        return true;
    }
    context.current_change_unit.as_ref().is_some_and(|record| {
        let close_basis = parse_json_object(&record.close_basis_json);
        bool_member(&close_basis, "baseline_stale")
            || string_member(&close_basis, "baseline_status").as_deref() == Some("stale")
    })
}

fn recovery_required(context: &CloseTaskContext) -> bool {
    if !context.blocker_refs.is_empty() {
        return true;
    }
    let close_summary = parse_json_object(&context.task.close_summary_json);
    if bool_member(&close_summary, "recovery_required") {
        return true;
    }
    context.current_change_unit.as_ref().is_some_and(|record| {
        bool_member(
            &parse_json_object(&record.lifecycle_json),
            "recovery_required",
        ) || bool_member(
            &parse_json_object(&record.close_basis_json),
            "recovery_required",
        )
    })
}

#[derive(Debug, Clone, Copy)]
struct ResidualRiskState {
    known: bool,
    visible: bool,
}

fn residual_risk_state(context: &CloseTaskContext) -> ResidualRiskState {
    let close_summary = parse_json_object(&context.task.close_summary_json);
    let visible = json_array_nonempty_member(&close_summary, "visible_risks")
        || bool_member(&close_summary, "residual_risk_visible");
    let known = visible
        || json_array_nonempty_member(&close_summary, "residual_risks")
        || bool_member(&close_summary, "residual_risk_present");
    ResidualRiskState { known, visible }
}

fn is_terminal_lifecycle(value: &str) -> bool {
    matches!(value, "completed" | "cancelled" | "superseded")
}

fn task_ref_for_close(request: &CloseTaskRequest, state_version: u64) -> StateRecordRef {
    state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(state_version),
    )
}

fn close_blocker(
    category: CloseReadinessBlockerCategory,
    code: &'static str,
    message: &'static str,
    related_refs: Vec<StateRecordRef>,
    next_actions: Vec<NextActionSummary>,
) -> CloseReadinessBlocker {
    CloseReadinessBlocker {
        category,
        code: code.to_owned(),
        message: message.to_owned(),
        related_refs,
        next_actions,
    }
}

fn close_next_action(label: &str, required_refs: Vec<StateRecordRef>) -> NextActionSummary {
    NextActionSummary {
        action_kind: NextActionKind::CloseTask,
        owner_method: Some(MethodName::CloseTask),
        label: label.to_owned(),
        blocking_question: None,
        required_refs,
    }
}

fn plan_intake(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: harness_types::IntakeRequest,
    verified_surface: &VerifiedSurfaceContext,
) -> CoreResult<MethodPlan> {
    let planned_state_version = project_state.state_version + 1;
    let mode = resolve_requested_mode(request.requested_mode);
    let active_task = store.active_task_record()?;

    let create_new = match request.resume_policy {
        ResumePolicy::ResumeActive => active_task.is_none(),
        ResumePolicy::CreateNew | ResumePolicy::RejectIfActive => true,
        ResumePolicy::SupersedeActive => true,
    };
    let task_id = if create_new {
        match request.envelope.task_id.clone() {
            Some(task_id) => task_id,
            None => allocate_task_id(service, store)?,
        }
    } else {
        TaskId::new(
            active_task
                .as_ref()
                .expect("active_task exists when create_new is false")
                .task_id
                .clone(),
        )
    };

    let mut storage_mutations = Vec::new();
    if request.resume_policy == ResumePolicy::SupersedeActive {
        if let Some(active) = &active_task {
            storage_mutations.push(CoreStorageMutation::SupersedeTask {
                task_id: active.task_id.clone(),
            });
        }
    }

    let task_record = if create_new {
        let shaping_summary = task_shaping_json(
            Some(request.plain_language_request.clone()),
            Some(request.initial_scope.boundary.clone()),
            request.initial_scope.non_goals.clone(),
            request.initial_scope.acceptance_criteria.clone(),
            None,
            None,
            Some(serde_json::to_value(&request.initial_context_refs)?),
        );
        let task = TaskRecord {
            project_id: request.envelope.project_id.as_str().to_owned(),
            task_id: task_id.as_str().to_owned(),
            mode: task_mode_storage(mode).to_owned(),
            lifecycle_phase: "shaping".to_owned(),
            result: Some("none".to_owned()),
            title: Some(request.plain_language_request.clone()),
            summary: Some(request.plain_language_request.clone()),
            shaping_summary_json: serde_json::to_string(&shaping_summary)?,
            bounded_context_json: serde_json::to_string(&json!({
                "initial_context_refs": request.initial_context_refs
            }))?,
            autonomy_boundary_json: serde_json::to_string(&json!({
                "autonomy_boundary": Value::Null
            }))?,
            close_summary_json: serde_json::to_string(&json!({
                "close_reason": "none"
            }))?,
            completion_policy_json: "{}".to_owned(),
            current_change_unit_id: None,
            closed_at: None,
        };
        storage_mutations.push(CoreStorageMutation::InsertTask(TaskInsert {
            task_id: task.task_id.clone(),
            created_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
            created_by_surface_instance_id: verified_surface
                .surface_instance_id
                .as_str()
                .to_owned(),
            mode: task.mode.clone(),
            lifecycle_phase: task.lifecycle_phase.clone(),
            result: task.result.clone(),
            title: task.title.clone(),
            summary: task.summary.clone(),
            shaping_summary_json: task.shaping_summary_json.clone(),
            bounded_context_json: task.bounded_context_json.clone(),
            autonomy_boundary_json: task.autonomy_boundary_json.clone(),
            close_summary_json: task.close_summary_json.clone(),
            completion_policy_json: task.completion_policy_json.clone(),
            current_change_unit_id: None,
        }));
        storage_mutations.push(CoreStorageMutation::SetActiveTask {
            task_id: task.task_id.clone(),
        });
        task
    } else {
        active_task.expect("active_task exists when create_new is false")
    };

    let task_ref = state_ref(
        StateRecordKind::Task,
        &task_record.task_id,
        &request.envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let pending_refs = Vec::new();
    let blocker_refs = Vec::new();
    let next_actions = next_actions_for_state(&task_ref, None);
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task_record,
        current_change_unit: None,
        pending_user_judgment_refs: pending_refs,
        blocker_refs,
        active_write_authorization: None,
        options: SummaryOptions::mutation(),
    })?;
    let result = harness_types::IntakeResult {
        base: placeholder_base(),
        task_ref: task_ref.clone(),
        change_unit_ref: None,
        state,
        next_actions: next_actions.clone(),
    };
    let event_payload = object_from_value(json!({
        "task_id": task_id,
        "resume_policy": request.resume_policy,
        "requested_mode": request.requested_mode,
        "resolved_mode": mode
    }))?;
    Ok(MethodPlan {
        task_id,
        change_unit_id: None,
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions,
    })
}

fn plan_update_scope(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: UpdateScopeRequest,
) -> Result<MethodPlan, PlanError> {
    let planned_state_version = project_state.state_version + 1;
    let task = store
        .task_record(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let current_change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;

    let current_scope = StoredScope::from_task(&task);
    let next_scope = current_scope.apply_request(&request);
    let scope_changed = current_scope != next_scope
        || request.change_unit.operation == ChangeUnitOperation::CreateCurrent
        || request.change_unit.operation == ChangeUnitOperation::ReplaceCurrent;

    let active_write_authorizations = store
        .active_write_authorizations(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let stale_write_authorization_refs = if scope_changed {
        active_write_authorizations
            .iter()
            .map(|record| write_authorization_ref(record, planned_state_version))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let mut storage_mutations = vec![CoreStorageMutation::UpdateTaskScope(TaskScopeUpdate {
        task_id: task.task_id.clone(),
        lifecycle_phase: None,
        result: None,
        title: next_scope.goal_summary.clone(),
        summary: next_scope.goal_summary.clone(),
        shaping_summary_json: Some(serde_json::to_string(&next_scope.to_json())?),
        bounded_context_json: Some(serde_json::to_string(&json!({
            "scope_update": request.scope_update.clone()
        }))?),
        autonomy_boundary_json: Some(serde_json::to_string(&json!({
            "autonomy_boundary": next_scope.autonomy_boundary
        }))?),
        close_summary_json: None,
        completion_policy_json: None,
    })];

    let mut synthetic_task = task.clone();
    synthetic_task.title = next_scope.goal_summary.clone();
    synthetic_task.summary = next_scope.goal_summary.clone();
    synthetic_task.shaping_summary_json = serde_json::to_string(&next_scope.to_json())?;
    synthetic_task.bounded_context_json = serde_json::to_string(&json!({
        "scope_update": request.scope_update.clone()
    }))?;
    synthetic_task.autonomy_boundary_json = serde_json::to_string(&json!({
        "autonomy_boundary": next_scope.autonomy_boundary
    }))?;

    let (change_unit_ref, synthetic_change_unit, branch_change_unit_id) =
        match request.change_unit.operation {
            ChangeUnitOperation::KeepCurrent => {
                let change_unit_ref = current_change_unit.as_ref().map(|record| {
                    state_ref(
                        StateRecordKind::ChangeUnit,
                        &record.change_unit_id,
                        &request.envelope.project_id,
                        Some(&request.task_id),
                        Some(record.basis_state_version.unwrap_or(planned_state_version)),
                    )
                });
                (
                    change_unit_ref,
                    current_change_unit.clone(),
                    current_change_unit
                        .as_ref()
                        .map(|record| ChangeUnitId::new(record.change_unit_id.clone())),
                )
            }
            ChangeUnitOperation::CreateCurrent => {
                if current_change_unit.is_some() {
                    let response = validation_rejected(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "change_unit.operation",
                        "create_current requires no current Change Unit",
                    )
                    .map_err(PlanError::Core)?;
                    return Err(PlanError::Response(Box::new(response)));
                }
                let change_unit_id =
                    allocate_change_unit_id(service, store).map_err(PlanError::Core)?;
                let insert = change_unit_insert(&request, &change_unit_id)?;
                let record = synthetic_change_unit_record(
                    &request.envelope.project_id,
                    &request.task_id,
                    &insert,
                    planned_state_version,
                );
                storage_mutations.push(CoreStorageMutation::InsertCurrentChangeUnit(insert));
                synthetic_task.current_change_unit_id = Some(change_unit_id.as_str().to_owned());
                synthetic_task.lifecycle_phase = "ready".to_owned();
                let change_unit_ref = state_ref(
                    StateRecordKind::ChangeUnit,
                    change_unit_id.as_str(),
                    &request.envelope.project_id,
                    Some(&request.task_id),
                    Some(planned_state_version),
                );
                (Some(change_unit_ref), Some(record), Some(change_unit_id))
            }
            ChangeUnitOperation::ReplaceCurrent => {
                if current_change_unit.is_none() {
                    let response = rejected_pipeline_response(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        vec![tool_error(
                            ErrorCode::NoActiveChangeUnit,
                            "replace_current requires a current Change Unit",
                            false,
                            None,
                        )],
                    )
                    .map_err(PlanError::Core)?;
                    return Err(PlanError::Response(Box::new(response)));
                }
                let change_unit_id =
                    allocate_change_unit_id(service, store).map_err(PlanError::Core)?;
                let insert = change_unit_insert(&request, &change_unit_id)?;
                let record = synthetic_change_unit_record(
                    &request.envelope.project_id,
                    &request.task_id,
                    &insert,
                    planned_state_version,
                );
                storage_mutations.push(CoreStorageMutation::ReplaceCurrentChangeUnit(insert));
                synthetic_task.current_change_unit_id = Some(change_unit_id.as_str().to_owned());
                synthetic_task.lifecycle_phase = "ready".to_owned();
                let change_unit_ref = state_ref(
                    StateRecordKind::ChangeUnit,
                    change_unit_id.as_str(),
                    &request.envelope.project_id,
                    Some(&request.task_id),
                    Some(planned_state_version),
                );
                (Some(change_unit_ref), Some(record), Some(change_unit_id))
            }
        };

    if scope_changed && !active_write_authorizations.is_empty() {
        storage_mutations.push(CoreStorageMutation::MarkActiveWriteAuthorizationsStale {
            task_id: request.task_id.as_str().to_owned(),
        });
    }

    let pending_refs = store
        .pending_user_judgment_refs(&request.task_id, planned_state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let blocker_refs = store
        .active_blocker_refs(&request.task_id, planned_state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let task_ref = state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let next_actions = next_actions_for_state(&task_ref, change_unit_ref.as_ref());
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &synthetic_task,
        current_change_unit: synthetic_change_unit.as_ref(),
        pending_user_judgment_refs: pending_refs,
        blocker_refs: blocker_refs.clone(),
        active_write_authorization: None,
        options: SummaryOptions::mutation(),
    })?;
    let result = harness_types::UpdateScopeResult {
        base: placeholder_base(),
        task_ref,
        change_unit_ref,
        linked_scope_decision_refs: request.related_scope_decision_refs.clone(),
        stale_write_authorization_refs,
        blocker_refs,
        state,
        next_actions: next_actions.clone(),
    };
    let event_payload = object_from_value(json!({
        "task_id": request.task_id.clone(),
        "change_unit_operation": request.change_unit.operation,
        "scope_changed": scope_changed
    }))?;

    Ok(MethodPlan {
        task_id: request.task_id,
        change_unit_id: branch_change_unit_id,
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions,
    })
}

fn plan_prepare_write(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: PrepareWriteRequest,
    verified_surface: &VerifiedSurfaceContext,
) -> Result<PrepareWritePlan, PlanError> {
    if request.intended_operation.trim().is_empty() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "intended_operation",
            "intended_operation must not be empty",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }

    let normalized_paths = match normalize_product_paths(
        &store.project_record().repo_root,
        &request.intended_paths,
    ) {
        Ok(paths) => paths,
        Err(ProductPathError::Invalid) => {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "intended_paths",
                "intended_paths must be relative Product Repository paths that stay inside the repository",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        Err(ProductPathError::LocalAccess) => {
            let response = rejected_pipeline_response(
                request.envelope.dry_run,
                Some(project_state.state_version),
                vec![tool_error(
                    ErrorCode::LocalAccessMismatch,
                    "intended_paths resolve outside the Product Repository",
                    false,
                    None,
                )],
            )
            .map_err(PlanError::Core)?;
            return Err(PlanError::Response(Box::new(response)));
        }
    };

    let planned_state_version = project_state.state_version + 1;
    let (task_id, task, mut reasons) = resolve_prepare_write_task(store, project_state, &request)?;
    let current_change_unit = store.current_change_unit(&task_id).map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })?;
    let change_unit = resolve_prepare_write_change_unit(
        &request,
        &task_id,
        current_change_unit.as_ref(),
        &mut reasons,
    );

    if request.product_file_write_intended == normalized_paths.is_empty() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::WriteCompatibility,
            "product_write_flag_mismatch",
            "product_file_write_intended must match the intended Product Repository paths.",
            Vec::new(),
        ));
    }

    if let Some(change_unit) = change_unit {
        if !baseline_matches(change_unit, &task, &request.baseline_ref) {
            reasons.push(write_decision_reason(
                WriteDecisionCategory::Baseline,
                "baseline_mismatch",
                "baseline_ref does not match the current write-compatibility basis.",
                vec![change_unit_ref(
                    &request.envelope.project_id,
                    &task_id,
                    change_unit,
                    project_state.state_version,
                )],
            ));
        }

        if !paths_match_current_change_unit(
            &store.project_record().repo_root,
            &normalized_paths,
            change_unit,
        ) {
            reasons.push(write_decision_reason(
                WriteDecisionCategory::Scope,
                "path_out_of_scope",
                "One or more intended paths are outside the current Change Unit path scope.",
                vec![change_unit_ref(
                    &request.envelope.project_id,
                    &task_id,
                    change_unit,
                    project_state.state_version,
                )],
            ));
        }
    }

    let pending_user_judgment_refs = store
        .pending_user_judgment_refs(&task_id, project_state.state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    if !pending_user_judgment_refs.is_empty() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::UserJudgment,
            "user_judgment_unresolved",
            "A user-owned judgment required before write preparation remains unresolved.",
            pending_user_judgment_refs.clone(),
        ));
    }

    let mut active_user_judgment_refs = Vec::new();
    if !request.sensitive_categories.is_empty() {
        let matching_sensitive_approval = matching_sensitive_approval(
            store,
            project_state,
            &request,
            &task_id,
            change_unit,
            &normalized_paths,
        )?;
        if let Some(record) = matching_sensitive_approval {
            active_user_judgment_refs.push(state_ref(
                StateRecordKind::UserJudgment,
                &record.judgment_id,
                &request.envelope.project_id,
                Some(&task_id),
                Some(project_state.state_version),
            ));
        } else {
            reasons.push(write_decision_reason(
                WriteDecisionCategory::SensitiveApproval,
                "sensitive_approval_missing",
                "A matching sensitive-action approval is required before Write Authorization.",
                Vec::new(),
            ));
        }
    }

    if verified_surface.access_class != AccessClass::WriteAuthorization {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::SurfaceCapability,
            "surface_access_class_mismatch",
            "The verified surface access class is incompatible with Write Authorization.",
            Vec::new(),
        ));
    }
    if !surface_supports_prepare_write(&verified_surface.capability_profile) {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::SurfaceCapability,
            "surface_capability_insufficient",
            "The verified surface lacks the write-authorization capability declaration.",
            Vec::new(),
        ));
    }

    let guarantee_display = Some(write_authorization_guarantee());
    let branch_change_unit_id =
        change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let scope_change_unit_id = branch_change_unit_id.clone().unwrap_or_else(|| {
        request
            .change_unit_id
            .clone()
            .unwrap_or_else(|| ChangeUnitId::new("missing_current_change_unit"))
    });
    let decision = prepare_write_decision(&reasons);
    let allowed = reasons.is_empty();
    let write_authorization_id =
        allocate_write_authorization_id(service, store).map_err(PlanError::Core)?;
    let authorized_attempt_scope = AuthorizedAttemptScope {
        task_id: task_id.clone(),
        change_unit_id: scope_change_unit_id.clone(),
        intended_operation: request.intended_operation.clone(),
        intended_paths: normalized_paths.clone(),
        product_file_write_intended: request.product_file_write_intended,
        sensitive_categories: request.sensitive_categories.clone(),
        baseline_ref: Some(request.baseline_ref.clone()),
    };
    let attempt_scope_json = serde_json::to_string(&authorized_attempt_scope)?;
    let expires_at = "2999-01-01T00:00:00Z".to_owned();
    let write_authorization_ref = allowed.then(|| {
        state_ref(
            StateRecordKind::WriteAuthorization,
            write_authorization_id.as_str(),
            &request.envelope.project_id,
            Some(&task_id),
            Some(planned_state_version),
        )
    });
    let write_authorization = write_authorization_ref
        .as_ref()
        .map(|write_authorization_ref| WriteAuthorizationSummary {
            write_authorization_ref: write_authorization_ref.clone(),
            status: WriteAuthorizationStatus::Active,
            authorized_attempt_scope: authorized_attempt_scope.clone(),
            basis_state_version: planned_state_version,
            expires_at: Some(expires_at.clone()),
        });
    let synthetic_write_authorization = allowed.then(|| WriteAuthorizationRecord {
        project_id: request.envelope.project_id.as_str().to_owned(),
        write_authorization_id: write_authorization_id.as_str().to_owned(),
        task_id: task_id.as_str().to_owned(),
        change_unit_id: Some(scope_change_unit_id.as_str().to_owned()),
        basis_state_version: planned_state_version,
        status: "active".to_owned(),
        attempt_scope_json: attempt_scope_json.clone(),
        expires_at: expires_at.clone(),
    });

    let blocker_refs = store
        .active_blocker_refs(&task_id, planned_state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: change_unit,
        pending_user_judgment_refs,
        blocker_refs,
        active_write_authorization: synthetic_write_authorization.as_ref(),
        options: SummaryOptions::prepare_write(),
    })?;
    let result = PrepareWriteResult {
        base: placeholder_base(),
        decision,
        state: Some(state),
        write_authorization_ref: write_authorization_ref.clone(),
        write_authorization,
        authorization_effect: if allowed {
            AuthorizationEffect::Created
        } else {
            AuthorizationEffect::None
        },
        active_user_judgment_refs,
        write_decision_reasons: reasons.clone(),
        user_judgment_candidate: None,
        guarantee_display: guarantee_display.clone(),
    };

    let storage_mutations = if allowed {
        vec![CoreStorageMutation::InsertWriteAuthorization(
            WriteAuthorizationInsert {
                write_authorization_id: write_authorization_id.as_str().to_owned(),
                task_id: task_id.as_str().to_owned(),
                change_unit_id: scope_change_unit_id.as_str().to_owned(),
                attempt_scope_json,
                created_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
                created_by_surface_instance_id: verified_surface
                    .surface_instance_id
                    .as_str()
                    .to_owned(),
                created_by_judgment_id: None,
                expires_at,
                metadata_json: serde_json::to_string(&json!({
                    "verification_basis": verified_surface.verification_basis.clone()
                }))?,
            },
        )]
    } else {
        Vec::new()
    };
    let event_kind = if allowed {
        "write_authorization_created"
    } else {
        "write_decision_recorded"
    }
    .to_owned();
    let event_payload = object_from_value(json!({
        "task_id": task_id.clone(),
        "change_unit_id": branch_change_unit_id.clone(),
        "decision": decision,
        "write_authorization_id": allowed.then(|| write_authorization_id.as_str().to_owned()),
        "reason_codes": reasons.iter().map(|reason| reason.code.clone()).collect::<Vec<_>>()
    }))?;

    Ok(PrepareWritePlan {
        task_id,
        change_unit_id: branch_change_unit_id,
        storage_mutations,
        event_kind,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        dry_run_summary: prepare_write_dry_run_summary(
            allowed,
            &reasons,
            write_authorization_ref,
            guarantee_display,
        ),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProductPathError {
    Invalid,
    LocalAccess,
}

fn resolve_prepare_write_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &PrepareWriteRequest,
) -> Result<(TaskId, TaskRecord, Vec<WriteDecisionReason>), PlanError> {
    let task_id = request
        .task_id
        .clone()
        .or_else(|| request.envelope.task_id.clone())
        .or_else(|| project_state.active_task_id.clone().map(TaskId::new))
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let task = store
        .task_record(&task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;

    let mut reasons = Vec::new();
    if project_state
        .active_task_id
        .as_deref()
        .is_some_and(|active_task_id| active_task_id != task_id.as_str())
    {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            "scope_not_current",
            "The addressed Task is not the current Task.",
            vec![state_ref(
                StateRecordKind::Task,
                task_id.as_str(),
                &request.envelope.project_id,
                Some(&task_id),
                Some(project_state.state_version),
            )],
        ));
    }

    Ok((task_id, task, reasons))
}

fn resolve_prepare_write_change_unit<'a>(
    request: &PrepareWriteRequest,
    task_id: &TaskId,
    current_change_unit: Option<&'a ChangeUnitRecord>,
    reasons: &mut Vec<WriteDecisionReason>,
) -> Option<&'a ChangeUnitRecord> {
    let Some(current_change_unit) = current_change_unit else {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            "no_current_change_unit",
            "No current Change Unit can be resolved for write preparation.",
            Vec::new(),
        ));
        return None;
    };

    if request
        .change_unit_id
        .as_ref()
        .is_some_and(|change_unit_id| change_unit_id.as_str() != current_change_unit.change_unit_id)
    {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            "scope_not_current",
            "The addressed Change Unit is not the current Change Unit.",
            vec![change_unit_ref(
                &request.envelope.project_id,
                task_id,
                current_change_unit,
                current_change_unit.basis_state_version.unwrap_or_default(),
            )],
        ));
    }

    Some(current_change_unit)
}

fn normalize_product_paths(
    repo_root: &Path,
    raw_paths: &[String],
) -> Result<Vec<String>, ProductPathError> {
    let canonical_repo_root =
        fs::canonicalize(repo_root).map_err(|_| ProductPathError::LocalAccess)?;
    raw_paths
        .iter()
        .map(|path| normalize_product_path(repo_root, &canonical_repo_root, path))
        .collect()
}

fn normalize_product_path(
    repo_root: &Path,
    canonical_repo_root: &Path,
    raw_path: &str,
) -> Result<String, ProductPathError> {
    if raw_path.trim().is_empty() || raw_path.contains('\\') {
        return Err(ProductPathError::Invalid);
    }
    let path = Path::new(raw_path);
    if path.is_absolute() {
        return Err(ProductPathError::Invalid);
    }

    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() {
                    return Err(ProductPathError::Invalid);
                }
            }
            Component::Normal(value) => {
                let value = value.to_str().ok_or(ProductPathError::Invalid)?;
                if value.is_empty() {
                    return Err(ProductPathError::Invalid);
                }
                parts.push(value.to_owned());
            }
            Component::RootDir | Component::Prefix(_) => return Err(ProductPathError::Invalid),
        }
    }
    if parts.is_empty() {
        return Err(ProductPathError::Invalid);
    }

    let normalized = parts.join("/");
    ensure_product_path_does_not_escape(repo_root, canonical_repo_root, &normalized)?;
    Ok(normalized)
}

fn ensure_product_path_does_not_escape(
    repo_root: &Path,
    canonical_repo_root: &Path,
    normalized_path: &str,
) -> Result<(), ProductPathError> {
    let mut candidate = repo_root.join(normalized_path);
    while !candidate.exists() {
        if !candidate.pop() {
            return Err(ProductPathError::LocalAccess);
        }
    }
    let canonical_candidate =
        fs::canonicalize(candidate).map_err(|_| ProductPathError::LocalAccess)?;
    if canonical_candidate.starts_with(canonical_repo_root) {
        Ok(())
    } else {
        Err(ProductPathError::LocalAccess)
    }
}

fn baseline_matches(
    change_unit: &ChangeUnitRecord,
    task: &TaskRecord,
    baseline_ref: &BaselineRef,
) -> bool {
    let write_basis = parse_json_object(&change_unit.write_basis_json);
    let baseline = string_member(&write_basis, "baseline_ref")
        .or_else(|| StoredScope::from_task(task).baseline_ref);
    baseline.as_deref() == Some(baseline_ref.as_str())
}

fn paths_match_current_change_unit(
    repo_root: &Path,
    intended_paths: &[String],
    change_unit: &ChangeUnitRecord,
) -> bool {
    if intended_paths.is_empty() {
        return true;
    }
    let raw_bounded_paths =
        serde_json::from_str::<Vec<String>>(&change_unit.bounded_paths_json).unwrap_or_default();
    if raw_bounded_paths.is_empty() {
        return false;
    }
    let bounded_paths = normalize_product_paths(repo_root, &raw_bounded_paths).unwrap_or_default();
    !bounded_paths.is_empty()
        && intended_paths.iter().all(|path| {
            bounded_paths
                .iter()
                .any(|scope| path_is_within(path, scope))
        })
}

fn path_is_within(path: &str, scope: &str) -> bool {
    path == scope
        || path
            .strip_prefix(scope)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn matching_sensitive_approval(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &PrepareWriteRequest,
    task_id: &TaskId,
    change_unit: Option<&ChangeUnitRecord>,
    normalized_paths: &[String],
) -> Result<Option<UserJudgmentRecord>, PlanError> {
    let records = store
        .resolved_user_judgment_records(task_id, "sensitive_approval")
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let now = store.current_timestamp().map_err(|error| {
        PlanError::Response(Box::new(infallible_rejected_pipeline_response(
            request.envelope.dry_run,
            Some(project_state.state_version),
            vec![store_unavailable_error(error)],
        )))
    })?;

    for record in records {
        if !record
            .change_unit_id
            .as_deref()
            .map(|record_change_unit_id| {
                change_unit.map(|change_unit| change_unit.change_unit_id.as_str())
                    == Some(record_change_unit_id)
            })
            .unwrap_or(true)
        {
            continue;
        }
        let Ok(scope) =
            serde_json::from_str::<SensitiveActionScope>(&record.sensitive_action_scope_json)
        else {
            continue;
        };
        if sensitive_scope_matches(
            &store.project_record().repo_root,
            &request.sensitive_categories,
            normalized_paths,
            &scope,
            &now,
        ) {
            return Ok(Some(record));
        }
    }

    Ok(None)
}

fn sensitive_scope_matches(
    repo_root: &Path,
    sensitive_categories: &[String],
    normalized_paths: &[String],
    scope: &SensitiveActionScope,
    now: &str,
) -> bool {
    if scope
        .expires_at
        .as_deref()
        .is_some_and(|expires_at| expires_at <= now)
    {
        return false;
    }
    let Ok(scope_paths) = normalize_product_paths(repo_root, &scope.intended_paths) else {
        return false;
    };
    string_set(sensitive_categories) == string_set(&scope.sensitive_categories)
        && string_set(normalized_paths) == string_set(&scope_paths)
}

fn string_set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn surface_supports_prepare_write(capability_profile: &Value) -> bool {
    if capability_profile
        .get("supported_access_classes")
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values
                .iter()
                .any(|value| value.as_str() == Some("write_authorization"))
        })
    {
        return true;
    }
    if capability_profile
        .get("access_class")
        .and_then(Value::as_str)
        == Some("write_authorization")
    {
        return true;
    }
    if capability_profile
        .get("write_authorization")
        .and_then(Value::as_bool)
        == Some(true)
    {
        return true;
    }
    capability_profile
        .pointer("/capabilities/write_authorization")
        .and_then(Value::as_bool)
        == Some(true)
}

fn prepare_write_decision(reasons: &[WriteDecisionReason]) -> PrepareWriteDecision {
    if reasons.is_empty() {
        PrepareWriteDecision::Allowed
    } else if reasons
        .iter()
        .any(|reason| reason.code == "user_judgment_unresolved")
    {
        PrepareWriteDecision::DecisionRequired
    } else if reasons
        .iter()
        .any(|reason| reason.code == "sensitive_approval_missing")
    {
        PrepareWriteDecision::ApprovalRequired
    } else {
        PrepareWriteDecision::Blocked
    }
}

fn prepare_write_dry_run_summary(
    allowed: bool,
    reasons: &[WriteDecisionReason],
    _write_authorization_ref: Option<StateRecordRef>,
    _guarantee_display: Option<GuaranteeDisplay>,
) -> DryRunSummary {
    DryRunSummary {
        planned_effects: if allowed {
            vec![PlannedEffect {
                target_kind: "write_authorization".to_owned(),
                action: "would_create".to_owned(),
                description: "Prepare write would create one active Write Authorization."
                    .to_owned(),
            }]
        } else {
            Vec::new()
        },
        would_blockers: reasons
            .iter()
            .map(|reason| PlannedBlocker {
                source_kind: PlannedBlockerSourceKind::WriteDecision,
                category: write_decision_category_value(reason.category).to_owned(),
                code: reason.code.clone(),
                message: reason.message.clone(),
                related_refs: reason.related_refs.clone(),
            })
            .collect(),
        would_errors: Vec::new(),
        next_actions: Vec::new(),
        diagnostics: Vec::new(),
    }
}

fn write_decision_reason(
    category: WriteDecisionCategory,
    code: &'static str,
    message: &'static str,
    related_refs: Vec<StateRecordRef>,
) -> WriteDecisionReason {
    WriteDecisionReason {
        category,
        code: code.to_owned(),
        message: message.to_owned(),
        related_refs,
    }
}

fn write_decision_category_value(category: WriteDecisionCategory) -> &'static str {
    match category {
        WriteDecisionCategory::Scope => "scope",
        WriteDecisionCategory::UserJudgment => "user_judgment",
        WriteDecisionCategory::SensitiveApproval => "sensitive_approval",
        WriteDecisionCategory::WriteCompatibility => "write_compatibility",
        WriteDecisionCategory::Baseline => "baseline",
        WriteDecisionCategory::SurfaceCapability => "surface_capability",
    }
}

fn change_unit_ref(
    project_id: &ProjectId,
    task_id: &TaskId,
    change_unit: &ChangeUnitRecord,
    state_version: u64,
) -> StateRecordRef {
    state_ref(
        StateRecordKind::ChangeUnit,
        &change_unit.change_unit_id,
        project_id,
        Some(task_id),
        Some(state_version),
    )
}

fn write_authorization_guarantee() -> GuaranteeDisplay {
    GuaranteeDisplay {
        level: GuaranteeLevel::Cooperative,
        basis: "Write Authorization is a Harness compatibility record, not OS permission."
            .to_owned(),
        capability_refs: Vec::new(),
    }
}

struct RecordRunArtifactPlan {
    artifact_ref: ArtifactRef,
    claim: Option<String>,
    source_mutation: Option<CoreStorageMutation>,
    run_link: CoreStorageMutation,
}

struct RecordRunArtifactContext<'a> {
    store: &'a CoreProjectStore,
    project_state: &'a ProjectStateHeader,
    request: &'a RecordRunRequest,
    verified_surface: &'a VerifiedSurfaceContext,
    run_id: &'a RunId,
    run_ref: &'a StateRecordRef,
}

fn plan_record_run(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: RecordRunRequest,
    verified_surface: &VerifiedSurfaceContext,
) -> Result<MethodPlan, PlanError> {
    if request.summary.trim().is_empty() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "summary",
            "summary must not be empty",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if request
        .run_id
        .as_ref()
        .is_some_and(|id| id.as_str().trim().is_empty())
    {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "run_id",
            "run_id must be null or a non-empty identifier",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }

    let normalized_changed_paths = match normalize_product_paths(
        &store.project_record().repo_root,
        &request.observed_changes.changed_paths,
    ) {
        Ok(paths) => sorted_unique(paths),
        Err(ProductPathError::Invalid) => {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "observed_changes.changed_paths",
                "changed_paths must be relative Product Repository paths that stay inside the repository",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        Err(ProductPathError::LocalAccess) => {
            let response = rejected_pipeline_response(
                request.envelope.dry_run,
                Some(project_state.state_version),
                vec![tool_error(
                    ErrorCode::LocalAccessMismatch,
                    "changed_paths resolve outside the Product Repository",
                    false,
                    None,
                )],
            )
            .map_err(PlanError::Core)?;
            return Err(PlanError::Response(Box::new(response)));
        }
    };
    if request.observed_changes.product_file_write_observed && normalized_changed_paths.is_empty() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_changes",
            "product_file_write_observed requires at least one changed_path",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if !request.observed_changes.product_file_write_observed && !normalized_changed_paths.is_empty()
    {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_changes",
            "changed_paths require product_file_write_observed=true",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    if request
        .observed_changes
        .baseline_ref
        .as_ref()
        .is_some_and(|baseline_ref| baseline_ref != &request.baseline_ref)
    {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "observed_changes.baseline_ref",
            "observed_changes.baseline_ref must match request baseline_ref when present",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }

    let task = store
        .task_record(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let change_unit = store
        .change_unit_record(&request.task_id, request.change_unit_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_change_unit_response(
                &request.envelope,
                Some(project_state.state_version),
                "change_unit_id does not identify a Change Unit for the Task",
            )))
        })?;
    if change_unit.status != "active" || !change_unit.is_current {
        return Err(PlanError::Response(Box::new(
            no_active_change_unit_response(
                &request.envelope,
                Some(project_state.state_version),
                "record_run requires the current active Change Unit",
            ),
        )));
    }
    if !baseline_matches(&change_unit, &task, &request.baseline_ref) {
        return Err(PlanError::Response(Box::new(baseline_stale_response(
            &request.envelope,
            Some(project_state.state_version),
            &request.baseline_ref,
        ))));
    }

    let planned_state_version = project_state.state_version + 1;
    let run_id = match request.run_id.clone() {
        Some(run_id) => run_id,
        None => allocate_run_id(service, store).map_err(PlanError::Core)?,
    };
    if request.run_id.is_some()
        && store.run_id_exists(run_id.as_str()).map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
    {
        let response = validation_rejected(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "run_id",
            "run_id already identifies an existing Run",
        )
        .map_err(PlanError::Core)?;
        return Err(PlanError::Response(Box::new(response)));
    }
    let run_ref = state_ref(
        StateRecordKind::Run,
        run_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let normalized_observed_changes = ObservedChanges {
        changed_paths: normalized_changed_paths.clone(),
        product_file_write_observed: request.observed_changes.product_file_write_observed,
        sensitive_categories: sorted_unique(request.observed_changes.sensitive_categories.clone()),
        baseline_ref: Some(request.baseline_ref.clone()),
    };

    let artifact_plans = plan_record_run_artifacts(
        service,
        store,
        project_state,
        &request,
        verified_surface,
        &run_id,
        &run_ref,
    )?;
    let registered_artifacts = artifact_plans
        .iter()
        .map(|plan| plan.artifact_ref.clone())
        .collect::<Vec<_>>();

    let authorization_record = if request.observed_changes.product_file_write_observed {
        let Some(write_authorization_id) = &request.write_authorization_id else {
            return Err(PlanError::Response(Box::new(
                write_authorization_required_response(
                    &request.envelope,
                    Some(project_state.state_version),
                ),
            )));
        };
        let record = store
            .write_authorization_record(write_authorization_id.as_str())
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?
            .ok_or_else(|| {
                PlanError::Response(Box::new(write_authorization_invalid_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "missing",
                    "write_authorization_id does not identify a Write Authorization",
                )))
            })?;
        validate_write_authorization_for_run(
            store,
            project_state,
            &request,
            &record,
            &normalized_observed_changes,
        )?;
        Some(record)
    } else {
        if request.write_authorization_id.is_some() {
            return Err(PlanError::Response(Box::new(
                write_authorization_invalid_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "incompatible",
                    "write_authorization_id is only consumed for observed product-file writes",
                ),
            )));
        }
        None
    };

    let evidence_summary = build_record_run_evidence_summary(
        &request,
        &run_ref,
        &registered_artifacts,
        &artifact_plans,
    );
    let blocker_refs = store
        .active_blocker_refs(&request.task_id, planned_state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: Some(&change_unit),
        pending_user_judgment_refs: store
            .pending_user_judgment_refs(&request.task_id, planned_state_version)
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?
            .into_iter()
            .map(state_ref_from_stored)
            .collect(),
        blocker_refs: blocker_refs.clone(),
        active_write_authorization: None,
        options: SummaryOptions::mutation(),
    })?;

    let run_summary = RunSummary {
        run_ref: run_ref.clone(),
        kind: request.kind,
        summary: request.summary.clone(),
        observed_changes: normalized_observed_changes.clone(),
        artifact_refs: registered_artifacts.clone(),
    };
    let result = RecordRunResult {
        base: placeholder_base(),
        run_summary,
        registered_artifacts: registered_artifacts.clone(),
        evidence_summary: evidence_summary.clone(),
        blocker_refs,
        state,
    };

    let mut storage_mutations = vec![CoreStorageMutation::InsertRun(RunInsert {
        run_id: run_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        change_unit_id: Some(request.change_unit_id.as_str().to_owned()),
        write_authorization_id: request
            .write_authorization_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        kind: storage_value(request.kind)?,
        status: "recorded".to_owned(),
        summary_json: serde_json::to_string(&json!({
            "summary": request.summary
        }))?,
        observed_changes_json: serde_json::to_string(&normalized_observed_changes)?,
        evidence_updates_json: serde_json::to_string(&request.evidence_updates)?,
        authorization_effect_json: serde_json::to_string(&json!({
            "write_authorization_id": request.write_authorization_id,
            "effect": if authorization_record.is_some() { "consumed" } else { "none" }
        }))?,
        created_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
        created_by_surface_instance_id: verified_surface.surface_instance_id.as_str().to_owned(),
        metadata_json: serde_json::to_string(&json!({
            "verification_basis": verified_surface.verification_basis.clone()
        }))?,
    })];
    if let Some(record) = &authorization_record {
        storage_mutations.push(CoreStorageMutation::ConsumeWriteAuthorization(
            WriteAuthorizationConsumption {
                write_authorization_id: record.write_authorization_id.clone(),
                run_id: run_id.as_str().to_owned(),
                expected_basis_state_version: record.basis_state_version,
            },
        ));
    }
    for plan in &artifact_plans {
        if let Some(mutation) = &plan.source_mutation {
            storage_mutations.push(mutation.clone());
        }
        storage_mutations.push(plan.run_link.clone());
    }
    if let Some(evidence_summary) = &evidence_summary {
        let evidence_summary_id =
            allocate_evidence_summary_id(service, store).map_err(PlanError::Core)?;
        storage_mutations.push(CoreStorageMutation::UpsertEvidenceSummary(
            EvidenceSummaryUpsert {
                evidence_summary_id: evidence_summary_id.clone(),
                task_id: request.task_id.as_str().to_owned(),
                change_unit_id: Some(request.change_unit_id.as_str().to_owned()),
                status: storage_value(evidence_summary.status)?,
                coverage_json: serde_json::to_string(&evidence_summary.coverage_items)?,
                supporting_refs_json: serde_json::to_string(
                    &evidence_summary
                        .coverage_items
                        .iter()
                        .flat_map(|item| item.supporting_refs.clone())
                        .collect::<Vec<_>>(),
                )?,
                gap_refs_json: serde_json::to_string(
                    &evidence_summary
                        .coverage_items
                        .iter()
                        .flat_map(|item| item.gap_refs.clone())
                        .collect::<Vec<_>>(),
                )?,
                metadata_json: serde_json::to_string(&json!({
                    "updated_by_run_id": run_id.as_str()
                }))?,
            },
        ));
        for artifact_ref in &registered_artifacts {
            storage_mutations.push(CoreStorageMutation::LinkArtifact(ArtifactLinkInsert {
                artifact_id: artifact_ref.artifact_id.as_str().to_owned(),
                task_id: request.task_id.as_str().to_owned(),
                owner_record_kind: "evidence_summary".to_owned(),
                owner_record_id: evidence_summary_id.clone(),
                created_by_run_id: run_id.as_str().to_owned(),
                metadata_json: serde_json::to_string(&json!({
                    "relation": "evidence_support"
                }))?,
            }));
        }
    }

    let event_payload = object_from_value(json!({
        "task_id": request.task_id,
        "change_unit_id": request.change_unit_id,
        "run_id": run_id,
        "kind": request.kind,
        "product_file_write_observed": normalized_observed_changes.product_file_write_observed,
        "write_authorization_id": authorization_record
            .as_ref()
            .map(|record| record.write_authorization_id.clone()),
        "artifact_ids": registered_artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.as_str().to_owned())
            .collect::<Vec<_>>()
    }))?;

    Ok(MethodPlan {
        task_id: request.task_id,
        change_unit_id: Some(request.change_unit_id),
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions: Vec::new(),
    })
}

fn plan_record_run_artifacts(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
    verified_surface: &VerifiedSurfaceContext,
    run_id: &RunId,
    run_ref: &StateRecordRef,
) -> Result<Vec<RecordRunArtifactPlan>, PlanError> {
    let context = RecordRunArtifactContext {
        store,
        project_state,
        request,
        verified_surface,
        run_id,
        run_ref,
    };
    let mut input_ids = BTreeSet::new();
    let mut staged_handles = BTreeSet::new();
    let mut plans = Vec::new();
    for input in &request.artifact_inputs {
        if input.artifact_input_id.as_str().trim().is_empty() {
            return artifact_input_validation_plan_error(
                request,
                project_state,
                input,
                "staged_handle_not_found",
                "artifact_input_id must not be empty",
            );
        }
        if !input_ids.insert(input.artifact_input_id.as_str()) {
            return artifact_input_validation_plan_error(
                request,
                project_state,
                input,
                "staged_handle_not_found",
                "artifact_input_id values must be unique within one request",
            );
        }
        match input.source_kind {
            ArtifactInputSourceKind::StagedArtifact => {
                if input.staged_artifact_handle.is_none() || input.existing_artifact_ref.is_some() {
                    return artifact_input_validation_plan_error(
                        request,
                        project_state,
                        input,
                        "staged_handle_not_found",
                        "staged_artifact inputs must populate only staged_artifact_handle",
                    );
                }
                let handle = input
                    .staged_artifact_handle
                    .as_ref()
                    .expect("checked staged_artifact_handle above");
                if !staged_handles.insert(handle.handle_id.as_str()) {
                    return artifact_input_validation_plan_error(
                        request,
                        project_state,
                        input,
                        "staged_handle_consumed",
                        "a staged artifact handle can be consumed at most once",
                    );
                }
                plans.push(plan_staged_artifact_input(
                    service, &context, input, handle,
                )?);
            }
            ArtifactInputSourceKind::ExistingArtifact => {
                if input.existing_artifact_ref.is_none() || input.staged_artifact_handle.is_some() {
                    return artifact_input_validation_plan_error(
                        request,
                        project_state,
                        input,
                        "staged_handle_not_found",
                        "existing_artifact inputs must populate only existing_artifact_ref",
                    );
                }
                plans.push(plan_existing_artifact_input(
                    &context,
                    input,
                    input
                        .existing_artifact_ref
                        .as_ref()
                        .expect("checked existing_artifact_ref above"),
                )?);
            }
        }
    }
    Ok(plans)
}

fn plan_staged_artifact_input(
    service: &CoreService,
    context: &RecordRunArtifactContext<'_>,
    input: &ArtifactInput,
    handle: &StagedArtifactHandle,
) -> Result<RecordRunArtifactPlan, PlanError> {
    let store = context.store;
    let project_state = context.project_state;
    let request = context.request;
    let verified_surface = context.verified_surface;
    let run_id = context.run_id;
    let run_ref = context.run_ref;
    if handle.project_id != request.envelope.project_id {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_project_mismatch",
            "staged artifact handle belongs to a different project",
        );
    }
    if handle.task_id != request.task_id {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_task_mismatch",
            "staged artifact handle belongs to a different Task",
        );
    }
    if handle.consumed {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_consumed",
            "staged artifact handle is already consumed",
        );
    }

    let record = store
        .artifact_staging_record(handle.handle_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(artifact_input_validation_response(
                request,
                project_state,
                input,
                "staged_handle_not_found",
                "staged artifact handle cannot be found",
            )))
        })?;
    validate_staged_artifact_record(
        store,
        project_state,
        request,
        verified_surface,
        input,
        handle,
        &record,
    )?;

    let artifact_id = allocate_artifact_id(service, store).map_err(PlanError::Core)?;
    let uri = format!(
        "harness-artifact://{}/{}",
        request.envelope.project_id.as_str(),
        artifact_id.as_str()
    );
    let display_name = staged_artifact_display_name(&record);
    let content_type = record
        .content_type
        .clone()
        .unwrap_or_else(|| handle.content_type.clone());
    let sha256 = record
        .sha256
        .clone()
        .expect("staged artifact validation ensures sha256 is present");
    let size_bytes = record
        .size_bytes
        .expect("staged artifact validation ensures size_bytes is present");
    let redaction_state =
        parse_storage_value("artifact_staging.redaction_state", &record.redaction_state)?;
    let artifact_ref = ArtifactRef {
        artifact_id: artifact_id.clone(),
        project_id: request.envelope.project_id.clone(),
        task_id: request.task_id.clone(),
        display_name: display_name.clone(),
        content_type: content_type.clone(),
        sha256: sha256.clone(),
        size_bytes,
        redaction_state,
        availability: ArtifactAvailability::Available,
        created_by_run_ref: Some(run_ref.clone()),
        created_by_surface_id: Some(SurfaceId::new(record.created_by_surface_id.clone())),
        created_by_surface_instance_id: Some(SurfaceInstanceId::new(
            record.created_by_surface_instance_id.clone(),
        )),
        storage_ref: Some(StorageRef::new(uri.clone())),
    };
    let source_mutation = Some(CoreStorageMutation::PromoteStagedArtifact(
        ArtifactPromotion {
            handle_id: handle.handle_id.as_str().to_owned(),
            artifact_id: artifact_id.as_str().to_owned(),
            task_id: request.task_id.as_str().to_owned(),
            run_id: run_id.as_str().to_owned(),
            expected_created_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
            expected_created_by_surface_instance_id: verified_surface
                .surface_instance_id
                .as_str()
                .to_owned(),
            expected_sha256: sha256,
            expected_size_bytes: size_bytes,
            expected_redaction_state: record.redaction_state.clone(),
            uri,
            retention_json: "{}".to_owned(),
            producer_json: serde_json::to_string(&json!({
                "display_name": display_name,
                "content_type": content_type,
                "created_by_surface_id": verified_surface.surface_id.as_str(),
                "created_by_surface_instance_id": verified_surface.surface_instance_id.as_str(),
                "artifact_input_id": input.artifact_input_id.as_str(),
                "relation_hint": input.relation_hint,
                "claim": input.claim
            }))?,
            metadata_json: serde_json::to_string(&json!({
                "source_kind": "staged_artifact"
            }))?,
        },
    ));
    let run_link = CoreStorageMutation::LinkArtifact(ArtifactLinkInsert {
        artifact_id: artifact_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        owner_record_kind: "run".to_owned(),
        owner_record_id: run_id.as_str().to_owned(),
        created_by_run_id: run_id.as_str().to_owned(),
        metadata_json: artifact_link_metadata(input)?,
    });

    Ok(RecordRunArtifactPlan {
        artifact_ref,
        claim: input.claim.clone(),
        source_mutation,
        run_link,
    })
}

fn validate_staged_artifact_record(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
    verified_surface: &VerifiedSurfaceContext,
    input: &ArtifactInput,
    handle: &StagedArtifactHandle,
    record: &StoredArtifactStagingRecord,
) -> Result<(), PlanError> {
    if record.project_id != request.envelope.project_id.as_str() {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_project_mismatch",
            "stored staged artifact belongs to a different project",
        );
    }
    if record.task_id != request.task_id.as_str() {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_task_mismatch",
            "stored staged artifact belongs to a different Task",
        );
    }
    if record.created_by_surface_id != verified_surface.surface_id.as_str()
        || record.created_by_surface_instance_id != verified_surface.surface_instance_id.as_str()
        || handle.created_by_surface_id.as_str() != record.created_by_surface_id
        || handle.created_by_surface_instance_id.as_str() != record.created_by_surface_instance_id
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_surface_mismatch",
            "staged artifact provenance does not match the verified surface",
        );
    }
    if record.status == "consumed" {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_consumed",
            "staged artifact handle is already consumed",
        );
    }
    let now = store.current_timestamp().map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })?;
    if record.status == "expired" || record.expires_at <= now {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_expired",
            "staged artifact handle is expired",
        );
    }
    if record.status != "staged" {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_not_found",
            "staged artifact handle is not consumable",
        );
    }
    if record.sha256.as_deref() != Some(handle.sha256.as_str())
        || input
            .expected_sha256
            .as_deref()
            .is_some_and(|expected| record.sha256.as_deref() != Some(expected))
        || record.sha256.is_none()
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "staged artifact checksum does not match the submitted handle or expectation",
        );
    }
    if record.size_bytes != Some(handle.size_bytes)
        || input
            .expected_size_bytes
            .is_some_and(|expected| record.size_bytes != Some(expected))
        || record.size_bytes.is_none()
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_size_mismatch",
            "staged artifact size does not match the submitted handle or expectation",
        );
    }
    let expected_redaction = input.redaction_state.unwrap_or(handle.redaction_state);
    if record.redaction_state != redaction_state_value(handle.redaction_state)
        || record.redaction_state != redaction_state_value(expected_redaction)
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "staged artifact redaction_state does not match the submitted handle or expectation",
        );
    }
    if record.content_type.as_deref() != Some(handle.content_type.as_str()) {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "staged artifact content_type does not match the submitted handle",
        );
    }
    Ok(())
}

fn plan_existing_artifact_input(
    context: &RecordRunArtifactContext<'_>,
    input: &ArtifactInput,
    existing_ref: &ArtifactRef,
) -> Result<RecordRunArtifactPlan, PlanError> {
    let store = context.store;
    let project_state = context.project_state;
    let request = context.request;
    let run_id = context.run_id;
    if existing_ref.project_id != request.envelope.project_id
        || existing_ref.task_id != request.task_id
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_project_mismatch",
            "existing artifact ref must belong to the request project and Task",
        );
    }
    let record = store
        .artifact_record(existing_ref.artifact_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(artifact_missing_response(
                request,
                project_state,
                "existing artifact cannot be found",
            )))
        })?;
    if record.task_id != request.task_id.as_str()
        || record.project_id != request.envelope.project_id.as_str()
        || record.status != "available"
        || !store
            .artifact_has_task_owner_link(
                existing_ref.artifact_id.as_str(),
                request.task_id.as_str(),
            )
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?
    {
        return Err(PlanError::Response(Box::new(artifact_missing_response(
            request,
            project_state,
            "existing artifact is not available for this Task",
        ))));
    }
    if record.sha256.as_deref() != Some(existing_ref.sha256.as_str())
        || input
            .expected_sha256
            .as_deref()
            .is_some_and(|expected| record.sha256.as_deref() != Some(expected))
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "existing artifact checksum does not match the stored artifact",
        );
    }
    if record.size_bytes != Some(existing_ref.size_bytes)
        || input
            .expected_size_bytes
            .is_some_and(|expected| record.size_bytes != Some(expected))
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_size_mismatch",
            "existing artifact size does not match the stored artifact",
        );
    }
    let expected_redaction = input
        .redaction_state
        .unwrap_or(existing_ref.redaction_state);
    if record.redaction_state != redaction_state_value(existing_ref.redaction_state)
        || record.redaction_state != redaction_state_value(expected_redaction)
    {
        return artifact_input_validation_plan_error(
            request,
            project_state,
            input,
            "staged_handle_checksum_mismatch",
            "existing artifact redaction_state does not match the stored artifact",
        );
    }
    let artifact_ref =
        artifact_ref_from_stored_record(&record, Some(existing_ref.display_name.clone()))?;
    let run_link = CoreStorageMutation::LinkArtifact(ArtifactLinkInsert {
        artifact_id: existing_ref.artifact_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        owner_record_kind: "run".to_owned(),
        owner_record_id: run_id.as_str().to_owned(),
        created_by_run_id: run_id.as_str().to_owned(),
        metadata_json: artifact_link_metadata(input)?,
    });
    Ok(RecordRunArtifactPlan {
        artifact_ref,
        claim: input.claim.clone(),
        source_mutation: None,
        run_link,
    })
}

fn validate_write_authorization_for_run(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
    record: &WriteAuthorizationRecord,
    observed_changes: &ObservedChanges,
) -> Result<(), PlanError> {
    if record.status != "active" {
        let reason = match record.status.as_str() {
            "consumed" => "consumed",
            "expired" => "expired",
            "stale" => "stale",
            "revoked" => "revoked",
            _ => "incompatible",
        };
        return Err(PlanError::Response(Box::new(
            write_authorization_invalid_response(
                &request.envelope,
                Some(project_state.state_version),
                reason,
                "Write Authorization is not active",
            ),
        )));
    }
    let now = store.current_timestamp().map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })?;
    if record.expires_at <= now {
        return Err(PlanError::Response(Box::new(
            write_authorization_invalid_response(
                &request.envelope,
                Some(project_state.state_version),
                "expired",
                "Write Authorization is expired",
            ),
        )));
    }
    if record.basis_state_version != project_state.state_version {
        return Err(PlanError::Response(Box::new(
            stale_write_authorization_basis_response(
                &request.envelope,
                record,
                project_state.state_version,
            ),
        )));
    }
    let scope: AuthorizedAttemptScope = parse_json_text(
        "write_authorizations.attempt_scope_json",
        &record.attempt_scope_json,
    )?;
    let scope_paths =
        normalize_product_paths(&store.project_record().repo_root, &scope.intended_paths)
            .unwrap_or_default();
    if record.task_id != request.task_id.as_str()
        || record.change_unit_id.as_deref() != Some(request.change_unit_id.as_str())
        || scope.task_id != request.task_id
        || scope.change_unit_id != request.change_unit_id
        || !scope.product_file_write_intended
        || scope.baseline_ref.as_ref() != Some(&request.baseline_ref)
        || string_set(&scope.sensitive_categories)
            != string_set(&observed_changes.sensitive_categories)
        || !paths_are_authorized(&observed_changes.changed_paths, &scope_paths)
    {
        return Err(PlanError::Response(Box::new(
            write_authorization_invalid_response(
                &request.envelope,
                Some(project_state.state_version),
                "incompatible",
                "Write Authorization is not compatible with the recorded run",
            ),
        )));
    }
    Ok(())
}

fn paths_are_authorized(observed_paths: &[String], authorized_paths: &[String]) -> bool {
    !observed_paths.is_empty()
        && !authorized_paths.is_empty()
        && observed_paths.iter().all(|path| {
            authorized_paths
                .iter()
                .any(|authorized| path_is_within(path, authorized))
        })
}

fn build_record_run_evidence_summary(
    request: &RecordRunRequest,
    run_ref: &StateRecordRef,
    registered_artifacts: &[ArtifactRef],
    artifact_plans: &[RecordRunArtifactPlan],
) -> Option<harness_types::EvidenceSummary> {
    if request.evidence_updates.is_empty() {
        return None;
    }
    let mut coverage_items = Vec::new();
    for update in &request.evidence_updates {
        let mut item = update.clone();
        if !item.supporting_refs.iter().any(|record_ref| {
            record_ref.record_kind == StateRecordKind::Run
                && record_ref.record_id == run_ref.record_id
        }) {
            item.supporting_refs.push(run_ref.clone());
        }
        for plan in artifact_plans {
            if plan.claim.as_deref() == Some(update.claim.as_str())
                && !item
                    .supporting_artifact_refs
                    .iter()
                    .any(|artifact| artifact.artifact_id == plan.artifact_ref.artifact_id)
            {
                item.supporting_artifact_refs
                    .push(plan.artifact_ref.clone());
            }
        }
        coverage_items.push(item);
    }
    let artifact_refs = unique_artifact_refs(
        registered_artifacts
            .iter()
            .cloned()
            .chain(
                coverage_items
                    .iter()
                    .flat_map(|item| item.supporting_artifact_refs.clone()),
            )
            .collect(),
    );
    let required_claims = coverage_items
        .iter()
        .filter(|item| item.required_for_close)
        .map(|item| item.claim.clone())
        .collect::<Vec<_>>();
    let status = evidence_status_for_items(&coverage_items);
    Some(harness_types::EvidenceSummary {
        status,
        completion_policy: CompletionPolicy {
            evidence_required: !required_claims.is_empty(),
            required_claims,
        },
        coverage_items,
        artifact_refs,
        updated_by_run_ref: Some(run_ref.clone()),
    })
}

fn evidence_status_for_items(items: &[EvidenceCoverageItem]) -> EvidenceStatus {
    if items
        .iter()
        .any(|item| item.coverage_state == EvidenceCoverageState::Blocked)
    {
        return EvidenceStatus::Blocked;
    }
    let required = items
        .iter()
        .filter(|item| item.required_for_close)
        .collect::<Vec<_>>();
    if required.is_empty() {
        return EvidenceStatus::Unknown;
    }
    if required.iter().all(|item| {
        matches!(
            item.coverage_state,
            EvidenceCoverageState::Supported | EvidenceCoverageState::NotApplicable
        )
    }) {
        EvidenceStatus::Sufficient
    } else {
        EvidenceStatus::Insufficient
    }
}

fn unique_artifact_refs(artifact_refs: Vec<ArtifactRef>) -> Vec<ArtifactRef> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for artifact_ref in artifact_refs {
        if seen.insert(artifact_ref.artifact_id.as_str().to_owned()) {
            unique.push(artifact_ref);
        }
    }
    unique
}

fn artifact_ref_from_stored_record(
    record: &StoredArtifactRecord,
    display_name: Option<String>,
) -> CoreResult<ArtifactRef> {
    let producer = parse_json_object(&record.producer_json);
    let task_id = TaskId::new(record.task_id.clone());
    Ok(ArtifactRef {
        artifact_id: ArtifactId::new(record.artifact_id.clone()),
        project_id: ProjectId::new(record.project_id.clone()),
        task_id: task_id.clone(),
        display_name: display_name
            .or_else(|| string_member(&producer, "display_name"))
            .unwrap_or_else(|| record.artifact_id.clone()),
        content_type: record
            .content_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_owned()),
        sha256: record.sha256.clone().unwrap_or_default(),
        size_bytes: record.size_bytes.unwrap_or_default(),
        redaction_state: parse_storage_value("artifacts.redaction_state", &record.redaction_state)?,
        availability: match record.status.as_str() {
            "available" => ArtifactAvailability::Available,
            "missing" => ArtifactAvailability::Missing,
            "integrity_failed" => ArtifactAvailability::IntegrityFailed,
            "unavailable" => ArtifactAvailability::Unavailable,
            _ => ArtifactAvailability::Unusable,
        },
        created_by_run_ref: record.producer_run_id.as_ref().map(|run_id| {
            state_ref(
                StateRecordKind::Run,
                run_id,
                &ProjectId::new(record.project_id.clone()),
                Some(&task_id),
                None,
            )
        }),
        created_by_surface_id: string_member(&producer, "created_by_surface_id")
            .map(SurfaceId::new),
        created_by_surface_instance_id: string_member(&producer, "created_by_surface_instance_id")
            .map(SurfaceInstanceId::new),
        storage_ref: Some(StorageRef::new(record.uri.clone())),
    })
}

fn staged_artifact_display_name(record: &StoredArtifactStagingRecord) -> String {
    string_member(&parse_json_object(&record.artifact_json), "display_name")
        .unwrap_or_else(|| record.handle_id.clone())
}

fn artifact_link_metadata(input: &ArtifactInput) -> CoreResult<String> {
    Ok(serde_json::to_string(&json!({
        "artifact_input_id": input.artifact_input_id.as_str(),
        "source_kind": input.source_kind,
        "relation_hint": input.relation_hint,
        "claim": input.claim
    }))?)
}

fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn artifact_input_validation_plan_error<T>(
    request: &RecordRunRequest,
    project_state: &ProjectStateHeader,
    input: &ArtifactInput,
    reason: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    Err(PlanError::Response(Box::new(
        artifact_input_validation_response(request, project_state, input, reason, message),
    )))
}

fn artifact_input_validation_response(
    request: &RecordRunRequest,
    project_state: &ProjectStateHeader,
    input: &ArtifactInput,
    reason: &'static str,
    message: &'static str,
) -> PipelineResponse {
    let details = object_from_value(json!({
        "artifact_input_error": {
            "artifact_input_id": input.artifact_input_id.as_str(),
            "reason": reason
        }
    }))
    .expect("artifact input error details should be an object");
    infallible_rejected_pipeline_response(
        request.envelope.dry_run,
        Some(project_state.state_version),
        vec![tool_error(
            ErrorCode::ValidationFailed,
            message,
            false,
            Some(details),
        )],
    )
}

fn artifact_missing_response(
    request: &RecordRunRequest,
    project_state: &ProjectStateHeader,
    message: &'static str,
) -> PipelineResponse {
    infallible_rejected_pipeline_response(
        request.envelope.dry_run,
        Some(project_state.state_version),
        vec![tool_error(ErrorCode::ArtifactMissing, message, false, None)],
    )
}

fn write_authorization_required_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
) -> PipelineResponse {
    let details = object_from_value(json!({
        "authorization_reason": "missing"
    }))
    .expect("authorization details should be an object");
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::WriteAuthorizationRequired,
            "product-file write observations require a compatible active Write Authorization",
            false,
            Some(details),
        )],
    )
}

fn write_authorization_invalid_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    reason: &'static str,
    message: &'static str,
) -> PipelineResponse {
    let details = object_from_value(json!({
        "authorization_reason": reason
    }))
    .expect("authorization details should be an object");
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::WriteAuthorizationInvalid,
            message,
            false,
            Some(details),
        )],
    )
}

fn stale_write_authorization_basis_response(
    envelope: &ToolEnvelope,
    record: &WriteAuthorizationRecord,
    current_state_version: u64,
) -> PipelineResponse {
    let mut details = Map::new();
    details.insert(
        "state_clock".to_owned(),
        Value::String("project_state.state_version".to_owned()),
    );
    details.insert(
        "current_state_version".to_owned(),
        Value::from(current_state_version),
    );
    details.insert(
        "write_authorization_basis_state_version".to_owned(),
        Value::from(record.basis_state_version),
    );
    details.insert(
        "write_authorization_id".to_owned(),
        Value::String(record.write_authorization_id.clone()),
    );
    details.insert(
        "project_id".to_owned(),
        Value::String(envelope.project_id.as_str().to_owned()),
    );
    if let Some(task_id) = &envelope.task_id {
        details.insert(
            "task_id".to_owned(),
            Value::String(task_id.as_str().to_owned()),
        );
    }
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        Some(current_state_version),
        vec![tool_error(
            ErrorCode::StateVersionConflict,
            "Write Authorization basis_state_version is stale",
            true,
            Some(details),
        )],
    )
}

fn baseline_stale_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    baseline_ref: &BaselineRef,
) -> PipelineResponse {
    let details = object_from_value(json!({
        "baseline_ref": baseline_ref.as_str()
    }))
    .expect("baseline details should be an object");
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::BaselineStale,
            "baseline_ref does not match the current Change Unit basis",
            true,
            Some(details),
        )],
    )
}

fn no_active_change_unit_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    message: &'static str,
) -> PipelineResponse {
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::NoActiveChangeUnit,
            message,
            false,
            None,
        )],
    )
}

fn plan_request_user_judgment(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: harness_types::RequestUserJudgmentRequest,
    verified_surface: &VerifiedSurfaceContext,
) -> Result<MethodPlan, PlanError> {
    validate_user_judgment_request_fields(UserJudgmentRequestValidation {
        dry_run: request.envelope.dry_run,
        state_version: Some(project_state.state_version),
        question: &request.question,
        options: &request.options,
        context: &request.context,
        affected_refs: &request.affected_refs,
        project_id: &request.envelope.project_id,
        task_id: &request.task_id,
        expires_at: request.expires_at.as_deref(),
        current_timestamp: store.current_timestamp().map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?,
    })?;

    let planned_state_version = project_state.state_version + 1;
    let task = store
        .task_record(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let current_change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let branch_change_unit_id = if let Some(change_unit_id) = &request.change_unit_id {
        let existing = store
            .change_unit_record(&request.task_id, change_unit_id.as_str())
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?;
        if existing.is_none() {
            let response = rejected_pipeline_response(
                request.envelope.dry_run,
                Some(project_state.state_version),
                vec![tool_error(
                    ErrorCode::NoActiveChangeUnit,
                    "change_unit_id does not identify a Change Unit for the Task",
                    false,
                    None,
                )],
            )
            .map_err(PlanError::Core)?;
            return Err(PlanError::Response(Box::new(response)));
        }
        Some(change_unit_id.clone())
    } else {
        None
    };

    let requested_at = store.current_timestamp().map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })?;
    let judgment_id = allocate_user_judgment_id(service, store).map_err(PlanError::Core)?;
    let user_judgment_ref = state_ref(
        StateRecordKind::UserJudgment,
        judgment_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let user_judgment = UserJudgment {
        judgment_id: judgment_id.clone(),
        project_id: request.envelope.project_id.clone(),
        task_id: request.task_id.clone(),
        change_unit_id: request.change_unit_id.clone(),
        judgment_kind: request.judgment_kind,
        status: UserJudgmentStatus::Pending,
        presentation: request.presentation,
        question: request.question.clone(),
        options: request.options.clone(),
        context: request.context.clone(),
        affected_refs: request.affected_refs.clone(),
        required_for: request.required_for,
        resolution: None,
        expires_at: request.expires_at.clone(),
        created_at: requested_at.clone(),
        resolved_at: None,
    };

    let mut pending_refs = store
        .pending_user_judgment_refs(&request.task_id, planned_state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    pending_refs.push(user_judgment_ref.clone());
    let blocker_refs = store
        .active_blocker_refs(&request.task_id, planned_state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let task_ref = state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let change_unit_ref = current_change_unit.as_ref().map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(record.basis_state_version.unwrap_or(planned_state_version)),
        )
    });
    let next_actions = next_actions_for_state(&task_ref, change_unit_ref.as_ref());
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: current_change_unit.as_ref(),
        pending_user_judgment_refs: pending_refs,
        blocker_refs: blocker_refs.clone(),
        active_write_authorization: None,
        options: SummaryOptions::mutation(),
    })?;
    let result = harness_types::RequestUserJudgmentResult {
        base: placeholder_base(),
        user_judgment_ref: user_judgment_ref.clone(),
        user_judgment,
        blocker_refs,
        state,
    };
    let storage_mutations = vec![CoreStorageMutation::InsertUserJudgment(
        UserJudgmentInsert {
            judgment_id: judgment_id.as_str().to_owned(),
            task_id: request.task_id.as_str().to_owned(),
            change_unit_id: request
                .change_unit_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            judgment_kind: storage_value(request.judgment_kind)?,
            request_json: serde_json::to_string(&json!({
                "presentation": request.presentation,
                "question": request.question,
                "required_for": request.required_for,
                "expires_at": request.expires_at
            }))?,
            context_json: serde_json::to_string(&request.context)?,
            options_json: serde_json::to_string(&request.options)?,
            affected_refs_json: serde_json::to_string(&request.affected_refs)?,
            artifact_refs_json: serde_json::to_string(&request.context.artifact_refs)?,
            sensitive_action_scope_json: "{}".to_owned(),
            requested_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
            requested_by_surface_instance_id: verified_surface
                .surface_instance_id
                .as_str()
                .to_owned(),
            requested_at,
            metadata_json: serde_json::to_string(&json!({
                "requested_by_actor_kind": request.envelope.actor_kind
            }))?,
        },
    )];
    let event_payload = object_from_value(json!({
        "task_id": request.task_id,
        "change_unit_id": request.change_unit_id,
        "judgment_id": judgment_id,
        "judgment_kind": request.judgment_kind,
        "required_for": request.required_for
    }))?;

    Ok(MethodPlan {
        task_id: user_judgment_ref
            .task_id
            .clone()
            .expect("user judgment refs are task-scoped"),
        change_unit_id: branch_change_unit_id,
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions,
    })
}

fn plan_record_user_judgment(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: RecordUserJudgmentRequest,
) -> Result<MethodPlan, PlanError> {
    let planned_state_version = project_state.state_version + 1;
    let record = store
        .user_judgment_record(request.user_judgment_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "user_judgment_id does not identify a pending user-owned judgment",
            )))
        })?;
    if let Some(envelope_task_id) = &request.envelope.task_id {
        if envelope_task_id.as_str() != record.task_id {
            let response = validation_rejected(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "task_id",
                "envelope.task_id must match the addressed UserJudgment task_id",
            )
            .map_err(PlanError::Core)?;
            return Err(PlanError::Response(Box::new(response)));
        }
    }
    if record.status != "pending" {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "user_judgment_id does not identify a pending user-owned judgment",
        ))));
    }

    let mut user_judgment = user_judgment_from_record(&record)?;
    let now = store.current_timestamp().map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })?;
    if user_judgment
        .expires_at
        .as_deref()
        .is_some_and(|expires_at| expires_at <= now.as_str())
    {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "pending user-owned judgment is expired",
        ))));
    }
    if request.judgment_kind != user_judgment.judgment_kind {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "judgment_kind is incompatible with the pending user-owned judgment",
        ))));
    }
    if !user_judgment
        .options
        .iter()
        .any(|option| option.option_id == request.selected_option_id)
    {
        let response = validation_rejected(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "selected_option_id",
            "selected_option_id is not one of the pending judgment options",
        )
        .map_err(PlanError::Core)?;
        return Err(PlanError::Response(Box::new(response)));
    }
    validate_answer_payload(
        request.envelope.dry_run,
        Some(project_state.state_version),
        request.judgment_kind,
        &request.answer,
    )?;

    let task_id = TaskId::new(record.task_id.clone());
    let task = store
        .task_record(&task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let current_change_unit = store.current_change_unit(&task_id).map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })?;
    let resolution = UserJudgmentResolution {
        selected_option_id: request.selected_option_id.clone(),
        answer: request.answer.clone(),
        note: request.note.clone(),
        accepted_risks: request.accepted_risks.clone(),
        resolved_by_actor_kind: request.envelope.actor_kind,
    };
    user_judgment.status = UserJudgmentStatus::Resolved;
    user_judgment.resolution = Some(resolution.clone());
    user_judgment.resolved_at = Some(now.clone());

    let user_judgment_ref = state_ref(
        StateRecordKind::UserJudgment,
        request.user_judgment_id.as_str(),
        &request.envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let pending_refs = store
        .pending_user_judgment_refs(&task_id, planned_state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .filter(|record_ref| record_ref.record_id != request.user_judgment_id.as_str())
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let blocker_refs = store
        .active_blocker_refs(&task_id, planned_state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let task_ref = state_ref(
        StateRecordKind::Task,
        task_id.as_str(),
        &request.envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let change_unit_ref = current_change_unit.as_ref().map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            &request.envelope.project_id,
            Some(&task_id),
            Some(record.basis_state_version.unwrap_or(planned_state_version)),
        )
    });
    let next_actions = next_actions_for_state(&task_ref, change_unit_ref.as_ref());
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: current_change_unit.as_ref(),
        pending_user_judgment_refs: pending_refs,
        blocker_refs,
        active_write_authorization: None,
        options: SummaryOptions::mutation(),
    })?;
    let result = harness_types::RecordUserJudgmentResult {
        base: placeholder_base(),
        user_judgment_ref: user_judgment_ref.clone(),
        user_judgment,
        updated_refs: vec![user_judgment_ref],
        state,
        next_actions: next_actions.clone(),
    };
    let sensitive_action_scope_json = request
        .answer
        .sensitive_action_scope
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let storage_mutations = vec![CoreStorageMutation::ResolveUserJudgment(
        UserJudgmentResolutionUpdate {
            judgment_id: request.user_judgment_id.as_str().to_owned(),
            status: storage_value(UserJudgmentStatus::Resolved)?,
            resolution_json: serde_json::to_string(&resolution)?,
            sensitive_action_scope_json,
            resolved_at: now,
        },
    )];
    let event_payload = object_from_value(json!({
        "task_id": task_id,
        "change_unit_id": record.change_unit_id,
        "judgment_id": request.user_judgment_id,
        "judgment_kind": request.judgment_kind,
        "selected_option_id": request.selected_option_id
    }))?;

    Ok(MethodPlan {
        task_id,
        change_unit_id: record.change_unit_id.map(ChangeUnitId::new),
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions,
    })
}

struct UserJudgmentRequestValidation<'a> {
    dry_run: bool,
    state_version: Option<u64>,
    question: &'a str,
    options: &'a [UserJudgmentOption],
    context: &'a UserJudgmentContext,
    affected_refs: &'a [StateRecordRef],
    project_id: &'a ProjectId,
    task_id: &'a TaskId,
    expires_at: Option<&'a str>,
    current_timestamp: String,
}

fn validate_user_judgment_request_fields(
    input: UserJudgmentRequestValidation<'_>,
) -> Result<(), PlanError> {
    if input.question.trim().is_empty() {
        return validation_plan_error(
            input.dry_run,
            input.state_version,
            "question",
            "question must not be empty",
        );
    }
    if input.options.is_empty() {
        return validation_plan_error(
            input.dry_run,
            input.state_version,
            "options",
            "options must include at least one judgment option",
        );
    }
    let mut option_ids = BTreeSet::new();
    for option in input.options {
        if option.option_id.as_str().trim().is_empty() {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "options.option_id",
                "option_id must not be empty",
            );
        }
        if !option_ids.insert(option.option_id.as_str()) {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "options.option_id",
                "option_id values must be unique within the judgment",
            );
        }
        if option.label.trim().is_empty() {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "options.label",
                "option label must not be empty",
            );
        }
    }
    if input.context.summary.trim().is_empty() {
        return validation_plan_error(
            input.dry_run,
            input.state_version,
            "context.summary",
            "context.summary must not be empty",
        );
    }
    for affected_ref in input.affected_refs {
        if affected_ref.project_id != *input.project_id {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "affected_refs.project_id",
                "affected_refs must belong to the request project",
            );
        }
        if affected_ref
            .task_id
            .as_ref()
            .is_some_and(|ref_task_id| ref_task_id != input.task_id)
        {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "affected_refs.task_id",
                "task-scoped affected_refs must belong to the request Task",
            );
        }
    }
    if let Some(expires_at) = input.expires_at {
        if expires_at.trim().is_empty() {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "expires_at",
                "expires_at must be null or a non-empty timestamp string",
            );
        }
        if expires_at <= input.current_timestamp.as_str() {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "expires_at",
                "expires_at must be in the future for a pending judgment request",
            );
        }
    }
    Ok(())
}

fn validate_answer_payload(
    dry_run: bool,
    state_version: Option<u64>,
    judgment_kind: JudgmentKind,
    answer: &RecordUserJudgmentPayload,
) -> Result<(), PlanError> {
    if populated_answer_branch_count(answer) != 1 {
        return validation_plan_error(
            dry_run,
            state_version,
            "answer",
            "answer must populate exactly one decision-specific payload branch",
        );
    }
    if !answer_branch_matches_kind(judgment_kind, answer) {
        return validation_plan_error(
            dry_run,
            state_version,
            "answer",
            "answer payload branch must match the pending judgment_kind",
        );
    }
    Ok(())
}

fn populated_answer_branch_count(answer: &RecordUserJudgmentPayload) -> usize {
    usize::from(answer.product_decision.is_some())
        + usize::from(answer.technical_decision.is_some())
        + usize::from(answer.scope_decision.is_some())
        + usize::from(answer.sensitive_action_scope.is_some())
        + usize::from(answer.final_acceptance.is_some())
        + usize::from(answer.residual_risk_acceptance.is_some())
        + usize::from(answer.cancellation.is_some())
}

fn answer_branch_matches_kind(
    judgment_kind: JudgmentKind,
    answer: &RecordUserJudgmentPayload,
) -> bool {
    match judgment_kind {
        JudgmentKind::ProductDecision => answer.product_decision.is_some(),
        JudgmentKind::TechnicalDecision => answer.technical_decision.is_some(),
        JudgmentKind::ScopeDecision => answer.scope_decision.is_some(),
        JudgmentKind::SensitiveApproval => answer.sensitive_action_scope.is_some(),
        JudgmentKind::FinalAcceptance => answer.final_acceptance.is_some(),
        JudgmentKind::ResidualRiskAcceptance => answer.residual_risk_acceptance.is_some(),
        JudgmentKind::Cancellation => answer.cancellation.is_some(),
    }
}

fn user_judgment_from_record(record: &UserJudgmentRecord) -> CoreResult<UserJudgment> {
    let request = parse_json_object(&record.request_json);
    Ok(UserJudgment {
        judgment_id: harness_types::UserJudgmentId::new(record.judgment_id.clone()),
        project_id: ProjectId::new(record.project_id.clone()),
        task_id: TaskId::new(record.task_id.clone()),
        change_unit_id: record.change_unit_id.clone().map(ChangeUnitId::new),
        judgment_kind: parse_storage_value("user_judgments.judgment_kind", &record.judgment_kind)?,
        status: parse_storage_value("user_judgments.status", &record.status)?,
        presentation: request_member(
            "user_judgments.request_json.presentation",
            &request,
            "presentation",
        )?,
        question: string_member(&request, "question").ok_or_else(|| {
            CorePipelineError::InvalidDispatch {
                detail: "user_judgments.request_json.question missing".to_owned(),
            }
        })?,
        options: parse_json_text("user_judgments.options_json", &record.options_json)?,
        context: parse_json_text("user_judgments.context_json", &record.context_json)?,
        affected_refs: parse_json_text(
            "user_judgments.affected_refs_json",
            &record.affected_refs_json,
        )?,
        required_for: request_member(
            "user_judgments.request_json.required_for",
            &request,
            "required_for",
        )?,
        resolution: record
            .resolution_json
            .as_deref()
            .map(|text| parse_json_text("user_judgments.resolution_json", text))
            .transpose()?,
        expires_at: request
            .get("expires_at")
            .and_then(Value::as_str)
            .map(str::to_owned),
        created_at: record.requested_at.clone(),
        resolved_at: record.resolved_at.clone(),
    })
}

fn request_member<T>(field: &'static str, object: &JsonObject, key: &str) -> CoreResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let value = object
        .get(key)
        .cloned()
        .ok_or_else(|| CorePipelineError::InvalidDispatch {
            detail: format!("{field} missing"),
        })?;
    serde_json::from_value(value).map_err(CorePipelineError::from)
}

fn parse_storage_value<T>(field: &'static str, value: &str) -> CoreResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(Value::String(value.to_owned())).map_err(|error| {
        CorePipelineError::InvalidDispatch {
            detail: format!("{field} contains unsupported value {value}: {error}"),
        }
    })
}

fn parse_json_text<T>(field: &'static str, text: &str) -> CoreResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(text).map_err(|error| CorePipelineError::InvalidDispatch {
        detail: format!("{field} is not valid stored JSON: {error}"),
    })
}

fn storage_value<T>(value: T) -> CoreResult<String>
where
    T: serde::Serialize,
{
    match serde_json::to_value(value)? {
        Value::String(value) => Ok(value),
        _ => Err(CorePipelineError::InvalidDispatch {
            detail: "storage value must serialize to a string".to_owned(),
        }),
    }
}

fn validation_plan_error(
    dry_run: bool,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> Result<(), PlanError> {
    let response =
        validation_rejected(dry_run, state_version, field, message).map_err(PlanError::Core)?;
    Err(PlanError::Response(Box::new(response)))
}

fn decision_rejected_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    message: &'static str,
) -> PipelineResponse {
    rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::DecisionUnresolved,
            message,
            false,
            None,
        )],
    )
    .expect("rejected response serialization should succeed")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredScope {
    goal_summary: Option<String>,
    scope_summary: Option<String>,
    non_goals: Vec<String>,
    acceptance_criteria: Vec<String>,
    autonomy_boundary: Option<String>,
    baseline_ref: Option<String>,
}

impl StoredScope {
    fn from_task(task: &TaskRecord) -> Self {
        let shaping = parse_json_object(&task.shaping_summary_json);
        let autonomy = parse_json_object(&task.autonomy_boundary_json);
        Self {
            goal_summary: string_member(&shaping, "goal_summary").or_else(|| task.summary.clone()),
            scope_summary: string_member(&shaping, "scope_summary"),
            non_goals: string_array_member(&shaping, "non_goals"),
            acceptance_criteria: string_array_member(&shaping, "acceptance_criteria"),
            autonomy_boundary: string_member(&autonomy, "autonomy_boundary"),
            baseline_ref: string_member(&shaping, "baseline_ref"),
        }
    }

    fn apply_request(&self, request: &UpdateScopeRequest) -> Self {
        Self {
            goal_summary: request
                .goal_summary
                .clone()
                .or_else(|| self.goal_summary.clone()),
            scope_summary: request
                .scope_boundary
                .clone()
                .or_else(|| self.scope_summary.clone()),
            non_goals: request
                .non_goals
                .clone()
                .unwrap_or_else(|| self.non_goals.clone()),
            acceptance_criteria: request
                .acceptance_criteria
                .clone()
                .unwrap_or_else(|| self.acceptance_criteria.clone()),
            autonomy_boundary: request
                .autonomy_boundary
                .clone()
                .or_else(|| self.autonomy_boundary.clone()),
            baseline_ref: request
                .baseline_ref
                .as_ref()
                .map(|value| value.as_str().to_owned())
                .or_else(|| self.baseline_ref.clone()),
        }
    }

    fn to_json(&self) -> Value {
        task_shaping_json(
            self.goal_summary.clone(),
            self.scope_summary.clone(),
            self.non_goals.clone(),
            self.acceptance_criteria.clone(),
            self.baseline_ref.clone(),
            self.autonomy_boundary.clone(),
            None,
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct SummaryOptions {
    pending_user_judgments: bool,
    blockers: bool,
    write_authority: bool,
}

impl SummaryOptions {
    fn mutation() -> Self {
        Self {
            pending_user_judgments: true,
            blockers: true,
            write_authority: false,
        }
    }

    fn prepare_write() -> Self {
        Self {
            pending_user_judgments: true,
            blockers: true,
            write_authority: true,
        }
    }

    fn status(include: &StatusInclude) -> Self {
        Self {
            pending_user_judgments: include.pending_user_judgments,
            blockers: true,
            write_authority: include.write_authority,
        }
    }
}

struct SummaryBuild<'a> {
    project_id: &'a ProjectId,
    state_version: u64,
    task: &'a TaskRecord,
    current_change_unit: Option<&'a ChangeUnitRecord>,
    pending_user_judgment_refs: Vec<StateRecordRef>,
    blocker_refs: Vec<StateRecordRef>,
    active_write_authorization: Option<&'a WriteAuthorizationRecord>,
    options: SummaryOptions,
}

fn status_result_fields(
    store: &CoreProjectStore,
    project_id: &ProjectId,
    state_version: u64,
    task: Option<&TaskRecord>,
    include: &StatusInclude,
) -> CoreResult<JsonObject> {
    let active_task = if include.task {
        match task {
            Some(task) => {
                let task_id = TaskId::new(task.task_id.clone());
                let current_change_unit = store.current_change_unit(&task_id)?;
                let pending_refs = if include.pending_user_judgments {
                    stored_refs_to_state_refs(
                        store.pending_user_judgment_refs(&task_id, state_version)?,
                    )
                } else {
                    Vec::new()
                };
                let blocker_refs =
                    stored_refs_to_state_refs(store.active_blocker_refs(&task_id, state_version)?);
                let active_write_auths = if include.write_authority {
                    store.active_write_authorizations(&task_id)?
                } else {
                    Vec::new()
                };
                Some(build_state_summary(SummaryBuild {
                    project_id,
                    state_version,
                    task,
                    current_change_unit: current_change_unit.as_ref(),
                    pending_user_judgment_refs: pending_refs,
                    blocker_refs,
                    active_write_authorization: active_write_auths.first(),
                    options: SummaryOptions::status(include),
                })?)
            }
            None => None,
        }
    } else {
        None
    };

    let pending_user_judgments = active_task
        .as_ref()
        .map(|state| state.pending_user_judgment_refs.clone())
        .unwrap_or_default();
    let blocker_refs = active_task
        .as_ref()
        .map(|state| state.blocker_refs.clone())
        .unwrap_or_default();
    let next_actions = active_task
        .as_ref()
        .map(|state| {
            if let Some(task_ref) = &state.task_ref {
                next_actions_for_state(task_ref, state.active_change_unit_ref.as_ref())
            } else {
                Vec::new()
            }
        })
        .unwrap_or_default();
    let result = harness_types::StatusResult {
        base: placeholder_base(),
        active_task,
        status_summary: if task.is_some() {
            "Current Task state is available.".to_owned()
        } else {
            "No current Task is selected.".to_owned()
        },
        next_actions,
        pending_user_judgments,
        blocker_refs,
        close_state: StatusCloseState::None,
        close_blockers: Vec::new(),
        guarantee_display: None,
    };
    strip_base(serde_json::to_value(result)?)
}

fn build_state_summary(input: SummaryBuild<'_>) -> CoreResult<harness_types::StateSummary> {
    let SummaryBuild {
        project_id,
        state_version,
        task,
        current_change_unit,
        pending_user_judgment_refs,
        blocker_refs,
        active_write_authorization,
        options,
    } = input;
    let task_id = TaskId::new(task.task_id.clone());
    let task_ref = state_ref(
        StateRecordKind::Task,
        &task.task_id,
        project_id,
        Some(&task_id),
        Some(state_version),
    );
    let active_change_unit_ref = current_change_unit.map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            project_id,
            Some(&task_id),
            Some(record.basis_state_version.unwrap_or(state_version)),
        )
    });
    let scope = StoredScope::from_task(task);
    let change_unit_scope = current_change_unit.and_then(|record| {
        string_member(
            &parse_json_object(&record.scope_summary_json),
            "scope_summary",
        )
    });
    let write_authority_summary = if options.write_authority {
        active_write_authorization.map(|record| WriteAuthoritySummary {
            status: WriteAuthorizationStatus::Active,
            write_authorization_ref: Some(write_authorization_ref(record, state_version)),
            basis_state_version: Some(record.basis_state_version),
            intended_paths: string_array_member(
                &parse_json_object(&record.attempt_scope_json),
                "intended_paths",
            ),
            guarantee_display: None,
        })
    } else {
        None
    };

    Ok(harness_types::StateSummary {
        project_id: project_id.clone(),
        state_version,
        task_ref: Some(task_ref),
        mode: parse_task_mode(&task.mode)?,
        lifecycle: Some(TaskLifecycleState {
            lifecycle_phase: parse_lifecycle_phase(&task.lifecycle_phase)?,
            close_reason: parse_close_reason(&task.close_summary_json),
            result: parse_task_result(task.result.as_deref().unwrap_or("none"))?,
            closed_at: task.closed_at.clone(),
        }),
        goal_summary: scope.goal_summary,
        scope_summary: change_unit_scope.or(scope.scope_summary),
        non_goals: scope.non_goals,
        acceptance_criteria: scope.acceptance_criteria,
        autonomy_boundary: scope.autonomy_boundary,
        active_change_unit_ref,
        baseline_ref: scope.baseline_ref.map(BaselineRef::new),
        shaping_readiness: None,
        pending_user_judgment_refs: if options.pending_user_judgments {
            pending_user_judgment_refs
        } else {
            Vec::new()
        },
        blocker_refs: if options.blockers {
            blocker_refs
        } else {
            Vec::new()
        },
        write_authority_summary,
        evidence_summary: None,
        close_state: None,
        close_blockers: Vec::new(),
        guarantee_display: None,
    })
}

fn change_unit_insert(
    request: &UpdateScopeRequest,
    change_unit_id: &ChangeUnitId,
) -> CoreResult<ChangeUnitInsert> {
    let fields = &request.change_unit.fields;
    let scope_summary = string_member(fields, "scope_summary")
        .or_else(|| request.scope_boundary.clone())
        .unwrap_or_else(|| "Current Change Unit".to_owned());
    let affected_areas = string_array_member(fields, "affected_areas");
    let affected_paths = string_array_member(fields, "affected_paths");
    let constraints = string_array_member(fields, "constraints");
    Ok(ChangeUnitInsert {
        change_unit_id: change_unit_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        scope_summary_json: serde_json::to_string(&json!({
            "scope_summary": scope_summary,
            "affected_areas": affected_areas,
            "constraints": constraints
        }))?,
        bounded_paths_json: serde_json::to_string(&affected_paths)?,
        write_basis_json: serde_json::to_string(&json!({
            "baseline_ref": request.baseline_ref
        }))?,
        close_basis_json: "{}".to_owned(),
        lifecycle_json: "{}".to_owned(),
    })
}

fn synthetic_change_unit_record(
    project_id: &ProjectId,
    task_id: &TaskId,
    insert: &ChangeUnitInsert,
    planned_state_version: u64,
) -> ChangeUnitRecord {
    ChangeUnitRecord {
        project_id: project_id.as_str().to_owned(),
        change_unit_id: insert.change_unit_id.clone(),
        task_id: task_id.as_str().to_owned(),
        status: "active".to_owned(),
        is_current: true,
        basis_state_version: Some(planned_state_version),
        scope_summary_json: insert.scope_summary_json.clone(),
        bounded_paths_json: insert.bounded_paths_json.clone(),
        write_basis_json: insert.write_basis_json.clone(),
        close_basis_json: insert.close_basis_json.clone(),
        lifecycle_json: insert.lifecycle_json.clone(),
    }
}

fn task_shaping_json(
    goal_summary: Option<String>,
    scope_summary: Option<String>,
    non_goals: Vec<String>,
    acceptance_criteria: Vec<String>,
    baseline_ref: Option<String>,
    autonomy_boundary: Option<String>,
    initial_context_refs: Option<Value>,
) -> Value {
    json!({
        "goal_summary": goal_summary,
        "scope_summary": scope_summary,
        "non_goals": non_goals,
        "acceptance_criteria": acceptance_criteria,
        "baseline_ref": baseline_ref,
        "autonomy_boundary": autonomy_boundary,
        "initial_context_refs": initial_context_refs.unwrap_or(Value::Array(Vec::new()))
    })
}

fn next_actions_for_state(
    task_ref: &StateRecordRef,
    change_unit_ref: Option<&StateRecordRef>,
) -> Vec<NextActionSummary> {
    match change_unit_ref {
        Some(change_unit_ref) => vec![NextActionSummary {
            action_kind: NextActionKind::PrepareWrite,
            owner_method: Some(MethodName::PrepareWrite),
            label: "Check the current change against current scope.".to_owned(),
            blocking_question: None,
            required_refs: vec![task_ref.clone(), change_unit_ref.clone()],
        }],
        None => vec![NextActionSummary {
            action_kind: NextActionKind::UpdateScope,
            owner_method: Some(MethodName::UpdateScope),
            label: "Create the first currently applied Change Unit before write checking."
                .to_owned(),
            blocking_question: None,
            required_refs: vec![task_ref.clone()],
        }],
    }
}

fn dry_run_summary(
    target_kind: &str,
    action: &str,
    description: &str,
    next_actions: Vec<NextActionSummary>,
) -> DryRunSummary {
    DryRunSummary {
        planned_effects: vec![PlannedEffect {
            target_kind: target_kind.to_owned(),
            action: action.to_owned(),
            description: description.to_owned(),
        }],
        would_blockers: Vec::new(),
        would_errors: Vec::new(),
        next_actions,
        diagnostics: Vec::new(),
    }
}

fn state_ref(
    record_kind: StateRecordKind,
    record_id: &str,
    project_id: &ProjectId,
    task_id: Option<&TaskId>,
    state_version: Option<u64>,
) -> StateRecordRef {
    StateRecordRef {
        record_kind,
        record_id: RecordId::new(record_id),
        project_id: project_id.clone(),
        task_id: task_id.cloned(),
        state_version,
    }
}

fn write_authorization_ref(
    record: &WriteAuthorizationRecord,
    state_version: u64,
) -> StateRecordRef {
    state_ref(
        StateRecordKind::WriteAuthorization,
        &record.write_authorization_id,
        &ProjectId::new(record.project_id.clone()),
        Some(&TaskId::new(record.task_id.clone())),
        Some(state_version),
    )
}

fn state_ref_from_stored(record: StoredRecordRef) -> StateRecordRef {
    let kind = match record.record_kind.as_str() {
        "user_judgment" => StateRecordKind::UserJudgment,
        "blocker" => StateRecordKind::Blocker,
        "write_authorization" => StateRecordKind::WriteAuthorization,
        "change_unit" => StateRecordKind::ChangeUnit,
        "task" => StateRecordKind::Task,
        _ => StateRecordKind::ProjectState,
    };
    StateRecordRef {
        record_kind: kind,
        record_id: RecordId::new(record.record_id),
        project_id: ProjectId::new(record.project_id),
        task_id: record.task_id.map(TaskId::new),
        state_version: record.state_version,
    }
}

fn stored_refs_to_state_refs(records: Vec<StoredRecordRef>) -> Vec<StateRecordRef> {
    records.into_iter().map(state_ref_from_stored).collect()
}

fn strip_base(value: Value) -> CoreResult<JsonObject> {
    let mut object = object_from_value(value)?;
    object.remove("base");
    Ok(object)
}

fn object_from_value(value: Value) -> CoreResult<JsonObject> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(CorePipelineError::InvalidDispatch {
            detail: "expected JSON object".to_owned(),
        }),
    }
}

fn placeholder_base() -> ToolResultBase {
    method_result_base(EffectKind::NoEffect, false, None, Vec::new())
}

fn validation_rejected(
    dry_run: bool,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> CoreResult<PipelineResponse> {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    rejected_pipeline_response(
        dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::ValidationFailed,
            message,
            false,
            Some(details),
        )],
    )
}

fn rejected_pipeline_response(
    dry_run: bool,
    state_version: Option<u64>,
    errors: Vec<harness_types::ToolError>,
) -> CoreResult<PipelineResponse> {
    let response = rejected_response(dry_run, state_version, errors);
    let response_value = serde_json::to_value(response)?;
    let response_json = serde_json::to_string(&response_value)?;
    Ok(PipelineResponse {
        response_json,
        response_value,
        verified_surface: None,
        resolved_task_id: None,
        replayed: false,
    })
}

fn infallible_rejected_pipeline_response(
    dry_run: bool,
    state_version: Option<u64>,
    errors: Vec<harness_types::ToolError>,
) -> PipelineResponse {
    rejected_pipeline_response(dry_run, state_version, errors)
        .expect("rejected response serialization should succeed")
}

fn store_error_response(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
    error: StoreError,
) -> PipelineResponse {
    rejected_pipeline_response(
        envelope.dry_run,
        Some(project_state.state_version),
        vec![store_unavailable_error(error)],
    )
    .expect("rejected response serialization should succeed")
}

fn no_active_task_response(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
) -> PipelineResponse {
    rejected_pipeline_response(
        envelope.dry_run,
        Some(project_state.state_version),
        vec![tool_error(
            ErrorCode::NoActiveTask,
            "a Task is required but no addressed or current Task is available",
            false,
            None,
        )],
    )
    .expect("rejected response serialization should succeed")
}

fn store_unavailable_error(error: StoreError) -> harness_types::ToolError {
    tool_error(
        match error {
            StoreError::NotFound { .. } => ErrorCode::LocalAccessMismatch,
            StoreError::InvalidInput { .. }
            | StoreError::Io(_)
            | StoreError::Sqlite(_)
            | StoreError::MigrationConflict { .. }
            | StoreError::SchemaInvariant { .. } => ErrorCode::McpUnavailable,
        },
        "Core storage or project binding is unavailable",
        true,
        None,
    )
}

fn resolve_requested_mode(requested_mode: RequestedMode) -> TaskMode {
    match requested_mode {
        RequestedMode::Advisor => TaskMode::Advisor,
        RequestedMode::Direct => TaskMode::Direct,
        RequestedMode::Work | RequestedMode::Auto => TaskMode::Work,
    }
}

fn task_mode_storage(mode: TaskMode) -> &'static str {
    match mode {
        TaskMode::Advisor => "advisor",
        TaskMode::Direct => "direct",
        TaskMode::Work => "work",
    }
}

fn parse_task_mode(value: &str) -> CoreResult<Option<TaskMode>> {
    match value {
        "advisor" => Ok(Some(TaskMode::Advisor)),
        "direct" => Ok(Some(TaskMode::Direct)),
        "work" => Ok(Some(TaskMode::Work)),
        _ => invalid_storage(format!("unsupported Task mode {value}")),
    }
}

fn parse_lifecycle_phase(value: &str) -> CoreResult<TaskLifecyclePhase> {
    match value {
        "shaping" => Ok(TaskLifecyclePhase::Shaping),
        "ready" => Ok(TaskLifecyclePhase::Ready),
        "executing" => Ok(TaskLifecyclePhase::Executing),
        "waiting_user" => Ok(TaskLifecyclePhase::WaitingUser),
        "blocked" => Ok(TaskLifecyclePhase::Blocked),
        "completed" => Ok(TaskLifecyclePhase::Completed),
        "cancelled" => Ok(TaskLifecyclePhase::Cancelled),
        "superseded" => Ok(TaskLifecyclePhase::Superseded),
        _ => invalid_storage(format!("unsupported Task lifecycle_phase {value}")),
    }
}

fn parse_task_result(value: &str) -> CoreResult<TaskResult> {
    match value {
        "none" => Ok(TaskResult::None),
        "advice_only" => Ok(TaskResult::AdviceOnly),
        "completed" => Ok(TaskResult::Completed),
        "cancelled" => Ok(TaskResult::Cancelled),
        "superseded" => Ok(TaskResult::Superseded),
        _ => invalid_storage(format!("unsupported Task result {value}")),
    }
}

fn parse_close_reason(close_summary_json: &str) -> CloseReason {
    let value = parse_json_object(close_summary_json);
    match string_member(&value, "close_reason").as_deref() {
        Some("completed_self_checked") => CloseReason::CompletedSelfChecked,
        Some("completed_with_risk_accepted") => CloseReason::CompletedWithRiskAccepted,
        Some("cancelled") => CloseReason::Cancelled,
        Some("superseded") => CloseReason::Superseded,
        _ => CloseReason::None,
    }
}

fn invalid_storage<T>(detail: String) -> CoreResult<T> {
    Err(CorePipelineError::InvalidDispatch { detail })
}

fn parse_json_object(text: &str) -> JsonObject {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|value| match value {
            Value::Object(object) => Some(object),
            _ => None,
        })
        .unwrap_or_default()
}

fn string_member(object: &JsonObject, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn string_array_member(object: &JsonObject, key: &str) -> Vec<String> {
    object
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn bool_member(object: &JsonObject, key: &str) -> bool {
    object.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn json_array_nonempty_member(object: &JsonObject, key: &str) -> bool {
    object
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

#[cfg(test)]
mod tests {
    use std::{error::Error, fs, path::PathBuf};

    use harness_store::{
        bootstrap::{
            initialize_runtime_home, register_project, register_surface, ProjectRegistration,
            SurfaceRegistration, ACTIVE_PROJECT_STATUS,
        },
        core_pipeline::{CoreProjectStore, StorageEffectCounts},
        sqlite::open_project_state_database,
    };
    use harness_test_support::TempRuntimeHome;
    use harness_types::{
        ActorKind, ChangeUnitUpdate, IdempotencyKey, InitialScope, RequestId, ScopeUpdate,
        SequenceDurableIdGenerator, SurfaceId,
    };
    use serde_json::{json, Map, Value};

    use super::*;

    const PROJECT_ID: &str = "project_methods";
    const SURFACE_ID: &str = "surface_methods";
    const SURFACE_INSTANCE_ID: &str = "surface_instance_methods";

    struct MethodHarness {
        _runtime_home: TempRuntimeHome,
        runtime_home_path: PathBuf,
        service: CoreService,
    }

    impl MethodHarness {
        fn new() -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new("core-methods")?;
            let repo_root = runtime_home.path().join("repo");
            fs::create_dir_all(&repo_root)?;
            initialize_runtime_home(runtime_home.path(), "runtime_home_methods", "{}")?;
            register_project(
                runtime_home.path(),
                ProjectRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    repo_root,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            register_surface(
                runtime_home.path(),
                SurfaceRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    surface_id: SURFACE_ID.to_owned(),
                    surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
                    surface_kind: "local_test".to_owned(),
                    display_name: Some("Method Test Surface".to_owned()),
                    capability_profile_json: json!({
                        "access_class": "write_authorization",
                        "supported_access_classes": ["write_authorization"]
                    })
                    .to_string(),
                    local_access_json: json!({
                        "access_class": "core_mutation",
                        "authorized_access_classes": [
                            "read_status",
                            "core_mutation",
                            "write_authorization",
                            "run_recording",
                            "artifact_registration",
                            "artifact_read"
                        ],
                        "verification_basis": "method_test_registration"
                    })
                    .to_string(),
                    metadata_json: "{}".to_owned(),
                },
            )?;

            let runtime_home_path = runtime_home.path().to_path_buf();
            let service = CoreService::new(&runtime_home_path);
            Ok(Self {
                _runtime_home: runtime_home,
                runtime_home_path,
                service,
            })
        }

        fn counts(&self) -> Result<StorageEffectCounts, Box<dyn Error>> {
            let store =
                CoreProjectStore::open(&self.runtime_home_path, &ProjectId::new(PROJECT_ID))?;
            Ok(store.effect_counts()?)
        }

        fn conn(&self) -> Result<rusqlite::Connection, Box<dyn Error>> {
            Ok(open_project_state_database(
                self.runtime_home_path
                    .join("projects")
                    .join(PROJECT_ID)
                    .join("state.sqlite"),
            )?)
        }
    }

    #[test]
    fn reused_request_id_does_not_collide_for_core_generated_records() -> Result<(), Box<dyn Error>>
    {
        let harness = MethodHarness::new()?;
        let request_id = "req_reused_for_generated_ids";

        let first_intake = harness.service.intake(
            intake_request(
                request_id,
                "idem_reused_intake_1",
                false,
                Some(0),
                RequestedMode::Work,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let first_task_id = response_record_id(&first_intake.response_value, "task_ref");
        let first_event_id = response_event_id(&first_intake.response_value);

        let second_intake = harness.service.intake(
            intake_request(
                request_id,
                "idem_reused_intake_2",
                false,
                Some(1),
                RequestedMode::Work,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let second_task_id = response_record_id(&second_intake.response_value, "task_ref");
        let second_event_id = response_event_id(&second_intake.response_value);
        assert_ne!(first_task_id, second_task_id);
        assert_ne!(first_event_id, second_event_id);

        let first_scope = harness.service.update_scope(
            update_scope_request(
                request_id,
                "idem_reused_scope_1",
                false,
                Some(2),
                &second_task_id,
                ChangeUnitOperation::CreateCurrent,
                "First reused request scope.",
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let first_change_unit_id =
            response_record_id(&first_scope.response_value, "change_unit_ref");
        let first_scope_event_id = response_event_id(&first_scope.response_value);

        let second_scope = harness.service.update_scope(
            update_scope_request(
                request_id,
                "idem_reused_scope_2",
                false,
                Some(3),
                &second_task_id,
                ChangeUnitOperation::ReplaceCurrent,
                "Second reused request scope.",
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let second_change_unit_id =
            response_record_id(&second_scope.response_value, "change_unit_ref");
        let second_scope_event_id = response_event_id(&second_scope.response_value);
        assert_ne!(first_change_unit_id, second_change_unit_id);
        assert_ne!(first_scope_event_id, second_scope_event_id);

        let first_write = harness.service.prepare_write(
            prepare_write_request(
                request_id,
                "idem_reused_write_1",
                Some(4),
                Some(&second_task_id),
                Some(&second_change_unit_id),
            ),
            invocation(AccessClass::WriteAuthorization),
        )?;
        let first_write_id =
            response_record_id(&first_write.response_value, "write_authorization_ref");
        let first_write_event_id = response_event_id(&first_write.response_value);

        let second_write = harness.service.prepare_write(
            prepare_write_request(
                request_id,
                "idem_reused_write_2",
                Some(5),
                Some(&second_task_id),
                Some(&second_change_unit_id),
            ),
            invocation(AccessClass::WriteAuthorization),
        )?;
        let second_write_id =
            response_record_id(&second_write.response_value, "write_authorization_ref");
        let second_write_event_id = response_event_id(&second_write.response_value);
        assert_ne!(first_write_id, second_write_id);
        assert_ne!(first_write_event_id, second_write_event_id);

        let first_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                request_id,
                "idem_reused_judgment_1",
                false,
                Some(6),
                &second_task_id,
                Some(&second_change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let first_judgment_id =
            response_record_id(&first_judgment.response_value, "user_judgment_ref");
        let first_judgment_event_id = response_event_id(&first_judgment.response_value);

        let second_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                request_id,
                "idem_reused_judgment_2",
                false,
                Some(7),
                &second_task_id,
                Some(&second_change_unit_id),
                JudgmentKind::TechnicalDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let second_judgment_id =
            response_record_id(&second_judgment.response_value, "user_judgment_ref");
        let second_judgment_event_id = response_event_id(&second_judgment.response_value);
        assert_ne!(first_judgment_id, second_judgment_id);
        assert_ne!(first_judgment_event_id, second_judgment_event_id);

        Ok(())
    }

    #[test]
    fn reused_request_id_stage_artifact_returns_distinct_handles() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_stage_artifact_capability(&harness)?;
        let (task_id, _) = create_task_with_change_unit(&harness, "stage_reused_request")?;

        let first = harness.service.stage_artifact(
            stage_artifact_request("req_stage_reused", None, false, None, &task_id),
            invocation(AccessClass::ArtifactRegistration),
        )?;
        let second = harness.service.stage_artifact(
            stage_artifact_request("req_stage_reused", None, false, None, &task_id),
            invocation(AccessClass::ArtifactRegistration),
        )?;

        let first_handle = first.response_value["staged_artifact_handle"]["handle_id"]
            .as_str()
            .expect("first handle should be present");
        let second_handle = second.response_value["staged_artifact_handle"]["handle_id"]
            .as_str()
            .expect("second handle should be present");
        assert_ne!(first_handle, second_handle);
        assert_eq!(harness.counts()?.artifact_staging, 2);
        Ok(())
    }

    #[test]
    fn idempotent_replay_returns_original_generated_ids() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let request = intake_request(
            "req_replay_generated_ids",
            "idem_replay_generated_ids",
            false,
            Some(0),
            RequestedMode::Work,
        );

        let first = harness
            .service
            .intake(request.clone(), invocation(AccessClass::CoreMutation))?;
        let second = harness
            .service
            .intake(request, invocation(AccessClass::CoreMutation))?;

        assert!(second.replayed);
        assert_eq!(
            response_record_id(&first.response_value, "task_ref"),
            response_record_id(&second.response_value, "task_ref")
        );
        assert_eq!(
            response_event_id(&first.response_value),
            response_event_id(&second.response_value)
        );
        assert_eq!(harness.counts()?.tasks, 1);
        assert_eq!(harness.counts()?.task_events, 1);
        Ok(())
    }

    #[test]
    fn deterministic_generated_id_collision_retries_bounded_candidates(
    ) -> Result<(), Box<dyn Error>> {
        let mut harness = MethodHarness::new()?;
        insert_superseding_task(&harness, "task_collision")?;
        harness.service = CoreService::with_id_generator(
            &harness.runtime_home_path,
            SequenceDurableIdGenerator::new(["collision", "fresh", "event"]),
        );

        let response = harness.service.intake(
            intake_request(
                "req_collision_retry",
                "idem_collision_retry",
                false,
                Some(0),
                RequestedMode::Work,
            ),
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(
            response_record_id(&response.response_value, "task_ref"),
            "task_fresh"
        );
        assert_eq!(response_event_id(&response.response_value), "evt_event");
        assert_eq!(harness.counts()?.tasks, 2);
        Ok(())
    }

    fn response_record_id(response_value: &Value, field: &str) -> String {
        response_value[field]["record_id"]
            .as_str()
            .expect("record_id should be present")
            .to_owned()
    }

    fn response_event_id(response_value: &Value) -> String {
        response_value["base"]["events"][0]["event_id"]
            .as_str()
            .expect("event_id should be present")
            .to_owned()
    }

    #[test]
    fn status_is_read_only_including_dry_run() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let before = harness.counts()?;

        let response = harness.service.status(
            StatusRequest {
                envelope: envelope("req_status", None, false, None, None),
                include: status_include(),
            },
            invocation(AccessClass::ReadStatus),
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
        assert_eq!(response.response_value["base"]["dry_run"], false);
        assert_eq!(response.response_value["base"]["events"], json!([]));
        assert_eq!(harness.counts()?, before);

        let dry_run = harness.service.status(
            StatusRequest {
                envelope: envelope(
                    "req_status_dry",
                    Some("idem_status_dry"),
                    true,
                    Some(0),
                    None,
                ),
                include: status_include(),
            },
            invocation(AccessClass::ReadStatus),
        )?;

        assert_eq!(dry_run.response_value["base"]["response_kind"], "result");
        assert_eq!(dry_run.response_value["base"]["effect_kind"], "read_only");
        assert_eq!(dry_run.response_value["base"]["dry_run"], true);
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn public_methods_use_same_verified_surface_context() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "verified_context")?;

        let status = harness.service.status(
            StatusRequest {
                envelope: envelope("req_verified_status", None, false, None, Some(&task_id)),
                include: status_include(),
            },
            invocation(AccessClass::ReadStatus),
        )?;
        assert_verified_surface(&status, AccessClass::ReadStatus);

        let intake = harness.service.intake(
            intake_request(
                "req_verified_intake",
                "idem_verified_intake",
                true,
                Some(2),
                RequestedMode::Work,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        assert_verified_surface(&intake, AccessClass::CoreMutation);

        let update_scope = harness.service.update_scope(
            update_scope_request(
                "req_verified_scope",
                "idem_verified_scope",
                true,
                Some(2),
                &task_id,
                ChangeUnitOperation::KeepCurrent,
                "Initial current scope.",
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        assert_verified_surface(&update_scope, AccessClass::CoreMutation);

        let mut prepare_write = prepare_write_request(
            "req_verified_prepare",
            "idem_verified_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        prepare_write.envelope.dry_run = true;
        let prepare_write = harness
            .service
            .prepare_write(prepare_write, invocation(AccessClass::WriteAuthorization))?;
        assert_verified_surface(&prepare_write, AccessClass::WriteAuthorization);

        let stage_artifact = harness.service.stage_artifact(
            stage_artifact_request(
                "req_verified_stage",
                Some("idem_verified_stage"),
                true,
                Some(2),
                &task_id,
            ),
            invocation(AccessClass::ArtifactRegistration),
        )?;
        assert_verified_surface(&stage_artifact, AccessClass::ArtifactRegistration);

        let record_run = harness.service.record_run(
            record_run_request(
                "req_verified_run",
                "idem_verified_run",
                true,
                Some(2),
                &task_id,
                &change_unit_id,
            ),
            invocation(AccessClass::RunRecording),
        )?;
        assert_verified_surface(&record_run, AccessClass::RunRecording);

        let request_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                "req_verified_judgment_preview",
                "idem_verified_judgment_preview",
                true,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        assert_verified_surface(&request_judgment, AccessClass::CoreMutation);

        let pending_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                "req_verified_judgment_pending",
                "idem_verified_judgment_pending",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let pending_judgment_id =
            response_record_id(&pending_judgment.response_value, "user_judgment_ref");
        let mut record_judgment = record_judgment_request(
            "req_verified_record_judgment",
            "idem_verified_record_judgment",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        );
        record_judgment.envelope.dry_run = true;
        let record_judgment = harness
            .service
            .record_user_judgment(record_judgment, invocation(AccessClass::CoreMutation))?;
        assert_verified_surface(&record_judgment, AccessClass::CoreMutation);

        let close_check = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: "req_verified_close",
                idempotency_key: None,
                dry_run: false,
                expected_state_version: None,
                task_id: &task_id,
                intent: CloseIntent::Check,
                close_reason: None,
                superseding_task_id: None,
            }),
            invocation(AccessClass::ReadStatus),
        )?;
        assert_verified_surface(&close_check, AccessClass::ReadStatus);

        Ok(())
    }

    #[test]
    fn intake_commits_once_and_replays_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let before = harness.counts()?;
        let request = intake_request(
            "req_intake",
            "idem_intake",
            false,
            Some(0),
            RequestedMode::Auto,
        );

        let first = harness
            .service
            .intake(request.clone(), invocation(AccessClass::CoreMutation))?;
        let after_first = harness.counts()?;

        assert_eq!(first.response_value["base"]["response_kind"], "result");
        assert_eq!(
            first.response_value["base"]["effect_kind"],
            "core_committed"
        );
        assert_eq!(first.response_value["base"]["state_version"], 1);
        assert_eq!(first.response_value["state"]["mode"], "work");
        assert_eq!(after_first.state_version, before.state_version + 1);
        assert_eq!(after_first.tasks, before.tasks + 1);
        assert_eq!(after_first.task_events, before.task_events + 1);
        assert_eq!(after_first.tool_invocations, before.tool_invocations + 1);

        let second = harness
            .service
            .intake(request, invocation(AccessClass::CoreMutation))?;
        assert!(second.replayed);
        assert_eq!(second.response_json, first.response_json);
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    #[test]
    fn intake_dry_run_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let before = harness.counts()?;
        let response = harness.service.intake(
            intake_request(
                "req_intake_dry",
                "idem_intake_dry",
                true,
                Some(0),
                RequestedMode::Work,
            ),
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
        assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn update_scope_commits_once_and_creates_one_current_change_unit() -> Result<(), Box<dyn Error>>
    {
        let harness = MethodHarness::new()?;
        let intake = harness.service.intake(
            intake_request(
                "req_scope_task",
                "idem_scope_task",
                false,
                Some(0),
                RequestedMode::Work,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("task ref should be present")
            .to_owned();
        let before = harness.counts()?;

        let response = harness.service.update_scope(
            update_scope_request(
                "req_scope_create",
                "idem_scope_create",
                false,
                Some(1),
                &task_id,
                ChangeUnitOperation::CreateCurrent,
                "Create current export scope.",
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["state_version"], 2);
        assert!(response.response_value["change_unit_ref"].is_object());
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.change_units, before.change_units + 1);
        assert_eq!(after.task_events, before.task_events + 1);
        assert_eq!(after.tool_invocations, before.tool_invocations + 1);
        assert_eq!(active_current_change_units(&harness, &task_id)?, 1);
        Ok(())
    }

    #[test]
    fn update_scope_replaces_current_and_marks_write_authorization_stale(
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let intake = harness.service.intake(
            intake_request(
                "req_replace_task",
                "idem_replace_task",
                false,
                Some(0),
                RequestedMode::Work,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("task ref should be present")
            .to_owned();
        let create = harness.service.update_scope(
            update_scope_request(
                "req_replace_create",
                "idem_replace_create",
                false,
                Some(1),
                &task_id,
                ChangeUnitOperation::CreateCurrent,
                "Initial current scope.",
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let change_unit_id = create.response_value["change_unit_ref"]["record_id"]
            .as_str()
            .expect("change unit ref should be present")
            .to_owned();
        insert_active_write_authorization(&harness, &task_id, &change_unit_id)?;
        let before = harness.counts()?;

        let response = harness.service.update_scope(
            update_scope_request(
                "req_replace_current",
                "idem_replace_current",
                false,
                Some(2),
                &task_id,
                ChangeUnitOperation::ReplaceCurrent,
                "Replacement current scope.",
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["base"]["state_version"], 3);
        assert_eq!(
            response.response_value["stale_write_authorization_refs"]
                .as_array()
                .expect("stale refs should be an array")
                .len(),
            1
        );
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.change_units, before.change_units + 1);
        assert_eq!(active_current_change_units(&harness, &task_id)?, 1);
        assert_eq!(write_authorization_status(&harness, "wa_replace")?, "stale");
        Ok(())
    }

    #[test]
    fn update_scope_dry_run_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let intake = harness.service.intake(
            intake_request(
                "req_dry_task",
                "idem_dry_task",
                false,
                Some(0),
                RequestedMode::Work,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("task ref should be present")
            .to_owned();
        let before = harness.counts()?;

        let response = harness.service.update_scope(
            update_scope_request(
                "req_scope_dry",
                "idem_scope_dry",
                true,
                Some(1),
                &task_id,
                ChangeUnitOperation::CreateCurrent,
                "Dry-run scope.",
            ),
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
        assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn scope_decision_ref_alone_does_not_change_current_scope() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let intake = harness.service.intake(
            intake_request(
                "req_decision_task",
                "idem_decision_task",
                false,
                Some(0),
                RequestedMode::Work,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("task ref should be present")
            .to_owned();
        let decision_ref = StateRecordRef {
            record_kind: StateRecordKind::UserJudgment,
            record_id: RecordId::new("uj_scope_decision"),
            project_id: ProjectId::new(PROJECT_ID),
            task_id: Some(TaskId::new(&task_id)),
            state_version: Some(1),
        };

        let response = harness.service.update_scope(
            UpdateScopeRequest {
                envelope: envelope(
                    "req_decision_only",
                    Some("idem_decision_only"),
                    false,
                    Some(1),
                    Some(&task_id),
                ),
                task_id: TaskId::new(&task_id),
                goal_summary: None,
                scope_update: None,
                scope_boundary: None,
                non_goals: None,
                acceptance_criteria: None,
                autonomy_boundary: None,
                baseline_ref: None,
                change_unit: ChangeUnitUpdate {
                    operation: ChangeUnitOperation::KeepCurrent,
                    fields: Map::new(),
                },
                related_scope_decision_refs: vec![decision_ref],
            },
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(
            response.response_value["state"]["scope_summary"],
            "Initial test scope."
        );
        assert_eq!(
            response.response_value["linked_scope_decision_refs"]
                .as_array()
                .expect("linked refs should be an array")
                .len(),
            1
        );
        Ok(())
    }

    #[test]
    fn prepare_write_allowed_creates_one_authorization_with_post_commit_basis(
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_allowed")?;
        let sensitive_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                "req_prepare_allowed_sensitive",
                "idem_prepare_allowed_sensitive",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::SensitiveApproval,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let sensitive_judgment_id =
            response_record_id(&sensitive_judgment.response_value, "user_judgment_ref");
        harness.service.record_user_judgment(
            record_judgment_request(
                "req_prepare_allowed_record",
                "idem_prepare_allowed_record",
                Some(3),
                &task_id,
                &sensitive_judgment_id,
                JudgmentKind::SensitiveApproval,
                answer_payload(JudgmentKind::SensitiveApproval),
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let before = harness.counts()?;

        let mut request = prepare_write_request(
            "req_prepare_allowed",
            "idem_prepare_allowed",
            Some(4),
            Some(&task_id),
            Some(&change_unit_id),
        );
        request.sensitive_categories = vec!["network".to_owned()];
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["decision"], "allowed");
        assert_eq!(response.response_value["authorization_effect"], "created");
        assert_eq!(response.response_value["base"]["state_version"], 5);
        assert_eq!(
            response.response_value["write_authorization"]["basis_state_version"],
            5
        );
        assert_eq!(
            response.response_value["write_authorization"]["authorized_attempt_scope"]
                ["intended_paths"],
            json!(["src/export.rs"])
        );
        assert_eq!(
            response.response_value["active_user_judgment_refs"]
                .as_array()
                .expect("active judgment refs should be an array")
                .len(),
            1
        );
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.write_authorizations, before.write_authorizations + 1);
        assert_eq!(after.task_events, before.task_events + 1);
        assert_eq!(after.tool_invocations, before.tool_invocations + 1);
        let write_authorization_id =
            response_record_id(&response.response_value, "write_authorization_ref");
        assert_eq!(
            write_authorization_basis(&harness, &write_authorization_id)?,
            5
        );
        Ok(())
    }

    #[test]
    fn prepare_write_blocked_path_creates_no_authorization() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_path")?;
        let before = harness.counts()?;

        let mut request = prepare_write_request(
            "req_prepare_path",
            "idem_prepare_path",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        request.intended_paths = vec!["src/other.rs".to_owned()];
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["decision"], "blocked");
        assert_prepare_reason(&response.response_value, "path_out_of_scope");
        assert!(response.response_value["write_authorization"].is_null());
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.write_authorizations, before.write_authorizations);
        Ok(())
    }

    #[test]
    fn prepare_write_missing_change_unit_returns_decision_reason() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let intake = harness.service.intake(
            intake_request(
                "req_prepare_no_cu_task",
                "idem_prepare_no_cu_task",
                false,
                Some(0),
                RequestedMode::Work,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("task ref should be present")
            .to_owned();
        let before = harness.counts()?;

        let request = prepare_write_request(
            "req_prepare_no_cu",
            "idem_prepare_no_cu",
            Some(1),
            Some(&task_id),
            None,
        );
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["decision"], "blocked");
        assert_prepare_reason(&response.response_value, "no_current_change_unit");
        assert_eq!(after.write_authorizations, before.write_authorizations);
        Ok(())
    }

    #[test]
    fn prepare_write_unresolved_user_judgment_requires_decision() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_judgment")?;
        harness.service.request_user_judgment(
            user_judgment_request(
                "req_prepare_judgment_pending",
                "idem_prepare_judgment_pending",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let before = harness.counts()?;

        let request = prepare_write_request(
            "req_prepare_judgment",
            "idem_prepare_judgment",
            Some(3),
            Some(&task_id),
            Some(&change_unit_id),
        );
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["decision"], "decision_required");
        assert_prepare_reason(&response.response_value, "user_judgment_unresolved");
        assert_eq!(after.write_authorizations, before.write_authorizations);
        Ok(())
    }

    #[test]
    fn prepare_write_missing_sensitive_approval_requires_approval() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, "prepare_sensitive")?;
        let before = harness.counts()?;

        let mut request = prepare_write_request(
            "req_prepare_sensitive",
            "idem_prepare_sensitive",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        request.sensitive_categories = vec!["network".to_owned()];
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["decision"], "approval_required");
        assert_prepare_reason(&response.response_value, "sensitive_approval_missing");
        assert_eq!(after.write_authorizations, before.write_authorizations);
        Ok(())
    }

    #[test]
    fn prepare_write_baseline_mismatch_blocks_authorization() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_baseline")?;
        let before = harness.counts()?;

        let mut request = prepare_write_request(
            "req_prepare_baseline",
            "idem_prepare_baseline",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        request.baseline_ref = BaselineRef::new("baseline_other");
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["decision"], "blocked");
        assert_prepare_reason(&response.response_value, "baseline_mismatch");
        assert_eq!(after.write_authorizations, before.write_authorizations);
        Ok(())
    }

    #[test]
    fn prepare_write_surface_access_mismatch_is_method_decision() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_surface")?;
        let before = harness.counts()?;

        let request = prepare_write_request(
            "req_prepare_surface_access",
            "idem_prepare_surface_access",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::CoreMutation))?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["decision"], "blocked");
        assert_prepare_reason(&response.response_value, "surface_access_class_mismatch");
        assert_eq!(after.write_authorizations, before.write_authorizations);
        Ok(())
    }

    #[test]
    fn prepare_write_unregistered_grant_fails_before_method_decision() -> Result<(), Box<dyn Error>>
    {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, "prepare_grant_fail")?;
        set_surface_local_access(
            &harness,
            json!({
                "authorized_access_classes": ["core_mutation"],
                "verification_basis": "method_test_registration"
            }),
        )?;
        let before = harness.counts()?;

        let request = prepare_write_request(
            "req_prepare_grant_fail",
            "idem_prepare_grant_fail",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "LOCAL_ACCESS_MISMATCH"
        );
        assert!(response
            .response_value
            .get("write_decision_reasons")
            .is_none());
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn prepare_write_surface_capability_insufficient_is_method_decision(
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_cap")?;
        set_surface_capability(&harness, "{}")?;
        let before = harness.counts()?;

        let request = prepare_write_request(
            "req_prepare_capability",
            "idem_prepare_capability",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["decision"], "blocked");
        assert_prepare_reason(&response.response_value, "surface_capability_insufficient");
        assert_eq!(after.write_authorizations, before.write_authorizations);
        Ok(())
    }

    #[test]
    fn prepare_write_product_write_flag_mismatch_blocks_authorization() -> Result<(), Box<dyn Error>>
    {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_flag")?;
        let before = harness.counts()?;

        let mut request = prepare_write_request(
            "req_prepare_flag",
            "idem_prepare_flag",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        request.product_file_write_intended = false;
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["decision"], "blocked");
        assert_prepare_reason(&response.response_value, "product_write_flag_mismatch");
        assert_eq!(after.write_authorizations, before.write_authorizations);
        Ok(())
    }

    #[test]
    fn prepare_write_dry_run_has_no_authorization_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_dry")?;
        let before = harness.counts()?;

        let mut request = prepare_write_request(
            "req_prepare_dry",
            "idem_prepare_dry",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        request.envelope.dry_run = true;
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

        assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
        assert_eq!(
            response.response_value["dry_run_summary"]["planned_effects"][0]["action"],
            "would_create"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn prepare_write_rejects_escaping_product_path_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_escape")?;
        let before = harness.counts()?;

        let mut request = prepare_write_request(
            "req_prepare_escape",
            "idem_prepare_escape",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        request.intended_paths = vec!["../outside.rs".to_owned()];
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn prepare_write_stale_state_rejects_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_stale")?;
        let before = harness.counts()?;

        let request = prepare_write_request(
            "req_prepare_stale",
            "idem_prepare_stale",
            Some(1),
            Some(&task_id),
            Some(&change_unit_id),
        );
        let response = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "STATE_VERSION_CONFLICT"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn prepare_write_idempotency_replays_without_second_authorization() -> Result<(), Box<dyn Error>>
    {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "prepare_replay")?;
        let request = prepare_write_request(
            "req_prepare_replay",
            "idem_prepare_replay",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );

        let first = harness
            .service
            .prepare_write(request.clone(), invocation(AccessClass::WriteAuthorization))?;
        let after_first = harness.counts()?;
        let second = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

        assert_eq!(first.response_value["decision"], "allowed");
        assert!(second.replayed);
        assert_eq!(second.response_json, first.response_json);
        assert_eq!(harness.counts()?, after_first);
        assert_eq!(write_authorization_count(&harness)?, 1);
        Ok(())
    }

    #[test]
    fn prepare_write_replay_requires_current_verified_grant() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, "prepare_replay_verify")?;
        let request = prepare_write_request(
            "req_prepare_replay_verify",
            "idem_prepare_replay_verify",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        );
        let first = harness
            .service
            .prepare_write(request.clone(), invocation(AccessClass::WriteAuthorization))?;
        let after_first = harness.counts()?;
        set_surface_local_access(
            &harness,
            json!({
                "authorized_access_classes": ["core_mutation"],
                "verification_basis": "method_test_registration"
            }),
        )?;

        let second = harness
            .service
            .prepare_write(request, invocation(AccessClass::WriteAuthorization))?;

        assert_eq!(first.response_value["decision"], "allowed");
        assert!(!second.replayed);
        assert_eq!(second.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            second.response_value["errors"][0]["code"],
            "LOCAL_ACCESS_MISMATCH"
        );
        assert_ne!(second.response_json, first.response_json);
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    #[test]
    fn stage_artifact_creates_transient_handle_without_core_commit() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_stage_artifact_capability(&harness)?;
        let (task_id, _) = create_task_with_change_unit(&harness, "stage_valid")?;
        let before = harness.counts()?;

        let mut request = stage_artifact_request(
            "req_stage_valid",
            Some("idem_stage_valid"),
            false,
            Some(2),
            &task_id,
        );
        request.display_name = "trace.log".to_owned();
        request.content_type = "text/plain; charset=utf-8".to_owned();
        request.safe_bytes_or_notice = "Local trace sample captured for debugging.".to_owned();
        let response = harness
            .service
            .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;
        let after = harness.counts()?;
        let handle_id = response.response_value["staged_artifact_handle"]["handle_id"]
            .as_str()
            .expect("handle id should be present")
            .to_owned();
        let row = staged_artifact_row(&harness, &handle_id)?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(
            response.response_value["base"]["effect_kind"],
            "staging_created"
        );
        assert_eq!(response.response_value["base"]["state_version"], 2);
        assert_eq!(response.response_value["base"]["events"], json!([]));
        assert_eq!(
            response.response_value["staged_artifact_handle"]["consumed"],
            false
        );
        assert_eq!(response.response_value.get("artifact_ref"), None);
        assert_eq!(after.state_version, before.state_version);
        assert_eq!(after.artifact_staging, before.artifact_staging + 1);
        assert_eq!(after.artifacts, before.artifacts);
        assert_eq!(after.task_events, before.task_events);
        assert_eq!(after.tool_invocations, before.tool_invocations);
        assert_eq!(row.status, "staged");
        assert_eq!(row.redaction_state, "none");
        assert_eq!(row.created_by_surface_id, SURFACE_ID);
        assert_eq!(row.created_by_surface_instance_id, SURFACE_INSTANCE_ID);
        assert!(row.tmp_path.ends_with(".txt"));
        assert!(harness
            .runtime_home_path
            .join("projects")
            .join(PROJECT_ID)
            .join(&row.tmp_path)
            .exists());
        assert!(
            (23.99..=24.01).contains(&row.ttl_hours),
            "expected 24h TTL, got {}",
            row.ttl_hours
        );
        Ok(())
    }

    #[test]
    fn stage_artifact_rejects_checksum_mismatch_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_stage_artifact_capability(&harness)?;
        let (task_id, _) = create_task_with_change_unit(&harness, "stage_sha")?;
        let before = harness.counts()?;

        let mut request = stage_artifact_request(
            "req_stage_sha",
            Some("idem_stage_sha"),
            false,
            Some(2),
            &task_id,
        );
        request.safe_bytes_or_notice = "checksum mismatch sample".to_owned();
        request.expected_sha256 = Some("sha256:0000".to_owned());
        let response = harness
            .service
            .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn stage_artifact_rejects_size_mismatch_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_stage_artifact_capability(&harness)?;
        let (task_id, _) = create_task_with_change_unit(&harness, "stage_size")?;
        let before = harness.counts()?;

        let mut request = stage_artifact_request(
            "req_stage_size",
            Some("idem_stage_size"),
            false,
            Some(2),
            &task_id,
        );
        request.safe_bytes_or_notice = "size mismatch sample".to_owned();
        request.expected_size_bytes = Some(999);
        let response = harness
            .service
            .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn stage_artifact_rejects_oversized_input_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_stage_artifact_capability(&harness)?;
        let (task_id, _) = create_task_with_change_unit(&harness, "stage_big")?;
        let before = harness.counts()?;

        let mut request = stage_artifact_request(
            "req_stage_big",
            Some("idem_stage_big"),
            false,
            Some(2),
            &task_id,
        );
        request.display_name = "huge.log".to_owned();
        request.safe_bytes_or_notice = "x".repeat(MAX_STAGED_BODY_BYTES + 1);
        let response = harness
            .service
            .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn stage_artifact_rejects_unsafe_secret_input_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_stage_artifact_capability(&harness)?;
        let (task_id, _) = create_task_with_change_unit(&harness, "stage_secret")?;
        let before = harness.counts()?;

        let mut request = stage_artifact_request(
            "req_stage_secret",
            Some("idem_stage_secret"),
            false,
            Some(2),
            &task_id,
        );
        request.display_name = "secrets.log".to_owned();
        request.safe_bytes_or_notice = "password=hunter2".to_owned();
        let response = harness
            .service
            .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn stage_artifact_rejects_unsupported_redaction_state() -> Result<(), Box<dyn Error>> {
        let mut value = serde_json::to_value(stage_artifact_request(
            "req_stage_bad_redaction",
            Some("idem_stage_bad_redaction"),
            false,
            Some(2),
            "task_redaction",
        ))?;
        value["redaction_state"] = json!("unsupported");

        let error = serde_json::from_value::<StageArtifactRequest>(value)
            .expect_err("unsupported redaction_state should not deserialize");
        assert!(error.to_string().contains("unknown variant"));
        Ok(())
    }

    #[test]
    fn stage_artifact_dry_run_creates_no_handle_or_storage() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_stage_artifact_capability(&harness)?;
        let (task_id, _) = create_task_with_change_unit(&harness, "stage_dry")?;
        let before = harness.counts()?;

        let mut request = stage_artifact_request(
            "req_stage_dry",
            Some("idem_stage_dry"),
            true,
            Some(2),
            &task_id,
        );
        request.display_name = "trace.md".to_owned();
        request.content_type = "text/markdown".to_owned();
        request.redaction_state = RedactionState::Redacted;
        request.safe_bytes_or_notice = "Redacted diagnostic excerpt.".to_owned();
        let response = harness
            .service
            .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

        assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
        assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
        assert!(response
            .response_value
            .get("staged_artifact_handle")
            .is_none());
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn stage_artifact_dry_run_still_checks_stale_state() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_stage_artifact_capability(&harness)?;
        let (task_id, _) = create_task_with_change_unit(&harness, "stage_dry_stale")?;
        let before = harness.counts()?;

        let request = stage_artifact_request(
            "req_stage_dry_stale",
            Some("idem_stage_dry_stale"),
            true,
            Some(1),
            &task_id,
        );
        let response = harness
            .service
            .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "STATE_VERSION_CONFLICT"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn stage_artifact_invalid_input_does_not_bypass_access_preflight() -> Result<(), Box<dyn Error>>
    {
        let harness = MethodHarness::new()?;
        enable_stage_artifact_capability(&harness)?;
        let (task_id, _) = create_task_with_change_unit(&harness, "stage_access_first")?;
        let before = harness.counts()?;

        let mut request = stage_artifact_request(
            "req_stage_access_first",
            Some("idem_stage_access_first"),
            true,
            Some(2),
            &task_id,
        );
        request.safe_bytes_or_notice = String::new();
        let response = harness
            .service
            .stage_artifact(request, invocation(AccessClass::ReadStatus))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "CAPABILITY_INSUFFICIENT"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn stage_artifact_uses_verified_surface_provenance() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_stage_artifact_capability(&harness)?;
        let (task_id, _) = create_task_with_change_unit(&harness, "stage_provenance")?;

        let mut request = stage_artifact_request(
            "req_stage_provenance",
            Some("idem_stage_provenance"),
            false,
            Some(2),
            &task_id,
        );
        request.display_name = "binary.bin".to_owned();
        request.content_type = "application/octet-stream".to_owned();
        request.redaction_state = RedactionState::Blocked;
        request.safe_bytes_or_notice = "Binary output omitted; see local run context.".to_owned();

        let response = harness
            .service
            .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;

        assert_eq!(
            response.response_value["staged_artifact_handle"]["created_by_surface_id"],
            SURFACE_ID
        );
        assert_eq!(
            response.response_value["staged_artifact_handle"]["created_by_surface_instance_id"],
            SURFACE_INSTANCE_ID
        );
        assert_eq!(
            response.response_value["staged_artifact_handle"]["redaction_state"],
            "blocked"
        );
        let handle_id = response.response_value["staged_artifact_handle"]["handle_id"]
            .as_str()
            .expect("handle id should be present");
        let row = staged_artifact_row(&harness, handle_id)?;
        assert_eq!(row.created_by_surface_id, SURFACE_ID);
        assert_eq!(row.created_by_surface_instance_id, SURFACE_INSTANCE_ID);
        Ok(())
    }

    #[test]
    fn stage_artifact_rejects_caller_submitted_provenance_fields() -> Result<(), Box<dyn Error>> {
        let mut value = serde_json::to_value(stage_artifact_request(
            "req_stage_forged_provenance",
            Some("idem_stage_forged_provenance"),
            false,
            Some(2),
            "task_forged_provenance",
        ))?;
        value["created_by_surface_id"] = json!("forged_surface");
        value["created_by_surface_instance_id"] = json!("forged_instance");

        let error = serde_json::from_value::<StageArtifactRequest>(value)
            .expect_err("caller-submitted provenance fields should be rejected");

        assert!(error.to_string().contains("created_by_surface_id"));
        Ok(())
    }

    #[test]
    fn record_run_without_product_write_commits_run_only() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_no_write")?;
        let before = harness.counts()?;

        let response = harness.service.record_run(
            record_run_request(
                "req_run_no_write",
                "idem_run_no_write",
                false,
                Some(2),
                &task_id,
                &change_unit_id,
            ),
            invocation(AccessClass::RunRecording),
        )?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["state_version"], 3);
        assert_eq!(
            response.response_value["run_summary"]["observed_changes"]
                ["product_file_write_observed"],
            false
        );
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.runs, before.runs + 1);
        assert_eq!(after.write_authorizations, before.write_authorizations);
        assert_eq!(after.artifacts, before.artifacts);
        assert_eq!(after.task_events, before.task_events + 1);
        assert_eq!(after.tool_invocations, before.tool_invocations + 1);
        Ok(())
    }

    #[test]
    fn record_run_product_write_consumes_valid_authorization_once() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_write")?;
        let write_authorization_id =
            prepare_write_authorization(&harness, &task_id, &change_unit_id, 2, "run_write")?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            "req_run_write",
            "idem_run_write",
            false,
            Some(3),
            &task_id,
            &change_unit_id,
        );
        request.observed_changes.product_file_write_observed = true;
        request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
        request.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
        let response = harness
            .service
            .record_run(request, invocation(AccessClass::RunRecording))?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["base"]["state_version"], 4);
        assert_eq!(
            write_authorization_status(&harness, &write_authorization_id)?,
            "consumed"
        );
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.runs, before.runs + 1);
        assert_eq!(after.write_authorizations, before.write_authorizations);
        assert_eq!(after.task_events, before.task_events + 1);
        assert_eq!(after.tool_invocations, before.tool_invocations + 1);
        Ok(())
    }

    #[test]
    fn record_run_missing_authorization_rejects_product_write_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_missing_auth")?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            "req_run_missing_auth",
            "idem_run_missing_auth",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.observed_changes.product_file_write_observed = true;
        request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
        let response = harness
            .service
            .record_run(request, invocation(AccessClass::RunRecording))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "WRITE_AUTHORIZATION_REQUIRED"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn record_run_stale_authorization_basis_rejects_before_consumption(
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stale_auth")?;
        let write_authorization_id =
            prepare_write_authorization(&harness, &task_id, &change_unit_id, 2, "run_stale_auth")?;
        harness.service.update_scope(
            update_scope_request(
                "req_run_stale_auth_touch",
                "idem_run_stale_auth_touch",
                false,
                Some(3),
                &task_id,
                ChangeUnitOperation::KeepCurrent,
                "Initial current scope.",
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            "req_run_stale_auth",
            "idem_run_stale_auth",
            false,
            Some(4),
            &task_id,
            &change_unit_id,
        );
        request.observed_changes.product_file_write_observed = true;
        request.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
        request.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
        let response = harness
            .service
            .record_run(request, invocation(AccessClass::RunRecording))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "STATE_VERSION_CONFLICT"
        );
        assert_eq!(
            write_authorization_status(&harness, &write_authorization_id)?,
            "active"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn record_run_consumed_authorization_reuse_rejects_without_effect() -> Result<(), Box<dyn Error>>
    {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_reuse_auth")?;
        let write_authorization_id =
            prepare_write_authorization(&harness, &task_id, &change_unit_id, 2, "run_reuse_auth")?;

        let mut first = record_run_request(
            "req_run_reuse_first",
            "idem_run_reuse_first",
            false,
            Some(3),
            &task_id,
            &change_unit_id,
        );
        first.observed_changes.product_file_write_observed = true;
        first.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
        first.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
        harness
            .service
            .record_run(first, invocation(AccessClass::RunRecording))?;
        let before = harness.counts()?;

        let mut second = record_run_request(
            "req_run_reuse_second",
            "idem_run_reuse_second",
            false,
            Some(4),
            &task_id,
            &change_unit_id,
        );
        second.observed_changes.product_file_write_observed = true;
        second.observed_changes.changed_paths = vec!["src/export.rs".to_owned()];
        second.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
        let response = harness
            .service
            .record_run(second, invocation(AccessClass::RunRecording))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "WRITE_AUTHORIZATION_INVALID"
        );
        assert_eq!(
            response.response_value["errors"][0]["details"]["authorization_reason"],
            "consumed"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn record_run_path_mismatch_rejects_without_consuming_authorization(
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_path_auth")?;
        let write_authorization_id =
            prepare_write_authorization(&harness, &task_id, &change_unit_id, 2, "run_path_auth")?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            "req_run_path_auth",
            "idem_run_path_auth",
            false,
            Some(3),
            &task_id,
            &change_unit_id,
        );
        request.observed_changes.product_file_write_observed = true;
        request.observed_changes.changed_paths = vec!["tests/export.rs".to_owned()];
        request.write_authorization_id = Some(WriteAuthorizationId::new(&write_authorization_id));
        let response = harness
            .service
            .record_run(request, invocation(AccessClass::RunRecording))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "WRITE_AUTHORIZATION_INVALID"
        );
        assert_eq!(
            write_authorization_status(&harness, &write_authorization_id)?,
            "active"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn record_run_promotes_staged_artifact_and_updates_evidence() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_artifact")?;
        let handle = stage_artifact_for_record_run(&harness, &task_id, "run_artifact", 2)?;
        let handle_id = handle.handle_id.as_str().to_owned();
        let before = harness.counts()?;

        let mut request = record_run_request(
            "req_run_artifact",
            "idem_run_artifact",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.artifact_inputs = vec![artifact_input_for_handle(
            "artifact_input_report",
            handle,
            Some("validation_report"),
            Some("Search-result count validation passed."),
        )];
        request.evidence_updates = vec![supported_evidence_update(
            "Search-result count validation passed.",
        )];
        let response = harness
            .service
            .record_run(request, invocation(AccessClass::RunRecording))?;
        let after = harness.counts()?;
        let artifact_id = response.response_value["registered_artifacts"][0]["artifact_id"]
            .as_str()
            .expect("artifact id should be present")
            .to_owned();

        assert_eq!(response.response_value["base"]["state_version"], 3);
        assert_eq!(
            response.response_value["evidence_summary"]["status"],
            "sufficient"
        );
        assert_eq!(
            response.response_value["evidence_summary"]["coverage_items"][0]["supporting_refs"][0]
                ["record_kind"],
            "run"
        );
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.runs, before.runs + 1);
        assert_eq!(after.artifacts, before.artifacts + 1);
        assert_eq!(after.artifact_links, before.artifact_links + 2);
        assert_eq!(after.evidence_summaries, before.evidence_summaries + 1);
        assert_eq!(artifact_staging_status(&harness, &handle_id)?, "consumed");
        assert!(artifact_owner_link_exists(&harness, &artifact_id, "run")?);
        assert!(artifact_owner_link_exists(
            &harness,
            &artifact_id,
            "evidence_summary"
        )?);
        Ok(())
    }

    #[test]
    fn record_run_staged_artifact_surface_mismatch_rejects_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, "run_stage_surface")?;
        let mut handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_surface", 2)?;
        handle.created_by_surface_id = SurfaceId::new("forged_surface");
        let before = harness.counts()?;

        let mut request = record_run_request(
            "req_run_stage_surface",
            "idem_run_stage_surface",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.artifact_inputs = vec![artifact_input_for_handle(
            "artifact_input_surface",
            handle,
            None,
            None,
        )];
        let response = harness
            .service
            .record_run(request, invocation(AccessClass::RunRecording))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
            "staged_handle_surface_mismatch"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn record_run_expired_staged_artifact_rejects_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, "run_stage_expired")?;
        let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_expired", 2)?;
        expire_staged_artifact(&harness, handle.handle_id.as_str())?;
        let before = harness.counts()?;

        let mut request = record_run_request(
            "req_run_stage_expired",
            "idem_run_stage_expired",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.artifact_inputs = vec![artifact_input_for_handle(
            "artifact_input_expired",
            handle,
            None,
            None,
        )];
        let response = harness
            .service
            .record_run(request, invocation(AccessClass::RunRecording))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
            "staged_handle_expired"
        );
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn record_run_checksum_mismatch_rejects_and_rolls_back_all_effects(
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_stage_sha")?;
        let handle = stage_artifact_for_record_run(&harness, &task_id, "run_stage_sha", 2)?;
        let handle_id = handle.handle_id.as_str().to_owned();
        let before = harness.counts()?;

        let mut input = artifact_input_for_handle("artifact_input_sha", handle, None, None);
        input.expected_sha256 = Some("sha256:0000".to_owned());
        let mut request = record_run_request(
            "req_run_stage_sha",
            "idem_run_stage_sha",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.artifact_inputs = vec![input];
        let response = harness
            .service
            .record_run(request, invocation(AccessClass::RunRecording))?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["details"]["artifact_input_error"]["reason"],
            "staged_handle_checksum_mismatch"
        );
        assert_eq!(harness.counts()?, before);
        assert_eq!(artifact_staging_status(&harness, &handle_id)?, "staged");
        Ok(())
    }

    #[test]
    fn record_run_dry_run_and_idempotency_replay_have_no_extra_effects(
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "run_replay")?;
        let before_dry = harness.counts()?;
        let dry_run = harness.service.record_run(
            record_run_request(
                "req_run_dry",
                "idem_run_dry",
                true,
                Some(2),
                &task_id,
                &change_unit_id,
            ),
            invocation(AccessClass::RunRecording),
        )?;
        assert_eq!(dry_run.response_value["base"]["response_kind"], "dry_run");
        assert_eq!(harness.counts()?, before_dry);

        let request = record_run_request(
            "req_run_replay",
            "idem_run_replay",
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        let first = harness
            .service
            .record_run(request.clone(), invocation(AccessClass::RunRecording))?;
        let after_first = harness.counts()?;
        let second = harness
            .service
            .record_run(request, invocation(AccessClass::RunRecording))?;

        assert!(second.replayed);
        assert_eq!(second.response_json, first.response_json);
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    #[test]
    fn request_user_judgment_creates_pending_record() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "pending")?;
        let before = harness.counts()?;

        let response = harness.service.request_user_judgment(
            user_judgment_request(
                "req_judgment_pending",
                "idem_judgment_pending",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let after = harness.counts()?;
        let judgment_id = response_record_id(&response.response_value, "user_judgment_ref");

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["state_version"], 3);
        assert_eq!(
            response.response_value["user_judgment"]["status"],
            "pending"
        );
        assert_eq!(
            response.response_value["user_judgment"]["judgment_kind"],
            "product_decision"
        );
        assert_eq!(
            response.response_value["state"]["pending_user_judgment_refs"]
                .as_array()
                .expect("pending refs should be an array")
                .len(),
            1
        );
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.user_judgments, before.user_judgments + 1);
        assert_eq!(user_judgment_status(&harness, &judgment_id)?, "pending");
        Ok(())
    }

    #[test]
    fn record_user_judgment_resolves_pending_record() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "resolve")?;
        let pending_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                "req_judgment_resolve",
                "idem_judgment_resolve",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let pending_judgment_id =
            response_record_id(&pending_judgment.response_value, "user_judgment_ref");
        let before = harness.counts()?;

        let response = harness.service.record_user_judgment(
            record_judgment_request(
                "req_record_resolve",
                "idem_record_resolve",
                Some(3),
                &task_id,
                &pending_judgment_id,
                JudgmentKind::ProductDecision,
                answer_payload(JudgmentKind::ProductDecision),
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["state_version"], 4);
        assert_eq!(
            response.response_value["user_judgment"]["status"],
            "resolved"
        );
        assert_eq!(
            response.response_value["user_judgment"]["resolution"]["resolved_by_actor_kind"],
            "user"
        );
        assert_eq!(
            response.response_value["state"]["pending_user_judgment_refs"]
                .as_array()
                .expect("pending refs should be an array")
                .len(),
            0
        );
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.user_judgments, before.user_judgments);
        assert_eq!(
            user_judgment_status(&harness, &pending_judgment_id)?,
            "resolved"
        );
        assert!(
            resolution_json(&harness, &pending_judgment_id)?["answer"]["product_decision"]
                .is_object()
        );
        Ok(())
    }

    #[test]
    fn incompatible_judgment_kind_is_rejected_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "kind")?;
        let pending_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                "req_judgment_kind",
                "idem_judgment_kind",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let pending_judgment_id =
            response_record_id(&pending_judgment.response_value, "user_judgment_ref");
        let before = harness.counts()?;

        let response = harness.service.record_user_judgment(
            record_judgment_request(
                "req_record_wrong_kind",
                "idem_record_wrong_kind",
                Some(3),
                &task_id,
                &pending_judgment_id,
                JudgmentKind::TechnicalDecision,
                answer_payload(JudgmentKind::TechnicalDecision),
            ),
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "DECISION_UNRESOLVED"
        );
        assert_eq!(harness.counts()?, before);
        assert_eq!(
            user_judgment_status(&harness, &pending_judgment_id)?,
            "pending"
        );
        Ok(())
    }

    #[test]
    fn final_acceptance_does_not_substitute_for_residual_risk_acceptance(
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "risk")?;
        let pending_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                "req_judgment_risk",
                "idem_judgment_risk",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ResidualRiskAcceptance,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let pending_judgment_id =
            response_record_id(&pending_judgment.response_value, "user_judgment_ref");
        let before = harness.counts()?;

        let response = harness.service.record_user_judgment(
            record_judgment_request(
                "req_record_final_for_risk",
                "idem_record_final_for_risk",
                Some(3),
                &task_id,
                &pending_judgment_id,
                JudgmentKind::ResidualRiskAcceptance,
                answer_payload(JudgmentKind::FinalAcceptance),
            ),
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(harness.counts()?, before);
        assert_eq!(
            user_judgment_status(&harness, &pending_judgment_id)?,
            "pending"
        );
        Ok(())
    }

    #[test]
    fn sensitive_action_scope_does_not_create_write_authorization() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "sensitive")?;
        let pending_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                "req_judgment_sensitive",
                "idem_judgment_sensitive",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::SensitiveApproval,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let pending_judgment_id =
            response_record_id(&pending_judgment.response_value, "user_judgment_ref");
        let before = harness.counts()?;

        let response = harness.service.record_user_judgment(
            record_judgment_request(
                "req_record_sensitive",
                "idem_record_sensitive",
                Some(3),
                &task_id,
                &pending_judgment_id,
                JudgmentKind::SensitiveApproval,
                answer_payload(JudgmentKind::SensitiveApproval),
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(after.write_authorizations, before.write_authorizations);
        assert_eq!(
            response.response_value["state"]["write_authority_summary"],
            Value::Null
        );
        Ok(())
    }

    #[test]
    fn recorded_scope_decision_does_not_change_scope_or_current_change_unit(
    ) -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "scope_judgment")?;
        let original_scope = current_change_unit_scope(&harness, &task_id)?;
        let original_current = current_change_unit_id(&harness, &task_id)?;
        let pending_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                "req_judgment_scope",
                "idem_judgment_scope",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ScopeDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let pending_judgment_id =
            response_record_id(&pending_judgment.response_value, "user_judgment_ref");
        let before = harness.counts()?;

        let response = harness.service.record_user_judgment(
            record_judgment_request(
                "req_record_scope",
                "idem_record_scope",
                Some(3),
                &task_id,
                &pending_judgment_id,
                JudgmentKind::ScopeDecision,
                answer_payload(JudgmentKind::ScopeDecision),
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let after = harness.counts()?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(
            response.response_value["state"]["scope_summary"],
            "Initial current scope."
        );
        assert_eq!(
            current_change_unit_scope(&harness, &task_id)?,
            original_scope
        );
        assert_eq!(
            current_change_unit_id(&harness, &task_id)?,
            original_current
        );
        assert_eq!(after.change_units, before.change_units);
        Ok(())
    }

    #[test]
    fn judgment_dry_runs_have_no_storage_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "dry_judgment")?;
        let before_request = harness.counts()?;

        let request_preview = harness.service.request_user_judgment(
            user_judgment_request(
                "req_judgment_dry",
                "idem_judgment_dry",
                true,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(
            request_preview.response_value["base"]["response_kind"],
            "dry_run"
        );
        assert_eq!(harness.counts()?, before_request);

        let pending_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                "req_judgment_dry_record",
                "idem_judgment_dry_record",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let pending_judgment_id =
            response_record_id(&pending_judgment.response_value, "user_judgment_ref");
        let before_record = harness.counts()?;

        let mut record_preview_request = record_judgment_request(
            "req_record_dry",
            "idem_record_dry",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        );
        record_preview_request.envelope.dry_run = true;
        let record_preview = harness.service.record_user_judgment(
            record_preview_request,
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(
            record_preview.response_value["base"]["response_kind"],
            "dry_run"
        );
        assert_eq!(harness.counts()?, before_record);
        assert_eq!(
            user_judgment_status(&harness, &pending_judgment_id)?,
            "pending"
        );
        Ok(())
    }

    #[test]
    fn stale_state_rejects_record_user_judgment_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "stale_judgment")?;
        let pending_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                "req_judgment_stale",
                "idem_judgment_stale",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let pending_judgment_id =
            response_record_id(&pending_judgment.response_value, "user_judgment_ref");
        let before = harness.counts()?;

        let response = harness.service.record_user_judgment(
            record_judgment_request(
                "req_record_stale",
                "idem_record_stale",
                Some(2),
                &task_id,
                &pending_judgment_id,
                JudgmentKind::ProductDecision,
                answer_payload(JudgmentKind::ProductDecision),
            ),
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "STATE_VERSION_CONFLICT"
        );
        assert_eq!(harness.counts()?, before);
        assert_eq!(
            user_judgment_status(&harness, &pending_judgment_id)?,
            "pending"
        );
        Ok(())
    }

    #[test]
    fn record_user_judgment_idempotency_replays_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "replay_judgment")?;
        let pending_judgment = harness.service.request_user_judgment(
            user_judgment_request(
                "req_judgment_replay",
                "idem_judgment_replay",
                false,
                Some(2),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::ProductDecision,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let pending_judgment_id =
            response_record_id(&pending_judgment.response_value, "user_judgment_ref");
        let request = record_judgment_request(
            "req_record_replay",
            "idem_record_replay",
            Some(3),
            &task_id,
            &pending_judgment_id,
            JudgmentKind::ProductDecision,
            answer_payload(JudgmentKind::ProductDecision),
        );

        let first = harness
            .service
            .record_user_judgment(request.clone(), invocation(AccessClass::CoreMutation))?;
        let after_first = harness.counts()?;
        let second = harness
            .service
            .record_user_judgment(request, invocation(AccessClass::CoreMutation))?;

        assert!(second.replayed);
        assert_eq!(second.response_json, first.response_json);
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    #[test]
    fn close_task_check_is_read_only() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, _) = create_task_with_change_unit(&harness, "close_check")?;
        let before = harness.counts()?;

        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: "req_close_check",
                idempotency_key: None,
                dry_run: false,
                expected_state_version: None,
                task_id: &task_id,
                intent: CloseIntent::Check,
                close_reason: None,
                superseding_task_id: None,
            }),
            invocation(AccessClass::ReadStatus),
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
        assert_eq!(response.response_value["base"]["events"], json!([]));
        assert_close_blocker(&response.response_value, "missing_final_acceptance");
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn close_task_check_dry_run_is_read_only() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, _) = create_task_with_change_unit(&harness, "close_check_dry")?;
        let before = harness.counts()?;

        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: "req_close_check_dry",
                idempotency_key: Some("idem_close_check_dry"),
                dry_run: true,
                expected_state_version: Some(1),
                task_id: &task_id,
                intent: CloseIntent::Check,
                close_reason: None,
                superseding_task_id: None,
            }),
            invocation(AccessClass::ReadStatus),
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
        assert_eq!(response.response_value["base"]["dry_run"], true);
        assert_close_blocker(&response.response_value, "missing_final_acceptance");
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn close_task_complete_blocks_missing_final_acceptance() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_no_final")?;
        let state_version =
            record_close_evidence(&harness, &task_id, &change_unit_id, 2, "no_final", true)?;
        let before = harness.counts()?;

        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: "req_close_no_final",
                idempotency_key: Some("idem_close_no_final"),
                dry_run: false,
                expected_state_version: Some(state_version),
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }),
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
        assert_eq!(response.response_value["close_state"], "blocked");
        assert_close_blocker(&response.response_value, "missing_final_acceptance");
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn close_task_complete_blocks_unsupported_evidence_claim() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, "close_bad_evidence")?;
        let after_evidence = record_close_evidence(
            &harness,
            &task_id,
            &change_unit_id,
            2,
            "bad_evidence",
            false,
        )?;
        let after_final =
            record_final_acceptance(&harness, &task_id, &change_unit_id, after_evidence, "bad")?;
        let before = harness.counts()?;

        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: "req_close_bad_evidence",
                idempotency_key: Some("idem_close_bad_evidence"),
                dry_run: false,
                expected_state_version: Some(after_final),
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }),
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(response.response_value["close_state"], "blocked");
        assert_close_blocker(&response.response_value, "evidence_claim_unsupported");
        assert_no_close_blocker(&response.response_value, "STATE_VERSION_CONFLICT");
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn close_task_complete_success() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_success")?;
        let after_evidence =
            record_close_evidence(&harness, &task_id, &change_unit_id, 2, "success", true)?;
        let after_final =
            record_final_acceptance(&harness, &task_id, &change_unit_id, after_evidence, "ok")?;
        let before = harness.counts()?;

        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: "req_close_success",
                idempotency_key: Some("idem_close_success"),
                dry_run: false,
                expected_state_version: Some(after_final),
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }),
            invocation(AccessClass::CoreMutation),
        )?;
        let after = harness.counts()?;
        let fields = task_terminal_fields(&harness, &task_id)?;

        assert_eq!(response.response_value["close_state"], "closed");
        assert_eq!(response.response_value["blockers"], json!([]));
        assert_eq!(
            response.response_value["base"]["effect_kind"],
            "core_committed"
        );
        assert_eq!(
            response.response_value["base"]["state_version"],
            after_final + 1
        );
        assert_eq!(fields.lifecycle_phase, "completed");
        assert_eq!(fields.result.as_deref(), Some("completed"));
        assert_eq!(
            fields.close_summary["close_reason"],
            "completed_self_checked"
        );
        assert!(fields.closed_at.is_some());
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.task_events, before.task_events + 1);
        assert_eq!(after.tool_invocations, before.tool_invocations + 1);
        Ok(())
    }

    #[test]
    fn close_task_cancel_success_despite_missing_completion_evidence() -> Result<(), Box<dyn Error>>
    {
        let harness = MethodHarness::new()?;
        let (task_id, _) = create_task_with_change_unit(&harness, "close_cancel")?;
        let before = harness.counts()?;

        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: "req_close_cancel",
                idempotency_key: Some("idem_close_cancel"),
                dry_run: false,
                expected_state_version: Some(2),
                task_id: &task_id,
                intent: CloseIntent::Cancel,
                close_reason: Some(CloseReason::Cancelled),
                superseding_task_id: None,
            }),
            invocation(AccessClass::CoreMutation),
        )?;
        let after = harness.counts()?;
        let fields = task_terminal_fields(&harness, &task_id)?;

        assert_eq!(response.response_value["close_state"], "cancelled");
        assert_eq!(response.response_value["blockers"], json!([]));
        assert_eq!(fields.lifecycle_phase, "cancelled");
        assert_eq!(fields.result.as_deref(), Some("cancelled"));
        assert_eq!(fields.close_summary["close_reason"], "cancelled");
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.task_events, before.task_events + 1);
        assert_eq!(after.tool_invocations, before.tool_invocations + 1);
        Ok(())
    }

    #[test]
    fn close_task_supersede_success() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, _) = create_task_with_change_unit(&harness, "close_supersede")?;
        let superseding_task_id = "task_close_superseding";
        insert_superseding_task(&harness, superseding_task_id)?;
        let before = harness.counts()?;

        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: "req_close_supersede",
                idempotency_key: Some("idem_close_supersede"),
                dry_run: false,
                expected_state_version: Some(2),
                task_id: &task_id,
                intent: CloseIntent::Supersede,
                close_reason: Some(CloseReason::Superseded),
                superseding_task_id: Some(superseding_task_id),
            }),
            invocation(AccessClass::CoreMutation),
        )?;
        let after = harness.counts()?;
        let fields = task_terminal_fields(&harness, &task_id)?;

        assert_eq!(response.response_value["close_state"], "superseded");
        assert_eq!(response.response_value["blockers"], json!([]));
        assert_eq!(fields.lifecycle_phase, "superseded");
        assert_eq!(fields.result.as_deref(), Some("superseded"));
        assert_eq!(
            active_task_id(&harness)?.as_deref(),
            Some(superseding_task_id)
        );
        assert_eq!(after.state_version, before.state_version + 1);
        assert_eq!(after.task_events, before.task_events + 1);
        assert_eq!(after.tool_invocations, before.tool_invocations + 1);
        Ok(())
    }

    #[test]
    fn close_task_stale_state_rejected_without_blocker() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, _) = create_task_with_change_unit(&harness, "close_stale")?;
        let before = harness.counts()?;

        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: "req_close_stale",
                idempotency_key: Some("idem_close_stale"),
                dry_run: false,
                expected_state_version: Some(1),
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }),
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "STATE_VERSION_CONFLICT"
        );
        assert!(response.response_value.get("blockers").is_none());
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn close_task_blocker_code_routing_uses_method_local_codes() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, _) = create_task_with_change_unit(&harness, "close_codes")?;
        let before = harness.counts()?;

        let response = harness.service.close_task(
            close_task_request(CloseTaskFixture {
                request_id: "req_close_codes",
                idempotency_key: Some("idem_close_codes"),
                dry_run: false,
                expected_state_version: Some(2),
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }),
            invocation(AccessClass::CoreMutation),
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_close_blocker(&response.response_value, "missing_final_acceptance");
        assert_no_close_blocker(&response.response_value, "STATE_VERSION_CONFLICT");
        assert!(response.response_value.get("errors").is_none());
        assert_eq!(harness.counts()?, before);
        Ok(())
    }

    #[test]
    fn close_task_idempotency_replays_terminal_transition() -> Result<(), Box<dyn Error>> {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) = create_task_with_change_unit(&harness, "close_replay")?;
        let after_evidence =
            record_close_evidence(&harness, &task_id, &change_unit_id, 2, "replay", true)?;
        let after_final = record_final_acceptance(
            &harness,
            &task_id,
            &change_unit_id,
            after_evidence,
            "replay",
        )?;
        let request = close_task_request(CloseTaskFixture {
            request_id: "req_close_replay",
            idempotency_key: Some("idem_close_replay"),
            dry_run: false,
            expected_state_version: Some(after_final),
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        });

        let first = harness
            .service
            .close_task(request.clone(), invocation(AccessClass::CoreMutation))?;
        let after_first = harness.counts()?;
        let second = harness
            .service
            .close_task(request, invocation(AccessClass::CoreMutation))?;

        assert_eq!(first.response_value["close_state"], "closed");
        assert!(second.replayed);
        assert_eq!(second.response_json, first.response_json);
        assert_eq!(harness.counts()?, after_first);
        Ok(())
    }

    fn envelope(
        request_id: &str,
        idempotency_key: Option<&str>,
        dry_run: bool,
        expected_state_version: Option<u64>,
        task_id: Option<&str>,
    ) -> ToolEnvelope {
        ToolEnvelope {
            project_id: ProjectId::new(PROJECT_ID),
            task_id: task_id.map(TaskId::new),
            actor_kind: ActorKind::Agent,
            surface_id: SurfaceId::new(SURFACE_ID),
            request_id: RequestId::new(request_id),
            idempotency_key: idempotency_key.map(IdempotencyKey::new),
            expected_state_version,
            dry_run,
            locale: None,
        }
    }

    fn invocation(access_class: AccessClass) -> InvocationContext {
        InvocationContext {
            surface_instance_id: Some(SurfaceInstanceId::new(SURFACE_INSTANCE_ID)),
            requested_access_class: access_class,
            invocation_binding_basis: "method_test_invocation".to_owned(),
        }
    }

    fn assert_verified_surface(response: &PipelineResponse, access_class: AccessClass) {
        let verified = response
            .verified_surface
            .as_ref()
            .expect("method response should carry verified surface context");
        assert_eq!(verified.project_id.as_str(), PROJECT_ID);
        assert_eq!(verified.surface_id.as_str(), SURFACE_ID);
        assert_eq!(verified.surface_instance_id.as_str(), SURFACE_INSTANCE_ID);
        assert_eq!(verified.access_class, access_class);
        assert!(verified
            .verification_basis
            .contains("method_test_registration"));
        assert!(verified
            .verification_basis
            .contains("method_test_invocation"));
    }

    fn status_include() -> StatusInclude {
        StatusInclude {
            task: true,
            pending_user_judgments: true,
            write_authority: true,
            evidence: true,
            close: true,
            guarantees: true,
        }
    }

    fn intake_request(
        request_id: &str,
        idempotency_key: &str,
        dry_run: bool,
        expected_state_version: Option<u64>,
        requested_mode: RequestedMode,
    ) -> harness_types::IntakeRequest {
        harness_types::IntakeRequest {
            envelope: envelope(
                request_id,
                Some(idempotency_key),
                dry_run,
                expected_state_version,
                None,
            ),
            plain_language_request: "Create a test export flow.".to_owned(),
            requested_mode,
            resume_policy: ResumePolicy::CreateNew,
            initial_scope: InitialScope {
                boundary: "Initial test scope.".to_owned(),
                non_goals: vec!["Changing unrelated flows.".to_owned()],
                acceptance_criteria: vec!["The test export flow is represented.".to_owned()],
            },
            initial_context_refs: Vec::new(),
        }
    }

    fn update_scope_request(
        request_id: &str,
        idempotency_key: &str,
        dry_run: bool,
        expected_state_version: Option<u64>,
        task_id: &str,
        operation: ChangeUnitOperation,
        scope_summary: &str,
    ) -> UpdateScopeRequest {
        let mut fields = Map::new();
        fields.insert(
            "scope_summary".to_owned(),
            Value::String(scope_summary.to_owned()),
        );
        fields.insert(
            "affected_paths".to_owned(),
            json!(["src/export.rs", "tests/export.rs"]),
        );
        UpdateScopeRequest {
            envelope: envelope(
                request_id,
                Some(idempotency_key),
                dry_run,
                expected_state_version,
                Some(task_id),
            ),
            task_id: TaskId::new(task_id),
            goal_summary: Some(scope_summary.to_owned()),
            scope_update: Some(ScopeUpdate {
                include: vec![scope_summary.to_owned()],
                exclude: vec!["Unrelated behavior.".to_owned()],
            }),
            scope_boundary: Some(scope_summary.to_owned()),
            non_goals: Some(vec!["Unrelated behavior.".to_owned()]),
            acceptance_criteria: Some(vec!["The scoped behavior is represented.".to_owned()]),
            autonomy_boundary: Some("Stay inside the scoped test behavior.".to_owned()),
            baseline_ref: Some(BaselineRef::new("baseline_test")),
            change_unit: ChangeUnitUpdate { operation, fields },
            related_scope_decision_refs: Vec::new(),
        }
    }

    fn prepare_write_request(
        request_id: &str,
        idempotency_key: &str,
        expected_state_version: Option<u64>,
        task_id: Option<&str>,
        change_unit_id: Option<&str>,
    ) -> PrepareWriteRequest {
        PrepareWriteRequest {
            envelope: envelope(
                request_id,
                Some(idempotency_key),
                false,
                expected_state_version,
                task_id,
            ),
            task_id: task_id.map(TaskId::new),
            change_unit_id: change_unit_id.map(ChangeUnitId::new),
            intended_operation: "local_sensitive_step".to_owned(),
            intended_paths: vec!["src/export.rs".to_owned()],
            product_file_write_intended: true,
            sensitive_categories: Vec::new(),
            baseline_ref: BaselineRef::new("baseline_test"),
        }
    }

    fn stage_artifact_request(
        request_id: &str,
        idempotency_key: Option<&str>,
        dry_run: bool,
        expected_state_version: Option<u64>,
        task_id: &str,
    ) -> StageArtifactRequest {
        StageArtifactRequest {
            envelope: envelope(
                request_id,
                idempotency_key,
                dry_run,
                expected_state_version,
                Some(task_id),
            ),
            task_id: TaskId::new(task_id),
            display_name: "trace.log".to_owned(),
            content_type: "text/plain".to_owned(),
            redaction_state: RedactionState::None,
            safe_bytes_or_notice: "staging sample".to_owned(),
            expected_sha256: None,
            expected_size_bytes: None,
            relation_hint: Some("diagnostic_log".to_owned()),
        }
    }

    fn record_run_request(
        request_id: &str,
        idempotency_key: &str,
        dry_run: bool,
        expected_state_version: Option<u64>,
        task_id: &str,
        change_unit_id: &str,
    ) -> RecordRunRequest {
        RecordRunRequest {
            envelope: envelope(
                request_id,
                Some(idempotency_key),
                dry_run,
                expected_state_version,
                Some(task_id),
            ),
            task_id: TaskId::new(task_id),
            change_unit_id: ChangeUnitId::new(change_unit_id),
            kind: harness_types::RunKind::Implementation,
            run_id: None,
            baseline_ref: BaselineRef::new("baseline_test"),
            write_authorization_id: None,
            summary: "Recorded implementation run.".to_owned(),
            observed_changes: ObservedChanges {
                changed_paths: Vec::new(),
                product_file_write_observed: false,
                sensitive_categories: Vec::new(),
                baseline_ref: Some(BaselineRef::new("baseline_test")),
            },
            artifact_inputs: Vec::new(),
            evidence_updates: Vec::new(),
        }
    }

    struct CloseTaskFixture<'a> {
        request_id: &'a str,
        idempotency_key: Option<&'a str>,
        dry_run: bool,
        expected_state_version: Option<u64>,
        task_id: &'a str,
        intent: CloseIntent,
        close_reason: Option<CloseReason>,
        superseding_task_id: Option<&'a str>,
    }

    fn close_task_request(input: CloseTaskFixture<'_>) -> CloseTaskRequest {
        CloseTaskRequest {
            envelope: envelope(
                input.request_id,
                input.idempotency_key,
                input.dry_run,
                input.expected_state_version,
                Some(input.task_id),
            ),
            task_id: TaskId::new(input.task_id),
            intent: input.intent,
            close_reason: input.close_reason,
            superseding_task_id: input.superseding_task_id.map(TaskId::new),
            user_note: Some("Focused close-task test.".to_owned()),
        }
    }

    fn record_close_evidence(
        harness: &MethodHarness,
        task_id: &str,
        change_unit_id: &str,
        expected_state_version: u64,
        suffix: &str,
        supported: bool,
    ) -> Result<u64, Box<dyn Error>> {
        enable_record_run_capabilities(harness)?;
        let request_id = format!("req_close_evidence_{suffix}");
        let idempotency_key = format!("idem_close_evidence_{suffix}");
        let mut request = record_run_request(
            &request_id,
            &idempotency_key,
            false,
            Some(expected_state_version),
            task_id,
            change_unit_id,
        );
        request.evidence_updates = vec![if supported {
            supported_evidence_update("Close claim supported.")
        } else {
            unsupported_evidence_update("Close claim supported.")
        }];
        let response = harness
            .service
            .record_run(request, invocation(AccessClass::RunRecording))?;
        Ok(response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state_version should be present"))
    }

    fn record_final_acceptance(
        harness: &MethodHarness,
        task_id: &str,
        change_unit_id: &str,
        expected_state_version: u64,
        suffix: &str,
    ) -> Result<u64, Box<dyn Error>> {
        let request_id = format!("req_close_final_{suffix}");
        let idempotency_key = format!("idem_close_final_{suffix}");
        let judgment = harness.service.request_user_judgment(
            user_judgment_request(
                &request_id,
                &idempotency_key,
                false,
                Some(expected_state_version),
                task_id,
                Some(change_unit_id),
                JudgmentKind::FinalAcceptance,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let judgment_id = judgment.response_value["user_judgment_ref"]["record_id"]
            .as_str()
            .expect("user judgment ref should be present")
            .to_owned();
        let record_request_id = format!("req_close_final_record_{suffix}");
        let record_idempotency_key = format!("idem_close_final_record_{suffix}");
        let response = harness.service.record_user_judgment(
            record_judgment_request(
                &record_request_id,
                &record_idempotency_key,
                Some(expected_state_version + 1),
                task_id,
                &judgment_id,
                JudgmentKind::FinalAcceptance,
                answer_payload(JudgmentKind::FinalAcceptance),
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        Ok(response.response_value["base"]["state_version"]
            .as_u64()
            .expect("state_version should be present"))
    }

    fn prepare_write_authorization(
        harness: &MethodHarness,
        task_id: &str,
        change_unit_id: &str,
        expected_state_version: u64,
        suffix: &str,
    ) -> Result<String, Box<dyn Error>> {
        let request_id = format!("req_prepare_{suffix}");
        let idempotency_key = format!("idem_prepare_{suffix}");
        let response = harness.service.prepare_write(
            prepare_write_request(
                &request_id,
                &idempotency_key,
                Some(expected_state_version),
                Some(task_id),
                Some(change_unit_id),
            ),
            invocation(AccessClass::WriteAuthorization),
        )?;
        assert_eq!(response.response_value["decision"], "allowed");
        Ok(
            response.response_value["write_authorization_ref"]["record_id"]
                .as_str()
                .expect("write authorization ref should be present")
                .to_owned(),
        )
    }

    fn stage_artifact_for_record_run(
        harness: &MethodHarness,
        task_id: &str,
        suffix: &str,
        expected_state_version: u64,
    ) -> Result<StagedArtifactHandle, Box<dyn Error>> {
        let request_id = format!("req_stage_{suffix}");
        let idempotency_key = format!("idem_stage_{suffix}");
        let mut request = stage_artifact_request(
            &request_id,
            Some(&idempotency_key),
            false,
            Some(expected_state_version),
            task_id,
        );
        request.display_name = format!("{suffix}.json");
        request.content_type = "application/json".to_owned();
        request.safe_bytes_or_notice = format!("{{\"fixture\":\"{suffix}\"}}");
        let response = harness
            .service
            .stage_artifact(request, invocation(AccessClass::ArtifactRegistration))?;
        Ok(serde_json::from_value(
            response.response_value["staged_artifact_handle"].clone(),
        )?)
    }

    fn artifact_input_for_handle(
        artifact_input_id: &str,
        handle: StagedArtifactHandle,
        relation_hint: Option<&str>,
        claim: Option<&str>,
    ) -> ArtifactInput {
        ArtifactInput {
            artifact_input_id: harness_types::ArtifactInputId::new(artifact_input_id),
            source_kind: ArtifactInputSourceKind::StagedArtifact,
            staged_artifact_handle: Some(handle.clone()),
            existing_artifact_ref: None,
            relation_hint: relation_hint.map(str::to_owned),
            claim: claim.map(str::to_owned),
            expected_sha256: Some(handle.sha256),
            expected_size_bytes: Some(handle.size_bytes),
            redaction_state: Some(handle.redaction_state),
        }
    }

    fn supported_evidence_update(claim: &str) -> EvidenceCoverageItem {
        EvidenceCoverageItem {
            claim: claim.to_owned(),
            required_for_close: true,
            coverage_state: EvidenceCoverageState::Supported,
            supporting_refs: Vec::new(),
            supporting_artifact_refs: Vec::new(),
            gap_refs: Vec::new(),
        }
    }

    fn unsupported_evidence_update(claim: &str) -> EvidenceCoverageItem {
        EvidenceCoverageItem {
            claim: claim.to_owned(),
            required_for_close: true,
            coverage_state: EvidenceCoverageState::Unsupported,
            supporting_refs: Vec::new(),
            supporting_artifact_refs: Vec::new(),
            gap_refs: Vec::new(),
        }
    }

    fn enable_record_run_capabilities(harness: &MethodHarness) -> Result<(), Box<dyn Error>> {
        set_surface_capability(
            harness,
            &json!({
                "access_class": "run_recording",
                "supported_access_classes": [
                    "write_authorization",
                    "artifact_registration",
                    "run_recording"
                ],
                "manual_artifact_attachment_supported": true
            })
            .to_string(),
        )
    }

    fn assert_close_blocker(response_value: &Value, code: &str) {
        let codes = close_blocker_codes(response_value);
        assert!(
            codes.iter().any(|candidate| candidate == code),
            "expected close blocker code {code}, got {codes:?}"
        );
    }

    fn assert_no_close_blocker(response_value: &Value, code: &str) {
        let codes = close_blocker_codes(response_value);
        assert!(
            codes.iter().all(|candidate| candidate != code),
            "did not expect close blocker code {code}, got {codes:?}"
        );
    }

    fn close_blocker_codes(response_value: &Value) -> Vec<String> {
        response_value["blockers"]
            .as_array()
            .expect("blockers should be an array")
            .iter()
            .filter_map(|blocker| blocker["code"].as_str().map(str::to_owned))
            .collect()
    }

    fn assert_prepare_reason(response_value: &Value, code: &str) {
        let reasons = response_value["write_decision_reasons"]
            .as_array()
            .expect("write_decision_reasons should be an array");
        assert!(
            reasons.iter().any(|reason| reason["code"] == code),
            "expected prepare_write reason code {code}, got {reasons:?}"
        );
    }

    fn create_task_with_change_unit(
        harness: &MethodHarness,
        prefix: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        let intake_request_id = format!("req_{prefix}_task");
        let intake_idempotency_key = format!("idem_{prefix}_task");
        let intake = harness.service.intake(
            intake_request(
                &intake_request_id,
                &intake_idempotency_key,
                false,
                Some(0),
                RequestedMode::Work,
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let task_id = intake.response_value["task_ref"]["record_id"]
            .as_str()
            .expect("task ref should be present")
            .to_owned();

        let scope_request_id = format!("req_{prefix}_scope");
        let scope_idempotency_key = format!("idem_{prefix}_scope");
        let scope = harness.service.update_scope(
            update_scope_request(
                &scope_request_id,
                &scope_idempotency_key,
                false,
                Some(1),
                &task_id,
                ChangeUnitOperation::CreateCurrent,
                "Initial current scope.",
            ),
            invocation(AccessClass::CoreMutation),
        )?;
        let change_unit_id = scope.response_value["change_unit_ref"]["record_id"]
            .as_str()
            .expect("change unit ref should be present")
            .to_owned();
        Ok((task_id, change_unit_id))
    }

    #[derive(Debug, PartialEq)]
    struct TaskTerminalFields {
        lifecycle_phase: String,
        result: Option<String>,
        close_summary: Value,
        closed_at: Option<String>,
    }

    fn task_terminal_fields(
        harness: &MethodHarness,
        task_id: &str,
    ) -> Result<TaskTerminalFields, Box<dyn Error>> {
        let conn = harness.conn()?;
        let (lifecycle_phase, result, close_summary_text, closed_at): (
            String,
            Option<String>,
            String,
            Option<String>,
        ) = conn.query_row(
            "SELECT lifecycle_phase, result, close_summary_json, closed_at
               FROM tasks
              WHERE project_id = ?1
                AND task_id = ?2",
            rusqlite::params![PROJECT_ID, task_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        Ok(TaskTerminalFields {
            lifecycle_phase,
            result,
            close_summary: serde_json::from_str(&close_summary_text)?,
            closed_at,
        })
    }

    fn insert_superseding_task(
        harness: &MethodHarness,
        task_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = harness.conn()?;
        conn.execute(
            "INSERT INTO tasks (
                project_id,
                task_id,
                created_by_surface_id,
                created_by_surface_instance_id,
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
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                'work',
                'ready',
                'none',
                'Superseding task',
                'Superseding task',
                '{\"goal_summary\":\"Superseding task\"}',
                '{}',
                '{}',
                '{\"close_reason\":\"none\"}',
                '{}',
                't0',
                't0'
            )",
            rusqlite::params![PROJECT_ID, task_id, SURFACE_ID, SURFACE_INSTANCE_ID],
        )?;
        Ok(())
    }

    fn active_task_id(harness: &MethodHarness) -> Result<Option<String>, Box<dyn Error>> {
        let conn = harness.conn()?;
        Ok(conn.query_row(
            "SELECT active_task_id
               FROM project_state
              WHERE project_id = ?1",
            rusqlite::params![PROJECT_ID],
            |row| row.get(0),
        )?)
    }

    #[derive(Debug, PartialEq)]
    struct StagedArtifactRow {
        created_by_surface_id: String,
        created_by_surface_instance_id: String,
        status: String,
        redaction_state: String,
        tmp_path: String,
        ttl_hours: f64,
    }

    fn enable_stage_artifact_capability(harness: &MethodHarness) -> Result<(), Box<dyn Error>> {
        set_surface_capability(
            harness,
            &json!({
                "access_class": "artifact_registration",
                "supported_access_classes": ["artifact_registration"],
                "manual_artifact_attachment_supported": true
            })
            .to_string(),
        )
    }

    fn staged_artifact_row(
        harness: &MethodHarness,
        handle_id: &str,
    ) -> Result<StagedArtifactRow, Box<dyn Error>> {
        let conn = harness.conn()?;
        Ok(conn.query_row(
            "SELECT
                created_by_surface_id,
                created_by_surface_instance_id,
                status,
                redaction_state,
                tmp_path,
                (julianday(expires_at) - julianday(created_at)) * 24.0
             FROM artifact_staging
             WHERE project_id = ?1
               AND handle_id = ?2",
            rusqlite::params![PROJECT_ID, handle_id],
            |row| {
                Ok(StagedArtifactRow {
                    created_by_surface_id: row.get(0)?,
                    created_by_surface_instance_id: row.get(1)?,
                    status: row.get(2)?,
                    redaction_state: row.get(3)?,
                    tmp_path: row.get(4)?,
                    ttl_hours: row.get(5)?,
                })
            },
        )?)
    }

    fn user_judgment_request(
        request_id: &str,
        idempotency_key: &str,
        dry_run: bool,
        expected_state_version: Option<u64>,
        task_id: &str,
        change_unit_id: Option<&str>,
        judgment_kind: JudgmentKind,
    ) -> harness_types::RequestUserJudgmentRequest {
        harness_types::RequestUserJudgmentRequest {
            envelope: envelope(
                request_id,
                Some(idempotency_key),
                dry_run,
                expected_state_version,
                Some(task_id),
            ),
            task_id: TaskId::new(task_id),
            change_unit_id: change_unit_id.map(ChangeUnitId::new),
            judgment_kind,
            presentation: harness_types::JudgmentPresentation::Short,
            question: "Choose the focused test judgment outcome.".to_owned(),
            options: vec![
                UserJudgmentOption {
                    option_id: harness_types::UserJudgmentOptionId::new("accept"),
                    label: "Accept".to_owned(),
                    description: "Record the focused user-owned judgment.".to_owned(),
                    consequence: "Only this judgment record is resolved.".to_owned(),
                    is_default: true,
                },
                UserJudgmentOption {
                    option_id: harness_types::UserJudgmentOptionId::new("decline"),
                    label: "Decline".to_owned(),
                    description: "Record that the focused judgment was not accepted.".to_owned(),
                    consequence: "The Task remains unresolved for this question.".to_owned(),
                    is_default: false,
                },
            ],
            context: UserJudgmentContext {
                summary: "A focused test judgment needs a user-owned answer.".to_owned(),
                related_refs: Vec::new(),
                artifact_refs: Vec::new(),
                visible_risks: Vec::new(),
                constraints: vec!["The answer covers only the requested judgment kind.".to_owned()],
            },
            affected_refs: vec![StateRecordRef {
                record_kind: StateRecordKind::Task,
                record_id: RecordId::new(task_id),
                project_id: ProjectId::new(PROJECT_ID),
                task_id: Some(TaskId::new(task_id)),
                state_version: expected_state_version,
            }],
            required_for: harness_types::JudgmentRequiredFor::Close,
            expires_at: None,
        }
    }

    fn record_judgment_request(
        request_id: &str,
        idempotency_key: &str,
        expected_state_version: Option<u64>,
        task_id: &str,
        user_judgment_id: &str,
        judgment_kind: JudgmentKind,
        answer: RecordUserJudgmentPayload,
    ) -> RecordUserJudgmentRequest {
        let mut request_envelope = envelope(
            request_id,
            Some(idempotency_key),
            false,
            expected_state_version,
            Some(task_id),
        );
        request_envelope.actor_kind = ActorKind::User;
        RecordUserJudgmentRequest {
            envelope: request_envelope,
            user_judgment_id: harness_types::UserJudgmentId::new(user_judgment_id),
            judgment_kind,
            selected_option_id: harness_types::UserJudgmentOptionId::new("accept"),
            answer,
            note: Some("Recorded by the focused judgment test.".to_owned()),
            accepted_risks: Vec::new(),
        }
    }

    fn answer_payload(judgment_kind: JudgmentKind) -> RecordUserJudgmentPayload {
        let mut payload = RecordUserJudgmentPayload {
            product_decision: None,
            technical_decision: None,
            scope_decision: None,
            sensitive_action_scope: None,
            final_acceptance: None,
            residual_risk_acceptance: None,
            cancellation: None,
        };
        match judgment_kind {
            JudgmentKind::ProductDecision => {
                payload.product_decision = Some(json_object(json!({
                    "judgment": {
                        "decision": "accepted",
                        "rationale": "The product direction is accepted for this focused test."
                    }
                })));
            }
            JudgmentKind::TechnicalDecision => {
                payload.technical_decision = Some(json_object(json!({
                    "judgment": {
                        "decision": "accepted",
                        "rationale": "The technical direction is accepted for this focused test."
                    }
                })));
            }
            JudgmentKind::ScopeDecision => {
                payload.scope_decision = Some(json_object(json!({
                    "requested_scope_summary": "Expanded scope that must not apply silently.",
                    "decision": "accepted"
                })));
            }
            JudgmentKind::SensitiveApproval => {
                payload.sensitive_action_scope = Some(harness_types::SensitiveActionScope {
                    action_kind: "local_sensitive_step".to_owned(),
                    description: "Allow the named sensitive step only.".to_owned(),
                    intended_paths: vec!["src/export.rs".to_owned()],
                    sensitive_categories: vec!["network".to_owned()],
                    command_or_tool_summary: Some("Run a local diagnostic command.".to_owned()),
                    network_or_host_summary: Some("No remote host is authorized here.".to_owned()),
                    secret_or_credential_summary: None,
                    capability_claim: "This is not Write Authorization.".to_owned(),
                    expires_at: None,
                });
            }
            JudgmentKind::FinalAcceptance => {
                payload.final_acceptance = Some(json_object(json!({
                    "judgment": {
                        "decision": "accepted",
                        "basis": "The visible close basis is acceptable."
                    }
                })));
            }
            JudgmentKind::ResidualRiskAcceptance => {
                payload.residual_risk_acceptance = Some(json_object(json!({
                    "risk_id": "risk_visible_001",
                    "decision": "accepted"
                })));
            }
            JudgmentKind::Cancellation => {
                payload.cancellation = Some(json_object(json!({
                    "decision": "cancel",
                    "reason": "The user chose to stop the Task."
                })));
            }
        }
        payload
    }

    fn json_object(value: Value) -> JsonObject {
        match value {
            Value::Object(object) => object,
            _ => panic!("test helper expected a JSON object"),
        }
    }

    fn insert_active_write_authorization(
        harness: &MethodHarness,
        task_id: &str,
        change_unit_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = harness.conn()?;
        conn.execute(
            "INSERT INTO write_authorizations (
                project_id,
                write_authorization_id,
                task_id,
                change_unit_id,
                basis_state_version,
                status,
                attempt_scope_json,
                created_by_surface_id,
                created_by_surface_instance_id,
                expires_at,
                created_at
            )
            VALUES (
                ?1,
                'wa_replace',
                ?2,
                ?3,
                2,
                'active',
                '{\"intended_paths\":[\"src/export.rs\"]}',
                ?4,
                ?5,
                '2999-01-01T00:00:00Z',
                't0'
            )",
            rusqlite::params![
                PROJECT_ID,
                task_id,
                change_unit_id,
                SURFACE_ID,
                SURFACE_INSTANCE_ID
            ],
        )?;
        Ok(())
    }

    fn set_surface_capability(
        harness: &MethodHarness,
        capability_profile_json: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = harness.conn()?;
        conn.execute(
            "UPDATE surfaces
                SET capability_profile_json = ?3
              WHERE project_id = ?1
                AND surface_id = ?2",
            rusqlite::params![PROJECT_ID, SURFACE_ID, capability_profile_json],
        )?;
        Ok(())
    }

    fn set_surface_local_access(
        harness: &MethodHarness,
        local_access: Value,
    ) -> Result<(), Box<dyn Error>> {
        let conn = harness.conn()?;
        conn.execute(
            "UPDATE surfaces
                SET local_access_json = ?3
              WHERE project_id = ?1
                AND surface_id = ?2",
            rusqlite::params![PROJECT_ID, SURFACE_ID, local_access.to_string()],
        )?;
        Ok(())
    }

    fn write_authorization_count(harness: &MethodHarness) -> Result<u64, Box<dyn Error>> {
        let conn = harness.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*)
               FROM write_authorizations
              WHERE project_id = ?1",
            rusqlite::params![PROJECT_ID],
            |row| row.get(0),
        )?;
        Ok(u64::try_from(count)?)
    }

    fn write_authorization_basis(
        harness: &MethodHarness,
        write_authorization_id: &str,
    ) -> Result<u64, Box<dyn Error>> {
        let conn = harness.conn()?;
        let basis: i64 = conn.query_row(
            "SELECT basis_state_version
               FROM write_authorizations
              WHERE project_id = ?1
                AND write_authorization_id = ?2",
            rusqlite::params![PROJECT_ID, write_authorization_id],
            |row| row.get(0),
        )?;
        Ok(u64::try_from(basis)?)
    }

    fn user_judgment_status(
        harness: &MethodHarness,
        user_judgment_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        let conn = harness.conn()?;
        Ok(conn.query_row(
            "SELECT status
               FROM user_judgments
              WHERE project_id = ?1
                AND judgment_id = ?2",
            rusqlite::params![PROJECT_ID, user_judgment_id],
            |row| row.get(0),
        )?)
    }

    fn resolution_json(
        harness: &MethodHarness,
        user_judgment_id: &str,
    ) -> Result<Value, Box<dyn Error>> {
        let conn = harness.conn()?;
        let text: String = conn.query_row(
            "SELECT resolution_json
               FROM user_judgments
              WHERE project_id = ?1
                AND judgment_id = ?2",
            rusqlite::params![PROJECT_ID, user_judgment_id],
            |row| row.get(0),
        )?;
        Ok(serde_json::from_str(&text)?)
    }

    fn current_change_unit_id(
        harness: &MethodHarness,
        task_id: &str,
    ) -> Result<Option<String>, Box<dyn Error>> {
        let conn = harness.conn()?;
        Ok(conn.query_row(
            "SELECT current_change_unit_id
               FROM tasks
              WHERE project_id = ?1
                AND task_id = ?2",
            rusqlite::params![PROJECT_ID, task_id],
            |row| row.get(0),
        )?)
    }

    fn current_change_unit_scope(
        harness: &MethodHarness,
        task_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        let conn = harness.conn()?;
        let text: String = conn.query_row(
            "SELECT scope_summary_json
               FROM change_units
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'
                AND is_current = 1",
            rusqlite::params![PROJECT_ID, task_id],
            |row| row.get(0),
        )?;
        let value: Value = serde_json::from_str(&text)?;
        Ok(value["scope_summary"]
            .as_str()
            .expect("scope_summary should be a string")
            .to_owned())
    }

    fn active_current_change_units(
        harness: &MethodHarness,
        task_id: &str,
    ) -> Result<i64, Box<dyn Error>> {
        let conn = harness.conn()?;
        Ok(conn.query_row(
            "SELECT COUNT(*)
               FROM change_units
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'
                AND is_current = 1",
            rusqlite::params![PROJECT_ID, task_id],
            |row| row.get(0),
        )?)
    }

    fn write_authorization_status(
        harness: &MethodHarness,
        write_authorization_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        let conn = harness.conn()?;
        Ok(conn.query_row(
            "SELECT status
               FROM write_authorizations
              WHERE project_id = ?1
                AND write_authorization_id = ?2",
            rusqlite::params![PROJECT_ID, write_authorization_id],
            |row| row.get(0),
        )?)
    }

    fn artifact_staging_status(
        harness: &MethodHarness,
        handle_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        let conn = harness.conn()?;
        Ok(conn.query_row(
            "SELECT status
               FROM artifact_staging
              WHERE project_id = ?1
                AND handle_id = ?2",
            rusqlite::params![PROJECT_ID, handle_id],
            |row| row.get(0),
        )?)
    }

    fn expire_staged_artifact(
        harness: &MethodHarness,
        handle_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = harness.conn()?;
        conn.execute(
            "UPDATE artifact_staging
                SET expires_at = '2000-01-01T00:00:00.000Z'
              WHERE project_id = ?1
                AND handle_id = ?2",
            rusqlite::params![PROJECT_ID, handle_id],
        )?;
        Ok(())
    }

    fn artifact_owner_link_exists(
        harness: &MethodHarness,
        artifact_id: &str,
        owner_record_kind: &str,
    ) -> Result<bool, Box<dyn Error>> {
        let conn = harness.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*)
               FROM artifact_links
              WHERE project_id = ?1
                AND artifact_id = ?2
                AND owner_record_kind = ?3",
            rusqlite::params![PROJECT_ID, artifact_id, owner_record_kind],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}
