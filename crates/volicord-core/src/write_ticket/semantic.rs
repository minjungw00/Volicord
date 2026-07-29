use volicord_store::core_pipeline::StoredWriteTicket;
use volicord_types::ids::{ProjectId, RunId, WriteTicketId};
use volicord_types::product_path::ProductRelativePath;
use volicord_types::schema::{WriteTicketAttemptScope, WriteTicketValidityBasis};
use volicord_types::values::{UtcTimestamp, WriteTicketInvalidationReason, WriteTicketStatus};

use super::planning::PlannedWriteTicket;

/// Immutable Write Ticket meaning shared by planned and stored forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteTicketSemanticFacts {
    pub(crate) project_id: ProjectId,
    pub(crate) basis_state_version: u64,
    pub(crate) validity_basis: WriteTicketValidityBasis,
    pub(crate) allowed_path_prefixes: Vec<ProductRelativePath>,
    pub(crate) denied_path_prefixes: Vec<ProductRelativePath>,
    pub(crate) attempt_scope: WriteTicketAttemptScope,
    pub(crate) idle_expires_at: Option<UtcTimestamp>,
}

/// Store-validated ticket facts after physical representation has been removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredWriteTicketFacts {
    pub(crate) write_ticket_id: WriteTicketId,
    pub(crate) ticket: WriteTicketSemanticFacts,
    pub(crate) status: WriteTicketStatus,
    pub(crate) invalidation_reason: Option<WriteTicketInvalidationReason>,
    pub(crate) consumed_by_run_id: Option<RunId>,
}

impl StoredWriteTicketFacts {
    pub(crate) fn from_record(record: &StoredWriteTicket) -> Self {
        Self {
            write_ticket_id: WriteTicketId::new(record.write_ticket_id()),
            ticket: WriteTicketSemanticFacts {
                project_id: ProjectId::new(record.project_id()),
                basis_state_version: record.basis_state_version(),
                validity_basis: record.validity_basis().clone(),
                allowed_path_prefixes: record.allowed_path_prefixes().to_vec(),
                denied_path_prefixes: record.denied_path_prefixes().to_vec(),
                attempt_scope: record.attempt_scope().clone(),
                idle_expires_at: record.idle_expires_at().cloned(),
            },
            status: record.status(),
            invalidation_reason: record.invalidation_reason(),
            consumed_by_run_id: record.consumed_by_run_id().map(RunId::new),
        }
    }
}

pub(crate) fn planned_write_ticket_semantic_facts(
    plan: &PlannedWriteTicket,
) -> WriteTicketSemanticFacts {
    WriteTicketSemanticFacts {
        project_id: plan.project_id().clone(),
        basis_state_version: plan.basis_state_version(),
        validity_basis: plan.validity_basis().clone(),
        allowed_path_prefixes: plan.allowed_path_prefixes().to_vec(),
        denied_path_prefixes: plan.denied_path_prefixes().to_vec(),
        attempt_scope: plan.attempt_scope().clone(),
        idle_expires_at: plan.idle_expires_at().cloned(),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use volicord_types::ids::{BaselineRef, ChangeUnitId, ProjectId, TaskId, WriteTicketId};
    use volicord_types::product_path::ProductRelativePath;
    use volicord_types::schema::{WriteTicketAttemptScope, WriteTicketValidityBasis};
    use volicord_types::values::{UtcTimestamp, WriteTicketStatus};

    use super::{StoredWriteTicketFacts, WriteTicketSemanticFacts};

    pub(crate) fn timestamp(value: &str) -> UtcTimestamp {
        UtcTimestamp::parse(value).expect("test timestamp should be valid")
    }

    pub(crate) fn semantic_facts(basis_state_version: u64) -> WriteTicketSemanticFacts {
        let task_id = TaskId::new("task-test");
        let change_unit_id = ChangeUnitId::new("change-test");
        let baseline_ref = BaselineRef::new("baseline-test");
        let intended_path = ProductRelativePath::parse("src").expect("test path should be valid");
        WriteTicketSemanticFacts {
            project_id: ProjectId::new("project-test"),
            basis_state_version,
            validity_basis: WriteTicketValidityBasis {
                task_id: task_id.clone(),
                change_unit_id: change_unit_id.clone(),
                scope_revision: 3,
                baseline_ref: Some(baseline_ref.clone()),
                workspace_context_sha256: None,
                write_authority_fingerprint: format!("sha256:{}", "0".repeat(64)),
                approval_basis_refs: Vec::new(),
            },
            allowed_path_prefixes: vec![intended_path.clone()],
            denied_path_prefixes: Vec::new(),
            attempt_scope: WriteTicketAttemptScope {
                task_id,
                change_unit_id,
                intended_operation: "edit".to_owned(),
                intended_paths: vec![intended_path],
                product_file_write_intended: true,
                sensitive_categories: Vec::new(),
                baseline_ref: Some(baseline_ref),
            },
            idle_expires_at: Some(timestamp("2026-07-29T00:15:00Z")),
        }
    }

    pub(crate) fn stored_facts(
        write_ticket_id: &str,
        status: WriteTicketStatus,
        basis_state_version: u64,
    ) -> StoredWriteTicketFacts {
        StoredWriteTicketFacts {
            write_ticket_id: WriteTicketId::new(write_ticket_id),
            ticket: semantic_facts(basis_state_version),
            status,
            invalidation_reason: None,
            consumed_by_run_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use volicord_types::values::WriteTicketStatus;

    use super::test_support::stored_facts;

    #[test]
    fn stored_identity_is_mandatory_and_distinct_from_shared_ticket_meaning() {
        let stored = stored_facts("ticket-stored", WriteTicketStatus::Active, 7);
        let shared_meaning = stored.ticket.clone();

        assert_eq!(stored.ticket, shared_meaning);
        assert_eq!(stored.write_ticket_id.as_str(), "ticket-stored");
    }
}
