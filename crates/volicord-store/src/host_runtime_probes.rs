use std::{collections::BTreeSet, path::Path};

use chrono::Duration;
use rusqlite::{params, OptionalExtension};
use serde_json::{Map, Value};
use volicord_types::{
    HostRuntimeProbeFailureClass, HostRuntimeProbeObservation, HostRuntimeProbeOutcome,
    HostRuntimeProbeSnapshot, HOST_RUNTIME_PROBE_SNAPSHOT_SCHEMA,
};

use crate::{
    schema::REGISTRY_DATABASE_KIND,
    sqlite::{
        begin_immediate_transaction, open_registry_database, open_registry_database_read_only,
        registry_db_path,
    },
    StoreError, StoreResult,
};

/// JSON member owned by the runtime-probe store inside a connection verification report.
pub const HOST_RUNTIME_PROBES_REPORT_KEY: &str = "host_runtime_probes";

/// Maximum freshness interval accepted for one current runtime observation.
pub const HOST_RUNTIME_PROBE_MAX_TTL_SECONDS: i64 = 86_400;

const MAX_PROBE_TEXT_BYTES: usize = 1_024;
const MAX_SNAPSHOT_OBSERVATIONS: usize = 14;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectionProbeIdentity {
    host_kind: String,
    managed_fingerprint: String,
    enabled: bool,
    last_verification_report_json: String,
}

