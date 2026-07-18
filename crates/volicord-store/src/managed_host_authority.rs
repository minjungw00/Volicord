//! Strict current managed-host authority records in `registry.sqlite`.

use std::path::Path;

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use volicord_types::{
    generated_managed_artifacts_digest, require_current_managed_host_binding_external_descriptor,
    ExternalContractDescriptor, GeneratedManagedArtifact, HostVerificationReceipt,
    IntegrationProfile, ManagedHostBinding,
};

use crate::{
    agent_connections::{agent_connection_record, agent_connection_record_from_conn},
    bootstrap::project_record,
    sqlite::{
        begin_immediate_transaction, open_registry_database, open_registry_database_read_only,
        registry_db_path,
    },
    StoreError, StoreResult,
};

/// Complete current authority facts for one managed connection/project pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedHostAuthorityRecord {
    pub connection_internal_id: String,
    pub project_internal_id: String,
    pub external_contract_descriptor: ExternalContractDescriptor,
    pub managed_host_binding: ManagedHostBinding,
    pub binding_digest: String,
    pub generated_artifacts: Vec<GeneratedManagedArtifact>,
    pub generated_artifacts_digest: String,
    pub host_verification_receipt: HostVerificationReceipt,
    pub observed_at: String,
    pub expires_at: String,
    pub updated_at: String,
}

/// Typed replacement input for the current managed-host authority facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedHostAuthorityUpsert {
    pub connection_internal_id: String,
    pub project_internal_id: String,
    pub external_contract_descriptor: ExternalContractDescriptor,
    pub managed_host_binding: ManagedHostBinding,
    pub generated_artifacts: Vec<GeneratedManagedArtifact>,
    pub host_verification_receipt: HostVerificationReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredGeneratedArtifact {
    path: String,
    digest: String,
}

#[derive(Debug)]
struct RawManagedHostAuthorityRecord {
    connection_internal_id: String,
    project_internal_id: String,
    external_contract_descriptor_json: String,
    managed_host_binding_json: String,
    binding_digest: String,
    generated_artifacts_json: String,
    generated_artifacts_digest: String,
    host_verification_receipt_json: String,
    observed_at: String,
    expires_at: String,
    updated_at: String,
}

