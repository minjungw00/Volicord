use std::collections::BTreeSet;
use volicord_types::ids::{ProjectId, RecordId, TaskId};
use volicord_types::schema::StateRecordRef;
use volicord_types::values::StateRecordKind;

use volicord_store::core_pipeline::{ChangeUnitRecord, StoredRecordRef, WriteTicketRecord};

use crate::policy::evidence::state_record_ref_identity_key;

pub(crate) fn change_unit_ref(
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

pub(crate) fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn unique_state_refs(values: Vec<StateRecordRef>) -> Vec<StateRecordRef> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for value in values {
        let key = state_record_ref_identity_key(&value);
        if seen.insert(key) {
            unique.push(value);
        }
    }
    unique
}

pub(crate) fn state_ref(
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
        task_id: task_id.cloned().into(),
        produced_at_state_version: state_version.into(),
    }
}

pub(crate) fn write_ticket_ref(record: &WriteTicketRecord, state_version: u64) -> StateRecordRef {
    state_ref(
        StateRecordKind::WriteTicket,
        &record.write_ticket_id,
        &ProjectId::new(record.project_id.clone()),
        Some(&TaskId::new(record.task_id.clone())),
        Some(state_version),
    )
}

pub(crate) fn state_ref_from_stored(record: StoredRecordRef) -> StateRecordRef {
    StateRecordRef {
        record_kind: record.record_kind,
        record_id: RecordId::new(record.record_id),
        project_id: ProjectId::new(record.project_id),
        task_id: record.task_id.map(TaskId::new).into(),
        produced_at_state_version: record.state_version.into(),
    }
}

pub(crate) fn stored_refs_to_state_refs(records: Vec<StoredRecordRef>) -> Vec<StateRecordRef> {
    records.into_iter().map(state_ref_from_stored).collect()
}