/// Reads the current bounded runtime-probe snapshot without creating or writing Registry state.
pub fn host_runtime_probe_snapshot_read_only(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Option<HostRuntimeProbeSnapshot>> {
    validate_bounded_text("connection_internal_id", connection_internal_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(registry_path)?;
    let Some(connection) = connection_probe_identity(&conn, connection_internal_id)? else {
        return Ok(None);
    };
    Ok(Some(host_runtime_probe_snapshot_from_report(
        &connection.last_verification_report_json,
    )?))
}

/// Decodes and validates the store-owned snapshot member from a stored report object.
pub fn host_runtime_probe_snapshot_from_report(
    report_json: &str,
) -> StoreResult<HostRuntimeProbeSnapshot> {
    let report = report_object(report_json)?;
    snapshot_from_object(&report)
}

/// Replaces the same probe/profile slot with a strictly newer bounded observation.
pub fn record_host_runtime_probe_observation(
    runtime_home: impl AsRef<Path>,
    observation: HostRuntimeProbeObservation,
) -> StoreResult<HostRuntimeProbeSnapshot> {
    validate_observation(&observation)?;
    let connection_internal_id = observation.connection_internal_id.clone();
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let connection = connection_probe_identity(&tx, &observation.connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: observation.connection_internal_id.clone(),
        })?;
    if !connection.enabled
        || connection.host_kind != observation.host_kind
        || connection.managed_fingerprint != observation.managed_fingerprint
    {
        return Err(StoreError::InvalidInput {
            detail: "runtime probe must match the current enabled Agent Connection host kind and managed fingerprint".to_owned(),
        });
    }

    let mut report = report_object(&connection.last_verification_report_json)?;
    let mut snapshot = snapshot_from_object(&report)?;
    if let Some(index) = snapshot.observations.iter().position(|current| {
        current.probe_id == observation.probe_id
            && current.adapter_profile == observation.adapter_profile
    }) {
        let current = &snapshot.observations[index];
        if current.observed_at > observation.observed_at {
            return Err(StoreError::Conflict {
                entity: "host_runtime_probe_observation",
                id: observation.probe_id.as_str().to_owned(),
                detail: "a runtime probe slot can be replaced only by a newer observation"
                    .to_owned(),
            });
        }
        if current.observed_at == observation.observed_at {
            if current == &observation {
                tx.commit()?;
                return Ok(snapshot);
            }
            return Err(StoreError::Conflict {
                entity: "host_runtime_probe_observation",
                id: observation.probe_id.as_str().to_owned(),
                detail: "the same runtime probe timestamp is bound to different content".to_owned(),
            });
        }
        snapshot.observations[index] = observation;
    } else {
        snapshot.observations.push(observation);
    }
    snapshot.observations.sort_by(|left, right| {
        left.probe_id.cmp(&right.probe_id).then_with(|| {
            left.adapter_profile
                .as_str()
                .cmp(right.adapter_profile.as_str())
        })
    });
    validate_snapshot(&snapshot, StoreValueKind::Input)?;
    report.insert(
        HOST_RUNTIME_PROBES_REPORT_KEY.to_owned(),
        serde_json::to_value(&snapshot).map_err(|_| StoreError::InvalidInput {
            detail: "runtime probe snapshot could not be serialized".to_owned(),
        })?,
    );
    let report_json =
        serde_json::to_string(&Value::Object(report)).map_err(|_| StoreError::InvalidInput {
            detail: "Agent Connection verification report could not be serialized".to_owned(),
        })?;
    tx.execute(
        "UPDATE agent_connections
            SET last_verification_report_json = ?2
          WHERE connection_internal_id = ?1",
        params![connection_internal_id, report_json],
    )?;
    tx.commit()?;
    Ok(snapshot)
}

/// Preserves the store-owned runtime-probe member across administrative report replacement.
pub(crate) fn preserve_runtime_probe_snapshot(
    existing_report_json: &str,
    replacement_report_json: &str,
) -> StoreResult<String> {
    let existing = report_object(existing_report_json)?;
    let mut replacement = report_object_input(replacement_report_json)?;
    if let Some(snapshot) = existing.get(HOST_RUNTIME_PROBES_REPORT_KEY) {
        let parsed: HostRuntimeProbeSnapshot =
            serde_json::from_value(snapshot.clone()).map_err(|_| {
                StoreError::CorruptStoredJson {
                    database_kind: REGISTRY_DATABASE_KIND,
                    field: "agent_connections.last_verification_report_json.host_runtime_probes",
                }
            })?;
        validate_snapshot(&parsed, StoreValueKind::Stored)?;
        replacement.insert(HOST_RUNTIME_PROBES_REPORT_KEY.to_owned(), snapshot.clone());
    } else {
        replacement.remove(HOST_RUNTIME_PROBES_REPORT_KEY);
    }
    serde_json::to_string(&Value::Object(replacement)).map_err(|_| StoreError::InvalidInput {
        detail: "Agent Connection verification report could not be serialized".to_owned(),
    })
}

fn snapshot_from_object(report: &Map<String, Value>) -> StoreResult<HostRuntimeProbeSnapshot> {
    let Some(value) = report.get(HOST_RUNTIME_PROBES_REPORT_KEY) else {
        return Ok(HostRuntimeProbeSnapshot::default());
    };
    let snapshot: HostRuntimeProbeSnapshot =
        serde_json::from_value(value.clone()).map_err(|_| StoreError::CorruptStoredJson {
            database_kind: REGISTRY_DATABASE_KIND,
            field: "agent_connections.last_verification_report_json.host_runtime_probes",
        })?;
    validate_snapshot(&snapshot, StoreValueKind::Stored)?;
    Ok(snapshot)
}

fn report_object(report_json: &str) -> StoreResult<Map<String, Value>> {
    serde_json::from_str::<Value>(report_json)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .ok_or(StoreError::CorruptStoredJson {
            database_kind: REGISTRY_DATABASE_KIND,
            field: "agent_connections.last_verification_report_json",
        })
}

fn report_object_input(report_json: &str) -> StoreResult<Map<String, Value>> {
    serde_json::from_str::<Value>(report_json)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "last_verification_report_json must be a JSON object".to_owned(),
        })
}

