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
