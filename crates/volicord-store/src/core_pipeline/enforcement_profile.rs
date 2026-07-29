use rusqlite::{params, OptionalExtension};
use volicord_types::schema::ProjectEnforcementProfile;

use super::{facade::CoreProjectStore, validation::validate_project_enforcement_profile};
use crate::{StoreError, StoreResult};

const PROJECT_ENFORCEMENT_PROFILE_COLUMNS: &str = "
    project_id, enforcement_profile_json";

/// Strict-decoded project-owned enforcement profile row.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectEnforcementProfileRecord {
    pub project_id: String,
    pub profile: ProjectEnforcementProfile,
}

struct ProjectEnforcementProfileRow {
    project_id: String,
    enforcement_profile_json: String,
}

impl CoreProjectStore<'_> {
    /// Reads and strictly validates the active project enforcement profile.
    pub fn project_enforcement_profile(&self) -> StoreResult<ProjectEnforcementProfileRecord> {
        let sql = format!(
            "SELECT {PROJECT_ENFORCEMENT_PROFILE_COLUMNS}
               FROM project_state
              WHERE project_id = ?1"
        );
        let row = self
            .conn
            .query_row(
                &sql,
                params![self.project.project_id],
                project_enforcement_profile_row,
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound {
                entity: "project_state",
                id: self.project.project_id.clone(),
            })?;
        decode_project_enforcement_profile(row)
    }
}

fn project_enforcement_profile_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProjectEnforcementProfileRow> {
    Ok(ProjectEnforcementProfileRow {
        project_id: row.get(0)?,
        enforcement_profile_json: row.get(1)?,
    })
}

fn decode_project_enforcement_profile(
    row: ProjectEnforcementProfileRow,
) -> StoreResult<ProjectEnforcementProfileRecord> {
    let profile = serde_json::from_str::<ProjectEnforcementProfile>(&row.enforcement_profile_json)
        .map_err(|_| {
            StoreError::corrupt_owner_state_json(
                "project_state",
                row.project_id.clone(),
                "enforcement_profile_json",
            )
        })?;
    validate_project_enforcement_profile(&profile, &row.project_id)?;
    Ok(ProjectEnforcementProfileRecord {
        project_id: row.project_id,
        profile,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforcement_profile_decoder_rejects_malformed_json() {
        let error = decode_project_enforcement_profile(ProjectEnforcementProfileRow {
            project_id: "project".to_owned(),
            enforcement_profile_json: "{".to_owned(),
        })
        .expect_err("malformed profile JSON must fail closed");

        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateJson {
                table: "project_state",
                logical_column: "enforcement_profile_json",
                ..
            }
        ));
    }
}