/// Atomically replaces the current facts after validating their complete typed identity.
pub fn upsert_managed_host_authority(
    runtime_home: impl AsRef<Path>,
    input: ManagedHostAuthorityUpsert,
) -> StoreResult<ManagedHostAuthorityRecord> {
    let runtime_home = runtime_home.as_ref();
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_identifier("project_internal_id", &input.project_internal_id)?;
    validate_input(runtime_home, &input)?;

    let binding_digest = input
        .managed_host_binding
        .binding_digest()
        .map_err(|error| invalid_input(error.reason()))?;
    let generated_artifacts_digest = generated_managed_artifacts_digest(
        input.managed_host_binding.platform_environment,
        &input.generated_artifacts,
    )
    .map_err(|error| invalid_input(error.reason()))?;
    let descriptor_json = serde_json::to_string(&input.external_contract_descriptor)
        .map_err(|error| invalid_input(error.to_string()))?;
    let binding_json = serde_json::to_string(&input.managed_host_binding)
        .map_err(|error| invalid_input(error.to_string()))?;
    let artifacts_json = serde_json::to_string(
        &input
            .generated_artifacts
            .iter()
            .map(|artifact| StoredGeneratedArtifact {
                path: artifact.path.clone(),
                digest: artifact.digest.clone(),
            })
            .collect::<Vec<_>>(),
    )
    .map_err(|error| invalid_input(error.to_string()))?;
    let receipt_json = serde_json::to_string(&input.host_verification_receipt)
        .map_err(|error| invalid_input(error.to_string()))?;
    let observed_at = input
        .host_verification_receipt
        .observed_at
        .to_canonical_string();
    let expires_at = input
        .host_verification_receipt
        .expires_at
        .to_canonical_string();

    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    agent_connection_record_from_conn(&tx, &input.connection_internal_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "agent_connection",
            id: input.connection_internal_id.clone(),
        }
    })?;
    let membership_count: i64 = tx.query_row(
        "SELECT COUNT(*)
           FROM connection_projects
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![input.connection_internal_id, input.project_internal_id],
        |row| row.get(0),
    )?;
    if membership_count != 1 {
        return Err(StoreError::NotFound {
            entity: "connection_project",
            id: format!(
                "{}/{}",
                input.connection_internal_id, input.project_internal_id
            ),
        });
    }
    tx.execute(
        "INSERT INTO managed_host_authority (
            connection_internal_id,
            project_internal_id,
            external_contract_descriptor_json,
            managed_host_binding_json,
            binding_digest,
            generated_artifacts_json,
            generated_artifacts_digest,
            host_verification_receipt_json,
            observed_at,
            expires_at,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                  strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        ON CONFLICT(connection_internal_id, project_internal_id) DO UPDATE SET
            external_contract_descriptor_json = excluded.external_contract_descriptor_json,
            managed_host_binding_json = excluded.managed_host_binding_json,
            binding_digest = excluded.binding_digest,
            generated_artifacts_json = excluded.generated_artifacts_json,
            generated_artifacts_digest = excluded.generated_artifacts_digest,
            host_verification_receipt_json = excluded.host_verification_receipt_json,
            observed_at = excluded.observed_at,
            expires_at = excluded.expires_at,
            updated_at = excluded.updated_at",
        params![
            input.connection_internal_id,
            input.project_internal_id,
            descriptor_json,
            binding_json,
            binding_digest,
            artifacts_json,
            generated_artifacts_digest,
            receipt_json,
            observed_at,
            expires_at,
        ],
    )?;
    tx.commit()?;

    managed_host_authority(
        runtime_home,
        &input.connection_internal_id,
        &input.project_internal_id,
    )?
    .ok_or_else(|| StoreError::NotFound {
        entity: "managed_host_authority",
        id: format!(
            "{}/{}",
            input.connection_internal_id, input.project_internal_id
        ),
    })
}

/// Reads and strict-decodes the current authority facts without writing registry state.
pub fn managed_host_authority_read_only(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
    project_internal_id: &str,
) -> StoreResult<Option<ManagedHostAuthorityRecord>> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    validate_identifier("project_internal_id", project_internal_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(registry_path)?;
    let raw = select_raw(&conn, connection_internal_id, project_internal_id)?;
    if raw.is_some() {
        agent_connection_record_from_conn(&conn, connection_internal_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "agent_connection",
                id: connection_internal_id.to_owned(),
            }
        })?;
    }
    raw.map(decode_record).transpose()
}

pub(crate) fn delete_managed_host_authority_for_membership_in_transaction(
    conn: &rusqlite::Connection,
    connection_internal_id: &str,
    project_internal_id: &str,
) -> StoreResult<usize> {
    conn.execute(
        "DELETE FROM managed_host_authority
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![connection_internal_id, project_internal_id],
    )
    .map_err(StoreError::from)
}

pub(crate) fn delete_managed_host_authority_for_connection_in_transaction(
    conn: &rusqlite::Connection,
    connection_internal_id: &str,
) -> StoreResult<usize> {
    conn.execute(
        "DELETE FROM managed_host_authority
          WHERE connection_internal_id = ?1",
        [connection_internal_id],
    )
    .map_err(StoreError::from)
}

pub(crate) fn delete_managed_host_authority_for_project_in_transaction(
    conn: &rusqlite::Connection,
    project_internal_id: &str,
) -> StoreResult<usize> {
    conn.execute(
        "DELETE FROM managed_host_authority
          WHERE project_internal_id = ?1",
        [project_internal_id],
    )
    .map_err(StoreError::from)
}

fn managed_host_authority(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
    project_internal_id: &str,
) -> StoreResult<Option<ManagedHostAuthorityRecord>> {
    let registry_path = registry_db_path(runtime_home);
    let conn = open_registry_database(registry_path)?;
    let raw = select_raw(&conn, connection_internal_id, project_internal_id)?;
    if raw.is_some() {
        agent_connection_record_from_conn(&conn, connection_internal_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "agent_connection",
                id: connection_internal_id.to_owned(),
            }
        })?;
    }
    raw.map(decode_record).transpose()
}

