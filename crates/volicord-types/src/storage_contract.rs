//! Canonical SQLite storage manifest and generated schema metadata values.

use std::{error::Error, fmt};

use schemars::JsonSchema;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

use crate::canonical::is_canonical_sha256_digest;

/// Semantic identity of the canonical SQLite storage contract.
pub const STORAGE_CONTRACT_ID: &str = "volicord.sqlite.canonical";

/// Complete capability set enabled by the canonical SQLite storage contract.
pub const STORAGE_ENABLED_CAPABILITIES: &[&str] = &[
    "artifact_storage",
    "authority_event_chain",
    "exact_operation_result",
    "invocation_repository_observation",
    "managed_codex_connection",
    "operational_mcp_sessions",
    "project_continuity",
    "shaping_checkpoint_lineage",
    "shaping_decision_recovery",
    "shaping_progression",
    "user_action_cli_resolution",
];

/// Complete exact identity persisted in each canonical SQLite carrier column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct StorageManifest {
    pub contract_id: String,
    pub canonical_ddl_digest: String,
    pub integrity_constraints_digest: String,
    pub enabled_capabilities: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageManifestWire {
    contract_id: String,
    canonical_ddl_digest: String,
    integrity_constraints_digest: String,
    enabled_capabilities: Vec<String>,
}

impl StorageManifest {
    /// Constructs the current manifest from digests generated from canonical SQL.
    pub fn current(
        canonical_ddl_digest: impl Into<String>,
        integrity_constraints_digest: impl Into<String>,
    ) -> Result<Self, StorageManifestError> {
        Self::new(
            STORAGE_CONTRACT_ID,
            canonical_ddl_digest,
            integrity_constraints_digest,
            STORAGE_ENABLED_CAPABILITIES
                .iter()
                .map(|capability| (*capability).to_owned())
                .collect(),
        )
    }

    /// Constructs a strictly shaped manifest without normalizing any field.
    pub fn new(
        contract_id: impl Into<String>,
        canonical_ddl_digest: impl Into<String>,
        integrity_constraints_digest: impl Into<String>,
        enabled_capabilities: Vec<String>,
    ) -> Result<Self, StorageManifestError> {
        let manifest = Self {
            contract_id: contract_id.into(),
            canonical_ddl_digest: canonical_ddl_digest.into(),
            integrity_constraints_digest: integrity_constraints_digest.into(),
            enabled_capabilities,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validates the exact persisted representation rules without supplying defaults.
    pub fn validate(&self) -> Result<(), StorageManifestError> {
        if self.contract_id.is_empty() {
            return Err(StorageManifestError::ContractIdRequired);
        }
        if has_numeric_revision_suffix(&self.contract_id) {
            return Err(StorageManifestError::NumericContractRevisionForbidden);
        }
        if !is_canonical_sha256_digest(&self.canonical_ddl_digest) {
            return Err(StorageManifestError::CanonicalDdlDigestInvalid);
        }
        if !is_canonical_sha256_digest(&self.integrity_constraints_digest) {
            return Err(StorageManifestError::IntegrityConstraintsDigestInvalid);
        }

        let mut previous: Option<&str> = None;
        for capability in &self.enabled_capabilities {
            if capability.is_empty() {
                return Err(StorageManifestError::EmptyCapabilityForbidden);
            }
            if previous.is_some_and(|value| value >= capability.as_str()) {
                return Err(StorageManifestError::CapabilitiesNotStrictlySorted);
            }
            previous = Some(capability);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for StorageManifest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = StorageManifestWire::deserialize(deserializer)?;
        Self::new(
            wire.contract_id,
            wire.canonical_ddl_digest,
            wire.integrity_constraints_digest,
            wire.enabled_capabilities,
        )
        .map_err(D::Error::custom)
    }
}

fn has_numeric_revision_suffix(contract_id: &str) -> bool {
    contract_id.rsplit_once("-v").is_some_and(|(_, suffix)| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

/// Strict manifest-shape validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageManifestError {
    ContractIdRequired,
    NumericContractRevisionForbidden,
    CanonicalDdlDigestInvalid,
    IntegrityConstraintsDigestInvalid,
    EmptyCapabilityForbidden,
    CapabilitiesNotStrictlySorted,
}

impl StorageManifestError {
    pub const fn reason(self) -> &'static str {
        match self {
            Self::ContractIdRequired => "storage_contract_id_required",
            Self::NumericContractRevisionForbidden => "numeric_storage_contract_revision_forbidden",
            Self::CanonicalDdlDigestInvalid => "canonical_ddl_digest_invalid",
            Self::IntegrityConstraintsDigestInvalid => "integrity_constraints_digest_invalid",
            Self::EmptyCapabilityForbidden => "empty_storage_capability_forbidden",
            Self::CapabilitiesNotStrictlySorted => "storage_capabilities_not_strictly_sorted",
        }
    }
}

impl fmt::Display for StorageManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}

impl Error for StorageManifestError {}

/// Fixed source database for one generated schema fact.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum StorageDatabaseKind {
    Registry,
    ProjectState,
}

/// SQLite relation kind represented in the generated relation inventory.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedRelationKind {
    Table,
    View,
    Trigger,
}

