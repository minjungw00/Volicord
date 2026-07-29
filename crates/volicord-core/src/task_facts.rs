use crate::pipeline::{CorePipelineError, CoreResult};
use crate::record_refs::stored_refs_to_state_refs;
use volicord_store::core_pipeline::CoreProjectStore;
use volicord_types::ids::TaskId;
use volicord_types::schema::{CurrentCloseBasis, StateRecordRef};

pub(crate) fn active_blocker_refs(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
) -> CoreResult<Vec<StateRecordRef>> {
    Ok(stored_refs_to_state_refs(
        store
            .active_blocker_refs(task_id, state_version)
            .map_err(CorePipelineError::from)?,
    ))
}

pub(crate) fn current_close_basis(
    store: &CoreProjectStore,
    task_id: &TaskId,
) -> CoreResult<Option<CurrentCloseBasis>> {
    Ok(store
        .task_revision_record(task_id)
        .map_err(CorePipelineError::from)?
        .and_then(|record| record.current_close_basis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_fact_owner_exposes_only_typed_store_reads() {
        let _: fn(&CoreProjectStore<'_>, &TaskId, u64) -> CoreResult<Vec<StateRecordRef>> =
            active_blocker_refs;
        let _: fn(&CoreProjectStore<'_>, &TaskId) -> CoreResult<Option<CurrentCloseBasis>> =
            current_close_basis;
    }
}
