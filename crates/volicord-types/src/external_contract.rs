//! Exact external-contract descriptors and boundary adapter selection.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use schemars::{schema_for, JsonSchema};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    canonical_json_bytes, is_canonical_sha256_digest, ConfigurationTarget,
    ConfigurationTargetOwner, EnvironmentForwarding, ErrorCode, FailureCategory, HostKind,
    ManagedCommand, ManagedCommandResolution, ManagedConnectionScope, ManagedHostBinding,
    PlatformEnvironment, PlatformReleaseCoordinate, ProcessBinding,
    FIRST_RELEASE_CODEX_CAPABILITIES, MANAGED_HOST_BINDING_CANONICAL_FIELDS,
    MANAGED_HOST_BINDING_DOMAIN,
};

/// Exact semantic identity of the current canonical managed-host binding payload.
pub const MANAGED_HOST_BINDING_EXTERNAL_CONTRACT_ID: &str = "volicord.managed-host-binding";

/// Exact structural digest of the current canonical managed-host binding payload.
pub const MANAGED_HOST_BINDING_EXTERNAL_SCHEMA_DIGEST: &str =
    "sha256:c21ddd87aa848e56363f46b427b84d7f89d761e1762fec680e2dbb46dd38f15d";

/// Domain separator for the current managed-host external schema identity.
pub const MANAGED_HOST_BINDING_EXTERNAL_SCHEMA_DOMAIN: &[u8] =
    b"volicord.external-contract-schema\0";

/// Complete capability set required by the current managed-host binding boundary.
pub const MANAGED_HOST_BINDING_EXTERNAL_CAPABILITIES: [&str; 4] = [
    "managed_stdio_mcp",
    "personal_managed_binding",
    "record_workflow",
    "shared_managed_binding",
];

/// Describes one exact Volicord-owned external format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ExternalContractDescriptor {
    /// Semantic kind of contract, without a numeric revision suffix.
    pub contract_id: String,
    /// Digest of the exact structure and canonical encoding.
    pub schema_digest: String,
    /// Complete capability set supplied by the format.
    pub capabilities: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalContractDescriptorWire {
    contract_id: String,
    schema_digest: String,
    capabilities: Vec<String>,
}

impl ExternalContractDescriptor {
    /// Creates and validates an exact descriptor without normalizing its identity.
    pub fn new(
        contract_id: impl Into<String>,
        schema_digest: impl Into<String>,
        capabilities: Vec<String>,
    ) -> Result<Self, ExternalContractRegistryError> {
        let descriptor = Self {
            contract_id: contract_id.into(),
            schema_digest: schema_digest.into(),
            capabilities,
        };
        descriptor.validate()?;
        Ok(descriptor)
    }

    /// Validates structural descriptor requirements without trimming or case folding.
    pub fn validate(&self) -> Result<(), ExternalContractRegistryError> {
        if self.contract_id.is_empty() {
            return Err(ExternalContractRegistryError::MalformedDescriptor(
                "contract_id_required",
            ));
        }
        if has_numeric_revision_suffix(&self.contract_id) {
            return Err(ExternalContractRegistryError::MalformedDescriptor(
                "numeric_contract_revision_forbidden",
            ));
        }
        if self.schema_digest.is_empty() {
            return Err(ExternalContractRegistryError::MalformedDescriptor(
                "schema_digest_required",
            ));
        }
        if !is_canonical_sha256_digest(&self.schema_digest) {
            return Err(ExternalContractRegistryError::MalformedDescriptor(
                "schema_digest_invalid",
            ));
        }

        let mut seen = BTreeSet::new();
        for capability in &self.capabilities {
            if capability.is_empty() {
                return Err(ExternalContractRegistryError::MalformedDescriptor(
                    "empty_capability_forbidden",
                ));
            }
            if !seen.insert(capability.as_str()) {
                return Err(ExternalContractRegistryError::MalformedDescriptor(
                    "duplicate_capability_forbidden",
                ));
            }
        }
        Ok(())
    }

    fn capability_set(&self) -> BTreeSet<&str> {
        self.capabilities.iter().map(String::as_str).collect()
    }
}

impl<'de> Deserialize<'de> for ExternalContractDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ExternalContractDescriptorWire::deserialize(deserializer)?;
        Self::new(wire.contract_id, wire.schema_digest, wire.capabilities).map_err(D::Error::custom)
    }
}