fn select_raw(
    conn: &rusqlite::Connection,
    connection_internal_id: &str,
    project_internal_id: &str,
) -> StoreResult<Option<RawManagedHostAuthorityRecord>> {
    conn.query_row(
        "SELECT
            connection_internal_id,
            project_internal_id,
            external_contract_descriptor_json,
            managed_host_binding_json,
            binding_digest,
            generated_artifacts_json,
            generated_artifacts_digest,
            host_verification_receipt_json,
            observed_at,
            expires_at,
            updated_at
         FROM managed_host_authority
        WHERE connection_internal_id = ?1
          AND project_internal_id = ?2",
        params![connection_internal_id, project_internal_id],
        |row| {
            Ok(RawManagedHostAuthorityRecord {
                connection_internal_id: row.get(0)?,
                project_internal_id: row.get(1)?,
                external_contract_descriptor_json: row.get(2)?,
                managed_host_binding_json: row.get(3)?,
                binding_digest: row.get(4)?,
                generated_artifacts_json: row.get(5)?,
                generated_artifacts_digest: row.get(6)?,
                host_verification_receipt_json: row.get(7)?,
                observed_at: row.get(8)?,
                expires_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

fn decode_record(raw: RawManagedHostAuthorityRecord) -> StoreResult<ManagedHostAuthorityRecord> {
    let record_ref = format!("{}/{}", raw.connection_internal_id, raw.project_internal_id);
    let descriptor: ExternalContractDescriptor =
        serde_json::from_str(&raw.external_contract_descriptor_json)
            .map_err(|_| corrupt(&record_ref, "external_contract_descriptor_json"))?;
    require_current_managed_host_binding_external_descriptor(&descriptor).map_err(|error| {
        StoreError::UnsupportedExternalContract {
            contract_id: descriptor.contract_id.clone(),
            reason: error.reason(),
        }
    })?;
    let binding: ManagedHostBinding = serde_json::from_str(&raw.managed_host_binding_json)
        .map_err(|_| corrupt(&record_ref, "managed_host_binding_json"))?;
    let stored_artifacts: Vec<StoredGeneratedArtifact> =
        serde_json::from_str(&raw.generated_artifacts_json)
            .map_err(|_| corrupt(&record_ref, "generated_artifacts_json"))?;
    let artifacts = stored_artifacts
        .into_iter()
        .map(|artifact| GeneratedManagedArtifact {
            path: artifact.path,
            digest: artifact.digest,
        })
        .collect::<Vec<_>>();
    let receipt: HostVerificationReceipt =
        serde_json::from_str(&raw.host_verification_receipt_json)
            .map_err(|_| corrupt(&record_ref, "host_verification_receipt_json"))?;
    validate_decoded(DecodedManagedHostAuthority {
        connection_internal_id: &raw.connection_internal_id,
        project_internal_id: &raw.project_internal_id,
        binding: &binding,
        binding_digest: &raw.binding_digest,
        artifacts: &artifacts,
        artifacts_digest: &raw.generated_artifacts_digest,
        receipt: &receipt,
        observed_at: &raw.observed_at,
        expires_at: &raw.expires_at,
    })
    .map_err(|column| corrupt(&record_ref, column))?;
    Ok(ManagedHostAuthorityRecord {
        connection_internal_id: raw.connection_internal_id,
        project_internal_id: raw.project_internal_id,
        external_contract_descriptor: descriptor,
        managed_host_binding: binding,
        binding_digest: raw.binding_digest,
        generated_artifacts: artifacts,
        generated_artifacts_digest: raw.generated_artifacts_digest,
        host_verification_receipt: receipt,
        observed_at: raw.observed_at,
        expires_at: raw.expires_at,
        updated_at: raw.updated_at,
    })
}

fn validate_input(runtime_home: &Path, input: &ManagedHostAuthorityUpsert) -> StoreResult<()> {
    require_current_managed_host_binding_external_descriptor(&input.external_contract_descriptor)
        .map_err(|error| invalid_input(error.reason()))?;
    let connection = agent_connection_record(runtime_home, &input.connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: input.connection_internal_id.clone(),
        })?;
    let project = project_record(runtime_home, &input.project_internal_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "project",
            id: input.project_internal_id.clone(),
        }
    })?;
    input
        .managed_host_binding
        .validate()
        .map_err(|error| invalid_input(error.reason()))?;
    input
        .host_verification_receipt
        .validate_shape()
        .map_err(|error| invalid_input(error.reason()))?;
    let binding_digest = input
        .managed_host_binding
        .binding_digest()
        .map_err(|error| invalid_input(error.reason()))?;
    let artifacts_digest = generated_managed_artifacts_digest(
        input.managed_host_binding.platform_environment,
        &input.generated_artifacts,
    )
    .map_err(|error| invalid_input(error.reason()))?;
    let receipt = &input.host_verification_receipt;
    if input.generated_artifacts.is_empty()
        || connection.host_kind != input.managed_host_binding.host_kind.as_str()
        || connection.intent != input.managed_host_binding.connection_scope.as_str()
        || connection.config_target != input.managed_host_binding.configuration_target.path
        || project.project_id != input.project_internal_id
        || receipt.project_id.as_str() != input.project_internal_id
        || receipt.connection_id.as_str() != input.connection_internal_id
        || receipt.host_kind != input.managed_host_binding.host_kind
        || receipt.integration_profile != IntegrationProfile::Record
        || receipt.platform_environment != input.managed_host_binding.platform_environment
        || receipt.platform_release_coordinate
            != input.managed_host_binding.platform_release_coordinate
        || receipt.required_capabilities != input.managed_host_binding.required_capabilities
        || receipt.verified_capabilities != input.managed_host_binding.required_capabilities
        || receipt.binding_digest != binding_digest
        || receipt.generated_artifacts_digest != artifacts_digest
        || receipt.executable_digest != input.managed_host_binding.process_binding.executable_digest
    {
        return Err(invalid_input("managed_host_authority_identity_mismatch"));
    }
    Ok(())
}

struct DecodedManagedHostAuthority<'a> {
    connection_internal_id: &'a str,
    project_internal_id: &'a str,
    binding: &'a ManagedHostBinding,
    binding_digest: &'a str,
    artifacts: &'a [GeneratedManagedArtifact],
    artifacts_digest: &'a str,
    receipt: &'a HostVerificationReceipt,
    observed_at: &'a str,
    expires_at: &'a str,
}

fn validate_decoded(input: DecodedManagedHostAuthority<'_>) -> Result<(), &'static str> {
    let DecodedManagedHostAuthority {
        connection_internal_id,
        project_internal_id,
        binding,
        binding_digest,
        artifacts,
        artifacts_digest,
        receipt,
        observed_at,
        expires_at,
    } = input;
    binding
        .validate()
        .map_err(|_| "managed_host_binding_json")?;
    receipt
        .validate_shape()
        .map_err(|_| "host_verification_receipt_json")?;
    let actual_binding_digest = binding
        .binding_digest()
        .map_err(|_| "managed_host_binding_json")?;
    let actual_artifacts_digest =
        generated_managed_artifacts_digest(binding.platform_environment, artifacts)
            .map_err(|_| "generated_artifacts_json")?;
    if artifacts.is_empty() || actual_artifacts_digest != artifacts_digest {
        return Err("generated_artifacts_digest");
    }
    if actual_binding_digest != binding_digest {
        return Err("binding_digest");
    }
    if receipt.connection_id.as_str() != connection_internal_id
        || receipt.project_id.as_str() != project_internal_id
        || receipt.host_kind != binding.host_kind
        || receipt.platform_environment != binding.platform_environment
        || receipt.platform_release_coordinate != binding.platform_release_coordinate
        || receipt.required_capabilities != binding.required_capabilities
        || receipt.verified_capabilities != binding.required_capabilities
        || receipt.binding_digest != binding_digest
        || receipt.generated_artifacts_digest != artifacts_digest
        || receipt.executable_digest != binding.process_binding.executable_digest
    {
        return Err("host_verification_receipt_json");
    }
    if receipt.observed_at.to_canonical_string() != observed_at {
        return Err("observed_at");
    }
    if receipt.expires_at.to_canonical_string() != expires_at {
        return Err("expires_at");
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    if value.is_empty()
        || value.len() > 1_024
        || value.chars().all(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} is invalid"),
        });
    }
    Ok(())
}

