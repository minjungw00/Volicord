//! Current-state diagnostic activation, resolution, and reactivation.

use rusqlite::params;
use volicord_types::{
    CurrentDiagnosticFinding, CurrentDiagnosticKey, CurrentDiagnosticStatus, UtcTimestamp,
};

use crate::{
    sqlite::{begin_immediate_transaction, open_registry_database_for_mutation},
    RuntimeHomeMutationContext, StoreError, StoreResult,
};

use super::{
    graph::validate_current_references,
    row::{
        corrupt_value, replace_current_snapshot, stored_finding_from_conn, PreparedFinding,
        StoredFinding,
    },
};

/// Inserts or refreshes only the replaceable snapshot for one current key.
///
/// The key derives the finding ID. Existing identity fields are compared and
/// never updated. A successful refresh always leaves the condition active and
/// atomically replaces its outgoing causes.
pub fn upsert_current_snapshot(
    context: &RuntimeHomeMutationContext<'_>,
    finding: &CurrentDiagnosticFinding,
) -> StoreResult<CurrentDiagnosticFinding> {
    if finding.snapshot().status() != CurrentDiagnosticStatus::Active
        || finding.snapshot().resolved_at().is_some()
    {
        return Err(StoreError::InvalidInput {
            detail: "current diagnostic upsert requires an active snapshot".to_owned(),
        });
    }
    let prepared = PreparedFinding::current(finding)?;
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    validate_current_references(&tx, finding, &prepared)?;
    replace_current_snapshot(&tx, &prepared)?;
    tx.commit()?;
    Ok(finding.clone())
}

/// Marks one current condition resolved and removes current actions and causes.
pub fn resolve_current_finding(
    context: &RuntimeHomeMutationContext<'_>,
    key: &CurrentDiagnosticKey,
    resolved_at: UtcTimestamp,
) -> StoreResult<CurrentDiagnosticFinding> {
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let finding_id = key.finding_id();
    let stored = stored_finding_from_conn(&tx, finding_id.as_str())?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "diagnostic_finding",
            id: finding_id.to_string(),
        }
    })?;
    let StoredFinding::Current(existing) = stored else {
        return Err(StoreError::Conflict {
            entity: "diagnostic_finding",
            id: finding_id.to_string(),
            detail: "diagnostic finding is not a current-state record".to_owned(),
        });
    };
    if existing.key() != key {
        return Err(StoreError::Conflict {
            entity: "diagnostic_finding",
            id: finding_id.to_string(),
            detail: "current diagnostic identity fields do not match the persisted key".to_owned(),
        });
    }
    if existing.snapshot().status() == CurrentDiagnosticStatus::Resolved {
        tx.commit()?;
        return Ok(existing);
    }

    tx.execute(
        "DELETE FROM diagnostic_cause_edges WHERE finding_id = ?1",
        [finding_id.as_str()],
    )?;
    tx.execute(
        "UPDATE diagnostic_findings
            SET actions_json = '[]',
                current_state_status = 'resolved',
                resolved_at = ?2
          WHERE finding_id = ?1 AND lifecycle = 'current_state'",
        params![finding_id.as_str(), resolved_at.to_canonical_string()],
    )?;
    let resolved = stored_finding_from_conn(&tx, finding_id.as_str())?
        .and_then(|finding| match finding {
            StoredFinding::Current(current) => Some(current),
            StoredFinding::Occurrence(_) => None,
        })
        .ok_or_else(|| corrupt_value(finding_id.as_str(), "lifecycle"))?;
    tx.commit()?;
    Ok(resolved)
}
