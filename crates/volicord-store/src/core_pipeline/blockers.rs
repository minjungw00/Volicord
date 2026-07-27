use rusqlite::params;
use volicord_types::{ids::TaskId, values::StateRecordKind};

use super::{facade::CoreProjectStore, record_refs::StoredRecordRef};
use crate::StoreResult;

impl CoreProjectStore<'_> {
    /// Lists active blocker refs for a Task.
    pub fn active_blocker_refs(
        &self,
        task_id: &TaskId,
        state_version: u64,
    ) -> StoreResult<Vec<StoredRecordRef>> {
        let mut stmt = self.conn.prepare(
            "SELECT blocker_id
               FROM blockers
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'
              ORDER BY blocker_id",
        )?;
        let rows = stmt.query_map(params![self.project.project_id, task_id.as_str()], |row| {
            row.get::<_, String>(0)
        })?;
        let mut refs = Vec::new();
        for row in rows {
            refs.push(StoredRecordRef {
                record_kind: StateRecordKind::Blocker,
                record_id: row?,
                project_id: self.project.project_id.clone(),
                task_id: Some(task_id.as_str().to_owned()),
                state_version: Some(state_version),
            });
        }
        Ok(refs)
    }
}