fn invalid_input(detail: impl Into<String>) -> StoreError {
    StoreError::InvalidInput {
        detail: detail.into(),
    }
}

fn corrupt(record_ref: &str, logical_column: &'static str) -> StoreError {
    StoreError::CorruptOwnerStateJson {
        database_kind: "registry",
        table: "managed_host_authority",
        record_ref: record_ref.to_owned(),
        logical_column,
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use rusqlite::params;
    use volicord_test_support::core_fixtures::CoreFixture;
    use volicord_types::{
        current_managed_host_binding_external_descriptor, generated_managed_artifacts_digest,
        AgentConnectionId, ConfigurationTarget, ConfigurationTargetOwner, EnvironmentForwarding,
        HostKind, HostVerificationResult, ManagedCommand, ManagedCommandResolution,
        ManagedConnectionScope, PlatformEnvironment, PlatformReleaseCoordinate, ProcessBinding,
        ProjectId, UtcTimestamp, FIRST_RELEASE_CODEX_CAPABILITIES,
        HOST_VERIFICATION_RECEIPT_CONTRACT_ID,
    };

    use super::*;
    use crate::agent_connections::{
        ensure_agent_connection, remove_agent_connection_if_unused, remove_connection_project,
        set_connection_enabled, set_connection_mode, AgentConnectionRegistration,
        CONNECTION_MODE_READ_ONLY, CONNECTION_MODE_WORKFLOW,
    };

    const RAW_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const RAW_B: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    const PREFIXED_A: &str =
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn current_authority_round_trips_only_complete_typed_identity() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("managed-host-authority-round-trip")?;
        let input = authority_input(&fixture)?;

        let written = upsert_managed_host_authority(fixture.runtime_home_path(), input.clone())?;
        let read = managed_host_authority_read_only(
            fixture.runtime_home_path(),
            fixture.connection_id(),
            fixture.project_id(),
        )?
        .expect("current authority should exist");

        assert_eq!(written, read);
        assert_eq!(
            read.external_contract_descriptor,
            input.external_contract_descriptor
        );
        assert_eq!(
            read.host_verification_receipt.binding_digest,
            read.binding_digest
        );
        assert_eq!(
            read.host_verification_receipt.generated_artifacts_digest,
            read.generated_artifacts_digest
        );
        Ok(())
    }

    #[test]
    fn unknown_descriptor_and_corrupt_payload_remain_distinct() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("managed-host-authority-strict-read")?;
        upsert_managed_host_authority(fixture.runtime_home_path(), authority_input(&fixture)?)?;
        let conn = rusqlite::Connection::open(fixture.runtime_home_path().join("registry.sqlite"))?;
        let unknown_descriptor = serde_json::json!({
            "contract_id": "volicord.managed-host-binding",
            "schema_digest": format!("sha256:{}", "b".repeat(64)),
            "capabilities": [
                "managed_stdio_mcp",
                "personal_managed_binding",
                "record_workflow",
                "shared_managed_binding"
            ]
        });
        conn.execute(
            "UPDATE managed_host_authority
                SET external_contract_descriptor_json = ?1
              WHERE connection_internal_id = ?2
                AND project_internal_id = ?3",
            params![
                serde_json::to_string(&unknown_descriptor)?,
                fixture.connection_id(),
                fixture.project_id()
            ],
        )?;
        let unsupported = managed_host_authority_read_only(
            fixture.runtime_home_path(),
            fixture.connection_id(),
            fixture.project_id(),
        )
        .unwrap_err();
        assert!(matches!(
            unsupported,
            StoreError::UnsupportedExternalContract {
                reason: "unsupported_external_contract",
                ..
            }
        ));

        conn.execute(
            "UPDATE managed_host_authority
                SET external_contract_descriptor_json = ?1,
                    managed_host_binding_json = '{}'
              WHERE connection_internal_id = ?2
                AND project_internal_id = ?3",
            params![
                serde_json::to_string(&current_managed_host_binding_external_descriptor())?,
                fixture.connection_id(),
                fixture.project_id()
            ],
        )?;
        let corrupt = managed_host_authority_read_only(
            fixture.runtime_home_path(),
            fixture.connection_id(),
            fixture.project_id(),
        )
        .unwrap_err();
        assert!(matches!(
            corrupt,
            StoreError::CorruptOwnerStateJson {
                table: "managed_host_authority",
                logical_column: "managed_host_binding_json",
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn corrupt_parent_connection_json_blocks_authority_reads_and_writes(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("managed-host-authority-parent-corruption")?;
        let input = authority_input(&fixture)?;
        upsert_managed_host_authority(fixture.runtime_home_path(), input.clone())?;
        let conn = rusqlite::Connection::open(fixture.runtime_home_path().join("registry.sqlite"))?;

        for (logical_column, update_sql, damaged) in [
            (
                "verification_report_json",
                "UPDATE agent_connections
                    SET verification_report_json = ?2
                  WHERE connection_internal_id = ?1",
                "[]",
            ),
            (
                "metadata_json",
                "UPDATE agent_connections
                    SET metadata_json = ?2
                  WHERE connection_internal_id = ?1",
                "null",
            ),
        ] {
            conn.execute(update_sql, params![fixture.connection_id(), damaged])?;
            for error in [
                managed_host_authority_read_only(
                    fixture.runtime_home_path(),
                    fixture.connection_id(),
                    fixture.project_id(),
                )
                .expect_err("authority read must reject corrupt parent connection JSON"),
                upsert_managed_host_authority(fixture.runtime_home_path(), input.clone())
                    .expect_err("authority mutation must reject corrupt parent connection JSON"),
            ] {
                assert!(matches!(
                    error,
                    StoreError::CorruptOwnerStateJson {
                        database_kind: "registry",
                        table: "agent_connections",
                        ref record_ref,
                        logical_column: actual_column,
                    } if record_ref == fixture.connection_id() && actual_column == logical_column
                ));
            }
            conn.execute(
                "UPDATE agent_connections
                    SET verification_report_json = NULL,
                        metadata_json = '{}'
                  WHERE connection_internal_id = ?1",
                [fixture.connection_id()],
            )?;
        }
        Ok(())
    }

    #[test]
    fn connection_mutations_invalidate_current_authority() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("managed-host-authority-lifecycle-mutations")?;

        upsert_managed_host_authority(fixture.runtime_home_path(), authority_input(&fixture)?)?;
        set_connection_mode(
            fixture.runtime_home_path(),
            fixture.connection_id(),
            CONNECTION_MODE_WORKFLOW,
        )?;
        assert!(authority_exists(&fixture)?);
        set_connection_mode(
            fixture.runtime_home_path(),
            fixture.connection_id(),
            CONNECTION_MODE_READ_ONLY,
        )?;
        assert!(!authority_exists(&fixture)?);

        upsert_managed_host_authority(fixture.runtime_home_path(), authority_input(&fixture)?)?;
        set_connection_enabled(fixture.runtime_home_path(), fixture.connection_id(), true)?;
        assert!(authority_exists(&fixture)?);
        set_connection_enabled(fixture.runtime_home_path(), fixture.connection_id(), false)?;
        assert!(!authority_exists(&fixture)?);

        upsert_managed_host_authority(fixture.runtime_home_path(), authority_input(&fixture)?)?;
        let connection =
            agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
                .expect("fixture connection should exist");
        ensure_agent_connection(
            fixture.runtime_home_path(),
            AgentConnectionRegistration {
                connection_internal_id: connection.connection_internal_id,
                host_kind: connection.host_kind,
                intent: connection.intent,
                host_scope: connection.host_scope,
                server_name: connection.server_name,
                config_target: connection.config_target,
                mode: connection.mode,
                enabled: connection.enabled,
                managed_fingerprint: connection.managed_fingerprint,
                verification_report_json: connection.verification_report_json,
                metadata_json: connection.metadata_json,
            },
        )?;
        assert!(!authority_exists(&fixture)?);
        Ok(())
    }

    #[test]
    fn membership_removal_cleans_authority_before_parent_records() -> Result<(), Box<dyn Error>> {
        let fixture = CoreFixture::new("managed-host-authority-membership-cleanup")?;
        upsert_managed_host_authority(fixture.runtime_home_path(), authority_input(&fixture)?)?;

        assert!(remove_connection_project(
            fixture.runtime_home_path(),
            fixture.connection_id(),
            fixture.project_id(),
        )?);
        assert!(!authority_exists(&fixture)?);
        assert!(remove_agent_connection_if_unused(
            fixture.runtime_home_path(),
            fixture.connection_id(),
        )?);
        assert!(
            agent_connection_record(fixture.runtime_home_path(), fixture.connection_id(),)?
                .is_none()
        );
        Ok(())
    }

    fn authority_exists(fixture: &CoreFixture) -> StoreResult<bool> {
        managed_host_authority_read_only(
            fixture.runtime_home_path(),
            fixture.connection_id(),
            fixture.project_id(),
        )
        .map(|record| record.is_some())
    }

    fn authority_input(
        fixture: &CoreFixture,
    ) -> Result<ManagedHostAuthorityUpsert, Box<dyn Error>> {
        let connection =
            agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
                .expect("fixture connection should exist");
        let binding = ManagedHostBinding {
            host_kind: HostKind::Codex,
            connection_scope: ManagedConnectionScope::Shared,
            command: ManagedCommand {
                resolution: ManagedCommandResolution::PathLookup,
                program: "volicord".to_owned(),
            },
            arguments: vec![
                "mcp".to_owned(),
                "--stdio".to_owned(),
                "--discover-repository".to_owned(),
                "--host".to_owned(),
                "codex".to_owned(),
            ],
            forwarded_environment: vec![EnvironmentForwarding {
                source_name: "VOLICORD_HOME".to_owned(),
                target_name: "VOLICORD_HOME".to_owned(),
            }],
            configuration_target: ConfigurationTarget {
                owner: ConfigurationTargetOwner::Project,
                path: connection.config_target,
            },
            process_binding: ProcessBinding {
                process_id: 42,
                process_start_token: "linux-proc-start:42".to_owned(),
                platform_instance_token: "linux-boot-id:test".to_owned(),
                executable_path: "/usr/bin/codex".to_owned(),
                executable_digest: RAW_A.to_owned(),
            },
            required_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
            platform_environment: PlatformEnvironment::Linux,
            platform_release_coordinate: PlatformReleaseCoordinate::Native,
        };
        let artifacts = vec![GeneratedManagedArtifact {
            path: binding.configuration_target.path.clone(),
            digest: RAW_B.to_owned(),
        }];
        let receipt = HostVerificationReceipt {
            contract_id: HOST_VERIFICATION_RECEIPT_CONTRACT_ID.to_owned(),
            project_id: ProjectId::new(fixture.project_id()),
            connection_id: AgentConnectionId::new(fixture.connection_id()),
            host_kind: HostKind::Codex,
            integration_profile: IntegrationProfile::Record,
            platform_environment: PlatformEnvironment::Linux,
            platform_release_coordinate: PlatformReleaseCoordinate::Native,
            required_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
            verified_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
            binding_digest: binding.binding_digest()?,
            generated_artifacts_digest: generated_managed_artifacts_digest(
                PlatformEnvironment::Linux,
                &artifacts,
            )?,
            executable_digest: RAW_A.to_owned(),
            policy_digest: PREFIXED_A.to_owned(),
            verifier_build_digest: RAW_B.to_owned(),
            observed_at: UtcTimestamp::parse("2026-07-17T01:00:00Z")?,
            expires_at: UtcTimestamp::parse("2026-07-17T01:05:00Z")?,
            result: HostVerificationResult::Verified,
        };
        Ok(ManagedHostAuthorityUpsert {
            connection_internal_id: fixture.connection_id().to_owned(),
            project_internal_id: fixture.project_id().to_owned(),
            external_contract_descriptor: current_managed_host_binding_external_descriptor(),
            managed_host_binding: binding,
            generated_artifacts: artifacts,
            host_verification_receipt: receipt,
        })
    }
}