fn has_numeric_revision_suffix(contract_id: &str) -> bool {
    contract_id.rsplit_once("-v").is_some_and(|(_, suffix)| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

#[derive(Debug)]
struct RegisteredExternalContract<T> {
    descriptor: ExternalContractDescriptor,
    adapter: T,
}

/// Boundary registry keyed only by exact `contract_id + schema_digest`.
#[derive(Debug)]
pub struct ExactExternalContractRegistry<T> {
    entries: BTreeMap<(String, String), RegisteredExternalContract<T>>,
}

impl<T> Default for ExactExternalContractRegistry<T> {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

impl<T> ExactExternalContractRegistry<T> {
    /// Creates an empty exact-match registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one adapter for one exact descriptor identity.
    pub fn register(
        &mut self,
        descriptor: ExternalContractDescriptor,
        adapter: T,
    ) -> Result<(), ExternalContractRegistryError> {
        descriptor.validate()?;
        let key = (
            descriptor.contract_id.clone(),
            descriptor.schema_digest.clone(),
        );
        if self.entries.contains_key(&key) {
            return Err(ExternalContractRegistryError::DuplicateRegistration);
        }
        self.entries.insert(
            key,
            RegisteredExternalContract {
                descriptor,
                adapter,
            },
        );
        Ok(())
    }

    /// Selects exactly one adapter and verifies the complete and required capabilities.
    pub fn resolve(
        &self,
        descriptor: &ExternalContractDescriptor,
        required_capabilities: &[&str],
    ) -> Result<&T, ExternalContractRegistryError> {
        descriptor.validate()?;
        let registered = self
            .entries
            .get(&(
                descriptor.contract_id.clone(),
                descriptor.schema_digest.clone(),
            ))
            .ok_or(ExternalContractRegistryError::UnsupportedExternalContract)?;

        let supplied = descriptor.capability_set();
        let registered_capabilities = registered.descriptor.capability_set();
        if supplied != registered_capabilities
            || required_capabilities
                .iter()
                .any(|capability| capability.is_empty() || !supplied.contains(capability))
        {
            return Err(ExternalContractRegistryError::UnsupportedExternalContract);
        }
        Ok(&registered.adapter)
    }

    /// Returns the number of exact registrations.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the registry contains no registrations.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Structured failure from descriptor validation or exact registry selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalContractRegistryError {
    /// The descriptor is structurally malformed and must not be probed or defaulted.
    MalformedDescriptor(&'static str),
    /// Two adapters attempted to claim the same exact registry key.
    DuplicateRegistration,
    /// No exact compatible descriptor and capability registration exists.
    UnsupportedExternalContract,
}

impl ExternalContractRegistryError {
    /// Returns the product-wide failure category for this boundary failure.
    pub const fn failure_category(self) -> FailureCategory {
        match self {
            Self::MalformedDescriptor(_) | Self::DuplicateRegistration => FailureCategory::Rejected,
            Self::UnsupportedExternalContract => FailureCategory::UnsupportedContract,
        }
    }

    /// Returns the public error code for this boundary failure.
    pub const fn error_code(self) -> ErrorCode {
        match self {
            Self::MalformedDescriptor(_) | Self::DuplicateRegistration => {
                ErrorCode::ValidationFailed
            }
            Self::UnsupportedExternalContract => ErrorCode::UnsupportedContract,
        }
    }

    /// Returns a stable machine-readable reason.
    pub const fn reason(self) -> &'static str {
        match self {
            Self::MalformedDescriptor(reason) => reason,
            Self::DuplicateRegistration => "duplicate_external_contract_registration",
            Self::UnsupportedExternalContract => "unsupported_external_contract",
        }
    }
}

impl fmt::Display for ExternalContractRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason())
    }
}

impl Error for ExternalContractRegistryError {}

/// Returns the only managed-host binding descriptor accepted before 1.0.
pub fn current_managed_host_binding_external_descriptor() -> ExternalContractDescriptor {
    ExternalContractDescriptor::new(
        MANAGED_HOST_BINDING_EXTERNAL_CONTRACT_ID,
        current_managed_host_binding_external_schema_digest(),
        MANAGED_HOST_BINDING_EXTERNAL_CAPABILITIES
            .iter()
            .map(|capability| (*capability).to_owned())
            .collect(),
    )
    .expect("the built-in managed-host binding descriptor is valid")
}

/// Computes the exact current managed-host external schema digest.
pub fn current_managed_host_binding_external_schema_digest() -> String {
    let identity = managed_host_binding_external_schema_identity();
    let mut digest = Sha256::new();
    digest.update(MANAGED_HOST_BINDING_EXTERNAL_SCHEMA_DOMAIN);
    digest.update(identity);
    format!("sha256:{:x}", digest.finalize())
}

#[derive(Serialize)]
struct ManagedHostBindingExternalSchemaIdentity {
    canonical_type_schema: serde_json::Value,
    canonical_codec_domain: Vec<u8>,
    canonical_codec_fields: Vec<&'static str>,
    canonical_codec_probe: Vec<u8>,
}

fn managed_host_binding_external_schema_identity() -> Vec<u8> {
    let schema = serde_json::to_value(schema_for!(ManagedHostBinding))
        .expect("the built-in managed-host binding schema serializes");
    managed_host_binding_external_schema_identity_for(schema)
}

