use crate::pipeline::VerifiedInvocationContext;
use crate::policy::workflow::ProjectWorkflowPolicy;
use volicord_store::core_pipeline::{
    ArtifactMutation, ChangeUnitRecord, CoreProjectStore, CoreStorageMutation, EvidenceMutation,
    ProjectStateHeader, RunMutation, TaskMutation, TaskRecord, UserActionMutation,
    WriteTicketMutation, WriteTicketRecord,
};
use volicord_store::evidence_capture::EvidenceCaptureReceiptRecord;
use volicord_types::ids::{AgentConnectionId, RunId};
use volicord_types::schema::{
    AcceptanceCriterion, ArtifactRef, CurrentCloseBasis, EvidenceCaptureIntent,
    EvidenceObservation, EvidenceProducer, EvidenceSummary, EvidenceTarget, JsonObject,
    ObservedChanges, StateRecordRef, WriteTicketAttemptScope,
};
use volicord_types::values::{
    ActorSource, EvidenceAssuranceLevel, EvidenceProducerKind, EvidenceRelevanceStatus,
    EvidenceSourceKind, UtcTimestamp,
};
use volicord_user_action_service::UserActionAuthority;

use super::RecordRunInput;

pub(super) struct RecordRunRawRequest {
    pub(super) request: RecordRunInput,
    pub(super) plan_now: UtcTimestamp,
}

impl RecordRunRawRequest {
    pub(super) fn new(request: RecordRunInput, operation_now: &UtcTimestamp) -> Self {
        Self {
            request,
            plan_now: operation_now.clone(),
        }
    }
}

pub(super) struct RecordRunNormalizedRequest {
    pub(super) raw: RecordRunRawRequest,
    pub(super) planned_state_version: u64,
    pub(super) normalized_changed_paths: Vec<String>,
    pub(super) normalized_observed_changes: ObservedChanges,
}

pub(super) struct RecordRunFacts {
    pub(super) normalized: RecordRunNormalizedRequest,
    pub(super) task: TaskRecord,
    pub(super) change_unit: ChangeUnitRecord,
    pub(super) workflow_policy: ProjectWorkflowPolicy,
    pub(super) resolved_control: crate::policy::workflow::ResolvedTaskControlAuthority,
}

pub(super) struct RecordRunPolicyDecision {
    pub(super) facts: RecordRunFacts,
    pub(super) write_ticket_scope: Option<(WriteTicketRecord, WriteTicketAttemptScope)>,
    pub(super) run_id: RunId,
    pub(super) run_ref: StateRecordRef,
}

pub(super) struct RecordRunPlannedMutations {
    pub(super) request: RecordRunInput,
    pub(super) plan_now: UtcTimestamp,
    pub(super) planned_state_version: u64,
    pub(super) change_unit: ChangeUnitRecord,
    pub(super) write_ticket_scope: Option<(WriteTicketRecord, WriteTicketAttemptScope)>,
    pub(super) run_id: RunId,
    pub(super) run_ref: StateRecordRef,
    pub(super) normalized_observed_changes: ObservedChanges,
    pub(super) registered_artifacts: Vec<ArtifactRef>,
    pub(super) evidence_observations: Vec<EvidenceObservation>,
    pub(super) observation_refs: Vec<StateRecordRef>,
    pub(super) evidence_producers: Vec<EvidenceProducer>,
    pub(super) acceptance_criteria: Vec<AcceptanceCriterion>,
    pub(super) recorded_evidence_summary: Option<EvidenceSummary>,
    pub(super) projected_close_evidence_summary: Option<EvidenceSummary>,
    pub(super) projected_state_evidence_summary: Option<EvidenceSummary>,
    pub(super) current_close_basis: Option<CurrentCloseBasis>,
    pub(super) blocker_refs: Vec<StateRecordRef>,
    pub(super) pending_user_action_refs: Vec<StateRecordRef>,
    pub(super) pending_authorities: Vec<UserActionAuthority>,
    pub(super) projected_task: TaskRecord,
    pub(super) mutation_plan: RecordRunMutationPlan,
    pub(super) event_payload: JsonObject,
}