fn validate_observation(observation: &HostRuntimeProbeObservation) -> StoreResult<()> {
    for (field, value) in [
        (
            "runtime_probe.connection_internal_id",
            observation.connection_internal_id.as_str(),
        ),
        ("runtime_probe.host_kind", observation.host_kind.as_str()),
        (
            "runtime_probe.adapter_version",
            observation.adapter_version.as_str(),
        ),
        (
            "runtime_probe.managed_fingerprint",
            observation.managed_fingerprint.as_str(),
        ),
    ] {
        validate_bounded_text(field, value)?;
    }
    for (field, value) in [
        (
            "runtime_probe.host_version",
            observation.host_version.as_deref(),
        ),
        (
            "runtime_probe.client_name",
            observation.client_name.as_deref(),
        ),
        (
            "runtime_probe.client_version",
            observation.client_version.as_deref(),
        ),
    ] {
        if let Some(value) = value {
            validate_bounded_text(field, value)?;
        }
    }
    if observation.client_name.is_some() != observation.client_version.is_some() {
        return Err(StoreError::InvalidInput {
            detail: "runtime probe client_name and client_version must both be present or absent"
                .to_owned(),
        });
    }
    let coherent = matches!(
        (observation.outcome, observation.failure_class),
        (
            HostRuntimeProbeOutcome::Passed,
            HostRuntimeProbeFailureClass::None
        ) | (
            HostRuntimeProbeOutcome::Unsupported,
            HostRuntimeProbeFailureClass::ExplicitCapabilityAbsent
        ) | (
            HostRuntimeProbeOutcome::Unavailable,
            HostRuntimeProbeFailureClass::ProbeNotRun
        )
    ) || matches!(
        observation.outcome,
        HostRuntimeProbeOutcome::Failed | HostRuntimeProbeOutcome::Unavailable
    ) && !matches!(
        observation.failure_class,
        HostRuntimeProbeFailureClass::None
            | HostRuntimeProbeFailureClass::ExplicitCapabilityAbsent
            | HostRuntimeProbeFailureClass::ProbeNotRun
    );
    if !coherent {
        return Err(StoreError::InvalidInput {
            detail: "runtime probe outcome and failure_class are not coherent".to_owned(),
        });
    }
    observation
        .observed_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::InvalidInput {
            detail: "runtime probe observed_at must be canonical RFC 3339 UTC".to_owned(),
        })?;
    observation
        .expires_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::InvalidInput {
            detail: "runtime probe expires_at must be canonical RFC 3339 UTC".to_owned(),
        })?;
    let max_expiry = observation
        .observed_at
        .checked_add(Duration::seconds(HOST_RUNTIME_PROBE_MAX_TTL_SECONDS))
        .map_err(|_| StoreError::InvalidInput {
            detail: "runtime probe expiry is outside the supported timestamp range".to_owned(),
        })?;
    if observation.expires_at <= observation.observed_at || observation.expires_at > max_expiry {
        return Err(StoreError::InvalidInput {
            detail: "runtime probe requires observed_at < expires_at within 24 hours".to_owned(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum StoreValueKind {
    Input,
    Stored,
}

fn validate_snapshot(snapshot: &HostRuntimeProbeSnapshot, kind: StoreValueKind) -> StoreResult<()> {
    let invalid = |detail: String| match kind {
        StoreValueKind::Input => StoreError::InvalidInput { detail },
        StoreValueKind::Stored => StoreError::CorruptStoredValue {
            database_kind: REGISTRY_DATABASE_KIND,
            field: "agent_connections.last_verification_report_json.host_runtime_probes",
        },
    };
    if snapshot.schema != HOST_RUNTIME_PROBE_SNAPSHOT_SCHEMA {
        return Err(invalid(
            "runtime probe snapshot schema is not supported".to_owned(),
        ));
    }
    if snapshot.observations.len() > MAX_SNAPSHOT_OBSERVATIONS {
        return Err(invalid(
            "runtime probe snapshot has too many observations".to_owned(),
        ));
    }
    let mut slots = BTreeSet::new();
    for observation in &snapshot.observations {
        if validate_observation(observation).is_err() {
            return Err(invalid("runtime probe observation is malformed".to_owned()));
        }
        if !slots.insert((observation.probe_id, observation.adapter_profile.as_str())) {
            return Err(invalid(
                "runtime probe snapshot contains a duplicate slot".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_bounded_text(field: &str, value: &str) -> StoreResult<()> {
    if value.is_empty() || value.len() > MAX_PROBE_TEXT_BYTES || value.chars().any(char::is_control)
    {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must be nonempty, content-free bounded text"),
        });
    }
    Ok(())
}

fn connection_probe_identity(
    conn: &rusqlite::Connection,
    connection_internal_id: &str,
) -> StoreResult<Option<ConnectionProbeIdentity>> {
    conn.query_row(
        "SELECT host_kind, managed_fingerprint, enabled, last_verification_report_json
           FROM agent_connections
          WHERE connection_internal_id = ?1",
        [connection_internal_id],
        |row| {
            Ok(ConnectionProbeIdentity {
                host_kind: row.get(0)?,
                managed_fingerprint: row.get(1)?,
                enabled: row.get::<_, i64>(2)? == 1,
                last_verification_report_json: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use volicord_test_support::TempRuntimeHome;
    use volicord_types::{
        HostRuntimeProbeFailureClass, HostRuntimeProbeId, HostRuntimeProbeOutcome,
        IntegrationProfile, UtcTimestamp,
    };

    use super::*;
    use crate::{
        agent_connections::{
            agent_connection_record, ensure_agent_connection,
            update_agent_connection_verification_report, AgentConnectionRegistration,
            CONNECTION_INTENT_PERSONAL, CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_USER,
            VERIFIED_STATUS_COMPLETE,
        },
        bootstrap::initialize_runtime_home,
    };

    #[test]
    fn current_probe_round_trips_and_survives_verification_report_replacement(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = fixture("runtime-probe-roundtrip")?;
        let observation = probe(
            HostRuntimeProbeId::LifecycleHookDelivery,
            HostRuntimeProbeOutcome::Passed,
            HostRuntimeProbeFailureClass::None,
            "2026-07-16T00:00:00Z",
        );
        let snapshot = record_host_runtime_probe_observation(fixture.path(), observation)?;
        assert_eq!(snapshot.observations.len(), 1);
        assert_eq!(
            host_runtime_probe_snapshot_read_only(fixture.path(), "conn_a")?,
            Some(snapshot.clone())
        );

        update_agent_connection_verification_report(
            fixture.path(),
            "conn_a",
            VERIFIED_STATUS_COMPLETE,
            "fingerprint-a",
            r#"{"verification":"fresh"}"#,
            "[]",
        )?;
        let stored =
            agent_connection_record(fixture.path(), "conn_a")?.expect("connection remains present");
        let report: Value = serde_json::from_str(&stored.last_verification_report_json)?;
        assert_eq!(report["verification"], "fresh");
        assert!(report.get(HOST_RUNTIME_PROBES_REPORT_KEY).is_some());
        assert_eq!(
            host_runtime_probe_snapshot_read_only(fixture.path(), "conn_a")?,
            Some(snapshot)
        );
        Ok(())
    }

    #[test]
    fn observation_slots_require_newer_coherent_current_binding() -> Result<(), Box<dyn Error>> {
        let fixture = fixture("runtime-probe-replacement")?;
        let passed = probe(
            HostRuntimeProbeId::PreToolStructuredTargetPaths,
            HostRuntimeProbeOutcome::Passed,
            HostRuntimeProbeFailureClass::None,
            "2026-07-16T00:00:00Z",
        );
        record_host_runtime_probe_observation(fixture.path(), passed.clone())?;
        assert_eq!(
            record_host_runtime_probe_observation(fixture.path(), passed.clone())?
                .observations
                .len(),
            1,
            "an exact retry is idempotent"
        );

        let mut conflict = passed.clone();
        conflict.outcome = HostRuntimeProbeOutcome::Failed;
        conflict.failure_class = HostRuntimeProbeFailureClass::StructuredPathsMissing;
        assert!(matches!(
            record_host_runtime_probe_observation(fixture.path(), conflict),
            Err(StoreError::Conflict { .. })
        ));

        let mut newer = passed;
        newer.outcome = HostRuntimeProbeOutcome::Unavailable;
        newer.failure_class = HostRuntimeProbeFailureClass::ListenerUnavailable;
        newer.observed_at = UtcTimestamp::parse("2026-07-16T00:01:00Z")?;
        newer.expires_at = UtcTimestamp::parse("2026-07-16T01:01:00Z")?;
        let snapshot = record_host_runtime_probe_observation(fixture.path(), newer.clone())?;
        assert_eq!(snapshot.observations, vec![newer]);

        let mut mismatched = probe(
            HostRuntimeProbeId::PostToolStructuredChangedPaths,
            HostRuntimeProbeOutcome::Passed,
            HostRuntimeProbeFailureClass::None,
            "2026-07-16T00:02:00Z",
        );
        mismatched.managed_fingerprint = "old-fingerprint".to_owned();
        assert!(matches!(
            record_host_runtime_probe_observation(fixture.path(), mismatched),
            Err(StoreError::InvalidInput { .. })
        ));
        Ok(())
    }

    #[test]
    fn probe_not_run_is_only_a_coherent_unavailable_observation() -> Result<(), Box<dyn Error>> {
        let unavailable = probe(
            HostRuntimeProbeId::FixedUiAuthorityDisclosure,
            HostRuntimeProbeOutcome::Unavailable,
            HostRuntimeProbeFailureClass::ProbeNotRun,
            "2026-07-16T00:00:00Z",
        );
        assert!(validate_observation(&unavailable).is_ok());

        for outcome in [
            HostRuntimeProbeOutcome::Passed,
            HostRuntimeProbeOutcome::Failed,
            HostRuntimeProbeOutcome::Unsupported,
        ] {
            let mut invalid = unavailable.clone();
            invalid.outcome = outcome;
            assert!(matches!(
                validate_observation(&invalid),
                Err(StoreError::InvalidInput { .. })
            ));
        }
        Ok(())
    }

    fn fixture(name: &str) -> StoreResult<TempRuntimeHome> {
        let runtime_home = TempRuntimeHome::new(name)?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_test", "{}")?;
        ensure_agent_connection(
            runtime_home.path(),
            AgentConnectionRegistration {
                connection_internal_id: "conn_a".to_owned(),
                host_kind: HOST_KIND_CODEX.to_owned(),
                intent: CONNECTION_INTENT_PERSONAL.to_owned(),
                host_scope: HOST_SCOPE_USER.to_owned(),
                server_name: "volicord".to_owned(),
                config_target: "codex-target".to_owned(),
                mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                enabled: true,
                managed_fingerprint: "fingerprint-a".to_owned(),
                last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                last_verification_report_json: "{}".to_owned(),
                last_user_actions_json: "[]".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        Ok(runtime_home)
    }

    fn probe(
        probe_id: HostRuntimeProbeId,
        outcome: HostRuntimeProbeOutcome,
        failure_class: HostRuntimeProbeFailureClass,
        observed_at: &str,
    ) -> HostRuntimeProbeObservation {
        let observed_at = UtcTimestamp::parse(observed_at).expect("valid test timestamp");
        let expires_at = observed_at
            .checked_add(Duration::hours(1))
            .expect("valid test expiry");
        HostRuntimeProbeObservation {
            probe_id,
            outcome,
            failure_class,
            connection_internal_id: "conn_a".to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            host_version: Some("9.9.9".to_owned()),
            client_name: None,
            client_version: None,
            adapter_profile: IntegrationProfile::Detective,
            adapter_version: "test-adapter".to_owned(),
            managed_fingerprint: "fingerprint-a".to_owned(),
            observed_at,
            expires_at,
        }
    }
}