/// One generated SQLite table or view fact.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeneratedTable {
    pub database: StorageDatabaseKind,
    pub relation_kind: GeneratedRelationKind,
    pub name: String,
    pub canonical_sql: String,
}

/// One generated SQLite column fact, in physical ordinal order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeneratedColumn {
    pub database: StorageDatabaseKind,
    pub table: String,
    pub ordinal: u32,
    pub name: String,
    pub declared_type: String,
    pub not_null: bool,
    pub default_value: Option<String>,
    pub primary_key_ordinal: u32,
    pub hidden: u32,
}

/// One generated explicit SQLite index fact.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeneratedIndex {
    pub database: StorageDatabaseKind,
    pub table: String,
    pub name: String,
    pub unique: bool,
    pub partial: bool,
    pub canonical_sql: String,
}

/// Canonical table definition used as the complete integrity-constraint fact.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeneratedConstraint {
    pub database: StorageDatabaseKind,
    pub table: String,
    pub canonical_table_sql: String,
}

/// Deterministic schema inventory derived from the two canonical SQL sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeneratedSchemaMetadata {
    pub tables: Vec<GeneratedTable>,
    pub columns: Vec<GeneratedColumn>,
    pub indexes: Vec<GeneratedIndex>,
    pub constraints: Vec<GeneratedConstraint>,
    pub canonical_ddl_digest: String,
    pub integrity_constraints_digest: String,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn digest(hex: char) -> String {
        format!("sha256:{}", hex.to_string().repeat(64))
    }

    #[test]
    fn current_manifest_has_exact_semantic_identity_and_capabilities() {
        let manifest = StorageManifest::current(digest('a'), digest('b'))
            .expect("current manifest should be valid");
        assert_eq!(manifest.contract_id, STORAGE_CONTRACT_ID);
        assert_eq!(
            manifest.enabled_capabilities,
            STORAGE_ENABLED_CAPABILITIES
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn manifest_decode_is_strict_and_supplies_no_defaults() {
        let valid = json!({
            "contract_id": STORAGE_CONTRACT_ID,
            "canonical_ddl_digest": digest('a'),
            "integrity_constraints_digest": digest('b'),
            "enabled_capabilities": STORAGE_ENABLED_CAPABILITIES,
        });
        serde_json::from_value::<StorageManifest>(valid.clone())
            .expect("current shape should decode");

        for invalid in [
            json!({
                "canonical_ddl_digest": digest('a'),
                "integrity_constraints_digest": digest('b'),
                "enabled_capabilities": STORAGE_ENABLED_CAPABILITIES,
            }),
            json!({
                "contract_id": STORAGE_CONTRACT_ID,
                "canonical_ddl_digest": digest('a'),
                "integrity_constraints_digest": digest('b'),
            }),
            json!({
                "contract_id": STORAGE_CONTRACT_ID,
                "canonical_ddl_digest": "sha256:ABC",
                "integrity_constraints_digest": digest('b'),
                "enabled_capabilities": STORAGE_ENABLED_CAPABILITIES,
            }),
            json!({
                "contract_id": STORAGE_CONTRACT_ID,
                "canonical_ddl_digest": digest('a'),
                "integrity_constraints_digest": digest('b'),
                "enabled_capabilities": ["authority_event_chain", "artifact_storage"],
            }),
            json!({
                "contract_id": STORAGE_CONTRACT_ID,
                "canonical_ddl_digest": digest('a'),
                "integrity_constraints_digest": digest('b'),
                "enabled_capabilities": STORAGE_ENABLED_CAPABILITIES,
                "unknown": true,
            }),
        ] {
            assert!(serde_json::from_value::<StorageManifest>(invalid).is_err());
        }

        let duplicate = format!(
            "{{\"contract_id\":\"{STORAGE_CONTRACT_ID}\",\"contract_id\":\"duplicate\",\"canonical_ddl_digest\":\"{}\",\"integrity_constraints_digest\":\"{}\",\"enabled_capabilities\":[]}}",
            digest('a'),
            digest('b')
        );
        assert!(serde_json::from_str::<StorageManifest>(&duplicate).is_err());
    }
}
