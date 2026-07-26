/// Public record reference facts read from storage rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRecordRef {
    pub record_kind: String,
    pub record_id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub state_version: Option<u64>,
}
