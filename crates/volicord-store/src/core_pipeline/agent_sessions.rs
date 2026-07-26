use super::{facade::CoreProjectStore, validation::validate_identifier};
use crate::{
    guards::{agent_session_from_conn, AgentSessionRecord},
    StoreResult,
};

impl CoreProjectStore<'_> {
    /// Reads one Agent Session through this handle's current SQLite snapshot.
    pub fn agent_session(&self, session_id: &str) -> StoreResult<Option<AgentSessionRecord>> {
        validate_identifier("session_id", session_id)?;
        agent_session_from_conn(&self.conn, &self.project.project_id, session_id)
    }
}
