use std::path::Path;

use rusqlite::Transaction;

use super::{
    ArtifactMutation, ChangeUnitMutation, CommittedMutationFacts, ContinuityMutation,
    EvidenceMutation, RunMutation, ShapingCheckpointMutation, TaskMutation, UserActionMutation,
    WriteTicketMutation,
};
use crate::{
    workflow_records::{ProjectWorkflowPolicyMutationEffect, WorkflowPolicyMutation},
    StoreResult,
};

/// Validates that a mutation batch contains the aggregate operation owned by
/// one admitted transition effect. Ancillary aggregate mutations are allowed,
/// but cannot substitute for the transition's primary effect.
pub fn transition_effect_matches_mutations(
    action_key: volicord_types::schema::WorkflowActionKey,
    effect: volicord_types::values::WorkflowTransitionEffectClass,
    mutations: &[CoreStorageMutation],
) -> bool {
    use volicord_types::values::{
        MethodName, WorkflowActionSemanticVariant, WorkflowTransitionEffectClass,
    };

    let has_task = mutations
        .iter()
        .any(|mutation| matches!(mutation, CoreStorageMutation::Task(_)));
    let has_change_unit = mutations
        .iter()
        .any(|mutation| matches!(mutation, CoreStorageMutation::ChangeUnit(_)));
    let has_run = mutations
        .iter()
        .any(|mutation| matches!(mutation, CoreStorageMutation::Run(_)));
    let has_shaping = mutations
        .iter()
        .any(|mutation| matches!(mutation, CoreStorageMutation::Shaping(_)));
    let has_evidence = mutations
        .iter()
        .any(|mutation| matches!(mutation, CoreStorageMutation::Evidence(_)));
    let has_user_action = mutations
        .iter()
        .any(|mutation| matches!(mutation, CoreStorageMutation::UserAction(_)));
    let has_continuity = mutations
        .iter()
        .any(|mutation| matches!(mutation, CoreStorageMutation::Continuity(_)));

    match (effect, action_key.method, action_key.semantic_variant) {
        (
            WorkflowTransitionEffectClass::CoreStateMutation,
            MethodName::RecordShapingCheckpoint,
            WorkflowActionSemanticVariant::CreateInitial
            | WorkflowActionSemanticVariant::ReplaceCurrent,
        ) => has_shaping,
        (
            WorkflowTransitionEffectClass::CoreStateMutation,
            MethodName::UpdateScope,
            WorkflowActionSemanticVariant::KeepCurrentChangeUnit,
        ) => has_task,
        (
            WorkflowTransitionEffectClass::CoreStateMutation,
            MethodName::UpdateScope,
            WorkflowActionSemanticVariant::CreateCurrentChangeUnit
            | WorkflowActionSemanticVariant::ReplaceCurrentChangeUnit,
        ) => has_change_unit,
        (
            WorkflowTransitionEffectClass::CoreStateMutation,
            MethodName::FinalizeAdvice | MethodName::AdvanceTask,
            WorkflowActionSemanticVariant::FinalizeAdvice
            | WorkflowActionSemanticVariant::AdvanceTask,
        ) => has_shaping,
        (
            WorkflowTransitionEffectClass::CoreStateMutation,
            MethodName::RequestUserAction,
            WorkflowActionSemanticVariant::RequestUserAction,
        ) => has_user_action,
        (
            WorkflowTransitionEffectClass::CoreStateMutation,
            MethodName::ReconcileChanges,
            WorkflowActionSemanticVariant::ReconcileChanges,
        ) => has_continuity || has_user_action,
        (
            WorkflowTransitionEffectClass::UserChannelMutation,
            MethodName::ResolveUserAction,
            WorkflowActionSemanticVariant::ResolveUserAction,
        ) => has_user_action,
        (
            WorkflowTransitionEffectClass::WriteAuthorization,
            MethodName::PrepareWrite,
            WorkflowActionSemanticVariant::PrepareWrite,
        ) => mutations.iter().all(|mutation| {
            matches!(
                mutation,
                CoreStorageMutation::Task(_)
                    | CoreStorageMutation::WriteTicket(_)
                    | CoreStorageMutation::WorkflowPolicy(_)
            )
        }),
        (
            WorkflowTransitionEffectClass::EvidenceCapture,
            MethodName::PrepareEvidenceCapture,
            WorkflowActionSemanticVariant::PrepareEvidenceCapture,
        ) => has_evidence,
        (
            WorkflowTransitionEffectClass::ExecutionRecording,
            MethodName::RecordRun,
            WorkflowActionSemanticVariant::RecordRun,
        ) => has_run,
        (
            WorkflowTransitionEffectClass::TerminalMutation,
            MethodName::CloseTask,
            WorkflowActionSemanticVariant::CloseTask,
        ) => has_task,
        (
            WorkflowTransitionEffectClass::ArtifactStaging
            | WorkflowTransitionEffectClass::ReadOnlyAssessment,
            _,
            _,
        ) => false,
        _ => false,
    }
}