pub(super) struct RecordRunMutationAssembly<'a> {
    pub(super) request: &'a RecordRunInput,
    pub(super) task: &'a TaskRecord,
    pub(super) workflow_policy: &'a ProjectWorkflowPolicy,
    pub(super) write_ticket_scope: Option<&'a (WriteTicketRecord, WriteTicketAttemptScope)>,
    pub(super) run_id: &'a RunId,
    pub(super) normalized_observed_changes: &'a ObservedChanges,
    pub(super) close_basis_revision: u64,
    pub(super) close_basis: Option<CurrentCloseBasis>,
    pub(super) lifecycle_phase: Option<volicord_types::values::TaskLifecyclePhase>,
    pub(super) sensitive_category_acceptance_update: Option<TaskMutation>,
    pub(super) evidence_claim_mutations: Vec<EvidenceMutation>,
    pub(super) artifact_plans: &'a [RecordRunArtifactPlan],
    pub(super) observation_plans: &'a [RecordRunObservationPlan],
    pub(super) recorded_evidence_summary: Option<&'a EvidenceSummary>,
    pub(super) evidence_summary_id: Option<&'a String>,
    pub(super) registered_artifacts: &'a [ArtifactRef],
    pub(super) verified_invocation: &'a VerifiedInvocationContext,
}

pub(super) struct RecordRunArtifactPlan {
    pub(super) artifact_ref: ArtifactRef,
    pub(super) evidence_target: Option<EvidenceTarget>,
    pub(super) source_mutation: Option<ArtifactMutation>,
    pub(super) run_link: ArtifactMutation,
}

pub(super) struct RecordRunObservationPlan {
    pub(super) observation: EvidenceObservation,
    pub(super) observation_ref: StateRecordRef,
    pub(super) mutation: EvidenceMutation,
    pub(super) producer: Option<EvidenceProducer>,
    pub(super) producer_mutation: Option<EvidenceMutation>,
}

pub(super) struct RecordRunEvidenceTargetPlan {
    pub(super) claim_mutations: Vec<EvidenceMutation>,
}

pub(super) struct RecordRunMutationPlan {
    pub(super) steps: Vec<RecordRunMutation>,
}

pub(super) enum RecordRunMutation {
    Run(RunMutation),
    Task(TaskMutation),
    UserAction(UserActionMutation),
    WriteTicket(WriteTicketMutation),
    Evidence(Box<EvidenceMutation>),
    Artifact(ArtifactMutation),
}

impl RecordRunMutationPlan {
    pub(super) fn into_storage_mutations(self) -> Vec<CoreStorageMutation> {
        self.steps
            .into_iter()
            .map(|mutation| match mutation {
                RecordRunMutation::Run(mutation) => CoreStorageMutation::Run(mutation),
                RecordRunMutation::Task(mutation) => CoreStorageMutation::Task(mutation),
                RecordRunMutation::UserAction(mutation) => {
                    CoreStorageMutation::UserAction(mutation)
                }
                RecordRunMutation::WriteTicket(mutation) => {
                    CoreStorageMutation::WriteTicket(mutation)
                }
                RecordRunMutation::Evidence(mutation) => CoreStorageMutation::Evidence(*mutation),
                RecordRunMutation::Artifact(mutation) => CoreStorageMutation::Artifact(mutation),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub(super) struct RecordRunCaptureAuthority {
    pub(super) intent: EvidenceCaptureIntent,
    pub(super) intent_ref: StateRecordRef,
    pub(super) receipt: EvidenceCaptureReceiptRecord,
    pub(super) producer_kind: EvidenceProducerKind,
    pub(super) source_kind: EvidenceSourceKind,
    pub(super) assurance_level: EvidenceAssuranceLevel,
    pub(super) relevance_status: EvidenceRelevanceStatus,
    pub(super) receipt_artifact_ref: ArtifactRef,
    pub(super) source_refs: Vec<StateRecordRef>,
    pub(super) connection_id: AgentConnectionId,
    pub(super) host_invocation_id: Option<String>,
    pub(super) observed_by_actor_source: ActorSource,
    pub(super) observed_outcome: JsonObject,
    pub(super) limitations: Vec<String>,
    pub(super) observed_at: UtcTimestamp,
    pub(super) tool_name: Option<String>,
    pub(super) verification_basis: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RecordRunObservationOrigin {
    Caller,
    ValidatedReuse,
}

pub(super) struct RecordRunArtifactContext<'a> {
    pub(super) store: &'a CoreProjectStore<'a>,
    pub(super) project_state: &'a ProjectStateHeader,
    pub(super) request: &'a RecordRunInput,
    pub(super) verified_invocation: &'a VerifiedInvocationContext,
    pub(super) run_id: &'a RunId,
    pub(super) run_ref: &'a StateRecordRef,
    pub(super) now: &'a UtcTimestamp,
}
