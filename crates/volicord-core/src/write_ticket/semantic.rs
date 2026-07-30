use volicord_store::core_pipeline::StoredWriteTicket;
use volicord_types::ids::{ProjectId, RunId, WriteTicketId};
use volicord_types::product_path::WriteTicketPathScope;
use volicord_types::schema::{WriteTicketAttemptScope, WriteTicketValidityBasis};
use volicord_types::values::{UtcTimestamp, WriteTicketInvalidationReason, WriteTicketStatus};

use super::planning::PlannedWriteTicket;

/// Immutable Write Ticket meaning shared by planned and stored forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteTicketSemanticFacts {
    project_id: ProjectId,
    basis_state_version: u64,
    validity_basis: WriteTicketValidityBasis,
    path_scope: WriteTicketPathScope,
    attempt_scope: WriteTicketAttemptScope,
    idle_expires_at: Option<UtcTimestamp>,
}

impl WriteTicketSemanticFacts {
    pub(crate) fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub(crate) fn basis_state_version(&self) -> u64 {
        self.basis_state_version
    }

    pub(crate) fn validity_basis(&self) -> &WriteTicketValidityBasis {
        &self.validity_basis
    }

    pub(crate) fn path_scope(&self) -> &WriteTicketPathScope {
        &self.path_scope
    }

    pub(crate) fn attempt_scope(&self) -> &WriteTicketAttemptScope {
        &self.attempt_scope
    }

    pub(crate) fn idle_expires_at(&self) -> Option<&UtcTimestamp> {
        self.idle_expires_at.as_ref()
    }
}

/// Store-validated ticket facts after physical representation has been removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredWriteTicketFacts {
    write_ticket_id: WriteTicketId,
    ticket: WriteTicketSemanticFacts,
    status: WriteTicketStatus,
    invalidation_reason: Option<WriteTicketInvalidationReason>,
    consumed_by_run_id: Option<RunId>,
}

impl StoredWriteTicketFacts {
    pub(crate) fn from_record(record: &StoredWriteTicket) -> Self {
        Self {
            write_ticket_id: WriteTicketId::new(record.write_ticket_id()),
            ticket: WriteTicketSemanticFacts {
                project_id: ProjectId::new(record.project_id()),
                basis_state_version: record.basis_state_version(),
                validity_basis: record.validity_basis().clone(),
                path_scope: record.path_scope().clone(),
                attempt_scope: record.attempt_scope().clone(),
                idle_expires_at: record.idle_expires_at().cloned(),
            },
            status: record.status(),
            invalidation_reason: record.invalidation_reason(),
            consumed_by_run_id: record.consumed_by_run_id().map(RunId::new),
        }
    }

    #[cfg(test)]
    pub(crate) fn write_ticket_id(&self) -> &WriteTicketId {
        &self.write_ticket_id
    }

    #[cfg(test)]
    pub(crate) fn semantic_facts(&self) -> &WriteTicketSemanticFacts {
        &self.ticket
    }

    pub(crate) fn into_lifecycle_parts(
        self,
    ) -> (
        WriteTicketId,
        WriteTicketSemanticFacts,
        WriteTicketStatus,
        Option<WriteTicketInvalidationReason>,
        Option<RunId>,
    ) {
        (
            self.write_ticket_id,
            self.ticket,
            self.status,
            self.invalidation_reason,
            self.consumed_by_run_id,
        )
    }
}

pub(crate) fn planned_write_ticket_semantic_facts(
    plan: &PlannedWriteTicket,
) -> WriteTicketSemanticFacts {
    WriteTicketSemanticFacts {
        project_id: plan.project_id().clone(),
        basis_state_version: plan.basis_state_version(),
        validity_basis: plan.validity_basis().clone(),
        path_scope: plan.path_scope().clone(),
        attempt_scope: plan.attempt_scope().clone(),
        idle_expires_at: plan.idle_expires_at().cloned(),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use volicord_types::ids::{BaselineRef, ChangeUnitId, ProjectId, RunId, TaskId, WriteTicketId};
    use volicord_types::product_path::{ProductRelativePath, WriteTicketPathScope};
    use volicord_types::schema::{WriteTicketAttemptScope, WriteTicketValidityBasis};
    use volicord_types::values::{UtcTimestamp, WriteTicketInvalidationReason, WriteTicketStatus};

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
            path_scope: WriteTicketPathScope::new(vec![intended_path.clone()], Vec::new())
                .expect("test scope should be valid"),
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
        stored_facts_from_semantic(write_ticket_id, status, semantic_facts(basis_state_version))
    }

    pub(crate) fn stored_facts_from_semantic(
        write_ticket_id: &str,
        status: WriteTicketStatus,
        ticket: WriteTicketSemanticFacts,
    ) -> StoredWriteTicketFacts {
        StoredWriteTicketFacts {
            write_ticket_id: WriteTicketId::new(write_ticket_id),
            ticket,
            status,
            invalidation_reason: None,
            consumed_by_run_id: None,
        }
    }

    pub(crate) fn invalidated_facts(
        write_ticket_id: &str,
        invalidation: WriteTicketInvalidationReason,
        basis_state_version: u64,
    ) -> StoredWriteTicketFacts {
        let mut facts = stored_facts(
            write_ticket_id,
            WriteTicketStatus::Invalidated,
            basis_state_version,
        );
        facts.invalidation_reason = Some(invalidation);
        facts
    }

    pub(crate) fn consumed_facts(
        write_ticket_id: &str,
        run_id: &str,
        basis_state_version: u64,
    ) -> StoredWriteTicketFacts {
        let mut facts = stored_facts(
            write_ticket_id,
            WriteTicketStatus::Consumed,
            basis_state_version,
        );
        facts.consumed_by_run_id = Some(RunId::new(run_id));
        facts
    }

    pub(crate) fn revoked_facts(
        write_ticket_id: &str,
        invalidation: WriteTicketInvalidationReason,
        basis_state_version: u64,
    ) -> StoredWriteTicketFacts {
        let mut facts = stored_facts(
            write_ticket_id,
            WriteTicketStatus::Revoked,
            basis_state_version,
        );
        facts.invalidation_reason = Some(invalidation);
        facts
    }
}

#[cfg(test)]
mod tests {
    use volicord_types::values::WriteTicketStatus;

    use super::test_support::stored_facts;

    #[test]
    fn stored_identity_is_mandatory_and_distinct_from_shared_ticket_meaning() {
        let stored = stored_facts("ticket-stored", WriteTicketStatus::Active, 7);
        let shared_meaning = stored.semantic_facts().clone();

        assert_eq!(stored.semantic_facts(), &shared_meaning);
        assert_eq!(stored.write_ticket_id().as_str(), "ticket-stored");
    }
}