/// One responsibility-owned storage mutation in an ordered Core commit.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum CoreStorageMutation {
    Task(TaskMutation),
    ChangeUnit(ChangeUnitMutation),
    WriteTicket(WriteTicketMutation),
    Run(RunMutation),
    Shaping(ShapingCheckpointMutation),
    Evidence(EvidenceMutation),
    Artifact(ArtifactMutation),
    UserAction(UserActionMutation),
    Continuity(ContinuityMutation),
    WorkflowPolicy(WorkflowPolicyMutation),
}

/// Aggregate-specific facts returned while applying an ordered mutation batch.
pub(crate) enum AggregateMutationResult {
    Applied,
    WorkflowPolicy(ProjectWorkflowPolicyMutationEffect),
}

/// Storage application context scoped to one active commit transaction.
pub(crate) struct MutationContext<'tx> {
    pub(super) project_id: &'tx str,
    pub(super) project_home: &'tx Path,
    pub(super) committed_at: &'tx str,
    pub(super) tx: &'tx Transaction<'tx>,
}

impl<'tx> MutationContext<'tx> {
    pub(crate) fn new(
        project_id: &'tx str,
        project_home: &'tx Path,
        committed_at: &'tx str,
        tx: &'tx Transaction<'tx>,
    ) -> Self {
        Self {
            project_id,
            project_home,
            committed_at,
            tx,
        }
    }

    pub(crate) fn project_id(&self) -> &str {
        self.project_id
    }

    pub(crate) fn committed_at(&self) -> &str {
        self.committed_at
    }

    pub(crate) fn transaction(&self) -> &Transaction<'tx> {
        self.tx
    }
}

impl CoreStorageMutation {
    pub(crate) fn apply(
        &self,
        context: &mut MutationContext<'_>,
        facts: &CommittedMutationFacts,
    ) -> StoreResult<AggregateMutationResult> {
        match self {
            Self::Task(mutation) => mutation
                .apply(context)
                .map(|()| AggregateMutationResult::Applied),
            Self::ChangeUnit(mutation) => mutation
                .apply(context, facts.committed_state_version)
                .map(|()| AggregateMutationResult::Applied),
            Self::WriteTicket(mutation) => mutation
                .apply(context, facts.committed_state_version)
                .map(|()| AggregateMutationResult::Applied),
            Self::Run(mutation) => mutation
                .apply(context)
                .map(|()| AggregateMutationResult::Applied),
            Self::Shaping(mutation) => mutation
                .apply(context)
                .map(|()| AggregateMutationResult::Applied),
            Self::Evidence(mutation) => mutation
                .apply(context, facts.committed_state_version)
                .map(|()| AggregateMutationResult::Applied),
            Self::Artifact(mutation) => mutation
                .apply(context)
                .map(|()| AggregateMutationResult::Applied),
            Self::UserAction(mutation) => mutation
                .apply(context)
                .map(|()| AggregateMutationResult::Applied),
            Self::Continuity(mutation) => mutation
                .apply(context)
                .map(|()| AggregateMutationResult::Applied),
            Self::WorkflowPolicy(mutation) => mutation
                .apply(context)
                .map(AggregateMutationResult::WorkflowPolicy),
        }
    }
}

#[cfg(test)]
pub(super) fn with_empty_mutation_context<T>(
    apply: impl FnOnce(&mut MutationContext<'_>) -> T,
) -> T {
    let mut connection =
        rusqlite::Connection::open_in_memory().expect("in-memory database must open");
    let transaction = connection
        .transaction()
        .expect("in-memory transaction must open");
    let mut context = MutationContext::new(
        "project_mutation_test",
        Path::new("/tmp/project_mutation_test"),
        "2026-01-01T00:00:00Z",
        &transaction,
    );
    apply(&mut context)
}