fn managed_host_binding_external_schema_identity_for(
    canonical_type_schema: serde_json::Value,
) -> Vec<u8> {
    let probe = managed_host_binding_codec_probe();
    let canonical_codec_probe = probe
        .canonical_bytes()
        .expect("the built-in managed-host binding codec probe is valid");
    canonical_json_bytes(&ManagedHostBindingExternalSchemaIdentity {
        canonical_type_schema,
        canonical_codec_domain: MANAGED_HOST_BINDING_DOMAIN.to_vec(),
        canonical_codec_fields: MANAGED_HOST_BINDING_CANONICAL_FIELDS.to_vec(),
        canonical_codec_probe,
    })
    .expect("the built-in managed-host external schema identity serializes")
}

fn managed_host_binding_codec_probe() -> ManagedHostBinding {
    ManagedHostBinding {
        host_kind: HostKind::Codex,
        connection_scope: ManagedConnectionScope::Personal,
        command: ManagedCommand {
            resolution: ManagedCommandResolution::PathLookup,
            program: "volicord".to_owned(),
        },
        arguments: vec!["mcp".to_owned(), "--managed-schema-probe".to_owned()],
        forwarded_environment: vec![
            EnvironmentForwarding {
                source_name: "CODEX_HOME".to_owned(),
                target_name: "CODEX_HOME".to_owned(),
            },
            EnvironmentForwarding {
                source_name: "VOLICORD_HOME".to_owned(),
                target_name: "VOLICORD_HOME".to_owned(),
            },
        ],
        configuration_target: ConfigurationTarget {
            owner: ConfigurationTargetOwner::User,
            path: "/tmp/volicord-managed-host-schema-probe.toml".to_owned(),
        },
        process_binding: ProcessBinding {
            process_id: 1,
            process_start_token: "schema-probe-start".to_owned(),
            platform_instance_token: "schema-probe-platform".to_owned(),
            executable_path: "/usr/bin/codex".to_owned(),
            executable_digest: "0".repeat(64),
        },
        required_capabilities: FIRST_RELEASE_CODEX_CAPABILITIES.to_vec(),
        platform_environment: PlatformEnvironment::Linux,
        platform_release_coordinate: PlatformReleaseCoordinate::Native,
    }
}

