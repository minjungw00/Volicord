use volicord_types::{
    ids::TaskId,
    values::{UserActionKind, UtcTimestamp},
};

use super::{
    AcceptanceCriterionRecord, ChangeUnitRecord, CoreProjectStore, EffectiveUserActionRecord,
    EvidenceClaimRecord, StoredArtifactRecord, StoredRecordRef, TaskRevisionRecord,
};
use crate::{artifacts::PersistentArtifactVerification, StoreResult};

/// Focused typed Store reads used by UserAction semantic services.
pub trait UserActionStoreReader {
    fn task_revision_record(&self, task_id: &TaskId) -> StoreResult<Option<TaskRevisionRecord>>;

    fn change_unit_record(
        &self,
        task_id: &TaskId,
        change_unit_id: &str,
    ) -> StoreResult<Option<ChangeUnitRecord>>;

    fn acceptance_criterion_record(
        &self,
        acceptance_criterion_id: &str,
    ) -> StoreResult<Option<AcceptanceCriterionRecord>>;

    fn evidence_claim_record(
        &self,
        task_id: &TaskId,
        evidence_claim_id: &str,
    ) -> StoreResult<Option<EvidenceClaimRecord>>;

    fn artifact_record(&self, artifact_id: &str) -> StoreResult<Option<StoredArtifactRecord>>;

    fn artifact_has_task_owner_link(&self, artifact_id: &str, task_id: &str) -> StoreResult<bool>;

    fn verify_persistent_artifact_body(
        &self,
        record: &StoredArtifactRecord,
    ) -> StoreResult<PersistentArtifactVerification>;

    fn resolved_user_action_records(
        &self,
        task_id: &TaskId,
        action_kind: UserActionKind,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>>;

    fn pending_user_action_records(
        &self,
        task_id: &TaskId,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>>;

    fn user_action_records_for_task(
        &self,
        task_id: &TaskId,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>>;

    fn pending_user_action_refs(
        &self,
        task_id: &TaskId,
        state_version: u64,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<StoredRecordRef>>;
}

impl UserActionStoreReader for CoreProjectStore<'_> {
    fn task_revision_record(&self, task_id: &TaskId) -> StoreResult<Option<TaskRevisionRecord>> {
        CoreProjectStore::task_revision_record(self, task_id)
    }

    fn change_unit_record(
        &self,
        task_id: &TaskId,
        change_unit_id: &str,
    ) -> StoreResult<Option<ChangeUnitRecord>> {
        CoreProjectStore::change_unit_record(self, task_id, change_unit_id)
    }

    fn acceptance_criterion_record(
        &self,
        acceptance_criterion_id: &str,
    ) -> StoreResult<Option<AcceptanceCriterionRecord>> {
        CoreProjectStore::acceptance_criterion_record(self, acceptance_criterion_id)
    }

    fn evidence_claim_record(
        &self,
        task_id: &TaskId,
        evidence_claim_id: &str,
    ) -> StoreResult<Option<EvidenceClaimRecord>> {
        CoreProjectStore::evidence_claim_record(self, task_id, evidence_claim_id)
    }

    fn artifact_record(&self, artifact_id: &str) -> StoreResult<Option<StoredArtifactRecord>> {
        CoreProjectStore::artifact_record(self, artifact_id)
    }

    fn artifact_has_task_owner_link(&self, artifact_id: &str, task_id: &str) -> StoreResult<bool> {
        CoreProjectStore::artifact_has_task_owner_link(self, artifact_id, task_id)
    }

    fn verify_persistent_artifact_body(
        &self,
        record: &StoredArtifactRecord,
    ) -> StoreResult<PersistentArtifactVerification> {
        CoreProjectStore::verify_persistent_artifact_body(self, record)
    }

    fn resolved_user_action_records(
        &self,
        task_id: &TaskId,
        action_kind: UserActionKind,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>> {
        CoreProjectStore::resolved_user_action_records(self, task_id, action_kind, now)
    }

    fn pending_user_action_records(
        &self,
        task_id: &TaskId,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>> {
        CoreProjectStore::pending_user_action_records(self, task_id, now)
    }

    fn user_action_records_for_task(
        &self,
        task_id: &TaskId,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>> {
        CoreProjectStore::user_action_records_for_task(self, task_id, now)
    }

    fn pending_user_action_refs(
        &self,
        task_id: &TaskId,
        state_version: u64,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<StoredRecordRef>> {
        CoreProjectStore::pending_user_action_refs(self, task_id, state_version, now)
    }
}
