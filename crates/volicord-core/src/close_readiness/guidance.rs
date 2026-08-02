use volicord_types::schema::{NextActionSummary, RequiredNullable, StateRecordRef};
use volicord_types::values::{MethodName, NextActionKind, OperationCategory};

/// Semantic close-readiness continuation selected from one evaluated condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloseGuidance {
    ReviewCurrentTask,
    ResolveRecoveryBlockers,
    ResolveRecoveryConstraints,
    ReconcileChanges,
    RecordOpenTicket,
    ResolvePendingUserAction,
    RequestCancellationAuthority,
    RestoreActiveChangeUnit,
    PrepareSensitiveAction,
    RequestSensitiveApproval,
    RefreshCurrentBasis,
    RepairArtifact,
    MakeResidualRiskVisible,
    RequestResidualRiskAcceptance,
    RecordCurrentCloseBasis,
    RecordFreshScopeBasis,
    RecordFreshRunBasis,
    RecordRequiredEvidence,
    RequestFinalAcceptance,
}

/// Selects the adapter-neutral method, operation category, and semantic label
/// for one close-readiness continuation.
pub(crate) fn close_guidance(
    guidance: CloseGuidance,
    required_refs: Vec<StateRecordRef>,
) -> NextActionSummary {
    let (action_kind, owner_method, allowed_operation_categories, label, blocking_question) =
        match guidance {
            CloseGuidance::ReviewCurrentTask => (
                NextActionKind::CloseTask,
                Some(MethodName::CloseTask),
                vec![OperationCategory::AgentWorkflow],
                "Review the current Task before closing.",
                None,
            ),
            CloseGuidance::ResolveRecoveryBlockers => (
                NextActionKind::CloseTask,
                Some(MethodName::CloseTask),
                vec![OperationCategory::AgentWorkflow],
                "Resolve recovery blockers before closing the Task.",
                None,
            ),
            CloseGuidance::ResolveRecoveryConstraints => (
                NextActionKind::CloseTask,
                Some(MethodName::CloseTask),
                vec![OperationCategory::AgentWorkflow],
                "Resolve recovery constraints before completing the Task.",
                None,
            ),
            CloseGuidance::ReconcileChanges => (
                NextActionKind::ReconcileChanges,
                Some(MethodName::ReconcileChanges),
                vec![
                    OperationCategory::AgentWorkflow,
                    OperationCategory::LocalRecovery,
                ],
                "Run reconciliation for observed Product Repository changes before close.",
                Some(
                    "Does the user accept any remaining observed Product Repository change as intentional?",
                ),
            ),
            CloseGuidance::RecordOpenTicket => (
                NextActionKind::RecordRun,
                Some(MethodName::RecordRun),
                vec![OperationCategory::AgentWorkflow],
                "Record the ticket-backed run or reconcile observed changes before close.",
                None,
            ),
            CloseGuidance::ResolvePendingUserAction => (
                NextActionKind::ResolveUserAction,
                Some(MethodName::ResolveUserAction),
                vec![OperationCategory::UserOnly],
                "Resolve the pending user action.",
                None,
            ),
            CloseGuidance::RequestCancellationAuthority => (
                NextActionKind::RequestUserAction,
                Some(MethodName::RequestUserAction),
                vec![OperationCategory::AgentWorkflow],
                "Request current user cancellation authority.",
                None,
            ),
            CloseGuidance::RestoreActiveChangeUnit => (
                NextActionKind::UpdateScope,
                Some(MethodName::UpdateScope),
                vec![OperationCategory::AgentWorkflow],
                "Create or restore the current active Change Unit.",
                None,
            ),
            CloseGuidance::PrepareSensitiveAction => (
                NextActionKind::PrepareWrite,
                Some(MethodName::PrepareWrite),
                vec![OperationCategory::AgentWorkflow],
                "Prepare the exact sensitive action with user-owned approval, then record its ticket-backed Run.",
                None,
            ),
            CloseGuidance::RequestSensitiveApproval => (
                NextActionKind::RequestUserAction,
                Some(MethodName::RequestUserAction),
                vec![OperationCategory::AgentWorkflow],
                "Request the user-owned sensitive-action approval.",
                None,
            ),
            CloseGuidance::RefreshCurrentBasis => (
                NextActionKind::UpdateScope,
                Some(MethodName::UpdateScope),
                vec![OperationCategory::AgentWorkflow],
                "Refresh the current scope or close basis before completing the Task.",
                None,
            ),
            CloseGuidance::RepairArtifact => (
                NextActionKind::RecordRun,
                Some(MethodName::RecordRun),
                vec![OperationCategory::AgentWorkflow],
                "Record or repair the artifact supporting close evidence.",
                None,
            ),
            CloseGuidance::MakeResidualRiskVisible => (
                NextActionKind::RequestUserAction,
                Some(MethodName::RequestUserAction),
                vec![OperationCategory::AgentWorkflow],
                "Make residual risk visible before requesting acceptance.",
                None,
            ),
            CloseGuidance::RequestResidualRiskAcceptance => (
                NextActionKind::RequestUserAction,
                Some(MethodName::RequestUserAction),
                vec![OperationCategory::AgentWorkflow],
                "Request current residual-risk acceptance from the user.",
                None,
            ),
            CloseGuidance::RecordCurrentCloseBasis => (
                NextActionKind::RecordRun,
                Some(MethodName::RecordRun),
                vec![OperationCategory::AgentWorkflow],
                "Record the current result and close basis.",
                None,
            ),
            CloseGuidance::RecordFreshScopeBasis => (
                NextActionKind::RecordRun,
                Some(MethodName::RecordRun),
                vec![OperationCategory::AgentWorkflow],
                "Record a fresh close basis for the current scope.",
                None,
            ),
            CloseGuidance::RecordFreshRunBasis => (
                NextActionKind::RecordRun,
                Some(MethodName::RecordRun),
                vec![OperationCategory::AgentWorkflow],
                "Record a fresh close basis for the current Run context.",
                None,
            ),
            CloseGuidance::RecordRequiredEvidence => (
                NextActionKind::RecordRun,
                Some(MethodName::RecordRun),
                vec![OperationCategory::AgentWorkflow],
                "Record evidence that supports the required close claims.",
                None,
            ),
            CloseGuidance::RequestFinalAcceptance => (
                NextActionKind::RequestUserAction,
                Some(MethodName::RequestUserAction),
                vec![OperationCategory::AgentWorkflow],
                "The Agent Connection must create a current final-acceptance request for the user.",
                Some(
                    "Does the user accept the current Task result and close basis as complete?",
                ),
            ),
        };
    NextActionSummary {
        action_kind,
        owner_method,
        allowed_operation_categories,
        label: label.to_owned(),
        blocking_question: blocking_question.map(str::to_owned),
        expected_state_version: RequiredNullable::null(),
        required_refs,
    }
}

#[cfg(test)]
#[path = "tests/guidance.rs"]
mod tests;