/// Selects the current managed-host binding adapter by exact descriptor identity.
pub fn require_current_managed_host_binding_external_descriptor(
    descriptor: &ExternalContractDescriptor,
) -> Result<(), ExternalContractRegistryError> {
    let mut registry = ExactExternalContractRegistry::new();
    registry.register(current_managed_host_binding_external_descriptor(), ())?;
    registry
        .resolve(descriptor, &MANAGED_HOST_BINDING_EXTERNAL_CAPABILITIES)
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const SCHEMA_DIGEST: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const OTHER_SCHEMA_DIGEST: &str =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn descriptor(
        contract_id: &str,
        digest: &str,
        capabilities: &[&str],
    ) -> ExternalContractDescriptor {
        ExternalContractDescriptor::new(
            contract_id,
            digest,
            capabilities
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        )
        .expect("test descriptor should be valid")
    }

    #[test]
    fn current_managed_host_descriptor_is_derived_from_one_exact_schema_identity() {
        assert_eq!(
            current_managed_host_binding_external_schema_digest(),
            MANAGED_HOST_BINDING_EXTERNAL_SCHEMA_DIGEST
        );
        assert_eq!(
            current_managed_host_binding_external_descriptor().capabilities,
            MANAGED_HOST_BINDING_EXTERNAL_CAPABILITIES
        );
    }

    #[test]
    fn managed_host_type_schema_drift_changes_the_external_descriptor_identity() {
        let mut changed_schema = serde_json::to_value(schema_for!(ManagedHostBinding))
            .expect("managed-host schema should serialize");
        changed_schema
            .as_object_mut()
            .expect("root schema should be an object")
            .insert("x-drift-probe".to_owned(), serde_json::Value::Bool(true));

        let changed_identity = managed_host_binding_external_schema_identity_for(changed_schema);
        let mut changed_digest = Sha256::new();
        changed_digest.update(MANAGED_HOST_BINDING_EXTERNAL_SCHEMA_DOMAIN);
        changed_digest.update(changed_identity);
        let changed_digest = format!("sha256:{:x}", changed_digest.finalize());

        assert_ne!(
            changed_digest,
            current_managed_host_binding_external_schema_digest()
        );
    }

    #[test]
    fn descriptor_decode_is_strict_and_does_not_supply_defaults() {
        let valid = json!({
            "contract_id": "volicord.codex.release-manifest",
            "schema_digest": SCHEMA_DIGEST,
            "capabilities": ["managed_stdio", "record"]
        });
        assert_eq!(
            serde_json::from_value::<ExternalContractDescriptor>(valid.clone())
                .expect("descriptor should decode")
                .capabilities,
            vec!["managed_stdio", "record"]
        );

        for invalid in [
            json!({"schema_digest": SCHEMA_DIGEST, "capabilities": []}),
            json!({"contract_id": "x", "capabilities": []}),
            json!({"contract_id": "", "schema_digest": SCHEMA_DIGEST, "capabilities": []}),
            json!({"contract_id": "x", "schema_digest": "", "capabilities": []}),
            json!({"contract_id": "x", "schema_digest": "sha256:abc", "capabilities": []}),
            json!({"contract_id": "x", "schema_digest": SCHEMA_DIGEST}),
            json!({"contract_id": "x", "schema_digest": SCHEMA_DIGEST, "capabilities": [], "extra": true}),
            json!({"contract_id": "x", "schema_digest": SCHEMA_DIGEST, "capabilities": [""]}),
            json!({"contract_id": "x", "schema_digest": SCHEMA_DIGEST, "capabilities": ["record", "record"]}),
        ] {
            assert!(serde_json::from_value::<ExternalContractDescriptor>(invalid).is_err());
        }
    }

    #[test]
    fn volicord_contract_ids_reject_numeric_revision_suffixes() {
        for contract_id in ["contract-v1", "contract-v2", "contract-v01", "contract-v0"] {
            assert_eq!(
                ExternalContractDescriptor::new(contract_id, SCHEMA_DIGEST, vec![])
                    .expect_err("numeric revision suffix must be rejected")
                    .reason(),
                "numeric_contract_revision_forbidden"
            );
        }
        for contract_id in ["contract-v", "contract-vnext", "mcp/2025-06-18"] {
            assert!(ExternalContractDescriptor::new(contract_id, SCHEMA_DIGEST, vec![]).is_ok());
        }
    }

    #[test]
    fn registry_uses_only_the_exact_pair_without_normalization_or_fallback() {
        let current = descriptor(
            "volicord.codex.release-manifest",
            SCHEMA_DIGEST,
            &["record"],
        );
        let mut registry = ExactExternalContractRegistry::new();
        registry
            .register(current.clone(), "current-adapter")
            .expect("current adapter should register");

        assert_eq!(
            registry.resolve(&current, &["record"]),
            Ok(&"current-adapter")
        );
        for unsupported in [
            descriptor(
                "Volicord.codex.release-manifest",
                SCHEMA_DIGEST,
                &["record"],
            ),
            descriptor(
                "volicord.codex.release-manifest ",
                SCHEMA_DIGEST,
                &["record"],
            ),
            descriptor(
                "volicord.codex.release-manifest",
                OTHER_SCHEMA_DIGEST,
                &["record"],
            ),
        ] {
            let error = registry
                .resolve(&unsupported, &["record"])
                .expect_err("nearby descriptor must not fall back");
            assert_eq!(error.reason(), "unsupported_external_contract");
            assert_eq!(error.error_code(), ErrorCode::UnsupportedContract);
            assert_eq!(
                error.failure_category(),
                FailureCategory::UnsupportedContract
            );
        }
        assert_eq!(
            ExternalContractDescriptor::new(
                "volicord.codex.release-manifest",
                "sha256:abc",
                vec!["record".to_owned()],
            ),
            Err(ExternalContractRegistryError::MalformedDescriptor(
                "schema_digest_invalid"
            ))
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn registry_requires_exact_declared_and_receiving_capabilities() {
        let current = descriptor(
            "volicord.codex.release-manifest",
            SCHEMA_DIGEST,
            &["managed_stdio", "record"],
        );
        let mut registry = ExactExternalContractRegistry::new();
        registry
            .register(current.clone(), 7_u8)
            .expect("current adapter should register");

        assert_eq!(registry.resolve(&current, &["record"]), Ok(&7));
        let incomplete = descriptor(
            "volicord.codex.release-manifest",
            SCHEMA_DIGEST,
            &["record"],
        );
        assert_eq!(
            registry.resolve(&incomplete, &["record"]),
            Err(ExternalContractRegistryError::UnsupportedExternalContract)
        );
        assert_eq!(
            registry.resolve(&current, &["guard"]),
            Err(ExternalContractRegistryError::UnsupportedExternalContract)
        );
    }

    #[test]
    fn duplicate_exact_registration_is_rejected() {
        let current = descriptor("volicord.storage", SCHEMA_DIGEST, &["sqlite"]);
        let mut registry = ExactExternalContractRegistry::new();
        registry
            .register(current.clone(), "first")
            .expect("first adapter should register");
        assert_eq!(
            registry.register(current, "second"),
            Err(ExternalContractRegistryError::DuplicateRegistration)
        );
    }
}
