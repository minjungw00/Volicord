//! Typed diagnostic finding lookup APIs.

use std::path::Path;

use volicord_types::{
    CurrentDiagnosticFinding, CurrentDiagnosticStatus, DiagnosticFinding, DiagnosticFindingId,
    DiagnosticScope, OccurrenceDiagnosticFinding, StoredDiagnosticFinding, MAX_DIAGNOSTIC_FINDINGS,
};

use crate::{
    sqlite::{open_registry_database_read_only, registry_db_path},
    StoreError, StoreResult,
};

use super::row::{
    corrupt_value, stored_finding_from_conn, stored_finding_query, validate_lookup_id,
    StoredFinding,
};

/// Reads existing lifecycle-aware findings by ID after strict persisted validation.
pub fn stored_diagnostic_findings_by_ids(
    runtime_home: impl AsRef<Path>,
    finding_ids: &[DiagnosticFindingId],
) -> StoreResult<Vec<StoredDiagnosticFinding>> {
    if finding_ids.len() > MAX_DIAGNOSTIC_FINDINGS {
        return Err(StoreError::InvalidInput {
            detail: format!("diagnostic lookup exceeds {MAX_DIAGNOSTIC_FINDINGS} finding IDs"),
        });
    }
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    let mut ids = finding_ids.to_vec();
    ids.sort();
    ids.dedup();
    let mut findings = Vec::new();
    for finding_id in ids {
        if let Some(stored) = stored_finding_from_conn(&conn, finding_id.as_str())? {
            findings.push(stored);
        }
    }
    Ok(findings)
}

/// Reads one exact lifecycle-aware finding by ID.
pub fn stored_diagnostic_finding_by_id(
    runtime_home: impl AsRef<Path>,
    finding_id: &DiagnosticFindingId,
) -> StoreResult<Option<StoredDiagnosticFinding>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    stored_finding_from_conn(&conn, finding_id.as_str())
}

/// Reads findings eligible for a current report by explicit ID.
///
/// Immutable occurrences and active current-state findings are eligible.
/// Resolved current-state findings remain available through exact historical
/// lookup but are deliberately absent from this current-report selection.
pub fn reportable_diagnostic_findings_by_ids(
    runtime_home: impl AsRef<Path>,
    finding_ids: &[DiagnosticFindingId],
) -> StoreResult<Vec<DiagnosticFinding>> {
    if finding_ids.len() > MAX_DIAGNOSTIC_FINDINGS {
        return Err(StoreError::InvalidInput {
            detail: format!("diagnostic lookup exceeds {MAX_DIAGNOSTIC_FINDINGS} finding IDs"),
        });
    }
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    let mut ids = finding_ids.to_vec();
    ids.sort();
    ids.dedup();
    let mut findings = Vec::new();
    for finding_id in ids {
        let Some(stored) = stored_finding_from_conn(&conn, finding_id.as_str())? else {
            continue;
        };
        match stored {
            StoredFinding::Occurrence(occurrence) => {
                findings.push(occurrence.to_diagnostic_finding());
            }
            StoredFinding::Current(current)
                if current.snapshot().status() == CurrentDiagnosticStatus::Active =>
            {
                findings.push(current.to_diagnostic_finding());
            }
            StoredFinding::Current(_) => {}
        }
    }
    Ok(findings)
}

/// Reads immutable occurrences for one runtime session in observation/ID order.
pub fn diagnostic_occurrences_for_runtime_session(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
) -> StoreResult<Vec<OccurrenceDiagnosticFinding>> {
    validate_lookup_id("runtime_session_id", runtime_session_id)?;
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    stored_finding_query(
        &conn,
        "WHERE lifecycle = 'occurrence' AND runtime_session_id = ?1
         ORDER BY observed_at, finding_id",
        [runtime_session_id],
    )?
    .into_iter()
    .map(|finding| match finding {
        StoredFinding::Occurrence(occurrence) => Ok(occurrence),
        StoredFinding::Current(current) => Err(corrupt_value(current.id().as_str(), "lifecycle")),
    })
    .collect()
}

/// Reads active current findings for one exact diagnostic scope.
pub fn active_current_findings_for_scope(
    runtime_home: impl AsRef<Path>,
    scope: &DiagnosticScope,
) -> StoreResult<Vec<CurrentDiagnosticFinding>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    stored_finding_query(
        &conn,
        "WHERE lifecycle = 'current_state'
           AND current_state_status = 'active'
           AND diagnostic_scope_kind = ?1
           AND diagnostic_scope_identity = ?2
         ORDER BY observed_at, finding_id",
        [scope.kind().as_str(), scope.identity()],
    )?
    .into_iter()
    .map(|finding| match finding {
        StoredFinding::Current(current) => Ok(current),
        StoredFinding::Occurrence(occurrence) => Err(corrupt_value(
            occurrence.occurrence_id().as_str(),
            "lifecycle",
        )),
    })
    .collect()
}
